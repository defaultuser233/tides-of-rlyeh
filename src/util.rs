use macroquad::prelude::*;
use std::f32::consts::TAU;

pub struct Rng {
    s: u64,
}

impl Rng {
    pub const fn new(seed: u64) -> Self {
        Self { s: seed | 1 }
    }

    pub fn u32(&mut self) -> u32 {
        self.s = self
            .s
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        // Take the high 32 bits. `>> 33` kept only 31, so every value above
        // 2^31 - 1 was unreachable and the whole game's randomness was skewed.
        (self.s >> 32) as u32
    }

    /// Uniform over `[0, 1)`.
    pub fn f32(&mut self) -> f32 {
        // 24 significant bits, which an `f32` represents exactly, so the result
        // never rounds up to 1.0 the way `u32::MAX as f32 / 2^32` would.
        (self.u32() >> 8) as f32 / (1u32 << 24) as f32
    }

    pub fn range(&mut self, lo: f32, hi: f32) -> f32 {
        lo + (hi - lo) * self.f32()
    }

    pub fn signed(&mut self) -> f32 {
        self.range(-1.0, 1.0)
    }

    pub fn chance(&mut self, p: f32) -> bool {
        self.f32() < p
    }

    /// Random direction inside a cone of `spread` radians around `dir`.
    pub fn cone(&mut self, dir: Vec3, spread: f32) -> Vec3 {
        let d = dir.normalize_or_zero();
        if d == Vec3::ZERO {
            return Vec3::NEG_Z;
        }
        let up = if d.y.abs() < 0.9 { Vec3::Y } else { Vec3::X };
        let t = d.cross(up).normalize();
        let b = d.cross(t);
        let a = self.range(0.0, TAU);
        let r = self.f32().sqrt() * spread;
        (d + (t * a.cos() + b * a.sin()) * r).normalize()
    }
}

/// Slab-method ray/AABB intersection. Returns the entry distance along `rd`.
pub fn ray_aabb(ro: Vec3, rd: Vec3, min: Vec3, max: Vec3) -> Option<f32> {
    // Perturb near-zero components so the reciprocal never produces 0 * inf = NaN.
    let rd = vec3(
        if rd.x.abs() < 1e-6 { 1e-6 } else { rd.x },
        if rd.y.abs() < 1e-6 { 1e-6 } else { rd.y },
        if rd.z.abs() < 1e-6 { 1e-6 } else { rd.z },
    );
    let inv = Vec3::ONE / rd;
    let t0 = (min - ro) * inv;
    let t1 = (max - ro) * inv;
    let tmin = t0.min(t1);
    let tmax = t0.max(t1);
    let enter = tmin.x.max(tmin.y).max(tmin.z);
    let exit = tmax.x.min(tmax.y).min(tmax.z);
    if exit < 0.0 || exit <= enter {
        None
    } else {
        Some(enter.max(0.0))
    }
}

pub fn approach(current: f32, target: f32, rate: f32) -> f32 {
    if current < target {
        (current + rate).min(target)
    } else {
        (current - rate).max(target)
    }
}

/// Forward direction from yaw/pitch, matching the convention used by `Player`.
pub fn dir_from_angles(yaw: f32, pitch: f32) -> Vec3 {
    let cp = pitch.cos();
    vec3(-yaw.sin() * cp, pitch.sin(), -yaw.cos() * cp)
}

