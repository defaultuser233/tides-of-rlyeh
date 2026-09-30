//! Campaign arena layouts. All geometry is axis-aligned boxes so that collision,
//! navigation and rendering all derive from one source of truth.

use crate::level::{ARENA, Layout, WALL_H};
use crate::util::Rng;
use macroquad::prelude::*;

pub fn arena() -> Layout {
    let mut boxes: Vec<(Vec3, Vec3, Color)> = Vec::new();
    let mut rng = Rng::new(0x5EED_1234);

    let floor = Color::new(0.15, 0.16, 0.19, 1.0);
    let wall = Color::new(0.25, 0.25, 0.29, 1.0);
    let platform = Color::new(0.29, 0.29, 0.35, 1.0);
    let pillar = Color::new(0.27, 0.27, 0.32, 1.0);
    let cover = Color::new(0.23, 0.25, 0.28, 1.0);
    let crate_col = Color::new(0.42, 0.32, 0.20, 1.0);
    let accent = Color::new(0.74, 0.44, 0.12, 1.0);
    let rail = Color::new(0.35, 0.38, 0.44, 1.0);

    // Slight per-panel variation keeps the large flat surfaces from reading
    // as one solid colour.
    let mut vary = |c: Color| {
        let k = 0.94 + rng.f32() * 0.12;
        Color::new(c.r * k, c.g * k, c.b * k, c.a)
    };

    let mut b = |x0: f32, y0: f32, z0: f32, x1: f32, y1: f32, z1: f32, c: Color| {
        boxes.push((vec3(x0, y0, z0), vec3(x1, y1, z1), c));
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
    }
}

pub fn foundry() -> Layout {
    let mut boxes: Vec<(Vec3, Vec3, Color)> = Vec::new();
    let mut rng = Rng::new(0xF0_11_D2_0A);

    let floor = Color::new(0.16, 0.13, 0.12, 1.0);
    let wall = Color::new(0.30, 0.20, 0.16, 1.0);
    let machine = Color::new(0.36, 0.27, 0.21, 1.0);
    let pipe = Color::new(0.30, 0.24, 0.22, 1.0);
    let crate_col = Color::new(0.45, 0.32, 0.18, 1.0);
    let catwalk = Color::new(0.38, 0.30, 0.26, 1.0);
    let rail = Color::new(0.44, 0.36, 0.30, 1.0);
    let ember = Color::new(0.85, 0.38, 0.10, 1.0);

    let mut vary = |c: Color| {
        let k = 0.94 + rng.f32() * 0.12;
        Color::new(c.r * k, c.g * k, c.b * k, c.a)
    };

    let mut b = |x0: f32, y0: f32, z0: f32, x1: f32, y1: f32, z1: f32, c: Color| {
        boxes.push((vec3(x0, y0, z0), vec3(x1, y1, z1), c));
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
    }
}

pub fn docks() -> Layout {
    let mut boxes: Vec<(Vec3, Vec3, Color)> = Vec::new();
    let mut rng = Rng::new(0xD0_C5_5A_17);

    let floor = Color::new(0.13, 0.15, 0.18, 1.0);
    let wall = Color::new(0.22, 0.26, 0.30, 1.0);
    let crate_col = Color::new(0.28, 0.33, 0.34, 1.0);
    let steel = Color::new(0.30, 0.33, 0.36, 1.0);
    let paint_a = Color::new(0.20, 0.32, 0.38, 1.0);
    let paint_b = Color::new(0.26, 0.30, 0.36, 1.0);
    let paint_c = Color::new(0.18, 0.24, 0.30, 1.0);
    let accent = Color::new(0.35, 0.55, 0.62, 1.0);

    let mut vary = |c: Color| {
        let k = 0.94 + rng.f32() * 0.12;
        Color::new(c.r * k, c.g * k, c.b * k, c.a)
    };

    let mut b = |x0: f32, y0: f32, z0: f32, x1: f32, y1: f32, z1: f32, c: Color| {
        boxes.push((vec3(x0, y0, z0), vec3(x1, y1, z1), c));
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
    b(18.0, 0.0, 0.0, 19.5, 0.6, 1.5, vary(crate_col));
    b(-16.0, 0.0, 16.0, -14.5, 0.6, 17.5, vary(crate_col));
    b(0.0, 0.0, -16.0, 1.5, 0.6, -14.5, vary(crate_col));
    b(2.0, 0.0, 2.0, 3.5, 0.6, 3.5, vary(crate_col));
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
    }
}

