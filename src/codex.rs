//! The field manual's content: the controls, the bestiary and the armoury.
//!
//! Every number the manual prints is read from the tables the simulation uses
//! (`enemy::stats`, `WeaponKind::stats`), so a balance change cannot leave the
//! manual lying the way a hand-written stat table would. The portraits are the
//! archetypes' own models, built through `Enemy::write_mesh` in the pose they
//! stand in — the same silhouettes the player has to read at 20 m.
//!
//! Nothing here runs while the manual is closed: a page is built when it is
//! opened, not on the drawing path, so nothing in this module costs the HUD a
//! frame. The controls page is the one page that is kept — its rows are
//! `&'static` slices (see `HELP`) — and it is kept once per language.
//!
//! One class of number cannot be read: `grenade.rs` keeps a frag's fuse, radius
//! and damage private, and `main.rs` keeps the medical and armour figures
//! private, so a module with only the public API in front of it has nothing to
//! query. Those live in the i18n text of their card, beside the source line
//! named in the comment there, which is the one place a reader has to compare
//! by hand.

use crate::enemy::{self, Enemy, Kind, KindStats};
use crate::grenade::GrenadeKind;
use crate::hud::{CodexRow, HelpRow, HelpSection};
use crate::i18n::{self, Key, Lang};
use crate::mesh::empty_mesh;
use crate::pickup::PickupKind;
use crate::util::Rng;
use crate::weapon::{self, SlotClass, WeaponKind, WeaponStats};
use macroquad::prelude::*;
use std::cell::RefCell;
use std::f32::consts::PI;

/// The archetypes in the order the campaign introduces them, which is the order
/// the bestiary reads in: the two the teaching mission sends, the specialists
/// the later arenas add, then what R'LYEH wakes up.
///
/// A list rather than an inherent `Kind::ALL`, so the manual's order is the
/// manual's business and the enum stays a plain enumeration.
const ARCHETYPES: [Kind; 8] = [
    Kind::Grunt,
    Kind::Rusher,
    Kind::Heavy,
    Kind::Sniper,
    Kind::Drone,
    Kind::Tentacle,
    Kind::Colossus,
    Kind::Spawnling,
];

/// What colours a kit card's dot. The cards are named by i18n keys rather than
/// by a table the game owns — the grenade and medical tables that name these
/// things are private to the modules that hold them — but the *colour* can come
/// off the world's own table for that class of item, so the manual cannot pick
/// a shade the pickup does not have.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum KitSwatch {
    Frag,
    Smoke,
    Medical,
    Armor,
    Ammo,
}

impl KitSwatch {
    fn color(self) -> Color {
        match self {
            KitSwatch::Frag => GrenadeKind::Frag.color(),
            KitSwatch::Smoke => GrenadeKind::Smoke.color(),
            KitSwatch::Medical => PickupKind::Health.color(),
            KitSwatch::Armor => PickupKind::Armor.color(),
            KitSwatch::Ammo => PickupKind::Ammo.color(),
        }
    }
}

/// The kit cards in display order: what you throw, what you take, what you wear
/// and what you feed the guns. `kit` is built straight off this, so a card and
/// its key cannot drift apart.
const KIT: [(Key, KitSwatch); 7] = [
    (Key::CodexItemFrag, KitSwatch::Frag),
    (Key::CodexItemSmoke, KitSwatch::Smoke),
    (Key::CodexItemBandage, KitSwatch::Medical),
    (Key::CodexItemMedkit, KitSwatch::Medical),
    (Key::CodexItemPainkiller, KitSwatch::Medical),
    (Key::CodexItemArmor, KitSwatch::Armor),
    (Key::CodexItemAmmo, KitSwatch::Ammo),
];

/// Where the card's camera stands: half a turn from the model's own facing.
///
/// A unit's models are built facing `-Z` — `util::dir_from_angles` and
/// `util::yaw_of` both put yaw 0 along `-Z` — so a yaw of PI turns the
/// archetype's front toward a camera that sits on `+Z` looking back down the
/// axis. Every portrait uses the same yaw, so two cards can be compared at a
/// glance.
const PORTRAIT_YAW: f32 = PI;

