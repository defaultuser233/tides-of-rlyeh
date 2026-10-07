use crate::enemy::Enemy;
use crate::level::Level;
use crate::weapon::Penetration;
use macroquad::prelude::*;

/// Where a ray stopped, and how far along it was.
///
/// There is no "what did it hit" here any more: a body is a [`BodyHit`] inside a
/// [`Lane`] and the world is the end of it, so a caller walking a lane already
/// knows which it has. The last thing that needed a tagged union was the
/// single-hit read of a ray, and the shot path does not take that read now — an
/// enemy firing at the player is an accuracy roll, not a ray.
#[derive(Clone, Copy)]
pub struct Hit {
    pub point: Vec3,
    pub dist: f32,
}

/// A body the ray met, with everything putting a round in it needs. Separate
/// from [`Hit`] because a body in a lane is never the world: the caller walking
/// a lane should not have to ask again which it has.
#[derive(Clone, Copy, Debug)]
pub struct BodyHit {
    pub index: usize,
    pub head: bool,
    pub point: Vec3,
    pub dist: f32,
}

/// One thing in the way, in the order the ray meets it.
#[derive(Clone, Copy, Debug)]
pub enum Step {
    Body(BodyHit),
    /// Solid cover. What it costs a round is its *thickness*, which is the one
    /// thing a single distance cannot say — the old world query answered "the
    /// first surface is 9 m away" and threw the far side of it away.
    Cover {
        point: Vec3,
        dist: f32,
        thickness: f32,
    },
}

impl Step {
    /// How far along the ray this step is.
    pub fn dist(&self) -> f32 {
        match self {
            Step::Body(b) => b.dist,
            Step::Cover { dist, .. } => *dist,
        }
    }
}

/// Everything one ray meets on its way out from the eye, in order.
///
/// The lane is geometry and nothing else. How much of it a *round* can spend is
/// `Penetration`'s business, and keeping that out of here is what lets one lane
/// answer for a pistol round and for a hardened core.
pub struct Lane {
    pub steps: Vec<Step>,
    /// Where the ray ran out of arena: the far side of the last cover it met, or
    /// `max` when there was none.
    pub far: Hit,
}

/// The lane a ray takes from the eye: every body and every span of cover it
/// meets, nearest first.
pub fn hitscan_through(
    level: &Level,
    enemies: &[Enemy],
    origin: Vec3,
    dir: Vec3,
    max: f32,
) -> Lane {
    let spans = level.ray_solids(origin, dir, max);
    let mut steps: Vec<Step> = Vec::with_capacity(spans.len() + enemies.len());
    // Where the far side of the last cover met is, if any cover was met at all.
    // It used to start at `max` and take a running maximum, and because every
    // exit is already clamped to `max` that made the far end the ray's own range
    // whatever stood in the way — a tracer drawn the width of the map for a
    // round that had stopped at a wall (review R11).
    let mut cover_end: Option<f32> = None;
    for (enter, exit) in spans {
        steps.push(Step::Cover {
            point: origin + dir * enter,
            dist: enter,
            thickness: exit - enter,
        });
        cover_end = Some(cover_end.map_or(exit, |end: f32| end.max(exit)));
    }

    for (i, e) in enemies.iter().enumerate() {
        if !e.targetable() {
            continue;
        }
        // No lower bound on `t`. At point-blank range the camera sits inside
        // the enemy's box, `ray_aabb` reports an entry distance of 0, and the
        // old `t > 0.02` test discarded the hit and blamed the world — so the
        // enemy you were touching was the one you could not shoot. `head` is
        // forced false there, because the origin is the eye and every
        // point-blank shot would otherwise score as a headshot.
        if let Some(t) = e.body_aabb().ray(origin, dir)
            && t < max
        {
            let point = origin + dir * t;
            steps.push(Step::Body(BodyHit {
                index: i,
                head: t > 0.001 && point.y > e.head_y(),
                point,
                dist: t,
            }));
        }
    }
    // Ordered by distance, so "the next thing" is a fact about the lane rather
    // than about the order the two tables happen to be kept in.
    steps.sort_by(|a, b| a.dist().total_cmp(&b.dist()));

    // The end of the lane is the far side of the last cover it met, and `max`
    // when it met none. It is never allowed to come back before the lane's own
    // last step: a body standing beyond the last wall is still something the ray
    // reached, and a far end in front of it would put the end of a tracer in the
    // middle of a body it went through.
    let far = match cover_end {
        Some(end) => end.max(steps.last().map_or(0.0, |s| s.dist())),
        None => max,
    };
    Lane {
        steps,
        far: Hit {
            point: origin + dir * far,
            dist: far,
        },
    }
}

