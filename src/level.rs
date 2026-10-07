use crate::mesh::{MeshBuilder, contact_shading};
use crate::surface::{Skin, Surface, Textures};
use crate::util::ray_aabb;
use macroquad::prelude::*;
use std::collections::VecDeque;

pub const ARENA: f32 = 26.0;
pub const WALL_H: f32 = 5.0;
/// Bodies can step over obstacles up to this height (crates, stairs).
pub const STEP_HEIGHT: f32 = 0.62;

#[derive(Clone, Copy)]
pub struct Aabb {
    pub min: Vec3,
    pub max: Vec3,
}

impl Aabb {
    pub const fn new(min: Vec3, max: Vec3) -> Self {
        Self { min, max }
    }

    /// Inside or on the boundary.
    pub fn contains(&self, p: Vec3) -> bool {
        p.x >= self.min.x
            && p.x <= self.max.x
            && p.y >= self.min.y
            && p.y <= self.max.y
            && p.z >= self.min.z
            && p.z <= self.max.z
    }

    pub fn overlaps(&self, o: &Aabb) -> bool {
        self.min.x < o.max.x
            && self.max.x > o.min.x
            && self.min.y < o.max.y
            && self.max.y > o.min.y
            && self.min.z < o.max.z
            && self.max.z > o.min.z
    }

    pub fn ray(&self, ro: Vec3, rd: Vec3) -> Option<f32> {
        ray_aabb(ro, rd, self.min, self.max)
    }
}

#[derive(Default, Clone, Copy)]
pub struct Collision {
    pub x: bool,
    pub y: bool,
    pub z: bool,
    pub ground: bool,
    pub ceiling: bool,
}

fn body(pos: Vec3, radius: f32, height: f32) -> Aabb {
    Aabb::new(
        vec3(pos.x - radius, pos.y, pos.z - radius),
        vec3(pos.x + radius, pos.y + height, pos.z + radius),
    )
}

const EPS: f32 = 1e-3;

/// True when the segment `a` .. `a + dir * dist` passes through a sphere.
fn segment_hits_sphere(a: Vec3, dir: Vec3, dist: f32, centre: Vec3, radius: f32) -> bool {
    let t = (centre - a).dot(dir).clamp(0.0, dist);
    (a + dir * t).distance_squared(centre) < radius * radius
}

fn resolve_axis(
    level: &Level,
    pos: Vec3,
    radius: f32,
    height: f32,
    delta: f32,
    axis: usize,
) -> (f32, bool) {
    let origin = if axis == 0 {
        pos.x
    } else if axis == 1 {
        pos.y
    } else {
        pos.z
    };
    if delta == 0.0 {
        return (origin, false);
    }
    let moved = origin + delta;

    // Sweep: test the body at its destination, not at its origin.
    let mut probe = pos;
    match axis {
        0 => probe.x = moved,
        1 => probe.y = moved,
        _ => probe.z = moved,
    }
    let b = body(probe, radius, height);

    if delta > 0.0 {
        let mut lim = f32::INFINITY;
        for s in &level.solids {
            if s.overlaps(&b) {
                let v = match axis {
                    0 => s.min.x - radius - EPS,
                    1 => s.min.y - height - EPS,
                    _ => s.min.z - radius - EPS,
                };
                lim = lim.min(v);
            }
        }
        if lim.is_finite() {
            // `lim` is always behind `moved` for a solid we actually entered,
            // so this both stops the sweep and pushes out of any pre-existing
            // overlap.
            return (lim, true);
        }
    } else {
        let mut lim = f32::NEG_INFINITY;
        for s in &level.solids {
            if s.overlaps(&b) {
                let v = match axis {
                    0 => s.max.x + radius + EPS,
                    1 => s.max.y + EPS,
                    _ => s.max.z + radius + EPS,
                };
                lim = lim.max(v);
            }
        }
        if lim.is_finite() {
            return (lim, true);
        }
    }
    (moved, false)
}

fn resolve_xz(
    level: &Level,
    pos: Vec3,
    radius: f32,
    height: f32,
    dx: f32,
    dz: f32,
) -> (Vec3, bool, bool) {
    let mut p = pos;
    let (nx, hx) = resolve_axis(level, p, radius, height, dx, 0);
    p.x = nx;
    let (nz, hz) = resolve_axis(level, p, radius, height, dz, 2);
    p.z = nz;
    (p, hx, hz)
}

