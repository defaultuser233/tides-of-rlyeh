use crate::level::{Aabb, Level, STEP_HEIGHT, move_and_collide};
use crate::mesh::{MeshBuilder, contact_shading, tint_in_place, transform_in_place};
use crate::player::Player;
use crate::util::{Rng, approach, yaw_of};
use macroquad::prelude::*;
use std::f32::consts::{FRAC_PI_2, PI, TAU};

const GRAVITY: f32 = 23.0;

/// How close to the last known position counts as having searched it. A unit
/// that has lost sight of the player walks back to where it last saw them and
/// then holds; this is the radius that counts as "there".
const SEARCH_REACH: f32 = 1.5;

/// Ground covered per full two-step cycle, in metres. Dividing a unit's speed
/// by this is what ties the legs to how fast it is actually travelling, and
/// about doubles the time a step takes against what the eye expects at that
/// speed — which is what stops a low-poly gait reading as a comedy march.
const GAIT_STRIDE: f32 = 4.4;

/// Cycle rate at which a walk stride reaches full amplitude, in cycles per
/// second. Slower than this and the limb angles scale down with it, so a unit
/// easing to a halt takes shortening steps rather than full ones in slow
/// motion.
const GAIT_FULL_RATE: f32 = 0.5;

/// How hard the cycle rate chases the speed, in cycles per second per second.
const GAIT_ACCEL: f32 = 2.4;

/// Rotor rate for the airborne archetypes, cycles per second. A hovering
/// machine spins its rotors whether it is translating or not, so this one does
/// not scale with speed the way a walk does.
const HOVER_RATE: f32 = 1.9;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    /// Balanced ranged infantry.
    Grunt,
    /// Fast melee charger.
    Rusher,
    /// Slow, heavily armoured, fires three-round bursts.
    Heavy,
    /// Long-range marksman that telegraphs its shot before firing.
    Sniper,
    /// Hovering flying skirmisher.
    Drone,
    /// Cosmic horror. Lumbers forward and lashes with tentacles from beyond
    /// arm's length; spends its last breath birthing spawnlings.
    Tentacle,
    /// Cosmic horror. A walking mountain: slow, enormous, and all but
    /// immune to small arms.
    Colossus,
    /// Cosmic horror. Tiny, mindless and fast; exists to swarm.
    Spawnling,
}

pub struct KindStats {
    pub health: f32,
    pub speed: f32,
    pub radius: f32,
    pub height: f32,
    pub damage: f32,
    pub interval: f32,
    pub range: f32,
    /// Distance the AI tries to hold from the player.
    pub standoff: f32,
    /// Melee reach. `0.0` means this archetype is a shooter.
    pub reach: f32,
    /// Sanity lost per second while the player has line of sight to one of these.
    pub sanity_drain: f32,
    /// How many `Spawnling`s burst out of the corpse (body horror).
    pub spawns_on_death: u32,
    pub score: u32,
    pub body: Color,
    pub trim: Color,
    pub visor: Color,
    pub flying: bool,
    pub hover: f32,
    pub burst: u32,
    /// Seconds of aiming before the shot lands. Telegraphs snipers.
    pub windup: f32,
    /// Multiplies accuracy spread; lower is more accurate.
    pub accuracy: f32,
}

pub fn stats(kind: Kind) -> KindStats {
    match kind {
        Kind::Grunt => KindStats {
            health: 62.0,
            speed: 2.9,
            radius: 0.42,
            height: 1.78,
            damage: 8.0,
            interval: 1.05,
            range: 24.0,
            standoff: 12.0,
            reach: 0.0,
            sanity_drain: 0.0,
            spawns_on_death: 0,
            score: 100,
            body: Color::new(0.26, 0.46, 0.32, 1.0),
            trim: Color::new(0.16, 0.30, 0.22, 1.0),
            visor: Color::new(1.0, 0.42, 0.16, 1.0),
            flying: false,
            hover: 0.0,
            burst: 0,
            windup: 0.0,
            accuracy: 1.0,
        },
        Kind::Rusher => KindStats {
            health: 44.0,
            speed: 5.6,
            radius: 0.36,
            height: 1.62,
            damage: 13.0,
            interval: 0.85,
            range: 2.0,
            standoff: 0.0,
            // Exactly its old `radius + 0.85` melee test, now data.
            reach: 0.36 + 0.85,
            sanity_drain: 0.0,
            spawns_on_death: 0,
            score: 150,
            body: Color::new(0.58, 0.20, 0.16, 1.0),
            trim: Color::new(0.36, 0.12, 0.10, 1.0),
            visor: Color::new(1.0, 0.85, 0.25, 1.0),
            flying: false,
            hover: 0.0,
            burst: 0,
            windup: 0.0,
            accuracy: 1.0,
        },
        Kind::Heavy => KindStats {
            health: 185.0,
            speed: 1.85,
            radius: 0.60,
            height: 2.30,
            damage: 15.0,
            interval: 1.75,
            range: 26.0,
            standoff: 14.0,
            reach: 0.0,
            sanity_drain: 0.0,
            spawns_on_death: 0,
            score: 320,
            body: Color::new(0.33, 0.27, 0.52, 1.0),
            trim: Color::new(0.20, 0.16, 0.33, 1.0),
            visor: Color::new(1.0, 0.38, 0.92, 1.0),
            flying: false,
            hover: 0.0,
            burst: 3,
            windup: 0.0,
            accuracy: 1.1,
        },
        Kind::Sniper => KindStats {
            health: 55.0,
            speed: 2.2,
            radius: 0.40,
            height: 1.80,
            damage: 34.0,
            interval: 2.40,
            range: 55.0,
            standoff: 24.0,
            reach: 0.0,
            // The scope glint is bad for morale.
            sanity_drain: 1.0,
            spawns_on_death: 0,
            score: 260,
            body: Color::new(0.44, 0.40, 0.22, 1.0),
            trim: Color::new(0.28, 0.25, 0.14, 1.0),
            visor: Color::new(0.35, 1.0, 0.55, 1.0),
            flying: false,
            hover: 0.0,
            burst: 0,
            windup: 0.95,
            accuracy: 0.35,
        },
        Kind::Drone => KindStats {
            health: 34.0,
            speed: 4.2,
            radius: 0.38,
            // Matches the model: `write_drone` lifts its geometry so it occupies
            // [0, 0.97r] on the origin, and the box used to be 0.70 tall, which
            // left a third of a metre of empty air above the drone that counted
            // as head. Keep this in step with the mesh offset.
            height: 0.37,
            damage: 6.0,
            interval: 0.50,
            range: 16.0,
            standoff: 7.0,
            reach: 0.0,
            sanity_drain: 0.0,
            spawns_on_death: 0,
            score: 180,
            body: Color::new(0.30, 0.32, 0.38, 1.0),
            trim: Color::new(0.18, 0.19, 0.24, 1.0),
            visor: Color::new(0.30, 0.85, 1.0, 1.0),
            flying: true,
            hover: 2.90,
            burst: 0,
            windup: 0.0,
            accuracy: 1.3,
        },
        Kind::Tentacle => KindStats {
            health: 95.0,
            speed: 3.6,
            radius: 0.55,
            height: 2.00,
            damage: 16.0,
            interval: 1.10,
            range: 4.0,
            // Holds at the edge of its reach and lashes rather than pressing in.
            standoff: 3.0,
            reach: 3.20,
            sanity_drain: 3.0,
            spawns_on_death: 3,
            score: 400,
            body: Color::new(0.20, 0.34, 0.22, 1.0),
            trim: Color::new(0.30, 0.16, 0.34, 1.0),
            visor: Color::new(0.55, 1.0, 0.32, 1.0),
            flying: false,
            hover: 0.0,
            burst: 0,
            windup: 0.0,
            accuracy: 1.0,
        },
        Kind::Colossus => KindStats {
            health: 900.0,
            speed: 1.15,
            // 1.70 m of radius is 3.4 m of body, and R'LYEH's tightest channels
            // are 2.4 m: it wedged itself into the geometry and stayed there,
            // which also meant the mission could never be completed because a
            // stuck enemy still counts as alive.
            radius: 1.05,
            height: 5.50,
            damage: 42.0,
            interval: 2.60,
            range: 4.5,
            standoff: 0.0,
            reach: 3.60,
            sanity_drain: 14.0,
            spawns_on_death: 0,
            score: 2500,
            body: Color::new(0.60, 0.58, 0.50, 1.0),
            trim: Color::new(0.24, 0.16, 0.30, 1.0),
            visor: Color::new(0.78, 0.62, 1.0, 1.0),
            flying: false,
            hover: 0.0,
            burst: 0,
            windup: 0.0,
            accuracy: 1.0,
        },
        Kind::Spawnling => KindStats {
            health: 18.0,
            speed: 6.4,
            radius: 0.26,
            height: 0.72,
            damage: 6.0,
            interval: 0.45,
            range: 2.0,
            standoff: 0.0,
            reach: 0.90,
            sanity_drain: 0.0,
            spawns_on_death: 0,
            score: 25,
            body: Color::new(0.40, 0.14, 0.26, 1.0),
            trim: Color::new(0.22, 0.08, 0.16, 1.0),
            visor: Color::new(0.90, 1.0, 0.35, 1.0),
            flying: false,
            hover: 0.0,
            burst: 0,
            windup: 0.0,
            accuracy: 1.0,
        },
    }
}

/// A pivot and the rotation hanging off it: where a hip, a knee or a shoulder
/// is this frame.
#[derive(Clone, Copy)]
struct Joint {
    pivot: Vec3,
    rot: Mat4,
}

impl Joint {
    /// A joint that does not move. Everything that is not a limb — and every
    /// archetype without a rig at all — is written through one of these.
    const ROOT: Self = Self {
        pivot: Vec3::ZERO,
        rot: Mat4::IDENTITY,
    };

    fn new(pivot: Vec3, rot: Mat4) -> Self {
        Self { pivot, rot }
    }

    /// Where a point fixed to this joint ends up.
    fn point(&self, p: Vec3) -> Vec3 {
        self.pivot + self.rot.transform_vector3(p - self.pivot)
    }

    /// This joint carried along by `parent`, which is how a knee ends up at
    /// the bottom of a thigh that is swinging.
    fn under(&self, parent: &Joint) -> Self {
        Self {
            pivot: parent.point(self.pivot),
            rot: parent.rot * self.rot,
        }
    }
}

