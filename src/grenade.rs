//! Thrown ordnance: frag and smoke grenades.
//!
//! `Grenades` owns only the bodies in flight. Damage stays with the caller,
//! which owns the enemy list, so `update` hands back `Blast` volumes instead of
//! applying them. The screen a smoke grenade leaves behind lives in
//! `Level::smoke`, where `line_of_sight` already looks for it.
//!
//! The HUD and viewmodel are wired to this module by a separate change, so the
//! full surface is kept live in the meantime.
#![allow(dead_code)]

use crate::effects::Effects;
use crate::i18n::{self, Key};
use crate::level::{Aabb, Level};
use crate::mesh::MeshBuilder;
use crate::util::Rng;
use macroquad::prelude::*;
use std::f32::consts::TAU;
use std::sync::OnceLock;

/// Bodies in flight at once. A throw beyond this is refused and the grenade
/// stays in the pouch; see `Grenades::throw`.
const MAX_LIVE: usize = 8;
/// Body half-extent. Collision and the drawn casing share it, so the grenade
/// that bounces is the grenade you see — and the viewmodel's tests read it to
/// measure the casing in the hand against the casing in the air.
pub(crate) const HALF: f32 = 0.085;
/// Sides a casing is turned with. A grenade is read by its silhouette and a
/// box is a square from every angle but one, which is what it used to be
/// drawn as.
///
/// Read by the viewmodel too, which draws the same object in a hand: the
/// openings in a wall sit in the middle of these facets, so a second count
/// would put the holes on the corners.
pub(crate) const CASING_SIDES: usize = 8;
/// What one casing costs against `MAX_BOXES`, in box-equivalents (24 vertices
/// each), reserved before it is drawn and charged after. An upper bound on the
/// four shapes below, so the smoke — which shares the same budget and is what
/// the budget exists for — cannot be squeezed out by a handful of grenades.
/// The test `every_casing_costs_what_is_reserved_for_it` is what holds it
/// above the real cost.
const CASING_RESERVE: usize = 16;
/// An opening in a casing's wall, in fractions of `HALF`: half its width, half
/// its height. A hole is what a grenade breathes through — and the viewmodel
/// draws the same ones in a hand, off this same number.
pub(crate) const PORT_HALF: f32 = 0.045;
/// How thick the lever is drawn, and the two strands of a bottle's rag, in
/// fractions of `HALF`. Read by the viewmodel as well, which draws the same
/// parts in a hand: a part of the object has to shrink with the object.
pub(crate) const LEVER_HALF: f32 = 0.10;
pub(crate) const WICK_HALF: f32 = 0.075;
/// The second strand, as a fraction of the first.
pub(crate) const WICK_STRAND: f32 = 0.6;
/// What the four casings are made of.
///
/// They are four *objects*, not one shape in four colours. An M67 is an oval
/// with the fuze screwed into the top of it and the safety lever folded down
/// its side; an M18 is a tin can that breathes through a row of vents at each
/// end; an M84 is a cylinder built out of hexagonal openings; a molotov is a
/// bottle with a rag in its neck. A player has one glance at a grenade in the
/// air to know what is coming, and the silhouette is most of that glance — so
/// the whole of what makes them different is written down here, once, in
/// fractions of `HALF` with `y` measured from the casing's centre.
///
/// Once, because two renderers draw it: `casing` puts one in the world and
/// `viewmodel::held_casing` puts the same numbers in a hand. Numbers that
/// disagreed between them would be two different grenades.
pub(crate) struct Casing {
    /// The body of revolution, bottom first, as `(radius, y)`.
    pub profile: &'static [(f32, f32)],
    /// The safety lever down the side, as `(x, y)` at each end: it lies along
    /// the body from the fuze to the shoulder. A bottle has none — there is
    /// nothing on a bottle to pull.
    pub lever: Option<(f32, f32, f32, f32)>,
    /// Rows of openings around the wall, as `(radius, y)`; one opening per
    /// facet, so a row is a ring of `CASING_SIDES`.
    pub ports: &'static [(f32, f32)],
    /// The rag in a bottle's neck, as `(radius, y)` at the end that burns.
    pub wick: Option<(f32, f32)>,
}

/// The frag: a ball with a fuze in the top of it.
///
/// The ball is a ball — five rings of a sphere, and not the barrel with rounded
/// ends that the first version of this was, which read as a light bulb. The
/// fuze is a *step* in the same body of revolution rather than a second object,
/// because that is what it is on the real thing: a cylindrical head screwed into
/// the top of the ovoid, with the lever lying along it.
const FRAG: Casing = Casing {
    profile: &[
        (0.14, -0.94),
        (0.47, -0.70),
        (0.62, -0.30),
        (0.55, 0.10),
        (0.42, 0.34),
        // The shoulder: the fuze head is screwed into the top of the ovoid, so
        // it meets it in a step and not in a taper. Without the step this reads
        // as a pear with a stem, which is what the first version of it was.
        (0.26, 0.38),
        (0.25, 0.44),
        (0.24, 0.80),
        (0.17, 0.94),
    ],
    lever: Some((0.23, 0.82, 0.60, -0.20)),
    ports: &[],
    wick: None,
};

/// The smoke: a tin can, wider than the frag is not, with vents at both ends
/// and a fuze cap. Taller and thinner than anything else in the pouch.
const SMOKE: Casing = Casing {
    profile: &[
        (0.40, -0.98),
        (0.44, -0.88),
        (0.44, 0.44),
        (0.34, 0.54),
        (0.20, 0.60),
        (0.17, 0.90),
        (0.13, 0.96),
    ],
    lever: Some((0.22, 0.86, 0.46, 0.14)),
    ports: &[(0.44, 0.30), (0.44, -0.80)],
    wick: None,
};

/// The flash: the slimmest can of the three, and the only one whose wall is
/// mostly holes — that is what an M84 is, a cylinder of hexagons.
const FLASH: Casing = Casing {
    profile: &[
        (0.30, -0.94),
        (0.34, -0.86),
        (0.34, 0.30),
        (0.28, 0.40),
        (0.16, 0.46),
        (0.15, 0.88),
        (0.11, 0.94),
    ],
    lever: Some((0.20, 0.86, 0.40, -0.14)),
    ports: &[(0.34, 0.12), (0.34, -0.30), (0.34, -0.72)],
    wick: None,
};

/// The bottle: a shoulder, a neck and a rag. Opaque glass, so the body is the
/// kind's own amber and nothing shows through it.
const MOLOTOV: Casing = Casing {
    profile: &[
        (0.18, -0.98),
        (0.40, -0.88),
        (0.42, -0.50),
        (0.40, 0.02),
        (0.28, 0.26),
        (0.15, 0.42),
        (0.14, 0.74),
        (0.18, 0.78),
        (0.15, 0.82),
    ],
    lever: None,
    ports: &[],
    wick: Some((0.10, 1.02)),
};

pub(crate) fn casing_shape(kind: GrenadeKind) -> &'static Casing {
    match kind {
        GrenadeKind::Frag => &FRAG,
        GrenadeKind::Smoke => &SMOKE,
        GrenadeKind::Flash => &FLASH,
        GrenadeKind::Molotov => &MOLOTOV,
    }
}

impl Casing {
    /// The widest radius in the profile, in fractions of the half-extent.
    pub(crate) fn widest(&self) -> f32 {
        self.profile
            .iter()
            .map(|(r, _)| *r)
            .fold(f32::MIN, f32::max)
    }

    /// How far the body runs below its own centre, in the same fractions, as a
    /// negative number.
    pub(crate) fn bottom(&self) -> f32 {
        self.profile
            .iter()
            .map(|(_, y)| *y)
            .fold(f32::MAX, f32::min)
    }