/// One collision step: resolve each axis against the most restrictive contact,
/// optionally attempting a step-up when blocked.
fn move_once(
    level: &Level,
    pos: Vec3,
    radius: f32,
    height: f32,
    delta: Vec3,
    step_height: f32,
) -> (Vec3, Collision) {
    let mut c = Collision::default();
    let (mut p, hx, hz) = resolve_xz(level, pos, radius, height, delta.x, delta.z);
    c.x = hx;
    c.z = hz;

    if (hx || hz) && step_height > 0.0 {
        let lifted = pos + vec3(0.0, step_height, 0.0);
        if level.point_free(lifted, radius, height) {
            let (p2, hx2, hz2) = resolve_xz(level, lifted, radius, height, delta.x, delta.z);
            if !hx2
                && !hz2
                && let Some(top) = level.support_height(p2.x, p2.z, radius, lifted.y)
                && top > pos.y + 0.004
            {
                p.x = p2.x;
                p.z = p2.z;
                p.y = top;
                c.x = false;
                c.z = false;
            }
        }
    }

    let (ny, hy) = resolve_axis(level, p, radius, height, delta.y, 1);
    if hy {
        c.y = true;
        if delta.y < 0.0 {
            c.ground = true;
        } else {
            c.ceiling = true;
        }
    }
    p.y = ny;

    (p, c)
}

/// Sweep a body through the level in substeps short enough that it cannot pass
/// through anything.
///
/// The per-axis resolution only ever tests the *destination*, so any single move
/// longer than the thinnest solid plus the body's own width would emerge on the
/// far side. Keeping that guarantee here — instead of trusting every caller to
/// clamp its own step — is what lets `MAX_FALL` and the sprint speed mean what
/// they say, and it gives enemies the same protection for free. The old
/// arrangement had the player clamping its delta to 0.85 m (so the effective
/// terminal velocity silently became frame-rate dependent) and enemies clamping
/// nothing at all.
pub fn move_and_collide(
    level: &Level,
    pos: Vec3,
    radius: f32,
    height: f32,
    delta: Vec3,
    step_height: f32,
) -> (Vec3, Collision) {
    // Comfortably under the thinnest solid in any arena (0.15 m) plus the
    // smallest body radius (0.26 m).
    const MAX_SUBSTEP: f32 = 0.20;

    let steps = (delta.length() / MAX_SUBSTEP).ceil().max(1.0) as usize;
    let sub = delta / steps as f32;
    let mut p = pos;
    let mut c = Collision::default();
    for _ in 0..steps {
        let (next, nc) = move_once(level, p, radius, height, sub, step_height);
        p = next;
        c.x |= nc.x;
        c.y |= nc.y;
        c.z |= nc.z;
        c.ground |= nc.ground;
        c.ceiling |= nc.ceiling;
    }
    (p, c)
}

pub struct FlowField {
    min: Vec2,
    cell: f32,
    w: usize,
    h: usize,
    dist: Vec<u16>,
    dir: Vec<Vec2>,
    passable: Vec<bool>,
    /// Flat position of the last target `update` was given, kept so `dir_at`
    /// can point a unit straight at it when the grid has nothing to say.
    target: Vec2,
    /// False until `update` has been given a target. The fallback has nowhere to
    /// point before then, and a field that has never been built must steer
    /// nothing rather than point every unit at the world origin — which is what
    /// the first version of this did, and what
    /// `a_standing_enemy_does_not_march_in_place` caught.
    aimed: bool,
}

impl FlowField {
    fn new(min: Vec2, max: Vec2, cell: f32) -> Self {
        let w = ((max.x - min.x) / cell).ceil() as usize + 1;
        let h = ((max.y - min.y) / cell).ceil() as usize + 1;
        Self {
            min,
            cell,
            w,
            h,
            dist: vec![u16::MAX; w * h],
            dir: vec![Vec2::ZERO; w * h],
            passable: vec![true; w * h],
            target: Vec2::ZERO,
            aimed: false,
        }
    }

    fn idx(&self, ix: usize, iz: usize) -> usize {
        iz * self.w + ix
    }