/// The seed the portraits are posed with. A fresh spawn has no gait, no alert
/// glow and no death fade, so the mesh is a function of the archetype alone;
/// the seed is fixed anyway, so that stays true if `spawn` ever rolls something
/// the model reads.
const PORTRAIT_SEED: u64 = 0x5EED_1234;

/// How a kit card's one string is cut into the name line and the line under it.
const ITEM_SEPARATOR: &str = " — ";

thread_local! {
    /// The controls page, built once per language.
    ///
    /// `HelpSection::rows` is a `&'static [HelpRow]` — the page is drawn
    /// straight out of the slice — so the localised rows have to be leaked to
    /// outlive the call that built them. Keeping one table per language bounds
    /// that leak: a page rebuilt on every `H` would leak a table per key press.
    static HELP: RefCell<[Option<Vec<HelpSection>>; 2]> = const { RefCell::new([None, None]) };

    /// The weapon tags, one per kind, built once per language.
    ///
    /// A tag is a `&'static str` (see `CodexRow`) and it carries a rate read off
    /// the weapon table, so the string it arrives in has to be leaked. Caching
    /// per language bounds that the same way `HELP` does.
    static TAGS: RefCell<[Option<Vec<&'static str>>; 2]> = const { RefCell::new([None, None]) };
}

fn lang_index(l: Lang) -> usize {
    match l {
        Lang::En => 0,
        Lang::Zh => 1,
    }
}

/// The controls page, in display order: what to press, grouped by what it is
/// for, then the survival notes underneath.
pub fn controls() -> Vec<HelpSection> {
    let lang = i18n::lang();
    HELP.with(|cell| {
        let mut slots = cell.borrow_mut();
        let page = slots[lang_index(lang)].get_or_insert_with(build_controls);
        page.iter()
            .map(|s| HelpSection {
                title: s.title,
                rows: s.rows,
            })
            .collect()
    })
}

/// One line per survival note, in display order.
pub fn tips() -> Vec<&'static str> {
    [
        Key::TipSprint,
        Key::TipAim,
        Key::TipArmor,
        Key::TipCover,
        Key::TipSanity,
        Key::TipPickups,
        Key::TipFireModes,
    ]
    .iter()
    .map(|k| i18n::t(*k))
    .collect()
}

/// The bestiary: one row per `enemy::Kind`, in the order the campaign
/// introduces them.
pub fn enemies() -> Vec<CodexRow> {
    ARCHETYPES.iter().map(|&k| enemy_row(k)).collect()
}

/// The armoury: one row per `weapon::WeaponKind`, in slot order.
pub fn weapons() -> Vec<CodexRow> {
    weapon::ALL_KINDS.iter().map(|&k| weapon_row(k)).collect()
}

/// The rest of the kit: grenades, meds, armour, ammunition.
pub fn kit() -> Vec<CodexRow> {
    KIT.iter()
        .map(|&(key, swatch)| item(key, swatch.color()))
        .collect()
}

// --- controls page ---------------------------------------------------------

fn build_controls() -> Vec<HelpSection> {
    vec![
        section(
            Key::ManualSectionMoving,
            &[
                Key::CtlMove,
                Key::CtlSprint,
                Key::CtlJump,
                Key::CtlCrouch,
                Key::CtlProne,
                Key::CtlWalk,
                Key::CtlLean,
                Key::CtlFreeLook,
            ],
        ),
        section(
            Key::ManualSectionFighting,
            &[
                Key::CtlFire,
                Key::CtlAim,
                Key::CtlReload,
                Key::CtlFireMode,
                Key::CtlCheckMode,
                Key::CtlCheckMag,
                Key::CtlHoldBreath,
                Key::CtlInspect,
                Key::CtlWeapons,
                Key::CtlGrenade,
                Key::CtlHeal,
                Key::CtlInteract,
            ],
        ),
        section(
            Key::ManualSectionScreens,
            &[
                Key::CtlInventory,
                Key::CtlMap,
                Key::CtlMission,
                Key::CtlHelp,
                Key::CtlCodex,
                Key::CtlPause,
            ],
        ),
        // The notes are handed to the page as a separate list, so this heading
        // is a section with no rows rather than a fifth parameter threaded
        // through the HUD's drawing code.
        section(Key::ManualSectionSurvival, &[]),
    ]
}

