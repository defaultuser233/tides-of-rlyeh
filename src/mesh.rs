use macroquad::prelude::*;

/// Direction of the baked-in key light. All shading is precomputed into vertex
/// colours because macroquad's default 3D material has no lighting model.
pub const SUN: Vec3 = vec3(-0.42, 0.80, 0.43);

/// How far up a wall the contact shading reaches, in metres.
const CONTACT_HEIGHT: f32 = 1.9;
/// How much darker a surface is at the point where it meets the floor.
const CONTACT_DARKEN: f32 = 0.34;

pub fn shade(color: Color, normal: Vec3) -> Color {
    let d = normal.normalize_or_zero().dot(SUN).max(0.0);
    // A little sky bounce from above keeps flat faces from going fully dark.
    let sky = (normal.y * 0.5 + 0.5) * 0.18;
    // Clamped rather than allowed to run to 1.12: a vertex colour above full
    // is clipped when it is packed, so the brightest faces lose the shade
    // separation between them — and now that a texture is multiplied in, the
    // texture pays for the clipping too.
    let f = (0.44 + 0.62 * d + sky).min(1.0);
    Color::new(color.r * f, color.g * f, color.b * f, color.a)
}

/// Cheap contact shading, standing in for the shadow pass this renderer does
/// not have: a surface is darker where it meets the floor and lightens as it
/// climbs. This is the whole of the cue that a crate is resting on something
/// rather than floating over it.
///
/// Only faces that stand up get it. A floor is at ground level everywhere, so
/// darkening by height would just darken every floor in the game.
///
/// The mesh has to be in its own local space with the base at `y = 0` — a
/// level mesh in world space, an enemy prototype before it is moved — or the
/// gradient lands in the wrong place.
pub fn contact_shading(mesh: &mut Mesh) {
    for v in &mut mesh.vertices {
        let normal = vec3(v.normal.x, v.normal.y, v.normal.z);
        let k = contact(normal, v.position.y);
        if k < 1.0 {
            v.color = rgba(Color::new(
                v.color[0] as f32 / 255.0 * k,
                v.color[1] as f32 / 255.0 * k,
                v.color[2] as f32 / 255.0 * k,
                v.color[3] as f32 / 255.0,
            ));
        }
    }
}

fn contact(normal: Vec3, y: f32) -> f32 {
    let upright = 1.0 - normal.normalize_or_zero().y.abs();
    let near = (1.0 - y / CONTACT_HEIGHT).clamp(0.0, 1.0);
    1.0 - CONTACT_DARKEN * upright * near * near
}

pub fn rgba(c: Color) -> [u8; 4] {
    [
        (c.r.clamp(0.0, 1.0) * 255.0) as u8,
        (c.g.clamp(0.0, 1.0) * 255.0) as u8,
        (c.b.clamp(0.0, 1.0) * 255.0) as u8,
        (c.a.clamp(0.0, 1.0) * 255.0) as u8,
    ]
}

/// The two unit axes a face's UVs are measured along, both inside its plane.
///
/// `u` is the world axis with the most of itself left over after the normal is
/// taken out of it, and `v` completes the pair. Picking the axis the normal is
/// *least* parallel to keeps that leftover at `sqrt(1 - 0.9²)`, about 0.44, so
/// neither normalisation ever sits near its degenerate case.
///
/// For the level this is exact rather than approximate, and the level is the
/// only thing here that is textured: everything in `maps.rs` is an AABB, so `n`
/// is `±X`, `±Y` or `±Z` to the bit, `n.dot(r)` is exactly zero, and the
/// reference axis crosses into `u` unchanged — two faces of one plane do not
/// merely agree to within a texel, they agree to the last bit. Beams and tumbled
/// grenade boxes are the general case: `u` and `v` are still orthonormal and
/// still in the face, so they keep the same texels per metre, and only the
/// rotation of the pattern inside such a face follows the world axes. Nothing
/// textured is built that way, so that rotation is never seen.
///
/// A face with no plane at all — three collinear corners, a beam of zero radius
/// — has no direction to be measured along, and neither does one whose normal
/// came out non-finite from non-finite corners, which `normalize_or_zero`
/// collapses to zero as well. Those get a fixed pair of world axes: an arbitrary
/// orientation is invisible on a face with no area, while a zero axis is what
/// would put a NaN in every UV on the face.
fn face_axes(n: Vec3) -> (Vec3, Vec3) {
    let r = if n.y.abs() > 0.9 { Vec3::X } else { Vec3::Y };
    let u = (r - n * n.dot(r)).normalize_or_zero();
    let v = n.cross(u).normalize_or_zero();
    if u == Vec3::ZERO || v == Vec3::ZERO {
        return (Vec3::X, Vec3::Z);
    }
    (u, v)
}

pub struct MeshBuilder {
    verts: Vec<Vertex>,
    idx: Vec<u16>,
    /// Texture repeats per metre. Set once per builder, because a builder fills
    /// exactly one mesh and a mesh carries exactly one texture — so the tile
    /// size is a property of the batch, not of each face.
    uv_scale: f32,
}

impl Default for MeshBuilder {
    fn default() -> Self {
        Self {
            verts: Vec::new(),
            idx: Vec::new(),
            // One repeat per metre. Untextured geometry ignores its UVs, so the
            // only caller that has to think about this is the level.
            uv_scale: 1.0,
        }
    }
}

