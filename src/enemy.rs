use crate::level::{Aabb, Level, STEP_HEIGHT, move_and_collide};
use crate::mesh::{MeshBuilder, tint_mesh, transform_mesh};
use crate::player::Player;
use crate::util::{Rng, approach, yaw_of};
use macroquad::prelude::*;
use std::f32::consts::{FRAC_PI_2, PI, TAU};

const GRAVITY: f32 = 23.0;

/// How close to the last known position counts as having searched it. A unit
/// that has lost sight of the player walks back to where it last saw them and
/// then holds; this is the radius that counts as "there".
const SEARCH_REACH: f32 = 1.5;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    /// Balanced ranged infantry.
    Grunt,
    /// Fast melee charger.
    Rusher,
    /// Slow, heavily armoured, fires three-round bursts.
    Heavy,
    /// Long-range marksman that telegraphs its shot before firing.
    Sniper,
    /// Hovering flying skirmisher.
    Drone,
    /// Cosmic horror. Lumbers forward and lashes with tentacles from beyond
    /// arm's length; spends its last breath birthing spawnlings.
    Tentacle,
    /// Cosmic horror. A walking mountain: slow, enormous, and all but
    /// immune to small arms.
    Colossus,
    /// Cosmic horror. Tiny, mindless and fast; exists to swarm.
    Spawnling,
}

pub struct KindStats {
    pub health: f32,
    pub speed: f32,
    pub radius: f32,
    pub height: f32,
    pub damage: f32,
    pub interval: f32,
    pub range: f32,
    /// Distance the AI tries to hold from the player.
    pub standoff: f32,
    /// Melee reach. `0.0` means this archetype is a shooter.
    pub reach: f32,
    /// Sanity lost per second while the player has line of sight to one of these.
    pub sanity_drain: f32,
    /// How many `Spawnling`s burst out of the corpse (body horror).
    pub spawns_on_death: u32,
    pub score: u32,
    pub body: Color,
    pub trim: Color,
    pub visor: Color,
    pub flying: bool,
    pub hover: f32,
    pub burst: u32,
    /// Seconds of aiming before the shot lands. Telegraphs snipers.
    pub windup: f32,
    /// Multiplies accuracy spread; lower is more accurate.
    pub accuracy: f32,
}

