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

use crate::player::{Player, Stance};
use crate::util::approach;
use crate::weapon::{ALL_KINDS, Arsenal, Weapon, WeaponKind};
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
/// `Weapon::start_inspect` winds its timer down from this.
const INSPECT_TIME: f32 = 2.20;

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
/// Gloved hands. Without them the weapon is the only thing in the frame and
/// reads as floating in front of the camera, which is the first thing anyone
/// notices about a first-person view.
const GLOVE: Color = Color::new(0.30, 0.25, 0.19, 1.0);
const GLOVE_DARK: Color = Color::new(0.19, 0.16, 0.12, 1.0);
const SLEEVE: Color = Color::new(0.31, 0.33, 0.26, 1.0);
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
fn rifle(out: &mut Vec<Tri>) {
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
    bar(
        out,
        vec3(0.0, -0.123, 0.020),
        vec2(0.013, 0.034),
        vec3(0.0, -0.212, 0.062),
        vec2(0.012, 0.030),
        POLY,
    );
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
fn smg(out: &mut Vec<Tri>) {
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
    bar(
        out,
        vec3(0.0, -0.120, 0.000),
        vec2(0.017, 0.042),
        vec3(0.0, -0.268, 0.030),
        vec2(0.016, 0.038),
        POLY,
    );
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
fn sniper(out: &mut Vec<Tri>) {
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
    box3(
        out,
        vec3(-0.020, -0.185, -0.090),
        vec3(0.020, -0.122, 0.020),
        POLY,
    );
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
    box3(
        out,
        vec3(-0.072, -0.238, -0.060),
        vec3(0.072, -0.138, 0.110),
        STEEL_DARK,
    );
    bar(
        out,
        vec3(0.0, -0.188, 0.110),
        vec2(0.072, 0.050),
        vec3(0.0, -0.188, 0.170),
        vec2(0.044, 0.030),
        STEEL_DARK,
    );
    bar(
        out,
        vec3(0.0, -0.188, -0.060),
        vec2(0.072, 0.050),
        vec3(0.0, -0.188, -0.120),
        vec2(0.044, 0.030),
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

fn build(kind: WeaponKind) -> Vec<Tri> {
    let mut out = Vec::new();
    match kind {
        WeaponKind::Rifle => rifle(&mut out),
        WeaponKind::Shotgun => shotgun(&mut out),
        WeaponKind::Smg => smg(&mut out),
        WeaponKind::Sniper => sniper(&mut out),
        WeaponKind::Lmg => lmg(&mut out),
        WeaponKind::Pistol => pistol(&mut out),
        WeaponKind::Knife => knife(&mut out),
    }
    out
}

/// One gloved hand closed around `at`, with the forearm running back and
/// outboard. `side` is +1 for the right hand and -1 for the left, and is the
/// only thing that differs between the two.
fn hand(out: &mut Vec<Tri>, at: Vec3, side: f32) {
    // Fist, wrapped round whatever it is holding.
    box3(
        out,
        at + vec3(-0.036, -0.050, -0.044),
        at + vec3(0.036, 0.034, 0.044),
        GLOVE,
    );
    // Knuckles on top, so it reads as a hand rather than a cube.
    box3(
        out,
        at + vec3(-0.030, 0.030, -0.028),
        at + vec3(0.030, 0.046, 0.028),
        GLOVE_DARK,
    );
    // Thumb, folded over the near side of the grip.
    box3(
        out,
        at + vec3(side * 0.020, -0.014, 0.024),
        at + vec3(side * 0.047, 0.024, 0.062),
        GLOVE_DARK,
    );
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

fn hands_for(kind: WeaponKind) -> Vec<Tri> {
    let d = def(kind);
    let mut out = Vec::new();
    hand(&mut out, d.grip, 1.0);
    if let Some(at) = d.support {
        hand(&mut out, at, -1.0);
    }
    out
}

/// Eased 0..1 ramp, flat at both ends.
fn smooth(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Smooth 0 -> 1 -> 0 window over `t` in 0..1: swells out over the first `out`,
/// holds, and falls back over the last `back`. Used by every timed motion, so
/// none of them can start or stop with a jerk.
fn swing(t: f32, out: f32, back: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    if t < out {
        smooth(t / out)
    } else if t > 1.0 - back {
        smooth((1.0 - t) / back)
    } else {
        1.0
    }
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
        },
        WeaponKind::Shotgun => Def {
            hip: vec3(0.165, -0.112, 0.420),
            rotation: vec3(-0.140, 0.060, 0.090),
            aim: vec3(0.0, 0.0, 0.365),
            muzzle: vec3(0.0, -0.094, 0.746),
            grip: vec3(0.0, -0.176, -0.076),
            support: Some(vec3(0.0, -0.164, 0.120)),
        },
        WeaponKind::Smg => Def {
            hip: vec3(0.130, -0.077, 0.360),
            rotation: vec3(-0.100, 0.030, 0.040),
            aim: vec3(0.0, 0.0, 0.305),
            muzzle: vec3(0.0, -0.076, 0.406),
            grip: vec3(0.0, -0.155, -0.096),
            support: Some(vec3(0.0, -0.148, 0.158)),
        },
        WeaponKind::Sniper => Def {
            hip: vec3(0.170, -0.102, 0.440),
            rotation: vec3(-0.150, 0.050, 0.100),
            aim: vec3(0.0, 0.0, 0.340),
            muzzle: vec3(0.0, -0.085, 1.006),
            grip: vec3(0.0, -0.158, -0.130),
            support: Some(vec3(0.0, -0.146, 0.215)),
        },
        WeaponKind::Lmg => Def {
            hip: vec3(0.175, -0.117, 0.430),
            rotation: vec3(-0.130, 0.070, 0.070),
            aim: vec3(0.0, 0.0, 0.360),
            muzzle: vec3(0.0, -0.095, 0.891),
            grip: vec3(0.0, -0.190, -0.138),
            support: Some(vec3(0.0, -0.172, 0.300)),
        },
        WeaponKind::Pistol => Def {
            hip: vec3(0.140, -0.047, 0.355),
            rotation: vec3(-0.090, 0.035, 0.055),
            aim: vec3(0.0, 0.0, 0.300),
            muzzle: vec3(0.0, -0.036, 0.132),
            grip: vec3(0.0, -0.118, -0.066),
            support: None,
        },
        WeaponKind::Knife => Def {
            hip: vec3(0.150, -0.057, 0.350),
            rotation: vec3(-0.105, 0.045, 0.085),
            aim: vec3(0.0, 0.0, 0.330),
            muzzle: vec3(0.0, 0.0, 0.212),
            grip: vec3(0.0, -0.072, -0.074),
            support: None,
        },
    }
}

fn slot(kind: WeaponKind) -> usize {
    match kind {
        WeaponKind::Rifle => 0,
        WeaponKind::Shotgun => 1,
        WeaponKind::Smg => 2,
        WeaponKind::Sniper => 3,
        WeaponKind::Lmg => 4,
        WeaponKind::Pistol => 5,
        WeaponKind::Knife => 6,
    }
}

/// Where the weapon sits and how it is turned, in view space.
struct Pose {
    offset: Vec3,
    /// Yaw, pitch and roll about the weapon's own origin. Pitch is positive
    /// muzzle-down, yaw positive muzzle-right.
    rotation: Vec3,
}

impl Pose {
    fn matrix(&self) -> Mat4 {
        Mat4::from_translation(self.offset)
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
    parts: [Vec<Tri>; 7],
    /// The hands that hold each weapon. Kept apart from `parts` because they do
    /// not move with it — see `HAND_LAG`.
    hands: [Vec<Tri>; 7],
    /// Scratch for `draw`. The body and the flash are rebuilt from `parts` every
    /// frame anyway; collecting them into a fresh vector each time was a handful
    /// of allocations a frame for geometry that never changes size.
    body: Vec<Tri>,
    flash: Vec<Tri>,
}

impl ViewModel {
    pub fn new() -> Self {
        Self {
            sway: Vec2::ZERO,
            switch_drop: 0.0,
            crouch: 0.0,
            prone: 0.0,
            grenade_t: 0.0,
            // One entry per kind, in `slot` order.
            parts: ALL_KINDS.map(build),
            hands: ALL_KINDS.map(hands_for),
            body: Vec::new(),
            flash: Vec::new(),
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
    }

    /// Plays a short lob/throw motion. Called when the player throws a grenade.
    pub fn trigger_grenade_throw(&mut self) {
        self.grenade_t = GRENADE_TIME;
    }

    /// Call in the 2D pass, after `set_default_camera()`.
    pub fn draw(&mut self, player: &Player, arsenal: &Arsenal) {
        let viewport = vec2(screen_width(), screen_height());
        let fovy = player.fovy();
        if viewport.x < 1.0 || viewport.y < 1.0 || fovy <= 0.0 {
            return;
        }
        // Matches the 3D camera, so the weapon keeps the right scale.
        let f = (viewport.y * 0.5) / (fovy * 0.5).tan();

        let weapon = arsenal.active_weapon();
        let pose = self.pose(player, weapon);
        let mat = pose.matrix();

        let parts = &self.parts[slot(weapon.kind)];
        let hand_parts = &self.hands[slot(weapon.kind)];
        let body = &mut self.body;
        body.clear();
        body.extend(parts.iter().map(|t| t.map(mat)).filter(Tri::finite));
        // The hands are held slightly back from the weapon's sway, so a weapon
        // being swung around does not read as one rigid object with arms welded
        // to it.
        let lag = mat * Mat4::from_translation(vec3(-self.sway.x, -self.sway.y, 0.0) * HAND_LAG);
        body.extend(hand_parts.iter().map(|t| t.map(lag)).filter(Tri::finite));
        // Painter's algorithm: farthest faces first, nearer ones cover them.
        body.sort_by(|x, y| y.depth().total_cmp(&x.depth()));
        for t in body.iter() {
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

        // Inspecting and throwing are the two things the hands do on their own.
        // The throw is the louder of the two, so it takes over from the inspect
        // rather than adding to it, and neither one fights a reload — both are
        // weighted by what the reload has taken.
        let throw_w = if self.grenade_t > 0.0 {
            let t = 1.0 - (self.grenade_t / GRENADE_TIME).clamp(0.0, 1.0);
            // Out fast and back slow: the throw itself is a flick of the arm.
            swing(t, 0.28, 0.58) * (1.0 - reload)
        } else {
            0.0
        };
        if throw_w > 0.0 {
            // The weapon falls away as the off hand lobs the grenade: dropped,
            // rolled over and tipped muzzle-down. Most of the drop is in the
            // tip, so the grip stays near the eye where the hand is.
            offset += vec3(0.045 * hip, -0.220 * hip, 0.020) * throw_w;
            rotation += vec3(-0.170 * hip, 0.420 * hip, -0.420) * throw_w;
        }

        // Inspect: the weapon is turned inward and canted over so the player can
        // look it over, then put back. The timer runs from 2.2 s down to 0, and
        // the window is zero at both ends, so the motion cannot snap.
        let inspect_w = if weapon.inspect_t > 0.0 {
            let t = 1.0 - (weapon.inspect_t / INSPECT_TIME).clamp(0.0, 1.0);
            swing(t, 0.30, 0.34) * (1.0 - reload) * (1.0 - throw_w)
        } else {
            0.0
        };
        if inspect_w > 0.0 {
            offset += vec3(-0.075 * hip, 0.035 * hip, -0.080) * inspect_w;
            rotation += vec3(-0.520 * hip, -0.240 * hip, -1.050) * inspect_w;
        }

        Pose { offset, rotation }
    }
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