/// A titled group. Its rows are leaked into the table `HELP` keeps alive.
fn section(title: Key, rows: &[Key]) -> HelpSection {
    let rows: Vec<HelpRow> = rows.iter().map(|k| row(*k)).collect();
    HelpSection {
        title: i18n::t(title),
        rows: Box::leak(rows.into_boxed_slice()),
    }
}

/// One row. The keys are language-independent and belong on the same line as
/// what they do, so the whole row is one translated string and the `keys`
/// column stays empty rather than splitting a string at runtime.
fn row(text: Key) -> HelpRow {
    HelpRow {
        keys: "",
        text: i18n::t(text),
    }
}

// --- bestiary --------------------------------------------------------------

fn enemy_row(kind: Kind) -> CodexRow {
    let s = enemy::stats(kind);
    let (name, tag, note) = enemy_keys(kind);
    CodexRow {
        name: i18n::t(name),
        tag: i18n::t(tag),
        note: i18n::t(note),
        stats: enemy_stats(kind, &s),
        swatch: s.body,
        portrait: Some(portrait(kind)),
    }
}

/// An archetype's name (shared with the kill feed), one-line positioning and
/// note on how to fight it.
fn enemy_keys(kind: Kind) -> (Key, Key, Key) {
    match kind {
        Kind::Grunt => (Key::EnemyGrunt, Key::CodexGruntTag, Key::CodexGruntNote),
        Kind::Rusher => (Key::EnemyRusher, Key::CodexRusherTag, Key::CodexRusherNote),
        Kind::Heavy => (Key::EnemyHeavy, Key::CodexHeavyTag, Key::CodexHeavyNote),
        Kind::Sniper => (
            Key::EnemySniper,
            Key::CodexMarksmanTag,
            Key::CodexMarksmanNote,
        ),
        Kind::Drone => (Key::EnemyDrone, Key::CodexDroneTag, Key::CodexDroneNote),
        Kind::Tentacle => (
            Key::EnemyTentacle,
            Key::CodexTentacleTag,
            Key::CodexTentacleNote,
        ),
        Kind::Colossus => (
            Key::EnemyColossus,
            Key::CodexColossusTag,
            Key::CodexColossusNote,
        ),
        Kind::Spawnling => (
            Key::EnemySpawnling,
            Key::CodexSpawnlingTag,
            Key::CodexSpawnlingNote,
        ),
    }
}

/// The number line of an archetype's card, off `enemy::stats`.
///
/// `reach` is what a melee archetype can actually hit with — the shooters are
/// the ones with a reach of zero — so the range field reports whichever of the
/// two the archetype fights at.
fn enemy_stats(kind: Kind, s: &KindStats) -> String {
    let mut fields = vec![
        field(Key::StatHealth, num(s.health)),
        field(Key::StatSpeed, format!("{} m/s", num(s.speed))),
        field(
            Key::StatDamage,
            // A burst is rounds of that damage, not one bigger hit.
            if s.burst > 1 {
                format!("{}×{}", num(s.damage), s.burst)
            } else {
                num(s.damage)
            },
        ),
        field(
            Key::StatRange,
            format!("{} m", num(if s.reach > 0.0 { s.reach } else { s.range })),
        ),
        field(Key::StatRate, format!("{} RPM", rate(s.interval))),
    ];
    // Only the plated archetypes show the line, so its presence is itself
    // the warning (reference §17.2③: the player needs to be able to read
    // "bring something that pierces" off the card).
    let armor = enemy::armor(kind);
    if armor > 0.0 {
        fields.push(field(Key::StatArmor, format!("{:.0}%", armor * 100.0)));
    }
    if s.sanity_drain > 0.0 {
        fields.push(field(Key::Sanity, format!("-{}/s", num(s.sanity_drain))));
    }
    fields.join(" · ")
}