    fn cell_center(&self, ix: usize, iz: usize) -> Vec2 {
        self.min + vec2(ix as f32 + 0.5, iz as f32 + 0.5) * self.cell
    }

    fn cell_of(&self, p: Vec3) -> (isize, isize) {
        (
            ((p.x - self.min.x) / self.cell).floor() as isize,
            ((p.z - self.min.y) / self.cell).floor() as isize,
        )
    }

    fn in_bounds(&self, ix: isize, iz: isize) -> bool {
        ix >= 0 && iz >= 0 && (ix as usize) < self.w && (iz as usize) < self.h
    }

    /// Nearest passable cell to `p`, spiralling outward on the grid.
    fn nearest_passable(&self, p: Vec3) -> Option<(usize, usize)> {
        let (cx, cz) = self.cell_of(p);
        if self.in_bounds(cx, cz) && self.passable[self.idx(cx as usize, cz as usize)] {
            return Some((cx as usize, cz as usize));
        }
        for r in 1..=8isize {
            for dz in -r..=r {
                for dx in -r..=r {
                    if dx.abs() != r && dz.abs() != r {
                        continue;
                    }
                    let (ix, iz) = (cx + dx, cz + dz);
                    if self.in_bounds(ix, iz) && self.passable[self.idx(ix as usize, iz as usize)] {
                        return Some((ix as usize, iz as usize));
                    }
                }
            }
        }
        None
    }

    /// The field's own answer for the cell `p` stands in: zero where the search
    /// found no route. `dir_at` is what steering asks; this is what a unit asks
    /// about *itself*, to find out whether it is on the field at all.
    pub fn flow_at(&self, p: Vec3) -> Vec2 {
        let (ix, iz) = self.cell_of(p);
        if !self.in_bounds(ix, iz) {
            return Vec2::ZERO;
        }
        self.dir[self.idx(ix as usize, iz as usize)]
    }

    /// Steering toward the target. Falls back to walking straight at it where
    /// the field has nothing to say — which is the difference between a unit
    /// that takes a long way round and a unit that stands in a corridor for the
    /// rest of the mission.
    ///
    /// Nothing on the walkable field is ever without a direction: the search
    /// fills every cell it can reach, and a cell it cannot reach is a cell the
    /// unit has no business being in. One that is there anyway is off the grid
    /// — inside the footprint of a solid, or shoved into a pocket — and standing
    /// still is what it used to do about it, forever. Since a wave does not end
    /// until every body it released is down, one such unit was enough to stop
    /// the mission. Straight at the target is not a route, but the collision
    /// solver slides a unit along geometry, so it is usually enough to get back
    /// onto the field.
    pub fn dir_at(&self, p: Vec3) -> Vec2 {
        let d = self.flow_at(p);
        if d != Vec2::ZERO {
            return d;
        }
        if !self.aimed {
            return Vec2::ZERO;
        }
        (self.target - vec2(p.x, p.z)).normalize_or_zero()
    }

    /// Recompute distances from `target` and derive a flow direction per cell.
    pub fn update(&mut self, target: Vec3) {
        // Remembered so `dir_at` has something to fall back to: a unit standing
        // where the field never reached still has to walk somewhere.
        self.target = vec2(target.x, target.z);
        self.aimed = true;
        self.dist.fill(u16::MAX);
        let Some((tx, tz)) = self.nearest_passable(target) else {
            return;
        };

        let mut queue = VecDeque::new();
        let start = self.idx(tx, tz);
        self.dist[start] = 0;
        queue.push_back(start);

        while let Some(i) = queue.pop_front() {
            let d = self.dist[i];
            let ix = i % self.w;
            let iz = i / self.w;
            let neighbours = [
                (ix.wrapping_sub(1), iz),
                (ix + 1, iz),
                (ix, iz.wrapping_sub(1)),
                (ix, iz + 1),
            ];
            for (nx, nz) in neighbours {
                if nx >= self.w || nz >= self.h {
                    continue;
                }
                let ni = self.idx(nx, nz);
                if !self.passable[ni] || self.dist[ni] != u16::MAX {
                    continue;
                }
                self.dist[ni] = d.saturating_add(1);
                queue.push_back(ni);
            }
        }

        // Convert the distance field into steering vectors.
        self.dir.fill(Vec2::ZERO);
        for iz in 0..self.h {
            for ix in 0..self.w {
                let i = self.idx(ix, iz);
                let d = self.dist[i];
                if d == u16::MAX || d == 0 {
                    continue;
                }
                let mut steer = Vec2::ZERO;
                let neighbours = [
                    (ix.wrapping_sub(1), iz),
                    (ix + 1, iz),
                    (ix, iz.wrapping_sub(1)),
                    (ix, iz + 1),
                ];
                for (nx, nz) in neighbours {
                    if nx >= self.w || nz >= self.h {
                        continue;
                    }
                    let ni = self.idx(nx, nz);
                    if self.dist[ni] < d {
                        let delta = self.cell_center(nx, nz) - self.cell_center(ix, iz);
                        steer += delta.normalize_or_zero();
                    }
                }
                self.dir[i] = steer.normalize_or_zero();
            }
        }
    }
}

