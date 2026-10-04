# Audio sources — the nine event cues added on 2026-10-03

Nine files were dropped into `assets/sounds/`. All of them are **CC0 / public
domain**, all are **Ogg Vorbis or PCM/float WAV** (no MP3, no Opus), all are
under 2 MB, and the whole batch is **857,509 bytes (838 KiB)**. Nothing was
transcoded: every byte here is the byte the source published.

The engine picks a file up from `assets/sounds/<slot>.{ogg,wav}` by name; the
slot names come from `Sfx::file_name()` in `src/audio.rs`.

> **Wired up since this document was written.** Two things it records as
> outstanding are done, and the notes below have been corrected in place:
> 1. `Sfx::Healing` now exists (`src/audio.rs`), `begin_heal` in `src/main.rs`
>    plays it instead of the reload's click, and `healing.ogg` is heard.
> 2. All nine files are embedded in the binary (`Asset::Sfx*` in
>    `src/assets.rs`, mapped by `Sfx::asset()`), so they play from any working
>    directory, exactly like the original nineteen — see "Where the file is
>    read from" at the end, which this replaces.
>
> Verified on the machine: a probe build run from a directory where
> `assets/sounds/` cannot resolve loaded all 29 slots, these nine as `embedded`
> with their own byte counts. Only `ReloadDone` came back `synth`, which is the
> one slot with no file on purpose.

---

## How the licences were checked

Every source below is an OpenGameArt submission except `elite_arrival`, which is
reused from the Kenney sci-fi pack already vendored in `_downloads/`. For each
OpenGameArt page the check was:

1. fetch the content page,
2. read the `License(s):` field, which is rendered as
   `<div class='license-name'>CC0</div>` inside a link,
3. read the `href` of that link — every one of them points at
   `http://creativecommons.org/publicdomain/zero/1.0/`,
4. confirm the file's own download link (`/sites/default/files/...`) is on the
   same page.

`Completion sound.` is dual-licensed. Read the note on it below.

---

## 1. `healing.ogg` — 16,508 B, 0.66 s, stereo, 48 kHz

- **Original asset:** `cloth1.ogg`
- **Source pack:** *50 RPG sound effects* by Kenney
- **Source page:** https://opengameart.org/content/50-rpg-sound-effects
- **Direct link:** https://opengameart.org/sites/default/files/RPGsounds_Kenney.zip
  → `OGG/cloth1.ogg`
- **Author:** Kenney Vleugels (kenney.nl)
- **Licence string as shown on the page:** `License(s): CC0`;
  link target `http://creativecommons.org/publicdomain/zero/1.0/`.
  The pack's own `license.txt` reads
  `License (CC0)` / `http://creativecommons.org/publicdomain/zero/1.0/`.
- **Where on the page:** the `License(s):` field in the right-hand metadata
  column, immediately under `Author:`.
- **Bytes:** 16,508 · **Duration:** 0.661 s · **rate** 48,000 · **ch** 2 · peak
  0.669, RMS 0.059, 0 samples at full scale
- **Why this one:** the brief asked for a bandage / injector / vial-handling
  sound at 0.4–1.2 s. `cloth1` is 0.66 s and 55.9 % of its energy sits below
  200 Hz, which is a soft fabric-and-handling noise rather than a metallic hit.
  It is also the same author as every other sound already in `assets/sounds/`,
  so the licence chain is the one the project already documents.
- **Caveat:** 48 kHz, not 44.1 kHz. `quad-snd 0.2.8` resamples anything that is
  not 44.1 kHz with a nearest-neighbour resampler
  (`quad-snd-0.2.8/src/mixer.rs:268-283`), so it plays at the right pitch; it
  just goes through that resampler.

```
00000000: 4f67 6753 0002 0000 0000 0000 0000 4062  OggS..........@b
00000010: 0000 0000 0000 045c d58d 011e 0176 6f72  .......\.....vor
```

---

## 2. `pickup_health.ogg` — 15,531 B, 0.39 s, stereo, 48 kHz

- **Original asset:** `item_misc_05.ogg`
- **Source pack:** *80 CC0 RPG SFX* (item sounds)
- **Source page:** https://opengameart.org/content/80-cc0-rpg-sfx
- **Direct link:** https://opengameart.org/sites/default/files/80-CC0-RPG-SFX_0.zip
  → `item_misc_05.ogg`
- **Author:** rubberduck
- **Licence string as shown on the page:** `License(s): CC0`; link target
  `http://creativecommons.org/publicdomain/zero/1.0/`.
- **Where on the page:** the `License(s):` field in the metadata column.
- **Bytes:** 15,531 · **Duration:** 0.393 s · **rate** 48,000 · **ch** 2 · peak
  0.762, RMS 0.064, 0 samples at full scale
- **Why this one:** the slot wants a clean rising cue at 0.2–0.6 s. Measured
  with an 8-segment FFT peak tracker the fundamental climbs
  `955 → 990 → 1822 → 1869` Hz across the first half and then decays — a single
  short upward sweep, which is exactly the shape asked for. 0.39 s sits
  comfortably inside the window, and of the ten `Pickup__NNN.ogg` candidates in
  phoenix1291's pack this was the only one that both rose and stayed clear of
  full scale (`item_misc_03.ogg`, the other riser, hits 0.9993 with 2 samples at
  full scale and was rejected).
- **Caveat:** 48 kHz, same resampler note as above.

```
00000000: 4f67 6753 0002 0000 0000 0000 0000 17d5  OggS............
00000010: a473 0000 0000 95c4 687f 011e 0176 6f72  .s......h....vor
```

---

## 3. `pickup_ammo.wav` — 22,572 B, 0.26 s, mono, 44.1 kHz, 16-bit PCM

- **Original asset:** `clink2.wav`
- **Source pack:** *Metal Impact Sounds*
- **Source page:** https://opengameart.org/content/metal-impact-sounds
- **Direct link:** https://opengameart.org/sites/default/files/clink2.wav
- **Author:** bmaczero (Brian MacIntosh)
- **Licence string as shown on the page:** `License(s): CC0`; link target
  `http://creativecommons.org/publicdomain/zero/1.0/`.
  The page's attribution note reads "Optionally credit Brian MacIntosh."
- **Where on the page:** the `License(s):` field in the metadata column.
- **Bytes:** 22,572 · **Duration:** 0.255 s · **rate** 44,100 · **ch** 1 ·
  16-bit PCM (`fmt` chunk `01 00`, 1 channel) · peak 27265/32767 = 0.832,
  0 samples at full scale
- **Why this one:** a recorded metal clink, 0.26 s, no low-frequency content at
  all (0.0 % under 200 Hz) so it reads as brass/magazine rather than as a body
  hit. Deliberately distinct from the existing `hit_world` / `hit_head` slots,
  which are both Kenney `impactMetal_*` material.
- **Caveat:** mono, and the brief asked for "metal clash + short cue"; this is
  the clash only. Layering a second file is not possible without editing, which
  the brief forbade.

```
00000000: 5249 4646 2458 0000 5741 5645 666d 7420  RIFF$X..WAVEfmt
00000010: 1000 0000 0100 0100 44ac 0000 8858 0100  ........D....X..
```

---

## 4. `objective_done.wav` — 63,532 B, 0.72 s, mono, 44.1 kHz, 16-bit PCM

- **Original asset:** `bing1.wav`
- **Source pack:** *Metal Impact Sounds* (same pack as `pickup_ammo`)
- **Source page:** https://opengameart.org/content/metal-impact-sounds
- **Direct link:** https://opengameart.org/sites/default/files/bing1.wav
- **Author:** bmaczero (Brian MacIntosh)
- **Licence string as shown on the page:** `License(s): CC0`; link target
  `http://creativecommons.org/publicdomain/zero/1.0/`.
- **Where on the page:** the `License(s):` field in the metadata column.
- **Bytes:** 63,532 · **Duration:** 0.720 s · **rate** 44,100 · **ch** 1 ·
  16-bit PCM · peak 32686/32767 = 0.9975, 0 samples at full scale
- **Why this one:** the brief asked for a restrained *two-note* cue at
  0.5–1.5 s. This is the only candidate found whose fundamental actually steps
  once and holds: `1389 → 1491 (×3) → 1992 (×4)` Hz, onset detector reports 2
  events. That is a two-note cue by construction, not by description.
- **Two things to listen for:** (a) it is a struck metal bell at ~1.4–2 kHz, so
  it is bright — "restrained" was the brief, "not cheerful/cartoonish" was the
  harder constraint, and a bell sits closer to the line than a dark synth would;
  (b) it peaks at −0.02 dBFS and `Audio::emit` applies no per-slot gain
  (`src/audio.rs:257-286`), so it will be noticeably louder than its neighbours,
  which sit around 0.9. Both are audible-only judgements I cannot make.

```
00000000: 5249 4646 24f8 0000 5741 5645 666d 7420  RIFF$...WAVEfmt
00000010: 1000 0000 0100 0100 44ac 0000 8858 0100  ........D....X..
```

---

## 5. `mission_complete.wav` — 186,412 B, 2.11 s, mono, 44.1 kHz, 16-bit PCM

