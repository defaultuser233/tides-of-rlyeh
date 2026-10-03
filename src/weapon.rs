use crate::i18n::Key;
use crate::util::approach;
use macroquad::prelude::*;

pub const MAX_RANGE: f32 = 160.0;

/// Slot the weapon occupies, matching the 1-4 number keys.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SlotClass {
    Primary,
    Secondary,
    Pistol,
    Melee,
}

impl SlotClass {
    pub const ALL: [SlotClass; 4] = [
        SlotClass::Primary,
        SlotClass::Secondary,
        SlotClass::Pistol,
        SlotClass::Melee,
    ];
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FireMode {
    Auto,
    Burst,
    Single,
}

impl FireMode {
    pub fn label(self) -> Key {
        match self {
            FireMode::Auto => Key::FireAuto,
            FireMode::Burst => Key::FireBurst,
            FireMode::Single => Key::FireSingle,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum WeaponKind {
    Rifle,
    Shotgun,
    Smg,
    Sniper,
    Lmg,
    Pistol,
    Knife,
}

pub const ALL_KINDS: [WeaponKind; 7] = [
    WeaponKind::Rifle,
    WeaponKind::Shotgun,
    WeaponKind::Smg,
    WeaponKind::Sniper,
    WeaponKind::Lmg,
    WeaponKind::Pistol,
    WeaponKind::Knife,
];

/// The kind a settings key names — the inverse of [`WeaponKind::key`] — or
/// `None` when nothing matches.
pub fn kind_named(name: &str) -> Option<WeaponKind> {
    ALL_KINDS.iter().copied().find(|k| k.key() == name.trim())
}

const SEMI: &[FireMode] = &[FireMode::Single];
const AUTO_SEMI: &[FireMode] = &[FireMode::Auto, FireMode::Single];
const RIFLE_MODES: &[FireMode] = &[FireMode::Auto, FireMode::Burst, FireMode::Single];
const SMG_MODES: &[FireMode] = &[FireMode::Auto, FireMode::Burst, FireMode::Single];

/// The numbers the rest of the game reads, one field per concept.
///
/// This is the field-named view of a `ResolvedStats`: produced by resolving a
/// def against a build, and no longer where a weapon's numbers are authored —
/// `WeaponDef::base` is. Kept field for field so that every caller that reads
/// `WeaponStats` needed no change when the model behind it arrived.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct WeaponStats {
    pub class: SlotClass,
    pub damage: f32,
    /// Projectiles per trigger pull.
    pub pellets: u32,
    pub fire_interval: f32,
    pub mag_size: u32,
    pub reload_time: f32,
    pub base_spread: f32,
    pub bloom_per_shot: f32,
    pub max_bloom: f32,
    pub bloom_decay: f32,
    pub recoil_pitch: f32,
    pub recoil_yaw: f32,
    pub ads_fov: f32,
    pub move_scale: f32,
    pub headshot_mult: f32,
    pub falloff_start: f32,
    pub falloff_end: f32,
    pub falloff_min: f32,
    /// Cycled with B. The first entry is the starting mode.
    pub fire_modes: &'static [FireMode],
    pub burst_count: u32,
    /// Knives and other tools never consume ammunition.
    pub infinite_ammo: bool,
    /// Melee weapons resolve as a very short, very wide hit instead of a shot.
    pub melee: bool,
}

/// Where a number comes from.
///
/// `Static` is what every shipping weapon uses. `Range` is the source of "same
/// part, different feel": whoever builds the instance draws a roll and hands it
/// in, so nothing here needs to know about `util::Rng`, and `resolve` stays a
/// function that a test, a tutorial hint and a network check can all replay.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Value {
    Static(f32),
    /// A number that differs between two copies of one part, sampled by the
    /// build entry's roll. The shipping catalogue uses one: surplus ammo.
    Range {
        min: f32,
        max: f32,
    },
}

impl Value {
    /// The number this stands for at `roll`, clamped into `[0, 1]`: a caller
    /// that draws outside the unit interval gets the nearest end of the range,
    /// not a value the range never allowed.
    pub fn sample(self, roll: f32) -> f32 {
        match self {
            Value::Static(v) => v,
            Value::Range { min, max } => min + (max - min) * roll.clamp(0.0, 1.0),
        }
    }
}

/// One tunable number of a weapon: the numeric half of `WeaponStats`.
///
/// Identity is deliberately not a stat. A part changes how a weapon performs;
/// it cannot change what the weapon *is* — its class, its fire modes, whether
/// it burns ammunition.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum StatId {
    Damage,
    Pellets,
    FireInterval,
    MagSize,
    ReloadTime,
    BaseSpread,
    BloomPerShot,
    MaxBloom,
    BloomDecay,
    RecoilPitch,
    RecoilYaw,
    AdsFov,
    MoveScale,
    HeadshotMult,
    FalloffStart,
    FalloffEnd,
    FalloffMin,
    BurstCount,
}

/// How many stats there are: the length of `WeaponDef::base` and of the value
/// block inside a `ResolvedStats`.
pub const STAT_COUNT: usize = 18;

impl StatId {
    /// The stats in declaration order, which is the order `WeaponDef::base` is
    /// written in and the order `resolve` walks.
    pub const ALL: [StatId; 18] = [
        StatId::Damage,
        StatId::Pellets,
        StatId::FireInterval,
        StatId::MagSize,
        StatId::ReloadTime,
        StatId::BaseSpread,
        StatId::BloomPerShot,
        StatId::MaxBloom,
        StatId::BloomDecay,
        StatId::RecoilPitch,
        StatId::RecoilYaw,
        StatId::AdsFov,
        StatId::MoveScale,
        StatId::HeadshotMult,
        StatId::FalloffStart,
        StatId::FalloffEnd,
        StatId::FalloffMin,
        StatId::BurstCount,
    ];

    /// Place in `WeaponDef::base`. It is the declaration order, which is why
    /// `ALL` has to be written in that order too — the check below is what
    /// stops a stat from resolving out of another stat's row.
    pub const fn index(self) -> usize {
        self as usize
    }

    /// Whether the stat counts whole things. Those are rounded between the
    /// `MulPre` and `MulPost` stages: nothing holds 14.5 rounds.
    pub const fn whole(self) -> bool {
        matches!(self, StatId::Pellets | StatId::MagSize | StatId::BurstCount)
    }

    /// The envelope a stat is snapped into between the `MulPre` and `MulPost`
    /// stages. Every number of today's seven weapons sits inside its own
    /// envelope, which is what lets an empty build resolve to the untouched
    /// table.
    pub const fn bounds(self) -> (f32, f32) {
        match self {
            StatId::Damage => (0.0, 1000.0),
            StatId::Pellets => (0.0, 64.0),
            StatId::FireInterval => (0.02, 5.0),
            StatId::MagSize => (0.0, 500.0),
            StatId::ReloadTime => (0.0, 10.0),
            StatId::BaseSpread => (0.0, 0.5),
            StatId::BloomPerShot => (0.0, 0.5),
            StatId::MaxBloom => (0.0, 1.0),
            StatId::BloomDecay => (0.0, 5.0),
            StatId::RecoilPitch => (0.0, 1.0),
            StatId::RecoilYaw => (0.0, 1.0),
            StatId::AdsFov => (5.0, 120.0),
            StatId::MoveScale => (0.1, 2.0),
            StatId::HeadshotMult => (1.0, 10.0),
            StatId::FalloffStart => (0.0, 1000.0),
            StatId::FalloffEnd => (0.0, 1000.0),
            StatId::FalloffMin => (0.0, 1.0),
            StatId::BurstCount => (0.0, 16.0),
        }
    }
}

const _: () = {
    assert!(StatId::ALL.len() == STAT_COUNT);
    let mut i: usize = 0;
    while i < STAT_COUNT {
        assert!(StatId::ALL[i].index() == i);
        i += 1;
    }
};

/// One arithmetic step, already carrying its number.
///
/// Steps are not applied in the order their parts are listed. `resolve` walks
/// one stat at a time and runs the stages in this fixed order:
///
/// 1. every `Add` is summed into the base value, so two flat bonuses cannot
///    depend on which part was bolted on first;
/// 2. every `MulPre` multiplies that sum;
/// 3. the result is snapped into the stat's envelope (`StatId::bounds`), and a
///    whole-number stat is rounded there;
/// 4. every `MulPost` multiplies what survived step 3. It has the last word, so
///    it is the one stage allowed to leave the envelope the design declared —
///    that is what a late or cursed bonus uses — and the only thing it may not
///    do is leave a stat non-finite or negative.
///
/// `MulPre` and `MulPost` are the same arithmetic, and part ways only where
/// step 3 bites: at the envelope, or on a whole-number stat. They are two
/// stages rather than one so that a designer can say which side of the design
/// envelope a multiplier was meant for.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Modifier {
    Add(f32),
    MulPre(f32),
    MulPost(f32),
}

/// Which stage a `StatMod` feeds: the declarative half of `Modifier`.
///
/// The two are separate because a `Modifier` carries a number while a `StatMod`
/// carries a `Value`, whose number is only rolled when an instance is built.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ModKind {
    Add,
    MulPre,
    MulPost,
}

/// One line of a part's spec sheet: the stat it touches, the stage it feeds,
/// and how much — as a `Value`, so the same part can be authored to roll.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct StatMod {
    pub stat: StatId,
    pub kind: ModKind,
    pub amount: Value,
}

impl StatMod {
    /// Materialise this line for one instance. The `roll` belongs to the build
    /// entry, which is the only thing that differs between two copies of the
    /// same part.
    pub fn modifier(&self, roll: f32) -> Modifier {
        let n = self.amount.sample(roll);
        match self.kind {
            ModKind::Add => Modifier::Add(n),
            ModKind::MulPre => Modifier::MulPre(n),
            ModKind::MulPost => Modifier::MulPost(n),
        }
    }
}

/// A slot: one the weapon offers, or one a part opens up.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct SlotId(pub &'static str);

/// The mounting standard a slot takes and a part declares. Two slots that take
/// the same standard take the same parts, which is how one optic can fit a
/// rifle and a pistol without either def knowing about the other.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct InterfaceId(pub &'static str);

/// A part, by the name the catalogue files it under.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct PartId(pub &'static str);

/// One slot on a weapon.
#[derive(Clone, Copy, Debug)]
pub struct SlotDef {
    pub id: SlotId,
    /// The standard a part must declare to seat here.
    pub accepts: InterfaceId,
    /// How many parts the slot holds at once. An optic rail takes one; a
    /// handguard rail takes as many as it has room for.
    pub max_parts: usize,
}

/// A part, as its designer writes it: what it is built for, what standard it
/// declares, what it opens up, and what it does to the numbers.
#[derive(Clone, Copy, Debug)]
pub struct PartDef {
    pub id: PartId,
    /// The slot it is built for.
    pub slot: SlotId,
    pub interface: InterfaceId,
    /// Slots it opens once installed — which is how a scope ends up on a rifle
    /// whose def never mentions it, on the invitation of the handguard.
    pub subslots: &'static [SlotDef],
    pub mods: &'static [StatMod],
}