/// One block of level geometry: an axis-aligned box and how it looks.
#[derive(Clone, Copy)]
pub struct Block {
    pub min: Vec3,
    pub max: Vec3,
    /// The colour it is painted and the surface texture under it.
    pub skin: Skin,
}

/// Declarative arena description. Campaign missions supply one of these and
/// `Level::from_layout` turns it into collision geometry, a render mesh and a
/// navigation grid.
pub struct Layout {
    pub name_en: &'static str,
    pub name_zh: &'static str,
    pub boxes: Vec<Block>,
    pub player_start: Vec3,
    pub spawns: Vec<Vec3>,
    /// Where the player must reach to finish "extract" objectives.
    pub extraction: Option<Vec3>,
    /// Half-extent of the navigation grid; also the default outer shell size.
    pub half: f32,
    pub sky: Color,
    /// The arena is under water: the backdrop is a water column rather than a
    /// sky, the fog is murk instead of haze, and suspended matter drifts
    /// through the air. Only the drowned city sets it.
    pub sunken: bool,
    /// The colour this arena is remembered by, for the HUD to pick up. Brighter
    /// than the palette's own trim, which is a paint colour seen under lighting
    /// rather than a colour drawn straight onto a panel.
    pub accent: Color,
    /// Bottomless pits, as XZ rectangles. Navigation treats them as
    /// impassable, and anything that ends up below the floor line inside one
    /// is gone. They exist because the nav rule ("a solid taller than 1.2 m
    /// blocks") cannot express a hole: any box tall enough to block pathing is
    /// also tall enough to bump into a 1.8 m player.
    pub voids: Vec<(Vec2, Vec2)>,
}

/// A lingering volume in the arena: a cloud, or a fire.
///
/// It never affects movement, only sight — and, when it burns, whatever is
/// standing in it. One type for both because that is what they are: a fire is a
/// cloud that also burns, with a smaller radius and a longer life. Keeping them
/// apart would mean two lists with the same radius curve, the same decay and the
/// same entry in `line_of_sight`.
pub struct Volume {
    pub pos: Vec3,
    pub radius: f32,
    pub life: f32,
    pub max_life: f32,
    /// Damage per second to anything standing in it. Zero for a smoke screen,
    /// which is the whole difference between the two.
    pub burn: f32,
}

impl Volume {
    /// Smoke billows as it ages, then thins out at the very end. A fire uses the
    /// same curve: it spreads as it takes hold and dies down at the end of its
    /// life, and what a volume's radius means to the reader — how much of the
    /// arena it covers — is the same question either way.
    pub fn current_radius(&self) -> f32 {
        let age = 1.0 - (self.life / self.max_life).clamp(0.0, 1.0);
        let grow = (age / 0.25).clamp(0.0, 1.0);
        let fade = (self.life / self.max_life).clamp(0.0, 1.0).powf(0.35);
        self.radius * (0.25 + 0.75 * grow) * fade
    }
}