pub fn stats(kind: Kind) -> KindStats {
    match kind {
        Kind::Grunt => KindStats {
            health: 62.0,
            speed: 2.9,
            radius: 0.42,
            height: 1.78,
            damage: 8.0,
            interval: 1.05,
            range: 24.0,
            standoff: 12.0,
            reach: 0.0,
            sanity_drain: 0.0,
            spawns_on_death: 0,
            score: 100,
            body: Color::new(0.26, 0.46, 0.32, 1.0),
            trim: Color::new(0.16, 0.30, 0.22, 1.0),
            visor: Color::new(1.0, 0.42, 0.16, 1.0),
            flying: false,
            hover: 0.0,
            burst: 0,
            windup: 0.0,
            accuracy: 1.0,
        },
        Kind::Rusher => KindStats {
            health: 44.0,
            speed: 5.6,
            radius: 0.36,
            height: 1.62,
            damage: 13.0,
            interval: 0.85,
            range: 2.0,
            standoff: 0.0,
            // Exactly its old `radius + 0.85` melee test, now data.
            reach: 0.36 + 0.85,
            sanity_drain: 0.0,
            spawns_on_death: 0,
            score: 150,
            body: Color::new(0.58, 0.20, 0.16, 1.0),
            trim: Color::new(0.36, 0.12, 0.10, 1.0),
            visor: Color::new(1.0, 0.85, 0.25, 1.0),
            flying: false,
            hover: 0.0,
            burst: 0,
            windup: 0.0,
            accuracy: 1.0,
        },
        Kind::Heavy => KindStats {
            health: 185.0,
            speed: 1.85,
            radius: 0.60,
            height: 2.30,
            damage: 15.0,
            interval: 1.75,
            range: 26.0,
            standoff: 14.0,
            reach: 0.0,
            sanity_drain: 0.0,
            spawns_on_death: 0,
            score: 320,
            body: Color::new(0.33, 0.27, 0.52, 1.0),
            trim: Color::new(0.20, 0.16, 0.33, 1.0),
            visor: Color::new(1.0, 0.38, 0.92, 1.0),
            flying: false,
            hover: 0.0,
            burst: 3,
            windup: 0.0,
            accuracy: 1.1,
        },
        Kind::Sniper => KindStats {
            health: 55.0,
            speed: 2.2,
            radius: 0.40,
            height: 1.80,
            damage: 34.0,
            interval: 2.40,
            range: 55.0,
            standoff: 24.0,
            reach: 0.0,
            // The scope glint is bad for morale.
            sanity_drain: 1.0,
            spawns_on_death: 0,
            score: 260,
            body: Color::new(0.44, 0.40, 0.22, 1.0),
            trim: Color::new(0.28, 0.25, 0.14, 1.0),
            visor: Color::new(0.35, 1.0, 0.55, 1.0),
            flying: false,
            hover: 0.0,
            burst: 0,
            windup: 0.95,
            accuracy: 0.35,
        },
        Kind::Drone => KindStats {
            health: 34.0,
            speed: 4.2,
            radius: 0.38,
            // Matches the model: `build_drone` lifts its geometry so it occupies
            // [0, 0.97r] on the origin, and the box used to be 0.70 tall, which
            // left a third of a metre of empty air above the drone that counted
            // as head. Keep this in step with the mesh offset.
            height: 0.37,
            damage: 6.0,
            interval: 0.50,
            range: 16.0,
            standoff: 7.0,
            reach: 0.0,
            sanity_drain: 0.0,
            spawns_on_death: 0,
            score: 180,
            body: Color::new(0.30, 0.32, 0.38, 1.0),
            trim: Color::new(0.18, 0.19, 0.24, 1.0),
            visor: Color::new(0.30, 0.85, 1.0, 1.0),
            flying: true,
            hover: 2.90,
            burst: 0,
            windup: 0.0,
            accuracy: 1.3,
        },
        Kind::Tentacle => KindStats {
            health: 95.0,
            speed: 3.6,
            radius: 0.55,
            height: 2.00,
            damage: 16.0,
            interval: 1.10,
            range: 4.0,
            // Holds at the edge of its reach and lashes rather than pressing in.
            standoff: 3.0,
            reach: 3.20,
            sanity_drain: 3.0,
            spawns_on_death: 3,
            score: 400,
            body: Color::new(0.20, 0.34, 0.22, 1.0),
            trim: Color::new(0.30, 0.16, 0.34, 1.0),
            visor: Color::new(0.55, 1.0, 0.32, 1.0),
            flying: false,
            hover: 0.0,
            burst: 0,
            windup: 0.0,
            accuracy: 1.0,
        },
        Kind::Colossus => KindStats {
            health: 900.0,
            speed: 1.15,
            // 1.70 m of radius is 3.4 m of body, and R'LYEH's tightest channels
            // are 2.4 m: it wedged itself into the geometry and stayed there,
            // which also meant the mission could never be completed because a
            // stuck enemy still counts as alive.
            radius: 1.05,
            height: 5.50,
            damage: 42.0,
            interval: 2.60,
            range: 4.5,
            standoff: 0.0,
            reach: 3.60,
            sanity_drain: 14.0,
            spawns_on_death: 0,
            score: 2500,
            body: Color::new(0.60, 0.58, 0.50, 1.0),
            trim: Color::new(0.24, 0.16, 0.30, 1.0),
            visor: Color::new(0.78, 0.62, 1.0, 1.0),
            flying: false,
            hover: 0.0,
            burst: 0,
            windup: 0.0,
            accuracy: 1.0,
        },
        Kind::Spawnling => KindStats {
            health: 18.0,
            speed: 6.4,
            radius: 0.26,
            height: 0.72,
            damage: 6.0,
            interval: 0.45,
            range: 2.0,
            standoff: 0.0,
            reach: 0.90,
            sanity_drain: 0.0,
            spawns_on_death: 0,
            score: 25,
            body: Color::new(0.40, 0.14, 0.26, 1.0),
            trim: Color::new(0.22, 0.08, 0.16, 1.0),
            visor: Color::new(0.90, 1.0, 0.35, 1.0),
            flying: false,
            hover: 0.0,
            burst: 0,
            windup: 0.0,
            accuracy: 1.0,
        },
    }
}

/// Low-poly humanoid built once in local space, feet at the origin, facing -Z.
/// Translate a finished prototype so its lowest vertex sits on `y = 0`, which
/// is the convention every collision box assumes.
fn drop_to_floor(mesh: &mut Mesh) {
    let mut lowest = f32::MAX;
    for v in &mesh.vertices {
        lowest = lowest.min(v.position.y);
    }
    if lowest.is_finite() && lowest.abs() > 1e-4 {
        for v in &mut mesh.vertices {
            v.position.y -= lowest;
        }
    }
}

fn build_humanoid(s: &KindStats) -> Mesh {
    let mut mb = MeshBuilder::new();
    let (r, h) = (s.radius, s.height);

    mb.add_box(
        vec3(-r * 0.58, 0.0, -r * 0.34),
        vec3(-r * 0.12, h * 0.44, r * 0.34),
        s.trim,
    );
    mb.add_box(
        vec3(r * 0.12, 0.0, -r * 0.34),
        vec3(r * 0.58, h * 0.44, r * 0.34),
        s.trim,
    );
    mb.add_box(
        vec3(-r * 0.72, h * 0.40, -r * 0.44),
        vec3(r * 0.72, h * 0.80, r * 0.44),
        s.body,
    );
    mb.add_box(
        vec3(-r * 1.02, h * 0.68, -r * 0.40),
        vec3(r * 1.02, h * 0.83, r * 0.40),
        s.trim,
    );
    mb.add_box(
        vec3(-r * 1.00, h * 0.42, -r * 0.20),
        vec3(-r * 0.70, h * 0.72, r * 0.24),
        s.body,
    );
    mb.add_box(
        vec3(r * 0.70, h * 0.42, -r * 0.20),
        vec3(r * 1.00, h * 0.72, r * 0.24),
        s.body,
    );
    mb.add_box(
        vec3(-r * 0.36, h * 0.82, -r * 0.34),
        vec3(r * 0.36, h * 1.0, r * 0.34),
        s.trim,
    );
    mb.add_box(
        vec3(-r * 0.30, h * 0.88, -r * 0.42),
        vec3(r * 0.30, h * 0.95, -r * 0.30),
        s.visor,
    );
    mb.add_box(
        vec3(-r * 0.16, h * 0.54, -r * 0.50),
        vec3(r * 0.16, h * 0.68, -r * 0.40),
        s.visor,
    );
    mb.finish()
}