/// The portrait: the archetype's own model, turned to face the card's camera.
fn portrait(kind: Kind) -> Mesh {
    portrait_at(kind, PORTRAIT_YAW)
}

fn portrait_at(kind: Kind, yaw: f32) -> Mesh {
    let mut enemy = Enemy::spawn(kind, Vec3::ZERO, 1, &mut Rng::new(PORTRAIT_SEED));
    enemy.yaw = yaw;
    let mut mesh = empty_mesh();
    enemy.write_mesh(&mut mesh);
    mesh
}

// --- armoury ---------------------------------------------------------------

fn weapon_row(kind: WeaponKind) -> CodexRow {
    let s = kind.stats();
    CodexRow {
        name: i18n::t(kind.name()),
        tag: weapon_tag(kind),
        note: i18n::t(weapon_keys(kind).1),
        stats: weapon_stats(&s, kind),
        swatch: class_swatch(s.class),
        portrait: None,
    }
}

/// A weapon's one-line tag: the character line, plus the rate the card quotes.
///
/// The rate is appended here rather than written into the string, so the tag
/// comes off the same table as the stats row under it. `WeaponKind` is a
/// fieldless enum and `weapon::ALL_KINDS` is declared in that same order, which
/// is what lets the cache be indexed by the kind; a test pins that.
fn weapon_tag(kind: WeaponKind) -> &'static str {
    let lang = i18n::lang();
    TAGS.with(|cell| {
        let mut slots = cell.borrow_mut();
        let tags = slots[lang_index(lang)].get_or_insert_with(|| {
            weapon::ALL_KINDS
                .iter()
                .map(|&k| {
                    let s = k.stats();
                    let text = format!(
                        "{} · {} RPM",
                        i18n::t(weapon_keys(k).0),
                        rate(s.fire_interval)
                    );
                    &*Box::leak(text.into_boxed_str())
                })
                .collect()
        });
        tags[kind as usize]
    })
}

/// A weapon's positioning and note. The name is `WeaponKind::name`, which the
/// HUD's own weapon readouts already use.
fn weapon_keys(kind: WeaponKind) -> (Key, Key) {
    match kind {
        WeaponKind::Rifle => (Key::CodexRifleTag, Key::CodexRifleNote),
        WeaponKind::Shotgun => (Key::CodexShotgunTag, Key::CodexShotgunNote),
        WeaponKind::Smg => (Key::CodexSmgTag, Key::CodexSmgNote),
        WeaponKind::Sniper => (Key::CodexSniperTag, Key::CodexSniperNote),
        WeaponKind::Lmg => (Key::CodexLmgTag, Key::CodexLmgNote),
        WeaponKind::Pistol => (Key::CodexPistolTag, Key::CodexPistolNote),
        WeaponKind::Knife => (Key::CodexKnifeTag, Key::CodexKnifeNote),
        WeaponKind::M4a1 => (Key::CodexM4a1Tag, Key::CodexM4a1Note),
        WeaponKind::Scar => (Key::CodexScarTag, Key::CodexScarNote),
        WeaponKind::Fal => (Key::CodexFalTag, Key::CodexFalNote),
        WeaponKind::Mp5 => (Key::CodexMp5Tag, Key::CodexMp5Note),
        WeaponKind::Mpx => (Key::CodexMpxTag, Key::CodexMpxNote),
        WeaponKind::M110 => (Key::CodexM110Tag, Key::CodexM110Note),
        WeaponKind::Mosin => (Key::CodexMosinTag, Key::CodexMosinNote),
    }
}