pub struct Level {
    pub solids: Vec<Aabb>,
    /// The material of each entry of `solids`, same order and same length.
    ///
    /// Kept because nothing else does: the boxes are batched into one mesh per
    /// surface and the block list is dropped, so the material a round lands on
    /// is gone by the time it lands. Without this the impact sound is the same
    /// for a concrete wall and a steel plate (review R17).
    pub materials: Vec<Surface>,
    /// Obstacles tall enough to block movement and line of sight.
    pub blockers: Vec<Aabb>,
    /// One mesh per surface, in `Surface::ALL` order, and empty where an arena
    /// uses none of that material. A `Mesh` carries exactly one texture
    /// (`models.rs:41-45`), so batching by material is what makes a textured
    /// world possible at all — it costs at most six draw calls.
    pub meshes: Vec<Mesh>,
    pub flow: FlowField,
    pub spawns: Vec<Vec3>,
    pub player_start: Vec3,
    pub extraction: Option<Vec3>,
    pub sky: Color,
    /// See `Layout::sunken`.
    pub sunken: bool,
    pub name_en: &'static str,
    pub name_zh: &'static str,
    /// See `Layout::accent`.
    pub accent: Color,
    /// Active smoke screens and fires. Owned here so that `line_of_sight` can
    /// consult them without every caller having to thread a separate list
    /// through — a wall of flame hides what is behind it as well as a cloud does.
    pub volumes: Vec<Volume>,
    /// See `Layout::voids`.
    pub voids: Vec<(Vec2, Vec2)>,
    /// Half-extent of the arena, from `Layout::half`.
    pub half: f32,
}

impl Level {
    pub fn from_layout(layout: &Layout) -> Self {
        // One builder per surface. Each carries its own tile density, so a
        // concrete slab and a crate panel both get texture at the same scale
        // per metre no matter how big the box is.
        let mut builders: [MeshBuilder; Surface::ALL.len()] = std::array::from_fn(|i| {
            let mut mb = MeshBuilder::new();
            mb.set_uv_scale(1.0 / Surface::ALL[i].metres_per_tile());
            mb
        });
        let mut solids = Vec::with_capacity(layout.boxes.len());
        let mut materials = Vec::with_capacity(layout.boxes.len());
        let mut blockers = Vec::new();
        for block in &layout.boxes {
            builders[block.skin.surface.index()].add_box(
                block.min,
                block.max,
                block.skin.uniform(),
            );
            let (min, max) = (block.min, block.max);
            let a = Aabb::new(min, max);
            solids.push(a);
            materials.push(block.skin.surface);
            // Anything taller than a body can step over has to block the
            // navigation grid, or the flow field routes enemies straight at
            // knee-high cover and they grind against it instead of going round.
            // This used to be 1.2, which left everything in (0.62, 1.2] —
            // several 1.0-1.2 m walls, crates and one 15x26 m platform —
            // passable on the grid but not climbable in the world.
            if max.y > STEP_HEIGHT + 0.02 && min.y < 1.6 {
                blockers.push(a);
            }
        }

        let h = layout.half;
        let mut flow = FlowField::new(vec2(-h, -h), vec2(h, h), 1.0);
        let enemy_clearance = 0.62;
        for iz in 0..flow.h {
            for ix in 0..flow.w {
                let c = flow.cell_center(ix, iz);
                let probe = Aabb::new(
                    vec3(c.x - enemy_clearance, 0.15, c.y - enemy_clearance),
                    vec3(c.x + enemy_clearance, 1.7, c.y + enemy_clearance),
                );
                let in_void = layout
                    .voids
                    .iter()
                    .any(|(min, max)| c.x >= min.x && c.x <= max.x && c.y >= min.y && c.y <= max.y);
                let blocked = in_void || blockers.iter().any(|s| s.overlaps(&probe));
                let i = iz * flow.w + ix;
                flow.passable[i] = !blocked;
            }
        }

        let mut meshes: Vec<Mesh> = builders.into_iter().map(MeshBuilder::finish).collect();
        for mesh in &mut meshes {
            contact_shading(mesh);
        }

        Self {
            solids,
            materials,
            blockers,
            meshes,
            flow,
            spawns: layout.spawns.clone(),
            player_start: layout.player_start,
            extraction: layout.extraction,
            sky: layout.sky,
            sunken: layout.sunken,
            accent: layout.accent,
            name_en: layout.name_en,
            name_zh: layout.name_zh,
            volumes: Vec::new(),
            voids: layout.voids.clone(),
            half: layout.half,
        }
    }

