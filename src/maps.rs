//! Campaign arena layouts. All geometry is axis-aligned boxes so that collision,
//! navigation and rendering all derive from one source of truth.
//!
//! Every measurement in the doc comments below comes from
//! `tests::show_the_design_report`, which re-measures the real geometry rather
//! than restating an intention: `cargo test -- --ignored --nocapture`. The
//! method is fixed there and is worth knowing when reading the numbers.
//!
//! * Distances are metres. The project's unit *is* the metre — a standing body
//!   is 1.80 m tall, steps over 0.62 m and has its eye at 1.64 m — so the
//!   level-design reference's Hammer measurements convert at 1 unit =
//!   0.01905 m: 256 u (close combat) = 4.9 m, 1024 u (mid) = 19.5 m, 2048 u
//!   (sniper and rocket) = 39 m.
//! * Sightlines are traced at 1.6 m through `Level::line_of_sight`, the same
//!   test the game's AI and its spawn chooser use. That is the plane a fight
//!   happens on, and it sits under the 1.75 m at which cover stops a standing
//!   player's line of sight.
//! * Positions are sampled on a 2 m lattice, so a "longest sightline" is a
//!   lower bound good to a couple of metres.
//! * Cover bands are the reference's, re-anchored to this project's eye
//!   heights: low 1.0-1.25 m (shoot over standing, hide crouched), full 1.75 m+,
//!   everything under 0.62 m is stepped over and counts as nothing.
//! * The last measurement worth knowing before reading any arena: **every
//!   perch in the campaign is player-only ground.** A platform is a solid, and
//!   the navigation grid blocks any cell whose footprint touches one, so the
//!   highest ground the AI can path onto anywhere is 0.60 m. The yard's plinth,
//!   the citadel's keep, the ziggurat and the catwalks are all unclimbable to
//!   anything the flow field steers. `elevation_is_player_only_in_every_arena`
//!   is what holds that down.

use crate::level::{ARENA, Block, Layout, WALL_H};
use crate::surface::{Skin, Surface, skin};
use crate::util::Rng;
use macroquad::prelude::*;

/// RUST YARD — mission 1, the teaching arena.
///
/// * **Plan** — one 52 x 52 m room with a 14 x 14 m plinth in the middle, eight
///   pillars round it, and walls closing the corners into rooms. Closest
///   reference type is *no man's land*: an open middle crossed under fire with
///   the cover pushed to the rim. It is not a figure 8 (the two loops never
///   close) and not three lanes (one ring, no parallel routes).
/// * **Sightlines** — 62.5 m corner to corner; the *shortest* deepest-visible
///   point anywhere is 22.8 m, so no standing position in the yard is out of
///   somebody's line. Typical unobstructed ray 11.0 m, 23% of rays under 4.9 m
///   and 5% over 39 m.
/// * **Cover** — 6 low pieces (four 1.2 m walls, two 1.1 m crates) against 25
///   full-height ones. The only arena in the campaign that teaches the 1.0-1.25 m
///   band, which is what mission 1 is for; its 1.2 m walls are also the only
///   place in the campaign where a crouched player can shoot over cover.
pub fn arena() -> Layout {
    let mut boxes: Vec<Block> = Vec::new();
    let mut rng = Rng::new(0x5EED_1234);

    let floor = skin(0.15, 0.16, 0.19, Surface::Concrete);
    let wall = skin(0.25, 0.25, 0.29, Surface::Corrugated);
    let platform = skin(0.29, 0.29, 0.35, Surface::Tiles);
    let pillar = skin(0.27, 0.27, 0.32, Surface::Tiles);
    let cover = skin(0.23, 0.25, 0.28, Surface::MetalRusty);
    let crate_col = skin(0.34, 0.30, 0.19, Surface::Crate);
    let accent = skin(0.74, 0.44, 0.12, Surface::Corrugated);
    let rail = skin(0.35, 0.38, 0.44, Surface::Corrugated);

    // Slight per-panel variation keeps the large flat surfaces from reading
    // as one solid colour.
    let mut vary = |c: Skin| c.vary(0.94 + rng.f32() * 0.12);

    let mut b = |x0: f32, y0: f32, z0: f32, x1: f32, y1: f32, z1: f32, s: Skin| {
        boxes.push(Block {
            min: vec3(x0, y0, z0),
            max: vec3(x1, y1, z1),
            skin: s,
        });
    };

    // --- ground & outer shell ---
    b(-ARENA, -1.0, -ARENA, ARENA, 0.0, ARENA, floor);
    b(-ARENA, 0.0, 25.0, ARENA, WALL_H, 26.0, wall);
    b(-ARENA, 0.0, -26.0, ARENA, WALL_H, -25.0, wall);
    b(-26.0, 0.0, -26.0, -25.0, WALL_H, 26.0, wall);
    b(25.0, 0.0, -26.0, 26.0, WALL_H, 26.0, wall);
    // Wall trim
    b(-ARENA, 0.0, 24.85, ARENA, 0.35, 25.0, accent);
    b(-ARENA, 0.0, -25.0, ARENA, 0.35, -24.85, accent);
    b(-25.0, 0.0, -ARENA, -24.85, 0.35, ARENA, accent);
    b(24.85, 0.0, -ARENA, 25.0, 0.35, ARENA, accent);

    // --- central raised platform ---
    b(-7.0, 0.0, -7.0, 7.0, 1.6, 7.0, platform);
    b(-7.1, 1.45, -7.1, 7.1, 1.6, 7.1, rail);
    b(-1.3, 1.6, -1.3, 1.3, 4.4, 1.3, vary(pillar));
    b(-1.4, 4.4, -1.4, 1.4, 4.7, 1.4, accent);

    // Access crates: 0 -> 0.55 -> 1.1 -> 1.6
    b(7.0, 0.0, -2.0, 8.6, 1.1, 0.6, vary(crate_col));
    b(8.6, 0.0, -2.0, 10.2, 0.55, 0.6, vary(crate_col));
    b(-8.6, 0.0, -2.0, -7.0, 1.1, 0.6, vary(crate_col));
    b(-10.2, 0.0, -2.0, -8.6, 0.55, 0.6, vary(crate_col));
    b(-2.0, 0.0, 7.0, 0.6, 1.1, 8.6, vary(crate_col));
    b(-2.0, 0.0, 8.6, 0.6, 0.55, 10.2, vary(crate_col));

    // --- pillars ---
    for (sx, sz) in [(13.0, 13.0), (13.0, -13.0), (-13.0, 13.0), (-13.0, -13.0)] {
        b(
            sx - 1.1,
            0.0,
            sz - 1.1,
            sx + 1.1,
            4.6,
            sz + 1.1,
            vary(pillar),
        );
        b(sx - 1.3, 0.0, sz - 1.3, sx + 1.3, 0.45, sz + 1.3, rail);
    }
    for (sx, sz) in [(0.0, 17.0), (0.0, -17.0), (17.0, 0.0), (-17.0, 0.0)] {
        b(
            sx - 0.9,
            0.0,
            sz - 0.9,
            sx + 0.9,
            4.0,
            sz + 0.9,
            vary(pillar),
        );
        b(sx - 1.1, 0.0, sz - 1.1, sx + 1.1, 0.45, sz + 1.1, rail);
    }

    // --- mid-height sightline breaks ---
    b(-18.0, 0.0, 8.0, -8.0, 2.4, 9.0, vary(cover));
    b(8.0, 0.0, -9.0, 18.0, 2.4, -8.0, vary(cover));
    b(-9.0, 0.0, -18.0, -8.0, 2.4, -8.0, vary(cover));
    b(8.0, 0.0, 8.0, 9.0, 2.4, 18.0, vary(cover));

    // --- low cover, shootable over ---
    b(-18.0, 0.0, -4.0, -12.0, 1.2, -3.0, vary(cover));
    b(12.0, 0.0, 3.0, 18.0, 1.2, 4.0, vary(cover));
    b(-4.0, 0.0, 14.0, 4.0, 1.2, 15.0, vary(cover));
    b(-4.0, 0.0, -15.0, 4.0, 1.2, -14.0, vary(cover));
    b(-14.0, 0.0, -4.0, -13.0, 2.0, 4.0, vary(cover));
    b(13.0, 0.0, -4.0, 14.0, 2.0, 4.0, vary(cover));

    // --- crates ---
    b(-20.0, 0.0, 18.0, -18.4, 1.1, 19.6, vary(crate_col));
    b(-18.4, 0.0, 18.0, -16.8, 0.55, 19.6, vary(crate_col));
    b(16.0, 0.0, 18.0, 18.0, 1.1, 20.0, vary(crate_col));
    b(18.0, 0.0, -20.0, 19.6, 0.55, -18.4, vary(crate_col));
    b(20.0, 0.0, -8.0, 21.5, 0.55, -6.5, vary(crate_col));
    b(-21.0, 0.0, -14.0, -19.5, 0.55, -12.5, vary(crate_col));
    b(-14.0, 0.0, 20.0, -12.5, 0.55, 21.5, vary(crate_col));
    b(20.0, 0.0, 12.0, 21.6, 1.1, 13.6, vary(crate_col));

    // --- corner rooms ---
    b(-25.0, 0.0, 14.0, -20.0, 3.0, 15.0, vary(cover));
    b(-20.0, 0.0, 20.0, -19.0, 3.0, 25.0, vary(cover));
    b(20.0, 0.0, -25.0, 25.0, 3.0, -24.0, vary(cover));
    b(20.0, 0.0, -20.0, 21.0, 3.0, -19.0, vary(cover));
    b(-25.0, 0.0, -15.0, -20.0, 2.6, -14.0, vary(cover));
    b(14.0, 0.0, 20.0, 20.0, 2.6, 21.0, vary(cover));

    let spawns = [
        (-22.0, -22.0),
        (22.0, -22.0),
        (-22.0, 22.0),
        (22.0, 22.0),
        (0.0, -22.0),
        (0.0, 22.0),
        (-22.0, 0.0),
        (22.0, 0.0),
        (-13.0, -18.0),
        (13.0, 18.0),
        (-18.0, 13.0),
        (18.0, -13.0),
    ]
    .iter()
    .map(|(x, z)| vec3(*x, 0.0, *z))
    .collect();

    Layout {
        name_en: "RUST YARD",
        name_zh: "废土竞技场",
        boxes,
        voids: Vec::new(),
        player_start: vec3(0.0, 0.5, 21.0),
        spawns,
        extraction: None,
        half: ARENA,
        sky: Color::new(0.05, 0.06, 0.09, 1.0),
        accent: Color::new(0.88, 0.52, 0.16, 1.0),
    }
}

