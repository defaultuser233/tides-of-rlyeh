//! Sound effects.
//!
//! Every effect is synthesised in memory as 16-bit PCM WAV data — the RIFF
//! header is written by hand — so the game needs no audio assets on disk.
//! A real file dropped at `assets/sounds/<name>.ogg` or
//! `assets/sounds/<name>.wav` takes priority over the synthesised version;
//! see [`Sfx::file_name`] for the names.
//!
//! Without the `sound` feature this module still compiles and [`Audio`] keeps
//! the same API, it just does nothing.

#[cfg(feature = "sound")]
use crate::assets::Asset;
#[cfg(feature = "sound")]
use macroquad::audio::{PlaySoundParams, Sound, load_sound_from_bytes, play_sound};
#[cfg(feature = "sound")]
use std::cell::RefCell;
#[cfg(feature = "sound")]
use std::time::Instant;

/// Sample rate of every synthesised effect.
#[cfg(feature = "sound")]
const SAMPLE_RATE: u32 = 44_100;

/// `play_at` drops anything this far away.
#[cfg(feature = "sound")]
const MAX_DISTANCE: f32 = 45.0;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Sfx {
    ShotRifle,
    ShotShotgun,
    ShotSmg,
    ShotSniper,
    ShotLmg,
    EnemyShot,
    HitFlesh,
    HitWorld,
    HitHead,
    EnemyDeath,
    Reload,
    ReloadEmpty,
    /// The magazine seated and the weapon live again. `Reload` is the start of
    /// the reload; this is the moment it stops being true, which is the half a
    /// player reloading behind cover actually needs to hear.
    ReloadDone,
    Footstep,
    PlayerHurt,
    /// A medical item going on: the bandage, the injector, the bottle. Its own
    /// slot rather than the reload's click, which is what it used to play —
    /// two things a player has to be able to tell apart with their eyes on
    /// something else.
    Healing,
    /// The mind crossed into the critical band. A warning, not a scare: it has
    /// to carry under gunfire, so it is quieter than the shot it plays beside.
    SanityCritical,
    PickupHealth,
    PickupAmmo,
    UiClick,
    UiMove,
    UiBack,
    Explosion,
    Spawn,
    /// Something enormous has been let into the arena (§19.4-10). Played at
    /// full volume rather than by distance: the point is that it is heard from
    /// anywhere, before it is seen.
    EliteArrival,
    /// The same event on a mission's last wave, where the elite is the climax
    /// rather than a spike: lower, longer, and allowed to be unpleasant.
    BossArrival,
    ObjectiveDone,
    MissionComplete,
    MissionFailed,
}

impl Sfx {
    /// Every effect, in enum order. Only the sound backend enumerates them.
    #[cfg(feature = "sound")]
    const ALL: [Sfx; 29] = [
        Sfx::ShotRifle,
        Sfx::ShotShotgun,
        Sfx::ShotSmg,
        Sfx::ShotSniper,
        Sfx::ShotLmg,
        Sfx::EnemyShot,
        Sfx::HitFlesh,
        Sfx::HitWorld,
        Sfx::HitHead,
        Sfx::EnemyDeath,
        Sfx::Reload,
        Sfx::ReloadEmpty,
        Sfx::ReloadDone,
        Sfx::Footstep,
        Sfx::PlayerHurt,
        Sfx::Healing,
        Sfx::SanityCritical,
        Sfx::PickupHealth,
        Sfx::PickupAmmo,
        Sfx::UiClick,
        Sfx::UiMove,
        Sfx::UiBack,
        Sfx::Explosion,
        Sfx::Spawn,
        Sfx::EliteArrival,
        Sfx::BossArrival,
        Sfx::ObjectiveDone,
        Sfx::MissionComplete,
        Sfx::MissionFailed,
    ];

    /// Number of distinct effects.
    #[cfg(feature = "sound")]
    const COUNT: usize = Self::ALL.len();

    #[cfg(feature = "sound")]
    const fn index(self) -> usize {
        self as usize
    }