    /// Attach the loaded textures to the per-surface meshes.
    ///
    /// Separate from `from_layout` because it is the one step that needs a live
    /// GL context, and everything that builds a `Level` in a test has none.
    pub fn dress(&mut self, textures: &Textures) {
        for (mesh, surface) in self.meshes.iter_mut().zip(Surface::ALL) {
            mesh.texture = Some(textures.get(surface).clone());
        }
    }

    pub fn point_free(&self, pos: Vec3, radius: f32, height: f32) -> bool {
        let b = body(pos, radius, height);
        !self.solids.iter().any(|s| s.overlaps(&b))
    }

    /// Whether the navigation field has a route from where `p` stands.
    ///
    /// The question a unit asks about itself when it has been pushing in one
    /// direction and going nowhere: a unit whose own cell has no flow is not
    /// taking a long way round, it is off the field.
    pub fn has_flow(&self, p: Vec3) -> bool {
        self.flow.flow_at(p) != Vec2::ZERO
    }

    /// Centre of the nearest walkable, unoccupied cell to `p`, if there is one.
    ///
    /// The other half of `has_flow`: a unit off the field is put back on it. A
    /// small teleport is a worse-looking thing than any other fix here, and it is
    /// still the right trade — a unit that can never reach the player holds its
    /// wave open for the rest of the mission, and a wave nobody can finish is a
    /// mission nobody can finish.
    pub fn navigable_near(&self, p: Vec3, radius: f32, height: f32) -> Option<Vec3> {
        let (cx, cz) = self.flow.nearest_passable(p)?;
        let c = self.flow.cell_center(cx, cz);
        // The grid is flat and the world is not: try the unit's own height first,
        // then a little above and below it, and give up rather than drop it
        // inside a wall.
        for dy in [0.0f32, 0.2, -0.2, 0.5, -0.5, 1.0, -1.0, 2.0, -2.0] {
            let at = vec3(c.x, p.y + dy, c.y);
            if self.point_free(at, radius, height) {
                return Some(at);
            }
        }
        None
    }

    /// Highest surface at or below `ceiling` under the given footprint.
    pub fn support_height(&self, x: f32, z: f32, radius: f32, ceiling: f32) -> Option<f32> {
        let probe = Aabb::new(
            vec3(x - radius, ceiling - 3.0, z - radius),
            vec3(x + radius, ceiling + 0.001, z + radius),
        );
        let mut best: Option<f32> = None;
        for s in &self.solids {
            if s.overlaps(&probe) {
                let top = s.max.y;
                if top <= ceiling + 0.001 && best.is_none_or(|b| top > b) {
                    best = Some(top);
                }
            }
        }
        best
    }

    pub fn ground_below(&self, x: f32, z: f32, radius: f32, from: f32) -> Option<f32> {
        let probe = Aabb::new(
            vec3(x - radius, from - 0.35, z - radius),
            vec3(x + radius, from + 0.05, z + radius),
        );
        let mut best: Option<f32> = None;
        for s in &self.solids {
            if s.overlaps(&probe) && s.max.y <= from + 0.05 && best.is_none_or(|b| s.max.y > b) {
                best = Some(s.max.y);
            }
        }
        best
    }

    /// Distance to the first solid hit, if any.
    pub fn ray_world(&self, ro: Vec3, rd: Vec3, max_dist: f32) -> Option<f32> {
        let mut best = max_dist;
        let mut hit = false;
        for s in &self.solids {
            if let Some(t) = s.ray(ro, rd)
                && t < best
            {
                best = t;
                hit = true;
            }
        }
        if hit { Some(best) } else { None }
    }

    /// The material of the solid a round stopped on, from the point it stopped
    /// at and the direction it was travelling.
    ///
    /// The stop point of a cover hit is *on* the entry face — exactly on the
    /// boundary — so the probe takes one centimetre more along the ray, which is
    /// inside any wall thick enough to have stopped the round and short of the
    /// one behind it. Two boxes that meet are indistinguishable here; either is
    /// the material the round is in.
    pub fn material_behind(&self, at: Vec3, dir: Vec3) -> Option<Surface> {
        let p = at + dir * 0.01;
        self.solids
            .iter()
            .zip(&self.materials)
            .find_map(|(s, m)| s.contains(p).then_some(*m))
    }