/// A quad-rotor body: central chassis, four arms, a glowing sensor eye.
fn build_drone(s: &KindStats) -> Mesh {
    let mut mb = MeshBuilder::new();
    let r = s.radius;

    mb.add_box(
        vec3(-r * 0.55, -r * 0.30, -r * 0.60),
        vec3(r * 0.55, r * 0.35, r * 0.60),
        s.body,
    );
    // sensor eye, angled forward
    mb.add_box(
        vec3(-r * 0.30, -r * 0.16, -r * 0.78),
        vec3(r * 0.30, r * 0.16, -r * 0.58),
        s.visor,
    );
    // arms + rotor pads
    for (sx, sz) in [(1.0f32, 1.0f32), (1.0, -1.0), (-1.0, 1.0), (-1.0, -1.0)] {
        mb.add_box(
            vec3(
                (sx * 0.35).min(sx * 1.15),
                -r * 0.08,
                (sz * 0.35).min(sz * 1.15),
            ),
            vec3(
                (sx * 0.35).max(sx * 1.15),
                r * 0.08,
                (sz * 0.35).max(sz * 1.15),
            ),
            s.trim,
        );
        let (cx, cz) = (sx * 1.15, sz * 1.15);
        mb.add_box(
            vec3(cx - r * 0.42, r * 0.02, cz - r * 0.42),
            vec3(cx + r * 0.42, r * 0.10, cz + r * 0.42),
            s.visor,
        );
    }
    // under-slung gun
    mb.add_box(
        vec3(-r * 0.12, -r * 0.62, -r * 0.30),
        vec3(r * 0.12, -r * 0.28, r * 0.30),
        s.trim,
    );
    // A quad-rotor reads better modelled around its centre of mass, which is
    // how the boxes above are laid out — but `body_aabb`, `head_y` and `eye`
    // all measure upward from `pos.y`, the way every other archetype's model
    // stands on it. Left as it was, two thirds of the drone hung below its own
    // hitbox (so shooting the thing missed), the hitbox was mostly empty air,
    // and every shot that did land did so above `head_y`. Lifting the finished
    // mesh by the depth of its under-slung gun, `0.62r`, puts the model's
    // lowest vertex on `pos.y` and the whole model inside `[0, height]`.
    transform_mesh(
        &mb.finish(),
        Mat4::from_translation(vec3(0.0, r * 0.62, 0.0)),
    )
}

/// Axis-aligned box from a centre and half-extents. The cosmic archetypes are
/// laid out along rays, where a centre reads more clearly than a min/max pair.
fn box_at(mb: &mut MeshBuilder, centre: Vec3, half: Vec3, color: Color) {
    mb.add_box(centre - half, centre + half, color);
}

/// Squat bulb sitting on a ring of tapering tentacles, eyes clustered on the
/// front.
fn build_tentacle(s: &KindStats) -> Mesh {
    let mut mb = MeshBuilder::new();
    let (r, h) = (s.radius, s.height);

    // Eight rays, each four ever-smaller boxes drooping toward the ground.
    for i in 0..8 {
        let a = i as f32 / 8.0 * TAU;
        let (dx, dz) = (a.sin(), a.cos());
        for j in 0..4 {
            let step = j as f32;
            let out = r * (1.06 + 0.52 * step);
            let half = r * (0.26 - 0.055 * step);
            box_at(
                &mut mb,
                vec3(dx * out, h * (0.34 - 0.068 * step), dz * out),
                vec3(half, half * 0.9, half),
                if j % 2 == 0 { s.trim } else { s.body },
            );
        }
    }

    // Bulb: widest above the tentacle roots, tapering to a wet crown.
    box_at(
        &mut mb,
        vec3(0.0, h * 0.34, 0.0),
        vec3(r * 0.80, h * 0.20, r * 0.80),
        s.body,
    );
    box_at(
        &mut mb,
        vec3(0.0, h * 0.56, 0.0),
        vec3(r * 0.95, h * 0.16, r * 0.92),
        s.body,
    );
    box_at(
        &mut mb,
        vec3(0.0, h * 0.74, 0.0),
        vec3(r * 0.72, h * 0.12, r * 0.70),
        s.trim,
    );
    box_at(
        &mut mb,
        vec3(0.0, h * 0.88, 0.0),
        vec3(r * 0.46, h * 0.10, r * 0.44),
        s.body,
    );

    // The eyes, wet and luminous, pushed out through the front of the bulb.
    for (ex, ey, ez, eh) in [
        (0.0f32, 0.58f32, 0.86f32, 0.20f32),
        (-0.40, 0.50, 0.80, 0.14),
        (0.40, 0.50, 0.80, 0.14),
        (-0.30, 0.76, 0.66, 0.13),
        (0.30, 0.76, 0.66, 0.13),
        (0.0, 0.89, 0.42, 0.12),
    ] {
        box_at(
            &mut mb,
            vec3(ex * r, h * ey, -ez * r),
            vec3(r * eh, r * eh, r * 0.20),
            s.visor,
        );
    }
    mb.finish()
}