/// Emits a box whose eight corners are swung about `joint` and then carried
/// through `body`.
///
/// `MeshBuilder::add_box` can only place axis-aligned boxes, and the whole of a
/// walk is limbs rotating about a hip or a shoulder. `quad` takes a face's
/// normal from its winding, `(b - a) × (c - a)`, so the corner order below is
/// `add_box`'s with the rotation applied to all eight corners: wind a face the
/// other way and it lights as though it pointed into the limb, which is the
/// difference between a leg and a hole in the body.
fn rig_box(mb: &mut MeshBuilder, centre: Vec3, half: Vec3, joint: Joint, body: Mat4, color: Color) {
    let (x0, y0, z0) = (centre.x - half.x, centre.y - half.y, centre.z - half.z);
    let (x1, y1, z1) = (centre.x + half.x, centre.y + half.y, centre.z + half.z);
    let at = |x: f32, y: f32, z: f32| body.transform_point3(joint.point(vec3(x, y, z)));
    mb.quad(
        at(x0, y1, z0),
        at(x0, y1, z1),
        at(x1, y1, z1),
        at(x1, y1, z0),
        color,
    );
    mb.quad(
        at(x0, y0, z1),
        at(x0, y0, z0),
        at(x1, y0, z0),
        at(x1, y0, z1),
        color,
    );
    mb.quad(
        at(x0, y0, z1),
        at(x1, y0, z1),
        at(x1, y1, z1),
        at(x0, y1, z1),
        color,
    );
    mb.quad(
        at(x1, y0, z0),
        at(x0, y0, z0),
        at(x0, y1, z0),
        at(x1, y1, z0),
        color,
    );
    mb.quad(
        at(x1, y0, z1),
        at(x1, y0, z0),
        at(x1, y1, z0),
        at(x1, y1, z1),
        color,
    );
    mb.quad(
        at(x0, y0, z0),
        at(x0, y0, z1),
        at(x0, y1, z1),
        at(x0, y1, z0),
        color,
    );
}

/// Peak hip swing of one leg, radians. A third of what a real stride flexes
/// through, and the third is the point: at this poly count the honest amount
/// reads as a pantomime march, and a walk is what the march was supposed to
/// stop looking like.
const LEG_SWING: f32 = 0.26;
/// Knee flexion at mid-swing, radians. The knee only ever bends backwards.
const KNEE_BEND: f32 = 0.40;
/// How much of the hip swing the arms counter-swing with.
const ARM_SWING: f32 = 0.62;
/// Lateral lean of the whole body, radians. Deliberately tiny — the body is
/// not what sells a step.
const BODY_LEAN: f32 = 0.05;
/// Vertical dip at each step, as a fraction of height. Smaller than the feet
/// travel by design: the feet sell the weight, the body only follows it.
const BODY_BOB: f32 = 0.010;
/// Where the body leans about, as a fraction of height. Chest high, so the
/// head sits almost on the pivot and walks an almost straight line while the
/// hips swing under it. A walk pivoting at the ankles reads as a rocking
/// statue, and the head is the tell.
const LEAN_PIVOT: f32 = 0.72;
/// How far the sniper's coat swings off the hips, radians.
const COAT_SWAY: f32 = 0.07;

/// The walk cycle as a rig: the body-wide transform for this frame and the
/// joint angles, so a limb writer asks for a hip instead of repeating the
/// trigonometry.
struct Walk {
    /// Carries the whole body: the lean about the chest and the dip.
    body: Mat4,
    /// Phase in radians; one turn is one two-step cycle.
    phase: f32,
    /// How much of the cycle is being expressed, 0..1. A unit coming to a halt
    /// scales every angle down with its speed, which lands it in the standing
    /// pose instead of freezing it mid-stride.
    amount: f32,
}

impl Walk {
    fn new(phase: f32, amount: f32, h: f32) -> Self {
        // The dip is `sin²`, not `|sin|`. `|sin|` has a corner at the bottom of
        // every step, and a corner in a vertical curve is the difference
        // between a walk that settles and a walk that hitches.
        let dip = phase.sin().powi(2);
        let bob = -BODY_BOB * h * amount * dip;
        let lean = BODY_LEAN * amount * phase.sin();
        let pivot = h * LEAN_PIVOT;
        let body = Mat4::from_translation(vec3(0.0, bob, 0.0))
            * Mat4::from_translation(vec3(0.0, pivot, 0.0))
            * Mat4::from_rotation_z(lean)
            * Mat4::from_translation(vec3(0.0, -pivot, 0.0));
        Self {
            body,
            phase,
            amount,
        }
    }

    /// Phase of one side's leg. The two are half a cycle apart, so one plants
    /// exactly as the other swings.
    fn side_phase(&self, side: f32) -> f32 {
        self.phase + if side < 0.0 { PI } else { 0.0 }
    }

    /// Hip swing of one side, radians. Positive swings the foot forward:
    /// models face -Z, so a positive rotation about X takes the bottom of the
    /// leg that way.
    fn hip(&self, side: f32) -> f32 {
        LEG_SWING * self.amount * self.side_phase(side).sin()
    }

    /// Knee flexion of one side, radians, never negative.
    ///
    /// `(1 + cos)/2` peaks through the middle of the swing, where the foot has
    /// to clear the ground or it drags, and is straight at mid-stance, where
    /// that leg is carrying the body.
    fn knee(&self, side: f32) -> f32 {
        KNEE_BEND * self.amount * (0.5 + 0.5 * self.side_phase(side).cos())
    }

    /// The hip and knee of one leg this frame. `thigh` is the hip-to-knee
    /// length; the knee lands wherever the hip's swing puts it.
    fn leg(&self, hip: Vec3, thigh: f32, side: f32) -> (Joint, Joint) {
        let upper = Joint::new(hip, Mat4::from_rotation_x(self.hip(side)));
        let knee = Joint::new(
            hip + vec3(0.0, -thigh, 0.0),
            Mat4::from_rotation_x(self.hip(side) - self.knee(side)),
        );
        (upper, knee.under(&upper))
    }

    /// The shoulder of one arm. Arms counter-swing: the arm on a side goes
    /// back as that side's leg goes forward.
    fn arm(&self, shoulder: Vec3, side: f32) -> Joint {
        Joint::new(shoulder, Mat4::from_rotation_x(-self.hip(side) * ARM_SWING))
    }
}

