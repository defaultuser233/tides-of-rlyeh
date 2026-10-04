use crate::i18n::Key;
use crate::util::approach;
use macroquad::prelude::*;

pub const MAX_RANGE: f32 = 160.0;

/// How long the inspect motion runs, in seconds. `start_inspect` arms the timer
/// and `viewmodel` draws it; the two read the same number so a change to the
/// choreography cannot leave the drawing running past its own clock.
pub const INSPECT_TIME: f32 = 2.20;

/// The same, for the shorter motion a magazine check plays. Armed where the
/// check is asked for, not by the weapon, because it is a reading of the weapon
/// rather than something the weapon does.
pub const MAG_CHECK_TIME: f32 = 1.10;

/// How much faster the inspect runs when something has interrupted it, as a
/// multiple of its own rate. Enough that the arm is back at rest inside a
/// tenth of a second — a trigger pull has to feel immediate — and not so much
/// that it is the hard cut it replaced.
const INSPECT_REWIND: f32 = 9.0;

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
    /// 5.56 carbine: the rifle's round at a carbine's rate.
    M4a1,
    /// 5.56 rifle: slower, harder-hitting and steadier than the MK-7.
    Scar,
    /// 7.62x51 battle rifle: hits like a truck and kicks like one.
    Fal,
    /// 9 mm machine pistol with the accuracy the Viper trades away.
    Mp5,
    /// 9 mm machine pistol built for rate rather than control.
    Mpx,
    /// 7.62x51 semi-automatic marksman's rifle — the middle of the range.
    M110,
    /// 7.62x54R bolt-action: one shot, one kill, and a long wait for the next.
    Mosin,
}

/// Every kind the game has, in the order `WeaponKind::index` returns.
///
/// The length of this list *is* the arity of every per-kind table in the game:
/// `KIND_COUNT` is what those tables are sized by, so adding a weapon is an entry
/// here and its data, not a hunt for the number seven in four files.
pub const ALL_KINDS: [WeaponKind; 14] = [
    WeaponKind::Rifle,
    WeaponKind::Shotgun,
    WeaponKind::Smg,
    WeaponKind::Sniper,
    WeaponKind::Lmg,
    WeaponKind::Pistol,
    WeaponKind::Knife,
    WeaponKind::M4a1,
    WeaponKind::Scar,
    WeaponKind::Fal,
    WeaponKind::Mp5,
    WeaponKind::Mpx,
    WeaponKind::M110,
    WeaponKind::Mosin,
];

/// How many weapon kinds there are. See `ALL_KINDS`.
pub const KIND_COUNT: usize = ALL_KINDS.len();

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
/// `Static` is what every shipping part uses. `Range` is the source of "same
/// part, different feel": whoever builds the instance draws a roll and hands it
/// in, so nothing here needs to know about `util::Rng`, and `resolve` stays a
/// function that a test, a tutorial hint and a network check can all replay.
///
/// Nothing in the shipping catalogue rolls any more — the magazine probes moved
/// beside the tests, and a part whose capacity is rolled is a part a player
/// cannot plan around — so the not-test build is told this variant is
/// deliberately unused rather than being told to delete a capability the model
/// is built to carry.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Value {
    Static(f32),
    /// A number that differs between two copies of one part, sampled by the
    /// build entry's roll.
    #[cfg_attr(not(test), allow(dead_code))]
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
    /// Applied after the stat's envelope and rounding, which is what makes
    /// handling read as "after the design bounds, this is what practice buys".
    /// The shipping catalogue has no user of it since the quick magazine moved
    /// beside the tests; the resolver still offers the stage.
    #[cfg_attr(not(test), allow(dead_code))]
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
    CATALOG
        .iter()
        .find(|p| p.id == id)
        .or_else(|| probes().iter().find(|p| p.id == id))
}

/// The probe parts a build may name in a test, empty in a shipping build. This
/// is how `resolve` finds the parts `demo_parts` keeps.
fn probes() -> &'static [PartDef] {
    #[cfg(test)]
    {
        PROBES
    }
    #[cfg(not(test))]
    {
        &[]
    }
}

/// The static slot id a settings string names, or `None`. Ids are
/// `&'static str` by design, so text read back from a file is matched to the
/// table rather than carried as itself; the subslots a part can open are
/// deliberately not nameable this way.
fn slot_named(name: &str) -> Option<SlotId> {
    SLOT_IDS.iter().copied().find(|id| id.0 == name)
}

