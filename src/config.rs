//! The settings file: where it lives, what it looks like, and what happens when
//! it is missing or damaged.
//!
//! # Where
//!
//! One file named `config`, in a `tides-of-rlyeh` directory under the user's
//! settings directory:
//!
//! - Windows: `%APPDATA%\tides-of-rlyeh\config`
//! - Linux: `$XDG_CONFIG_HOME/tides-of-rlyeh/config`, falling back to
//!   `$HOME/.config/tides-of-rlyeh/config`
//!
//! Nothing is ever written next to the executable: this game ships as a single
//! file that is meant to run from anywhere, so the directory it happens to sit
//! in is not a directory it may assume it can write to. When the environment
//! names no settings directory at all, [`path`] returns `None` and the game runs
//! — and saves nothing — on its defaults.
//!
//! # The format
//!
//! `key = value`, one per line. `#` starts a comment that runs to the end of the
//! line, blank lines are ignored, whitespace around a key or a value is dropped,
//! keys are case-insensitive, and a key this build does not know is ignored (a
//! file from a newer build stays readable). Every key is optional:
//!
//! ```text
//! # TIDES OF R'LYEH settings. Delete this file to go back to the defaults.
//! fullscreen = false
//! width = 1280
//! height = 720
//! sensitivity = 1.6
//! mouse_dpi = 800
//! volume = 0.8
//! aim_toggle = true
//! lean_toggle = true
//! invert_y = false
//! show_fps = false
//! lang = zh
//! unlocked = 1
//! last_mission = 0
//! ```
//!
//! Booleans are `true`/`false` (`1`/`0` and `on`/`off` are taken too), `lang` is
//! `en` or `zh`, and a width or height outside `640..=7680` — or a mouse DPI
//! outside `100..=100_000` — is not believed.
//!
//! # When it is wrong
//!
//! Repairs are field by field, never file-wide: a line that cannot be read
//! leaves its own field at its default, so one bad character cannot cost the
//! player every other setting. A file that is missing, empty, unreadable or full
//! of nonsense loads as [`Config::default`], and a save that fails prints one
//! line to stderr and changes nothing else. Nothing in this module panics: the
//! file is user-editable text that the game has to survive.

use std::io::Write;
use std::ops::RangeInclusive;
use std::path::{Path, PathBuf};

use crate::i18n::Lang;

/// The directory and file name, shared by every base directory the environment
/// can name, so the file is in the same place relative to whichever one it is.
const DIR: &str = "tides-of-rlyeh";
/// Where the file lived under the game's earlier names, newest first. The first
/// one that exists is read when the current path has never been written, so a
/// save survives a rename; the next save puts it where it belongs now.
const LEGACY_DIRS: [&str; 2] = ["eldritch-front", "steel-front"];
const FILE: &str = "config";

/// The window sizes this game believes. A saved size outside the range is a
/// hand-edit or a corrupt line, and the default is a better answer than asking
/// the OS for a one-pixel window.
const DIMENSION_RANGE: RangeInclusive<i32> = 640..=7680;

/// The ranges the settings page itself moves within (`main.rs`). A hand-edited
/// value outside one is pulled back into it rather than thrown away: a file that
/// says `sensitivity = 99` means "as fast as it goes", and resetting it to the
/// default would be a silent lie about what was read.
const SENSITIVITY_RANGE: RangeInclusive<f32> = 0.30..=5.0;
const VOLUME_RANGE: RangeInclusive<f32> = 0.0..=1.0;
/// The mouse's counts per inch, as the DPI row and the cm/360 readout believe
/// it. The range is a typo guard rather than a rule: nothing in the game
/// divides by this number, but a mouse with three counts to the inch does not
/// exist, and neither does one with a million.
const DPI_RANGE: RangeInclusive<u32> = 100..=100_000;
/// Campaign progress is clamped rather than refused: a hand-edited `unlocked =
/// 400` should not be believed by `continue`, but it is also not a reason to
/// throw the line away — the campaigns are shorter than this by a wide margin.
const MAX_UNLOCKED: usize = 99;

