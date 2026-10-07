//! The first-person weapon, drawn in **view space** and flattened to 2D
//! triangles.
//!
//! View space is a right-handed frame centred on the eye: `+x` right, `+y` up,
//! `+z` straight ahead. Every weapon is modelled there as a handful of boxes and
//! wedges, projected with the same formula as the 3D camera, and drawn with
//! `draw_triangle` in the 2D pass. Keeping the gun out of the world pass is what
//! stops it poking through walls the player is standing against.
//!
//! Each weapon's local origin sits on its sight line, so the pose offset alone
//! decides where the sights point: with `player.ads == 1.0` the offset is
//! `(0, 0, z)` and the sight line lands on the screen centre.
//!
//! Around that aim blend the pose carries the body as well: the stance the
//! weapon is held in, the sideways slide of a lean, the inspect and the
//! grenade-throw motions. Everything in those that would push the sight line
//! off the crosshair — sideways and vertical shifts, yaw and pitch — is scaled
//! by `hip`, which is 1 at the waist and 0 with the sights up, so aiming always
//! wins. Depth and roll move the weapon without moving the line its sights sit
//! on, so they stay at full strength whatever the aim.

use crate::grenade::GrenadeKind;
use crate::player::{Player, Stance};
use crate::util::approach;
use crate::weapon::{ALL_KINDS, Arsenal, Weapon, WeaponBuild, WeaponKind};
use macroquad::prelude::*;
use std::f32::consts::TAU;

/// Vertices nearer than this to the eye are dropped instead of projected.
const NEAR: f32 = 0.05;
/// Sway bleeds off by this much per 60 Hz frame.
const SWAY_DECAY: f32 = 0.85;
/// Largest sway a single frame of mouse movement can add, in view units.
const SWAY_STEP: f32 = 0.022;
const SWAY_MAX: f32 = 0.032;
/// `Arsenal::switch_t` counts down from this.
const SWITCH_TIME: f32 = 0.32;
/// How fast the pose catches up with a change of stance, per second.
const STANCE_RATE: f32 = 5.5;
/// Bodily bob left while walking, on top of the damping `Player` already does.
const WALK_BOB: f32 = 0.50;
/// Length of the grenade throw motion, in seconds.
const GRENADE_TIME: f32 = 0.50;

/// How fast the weapon leaves the eye when a throwable comes to hand, and comes
/// back when it goes.
const HOLD_RATE: f32 = 6.0;

/// Where a throwable is held, in the view's own space: right, down, and metres
/// forward of the eye.
///
/// Written down once, because two things have to agree about it: the viewmodel
/// draws the thing here, and the game throws it from here — `main.rs` maps this
/// into the world with the camera's own basis (`hand_pos`), which is why there is
/// one mark on screen and one starting point for the arc rather than two that
/// nearly line up.
///
/// The right hand, because the gun is not in it: a throwable in hand puts the
/// weapon away (see `HOLD_DROP`), so the hand that was holding the rifle is the
/// hand that throws. Down and out of the middle as well, so the arc — which
/// leaves this point — is not drawn through the thing it leaves.
pub const HAND_OFFSET: Vec3 = vec3(0.129, -0.126, 0.335);

/// How far the weapon is put away while a throwable is in the throwing hand:
/// down, back towards the eye, and a little outboard, with the muzzle canted
/// over. How far is not a matter of taste — the test
/// `the_weapon_is_put_away_while_a_throwable_is_in_hand` measures every vertex
/// of every weapon against the frame, and "put away" means none of them is in
/// it.
const HOLD_DROP: f32 = 0.62;
const HOLD_BACK: f32 = 0.080;
const HOLD_OUT: f32 = 0.040;
const HOLD_CANT: Vec3 = vec3(0.34, 0.16, 0.30);

/// How far the throwing hand has to rise into the frame with the thing in it,
/// and how far it is carried off the bottom of the frame by the throw.
const HAND_RISE: f32 = 0.30;
const HAND_FALL: f32 = 0.45;
/// How much of the throw motion the arm takes to clear the frame — the whole of
/// the time the weapon waits for it. See `arm_falling`.
const THROW_CLEAR: f32 = 0.45;

/// How far the sights have to be up before the sniper's scope tube is left out
/// of the in-hand model, as a share of the ADS weight. Halfway is where the eye
/// reaches the eyepiece; past it the tube can only block the view the scope
/// exists to provide, and `hud::draw_scope_overlay` is drawing the aperture
/// instead. The HUD reads this too, so the two swap on the same frame.
pub const SCOPE_HIDE_ADS: f32 = 0.55;
/// The sniper's aperture, as a share of screen height, at the moment the tube
/// drops and again once the sight is fully up. Opening between the two reads as
/// the scope settling rather than as a cut.
pub const SCOPE_RADIUS_MIN: f32 = 0.29;
pub const SCOPE_RADIUS_MAX: f32 = 0.40;

/// How a crouch carries the weapon: a little lower, a little closer in.
const CROUCH_CARRY: Vec3 = vec3(0.0, -0.035, -0.030);
const CROUCH_CANT: Vec3 = vec3(0.040, 0.025, 0.055);
/// Prone is the biggest change of the three — the weapon drops away from the
/// eye line and cants right over as though braced on the ground. The drop is
/// kept modest because the short weapons are held close to the eye, where a
/// view unit is worth far more on screen; the cant carries the rest of it.
const PRONE_CARRY: Vec3 = vec3(-0.018, -0.070, -0.045);
const PRONE_CANT: Vec3 = vec3(0.180, 0.065, 0.360);
/// Key light in view space: above and to the left of the shooter, tipped
/// forward so surfaces turned away from the player catch the most light.
const KEY: Vec3 = vec3(-0.45, 0.76, 0.47);

const STEEL: Color = Color::new(0.22, 0.23, 0.26, 1.0);
const STEEL_DARK: Color = Color::new(0.13, 0.14, 0.16, 1.0);
const POLY: Color = Color::new(0.15, 0.16, 0.17, 1.0);
const RAIL: Color = Color::new(0.30, 0.31, 0.34, 1.0);
const BLACK: Color = Color::new(0.07, 0.07, 0.08, 1.0);
const WOOD: Color = Color::new(0.45, 0.28, 0.14, 1.0);
const WOOD_DARK: Color = Color::new(0.31, 0.19, 0.10, 1.0);
const OLIVE: Color = Color::new(0.28, 0.30, 0.22, 1.0);
/// Cartridge brass, for the belt on the support weapon's tin. Deliberately not
/// in `refinished`: a finish paints the gun, not the ammunition in it.
const BRASS: Color = Color::new(0.52, 0.39, 0.17, 1.0);
/// Gloved hands. Without them the weapon is the only thing in the frame and
/// reads as floating in front of the camera, which is the first thing anyone
/// notices about a first-person view.
const GLOVE: Color = Color::new(0.30, 0.25, 0.19, 1.0);
const GLOVE_DARK: Color = Color::new(0.19, 0.16, 0.12, 1.0);
const SLEEVE: Color = Color::new(0.31, 0.33, 0.26, 1.0);
/// A throwable's openings — a smoke grenade's vents, the perforations an M84 is
/// made of. Its body is the kind's own colour (`GrenadeKind::color`), which is
/// the part that says which one is in the hand; this is what is behind the
/// wall of it.
const CASE: Color = Color::new(0.09, 0.10, 0.12, 1.0);
/// The rag in a bottle's neck, which is cloth and not steel.
const RAG: Color = Color::new(0.66, 0.62, 0.50, 1.0);
/// The lever's and the rag's own thicknesses, from the world's constants: they
/// are drawn here at `HELD_HALF` instead of `HALF`, and a part that did not
/// shrink with the object would be a full-sized spoon in a hand-sized grenade.
const LEVER_HALF: f32 = crate::grenade::LEVER_HALF;
const WICK_HALF: f32 = crate::grenade::WICK_HALF;
const WICK_STRAND: f32 = crate::grenade::WICK_STRAND;

/// A finish: the colour each paintable role is drawn in.
///
/// The roles are the consts above, one name each, so a finish is a table of
/// substitutions and no geometry changes. `BLACK` and the gloves are
/// deliberately absent — sights stay black under every finish, and hands are
/// gear, not weapon.
#[derive(Clone, Copy)]
pub struct Finish {
    pub id: &'static str,
    pub name_en: &'static str,
    pub name_zh: &'static str,
    pub metal: Color,
    pub metal_dark: Color,
    pub poly: Color,
    pub rail: Color,
    pub wood: Color,
    pub wood_dark: Color,
    pub olive: Color,
}

/// The finishes the game ships, in the order the bench walks them. The first
/// is the factory paint: the exact consts above, so an unpainted weapon looks
/// the way it always has.
pub const FINISHES: [Finish; 6] = [
    Finish {
        id: "factory",
        name_en: "FACTORY",
        name_zh: "原厂",
        metal: STEEL,
        metal_dark: STEEL_DARK,
        poly: POLY,
        rail: RAIL,
        wood: WOOD,
        wood_dark: WOOD_DARK,
        olive: OLIVE,
    },
    Finish {
        id: "driftwood",
        name_en: "DRIFTWOOD",
        name_zh: "沉木",
        metal: Color::new(0.34, 0.33, 0.30, 1.0),
        metal_dark: Color::new(0.22, 0.21, 0.19, 1.0),
        poly: Color::new(0.26, 0.25, 0.23, 1.0),
        rail: Color::new(0.42, 0.40, 0.36, 1.0),
        wood: Color::new(0.45, 0.43, 0.38, 1.0),
        wood_dark: Color::new(0.32, 0.30, 0.26, 1.0),
        olive: Color::new(0.36, 0.36, 0.30, 1.0),
    },
    Finish {
        id: "desert",
        name_en: "DESERT",
        name_zh: "荒漠",
        metal: Color::new(0.62, 0.54, 0.38, 1.0),
        metal_dark: Color::new(0.44, 0.38, 0.27, 1.0),
        poly: Color::new(0.72, 0.65, 0.48, 1.0),
        rail: Color::new(0.50, 0.44, 0.32, 1.0),
        wood: Color::new(0.55, 0.44, 0.28, 1.0),
        wood_dark: Color::new(0.40, 0.31, 0.19, 1.0),
        olive: Color::new(0.58, 0.55, 0.38, 1.0),
    },
    Finish {
        id: "abyss",
        name_en: "ABYSS",
        name_zh: "深渊",
        metal: Color::new(0.10, 0.16, 0.17, 1.0),
        metal_dark: Color::new(0.05, 0.09, 0.10, 1.0),
        poly: Color::new(0.08, 0.12, 0.13, 1.0),
        rail: Color::new(0.45, 0.68, 0.64, 1.0),
        wood: Color::new(0.16, 0.22, 0.20, 1.0),
        wood_dark: Color::new(0.10, 0.15, 0.14, 1.0),
        olive: Color::new(0.14, 0.20, 0.18, 1.0),
    },
    Finish {
        id: "blight",
        name_en: "BLIGHT",
        name_zh: "腐化",
        metal: Color::new(0.30, 0.42, 0.26, 1.0),
        metal_dark: Color::new(0.20, 0.28, 0.17, 1.0),
        poly: Color::new(0.24, 0.20, 0.28, 1.0),
        rail: Color::new(0.52, 0.72, 0.34, 1.0),
        wood: Color::new(0.42, 0.29, 0.40, 1.0),
        wood_dark: Color::new(0.28, 0.19, 0.27, 1.0),
        olive: Color::new(0.36, 0.46, 0.24, 1.0),
    },
    Finish {
        id: "ashen",
        name_en: "ASHEN",
        name_zh: "灰烬",
        metal: Color::new(0.16, 0.16, 0.17, 1.0),
        metal_dark: Color::new(0.09, 0.09, 0.10, 1.0),
        poly: Color::new(0.12, 0.12, 0.13, 1.0),
        rail: Color::new(0.72, 0.70, 0.62, 1.0),
        wood: Color::new(0.64, 0.60, 0.50, 1.0),
        wood_dark: Color::new(0.45, 0.42, 0.35, 1.0),
        olive: Color::new(0.30, 0.30, 0.27, 1.0),
    },
];

/// A finish and the wear it has taken: what the armory edits and every mission
/// carries.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Paint {
    /// Index into [`FINISHES`], clamped on read.
    pub finish: usize,
    /// 0 brand new, 1 fully worn. Blends each face towards `WORN` by this much,
    /// jittered per face, so wear reads as a pattern rather than a flat fade.
    pub wear: f32,
    /// The wear pattern's seed.
    pub seed: u32,
}

impl Paint {
    /// The weapon as it comes from the crate: factory paint, no wear.
    pub const fn factory() -> Self {
        Self {
            finish: 0,
            wear: 0.0,
            seed: 0,
        }
    }

    pub fn finish(self) -> &'static Finish {
        &FINISHES[self.finish.min(FINISHES.len() - 1)]
    }

    /// The settings-file spelling: `id@wear:seed` — or empty for the weapon as
    /// it came, because a factory finish with no wear is exactly what an
    /// absent line means, and writing it out would turn every save into seven
    /// lines about nothing.
    pub fn to_text(self) -> String {
        if self == Self::factory() {
            return String::new();
        }
        format!("{}@{:.2}:{}", self.finish().id, self.wear, self.seed)
    }

    /// Parse [`Paint::to_text`]. Anything unreadable falls back to the factory
    /// paint rather than failing: the file is user-editable text.
    pub fn from_text(text: &str) -> Self {
        let Some((head, seed)) = text.trim().split_once(':') else {
            return Self::factory();
        };
        let (id, wear) = head.split_once('@').unwrap_or((head, "0.0"));
        Self {
            finish: FINISHES.iter().position(|f| f.id == id.trim()).unwrap_or(0),
            wear: wear.trim().parse::<f32>().unwrap_or(0.0).clamp(0.0, 1.0),
            seed: seed.trim().parse::<u32>().unwrap_or(0),
        }
    }
}

/// Where wear takes a face. Scuffed metal and greyed wood are both this colour,
/// which is what makes one `WORN` enough for every finish.
const WORN: Color = Color::new(0.30, 0.28, 0.25, 1.0);

/// The finish colour a factory colour becomes, by exact match: the consts
/// above are what the builders emit, so this is the whole substitution table.
fn refinished(c: Color, f: &Finish) -> Color {
    if c == STEEL {
        f.metal
    } else if c == STEEL_DARK {
        f.metal_dark
    } else if c == POLY {
        f.poly
    } else if c == RAIL {
        f.rail
    } else if c == WOOD {
        f.wood
    } else if c == WOOD_DARK {
        f.wood_dark
    } else if c == OLIVE {
        f.olive
    } else {
        c
    }
}

/// A stable 0..1 from a position and a seed, quantised to four centimetres so
/// wear reads as patches rather than as per-pixel noise.
fn wear_hash(at: Vec3, seed: u32) -> f32 {
    let mut h = seed ^ 0x9E37_79B9;
    for v in [at.x, at.y, at.z] {
        h ^= ((v * 25.0) as i32 as u32).wrapping_mul(0x85EB_CA6B);
        h = h.rotate_left(13).wrapping_mul(0xC2B2_AE35);
        h ^= h >> 15;
    }
    (h >> 8) as f32 / (1u32 << 24) as f32
}

/// A triangle's centroid, what wear is hashed on.
fn centroid(t: Tri) -> Vec3 {
    (t.a + t.b + t.c) / 3.0
}

/// One factory-coloured face as the finish and its wear leave it.
fn painted(c: Color, paint: Paint, at: Vec3) -> Color {
    let c = refinished(c, paint.finish());
    if paint.wear <= 0.0 {
        return c;
    }
    // Some patches wear harder than the rest, so a worn weapon reads as
    // handled rather than as globally faded.
    let t = (paint.wear * (0.35 + 0.65 * wear_hash(at, paint.seed))).clamp(0.0, 1.0);
    Color::new(
        c.r + (WORN.r - c.r) * t,
        c.g + (WORN.g - c.g) * t,
        c.b + (WORN.b - c.b) * t,
        c.a,
    )
}

/// Repaint and weather a batch of weapon triangles, in whichever local space
/// they were built. Hands and the muzzle flash never pass through this: gear
/// and light do not take the finish.
fn paint_tris(tris: &mut [Tri], paint: Paint) {
    for t in tris.iter_mut() {
        t.color = painted(t.color, paint, centroid(*t));
    }
}

/// [`paint_tris`] for one triangle, on the way into the per-frame draw list.
fn painted_tri(t: Tri, paint: Paint) -> Tri {
    Tri::new(t.a, t.b, t.c, painted(t.color, paint, centroid(t)))
}

/// How much of the weapon's sway the hands do not follow. A hand that tracked
/// the weapon exactly would make the two read as one rigid object; a little
/// lag is what makes an arm look like it is holding something.
const HAND_LAG: f32 = 0.4;

/// One solid face, in whichever space it is handed to the pipeline.
#[derive(Clone, Copy)]
struct Tri {
    a: Vec3,
    b: Vec3,
    c: Vec3,
    color: Color,
}

impl Tri {
    fn new(a: Vec3, b: Vec3, c: Vec3, color: Color) -> Self {
        Self { a, b, c, color }
    }

    fn map(&self, m: Mat4) -> Self {
        Self::new(
            m.transform_point3(self.a),
            m.transform_point3(self.b),
            m.transform_point3(self.c),
            self.color,
        )
    }

    fn finite(&self) -> bool {
        self.a.is_finite() && self.b.is_finite() && self.c.is_finite()
    }

    fn depth(&self) -> f32 {
        (self.a.z + self.b.z + self.c.z) / 3.0
    }

    /// Two-sided shading: the face normal is flipped towards the eye, so a panel
    /// is lit by whichever of its sides the player can actually see.
    fn shaded(&self) -> Color {
        let n = (self.b - self.a).cross(self.c - self.a).normalize_or_zero();
        let middle = (self.a + self.b + self.c) / 3.0;
        let n = if n.dot(middle) < 0.0 { -n } else { n };
        let key = n.dot(KEY).max(0.0);
        let sky = n.y.max(0.0) * 0.14;
        let fill = (-n.z).max(0.0) * 0.10;
        let f = (0.26 + 0.60 * key + sky + fill).min(1.0);
        Color::new(
            self.color.r * f,
            self.color.g * f,
            self.color.b * f,
            self.color.a,
        )
    }
}

fn quad(out: &mut Vec<Tri>, a: Vec3, b: Vec3, c: Vec3, d: Vec3, color: Color) {
    out.push(Tri::new(a, b, c, color));
    out.push(Tri::new(a, c, d, color));
}

/// Axes across a bar: `Y` unless the bar is close to vertical.
fn cross_axes(axis: Vec3) -> Option<(Vec3, Vec3)> {
    let d = axis.normalize_or_zero();
    if d == Vec3::ZERO {
        return None;
    }
    let up = if d.y.abs() < 0.85 { Vec3::Y } else { Vec3::Z };
    let r = d.cross(up).normalize_or_zero();
    Some((r, r.cross(d)))
}

fn corners(c: Vec3, h: Vec2, r: Vec3, u: Vec3) -> [Vec3; 4] {
    [
        c - r * h.x - u * h.y,
        c + r * h.x - u * h.y,
        c + r * h.x + u * h.y,
        c - r * h.x + u * h.y,
    ]
}

/// A faceted tube between two cross-sections, the round counterpart of `bar`.
///
/// A roller-locked submachine gun's receiver and handguard are rolled sheet —
/// round, not boxy — and drawing them as boxes puts the weapon in the wrong
/// family. Eight facets is enough to read as round at the size a viewmodel is
/// actually seen, and the caps are fans around the section centres.
fn tube(out: &mut Vec<Tri>, a: Vec3, ra: f32, b: Vec3, rb: f32, color: Color) {
    const FACETS: usize = 8;
    let Some((r, u)) = cross_axes(b - a) else {
        return;
    };
    let ring = |at: Vec3, rad: f32| -> Vec<Vec3> {
        (0..FACETS)
            .map(|i| {
                let t = i as f32 / FACETS as f32 * std::f32::consts::TAU;
                at + r * (rad * t.cos()) + u * (rad * t.sin())
            })
            .collect()
    };
    let (p, q) = (ring(a, ra), ring(b, rb));
    for i in 0..FACETS {
        let j = (i + 1) % FACETS;
        quad(out, p[i], q[i], q[j], p[j], color);
    }
    for (hub, ring) in [(a, &p), (b, &q)] {
        for i in 0..FACETS {
            let j = (i + 1) % FACETS;
            out.push(Tri::new(hub, ring[i], ring[j], color));
        }
    }
}

/// Rectangular bar between two cross-sections, each a centre plus half-extents
/// in the plane across the bar.
fn bar(out: &mut Vec<Tri>, a: Vec3, ha: Vec2, b: Vec3, hb: Vec2, color: Color) {
    let Some((r, u)) = cross_axes(b - a) else {
        return;
    };
    let p = corners(a, ha, r, u);
    let q = corners(b, hb, r, u);
    for i in 0..4 {
        let j = (i + 1) % 4;
        quad(out, p[i], q[i], q[j], p[j], color);
    }
    quad(out, p[0], p[1], p[2], p[3], color);
    quad(out, q[0], q[1], q[2], q[3], color);
}

/// A box magazine: sections laid along an arc that sweeps forward as it drops —
/// the way a magazine for a tapered case does — closed by a floorplate.
///
/// The sections used to step straight down with a fixed forward offset per step:
/// a stair, which is what the old `rake` was approximating. A real magazine
/// curves, and the difference shows most at a join — a fitting that continued a
/// stair from the wrong rung was a fitting that did not meet the magazine it was
/// meant to be continuing.
fn magazine(out: &mut Vec<Tri>, top: Vec3, length: f32, half: Vec2, curve: f32, color: Color) {
    const SECTIONS: usize = 5;
    let step = length / SECTIONS as f32;
    // `curve` is radians per metre, not radians for this magazine: the radius is
    // set by the cartridge's case taper, so a bigger magazine is a longer piece
    // of the same arc rather than a flatter one. A total angle would make the
    // sixty-round magazine straighter than the thirty, which is backwards.
    let turn = curve * step;
    let mut at = top;
    let mut h = half;
    let mut angle = 0.0f32;
    for i in 0..SECTIONS {
        angle += turn;
        let next = at + vec3(0.0, -angle.cos(), angle.sin()) * step;
        let nh = half * (1.0 - 0.03 * (i + 1) as f32);
        bar(out, at, h, next, nh, color);
        at = next;
        h = nh;
    }
    let end = curve * length;
    // The floorplate, wider than the body and darker, square to the end the
    // rounds leave by.
    bar(
        out,
        at,
        vec2(h.x * 1.20, h.y * 1.05),
        at + vec3(0.0, -end.cos(), end.sin()) * 0.011,
        vec2(h.x * 1.24, h.y * 1.05),
        BLACK,
    );
}