/// FOUNDRY — mission 2, two long machine-hall lanes either side of a central
/// hall.
///
/// * **Plan** — *three lane*, the reference's type by name: pipe walls at
///   |x| = 17-18 run the whole length as the outer lane walls, machine banks at
///   |x| = 9-13 flank an 18 m central hall, and the 3 m walkways between the
///   banks are the connections. Cover is architecture, not furniture.
/// * **Sightlines** — 46.8 m down the central hall, and none of the shorter
///   ones matter: the typical unobstructed ray is 5.4 m, 46% are under 4.9 m
///   and only 2% reach 39 m. This is the campaign's second-tightest arena, and
///   its 46.8 m line is the one that makes the hall worth crossing.
/// * **Cover** — 26 full-height pieces (machine banks 2.4-2.6 m, pipe walls
///   3.2 m, catwalk legs 2.6-3.4 m) and *no* low cover at all. Deliberate: the
///   lanes are walls, and half-height pieces in a 3 m walkway would do nothing
///   but narrow it. Nor is there any 1.0-1.75 m cover — in this arena you
///   either break contact behind a machine or stand in the open.
pub fn foundry() -> Layout {
    let mut boxes: Vec<Block> = Vec::new();
    let mut rng = Rng::new(0xF0_11_D2_0A);

    let floor = skin(0.16, 0.13, 0.12, Surface::MetalPlate);
    let wall = skin(0.30, 0.20, 0.16, Surface::MetalRusty);
    let machine = skin(0.36, 0.27, 0.21, Surface::Tiles);
    let pipe = skin(0.30, 0.24, 0.22, Surface::MetalRusty);
    let crate_col = skin(0.34, 0.30, 0.17, Surface::Crate);
    let catwalk = skin(0.38, 0.30, 0.26, Surface::Corrugated);
    let rail = skin(0.44, 0.36, 0.30, Surface::Corrugated);
    let ember = skin(0.60, 0.35, 0.10, Surface::Corrugated);

    let mut vary = |c: Skin| c.vary(0.94 + rng.f32() * 0.12);

    let mut b = |x0: f32, y0: f32, z0: f32, x1: f32, y1: f32, z1: f32, s: Skin| {
        boxes.push(Block {
            min: vec3(x0, y0, z0),
            max: vec3(x1, y1, z1),
            skin: s,
        });
    };

    // --- ground & outer shell ---
    b(-24.0, -1.0, -24.0, 24.0, 0.0, 24.0, floor);
    b(-24.0, 0.0, 23.0, 24.0, WALL_H, 24.0, wall);
    b(-24.0, 0.0, -24.0, 24.0, WALL_H, -23.0, wall);
    b(-24.0, 0.0, -23.0, -23.0, WALL_H, 23.0, wall);
    b(23.0, 0.0, -23.0, 24.0, WALL_H, 23.0, wall);
    // Wall trim
    b(-24.0, 0.0, 22.85, 24.0, 0.35, 23.0, ember);
    b(-24.0, 0.0, -23.0, 24.0, 0.35, -22.85, ember);
    b(-23.0, 0.0, -24.0, -22.85, 0.35, 24.0, ember);
    b(22.85, 0.0, -24.0, 23.0, 0.35, 24.0, ember);

    // --- machine banks flanking the central hall (3 m walkways between) ---
    let bank = [
        (-22.0, -16.0),
        (-13.0, -7.0),
        (-4.0, 4.0),
        (7.0, 13.0),
        (16.0, 22.0),
    ];
    for (x0, x1) in [(9.0, 13.0), (-13.0, -9.0)] {
        for (z0, z1) in bank {
            b(x0, 0.0, z0, x1, 2.6, z1, vary(machine));
        }
    }

    // --- pipe walls separating the service lanes (3 m openings) ---
    for x0 in [17.0, -18.0] {
        for (z0, z1) in [(-22.0, -13.0), (-10.0, -1.0), (2.0, 11.0), (14.0, 22.0)] {
            b(x0, 0.0, z0, x0 + 1.0, 3.2, z1, vary(pipe));
        }
    }

    // --- furnace blocks at the hall ends and along its spine ---
    b(-6.0, 0.0, 19.0, 6.0, 2.6, 22.0, vary(machine));
    b(-6.0, 0.0, -22.0, 6.0, 2.6, -19.0, vary(machine));
    b(-3.0, 0.0, -15.0, 3.0, 2.4, -12.0, vary(machine));
    b(-3.0, 0.0, 12.0, 3.0, 2.4, 15.0, vary(machine));

    // --- overhead catwalks, carried on the machine banks ---
    for z0 in [-9.0, 8.0] {
        b(-13.0, 3.4, z0, 13.0, 3.8, z0 + 1.0, vary(catwalk));
        b(-13.0, 3.8, z0 + 0.8, 13.0, 4.2, z0 + 1.0, rail);
        b(10.6, 2.6, z0 + 0.1, 11.4, 3.4, z0 + 0.9, vary(rail));
        b(-11.4, 2.6, z0 + 0.1, -10.6, 3.4, z0 + 0.9, vary(rail));
    }

    // --- crates ---
    b(19.0, 0.0, 4.0, 21.0, 0.6, 6.0, vary(crate_col));
    b(-21.0, 0.0, -8.0, -19.0, 0.6, -6.0, vary(crate_col));
    b(-2.0, 0.0, 8.0, -0.5, 0.6, 9.5, vary(crate_col));
    b(13.9, 0.0, -16.2, 15.9, 0.6, -14.2, vary(crate_col));

    let spawns = [
        (0.5, -17.5),
        (-5.5, 6.5),
        (5.5, -8.5),
        (14.5, -8.5),
        (15.5, 4.5),
        (14.5, 18.5),
        (-14.5, 8.5),
        (-15.5, -5.5),
        (-14.5, -18.5),
        (20.5, 0.5),
        (20.5, 20.5),
        (-20.5, -20.5),
    ]
    .iter()
    .map(|(x, z)| vec3(*x, 0.0, *z))
    .collect();

    Layout {
        name_en: "FOUNDRY",
        name_zh: "铸造厂",
        boxes,
        voids: Vec::new(),
        player_start: vec3(0.5, 0.5, 17.5),
        spawns,
        extraction: Some(vec3(-20.5, 0.0, 20.5)),
        half: 24.0,
        sky: Color::new(0.09, 0.05, 0.04, 1.0),
        accent: Color::new(0.95, 0.44, 0.14, 1.0),
    }
}