/// Everything a weapon kind is authored as: what it is, what it starts at, and
/// what can be hung on it.
///
/// A def is shared by every instance of its kind and is never written to.
/// `resolve` reads it; the outcome is a `ResolvedStats`, which belongs to the
/// instance. Writing a build's outcome back into a def is how one weapon's
/// scope becomes every weapon's scope.
pub struct WeaponDef {
    /// Identity: what the weapon is. No part may change these.
    pub class: SlotClass,
    pub fire_modes: &'static [FireMode],
    pub infinite_ammo: bool,
    pub melee: bool,
    /// The starting value of every stat, in `StatId::ALL` order. A `Range` here
    /// is sampled at its midpoint: a def is shared, so it has no per-instance
    /// roll of its own, and a number meant to differ between two copies belongs
    /// on a part, whose build entry carries one.
    pub base: [Value; STAT_COUNT],
    /// The slots the weapon offers before anything is installed.
    pub slots: &'static [SlotDef],
}

/// One weapon as it is actually built: which part went into which slot, and the
/// roll that samples any range that part declared.
///
/// The roll is the caller's — drawn from `util::Rng` at the bench, pinned to a
/// constant by a test — because `resolve` may not draw one itself.
#[derive(Clone, PartialEq, Debug)]
pub struct WeaponBuild {
    pub entries: Vec<BuildEntry>,
}

impl WeaponBuild {
    /// A weapon as it comes out of the crate: no parts, so resolving it yields
    /// the def's own numbers.
    pub const fn empty() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// The build as the settings file spells it: `slot=part@roll`, one per
    /// comma, empty when nothing is installed. Rolls are written to two
    /// decimals — the file reads back into the same build, not a new one.
    pub fn to_text(&self) -> String {
        use std::fmt::Write as _;
        let mut out = String::new();
        for (i, entry) in self.entries.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            let _ = write!(out, "{}={}@{:.2}", entry.slot.0, entry.part.0, entry.roll);
        }
        out
    }

    /// Parse [`WeaponBuild::to_text`]'s spelling. Junk is skipped rather than
    /// reported: this reads a hand-editable file. An entry naming a slot or a
    /// part this build does not know is skipped too — ids are `&'static str`
    /// here, so a name that is not in the tables could not be carried anyway —
    /// and the rest of the line still reads.
    pub fn from_text(text: &str) -> Self {
        let mut entries = Vec::new();
        for item in text.split(',') {
            let Some((slot, rest)) = item.trim().split_once('=') else {
                continue;
            };
            let (part, roll) = rest.split_once('@').unwrap_or((rest, "0.5"));
            let (Some(slot), Some(part)) = (slot_named(slot.trim()), part_named(part.trim()))
            else {
                continue;
            };
            entries.push(BuildEntry {
                slot,
                part,
                roll: roll.trim().parse::<f32>().unwrap_or(0.5).clamp(0.0, 1.0),
            });
        }
        Self { entries }
    }

    /// The build with everything a def cannot hold removed: entries are kept in
    /// order, each only if it validates against what is already kept. A
    /// hand-edited file degrades to the parts that fit, not to nonsense.
    pub fn screened(self, def: &WeaponDef) -> Self {
        let mut kept = Self::empty();
        for entry in self.entries {
            kept.entries.push(entry);
            if validate(def, &kept).is_err() {
                kept.entries.pop();
            }
        }
        kept
    }
}

/// One choice: a part in a slot, at a roll.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct BuildEntry {
    pub slot: SlotId,
    pub part: PartId,
    /// In `[0, 1]`; samples every ranged number the part declares.
    pub roll: f32,
}

/// A weapon's numbers once its build has been resolved.
///
/// Cache, not truth: this is what an instance carries instead of recomputing
/// `resolve` per shot, and it exists per weapon rather than per kind, so no
/// instance can see another's build.
#[derive(Clone, Copy, Debug)]
pub struct ResolvedStats {
    class: SlotClass,
    fire_modes: &'static [FireMode],
    infinite_ammo: bool,
    melee: bool,
    values: [f32; STAT_COUNT],
}

impl ResolvedStats {
    /// One stat by id: what a gunsmith screen draws its bars from, and what a
    /// delta readout compares two builds with.
    pub fn stat(&self, id: StatId) -> f32 {
        self.values[id.index()]
    }

    /// The field-named view the rest of the game reads. Whole-number stats are
    /// already rounded, so nothing is converted here — only named.
    pub fn as_stats(&self) -> WeaponStats {
        WeaponStats {
            class: self.class,
            damage: self.stat(StatId::Damage),
            pellets: self.stat(StatId::Pellets) as u32,
            fire_interval: self.stat(StatId::FireInterval),
            mag_size: self.stat(StatId::MagSize) as u32,
            reload_time: self.stat(StatId::ReloadTime),
            base_spread: self.stat(StatId::BaseSpread),
            bloom_per_shot: self.stat(StatId::BloomPerShot),
            max_bloom: self.stat(StatId::MaxBloom),
            bloom_decay: self.stat(StatId::BloomDecay),
            recoil_pitch: self.stat(StatId::RecoilPitch),
            recoil_yaw: self.stat(StatId::RecoilYaw),
            ads_fov: self.stat(StatId::AdsFov),
            move_scale: self.stat(StatId::MoveScale),
            headshot_mult: self.stat(StatId::HeadshotMult),
            falloff_start: self.stat(StatId::FalloffStart),
            falloff_end: self.stat(StatId::FalloffEnd),
            falloff_min: self.stat(StatId::FalloffMin),
            fire_modes: self.fire_modes,
            burst_count: self.stat(StatId::BurstCount) as u32,
            infinite_ammo: self.infinite_ammo,
            melee: self.melee,
        }
    }
}

/// The part the catalogue files under `id`, or `None` when it has never heard
/// of it — a build from another version, or a typo.
fn part_def(id: PartId) -> Option<&'static PartDef> {
    CATALOG.iter().find(|p| p.id == id)
}

/// The static slot id a settings string names, or `None`. Ids are
/// `&'static str` by design, so text read back from a file is matched to the
/// table rather than carried as itself; the subslots a part can open are
/// deliberately not nameable this way.
fn slot_named(name: &str) -> Option<SlotId> {
    GUN_SLOTS.iter().map(|s| s.id).find(|id| id.0 == name)
}

/// The static part id a settings string names, or `None`.
fn part_named(name: &str) -> Option<PartId> {
    CATALOG.iter().map(|p| p.id).find(|id| id.0 == name)
}

/// The parts the game ships, as the armory installs them.
///
/// Every one of them has to be a choice rather than a rung: within a slot the
/// parts trade different stats against each other, or the "best" build picks
/// itself and the bench is a formality.
mod parts {
    use super::*;

    pub const MICRO_DOT: PartDef = PartDef {
        id: PartId("micro_dot"),
        slot: SlotId("optic"),
        interface: InterfaceId("rail"),
        subslots: &[],
        mods: &[StatMod {
            stat: StatId::AdsFov,
            kind: ModKind::Add,
            amount: Value::Static(-4.0),
        }],
    };

    /// Magnified: it trades field of view for a slower walk.
    pub const MARKSMAN_SCOPE: PartDef = PartDef {
        id: PartId("marksman_scope"),
        slot: SlotId("optic"),
        interface: InterfaceId("rail"),
        subslots: &[],
        mods: &[
            StatMod {
                stat: StatId::AdsFov,
                kind: ModKind::Add,
                amount: Value::Static(-10.0),
            },
            StatMod {
                stat: StatId::MoveScale,
                kind: ModKind::MulPre,
                amount: Value::Static(0.94),
            },
        ],
    };

    pub const COMPENSATOR: PartDef = PartDef {
        id: PartId("compensator"),
        slot: SlotId("muzzle"),
        interface: InterfaceId("threaded_muzzle"),
        subslots: &[],
        mods: &[
            StatMod {
                stat: StatId::RecoilPitch,
                kind: ModKind::MulPre,
                amount: Value::Static(0.75),
            },
            StatMod {
                stat: StatId::RecoilYaw,
                kind: ModKind::MulPre,
                amount: Value::Static(0.75),
            },
        ],
    };

    /// Tames the walk at full auto less than the compensator, but keeps the
    /// burst tight.
    pub const FLASH_HIDER: PartDef = PartDef {
        id: PartId("flash_hider"),
        slot: SlotId("muzzle"),
        interface: InterfaceId("threaded_muzzle"),
        subslots: &[],
        mods: &[
            StatMod {
                stat: StatId::BaseSpread,
                kind: ModKind::MulPre,
                amount: Value::Static(0.88),
            },
            StatMod {
                stat: StatId::RecoilYaw,
                kind: ModKind::MulPre,
                amount: Value::Static(0.90),
            },
        ],
    };

    /// Longer reach, and a barrel that pulls harder for it.
    pub const HEAVY_BARREL: PartDef = PartDef {
        id: PartId("heavy_barrel"),
        slot: SlotId("muzzle"),
        interface: InterfaceId("threaded_muzzle"),
        subslots: &[],
        mods: &[
            StatMod {
                stat: StatId::FalloffStart,
                kind: ModKind::MulPre,
                amount: Value::Static(1.35),
            },
            StatMod {
                stat: StatId::FalloffEnd,
                kind: ModKind::MulPre,
                amount: Value::Static(1.25),
            },
            StatMod {
                stat: StatId::RecoilPitch,
                kind: ModKind::MulPre,
                amount: Value::Static(1.10),
            },
        ],
    };

    pub const EXTENDED_MAG: PartDef = PartDef {
        id: PartId("extended_mag"),
        slot: SlotId("magazine"),
        interface: InterfaceId("magwell"),
        subslots: &[],
        mods: &[
            StatMod {
                stat: StatId::MagSize,
                kind: ModKind::Add,
                amount: Value::Static(10.0),
            },
            StatMod {
                stat: StatId::ReloadTime,
                kind: ModKind::Add,
                amount: Value::Static(0.15),
            },
        ],
    };

    /// A magazine that comes out faster, for a magazine that holds less.
    pub const QUICK_MAG: PartDef = PartDef {
        id: PartId("quick_mag"),
        slot: SlotId("magazine"),
        interface: InterfaceId("magwell"),
        subslots: &[],
        mods: &[
            StatMod {
                stat: StatId::ReloadTime,
                // `MulPost`: the reload time is snapped into its envelope and
                // rounded first, then shortened — the stage that reads "after
                // the design bounds, this is what handling buys".
                kind: ModKind::MulPost,
                amount: Value::Static(0.72),
            },
            StatMod {
                stat: StatId::MagSize,
                kind: ModKind::MulPre,
                amount: Value::Static(0.85),
            },
        ],
    };

    /// Surplus ammunition: the same crate, a different lot. A range is what
    /// makes two copies of one part feel different without a second part, and
    /// this is the one shipping part whose numbers are rolled per copy.
    pub const SURPLUS_LOT: PartDef = PartDef {
        id: PartId("surplus_lot"),
        slot: SlotId("magazine"),
        interface: InterfaceId("magwell"),
        subslots: &[],
        mods: &[StatMod {
            stat: StatId::Damage,
            kind: ModKind::Add,
            amount: Value::Range {
                min: -2.0,
                max: 6.0,
            },
        }],
    };