/// A drum magazine: a faceted cylinder lying across the weapon.
///
/// Built from `quad`s rather than from `bar`s because the cross-section is a
/// circle, which is the whole point of a drum — it has to read differently from
/// a box in the corner of the screen, and a box was what the first attempt at a
/// fifty-round magazine drew.
fn drum(out: &mut Vec<Tri>, at: Vec3, radius: f32, width: f32, color: Color) {
    const FACETS: usize = 8;
    let half = width * 0.5;
    let ring = |x: f32| -> Vec<Vec3> {
        (0..FACETS)
            .map(|i| {
                let a = i as f32 / FACETS as f32 * std::f32::consts::TAU;
                vec3(at.x + x, at.y + radius * a.cos(), at.z + radius * a.sin())
            })
            .collect()
    };
    let left = ring(-half);
    let right = ring(half);
    for i in 0..FACETS {
        let j = (i + 1) % FACETS;
        quad(out, left[i], right[i], right[j], left[j], color);
    }
    for (x, ring) in [(-half, &left), (half, &right)] {
        let hub = vec3(at.x + x, at.y, at.z);
        for i in 0..FACETS {
            let j = (i + 1) % FACETS;
            out.push(Tri::new(hub, ring[i], ring[j], color));
        }
    }
}

/// The workhorse: an axis-aligned box.
fn box3(out: &mut Vec<Tri>, min: Vec3, max: Vec3, color: Color) {
    let c = (min + max) * 0.5;
    let h = (max - min) * 0.5;
    bar(
        out,
        vec3(c.x, c.y, min.z),
        vec2(h.x, h.y),
        vec3(c.x, c.y, max.z),
        vec2(h.x, h.y),
        color,
    );
}

/// A wedge: rectangular base tapering to a point.
fn spire(out: &mut Vec<Tri>, base: Vec3, half: Vec2, tip: Vec3, color: Color) {
    let Some((r, u)) = cross_axes(tip - base) else {
        return;
    };
    let p = corners(base, half, r, u);
    quad(out, p[0], p[1], p[2], p[3], color);
    for i in 0..4 {
        out.push(Tri::new(p[i], p[(i + 1) % 4], tip, color));
    }
}

/// Front sight: a small tower carrying a blade whose tip touches the sight line.
fn front_sight(out: &mut Vec<Tri>, tower: Vec3, tower_top: Vec3, blade_z: f32) {
    box3(out, tower, tower_top, BLACK);
    spire(
        out,
        vec3(0.0, tower_top.y, blade_z),
        vec2(0.006, 0.007),
        vec3(0.0, 0.0, blade_z),
        BLACK,
    );
}

/// Rear sight: two posts either side of the sight line, leaving a notch.
fn rear_sight(out: &mut Vec<Tri>, min_z: f32, max_z: f32, gap: f32, top: f32) {
    box3(
        out,
        vec3(-gap - 0.016, top, min_z),
        vec3(-gap, 0.0, max_z),
        BLACK,
    );
    box3(
        out,
        vec3(gap, top, min_z),
        vec3(gap + 0.016, 0.0, max_z),
        BLACK,
    );
}

/// Balanced boxy carbine with a curved magazine.
fn rifle(out: &mut Vec<Tri>, d: &Def) {
    box3(
        out,
        vec3(-0.044, -0.123, -0.170),
        vec3(0.044, -0.030, 0.300),
        STEEL,
    );
    box3(
        out,
        vec3(-0.030, -0.030, -0.100),
        vec3(0.030, -0.021, 0.260),
        RAIL,
    );
    box3(
        out,
        vec3(-0.034, -0.110, 0.300),
        vec3(0.034, -0.036, 0.520),
        POLY,
    );
    box3(
        out,
        vec3(-0.015, -0.090, 0.520),
        vec3(0.015, -0.058, 0.800),
        STEEL,
    );
    box3(
        out,
        vec3(-0.021, -0.096, 0.800),
        vec3(0.021, -0.052, 0.866),
        BLACK,
    );
    front_sight(
        out,
        vec3(-0.013, -0.058, 0.762),
        vec3(0.013, -0.030, 0.798),
        0.780,
    );
    rear_sight(out, -0.060, -0.020, 0.010, -0.030);
    let _ = d;
    bar(
        out,
        vec3(0.0, -0.123, -0.090),
        vec2(0.014, 0.022),
        vec3(0.0, -0.205, -0.138),
        vec2(0.013, 0.020),
        POLY,
    );
    bar(
        out,
        vec3(0.0, -0.086, -0.170),
        vec2(0.020, 0.030),
        vec3(0.0, -0.120, -0.286),
        vec2(0.026, 0.050),
        POLY,
    );
    box3(
        out,
        vec3(-0.052, -0.072, -0.060),
        vec3(-0.036, -0.048, 0.040),
        RAIL,
    );
}

/// Fat magazine tube under the barrel and a wide wooden pump.
fn shotgun(out: &mut Vec<Tri>) {
    box3(
        out,
        vec3(-0.046, -0.130, -0.150),
        vec3(0.046, -0.030, 0.230),
        STEEL,
    );
    box3(
        out,
        vec3(-0.022, -0.116, 0.230),
        vec3(0.022, -0.072, 0.740),
        STEEL,
    );
    box3(
        out,
        vec3(-0.019, -0.168, 0.230),
        vec3(0.019, -0.120, 0.700),
        STEEL_DARK,
    );
    box3(
        out,
        vec3(-0.038, -0.180, 0.430),
        vec3(0.038, -0.112, 0.575),
        WOOD,
    );
    box3(
        out,
        vec3(-0.038, -0.186, 0.462),
        vec3(0.038, -0.176, 0.482),
        WOOD_DARK,
    );
    box3(
        out,
        vec3(-0.038, -0.186, 0.523),
        vec3(0.038, -0.176, 0.543),
        WOOD_DARK,
    );
    front_sight(
        out,
        vec3(-0.013, -0.072, 0.700),
        vec3(0.013, -0.048, 0.740),
        0.718,
    );
    rear_sight(out, -0.050, 0.010, 0.010, -0.030);
    bar(
        out,
        vec3(0.0, -0.096, -0.150),
        vec2(0.026, 0.040),
        vec3(0.0, -0.150, -0.290),
        vec2(0.032, 0.058),
        WOOD,
    );
    bar(
        out,
        vec3(0.0, -0.130, -0.055),
        vec2(0.017, 0.024),
        vec3(0.0, -0.215, -0.095),
        vec2(0.016, 0.022),
        WOOD_DARK,
    );
    box3(
        out,
        vec3(-0.060, -0.112, -0.020),
        vec3(-0.046, -0.040, 0.120),
        WOOD_DARK,
    );
}

/// Stubby machine pistol with a long magazine and a forward grip.
fn smg(out: &mut Vec<Tri>, d: &Def) {
    box3(
        out,
        vec3(-0.040, -0.120, -0.130),
        vec3(0.040, -0.028, 0.200),
        STEEL,
    );
    box3(
        out,
        vec3(-0.030, -0.104, 0.200),
        vec3(0.030, -0.048, 0.310),
        POLY,
    );
    box3(
        out,
        vec3(-0.014, -0.092, 0.310),
        vec3(0.014, -0.060, 0.400),
        STEEL_DARK,
    );
    let _ = d;
    front_sight(
        out,
        vec3(-0.012, -0.060, 0.362),
        vec3(0.012, -0.044, 0.396),
        0.380,
    );
    rear_sight(out, -0.080, -0.040, 0.010, -0.028);
    bar(
        out,
        vec3(0.0, -0.120, -0.075),
        vec2(0.015, 0.021),
        vec3(0.0, -0.185, -0.115),
        vec2(0.014, 0.019),
        POLY,
    );
    bar(
        out,
        vec3(0.0, -0.104, 0.170),
        vec2(0.013, 0.016),
        vec3(0.0, -0.186, 0.145),
        vec2(0.012, 0.015),
        POLY,
    );
    bar(
        out,
        vec3(0.0, -0.070, -0.130),
        vec2(0.020, 0.022),
        vec3(0.0, -0.104, -0.290),
        vec2(0.012, 0.016),
        POLY,
    );
    box3(
        out,
        vec3(-0.050, -0.078, -0.020),
        vec3(-0.036, -0.052, 0.060),
        RAIL,
    );
}

/// Long slim barrel, oversized scope and a folded bipod.
fn sniper(out: &mut Vec<Tri>, d: &Def) {
    box3(
        out,
        vec3(-0.038, -0.122, -0.200),
        vec3(0.038, -0.028, 0.235),
        STEEL,
    );
    box3(
        out,
        vec3(-0.013, -0.100, 0.235),
        vec3(0.013, -0.070, 1.000),
        STEEL,
    );
    // The scope tube, the objective bell and the post reticle are not part of
    // the rifle: they are the sight the player ends up looking *through*, and
    // everything about them is chosen to block the view. `sniper_scope` builds
    // them separately so the in-hand path can leave them out once the eye is
    // inside the tube — see `ViewModel::draw`.
    //
    // Box magazine, raked forward the way a detachable one sits in the well.
    let _ = d;
    bar(
        out,
        vec3(0.0, -0.090, -0.200),
        vec2(0.026, 0.038),
        vec3(0.0, -0.115, -0.300),
        vec2(0.028, 0.052),
        OLIVE,
    );
    bar(
        out,
        vec3(0.0, -0.122, -0.110),
        vec2(0.014, 0.020),
        vec3(0.0, -0.190, -0.150),
        vec2(0.013, 0.019),
        OLIVE,
    );
    bar(
        out,
        vec3(-0.026, -0.100, 0.640),
        vec2(0.008, 0.008),
        vec3(-0.062, -0.230, 0.560),
        vec2(0.007, 0.007),
        BLACK,
    );
    bar(
        out,
        vec3(0.026, -0.100, 0.640),
        vec2(0.008, 0.008),
        vec3(0.062, -0.230, 0.560),
        vec2(0.007, 0.007),
        BLACK,
    );
}

/// The sniper's scope: the tube, its objective bell and the post reticle.
///
/// Kept out of `sniper` because it is the one piece of any weapon that is meant
/// to be looked *through* rather than at. Every wall here is sized to enclose
/// the sight line, which is what makes the tube read as a scope from the hip —
/// and what makes it useless the moment the eye is at the eyepiece. So the
/// in-hand path drops it once the sights are up and the 2D scope in `hud` is
/// drawing the aperture, and keeps it the rest of the time.
fn sniper_scope(out: &mut Vec<Tri>) {
    // Scope tube: four walls around the sight line, so the middle stays open.
    box3(
        out,
        vec3(-0.040, 0.021, -0.140),
        vec3(0.040, 0.040, 0.300),
        BLACK,
    );
    box3(
        out,
        vec3(-0.040, -0.040, -0.140),
        vec3(0.040, -0.021, 0.300),
        BLACK,
    );
    box3(
        out,
        vec3(-0.040, -0.021, -0.140),
        vec3(-0.021, 0.021, 0.300),
        BLACK,
    );
    box3(
        out,
        vec3(0.021, -0.021, -0.140),
        vec3(0.040, 0.021, 0.300),
        BLACK,
    );
    // Objective bell, flaring outwards from the tube.
    bar(
        out,
        vec3(0.0, 0.0305, 0.300),
        vec2(0.031, 0.0095),
        vec3(0.0, 0.0355, 0.380),
        vec2(0.050, 0.0145),
        BLACK,
    );
    bar(
        out,
        vec3(0.0, -0.0305, 0.300),
        vec2(0.031, 0.0095),
        vec3(0.0, -0.0355, 0.380),
        vec2(0.050, 0.0145),
        BLACK,
    );
    bar(
        out,
        vec3(-0.0305, 0.0, 0.300),
        vec2(0.0095, 0.021),
        vec3(-0.0355, 0.0, 0.380),
        vec2(0.0145, 0.021),
        BLACK,
    );
    bar(
        out,
        vec3(0.0305, 0.0, 0.300),
        vec2(0.0095, 0.021),
        vec3(0.0355, 0.0, 0.380),
        vec2(0.0145, 0.021),
        BLACK,
    );
    // Post reticle, rising to the sight line inside the tube.
    spire(
        out,
        vec3(0.0, -0.021, 0.115),
        vec2(0.003, 0.004),
        vec3(0.0, 0.0, 0.115),
        BLACK,
    );
}

/// A carbine: the rifle's receiver shortened, a flat-top rail, a telescoping
/// stock and a barrel short enough to come up fast.
fn m4a1(out: &mut Vec<Tri>, _d: &Def) {
    box3(
        out,
        vec3(-0.036, -0.120, -0.190),
        vec3(0.036, -0.034, 0.200),
        STEEL,
    );
    box3(
        out,
        vec3(-0.030, -0.034, -0.140),
        vec3(0.030, -0.020, 0.235),
        RAIL,
    );
    box3(
        out,
        vec3(-0.028, -0.110, 0.200),
        vec3(0.028, -0.056, 0.415),
        POLY,
    );
    box3(
        out,
        vec3(-0.012, -0.098, 0.415),
        vec3(0.012, -0.064, 0.610),
        STEEL_DARK,
    );
    box3(
        out,
        vec3(-0.018, -0.102, 0.610),
        vec3(0.018, -0.060, 0.672),
        BLACK,
    );
    front_sight(
        out,
        vec3(-0.012, -0.056, 0.566),
        vec3(0.012, -0.032, 0.598),
        0.582,
    );
    rear_sight(out, -0.062, -0.034, 0.004, -0.030);
    bar(
        out,
        vec3(0.0, -0.120, -0.056),
        vec2(0.016, 0.024),
        vec3(0.0, -0.196, -0.104),
        vec2(0.014, 0.021),
        POLY,
    );
    // The buffer tube, and the stock sliding on it.
    bar(
        out,
        vec3(0.0, -0.086, -0.190),
        vec2(0.016, 0.020),
        vec3(0.0, -0.092, -0.296),
        vec2(0.014, 0.017),
        STEEL_DARK,
    );
    box3(
        out,
        vec3(-0.030, -0.128, -0.300),
        vec3(0.030, -0.050, -0.272),
        POLY,
    );
    box3(
        out,
        vec3(-0.008, -0.132, -0.268),
        vec3(0.008, -0.046, -0.206),
        RAIL,
    );
}

/// A rifle with a folding stock and a long railed handguard: the MK-7's round,
/// fired slower and harder.
#[allow(clippy::approx_constant)] // the folded stock's length, not a constant
fn scar(out: &mut Vec<Tri>, _d: &Def) {
    box3(
        out,
        vec3(-0.040, -0.126, -0.196),
        vec3(0.040, -0.030, 0.230),
        STEEL,
    );
    box3(
        out,
        vec3(-0.032, -0.030, -0.150),
        vec3(0.032, -0.014, 0.300),
        RAIL,
    );
    box3(
        out,
        vec3(-0.030, -0.116, 0.230),
        vec3(0.030, -0.052, 0.470),
        POLY,
    );
    box3(
        out,
        vec3(-0.030, -0.052, 0.230),
        vec3(0.030, -0.040, 0.470),
        RAIL,
    );
    box3(
        out,
        vec3(-0.013, -0.100, 0.470),
        vec3(0.013, -0.062, 0.700),
        STEEL_DARK,
    );
    box3(
        out,
        vec3(-0.019, -0.104, 0.700),
        vec3(0.019, -0.058, 0.762),
        BLACK,
    );
    front_sight(
        out,
        vec3(-0.013, -0.058, 0.652),
        vec3(0.013, -0.032, 0.686),
        0.668,
    );
    rear_sight(out, -0.066, -0.030, 0.006, -0.026);
    bar(
        out,
        vec3(0.0, -0.126, -0.052),
        vec2(0.017, 0.026),
        vec3(0.0, -0.204, -0.104),
        vec2(0.015, 0.022),
        POLY,
    );
    // The folding stock: a hinge block, then a slab that folds against the side.
    box3(
        out,
        vec3(-0.024, -0.106, -0.230),
        vec3(0.024, -0.056, -0.196),
        STEEL_DARK,
    );
    box3(
        out,
        vec3(-0.026, -0.120, -0.318),
        vec3(0.026, -0.048, -0.230),
        POLY,
    );
    box3(
        out,
        vec3(-0.028, -0.118, -0.322),
        vec3(0.028, -0.050, -0.310),
        BLACK,
    );
    // A cheek piece, which is most of what tells it from the carbine.
    box3(
        out,
        vec3(-0.022, -0.048, -0.300),
        vec3(0.022, -0.020, -0.230),
        POLY,
    );
}

/// A battle rifle: a long receiver, a wooden stock and a carry handle, which is
/// what the full-power rifles were.
fn fal(out: &mut Vec<Tri>, _d: &Def) {
    box3(
        out,
        vec3(-0.038, -0.130, -0.230),
        vec3(0.038, -0.030, 0.250),
        STEEL,
    );
    box3(
        out,
        vec3(-0.026, -0.030, -0.070),
        vec3(0.026, -0.010, 0.130),
        STEEL_DARK,
    );
    box3(
        out,
        vec3(-0.026, -0.116, 0.250),
        vec3(0.026, -0.056, 0.520),
        POLY,
    );
    box3(
        out,
        vec3(-0.014, -0.102, 0.520),
        vec3(0.014, -0.062, 0.790),
        STEEL_DARK,
    );
    box3(
        out,
        vec3(-0.021, -0.106, 0.790),
        vec3(0.021, -0.058, 0.856),
        BLACK,
    );
    front_sight(
        out,
        vec3(-0.014, -0.060, 0.742),
        vec3(0.014, -0.034, 0.776),
        0.758,
    );
    // The carry handle sits where a rear sight would.
    box3(
        out,
        vec3(-0.008, -0.048, -0.070),
        vec3(0.008, -0.026, 0.100),
        STEEL_DARK,
    );
    bar(
        out,
        vec3(0.0, -0.130, -0.060),
        vec2(0.016, 0.026),
        vec3(0.0, -0.212, -0.116),
        vec2(0.014, 0.022),
        WOOD_DARK,
    );
    // A solid wooden stock, the thing an issued rifle no longer has.
    bar(
        out,
        vec3(0.0, -0.086, -0.230),
        vec2(0.030, 0.044),
        vec3(0.0, -0.104, -0.400),
        vec2(0.032, 0.050),
        WOOD,
    );
    box3(
        out,
        vec3(-0.032, -0.150, -0.408),
        vec3(0.032, -0.058, -0.396),
        WOOD_DARK,
    );
}

/// A roller-locked machine pistol, drawn from the reference photograph: a rolled
/// receiver with a flat top, the cocking-handle tube riding above the barrel, a
/// slim slotted handguard, the fixed polymer stock of the A2, and the long
/// forward-curving thirty-round magazine — which comes from `well`, not from
/// here, because a magazine in this game is a fitting.
fn mp5(out: &mut Vec<Tri>, _d: &Def) {
    // The receiver: round, with the flat top the sights sit on.
    tube(
        out,
        vec3(0.0, -0.084, -0.150),
        0.042,
        vec3(0.0, -0.084, 0.170),
        0.042,
        STEEL,
    );
    box3(
        out,
        vec3(-0.036, -0.050, -0.150),
        vec3(0.036, -0.036, 0.186),
        STEEL,
    );
    // The cocking-handle tube, forward along the left of the barrel, with the
    // handle at its front end. It is the one line that says MP5 at a glance.
    tube(
        out,
        vec3(-0.026, -0.040, 0.120),
        0.014,
        vec3(-0.026, -0.040, 0.372),
        0.012,
        STEEL_DARK,
    );
    box3(
        out,
        vec3(-0.042, -0.052, 0.356),
        vec3(-0.022, -0.028, 0.392),
        BLACK,
    );
    // The handguard: slim, and slotted along its underside.
    tube(
        out,
        vec3(0.0, -0.092, 0.170),
        0.034,
        vec3(0.0, -0.092, 0.372),
        0.030,
        POLY,
    );
    for i in 0..3 {
        let z = 0.208 + i as f32 * 0.052;
        box3(
            out,
            vec3(-0.030, -0.118, z),
            vec3(0.030, -0.062, z + 0.014),
            STEEL_DARK,
        );
    }
    // The barrel, and the muzzle out of the front of the handguard.
    tube(
        out,
        vec3(0.0, -0.086, 0.372),
        0.014,
        vec3(0.0, -0.086, 0.452),
        0.012,
        STEEL_DARK,
    );
    // The front sight sits on the cocking tube, not on the barrel.
    box3(
        out,
        vec3(-0.012, -0.048, 0.336),
        vec3(0.012, -0.024, 0.368),
        BLACK,
    );
    spire(
        out,
        vec3(0.0, -0.024, 0.352),
        vec2(0.005, 0.006),
        vec3(0.0, 0.0, 0.352),
        BLACK,
    );
    rear_sight(out, -0.116, -0.098, 0.007, -0.036);
    // The trigger housing, the guard and the grip.
    box3(
        out,
        vec3(-0.030, -0.130, -0.062),
        vec3(0.030, -0.096, 0.040),
        POLY,
    );
    bar(
        out,
        vec3(0.0, -0.150, -0.010),
        vec2(0.006, 0.008),
        vec3(0.0, -0.150, 0.052),
        vec2(0.006, 0.008),
        STEEL,
    );
    bar(
        out,
        vec3(0.0, -0.150, 0.052),
        vec2(0.006, 0.008),
        vec3(0.0, -0.126, 0.036),
        vec2(0.006, 0.008),
        STEEL,
    );
    bar(
        out,
        vec3(0.0, -0.126, -0.036),
        vec2(0.016, 0.024),
        vec3(0.0, -0.204, -0.086),
        vec2(0.014, 0.021),
        POLY,
    );
    // The A2 stock: a solid polymer wedge, tallest at the butt, with the sling
    // loop cut through it.
    bar(
        out,
        vec3(0.0, -0.088, -0.150),
        vec2(0.030, 0.044),
        vec3(0.0, -0.118, -0.288),
        vec2(0.034, 0.062),
        POLY,
    );
    box3(
        out,
        vec3(-0.036, -0.190, -0.302),
        vec3(0.036, -0.046, -0.288),
        BLACK,
    );
    box3(
        out,
        vec3(-0.026, -0.146, -0.276),
        vec3(0.026, -0.106, -0.244),
        STEEL_DARK,
    );
}