    /// The spans of solid cover a ray crosses, nearest first and merged: each is
    /// an entry and an exit along the ray, and two boxes that meet are one wall
    /// to a bullet.
    ///
    /// `ray_world` is the first of these. A round that carries through cover
    /// needs the list, because what a wall costs is its *thickness* — which is
    /// the one thing a single distance cannot say.
    pub fn ray_solids(&self, ro: Vec3, rd: Vec3, max_dist: f32) -> Vec<(f32, f32)> {
        let mut spans: Vec<(f32, f32)> = self
            .solids
            .iter()
            .filter_map(|s| crate::util::ray_aabb_span(ro, rd, s.min, s.max))
            .map(|(enter, exit)| (enter, exit.min(max_dist)))
            .filter(|(enter, exit)| exit > enter)
            .collect();
        spans.sort_by(|a, b| a.0.total_cmp(&b.0));

        let mut merged: Vec<(f32, f32)> = Vec::with_capacity(spans.len());
        for (enter, exit) in spans {
            match merged.last_mut() {
                Some(last) if enter <= last.1 => last.1 = last.1.max(exit),
                _ => merged.push((enter, exit)),
            }
        }
        merged
    }

    pub fn line_of_sight(&self, a: Vec3, b: Vec3) -> bool {
        let d = b - a;
        let dist = d.length();
        if dist < 1e-4 {
            return true;
        }
        let dir = d / dist;
        for s in &self.blockers {
            if let Some(t) = s.ray(a, dir)
                && t > 0.05
                && t < dist - 0.05
            {
                return false;
            }
        }
        for s in &self.volumes {
            if s.life > 0.0 && segment_hits_sphere(a, dir, dist, s.pos, s.current_radius()) {
                return false;
            }
        }
        true
    }

    /// True when `p` is inside a bottomless pit and has fallen below the
    /// floor line.
    pub fn swallowed_by_void(&self, p: Vec3) -> bool {
        p.y < -1.0
            && self
                .voids
                .iter()
                .any(|(min, max)| p.x >= min.x && p.x <= max.x && p.z >= min.y && p.z <= max.y)
    }

    /// Adds a cloud to the arena. `burn` is damage per second inside it, `0.0`
    /// for a smoke screen — see [`Volume`].
    pub fn add_volume(&mut self, pos: Vec3, radius: f32, life: f32, burn: f32) {
        self.volumes.push(Volume {
            pos,
            radius,
            life,
            max_life: life,
            burn,
        });
    }