    /// The shipping catalogue, as one list: what the game resolves against
    /// before the tests add their own parts to the copy they build.
    pub const SHIPPED: &[PartDef] = &[
        MICRO_DOT,
        MARKSMAN_SCOPE,
        COMPENSATOR,
        FLASH_HIDER,
        HEAVY_BARREL,
        EXTENDED_MAG,
        QUICK_MAG,
        SURPLUS_LOT,
    ];
}

/// The catalogue: what the armory installs, and what a saved build may name.
#[cfg(not(test))]
static CATALOG: &[PartDef] = parts::SHIPPED;

/// The catalogue the tests resolve against: the shipping parts plus the demo
/// and probe parts `demo_parts` keeps for exercising the model itself.
#[cfg(test)]
static CATALOG: &[PartDef] = &[
    parts::MICRO_DOT,
    parts::MARKSMAN_SCOPE,
    parts::COMPENSATOR,
    parts::FLASH_HIDER,
    parts::HEAVY_BARREL,
    parts::EXTENDED_MAG,
    parts::QUICK_MAG,
    parts::SURPLUS_LOT,
    demo_parts::RISER,
    demo_parts::SCOPE,
    demo_parts::PROBE_FLAT,
    demo_parts::PROBE_X3,
    demo_parts::PROBE_HALF,
    demo_parts::PROBE_HEAVY,
    demo_parts::PROBE_PELLETS,
];

/// Every part the game knows, for the armory to offer.
pub fn catalogue() -> &'static [PartDef] {
    CATALOG
}

/// A part's name for the bench, in both languages. `None` for an id this build
/// does not know — the armory prints the raw id for such a line rather than
/// hiding it, because a save from another version is worth seeing.
pub fn part_label(id: PartId) -> Option<(&'static str, &'static str)> {
    Some(match id.0 {
        "micro_dot" => ("MICRO DOT", "微型红点"),
        "marksman_scope" => ("MARKSMAN SCOPE", "射手瞄准镜"),
        "compensator" => ("COMPENSATOR", "补偿器"),
        "flash_hider" => ("FLASH HIDER", "消焰器"),
        "heavy_barrel" => ("HEAVY BARREL", "重枪管"),
        "extended_mag" => ("EXTENDED MAG", "扩容弹匣"),
        "quick_mag" => ("QUICK MAG", "快拔弹匣"),
        "surplus_lot" => ("SURPLUS LOT", "库存弹药"),
        _ => return None,
    })
}

/// A slot's name for the bench, in both languages.
pub fn slot_label(id: SlotId) -> Option<(&'static str, &'static str)> {
    match id.0 {
        "optic" => Some(("OPTIC", "瞄具")),
        "muzzle" => Some(("MUZZLE", "枪口")),
        "magazine" => Some(("MAGAZINE", "弹匣")),
        _ => None,
    }
}

/// Roll a def's own base value is sampled with; see `WeaponDef::base`.
const DEF_BASE_ROLL: f32 = 0.5;

/// The slots a weapon offers with this build installed: the def's own, plus
/// every subslot an installed part opens.
///
/// Walked to a fixpoint rather than in list order, so a part may be listed
/// before the part that opened its slot. A build is a set of choices, not a
/// sequence of assembly steps.
pub fn reachable_slots(def: &WeaponDef, build: &WeaponBuild) -> Vec<SlotDef> {
    let mut slots = def.slots.to_vec();
    let mut changed = true;
    while changed {
        changed = false;
        for entry in &build.entries {
            let Some(slot) = slots.iter().find(|s| s.id == entry.slot).copied() else {
                continue;
            };
            let Some(part) = part_def(entry.part) else {
                continue;
            };
            // A part that does not seat properly opens nothing.
            if part.slot != entry.slot || part.interface != slot.accepts {
                continue;
            }
            for sub in part.subslots {
                if !slots.iter().any(|s| s.id == sub.id) {
                    slots.push(*sub);
                    changed = true;
                }
            }
        }
    }
    slots
}

/// The parts a build lands, each with the roll its entry carried: everything
/// `validate` would accept.
///
/// Resolution drops an entry it cannot accept instead of guessing at it, so a
/// build that was never validated degrades to the parts that do fit rather than
/// applying a muzzle brake as a stock.
fn installed(def: &WeaponDef, build: &WeaponBuild) -> Vec<(f32, &'static [StatMod])> {
    let slots = reachable_slots(def, build);
    build
        .entries
        .iter()
        .filter_map(|entry| {
            let slot = slots.iter().find(|s| s.id == entry.slot)?;
            let part = part_def(entry.part)?;
            (part.slot == entry.slot && part.interface == slot.accepts)
                .then_some((entry.roll, part.mods))
        })
        .collect()
}

/// Push a build through a def: what one weapon built this way ends up with.
///
/// Pure. Both inputs are read and never written, and every roll comes from the
/// build, so the same pair always resolves to the same numbers.
pub fn resolve(def: &WeaponDef, build: &WeaponBuild) -> ResolvedStats {
    let parts = installed(def, build);
    let mut values = [0.0; STAT_COUNT];
    for stat in StatId::ALL {
        // One pass per stat into three buckets. Bucketing is what keeps the
        // stages from being applied in the order the build happened to list its
        // parts in: the order is the contract, not the file.
        let (mut add, mut pre, mut post) = (0.0, 1.0, 1.0);
        for (roll, mods) in &parts {
            for m in mods.iter().filter(|m| m.stat == stat) {
                match m.modifier(*roll) {
                    Modifier::Add(n) => add += n,
                    Modifier::MulPre(n) => pre *= n,
                    Modifier::MulPost(n) => post *= n,
                }
            }
        }
        // Add, then MulPre, then the envelope and rounding, then MulPost — the
        // order documented on `Modifier`.
        let (lo, hi) = stat.bounds();
        let mut value = (def.base[stat.index()].sample(DEF_BASE_ROLL) + add) * pre;
        if stat.whole() {
            value = value.round();
        }
        value = value.clamp(lo, hi) * post;
        // A post multiplier may leave the design envelope, but not the realm of
        // numbers a weapon can be simulated with.
        values[stat.index()] = if value.is_nan() { lo } else { value.max(0.0) };
    }
    ResolvedStats {
        class: def.class,
        fire_modes: def.fire_modes,
        infinite_ammo: def.infinite_ammo,
        melee: def.melee,
        values,
    }
}

/// A reason a build is not legal. Every one of them is a data error — a loadout
/// table, a tutorial reward, a save from another version — not a runtime state,
/// so `validate` reports all of them instead of stopping at the first and
/// callers log the list rather than picking a favourite.
#[derive(Clone, PartialEq, Debug)]
pub enum Violation {
    /// The build names a slot the weapon does not offer, not even with every
    /// part that is installed opening what it can.
    UnknownSlot(SlotId),
    /// The catalogue has no such part.
    UnknownPart(PartId),
    /// The part was built for a different slot.
    WrongSlot {
        slot: SlotId,
        part: PartId,
        built_for: SlotId,
    },
    /// The slot takes a different mounting standard.
    WrongInterface {
        slot: SlotId,
        part: PartId,
        accepts: InterfaceId,
        declares: InterfaceId,
    },
    /// More parts in one slot than it holds.
    Overfull {
        slot: SlotId,
        max: usize,
        found: usize,
    },
}

/// Check a build against a def: the one gate that a mission load, a tutorial, an
/// achievement and a network check all share, so that they cannot disagree about
/// what a legal weapon is. Pure, like `resolve`, and it keeps going after the
/// first fault so a screen can show the whole list at once.
pub fn validate(def: &WeaponDef, build: &WeaponBuild) -> Result<(), Vec<Violation>> {
    let slots = reachable_slots(def, build);
    let mut violations = Vec::new();
    for entry in &build.entries {
        let Some(slot) = slots.iter().find(|s| s.id == entry.slot) else {
            violations.push(Violation::UnknownSlot(entry.slot));
            continue;
        };
        let Some(part) = part_def(entry.part) else {
            violations.push(Violation::UnknownPart(entry.part));
            continue;
        };
        if part.slot != entry.slot {
            violations.push(Violation::WrongSlot {
                slot: entry.slot,
                part: entry.part,
                built_for: part.slot,
            });
        }
        if part.interface != slot.accepts {
            violations.push(Violation::WrongInterface {
                slot: entry.slot,
                part: entry.part,
                accepts: slot.accepts,
                declares: part.interface,
            });
        }
    }
    for slot in &slots {
        let found = build.entries.iter().filter(|e| e.slot == slot.id).count();
        if found > slot.max_parts {
            violations.push(Violation::Overfull {
                slot: slot.id,
                max: slot.max_parts,
                found,
            });
        }
    }
    if violations.is_empty() {
        Ok(())
    } else {
        Err(violations)
    }
}

/// Shorthand for the only `Value` a shipping weapon uses.
const fn st(v: f32) -> Value {
    Value::Static(v)
}

/// The slots a gun offers before anything is installed: an optic rail, a
/// threaded muzzle and a magazine well.
///
/// One table for every gun for now. The standards are the part of this that
/// matters — a shotgun choke and a rifle brake are different standards the day
/// a part needs them to be, and splitting one is a line here, not a rewrite.
/// The knife offers nothing, which is why it is absent.
const GUN_SLOTS: &[SlotDef] = &[
    SlotDef {
        id: SlotId("optic"),
        accepts: InterfaceId("rail"),
        max_parts: 1,
    },
    SlotDef {
        id: SlotId("muzzle"),
        accepts: InterfaceId("threaded_muzzle"),
        max_parts: 1,
    },
    SlotDef {
        id: SlotId("magazine"),
        accepts: InterfaceId("magwell"),
        max_parts: 1,
    },
];

/// The rifle. Every number is the one the pre-resolver table held, pinned by
/// `the_shipped_tables_survive_the_resolver_bit_for_bit` below.
const RIFLE: WeaponDef = WeaponDef {
    class: SlotClass::Primary,
    fire_modes: RIFLE_MODES,
    infinite_ammo: false,
    melee: false,
    base: [
        st(24.0),   // Damage
        st(1.0),    // Pellets
        st(0.088),  // FireInterval
        st(30.0),   // MagSize
        st(1.35),   // ReloadTime
        st(0.0042), // BaseSpread
        st(0.0055), // BloomPerShot
        st(0.040),  // MaxBloom
        st(0.085),  // BloomDecay
        st(0.0165), // RecoilPitch
        st(0.0042), // RecoilYaw
        st(42.0),   // AdsFov
        st(1.0),    // MoveScale
        st(2.4),    // HeadshotMult
        st(30.0),   // FalloffStart
        st(70.0),   // FalloffEnd
        st(0.55),   // FalloffMin
        st(3.0),    // BurstCount
    ],
    slots: GUN_SLOTS,
};