/// A boxier machine pistol: a squared receiver, a short slotted handguard and a
/// stock that folds to the side.
fn mpx(out: &mut Vec<Tri>, _d: &Def) {
    box3(
        out,
        vec3(-0.036, -0.124, -0.160),
        vec3(0.036, -0.030, 0.196),
        STEEL,
    );
    box3(
        out,
        vec3(-0.032, -0.030, -0.130),
        vec3(0.032, -0.018, 0.210),
        RAIL,
    );
    box3(
        out,
        vec3(-0.028, -0.110, 0.196),
        vec3(0.028, -0.052, 0.352),
        POLY,
    );
    for i in 0..3 {
        let z = 0.216 + i as f32 * 0.046;
        box3(
            out,
            vec3(-0.030, -0.106, z),
            vec3(0.030, -0.078, z + 0.028),
            STEEL_DARK,
        );
    }
    box3(
        out,
        vec3(-0.012, -0.098, 0.352),
        vec3(0.012, -0.064, 0.430),
        STEEL_DARK,
    );
    box3(
        out,
        vec3(-0.017, -0.102, 0.430),
        vec3(0.017, -0.060, 0.470),
        BLACK,
    );
    front_sight(
        out,
        vec3(-0.012, -0.058, 0.388),
        vec3(0.012, -0.036, 0.416),
        0.402,
    );
    rear_sight(out, -0.062, -0.030, 0.008, -0.026);
    bar(
        out,
        vec3(0.0, -0.124, -0.044),
        vec2(0.016, 0.025),
        vec3(0.0, -0.202, -0.092),
        vec2(0.014, 0.022),
        POLY,
    );
    box3(
        out,
        vec3(-0.022, -0.104, -0.196),
        vec3(0.022, -0.052, -0.160),
        STEEL_DARK,
    );
    box3(
        out,
        vec3(-0.024, -0.112, -0.286),
        vec3(0.024, -0.046, -0.196),
        POLY,
    );
}

/// A marksman's rifle: a heavy barrel, a long rail over it, a cheek riser and
/// the short bipod stubs a rifle of this length has.
fn m110(out: &mut Vec<Tri>, _d: &Def) {
    box3(
        out,
        vec3(-0.038, -0.132, -0.240),
        vec3(0.038, -0.030, 0.220),
        STEEL,
    );
    box3(
        out,
        vec3(-0.030, -0.030, -0.190),
        vec3(0.030, -0.012, 0.300),
        RAIL,
    );
    box3(
        out,
        vec3(-0.030, -0.122, 0.220),
        vec3(0.030, -0.056, 0.520),
        POLY,
    );
    box3(
        out,
        vec3(-0.017, -0.108, 0.520),
        vec3(0.017, -0.062, 0.900),
        STEEL_DARK,
    );
    box3(
        out,
        vec3(-0.025, -0.114, 0.900),
        vec3(0.025, -0.056, 0.980),
        BLACK,
    );
    front_sight(
        out,
        vec3(-0.017, -0.062, 0.836),
        vec3(0.017, -0.034, 0.876),
        0.856,
    );
    bar(
        out,
        vec3(0.0, -0.132, -0.070),
        vec2(0.017, 0.026),
        vec3(0.0, -0.208, -0.126),
        vec2(0.015, 0.022),
        POLY,
    );
    bar(
        out,
        vec3(0.0, -0.090, -0.240),
        vec2(0.028, 0.042),
        vec3(0.0, -0.102, -0.402),
        vec2(0.030, 0.046),
        POLY,
    );
    box3(
        out,
        vec3(-0.024, -0.052, -0.360),
        vec3(0.024, -0.028, -0.262),
        POLY,
    );
    box3(
        out,
        vec3(-0.030, -0.150, -0.410),
        vec3(0.030, -0.056, -0.398),
        BLACK,
    );
    for side in [-1.0f32, 1.0] {
        bar(
            out,
            vec3(side * 0.024, -0.106, 0.560),
            vec2(0.006, 0.006),
            vec3(side * 0.062, -0.216, 0.470),
            vec2(0.005, 0.005),
            BLACK,
        );
    }
}

/// A bolt-action: a long barrel out of a full wooden stock, with the bolt handle
/// standing out of the side, which is the one thing that has to read.
fn mosin(out: &mut Vec<Tri>, _d: &Def) {
    box3(
        out,
        vec3(-0.030, -0.124, -0.240),
        vec3(0.030, -0.048, 0.260),
        STEEL,
    );
    box3(
        out,
        vec3(-0.026, -0.112, 0.260),
        vec3(0.026, -0.060, 0.560),
        WOOD,
    );
    box3(
        out,
        vec3(-0.011, -0.096, 0.560),
        vec3(0.011, -0.070, 1.040),
        STEEL_DARK,
    );
    box3(
        out,
        vec3(-0.017, -0.100, 1.040),
        vec3(0.017, -0.062, 1.092),
        BLACK,
    );
    front_sight(
        out,
        vec3(-0.011, -0.066, 0.988),
        vec3(0.011, -0.040, 1.022),
        1.004,
    );
    rear_sight(out, -0.052, -0.034, 0.006, -0.030);
    // The bolt: a straight handle out to the right and the knob on its end.
    bar(
        out,
        vec3(0.022, -0.076, 0.096),
        vec2(0.007, 0.007),
        vec3(0.062, -0.074, 0.056),
        vec2(0.006, 0.006),
        STEEL_DARK,
    );
    box3(
        out,
        vec3(0.058, -0.086, 0.040),
        vec3(0.074, -0.064, 0.060),
        RAIL,
    );
    bar(
        out,
        vec3(0.0, -0.124, -0.240),
        vec2(0.026, 0.040),
        vec3(0.0, -0.150, -0.400),
        vec2(0.030, 0.048),
        WOOD,
    );
    box3(
        out,
        vec3(-0.032, -0.198, -0.412),
        vec3(0.032, -0.140, -0.398),
        WOOD_DARK,
    );
}

/// Bulky support weapon with a drum magazine.
fn lmg(out: &mut Vec<Tri>) {
    box3(
        out,
        vec3(-0.056, -0.150, -0.190),
        vec3(0.056, -0.030, 0.330),
        STEEL,
    );
    box3(
        out,
        vec3(-0.038, -0.030, -0.150),
        vec3(0.038, -0.019, 0.290),
        RAIL,
    );
    box3(
        out,
        vec3(-0.021, -0.116, 0.330),
        vec3(0.021, -0.074, 0.800),
        STEEL,
    );
    box3(
        out,
        vec3(-0.028, -0.124, 0.800),
        vec3(0.028, -0.066, 0.885),
        BLACK,
    );
    front_sight(
        out,
        vec3(-0.013, -0.074, 0.742),
        vec3(0.013, -0.050, 0.780),
        0.760,
    );
    rear_sight(out, -0.070, -0.030, 0.010, -0.030);
    // The belt box, hung under the receiver.
    belt_box(
        out,
        vec3(0.0, -0.188, 0.025),
        vec3(0.072, 0.050, 0.085),
        STEEL_DARK,
    );
    bar(
        out,
        vec3(0.0, -0.150, -0.115),
        vec2(0.016, 0.022),
        vec3(0.0, -0.225, -0.160),
        vec2(0.015, 0.020),
        POLY,
    );
    bar(
        out,
        vec3(0.0, -0.095, -0.190),
        vec2(0.028, 0.042),
        vec3(0.0, -0.135, -0.330),
        vec2(0.032, 0.056),
        POLY,
    );
    bar(
        out,
        vec3(-0.030, -0.126, 0.620),
        vec2(0.009, 0.009),
        vec3(-0.070, -0.235, 0.545),
        vec2(0.008, 0.008),
        BLACK,
    );
    bar(
        out,
        vec3(0.030, -0.126, 0.620),
        vec2(0.009, 0.009),
        vec3(0.070, -0.235, 0.545),
        vec2(0.008, 0.008),
        BLACK,
    );
}

/// Compact sidearm: a short slide, a deep raked grip and no stock at all, so it
/// reads as much smaller than any of the long guns.
fn pistol(out: &mut Vec<Tri>) {
    box3(
        out,
        vec3(-0.019, -0.052, -0.040),
        vec3(0.019, -0.020, 0.130),
        STEEL,
    );
    box3(
        out,
        vec3(-0.017, -0.070, -0.012),
        vec3(0.017, -0.052, 0.118),
        STEEL_DARK,
    );
    // Grip, raked back the way a sidearm is held.
    bar(
        out,
        vec3(0.0, -0.062, -0.032),
        vec2(0.018, 0.026),
        vec3(0.0, -0.148, -0.082),
        vec2(0.016, 0.024),
        POLY,
    );
    bar(
        out,
        vec3(0.0, -0.148, -0.082),
        vec2(0.016, 0.024),
        vec3(0.0, -0.164, -0.094),
        vec2(0.015, 0.022),
        BLACK,
    );
    // Trigger guard: front strap, floor, then the grip closes the back.
    bar(
        out,
        vec3(0.0, -0.068, 0.008),
        vec2(0.006, 0.009),
        vec3(0.0, -0.098, 0.016),
        vec2(0.006, 0.009),
        STEEL_DARK,
    );
    bar(
        out,
        vec3(0.0, -0.098, 0.016),
        vec2(0.006, 0.008),
        vec3(0.0, -0.086, -0.034),
        vec2(0.006, 0.008),
        STEEL_DARK,
    );
    box3(
        out,
        vec3(-0.004, -0.088, -0.002),
        vec3(0.004, -0.070, 0.012),
        BLACK,
    );
    front_sight(
        out,
        vec3(-0.008, -0.020, 0.100),
        vec3(0.008, -0.008, 0.132),
        0.114,
    );
    rear_sight(out, -0.048, -0.016, 0.005, -0.020);
}

/// Combat knife, point forward: a wrapped handle, a guard, and a blade that
/// tapers up to a point sitting on the sight line.
fn knife(out: &mut Vec<Tri>) {
    bar(
        out,
        vec3(0.0, -0.082, -0.134),
        vec2(0.014, 0.018),
        vec3(0.0, -0.050, -0.012),
        vec2(0.013, 0.017),
        POLY,
    );
    box3(
        out,
        vec3(-0.016, -0.090, -0.150),
        vec3(0.016, -0.072, -0.130),
        STEEL_DARK,
    );
    box3(
        out,
        vec3(-0.026, -0.060, -0.014),
        vec3(0.026, -0.038, 0.004),
        STEEL_DARK,
    );
    // Blade stock, climbing forward from the guard...
    bar(
        out,
        vec3(0.0, -0.042, 0.000),
        vec2(0.004, 0.014),
        vec3(0.0, -0.014, 0.100),
        vec2(0.003, 0.011),
        STEEL,
    );
    // ...to the point, which ends up on the sight line.
    spire(
        out,
        vec3(0.0, -0.014, 0.100),
        vec2(0.003, 0.011),
        vec3(0.0, 0.0, 0.212),
        STEEL,
    );
    // Spine, catching the key light along the back of the blade.
    bar(
        out,
        vec3(0.0, -0.030, 0.010),
        vec2(0.0046, 0.0035),
        vec3(0.0, -0.008, 0.096),
        vec2(0.0036, 0.0035),
        RAIL,
    );
}

/// The weapon as the armory draws it: everything the kind is made of, the
/// sights it is looked through included.
fn build(kind: WeaponKind) -> Vec<Tri> {
    build_assembled(kind, true)
}

/// The weapon as a build assembles it. `issued_mag` is false when a magazine
/// part is fitted, because that part *is* the magazine rather than an addition
/// to it.
fn build_assembled(kind: WeaponKind, issued_mag: bool) -> Vec<Tri> {
    let mut out = build_held(kind, issued_mag);
    if kind == WeaponKind::Sniper {
        sniper_scope(&mut out);
    }
    out
}

/// Whether a build fits a magazine — the one slot whose part replaces geometry
/// instead of adding to it. See `build_assembled`.
fn fits_a_magazine(build: &WeaponBuild) -> bool {
    build
        .entries
        .iter()
        .any(|e| e.slot == crate::weapon::SlotId("magazine"))
}

/// The weapon as the hands carry it: everything `build` has except the sniper's/// scope, which `ViewModel::draw` puts back while the sights are down.
fn build_held(kind: WeaponKind, with_magazine: bool) -> Vec<Tri> {
    let d = def(kind);
    let mut out = Vec::new();
    match kind {
        WeaponKind::Rifle => rifle(&mut out, &d),
        WeaponKind::Shotgun => shotgun(&mut out),
        WeaponKind::Smg => smg(&mut out, &d),
        WeaponKind::Sniper => sniper(&mut out, &d),
        WeaponKind::Lmg => lmg(&mut out),
        WeaponKind::Pistol => pistol(&mut out),
        WeaponKind::Knife => knife(&mut out),
        WeaponKind::M4a1 => m4a1(&mut out, &d),
        WeaponKind::Scar => scar(&mut out, &d),
        WeaponKind::Fal => fal(&mut out, &d),
        WeaponKind::Mp5 => mp5(&mut out, &d),
        WeaponKind::Mpx => mpx(&mut out, &d),
        WeaponKind::M110 => m110(&mut out, &d),
        WeaponKind::Mosin => mosin(&mut out, &d),
    }
    // The magazine three of these weapons hang under the well is the one piece
    // of a weapon a fitting *replaces* rather than adds to, so it is drawn apart
    // from the body and left off when something is fitted in its place. The
    // others continue theirs instead — see `kit_into`.
    if with_magazine && d.well_len > 0.0 {
        issued_magazine(&mut out, &d);
    }
    out
}

/// The magazine the weapon is issued with, drawn from the same numbers a
/// capacity fitting starts from. It has to be one function: a fitting that
/// started from numbers the body did not use is a fitting that does not meet the
/// body, which is exactly what the first version of this did.
fn issued_magazine(out: &mut Vec<Tri>, d: &Def) {
    magazine(out, d.well, d.well_len, d.well_half, d.well_curve, POLY);
}

/// The fingers, closed round whatever is at `at`: the fist, the knuckles and the
/// thumb, and nothing else.
///
/// `side` is +1 for the right hand and -1 for the left, and is the only thing
/// that differs between the two. `fingers` is how much of what it is holding the
/// fingers close over: 1 for a weapon's grip, which is a shape a hand is made
/// for, and less for a grenade. A fist the size of the thing inside it leaves the
/// player looking at their own knuckles — which is what the four casings were:
/// four colours of fist, each with the top two centimetres of a grenade on it, so
/// the shapes that tell the four of them apart were the one part of them nobody
/// could see.
///
/// Apart from the arm because the casings sheet draws this: what a player sees of
/// a grenade in their hand is the fingers and the thing, and the arm is off the
/// bottom of the frame.
fn fist(out: &mut Vec<Tri>, at: Vec3, side: f32, fingers: f32) {
    // Fist, wrapped round whatever it is holding.
    box3(
        out,
        at + vec3(-0.036, -0.050, -0.044) * fingers,
        at + vec3(0.036, 0.034, 0.044) * fingers,
        GLOVE,
    );
    // Knuckles on top, so it reads as a hand rather than a cube.
    box3(
        out,
        at + vec3(-0.030, 0.030, -0.028) * fingers,
        at + vec3(0.030, 0.046, 0.028) * fingers,
        GLOVE_DARK,
    );
    // Thumb, folded over the near side of the grip.
    box3(
        out,
        at + vec3(side * 0.020, -0.014, 0.024) * fingers,
        at + vec3(side * 0.047, 0.024, 0.062) * fingers,
        GLOVE_DARK,
    );
}

/// One gloved hand closed around `at`, with the forearm running back and
/// outboard. See `fist` for `side` and `fingers`.
fn hand(out: &mut Vec<Tri>, at: Vec3, side: f32, fingers: f32) {
    fist(out, at, side, fingers);
    // The forearm leaves the frame low and outboard, which is where an arm
    // holding a weapon at the hip actually goes. It stops well short of the eye
    // so that no corner of it crosses the near plane — one vertex behind the
    // eye drops the whole triangle, and the arm would wink out mid-swing.
    let wrist = at + vec3(side * 0.008, -0.040, -0.048);
    let elbow = wrist + vec3(side * 0.115, -0.165, -0.105);
    bar(
        out,
        wrist,
        vec2(0.038, 0.042),
        elbow,
        vec2(0.052, 0.050),
        SLEEVE,
    );
}

fn grip_hand(kind: WeaponKind) -> Vec<Tri> {
    let mut out = Vec::new();
    hand(&mut out, def(kind).grip, 1.0, 1.0);
    out
}

/// The hand on the foregrip, or nothing where the weapon has no place to put
/// one — a pistol and a knife are held in one hand.
fn support_hand(kind: WeaponKind) -> Vec<Tri> {
    let mut out = Vec::new();
    if let Some(at) = def(kind).support {
        hand(&mut out, at, -1.0, 1.0);
    }
    out
}

/// How big the thing in the throwing hand is: the half-extent its profile is a
/// fraction of. Fourteen centimetres top to bottom, which is a *large* grenade —
/// the same object `grenade::casing` puts in the air at the size collision gives
/// it (17 cm), drawn smaller than that and larger than a hand holds it.
///
/// Oversized on purpose, and the one place in this file that is. A casing drawn
/// at the size a hand really holds one is small enough that the hand is the
/// bigger object, and what a player reads at a glance is then the hand. If it
/// ever wants to be honest, it is this one number.
const HELD_HALF: f32 = 0.072;
/// The casing's facet count, taken from the world's own constant rather than
/// written again: the openings a casing breathes through sit in the middle of
/// those facets (`ports`), so two counts that disagreed would put holes on the
/// corners of the wall instead of on its faces.
const CASING_FACETS: usize = crate::grenade::CASING_SIDES;
/// And how big an opening in a casing's wall is, from the same place.
const PORT_HALF: f32 = crate::grenade::PORT_HALF;
/// How far above the point the throw comes out of the thing in the hand sits.
/// Small, because that point is where the arc starts and where the body leaves
/// the hand: the line has to come off the thing rather than out of the middle of
/// it.
const HELD_LIFT: f32 = 0.022;

/// A throwable in the throwing hand: the thing, and the hand *under* it.
///
/// The thing is the one `grenade::casing_shape` describes — an M67 is a ball
/// with a fuze in the top of it, an M18 a tin can, an M84 a cylinder of
/// openings, a bottle a bottle — drawn here out of the viewmodel's own
/// primitives, because one is a `Mesh` in the world and this is a triangle list
/// in the view. The two cannot share code; they share the *shape*, which is
/// what keeps the grenade in the hand and the grenade in the air the same
/// object. The fallback when they disagree is a player throwing one thing and
/// watching another land.
fn held_throwable(out: &mut Vec<Tri>, kind: GrenadeKind, at: Vec3) {
    let shape = crate::grenade::casing_shape(kind);
    let centre = at + vec3(0.0, HELD_LIFT, 0.0);
    held_casing(out, kind, centre);
    throw_hand(
        out,
        centre,
        1.0,
        shape.widest() * HELD_HALF,
        shape.bottom() * HELD_HALF,
    );
}

/// The arm that throws, with nothing in it: what is left of the motion after the
/// thing has gone. Drawn while that arm is still on its way out of the frame, so
/// the throw ends with a hand falling away rather than with a hand that was there
/// one frame and not the next.
///
/// Sized off the frag, since there is nothing in it to be sized off: the hand is a
/// cupped shape either way, and the difference between one grenade's cradle and
/// another's is a centimetre, for a fifth of a second, while it leaves the frame.
fn throwing_hand(out: &mut Vec<Tri>, at: Vec3) {
    let shape = crate::grenade::casing_shape(GrenadeKind::Frag);
    throw_hand(
        out,
        at + vec3(0.0, HELD_LIFT, 0.0),
        1.0,
        shape.widest() * HELD_HALF,
        shape.bottom() * HELD_HALF,
    );
}