/// DOCKS — mission 3, the marksman arena.
///
/// * **Plan** — four container blocks in the corners of a plus-shaped expanse
///   of open ground: a 24 m wide boulevard north to south, matching arms east
///   and west, and a 4 m lane through each quadrant whose far end the container
///   rows shut off from the boulevard. *No man's land* again, but with a real
///   alternative route, which is what keeps the long lanes a threat rather than
///   a wall.
/// * **Sightlines** — 59.1 m down the boulevard, and the openest arena in the
///   campaign by depth: the *shortest* deepest-visible point anywhere is 29.5 m,
///   so there is no position on this map from which you cannot be seen a long
///   way off. Typical unobstructed ray 13.1 m; 10% of rays reach 39 m, twice
///   any other arena.
/// * **Cover** — 24 full-height pieces (containers 2.6 and 5.2 m, crane legs
///   8 m) and two low ones: the arterial pallets at (2, 2) and (0, -16), which
///   are stacked to 1.15 m. They are the fix for the one design hole the
///   measurements found here — 1.15 m hides a crouched body (eye 1.03 m) and
///   still breaks no sightline for a standing one (eye 1.6 m), which is what
///   lets the boulevard punish careless crossing and reward a careful one. The
///   other four pallets stay at 0.6 m, under the 0.62 m a body steps over.
pub fn docks() -> Layout {
    let mut boxes: Vec<Block> = Vec::new();
    let mut rng = Rng::new(0xD0_C5_5A_17);

    let floor = skin(0.13, 0.15, 0.18, Surface::Concrete);
    let wall = skin(0.22, 0.26, 0.30, Surface::Corrugated);
    let crate_col = skin(0.27, 0.32, 0.29, Surface::Crate);
    let steel = skin(0.30, 0.33, 0.36, Surface::Corrugated);
    let paint_a = skin(0.20, 0.32, 0.38, Surface::Tiles);
    let paint_b = skin(0.26, 0.30, 0.36, Surface::Tiles);
    let paint_c = skin(0.18, 0.24, 0.30, Surface::Tiles);
    let accent = skin(0.35, 0.55, 0.62, Surface::Corrugated);

    let mut vary = |c: Skin| c.vary(0.94 + rng.f32() * 0.12);

    let mut b = |x0: f32, y0: f32, z0: f32, x1: f32, y1: f32, z1: f32, s: Skin| {
        boxes.push(Block {
            min: vec3(x0, y0, z0),
            max: vec3(x1, y1, z1),
            skin: s,
        });
    };

    // --- ground & outer shell ---
    b(-28.0, -1.0, -28.0, 28.0, 0.0, 28.0, floor);
    b(-28.0, 0.0, 27.0, 28.0, WALL_H, 28.0, wall);
    b(-28.0, 0.0, -28.0, 28.0, WALL_H, -27.0, wall);
    b(-28.0, 0.0, -27.0, -27.0, WALL_H, 27.0, wall);
    b(27.0, 0.0, -27.0, 28.0, WALL_H, 27.0, wall);
    // Wall trim
    b(-28.0, 0.0, 26.85, 28.0, 0.35, 27.0, accent);
    b(-28.0, 0.0, -27.0, 28.0, 0.35, -26.85, accent);
    b(-27.0, 0.0, -28.0, -26.85, 0.35, 28.0, accent);
    b(26.85, 0.0, -28.0, 27.0, 0.35, 28.0, accent);

    // --- container blocks: four quadrants, lanes of 4 m between the rows ---
    for (sx, sz) in [(1.0f32, 1.0f32), (-1.0, 1.0), (1.0, -1.0), (-1.0, -1.0)] {
        let (xa, xb) = (12.0 * sx, 21.0 * sx);
        let (xc, xd) = (23.0 * sx, 26.0 * sx);
        let (za, zb) = (12.0 * sz, 15.0 * sz);
        let (zc, zd) = (19.0 * sz, 22.0 * sz);
        let col = if sx * sz > 0.0 { paint_a } else { paint_c };
        let (x0, x1) = (xa.min(xb), xa.max(xb));
        let (z0, z1) = (za.min(zb), za.max(zb));
        let (z2, z3) = (zc.min(zd), zc.max(zd));
        let (x2, x3) = (xc.min(xd), xc.max(xd));
        let (z4, z5) = (z0.min(z2), z1.max(z3));
        // row running along x
        b(x0, 0.0, z0, x1, 2.6, z1, vary(col));
        // double-stacked row running along x
        b(x0, 0.0, z2, x1, 5.2, z3, vary(paint_b));
        // cross row walling the quadrant off from the apron
        b(x2, 0.0, z4, x3, 2.6, z5, vary(crate_col));
    }

    // --- gantry cranes over the central yard ---
    for (x0, z0) in [(6.0, 6.0), (-8.0, 6.0), (6.0, -8.0), (-8.0, -8.0)] {
        b(x0, 0.0, z0, x0 + 2.0, 8.0, z0 + 2.0, vary(steel));
    }
    b(-8.5, 8.0, 6.2, 8.5, 10.0, 7.8, steel);
    b(-8.5, 8.0, -7.8, 8.5, 10.0, -6.2, steel);
    b(6.2, 8.0, -8.5, 7.8, 10.0, 8.5, steel);
    b(-7.8, 8.0, -8.5, -6.2, 10.0, 8.5, steel);

    // --- pallets ---
    //
    // Six pallets, four of them out in the lanes the mission wants long and
    // open, two of them in the artery the player has to cross to get anywhere:
    // the central yard and the south of the boulevard. Both artery pallets are
    // stacked to 1.15 m and the lane ones stay at 0.6 m. That is the height at
    // which DOCKS' own cover finally does something — a 0.6 m pallet is under
    // the 0.62 m a body steps over and far under the 1.03 m eye of a crouched
    // player, so it protects nobody — while 1.15 m is still under the 1.6 m
    // sightline datum, so it breaks no sightline that a standing player was
    // relying on. The yard stays a place where careless movement is punished;
    // what changes is that careful movement is now possible in it.
    b(18.0, 0.0, 0.0, 19.5, 0.6, 1.5, vary(crate_col));
    b(-16.0, 0.0, 16.0, -14.5, 0.6, 17.5, vary(crate_col));
    b(0.0, 0.0, -16.0, 1.5, 1.15, -14.5, vary(crate_col));
    b(2.0, 0.0, 2.0, 3.5, 1.15, 3.5, vary(crate_col));
    b(24.0, 0.0, -6.0, 25.5, 0.6, -4.5, vary(crate_col));
    b(-25.5, 0.0, 4.5, -24.0, 0.6, 6.0, vary(crate_col));

    let spawns = [
        (-4.5, -4.5),
        (4.5, 4.5),
        (0.5, 10.5),
        (10.5, 16.5),
        (16.5, 16.5),
        (-16.5, 16.5),
        (16.5, -16.5),
        (25.5, 0.5),
        (-25.5, 0.5),
        (0.5, 25.5),
        (0.5, -25.5),
        (25.5, 25.5),
    ]
    .iter()
    .map(|(x, z)| vec3(*x, 0.0, *z))
    .collect();

    Layout {
        name_en: "DOCKS",
        name_zh: "码头",
        boxes,
        voids: Vec::new(),
        player_start: vec3(0.5, 0.5, -4.5),
        spawns,
        extraction: Some(vec3(-25.5, 0.0, 25.5)),
        half: 28.0,
        sky: Color::new(0.06, 0.08, 0.11, 1.0),
        accent: Color::new(0.38, 0.66, 0.76, 1.0),
    }
}

/// BUNKER — mission 4, the close-quarters hold-out.
///
/// * **Plan** — a single loop: a solid reactor core with a 2.5 m corridor round
///   it, four 3 m blast doors through the ring wall, and radial partitions
///   carving the outer ring into small rooms. None of the reference's types
///   fits — it is one ring, and every space in it is a way through rather than a
///   place to hold, which is what makes the survive timer work.
/// * **Sightlines** — the shortest in the campaign: 32.6 m end to end, which is
///   under the 39 m a marksman needs, and the levels agree (BUNKER is the one
///   mission with no `Sniper` in its spawn table). Typical unobstructed ray
///   5.2 m, 46% under 4.9 m, and a position exists that sees 8.2 m at its
///   farthest. Two rooms can be fought over without either side ever seeing the
///   other, which is what makes the ring work as a hold-out.
/// * **Cover** — 20 full-height pieces (the core, the ring wall, the radial
///   partitions) and four equipment racks at 1.4 m. The racks are this arena's
///   crouch cover: tall enough to hide a crouched body (eye 1.03 m) and short
///   enough to shoot over standing (eye 1.64 m), so it is cover at crouch
///   height even though it sits above the reference's 1.25 m band.
pub fn bunker() -> Layout {
    let mut boxes: Vec<Block> = Vec::new();
    let mut rng = Rng::new(0xB0_0B_1E_5F);

    let floor = skin(0.13, 0.15, 0.13, Surface::Concrete);
    let wall = skin(0.22, 0.26, 0.23, Surface::Tiles);
    let core = skin(0.26, 0.30, 0.27, Surface::Tiles);
    let rack = skin(0.20, 0.24, 0.21, Surface::Corrugated);
    let crate_col = skin(0.30, 0.32, 0.26, Surface::Crate);
    let accent = skin(0.42, 0.52, 0.40, Surface::Corrugated);

    let mut vary = |c: Skin| c.vary(0.94 + rng.f32() * 0.12);

    let mut b = |x0: f32, y0: f32, z0: f32, x1: f32, y1: f32, z1: f32, s: Skin| {
        boxes.push(Block {
            min: vec3(x0, y0, z0),
            max: vec3(x1, y1, z1),
            skin: s,
        });
    };

    // --- ground & outer shell ---
    b(-18.0, -1.0, -18.0, 18.0, 0.0, 18.0, floor);
    b(-18.0, 0.0, 17.0, 18.0, WALL_H, 18.0, wall);
    b(-18.0, 0.0, -18.0, 18.0, WALL_H, -17.0, wall);
    b(-18.0, 0.0, -17.0, -17.0, WALL_H, 17.0, wall);
    b(17.0, 0.0, -17.0, 18.0, WALL_H, 17.0, wall);
    // Wall trim
    b(-18.0, 0.0, 16.85, 18.0, 0.35, 17.0, accent);
    b(-18.0, 0.0, -17.0, 18.0, 0.35, -16.85, accent);
    b(-17.0, 0.0, -18.0, -16.85, 0.35, 18.0, accent);
    b(16.85, 0.0, -18.0, 17.0, 0.35, 18.0, accent);

    // --- reactor core ---
    b(-4.5, 0.0, -4.5, 4.5, 3.4, 4.5, vary(core));
    b(-4.7, 3.4, -4.7, 4.7, 3.7, 4.7, accent);

    // --- ring wall around the core: a 2.5 m corridor loop with 3 m blast doors ---
    b(-8.0, 0.0, 7.0, -1.5, 3.4, 8.0, vary(wall));
    b(1.5, 0.0, 7.0, 8.0, 3.4, 8.0, vary(wall));
    b(-8.0, 0.0, -8.0, -1.5, 3.4, -7.0, vary(wall));
    b(1.5, 0.0, -8.0, 8.0, 3.4, -7.0, vary(wall));
    b(-8.0, 0.0, -7.0, -7.0, 3.4, -1.5, vary(wall));
    b(-8.0, 0.0, 1.5, -7.0, 3.4, 7.0, vary(wall));
    b(7.0, 0.0, -7.0, 8.0, 3.4, -1.5, vary(wall));
    b(7.0, 0.0, 1.5, 8.0, 3.4, 7.0, vary(wall));

    // --- radial partitions carving the outer ring into small rooms (3 m doors) ---
    for (x0, x1) in [(4.0, 5.0), (-5.0, -4.0)] {
        for (z0, z1) in [(8.0, 12.0), (15.0, 17.0), (-12.0, -8.0), (-17.0, -15.0)] {
            b(x0, 0.0, z0, x1, 3.2, z1, vary(wall));
        }
    }
    for (z0, z1) in [(3.0, 4.0), (-4.0, -3.0)] {
        for (x0, x1) in [(8.0, 12.5), (15.5, 17.0), (-12.5, -8.0), (-17.0, -15.5)] {
            b(x0, 0.0, z0, x1, 3.2, z1, vary(wall));
        }
    }

    // --- equipment racks ---
    b(11.0, 0.0, 9.0, 13.0, 1.4, 11.0, vary(rack));
    b(-13.0, 0.0, 9.0, -11.0, 1.4, 11.0, vary(rack));
    b(11.0, 0.0, -11.0, 13.0, 1.4, -9.0, vary(rack));
    b(-13.0, 0.0, -11.0, -11.0, 1.4, -9.0, vary(rack));

    // --- crates ---
    b(14.0, 0.0, 0.0, 15.5, 0.6, 1.5, vary(crate_col));
    b(-15.5, 0.0, 5.0, -14.0, 0.6, 6.5, vary(crate_col));

    let spawns = [
        (13.5, 13.5),
        (-13.5, 13.5),
        (13.5, -13.5),
        (-13.5, -13.5),
        (0.5, 11.5),
        (0.5, -11.5),
        (13.5, 0.5),
        (-14.5, -0.5),
        (5.5, 0.5),
        (-5.5, -0.5),
        (0.5, 5.5),
        (9.5, 9.5),
    ]
    .iter()
    .map(|(x, z)| vec3(*x, 0.0, *z))
    .collect();

    Layout {
        name_en: "BUNKER",
        name_zh: "地堡",
        boxes,
        voids: Vec::new(),
        player_start: vec3(0.5, 0.5, 13.5),
        spawns,
        extraction: Some(vec3(-15.5, 0.0, 15.5)),
        half: 18.0,
        sky: Color::new(0.07, 0.09, 0.07, 1.0),
        accent: Color::new(0.50, 0.68, 0.46, 1.0),
    }
}