/// Low-poly humanoid, rebuilt every frame so the walk cycle reaches the limbs.
/// Feet on `y = 0`, facing -Z, and — with the cycle standing still — the same
/// boxes in the same places as the static prototype this replaced, which is
/// what the silhouettes and the hitboxes were measured against.
///
/// All four humanoid archetypes fill the same `±radius` footprint, because that
/// footprint is their collision box and a silhouette that spilled out of it
/// would be a silhouette you cannot shoot. They are told apart by *how* they
/// fill it: a Grunt is balanced, a Heavy is a slab with its head sunk between
/// the pauldrons, a Rusher is a narrow hunched top over a wide planted base
/// with blades hanging low, and a Sniper is a thin column with a coat flaring
/// off the back and a scope standing on its head. Colour is the last thing to
/// survive distance, so none of it leans on colour.
///
/// Every writer here builds its feet on `y = 0` by construction. The old
/// prototype pass could drop a finished mesh onto the floor after the fact; a
/// mesh rebuilt every frame cannot, because the snap would cancel the dip that
/// is half of what the walk is doing.
fn write_humanoid(dst: &mut Mesh, kind: Kind, s: &KindStats, phase: f32, amount: f32) {
    let mut mb = MeshBuilder::reclaim(dst);
    let (r, h) = (s.radius, s.height);
    let walk = Walk::new(phase, amount, h);
    let body = walk.body;
    // Anything that is not a limb still rides the body's lean and dip.
    let still = |mb: &mut MeshBuilder, centre: Vec3, half: Vec3, color: Color| {
        rig_box(mb, centre, half, Joint::ROOT, body, color);
    };

    match kind {
        Kind::Heavy => {
            // One slab with no gaps in it anywhere: legs that meet in the
            // middle, a skirt at the hips, and pauldrons as deep as the body so
            // the mass reads from the side as well as the front. The head is
            // the smallest of the four and sits *between* the shoulders, and
            // the absence of any opening below it is half of what tells this
            // apart from a Grunt at a distance.
            for side in [-1.0f32, 1.0] {
                let (thigh, shin) = walk.leg(vec3(side * r * 0.46, h * 0.52, 0.0), h * 0.25, side);
                rig_box(
                    &mut mb,
                    vec3(side * r * 0.46, h * 0.395, 0.0),
                    vec3(r * 0.46, h * 0.125, r * 0.62),
                    thigh,
                    body,
                    s.trim,
                );
                rig_box(
                    &mut mb,
                    vec3(side * r * 0.46, h * 0.14, 0.0),
                    vec3(r * 0.46, h * 0.14, r * 0.62),
                    shin,
                    body,
                    s.trim,
                );
            }
            still(
                &mut mb,
                vec3(0.0, h * 0.51, 0.0),
                vec3(r, h * 0.09, r * 0.86),
                s.body,
            );
            still(
                &mut mb,
                vec3(0.0, h * 0.71, 0.0),
                vec3(r * 0.98, h * 0.17, r * 0.78),
                s.body,
            );
            still(
                &mut mb,
                vec3(0.0, h * 0.71, -r * 0.80),
                vec3(r * 0.62, h * 0.09, r * 0.20),
                s.trim,
            );
            // The pauldrons are armour strapped to the shoulders, so they stay
            // with the torso; only what hangs below them swings.
            for side in [-1.0f32, 1.0] {
                still(
                    &mut mb,
                    vec3(side * r * 0.74, h * 0.80, 0.0),
                    vec3(r * 0.26, h * 0.20, r * 0.92),
                    s.trim,
                );
            }
            for side in [-1.0f32, 1.0] {
                let arm = walk.arm(vec3(side * r * 0.77, h * 0.62, r * 0.02), side);
                rig_box(
                    &mut mb,
                    vec3(side * r * 0.77, h * 0.45, r * 0.02),
                    vec3(r * 0.19, h * 0.17, r * 0.32),
                    arm,
                    body,
                    s.body,
                );
            }
            still(
                &mut mb,
                vec3(0.0, h * 0.87, 0.0),
                vec3(r * 0.22, h * 0.07, r * 0.28),
                s.trim,
            );
            still(
                &mut mb,
                vec3(0.0, h * 0.88, -r * 0.31),
                vec3(r * 0.17, h * 0.03, r * 0.05),
                s.visor,
            );
        }
        Kind::Rusher => {
            // Feet planted wide, a narrow top with no pauldrons at all, and the
            // chest stepped forward of the hips — an axis-aligned box cannot
            // lean, so the hunch is two boxes that do not line up.
            for side in [-1.0f32, 1.0] {
                let (thigh, shin) = walk.leg(vec3(side * r * 0.50, h * 0.50, 0.0), h * 0.26, side);
                rig_box(
                    &mut mb,
                    vec3(side * r * 0.50, h * 0.37, 0.0),
                    vec3(r * 0.20, h * 0.13, r * 0.34),
                    thigh,
                    body,
                    s.trim,
                );
                rig_box(
                    &mut mb,
                    vec3(side * r * 0.50, h * 0.15, 0.0),
                    vec3(r * 0.20, h * 0.10, r * 0.34),
                    shin,
                    body,
                    s.trim,
                );
                // The foot is a slab splayed outward from the ankle, welded to
                // the shin: a leg that swings without its foot reads as a peg.
                rig_box(
                    &mut mb,
                    vec3(side * r * 0.62, h * 0.035, -r * 0.025),
                    vec3(r * 0.34, h * 0.035, r * 0.575),
                    shin,
                    body,
                    s.trim,
                );
            }
            still(
                &mut mb,
                vec3(0.0, h * 0.54, -r * 0.10),
                vec3(r * 0.62, h * 0.08, r * 0.40),
                s.body,
            );
            still(
                &mut mb,
                vec3(0.0, h * 0.70, -r * 0.46),
                vec3(r * 0.60, h * 0.10, r * 0.40),
                s.body,
            );
            // A ridge down the back where the others wear armour.
            still(
                &mut mb,
                vec3(0.0, h * 0.68, r * 0.36),
                vec3(r * 0.26, h * 0.16, r * 0.16),
                s.trim,
            );
            // Long arms hanging forward past the knee, ending in a blade. The
            // whole arm swings from the shoulder as one piece, blade included.
            for side in [-1.0f32, 1.0] {
                let arm = walk.arm(vec3(side * r * 0.75, h * 0.74, -r * 0.41), side);
                rig_box(
                    &mut mb,
                    vec3(side * r * 0.75, h * 0.63, -r * 0.41),
                    vec3(r * 0.17, h * 0.11, r * 0.31),
                    arm,
                    body,
                    s.body,
                );
                rig_box(
                    &mut mb,
                    vec3(side * r * 0.75, h * 0.39, -r * 0.56),
                    vec3(r * 0.17, h * 0.17, r * 0.22),
                    arm,
                    body,
                    s.body,
                );
                rig_box(
                    &mut mb,
                    vec3(side * r * 0.78, h * 0.24, -r * 0.75),
                    vec3(r * 0.17, h * 0.10, r * 0.19),
                    arm,
                    body,
                    s.visor,
                );
            }
            still(
                &mut mb,
                vec3(0.0, h * 0.85, -r * 0.50),
                vec3(r * 0.28, h * 0.07, r * 0.30),
                s.trim,
            );
            still(
                &mut mb,
                vec3(0.0, h * 0.85, -r * 0.83),
                vec3(r * 0.22, h * 0.03, r * 0.05),
                s.visor,
            );
        }
        Kind::Sniper => {
            // A column: thin legs, narrow shoulders, and the width all at the
            // back where the coat hangs. The scope on top is the only thing
            // above the head on any archetype.
            for side in [-1.0f32, 1.0] {
                let (thigh, shin) = walk.leg(vec3(side * r * 0.21, h * 0.52, 0.0), h * 0.26, side);
                rig_box(
                    &mut mb,
                    vec3(side * r * 0.21, h * 0.39, 0.0),
                    vec3(r * 0.13, h * 0.13, r * 0.26),
                    thigh,
                    body,
                    s.trim,
                );
                rig_box(
                    &mut mb,
                    vec3(side * r * 0.21, h * 0.135, 0.0),
                    vec3(r * 0.13, h * 0.135, r * 0.26),
                    shin,
                    body,
                    s.trim,
                );
            }
            still(
                &mut mb,
                vec3(0.0, h * 0.66, -r * 0.02),
                vec3(r * 0.50, h * 0.18, r * 0.32),
                s.body,
            );
            // Coat, flaring off the shoulders and down the back.
            still(
                &mut mb,
                vec3(0.0, h * 0.69, r * 0.39),
                vec3(r * 0.62, h * 0.17, r * 0.23),
                s.trim,
            );
            // The skirt of the coat is the one part of the torso that is free
            // to lag, so it swings once a step while the body carries it.
            let tail = Joint::new(
                vec3(0.0, h * 0.56, r * 0.48),
                Mat4::from_rotation_x(COAT_SWAY * amount * (phase * 2.0).sin()),
            );
            rig_box(
                &mut mb,
                vec3(0.0, h * 0.36, r * 0.48),
                vec3(r * 0.88, h * 0.20, r * 0.42),
                tail,
                body,
                s.trim,
            );
            for side in [-1.0f32, 1.0] {
                let arm = walk.arm(vec3(side * r * 0.62, h * 0.68, -r * 0.07), side);
                rig_box(
                    &mut mb,
                    vec3(side * r * 0.62, h * 0.56, -r * 0.07),
                    vec3(r * 0.12, h * 0.12, r * 0.23),
                    arm,
                    body,
                    s.body,
                );
            }
            // The rifle stays with the torso: how the man is walking does not
            // change where the muzzle points.
            still(
                &mut mb,
                vec3(r * 0.25, h * 0.66, -r * 0.40),
                vec3(r * 0.15, h * 0.04, r * 0.60),
                s.trim,
            );
            still(
                &mut mb,
                vec3(r * 0.25, h * 0.74, -r * 0.45),
                vec3(r * 0.09, h * 0.04, r * 0.15),
                s.visor,
            );
            still(
                &mut mb,
                vec3(0.0, h * 0.90, -r * 0.05),
                vec3(r * 0.24, h * 0.06, r * 0.25),
                s.trim,
            );
            still(
                &mut mb,
                vec3(0.0, h * 0.905, -r * 0.33),
                vec3(r * 0.16, h * 0.025, r * 0.05),
                s.visor,
            );
            // Aerial, standing on the crown.
            still(
                &mut mb,
                vec3(0.0, h * 0.97, 0.0),
                vec3(r * 0.07, h * 0.03, r * 0.10),
                s.trim,
            );
        }
        _ => {
            // The Grunt is the reference figure, and the one every player reads
            // first, so it is the only one of the four with daylight in it: a
            // neck between the shoulders and the head, a gap under each arm,
            // and the usual space between the legs.
            for side in [-1.0f32, 1.0] {
                let (thigh, shin) = walk.leg(vec3(side * r * 0.33, h * 0.44, 0.0), h * 0.21, side);
                rig_box(
                    &mut mb,
                    vec3(side * r * 0.33, h * 0.33, 0.0),
                    vec3(r * 0.19, h * 0.11, r * 0.34),
                    thigh,
                    body,
                    s.trim,
                );
                rig_box(
                    &mut mb,
                    vec3(side * r * 0.33, h * 0.12, 0.0),
                    vec3(r * 0.19, h * 0.12, r * 0.34),
                    shin,
                    body,
                    s.trim,
                );
            }
            still(
                &mut mb,
                vec3(0.0, h * 0.56, 0.0),
                vec3(r * 0.56, h * 0.16, r * 0.44),
                s.body,
            );
            still(
                &mut mb,
                vec3(0.0, h * 0.71, 0.0),
                vec3(r * 0.80, h * 0.07, r * 0.40),
                s.trim,
            );
            for side in [-1.0f32, 1.0] {
                let arm = walk.arm(vec3(side * r * 0.85, h * 0.64, r * 0.02), side);
                rig_box(
                    &mut mb,
                    vec3(side * r * 0.85, h * 0.49, r * 0.02),
                    vec3(r * 0.15, h * 0.15, r * 0.22),
                    arm,
                    body,
                    s.body,
                );
            }
            still(
                &mut mb,
                vec3(0.0, h * 0.80, 0.0),
                vec3(r * 0.13, h * 0.04, r * 0.20),
                s.trim,
            );
            still(
                &mut mb,
                vec3(0.0, h * 0.91, 0.0),
                vec3(r * 0.34, h * 0.09, r * 0.34),
                s.trim,
            );
            still(
                &mut mb,
                vec3(0.0, h * 0.915, -r * 0.36),
                vec3(r * 0.29, h * 0.035, r * 0.06),
                s.visor,
            );
            still(
                &mut mb,
                vec3(0.0, h * 0.53, -r * 0.45),
                vec3(r * 0.14, h * 0.07, r * 0.05),
                s.visor,
            );
        }
    }
    *dst = mb.finish();
}

/// A quad-rotor body: central chassis, four arms, a glowing sensor eye, and
/// rotor pads that are always turning — a hover is not a gait, so the rotors
/// do not wait for the airframe to start translating.
fn write_drone(dst: &mut Mesh, s: &KindStats, phase: f32) {
    let mut mb = MeshBuilder::reclaim(dst);
    let r = s.radius;
    let spin = phase * TAU;
    // A hover wobble, out of phase with the rotor tilt below, and applied to
    // everything the machine is made of: a drone holding station perfectly
    // still reads as a prop on a stick.
    let body = Mat4::from_translation(vec3(0.0, (spin * 0.5).sin() * r * 0.07, 0.0))
        * Mat4::from_rotation_z((spin * 0.37).sin() * 0.06)
        // A quad-rotor reads better modelled around its centre of mass, which
        // is how the boxes below are laid out — but `body_aabb`, `head_y` and
        // `eye` all measure upward from `pos.y`, the way every other
        // archetype's model stands on it. Left as it was, two thirds of the
        // drone hung below its own hitbox (so shooting the thing missed), the
        // hitbox was mostly empty air, and every shot that did land did so
        // above `head_y`. Lifting the model by the depth of its under-slung
        // gun, `0.62r`, puts its lowest vertex on `pos.y` and the whole model
        // inside `[0, height]`.
        * Mat4::from_translation(vec3(0.0, r * 0.62, 0.0));
    let at = |mb: &mut MeshBuilder, centre: Vec3, half: Vec3, joint: Joint, color: Color| {
        rig_box(mb, centre, half, joint, body, color);
    };

    at(
        &mut mb,
        vec3(0.0, r * 0.025, 0.0),
        vec3(r * 0.55, r * 0.325, r * 0.60),
        Joint::ROOT,
        s.body,
    );
    // sensor eye, angled forward
    at(
        &mut mb,
        vec3(0.0, 0.0, -r * 0.68),
        vec3(r * 0.30, r * 0.16, r * 0.10),
        Joint::ROOT,
        s.visor,
    );
    // arms + rotor pads
    for (i, (sx, sz)) in [(1.0f32, 1.0f32), (1.0, -1.0), (-1.0, 1.0), (-1.0, -1.0)]
        .into_iter()
        .enumerate()
    {
        at(
            &mut mb,
            vec3(sx * 0.75, 0.0, sz * 0.75),
            vec3(r * 0.40, r * 0.08, r * 0.40),
            Joint::ROOT,
            s.trim,
        );
        let (cx, cz) = (sx * 1.15, sz * 1.15);
        // A flat pad turning about its own axis is invisible from the side,
        // which is where the player sees it, so each rotor also rocks — the
        // diagonal pairs against each other. The rock is what carries the
        // motion at range; the spin keeps the corners moving up close.
        let tilt = (spin * 0.5).sin() * if i % 2 == 0 { 0.06 } else { -0.06 };
        let hub = Joint::new(
            vec3(cx, r * 0.06, cz),
            Mat4::from_rotation_x(tilt)
                * Mat4::from_rotation_y(spin * if i % 2 == 0 { 1.0 } else { -1.0 }),
        );
        at(
            &mut mb,
            vec3(cx, r * 0.06, cz),
            vec3(r * 0.42, r * 0.04, r * 0.42),
            hub,
            s.visor,
        );
    }
    // under-slung gun
    at(
        &mut mb,
        vec3(0.0, -r * 0.45, 0.0),
        vec3(r * 0.12, r * 0.17, r * 0.30),
        Joint::ROOT,
        s.trim,
    );
    *dst = mb.finish();
}