/// The hand that holds a throwable, drawn around the thing's own centre `at`: the
/// fingers as a cradle under its body, the back of the hand below them, the wrist
/// and the forearm from there.
///
/// `r` is the thing's widest radius and `bottom` how far its body runs below `at`,
/// both in view metres, so the cradle sits *against* whatever is in it rather than
/// at a size of its own. Everything here is at or below `bottom` — the one place
/// the body cannot be — so no part of the hand is inside the thing, and the test
/// `no_part_of_the_hand_is_inside_the_throwable` is what holds that.
///
/// It used to be a fist closed *round* the thing, which is what a hand really
/// does, and it is exactly why the four casings could not be told apart: a ball
/// sunk into a mitten is a mitten with a dark shape on it — "投掷物和手重合了，所以
/// 我看不出来", as it was put. Holding it from below is also what the throw wants.
/// Fingers wrapped round a body have to be drawn *through* it to open; fingers
/// under it simply let go.
fn throw_hand(out: &mut Vec<Tri>, at: Vec3, side: f32, r: f32, bottom: f32) {
    // The fingers: three bars across the underside, back to front, so the thing
    // rests on the middle joints of them.
    let top = bottom - 0.002;
    for (i, z) in [-0.55f32, 0.0, 0.55].iter().enumerate() {
        // The front finger is the shortest, which is what stops three bars
        // reading as a shelf.
        let half = r * (0.52 - 0.05 * i as f32);
        let y = top - 0.010 - 0.004 * (i as f32 % 2.0);
        tube(
            out,
            at + vec3(-half, y, z * r),
            0.011,
            at + vec3(half, y, z * r),
            0.011,
            GLOVE,
        );
    }
    // The back of the hand under them, and the thumb along the near side of it:
    // the two things that make this a hand and not a bracket. The thumb stays
    // *below* the body like everything else — an earlier one curled up the side
    // of it and the test caught it inside a smoke grenade, whose wall is nearly
    // at its full width by the time it reaches its own base.
    box3(
        out,
        at + vec3(-0.60 * r, top - 0.056, -0.70 * r),
        at + vec3(0.60 * r, top - 0.014, 0.70 * r),
        GLOVE,
    );
    tube(
        out,
        at + vec3(0.50 * r, top - 0.028, -0.30 * r),
        0.011,
        at + vec3(0.86 * r, top - 0.008, 0.42 * r),
        0.009,
        GLOVE_DARK,
    );
    // The forearm leaves the frame low and outboard, which is where an arm held
    // out in front of the body goes. It stops well short of the eye so that no
    // corner of it crosses the near plane — one vertex behind the eye drops the
    // whole triangle, and the arm would wink out mid-swing.
    let wrist = at + vec3(side * 0.012, top - 0.046, -0.34 * r);
    let elbow = wrist + vec3(side * 0.115, -0.165, -0.105);
    bar(
        out,
        wrist,
        vec2(0.036, 0.040),
        elbow,
        vec2(0.052, 0.050),
        SLEEVE,
    );
}

/// The throwable itself, at the size a hand holds it.
fn held_casing(out: &mut Vec<Tri>, kind: GrenadeKind, at: Vec3) {
    let shape = crate::grenade::casing_shape(kind);
    let color = kind.color();
    lathe(out, at, HELD_HALF, shape.profile, color);
    for &(wall, y) in shape.ports {
        ports(out, at, HELD_HALF, wall, y);
    }
    let of = |x: f32, y: f32| at + vec3(x * HELD_HALF, y * HELD_HALF, 0.0);
    // Thicknesses in fractions of `HELD_HALF` and not in metres: the lever and
    // the rag are parts of the object, so they have to shrink with it. Written
    // in metres they stay the world's size, and a hand-sized grenade grows a
    // full-sized spoon.
    //
    // `tube` and not `bar`, because a tube's cross-section is a beam's: a `bar`
    // with the same half-extents is a *box*, and a box's corners stand out
    // √2 further than a beam's do. Two renderers drawing one object should
    // disagree about nothing, and that one showed up as 1.5 cm of spoon.
    if let Some((x0, y0, x1, y1)) = shape.lever {
        let r = LEVER_HALF * HELD_HALF;
        tube(out, of(x0, y0), r, of(x1, y1), r, STEEL);
    }
    if let Some((r, y)) = shape.wick {
        let (bx, by) = *shape.profile.last().unwrap_or(&(0.15, 0.8));
        // Two strands rather than one: a rag in a neck is not a rod, and a rod
        // is what the first bottle would have had.
        let ra = WICK_HALF * HELD_HALF;
        let rb = WICK_HALF * WICK_STRAND * HELD_HALF;
        tube(out, of(bx, by), ra, of(r, y), ra, RAG);
        tube(
            out,
            of(bx - 0.04, by - 0.10),
            rb,
            of(r + 0.07, y - 0.09),
            rb,
            RAG,
        );
    }
}

/// A body of revolution about the local up axis, `profile` bottom first in
/// fractions of `half`, closed at both ends.
///
/// The view's copy of `grenade::lathe`: the primitives differ (a `MeshBuilder`
/// quad in the world, a `Tri` here) and the profiles do not, which is the point
/// of `casing_shape`. Winding is left alone — `Tri::shaded` flips the normal
/// towards the eye, so a face of this is lit from whichever side is looked at.
fn lathe(out: &mut Vec<Tri>, at: Vec3, half: f32, profile: &[(f32, f32)], color: Color) {
    let step = TAU / CASING_FACETS as f32;
    let ring = |i: usize, r: f32, y: f32| {
        let a = step * i as f32;
        at + vec3(a.cos() * r * half, y * half, a.sin() * r * half)
    };
    for w in profile.windows(2) {
        let ((r0, y0), (r1, y1)) = (w[0], w[1]);
        for i in 0..CASING_FACETS {
            let j = (i + 1) % CASING_FACETS;
            quad(
                out,
                ring(i, r0, y0),
                ring(j, r0, y0),
                ring(j, r1, y1),
                ring(i, r1, y1),
                color,
            );
        }
    }
    for (r, y) in [profile[0], profile[profile.len() - 1]] {
        let hub = at + Vec3::Y * (y * half);
        for k in 0..CASING_FACETS {
            let j = (k + 1) % CASING_FACETS;
            out.push(Tri::new(hub, ring(k, r, y), ring(j, r, y), color));
        }
    }
}

/// A row of openings around the thing's wall: one per facet, lying in the wall's
/// own plane. The view's copy of `grenade::ports`, and dark for the same reason
/// — what is behind the wall is a hole, not a panel. The openings are
/// `PORT_HALF` big, the world's own number: a hole that grew on the way into the
/// hand would be a second kind of hole.
fn ports(out: &mut Vec<Tri>, at: Vec3, half: f32, wall: f32, y: f32) {
    let step = TAU / CASING_FACETS as f32;
    let r = (wall * (std::f32::consts::PI / CASING_FACETS as f32).cos() + 0.006) * half;
    let (h, v) = (PORT_HALF * half, PORT_HALF * 1.15 * half);
    for i in 0..CASING_FACETS {
        // Mid-facet azimuth, which is where a facet's own surface plane is.
        let a = step * (i as f32 + 0.5);
        let out_w = vec3(a.cos(), 0.0, a.sin());
        let side = vec3(-a.sin(), 0.0, a.cos());
        let mid = at + out_w * r + Vec3::Y * (y * half);
        quad(
            out,
            mid - side * h - Vec3::Y * v,
            mid + side * h - Vec3::Y * v,
            mid + side * h + Vec3::Y * v,
            mid - side * h + Vec3::Y * v,
            CASE,
        );
    }
}

