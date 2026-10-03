mod assets;
mod audio;
mod campaign;
mod codex;
mod combat;
mod config;
mod effects;
mod enemy;
mod fog;
mod grenade;
mod hud;
mod i18n;
mod level;
mod maps;
mod mesh;
#[cfg(test)]
mod mesh_tests;
mod pickup;
mod player;
mod pointer;
mod surface;
mod util;
mod viewmodel;
mod weapon;
#[cfg(windows)]
mod win32;

use audio::Sfx;
use campaign::{Mission, Objective};
use enemy::{Attack, Enemy, Kind as EnemyKind};
use grenade::{GrenadeKind, Grenades};
use hud::{CodexRow, HelpSection};
use i18n::Key;
use level::Level;
use macroquad::prelude::*;
use pickup::{Pickup, PickupKind};
use player::{MoveInput, Player, Stance};
use util::{Rng, ray_aabb};
use viewmodel::ViewModel;
use weapon::{Arsenal, SlotClass, WeaponKind};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Phase {
    Menu,
    MissionSelect,
    Settings,
    Armory,
    Playing,
    Paused,
    Result,
}

/// Full-screen overlays that suspend the simulation but keep the world drawn.
///
/// `Manual` is the field's manual: over a menu the same pages are carried by
/// `Game::menu_manual` instead, because a menu has no simulation to suspend.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Overlay {
    None,
    Inventory,
    Map,
    Mission,
    Manual,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum WheelKind {
    Grenade,
    Med,
}

struct Wheel {
    kind: WheelKind,
    selected: usize,
    held: f32,
}

const GRENADE_KINDS: [GrenadeKind; 2] = [GrenadeKind::Frag, GrenadeKind::Smoke];
/// (health restored, seconds channelled, count as issued). The third field is
/// the stash's default count — `Kit::issued` carries the same numbers, and a
/// test pins the two together.
const MED_KINDS: [(f32, f32, u32); 3] = [(25.0, 1.2, 3), (60.0, 2.5, 1), (35.0, 1.8, 1)];
/// Spare magazines a sidearm always carries. Not part of the stash, because a
/// pistol with no ammo is not a loadout choice, it is a nuisance.
const SIDEARM_MAGS: u32 = 2;

fn med_key(i: usize) -> Key {
    match i {
        0 => Key::MedBandage,
        1 => Key::MedKit,
        _ => Key::MedPainkiller,
    }
}

/// The settings the page edits, live. There is no `Default` here on purpose:
/// the defaults live in `config::Config::default`, which is what a machine
/// without a settings file loads, and two tables of defaults would eventually
/// disagree about one of them.
#[derive(Clone, Copy)]
struct Settings {
    sensitivity: f32,
    /// The mouse's counts per inch. Nothing in the game's feel reads it — the
    /// settings row shows the sensitivity as a distance per full turn with it.
    mouse_dpi: u32,
    volume: f32,
    aim_toggle: bool,
    lean_toggle: bool,
    invert_y: bool,
    show_fps: bool,
    fullscreen: bool,
    width: i32,
    height: i32,
}

impl Settings {
    /// The settings the file carries. Every field is named in both directions,
    /// so a field added on one side is a compile error on the other.
    ///
    /// The language is not here: `i18n` owns it, and a copy on `Settings` would
    /// be a second thing to keep in step with it. `to_config` reads it back out
    /// of `i18n` at the moment it builds the file.
    fn from_config(c: &config::Config) -> Self {
        Self {
            sensitivity: c.sensitivity,
            mouse_dpi: c.mouse_dpi,
            volume: c.volume,
            aim_toggle: c.aim_toggle,
            lean_toggle: c.lean_toggle,
            invert_y: c.invert_y,
            show_fps: c.show_fps,
            fullscreen: c.fullscreen,
            width: c.width,
            height: c.height,
        }
    }