    /// Base name of the optional override file under `assets/sounds/`.
    #[cfg(feature = "sound")]
    const fn file_name(self) -> &'static str {
        match self {
            Sfx::ShotRifle => "shot_rifle",
            Sfx::ShotShotgun => "shot_shotgun",
            Sfx::ShotSmg => "shot_smg",
            Sfx::ShotSniper => "shot_sniper",
            Sfx::ShotLmg => "shot_lmg",
            Sfx::EnemyShot => "enemy_shot",
            Sfx::HitFlesh => "hit_flesh",
            Sfx::HitWorld => "hit_world",
            Sfx::HitHead => "hit_head",
            Sfx::EnemyDeath => "enemy_death",
            Sfx::Reload => "reload",
            Sfx::ReloadEmpty => "reload_empty",
            Sfx::ReloadDone => "reload_done",
            Sfx::Footstep => "footstep",
            Sfx::PlayerHurt => "player_hurt",
            Sfx::Healing => "healing",
            Sfx::SanityCritical => "sanity_critical",
            Sfx::PickupHealth => "pickup_health",
            Sfx::PickupAmmo => "pickup_ammo",
            Sfx::UiClick => "ui_click",
            Sfx::UiMove => "ui_move",
            Sfx::UiBack => "ui_back",
            Sfx::Explosion => "explosion",
            Sfx::Spawn => "spawn",
            Sfx::EliteArrival => "elite_arrival",
            Sfx::BossArrival => "boss_arrival",
            Sfx::ObjectiveDone => "objective_done",
            Sfx::MissionComplete => "mission_complete",
            Sfx::MissionFailed => "mission_failed",
        }
    }

    /// The compiled-in file for this effect, if it has one. The effects with no
    /// file are synthesised at load time and never name an asset at all.
    #[cfg(feature = "sound")]
    const fn asset(self) -> Option<Asset> {
        Some(match self {
            Sfx::ShotRifle => Asset::SfxShotRifle,
            Sfx::ShotShotgun => Asset::SfxShotShotgun,
            Sfx::ShotSmg => Asset::SfxShotSmg,
            Sfx::ShotSniper => Asset::SfxShotSniper,
            Sfx::ShotLmg => Asset::SfxShotLmg,
            Sfx::EnemyShot => Asset::SfxEnemyShot,
            Sfx::HitFlesh => Asset::SfxHitFlesh,
            Sfx::HitWorld => Asset::SfxHitWorld,
            Sfx::HitHead => Asset::SfxHitHead,
            Sfx::EnemyDeath => Asset::SfxEnemyDeath,
            Sfx::Reload => Asset::SfxReload,
            Sfx::ReloadEmpty => Asset::SfxReloadEmpty,
            Sfx::Footstep => Asset::SfxFootstep,
            Sfx::PlayerHurt => Asset::SfxPlayerHurt,
            Sfx::UiClick => Asset::SfxUiClick,
            Sfx::UiMove => Asset::SfxUiMove,
            Sfx::UiBack => Asset::SfxUiBack,
            Sfx::Explosion => Asset::SfxExplosion,
            Sfx::Spawn => Asset::SfxSpawn,
            Sfx::Healing => Asset::SfxHealing,
            Sfx::EliteArrival => Asset::SfxEliteArrival,
            Sfx::BossArrival => Asset::SfxBossArrival,
            Sfx::SanityCritical => Asset::SfxSanityCritical,
            Sfx::PickupHealth => Asset::SfxPickupHealth,
            Sfx::PickupAmmo => Asset::SfxPickupAmmo,
            Sfx::ObjectiveDone => Asset::SfxObjectiveDone,
            Sfx::MissionComplete => Asset::SfxMissionComplete,
            Sfx::MissionFailed => Asset::SfxMissionFailed,
            // `ReloadDone` has no curated file: it is one of the two effects
            // still synthesised on purpose, and the recipe below is what plays.
            _ => return None,
        })
    }

    /// Shortest gap allowed between two plays of the same effect, in seconds.
    /// Keeps a full-auto stream of callers from flooding the mixer.
    #[cfg(feature = "sound")]
    const fn interval(self) -> f32 {
        match self {
            Sfx::ShotRifle
            | Sfx::ShotShotgun
            | Sfx::ShotSmg
            | Sfx::ShotSniper
            | Sfx::ShotLmg
            | Sfx::EnemyShot => 0.040,
            Sfx::HitFlesh | Sfx::HitWorld | Sfx::HitHead => 0.025,
            Sfx::Footstep => 0.120,
            Sfx::ReloadDone => 0.120,
            // The three event cues are one-shot notices: two of them inside a
            // few seconds is always a mistake, never a firefight.
            Sfx::SanityCritical => 2.500,
            Sfx::EliteArrival | Sfx::BossArrival => 2.000,
            Sfx::UiClick | Sfx::UiMove | Sfx::UiBack => 0.030,
            Sfx::EnemyDeath
            | Sfx::Reload
            | Sfx::ReloadEmpty
            | Sfx::PlayerHurt
            | Sfx::Healing
            | Sfx::PickupHealth
            | Sfx::PickupAmmo
            | Sfx::Explosion
            | Sfx::Spawn
            | Sfx::ObjectiveDone
            | Sfx::MissionComplete
            | Sfx::MissionFailed => 0.060,
        }
    }
}

#[cfg(feature = "sound")]
pub struct Audio {
    master: f32,
    /// One slot per effect; `None` when neither the override file nor the
    /// synthesised buffer could be loaded, which makes that effect silent.
    sounds: [Option<Sound>; Sfx::COUNT],
    last: RefCell<[Option<Instant>; Sfx::COUNT]>,
}

