//! Scratch-mesh tests, kept out of `mesh.rs` so the allocator probe does not
//! sit in the middle of the geometry helpers.

use crate::effects::Effects;
use crate::enemy::{Enemy, Kind};
use crate::grenade::{GrenadeKind, Grenades};
use crate::level::Level;
use crate::maps;
use crate::mesh::{MeshBuilder, empty_mesh};
use crate::pickup::{Pickup, PickupKind};
use crate::util::Rng;
use macroquad::prelude::*;
use std::alloc::{GlobalAlloc, Layout as AllocLayout, System};
use std::cell::Cell;
use std::collections::HashMap;

// Every allocation made on this thread. `const`-initialised and a plain `Cell`,
// so the allocator below can touch it without allocating and without
// re-entering itself.
thread_local! {
    static ALLOCS: Cell<u64> = const { Cell::new(0) };
}

struct Counting;

// SAFETY: this comment is the one that covers the whole impl and the four
// `System` forwards inside it — they share a single obligation, which is that
// each hands `System` the same `Layout` and pointer it was given, unchanged and
// without touching memory. The counter is a const-initialised thread-local
// `Cell<u64>`, whose `with` neither allocates nor calls back into the
// allocator, so the accounting cannot re-enter itself.
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: AllocLayout) -> *mut u8 {
        ALLOCS.with(|c| c.set(c.get() + 1));
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: AllocLayout) {
        unsafe { System.dealloc(ptr, layout) }
    }

    unsafe fn alloc_zeroed(&self, layout: AllocLayout) -> *mut u8 {
        ALLOCS.with(|c| c.set(c.get() + 1));
        unsafe { System.alloc_zeroed(layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: AllocLayout, new_size: usize) -> *mut u8 {
        ALLOCS.with(|c| c.set(c.get() + 1));
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}

#[global_allocator]
static ALLOCATOR: Counting = Counting;

/// Allocations made by `f` on this thread. The counter is thread-local, so the
/// other tests running in parallel beside this one are invisible to it.
fn allocations<T>(f: impl FnOnce() -> T) -> (T, u64) {
    let before = ALLOCS.with(Cell::get);
    let out = f();
    (out, ALLOCS.with(Cell::get) - before)
}

/// One pass of `render_world`'s mesh work, in the same order and into the same
/// scratch. Returns a checksum, so nothing here can be optimised away and a
/// frame that built nothing cannot pass.
fn build_frame(
    scratch: &mut Mesh,
    enemies: &[Enemy],
    pickups: &[Pickup],
    grenades: &Grenades,
    level: &Level,
    effects: &Effects,
) -> usize {
    let mut verts = 0;
    for e in enemies {
        e.write_mesh(scratch);
        verts += scratch.vertices.len();
    }
    for p in pickups {
        p.write_mesh(scratch);
        verts += scratch.vertices.len();
    }
    grenades.write_mesh(level, 0.0, scratch);
    verts += scratch.vertices.len();
    effects.write_mesh(scratch);
    verts + scratch.vertices.len()
}

fn scene() -> (Vec<Enemy>, Vec<Pickup>, Grenades, Level, Effects) {
    let mut rng = Rng::new(11);
    let enemies = [
        Kind::Grunt,
        Kind::Rusher,
        Kind::Heavy,
        Kind::Sniper,
        Kind::Drone,
        Kind::Tentacle,
        Kind::Colossus,
        Kind::Spawnling,
    ]
    .into_iter()
    .enumerate()
    .map(|(i, kind)| Enemy::spawn(kind, vec3(i as f32 * 4.0, 0.0, 0.0), 3, &mut rng))
    .collect();

    let pickups = [PickupKind::Health, PickupKind::Ammo, PickupKind::Armor]
        .into_iter()
        .enumerate()
        .map(|(i, kind)| Pickup::new(vec3(i as f32 * 2.0, 0.6, 6.0), kind, 20.0))
        .collect();

    let mut effects = Effects::new();
    effects.spark(
        vec3(0.0, 1.0, 0.0),
        Vec3::Y,
        Color::new(1.0, 0.8, 0.4, 1.0),
        40,
        &mut rng,
    );
    effects.blood(
        vec3(1.0, 1.0, 0.0),
        Vec3::X,
        Color::new(0.7, 0.1, 0.1, 1.0),
        &mut rng,
    );
    effects.smoke(
        vec3(2.0, 1.0, 0.0),
        Color::new(0.5, 0.5, 0.5, 0.8),
        30,
        &mut rng,
    );
    effects.tracer(
        vec3(0.0, 1.0, 0.0),
        vec3(12.0, 1.4, 3.0),
        Color::new(1.0, 0.9, 0.5, 1.0),
        0.02,
    );

    let mut grenades = Grenades::new();
    grenades.throw(GrenadeKind::Frag, vec3(0.0, 1.6, 0.0), vec3(0.0, 0.35, 1.0));

    let mut level = Level::from_layout(&maps::campaign()[0]);
    level.add_volume(vec3(5.0, 1.2, 0.0), 6.0, 15.0, 0.0);

    (enemies, pickups, grenades, level, effects)
}

/// Tiling has to be a property of the world, not of the box that carries it.
/// A 20 m wall and a 1 m crate have to come out at the same texels per metre —
/// otherwise the wall is one enormous stretched tile, which is the failure the
/// whole UV pass exists to prevent and is invisible until you stand next to
/// both.
///
/// The span is read off all four corners rather than from `a` to `b`. Which
/// world axis a face's `u` runs along is the builder's business, and this test
/// used to pin the old per-face convention — the face's own first corner as the
/// UV origin, `u` along its first edge — which is exactly the phase bug it
/// could not see.
#[test]
fn tiling_does_not_stretch_with_the_size_of_the_box() {
    const SCALE: f32 = 0.25; // one tile per four metres
    let mut mb = MeshBuilder::new();
    mb.set_uv_scale(SCALE);
    mb.add_box(Vec3::ZERO, vec3(20.0, 4.0, 20.0), WHITE);
    mb.add_box(Vec3::ZERO, vec3(1.0, 1.0, 1.0), WHITE);
    let mesh = mb.finish();

    // The first quad of a box is its top face; the extent of its UVs is how much
    // texture it covers, whichever way round the two axes came out.
    let span = |first: usize| {
        let uv: Vec<Vec2> = (first..first + 4).map(|i| mesh.vertices[i].uv).collect();
        uv[1..]
            .iter()
            .fold((uv[0], uv[0]), |(lo, hi), t| (lo.min(*t), hi.max(*t)))
    };
    let (lo, hi) = span(0);
    let big = hi - lo;
    let (lo, hi) = span(24);
    let small = hi - lo;

    assert!(
        (big.x - 20.0 * SCALE).abs() < 1e-4 && (big.y - 20.0 * SCALE).abs() < 1e-4,
        "a 20 m face spans {big} of a tile, not {}",
        20.0 * SCALE,
    );
    assert!(
        (small.x - SCALE).abs() < 1e-4 && (small.y - SCALE).abs() < 1e-4,
        "a 1 m face spans {small} of a tile, not {SCALE}",
    );
}

/// Two panels of one floor are two boxes, and the seam between them is invisible
/// only while both name the same texel at the same world point. Measured from
/// each face's own first corner — what `quad` used to do — the second panel
/// restarts the pattern at its own corner, so its half of the floor is out of
/// phase by however far that corner is from the first panel's.
#[test]
fn adjacent_floor_slabs_meet_in_phase() {
    const SCALE: f32 = 0.25;
    let mut mb = MeshBuilder::new();
    mb.set_uv_scale(SCALE);
    // A floor in two panels, sharing the plane `x = 2`.
    mb.add_box(vec3(0.0, 0.0, 0.0), vec3(2.0, 1.0, 2.0), WHITE);
    mb.add_box(vec3(2.0, 0.0, 0.0), vec3(4.0, 1.0, 2.0), WHITE);
    let mesh = mb.finish();

    // Their top faces: the left one closes on `(2, 1, 2)` and `(2, 1, 0)` — its
    // `c` and `d` — and the right one opens on those same two points as its `b`
    // and `a`. The two faces are one plane with one normal, so the UVs are not
    // merely near each other; they are the same bits.
    let (left, right) = (&mesh.vertices[0..4], &mesh.vertices[24..28]);
    assert_eq!(
        left[2].position, right[1].position,
        "the test's own geometry does not share the corner (2, 1, 2)",
    );
    assert_eq!(left[2].uv, right[1].uv, "the seam steps at (2, 1, 2)");
    assert_eq!(left[3].uv, right[0].uv, "the seam steps at (2, 1, 0)");

    // And the phase carries across the join rather than restarting: from the
    // outer corner of one panel to the outer corner of the other is four metres
    // of floor, so four metres of texture. A second origin at the second panel
    // would leave two.
    let across = (right[3].uv - left[0].uv).length();
    assert!(
        (across - 4.0 * SCALE).abs() < 1e-4,
        "the pattern runs {across} tiles across the seam, not {}",
        4.0 * SCALE,
    );
}

/// The other half of the UV contract: `uv_scale` is repeats per metre, so a
/// metre of face has to be a metre of texture whichever way the metre runs. The
/// edges of a unit box are one metre each, and the six faces between them cover
/// all three world axes.
#[test]
fn one_metre_of_face_is_one_metre_of_texture() {
    const SCALE: f32 = 0.25;
    let mut mb = MeshBuilder::new();
    mb.set_uv_scale(SCALE);
    mb.add_box(Vec3::ZERO, Vec3::ONE, WHITE);
    let mesh = mb.finish();

    let mut seen = [false; 3];
    for face in 0..6 {
        let v: [&Vertex; 4] = std::array::from_fn(|i| &mesh.vertices[face * 4 + i]);
        // The two edges out of `a`, each one metre long on a unit box.
        for (i, j) in [(0, 1), (0, 3)] {
            let step = v[j].position - v[i].position;
            let axis = if step.x != 0.0 {
                0
            } else if step.y != 0.0 {
                1
            } else if step.z != 0.0 {
                2
            } else {
                panic!(
                    "face {face} runs from {} to {} without moving",
                    v[i].position, v[j].position
                )
            };
            seen[axis] = true;
            let moved = (v[j].uv - v[i].uv).length();
            assert!(
                (moved - SCALE).abs() < 1e-4,
                "a metre of face {face} along {} moved the UV by {moved}, not {SCALE}",
                ["x", "y", "z"][axis],
            );
        }
        // The diagonal lies in the face too, so it comes out at the same density
        // — the check that the UV axes are square to each other and the texture
        // is sheared nowhere inside a face.
        let diagonal = (v[2].uv - v[0].uv).length();
        assert!(
            (diagonal - SCALE * 2.0f32.sqrt()).abs() < 1e-4,
            "the diagonal of face {face} covers {diagonal} of a tile, not {}",
            SCALE * 2.0f32.sqrt(),
        );
    }
    assert!(
        seen == [true; 3],
        "the box's edges did not cover all three world axes: {seen:?}",
    );
}

/// A beam's faces are the one shape here that is not square to the world, and
/// the density has to survive them: the two axes are still orthonormal inside
/// each face, so an edge of any length is covered by the same texels per metre
/// as a wall of the level is — however the beam is aimed.
#[test]
fn slanted_geometry_tiles_at_the_same_density() {
    const SCALE: f32 = 0.5;
    let mut mb = MeshBuilder::new();
    mb.set_uv_scale(SCALE);
    // Aimed at an angle no world axis lines up with, and framed by its own
    // cross-section rather than by any plane of the level.
    mb.add_beam(vec3(-3.0, 1.5, 2.0), vec3(4.5, 0.25, -1.0), 0.3, WHITE);
    let mesh = mb.finish();
    assert_eq!(mesh.vertices.len(), 6 * 4, "a beam is six quads");

    for quad in 0..6 {
        let v: [&Vertex; 4] = std::array::from_fn(|i| &mesh.vertices[quad * 4 + i]);
        for (i, j) in [(0, 1), (1, 2), (2, 3), (3, 0)] {
            let metres = (v[j].position - v[i].position).length();
            let covered = (v[j].uv - v[i].uv).length();
            assert!(
                (covered - SCALE * metres).abs() < 1e-3,
                "an edge of beam face {quad} is {metres} m long and covers \
                 {covered} of a tile, not {}",
                SCALE * metres,
            );
        }
    }
}

/// Faces no plane can be built from still have to come out with usable UVs: a
/// box with no size, corners that are collinear before the fourth one, a beam
/// with no radius, and a box far enough out that its coordinates are large. The
/// failure being guarded against is a NaN, which would spread through the vertex
/// buffer and take the whole triangle with it.
#[test]
fn degenerate_faces_still_get_finite_uvs() {
    const SCALE: f32 = 0.5;
    let mut mb = MeshBuilder::new();
    mb.set_uv_scale(SCALE);

    // A box with no size: every cross product is zero, so no face has a normal.
    mb.add_box(vec3(3.0, -2.0, 0.5), vec3(3.0, -2.0, 0.5), WHITE);
    // The first three corners on one line — the normal is zero although the quad
    // still has area, which is the case `shade` also sees as flat.
    mb.quad(
        vec3(0.0, 0.0, 0.0),
        vec3(1.0, 0.0, 0.0),
        vec3(2.0, 0.0, 0.0),
        vec3(0.0, 0.0, 4.0),
        WHITE,
    );
    // A beam with no radius: four collapsed sides and two collapsed caps.
    mb.add_beam(vec3(1.0, 2.0, 3.0), vec3(-4.0, 0.5, 2.0), 0.0, WHITE);
    // Large, and thin enough that every cross product is small.
    mb.add_box(Vec3::splat(-1.0e6), Vec3::splat(1.0e6), WHITE);
    mb.add_box(Vec3::ZERO, vec3(5.0, 1.0e-6, 5.0), WHITE);

    let mesh = mb.finish();
    assert!(
        !mesh.vertices.is_empty(),
        "the degenerate cases built nothing",
    );
    for v in &mesh.vertices {
        assert!(
            v.uv.x.is_finite() && v.uv.y.is_finite(),
            "the face at {} came out with the UV {}",
            v.position,
            v.uv,
        );
    }

    // A face with no plane falls back to a fixed pair of world axes — `x` and
    // `z` — rather than to whatever the failed projection left behind.
    let corner = vec3(3.0, -2.0, 0.5);
    assert_eq!(
        mesh.vertices[0].uv,
        vec2(corner.x, corner.z) * SCALE,
        "the fallback for a face with no plane changed",
    );
}

/// The UV a quad carries at `p`, which has to be one of its four corners.
fn uv_at(mesh: &Mesh, first: usize, p: Vec3) -> Vec2 {
    for v in &mesh.vertices[first..first + 4] {
        if v.position == p {
            return v.uv;
        }
    }
    panic!("the quad at vertex {first} has no corner at {p}");
}

/// How far a face's UVs move per metre of rise, read off the edge of it that
/// runs straight up. A face with no vertical edge answers zero, which no caller
/// here passes.
fn rise_per_metre(mesh: &Mesh, first: usize) -> Vec2 {
    let v: [&Vertex; 4] = std::array::from_fn(|i| &mesh.vertices[first + i]);
    for (i, j) in [(0, 1), (0, 3), (1, 2), (2, 3)] {
        let d = v[j].position - v[i].position;
        if d.x == 0.0 && d.z == 0.0 && d.y != 0.0 {
            return (v[j].uv - v[i].uv) / d.y;
        }
    }
    Vec2::ZERO
}

/// Stacking and corners are where a private phase shows up in play: a crate lid
/// above the floor it covers, and two walls meeting at a corner at different
/// heights. Neither pair shares a face or even a vertex with the other, only a
/// world point — so a per-face origin gets both wrong, and a world-anchored one
/// cannot.
#[test]
fn stacked_tops_and_cornered_walls_hold_one_phase() {
    const SCALE: f32 = 0.25;
    let mut mb = MeshBuilder::new();
    mb.set_uv_scale(SCALE);
    // A slab, and a step beside it one metre higher. Their lids share the plan
    // view corners `(4, 0)` and `(4, 4)` while sitting at different heights.
    mb.add_box(vec3(0.0, 0.0, 0.0), vec3(4.0, 1.0, 4.0), WHITE);
    mb.add_box(vec3(4.0, 0.0, 0.0), vec3(8.0, 2.0, 4.0), WHITE);
    // Two walls meeting along `x = 1, z = 4`: the first rises from the floor,
    // the second stands on its top edge three metres up.
    mb.add_box(vec3(0.0, 0.0, 0.0), vec3(1.0, 3.0, 4.0), WHITE);
    mb.add_box(vec3(1.0, 3.0, 4.0), vec3(5.0, 6.0, 5.0), WHITE);
    let mesh = mb.finish();

    // The two lids. A top face is the first quad of its box, and both are wound
    // from `(x0, y1, z0)`, so the same tile shows at the same `(x, z)` whatever
    // the height — which is what keeps a crate's pattern from shifting when a
    // second crate is stacked on it.
    for (low, high) in [
        (vec3(4.0, 1.0, 0.0), vec3(4.0, 2.0, 0.0)),
        (vec3(4.0, 1.0, 4.0), vec3(4.0, 2.0, 4.0)),
    ] {
        assert_eq!(
            uv_at(&mesh, 0, low),
            uv_at(&mesh, 24, high),
            "the two lids disagree above {low} and {high}",
        );
    }

    // The two walls. `add_box` lays a box out top, bottom, `+z`, `-z`, `+x`,
    // `-x`, so the third box's `+x` wall opens at vertex 48 + 16 and the fourth
    // box's `-z` wall at 72 + 12.
    let wall_a = 48 + 16; // the +x face of `(0, 0, 0)-(1, 3, 4)`
    let wall_b = 72 + 12; // the -z face of `(1, 3, 4)-(5, 6, 5)`
    let (rise_a, rise_b) = (rise_per_metre(&mesh, wall_a), rise_per_metre(&mesh, wall_b));
    assert!(
        (rise_a - rise_b).length() < 1e-4,
        "the two walls measure height differently: {rise_a} against {rise_b}",
    );

    // The walls run in different directions, so their horizontal coordinates are
    // different measurements and are not expected to agree. Height is the same
    // world axis on both, and it is the coordinate that has to carry on around
    // the corner: it does when the offset between the two faces there never
    // moves up or down.
    let (corner_a, corner_b) = (
        uv_at(&mesh, wall_a, vec3(1.0, 3.0, 4.0)),
        uv_at(&mesh, wall_b, vec3(1.0, 3.0, 4.0)),
    );
    let step = (corner_a - corner_b).dot(rise_a.normalize_or_zero()).abs();
    assert!(
        step < 1e-4,
        "the corner steps {step} of a tile vertically: {corner_a} against {corner_b}",
    );
}

/// The real level, not two toy boxes: wherever two of its faces put a corner at
/// the same world point with the same normal, they have to name the same texel.
/// The campaign is where the phase a hand-picked pair can miss actually lives —
/// three sizes of slab in a row, a rail against the wall it trims — and because
/// every block is an AABB the two UVs are bit-identical, not merely close.
#[test]
fn every_level_face_samples_one_uv_field() {
    let mut corners = 0usize;
    for layout in maps::campaign() {
        let level = Level::from_layout(&layout);
        // Per mesh, because each surface carries its own tile density.
        for mesh in &level.meshes {
            let mut seen: HashMap<([u32; 3], [u32; 4]), (Vec3, Vec2)> = HashMap::new();
            for v in &mesh.vertices {
                let key = (
                    v.position.to_array().map(f32::to_bits),
                    v.normal.to_array().map(f32::to_bits),
                );
                match seen.insert(key, (v.position, v.uv)) {
                    Some((at, uv)) => assert!(
                        uv == v.uv,
                        "in {}, two faces meet at {at} out of phase: {uv} against {}",
                        layout.name_en,
                        v.uv,
                    ),
                    None => corners += 1,
                }
            }
        }
    }
    assert!(
        corners > 2000,
        "the campaign barely has two faces meeting anywhere: {corners} corners",
    );
}

/// The per-frame meshes have to be rebuilt each frame; the buffers must not be.
/// `render_world` keeps one scratch mesh for all of them, so once the scene has
/// been drawn once, drawing it again may not allocate at all — an enemy, a
/// pickup, the grenade batch and the particle batch every frame were four
/// hundred-odd allocations a second and none of them bought anything.
#[test]
fn a_rebuilt_frame_allocates_nothing() {
    let (enemies, pickups, grenades, level, effects) = scene();
    let mut scratch = empty_mesh();

    let first = build_frame(
        &mut scratch,
        &enemies,
        &pickups,
        &grenades,
        &level,
        &effects,
    );
    assert!(first > 0, "the frame built no geometry at all");

    let (second, allocs) = allocations(|| {
        build_frame(
            &mut scratch,
            &enemies,
            &pickups,
            &grenades,
            &level,
            &effects,
        )
    });

    // A scratch that was not cleared first would still be holding the previous
    // write, and every count after the first would come out too large.
    assert_eq!(second, first, "the same scene built a different frame");
    assert_eq!(allocs, 0, "a repeated frame made {allocs} allocations");
}

/// Bit-exact signature of a mesh vertex, for comparing two builds of the same
/// thing. Floats go through `to_bits` because the comparison has to be exact:
/// "close enough" would let a stale vertex through.
fn vertex_key(v: &Vertex) -> ([u32; 3], [u32; 2], [u8; 4], [u32; 4]) {
    (
        v.position.to_array().map(f32::to_bits),
        v.uv.to_array().map(f32::to_bits),
        v.color,
        v.normal.to_array().map(f32::to_bits),
    )
}

/// Where two meshes first disagree, in enough words to point at it, or `None`
/// when they are identical. Comparing the vectors directly would print
/// thousands of vertices on failure.
fn first_difference(a: &Mesh, b: &Mesh) -> Option<String> {
    if a.vertices.len() != b.vertices.len() {
        return Some(format!(
            "{} vertices against {}",
            a.vertices.len(),
            b.vertices.len()
        ));
    }
    let keyed: Vec<_> = a.vertices.iter().map(vertex_key).collect();
    let theirs: Vec<_> = b.vertices.iter().map(vertex_key).collect();
    if let Some(i) = keyed.iter().zip(&theirs).position(|(x, y)| x != y) {
        return Some(format!("vertex {i} differs"));
    }
    if a.indices != b.indices {
        let count = a.indices.len().min(b.indices.len());
        return Some(match (0..count).find(|&i| a.indices[i] != b.indices[i]) {
            Some(i) => format!("index {i} differs"),
            None => format!("{} indices against {}", a.indices.len(), b.indices.len()),
        });
    }
    None
}

/// Writing into a scratch that is already full of something else must produce
/// exactly what a fresh mesh would. Reuse is only correct if the previous
/// contents are gone, and a vertex count on its own would not notice a stale
/// vertex left behind — every field has to match, bit for bit.
#[test]
fn a_reused_scratch_matches_a_fresh_mesh() {
    let mut rng = Rng::new(3);
    let mut reused = empty_mesh();

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
    // The branches that matter: no glow at all, mid-flash, alerted, and far
    // enough into the death animation to be fading and sinking.
    let states = [
        (0.0f32, 0.0f32, false, 0.0f32, 0.0f32),
        (0.9, 0.0, false, 0.0, 1.2),
        (0.0, 0.8, false, 0.3, 2.5),
        (0.0, 0.0, true, 1.6, 4.0),
    ];

    for kind in kinds {
        for (flash, alert, dead, death_t, spawned_t) in states {
            let mut enemy = Enemy::spawn(kind, vec3(1.5, 0.0, -2.0), 4, &mut rng);
            enemy.flash = flash;
            enemy.alert = alert;
            enemy.dead = dead;
            enemy.death_t = death_t;
            enemy.spawned_t = spawned_t;

            enemy.write_mesh(&mut reused);
            let mut fresh = empty_mesh();
            enemy.write_mesh(&mut fresh);
            assert_eq!(
                first_difference(&reused, &fresh),
                None,
                "{kind:?} at flash {flash}, alert {alert}, dead {dead} after {death_t}s \
                 (spawned {spawned_t}s)",
            );
            assert!(
                !reused.indices.is_empty(),
                "{kind:?}: wrote no geometry at all"
            );
        }
    }

    for kind in [PickupKind::Health, PickupKind::Ammo, PickupKind::Armor] {
        let mut pickup = Pickup::new(vec3(0.0, 0.4, 0.0), kind, 20.0);
        pickup.update(0.37);
        pickup.write_mesh(&mut reused);
        let mut fresh = empty_mesh();
        pickup.write_mesh(&mut fresh);
        assert_eq!(first_difference(&reused, &fresh), None, "{kind:?} pickup");
    }
}

/// `reclaim` hands a mesh's buffers to the builder and `finish` hands them
/// back; the pair has to round-trip the capacity, which is the whole reason it
/// exists.
#[test]
fn reclaim_keeps_the_buffers_and_clears_the_mesh() {
    let mut builder = MeshBuilder::new();
    builder.add_box(vec3(-1.0, -1.0, -1.0), vec3(1.0, 1.0, 1.0), WHITE);
    let mut mesh = builder.finish();
    let cap = (mesh.vertices.capacity(), mesh.indices.capacity());
    assert!(cap.0 > 0 && cap.1 > 0, "the first build did not allocate");

    let mut builder = MeshBuilder::reclaim(&mut mesh);
    assert!(
        mesh.vertices.is_empty() && mesh.indices.is_empty(),
        "reclaim left the mesh holding geometry it does not own any more",
    );
    builder.add_box(vec3(-1.0, -1.0, -1.0), vec3(1.0, 1.0, 1.0), WHITE);
    mesh = builder.finish();

    assert_eq!(
        (mesh.vertices.len(), mesh.indices.len()),
        (24, 36),
        "one box is 6 quads",
    );
    assert_eq!(
        (mesh.vertices.capacity(), mesh.indices.capacity()),
        cap,
        "the round trip reallocated",
    );
}