/// Everything the game remembers between runs.
///
/// The fields are the settings page's rows: the values the page edits, plus the
/// language and the window, which the page also shows but does not own — `i18n`
/// owns the live language and the OS owns the live window, so those two are
/// copied in and out of here at load and save time.
#[derive(Clone, Debug, PartialEq)]
pub struct Config {
    pub fullscreen: bool,
    pub width: i32,
    pub height: i32,
    pub sensitivity: f32,
    /// The mouse's counts per inch. Nothing in the game's feel reads it — the
    /// settings page shows the sensitivity as a distance per full turn with it,
    /// which is the number players carry between games.
    pub mouse_dpi: u32,
    pub volume: f32,
    pub aim_toggle: bool,
    pub lean_toggle: bool,
    pub invert_y: bool,
    pub show_fps: bool,
    pub lang: Lang,
    /// How many missions the player has reached, one-based: `1` is a fresh
    /// save and `3` means missions 1 and 2 are done. The file is the only
    /// place campaign progress lives, so quitting is not losing it.
    pub unlocked: usize,
    /// The mission the player was last in, zero-based. `continue` resumes here
    /// rather than at the furthest reached, so deliberately replaying an
    /// earlier mission does not skip the player forward.
    pub last_mission: usize,
    /// The best score on each mission, one entry per campaign slot —
    /// `best_1` … `best_6` in the file. This is what makes the score mean
    /// something after the run is over (reference §19.3): without it every
    /// number the HUD counts is forgotten the moment the mission ends.
    pub best: [u32; crate::campaign::COUNT],
    /// The armory: the build each weapon kind carries and the finish it
    /// wears, one settings spelling per entry of `weapon::ALL_KINDS`, held as
    /// the text the file holds. Parsing is `weapon`'s job and is lossy in the
    /// kind way settings are: a line this build cannot read is repaired, the
    /// way one bad setting costs its own line and no more.
    pub builds: [String; 7],
    pub paints: [String; 7],
    /// The loadout the stash carries into the field: the two long-gun slots
    /// (kind keys, or `none`), how many spare magazines each slot brings, and
    /// the consumable counts. `Kit` turns these into kinds and clamps them.
    pub primary: String,
    pub secondary: String,
    pub primary_mags: u32,
    pub secondary_mags: u32,
    pub frags: u32,
    pub smokes: u32,
    pub bandages: u32,
    pub medkits: u32,
    pub painkillers: u32,
}

impl Default for Config {
    /// The values the game shipped with before there was a file: 1280×720
    /// windowed, the mouse at 1.60 on an 800-count mouse, the master volume at
    /// 0.80, aim and lean on toggle, no Y inversion, no FPS counter, Chinese.
    /// `main` used to set the language and the volume by hand at startup; a
    /// missing file has to produce the same game, so these are the numbers it
    /// names.
    fn default() -> Self {
        Self {
            fullscreen: false,
            width: 1280,
            height: 720,
            sensitivity: 1.60,
            mouse_dpi: 800,
            volume: 0.80,
            aim_toggle: true,
            lean_toggle: true,
            invert_y: false,
            show_fps: false,
            lang: Lang::Zh,
            unlocked: 1,
            last_mission: 0,
            best: [0; crate::campaign::COUNT],
            // No parts, no finish: a fresh save is every weapon as issued.
            builds: std::array::from_fn(|_| String::new()),
            paints: std::array::from_fn(|_| String::new()),
            // The loadout as issued: a rifle with four spare magazines, no
            // second gun, and the consumable counts missions have always
            // started with.
            primary: "rifle".to_string(),
            secondary: "none".to_string(),
            primary_mags: 4,
            secondary_mags: 0,
            frags: 2,
            smokes: 1,
            bandages: 3,
            medkits: 1,
            painkillers: 1,
        }
    }
}

/// The settings file's path, read out of the environment. `None` when the
/// environment names no per-user settings directory at all.
pub fn path() -> Option<PathBuf> {
    path_in(
        std::env::var("APPDATA").ok().as_deref(),
        std::env::var("XDG_CONFIG_HOME").ok().as_deref(),
        std::env::var("HOME").ok().as_deref(),
    )
}