#[cfg(feature = "sound")]
impl Audio {
    /// Never panics. Anything that fails to load — a missing override file, an
    /// unreadable container, a decoder that refuses the bytes — turns into a
    /// silent slot, and an audio device that never started simply means every
    /// slot is silent.
    pub async fn load() -> Audio {
        let mut sounds: [Option<Sound>; Sfx::COUNT] = std::array::from_fn(|_| None);
        for sfx in Sfx::ALL {
            let Some(slot) = sounds.get_mut(sfx.index()) else {
                continue;
            };
            *slot = load_effect(sfx).await;
        }
        Audio {
            master: 1.0,
            sounds,
            last: RefCell::new(std::array::from_fn(|_| None)),
        }
    }

    pub fn play(&self, sfx: Sfx) {
        self.emit(sfx, 1.0);
    }

    pub fn play_at(&self, sfx: Sfx, distance: f32) {
        if !distance.is_finite() || distance >= MAX_DISTANCE {
            return;
        }
        let falloff = 1.0 - distance.max(0.0) / MAX_DISTANCE;
        self.emit(sfx, falloff * falloff);
    }

    pub fn set_master(&mut self, v: f32) {
        self.master = v.clamp(0.0, 1.0);
    }

    fn emit(&self, sfx: Sfx, gain: f32) {
        let volume = self.master * gain;
        if volume <= 0.0 {
            return;
        }

        let slot = sfx.index();
        {
            let mut last = self.last.borrow_mut();
            let Some(entry) = last.get_mut(slot) else {
                return;
            };
            let now = Instant::now();
            if let Some(prev) = *entry
                && now.duration_since(prev).as_secs_f32() < sfx.interval()
            {
                return;
            }
            *entry = Some(now);
        }

        if let Some(Some(sound)) = self.sounds.get(slot) {
            play_sound(
                sound,
                PlaySoundParams {
                    looped: false,
                    volume,
                },
            );
        }
    }
}

#[cfg(not(feature = "sound"))]
pub struct Audio {
    master: f32,
}

#[cfg(not(feature = "sound"))]
impl Audio {
    /// Same API as the `sound` build; there is nothing to load.
    pub async fn load() -> Audio {
        Audio { master: 1.0 }
    }

    pub fn play(&self, _sfx: Sfx) {}

    pub fn play_at(&self, _sfx: Sfx, _distance: f32) {}

    pub fn set_master(&mut self, v: f32) {
        self.master = v.clamp(0.0, 1.0);
    }
}

/// WAV bytes for one effect. Pure maths: no audio device, no file system, so
/// it can be exercised (and listened to) on its own.
#[cfg(feature = "sound")]
pub(crate) fn wav_bytes(sfx: Sfx) -> Vec<u8> {
    synth::wav(sfx)
}

/// The curated override files, compiled into the binary.
///
/// A release build is normally launched from `target/release` (or copied
/// elsewhere), where `assets/sounds/...` does not resolve, so every effect
/// silently fell back to its synthesised version. Embedding makes the shipped
/// executable self-contained; the on-disk probe above still wins when present,
/// so the files remain replaceable.
#[cfg(feature = "sound")]
fn embedded(sfx: Sfx) -> Option<&'static [u8]> {
    Some(crate::assets::bytes(sfx.asset()?))
}

/// Load an override file if there is one, otherwise the synthesised buffer.
/// Silently returns `None` when neither works.
#[cfg(feature = "sound")]
async fn load_effect(sfx: Sfx) -> Option<Sound> {
    let base = sfx.file_name();
    for ext in ["ogg", "wav"] {
        let path = format!("assets/sounds/{base}.{ext}");
        if let Some(sound) = load_override(&path, ext).await {
            return Some(sound);
        }
    }
    // The container is asked of the bytes rather than assumed: an embedded
    // effect may be an Ogg or a WAV, and the shipped table has both — the first
    // version of this said "ogg" for every asset, which made every embedded WAV
    // fail its own check and fall back to the synthesiser without a word.
    if let Some(bytes) = embedded(sfx)
        && (wav_ok(bytes) || ogg_ok(bytes))
        && let Ok(sound) = load_sound_from_bytes(bytes).await
    {
        return Some(sound);
    }
    load_sound_from_bytes(&wav_bytes(sfx)).await.ok()
}

/// Read one candidate file. Missing files are normal; a file whose container
/// looks wrong is skipped as well, because macroquad's mixer `unwrap`s inside
/// the decoder and would take the whole game down instead of returning `Err`.
#[cfg(feature = "sound")]
async fn load_override(path: &str, ext: &str) -> Option<Sound> {
    let bytes = macroquad::file::load_file(path).await.ok()?;
    if !container_ok(&bytes, ext) {
        return None;
    }
    load_sound_from_bytes(&bytes).await.ok()
}