/// Towering walker, rebuilt every frame so `phase` can drive a slow, heavy
/// stride: the legs swing half a cycle apart, the frame lurches, and the crown
/// of tentacles writhes.
fn build_colossus(s: &KindStats, phase: f32) -> Mesh {
    let mut mb = MeshBuilder::new();
    let (r, h) = (s.radius, s.height);
    let stride = phase * 1.7;

    for (side, lag) in [(1.0f32, 0.0f32), (-1.0f32, PI)] {
        let cyc = (stride + lag).sin();
        let lift = cyc.max(0.0) * h * 0.05;
        let fwd = -cyc * r * 0.20;
        box_at(
            &mut mb,
            vec3(side * r * 0.50, h * 0.21 + lift, fwd),
            vec3(r * 0.30, h * 0.21, r * 0.34),
            s.trim,
        );
        box_at(
            &mut mb,
            vec3(side * r * 0.52, h * 0.05 + lift, fwd - r * 0.10),
            vec3(r * 0.36, h * 0.05, r * 0.48),
            s.trim,
        );
    }

    let bob = (stride * 2.0).sin().abs() * h * 0.012;
    let sway = stride.sin() * r * 0.05;

    box_at(
        &mut mb,
        vec3(sway, h * 0.60 + bob, 0.0),
        vec3(r * 0.92, h * 0.21, r * 0.62),
        s.body,
    );
    box_at(
        &mut mb,
        vec3(sway, h * 0.48 + bob, -r * 0.30),
        vec3(r * 0.78, h * 0.10, r * 0.36),
        s.trim,
    );
    box_at(
        &mut mb,
        vec3(sway, h * 0.75 + bob, 0.0),
        vec3(r * 1.24, h * 0.09, r * 0.55),
        s.trim,
    );

    // Arms counter-swing against the legs.
    for (side, lag) in [(1.0f32, PI), (-1.0f32, 0.0)] {
        let cyc = (stride + lag).sin();
        box_at(
            &mut mb,
            vec3(side * r * 1.14 + sway, h * 0.50 + bob, -cyc * r * 0.16),
            vec3(r * 0.28, h * 0.26, r * 0.30),
            s.body,
        );
    }

    // A head, then a crown of tentacles where a face should be.
    let head_y = h * 0.86 + bob;
    box_at(
        &mut mb,
        vec3(sway, head_y, 0.0),
        vec3(r * 0.44, h * 0.06, r * 0.42),
        s.body,
    );
    for i in 0..7 {
        let a = i as f32 / 7.0 * TAU + phase * 0.35;
        let (dx, dz) = (a.sin(), a.cos());
        let curl = (phase * 1.9 + i as f32 * 1.3).sin() * h * 0.015;
        for j in 0..3 {
            let step = j as f32;
            let spread = r * (0.30 + 0.22 * step);
            let half = r * (0.13 - 0.032 * step);
            box_at(
                &mut mb,
                vec3(
                    sway + dx * spread,
                    head_y + h * (0.045 + 0.042 * step) + curl,
                    dz * spread,
                ),
                vec3(half, h * 0.045, half),
                if j == 1 { s.trim } else { s.body },
            );
        }
    }

    for (ex, ey, ez, eh) in [
        (0.0f32, 0.87f32, 0.42f32, 0.10f32),
        (-0.30, 0.84, 0.38, 0.07),
        (0.30, 0.84, 0.38, 0.07),
        (-0.16, 0.92, 0.30, 0.055),
        (0.16, 0.92, 0.30, 0.055),
    ] {
        box_at(
            &mut mb,
            vec3(ex * r + sway, h * ey + bob, -ez * r),
            vec3(r * eh, r * eh, r * 0.12),
            s.visor,
        );
    }
    mb.finish()
}