/// CITADEL — mission 5, the one arena where the high ground is handed to the
/// player.
///
/// * **Plan** — nested rings: a 16 x 16 m keep raised 2.0 m and reachable by
///   four 0.5 m staircases, a courtyard of four 4.5 m pillars around it, a
///   curtain wall at |x| or |z| = 18-19 with four 8 m gates, and four corner
///   galleries outside it. Closest reference type is *base*: a defended core
///   with one way in per side and the flanks outside the wall.
/// * **Sightlines** — 56.6 m across the courtyard; typical unobstructed ray
///   9.7 m, 26% under 4.9 m, 3% past 39 m. The gate-to-gate lines are the
///   marksman's, and the tower is the answer to them.
/// * **Elevation** — the keep is the campaign's only player-only perch: the
///   navigation grid blocks every cell whose footprint touches a solid, so the
///   2.0 m platform is impassable to AI pathing and the staircases up to it are
///   never used by anything but the player. That is a *sniper nest* by
///   construction, and it is deliberate — but it is also the one property here
///   that was never written down, so it is asserted in the tests below.
/// * **Cover** — 21 full-height pieces and nothing in between. All of it is
///   wall: the keep's edge, the curtain wall, the gates' flanks, the pillars.
///   No piece in this arena is between 1.0 m and 1.75 m, so every fight here is
///   either in the open or round a corner.
pub fn citadel() -> Layout {
    let mut boxes: Vec<Block> = Vec::new();
    let mut rng = Rng::new(0xC1_7A_DE_11);

    let floor = skin(0.12, 0.10, 0.16, Surface::Tiles);
    let wall = skin(0.22, 0.18, 0.30, Surface::Tiles);
    let platform = skin(0.26, 0.21, 0.34, Surface::Concrete);
    let stair = skin(0.30, 0.24, 0.38, Surface::Concrete);
    let pillar = skin(0.24, 0.19, 0.31, Surface::Corrugated);
    let crate_col = skin(0.30, 0.26, 0.29, Surface::Crate);
    let accent = skin(0.62, 0.30, 0.72, Surface::Corrugated);

    let mut vary = |c: Skin| c.vary(0.94 + rng.f32() * 0.12);

    let mut b = |x0: f32, y0: f32, z0: f32, x1: f32, y1: f32, z1: f32, s: Skin| {
        boxes.push(Block {
            min: vec3(x0, y0, z0),
            max: vec3(x1, y1, z1),
            skin: s,
        });
    };

    // --- ground & outer shell ---
    b(-30.0, -1.0, -30.0, 30.0, 0.0, 30.0, floor);
    b(-30.0, 0.0, 29.0, 30.0, WALL_H, 30.0, wall);
    b(-30.0, 0.0, -30.0, 30.0, WALL_H, -29.0, wall);
    b(-30.0, 0.0, -29.0, -29.0, WALL_H, 29.0, wall);
    b(29.0, 0.0, -29.0, 30.0, WALL_H, 29.0, wall);
    // Wall trim
    b(-30.0, 0.0, 28.85, 30.0, 0.35, 29.0, accent);
    b(-30.0, 0.0, -29.0, 30.0, 0.35, -28.85, accent);
    b(-29.0, 0.0, -30.0, -28.85, 0.35, 30.0, accent);
    b(28.85, 0.0, -30.0, 29.0, 0.35, 30.0, accent);

    // --- raised keep, reachable from four staircases of 0.5 m steps ---
    b(-8.0, 0.0, -8.0, 8.0, 2.0, 8.0, vary(platform));
    for (dx, dz) in [(1.0, 0.0), (-1.0, 0.0), (0.0, 1.0), (0.0, -1.0)] {
        for (i, h) in [1.5f32, 1.0, 0.5].iter().enumerate() {
            let near = 8.0 + i as f32 * 2.0;
            let far = near + 2.0;
            let (x0, x1, z0, z1) = if dz == 0.0 {
                if dx > 0.0 {
                    (near, far, -3.0, 3.0)
                } else {
                    (-far, -near, -3.0, 3.0)
                }
            } else if dz > 0.0 {
                (-3.0, 3.0, near, far)
            } else {
                (-3.0, 3.0, -far, -near)
            };
            b(x0, 0.0, z0, x1, *h, z1, vary(stair));
        }
    }
    b(-1.4, 2.0, -1.4, 1.4, 4.6, 1.4, vary(pillar));

    // --- curtain wall with four wide gates ---
    b(-19.0, 0.0, 18.0, -4.0, 3.0, 19.0, vary(wall));
    b(4.0, 0.0, 18.0, 19.0, 3.0, 19.0, vary(wall));
    b(-19.0, 0.0, -19.0, -4.0, 3.0, -18.0, vary(wall));
    b(4.0, 0.0, -19.0, 19.0, 3.0, -18.0, vary(wall));
    b(-19.0, 0.0, -18.0, -18.0, 3.0, -4.0, vary(wall));
    b(-19.0, 0.0, 4.0, -18.0, 3.0, 18.0, vary(wall));
    b(18.0, 0.0, -18.0, 19.0, 3.0, -4.0, vary(wall));
    b(18.0, 0.0, 4.0, 19.0, 3.0, 18.0, vary(wall));

    // --- courtyard pillars ---
    for (x0, z0) in [(12.0, 12.0), (-14.0, 12.0), (12.0, -14.0), (-14.0, -14.0)] {
        b(x0, 0.0, z0, x0 + 2.0, 4.5, z0 + 2.0, vary(pillar));
    }

    // --- corner keeps closing the outer ring into four flanking galleries ---
    for (x0, z0) in [(24.0, 19.0), (-28.0, 19.0), (24.0, -29.0), (-28.0, -29.0)] {
        // wing along the outer wall
        b(x0, 0.0, z0, x0 + 4.0, 3.5, z0 + 10.0, vary(wall));
    }
    for (x0, z0) in [(19.0, 19.0), (-24.0, 19.0), (19.0, -24.0), (-24.0, -24.0)] {
        // block tying the wing back to the curtain wall
        b(x0, 0.0, z0, x0 + 5.0, 3.5, z0 + 5.0, vary(wall));
    }

    // --- cover ---
    b(2.0, 2.0, 2.0, 4.0, 2.6, 4.0, vary(crate_col));
    b(-4.0, 2.0, -4.0, -2.0, 2.6, -2.0, vary(crate_col));

    let spawns = [
        (-9.5, -9.5),
        (9.5, -9.5),
        (-9.5, 9.5),
        (9.5, 9.5),
        (-16.5, 0.5),
        (16.5, 0.5),
        (-16.5, -16.5),
        (25.5, -15.5),
        (-25.5, 15.5),
        (15.5, 25.5),
        (-15.5, -25.5),
        (0.5, -25.5),
    ]
    .iter()
    .map(|(x, z)| vec3(*x, 0.0, *z))
    .collect();

    Layout {
        name_en: "CITADEL",
        name_zh: "要塞",
        boxes,
        voids: Vec::new(),
        player_start: vec3(0.5, 0.5, -16.5),
        spawns,
        extraction: Some(vec3(0.5, 0.0, 16.5)),
        half: 30.0,
        sky: Color::new(0.08, 0.05, 0.12, 1.0),
        accent: Color::new(0.74, 0.38, 0.88, 1.0),
    }
}

