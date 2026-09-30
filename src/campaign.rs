//! Campaign mission definitions: objective type, enemy composition and the
//! loadout the player starts with. Pure data, so the whole campaign is
//! reviewable in one place.

use crate::enemy::Kind as EnemyKind;
use crate::i18n::Key;
use crate::weapon::WeaponKind;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Objective {
    /// Kill every hostile on the field.
    Eliminate,
    /// Kill every hostile, then walk to the extraction point.
    EliminateThenExtract,
    /// Survive for `survive_seconds`, then extraction opens up.
    Survive,
}

impl Objective {
    pub fn key(self) -> Key {
        match self {
            Objective::Eliminate => Key::ObjEliminate,
            Objective::EliminateThenExtract => Key::ObjEliminate,
            Objective::Survive => Key::ObjSurvive,
        }
    }

    /// Secondary objective line shown under the primary one.
    pub fn sub_key(self) -> Option<Key> {
        match self {
            Objective::Eliminate => None,
            Objective::EliminateThenExtract => Some(Key::ObjThenExtract),
            Objective::Survive => None,
        }
    }
}

pub struct Mission {
    /// Index into `maps::campaign()`.
    pub layout: usize,
    pub objective: Objective,
    /// Only meaningful for `Objective::Survive`.
    pub survive_seconds: f32,
    /// Weapons the player starts holding, one per long-gun slot.
    ///
    /// Exactly as long as the arsenal's two long slots, on purpose: a longer
    /// list used to be truncated silently by `Arsenal::new`, which is how
    /// mission 5 shipped carrying five weapons and getting two. A list that
    /// does not fit is now a type error.
    pub loadout: [Option<WeaponKind>; 2],
    /// Total hostiles to spawn before the field stops replenishing.
    pub total_enemies: u32,
    /// Cap on simultaneously live hostiles.
    pub max_alive: u32,
    /// Spawn weights: `(kind, weight)`.
    pub kinds: &'static [(EnemyKind, u32)],
    /// Enemy strength, handed to `Enemy::spawn` as its `wave` argument: health
    /// scales with `1 + 0.13 * (wave - 1)`, speed and damage with it too.
    /// Declared outright so the campaign can neither skip a step nor derive a
    /// value nobody intended.
    pub wave: u32,
    /// Spawn-cadence accelerator only: shortens the delay between
    /// reinforcements. Enemy strength lives in `wave`.
    pub difficulty: f32,
    pub score_bonus: u32,
}

macro_rules! mission {
    ($layout:expr, $obj:expr, $survive:expr, $loadout:expr, $total:expr, $alive:expr, $kinds:expr, $wave:expr, $diff:expr, $bonus:expr) => {
        Mission {
            layout: $layout,
            objective: $obj,
            survive_seconds: $survive,
            loadout: $loadout,
            total_enemies: $total,
            max_alive: $alive,
            kinds: $kinds,
            wave: $wave,
            difficulty: $diff,
            score_bonus: $bonus,
        }
    };
}

pub static MISSIONS: [Mission; 6] = [
    // 1 — teaching mission: open ground, light infantry only.
    mission!(
        0,
        Objective::Eliminate,
        0.0,
        [Some(WeaponKind::Rifle), None],
        12,
        5,
        &[(EnemyKind::Grunt, 6), (EnemyKind::Rusher, 2)],
        1,
        1.00,
        500
    ),
    // 2 — introduces armour and a second weapon slot.
    mission!(
        1,
        Objective::EliminateThenExtract,
        0.0,
        [Some(WeaponKind::Rifle), Some(WeaponKind::Shotgun)],
        18,
        6,
        &[
            (EnemyKind::Grunt, 5),
            (EnemyKind::Rusher, 3),
            (EnemyKind::Heavy, 2),
        ],
        2,
        1.15,
        900
    ),
    // 3 — long sightlines punish careless movement; marksmen appear.
    mission!(
        2,
        Objective::EliminateThenExtract,
        0.0,
        [Some(WeaponKind::Rifle), Some(WeaponKind::Smg)],
        20,
        7,
        &[
            (EnemyKind::Grunt, 4),
            (EnemyKind::Rusher, 3),
            (EnemyKind::Sniper, 3),
        ],
        3,
        1.30,
        1400
    ),
    // 4 — close quarters, timed hold-out, stalkers swarm. The budget has to
    // outlast the clock, not the player's patience: the drip lands one hostile
    // every `1.9 - 0.06 * difficulty` = 1.81 s while fewer than `max_alive` are
    // up, so 120 s of arena needs about `120 / 1.81` = 66 of them. Twenty-six
    // ran dry inside a minute and left the second half of the timer empty.
    mission!(
        3,
        Objective::Survive,
        120.0,
        [Some(WeaponKind::Shotgun), Some(WeaponKind::Smg)],
        66,
        6,
        &[
            (EnemyKind::Rusher, 6),
            (EnemyKind::Grunt, 3),
            (EnemyKind::Heavy, 2),
        ],
        4,
        1.45,
        2000
    ),
    // 5 — everything at once: all five archetypes show up, and the player
    // carries two of the five weapons. The sniper is the one that matters —
    // this is the mission with marksmen on the rooftops and drones in the air,
    // and until now the loadout asked for it and the arsenal silently dropped
    // it, leaving nothing that could answer either at range.
    mission!(
        4,
        Objective::EliminateThenExtract,
        0.0,
        [Some(WeaponKind::Rifle), Some(WeaponKind::Sniper)],
        30,
        8,
        &[
            (EnemyKind::Grunt, 4),
            (EnemyKind::Rusher, 3),
            (EnemyKind::Heavy, 3),
            (EnemyKind::Sniper, 2),
            (EnemyKind::Drone, 3),
        ],
        5,
        1.60,
        4000
    ),
    // 6 — R'LYEH. The finale: the geometry is wrong, something enormous is
    // awake, and looking at it costs you your mind. Spawnlings are not rolled
    // here on purpose — they only come out of the dead, which is what makes
    // killing a Tentacle horror a decision rather than a win. Wave 7 rather
    // than 6: the finale earns a step the rest of the campaign does not.
    mission!(
        5,
        Objective::EliminateThenExtract,
        0.0,
        [Some(WeaponKind::Rifle), Some(WeaponKind::Lmg)],
        26,
        5,
        &[
            (EnemyKind::Tentacle, 5),
            (EnemyKind::Colossus, 1),
            (EnemyKind::Heavy, 3),
            (EnemyKind::Grunt, 3),
        ],
        7,
        1.80,
        6000
    ),
];

pub fn count() -> usize {
    MISSIONS.len()
}

pub fn get(i: usize) -> &'static Mission {
    &MISSIONS[i.min(MISSIONS.len() - 1)]
}

/// Picks the next enemy archetype from the mission's spawn weights.
pub fn roll_kind(m: &Mission, roll: u32) -> EnemyKind {
    let total: u32 = m.kinds.iter().map(|(_, w)| *w).sum();
    if total == 0 {
        return EnemyKind::Grunt;
    }
    let mut pick = roll % total;
    for (kind, weight) in m.kinds {
        if pick < *weight {
            return *kind;
        }
        pick -= *weight;
    }
    EnemyKind::Grunt
}