/// The static part id a settings string names, or `None`.
fn part_named(name: &str) -> Option<PartId> {
    CATALOG
        .iter()
        .chain(probes())
        .map(|p| p.id)
        .find(|id| id.0 == name)
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

    // A magazine's three trades, one line each, so the section below reads as a
    // table instead of as forty-odd struct literals: how many rounds it adds to
    // the magazine the weapon is issued with, how many seconds it adds to the
    // change, and how it carries — `MoveScale` below one is a heavier gun.
    const fn rounds(n: f32) -> StatMod {
        StatMod {
            stat: StatId::MagSize,
            kind: ModKind::Add,
            amount: Value::Static(n),
        }
    }
    const fn slower(secs: f32) -> StatMod {
        StatMod {
            stat: StatId::ReloadTime,
            kind: ModKind::Add,
            amount: Value::Static(secs),
        }
    }
    const fn weight(scale: f32) -> StatMod {
        StatMod {
            stat: StatId::MoveScale,
            kind: ModKind::MulPre,
            amount: Value::Static(scale),
        }
    }

    // --- magazines ---------------------------------------------------------
    //
    // Each names the well it fits, because a magazine *is* a fitting of the
    // round it feeds: the rifle's quad-stack does not go into a 9 mm well, and
    // a drum does not go into a shotgun's tube. Capacity is bought with weight
    // and with handling — every extra round is also an extra second in the
    // open, which is the trade the player is actually making.
    //
    // The set follows the real catalogue for each platform rather than a ladder
    // of round numbers. A 5.56 rifle is fed from 20-round straight magazines,
    // 30-round STANAGs, 40-round boxes, the 60-round quad-stack and the
    // hundred-round twin drum; a 9 mm submachine gun from 30- and 40-round
    // sticks and the two drums; a belt-fed gun from its box *or* a rifle
    // magazine dropped into the emergency well, which is a real thing the M249
    // does and the reason its well is shaped the way it is. Each one is
    // therefore a fitting somebody has actually carried.

    /// 5.56: forty rounds in a longer box.
    pub const MAG_40_556: PartDef = PartDef {
        id: PartId("mag_40_556"),
        slot: SlotId("magazine"),
        interface: InterfaceId("magwell_556"),
        subslots: &[],
        mods: &[rounds(10.0), slower(0.20)],
    };

    /// 5.56: the quad-stack — twice the rifle's magazine, and it shows.
    pub const MAG_60_556: PartDef = PartDef {
        id: PartId("mag_60_556"),
        slot: SlotId("magazine"),
        interface: InterfaceId("magwell_556"),
        subslots: &[],
        mods: &[rounds(30.0), slower(0.45), weight(0.96)],
    };

    /// 5.56: the hundred-round twin drum — two drums and a feed tower, which is
    /// what the real one is, and about as much weight as a rifle will take.
    pub const DRUM_100_556: PartDef = PartDef {
        id: PartId("drum_100_556"),
        slot: SlotId("magazine"),
        interface: InterfaceId("magwell_556"),
        subslots: &[],
        mods: &[rounds(70.0), slower(1.40), weight(0.88)],
    };

    /// 9 mm: forty rounds in the long stick.
    pub const MAG_40_9MM: PartDef = PartDef {
        id: PartId("mag_40_9mm"),
        slot: SlotId("magazine"),
        interface: InterfaceId("magwell_9mm"),
        subslots: &[],
        mods: &[rounds(10.0), slower(0.15)],
    };

    /// 9 mm: a fifty-round drum on a submachine gun that feeds thirty.
    pub const DRUM_50_9MM: PartDef = PartDef {
        id: PartId("drum_50_9mm"),
        slot: SlotId("magazine"),
        interface: InterfaceId("magwell_9mm"),
        subslots: &[],
        mods: &[rounds(20.0), slower(0.60), weight(0.95)],
    };

    /// 9 mm: the same twin drum scaled to the round it feeds.
    pub const DRUM_100_9MM: PartDef = PartDef {
        id: PartId("drum_100_9mm"),
        slot: SlotId("magazine"),
        interface: InterfaceId("magwell_9mm"),
        subslots: &[],
        mods: &[rounds(70.0), slower(1.30), weight(0.88)],
    };

    /// 7.62: a longer box for the marksman's rifle.
    pub const MAG_10_762: PartDef = PartDef {
        id: PartId("mag_10_762"),
        slot: SlotId("magazine"),
        interface: InterfaceId("magwell_762"),
        subslots: &[],
        mods: &[rounds(5.0), slower(0.30)],
    };

    /// 7.62: the twenty-round box the same rifle takes in its full-length form.
    pub const MAG_20_762: PartDef = PartDef {
        id: PartId("mag_20_762"),
        slot: SlotId("magazine"),
        interface: InterfaceId("magwell_762"),
        subslots: &[],
        mods: &[rounds(15.0), slower(0.60), weight(0.96)],
    };

    /// The belt box: a hundred more rounds, and a long time to change it.
    pub const BELT_200: PartDef = PartDef {
        id: PartId("belt_200"),
        slot: SlotId("magazine"),
        interface: InterfaceId("magwell_belt"),
        subslots: &[],
        mods: &[rounds(100.0), slower(1.20), weight(0.93)],
    };

    /// 9 mm pistol: the seventeen-round magazine the pistol is also sold with.
    pub const MAG_17_PISTOL: PartDef = PartDef {
        id: PartId("mag_17_pistol"),
        slot: SlotId("magazine"),
        interface: InterfaceId("magwell_pistol"),
        subslots: &[],
        mods: &[rounds(2.0), slower(0.06)],
    };

    /// 9 mm pistol: five more in the same grip length.
    pub const MAG_20_PISTOL: PartDef = PartDef {
        id: PartId("mag_20_pistol"),
        slot: SlotId("magazine"),
        interface: InterfaceId("magwell_pistol"),
        subslots: &[],
        mods: &[rounds(5.0), slower(0.15)],
    };

    /// 9 mm pistol: the thirty-round stick, which is a magazine that hangs out
    /// of the grip and is carried for exactly that reason.
    pub const MAG_30_PISTOL: PartDef = PartDef {
        id: PartId("mag_30_pistol"),
        slot: SlotId("magazine"),
        interface: InterfaceId("magwell_pistol"),
        subslots: &[],
        mods: &[rounds(15.0), slower(0.55), weight(0.97)],
    };

    /// The shotgun's tube: two more shells, and a slower feed.
    pub const TUBE_8: PartDef = PartDef {
        id: PartId("tube_8"),
        slot: SlotId("magazine"),
        interface: InterfaceId("magwell_tube"),
        subslots: &[],
        mods: &[rounds(2.0), slower(0.30)],
    };

    /// The shotgun's drum: fourteen more shells in a drum that hangs under the
    /// barrel, and the weight to go with them.
    pub const DRUM_20_SHELL: PartDef = PartDef {
        id: PartId("drum_20_shell"),
        slot: SlotId("magazine"),
        interface: InterfaceId("magwell_tube"),
        subslots: &[],
        mods: &[rounds(14.0), slower(1.60), weight(0.90)],
    };

    /// 7.62x51, FAL well: thirty rounds in the long aftermarket box. The FAL's
    /// own magazines are twenty and thirty; a twenty-five is an SR-25 magazine,
    /// which is why it is on the other well.
    pub const MAG_30_FAL: PartDef = PartDef {
        id: PartId("mag_30_fal"),
        slot: SlotId("magazine"),
        interface: InterfaceId("magwell_fal"),
        subslots: &[],
        mods: &[rounds(10.0), slower(0.55), weight(0.96)],
    };

    /// 7.62x51, FAL well: the ten-round magazine sold where a twenty is not
    /// allowed, and carried by hunters everywhere else. Lighter and quicker to
    /// change, and the first fitting in the game that takes capacity *away* —
    /// which the replace model is what made possible.
    pub const MAG_10_FAL: PartDef = PartDef {
        id: PartId("mag_10_fal"),
        slot: SlotId("magazine"),
        interface: InterfaceId("magwell_fal"),
        subslots: &[],
        mods: &[rounds(-10.0), slower(-0.45), weight(1.05)],
    };

    /// 7.62x51, SR-25 well: twenty-five rounds, which is the marksman's rifle's
    /// best-known magazine.
    pub const MAG_25_SR25: PartDef = PartDef {
        id: PartId("mag_25_sr25"),
        slot: SlotId("magazine"),
        interface: InterfaceId("magwell_sr25"),
        subslots: &[],
        mods: &[rounds(5.0), slower(0.35), weight(0.96)],
    };

    /// 7.62x51, SR-25 well: the ten-round box the same rifle is sold with.
    pub const MAG_10_SR25: PartDef = PartDef {
        id: PartId("mag_10_sr25"),
        slot: SlotId("magazine"),
        interface: InterfaceId("magwell_sr25"),
        subslots: &[],
        mods: &[rounds(-10.0), slower(-0.45), weight(1.05)],
    };

    /// 7.62x54R: a stripper clip. The bolt-action's box holds five either way —
    /// what the clip buys is a second's worth of loading, which on a weapon whose
    /// whole character is the wait for the next round is the only fitting worth
    /// carrying.
    pub const CLIP_54R: PartDef = PartDef {
        id: PartId("clip_54r"),
        slot: SlotId("magazine"),
        interface: InterfaceId("magwell_54r"),
        subslots: &[],
        mods: &[slower(-1.20)],
    };

    /// 9 mm, MP5 well: the fifteen-round magazine, which is the short one and —
    /// because the curve is stored per metre — the nearly straight one, which is
    /// what the real fifteen is. Half the rounds, a lighter gun, a quicker change.
    pub const MAG_15_MP5: PartDef = PartDef {
        id: PartId("mag_15_mp5"),
        slot: SlotId("magazine"),
        interface: InterfaceId("magwell_mp5"),
        subslots: &[],
        mods: &[rounds(-15.0), slower(-0.45), weight(1.06)],
    };

    /// 9 mm, MP5 well: the forty-round long magazine.
    pub const MAG_40_MP5: PartDef = PartDef {
        id: PartId("mag_40_mp5"),
        slot: SlotId("magazine"),
        interface: InterfaceId("magwell_mp5"),
        subslots: &[],
        mods: &[rounds(10.0), slower(0.20)],
    };

    /// 9 mm, MP5 well: the fifty-round drum. Its own well's drum, not the
    /// generic 9 mm one's, because an MP5 drum is an MP5 drum.
    pub const DRUM_50_MP5: PartDef = PartDef {
        id: PartId("drum_50_mp5"),
        slot: SlotId("magazine"),
        interface: InterfaceId("magwell_mp5"),
        subslots: &[],
        mods: &[rounds(20.0), slower(0.60), weight(0.95)],
    };

    /// 9 mm, MPX well: the ten-round magazine, for the states and the countries
    /// that cap it.
    pub const MAG_10_MPX: PartDef = PartDef {
        id: PartId("mag_10_mpx"),
        slot: SlotId("magazine"),
        interface: InterfaceId("magwell_mpx"),
        subslots: &[],
        mods: &[rounds(-20.0), slower(-0.50), weight(1.07)],
    };

    /// 9 mm, MPX well: twenty rounds in the same tube.
    pub const MAG_20_MPX: PartDef = PartDef {
        id: PartId("mag_20_mpx"),
        slot: SlotId("magazine"),
        interface: InterfaceId("magwell_mpx"),
        subslots: &[],
        mods: &[rounds(-10.0), slower(-0.25), weight(1.03)],
    };

    /// 9 mm, MPX well: Magpul's thirty-five, which is the magazine an MPX is
    /// most often photographed with.
    pub const MAG_35_MPX: PartDef = PartDef {
        id: PartId("mag_35_mpx"),
        slot: SlotId("magazine"),
        interface: InterfaceId("magwell_mpx"),
        subslots: &[],
        mods: &[rounds(5.0), slower(0.18)],
    };

    /// The shipping catalogue, as one list: what the game resolves against
    /// before the tests add their own parts to the copy they build.
    pub const SHIPPED: &[PartDef] = &[
        MICRO_DOT,
        MARKSMAN_SCOPE,
        COMPENSATOR,
        FLASH_HIDER,
        HEAVY_BARREL,
        MAG_40_556,
        MAG_60_556,
        DRUM_100_556,
        MAG_40_9MM,
        DRUM_50_9MM,
        DRUM_100_9MM,
        MAG_10_762,
        MAG_20_762,
        BELT_200,
        MAG_15_MP5,
        MAG_40_MP5,
        DRUM_50_MP5,
        MAG_10_MPX,
        MAG_20_MPX,
        MAG_35_MPX,
        MAG_17_PISTOL,
        MAG_20_PISTOL,
        MAG_30_PISTOL,
        TUBE_8,
        DRUM_20_SHELL,
        MAG_30_FAL,
        MAG_10_FAL,
        MAG_25_SR25,
        MAG_10_SR25,
        CLIP_54R,
    ];
}