    pub fn update_volumes(&mut self, dt: f32) {
        for s in &mut self.volumes {
            s.life -= dt;
        }
        self.volumes.retain(|s| s.life > 0.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::maps;

    /// Cells reachable from the player's start, walking the navigation grid.
    fn reachable_from(level: &Level) -> Vec<bool> {
        let (w, h) = (level.flow.w, level.flow.h);
        let mut seen = vec![false; w * h];
        let (cx, cz) = level.flow.cell_of(level.player_start);
        if cx < 0 || cz < 0 || cx as usize >= w || cz as usize >= h {
            return seen;
        }
        let start = cz as usize * w + cx as usize;
        if !level.flow.passable[start] {
            return seen;
        }
        let visit = |nx: usize, nz: usize, seen: &mut Vec<bool>, queue: &mut Vec<usize>| {
            let ni = nz * w + nx;
            if !seen[ni] && level.flow.passable[ni] {
                seen[ni] = true;
                queue.push(ni);
            }
        };
        seen[start] = true;
        let mut queue = vec![start];
        while let Some(i) = queue.pop() {
            let (x, z) = (i % w, i / w);
            if x > 0 {
                visit(x - 1, z, &mut seen, &mut queue);
            }
            if x + 1 < w {
                visit(x + 1, z, &mut seen, &mut queue);
            }
            if z > 0 {
                visit(x, z - 1, &mut seen, &mut queue);
            }
            if z + 1 < h {
                visit(x, z + 1, &mut seen, &mut queue);
            }
        }
        seen
    }

    fn cell_index(level: &Level, p: Vec3, arena: &str) -> usize {
        let (x, z) = level.flow.cell_of(p);
        assert!(
            x >= 0 && z >= 0 && (x as usize) < level.flow.w && (z as usize) < level.flow.h,
            "{arena}: {p:?} is off the navigation grid"
        );
        z as usize * level.flow.w + x as usize
    }

    /// Every arena has to be walkable end to end. The navigation grid decides
    /// what blocks pathing from a height threshold, so raising that threshold —
    /// or adding one wall — can silently seal off a pocket, and the symptom is a
    /// mission that can never be completed because a hostile is stranded.
    #[test]
    fn every_arena_is_fully_navigable() {
        for layout in maps::campaign() {
            let level = Level::from_layout(&layout);
            let arena = layout.name_en;
            let seen = reachable_from(&level);

            let start = cell_index(&level, level.player_start, arena);
            assert!(seen[start], "{arena}: the player start is walled in");
            assert!(
                level.point_free(level.player_start, 0.40, 1.80),
                "{arena}: the player start is inside geometry"
            );

            for (i, spawn) in level.spawns.iter().enumerate() {
                assert!(
                    level.point_free(*spawn, 0.45, 1.80),
                    "{arena}: spawn {i} at {spawn:?} is embedded in geometry"
                );
                let c = cell_index(&level, *spawn, arena);
                assert!(
                    seen[c],
                    "{arena}: spawn {i} at {spawn:?} is unreachable from the player start"
                );
            }

            if let Some(x) = level.extraction {
                let c = cell_index(&level, x, arena);
                assert!(
                    seen[c],
                    "{arena}: the extraction point at {x:?} is unreachable"
                );
            }
        }
    }

    /// Nothing may be routed through a bottomless pit.
    #[test]
    fn voids_are_impassable() {
        for layout in maps::campaign() {
            let level = Level::from_layout(&layout);
            if level.voids.is_empty() {
                continue;
            }
            let seen = reachable_from(&level);
            for (i, (min, max)) in level.voids.iter().enumerate() {
                let centre = vec3((min.x + max.x) * 0.5, 0.0, (min.y + max.y) * 0.5);
                if centre.x <= level.flow.min.x || centre.z <= level.flow.min.y {
                    continue;
                }
                let c = cell_index(&level, centre, layout.name_en);
                assert!(
                    !seen[c],
                    "{}: void {i} is reachable — enemies would walk into the pit",
                    layout.name_en
                );
            }
        }
    }

    /// Every solid carries the material it is drawn with, and a point inside one
    /// answers with that material.
    ///
    /// This is what makes an impact sound surface-aware: the boxes are batched
    /// into one mesh per surface and the block list is dropped, so `materials`
    /// is the only thing left that remembers what a wall is made of (review
    /// R17).
    #[test]
    fn every_solid_remembers_its_material() {
        for layout in maps::campaign() {
            let level = Level::from_layout(&layout);
            assert_eq!(
                level.solids.len(),
                level.materials.len(),
                "{}: the material list is not the solid list",
                layout.name_en
            );
            assert_eq!(
                level.solids.len(),
                layout.boxes.len(),
                "{}: a block did not become a solid",
                layout.name_en
            );
            for (i, block) in layout.boxes.iter().enumerate() {
                assert_eq!(
                    level.materials[i], block.skin.surface,
                    "{}: block {i} is wearing another material",
                    layout.name_en
                );
            }
        }
    }

    /// A round that stops on a wall reports the wall's own material, and a
    /// point in open air reports none.
    #[test]
    fn a_stop_on_a_wall_answers_with_the_wall() {
        let level = Level::from_layout(&maps::arena());
        let at = level.spawns[0];
        // Whatever is under the spawn's own column: the floor it stands on.
        let below = level.ground_below(at.x, at.z, 0.2, at.y).unwrap_or(0.0);
        let hit = vec3(at.x, below, at.z);
        let found = level
            .material_behind(hit, Vec3::NEG_Y)
            .or_else(|| level.material_behind(hit, Vec3::Y));
        assert!(
            found.is_some(),
            "the floor a spawn stands on has no material"
        );

        // Well above the floor and away from anything: nothing to report.
        assert_eq!(level.material_behind(at + Vec3::Y * 40.0, Vec3::Y), None);
    }
}