/// The settings path for the three environment values `path` reads.
///
/// Taking them as arguments is what makes the choice testable: `set_var` is not
/// safe to call from a test binary that runs its tests on threads, and a test
/// that mutated the process environment would be mutating it for every other
/// test at the same time.
///
/// `APPDATA` is the Windows answer and is tried first, `XDG_CONFIG_HOME` is the
/// Linux one, and `$HOME/.config` is the fallback the XDG spec itself names. In
/// practice only one of the three is ever set — a Windows session sets `APPDATA`
/// and neither of the others, a Linux session sets one of the XDG pair — so each
/// platform lands where its own convention says without a `cfg` in the middle of
/// the decision, and a machine that sets none of them gets `None`.
pub fn path_in(appdata: Option<&str>, xdg: Option<&str>, home: Option<&str>) -> Option<PathBuf> {
    path_in_with(DIR, appdata, xdg, home)
}

/// `path_in` for any directory name: the current one, or the pre-rename one.
fn path_in_with(
    dir: &str,
    appdata: Option<&str>,
    xdg: Option<&str>,
    home: Option<&str>,
) -> Option<PathBuf> {
    // An empty variable is the same as an unset one: some shells export
    // `XDG_CONFIG_HOME=` and some build environments leave `APPDATA` empty.
    let base = [appdata, xdg]
        .into_iter()
        .flatten()
        .find(|dir| !dir.trim().is_empty())
        .map(PathBuf::from)
        .or_else(|| {
            let home = home.filter(|dir| !dir.trim().is_empty())?;
            Some(Path::new(home).join(".config"))
        })?;
    Some(base.join(dir).join(FILE))
}

/// Reads the settings, or the defaults when there is nothing to read.
///
/// Never fails: a missing file is the first run, and a file that cannot be read
/// or understood is reported on stderr and answered with the defaults.
pub fn load() -> Config {
    match path() {
        Some(path) => load_from(&source_path(path, legacy_path())),
        None => Config::default(),
    }
}

/// Which file to read: the current one, or the pre-rename one when the current
/// one has never been written. Split out so the rule can be tested with two
/// files in a temporary directory rather than by renaming the reader.
fn source_path(current: PathBuf, legacy: Option<PathBuf>) -> PathBuf {
    match legacy {
        Some(old) if !current.exists() && old.exists() => old,
        _ => current,
    }
}

/// The pre-rename settings path, from the same environment `path` reads.
pub fn legacy_path() -> Option<PathBuf> {
    let appdata = std::env::var("APPDATA").ok();
    let xdg = std::env::var("XDG_CONFIG_HOME").ok();
    let home = std::env::var("HOME").ok();
    LEGACY_DIRS
        .iter()
        .filter_map(|dir| path_in_with(dir, appdata.as_deref(), xdg.as_deref(), home.as_deref()))
        .find(|path| path.exists())
}

/// Writes the settings, creating the settings directory on the first save.
///
/// A failed write is one line on stderr: the settings are still in memory and
/// the session carries on with them, because a read-only home directory is not a
/// reason to interrupt a game.
pub fn save(config: &Config) {
    let Some(path) = path() else {
        // No settings directory in the environment: the game runs, the setting
        // simply does not outlive the session. There is nowhere to report this
        // to either, so it is silent.
        return;
    };
    if let Err(e) = save_to(&path, config) {
        eprintln!("tides-of-rlyeh: cannot write {}: {e}", path.display());
    }
}

/// `load` for a path the caller resolved. Splitting it from the environment is
/// what lets a test read a file it wrote itself rather than the player's.
fn load_from(path: &Path) -> Config {
    match std::fs::read_to_string(path) {
        Ok(text) => parse(&text),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Config::default(),
        Err(e) => {
            eprintln!("tides-of-rlyeh: cannot read {}: {e}", path.display());
            Config::default()
        }
    }
}

/// `save` for a path the caller resolved: see `load_from`.
fn save_to(path: &Path, config: &Config) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    std::fs::File::create(path)?.write_all(render(config).as_bytes())
}

