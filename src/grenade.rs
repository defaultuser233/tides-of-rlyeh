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

/// Bodies in flight at once; a throw beyond this drops the oldest.
const MAX_LIVE: usize = 8;
/// Body half-extent. Collision and the drawn casing share it, so the grenade
/// that bounces is the grenade you see.
const HALF: f32 = 0.085;
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
const BODY_TINT: Color = Color::new(0.09, 0.10, 0.12, 1.0);

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum GrenadeKind {
    Frag,
    Smoke,
}

impl GrenadeKind {
    pub fn name(self) -> i18n::Key {
        match self {
            GrenadeKind::Frag => Key::GrenadeFrag,
            GrenadeKind::Smoke => Key::GrenadeSmoke,
        }
    }

    pub fn color(self) -> Color {
        match self {
            GrenadeKind::Frag => Color::new(0.47, 0.51, 0.26, 1.0),
            GrenadeKind::Smoke => Color::new(0.58, 0.66, 0.76, 1.0),
        }
    }

    fn fuse(self) -> f32 {
        match self {
            GrenadeKind::Frag => FRAG_FUSE,
            GrenadeKind::Smoke => SMOKE_FUSE,
        }
    }
}

/// A resolved detonation. Everything within `radius` takes `damage`; how that
/// falls off with distance is the caller's call.
pub struct Blast {
    pub pos: Vec3,
    pub radius: f32,
    pub damage: f32,
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
    pub fn throw(&mut self, kind: GrenadeKind, origin: Vec3, dir: Vec3) {
        if self.live.len() >= MAX_LIVE {
            self.live.remove(0);
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
            fuse: kind.fuse(),
            resting: false,
            dead: false,
        });

        self.last_kind = kind;
        self.throw_t = THROW_TIME;
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
            if !g.resting {
                let speed = g.vel.length();
                let steps =
                    ((speed * dt / MAX_STEP).ceil() as u32).clamp(MIN_SUBSTEPS, MAX_SUBSTEPS);
                let h = dt / steps as f32;

                let mut touched = false;
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
                    blasts.push(Blast {
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
                    level.add_smoke(g.pos, SMOKE_RADIUS, SMOKE_LIFE);
                    fx.smoke(g.pos, SMOKE_TINT, 18, rng);
                }
                _ => {}
            }
        }

        self.live.retain(|g| !g.dead);
        blasts
    }

    /// Live bodies plus the smoke clouds they left, batched for one draw call
    /// into `dst`.
    pub fn write_mesh(&self, level: &Level, dst: &mut Mesh) {
        let mut mb = MeshBuilder::reclaim(dst);
        let mut budget = MAX_BOXES;

        for g in &self.live {
            if budget < 2 {
                break;
            }
            budget -= 2;
            let rot = Quat::from_axis_angle(g.spin_axis, g.spin_angle);
            oriented_box(&mut mb, g.pos, Vec3::splat(HALF), rot, BODY_TINT);
            let band = vec3(HALF * 1.12, HALF * 0.34, HALF * 1.12);
            oriented_box(&mut mb, g.pos, band, rot, g.kind.color());
        }

        let seeds = shell_seeds();
        for s in &level.smoke {
            if s.life <= 0.0 {
                continue;
            }
            let r = s.current_radius();
            if r < 0.05 {
                continue;
            }
            let fade = (s.life / s.max_life).clamp(0.0, 1.0).powf(0.45);
            for (shell, &(rf, count, sf, alpha)) in SMOKE_SHELLS.iter().enumerate() {
                if budget < count {
                    break;
                }
                budget -= count;
                let cell = r * sf;
                for seed in &seeds[shell].boxes {
                    let centre = s.pos + seed.dir * (r * rf);
                    let h = cell * seed.height;
                    let c = Color::new(SMOKE_TINT.r, SMOKE_TINT.g, SMOKE_TINT.b, alpha * fade);
                    mb.add_box(centre - Vec3::splat(h), centre + Vec3::splat(h), c);
                }
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

/// A box with its corners rotated by `rot`, so the tumble is visible.
fn oriented_box(mb: &mut MeshBuilder, centre: Vec3, half: Vec3, rot: Quat, color: Color) {
    let mut p = [Vec3::ZERO; 8];
    for (i, slot) in p.iter_mut().enumerate() {
        let local = vec3(
            if i & 1 == 0 { -half.x } else { half.x },
            if i & 2 == 0 { -half.y } else { half.y },
            if i & 4 == 0 { -half.z } else { half.z },
        );
        *slot = centre + rot * local;
    }
    mb.quad(p[2], p[6], p[7], p[3], color);
    mb.quad(p[4], p[0], p[1], p[5], color);
    mb.quad(p[4], p[5], p[7], p[6], color);
    mb.quad(p[1], p[0], p[2], p[3], color);
    mb.quad(p[5], p[1], p[3], p[7], color);
    mb.quad(p[0], p[4], p[6], p[2], color);
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