/// The catalogue: what the armory installs, and what a saved build may name.
///
/// One list, and the shipping one: the tests' own probes are searched beside it
/// (`part_def`) rather than duplicated into it, because the copy this replaces
/// had to be edited every time a shipping part was added, and forgetting was a
/// test failure that read as a mystery (`UnknownPart`) instead of a missing
/// line here.
static CATALOG: &[PartDef] = parts::SHIPPED;

/// The probe parts the model's own tests build with. Not in `CATALOG`: nothing
/// the game offers may name one of these, and `catalogue()` is what the armory
/// reads.
#[cfg(test)]
static PROBES: &[PartDef] = &[
    demo_parts::RISER,
    demo_parts::SCOPE,
    demo_parts::EXTENDED_MAG,
    demo_parts::QUICK_MAG,
    demo_parts::SURPLUS_LOT,
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
        "mag_40_556" => ("MAG 40 / 5.56", "40 发弹匣"),
        "mag_60_556" => ("QUAD-STACK 60", "60 发四排弹匣"),
        "drum_100_556" => ("C-MAG 100 / 5.56", "100 发双鼓"),
        "mag_40_9mm" => ("MAG 40 / 9MM", "40 发弹匣"),
        "drum_50_9mm" => ("DRUM 50 / 9MM", "50 发弹鼓"),
        "mag_15_mp5" => ("MAG 15 / MP5", "15 发弹匣"),
        "mag_40_mp5" => ("MAG 40 / MP5", "40 发长弹匣"),
        "drum_50_mp5" => ("DRUM 50 / MP5", "50 发弹鼓"),
        "mag_10_mpx" => ("MAG 10 / MPX", "10 发弹匣"),
        "mag_20_mpx" => ("MAG 20 / MPX", "20 发弹匣"),
        "mag_35_mpx" => ("MAG 35 / MPX", "35 发弹匣"),
        "drum_100_9mm" => ("C-MAG 100 / 9MM", "100 发双鼓"),
        "mag_10_762" => ("MAG 10 / 7.62", "10 发弹匣"),
        "mag_20_762" => ("MAG 20 / 7.62", "20 发弹匣"),
        "belt_200" => ("BELT BOX 200", "200 发弹链箱"),
        "mag_17_pistol" => ("MAG 17 / 9MM", "17 发弹匣"),
        "mag_20_pistol" => ("MAG 20 / 9MM", "20 发手枪弹匣"),
        "mag_30_pistol" => ("MAG 30 / 9MM", "30 发加长弹匣"),
        "tube_8" => ("TUBE +2", "加长弹管"),
        "drum_20_shell" => ("DRUM 20 / 12GA", "20 发霰弹鼓"),
        "mag_30_fal" => ("MAG 30 / FAL", "30 发弹匣"),
        "mag_10_fal" => ("MAG 10 / FAL", "10 发弹匣"),
        "mag_25_sr25" => ("MAG 25 / SR-25", "25 发弹匣"),
        "mag_10_sr25" => ("MAG 10 / SR-25", "10 发弹匣"),
        "clip_54r" => ("STRIPPER CLIP", "桥夹"),
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
/// The rail and the thread are shared — a scope or a brake fits whatever it is
/// fastened to. The **well is per weapon**, because a magazine is a fitting of
/// the round it feeds: a 50-round 9 mm drum does not go into a 5.56 mm well, and
/// a 60-round quad-stack does not go into a shotgun's tube. Splitting the one
/// table per weapon is what the comment here used to promise, and it is what
/// makes "the rifle's 60" and "the SMG's 50" the same kind of part.
const RAIL: SlotDef = SlotDef {
    id: SlotId("optic"),
    accepts: InterfaceId("rail"),
    max_parts: 1,
};

const MUZZLE: SlotDef = SlotDef {
    id: SlotId("muzzle"),
    accepts: InterfaceId("threaded_muzzle"),
    max_parts: 1,
};

/// A magazine well, named for what it takes.
const fn well(name: &'static str) -> SlotDef {
    SlotDef {
        id: SlotId("magazine"),
        accepts: InterfaceId(name),
        max_parts: 1,
    }
}

/// The slot ids a settings string may name. The wells differ per weapon; the
/// ids do not, which is why this is a list of names rather than of slots.
const SLOT_IDS: [SlotId; 3] = [SlotId("optic"), SlotId("muzzle"), SlotId("magazine")];

const RIFLE_SLOTS: &[SlotDef] = &[RAIL, MUZZLE, well("magwell_556")];
const SHOTGUN_SLOTS: &[SlotDef] = &[RAIL, MUZZLE, well("magwell_tube")];
const SMG_SLOTS: &[SlotDef] = &[RAIL, MUZZLE, well("magwell_9mm")];
/// The MP5's own well. A curved HK magazine does not fit an MPX and an MPX
/// magazine does not fit an MP5, both facts being about the catch rather than
/// about the round, so they are two wells and not one. The Viper keeps the
/// generic 9 mm well it has always had, and its part ids with it — a saved
/// build naming `drum_50_9mm` has to keep working.
const MP5_SLOTS: &[SlotDef] = &[RAIL, MUZZLE, well("magwell_mp5")];
/// And the MPX's: SIG's own magazine, which the aftermarket takes to 35.
const MPX_SLOTS: &[SlotDef] = &[RAIL, MUZZLE, well("magwell_mpx")];
const SNIPER_SLOTS: &[SlotDef] = &[RAIL, MUZZLE, well("magwell_762")];
const LMG_SLOTS: &[SlotDef] = &[RAIL, MUZZLE, well("magwell_belt")];
const PISTOL_SLOTS: &[SlotDef] = &[RAIL, MUZZLE, well("magwell_pistol")];
/// The battle rifle's well. Its own, and not the marksman's: a FAL magazine and
/// an SR-25 magazine are both 7.62x51 and neither fits the other's well, which
/// is why they are two wells and not one. (Putting both weapons on one well is
/// what put a twenty-five-round magazine — a real SR-25 magazine, and not a real
/// FAL one — on the FAL.)
const FAL_SLOTS: &[SlotDef] = &[RAIL, MUZZLE, well("magwell_fal")];

/// The marksman's rifle's well: the SR-25 pattern, whose magazines go to 25.
const M110_SLOTS: &[SlotDef] = &[RAIL, MUZZLE, well("magwell_sr25")];
/// 7.62x54R: the bolt-action, whose five rounds are loaded by clip.
const MOSIN_SLOTS: &[SlotDef] = &[RAIL, MUZZLE, well("magwell_54r")];

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
    slots: RIFLE_SLOTS,
};

