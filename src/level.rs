use crate::mesh::MeshBuilder;
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

    pub fn dir_at(&self, p: Vec3) -> Vec2 {
        let (ix, iz) = self.cell_of(p);
        if !self.in_bounds(ix, iz) {
            return Vec2::ZERO;
        }
        self.dir[self.idx(ix as usize, iz as usize)]
    }

    /// Recompute distances from `target` and derive a flow direction per cell.
    pub fn update(&mut self, target: Vec3) {
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

/// Declarative arena description. Campaign missions supply one of these and
/// `Level::from_layout` turns it into collision geometry, a render mesh and a
/// navigation grid.
pub struct Layout {
    pub name_en: &'static str,
    pub name_zh: &'static str,
    pub boxes: Vec<(Vec3, Vec3, Color)>,
    pub player_start: Vec3,
    pub spawns: Vec<Vec3>,
    /// Where the player must reach to finish "extract" objectives.
    pub extraction: Option<Vec3>,
    /// Half-extent of the navigation grid; also the default outer shell size.
    pub half: f32,
    pub sky: Color,
    /// Bottomless pits, as XZ rectangles. Navigation treats them as
    /// impassable, and anything that ends up below the floor line inside one
    /// is gone. They exist because the nav rule ("a solid taller than 1.2 m
    /// blocks") cannot express a hole: any box tall enough to block pathing is
    /// also tall enough to bump into a 1.8 m player.
    pub voids: Vec<(Vec2, Vec2)>,
}

/// A lingering smoke volume. It never affects movement, only sight.
#[derive(Clone, Copy)]
pub struct Smoke {
    pub pos: Vec3,
    pub radius: f32,
    pub life: f32,
    pub max_life: f32,
}

impl Smoke {
    /// Smoke billows as it ages, then thins out at the very end.
    pub fn current_radius(&self) -> f32 {
        let age = 1.0 - (self.life / self.max_life).clamp(0.0, 1.0);
        let grow = (age / 0.25).clamp(0.0, 1.0);
        let fade = (self.life / self.max_life).clamp(0.0, 1.0).powf(0.35);
        self.radius * (0.25 + 0.75 * grow) * fade
    }
}

pub struct Level {
    pub solids: Vec<Aabb>,
    /// Obstacles tall enough to block movement and line of sight.
    pub blockers: Vec<Aabb>,
    pub mesh: Mesh,
    pub flow: FlowField,
    pub spawns: Vec<Vec3>,
    pub player_start: Vec3,
    pub extraction: Option<Vec3>,
    pub sky: Color,
    pub name_en: &'static str,
    pub name_zh: &'static str,
    /// Active smoke screens. Owned here so that `line_of_sight` can consult them
    /// without every caller having to thread a separate smoke list through.
    pub smoke: Vec<Smoke>,
    /// See `Layout::voids`.
    pub voids: Vec<(Vec2, Vec2)>,
}

impl Level {
    pub fn from_layout(layout: &Layout) -> Self {
        let mut mb = MeshBuilder::new();
        let mut solids = Vec::with_capacity(layout.boxes.len());
        let mut blockers = Vec::new();
        for (min, max, color) in &layout.boxes {
            mb.add_box(*min, *max, *color);
            let a = Aabb::new(*min, *max);
            solids.push(a);
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

        Self {
            solids,
            blockers,
            mesh: mb.finish(),
            flow,
            spawns: layout.spawns.clone(),
            player_start: layout.player_start,
            extraction: layout.extraction,
            sky: layout.sky,
            name_en: layout.name_en,
            name_zh: layout.name_zh,
            smoke: Vec::new(),
            voids: layout.voids.clone(),
        }
    }

    pub fn point_free(&self, pos: Vec3, radius: f32, height: f32) -> bool {
        let b = body(pos, radius, height);
        !self.solids.iter().any(|s| s.overlaps(&b))
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
        for s in &self.smoke {
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

    pub fn add_smoke(&mut self, pos: Vec3, radius: f32, life: f32) {
        self.smoke.push(Smoke {
            pos,
            radius,
            life,
            max_life: life,
        });
    }

    pub fn update_smoke(&mut self, dt: f32) {
        for s in &mut self.smoke {
            s.life -= dt;
        }
        self.smoke.retain(|s| s.life > 0.0);
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
}