    /// The radius the body has at `y`, interpolated between the two rings it
    /// lies between and clamped to the ends.
    ///
    /// The silhouette, read off the shape rather than off the faces: a can's wall
    /// has a ring at each end and nothing between them, so anything measuring the
    /// *drawn* vertices sees a profile with a hole in the middle of it. Two
    /// callers want this — the fingers of the hand in the viewmodel, which close
    /// on the body at a height of its own rather than at a size of their own, and
    /// the test that requires no part of that hand to be inside it.
    pub(crate) fn radius_at(&self, y: f32) -> f32 {
        if y <= self.profile[0].1 {
            return self.profile[0].0;
        }
        for w in self.profile.windows(2) {
            let ((r0, y0), (r1, y1)) = (w[0], w[1]);
            if y <= y1 {
                let t = if (y1 - y0).abs() < 1e-6 {
                    0.0
                } else {
                    ((y - y0) / (y1 - y0)).clamp(0.0, 1.0)
                };
                return r0 + (r1 - r0) * t;
            }
        }
        self.profile[self.profile.len() - 1].0
    }
}
/// The trajectory preview's step, in seconds, and how many of them it will
/// follow before giving up.
///
/// `1/240` is what a frame's worth of flight is subdivided into at 60 fps
/// (`MIN_SUBSTEPS` of `1/60`), so the preview is the same curve the body will
/// take. `PREVIEW_STEPS` is four seconds of that — longer than any fuse, and
/// longer than any throw in an arena this size. `PREVIEW_SAMPLE` thins the
/// samples out for drawing; the landing point is always kept, whatever the
/// stride.
const PREVIEW_STEP: f32 = 1.0 / 240.0;
const PREVIEW_STEPS: usize = 960;
/// How many of those steps go by between two samples of the drawn line.
///
/// Five is about 0.44 m of flight, which is fine enough that the facets of a
/// curve made of straight segments do not read as facets, and coarse enough that
/// a long throw is under a hundred segments.
const PREVIEW_SAMPLE: usize = 5;
/// Nothing nearer the *origin* of the throw than this is emitted.
///
/// A throw starts in the hand, which is about 0.38 m in front of the eye, so the
/// near plane is no longer the danger it was: this is only about not emitting the
/// origin itself, which would be a zero-length first segment. It is small for
/// that reason — every centimetre here is a gap between the thing in the hand and
/// the start of the line drawn from it.
///
/// The danger it guards against in principle is real, all the same: a marker
/// sitting *on* the camera is not clipped by anything, because macroquad's 3D
/// camera has a 0.01 m near plane (`camera.rs:235`) — the viewer is inside it and
/// its inside faces cover the whole screen. That is what this was when the path
/// began at the eye, and it is what the hand offset fixes for good.
const PREVIEW_NEAR: f32 = 0.05;
const THROW_SPEED: f32 = 21.0;
const THROW_LIFT: f32 = 3.2;
/// Seconds the viewmodel has to play a throw animation.
const THROW_TIME: f32 = 0.55;
const GRAVITY: f32 = 23.0;
const RESTITUTION: f32 = 0.35;
const FRICTION: f32 = 0.7;
const FRAG_FUSE: f32 = 3.2;
const SMOKE_FUSE: f32 = 1.6;
const BLAST_RADIUS: f32 = 7.0;
const BLAST_DAMAGE: f32 = 130.0;
const SMOKE_RADIUS: f32 = 5.0;
const SMOKE_LIFE: f32 = 16.0;

/// The flash: the shortest fuse of the four, because what it is for is being
/// where the enemy is looking *now*, and a light that goes off four seconds
/// after you throw it is a light the enemy has already walked away from.
const FLASH_FUSE: f32 = 1.7;
/// How far the light reaches.
///
/// Wider than the frag's blast by a long way, because it has no damage to trade
/// for the range — and wide enough that a flash thrown *away* from the thrower is
/// still thrown *through* their view: a light that only reached as far as a
/// grenade can be thrown would never blind anybody who was not already on top of
/// it, which is what 11 m turned out to mean. Looking at it is what costs the
/// screen, and looking away is the defence — see `main.rs`'s `Blast::Light`, where
/// the white is scaled by how much of the beam the player's face is in.
const FLASH_RADIUS: f32 = 18.0;
/// How long the light takes the player's own screen, and an enemy's ability to
/// find them. Short: it is a window, not a stun.
pub const FLASH_BLIND: f32 = 3.0;

/// A bottle that never hits anything is a bottle with a short life, not a
/// fuse — see `GrenadeKind::fuse`.
const MOLOTOV_FALLBACK_FUSE: f32 = 2.5;
pub const FIRE_RADIUS: f32 = 4.0;
/// Longer than a smoke screen by half again: what makes a fire a threat is
/// that the ground stays denied after the bottle is gone.
pub const FIRE_LIFE: f32 = 24.0;
/// Damage per second inside the flame. A player caught in one has about three
/// seconds to leave, which is a walk and not a jog.
pub const FIRE_DPS: f32 = 34.0;
/// No substep may carry a body further than this, so a fast throw cannot skip
/// past a wall between two samples.
const MAX_STEP: f32 = 0.15;
const MIN_SUBSTEPS: u32 = 4;
const MAX_SUBSTEPS: u32 = 64;
/// A contact that leaves the body slower than this parks it.
const REST_SPEED: f32 = 0.5;
const EPS: f32 = 1e-3;
/// Fuse time a single frame may consume, so a stalled window cannot swallow a
/// whole fuse and teleport a detonation.
const MAX_DT: f32 = 0.1;
/// `MeshBuilder` indexes with `u16`, so one batch has to stay under 65536
/// vertices — 24 per box.
const MAX_BOXES: usize = 2400;
/// Quads in a box. What a drawn face count is divided by to charge the budget
/// above, which is counted in boxes because that is what used to be drawn.
const BOX_QUADS: usize = 6;

/// One smoke cloud: `(radius fraction, block count, half-size fraction, alpha)`
/// per shell. Blocks overlap, densest at the core, so the cloud reads as an
/// opaque screen instead of a thin haze.
const SMOKE_SHELLS: [(f32, usize, f32, f32); 5] = [
    (0.00, 1, 0.55, 0.90),
    (0.40, 6, 0.36, 0.58),
    (0.64, 10, 0.32, 0.44),
    (0.82, 14, 0.28, 0.34),
    (0.95, 20, 0.26, 0.28),
];

const SMOKE_TINT: Color = Color::new(0.68, 0.70, 0.75, 1.0);
/// A flame's palette from its base to its tips.
///
/// Fire is a *gradient* — hottest and brightest at the bottom, cooling through
/// orange to red and then into the smoke it makes — and that gradient is most of
/// what a still frame of fire says. One orange is a cloud somebody lit; four
/// levels stacked in the right order is fire. See `flame`.
const FLAME_RAMP: [Color; 4] = [
    Color::new(1.00, 0.95, 0.70, 1.0),
    Color::new(1.00, 0.68, 0.22, 1.0),
    Color::new(0.95, 0.36, 0.08, 1.0),
    Color::new(0.56, 0.16, 0.05, 1.0),
];
/// How many flames stand on one burning patch, and how much of the patch's
/// radius each takes up. A molotov's fuel is spread over four metres; four small
/// fires on it read as ground that is alight, where one fire at the centre reads
/// as a campfire with a mist of damage around it.
const FLAME_PATCH: u32 = 4;
const FLAME_SHARE: f32 = 0.30;
/// How tall a flame stands, how much of its own radius it still has at the top,
/// and how far it leans as it streams upward.
const FLAME_HEIGHT: f32 = 1.25;
const FLAME_TAPER: f32 = 0.28;
const FLAME_LEAN: f32 = 0.30;
/// Boxes around one ring of a flame, and how fast it moves.
///
/// The last of those is the quality a still frame cannot carry: fire is the one
/// thing in the arena that is never still, and a photograph of fire is a
/// photograph of orange rocks.
const FLAME_RING: u32 = 6;
const FLAME_FLICKER: f32 = 3.6;
/// What one flame costs against `MAX_BOXES`, in box-equivalents (24 vertices
/// each): a box per tongue, one ring per colour of the ramp.
const FLAME_COST: usize = FLAME_RAMP.len() * FLAME_RING as usize;
/// What is behind the wall of a casing: the openings an M84 is made of, and
/// the inside of the one you are looking at. Its body is the kind's own colour
/// now (`GrenadeKind::color`) rather than a dark casing with a coloured band,
/// which is the shape saying what it is twice over.
const HOLE_TINT: Color = Color::new(0.09, 0.10, 0.12, 1.0);
/// Fuze, lever: the one part of a grenade that is machined.
const METAL_TINT: Color = Color::new(0.55, 0.56, 0.60, 1.0);
/// The rag in a bottle's neck, which is cloth and not steel.
const CLOTH_TINT: Color = Color::new(0.66, 0.62, 0.50, 1.0);

/// A body of revolution about the casing's own up axis: `CASING_SIDES` faces
/// around it, closed at both ends, all of it turned by `rot` about `centre`.
///
/// The mesh primitives are boxes and quads, and a grenade is neither — a box is
/// a square from every angle but one, which is what the casing used to be drawn
/// as. Outward winding throughout, matching `MeshBuilder::add_box`: the faces
/// are lit from their own normals, so an inside-out one would shade wrong even
/// if it were drawn.
///
/// `profile` is `(radius, y)` bottom first, in fractions of `HALF`. Repeated
/// `y` values are how a step is drawn — the shoulder of a bottle, the flare
/// where a fuze meets a body — because a run of faces at one height is an
/// annulus and an annulus is what a step looks like from the side.
fn lathe(mb: &mut MeshBuilder, centre: Vec3, rot: Quat, profile: &[(f32, f32)], color: Color) {
    let step = TAU / CASING_SIDES as f32;
    let ring = |i: usize, r: f32, y: f32| {
        let a = step * i as f32;
        centre + rot * vec3(a.cos() * r * HALF, y * HALF, a.sin() * r * HALF)
    };
    for w in profile.windows(2) {
        let ((r0, y0), (r1, y1)) = (w[0], w[1]);
        for i in 0..CASING_SIDES {
            let j = (i + 1) % CASING_SIDES;
            mb.quad(
                ring(i, r0, y0),
                ring(j, r0, y0),
                ring(j, r1, y1),
                ring(i, r1, y1),
                color,
            );
        }
    }
    // The caps, as a wedge per side: a triangle is a quad with one corner
    // repeated, which is the only form `quad` has. The bottom one winds the
    // other way round so that both face outward.
    let last = profile.len() - 1;
    for (r, y, top) in [
        (profile[0].0, profile[0].1, false),
        (profile[last].0, profile[last].1, true),
    ] {
        let hub = centre + rot * Vec3::Y * (y * HALF);
        for k in 0..CASING_SIDES {
            let j = (k + 1) % CASING_SIDES;
            let (a, b) = (ring(k, r, y), ring(j, r, y));
            let (a, b) = if top { (b, a) } else { (a, b) };
            mb.quad(hub, a, b, hub, color);
        }
    }
}