// --- the 5.56 carbine and rifle ---------------------------------------------
//
// Three weapons on one round, separated the way the real ones are: the MK-7 is
// the issued middle, the M4A1 spends steadiness for rate, and the SCAR-L spends
// rate for reach and a flatter kick. They share the 5.56 well, so they share its
// magazines.

/// The carbine: 857 rounds a minute against the MK-7's 682, and each one weaker.
const M4A1: WeaponDef = WeaponDef {
    class: SlotClass::Primary,
    fire_modes: RIFLE_MODES,
    infinite_ammo: false,
    melee: false,
    base: [
        st(22.0),   // Damage
        st(1.0),    // Pellets
        st(0.070),  // FireInterval
        st(30.0),   // MagSize
        st(1.25),   // ReloadTime
        st(0.0044), // BaseSpread
        st(0.0052), // BloomPerShot
        st(0.042),  // MaxBloom
        st(0.090),  // BloomDecay
        st(0.0150), // RecoilPitch
        st(0.0040), // RecoilYaw
        st(42.0),   // AdsFov
        st(1.03),   // MoveScale
        st(2.4),    // HeadshotMult
        st(26.0),   // FalloffStart
        st(62.0),   // FalloffEnd
        st(0.52),   // FalloffMin
        st(3.0),    // BurstCount
    ],
    slots: RIFLE_SLOTS,
};

/// The rifle: half again the MK-7's punch for two thirds of its rate.
const SCAR: WeaponDef = WeaponDef {
    class: SlotClass::Primary,
    fire_modes: RIFLE_MODES,
    infinite_ammo: false,
    melee: false,
    base: [
        st(27.0),   // Damage
        st(1.0),    // Pellets
        st(0.100),  // FireInterval
        st(30.0),   // MagSize
        st(1.60),   // ReloadTime
        st(0.0038), // BaseSpread
        st(0.0060), // BloomPerShot
        st(0.038),  // MaxBloom
        st(0.080),  // BloomDecay
        st(0.0195), // RecoilPitch
        st(0.0046), // RecoilYaw
        st(40.0),   // AdsFov
        st(0.97),   // MoveScale
        st(2.4),    // HeadshotMult
        st(34.0),   // FalloffStart
        st(78.0),   // FalloffEnd
        st(0.58),   // FalloffMin
        st(3.0),    // BurstCount
    ],
    slots: RIFLE_SLOTS,
};

// --- the full-power rifles ---------------------------------------------------

/// The battle rifle. Forty-three a hit is nearly the marksman's round at a
/// rifleman's rate, and the recoil is what it costs.
const FAL: WeaponDef = WeaponDef {
    class: SlotClass::Primary,
    fire_modes: RIFLE_MODES,
    infinite_ammo: false,
    melee: false,
    base: [
        st(43.0),   // Damage
        st(1.0),    // Pellets
        st(0.090),  // FireInterval
        st(20.0),   // MagSize
        st(2.20),   // ReloadTime
        st(0.0050), // BaseSpread
        st(0.0075), // BloomPerShot
        st(0.050),  // MaxBloom
        st(0.075),  // BloomDecay
        st(0.0270), // RecoilPitch
        st(0.0070), // RecoilYaw
        st(44.0),   // AdsFov
        st(0.93),   // MoveScale
        st(2.3),    // HeadshotMult
        st(40.0),   // FalloffStart
        st(90.0),   // FalloffEnd
        st(0.62),   // FalloffMin
        st(3.0),    // BurstCount
    ],
    slots: FAL_SLOTS,
};