#[cfg(feature = "sound")]
fn container_ok(bytes: &[u8], ext: &str) -> bool {
    match ext {
        "wav" => wav_ok(bytes),
        "ogg" => ogg_ok(bytes),
        _ => false,
    }
}

/// Walks the chunk list and accepts only plain PCM/float files with a sane
/// channel count, which is what `audrey` is able to stream back.
#[cfg(feature = "sound")]
fn wav_ok(bytes: &[u8]) -> bool {
    if bytes.len() < 44 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return false;
    }

    let (mut fmt_ok, mut data_ok) = (false, false);
    let mut at = 12;
    while at + 8 <= bytes.len() {
        let id = &bytes[at..at + 4];
        let size = le_u32(&bytes[at + 4..at + 8]) as usize;
        let body = at + 8;
        if body + size > bytes.len() {
            return false;
        }

        if id == b"fmt " && size >= 16 {
            let format = le_u16(&bytes[body..body + 2]);
            let channels = le_u16(&bytes[body + 2..body + 4]);
            let bits = le_u16(&bytes[body + 14..body + 16]);
            fmt_ok = matches!(format, 1 | 3)
                && (1..=2).contains(&channels)
                && matches!(bits, 8 | 16 | 24 | 32);
        } else if id == b"data" {
            data_ok = size > 0;
        }

        // Chunks sit on even offsets.
        at = body + size + (size & 1);
    }

    fmt_ok && data_ok
}

/// Accepts only complete Ogg pages holding a Vorbis identification header:
/// Opus or an interrupted download would otherwise panic the Vorbis decoder.
#[cfg(feature = "sound")]
fn ogg_ok(bytes: &[u8]) -> bool {
    let mut at = 0;
    let mut header = Vec::with_capacity(7);
    while at + 27 <= bytes.len() {
        if &bytes[at..at + 4] != b"OggS" || bytes[at + 4] != 0 {
            return false;
        }
        let segments = bytes[at + 26] as usize;
        let table = at + 27;
        if table + segments > bytes.len() {
            return false;
        }
        let body: usize = bytes[table..table + segments]
            .iter()
            .map(|n| *n as usize)
            .sum();
        let start = table + segments;
        if start + body > bytes.len() {
            return false;
        }

        if header.len() < 7 {
            let take = body.min(7 - header.len());
            header.extend_from_slice(&bytes[start..start + take]);
            if header.len() == 7 && header != b"\x01vorbis" {
                return false;
            }
        }

        at = start + body;
    }
    at == bytes.len() && header.len() == 7
}

#[cfg(feature = "sound")]
fn le_u16(bytes: &[u8]) -> u16 {
    u16::from_le_bytes([bytes[0], bytes[1]])
}

#[cfg(feature = "sound")]
fn le_u32(bytes: &[u8]) -> u32 {
    u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
}

/// In-memory synthesis of the effect table. Nothing here touches the audio
/// backend, so it stays testable on a machine without a sound card.
#[cfg(feature = "sound")]
mod synth {
    use super::{SAMPLE_RATE, Sfx};
    use std::f32::consts::TAU;

    /// Render one effect: build the layers, normalise to its target peak, wrap
    /// in a RIFF/WAVE container.
    pub fn wav(sfx: Sfx) -> Vec<u8> {
        let (mut track, peak) = recipe(sfx);
        track.normalise(peak);
        track.encode()
    }

    /// Deterministic white noise. A local generator keeps the module
    /// self-contained and every build byte-identical.
    struct Noise(u32);

    impl Noise {
        fn new(seed: u32) -> Self {
            Self(seed | 1)
        }

        fn sample(&mut self) -> f32 {
            let mut x = self.0;
            x ^= x << 13;
            x ^= x >> 17;
            x ^= x << 5;
            self.0 = x;
            (x as f32 / u32::MAX as f32) * 2.0 - 1.0
        }
    }

    /// One-pole low-pass coefficient for a corner at `cutoff` Hz.
    fn alpha(cutoff: f32) -> f32 {
        (1.0 - (-TAU * cutoff / SAMPLE_RATE as f32).exp()).clamp(0.0, 1.0)
    }

    /// Percussive envelope: brief attack, exponential decay, short release so a
    /// burst that is still loud at its end cannot click.
    fn punch(t: f32, dur: f32, decay: f32) -> f32 {
        const ATTACK: f32 = 0.0015;
        const RELEASE: f32 = 0.006;
        (t / ATTACK).min(1.0) * (-t / decay.max(1e-4)).exp() * ((dur - t) / RELEASE).clamp(0.0, 1.0)
    }

    /// Slow triangle swell, for risers and pads.
    fn swell(t: f32, dur: f32) -> f32 {
        let k = (t / dur.max(1e-4)).clamp(0.0, 1.0);
        (k / 0.35).min(1.0) * ((1.0 - k) / 0.35).min(1.0)
    }

    struct Track {
        buf: Vec<f32>,
        rng: Noise,
    }