const SHOTGUN: WeaponDef = WeaponDef {
    class: SlotClass::Primary,
    fire_modes: SEMI,
    infinite_ammo: false,
    melee: false,
    base: [
        st(15.0),  // Damage
        st(9.0),   // Pellets
        st(0.78),  // FireInterval
        st(6.0),   // MagSize
        st(2.30),  // ReloadTime
        st(0.055), // BaseSpread
        st(0.0),   // BloomPerShot
        st(0.0),   // MaxBloom
        st(0.1),   // BloomDecay
        st(0.055), // RecoilPitch
        st(0.012), // RecoilYaw
        st(50.0),  // AdsFov
        st(0.94),  // MoveScale
        st(1.6),   // HeadshotMult
        st(8.0),   // FalloffStart
        st(26.0),  // FalloffEnd
        st(0.25),  // FalloffMin
        st(3.0),   // BurstCount
    ],
    slots: GUN_SLOTS,
};

const SMG: WeaponDef = WeaponDef {
    class: SlotClass::Primary,
    fire_modes: SMG_MODES,
    infinite_ammo: false,
    melee: false,
    base: [
        st(14.0),   // Damage
        st(1.0),    // Pellets
        st(0.055),  // FireInterval
        st(40.0),   // MagSize
        st(1.50),   // ReloadTime
        st(0.0105), // BaseSpread
        st(0.0045), // BloomPerShot
        st(0.045),  // MaxBloom
        st(0.110),  // BloomDecay
        st(0.0105), // RecoilPitch
        st(0.0055), // RecoilYaw
        st(46.0),   // AdsFov
        st(1.05),   // MoveScale
        st(2.0),    // HeadshotMult
        st(18.0),   // FalloffStart
        st(45.0),   // FalloffEnd
        st(0.50),   // FalloffMin
        st(3.0),    // BurstCount
    ],
    slots: GUN_SLOTS,
};

const SNIPER: WeaponDef = WeaponDef {
    class: SlotClass::Primary,
    fire_modes: SEMI,
    infinite_ammo: false,
    melee: false,
    base: [
        st(115.0),  // Damage
        st(1.0),    // Pellets
        st(1.15),   // FireInterval
        st(5.0),    // MagSize
        st(2.50),   // ReloadTime
        st(0.0009), // BaseSpread
        st(0.020),  // BloomPerShot
        st(0.030),  // MaxBloom
        st(0.500),  // BloomDecay
        st(0.075),  // RecoilPitch
        st(0.010),  // RecoilYaw
        st(16.0),   // AdsFov
        st(0.85),   // MoveScale
        st(2.2),    // HeadshotMult
        st(1000.0), // FalloffStart
        st(1000.0), // FalloffEnd
        st(1.0),    // FalloffMin
        st(3.0),    // BurstCount
    ],
    slots: GUN_SLOTS,
};

const LMG: WeaponDef = WeaponDef {
    class: SlotClass::Primary,
    fire_modes: AUTO_SEMI,
    infinite_ammo: false,
    melee: false,
    base: [
        st(21.0),   // Damage
        st(1.0),    // Pellets
        st(0.075),  // FireInterval
        st(100.0),  // MagSize
        st(3.40),   // ReloadTime
        st(0.0135), // BaseSpread
        st(0.0075), // BloomPerShot
        st(0.075),  // MaxBloom
        st(0.050),  // BloomDecay
        st(0.0135), // RecoilPitch
        st(0.0060), // RecoilYaw
        st(48.0),   // AdsFov
        st(0.82),   // MoveScale
        st(2.0),    // HeadshotMult
        st(35.0),   // FalloffStart
        st(80.0),   // FalloffEnd
        st(0.60),   // FalloffMin
        st(3.0),    // BurstCount
    ],
    slots: GUN_SLOTS,
};

const PISTOL: WeaponDef = WeaponDef {
    class: SlotClass::Pistol,
    fire_modes: SEMI,
    infinite_ammo: false,
    melee: false,
    base: [
        st(26.0),   // Damage
        st(1.0),    // Pellets
        st(0.160),  // FireInterval
        st(15.0),   // MagSize
        st(1.25),   // ReloadTime
        st(0.0060), // BaseSpread
        st(0.0080), // BloomPerShot
        st(0.038),  // MaxBloom
        st(0.160),  // BloomDecay
        st(0.0220), // RecoilPitch
        st(0.0030), // RecoilYaw
        st(48.0),   // AdsFov
        st(1.10),   // MoveScale
        st(2.6),    // HeadshotMult
        st(14.0),   // FalloffStart
        st(45.0),   // FalloffEnd
        st(0.45),   // FalloffMin
        st(3.0),    // BurstCount
    ],
    slots: GUN_SLOTS,
};

const KNIFE: WeaponDef = WeaponDef {
    class: SlotClass::Melee,
    fire_modes: SEMI,
    infinite_ammo: true,
    melee: true,
    base: [
        st(55.0),   // Damage
        st(1.0),    // Pellets
        st(0.55),   // FireInterval
        st(0.0),    // MagSize
        st(0.0),    // ReloadTime
        st(0.0),    // BaseSpread
        st(0.0),    // BloomPerShot
        st(0.0),    // MaxBloom
        st(0.1),    // BloomDecay
        st(0.030),  // RecoilPitch
        st(0.0),    // RecoilYaw
        st(55.0),   // AdsFov
        st(1.18),   // MoveScale
        st(1.0),    // HeadshotMult
        st(1000.0), // FalloffStart
        st(1000.0), // FalloffEnd
        st(1.0),    // FalloffMin
        st(3.0),    // BurstCount
    ],
    slots: &[],
};

/// The authoring table for a weapon kind.
pub fn def(kind: WeaponKind) -> &'static WeaponDef {
    match kind {
        WeaponKind::Rifle => &RIFLE,
        WeaponKind::Shotgun => &SHOTGUN,
        WeaponKind::Smg => &SMG,
        WeaponKind::Sniper => &SNIPER,
        WeaponKind::Lmg => &LMG,
        WeaponKind::Pistol => &PISTOL,
        WeaponKind::Knife => &KNIFE,
    }
}

impl WeaponKind {
    pub fn name(self) -> Key {
        match self {
            WeaponKind::Rifle => Key::WeaponRifle,
            WeaponKind::Shotgun => Key::WeaponShotgun,
            WeaponKind::Smg => Key::WeaponSmg,
            WeaponKind::Sniper => Key::WeaponSniper,
            WeaponKind::Lmg => Key::WeaponLmg,
            WeaponKind::Pistol => Key::WeaponPistol,
            WeaponKind::Knife => Key::WeaponKnife,
        }
    }

    /// The share of a hit this weapon's fire carries through physical plate
    /// (reference §17.2③ — "one visible armour-penetration number per
    /// weapon", so "how does this gun do against a Juggernaut" is a decision
    /// the player can make).
    ///
    /// Anchored on CS2's published values (AK-47 77.5%, M4A4 70%, Glock-18
    /// 47%, Tec-9 90.6%, SG 553 100%, AWP 97.5%): rifles pierce, buckshot
    /// and pistol rounds mostly do not, and a knife does not care about
    /// plate at all. The enemy side of the exchange is `enemy::armor`.
    pub fn armor_pen(self) -> f32 {
        match self {
            WeaponKind::Rifle => 0.78,
            WeaponKind::Shotgun => 0.30,
            WeaponKind::Smg => 0.55,
            WeaponKind::Sniper => 0.97,
            WeaponKind::Lmg => 0.85,
            WeaponKind::Pistol => 0.45,
            WeaponKind::Knife => 1.0,
        }
    }

    /// Short tag for the HUD slot strip.
    pub fn tag(self) -> &'static str {
        match self {
            WeaponKind::Rifle => "RIF",
            WeaponKind::Shotgun => "SG",
            WeaponKind::Smg => "SMG",
            WeaponKind::Sniper => "SNP",
            WeaponKind::Lmg => "LMG",
            WeaponKind::Pistol => "PST",
            WeaponKind::Knife => "KNF",
        }
    }

    /// Position in `ALL_KINDS`: the index of every per-kind table the game
    /// keeps — the armory's kit, the viewmodel's model cache.
    pub fn index(self) -> usize {
        match self {
            WeaponKind::Rifle => 0,
            WeaponKind::Shotgun => 1,
            WeaponKind::Smg => 2,
            WeaponKind::Sniper => 3,
            WeaponKind::Lmg => 4,
            WeaponKind::Pistol => 5,
            WeaponKind::Knife => 6,
        }
    }

    /// The name a settings key is built from: `build_rifle`, `paint_rifle`.
    pub fn key(self) -> &'static str {
        match self {
            WeaponKind::Rifle => "rifle",
            WeaponKind::Shotgun => "shotgun",
            WeaponKind::Smg => "smg",
            WeaponKind::Sniper => "sniper",
            WeaponKind::Lmg => "lmg",
            WeaponKind::Pistol => "pistol",
            WeaponKind::Knife => "knife",
        }
    }

    pub fn class(self) -> SlotClass {
        self.stats().class
    }

    /// The weapon as it comes out of the crate: its def resolved against an
    /// empty build. An instance resolves its own build — see `Weapon::stats` —
    /// and this is the kind-level answer, from the same code path, so the two
    /// cannot drift apart.
    pub fn stats(self) -> WeaponStats {
        resolve(def(self), &WeaponBuild::empty()).as_stats()
    }
}

pub struct Weapon {
    pub kind: WeaponKind,
    pub mag: u32,
    /// Rounds carried outside the magazine — what a reload draws on. The stash
    /// decides how much a weapon goes into a mission with; ammo pickups top it
    /// up.
    pub reserve: u32,
    pub cooldown: f32,
    pub reload_t: f32,
    pub bloom: f32,
    pub kick: f32,
    pub flash: f32,
    pub shots: u32,
    /// Index into `stats().fire_modes`.
    pub fire_mode: usize,
    burst_left: u32,
    pub inspect_t: f32,
    pub mag_check_t: f32,
    /// What this instance is built with, and the numbers that came out of it.
    /// Both belong here rather than to the def, which is shared by every weapon
    /// of its kind: a build's outcome must never be written back into one.
    build: WeaponBuild,
    resolved: ResolvedStats,
}

impl Weapon {
    /// The weapon as it comes out of the crate: its def resolved against an
    /// empty build.
    // The game reaches every carried weapon through the kit, so nothing in the
    // shipping path calls this; the tests and any kit-less reader use it.
    #[allow(dead_code)]
    pub fn new(kind: WeaponKind) -> Self {
        Self::with_build(kind, WeaponBuild::empty())
    }

    /// A weapon carrying `build`. The def is read and the numbers it resolves to
    /// are kept on this instance.
    pub fn with_build(kind: WeaponKind, build: WeaponBuild) -> Self {
        let resolved = resolve(def(kind), &build);
        Self {
            kind,
            mag: resolved.as_stats().mag_size,
            reserve: 0,
            cooldown: 0.0,
            reload_t: 0.0,
            bloom: 0.0,
            kick: 0.0,
            flash: 0.0,
            shots: 0,
            fire_mode: 0,
            burst_left: 0,
            inspect_t: 0.0,
            mag_check_t: 0.0,
            build,
            resolved,
        }
    }

    /// The build this instance carries, for whoever wants to show, check or
    /// change it.
    pub fn build(&self) -> &WeaponBuild {
        &self.build
    }