/// Eased 0..1 ramp, flat at both ends.
fn smooth(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Resting and aimed poses for one weapon. `aim` lies on the sight line, so its
/// `x` and `y` must stay exactly zero for the sights to meet the crosshair.
struct Def {
    hip: Vec3,
    /// Yaw, pitch and roll at the hip; the aim pose is square with the eye.
    rotation: Vec3,
    aim: Vec3,
    /// Where the flash appears, in local space.
    muzzle: Vec3,
    /// Where the firing hand closes on the grip, in local space.
    grip: Vec3,
    /// Where the support hand closes on the fore-end. `None` for the weapons
    /// that are held in one hand, where a second hand would have nowhere to be.
    support: Option<Vec3>,
    /// Where an optic is bolted on: the centre of the sight it adds.
    optic: Vec3,
    /// Where a magazine extension grows from — the centre of the box an
    /// extended magazine adds — and its half-extents. Authored per weapon,
    /// because a box magazine, a tube and a drum do not extend alike.
    mag: Vec3,
    mag_ext: Vec3,
    /// Where the magazine is issued: the top of it at the mouth of the well, its
    /// half-extents there, how far it runs down and how far it curves. A
    /// capacity fitting *is* this magazine, longer or wider, so it is drawn from
    /// these numbers — including the curve, which a fitting that restarted its
    /// own arc could never match. Zero `well_len` means the weapon has no
    /// replaceable box magazine and its fittings continue something else.
    well: Vec3,
    well_half: Vec2,
    well_len: f32,
    well_curve: f32,
    /// Where a drum hangs, if the weapon takes one. A drum is not a magazine
    /// continued — it is a different object in a different place: under the
    /// receiver, ahead of the trigger, not out at the end of a tube where the
    /// shotgun's well anchor lives.
    drum: Vec3,
    /// Where the drum's radius comes from, in metres.
    drum_r: f32,
}

fn def(kind: WeaponKind) -> Def {
    match kind {
        WeaponKind::Rifle => Def {
            hip: vec3(0.150, -0.092, 0.400),
            rotation: vec3(-0.120, 0.045, 0.060),
            aim: vec3(0.0, 0.0, 0.340),
            muzzle: vec3(0.0, -0.074, 0.872),
            grip: vec3(0.0, -0.166, -0.114),
            support: Some(vec3(0.0, -0.150, 0.190)),
            optic: vec3(0.0, -0.010, 0.080),
            // `mag` and `mag_ext` go unread on this weapon: its magazine is
            // *replaced* by a fitting rather than extended by one, so the fitting
            // starts from `well` below instead.
            mag: Vec3::ZERO,
            mag_ext: Vec3::ZERO,
            // The issued magazine and the arc a fitting has to continue.
            well: vec3(0.0, -0.118, 0.006),
            well_half: vec2(0.014, 0.035),
            well_len: 0.112,
            // Radians per metre of magazine: 5.56×45 tapers about a degree, so a
            // thirty-round magazine is gently curved and a sixty is a longer
            // piece of the same arc. See `magazine`.
            well_curve: 2.00,
            drum: Vec3::ZERO,
            drum_r: 0.0,
        },
        WeaponKind::Shotgun => Def {
            hip: vec3(0.165, -0.112, 0.420),
            rotation: vec3(-0.140, 0.060, 0.090),
            aim: vec3(0.0, 0.0, 0.365),
            muzzle: vec3(0.0, -0.094, 0.746),
            grip: vec3(0.0, -0.176, -0.076),
            support: Some(vec3(0.0, -0.164, 0.120)),
            optic: vec3(0.0, -0.018, 0.070),
            // The tube runs the whole barrel; the extension is a collar past
            // its front end.
            mag: vec3(0.0, -0.096, 0.800),
            mag_ext: vec3(0.018, 0.018, 0.055),
            // No box magazine: a tube is not a thing a fitting replaces, so a
            // capacity part continues it and a drum hangs under the receiver.
            well: Vec3::ZERO,
            well_half: Vec2::ZERO,
            well_len: 0.0,
            well_curve: 0.0,
            drum: vec3(0.0, -0.196, 0.055),
            drum_r: 0.056,
        },
        WeaponKind::Smg => Def {
            hip: vec3(0.130, -0.077, 0.360),
            rotation: vec3(-0.100, 0.030, 0.040),
            aim: vec3(0.0, 0.0, 0.305),
            muzzle: vec3(0.0, -0.076, 0.406),
            grip: vec3(0.0, -0.155, -0.096),
            support: Some(vec3(0.0, -0.148, 0.158)),
            optic: vec3(0.0, -0.016, 0.060),
            mag: vec3(0.0, -0.270, -0.004),
            mag_ext: vec3(0.018, 0.042, 0.044),
            well: vec3(0.0, -0.116, -0.008),
            well_half: vec2(0.017, 0.042),
            well_len: 0.172,
            // A 9x19 case tapers less than a rifle's, but a *thirty-round*
            // magazine is long enough for what taper there is to accumulate: the
            // MP5's is the classic curved one, and this is that. What is straight
            // is the fifteen-round MP5 magazine, and the Uzi's and the MP40's —
            // straight by choice of manufacture, not because the round is a
            // cylinder. The first version of this said "very nearly straight",
            // and drew a stick.
            well_curve: 1.70,
            drum: Vec3::ZERO,
            drum_r: 0.0,
        },
        WeaponKind::Sniper => Def {
            hip: vec3(0.170, -0.102, 0.440),
            rotation: vec3(-0.150, 0.050, 0.100),
            aim: vec3(0.0, 0.0, 0.340),
            muzzle: vec3(0.0, -0.085, 1.006),
            grip: vec3(0.0, -0.158, -0.130),
            support: Some(vec3(0.0, -0.146, 0.215)),
            // The sniper carries its own scope; an optic rides above it.
            optic: vec3(0.0, 0.052, 0.080),
            mag: vec3(0.0, -0.190, -0.026),
            mag_ext: vec3(0.021, 0.030, 0.056),
            well: vec3(0.0, -0.122, -0.030),
            well_half: vec2(0.020, 0.055),
            well_len: 0.063,
            // 7.62×51 tapers about twice what 5.56 does, so the marksman's
            // twenty is distinctly curved where the rifle's thirty is gently so.
            well_curve: 1.70,
            drum: Vec3::ZERO,
            drum_r: 0.0,
        },
        WeaponKind::Lmg => Def {
            hip: vec3(0.175, -0.117, 0.430),
            rotation: vec3(-0.130, 0.070, 0.070),
            aim: vec3(0.0, 0.0, 0.360),
            muzzle: vec3(0.0, -0.095, 0.891),
            grip: vec3(0.0, -0.190, -0.138),
            support: Some(vec3(0.0, -0.172, 0.300)),
            optic: vec3(0.0, -0.008, 0.070),
            mag: vec3(0.0, -0.243, 0.021),
            mag_ext: vec3(0.074, 0.050, 0.090),
            well: Vec3::ZERO,
            well_half: Vec2::ZERO,
            well_len: 0.0,
            well_curve: 0.0,
            drum: Vec3::ZERO,
            drum_r: 0.0,
        },
        WeaponKind::Pistol => Def {
            hip: vec3(0.140, -0.047, 0.355),
            rotation: vec3(-0.090, 0.035, 0.055),
            aim: vec3(0.0, 0.0, 0.300),
            muzzle: vec3(0.0, -0.036, 0.132),
            grip: vec3(0.0, -0.118, -0.066),
            support: None,
            optic: vec3(0.0, -0.010, 0.045),
            mag: vec3(0.0, -0.172, -0.090),
            mag_ext: vec3(0.013, 0.032, 0.022),
            well: Vec3::ZERO,
            well_half: Vec2::ZERO,
            well_len: 0.0,
            well_curve: 0.0,
            drum: Vec3::ZERO,
            drum_r: 0.0,
        },
        WeaponKind::M4a1 => Def {
            hip: vec3(0.148, -0.090, 0.396),
            rotation: vec3(-0.118, 0.044, 0.058),
            aim: vec3(0.0, 0.0, 0.336),
            muzzle: vec3(0.0, -0.081, 0.672),
            grip: vec3(0.0, -0.162, -0.082),
            support: Some(vec3(0.0, -0.128, 0.300)),
            optic: vec3(0.0, -0.020, 0.060),
            mag: Vec3::ZERO,
            mag_ext: Vec3::ZERO,
            well: vec3(0.0, -0.120, 0.010),
            well_half: vec2(0.013, 0.032),
            well_len: 0.105,
            well_curve: 2.00,
            drum: Vec3::ZERO,
            drum_r: 0.0,
        },
        WeaponKind::Scar => Def {
            hip: vec3(0.154, -0.096, 0.410),
            rotation: vec3(-0.124, 0.046, 0.062),
            aim: vec3(0.0, 0.0, 0.344),
            muzzle: vec3(0.0, -0.081, 0.762),
            grip: vec3(0.0, -0.168, -0.080),
            support: Some(vec3(0.0, -0.134, 0.350)),
            optic: vec3(0.0, -0.016, 0.070),
            mag: Vec3::ZERO,
            mag_ext: Vec3::ZERO,
            well: vec3(0.0, -0.126, 0.006),
            well_half: vec2(0.014, 0.033),
            well_len: 0.108,
            well_curve: 2.00,
            drum: Vec3::ZERO,
            drum_r: 0.0,
        },
        WeaponKind::Fal => Def {
            hip: vec3(0.158, -0.100, 0.424),
            rotation: vec3(-0.128, 0.048, 0.066),
            aim: vec3(0.0, 0.0, 0.348),
            muzzle: vec3(0.0, -0.084, 0.856),
            grip: vec3(0.0, -0.172, -0.088),
            support: Some(vec3(0.0, -0.140, 0.386)),
            optic: vec3(0.0, -0.014, 0.060),
            mag: Vec3::ZERO,
            mag_ext: Vec3::ZERO,
            well: vec3(0.0, -0.130, 0.012),
            well_half: vec2(0.015, 0.036),
            well_len: 0.118,
            // 7.62x51 tapers about twice what 5.56 does.
            well_curve: 1.70,
            drum: Vec3::ZERO,
            drum_r: 0.0,
        },
        WeaponKind::Mp5 => Def {
            hip: vec3(0.130, -0.079, 0.364),
            rotation: vec3(-0.102, 0.030, 0.042),
            aim: vec3(0.0, 0.0, 0.308),
            muzzle: vec3(0.0, -0.081, 0.446),
            grip: vec3(0.0, -0.158, -0.064),
            support: Some(vec3(0.0, -0.128, 0.264)),
            optic: vec3(0.0, -0.020, 0.058),
            mag: Vec3::ZERO,
            mag_ext: Vec3::ZERO,
            well: vec3(0.0, -0.126, -0.006),
            well_half: vec2(0.015, 0.040),
            well_len: 0.168,
            well_curve: 1.70,
            drum: Vec3::ZERO,
            drum_r: 0.0,
        },
        WeaponKind::Mpx => Def {
            hip: vec3(0.134, -0.081, 0.372),
            rotation: vec3(-0.104, 0.032, 0.044),
            aim: vec3(0.0, 0.0, 0.312),
            muzzle: vec3(0.0, -0.081, 0.470),
            grip: vec3(0.0, -0.160, -0.068),
            support: Some(vec3(0.0, -0.132, 0.276)),
            optic: vec3(0.0, -0.018, 0.062),
            mag: Vec3::ZERO,
            mag_ext: Vec3::ZERO,
            well: vec3(0.0, -0.124, -0.004),
            well_half: vec2(0.016, 0.041),
            well_len: 0.170,
            well_curve: 1.70,
            drum: Vec3::ZERO,
            drum_r: 0.0,
        },
        WeaponKind::M110 => Def {
            hip: vec3(0.166, -0.100, 0.436),
            rotation: vec3(-0.146, 0.050, 0.096),
            aim: vec3(0.0, 0.0, 0.342),
            muzzle: vec3(0.0, -0.085, 0.980),
            grip: vec3(0.0, -0.166, -0.104),
            support: Some(vec3(0.0, -0.150, 0.420)),
            optic: vec3(0.0, -0.014, 0.070),
            mag: Vec3::ZERO,
            mag_ext: Vec3::ZERO,
            well: vec3(0.0, -0.132, -0.020),
            well_half: vec2(0.016, 0.044),
            well_len: 0.116,
            well_curve: 1.70,
            drum: Vec3::ZERO,
            drum_r: 0.0,
        },
        WeaponKind::Mosin => Def {
            hip: vec3(0.172, -0.104, 0.448),
            rotation: vec3(-0.152, 0.052, 0.102),
            aim: vec3(0.0, 0.0, 0.344),
            muzzle: vec3(0.0, -0.081, 1.092),
            grip: vec3(0.0, -0.170, -0.096),
            support: Some(vec3(0.0, -0.156, 0.360)),
            optic: vec3(0.0, -0.012, 0.070),
            mag: Vec3::ZERO,
            mag_ext: Vec3::ZERO,
            // The internal five-round box, under the receiver.
            well: vec3(0.0, -0.126, -0.024),
            well_half: vec2(0.014, 0.034),
            well_len: 0.052,
            well_curve: 1.10,
            drum: Vec3::ZERO,
            drum_r: 0.0,
        },
        WeaponKind::Knife => Def {
            hip: vec3(0.150, -0.057, 0.350),
            rotation: vec3(-0.105, 0.045, 0.085),
            aim: vec3(0.0, 0.0, 0.330),
            muzzle: vec3(0.0, 0.0, 0.212),
            grip: vec3(0.0, -0.072, -0.074),
            support: None,
            // The knife takes no parts; the anchors are never read.
            optic: Vec3::ZERO,
            mag: Vec3::ZERO,
            mag_ext: Vec3::ZERO,
            well: Vec3::ZERO,
            well_half: Vec2::ZERO,
            well_len: 0.0,
            well_curve: 0.0,
            drum: Vec3::ZERO,
            drum_r: 0.0,
        },
    }
}

/// One box of kit geometry: centred on `at`, half-extents `half`.
fn kit_box(out: &mut Vec<Tri>, at: Vec3, half: Vec3, color: Color) {
    box3(out, at - half, at + half, color);
}

/// A quad-stack magazine: two box magazines sharing one floorplate, which is
/// what a quad-stack physically is. Built as two rather than as one wider box,
/// because the seam between the columns is the whole tell — a single wider box
/// reads as a longer magazine that lost its rake, not as a four-column one.
///
/// `long` and `spread` are the two knobs `magazine_kit` takes: how far the
/// magazine runs below the well, and how much wider than the standard one it is
/// across the gun. The columns are sized so that together they are `spread`
/// times the standard width, and they meet on the weapon's centre line.
fn quad_stack(out: &mut Vec<Tri>, d: &Def, long: f32, spread: f32, color: Color) {
    let column = d.well_half.x * spread * 0.5;
    for side in [-1.0f32, 1.0] {
        magazine(
            out,
            d.well + vec3(side * column, 0.0, 0.0),
            d.well_len * long,
            vec2(column, d.well_half.y),
            d.well_curve,
            color,
        );
    }
}

/// The hundred-round drum: one wide drum with a feed tower.
///
/// The real one is two fifty-round drums side by side *across* the weapon, which
/// from the side is a single drum face — so a single drum face is what it is
/// drawn as, just a bigger one than the fifty. Two drums *in line* along the
/// bore was the first attempt, and two overlapping circles merge into one lump:
/// what that reads as is a potato. The concentric hub the same version drew is
/// what made the pair read as wheels.
fn wide_drum(out: &mut Vec<Tri>, d: &Def, r_frac: f32, thick: f32, color: Color) {
    // Hangs from the well: the drum's top is the mouth of the well, so nothing
    // of the issued magazine is left sticking out of it.
    let r = d.well_len * r_frac;
    let at = vec3(d.well.x, d.well.y - r, d.well.z);
    drum(out, at, r, d.well_half.x * thick, color);
}

/// A pistol's magazine, which is the grip continued: a column of rounds showing
/// below the frame, closed by a floorplate. The three lengths on offer differ in
/// nothing else, so `long` is the whole of it.
fn pistol_mag(out: &mut Vec<Tri>, d: &Def, long: f32) {
    let at = d.mag;
    kit_box(out, at, d.mag_ext * vec3(1.02, 1.30 * long, 1.05), POLY);
    kit_box(
        out,
        at + vec3(0.0, -d.mag_ext.y * 1.36 * long, 0.0),
        d.mag_ext * vec3(1.10, 0.10, 1.12),
        BLACK,
    );
}

/// The support weapon's belt box: a tin of linked rounds. A plain box read as a/// lump hanging under the gun, so it gets a lid a shade proud of the body, a
/// pair of ribs down its flanks, and the top course of the belt showing between
/// them — the belt is wider than the receiver it feeds, so the part that is not
/// buried inside the gun is the part you can see.
fn belt_box(out: &mut Vec<Tri>, at: Vec3, half: Vec3, color: Color) {
    kit_box(out, at, half, color);
    kit_box(
        out,
        at + vec3(0.0, half.y * 0.94, 0.0),
        half * vec3(1.06, 0.10, 1.06),
        STEEL_DARK,
    );
    kit_box(
        out,
        at + vec3(0.0, half.y * 1.02, 0.0),
        vec3(half.x * 1.16, 0.005, half.z * 0.88),
        BRASS,
    );
    for side in [-1.0f32, 1.0] {
        kit_box(
            out,
            at + vec3(side * half.x * 0.90, 0.0, 0.0),
            vec3(half.x * 0.12, half.y * 0.76, half.z * 0.84),
            STEEL_DARK,
        );
    }
}

/// A muzzle device: a sleeve growing forward from the barrel tip.
fn muzzle_kit(out: &mut Vec<Tri>, at: Vec3, len: f32, half: f32) {
    kit_box(
        out,
        at + vec3(0.0, 0.0, len * 0.5 - 0.010),
        vec3(half, half, len * 0.5),
        STEEL_DARK,
    );
}

/// A capacity magazine: the issued one again, longer and no narrower.
///
/// `long` is how many times the issued length it runs and `wide` how much wider
/// across the gun it is. Nothing is *continued*, because the issued magazine is
/// not drawn when this is: the two are the same object at two sizes, and building
/// it that way is the only way the curve, the taper and the floorplate can all
/// come out matching at the join. The version before this extended the issued
/// magazine from a separate anchor, which is why the fittings never quite met it.
fn magazine_kit(out: &mut Vec<Tri>, d: &Def, long: f32, wide: f32, color: Color) {
    magazine(
        out,
        d.well,
        d.well_len * long,
        vec2(d.well_half.x * wide, d.well_half.y),
        d.well_curve,
        color,
    );
}

/// How many times the issued magazine's length a fitting runs, read off the
/// rounds it holds rather than off its name.
///
/// Rounds are the one thing a magazine part is *asked* for: a fifty-round drum
/// is fifty rounds in whatever well it fits, and how long that makes it is the
/// well's business, not the catalogue's. On a well that is a box, length and
/// capacity are the same line — which is why the lengths `kit_into` spells out
/// are the sizes they are, and why a magazine the catalogue gains later can be
/// drawn without any of them being written again.
fn magazine_fit_length(kind: WeaponKind, build: &WeaponBuild) -> f32 {
    let rounds = |build: &WeaponBuild| {
        crate::weapon::resolve(crate::weapon::def(kind), build)
            .as_stats()
            .mag_size
            .max(1) as f32
    };
    rounds(build) / rounds(&WeaponBuild::empty())
}

/// The attachment geometry a build adds to a weapon's own, in local space and
/// factory colours — the paint pass recolours it with everything else.
///
/// The shape is keyed on the part id, the one name both the bench and the
/// in-hand draw read. A part the match has never heard of still draws — see its
/// last arm, which is there because six magazines once reached the catalogue
/// without geometry and nothing in the game could tell.
fn kit_into(out: &mut Vec<Tri>, kind: WeaponKind, build: &WeaponBuild) {
    let d = def(kind);
    for entry in &build.entries {
        match entry.part.0 {
            "micro_dot" => {
                kit_box(out, d.optic, vec3(0.015, 0.016, 0.028), RAIL);
                kit_box(
                    out,
                    d.optic + vec3(0.0, 0.0, 0.028),
                    vec3(0.011, 0.011, 0.002),
                    BLACK,
                );
            }
            "marksman_scope" => {
                kit_box(out, d.optic, vec3(0.017, 0.018, 0.080), RAIL);
                kit_box(
                    out,
                    d.optic + vec3(0.0, 0.0, 0.082),
                    vec3(0.019, 0.019, 0.006),
                    BLACK,
                );
            }
            "compensator" => muzzle_kit(out, d.muzzle, 0.065, 0.020),
            "flash_hider" => muzzle_kit(out, d.muzzle, 0.048, 0.017),
            "heavy_barrel" => muzzle_kit(out, d.muzzle, 0.130, 0.012),
            // Magazines. The issued one is part of the weapon; a capacity part
            // is that same magazine continued past the well, drawn from the
            // def's own anchor so each gun keeps its proportions. Four of them
            // are not a longer box: the quad-stack, which is two boxes, the two
            // 9 mm drums, which are drums, the hundred-round twin drum, which is
            // two of those, and the belt box, which is a tin.
            "mag_10_762" => magazine_kit(out, &d, 1.60, 1.00, POLY),
            // The full-power wells. Ten rounds is a *shorter* magazine than the
            // issued twenty, which is the one thing the replace model can draw
            // and the extend model could not.
            "mag_10_fal" | "mag_10_sr25" => magazine_kit(out, &d, 0.55, 1.00, POLY),
            "mag_25_sr25" => magazine_kit(out, &d, 1.30, 1.00, POLY),
            "mag_30_fal" => magazine_kit(out, &d, 1.55, 1.00, POLY),
            "mag_20_762" => magazine_kit(out, &d, 2.60, 1.00, POLY),
            "mag_40_556" | "mag_40_9mm" => magazine_kit(out, &d, 1.33, 1.00, POLY),
            // The 9 mm submachine wells. Each keeps its own magazine lengths, as
            // the real ones do: the MP5's fifteen is half its thirty and its
            // forty is a third longer, the MPX's ten is a third of its own
            // thirty. The curve is not scaled with them — it belongs to the
            // cartridge — so the short ones come out nearly straight and the long
            // ones visibly bent, which is what the photographs of them show.
            "mag_15_mp5" => magazine_kit(out, &d, 0.50, 1.00, POLY),
            "mag_40_mp5" => magazine_kit(out, &d, 1.33, 1.00, POLY),
            "mag_10_mpx" => magazine_kit(out, &d, 0.33, 1.00, POLY),
            "mag_20_mpx" => magazine_kit(out, &d, 0.67, 1.00, POLY),
            "mag_35_mpx" => magazine_kit(out, &d, 1.17, 1.00, POLY),
            // The quad-stack is not a longer box: it is two box magazines under a
            // shared floorplate, so it is built as two. A single wider box read as
            // a bigger magazine, not as a four-column one.
            "mag_60_556" => quad_stack(out, &d, 1.70, 1.55, POLY),
            "drum_50_9mm" => wide_drum(out, &d, 0.36, 2.4, POLY),
            "drum_100_556" => wide_drum(out, &d, 0.65, 3.0, POLY),
            "drum_100_9mm" => wide_drum(out, &d, 0.56, 3.0, POLY),
            "belt_200" => belt_box(
                out,
                d.mag + vec3(0.0, -d.mag_ext.y * 0.35, 0.0),
                d.mag_ext * vec3(1.45, 1.35, 1.20),
                OLIVE,
            ),
            // A pistol's magazine lives in the grip: more rounds is more of the
            // same column showing below the frame, closed by its own floorplate
            // the way the standard one's is.
            "mag_17_pistol" => pistol_mag(out, &d, 1.10),
            "mag_20_pistol" => pistol_mag(out, &d, 1.30),
            "mag_30_pistol" => pistol_mag(out, &d, 1.78),
            // A stripper clip: five rounds in a strip of spring steel, laid on
            // the action. The box holds five with or without it.
            "clip_54r" => {
                // Lying in the open action with its rounds pointing down, shoved
                // in from the right — which is what loading one looks like, and
                // the only place a fitting this small can show at all: the iron
                // sights already reach the sight line, so nothing laid flat on
                // the receiver changes the weapon's silhouette.
                let at = vec3(d.optic.x + 0.062, d.optic.y - 0.020, d.optic.z - 0.010);
                kit_box(out, at, vec3(0.028, 0.004, 0.036), STEEL_DARK);
                for i in 0..5 {
                    let z = at.z - 0.028 + i as f32 * 0.014;
                    kit_box(
                        out,
                        vec3(at.x, at.y + 0.008, z),
                        vec3(0.020, 0.007, 0.0055),
                        BRASS,
                    );
                }
            }
            // The tube is a tube: the extension is a collar past its end.
            "tube_8" => kit_box(out, d.mag, d.mag_ext, POLY),
            // And the drum: it hangs under the receiver, which is a different
            // place from the end of the tube the well anchor describes. A drum
            // out at the muzzle is a ball on the end of a stick.
            "drum_20_shell" => {
                drum(out, d.drum, d.drum_r, d.drum_r * 0.95, POLY);
                kit_box(
                    out,
                    vec3(d.drum.x, d.drum.y + d.drum_r * 0.95, d.drum.z),
                    vec3(d.drum_r * 0.55, d.drum_r * 0.45, d.drum_r * 0.80),
                    POLY,
                );
            }
            _ if entry.slot == crate::weapon::SlotId("magazine") => {
                // A magazine draws. The well says where it hangs and how it
                // curves, the rounds it holds say how long it is, and a name
                // beginning `drum_` says it is round. This arm is here because
                // six magazines once entered the catalogue — the day three 9 mm
                // wells were split one from another — whose ids were in the
                // catalogue and not in this match, and every one of them drew
                // nothing at all.
                if entry.part.0.starts_with("drum_") {
                    wide_drum(out, &d, 0.36, 2.4, POLY);
                } else if d.well_len > 0.0 {
                    let long = magazine_fit_length(kind, build);
                    magazine_kit(out, &d, long, 1.00, POLY);
                } else {
                    // No well to hang a box from: the fittings these weapons
                    // take continue something else — a tube, a belt box.
                    kit_box(out, d.mag, d.mag_ext, POLY);
                }
            }
            _ => {}
        }
    }
}

/// The weapon as the armory shows it: the model with its attachments, painted
/// and worn, turned to a left profile with a touch of top and front so the
/// optic and the muzzle read — centred on the origin, ready for an
/// orthographic, portrait-style draw.
/// How much room a preview leaves around the weapon: under it for a magazine,
/// over it for a sight, and beside it for a muzzle device.
///
/// The frame is measured on the weapon *as issued* plus this room, so every build
/// of one weapon is drawn at the same size in the same place. The version before
/// this fitted the frame to the model in hand, which meant a longer magazine moved
/// the whole weapon up and shrank it — and two magazines that move the gun between
/// them cannot be compared, which is the one thing the preview is for.
const PREVIEW_MAG_ROOM: f32 = 0.15;
const PREVIEW_SIGHT_ROOM: f32 = 0.10;
const PREVIEW_SIDE_ROOM: f32 = 0.06;

pub fn preview_mesh(kind: WeaponKind, fitted: &WeaponBuild, paint: Paint) -> Mesh {
    // Assembled as the build has it — a magazine fitting replaces the issued one
    // — but framed as the weapon is issued.
    let reference = build(kind);
    let mut tris = build_assembled(kind, !fits_a_magazine(fitted));
    kit_into(&mut tris, kind, fitted);
    paint_tris(&mut tris, paint);
    let pose = Mat4::from_rotation_y(-1.32) * Mat4::from_rotation_x(0.10);
    let mut lo = Vec3::splat(f32::INFINITY);
    let mut hi = Vec3::splat(f32::NEG_INFINITY);
    for t in &reference {
        let t = t.map(pose);
        for p in [t.a, t.b, t.c] {
            lo = lo.min(p);
            hi = hi.max(p);
        }
    }
    lo.x -= PREVIEW_SIDE_ROOM;
    hi.x += PREVIEW_SIDE_ROOM;
    lo.y -= PREVIEW_MAG_ROOM;
    hi.y += PREVIEW_SIGHT_ROOM;
    let mid = (lo + hi) * 0.5;
    let tris: Vec<Tri> = tris
        .into_iter()
        .map(|t| t.map(pose))
        .map(|t| Tri::new(t.a - mid, t.b - mid, t.c - mid, t.color))
        .collect();
    tris_to_mesh(&tris)
}

/// Flat triangles as a `Mesh`, for the portrait draw: the colours ride on the
/// vertices, and the uvs and normals are never read by it.
fn tris_to_mesh(tris: &[Tri]) -> Mesh {
    let mut vertices = Vec::with_capacity(tris.len() * 3);
    let mut indices = Vec::with_capacity(tris.len() * 3);
    for (i, t) in tris.iter().enumerate() {
        for p in [t.a, t.b, t.c] {
            vertices.push(Vertex {
                position: p,
                uv: Vec2::ZERO,
                color: crate::mesh::rgba(t.color),
                normal: Vec4::new(0.0, 0.0, 1.0, 0.0),
            });
        }
        let base = (i * 3) as u16;
        indices.extend_from_slice(&[base, base + 1, base + 2]);
    }
    Mesh {
        vertices,
        indices,
        texture: None,
    }
}

/// Where the weapon sits and how it is turned, in view space.
struct Pose {
    offset: Vec3,
    /// Yaw, pitch and roll about the weapon's own origin. Pitch is positive
    /// muzzle-down, yaw positive muzzle-right.
    rotation: Vec3,
    /// The arm's own contribution: a rotation about the *grip* rather than about
    /// the weapon. Kept apart from `rotation` because what a rotation turns
    /// about is the whole difference between a person turning a weapon over in
    /// their hands and a weapon spinning on the spot — the grip is where a hand
    /// is, so it is the one point that has to stay where the hand leaves it
    /// while everything else swings.
    swing: Vec3,
    /// The point `swing` turns about, in the weapon's own space: the grip.
    pivot: Vec3,
}

impl Pose {
    fn matrix(&self) -> Mat4 {
        let arm = Mat4::from_translation(self.pivot)
            * Mat4::from_rotation_y(self.swing.x)
            * Mat4::from_rotation_x(self.swing.y)
            * Mat4::from_rotation_z(self.swing.z)
            * Mat4::from_translation(-self.pivot);
        Mat4::from_translation(self.offset)
            * arm
            * Mat4::from_rotation_y(self.rotation.x)
            * Mat4::from_rotation_x(self.rotation.y)
            * Mat4::from_rotation_z(self.rotation.z)
    }
}

pub struct ViewModel {
    /// Weapon lag behind the crosshair, in view units.
    sway: Vec2,
    /// How far a weapon switch has the weapon lowered, 1 at the start.
    switch_drop: f32,
    /// How much of the crouch and prone poses applies, each eased towards 0 or
    /// 1 so a change of stance carries the weapon down rather than cutting it.
    crouch: f32,
    prone: f32,
    /// Countdown for the grenade throw motion; zero when idle.
    grenade_t: f32,
    /// How much of the "something is in my other hand" pose is on, eased so the
    /// weapon comes back up rather than being cut back.
    hold_t: f32,
    /// The throwable in the other hand, if any. Its *kind* is here because the
    /// thing drawn in the hand is that kind's, and the pose is the same for all
    /// of them.
    grabbed: Option<GrenadeKind>,
    parts: [Vec<Tri>; crate::weapon::KIND_COUNT],
    /// The same, with the issued magazine left off. A magazine fitting replaces
    /// the magazine rather than extending it, so the weapon it is fitted to is
    /// the one without: keeping both means the swap costs a lookup instead of a
    /// rebuild.
    bare: [Vec<Tri>; crate::weapon::KIND_COUNT],
    /// The hands that hold each weapon, kept apart from `parts` because they do
    /// not move with it — see `HAND_LAG`. Split into shooting hand and support
    /// hand because a throwable in the off hand takes the support hand's place:
    /// the fist that would be on the foregrip is the fist that is holding the
    /// grenade.
    grip: [Vec<Tri>; crate::weapon::KIND_COUNT],
    support: [Vec<Tri>; crate::weapon::KIND_COUNT],
    /// The sniper's scope, which `parts` deliberately leaves out. It is the one
    /// piece of a weapon the player looks through rather than at, so it is
    /// drawn only while the sights are down; see `ViewModel::draw`.
    scope: Vec<Tri>,
    /// Scratch for `draw`. The body and the flash are rebuilt from `parts` every
    /// frame anyway; collecting them into a fresh vector each time was a handful
    /// of allocations a frame for geometry that never changes size.
    body: Vec<Tri>,
    flash: Vec<Tri>,
    /// Scratch for the attachments of whatever is in hand, rebuilt per frame
    /// like the body and for the same reason.
    kit: Vec<Tri>,
    /// Scratch for the throwable in the off hand, which changes kind and so
    /// cannot be built once per weapon kind like the rest.
    held: Vec<Tri>,
}

impl ViewModel {
    pub fn new() -> Self {
        Self {
            sway: Vec2::ZERO,
            switch_drop: 0.0,
            crouch: 0.0,
            prone: 0.0,
            grenade_t: 0.0,
            hold_t: 0.0,
            grabbed: None,
            // One entry per kind, in `WeaponKind::index` order.
            parts: ALL_KINDS.map(|k| build_held(k, true)),
            bare: ALL_KINDS.map(|k| build_held(k, false)),
            grip: ALL_KINDS.map(grip_hand),
            support: ALL_KINDS.map(support_hand),
            scope: {
                let mut out = Vec::new();
                sniper_scope(&mut out);
                out
            },
            body: Vec::new(),
            flash: Vec::new(),
            kit: Vec::new(),
            held: Vec::new(),
        }
    }

    /// Call once per frame while playing.
    pub fn update(&mut self, dt: f32, mouse_delta: Vec2, player: &Player, arsenal: &Arsenal) {
        let dt = dt.clamp(0.0, 0.1);

        // `mouse_delta` may be pixels or a screen fraction: the saturating
        // response copes with either, and aiming damps it right down.
        let damp = 1.0 - player.ads.clamp(0.0, 1.0) * 0.92;
        let impulse = SWAY_STEP * (mouse_delta.length() * 8.0).min(1.0) * damp;
        let decay = SWAY_DECAY.powf(dt * 60.0);
        self.sway = (self.sway * decay - mouse_delta.normalize_or_zero() * impulse)
            .clamp_length_max(SWAY_MAX);

        // Eased so the next weapon rises into place rather than snapping up.
        self.switch_drop = smooth(arsenal.switch_t / SWITCH_TIME);

        // The eye is already eased by `Player` when the stance changes, and the
        // weapon has to follow it rather than jump to the new posture.
        let (crouch, prone) = match player.stance() {
            Stance::Stand => (0.0, 0.0),
            Stance::Crouch => (1.0, 0.0),
            Stance::Prone => (0.0, 1.0),
        };
        self.crouch = approach(self.crouch, crouch, dt * STANCE_RATE);
        self.prone = approach(self.prone, prone, dt * STANCE_RATE);

        self.grenade_t = (self.grenade_t - dt).max(0.0);
        // The weapon is away while the throwing hand is out, and the hand is out
        // until the throw has carried it off the bottom of the frame — see
        // `arm_falling`. The weapon waits for the arm rather than rising through
        // it, because both hands are the same hand and two of them on screen at
        // once is worse than a rifle that comes back a moment late.
        let want = if self.holding() || self.arm_falling() {
            1.0
        } else {
            0.0
        };
        self.hold_t = approach(self.hold_t, want, dt * HOLD_RATE);
    }

    /// Called once a frame with what is in the throwing hand, if anything.
    pub fn set_grabbed(&mut self, grabbed: Option<GrenadeKind>) {
        self.grabbed = grabbed;
    }

    /// Whether a throwable is in the hand — which is what the weapon's own pose
    /// is waiting on.
    fn holding(&self) -> bool {
        self.grabbed.is_some()
    }

    /// Whether the arm that threw is still on its way out of the frame: the
    /// front of the throw motion, and the whole of the time the weapon waits for
    /// it. Past this the hand is below the frame, and the weapon's own hand can
    /// take over from it without there being two of them.
    fn arm_falling(&self) -> bool {
        self.grenade_t > GRENADE_TIME * (1.0 - THROW_CLEAR)
    }

    /// Where the throwing hand is drawn, in the view's own right/up/forward
    /// metres: `HAND_OFFSET`, raised into the frame as the thing comes to hand
    /// and carried off the bottom of it by the throw.
    ///
    /// Read by the drawing *and* by `main.rs`, which throws from here and draws
    /// the arc from here — so the line leaves the thing in the hand on every
    /// frame and not only on the finished pose.
    ///
    /// The way out is a one-way ramp rather than the window the rest of the
    /// poses use: a window has to come back to where it started, and a hand that
    /// came back up to the middle of the screen after a throw — and then went out
    /// in one frame when the motion ended — would be a worse cut than the one
    /// this exists to hide.
    pub fn throw_hand(&self) -> Vec3 {
        let rise = if self.holding() { self.hold_t } else { 1.0 };
        let fall = if self.grenade_t > 0.0 {
            let t = 1.0 - (self.grenade_t / GRENADE_TIME).clamp(0.0, 1.0);
            smooth(t / THROW_CLEAR)
        } else {
            0.0
        };
        HAND_OFFSET + vec3(0.0, -((1.0 - rise) * HAND_RISE + fall * HAND_FALL), 0.0)
    }

    /// Plays a short throw motion. Called when the player throws a grenade.
    pub fn trigger_grenade_throw(&mut self) {
        self.grenade_t = GRENADE_TIME;
    }

    /// Call in the 2D pass, after `set_default_camera()`. `paints` is the
    /// armory's finish per weapon kind.
    pub fn draw(
        &mut self,
        player: &Player,
        arsenal: &Arsenal,
        paints: &[Paint; crate::weapon::KIND_COUNT],
    ) {
        let viewport = vec2(screen_width(), screen_height());
        let fovy = player.fovy();
        if viewport.x < 1.0 || viewport.y < 1.0 || fovy <= 0.0 {
            return;
        }
        // Matches the 3D camera, so the weapon keeps the right scale.
        let f = (viewport.y * 0.5) / (fovy * 0.5).tan();

        let weapon = arsenal.active_weapon();
        let paint = paints[weapon.kind.index()];
        let mat = self.assemble(player, weapon, paint);
        // Painter's algorithm: farthest faces first, nearer ones cover them.
        for t in self.body.iter() {
            if let Some(p) = project(t, viewport, f) {
                draw_triangle(p[0], p[1], p[2], t.shaded());
            }
        }

        // The flash is light rather than geometry, so it goes over the top —
        // except for blades, which flash nothing at all.
        if weapon.flash > 0.0 && !weapon.stats().melee {
            let flash = &mut self.flash;
            flash.clear();
            flash_shape(flash, def(weapon.kind).muzzle, weapon.flash, weapon.shots);
            for t in flash.iter().map(|t| t.map(mat)).filter(Tri::finite) {
                if let Some(p) = project(&t, viewport, f) {
                    draw_triangle(p[0], p[1], p[2], t.color);
                }
            }
        }
    }

    /// Everything the viewmodel draws for one frame of holding `weapon`, posed
    /// and finished, into `self.body`: the weapon, its fittings, and whichever
    /// hands are on it — sorted back to front, ready to be projected.
    ///
    /// Apart from `draw` because the tests have to measure *this* list and not a
    /// second one assembled beside it: "the weapon is put away while a throwable
    /// is in hand" is a claim about what is on the screen, and a test that built
    /// its own triangles would be measuring a pose that is nearly the drawn one.
    ///
    /// Returns the weapon's own matrix, which the flash needs and which casting
    /// from the body would be guesswork about.
    fn assemble(&mut self, player: &Player, weapon: &Weapon, paint: Paint) -> Mat4 {
        let mat = self.weapon_into(player, weapon, paint);
        let kind = weapon.kind;
        // The hands, which are the weapon's only while they are on it.
        //
        // A throwable in the throwing hand takes the *right* hand off the
        // weapon: it is the hand that throws, and the weapon is put away
        // (`HOLD_DROP`), so the fist that was on the grip is the fist holding
        // the grenade. The support hand goes with it — a hand on a foregrip
        // that is down by the shooter's knee is a hand on nothing — and both
        // come back when the throwing arm is out of the frame and the weapon
        // has risen again.
        //
        // Until then the throwing arm is drawn at its own place rather than
        // through the weapon's pose: it is not holding the gun and does not
        // follow it down. It keeps the hands' same lag, so the two stay a pair
        // when the view swings.
        let lag = mat * Mat4::from_translation(vec3(-self.sway.x, -self.sway.y, 0.0) * HAND_LAG);
        let hand_mat = Mat4::from_translation(vec3(-self.sway.x, -self.sway.y, 0.0) * HAND_LAG);
        if self.holding() || self.arm_falling() {
            let at = self.throw_hand();
            let grabbed = self.grabbed;
            let held = &mut self.held;
            held.clear();
            match grabbed {
                Some(grenade) => held_throwable(held, grenade, at),
                None => throwing_hand(held, at),
            }
            let body = &mut self.body;
            body.extend(held.iter().map(|t| t.map(hand_mat)).filter(Tri::finite));
        } else {
            let grip = &self.grip[kind.index()];
            let support = &self.support[kind.index()];
            let body = &mut self.body;
            body.extend(
                grip.iter()
                    .chain(support.iter())
                    .map(|t| t.map(lag))
                    .filter(Tri::finite),
            );
        }
        // Painter's algorithm: farthest faces first, nearer ones cover them.
        self.body.sort_by(|x, y| y.depth().total_cmp(&x.depth()));
        mat
    }

    /// The gun half of [`ViewModel::assemble`], into `self.body`: the weapon, its
    /// fittings and the scope, posed and finished, and no hands.
    ///
    /// Split out so a test can measure the weapon alone. "The weapon is put
    /// away" is a claim about the gun, and the hand holding the throwable is
    /// supposed to be on the screen at the same moment — so a test that measured
    /// the whole list would be measuring the wrong thing.
    fn weapon_into(&mut self, player: &Player, weapon: &Weapon, paint: Paint) -> Mat4 {
        let kind = weapon.kind;
        let mat = self.pose(player, weapon).matrix();
        let kit = &mut self.kit;
        kit.clear();
        kit_into(kit, kind, weapon.build());
        let body = &mut self.body;
        body.clear();
        // The finish is applied in local space, before the pose: the wear
        // pattern hashes to the weapon, not to the world it is moving through.
        let parts = if fits_a_magazine(weapon.build()) {
            &self.bare[kind.index()]
        } else {
            &self.parts[kind.index()]
        };
        body.extend(
            parts
                .iter()
                .chain(kit.iter())
                .map(|t| painted_tri(*t, paint))
                .map(|t| t.map(mat))
                .filter(Tri::finite),
        );
        // The sniper's scope belongs to the rifle everywhere except at the
        // eyepiece. Once the eye is inside the tube the only thing the tube can
        // do is block the view the scope exists to provide, so it is left out
        // and `hud::draw_scope_overlay` draws a round aperture in its place —
        // the two swap on the same frame, off the same constant.
        if kind == WeaponKind::Sniper && player.ads < SCOPE_HIDE_ADS {
            let scope = &self.scope;
            body.extend(
                scope
                    .iter()
                    .map(|t| painted_tri(*t, paint))
                    .map(|t| t.map(mat))
                    .filter(Tri::finite),
            );
        }
        mat
    }

    fn pose(&self, player: &Player, weapon: &Weapon) -> Pose {
        let d = def(weapon.kind);
        let ads = player.ads.clamp(0.0, 1.0);
        // Weight of everything that would move the sight line: full at the hip,
        // gone with the sights up, so aiming always wins.
        let hip = 1.0 - ads;

        // The aim offset is the only thing that moves the sight line, and it
        // sits on the middle of the screen at full ADS by construction.
        let mut offset = d.hip.lerp(d.aim, ads);
        let mut rotation = d.rotation * hip;
        // The arm's own motion, turned about the grip. A rotation by nothing is
        // the identity whatever it turns about, so the pivot is always the grip
        // and nothing else has to know whether the arm is moving.
        let mut arm = Vec3::ZERO;

        // Stance: crouching carries the weapon down and pulls it in, and going
        // prone drops it away and cants it over as though braced on the ground.
        // The pull in is depth, so it stays while aiming.
        let carry = CROUCH_CARRY * self.crouch + PRONE_CARRY * self.prone;
        let cant = CROUCH_CANT * self.crouch + PRONE_CANT * self.prone;
        offset += vec3(carry.x * hip, carry.y * hip, carry.z);
        rotation += vec3(cant.x * hip, cant.y * hip, cant.z);

        // Sway and bob shift the weapon across the screen; the bob follows the
        // same phase as the head so the two read as one movement. Walking is
        // meant to be quiet, so it damps the bob hard.
        offset.x += self.sway.x;
        offset.y += self.sway.y;
        rotation.z += self.sway.x * 0.35;

        let bob = player.bob_amount() * if player.walk { WALK_BOB } else { 1.0 };
        let phase = player.bob_phase();
        offset.x += phase.sin() * 0.012 * bob;
        offset.y += (phase * 2.0).sin() * 0.009 * bob;

        // Recoil shoves the weapon straight back and tips the muzzle up.
        offset.z -= weapon.kick * 0.030;
        rotation.y -= weapon.kick * 0.070;

        // Leaning takes the eye off the body's centre line, so the weapon —
        // still held by the body — slides the other way, and the camera's roll
        // is mirrored onto the weapon so the two stay glued together.
        let lean = player.lean.clamp(-1.0, 1.0);
        offset.x -= lean * 0.055 * hip;
        offset.y -= lean.abs() * 0.012 * hip;
        rotation.z += player.lean_roll();

        // Reloading dips the weapon out of the sight line and back again.
        let reload = if weapon.reload_t > 0.0 {
            (weapon.reload_progress() * std::f32::consts::PI).sin()
        } else {
            0.0
        };
        if reload > 0.0 {
            offset.y -= reload * 0.150;
            offset.z -= reload * 0.050;
            rotation.x -= reload * 0.100;
            rotation.y += reload * 0.450;
            rotation.z += reload * 0.300;
        }

        // A switch drops the incoming weapon and brings it back up.
        let drop = self.switch_drop;
        offset.y -= drop * 0.300;
        offset.z -= drop * 0.060;
        rotation.y += drop * 0.300;
        rotation.z += drop * 0.450;

        // A throwable in hand puts the weapon away — not lowered to the hip, but
        // *away*, out of the frame, because the hand that was holding it is the
        // hand that is now holding the grenade and a rifle left in the other one
        // is a rifle between the eye and the throw. What is holding it now is a
        // sling: the hand has let go of it, which is why its own pose has no
        // throwing motion any more. The throw is the arm's.
        //
        // The drop is not weighted by `hip`. A weapon being aimed at the moment
        // the throwable comes to hand has to leave by the same road as one at
        // the hip; `main.rs` drops the aim at the same time, and a pose that
        // only moved off a sight line that was already gone would leave the
        // sights up over a stowed rifle.
        let hold_w = self.hold_t;
        if hold_w > 0.0 {
            offset += vec3(HOLD_OUT * hip, -HOLD_DROP, -HOLD_BACK) * hold_w;
            rotation += HOLD_CANT * hold_w;
        }

        // Inspecting and the magazine check are two different motions, because
        // they are two different things: `U` turns the weapon over to be read,
        // and `Alt`+`T` is a look at the well. Both are paths of keyframes
        // rather than summed windows — a path can hold still at a feature, which
        // is where the reading happens, and both turn about the grip rather than
        // about the weapon, which is what makes them the arm's doing.
        //
        // Both are the same hand's work as the throwable would be, so a hand
        // that is holding one is not free for either: an inspect that was
        // already running when the thing came to hand fades out with the weapon
        // rather than turning a rifle that is down by the shooter's knee.
        let damp = (1.0 - reload) * (1.0 - self.hold_t) * hip;
        let mut arms = None;
        if weapon.inspect_t > 0.0 {
            let t = 1.0 - weapon.inspect_t / crate::weapon::INSPECT_TIME;
            arms = Some((t, inspect_flavour(weapon.kind)));
        } else if weapon.mag_check_t > 0.0 {
            let t = 1.0 - weapon.mag_check_t / crate::weapon::MAG_CHECK_TIME;
            arms = Some((t, WELL_CHECK));
        }
        if let Some((t, chosen)) = arms {
            let (o, r) = inspect_at(t.clamp(0.0, 1.0), inspect_path(chosen), chosen.scale);
            offset += o * damp;
            arm += r * damp;
        }

        Pose {
            offset,
            rotation,
            swing: arm,
            pivot: d.grip,
        }
    }
}

/// One pose of an inspect, and when the motion is at it. Two keys with the same
/// pose at different times are a hold — the beat where the player is actually
/// looking at something — which is the part a set of summed windows cannot do.
struct InspectKey {
    t: f32,
    offset: Vec3,
    /// Yaw, pitch and roll, in the same sense `Pose::rotation` uses.
    rotation: Vec3,
}

/// Which of the two inspect motions a weapon gets, and how big it is.
///
/// The branch is the point: a rifle is turned over to show its other side, and a
/// sidearm is tipped muzzle-down to show the magazine, because that is what
/// there is to look at on each. The scale is the weight — a light machine gun is
/// hefted through a longer arc than a submachine gun is flicked through.
#[derive(Clone, Copy)]
struct InspectFlavour {
    scale: f32,
    turn: bool,
}

/// A turned-over look at the receiver: in, one flank rolled toward the eye,
/// hold, rolled the other way to show the far side and the stock, hold, and
/// back.
///
/// The two holds are *opposite* rotations rather than one growing into the
/// other, because that is what turning something over is: the first shows the
/// side the eye was already behind, and the second shows the side it was not.
/// Two holds on the same side of the weapon, however far apart in angle, read
/// as the same view twice.
const INSPECT_TURN: &[InspectKey] = &[
    InspectKey {
        t: 0.00,
        offset: Vec3::new(0.0, 0.0, 0.0),
        rotation: Vec3::new(0.0, 0.0, 0.0),
    },
    // The wrist rolls in and the muzzle drops a little: the hand starts the
    // turn before the weapon has finished it.
    InspectKey {
        t: 0.08,
        offset: Vec3::new(-0.022, 0.006, -0.014),
        rotation: Vec3::new(-0.07, 0.10, -0.14),
    },
    // One flank toward the eye, with the optic and the fore-end on show.
    InspectKey {
        t: 0.30,
        offset: Vec3::new(-0.080, 0.052, -0.072),
        rotation: Vec3::new(-0.34, 0.16, -0.62),
    },
    // Held while the rail and the optic are read.
    InspectKey {
        t: 0.44,
        offset: Vec3::new(-0.080, 0.052, -0.072),
        rotation: Vec3::new(-0.34, 0.16, -0.62),
    },
    // Over the top and the other way: the far flank, the stock joint, the
    // furniture on the side the eye never sees. The yaw is kept small and the
    // roll does the work — a big yaw here swings the muzzle up past the
    // crosshair, and a weapon being read is not pointed at anything.
    InspectKey {
        t: 0.66,
        offset: Vec3::new(-0.084, 0.030, -0.066),
        rotation: Vec3::new(0.16, -0.06, 0.52),
    },
    InspectKey {
        t: 0.79,
        offset: Vec3::new(-0.084, 0.030, -0.066),
        rotation: Vec3::new(0.16, -0.06, 0.52),
    },
    // Back, and a hair past: the weight carries it before it settles.
    InspectKey {
        t: 0.93,
        offset: Vec3::new(0.010, -0.006, 0.008),
        rotation: Vec3::new(0.04, 0.03, 0.06),
    },
    InspectKey {
        t: 1.00,
        offset: Vec3::new(0.0, 0.0, 0.0),
        rotation: Vec3::new(0.0, 0.0, 0.0),
    },
];

/// A tipped-down look at the magazine well: muzzle down and the well rolled
/// toward the eye, hold, a roll the other way, hold, and back.
const INSPECT_WELL: &[InspectKey] = &[
    InspectKey {
        t: 0.00,
        offset: Vec3::new(0.0, 0.0, 0.0),
        rotation: Vec3::new(0.0, 0.0, 0.0),
    },
    InspectKey {
        t: 0.08,
        offset: Vec3::new(-0.016, 0.006, -0.012),
        rotation: Vec3::new(-0.06, 0.10, -0.10),
    },
    // Muzzle down: this is the whole point of the motion.
    InspectKey {
        t: 0.32,
        offset: Vec3::new(-0.058, 0.016, -0.078),
        rotation: Vec3::new(-0.20, 0.44, -0.30),
    },
    InspectKey {
        t: 0.52,
        offset: Vec3::new(-0.058, 0.016, -0.078),
        rotation: Vec3::new(-0.20, 0.44, -0.30),
    },
    // And the other face of the frame, briefly.
    InspectKey {
        t: 0.70,
        offset: Vec3::new(-0.048, 0.030, -0.058),
        rotation: Vec3::new(0.16, 0.16, 0.44),
    },
    InspectKey {
        t: 0.80,
        offset: Vec3::new(-0.048, 0.030, -0.058),
        rotation: Vec3::new(0.16, 0.16, 0.44),
    },
    InspectKey {
        t: 0.92,
        offset: Vec3::new(0.008, -0.005, 0.006),
        rotation: Vec3::new(0.03, -0.02, 0.05),
    },
    InspectKey {
        t: 1.00,
        offset: Vec3::new(0.0, 0.0, 0.0),
        rotation: Vec3::new(0.0, 0.0, 0.0),
    },
];

/// The magazine check's own motion: shorter, smaller, and only ever tipped
/// toward the well. `Alt`+`T` is a reading of the magazine, not a look at the
/// weapon, so it does not get the rifle's turn.
const WELL_CHECK: InspectFlavour = InspectFlavour {
    scale: 0.72,
    turn: false,
};

fn inspect_flavour(kind: WeaponKind) -> InspectFlavour {
    match kind {
        // A blade has nothing to read but its edge, and it is turned in the
        // fingers rather than hefted.
        WeaponKind::Knife => InspectFlavour {
            scale: 0.80,
            turn: true,
        },
        // A sidearm's magazine is in the grip and the grip is what the eye is
        // already behind: tipped down, it is the first thing you see.
        WeaponKind::Pistol => InspectFlavour {
            scale: 0.86,
            turn: false,
        },
        WeaponKind::Smg => InspectFlavour {
            scale: 0.94,
            turn: true,
        },
        // The two machine pistols are the submachine gun's motion; the
        // bolt-action is heaviest of all because it is the longest thing here.
        WeaponKind::Mp5 | WeaponKind::Mpx => InspectFlavour {
            scale: 0.94,
            turn: true,
        },
        WeaponKind::M4a1 => InspectFlavour {
            scale: 1.00,
            turn: true,
        },
        WeaponKind::Scar => InspectFlavour {
            scale: 1.08,
            turn: true,
        },
        WeaponKind::Fal => InspectFlavour {
            scale: 1.18,
            turn: true,
        },
        WeaponKind::M110 => InspectFlavour {
            scale: 1.14,
            turn: true,
        },
        WeaponKind::Mosin => InspectFlavour {
            scale: 1.24,
            turn: true,
        },
        WeaponKind::Rifle => InspectFlavour {
            scale: 1.06,
            turn: true,
        },
        WeaponKind::Sniper => InspectFlavour {
            scale: 1.10,
            turn: true,
        },
        // Heavier weapons get a longer arc and end up looking at the tube or
        // the belt box, which is what is under them.
        WeaponKind::Shotgun => InspectFlavour {
            scale: 1.20,
            turn: false,
        },
        WeaponKind::Lmg => InspectFlavour {
            scale: 1.26,
            turn: true,
        },
    }
}

fn inspect_path(f: InspectFlavour) -> &'static [InspectKey] {
    if f.turn { INSPECT_TURN } else { INSPECT_WELL }
}