/// What a round makes of a lane: which bodies it reaches, what share of the shot
/// each of them takes, and where it stops.
///
/// Pure, and separate from the shot that reads it, because this is the whole of
/// what a round's `Penetration` buys and none of it needs a world to test. A
/// round keeps `carry` of itself for every thing it crosses, body or wall: what
/// it has left when it arrives is the product. With nothing to stop it the round
/// leaves at `lane.far`, where there is nothing left to mark.
pub struct Sweep {
    /// The bodies the round reaches, in order, with the share of the shot each
    /// one takes.
    pub hits: Vec<(BodyHit, f32)>,
    pub stop: Hit,
    /// True when cover is what stopped it, which is the one case that leaves a
    /// mark on a wall.
    pub stopped_on_cover: bool,
}

pub fn sweep(lane: &Lane, round: Penetration) -> Sweep {
    let mut hits: Vec<(BodyHit, f32)> = Vec::new();
    let mut cover = 0.0f32;
    // Everything the round has crossed, bodies and cover alike: what is left of
    // the shot is `carry` raised to that count.
    let mut paid = 0u32;
    // The body the round spent its last target on, if it spent it at all.
    let mut spent_on_body: Option<Hit> = None;

    for step in &lane.steps {
        // A round with no targets left has already stopped in the last body it
        // reached, so nothing behind that body is met at all. Asking this first
        // is what keeps the stop point on the body it hit instead of on the next
        // one down the lane, and stops a spent round reporting a wall it never
        // got to (review R6).
        if hits.len() as u32 >= round.max_targets {
            break;
        }
        match step {
            Step::Cover {
                point, thickness, ..
            } => {
                if cover + thickness > round.wall_budget {
                    return Sweep {
                        hits,
                        stop: Hit {
                            point: *point,
                            dist: step.dist(),
                        },
                        stopped_on_cover: true,
                    };
                }
                cover += thickness;
                paid += 1;
            }
            Step::Body(body) => {
                hits.push((*body, round.after(paid)));
                paid += 1;
                spent_on_body = Some(Hit {
                    point: body.point,
                    dist: body.dist,
                });
            }
        }
    }
    // A round that spent its last target has nowhere left to go even when the
    // lane had nothing behind it: the shot stopped in that body. One with
    // anything left travelled off the end of the lane instead, where there is
    // nothing to mark.
    let stop = match spent_on_body {
        Some(stopped) if hits.len() as u32 >= round.max_targets => stopped,
        _ => lane.far,
    };
    Sweep {
        hits,
        stop,
        stopped_on_cover: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::enemy::{Enemy, Kind};
    use crate::player::Player;
    use crate::util::Rng;
    use crate::weapon::AmmoKind;

    /// The round spends itself in the order the lane is in, and pays for every
    /// thing it crosses — a wall or a body, at the same price. This is the half
    /// of penetration that needs no world at all: the lane is handed in.
    #[test]
    fn a_round_spends_itself_in_the_order_the_lane_is_in() {
        let body = |index: usize, dist: f32| {
            Step::Body(BodyHit {
                index,
                head: false,
                point: vec3(0.0, 0.0, dist),
                dist,
            })
        };
        let cover = |dist: f32, thickness: f32| Step::Cover {
            point: vec3(0.0, 0.0, dist),
            dist,
            thickness,
        };
        let lane = |steps: Vec<Step>| Lane {
            steps,
            far: Hit {
                point: vec3(0.0, 0.0, 40.0),
                dist: 40.0,
            },
        };

        let through_cover = lane(vec![body(0, 3.0), cover(5.0, 0.2), body(1, 8.0)]);
        let core = sweep(&through_cover, AmmoKind::Ap.penetration());
        assert_eq!(core.hits.len(), 2, "the core did not reach both bodies");
        assert_eq!(
            (core.hits[0].0.index, core.hits[1].0.index),
            (0, 1),
            "the bodies came back out of order"
        );
        assert!((core.hits[0].1 - 1.0).abs() < 1.0e-6, "the first body");
        // A body and a wall crossed: two things, two shares of what was left.
        assert!(
            (core.hits[1].1 - 0.36).abs() < 1.0e-6,
            "the second body took {}",
            core.hits[1].1
        );
        assert!(!core.stopped_on_cover, "nothing was left to stop it");
        assert!((core.stop.dist - 40.0).abs() < 1.0e-6);

        // The same lane, a plain round: one body, and it stops in that body.
        // The wall behind it is never reached — the round had no target left to
        // spend by the time it got there, so it cannot report a wall it never
        // touched or spark on one (review R6).
        let ball = sweep(&through_cover, AmmoKind::Fmj.penetration());
        assert_eq!(ball.hits.len(), 1, "the plain round crossed a wall");
        assert!(!ball.stopped_on_cover, "a spent round reported a wall");
        assert!(
            (ball.stop.dist - 3.0).abs() < 1.0e-6,
            "the plain round ended at {} instead of in the body",
            ball.stop.dist
        );

        // Two bodies in a line and a plain round: the tracer ends on the one it
        // hit. It used to end on the *second* one, which is the body it never
        // reached, and read as a hit that did no damage (review R6).
        let pair = lane(vec![body(0, 3.0), body(1, 8.0)]);
        let stopped = sweep(&pair, AmmoKind::Fmj.penetration());
        assert_eq!(stopped.hits.len(), 1);
        assert!(
            (stopped.stop.dist - 3.0).abs() < 1.0e-6,
            "the tracer ended at {} instead of on the first body",
            stopped.stop.dist
        );

        // And a core against cover too thick for it stops there instead.
        let thick = lane(vec![body(0, 3.0), cover(5.0, 0.4)]);
        let blocked = sweep(&thick, AmmoKind::Ap.penetration());
        assert_eq!(blocked.hits.len(), 1);
        assert!(blocked.stopped_on_cover);
        assert!((blocked.stop.dist - 5.0).abs() < 1.0e-6);

        // A round that spends its last target with nothing behind it still
        // stopped in that body rather than flying on to the end of the lane.
        let alone = lane(vec![body(0, 3.0)]);
        let stopped = sweep(&alone, AmmoKind::Fmj.penetration());
        assert_eq!(stopped.hits.len(), 1);
        assert!(
            !stopped.stopped_on_cover && (stopped.stop.dist - 3.0).abs() < 1.0e-6,
            "it ended at {} instead of in the body",
            stopped.stop.dist
        );
    }

    /// A lane is what the ray meets, in order, and cover is a span rather than a
    /// distance: a round that carries through cover needs to know how much of it
    /// there is. Straight down is the one direction every arena answers the same
    /// way — the floor.
    ///
    /// The layout is the open floor the physics tests use, and the ray is aimed
    /// down whichever axis has the most room, so what the lane holds is a fact
    /// about the arena rather than about where the spawn happens to point.
    #[test]
    fn a_lane_orders_what_it_meets_and_cover_has_a_thickness() {
        let level = Level::from_layout(&crate::maps::arena());
        let at = level.spawns[0];
        let eye = Player::new(at).eye();
        let room = |dir: Vec3| level.ray_world(eye, dir, 200.0).unwrap_or(200.0);
        let dir = [vec3(1.0, 0.0, 0.0), vec3(-1.0, 0.0, 0.0)]
            .into_iter()
            .chain([vec3(0.0, 0.0, 1.0), vec3(0.0, 0.0, -1.0)])
            .max_by(|a, b| room(*a).total_cmp(&room(*b)))
            .expect("four directions");
        assert!(
            room(dir) > 12.0,
            "the arena has only {} m to cross",
            room(dir)
        );

        let mut rng = Rng::new(5);
        let mut near = Enemy::spawn(Kind::Grunt, at, 1, &mut rng);
        let mut far = Enemy::spawn(Kind::Grunt, at, 1, &mut rng);
        near.pos = at + dir * 3.0;
        far.pos = at + dir * 8.0;
        // Far first on purpose: the lane is ordered by distance, not by the
        // order the enemy table happens to be kept in.
        let enemies = vec![far, near];

        let lane = hitscan_through(&level, &enemies, eye, dir, 200.0);
        let bodies: Vec<usize> = lane
            .steps
            .iter()
            .filter_map(|s| match s {
                Step::Body(b) => Some(b.index),
                Step::Cover { .. } => None,
            })
            .collect();
        assert_eq!(bodies, vec![1, 0], "the lane is not ordered by distance");
        assert!(
            lane.steps.windows(2).all(|w| w[0].dist() <= w[1].dist()),
            "the lane mixes bodies and cover out of distance order"
        );
        assert!(
            lane.far.dist >= lane.steps.last().map_or(0.0, |s| s.dist()),
            "the lane ends before its own last step"
        );

        // Straight down: the floor, as a span with a thickness — and more of it
        // than a hardened core can cross.
        let down = hitscan_through(&level, &[], eye, Vec3::NEG_Y, 200.0);
        match down.steps.first() {
            Some(Step::Cover {
                dist, thickness, ..
            }) => {
                assert!(*dist > 0.5, "the floor is {dist} m below the eye");
                assert!(
                    *thickness > AmmoKind::Ap.penetration().wall_budget,
                    "the floor reads as {thickness} m thick, which a core crosses"
                );
            }
            other => panic!("straight down met {other:?} rather than the floor"),
        }
    }
}
