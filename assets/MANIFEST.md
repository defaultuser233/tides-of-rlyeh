# Asset manifest

Everything under `assets/models/`, `assets/sounds/` and `assets/textures/` is
**CC0 (public domain)**. Sources: Kenney (kenney.nl) and Poly Haven
(polyhaven.com). No attribution is required; it is given here anyway.

Only the curated files are listed. The original archives and their full
extractions are still in `assets/_downloads/` (61.8 MB — see the note at the
end) for re-curation.

| Source pack | License | Evidence |
| --- | --- | --- |
| Kenney Blaster Kit 2.1 | CC0 1.0 | `_downloads/kenney_blaster-kit_2.1/License.txt` — "License: (Creative Commons Zero, CC0)" |
| Kenney Blocky Characters 2.0 | CC0 1.0 | `_downloads/kenney_blocky-characters_20/License.txt` — same wording |
| Kenney Impact Sounds 1.0 | CC0 1.0 | `_downloads/kenney_impact-sounds/License.txt` — same wording |
| Kenney Sci-Fi Sounds 1.0 | CC0 1.0 | `_downloads/kenney_sci-fi-sounds/License.txt` — same wording |
| Kenney UI SFX Set | CC0 1.0 | `_downloads/kenney_ui-audio/License.txt` — same wording |
| Poly Haven textures | CC0 1.0 | https://polyhaven.com/license — "Our assets are all licensed as CC0" |

---

## 1. Models — `assets/models/`

Wavefront `.obj` + `.mtl`, all triangulated, with `v`/`vt`/`vn` present.
Every `.mtl` `map_Kd` reference was checked to resolve on disk; each model
folder carries the pack's own `Textures/` subfolder so the reference
`Textures/<name>.png` works verbatim without editing the `.mtl`.

### `assets/models/weapons/` — 5 models, one per `WeaponKind`

All from **Kenney Blaster Kit 2.1**, texture `Textures/colormap.png`
(10,223 B, shared by all five).

| File | Game role (`src/weapon.rs`) | v | vt | vn | tris |
| --- | --- | ---: | ---: | ---: | ---: |
| `blaster-a.obj` | `WeaponKind::Rifle` | 298 | 62 | 84 | 880 |
| `blaster-b.obj` | `WeaponKind::Lmg` | 230 | 68 | 90 | 736 |
| `blaster-h.obj` | `WeaponKind::Shotgun` | 208 | 43 | 84 | 592 |
| `blaster-j.obj` | `WeaponKind::Smg` | 344 | 73 | 117 | 1036 |
| `blaster-p.obj` | `WeaponKind::Sniper` | 524 | 96 | 149 | 1704 |

Each has a sibling `.mtl` (98 B). Roles were assigned from the rendered
previews: `-a` long rifle, `-b` bulky launcher silhouette, `-h` stubby
barrel, `-j` compact, `-p` long with a large scope.

### `assets/models/characters/` — 6 humanoids

From **Kenney Blocky Characters 2.0**. Each is a Minecraft-style rig with the
`g` groups `leg-left`, `leg-right`, `torso`, `arm-left`, `arm-right`, `head`,
so limbs can be animated independently.

| File | Suggested role (`src/enemy.rs`) | v | vt | vn | tris | texture |
| --- | --- | ---: | ---: | ---: | ---: | --- |
| `character-b.obj` | `Kind::Grunt` | 44 | 143 | 19 | 72 | `Textures/texture-b.png` |
| `character-c.obj` | `Kind::Rusher` | 42 | 143 | 19 | 72 | `Textures/texture-c.png` |
| `character-h.obj` | `Kind::Heavy` | 44 | 143 | 19 | 72 | `Textures/texture-h.png` |
| `character-j.obj` | `Kind::Sniper` | 44 | 143 | 19 | 72 | `Textures/texture-j.png` |
| `character-q.obj` | spare / variant | 44 | 143 | 19 | 72 | `Textures/texture-q.png` |
| `character-r.obj` | spare / variant | 44 | 143 | 19 | 72 | `Textures/texture-r.png` |

`Kind::Drone` has no model here — all 18 characters in the pack are
humanoids, so a drone has to stay procedural (the game already builds meshes
by hand in `src/mesh.rs`).

### `assets/models/props/` — 3 crates

From **Kenney Blaster Kit 2.1**, shared texture `Textures/colormap.png`.
Useful as arena cover; `src/level.rs` already works in AABBs.