    impl Track {
        fn new(seconds: f32) -> Self {
            let samples = (seconds * SAMPLE_RATE as f32).round().max(1.0) as usize;
            Self {
                buf: vec![0.0; samples],
                rng: Noise::new(0x9E37_79B9),
            }
        }

        /// Mix in a low-passed noise burst, the low-pass corner sweeping from
        /// `cut0` to `cut1` Hz over the burst.
        fn noise(&mut self, at: f32, dur: f32, amp: f32, cut0: f32, cut1: f32, decay: f32) {
            let start = (at * SAMPLE_RATE as f32).round() as usize;
            let len = (dur * SAMPLE_RATE as f32).round() as usize;
            let mut lp = 0.0;

            for i in 0..len {
                let Some(cell) = self.buf.get_mut(start + i) else {
                    break;
                };
                let t = i as f32 / SAMPLE_RATE as f32;
                let k = i as f32 / len.max(1) as f32;
                let a = alpha(cut0 + (cut1 - cut0) * k);
                lp += a * (self.rng.sample() - lp);
                *cell += lp * amp * punch(t, dur, decay);
            }
        }

        /// Mix in a sine sweeping from `f0` to `f1` Hz.
        fn tone(&mut self, at: f32, dur: f32, amp: f32, f0: f32, f1: f32, decay: f32) {
            let start = (at * SAMPLE_RATE as f32).round() as usize;
            let len = (dur * SAMPLE_RATE as f32).round() as usize;
            let mut phase = 0.0;

            for i in 0..len {
                let Some(cell) = self.buf.get_mut(start + i) else {
                    break;
                };
                let t = i as f32 / SAMPLE_RATE as f32;
                let f = f0.max(1.0) * (f1.max(1.0) / f0.max(1.0)).powf((t / dur).clamp(0.0, 1.0));
                phase = (phase + TAU * f / SAMPLE_RATE as f32) % TAU;
                *cell += phase.sin() * amp * punch(t, dur, decay);
            }
        }

        /// Mix in a slow-attack sine sweep.
        fn swell(&mut self, at: f32, dur: f32, amp: f32, f0: f32, f1: f32) {
            let start = (at * SAMPLE_RATE as f32).round() as usize;
            let len = (dur * SAMPLE_RATE as f32).round() as usize;
            let mut phase = 0.0;

            for i in 0..len {
                let Some(cell) = self.buf.get_mut(start + i) else {
                    break;
                };
                let t = i as f32 / SAMPLE_RATE as f32;
                let f = f0.max(1.0) * (f1.max(1.0) / f0.max(1.0)).powf((t / dur).clamp(0.0, 1.0));
                phase = (phase + TAU * f / SAMPLE_RATE as f32) % TAU;
                *cell += phase.sin() * amp * swell(t, dur);
            }
        }

        /// Scale so the loudest sample sits exactly at `peak` (0.0..=1.0), which
        /// also means the 16-bit conversion can never overflow.
        fn normalise(&mut self, peak: f32) {
            let loudest = self.buf.iter().fold(0.0f32, |m, s| m.max(s.abs()));
            if loudest > 1e-6 {
                let gain = peak.clamp(0.0, 1.0) / loudest;
                for s in &mut self.buf {
                    *s *= gain;
                }
            }
        }

        fn encode(&self) -> Vec<u8> {
            let data = self.buf.len() * 2;
            let mut out = Vec::with_capacity(44 + data);

            out.extend_from_slice(b"RIFF");
            out.extend_from_slice(&(36 + data as u32).to_le_bytes());
            out.extend_from_slice(b"WAVE");
            out.extend_from_slice(b"fmt ");
            out.extend_from_slice(&16u32.to_le_bytes());
            out.extend_from_slice(&1u16.to_le_bytes()); // PCM
            out.extend_from_slice(&1u16.to_le_bytes()); // mono
            out.extend_from_slice(&SAMPLE_RATE.to_le_bytes());
            out.extend_from_slice(&(SAMPLE_RATE * 2).to_le_bytes()); // byte rate
            out.extend_from_slice(&2u16.to_le_bytes()); // block align
            out.extend_from_slice(&16u16.to_le_bytes());
            out.extend_from_slice(b"data");
            out.extend_from_slice(&(data as u32).to_le_bytes());

            for s in &self.buf {
                let v = (s.clamp(-1.0, 1.0) * i16::MAX as f32).round() as i16;
                out.extend_from_slice(&v.to_le_bytes());
            }
            out
        }
    }