/// Yaw angle that points along `d` (XZ plane only).
pub fn yaw_of(d: Vec3) -> f32 {
    (-d.x).atan2(-d.z)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The generator used to shift out only 31 bits and then divide by 2^32-1,
    /// so `f32()` could never exceed 0.5. Everything random in the game — drop
    /// rolls, spread, spawn facings, strafe direction — was skewed by it.
    #[test]
    fn f32_covers_the_whole_unit_interval() {
        let mut r = Rng::new(1);
        let (mut lo, mut hi) = (f32::MAX, f32::MIN);
        for _ in 0..100_000 {
            let v = r.f32();
            assert!((0.0..1.0).contains(&v), "f32() left [0,1): {v}");
            lo = lo.min(v);
            hi = hi.max(v);
        }
        assert!(lo < 0.01, "lower bound never reached: {lo}");
        assert!(hi > 0.99, "upper bound never reached: {hi}");
    }

    #[test]
    fn u32_uses_the_full_width() {
        let mut r = Rng::new(7);
        let hi = (0..10_000).map(|_| r.u32()).max().expect("10k draws");
        assert!(hi > u32::MAX / 2, "high bits are missing: {hi}");
    }

    /// `chance(0.5)` used to be unconditionally true, which is why every enemy
    /// strafed the same way.
    #[test]
    fn chance_is_actually_a_coin_flip() {
        let mut r = Rng::new(99);
        let heads = (0..20_000).filter(|_| r.chance(0.5)).count();
        assert!(
            (9_400..10_600).contains(&heads),
            "chance(0.5) gave {heads}/20000"
        );
    }

    #[test]
    fn signed_spans_both_signs() {
        let mut r = Rng::new(3);
        let (mut neg, mut pos) = (0, 0);
        for _ in 0..10_000 {
            if r.signed() < 0.0 {
                neg += 1;
            } else {
                pos += 1;
            }
        }
        assert!(
            neg > 4_000 && pos > 4_000,
            "signed(): {neg} negative, {pos} positive"
        );
    }

    /// `cone` is used for every pellet and every enemy shot.
    #[test]
    fn cone_stays_inside_its_angle_and_is_normalised() {
        let mut r = Rng::new(11);
        let axis = Vec3::NEG_Z;
        let spread = 0.2f32;
        for _ in 0..10_000 {
            let d = r.cone(axis, spread);
            assert!(
                (d.length() - 1.0).abs() < 1e-3,
                "not normalised: {}",
                d.length()
            );
            let angle = d.dot(axis).clamp(-1.0, 1.0).acos();
            assert!(
                angle <= spread + 1e-3,
                "escaped the cone: {angle} > {spread}"
            );
        }
    }

    fn hit(ro: Vec3, rd: Vec3) -> Option<f32> {
        ray_aabb(ro, rd, Vec3::splat(-1.0), Vec3::splat(1.0))
    }

    #[test]
    fn ray_aabb_handles_the_awkward_cases() {
        // Straight at it.
        let t = hit(vec3(0.0, 0.0, -5.0), Vec3::Z).expect("head-on hit");
        assert!((t - 4.0).abs() < 1e-3, "entry distance {t}");
        // Parallel to a face and outside: the degenerate reciprocal must not
        // turn into a NaN hit.
        assert!(
            hit(vec3(0.0, 5.0, -5.0), Vec3::Z).is_none(),
            "ray above the box hit"
        );
        // Pointing away.
        assert!(
            hit(vec3(0.0, 0.0, -5.0), Vec3::NEG_Z).is_none(),
            "ray aimed away hit"
        );
        // Starting inside reports zero, not a miss.
        assert_eq!(hit(Vec3::ZERO, Vec3::Z), Some(0.0), "origin inside the box");
        // Exactly along the +x face. The ray is tangent — it never enters the
        // interior — and the near-zero component is perturbed positive, so the
        // resolved entry falls outside the x slab and this reports a miss. A
        // tangency has no single right answer; what matters is that it is
        // deterministic and finite rather than a 0 * inf NaN.
        assert!(
            hit(vec3(1.0, 0.0, -5.0), Vec3::Z).is_none(),
            "a tangent ray is treated as a miss"
        );
        // One hair inside the face and it does hit, at a sane distance.
        let t = hit(vec3(1.0 - 1e-4, 0.0, -5.0), Vec3::Z).expect("just inside the face");
        assert!((t - 4.0).abs() < 1e-2, "entry distance {t}");
        // Diagonal, and nothing non-finite for any direction tried.
        for i in 0..64 {
            let a = i as f32 * 0.1;
            let d = vec3(a.sin(), (a * 0.7).cos(), (a * 1.3).sin()).normalize();
            if let Some(t) = hit(vec3(0.0, 0.0, -5.0), d) {
                assert!(t.is_finite() && t >= 0.0, "non-finite entry distance {t}");
            }
        }
    }
}