/// Low and wide, many-legged, with one oversized glowing eye.
fn build_spawnling(s: &KindStats) -> Mesh {
    let mut mb = MeshBuilder::new();
    let (r, h) = (s.radius, s.height);

    // Flat carapace hunched over three pairs of splayed legs.
    box_at(
        &mut mb,
        vec3(0.0, h * 0.46, 0.0),
        vec3(r * 0.90, h * 0.24, r * 1.10),
        s.body,
    );
    box_at(
        &mut mb,
        vec3(0.0, h * 0.72, -r * 0.10),
        vec3(r * 0.62, h * 0.16, r * 0.70),
        s.trim,
    );
    for side in [-1.0f32, 1.0] {
        for i in 0..3 {
            let z = (i as f32 - 1.0) * r * 0.62;
            box_at(
                &mut mb,
                vec3(side * r * 1.15, h * 0.24, z),
                vec3(r * 0.55, h * 0.10, r * 0.09),
                s.trim,
            );
            box_at(
                &mut mb,
                vec3(side * r * 1.70, h * 0.16, z),
                vec3(r * 0.12, h * 0.14, r * 0.09),
                s.trim,
            );
        }
    }
    for i in 0..4 {
        box_at(
            &mut mb,
            vec3(0.0, h * 0.86, (i as f32 - 1.5) * r * 0.44),
            vec3(r * 0.12, h * 0.12, r * 0.12),
            s.trim,
        );
    }

    box_at(
        &mut mb,
        vec3(0.0, h * 0.52, -r * 1.00),
        vec3(r * 0.48, h * 0.20, r * 0.18),
        s.visor,
    );
    mb.finish()
}

pub enum Attack {
    Shot {
        origin: Vec3,
        dir: Vec3,
        damage: f32,
    },
    Melee {
        damage: f32,
    },
}

pub struct Enemy {
    pub pos: Vec3,
    pub vel: Vec3,
    pub yaw: f32,
    pub kind: Kind,
    pub health: f32,
    pub radius: f32,
    pub height: f32,
    pub speed: f32,
    pub damage: f32,
    pub interval: f32,
    pub range: f32,
    pub standoff: f32,
    /// Melee reach, mirrored from the archetype. `0.0` means this one shoots.
    pub reach: f32,
    /// Sanity per second this archetype costs the player while visible.
    pub sanity_drain: f32,
    /// Spawnlings released when this enemy dies.
    pub spawns_on_death: u32,
    pub score: u32,
    pub cooldown: f32,
    pub flash: f32,
    /// Ramps to 1 while the player is visible and in range; drives the visor glow
    /// and, for snipers, the shot telegraph.
    pub alert: f32,
    pub aim_t: f32,
    /// Where the player was last seen, and how long ago that was.
    pub last_seen: Vec3,
    /// True once this unit has actually laid eyes on the player. One that was
    /// only *told* where the player was — a reinforcement, via `alert_to` — has
    /// a memory of a place it never saw anyone at, and follows the flow field to
    /// the player instead of searching that spot.
    pub seen_player: bool,
    pub aware_t: f32,
    pub dead: bool,
    pub death_t: f32,
    pub spawned_t: f32,
    pub flying: bool,
    hover: f32,
    strafe_dir: f32,
    strafe_t: f32,
    burst_left: u32,
    proto: Mesh,
}

impl Enemy {
    pub fn spawn(kind: Kind, pos: Vec3, wave: u32, rng: &mut Rng) -> Self {
        let s = stats(kind);
        let w = (wave.saturating_sub(1)) as f32;
        let health = s.health * (1.0 + 0.13 * w);
        let proto = {
            let mut mesh = if kind == Kind::Tentacle {
                build_tentacle(&s)
            } else if kind == Kind::Colossus {
                build_colossus(&s, 0.0)
            } else if kind == Kind::Spawnling {
                build_spawnling(&s)
            } else if s.flying {
                build_drone(&s)
            } else {
                build_humanoid(&s)
            };
            // Structural, not per-archetype: a model that floats above or
            // straddles its own origin does not match its collision box, and the
            // mismatch is invisible until you shoot one. Normalising here means
            // adding an archetype cannot reintroduce it.
            drop_to_floor(&mut mesh);
            mesh
        };
        Self {
            pos,
            vel: Vec3::ZERO,
            yaw: rng.range(0.0, TAU),
            kind,
            health,
            radius: s.radius,
            height: s.height,
            speed: s.speed * (1.0 + 0.022 * w),
            damage: s.damage * (1.0 + 0.06 * w),
            interval: (s.interval * (1.0 - 0.015 * w)).max(0.30),
            range: s.range,
            standoff: s.standoff,
            reach: s.reach,
            sanity_drain: s.sanity_drain,
            spawns_on_death: s.spawns_on_death,
            score: s.score,
            cooldown: rng.range(0.3, 1.0),
            flash: 0.0,
            alert: 0.0,
            aim_t: 0.0,
            last_seen: pos,
            seen_player: false,
            aware_t: 99.0,
            dead: false,
            death_t: 0.0,
            spawned_t: 0.0,
            flying: s.flying,
            hover: s.hover,
            strafe_dir: if rng.chance(0.5) { 1.0 } else { -1.0 },
            strafe_t: rng.range(0.5, 1.8),
            burst_left: 0,
            proto,
        }
    }

