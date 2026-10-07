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

35 files, 1.15 MB total (1,156,225 B) in four groups: the six Kenney
one-shots that were kept (§2), the thirteen weapon and combat cues re-cut from
CC0 field recordings on 2026-10-03 (§2a), the nine mostly-WAV files found for
the effects that used to be synthesiser-only (§2b), and the seven surface and
range takes promoted out of `variants/` on 2026-10-04 and 2026-10-05 (§2c). Two
more takes were dropped rather than wired; that decision is §2d. Mono or
stereo, 44.1 kHz throughout except the three §2b Oggs (`healing`,
`pickup_health`, `mission_failed`), which are 48 kHz.

**They are named after the `Sfx::file_name()` slots in `src/audio.rs`**, so
`load_effect()` finds them automatically and they replace the built-in synth.
Nothing else needs wiring up.

From the **sci-fi pack** (S), **impact pack** (I) or **UI pack** (U); source
path is `assets/_downloads/<pack>/Audio/<stem>.ogg`. These six are unchanged.

| Copied file | Pack | Source stem | Len | ch | Engine slot |
| --- | :-: | --- | ---: | :-: | --- |
| `player_hurt.ogg` | I | `impactPunch_heavy_000` | 0.65 s | 2 | `Sfx::PlayerHurt` |
| `ui_click.ogg` | U | `click1` | 0.09 s | 2 | `Sfx::UiClick` |
| `ui_move.ogg` | U | `rollover1` | 0.23 s | 2 | `Sfx::UiMove` |
| `ui_back.ogg` | U | `click2` | 0.06 s | 2 | `Sfx::UiBack` |
| `explosion.ogg` | S | `explosionCrunch_000` | 0.78 s | 1 | `Sfx::Explosion` |
| `spawn.ogg` | S | `forceField_000` | 0.95 s | 1 | `Sfx::Spawn` |

### 2c. Seven more Kenney takes, promoted from `variants/`

These six were downloaded with the rest of the packs and parked in
`assets/sounds/variants/` — named after a *material* or a *range* rather than
after an engine slot, so nothing auto-loaded them. Four had no slot to load
into at all: the mixer had one `hit_world`, and which of them a round should
play is a question about the wall, which the level could not answer until
`Level` started keeping the material of every solid. The other two are the
ends of the explosion range.

| Copied file | Pack | Source stem | Len | Engine slot |
| --- | :-: | --- | ---: | --- |
| `impact_concrete.ogg` | I | `impactPlate_light_000` | 0.54 s | `Sfx::HitConcrete` |
| `impact_metal_light.ogg` | I | `impactMetal_light_000` | 0.35 s | `Sfx::HitMetal` |
| `impact_metal_heavy.ogg` | I | `impactMetal_heavy_000` | 0.17 s | `Sfx::HitMetalHeavy` |
| `impact_wood.ogg` | I | `impactPlank_medium_000` | 0.78 s | `Sfx::HitWood` |
| `explosion_large.ogg` | S | `explosionCrunch_003` | 1.55 s | `Sfx::ExplosionLarge` |
| `explosion_distant.ogg` | S | `lowFrequency_explosion_001` | 1.00 s | `Sfx::ExplosionDistant` |
| `impact_glass.ogg` | I | `impactGlass_medium_000` | 0.54 s | `Sfx::HitGlass` |

The choice is `impact_sfx()` in `src/main.rs`: the material of the solid the
round stopped in, with `Sfx::HitWorld` — the field recording of a shooting
gallery — as what a wall with no material of its own plays. `Sfx::Explosion` is
still the bang across the middle of the arena; `ExplosionLarge` takes over
within 12 m and `ExplosionDistant` past 35 m, where the crack never reaches the
player and the tail does.

### 2a. The thirteen weapon and combat cues re-cut from field recordings