- **Original asset:** `bong1.wav`
- **Source pack:** *Metal Impact Sounds* (same pack as `pickup_ammo`)
- **Source page:** https://opengameart.org/content/metal-impact-sounds
- **Direct link:** https://opengameart.org/sites/default/files/bong1.wav
- **Author:** bmaczero (Brian MacIntosh)
- **Licence string as shown on the page:** `License(s): CC0`; link target
  `http://creativecommons.org/publicdomain/zero/1.0/`.
- **Where on the page:** the `License(s):` field in the metadata column.
- **Bytes:** 186,412 · **Duration:** 2.113 s · **rate** 44,100 · **ch** 1 ·
  16-bit PCM · peak 29754/32767 = 0.908, 0 samples at full scale
- **Why this one:** the brief wanted 1.5–3 s, low, solemn, cosmic-horror, and
  explicitly *not* a major-key victory fanfare. This is a single struck deep
  bell: fundamental `213 → 419` Hz, 0.1 % of its energy above the 200 Hz line,
  spectral centroid staying between 1751 and 807 Hz, one onset. No melody, no
  major third, nothing triumphant — a toll.
- **Caveat:** it is a bell, i.e. "a struck object", not an authored four-chord
  resolution. Compared with the synth it replaces (the page-19 note in
  `assets/MANIFEST.md` describes a C-E-G-C major figure) this is a change of
  character, not just of fidelity. Worth a listen before it ships.

```
00000000: 5249 4646 24d8 0200 5741 5645 666d 7420  RIFF$...WAVEfmt
00000010: 1000 0000 0100 0100 44ac 0000 8858 0100  ........D....X..
```

---

## 6. `mission_failed.ogg` — 21,987 B, 1.13 s, stereo, 48 kHz

- **Original asset:** `monster_07.ogg`
- **Source pack:** *80 CC0 creature SFX*
- **Source page:** https://opengameart.org/content/80-cc0-creature-sfx
- **Direct link:** https://opengameart.org/sites/default/files/80-CC0-creature-SFX_0.zip
  → `monster_07.ogg`
- **Author:** rubberduck
- **Licence string as shown on the page:** `License(s): CC0`; link target
  `http://creativecommons.org/publicdomain/zero/1.0/`.
- **Where on the page:** the `License(s):` field in the metadata column.
- **Bytes:** 21,987 · **Duration:** 1.131 s · **rate** 48,000 · **ch** 2 ·
  peak 0.962, RMS 0.163, 0 samples at full scale
- **Why this one:** 93.9 % of the energy is under 200 Hz and the fundamental
  falls `804 → 372 → 406 → 307` Hz across the file — a short downward collapse,
  which is the "下坠、压抑" the brief described, and as far from a cheerful
  fail-jingle as anything CC0 I could find.
- **Caveat — this is the weakest pick of the nine.** The brief asked for
  1.5–3 s; this is **1.13 s**, roughly 0.4 s short. Every CC0 failure cue that
  was the right length turned out to be a whole track (`gameoverdark.ogg` 10 s,
  `gameoverstrangesounding.ogg` 8.5 s, `GameOverII_2.ogg` 21 s,
  `game_over_bad_chest.wav` 12 s, `defeat.wav` 37 s), and the ones that were the
  right length were chiptune comedy (`Total Fail`, `Better luck next time`).
  I chose character over the duration window and am flagging it so you can
  delete the file if you disagree — the synth falls straight back in.

```
00000000: 4f67 6753 0002 0000 0000 0000 0000 6303  OggS..........c.
00000010: 0b16 0000 0000 6f1a 0fb9 011e 0176 6f72  ......o......vor
```

---

## 7. `elite_arrival.ogg` — 14,317 B, 2.00 s, stereo, 44.1 kHz

- **Original asset:** `lowFrequency_explosion_000.ogg`
- **Source pack:** *Sci-Fi Sounds 1.0* by Kenney — **already vendored**, no
  download was made; copied from
  `assets/_downloads/kenney_sci-fi-sounds/Audio/lowFrequency_explosion_000.ogg`
- **Source page:** https://kenney.nl/assets/sci-fi-sounds (pack also mirrored at
  https://opengameart.org/content/sci-fi-sounds, but the bytes here come from
  the local vendored copy)
- **Author:** Kenney Vleugels (kenney.nl)
- **Licence string as shown in the vendored file:**
  `_downloads/kenney_sci-fi-sounds/License.txt` — `License: (Creative Commons
  Zero, CC0)` / `http://creativecommons.org/publicdomain/zero/1.0/`. This is the
  same evidence `assets/MANIFEST.md` already cites for the pack.
- **Bytes:** 14,317 · **Duration:** 2.000 s · **rate** 44,100 · **ch** 2 ·
  peak 0.902, RMS 0.126, 0 samples at full scale
- **Why this one:** the slot needs 1–3 s of sub-bass pressure that survives
  gunfire. **99.8 % of the energy is below 200 Hz** with a spectral centroid of
  87–125 Hz — the lowest-frequency material in the whole local pack. It sits at
  exactly 2.00 s, inside the window.
- **Known duplication:** the same source file was already curated into
  `assets/sounds/variants/explosion_deep.ogg` by `_tools/curate.py`. Nothing
  auto-loads `variants/`, so there is no collision, but the two files are
  byte-identical. If you would rather not duplicate, the next-best in-range
  candidate was `monster_04.ogg` from *80 CC0 creature SFX* (1.28 s, 99.6 %
  under 200 Hz, 48 kHz) — shorter, and 48 kHz.

```
00000000: 4f67 6753 0002 0000 0000 0000 0000 584a  OggS..........XJ
00000010: 0000 0000 0000 0921 cf4d 011e 0176 6f72  .......!.M....vor
```

---

## 8. `boss_arrival.ogg` — 119,570 B, 5.15 s, stereo, 44.1 kHz

- **Original asset:** `theircoming3.ogg` (downloaded as `theircoming3_0.ogg`)
- **Source page:** https://opengameart.org/content/their-coming-generic-horn-sound
- **Direct link:** https://opengameart.org/sites/default/files/theircoming3_0.ogg
- **Author:** stumpystrust
- **Licence string as shown on the page:** `License(s): CC0`; link target
  `http://creativecommons.org/publicdomain/zero/1.0/`.
  The attribution note reads "Link to Opengameart maybe."
- **Where on the page:** the `License(s):` field in the metadata column.
- **Bytes:** 119,570 · **Duration:** 5.147 s · **rate** 44,100 · **ch** 2 ·
  peak 0.578, RMS 0.176, 0 samples at full scale
- **Why this one:** the author's own description is "Tried to make that weird
  horn sound you hear in all movies today. Maybe a boss intro." Measured: the
  fundamental is a rock-steady 73–76 Hz for the whole five seconds — no
  melodic movement, just a held low horn — with 79 % of the energy below
  200 Hz and a centroid that falls 1895 → 1088 Hz. It is lower and longer than
  `elite_arrival`, which is precisely the relationship the brief asked for.
- **Caveat:** **5.15 s against a 1.5–3.5 s window.** It is the longest file in
  the batch and the only one that overshoots its window in the other direction.
  Nothing CC0 of the right length had this character; the alternatives were
  Kenney `explosionCrunch_004.ogg` (1.98 s) and `lowFrequency_explosion_001.ogg`
  (1.00 s), both of which read as explosions rather than as an arrival.
- **`theircoming_0.ogg`** from the same page is a 14.0 s variant and was
  rejected on length.

```
00000000: 4f67 6753 0002 0000 0000 0000 0000 f76b  OggS...........k
00000010: 0000 0000 0000 d2fe 87ad 011e 0176 6f72  .............vor
```

---

## 9. `sanity_critical.wav` — 397,080 B, 2.25 s, stereo, 44.1 kHz, 32-bit float

- **Original asset:** `heartbeat_fast_0.wav`
- **Source page:** https://opengameart.org/content/heartbeat-sounds
- **Direct link:** https://opengameart.org/sites/default/files/heartbeat_fast_0.wav
- **Author:** bart
- **Licence string as shown on the page:** `License(s): CC0`; link target
  `http://creativecommons.org/publicdomain/zero/1.0/`. The page is also in
  OGA's "CC0 Audio" collection.
- **Where on the page:** the `License(s):` field in the metadata column.
- **Bytes:** 397,080 · **Duration:** 2.250 s · **rate** 44,100 · **ch** 2 ·
  IEEE float PCM (`fmt` chunk `03 00`, 2 channels, 32 bits) · peak 0.207,
  RMS 0.036, 0 samples at full scale
- **Why this one:** the sibling `heartbeat_slow_0.wav` / `heartbeat_slow_reverb.wav`
  are both 3.6 s; this is the fast variant at 2.25 s, the closest to the
  0.8–2 s window. **100 % of its energy is below 200 Hz**, the fundamental sits
  at 40 Hz, and the onset detector counts 12 events — i.e. several heartbeat
  thumps in a row, not one hit. Its low level (peak 0.21) also means it will sit
  *under* gunfire rather than startle, which is what the slot's own doc comment
  in `src/audio.rs` asks for ("a warning, not a scare").