/// R'LYEH — mission 6. The drowned city. Every piece of it is still an
/// axis-aligned box; the wrongness is entirely in the composition. Doorways
/// never line up with the opening beyond them, the colonnade's spacing follows
/// no rule you can learn, one trilithon motif repeats at three scales, and the
/// middle of the floor is an 18 x 18 hole with nothing under it.
///
/// * **Plan** — no single reference type: four unalike wings (a colonnade west,
///   a stepped ziggurat east, the Mirror Hall north, the slabbed halls south)
///   around a bottomless centre. It is the only arena in the campaign that is
///   not a variation on one room, and the only one you can get lost in.
/// * **Sightlines** — the campaign's extremes at once: 70.0 m across the
///   diagonal, longer than any other arena, and a position that sees only 7.2 m
///   at its farthest, shorter than any other arena. Typical unobstructed ray
///   7.1 m with 36% under 4.9 m, and the deepest-visible-point median is 45.7 m
///   because the wings all open onto the pit. Fighting here alternates between
///   a corner and a cathedral.
/// * **Cover** — 43 full-height pieces (colonnade and promenade monoliths
///   11-15 m, the four Watchers, the Mirror Hall and south-hall wall runs, the
///   slabs across the halls, the idol over the ziggurat), 5 chest-high ones and
///   exactly one low piece. The 0.6 m trilithons are deliberately not cover:
///   they are the motif at its smallest and bodies step over them, which is the
///   point of repeating it. Nothing here is at eye height by accident; the
///   arena is built to be misread.
pub fn rlyeh() -> Layout {
    let mut boxes: Vec<Block> = Vec::new();
    let mut rng = Rng::new(0x52_4C_59_45);

    // Cold and drowned: teal-grey stone, bone, wet purple, and a sick green
    // light that has no business being down here.
    let flag = skin(0.24, 0.30, 0.31, Surface::Tiles);
    let shell = skin(0.27, 0.33, 0.35, Surface::Tiles);
    let monolith = skin(0.31, 0.37, 0.39, Surface::Tiles);
    let drowned = skin(0.20, 0.26, 0.28, Surface::Concrete);
    let bone = skin(0.68, 0.67, 0.58, Surface::Corrugated);
    let wet = skin(0.37, 0.21, 0.33, Surface::Tiles);
    let glow = skin(0.34, 0.64, 0.43, Surface::Corrugated);

    let mut vary = |c: Skin| c.vary(0.90 + rng.f32() * 0.18);

    let mut b = |x0: f32, y0: f32, z0: f32, x1: f32, y1: f32, z1: f32, s: Skin| {
        boxes.push(Block {
            min: vec3(x0, y0, z0),
            max: vec3(x1, y1, z1),
            skin: s,
        });
    };

    // --- ground: four slabs framing the abyss, so the hole is a genuine gap
    // in the floor rather than a rectangle painted over one ---
    b(-34.0, -1.0, -34.0, -9.0, 0.0, 34.0, flag);
    b(9.0, -1.0, -34.0, 34.0, 0.0, 34.0, flag);
    b(-9.0, -1.0, -34.0, 9.0, 0.0, -9.0, flag);
    b(-9.0, -1.0, 9.0, 9.0, 0.0, 34.0, flag);

    // --- shell: 16 m of stone on every side ---
    b(-34.0, 0.0, 33.0, 34.0, 16.0, 34.0, shell);
    b(-34.0, 0.0, -34.0, 34.0, 16.0, -33.0, shell);
    b(-34.0, 0.0, -33.0, -33.0, 16.0, 33.0, shell);
    b(33.0, 0.0, -33.0, 34.0, 16.0, 33.0, shell);
    // Something luminous has been creeping along the foot of the walls.
    b(-34.0, 0.0, 32.8, 34.0, 0.35, 33.0, glow);
    b(-34.0, 0.0, -33.0, 34.0, 0.35, -32.8, glow);
    b(-33.0, 0.0, -32.8, -32.8, 0.35, 32.8, glow);
    b(32.8, 0.0, -32.8, 33.0, 0.35, 32.8, glow);

    // --- the abyss: a bone lip you step over, and four Watchers standing
    // just clear of the corners ---
    b(-9.8, 0.0, -9.0, -9.0, 0.45, 9.0, bone);
    b(9.0, 0.0, -9.0, 9.8, 0.45, 9.0, bone);
    b(-9.8, 0.0, 9.0, 9.8, 0.45, 9.8, bone);
    b(-9.8, 0.0, -9.8, 9.8, 0.45, -9.0, bone);
    for (sx, sz) in [(-1.0f32, 1.0f32), (1.0, 1.0), (-1.0, -1.0), (1.0, -1.0)] {
        let (x, z) = (12.0 * sx, 11.6 * sz);
        b(
            x - 1.8,
            0.0,
            z - 1.8,
            x + 1.8,
            11.0,
            z + 1.8,
            vary(monolith),
        );
        b(x - 2.1, 11.0, z - 2.1, x + 2.1, 11.6, z + 2.1, bone);
    }

    // --- the motif at its largest: one trilithon standing in the abyss, its
    // uprights on either shore and its lintel spanning the hole ---
    b(-13.0, 0.0, -2.6, -9.6, 15.0, 2.6, vary(monolith));
    b(9.6, 0.0, -2.6, 13.0, 15.0, 2.6, vary(monolith));
    b(-13.6, 15.0, -3.0, 13.6, 17.6, 3.0, bone);

    // --- west: the colonnade. Nine monoliths whose spacing and setback never
    // settle into a rhythm you can predict ---
    for (x, z, w, h) in [
        (-22.6f32, -28.0f32, 2.4f32, 12.5f32),
        (-24.0, -21.5, 2.4, 12.0),
        (-21.9, -16.0, 3.0, 13.4),
        (-23.3, -9.0, 2.4, 12.0),
        (-22.0, -3.5, 2.4, 12.6),
        (-24.1, 3.0, 3.0, 13.0),
        (-21.7, 10.5, 2.4, 12.0),
        (-23.6, 16.0, 2.4, 12.6),
        (-22.2, 23.5, 2.4, 12.0),
    ] {
        b(
            x - w * 0.5,
            0.0,
            z - w * 0.5,
            x + w * 0.5,
            h,
            z + w * 0.5,
            vary(monolith),
        );
    }
    // Two of the gaps get the lintel that turns a pair of columns into the
    // same trilithon, one scale down.
    b(-24.8, 12.5, -5.0, -21.0, 14.2, 4.7, bone);
    b(-24.4, 12.0, 14.6, -21.4, 13.6, 24.9, bone);
    // Buttresses stepping into the west aisle from alternating sides, so no
    // straight line runs its length: the wall-side one pushes you east, the
    // colonnade-side pair pushes you west, and the aisle bends between them.
    b(-33.0, 0.0, 2.5, -29.0, 7.0, 3.5, vary(shell));
    b(-29.0, 0.0, -19.5, -25.6, 7.0, -18.5, vary(shell));
    b(-29.0, 0.0, 9.5, -25.6, 7.0, 10.5, vary(shell));

    // --- east: four tiers rising to an idol. Every riser is 0.5 m, so the
    // whole ziggurat is climbable, and each tier is narrower than the last ---
    b(14.0, 0.0, -16.0, 32.0, 0.5, 16.0, vary(monolith));
    b(17.0, 0.0, -13.0, 32.0, 1.0, 13.0, vary(drowned));
    b(20.0, 0.0, -10.0, 32.0, 1.5, 10.0, vary(monolith));
    b(23.0, 0.0, -7.0, 32.0, 2.0, 7.0, vary(drowned));
    b(25.5, 2.0, -2.5, 29.5, 14.0, 2.5, vary(wet));

    // --- north: the Mirror Hall. Two wall lines mirrored about the centre,
    // with their doorways deliberately out of step with each other ---
    b(-29.0, 0.0, 14.0, -8.0, 9.0, 15.0, vary(shell));
    b(2.0, 0.0, 14.0, 29.0, 9.0, 15.0, vary(shell));
    b(-29.0, 0.0, 22.0, 5.0, 9.0, 23.0, vary(shell));
    b(11.0, 0.0, 22.0, 29.0, 9.0, 23.0, vary(shell));

    // --- south: the same two lines, with the openings thrown to the far side,
    // so the way you came in is not the way you go out ---
    b(-29.0, 0.0, -14.0, -2.0, 8.0, -13.0, vary(shell));
    b(8.0, 0.0, -14.0, 29.0, 8.0, -13.0, vary(shell));
    b(-29.0, 0.0, -22.0, -11.0, 8.0, -21.0, vary(shell));
    b(-1.0, 0.0, -22.0, 29.0, 8.0, -21.0, vary(shell));

    // --- the two promenades. Monoliths pushed out of the walls in pairs that
    // overlap in depth but not in position, so no aisle is clear end to end
    // and the way through any of them has to zigzag ---
    b(-12.0, 0.0, -33.0, -8.0, 13.0, -28.0, vary(monolith));
    b(6.0, 0.0, -27.5, 10.5, 12.0, -22.0, vary(monolith));
    b(-19.0, 0.0, 23.0, -15.0, 13.5, 27.4, vary(monolith));
    b(-21.5, 0.0, 28.5, -17.5, 12.0, 33.0, vary(monolith));
    b(11.0, 0.0, 28.0, 15.0, 12.5, 33.0, vary(monolith));

    // --- slabs across the hall between the south wall lines, alternating
    // which side they leave the gap on: the passage bends three times and
    // every part of it is hidden from every other ---
    b(-1.0, 0.0, -21.0, 0.0, 9.0, -16.6, vary(drowned));
    b(-10.0, 0.0, -18.6, -9.2, 8.0, -14.0, vary(drowned));
    b(14.0, 0.0, -18.6, 14.8, 8.0, -14.0, vary(drowned));

    // --- the motif at its smallest: same three stones, 0.6 m high, so bodies
    // step over them instead of pathing around them ---
    for (tx, tz) in [(-16.5f32, 12.5f32), (16.5, 17.5), (16.5, -17.5)] {
        b(tx - 1.1, 0.0, tz - 0.35, tx - 0.5, 0.45, tz + 0.35, bone);
        b(tx + 0.5, 0.0, tz - 0.35, tx + 1.1, 0.45, tz + 0.35, bone);
        b(tx - 1.1, 0.45, tz - 0.35, tx + 1.1, 0.6, tz + 0.35, wet);
    }

    // --- 触手 / growths: something under the flagstones is pushing up. Each is
    // a stack of shrinking boxes that leans as it climbs, so four
    // axis-aligned blocks still read as one tapering limb ---
    for (gx, gz, lx, lz) in [
        (-30.6f32, 5.0f32, 0.46f32, 0.30f32),
        (26.4, 16.4, -0.06, -0.52),
        (10.9, -4.8, -0.62, 0.22),
    ] {
        let (mut x, mut z) = (gx, gz);
        let (mut y, mut w) = (0.0f32, 1.7f32);
        for i in 0..4 {
            let h = w * 1.45;
            let c = if i % 2 == 0 { bone } else { wet };
            b(
                x - w * 0.5,
                y,
                z - w * 0.5,
                x + w * 0.5,
                y + h,
                z + w * 0.5,
                vary(c),
            );
            // Overlap the next piece into the last one so the limb has no
            // visible joints.
            y += h * 0.72;
            w *= 0.74;
            x += lx;
            z += lz;
        }
    }

    // --- loose stones, an order of magnitude smaller than anything else ---
    b(24.5, 2.0, 5.0, 25.5, 3.2, 6.0, vary(wet));
    b(-14.6, 0.0, 4.2, -13.5, 0.6, 5.3, vary(monolith));
    b(18.0, 0.0, -24.5, 19.1, 0.6, -23.4, vary(monolith));

    // --- monoliths crowding the lip of the pit: one in front of the Mirror
    // Hall door so the void is not revealed until you step around it, one
    // growing out of the south-east Watcher into a corner mass ---
    b(-1.0, 0.0, 9.8, 5.0, 13.5, 13.4, vary(monolith));
    b(10.2, 0.0, -9.8, 13.8, 12.5, -7.0, vary(monolith));

    // --- and the middle of the floor is not there ---
    let abyss = (vec2(-9.0, -9.0), vec2(9.0, 9.0));

    let spawns = [
        (19.5, 19.5),
        (-19.5, 19.5),
        (21.5, -19.5),
        (-21.5, -19.5),
        (-16.5, 30.0),
        (17.5, 27.5),
        // Was (7.5, 11.5). That pocket is walled off from the rest of the
        // arena on every side but one, and its only connection was the
        // navigation grid wrongly treating the 1.0 m terrace to its east as
        // passable. With the grid threshold corrected to the height a body
        // can actually climb, the pocket is unreachable — which would have
        // stranded a hostile and made the mission impossible to finish.
        (7.5, 17.5),
        (-29.5, -3.5),
        (30.5, -24.5),
        (0.5, -27.5),
        (30.5, 30.5),
        (-26.5, -27.5),
    ]
    .iter()
    .map(|(x, z)| vec3(*x, 0.0, *z))
    .collect();

    Layout {
        name_en: "R'LYEH",
        name_zh: "拉莱耶",
        boxes,
        voids: vec![abyss],
        player_start: vec3(2.5, 0.5, 30.5),
        spawns,
        extraction: Some(vec3(-30.5, 0.0, -29.5)),
        half: 34.0,
        sky: Color::new(0.035, 0.075, 0.065, 1.0),
        accent: Color::new(0.38, 0.88, 0.50, 1.0),
    }
}