    /// Layers plus the peak the effect is normalised to.
    fn recipe(sfx: Sfx) -> (Track, f32) {
        use Sfx::*;
        match sfx {
            // --- guns -------------------------------------------------------
            ShotRifle => {
                let mut t = Track::new(0.26);
                t.noise(0.00, 0.10, 1.00, 4500.0, 2200.0, 0.030);
                t.tone(0.00, 0.16, 0.90, 120.0, 54.0, 0.045);
                t.noise(0.02, 0.18, 0.35, 1100.0, 600.0, 0.070);
                (t, 0.85)
            }
            ShotShotgun => {
                let mut t = Track::new(0.44);
                t.noise(0.00, 0.20, 1.00, 2600.0, 900.0, 0.060);
                t.tone(0.00, 0.28, 1.00, 95.0, 34.0, 0.110);
                t.noise(0.03, 0.34, 0.50, 700.0, 260.0, 0.160);
                (t, 0.98)
            }
            ShotSmg => {
                let mut t = Track::new(0.14);
                t.noise(0.00, 0.045, 1.00, 6500.0, 3500.0, 0.014);
                t.tone(0.00, 0.070, 0.80, 210.0, 95.0, 0.022);
                (t, 0.68)
            }
            ShotSniper => {
                let mut t = Track::new(0.72);
                t.noise(0.00, 0.05, 1.00, 9000.0, 5000.0, 0.010);
                t.noise(0.01, 0.45, 0.50, 2600.0, 700.0, 0.200);
                t.tone(0.00, 0.35, 0.90, 80.0, 30.0, 0.160);
                (t, 1.00)
            }
            ShotLmg => {
                let mut t = Track::new(0.30);
                t.noise(0.00, 0.12, 1.00, 3800.0, 1800.0, 0.038);
                t.tone(0.00, 0.18, 1.00, 100.0, 44.0, 0.055);
                t.noise(0.03, 0.22, 0.30, 900.0, 400.0, 0.080);
                (t, 0.90)
            }
            EnemyShot => {
                let mut t = Track::new(0.22);
                t.noise(0.00, 0.07, 1.00, 3200.0, 1200.0, 0.020);
                t.tone(0.00, 0.14, 0.90, 330.0, 110.0, 0.045);
                t.noise(0.05, 0.12, 0.25, 700.0, 300.0, 0.060);
                (t, 0.62)
            }

            // --- impacts ----------------------------------------------------
            HitFlesh => {
                let mut t = Track::new(0.18);
                t.noise(0.00, 0.075, 1.00, 1000.0, 420.0, 0.022);
                t.tone(0.00, 0.100, 0.70, 170.0, 70.0, 0.035);
                (t, 0.60)
            }
            HitWorld => {
                let mut t = Track::new(0.15);
                t.noise(0.00, 0.035, 1.00, 7000.0, 4000.0, 0.008);
                t.tone(0.00, 0.060, 0.60, 1500.0, 700.0, 0.014);
                (t, 0.50)
            }
            HitHead => {
                let mut t = Track::new(0.19);
                t.noise(0.00, 0.030, 1.00, 8000.0, 5000.0, 0.007);
                t.tone(0.00, 0.090, 0.80, 2400.0, 1500.0, 0.030);
                t.tone(0.00, 0.120, 0.50, 190.0, 90.0, 0.040);
                (t, 0.68)
            }
            EnemyDeath => {
                let mut t = Track::new(0.62);
                t.tone(0.00, 0.50, 1.00, 340.0, 62.0, 0.200);
                t.noise(0.00, 0.10, 0.60, 1600.0, 500.0, 0.050);
                t.tone(0.15, 0.36, 0.35, 170.0, 45.0, 0.160);
                (t, 0.65)
            }
            PlayerHurt => {
                let mut t = Track::new(0.40);
                t.noise(0.00, 0.20, 0.80, 800.0, 300.0, 0.075);
                t.tone(0.00, 0.28, 1.00, 240.0, 96.0, 0.110);
                t.tone(0.02, 0.20, 0.40, 480.0, 200.0, 0.060);
                (t, 0.70)
            }
            Healing => {
                // The synth fallback: cloth, then the small metal of a cap or a
                // plunger. The curated file is the real one.
                let mut t = Track::new(0.55);
                t.noise(0.00, 0.22, 1.00, 2600.0, 700.0, 0.090);
                t.tone(0.30, 0.06, 0.45, 900.0, 620.0, 0.020);
                t.noise(0.32, 0.05, 0.55, 4200.0, 2000.0, 0.016);
                (t, 0.52)
            }
            SanityCritical => {
                let mut t = Track::new(1.50);
                // A double heartbeat under a breath that opens upward into a
                // thin high tone: the body first, then the thing the body is
                // reacting to.
                t.tone(0.00, 0.32, 1.00, 58.0, 46.0, 0.130);
                t.tone(0.30, 0.28, 0.75, 54.0, 44.0, 0.110);
                t.noise(0.00, 1.10, 0.40, 260.0, 1500.0, 0.420);
                t.tone(0.35, 0.90, 0.32, 880.0, 1318.5, 0.500);
                (t, 0.60)
            }

            // --- gear -------------------------------------------------------
            Reload => {
                let mut t = Track::new(0.50);
                t.noise(0.00, 0.05, 0.90, 5200.0, 2600.0, 0.012);
                t.tone(0.00, 0.05, 0.45, 900.0, 500.0, 0.015);
                t.noise(0.30, 0.06, 1.00, 4200.0, 1800.0, 0.020);
                t.tone(0.30, 0.08, 0.60, 620.0, 320.0, 0.030);
                t.noise(0.40, 0.05, 0.75, 6000.0, 3000.0, 0.014);
                (t, 0.55)
            }
            ReloadEmpty => {
                let mut t = Track::new(0.26);
                t.noise(0.00, 0.030, 1.00, 6500.0, 4000.0, 0.009);
                t.tone(0.00, 0.050, 0.50, 1800.0, 1200.0, 0.016);
                (t, 0.48)
            }
            ReloadDone => {
                let mut t = Track::new(0.24);
                // The magazine catch, then a short rising pair: metal, then
                // the weapon saying it is ready again.
                t.noise(0.00, 0.030, 1.00, 5200.0, 2800.0, 0.008);
                t.tone(0.00, 0.070, 0.60, 1500.0, 950.0, 0.018);
                t.tone(0.060, 0.110, 0.50, 700.0, 1120.0, 0.035);
                (t, 0.55)
            }
            Footstep => {
                let mut t = Track::new(0.16);
                t.noise(0.00, 0.085, 1.00, 600.0, 240.0, 0.028);
                t.tone(0.00, 0.060, 0.50, 130.0, 60.0, 0.020);
                (t, 0.32)
            }

            // --- pickups ----------------------------------------------------
            PickupHealth => {
                let mut t = Track::new(0.34);
                t.tone(0.00, 0.20, 1.00, 520.0, 780.0, 0.080);
                t.tone(0.06, 0.24, 0.60, 780.0, 1170.0, 0.100);
                (t, 0.50)
            }
            PickupAmmo => {
                let mut t = Track::new(0.24);
                t.tone(0.00, 0.10, 1.00, 700.0, 1050.0, 0.035);
                t.noise(0.00, 0.03, 0.35, 7000.0, 4000.0, 0.008);
                (t, 0.50)
            }

            // --- interface --------------------------------------------------
            UiClick => {
                let mut t = Track::new(0.06);
                t.noise(0.00, 0.012, 1.00, 5000.0, 3000.0, 0.005);
                t.tone(0.00, 0.030, 0.70, 1600.0, 1100.0, 0.010);
                (t, 0.40)
            }
            UiMove => {
                let mut t = Track::new(0.05);
                t.noise(0.00, 0.010, 1.00, 3000.0, 1800.0, 0.005);
                t.tone(0.00, 0.025, 0.60, 800.0, 700.0, 0.008);
                (t, 0.28)
            }
            UiBack => {
                let mut t = Track::new(0.14);
                t.tone(0.00, 0.06, 1.00, 820.0, 560.0, 0.022);
                t.tone(0.05, 0.08, 0.70, 560.0, 380.0, 0.030);
                (t, 0.40)
            }

            // --- set pieces -------------------------------------------------
            Explosion => {
                let mut t = Track::new(1.70);
                t.noise(0.00, 0.09, 1.00, 6000.0, 2000.0, 0.018);
                t.noise(0.00, 1.20, 1.00, 900.0, 260.0, 0.420);
                t.tone(0.00, 0.90, 1.00, 62.0, 24.0, 0.300);
                t.noise(0.25, 1.30, 0.50, 300.0, 120.0, 0.550);
                (t, 1.00)
            }
            Spawn => {
                let mut t = Track::new(0.80);
                t.swell(0.00, 0.70, 1.00, 150.0, 520.0);
                t.noise(0.00, 0.70, 0.55, 200.0, 1800.0, 0.300);
                (t, 0.55)
            }
            EliteArrival => {
                let mut t = Track::new(1.50);
                // A rise that never resolves into a chord, then one step: the
                // dread of §19.4-10 rather than a fanfare, because whatever is
                // arriving is on the other side.
                t.swell(0.00, 1.00, 0.90, 48.0, 96.0);
                t.tone(0.55, 0.85, 1.00, 82.0, 41.0, 0.400);
                t.tone(0.58, 0.80, 0.55, 58.0, 55.0, 0.360);
                t.noise(0.55, 0.35, 0.45, 600.0, 180.0, 0.120);
                (t, 0.92)
            }
            BossArrival => {
                let mut t = Track::new(2.30);
                // The elite cue stretched: a longer descent, a detuned twin
                // voice to make it beat against itself, and a tritone over the
                // fundamental — the interval the dread is built out of.
                t.swell(0.00, 1.60, 0.95, 36.0, 78.0);
                t.tone(0.90, 1.30, 1.00, 65.0, 32.0, 0.550);
                t.tone(0.92, 1.20, 0.60, 46.0, 44.0, 0.500);
                t.tone(1.25, 0.95, 0.45, 92.0, 90.0, 0.420);
                t.noise(0.90, 0.50, 0.55, 500.0, 150.0, 0.160);
                t.noise(1.30, 0.80, 0.30, 250.0, 90.0, 0.300);
                (t, 1.00)
            }
            ObjectiveDone => {
                let mut t = Track::new(0.90);
                t.tone(0.00, 0.35, 1.00, 587.0, 587.0, 0.140);
                t.tone(0.18, 0.50, 0.90, 880.0, 880.0, 0.220);
                (t, 0.55)
            }
            MissionComplete => {
                let mut t = Track::new(1.80);
                t.tone(0.00, 0.30, 0.85, 523.3, 523.3, 0.130);
                t.tone(0.16, 0.30, 0.90, 659.3, 659.3, 0.140);
                t.tone(0.32, 0.34, 0.95, 784.0, 784.0, 0.160);
                t.tone(0.50, 0.70, 1.00, 1046.5, 1046.5, 0.320);
                t.swell(0.00, 1.20, 0.45, 220.0, 330.0);
                (t, 0.70)
            }
            MissionFailed => {
                let mut t = Track::new(1.60);
                t.tone(0.00, 0.30, 0.90, 440.0, 440.0, 0.130);
                t.tone(0.22, 0.30, 0.90, 392.0, 392.0, 0.140);
                t.tone(0.44, 0.32, 0.95, 349.2, 349.2, 0.150);
                t.tone(0.68, 0.62, 1.00, 261.6, 261.6, 0.280);
                t.swell(0.00, 1.10, 0.50, 110.0, 82.0);
                (t, 0.65)
            }
        }
    }
}

