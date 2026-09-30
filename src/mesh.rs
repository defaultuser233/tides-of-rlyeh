use macroquad::prelude::*;

/// Direction of the baked-in key light. All shading is precomputed into vertex
/// colours because macroquad's default 3D material has no lighting model.
pub const SUN: Vec3 = vec3(-0.42, 0.80, 0.43);

pub fn shade(color: Color, normal: Vec3) -> Color {
    let d = normal.normalize_or_zero().dot(SUN).max(0.0);
    // A little sky bounce from above keeps flat faces from going fully dark.
    let sky = (normal.y * 0.5 + 0.5) * 0.18;
    let f = 0.44 + 0.62 * d + sky;
    Color::new(color.r * f, color.g * f, color.b * f, color.a)
}

pub fn rgba(c: Color) -> [u8; 4] {
    [
        (c.r.clamp(0.0, 1.0) * 255.0) as u8,
        (c.g.clamp(0.0, 1.0) * 255.0) as u8,
        (c.b.clamp(0.0, 1.0) * 255.0) as u8,
        (c.a.clamp(0.0, 1.0) * 255.0) as u8,
    ]
}

#[derive(Default)]
pub struct MeshBuilder {
    verts: Vec<Vertex>,
    idx: Vec<u16>,
}

impl MeshBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    /// A builder with room for `verts` vertices and `idx` indices. A caller that
    /// knows the size of its batch up front — a particle system, say — can size
    /// its builder once instead of paying for the vectors doubling a dozen
    /// times, which on a per-frame mesh is a dozen reallocations per frame.
    pub fn with_capacity(verts: usize, idx: usize) -> Self {
        Self {
            verts: Vec::with_capacity(verts),
            idx: Vec::with_capacity(idx),
        }
    }

    pub fn quad(&mut self, a: Vec3, b: Vec3, c: Vec3, d: Vec3, color: Color) {
        let n = (b - a).cross(c - a).normalize_or_zero();
        let col = rgba(shade(color, n));
        let n4 = n.extend(0.0);
        let base = self.verts.len() as u16;
        for p in [a, b, c, d] {
            self.verts.push(Vertex {
                position: p,
                uv: Vec2::ZERO,
                color: col,
                normal: n4,
            });
        }
        self.idx
            .extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }

    /// Axis-aligned box. Vertices are placed so every face gets its own
    /// precomputed shading.
    pub fn add_box(&mut self, min: Vec3, max: Vec3, color: Color) {
        let (x0, y0, z0) = (min.x, min.y, min.z);
        let (x1, y1, z1) = (max.x, max.y, max.z);
        self.quad(
            vec3(x0, y1, z0),
            vec3(x0, y1, z1),
            vec3(x1, y1, z1),
            vec3(x1, y1, z0),
            color,
        );
        self.quad(
            vec3(x0, y0, z1),
            vec3(x0, y0, z0),
            vec3(x1, y0, z0),
            vec3(x1, y0, z1),
            color,
        );
        self.quad(
            vec3(x0, y0, z1),
            vec3(x1, y0, z1),
            vec3(x1, y1, z1),
            vec3(x0, y1, z1),
            color,
        );
        self.quad(
            vec3(x1, y0, z0),
            vec3(x0, y0, z0),
            vec3(x0, y1, z0),
            vec3(x1, y1, z0),
            color,
        );
        self.quad(
            vec3(x1, y0, z1),
            vec3(x1, y0, z0),
            vec3(x1, y1, z0),
            vec3(x1, y1, z1),
            color,
        );
        self.quad(
            vec3(x0, y0, z0),
            vec3(x0, y0, z1),
            vec3(x0, y1, z1),
            vec3(x0, y1, z0),
            color,
        );
    }

    /// Thin prism from `a` to `b`; used for tracers, spawn beams and sparks.
    pub fn add_beam(&mut self, a: Vec3, b: Vec3, radius: f32, color: Color) {
        let d = (b - a).normalize_or_zero();
        if d == Vec3::ZERO {
            return;
        }
        let up = if d.y.abs() < 0.9 { Vec3::Y } else { Vec3::X };
        let t = d.cross(up).normalize() * radius;
        let u = d.cross(t).normalize() * radius;
        let p = [a + t, a + u, a - t, a - u];
        let q = [b + t, b + u, b - t, b - u];
        for i in 0..4 {
            let j = (i + 1) % 4;
            self.quad(p[i], q[i], q[j], p[j], color);
        }
        self.quad(p[3], p[2], p[1], p[0], color);
        self.quad(q[0], q[1], q[2], q[3], color);
    }

    pub fn finish(self) -> Mesh {
        Mesh {
            vertices: self.verts,
            indices: self.idx,
            texture: None,
        }
    }
}

/// `Mesh` does not implement `Clone`, so copies are built field by field.
pub fn clone_mesh(src: &Mesh) -> Mesh {
    Mesh {
        vertices: src
            .vertices
            .iter()
            .map(|v| Vertex {
                position: v.position,
                uv: v.uv,
                color: v.color,
                normal: v.normal,
            })
            .collect(),
        indices: src.indices.clone(),
        texture: src.texture.clone(),
    }
}

pub fn transform_mesh(src: &Mesh, mat: Mat4) -> Mesh {
    let mut out = clone_mesh(src);
    for v in &mut out.vertices {
        v.position = mat.transform_point3(v.position);
        v.normal = mat
            .transform_vector3(v.normal.truncate())
            .normalize_or_zero()
            .extend(0.0);
    }
    out
}

/// Recolour a prototype mesh: `amount` blends toward `tint`, `alpha` overrides
/// opacity (used for the death fade-out).
pub fn tint_mesh(src: &Mesh, tint: Color, amount: f32, alpha: f32) -> Mesh {
    let a = amount.clamp(0.0, 1.0);
    let mut out = clone_mesh(src);
    for v in &mut out.vertices {
        let c = Color::new(
            (v.color[0] as f32 / 255.0) * (1.0 - a) + tint.r * a,
            (v.color[1] as f32 / 255.0) * (1.0 - a) + tint.g * a,
            (v.color[2] as f32 / 255.0) * (1.0 - a) + tint.b * a,
            alpha,
        );
        v.color = rgba(c);
    }
    out
}
