use crate::level::Level;
use crate::mesh::MeshBuilder;
use crate::util::{Rng, approach};
use macroquad::prelude::*;

const MAX_PARTICLES: usize = 900;
/// Debris settles this far above the surface it lands on. The arena floor sits
/// at y = 0, so on flat ground this reproduces the bounce plane the bounce used
/// to be hard-coded to; it is also the fallback when the level reports no
/// surface under a particle at all.
const GROUND_CLEARANCE: f32 = 0.03;

pub struct Particle {
    pub pos: Vec3,
    pub vel: Vec3,
    pub life: f32,
    pub max_life: f32,
    pub size: f32,
    pub color: Color,
    pub gravity: f32,
    pub drag: f32,
    pub bounce: bool,
}

pub struct Tracer {
    pub a: Vec3,
    pub b: Vec3,
    pub life: f32,
    pub max_life: f32,
    pub width: f32,
    pub color: Color,
}

pub struct Effects {
    pub particles: Vec<Particle>,
    pub tracers: Vec<Tracer>,
    pub hit_marker: f32,
    pub head_marker: f32,
    pub kill_flash: f32,
    pub damage_numbers: Vec<(Vec3, f32, f32, bool)>,
}

impl Effects {
    pub fn new() -> Self {
        Self {
            particles: Vec::new(),
            tracers: Vec::new(),
            hit_marker: 0.0,
            head_marker: 0.0,
            kill_flash: 0.0,
            damage_numbers: Vec::new(),
        }
    }

    pub fn clear(&mut self) {
        self.particles.clear();
        self.tracers.clear();
        self.damage_numbers.clear();
        self.hit_marker = 0.0;
        self.head_marker = 0.0;
        self.kill_flash = 0.0;
    }

    pub fn update(&mut self, dt: f32, level: &Level) {
        for p in &mut self.particles {
            p.vel.y -= p.gravity * dt;
            p.vel *= (1.0 - p.drag * dt).clamp(0.0, 1.0);
            // Sample the ground where the particle was: one frame of fall can
            // carry it straight past the surface it should have landed on.
            let from = p.pos.y;
            p.pos += p.vel * dt;
            if p.bounce && p.pos.y < from {
                let ground = level
                    .ground_below(p.pos.x, p.pos.z, p.size, from)
                    .map_or(GROUND_CLEARANCE, |g| g + GROUND_CLEARANCE);
                if p.pos.y < ground {
                    p.pos.y = ground;
                    p.vel.y = p.vel.y.abs() * 0.28;
                    p.vel.x *= 0.6;
                    p.vel.z *= 0.6;
                }
            }
            p.life -= dt;
        }
        self.particles.retain(|p| p.life > 0.0);
        if self.particles.len() > MAX_PARTICLES {
            let drop = self.particles.len() - MAX_PARTICLES;
            self.particles.drain(0..drop);
        }

        for t in &mut self.tracers {
            t.life -= dt;
        }
        self.tracers.retain(|t| t.life > 0.0);

        self.hit_marker = approach(self.hit_marker, 0.0, dt * 4.5);
        self.head_marker = approach(self.head_marker, 0.0, dt * 4.5);
        self.kill_flash = approach(self.kill_flash, 0.0, dt * 3.0);

        for d in &mut self.damage_numbers {
            d.0.y += dt * 0.85;
            d.2 -= dt;
        }
        self.damage_numbers.retain(|d| d.2 > 0.0);
    }

    pub fn spark(&mut self, point: Vec3, normal: Vec3, color: Color, count: usize, rng: &mut Rng) {
        for _ in 0..count {
            let dir = rng.cone(normal, 0.9);
            let speed = rng.range(2.0, 9.0);
            self.particles.push(Particle {
                pos: point,
                vel: dir * speed,
                life: rng.range(0.18, 0.5),
                max_life: 0.5,
                size: rng.range(0.016, 0.042),
                color,
                gravity: 16.0,
                drag: 1.6,
                bounce: true,
            });
        }
    }

    pub fn blood(&mut self, point: Vec3, dir: Vec3, color: Color, rng: &mut Rng) {
        for _ in 0..14 {
            let d = rng.cone(dir, 1.1);
            let speed = rng.range(1.5, 7.0);
            self.particles.push(Particle {
                pos: point,
                vel: d * speed,
                life: rng.range(0.25, 0.7),
                max_life: 0.7,
                size: rng.range(0.02, 0.055),
                color,
                gravity: 20.0,
                drag: 1.1,
                bounce: true,
            });
        }
    }

    pub fn smoke(&mut self, point: Vec3, color: Color, count: usize, rng: &mut Rng) {
        for _ in 0..count {
            let dir = vec3(rng.signed(), rng.range(0.2, 1.0), rng.signed()).normalize_or_zero();
            self.particles.push(Particle {
                pos: point,
                vel: dir * rng.range(0.3, 1.6),
                life: rng.range(0.5, 1.2),
                max_life: 1.2,
                size: rng.range(0.05, 0.14),
                color,
                gravity: -0.6,
                drag: 2.2,
                bounce: false,
            });
        }
    }

    pub fn tracer(&mut self, a: Vec3, b: Vec3, color: Color, width: f32) {
        self.tracers.push(Tracer {
            a,
            b,
            life: 0.055,
            max_life: 0.055,
            width,
            color,
        });
    }

    pub fn damage_number(&mut self, at: Vec3, amount: f32, head: bool) {
        self.damage_numbers.push((at, amount, 0.75, head));
    }

    /// All transient geometry batched into `dst` for one draw call.
    pub fn write_mesh(&self, dst: &mut Mesh) {
        let mut mb = MeshBuilder::reclaim(dst);
        for p in &self.particles {
            let t = (p.life / p.max_life).clamp(0.0, 1.0);
            let c = Color::new(p.color.r, p.color.g, p.color.b, p.color.a * t);
            let s = p.size * (0.4 + 0.6 * t);
            mb.add_box(p.pos - Vec3::splat(s), p.pos + Vec3::splat(s), c);
        }
        for t in &self.tracers {
            let k = (t.life / t.max_life).clamp(0.0, 1.0);
            let c = Color::new(t.color.r, t.color.g, t.color.b, t.color.a * k);
            mb.add_beam(t.a, t.b, t.width * k, c);
        }
        *dst = mb.finish();
    }
}