| File | v | vt | vn | tris |
| --- | ---: | ---: | ---: | ---: |
| `crate-small.obj` | 354 | 38 | 55 | 1000 |
| `crate-medium.obj` | 354 | 38 | 55 | 1000 |
| `crate-wide.obj` | 354 | 38 | 55 | 1000 |

---

## 2. Sounds — `assets/sounds/`

19 files, 220 KB total. All Ogg Vorbis 44.1 kHz, mono or stereo.

**They are named after the `Sfx::file_name()` slots in `src/audio.rs`**, so
`load_effect()` finds them automatically and they replace the built-in synth.
Nothing else needs wiring up.

From the **sci-fi pack** (S), **impact pack** (I) or **UI pack** (U); source
path is `assets/_downloads/<pack>/Audio/<stem>.ogg`.

| Copied file | Pack | Source stem | Len | ch | Engine slot |
| --- | :-: | --- | ---: | :-: | --- |
| `shot_rifle.ogg` | S | `laserSmall_000` | 0.24 s | 2 | `Sfx::ShotRifle` |
| `shot_shotgun.ogg` | S | `laserLarge_000` | 0.68 s | 1 | `Sfx::ShotShotgun` |
| `shot_smg.ogg` | S | `laserSmall_002` | 0.34 s | 2 | `Sfx::ShotSmg` |
| `shot_sniper.ogg` | S | `laserLarge_004` | 0.69 s | 1 | `Sfx::ShotSniper` |
| `shot_lmg.ogg` | S | `laserRetro_002` | 0.26 s | 1 | `Sfx::ShotLmg` |
| `enemy_shot.ogg` | S | `laserRetro_000` | 0.24 s | 1 | `Sfx::EnemyShot` |
| `hit_flesh.ogg` | I | `impactPunch_medium_000` | 0.43 s | 2 | `Sfx::HitFlesh` |
| `hit_world.ogg` | I | `impactPlate_medium_000` | 0.61 s | 2 | `Sfx::HitWorld` |
| `hit_head.ogg` | I | `impactMetal_light_001` | 0.25 s | 2 | `Sfx::HitHead` |
| `enemy_death.ogg` | I | `impactSoft_heavy_003` | 0.54 s | 2 | `Sfx::EnemyDeath` |
| `player_hurt.ogg` | I | `impactPunch_heavy_000` | 0.65 s | 2 | `Sfx::PlayerHurt` |
| `reload.ogg` | U | `switch4` | 0.42 s | 2 | `Sfx::Reload` |
| `reload_empty.ogg` | U | `click3` | 0.09 s | 2 | `Sfx::ReloadEmpty` |
| `footstep.ogg` | I | `footstep_concrete_000` | 0.11 s | 2 | `Sfx::Footstep` |
| `ui_click.ogg` | U | `click1` | 0.09 s | 2 | `Sfx::UiClick` |
| `ui_move.ogg` | U | `rollover1` | 0.23 s | 2 | `Sfx::UiMove` |
| `ui_back.ogg` | U | `click2` | 0.06 s | 2 | `Sfx::UiBack` |
| `explosion.ogg` | S | `explosionCrunch_000` | 0.78 s | 1 | `Sfx::Explosion` |
| `spawn.ogg` | S | `forceField_000` | 0.95 s | 1 | `Sfx::Spawn` |

### Six slots deliberately left to the synth

`pickup_health`, `pickup_ammo`, `pickup_weapon`, `objective_done`,
`mission_complete`, `mission_failed` have **no file here on purpose**. Their
synth recipes in `src/audio.rs` are purpose-written musical stings (rising
pickup arpeggios; a C-E-G-C major figure for mission complete; an A-G-F-C
descending minor for failure). The Kenney packs contain nothing comparable, so
dropping a UI blip into those slots would be a downgrade. Add files with those
names only if you want to override them.

### `assets/sounds/variants/` — 10 extra one-shots, not auto-loaded

The engine has a single `hit_world` slot, so it cannot vary by surface. These
are named *after a surface, not after a slot*, so nothing here is picked up by
accident. Optional material for a future surface-aware impact system.
10 files, 140 KB.