/// The text a settings file holds for these settings. See the module docs for
/// the format; `the_documented_sample_is_what_a_default_file_holds` keeps the
/// sample there honest.
fn render(config: &Config) -> String {
    use std::fmt::Write as _;

    let mut out = String::with_capacity(256);
    out.push_str("# TIDES OF R'LYEH settings. Delete this file to go back to the defaults.\n");
    // Floats use `{}`, the shortest text that reads back as the same value: a
    // fixed number of decimals would round a hand-edited 1.2345678 on the way
    // out, and the file is one people are meant to edit.
    let _ = writeln!(out, "fullscreen = {}", config.fullscreen);
    let _ = writeln!(out, "width = {}", config.width);
    let _ = writeln!(out, "height = {}", config.height);
    let _ = writeln!(out, "sensitivity = {}", config.sensitivity);
    let _ = writeln!(out, "mouse_dpi = {}", config.mouse_dpi);
    let _ = writeln!(out, "volume = {}", config.volume);
    let _ = writeln!(out, "aim_toggle = {}", config.aim_toggle);
    let _ = writeln!(out, "lean_toggle = {}", config.lean_toggle);
    let _ = writeln!(out, "invert_y = {}", config.invert_y);
    let _ = writeln!(out, "show_fps = {}", config.show_fps);
    let _ = writeln!(
        out,
        "lang = {}",
        match config.lang {
            Lang::En => "en",
            Lang::Zh => "zh",
        }
    );
    let _ = writeln!(out, "unlocked = {}", config.unlocked);
    let _ = writeln!(out, "last_mission = {}", config.last_mission);
    // Best scores, written only once a mission has been played, so a fresh
    // file stays the short one the module docs show.
    for (i, best) in config.best.iter().enumerate() {
        if *best > 0 {
            let _ = writeln!(out, "best_{} = {}", i + 1, best);
        }
    }
    // The armory: only what has been touched is written, so a fresh file stays
    // the short one the module docs show. Built in `ALL_KINDS` order, the same
    // order the arrays are read back in.
    for (i, kind) in crate::weapon::ALL_KINDS.iter().enumerate() {
        if !config.builds[i].is_empty() {
            let _ = writeln!(out, "build_{} = {}", kind.key(), config.builds[i]);
        }
        if !config.paints[i].is_empty() {
            let _ = writeln!(out, "paint_{} = {}", kind.key(), config.paints[i]);
        }
    }
    // The loadout, written only once it differs from the issued kit, so a
    // fresh file stays the short one the module docs show.
    let issued = Config::default();
    if config.primary != issued.primary {
        let _ = writeln!(out, "primary = {}", config.primary);
    }
    if config.secondary != issued.secondary {
        let _ = writeln!(out, "secondary = {}", config.secondary);
    }
    if config.primary_mags != issued.primary_mags {
        let _ = writeln!(out, "primary_mags = {}", config.primary_mags);
    }
    if config.secondary_mags != issued.secondary_mags {
        let _ = writeln!(out, "secondary_mags = {}", config.secondary_mags);
    }
    if config.frags != issued.frags {
        let _ = writeln!(out, "frags = {}", config.frags);
    }
    if config.smokes != issued.smokes {
        let _ = writeln!(out, "smokes = {}", config.smokes);
    }
    if config.bandages != issued.bandages {
        let _ = writeln!(out, "bandages = {}", config.bandages);
    }
    if config.medkits != issued.medkits {
        let _ = writeln!(out, "medkits = {}", config.medkits);
    }
    if config.painkillers != issued.painkillers {
        let _ = writeln!(out, "painkillers = {}", config.painkillers);
    }
    out
}

