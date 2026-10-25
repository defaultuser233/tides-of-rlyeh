# TIDES OF R'LYEH / 拉莱耶的潮声

A single-player campaign FPS written in Rust, built on [macroquad](https://macroquad.rs).
Six hand-built missions, five weapons, five enemy archetypes and three cosmic
horrors, a bilingual (中文 / English) interface, and no launcher, engine install
or runtime to deploy.

- **Windows**: a single `tides-of-rlyeh.exe`
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

Both `U` and `Alt`+`T` are motions as well as words, and they turn the weapon
**about the grip** rather than about the weapon. The grip is where the hand is,
so it is the one point that stays where the hand left it while the muzzle and
the stock swing round it — which is the difference between an arm reading a
weapon and a weapon on a turntable. `U` runs a short path of poses: the wrist
rolls in, one flank is brought round to the eye and *held* there while the optic
and the fore-end are read, then over the top to the far side and the stock, held
again, then back with the weight overshooting once before it settles. It is a
path and not a set of overlapping windows for the same reason: a window can only
swell and fade, and a turn-over has to stand still where there is something to
read. `Alt`+`T` is its own motion rather than a shorter cut of that one — the
weapon tipped muzzle-down until the well comes to the eye, because a magazine
check is a measurement, not an inspection. Which motion a weapon gets is per
weapon: a sidearm and a shotgun are tipped to the well, a rifle, a submachine gun
and a light machine gun are turned over, a knife is turned in the fingers with
nothing to read, and the heavier the weapon the longer the arc it is hefted
through. Firing or reloading takes the weapon back out of the player's hands over
a tenth of a second rather than instantly, sprinting will not start an inspect at
all though walking will, and the mouse keeps working throughout so the head can
follow the weapon round — the view is never locked.

An empty trigger has something to say. A magazine that has run dry with a spare
behind it says so and starts the reload; one with nothing behind it says that
instead, on the same line, because the dry click alone read as the game having
eaten the input. `R` on a magazine that is merely full stays quiet, because that
is not a mistake.

A reload belongs to the weapon in hand: switching slots cancels it. A magazine
that went away half-full comes back half-full — nothing finishes reloading in
the holster.

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

`G` takes one out and puts the rifle away — the hand that was holding it is the
hand that throws — so the sights come down with it; a second `G` puts it back.
The four are four objects and not one casing in four colours: the frag is a ball
with a fuze screwed into the top of it and the lever lying along the side, the
smoke a tin can breathing through a row of vents at each end, the flash a
cylinder built out of openings, and the molotov a bottle with a rag in its neck.

Healing channels for 1.2–2.5 s and is interrupted by taking damage, by firing,
or by moving faster than a walk — you cannot patch up mid-sprint.

### Between waves

| Action | Key |
| --- | --- |
| Call the next wave in early | `F` — during the countdown after a wave falls |

During that countdown the field is empty and the next wave's entrances are
already chosen; pressing `F` is the "I am done reloading" button. At any other
time `F` does nothing.

The countdown is silent and invisible, and so is the wave itself. Nothing about
the wave clock is drawn anywhere on the field: there is no "wave 2 of 4" on the
HUD and none on the mission panel either, because a battlefield does not tell you
which wave you are on, and a counter that does is a piece of information the
player is playing the interface rather than the fight to get. What the HUD keeps
is the objective panel's hostile count — and that is the **mission**, not the
wave: every body still to come, released or not, which the rosters make an exact
number rather than an estimate. The wave survives in one place, the result
screen, which is a debrief rather than a battle.

The mission panel's threat row is still there, and it is worth saying that it is
computed from the wave index — `伤害 ×1.00 · 攻速 ×1.00` is the wave's strength
multiplier — so hiding the wave number while printing its multiplier is a
half-measure. It is left in because nothing asked for it and because it answers a
different question, but it is the next thing to go if the same rule is applied
consistently.

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
| Stash — loadout, parts and paint | Main menu |

The map only plots hostiles that are currently aware of you, so it does not
double as a wallhack.

Settings are the one screen that is not reached by a key: they hang off the main
menu, and every row saves itself the moment it changes — see *Settings* below.

### The field manual

`H` and `K` open a three-page manual, and both are also on the main menu, which
is where a new player meets them before ever holding a gun. Page one is how the
game is played: the objective (clear the arena, then reach extraction — there is
no mid-mission save), every binding grouped by what it is for, and the survival
notes that are not obvious from a key list (armour soaks 65% of a body hit and
nothing at all below the hips, nothing shoots you without line of sight, sanity
is drained by what you look at rather than by what hits you, sprint is a tap
rather than a hold).

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

### The armory

The main menu's armory — the bench, the paint shop and the loadout in one —
saves the moment a choice moves, like the settings screen. Three tabs:

- **Mods** — a three-column bench: the slots and the weapon on the left, the
  candidates for the focused slot in the middle, the numbers on the right.
  Hovering a candidate previews it — the panel turns into old → new and the
  weapon model redraws with it — and clicking (or Enter) fits it; nothing is
  fitted by accident, which is what the old "click steps the row" model did.
  Every slot offers "none" plus the catalogue, and the parts are choices rather
  than rungs: the compensator tames the climb, the flash hider keeps the burst
  tight, the heavy barrel buys reach and pays in recoil, the suppressor takes the
  flash off the muzzle and the crack out of the report (and does nothing else —
  see below). **Five sights**, and they are a ladder of what they are for rather
  than of price: a micro dot and a holo sight for finding a target, a marksman's
  scope, a four-power prism and a long scope for seeing one. The magnified three
  are the ones the game then puts the player *behind*: fitting one turns the
  weapon into something shot through glass (see `Weapon::scoped`), which is the
  .50's own sight on rifles that are not the .50 — aim with a prism on the rifle
  and the window opens out into the magnified view the .50 has had all along.
  Which of them counts as glass is a rule and not a list: a fitting that pulls the
  aim in past 30° is magnified, and a weapon whose *own* sights were already that
  narrow is holding iron — the two marksman rifles shoot 22 and 26 degrees through
  a rear notch, and an eyepiece drawn in front of them would be one they have not
  got. Magazines are capacity
  fittings and each names the well it goes into, because a magazine is a fitting
  of the round it feeds — and the set follows the real catalogue for each
  platform rather than a ladder of round numbers: a 5.56 rifle takes the thirty
  it is issued with, a forty-round box, the sixty-round quad-stack and the
  hundred-round drum; each of the three 9 mm machine pistols its own well and
  its own set — the Viper a thirty, a forty and the fifty-
  and hundred-round drums, the MP5 a fifteen, a thirty, a forty or its fifty
  drum, the MPX a ten, a twenty, a thirty or Magpul's thirty-five; the
  marksman's rifle a five, a ten or a twenty; the
  belt-fed gun its hundred-round box or a two-hundred; a sidearm a fifteen, a
  seventeen, a twenty or the thirty-round stick; and a shotgun its six-shell
  tube, the eight-shell tube or a twenty-shell drum. The two full-power rifles
  have **a well each**, because a FAL magazine and an SR-25 magazine are both
  7.62×51 and neither fits the other's well: twenty or thirty or ten for the
  battle rifle, twenty or twenty-five or ten for the marksman's rifle. Putting
  them on one well is what put a twenty-five-round magazine on the FAL, and a
  twenty-five is an SR-25 magazine. **The same reasoning split the 9 mm well**
  once the machine pistols arrived: an MP5 magazine and an MPX magazine do not
  fit one another either, so there are three 9 mm wells now — and the Viper's is
  the one that already existed, so a saved build naming `drum_50_9mm` still
  loads. Sharing a well is right only where the real magazines really are
  interchangeable, which for the three 5.56 rifles they are and for these they
  are not. The issued magazine is a
  named option in its own right, not a `NONE`: a magazine well that offers one
  part and a NONE reads as a weapon with one magazine, which is not what it is.
  Every one of them trades: a bigger magazine is a longer reload and, on the
  drums and the quad-stack, a heavier gun.

  A magazine is the one fitting that **replaces** what it goes into rather than
  adding to it. On the rifle, the submachine gun and the marksman's rifle the
  issued magazine is left off the weapon and the fitting draws the whole thing —
  same mouth, same centre line, same taper and same **curve**, which is why the
  magazines are arcs now instead of the three-step stair the old `rake`
  approximated. A fitting that starts its own arc can never meet a magazine that
  is halfway through one, and that is exactly what the fittings used to look
  like: a second magazine dropped in behind the first.

  **The rail under the fore-end** is its own slot and not the optic's, because a
  weapon can wear a sight and a light at once and they are bolted to two different
  rails. Two things go on it. A **laser** does what a laser is for and only that:
  the hipfire stops being a guess (`BaseSpread` ×0.80) and nothing else moves — not
  the sighted shot, not the recoil, not the damage. A **flashlight** is the one
  fitting in the catalogue whose trade is not in the numbers: what it buys is
  light, **and this game has no dynamic lighting** — the arena's light is baked
  into its faces when the level is built, so a torch cannot brighten a wall. What
  it does instead is draw the beam in the hand and a pool of light where the beam
  is pointed (`hud::draw_torch`), which is what a torch does to the *view* of the
  person holding it and is not the same thing as lighting the room; the only line
  of numbers it moves is what it weighs. Both hang under the *hand* on the
  fore-end, which is where a fore-end rail is, and that placement is **derived**
  (`Def::rail`) rather than authored per weapon: the support hand is the one anchor
  every weapon that can take a fitting already has, and a weapon with no foregrip
  is measured off the barrel instead. The derivation is what
  `render_the_rail_fittings` exists to look at.

  The curve is **radians per metre**, not radians for the magazine, because that
  is how the real thing works: the radius is set by the cartridge's case taper,
  and a bigger magazine is a longer piece of the same arc rather than a flatter
  one. Storing a total angle made the sixty-round magazine *straighter* than the
  thirty, which is backwards. The three numbers are read off the real cases —
  5.56×45 tapers about a degree, so the rifle's thirty is gently curved at
  2.0 rad/m; 9×19 tapers less, but a *thirty-round* magazine is long enough for
  what taper there is to accumulate, which is why the MP5's is the classic
  curved one: 1.7 rad/m. What is straight is the fifteen-round MP5 magazine, and
  the Uzi's and the MP40's, which are straight by choice of manufacture rather
  than because the round is a cylinder. (The first version of this had 9 mm at
  0.55 rad/m and drew a stick.) 7.62×51 tapers about twice what 5.56 does, so
  the marksman's twenty is distinctly curved at 1.7. The sidearm, the shotgun and
  the belt gun do not come through that path at all, because a grip, a tube and a
  belt box are not columns of stacked cartridges. On the sidearm, the
  shotgun and the belt gun there is nothing to replace — a grip, a tube and a
  belt box are not magazines — so their fittings continue what is there and the
  drums hang where a drum belongs, under the receiver.

  **A smaller one is not on offer.** That is a limitation rather than a decision:
  the slot resolves a capacity and a reload time, and a weapon issued with
  thirty rounds has no fitting that takes it to twenty. A twenty-round rifle
  magazine and the belt gun's thirty-round emergency well are both real, and both
  are now *possible* — the replace model is what unblocked them — but neither is
  authored. The panel shows the weapon's numbers against the issued build, arrows
  up or down, cyan good red bad, with the platform's armour penetration at the
  bottom.

  The preview beside it is framed on the weapon **as issued** plus fixed room for
  a magazine, a sight and a muzzle device. It used to be fitted to the model in
  hand, which meant a longer magazine moved the whole weapon up and shrank it —
  and two magazines that move the gun between them cannot be compared, which is
  the one thing that panel is for.
- **Paint** — six finishes, from factory to the drowned teal of the abyss, with
  a wear value and a pattern seed. Wear scuffs the finish towards bare metal in
  patches, strongest on the faces that hash hardest, and reads as one of the
  CS2 wear tiers under the numbers. Choosing a finish rolls its wear and seed
  over the 0.06–0.80 case range; the rows wind the wear and reroll afterwards.
- **Loadout** — the two long-gun slots, each cycled through "none" and the six
  guns, plus how many spare magazines each slot carries and how many frags,
  smokes and medicines come along. There is no economy, so the caps are the
  cost: what they stop you doing is carrying everything at once.
- **Magazines are containers**, not a pool of loose rounds. Each spare is packed
  full and each keeps its own count: a reload swaps the fullest spare into the
  gun and puts the magazine that came out back with whatever it still held, so
  ammunition is conserved and a half-empty magazine is still worth carrying. The
  HUD reads `备弹 140 · 4 匣` for the same reason — 140 rounds over four
  magazines is a different position from 140 over one. A reload with no spare
  holding a round is a dry click. Field ammo fills the gun's magazine first and
  then the fullest spare with room; what does not fit stays on the ground. The
  sidearm always carries two magazines of its own, and a mission with both long
  slots empty is a pistol mission, which is a choice the gear allows.

The weapon is drawn in the panel the way it will be held — attachments and
finish and all — and the same geometry rides into the mission. A build moves
only the numbers its parts name, through the resolver the tests already pin; the
finish is purely the colours the model is drawn in. The choices live in the
settings file, one line per weapon kind (`build_rifle = optic=micro_dot@0.50,…`,
`paint_rifle = abyss@0.32:481516`) plus the loadout, written once it differs
from the issued kit (`primary`, `secondary`, `primary_mags`, `secondary_mags`,
`frags`, `smokes`, `bandages`, `medkits`, `painkillers`) — hand-editable like
everything else, and repaired one entry at a time if it is wrong.

---

## Settings

The settings screen is on the main menu, and every row saves itself the moment it
changes: there is no Apply button and nothing to confirm.

| Row | Values |
| --- | --- |
| MOUSE SENSITIVITY | 0.30 – 5.00 in steps of 0.10, shown with the cm/360 it works out to |
| MOUSE DPI | 400 / 800 / 1200 / 1600 / 2400 / 3200 — the mouse's counts per inch, for that readout |
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

The sensitivity row reports its value as **cm/360** — centimetres of desk travel
per full 360-degree turn — computed from the multiplier and the DPI row below it.
It is the unit players use to carry a setting from one game to another: 1.60 on
an 800 DPI mouse is about a 12 cm turn, and the number updates live as either row
changes. The DPI row walks its presets and wraps at both ends, and a value in
between is carried while it is current, like an odd resolution. Pointer travel is
the unit of aiming everywhere in the game, so the feel does not change with the
window size or the resolution.

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
| Windows | `%APPDATA%\tides-of-rlyeh\config` |
| Linux | `$XDG_CONFIG_HOME/tides-of-rlyeh/config`, or `~/.config/tides-of-rlyeh/config` when `XDG_CONFIG_HOME` is unset |

The file lived in `steel-front` (and later `eldritch-front`) before the game was
renamed. Those are still **read** when the new path has never been written, so a
rename does not cost the player their settings or their progress; the first save
puts everything where it belongs now, and the old directories can be deleted.

The format is `key = value`, one per line, with `#` comments — the file is meant
to be readable and editable by hand:

```text
# TIDES OF R'LYEH settings. Delete this file to go back to the defaults.
fullscreen = false
width = 1280
height = 720
sensitivity = 1.6
mouse_dpi = 800
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

### The gates, in one line

```bash
tools/gates.sh
```

`fmt --check`, `clippy --all-targets -- -D warnings`, `test` and
`check --no-default-features`, in that order, stopping at the first failure: all
four, every time, and either one summary line out or the first twenty lines of
whatever failed. The gates are run for the code's sake; the *output* is what costs
a conversation, and a passing run has nothing to say beyond the count.

### Windows

```powershell
.\build-windows.ps1
```

Produces `target\x86_64-pc-windows-msvc\release\tides-of-rlyeh.exe` (~8.3 MiB).

The Chinese font and the curated sound effects are **compiled into the
executable**, so it is genuinely a single file: copy it anywhere and it runs.
Nothing needs to sit beside it, and the working directory does not matter. The
C runtime is linked in statically as well (`.cargo/config.toml` sets
`+crt-static`), so the executable does not need the Visual C++ redistributable
either — its only imports are DLLs that ship with Windows.

What is in there, in round numbers: the UI font 2.8 MiB, six 512² surface
textures 2.5 MiB, thirty-four sound effects 1.1 MiB. Two tests keep the
distribution honest. `assets::tests::the_payload_stays_within_its_budget` fails
once the embedded total passes 16 MiB, naming the largest contributor — that
failure is the moment to choose between a packed archive beside the executable
and an asset directory beside it, not a bug. And
`audio::tests::every_embedded_sound_is_in_a_container_the_decoder_accepts`
reads every shipped sound the way the loader will.

### Linux, cross-compiled from Windows

```powershell
.\build-linux.ps1
```

Produces `target\x86_64-unknown-linux-musl\release\tides-of-rlyeh`, a **statically
linked x86_64 ELF** (~7.0 MiB). No glibc, no shared libraries, no X11 development
packages needed on the target: macroquad resolves X11 and OpenGL at runtime with
`dlopen`, so the binary needs nothing but a desktop session. It has no `NEEDED`
entries at all. Audio is off in this build — see the script for why, and
`./build.sh` for the native build that has it.

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

Produces `target/release/tides-of-rlyeh` with **full audio**.

---

## Campaign

Six missions. Clearing a mission unlocks the next, and the campaign remembers
the best score on each one — the result screen reads it back, so a number the
HUD spent a run counting is not forgotten the moment the run ends.

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

It is also the one arena that is **under water**, because that is where the city
is supposed to be. The backdrop is a water column lit from the surface far above
— the gradient runs the other way round from a night sky — the fog is murk pulled
in close, and motes of suspended matter drift around the player. The geometry is
unchanged; the drowning is in the light.

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

### Waves

A mission is fought in waves. Each wave is paid for out of a points budget —
one point is one unit of the archetype's own score, so the cost table and the
score table can never drift apart — and the budgets climb across the mission:
the opening wave releases about 45% of the mission's mean, the last about 1.6×.
Every mission therefore opens quiet and builds to a peak that beats its own
mean by 1.5× (`every_mission_builds_to_a_peak`), and the ladder between
missions multiplies on top — mission 6's peak is more than 6× mission 1's.

Strength climbs too: a hostile's spawn strength is the mission's base (its
`wave` table entry) plus the wave index, so late waves hit harder per hostile
as well as arriving in greater numbers — the finale's last wave fights at
strength 12. The mission panel's threat numbers always show the wave in
progress, not the mission's opening step.

Between waves there is a six-second window (three seconds in BUNKER, where the
clock is the pressure): the field is empty, health and breath come back, and the
next wave is announced — `F` calls it in early. Ammunition is not refilled
between waves; that economy belongs to the armory.

That window used to be twelve seconds, and a mission used to open with one like
any other — which meant a player had twelve seconds of empty arena before the
first hostile of the *mission* and twelve more after every two or three kills.
On RUST YARD, whose opening wave is a pair, that is most of the time alone: walk
the whole arena, find nobody, conclude that nothing is coming. The opening is now
a beat rather than a window (`MISSION_OPEN_GAP`, 2.5 s — enough for the map-name
banner and one look at the weapon) and the wave's own releases are a second
apart instead of nearly two, so a wave arrives as a group to be met rather than a
trickle to be waited out.

Each wave uses two or three fixed entrances, picked when the wave opens and
used in rotation, so arrivals read as a direction to hold rather than noise
from every corner. An elite — anything worth 1000 points, which today means
the Colossus — cannot arrive two waves in a row. The whole field, spawnlings
included, is capped at 24 live hostiles.

### Weapons

Twelve guns and a knife. The five issued ones come first; the seven after them
are real weapons, and the ones that share a calibre are separated by *handling*
rather than by numbers on a card — how fast they come up, how hard they climb,
how long they leave you standing still.

| Weapon | Magazine | Damage | Rate | Armour pen. | Character |
| --- | ---: | ---: | ---: | ---: | --- |
| MK-7 RIFLE / MK-7 突击步枪 | 30 | 24 | 682 | 78% | The issued middle: auto, burst or single, 2.4× headshot |
| M4A1 CARBINE / M4A1 卡宾枪 | 30 | 22 | 857 | 76% | A carbine's rate for a carbine's reach; the fastest 5.56 |
| SCAR-L | 30 | 27 | 600 | 82% | Half again the MK-7's punch for two thirds of its rate |
| FAL / FAL 战斗步枪 | 20 | 43 | 667 | 90% | Hits like a truck and kicks like one, on full auto |
| VIPER SMG / 毒蛇冲锋枪 | 30 | 14 | 1090 | 55% | Rate and bloom; the 50-round drum makes it a room-clearer |
| MP5 | 30 | 16 | 770 | 57% | The submachine gun to aim with |
| MPX | 30 | 15 | 923 | 52% | Rate over control; empty in two seconds |
| BREACHER 12 / 破门者-12 | 6 | 15 × 9 | 77 | 30% | Brutal up close, falls off hard |
| LONGSHOT .50 / 远程 .50 | 5 | 115 | 52 | 97% | 16° scope, fires .50 by the card |
| M110 SASS / M110 精确射手步枪 | 20 | 55 | 207 | 90% | One shot per pull: two for a grunt, one for a head |
| MOSIN-NAGANT / 莫辛纳甘栓动步枪 | 5 | 140 | 43 | 98% | The highest number here, and a bolt between each |
| VULCAN LMG / 火神轻机枪 | 100 | 21 | 800 | 85% | Sustained fire, slows you down |

The armour-penetration share is the part of a hit that ignores physical plate,
and it is on every weapon's codex card; the sidearm (M9, 45%) and the knife
(which ignores plate entirely) round out the set.

**The roster has near-pairs and that is deliberate.** The MK-7, the M4A1 and the
SCAR-L are three 5.56 rifles because those are three real answers to one
question, and they share the 5.56 well — so they share its magazines. The same
goes for the Viper, the MP5 and the MPX on 9 mm. What separates them is
handling, not calibre: 682 against 857 against 600 rounds a minute, three
different recoil climbs and three different reloads. If a pair ever stops being
worth the choice, the honest fix is to delete one, not to renumber it.

### Enemies

| Enemy | Behaviour |
| --- | --- |
| GRUNT / 步兵 | Balanced ranged infantry |
| STALKER / 突袭者 | Fast melee charger |
| JUGGERNAUT / 重装兵 | Slow, armoured, three-round bursts |
| MARKSMAN / 射手 | Long range; **telegraphs** its shot with a 0.95 s aim-up |
| DRONE / 无人机 | Hovering flier at ~2.9 m, fast and erratic |

Two of the eight archetypes wear physical plate: the **Juggernaut (30%)** and
the **Colossus (45%)**, both shown on their codex cards. What the plate stops
depends on what is shooting it — a rifle carries 78% of its damage through,
buckshot 30% — which is what the armour-penetration line on every weapon card
is for.

The player's own armour follows the same genre rule set, in reference order:
a body hit gives up 65% of its damage to the plates until they run dry, a hit
that lands **below the hips never does** (legs are never covered), and a
stopped round barely moves the view — armour takes 85% off the aimpunch an
unarmoured hit would leave, so you feel the vest working before you read it.

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

The field follows the player's *current* position, and everything that cannot see
the player walks it. A reinforcement that was only *told* where the player was
has no memory to check, so it follows the field in until it arrives; a unit that
has had eyes on the player goes back to that spot first, and only joins the hunt
once it has stood there and found nothing. Breaking contact therefore still buys
real distance — the squad is briefly walking to the wrong place — but it no
longer ends the engagement. It used to: pursuit was capped at the entry budget
plus six seconds, after which a unit held position for the rest of the mission,
which on a large arena left hostiles standing in corridors the player had no
reason to walk down while the objective counter went on counting them.

**A unit can no longer be permanently lost, because one lost unit used to cost
the whole mission.** The release gate holds while `max_alive` is full, and a wave
does not end until every body it released is down — so a hostile wedged in
geometry, or in a pocket the navigation grid has no route through, stops the wave,
and the wave stops the mission. Nothing about that is visible from the player's
side: the counter still counts the body and there is nothing on the map to find.
Two things close it. The field's steering now falls back to walking straight at
the player in any cell it has nothing to say about, which is every cell a unit
has no business being in; and a unit that spends three seconds pushing without
moving, in such a cell, is put back on the nearest walkable one
(`Level::navigable_near`). A small teleport is a worse-looking thing than any
other fix here and it is still the right trade.

---

## Architecture

```
src/
  main.rs       state machine, game loop, input, objective tracking
  campaign.rs   mission table: objectives, enemy weights, waves
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
  audio.rs      29 effects: 28 curated files with a synthesised fallback
  assets.rs     the embedded-asset table, the one place a path is named
  surface.rs    the six world textures and their sampling
  fog.rs        distance fog and the near-death grade, the one custom shader
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
the default material. The same pass carries the near-death grade: as health or
sanity runs out the world is pulled halfway towards grey in the same fragment
shader, before the fog, so a far wall still meets the sky in the sky's own
colour. The viewmodel and the HUD are drawn after the pass and keep their colour
— the signals a dying player needs are the ones that must not fade.

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

**The sniper's scope is a 2D overlay, and the rifle comes out of the frame to
make room for it.** A scope built as geometry around the sight line can only block
the view it exists to provide: at the 16° the .50 aims down, the tube's own
aperture projects to a couple of hundred pixels, so what the player saw through it
was a black mask with a keyhole in it. Past halfway into the aim the rifle — tube,
objective bell, post reticle and all of it — is left out of the in-hand model, and
`hud::draw_scope_overlay` draws what the eye behind the eyepiece sees: a circular
aperture, a duplex reticle, and the tube's shadow as a ring around the two. The two
swap on the same frame, off the same constant.

**And the ring is translucent, because the circle is wanted and the blackness is
not.** This has been three things, and the last two complaints are what settled it.
First a fixed circle with everything outside it opaque — "狙击枪开镜时只有屏幕中间的
圆形视野，现实中不是这样的". Then, answering that, an aperture that opened until it
had swallowed the window — "应该是中间一个圆，但圆形以外的视野不应该是看不见的". The
revision that holds is the two answers together: **a circle**, opening across the
first half of the aim and stopping at 40% of the height (`hud::scope_radius`), and
**a ring at `SCOPE_MASK`'s alpha rather than solid** — it darkens the periphery,
which is what looking through a tube does to it, and does not pretend the rest of
the world is not there, which is what having the other eye open means. The rifle is
out of the frame by then, so nothing of it is sitting in the middle of the image the
sight exists to provide. The scope is still on the rifle at the hip, and still on the
armory's preview, because that is the only place it is looked *at*.

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
`the_shipped_tables_survive_the_resolver_bit_for_bit` is the guard. The bench
installs parts into that build and nothing else: a fitted optic, muzzle or
magazine is a data change on top of one table, rather than a second set of
numbers to keep in sync.

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

Everything under `assets/` is CC0 or OFL and safe to redistribute, with **one
exception**, which is credited below rather than left implicit. See
`assets/MANIFEST.md` for the full per-file inventory and `assets/SOUND_SOURCES.md`
for the provenance of every sound that is not Kenney's.

| Kind | Source | Licence |
| --- | --- | --- |
| Weapon, character and prop models | Kenney (Blaster Kit 2.1, Blocky Characters 2.0) | CC0 |
| Sound effects (19) | Kenney (Impact / Sci-Fi / UI audio) | CC0 |
| Sound effects (9) | OpenGameArt contributors (bcamzero, rubberduck, bart, stumpystrust) + Kenney RPG | CC0 |
| Sound effects (12) | [Freesound](https://freesound.org) contributors — the guns, the reload, the impacts and the footsteps | CC0 |
| `shot_smg.ogg` (1) | Vincent Sevedge, *Gunshot Sounds*, [OpenGameArt](https://opengameart.org/content/gunshot-sounds) | **CC BY 3.0** |
| Surface textures (6) | [Poly Haven](https://polyhaven.com) | CC0 |
| Chinese UI font | LXGW WenKai 霞鹜文楷 1.511, subset | SIL OFL 1.1 |

The one CC BY file is the submachine gun's shot, and it is the only sound in the
game that is credited rather than dedicated. The pack's page declares CC0 and its
own bundled `creativecommons.txt` declares **Creative Commons Attribution 3.0
Unported, © 2009 Vincent Sevedge**; the stricter of the two is the one honoured
here. It is a CZ-52 pistol, cut at its crack and shaped for a weapon that fires
eighteen rounds a second — the take it replaced was a twelve-gauge sped up by
two, because no CC0 single 9 mm shot exists. The reasoning, the measurements and
an undo path are in `assets/SOUND_SOURCES.md`, Part 3.

The font is subset to GB2312 Level 1 plus common punctuation and every CJK
character in the source, which is why it is 2.9 MB rather than 20 MB. Its family
name was changed to `RustFPS UI` as OFL-1.1 requires for modified versions —
**load it by path, not by family name**. Regenerate with
`python tools/font/fetch_font.py`.

### Embedded vs. on-disk assets

The font, the six world textures and all 28 sound files are embedded
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
`.wav` for each of its 29 slots and falls back to its own synthesis, so the 28
files take effect with no wiring. The embedded copy is asked which container it
is rather than told — the table now holds both Ogg and WAV, and the first
version of that check said "ogg" for every asset, which quietly sent every
embedded WAV to the synthesiser instead.

---

## Verification status

What was actually tested, and what was not:

- **The fitting did not face its own light (latest round)**: "镭射的光线确实指向准心了，
  但是和发射镭射的配件不平行了". Both halves were true, and only one of them could be
  answered by moving the beam: aimed at the crosshair the light runs along the *view's*
  axis, while the module goes through the weapon's pose like the rest of the gun — and
  at the hip that pose is canted for the eye's sake, so the box and the light it emits
  were 7.3° apart (5.9° on the sub-machine gun, 5.3° on the pistol, 0.4° with the
  sights up). It is a hip-hold problem from end to end, which is why it appeared the
  moment the beam started hitting the crosshair.
  A real laser is *zeroed* — adjusted in its mount until the beam meets the sight line
  — and that is what `fitting_aim` does to the module now: the pose, then a rotation
  about the module's own mouth that puts its axis on the beam. The mouth is the pivot,
  so the window the light leaves from is a fixed point of the turn: the beam starts
  where it started. The torch goes through the same turn and for the same reason, plus
  one more — where a torch is pointed *is* the middle of the view (the pool and the
  dazzle are both effects of the crosshair), so its cone had the same disagreement
  with its own light, only visible earlier because a cone is 19 cm long.
  `a_rail_fitting_faces_its_own_light` is three claims: the angle on the matrix the
  fitting is drawn through (under half a degree, three weapons, three aims), the same
  angle on the pose alone (over 4° at the hip, so the test says what it was fixing),
  and the module's own back corner looked for in `assemble`'s *output* where the
  turned matrix puts it — the one assertion that fails when the wiring is taken back
  out of `assemble`, which is how it was checked.
  *And looked at in the running game*: `docs/shots/16_laser_module_and_beam_in_game.png`
  is the player's own build — MK-7, laser on the under-rail, 废土竞技场 — with the
  module clear of the fore-end and the beam leaving its face on the way to the
  crosshair. **The same frame before the fix was not compared**: the angle is the
  test's, and the picture is the after.

- **The knife swung like a spear, so it swings like a knife now (latest round)**: two
  reports, one animation, and the second report is the one that shaped it — "近战武器
  没有攻击动画", and then, of what replaced it, "不应该是戳，而应该是挥刀".
  *Why there was nothing at all.* A blade goes through the same path a round does
  (`fire_player_shot`), and every motion on that path belongs to the round: camera
  recoil is already skipped for a blade (`if !s.melee`), but the arm is not —
  `Weapon::fire` adds one `kick` for whatever was fired, and `kick` is the weapon
  shoved *back* into the shoulder with the muzzle tipped up. For a rifle that is the
  round leaving; for a knife it is a gun that hits, with nothing in the arm.
  *And why the replacement was wrong.* It lunged: 0.15 m away from the eye and back,
  which is a stab, and a stab is what a knife does not do. What the hand does is
  stay where it is and let the point travel — the knife's point is 29 cm up the blade
  from its grip, and `Pose::swing` is a rotation *about that grip*, so a swing is
  `SLASH_YAW` (0.72 rad) to each side of rest with `SLASH_DIP` (0.18) of it dropping
  and `SLASH_REACH` (0.035 m) of step at the end of the cut, over `SLASH_WINDUP` /
  `SLASH_CUT` / the rest of a `MELEE_TIME` of 0.36 s — drawn back to the right and
  up, whipped across to the left and down, brought home. Forward travel is 0.011 m
  against 0.133 m right and 0.145 m left, and
  `a_knife_is_swung_across_and_comes_back` is those three numbers plus the rise, the
  fall, the return to within 5 mm, and a blade at rest that does not move at all.
  It fails on the old motion, which is how the shape was checked rather than the
  size.
  *What it looks like* is `docs/shots/swing.png`: eight frames of the swing on one
  sheet, through the game's own projection at 640×360 a frame
  (`cargo test -- --ignored --nocapture render_the_knife_swing`). **The running game
  was not re-driven** — the frames are the pose function's and not a play session's,
  so the shape is verified and the *feel* is not; hold the trigger on a knife and
  the sweep is what to look at, and `MELEE_TIME`, `SLASH_YAW`, `SLASH_DIP` are the
  three numbers that set how long, how wide and how deep.
  *A swing that meets nothing is silent*, and it stays silent: there is no whoosh in
  `assets/sounds/` and none was invented for this. The only sound a blade makes is
  the flesh hit the round path already plays, and a blade into a wall plays nothing
  at all, because melee skips the whole cover branch — no mark, no spark, no impact
  voice. That is older than this round and was left alone rather than changed under
  a report about animation.
  *And a real bug the swing found.* The in-hand sheet was **upside down** — and had
  been since the function that writes it was written. `Image::export_png` flips what
  it is given (macroquad 0.4.16, `src/texture.rs:318`), the mesh rasteriser feeds it
  rows counted the other way and comes out right, and the screen-space writer fed it
  screen rows straight across, so every picture through it came out mirrored: the
  weapon drawn *above* the crosshair it is held under, which is why the first version
  of this round's sheet showed a cut that drops as a cut that rises. Nothing judged
  on those sheets — sight sizes, in the armory and in the hand — was about up and
  down, which is how it survived. `docs/shots/optics-in-hand/` was re-rendered from
  the fixed writer; the rifle is under the crosshair in it now.

- **A laser made the rifle shrink in the armory, and only there (latest round)**: "加上
  镭射之后枪突然变小了，差点找不到了". The portrait frames itself on what it is handed
  (`hud::portrait_fit` fits the panel to the drawing's width and height), and a laser's
  beam is twenty-six metres of geometry, so the rifle was drawn at a twentieth of its
  size with a beam running off the panel. In the game the weapon was always fine: the
  viewmodel is not fitted to anything. `preview_mesh` leaves light out now — the
  *module* is lit and stays, so the bench still shows what was fitted — and
  `a_laser_does_not_shrink_the_armory_portrait` is the number, checked by deleting the
  filter and watching it fail with "the portrait is 25.6 m wide, which is a beam rather
  than a rifle". `docs/shots/rail/Rifle.png` is the same weapon bare, with a laser and
  with a torch, all three the same size. **The armory panel itself was not photographed**:
  the sheet draws `preview_mesh` — the mesh the panel draws — but the panel's own draw
  was not looked at.

- **The flashbang did nothing, and here is why (latest round)**: "闪光弹完全不闪", against
  an expectation that is exactly right — a hostile that stops for a few seconds, a
  screen that goes white and comes back, and turning away as the defence. Three
  separate things were wrong, and none of them was the code that *looked* wrong.
  *The radius was 11 m and a grenade goes 15 to 20.* So the one person who never
  got flashed by a flashbang was the person who threw it: `d < radius` failed every
  time, and the enemies only went blind if they happened to be near where it landed.
  It is 18 m now, and the white has a floor of half the screen, so a flash thrown
  across a room and *looked at* costs the screen — which is what makes turning away
  the defence rather than standing further off.
  *There was nothing in the world.* A flashbang was a white screen and no event: the
  arena looked identical to everyone not looking at it. `Effects::flash_burst` is
  three shells at the blast for 0.22 s now, drawn in the world pass.
  *A blinded hostile kept walking.* `blind_t` was wired into `los` and nothing else,
  and a unit that cannot see the player walks to where it last saw them — which is
  what a unit that never saw them does, so the blindness was real in the code and
  invisible on the field. A flashed unit now stands still for `FLASH_STAGGER`
  (1.6 s) and spends the rest of its three seconds coming off the light.
  `a_flashed_unit_stands_still_while_the_light_is_on_it` measures the window as
  distance covered against an unflashed control (under a quarter of it), and
  `a_flash_leaves_a_burst_that_is_gone_by_the_next_breath` holds both halves of the
  burst: it draws, and it is over in a fifth of a second. **The running game was not
  re-driven**: what a flash feels like to throw and be caught by is a thing to play,
  and this round it is a thing to test.

- **The four corrections, and a bug they exposed (latest round)**: player feedback
  on the fittings round, four items, plus something the play session found that
  nothing in the repo could have.
  *The scope is a circle again, with a ring you can see through.* The revision
  before this had the aperture open until it swallowed the window; the answer was
  "应该是中间一个圆，但圆形以外的视野不应该是看不见的". So the circle is back
  (`hud::scope_radius`: 0.29 → 0.40 of the height, opening across the first half of
  the aim) and the ring around it is `SCOPE_MASK` at 0.55 rather than solid —
  `docs/shots/15_scope_circle_with_visible_around_it.png` is a prism on the M4A1,
  aimed: the reticle, the circle, and the darkened world still readable outside it.
  `the_scope_is_a_circle_and_the_ring_around_it_is_not_a_wall` holds the arithmetic,
  including the alpha — a ring at 1.0 is the keyhole this was changed twice to get
  rid of, and the test says so.
  *The three magnified sights have been resized four times*, and the last answer is
  the one that is measured rather than guessed: 6.5 cm for the marksman's, 5.5 for the
  prism, 8.5 for the long one, against the micro dot's own 5.6. `docs/shots/optics/`
  is one picture per sight on the same rifle at the same scale (the armory's own
  orthographic portrait), and `docs/shots/optics-in-hand/` is the same six through the
  *viewmodel's* projection at 1280×720 — which is the sheet that matters, because the
  complaint was "改枪界面没问题，局内看着瞄准镜很大" and the difference between those two
  pictures is perspective: a sight sits a third of a metre from the eye, where the
  same millimetres are worth twice the screen. `a_sight_in_the_hand_takes_a_small_share_of_the_screen`
  is the measurement, and it prints every sight's share as well as asserting it: the
  bench's are 4 to 9% of the screen height in the hand, where the 11.5 cm long scope
  was 11. What to look at in those pictures is that the *dark square in front of the
  receiver* — which is the rear of the rifle itself, seen through its own near faces
  where the near plane cuts them — is not the sight.
  *The rail fittings were hanging in the air.* `Def::rail` put the module 6.2 cm
  below the support hand's centre, which is below the bottom of the fist's own box:
  a light under a hand that is not touching it, and the report was "应该装在护木或
  导轨上，你看看你装在哪了". They now sit just under the fore-end and forward of the
  fingers; looked at on `target/rail/*.png` (the sheet draws the torch and not the
  laser, because a 26 m beam takes the sheet's whole bounding box with it).
  *And a real bug, found by looking at a screenshot of the game and asking what the
  big hollow black shape was.* Faces that cross the near plane were being **dropped**
  rather than **cut**: `project` refused any triangle with a corner behind the eye, and
  refusing is not the same as cutting — the near plane is a plane, so the part of a
  face in front of it should be drawn and the part behind it should not. What dropping
  does instead is remove the *front* of a box and leave the player looking into the
  back of it, which is what a rifle's stock a hand's width from the eye was. No
  geometry moved; `screen_face` is Sutherland–Hodgman on one plane now, the drawing
  path fans what it returns, and `a_face_across_the_near_plane_is_cut_and_not_dropped`
  holds it. (The hollow shape in that screenshot turned out **not** to be this — it is
  the gap between the rifle's handguard and its receiver with the dark arena showing
  through, which the in-hand sheet shows as geometry — but the stock was being cut
  open every frame and nobody had looked.)
  *A laser is a line, not a stub.* Its beam runs to 26 m and the projection does
  the rest — and it is drawn **unlit** (`Tri::lit`), because the first version went
  through the viewmodel's key-light shading like a surface and a red light at 0.26
  brightness is a black one. `docs/shots/14_laser_beam_to_the_horizon.png` is it in
  the hand.
  *A torch dazzles, like a flashbang and for as long as it is held on somebody*:
  the same `Enemy::blind` and the same look-at cone, topped up every frame
  (`dazzled_by_torch`, `TORCH_RANGE` 20 m, `TORCH_DAZZLE` 0.35 s).
  `a_torch_dazzles_what_it_is_pointed_at` holds the two directional halves — what it
  is pointed at, and what it is turned away from. **It is invisible to the player**:
  there is no feedback that a hostile has gone blind, so this is a test and cannot
  be photographed — which is the same gap the flashbang has, and is what "闪光弹也没
  做好" may be about.
  *And a build stamp, because three rounds of this have been a stale binary.* The
  main menu's bottom-left corner now reads `TIDES OF R'LYEH v1.0.0 · 10-07 21:52`:
  the version and when the *executable on disk* was written, read from its own
  metadata at startup (`hud::build_stamp`) rather than recorded at compile time,
  because the question is which file is running. The time is local, by way of a
  `GetLocalTime`/`GetSystemTime` pair in `win32` — the first version printed UTC, and
  a file written at 21:46 shown as 13:46 is a file from somebody else's day. A player
  who is told about a change and cannot see it can now check whether they are running
  it at all.
  *And the bug.* The player fitted a laser to the M4A1, played, and it was gone the
  next time they looked: `config.rs`'s reader listed seven weapon keys **by hand**
  while the writer walked `ALL_KINDS`, so every weapon added after the seventh had
  its build written to a file that never read one back. Two lists, one of them
  silently short. Both are derived from `ALL_KINDS` now, and
  `every_weapon_kind_keeps_its_build_and_paint_through_the_file` round-trips the
  *file text* for all fourteen kinds — it fails with the old table, which is how it
  was checked. The player's own M4A1 laser was put back in their config by hand
  (their settings file is theirs; the fix is so it stays there).

- **A rail under the fore-end: a laser and a torch**: the third
  slot in the bench — its own and not the optic's, because a weapon wears a sight
  and a light at once and they are bolted to two different rails (`UNDERBARREL`,
  on every gun that has a fore-end; the knife has none). Two fittings, and the two
  are verified differently because they are different kinds of thing.
  *The laser* is numbers: `BaseSpread` ×0.80 and nothing else, held by
  `a_laser_tightens_the_hipfire_and_nothing_else` — which checks the *negative*
  half too, that the sight picture and the damage did not move.
  *The torch* is not numbers at all, and that is the honest part: there is no
  dynamic lighting in this game to brighten a wall with, so what it draws is a
  beam in the hand and a pool of light where it is pointed (`hud::draw_torch`).
  `docs/shots/12_rifle_torch_pool_of_light.png` is the MK-7 with one fitted in a
  bunker corridor: the wall at the crosshair is a third brighter than the corners
  and the falloff has no edge in it. **The first version of that pool was too
  faint to see** — four discs at under a tenth of alpha each, which the shot above
  was taken to find out — and a fitting a player cannot tell they fitted is a
  fitting that does nothing, so the alphas went up.
  *Where it hangs* is derived rather than authored per weapon (`Def::rail`): under
  the support hand, which is where a fore-end rail is and the one anchor every
  weapon that can take a fitting already has, and off the barrel for the weapons
  held in one hand. A derivation is exactly the sort of thing that looks right in
  the arithmetic and hangs wrong on the weapon, so `render_the_rail_fittings`
  draws it — bare, laser, torch, five weapons, on the armory's own preview
  projection. **Those sheets were looked at**, and they can only show so much: the
  preview has no hands in it, so what they confirm is that the module sits under
  the weapon's line and the beam leaves forward. **That placement was wrong
  first time round** — 6.2 cm below the hand's centre is *below the fist's own
  box*, a light hanging in the air under a hand that is not holding it, and the
  report was "应该装在护木或导轨上，你看看你装在哪了". It is now just below the
  fore-end and *forward of the fingers* (`Def::rail`'s offsets), which is where an
  underbarrel rail is, and `target/rail/*.png` is where that was looked at — the
  laser's own beam is twenty-six metres long and swallows a sheet's own box, so
  the sheet draws the torch, whose module is the same box at the same anchor.
- **Three more sights, a suppressor, and the rule they hang off**:
  *The sights*: `holo_sight` (a square pane), `prism_4x` and `long_scope`, with
  `marksman_scope` retuned from −10 to −14 so that "a scope" is magnified at all.
  Fitting one of the magnified three makes *any* weapon scoped, which is the rule
  in `Weapon::scoped`: the fitting has to pull the aim past 30° **and** the
  weapon's own sights have to have been wider than that — two-sided on purpose,
  because the two marksman rifles shoot 22 and 26 degrees through a rear notch and
  an eyepiece drawn in front of iron sights is one the rifle has not got.
  `a_magnified_optic_is_scoped_and_a_dot_sight_is_not` is that rule, and
  `docs/shots/11_rifle_prism4x_scoped.png` is the MK-7 with a prism on its rail,
  aimed. **The three of them were drawn too big the first time** — "三个倍镜的建模
  不对，装在枪上看起来体积很大" — and have been resized twice since; the sizes that
  hold and the pictures to judge them by are in this round's bullet above.
  `every_optic_is_small_against_the_weapon_it_sits_on` is the bound that keeps the
  22 cm version from coming back: fifteen centimetres of tube, and no taller than the
  receiver it sits on. The bound that holds the *in-hand* size is
  `a_sight_in_the_hand_takes_a_small_share_of_the_screen`, at nine percent of the
  screen's height. `docs/shots/10_rifle_suppressor_prism4x_hip.png` is the rifle at the
  hip with the can on the muzzle and a prism on the rail, painted abyss (the green
  panes in it are the *finish*'s rail colour, not sight glass).
  *The suppressor*: `RecoilPitch` ×0.90, more reach, and the half that is the
  point — no muzzle flash and a quieter report (`Weapon::suppressed`, read by
  `Weapon::try_fire` and by the shot's own sound; `Audio::play_quiet` plays the
  same recording at a third, because a suppressed shot is not a second event).
  `a_suppressed_barrel_has_no_muzzle_flash` holds it.
  **What a suppressor does not do here is hide the player.** Nothing in this game
  listens for a shot: hostiles find a player by `line_of_sight` and nothing else,
  so there is no noise for a can to take out of it. Making that true is a gameplay
  system — a hearing radius on firing, a sound-shaped alert beside the sight-shaped
  one — and it wants deciding on purpose, not slipping in behind a fitting.
  *And the net that was missing*: `every_shipped_fitting_draws_on_a_weapon_that_takes_it`.
  Until now an optic or a muzzle device with no arm in `kit_into` drew **nothing at
  all** and nothing in the repo could tell: the bench offered it, the numbers moved
  and the weapon looked the same. Magazines have had that net since six of them
  arrived without geometry; optics and muzzles had none, so all four of these were
  one missing line away from invisible. The net was checked by *deleting* the
  suppressor's arm — it fails with "suppressor draws nothing on Rifle, which takes
  it" and passes again with it back.
  **Not verified**: the new shapes have not been looked at on a sheet beside the
  old ones (`render_the_magazines` covers magazines only, and there is no
  equivalent for sights and muzzles); and no shot of a suppressed weapon *firing*
  was taken, so "no flash" is a test rather than a photograph.
  **The bench was looked at**, though: `docs/shots/13_armory_new_slot_and_five_optics.png`
  is the armory with the rifle's 瞄具 slot focused, listing 无 / 微型红点 (已装) /
  全息瞄具 / 射手瞄准镜 / 4 倍棱镜 / 长望远式镜, and the slot column above it reading
  瞄具 / 枪口 / **下挂导轨** / 弹匣 — the new slot is in the bench, named, with the two
  rail fittings one row below it, and the numbers panel showing the hovered scope's
  瞄准视野 38° → 28°.
- **The throwable in hand, and the four casings**: two changes, and
  the two halves of each were verified differently because they are different
  kinds of claim.
  *The weapon is put away* — not lowered to the hip, but out of the frame — and
  that is measured rather than eyeballed: `the_weapon_is_put_away_while_a_throwable_is_in_hand`
  assembles what `draw` would paint, for all fourteen weapons, and requires none
  of seven sampled points of any triangle to land inside the view. It reads the
  same list `draw` does (`ViewModel::assemble` was split out of `draw` to make
  that possible), so it cannot pass by measuring a pose that is nearly the drawn
  one. The hand that throws is the right one now, and it is the hand the weapon's
  own grip hand is suppressed for while the arm is out
  (`a_throwable_in_hand_takes_the_weapon_off_the_throw` still requires the weapon
  to move at least 40 px off the arc); aiming is dropped while a throwable is in
  hand, which is `main.rs` and is **not covered by any test**. The hand comes up
  into frame with the thing in it and the throw carries it away again, and both
  the *drawing* and the arc read that position (`ViewModel::throw_hand`) rather
  than the constant behind it, so the line leaves the grenade while the hand is
  still on its way up; the three ends of that motion — below the rest point, on
  it, below it again — are
  `the_throwing_hand_rises_into_place_and_leaves_it_again`. Whether that reads as
  an arm rather than as a hand moving is not measured.
  *The four shapes*: `grenade::casing_shape` is one description, drawn by two
  renderers — the world's `MeshBuilder` and the viewmodel's triangle list — and
  `the_held_casing_is_the_casing_in_the_air` requires their silhouettes to agree
  within a hundredth of the half-extent on every one of the four, in five bands
  from bottom to top; `the_four_casings_are_four_shapes` requires every pair of
  the four to differ by more than six hundredths somewhere. That test and the
  sheet below are what found the two things a reading would not: the frag's fuze
  had no shoulder, so the whole thing read as a light bulb, and the view's lever
  was drawn with `bar`, whose box corners stand √2 further out than the beam it is
  meant to be — 1.5 cm of spoon, visible as a difference between the two
  renderers' silhouettes. **The shapes were looked at**:
  `cargo test -- --ignored render_the_casings` writes `target/casings/*.png`, the
  world's casing on the left and the same casing in the hand on the right, and
  they were read off that sheet against the real prototypes.
  **And it was watched in a running game, twice**, and both runs found something
  the sheet could not. The first: the sheet drew the casings *by themselves*, and a
  player never sees that — a hand closed round a grenade covers the part of it that
  is inside the fingers, so all four showed as four colours of knuckle with a stub
  on each, and the player said so exactly: "投掷物和手重合了，所以我看不出来". The
  sheet draws the hand now, because a picture that leaves it out cannot see what it
  hides. The second run settled it: a *fist* closed round a grenade is the wrong
  idea at any size, because the two meshes occupy the same cubic centimetres and
  the eye reading one of them is not reading the other. The hand is under the thing
  now (`viewmodel::throw_hand`): fingers as a cradle below its body, back of the
  hand below them, and nothing anywhere *above* the underside of the body — which
  is the property `the_held_throwable_is_on_the_screen` checks, point by point of
  the hand against the body's radius at that height, and which no vertex test could
  see (a can's wall has a ring at each end and nothing between them). It also
  requires the fingers to *reach* the underside, since a hand a centimetre below a
  grenade satisfies "not inside it" as easily as one holding it. Holding it from
  below is what the throw wants anyway: fingers wrapped round a body have to be
  drawn through it to open, and fingers under it simply let go. And the thing in
  the hand is drawn **oversized** (14 cm rather than the 10 cm a hand holds,
  `HELD_HALF`), which is a deliberate departure and the only one in the viewmodel:
  every first-person shooter does it with a grenade, and it is the one number to
  turn down if it reads as too large. The evidence from all of this is in
  `docs/shots/` — `01_rifle_in_hand` and `02_after_G_fist_grip_old` are the mission
  with the rifle up and the same frame after `G` with the old grip,
  `03`–`06` are the four kinds *in hand* with that old grip (the four colours of
  fist), and `07_in_hand_frag_cradle_new` is the frag held from below: the whole
  ball and its fuze, nothing over any of it. (They live in `docs/shots` and not in
  `target` because `target` is disposable and has already taken one set of these
  with it.) That run is also where the retry bug was found: a player who dies with
  a throwable out gets it back in hand on the retry, so the first `G` of the new
  attempt put it away and read as a key that did nothing (`Game::start` clears the
  hand now).
  **What is still not verified**: how the stow and the throw read *in motion*; the
  hand in the game is mostly below the frame and behind the weapon panel of the
  HUD, so what a player sees of this is the thing itself — which is the point, but
  it does mean the cradle was judged from the sheet rather than from play; the four
  were photographed while the player was being shot at, at one resolution; and the
  casings have never been seen in the air by eye — a frag lands 6–10 m out, where
  the drawn casing is about 20 px across in a 2560-wide capture, and two attempts to
  stand still long enough to photograph one ended in the player being killed at 15
  and 22 s.

- **The encounter gaps (latest round)**: measured, not judged. A probe build was
  run on RUST YARD with the player doing nothing and logging the wave machine
  once a second — `target/shots_probe2/stderr.log` is the *before*, where the
  mission opens `prep` for **12.1 s** with `alive=0`, and
  `target/shots_probe3/stderr.log` is the after, where the first hostile is on
  the field at **t=2.5 s** and the wave's second is out a second later. The
  between-wave window is 6 s and the waves were not resized: mission 1 is still
  2 / 2 / 4 / 5 hostiles. **The 6 s window itself was not timed in a running
  game** — the probe run dies before a wave is cleared, because the player in it
  does nothing — nor was the claim that a lost unit no longer stalls a mission,
  which is pinned by test instead: `a_hostile_inside_geometry_gets_itself_out`
  starts a Grunt inside a solid found by scanning the arena and requires it to
  get out and then close on the player, and the steering fallback is asserted
  directly at the top of that test.
- **The magazine row and the wave counter (latest round)**:
  `target/shots_mags4/03_candidate_2.png` — the rifle's magazine slot now offers
  four things by name: `标准 30 发`, `40 发弹匣`, `60 发四排弹匣` (the one in the
  gun) and `100 发双鼓`. Before it offered `无` and the 60, which read as a weapon
  with one magazine. Fourteen magazines ship now, against six, and the geometry of
  each is pinned by test rather than by eye:
  `every_shipped_magazine_draws_and_the_bigger_ones_show` reads the catalogue,
  requires every one of them to draw on the weapon it fits, and requires every
  one that holds more rounds than the issued magazine to make the weapon's
  bounding box bigger somewhere. That last part replaced "the long ones hang
  lower", which could not tell a box magazine growing downwards from a shotgun's
  tube growing forwards from a belt box growing outwards.
  **The shapes were not looked at, and that is what the first two attempts at
  them got wrong.** The test above checks triangle counts and bounding boxes; it
  cannot see a gap, and it cannot see a wheel. `render_the_magazines` — an
  ignored test that rasterises `preview_mesh`, the armory's own mesh through the
  armory's own projection, one PPM per weapon and magazine — is what found the
  three defects that were actually there: every capacity fitting sat 3.5 cm behind
  the magazine it was meant to continue (the anchors had been authored against an
  older body), so it read as a second magazine dropped in behind the first; the
  hundred-round drum was two circles in line with a concentric hub on each, which
  is not a magazine, it is the front axle of a toy; and the shotgun's drum hung
  off the end of its tube, out at the muzzle, because a drum was using the anchor
  a tube extension wants. The first renderer was also wrong in the same way the
  test was — it assembled its own triangles instead of drawing the armory's mesh,
  and so it did not show that the preview re-centred on its own contents and moved
  the gun every time the magazine changed. **The pictures are now the evidence and
  the test says only what it can honestly say**: every magazine draws something,
  every one changes the weapon it fits, every one in the catalogue fits a weapon,
  and a fitting on the three replace-the-magazine weapons is not the issued
  magazine over again. "A bigger magazine looks bigger" is not asserted — a
  fifty-round drum is wider and deeper than the thirty-round stick it replaces and
  no taller at all, so every version of that rule that was tried was wrong; it is
  checked by looking at `target/magsheets`.
  `target/shots_ui` is the in-game HUD and the mission panel with the wave row
  taken out of both: the top-right panel reads score, kills and clock, and the
  mission panel reads objective, threat, hostiles and time. **The result screen
  still shows the wave** and was not touched, since it is a debrief and nothing
  asked for it there; the mission panel's threat row is still there too, and the
  README says why and what it would take to finish the job.

- **A gun that runs dry (latest round)**: driven in the running game with the
  trigger *held*, which is the case that mattered — a fresh press is an edge and
  the last round of a magazine leaves the button down, so the old code produced
  a click, no reload and no line for a player who simply kept firing.
  `target/shots_fix2` has both positions captured: `弹匣已空` with the reload bar
  running underneath it, and then `没有可用弹药` with the HUD reading `备弹 0 · 1 匣`
  and the magazine `0 / 50` in red.
- **The sniper's scope**: `target/shots_fix4`, the sight brought up on the .50
  with the view turned onto the arena floor first (the player spawns facing a
  wall, and a scope aimed at a wall is a photograph of a wall): a clear circular
  aperture with the world in it, an opaque surround, a duplex reticle with mil
  ticks, and no rifle drawn behind it. The frame before the tube drops was
  captured too, so the swap is on the record. **That surround has been through two
  more revisions** — an opaque ring, then no ring at all — and the one that holds
  is the circle with a *translucent* ring around it: see the scope overlay's
  section above, and `docs/shots/15_scope_circle_with_visible_around_it.png`, which
  is the M4A1 with a prism on it, aimed, with the dimmed world still readable
  outside the circle. The arithmetic is pinned by
  `the_scope_is_a_circle_and_the_ring_around_it_is_not_a_wall` (a circle inside the
  window at four heights, monotone opening, and a ring under 0.8 opaque);
  **no frame of the raise itself was captured**, so the aperture opening across
  those tenths of a second is reasoned rather than seen.
- **The enemy hunt**: two unit tests drive a Grunt from R'LYEH's farthest spawn —
  64.8 m, the campaign's longest path — and require it to close; one that has
  never seen the player and one that has finished searching its memory. In the
  running game, standing still in R'LYEH draws contact and a death inside half a
  minute, `target/shots_fix6`. **That run also showed how much of the encounter
  problem was near spawns rather than far ones**: the wave's entrances are drawn
  from the 19 m ring as well as the 65 m one, and the near ones were never the
  broken case. The far case is pinned by test rather than by play, because
  arranging one on demand means walking a labyrinth for a minute.
- **The magazines, by eye**: `target/shots_fix8` is the rifle in hand with the
  60-round quad-stack (`60 / 60`) and the SMG with the fifty-round drum
  (`50 / 50`), and `target/shots_fix9` is the bench, whose magazine row now
  offers `无 / 60 发四排弹匣` for the rifle — the 45-round fitting is gone. The
  quad-stack's extra *width* is the thing that makes it a quad-stack and is
  across the gun, so the armory's left-profile preview shows its length and not
  its columns; the columns are visible in hand, where the weapon is held at an
  angle. **The sniper's new raked magazine, the LMG's lid-and-belt box and the
  pistol's floorplate were not photographed** — they are pinned by the geometry
  test, which requires every shipped magazine to add triangles and the long ones
  to hang lower, but they have not been looked at.
- **The inspect motion**: `target/shots_insp3` samples `U` across its 2.2 s on
  the rifle and `target/insp3_sheet.png` is the twelve frames as one sheet. The
  arc reads: the wrist rolls in, one flank comes round and stays for a beat, the
  weapon turns the other way for a second beat, and it comes back upright and
  settles. Two things about it are pinned by test rather than by eye — the grip
  does not move while the weapon turns (`the_inspect_turns_the_weapon_about_the_grip`,
  which requires the muzzle to travel more than 30 cm while the grip travels
  less than a hundredth of a millimetre), and every weapon's path starts and
  ends at the rest pose with a real hold in the middle
  (`the_inspect_path_rests_at_the_ends_and_holds_in_the_middle`). **The motion
  was not watched as motion** — twelve stills are not a frame rate, so nothing
  here says the 2.2 s of it does or does not flow. `Alt`+`T`'s check was
  captured in the same run and is visibly a different pose from any of the
  twelve, which is all that is claimed for it.

- **New control systems**: the running game was driven through Tab / M / J /
  C / 3 / 4 with window captures confirming the inventory screen (four slots,
  grenade and medical counts), the mission panel, the map (arena footprint,
  player heading, scale bar), the knife viewmodel and the 4-slot HUD strip.
- **The wave clock and the bench**: the running game was driven through a whole
  wave cycle — `F` calling wave 1 in early (proved with a probe build: `F skip
  at left=7.26` → `begin wave 0`), enemies engaging from off-screen while the
  player stood still (health 100 → 82 → 28), and the HUD's WAVE row and the
  mission panel's WAVE/THREAT rows captured in `target/shots_waves`. **The WAVE
  rows in both of those are gone now** — `target/shots_ui` is the same two
  screens without them: the top-right panel reads score, kills and clock, and the
  mission panel reads objective, threat, hostiles, time. The armory
  bench was then driven end to end by keyboard alone: slot → candidates → hover
  preview (ADS 38° → 32°) → Enter fitted the scope, `target/shots_bench`. Every
  key injection first proves the game owns the foreground, or sends nothing.
- **The magazines (the round before)**: the bench was clicked open and driven to
  the magazine row, photographed at every step — the rifle's slot then offered
  `none / 45 发弹匣 / 60 发四排弹匣`, hovering the 60 turned the diff panel into
  `弹匣 30 → 60` with `换弹 1.35s → 1.80s`, and the preview grew a longer
  magazine, captured in `target/shots_mags` (script `target/e2e_mags.ps1`). That
  fitting is gone now: the rifle takes a thirty or the sixty and nothing in
  between. Every shipped magazine's geometry is still pinned by test
  (`every_shipped_magazine_draws_and_the_big_ones_reach_lower`): each one adds
  triangles to the weapon it fits, and the long ones hang lower than the standard
  magazine. **The SMG's drum was not looked at in the running game** in that
  round either, so its shape has been verified structurally rather than by eye,
  then and now. The magazine model itself (rounds conserved across a reload,
  spares drawn fullest-first, pickups filling the gun and then the fullest
  partial) is covered by tests, not by play.
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
- **Audio (the guns, this round)**: thirteen clips were replaced with real
  recordings rather than the Kenney science-fiction lasers they used to be — the
  five player weapons, the enemy's shot, the reload and its dry click, the three
  impacts, the death and the footstep. Twelve are **CC0 on their own Freesound
  pages**, checked one page at a time; the thirteenth is the submachine gun's
  shot, which is a CZ-52 recording from an OpenGameArt pack whose page says CC0
  and whose bundled licence text says **CC BY 3.0, © 2009 Vincent Sevedge** — so
  it is credited in the asset table above and is the one non-CC0 byte in
  `assets/`. Each one's source page, author, licence string, input and output
  measurements and decode check is in `assets/SOUND_SOURCES.md`. The Freesound
  takes were pulled from the CDN previews rather than the uploader originals,
  which need a login; previews are published under the item's own licence and the
  originals can be dropped in over them without touching anything else. **Nobody
  has listened to them.** What was measured instead: every file decodes under two
  independent decoders to within 5e-7, every one is 44.1 kHz mono at −1.0 to
  −1.3 dBFS with no sample at full scale, all 28 files hash distinct, and the
  worst case for full-auto — the sum of every copy that overlaps at the weapon's
  own rate — peaks *lower* than the clips they replace did (the old machine gun,
  shotgun and sniper summed past full scale; the new ones stay at 0.88–0.95,
  including the CZ-52, whose room tail is deliberately pulled down after its
  first 15 ms because eighteen rounds a second of a one-second tail is not a tail
  but a bed). Two of the thirteen are stand-ins the player can hear for what they
  are: `hit_head` is cracked ice offered by its author as a bone substitute, and
  nothing else in the set is a substitution any more.
  Thirteen clips they replaced are in `assets/_downloads/sfx_new/orig_backup/`.
- **Audio (the first pass)**: 28 of 29 slots have a real file — the 19 Kenney
  ones plus nine CC0 files found for the slots that were synthesiser-only (the
  pickup and objective stings, the completion and failure cues, and the §19.4-10
  event cues for elite arrival, boss arrival and low sanity). `reload_done` is
  the one slot still synthesised, on purpose. The nine were verified as CC0 on
  their own pages, and their headers, containers, decoded levels and byte-for-byte
  identity are recorded in `assets/SOUND_SOURCES.md`. **None of them has been
  listened to**, and two are outside the length the brief asked for
  (`boss_arrival` 5.15 s, `mission_failed` 1.13 s) — they were picked for
  character. Deleting any of them falls back to its recipe.
- **The load path**: a probe build was run from a working directory where the
  on-disk `assets/sounds/` probe cannot resolve, so every effect had to come out
  of the embedded copy or the synthesiser. All 29 slots loaded, 28 as
  `embedded` with the byte counts of their files and `reload_done` as `synth` —
  no slot came back empty, which is what would have happened to every embedded
  WAV under the old container check.
- **The §19.4-10 event cues**: a probe build that logs every effect it hands to
  the mixer was driven through three bursts and three reloads, then a fourth `R`
  on a full magazine as a control. The trace is exactly three `Reload` and three
  `ReloadDone`, each finish following its own start, and the control silent —
  `target/sfx_trace_debug.log`, script `target/e2e_sfx.ps1`. The elite, boss and
  sanity cues were **not** driven in the running game: they are one-shot events
  behind a random draw, and the sanity drain needs a mission 1 never fields. What
  is verified for those is the wiring and the trigger arithmetic, by test — the
  cue picked by `campaign::is_final_wave`, both cues proved reachable against the
  wave budgets, and the alarm's crossing and gap behaviour driven directly.
  None of the four was judged by ear.
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
- There is no bonus wave and no starting-wave selector. Both assume an endless
  run — a "wave 5 of 5" mission has no room for a fifth-wave celebration, and
  the campaign ladder already supplies the escalation a selector would be for.
  The shapes stay in the reference notes until there is a mode they fit.
- A run in progress is not saved: quitting mid-mission loses the bodies, the
  score and the wave the mission had reached. The campaign position is kept
  (`unlocked` / `last_mission` in the settings file), so **continue** puts the
  player back at the start of the mission they were in.
- Enemies cannot climb the raised structures, only step over crates under 0.62 m.
  This is deliberate — it gives the player a vertical advantage — but it also means
  a player who camps somewhere unreachable is only threatened by ranged enemies.