    /// Tells this unit where the player was and starts it hunting.
    ///
    /// A freshly spawned enemy has never seen anything (`aware_t` starts at
    /// 99), and `pick_spawn` deliberately places reinforcements out of sight —
    /// so without this they stand at their spawn until the player happens to
    /// walk into view. On an open arena that resolves itself; inside R'LYEH's
    /// labyrinth it never does, and seven hostiles sat 20 m away for a full
    /// minute doing nothing. `pursue` is how many seconds they keep coming
    /// before giving up and returning to an idle state, so breaking contact is
    /// still a real defence.
    ///
    /// A unit that has not laid eyes on the player yet has no memory to search,
    /// only a place to go, so it keeps walking the flow field toward wherever
    /// the player is now — see `search_dir` for what happens once it has.
    pub fn alert_to(&mut self, pos: Vec3, pursue: f32) {
        self.last_seen = pos;
        self.aware_t = -pursue;
    }

    pub fn targetable(&self) -> bool {
        !self.dead
    }

    pub fn body_aabb(&self) -> Aabb {
        Aabb::new(
            vec3(
                self.pos.x - self.radius,
                self.pos.y,
                self.pos.z - self.radius,
            ),
            vec3(
                self.pos.x + self.radius,
                self.pos.y + self.height,
                self.pos.z + self.radius,
            ),
        )
    }

    /// Height above the enemy's feet that separates a body shot from a head shot.
    pub fn head_y(&self) -> f32 {
        if self.flying {
            // Upper part of a body that is only 0.37 tall and sits on its own
            // origin, so the usual 0.78 of a humanoid's height would be above
            // the top of the model.
            self.pos.y + self.height * 0.72
        } else {
            self.pos.y + self.height * 0.78
        }
    }

    pub fn eye(&self) -> Vec3 {
        if self.flying {
            vec3(self.pos.x, self.pos.y + self.height * 0.5, self.pos.z)
        } else {
            vec3(self.pos.x, self.pos.y + self.height * 0.86, self.pos.z)
        }
    }

    pub fn is_removable(&self) -> bool {
        self.dead && self.death_t > 1.9
    }

    /// Steering for a unit that has lost sight of the player: back to the last
    /// position it saw them at, and nothing at all once it arrives.
    ///
    /// The flow field cannot be trusted on its own here. It is rebuilt four
    /// times a second toward the player's *current* position, so a unit that
    /// has just lost contact follows it straight to wherever the player has
    /// moved to since — through walls, and never stopping to look. The intent
    /// was always "the squad searches your last known position"; the field is
    /// kept as the navigator (it is the only one the level has), but it is used
    /// only when the straight line to the memory is blocked *and* the field
    /// roughly agrees with it. Whenever it points somewhere else, the memory
    /// wins and the unit walks it directly.
    fn search_dir(&self, level: &Level) -> Vec2 {
        let to_memory = vec2(self.last_seen.x - self.pos.x, self.last_seen.z - self.pos.z);
        let d = to_memory.length();
        if d <= SEARCH_REACH {
            return Vec2::ZERO;
        }
        let direct = to_memory / d;
        let blocked = !level.line_of_sight(self.eye(), self.last_seen + vec3(0.0, 1.0, 0.0));
        let flow = level.flow.dir_at(self.pos);
        if blocked && flow.dot(direct) >= 0.25 {
            flow
        } else {
            direct
        }
    }

    pub fn take_damage(&mut self, amount: f32) -> bool {
        if self.dead {
            return false;
        }
        self.health -= amount;
        self.flash = 1.0;
        if self.health <= 0.0 {
            self.health = 0.0;
            self.dead = true;
            self.death_t = 0.0;
            return true;
        }
        false
    }