/// Reads a settings file.
///
/// Every field starts at its default and is replaced only by a line that reads
/// cleanly, which is what keeps a damaged file costing one setting instead of
/// all of them.
fn parse(text: &str) -> Config {
    let mut config = Config::default();
    for line in text.lines() {
        // A comment runs to the end of the line, so it may follow a value.
        let line = match line.split_once('#') {
            Some((before, _)) => before,
            None => line,
        };
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let value = value.trim();
        match key.trim().to_ascii_lowercase().as_str() {
            "fullscreen" => set_bool(&mut config.fullscreen, value),
            "width" => set_dimension(&mut config.width, value),
            "height" => set_dimension(&mut config.height, value),
            "sensitivity" => set_float(&mut config.sensitivity, value, SENSITIVITY_RANGE),
            "mouse_dpi" => set_dpi(&mut config.mouse_dpi, value),
            "volume" => set_float(&mut config.volume, value, VOLUME_RANGE),
            "aim_toggle" => set_bool(&mut config.aim_toggle, value),
            "lean_toggle" => set_bool(&mut config.lean_toggle, value),
            "invert_y" => set_bool(&mut config.invert_y, value),
            "show_fps" => set_bool(&mut config.show_fps, value),
            "lang" => {
                if let Some(lang) = parse_lang(value) {
                    config.lang = lang;
                }
            }
            "unlocked" => set_progress(&mut config.unlocked, value),
            "last_mission" => set_progress(&mut config.last_mission, value),
            // The armory, one key per weapon kind. The same names `render`
            // writes; `weapon::ALL_KINDS` order is the array order.
            "build_rifle" => set_text(&mut config.builds[0], value),
            "build_shotgun" => set_text(&mut config.builds[1], value),
            "build_smg" => set_text(&mut config.builds[2], value),
            "build_sniper" => set_text(&mut config.builds[3], value),
            "build_lmg" => set_text(&mut config.builds[4], value),
            "build_pistol" => set_text(&mut config.builds[5], value),
            "build_knife" => set_text(&mut config.builds[6], value),
            "paint_rifle" => set_text(&mut config.paints[0], value),
            "paint_shotgun" => set_text(&mut config.paints[1], value),
            "paint_smg" => set_text(&mut config.paints[2], value),
            "paint_sniper" => set_text(&mut config.paints[3], value),
            "paint_lmg" => set_text(&mut config.paints[4], value),
            "paint_pistol" => set_text(&mut config.paints[5], value),
            "paint_knife" => set_text(&mut config.paints[6], value),
            // The loadout. The kinds are validated by `Kit` against the weapon
            // table; the counts are clamped here, so a typo costs its own line.
            "primary" => set_text(&mut config.primary, value),
            "secondary" => set_text(&mut config.secondary, value),
            "primary_mags" => set_count(&mut config.primary_mags, value, 0..=8),
            "secondary_mags" => set_count(&mut config.secondary_mags, value, 0..=8),
            "frags" => set_count(&mut config.frags, value, 0..=4),
            "smokes" => set_count(&mut config.smokes, value, 0..=3),
            "bandages" => set_count(&mut config.bandages, value, 0..=5),
            "medkits" => set_count(&mut config.medkits, value, 0..=5),
            "painkillers" => set_count(&mut config.painkillers, value, 0..=5),
            // A best score per mission, `best_1` … `best_6`. One guarded arm
            // rather than a list, so the campaign can grow without a key to
            // remember; anything else falls through and is ignored, so a file
            // written by a newer build is still readable here.
            _ => {
                let lower = key.trim().to_ascii_lowercase();
                if let Some(n) = lower
                    .strip_prefix("best_")
                    .and_then(|rest| rest.parse::<usize>().ok())
                    && (1..=crate::campaign::COUNT).contains(&n)
                {
                    set_best(&mut config.best[n - 1], value);
                }
            }
        }
    }
    config
}

/// Assigns `slot` when the value reads as a boolean, and leaves it alone — at
/// its default, or at the last good line for a repeated key — when it does not.
fn set_bool(slot: &mut bool, value: &str) {
    let parsed = match value.to_ascii_lowercase().as_str() {
        "true" | "on" | "1" => Some(true),
        "false" | "off" | "0" => Some(false),
        _ => None,
    };
    if let Some(v) = parsed {
        *slot = v;
    }
}

/// Assigns `slot` a whole number inside the accepted range; a size outside it is
/// not believed, because it would be handed straight to the window.
fn set_dimension(slot: &mut i32, value: &str) {
    if let Ok(v) = value.parse::<i32>()
        && DIMENSION_RANGE.contains(&v)
    {
        *slot = v;
    }
}

/// Assigns `slot` a progress counter. `0` is accepted for `last_mission`, which
/// is zero-based, and everything is clamped to `MAX_UNLOCKED`.
fn set_progress(slot: &mut usize, value: &str) {
    if let Ok(v) = value.parse::<usize>() {
        *slot = v.min(MAX_UNLOCKED);
    }
}

/// Assigns `slot` a score. Unlike progress there is nothing meaningful to
/// clamp to — a score is the player's own record, not a length — so anything
/// that parses is believed.
fn set_best(slot: &mut u32, value: &str) {
    if let Ok(v) = value.parse::<u32>() {
        *slot = v;
    }
}

/// Assigns `slot` the value as written, even empty. The armory's text is
/// parsed — and repaired — by the weapon modules at load, so the file's job is
/// only to carry it faithfully.
fn set_text(slot: &mut String, value: &str) {
    slot.clear();
    slot.push_str(value.trim());
}