/// The inspect path at `t`, eased between the keys it falls between.
///
/// `smooth` on each segment is what gives the motion its accelerating and
/// braking; two keys holding the same pose is what gives it its stops. Both are
/// the difference between this and a constant-rate rotation, which is the
/// signature of a turntable rather than a hand.
fn inspect_at(t: f32, path: &[InspectKey], scale: f32) -> (Vec3, Vec3) {
    let t = t.clamp(0.0, 1.0);
    let mut i = 0;
    while i + 1 < path.len() && t > path[i + 1].t {
        i += 1;
    }
    let a = &path[i];
    let b = &path[(i + 1).min(path.len() - 1)];
    let span = (b.t - a.t).max(1e-5);
    let u = smooth(((t - a.t) / span).clamp(0.0, 1.0));
    (
        (a.offset + (b.offset - a.offset) * u) * scale,
        (a.rotation + (b.rotation - a.rotation) * u) * scale,
    )
}

/// Muzzle flash: a warm glow, a star of spikes and a short forward cone, all
/// scaled by how bright `weapon.flash` currently is.
fn flash_shape(out: &mut Vec<Tri>, at: Vec3, amount: f32, shots: u32) {
    let amount = amount.clamp(0.0, 1.0);
    // Golden angle: the star lands somewhere new on every shot.
    let seed = shots as f32 * 2.399_963_2;
    let radius = 0.050 + 0.055 * amount;
    let warm = Color::new(1.0, 0.58, 0.20, 0.20 + 0.28 * amount);
    let hot = Color::new(1.0, 0.93, 0.70, 0.85);
    let core = at + vec3(0.0, 0.0, 0.012);

    let spoke = |i: usize, n: usize, r: f32| {
        let a = seed + TAU * i as f32 / n as f32;
        at + vec3(a.cos(), a.sin(), 0.0) * r
    };

    for i in 0..8 {
        let j = (i + 1) % 8;
        out.push(Tri::new(
            core,
            spoke(i, 8, radius * 1.5),
            spoke(j, 8, radius * 1.5),
            warm,
        ));
    }
    for i in 0..7 {
        let j = (i + 1) % 7;
        out.push(Tri::new(
            core,
            spoke(i, 7, radius),
            spoke(j, 7, radius),
            hot,
        ));
    }
    let tip = at + vec3(0.0, 0.0, radius * 1.5);
    for i in 0..4 {
        let j = (i + 1) % 4;
        out.push(Tri::new(
            spoke(i, 4, radius * 0.35),
            spoke(j, 4, radius * 0.35),
            tip,
            hot,
        ));
    }
}

fn project(t: &Tri, viewport: Vec2, f: f32) -> Option<[Vec2; 3]> {
    Some([
        project_point(t.a, viewport, f)?,
        project_point(t.b, viewport, f)?,
        project_point(t.c, viewport, f)?,
    ])
}