- **Caveat:** the brief also asked for a tinnitus/whisper layer. This file is
  heartbeat only — there is no high-frequency component whatsoever. It is
  2.25 s against a 2.0 s ceiling.

```
00000000: 5249 4646 100f 0600 5741 5645 666d 7420  RIFF....WAVEfmt
00000010: 1000 0000 0300 0200 44ac 0000 2062 0500  ........D... b..
```

---

## Verification that was actually run

Container and length, directly on the nine files in `assets/sounds/`:

```
$ for f in healing.ogg pickup_health.ogg pickup_ammo.wav objective_done.wav \
           mission_complete.wav mission_failed.ogg elite_arrival.ogg \
           boss_arrival.ogg sanity_critical.wav; do
    echo "===== $f ($(stat -c %s "$f") bytes)"; xxd "$f" | head -2
  done
```

gives the `xxd` blocks quoted above — every Ogg starts `4f67 6753` ("OggS")
with `01 1e 01 76 6f 72` (`\x01vorbis`) at offset 28, and every WAV starts
`5249 4646` / `5741 5645` ("RIFF"/"WAVE"). The Ogg page walker additionally
reported, for all five Oggs, `trailing bytes=0, last page complete=True, EOS
flag on last page=True` and an `\x01vorbis` ident header, which is exactly what
`audio.rs::ogg_ok` tests for. The WAV walker reported the RIFF size field equal
to `actual bytes - 8` on all four, i.e. nothing truncated, with `fmt` format 1
and 16 bits (PCM) for the three 16-bit files and format 3 / 32 bits (float) for
`sanity_critical.wav` — both accepted by `audio.rs::wav_ok`.

Levels were measured by decoding each file and normalising integer PCM to
±1.0. No file has a single sample at full scale; the hottest is
`objective_done.wav` at 0.9975 and the quietest `sanity_critical.wav` at 0.2071.

`sha256sum` over each staging copy and its destination in `assets/sounds/`
matched for all nine, so what is on disk is byte-for-byte what was downloaded.

`du -cb assets/sounds/*.ogg assets/sounds/*.wav` reports **1,082,987 bytes**
across all 28 files now in `assets/sounds/` (the 19 pre-existing ones plus these
nine). The nine new files alone are **857,509 bytes (838 KiB)**; the largest
single file is `sanity_critical.wav` at 397,080 B, and the whole batch is well
inside the 2 MB-per-file / 12 MB-total budget.

---

## Where the file is read from

`load_effect()` (`src/audio.rs`) probes `assets/sounds/<name>.{ogg,wav}`
**relative to the process working directory** first, then falls back to the
compiled-in copy (`Sfx::asset()` → `Asset::bytes()`), then to the synthesiser.

All nine of these have a compiled-in copy, so they are heard wherever the game is
launched from. The probe-then-embedded order means a replacement file dropped
into `assets/sounds/` still wins over the embedded one without a rebuild.

One wrinkle worth recording: the embedded copy has to be told which container it
is (`wav_ok` or `ogg_ok`). The check used to say "ogg" for every asset, which
made every embedded **WAV** fail it and fall back to the synthesiser in silence.
Four of these nine are WAVs, so this batch is what would have exposed it.

## Not done

- **`reload_done`** (`Sfx::ReloadDone`, `src/audio.rs:130`) is a real slot with
  no file, and it was not on the brief's list of nine, so no file was added for
  it. It is the one remaining gap. Good local candidates already on disk:
  `_downloads/kenney_ui-audio/Audio/switch28.ogg` (0.20 s) or
  `_downloads/kenney_rpg.../metalLatch.ogg` (0.26 s, but peaks at 1.20).
- The source archives were **not** copied into `assets/_downloads/`. They are
  all re-fetchable from the direct links above; staging happened in a temp
  directory. `assets/MANIFEST.md` was deliberately not touched.

> **Correction (2026-10-03, later the same day).** The last line above is no
> longer true: the thirteen weapon and combat cues were replaced in a second
> pass and `assets/MANIFEST.md` was updated for it (§2 and the new §2a). That
> second batch has its own chapter below.

---

# Part 2 — the thirteen weapon and combat cues, replaced 2026-10-03

The brief for this pass: the game's gunfire was Kenney sci-fi `laser*` material
("pew"), and the hits, reload, footstep and death cues were Kenney impacts. All
thirteen were to become **real recordings**, keeping the same file names so
nothing in `src/` had to change.

**Outcome: thirteen slots replaced, all thirteen CC0, all mono Ogg Vorbis at
44.1 kHz, 110,885 B total (the thirteen they replace were 144,344 B, so the
folder lost 33,459 B). `src/assets.rs` was NOT modified — every file kept its
`.ogg` name.**

## How the sources were found and checked

Everything came from **freesound.org** with the licence filter
`f=license:"Creative Commons 0"` in the search URL. The brief listed
OpenGameArt as the preferred source, so it was tried first; the one real
target-shooting pack found there (`/content/gunshot-sounds`, CZ-52 / Mosin
Nagant / SKS / shotgun) renders `License(s): CC0` on its page **but the
`creativecommons.txt` inside its own `sounds.zip` says "Creative Commons
Attribution 3.0 Unported"**. Page and bundle disagree, so it was dropped rather
than used with an ambiguous provenance. `gunshots` (CC0), `gunshot-sound-effect`
(CC0) and the reload packs were also fetched but lost to better material.

Every candidate that made it into `assets/sounds/` was then opened on its own
page and the licence re-read there. The check, per page:

1. fetch `https://freesound.org/s/<id>/` (it 302s to the canonical
   `/people/<user>/sounds/<id>/`),
2. read the licence block, which renders
   `<a href="http://creativecommons.org/publicdomain/zero/1.0/"
   title="Go to the full license text">Creative Commons 0</a>`,
3. collect **every** `creativecommons.org/...` link on the page and confirm
   there is exactly one and it is `publicdomain/zero/1.0/`.

All thirteen pass step 3 with a single CC0 link and no CC-BY or CC-BY-SA link
anywhere on the page.

Two traps this step caught, and why the descriptions were read too:

- **db465's `m4carbine_fire` / `m249_fire` / `barrett_fire` / `mp5_fire` /
  `ak74m_fire` / `aa12_fire`** — superficially a perfect one-shot-per-weapon
  CC0 set, all mono 44.1 kHz, all ~0.7 s. The sound page's own description says
  *"Physically Synthesized … It is NOT a field recording"*. Rejected: the brief
  asked for real gun recordings.
- `419...`/`Foley`/`music` hits, "cartoon", "8-bit", "sci-fi" and anything
  whose description named a plugin or a synthesiser were rejected the same way,
  before any waveform work.

## How the files were cut

No ffmpeg or sox exists on this machine, so the whole pipeline is
`uv run --no-project --python 3.12 --with av --with numpy --with soundfile`.

For each slot the script decodes the source to float, downmixes to mono,
resamples to 44.1 kHz with a zero-phase FFT resampler, picks the loudest sample
inside a hand-chosen search window, cuts a fixed window around it, applies a
30 Hz zero-phase high-pass (which also removes any DC offset), fades 3 ms in /
5 ms out, and encodes Ogg Vorbis through **libsndfile's libvorbis** (the
PyAV wheel on this machine has only FFmpeg's experimental native vorbis
encoder, which is worse; soundfile was used instead).

Level: the target is −1.0 dBFS, but Vorbis overshoots its input peak by an
amount that is not linear in level, so the gain cannot be solved in one step.
The script encodes, measures the decoded peak, adjusts and repeats up to 18
times, then keeps the loudest take that still decodes **at or below**
−1.0 dBFS. That is why the realised peaks sit between −1.00 and −1.30 dBFS
instead of exactly on the target.

Each cut was chosen from an RMS envelope printed at 5 ms resolution — the
attack and the natural decay tail are both inside the window, and nothing was
sliced out of music. The cut points are recorded per file below.

## What the pipeline was verified against

Two bugs were caught this way and are worth recording, because both would have
shipped silently:

- The first high-pass was written as `1/sqrt(1 + (f/fc)^(2n))`, which is the
  **low**-pass response. It left 100 % of every file's energy below 200 Hz —
  i.e. rumble. The tell was a spectral centroid of 44 Hz on a rifle shot. It is
  now `1/sqrt(1 + (fc/f)^(2n))`, and the build prints the source and output
  centroids side by side so a filter that moves the spectrum cannot pass: they
  now agree to within 1 %, e.g. `shot_sniper` 2209 → 2211 Hz.
- The first FFT resampler called `irfft(spec, n=n_out*2)`, which halved every
  frequency — every 48 kHz source came out an octave low. It was caught with a
  synthetic 1 kHz tone and an impulse, and is now `irfft(spec[:keep], n=n_out)`;
  a 1 kHz tone and a 0.200 s impulse come back at 1 kHz and 0.200 s from every
  input rate tried (22050 / 32000 / 44100 / 48000 / 96000).