/// Axis-aligned box from a centre and half-extents. The cosmic archetypes are
/// laid out along rays, where a centre reads more clearly than a min/max pair.
fn box_at(mb: &mut MeshBuilder, centre: Vec3, half: Vec3, color: Color) {
    mb.add_box(centre - half, centre + half, color);
}

/// Shifts a mesh so its lowest vertex sits on `y = 0`, which is the convention
/// every collision box assumes.
///
/// Only the crawling archetypes want this per frame. A walker's legs are built
/// onto the floor, and dropping one every frame would cancel the dip that is
/// half of what the walk is doing.
fn drop_to_floor(mesh: &mut Mesh) {
    let mut lowest = f32::MAX;
    for v in &mesh.vertices {
        lowest = lowest.min(v.position.y);
    }
    if lowest.is_finite() && lowest.abs() > 1e-4 {
        for v in &mut mesh.vertices {
            v.position.y -= lowest;
        }
    }
}

/// Squat bulb sitting on a ring of tapering tentacles, eyes clustered on the
/// front, rebuilt every frame so the rays can crawl. Each arm reaches a beat
/// after its neighbour: a ring bending in unison reads as a shutter closing,
/// and what this has to read as is a thing walking on eight legs.
fn write_tentacle(dst: &mut Mesh, s: &KindStats, phase: f32, amount: f32) {
    let mut mb = MeshBuilder::reclaim(dst);
    let (r, h) = (s.radius, s.height);
    // The ray roots sit on the bulb's widest ring.
    let root_y = h * 0.34;

    // Eight rays, each four ever-smaller boxes drooping toward the ground. The
    // bend accumulates down a ray, so the tip of an arm swings widest while
    // the root barely moves — the same "pivot high, amplitude below it" trick
    // the walk cycle plays on the legs.
    for i in 0..8 {
        let a = i as f32 / 8.0 * TAU;
        let (dx, dz) = (a.sin(), a.cos());
        // Horizontal and square to the ray: bending about it curls the arm up
        // and down without twisting it sideways.
        let hinge = vec3(-dz, 0.0, dx);
        let root = vec3(dx * r * 1.06, root_y, dz * r * 1.06);
        let mut bend = 0.0;
        for j in 0..4 {
            let step = j as f32;
            bend += amount * 0.05 * (phase + i as f32 * 0.7 + step * 0.9).sin();
            let out = r * (1.06 + 0.52 * step);
            let half = r * (0.26 - 0.055 * step);
            rig_box(
                &mut mb,
                vec3(dx * out, h * (0.34 - 0.068 * step), dz * out),
                vec3(half, half * 0.9, half),
                Joint::new(root, Mat4::from_axis_angle(hinge, bend)),
                Mat4::IDENTITY,
                if j % 2 == 0 { s.trim } else { s.body },
            );
        }
    }

    // Bulb: widest above the tentacle roots, tapering to a wet crown.
    box_at(
        &mut mb,
        vec3(0.0, h * 0.34, 0.0),
        vec3(r * 0.80, h * 0.20, r * 0.80),
        s.body,
    );
    box_at(
        &mut mb,
        vec3(0.0, h * 0.56, 0.0),
        vec3(r * 0.95, h * 0.16, r * 0.92),
        s.body,
    );
    box_at(
        &mut mb,
        vec3(0.0, h * 0.74, 0.0),
        vec3(r * 0.72, h * 0.12, r * 0.70),
        s.trim,
    );
    box_at(
        &mut mb,
        vec3(0.0, h * 0.88, 0.0),
        vec3(r * 0.46, h * 0.10, r * 0.44),
        s.body,
    );

    // The eyes, wet and luminous, pushed out through the front of the bulb.
    for (ex, ey, ez, eh) in [
        (0.0f32, 0.58f32, 0.86f32, 0.20f32),
        (-0.40, 0.50, 0.80, 0.14),
        (0.40, 0.50, 0.80, 0.14),
        (-0.30, 0.76, 0.66, 0.13),
        (0.30, 0.76, 0.66, 0.13),
        (0.0, 0.89, 0.42, 0.12),
    ] {
        box_at(
            &mut mb,
            vec3(ex * r, h * ey, -ez * r),
            vec3(r * eh, r * eh, r * 0.20),
            s.visor,
        );
    }
    *dst = mb.finish();
    // The arms reach below the origin when they curl down, so the creature is
    // dropped back onto the floor — which leaves it riding whichever arms are
    // pushing, the way a thing on legs ought to.
    drop_to_floor(dst);
}

/// Towering walker, rebuilt every frame so `phase` can drive a slow, heavy
/// stride: the legs swing half a cycle apart, the frame lurches, and the crown
/// of tentacles writhes. `dst` is both the output and the scratch.
fn write_colossus(dst: &mut Mesh, s: &KindStats, phase: f32) {
    let mut mb = MeshBuilder::reclaim(dst);
    let (r, h) = (s.radius, s.height);
    let stride = phase * 1.7;

    for (side, lag) in [(1.0f32, 0.0f32), (-1.0f32, PI)] {
        let cyc = (stride + lag).sin();
        let lift = cyc.max(0.0) * h * 0.05;
        let fwd = -cyc * r * 0.20;
        box_at(
            &mut mb,
            vec3(side * r * 0.50, h * 0.21 + lift, fwd),
            vec3(r * 0.30, h * 0.21, r * 0.34),
            s.trim,
        );
        box_at(
            &mut mb,
            vec3(side * r * 0.52, h * 0.05 + lift, fwd - r * 0.10),
            vec3(r * 0.36, h * 0.05, r * 0.48),
            s.trim,
        );
    }

    let bob = (stride * 2.0).sin().abs() * h * 0.012;
    let sway = stride.sin() * r * 0.05;

    box_at(
        &mut mb,
        vec3(sway, h * 0.60 + bob, 0.0),
        vec3(r * 0.92, h * 0.21, r * 0.62),
        s.body,
    );
    box_at(
        &mut mb,
        vec3(sway, h * 0.48 + bob, -r * 0.30),
        vec3(r * 0.78, h * 0.10, r * 0.36),
        s.trim,
    );
    box_at(
        &mut mb,
        vec3(sway, h * 0.75 + bob, 0.0),
        vec3(r * 1.24, h * 0.09, r * 0.55),
        s.trim,
    );

    // Arms counter-swing against the legs.
    for (side, lag) in [(1.0f32, PI), (-1.0f32, 0.0)] {
        let cyc = (stride + lag).sin();
        box_at(
            &mut mb,
            vec3(side * r * 1.14 + sway, h * 0.50 + bob, -cyc * r * 0.16),
            vec3(r * 0.28, h * 0.26, r * 0.30),
            s.body,
        );
    }

    // A head, then a crown of tentacles where a face should be.
    let head_y = h * 0.86 + bob;
    box_at(
        &mut mb,
        vec3(sway, head_y, 0.0),
        vec3(r * 0.44, h * 0.06, r * 0.42),
        s.body,
    );
    for i in 0..7 {
        let a = i as f32 / 7.0 * TAU + phase * 0.35;
        let (dx, dz) = (a.sin(), a.cos());
        let curl = (phase * 1.9 + i as f32 * 1.3).sin() * h * 0.015;
        for j in 0..3 {
            let step = j as f32;
            let spread = r * (0.30 + 0.22 * step);
            let half = r * (0.13 - 0.032 * step);
            box_at(
                &mut mb,
                vec3(
                    sway + dx * spread,
                    head_y + h * (0.045 + 0.042 * step) + curl,
                    dz * spread,
                ),
                vec3(half, h * 0.045, half),
                if j == 1 { s.trim } else { s.body },
            );
        }
    }

    for (ex, ey, ez, eh) in [
        (0.0f32, 0.87f32, 0.42f32, 0.10f32),
        (-0.30, 0.84, 0.38, 0.07),
        (0.30, 0.84, 0.38, 0.07),
        (-0.16, 0.92, 0.30, 0.055),
        (0.16, 0.92, 0.30, 0.055),
    ] {
        box_at(
            &mut mb,
            vec3(ex * r + sway, h * ey + bob, -ez * r),
            vec3(r * eh, r * eh, r * 0.12),
            s.visor,
        );
    }
    *dst = mb.finish();
}

