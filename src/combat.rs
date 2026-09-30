use crate::enemy::Enemy;
use crate::level::Level;
use macroquad::prelude::*;

#[derive(Clone, Copy)]
pub enum HitKind {
    World,
    Enemy { index: usize, head: bool },
}

#[derive(Clone, Copy)]
pub struct Hit {
    pub kind: HitKind,
    pub point: Vec3,
    pub dist: f32,
}

/// Nearest thing struck by a ray, considering both level geometry and enemies.
pub fn hitscan(level: &Level, enemies: &[Enemy], origin: Vec3, dir: Vec3, max: f32) -> Hit {
    let mut best_t = level.ray_world(origin, dir, max).unwrap_or(max);
    let mut kind = HitKind::World;

    for (i, e) in enemies.iter().enumerate() {
        if !e.targetable() {
            continue;
        }
        // No lower bound on `t`. At point-blank range the camera sits inside
        // the enemy's box, `ray_aabb` reports an entry distance of 0, and the
        // old `t > 0.02` test discarded the hit and blamed the world — so the
        // enemy you were touching was the one you could not shoot. `head` is
        // forced false there, because the origin is the eye and every
        // point-blank shot would otherwise score as a headshot.
        if let Some(t) = e.body_aabb().ray(origin, dir)
            && t < best_t
        {
            best_t = t;
            let point = origin + dir * t;
            let head = t > 0.001 && point.y > e.head_y();
            kind = HitKind::Enemy { index: i, head };
        }
    }

    Hit {
        kind,
        point: origin + dir * best_t,
        dist: best_t,
    }
}