Final verification, run over the thirteen files on disk: exact re-implementation
of `src/audio.rs::ogg_ok` (Ogg page walk, `\x01vorbis` ident header, no trailing
bytes) → **24/24 `assets/sounds/*.ogg` pass**; decode with PyAV *and* with
libsndfile and compare sample for sample → agree to ≤ 5 × 10⁻⁷; peak, length,
rate, channel count and clipped-sample count as tabulated below.

## The thirteen, one section each

Each entry gives: the original asset and its page, the exact URL that was
downloaded, the author, the licence string as the page renders it, the measured
properties of both the downloaded source and the file written, why that
recording was chosen, and the first 32 bytes of the file that is on disk.

**One caveat that applies to all thirteen.** These are freesound's **CDN
preview** files (`…-hq.ogg`), not the uploader's original WAV/MP3/AIFF — the
originals sit behind a login. The preview is a Vorbis transcode that freesound
publishes under the same CC0 licence, so the licence chain is intact; the cost
is that the very top of the spectrum is already lossy-coded once before the
game's Ogg is decoded. Every source is presented below with its own byte count
and rate so the originals can be re-fetched by anyone with an account if the
game ever wants the clean versions.

---

### 1. `shot_rifle.ogg` — 8,002 B, 0.500 s, mono, 44.1 kHz, peak 0.8846

- **Original asset:** `Rifle gunshot, one shot` (id 710084)
- **Source page:** https://freesound.org/people/felix.blume/sounds/710084/
- **Direct link:** https://cdn.freesound.org/previews/710/710084_1661766-hq.ogg
- **Author:** felix.blume (Félix Blume)
- **Licence string as shown on the page:** `Creative Commons 0`, link target
  `http://creativecommons.org/publicdomain/zero/1.0/`; that is the only
  `creativecommons.org` link on the page.
- **Source as downloaded:** 86,249 B · 4.264 s · 48,000 Hz · 2 ch · peak 0.949
- **Written file:** 8,002 B · 0.500 s · 44,100 Hz · 1 ch · peak 0.8846
  (−1.07 dBFS) · 0 samples at full scale · RMS 0.0687 · spectral centroid
  611 Hz (source 614 Hz)
- **Cut:** the single shot's peak is at 0.837 s in the source; the file is
  source seconds **0.812 → 1.312**.
- **Why this one:** this is the closest thing to a reference field recording of
  a rifle shot available under CC0 — a Schoeps CCM41 + CCM8 MS pair into a
  Sound Devices 744T, recorded outdoors in California in 2011, with no room and
  no processing. Its envelope is a hard crack followed by a fast decay and then
  digital silence — the peak is 25 ms into the 0.50 s file and the second half
  is the tail — which is exactly the shape the brief's 0.20–0.70 s window
  wanted, and it is the driest of all the rifle candidates so it layers cleanly
  when the weapon is held down in a long burst. The other 5.56-class candidates (`garand` M1,
  `enfield` .303, `ak47range`) are all multi-shot recordings with audible
  echo or room.

```
00000000: 4f 67 67 53 00 02 00 00 00 00 00 00 00 00 1c 02 OggS............
00000010: 92 0e 00 00 00 00 2a 22 15 5b 01 1e 01 76 6f 72 ......*".[...vor
```

---

### 2. `shot_shotgun.ogg` — 8,718 B, 0.570 s, mono, 44.1 kHz, peak 0.8911

- **Original asset:** `Saiga 12k Gunshot(12ga).mp3` (id 569309)
- **Source page:** https://freesound.org/people/Kobrakon47/sounds/569309/
- **Direct link:** https://cdn.freesound.org/previews/569/569309_12809197-hq.ogg
- **Author:** Kobrakon47
- **Licence string as shown on the page:** `Creative Commons 0`, link target
  `http://creativecommons.org/publicdomain/zero/1.0/`; only CC link on the page.
- **Source as downloaded:** 32,322 B · 1.835 s · 48,000 Hz · 2 ch · peak 0.758
- **Written file:** 8,718 B · 0.570 s · 44.1 kHz · 1 ch · peak 0.8911
  (−1.00 dBFS) · 0 samples at full scale · RMS 0.1229 · centroid 951 Hz
  (source 955 Hz)