/// Assigns `slot` a whole count inside `range`; a value outside it is a typo
/// and is not believed, the way a window size outside its range is not.
fn set_count(slot: &mut u32, value: &str, range: RangeInclusive<u32>) {
    if let Ok(v) = value.parse::<u32>()
        && range.contains(&v)
    {
        *slot = v;
    }
}

/// Assigns `slot` a mouse resolution in counts per inch, inside the accepted
/// range. The value only feeds the cm/360 readout, so this is a typo guard
/// rather than a rule: a mouse with three counts to the inch does not exist.
fn set_dpi(slot: &mut u32, value: &str) {
    if let Ok(v) = value.parse::<u32>()
        && DPI_RANGE.contains(&v)
    {
        *slot = v;
    }
}

/// Assigns `slot` a finite float, pulled into its range; a value that is not a
/// number — or is a NaN, which would poison every later comparison — is left.
fn set_float(slot: &mut f32, value: &str, range: RangeInclusive<f32>) {
    if let Ok(v) = value.parse::<f32>()
        && v.is_finite()
    {
        *slot = v.clamp(*range.start(), *range.end());
    }
}

/// The file's two language names. The full labels are accepted too, so that a
/// player who copies what the settings page shows writes a file that works.
fn parse_lang(value: &str) -> Option<Lang> {
    match value.to_ascii_lowercase().as_str() {
        "en" | "english" => Some(Lang::En),
        "zh" | "chinese" | "中文" => Some(Lang::Zh),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The directory the tests write their throwaway files into.
    ///
    /// `std::env::temp_dir()` is the natural home for one, and is tried first.
    /// A locked-down Windows host, however, can refuse new files there to a
    /// binary built in a workspace — the test binary lives in `target/` and is
    /// treated as untrusted — so the directory the test itself was built into is
    /// the fallback. It is writable by definition: the build has just written
    /// the test into it.
    fn scratch_dir() -> PathBuf {
        let temp = std::env::temp_dir();
        let probe = temp.join(format!("tides-of-rlyeh-{}.probe", std::process::id()));
        match std::fs::File::create(&probe) {
            Ok(_) => {
                let _ = std::fs::remove_file(&probe);
                temp
            }
            Err(_) => Path::new(env!("CARGO_MANIFEST_DIR")).join("target"),
        }
    }

    /// A path that no other test uses: the tag keeps this module's tests from
    /// colliding with each other when they run in parallel, and the pid keeps
    /// two `cargo test` runs apart.
    fn temp_file(tag: &str) -> PathBuf {
        scratch_dir().join(format!(
            "tides-of-rlyeh-{}-{tag}.config",
            std::process::id()
        ))
    }

    /// The path a base directory produces, so the tests below state only the
    /// base they are testing.
    fn under(base: &str) -> Option<PathBuf> {
        Some(PathBuf::from(base).join(DIR).join(FILE))
    }

    /// The defaults are the settings the game shipped with. The file only earns
    /// its keep if a machine without one still gets the same game, so the
    /// numbers are spelled out here rather than compared with a copy of
    /// themselves.
    #[test]
    fn the_defaults_are_the_settings_the_game_shipped_with() {
        let c = Config::default();
        assert_eq!((c.width, c.height), (1280, 720));
        assert!(!c.fullscreen);
        assert!((c.sensitivity - 1.60).abs() < 1e-6);
        assert_eq!(c.mouse_dpi, 800);
        assert!((c.volume - 0.80).abs() < 1e-6);
        assert!(c.aim_toggle);
        assert!(c.lean_toggle);
        assert!(!c.invert_y);
        assert!(!c.show_fps);
        assert_eq!(c.lang, Lang::Zh, "the game opens in Chinese");
    }

    /// Every field has to survive a real file: written by `save`, read back by
    /// `load`, unchanged. A field that `render` forgets, or `parse` does not
    /// know, shows up here as a default where the test put a value.
    #[test]
    fn saved_settings_load_back_identical() {
        let path = temp_file("round-trip");
        let want = Config {
            fullscreen: true,
            width: 1600,
            height: 900,
            sensitivity: 2.35,
            mouse_dpi: 1600,
            volume: 0.35,
            aim_toggle: false,
            lean_toggle: false,
            invert_y: true,
            show_fps: true,
            lang: Lang::En,
            unlocked: 4,
            last_mission: 3,
            // Two nonzero entries: the round trip has to keep the key
            // numbering and the absence of a score distinguishable.
            best: [0, 1500, 325, 0, 0, 0],
            builds: {
                let mut v: [String; 7] = std::array::from_fn(|_| String::new());
                v[0] = "magazine=extended_mag@0.50".to_string();
                v
            },
            paints: {
                let mut v: [String; 7] = std::array::from_fn(|_| String::new());
                v[2] = "desert@0.41:2718".to_string();
                v
            },
            primary: "sniper".to_string(),
            secondary: "shotgun".to_string(),
            primary_mags: 6,
            secondary_mags: 3,
            frags: 0,
            smokes: 3,
            bandages: 5,
            medkits: 2,
            painkillers: 0,
        };
        assert!(
            save_to(&path, &want).is_ok(),
            "cannot write {}",
            path.display()
        );
        let got = load_from(&path);
        let _ = std::fs::remove_file(&path);
        assert_eq!(got, want);
    }

    /// Floats are written with enough precision to read back as the same value,
    /// which is what the shortest-round-trip formatting buys: the settings page
    /// steps in hundredths, but the file can hold anything a player typed.
    #[test]
    fn floats_survive_the_round_trip() {
        let path = temp_file("floats");
        let want = Config {
            sensitivity: 4.37,
            volume: 0.05,
            ..Config::default()
        };
        save_to(&path, &want).expect("the temp file is writable");
        let got = load_from(&path);
        let _ = std::fs::remove_file(&path);
        assert!((got.sensitivity - want.sensitivity).abs() < 1e-6);
        assert!((got.volume - want.volume).abs() < 1e-6);
    }

    /// A damaged file is repaired field by field. One unreadable line costs its
    /// own setting; the good lines around it are kept.
    #[test]
    fn a_damaged_file_falls_back_field_by_field() {
        let text = concat!(
            "# a comment\n",
            "\n",
            "width = 1920\n",
            "height = wide   # not a number\n",
            "fullscreen = maybe\n",
            "mystery = 7\n",
            "sensitivity = -3\n",
            "volume = loud\n",
            "aim_toggle = off\n",
            "show_fps = 1\n",
        );
        let c = parse(text);
        assert_eq!(c.width, 1920, "a good line is read");
        assert_eq!(c.height, Config::default().height);
        assert_eq!(c.fullscreen, Config::default().fullscreen);
        assert!(
            (c.sensitivity - 0.30).abs() < 1e-6,
            "a speed below the range is pulled up to it, not discarded"
        );
        assert!((c.volume - Config::default().volume).abs() < 1e-6);
        assert!(!c.aim_toggle);
        assert!(c.show_fps);
        // Keys the file never mentions keep their defaults.
        assert_eq!(c.lean_toggle, Config::default().lean_toggle);
        assert_eq!(c.invert_y, Config::default().invert_y);
        assert_eq!(c.lang, Config::default().lang);
    }

    /// Whitespace, key case and trailing comments are tolerated: this is a file
    /// people are meant to be able to edit by hand.
    #[test]
    fn the_parser_tolerates_hand_edits() {
        let c = parse("  WIDTH   =   1920   # a big monitor\nLANG=EN\nVolume\t=\t1\n");
        assert_eq!(c.width, 1920);
        assert_eq!(c.lang, Lang::En);
        assert!((c.volume - 1.0).abs() < 1e-6);
    }

    /// Nothing to read is the default game: an empty file, a file of comments,
    /// and a file that is not there at all.
    #[test]
    fn a_missing_or_empty_file_is_the_defaults() {
        assert_eq!(parse(""), Config::default());
        assert_eq!(parse("# nothing here\n\n   \n"), Config::default());
        assert_eq!(parse("this line has no equals sign\n"), Config::default());
        assert_eq!(
            load_from(Path::new("tides-of-rlyeh-file-that-does-not-exist.config")),
            Config::default()
        );
    }

    /// The file used to live in a directory named after the old title. A save
    /// written then still has to load, or renaming the game would silently cost
    /// the player their settings and their progress.
    #[test]
    fn a_save_from_before_the_rename_is_still_read() {
        let old = temp_file("rename-legacy");
        let new = temp_file("rename-current");
        let _ = std::fs::remove_file(&new);

        let before = Config {
            volume: 0.42,
            unlocked: 5,
            ..Config::default()
        };
        save_to(&old, &before).expect("write the pre-rename file");

        // Nothing has been written to the new path yet: the old one is read, and
        // its values survive.
        let chosen = source_path(new.clone(), Some(old.clone()));
        assert_eq!(chosen, old, "the pre-rename file was ignored");
        let got = load_from(&chosen);
        assert_eq!(got.volume, 0.42);
        assert_eq!(got.unlocked, 5);

        // Once the current file exists it wins, so the rename is a one-way trip.
        save_to(&new, &Config::default()).expect("write the current file");
        assert_eq!(source_path(new.clone(), Some(old.clone())), new);

        // No legacy file at all (a first run, or one already migrated): the
        // current path is the answer whether or not it exists.
        assert_eq!(source_path(new.clone(), None), new);
        let _ = std::fs::remove_file(&old);
        let _ = std::fs::remove_file(&new);
    }

    /// Best scores come back by mission number, and a mission number the
    /// campaign does not have is ignored like any other unknown key.
    #[test]
    fn best_scores_round_trip_by_mission() {
        let c = parse("best_1 = 1250\nbest_6 = 98765\nbest_7 = 1\nbest_0 = 1\n");
        assert_eq!(c.best[0], 1250);
        assert_eq!(c.best[5], 98765);
        assert_eq!(c.best[1], 0, "an unwritten mission has no score");
        // The file only carries the missions that have a score on them.
        let text = render(&c);
        assert!(text.contains("best_1 = 1250"));
        assert!(!text.contains("best_7"));
        assert!(!text.contains("best_2"));
    }

    /// A window size is only believable inside the range the game will hand to
    /// the OS; the ends of the range are inside it.
    #[test]
    fn dimensions_outside_the_range_fall_back() {
        for bad in ["100", "0", "-1920", "99999", "1920.5", "", "1920x1080"] {
            let c = parse(&format!("width = {bad}\nheight = {bad}\n"));
            assert_eq!(
                c.width,
                Config::default().width,
                "width = {bad} was believed"
            );
            assert_eq!(
                c.height,
                Config::default().height,
                "height = {bad} was believed"
            );
        }
        let c = parse("width = 640\nheight = 7680\n");
        assert_eq!((c.width, c.height), (640, 7680));
    }

    /// The three places a per-user settings directory comes from, in order, and
    /// the machine that has none of them.
    #[test]
    fn the_config_path_follows_the_platform_conventions() {
        assert_eq!(
            path_in(Some(r"C:\Users\me\AppData\Roaming"), None, None),
            under(r"C:\Users\me\AppData\Roaming")
        );
        assert_eq!(
            path_in(None, Some("/home/me/.config"), Some("/home/me")),
            under("/home/me/.config"),
            "XDG_CONFIG_HOME wins over the HOME fallback"
        );
        assert_eq!(
            path_in(None, None, Some("/home/me")),
            under("/home/me/.config"),
            "HOME is the documented fallback"
        );
        assert_eq!(path_in(None, None, None), None);
        // An empty variable is an unset one.
        assert_eq!(path_in(Some(""), Some(""), Some("")), None);
        assert_eq!(
            path_in(Some(""), None, Some("/home/me")),
            under("/home/me/.config")
        );
    }

    /// A save that cannot be written reports an error instead of taking the game
    /// down with it. A directory is the cheapest unwritable path: it exists, so
    /// nothing can be created at its own name.
    #[test]
    fn a_failed_write_is_an_error_not_a_panic() {
        assert!(save_to(&std::env::temp_dir(), &Config::default()).is_err());
    }

    /// The sample in the module docs is the file this build writes. If a field
    /// is added and the sample is not updated, the docs are lying — and the
    /// players who read them will write files that do nothing.
    #[test]
    fn the_documented_sample_is_what_a_default_file_holds() {
        let sample = "# TIDES OF R'LYEH settings. Delete this file to go back to the defaults.\n\
                      fullscreen = false\n\
                      width = 1280\n\
                      height = 720\n\
                      sensitivity = 1.6\n\
                      mouse_dpi = 800\n\
                      volume = 0.8\n\
                      aim_toggle = true\n\
                      lean_toggle = true\n\
                      invert_y = false\n\
                      show_fps = false\n\
                      lang = zh\n\
                      unlocked = 1\n\
                      last_mission = 0\n";
        assert_eq!(render(&Config::default()), sample);
    }
}