    pub fn stats(&self) -> WeaponStats {
        self.resolved.as_stats()
    }

    pub fn mode(&self) -> FireMode {
        let modes = self.stats().fire_modes;
        modes[self.fire_mode.min(modes.len() - 1)]
    }

    pub fn cycle_fire_mode(&mut self) -> FireMode {
        let modes = self.stats().fire_modes;
        self.fire_mode = (self.fire_mode + 1) % modes.len();
        self.burst_left = 0;
        self.mode()
    }

    pub fn is_reloading(&self) -> bool {
        self.reload_t > 0.0
    }

    pub fn reload_progress(&self) -> f32 {
        let t = self.stats().reload_time;
        if t <= 0.0 {
            return 1.0;
        }
        1.0 - (self.reload_t / t).clamp(0.0, 1.0)
    }

    pub fn start_inspect(&mut self) {
        if !self.is_reloading() {
            self.inspect_t = 2.2;
        }
    }

    /// Coarse magazine check — you get a band, not an exact count.
    pub fn mag_estimate(&self) -> Key {
        let s = self.stats();
        if s.infinite_ammo {
            return Key::MagFull;
        }
        if self.mag == 0 {
            return Key::MagEmpty;
        }
        if self.mag >= s.mag_size {
            return Key::MagFull;
        }
        let f = self.mag as f32 / s.mag_size.max(1) as f32;
        if f > 0.6 {
            Key::MagHigh
        } else if f > 0.3 {
            Key::MagHalf
        } else {
            Key::MagLow
        }
    }

    /// Rounds found in the field: the magazine fills first and the rest goes
    /// into the reserve, so a pickup is never wasted on a full magazine.
    /// Returns how many were taken.
    pub fn add_ammo(&mut self, rounds: u32) -> u32 {
        if self.stats().infinite_ammo {
            return 0;
        }
        let into_mag = self.stats().mag_size.saturating_sub(self.mag).min(rounds);
        self.mag += into_mag;
        let into_reserve = rounds - into_mag;
        self.reserve += into_reserve;
        into_mag + into_reserve
    }

    /// Spread in radians for the next shot.
    pub fn spread(&self, move_ratio: f32, ads: f32, stance_scale: f32) -> f32 {
        let s = self.stats();
        (s.base_spread * (1.0 - ads * 0.55) + 0.020 * move_ratio * (1.0 - ads * 0.35) + self.bloom)
            * stance_scale
    }

    pub fn can_fire(&self) -> bool {
        if self.cooldown > 0.0 || self.is_reloading() {
            return false;
        }
        self.stats().infinite_ammo || self.mag > 0
    }

    /// `pressed` is the frame the trigger went down, `held` is the sustained state.
    pub fn try_fire(&mut self, pressed: bool, held: bool) -> bool {
        if !self.can_fire() {
            return false;
        }
        let s = self.stats();
        let mode = self.mode();
        let allowed = match mode {
            FireMode::Auto => held,
            FireMode::Single => pressed,
            FireMode::Burst => self.burst_left > 0 || pressed,
        };
        if !allowed {
            return false;
        }

        if !s.infinite_ammo {
            self.mag -= 1;
        }
        let burst = mode == FireMode::Burst;
        if burst {
            if self.burst_left == 0 {
                self.burst_left = s.burst_count;
            }
            self.burst_left -= 1;
            if self.burst_left == 0 {
                self.cooldown = s.fire_interval;
            } else {
                self.cooldown = s.fire_interval * 0.42;
            }
        } else {
            self.cooldown = s.fire_interval;
        }

        self.bloom = (self.bloom + s.bloom_per_shot).min(s.max_bloom);
        self.kick = (self.kick + 1.0).min(2.4);
        self.flash = 1.0;
        self.shots += 1;
        self.inspect_t = 0.0;
        true
    }

    pub fn start_reload(&mut self) -> bool {
        // A reload moves rounds *from the reserve into the magazine*: with an
        // empty reserve there is nothing to move, and the click is dry.
        if self.stats().infinite_ammo
            || self.is_reloading()
            || self.mag >= self.stats().mag_size
            || self.reserve == 0
        {
            return false;
        }
        self.reload_t = self.stats().reload_time;
        self.inspect_t = 0.0;
        self.burst_left = 0;
        true
    }

    pub fn update(&mut self, dt: f32) {
        let s = self.stats();
        self.cooldown = (self.cooldown - dt).max(0.0);
        self.flash = approach(self.flash, 0.0, dt * 16.0);
        self.bloom = approach(self.bloom, 0.0, dt * s.bloom_decay);
        self.kick = approach(self.kick, 0.0, dt * 9.0);
        self.inspect_t = (self.inspect_t - dt).max(0.0);
        self.mag_check_t = (self.mag_check_t - dt).max(0.0);

        if self.reload_t > 0.0 {
            self.reload_t -= dt;
            if self.reload_t <= 0.0 {
                self.reload_t = 0.0;
                // The magazine takes what the reserve has: a reload out of a
                // short reserve loads a partial magazine, which is what a
                // finite reserve means.
                let want = s.mag_size.saturating_sub(self.mag);
                let take = want.min(self.reserve);
                self.mag += take;
                self.reserve -= take;
                self.bloom = 0.0;
            }
        }
    }

    pub fn damage_at(&self, dist: f32) -> f32 {
        let s = self.stats();
        if dist <= s.falloff_start || s.falloff_end <= s.falloff_start {
            return s.damage;
        }
        let t = ((dist - s.falloff_start) / (s.falloff_end - s.falloff_start)).clamp(0.0, 1.0);
        s.damage * (1.0 - t * (1.0 - s.falloff_min))
    }
}

/// Four fixed slots — primary, secondary, sidearm, melee — matching keys 1-4.
pub struct Arsenal {
    slots: [Option<Weapon>; 4],
    pub active: usize,
    pub switch_t: f32,
}

impl Arsenal {
    /// A sidearm and a knife are always carried; `loadout` fills the two long
    /// slots, in order.
    ///
    /// The array is exactly as long as the number of long-gun slots, so a
    /// mission cannot ask for more weapons than it will be given. The slice
    /// this used to take was walked with a `continue` past anything that did
    /// not fit, which is how mission 5 shipped declaring five weapons and
    /// carrying two without a word to anyone.
    // The game always carries the kit, so this is the kit-less path the tests
    // (and any future "plain loadout" reader) use.
    #[allow(dead_code)]
    pub fn new(loadout: [Option<WeaponKind>; 2]) -> Self {
        Self::with_builds(loadout, &std::array::from_fn(|_| WeaponBuild::empty()))
    }

    /// [`Arsenal::new`] with the player's kit: each weapon is carried with the
    /// build the armory gave its kind, kept on the instance so no kind-mate
    /// sees it. `Kit` screens the builds before they get here; `carry` checks
    /// them again on the way in.
    pub fn with_builds(loadout: [Option<WeaponKind>; 2], builds: &[WeaponBuild; 7]) -> Self {
        let mut ars = Self {
            slots: [None, None, None, None],
            active: 0,
            switch_t: 0.0,
        };
        for (i, k) in loadout.into_iter().enumerate() {
            // A melee weapon never takes a long-gun slot — that is what slot 4
            // is for — and the sidearm is always carried.
            if let Some(k) = k.filter(|k| k.class() != SlotClass::Melee) {
                ars.slots[i] = Some(Self::carry(k, builds[k.index()].clone()));
            }
        }
        let pistol = WeaponKind::Pistol.index();
        let knife = WeaponKind::Knife.index();
        ars.slots[2] = Some(Self::carry(WeaponKind::Pistol, builds[pistol].clone()));
        ars.slots[3] = Some(Self::carry(WeaponKind::Knife, builds[knife].clone()));
        ars
    }

    /// One carried weapon, checked against its def before it is handed out.
    ///
    /// A build that breaks the gunsmith rules is reported and carried anyway:
    /// the loadout a mission issues and the armory screening both aim at clean
    /// builds, so a violation here means a table somewhere is wrong, and a
    /// mission that refuses to start is a worse answer to a typo than a log
    /// line and the weapon as issued.
    fn carry(kind: WeaponKind, build: WeaponBuild) -> Weapon {
        let w = Weapon::with_build(kind, build);
        if let Err(bad) = validate(def(kind), w.build()) {
            eprintln!("{kind:?} does not build cleanly: {bad:?}");
        }
        w
    }

    pub fn slot(&self, i: usize) -> Option<&Weapon> {
        self.slots.get(i).and_then(|w| w.as_ref())
    }

    /// The weapon in slot `i`, for the few things that stock it directly —
    /// the stash's spare magazines, at mission start.
    pub fn slot_mut(&mut self, i: usize) -> Option<&mut Weapon> {
        self.slots.get_mut(i).and_then(|w| w.as_mut())
    }

    /// The active weapon, falling back to the first occupied slot.
    pub fn active_weapon(&self) -> &Weapon {
        self.slots
            .get(self.active)
            .and_then(|w| w.as_ref())
            .or_else(|| self.slots.iter().flatten().next())
            .expect("arsenal always carries at least a sidearm")
    }

    /// The slot the weapon in hand really lives in: `active` when that slot is
    /// occupied, and the first occupied slot otherwise — the fallback
    /// [`Arsenal::active_weapon`] reads through, in index form.
    fn held_index(&self) -> usize {
        if self.slots[self.active].is_some() {
            self.active
        } else {
            self.slots.iter().position(|w| w.is_some()).unwrap_or(0)
        }
    }

    pub fn active_weapon_mut(&mut self) -> &mut Weapon {
        let i = self.held_index();
        self.slots[i].as_mut().expect("slot checked above")
    }

    pub fn select(&mut self, i: usize) {
        if i >= self.slots.len() || i == self.active || self.slots[i].is_none() {
            return;
        }
        // Putting the weapon in hand away cancels whatever it was doing. A
        // reload used to keep ticking while holstered and quietly refill the
        // magazine behind the player's back — to the knife and back and the
        // rifle came up full with no reload ever seen. Only the weapon in hand
        // can be reloading (every `start_reload` goes through
        // `active_weapon_mut`), so cancelling this one is cancelling the reload.
        if let Some(w) = self.slots[self.held_index()].as_mut() {
            w.reload_t = 0.0;
        }
        self.active = i;
        self.switch_t = 0.34;
        // The weapon being drawn starts clean too: no carry-over inspect pose.
        if let Some(w) = self.slots[i].as_mut() {
            w.inspect_t = 0.0;
        }
    }

    pub fn cycle(&mut self, delta: i32) {
        let n = self.slots.len() as i32;
        let mut i = self.active as i32;
        for _ in 0..n {
            i = (i + delta).rem_euclid(n);
            if self.slots[i as usize].is_some() {
                self.select(i as usize);
                return;
            }
        }
    }

    pub fn update(&mut self, dt: f32) {
        self.switch_t = (self.switch_t - dt).max(0.0);
        for w in self.slots.iter_mut().flatten() {
            w.update(dt);
        }
    }

    pub fn move_scale(&self) -> f32 {
        self.active_weapon().stats().move_scale
    }
}