/// Low and wide, many-legged, with one oversized glowing eye. Rebuilt every
/// frame so the legs can scuttle: the wave runs down the body a beat per pair,
/// which at this size is most of what separates the thing from a crawling
/// rock.
fn write_spawnling(dst: &mut Mesh, s: &KindStats, phase: f32, amount: f32) {
    let mut mb = MeshBuilder::reclaim(dst);
    let (r, h) = (s.radius, s.height);
    // The carapace rocks on its legs rather than being carried flat.
    let rock = Mat4::from_translation(vec3(0.0, h * 0.30, 0.0))
        * Mat4::from_rotation_z(amount * 0.07 * phase.sin())
        * Mat4::from_translation(vec3(0.0, -h * 0.30, 0.0));
    let still = |mb: &mut MeshBuilder, centre: Vec3, half: Vec3, color: Color| {
        rig_box(mb, centre, half, Joint::ROOT, rock, color);
    };

    // Flat carapace hunched over three pairs of splayed legs.
    still(
        &mut mb,
        vec3(0.0, h * 0.46, 0.0),
        vec3(r * 0.90, h * 0.24, r * 1.10),
        s.body,
    );
    still(
        &mut mb,
        vec3(0.0, h * 0.72, -r * 0.10),
        vec3(r * 0.62, h * 0.16, r * 0.70),
        s.trim,
    );
    for side in [-1.0f32, 1.0] {
        for i in 0..3 {
            let z = (i as f32 - 1.0) * r * 0.62;
            // Legs step in a wave from front to back, and the two sides
            // alternate: a pair that lifted together would hop, not crawl.
            let p = phase + i as f32 * 1.9 + if side > 0.0 { 0.0 } else { PI };
            let lift = (0.5 + 0.5 * p.sin()) * amount * 0.20;
            let swing = p.cos() * amount * 0.13;
            let hip = Joint::new(
                vec3(side * r * 0.62, h * 0.24, z),
                Mat4::from_rotation_y(side * swing) * Mat4::from_rotation_z(side * lift),
            );
            // The foot hangs off the far end of the first segment, so it is
            // carried by the hip's rotation rather than animated separately.
            let foot = Joint::new(vec3(side * r * 1.70, h * 0.30, z), Mat4::IDENTITY).under(&hip);
            rig_box(
                &mut mb,
                vec3(side * r * 1.15, h * 0.24, z),
                vec3(r * 0.55, h * 0.10, r * 0.09),
                hip,
                rock,
                s.trim,
            );
            rig_box(
                &mut mb,
                vec3(side * r * 1.70, h * 0.16, z),
                vec3(r * 0.12, h * 0.14, r * 0.09),
                foot,
                rock,
                s.trim,
            );
        }
    }
    for i in 0..4 {
        still(
            &mut mb,
            vec3(0.0, h * 0.86, (i as f32 - 1.5) * r * 0.44),
            vec3(r * 0.12, h * 0.12, r * 0.12),
            s.trim,
        );
    }
    still(
        &mut mb,
        vec3(0.0, h * 0.52, -r * 1.00),
        vec3(r * 0.48, h * 0.20, r * 0.18),
        s.visor,
    );
    *dst = mb.finish();
    // The lift is one-signed, so a leg never pushes below the floor, but the
    // rock can dip a corner of the stance. The floor is enforced rather than
    // assumed.
    drop_to_floor(dst);
}

pub enum Attack {
    Shot {
        origin: Vec3,
        dir: Vec3,
        damage: f32,
    },
    Melee {
        damage: f32,
    },
}

pub struct Enemy {
    pub pos: Vec3,
    pub vel: Vec3,
    pub yaw: f32,
    pub kind: Kind,
    pub health: f32,
    pub radius: f32,
    pub height: f32,
    pub speed: f32,
    pub damage: f32,
    pub interval: f32,
    pub range: f32,
    pub standoff: f32,
    /// Melee reach, mirrored from the archetype. `0.0` means this one shoots.
    pub reach: f32,
    /// Sanity per second this archetype costs the player while visible.
    pub sanity_drain: f32,
    /// Spawnlings released when this enemy dies.
    pub spawns_on_death: u32,
    pub score: u32,
    pub cooldown: f32,
    pub flash: f32,
    /// Ramps to 1 while the player is visible and in range; drives the visor glow
    /// and, for snipers, the shot telegraph.
    pub alert: f32,
    pub aim_t: f32,
    /// Where the player was last seen, and how long ago that was.
    pub last_seen: Vec3,
    /// True once this unit has actually laid eyes on the player. One that was
    /// only *told* where the player was — a reinforcement, via `alert_to` — has
    /// a memory of a place it never saw anyone at, and follows the flow field to
    /// the player instead of searching that spot.
    pub seen_player: bool,
    pub aware_t: f32,
    pub dead: bool,
    pub death_t: f32,
    pub spawned_t: f32,
    pub flying: bool,
    hover: f32,
    strafe_dir: f32,
    strafe_t: f32,
    burst_left: u32,
    /// Walk-cycle phase in radians and the rate it is turning at, in cycles per
    /// second. Both live in the struct because `write_mesh` only has `&self`:
    /// the pose is state a frame of movement leaves behind, not something the
    /// draw call can work out for itself.
    gait: f32,
    gait_rate: f32,
    /// Half-cycle counter. A step is the crossing of a phase boundary, and this
    /// is how that crossing is reported exactly once instead of on every frame
    /// the phase spends past it.
    step_index: i32,
    /// Set for the frame a foot lands, and consumed by `take_footfall`.
    footfall: bool,
}

impl Enemy {
    pub fn spawn(kind: Kind, pos: Vec3, wave: u32, rng: &mut Rng) -> Self {
        let s = stats(kind);
        let w = (wave.saturating_sub(1)) as f32;
        let health = s.health * (1.0 + 0.13 * w);
        Self {
            pos,
            vel: Vec3::ZERO,
            yaw: rng.range(0.0, TAU),
            kind,
            health,
            radius: s.radius,
            height: s.height,
            speed: s.speed * (1.0 + 0.022 * w),
            damage: s.damage * (1.0 + 0.06 * w),
            interval: (s.interval * (1.0 - 0.015 * w)).max(0.30),
            range: s.range,
            standoff: s.standoff,
            reach: s.reach,
            sanity_drain: s.sanity_drain,
            spawns_on_death: s.spawns_on_death,
            score: s.score,
            cooldown: rng.range(0.3, 1.0),
            flash: 0.0,
            alert: 0.0,
            aim_t: 0.0,
            last_seen: pos,
            seen_player: false,
            aware_t: 99.0,
            dead: false,
            death_t: 0.0,
            spawned_t: 0.0,
            flying: s.flying,
            hover: s.hover,
            strafe_dir: if rng.chance(0.5) { 1.0 } else { -1.0 },
            strafe_t: rng.range(0.5, 1.8),
            burst_left: 0,
            // Phase 0 is the standing pose of every rig — one leg is at
            // mid-swing at that phase, so what it really rests on is the rate
            // being zero, which scales the whole cycle down to nothing.
            gait: 0.0,
            gait_rate: 0.0,
            step_index: 0,
            footfall: false,
        }
    }

    /// Tells this unit where the player was and starts it hunting.
    ///
    /// A freshly spawned enemy has never seen anything (`aware_t` starts at
    /// 99), and `pick_spawn` deliberately places reinforcements out of sight —
    /// so without this they stand at their spawn until the player happens to
    /// walk into view. On an open arena that resolves itself; inside R'LYEH's
    /// labyrinth it never does, and seven hostiles sat 20 m away for a full
    /// minute doing nothing. `pursue` is how many seconds they keep coming
    /// before giving up and returning to an idle state, so breaking contact is
    /// still a real defence.
    ///
    /// A unit that has not laid eyes on the player yet has no memory to search,
    /// only a place to go, so it keeps walking the flow field toward wherever
    /// the player is now — see `search_dir` for what happens once it has.
    pub fn alert_to(&mut self, pos: Vec3, pursue: f32) {
        self.last_seen = pos;
        self.aware_t = -pursue;
    }

    pub fn targetable(&self) -> bool {
        !self.dead
    }

    pub fn body_aabb(&self) -> Aabb {
        Aabb::new(
            vec3(
                self.pos.x - self.radius,
                self.pos.y,
                self.pos.z - self.radius,
            ),
            vec3(
                self.pos.x + self.radius,
                self.pos.y + self.height,
                self.pos.z + self.radius,
            ),
        )
    }

    /// Height above the enemy's feet that separates a body shot from a head shot.
    pub fn head_y(&self) -> f32 {
        if self.flying {
            // Upper part of a body that is only 0.37 tall and sits on its own
            // origin, so the usual 0.78 of a humanoid's height would be above
            // the top of the model.
            self.pos.y + self.height * 0.72
        } else {
            self.pos.y + self.height * 0.78
        }
    }

    pub fn eye(&self) -> Vec3 {
        if self.flying {
            vec3(self.pos.x, self.pos.y + self.height * 0.5, self.pos.z)
        } else {
            vec3(self.pos.x, self.pos.y + self.height * 0.86, self.pos.z)
        }
    }

    pub fn is_removable(&self) -> bool {
        self.dead && self.death_t > 1.9
    }

    /// Steering for a unit that has lost sight of the player: back to the last
    /// position it saw them at, and nothing at all once it arrives.
    ///
    /// The flow field cannot be trusted on its own here. It is rebuilt four
    /// times a second toward the player's *current* position, so a unit that
    /// has just lost contact follows it straight to wherever the player has
    /// moved to since — through walls, and never stopping to look. The intent
    /// was always "the squad searches your last known position"; the field is
    /// kept as the navigator (it is the only one the level has), but it is used
    /// only when the straight line to the memory is blocked *and* the field
    /// roughly agrees with it. Whenever it points somewhere else, the memory
    /// wins and the unit walks it directly.
    fn search_dir(&self, level: &Level) -> Vec2 {
        let to_memory = vec2(self.last_seen.x - self.pos.x, self.last_seen.z - self.pos.z);
        let d = to_memory.length();
        if d <= SEARCH_REACH {
            return Vec2::ZERO;
        }
        let direct = to_memory / d;
        let blocked = !level.line_of_sight(self.eye(), self.last_seen + vec3(0.0, 1.0, 0.0));
        let flow = level.flow.dir_at(self.pos);
        if blocked && flow.dot(direct) >= 0.25 {
            flow
        } else {
            direct
        }
    }

    pub fn take_damage(&mut self, amount: f32) -> bool {
        if self.dead {
            return false;
        }
        self.health -= amount;
        self.flash = 1.0;
        if self.health <= 0.0 {
            self.health = 0.0;
            self.dead = true;
            self.death_t = 0.0;
            return true;
        }
        false
    }

    /// Whether a foot landed on this frame, reported once per step.
    ///
    /// The walk cycle is a phase, so "the foot is down" is not something a
    /// caller can test for itself without knowing the rig. What a caller wants
    /// is the instant a step lands — for dust, for a sound, for a stagger —
    /// and that instant is a phase boundary. Taking it clears it, so one step
    /// is reported once however many times this is called.
    pub fn take_footfall(&mut self) -> bool {
        std::mem::take(&mut self.footfall)
    }

