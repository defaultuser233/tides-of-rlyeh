# STEEL FRONT / 钢铁前线

A single-player campaign FPS written in Rust, built on [macroquad](https://macroquad.rs).
Six hand-built missions, five weapons, five enemy archetypes and three cosmic
horrors, a bilingual (中文 / English) interface, and no launcher, engine install
or runtime to deploy.

- **Windows**: a single `rust-fps.exe`
- **Linux**: a single statically linked ELF that runs on any x86_64 distro

---

## Controls

### Movement and stance

| Action | Key |
| --- | --- |
| Move | `W` `A` `S` `D` |
| Jump | `Space` |
| Sprint | `Left Shift` — a tap latches it, and it drops on its own the moment you stop pushing forward or raise the sights; there is nothing to hold |
| Crouch (toggle) | `C` |
| Prone (toggle) | `Z` |
| Walk quietly | Hold `Caps Lock` |
| Lean left / right | `Q` / `E` — a tap latches the lean, another tap drops it; switch to hold under Settings → LEAN MODE |

Aiming and leaning share one rule: each has its own `HOLD` / `TOGGLE` option
under Settings, and both default to toggle so the two never disagree. Sprinting
drops a latched lean, because otherwise a toggled lean quietly blocks sprinting
until you notice why.

Stance changes more than the camera height. It changes the collision box, so
crouching really does present a smaller target; it changes speed; it changes
weapon sway and spread; and it changes how far away enemies can spot you
(standing 1.0, crouched 0.72, prone 0.45, and walking multiplies that by 0.82).

You cannot stand up under a low ceiling — the game checks headroom and keeps you
down until there is room.

Movement keeps the Quake/Source shape — an acceleration rate rather than an
acceleration cap, so one second of held input adds a fixed fraction of the speed
cap to the velocity's projection on the wish direction — but the cap itself
binds everywhere: on the ground, in the air, and along the way. Ground
acceleration is 15 /s (the old 95 m/s², at walking speed), air control is
deliberately weak (2.6 /s), and the speed a stance and weapon allow is a
ceiling, not a suggestion: the velocity is clamped after the acceleration, so a
hop cannot bank speed the legs never had and holding the jump key is not a
faster way to travel. Letting go of the keys brakes through friction (20 /s on
the ground, 6 /s in the air) and `STOP_SPEED` keeps the last two metres per
second from turning the stop into a crawl: a released run at walking speed
slides 0.23 m and is over in a tenth of a second, a sprint 0.34 m. The model is
integrated in fixed 1/120 s substeps, at most eight per frame, so a frame that
stalls degrades the step instead of spiralling, because these constants stop
meaning the same thing at a different step length. Setting `AIR_FRICTION` to
zero re-opens the air — a strafe jump then accelerates past the cap again, which
is the one thing this game deliberately does not want.

### Looking

| Action | Key |
| --- | --- |
| Look | Mouse |
| Free look (turn head without turning the body) | Hold middle mouse |
| Hold breath (steady the sights) | Hold `Left Alt` while aiming |

Free look is for checking a corner while keeping your weapon and your movement
facing one direction. Holding breath almost erases aim sway, but the meter in
`player.rs` drains in about three seconds and running it dry makes you *less*
steady than not aiming at all.

### Weapons

| Action | Key |
| --- | --- |
| Fire | Left mouse |
| Aim down sights | Right mouse — **toggles** by default; switch to hold under Settings → AIM MODE |
| Switch slot | `1` primary · `2` secondary · `3` sidearm · `4` melee, or mouse wheel |
| Reload | `R` |
| Cycle fire mode (auto / burst / single) | `B` |
| Check fire mode | `Alt`+`B` |
| Check magazine (approximate) | `Alt`+`T` |
| Inspect weapon | `U` |

Firing modes are per-weapon: the rifle and SMG offer auto, burst and single;
the sniper and sidearm are single only; the shotgun is single only.

A magazine check deliberately reports a *band* — full, mostly full, about half,
running low, empty — rather than an exact count. It also prints the real number
next to it, so the honest answer is one glance away.

Recoil is a learnable pattern rather than a random kick. The burst is counted:
shots 1–3 climb evenly, 4–7 climb harder with each shot, 8–10 walk up and to
the left, and 11–17 hand over to a sideways walk that plateaus; a gap of 0.25 s
without firing resets the count, so trigger discipline is rewarded. The muzzle
only starts falling back 0.05 s after the last shot, which is what lets a burst
actually climb instead of being flattened to the first shot's kick. Pulling
against the climb with the mouse cancels the accumulated recoil by the amount
pulled — capped at what has accumulated, so you can cancel the climb but never
invert it — because otherwise the automatic fallback would stack on top of your
correction and end the burst aiming below where it began.

### Equipment

| Action | Key |
| --- | --- |
| Throw grenade | `G` |
| Grenade wheel | Hold `G`, aim with the mouse, release to throw |
| Quick heal | `V` |
| Medical wheel | Hold `V`, aim with the mouse, release to apply |

Grenades are physical: they arc, bounce off geometry with restitution and
friction, and run on a fuse. The frag has a 3.2 s fuse and a 7 m lethal radius
that respects line of sight, so cover works. The smoke deploys on contact and
occludes enemy sight for 16 seconds.

Healing channels for 1.2–2.5 s and is interrupted by taking damage, by firing,
or by moving faster than a walk — you cannot patch up mid-sprint.

### Screens

| Action | Key |
| --- | --- |
| Inventory | `Tab` |
| Map | `M` |
| Mission panel | `J` |
| How to play (the manual's first page) | `H` |
| Codex — hostiles, weapons and kit | `K` |
| Next / previous manual page | `Left` / `Right` |
| Close a screen | `Esc` or the same key again |
| Pause | `Esc` |
| Settings — mouse, volume, fullscreen, resolution, language | Main menu |

The map only plots hostiles that are currently aware of you, so it does not
double as a wallhack.

Settings are the one screen that is not reached by a key: they hang off the main
menu, and every row saves itself the moment it changes — see *Settings* below.

### The field manual

`H` and `K` open a three-page manual, and both are also on the main menu, which
is where a new player meets them before ever holding a gun. Page one is how the
game is played: the objective (clear the arena, then reach extraction — there is
no mid-mission save), every binding grouped by what it is for, and the survival
notes that are not obvious from a key list (armour soaks 65% of a hit, nothing
shoots you without line of sight, sanity is drained by what you look at rather
than by what hits you, sprint is a tap rather than a hold).

Page two is the bestiary: one card per archetype, carrying its own model as a
portrait and the numbers the simulation actually uses — health, speed, damage,
range and rate are read out of `enemy::stats` when the page is built, so a
balance change cannot leave the manual lying. The tactic line under each card is
derived from those numbers rather than invented. Page three does the same for
the seven weapons (damage, rate in rounds per minute, magazine, reload, spread
and recoil) and the kit.

The whole thing is built when a page is opened and dropped when it closes, so
the drawing path allocates nothing, and the world is frozen behind it like the
other screens.

---

## Settings

The settings screen is on the main menu, and every row saves itself the moment it
changes: there is no Apply button and nothing to confirm.

| Row | Values |
| --- | --- |
| MOUSE SENSITIVITY | 0.30 – 5.00 in steps of 0.10 |
| MASTER VOLUME | 0 – 100% in steps of 5% |
| AIM MODE | Toggle or hold, for aiming down the sights |
| LEAN MODE | Toggle or hold, for leaning on `Q` / `E` |
| INVERT MOUSE Y | On / off |
| SHOW FPS | On / off |
| FULLSCREEN | On / off — borderless fullscreen, applied immediately |
| RESOLUTION | 1280 × 720, 1600 × 900, 1920 × 1080, 2560 × 1440 |
| LANGUAGE | 中文 / English |

`←` and `→` change the selected row, `↑` and `↓` move between rows, and the last
row, BACK, returns to the menu on `Enter` or a click. The mouse works on every
menu in the game and does not fight the keyboard: moving the cursor onto a row
selects that row, but a cursor that is merely *parked* over one leaves the
selection to the arrow keys — otherwise the highlight would snap back under the
pointer on the next frame. Clicking a settings row steps its value, exactly as
`→` does; there is no Apply button anywhere, so a click means "change this".

The resolution row walks its presets and wraps at both ends, and it only touches
the window while the game is windowed: in fullscreen the display decides the
size, so the row is dimmed and the choice is kept for the moment the game returns
to a window. A size in the settings file that is not one of the presets — a
hand-edit, or one written by an older build — is carried in the list as an extra
entry rather than snapped to the nearest preset, so nothing quietly rewrites it.

### The settings file

Settings persist between runs, in one small text file per user:

| Platform | Path |
| --- | --- |
| Windows | `%APPDATA%\steel-front\config` |
| Linux | `$XDG_CONFIG_HOME/steel-front/config`, or `~/.config/steel-front/config` when `XDG_CONFIG_HOME` is unset |

The format is `key = value`, one per line, with `#` comments — the file is meant
to be readable and editable by hand:

```text
# STEEL FRONT settings. Delete this file to go back to the defaults.
fullscreen = false
width = 1280
height = 720
sensitivity = 1.6
volume = 0.8
aim_toggle = true
lean_toggle = true
invert_y = false
show_fps = false
lang = zh
unlocked = 1
last_mission = 0
```

Unknown keys are ignored, so a file written by a newer build still loads, and a
line that cannot be read costs its own setting and nothing else: the rest of the
file is kept. **Delete the file and the next launch is a fresh install** —
defaults everywhere, with no other state to clear. Nothing is ever written next
to the executable, because a game that ships as one file has to run from
anywhere; on a machine with none of the directories above it runs on its defaults
and saves nothing rather than guessing at a location.

The last two keys are the campaign: `unlocked` is how many missions the player
has reached and `last_mission` is the one they were in. Every mission start
writes them, so **continue** on the main menu resumes exactly where the game was
left rather than at the chapter select; deleting the file starts the campaign
over. `unlocked = 400` in a hand-edited file is clamped rather than believed.

---

## Requirements

- Rust 1.85+ (`edition = "2024"`; declared as `rust-version` in `Cargo.toml`,
  developed against 1.98). `rustup` recommended
- **Windows build**: nothing beyond the MSVC toolchain
- **Linux cross-build from Windows**: nothing beyond `rustup` — see below
- **Linux native build**: a C toolchain plus `pkg-config`
  - Debian/Ubuntu: `sudo apt install build-essential pkg-config libasound2-dev libx11-dev libgl1-mesa-dev`
  - Fedora: `sudo dnf install gcc pkgconf-pkg-config alsa-lib-devel libX11-devel mesa-libGL-devel`

---

## Building

### Windows

```powershell
.\build-windows.ps1
```

Produces `target\x86_64-pc-windows-msvc\release\rust-fps.exe` (~4.2 MiB).

The Chinese font and the curated sound effects are **compiled into the
executable**, so it is genuinely a single file: copy it anywhere and it runs.
Nothing needs to sit beside it, and the working directory does not matter.

### Linux, cross-compiled from Windows

```powershell
.\build-linux.ps1
```

Produces `target\x86_64-unknown-linux-musl\release\rust-fps`, a **statically
linked x86_64 ELF** (~4.0 MiB). No glibc, no shared libraries, no X11 development
packages needed on the target: macroquad resolves X11 and OpenGL at runtime with
`dlopen`, so the binary needs nothing but a desktop session.

The script locates the `rust-lld` that already ships inside the Rust sysroot and
uses it as the linker — no `zig`, no `wsl`, no Linux toolchain of any kind.

**All audio is absent from this build**, and that is a real limitation rather than an
oversight. The audio stack is `quad-snd` → `quad-alsa-sys`, whose `build.rs`
emits `cargo:rustc-link-lib=dylib=asound`. Linux distributions ship ALSA as a
shared object (`libasound.so.2`) and generally not as a static library, so a
fully static link has no `-lasound` to resolve and fails with
`unable to find library -lasound`.
Static + ALSA is not a viable combination. Cross-compiled Linux builds are
therefore **completely silent, and carry no sound data at all** — the
`--no-default-features` switch disables the `sound` feature, which gates both
playback and the `include_bytes!` table of Kenney effects. A byte scan of the two
artefacts shows 85 `OggS` headers in the Windows executable and 0 in the Linux
ELF. Build natively on Linux for sound.

### Linux, native

```sh
./build.sh
```

Produces `target/release/rust-fps` with **full audio**.

---

## Campaign

Six missions. Progress is tracked in memory; clearing a mission unlocks the next.

| # | Arena | Objective | Introduces |
| --- | --- | --- | --- |
| 1 | RUST YARD / 废土竞技场 | Eliminate all hostiles | Grunts, Stalkers |
| 2 | FOUNDRY / 铸造厂 | Eliminate, then extract | Juggernauts, second weapon slot |
| 3 | DOCKS / 码头 | Eliminate, then extract | Marksmen, long sightlines |
| 4 | BUNKER / 地堡 | Hold out for 120 s | Close quarters, Stalker swarms |
| 5 | CITADEL / 要塞 | Eliminate, then extract | Drones, all five enemy types, sniper rifle |
| 6 | R'LYEH / 拉莱耶 | Eliminate, then extract | Tentacles, Colossus; a bottomless pit |

R'LYEH is the largest arena and the only one with a `voids` entry: an 18 x 18 m
pit in the middle of the floor, built as a gap between four floor slabs. The
navigation grid marks it impassable, so nothing walks into it, and anything that
falls past the floor line inside it is gone.

### Arenas

The layouts are measured, not described from intent: `maps.rs`'s ignored design
report (`cargo test -- --ignored --nocapture`) re-traces each arena's sightlines,
cover bands and engagement scales against the real geometry, and the tests beside
it pin the numbers. RUST YARD is an open ring that teaches the 1.0–1.25 m cover
band; FOUNDRY is a three-lane machine hall whose one long line is the 46.8 m
central hall, with no low cover at all; DOCKS is the long-range arena — a 59.1 m
boulevard and the campaign's deepest sightlines; BUNKER is a single
close-quarters ring, short enough (32.6 m end to end) that no marksman appears in
its spawn table; CITADEL is a defended base whose raised keep is the campaign's
one sniper nest; R'LYEH puts the campaign's longest diagonal (70 m) beside dead
ends that see 7.2 m, around a bottomless pit. One measurement holds in every
arena: the navigation grid blocks any cell whose footprint touches a solid, so
the highest ground an AI can path onto anywhere is 0.60 m and every perch
belongs to the player.

The pressure curve is flat inside a mission and climbs between them. A mission's
own waves hold a plateau, and the escalation comes from the campaign ladder —
mission 6's plateau is more than 3× mission 1's, which
`the_campaign_is_where_the_peak_lives` asserts. The known design gap is that there
is no pressure knob *inside* a level: a mission's spawner has only wave strength,
`max_alive` and the total budget, so what varies within one arena is what spawns,
not how hard the mission presses. That limit is written down beside the curve in
`campaign.rs` rather than left implicit.

### Weapons

| Weapon | Magazine | Damage | Character |
| --- | ---: | ---: | --- |
| MK-7 RIFLE / MK-7 突击步枪 | 30 | 24 | Balanced full-auto, 2.4× headshot |
| BREACHER 12 / 破门者-12 | 6 | 15 × 9 | Shotgun, brutal up close, falls off hard |
| VIPER SMG / 毒蛇冲锋枪 | 40 | 14 | 18 rounds/sec, high bloom |
| LONGSHOT .50 / 远程 .50 | 5 | 115 | 16° scope, no damage falloff |
| VULCAN LMG / 火神轻机枪 | 100 | 21 | Sustained fire, slows you down |

### Enemies

| Enemy | Behaviour |
| --- | --- |
| GRUNT / 步兵 | Balanced ranged infantry |
| STALKER / 突袭者 | Fast melee charger |
| JUGGERNAUT / 重装兵 | Slow, armoured, three-round bursts |
| MARKSMAN / 射手 | Long range; **telegraphs** its shot with a 0.95 s aim-up |
| DRONE / 无人机 | Hovering flier at ~2.9 m, fast and erratic |

Every enemy kind — the five archetypes above and the three cosmic horrors — is
rebuilt and animated every frame. The four humanoids walk through a rigged cycle
(hip and knee swing half a cycle apart, counter-swinging arms, a body lean that
pivots at chest height so the head walks a straight line); the Drone's rotors
turn whether or not it is translating; the Tentacle's arms crawl a beat apart;
the Spawnling scuttles on its legs and the Colossus strides. The standing pose is
the pose the static prototype had, so the silhouettes and the hitboxes the arenas
were measured against are unchanged. The frame a foot lands is reported once per
step, and the game hangs a small ground-hugging dust puff off it.

Enemies navigate with a **BFS flow field** rebuilt four times a second toward the
player, so they path around cover instead of pressing into walls. They keep squad
separation, hold archetype-specific standoff distances, and need line of sight to
shoot. Accuracy degrades with distance.

The field follows the player's *current* position, which is right for a unit that
has never seen them — a reinforcement works its way in until it arrives. A unit
that has actually had eyes on the player does not use the field that way: it
searches, walking back to the position where it last saw the player and holding
there, so breaking contact and moving is a real defence rather than a few seconds
until the squad walks straight to wherever you went.

---

## Architecture

```
src/
  main.rs       state machine, game loop, input, objective tracking
  campaign.rs   mission table: objectives, loadouts, enemy weights, waves
  maps.rs       six arena layouts as axis-aligned box lists
  level.rs      box collision, raycasts, BFS flow field, level construction
  player.rs     movement (Quake/Source model, 120 Hz substeps), recoil, ADS
  enemy.rs      eight enemy kinds, AI, meshes rebuilt per frame
  weapon.rs     weapon defs and build resolver, arsenal, damage falloff
  grenade.rs    thrown physics, bouncing, fuses, frag blasts and smoke clouds
  combat.rs     hitscan resolution against level and enemies
  config.rs     the settings file: path, format, and its failure behaviour
  viewmodel.rs  first-person weapon, projected to 2D
  pickup.rs     health / ammo / armor drops
  effects.rs    particles, tracers, hit markers, damage readouts
  hud.rs        text, HUD, menus, combat feedback overlays, the manual's pages
  codex.rs      the manual's content: controls, bestiary, armoury and kit
  audio.rs      25 procedurally synthesised effects with file override
  assets.rs     the embedded-asset table, the one place a path is named
  surface.rs    the six world textures and their sampling
  fog.rs        distance fog, the one custom shader
  win32.rs      Windows accessibility-hotkey suppression
  i18n.rs       all user-facing strings, 中文 / English
  mesh.rs       mesh building, baked vertex lighting, world-space UVs
  mesh_tests.rs mesh and UV regression tests
  util.rs       RNG, ray/AABB
```

Two design decisions worth knowing:

**Lighting is baked into vertex colours.** macroquad's default 3D material has no
lighting model — it draws `vertex_color × texture`. Rather than write a shader
for it, every mesh computes a face normal at build time and folds directional +
ambient + sky-bounce lighting into the vertex colour. Geometry that moves
(enemies, pickups) rewrites its transformed vertices each frame, into a scratch
buffer the renderer keeps, so the work is real but the allocation is not.

**The one custom shader is the fog.** Distance haze is the one effect vertex
colours cannot carry: it depends on where the camera is standing, so baking it
would mean rewriting every vertex of every mesh every frame. `src/fog.rs` is a
material that does it on the GPU, from the eye-space depth the vertex shader
already has. It is written in GLSL ES 100 — the dialect macroquad's own shaders
use, which miniquad translates for desktop GL — and a driver that refuses it
costs the fog and nothing else: the game logs the error and draws the world with
the default material.

Two things about that path that cost an afternoon each if missed, both recorded
in the file: a custom material does **not** inherit the default one's depth
state, so `depth_write` and `depth_test` have to be set explicitly or the world
draws in submission order; and `Model` really is the view matrix, so fog range is
in metres with no camera position to pass in.

A measured pass over the render path removed the per-frame allocation entirely:
pre-sizing the batch builders, caching the three pickup prototypes, hoisting the
smoke clouds' trigonometry out of the loop, and — with the largest single win —
giving every per-frame mesh one scratch buffer instead of a fresh `Mesh` each
time took a representative gameplay frame from about **135 allocations and
3.1 MB of churn to 0**, with the vertex and index buffers verified byte-identical
by hash along the way. `a_rebuilt_frame_allocates_nothing` is the standing guard.

**The weapon is drawn in 2D, not in the 3D pass.** It is defined in view space and
projected to screen triangles with the camera's own FOV. Drawing it as world
geometry would make it clip through walls whenever the player stands near one.

**Textured faces take their UVs from world position.** A face's UV is where its
vertices are, measured along two axes that lie in the face's own plane, so two
faces of one plane — the slabs of a floor, a crate lid and the ledge it sits on,
a wall that carries on around a corner — name the same texel at the same world
point. Each quad used to start its pattern at its own first corner, which is
invisible on a lone box and a visible tiling step everywhere two of them meet;
`every_level_face_samples_one_uv_field` walks all six arenas and requires the
shared corners to agree bit for bit.

**Weapon numbers go through a resolver.** A weapon kind is an authored def
(`WeaponDef`); an instance carries a build (`WeaponBuild`), and the numbers it
actually uses are `resolve(def, build)`. A shipping gun has an empty build,
which resolves bit-for-bit to the table the game shipped with —
`the_shipped_tables_survive_the_resolver_bit_for_bit` is the guard. Nothing in
the game installs a part yet: the layering exists so that a gunsmith screen, a
paint or a loadout that changes a stat is a data change on top of one table,
rather than a second set of numbers to keep in sync.

**Combat feedback is its own layer.** A kill flashes a warm frame in from the
screen edges and buys a short hit-stop — 0.05 s during which the world is
simulated at a quarter rate while input, menus and the pointer keep the real
clock. Damage you deal floats off the impact point as a number, projected back
from the world each frame (macroquad 0.4.16 has no `Camera3D::world_to_screen`,
so the frame's view-projection is rebuilt on the CPU from the camera's own
numbers). Damage you take draws a bearing arc around the crosshair, and a walking
enemy kicks dust off each footfall. Colour is never the only channel: the arc's
length carries the hit's strength, the two tracers differ in width and shape as
well as in colour, and a headshot's number is larger as well as brighter.

---

## Assets

Everything under `assets/` is CC0 or OFL and safe to redistribute. See
`assets/MANIFEST.md` for the full per-file inventory.

| Kind | Source | Licence |
| --- | --- | --- |
| Weapon, character and prop models | Kenney (Blaster Kit 2.1, Blocky Characters 2.0) | CC0 |
| Sound effects (19) | Kenney (Impact / Sci-Fi / UI audio) | CC0 |
| Surface textures (6) | [Poly Haven](https://polyhaven.com) | CC0 |
| Chinese UI font | LXGW WenKai 霞鹜文楷 1.511, subset | SIL OFL 1.1 |

The font is subset to GB2312 Level 1 plus common punctuation and every CJK
character in the source, which is why it is 2.9 MB rather than 20 MB. Its family
name was changed to `RustFPS UI` as OFL-1.1 requires for modified versions —
**load it by path, not by family name**. Regenerate with
`python tools/font/fetch_font.py`.

### Embedded vs. on-disk assets

The font, the six world textures and the 19 Kenney sound effects are embedded
with `include_bytes!`. The font and the sounds used to be loaded from a relative
`assets/` path, which works under `cargo run` (the working directory is the
project root) but silently fails for a released executable launched from
anywhere else — Chinese text fell back to the built-in ASCII font, and every
sound fell back to its synthesised version without saying so.

The sounds still probe `assets/` **first**, so dropping a replacement file into
`assets/sounds/` overrides the embedded copy without a rebuild.

`src/assets.rs` is the only module that names a path: everything goes through
`assets::bytes(Asset::…)`. That indirection exists to keep the decision above
reversible — moving the files out to disk, or into an archive beside the
executable, is a change to one function rather than to its 26 call sites. The
`Asset` enum has a variant per file rather than taking a path as a string, so a
renamed file is still a compile error.

**When to revisit it**, in the order the costs actually arrive:

| Trigger | Move |
| --- | --- |
| Editing art hurts more than shipping one file helps | probe `assets/` before the embedded copy, as the sounds already do |
| The executable gets awkward to publish (~100 MB) | pack `assets/` into one indexed archive and ship it beside the binary |
| Assets stop fitting in memory | stream per level instead of loading everything at start |

Two things worth knowing before touching it. Embedding does **not** cost RAM in
proportion to the payload: the bytes sit in the binary's read-only segment,
which the OS maps from the file and pages in on demand, so a large executable is
not a resident one. And the first cost to arrive is not size but iteration —
`lto`, `codegen-units = 1` and `strip` mean every art change re-optimises and
relinks the whole binary.

### On the 3D models

The Kenney `.obj` models are staged and validated in `assets/models/`, but the
running game builds its characters and weapons procedurally. macroquad 0.4.16 has
**no model loader at all** — no `load_model`, no OBJ parser, no feature flag that
enables one. Using them would mean hand-rolling an OBJ→`Mesh` parser, and since
the weapon viewmodel is necessarily drawn in 2D (see above) only the characters
would benefit. They are left in place so that work is easy to pick up.

The sound effects *are* live: `audio.rs` probes `assets/sounds/<name>.ogg` then
`.wav` for each of its 25 slots and falls back to its own synthesis, so the 19
Kenney files take effect with no wiring.

---

## Verification status

What was actually tested, and what was not:

- **New control systems**: the running game was driven through Tab / M / J /
  C / 3 / 4 with window captures confirming the inventory screen (four slots,
  grenade and medical counts), the mission panel, the map (arena footprint,
  player heading, scale bar), the knife viewmodel and the 4-slot HUD strip.
- **Mouse look**: verified by injecting 800 px of relative pointer travel and
  confirming the view rotated to face a different part of the arena with the
  horizon at a sane height — no runaway drift, which is what the naive
  cancel-based approach produced.
- **No stray shots**: a mission now starts with a full 30/30 magazine. The click
  that leaves the menu used to still be held on the first frame of play.
- **Windows build**: `cargo build --release` succeeds. The executable was launched
  and run for 12 s with no crash and empty stderr, then driven through
  menu → mission → gameplay with window captures confirming the 3D arena, HUD,
  weapon viewmodel and Chinese text all render. Across an 11 s capture the
  player's health fell 100 → 47 without any input, which exercises the whole
  combat loop end to end: spawning, flow-field pathing, line-of-sight checks,
  enemy fire and damage application.
- **Linux build**: `build-linux.ps1` produces a valid ELF — verified by magic
  bytes and `file` (`ELF 64-bit LSB pie executable, x86-64, static-pie linked`).
  **It has not been executed**, because this machine has no Linux environment
  (WSL is installed but has no distribution). It is verified as a well-formed
  static binary, not as a running program.
- **Audio**: 19 of 25 slots are overridden by real Kenney files; the remaining 6
  (pickup stings and mission-complete/failed fanfares) stay synthesised on purpose.
  Whether a given sound *feels* right in context was not auditioned.
- Font rendering was verified end-to-end against `fontdue` — the exact rasterizer
  macroquad uses — and visually in the running game.

### Why the pointer is recentred every frame

macroquad refreshes its mouse position **only while the cursor is not grabbed**:

```rust
fn mouse_motion_event(&mut self, x: f32, y: f32) {
    if !context.cursor_grabbed { context.mouse_position = Vec2::new(x, y); }
}
```

So calling macroquad's own `set_cursor_grab(true)` freezes mouse look outright —
WASD keeps working while the view simply stops turning. The game therefore
confines the pointer through **miniquad** directly, which is what actually issues
`ClipCursor`, and leaves macroquad's flag alone.

Confining alone is not enough either: `ClipCursor` pins the cursor to the window,
so after one sweep of the mouse it reaches an edge and the delta stays zero
forever. The pointer is recentred to the window centre every frame instead, using
`SetCursorPos` on Windows and `XWarpPointer` on X11 — both ignore pointer
acceleration, so the cursor lands exactly where it is aimed and no drift builds
up. The delta baseline is owned by the game and rebased to that centre.

Cancelling each frame's movement instead of recentring does **not** work: the
cancelling move is itself picked up as the next frame's delta, and the feedback
loop drifts until the view is pointing at the sky. That failure was observed, not
theorised.

### Shift and the Windows IME

On a Chinese Windows install **Left Shift is the IME's Chinese/English toggle**,
and Shift is sprint here — so every sprint flipped the input method and the IME
then swallowed WASD. The game calls
`macroquad::miniquad::window::set_ime_enabled(false)` at startup, which detaches
the IME context from the window (`ImmAssociateContextEx(hwnd, NULL, 0)`) and sets
a flag miniquad re-checks on `WM_SETFOCUS` and `WM_IME_SETCONTEXT`, so it
survives focus changes. There is no text input in the game, so the IME stays off
for the whole session. Off Windows it is a no-op.

If Shift still toggles after this, the IME in question uses a **global hotkey**
rather than the per-window IMM path, which no application can override. Fix it in
the IME's own settings — Sogou, QQ Pinyin and friends all have a
"中英文切换" option where the Shift toggle can be switched to Ctrl+Space or
turned off. That is the only lever that reaches a global hook.

### Shift and the Windows accessibility shortcuts

The same key is also how Windows offers Sticky Keys — five taps — while holding
Right Shift for eight seconds offers Filter Keys and holding NumLock for five
offers Toggle Keys. All three answer with a dialog that takes focus mid-fight,
and sprint was one short burst of taps away from the first of them.

`src/win32.rs` clears the hotkey bit on all three at startup and puts them back
when the game exits, through `SystemParametersInfoW`. There is no per-process
form of the switch, so the flags really are cleared for the session while the
game runs — `SPIF_UPDATEINIFILE` is deliberately never passed, so nothing is
written to the user's profile, and a process that is killed rather than closed
leaves them cleared until logoff, which is the price of the trick.

Two things that cost an afternoon each if missed, both recorded in the file:
`SPI_SETSTICKYKEYS` is **`0x003B`**, not the `0x000C` that the neighbouring names
suggest, and `SystemParametersInfoW`'s `uiParam` is the block size **in bytes** —
passing the word count is accepted by the compiler, rejected by Windows, and the
only symptom is that nothing happens.

### A note on automating keyboard input

Sending synthetic keystrokes to this game needs care, and it cost real time to
work out why: **macroquad's windowing layer (miniquad) reads the key from the
*scancode* in `WM_KEYDOWN`'s `lparam`** (`HIWORD(lparam) & 0x1FF`), not from the
virtual-key code in `wparam`. Tools that inject a virtual key with a zero scan
code — including PowerShell's `SendKeys` — produce events that translate to
nothing. `keybd_event`/`SendInput` must be given the real scan code (`W` = 0x11,
`Tab` = 0x0F, `M` = 0x32 …).

It also registers **raw input for the mouse only**, which is why synthetic mouse
input always worked while synthetic keys silently did nothing. That was a test
harness problem, not a game bug — but it is worth knowing before concluding
"keyboard input is broken".

## Known limitations

- Audio is absent from the cross-compiled Linux build (see above).
- A run in progress is not saved: quitting mid-mission loses the bodies, the
  score and the wave the mission had reached. The campaign position is kept
  (`unlocked` / `last_mission` in the settings file), so **continue** puts the
  player back at the start of the mission they were in.
- Enemies cannot climb the raised structures, only step over crates under 0.62 m.
  This is deliberate — it gives the player a vertical advantage — but it also means
  a player who camps somewhere unreachable is only threatened by ranged enemies.
- **Cosmetic:** the ammo readout shows `0 / 0` in red for the knife, because the
  HUD renders a magazine count unconditionally and melee weapons have none. It
  should read as "no ammo" rather than "empty".
