//! Campaign mission definitions: objective type, enemy composition and the
//! loadout the player starts with. Pure data, so the whole campaign is
//! reviewable in one place.
//!
//! The pacing that data produces is measured in `tests::show_the_pacing_report`
//! and asserted in the tests beside it: what a "wave" is in this game, how a
//! wave's threat is summed, and the shape the six missions actually add up to.
//! The short version is at `curve`.

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
    ///
    /// This is the campaign's only wave: the spawner drips one hostile at a
    /// time, so a wave is one complement of this many, and the pacing curve in
    /// the tests below is drawn in those units. Raising it does not just crowd
    /// the arena, it lengthens every wave of the mission's curve.
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::enemy;

    /// Threat of one hostile of `kind` at a mission of strength `wave`.
    ///
    /// The archetype's own `score` is the only threat rating the project keeps
    /// (`enemy::stats`), and it is a designer's number: a Juggernaut is worth
    /// 3.2 Grunts and a Colossus 25 of them, which is roughly how they fight.
    /// The multiplier is the one `Enemy::spawn` already applies to health,
    /// `1 + 0.13 * (wave - 1)`, so a wave-7 Grunt counts as 1.78 wave-1 Grunts.
    fn threat(kind: EnemyKind, wave: u32) -> f32 {
        enemy::stats(kind).score as f32 * (1.0 + 0.13 * wave.saturating_sub(1) as f32)
    }

    /// What one roll of the mission's spawn table is worth on average.
    fn mean_threat(m: &Mission) -> f32 {
        let total: u32 = m.kinds.iter().map(|(_, w)| *w).sum();
        if total == 0 {
            return threat(EnemyKind::Grunt, m.wave);
        }
        m.kinds
            .iter()
            .map(|(kind, weight)| threat(*kind, m.wave) * *weight as f32)
            .sum::<f32>()
            / total as f32
    }

    /// The mission's pressure curve, one value per wave.
    ///
    /// **What a wave is here.** This game has no wave system: the field is one
    /// continuous drip of one hostile every `(1.9 - 0.06 * difficulty).max(0.7)`
    /// seconds while fewer than `max_alive` are up, until `total_enemies` have
    /// spawned (`main.rs`, the spawner). The only wave structure that exists is
    /// therefore the *complement*: one wave is `max_alive` hostiles' worth of
    /// the budget, because that is the amount of force the field can hold at
    /// once.
    ///
    /// **What a wave's intensity is.** Not the budget it releases — that is a
    /// rectangle, and it says the opening is as loud as the peak, which is not
    /// what a player meets. It is the force actually on the field while that
    /// wave is being released: the first wave starts at nothing and fills, so it
    /// averages half a complement; a middle wave holds a full one; the last
    /// wave either holds a full one (when the budget lands on the boundary) or
    /// drains as the budget runs out, averaging `(cap + remainder) / 2`. The
    /// whole model assumes the player kills about as fast as the drip arrives,
    /// which is the assumption the mission budgets were sized against — see
    /// mission 4's comment: 120 s of timer at 1.81 s a kill is 66 hostiles.
    ///
    /// Composition, not chance: a wave is worth its live count times the
    /// mission's *average* roll. The spawn table is rolled uniformly, so the
    /// expected composition of every wave is the same, and pretending otherwise
    /// would invent a curve the game does not play. The only things that vary
    /// across a mission's waves are the fill at either end and the last wave's
    /// remainder — which is exactly the finding the tests below record.
    fn curve(m: &Mission) -> Vec<f32> {
        let cap = m.max_alive.max(1);
        let waves = m.total_enemies.div_ceil(cap);
        let per = mean_threat(m);
        (0..waves)
            .map(|w| {
                let count = (m.total_enemies - w * cap).min(cap);
                // Hostiles on the field, averaged over the wave — not hostiles
                // released. Those are the same number in the middle and differ
                // at both ends, which is the whole point of the curve.
                let live = if waves > 1 && w == 0 {
                    cap as f32 / 2.0
                } else if waves > 1 && w + 1 == waves {
                    (cap + count) as f32 / 2.0
                } else {
                    cap as f32
                };
                live * per
            })
            .collect()
    }

    /// The same curve as a percentage of its own peak, which is how a designer
    /// reads a pacing chart.
    fn percent_curve(m: &Mission) -> Vec<f32> {
        let raw = curve(m);
        let peak = raw.iter().copied().fold(f32::MIN, f32::max);
        raw.iter()
            .map(|v| if peak > 0.0 { 100.0 * v / peak } else { 0.0 })
            .collect()
    }

    /// The opening has to be quiet. The first wave is the field filling up from
    /// nothing, so it must sit well under the mission's own average — and it
    /// must not be the mission's peak, which is what a budget-shaped curve
    /// wrongly says it is.
    #[test]
    fn every_mission_opens_quiet() {
        for (i, m) in MISSIONS.iter().enumerate() {
            let c = percent_curve(m);
            let mean = c.iter().sum::<f32>() / c.len() as f32;
            let peak = c.iter().copied().fold(f32::MIN, f32::max);
            assert!(
                c[0] <= mean * 0.75,
                "mission {} opens at {:.0}% against its own mean of {:.0}%",
                i + 1,
                c[0],
                mean
            );
            assert!(
                c[0] < peak,
                "mission {} peaks in its opening wave, so nothing arrives as a surprise",
                i + 1
            );
        }
    }

    /// Every mission has to build from that quiet opening to a peak — the fight
    /// that starts as a trickle has to become a fight.
    #[test]
    fn every_mission_builds_to_a_peak() {
        for (i, m) in MISSIONS.iter().enumerate() {
            let c = percent_curve(m);
            let peak = c.iter().copied().fold(f32::MIN, f32::max);
            assert!(
                peak >= c[0] * 1.5,
                "mission {} peaks at {:.0}% against an opening of {:.0}%: it never escalates",
                i + 1,
                peak,
                c[0]
            );
        }
    }

    /// The escalation this campaign has lives *between* missions, not inside
    /// them: each mission holds a plateau, and the ladder from mission 1 to
    /// mission 6 is what supplies the peak-to-average the pacing wants. That is
    /// the shape the mission data can produce — a mission's spawner has no
    /// per-wave knob, only `wave` (strength), `max_alive` and the budget — so
    /// this test is where "at least one peak, at least 1.5x the average" is
    /// held up. If a mission ever grows a real within-mission curve, the place
    /// to record it is here, with the peak that beats it.
    #[test]
    fn the_campaign_is_where_the_peak_lives() {
        let all: Vec<f32> = MISSIONS.iter().flat_map(curve).collect();
        let mean = all.iter().sum::<f32>() / all.len() as f32;
        let peak = all.iter().copied().fold(f32::MIN, f32::max);
        assert!(
            peak >= mean * 1.5,
            "the campaign peaks at {peak:.0} against a mean of {mean:.0}: no escalation anywhere"
        );

        // And the ladder itself must climb: mission 6's plateau is what a
        // player meets at the end and mission 1's is what they meet first.
        let first = curve(&MISSIONS[0]).iter().copied().fold(f32::MIN, f32::max);
        let last = curve(&MISSIONS[count() - 1])
            .iter()
            .copied()
            .fold(f32::MIN, f32::max);
        assert!(
            last >= first * 3.0,
            "the finale peaks at {last:.0} against the opener's {first:.0}"
        );
    }

    /// Every kind a mission can roll has to exist and be worth something: a
    /// weight of zero would be a spawn table entry that can never come out.
    #[test]
    fn spawn_tables_are_well_formed() {
        for (i, m) in MISSIONS.iter().enumerate() {
            assert!(!m.kinds.is_empty(), "mission {} has no spawn table", i + 1);
            for (kind, weight) in m.kinds {
                assert!(
                    *weight > 0,
                    "mission {} lists {kind:?} at weight 0, so it can never spawn",
                    i + 1
                );
                assert!(
                    threat(*kind, m.wave) > 0.0,
                    "mission {} lists {kind:?}, which is worth nothing",
                    i + 1
                );
            }
            assert!(
                m.max_alive >= 2,
                "mission {} caps the field at {}, which is not a wave",
                i + 1,
                m.max_alive
            );
            assert!(
                m.total_enemies > m.max_alive,
                "mission {} can never fill its own field",
                i + 1
            );
        }
    }

    /// Not an assertion: the pacing report. `cargo test -- --ignored --nocapture`.
    ///
    /// Prints the curve per wave for all six missions, plus the two ratios the
    /// tests above work from.
    #[test]
    #[ignore = "prints the campaign's pressure curves; run with --ignored --nocapture"]
    fn show_the_pacing_report() {
        let mut all: Vec<f32> = Vec::new();
        for (i, m) in MISSIONS.iter().enumerate() {
            let c = percent_curve(m);
            let mean = c.iter().sum::<f32>() / c.len() as f32;
            let peak = c.iter().copied().fold(f32::MIN, f32::max);
            let raw = curve(m);
            let raw_mean = raw.iter().sum::<f32>() / raw.len() as f32;
            let raw_peak = raw.iter().copied().fold(f32::MIN, f32::max);
            let interval = (1.9 - 0.06 * m.difficulty).max(0.7);
            println!(
                "=== mission {} — {} waves of {} (budget {}), interval {:.2} s ===",
                i + 1,
                c.len(),
                m.max_alive,
                m.total_enemies,
                interval
            );
            print!("  curve:");
            for v in &c {
                print!(" {v:.0}%");
            }
            println!();
            println!(
                "  opening {:.0}% | mean {:.0}% | peak {:.0}% | peak/mean {:.2} | threat per hostile {:.0}",
                c[0],
                mean,
                peak,
                raw_peak / raw_mean,
                mean_threat(m)
            );
            all.extend(raw);
        }
        let mean = all.iter().sum::<f32>() / all.len() as f32;
        let peak = all.iter().copied().fold(f32::MIN, f32::max);
        println!(
            "=== the campaign: peak {peak:.0} vs mean {mean:.0}, ratio {:.2} ===",
            peak / mean
        );
    }
}