/// A row of openings around a casing's wall, one per facet, drawn as dark
/// quads lying in the wall's own plane.
///
/// This is what makes an M84 recognizable: its body is perforated, and the
/// pattern of holes reads at a glance where a plain cylinder does not. The
/// quad sits at the facet's own mid-plane radius rather than at the profile's,
/// because the profile gives the radius of the *corners* of a faceted wall — a
/// hole placed at that radius would float a centimetre off the surface.
///
/// Round holes are not on offer: `quad` is what the builder has. At seven
/// millimetres across, on an object seen in flight, a square is a hole.
fn ports(mb: &mut MeshBuilder, centre: Vec3, rot: Quat, wall: f32, y: f32, color: Color) {
    let step = TAU / CASING_SIDES as f32;
    let r = (wall * (std::f32::consts::PI / CASING_SIDES as f32).cos() + 0.006) * HALF;
    let (h, v) = (PORT_HALF * HALF, PORT_HALF * HALF * 1.15);
    for i in 0..CASING_SIDES {
        // Mid-facet azimuth, which is where a facet's own surface plane is.
        let a = step * (i as f32 + 0.5);
        let out = rot * vec3(a.cos(), 0.0, a.sin());
        let side = rot * vec3(-a.sin(), 0.0, a.cos());
        let mid = centre + out * r + rot * Vec3::Y * (y * HALF);
        mb.quad(
            mid - side * h - Vec3::Y * v,
            mid + side * h - Vec3::Y * v,
            mid + side * h + Vec3::Y * v,
            mid - side * h + Vec3::Y * v,
            color,
        );
    }
}