| File | Pack | Source stem | Len |
| --- | :-: | --- | ---: |
| `impact_metal_light.ogg` | I | `impactMetal_light_000` | 0.35 s |
| `impact_metal_heavy.ogg` | I | `impactMetal_heavy_000` | 0.17 s |
| `impact_concrete.ogg` | I | `impactPlate_light_000` | 0.54 s |
| `impact_wood.ogg` | I | `impactPlank_medium_000` | 0.78 s |
| `impact_glass.ogg` | I | `impactGlass_medium_000` | 0.54 s |
| `footstep_concrete_02.ogg` | I | `footstep_concrete_002` | 0.11 s |
| `explosion_large.ogg` | S | `explosionCrunch_003` | 1.55 s |
| `explosion_deep.ogg` | S | `lowFrequency_explosion_000` | 2.00 s |
| `explosion_distant.ogg` | S | `lowFrequency_explosion_001` | 1.00 s |
| `enemy_shot_alt.ogg` | S | `laserRetro_003` | 0.25 s |

---

## 3. Textures — `assets/textures/`

6 tileable surface PNGs, RGB 512×512, 2.6 MB total. Fetched from the Poly
Haven API as 1k PNGs, then LANCZOS-downscaled to 512 and re-encoded
(`optimize=True`): the originals are 1024×1024 and ~5 MB each, which is far
more than a tiled arena surface needs.

| File | Poly Haven asset | Original URL | Role |
| --- | --- | --- | --- |
| `metal_plate.png` | `metal_plate` | `dl.polyhaven.org/file/ph-assets/Textures/png/1k/metal_plate/metal_plate_diff_1k.png` | painted steel panel |
| `metal_rusty.png` | `rusty_metal_02` | `.../png/1k/rusty_metal_02/rusty_metal_02_diff_1k.png` | rusted metal |
| `metal_corrugated.png` | `corrugated_iron` | `.../png/1k/corrugated_iron/corrugated_iron_diff_1k.png` | corrugated wall |
| `concrete_floor.png` | `concrete_floor` | `.../png/1k/concrete_floor/concrete_floor_diff_1k.png` | floor |
| `concrete_tiles.png` | `concrete_tiles_02` | `.../png/1k/concrete_tiles_02/concrete_tiles_02_diff_1k.png` | tiled concrete |
| `crate_side.png` | `container_side` | `.../png/1k/container_side/container_side_diff_1k.png` | shipping-container / crate |

Only the diffuse map was taken. Normal/roughness/AO exist upstream under the
same asset IDs if they are ever needed.

Diffuse **JPG would not work**: macroquad 0.4.16 builds `image` with
`default-features = false, features = ["png", "tga"]`, so `load_texture`
accepts PNG and TGA only.

---

## 4. Format constraints, as actually verified

Read out of the vendored crate source, not assumed:

- **Models** — macroquad **0.4.16 has no model file loader at all**.
  `macroquad::models` exposes only procedural helpers (`draw_cube`,
  `draw_sphere`, `draw_cylinder`, …) plus the `Mesh` struct; there is no
  `load_model` and no OBJ parser anywhere in the crate, and no feature flag
  enables one. See `src/models.rs` (730 lines, no `load_model`).
  **Consequence: the `.obj` files above are staged and validated, but nothing
  loads them yet.** Using them needs a new dependency (`tobj`) or a small
  hand-written parser feeding `Mesh { vertices, indices, texture }` — which
  pairs naturally with the existing `MeshBuilder` in `src/mesh.rs`. The files
  were chosen to make that easy: fully triangulated, with `v`/`vt`/`vn`.
- **Textures** — PNG/TGA only, per the `image` feature list above.
- **Audio** — `quad-snd` 0.2.8 depends on `audrey` with
  `features = ["wav", "ogg_vorbis"]`, `default-features = false`. So `.ogg`
  (Vorbis) and `.wav` are the only decodable formats.

---

## 5. Reproducing / re-verifying

```bash
python assets/_tools/curate.py       # re-copy from _downloads into models/sounds/textures
python assets/_tools/objcheck.py assets/models   # v/vt/vn/tri counts, exits 2 on a bad file
```

`_tools/` is scratch tooling, not game content — delete both scripts if you
would rather not keep them.

## 6. Note on `_downloads/`

`assets/_downloads/` is 61.8 MB (the five zips plus their extractions, kept
so curation can be re-run or widened). `.gitignore` currently only lists
`/target`, so this directory would otherwise be committed. Consider adding
`assets/_downloads/` to `.gitignore`.