/// The table is data, and data can be wrong in ways the compiler cannot see:
/// an effect added to the enum but not to `ALL` never loads and is simply
/// silent, and an effect never given a recipe does not compile but one given a
/// recipe that is all zeroes does. These run on a machine with no sound card,
/// because nothing here opens a device.
#[cfg(all(test, feature = "sound"))]
mod tests {
    use super::*;

    /// Every variant is in `ALL` exactly once, at the index it is played at.
    #[test]
    fn every_effect_is_in_the_table_once() {
        let mut seen = [false; Sfx::COUNT];
        for sfx in Sfx::ALL {
            let i = sfx.index();
            assert!(i < Sfx::COUNT, "{sfx:?} indexes past the table");
            assert!(!seen[i], "{sfx:?} is listed twice");
            seen[i] = true;
        }
        assert!(
            seen.iter().all(|&s| s),
            "a variant is missing from `Sfx::ALL`, so it would never load"
        );
    }

    /// Two effects with one name would share an override file, and whichever
    /// loaded second would win.
    #[test]
    fn file_names_are_distinct() {
        for (i, a) in Sfx::ALL.iter().enumerate() {
            for b in &Sfx::ALL[i + 1..] {
                assert_ne!(
                    a.file_name(),
                    b.file_name(),
                    "{a:?} and {b:?} share an override name"
                );
            }
        }
    }

