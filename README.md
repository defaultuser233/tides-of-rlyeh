# STEEL FRONT / 钢铁前线

A single-player campaign FPS written in Rust, built on [macroquad](https://macroquad.rs).
Six hand-built missions, five weapons, five enemy archetypes, a bilingual
(中文 / English) interface, and no launcher, engine install or runtime to deploy.

- **Windows**: a single `rust-fps.exe`
- **Linux**: a single statically linked ELF that runs on any x86_64 distro

---

## Controls

### Movement and stance

| Action | Key |
| --- | --- |
| Move | `W` `A` `S` `D` |
| Jump | `Space` |
| Sprint | `Left Shift` |
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
| Close a screen | `Esc` or the same key again |
| Pause | `Esc` |

The map only plots hostiles that are currently aware of you, so it does not
double as a wallhack.

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
  player.rs     movement, collision resolution, head bob, recoil, ADS
  enemy.rs      five archetypes, AI, meshes
  weapon.rs     five weapons, arsenal/switching, damage falloff
  grenade.rs    thrown physics, bouncing, fuses, frag blasts and smoke clouds
  combat.rs     hitscan resolution against level and enemies
  viewmodel.rs  first-person weapon, projected to 2D
  pickup.rs     health / ammo / armor drops
  effects.rs    particles, tracers, hit markers
  hud.rs        text, HUD, menus, result screens
  audio.rs      25 procedurally synthesised effects with file override
  i18n.rs       all user-facing strings, 中文 / English
  mesh.rs       mesh building, baked vertex lighting
  util.rs       RNG, ray/AABB
```

Two design decisions worth knowing:

**Lighting is baked into vertex colours.** macroquad's default 3D material has no
lighting model — it draws `vertex_color × texture`. Rather than write custom
shaders (and risk GLSL differences between desktop GL and GLES), every mesh
computes a face normal at build time and folds directional + ambient + sky-bounce
lighting into the vertex colour. Geometry that moves (enemies, pickups) rebuilds
its transformed mesh each frame. At these vertex counts it is affordable,
but it is not free, and it used to be considerably worse.

A measured pass over the render path removed most of the churn: pre-sizing the
batch builders, caching the three pickup prototypes, and hoisting the smoke
clouds' trigonometry out of the per-frame loop took a representative gameplay
frame from about **135 allocations and 3.1 MB of churn to about 14 and 1.0 MB**,
with the vertex and index buffers verified byte-identical by hash. The remaining
allocations are largely structural: `build_mesh` returns a `Mesh` *by value*, so
the buffer leaves with it and one allocation per vector per frame is the floor
for that signature. Each enemy still builds a transformed copy of its prototype
per frame, which is the next place to look if it ever matters.

**The weapon is drawn in 2D, not in the 3D pass.** It is defined in view space and
projected to screen triangles with the camera's own FOV. Drawing it as world
geometry would make it clip through walls whenever the player stands near one.

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

The font and the 19 Kenney sound effects are embedded with `include_bytes!`.
They used to be loaded from a relative `assets/` path, which works under
`cargo run` (the working directory is the project root) but silently fails for a
released executable launched from anywhere else — Chinese text fell back to the
built-in ASCII font, and every sound fell back to its synthesised version
without saying so.

Both still probe `assets/` **first**, so dropping a replacement file into
`assets/sounds/` overrides the embedded copy without a rebuild.

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
- No save file: campaign progress resets when the process exits.
- Enemies cannot climb the raised structures, only step over crates under 0.62 m.
  This is deliberate — it gives the player a vertical advantage — but it also means
  a player who camps somewhere unreachable is only threatened by ranged enemies.
- Only enemy fire is tracers; player shots use the same visual, so there is no
  distinction between incoming and outgoing at a glance.
- **Cosmetic:** the ammo readout shows `0 / 0` in red for the knife, because the
  HUD renders a magazine count unconditionally and melee weapons have none. It
  should read as "no ammo" rather than "empty".