/// The marksman's rifle: one shot per trigger pull, and the next one a beat
/// later. The middle of the range the campaign had no weapon in.
const M110: WeaponDef = WeaponDef {
    class: SlotClass::Primary,
    fire_modes: SEMI,
    infinite_ammo: false,
    melee: false,
    base: [
        st(55.0),   // Damage
        st(1.0),    // Pellets
        st(0.290),  // FireInterval
        st(20.0),   // MagSize
        st(2.10),   // ReloadTime
        st(0.0016), // BaseSpread
        st(0.0100), // BloomPerShot
        st(0.045),  // MaxBloom
        st(0.070),  // BloomDecay
        st(0.0300), // RecoilPitch
        st(0.0060), // RecoilYaw
        st(26.0),   // AdsFov
        st(0.95),   // MoveScale
        st(2.8),    // HeadshotMult
        st(60.0),   // FalloffStart
        st(140.0),  // FalloffEnd
        st(0.75),   // FalloffMin
        st(3.0),    // BurstCount
    ],
    slots: M110_SLOTS,
};

/// The bolt-action. A hundred and forty a hit is the highest number in the game
/// and the only one that puts a Juggernaut down outright; forty-three rounds a
/// minute is the price, and the wait is the weapon.
const MOSIN: WeaponDef = WeaponDef {
    class: SlotClass::Primary,
    fire_modes: SEMI,
    infinite_ammo: false,
    melee: false,
    base: [
        st(140.0),  // Damage
        st(1.0),    // Pellets
        st(1.40),   // FireInterval
        st(5.0),    // MagSize
        st(3.10),   // ReloadTime
        st(0.0012), // BaseSpread
        st(0.0200), // BloomPerShot
        st(0.060),  // MaxBloom
        st(0.060),  // BloomDecay
        st(0.0450), // RecoilPitch
        st(0.0090), // RecoilYaw
        st(22.0),   // AdsFov
        st(0.90),   // MoveScale
        st(3.0),    // HeadshotMult
        st(80.0),   // FalloffStart
        st(200.0),  // FalloffEnd
        st(0.85),   // FalloffMin
        st(3.0),    // BurstCount
    ],
    slots: MOSIN_SLOTS,
};

// --- the 9 mm machine pistols -------------------------------------------------

/// The accurate one: seven hundred and seventy a minute with a spread the Viper
/// does not have.
const MP5: WeaponDef = WeaponDef {
    class: SlotClass::Primary,
    fire_modes: SMG_MODES,
    infinite_ammo: false,
    melee: false,
    base: [
        st(16.0),   // Damage
        st(1.0),    // Pellets
        st(0.078),  // FireInterval
        st(30.0),   // MagSize
        st(1.40),   // ReloadTime
        st(0.0072), // BaseSpread
        st(0.0038), // BloomPerShot
        st(0.040),  // MaxBloom
        st(0.095),  // BloomDecay
        st(0.0110), // RecoilPitch
        st(0.0030), // RecoilYaw
        st(46.0),   // AdsFov
        st(1.04),   // MoveScale
        st(2.0),    // HeadshotMult
        st(18.0),   // FalloffStart
        st(46.0),   // FalloffEnd
        st(0.45),   // FalloffMin
        st(3.0),    // BurstCount
    ],
    slots: MP5_SLOTS,
};