/// Parts for the tests to build with.
///
/// The shipping catalogue has its own parts now (see `parts`, above); these
/// stay beside the tests because nothing in the game should install them: one
/// part opens a slot for another, and the rest are arithmetic probes whose
/// names say what they do, so that the ordering tests are about the order and
/// not about anyone's idea of what a compensator should feel like.
#[cfg(test)]
mod demo_parts {
    use super::*;

    /// A slot with room for a handful of probe parts.
    pub const PROBE_SLOT: SlotDef = SlotDef {
        id: SlotId("probe"),
        accepts: InterfaceId("probe"),
        max_parts: 8,
    };

    /// A rail riser: no numbers of its own, one slot for whatever goes on top.
    pub const RISER: PartDef = PartDef {
        id: PartId("riser"),
        slot: SlotId("optic"),
        interface: InterfaceId("rail"),
        subslots: &[SlotDef {
            id: SlotId("riser_top"),
            accepts: InterfaceId("rail"),
            max_parts: 1,
        }],
        mods: &[],
    };

    pub const SCOPE: PartDef = PartDef {
        id: PartId("scope"),
        slot: SlotId("riser_top"),
        interface: InterfaceId("rail"),
        subslots: &[],
        mods: &[StatMod {
            stat: StatId::AdsFov,
            kind: ModKind::Add,
            amount: Value::Static(-8.0),
        }],
    };

    pub const PROBE_FLAT: PartDef = PartDef {
        id: PartId("probe_flat"),
        slot: PROBE_SLOT.id,
        interface: PROBE_SLOT.accepts,
        subslots: &[],
        mods: &[StatMod {
            stat: StatId::Damage,
            kind: ModKind::Add,
            amount: Value::Static(5.0),
        }],
    };

    /// Two `MulPre` lines on one part, in one stage, on one stat.
    pub const PROBE_X3: PartDef = PartDef {
        id: PartId("probe_x3"),
        slot: PROBE_SLOT.id,
        interface: PROBE_SLOT.accepts,
        subslots: &[],
        mods: &[
            StatMod {
                stat: StatId::Damage,
                kind: ModKind::MulPre,
                amount: Value::Static(2.0),
            },
            StatMod {
                stat: StatId::Damage,
                kind: ModKind::MulPre,
                amount: Value::Static(1.5),
            },
        ],
    };

    pub const PROBE_HALF: PartDef = PartDef {
        id: PartId("probe_half"),
        slot: PROBE_SLOT.id,
        interface: PROBE_SLOT.accepts,
        subslots: &[],
        mods: &[StatMod {
            stat: StatId::Damage,
            kind: ModKind::MulPost,
            amount: Value::Static(0.5),
        }],
    };

    /// Both multiply stages on a stat whose envelope is tight enough to tell
    /// them apart.
    pub const PROBE_HEAVY: PartDef = PartDef {
        id: PartId("probe_heavy"),
        slot: PROBE_SLOT.id,
        interface: PROBE_SLOT.accepts,
        subslots: &[],
        mods: &[
            StatMod {
                stat: StatId::MoveScale,
                kind: ModKind::MulPre,
                amount: Value::Static(3.0),
            },
            StatMod {
                stat: StatId::MoveScale,
                kind: ModKind::MulPost,
                amount: Value::Static(1.5),
            },
        ],
    };

    /// Both multiply stages on a whole-number stat, with the rounding stage
    /// sitting between them.
    pub const PROBE_PELLETS: PartDef = PartDef {
        id: PartId("probe_pellets"),
        slot: PROBE_SLOT.id,
        interface: PROBE_SLOT.accepts,
        subslots: &[],
        mods: &[
            StatMod {
                stat: StatId::Pellets,
                kind: ModKind::MulPre,
                amount: Value::Static(1.5),
            },
            StatMod {
                stat: StatId::Pellets,
                kind: ModKind::MulPost,
                amount: Value::Static(1.5),
            },
        ],
    };
}

#[cfg(test)]
mod tests {
    use super::demo_parts::*;
    use super::parts::*;
    use super::*;

    const FRAME: f32 = 1.0 / 60.0;

    /// Drives a weapon the way `main.rs` does: `held` is the button state and
    /// `pressed` is true only on the frame the button went down.
    fn run(w: &mut Weapon, held_frames: usize) -> usize {
        let mut shots = 0;
        for frame in 0..held_frames {
            w.update(FRAME);
            if w.try_fire(frame == 0, true) {
                shots += 1;
            }
        }
        shots
    }

    // ---- the resolver ------------------------------------------------------

    /// A build from `(slot, part, roll)` triples: the shape the rest of the game
    /// thinks in.
    fn build(entries: &[(SlotId, PartId, f32)]) -> WeaponBuild {
        WeaponBuild {
            entries: entries
                .iter()
                .map(|&(slot, part, roll)| BuildEntry { slot, part, roll })
                .collect(),
        }
    }

    /// A def with one stat set and slots to hang parts on. Every other stat is
    /// left at a value its own envelope keeps, so a test only has to think about
    /// the stat it names.
    fn probe_def(stat: StatId, base: f32, slots: &'static [SlotDef]) -> WeaponDef {
        let mut values = [st(1.0); STAT_COUNT];
        values[stat.index()] = st(base);
        WeaponDef {
            class: SlotClass::Primary,
            fire_modes: SEMI,
            infinite_ammo: false,
            melee: false,
            base: values,
            slots,
        }
    }

    /// A `WeaponStats` field read by id. Written out separately from the
    /// resolver's own mapping on purpose: a mix-up there — bloom decay taken
    /// from max bloom, say — has to fail the regression below, not agree with it.
    fn field(s: WeaponStats, stat: StatId) -> f32 {
        match stat {
            StatId::Damage => s.damage,
            StatId::Pellets => s.pellets as f32,
            StatId::FireInterval => s.fire_interval,
            StatId::MagSize => s.mag_size as f32,
            StatId::ReloadTime => s.reload_time,
            StatId::BaseSpread => s.base_spread,
            StatId::BloomPerShot => s.bloom_per_shot,
            StatId::MaxBloom => s.max_bloom,
            StatId::BloomDecay => s.bloom_decay,
            StatId::RecoilPitch => s.recoil_pitch,
            StatId::RecoilYaw => s.recoil_yaw,
            StatId::AdsFov => s.ads_fov,
            StatId::MoveScale => s.move_scale,
            StatId::HeadshotMult => s.headshot_mult,
            StatId::FalloffStart => s.falloff_start,
            StatId::FalloffEnd => s.falloff_end,
            StatId::FalloffMin => s.falloff_min,
            StatId::BurstCount => s.burst_count as f32,
        }
    }

    /// The documented stage order, on one stat: the flat bonus is summed in
    /// first, then both `MulPre`s, then the `MulPost` — `(10 + 5) * 2 * 1.5 *
    /// 0.5`. An `Add` applied last would give 20.0 and multiplies applied to the
    /// base first would give 15.0, so only one order produces 22.5.
    #[test]
    fn modifiers_run_add_then_mul_pre_then_mul_post() {
        let def = probe_def(StatId::Damage, 10.0, &[PROBE_SLOT]);
        let b = build(&[
            (PROBE_SLOT.id, PROBE_FLAT.id, 0.0),
            (PROBE_SLOT.id, PROBE_X3.id, 0.0),
            (PROBE_SLOT.id, PROBE_HALF.id, 0.0),
        ]);
        let got = resolve(&def, &b).stat(StatId::Damage);
        assert!(
            (got - 22.5).abs() < 1e-6,
            "the stages ran out of order: {got}"
        );
    }

    /// `MulPost` has the last word: it runs after the envelope, so it can push a
    /// stat past what the design allowed — 3.0 here, where the envelope stops at
    /// 2.0. Run before the envelope it would give 2.0, which is what makes the
    /// two stages worth distinguishing.
    #[test]
    fn a_post_multiplier_is_applied_after_the_envelope() {
        let def = probe_def(StatId::MoveScale, 1.0, &[PROBE_SLOT]);
        let b = build(&[(PROBE_SLOT.id, PROBE_HEAVY.id, 0.0)]);
        let got = resolve(&def, &b).stat(StatId::MoveScale);
        assert!(
            (got - 3.0).abs() < 1e-6,
            "the post stage was clamped: {got}"
        );
        assert!(got > StatId::MoveScale.bounds().1);
    }

    /// Whole-number stats are rounded between the stages, so the post multiplier
    /// sees the rounds the pre multiplier produced: 9 pellets, half again is
    /// 13.5, which is 14, and half again is 21. Rounding once at the end would
    /// give 20.
    #[test]
    fn whole_number_stats_are_rounded_between_the_stages() {
        let def = probe_def(StatId::Pellets, 9.0, &[PROBE_SLOT]);
        let b = build(&[(PROBE_SLOT.id, PROBE_PELLETS.id, 0.0)]);
        let got = resolve(&def, &b).stat(StatId::Pellets);
        assert!((got - 21.0).abs() < 1e-6, "{got} pellets");
    }

    /// A roll belongs to the build entry, not to the resolver: two copies of one
    /// part differ only because their entries say so, and the same entry always
    /// resolves to the same number.
    #[test]
    fn a_range_is_rolled_by_the_caller_and_resolve_stays_pure() {
        let rifle = def(WeaponKind::Rifle);
        let at = |roll: f32| {
            let b = build(&[(SlotId("magazine"), SURPLUS_LOT.id, roll)]);
            resolve(rifle, &b).stat(StatId::Damage)
        };
        let (lo, mid, hi) = (at(0.0), at(0.5), at(1.0));
        assert!(
            lo < mid && mid < hi,
            "the range did not interpolate: {lo} {mid} {hi}"
        );
        assert_eq!(
            mid,
            at(0.5),
            "resolve stopped being a function of its inputs"
        );
        assert_eq!(hi, at(9.0), "a roll outside [0, 1] was not clamped");

        // A roll from the game's own generator is still just a number handed in:
        // nothing in the resolver draws one.
        let mut rng = crate::util::Rng::new(7);
        let roll = rng.f32();
        assert_eq!(at(roll), at(roll), "the resolver rolled its own dice");
    }

    /// Resolution reads the def and leaves it alone. A build's outcome written
    /// back into a def is how one weapon's scope becomes every weapon's scope.
    #[test]
    fn resolving_a_build_leaves_the_def_untouched() {
        let rifle = def(WeaponKind::Rifle);
        let issued = resolve(rifle, &WeaponBuild::empty()).as_stats();
        let base_before = rifle.base;
        let built = resolve(
            rifle,
            &build(&[
                (SlotId("optic"), MICRO_DOT.id, 1.0),
                (SlotId("muzzle"), COMPENSATOR.id, 1.0),
                (SlotId("magazine"), SURPLUS_LOT.id, 1.0),
            ]),
        );
        assert_ne!(built.as_stats(), issued, "the build changed nothing");
        assert_eq!(rifle.base, base_before, "resolve wrote back into the def");
        assert_eq!(
            resolve(rifle, &WeaponBuild::empty()).as_stats(),
            issued,
            "the issued numbers changed after a build was resolved",
        );
    }

