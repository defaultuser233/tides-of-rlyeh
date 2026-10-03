//! Campaign mission definitions: objective type, enemy composition and spawn
//! pacing. The loadout is the player's — picked and stocked in the stash — so
//! what a mission carries is no longer tabled here. Pure data, so the whole
//! campaign is reviewable in one place.
//!
//! The pacing that data produces is measured in `tests::show_the_pacing_report`
//! and asserted in the tests beside it: how each mission's points budget is
//! split into waves, that every wave can pay for its floor, that the opening
//! is quiet and the peak beats the mean, and where the campaign's ladder
//! lands. The short version is at `curve`.

use crate::enemy;
use crate::enemy::Kind as EnemyKind;
use crate::i18n::Key;

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
    /// The mission's total spawn budget, in points. One point is one unit of
    /// `enemy::stats(kind).score` — the score table doubles as the cost table
    /// (reference §19.4-3), so an archetype's kill value and its price can
    /// never drift apart.
    pub budget: u32,
    /// How many waves the budget is paid out over. Each wave's share grows
    /// across the mission (`wave_budget`), so a run has a shape instead of a
    /// plateau.
    pub waves: u32,
    /// Cap on simultaneously live hostiles.
    ///
    /// The field refills while fewer than this many are up, so it is also
    /// what paces a single wave's releases.
    pub max_alive: u32,
    /// Spawn weights: `(kind, weight)`.
    pub kinds: &'static [(EnemyKind, u32)],
    /// The enemy strength the mission *opens* at — `Enemy::spawn`'s `wave`
    /// argument for its first wave: health scales with `1 + 0.13 * (wave - 1)`,
    /// speed and damage with it too. Declared outright as the base; the wave
    /// index is added on top (`wave_strength` below), so a mission's late
    /// waves fight stronger hostiles than its opening without the campaign
    /// having to table every step.
    pub wave: u32,
    /// Spawn-cadence accelerator: shortens the delay between releases inside
    /// a wave. Enemy strength lives in `wave`.
    pub difficulty: f32,
    pub score_bonus: u32,
}

macro_rules! mission {
    ($layout:expr, $obj:expr, $survive:expr, $budget:expr, $waves:expr, $alive:expr, $kinds:expr, $wave:expr, $diff:expr, $bonus:expr) => {
        Mission {
            layout: $layout,
            objective: $obj,
            survive_seconds: $survive,
            budget: $budget,
            waves: $waves,
            max_alive: $alive,
            kinds: $kinds,
            wave: $wave,
            difficulty: $diff,
            score_bonus: $bonus,
        }
    };
}