    /// `neighbours` is every live enemy's position from the start of the frame,
    /// and `self_index` is this one's place in it — passing the slice and
    /// skipping our own entry avoids rebuilding a filtered copy per enemy per
    /// frame.
    pub fn update(
        &mut self,
        level: &Level,
        player: &Player,
        neighbours: &[Vec3],
        self_index: usize,
        dt: f32,
        rng: &mut Rng,
    ) -> Option<Attack> {
        if self.dead {
            self.death_t += dt;
            return None;
        }

        self.spawned_t += dt;
        self.flash = approach(self.flash, 0.0, dt * 6.0);
        self.cooldown = (self.cooldown - dt).max(0.0);
        let s = stats(self.kind);

        let to_player = player.pos - self.pos;
        let flat = vec2(to_player.x, to_player.z);
        let dist = flat.length();

        // Detection range depends on how much the player is exposing
        // themselves: standing and sprinting are spotted far sooner than
        // crouched, prone and walking. Long-ranged archetypes notice further.
        let detect = (self.range * 1.2).max(24.0) * player.visibility();
        let los = player.alive && dist < detect && level.line_of_sight(self.eye(), player.eye());
        if los {
            self.last_seen = player.pos;
            self.seen_player = true;
            self.aware_t = 0.0;
        } else {
            self.aware_t += dt;
        }

        self.alert = approach(
            self.alert,
            if los && dist < self.range { 1.0 } else { 0.0 },
            dt * 4.0,
        );

        // --- desired velocity -------------------------------------------------
        // With neither sight nor recent contact the squad holds position. That
        // is what makes crouching, going prone and walking worth doing.
        let aware = player.alive && (los || self.aware_t < 6.0);
        // A unit that has actually had eyes on the player searches the memory of
        // where it last saw them. One that was merely *told* where the player was
        // — a reinforcement, alerted at spawn — has no memory to search, only a
        // place to go, so it keeps walking the flow field toward the player as
        // before: that is what makes it arrive at the fight at all.
        let searching = !los && self.seen_player;
        let mut desired = Vec2::ZERO;
        if aware && self.spawned_t > 0.35 {
            if s.reach > 0.0 {
                // Melee: close the distance. An archetype with a standoff
                // lashes from the edge of its reach instead of pressing in.
                desired = if searching {
                    self.search_dir(level)
                } else if !los {
                    level.flow.dir_at(self.pos)
                } else if self.standoff > 0.0 && dist < self.standoff {
                    -flat.normalize_or_zero() * 0.8
                } else {
                    flat.normalize_or_zero()
                } * self.speed;
            } else if searching {
                // Lost contact: walk back to the last known position and hold
                // there — slower than a charge, and without shooting.
                desired = self.search_dir(level) * self.speed * 0.8;
            } else if !los {
                desired = level.flow.dir_at(self.pos) * self.speed * 0.8;
            } else if dist > self.range * 0.9 || dist > self.standoff * 1.6 {
                desired = level.flow.dir_at(self.pos) * self.speed;
            } else if dist < self.standoff * 0.55 {
                desired = -flat.normalize_or_zero() * self.speed * 0.8;
            } else {
                self.strafe_t -= dt;
                if self.strafe_t <= 0.0 {
                    self.strafe_t = rng.range(0.7, 2.1);
                    if rng.chance(0.45) {
                        self.strafe_dir = -self.strafe_dir;
                    }
                }
                let perp = vec2(-flat.y, flat.x).normalize_or_zero();
                desired = perp * self.strafe_dir * self.speed * 0.7;
            }
        }

        for (j, other) in neighbours.iter().enumerate() {
            if j == self_index {
                continue;
            }
            let d = vec2(self.pos.x - other.x, self.pos.z - other.z);
            let l = d.length();
            let min_d = (self.radius + 0.5) * 2.0;
            if l > 1e-3 && l < min_d {
                desired += d.normalize() * (1.0 - l / min_d) * self.speed * 1.8;
            }
        }
        if desired.length() > self.speed {
            desired = desired.normalize() * self.speed;
        }

        let current = vec2(self.vel.x, self.vel.z);
        let d = desired - current;
        let max_d = 26.0 * dt;
        let new_v = if d.length() > max_d {
            current + d.normalize() * max_d
        } else {
            desired
        };
        self.vel.x = new_v.x;
        self.vel.z = new_v.y;

        if self.flying {
            // Hovering units ignore gravity and float to their cruise altitude.
            self.vel.y = 0.0;
            let cruise = self.hover + (self.spawned_t * 1.7).sin() * 0.18;
            self.pos.y = approach(self.pos.y, cruise, dt * 2.5);
            let (pos, _) = move_and_collide(
                level,
                self.pos,
                self.radius,
                self.height,
                vec3(self.vel.x * dt, 0.0, self.vel.z * dt),
                0.0,
            );
            self.pos.x = pos.x;
            self.pos.z = pos.z;
        } else {
            self.vel.y = (self.vel.y - GRAVITY * dt).max(-60.0);
            let (pos, col) = move_and_collide(
                level,
                self.pos,
                self.radius,
                self.height,
                self.vel * dt,
                STEP_HEIGHT,
            );
            self.pos = pos;
            if col.x {
                self.vel.x = 0.0;
            }
            if col.z {
                self.vel.z = 0.0;
            }
            if col.ground && self.vel.y < 0.0 {
                self.vel.y = 0.0;
            }
        }

        // --- facing -----------------------------------------------------------
        let face = if los && player.alive {
            flat.normalize_or_zero()
        } else {
            vec2(self.vel.x, self.vel.z).normalize_or_zero()
        };
        if face != Vec2::ZERO {
            let target = yaw_of(vec3(face.x, 0.0, face.y));
            self.yaw = angle_toward(self.yaw, target, dt * 7.0);
        }

        // --- attack -----------------------------------------------------------
        if !player.alive || self.spawned_t < 0.5 {
            return None;
        }
        if s.reach > 0.0 {
            // Reach has to be earned. Without the sight test a 3.2 m tentacle
            // claws through a 1 m wall; without the height test a Colossus
            // reaches up through the terrace you are standing on. The old code
            // compared XZ distance only, so cover did nothing at all.
            let same_level = (player.pos.y - self.pos.y).abs() < self.height + 0.5;
            if los && same_level && dist < self.reach && self.cooldown <= 0.0 {
                self.cooldown = self.interval;
                return Some(Attack::Melee {
                    damage: self.damage,
                });
            }
            return None;
        }

        let in_range = los && dist < self.range;
        if !in_range {
            self.aim_t = 0.0;
            return None;
        }

        if s.windup > 0.0 {
            // Telegraph: hold the shot until the aim timer has charged.
            if self.cooldown > 0.0 {
                // Still recovering from the previous shot. Without this return
                // the branch falls through to the firing code below and a
                // marksman fires every single frame, which is exactly what a
                // harness measured: 514 shots in 600 frames.
                self.aim_t = 0.0;
                return None;
            }
            self.aim_t += dt;
            if self.aim_t < s.windup {
                return None;
            }
        } else if self.cooldown > 0.0 {
            return None;
        }

        self.aim_t = 0.0;
        let origin = self.eye();
        let aim = player.eye() + vec3(0.0, -0.22, 0.0);
        let spread = (0.018 + dist * 0.0017).min(0.070) * s.accuracy;
        let dir = rng.cone((aim - origin).normalize_or_zero(), spread);

        if s.burst > 1 {
            if self.burst_left == 0 {
                self.burst_left = s.burst;
            }
            self.burst_left -= 1;
            self.cooldown = if self.burst_left == 0 {
                self.interval
            } else {
                0.15
            };
        } else {
            self.cooldown = self.interval;
        }

        Some(Attack::Shot {
            origin,
            dir,
            damage: self.damage,
        })
    }