    /// Every effect synthesises to a WAV the loader will accept, of the length
    /// its own header claims, and it is not silence.
    #[test]
    fn every_effect_synthesises_to_a_valid_wav() {
        for sfx in Sfx::ALL {
            let bytes = wav_bytes(sfx);
            assert!(wav_ok(&bytes), "{sfx:?} did not produce a RIFF/WAVE file");

            let declared = le_u32(&bytes[40..44]) as usize;
            assert_eq!(
                declared + 44,
                bytes.len(),
                "{sfx:?} declares a data length it does not have"
            );

            let loudest = bytes[44..]
                .chunks_exact(2)
                .map(|c| i16::from_le_bytes([c[0], c[1]]).unsigned_abs())
                .max()
                .unwrap_or(0);
            assert!(loudest > 0, "{sfx:?} synthesised to silence");
        }
    }

    /// Every sound the binary carries is in a container the decoder will take.
    ///
    /// The loader is deliberately forgiving — a file it cannot read falls back
    /// to the synthesiser without saying so — which is what makes a bad file
    /// here invisible: the game plays *something* either way, and the only way
    /// to hear the difference is to already know the sound. So the shipped
    /// bytes are checked here instead, which is also the only place the curated
    /// files are looked at by the code that has to read them.
    #[cfg(feature = "sound")]
    #[test]
    fn every_embedded_sound_is_in_a_container_the_decoder_accepts() {
        for sfx in Sfx::ALL {
            let Some(asset) = sfx.asset() else { continue };
            let bytes = crate::assets::bytes(asset);
            assert!(
                ogg_ok(bytes) || wav_ok(bytes),
                "{} is not an Ogg or a WAV the decoder will accept",
                sfx.file_name()
            );
        }
    }
}