The six gunshot slots were Kenney `laser*` material — "pew" synth beams — and
the hit, reload, footstep and death slots were Kenney impacts. All thirteen
were replaced with real recordings: **freesound.org**, licence-filtered to
`license:"Creative Commons 0"`, and every submission page re-checked by hand
(it renders `<a href="http://creativecommons.org/publicdomain/zero/1.0/">`
with the text `Creative Commons 0`; none of the thirteen carries CC-BY or
CC-BY-SA). Per-file evidence — source page, preview URL, author, the licence
string as the page shows it, the measurements and the header bytes — is in
`assets/SOUND_SOURCES.md`.

Each file is one isolated transient cut out of its source, downmixed to mono,
30 Hz high-passed, faded 3 ms in / 5 ms out and peak-normalised to ≈ −1 dBFS.
The five player guns are deliberately five *different* weapons so they can be
told apart with the eyes shut. Nothing here is synthesised, and no source was
music.

| Copied file | Source (freesound) | Original asset | Len | ch | Engine slot |
| --- | --- | --- | ---: | :-: | --- |
| `shot_rifle.ogg` | 710084 · felix.blume | `Rifle gunshot, one shot` | 0.50 s | 1 | `Sfx::ShotRifle` |
| `shot_shotgun.ogg` | 569309 · Kobrakon47 | `Saiga 12k Gunshot(12ga)` | 0.57 s | 1 | `Sfx::ShotShotgun` |
| `shot_smg.ogg` | OpenGameArt · Vincent Sevedge (CC **BY** 3.0) | `cz.wav` | 0.22 s | 1 | `Sfx::ShotSmg` |
| `shot_sniper.ogg` | 855607 · qubodup | `Sniper Shot in Field 2 (M2010 ESR)` | 0.68 s | 1 | `Sfx::ShotSniper` |
| `shot_lmg.ogg` | 854641 · qubodup | `M240 Machine Gun Single Shot` | 0.58 s | 1 | `Sfx::ShotLmg` |
| `enemy_shot.ogg` | 427597 · michorvath | `AR15 rifle shot from 50 yards away` | 0.65 s | 1 | `Sfx::EnemyShot` |
| `reload.ogg` | 815879 · qubodup | `Assault Rifle Reload` | 1.45 s | 1 | `Sfx::Reload` |
| `reload_empty.ogg` | 702662 · areniporgen | `Remington 870 pull up and dry fire` | 0.25 s | 1 | `Sfx::ReloadEmpty` |
| `hit_flesh.ogg` | 344409 · jawbutch | `Three Flesh Impacts.wav` | 0.38 s | 1 | `Sfx::HitFlesh` |
| `hit_head.ogg` | 703115 · franzzle | `crush1.wav` | 0.25 s | 1 | `Sfx::HitHead` |
| `hit_world.ogg` | 675951 · craigsmith | `S31-15 Shooting gallery; hitting metal targets` | 0.42 s | 1 | `Sfx::HitWorld` |
| `enemy_death.ogg` | 416839 · tonsil5 | `Grunt1 - Death Pain.wav` | 0.55 s | 1 | `Sfx::EnemyDeath` |
| `footstep.ogg` | 521590 · Fission9 | `Hiking Boot Footsteps on Stone` | 0.26 s | 1 | `Sfx::Footstep` |
| `shot_smg.ogg` (replaced again) | OpenGameArt · Vincent Sevedge | `cz.wav` | 0.22 s | 1 | `Sfx::ShotSmg` |

Twelve of the thirteen are mono Ogg Vorbis at 44.1 kHz, 6.0–16.7 KB each; the
whole swap is 110,885 B against 144,344 B for what it replaced, so
`assets/sounds/` got 33,459 B **smaller**. Every shot sits inside the 0.20–0.70 s
window the brief set (0.42–0.68 s) and every non-shot inside 1.5 s. None of the
thirteen has a sample at full scale; the hottest is `shot_shotgun.ogg` at
0.8911 (−1.00 dBFS).