pub fn bunker() -> Layout {
    let mut boxes: Vec<(Vec3, Vec3, Color)> = Vec::new();
    let mut rng = Rng::new(0xB0_0B_1E_5F);

    let floor = Color::new(0.13, 0.15, 0.13, 1.0);
    let wall = Color::new(0.22, 0.26, 0.23, 1.0);
    let core = Color::new(0.26, 0.30, 0.27, 1.0);
    let rack = Color::new(0.20, 0.24, 0.21, 1.0);
    let crate_col = Color::new(0.30, 0.32, 0.26, 1.0);
    let accent = Color::new(0.42, 0.52, 0.40, 1.0);

    let mut vary = |c: Color| {
        let k = 0.94 + rng.f32() * 0.12;
        Color::new(c.r * k, c.g * k, c.b * k, c.a)
    };

    let mut b = |x0: f32, y0: f32, z0: f32, x1: f32, y1: f32, z1: f32, c: Color| {
        boxes.push((vec3(x0, y0, z0), vec3(x1, y1, z1), c));
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

    // --- ring wall around the core: a 3.5 m corridor loop with 3 m blast doors ---
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
    }
}

pub fn citadel() -> Layout {
    let mut boxes: Vec<(Vec3, Vec3, Color)> = Vec::new();
    let mut rng = Rng::new(0xC1_7A_DE_11);

    let floor = Color::new(0.12, 0.10, 0.16, 1.0);
    let wall = Color::new(0.22, 0.18, 0.30, 1.0);
    let platform = Color::new(0.26, 0.21, 0.34, 1.0);
    let stair = Color::new(0.30, 0.24, 0.38, 1.0);
    let pillar = Color::new(0.24, 0.19, 0.31, 1.0);
    let crate_col = Color::new(0.34, 0.26, 0.42, 1.0);
    let accent = Color::new(0.62, 0.30, 0.72, 1.0);

    let mut vary = |c: Color| {
        let k = 0.94 + rng.f32() * 0.12;
        Color::new(c.r * k, c.g * k, c.b * k, c.a)
    };

    let mut b = |x0: f32, y0: f32, z0: f32, x1: f32, y1: f32, z1: f32, c: Color| {
        boxes.push((vec3(x0, y0, z0), vec3(x1, y1, z1), c));
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
    }
}

/// R'LYEH — the drowned city. Every piece of it is still an axis-aligned box;
/// the wrongness is entirely in the composition. Doorways never line up with
/// the opening beyond them, the colonnade's spacing follows no rule you can
/// learn, one trilithon motif repeats at three scales, and the middle of the
/// floor is an 18 x 18 hole with nothing under it.
pub fn rlyeh() -> Layout {
    let mut boxes: Vec<(Vec3, Vec3, Color)> = Vec::new();
    let mut rng = Rng::new(0x52_4C_59_45);

    // Cold and drowned: teal-grey stone, bone, wet purple, and a sick green
    // light that has no business being down here.
    let flag = Color::new(0.24, 0.30, 0.31, 1.0);
    let shell = Color::new(0.27, 0.33, 0.35, 1.0);
    let monolith = Color::new(0.31, 0.37, 0.39, 1.0);
    let drowned = Color::new(0.20, 0.26, 0.28, 1.0);
    let bone = Color::new(0.68, 0.67, 0.58, 1.0);
    let wet = Color::new(0.38, 0.22, 0.44, 1.0);
    let glow = Color::new(0.34, 0.84, 0.43, 1.0);

    let mut vary = |c: Color| {
        let k = 0.90 + rng.f32() * 0.18;
        Color::new(c.r * k, c.g * k, c.b * k, c.a)
    };

    let mut b = |x0: f32, y0: f32, z0: f32, x1: f32, y1: f32, z1: f32, c: Color| {
        boxes.push((vec3(x0, y0, z0), vec3(x1, y1, z1), c));
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
    }
}

/// The six campaign missions, in play order.
pub fn campaign() -> [Layout; 6] {
    [arena(), foundry(), docks(), bunker(), citadel(), rlyeh()]
}