/// The fast one: nine hundred and twenty-three a minute, and the bloom that goes
/// with it.
const MPX: WeaponDef = WeaponDef {
    class: SlotClass::Primary,
    fire_modes: SMG_MODES,
    infinite_ammo: false,
    melee: false,
    base: [
        st(15.0),   // Damage
        st(1.0),    // Pellets
        st(0.065),  // FireInterval
        st(30.0),   // MagSize
        st(1.35),   // ReloadTime
        st(0.0080), // BaseSpread
        st(0.0050), // BloomPerShot
        st(0.048),  // MaxBloom
        st(0.090),  // BloomDecay
        st(0.0125), // RecoilPitch
        st(0.0036), // RecoilYaw
        st(44.0),   // AdsFov
        st(1.05),   // MoveScale
        st(2.0),    // HeadshotMult
        st(16.0),   // FalloffStart
        st(42.0),   // FalloffEnd
        st(0.42),   // FalloffMin
        st(3.0),    // BurstCount
    ],
    slots: MPX_SLOTS,
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
    slots: SHOTGUN_SLOTS,
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
        st(30.0),   // MagSize — thirty in the gun, fifty in the drum
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
    slots: SMG_SLOTS,
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
    slots: SNIPER_SLOTS,
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
    slots: LMG_SLOTS,
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
    slots: PISTOL_SLOTS,
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
        WeaponKind::M4a1 => &M4A1,
        WeaponKind::Scar => &SCAR,
        WeaponKind::Fal => &FAL,
        WeaponKind::Mp5 => &MP5,
        WeaponKind::Mpx => &MPX,
        WeaponKind::M110 => &M110,
        WeaponKind::Mosin => &MOSIN,
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
            WeaponKind::M4a1 => Key::WeaponM4a1,
            WeaponKind::Scar => Key::WeaponScar,
            WeaponKind::Fal => Key::WeaponFal,
            WeaponKind::Mp5 => Key::WeaponMp5,
            WeaponKind::Mpx => Key::WeaponMpx,
            WeaponKind::M110 => Key::WeaponM110,
            WeaponKind::Mosin => Key::WeaponMosin,
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
            // 5.56 through a carbine's barrel; then the full-power 7.62 case,
            // which is what plate is for; and a 7.62x54R that barely notices it.
            WeaponKind::M4a1 => 0.76,
            WeaponKind::Scar => 0.82,
            WeaponKind::Fal => 0.90,
            WeaponKind::Mp5 => 0.57,
            WeaponKind::Mpx => 0.52,
            WeaponKind::M110 => 0.90,
            WeaponKind::Mosin => 0.98,
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
            WeaponKind::M4a1 => "M4",
            WeaponKind::Scar => "SCA",
            WeaponKind::Fal => "FAL",
            WeaponKind::Mp5 => "MP5",
            WeaponKind::Mpx => "MPX",
            WeaponKind::M110 => "M11",
            WeaponKind::Mosin => "MOS",
        }
    }

    /// Position in `ALL_KINDS`: the index of every per-kind table the game
    /// keeps — the armory's kit, the viewmodel's model cache.
    pub fn index(self) -> usize {
        // `self as usize` rather than a match: the enum has no explicit
        // discriminants, so this is the declaration order, which is the order
        // `ALL_KINDS` is written in and the order every per-kind table is indexed
        // by. The match this replaces had to be edited by hand for every new
        // variant, and forgetting it indexed another weapon's data.
        self as usize
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
            WeaponKind::M4a1 => "m4a1",
            WeaponKind::Scar => "scar",
            WeaponKind::Fal => "fal",
            WeaponKind::Mp5 => "mp5",
            WeaponKind::Mpx => "mpx",
            WeaponKind::M110 => "m110",
            WeaponKind::Mosin => "mosin",
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
    /// Rounds in the magazine that is in the gun.
    pub mag: u32,
    /// The spare magazines, one round count per magazine, in the order they
    /// were packed. `mag` is the one in the gun.
    ///
    /// This is what a magazine *is*: a container with its own count, not a
    /// number of loose rounds. A reload swaps containers, so the rounds left in
    /// a half-empty magazine are carried rather than thrown away — the old
    /// field (`reserve`) let a reload quietly destroy them, which is why a
    /// reservist's ammunition never added up to whole magazines.
    mags: Vec<u32>,
    pub cooldown: f32,
    pub reload_t: f32,
    /// Set for the frame a reload finishes and the magazine is seated,
    /// consumed by [`Weapon::take_reload_done`]. `reload_t` says a reload is
    /// happening; this says it has stopped being true, which is the half a
    /// player behind cover is actually listening for.
    reload_done: bool,
    pub bloom: f32,
    pub kick: f32,
    pub flash: f32,
    pub shots: u32,
    /// Index into `stats().fire_modes`.
    pub fire_mode: usize,
    burst_left: u32,
    pub inspect_t: f32,
    /// Set when something interrupts an inspect, and cleared when the motion has
    /// run back to rest. The timer is wound down at `INSPECT_REWIND` times its
    /// usual rate while this is set, so a trigger pull retracts the arm over a
    /// tenth of a second instead of teleporting it home. Cutting straight to the
    /// rest pose is the one thing that reads as an animation being cancelled
    /// rather than as a hand putting a weapon down.
    inspect_rewind: bool,
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
            mags: Vec::new(),
            cooldown: 0.0,
            reload_t: 0.0,
            reload_done: false,
            bloom: 0.0,
            kick: 0.0,
            flash: 0.0,
            shots: 0,
            fire_mode: 0,
            burst_left: 0,
            inspect_t: 0.0,
            inspect_rewind: false,
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

    /// Whether a reload seated a magazine on this frame, reported once.
    ///
    /// A reload is three events, not one: it starts, it is halfway, and it is
    /// over. Only the last one lets the player leave cover, and it cannot be
    /// read off `reload_t` from outside without polling it — so it is a signal,
    /// like `Enemy::take_footfall`, and taking it clears it.
    pub fn take_reload_done(&mut self) -> bool {
        std::mem::take(&mut self.reload_done)
    }

    pub fn start_inspect(&mut self) {
        if !self.is_reloading() {
            self.inspect_t = INSPECT_TIME;
            self.inspect_rewind = false;
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

    /// Rounds in the spare magazines. This is the number the HUD shows beside
    /// the magazine in the gun, and it is a sum, not a pool: the rounds are in
    /// specific magazines (`magazine_count` of them).
    pub fn reserve(&self) -> u32 {
        self.mags.iter().sum()
    }

    /// How many spare magazines are carried, empty ones included — the count a
    /// player reads to know how many times they can reload.
    pub fn magazine_count(&self) -> usize {
        self.mags.len()
    }

    /// Packs `count` full spare magazines, in the space the resolved build
    /// gives them. What the armory hands over at the start of a mission; the
    /// magazine in the gun is already seated by `with_build`.
    pub fn pack_magazines(&mut self, count: usize) {
        let size = self.stats().mag_size;
        self.mags = vec![size; count];
    }

    /// Rounds found in the field: the magazine in the gun fills first, then the
    /// fullest spare that still has room, so a pickup is never wasted on a full
    /// magazine and never leaves more half-empty magazines behind than it has
    /// to. Returns how many were taken.
    pub fn add_ammo(&mut self, rounds: u32) -> u32 {
        if self.stats().infinite_ammo {
            return 0;
        }
        let size = self.stats().mag_size;
        let mut left = rounds;
        let into_mag = size.saturating_sub(self.mag).min(left);
        self.mag += into_mag;
        left -= into_mag;

        while left > 0 {
            let next = self
                .mags
                .iter()
                .enumerate()
                .filter(|(_, held)| **held < size)
                .max_by_key(|(_, held)| **held)
                .map(|(i, _)| i);
            let Some(i) = next else {
                // Every magazine is full: the rest of the pickup stays on the
                // ground, which is what a full load means.
                break;
            };
            let take = (size - self.mags[i]).min(left);
            self.mags[i] += take;
            left -= take;
        }
        rounds - left
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
        // Firing takes the weapon back off the player's eye, but not instantly:
        // the arm retracts over the next tenth of a second, which is what a hand
        // doing it looks like. See `inspect_rewind`.
        self.inspect_rewind = true;
        true
    }

    /// Whether a spare magazine holds a round — what a reload would take.
    fn fullest_spare(&self) -> Option<usize> {
        self.mags
            .iter()
            .enumerate()
            .filter(|(_, held)| **held > 0)
            .max_by_key(|(_, held)| **held)
            .map(|(i, _)| i)
    }

    pub fn start_reload(&mut self) -> bool {
        // A reload swaps the magazine in the gun for a spare. With no spare
        // holding a round there is nothing to swap in, and the click is dry —
        // where this used to ask whether any loose rounds were left, which is
        // not the same question once magazines are containers.
        if self.stats().infinite_ammo
            || self.is_reloading()
            || self.mag >= self.stats().mag_size
            || self.fullest_spare().is_none()
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
        // An interrupted inspect runs back to rest rather than being cut off, so
        // the arm retracts instead of teleporting home. The timer still ends at
        // zero either way; only the rate differs.
        let rate = if self.inspect_rewind {
            INSPECT_REWIND
        } else {
            1.0
        };
        self.inspect_t = (self.inspect_t - dt * rate).max(0.0);
        if self.inspect_t <= 0.0 {
            self.inspect_rewind = false;
        }
        self.mag_check_t = (self.mag_check_t - dt).max(0.0);

        if self.reload_t > 0.0 {
            self.reload_t -= dt;
            if self.reload_t <= 0.0 {
                self.reload_t = 0.0;
                // The swap. The fullest spare goes in, and the magazine that
                // came out goes back into the spares with whatever it still
                // held — so a reload conserves ammunition instead of destroying
                // the rounds in a half-empty magazine, and a magazine emptied
                // later in the mission is a magazine with nothing in it, not a
                // magazine that never existed.
                if let Some(i) = self.fullest_spare() {
                    let incoming = self.mags.remove(i);
                    let outgoing = std::mem::take(&mut self.mag);
                    self.mags.push(outgoing);
                    self.mag = incoming;
                }
                self.bloom = 0.0;
                self.reload_done = true;
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
    pub fn with_builds(loadout: [Option<WeaponKind>; 2], builds: &[WeaponBuild; KIND_COUNT]) -> Self {
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
    /// the armory's spare magazines, at mission start.
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
            // A reload that was cancelled must not report finishing later.
            w.reload_done = false;
        }
        self.active = i;
        self.switch_t = 0.34;
        // The weapon being drawn starts clean too: no carry-over inspect pose.
        // The switch's own drop-and-rise covers the change, so this one is a cut
        // by design.
        if let Some(w) = self.slots[i].as_mut() {
            w.inspect_t = 0.0;
            w.inspect_rewind = false;
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
    // The three generic magazine probes. They used to ship; the shipping
    // magazine slot is per well now (see `parts`), and these stay where the
    // model's own tests can reach them: one that adds capacity, one that trades
    // capacity for speed through `MulPost`, and one whose numbers are rolled
    // per copy.
    pub const EXTENDED_MAG: PartDef = PartDef {
        id: PartId("extended_mag"),
        slot: SlotId("magazine"),
        interface: InterfaceId("magwell_556"),
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
        interface: InterfaceId("magwell_556"),
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
    /// makes two copies of one part feel different without a second part.
    pub const SURPLUS_LOT: PartDef = PartDef {
        id: PartId("surplus_lot"),
        slot: SlotId("magazine"),
        interface: InterfaceId("magwell_556"),
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
                ..base(SlotClass::Primary, 14.0, 1, 0.055, 30, 1.50)
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
            WeaponKind::M4a1 => WeaponStats {
                base_spread: 0.0044,
                bloom_per_shot: 0.0052,
                max_bloom: 0.042,
                bloom_decay: 0.090,
                recoil_pitch: 0.0150,
                recoil_yaw: 0.0040,
                ads_fov: 42.0,
                move_scale: 1.03,
                headshot_mult: 2.4,
                falloff_start: 26.0,
                falloff_end: 62.0,
                falloff_min: 0.52,
                fire_modes: RIFLE_MODES,
                ..base(SlotClass::Primary, 22.0, 1, 0.070, 30, 1.25)
            },
            WeaponKind::Scar => WeaponStats {
                base_spread: 0.0038,
                bloom_per_shot: 0.0060,
                max_bloom: 0.038,
                bloom_decay: 0.080,
                recoil_pitch: 0.0195,
                recoil_yaw: 0.0046,
                ads_fov: 40.0,
                move_scale: 0.97,
                headshot_mult: 2.4,
                falloff_start: 34.0,
                falloff_end: 78.0,
                falloff_min: 0.58,
                fire_modes: RIFLE_MODES,
                ..base(SlotClass::Primary, 27.0, 1, 0.100, 30, 1.60)
            },
            WeaponKind::Fal => WeaponStats {
                base_spread: 0.0050,
                bloom_per_shot: 0.0075,
                max_bloom: 0.050,
                bloom_decay: 0.075,
                recoil_pitch: 0.0270,
                recoil_yaw: 0.0070,
                ads_fov: 44.0,
                move_scale: 0.93,
                headshot_mult: 2.3,
                falloff_start: 40.0,
                falloff_end: 90.0,
                falloff_min: 0.62,
                fire_modes: RIFLE_MODES,
                ..base(SlotClass::Primary, 43.0, 1, 0.090, 20, 2.20)
            },
            WeaponKind::Mp5 => WeaponStats {
                base_spread: 0.0072,
                bloom_per_shot: 0.0038,
                max_bloom: 0.040,
                bloom_decay: 0.095,
                recoil_pitch: 0.0110,
                recoil_yaw: 0.0030,
                ads_fov: 46.0,
                move_scale: 1.04,
                headshot_mult: 2.0,
                falloff_start: 18.0,
                falloff_end: 46.0,
                falloff_min: 0.45,
                fire_modes: SMG_MODES,
                ..base(SlotClass::Primary, 16.0, 1, 0.078, 30, 1.40)
            },
            WeaponKind::Mpx => WeaponStats {
                base_spread: 0.0080,
                bloom_per_shot: 0.0050,
                max_bloom: 0.048,
                bloom_decay: 0.090,
                recoil_pitch: 0.0125,
                recoil_yaw: 0.0036,
                ads_fov: 44.0,
                move_scale: 1.05,
                headshot_mult: 2.0,
                falloff_start: 16.0,
                falloff_end: 42.0,
                falloff_min: 0.42,
                fire_modes: SMG_MODES,
                ..base(SlotClass::Primary, 15.0, 1, 0.065, 30, 1.35)
            },
            WeaponKind::M110 => WeaponStats {
                base_spread: 0.0016,
                bloom_per_shot: 0.0100,
                max_bloom: 0.045,
                bloom_decay: 0.070,
                recoil_pitch: 0.0300,
                recoil_yaw: 0.0060,
                ads_fov: 26.0,
                move_scale: 0.95,
                headshot_mult: 2.8,
                falloff_start: 60.0,
                falloff_end: 140.0,
                falloff_min: 0.75,
                fire_modes: SEMI,
                ..base(SlotClass::Primary, 55.0, 1, 0.290, 20, 2.10)
            },
            WeaponKind::Mosin => WeaponStats {
                base_spread: 0.0012,
                bloom_per_shot: 0.0200,
                max_bloom: 0.060,
                bloom_decay: 0.060,
                recoil_pitch: 0.0450,
                recoil_yaw: 0.0090,
                ads_fov: 22.0,
                move_scale: 0.90,
                headshot_mult: 3.0,
                falloff_start: 80.0,
                falloff_end: 200.0,
                falloff_min: 0.85,
                fire_modes: SEMI,
                ..base(SlotClass::Primary, 140.0, 1, 1.40, 5, 3.10)
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
        ars.active_weapon_mut().pack_magazines(1);
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

    /// A reload reports finishing on the frame it finishes, and only then: the
    /// signal `main.rs` turns into the "magazine in, weapon live" cue. Nothing
    /// else in the weapon can say when it is safe to come out of cover.
    #[test]
    fn a_reload_reports_finishing_exactly_once() {
        let mut w = Weapon::new(WeaponKind::Rifle);
        w.mag = 4;
        w.pack_magazines(1);
        assert!(!w.take_reload_done(), "a fresh weapon reported a reload");

        assert!(w.start_reload(), "the reload did not start");
        w.update(w.stats().reload_time - FRAME * 0.5);
        assert!(
            !w.take_reload_done(),
            "the reload was reported before the magazine was in"
        );

        w.update(FRAME);
        assert!(w.take_reload_done(), "the finished reload went unreported");
        assert!(!w.take_reload_done(), "the same reload was reported twice");
    }

    /// The other side of the same signal: a reload that was cancelled must not
    /// fire the cue later, or the player hears "ready" from a weapon that never
    /// finished loading.
    #[test]
    fn a_cancelled_reload_never_reports_finishing() {
        let mut ars = Arsenal::new([Some(WeaponKind::Rifle), None]);
        let rifle = SlotClass::Primary as usize;
        ars.active_weapon_mut().mag = 4;
        ars.active_weapon_mut().pack_magazines(1);
        assert!(ars.active_weapon_mut().start_reload());

        ars.select(SlotClass::Melee as usize);
        ars.update(10.0);
        ars.select(rifle);
        assert!(
            !ars.active_weapon_mut().take_reload_done(),
            "a cancelled reload reported finishing"
        );
    }

    /// The magazine is a container, and a reload swaps containers: what is left
    /// in the magazine that comes out goes back into the spares. Rounds are
    /// conserved, which the pool-of-loose-rounds model this replaced did not
    /// manage — a reload out of a half-empty magazine destroyed the half that
    /// was in it.
    #[test]
    fn a_reload_swaps_magazines_and_conserves_rounds() {
        let mut w = Weapon::new(WeaponKind::Rifle);
        assert_eq!(w.mag, 30, "a weapon out of the crate has a magazine seated");
        assert_eq!(
            w.reserve(),
            0,
            "a weapon out of the crate carries no spares"
        );
        assert_eq!(w.magazine_count(), 0);

        // Half the magazine gone, then two full spares packed.
        w.mag = 12;
        w.pack_magazines(2);
        assert_eq!(w.reserve(), 60);
        assert_eq!(w.magazine_count(), 2);

        let total = w.mag + w.reserve();
        assert!(w.start_reload(), "the reload did not start");
        w.update(w.stats().reload_time + 0.01);
        assert_eq!(w.mag, 30, "the spare did not go into the gun");
        assert_eq!(
            w.mag + w.reserve(),
            total,
            "the rounds in the magazine that came out went missing"
        );
        assert_eq!(w.reserve(), 42, "the half-empty magazine was not kept");
        assert_eq!(
            w.magazine_count(),
            2,
            "a magazine changed hands into thin air"
        );
        // A full gun is not reloaded, spares or not.
        assert!(!w.start_reload());

        // The next reload takes the fullest spare, not whatever is next in line.
        w.mag = 4;
        w.mags = vec![9, 30];
        assert!(w.start_reload());
        w.update(w.stats().reload_time + 0.01);
        assert_eq!(w.mag, 30, "the reload did not reach for the fullest spare");
        assert_eq!(w.mags, vec![9, 4], "what came out did not go back in");
    }

    /// A reload needs a spare that holds a round, and an empty magazine is
    /// still carried: it is a container, and the armory's count of them does not
    /// shrink when one is emptied. That is what a later pickup has to fill.
    #[test]
    fn a_reload_needs_a_spare_with_a_round_in_it() {
        let mut w = Weapon::new(WeaponKind::Smg);
        w.pack_magazines(1);
        w.mag = 0;
        w.mags = vec![0];
        assert!(!w.start_reload(), "a reload loaded from an empty magazine");
        assert_eq!(w.magazine_count(), 1, "the empty magazine was dropped");

        // A pickup fills the gun first, then the spare.
        assert_eq!(w.add_ammo(40), 40);
        assert_eq!(w.mag, 30);
        assert_eq!(w.reserve(), 10, "the pickup did not go into the spare");

        // A full load takes nothing.
        w.mag = 30;
        w.mags = vec![30];
        let total = w.mag + w.reserve();
        assert_eq!(w.add_ammo(60), 0, "a full load took more ammunition");
        assert_eq!(w.mag + w.reserve(), total);
    }

    /// The catalogue the game ships is a set of choices, and every part seats
    /// somewhere: ids unique, each part built for a slot some gun offers, each
    /// standard matching the slot it names — and a fully-kitted rifle legal.
    ///
    /// "Some gun" rather than "every gun" since the wells split: a 60-round
    /// 5.56 quad-stack is meant to fit the rifle and nothing else, and this is
    /// where that stops being an accident.
    #[test]
    fn the_shipped_catalogue_is_sound() {
        let mut seen = Vec::new();
        for p in SHIPPED {
            assert!(!seen.contains(&p.id.0), "duplicate part id {}", p.id.0);
            seen.push(p.id.0);

            let fits = ALL_KINDS.iter().any(|kind| {
                def(*kind)
                    .slots
                    .iter()
                    .any(|s| s.id == p.slot && s.accepts == p.interface)
            });
            assert!(
                fits,
                "{} fits no gun: {} on {}",
                p.id.0, p.interface.0, p.slot.0
            );
        }

        // A magazine is the interesting case: every weapon has a well, and every
        // well takes at least one magazine we ship — otherwise that weapon's
        // magazine row is a single entry, which reads as a weapon with one
        // magazine.
        //
        // Weapons *may* share a well, and now do: an M4A1, a SCAR-L and the
        // MK-7 all take STANAG 5.56 magazines, and the two 9 mm machine pistols
        // take the same sticks as the Viper. This used to assert that no two
        // weapons shared a well, on the theory that a shared well let the
        // rifle's sixty fit the shotgun; it never could, because the resolver
        // checks the interface at fit time, and the rule that actually matters —
        // a well with nothing written for it — is the assertion below.
        for kind in ALL_KINDS {
            let slots = def(kind).slots;
            if slots.is_empty() {
                assert_eq!(kind, WeaponKind::Knife, "{kind:?} offers no slots at all");
                continue;
            }
            let well = slots
                .iter()
                .find(|s| s.id == SlotId("magazine"))
                .unwrap_or_else(|| panic!("{kind:?} offers no magazine well"));
            assert!(
                SHIPPED
                    .iter()
                    .any(|p| p.slot == SlotId("magazine") && p.interface == well.accepts),
                "{kind:?}'s well takes none of the magazines we ship"
            );
        }

        // And the rifle's every slot can be filled from the shipping catalogue.
        let kit = WeaponBuild {
            entries: def(WeaponKind::Rifle)
                .slots
                .iter()
                .map(|slot| BuildEntry {
                    slot: slot.id,
                    part: SHIPPED
                        .iter()
                        .find(|p| p.slot == slot.id && p.interface == slot.accepts)
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
        for id in SLOT_IDS {
            assert!(slot_label(id).is_some(), "{} has no name", id.0);
        }
    }
}