**`shot_smg.ogg` was replaced a second time**, and is the one file in `assets/`
that is not CC0. The freesound take it had was the only one of the thirteen that
was not a recording of the weapon it plays: it is a twelve-gauge sped up by two,
because no CC0 single 9 mm shot exists. The OpenGameArt pack *Gunshot Sounds*
(CZ-52, Mosin, SKS, shotgun) has a real pistol crack in it, and its page says
CC0 — but its own `creativecommons.txt` says **Creative Commons Attribution 3.0
Unported**, © 2009 Vincent Sevedge. The bundle is the stricter and the more
specific statement, so that is the one recorded here and attributed in the
README. Only this one file was taken from it: the other three are recordings of
weapons the shipping set already has CC0 takes of.

The CZ-52 take is a field recording of five shots with a second of room on each.
The cut is the isolated shot at 2.82 s of the source, and its tail is pulled down
after the first 15 ms (`exp(-(t−0.015)/0.045)`): this weapon fires eighteen
rounds a second, so six copies of a one-second room tail would be summed at once
and the mixer, which has no ducking and no per-slot gain, would clip on the pile.
Unshaped, four overlapping copies peak at 2.5; shaped, at **0.92** — the same
headroom the take it replaced had at 0.88. See `assets/SOUND_SOURCES.md`.

### 2b. The nine curated non-Kenney files

These filled the slots that had no file, so they were playing the built-in
synthesis. All nine are **CC0**, verified on their own pages (the licence field
of each OpenGameArt page links to
`creativecommons.org/publicdomain/zero/1.0/`); the per-file evidence — source
page, direct URL, author, the licence string as the page renders it, and the
header bytes — is in `assets/SOUND_SOURCES.md`. The source archives
were not vendored (about 30 MB); the doc's direct links re-fetch each file.

| File | Source pack | Author | Len | Engine slot |
| --- | --- | --- | ---: | --- |
| `pickup_health.ogg` | 80 CC0 RPG SFX | rubberduck | 0.39 s | `Sfx::PickupHealth` |
| `pickup_ammo.wav` | Metal Impact Sounds | bmaczero | 0.26 s | `Sfx::PickupAmmo` |
| `objective_done.wav` | Metal Impact Sounds | bmaczero | 0.72 s | `Sfx::ObjectiveDone` |
| `mission_complete.wav` | Metal Impact Sounds | bmaczero | 2.11 s | `Sfx::MissionComplete` |
| `mission_failed.ogg` | 80 CC0 creature SFX | rubberduck | 1.13 s | `Sfx::MissionFailed` |
| `healing.ogg` | 50 RPG sound effects | Kenney | 0.66 s | `Sfx::Healing` |
| `elite_arrival.ogg` | Sci-Fi Sounds 1.0 (local) | Kenney | 2.00 s | `Sfx::EliteArrival` |
| `boss_arrival.ogg` | Their Coming | stumpystrust | 5.15 s | `Sfx::BossArrival` |
| `sanity_critical.wav` | Heartbeat sounds | bart | 2.25 s | `Sfx::SanityCritical` |

Two of these are longer than the brief asked for (`boss_arrival` 5.15 s against a
3.5 s ceiling, `mission_failed` 1.13 s against a 1.5 s floor) and one
(`objective_done`) sits at −0.02 dBFS, so it is the loudest effect in the table.
They were chosen for character rather than length; **none has been listened to**,
so any of them that reads wrong in play should be deleted — the slot falls back
to its recipe in `src/audio.rs`.

### The one slot deliberately left to the synth

`reload_done` has **no file on purpose**: it is the last effect still
synthesised, a two-part click-clack written for the frame a magazine seats, and
it is the one cue the game is happy to own outright. Its recipe is in
`src/audio.rs`; add `assets/sounds/reload_done.ogg` if a file is ever wanted.

The five that used to be listed here — `pickup_health`, `pickup_ammo`,
`objective_done`, `mission_complete`, `mission_failed` — now have real files and
moved to §2b. Their recipes remain as the fallback a missing or broken file
falls back to.