pub static MISSIONS: [Mission; 6] = [
    // 1 — teaching mission: open ground, light infantry only, four short
    // waves that climb from a two-man probe to a full skirmish.
    mission!(
        0,
        Objective::Eliminate,
        0.0,
        1350,
        4,
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
        3050,
        5,
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
        3250,
        5,
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
    // 4 — close quarters, timed hold-out, stalkers swarm. The budget is sized
    // against the clock, not the player's patience: six waves over the 120 s
    // hold-out, the last worth more than three times the first, so the timer
    // ends on the mission's own peak rather than trailing off.
    mission!(
        3,
        Objective::Survive,
        120.0,
        11000,
        6,
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
    // 5 — everything at once: all five archetypes show up, marksmen on the
    // rooftops and drones in the air, and the answer to both is a scoped
    // weapon. Whether the player brings one is the stash's business now.
    mission!(
        4,
        Objective::EliminateThenExtract,
        0.0,
        5750,
        6,
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
    // than 6: the finale earns a step the rest of the campaign does not. The
    // Colossus is only affordable in the last waves, and its buffer rule
    // keeps it from returning two waves in a row.
    mission!(
        5,
        Objective::EliminateThenExtract,
        0.0,
        12500,
        6,
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

/// The number of missions, as a constant: the saved best scores are one per
/// mission, and a table's length is not a value the settings file gets to
/// disagree with.
pub const COUNT: usize = MISSIONS.len();

pub fn count() -> usize {
    COUNT
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

/// One point is one unit of `enemy::stats(kind).score`.
pub fn cost(kind: EnemyKind) -> u32 {
    enemy::stats(kind).score
}

/// The cheapest hostile this mission can roll.
pub fn cheapest(m: &Mission) -> u32 {
    m.kinds
        .iter()
        .map(|(k, _)| cost(*k))
        .min()
        .unwrap_or_else(|| cost(EnemyKind::Grunt))
}

/// What one roll of the table is worth on average, in points.
pub fn mean_cost(m: &Mission) -> f32 {
    let total: u32 = m.kinds.iter().map(|(_, w)| *w).sum();
    if total == 0 {
        return cost(EnemyKind::Grunt) as f32;
    }
    let sum: f32 = m
        .kinds
        .iter()
        .map(|(k, w)| cost(*k) as f32 * *w as f32)
        .sum();
    sum / total as f32
}

/// The least number of hostiles a wave is allowed to field: two, because a
/// wave of one is a stray, not a wave. Every wave's budget is raised to this
/// floor when a share alone cannot cover it — which in practice is only ever
/// the opening wave of the smallest mission.
pub const WAVE_FLOOR: u32 = 2;

/// Wave `wave`'s share of the mission's budget.
///
/// The weights climb linearly from 1 to 3.8 across the mission, so every
/// mission gets the same shape: the opening wave releases ~42% of the mean
/// (a quiet start — there is time to look around) and the peak wave ~1.58×
/// the mean. The pacing tests assert those ratios; they are properties of
/// this one formula, not of each table.
pub fn wave_share(waves: u32, wave: u32) -> f32 {
    if waves <= 1 {
        return 1.0;
    }
    let n = (waves - 1) as f32;
    let i = wave.min(waves - 1) as f32;
    let weight = 1.0 + 2.8 * i / n;
    let total: f32 = (0..waves).map(|w| 1.0 + 2.8 * w as f32 / n).sum();
    weight / total
}

/// What wave `wave` may spend, in points.
///
/// Rounded on cumulative boundaries, not per share, so a mission's waves add
/// up to its budget exactly when no floor binds. The floor is a separate
/// promise: even a wave whose share is small must be able to buy
/// `wave_floor` cheap hostiles, so the opening wave of the smallest mission
/// is a skirmish rather than a lone enemy. Overshoot comes out of nothing —
/// mission totals stay approximate by design.
pub fn wave_budget(m: &Mission, wave: u32) -> u32 {
    let upto = |w: u32| -> u32 {
        let sum: f32 = (0..w).map(|i| wave_share(m.waves, i)).sum();
        (m.budget as f32 * sum).round() as u32
    };
    let share = upto(wave + 1).saturating_sub(upto(wave));
    share.max(WAVE_FLOOR * cheapest(m))
}

/// The strength argument for `Enemy::spawn`: the mission's declared base plus
/// the wave index.
///
/// Every hostile in a mission used to spawn at the same constant, so the only
/// escalation inside a mission was *how many* arrived; with this, the late
/// waves hit harder per hostile as well. The finale's last wave fights at
/// strength 12 — the top of the spawn curve the campaign had already sized
/// (reference §19.4-3: the shortest path left, and the one piece an endless
/// mode was still missing).
pub fn wave_strength(m: &Mission, wave_index: u32) -> u32 {
    m.wave.saturating_add(wave_index)
}

/// An archetype that may not arrive two waves in a row (reference §19.2 —
/// ULTRAKILL's buffer rule for its Hideous Mass, scaled down). A threshold
/// rather than a hand-kept list: anything worth 1000 points — a fifth of the
/// finale's budget — is a moment, not a mob.
pub fn is_elite(kind: EnemyKind) -> bool {
    cost(kind) >= 1000
}

/// Can `kind` be paid for now, leaving enough budget for the wave's floor?
pub fn kind_sits(m: &Mission, kind: EnemyKind, remaining: u32, floor_left: u32) -> bool {
    let reserve = cheapest(m).saturating_mul(floor_left.saturating_sub(1));
    cost(kind) <= remaining.saturating_sub(reserve)
}

/// The best-fitting kind when the draws keep missing: the most expensive
/// archetype that fits and is allowed. `None` when nothing does — the wave's
/// budget is spent.
pub fn best_fitting(
    m: &Mission,
    remaining: u32,
    floor_left: u32,
    elite_ready: bool,
) -> Option<EnemyKind> {
    m.kinds
        .iter()
        .map(|(k, _)| *k)
        .filter(|k| kind_sits(m, *k, remaining, floor_left))
        .filter(|k| elite_ready || !is_elite(*k))
        .max_by_key(|k| cost(*k))
}

/// Draws the kind for one release: a weighted roll from the mission's table
/// with two rules layered on top — the kind has to fit the wave's remaining
/// points without eating the floor, and an elite may not arrive while its
/// buffer is still counting down.
///
/// `rolls` yields raw `u32`s so the whole decision stays a pure function a
/// test can replay; a draw that trips either rule is simply retried, and only
/// after the retries are exhausted does the deterministic fallback
/// (`best_fitting`) decide.
pub fn draw_kind(
    m: &Mission,
    rolls: &mut impl FnMut() -> u32,
    remaining: u32,
    floor_left: u32,
    elite_ready: bool,
) -> Option<EnemyKind> {
    const ATTEMPTS: usize = 8;
    for _ in 0..ATTEMPTS {
        let kind = roll_kind(m, rolls());
        if !kind_sits(m, kind, remaining, floor_left) {
            continue;
        }
        if !elite_ready && is_elite(kind) {
            continue;
        }
        return Some(kind);
    }
    best_fitting(m, remaining, floor_left, elite_ready)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The mission's pressure curve, one value per wave: the points that wave
    /// may release. Points are threat (cost *is* score — see `cost`), so this
    /// curve is also the shape a player meets.
    ///
    /// The two properties the tests below hold up — a quiet opening and a
    /// peak at least 1.5× the mean — are properties of `wave_share`, but not
    /// of the *floored* budgets: a floor lifts the smallest waves, which trims
    /// the ratio. That is why the tests read the real budgets and this comment
    /// records both.
    fn curve(m: &Mission) -> Vec<f32> {
        (0..m.waves).map(|w| wave_budget(m, w) as f32).collect()
    }

    /// The opening has to be quiet: the first wave must sit well under the
    /// mission's own average, and it must not be the mission's peak.
    #[test]
    fn every_mission_opens_quiet() {
        for (i, m) in MISSIONS.iter().enumerate() {
            let c = curve(m);
            let mean = c.iter().sum::<f32>() / c.len() as f32;
            let peak = c.iter().copied().fold(f32::MIN, f32::max);
            assert!(
                c[0] <= mean * 0.75,
                "mission {} opens at {:.0} against its own mean of {:.0}",
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

    /// Every mission has to build from that quiet opening to a peak — the
    /// fight that starts as a skirmish has to become a fight — and the peak
    /// has to beat both the opening and the mission's own mean by 1.5×.
    #[test]
    fn every_mission_builds_to_a_peak() {
        for (i, m) in MISSIONS.iter().enumerate() {
            let c = curve(m);
            let mean = c.iter().sum::<f32>() / c.len() as f32;
            let peak = c.iter().copied().fold(f32::MIN, f32::max);
            assert!(
                peak >= c[0] * 1.5,
                "mission {} peaks at {:.0} against an opening of {:.0}: it never escalates",
                i + 1,
                peak,
                c[0]
            );
            assert!(
                peak >= mean * 1.5,
                "mission {} peaks at {:.0} against a mean of {:.0}",
                i + 1,
                peak,
                mean
            );
        }
    }

    /// The ladder from mission 1 to mission 6 is the other half of the
    /// escalation: the finale's peak must tower over the opener's.
    #[test]
    fn the_campaign_is_where_the_peak_lives() {
        let peak_of = |m: &Mission| curve(m).into_iter().fold(f32::MIN, f32::max);
        let first = peak_of(&MISSIONS[0]);
        let last = peak_of(&MISSIONS[count() - 1]);
        assert!(
            last >= first * 3.0,
            "the finale peaks at {last:.0} against the opener's {first:.0}"
        );
    }

    /// Shares are a distribution and they climb: every wave worth more than
    /// the last, and the shares of one mission summing to one.
    #[test]
    fn wave_shares_climb_and_sum_to_one() {
        for m in MISSIONS.iter() {
            let shares: Vec<f32> = (0..m.waves).map(|w| wave_share(m.waves, w)).collect();
            let sum: f32 = shares.iter().sum();
            assert!(
                (sum - 1.0).abs() < 1e-4,
                "a mission's wave shares sum to {sum}"
            );
            for pair in shares.windows(2) {
                assert!(pair[1] > pair[0], "wave shares {shares:?} do not climb");
            }
        }
    }

    /// Wave strength climbs from the mission's own base, one step per wave:
    /// the opening wave fights at the declared strength, the last well above.
    #[test]
    fn wave_strength_climbs_from_the_missions_base() {
        for m in MISSIONS.iter() {
            assert_eq!(wave_strength(m, 0), m.wave, "the opening wave is the base");
            for w in 1..m.waves {
                assert_eq!(
                    wave_strength(m, w),
                    wave_strength(m, w - 1) + 1,
                    "strength has to step by one a wave"
                );
            }
        }
    }

    /// Every wave's budget can at least buy its floor — a wave that cannot
    /// field its minimum is not a wave, whatever the arithmetic says.
    #[test]
    fn every_wave_can_afford_its_floor() {
        for (i, m) in MISSIONS.iter().enumerate() {
            for w in 0..m.waves {
                assert!(
                    wave_budget(m, w) >= WAVE_FLOOR * cheapest(m),
                    "mission {} wave {} can buy {} points of a floor that costs {}",
                    i + 1,
                    w + 1,
                    wave_budget(m, w),
                    WAVE_FLOOR * cheapest(m)
                );
            }
        }
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
                    cost(*kind) > 0,
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
                m.waves >= 2,
                "mission {} is a single wave, so it has no shape",
                i + 1
            );
            assert!(
                m.budget >= m.waves * cheapest(m),
                "mission {} cannot pay for one hostile a wave",
                i + 1
            );
        }
    }

    /// The elite buffer (reference §19.2): an elite may not come back two
    /// waves in a row. The old spawner rolled every release independently, so
    /// a Colossus could die and another arrive immediately behind it — an
    /// event with no cooldown reads as a bug, and it was one. Twenty
    /// simulated waves on the finale's table: every elite has to keep its
    /// distance, and every wave still has to fill its floor.
    #[test]
    fn elites_never_come_back_to_back() {
        let m = &MISSIONS[5];
        let mut seed = 0x1234_5678u32;
        let mut rolls = move || {
            seed ^= seed << 13;
            seed ^= seed >> 17;
            seed ^= seed << 5;
            seed
        };
        let mut last: Option<u32> = None;
        for wave in 0..20u32 {
            let mut ready = last.is_none_or(|l| wave >= l + 2);
            let mut remaining = 3300;
            let mut released = 0;
            loop {
                let floor_left = WAVE_FLOOR.saturating_sub(released);
                let Some(kind) = draw_kind(m, &mut rolls, remaining, floor_left, ready) else {
                    break;
                };
                if is_elite(kind) {
                    assert!(ready, "wave {wave} released an elite while buffered");
                    last = Some(wave);
                    ready = false;
                }
                remaining -= cost(kind);
                released += 1;
                if remaining < cheapest(m) {
                    break;
                }
            }
            assert!(
                released >= WAVE_FLOOR,
                "wave {wave} released {released} hostiles, under its floor"
            );
        }
        assert!(last.is_some(), "twenty waves and not one elite appeared");
    }

    /// Not an assertion: the pacing report. `cargo test -- --ignored --nocapture`.
    ///
    /// Prints each mission's per-wave budgets, the ratios the tests above work
    /// from, and the mean roll a budget buys against.
    #[test]
    #[ignore = "prints the campaign's pressure curves; run with --ignored --nocapture"]
    fn show_the_pacing_report() {
        let mut all: Vec<f32> = Vec::new();
        for (i, m) in MISSIONS.iter().enumerate() {
            let c = curve(m);
            let mean = c.iter().sum::<f32>() / c.len() as f32;
            let peak = c.iter().copied().fold(f32::MIN, f32::max);
            let interval = (1.9 - 0.06 * m.difficulty).max(0.7);
            println!(
                "=== mission {} — {} waves, budget {} (mean roll {:.0}), interval {:.2} s ===",
                i + 1,
                m.waves,
                m.budget,
                mean_cost(m),
                interval
            );
            print!("  budget:");
            for v in &c {
                print!(" {v:.0}");
            }
            println!();
            println!(
                "  opening {:.0} | mean {:.0} | peak {:.0} | peak/mean {:.2}",
                c[0],
                mean,
                peak,
                peak / mean
            );
            all.extend(c);
        }
        let mean = all.iter().sum::<f32>() / all.len() as f32;
        let peak = all.iter().copied().fold(f32::MIN, f32::max);
        println!(
            "=== the campaign: peak {peak:.0} vs mean {mean:.0}, ratio {:.2} ===",
            peak / mean
        );
    }
}