/// The six campaign missions, in play order.
pub fn campaign() -> [Layout; 6] {
    [arena(), foundry(), docks(), bunker(), citadel(), rlyeh()]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::level::{Aabb, Level, STEP_HEIGHT};

    /// Height the sightline measurements are taken at.
    ///
    /// A standing player's eye is at 1.64 m (`Stance::Stand::eye`), an enemy's
    /// at 0.86 of its body height, and the game's own spawn chooser already
    /// probes candidate sightlines at a flat 1.6 m (`pick_spawn` in `main.rs`).
    /// 1.6 m is therefore the plane a firefight actually happens on, and it sits
    /// just under the 1.75 m the design reference calls full cover, which is
    /// what makes the cover bands below meaningful in this project's units.
    const EYE: f32 = 1.6;

    /// Grid the arenas are sampled on. Two metres is fine enough that a 50 m
    /// arena still reports its long lanes to within a couple of metres, and
    /// coarse enough that the pairwise sightline sweep stays a second or so.
    const STRIDE: f32 = 2.0;

    /// A cover piece is something a body can use, not a building: floor slabs,
    /// the outer shell and the big platforms are structure. Footprint area is
    /// what separates the two here — the largest thing that is still cover
    /// (a dock container, 27 m^2) is well under this, and the smallest thing
    /// that is not (R'LYEH's 16 m shell) is far over it.
    const COVER_AREA: f32 = 60.0;

    /// How far a sightline is traced before the space it crosses counts as
    /// open. Above the diagonal of the largest arena (R'LYEH, 96 m), so a ray
    /// that reaches it is not clipped by the cap.
    const REACH: f32 = 100.0;

    /// What a piece of geometry is worth to a body standing on the same floor.
    ///
    /// The bands come from the level-design reference (high cover >= 1.75 m,
    /// low cover 1.0-1.25 m, non-cover <= 0.5 m), re-anchored to this project's
    /// own eye heights: standing eye 1.64 m, crouched eye 1.03 m, crouch body
    /// height 1.20 m, step height 0.62 m. So a `Low` piece is one you shoot
    /// over standing and hide behind crouched, and a `Full` piece is one that
    /// breaks a standing player's line of sight outright. The project's units
    /// are metres, so the reference's bands carry over unchanged.
    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    enum Cover {
        /// Stepped over: no protection either way.
        Ankle,
        /// Blocks a body but never hides one.
        Kerb,
        /// The reference's low cover, 1.0-1.25 m.
        Low,
        /// Too tall to shoot over crouched, too short to break standing sight.
        Chest,
        /// 1.75 m or more: breaks a standing player's line of sight.
        Full,
    }

    fn label(class: Cover) -> &'static str {
        match class {
            Cover::Ankle => "ankle",
            Cover::Kerb => "kerb",
            Cover::Low => "low",
            Cover::Chest => "chest",
            Cover::Full => "full",
        }
    }

    fn cover_class(height: f32) -> Cover {
        if height <= 0.5 {
            Cover::Ankle
        } else if height < 1.0 {
            Cover::Kerb
        } else if height <= 1.25 {
            Cover::Low
        } else if height < 1.75 {
            Cover::Chest
        } else {
            Cover::Full
        }
    }

    /// One non-structural block, classified by what it does for a body beside
    /// it.
    #[derive(Clone, Copy)]
    struct Piece {
        class: Cover,
        /// Height above the surface it stands on. Blocks rest on the floor
        /// below them in all six layouts, so this is also how far the top sits
        /// above that floor.
        height: f32,
        top: f32,
        /// XZ footprint.
        min: Vec2,
        max: Vec2,
    }

    /// Everything that is a thing *in* the arena rather than the arena itself.
    fn pieces(layout: &Layout) -> Vec<Piece> {
        layout
            .boxes
            .iter()
            .filter(|b| (b.max.x - b.min.x) * (b.max.z - b.min.z) <= COVER_AREA)
            .map(|b| Piece {
                class: cover_class(b.max.y - b.min.y),
                height: b.max.y - b.min.y,
                top: b.max.y,
                min: vec2(b.min.x, b.min.z),
                max: vec2(b.max.x, b.max.z),
            })
            .collect()
    }

    /// True when a piece is a way through the arena rather than a place to hide
    /// in it: something a body walks up onto, either from the floor or from the
    /// piece beside it.
    ///
    /// Without this, CITADEL's 1.0 m and 1.5 m stair treads count as low and
    /// chest cover, and R'LYEH's 1.0/1.5 m ziggurat tiers do the same — the
    /// arena would pass its cover checks off the back of its own staircase.
    /// Every stair here is built 0.5 m to a step and every piece meant as cover
    /// stands at least 1.0 m above whatever is next to it, so the project's own
    /// step height separates the two without anyone having to declare which
    /// boxes were drawn as a staircase.
    fn is_tread(p: &Piece, all: &[Piece]) -> bool {
        if p.height <= STEP_HEIGHT + 0.02 {
            return true;
        }
        all.iter().any(|q| {
            let gap = (p.min - q.max).max(q.min - p.max);
            let rise = p.top - q.top;
            gap.x <= 0.3 && gap.y <= 0.3 && rise > 0.05 && rise <= STEP_HEIGHT + 0.02
        })
    }

    /// Cover, as opposed to terrain: what a body can crouch or stand behind.
    fn cover_pieces(layout: &Layout) -> Vec<Piece> {
        let all = pieces(layout);
        all.iter()
            .copied()
            .filter(|p| {
                !is_tread(p, &all) && matches!(p.class, Cover::Low | Cover::Chest | Cover::Full)
            })
            .collect()
    }

    /// Standing-eye positions on the sampled grid: anywhere a body fits, on any
    /// surface it can climb to. Deliberately not limited to the navigation grid —
    /// the player climbs things the AI will not.
    fn stand_points(level: &Level) -> Vec<Vec3> {
        let h = level.half;
        let n = (2.0 * h / STRIDE).floor() as i32;
        let mut out = Vec::new();
        for ix in 0..=n {
            for iz in 0..=n {
                let x = -h + ix as f32 * STRIDE;
                let z = -h + iz as f32 * STRIDE;
                // 2.4 m ceiling: the highest floor the player can climb to in
                // any of these arenas (CITADEL's keep, R'LYEH's ziggurat).
                let Some(top) = level.support_height(x, z, 0.45, 2.4) else {
                    continue;
                };
                let foot = vec3(x, top + 0.05, z);
                if level.point_free(foot, 0.42, 1.70) {
                    out.push(vec3(x, top + EYE, z));
                }
            }
        }
        out
    }

    fn visible(level: &Level, a: Vec3, b: Vec3) -> bool {
        level.line_of_sight(a, b)
    }

    /// The navigation grid's blocking rule, mirrored from `Level::from_layout`:
    /// a 1.24 m footprint swept from 0.15 m to 1.70 m is tested against every
    /// blocker. `level.rs`'s own tests are what hold the real grid; this mirror
    /// exists so a design claim about *what the AI can walk on* can be checked
    /// from here, where the layouts live.
    fn nav_blocked(level: &Level, x: f32, z: f32) -> bool {
        const CLEARANCE: f32 = 0.62;
        let probe = Aabb::new(
            vec3(x - CLEARANCE, 0.15, z - CLEARANCE),
            vec3(x + CLEARANCE, 1.70, z + CLEARANCE),
        );
        level.blockers.iter().any(|s| s.overlaps(&probe))
    }

    /// Farthest visible point from each sample, in metres; `-1.0` where nothing
    /// visible was found.
    ///
    /// Pairs are visited longest first, so a sample drops out as soon as its
    /// longest clear pair is found: the cost is one sort plus one line-of-sight
    /// test per sample rather than one per pair.
    fn depths(level: &Level, points: &[Vec3]) -> Vec<f32> {
        let mut pairs: Vec<(f32, u32, u32)> = Vec::new();
        for i in 0..points.len() {
            for j in i + 1..points.len() {
                pairs.push(((points[j] - points[i]).length(), i as u32, j as u32));
            }
        }
        pairs.sort_unstable_by(|a, b| b.0.total_cmp(&a.0));

        let mut best = vec![-1.0f32; points.len()];
        let mut left = points.len();
        for (d, i, j) in pairs {
            let (i, j) = (i as usize, j as usize);
            if best[i] >= 0.0 && best[j] >= 0.0 {
                continue;
            }
            if visible(level, points[i], points[j]) {
                if best[i] < 0.0 {
                    best[i] = d;
                    left -= 1;
                }
                if best[j] < 0.0 {
                    best[j] = d;
                    left -= 1;
                }
                if left == 0 {
                    break;
                }
            }
        }
        best
    }

    /// Unobstructed eye-height ray lengths, 16 directions per sample position.
    ///
    /// This is the arena's *engagement* profile rather than its extremes. The
    /// deepest-visible-point map above answers "how far away could I be shot
    /// from", which in an open yard is always the far wall; the median of these
    /// rays answers "how far away is the thing I am shooting at", because it is
    /// what a body sees when it looks anywhere rather than when it looks for
    /// the longest line it can find.
    fn ray_lengths(level: &Level, points: &[Vec3]) -> Vec<f32> {
        const DIRECTIONS: usize = 16;
        let mut out = Vec::with_capacity(points.len() * DIRECTIONS);
        for p in points {
            for i in 0..DIRECTIONS {
                let a = i as f32 * std::f32::consts::TAU / DIRECTIONS as f32;
                let dir = vec3(a.cos(), 0.0, a.sin());
                out.push(level.ray_world(*p, dir, REACH).unwrap_or(REACH));
            }
        }
        out
    }

    fn percentile(sorted: &[f32], p: f32) -> f32 {
        if sorted.is_empty() {
            return 0.0;
        }
        let i = ((sorted.len() - 1) as f32 * p).round() as usize;
        sorted[i]
    }

    fn sorted(mut v: Vec<f32>) -> Vec<f32> {
        v.sort_unstable_by(f32::total_cmp);
        v
    }

    struct Measured {
        level: Level,
        points: Vec<Vec3>,
        /// Every classified piece, treads included.
        all: Vec<Piece>,
        /// Farthest visible point per sample, for the samples that see anything.
        depth: Vec<f32>,
        /// Every unobstructed ray from every sample.
        rays: Vec<f32>,
        cover: Vec<Piece>,
    }

    fn measure(layout: &Layout) -> Measured {
        let level = Level::from_layout(layout);
        let points = stand_points(&level);
        let depth: Vec<f32> = sorted(depths(&level, &points))
            .into_iter()
            .filter(|d| *d >= 0.0)
            .collect();
        let rays = sorted(ray_lengths(&level, &points));
        Measured {
            level,
            points,
            all: pieces(layout),
            depth,
            rays,
            cover: cover_pieces(layout),
        }
    }

    /// The reference's three engagement bands, in this project's metres. TF2's
    /// Hammer unit is 0.01905 m, so close combat (256 u) is 4.9 m, the mid band
    /// (1024 u) is 19.5 m, and the range a marksman or a rocket wants (2048 u)
    /// is 39 m.
    const CLOSE: f32 = 4.9;
    const MID: f32 = 19.5;
    const LONG: f32 = 39.0;

    fn cover_count(layout: &Layout, class: Cover) -> usize {
        cover_pieces(layout)
            .iter()
            .filter(|p| p.class == class)
            .count()
    }

    /// Fraction of sampled rays at or under `distance`.
    fn at_most(rays: &[f32], distance: f32) -> f32 {
        rays.iter().filter(|r| **r <= distance).count() as f32 / rays.len() as f32
    }

    /// An arena whose lines are all one length is one fight repeated. Every map
    /// in the campaign has to hold a broken-up middle *and* at least one line
    /// long enough to be worth crossing — the two engagement scales, measured
    /// in the space rather than asserted about it.
    #[test]
    fn every_arena_offers_two_engagement_scales() {
        for layout in campaign() {
            let m = measure(&layout);
            let arena = layout.name_en;
            let median = percentile(&m.rays, 0.50);
            let longest = m.depth.last().copied().unwrap_or(0.0);
            assert!(
                median <= MID,
                "{arena}: a typical sightline is {median:.1} m, past the {MID} m mid band — nothing here is close"
            );
            // BUNKER is the floor at 32.6 m, allowed under the reference's
            // 39 m marksman band on purpose: it is the one arena whose mission
            // has no marksman in its spawn table. 29 m is as low as the
            // campaign can go and still have a line worth crossing.
            assert!(
                longest >= 29.0,
                "{arena}: the longest sightline is {longest:.1} m, so there is nothing to cross"
            );
            assert!(
                longest >= median * 3.0,
                "{arena}: {longest:.1} m against a typical {median:.1} m is one scale, not two"
            );
        }
    }

    /// The reference's warning about "cover boxes" — a level made of repeated
    /// waist-high blocks — in measurable form. Mid-height cover is welcome (it
    /// is the band a crouched player fights from) but it must never become the
    /// bulk of an arena's cover, because then every fight is the same fight.
    #[test]
    fn no_arena_is_a_field_of_cover_boxes() {
        for layout in campaign() {
            let cover = cover_pieces(&layout);
            let mid = cover
                .iter()
                .filter(|p| matches!(p.class, Cover::Low | Cover::Chest))
                .count();
            assert!(
                mid * 3 <= cover.len(),
                "{}: {mid} of {} cover pieces sit between 1.0 and 1.75 m",
                layout.name_en,
                cover.len()
            );
        }
    }

    /// `pick_spawn` refuses any spawn within 14 m of the player and quietly
    /// falls back to spawn 0 when every candidate is refused, which would put a
    /// hostile in the player's lap. Every arena has to keep a few candidates
    /// out of range from anywhere a body can stand.
    #[test]
    fn every_arena_gives_the_spawn_chooser_somewhere_to_put_an_enemy() {
        for layout in campaign() {
            let m = measure(&layout);
            let worst = m
                .points
                .iter()
                .map(|p| {
                    layout
                        .spawns
                        .iter()
                        .filter(|s| vec2(s.x - p.x, s.z - p.z).length() >= 14.0)
                        .count()
                })
                .min()
                .unwrap_or(0);
            assert!(
                worst >= 4,
                "{}: the worst position on the map leaves the chooser {worst} spawns it may use",
                layout.name_en
            );
        }
    }

    /// A platform is a solid, and the navigation grid blocks any cell whose
    /// footprint touches one, so no arena in the campaign has AI-navigable
    /// ground above 0.62 m. Every perch — the yard's plinth, the citadel's keep,
    /// the ziggurat, the catwalks — is player-only ground.
    ///
    /// That follows from a rule in `level.rs` rather than from a decision about
    /// these layouts, which is exactly why it is worth a test. The day it stops
    /// being true, every perch becomes a place the AI follows the player onto,
    /// and the arenas need a design pass rather than a fixed constant. The rule
    /// is mirrored in `nav_blocked`.
    #[test]
    fn elevation_is_player_only_in_every_arena() {
        for layout in campaign() {
            let m = measure(&layout);
            let h = layout.half;
            let n = (2.0 * h / STRIDE).floor() as i32;
            let mut highest = 0.0f32;
            for ix in 0..=n {
                for iz in 0..=n {
                    let (x, z) = (-h + ix as f32 * STRIDE, -h + iz as f32 * STRIDE);
                    if nav_blocked(&m.level, x, z) {
                        continue;
                    }
                    if let Some(top) = m.level.support_height(x, z, 0.45, 2.4) {
                        highest = highest.max(top);
                    }
                }
            }
            assert!(
                highest <= STEP_HEIGHT + 0.02,
                "{}: the AI can path onto ground {highest:.2} m up",
                layout.name_en
            );
        }
    }

    /// Mission 1's arena is the one that has to teach every cover band at once:
    /// enough 1.0-1.25 m pieces to learn crouching behind (four of its six are
    /// the 1.2 m walls crossing the middle yard, and they are the only pieces in
    /// the campaign a standing player can shoot over and a crouched one hide
    /// behind), enough full-height pieces to learn breaking contact behind, and
    /// a yard long enough to be worth crossing under fire.
    #[test]
    fn rust_yard_teaches_the_cover_bands() {
        let layout = arena();
        let (low, full) = (
            cover_count(&layout, Cover::Low),
            cover_count(&layout, Cover::Full),
        );
        assert!(
            low >= 4,
            "RUST YARD has {low} pieces in the 1.0-1.25 m band: crouching has nothing to teach"
        );
        assert!(
            full >= 8,
            "RUST YARD has {full} full-height pieces: there is nowhere to break contact"
        );
        let m = measure(&layout);
        assert!(
            m.depth.last().copied().unwrap_or(0.0) >= LONG,
            "RUST YARD's longest line is under the {LONG} m marksman band"
        );
        assert!(
            percentile(&m.rays, 0.50) <= MID,
            "RUST YARD's typical sightline has left the mid band"
        );
    }

    /// FOUNDRY is a lane map and its measurements have to agree: knife-range
    /// fights in nearly half of its sightlines, one central hall long enough to
    /// cross, and cover that is machinery rather than furniture. It has no
    /// 1.0-1.75 m cover at all, which is deliberate — a waist-high block in a
    /// 3 m walkway narrows the lane and protects nobody.
    #[test]
    fn foundry_is_a_lane_map() {
        let layout = foundry();
        let m = measure(&layout);
        let median = percentile(&m.rays, 0.50);
        assert!(
            median <= 8.0,
            "FOUNDRY's typical sightline is {median:.1} m: the lanes have stopped being lanes"
        );
        let close = at_most(&m.rays, CLOSE);
        assert!(
            close >= 0.40,
            "FOUNDRY fights inside {CLOSE} m in only {:.0}% of its sightlines",
            100.0 * close
        );
        assert!(
            m.depth.last().copied().unwrap_or(0.0) >= LONG,
            "FOUNDRY has lost the hall line that makes crossing it a decision"
        );
        let full = cover_count(&layout, Cover::Full);
        assert!(
            full >= 12,
            "FOUNDRY has {full} full-height pieces: the lanes no longer have walls"
        );
    }

    /// DOCKS is the marksman mission, and this is the arena the design pass
    /// changed. The long lane has to survive — it is what mission 3 is about —
    /// and the artery the player crosses to reach anything has to give a
    /// crouched body somewhere to be. Both are measured: raising the artery's
    /// two pallets to 1.15 m costs the map none of its standing sightlines,
    /// because a 1.15 m piece is under the 1.6 m sightline datum.
    #[test]
    fn docks_keeps_its_long_lane_and_arms_its_artery() {
        let layout = docks();
        let m = measure(&layout);
        assert!(
            m.depth.last().copied().unwrap_or(0.0) >= 45.0,
            "DOCKS's longest line has fallen under the marksman's"
        );
        let median = percentile(&m.rays, 0.50);
        assert!(
            median >= 10.0,
            "DOCKS's typical sightline is down to {median:.1} m: it has stopped being the open map"
        );
        let artery = cover_pieces(&layout)
            .iter()
            .filter(|p| p.class == Cover::Low && (p.min.x + p.max.x).abs() < 24.0)
            .count();
        assert!(
            artery >= 2,
            "DOCKS has {artery} low pieces in its artery: the boulevard is a firing range with no answer"
        );
    }

    /// BUNKER is the hold-out, and the numbers that make it one are the ones
    /// under the marksman's band: 32.6 m end to end, a position that sees 8.2 m
    /// at its farthest, and a median sightline of 5.2 m. Its racks are its
    /// crouch cover — 1.4 m, above the reference's 1.25 m band but exactly what
    /// a 1.03 m crouched eye needs.
    #[test]
    fn bunker_has_no_marksman_lane() {
        let layout = bunker();
        let m = measure(&layout);
        assert!(
            m.depth.last().copied().unwrap_or(0.0) < LONG,
            "BUNKER has grown a {LONG} m lane, which is what its mission has no marksman for"
        );
        let shut_in = m.depth.first().copied().unwrap_or(f32::MAX);
        assert!(
            shut_in <= 10.0,
            "BUNKER's most enclosed position is {shut_in:.1} m: two rooms can no longer be fought over unseen"
        );
        let median = percentile(&m.rays, 0.50);
        assert!(
            median <= 8.0,
            "BUNKER's typical sightline is {median:.1} m: the ring has opened up"
        );
        let racks = cover_count(&layout, Cover::Chest);
        assert!(
            racks >= 3,
            "BUNKER has {racks} racks left, and they are its crouch cover"
        );
    }

    /// CITADEL hands the player a nest and expects them to use it: the keep is
    /// standable, it is not AI-navigable ground (see the elevation test), and
    /// the courtyard around it is long enough to need one. The rest of the map
    /// is all wall — 21 full-height pieces and nothing between 1.0 and 1.75 m —
    /// so a fight here is either in the open or round a corner.
    #[test]
    fn citadel_hands_the_player_the_keep() {
        let layout = citadel();
        let level = Level::from_layout(&layout);
        let top = level.support_height(5.0, 5.0, 0.45, 2.4).unwrap_or(0.0);
        assert!(
            (top - 2.0).abs() < 0.01,
            "CITADEL's keep deck is at {top:.2} m, not the 2.0 m it was designed at"
        );
        assert!(
            level.point_free(vec3(5.0, 2.05, 5.0), 0.40, 1.75),
            "CITADEL's keep deck is no longer standable, so the nest does not exist"
        );
        assert!(
            nav_blocked(&level, 5.0, 5.0),
            "CITADEL's keep has become AI-navigable ground: the nest has been taken away"
        );
        let m = measure(&layout);
        assert!(
            m.depth.last().copied().unwrap_or(0.0) >= 45.0,
            "CITADEL's courtyard is too short to need a nest"
        );
        let full = cover_count(&layout, Cover::Full);
        assert!(
            full >= 12,
            "CITADEL has {full} full-height pieces: the curtain wall has gaps in it"
        );
    }

    /// R'LYEH has to be both worst cases at once: the shortest sightlines in the
    /// campaign (a position that sees 7.2 m) and the longest (70.0 m across the
    /// diagonal). Open the wings up and the labyrinth stops hiding anything;
    /// wall them off and the cathedral goes. Its one 1.0-1.25 m piece is an
    /// accident of a 1.2 m loose stone; the small trilithons are 0.6 m on
    /// purpose, so that bodies step over them instead of around them.
    #[test]
    fn rlyeh_is_a_labyrinth_and_a_cathedral_at_once() {
        let layout = rlyeh();
        let m = measure(&layout);
        let shut_in = m.depth.first().copied().unwrap_or(f32::MAX);
        assert!(
            shut_in <= 10.0,
            "R'LYEH's most enclosed position is {shut_in:.1} m: the wings have opened up"
        );
        let median = percentile(&m.rays, 0.50);
        assert!(
            median <= 10.0,
            "R'LYEH's typical sightline is {median:.1} m: it has stopped being a labyrinth"
        );
        assert!(
            m.depth.last().copied().unwrap_or(0.0) >= 55.0,
            "R'LYEH's longest line has shrunk below the cathedral it needs"
        );
        let full = cover_count(&layout, Cover::Full);
        assert!(
            full >= 20,
            "R'LYEH has {full} full-height pieces: the monoliths are gone"
        );
    }

    /// Not an assertion: the design report. `cargo test -- --ignored --nocapture`.
    ///
    /// Kept because every number in the doc comments above the six map
    /// functions comes from here, and a claim about geometry that cannot be
    /// re-measured goes stale the first time somebody moves a wall.
    #[test]
    #[ignore = "prints the six arenas' measurements; run with --ignored --nocapture"]
    fn show_the_design_report() {
        for layout in campaign() {
            let m = measure(&layout);
            let share = |d: f32| {
                100.0 * m.rays.iter().filter(|x| **x <= d).count() as f32 / m.rays.len() as f32
            };
            let count = |v: &[Piece], c: Cover| v.iter().filter(|p| p.class == c).count();
            println!("=== {} (half {} m) ===", layout.name_en, layout.half);
            println!(
                "  boxes {} | samples {} | spawns {}",
                layout.boxes.len(),
                m.points.len(),
                layout.spawns.len()
            );
            print!("  cover pieces:");
            for class in [
                Cover::Ankle,
                Cover::Kerb,
                Cover::Low,
                Cover::Chest,
                Cover::Full,
            ] {
                print!(
                    " {} {} ({} with treads)",
                    label(class),
                    count(&m.cover, class),
                    count(&m.all, class)
                );
            }
            println!();
            println!(
                "  longest sightline {:.1} | furthest visible point: min {:.1} median {:.1} p90 {:.1}",
                m.depth.last().copied().unwrap_or(0.0),
                m.depth.first().copied().unwrap_or(0.0),
                percentile(&m.depth, 0.50),
                percentile(&m.depth, 0.90)
            );
            println!(
                "  rays: median {:.1} p90 {:.1} max {:.1} | <=4.9 m {:.0}% | <=19.5 m {:.0}% | >=39 m {:.0}%",
                percentile(&m.rays, 0.50),
                percentile(&m.rays, 0.90),
                m.rays.last().copied().unwrap_or(0.0),
                share(4.9),
                share(19.5),
                100.0 - share(38.999),
            );
            print!("  nearest cover to each spawn (m):");
            for spawn in &layout.spawns {
                let d = m
                    .cover
                    .iter()
                    .map(|p| {
                        let cx = (p.min.x + p.max.x) * 0.5;
                        let cz = (p.min.y + p.max.y) * 0.5;
                        vec2(cx - spawn.x, cz - spawn.z).length()
                    })
                    .fold(f32::MAX, f32::min);
                print!(" {d:.1}");
            }
            println!();
            // `pick_spawn` rejects anything within 14 m of the player, and falls
            // back to spawn 0 when it rejects everything.
            let choice = |from: Vec3| {
                layout
                    .spawns
                    .iter()
                    .filter(|s| vec2(s.x - from.x, s.z - from.z).length() >= 14.0)
                    .count()
            };
            let worst = m.points.iter().map(|p| choice(*p)).min().unwrap_or(0);
            println!(
                "  spawns a body can be placed at 14 m+ from: worst position {worst} of {}",
                layout.spawns.len()
            );
            // Is any raised ground navigable? A platform is a solid, and the
            // grid blocks any cell whose footprint touches one, so the
            // expectation is that all elevation is player-only.
            let h = layout.half;
            let n = (2.0 * h / STRIDE).floor() as i32;
            let (mut raised, mut highest) = (0usize, 0.0f32);
            for ix in 0..=n {
                for iz in 0..=n {
                    let (x, z) = (-h + ix as f32 * STRIDE, -h + iz as f32 * STRIDE);
                    if nav_blocked(&m.level, x, z) {
                        continue;
                    }
                    if let Some(top) = m.level.support_height(x, z, 0.45, 2.4)
                        && top > 0.05
                    {
                        raised += 1;
                        highest = highest.max(top);
                    }
                }
            }
            println!(
                "  raised ground the AI can path onto: {raised} sample positions, highest {highest:.2} m"
            );
            if let Some(x) = layout.extraction {
                let start = vec3(
                    layout.player_start.x,
                    layout.player_start.y + EYE,
                    layout.player_start.z,
                );
                let end = vec3(x.x, x.y + EYE, x.z);
                println!(
                    "  extraction {:.1} m from the start, in sight from it: {}",
                    (end - start).length(),
                    visible(&m.level, start, end)
                );
            }
        }
    }
}