impl MeshBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    /// How many times the texture repeats over one metre of face.
    ///
    /// This is what stops a 20 m wall from being one stretched tile: the axes a
    /// face is measured along are unit vectors lying in that face's plane, so a
    /// metre of surface is a metre of texture on a wall panel and on a crate
    /// alike, whatever their sizes.
    pub fn set_uv_scale(&mut self, repeats_per_metre: f32) {
        self.uv_scale = repeats_per_metre;
    }

    /// One planar face, wound `a b c d`.
    ///
    /// A vertex's UV is where it is in the world, measured along the two axes
    /// of the face's own plane (`face_axes`) and scaled by the tile density.
    /// Nothing in it comes from `a` — that is the point of the change. Two faces
    /// of one plane then carry one continuous pattern however the world is
    /// carved into boxes: the halves of a floor built from separate slabs, the
    /// lid of a crate and the ledge it sits on, a wall that carries on around a
    /// corner. Taking each face's own first corner as the UV origin put a
    /// private phase on every quad, which is invisible on a lone box and a
    /// visible step in the tiling everywhere two of them meet — the floor of
    /// `maps.rs` is a run of slabs, so it was a seam every few metres.
    ///
    /// The UVs are baked from world position when the mesh is built, so geometry
    /// transformed afterwards ([`transform_in_place`]) keeps the UVs of where it
    /// was built. Only the level is textured, and it is built in place.
    pub fn quad(&mut self, a: Vec3, b: Vec3, c: Vec3, d: Vec3, color: Color) {
        let n = (b - a).cross(c - a).normalize_or_zero();
        let col = rgba(shade(color, n));
        let n4 = n.extend(0.0);
        let (u_axis, v_axis) = face_axes(n);
        let base = self.verts.len() as u16;
        for p in [a, b, c, d] {
            let uv = vec2(p.dot(u_axis), p.dot(v_axis)) * self.uv_scale;
            self.verts.push(Vertex {
                position: p,
                uv,
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
    ///
    /// The cross-section is what the UVs of these faces are measured in, and it
    /// is the one thing here that is not an AABB: `face_axes` gives a beam's
    /// faces an orthonormal pair of world-referenced axes all the same, so they
    /// tile at the same density as the level. Nothing drawn this way is
    /// textured, so the rotation of the pattern inside the face never shows.
    pub fn add_beam(&mut self, a: Vec3, b: Vec3, radius: f32, color: Color) {
        let d = (b - a).normalize_or_zero();
        if d == Vec3::ZERO {
            return;
        }
        // `up` is picked to leave `d` well out of line with it, so the first
        // cross product cannot be zero; the second one is zero whenever the
        // radius is, and normalising that would put a NaN in every corner of the
        // beam — which reaches the vertex buffer and kills the draw rather than
        // just the shape. A zero radius collapses the prism onto the segment
        // instead, which draws nothing, as it should.
        let up = if d.y.abs() < 0.9 { Vec3::Y } else { Vec3::X };
        let t = d.cross(up).normalize() * radius;
        let u = d.cross(t).normalize_or_zero() * radius;
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

    /// Borrow `mesh`'s buffers for this builder, leaving `mesh` empty but with
    /// its allocations intact. Hand them straight back with
    /// [`MeshBuilder::finish`] — `*dst = MeshBuilder::reclaim(dst).finish()` is
    /// the whole round trip.
    ///
    /// A mesh rebuilt every frame wants this instead of [`MeshBuilder::new`]:
    /// the vectors keep the capacity they last needed, so the rebuild stops
    /// costing two allocations per frame.
    pub fn reclaim(mesh: &mut Mesh) -> Self {
        let mut verts = std::mem::take(&mut mesh.vertices);
        let mut idx = std::mem::take(&mut mesh.indices);
        verts.clear();
        idx.clear();
        mesh.texture = None;
        Self {
            verts,
            idx,
            ..Self::default()
        }
    }
}

/// A mesh with nothing in it. `Mesh` does not implement `Default`, so every
/// scratch or output mesh starts here.
pub fn empty_mesh() -> Mesh {
    Mesh {
        vertices: Vec::new(),
        indices: Vec::new(),
        texture: None,
    }
}

/// Overwrite `dst` with a copy of `src`, keeping whatever `dst` already has
/// allocated. The allocation-free half of [`transform_into`].
pub fn copy_into(dst: &mut Mesh, src: &Mesh) {
    dst.vertices.clear();
    dst.vertices.extend_from_slice(&src.vertices);
    dst.indices.clear();
    dst.indices.extend_from_slice(&src.indices);
    dst.texture = src.texture.clone();
}

/// Apply `mat` to every vertex, in place.
pub fn transform_in_place(mesh: &mut Mesh, mat: Mat4) {
    for v in &mut mesh.vertices {
        v.position = mat.transform_point3(v.position);
        v.normal = mat
            .transform_vector3(v.normal.truncate())
            .normalize_or_zero()
            .extend(0.0);
    }
}

/// Transform `src` into `dst`, reusing `dst`'s buffers.
pub fn transform_into(dst: &mut Mesh, src: &Mesh, mat: Mat4) {
    copy_into(dst, src);
    transform_in_place(dst, mat);
}

/// Recolour a mesh in place: `amount` blends toward `tint`, `alpha` overrides
/// opacity (used for the death fade-out).
pub fn tint_in_place(mesh: &mut Mesh, tint: Color, amount: f32, alpha: f32) {
    let a = amount.clamp(0.0, 1.0);
    for v in &mut mesh.vertices {
        let c = Color::new(
            (v.color[0] as f32 / 255.0) * (1.0 - a) + tint.r * a,
            (v.color[1] as f32 / 255.0) * (1.0 - a) + tint.g * a,
            (v.color[2] as f32 / 255.0) * (1.0 - a) + tint.b * a,
            alpha,
        );
        v.color = rgba(c);
    }
}