/// One casing in the air: the body, its openings, and whatever names it.
fn casing(mb: &mut MeshBuilder, g: &Live) {
    let rot = Quat::from_axis_angle(g.spin_axis, g.spin_angle);
    let shape = casing_shape(g.kind);
    let body = g.kind.color();
    lathe(mb, g.pos, rot, shape.profile, body);
    for &(wall, y) in shape.ports {
        ports(mb, g.pos, rot, wall, y, HOLE_TINT);
    }
    // The lever and the rag are both a local `(x, y)` pair in the casing's own
    // frame, so the spin carries them round with the body they are attached to.
    let at = |x: f32, y: f32| g.pos + rot * vec3(x * HALF, y * HALF, 0.0);
    if let Some((x0, y0, x1, y1)) = shape.lever {
        mb.add_beam(at(x0, y0), at(x1, y1), HALF * LEVER_HALF, METAL_TINT);
    }
    if let Some((r, y)) = shape.wick {
        let (bx, by) = *shape.profile.last().unwrap_or(&(0.15, 0.8));
        // Two strands rather than one: a rag in a neck is not a rod, and a rod
        // is what the first bottle would have had.
        mb.add_beam(at(bx, by), at(r, y), HALF * WICK_HALF, CLOTH_TINT);
        mb.add_beam(
            at(bx - 0.04, by - 0.10),
            at(r + 0.07, y - 0.09),
            HALF * WICK_HALF * WICK_STRAND,
            CLOTH_TINT,
        );
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum GrenadeKind {
    Frag,
    Smoke,
    /// Light rather than pressure: no damage anywhere, and everything with a
    /// line of sight to it loses the player for a few seconds. The player
    /// looking at it loses their own screen for a moment, which is the same
    /// trade the game already makes with sanity — it costs what you look at.
    Flash,
    /// A bottle. It breaks on whatever it hits rather than on a fuse, and what
    /// it leaves behind is a fire: damage per second and a wall of flame no line
    /// of sight gets through.
    Molotov,
}

impl GrenadeKind {
    pub fn name(self) -> i18n::Key {
        match self {
            GrenadeKind::Frag => Key::GrenadeFrag,
            GrenadeKind::Smoke => Key::GrenadeSmoke,
            GrenadeKind::Flash => Key::GrenadeFlash,
            GrenadeKind::Molotov => Key::GrenadeMolotov,
        }
    }

    pub fn color(self) -> Color {
        match self {
            GrenadeKind::Frag => Color::new(0.47, 0.51, 0.26, 1.0),
            GrenadeKind::Smoke => Color::new(0.58, 0.66, 0.76, 1.0),
            GrenadeKind::Flash => Color::new(0.86, 0.88, 0.92, 1.0),
            GrenadeKind::Molotov => Color::new(0.78, 0.42, 0.16, 1.0),
        }
    }

    /// What the fuse is worth in the hand, and all a bottle needs on the way to
    /// something hard.
    pub fn fuse(self) -> f32 {
        match self {
            GrenadeKind::Frag => FRAG_FUSE,
            GrenadeKind::Smoke => SMOKE_FUSE,
            GrenadeKind::Flash => FLASH_FUSE,
            // A bottle has no fuse. This is only the time it has in the air
            // before it breaks on the floor by itself, in case it never hits
            // anything — its real detonation is the impact.
            GrenadeKind::Molotov => MOLOTOV_FALLBACK_FUSE,
        }
    }

    /// Whether this kind has a pin to pull before it can be thrown.
    ///
    /// A grenade does. A bottle does not: there is nothing to arm, so the throw
    /// is the whole gesture and the first left click is the throw rather than a
    /// step before it. `main.rs` reads this to decide which of the two the click
    /// means.
    pub fn pin(self) -> bool {
        match self {
            GrenadeKind::Frag | GrenadeKind::Smoke | GrenadeKind::Flash => true,
            // A bottle: there is nothing to arm, so the click that would have
            // armed it is the throw.
            GrenadeKind::Molotov => false,
        }
    }
}

/// A resolved detonation, as what it does to the world rather than as one set of
/// numbers.
///
/// The three are not the same event with different scales. A pressure wave hurts
/// whatever is inside its radius and needs a distance curve; a light hurts
/// nobody and takes away the ability to *see*, which is a line-of-sight question
/// and not a distance one; and a fire does not happen at a moment at all — it
/// leaves something behind that goes on working.
pub enum Blast {
    /// Everything within `radius` takes `damage`, by whatever falloff the caller
    /// thinks is right.
    Pressure { pos: Vec3, radius: f32, damage: f32 },
    /// Everything within `radius` with a line of sight to it loses the player
    /// for [`FLASH_BLIND`] seconds. Nobody takes any damage: the whole point of
    /// taking a flash instead of a frag is that it is not a frag.
    Light { pos: Vec3, radius: f32 },
    /// A pool of flame at `pos` for [`FIRE_LIFE`] seconds: damage per second
    /// inside it, and no line of sight through it.
    Fire { pos: Vec3, radius: f32 },
}

impl Blast {
    pub fn pos(&self) -> Vec3 {
        match self {
            Blast::Pressure { pos, .. } | Blast::Light { pos, .. } | Blast::Fire { pos, .. } => {
                *pos
            }
        }
    }

    pub fn radius(&self) -> f32 {
        match self {
            Blast::Pressure { radius, .. }
            | Blast::Light { radius, .. }
            | Blast::Fire { radius, .. } => *radius,
        }
    }
}

struct Live {
    kind: GrenadeKind,
    pos: Vec3,
    vel: Vec3,
    /// Visual tumble only; the collision body stays axis-aligned.
    spin_axis: Vec3,
    spin_rate: f32,
    spin_angle: f32,
    fuse: f32,
    resting: bool,
    dead: bool,
}

/// One casing of `kind`, at the origin and unrotated, in the world's own
/// primitives. What the viewmodel's tests measure their own copy against: the
/// two renderers cannot share code, so they check each other instead.
#[cfg(test)]
pub(crate) fn casing_for_test(mb: &mut MeshBuilder, kind: GrenadeKind) {
    casing(
        mb,
        &Live {
            kind,
            pos: Vec3::ZERO,
            vel: Vec3::ZERO,
            spin_axis: Vec3::Y,
            spin_rate: 1.0,
            spin_angle: 0.0,
            fuse: 0.0,
            resting: false,
            dead: false,
        },
    );
}

pub struct Grenades {
    live: Vec<Live>,
    /// Counts down from `THROW_TIME` after a throw; the viewmodel reads it.
    pub throw_t: f32,
    /// Kind thrown most recently, for the viewmodel to draw in flight.
    pub last_kind: GrenadeKind,
    spin_seed: u64,
}

impl Grenades {
    pub fn new() -> Self {
        Self {
            live: Vec::new(),
            throw_t: 0.0,
            last_kind: GrenadeKind::Frag,
            spin_seed: 0x2545_F491_4F6C_DD1D,
        }
    }

    /// Launch a grenade from `origin` along `dir` (which need not be unit).
    /// False when the field is already holding `MAX_LIVE` of them, in which case
    /// nothing is thrown and the caller keeps the one in the pouch.
    ///
    /// This used to evict the oldest live grenade instead: the ninth in the air
    /// made the first vanish with no bang, no smoke and no sound. Nothing can
    /// reach that today — the armory caps a loadout at seven — which is exactly
    /// why it was worth deciding before a grenade resupply made it live
    /// (review R15).
    pub fn throw(&mut self, kind: GrenadeKind, origin: Vec3, dir: Vec3) -> bool {
        self.spawn(kind, origin, dir, kind.fuse())
    }

    /// The same throw from a hand that has been holding it with the pin already
    /// out, so what is left of the fuse is what it has left.
    ///
    /// A held grenade is thrown with the fuse it has, not a fresh one: the
    /// seconds spent deciding are seconds the fuse was burning, and a fresh fuse
    /// on release would make holding it free.
    pub fn throw_with_fuse(
        &mut self,
        kind: GrenadeKind,
        origin: Vec3,
        dir: Vec3,
        fuse: f32,
    ) -> bool {
        self.spawn(kind, origin, dir, fuse)
    }

    /// A grenade that went off where it stood: the fuse ran out in the player's
    /// hand. Placed at rest and detonated by the next `update`, which is the
    /// same explosion every other one is — the only thing that differs is who
    /// ends up standing in it.
    ///
    /// This one does *not* respect `MAX_LIVE`. The ceiling exists to bound what
    /// the frame has to draw and to step; a live grenade in the player's hand is
    /// not something it can refuse, because the alternative is a fuse that runs
    /// out and nothing happens.
    pub fn drop_live(&mut self, kind: GrenadeKind, at: Vec3) {
        self.live.push(Live {
            kind,
            pos: at,
            vel: Vec3::ZERO,
            spin_axis: Vec3::Y,
            spin_rate: 0.0,
            spin_angle: 0.0,
            fuse: 0.0,
            resting: true,
            dead: false,
        });
        self.last_kind = kind;
    }

    fn spawn(&mut self, kind: GrenadeKind, origin: Vec3, dir: Vec3, fuse: f32) -> bool {
        if self.live.len() >= MAX_LIVE {
            return false;
        }

        let d = dir.normalize_or_zero();
        let d = if d == Vec3::ZERO { Vec3::NEG_Z } else { d };
        let vel = d * THROW_SPEED + Vec3::Y * THROW_LIFT;

        let axis =
            vec3(self.next_unit(), self.next_unit() * 0.4, self.next_unit()).normalize_or_zero();
        let spin_axis = if axis == Vec3::ZERO { Vec3::Y } else { axis };
        let spin_rate = 5.0 + 7.0 * self.next_unit();
        self.live.push(Live {
            kind,
            pos: origin,
            vel,
            spin_axis,
            spin_rate,
            spin_angle: 0.0,
            fuse,
            resting: false,
            dead: false,
        });

        self.last_kind = kind;
        self.throw_t = THROW_TIME;
        true
    }

    /// Advance every body and return the detonations that happened this frame.
    pub fn update(
        &mut self,
        dt: f32,
        level: &mut Level,
        fx: &mut Effects,
        rng: &mut Rng,
    ) -> Vec<Blast> {
        let mut blasts = Vec::new();
        if !dt.is_finite() {
            return blasts;
        }
        let dt = dt.clamp(0.0, MAX_DT);
        self.throw_t = (self.throw_t - dt).max(0.0);
        if dt <= 0.0 {
            return blasts;
        }

        for g in &mut self.live {
            if g.dead {
                continue;
            }
            g.fuse -= dt;

            let mut on_floor = false;
            // Hoisted out of the movement block because the detonation below
            // reads it: a bottle breaks on contact, and contact is decided in
            // there.
            let mut touched = false;
            if !g.resting {
                let speed = g.vel.length();
                let steps =
                    ((speed * dt / MAX_STEP).ceil() as u32).clamp(MIN_SUBSTEPS, MAX_SUBSTEPS);
                let h = dt / steps as f32;

                for _ in 0..steps {
                    if let Some(n) = step(level, &mut g.pos, &mut g.vel, h) {
                        touched = true;
                        // Only an upward face carries weight; a wall scrape must
                        // not park the body in mid-air.
                        on_floor |= n.y > 0.5;
                    }
                    g.spin_angle = (g.spin_angle + g.spin_rate * h).rem_euclid(TAU);
                }

                if touched {
                    separate(level, &mut g.pos, &mut g.vel, &mut on_floor);
                    if on_floor && g.vel.length() < REST_SPEED {
                        g.vel = Vec3::ZERO;
                        g.resting = true;
                    }
                }
            }

            let expired = g.fuse <= 0.0;
            match g.kind {
                GrenadeKind::Frag if expired => {
                    g.dead = true;
                    blasts.push(Blast::Pressure {
                        pos: g.pos,
                        radius: BLAST_RADIUS,
                        damage: BLAST_DAMAGE,
                    });
                    fx.spark(g.pos, Vec3::Y, Color::new(1.0, 0.78, 0.34, 1.0), 44, rng);
                    fx.spark(g.pos, Vec3::Y, Color::new(1.0, 0.42, 0.12, 1.0), 22, rng);
                    fx.smoke(g.pos, Color::new(0.20, 0.18, 0.17, 0.9), 26, rng);
                }
                GrenadeKind::Smoke if on_floor || expired => {
                    g.dead = true;
                    level.add_volume(g.pos, SMOKE_RADIUS, SMOKE_LIFE, 0.0);
                    fx.smoke(g.pos, SMOKE_TINT, 18, rng);
                }
                GrenadeKind::Flash if expired => {
                    g.dead = true;
                    blasts.push(Blast::Light {
                        pos: g.pos,
                        radius: FLASH_RADIUS,
                    });
                    // White, and a lot of it: this is the one detonation in the
                    // game that is supposed to read as *bright*.
                    fx.spark(g.pos, Vec3::Y, Color::new(1.0, 0.98, 0.90, 1.0), 60, rng);
                    fx.spark(g.pos, Vec3::Y, Color::new(1.0, 1.0, 1.0, 1.0), 30, rng);
                    fx.smoke(g.pos, Color::new(0.86, 0.88, 0.92, 0.8), 20, rng);
                }
                // A bottle breaks on what it hits. `on_floor` is not enough: a
                // bottle that clips a wall should break too, so this is any
                // contact at all — and the fuse is the fallback for one that is
                // thrown out over something it never reaches.
                GrenadeKind::Molotov if touched || expired => {
                    g.dead = true;
                    blasts.push(Blast::Fire {
                        pos: g.pos,
                        radius: FIRE_RADIUS,
                    });
                    fx.spark(g.pos, Vec3::Y, Color::new(1.0, 0.62, 0.20, 1.0), 36, rng);
                    fx.smoke(g.pos, Color::new(0.22, 0.19, 0.17, 0.9), 18, rng);
                }
                _ => {}
            }
        }

        self.live.retain(|g| !g.dead);
        blasts
    }

    /// Live bodies, the clouds they left, and whatever is burning, batched into
    /// `dst` for one draw call.
    ///
    /// `t` is the world's clock and only the fires read it: a cloud is a thing
    /// that hangs and a flame is a thing that never stops moving, so one of the
    /// two has to be given the time and it is not the cloud.
    pub fn write_mesh(&self, level: &Level, t: f32, dst: &mut Mesh) {
        let mut mb = MeshBuilder::reclaim(dst);
        let mut budget = MAX_BOXES;

        for g in &self.live {
            if budget < CASING_RESERVE {
                break;
            }
            // Charged what it really costs rather than a number kept beside the
            // drawing: the four are different objects now, and a count written
            // down twice drifts the first time one of them changes.
            let before = mb.quads();
            casing(&mut mb, g);
            budget -= (mb.quads() - before).div_ceil(BOX_QUADS);
        }

        for s in &level.volumes {
            if s.life <= 0.0 {
                continue;
            }
            let r = s.current_radius();
            if r < 0.05 {
                continue;
            }
            let fade = (s.life / s.max_life).clamp(0.0, 1.0).powf(0.45);
            if s.burn > 0.0 {
                // Fire is not a cloud. It has its own palette, its own shape, and
                // it moves — see `flame`.
                patch(&mut mb, s.pos, r, fade, t, &mut budget);
            } else {
                cloud(&mut mb, s.pos, r, fade, &mut budget);
            }
        }

        *dst = mb.finish();
    }

    pub fn live_count(&self) -> usize {
        self.live.len()
    }

    fn next_unit(&mut self) -> f32 {
        self.spin_seed = self
            .spin_seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((self.spin_seed >> 33) as f32) / 2_147_483_647.0
    }
}

/// Where a throw made now would go: the path from the eye to the first thing it
/// would land on, sampled for drawing.
///
/// It runs the same integration `update` does — the same gravity, the same
/// launch, the same swept box against the level — because an arc that disagrees
/// with the throw is worse than no arc at all. The step is the one the
/// simulation itself takes at 60 fps (`MIN_SUBSTEPS` of a frame), so the preview
/// and the flight are the same curve rather than two curves that happen to start
/// together.
///
/// Nothing here is random: the tumble a real grenade has is decoration, and the
/// bend air would put in the path is not modelled anywhere.
///
/// The path starts `PREVIEW_NEAR` out rather than at the eye, because the part
/// of it that is on the eye cannot be drawn — see that constant. A throw into
/// something closer than that emits no samples at all, and the caller draws
/// nothing, which is the right answer for a throw that has nowhere to go.
pub fn preview(level: &Level, origin: Vec3, dir: Vec3, out: &mut Vec<Vec3>) {
    out.clear();
    let d = dir.normalize_or_zero();
    let d = if d == Vec3::ZERO { Vec3::NEG_Z } else { d };
    let mut pos = origin;
    let mut vel = d * THROW_SPEED + Vec3::Y * THROW_LIFT;
    for i in 0..PREVIEW_STEPS {
        let touched = step(level, &mut pos, &mut vel, PREVIEW_STEP).is_some();
        // Where it stopped is the point of the whole exercise: it is the one
        // sample that has to be exact, so the stride does not apply to it.
        if pos.distance(origin) >= PREVIEW_NEAR && (touched || i % PREVIEW_SAMPLE == 0) {
            out.push(pos);
        }
        if touched {
            return;
        }
    }
}

/// A smoke cloud: the shells and seeds it has always been drawn with.
fn cloud(mb: &mut MeshBuilder, at: Vec3, radius: f32, fade: f32, budget: &mut usize) {
    let seeds = shell_seeds();
    for (shell, &(rf, count, sf, alpha)) in SMOKE_SHELLS.iter().enumerate() {
        if *budget < count {
            break;
        }
        *budget -= count;
        let cell = radius * sf;
        for seed in &seeds[shell].boxes {
            let centre = at + seed.dir * (radius * rf);
            let half = Vec3::splat(cell * seed.height);
            mb.add_box(
                centre - half,
                centre + half,
                Color::new(SMOKE_TINT.r, SMOKE_TINT.g, SMOKE_TINT.b, alpha * fade),
            );
        }
    }
}

/// A burning patch of ground: a few small flames spread over it, and nothing
/// else. A real fire makes smoke and this one deliberately does not — the note
/// where a molotov's blast is handled in `main.rs` says why.
fn patch(mb: &mut MeshBuilder, at: Vec3, radius: f32, fade: f32, t: f32, budget: &mut usize) {
    // Flames are light, and none of this is shaded. Nothing else drawn through
    // this builder is; see `MeshBuilder::plain`.
    mb.plain();
    for i in 0..FLAME_PATCH {
        if *budget < FLAME_COST {
            break;
        }
        *budget -= FLAME_COST;
        // Where on the patch this one burns and how big it is: hashed rather than
        // random, because the mesh is rebuilt every frame and a fire that moved
        // house between frames would be a fire that stutters.
        let a = hash01(i, 0, 0x9E37_79B9) * TAU;
        let d = radius * (0.30 + 0.55 * hash01(i, 1, 0x85EB_CA6B));
        let base = at + vec3(a.cos() * d, 0.0, a.sin() * d);
        let scale = 0.75 + 0.5 * hash01(i, 2, 0xC2B2_AE35);
        flame(mb, base, radius * FLAME_SHARE * scale, fade, t, i);
    }
    mb.lit();
}

/// One flame: rings of boxes stacked upward, narrowing as they go, coloured by
/// [`FLAME_RAMP`] from the base to the tips, and moving.
///
/// Three things, because those are the three that make fire read as fire: it is
/// **bright** — drawn unshaded, since a flame is a light and no face of it may be
/// darkened for pointing away from a sun it has nothing to do with (`shade` would
/// put it at 0.46 of its colour and darker than the wall behind it); it is a
/// **gradient**, hot at the bottom and cooling upward; and it is **never still** —
/// every box wobbles on its own phase and the whole flame leans and streams.
///
/// It is a **scatter of separate boxes, and that is the point**. A solid of
/// revolution — one connected surface, rings joined by quads, a smooth ramp up it
/// — was built here and taken back out again: a smooth eight-sided cone with a
/// gradient on it reads as a traffic cone, and what was wanted is tongues of
/// flame with air between them. The pieces are separate on purpose. Do not
/// "fix" them into a body again without asking.
fn flame(mb: &mut MeshBuilder, at: Vec3, radius: f32, fade: f32, t: f32, seed: u32) {
    let levels = FLAME_RAMP.len();
    // A dying fire is a low one: the column sinks with the volume's fade rather
    // than only dimming.
    let height = FLAME_HEIGHT * (0.35 + 0.65 * fade);
    for (level, colour) in FLAME_RAMP.iter().enumerate() {
        let up = level as f32 / (levels - 1) as f32;
        let ring = radius * (1.0 - up * (1.0 - FLAME_TAPER));
        let lean = vec3(
            (t * 0.63 + seed as f32 * 1.7).sin(),
            0.0,
            (t * 0.51 + seed as f32 * 2.3).cos(),
        ) * (FLAME_LEAN * radius * up);
        for i in 0..FLAME_RING {
            let a = TAU * i as f32 / FLAME_RING as f32;
            // Each box on its own phase, so a ring does not pulse in unison and
            // the flame reads as several tongues rather than one breathing blob.
            let phase = hash01(i, level as u32 * 7 + seed, 0x2545_F491) * TAU;
            let flick = 1.0 + (t * FLAME_FLICKER + phase).sin() * 0.24;
            let out = ring * (0.55 + 0.45 * hash01(i, level as u32, 0x27D4_EB2F));
            let centre = at + lean + vec3(a.cos() * out, up * height * flick, a.sin() * out);
            let half = Vec3::splat(radius * 0.34 * flick * (1.0 - up * 0.35));
            mb.add_box(
                centre - half,
                centre + half,
                Color::new(colour.r, colour.g, colour.b, fade),
            );
        }
    }
}

/// One collision-resolved movement substep. Returns the contacted normal when
/// something was hit.
fn step(level: &Level, pos: &mut Vec3, vel: &mut Vec3, h: f32) -> Option<Vec3> {
    vel.y -= GRAVITY * h;
    let delta = *vel * h;
    let dist = delta.length();
    if dist <= 1e-6 {
        return None;
    }
    let dir = delta / dist;

    // Growing each solid by the body's half-extent turns the sweep of a box
    // into a ray test against the Minkowski sum, which for two axis-aligned
    // boxes is exact — no tunnelling at the corners.
    let mut best = dist;
    let mut hit: Option<usize> = None;
    for (i, s) in level.solids.iter().enumerate() {
        let grown = Aabb::new(s.min - Vec3::splat(HALF), s.max + Vec3::splat(HALF));
        match grown.ray(*pos, dir) {
            Some(t) if t > EPS => {
                if t <= best {
                    best = t;
                    hit = Some(i);
                }
            }
            // Already touching: only count it when the destination really
            // overlaps, or a body lying on a surface could never slide along it.
            Some(_) if s.overlaps(&body_box(*pos + delta)) && best > 0.0 => {
                best = 0.0;
                hit = Some(i);
            }
            Some(_) | None => {}
        }
    }

    match hit {
        Some(i) => {
            let p = *pos + dir * best;
            let n = match level.solids.get(i) {
                Some(s) => face_normal(s, p),
                None => Vec3::Y,
            };
            *pos = p + n * EPS;
            bounce(vel, n);
            Some(n)
        }
        None => {
            *pos += delta;
            None
        }
    }
}

/// Reflect the closing velocity about `n` and scrub the tangential part.
fn bounce(vel: &mut Vec3, n: Vec3) {
    let vn = vel.dot(n);
    if vn >= 0.0 {
        return;
    }
    let tangential = *vel - n * vn;
    *vel = tangential * FRICTION - n * (vn * RESTITUTION);
}

/// Push the body out of anything it ended a step inside, shallowest overlap
/// first, and kill the velocity driving it back in.
fn separate(level: &Level, pos: &mut Vec3, vel: &mut Vec3, on_floor: &mut bool) {
    for _ in 0..4 {
        let body = body_box(*pos);
        let mut push = Vec3::ZERO;
        for s in &level.solids {
            if !s.overlaps(&body) {
                continue;
            }
            let x = shallow_axis(s.max.x + HALF - pos.x, pos.x + HALF - s.min.x);
            let y = shallow_axis(s.max.y + HALF - pos.y, pos.y + HALF - s.min.y);
            let z = shallow_axis(s.max.z + HALF - pos.z, pos.z + HALF - s.min.z);
            let (ax, ay, az) = (x.abs(), y.abs(), z.abs());
            let pick = if ax <= ay && ax <= az {
                vec3(x, 0.0, 0.0)
            } else if ay <= az {
                vec3(0.0, y, 0.0)
            } else {
                vec3(0.0, 0.0, z)
            };
            if push == Vec3::ZERO || pick.length_squared() < push.length_squared() {
                push = pick;
            }
        }
        if push == Vec3::ZERO {
            return;
        }
        *pos += push;
        if push.y > 0.0 {
            *on_floor = true;
        }
        if push.x * vel.x < 0.0 {
            vel.x = -vel.x * RESTITUTION;
        }
        if push.y * vel.y < 0.0 {
            vel.y = -vel.y * RESTITUTION;
        }
        if push.z * vel.z < 0.0 {
            vel.z = -vel.z * RESTITUTION;
        }
    }
    unstick(level, pos, vel);
}

/// Last resort for a body wedged somewhere it does not fit — a pinch between a
/// floor slab and the solid resting flush on it, for instance, where pushing
/// clear of one only buries it in the other. Walks it up to the nearest clear
/// height rather than leaving it embedded.
fn unstick(level: &Level, pos: &mut Vec3, vel: &mut Vec3) {
    if free_at(level, *pos) {
        return;
    }
    for step in 1..=120 {
        let probe = vec3(pos.x, pos.y + step as f32 * 0.05, pos.z);
        if free_at(level, probe) {
            *pos = probe;
            *vel = Vec3::ZERO;
            return;
        }
    }
}

fn body_box(pos: Vec3) -> Aabb {
    Aabb::new(pos - Vec3::splat(HALF), pos + Vec3::splat(HALF))
}

fn free_at(level: &Level, pos: Vec3) -> bool {
    let body = body_box(pos);
    !level.solids.iter().any(|s| s.overlaps(&body))
}

/// Signed distance to leave a solid through its lower or upper face. Ties pop
/// upward, so a body buried in the floor comes back out of the top.
fn shallow_axis(to_max: f32, to_min: f32) -> f32 {
    if to_max <= to_min {
        to_max + EPS
    } else {
        -(to_min + EPS)
    }
}

/// The face of `s` nearest to the contact point `p`.
fn face_normal(s: &Aabb, p: Vec3) -> Vec3 {
    let c = (s.min + s.max) * 0.5;
    let e = (s.max - s.min) * 0.5;
    let nx = if e.x > 1e-5 { (p.x - c.x) / e.x } else { 0.0 };
    let ny = if e.y > 1e-5 { (p.y - c.y) / e.y } else { 0.0 };
    let nz = if e.z > 1e-5 { (p.z - c.z) / e.z } else { 0.0 };
    let (ax, ay, az) = (nx.abs(), ny.abs(), nz.abs());
    if ax >= ay && ax >= az {
        vec3(nx.signum(), 0.0, 0.0)
    } else if ay >= az {
        vec3(0.0, ny.signum(), 0.0)
    } else {
        vec3(0.0, 0.0, nz.signum())
    }
}

/// One smoke box before the cloud's radius and fade are applied: the direction
/// out from the cloud centre, and the box's height as a fraction of a cell.
struct SmokeSeed {
    dir: Vec3,
    height: f32,
}

/// The box layout of one shell, in the order the shell draws them.
struct ShellSeed {
    boxes: Vec<SmokeSeed>,
}

/// The box layout of a smoke cloud. It depends on nothing but `SMOKE_SHELLS`, so
/// it is derived once instead of per cloud per frame — `sin_cos` and hashing
/// included.
fn shell_seeds() -> &'static [ShellSeed] {
    static SEEDS: OnceLock<Vec<ShellSeed>> = OnceLock::new();
    SEEDS.get_or_init(|| {
        SMOKE_SHELLS
            .iter()
            .enumerate()
            .map(|(shell, &(_, count, _, _))| ShellSeed {
                boxes: (0..count)
                    .map(|i| {
                        let jitter = hash01(shell as u32, i as u32, 0);
                        SmokeSeed {
                            dir: shell_dir(i, count, shell, jitter),
                            height: 0.85 + 0.3 * hash01(shell as u32, i as u32, 1),
                        }
                    })
                    .collect(),
            })
            .collect()
    })
}

/// Fibonacci sphere point, jittered a little so the lattice does not read as a
/// pattern. Deterministic, so the mesh is stable frame to frame.
fn shell_dir(i: usize, count: usize, shell: usize, jitter: f32) -> Vec3 {
    const GOLDEN: f32 = 2.399_963_2;
    let y = if count <= 1 {
        0.0
    } else {
        1.0 - 2.0 * (i as f32 + 0.5) / count as f32
    };
    let ring = (1.0 - y * y).max(0.0).sqrt();
    let a = GOLDEN * i as f32 + shell as f32 * 1.7;
    let (sin_a, cos_a) = a.sin_cos();
    let d = vec3(cos_a * ring, y, sin_a * ring);
    let d = d + vec3(0.0, (jitter - 0.5) * 0.2, 0.0);
    let n = d.normalize_or_zero();
    if n == Vec3::ZERO { Vec3::Y } else { n }
}

/// Cheap deterministic hash in `0.0 ..= 1.0`.
fn hash01(a: u32, b: u32, salt: u32) -> f32 {
    let mut h = a
        .wrapping_mul(0x9E37_79B9)
        .wrapping_add(b.wrapping_mul(0x85EB_CA6B))
        .wrapping_add(salt.wrapping_mul(0xC2B2_AE35));
    h ^= h >> 15;
    h = h.wrapping_mul(0x2545_F491);
    h ^= h >> 13;
    (h >> 8) as f32 / 16_777_215.0
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A casing's silhouette, sampled: the widest radius in each of five bands
    /// from its bottom to its top, in fractions of `HALF`.
    ///
    /// A silhouette and not a bounding box, because the box of a grenade is
    /// nearly the box of a grenade: what tells an M67 from a bottle at a glance
    /// is *where* it is fat. Five is about as coarse as that can be measured and
    /// still tell a ball from a can.
    fn silhouette(points: &[Vec3]) -> [f32; 5] {
        let lo = points.iter().map(|p| p.y).fold(f32::INFINITY, f32::min);
        let hi = points.iter().map(|p| p.y).fold(f32::NEG_INFINITY, f32::max);
        let span = (hi - lo).max(1e-6);
        let mut bands = [0.0f32; 5];
        for p in points {
            let band = (((p.y - lo) / span * 5.0) as usize).min(4);
            bands[band] = bands[band].max((p.x * p.x + p.z * p.z).sqrt() / HALF);
        }
        bands
    }

    fn casing_points(kind: GrenadeKind) -> Vec<Vec3> {
        let mut mb = MeshBuilder::new();
        casing_for_test(&mut mb, kind);
        mb.finish().vertices.iter().map(|v| v.position).collect()
    }

    const ALL_SHAPES: [GrenadeKind; 4] = [
        GrenadeKind::Frag,
        GrenadeKind::Smoke,
        GrenadeKind::Flash,
        GrenadeKind::Molotov,
    ];

    /// The four casings are four shapes, and not one shape in four colours.
    ///
    /// "所有投掷物的形状相同" was the bug: a barrel with a coloured band on it, so
    /// the only thing an M67 and a bottle had in common was that they were
    /// neither of each other. This measures the silhouettes against one another
    /// rather than trusting that they were authored differently — a profile
    /// copied and never edited would pass a reading of the code and fail here.
    ///
    /// The margin is six hundredths of `HALF` — five millimetres at the size a
    /// casing is drawn in the world — which is about the width of a fuze, and
    /// more than any two of the four are apart by accident.
    #[test]
    fn the_four_casings_are_four_shapes() {
        let shapes: Vec<(GrenadeKind, [f32; 5])> = ALL_SHAPES
            .iter()
            .map(|kind| (*kind, silhouette(&casing_points(*kind))))
            .collect();
        for (i, (a_kind, a)) in shapes.iter().enumerate() {
            for (b_kind, b) in shapes.iter().skip(i + 1) {
                let apart = a
                    .iter()
                    .zip(b.iter())
                    .map(|(x, y)| (x - y).abs())
                    .fold(0.0f32, f32::max);
                assert!(
                    apart > 0.06,
                    "{a_kind:?} and {b_kind:?} have the same silhouette: {a:?} against {b:?}"
                );
            }
        }
    }

    /// The reserve the budget is charged against is above what a casing costs.
    ///
    /// `write_mesh` refuses to draw a casing it has not reserved room for, so a
    /// shape that grew past `CASING_RESERVE` would stop appearing — silently,
    /// and only for the grenade that grew. This is the test that turns that into
    /// a failure at the point the shape changed.
    #[test]
    fn every_casing_costs_what_is_reserved_for_it() {
        for kind in ALL_SHAPES {
            let mut mb = MeshBuilder::new();
            casing_for_test(&mut mb, kind);
            let quads = mb.quads();
            assert!(quads > 0, "{kind:?} drew no casing at all");
            assert!(
                quads <= CASING_RESERVE * BOX_QUADS,
                "{kind:?} costs {quads} quads, which is {} of the {} reserved for it — \
                 raise `CASING_RESERVE` or make the casing smaller",
                quads,
                CASING_RESERVE * BOX_QUADS,
            );
        }
    }

    /// The casings are not boxes.
    ///
    /// The sides of each body of revolution are what a player reads, and a
    /// faceted lathe is the only thing standing between a grenade and the die
    /// with a stripe across it that this used to be. One side direction per
    /// facet, so every kind has at least `CASING_SIDES` of them — counted the
    /// way the flat-shaded renderer counts them, off the baked normals.
    #[test]
    fn no_casing_is_a_box() {
        for kind in ALL_SHAPES {
            let mut mb = MeshBuilder::new();
            casing_for_test(&mut mb, kind);
            let mut sides: Vec<Vec3> = Vec::new();
            for v in &mb.finish().vertices {
                let n = vec3(v.normal.x, 0.0, v.normal.z);
                // A cap faces up or down and has no side direction.
                if n.length() < 0.5 {
                    continue;
                }
                let n = n.normalize();
                if !sides.iter().any(|s| s.dot(n) > 0.999) {
                    sides.push(n);
                }
            }
            assert!(
                sides.len() >= CASING_SIDES,
                "{kind:?} is drawn with {} side directions; a box has four",
                sides.len()
            );
        }
    }

    /// The field holds at most `MAX_LIVE` grenades, and the throw that would
    /// overflow it is refused rather than paid for by making the oldest one
    /// vanish in mid-air — no bang, no smoke, the mesh simply gone (review R15).
    #[test]
    fn a_full_field_refuses_the_next_throw() {
        let mut g = Grenades::new();
        let at = vec3(0.0, 0.0, 0.0);
        for i in 0..MAX_LIVE {
            assert!(
                g.throw(GrenadeKind::Frag, at, Vec3::NEG_Z),
                "throw {i} of a legal count was refused"
            );
        }
        assert_eq!(g.live_count(), MAX_LIVE);

        let before = g.throw_t;
        assert!(
            !g.throw(GrenadeKind::Smoke, at, Vec3::NEG_Z),
            "a grenade past the limit was accepted"
        );
        assert_eq!(
            g.live_count(),
            MAX_LIVE,
            "a live grenade was dropped to make room"
        );
        // The refused throw left nothing of itself behind either: the viewmodel
        // would have drawn a smoke grenade that never existed.
        assert_eq!(g.last_kind, GrenadeKind::Frag);
        assert_eq!(g.throw_t, before, "the refused throw moved the arm");
    }

    /// The thrown casing is not a box.
    ///
    /// Its silhouette is the whole of what tells a player what is sailing at
    /// them, and it used to be one box and a stripe — a die with a line across
    /// it, which is what "the grenade is a cube" was. A box has four distinct
    /// vertical side directions; this counts the casing's sides instead, so the
    /// round body cannot quietly become a box again.
    #[test]
    fn the_thrown_casing_is_round() {
        let level = Level::from_layout(&crate::maps::arena());
        let mut g = Grenades::new();
        assert!(g.throw(GrenadeKind::Frag, vec3(0.0, 2.0, 0.0), Vec3::NEG_Z));
        let mut mesh = crate::mesh::empty_mesh();
        g.write_mesh(&level, 0.0, &mut mesh);
        assert!(!mesh.vertices.is_empty(), "the casing drew nothing");

        let mut sides: Vec<Vec3> = Vec::new();
        for v in &mesh.vertices {
            let n = vec3(v.normal.x, 0.0, v.normal.z);
            if n.length() < 0.5 {
                continue;
            }
            let n = n.normalize();
            if !sides.iter().any(|s| s.dot(n) > 0.999) {
                sides.push(n);
            }
        }
        assert!(
            sides.len() >= CASING_SIDES,
            "the casing has {} distinct side directions; a box has four",
            sides.len()
        );
    }

    /// Runs the field for `seconds`, counting the detonations, at the frame rate
    /// the game runs at. `update` clamps its own step to `MAX_DT`, so a single
    /// call with a long `dt` is not "time passing" — this is.
    fn run(
        g: &mut Grenades,
        level: &mut Level,
        fx: &mut Effects,
        rng: &mut Rng,
        seconds: f32,
    ) -> usize {
        let dt = 1.0 / 60.0;
        let frames = (seconds / dt).ceil() as usize;
        let mut blasts = 0;
        for _ in 0..frames {
            blasts += g.update(dt, level, fx, rng).len();
        }
        blasts
    }

    /// A grenade thrown from a hand that has been holding it keeps the fuse it
    /// has left, and the seconds spent deciding are not free.
    ///
    /// The other half of the same contract: a fresh throw gets the whole fuse,
    /// so the two only differ by what the hand was doing.
    #[test]
    fn a_held_fuse_is_the_fuse_it_is_thrown_with() {
        let mut level = Level::from_layout(&crate::maps::arena());
        let mut fx = Effects::new();
        let mut rng = Rng::new(7);
        let at = level.spawns[0] + vec3(0.0, 1.8, 0.0);
        let whole = GrenadeKind::Frag.fuse();

        let mut fresh = Grenades::new();
        assert!(fresh.throw(GrenadeKind::Frag, at, Vec3::Y));
        assert_eq!(
            run(&mut fresh, &mut level, &mut fx, &mut rng, whole - 0.4),
            0,
            "a fresh grenade went off with 0.4 s of fuse left"
        );
        assert_eq!(
            run(&mut fresh, &mut level, &mut fx, &mut rng, 0.6),
            1,
            "a fresh grenade never went off"
        );

        let mut cooked = Grenades::new();
        assert!(cooked.throw_with_fuse(GrenadeKind::Frag, at, Vec3::Y, 0.4));
        assert_eq!(
            run(&mut cooked, &mut level, &mut fx, &mut rng, 0.3),
            0,
            "a grenade with 0.4 s left went off early"
        );
        assert_eq!(
            run(&mut cooked, &mut level, &mut fx, &mut rng, 0.2),
            1,
            "the fuse a hand had left was not the one it was thrown with"
        );
    }

    /// A fuse that runs out in the hand goes off there, whatever the field's
    /// own ceiling says.
    ///
    /// `MAX_LIVE` exists to bound what a frame has to draw and to step. A live
    /// grenade in a player's hand is not something it can refuse: the
    /// alternative is a fuse that runs out and nothing happens, which is a free
    /// pass nobody should get.
    #[test]
    fn a_cook_off_goes_off_where_it_is_held() {
        let mut level = Level::from_layout(&crate::maps::arena());
        let mut fx = Effects::new();
        let mut rng = Rng::new(11);
        let at = level.spawns[0] + vec3(0.0, 1.6, 0.0);

        let mut g = Grenades::new();
        for _ in 0..MAX_LIVE {
            assert!(g.throw(GrenadeKind::Smoke, vec3(0.0, 40.0, 0.0), Vec3::Y));
        }
        g.drop_live(GrenadeKind::Frag, at);
        assert_eq!(g.live_count(), MAX_LIVE + 1, "the drop was refused");

        // One frame is all it takes: the fuse is already spent when it lands in
        // the field.
        let blasts = g.update(1.0 / 60.0, &mut level, &mut fx, &mut rng);
        assert_eq!(blasts.len(), 1, "a cooked-off grenade did not go off");
        assert!(
            blasts[0].pos().distance(at) < 0.01,
            "it went off {} m from the hand",
            blasts[0].pos().distance(at)
        );
    }

    /// The preview ends where the throw lands.
    ///
    /// This is the whole contract of the arc: a line that stops short of the
    /// wall, or runs through it, is worse than no line at all. Straight down is
    /// the one direction every arena answers the same way — the floor — so the
    /// landing point can be checked against `ground_below` rather than against
    /// another copy of the same physics.
    #[test]
    fn the_preview_lands_where_the_throw_would() {
        let level = Level::from_layout(&crate::maps::arena());
        let at = level.spawns[0];
        let eye = vec3(at.x, at.y + 1.65, at.z);

        let mut path = Vec::new();
        preview(&level, eye, Vec3::NEG_Y, &mut path);
        assert!(path.len() >= 2, "the arc is {} points", path.len());
        // And none of it is on the eye. A sample there is a marker around the
        // camera, not in front of it, and it covers the whole screen — which is
        // what "拿着投掷物会挡视野" turned out to be.
        for p in &path {
            assert!(
                p.distance(eye) >= PREVIEW_NEAR,
                "a sample sits {:.2} m from the eye, inside the {PREVIEW_NEAR} m \
                 that cannot be drawn",
                p.distance(eye)
            );
        }

        // A spawn's own y is the ground it stands on, which is the floor this
        // throw has to find. The body is a box, so the centre stops half an
        // extent above it rather than on it.
        let landed = path.last().copied().expect("a landing point");
        assert!(
            (landed.y - at.y).abs() < 0.35,
            "straight down landed at y {:.3} over ground at {:.3}",
            landed.y,
            at.y
        );
        assert!(
            landed.x == eye.x && landed.z == eye.z,
            "straight down drifted sideways"
        );
    }

    /// A throw with nowhere to go emits nothing, rather than emitting the eye.
    ///
    /// The counterpart of the sample guard: a wall inside `PREVIEW_NEAR` leaves
    /// no part of the path that may be drawn, and an empty path is what the
    /// caller draws nothing from.
    #[test]
    fn a_throw_into_the_eye_itself_draws_nothing() {
        let level = Level::from_layout(&crate::maps::arena());
        let at = level.spawns[0];
        let eye = vec3(at.x, at.y + 1.65, at.z);
        // Straight down inside a wall the arena already has: the floor is 1.65 m
        // below, so a probe from *inside* the floor box has nowhere legal to go.
        let inside = vec3(eye.x, at.y - 0.2, eye.z);
        let mut path = Vec::new();
        preview(&level, inside, Vec3::NEG_Y, &mut path);
        assert!(
            path.len() <= 1,
            "a throw with nowhere to go emitted {} samples",
            path.len()
        );
    }

    /// And the arc is a throw, not a ray: it rises before it falls, and it
    /// reaches further than the distance it was launched from.
    #[test]
    fn the_preview_is_a_parabola_and_not_a_line() {
        let level = Level::from_layout(&crate::maps::arena());
        let at = level.spawns[0];
        let eye = vec3(at.x, at.y + 1.65, at.z);
        // Level, along the axis with the most room, so it has somewhere to go.
        let room = |d: Vec3| level.ray_world(eye, d, 200.0).unwrap_or(200.0);
        let dir = [vec3(1.0, 0.0, 0.0), vec3(-1.0, 0.0, 0.0)]
            .into_iter()
            .chain([vec3(0.0, 0.0, 1.0), vec3(0.0, 0.0, -1.0)])
            .max_by(|a, b| room(*a).total_cmp(&room(*b)))
            .expect("four directions");

        let mut path = Vec::new();
        preview(&level, eye, dir, &mut path);
        let top = path
            .iter()
            .map(|p| p.y)
            .fold(f32::MIN, f32::max)
            .max(f32::MIN);
        assert!(
            top > eye.y + 0.05,
            "the arc never rose above the hand (top {top:.3} against {:.3})",
            eye.y
        );
        let reach = path
            .last()
            .map(|p| vec3(p.x - eye.x, 0.0, p.z - eye.z).length())
            .unwrap_or(0.0);
        assert!(reach > 6.0, "a level throw reached only {reach:.1} m");
    }
    /// Every kind answers about its pin, and only the bottle is thrown by the
    /// click that would have armed it.
    ///
    /// The enum is walked rather than listed, which is the point: `pin` and
    /// `fuse` are matches the compiler already forces to be total, but nothing
    /// forces the *answers* to be sensible, and "every throwable has a pin"
    /// would be a plausible-looking way to get `Molotov` wrong.
    #[test]
    fn every_kind_has_a_pin_unless_it_is_a_bottle() {
        let mut pinned = 0;
        for kind in [
            GrenadeKind::Frag,
            GrenadeKind::Smoke,
            GrenadeKind::Flash,
            GrenadeKind::Molotov,
        ] {
            assert!(kind.fuse() > 0.0, "{kind:?} has no fuse at all");
            if kind.pin() {
                pinned += 1;
            } else {
                assert_eq!(kind, GrenadeKind::Molotov, "{kind:?} has no pin");
            }
        }
        assert_eq!(pinned, 3, "the pin table changed shape");
    }

    /// Runs the field until something goes off and hands back that frame's
    /// detonations. `update` clamps its own step to `MAX_DT`, so a single call
    /// with a whole fuse is not "waiting for the fuse" — this is.
    fn until_blast(
        g: &mut Grenades,
        level: &mut Level,
        fx: &mut Effects,
        rng: &mut Rng,
        seconds: f32,
    ) -> Vec<Blast> {
        let dt = 1.0 / 60.0;
        let frames = (seconds / dt).ceil() as usize;
        for _ in 0..frames {
            let blasts = g.update(dt, level, fx, rng);
            if !blasts.is_empty() {
                return blasts;
            }
        }
        Vec::new()
    }

    /// The three detonations are three different events, and the kinds produce
    /// the ones they are for.
    #[test]
    fn the_kinds_produce_the_detonations_they_are_for() {
        let mut level = Level::from_layout(&crate::maps::arena());
        let mut fx = Effects::new();
        let mut rng = Rng::new(3);
        let at = level.spawns[0] + vec3(0.0, 1.6, 0.0);

        // A frag at rest on the floor: pressure, and it hurts.
        let mut g = Grenades::new();
        assert!(g.throw(GrenadeKind::Frag, at, Vec3::Y));
        let blasts = until_blast(
            &mut g,
            &mut level,
            &mut fx,
            &mut rng,
            GrenadeKind::Frag.fuse() + 0.2,
        );
        assert_eq!(blasts.len(), 1);
        assert!(
            matches!(blasts[0], Blast::Pressure { damage, .. } if damage > 0.0),
            "a frag is a pressure wave"
        );

        // A flash: light, and no damage anywhere in it.
        let mut g = Grenades::new();
        assert!(g.throw(GrenadeKind::Flash, at, Vec3::Y));
        let blasts = until_blast(
            &mut g,
            &mut level,
            &mut fx,
            &mut rng,
            GrenadeKind::Flash.fuse() + 0.2,
        );
        assert_eq!(blasts.len(), 1);
        assert!(
            matches!(blasts[0], Blast::Light { .. }),
            "a flash is not a frag with a longer name"
        );

        // A bottle: fire, and it breaks on the floor without waiting for a fuse.
        let mut g = Grenades::new();
        assert!(g.throw(GrenadeKind::Molotov, at, Vec3::NEG_Y));
        let blasts = until_blast(&mut g, &mut level, &mut fx, &mut rng, 1.0);
        assert_eq!(blasts.len(), 1, "a bottle did not break on the floor");
        assert!(matches!(blasts[0], Blast::Fire { .. }));
    }

    /// A fire blocks sight, because that is what the field says it does: it is
    /// the same list the smoke screens are in.
    #[test]
    fn a_fire_hides_what_smoke_hides() {
        let mut level = Level::from_layout(&crate::maps::arena());
        let at = level.spawns[0];
        let (a, b) = (at + vec3(0.0, 1.0, 0.0), at + vec3(0.0, 1.0, 6.0));
        assert!(
            level.line_of_sight(a, b),
            "the arena has something between two points 6 m apart"
        );
        level.add_volume(a + vec3(0.0, 0.0, 3.0), 4.0, FIRE_LIFE, FIRE_DPS);
        assert!(
            !level.line_of_sight(a, b),
            "a fire in the way did not block sight"
        );
    }
    /// A flame is drawn as light, not as a lit surface.
    ///
    /// `shade` multiplies a colour by 0.44 on a face pointing away from the sun
    /// and by 1.0 on one facing it, so a *lit* flame is a lump of orange that is
    /// brightest towards a sun it has nothing to do with — and darker than the
    /// wall behind it wherever it does not face the sun. That is what this was:
    /// the palette's hottest colour came out of the vertex buffer at
    /// `[112, 106, 78]`, which is 44% of it.
    #[test]
    fn a_flame_is_drawn_as_light() {
        let mut level = Level::from_layout(&crate::maps::arena());
        level.add_volume(vec3(0.0, 0.0, 0.0), FIRE_RADIUS, FIRE_LIFE, FIRE_DPS);
        let g = Grenades::new();
        let mut mesh = crate::mesh::empty_mesh();
        // A fresh volume, so the fade is 1 and the colours are the palette's.
        g.write_mesh(&level, 0.4, &mut mesh);
        assert!(!mesh.vertices.is_empty(), "a fire drew nothing");

        for v in &mesh.vertices {
            assert!(
                FLAME_RAMP.iter().any(|c| crate::mesh::rgba(*c) == v.color),
                "a flame vertex is off-palette ({:?}), so something shaded it",
                v.color
            );
        }
    }

    /// A flame is a column that narrows, and it is never still.
    ///
    /// What a *cloud* cannot do: the shells are as wide at the top as anywhere
    /// else, and they do not move. Both are what tells a player the thing in
    /// front of them is burning rather than orange.
    ///
    /// It is deliberately *not* asserted that the flame is one connected body.
    /// It is a scatter of boxes by choice — see `flame` — so the thing to pin is
    /// that the scatter narrows as it rises and never stops moving, not that it
    /// holds together.
    #[test]
    fn a_flame_is_a_column_and_it_moves() {
        let build = |t: f32| {
            let mut mb = MeshBuilder::new();
            flame(&mut mb, Vec3::ZERO, 1.0, 1.0, t, 0);
            mb.finish()
        };
        let mesh = build(0.0);
        assert!(!mesh.vertices.is_empty(), "the flame drew nothing");

        let lo = mesh
            .vertices
            .iter()
            .map(|v| v.position.y)
            .fold(f32::MAX, f32::min);
        let hi = mesh
            .vertices
            .iter()
            .map(|v| v.position.y)
            .fold(f32::MIN, f32::max);
        let height = hi - lo;
        assert!(
            height > 0.8,
            "a metre-wide flame is only {height:.2} m tall, which is a plate"
        );

        let widest = |from: f32, to: f32| {
            mesh.vertices
                .iter()
                .filter(|v| v.position.y >= lo + height * from && v.position.y <= lo + height * to)
                .map(|v| (v.position.x * v.position.x + v.position.z * v.position.z).sqrt())
                .fold(0.0f32, f32::max)
        };
        let base = widest(0.0, 0.3);
        let tip = widest(0.7, 1.0);
        assert!(
            tip < base * 0.7,
            "the flame does not narrow: {base:.2} m wide at the base, {tip:.2} m at the top"
        );

        // And it moves. Half a second apart, the boxes are somewhere else.
        let moved = build(0.0);
        let later = build(0.5);
        assert_eq!(moved.vertices.len(), later.vertices.len());
        assert!(
            moved
                .vertices
                .iter()
                .zip(&later.vertices)
                .any(|(a, b)| a.position.distance(b.position) > 0.02),
            "the flame is in the same place half a second later"
        );
    }
}