    /// The live path, end to end: a real weapon, a real part, and only the stats
    /// that part names are allowed to move.
    #[test]
    fn an_installed_part_moves_only_the_stats_it_names() {
        let rifle = def(WeaponKind::Rifle);
        let issued = resolve(rifle, &WeaponBuild::empty());
        let tuned = resolve(rifle, &build(&[(SlotId("muzzle"), COMPENSATOR.id, 0.0)]));
        for stat in StatId::ALL {
            let moved = tuned.stat(stat) != issued.stat(stat);
            let meant = matches!(stat, StatId::RecoilPitch | StatId::RecoilYaw);
            assert_eq!(moved, meant, "{stat:?} moved: {moved}");
        }
        assert!(tuned.stat(StatId::RecoilPitch) < issued.stat(StatId::RecoilPitch));
    }

    /// A flat bonus on a whole-number stat passes the rounding stage untouched:
    /// 30 rounds plus 10 is 40, and the reload penalty comes with it.
    #[test]
    fn a_part_adds_to_whole_number_stats() {
        let rifle = def(WeaponKind::Rifle);
        let tuned = resolve(rifle, &build(&[(SlotId("magazine"), EXTENDED_MAG.id, 0.0)]));
        assert!((tuned.stat(StatId::MagSize) - 40.0).abs() < 1e-6);
        assert!((tuned.stat(StatId::ReloadTime) - 1.50).abs() < 1e-6);
    }

    /// A slot a part opens is a slot like any other — and the build may list it
    /// before the part that opened it, because a build is a set of choices, not a
    /// sequence of assembly steps.
    #[test]
    fn a_part_can_open_a_slot_for_another_part_in_any_order() {
        let rifle = def(WeaponKind::Rifle);
        let stacked = build(&[
            (SlotId("riser_top"), SCOPE.id, 0.0),
            (SlotId("optic"), RISER.id, 0.0),
        ]);
        assert_eq!(validate(rifle, &stacked), Ok(()), "the slot stayed shut");
        let issued = resolve(rifle, &WeaponBuild::empty()).stat(StatId::AdsFov);
        assert!(resolve(rifle, &stacked).stat(StatId::AdsFov) < issued);

        // Without the part that opens it, the slot does not exist.
        let orphan = build(&[(SlotId("riser_top"), SCOPE.id, 0.0)]);
        assert_eq!(
            validate(rifle, &orphan),
            Err(vec![Violation::UnknownSlot(SlotId("riser_top"))]),
            "a slot existed before anything opened it",
        );
    }

    /// An instance resolves its own build and keeps the numbers: the cache is
    /// per weapon, and the magazine follows the resolved size, not the def's.
    #[test]
    fn a_weapon_with_a_build_uses_its_own_numbers() {
        let built = Weapon::with_build(
            WeaponKind::Rifle,
            build(&[(SlotId("magazine"), EXTENDED_MAG.id, 0.0)]),
        );
        let issued = Weapon::new(WeaponKind::Rifle);
        assert_eq!(built.build().entries.len(), 1);
        assert_eq!(built.mag, 40, "the magazine did not follow the build");
        assert_eq!(issued.mag, 30);
        assert!(built.stats().mag_size > issued.stats().mag_size);
        assert_eq!(built.stats().recoil_pitch, issued.stats().recoil_pitch);
    }

    /// The two paths to a weapon's numbers — the kind-level resolve and the one
    /// `Weapon::new` took — agree, which is what lets the rest of the game keep
    /// reading `Weapon::stats` without knowing this model exists.
    #[test]
    fn an_empty_build_resolves_to_the_weapon_stats_table() {
        for kind in ALL_KINDS {
            let resolved = resolve(def(kind), &WeaponBuild::empty()).as_stats();
            assert_eq!(resolved, Weapon::new(kind).stats(), "{kind:?}");
        }
    }

    // ---- validation --------------------------------------------------------

    #[test]
    fn validate_reports_a_slot_the_weapon_does_not_offer() {
        let knife = def(WeaponKind::Knife);
        let bad = validate(knife, &build(&[(SlotId("optic"), MICRO_DOT.id, 0.0)]))
            .expect_err("a knife accepted an optic");
        assert_eq!(bad, vec![Violation::UnknownSlot(SlotId("optic"))]);
    }

    #[test]
    fn validate_reports_a_part_that_does_not_fit_the_standard() {
        let rifle = def(WeaponKind::Rifle);
        // A dot sight on the muzzle: built for another slot *and* speaking a
        // different mounting standard. Both faults are reported, not just the
        // first one the check happened to run into.
        let bad = validate(rifle, &build(&[(SlotId("muzzle"), MICRO_DOT.id, 0.0)]))
            .expect_err("a sight fitted a muzzle");
        assert!(
            bad.contains(&Violation::WrongSlot {
                slot: SlotId("muzzle"),
                part: MICRO_DOT.id,
                built_for: SlotId("optic"),
            }),
            "{bad:?}"
        );
        assert!(
            bad.contains(&Violation::WrongInterface {
                slot: SlotId("muzzle"),
                part: MICRO_DOT.id,
                accepts: InterfaceId("threaded_muzzle"),
                declares: InterfaceId("rail"),
            }),
            "{bad:?}"
        );
        assert_eq!(bad.len(), 2, "one fault reported twice: {bad:?}");
    }

    #[test]
    fn validate_reports_more_parts_than_the_slot_holds() {
        let rifle = def(WeaponKind::Rifle);
        let stacked = build(&[
            (SlotId("optic"), MICRO_DOT.id, 0.0),
            (SlotId("optic"), RISER.id, 0.0),
        ]);
        let bad = validate(rifle, &stacked).expect_err("two parts in a one-part slot");
        assert_eq!(
            bad,
            vec![Violation::Overfull {
                slot: SlotId("optic"),
                max: 1,
                found: 2,
            }]
        );
    }

    #[test]
    fn validate_reports_a_part_the_catalogue_never_heard_of() {
        let rifle = def(WeaponKind::Rifle);
        let bad = validate(
            rifle,
            &build(&[(SlotId("optic"), PartId("no_such_part"), 0.0)]),
        )
        .expect_err("a part materialised out of nothing");
        assert_eq!(bad, vec![Violation::UnknownPart(PartId("no_such_part"))]);
    }

    /// The table must not move. The expectations are the numbers the old
    /// `WeaponKind::stats` match held, copied longhand on purpose: a test that
    /// read them back out of the defs would agree with any change to them, and
    /// this is the only thing standing between a refactor and a silent rebalance.
    #[test]
    fn the_shipped_tables_survive_the_resolver_bit_for_bit() {
        for kind in ALL_KINDS {
            let got = resolve(def(kind), &WeaponBuild::empty()).as_stats();
            let want = table_before_the_resolver(kind);
            assert_eq!(got.class, want.class, "{kind:?} class");
            assert_eq!(got.fire_modes, want.fire_modes, "{kind:?} fire modes");
            assert_eq!(got.infinite_ammo, want.infinite_ammo, "{kind:?} ammunition");
            assert_eq!(got.melee, want.melee, "{kind:?} melee");
            for stat in StatId::ALL {
                assert_eq!(field(got, stat), field(want, stat), "{kind:?} {stat:?}");
            }
        }
    }

    /// `WeaponKind::stats` as it stood before the def table existed, including
    /// the shared `base` it used to build each weapon from.
    fn table_before_the_resolver(kind: WeaponKind) -> WeaponStats {
        let base = |class, damage, pellets, fire_interval, mag_size, reload_time| WeaponStats {
            class,
            damage,
            pellets,
            fire_interval,
            mag_size,
            reload_time,
            base_spread: 0.0,
            bloom_per_shot: 0.0,
            max_bloom: 0.0,
            bloom_decay: 0.1,
            recoil_pitch: 0.0,
            recoil_yaw: 0.0,
            ads_fov: 45.0,
            move_scale: 1.0,
            headshot_mult: 2.0,
            falloff_start: 1000.0,
            falloff_end: 1000.0,
            falloff_min: 1.0,
            fire_modes: SEMI,
            burst_count: 3,
            infinite_ammo: false,
            melee: false,
        };
        match kind {
            WeaponKind::Rifle => WeaponStats {
                base_spread: 0.0042,
                bloom_per_shot: 0.0055,
                max_bloom: 0.040,
                bloom_decay: 0.085,
                recoil_pitch: 0.0165,
                recoil_yaw: 0.0042,
                ads_fov: 42.0,
                headshot_mult: 2.4,
                falloff_start: 30.0,
                falloff_end: 70.0,
                falloff_min: 0.55,
                fire_modes: RIFLE_MODES,
                ..base(SlotClass::Primary, 24.0, 1, 0.088, 30, 1.35)
            },
            WeaponKind::Shotgun => WeaponStats {
                base_spread: 0.055,
                recoil_pitch: 0.055,
                recoil_yaw: 0.012,
                ads_fov: 50.0,
                move_scale: 0.94,
                headshot_mult: 1.6,
                falloff_start: 8.0,
                falloff_end: 26.0,
                falloff_min: 0.25,
                ..base(SlotClass::Primary, 15.0, 9, 0.78, 6, 2.30)
            },
            WeaponKind::Smg => WeaponStats {
                base_spread: 0.0105,
                bloom_per_shot: 0.0045,
                max_bloom: 0.045,
                bloom_decay: 0.110,
                recoil_pitch: 0.0105,
                recoil_yaw: 0.0055,
                ads_fov: 46.0,
                move_scale: 1.05,
                headshot_mult: 2.0,
                falloff_start: 18.0,
                falloff_end: 45.0,
                falloff_min: 0.50,
                fire_modes: SMG_MODES,
                ..base(SlotClass::Primary, 14.0, 1, 0.055, 40, 1.50)
            },
            WeaponKind::Sniper => WeaponStats {
                base_spread: 0.0009,
                bloom_per_shot: 0.020,
                max_bloom: 0.030,
                bloom_decay: 0.500,
                recoil_pitch: 0.075,
                recoil_yaw: 0.010,
                ads_fov: 16.0,
                move_scale: 0.85,
                headshot_mult: 2.2,
                ..base(SlotClass::Primary, 115.0, 1, 1.15, 5, 2.50)
            },
            WeaponKind::Lmg => WeaponStats {
                base_spread: 0.0135,
                bloom_per_shot: 0.0075,
                max_bloom: 0.075,
                bloom_decay: 0.050,
                recoil_pitch: 0.0135,
                recoil_yaw: 0.0060,
                ads_fov: 48.0,
                move_scale: 0.82,
                falloff_start: 35.0,
                falloff_end: 80.0,
                falloff_min: 0.60,
                fire_modes: AUTO_SEMI,
                ..base(SlotClass::Primary, 21.0, 1, 0.075, 100, 3.40)
            },
            WeaponKind::Pistol => WeaponStats {
                base_spread: 0.0060,
                bloom_per_shot: 0.0080,
                max_bloom: 0.038,
                bloom_decay: 0.160,
                recoil_pitch: 0.0220,
                recoil_yaw: 0.0030,
                ads_fov: 48.0,
                move_scale: 1.10,
                headshot_mult: 2.6,
                falloff_start: 14.0,
                falloff_end: 45.0,
                falloff_min: 0.45,
                ..base(SlotClass::Pistol, 26.0, 1, 0.160, 15, 1.25)
            },
            WeaponKind::Knife => WeaponStats {
                base_spread: 0.0,
                recoil_pitch: 0.030,
                ads_fov: 55.0,
                move_scale: 1.18,
                headshot_mult: 1.0,
                fire_modes: SEMI,
                infinite_ammo: true,
                melee: true,
                ..base(SlotClass::Melee, 55.0, 1, 0.55, 0, 0.0)
            },
        }
    }