- **Cut:** peak at 0.207 s; the file is seconds **0.182 → 0.752**.
- **Why this one:** the description is the author's own "Raw audio of me
  shooting my Saiga 12 using 00 Buckshot 12 Gauge, very punchy and loud" — a
  real 12-gauge, not a foley. Of the five player guns it has the most weight
  under 200 Hz (13.5 % of the file's energy, against the SMG's 2.0 % and the
  rifle's 5.1 %) and it is one of the two with a real low end at all; 76.1 % of
  it sits in the 200 Hz–2 kHz band, which is the "low and massive" profile the
  brief asked for. Compared with the
  other 12-gauge takes: `mossberg` (`AnthonyChan0`) has a +0.013 DC offset and
  reads 97 % sub-200 Hz once that is counted, `sg12_4shot` peaks at only 0.359
  and is four shots in a row, and `shotgun_uts` is four mixed takes with a
  resynthesised bass layer.

```
00000000: 4f 67 67 53 00 02 00 00 00 00 00 00 00 00 0c 72 OggS...........r
00000010: 76 28 00 00 00 00 64 15 1e 10 01 1e 01 76 6f 72 v(....d......vor
```

---

### 3. `shot_smg.ogg` — 7,337 B, 0.417 s, mono, 44.1 kHz, peak 0.8761

- **Original asset:** `9mm_gunshot.wav` (id 244138)
- **Source page:** https://freesound.org/people/mnslugger20/sounds/244138/
- **Direct link:** https://cdn.freesound.org/previews/244/244138_4121395-hq.ogg
- **Author:** mnslugger20
- **Licence string as shown on the page:** `Creative Commons 0`, link target
  `http://creativecommons.org/publicdomain/zero/1.0/`; only CC link on the page.
- **Source as downloaded:** 12,711 B · 1.068 s · 44,100 Hz · 2 ch · peak 0.951
- **Written file:** 7,337 B · 0.417 s · 44.1 kHz · 1 ch · peak 0.8761
  (−1.15 dBFS) · 0 samples at full scale · RMS 0.0726 · centroid 1083 Hz
  (source 1093 Hz)
- **Cut:** peak at 0.174 s; the file is seconds **0.149 → 0.566**.
- **Why this one:** the brief says the slot fires up to 18 rounds a second, so
  it asked
  for a *short, crisp* report and this is the shortest clean single gunshot of
  all the candidates: the shot is over and back to digital silence in ~0.35 s,
  which is what stops 18 overlapping copies from turning into mush. It has the
  least low end of the five player guns (2.0 % of its energy below 200 Hz
  against the shotgun's 13.5 %) and its energy is concentrated in the mid band
  (85.0 %), so it cannot be confused with the shotgun.
- **Caveat, from the author's own description:** *"Field recording of 12 gauge
  shotgun sped up 2x, result is a handgun caliber-like report."* It is a real
  recording, but it is a 12-gauge pitched up, not a real 9 mm. A real 9 mm
  single shot was not available under CC0 at the time: the full-auto MP5A3
  street recording (201671) is a continuous burst with street echo and no
  isolated shot, and the pistol takes (427592, 217805) are either 40 % louder
  than everything else or 32 % of energy above 2 kHz.

```
00000000: 4f 67 67 53 00 02 00 00 00 00 00 00 00 00 fc 35 OggS...........5
00000010: f6 09 00 00 00 00 c7 8b d4 d4 01 1e 01 76 6f 72 .............vor
```

---

### 4. `shot_sniper.ogg` — 9,708 B, 0.680 s, mono, 44.1 kHz, peak 0.8899

- **Original asset:** `Sniper Shot in Field 2 (M2010 Enhanced Sniper Rifle
  ESR)` (id 855607)
- **Source page:** https://freesound.org/people/qubodup/sounds/855607/
- **Direct link:** https://cdn.freesound.org/previews/855/855607_71257-hq.ogg
- **Author:** qubodup
- **Licence string as shown on the page:** `Creative Commons 0`, link target
  `http://creativecommons.org/publicdomain/zero/1.0/`; only CC link on the page.
- **Source as downloaded:** 22,777 B · 1.543 s · 48,000 Hz · 1 ch · peak 0.635
- **Written file:** 9,708 B · 0.680 s · 44.1 kHz · 1 ch · peak 0.8899
  (−1.01 dBFS) · 0 samples at full scale · RMS 0.1078 · centroid 2209 Hz
  (source 2209 Hz)
- **Cut:** peak at 0.019 s; the file is seconds **0.014 → 0.694**.
- **Why this one:** the brief wanted the sniper to be the one with a blast and
  a reverb tail, and this is a genuine M2010 ESR — the .300 Win Mag chassis
  rifle the US Army adopted as the M2010 — fired on a live range. qubodup cut
  it out of a US National Guard marksmanship-training video (public-domain
  source, CC0 re-licence, attributed in the description). It is the most
  broadband of the five player guns (54 % of energy above 2 kHz) and it keeps
  ringing for ~0.25 s after the crack, which is the tail the brief asked for.
  `sniper_muffled` (815477) was rejected for being far too bright and short
  (centroid 2792 Hz, 0.62 s) and the .50 `Barrett` material for being
  synthesised (see the db465 note above).

```
00000000: 4f 67 67 53 00 02 00 00 00 00 00 00 00 00 ec 4d OggS...........M
00000010: d8 76 00 00 00 00 59 bc f1 7e 01 1e 01 76 6f 72 .v....Y..~...vor
```

---

### 5. `shot_lmg.ogg` — 8,955 B, 0.580 s, mono, 44.1 kHz, peak 0.8781

- **Original asset:** `M240 Machine Gun Single Shot` (id 854641)
- **Source page:** https://freesound.org/people/qubodup/sounds/854641/
- **Direct link:** https://cdn.freesound.org/previews/854/854641_71257-hq.ogg
- **Author:** qubodup
- **Licence string as shown on the page:** `Creative Commons 0`, link target
  `http://creativecommons.org/publicdomain/zero/1.0/`; only CC link on the page.
- **Source as downloaded:** 13,493 B · 0.786 s · 48,000 Hz · 1 ch · peak 1.052
  (the preview's codec overshoots; it is decoded, not trusted, before use)
- **Written file:** 8,955 B · 0.580 s · 44.1 kHz · 1 ch · peak 0.8781
  (−1.13 dBFS) · 0 samples at full scale · RMS 0.1094 · centroid 1371 Hz
  (source 1421 Hz)
- **Cut:** peak at 0.082 s; the file is seconds **0.042 → 0.622**.
- **Why this one:** the same qubodup treatment, this time a real M240 7.62 mm
  general-purpose machine gun (a US/UK live-fire exercise in Pabradė,
  Lithuania), which is exactly the "heavy" register the brief asked for and a
  different weapon class from the rifle's 5.56 — and it measures that way: it
  is the only player gun with a substantial share in every band (41.6 % below
  200 Hz, 33.3 % mid, 25.1 % above 2 kHz), i.e. a full-bodied boom rather than
  the rifle's mid-only crack (92.7 % mid). `mg_pgi` (212607) is the darker
  alternative at a 356 Hz centroid but only 2.3 % of its energy is above 2 kHz,
  so it reads as a distant thud. The M240's 0.786 s source is already the right
  length, so this cut is nearly the whole recording.

```
00000000: 4f 67 67 53 00 02 00 00 00 00 00 00 00 00 2c e6 OggS..........,.
00000010: c1 23 00 00 00 00 8f db 85 82 01 1e 01 76 6f 72 .#...........vor
```

---

### 6. `enemy_shot.ogg` — 9,682 B, 0.650 s, mono, 44.1 kHz, peak 0.8840

- **Original asset:** `AR15 rifle shot from 50 yards away` (id 427597)
- **Source page:** https://freesound.org/people/michorvath/sounds/427597/
- **Direct link:** https://cdn.freesound.org/previews/427/427597_3094998-hq.ogg
- **Author:** michorvath
- **Licence string as shown on the page:** `Creative Commons 0`, link target
  `http://creativecommons.org/publicdomain/zero/1.0/`; only CC link on the page.
- **Source as downloaded:** 49,190 B · 3.893 s · 44,100 Hz · 1 ch · peak 1.225
  (codec overshoot again)
- **Written file:** 9,682 B · 0.650 s · 44.1 kHz · 1 ch · peak 0.8840
  (−1.07 dBFS) · 0 samples at full scale · RMS 0.3468 · centroid 1085 Hz
  (source 1080 Hz)
- **Cut:** peak at 0.674 s; the file is seconds **0.649 → 1.299**.
- **Why this one:** the brief said the enemy shot should be a *different* one of
  the player guns, and this is the most different thing that is still a rifle:
  the same AR15 family as the player's rifle, but recorded 50 yards downrange,
  so the crack arrives with a long ground-and-wall echo and the whole 0.65 s
  stays loud (RMS 0.3468, five times the player rifle's 0.0687) instead of
  decaying into silence. That reads as "someone else is shooting, over there",
  which is exactly what the slot is for.

```
00000000: 4f 67 67 53 00 02 00 00 00 00 00 00 00 00 9c 52 OggS...........R
00000010: 7e 2c 00 00 00 00 e7 5a 82 13 01 1e 01 76 6f 72 ~,.....Z.....vor
```

---

### 7. `reload.ogg` — 16,667 B, 1.450 s, mono, 44.1 kHz, peak 0.8910

- **Original asset:** `Assault Rifle Reload` (id 815879)
- **Source page:** https://freesound.org/people/qubodup/sounds/815879/
- **Direct link:** https://cdn.freesound.org/previews/815/815879_71257-hq.ogg
- **Author:** qubodup
- **Licence string as shown on the page:** `Creative Commons 0`, link target
  `http://creativecommons.org/publicdomain/zero/1.0/`; only CC link on the page.
- **Source as downloaded:** 29,017 B · 1.870 s · 44,100 Hz · 1 ch · peak 1.125
  (codec overshoot)
- **Written file:** 16,667 B · 1.450 s · 44.1 kHz · 1 ch · peak 0.8910
  (−1.00 dBFS) · 0 samples at full scale · RMS 0.0590 · centroid 1396 Hz
  (source 1442 Hz)
- **Cut:** the whole 1.450 s from the front, seconds **0.000 → 1.450**.
- **Why this one:** it is a complete magazine change with the mechanism audible
  — magazine out, magazine in, bolt — with three distinct transients in the
  envelope at ≈0.0–0.15 s, 0.68–0.92 s and 1.34–1.46 s, so the sound tracks
  the reload animation rather than being one anonymous clatter. It was trimmed
  from qubodup's own longer edit to exactly 1.45 s to stay under the brief's
  1.5 s ceiling; the charging-handle release at 1.72 s in the source is the
  part that was dropped, and it lands on `reload_empty` instead. qubodup's
  description records that the file is an edit of two CC0 recordings by
  serøutōnin (`sounds/725403` and `sounds/674748`), both CC0, so the chain stays
  clean; `Kodack`'s 3.491 s take was the runner-up but has three times the
  length and a 5672 Hz centroid, i.e. it is mostly small high clicks.

```
00000000: 4f 67 67 53 00 02 00 00 00 00 00 00 00 00 4c aa OggS..........L.
00000010: 82 31 00 00 00 00 ee cf 73 19 01 1e 01 76 6f 72 .1......s....vor
```

---

### 8. `reload_empty.ogg` — 6,372 B, 0.247 s, mono, 44.1 kHz, peak 0.8903

- **Original asset:** `Remington 870 pull up and dry fire` (id 702662)
- **Source page:** https://freesound.org/people/areniporgen/sounds/702662/
- **Direct link:** https://cdn.freesound.org/previews/702/702662_15072041-hq.ogg
- **Author:** areniporgen
- **Licence string as shown on the page:** `Creative Commons 0`, link target
  `http://creativecommons.org/publicdomain/zero/1.0/`; only CC link on the page.
- **Source as downloaded:** 21,448 B · 0.856 s · 48,000 Hz · 2 ch · peak 0.837
- **Written file:** 6,372 B · 0.247 s · 44.1 kHz · 1 ch · peak 0.8903
  (−1.01 dBFS) · 0 samples at full scale · RMS 0.0849 · centroid 2550 Hz
  (source 2521 Hz)
- **Cut:** the dry-fire click peaks at 0.656 s; the file is seconds
  **0.606 → 0.853**, i.e. it runs to the end of the source.
- **Why this one:** the slot is the trigger falling on an empty chamber — a
  bare metallic click with nothing behind it — and this is a real one: the
  author's Remington 870 Express Tactical, the take where the hammer drops on
  an empty chamber. It has the third-highest spectral centroid of the thirteen
  (2550 Hz, after `hit_flesh` at 2921 Hz and `hit_world` at 2674 Hz) so it cuts
  through a firefight, and it is 0.25 s, short
  enough to read as a click and not a clatter. The earlier event in the same
  file (0.08–0.16 s, the "pull up") was left out on purpose. The Barrett
  charging-handle recording (855598, qubodup) was the alternative and is
  arguably the better mechanism sound, but it is a two-handed load-n-lock
  rather than an empty click.

```
00000000: 4f 67 67 53 00 02 00 00 00 00 00 00 00 00 3c 1e OggS..........<.
00000010: bf 0a 00 00 00 00 3e 33 a7 db 01 1e 01 76 6f 72 ......>3.....vor
```

---

### 9. `hit_flesh.ogg` — 6,835 B, 0.382 s, mono, 44.1 kHz, peak 0.8899

- **Original asset:** `Three Flesh Impacts.wav` (id 344409)
- **Source page:** https://freesound.org/people/jawbutch/sounds/344409/
- **Direct link:** https://cdn.freesound.org/previews/344/344409_4986214-hq.ogg
- **Author:** jawbutch
- **Licence string as shown on the page:** `Creative Commons 0`, link target
  `http://creativecommons.org/publicdomain/zero/1.0/`; only CC link on the page.
- **Source as downloaded:** 83,377 B · 5.539 s · 48,000 Hz · 2 ch · peak 0.509
- **Written file:** 6,835 B · 0.382 s · 44.1 kHz · 1 ch · peak 0.8899
  (−1.01 dBFS) · 0 samples at full scale · RMS 0.0751 · centroid 2921 Hz
  (source 2967 Hz)
- **Cut:** the third and loudest of the three impacts peaks at 4.169 s; the file
  is seconds **4.144 → 4.526**.
- **Why this one:** the author uploaded three takes of the same impact and said
  so ("there's three here for you to work with"); the third is the loudest
  (env 0.1785 against 0.1231 and 0.1431) and the most isolated, which is why it
  was taken. Its centroid — 2921 Hz, the highest of the thirteen — and the
  69.1 % of its energy above 2 kHz are
  what make it read as wet tissue rather than another metal
  hit, the opposite of the old Kenney
  `impactPunch_medium` it replaces. `VisceralBulletImpacts` (423301) is closer
  to the brief's words but is a continuous stream of whizzes and impacts with
  no clean single hit to cut, and `Meaty Damage Impact` (779805) is explicitly
  built from "spectral compression artifacts", i.e. not a recording.

```
00000000: 4f 67 67 53 00 02 00 00 00 00 00 00 00 00 7c 46 OggS..........|F
00000010: ed 28 00 00 00 00 c4 e7 91 8e 01 1e 01 76 6f 72 .(...........vor
```

---

### 10. `hit_head.ogg` — 7,019 B, 0.247 s, mono, 44.1 kHz, peak 0.8740

- **Original asset:** `crush1.wav` (id 703115)
- **Source page:** https://freesound.org/people/franzzle/sounds/703115/
- **Direct link:** https://cdn.freesound.org/previews/703/703115_1823770-hq.ogg
- **Author:** franzzle
- **Licence string as shown on the page:** `Creative Commons 0`, link target
  `http://creativecommons.org/publicdomain/zero/1.0/`; only CC link on the page.
- **Source as downloaded:** 11,906 B · 0.450 s · 44,100 Hz · 1 ch · peak 0.804
- **Written file:** 7,019 B · 0.247 s · 44.1 kHz · 1 ch · peak 0.8740
  (−1.17 dBFS) · 0 samples at full scale · RMS 0.0687 · centroid 1481 Hz
  (source slice 1551 Hz, whole source 2293 Hz)
- **Cut:** the opening crack peaks at 0.039 s; the file is the first 0.247 s,
  source seconds **0.024 → 0.271**. A second, louder crack sits at 0.36 s in the
  source and was deliberately left out so the headshot stays one hit; the
  crackle between the two is kept, which is what makes it a bone shatter rather
  than a single tick.
- **Why this one:** the headshot slot wants bone, and there is no CC0 "skull
  crack" that is actually a skull crack. This one is honest about what it is —
  the author's description says "This was actually some very thin ice I found
  in the grass. I cracked it and recorded the sound of it. I think you can use
  this as a bone or a cheap skull crack in a video game" — and it is a single
  recorded transient with no processing, which none of the vegetable-foley
  alternatives can match. (It is not a clean edit: recorded ice keeps
  crackling after the first snap, so the quietest 0.17 s of the source still
  peaks at 0.41, a 5.5 dB gap under the crack. That residual crackle is left in
  the file rather than gated out.) It is the dry,
  hollow counterpart to `hit_flesh.ogg`: 1481 Hz centroid against the flesh
  hit's 2921 Hz, and only 6.9 % of its energy in the 200 Hz–2 kHz band against
  the flesh hit's 25 %, so the two hit markers are told apart by ear. Rejected:
  `Headshot.wav` (608804), which is a *voice line saying "head shot"*;
  `Skull Shot` (699699), a cleaned-up version of the same idea; and the
  celery / foam-sheet bone foleys (`Bone Break (Celery)` 471594 at a 6521 Hz
  centroid, `BONE_BREAK_01` 449615 at 10,919 Hz), which measure as paper-like
  snaps rather than bone.

```
00000000: 4f 67 67 53 00 02 00 00 00 00 00 00 00 00 bc 2b OggS...........+
00000010: 1d 3d 00 00 00 00 d9 a8 dc 06 01 1e 01 76 6f 72 .=...........vor
```

---

### 11. `hit_world.ogg` — 7,166 B, 0.421 s, mono, 44.1 kHz, peak 0.8612

- **Original asset:** `S31-15 Shooting gallery; hitting metal targets.wav`
  (id 675951)
- **Source page:** https://freesound.org/people/craigsmith/sounds/675951/
- **Direct link:** https://cdn.freesound.org/previews/675/675951_2524442-hq.ogg
- **Author:** craigsmith
- **Licence string as shown on the page:** `Creative Commons 0`, link target
  `http://creativecommons.org/publicdomain/zero/1.0/`; only CC link on the page.
- **Source as downloaded:** 349,414 B · 35.327 s · 48,000 Hz · 1 ch · peak 0.600
- **Written file:** 7,166 B · 0.421 s · 44.1 kHz · 1 ch · peak 0.8612
  (−1.30 dBFS) · 0 samples at full scale · RMS 0.1471 · centroid 2674 Hz
  (source slice 2673 Hz)
- **Cut:** the loudest isolated target hit in the 35 s source peaks at 8.477 s;
  the file is seconds **8.452 → 8.873**.
- **Why this one:** this is a real bullet striking a real steel target, not a
  foley imitation — the source is a 35 mm optical-library recording of a
  shooting gallery, and its envelope is a row of clean, well-separated impacts
  (RMS-envelope peaks at 1.84, 2.86, 5.39, 8.48, 11.10, 12.92, 17.06, 19.80,
  21.79, 27.09 s), which is what made it possible to take one hit and nothing
  else. Its 2674 Hz centroid and 78.8 % of energy above 2 kHz are the highest of
  the thirteen, which is what a bullet on arena steel should measure. This slot
  is the one that most benefits from the swap: the old file was Kenney
  `impactPlate_medium_000`, another foley plate. Rejected candidates:
  `Basic Metal Clang Free` (842171), a blacksmith-style clang that rings too
  long and too pitched, `Concrete Hit` (321477), a real recording but of an axe
  on a paving slab and only 510 Hz centroid, and the `Foley bullet hit metal`
  set (182263, 267893), all balloon-pop or synthetic foley.

```
00000000: 4f 67 67 53 00 02 00 00 00 00 00 00 00 00 5c 8a OggS..........\.
00000010: 45 61 00 00 00 00 eb 67 5b 30 01 1e 01 76 6f 72 Ea.....g[0...vor
```

---

### 12. `enemy_death.ogg` — 8,246 B, 0.550 s, mono, 44.1 kHz, peak 0.8783

- **Original asset:** `Grunt1 - Death Pain.wav` (id 416839)
- **Source page:** https://freesound.org/people/tonsil5/sounds/416839/
- **Direct link:** https://cdn.freesound.org/previews/416/416839_8247784-hq.ogg
- **Author:** tonsil5
- **Licence string as shown on the page:** `Creative Commons 0`, link target
  `http://creativecommons.org/publicdomain/zero/1.0/`; only CC link on the page.
- **Source as downloaded:** 14,651 B · 1.031 s · 44,100 Hz · 1 ch · peak 0.885
- **Written file:** 8,246 B · 0.550 s · 44.1 kHz · 1 ch · peak 0.8783
  (−1.13 dBFS) · 0 samples at full scale · RMS 0.1288 · centroid 354 Hz
  (source 358 Hz)
- **Cut:** the grunt peaks at 0.207 s; the file is seconds **0.127 → 0.677** —
  it starts 80 ms before the peak so the breath before the grunt is kept.
- **Why this one:** the brief wanted the death cue to be a body, and this is a
  human death grunt rather than another impact. Its centroid of 354 Hz makes it
  the lowest thing in the set by a wide margin (the next lowest is `footstep` at
  536 Hz, and the lowest gun is `shot_rifle` at 611 Hz), and 99 % of its energy
  is below 2 kHz, so it reads as a voice and never as a hit — it cannot be
  mistaken for a bullet impact, which matters because the slot can fire in the
  same frame as one. The attack is a gradual 80 ms swell rather than a
  transient, for the same reason. The source is 1.031 s and the
  grunt occupies only 0.08–0.45 s, so the 0.55 s file is the event plus silence
  and stays clear of the 1.5 s ceiling. Rejected: `MaleDeathSound12` (577033),
  good but 1.74 s before trimming, and `Headshot.wav` (608804) and the
  voice-line "head shot" material, which is spoken text, not a death.

```
00000000: 4f 67 67 53 00 02 00 00 00 00 00 00 00 00 ac d6 OggS............
00000010: 54 2d 00 00 00 00 80 58 7e 96 01 1e 01 76 6f 72 T-.....X~....vor
```

---

### 13. `footstep.ogg` — 6,178 B, 0.257 s, mono, 44.1 kHz, peak 0.8815

- **Original asset:** `Hiking Boot Footsteps on Stone` (id 521590)
- **Source page:** https://freesound.org/people/Fission9/sounds/521590/
- **Direct link:** https://cdn.freesound.org/previews/521/521590_9395330-hq.ogg
- **Author:** Fission9
- **License string as shown on the page:** `Creative Commons 0`, link target
  `http://creativecommons.org/publicdomain/zero/1.0/`; only CC link on the page.
- **Source as downloaded:** 26,312 B · 3.549 s · 48,000 Hz · 1 ch · peak 0.611
- **Written file:** 6,178 B · 0.257 s · 44.1 kHz · 1 ch · peak 0.8815
  (−1.10 dBFS) · 0 samples at full scale · RMS 0.0702 · centroid 536 Hz
  (source 539 Hz)
- **Cut:** the third of the six steps peaks at 1.271 s; the file is seconds
  **1.241 → 1.501**.
- **Why this one:** the brief wanted a boot on hard ground and the author's
  description gives the recording for free — "Six footsteps on stone with
  hiking boots. Each step is separated by about 250 ms of silence. Recorded
  with a H4n Pro. Edited with Audacity." The steps are separated by near-exact
  digital silence — the longest gaps measure a peak of exactly 0 and the worst
  is 3 × 10⁻⁵ of full scale — so a single one can be cut with no cross-talk;
  the six candidates sit
  at 0.106, 0.713, 1.268, 1.818, 2.407 and 3.025 s and the 1.268 s one is the
  loudest by a 10 ms RMS envelope (0.221; the next is 0.217) while the 1.818 s
  one has the highest single sample (0.612). It is the same recording qubodup
  used for the FPS
  footstep loop in AssaultCube. Two other CC0 footstep files were rejected for
  being noisy rather than quiet: `snd_footsteps_concrete` (562192, only 6 dB
  between steps) and `Footstep Floor` (842186, 13 dB, and it peaks at 0.10).

```
00000000: 4f 67 67 53 00 02 00 00 00 00 00 00 00 00 84 46 OggS...........F
00000010: f9 1b 00 00 00 00 75 c1 8e 75 01 1e 01 76 6f 72 ......u..u...vor
```

---

## The thirteen, tabulated

| Slot | New file | Bytes | Len | Rate | ch | Peak | dBFS | Source id | Source author |
| --- | --- | ---: | ---: | ---: | :-: | ---: | ---: | --- | --- |
| `Sfx::ShotRifle` | `shot_rifle.ogg` | 8,002 | 0.500 s | 44,100 | 1 | 0.8846 | −1.07 | 710084 | felix.blume |
| `Sfx::ShotShotgun` | `shot_shotgun.ogg` | 8,718 | 0.570 s | 44,100 | 1 | 0.8911 | −1.00 | 569309 | Kobrakon47 |
| `Sfx::ShotSmg` | `shot_smg.ogg` | 7,337 | 0.417 s | 44,100 | 1 | 0.8761 | −1.15 | 244138 | mnslugger20 |
| `Sfx::ShotSniper` | `shot_sniper.ogg` | 9,708 | 0.680 s | 44,100 | 1 | 0.8899 | −1.01 | 855607 | qubodup |
| `Sfx::ShotLmg` | `shot_lmg.ogg` | 8,955 | 0.580 s | 44,100 | 1 | 0.8781 | −1.13 | 854641 | qubodup |
| `Sfx::EnemyShot` | `enemy_shot.ogg` | 9,682 | 0.650 s | 44,100 | 1 | 0.8840 | −1.07 | 427597 | michorvath |
| `Sfx::Reload` | `reload.ogg` | 16,667 | 1.450 s | 44,100 | 1 | 0.8910 | −1.00 | 815879 | qubodup |
| `Sfx::ReloadEmpty` | `reload_empty.ogg` | 6,372 | 0.247 s | 44,100 | 1 | 0.8903 | −1.01 | 702662 | areniporgen |
| `Sfx::HitFlesh` | `hit_flesh.ogg` | 6,835 | 0.382 s | 44,100 | 1 | 0.8899 | −1.01 | 344409 | jawbutch |
| `Sfx::HitHead` | `hit_head.ogg` | 7,019 | 0.247 s | 44,100 | 1 | 0.8740 | −1.17 | 703115 | franzzle |
| `Sfx::HitWorld` | `hit_world.ogg` | 7,166 | 0.421 s | 44,100 | 1 | 0.8612 | −1.30 | 675951 | craigsmith |
| `Sfx::EnemyDeath` | `enemy_death.ogg` | 8,246 | 0.550 s | 44,100 | 1 | 0.8783 | −1.13 | 416839 | tonsil5 |
| `Sfx::Footstep` | `footstep.ogg` | 6,178 | 0.257 s | 44,100 | 1 | 0.8815 | −1.10 | 521590 | Fission9 |

**110,885 B** for the thirteen, against 144,344 B for the files they replaced —
`assets/sounds/` is 33,459 B smaller than it was.

## Verification that was actually run on the thirteen

Container: an exact re-implementation of `src/audio.rs::ogg_ok` — walk every Ogg
page, require `OggS` and version byte 0, require the first seven body bytes to
be `\x01vorbis`, require the walk to consume the file exactly — reports
**24/24 `assets/sounds/*.ogg` pass, 0 trailing bytes, EOS flag set on the last
page**.

Decode: every one of the thirteen was decoded twice, once with PyAV
(FFmpeg's Vorbis decoder) and once with libsndfile 1.2.2 (libvorbis), and the
two renderings compared sample for sample — the last column below is the
largest per-sample disagreement, and it never exceeds 5 × 10⁻⁷, i.e.
floating-point noise. Both decoders report the same peak for every file.

```
$ uv run --no-project --python 3.12 --with av --with numpy --with soundfile \
      python verify.py   # output transcribed verbatim

=== ogg_ok (exact port of src/audio.rs::ogg_ok) over assets/sounds/*.ogg ===
24/24 pass

=== the thirteen, decoded twice and measured ===
file           bytes   dur_s   rate  ch      peak     dBFS  clip      rms centroid  pages
shot_rifle      8002   0.500  44100   1   0.88456    -1.07     0   0.0687      611 4/2e-07
shot_shotgun    8718   0.570  44100   1   0.89106    -1.00     0   0.1229      951 4/3e-07
shot_smg        7337   0.417  44100   1   0.87614    -1.15     0   0.0726     1083 3/2e-07
shot_sniper     9708   0.680  44100   1   0.88994    -1.01     0   0.1078     2211 4/3e-07
shot_lmg        8955   0.580  44100   1   0.87808    -1.13     0   0.1094     1371 4/3e-07
enemy_shot      9682   0.650  44100   1   0.88403    -1.07     0   0.3468     1085 4/5e-07
reload         16667   1.450  44100   1   0.89097    -1.00     0   0.0590     1396 6/2e-07
reload_empty    6372   0.247  44100   1   0.89034    -1.01     0   0.0849     2550 3/2e-07
hit_flesh       6835   0.382  44100   1   0.88991    -1.01     0   0.0751     2921 3/2e-07
hit_head        7019   0.247  44100   1   0.87402    -1.17     0   0.0687     1481 3/1e-07
hit_world       7166   0.421  44100   1   0.86116    -1.30     0   0.1471     2674 3/3e-07
enemy_death     8246   0.550  44100   1   0.87834    -1.13     0   0.1288      354 4/3e-07
footstep        6178   0.257  44100   1   0.88148    -1.10     0   0.0702      536 3/2e-07
total new = 110885 bytes

=== distinctness across the whole folder ===
28 files, 28 distinct sha256, duplicates: none
```

`clip` counts samples with `|sample| >= 32767`; it is 0 for all thirteen, and
no file comes closer than −1.00 dBFS, so nothing can clip in `quad-snd`'s float
mixer on the way to the device. `pages` is Ogg pages / the cross-decoder
disagreement.

The envelopes, in the same 40-bucket, 60 dB view used while cutting them. Every
shot's attack lands in the first 8 % of its file and decays into a tail;
`enemy_shot` is visibly the odd one out (near full scale for the whole 0.65 s —
the 50-yard echo), and `reload` shows its three separate mechanism events:

```
shot_rifle    0.500s ▁▇█▆▄▃▂▂▂▂▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁
shot_shotgun  0.570s ▇█▇█▇▅▄▃▂▂▁▁▂▂▂▂▂▂▂▂▂▂▂▂▁▁▂▁▁▂▁▁▁▁▁▁▁▁▁▁
shot_smg      0.417s ▁▁█▅▄▂▃▂▂▁▂▁▁▁▂▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁
shot_sniper   0.680s ██▅▄▄▄▃▃▁▁▂▂▃▄▂▃▃▂▂▁▁▁▂▂▃▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁
shot_lmg      0.580s ▁▁█▆▄▂▂▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁
enemy_shot    0.650s ▇▇█▇▇█▇█▇██▇█▇█▇▇█▇▆▆▇▇▆▅▅▅▆▄▃▄▅▄▅▄▃▃▃▄▂
reload        1.450s █▂▁▂▁▁▁▁▁▁▁▁▁▂▆▄▁▅▆▂▂▁▁▁▁▁▁▁▁▁▁▁▁▁▂▂█▃▁▁
reload_empty  0.247s ▁▁▃▃▅▄▂▂█▇█▆▃▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁
hit_flesh     0.382s ▁▄█▆▅▄▄▃▂▂▂▂▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁
hit_head      0.247s ▁▂█▅▄▂▂▂▁▆▂▂▂▁▁▁▁▂▃▃▃▁▁▂▁▃▃▂▄▃▂▁▂▁▁▅▃▁▁▁
hit_world     0.421s ▅█████▇▇▅▅▅▅▅▅▄▃▄▃▃▂▃▃▂▂▂▂▂▂▁▁▂▁▁▁▁▁▁▁▁▁
enemy_death   0.550s ▄▅▅▆▆▇█▇▆▅▄▃▃▂▃▂▂▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁
footstep      0.257s ▁▁▁▆█▂▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁
```

Distinctness: `sha256` over all **28** files in `assets/sounds/` gives 28
distinct digests, so `every_asset_is_a_different_file` in `src/assets.rs` cannot
be the test that fails. (`footstep.ogg` is not a byte-duplicate of any shipped
asset, including the `.wav` ones.)

## Scope: what was and was not touched

- **Changed:** the thirteen files listed above, all inside `assets/sounds/`;
  `assets/MANIFEST.md` §2 and the new §2a; this chapter.
- **Not changed:** every other file in `assets/sounds/` (including
  `variants/`), `assets/models/`, `assets/textures/`, and **all of `src/` —
  `src/assets.rs` in particular was NOT edited**, because every replacement
  kept its `.ogg` name and extension. `src/assets.rs` still reads
  `include_bytes!("../assets/sounds/shot_rifle.ogg")` and so on for all
  thirteen. (Other work was in flight against `src/` while this batch was
  built, so `git status` is not empty under `src/` — none of it is this
  batch's.)
- **Not run:** `cargo`. The build and the crate's own test suite were left to
  the process that owns `target/`.
- **Not vendored:** the source previews. The thirteen that were used total
  752,867 B (the raw staging directory also holds the rejected candidates); all
  of it lives in `assets/_downloads/sfx_new/raw/`, a git-ignored directory,
  along with the build script `build.py`, the analysis helpers and
  `sources.json`, so the pipeline can be re-run or the cut points changed. The
  direct links above re-fetch every source.

## Open questions for whoever owns this

1. **`shot_smg` is a pitched-up 12-gauge**, not a real 9 mm, because no CC0
   real 9 mm single shot was found. If that bothers you, the slot should be
   re-cut; the alternative on disk is the full-auto MP5A3 street recording
   (201671), which is genuinely 9 mm but has no isolated shot in it.
2. **`hit_head` is cracked ice**, presented by its author as a bone/skull
   stand-in. It measures like bone, but it is not bone.
3. **The sources are freesound previews**, not the uploader originals — see
   the caveat at the top of this chapter. Anyone with a freesound account can
   replace the thirteen with the originals and re-run `build.py` unchanged
   apart from the file names.
4. **Nothing has been listened to.** Every choice above was made from
   measurements — envelopes, spectral centroids, band energy, SNR between
   impacts. The shapes and levels are right by construction; whether each one
   *reads* as the right weapon is a judgement only a playtest can make.

## Two things noticed outside this batch, not touched

- `sanity_critical.wav` is **1.125 s**, not the 2.25 s that Part 1 above
  records. Both libsndfile and PyAV agree on 1.125 s from the file itself
  (397,080 B of 44.1 kHz stereo 32-bit float PCM is 49,629 frames); the file
  was not modified by this batch, so the number in Part 1 is simply wrong.
- `verify.py`, the checker quoted above, is written for these thirteen files:
  its "RATE" check fails anything that is not 44.1 kHz and its "NOT-OGG-VORBIS"
  check fails WAVs, so run over the whole folder it reports nine *pre-existing*
  files as problems (the four §2b WAVs, the three 48 kHz §2b Oggs, and
  `boss_arrival` / `elite_arrival` / `mission_complete` for length). All
  thirteen files in this batch come back `ok`.

The build and analysis scripts are in `assets/_downloads/sfx_new/`:
`build.py` (per-slot cut plan, filtering, normalisation, encode),
`probe.py` / `timeline.py` (decode, envelope, band stats),
`scan.py` / `dl.py` (freesound search, licence read-out, preview download),
`verify.py` (the container/decoder/level check quoted above), `sources.json`
(every downloaded preview with its URL and byte count) and `orig_backup/` (the
thirteen files as they were, so the swap can be undone without git).

---

# Part 3: `shot_smg.ogg`, replaced a second time — the one non-CC0 file

Everything else in `assets/` is CC0 or OFL. This file is **CC BY 3.0**, and it is
the only exception, so it is written up in full.

## Why the first replacement was not good enough

Part 2 above gave `shot_smg.ogg` freesound 244138 (`9mm_gunshot.wav`, mnslugger20,
CC0). It was the only one of the thirteen that was not a recording of the weapon
it plays: the file's own author says it is a twelve-gauge sped up by two, because
no CC0 single 9 mm shot exists anywhere the search reached. The real 9 mm material
on freesound is continuous full-auto bursts with street echo and no isolated shot
in them. Sped-up buckshot is a pistol *shape* and not a pistol *sound*, and the
submachine gun is the weapon that fires fastest in the game, so it is the one
where the ear has the most chances to notice.

## Source

| | |
| --- | --- |
| Page | https://opengameart.org/content/gunshot-sounds |
| Bundle | https://opengameart.org/sites/default/files/sounds.zip (5,504,243 B) |
| File used | `sounds/cz.wav` — a CZ-52 pistol, 6.84 s, stereo, five shots |
| Author | Vincent Sevedge, 2009 |
| Licence, as the page shows it | `License(s): CC0` |
| Licence, as the bundle states it | `Copyright (c) 2009 Vincent Sevedge` … `Creative Commons Attribution 3.0 Unported License`, in `sounds/creativecommons.txt` |

**The page and the bundle disagree, and the bundle wins.** Its
`creativecommons.txt` is the uploader's own file, it names an author and a year,
and it names a licence that requires something — a CC0 grant needs no such file.
A page field can be set with one click and changed afterwards; the text inside
the archive cannot. So this file is treated as **CC BY 3.0** and credited in the
README's asset table. If the author's intent was CC0 after all, the credit costs
nothing.

## Measurements

| | |
| --- | --- |
| Output | `assets/sounds/shot_smg.ogg`, 5,151 B |
| Length / rate / channels | 0.220 s · 44,100 Hz · mono |
| Peak | 0.8910 (−1.00 dBFS), no sample at full scale |
| Cut point | 2.822 s of the source — the isolated shot, with the crack's onset found as the first sample past half the local peak |
| Shaping | `exp(-max(0, t − 0.015) / 0.045)` |
| Envelope, 10 ms blocks | `0.89 0.81 0.75 0.54 0.37 0.35 0.24 0.19 0.17 0.14 0.08 0.08 0.06 0.04 0.03 0.03 0.02 0.01 0.01 0.01 0.00` |
| First 32 bytes | `4f 67 67 53 00 02 00 00 00 00 00 00 00 00 ec da 16 3f 00 00 00 00 1c 60 6a 7d 01 1e 01 76 6f 72` |

The source recording carries about a second of room on each shot. This weapon
fires **18 rounds a second**, so a one-second tail is not a tail — it is a bed.
Six copies would be summed at any instant, and the mixer has no ducking and no
per-slot gain (`Audio::emit` in `src/audio.rs` sums what it is given), so the
pile would clip almost immediately. Unshaped, four overlapping copies of the raw
cut peak at **2.5**; with the tail pulled down after the first 15 ms they peak at
**0.92** — the same headroom the take it replaced had at 0.88, measured the same
way. The first fifteen milliseconds are the shot itself and are not touched, so
what is left is the CZ-52's own crack with a shortened room behind it.

## Verification

- Decoded with PyAV and with libsndfile, to a per-sample disagreement below
  5e-7; both agree on 0.220 s, 44.1 kHz, mono.
- Passes `src/audio.rs`'s own `ogg_ok` — the test
  `every_embedded_sound_is_in_a_container_the_decoder_accepts` is what checks
  this, and it is run over the real embedded bytes.
- The 28 files in `assets/sounds/` still hash distinct
  (`every_asset_is_a_different_file`).
- **Not listened to.** Like everything else in Parts 2 and 3, the choice is
  measurement-driven: a real pistol crack, cut at its crack and shaped for the
  rate it is fired at. Whether it *reads* as a submachine gun is a playtest
  question.

## The other three files in the pack, not used

`sounds/mosin.wav` (14.7 s, 53 events), `sounds/sks.wav` (15.5 s, 27 events) and
`sounds/shotty.wav` (0.70 s, 2 events) are recordings of weapons the shipping set
already has CC0 takes of — `shot_sniper.ogg`, `shot_rifle.ogg` and
`shot_shotgun.ogg`. Nothing was wrong with those three, so nothing was fixed by
trading a CC0 file for a CC-BY one. They are left in
`assets/_downloads/sfx_new/oga/` with the licence text, so a preference for one
of them is a cut plan and a rebuild rather than a search.