    /// Advances the walk cycle by one frame of ground contact, leaving whatever
    /// foot landed this frame in `footfall`.
    ///
    /// The phase runs off the ground actually covered, not off the speed that
    /// was asked for: a unit pressed into a wall is not walking, and one that
    /// stopped is not marching on the spot. Everything else falls out of the
    /// same number — the cycle slows, then shortens, then settles.
    fn advance_gait(&mut self, dt: f32, ground_speed: f32) {
        let rate = if self.flying {
            // A hovering machine turns its rotors whether or not it is
            // translating, so its cycle runs off the clock, not the ground.
            HOVER_RATE
        } else {
            ground_speed / GAIT_STRIDE
        };
        self.gait_rate = approach(self.gait_rate, rate, dt * GAIT_ACCEL);
        self.gait = (self.gait + self.gait_rate * TAU * dt).rem_euclid(TAU);

        // A step lands as the phase crosses a half-cycle boundary, the
        // boundary placed half a cycle in so it falls where the swinging leg
        // reaches full extension. Comparing indices rather than watching the
        // phase keeps a long frame from reporting a step twice, and the index
        // runs backwards once per wrap — the phase is periodic, so that is not
        // a crossing, and the comparison is what rules it out.
        let step = ((self.gait - FRAC_PI_2) / PI).floor() as i32;
        let landed = step > self.step_index;
        self.step_index = step;
        // Overwritten every frame, not accumulated: a step is a pulse, and one
        // nobody collected has already been and gone.
        self.footfall = landed && !self.flying;
    }

    /// `neighbours` is every live enemy's position from the start of the frame,
    /// and `self_index` is this one's place in it — passing the slice and
    /// skipping our own entry avoids rebuilding a filtered copy per enemy per
    /// frame.
    pub fn update(
        &mut self,
        level: &Level,
        player: &Player,
        neighbours: &[Vec3],
        self_index: usize,
        dt: f32,
        rng: &mut Rng,
    ) -> Option<Attack> {
        if self.dead {
            self.death_t += dt;
            // Whatever the frame of the killing blow reported is not owed to
            // anyone: a corpse does not take a step.
            self.footfall = false;
            return None;
        }

        self.spawned_t += dt;
        self.flash = approach(self.flash, 0.0, dt * 6.0);
        self.cooldown = (self.cooldown - dt).max(0.0);
        let s = stats(self.kind);

        let to_player = player.pos - self.pos;
        let flat = vec2(to_player.x, to_player.z);
        let dist = flat.length();

        // Detection range depends on how much the player is exposing
        // themselves: standing and sprinting are spotted far sooner than
        // crouched, prone and walking. Long-ranged archetypes notice further.
        let detect = (self.range * 1.2).max(24.0) * player.visibility();
        let los = player.alive && dist < detect && level.line_of_sight(self.eye(), player.eye());
        if los {
            self.last_seen = player.pos;
            self.seen_player = true;
            self.aware_t = 0.0;
        } else {
            self.aware_t += dt;
        }

        self.alert = approach(
            self.alert,
            if los && dist < self.range { 1.0 } else { 0.0 },
            dt * 4.0,
        );

        // --- desired velocity -------------------------------------------------
        // With neither sight nor recent contact the squad holds position. That
        // is what makes crouching, going prone and walking worth doing.
        let aware = player.alive && (los || self.aware_t < 6.0);
        // A unit that has actually had eyes on the player searches the memory of
        // where it last saw them. One that was merely *told* where the player was
        // — a reinforcement, alerted at spawn — has no memory to search, only a
        // place to go, so it keeps walking the flow field toward the player as
        // before: that is what makes it arrive at the fight at all.
        let searching = !los && self.seen_player;
        let mut desired = Vec2::ZERO;
        if aware && self.spawned_t > 0.35 {
            if s.reach > 0.0 {
                // Melee: close the distance. An archetype with a standoff
                // lashes from the edge of its reach instead of pressing in.
                desired = if searching {
                    self.search_dir(level)
                } else if !los {
                    level.flow.dir_at(self.pos)
                } else if self.standoff > 0.0 && dist < self.standoff {
                    -flat.normalize_or_zero() * 0.8
                } else {
                    flat.normalize_or_zero()
                } * self.speed;
            } else if searching {
                // Lost contact: walk back to the last known position and hold
                // there — slower than a charge, and without shooting.
                desired = self.search_dir(level) * self.speed * 0.8;
            } else if !los {
                desired = level.flow.dir_at(self.pos) * self.speed * 0.8;
            } else if dist > self.range * 0.9 || dist > self.standoff * 1.6 {
                desired = level.flow.dir_at(self.pos) * self.speed;
            } else if dist < self.standoff * 0.55 {
                desired = -flat.normalize_or_zero() * self.speed * 0.8;
            } else {
                self.strafe_t -= dt;
                if self.strafe_t <= 0.0 {
                    self.strafe_t = rng.range(0.7, 2.1);
                    if rng.chance(0.45) {
                        self.strafe_dir = -self.strafe_dir;
                    }
                }
                let perp = vec2(-flat.y, flat.x).normalize_or_zero();
                desired = perp * self.strafe_dir * self.speed * 0.7;
            }
        }

        for (j, other) in neighbours.iter().enumerate() {
            if j == self_index {
                continue;
            }
            let d = vec2(self.pos.x - other.x, self.pos.z - other.z);
            let l = d.length();
            let min_d = (self.radius + 0.5) * 2.0;
            if l > 1e-3 && l < min_d {
                desired += d.normalize() * (1.0 - l / min_d) * self.speed * 1.8;
            }
        }
        if desired.length() > self.speed {
            desired = desired.normalize() * self.speed;
        }

        let current = vec2(self.vel.x, self.vel.z);
        let d = desired - current;
        let max_d = 26.0 * dt;
        let new_v = if d.length() > max_d {
            current + d.normalize() * max_d
        } else {
            desired
        };
        self.vel.x = new_v.x;
        self.vel.z = new_v.y;

        // Where the unit is before it moves, so the walk cycle below can be
        // driven by the ground it actually covers rather than by the velocity
        // it wanted: a unit shoved into a wall, or one holding station, is not
        // walking however hard its steering pushes.
        let from = vec2(self.pos.x, self.pos.z);

        if self.flying {
            // Hovering units ignore gravity and float to their cruise altitude.
            self.vel.y = 0.0;
            let cruise = self.hover + (self.spawned_t * 1.7).sin() * 0.18;
            self.pos.y = approach(self.pos.y, cruise, dt * 2.5);
            let (pos, _) = move_and_collide(
                level,
                self.pos,
                self.radius,
                self.height,
                vec3(self.vel.x * dt, 0.0, self.vel.z * dt),
                0.0,
            );
            self.pos.x = pos.x;
            self.pos.z = pos.z;
        } else {
            self.vel.y = (self.vel.y - GRAVITY * dt).max(-60.0);
            let (pos, col) = move_and_collide(
                level,
                self.pos,
                self.radius,
                self.height,
                self.vel * dt,
                STEP_HEIGHT,
            );
            self.pos = pos;
            if col.x {
                self.vel.x = 0.0;
            }
            if col.z {
                self.vel.z = 0.0;
            }
            if col.ground && self.vel.y < 0.0 {
                self.vel.y = 0.0;
            }
        }

        let ground_speed = if dt > 0.0 {
            vec2(self.pos.x - from.x, self.pos.z - from.y).length() / dt
        } else {
            0.0
        };
        self.advance_gait(dt, ground_speed);

        // --- facing -----------------------------------------------------------
        let face = if los && player.alive {
            flat.normalize_or_zero()
        } else {
            vec2(self.vel.x, self.vel.z).normalize_or_zero()
        };
        if face != Vec2::ZERO {
            let target = yaw_of(vec3(face.x, 0.0, face.y));
            self.yaw = angle_toward(self.yaw, target, dt * 7.0);
        }

        // --- attack -----------------------------------------------------------
        if !player.alive || self.spawned_t < 0.5 {
            return None;
        }
        if s.reach > 0.0 {
            // Reach has to be earned. Without the sight test a 3.2 m tentacle
            // claws through a 1 m wall; without the height test a Colossus
            // reaches up through the terrace you are standing on. The old code
            // compared XZ distance only, so cover did nothing at all.
            let same_level = (player.pos.y - self.pos.y).abs() < self.height + 0.5;
            if los && same_level && dist < self.reach && self.cooldown <= 0.0 {
                self.cooldown = self.interval;
                return Some(Attack::Melee {
                    damage: self.damage,
                });
            }
            return None;
        }

        let in_range = los && dist < self.range;
        if !in_range {
            self.aim_t = 0.0;
            return None;
        }

        if s.windup > 0.0 {
            // Telegraph: hold the shot until the aim timer has charged.
            if self.cooldown > 0.0 {
                // Still recovering from the previous shot. Without this return
                // the branch falls through to the firing code below and a
                // marksman fires every single frame, which is exactly what a
                // harness measured: 514 shots in 600 frames.
                self.aim_t = 0.0;
                return None;
            }
            self.aim_t += dt;
            if self.aim_t < s.windup {
                return None;
            }
        } else if self.cooldown > 0.0 {
            return None;
        }

        self.aim_t = 0.0;
        let origin = self.eye();
        let aim = player.eye() + vec3(0.0, -0.22, 0.0);
        let spread = (0.018 + dist * 0.0017).min(0.070) * s.accuracy;
        let dir = rng.cone((aim - origin).normalize_or_zero(), spread);

        if s.burst > 1 {
            if self.burst_left == 0 {
                self.burst_left = s.burst;
            }
            self.burst_left -= 1;
            self.cooldown = if self.burst_left == 0 {
                self.interval
            } else {
                0.15
            };
        } else {
            self.cooldown = self.interval;
        }

        Some(Attack::Shot {
            origin,
            dir,
            damage: self.damage,
        })
    }