/// The number line of a weapon card, off `WeaponKind::stats`.
///
/// Spread and recoil are angles in the table and are printed as degrees: 0.0042
/// rad is a fact a player cannot picture, and 0.24° is one they can.
fn weapon_stats(s: &WeaponStats, kind: WeaponKind) -> String {
    let mut fields = vec![
        field(Key::StatClass, i18n::t(class_label(s.class)).to_string()),
        field(Key::StatDamage, num(s.damage)),
    ];
    if s.pellets > 1 {
        fields.push(field(Key::StatPellets, s.pellets.to_string()));
    }
    fields.push(field(
        Key::StatRate,
        format!("{} RPM", rate(s.fire_interval)),
    ));
    if s.infinite_ammo {
        // No magazine to count. A dash says "not applicable" in both languages;
        // the cards' notes carry the fact that it never needs ammunition.
        fields.push(field(Key::StatMag, "—".to_string()));
    } else {
        fields.push(field(Key::StatMag, s.mag_size.to_string()));
        fields.push(field(Key::StatReload, format!("{} s", num(s.reload_time))));
    }
    if !s.melee {
        // A swing is not a shot: the melee path never adds recoil (`main.rs`
        // skips `add_recoil` for a melee weapon) and has no base spread, so
        // printing either would be quoting a number the game ignores.
        fields.push(field(
            Key::StatSpread,
            format!("{}°", num(s.base_spread.to_degrees())),
        ));
        fields.push(field(
            Key::StatRecoil,
            format!("{}°", num(s.recoil_pitch.to_degrees())),
        ));
    }
    if s.fire_modes.len() > 1 {
        let modes: Vec<&str> = s.fire_modes.iter().map(|m| i18n::t(m.label())).collect();
        fields.push(field(Key::StatModes, modes.join("/")));
    }
    // The share of a hit this weapon carries through plate — the one number
    // that answers "how does this do against a Juggernaut" (reference
    // §17.2③). Every weapon names it, so it can be compared at a glance.
    fields.push(field(
        Key::StatArmorPen,
        format!("{:.0}%", kind.armor_pen() * 100.0),
    ));
    fields.join(" · ")
}

/// Weapons carry no colour of their own — `WeaponDef` has no such field — so
/// the card's dot is keyed to the slot the kind fills, which is a fact from the
/// table. Presentation either way: nothing in the world reads it.
fn class_swatch(class: SlotClass) -> Color {
    match class {
        SlotClass::Primary => Color::new(0.44, 0.52, 0.64, 1.0),
        SlotClass::Secondary => Color::new(0.52, 0.48, 0.62, 1.0),
        SlotClass::Pistol => Color::new(0.66, 0.62, 0.42, 1.0),
        SlotClass::Melee => Color::new(0.60, 0.60, 0.64, 1.0),
    }
}

/// The slot label, from the same four strings the inventory screen uses.
fn class_label(class: SlotClass) -> Key {
    match class {
        SlotClass::Primary => Key::SlotPrimary,
        SlotClass::Secondary => Key::SlotSecondary,
        SlotClass::Pistol => Key::SlotPistol,
        SlotClass::Melee => Key::SlotMelee,
    }
}

// --- kit -------------------------------------------------------------------

/// A kit card. One translated string carries the name and the description, and
/// the card cuts it at the dash they are written with: half of a `&'static str`
/// is still `&'static str`, so the name line and the line under it cost no
/// allocation and no second key.
fn item(text: Key, swatch: Color) -> CodexRow {
    let (name, description) = split_item(i18n::t(text));
    CodexRow {
        name,
        tag: "",
        note: "",
        stats: description.to_string(),
        swatch,
        portrait: None,
    }
}

/// Splits `"NAME — description"`. A string without the separator would leave
/// the card's information line empty, so the tests walk every kit key.
fn split_item(s: &'static str) -> (&'static str, &'static str) {
    s.split_once(ITEM_SEPARATOR).unwrap_or((s, ""))
}

// --- formatting ------------------------------------------------------------

/// `LABEL value`, the unit a stat row is built from.
fn field(label: Key, value: String) -> String {
    format!("{} {}", i18n::t(label), value)
}