    /// The bug that shipped: a latch only the firing path could clear, while the
    /// firing path only runs while the button is held — so every semi-automatic
    /// weapon fired exactly once and then never again. Merely holding the
    /// trigger must not be enough; releasing and pressing must be.
    #[test]
    fn semi_auto_fires_once_per_press() {
        for kind in [WeaponKind::Pistol, WeaponKind::Shotgun, WeaponKind::Sniper] {
            let mut w = Weapon::new(kind);
            assert!(w.mode() == FireMode::Single, "{kind:?} is not single-shot");
            let shots = run(&mut w, 600);
            assert_eq!(shots, 1, "{kind:?} fired {shots} times while merely held");
            for _ in 0..10 {
                w.update(FRAME);
            }
            w.update(FRAME);
            assert!(
                w.try_fire(true, true),
                "{kind:?} would not fire a second time"
            );
        }
    }

    #[test]
    fn auto_fires_at_its_rate_and_not_faster() {
        let mut w = Weapon::new(WeaponKind::Rifle);
        let interval = w.stats().fire_interval;
        let seconds = 1.0;
        let shots = run(&mut w, (seconds / FRAME) as usize);
        let ceiling = (seconds / interval).ceil() as usize + 1;
        assert!(shots <= ceiling, "rifle fired {shots} times in a second");
        assert!(shots >= 8, "rifle only fired {shots} times in a second");
    }

    /// A burst completes itself once begun, and does not run on afterwards.
    #[test]
    fn burst_fires_exactly_its_count() {
        let mut w = Weapon::new(WeaponKind::Rifle);
        while w.mode() != FireMode::Burst {
            w.cycle_fire_mode();
        }
        let n = w.stats().burst_count;
        let shots = run(&mut w, 120);
        assert_eq!(shots, n as usize, "a burst fired {shots} rounds, not {n}");
    }

    #[test]
    fn an_empty_magazine_does_not_fire() {
        let mut w = Weapon::new(WeaponKind::Pistol);
        w.mag = 0;
        assert!(!w.try_fire(true, true), "fired with an empty magazine");
        let mut k = Weapon::new(WeaponKind::Knife);
        for _ in 0..50 {
            k.update(FRAME);
            k.try_fire(true, true);
        }
        assert_eq!(k.mag, 0, "the knife grew ammunition");
        assert!(k.stats().infinite_ammo);
    }

    /// Distance falloff must be monotone, never exceed the base damage, and
    /// never drop below the declared floor.
    #[test]
    fn damage_falloff_is_monotone_and_bounded() {
        for kind in ALL_KINDS {
            let w = Weapon::new(kind);
            let s = w.stats();
            let mut prev = f32::MAX;
            for i in 0..=200 {
                let d = i as f32;
                let dmg = w.damage_at(d);
                assert!(
                    dmg <= s.damage + 1e-3,
                    "{kind:?} hit for {dmg} > base {}",
                    s.damage
                );
                assert!(
                    dmg >= s.damage * s.falloff_min - 1e-3,
                    "{kind:?} fell below its floor at {d}m: {dmg}"
                );
                assert!(dmg <= prev + 1e-3, "{kind:?} damage rose with distance");
                prev = dmg;
            }
        }
    }

    /// The four-slot design: a sidearm and a knife are always carried, whatever
    /// the two long slots hold — an arsenal that carries nothing still leaves
    /// the player a weapon.
    #[test]
    fn the_sidearm_and_knife_are_always_present() {
        for loadout in [
            [Some(WeaponKind::Rifle), None],
            [Some(WeaponKind::Smg), Some(WeaponKind::Shotgun)],
            [None, None],
        ] {
            let ars = Arsenal::new(loadout);
            assert_eq!(
                ars.slot(SlotClass::Pistol as usize).map(|w| w.kind),
                Some(WeaponKind::Pistol),
                "arsenal without a sidearm"
            );
            assert_eq!(
                ars.slot(SlotClass::Melee as usize).map(|w| w.kind),
                Some(WeaponKind::Knife),
                "arsenal without a knife"
            );
        }
    }

    /// A reload is a thing done with the weapon in hand: putting it away
    /// cancels it, and nothing finishes in the holster. Holstered reloads used
    /// to keep ticking — knife out, rifle back full, no reload ever seen.
    #[test]
    fn putting_a_weapon_away_cancels_its_reload() {
        let mut ars = Arsenal::new([Some(WeaponKind::Rifle), None]);
        let rifle = SlotClass::Primary as usize;
        ars.active_weapon_mut().mag = 12;
        ars.active_weapon_mut().reserve = 30;
        assert!(
            ars.active_weapon_mut().start_reload(),
            "the reload did not start"
        );

        // To the knife, and time passes with the rifle out of hand.
        ars.select(SlotClass::Melee as usize);
        ars.update(10.0);

        assert_eq!(
            ars.slot(rifle).expect("rifle").mag,
            12,
            "the magazine refilled behind the player's back"
        );
        assert!(
            !ars.slot(rifle).expect("rifle").is_reloading(),
            "the reload survived the weapon being put away"
        );

        // Drawing it again changes nothing, and a reload asked for again works.
        ars.select(rifle);
        assert!(!ars.active_weapon().is_reloading());
        assert!(ars.active_weapon_mut().start_reload());
        ars.update(2.0);
        assert_eq!(ars.active_weapon().mag, 30);
    }

    /// The reserve is finite and a reload draws from it: a short reserve loads
    /// a short magazine, an empty one refuses to load at all, and a pickup
    /// fills the magazine before it tops the reserve up.
    #[test]
    fn a_reload_draws_from_the_reserve() {
        let mut w = Weapon::new(WeaponKind::Rifle);
        assert_eq!(w.reserve, 0, "a weapon out of the crate carries no spares");
        w.mag = 5;
        assert!(!w.start_reload(), "an empty reserve loaded anyway");

        w.reserve = 7;
        assert!(w.start_reload());
        w.update(2.0);
        assert_eq!(w.mag, 12, "the magazine took more than the reserve had");
        assert_eq!(w.reserve, 0, "the reserve kept what it gave");

        // A full magazine is not reloaded, reserve or not.
        w.mag = w.stats().mag_size;
        w.reserve = 30;
        assert!(!w.start_reload());

        // Pickups: the magazine first, the rest into the reserve.
        w.mag = 0;
        assert_eq!(w.add_ammo(40), 40);
        assert_eq!(w.mag, 30);
        assert_eq!(w.reserve, 40);
        // A full magazine wastes nothing.
        assert_eq!(w.add_ammo(10), 10);
        assert_eq!(w.reserve, 50);
    }

    /// The catalogue the game ships is a set of choices, and every part seats
    /// somewhere: ids unique, each part built for a slot some gun offers, each
    /// standard matching the slot it names — and a fully-kitted rifle legal.
    #[test]
    fn the_shipped_catalogue_is_sound() {
        let mut seen = Vec::new();
        for p in SHIPPED {
            assert!(!seen.contains(&p.id.0), "duplicate part id {}", p.id.0);
            seen.push(p.id.0);
            let slot = GUN_SLOTS
                .iter()
                .find(|s| s.id == p.slot)
                .unwrap_or_else(|| panic!("{} is built for an unknown slot", p.id.0));
            assert_eq!(
                p.interface, slot.accepts,
                "{} declares the wrong standard for {}",
                p.id.0, p.slot.0
            );
        }
        let kit = WeaponBuild {
            entries: GUN_SLOTS
                .iter()
                .map(|slot| BuildEntry {
                    slot: slot.id,
                    part: SHIPPED
                        .iter()
                        .find(|p| p.slot == slot.id)
                        .expect("a slot with nothing to go in it")
                        .id,
                    roll: 0.4,
                })
                .collect(),
        };
        assert_eq!(
            validate(def(WeaponKind::Rifle), &kit),
            Ok(()),
            "the fully-kitted rifle is illegal"
        );
    }

    /// A build survives the settings file: written, read back, the same parts
    /// at the same rolls — and junk costs its own entry, never the line.
    #[test]
    fn a_build_round_trips_through_text() {
        let built = build(&[
            (SlotId("optic"), MICRO_DOT.id, 0.25),
            (SlotId("magazine"), SURPLUS_LOT.id, 0.75),
        ]);
        assert_eq!(WeaponBuild::from_text(&built.to_text()), built);
        // Reading is faithful: a hand-edited roll is believed as typed.
        assert_eq!(
            WeaponBuild::from_text("optic=micro_dot@0.666"),
            build(&[(SlotId("optic"), MICRO_DOT.id, 0.666)])
        );
        // Writing rounds to two decimals, so a build beyond them reads back
        // rounded rather than refused.
        let many = build(&[(SlotId("optic"), MICRO_DOT.id, 2.0 / 3.0)]);
        assert_eq!(
            WeaponBuild::from_text(&many.to_text()).entries[0].roll,
            0.67
        );
        let rough = WeaponBuild::from_text("optix=?, optic=micro_dot@0.5, ,muzzle=who@0.2");
        assert_eq!(rough.entries.len(), 1);
        assert_eq!(rough.entries[0].part, MICRO_DOT.id);
        assert_eq!(WeaponBuild::from_text(""), WeaponBuild::empty());
    }

    /// Screening is the repair path for a hand-edited file: entries that do
    /// not fit their def drop out one at a time, and the rest stay.
    #[test]
    fn screening_keeps_the_parts_that_fit() {
        let rifle = def(WeaponKind::Rifle);
        let rough = build(&[
            (SlotId("optic"), MICRO_DOT.id, 0.5),
            (SlotId("muzzle"), MICRO_DOT.id, 0.5),
            (SlotId("magazine"), EXTENDED_MAG.id, 0.5),
        ]);
        let clean = rough.screened(rifle);
        assert_eq!(clean.entries.len(), 2);
        assert_eq!(clean.entries[0].part, MICRO_DOT.id);
        assert_eq!(clean.entries[1].part, EXTENDED_MAG.id);
        assert_eq!(validate(rifle, &clean), Ok(()));
    }

    /// Every shipping part and slot has a name for the bench, in both
    /// languages; a label missing is a row that cannot be drawn.
    #[test]
    fn every_shipped_part_and_slot_has_a_label() {
        for p in SHIPPED {
            let (en, zh) = part_label(p.id).unwrap_or_else(|| panic!("{} has no name", p.id.0));
            assert!(
                !en.is_empty() && !zh.is_empty(),
                "{} has half a name",
                p.id.0
            );
        }
        for slot in GUN_SLOTS {
            assert!(slot_label(slot.id).is_some(), "{} has no name", slot.id.0);
        }
    }
}