The four event cues the player has to place by ear (reference §19.4-10) live in
§2b too, except the last:

| Slot | File name | Event |
| --- | --- | --- |
| `Sfx::EliteArrival` | `elite_arrival` | an elite (the colossus) is released mid-mission |
| `Sfx::BossArrival` | `boss_arrival` | the same on a mission's last wave — lower and longer |
| `Sfx::SanityCritical` | `sanity_critical` | the mind crosses into the critical band |
| `Sfx::ReloadDone` | `reload_done` | the magazine is seated — synthesised, see above |

### 2d. Two takes that were dropped, and why they must not come back

These two were downloaded with the packs above, carried no slot, and are **no
longer in the tree**: on 2026-10-05 they were moved to
`assets/_downloads/rejected-takes/`, which is ignored by git. What is kept here is
the decision, because the reason not to wire them is the part that would be
expensive to rediscover.

| File | Pack | Source stem | Len | Why it must not be wired |
| --- | :-: | --- | ---: | --- |
| `footstep_concrete_02.ogg` | I | `footstep_concrete_002` | 0.11 s | a second *Kenney* footstep. `Sfx::Footstep` was deliberately re-cut to a field recording of boots on stone (§2a); this would put the library take back into the pool |
| `enemy_shot_alt.ogg` | S | `laserRetro_003` | 0.25 s | the same story: §2a replaced the enemy gunshot's Kenney `laser*` material with a real recording, and this is one of the "pew" beams it was replaced *for* |

**Do not bring these two back** as extra voices for their slots. They are the
material those two slots were deliberately moved off (§2a); a pool that plays
them some of the time is that decision undone. They were left out of the tree
rather than merely documented out of it, because a take sitting in
`assets/sounds/` reads as one that is shipped — every other file in that
directory is compiled into the executable, and a directory that is one thing for
35 files and another thing for two is how the next person wires the wrong sound.

The other two takes that were parked alongside these are both accounted for:
`explosion_deep.ogg` was byte-identical to `elite_arrival.ogg` and was deleted,
and `impact_glass.ogg` is now `Sfx::HitGlass` in §2c — a molotov is what breaks.

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

### How they are wired up

All six are in the game now. `src/surface.rs` pairs each file with the
`Surface` variant that names it; a level block carries a `Skin` (a colour and
its surface), `Level::from_layout` batches geometry into one mesh per surface,
and `MeshBuilder::set_uv_scale` generates UVs from each face's own edges, so a
20 m wall and a 1 m crate tile at the same density.

Three facts from the vendored source shape that code, and each one is a trap
that costs an afternoon if it is missed:

- The 3D shader is `color * texture2D(Texture, uv)`
  (`macroquad-0.4.16/src/quad_gl.rs:1060-1068`), so a block's palette colour
  *multiplies* the photo. `Skin::uniform` divides the texture's measured mean
  back out per channel (`Surface::mean`, which
  `the_mean_table_matches_the_textures` re-derives from the PNG), and that is
  what keeps an arena the colour its palette says it is instead of whatever hue
  the photo happens to average.
- A vertex colour cannot exceed 1, so a palette colour brighter than its
  texture's mean cannot be rendered at the brightness asked for — the level
  just goes dim, one material at a time.
  `every_block_can_be_painted_at_its_palette_brightness` pins that at 15% and
  names the pairings that have to change.
- miniquad creates textures with `TextureWrap::Clamp`
  (`miniquad-0.4.11/src/graphics.rs:405-418`) and macroquad exposes no setter
  for it, so `Textures::load` reaches through `get_internal_gl()` to set
  `Repeat`. Left clamped, every UV past 1.0 samples the edge texel and smears
  it into stripes across the whole face.

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

`assets/_downloads/` is 63 MB (the five zips plus their extractions, kept so
curation can be re-run or widened). `.gitignore` excludes it, along with
`/target` and the font download cache, so a clone carries only the curated
subset in `models/`, `sounds/` and `textures/`.