/// A table number as a stat row prints it: at most two decimals, with the
/// trailing zeros and the point dropped.
///
/// The tables are `f32`: printed bare, a converted angle is `0.24056113` and a
/// whole number is `62`, so there is no one format that fits both. Two decimals
/// with the zeros trimmed reads the same in either case.
fn num(v: f32) -> String {
    let s = format!("{:.2}", v);
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

/// The rounds per minute the rest of the world quotes a firing interval as.
fn rate(interval: f32) -> String {
    let rpm = 60.0 / interval.max(1e-3);
    format!("{:.0}", rpm)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_finite(mesh: &Mesh, what: &str) {
        for v in &mesh.vertices {
            assert!(
                v.position.is_finite(),
                "{what} has a non-finite vertex at {:?}",
                v.position
            );
        }
    }

    /// One card per archetype, and no invented ones: the bestiary is exactly
    /// the enum, in the manual's reading order.
    #[test]
    fn the_bestiary_has_one_card_for_every_archetype() {
        let rows = enemies();
        assert_eq!(rows.len(), ARCHETYPES.len(), "the bestiary changed size");
        assert_eq!(rows.len(), 8);
        for (i, &kind) in ARCHETYPES.iter().enumerate() {
            // Distinctness is what makes the list the whole enum rather than
            // the same horror printed twice.
            for &other in &ARCHETYPES[i + 1..] {
                assert_ne!(kind, other, "{kind:?} is listed twice");
            }
            let s = enemy::stats(kind);
            let row = &rows[i];
            assert_eq!(row.swatch, s.body, "{kind:?} has the wrong swatch");
            assert!(!row.name.is_empty(), "{kind:?} has no name");
            assert!(!row.tag.is_empty(), "{kind:?} has no tag");
            assert!(
                !row.note.is_empty(),
                "{kind:?} has no note on how to fight it"
            );
            // The table's health has to be on the card, or a balance change
            // turns the manual into a period piece.
            let health = num(s.health);
            assert!(
                row.stats.contains(&health),
                "{kind:?} prints {:?}, which does not carry its {health} HP",
                row.stats
            );
        }
    }

    /// Every card has a number line, and every number line is built from the
    /// table: the damage and the rate a weapon actually has.
    #[test]
    fn the_armoury_has_one_card_for_every_weapon() {
        let rows = weapons();
        assert_eq!(rows.len(), weapon::ALL_KINDS.len());
        for (i, &kind) in weapon::ALL_KINDS.iter().enumerate() {
            let s = kind.stats();
            let row = &rows[i];
            assert!(row.portrait.is_none(), "{kind:?} has no model to show");
            assert_eq!(row.swatch, class_swatch(s.class));
            let damage = num(s.damage);
            let rpm = rate(s.fire_interval);
            assert!(
                row.stats.contains(&damage),
                "{kind:?} prints {:?}, which does not carry its {damage} damage",
                row.stats
            );
            assert!(
                row.stats.contains(&rpm),
                "{kind:?} prints {:?}, which does not carry its {rpm} RPM",
                row.stats
            );
            // The tag quotes the same rate the row under it does.
            assert!(
                row.tag.contains(&rpm),
                "{kind:?}'s tag is {:?}, which is missing {rpm} RPM",
                row.tag
            );
            assert!(
                !row.note.is_empty(),
                "{kind:?} has no note on how to use it"
            );
        }
    }

    /// The kit cards are cut out of one string each, so a missing separator
    /// would leave the card's information line blank.
    #[test]
    fn every_kit_card_carries_its_name_and_its_information() {
        let rows = kit();
        assert_eq!(rows.len(), KIT.len());
        for (i, (key, swatch)) in KIT.iter().enumerate() {
            assert!(!rows[i].name.is_empty(), "{key:?} has no name");
            assert!(!rows[i].stats.is_empty(), "{key:?} has no information line");
            assert_eq!(rows[i].swatch, swatch.color(), "{key:?} has the wrong dot");
            let text = i18n::t(*key);
            assert!(
                text.contains(ITEM_SEPARATOR),
                "{text:?} is not written as a name and a description"
            );
            let (name, description) = split_item(text);
            assert!(!name.is_empty() && !description.is_empty(), "{text:?}");
        }
        // No two cards may share a name: the manual would print the same piece
        // of kit twice and leave another one out.
        for (i, row) in rows.iter().enumerate() {
            for other in &rows[i + 1..] {
                assert_ne!(row.name, other.name, "two kit cards share a name");
            }
        }
    }

    /// The portraits are the real models: not empty, not non-finite, and
    /// standing on the panel's floor.
    #[test]
    fn every_bestiary_card_carries_a_portrait_of_its_own_model() {
        let rows = enemies();
        for (i, &kind) in ARCHETYPES.iter().enumerate() {
            let mesh = rows[i]
                .portrait
                .as_ref()
                .unwrap_or_else(|| panic!("{kind:?} has no portrait"));
            assert!(!mesh.vertices.is_empty(), "{kind:?}'s portrait is empty");
            assert!(
                !mesh.indices.is_empty() && mesh.indices.len() % 3 == 0,
                "{kind:?}'s portrait is not a triangle list"
            );
            assert_finite(mesh, &format!("{kind:?}'s portrait"));
            let lowest = mesh
                .vertices
                .iter()
                .map(|v| v.position.y)
                .fold(f32::MAX, f32::min);
            assert!(
                lowest.abs() < 0.05,
                "{kind:?}'s portrait has its feet at {lowest}, not on y = 0"
            );
        }
    }

    /// The tag cache is indexed by the kind, which only works while
    /// `weapon::ALL_KINDS` is declared in the enum's own order. That is exactly
    /// the kind of assumption a later reordering breaks silently, so it is
    /// checked rather than trusted.
    #[test]
    fn the_weapon_table_is_in_enum_order() {
        for (i, &kind) in weapon::ALL_KINDS.iter().enumerate() {
            assert_eq!(kind as usize, i, "{kind:?} is out of order");
        }
    }

    /// A card faces the camera, not away from it.
    ///
    /// The models are built facing `-Z`, so the portrait has to be the same
    /// mesh with a half turn applied — checked against the model built at yaw
    /// 0, which is the only way to tell "turned to face you" from "left as it
    /// came out of the box" without reading colour at a glance.
    #[test]
    fn a_portrait_faces_the_camera() {
        for &kind in &ARCHETYPES {
            let turned = portrait_at(kind, PORTRAIT_YAW);
            let raw = portrait_at(kind, 0.0);
            assert_eq!(turned.vertices.len(), raw.vertices.len());
            for (a, b) in turned.vertices.iter().zip(raw.vertices.iter()) {
                // A half turn about Y sends (x, z) to (-x, -z).
                let want = vec3(-b.position.x, b.position.y, -b.position.z);
                assert!(
                    (a.position - want).length() < 1e-5,
                    "{kind:?} is not turned to face the camera: {:?} is not {:?}",
                    a.position,
                    want
                );
            }
        }
    }

    /// The same card twice is the same picture: an unseeded pose would shuffle
    /// between openings, which is a tell that the manual is a live simulation
    /// rather than a drawing of one.
    #[test]
    fn opening_a_card_twice_shows_the_same_pose() {
        let first = enemies();
        let second = enemies();
        for (a, b) in first.iter().zip(second.iter()) {
            let (ma, mb) = (
                a.portrait.as_ref().expect("a portrait"),
                b.portrait.as_ref().expect("a portrait"),
            );
            assert_eq!(ma.vertices.len(), mb.vertices.len());
            for (va, vb) in ma.vertices.iter().zip(mb.vertices.iter()) {
                assert_eq!(va.position, vb.position, "{}", a.name);
            }
        }
    }

    /// The controls page is the whole binding list, and the notes hang off the
    /// last heading — the one section with no rows of its own.
    #[test]
    fn the_controls_page_lists_every_binding_once() {
        let sections = controls();
        assert_eq!(sections.len(), 4, "the page lost a group");
        let rows: Vec<&HelpRow> = sections.iter().flat_map(|s| s.rows.iter()).collect();
        assert_eq!(rows.len(), 26, "a binding is missing from the page");
        assert!(sections[3].rows.is_empty(), "the notes heading has rows");
        for s in &sections {
            assert!(!s.title.is_empty());
            for row in s.rows {
                assert!(!row.keys.is_empty() || !row.text.is_empty());
                assert!(
                    row.text.contains('\u{3000}'),
                    "{:?} does not carry its keys",
                    row.text
                );
            }
        }
        // A key listed twice would print the same binding twice and leave
        // another one out, which the count above cannot see.
        for (i, row) in rows.iter().enumerate() {
            for other in &rows[i + 1..] {
                assert_ne!(row.text, other.text, "a binding is listed twice");
            }
        }
        // Reading the page twice gives the page again, not a fresh leak.
        assert_eq!(controls().len(), sections.len());
    }

    /// Every survival note is there and says something.
    #[test]
    fn the_survival_notes_are_all_present() {
        let tips = tips();
        assert_eq!(tips.len(), 7);
        for tip in &tips {
            assert!(!tip.is_empty());
        }
        for (i, tip) in tips.iter().enumerate() {
            for other in &tips[i + 1..] {
                assert_ne!(tip, other, "a note is repeated");
            }
        }
    }

    /// The page is cached per language, so switching has to rebuild it — a
    /// stale cache would hand a Chinese player the English page.
    #[test]
    fn the_page_follows_the_language() {
        let before = i18n::lang();
        i18n::set_lang(Lang::En);
        let en = controls();
        let en_move = en[0].rows[0].text;
        i18n::set_lang(Lang::Zh);
        let zh = controls();
        assert_ne!(
            en_move, zh[0].rows[0].text,
            "the cache returned the other language"
        );
        assert_eq!(en.len(), zh.len());
        assert_eq!(
            en.iter().map(|s| s.rows.len()).sum::<usize>(),
            zh.iter().map(|s| s.rows.len()).sum::<usize>()
        );
        // Put the suite back where it was found: strings read elsewhere are
        // global state, and a test that leaves it switched is a test that
        // breaks its neighbours.
        i18n::set_lang(before);
    }

    /// The stat formatter: no trailing zeros, no `f32` noise.
    #[test]
    fn the_stat_formatter_prints_what_a_reader_can_use() {
        assert_eq!(num(0.0), "0");
        assert_eq!(num(62.0), "62");
        assert_eq!(num(2.9), "2.9");
        assert_eq!(num(1.85), "1.85");
        assert_eq!(num(0.2406), "0.24");
        assert_eq!(rate(0.088), "682");
        assert_eq!(rate(0.075), "800");
        assert_eq!(rate(1.15), "52");
        // A zero interval must not divide by zero.
        assert_eq!(rate(0.0), "60000");
    }

    /// The whole manual, printed to be *read* rather than asserted on:
    /// `cargo test -- --ignored --nocapture the_manual_as_printed`. The same
    /// shape as `maps.rs`'s design report — the copy is the one part of this
    /// module a test can only confirm is present, so it is worth having a way
    /// to look at all of it at once, in both languages.
    #[test]
    #[ignore]
    fn the_manual_as_printed() {
        let before = i18n::lang();
        for lang in [Lang::En, Lang::Zh] {
            i18n::set_lang(lang);
            println!("\n================ {} ================", lang.label());
            for section in controls() {
                println!("[{}]", section.title);
                for row in section.rows {
                    println!("  {}", row.text);
                }
            }
            println!("[notes]");
            for tip in tips() {
                println!("  {tip}");
            }
            for (title, rows) in [
                ("bestiary", enemies()),
                ("armoury", weapons()),
                ("kit", kit()),
            ] {
                println!("[{title}]");
                for row in rows {
                    println!("  {}  —  {}", row.name, row.tag);
                    println!("      {}", row.stats);
                    if !row.note.is_empty() {
                        println!("      {}", row.note);
                    }
                    let portrait = match &row.portrait {
                        Some(m) => format!("{} verts", m.vertices.len()),
                        None => "none".to_string(),
                    };
                    println!("      (portrait {portrait})");
                }
            }
        }
        i18n::set_lang(before);
    }
}