    pub fn mesh(&self) -> Mesh {
        let fall = if self.dead {
            (self.death_t / 0.55).clamp(0.0, 1.0).powf(0.65)
        } else {
            0.0
        };
        let sink = if self.death_t > 1.15 {
            ((self.death_t - 1.15) / 0.75).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let alpha = (1.0 - sink).max(0.0);
        // Flying units spin down and drop rather than toppling over.
        let mat = if self.flying {
            let spin = self.yaw + fall * TAU * 1.5;
            Mat4::from_translation(self.pos + vec3(0.0, -fall * 1.6, 0.0))
                * Mat4::from_rotation_y(spin)
        } else {
            Mat4::from_translation(self.pos + vec3(0.0, -sink * 0.5, 0.0))
                * Mat4::from_rotation_y(self.yaw)
                * Mat4::from_rotation_x(fall * FRAC_PI_2)
        };
        // The colossus is the only archetype whose limbs move, so its mesh is
        // rebuilt from `spawned_t` every frame; everything else reuses the
        // prototype built at spawn time.
        let walker = if self.kind == Kind::Colossus {
            Some(build_colossus(&stats(self.kind), self.spawned_t))
        } else {
            None
        };
        let proto = walker.as_ref().unwrap_or(&self.proto);
        let base = transform_mesh(proto, mat);
        let glow = (self.flash * 0.85).max(self.alert * 0.22);
        // `tint_mesh` clones the whole mesh again. With no flash, no alert and
        // no death fade — the common case for every enemy but the one you just
        // shot — it would return exactly this, so skip it.
        if glow <= 0.001 && alpha >= 0.999 {
            return base;
        }
        tint_mesh(&base, Color::new(1.0, 0.92, 0.80, 1.0), glow, alpha)
    }
}

fn angle_toward(current: f32, target: f32, t: f32) -> f32 {
    let mut d = (target - current) % TAU;
    if d > PI {
        d -= TAU;
    } else if d < -PI {
        d += TAU;
    }
    current + d * t.clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every archetype's collision box has to agree with what you can see.
    ///
    /// The drone's did not: its model straddled its own origin while the box sat
    /// on top of it, so two thirds of the visible drone was unhittable and the
    /// empty air above it read as head. Nothing else in the suite notices a
    /// disagreement between geometry and hitbox, and it is invisible until you
    /// shoot one.
    #[test]
    fn every_hitbox_matches_its_model() {
        let kinds = [
            Kind::Grunt,
            Kind::Rusher,
            Kind::Heavy,
            Kind::Sniper,
            Kind::Drone,
            Kind::Tentacle,
            Kind::Colossus,
            Kind::Spawnling,
        ];
        let mut rng = Rng::new(4);
        for kind in kinds {
            let e = Enemy::spawn(kind, vec3(0.0, 0.0, 0.0), 1, &mut rng);
            let mesh = e.mesh();
            let (mut lo, mut hi) = (f32::MAX, f32::MIN);
            for v in &mesh.vertices {
                assert!(v.position.y.is_finite(), "{kind:?}: non-finite vertex");
                lo = lo.min(v.position.y);
                hi = hi.max(v.position.y);
            }
            assert!(hi > lo, "{kind:?}: degenerate model");

            let b = e.body_aabb();
            let model_h = hi - lo;
            let box_h = b.max.y - b.min.y;
            let ratio = box_h / model_h;
            assert!(
                (0.8..=1.25).contains(&ratio),
                "{kind:?}: the box is {box_h:.3} tall for a {model_h:.3} model (ratio {ratio:.2})",
            );
            assert!(
                (b.min.y - lo).abs() < 0.1,
                "{kind:?}: box base {:.3} does not sit on the model base {lo:.3}",
                b.min.y,
            );

            // And the head must be in the upper half of the model, or every
            // body shot becomes a headshot.
            let head = e.head_y();
            assert!(
                head > lo + model_h * 0.5 && head <= hi + 0.05,
                "{kind:?}: head_y {head:.3} is outside the model's upper half ({lo:.3}..{hi:.3})",
            );
        }
    }
}