    /// Writes this frame's model into `dst`, reusing its buffers.
    ///
    /// `dst` is the caller's scratch: this used to return a freshly built mesh,
    /// which cost two vector allocations per enemy per frame plus whatever the
    /// copy had to grow into. Every archetype is rebuilt here now — a prototype
    /// built once can only be a rigid one — so `reclaim`, not `new`, is what
    /// keeps that free.
    pub fn write_mesh(&self, dst: &mut Mesh) {
        let fall = if self.dead {
            (self.death_t / 0.55).clamp(0.0, 1.0).powf(0.65)
        } else {
            0.0
        };
        let sink = if self.death_t > 1.15 {
            ((self.death_t - 1.15) / 0.75).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let alpha = (1.0 - sink).max(0.0);
        // Flying units spin down and drop rather than toppling over.
        let mat = if self.flying {
            let spin = self.yaw + fall * TAU * 1.5;
            Mat4::from_translation(self.pos + vec3(0.0, -fall * 1.6, 0.0))
                * Mat4::from_rotation_y(spin)
        } else {
            Mat4::from_translation(self.pos + vec3(0.0, -sink * 0.5, 0.0))
                * Mat4::from_rotation_y(self.yaw)
                * Mat4::from_rotation_x(fall * FRAC_PI_2)
        };
        let s = stats(self.kind);
        // How much of the cycle the limbs are expressing. A unit easing to a
        // stop takes shorter steps, not slower ones, and at zero the rig is
        // back in the pose its prototype was built in.
        let amount = (self.gait_rate / GAIT_FULL_RATE).clamp(0.0, 1.0);
        match self.kind {
            Kind::Colossus => write_colossus(dst, &s, self.spawned_t),
            Kind::Grunt | Kind::Rusher | Kind::Heavy | Kind::Sniper => {
                write_humanoid(dst, self.kind, &s, self.gait, amount)
            }
            Kind::Tentacle => write_tentacle(dst, &s, self.gait, amount),
            Kind::Spawnling => write_spawnling(dst, &s, self.gait, amount),
            Kind::Drone => write_drone(dst, &s, self.gait),
        }
        // After the rebuild and before the transform: the contact gradient is
        // in the model's own space, with its feet on `y = 0`.
        contact_shading(dst);
        transform_in_place(dst, mat);
        let glow = (self.flash * 0.85).max(self.alert * 0.22);
        // With no flash, no alert and no death fade — the common case for every
        // enemy but the one you just shot — the blend would leave every vertex
        // exactly as it found it, so skip the pass.
        if glow > 0.001 || alpha < 0.999 {
            tint_in_place(dst, Color::new(1.0, 0.92, 0.80, 1.0), glow, alpha);
        }
    }
}

fn angle_toward(current: f32, target: f32, t: f32) -> f32 {
    let mut d = (target - current) % TAU;
    if d > PI {
        d -= TAU;
    } else if d < -PI {
        d += TAU;
    }
    current + d * t.clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Orthographic silhouette of a prototype, as text.
    ///
    /// `side` picks the profile view (looking along X) instead of the front
    /// (looking along Z) — the two together are what an archetype has to be
    /// recognisable by, because a player behind cover sees one or the other and
    /// never reads colour at 20 m.
    fn silhouette(mesh: &Mesh, side: bool, cols: usize, rows: usize) -> Vec<String> {
        let pick = |p: Vec3| if side { vec2(p.z, p.y) } else { vec2(p.x, p.y) };
        let (mut lo, mut hi) = (Vec2::splat(f32::MAX), Vec2::splat(f32::MIN));
        for v in &mesh.vertices {
            let q = pick(v.position);
            lo = lo.min(q);
            hi = hi.max(q);
        }
        let span = (hi - lo).max(Vec2::splat(1e-4));
        let cell = |q: Vec2| {
            vec2(
                (q.x - lo.x) / span.x * (cols as f32 - 1.0),
                (rows as f32 - 1.0) - (q.y - lo.y) / span.y * (rows as f32 - 1.0),
            )
        };
        let mut filled = vec![false; cols * rows];
        for tri in mesh.indices.chunks_exact(3) {
            let p: Vec<Vec2> = tri
                .iter()
                .map(|&i| cell(pick(mesh.vertices[i as usize].position)))
                .collect();
            let (min_x, max_x) = (
                p[0].x.min(p[1].x).min(p[2].x),
                p[0].x.max(p[1].x).max(p[2].x),
            );
            let (min_y, max_y) = (
                p[0].y.min(p[1].y).min(p[2].y),
                p[0].y.max(p[1].y).max(p[2].y),
            );
            for row in
                (min_y.floor().max(0.0) as usize)..=(max_y.ceil().min(rows as f32 - 1.0) as usize)
            {
                for col in (min_x.floor().max(0.0) as usize)
                    ..=(max_x.ceil().min(cols as f32 - 1.0) as usize)
                {
                    let q = vec2(col as f32, row as f32);
                    // Three half-plane tests: the sign only has to agree.
                    let edge =
                        |a: Vec2, b: Vec2| (b.x - a.x) * (q.y - a.y) - (b.y - a.y) * (q.x - a.x);
                    let (d0, d1, d2) = (edge(p[0], p[1]), edge(p[1], p[2]), edge(p[2], p[0]));
                    let inside = (d0 >= 0.0 && d1 >= 0.0 && d2 >= 0.0)
                        || (d0 <= 0.0 && d1 <= 0.0 && d2 <= 0.0);
                    if inside {
                        filled[row * cols + col] = true;
                    }
                }
            }
        }
        (0..rows)
            .map(|row| {
                (0..cols)
                    .map(|col| if filled[row * cols + col] { '#' } else { '.' })
                    .collect()
            })
            .collect()
    }

    fn humanoid_silhouette(kind: Kind) -> Vec<String> {
        let mut rng = Rng::new(4);
        let mut e = Enemy::spawn(kind, vec3(0.0, 0.0, 0.0), 1, &mut rng);
        // `spawn` picks a random facing, which would rotate the model inside
        // its own bounding box — and every silhouette here is drawn inside the
        // bounding box, so a random yaw would make two identical models look
        // different and two different ones look the same.
        e.yaw = 0.0;
        let mut mesh = crate::mesh::empty_mesh();
        e.write_mesh(&mut mesh);
        let mut out = silhouette(&mesh, false, 26, 18);
        out.push(String::new());
        out.extend(silhouette(&mesh, true, 26, 18));
        out
    }

    /// Not an assertion: a way to look at the four archetypes without building
    /// the game and walking into one. `cargo test -- --nocapture`.
    #[test]
    #[ignore = "prints the four silhouettes; run with --ignored --nocapture"]
    fn show_the_humanoids() {
        for kind in [Kind::Grunt, Kind::Rusher, Kind::Heavy, Kind::Sniper] {
            println!("=== {kind:?} (front, then profile) ===");
            for line in humanoid_silhouette(kind) {
                println!("{line}");
            }
            // And mid-stride, which is the one thing the standing grid above
            // cannot show: whether the walk is moving anything at all.
            println!("--- walking (front) ---");
            for line in silhouette(&posed(kind, FRAC_PI_2, GAIT_FULL_RATE), false, 26, 18) {
                println!("{line}");
            }
        }
    }

    /// The four humanoids used to be the same nine boxes with different numbers
    /// on them, which made them exact clones the moment the light was too low
    /// or the range too long to read colour at — and being able to tell the
    /// archetypes apart by shape is most of what the enemy roster is for.
    ///
    /// Colour is the thing that survives distance *least*, so this compares
    /// silhouettes alone: one grid per archetype, front and profile, no shading
    /// and no palette. Both views are in the same grid because a player behind
    /// cover sees one or the other and never both.
    #[test]
    fn the_four_humanoids_do_not_share_a_silhouette() {
        let kinds = [Kind::Grunt, Kind::Rusher, Kind::Heavy, Kind::Sniper];
        let grids: Vec<Vec<bool>> = kinds
            .iter()
            .map(|&kind| {
                humanoid_silhouette(kind)
                    .iter()
                    .flat_map(|line| line.chars().map(|c| c == '#'))
                    .collect()
            })
            .collect();

        let mut worst = 1.0f32;
        let mut worst_pair = (Kind::Grunt, Kind::Grunt);
        for i in 0..kinds.len() {
            for j in i + 1..kinds.len() {
                let (a, b) = (&grids[i], &grids[j]);
                let union = (0..a.len()).filter(|&k| a[k] || b[k]).count().max(1);
                let differ = (0..a.len()).filter(|&k| a[k] != b[k]).count();
                let distance = differ as f32 / union as f32;
                if distance < worst {
                    worst = distance;
                    worst_pair = (kinds[i], kinds[j]);
                }
            }
        }
        println!(
            "the closest pair is {:?}/{:?}, differing by {:.0}% of their silhouette",
            worst_pair.0,
            worst_pair.1,
            worst * 100.0,
        );
        assert!(
            worst > 0.25,
            "{:?} and {:?} share {:.0}% of their silhouette; the closest pair has to \
             differ by more than a quarter of its own outline",
            worst_pair.0,
            worst_pair.1,
            (1.0 - worst) * 100.0,
        );
    }

    /// Every archetype's collision box has to agree with what you can see.
    ///
    /// The drone's did not: its model straddled its own origin while the box sat
    /// on top of it, so two thirds of the visible drone was unhittable and the
    /// empty air above it read as head. Nothing else in the suite notices a
    /// disagreement between geometry and hitbox, and it is invisible until you
    /// shoot one.
    #[test]
    fn every_hitbox_matches_its_model() {
        let kinds = [Kind::Grunt, Kind::Rusher, Kind::Heavy, Kind::Sniper];
        let mut rng = Rng::new(4);
        for kind in kinds {
            let e = Enemy::spawn(kind, vec3(0.0, 0.0, 0.0), 1, &mut rng);
            let mut mesh = crate::mesh::empty_mesh();
            e.write_mesh(&mut mesh);
            let (mut lo, mut hi) = (f32::MAX, f32::MIN);
            for v in &mesh.vertices {
                assert!(v.position.y.is_finite(), "{kind:?}: non-finite vertex");
                lo = lo.min(v.position.y);
                hi = hi.max(v.position.y);
            }
            assert!(hi > lo, "{kind:?}: degenerate model");

            let b = e.body_aabb();
            let model_h = hi - lo;
            let box_h = b.max.y - b.min.y;
            let ratio = box_h / model_h;
            assert!(
                (0.8..=1.25).contains(&ratio),
                "{kind:?}: the box is {box_h:.3} tall for a {model_h:.3} model (ratio {ratio:.2})",
            );
            assert!(
                (b.min.y - lo).abs() < 0.1,
                "{kind:?}: box base {:.3} does not sit on the model base {lo:.3}",
                b.min.y,
            );

            // And the head must be in the upper half of the model, or every
            // body shot becomes a headshot.
            let head = e.head_y();
            assert!(
                head > lo + model_h * 0.5 && head <= hi + 0.05,
                "{kind:?}: head_y {head:.3} is outside the model's upper half ({lo:.3}..{hi:.3})",
            );
        }
    }

    /// One frame of a model, with its walk cycle driven to `phase` and
    /// expressing `rate` cycles per second.
    fn posed(kind: Kind, phase: f32, rate: f32) -> Mesh {
        let mut rng = Rng::new(4);
        let mut e = Enemy::spawn(kind, vec3(0.0, 0.0, 0.0), 1, &mut rng);
        // `spawn` picks a random facing, which would spin the model inside its
        // own bounding box and make two poses incomparable.
        e.yaw = 0.0;
        e.gait = phase;
        e.gait_rate = rate;
        let mut mesh = crate::mesh::empty_mesh();
        e.write_mesh(&mut mesh);
        mesh
    }

    /// The four humanoids were rigid: every archetype slid across the floor in
    /// the pose it was built in, and the only thing in the game that walked was
    /// the colossus. This is the test that fails for all four of them.
    ///
    /// It also pins down what makes a procedural walk read as a walk rather
    /// than as a rocking statue: the head travels least. The body leans about a
    /// pivot at chest height, so the limbs below it swing while the head nearly
    /// holds station — and the feet, which do the travelling, carry the step.
    #[test]
    fn the_walk_cycle_moves_the_limbs_and_the_head_least() {
        for kind in [Kind::Grunt, Kind::Rusher, Kind::Heavy, Kind::Sniper] {
            let h = stats(kind).height;
            // The standing pose has to be the same at any phase: what puts the
            // limbs at rest is the rate falling to zero, not the phase landing
            // on some particular angle. Otherwise a unit that stops is frozen
            // mid-stride and has to march on to reach a pose it can stand in.
            let stand = posed(kind, 0.0, 0.0);
            let elsewhere = posed(kind, 1.9, 0.0);
            assert_eq!(stand.vertices.len(), elsewhere.vertices.len());
            for (s, o) in stand.vertices.iter().zip(&elsewhere.vertices) {
                let d = (o.position - s.position).length();
                assert!(
                    d < 1e-5,
                    "{kind:?}: a stopped walk depends on the phase: {d:.5} m"
                );
            }

            let a = posed(kind, 0.0, GAIT_FULL_RATE);
            let b = posed(kind, FRAC_PI_2, GAIT_FULL_RATE);
            assert_eq!(
                a.vertices.len(),
                b.vertices.len(),
                "{kind:?}: the model changed size between frames"
            );

            let (mut head, mut feet, mut most) = (0.0f32, 0.0f32, 0.0f32);
            for (p, q) in a.vertices.iter().zip(&b.vertices) {
                let d = (q.position - p.position).length();
                most = most.max(d);
                if p.position.y > h * 0.85 {
                    head = head.max(d);
                } else if p.position.y < h * 0.20 {
                    feet = feet.max(d);
                }
            }
            assert!(
                most > 0.02,
                "{kind:?}: two phases of the walk differ by {most:.4} m — nothing is animating"
            );
            assert!(
                feet > 0.02,
                "{kind:?}: the feet moved {feet:.4} m between phases — the limbs are rigid"
            );
            assert!(
                head * 3.0 < feet,
                "{kind:?}: the head travels {head:.3} m against the feet's {feet:.3} m, \
                 which is the whole body swinging rather than a walk",
            );
            println!(
                "{kind:?}: limbs travel up to {most:.3} m, the feet {feet:.3} m of that, \
                 the head {head:.3} m"
            );
        }
    }

    /// A rotated box has to be wound the way an axis-aligned one is.
    ///
    /// `quad` takes a face's normal from its winding, so a face wound the wrong
    /// way round lights as though it points into the limb: the leg does not
    /// disappear, it goes black and reads as a hole. Nothing else in the suite
    /// would catch that, and it only shows up on the archetypes that move.
    #[test]
    fn a_rotated_limb_keeps_its_faces_pointing_out() {
        let centre = vec3(0.37, -0.21, 0.14);
        let half = vec3(0.28, 0.13, 0.22);
        let pivot = vec3(-0.10, 0.55, -0.08);
        let joint = Joint::new(
            pivot,
            Mat4::from_rotation_z(0.5) * Mat4::from_rotation_x(0.7),
        );

        let mut straight = MeshBuilder::new();
        rig_box(
            &mut straight,
            centre,
            half,
            Joint::ROOT,
            Mat4::IDENTITY,
            WHITE,
        );
        let straight = straight.finish();

        let mut mb = MeshBuilder::new();
        rig_box(&mut mb, centre, half, joint, Mat4::IDENTITY, WHITE);
        let mesh = mb.finish();

        assert_eq!(mesh.vertices.len(), 24, "a box is six quads");
        assert_eq!(mesh.indices.len(), 36, "six quads are twelve triangles");

        // The rotation has to have been applied, and about the right pivot: a
        // box is its own centre of vertices, so the mean of them has to land
        // where the joint says.
        let box_centre = mesh.vertices.iter().map(|v| v.position).sum::<Vec3>() / 24.0;
        assert!(
            (box_centre - joint.point(centre)).length() < 1e-4,
            "the box was not rotated about the joint: {box_centre:?}",
        );
        assert!(
            (box_centre - centre).length() > 0.05,
            "the box never moved, so this proves nothing",
        );
        for (s, r) in straight.vertices.iter().zip(&mesh.vertices) {
            assert!(
                (r.position - s.position).length() > 1e-6,
                "an unrotated and a rotated box came out identical",
            );
        }

        for (quad, idx) in mesh.indices.chunks_exact(6).enumerate() {
            let p = |i: usize| mesh.vertices[idx[i] as usize].position;
            let normal = (p(1) - p(0)).cross(p(2) - p(0));
            assert!(normal.length() > 1e-6, "quad {quad}: a degenerate face");
            let face_centre = (p(0) + p(1) + p(2) + p(3)) / 4.0;
            let outward = (face_centre - box_centre).normalize_or_zero();
            assert!(
                normal.normalize().dot(outward) > 0.5,
                "quad {quad}: the normal points into the limb, so the face winds the wrong way",
            );
        }
    }

    /// The other half of the contract: a unit that is not moving must not
    /// animate.
    ///
    /// This runs the real `update` against a real arena with the player out of
    /// the world entirely, which is the only way to test "standing still" — the
    /// cycle is driven by ground covered, and a cycle driven by a clock marches
    /// on the spot. Marching on the spot is the single most obvious way a
    /// procedural gait gives itself away.
    #[test]
    fn a_standing_enemy_does_not_march_in_place() {
        let level = Level::from_layout(&crate::maps::campaign()[0]);
        let spawn = level.spawns[0];
        let player = Player::new(vec3(500.0, 0.0, 500.0));
        let mut rng = Rng::new(7);
        let mut e = Enemy::spawn(Kind::Grunt, spawn, 1, &mut rng);
        let yaw = e.yaw;
        let mut scratch = crate::mesh::empty_mesh();
        let mut neighbours = [e.pos];
        let before: Vec<Vec3> = {
            e.write_mesh(&mut scratch);
            scratch
                .vertices
                .iter()
                .map(|v| v.position - e.pos)
                .collect()
        };

        let dt = 1.0 / 60.0;
        for _ in 0..180 {
            neighbours[0] = e.pos;
            e.update(&level, &player, &neighbours, 0, dt, &mut rng);
            assert!(!e.take_footfall(), "a standing unit reported a footfall");
        }
        assert_eq!(e.gait, 0.0, "the phase advanced while standing still");
        assert_eq!(e.gait_rate, 0.0, "the cycle rate ran while standing still");
        assert_eq!(
            e.yaw, yaw,
            "a unit with nothing to look at turned on the spot"
        );
        assert!(
            (e.pos.x - spawn.x).abs() < 1e-4 && (e.pos.z - spawn.z).abs() < 1e-4,
            "a unit with nothing to chase moved from {spawn:?} to {:?}",
            e.pos,
        );

        e.write_mesh(&mut scratch);
        for (i, (a, b)) in before
            .iter()
            .zip(scratch.vertices.iter().map(|v| v.position - e.pos))
            .enumerate()
        {
            let moved = (*a - b).length();
            assert!(
                moved < 1e-5,
                "vertex {i} of a standing model moved by {moved:.5} m",
            );
        }
    }

    /// A caller hangs dust, sound and a stagger off a footfall, so the contract
    /// is exactly one report per step: two per cycle, alternating between the
    /// feet, and none at all once the unit has stopped.
    #[test]
    fn a_footfall_is_reported_once_per_step() {
        let mut rng = Rng::new(4);
        let mut e = Enemy::spawn(Kind::Grunt, Vec3::ZERO, 1, &mut rng);
        let dt = 1.0 / 60.0;
        let speed = stats(Kind::Grunt).speed;

        let (mut steps, mut crossings) = (0, 0);
        let mut prev = e.gait;
        for _ in 0..240 {
            e.advance_gait(dt, speed);
            if e.take_footfall() {
                steps += 1;
            }
            // How many half-cycle boundaries the phase crossed this frame,
            // counted independently of the way the unit counts them.
            let from = (prev - FRAC_PI_2).rem_euclid(TAU);
            let to = from + (e.gait - prev).rem_euclid(TAU);
            crossings += (to / PI).floor() as i32 - (from / PI).floor() as i32;
            prev = e.gait;
        }
        assert!(steps >= 4, "four seconds of walking reported {steps} steps");
        assert_eq!(
            steps, crossings,
            "the reports do not match the phase crossings"
        );

        // The report is a pulse, not a level: the second read of one step has
        // to come back empty.
        let mut fired = false;
        for _ in 0..240 {
            e.advance_gait(dt, speed);
            if e.take_footfall() {
                assert!(!e.take_footfall(), "one step was reported twice");
                fired = true;
                break;
            }
        }
        assert!(fired, "a walking unit never reported a footfall");

        // And a unit that has stopped does not shuffle. The rate eases to zero,
        // which is what settles the walk; after that the phase does not creep,
        // so no step can come out of it.
        for _ in 0..90 {
            e.advance_gait(dt, 0.0);
        }
        assert_eq!(e.gait_rate, 0.0, "the cycle never came to a stop");
        let settled = e.gait;
        for _ in 0..120 {
            e.advance_gait(dt, 0.0);
            assert!(!e.take_footfall(), "a stopped unit is still taking steps");
        }
        assert_eq!(
            e.gait, settled,
            "the phase crept on after the cycle stopped"
        );
    }
}