/// Same projection the 3D camera applies, minus everything behind the eye.
fn project_point(v: Vec3, viewport: Vec2, f: f32) -> Option<Vec2> {
    if !v.is_finite() || v.z <= NEAR {
        return None;
    }
    Some(vec2(
        viewport.x * 0.5 + (v.x / v.z) * f,
        viewport.y * 0.5 - (v.y / v.z) * f,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::weapon::{BuildEntry, PartId, SlotId};

    /// The def of a weapon whose magazine a fitting replaces, if it has one.
    fn well_len_of(kind: WeaponKind) -> Option<Def> {
        let d = def(kind);
        (d.well_len > 0.0).then_some(d)
    }

    /// The volume of a triangle list's bounding box: how much room a weapon
    /// takes up on the screen.
    ///
    /// A bigger magazine has to buy *bulk*, and where it puts it is up to the
    /// magazine — a box grows downwards, a shotgun's tube forwards, a belt box
    /// outwards, and a fifty-round drum is wider and deeper than the thirty-round
    /// stick it replaces while being no taller at all. Height alone called that
    /// last one a fitting that buys rounds without bulk, which is why the drums
    /// were briefly the size of dinner plates.
    fn bulk_of(tris: &[Tri]) -> f32 {
        let mut lo = Vec3::splat(f32::INFINITY);
        let mut hi = Vec3::splat(f32::NEG_INFINITY);
        for t in tris {
            for p in [t.a, t.b, t.c] {
                lo = lo.min(p);
                hi = hi.max(p);
            }
        }
        let (x, y, z) = (hi - lo).into();
        x.max(0.0) * y.max(0.0) * z.max(0.0)
    }

    /// A weapon's extent with a build fitted, as a bounding box.
    fn extent(kind: WeaponKind, fitted: &WeaponBuild) -> (usize, Vec3, Vec3) {
        let mut tris = build_assembled(kind, !fits_a_magazine(fitted));
        kit_into(&mut tris, kind, fitted);
        let mut lo = Vec3::splat(f32::INFINITY);
        let mut hi = Vec3::splat(f32::NEG_INFINITY);
        for t in &tris {
            for p in [t.a, t.b, t.c] {
                lo = lo.min(p);
                hi = hi.max(p);
            }
        }
        (tris.len(), lo, hi)
    }

    /// Rasterises a triangle list into RGB pixels, the way the armory's portrait
    /// draws it: orthographic on (x, y), painter-sorted on z, flat shaded.
    ///
    /// Not an assertion — a way to *look* at a shape. Geometry is judged by eye
    /// and the armory is four menu presses and a cursor away, so this writes the
    /// pictures instead. `render_ppm` and `render_png` are the two files it
    /// leaves behind.
    fn raster(mesh: &Mesh, w: usize, h: usize) -> Vec<u8> {
        let mut lo = vec2(f32::INFINITY, f32::INFINITY);
        let mut hi = vec2(f32::NEG_INFINITY, f32::NEG_INFINITY);
        for v in &mesh.vertices {
            let q = vec2(v.position.x, v.position.y);
            lo = lo.min(q);
            hi = hi.max(q);
        }
        let span = (hi - lo).max(Vec2::splat(1e-4));
        let scale = ((w as f32 * 0.96) / span.x).min((h as f32 * 0.92) / span.y);
        let mid = (lo + hi) * 0.5;
        let mut buf = vec![20u8; w * h * 3];
        let mut order: Vec<usize> = (0..mesh.indices.len() / 3).collect();
        let depth = |t: usize| {
            let i = t * 3;
            let z = |k: usize| mesh.vertices[mesh.indices[i + k] as usize].position.z;
            (z(0) + z(1) + z(2)) / 3.0
        };
        order.sort_by(|&a, &b| depth(b).total_cmp(&depth(a)));
        for t in order {
            let i = t * 3;
            let pt = |k: usize| {
                let p = mesh.vertices[mesh.indices[i + k] as usize];
                vec2(
                    (p.position.x - mid.x) * scale + w as f32 * 0.5,
                    (p.position.y - mid.y) * scale + h as f32 * 0.5,
                )
            };
            let (a, b, c) = (pt(0), pt(1), pt(2));
            let col = mesh.vertices[mesh.indices[i] as usize].color;
            let rgb = [col[0], col[1], col[2]];
            let x0 = a.x.min(b.x).min(c.x).floor().max(0.0) as usize;
            let x1 = (a.x.max(b.x).max(c.x).ceil().max(0.0) as usize).min(w);
            let y0 = a.y.min(b.y).min(c.y).floor().max(0.0) as usize;
            let y1 = (a.y.max(b.y).max(c.y).ceil().max(0.0) as usize).min(h);
            for y in y0..y1 {
                for x in x0..x1 {
                    let p = vec2(x as f32 + 0.5, y as f32 + 0.5);
                    let e0 = (b.x - a.x) * (p.y - a.y) - (b.y - a.y) * (p.x - a.x);
                    let e1 = (c.x - b.x) * (p.y - b.y) - (c.y - b.y) * (p.x - b.x);
                    let e2 = (a.x - c.x) * (p.y - c.y) - (a.y - c.y) * (p.x - c.x);
                    let inside = (e0 >= 0.0 && e1 >= 0.0 && e2 >= 0.0)
                        || (e0 <= 0.0 && e1 <= 0.0 && e2 <= 0.0);
                    if inside {
                        let o = (y * w + x) * 3;
                        buf[o..o + 3].copy_from_slice(&rgb);
                    }
                }
            }
        }
        buf
    }

    /// The picture `raster` drew, as a PPM file.
    fn render_ppm(mesh: &Mesh, w: usize, h: usize, path: &std::path::Path) {
        let buf = raster(mesh, w, h);
        let mut out = format!(
            "P6
{w} {h}
255
"
        )
        .into_bytes();
        out.extend_from_slice(&buf);
        std::fs::write(path, out).expect("the sheet could not be written");
    }

    /// The same picture as a PNG — the file anybody can actually open, since
    /// most of them have no idea what a PPM is.
    ///
    /// The rows go out as `raster` drew them and let `export_png`'s own flip do
    /// the work, which is what puts the object the right way up: the rasteriser
    /// counts screen rows with the y it was given, and `export_png` is the one
    /// that knows a PNG counts them from the top.
    fn render_png(mesh: &Mesh, w: usize, h: usize, path: &std::path::Path) {
        let rgb = raster(mesh, w, h);
        let mut bytes = vec![0u8; w * h * 4];
        for (i, px) in rgb.chunks_exact(3).enumerate() {
            bytes[i * 4..i * 4 + 3].copy_from_slice(px);
            bytes[i * 4 + 3] = 255;
        }
        let image = Image {
            bytes,
            width: w as u16,
            height: h as u16,
        };
        image.export_png(&path.to_string_lossy());
    }

    /// A triangle list as a mesh, shifted by `at`, so `raster` can draw it.
    fn mesh_of(tris: &[Tri], at: Vec3) -> Mesh {
        let mut mesh = crate::mesh::empty_mesh();
        for t in tris {
            let base = mesh.vertices.len() as u16;
            for p in [t.a, t.b, t.c] {
                mesh.vertices.push(Vertex {
                    position: p + at,
                    uv: Vec2::ZERO,
                    color: t.shaded().into(),
                    normal: Vec4::Y,
                });
            }
            mesh.indices.extend_from_slice(&[base, base + 1, base + 2]);
        }
        mesh
    }

    /// One picture per throwable: the casing as the world draws it on the left,
    /// the same casing as the hand holds it on the right.
    ///
    /// Both renderers, both sizes, in one frame — the sheet is how the shapes
    /// were judged against the real ones (an M67 is a ball with a fuze in the
    /// top of it, an M18 a tin can, an M84 a perforated cylinder, a molotov a
    /// bottle with a rag in its neck), and how the two renderers are compared
    /// against each other by eye rather than only by numbers.
    ///
    /// `cargo test -- --ignored --nocapture render_the_casings`
    #[test]
    #[ignore = "writes a sheet of PNGs under target/casings; run with --ignored"]
    fn render_the_casings() {
        let dir = std::path::Path::new("target/casings");
        std::fs::create_dir_all(dir).unwrap();
        for (i, kind) in [
            GrenadeKind::Frag,
            GrenadeKind::Smoke,
            GrenadeKind::Flash,
            GrenadeKind::Molotov,
        ]
        .iter()
        .enumerate()
        {
            let mut sheet = crate::mesh::empty_mesh();
            let mut world = {
                let mut mb = crate::mesh::MeshBuilder::new();
                crate::grenade::casing_for_test(&mut mb, *kind);
                mb.finish()
            };
            crate::mesh::transform_in_place(
                &mut world,
                Mat4::from_translation(vec3(-0.08, 0.0, 0.0)),
            );
            append(&mut sheet, &world);
            // The right-hand object is what the player sees, fingers and all:
            // drawing the casing on its own is how this sheet said the four
            // shapes were fine while the game showed four colours of fist — the
            // hand is in front of most of a casing it is closed round, and a
            // sheet that leaves it out cannot see that.
            let casing_at = vec3(0.08, 0.0, 0.0);
            let shape = crate::grenade::casing_shape(*kind);
            let mut held = Vec::new();
            held_casing(&mut held, *kind, casing_at);
            // The hand is drawn *from the casing's centre*, the way
            // `held_throwable` draws it: the cradle under the body, at the body's
            // own width.
            throw_hand(
                &mut held,
                casing_at,
                1.0,
                shape.widest() * HELD_HALF,
                shape.bottom() * HELD_HALF,
            );
            let held = mesh_of(&held, Vec3::ZERO);
            append(&mut sheet, &held);
            // The arena's light is dim and this is a sheet for judging shape:
            // lifted towards white so the facets are not four kinds of black.
            crate::mesh::tint_in_place(&mut sheet, WHITE, 0.35, 1.0);
            let file = dir.join(format!("{i}-{kind:?}.png"));
            render_png(&sheet, 560, 360, &file);
            println!("{}", file.display());
        }
    }

    /// Appends `src` to `dst`, keeping what is already in it.
    ///
    /// `copy_into` is the overwrite — it clears the destination — and a sheet is
    /// several objects in one frame. What has to be fixed up is the indices: a
    /// mesh's are its own, so the second object's would point into the first.
    fn append(dst: &mut Mesh, src: &Mesh) {
        let base = dst.vertices.len() as u16;
        dst.vertices.extend_from_slice(&src.vertices);
        dst.indices.extend(src.indices.iter().map(|i| i + base));
    }

    /// Writes one picture per weapon with the issued magazine and with every
    /// magazine that fits it, so the shapes can be compared side by side.
    ///
    /// It draws `preview_mesh` — the armory's own mesh, through the armory's own
    /// projection — rather than assembling the triangles itself, because the
    /// first version did the latter and rendered a picture the armory never
    /// showed: it missed that the preview re-centred on its own contents.
    ///
    /// `cargo test -- --ignored --nocapture render_the_magazines`
    #[test]
    #[ignore = "writes a sheet of PPMs under target/magsheets; run with --ignored"]
    fn render_the_magazines() {
        let dir = std::path::Path::new("target/magsheets");
        std::fs::create_dir_all(dir).unwrap();
        for kind in ALL_KINDS {
            let Some(well) = crate::weapon::def(kind)
                .slots
                .iter()
                .find(|s| s.id == SlotId("magazine"))
            else {
                continue;
            };
            let mut builds: Vec<(String, WeaponBuild)> =
                vec![("issued".to_string(), WeaponBuild::empty())];
            for part in crate::weapon::catalogue()
                .iter()
                .filter(|p| p.slot == SlotId("magazine") && p.interface == well.accepts)
            {
                builds.push((
                    part.id.0.to_string(),
                    WeaponBuild {
                        entries: vec![BuildEntry {
                            slot: SlotId("magazine"),
                            part: part.id,
                            roll: 0.5,
                        }],
                        load: crate::weapon::AmmoKind::Fmj,
                    },
                ));
            }
            for (name, fitted) in builds {
                let mesh = preview_mesh(kind, &fitted, Paint::factory());
                let file = dir.join(format!("{kind:?}__{name}.ppm"));
                render_ppm(&mesh, 460, 240, &file);
                println!("{}", file.display());
            }
        }
    }

    /// A magazine no arm of `kit_into` names still draws.
    ///
    /// This is the net under its last arm, and the reason that arm is there: six
    /// magazines once arrived in the catalogue — the day three 9 mm wells were
    /// split one from another — whose ids no arm knew, and every one of them
    /// drew nothing at all. What the armory showed was a weapon with no magazine
    /// in it, and nothing in the game or in these tests could see the difference
    /// between that and a weapon whose magazine had simply been left off.
    ///
    /// What a forgotten magazine costs from here on is a length, not a magazine:
    /// it is drawn from the well and from the rounds it holds.
    #[test]
    fn a_magazine_no_arm_knows_still_draws() {
        for kind in ALL_KINDS {
            // Only the weapons that offer the slot at all: a knife has nowhere
            // to put a magazine, so a build cannot hand it one.
            if !crate::weapon::def(kind)
                .slots
                .iter()
                .any(|s| s.id == SlotId("magazine"))
            {
                continue;
            }
            let fitted = WeaponBuild {
                entries: vec![BuildEntry {
                    slot: SlotId("magazine"),
                    part: PartId("mag_nobody_has_heard_of"),
                    roll: 0.5,
                }],
                load: crate::weapon::AmmoKind::Fmj,
            };
            let mut tris = Vec::new();
            kit_into(&mut tris, kind, &fitted);
            assert!(
                bulk_of(&tris) > 0.0,
                "{kind:?} draws no magazine for a magazine it does not know"
            );
        }
    }

    /// A box magazine is as long as the rounds it holds.
    ///
    /// The rule `kit_into` draws by, read back off the catalogue: on every well
    /// that is a box, a `mag_`-named part holding fewer rounds than the issued
    /// magazine has to come out shorter than it, and one holding more has to come
    /// out longer. Length is the drop below the mouth of the well, which is the
    /// direction a box magazine grows in and what a player is looking at when two
    /// of them are held up side by side.
    ///
    /// It does not say twice the rounds is twice the length: a magazine's
    /// floorplate and feed lips take up room whatever it holds, so the shipped
    /// lengths run a little under the ratio, deliberately. And it says nothing
    /// about the parts that are not boxes — a drum is wider and deeper than the
    /// stick it replaces while being no taller at all.
    #[test]
    fn a_box_magazine_is_as_long_as_the_rounds_it_holds() {
        let mut checked = 0;
        for kind in ALL_KINDS {
            let d = def(kind);
            if d.well_len <= 0.0 {
                continue;
            }
            let Some(well) = crate::weapon::def(kind)
                .slots
                .iter()
                .find(|s| s.id == SlotId("magazine"))
            else {
                continue;
            };
            let mut issued_mag = Vec::new();
            issued_magazine(&mut issued_mag, &d);
            let issued_drop = drop_below(&issued_mag, d.well.y);
            let issued_rounds =
                crate::weapon::resolve(crate::weapon::def(kind), &WeaponBuild::empty())
                    .as_stats()
                    .mag_size;
            for part in crate::weapon::catalogue()
                .iter()
                .filter(|p| p.slot == SlotId("magazine") && p.interface == well.accepts)
                .filter(|p| p.id.0.starts_with("mag_"))
            {
                let fitted = WeaponBuild {
                    entries: vec![BuildEntry {
                        slot: SlotId("magazine"),
                        part: part.id,
                        roll: 0.5,
                    }],
                    load: crate::weapon::AmmoKind::Fmj,
                };
                let held = crate::weapon::resolve(crate::weapon::def(kind), &fitted)
                    .as_stats()
                    .mag_size;
                let mut tris = Vec::new();
                kit_into(&mut tris, kind, &fitted);
                let drawn = drop_below(&tris, d.well.y);
                let longer = drawn > issued_drop;
                let shorter = drawn < issued_drop;
                if held > issued_rounds {
                    assert!(
                        longer,
                        "{kind:?} with {} holds {held} against the issued {issued_rounds}, \
                         and is drawn no longer ({drawn:.3} against {issued_drop:.3})",
                        part.id.0
                    );
                } else if held < issued_rounds {
                    assert!(
                        shorter,
                        "{kind:?} with {} holds {held} against the issued {issued_rounds}, \
                         and is drawn no shorter ({drawn:.3} against {issued_drop:.3})",
                        part.id.0
                    );
                }
                checked += 1;
            }
        }
        assert!(checked > 0, "no box magazine fits any well");
    }

    /// How far a triangle list reaches below the mouth of a well.
    fn drop_below(tris: &[Tri], mouth: f32) -> f32 {
        tris.iter()
            .map(|t| mouth - t.a.y.max(t.b.y).max(t.c.y))
            .fold(f32::NEG_INFINITY, f32::max)
    }

    /// A fitted magazine is geometry, not a number.
    ///
    /// Every capacity part the catalogue ships has to change the weapon it fits
    /// — the day these parts were added they drew *nothing*, because the kit
    /// geometry was still matched on ids that had just been retired, and nothing
    /// in the weapon table could notice, since a part with no geometry still adds
    /// its rounds. And every magazine that holds *more* rounds than the issued one
    /// has to make the weapon bigger in some direction: what a capacity fitting
    /// buys is bulk, and a fitting that buys rounds without bulk is a number
    /// pretending to be a magazine.
    ///
    /// "Changes" rather than "adds triangles", because a magazine is the one part
    /// that *replaces* what it fits into: the same magazine drawn longer is the
    /// same triangle count.
    ///
    /// Both are read off the catalogue rather than listed, so adding a magazine
    /// cannot leave the test behind. The first version of this named four ids
    /// and counted to seven.
    #[test]
    fn every_shipped_magazine_draws_and_the_bigger_ones_show() {
        let magazines: Vec<_> = crate::weapon::catalogue()
            .iter()
            .filter(|p| p.slot == SlotId("magazine"))
            .copied()
            .collect();
        let mut seen: Vec<&str> = Vec::new();
        let mut checked = 0;
        for kind in ALL_KINDS {
            let Some(well) = crate::weapon::def(kind)
                .slots
                .iter()
                .find(|s| s.id == SlotId("magazine"))
            else {
                continue;
            };
            let stock = extent(kind, &WeaponBuild::empty());
            for part in magazines.iter().filter(|p| p.interface == well.accepts) {
                let fitted = WeaponBuild {
                    entries: vec![BuildEntry {
                        slot: SlotId("magazine"),
                        part: part.id,
                        roll: 0.5,
                    }],
                    load: crate::weapon::AmmoKind::Fmj,
                };
                let now = extent(kind, &fitted);
                assert!(
                    now.1 != stock.1 || now.2 != stock.2,
                    "{kind:?} with {} was drawn the same as the weapon alone",
                    part.id.0
                );
                let rounds = crate::weapon::resolve(crate::weapon::def(kind), &fitted)
                    .as_stats()
                    .mag_size;
                let issued =
                    crate::weapon::resolve(crate::weapon::def(kind), &WeaponBuild::empty())
                        .as_stats()
                        .mag_size;
                let mut fitting = Vec::new();
                kit_into(&mut fitting, kind, &fitted);
                // Whatever else a magazine is, it is geometry — and this is the
                // plainest thing there is to ask. It is asked first because the
                // day three 9 mm wells were split one from another, six magazines
                // arrived that drew nothing at all, and everything below passed:
                // a fitting that draws nothing still draws *differently* from the
                // weapon alone, because fitting it takes the issued magazine
                // away. Not drawing is the failure, so it is the one named.
                assert!(
                    !fitting.is_empty(),
                    "{kind:?} with {} draws no magazine at all",
                    part.id.0
                );
                // A magazine is the one fitting that *replaces* what it goes
                // into, so where the two overlap there is no "it added something"
                // to measure. What can be said is that a fitting which buys more
                // rounds has to buy *room*: "bigger in some direction" means at
                // least one of its three dimensions grows.
                //
                // Not volume, and not height. A fifty-round drum is wider and
                // deeper than the thirty-round stick it replaces while being no
                // taller at all, and its bounding box comes out *smaller* — a drum
                // packs rounds more densely than a raked stick does. Height alone
                // called that drum a fitting that buys rounds without bulk, which
                // is why the drums were briefly the size of dinner plates; volume
                // would call it one too, and fatten the drums to satisfy a metric.
                if let Some(def) = well_len_of(kind) {
                    let dims = |t: &[Tri]| {
                        let mut lo = Vec3::splat(f32::INFINITY);
                        let mut hi = Vec3::splat(f32::NEG_INFINITY);
                        for tri in t {
                            for p in [tri.a, tri.b, tri.c] {
                                lo = lo.min(p);
                                hi = hi.max(p);
                            }
                        }
                        let d = hi - lo;
                        [d.x.max(0.0), d.y.max(0.0), d.z.max(0.0)]
                    };
                    let mut issued_mag = Vec::new();
                    issued_magazine(&mut issued_mag, &def);
                    let fitted_dims = dims(&fitting);
                    let issued_dims = dims(&issued_mag);
                    assert!(
                        fitted_dims != issued_dims,
                        "{kind:?} with {} is drawn as the issued magazine",
                        part.id.0
                    );
                    if rounds > issued {
                        assert!(
                            fitted_dims
                                .iter()
                                .zip(issued_dims.iter())
                                .any(|(f, i)| f > i),
                            "{kind:?} with {} holds {rounds} rounds to the issued {issued} \
                             and measures {fitted_dims:?} w/h/d against {issued_dims:?}",
                            part.id.0
                        );
                    }
                }
                seen.push(part.id.0);
                checked += 1;
            }
        }
        assert!(checked > 0, "no magazine fits anything");
        for part in &magazines {
            assert!(
                seen.contains(&part.id.0),
                "{} fits no weapon in the game",
                part.id.0
            );
        }
    }

    /// The factory finish is the weapon as shipped: every role maps to the
    /// const it has always been, and the parts of a weapon that are not the
    /// weapon — sights, gloves — are outside the table.
    #[test]
    fn the_factory_finish_changes_nothing() {
        let f = &FINISHES[0];
        for c in [STEEL, STEEL_DARK, POLY, RAIL, WOOD, WOOD_DARK, OLIVE] {
            assert_eq!(refinished(c, f), c);
        }
        assert_eq!(refinished(BLACK, &FINISHES[4]), BLACK);
        assert_eq!(refinished(GLOVE, &FINISHES[4]), GLOVE);
    }

    /// Wear at zero is the finish itself; more wear is strictly further from
    /// it; the pattern is deterministic for one seed and not for another.
    #[test]
    fn wear_blends_towards_the_worn_colour() {
        let at = vec3(0.1, -0.2, 0.3);
        let paint = |wear: f32, seed: u32| Paint {
            finish: 0,
            wear,
            seed,
        };
        assert_eq!(painted(STEEL, paint(0.0, 7), at), STEEL);

        let light = painted(STEEL, paint(0.2, 7), at);
        let heavy = painted(STEEL, paint(0.8, 7), at);
        assert!(light.r > STEEL.r && light.r < WORN.r, "{light:?}");
        assert!(heavy.r > light.r, "{heavy:?} vs {light:?}");
        assert_eq!(heavy, painted(STEEL, paint(0.8, 7), at));
        assert_ne!(
            heavy,
            painted(STEEL, paint(0.8, 8), at),
            "the seed did nothing"
        );
    }

    /// A paint survives the settings spelling; junk falls back to the factory
    /// finish rather than failing, one field at a time.
    #[test]
    fn a_paint_round_trips_through_text() {
        for (i, f) in FINISHES.iter().enumerate() {
            let p = Paint {
                finish: i,
                wear: 0.35,
                seed: 12345,
            };
            assert_eq!(Paint::from_text(&p.to_text()), p, "{}", f.id);
        }
        assert_eq!(Paint::from_text(""), Paint::factory());
        assert_eq!(Paint::from_text("nonsense"), Paint::factory());
        let rough = Paint::from_text("abyss@oops:7");
        assert_eq!(rough.finish, 3, "the finish name still read");
        assert_eq!(rough.wear, 0.0, "a broken number falls back");
        assert_eq!(rough.seed, 7);
    }

    /// The preview builds real geometry for every gun, and fitting a part
    /// adds to it — which is what the bench's panel shows.
    #[test]
    fn the_preview_builds_a_mesh() {
        let plain = preview_mesh(WeaponKind::Rifle, &WeaponBuild::empty(), Paint::factory());
        assert!(!plain.vertices.is_empty());
        assert_eq!(plain.vertices.len() % 3, 0);
        assert_eq!(plain.indices.len(), plain.vertices.len());

        let fitted = preview_mesh(
            WeaponKind::Rifle,
            &WeaponBuild {
                entries: vec![BuildEntry {
                    slot: SlotId("optic"),
                    part: PartId("micro_dot"),
                    roll: 0.5,
                }],
                load: crate::weapon::AmmoKind::Fmj,
            },
            Paint::factory(),
        );
        assert!(
            fitted.vertices.len() > plain.vertices.len(),
            "the optic added no geometry"
        );
    }

    /// The inspect turns the weapon about the grip, not about the weapon.
    ///
    /// That is the whole difference between an arm and a turntable: the grip is
    /// where a hand is, so it is the one point that has to stay where the hand
    /// left it while the muzzle and the stock swing round it.
    #[test]
    fn the_inspect_turns_the_weapon_about_the_grip() {
        let d = def(WeaponKind::Rifle);
        let still = Pose {
            offset: Vec3::ZERO,
            rotation: Vec3::ZERO,
            swing: Vec3::ZERO,
            pivot: d.grip,
        };
        let turned = Pose {
            swing: Vec3::new(-0.62, 0.14, -0.44),
            ..still
        };
        let held =
            still.matrix().transform_point3(d.grip) - turned.matrix().transform_point3(d.grip);
        assert!(
            held.length() < 1e-5,
            "the grip slid {:.4} m out of the hand",
            held.length()
        );
        // And the far end, which is the point of the exercise, does move.
        let reach =
            still.matrix().transform_point3(d.muzzle) - turned.matrix().transform_point3(d.muzzle);
        assert!(
            reach.length() > 0.3,
            "the muzzle only moved {:.4} m",
            reach.length()
        );
    }

    /// Every weapon's inspect starts and ends at the rest pose, and holds still
    /// somewhere in between. A path that starts anywhere else is a cut; a path
    /// that never holds is a constant-rate rotation, which is the signature of a
    /// turntable rather than of a hand.
    #[test]
    fn the_inspect_path_rests_at_the_ends_and_holds_in_the_middle() {
        for kind in ALL_KINDS {
            let f = inspect_flavour(kind);
            let path = inspect_path(f);
            let (o0, r0) = inspect_at(0.0, path, f.scale);
            let (o1, r1) = inspect_at(1.0, path, f.scale);
            assert!(
                o0 == Vec3::ZERO && r0 == Vec3::ZERO,
                "{kind:?} does not start at the rest pose"
            );
            assert!(
                o1 == Vec3::ZERO && r1 == Vec3::ZERO,
                "{kind:?} does not come back to rest"
            );
            // Two keys holding one pose, read a tenth of the motion apart.
            let (a, ra) = inspect_at(0.34, path, f.scale);
            let (b, rb) = inspect_at(0.40, path, f.scale);
            assert!(
                a == b && ra == rb,
                "{kind:?} never holds still long enough to read anything"
            );
            assert!(ra != Vec3::ZERO, "{kind:?}'s hold is the rest pose");
        }
    }

    /// The window the pose is measured in, and the field of view it is measured
    /// with. `ViewModel::draw` reads both from the live window, which a test has
    /// none of, so the numbers are the game's own logical resolution.
    const TEST_VIEWPORT: Vec2 = Vec2::new(1280.0, 720.0);
    const TEST_FOVY: f32 = 62.0_f32.to_radians();

    /// How close the weapon comes to a point on the screen, in pixels: the
    /// nearest of its projected points to `at`.
    ///
    /// Unclamped, deliberately. An earlier version measured the distance to the
    /// edge of a guard box and saturated at zero, which made "the weapon is
    /// inside the region at rest and inside it while held" measure as no change
    /// at all — and a hip-fired weapon *is* inside it at rest, because it points
    /// at the crosshair. This keeps growing as the weapon moves away.
    fn closest_to(vm: &ViewModel, kind: WeaponKind, f: f32, at: Vec2) -> Option<f32> {
        let mat = vm
            .pose(&Player::new(Vec3::ZERO), arsenal_with(kind).active_weapon())
            .matrix();
        let mut closest = f32::INFINITY;
        let mut any = false;
        for t in &vm.parts[kind.index()] {
            for p in [t.a, t.b, t.c] {
                let Some(s) = project_point(mat.transform_point3(p), TEST_VIEWPORT, f) else {
                    continue;
                };
                any = true;
                closest = closest.min((s - at).length());
            }
        }
        any.then_some(closest)
    }

    fn arsenal_with(kind: WeaponKind) -> Arsenal {
        Arsenal::new([Some(kind), None])
    }

    /// Something in the other hand must move the weapon off the throw, and must
    /// not move it through the camera.
    ///
    /// The arc a throwable draws leaves the eye along the centre of the screen
    /// and the first half of it runs through where a hip-fired weapon sits — so
    /// the pose that takes the grenade out has to take the weapon with it. This
    /// measures where the weapon actually lands rather than trusting the numbers
    /// in the pose, because numbers that look like they move the gun are how the
    /// muzzle ended up swung *across* the crosshair: positive yaw is muzzle-right
    /// and the weapon already sits right of centre, so the first version of this
    /// pose turned it back over the middle.
    ///
    /// Two properties, both measured, and neither of them "the weapon is not on
    /// screen" — a hip-fired weapon points at the crosshair by design, and a pose
    /// that pulled it that far off would be a different bug:
    ///
    /// - it comes *further* from the region the throw is read in than it does at
    ///   rest, so a pose that moves nothing cannot pass;
    /// - no part of it goes behind the eye, where the projection drops the
    ///   triangle and the weapon grows a hole. Measured against the same weapon
    ///   at rest rather than against zero, because a sniper's stock is already
    ///   behind the eye in the shipped pose.
    #[test]
    fn a_throwable_in_hand_takes_the_weapon_off_the_throw() {
        let f = (TEST_VIEWPORT.y * 0.5) / (TEST_FOVY * 0.5).tan();
        // Where the throw is read: the middle of the band under the crosshair
        // that the arc and its landing mark run through. A throw that lands
        // anywhere from arm's length to the far side of an arena puts its mark
        // near there, and the near half of the arc is drawn across it.
        let around = TEST_VIEWPORT * 0.5 + vec2(0.0, TEST_VIEWPORT.y * 0.10);

        for kind in ALL_KINDS {
            let mut held = ViewModel::new();
            held.set_grabbed(Some(GrenadeKind::Frag));
            // Long enough for the ease to arrive: `HOLD_RATE` gets there in a
            // third of a second, and this is four times that.
            for _ in 0..80 {
                held.update(
                    1.0 / 60.0,
                    Vec2::ZERO,
                    &Player::new(Vec3::ZERO),
                    &arsenal_with(kind),
                );
            }

            let mut idle = ViewModel::new();
            for _ in 0..80 {
                idle.update(
                    1.0 / 60.0,
                    Vec2::ZERO,
                    &Player::new(Vec3::ZERO),
                    &arsenal_with(kind),
                );
            }

            let holding = closest_to(&held, kind, f, around)
                .unwrap_or_else(|| panic!("{kind:?} drew nothing at all while holding"));
            let rest = closest_to(&idle, kind, f, around)
                .unwrap_or_else(|| panic!("{kind:?} drew nothing at rest"));
            // A margin rather than merely "further": a pose that nudges the
            // weapon by a pixel is not a pose that gets it out of the way. Forty
            // pixels is a twentieth of the height — small, and not nothing.
            assert!(
                holding > rest + 40.0,
                "{kind:?} does not move off the throw while one is in hand: nearest point \
                 {holding:.0} px from the middle of the throw against {rest:.0} px at rest",
            );

            // The pose used to be measured against the camera here as well — no
            // part of the weapon may come *nearer* the eye than it does at rest,
            // which is how the version that took the grenade out by throwing the
            // gun at the player's face was caught. That bound is gone with the
            // pose it guarded: the weapon is now put away entirely, and being
            // absent from the frame is a stronger claim than being a bound away
            // from the eye. See `the_weapon_is_put_away_while_a_throwable_is_in_hand`.
        }
    }

    /// The points of a triangle a screen-space test has to look at: its corners,
    /// the middle of each edge, and its centre.
    fn sight_samples(t: &Tri) -> [Vec3; 7] {
        let mid = |a: Vec3, b: Vec3| (a + b) * 0.5;
        [
            t.a,
            t.b,
            t.c,
            mid(t.a, t.b),
            mid(t.b, t.c),
            mid(t.c, t.a),
            (t.a + t.b + t.c) / 3.0,
        ]
    }

    /// Nothing of the weapon is in the frame while a throwable is in the hand.
    ///
    /// "把枪放起来" is exactly this and nothing less. A weapon lowered to the hip
    /// is still a weapon on the screen — on the sight line, with a hand on it —
    /// and the throw the player is reading is drawn through the middle of it. So
    /// the pose is measured rather than trusted: the triangles `assemble` itself
    /// would paint are projected, and none of the seven sampled points of any of
    /// them may land inside the view.
    ///
    /// Seven points and not three corners, because three corners outside a
    /// rectangle do not mean the triangle between them misses it — a weapon
    /// lying across a corner of the frame passes a corner test. A muzzle poking
    /// into the bottom of the frame fails on its own corner either way.
    ///
    /// The hands are left out on purpose: see `weapon_into`. The hand holding the
    /// throwable is supposed to be on the screen at the same moment, which is
    /// what the next test is about.
    #[test]
    fn the_weapon_is_put_away_while_a_throwable_is_in_hand() {
        let f = (TEST_VIEWPORT.y * 0.5) / (TEST_FOVY * 0.5).tan();
        let player = Player::new(Vec3::ZERO);
        for kind in ALL_KINDS {
            let arsenal = arsenal_with(kind);
            let mut vm = ViewModel::new();
            vm.set_grabbed(Some(GrenadeKind::Frag));
            for _ in 0..80 {
                vm.update(1.0 / 60.0, Vec2::ZERO, &player, &arsenal);
            }
            vm.weapon_into(&player, arsenal.active_weapon(), Paint::factory());
            assert!(!vm.body.is_empty(), "{kind:?} drew no weapon at all");
            for t in &vm.body {
                for p in sight_samples(t) {
                    // Behind the eye, where the projection drops the triangle:
                    // not drawn, so not on the screen.
                    let Some(s) = project_point(p, TEST_VIEWPORT, f) else {
                        continue;
                    };
                    assert!(
                        s.x < 0.0 || s.y < 0.0 || s.x > TEST_VIEWPORT.x || s.y > TEST_VIEWPORT.y,
                        "{kind:?} is still on the screen with a throwable in hand: a point of it \
                         lands at {s:?} in a {}x{} view",
                        TEST_VIEWPORT.x,
                        TEST_VIEWPORT.y,
                    );
                }
            }
        }
    }
    /// The throwable in the hand is on the screen, and where the hand is.
    ///
    /// "拿到手上" is exactly this and nothing else: the thing is drawn, in front
    /// of the near plane, and inside the view. A model behind the camera or off
    /// the bottom of the screen is a model that does not exist to anyone playing
    /// the game — which is what "投掷物并没有拿到手上" was.
    #[test]
    fn the_held_throwable_is_on_the_screen() {
        let f = (TEST_VIEWPORT.y * 0.5) / (TEST_FOVY * 0.5).tan();
        let want = vec2(
            TEST_VIEWPORT.x * 0.5 + (HAND_OFFSET.x / HAND_OFFSET.z) * f,
            TEST_VIEWPORT.y * 0.5 - (HAND_OFFSET.y / HAND_OFFSET.z) * f,
        );
        for kind in [
            GrenadeKind::Frag,
            GrenadeKind::Smoke,
            GrenadeKind::Flash,
            GrenadeKind::Molotov,
        ] {
            let mut held = Vec::new();
            held_throwable(&mut held, kind, HAND_OFFSET);
            assert!(!held.is_empty(), "{kind:?} drew nothing in the hand");

            // The casing and the fist, not the forearm: an arm is supposed to
            // leave the frame, which is where the existing hand geometry sends it,
            // so measuring the whole list would measure the arm.
            let (mut lo, mut hi) = (Vec2::splat(f32::INFINITY), Vec2::splat(f32::NEG_INFINITY));
            let mut seen = 0;
            for t in &held {
                for p in [t.a, t.b, t.c] {
                    if p.distance(HAND_OFFSET) > 0.16 {
                        continue;
                    }
                    let s = project_point(p, TEST_VIEWPORT, f)
                        .unwrap_or_else(|| panic!("{kind:?} has a corner behind the near plane"));
                    seen += 1;
                    lo = lo.min(s);
                    hi = hi.max(s);
                }
            }
            assert!(seen > 0, "{kind:?} is not at the hand at all");
            assert!(
                hi.x > 0.0 && lo.x < TEST_VIEWPORT.x && hi.y > 0.0 && lo.y < TEST_VIEWPORT.y,
                "{kind:?} is off the screen: x {:.0}..{:.0}, y {:.0}..{:.0} in a {}x{} view",
                lo.x,
                hi.x,
                lo.y,
                hi.y,
                TEST_VIEWPORT.x,
                TEST_VIEWPORT.y,
            );
            // And centred on the hand, not merely somewhere on screen.
            let mid = (lo + hi) * 0.5;
            assert!(
                (mid - want).length() < 60.0,
                "{kind:?} is drawn at {mid:?} with the hand at {want:?}"
            );

            // And no part of the hand is inside the thing it is holding.
            //
            // That is the whole of what "投掷物和手重合了" was: a fist drawn round a
            // grenade puts the two meshes in the same cubic centimetres, and a
            // player looking at one of them is not looking at the other. A
            // drawn-vertex test cannot ask this — a can's wall has a ring at each
            // end and nothing between them — so the body is read off its profile
            // at the height of each point of the hand. The tolerance is a
            // hundredth of the thing's own half-extent: the fingers are tangent
            // to a body of revolution, and the facets of one meet those of the
            // other only approximately.
            let centre = HAND_OFFSET + vec3(0.0, HELD_LIFT, 0.0);
            let shape = crate::grenade::casing_shape(kind);
            let mut hand = Vec::new();
            throw_hand(
                &mut hand,
                centre,
                1.0,
                shape.widest() * HELD_HALF,
                shape.bottom() * HELD_HALF,
            );
            let mut casing = Vec::new();
            held_casing(&mut casing, kind, centre);
            let top = shape.profile[shape.profile.len() - 1].1;
            for p in hand.iter().flat_map(|t| [t.a, t.b, t.c]) {
                let y = (p.y - centre.y) / HELD_HALF;
                // Above or below the body there is no wall to be inside of.
                if y < shape.bottom() || y > top {
                    continue;
                }
                let r = ((p.x - centre.x).powi(2) + (p.z - centre.z).powi(2)).sqrt() / HELD_HALF;
                assert!(
                    r > shape.radius_at(y) - 0.01,
                    "{kind:?} has the hand inside it: a point {r:.3} of the half-extent out is \
                     inside a body {:.3} wide at {y:.3} up",
                    shape.radius_at(y),
                );
            }
            // And it is *holding* it rather than standing under it. Without this
            // the test above is satisfied by a hand a metre below: everything
            // that keeps the hand out of the body also keeps it out of the
            // picture.
            let finger_top = hand
                .iter()
                .flat_map(|t| [t.a, t.b, t.c])
                .map(|p| p.y)
                .fold(f32::MIN, f32::max);
            let underside = centre.y + shape.bottom() * HELD_HALF;
            assert!(
                (finger_top - underside).abs() < 0.006,
                "{kind:?} is not in the hand: the fingers reach {finger_top:.3} and the body's \
                 underside is at {underside:.3}",
            );
            let top_of = |tris: &[Tri]| {
                tris.iter()
                    .flat_map(|t| [t.a, t.b, t.c])
                    .map(|p| p.y)
                    .fold(f32::MIN, f32::max)
            };
            // And the thing is a *shape* in that hand: its top stands well clear
            // of the fingers, so what a player looks at is the grenade and not the
            // hand under it.
            assert!(
                top_of(&casing) > finger_top + 0.08,
                "{kind:?} is mostly hand: its top is at {:.3} and the fingers reach {:.3}",
                top_of(&casing),
                finger_top,
            );
        }
    }

    /// The widest radius in each of five bands from the bottom of a shape to its
    /// top, in fractions of `half`.
    fn silhouette(points: impl Iterator<Item = Vec3>, half: f32) -> [f32; 5] {
        let points: Vec<Vec3> = points.collect();
        let lo = points.iter().map(|p| p.y).fold(f32::INFINITY, f32::min);
        let hi = points.iter().map(|p| p.y).fold(f32::NEG_INFINITY, f32::max);
        let span = (hi - lo).max(1e-6);
        let mut bands = [0.0f32; 5];
        for p in &points {
            let band = (((p.y - lo) / span * 5.0) as usize).min(4);
            bands[band] = bands[band].max((p.x * p.x + p.z * p.z).sqrt() / half);
        }
        bands
    }

    /// The thing in the hand and the thing in the air are one object.
    ///
    /// Two renderers and one description (`grenade::casing_shape`): this one
    /// draws the profiles out of `Tri`s and the world draws them out of
    /// `MeshBuilder` quads. They cannot share code — a `Mesh` in the world is
    /// not a triangle list in the view — so they are checked against each other
    /// instead, on the one property a player reads at a glance: the silhouette,
    /// sampled in five bands from the bottom to the top.
    ///
    /// Shape, and not triangle counts, because the two spend their faces
    /// differently on the same object: a cap is one degenerate quad per facet in
    /// the mesh and one triangle per facet here, and a prism is six quads in one
    /// and six in the other only if both happen to mean the same thing by it.
    /// Those are bookkeeping. A profile that changed on the way to the hand
    /// would not be — and it would be seen at exactly the moment nobody is
    /// looking, which is the grenade leaving the hand.
    #[test]
    fn the_held_casing_is_the_casing_in_the_air() {
        for kind in [
            GrenadeKind::Frag,
            GrenadeKind::Smoke,
            GrenadeKind::Flash,
            GrenadeKind::Molotov,
        ] {
            let mut tris = Vec::new();
            held_casing(&mut tris, kind, Vec3::ZERO);
            let hand = silhouette(tris.iter().flat_map(|t| [t.a, t.b, t.c]), HELD_HALF);
            let mut mb = crate::mesh::MeshBuilder::new();
            crate::grenade::casing_for_test(&mut mb, kind);
            let world = mb.finish();
            let air = silhouette(
                world.vertices.iter().map(|v| v.position),
                crate::grenade::HALF,
            );
            for (band, (a, b)) in air.iter().zip(hand.iter()).enumerate() {
                // A hundredth of the half-extent: the same numbers drawn twice,
                // so anything above rounding is one renderer doing something the
                // other is not. The four *profiles* are apart by six hundredths
                // (`the_four_casings_are_four_shapes`), which is the size of the
                // drift this is looking for.
                assert!(
                    (a - b).abs() < 0.01,
                    "{kind:?} is a different shape in the hand than in the air: band {band} \
                     is {a:.3} of the half-extent in the world and {b:.3} in the hand",
                );
            }
        }
    }

    /// The throwing hand comes up into place, and the throw and the arc follow it
    /// while it does.
    ///
    /// `main.rs` throws from `throw_hand` and draws the arc from it — one
    /// position for the drawing, the flight and the line, rather than a constant
    /// that the drawing leaves behind. So this pins the three ends of that
    /// motion: below the resting point while the hand is on its way up (a line
    /// drawn from the resting point would hang in the air over a grenade that was
    /// still coming up), exactly on it by the time a throw is possible, and below
    /// it again once the throw has carried the arm away.
    #[test]
    fn the_throwing_hand_rises_into_place_and_leaves_it_again() {
        let player = Player::new(Vec3::ZERO);
        let arsenal = arsenal_with(WeaponKind::Rifle);
        let mut vm = ViewModel::new();
        let step = |vm: &mut ViewModel| vm.update(1.0 / 60.0, Vec2::ZERO, &player, &arsenal);

        vm.set_grabbed(Some(GrenadeKind::Frag));
        step(&mut vm);
        let coming_up = vm.throw_hand();
        assert!(
            coming_up.y < HAND_OFFSET.y - 0.05,
            "the hand is already in place one frame after the thing comes to hand: {coming_up:?} \
             against {HAND_OFFSET:?}",
        );

        // Four times the ease, which `HOLD_RATE` finishes in a sixth of a second.
        for _ in 0..80 {
            step(&mut vm);
        }
        assert!(
            (vm.throw_hand() - HAND_OFFSET).length() < 1e-3,
            "the hand never settles where the arc and the throw are measured from: {:?} against \
             {HAND_OFFSET:?}",
            vm.throw_hand(),
        );

        vm.trigger_grenade_throw();
        vm.set_grabbed(None);
        for _ in 0..10 {
            step(&mut vm);
        }
        assert!(
            vm.throw_hand().y < HAND_OFFSET.y - 0.10,
            "the arm does not follow the thing it threw out of the frame: {:?}",
            vm.throw_hand(),
        );
    }
}