    /// By value: `Settings` is `Copy`, and clippy's convention for `to_*` is
    /// that a copyable `self` is cheaper to pass than a reference to it.
    /// The settings half of the file. The campaign progress and the armory are
    /// not settings and live in `Game`, so they are passed in rather than
    /// stored twice.
    fn to_config(
        self,
        unlocked: usize,
        last_mission: usize,
        builds: [String; 7],
        paints: [String; 7],
    ) -> config::Config {
        config::Config {
            fullscreen: self.fullscreen,
            width: self.width,
            height: self.height,
            sensitivity: self.sensitivity,
            mouse_dpi: self.mouse_dpi,
            volume: self.volume,
            aim_toggle: self.aim_toggle,
            lean_toggle: self.lean_toggle,
            invert_y: self.invert_y,
            show_fps: self.show_fps,
            lang: i18n::lang(),
            unlocked,
            last_mission,
            // The best scores belong to `config_of`, which fills them from
            // the game right after; these are the issued values.
            best: [0; crate::campaign::COUNT],
            builds,
            paints,
            // The loadout belongs to `config_of`, which fills it from the kit
            // right after; these are the issued values.
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

/// What the player has made of each weapon kind and how the next mission is
/// equipped: the builds and finishes, the two long-gun choices, and the spare
/// magazines and consumables the stash sends along. Weapon arrays are indexed
/// by `WeaponKind::index`.
struct Kit {
    builds: [weapon::WeaponBuild; 7],
    paints: [viewmodel::Paint; 7],
    /// The two long-gun slots, chosen in the loadout tab.
    primary: Option<weapon::WeaponKind>,
    secondary: Option<weapon::WeaponKind>,
    /// Spare magazines per slot — what each gun's reserve is at mission start.
    ammo_mags: [u32; 2],
    /// Frags and smokes.
    grenades: [u32; 2],
    /// Bandages, medkits, painkillers, in `MED_KINDS` order.
    meds: [u32; 3],
}

/// The caps the loadout tab winds its counts to. They are the whole "cost" of
/// stocking up: there is no economy, so what a player cannot do is carry
/// everything at once.
const LOADOUT_MAGS_MAX: u32 = 8;
const LOADOUT_FRAGS_MAX: u32 = 4;
const LOADOUT_SMOKES_MAX: u32 = 3;
/// The most of each medicine. The three share one cap because they already
/// trade against each other by what they heal.
const LOADOUT_MEDS_MAX: u32 = 5;

impl Kit {
    /// Everything as issued: empty builds, factory paint, a rifle with four
    /// spare magazines, no second gun, and the consumable counts missions have
    /// always started with. A save with no armory lines produces exactly this.
    fn issued() -> Self {
        Self {
            builds: std::array::from_fn(|_| weapon::WeaponBuild::empty()),
            paints: [viewmodel::Paint::factory(); 7],
            primary: Some(weapon::WeaponKind::Rifle),
            secondary: None,
            ammo_mags: [4, 0],
            grenades: [2, 1],
            meds: [3, 1, 1],
        }
    }

    /// The kit the file describes. Build text is parsed by `weapon` and then
    /// screened against the def, so a hand-edited file degrades to the parts
    /// that fit; paint text falls back to the factory finish the same way; a
    /// weapon kind the table does not know falls back to as issued.
    fn from_config(c: &config::Config) -> Self {
        let mut kit = Self::issued();
        for kind in weapon::ALL_KINDS {
            let i = kind.index();
            kit.builds[i] =
                weapon::WeaponBuild::from_text(&c.builds[i]).screened(weapon::def(kind));
            kit.paints[i] = viewmodel::Paint::from_text(&c.paints[i]);
        }
        kit.primary = loadout_slot(&c.primary, kit.primary);
        kit.secondary = loadout_slot(&c.secondary, kit.secondary);
        kit.ammo_mags = [
            c.primary_mags.min(LOADOUT_MAGS_MAX),
            c.secondary_mags.min(LOADOUT_MAGS_MAX),
        ];
        kit.grenades = [
            c.frags.min(LOADOUT_FRAGS_MAX),
            c.smokes.min(LOADOUT_SMOKES_MAX),
        ];
        kit.meds = [c.bandages, c.medkits, c.painkillers].map(|count| count.min(LOADOUT_MEDS_MAX));
        kit
    }

    fn build_texts(&self) -> [String; 7] {
        std::array::from_fn(|i| self.builds[i].to_text())
    }

    fn paint_texts(&self) -> [String; 7] {
        std::array::from_fn(|i| self.paints[i].to_text())
    }

    /// A slot as settings text: the kind's key, or `none`.
    fn slot_text(slot: Option<weapon::WeaponKind>) -> String {
        slot.map(|k| k.key().to_string())
            .unwrap_or_else(|| "none".to_string())
    }
}

/// One loadout slot read from the file: a kind key, `none`, or the fallback
/// when the text names nothing this build knows.
fn loadout_slot(text: &str, fallback: Option<weapon::WeaponKind>) -> Option<weapon::WeaponKind> {
    let text = text.trim();
    if text.eq_ignore_ascii_case("none") {
        return None;
    }
    weapon::kind_named(text).or(fallback)
}

/// The weapon kinds the armory offers: the guns. The knife takes no parts and
/// is not painted.
const ARMORY_KINDS: [weapon::WeaponKind; 6] = [
    weapon::WeaponKind::Rifle,
    weapon::WeaponKind::Shotgun,
    weapon::WeaponKind::Smg,
    weapon::WeaponKind::Sniper,
    weapon::WeaponKind::Lmg,
    weapon::WeaponKind::Pistol,
];

fn armory_kind(i: usize) -> weapon::WeaponKind {
    ARMORY_KINDS[i.min(ARMORY_KINDS.len() - 1)]
}

/// English or Chinese by the live language, for the tables that carry both
/// spellings outside `i18n` (parts, finishes).
fn pick<'a>(en: &'a str, zh: &'a str) -> &'a str {
    match i18n::lang() {
        i18n::Lang::En => en,
        i18n::Lang::Zh => zh,
    }
}

/// A fresh finish's wear: uniform over the case range 0.06 – 0.80. Drawn from
/// `Rng::f32`, which is uniform over whole 24-bit integers scaled down — the
/// classic float-roll bug (most of the mass in the first thousandth) needs an
/// exponent-biased source, and this is not one.
fn roll_wear(rng: &mut Rng) -> f32 {
    rng.range(0.06, 0.80)
}

/// The wear tier a finish reads as, on the bands the reference library's CS2
/// numbers describe.
fn wear_tier(wear: f32) -> &'static str {
    let key = if wear < 0.07 {
        Key::WearFactoryNew
    } else if wear < 0.15 {
        Key::WearMinimal
    } else if wear < 0.37 {
        Key::WearFieldTested
    } else if wear < 0.44 {
        Key::WearWellWorn
    } else {
        Key::WearBattleScarred
    };
    i18n::t(key)
}

/// Whether a bigger number is a better weapon, for the armory's arrows.
enum Better {
    Up,
    Down,
    Flat,
}

/// The numbers the armory shows, and how to read them.
const ARMORY_STATS: [(Key, weapon::StatId, Better); 7] = [
    (Key::StatDamage, weapon::StatId::Damage, Better::Up),
    (Key::StatRate, weapon::StatId::FireInterval, Better::Up),
    (Key::StatSpread, weapon::StatId::BaseSpread, Better::Down),
    (Key::StatRecoil, weapon::StatId::RecoilPitch, Better::Down),
    (Key::StatMag, weapon::StatId::MagSize, Better::Up),
    (Key::StatReload, weapon::StatId::ReloadTime, Better::Down),
    (Key::StatAds, weapon::StatId::AdsFov, Better::Flat),
];

/// The value a stat is judged by — fire interval is shown and compared as a
/// rate, everything else as itself.
fn stat_value(id: weapon::StatId, stats: &weapon::ResolvedStats) -> f32 {
    let v = stats.stat(id);
    if id == weapon::StatId::FireInterval {
        60.0 / v.max(0.02)
    } else {
        v
    }
}

/// A stat's number as the panel spells it.
fn stat_text(id: weapon::StatId, v: f32) -> String {
    match id {
        weapon::StatId::Damage | weapon::StatId::MagSize => format!("{v:.0}"),
        // `v` has already been through `stat_value`: for this stat that is
        // rounds per minute, not the interval.
        weapon::StatId::FireInterval => format!("{v:.0}"),
        weapon::StatId::BaseSpread | weapon::StatId::RecoilPitch => {
            format!("{:.2}°", v.to_degrees())
        }
        weapon::StatId::ReloadTime => format!("{v:.2}s"),
        weapon::StatId::AdsFov => format!("{v:.0}°"),
        _ => format!("{v:.2}"),
    }
}

/// The stat lines for one weapon: each number, the number it moves from, and
/// which way it moved.
///
/// `against` is the build the panel is judging `fitted` by — the issued weapon
/// with nothing on the bench, or the build in the player's hands while a
/// candidate is being weighed. The second reading is what a player actually
/// asks a part ("is this worth fitting"), and with both numbers on screen the
/// line answers it instead of only reporting a sign; `weigh` is that reading,
/// and decides whether the old number is worth printing.
fn armory_stats(
    kind: weapon::WeaponKind,
    def: &weapon::WeaponDef,
    fitted: &weapon::WeaponBuild,
    against: &weapon::WeaponBuild,
    weigh: bool,
) -> Vec<hud::ArmoryStat> {
    let was = weapon::resolve(def, against);
    let now = weapon::resolve(def, fitted);
    let mut stats: Vec<hud::ArmoryStat> = ARMORY_STATS
        .iter()
        .map(|&(key, id, ref better)| {
            let (from, to) = (stat_value(id, &was), stat_value(id, &now));
            let eps = from.abs().max(1.0) * 1.0e-3;
            let good = match better {
                Better::Flat => 0,
                Better::Up if to > from + eps => 1,
                Better::Up if to < from - eps => -1,
                Better::Down if to < from - eps => 1,
                Better::Down if to > from + eps => -1,
                _ => 0,
            };
            hud::ArmoryStat {
                label: i18n::t(key).to_string(),
                text: stat_text(id, to),
                was: weigh.then(|| stat_text(id, from)),
                good,
            }
        })
        .collect();
    // Armour penetration is a property of the platform, not of the build — no
    // part moves it — but it is the number that answers "how does this hold
    // up against a Juggernaut" (reference §17.2③), so it rides with the stat
    // lines. `good: 0` because there is nothing here to have improved on.
    stats.push(hud::ArmoryStat {
        label: i18n::t(Key::StatArmorPen).to_string(),
        text: format!("{:.0}%", kind.armor_pen() * 100.0),
        was: None,
        good: 0,
    });
    stats
}

/// The part a slot holds, if any.
fn fitted_part(build: &weapon::WeaponBuild, slot: weapon::SlotId) -> Option<weapon::PartId> {
    build
        .entries
        .iter()
        .find(|e| e.slot == slot)
        .map(|e| e.part)
}

/// What a slot currently holds, for its row: the part's name, or NONE.
fn fitted_name(build: &weapon::WeaponBuild, slot: weapon::SlotId) -> String {
    fitted_part(build, slot)
        .and_then(weapon::part_label)
        .map(|(en, zh)| pick(en, zh).to_string())
        .unwrap_or_else(|| i18n::t(Key::PartNone).to_string())
}

/// Everything that fits one slot, "none" first and the rest in catalogue
/// order: the bench's candidate column, which is what replaced pressing the
/// right arrow until the wanted part came round.
///
/// `None` for a slot this build does not have. The filter is the same one
/// `validate` applies — the slot's standard and the part's — so a listed
/// candidate always seats, and a part built for another slot is never shown.
fn part_choices(
    def: &weapon::WeaponDef,
    build: &weapon::WeaponBuild,
    slot: weapon::SlotId,
) -> Option<Vec<Option<weapon::PartId>>> {
    let accepts = weapon::reachable_slots(def, build)
        .iter()
        .find(|s| s.id == slot)
        .map(|s| s.accepts)?;
    let mut choices: Vec<Option<weapon::PartId>> = vec![None];
    for part in weapon::catalogue() {
        if part.slot == slot && part.interface == accepts {
            choices.push(Some(part.id));
        }
    }
    Some(choices)
}

/// The build with `part` seated in `slot` at a given roll, or with the slot
/// emptied when `part` is `None`. Everything else in the build is copied
/// through untouched: one slot's part cannot move another slot's.
///
/// Pure, and the same call builds what the bench previews and what a click
/// lands, so the numbers on the panel are the numbers the gun gets.
fn seated(
    build: &weapon::WeaponBuild,
    slot: weapon::SlotId,
    part: Option<weapon::PartId>,
    roll: f32,
) -> weapon::WeaponBuild {
    let mut out = build.clone();
    out.entries.retain(|e| e.slot != slot);
    if let Some(part) = part {
        out.entries.push(weapon::BuildEntry { slot, part, roll });
    }
    out
}

/// The roll a candidate is seated at. A copy already in the slot keeps the roll
/// it came with — re-seating the same part is not a reroll, so the bench cannot
/// be clicked twice for a better number — while a new part samples a fresh one,
/// which is what makes two copies of a ranged part differ. "None" seats nothing
/// and so rolls nothing.
fn seat_roll(
    rng: &mut Rng,
    build: &weapon::WeaponBuild,
    slot: weapon::SlotId,
    part: Option<weapon::PartId>,
) -> f32 {
    match (part, build.entries.iter().find(|e| e.slot == slot)) {
        (Some(part), Some(entry)) if entry.part == part => entry.roll,
        (Some(_), _) => rng.f32(),
        (None, _) => 0.0,
    }
}

/// Fit one of the focused slot's candidates: what clicking it in the middle
/// column and pressing Enter on it both mean.
///
/// The roll comes from the preview when the panel was already describing this
/// exact choice, so what the player is looking at is what the click lands; a
/// candidate that was never previewed takes a fresh roll, like any newly
/// seated part. Returns whether the weapon actually changed.
fn fit_candidate(
    game: &mut Game,
    kind: weapon::WeaponKind,
    slot: weapon::SlotId,
    candidate: usize,
    choices: &[Option<weapon::PartId>],
) -> bool {
    let Some(part) = choices.get(candidate).copied() else {
        return false;
    };
    let at = kind.index();
    let previewed = game
        .armory_preview
        .as_ref()
        .filter(|p| p.kind == at && p.slot == slot && p.candidate == candidate)
        .and_then(|p| p.build.entries.iter().find(|e| e.slot == slot))
        .map(|e| e.roll);
    let roll = match previewed {
        Some(roll) => roll,
        None => seat_roll(&mut game.rng, &game.kit.builds[at], slot, part),
    };
    let build = seated(&game.kit.builds[at], slot, part, roll);
    if build == game.kit.builds[at] {
        return false;
    }
    game.kit.builds[at] = build;
    true
}

/// The candidate the bench's panel is describing without having fitted it:
/// which weapon, which slot and which row of the candidate column it is, and
/// the build a click on it would produce.
///
/// Kept between frames so the roll the panel shows is the roll the click lands
/// (a ranged part would otherwise show one number and get another), and so the
/// preview mesh is rebuilt only when the picture really changes.
struct ArmoryPreview {
    kind: usize,
    slot: weapon::SlotId,
    candidate: usize,
    build: weapon::WeaponBuild,
}

/// Cycle a loadout slot through "none" and the six guns. A pistol is as valid
/// a primary as a rifle, if that is the run the player wants.
fn step_kind_choice(cur: Option<weapon::WeaponKind>, forward: bool) -> Option<weapon::WeaponKind> {
    let options: Vec<Option<weapon::WeaponKind>> = std::iter::once(None)
        .chain(ARMORY_KINDS.iter().copied().map(Some))
        .collect();
    let at = options.iter().position(|o| *o == cur).unwrap_or(0) as i32;
    let n = options.len() as i32;
    let next = if forward {
        (at + 1).rem_euclid(n)
    } else {
        (at + n - 1).rem_euclid(n)
    };
    options[next as usize]
}

/// A window mode: whether it is fullscreen, and the size it should have when it
/// is not.
#[derive(Clone, Copy, PartialEq, Eq)]
struct WindowState {
    fullscreen: bool,
    width: i32,
    height: i32,
}

/// What the window still has to be told, given what it was last told.
///
/// The size is deliberately not pushed while fullscreen: the mode decides it,
/// and miniquad's `set_window_size` would resize the fullscreen window itself —
/// it keeps the borderless style, so the window would stop covering the screen.
/// The choice is not lost: it is recorded, and applied by the same change that
/// leaves fullscreen.
fn window_calls(want: WindowState, applied: WindowState) -> (Option<bool>, Option<(i32, i32)>) {
    let fullscreen = (want.fullscreen != applied.fullscreen).then_some(want.fullscreen);
    // Coming back from fullscreen has to re-apply the size even when it never
    // changed, because the mode change leaves the window at whatever size the OS
    // picked — hence the whole-state comparison rather than a size comparison.
    let size = (!want.fullscreen && want != applied).then_some((want.width, want.height));
    (fullscreen, size)
}

/// Pushes the settings at everything that reads them, and writes the file.
///
/// Every row on the settings page ends here, so "changed something" and "saved
/// it" cannot come apart as rows are added. The window is only touched when it
/// actually has to move — see `window_calls` — because both of its calls are
/// requests to the OS, while the volume is re-set unconditionally because it is
/// a field assignment and the audio side is the only other reader.
/// The whole settings file, from the one place that owns every field in it:
/// the settings screen, the window mode, the campaign progress and the stash.
fn config_of(game: &Game) -> config::Config {
    let mut config = game.settings.to_config(
        game.unlocked,
        game.last_mission,
        game.kit.build_texts(),
        game.kit.paint_texts(),
    );
    // The scores half of the record.
    config.best = game.best;
    // The loadout half of the stash.
    config.primary = Kit::slot_text(game.kit.primary);
    config.secondary = Kit::slot_text(game.kit.secondary);
    config.primary_mags = game.kit.ammo_mags[0];
    config.secondary_mags = game.kit.ammo_mags[1];
    config.frags = game.kit.grenades[0];
    config.smokes = game.kit.grenades[1];
    config.bandages = game.kit.meds[0];
    config.medkits = game.kit.meds[1];
    config.painkillers = game.kit.meds[2];
    config
}

/// Writes the file when the campaign moves on. Called from the two places a
/// mission is marked finished, so quitting mid-campaign cannot lose it.
fn save_progress(game: &Game) {
    config::save(&config_of(game));
}

fn apply_settings(game: &mut Game, audio: &mut audio::Audio) {
    let want = WindowState {
        fullscreen: game.settings.fullscreen,
        width: game.settings.width,
        height: game.settings.height,
    };
    let (fullscreen, size) = window_calls(want, game.window_state);
    if let Some(on) = fullscreen {
        set_fullscreen(on);
    }
    if let Some((w, h)) = size {
        // `Conf`, which built the window at startup, reads its size as physical
        // pixels; this call reads logical points and multiplies by the display's
        // scale itself. Undo that here so both paths mean the same thing: on a
        // 150% display a 1920 × 1080 entry must be the window the game opens at
        // 1920 × 1080, not one 1.5 times larger that spills off the screen.
        let scale = screen_dpi_scale().max(1.0);
        request_new_screen_size(w as f32 / scale, h as f32 / scale);
    }
    game.window_state = want;
    audio.set_master(game.settings.volume);
    config::save(&config_of(game));
}

/// The resolution row's presets: the sizes the game is framed for. The list is
/// short on purpose — every entry has to work on the screens players actually
/// have — and the file may hold a size that is not in it.
const RESOLUTIONS: [(i32, i32); 4] = [(1280, 720), (1600, 900), (1920, 1080), (2560, 1440)];

/// The resolution row's list: the presets, with the current size appended when
/// it is not one of them.
///
/// The alternative was to snap an unknown size to the nearest preset. That would
/// rewrite a size the file still names on the next launch and quietly change a
/// window the player had chosen; carrying it as an extra entry costs one row and
/// keeps the file honest about what is in it.
fn resolution_options(current: (i32, i32)) -> Vec<(i32, i32)> {
    let mut options = RESOLUTIONS.to_vec();
    if !options.contains(&current) {
        options.push(current);
    }
    options
}

/// Moves the resolution one step through `resolution_options`, wrapping at both
/// ends.
fn step_resolution(settings: &mut Settings, forward: bool) {
    let options = resolution_options((settings.width, settings.height));
    let current = options
        .iter()
        .position(|size| *size == (settings.width, settings.height))
        .unwrap_or(0);
    let next = if forward {
        (current + 1) % options.len()
    } else {
        (current + options.len() - 1) % options.len()
    };
    if let Some(&(w, h)) = options.get(next) {
        settings.width = w;
        settings.height = h;
    }
}

/// The DPI presets the mouse DPI row walks: the run of values gaming mice
/// actually ship with. The file may hold a value that is not here, and it is
/// carried as an extra entry, the same way an odd window size is.
const DPI_PRESETS: [u32; 6] = [400, 800, 1200, 1600, 2400, 3200];

/// The DPI row's list: the presets, plus the current value when it is not one
/// of them. Sorted, because DPI is one number and stepping it should walk in
/// order — an odd value belongs where it falls, not at the end.
fn dpi_options(current: u32) -> Vec<u32> {
    let mut options = DPI_PRESETS.to_vec();
    if !options.contains(&current) {
        options.push(current);
        options.sort_unstable();
    }
    options
}

/// Moves the DPI one step through `dpi_options`, wrapping at both ends.
fn step_dpi(settings: &mut Settings, forward: bool) {
    let options = dpi_options(settings.mouse_dpi);
    let current = options
        .iter()
        .position(|dpi| *dpi == settings.mouse_dpi)
        .unwrap_or(0);
    let next = if forward {
        (current + 1) % options.len()
    } else {
        (current + options.len() - 1) % options.len()
    };
    if let Some(&dpi) = options.get(next) {
        settings.mouse_dpi = dpi;
    }
}

/// Radians of view rotation per pixel of pointer travel at sensitivity 1.00.
///
/// Calibrated so a sweep across a 1920-pixel-wide window turns 2 radians, which
/// is the feel the game shipped with — but it is a constant, not `2 / window
/// width`. Pointer travel is what the hand does: scaling it by the window would
/// make the same movement turn further in a small window and less in a large
/// one, and the cm/360 the settings row reports would be a lie at every size
/// but one.
const LOOK_RADIANS_PER_COUNT: f32 = 2.0 / 1920.0;

/// The distance a full turn takes on the desk, in centimetres, for a
/// sensitivity and a mouse DPI — the cm/360 players use to compare games.
///
/// A pixel of pointer travel turns `LOOK_RADIANS_PER_COUNT` radians at
/// sensitivity 1.00, a mouse pixel is 1/DPI of an inch, and an inch is 2.54 cm:
/// the whole turn is `360 × 2.54 ÷ (degrees per pixel × pixels per centimetre)`.
/// Both arguments are guarded because the row draws this every frame, and a NaN
/// on screen is worse than a large number.
fn cm_per_360(sensitivity: f32, dpi: u32) -> f32 {
    let deg_per_count = sensitivity.max(0.01) * LOOK_RADIANS_PER_COUNT.to_degrees();
    360.0 * 2.54 / (deg_per_count * dpi.max(1) as f32)
}

const ARMOR_MAX: f32 = 50.0;

// --- the field manual -----------------------------------------------------

/// The manual's pages, in order: what to press and what to do, the hostiles,
/// and the weapons and kit. Every page number that arrives from a key binding
/// or a menu row is wrapped into this range by `wrap_page`.
const MANUAL_PAGES: usize = 3;

/// One built page of the manual.
///
/// `codex::*` formats the stat lines and builds the portrait meshes every time
/// it is called, so a page is built when it opens or is turned to and held on
/// `Game` until the manual shuts. Drawing only borrows it: this is the one
/// screen with a cache, because rebuilding a page in the draw path would
/// allocate and rebuild meshes sixty times a second.
enum ManualContent {
    /// Page 0: the keys, grouped, with the survival notes under them.
    Controls {
        sections: Vec<HelpSection>,
        tips: Vec<&'static str>,
    },
    /// The card pages, under their own title.
    Cards { title: Key, rows: Vec<CodexRow> },
}

/// Wraps a page number into the manual, so an index from a key binding or a
/// menu row can never point past the last page.
fn wrap_page(page: usize) -> usize {
    page % MANUAL_PAGES
}

/// The page a `←`/`→` lands on. The pager is a ring: `→` off the last page
/// comes back to the first, and `←` off the first goes to the last.
fn flip_page(page: usize, turn: isize) -> usize {
    (wrap_page(page) as isize + turn).rem_euclid(MANUAL_PAGES as isize) as usize
}

/// Page 2's rows: the armoury, then the rest of the kit. `draw_manual_cards`
/// draws one table per call, so the two lists are appended into one, weapons
/// first because they are what the page is opened to read.
fn weapons_and_kit() -> Vec<CodexRow> {
    let mut rows = codex::weapons();
    rows.extend(codex::kit());
    rows
}

/// Builds one page of the manual: the whole "page number → which table is
/// drawn" decision, in the one place the field and the menus both reach.
fn manual_content(page: usize) -> ManualContent {
    match wrap_page(page) {
        1 => ManualContent::Cards {
            title: Key::ManualTitleEnemies,
            rows: codex::enemies(),
        },
        2 => ManualContent::Cards {
            title: Key::ManualTitleKit,
            rows: weapons_and_kit(),
        },
        _ => ManualContent::Controls {
            sections: codex::controls(),
            tips: codex::tips(),
        },
    }
}

/// `H` and `K`: the page an opening key asks for, or `None` when neither is
/// down. The same two keys open the manual in the field and over a menu.
fn manual_jump_key() -> Option<usize> {
    if is_key_pressed(KeyCode::H) {
        Some(0)
    } else if is_key_pressed(KeyCode::K) {
        Some(1)
    } else {
        None
    }
}

/// `←`/`→`: which way the page keys turn the manual, `0` for neither.
fn manual_turn_key() -> isize {
    isize::from(is_key_pressed(KeyCode::Right)) - isize::from(is_key_pressed(KeyCode::Left))
}

/// The manual after this frame's keys: whether it is showing, and the page it
/// is on.
///
/// `was_open`/`page` are the state it was in, `jump` the page an opening key or
/// a menu row asked for, `turn` which way `←`/`→` pushed, and `escape` whether
/// Esc was pressed. The field and the menus drive this same rule — in the field
/// the manual is `Overlay::Manual`, over a menu it is `Game::menu_manual`,
/// because a menu has no world for an overlay to suspend — so its keys mean the
/// same thing in both, and the transition is a pure function of its inputs that
/// a test can drive without a window.
fn manual_after_keys(
    was_open: bool,
    page: usize,
    jump: Option<usize>,
    turn: isize,
    escape: bool,
) -> (bool, usize) {
    if escape {
        return (false, page);
    }
    if let Some(page) = jump {
        return (true, wrap_page(page));
    }
    if !was_open {
        return (false, page);
    }
    (true, flip_page(page, turn))
}

/// Where one wave's releases come from.
struct WaveRelease {
    /// Points left to spend this wave.
    budget: u32,
    /// How many hostiles this wave has released — the floor bookkeeping.
    released: u32,
    /// The entrances this wave uses: a handful of points picked when the wave
    /// opens, so arrivals read as "from the east and the north" — a direction
    /// to hold — instead of noise from every corner (reference §19.2).
    points: Vec<Vec3>,
    cursor: usize,
}

/// What the spawner is doing right now.
///
/// The field used to be one continuous drip; the reference (§19.1) measured
/// that as the reason a run has no decisions in it — pressure that never
/// changes shape gives the player nothing to time against. Now the fight is
/// fought in waves, with a window between them.
enum WavePhase {
    /// Countdown to the next wave; the field is empty.
    Prep { left: f32 },
    /// The wave's points are being paid out, one release at a time, while
    /// the live count is under `max_alive`.
    Release(WaveRelease),
    /// The budget is spent; wait for the field to empty.
    Clear,
    /// Every wave released and the field is empty — the mission can clear.
    Finished,
}

impl Default for WavePhase {
    /// Only ever held for the span of one `tick_waves` call, which takes the
    /// phase out of the game so its arms can reach the rest of `self`.
    fn default() -> Self {
        WavePhase::Finished
    }
}

struct Game {
    phase: Phase,
    overlay: Overlay,
    level: Level,
    player: Player,
    arsenal: Arsenal,
    /// The player's armory: what each weapon kind is built and painted as.
    kit: Kit,
    viewmodel: ViewModel,
    grenades: Grenades,
    enemies: Vec<Enemy>,
    pickups: Vec<Pickup>,
    effects: effects::Effects,
    /// The fog material, or `None` if the driver refused the shader.
    fog: Option<fog::Fog>,
    /// The six world textures. Held here rather than in `Level` because they
    /// outlive any one level — a mission switch rebuilds the level, not the
    /// GPU uploads.
    textures: surface::Textures,
    /// Reused by `render_world` for every per-frame mesh. `draw_mesh` copies
    /// the vertices into macroquad's own batch the moment it is called, so one
    /// buffer serves the whole frame — and, kept here rather than built per
    /// frame, it holds on to the capacity it last needed.
    scratch: Mesh,
    /// The same idea for the damage readouts: rebuilt every frame from the
    /// effects list, so a firefight does not allocate a vector per frame for
    /// labels that expire in three quarters of a second.
    damage_labels: Vec<(Vec2, f32, bool)>,
    mission: &'static Mission,
    mission_index: usize,
    rng: Rng,

    armor: f32,
    stance: Stance,
    ammo_grenades: [u32; 2],
    ammo_meds: [u32; 3],
    wheel: Option<Wheel>,
    /// Previous pointer position in window pixels, owned here so that
    /// recentring the pointer does not contaminate the delta.
    last_mouse: Vec2,
    mouse_ready: bool,
    /// True once the fire button has been released since the mission began.
    trigger_armed: bool,
    /// Latched state for toggle-style aiming.
    aim_on: bool,
    /// Sprint, as a mode rather than a held key. See `sprint_state`.
    sprint: bool,
    /// Commanded lean, -1 left .. +1 right. In toggle mode this is latched.
    lean_cmd: f32,
    heal_t: f32,
    heal_index: usize,

    spawned: u32,
    killed: u32,
    score: u32,
    shots_fired: u32,
    shots_hit: u32,
    elapsed: f32,
    spawn_timer: f32,
    /// Waves finished this mission. The spawner reads it; the HUD shows
    /// `waves_done + 1` as the wave in progress.
    waves_done: u32,
    /// What the spawner is doing right now.
    wave_phase: WavePhase,
    /// The last wave that released an elite, if any — the buffer rule of
    /// reference §19.2 insists on, so an elite cannot arrive two waves in a
    /// row.
    last_elite_wave: Option<u32>,
    /// This mission's saved best score, and whether the run that just ended
    /// beat it. Filled in by `settle_score`, read by the result screen.
    result_best: u32,
    result_new_best: bool,
    /// Seconds until the next mote of suspended matter is spawned in a drowned
    /// arena. Unused — and never ticked — anywhere else.
    mote_t: f32,
    flow_t: f32,
    survive_left: f32,
    objective_done: bool,
    extraction_open: bool,
    dead_timer: f32,
    /// Seconds of hit-stop left after a kill. While this is positive the world
    /// is advanced at `HITSTOP_SCALE`, and the timers that belong to the
    /// simulation (this one included) keep the real clock so the pause cannot
    /// extend itself.
    hitstop: f32,

    banner: Option<(String, f32)>,
    toast: Option<(String, f32)>,
    unlocked: usize,
    /// The mission the player is in, mirrored into the settings file so that
    /// `continue` can put them back here.
    last_mission: usize,
    /// The best score on each mission, loaded from the file and written back
    /// through `config_of`. The reason a score outlives its run.
    best: [u32; campaign::COUNT],
    menu_sel: usize,
    result_sel: usize,
    settings_sel: usize,
    /// The armory's screen state: which weapon kind and tab are selected, the
    /// row cursor, and the preview — rebuilt only when a choice moves.
    armory_kind: usize,
    armory_tab: usize,
    armory_sel: usize,
    armory_mesh: Option<Mesh>,
    armory_dirty: bool,
    /// The bench's own state: which column the keyboard is in, the slot it is
    /// pointed at, the candidate its caret is on — remembered with the slot it
    /// was placed in, so a slot's list opens where the player left it — the
    /// candidate under the pointer, and the build that candidate would make.
    /// All five are dead on the paint and loadout tabs, which are row lists.
    armory_focus: hud::ArmoryColumn,
    armory_slot: usize,
    armory_part: Option<(weapon::SlotId, usize)>,
    armory_hover_part: Option<usize>,
    armory_preview: Option<ArmoryPreview>,
    /// Where the cursor was on the previous frame, so a menu can tell "the
    /// mouse moved" from "the cursor happens to be sitting on a row".
    menu_mouse: Vec2,
    settings: Settings,
    /// The window mode the OS has actually been told about. Kept so a settings
    /// change pushes only what moved, and so leaving fullscreen re-applies the
    /// size the file holds — see `window_calls`.
    window_state: WindowState,
    /// The manual's page, 0-based and wrapped by `wrap_page`. One number for
    /// both places it opens, so its keys and the cache below mean the same
    /// thing wherever the page was opened from.
    manual_page: usize,
    /// The page's built content, or `None` while the manual is shut. Built on
    /// open and on every turn, never in the draw path: see `ManualContent`.
    manual_cache: Option<ManualContent>,
    /// Whether the manual is open over a menu. The field's manual is
    /// `Overlay::Manual` — an overlay is what freezes the world behind it, and
    /// the main menu and the pause screen have no world to freeze — and
    /// overloading that with a second meaning would overload its Esc handler
    /// too, which for an overlay means "back to the field". `phase` does not
    /// move while the manual is open over a menu, so closing it has nothing to
    /// restore: the menu underneath is still the one it was opened from.
    menu_manual: bool,
}

impl Game {
    /// `settings` and `kit` come from the file. The settings are what the
    /// window was built with — `window_conf` read the same file before the
    /// window existed — and the kit is the armory every carried weapon is
    /// built from. `best` is the saved score per mission, the same one file.
    fn new(
        settings: Settings,
        kit: Kit,
        unlocked: usize,
        last_mission: usize,
        best: [u32; campaign::COUNT],
    ) -> Self {
        let mission = campaign::get(0);
        let mut level = Level::from_layout(&maps::campaign()[mission.layout]);
        // Only once a window exists: this uploads textures and fixes up their
        // sampling, neither of which works from a headless test.
        let textures = surface::Textures::load();
        level.dress(&textures);
        let player = Player::new(level.player_start);
        let arsenal = Arsenal::with_builds([kit.primary, kit.secondary], &kit.builds);
        let grenades = kit.grenades;
        let meds = kit.meds;
        let survive_left = mission.survive_seconds;
        Self {
            phase: Phase::Menu,
            overlay: Overlay::None,
            level,
            player,
            arsenal,
            kit,
            viewmodel: ViewModel::new(),
            grenades: Grenades::new(),
            enemies: Vec::new(),
            pickups: Vec::new(),
            effects: effects::Effects::new(),
            textures,
            fog: fog::Fog::load(),
            scratch: mesh::empty_mesh(),
            damage_labels: Vec::new(),
            mission,
            mission_index: 0,
            rng: Rng::new(0x9E37_79B9),
            armor: 0.0,
            stance: Stance::Stand,
            ammo_grenades: grenades,
            ammo_meds: meds,
            wheel: None,
            last_mouse: Vec2::ZERO,
            mouse_ready: false,
            trigger_armed: false,
            aim_on: false,
            sprint: false,
            lean_cmd: 0.0,
            heal_t: 0.0,
            heal_index: 0,
            spawned: 0,
            killed: 0,
            score: 0,
            shots_fired: 0,
            shots_hit: 0,
            elapsed: 0.0,
            spawn_timer: 0.0,
            waves_done: 0,
            wave_phase: WavePhase::Finished,
            last_elite_wave: None,
            result_best: 0,
            result_new_best: false,
            mote_t: 0.0,
            flow_t: 0.0,
            survive_left,
            objective_done: false,
            extraction_open: false,
            dead_timer: 0.0,
            hitstop: 0.0,
            banner: None,
            toast: None,
            // Clamped here rather than trusted: the file is hand-editable and
            // the campaign is shorter than the file's range allows.
            unlocked: unlocked.clamp(1, campaign::count()),
            last_mission: last_mission.min(campaign::count() - 1),
            best,
            menu_sel: 0,
            result_sel: 0,
            settings_sel: 0,
            armory_kind: 0,
            armory_tab: 0,
            armory_sel: 0,
            armory_mesh: None,
            armory_dirty: true,
            armory_focus: hud::ArmoryColumn::Slots,
            armory_slot: 0,
            armory_part: None,
            armory_hover_part: None,
            armory_preview: None,
            menu_mouse: Vec2::ZERO,
            settings,
            window_state: WindowState {
                fullscreen: settings.fullscreen,
                width: settings.width,
                height: settings.height,
            },
            manual_page: 0,
            manual_cache: None,
            menu_manual: false,
        }
    }

    fn start(&mut self, index: usize) {
        self.mission_index = index.min(campaign::count() - 1);
        // The file is written on every start, so `continue` resumes the mission
        // the player was actually in rather than the furthest they reached.
        self.last_mission = self.mission_index;
        save_progress(self);
        self.mission = campaign::get(self.mission_index);
        let layouts = maps::campaign();
        self.level = Level::from_layout(&layouts[self.mission.layout]);
        // A retry or the next mission rebuilds the geometry; the textures are
        // already uploaded, so only the handle has to be reattached.
        self.level.dress(&self.textures);
        self.player = Player::new(self.level.player_start);
        self.player.alive = true;
        self.arsenal =
            Arsenal::with_builds([self.kit.primary, self.kit.secondary], &self.kit.builds);
        // Stock what the stash packed: each long gun's reserve is its spare
        // magazines, the sidearm always carries two magazines' worth, and the
        // consumables count out of the kit.
        for slot in 0..2 {
            let mags = self.kit.ammo_mags[slot];
            if let Some(w) = self.arsenal.slot_mut(slot) {
                let mag_size = w.stats().mag_size;
                w.reserve = mags * mag_size;
            }
        }
        if let Some(w) = self.arsenal.slot_mut(2) {
            let mag_size = w.stats().mag_size;
            w.reserve = SIDEARM_MAGS * mag_size;
        }
        self.grenades = Grenades::new();
        self.enemies.clear();
        self.pickups.clear();
        self.effects.clear();
        self.armor = 0.0;
        self.stance = Stance::Stand;
        // You go in with what the stash packed; there is no resupply.
        self.ammo_grenades = self.kit.grenades;
        self.ammo_meds = self.kit.meds;
        self.wheel = None;
        self.mouse_ready = false;
        self.trigger_armed = false;
        self.aim_on = false;
        self.sprint = false;
        self.lean_cmd = 0.0;
        self.heal_t = 0.0;
        self.overlay = Overlay::None;
        self.manual_page = 0;
        self.manual_cache = None;
        self.menu_manual = false;
        self.spawned = 0;
        self.killed = 0;
        self.score = 0;
        self.shots_fired = 0;
        self.shots_hit = 0;
        self.elapsed = 0.0;
        // The first wave is a prep window like any other: the map-name banner
        // owns its opening seconds and then the countdown takes over.
        self.spawn_timer = 0.0;
        self.waves_done = 0;
        self.wave_phase = WavePhase::Prep {
            left: wave_gap(self.mission),
        };
        self.last_elite_wave = None;
        self.result_best = self.best[self.mission_index];
        self.result_new_best = false;
        self.mote_t = 0.0;
        self.flow_t = 0.0;
        self.survive_left = self.mission.survive_seconds;
        self.objective_done = false;
        self.extraction_open = false;
        self.dead_timer = 0.0;
        self.hitstop = 0.0;
        self.result_sel = 0;

        let s = self.level.player_start;
        self.pickups.push(Pickup::new(
            vec3(s.x + 2.0, 0.0, s.z + 1.0),
            PickupKind::Ammo,
            40.0,
        ));
        self.pickups.push(Pickup::new(
            vec3(s.x - 2.0, 0.0, s.z + 1.5),
            PickupKind::Health,
            25.0,
        ));

        self.banner = Some((level_name(&self.level), 2.8));
        self.phase = Phase::Playing;
        // Re-assert in case a focus change slipped in since startup.
        disable_ime();
    }

    fn alive_enemies(&self) -> usize {
        self.enemies.iter().filter(|e| !e.dead).count()
    }

    /// Hostiles the mission still owes the field: everyone alive, plus a
    /// rough count of the points still to be released — this wave's unspent
    /// budget, or the next wave's whole budget while a prep window runs.
    /// Points are not bodies, so the average roll is the divisor and the
    /// number is an estimate by construction; rounding up keeps it from ever
    /// reading zero while something is still coming.
    fn remaining(&self) -> u32 {
        let points = match &self.wave_phase {
            WavePhase::Prep { .. } => {
                campaign::wave_budget(self.mission, self.waves_done.min(self.mission.waves - 1))
            }
            WavePhase::Release(rel) => rel.budget,
            WavePhase::Clear | WavePhase::Finished => 0,
        };
        let per = campaign::mean_cost(self.mission).max(1.0);
        self.alive_enemies() as u32 + (points as f32 / per).ceil() as u32
    }

    /// The wave number to show the player, one-based: the wave in progress
    /// (or being prepared for), held at the last wave after the last one.
    fn wave_display(&self) -> u32 {
        (self.waves_done + 1).min(self.mission.waves)
    }

    /// Files the score of the run that just ended: keeps the saved best up to
    /// date, writes it out, and remembers what the result screen has to say.
    /// Called from the two places a run ends — cleared or killed. Without
    /// this the score is a number the game forgets (reference §19.3).
    fn settle_score(&mut self) {
        let i = self.mission_index.min(campaign::COUNT - 1);
        self.result_new_best = self.score > self.best[i];
        if self.result_new_best {
            self.best[i] = self.score;
            save_progress(self);
        }
        self.result_best = self.best[i];
    }

    /// One tick of the wave clock.
    ///
    /// The phase is taken out of `self` for the span of the tick so each arm
    /// can reach into the rest of the game; every path hands a phase back
    /// before this returns.
    fn tick_waves(&mut self, dt: f32, audio: &audio::Audio) {
        let phase = std::mem::take(&mut self.wave_phase);
        let phase = match phase {
            WavePhase::Prep { mut left } => {
                left -= dt;
                if is_key_pressed(KeyCode::F) {
                    left = 0.0;
                }
                // The mission's first prep starts behind the map-name banner;
                // once that leaves, the countdown banner takes the slot.
                let gap = wave_gap(self.mission);
                if left <= gap - 2.6 && self.banner.is_none() {
                    self.banner = Some((
                        format!(
                            "{} {} / {}",
                            i18n::t(Key::Wave),
                            self.waves_done + 1,
                            self.mission.waves
                        ),
                        left.max(0.3),
                    ));
                }
                if left <= 0.0 {
                    self.begin_wave()
                } else {
                    WavePhase::Prep { left }
                }
            }
            WavePhase::Release(mut rel) => {
                self.spawn_timer -= dt;
                if self.alive_enemies() < self.mission.max_alive as usize && self.spawn_timer <= 0.0
                {
                    if self.release_one(&mut rel, audio) {
                        self.spawn_timer = (1.9 - 0.06 * self.mission.difficulty).max(0.7);
                        WavePhase::Release(rel)
                    } else if self.alive_enemies() == 0 {
                        // The last release is already down: no clear needed.
                        self.finish_wave()
                    } else {
                        WavePhase::Clear
                    }
                } else {
                    WavePhase::Release(rel)
                }
            }
            WavePhase::Clear => {
                if self.alive_enemies() == 0 {
                    self.finish_wave()
                } else {
                    WavePhase::Clear
                }
            }
            WavePhase::Finished => WavePhase::Finished,
        };
        self.wave_phase = phase;
    }

    /// Opens the next wave: the reset, the entrances it will use, and the
    /// first release's delay.
    fn begin_wave(&mut self) -> WavePhase {
        let mission = self.mission;
        let wave = self.waves_done;
        // §19.4-5: a wave starts as a clean duel. Health and breath come
        // back, so the deaths this buys are deaths of position and decision,
        // not of the last wave's leftovers. Ammunition is deliberately *not*
        // refilled — that economy is the stash's — and neither are armour or
        // sanity: those are things the arena is allowed to take from you.
        self.player.wave_reset();
        self.heal_t = 0.0;
        self.spawn_timer = 0.45;
        let points = spawn_set(&self.level, &self.player, &mut self.rng, WAVE_ENTRANCES);
        WavePhase::Release(WaveRelease {
            budget: campaign::wave_budget(mission, wave),
            released: 0,
            points,
            cursor: 0,
        })
    }

    /// Closes the wave that just ended: a prep window for the next one, or
    /// `Finished` if that was the last.
    fn finish_wave(&mut self) -> WavePhase {
        self.waves_done += 1;
        if self.waves_done >= self.mission.waves {
            return WavePhase::Finished;
        }
        let gap = wave_gap(self.mission);
        if !matches!(self.mission.objective, Objective::Survive) {
            self.say(i18n::t(Key::WaveEarly));
        }
        self.banner = Some((
            format!(
                "{} {} / {}",
                i18n::t(Key::Wave),
                self.waves_done + 1,
                self.mission.waves
            ),
            gap,
        ));
        WavePhase::Prep { left: gap }
    }

    /// Releases one hostile for the current wave, if the wave's remaining
    /// points still afford one. `false` means the budget is spent.
    fn release_one(&mut self, rel: &mut WaveRelease, audio: &audio::Audio) -> bool {
        let mission = self.mission;
        let wave = self.waves_done;
        let floor_left = campaign::WAVE_FLOOR.saturating_sub(rel.released);
        let elite_ready = self.last_elite_wave.is_none_or(|l| wave >= l + 2);
        let kind = {
            let mut rolls = || self.rng.u32();
            campaign::draw_kind(mission, &mut rolls, rel.budget, floor_left, elite_ready)
        };
        let Some(kind) = kind else {
            return false;
        };
        rel.budget -= campaign::cost(kind);
        rel.released += 1;
        if campaign::is_elite(kind) {
            self.last_elite_wave = Some(wave);
            // You should know something enormous has arrived before it sees you.
            self.banner = Some((i18n::t(Key::ColossusStirs).to_string(), 2.6));
        }
        let pos = wave_point(rel, &self.player);
        let mut e = Enemy::spawn(
            kind,
            pos,
            campaign::wave_strength(mission, wave),
            &mut self.rng,
        );
        // Spawned out of sight and never having seen anything, a
        // reinforcement would otherwise stand at its post forever.
        e.alert_to(self.player.pos, 20.0);
        self.enemies.push(e);
        self.spawned += 1;
        self.effects.smoke(
            pos + vec3(0.0, 1.0, 0.0),
            Color::new(0.5, 0.8, 1.0, 0.5),
            10,
            &mut self.rng,
        );
        audio.play_at(Sfx::Spawn, 12.0);
        true
    }

    /// Returns to a playable state, swallowing whatever input the menu left
    /// behind.
    ///
    /// Two separate leaks, same cause. The click that pressed RESUME is never
    /// seen by the re-arm check in `simulate`, so the button is still down on
    /// the first frame back and the weapon fires. And the mouse baseline is
    /// stale — the pointer moved freely while the menu was up — so the first
    /// frame back applies the whole accumulated displacement as one turn.
    fn resume_play(&mut self) {
        self.phase = Phase::Playing;
        self.trigger_armed = false;
        self.mouse_ready = false;
        // Latched input is dropped on the way back in, for the same reason the
        // trigger is: whatever the player was holding when they opened the menu
        // is not something they are still asking for now.
        self.sprint = false;
    }

    fn say(&mut self, text: impl Into<String>) {
        self.toast = Some((text.into(), 1.6));
    }

    /// Starts a healing channel for the given medical item.
    fn begin_heal(&mut self, index: usize, audio: &audio::Audio) {
        if index >= MED_KINDS.len() || self.ammo_meds[index] == 0 {
            self.say(i18n::t(Key::NoMeds));
            audio.play(Sfx::UiBack);
            return;
        }
        if self.player.health >= self.player.max_health {
            self.say(i18n::t(Key::HealthFull));
            audio.play(Sfx::UiBack);
            return;
        }
        if self.heal_t > 0.0 {
            return;
        }
        self.heal_index = index;
        self.heal_t = MED_KINDS[index].1;
        self.say(format!(
            "{} — {}",
            i18n::t(Key::Healing),
            i18n::t(med_key(index))
        ));
        audio.play(Sfx::Reload);
    }

    fn throw_grenade(&mut self, index: usize, audio: &audio::Audio) {
        if index >= GRENADE_KINDS.len() || self.ammo_grenades[index] == 0 {
            self.say(i18n::t(Key::NoGrenades));
            audio.play(Sfx::UiBack);
            return;
        }
        let eye = self.player.eye();
        let dir = self.player.look_dir();
        self.grenades.throw(GRENADE_KINDS[index], eye, dir);
        self.ammo_grenades[index] -= 1;
        self.viewmodel.trigger_grenade_throw();
        audio.play(Sfx::UiMove);
    }

    fn select_slot(&mut self, i: usize) {
        self.arsenal.select(i);
    }

    /// Begins a campaign from the top, with the manual's hint armed again: a
    /// player who opens the menu to start a new campaign is a player the manual
    /// has not introduced itself to yet. Retries and mission select go to
    /// `start` directly, so the hint cannot return on every mission.
    fn new_campaign(&mut self) {
        self.start(0);
    }

    /// Shows page `page` of the manual, wrapped into range, and builds it.
    ///
    /// The page and its built rows are only ever set here, together, so the
    /// cache can never be a page behind what is drawn — and both the field's
    /// keys and the menu's rows come through it rather than building their own.
    fn show_manual_page(&mut self, page: usize) {
        self.manual_page = wrap_page(page);
        self.manual_cache = Some(manual_content(self.manual_page));
    }

    /// Opens the manual over a menu on `page`, for the menu rows and the H and
    /// K keys: the page is built and the menu is flagged as the surface
    /// underneath, which is what Esc drops back to.
    fn open_menu_manual(&mut self, page: usize) {
        self.show_manual_page(page);
        self.menu_manual = true;
    }

    /// Shuts the manual and drops the page it had built. Clears both of the
    /// places it can be open — the field's overlay slot and the menu flag — one
    /// of which is already clear wherever this is called; the rows are built
    /// again the next time it opens.
    fn close_manual(&mut self) {
        if self.overlay == Overlay::Manual {
            self.overlay = Overlay::None;
        }
        self.menu_manual = false;
        self.manual_cache = None;
    }
}

fn level_name(level: &Level) -> String {
    match i18n::lang() {
        i18n::Lang::Zh => level.name_zh.to_string(),
        i18n::Lang::En => level.name_en.to_string(),
    }
}

/// How far forward the player has to be pushing for a sprint to keep itself on.
const SPRINT_FORWARD: f32 = 0.4;

/// Hit-stop: how long a kill slows the world, and what fraction of real time
/// the world gets while it lasts. Short enough to read as impact rather than as
/// a stutter, and never a full stop — a frozen frame in an FPS reads as a
/// hitch, not as weight.
const HITSTOP_TIME: f32 = 0.05;
const HITSTOP_SCALE: f32 = 0.25;

/// How far a tracer is kept away from the camera plane, in metres.
///
/// A beam has a ring of vertices at each end, and a ring that lands on the
/// camera plane projects through the near plane and smears into a wedge across
/// half the screen. The player's own shots start at the eye, and an enemy shot
/// that connects ends at it, so both ends have to be pushed out by this much.
const TRACER_NEAR: f32 = 0.55;

/// Whether the player is sprinting this frame.
///
/// Sprint is a mode rather than a held key: Shift latches it on and it drops
/// the moment the player stops asking to run — forward released, or sights up.
/// Holding the key is worse than it sounds on this keyboard, because Shift is
/// also the IME's language toggle (see `disable_ime`) and the shortcut Windows
/// uses to offer Sticky Keys, so a key that only has to be tapped is a key that
/// stops fighting both of them.
fn sprint_state(on: bool, toggled: bool, forward: f32, aiming: bool) -> bool {
    let latched = if toggled { !on } else { on };
    latched && forward > SPRINT_FORWARD && !aiming
}

/// Stop the Windows IME from eating the game's keys.
///
/// On a Chinese Windows install Left Shift is the IME's default
/// Chinese/English toggle, and Shift is sprint here — so every sprint flipped
/// the input method and the IME started swallowing WASD. Games solve this by
/// detaching the IME context from their window; miniquad exposes exactly that
/// as `set_ime_enabled(false)`, which calls
/// `ImmAssociateContextEx(hwnd, NULL, 0)` and sets a flag it re-checks on
/// `WM_SETFOCUS` / `WM_IME_SETCONTEXT`, so the setting survives focus changes.
///
/// There is no text input in this game, so the IME stays off for the whole
/// session. A no-op off Windows.
fn disable_ime() {
    macroquad::miniquad::window::set_ime_enabled(false);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Sprint had to become a mode rather than a held key, and the parts that
    /// matter are all in the transitions: a tap has to latch, a second tap has
    /// to drop it, and letting go of forward has to drop it too — otherwise you
    /// walk away from the keyboard still flagged as running.
    #[test]
    fn sprint_latches_and_releases_itself() {
        // A tap latches it, and it stays latched while running.
        assert!(sprint_state(false, true, 1.0, false));
        assert!(sprint_state(true, false, 1.0, false));
        // A second tap drops it.
        assert!(!sprint_state(true, true, 1.0, false));
        // Releasing forward drops it, and it stays dropped: the tap was a
        // command to run *now*, not a mode that waits for the next step.
        assert!(!sprint_state(true, false, 0.0, false));
        // Nor does it survive the sights coming up.
        assert!(!sprint_state(true, false, 1.0, true));
        // Backing up is not running.
        assert!(!sprint_state(true, false, -1.0, false));
        // A tap while standing still has nothing to attach to.
        assert!(!sprint_state(false, true, 0.0, false));
    }

    /// The damage readouts are the only thing the game projects from world
    /// space back into pixels, and a mirrored or shifted projection would pass
    /// a test that only checked for a finite number. A point on the ground five
    /// metres dead ahead has to land on the centre line and below the middle,
    /// at the angle the camera's own fov puts it; the player's right has to
    /// come out right; and a point behind the camera has to project to nothing.
    #[test]
    fn the_projection_puts_the_ground_ahead_below_the_crosshair() {
        let (w, h) = (1280.0f32, 720.0f32);
        let fovy = 62.0f32.to_radians();
        let eye = vec3(0.0, 1.8, 0.0);
        let vp = view_projection(eye, eye + Vec3::NEG_Z, Vec3::Y, fovy, w, h);

        let ground = project_to_screen(&vp, vec3(0.0, 0.0, -5.0), w, h).expect("in front");
        assert!(
            (ground.x - w * 0.5).abs() < 0.5,
            "not on the centre line: {ground:?}"
        );
        // Eye 1.8 m up, 5 m out: the point sits atan(1.8 / 5) below the horizon
        // and the camera puts `f * tan(angle)` of that down the screen.
        let f = (h * 0.5) / (fovy * 0.5).tan();
        let want = h * 0.5 + f * (1.8 / 5.0);
        assert!(
            (ground.y - want).abs() < 1.0,
            "ground point at {ground:?}, expected y {want}"
        );

        // Five metres ahead and 2.5 m to the player's right: right of centre.
        // Straight out to the side would be culled, correctly — it sits on the
        // eye plane, where the perspective divide has no answer.
        let right = project_to_screen(&vp, vec3(2.5, 1.8, -5.0), w, h).expect("in front");
        assert!(right.x > w * 0.5, "the player's right came out left");

        assert!(
            project_to_screen(&vp, vec3(0.0, 1.8, 5.0), w, h).is_none(),
            "a point behind the camera was projected anyway"
        );
    }

    /// The manual is three pages behind two keys and a pair of arrows, and the
    /// pages are a ring: `→` off the last page has to come back to the first
    /// and `←` off the first to the last, or a page is unreachable from one
    /// side. Page numbers that arrive from a binding are wrapped, not trusted.
    #[test]
    fn the_manual_pages_wrap_both_ways() {
        assert_eq!(flip_page(0, 1), 1);
        assert_eq!(flip_page(1, 1), 2);
        assert_eq!(flip_page(2, 1), 0);
        assert_eq!(flip_page(0, -1), MANUAL_PAGES - 1);
        assert_eq!(flip_page(2, -1), 1);
        // Turning one way and back lands on the page it started from, and a
        // turn of zero is not a turn.
        for page in 0..MANUAL_PAGES {
            assert_eq!(flip_page(flip_page(page, 1), -1), page);
            assert_eq!(flip_page(page, 0), page);
        }
        // Every page a key can ask for is inside the manual.
        assert_eq!(wrap_page(0), 0);
        assert_eq!(wrap_page(MANUAL_PAGES - 1), MANUAL_PAGES - 1);
        assert_eq!(wrap_page(MANUAL_PAGES), 0);
        assert_eq!(wrap_page(2 * MANUAL_PAGES + 2), 2);
    }

    /// The menus answer to the keyboard and the mouse at once, and this is the
    /// rule that keeps them from fighting: a hover only takes the selection when
    /// the cursor has just moved, so a mouse parked over a row cannot drag the
    /// highlight back while the arrow keys are being used.
    #[test]
    fn a_parked_mouse_does_not_take_the_selection_back() {
        assert_eq!(hover_selection(3, Some(0), false), 3);
        assert_eq!(hover_selection(3, Some(0), true), 0);
        assert_eq!(hover_selection(3, None, true), 3);
        assert_eq!(hover_selection(0, None, false), 0);
        // Out of range is the caller's business: it only ever passes a row it
        // found in the rect list.
        assert_eq!(hover_selection(2, Some(9), true), 9);
    }

    /// `continue` picks up where the player was, held inside what they have
    /// unlocked and inside the campaign. The file is hand-editable, so a
    /// nonsense value must not send them to a mission that does not exist.
    #[test]
    fn continue_resumes_where_the_player_was() {
        assert_eq!(resume_mission(1, 0), 0);
        assert_eq!(resume_mission(2, 1), 1);
        assert_eq!(resume_mission(6, 3), 3);
        // Ahead of the unlocks, or past the end of the campaign: clamped, never
        // off the table.
        assert_eq!(resume_mission(2, 5), 1);
        assert_eq!(resume_mission(99, 99), campaign::count() - 1);
        // A file that says nothing sensible still produces a real mission.
        assert_eq!(resume_mission(0, 0), 0);
    }

    /// The armour split, §17.2-style: the bar takes 65% of a body hit while
    /// it holds, takes nothing when the hit lands on the legs, and can never
    /// absorb more than it has left.
    #[test]
    fn armour_takes_the_body_never_the_legs() {
        let (absorbed, through) = armor_split(20.0, 50.0, false);
        assert!((absorbed - 13.0).abs() < 1e-4);
        assert!((through - 7.0).abs() < 1e-4);

        let (absorbed, through) = armor_split(20.0, 50.0, true);
        assert_eq!(absorbed, 0.0, "legs are never armoured");
        assert!((through - 20.0).abs() < 1e-4);

        // The bar absorbs what it has and lets the rest through.
        let (absorbed, through) = armor_split(20.0, 5.0, false);
        assert!((absorbed - 5.0).abs() < 1e-4);
        assert!((through - 15.0).abs() < 1e-4);
    }

    /// A ray that lands below the hips is a leg hit; the fraction is of the
    /// live stance's height, so crouching drops the line with the body.
    #[test]
    fn hits_below_the_hips_strike_the_legs() {
        let player = Player::new(Vec3::ZERO);
        let h = player.body_height();
        assert!(struck_legs(&player, vec3(0.0, 0.2 * h, 0.0)));
        assert!(struck_legs(&player, vec3(0.0, -0.5, 0.0)));
        assert!(!struck_legs(&player, vec3(0.0, 0.8 * h, 0.0)));
        assert!(!struck_legs(&player, vec3(0.0, h, 0.0)));
    }

    /// Which table a page draws is the manual's whole model: the menu's rows
    /// and the field's keys both name a page by number, so the two have to
    /// agree on what page 1 is. Page 2 is one card list made of two tables, the
    /// weapons first and the rest of the kit after them.
    #[test]
    fn each_manual_page_draws_its_own_table() {
        assert!(
            matches!(manual_content(0), ManualContent::Controls { .. }),
            "page 0 is the controls and the survival notes"
        );

        match manual_content(1) {
            ManualContent::Cards { title, rows } => {
                assert_eq!(title, Key::ManualTitleEnemies);
                assert_eq!(rows.len(), codex::enemies().len());
            }
            ManualContent::Controls { .. } => panic!("page 1 is the bestiary"),
        }

        match manual_content(2) {
            ManualContent::Cards { title, rows } => {
                assert_eq!(title, Key::ManualTitleKit);
                // Nothing may be dropped in the joining.
                assert_eq!(rows.len(), codex::weapons().len() + codex::kit().len());
            }
            ManualContent::Controls { .. } => panic!("page 2 is the armoury"),
        }

        // A page number off the end of the manual is wrapped before it is
        // matched on, so it lands on a page rather than drawing nothing.
        assert!(
            matches!(manual_content(MANUAL_PAGES), ManualContent::Controls { .. }),
            "an out-of-range page was not wrapped"
        );
    }

    /// The manual's keys are one rule in two places: in the field it is an
    /// overlay over the frozen world, and over the menu and pause screens it is
    /// a flag, so H, K, `←`, `→` and Esc have to mean the same thing in both.
    /// The parts worth testing are the transitions — a page key must not be
    /// able to open a shut manual, and Esc has to shut it whatever else is
    /// pressed with it.
    #[test]
    fn the_manual_opens_turns_and_closes() {
        let open = |page| manual_after_keys(false, 0, Some(page), 0, false);
        let turn = |page, turn| manual_after_keys(true, page, None, turn, false);
        let esc = |page| manual_after_keys(true, page, None, 0, true);

        // H and K open it on the controls and the codex, whatever page it was
        // left on, and an opening key cannot point past the end of the manual.
        assert_eq!(open(0), (true, 0));
        assert_eq!(open(1), (true, 1));
        assert_eq!(manual_after_keys(false, 2, Some(0), 0, false), (true, 0));
        assert_eq!(open(MANUAL_PAGES + 1), (true, 1));

        // Left and right walk it, wrapping at both ends.
        assert_eq!(turn(0, 1), (true, 1));
        assert_eq!(turn(2, 1), (true, 0));
        assert_eq!(turn(0, -1), (true, MANUAL_PAGES - 1));
        // A frame with nothing pressed leaves it where it was.
        assert_eq!(turn(1, 0), (true, 1));
        assert_eq!(manual_after_keys(false, 1, None, 0, false), (false, 1));

        // A page key on its own cannot open it: only H and K — and the menu's
        // own rows — do that, and an arrow reaching a shut manual would be a
        // manual opening itself.
        assert_eq!(manual_after_keys(false, 0, None, 1, false), (false, 0));

        // Esc shuts it, and outranks whatever was pressed with it.
        assert_eq!(esc(2), (false, 2));
        assert_eq!(manual_after_keys(true, 1, Some(0), 1, true), (false, 1));
        // Shutting it keeps the page for the next open, unless an opening key
        // names another one.
        assert_eq!(manual_after_keys(false, 2, None, 0, false), (false, 2));
        assert_eq!(manual_after_keys(false, 2, Some(1), 0, false), (true, 1));
    }

    /// The settings page and the file hold the same settings, and the two
    /// conversions are the only place that is written down. A field that one of
    /// them drops is a setting that quietly stops being saved.
    ///
    /// The language is deliberately not asserted here: it belongs to `i18n`, and
    /// checking it would mean setting the process-wide language from a test
    /// binary that runs its tests in parallel with the ones in `codex` that do
    /// the same thing.
    #[test]
    fn settings_survive_the_config_conversion() {
        let live = Settings {
            sensitivity: 3.45,
            mouse_dpi: 1600,
            volume: 0.15,
            aim_toggle: false,
            lean_toggle: false,
            invert_y: true,
            show_fps: true,
            fullscreen: true,
            width: 1920,
            height: 1080,
        };
        // The armory rides the same conversion: one touched weapon proves the
        // strings are carried through untouched.
        let mut builds: [String; 7] = std::array::from_fn(|_| String::new());
        builds[0] = "optic=micro_dot@0.50,muzzle=compensator@0.25".to_string();
        let mut paints: [String; 7] = std::array::from_fn(|_| String::new());
        paints[0] = "abyss@0.32:481516".to_string();
        let config = live.to_config(3, 2, builds, paints);
        // The armory rides the same conversion: one touched weapon proves the
        // strings are carried through untouched.
        assert_eq!(
            config.builds[0],
            "optic=micro_dot@0.50,muzzle=compensator@0.25"
        );
        assert_eq!(config.paints[0], "abyss@0.32:481516");
        assert!(config.builds[1..].iter().all(|s| s.is_empty()));
        assert!(config.paints[1..].iter().all(|s| s.is_empty()));
        let back = Settings::from_config(&config);
        assert_eq!(back.sensitivity, live.sensitivity);
        assert_eq!(back.mouse_dpi, live.mouse_dpi);
        assert_eq!(back.volume, live.volume);
        assert_eq!(back.aim_toggle, live.aim_toggle);
        assert_eq!(back.lean_toggle, live.lean_toggle);
        assert_eq!(back.invert_y, live.invert_y);
        assert_eq!(back.show_fps, live.show_fps);
        assert_eq!(back.fullscreen, live.fullscreen);
        assert_eq!(back.width, live.width);
        assert_eq!(back.height, live.height);
    }

    /// A game with no settings file has to be the game that shipped, so the live
    /// settings built from a default config are the values the old
    /// `Settings::default` held, plus the window it opened with.
    #[test]
    fn the_default_config_builds_the_shipped_settings() {
        let live = Settings::from_config(&config::Config::default());
        assert_eq!(live.sensitivity, 1.60);
        assert_eq!(live.mouse_dpi, 800);
        assert_eq!(live.volume, 0.80);
        assert!(live.aim_toggle);
        assert!(live.lean_toggle);
        assert!(!live.invert_y);
        assert!(!live.show_fps);
        assert!(!live.fullscreen);
        assert_eq!((live.width, live.height), (1280, 720));
    }

    /// What the window is told, and — more to the point — what it is not. A
    /// frame that changes nothing must not send window requests, and fullscreen
    /// must defer the size rather than resize a borderless window to something
    /// smaller than the screen.
    #[test]
    fn a_settings_change_only_moves_the_window_when_it_has_to() {
        let windowed = |w, h| WindowState {
            fullscreen: false,
            width: w,
            height: h,
        };
        let full = |w, h| WindowState {
            fullscreen: true,
            width: w,
            height: h,
        };

        // Nothing changed: nothing to say.
        assert_eq!(
            window_calls(windowed(1280, 720), windowed(1280, 720)),
            (None, None)
        );

        // A new size while windowed is applied now.
        assert_eq!(
            window_calls(windowed(1920, 1080), windowed(1280, 720)),
            (None, Some((1920, 1080)))
        );

        // Going fullscreen leaves the size alone...
        assert_eq!(
            window_calls(full(1920, 1080), windowed(1920, 1080)),
            (Some(true), None)
        );
        // ...and a size picked while fullscreen is only recorded, not pushed.
        assert_eq!(
            window_calls(full(2560, 1440), full(1920, 1080)),
            (None, None)
        );
        // Coming back re-applies the size the file holds, even though the size
        // itself never changed: leaving fullscreen restores whatever the OS had.
        assert_eq!(
            window_calls(windowed(2560, 1440), full(2560, 1440)),
            (Some(false), Some((2560, 1440)))
        );
    }

    /// The resolution row walks a list that always holds the current size, so a
    /// size the presets do not name is shown and kept instead of being rewritten
    /// by the first arrow press.
    #[test]
    fn the_resolution_row_keeps_a_size_that_is_not_a_preset() {
        assert_eq!(
            resolution_options((1280, 720)),
            RESOLUTIONS.to_vec(),
            "a preset is already in the list"
        );
        assert_eq!(
            resolution_options((1366, 768)).last(),
            Some(&(1366, 768)),
            "an odd size is carried as an extra entry"
        );

        let mut settings = Settings::from_config(&config::Config::default());
        // Right walks the presets in order...
        step_resolution(&mut settings, true);
        assert_eq!((settings.width, settings.height), (1600, 900));
        step_resolution(&mut settings, false);
        assert_eq!((settings.width, settings.height), (1280, 720));
        // ...and left from the first entry wraps to the last.
        step_resolution(&mut settings, false);
        assert_eq!((settings.width, settings.height), (2560, 1440));
        // A size that is not a preset takes part in the walk like any other
        // entry, rather than being snapped the first time an arrow is pressed.
        settings.width = 1366;
        settings.height = 768;
        step_resolution(&mut settings, true);
        assert_eq!((settings.width, settings.height), (1280, 720));
    }

    /// The DPI row walks a list that always holds the current value, so a DPI
    /// the presets do not name is shown and kept — sorted into its place,
    /// because unlike a window size a DPI that steps out of order would read as
    /// a bug.
    #[test]
    fn the_dpi_row_keeps_a_value_that_is_not_a_preset() {
        assert_eq!(
            dpi_options(800),
            DPI_PRESETS.to_vec(),
            "a preset is already in the list"
        );
        assert_eq!(
            dpi_options(1000),
            vec![400, 800, 1000, 1200, 1600, 2400, 3200],
            "an odd value is carried in order"
        );

        let mut settings = Settings::from_config(&config::Config::default());
        assert_eq!(settings.mouse_dpi, 800);
        // Right walks the presets in order...
        step_dpi(&mut settings, true);
        assert_eq!(settings.mouse_dpi, 1200);
        step_dpi(&mut settings, false);
        step_dpi(&mut settings, false);
        assert_eq!(settings.mouse_dpi, 400);
        // ...and left from the first entry wraps to the last.
        step_dpi(&mut settings, false);
        assert_eq!(settings.mouse_dpi, 3200, "left from the first wraps");
        // A value between presets takes part in the walk from where it falls,
        // and — like an odd resolution — is carried only while it is current:
        // stepping away from it lands on the neighbouring preset.
        settings.mouse_dpi = 1000;
        step_dpi(&mut settings, true);
        assert_eq!(settings.mouse_dpi, 1200);
        step_dpi(&mut settings, false);
        assert_eq!(settings.mouse_dpi, 800);
    }

    /// The sensitivity row reports cm/360 so a value tuned here can be carried
    /// to another game, and a habit from another game found here: it has to
    /// follow both numbers it is made of, and land where the calibration says —
    /// 1.60 on an 800-count mouse is a 12 cm turn.
    #[test]
    fn the_settings_row_reports_cm_per_360() {
        let base = cm_per_360(1.60, 800);
        assert!((base - 11.97).abs() < 0.05, "{base}");
        // Half the sensitivity is twice the distance; twice the DPI is half of
        // it, because a full turn costs the same counts either way.
        assert!((cm_per_360(0.80, 800) - base * 2.0).abs() < 1e-3);
        assert!((cm_per_360(1.60, 1600) - base * 0.5).abs() < 1e-3);
        // Hand-edited nonsense cannot put a NaN or an infinity on the row.
        let guarded = cm_per_360(0.0, 0);
        assert!(guarded.is_finite() && guarded > 0.0, "{guarded}");
    }

    /// The near-death grade is off until health or sanity crosses the same
    /// thresholds the vignette uses, deepens towards the worst state, and stops
    /// there — the world is never fully drained of colour.
    #[test]
    fn the_world_greys_out_only_near_death() {
        assert_eq!(critical_desaturation(100.0, 1.0), 0.0);
        assert_eq!(
            critical_desaturation(CRITICAL_HEALTH, CRITICAL_SANITY),
            0.0,
            "the thresholds themselves are not critical yet"
        );
        assert!(critical_desaturation(29.0, 1.0) > 0.0);
        assert!(
            critical_desaturation(100.0, 0.24) > 0.0,
            "sanity carries the grade alone"
        );
        assert!(
            (critical_desaturation(0.0, 1.0) - CRITICAL_DESATURATION).abs() < 1e-6,
            "zero health is the deepest grade"
        );
        let expected = CRITICAL_DESATURATION * (1.0 - 1.0 / CRITICAL_HEALTH);
        assert!((critical_desaturation(1.0, 0.5) - expected).abs() < 1e-6);
    }

    /// The water is not a palette swap. Only the drowned city is sunken, and
    /// its backdrop runs the other way round — a surface glow overhead fading
    /// down into murk — with the murk darker than any open horizon, so the
    /// arena reads as depth rather than as dusk.
    #[test]
    fn the_drowned_city_is_the_one_under_water() {
        let campaign = maps::campaign();
        let sunken: Vec<_> = campaign.iter().filter(|layout| layout.sunken).collect();
        assert_eq!(sunken.len(), 1, "exactly one arena is under water");
        assert_eq!(sunken[0].name_en, "R'LYEH");

        let sky = sunken[0].sky;
        let lum = |c: Color| 0.299 * c.r + 0.587 * c.g + 0.114 * c.b;
        assert!(
            lum(surface_glow(sky)) > lum(murk_colour(sky)),
            "the water column must run bright overhead to dark at the horizon"
        );
        assert!(
            lum(murk_colour(sky)) < lum(horizon_colour(sky)),
            "the murk must be darker than an open horizon, or it is just dusk"
        );
    }

    /// The loadout survives the stash: the file's text parses back into the
    /// same choices, `none` means none, and a kind this build does not know
    /// falls back to what the slot held rather than dropping it.
    #[test]
    fn the_loadout_round_trips_through_the_kit() {
        let kit = Kit {
            primary: Some(weapon::WeaponKind::Sniper),
            secondary: Some(weapon::WeaponKind::Shotgun),
            ammo_mags: [6, 3],
            grenades: [0, 3],
            meds: [5, 2, 0],
            ..Kit::issued()
        };
        let config = config::Config {
            primary: Kit::slot_text(kit.primary),
            secondary: Kit::slot_text(kit.secondary),
            primary_mags: kit.ammo_mags[0],
            secondary_mags: kit.ammo_mags[1],
            frags: kit.grenades[0],
            smokes: kit.grenades[1],
            bandages: kit.meds[0],
            medkits: kit.meds[1],
            painkillers: kit.meds[2],
            ..config::Config::default()
        };
        let back = Kit::from_config(&config);
        assert_eq!(back.primary, kit.primary);
        assert_eq!(back.secondary, kit.secondary);
        assert_eq!(back.ammo_mags, kit.ammo_mags);
        assert_eq!(back.grenades, kit.grenades);
        assert_eq!(back.meds, kit.meds);

        assert_eq!(loadout_slot("none", Some(weapon::WeaponKind::Rifle)), None);
        assert_eq!(
            loadout_slot("trebuchet", Some(weapon::WeaponKind::Rifle)),
            Some(weapon::WeaponKind::Rifle),
            "junk named a gun that does not exist"
        );
        assert_eq!(Kit::slot_text(None), "none");
        assert_eq!(Kit::slot_text(Some(weapon::WeaponKind::Smg)), "smg");
    }

    /// The counts `Kit::issued` starts with are the ones `MED_KINDS` documents
    /// beside its stats, and the grenade pair missions have always opened
    /// with — two tables of defaults that must not drift apart.
    #[test]
    fn the_issued_kit_matches_the_documented_counts() {
        let issued = Kit::issued();
        assert_eq!(
            issued.meds,
            [MED_KINDS[0].2, MED_KINDS[1].2, MED_KINDS[2].2]
        );
        assert_eq!(issued.grenades, [2, 1]);
        assert_eq!(issued.ammo_mags, [4, 0]);
    }

    /// The loadout slots walk "none" and the six guns, wrapping both ways; a
    /// kind the stash does not offer lands at the front of the walk.
    #[test]
    fn the_kind_choice_walks_none_and_every_gun() {
        use weapon::WeaponKind;
        let mut cur = None;
        let mut seen = Vec::new();
        for _ in 0..ARMORY_KINDS.len() + 1 {
            cur = step_kind_choice(cur, true);
            seen.push(cur);
        }
        assert_eq!(cur, None, "a full lap comes back to none");
        assert!(seen.contains(&Some(WeaponKind::Rifle)));
        assert!(seen.contains(&Some(WeaponKind::Pistol)));
        assert_eq!(
            step_kind_choice(None, false),
            Some(WeaponKind::Pistol),
            "backwards from none wraps to the last gun"
        );
        assert_eq!(
            step_kind_choice(Some(WeaponKind::Knife), true),
            Some(WeaponKind::Rifle),
            "a stray kind lands at the front of the walk"
        );
    }

    /// A part id from the live catalogue, by the name it is filed under. The
    /// catalogue a test build sees is not the shipped one — it carries the
    /// demo parts too — so the bench's tests ask for parts by name rather than
    /// by position.
    fn part_id(name: &str) -> Option<weapon::PartId> {
        weapon::catalogue()
            .iter()
            .map(|p| p.id)
            .find(|id| id.0 == name)
    }

    /// The bench writes through exactly one slot. Fitting a part must not move
    /// another slot's part or its roll, and taking one off must not take
    /// anything else with it: `seated` is the only write the bench makes, so
    /// this is the whole of what a click changes.
    #[test]
    fn seating_a_candidate_touches_one_slot() {
        let def = weapon::def(weapon::WeaponKind::Rifle);
        let (optic, muzzle) = (weapon::SlotId("optic"), weapon::SlotId("muzzle"));
        let dot = part_id("micro_dot").expect("the dot is in the catalogue");
        let brake = part_id("compensator").expect("the brake is in the catalogue");
        let start = seated(&weapon::WeaponBuild::empty(), optic, Some(dot), 0.25);

        let fitted = seated(&start, muzzle, Some(brake), 0.75);
        assert_eq!(fitted_part(&fitted, optic), Some(dot));
        assert_eq!(fitted_part(&fitted, muzzle), Some(brake));
        assert_eq!(
            fitted.entries.len(),
            2,
            "an entry was added twice: {fitted:?}"
        );
        assert!(
            fitted
                .entries
                .iter()
                .any(|e| e.slot == optic && e.roll == 0.25),
            "the optic lost the roll it was seated at: {:?}",
            fitted.entries
        );
        assert!(
            weapon::validate(def, &fitted).is_ok(),
            "the bench built a weapon that does not validate: {:?}",
            weapon::validate(def, &fitted)
        );

        let bare = seated(&fitted, muzzle, None, 0.0);
        assert_eq!(fitted_part(&bare, muzzle), None);
        assert_eq!(fitted_part(&bare, optic), Some(dot));
        assert!(weapon::validate(def, &bare).is_ok());
    }

    /// The candidate column lists what will seat and nothing else: "none"
    /// first, then every catalogue part whose slot and standard match — the
    /// same pair of checks `validate` makes. A part built for another slot is
    /// never offered, a slot the weapon does not have has no list at all, and
    /// the part already in the slot is on its own list, which is how the
    /// column can mark it.
    #[test]
    fn the_candidate_column_offers_only_what_fits() {
        let def = weapon::def(weapon::WeaponKind::Rifle);
        let empty = weapon::WeaponBuild::empty();
        let optic = weapon::SlotId("optic");
        let accepts = weapon::reachable_slots(def, &empty)
            .iter()
            .find(|s| s.id == optic)
            .map(|s| s.accepts)
            .expect("a rifle has an optic rail");

        let choices = part_choices(def, &empty, optic).expect("a rifle has the slot");
        assert_eq!(
            choices.first(),
            Some(&None),
            "the column does not start on none"
        );
        let want: Vec<_> = weapon::catalogue()
            .iter()
            .filter(|p| p.slot == optic && p.interface == accepts)
            .map(|p| Some(p.id))
            .collect();
        assert_eq!(
            &choices[1..],
            &want[..],
            "the column is not the catalogue's own order"
        );
        let dot = part_id("micro_dot").expect("the dot is in the catalogue");
        assert!(want.contains(&Some(dot)), "the rail lost its dot");

        let fitted = seated(&empty, optic, Some(dot), 0.4);
        let again = part_choices(def, &fitted, optic).expect("a rifle has the slot");
        assert!(
            again.contains(&Some(dot)),
            "the fitted part is not on its own list: {again:?}"
        );

        let knife = weapon::def(weapon::WeaponKind::Knife);
        assert!(
            part_choices(knife, &weapon::WeaponBuild::empty(), optic).is_none(),
            "a weapon with no rail offered optics"
        );
    }

    /// A roll is what makes two copies of a ranged part differ, so a part
    /// arriving in a slot draws one — but the copy already there keeps the roll
    /// it has, or the bench could be clicked twice to reroll it.
    #[test]
    fn reseating_the_same_part_is_not_a_reroll() {
        let slot = weapon::SlotId("magazine");
        let lot = part_id("surplus_lot").expect("the surplus lot is in the catalogue");
        let other = part_id("extended_mag").expect("the extended mag is in the catalogue");
        let mut rng = Rng::new(7);
        let build = seated(&weapon::WeaponBuild::empty(), slot, Some(lot), 0.31);

        assert_eq!(seat_roll(&mut rng, &build, slot, Some(lot)), 0.31);
        let fresh = seat_roll(&mut rng, &build, slot, Some(other));
        assert!(
            fresh.is_finite() && (0.0..1.0).contains(&fresh),
            "a new part was seated at {fresh}, which is not a roll"
        );
        assert_eq!(
            seat_roll(&mut rng, &build, slot, None),
            0.0,
            "emptying a slot has no number to roll"
        );
    }
}

/// The window's opening state, read from the settings file.
///
/// macroquad calls this once, before the window exists, so it is the only chance
/// the saved size and mode get to shape the window rather than being applied to
/// one that is already on screen. The file is read again in `main`, where the
/// language and the volume are wanted too; it is small, and reading it twice is
/// cheaper than a cache that has to be kept correct.
fn window_conf() -> Conf {
    let saved = config::load();
    Conf {
        window_title: "TIDES OF R'LYEH / 拉莱耶的潮声".to_owned(),
        window_width: saved.width,
        window_height: saved.height,
        fullscreen: saved.fullscreen,
        high_dpi: true,
        sample_count: 4,
        window_resizable: true,
        ..Default::default()
    }
}

/// The colour the sky reaches at the horizon.
///
/// Shared with the fog so that distant geometry fades into the sky rather than
/// crossing it with a hard edge — two expressions that had to agree would
/// eventually stop agreeing.
fn horizon_colour(sky: Color) -> Color {
    Color::new(
        (sky.r * 0.55 + 0.12).min(1.0),
        (sky.g * 0.50 + 0.10).min(1.0),
        (sky.b * 0.45 + 0.09).min(1.0),
        1.0,
    )
}

/// What distance dissolves into under water — [`horizon_colour`]'s drowned
/// counterpart, and the colour the fog shares with the sky either way.
///
/// Far darker than an open horizon: the sea takes light with distance instead
/// of scattering it back, so R'LYEH's far wing goes away into the dark rather
/// than paling out.
fn murk_colour(sky: Color) -> Color {
    Color::new(sky.r * 0.45, sky.g * 0.50, sky.b * 0.50, 1.0)
}

/// The top of a sunken frame: the surface glow, far overhead.
fn surface_glow(sky: Color) -> Color {
    Color::new(
        (sky.r * 1.30 + 0.030).min(1.0),
        (sky.g * 1.25 + 0.048).min(1.0),
        (sky.b * 1.25 + 0.042).min(1.0),
        1.0,
    )
}

/// The gradient both backdrops are built from: `top` at the top of the frame
/// down to `bottom` at the horizon, in the same forty bands either way.
fn draw_bands(top: Color, bottom: Color) {
    let h = screen_height();
    let w = screen_width();
    let bands = 40.0;
    for i in 0..bands as i32 {
        let t = i as f32 / bands;
        let c = Color::new(
            top.r + (bottom.r - top.r) * t,
            top.g + (bottom.g - top.g) * t,
            top.b + (bottom.b - top.b) * t,
            1.0,
        );
        draw_rectangle(0.0, h * t, w, h / bands + 1.0, c);
    }
}

/// A night sky: darkest overhead, brightening down to the horizon haze.
fn draw_sky(sky: Color) {
    let top = Color::new(sky.r * 0.30, sky.g * 0.32, sky.b * 0.42, 1.0);
    draw_bands(top, horizon_colour(sky));
}

/// The drowned city's backdrop: the same bands, lit from the other end.
///
/// Light reaches this arena from the surface far above, so the frame is
/// brightest at the top — a dim teal glow — and falls away to the murk at the
/// horizon, which is also what the fog fades the far geometry into. Night and
/// depth must not be mistaken for each other, and the way to keep them apart
/// is to run the gradient the other way and take the light out of it.
fn draw_sunken_sky(sky: Color) {
    draw_bands(surface_glow(sky), murk_colour(sky));
}

/// Ray against the player's own body box, for enemy fire. Uses the current
/// stance height so crouching genuinely presents a smaller target.
fn ray_player(player: &Player, origin: Vec3, dir: Vec3) -> Option<f32> {
    use player::PLAYER_RADIUS;
    ray_aabb(
        origin,
        dir,
        vec3(
            player.pos.x - PLAYER_RADIUS,
            player.pos.y,
            player.pos.z - PLAYER_RADIUS,
        ),
        vec3(
            player.pos.x + PLAYER_RADIUS,
            player.pos.y + player.body_height(),
            player.pos.z + PLAYER_RADIUS,
        ),
    )
}

/// Where a shot landed relative to the player's feet. Below the hips it
/// struck the legs, which are never armoured (reference §17.2① — CS2's
/// "legs are never armored", the one armour rule players feel immediately).
fn struck_legs(player: &Player, hit: Vec3) -> bool {
    const LEG_FRACTION: f32 = 0.45;
    let h = (hit.y - player.pos.y) / player.body_height().max(0.1);
    h < LEG_FRACTION
}

/// Splits a hit between the armour bar and the body: 65% absorbed while the
/// bar holds, everything past it through. Returns `(absorbed, through)`.
fn armor_split(amount: f32, armor: f32, legs: bool) -> (f32, f32) {
    // No hit point and no legs: a blast is a body hit — the rule is about
    // where bullets land, and a grenade does not land anywhere in particular.
    let absorbed = if legs {
        0.0
    } else {
        (amount * 0.65).min(armor.max(0.0))
    };
    (absorbed, amount - absorbed)
}

fn apply_player_damage(
    player: &mut Player,
    armor: &mut f32,
    audio: &audio::Audio,
    amount: f32,
    from: Vec3,
    hit: Option<Vec3>,
) {
    if !player.alive {
        return;
    }
    let legs = hit.is_some_and(|p| struck_legs(player, p));
    let (absorbed, through) = armor_split(amount, *armor, legs);
    *armor -= absorbed;
    player.damage(through);
    // §17.2②: armour absorbs the *punch* as well as the padding — a stopped
    // round barely moves the view where an unarmoured one shoves it. The
    // 0.0022 rad per damage point puts a Grunt's 8 at about one degree and
    // the Colossus's 42 at the clamp.
    let punch = amount
        * if absorbed > 0.0 {
            0.0022 * 0.15
        } else {
            0.0022
        };
    player.aim_punch(punch);
    // Where the hit came from, for the HUD's directional indicator. `hurt_from`
    // reads a direction from the player out toward the source and ignores a
    // flat zero, and this is the one place damage reaches the player, so the
    // bearing can never be recorded twice for one hit.
    player.hurt_from(from - player.pos);
    audio.play(Sfx::PlayerHurt);
}

/// Returns true only when a shot was actually fired, so the caller can keep
/// the gunshot audio in step with the real rate of fire.
fn fire_player_shot(game: &mut Game, audio: &audio::Audio, pressed: bool, held: bool) -> bool {
    let s = game.arsenal.active_weapon().stats();
    // `try_fire` already applies the fire-mode rule — it takes both the rising
    // edge and the sustained state. The extra latch that used to guard this was
    // not merely redundant, it was wrong: the caller only reaches here while
    // the button is down, so the latch could never be cleared and every
    // semi-automatic weapon fired exactly once per mission.
    if !game.arsenal.active_weapon_mut().try_fire(pressed, held) {
        return false;
    }

    let eye = game.player.eye();
    let spread = game.arsenal.active_weapon().spread(
        game.player.move_ratio(),
        game.player.ads,
        game.player.spread_scale(),
    );
    let headshot = s.headshot_mult;

    // A knife is a swing, not a shot: short reach, no falloff, no tracers.
    let (rays, range) = if s.melee {
        (3, 2.6)
    } else {
        (s.pellets, weapon::MAX_RANGE)
    };
    if !s.melee {
        game.shots_fired += s.pellets;
    }

    for _ in 0..rays {
        let dir = game.rng.cone(game.player.look_dir(), spread);
        let hit = combat::hitscan(&game.level, &game.enemies, eye, dir, range);
        match hit.kind {
            combat::HitKind::World => {
                if !s.melee {
                    let n = (eye - hit.point).normalize_or_zero();
                    game.effects.spark(
                        hit.point,
                        n,
                        Color::new(1.0, 0.85, 0.5, 1.0),
                        4,
                        &mut game.rng,
                    );
                    audio.play_at(Sfx::HitWorld, hit.dist);
                }
            }
            combat::HitKind::Enemy { index, head } => {
                game.shots_hit += 1;
                let dmg = game.arsenal.active_weapon().damage_at(hit.dist)
                    * if head { headshot } else { 1.0 };
                // The weapon's own penetration number goes in with the shot
                // (reference §17.2③): armour stops less of a rifle round than
                // of a pistol one, visibly.
                let pen = game.arsenal.active_weapon().kind.armor_pen();
                let died = game.enemies[index].take_damage(dmg, pen);
                game.effects.blood(
                    hit.point,
                    dir,
                    Color::new(0.55, 0.10, 0.10, 1.0),
                    &mut game.rng,
                );
                game.effects.damage_number(hit.point, dmg, head);
                if head {
                    game.effects.head_marker = 1.0;
                    audio.play(Sfx::HitHead);
                } else {
                    game.effects.hit_marker = 1.0;
                    audio.play(Sfx::HitFlesh);
                }
                if died {
                    kill_enemy(game, index, audio);
                }
                // A knife only strikes the first thing in its arc.
                if s.melee {
                    break;
                }
            }
        }
        if !s.melee {
            // Outgoing fire is the thin, pale strand. The two tracer looks are
            // built so they are told apart without colour: narrow and washed
            // out against the incoming one's wide, deep red — colour alone is
            // the first thing a colour-blind player or a washed-out monitor
            // loses. It starts out along the barrel rather than at the eye;
            // see `TRACER_NEAR`. The reach clamp is for a shot into something
            // the muzzle is already touching, where the offset would otherwise
            // start the strand beyond its own impact.
            let reach = (hit.point - eye).length();
            game.effects.tracer(
                eye + dir * TRACER_NEAR.min(reach * 0.9),
                hit.point,
                Color::new(1.0, 0.97, 0.80, 1.0),
                0.012,
            );
        }
    }

    if !s.melee {
        let jitter = game.rng.signed();
        game.player
            .add_recoil(s.recoil_pitch, s.recoil_yaw * jitter);
    }
    // Firing breaks a healing channel.
    game.heal_t = 0.0;
    true
}

fn kill_enemy(game: &mut Game, index: usize, audio: &audio::Audio) {
    game.killed += 1;
    let score = game.enemies[index].score;
    game.score += score;
    // A kill is the one moment worth a flash and a beat of slow motion. The
    // flash used to be a field that only ever decayed — nothing set it, so the
    // frame it was built for never came; it decays in `effects.update`.
    game.effects.kill_flash = 1.0;
    game.hitstop = HITSTOP_TIME;
    audio.play(Sfx::EnemyDeath);
    let at = vec3(game.enemies[index].pos.x, 0.0, game.enemies[index].pos.z);
    let spawns = game.enemies[index].spawns_on_death;
    let roll = game.rng.f32();
    if roll < 0.16 {
        game.pickups.push(Pickup::new(at, PickupKind::Health, 30.0));
    } else if roll < 0.42 {
        game.pickups.push(Pickup::new(at, PickupKind::Ammo, 45.0));
    } else if roll < 0.52 {
        game.pickups.push(Pickup::new(at, PickupKind::Armor, 35.0));
    }

    // Body horror: the corpse bursts open and smaller things crawl out, so
    // killing a Tentacle horror at point-blank range is a mistake.
    if spawns > 0 {
        // The hatchlings are born at the wave that killed their host, so they
        // inherit its strength like any other hostile of that wave.
        let wave = campaign::wave_strength(game.mission, game.waves_done);
        for i in 0..spawns {
            // The whole-scene ceiling (reference §19.4-4): pressure is meant
            // to be bounded by what the player can fight, not by how many
            // corpses happened to burst at once. Twenty-four is far above
            // anything the missions' own `max_alive` (5–8) can produce on its
            // own, so this only ever catches a pile-up of spawnlings — which
            // is exactly the pile-up it exists to catch.
            if game.alive_enemies() >= ENEMY_CAP {
                break;
            }
            let a = i as f32 / spawns as f32 * std::f32::consts::TAU;
            let offset = vec3(a.cos() * 1.05, 0.0, a.sin() * 1.05);
            let spot = at + offset;
            let spot = if game.level.point_free(spot, 0.26, 0.72) {
                spot
            } else {
                at
            };
            let mut spawned = Enemy::spawn(EnemyKind::Spawnling, spot, wave, &mut game.rng);
            // Otherwise the hatchling stands where it crawled out, never having
            // seen anything, and still counts as alive.
            spawned.alert_to(at, 20.0);
            game.enemies.push(spawned);
        }
        game.effects.blood(
            at + vec3(0.0, 0.7, 0.0),
            Vec3::Y,
            Color::new(0.42, 0.05, 0.16, 1.0),
            &mut game.rng,
        );
    }
}

/// Flat player-to-spawn distance: hostiles walk, they do not fly.
fn spawn_distance(at: Vec3, player: &Player) -> f32 {
    vec3(at.x - player.pos.x, 0.0, at.z - player.pos.z).length()
}

/// The spawn points this player may be attacked from: everything at least
/// `MIN` away, measured flat.
///
/// When nothing is far enough, the fallback is the *farthest point there
/// is*. The old chooser fell back to `spawns[0]`, which is not a rule about
/// spacing at all — it is a rule about array order, and on some arenas it
/// put a hostile in the player's lap. (Reference §19.2.1.)
fn spawn_pool(level: &Level, player: &Player) -> Vec<Vec3> {
    const MIN: f32 = 14.0;
    let legal: Vec<Vec3> = level
        .spawns
        .iter()
        .copied()
        .filter(|s| spawn_distance(*s, player) >= MIN)
        .collect();
    if !legal.is_empty() {
        return legal;
    }
    level
        .spawns
        .iter()
        .copied()
        .max_by(|a, b| spawn_distance(*a, player).total_cmp(&spawn_distance(*b, player)))
        .into_iter()
        .collect()
}

/// One weighted draw over `pool`: entrances the player cannot see are worth
/// more (`HIDDEN_WEIGHT`), and nearness to `IDEAL` is the rest. Returns the
/// index into `pool`.
fn weighted_spawn_index(pool: &[Vec3], level: &Level, player: &Player, rng: &mut Rng) -> usize {
    const IDEAL: f32 = 26.0;
    // What an unseen point is worth against a seen one at the same distance.
    // Two and a half prefers cover without letting a distant hatch beat a
    // good close entrance, and without freezing the spread onto whichever
    // corners happen to be blind.
    const HIDDEN_WEIGHT: f32 = 2.5;

    let eye = vec3(player.pos.x, player.pos.y + 1.6, player.pos.z);
    let mut weights: Vec<f32> = Vec::with_capacity(pool.len());
    let mut total = 0.0f32;
    for s in pool {
        let d = spawn_distance(*s, player);
        // A soft triangle around `IDEAL`: half weight at 1 m of error, a
        // quarter at 3 m, an eighth at 7 m. Deliberately gentler than
        // `1 / (1 + dev)` — the sharp version crowns one point per arena
        // even when nothing is wrong with the others.
        let mut w = 1.0 / (1.0 + 0.6 * (d - IDEAL).abs());
        if !level.line_of_sight(vec3(s.x, 1.6, s.z), eye) {
            w *= HIDDEN_WEIGHT;
        }
        total += w;
        weights.push(w);
    }
    let mut roll = rng.range(0.0, total);
    for (i, w) in weights.iter().enumerate() {
        roll -= w;
        if roll <= 0.0 {
            return i;
        }
    }
    // Float sums disagreeing with themselves lands here; the last legal
    // point is still a legal point.
    pool.len().saturating_sub(1)
}

/// Picks where the next hostile enters the arena.
///
/// The rule used to be ten draws *with replacement*, scored
/// `hidden ? 30 : 0 - |d - IDEAL|`. Out of sight was worth 30 and the
/// distance term is worth at most 16, so any draw into a blind corner won
/// outright — one corner per arena soaked up almost every spawn, which read
/// as "everything arrives from the same place". Now hiding is a weight
/// rather than an override, distance keeps its say through `IDEAL`, and the
/// weighted draw keeps a fixed player position from falling into a fixed
/// pattern. Preferring the farthest point was fine on the small arenas and
/// disastrous on R'LYEH, where the nearest spawn is 19 m and the farthest
/// 65 m through a deliberate labyrinth — forty seconds of nothing happening.
///
/// The wave system no longer calls this per release (a wave fixes its
/// entrances up front — see `spawn_set`), but it remains the one-draw
/// version of the same chooser, and the distribution test in `maps` pins it.
#[cfg(test)]
fn pick_spawn(level: &Level, player: &Player, rng: &mut Rng) -> Vec3 {
    let pool = spawn_pool(level, player);
    let i = weighted_spawn_index(&pool, level, player, rng);
    // An empty arena (no spawns at all, which the arena tests already forbid)
    // degrades to the player's position rather than panicking in a shipping
    // build.
    pool.get(i).copied().unwrap_or(player.pos)
}

/// The number of different entrances a single wave uses.
const WAVE_ENTRANCES: usize = 3;

/// The most hostiles that may be alive at once, from every source. Missions
/// cap their own releases at `max_alive` (5–8); this is the ceiling for
/// everything that *multiplies* — the spawnlings that burst out of corpses —
/// so a chain of deaths can never swell the field past what the frame budget
/// and the player's attention can carry (reference §19.4-4).
const ENEMY_CAP: usize = 24;

/// The entrances one wave uses: up to `count` distinct points, drawn the way
/// `pick_spawn` draws them but without replacement. Fixing a wave's entrances
/// is what turns arrivals into a direction the player can read and hold
/// (reference §19.2) instead of noise from every corner — and it is also why
/// `WaveRelease` carries a rotation cursor rather than calling `pick_spawn`
/// per release.
fn spawn_set(level: &Level, player: &Player, rng: &mut Rng, count: usize) -> Vec<Vec3> {
    let mut pool = spawn_pool(level, player);
    let mut set = Vec::with_capacity(count.min(pool.len()));
    while set.len() < count && !pool.is_empty() {
        let i = weighted_spawn_index(&pool, level, player, rng);
        set.push(pool.swap_remove(i));
    }
    set
}

/// The entrance for the next release: the wave's set is used in rotation, so
/// no two arrivals share a point back to back. A point the player is now
/// standing on is skipped while any other remains.
fn wave_point(rel: &mut WaveRelease, player: &Player) -> Vec3 {
    const MIN: f32 = 10.0;
    let n = rel.points.len();
    for _ in 0..n {
        let p = rel.points[rel.cursor % n];
        rel.cursor += 1;
        if spawn_distance(p, player) >= MIN {
            return p;
        }
    }
    // Every entrance is underfoot (or the arena has none, which the arena
    // tests forbid): the farthest of the set beats none at all.
    rel.points
        .iter()
        .copied()
        .max_by(|a, b| spawn_distance(*a, player).total_cmp(&spawn_distance(*b, player)))
        .unwrap_or(player.pos)
}

/// The window between a wave's last kill and the next wave's first release.
///
/// The reference suggests 45–60 s after KF2, where the window is a shop
/// phase; this game has no shop, so the window is only as long as the three
/// things it is actually for — reloading, healing, and picking where to stand
/// for the next wave's entrances. Twelve seconds fits all three with slack.
/// The hold-out shortens it further: there the clock *is* the pressure, and
/// six windows over two minutes must not eat the mission.
fn wave_gap(mission: &Mission) -> f32 {
    match mission.objective {
        Objective::Survive => 4.0,
        _ => 12.0,
    }
}

/// The mission's threat multipliers, exactly as `Enemy::spawn` applies them
/// to every hostile of the wave in progress: damage and attack rate at that
/// wave's strength (`campaign::wave_strength`). Shown in the mission panel so
/// the climb within a mission and the step between missions both have a
/// number on them (reference §19.4-6).
fn threat_numbers(mission: &Mission, wave_index: u32) -> (f32, f32) {
    let w = campaign::wave_strength(mission, wave_index).saturating_sub(1) as f32;
    let damage = 1.0 + 0.06 * w;
    let rate = 1.0 / (1.0 - 0.015 * w).max(0.01);
    (damage, rate)
}

/// True when the cursor actually moved since the last frame.
///
/// Every menu here answers to both devices, and the keyboard has to win while
/// it is in use: with the cursor parked over a row, an unguarded hover test
/// drags the selection back on every frame and makes the arrow keys look
/// broken. A pixel of slack keeps a jittering mouse from doing the same.
/// Where `continue` picks up: the mission the player was in, held inside what
/// they have actually unlocked and inside the campaign. A hand-edited file
/// cannot send it anywhere that does not exist.
fn resume_mission(unlocked: usize, last: usize) -> usize {
    last.min(unlocked.saturating_sub(1))
        .min(campaign::count().saturating_sub(1))
}

fn mouse_moved(last: &mut Vec2) -> bool {
    let (mx, my) = mouse_position();
    let now = vec2(mx, my);
    let moved = (now - *last).length_squared() > 1.0;
    *last = now;
    moved
}

/// The row a hover should select: the one under the cursor when the mouse has
/// just moved, otherwise whatever the keyboard had. Split out of the drawing
/// loops so the rule can be tested without a window.
fn hover_selection(current: usize, hover: Option<usize>, moved: bool) -> usize {
    match hover {
        Some(i) if moved => i,
        _ => current,
    }
}

fn menu_nav(sel: &mut usize, len: usize, audio: &audio::Audio) {
    if len == 0 {
        return;
    }
    let before = *sel;
    if is_key_pressed(KeyCode::Down) {
        *sel = (*sel + 1) % len;
    }
    if is_key_pressed(KeyCode::Up) {
        *sel = (*sel + len - 1) % len;
    }
    if *sel != before {
        audio.play(Sfx::UiMove);
    }
}

fn hovered_rect(rect: &(f32, f32, f32, f32)) -> bool {
    let (mx, my) = mouse_position();
    let (x, y, w, h) = *rect;
    mx >= x && mx <= x + w && my >= y && my <= y + h
}

fn clicked_rect(rect: &(f32, f32, f32, f32)) -> bool {
    is_mouse_button_pressed(MouseButton::Left) && hovered_rect(rect)
}

fn clicked(rects: &[(f32, f32, f32, f32)], idx: usize) -> bool {
    rects.get(idx).is_some_and(clicked_rect)
}

#[macroquad::main(window_conf)]
async fn main() {
    // The window exists by the time this body runs, so the IME can be detached
    // before the first keystroke is read.
    disable_ime();
    // Same reason, other direction: Shift is sprint here, and five taps of it is
    // how Windows asks whether you want Sticky Keys.
    #[cfg(windows)]
    let _quiet_keys = win32::QuietKeys::new();
    // The saved settings decide the language and the volume as well as the
    // window, and both have to be in place before the first frame: the language
    // before the fonts are asked for anything and the volume before an effect
    // can play. The window itself was built from this same file by
    // `window_conf`, before this body ran.
    let saved = config::load();
    i18n::set_lang(saved.lang);

    let fonts = hud::Fonts::load().await;
    let mut audio = audio::Audio::load().await;
    audio.set_master(saved.volume);

    let mut game = Game::new(
        Settings::from_config(&saved),
        Kit::from_config(&saved),
        saved.unlocked,
        saved.last_mission,
        saved.best,
    );
    let mut menu_rects: Vec<(f32, f32, f32, f32)>;

    loop {
        let dt = get_frame_time().min(0.05);
        // The HUD takes its accent from the arena, so this has to land before
        // any panel is drawn — menus included, where the arena is the last one
        // played.
        hud::set_accent(game.level.accent);

        match game.phase {
            Phase::Menu => {
                pointer::clip(false);
                pointer::show(true);
                // The manual, if it is up, is drawn over the menu and answers
                // to its own keys: H and K open it on the controls and the
                // codex, ← and → turn the pages, and Esc drops back to the menu
                // — which never went anywhere, because the phase does not move
                // while the manual is open over it (see `Game::menu_manual`).
                clear_background(Color::new(0.03, 0.04, 0.06, 1.0));
                draw_sky(Color::new(0.06, 0.07, 0.11, 1.0));
                if handle_manual_keys(&mut game, &audio, true) {
                    draw_manual(&game, &fonts);
                } else {
                    let items = [
                        hud::MenuItem {
                            label: i18n::t(Key::NewCampaign),
                            enabled: true,
                        },
                        // Where the player left off. Disabled on a fresh save,
                        // so a first run cannot offer a mission that has never
                        // been reached.
                        hud::MenuItem {
                            label: i18n::t(Key::ContinueGame),
                            enabled: game.unlocked > 1,
                        },
                        hud::MenuItem {
                            label: i18n::t(Key::SelectMission),
                            enabled: true,
                        },
                        hud::MenuItem {
                            label: i18n::t(Key::HowToPlay),
                            enabled: true,
                        },
                        hud::MenuItem {
                            label: i18n::t(Key::Codex),
                            enabled: true,
                        },
                        hud::MenuItem {
                            label: i18n::t(Key::Settings),
                            enabled: true,
                        },
                        hud::MenuItem {
                            label: i18n::t(Key::Armory),
                            enabled: true,
                        },
                        hud::MenuItem {
                            label: i18n::t(Key::Quit),
                            enabled: true,
                        },
                    ];
                    menu_nav(&mut game.menu_sel, items.len(), &audio);
                    menu_rects = hud::draw_menu(
                        &fonts,
                        i18n::t(Key::Title),
                        Some(i18n::t(Key::Subtitle)),
                        &items,
                        game.menu_sel,
                    );
                    let moved = mouse_moved(&mut game.menu_mouse);
                    let hover = menu_rects.iter().position(hovered_rect);
                    game.menu_sel = hover_selection(game.menu_sel, hover, moved);
                    let enter = is_key_pressed(KeyCode::Enter) || is_key_pressed(KeyCode::KpEnter);
                    let choose =
                        |i: usize| (enter && game.menu_sel == i) || clicked(&menu_rects, i);
                    if choose(0) {
                        audio.play(Sfx::UiClick);
                        game.new_campaign();
                    } else if choose(1) {
                        // Continues where the player was, not where the menu
                        // left the highlight.
                        audio.play(Sfx::UiClick);
                        game.start(resume_mission(game.unlocked, game.last_mission));
                    } else if choose(2) {
                        audio.play(Sfx::UiClick);
                        game.phase = Phase::MissionSelect;
                        game.menu_sel = 0;
                    } else if choose(3) {
                        audio.play(Sfx::UiClick);
                        game.open_menu_manual(0);
                    } else if choose(4) {
                        audio.play(Sfx::UiClick);
                        game.open_menu_manual(1);
                    } else if choose(5) {
                        audio.play(Sfx::UiClick);
                        game.phase = Phase::Settings;
                        game.settings_sel = 0;
                    } else if choose(6) {
                        audio.play(Sfx::UiClick);
                        game.phase = Phase::Armory;
                        game.armory_sel = 0;
                        game.armory_dirty = true;
                    } else if choose(7) {
                        break;
                    }
                }
            }

            Phase::MissionSelect => {
                pointer::clip(false);
                pointer::show(true);
                let layouts = maps::campaign();
                let names: Vec<String> = (0..campaign::count())
                    .map(|i| {
                        let m = campaign::get(i);
                        let l = &layouts[m.layout];
                        let n = match i18n::lang() {
                            i18n::Lang::Zh => l.name_zh,
                            i18n::Lang::En => l.name_en,
                        };
                        format!("{}. {}", i + 1, n)
                    })
                    .collect();
                let items: Vec<hud::MenuItem> = names
                    .iter()
                    .enumerate()
                    .map(|(i, n)| hud::MenuItem {
                        label: n.as_str(),
                        enabled: i < game.unlocked,
                    })
                    .chain(std::iter::once(hud::MenuItem {
                        label: i18n::t(Key::Back),
                        enabled: true,
                    }))
                    .collect();

                menu_nav(&mut game.menu_sel, items.len(), &audio);
                clear_background(Color::new(0.03, 0.04, 0.06, 1.0));
                draw_sky(Color::new(0.07, 0.08, 0.12, 1.0));
                menu_rects = hud::draw_menu(
                    &fonts,
                    i18n::t(Key::SelectMission),
                    None,
                    &items,
                    game.menu_sel,
                );
                let moved = mouse_moved(&mut game.menu_mouse);
                let hover = menu_rects
                    .iter()
                    .zip(items.iter())
                    .position(|(rect, item)| item.enabled && hovered_rect(rect));
                game.menu_sel = hover_selection(game.menu_sel, hover, moved);
                let enter = is_key_pressed(KeyCode::Enter) || is_key_pressed(KeyCode::KpEnter);
                let n = items.len();
                for (i, (rect, item)) in menu_rects.iter().zip(items.iter()).enumerate() {
                    if !item.enabled || !((enter && game.menu_sel == i) || clicked_rect(rect)) {
                        continue;
                    }
                    audio.play(Sfx::UiClick);
                    if i == n - 1 {
                        game.phase = Phase::Menu;
                        game.menu_sel = 0;
                    } else {
                        game.start(i);
                    }
                    break;
                }
            }

            Phase::Settings => {
                pointer::clip(false);
                pointer::show(true);
                let on = i18n::t(Key::On);
                let off = i18n::t(Key::Off);
                let aim_label = if game.settings.aim_toggle {
                    i18n::t(Key::AimToggle)
                } else {
                    i18n::t(Key::AimHold)
                };
                // Rows in the order the change handler below reads them:
                // 0 sensitivity, 1 mouse DPI, 2 volume, 3 aim mode, 4 lean mode,
                // 5 invert Y, 6 show FPS, 7 fullscreen, 8 resolution, 9 language,
                // 10 back.
                let rows = [
                    format!(
                        "{}  {:.2} · {:.1} cm/360",
                        i18n::t(Key::Sensitivity),
                        game.settings.sensitivity,
                        cm_per_360(game.settings.sensitivity, game.settings.mouse_dpi)
                    ),
                    format!("{}  {}", i18n::t(Key::MouseDpi), game.settings.mouse_dpi),
                    format!(
                        "{}  {:.0}%",
                        i18n::t(Key::Volume),
                        game.settings.volume * 100.0
                    ),
                    format!("{}  {}", i18n::t(Key::AimMode), aim_label),
                    format!(
                        "{}  {}",
                        i18n::t(Key::LeanMode),
                        if game.settings.lean_toggle {
                            i18n::t(Key::AimToggle)
                        } else {
                            i18n::t(Key::AimHold)
                        }
                    ),
                    format!(
                        "{}  {}",
                        i18n::t(Key::InvertY),
                        if game.settings.invert_y { on } else { off }
                    ),
                    format!(
                        "{}  {}",
                        i18n::t(Key::ShowFps),
                        if game.settings.show_fps { on } else { off }
                    ),
                    format!(
                        "{}  {}",
                        i18n::t(Key::Fullscreen),
                        if game.settings.fullscreen { on } else { off }
                    ),
                    format!(
                        "{}  {} × {}",
                        i18n::t(Key::Resolution),
                        game.settings.width,
                        game.settings.height
                    ),
                    format!("{}  {}", i18n::t(Key::Language), i18n::lang().label()),
                    i18n::t(Key::Back).to_string(),
                ];
                let resolution_row = 8;
                let items: Vec<hud::MenuItem> = rows
                    .iter()
                    .enumerate()
                    .map(|(i, r)| hud::MenuItem {
                        label: r.as_str(),
                        // The resolution row is dimmed while fullscreen: the mode
                        // decides the size then, so a left/right press records
                        // the choice — it is applied by the change that leaves
                        // fullscreen — without anything moving now.
                        enabled: i != resolution_row || !game.settings.fullscreen,
                    })
                    .collect();

                menu_nav(&mut game.settings_sel, items.len(), &audio);
                let left = is_key_pressed(KeyCode::Left);
                let right = is_key_pressed(KeyCode::Right);
                clear_background(Color::new(0.03, 0.04, 0.06, 1.0));
                draw_sky(Color::new(0.07, 0.08, 0.12, 1.0));
                menu_rects = hud::draw_menu(
                    &fonts,
                    i18n::t(Key::Settings),
                    None,
                    &items,
                    game.settings_sel,
                );

                // The mouse works here the way it works on the other menus:
                // moving it selects the row under it, and clicking a row steps
                // that row's value exactly as the right arrow does. There is no
                // Apply button anywhere in this game, so a click means "change
                // this" rather than "arm this" — and BACK is left to the code
                // below, which is what a click on it is for.
                let back = items.len() - 1;
                let moved = mouse_moved(&mut game.menu_mouse);
                let hover = menu_rects.iter().position(hovered_rect);
                game.settings_sel = hover_selection(game.settings_sel, hover, moved);
                let clicked_row = menu_rects.iter().position(clicked_rect);
                if let Some(i) = clicked_row {
                    game.settings_sel = i;
                }
                let step = if clicked_row == Some(back) {
                    None
                } else if right || clicked_row.is_some() {
                    Some(1.0)
                } else if left {
                    Some(-1.0)
                } else {
                    None
                };

                // Which way the selected row moves. A click counts as "forward"
                // so that a click steps a row exactly as the right arrow does;
                // the rows that walk a list rather than add a delta take this
                // bool and not `d`, because their list has an order but no
                // direction to add 1.0 to.
                let forward = right || clicked_row.is_some();
                if let Some(d) = step {
                    match game.settings_sel {
                        0 => {
                            game.settings.sensitivity =
                                (game.settings.sensitivity + d * 0.10).clamp(0.30, 5.0)
                        }
                        1 => step_dpi(&mut game.settings, forward),
                        2 => {
                            game.settings.volume = (game.settings.volume + d * 0.05).clamp(0.0, 1.0)
                        }
                        3 => game.settings.aim_toggle = !game.settings.aim_toggle,
                        4 => game.settings.lean_toggle = !game.settings.lean_toggle,
                        5 => game.settings.invert_y = !game.settings.invert_y,
                        6 => game.settings.show_fps = !game.settings.show_fps,
                        7 => game.settings.fullscreen = !game.settings.fullscreen,
                        8 => step_resolution(&mut game.settings, forward),
                        9 => i18n::set_lang(i18n::lang().next()),
                        _ => {}
                    }
                    audio.play(Sfx::UiMove);
                    // One exit for every row: the runtime side effects and the
                    // file are both written from the finished settings, so a row
                    // that forgets to save cannot exist.
                    apply_settings(&mut game, &mut audio);
                }
                let enter = is_key_pressed(KeyCode::Enter) || is_key_pressed(KeyCode::KpEnter);
                if (enter && game.settings_sel == back) || clicked(&menu_rects, back) {
                    audio.play(Sfx::UiBack);
                    game.phase = Phase::Menu;
                    game.menu_sel = 0;
                }
            }

            Phase::Armory => {
                pointer::clip(false);
                pointer::show(true);
                // Tabs 0/1 edit one weapon kind; tab 2 is the loadout, and what
                // it shows is the gun going in first.
                let tab = game.armory_tab.min(2);
                let kind = armory_kind(game.armory_kind);
                let shown = if tab < 2 {
                    kind
                } else {
                    game.kit
                        .primary
                        .or(game.kit.secondary)
                        .unwrap_or(weapon::WeaponKind::Rifle)
                };
                let def = weapon::def(shown);
                let build = game.kit.builds[shown.index()].clone();

                // Rows: the tab, then whichever list the tab owns, and the way
                // out. The step handler below reads them in this same order.
                let mut rows: Vec<String> = vec![format!(
                    "{}  {}",
                    i18n::t(Key::ArmoryTab),
                    match tab {
                        0 => i18n::t(Key::ModsTab),
                        1 => i18n::t(Key::PaintTab),
                        _ => i18n::t(Key::LoadoutTab),
                    }
                )];
                let slots = weapon::reachable_slots(def, &build);
                let paint = game.kit.paints[shown.index()];
                let mags = pick("mags", "匣");
                match tab {
                    0 => {
                        rows.push(format!(
                            "{}  {}",
                            i18n::t(Key::ArmoryWeapon),
                            i18n::t(kind.name())
                        ));
                        for slot in &slots {
                            let name = weapon::slot_label(slot.id)
                                .map(|(en, zh)| pick(en, zh))
                                .unwrap_or(slot.id.0);
                            rows.push(format!("{name}  {}", fitted_name(&build, slot.id)));
                        }
                    }
                    1 => {
                        rows.push(format!(
                            "{}  {}",
                            i18n::t(Key::ArmoryWeapon),
                            i18n::t(kind.name())
                        ));
                        rows.push(format!(
                            "{}  {}",
                            i18n::t(Key::PaintTab),
                            pick(paint.finish().name_en, paint.finish().name_zh)
                        ));
                        rows.push(format!(
                            "{}  {:.0}%",
                            i18n::t(Key::WearLabel),
                            paint.wear * 100.0
                        ));
                        rows.push(format!("{}  #{}", i18n::t(Key::SeedLabel), paint.seed));
                    }
                    _ => {
                        let gun = |slot: Option<weapon::WeaponKind>| match slot {
                            Some(k) => i18n::t(k.name()).to_string(),
                            None => i18n::t(Key::PartNone).to_string(),
                        };
                        rows.push(format!(
                            "{}  {}",
                            i18n::t(Key::PrimaryWeapon),
                            gun(game.kit.primary)
                        ));
                        rows.push(format!(
                            "{}  {}",
                            i18n::t(Key::SecondaryWeapon),
                            gun(game.kit.secondary)
                        ));
                        rows.push(format!(
                            "{}  {} {mags}",
                            i18n::t(Key::PrimaryMags),
                            game.kit.ammo_mags[0]
                        ));
                        rows.push(format!(
                            "{}  {} {mags}",
                            i18n::t(Key::SecondaryMags),
                            game.kit.ammo_mags[1]
                        ));
                        rows.push(format!(
                            "{}  {}",
                            i18n::t(Key::GrenadeFrag),
                            game.kit.grenades[0]
                        ));
                        rows.push(format!(
                            "{}  {}",
                            i18n::t(Key::GrenadeSmoke),
                            game.kit.grenades[1]
                        ));
                        for (i, key) in [Key::MedBandage, Key::MedKit, Key::MedPainkiller]
                            .iter()
                            .enumerate()
                        {
                            rows.push(format!("{}  {}", i18n::t(*key), game.kit.meds[i]));
                        }
                    }
                }
                rows.push(i18n::t(Key::Back).to_string());
                let back = rows.len() - 1;
                let items: Vec<hud::MenuItem> = rows
                    .iter()
                    .map(|r| hud::MenuItem {
                        label: r.as_str(),
                        enabled: true,
                    })
                    .collect();

                // The parts tab is a bench rather than a row list: the slot
                // column on the left, the candidates of the slot under the
                // cursor in the middle, and the weapon with the numbers a
                // candidate would move on the right. A weapon that offers no
                // slot at all has no bench to draw and keeps the plain list.
                let bench = tab == 0 && !slots.is_empty();

                // Keys. The arrows come first: on the bench ←/→ move between
                // the two columns, and the step tables at the end of this arm
                // only ever see an arrow the column move did not use. Then
                // up/down, which walk the slot column — the middle column is not
                // built until the cursor that decides it has stopped moving.
                let (mut left, mut right) = (
                    is_key_pressed(KeyCode::Left),
                    is_key_pressed(KeyCode::Right),
                );
                let enter = is_key_pressed(KeyCode::Enter) || is_key_pressed(KeyCode::KpEnter);
                if bench {
                    match game.armory_focus {
                        hud::ArmoryColumn::Parts => {
                            if left {
                                game.armory_focus = hud::ArmoryColumn::Slots;
                                audio.play(Sfx::UiMove);
                            }
                            // → has nothing further to open: the candidates for
                            // this slot are already on screen.
                            left = false;
                            right = false;
                        }
                        hud::ArmoryColumn::Slots => {
                            if right && (2..back).contains(&game.armory_sel) {
                                game.armory_focus = hud::ArmoryColumn::Parts;
                                audio.play(Sfx::UiMove);
                                right = false;
                            }
                        }
                    }
                }
                let in_parts = bench && game.armory_focus == hud::ArmoryColumn::Parts;
                if !in_parts {
                    menu_nav(&mut game.armory_sel, items.len(), &audio);
                }

                // Where the bench points: the slot row the cursor is on, what
                // fits that slot, and the caret's row in that list. The list is
                // walked every frame rather than cached, because fitting a part
                // can open a slot of its own and taking one off can close it
                // again.
                let mut focus_slot: Option<weapon::SlotId> = None;
                let mut choices: Vec<Option<weapon::PartId>> = Vec::new();
                let mut caret = 0usize;
                if bench {
                    if (2..back).contains(&game.armory_sel) {
                        game.armory_slot = game.armory_sel - 2;
                    }
                    game.armory_slot = game.armory_slot.min(slots.len().saturating_sub(1));
                    if let Some(slot) = slots.get(game.armory_slot) {
                        focus_slot = Some(slot.id);
                        choices = part_choices(def, &build, slot.id).unwrap_or_default();
                        // The caret is remembered with the slot it was put in: a
                        // slot the player has not been in yet opens on whatever
                        // it holds — the answer to "what is on the gun" — rather
                        // than on the same row number of another slot's list.
                        caret = match game.armory_part {
                            Some((was, at)) if was == slot.id => {
                                at.min(choices.len().saturating_sub(1))
                            }
                            _ => choices
                                .iter()
                                .position(|c| *c == fitted_part(&build, slot.id))
                                .unwrap_or(0),
                        };
                    }
                }
                if in_parts {
                    menu_nav(&mut caret, choices.len(), &audio);
                }
                if let Some(slot) = focus_slot {
                    game.armory_part = Some((slot, caret));
                }

                // The candidate the panel describes: the one under the pointer,
                // or the caret while the keyboard is in that column. Pointing at
                // one only asks what it would do — the fitting itself is the
                // click and the Enter at the end of this arm.
                let cursor: Option<usize> = if bench {
                    game.armory_hover_part
                        .filter(|i| *i < choices.len())
                        .or_else(|| in_parts.then_some(caret))
                } else {
                    None
                };
                // The build the cursor would land, kept between frames: a part
                // that declares a range has to preview the number it will really
                // have, and the preview mesh is rebuilt only when the picture
                // actually changes.
                let wanted = focus_slot
                    .and_then(|slot| cursor.map(|candidate| (shown.index(), slot, candidate)));
                let cached = game
                    .armory_preview
                    .as_ref()
                    .map(|p| (p.kind, p.slot, p.candidate));
                if wanted != cached {
                    game.armory_preview = match wanted {
                        Some((at, slot, candidate)) => {
                            choices.get(candidate).copied().map(|part| {
                                let roll = seat_roll(&mut game.rng, &build, slot, part);
                                ArmoryPreview {
                                    kind: at,
                                    slot,
                                    candidate,
                                    build: seated(&build, slot, part, roll),
                                }
                            })
                        }
                        None => None,
                    };
                    game.armory_dirty = true;
                }

                clear_background(Color::new(0.03, 0.04, 0.06, 1.0));
                draw_sky(Color::new(0.06, 0.07, 0.11, 1.0));

                // The preview is the whole weapon recoloured, so it is rebuilt
                // when a choice moves — a candidate under the cursor is a choice
                // being weighed, and the well has to show it — and merely redrawn
                // while the screen is only being looked at.
                if game.armory_dirty {
                    let fitted = match game.armory_preview.as_ref().filter(|_| bench) {
                        Some(p) => &p.build,
                        None => &game.kit.builds[shown.index()],
                    };
                    game.armory_mesh = Some(viewmodel::preview_mesh(shown, fitted, paint));
                    game.armory_dirty = false;
                }
                // The numbers: the build in hand against the issued weapon, or —
                // while a candidate is under the cursor — that candidate against
                // the build in hand, old → new. The second reading is the one a
                // part is on the bench to answer.
                let issued = weapon::WeaponBuild::empty();
                let (fitted, against, weigh) = match game.armory_preview.as_ref().filter(|_| bench)
                {
                    Some(p) => (&p.build, &build, true),
                    None => (&build, &issued, false),
                };
                let stats = armory_stats(shown, def, fitted, against, weigh);
                let rects = {
                    let note = if tab == 1 {
                        wear_tier(game.kit.paints[shown.index()].wear)
                    } else if bench {
                        i18n::t(Key::ArmoryBenchHint)
                    } else {
                        ""
                    };
                    let held = focus_slot.and_then(|slot| fitted_part(&build, slot));
                    let parts: Vec<hud::ArmoryPart> = choices
                        .iter()
                        .map(|c| hud::ArmoryPart {
                            label: match c {
                                Some(id) => weapon::part_label(*id)
                                    .map(|(en, zh)| pick(en, zh))
                                    .unwrap_or(id.0),
                                None => i18n::t(Key::PartNone),
                            },
                            fitted: *c == held,
                        })
                        .collect();
                    let bench_view = bench.then(|| hud::ArmoryBench {
                        parts: &parts,
                        cursor,
                        column: game.armory_focus,
                    });
                    let view = hud::ArmoryView {
                        weapon: i18n::t(shown.name()),
                        rows: &items,
                        bench: bench_view.as_ref(),
                        stats: &stats,
                        preview: game.armory_mesh.as_ref(),
                        note,
                    };
                    hud::draw_armory(&fonts, &view, game.armory_sel)
                };

                let moved = mouse_moved(&mut game.menu_mouse);
                let hover = rects.rows.iter().position(hovered_rect);
                game.armory_sel = hover_selection(game.armory_sel, hover, moved);
                game.armory_hover_part = rects.parts.iter().position(hovered_rect);
                let clicked_row = rects.rows.iter().position(clicked_rect);
                if let Some(i) = clicked_row {
                    game.armory_sel = i;
                }
                let clicked_part = rects.parts.iter().position(clicked_rect);
                if bench {
                    // A click on a slot row opens its candidates, the same thing
                    // → does. Two acts, not one: nothing on the bench is fitted
                    // by pointing at it, which is what clicking a row used to
                    // mean before the columns existed (reference §20.2).
                    if clicked_row.is_some_and(|i| (2..back).contains(&i)) {
                        game.armory_focus = hud::ArmoryColumn::Parts;
                        audio.play(Sfx::UiMove);
                    }
                    // Fitting: the click on a candidate and Enter on the one the
                    // caret is on are the same act, and they are the only acts
                    // that change the gun.
                    let pick = if in_parts && enter {
                        Some(caret)
                    } else {
                        clicked_part
                    };
                    if let (Some(slot), Some(i)) = (focus_slot, pick) {
                        if fit_candidate(&mut game, shown, slot, i, &choices) {
                            game.armory_dirty = true;
                            config::save(&config_of(&game));
                            audio.play(Sfx::UiClick);
                        }
                    }
                }

                // A click steps a row only where stepping is the row's job: the
                // section is a three-way switch and the weapon is a closed set
                // the whole screen is arranged around, while the other tabs'
                // rows are counts. The bench's slot rows open their candidates
                // instead of stepping, and a candidate is fitted by being
                // clicked, so no click here can change a part by accident.
                let stepped = match (bench, clicked_row) {
                    (true, Some(i)) => i < 2,
                    (false, Some(_)) => true,
                    _ => false,
                };
                let step = if clicked_row == Some(back) {
                    None
                } else if right || stepped {
                    Some(1.0)
                } else if left {
                    Some(-1.0)
                } else {
                    None
                };
                if let Some(d) = step {
                    let forward = right || stepped;
                    let changed = match tab {
                        2 => match game.armory_sel {
                            0 => {
                                game.armory_tab = if forward {
                                    (tab + 1) % 3
                                } else {
                                    (tab + 2) % 3
                                };
                                true
                            }
                            1 => {
                                game.kit.primary = step_kind_choice(game.kit.primary, forward);
                                true
                            }
                            2 => {
                                game.kit.secondary = step_kind_choice(game.kit.secondary, forward);
                                true
                            }
                            i @ 3..=4 => {
                                let at = i - 3;
                                let n = LOADOUT_MAGS_MAX + 1;
                                let cur = game.kit.ammo_mags[at].min(n - 1);
                                game.kit.ammo_mags[at] = if forward {
                                    (cur + 1) % n
                                } else {
                                    (cur + n - 1) % n
                                };
                                true
                            }
                            i @ 5..=6 => {
                                let at = i - 5;
                                let n = if at == 0 {
                                    LOADOUT_FRAGS_MAX + 1
                                } else {
                                    LOADOUT_SMOKES_MAX + 1
                                };
                                let cur = game.kit.grenades[at].min(n - 1);
                                game.kit.grenades[at] = if forward {
                                    (cur + 1) % n
                                } else {
                                    (cur + n - 1) % n
                                };
                                true
                            }
                            i @ 7..=9 => {
                                let at = i - 7;
                                let n = LOADOUT_MEDS_MAX + 1;
                                let cur = game.kit.meds[at].min(n - 1);
                                game.kit.meds[at] = if forward {
                                    (cur + 1) % n
                                } else {
                                    (cur + n - 1) % n
                                };
                                true
                            }
                            _ => false,
                        },
                        _ => match game.armory_sel {
                            0 => {
                                game.armory_tab = if forward {
                                    (tab + 1) % 3
                                } else {
                                    (tab + 2) % 3
                                };
                                // The bench's columns belong to the tab being
                                // left behind.
                                game.armory_focus = hud::ArmoryColumn::Slots;
                                true
                            }
                            1 => {
                                let n = ARMORY_KINDS.len();
                                let cur = game.armory_kind.min(n - 1);
                                game.armory_kind = if forward {
                                    (cur + 1) % n
                                } else {
                                    (cur + n - 1) % n
                                };
                                true
                            }
                            i if i >= 2 && i < back => {
                                if tab == 0 {
                                    // The bench has no row that steps: a slot
                                    // row opens its candidates, which the arrows
                                    // above have already handled.
                                    false
                                } else {
                                    let at = kind.index();
                                    match i - 2 {
                                        0 => {
                                            let n = viewmodel::FINISHES.len();
                                            let cur = game.kit.paints[at].finish.min(n - 1);
                                            let next = if forward {
                                                (cur + 1) % n
                                            } else {
                                                (cur + n - 1) % n
                                            };
                                            game.kit.paints[at] = viewmodel::Paint {
                                                finish: next,
                                                wear: roll_wear(&mut game.rng),
                                                seed: game.rng.u32(),
                                            };
                                            true
                                        }
                                        1 => {
                                            let wear = (game.kit.paints[at].wear + d * 0.05)
                                                .clamp(0.0, 1.0);
                                            game.kit.paints[at].wear = wear;
                                            true
                                        }
                                        _ => {
                                            game.kit.paints[at].seed = game.rng.u32();
                                            true
                                        }
                                    }
                                }
                            }
                            _ => false,
                        },
                    };
                    if changed {
                        game.armory_dirty = true;
                        config::save(&config_of(&game));
                        audio.play(Sfx::UiMove);
                    }
                }
                // Enter on the exit row leaves the screen; in the bench's
                // candidate column it belongs to the candidate, not the exit.
                if (enter && !in_parts && game.armory_sel == back)
                    || is_key_pressed(KeyCode::Escape)
                    || clicked(&rects.rows, back)
                {
                    audio.play(Sfx::UiBack);
                    game.phase = Phase::Menu;
                    game.menu_sel = 0;
                }
            }

            Phase::Playing => {
                if is_key_pressed(KeyCode::Escape) && game.overlay == Overlay::None {
                    audio.play(Sfx::UiBack);
                    game.phase = Phase::Paused;
                    game.menu_sel = 0;
                } else {
                    handle_overlay_keys(&mut game, &audio);
                    if game.overlay == Overlay::None {
                        pointer::clip(true);
                        pointer::show(false);
                        // Hit-stop slows the world, not the game: the pointer,
                        // the input edges and the HUD all keep the real clock,
                        // and only `simulate` sees the scaled rate. Timed here
                        // rather than inside `simulate` because it is the one
                        // timer that must not be slowed by itself.
                        let world_dt = if game.hitstop > 0.0 {
                            dt * HITSTOP_SCALE
                        } else {
                            dt
                        };
                        game.hitstop = (game.hitstop - dt).max(0.0);
                        simulate(&mut game, &mut audio, world_dt);
                    } else {
                        // The world holds still while a screen is open.
                        pointer::clip(false);
                        pointer::show(true);
                        game.effects.update(dt, &game.level);
                    }
                    render_world(&mut game, &fonts);
                }
            }

            Phase::Paused => {
                pointer::clip(false);
                pointer::show(true);
                clear_background(Color::new(0.02, 0.03, 0.04, 1.0));
                draw_rectangle(
                    0.0,
                    0.0,
                    screen_width(),
                    screen_height(),
                    Color::new(0.0, 0.0, 0.0, 0.55),
                );
                // The manual opens over the pause screen the same way it does
                // over the main menu, and Esc puts the pause screen back.
                if handle_manual_keys(&mut game, &audio, true) {
                    draw_manual(&game, &fonts);
                } else {
                    let items = [
                        hud::MenuItem {
                            label: i18n::t(Key::Resume),
                            enabled: true,
                        },
                        hud::MenuItem {
                            label: i18n::t(Key::Restart),
                            enabled: true,
                        },
                        hud::MenuItem {
                            label: i18n::t(Key::HowToPlay),
                            enabled: true,
                        },
                        hud::MenuItem {
                            label: i18n::t(Key::Codex),
                            enabled: true,
                        },
                        hud::MenuItem {
                            label: i18n::t(Key::MainMenu),
                            enabled: true,
                        },
                    ];
                    menu_nav(&mut game.menu_sel, items.len(), &audio);
                    menu_rects =
                        hud::draw_menu(&fonts, i18n::t(Key::Paused), None, &items, game.menu_sel);
                    let moved = mouse_moved(&mut game.menu_mouse);
                    let hover = menu_rects.iter().position(hovered_rect);
                    game.menu_sel = hover_selection(game.menu_sel, hover, moved);
                    let enter = is_key_pressed(KeyCode::Enter) || is_key_pressed(KeyCode::KpEnter);
                    for i in 0..items.len() {
                        if (enter && game.menu_sel == i) || clicked(&menu_rects, i) {
                            audio.play(Sfx::UiClick);
                            match i {
                                0 => game.resume_play(),
                                1 => game.start(game.mission_index),
                                2 => game.open_menu_manual(0),
                                3 => game.open_menu_manual(1),
                                _ => {
                                    game.phase = Phase::Menu;
                                    game.menu_sel = 0;
                                }
                            }
                            break;
                        }
                    }
                }
            }

            Phase::Result => {
                pointer::clip(false);
                pointer::show(true);
                let success = game.objective_done && game.player.alive;
                let rows = [
                    (
                        i18n::t(Key::Wave),
                        format!("{} / {}", game.wave_display(), game.mission.waves),
                    ),
                    (i18n::t(Key::Kills), format!("{}", game.killed)),
                    (i18n::t(Key::Score), format!("{}", game.score)),
                    if game.result_new_best {
                        (i18n::t(Key::NewBest), format!("{}", game.score))
                    } else {
                        (i18n::t(Key::Best), format!("{}", game.result_best))
                    },
                    (i18n::t(Key::Time), format_time(game.elapsed)),
                    (
                        i18n::t(Key::Accuracy),
                        if game.shots_fired == 0 {
                            "—".to_string()
                        } else {
                            format!(
                                "{:.0}%",
                                100.0 * game.shots_hit as f32 / game.shots_fired as f32
                            )
                        },
                    ),
                ];
                let has_next = success && game.mission_index + 1 < campaign::count();
                let mut labels: Vec<hud::MenuItem> = Vec::new();
                let next_label;
                if has_next {
                    next_label = i18n::t(Key::NextMission);
                    labels.push(hud::MenuItem {
                        label: next_label,
                        enabled: true,
                    });
                }
                labels.push(hud::MenuItem {
                    label: i18n::t(Key::RetryMission),
                    enabled: true,
                });
                labels.push(hud::MenuItem {
                    label: i18n::t(Key::MainMenu),
                    enabled: true,
                });

                menu_nav(&mut game.result_sel, labels.len(), &audio);
                clear_background(Color::new(0.03, 0.04, 0.06, 1.0));
                draw_sky(Color::new(0.06, 0.07, 0.11, 1.0));
                let title = if success {
                    if has_next {
                        i18n::t(Key::MissionComplete)
                    } else {
                        i18n::t(Key::CampaignCleared)
                    }
                } else if game.player.alive {
                    i18n::t(Key::MissionFailed)
                } else {
                    i18n::t(Key::YouWereKilled)
                };
                menu_rects =
                    hud::draw_result(&fonts, title, success, &rows, &labels, game.result_sel);
                let moved = mouse_moved(&mut game.menu_mouse);
                let hover = menu_rects.iter().position(hovered_rect);
                game.result_sel = hover_selection(game.result_sel, hover, moved);
                let enter = is_key_pressed(KeyCode::Enter) || is_key_pressed(KeyCode::KpEnter);
                for i in 0..labels.len() {
                    if !((enter && game.result_sel == i) || clicked(&menu_rects, i)) {
                        continue;
                    }
                    audio.play(Sfx::UiClick);
                    let is_next = has_next && i == 0;
                    let is_retry = i == if has_next { 1 } else { 0 };
                    if is_next {
                        game.unlocked = game.unlocked.max(game.mission_index + 2);
                        save_progress(&game);
                        game.start(game.mission_index + 1);
                    } else if is_retry {
                        game.start(game.mission_index);
                    } else {
                        game.phase = Phase::Menu;
                        game.menu_sel = 0;
                    }
                    break;
                }
            }
        }

        if game.settings.show_fps {
            fonts.text(
                &format!("{} FPS", get_fps()),
                10.0,
                screen_height() - 10.0,
                18.0,
                Color::new(0.6, 0.7, 0.8, 1.0),
            );
        }
        next_frame().await
    }
}

fn format_time(secs: f32) -> String {
    let s = secs.max(0.0) as u32;
    format!("{}:{:02}", s / 60, s % 60)
}

/// Tab / M / J / H / K, the manual's page keys, and the Esc-to-close behaviour
/// for the full-screen overlays.
fn handle_overlay_keys(game: &mut Game, audio: &audio::Audio) {
    let was_open = game.overlay != Overlay::None;
    if is_key_pressed(KeyCode::Tab) {
        game.overlay = if game.overlay == Overlay::Inventory {
            Overlay::None
        } else {
            Overlay::Inventory
        };
        audio.play(Sfx::UiClick);
    }
    if is_key_pressed(KeyCode::M) {
        game.overlay = if game.overlay == Overlay::Map {
            Overlay::None
        } else {
            Overlay::Map
        };
        audio.play(Sfx::UiClick);
    }
    if is_key_pressed(KeyCode::J) {
        game.overlay = if game.overlay == Overlay::Mission {
            Overlay::None
        } else {
            Overlay::Mission
        };
        audio.play(Sfx::UiClick);
    }
    // The manual, on the same overlay slot as the other screens and on the
    // same keys as it answers to over a menu. Esc is left to the line below,
    // which closes whichever screen is up; this one has already cleared the
    // overlay by then if the manual was the one showing.
    handle_manual_keys(game, audio, false);
    if is_key_pressed(KeyCode::Escape) && game.overlay != Overlay::None {
        game.overlay = Overlay::None;
        audio.play(Sfx::UiBack);
    }
    // Only the manual has a built page to let go of, and only while it is the
    // screen on top — one of the other keys above may just have taken the
    // overlay from it. For the rest this is a no-op, and the rows are built
    // again the next time it opens.
    if game.overlay != Overlay::Manual {
        game.close_manual();
    }
    // The pointer roamed free while the overlay was up; start a fresh baseline
    // so closing it does not snap the view.
    if was_open && game.overlay == Overlay::None {
        game.mouse_ready = false;
    }
}

/// The manual's own keys, in either place it opens.
///
/// `H` and `K` open it on the controls and the codex, `←`/`→` turn the page and
/// Esc shuts it. Only the slot that keeps it up differs between the two places
/// — the field's overlay, which is what holds the simulation still behind it,
/// and `Game::menu_manual` over the menus, which have no world to hold still —
/// so that is the one thing the caller passes in. Returns whether the manual is
/// showing afterwards.
fn handle_manual_keys(game: &mut Game, audio: &audio::Audio, over_menu: bool) -> bool {
    let was_open = if over_menu {
        game.menu_manual
    } else {
        game.overlay == Overlay::Manual
    };
    let jump = manual_jump_key();
    let (open, page) = manual_after_keys(
        was_open,
        game.manual_page,
        jump,
        manual_turn_key(),
        is_key_pressed(KeyCode::Escape),
    );
    if open {
        // Jumping to the page that is already showing still answers the key
        // with a move, rather than looking like the key did nothing.
        if jump.is_some() || !was_open || page != game.manual_page {
            game.show_manual_page(page);
            audio.play(if was_open { Sfx::UiMove } else { Sfx::UiClick });
        }
    } else if was_open {
        game.close_manual();
        audio.play(Sfx::UiBack);
    }
    if over_menu {
        game.menu_manual = open;
    } else if open {
        game.overlay = Overlay::Manual;
    }
    open
}

/// Seconds between motes of suspended matter in a drowned arena.
const MOTE_PERIOD: f32 = 0.07;

/// One mote for a drowned arena: slow, small, and faint enough to read as silt
/// in the water rather than as snow.
///
/// Motes are spawned in a shell around the eye rather than anywhere in the
/// level: the camera sees a few dozen metres, so matter that drifted out of
/// that bubble is invisible matter, and this keeps the cloud centred on where
/// it is read. It costs no collision either — a mote spawned inside a wall is
/// simply drawn behind it and never seen.
fn spawn_mote(game: &mut Game) {
    let eye = game.player.eye();
    let rng = &mut game.rng;
    let dir = vec3(rng.signed(), rng.signed(), rng.signed()).normalize_or_zero();
    let drift = vec3(
        rng.signed() * 0.12,
        -rng.range(0.0, 0.10),
        rng.signed() * 0.12,
    );
    let life = rng.range(7.0, 13.0);
    game.effects.particles.push(effects::Particle {
        pos: eye + dir * rng.range(3.5, 18.0),
        vel: drift,
        life,
        max_life: life,
        // A few centimetres across and faint: a mote is a speck that catches
        // the light, which is what suspended silt looks like — anything bigger
        // reads as debris, and anything nearer than the shell's inner edge
        // reads as a floating cube.
        size: rng.range(0.02, 0.04),
        color: Color::new(0.58, 0.76, 0.72, 0.38),
        gravity: 0.0,
        drag: 0.0,
        bounce: false,
    });
}

fn simulate(game: &mut Game, audio: &mut audio::Audio, dt: f32) {
    // --- suspended matter ----------------------------------------------------
    // Only the drowned city has anything to float in. Spawned here, ahead of
    // the input paths, because the motes belong to the world rather than to
    // the player: silt drifts past a corpse too.
    if game.level.sunken {
        game.mote_t -= dt;
        while game.mote_t <= 0.0 {
            game.mote_t += MOTE_PERIOD;
            spawn_mote(game);
        }
    }

    // A dead player is a spectator. `alive` gates every input path below —
    // look, stance, movement, weapon handling, firing, grenades and medical
    // items — because the two seconds before the result screen used to be two
    // seconds of running, shooting and scoring kills as a corpse. Everything
    // from the spawning pass down still runs, so the world keeps ticking: the
    // death animation plays out, in-flight grenades land, and the death timer
    // still reaches the result screen.
    let alive = game.player.alive;
    let alt = is_key_down(KeyCode::LeftAlt) || is_key_down(KeyCode::RightAlt);
    let aiming = if !alive {
        false
    } else if game.settings.aim_toggle {
        if is_mouse_button_pressed(MouseButton::Right) {
            game.aim_on = !game.aim_on;
        }
        game.aim_on
    } else {
        is_mouse_button_down(MouseButton::Right)
    };
    let free_look = alive && is_mouse_button_down(MouseButton::Middle);

    // --- look ---------------------------------------------------------------
    // Deltas are computed here rather than with `mouse_delta_position`, because
    // the pointer is recentred every frame and owning the baseline is what lets
    // us rebase it to the centre we just moved to. Cancelling the movement
    // instead would feed the cancel back in as the next frame's delta and the
    // view would drift until it pointed at the sky.
    // A radial menu owns the pointer while it is open. Recentring — which is
    // what makes mouse look work at all — parks the cursor a few pixels from
    // the middle, so a sector test with any dead zone at all could never fire
    // and the grenade and medical wheels were unusable.
    let wheel_open = game.wheel.is_some();

    let (mx, my) = mouse_position();
    let cur = vec2(mx, my);
    if !game.mouse_ready {
        game.last_mouse = cur;
        game.mouse_ready = true;
    }
    let delta = cur - game.last_mouse;
    game.last_mouse = cur;
    if !wheel_open && let Some(landed) = pointer::recenter(cur) {
        // Rebase to where the pointer really ended up, not to a nominal centre.
        game.last_mouse = landed;
    }

    let invert = if game.settings.invert_y { -1.0 } else { 1.0 };
    // Pointer travel is the unit — `mouse_position` deltas are pixels of it —
    // so the feel is the same in a window and in fullscreen, at any resolution.
    // See `LOOK_RADIANS_PER_COUNT`, which is also what the settings page
    // derives its cm/360 readout from.
    let sens = game.settings.sensitivity
        * LOOK_RADIANS_PER_COUNT
        * (1.0 - game.player.ads * 0.5)
        * match game.player.stance() {
            Stance::Stand => 1.0,
            Stance::Crouch => 0.88,
            Stance::Prone => 0.72,
        };
    // Moving right must turn right: increasing yaw turns left in this
    // convention, so both axes subtract.
    // While a menu is open the mouse steers the menu, not the view. A corpse
    // does not turn its head either.
    let look = if wheel_open || !alive {
        Vec2::ZERO
    } else {
        vec2(-delta.x * sens, -delta.y * sens * invert)
    };
    if free_look {
        game.player.free_yaw = (game.player.free_yaw + look.x).clamp(-2.6, 2.6);
        game.player.free_pitch = (game.player.free_pitch + look.y).clamp(-1.2, 1.2);
    } else {
        game.player.yaw += look.x;
        game.player.pitch = (game.player.pitch + look.y).clamp(-1.53, 1.53);
        // Pulling down against the climb has to eat the accumulated recoil, or
        // the automatic fallback stacks on top of the player's own correction
        // and lands the aim below where the burst started. Only here: in free
        // look the player is turning their head, not fighting the muzzle.
        game.player.compensate_recoil(look.y, look.x);
    }

    // --- stance -------------------------------------------------------------
    if alive && is_key_pressed(KeyCode::Z) {
        game.stance = if game.stance == Stance::Prone {
            Stance::Stand
        } else {
            Stance::Prone
        };
    }
    if alive && is_key_pressed(KeyCode::C) {
        game.stance = if game.stance == Stance::Crouch {
            Stance::Stand
        } else {
            Stance::Crouch
        };
    }

    // Lean follows the same rule as aiming, so the two never disagree: in
    // toggle mode a tap latches the lean and another tap drops it, in hold mode
    // it lasts as long as the key does. A dead body lies where it fell.
    if !alive {
        game.lean_cmd = 0.0;
    } else if game.settings.lean_toggle {
        if is_key_pressed(KeyCode::Q) {
            game.lean_cmd = if game.lean_cmd < -0.5 { 0.0 } else { -1.0 };
            audio.play(Sfx::UiMove);
        }
        if is_key_pressed(KeyCode::E) {
            game.lean_cmd = if game.lean_cmd > 0.5 { 0.0 } else { 1.0 };
            audio.play(Sfx::UiMove);
        }
    } else {
        game.lean_cmd = (is_key_down(KeyCode::E) as i32 - is_key_down(KeyCode::Q) as i32) as f32;
    }
    let lean = game.lean_cmd;

    let forward = (is_key_down(KeyCode::W) as i32 - is_key_down(KeyCode::S) as i32) as f32;
    let sprint_toggled =
        alive && (is_key_pressed(KeyCode::LeftShift) || is_key_pressed(KeyCode::RightShift));
    game.sprint = sprint_state(game.sprint, sprint_toggled, forward, game.player.ads > 0.4);
    // Sprinting forces you upright and drops any latched lean, otherwise a
    // toggled lean would silently block sprinting until you noticed.
    if alive && game.sprint && forward > SPRINT_FORWARD {
        game.stance = Stance::Stand;
        game.lean_cmd = 0.0;
    }

    // A corpse takes no input, but it is still simulated: gravity finishes
    // bringing it to the floor and the camera stops where the body is.
    let input = if alive {
        MoveInput {
            forward,
            right: (is_key_down(KeyCode::D) as i32 - is_key_down(KeyCode::A) as i32) as f32,
            jump: is_key_down(KeyCode::Space),
            sprint: game.sprint,
            aim: aiming,
            walk: is_key_down(KeyCode::CapsLock),
            lean,
            hold_breath: alt && aiming,
            free_look,
            stance: game.stance,
            speed_scale: game.arsenal.move_scale(),
        }
    } else {
        MoveInput {
            stance: game.stance,
            ..Default::default()
        }
    };
    game.player.ads_fov = game.arsenal.active_weapon().stats().ads_fov;
    game.player.update(&game.level, dt, &input);
    game.level.update_smoke(dt);

    if game.player.landing > 4.0 {
        audio.play(Sfx::Footstep);
    }

    // --- weapon selection ---------------------------------------------------
    for (i, k) in [KeyCode::Key1, KeyCode::Key2, KeyCode::Key3, KeyCode::Key4]
        .iter()
        .enumerate()
    {
        if alive && is_key_pressed(*k) {
            game.select_slot(i);
            audio.play(Sfx::UiMove);
        }
    }
    let (_, wheel_scroll) = mouse_wheel();
    if alive && wheel_scroll.abs() > 0.0 && game.wheel.is_none() {
        game.arsenal.cycle(if wheel_scroll > 0.0 { 1 } else { -1 });
    }

    // --- fire mode / magazine check / inspect --------------------------------
    if alive && is_key_pressed(KeyCode::B) {
        if alt {
            let mode = game.arsenal.active_weapon().mode();
            game.say(format!(
                "{}: {}",
                i18n::t(Key::FireModeIs),
                i18n::t(mode.label())
            ));
            audio.play(Sfx::UiClick);
        } else {
            let mode = game.arsenal.active_weapon_mut().cycle_fire_mode();
            game.say(format!(
                "{}: {}",
                i18n::t(Key::FireModeChanged),
                i18n::t(mode.label())
            ));
            audio.play(Sfx::UiClick);
        }
    }
    if alive && is_key_pressed(KeyCode::T) && alt {
        let key = game.arsenal.active_weapon().mag_estimate();
        let rounds = game.arsenal.active_weapon().mag;
        game.arsenal.active_weapon_mut().mag_check_t = 2.2;
        game.say(format!(
            "{}: {} ({})",
            i18n::t(Key::CheckingMag),
            i18n::t(key),
            rounds
        ));
        audio.play(Sfx::ReloadEmpty);
    }
    if alive && is_key_pressed(KeyCode::U) {
        game.arsenal.active_weapon_mut().start_inspect();
    }

    // --- grenades -----------------------------------------------------------
    if alive && is_key_pressed(KeyCode::G) {
        game.wheel = Some(Wheel {
            kind: WheelKind::Grenade,
            selected: 0,
            held: 0.0,
        });
    }
    // Dying with a wheel open would otherwise leave the pointer captured by a
    // menu nobody can close any more.
    if !alive {
        game.wheel = None;
    }
    if let Some(w) = &mut game.wheel {
        w.held += dt;
    }
    if is_key_down(KeyCode::G)
        && let Some(w) = &mut game.wheel
        && w.held > 0.28
        && w.kind == WheelKind::Grenade
        && let Some(s) = wheel_selection(GRENADE_KINDS.len())
    {
        w.selected = s;
    }
    if is_key_released(KeyCode::G)
        && let Some(w) = game.wheel.take()
    {
        // The pointer was free while the wheel was open; take a fresh baseline
        // so the travel does not snap the view.
        game.mouse_ready = false;
        if w.held > 0.28 {
            game.throw_grenade(w.selected, audio);
        } else {
            game.throw_grenade(0, audio);
        }
    }

    // --- medical ------------------------------------------------------------
    if alive && is_key_pressed(KeyCode::V) {
        game.wheel = Some(Wheel {
            kind: WheelKind::Med,
            selected: 0,
            held: 0.0,
        });
    }
    if is_key_down(KeyCode::V)
        && let Some(w) = &mut game.wheel
        && w.held > 0.28
        && w.kind == WheelKind::Med
        && let Some(s) = wheel_selection(MED_KINDS.len())
    {
        w.selected = s;
    }
    if is_key_released(KeyCode::V)
        && let Some(w) = game.wheel.take()
    {
        game.mouse_ready = false;
        let pick = if w.held > 0.28 {
            w.selected
        } else {
            best_med(game)
        };
        game.begin_heal(pick, audio);
    }

    // --- healing channel ----------------------------------------------------
    if game.heal_t > 0.0 {
        if !alive {
            // The channel dies with the player, and quietly: there is nobody
            // left to read a message about it.
            game.heal_t = 0.0;
        } else if game.player.horizontal_speed() > 2.5 {
            game.heal_t = 0.0;
            game.say(i18n::t(Key::HealInterrupted));
        } else {
            game.heal_t -= dt;
            if game.heal_t <= 0.0 {
                game.heal_t = 0.0;
                let amount = MED_KINDS[game.heal_index].0;
                game.player.heal(amount);
                game.ammo_meds[game.heal_index] = game.ammo_meds[game.heal_index].saturating_sub(1);
                audio.play(Sfx::PickupHealth);
            }
        }
    }

    // --- firing -------------------------------------------------------------
    // The click that left the menu is still held on the first frame of the
    // mission, which used to fire a burst into the wall every time.
    if !is_mouse_button_down(MouseButton::Left) {
        game.trigger_armed = true;
    }
    let held = is_mouse_button_down(MouseButton::Left) && game.trigger_armed;
    let pressed = is_mouse_button_pressed(MouseButton::Left) && game.trigger_armed;
    let melee = game.arsenal.active_weapon().stats().melee;
    if alive && (held || pressed) {
        if melee || game.arsenal.active_weapon().stats().infinite_ammo {
            // A knife's hit sound is played inside the shot resolution when it
            // actually connects; playing it here fired 40 times a second while
            // the button was held, hit or miss.
            fire_player_shot(game, audio, pressed, held);
        } else if game.arsenal.active_weapon().mag > 0 {
            let kind = game.arsenal.active_weapon().kind;
            // Only sound off when a round actually left the barrel. Otherwise
            // the shots come twice as fast as the weapon fires, and keep coming
            // all through a reload.
            if fire_player_shot(game, audio, pressed, held) {
                audio.play(match kind {
                    WeaponKind::Rifle => Sfx::ShotRifle,
                    WeaponKind::Smg => Sfx::ShotSmg,
                    WeaponKind::Sniper => Sfx::ShotSniper,
                    WeaponKind::Lmg => Sfx::ShotLmg,
                    _ => Sfx::ShotShotgun,
                });
            }
        } else if pressed {
            audio.play(Sfx::ReloadEmpty);
            game.arsenal.active_weapon_mut().start_reload();
        }
    }

    if alive && is_key_pressed(KeyCode::R) && game.arsenal.active_weapon_mut().start_reload() {
        audio.play(Sfx::Reload);
    }

    game.arsenal.update(dt);
    // The viewmodel's sway was tuned against `mouse_delta_position`'s
    // normalised scale, so convert back before handing it over.
    let sway_delta = delta * 2.0 / screen_width().max(1.0);
    game.viewmodel
        .update(dt, sway_delta, &game.player, &game.arsenal);

    // --- waves --------------------------------------------------------------
    // The spawner is a clock now: a prep window, a release window that pays
    // the wave's points out one hostile at a time, then a clear check. See
    // `tick_waves`.
    let mission = game.mission;
    game.tick_waves(dt, audio);

    // --- enemy AI -----------------------------------------------------------
    let positions: Vec<Vec3> = game.enemies.iter().map(|e| e.pos).collect();
    let mut attacks: Vec<(Vec3, Attack)> = Vec::new();
    // A footfall is a pulse that `update` overwrites every frame, so it has to
    // be read here, on the frame it fires; the dust is emitted after the loop
    // because that is where the other fields of `game` are free again.
    let mut footfalls: Vec<Vec3> = Vec::new();
    for (i, e) in game.enemies.iter_mut().enumerate() {
        // The whole squad's positions, once — the enemy skips its own entry.
        // This used to allocate a filtered copy per enemy per frame.
        if let Some(atk) = e.update(&game.level, &game.player, &positions, i, dt, &mut game.rng) {
            attacks.push((e.pos, atk));
        }
        if e.take_footfall() {
            footfalls.push(e.pos);
        }
    }
    for at in footfalls {
        // A small, ground-hugging puff: enough that a unit crossing open ground
        // reads as walking, little enough that it never looks like a hit.
        game.effects.smoke(
            at + vec3(0.0, 0.05, 0.0),
            Color::new(0.45, 0.42, 0.36, 0.5),
            5,
            &mut game.rng,
        );
    }
    for (from, atk) in attacks {
        match atk {
            Attack::Shot {
                origin,
                dir,
                damage,
            } => {
                let wall = game
                    .level
                    .ray_world(origin, dir, weapon::MAX_RANGE)
                    .unwrap_or(weapon::MAX_RANGE);
                let mut end = origin + dir * wall;
                if let Some(t) = ray_player(&game.player, origin, dir)
                    && t > 0.02
                    && t < wall
                {
                    // `origin` is the eye the round left, not the feet `from`
                    // holds: both sit on the same column, so the flattened arc
                    // cannot tell them apart today, but the muzzle is the thing
                    // the indicator is naming. The hit point is passed along
                    // because *where* it landed decides whether the armour
                    // applies at all.
                    apply_player_damage(
                        &mut game.player,
                        &mut game.armor,
                        audio,
                        damage,
                        origin,
                        Some(origin + dir * t),
                    );
                    game.heal_t = 0.0;
                    // Stop the strand short of the eye rather than on it: a
                    // beam that ends on the camera plane smears the same way a
                    // player's own tracer does at its start (see `TRACER_NEAR`),
                    // and the last half metre is behind the crosshair anyway.
                    end = origin + dir * (t - TRACER_NEAR).max(0.05);
                }
                // Incoming fire is the wide, deep orange-red line, and it
                // carries a short bright head at the muzzle: width and shape
                // both say "that one is coming at you" before the colour does,
                // and the head marks the shooter even when the rest of the
                // tracer is lost against a bright wall.
                game.effects
                    .tracer(origin, end, Color::new(1.0, 0.32, 0.10, 1.0), 0.024);
                // The head is clamped to the shot itself, so a point-blank one
                // cannot leave a bright stub sticking out past its own hit.
                let head = (end - origin).length().min(0.6);
                game.effects.tracer(
                    origin,
                    origin + dir * head,
                    Color::new(1.0, 0.86, 0.55, 1.0),
                    0.034,
                );
                let d = (from - game.player.pos).length();
                audio.play_at(Sfx::EnemyShot, d);
            }
            Attack::Melee { damage } => {
                // A swing carries no origin of its own; the attacker's feet are
                // where it came from, and it lands on the body — there is no
                // "leg shot" to armour-test for a claw.
                apply_player_damage(&mut game.player, &mut game.armor, audio, damage, from, None);
                game.heal_t = 0.0;
            }
        }
    }

    // --- grenades -----------------------------------------------------------
    // --- sanity --------------------------------------------------------------
    // Only what you *look at* costs you: facing matters, and so does line of
    // sight. Turning away is a real defence, which is the point.
    let eye = game.player.eye();
    let look = game.player.look_dir();
    let mut drain = 0.0f32;
    for e in &game.enemies {
        if e.dead || e.sanity_drain <= 0.0 {
            continue;
        }
        let to = e.eye() - eye;
        let dist = to.length();
        if dist > 45.0 {
            continue;
        }
        let facing = to.normalize_or_zero().dot(look);
        if facing < 0.35 {
            continue;
        }
        if !game.level.line_of_sight(eye, e.eye()) {
            continue;
        }
        drain += e.sanity_drain * ((facing - 0.35) / 0.65);
    }
    if drain > 0.0 {
        game.player.sanity = (game.player.sanity - drain * dt).max(0.0);
    } else {
        game.player.sanity = (game.player.sanity + 4.0 * dt).min(100.0);
    }

    let blasts = game
        .grenades
        .update(dt, &mut game.level, &mut game.effects, &mut game.rng);
    for b in blasts {
        audio.play_at(Sfx::Explosion, (b.pos - game.player.pos).length());
        game.effects
            .smoke(b.pos, Color::new(0.9, 0.75, 0.5, 0.7), 26, &mut game.rng);
        game.effects.spark(
            b.pos,
            Vec3::Y,
            Color::new(1.0, 0.75, 0.3, 1.0),
            30,
            &mut game.rng,
        );
        // Fall off with distance and require line of sight, so corners work.
        for i in 0..game.enemies.len() {
            if game.enemies[i].dead {
                continue;
            }
            let centre = game.enemies[i].pos + vec3(0.0, game.enemies[i].height * 0.5, 0.0);
            let d = centre.distance(b.pos);
            if d > b.radius || !game.level.line_of_sight(b.pos, centre) {
                continue;
            }
            let falloff = 1.0 - (d / b.radius).powf(1.4);
            let dmg = b.damage * falloff.clamp(0.0, 1.0);
            // A blast carries full penetration: a pressure wave is not a
            // round, and there is nothing for plate to stop.
            if game.enemies[i].take_damage(dmg, 1.0) {
                kill_enemy(game, i, audio);
            }
        }
        let eye = game.player.eye();
        let d = eye.distance(b.pos);
        if d < b.radius && game.level.line_of_sight(b.pos, eye) {
            let falloff = 1.0 - (d / b.radius).powf(1.4);
            // The blast has no single muzzle: the grenade's own position is the
            // source the player has to turn toward — and, having no hit point
            // of its own, it lands on the body.
            apply_player_damage(
                &mut game.player,
                &mut game.armor,
                audio,
                b.damage * falloff,
                b.pos,
                None,
            );
            game.heal_t = 0.0;
        }
    }

    // --- pickups ------------------------------------------------------------
    for p in &mut game.pickups {
        p.update(dt);
    }
    let mut taken: Vec<usize> = Vec::new();
    for i in 0..game.pickups.len() {
        // Copy out of the pickup first: collecting one mutates the inventory,
        // which cannot happen while the list itself is borrowed.
        let (kind, amount, at) = {
            let p = &game.pickups[i];
            (p.kind, p.amount, p.pos)
        };
        let close = vec3(at.x - game.player.pos.x, 0.0, at.z - game.player.pos.z).length() < 1.1
            && (at.y - game.player.pos.y).abs() < 1.8;
        if !close {
            continue;
        }
        let label = match kind {
            PickupKind::Health => {
                // Same class of loss as the ammo crate below: `heal` reports how
                // much it actually restored, and at full health that is nothing,
                // so the crate used to be consumed for no gain.
                let restored = game.player.heal(amount);
                if restored <= 0.0 {
                    game.say(i18n::t(Key::HealthFull));
                    continue;
                }
                game.pickups[i].amount = amount - restored;
                audio.play(Sfx::PickupHealth);
                i18n::t(Key::PickHealth)
            }
            PickupKind::Ammo => {
                // `add_ammo` reports the rounds that actually fit. Taking none
                // of them — a full magazine, or a weapon that never consumes
                // ammunition — used to still play the sting and delete the
                // crate, quietly destroying its contents. There is no reserve
                // pool to put them in, so the crate stays where it is.
                let rounds = game.arsenal.active_weapon_mut().add_ammo(amount as u32);
                if rounds == 0 {
                    game.say(i18n::t(Key::AmmoFull));
                    continue;
                }
                // A partial take leaves the remainder behind for later.
                game.pickups[i].amount = amount - rounds as f32;
                audio.play(Sfx::PickupAmmo);
                i18n::t(Key::PickAmmo)
            }
            PickupKind::Armor => {
                let restored = (ARMOR_MAX - game.armor).clamp(0.0, amount);
                if restored <= 0.0 {
                    game.say(i18n::t(Key::ArmorFull));
                    continue;
                }
                game.armor += restored;
                game.pickups[i].amount = amount - restored;
                audio.play(Sfx::PickupAmmo);
                i18n::t(Key::PickArmor)
            }
        };
        game.say(label);
        // Ammo that could not all be carried keeps the crate in the world; the
        // rest are emptied on contact.
        if kind != PickupKind::Ammo || game.pickups[i].amount < 0.5 {
            taken.push(i);
        }
    }
    for i in taken.into_iter().rev() {
        game.pickups.remove(i);
    }

    // --- progression --------------------------------------------------------
    game.enemies.retain(|e| !e.is_removable());
    // Anything that walks into a bottomless pit is gone. Without this a fallen
    // enemy would keep falling forever and the mission could never complete.
    game.enemies
        .retain(|e| !game.level.swallowed_by_void(e.pos));
    if game.level.swallowed_by_void(game.player.pos) {
        game.player.health = 0.0;
        game.player.alive = false;
        if game.toast.is_none() {
            game.say(i18n::t(Key::AbyssTaken));
        }
    }
    game.effects.update(dt, &game.level);
    game.elapsed += dt;
    game.flow_t -= dt;
    if game.flow_t <= 0.0 {
        game.flow_t = 0.25;
        game.level.flow.update(game.player.pos);
    }

    // The mission clears when the wave clock says every wave has been
    // released and the field has been left empty.
    let cleared = matches!(game.wave_phase, WavePhase::Finished) && game.alive_enemies() == 0;
    let at_extraction = game
        .level
        .extraction
        .map(|x| vec3(x.x - game.player.pos.x, 0.0, x.z - game.player.pos.z).length() < 2.2)
        .unwrap_or(false);

    match mission.objective {
        Objective::Eliminate => {
            if cleared {
                game.objective_done = true;
            }
        }
        Objective::EliminateThenExtract => {
            if cleared && !game.extraction_open {
                game.extraction_open = true;
                game.banner = Some((i18n::t(Key::ObjReach).to_string(), 2.4));
                audio.play(Sfx::ObjectiveDone);
            }
            if game.extraction_open && at_extraction {
                game.objective_done = true;
            }
        }
        Objective::Survive => {
            game.survive_left -= dt;
            if game.survive_left <= 0.0 {
                game.survive_left = 0.0;
                if !game.extraction_open {
                    game.extraction_open = true;
                    game.banner = Some((i18n::t(Key::ObjReach).to_string(), 2.4));
                    audio.play(Sfx::ObjectiveDone);
                }
                if at_extraction {
                    game.objective_done = true;
                }
            }
        }
    }

    tick_toast(game, dt);

    if game.objective_done && game.player.alive {
        game.score += mission.score_bonus;
        game.settle_score();
        if game.mission_index + 1 < campaign::count() {
            game.unlocked = game.unlocked.max(game.mission_index + 2);
            save_progress(game);
        }
        game.phase = Phase::Result;
        game.result_sel = 0;
        audio.play(Sfx::MissionComplete);
        return;
    }
    if !game.player.alive {
        game.dead_timer += dt;
        if game.dead_timer > 2.0 {
            game.settle_score();
            game.phase = Phase::Result;
            game.result_sel = 0;
            audio.play(Sfx::MissionFailed);
        }
    }
}

fn tick_toast(game: &mut Game, dt: f32) {
    if let Some((_, t)) = &mut game.banner {
        *t -= dt;
        if *t <= 0.0 {
            game.banner = None;
        }
    }
    if let Some((_, t)) = &mut game.toast {
        *t -= dt;
        if *t <= 0.0 {
            game.toast = None;
        }
    }
}

/// Which sector of a radial menu the mouse is pointing at, if any.
fn wheel_selection(n: usize) -> Option<usize> {
    if n == 0 {
        return None;
    }
    let (mx, my) = mouse_position();
    let c = vec2(screen_width() * 0.5, screen_height() * 0.5);
    let d = vec2(mx, my) - c;
    if d.length() < 40.0 {
        return None;
    }
    // Sectors start at the top and run clockwise.
    let mut a = d.y.atan2(d.x) + std::f32::consts::FRAC_PI_2;
    if a < 0.0 {
        a += std::f32::consts::TAU;
    }
    let step = std::f32::consts::TAU / n as f32;
    Some(((a / step).floor() as usize).min(n - 1))
}

fn best_med(game: &Game) -> usize {
    // Prefer the cheapest item that will not waste healing.
    let missing = game.player.max_health - game.player.health;
    for (i, (amount, _, _)) in MED_KINDS.iter().enumerate() {
        if game.ammo_meds[i] > 0 && *amount <= missing + 10.0 {
            return i;
        }
    }
    MED_KINDS
        .iter()
        .enumerate()
        .find(|(i, _)| game.ammo_meds[*i] > 0)
        .map(|(i, _)| i)
        .unwrap_or(0)
}

/// Seconds of a damage readout's life that are not drawn.
///
/// `hud::draw_damage_numbers` has no alpha and draws at a fixed opacity, so the
/// fade has to live here. A hard cut for the last stretch reads better than
/// nothing: by then the number is already rising away from the hit (the rise is
/// `effects.rs`'s, which lifts the world point every frame), so the cut looks
/// like the readout expiring rather than blinking out mid-flight.
const DAMAGE_NUMBER_CUTOFF: f32 = 0.3;

/// A perspective view-projection built from `Camera3D`'s own constructors and
/// defaults, for the CPU-side projection macroquad 0.4.16 does not offer for
/// 3D cameras.
fn view_projection(cam: Vec3, target: Vec3, up: Vec3, fovy: f32, width: f32, height: f32) -> Mat4 {
    let aspect = width / height.max(1.0);
    // The near and far planes are `Camera3D::default`'s; the near plane only
    // has to keep the divide away from zero.
    Mat4::perspective_rh_gl(fovy, aspect, 0.01, 10000.0) * Mat4::look_at_rh(cam, target, up)
}

/// Where a world point lands in window pixels, or `None` when it is behind the
/// camera. GL's clip space gives `w = -z_view`, so a `w` at or below the near
/// plane is the point being on or behind the eye plane, where the perspective
/// divide is meaningless and the result would smear across the screen.
fn project_to_screen(view_proj: &Mat4, p: Vec3, width: f32, height: f32) -> Option<Vec2> {
    let clip = *view_proj * p.extend(1.0);
    if clip.w <= 0.01 {
        return None;
    }
    let ndc = clip.truncate() / clip.w;
    if !ndc.x.is_finite() || !ndc.y.is_finite() {
        return None;
    }
    // NDC is y-up with 1 at the top of the screen; window pixels are y-down.
    Some(vec2(
        (ndc.x * 0.5 + 0.5) * width,
        (0.5 - ndc.y * 0.5) * height,
    ))
}

/// Projects this frame's damage readouts out of the world and hands them to the
/// HUD. The list is rebuilt in the buffer the game owns, so the projection —
/// not the allocation — is what runs per frame.
fn draw_damage_numbers(game: &mut Game, fonts: &hud::Fonts, view_proj: &Mat4) {
    game.damage_labels.clear();
    for &(at, amount, left, head) in &game.effects.damage_numbers {
        if left <= DAMAGE_NUMBER_CUTOFF {
            continue;
        }
        if let Some(screen) = project_to_screen(view_proj, at, screen_width(), screen_height()) {
            game.damage_labels.push((screen, amount, head));
        }
    }
    hud::draw_damage_numbers(fonts, &game.damage_labels);
}

/// Health and sanity at which the screen starts telling the player they are
/// nearly gone. Shared by the red vignette and the world grade so the two
/// cannot disagree about when "nearly" begins.
const CRITICAL_HEALTH: f32 = 30.0;
const CRITICAL_SANITY: f32 = 0.25;

/// The most the world grade takes out, at zero health or sanity: half the
/// colour, so even the worst state stays readable.
const CRITICAL_DESATURATION: f32 = 0.5;

/// How far the world is pulled towards grey as the player nears death: zero at
/// the thresholds above, `CRITICAL_DESATURATION` at zero health or sanity.
///
/// Deliberately the world only: this rides the fog pass, and the viewmodel and
/// HUD are drawn after it keep their colour, because the signals a dying
/// player needs are the ones that must not fade.
fn critical_desaturation(health: f32, sanity: f32) -> f32 {
    let health_t = 1.0 - health / CRITICAL_HEALTH;
    let sanity_t = 1.0 - sanity / CRITICAL_SANITY;
    health_t.max(sanity_t).clamp(0.0, 1.0) * CRITICAL_DESATURATION
}

fn render_world(game: &mut Game, fonts: &hud::Fonts) {
    // The same thresholds the vignette lights up at; the fog pass below greys
    // the world by this much.
    let desat = critical_desaturation(game.player.health, game.player.sanity_ratio());
    let shake = game.player.shake_offset();
    let eye = game.player.eye();
    let dir = game.player.look_dir();
    let fovy = game.player.fovy();

    // Camera roll is applied by tilting the up vector, which is how leaning
    // reads as leaning rather than as the world rotating.
    let right = vec3(dir.z, 0.0, -dir.x).normalize_or_zero();
    let up = dir.cross(right).normalize_or_zero();
    let roll = game.player.lean_roll();
    let up_tilted = (up * roll.cos() + right * roll.sin()).normalize_or_zero();
    let cam = eye - right * shake.x * 0.0025 - up * shake.y * 0.0025;

    // The backdrop: a night sky over the open arenas, and the water column over
    // the drowned one — with the colour the far geometry fades into chosen the
    // same way, so the fog and the backdrop can never disagree at the seam.
    let horizon = if game.level.sunken {
        murk_colour(game.level.sky)
    } else {
        horizon_colour(game.level.sky)
    };
    clear_background(horizon);
    if game.level.sunken {
        draw_sunken_sky(game.level.sky);
    } else {
        draw_sky(game.level.sky);
    }
    // macroquad 0.4.16 has no `Camera3D::world_to_screen` — its projection
    // helpers are `Camera2D`-only — so the frame's view-projection is built
    // here, with the same constructors and defaults this camera is made of.
    // The damage readouts are the one thing on screen that has to come back
    // out of the world.
    let view_proj = view_projection(
        cam,
        cam + dir,
        up_tilted,
        fovy,
        screen_width(),
        screen_height(),
    );
    set_camera(&Camera3D {
        position: cam,
        target: cam + dir,
        up: up_tilted,
        fovy,
        ..Default::default()
    });
    if let Some(gl_fog) = &game.fog {
        gl_fog.begin(horizon, game.level.half, desat, game.level.sunken);
    }
    for mesh in &game.level.meshes {
        // An arena that uses none of a material still gets a mesh for it, so
        // skip the empty batches rather than issue a draw call for nothing.
        if !mesh.indices.is_empty() {
            draw_mesh(mesh);
        }
    }
    // One scratch mesh for all of it: `draw_mesh` copies the vertices into
    // macroquad's batch as it is called, so nothing downstream still refers to
    // this buffer by the next iteration.
    for e in &game.enemies {
        if e.dead && e.death_t > 1.9 {
            continue;
        }
        e.write_mesh(&mut game.scratch);
        draw_mesh(&game.scratch);
    }
    for p in &game.pickups {
        p.write_mesh(&mut game.scratch);
        draw_mesh(&game.scratch);
    }
    game.grenades.write_mesh(&game.level, &mut game.scratch);
    draw_mesh(&game.scratch);
    game.effects.write_mesh(&mut game.scratch);
    draw_mesh(&game.scratch);
    // Everything in the world is behind us now; the viewmodel and the HUD are
    // drawn in the 2D pass and are not fogged — a weapon in your hands is not
    // twenty metres away.
    fog::Fog::end();
    set_default_camera();

    // The red edge is the "you are about to die" signal, not a per-hit flash.
    // A hit already draws a bearing arc and moves the health bar, and a
    // screen-wide wash on every one of them stacks into noise in a firefight —
    // so the wash carries madness alone, and the vignette lights up on its own
    // when health or sanity is nearly gone. The world grade above crosses the
    // same thresholds, so the two always agree about when "nearly" is.
    let madness = (1.0 - game.player.sanity_ratio()) * 0.7;
    hud::draw_vignette(madness, desat > 0.0);
    hud::draw_kill_flash(game.effects.kill_flash);
    // The one place the two yaw conventions meet. `hurt_dir` points from the
    // player at what hit them; the HUD's bearing is where that lands relative
    // to the view, 0 dead ahead and positive to the player's right. The game's
    // yaw grows to the left and the bearing grows to the right, so the
    // subtraction runs `cam_yaw - yaw_of(dir)`. Check it at yaw 0: the player
    // faces -Z and their right is +X, `yaw_of(+X)` is `atan2(-1, 0) = -PI/2`,
    // and `0 - (-PI/2) = +PI/2` — positive, i.e. right ✓.
    if let Some((dir, strength)) = game.player.hurt_dir() {
        let arcs = [(game.player.cam_yaw() - util::yaw_of(dir), strength)];
        hud::draw_hurt_arcs(&arcs);
    }
    draw_sanity_bar(fonts, game.player.sanity_ratio());
    if game.player.alive {
        game.viewmodel
            .draw(&game.player, &game.arsenal, &game.kit.paints);
    }
    draw_damage_numbers(game, fonts, &view_proj);
    draw_hud_state(game, fonts);
    draw_wheel_overlay(game, fonts);

    match game.overlay {
        Overlay::Inventory => draw_inventory_overlay(game, fonts),
        Overlay::Map => draw_map_overlay(game, fonts),
        Overlay::Mission => draw_mission_overlay(game, fonts),
        Overlay::Manual => draw_manual(game, fonts),
        Overlay::None => {}
    }
}

fn draw_wheel_overlay(game: &Game, fonts: &hud::Fonts) {
    let Some(w) = &game.wheel else { return };
    if w.held <= 0.28 {
        return;
    }
    let centre = vec2(screen_width() * 0.5, screen_height() * 0.5);
    let radius = screen_height() * 0.19;
    match w.kind {
        WheelKind::Grenade => {
            let items: Vec<(String, u32, bool)> = GRENADE_KINDS
                .iter()
                .enumerate()
                .map(|(i, k)| {
                    (
                        i18n::t(match k {
                            GrenadeKind::Frag => Key::GrenadeFrag,
                            GrenadeKind::Smoke => Key::GrenadeSmoke,
                        })
                        .to_string(),
                        game.ammo_grenades[i],
                        game.ammo_grenades[i] > 0,
                    )
                })
                .collect();
            hud::draw_wheel(
                fonts,
                i18n::t(Key::GrenadeWheel),
                &items,
                centre,
                radius,
                wheel_selection(items.len()).unwrap_or(w.selected),
            );
        }
        WheelKind::Med => {
            let items: Vec<(String, u32, bool)> = MED_KINDS
                .iter()
                .enumerate()
                .map(|(i, _)| {
                    (
                        i18n::t(med_key(i)).to_string(),
                        game.ammo_meds[i],
                        game.ammo_meds[i] > 0,
                    )
                })
                .collect();
            hud::draw_wheel(
                fonts,
                i18n::t(Key::MedWheel),
                &items,
                centre,
                radius,
                wheel_selection(items.len()).unwrap_or(w.selected),
            );
        }
    }
}

fn draw_hud_state(game: &Game, fonts: &hud::Fonts) {
    let weapon = game.arsenal.active_weapon();
    let slots: Vec<(&str, bool)> = SlotClass::ALL
        .iter()
        .enumerate()
        .map(|(i, _)| {
            let tag = game.arsenal.slot(i).map(|w| w.kind.tag()).unwrap_or("--");
            (
                tag,
                i == game.arsenal.active && game.arsenal.slot(i).is_some(),
            )
        })
        .collect();
    let objective = i18n::t(game.mission.objective.key());
    let sub = game.mission.objective.sub_key().map(i18n::t).unwrap_or("");

    let state = hud::HudState {
        health: game.player.health,
        max_health: game.player.max_health,
        armor: game.armor,
        score: game.score,
        kills: game.killed,
        elapsed: game.elapsed,
        enemies_left: game.remaining() as usize,
        wave: game.wave_display(),
        waves: game.mission.waves,
        objective,
        objective_sub: if game.extraction_open {
            i18n::t(Key::ObjReach)
        } else {
            sub
        },
        weapon_name: i18n::t(weapon.kind.name()),
        mag: weapon.mag,
        mag_size: weapon.stats().mag_size,
        reserve: (!weapon.stats().infinite_ammo && !weapon.stats().melee).then_some(weapon.reserve),
        reload_progress: if weapon.is_reloading() {
            Some(weapon.reload_progress())
        } else {
            None
        },
        slots,
        spread: weapon.spread(
            game.player.move_ratio(),
            game.player.ads,
            game.player.spread_scale(),
        ),
        hit_marker: game.effects.hit_marker,
        headshot_marker: game.effects.head_marker,
        hurt: game.player.hurt_flash,
        low_health: game.player.health < 30.0,
        prompt: game.toast.as_ref().map(|(s, _)| s.as_str()),
        banner: game
            .banner
            .as_ref()
            .map(|(s, t)| (s.as_str(), (*t / 2.8).clamp(0.0, 1.0))),
        locked: !game.objective_done,
    };
    hud::draw_hud(fonts, &state);
}

/// Drawn by us rather than by `hud.rs`, whose `HudState` predates sanity.
fn draw_sanity_bar(fonts: &hud::Fonts, ratio: f32) {
    if ratio > 0.995 {
        return;
    }
    let (w, h) = (screen_width(), screen_height());
    let ui = hud::ui_scale();
    let bar_w = 240.0 * ui;
    let bar_h = 6.0 * ui;
    let x = w * 0.5 - bar_w * 0.5;
    let y = h - 92.0 * ui;
    let colour = hud::sanity_colour(ratio);
    draw_rectangle(
        x - ui,
        y - ui,
        bar_w + 2.0 * ui,
        bar_h + 2.0 * ui,
        Color::new(0.05, 0.06, 0.08, 0.8),
    );
    draw_rectangle(x, y, bar_w * ratio, bar_h, colour);
    fonts.text_c(
        i18n::t(Key::Sanity),
        w * 0.5,
        y - 8.0 * ui,
        17.0 * ui,
        colour,
    );
}

fn draw_inventory_overlay(game: &Game, fonts: &hud::Fonts) {
    let name_of =
        |i: usize| -> Option<&'static str> { game.arsenal.slot(i).map(|w| i18n::t(w.kind.name())) };
    let grenades: Vec<(String, u32)> = GRENADE_KINDS
        .iter()
        .enumerate()
        .map(|(i, k)| {
            let label = i18n::t(match k {
                GrenadeKind::Frag => Key::GrenadeFrag,
                GrenadeKind::Smoke => Key::GrenadeSmoke,
            });
            (label.to_string(), game.ammo_grenades[i])
        })
        .collect();
    let meds: Vec<(String, u32)> = MED_KINDS
        .iter()
        .enumerate()
        .map(|(i, _)| (i18n::t(med_key(i)).to_string(), game.ammo_meds[i]))
        .collect();
    let s = hud::InventoryState {
        primary: name_of(0),
        secondary: name_of(1),
        pistol: name_of(2),
        melee: name_of(3),
        active_slot: game.arsenal.active,
        grenades: &grenades,
        meds: &meds,
        armour: game.armor,
        health: game.player.health,
    };
    hud::draw_inventory(fonts, &s);
}

fn draw_map_overlay(game: &Game, fonts: &hud::Fonts) {
    let boxes: Vec<(Vec2, Vec2)> = game
        .level
        .solids
        .iter()
        .map(|b| (vec2(b.min.x, b.min.z), vec2(b.max.x, b.max.z)))
        .collect();
    let half = game
        .level
        .solids
        .iter()
        .fold(1.0f32, |acc, b| acc.max(b.max.x.abs()).max(b.max.z.abs()))
        .max(1.0);
    let mut markers: Vec<hud::MapMarker> = Vec::new();
    if let Some(x) = game.level.extraction {
        markers.push(hud::MapMarker {
            pos: vec2(x.x, x.z),
            color: Color::new(0.35, 0.80, 1.0, 1.0),
            size: 7.0,
            label: Some('E'),
        });
    }
    // Only hostiles that are actually aware of the player show up, so the map
    // does not hand you a free wallhack.
    for e in &game.enemies {
        if e.dead || e.aware_t > 6.0 {
            continue;
        }
        markers.push(hud::MapMarker {
            pos: vec2(e.pos.x, e.pos.z),
            color: Color::new(0.85, 0.18, 0.16, 1.0),
            size: 6.0,
            label: None,
        });
    }
    hud::draw_map(
        fonts,
        &boxes,
        half,
        (vec2(game.player.pos.x, game.player.pos.z), game.player.yaw),
        &markers,
    );
}

fn draw_mission_overlay(game: &Game, fonts: &hud::Fonts) {
    let objective = i18n::t(game.mission.objective.key()).to_string();
    let progress = match game.mission.objective {
        Objective::Survive => format!(
            "{} / {}",
            format_time(game.survive_left),
            format_time(game.mission.survive_seconds)
        ),
        _ => format_time(game.elapsed),
    };
    let (damage, rate) = threat_numbers(game.mission, game.wave_display() - 1);
    let obj_rows = vec![
        hud::PanelRow {
            label: i18n::t(Key::Objective),
            value: objective,
        },
        hud::PanelRow {
            label: i18n::t(Key::Wave),
            value: format!("{} / {}", game.wave_display(), game.mission.waves),
        },
        hud::PanelRow {
            label: i18n::t(Key::Threat),
            value: format!(
                "{} ×{:.2} · {} ×{:.2}",
                i18n::t(Key::ThreatDamage),
                damage,
                i18n::t(Key::ThreatRate),
                rate
            ),
        },
        hud::PanelRow {
            label: i18n::t(Key::Hostiles),
            value: format!("{}", game.remaining()),
        },
        hud::PanelRow {
            label: i18n::t(Key::Time),
            value: progress,
        },
    ];
    let stat_rows = vec![
        hud::PanelRow {
            label: i18n::t(Key::Kills),
            value: format!("{}", game.killed),
        },
        hud::PanelRow {
            label: i18n::t(Key::Score),
            value: format!("{}", game.score),
        },
        hud::PanelRow {
            label: i18n::t(Key::Time),
            value: format_time(game.elapsed),
        },
        hud::PanelRow {
            label: i18n::t(Key::Accuracy),
            value: if game.shots_fired == 0 {
                "—".to_string()
            } else {
                format!(
                    "{:.0}%",
                    100.0 * game.shots_hit as f32 / game.shots_fired as f32
                )
            },
        },
    ];
    let sections: Vec<(&str, Vec<hud::PanelRow>)> = vec![
        (i18n::t(Key::Objectives), obj_rows),
        (i18n::t(Key::Statistics), stat_rows),
    ];
    let footer = format!(
        "{}   ·   {}   ·   {}",
        i18n::t(Key::HintTab),
        i18n::t(Key::HintMap),
        i18n::t(Key::HintClose)
    );
    hud::draw_panel(
        fonts,
        &format!(
            "{} — {}",
            i18n::t(Key::MissionPanel),
            level_name(&game.level)
        ),
        &sections,
        &footer,
    );
}

/// The manual's page, as the field and the menus both draw it.
///
/// The rows come out of `Game`'s cache — built when the page was opened or
/// turned to, never here — so this only draws. The pager counts pages from one,
/// which is why the stored page is bumped on the way out.
fn draw_manual(game: &Game, fonts: &hud::Fonts) {
    let page = game.manual_page + 1;
    match &game.manual_cache {
        Some(ManualContent::Controls { sections, tips }) => {
            hud::draw_manual_controls(fonts, page, MANUAL_PAGES, sections, tips);
        }
        Some(ManualContent::Cards { title, rows }) => {
            hud::draw_manual_cards(fonts, page, MANUAL_PAGES, i18n::t(*title), rows);
        }
        None => {}
    }
}
