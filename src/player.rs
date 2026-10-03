use crate::level::{Level, STEP_HEIGHT, move_and_collide};
use crate::util::{approach, dir_from_angles};
use macroquad::prelude::*;
use std::f32::consts::TAU;

pub const PLAYER_RADIUS: f32 = 0.40;
/// Standing body height. Crouched and prone bodies are shorter; see `Stance`.
pub const PLAYER_HEIGHT: f32 = 1.80;

const SPRINT_SPEED: f32 = 9.2;
const AIM_SPEED: f32 = 3.6;
const JUMP_VELOCITY: f32 = 7.5;
const GRAVITY: f32 = 23.0;
const MAX_FALL: f32 = 60.0;

/// Physics integration step, in seconds. This is not the game's tick rate:
/// `update` slices whatever the frame handed it into steps this long. The
/// model's constants stop meaning the same thing at a different step, and the
/// same Quake engine was measurably faster at 77 Hz than at 72 Hz for exactly
/// that reason.
const TARGET_STEP: f32 = 1.0 / 120.0;
/// Most substeps one frame may cost. A frame that stalls — a load, an alt-tab,
/// a breakpoint — must not be paid back with an unbounded number of steps:
/// past this the step grows and the model degrades instead of the frame
/// spiralling.
const MAX_STEPS: f32 = 8.0;
/// Acceleration as a rate rather than in m/s², the way Quake's `sv_accelerate`
/// is written: one second of held input may add `rate * max_speed` to the
/// velocity's projection on the wish direction. Full speed takes `1 / rate`
/// seconds from a standstill whatever the stance, and the rate is what the old
/// m/s² cap worked out to across the stances — 95 m/s² against 6.6 to 9.2 m/s
/// of target is 10 to 14 per second.
const GROUND_ACCEL_RATE: f32 = 15.0;
/// Air control, deliberately weak: the old 17 m/s² at standing speed.
const AIR_ACCEL_RATE: f32 = 2.6;
/// Ground friction: speed falls by `1 - FRICTION * dt` per step, so a released
/// player slides roughly `speed / FRICTION` metres. 20 is what the old model's
/// braking worked out to at walking speed — it stopped in 0.37 m and a tenth of
/// a second — and it is also what brings a player down to the cap after a
/// sprint, a crouch, a prone drop or the sights coming up. Friction only ever
/// bites *above* the cap, so holding a key still settles exactly on it.
const FRICTION: f32 = 20.0;
/// The speed below which friction stops scaling with speed and becomes a flat
/// deceleration. Without it the last metre of a slide takes several times as
/// long as the first, because a proportional drop shrinks with the speed it is
/// removing — the player feels a slow crawl to a halt with no key held. Quake
/// and Source both carry this as `stopspeed`; two metres per second is the
/// point below which a body is close enough to stopped to just stop.
const STOP_SPEED: f32 = 2.0;
/// Friction in the air, where the cap binds just as hard. Holding a key down
/// still steers and still accelerates up to the cap, but the air is not allowed
/// to carry more speed than the legs would have: a hop cannot be chained into a
/// faster mode of travel, and letting go of the keys in mid-air slows the body
/// down instead of leaving it to sail on. Set this to 0 to give the air its
/// momentum back (a strafe jump then accelerates past the cap again).
const AIR_FRICTION: f32 = 6.0;
/// Physics ticks after touchdown that are exempt from friction. One keeps the
/// hard landing tick from scrubbing the speed the body arrived with. Set it to
/// 0 to make landings bite immediately.
const LANDING_FRICTION_GRACE_TICKS: u32 = 1;

/// How far the head swings sideways when leaning around cover.
const LEAN_DISTANCE: f32 = 0.42;
/// Seconds of held breath available, and how fast it drains / recovers.
const BREATH_DRAIN: f32 = 0.34;
const BREATH_RECOVER: f32 = 0.26;

/// Recoil pattern, by shot number within one burst:
///
/// - `1..=3`: an even climb, the muzzle barely walks;
/// - `4..=7`: the climb grows with every shot;
/// - `8..=10`: the muzzle walks up and to the left;
/// - `11..=17`: the horizontal walk takes over and reaches its cap.
///
/// A scripted pattern exists because random recoil cannot be learned — a
/// player can only get lucky with it. `WeaponStats::recoil_pitch` and
/// `recoil_yaw` stay the per-shot base values; this only scales them.
const RECOIL_UNIFORM_SHOTS: u32 = 3;
const RECOIL_CLIMB_END: u32 = 7;
const RECOIL_LEFT_END: u32 = 10;
const RECOIL_HORIZONTAL_END: u32 = 17;
/// Pitch multiplier for the first shot of the growing-climb segment and for
/// its last, interpolated between.
const RECOIL_CLIMB_RISE: f32 = 1.15;
const RECOIL_CLIMB_PEAK: f32 = 1.60;
/// Pitch and yaw multipliers while the pattern walks left.
const RECOIL_LEFT_PITCH: f32 = 1.40;
const RECOIL_LEFT_YAW: f32 = 0.90;
/// Where the horizontal walk plateaus. The climb keeps some authority, but the
/// sideways kick dominates and stops growing — a longer spray than the pattern
/// was designed for must not walk off the screen.
const RECOIL_HORIZONTAL_PITCH: f32 = 0.85;
const RECOIL_HORIZONTAL_YAW_MAX: f32 = 2.40;
/// Fraction of a shot's pitch kick the horizontal walk uses when the weapon
/// itself has no yaw recoil, so that no burst ever walks straight up.
const RECOIL_YAW_FLOOR_RATIO: f32 = 0.35;
/// Seconds after the last shot before the muzzle starts falling back. A burst
/// has to be able to climb: decaying between shots flattens the pattern down
/// to the first shot's kick, which is what happened when the fallback ran
/// every frame regardless.
const RECOIL_FALLBACK_DELAY: f32 = 0.05;
/// A gap longer than this starts a new burst, and the pattern goes back to
/// shot one. Trigger discipline has to be able to reset it.
const RECOIL_BURST_GAP: f32 = 0.25;
/// Fallback speed once the delay has passed. This was inlined as
/// `(-11.0 * dt).exp()` on the recoil lines.
const RECOIL_FALLBACK_RATE: f32 = 11.0;
/// How fast a hit's view kick (the "aimpunch") recovers on its own. Slower
/// than the muzzle's fallback on purpose: the kick has to live long enough to
/// be *felt* as the thing armour takes away (reference §17.2②), which the
/// 11-per-second recoil fallback would erase inside a tenth of a second.
const PUNCH_RECOVERY: f32 = 6.0;
/// The most a single hit may kick the view, in radians. A Colossus slam
/// clamps here; it is a shove, not a spin.
const PUNCH_MAX: f32 = 0.09;

/// Seconds for a hit indicator to fade from full strength to nothing.
const HURT_DIR_DECAY: f32 = 1.5;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Stance {
    Stand,
    Crouch,
    Prone,
}

impl Stance {
    pub fn height(self) -> f32 {
        match self {
            Stance::Stand => PLAYER_HEIGHT,
            Stance::Crouch => 1.20,
            Stance::Prone => 0.58,
        }
    }

    pub fn eye(self) -> f32 {
        match self {
            Stance::Stand => 1.64,
            Stance::Crouch => 1.03,
            Stance::Prone => 0.42,
        }
    }

    pub fn speed(self) -> f32 {
        match self {
            Stance::Stand => 6.6,
            Stance::Crouch => 3.2,
            Stance::Prone => 1.15,
        }
    }

    /// How easily enemies spot a player in this stance. Lower is stealthier.
    pub fn detectability(self) -> f32 {
        match self {
            Stance::Stand => 1.0,
            Stance::Crouch => 0.72,
            Stance::Prone => 0.45,
        }
    }

    /// Multiplier on weapon sway and spread.
    pub fn steadiness(self) -> f32 {
        match self {
            Stance::Stand => 1.0,
            Stance::Crouch => 0.72,
            Stance::Prone => 0.42,
        }
    }
}

pub struct MoveInput {
    pub forward: f32,
    pub right: f32,
    pub jump: bool,
    pub sprint: bool,
    pub aim: bool,
    /// Caps Lock — a slow, quiet walk.
    pub walk: bool,
    /// Q / E, -1.0 is left, +1.0 is right.
    pub lean: f32,
    /// Left Alt.
    pub hold_breath: bool,
    /// Middle mouse — decouples the view from the body's facing.
    pub free_look: bool,
    /// C / Z, already resolved to a target stance by the caller.
    pub stance: Stance,
    /// Weight of the weapon in hand, from `Arsenal::move_scale`. Scales every
    /// movement speed, so a light knife moves faster than a heavy LMG.
    pub speed_scale: f32,
}

impl Default for MoveInput {
    fn default() -> Self {
        Self {
            forward: 0.0,
            right: 0.0,
            jump: false,
            sprint: false,
            aim: false,
            walk: false,
            lean: 0.0,
            hold_breath: false,
            free_look: false,
            stance: Stance::Stand,
            speed_scale: 1.0,
        }
    }
}

pub struct Player {
    pub pos: Vec3,
    pub vel: Vec3,
    /// Facing of the *body*: movement follows this, not the camera.
    pub yaw: f32,
    pub pitch: f32,
    pub on_ground: bool,
    pub health: f32,
    pub max_health: f32,
    pub alive: bool,
    pub ads: f32,
    pub ads_fov: f32,

    pub stance: Stance,
    /// Smoothed eye height so stance changes read as a movement, not a cut.
    eye_smooth: f32,
    /// Smoothed lean, -1..1.
    pub lean: f32,
    pub walk: bool,

    /// Offsets applied only to the camera while free-looking.
    pub free_yaw: f32,
    pub free_pitch: f32,

    /// 1.0 = full breath, 0.0 = spent.
    pub breath: f32,
    pub holding_breath: bool,
    pub free_looking: bool,

    bob_phase: f32,
    bob_amount: f32,
    pub recoil_pitch: f32,
    pub recoil_yaw: f32,
    /// The view kick a *hit taken* leaves behind — the genre's "aimpunch".
    /// Separate from the recoil above because it recovers on its own slower
    /// schedule; armour scales it down before it lands (reference §17.2②).
    punch: f32,
    /// Shot count within the current burst, and seconds since the last shot.
    /// The pattern is indexed by the first and restarted by the second.
    burst_shot: u32,
    since_shot: f32,
    pub hurt_flash: f32,
    pub since_damage: f32,
    pub shake: f32,
    /// Distance fallen this frame, non-zero only on the frame of landing.
    pub landing: f32,
    /// Grounded physics ticks since touchdown; the friction grace in `update`
    /// reads it. Counts up on the floor, resets in the air.
    ground_ticks: u32,
    /// 0..100. Drains while something cosmic is in view; recovers when it is
    /// not. Governs aim instability and how much the view writhes.
    pub sanity: f32,
    /// Bearing of the most recent hit, flattened, and how strongly to draw it.
    /// Read through `hurt_dir`.
    threat_dir: Vec3,
    threat_t: f32,
}

impl Player {
    pub fn new(pos: Vec3) -> Self {
        Self {
            pos,
            vel: Vec3::ZERO,
            yaw: 0.0,
            pitch: 0.0,
            on_ground: false,
            health: 100.0,
            max_health: 100.0,
            alive: true,
            ads: 0.0,
            ads_fov: 38.0,
            stance: Stance::Stand,
            eye_smooth: Stance::Stand.eye(),
            lean: 0.0,
            walk: false,
            free_yaw: 0.0,
            free_pitch: 0.0,
            breath: 1.0,
            holding_breath: false,
            free_looking: false,
            bob_phase: 0.0,
            bob_amount: 0.0,
            recoil_pitch: 0.0,
            recoil_yaw: 0.0,
            punch: 0.0,
            burst_shot: 0,
            // Long enough that the first shot counts as shot one of a burst.
            since_shot: 99.0,
            hurt_flash: 0.0,
            since_damage: 99.0,
            shake: 0.0,
            landing: 0.0,
            ground_ticks: LANDING_FRICTION_GRACE_TICKS,
            sanity: 100.0,
            threat_dir: Vec3::ZERO,
            threat_t: 0.0,
        }
    }

    pub fn sanity_ratio(&self) -> f32 {
        (self.sanity / 100.0).clamp(0.0, 1.0)
    }

    /// Multiplier on sway and spread once the mind starts going. 1.0 down to
    /// half sanity, then climbing to 2.6x at zero.
    pub fn madness_scale(&self) -> f32 {
        let r = self.sanity_ratio();
        if r > 0.6 {
            1.0
        } else {
            1.0 + (0.6 - r) / 0.6 * 1.6
        }
    }

    /// A slow writhing drift applied to the camera as sanity falls. This is
    /// deliberately not controllable — that is the point.
    pub fn madness_warp(&self) -> Vec2 {
        let r = self.sanity_ratio();
        if r > 0.5 {
            return Vec2::ZERO;
        }
        let k = (0.5 - r) / 0.5;
        let t = get_time() as f32;
        vec2((t * 0.73).sin() * 0.009 * k, (t * 1.31).cos() * 0.013 * k)
    }

    pub fn horizontal_speed(&self) -> f32 {
        vec2(self.vel.x, self.vel.z).length()
    }

    pub fn body_height(&self) -> f32 {
        self.stance.height()
    }

    pub fn stance(&self) -> Stance {
        self.stance
    }

    /// Total camera yaw, including free-look and sway.
    pub fn cam_yaw(&self) -> f32 {
        let w = self.madness_warp();
        self.yaw + self.free_yaw + self.recoil_yaw + self.sway().x + w.x
    }

    pub fn cam_pitch(&self) -> f32 {
        let w = self.madness_warp();
        // The hit kick rides on the camera like the recoil does; unlike the
        // recoil it recovers by itself, on `PUNCH_RECOVERY`'s clock.
        self.pitch + self.free_pitch + self.recoil_pitch + self.punch + self.sway().y + w.y
    }

    /// Aim wobble in radians. Only bites while aiming down sights, and holding
    /// breath almost cancels it — that is what makes the sniper usable.
    pub fn sway(&self) -> Vec2 {
        let ads_k = ((self.ads - 0.35) / 0.65).clamp(0.0, 1.0);
        if ads_k <= 0.0 {
            return Vec2::ZERO;
        }
        let mut amp = 0.0030 * self.stance.steadiness() * ads_k * self.madness_scale();
        if self.holding_breath && self.breath > 0.0 {
            amp *= 0.18;
        } else if self.breath <= 0.0 {
            amp *= 1.7;
        }
        let t = get_time() as f32;
        vec2((t * 1.10).sin() * amp, (t * 1.70).cos() * amp * 0.85)
    }

    pub fn eye(&self) -> Vec3 {
        let bob_y = (self.bob_phase * 2.0).sin() * 0.028 * self.bob_amount;
        let bob_x = self.bob_phase.sin() * 0.030 * self.bob_amount;
        let right = vec3(self.yaw.cos(), 0.0, -self.yaw.sin());
        vec3(self.pos.x, self.pos.y + self.eye_smooth + bob_y, self.pos.z)
            + right * (bob_x + self.lean * LEAN_DISTANCE)
    }

    pub fn look_dir(&self) -> Vec3 {
        dir_from_angles(self.cam_yaw(), self.cam_pitch())
    }

    /// Camera roll from leaning, in radians.
    pub fn lean_roll(&self) -> f32 {
        -self.lean * 0.11
    }

    pub fn bob_phase(&self) -> f32 {
        self.bob_phase
    }

    pub fn bob_amount(&self) -> f32 {
        self.bob_amount
    }

    /// How visible the player is to enemies: stance and movement both matter.
    pub fn visibility(&self) -> f32 {
        let mut v = self.stance.detectability();
        let speed = self.horizontal_speed();
        if self.walk {
            v *= 0.82;
        }
        v *= 1.0 + (speed / 9.2).clamp(0.0, 1.0) * 0.25;
        v
    }

    /// Multiplier applied on top of the weapon's own spread.
    pub fn spread_scale(&self) -> f32 {
        let mut k = self.stance.steadiness();
        if self.walk {
            k *= 0.85;
        }
        if self.holding_breath && self.breath > 0.0 {
            k *= 0.55;
        } else if self.breath <= 0.0 {
            k *= 1.45;
        }
        k * self.madness_scale()
    }

    /// Movement speed as a 0..1 ratio against a full sprint, for weapon bloom.
    pub fn move_ratio(&self) -> f32 {
        (self.horizontal_speed() / SPRINT_SPEED).clamp(0.0, 1.0)
    }

    pub fn fovy(&self) -> f32 {
        let base = 62.0f32.to_radians();
        let aimed = self.ads_fov.to_radians();
        base + (aimed - base) * self.ads
    }

    /// Kicks the muzzle by one shot's worth of recoil, shaped by where the
    /// shot falls in the burst. `pitch` and `yaw` are the weapon's per-shot
    /// base values — the pattern scales them, it does not replace them.
    pub fn add_recoil(&mut self, pitch: f32, yaw: f32) {
        // A burst that stopped for longer than the gap is a new burst: the
        // pattern starts over from shot one.
        if self.since_shot > RECOIL_BURST_GAP {
            self.burst_shot = 0;
        }
        self.burst_shot = self.burst_shot.saturating_add(1);
        self.since_shot = 0.0;

        let (pitch_scale, yaw_scale, leftward) = recoil_shape(self.burst_shot);
        let yaw_kick = if leftward {
            // Once the pattern walks left it walks left every burst, whatever
            // sign the caller's random yaw jitter produced — a direction the
            // player cannot predict is a direction they cannot learn to pull
            // against. The magnitude takes a floor too, so a weapon with no yaw
            // recoil of its own still moves sideways.
            yaw.abs().max(pitch.abs() * RECOIL_YAW_FLOOR_RATIO) * yaw_scale
        } else {
            yaw * yaw_scale
        };
        self.recoil_yaw += yaw_kick;
        self.recoil_pitch += pitch * pitch_scale;
    }

    /// Kicks the *view* by a hit taken — the genre's "aimpunch". The caller
    /// scales it by whether armour stopped the round (reference §17.2②):
    /// kevlar takes most of the punch, which is how the player feels the
    /// armour working without another damage number on screen.
    pub fn aim_punch(&mut self, amount: f32) {
        self.punch = (self.punch + amount).min(PUNCH_MAX);
    }

    /// The player pulling back against the climb. `pitch_delta` and `yaw_delta`
    /// are view deltas in the same convention as `pitch` and `yaw` — the caller
    /// passes the mouse movement it is about to apply to them.
    ///
    /// Recoil is a view offset, not a change to where the player is looking:
    /// the muzzle falls back on its own once the burst ends, so a player who
    /// fought the climb with the mouse would end up aiming below where they
    /// started. Eating the accumulated recoil by the amount they pulled
    /// removes that double correction. The compensation is capped by what has
    /// actually accumulated: it can cancel the climb, never invert it.
    pub fn compensate_recoil(&mut self, pitch_delta: f32, yaw_delta: f32) {
        self.recoil_pitch = compensate_axis(self.recoil_pitch, pitch_delta);
        self.recoil_yaw = compensate_axis(self.recoil_yaw, yaw_delta);
    }

    /// Records that damage just came in from `dir`, a world-space direction
    /// pointing from the player at whatever hit them. The HUD turns it into the
    /// arc that tells the player which way to turn.
    ///
    /// The newest hit wins: in a firefight the last thing to shoot is the thing
    /// that matters, and blending two bearings would point at neither.
    pub fn hurt_from(&mut self, dir: Vec3) {
        // Flat, because the arc is drawn in the horizontal plane: a hit from a
        // floor trap or from straight above has no bearing to show.
        let flat = vec3(dir.x, 0.0, dir.z);
        if flat.length_squared() < 1e-6 {
            // Nothing to point at. Keep the last bearing rather than blanking
            // an indicator that is still counting down.
            return;
        }
        self.threat_dir = flat.normalize();
        self.threat_t = 1.0;
    }

    /// Bearing of the most recent damage and how strongly to show it: 1.0 at
    /// the moment of the hit, fading on a linear ramp to 0 over
    /// `HURT_DIR_DECAY` seconds, after which this is `None`.
    pub fn hurt_dir(&self) -> Option<(Vec3, f32)> {
        if self.threat_t <= 0.0 {
            None
        } else {
            Some((self.threat_dir, self.threat_t))
        }
    }

    pub fn damage(&mut self, amount: f32) {
        if !self.alive {
            return;
        }
        self.health -= amount;
        self.hurt_flash = 1.0;
        self.since_damage = 0.0;
        self.shake = (self.shake + 0.30 + amount * 0.012).min(1.0);
        if self.health <= 0.0 {
            self.health = 0.0;
            self.alive = false;
        }
    }

    pub fn heal(&mut self, amount: f32) -> f32 {
        let before = self.health;
        self.health = (self.health + amount).min(self.max_health);
        self.health - before
    }

    /// Puts the body back to neutral for a new wave: full health and full
    /// breath, with nothing left channeling. The wave clock calls this
    /// between waves (reference §19.4-5), so a death is a death of position
    /// and decision rather than of the last wave's leftovers. Ammunition,
    /// armour and sanity are deliberately not part of it — those are the
    /// arena's economy, not the wave's.
    pub fn wave_reset(&mut self) {
        self.health = self.max_health;
        self.breath = 1.0;
    }

    /// Can the body occupy `stance` right here, or is there a ceiling in the way?
    pub fn can_assume(&self, level: &Level, stance: Stance) -> bool {
        level.point_free(self.pos, PLAYER_RADIUS, stance.height())
    }

    /// One frame of player simulation. The frame-level bookkeeping — stance,
    /// breath, aim, camera smoothing, bob, recoil fallback, healing — happens
    /// once, and only the physics integration is substepped at `TARGET_STEP`.
    pub fn update(&mut self, level: &Level, dt: f32, input: &MoveInput) {
        self.walk = input.walk;
        self.holding_breath = input.hold_breath;
        self.free_looking = input.free_look;

        // --- stance ---------------------------------------------------------
        // Never grow into geometry: if there is no headroom, stay small.
        if input.stance.height() > self.stance.height() && !self.can_assume(level, input.stance) {
            // keep the current stance
        } else {
            self.stance = input.stance;
        }

        // --- breath ---------------------------------------------------------
        if input.hold_breath && self.breath > 0.0 {
            self.breath = (self.breath - BREATH_DRAIN * dt).max(0.0);
        } else {
            self.breath = (self.breath + BREATH_RECOVER * dt).min(1.0);
        }

        // Free look collapses back onto the body once released.
        if !input.free_look {
            let k = (-9.0 * dt).exp();
            self.free_yaw *= k;
            self.free_pitch *= k;
        }

        // Lean eases in and out rather than snapping.
        self.lean = approach(self.lean, input.lean, dt * 4.5);

        self.ads = approach(self.ads, if input.aim { 1.0 } else { 0.0 }, dt * 7.0);

        // --- movement intent -------------------------------------------------
        // Resolved once, outside the substep loop: the facing, the stance and
        // the weapon in hand cannot change between substeps, so the direction
        // the player is pushing is a frame-level fact. Everything below it —
        // friction, acceleration, jumping, gravity, collisions — is integrated
        // at a fixed step instead, because the model's output depends on the
        // step it is given.
        let fwd = vec3(-self.yaw.sin(), 0.0, -self.yaw.cos());
        let right = vec3(self.yaw.cos(), 0.0, -self.yaw.sin());
        let mut wish = fwd * input.forward + right * input.right;
        if wish.length() > 1.0 {
            wish = wish.normalize();
        }

        let stance = self.stance;
        let sprinting = input.sprint
            && input.forward > 0.4
            && self.ads < 0.4
            && stance == Stance::Stand
            && input.lean.abs() < 0.5;

        let base_speed = if self.ads > 0.4 {
            AIM_SPEED * stance.height() / PLAYER_HEIGHT
        } else if sprinting {
            SPRINT_SPEED
        } else if input.walk {
            stance.speed() * 0.52
        } else {
            stance.speed()
        };
        // The weapon in hand has the final say on foot speed.
        let speed = base_speed * input.speed_scale;

        // Direction and cap stay apart the way Quake keeps `wishdir` and
        // `wishspeed` apart: pushing two keys at once is a direction, not extra
        // speed.
        let wish_h = vec2(wish.x, wish.z);
        let wishdir = wish_h.normalize_or_zero();
        let max_speed = speed * wish_h.length();

        // --- physics ----------------------------------------------------------
        // Fixed-step integration. `round` rather than a leftover-time
        // accumulator: rounding always advances the body by the frame's whole
        // `dt` of travel, so the camera never has to stutter to stay in phase
        // with a step boundary, while the step itself lands within about ±20%
        // of `TARGET_STEP` at 60, 120 and 144 Hz — against the 140% spread a
        // raw `dt` leaves the model with between 60 and 144 Hz.
        let steps = (dt / TARGET_STEP).round().clamp(1.0, MAX_STEPS);
        let h = dt / steps;
        let mut landing = 0.0f32;

        for _ in 0..steps as u32 {
            // Ticks since touchdown, for the friction grace.
            if self.on_ground {
                self.ground_ticks = self.ground_ticks.saturating_add(1);
            } else {
                self.ground_ticks = 0;
            }
            // A step acts on the ground state the previous step ended with,
            // which is how the engine this model comes from reads it too.
            let grounded = self.on_ground;

            // Two rules, for the two jobs the old model did with one step
            // ("move the whole velocity toward the target velocity"):
            //
            // * Released — the cap is zero — friction brings the body to rest,
            //   crisp at the end thanks to `STOP_SPEED`, which is what the old
            //   linear brake did and a proportional drop does not.
            // * Held, the projection rule accelerates, and the *total* is then
            //   held to the cap. The projection rule alone only limits the
            //   speed along the wish direction, so a body already moving across
            //   it ends the step above the cap — which is the whole trick a
            //   strafe jump is built on, and not something this game wants: a
            //   hop must not bank speed the legs never had. Clamping after the
            //   acceleration rather than before it is what makes the cap a
            //   ceiling instead of something the step ends above.
            //
            // The landing grace is a ground-only window: airborne ticks never
            // take it, or a hop would keep every metre per second it left the
            // floor with.
            let landing_grace = grounded && self.ground_ticks <= LANDING_FRICTION_GRACE_TICKS;
            let mut vel_h = vec2(self.vel.x, self.vel.z);
            if !landing_grace && vel_h.length() > max_speed && wishdir == Vec2::ZERO {
                let rate = if grounded { FRICTION } else { AIR_FRICTION };
                vel_h = apply_friction(vel_h, rate, h);
            }
            if wishdir != Vec2::ZERO {
                let rate = if !grounded {
                    AIR_ACCEL_RATE
                } else if stance == Stance::Prone {
                    GROUND_ACCEL_RATE * 0.5
                } else {
                    GROUND_ACCEL_RATE
                };
                vel_h = accelerate(vel_h, wishdir, max_speed, rate, h);
                let over = vel_h.length();
                if over > max_speed {
                    vel_h *= max_speed / over;
                }
            }
            self.vel.x = vel_h.x;
            self.vel.z = vel_h.y;

            // A held jump relaunches the body on the tick it touches down,
            // which is the other half of the friction grace above.
            if input.jump && grounded && stance == Stance::Stand {
                self.vel.y = JUMP_VELOCITY;
                self.on_ground = false;
            }

            self.vel.y = (self.vel.y - GRAVITY * h).max(-MAX_FALL);

            // Deliberately unclamped: `move_and_collide` substeps internally, so
            // a fast body cannot tunnel and the caller does not need to know a
            // safe step size. The clamp that used to live here also capped the
            // effective fall speed at 0.85 / dt, which made the same drop take
            // different times at 30 fps and at 144 fps.
            let step = self.vel * h;

            let (pos, col) = move_and_collide(
                level,
                self.pos,
                PLAYER_RADIUS,
                stance.height(),
                step,
                STEP_HEIGHT,
            );
            self.pos = pos;

            if col.x {
                self.vel.x = 0.0;
            }
            if col.z {
                self.vel.z = 0.0;
            }
            if col.ground {
                if self.vel.y < 0.0 {
                    // Several substeps can touch down in one frame, and a body
                    // resting on the floor reports a ground contact on every one
                    // of them. The frame keeps the hardest: assigning instead
                    // would let a later resting step overwrite a hard landing
                    // with its own 0.4 m/s.
                    landing = landing.max(-self.vel.y);
                    self.vel.y = 0.0;
                }
                self.on_ground = true;
            } else {
                self.on_ground = false;
            }
            if col.ceiling && self.vel.y > 0.0 {
                self.vel.y = 0.0;
            }
        }

        // Non-zero only on the frame that touched down, and then it is the
        // hardest of the substeps that did.
        self.landing = landing;

        // --- camera smoothing ------------------------------------------------
        let target_eye = stance.eye();
        // Rising is slower than dropping, so standing up reads as deliberate.
        let rate = if target_eye > self.eye_smooth {
            7.0
        } else {
            11.0
        };
        self.eye_smooth = approach(self.eye_smooth, target_eye, dt * rate);

        let moving = self.horizontal_speed() > 0.4;
        let speed_ratio = (self.horizontal_speed() / SPRINT_SPEED).clamp(0.0, 1.0);
        // Walking deliberately keeps the weapon still, which is its whole point.
        let quiet = if self.walk { 0.25 } else { 1.0 };
        let stance_bob = match stance {
            Stance::Stand => 1.0,
            Stance::Crouch => 0.55,
            Stance::Prone => 0.15,
        };
        let target_bob = if self.on_ground && moving {
            speed_ratio * (1.0 - self.ads * 0.7) * quiet * stance_bob
        } else {
            0.0
        };
        self.bob_amount = approach(self.bob_amount, target_bob, dt * 4.0);
        self.bob_phase = (self.bob_phase + dt * self.horizontal_speed() * 1.15) % TAU;

        // The muzzle falls back only once the burst has stopped. A fallback
        // that ran between shots flattened every burst to the first shot's
        // kick, so the pattern had nothing to climb and the player had nothing
        // to learn.
        self.since_shot += dt;
        if self.since_shot > RECOIL_FALLBACK_DELAY {
            let decay = (-RECOIL_FALLBACK_RATE * dt).exp();
            self.recoil_pitch *= decay;
            self.recoil_yaw *= decay;
        }
        self.hurt_flash = approach(self.hurt_flash, 0.0, dt * 2.2);
        self.shake = approach(self.shake, 0.0, dt * 2.0);
        self.punch *= (-PUNCH_RECOVERY * dt).exp();

        // The hit indicator fades on a schedule rather than an asymptote, so
        // that "gone" is an exact state for the HUD to test instead of a value
        // that halves forever.
        self.threat_t = (self.threat_t - dt / HURT_DIR_DECAY).max(0.0);

        self.since_damage += dt;
        if self.alive && self.since_damage > 4.5 && self.health < self.max_health {
            self.health = (self.health + 8.0 * dt).min(self.max_health);
        }
    }

    /// Screen-space offset (pixels) used to shake the camera.
    pub fn shake_offset(&self) -> Vec2 {
        if self.shake <= 0.0 {
            return Vec2::ZERO;
        }
        let t = get_time() as f32 * 47.0;
        vec2(t.sin(), (t * 1.37).cos()) * self.shake * 13.0
    }
}

/// Quake's acceleration rule: the horizontal velocity is limited only along the
/// direction the player is pushing.
///
/// `add` is clamped so the *projection* on `wishdir` never passes `max_speed`.
/// That is why a body already faster than the cap keeps the extra speed instead
/// of being dragged back to it, and why a body moving across the wish direction
/// gains rather than being rotated into it. The old rule — step the whole
/// velocity vector toward a target velocity, by at most `accel * dt` — removed
/// exactly the perpendicular component a strafe builds, which made air-strafing
/// impossible by construction.
fn accelerate(vel_h: Vec2, wishdir: Vec2, max_speed: f32, accel_rate: f32, dt: f32) -> Vec2 {
    let proj = vel_h.dot(wishdir);
    let mut add = accel_rate * max_speed * dt;
    if proj + add > max_speed {
        add = max_speed - proj;
    }
    if add <= 0.0 {
        // Already at or past the cap in this direction. Holding the key must
        // not brake the player, so nothing is added.
        return vel_h;
    }
    vel_h + wishdir * add
}

/// Ground friction. The velocity keeps its direction and loses `friction` of
/// its speed per second, except that the drop is never taken from below
/// `STOP_SPEED` — there it becomes flat, so the body arrives at a stop instead
/// of crawling the last metre.
fn apply_friction(vel_h: Vec2, friction: f32, dt: f32) -> Vec2 {
    let speed = vel_h.length();
    if speed <= 0.0 {
        // A body at rest. Scaling by `0 / 0` here would write a NaN into the
        // velocity that no later frame could clear.
        return Vec2::ZERO;
    }
    let drop = speed.max(STOP_SPEED) * friction * dt;
    // A step long enough to eat the whole speed stops the body rather than
    // turning it around: frames hitch.
    vel_h * ((speed - drop).max(0.0) / speed)
}

/// Moves `recoil` toward zero by the part of `delta` that opposes it. Movement
/// in the same direction as the climb is not compensation — the caller has
/// already applied that to the view — and the result never crosses zero, so a
/// pull can cancel the climb but never invert it.
fn compensate_axis(recoil: f32, delta: f32) -> f32 {
    if recoil > 0.0 {
        (recoil + delta.min(0.0)).max(0.0)
    } else if recoil < 0.0 {
        (recoil + delta.max(0.0)).min(0.0)
    } else {
        0.0
    }
}

/// Recoil multipliers for shot `shot` of a burst, 1-based: pitch scale, yaw
/// scale, and whether the yaw kick is forced left. The segment boundaries and
/// the values they interpolate between are the `RECOIL_*` constants above.
fn recoil_shape(shot: u32) -> (f32, f32, bool) {
    match shot {
        // Only reachable before the first shot of a burst.
        0 => (1.0, 1.0, false),
        // 1..=3: an even climb, the sides left to the weapon's own jitter.
        1..=RECOIL_UNIFORM_SHOTS => (1.0, 0.55, false),
        // 4..=7: the climb grows shot by shot.
        n if n <= RECOIL_CLIMB_END => {
            let t = (n - RECOIL_UNIFORM_SHOTS) as f32
                / (RECOIL_CLIMB_END - RECOIL_UNIFORM_SHOTS) as f32;
            (
                RECOIL_CLIMB_RISE + (RECOIL_CLIMB_PEAK - RECOIL_CLIMB_RISE) * t,
                0.70,
                false,
            )
        }
        // 8..=10: the muzzle starts walking up and to the left.
        n if n <= RECOIL_LEFT_END => (RECOIL_LEFT_PITCH, RECOIL_LEFT_YAW, true),
        // 11..=17: the horizontal walk takes over while the climb eases off.
        n if n <= RECOIL_HORIZONTAL_END => {
            let t = (n - RECOIL_LEFT_END) as f32 / (RECOIL_HORIZONTAL_END - RECOIL_LEFT_END) as f32;
            (
                RECOIL_LEFT_PITCH + (RECOIL_HORIZONTAL_PITCH - RECOIL_LEFT_PITCH) * t,
                RECOIL_LEFT_YAW + (RECOIL_HORIZONTAL_YAW_MAX - RECOIL_LEFT_YAW) * t,
                true,
            )
        }
        // Past the cap the pattern holds.
        _ => (RECOIL_HORIZONTAL_PITCH, RECOIL_HORIZONTAL_YAW_MAX, true),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::maps;

    /// The physics tests want a floor and real geometry under them, so they use
    /// the layout the campaign starts on.
    fn test_level() -> Level {
        Level::from_layout(&maps::arena())
    }

    /// A body standing in the arena's clear lane: x = 20, running from z = 8
    /// towards -z, with nothing in the way for over 14 m. It starts half a
    /// metre up, where `Layout::player_start` starts, and settles.
    fn spawn() -> Player {
        Player::new(vec3(20.0, 0.5, 8.0))
    }

    /// Drives `frames` updates at a fixed frame time.
    fn run(player: &mut Player, level: &Level, input: &MoveInput, dt: f32, frames: u32) {
        for _ in 0..frames {
            player.update(level, dt, input);
        }
    }

    fn flat(pos: Vec3) -> Vec2 {
        vec2(pos.x, pos.z)
    }

    /// The hit kick (reference §17.2②): a hit raises the camera, the kick is
    /// clamped — a shove, not a spin — and it recovers on its own clock.
    #[test]
    fn a_hit_kicks_the_view_up_and_the_kick_recovers() {
        let mut player = spawn();
        let level = test_level();
        let input = MoveInput::default();
        let before = player.cam_pitch();
        player.aim_punch(0.02);
        assert!(player.cam_pitch() > before, "a hit has to move the aim");
        player.aim_punch(10.0);
        assert!(player.punch <= PUNCH_MAX + 1e-5, "the kick is not clamped");

        run(&mut player, &level, &input, 1.0 / 120.0, 60);
        assert!(
            player.punch < PUNCH_MAX * 0.06,
            "the kick is still {} after half a second",
            player.punch
        );
    }

    /// The rule the model is built on: pushing at a right angle to the velocity
    /// adds to it instead of rotating it back toward the push.
    #[test]
    fn acceleration_spares_speed_perpendicular_to_the_wish() {
        let wishdir = vec2(0.0, 1.0);
        let before = vec2(8.0, 0.0);
        let after = accelerate(before, wishdir, 6.6, GROUND_ACCEL_RATE, 1.0 / 120.0);
        assert_eq!(
            after.x, 8.0,
            "the speed across the wish was eaten: {after:?}"
        );
        let gained = GROUND_ACCEL_RATE * 6.6 / 120.0;
        assert!((after.y - gained).abs() < 1e-5, "{after:?}");
        assert!(after.length() > before.length());

        // The cap binds on the projection, so a body that keeps pushing
        // sideways keeps every bit of the speed it already had. That is the
        // whole reason a strafe jump can build speed at all.
        let mut v = before;
        for _ in 0..600 {
            v = accelerate(v, wishdir, 6.6, GROUND_ACCEL_RATE, 1.0 / 120.0);
        }
        assert!(
            (v.y - 6.6).abs() < 1e-4,
            "the wish direction is not capped: {v:?}"
        );
        assert_eq!(v.x, 8.0, "the perpendicular speed was eaten: {v:?}");

        // At the cap, holding the key adds nothing rather than braking, and a
        // body over the cap along the wish direction keeps its speed — that is
        // what lets a bunny hop carry into the next jump.
        assert_eq!(
            accelerate(v, wishdir, 6.6, GROUND_ACCEL_RATE, 1.0 / 120.0),
            v
        );
        let fast = vec2(0.0, 12.0);
        assert_eq!(
            accelerate(fast, wishdir, 6.6, GROUND_ACCEL_RATE, 1.0 / 120.0),
            fast
        );
    }

    /// Friction has to be a one-way street, and it has to survive being handed
    /// a body that is already stopped.
    #[test]
    fn friction_only_ever_takes_speed_away() {
        let mut v = vec2(6.6, -3.3);
        let mut last = v.length();
        let mut stopped_at = None;
        for step in 0..1200 {
            v = apply_friction(v, FRICTION, 1.0 / 120.0);
            let speed = v.length();
            assert!(speed.is_finite(), "step {step} produced {v:?}");
            assert!(
                speed <= last + 1e-6,
                "step {step} sped up: {last} -> {speed}"
            );
            if speed == 0.0 && stopped_at.is_none() {
                stopped_at = Some(step);
            }
            last = speed;
        }
        assert!(
            stopped_at.is_some(),
            "ten seconds of friction never stopped it"
        );
        assert_eq!(last, 0.0, "ten seconds of friction left {last} m/s");

        // A body at rest stays at rest: the `0 / 0` in the scaling factor would
        // have written a NaN into the velocity that nothing could clear.
        let rest = apply_friction(Vec2::ZERO, FRICTION, 1.0 / 120.0);
        assert_eq!(rest, Vec2::ZERO);
        assert!(rest.x.is_finite() && rest.y.is_finite());

        // A step long enough to eat the whole speed stops the body instead of
        // turning it around — frames hitch.
        assert_eq!(apply_friction(vec2(0.5, 0.0), FRICTION, 1.0), Vec2::ZERO);
    }

    /// The fixed step is a guard rail, not a nicety: the same second of input
    /// has to arrive in nearly the same place at 60 Hz and at 144 Hz. Friction
    /// is an Euler step and the acceleration cap binds a step at a time, so
    /// both read `dt`, and without substepping the feel drifts with the
    /// refresh rate.
    #[test]
    fn sixty_and_one_forty_four_hz_agree() {
        let level = test_level();
        let forward = MoveInput {
            forward: 1.0,
            ..Default::default()
        };
        let idle = MoveInput::default();
        let (dt_slow, dt_fast) = (1.0 / 60.0, 1.0 / 144.0);

        // Settle both bodies onto the floor first, so the measured run starts
        // from one place and one ground state at either rate.
        let mut slow = spawn();
        let mut fast = spawn();
        run(&mut slow, &level, &idle, dt_slow, 20);
        run(&mut fast, &level, &idle, dt_fast, 48);
        assert!(
            slow.on_ground && fast.on_ground,
            "the bodies did not settle"
        );
        let start = slow.pos;
        assert!(
            (fast.pos - start).length() < 1e-3,
            "settled apart: {start:?} vs {:?}",
            fast.pos
        );

        // One second of held forward.
        run(&mut slow, &level, &forward, dt_slow, 60);
        run(&mut fast, &level, &forward, dt_fast, 144);
        let (v_slow, v_fast) = (slow.horizontal_speed(), fast.horizontal_speed());
        let travel_slow = (flat(slow.pos) - flat(start)).length();
        let travel_fast = (flat(fast.pos) - flat(start)).length();
        assert!(
            (travel_slow - travel_fast).abs() / travel_slow < 0.02,
            "travel: {travel_slow} vs {travel_fast}"
        );
        assert!(
            (v_slow - v_fast).abs() / v_slow < 0.01,
            "speed: {v_slow} vs {v_fast}"
        );
        // Both against the closed form the fixed step is meant to reproduce: a
        // ramp of `1 / rate` seconds and then the cap. The discrete ramp lands
        // a hair long, because its last step is a whole step and not a partial
        // one.
        let ideal = Stance::Stand.speed() * (1.0 - 1.0 / (2.0 * GROUND_ACCEL_RATE));
        assert!(
            (travel_slow - ideal).abs() < 0.15,
            "travel {travel_slow} against the closed form {ideal}"
        );

        // Then let go and slide: friction is the part of the model most
        // sensitive to the step size, so it is where a variable frame time
        // would show up first.
        let (before_slow, before_fast) = (flat(slow.pos), flat(fast.pos));
        run(&mut slow, &level, &idle, dt_slow, 36);
        run(&mut fast, &level, &idle, dt_fast, 86);
        let slide_slow = (flat(slow.pos) - before_slow).length();
        let slide_fast = (flat(fast.pos) - before_fast).length();
        // A quarter of a metre from the cap: long enough to read as a slide,
        // short enough that the body is not still drifting when the player has
        // already stopped pushing.
        assert!(
            (0.1..0.4).contains(&slide_slow),
            "the slide left its band: {slide_slow}"
        );
        assert!(
            (slide_slow - slide_fast).abs() / slide_slow < 0.05,
            "slide: {slide_slow} vs {slide_fast}"
        );
    }

    /// Random recoil cannot be learned, so a burst has to walk the same path
    /// every time — and it has to move sideways, or "pattern" means nothing.
    #[test]
    fn burst_recoil_is_a_repeatable_pattern() {
        let (pitch, yaw) = (0.02, 0.004);
        let mut a = Player::new(Vec3::ZERO);
        let mut b = Player::new(Vec3::ZERO);
        let mut climbed = 0.0;
        for shot in 1..=10 {
            a.add_recoil(pitch, yaw);
            b.add_recoil(pitch, yaw);
            assert_eq!(a.recoil_pitch, b.recoil_pitch, "shot {shot} differs");
            assert_eq!(a.recoil_yaw, b.recoil_yaw, "shot {shot} differs");
            assert!(a.recoil_pitch > climbed, "shot {shot} did not climb");
            climbed = a.recoil_pitch;
        }
        assert!(a.recoil_yaw.abs() > 1e-3, "the pattern never went sideways");

        // The leftward walk cannot be undone by whichever sign the caller's
        // random yaw jitter happened to have: cam yaw grows to the left in this
        // engine, and shots 8..=10 walk left no matter what they are handed.
        let mut c = Player::new(Vec3::ZERO);
        for _ in 0..RECOIL_CLIMB_END {
            c.add_recoil(pitch, -yaw);
        }
        let before_left = c.recoil_yaw;
        for _ in 0..(RECOIL_LEFT_END - RECOIL_CLIMB_END) {
            c.add_recoil(pitch, -yaw);
        }
        assert!(
            c.recoil_yaw - before_left > 0.0,
            "shots {}..={} did not walk left: {before_left} -> {}",
            RECOIL_CLIMB_END + 1,
            RECOIL_LEFT_END,
            c.recoil_yaw
        );
    }

    /// A burst that pauses longer than the group gap starts over: the pattern
    /// follows trigger discipline, not the magazine.
    #[test]
    fn a_pause_starts_a_new_burst() {
        let level = test_level();
        let idle = MoveInput::default();
        let mut p = spawn();

        // Four shots. The first is the bare base value; the fourth opens the
        // climbing segment and is worth more.
        p.add_recoil(0.02, 0.0);
        let shot_one = p.recoil_pitch;
        for _ in 0..2 {
            p.add_recoil(0.02, 0.0);
        }
        let before = p.recoil_pitch;
        p.add_recoil(0.02, 0.0);
        let shot_four = p.recoil_pitch - before;
        assert!(
            shot_four > shot_one,
            "shot four should out-climb shot one: {shot_four} vs {shot_one}"
        );

        // Wait out the group gap, fire again, and the pattern is back at shot
        // one.
        run(&mut p, &level, &idle, 1.0 / 60.0, 30);
        let before = p.recoil_pitch;
        p.add_recoil(0.02, 0.0);
        let after_pause = p.recoil_pitch - before;
        assert!(
            (after_pause - shot_one).abs() < 1e-6,
            "the burst did not reset: {after_pause} vs {shot_one}"
        );
    }

    /// The climb has to survive the burst. A fallback that ran regardless of
    /// how recently the weapon fired left the muzzle at the first shot's kick
    /// for the whole magazine.
    #[test]
    fn recoil_holds_during_the_burst_and_falls_back_after_it() {
        let level = test_level();
        let idle = MoveInput::default();
        let mut p = spawn();

        p.add_recoil(0.05, 0.0);
        let peak = p.recoil_pitch;
        assert!(peak > 0.0);
        // Constant, so the delay is checked when the test binary is built
        // rather than after the first shot — and it cannot be tuned to zero.
        const _: () = assert!(RECOIL_FALLBACK_DELAY > 0.0);

        // A frame inside the fallback delay leaves the muzzle where it is.
        p.update(&level, 0.01, &idle);
        assert_eq!(p.recoil_pitch, peak, "it fell back inside the delay");

        // Sustained fire holds it up: sixty shots fired faster than the delay,
        // and the muzzle never comes down.
        let mut held = peak;
        for shot in 2..=60 {
            p.add_recoil(0.05, 0.0);
            p.update(&level, 1.0 / 60.0, &idle);
            assert!(p.recoil_pitch > held, "the climb stalled on shot {shot}");
            held = p.recoil_pitch;
        }

        // Stop firing and it comes back down on its own.
        let top = p.recoil_pitch;
        run(&mut p, &level, &idle, 1.0 / 60.0, 30);
        assert!(
            p.recoil_pitch < top * 0.5,
            "the muzzle did not fall back: {top} -> {}",
            p.recoil_pitch
        );
    }

    /// The HUD needs to know which way to draw the arc and when to stop drawing
    /// it, and "stopped" has to be an exact state rather than a value that
    /// halves forever.
    #[test]
    fn hurt_direction_fades_to_nothing() {
        let level = test_level();
        let idle = MoveInput::default();
        let mut p = spawn();
        assert!(p.hurt_dir().is_none(), "a fresh body has not been hit");

        p.hurt_from(vec3(3.0, 9.0, 4.0));
        let (dir, fresh) = p.hurt_dir().expect("just hit");
        assert!((fresh - 1.0).abs() < 1e-6, "strength {fresh}");
        assert!((dir.length() - 1.0).abs() < 1e-6, "not a bearing: {dir:?}");
        assert_eq!(dir.y, 0.0, "the arc is horizontal: {dir:?}");
        assert!(dir.x > 0.0 && dir.z > 0.0, "{dir:?}");

        // Half of the window in, half the strength.
        run(&mut p, &level, &idle, 1.0 / 60.0, 45);
        let (_, half) = p.hurt_dir().expect("still inside the window");
        assert!((half - 0.5).abs() < 0.05, "half of 1.5 s: {half}");

        // A hit from straight above carries no bearing, and that must not blank
        // an arc that is still counting down.
        p.hurt_from(vec3(0.0, 5.0, 0.0));
        assert!(p.hurt_dir().is_some(), "the arc was blanked");

        // A fresh hit resets the strength and takes over the bearing...
        p.hurt_from(vec3(-1.0, 0.0, 0.0));
        let (dir, again) = p.hurt_dir().expect("hit again");
        assert!((again - 1.0).abs() < 1e-6, "strength {again}");
        assert!(dir.x < 0.0, "{dir:?}");

        // ...and then it runs out exactly.
        run(&mut p, &level, &idle, 1.0 / 60.0, 100);
        assert!(p.hurt_dir().is_none(), "still showing {:?}", p.hurt_dir());
    }

    /// The caps are the balance, and the model is not allowed to move them:
    /// every stance, the sprint, the walk, the sights and the weapon's own
    /// move scale still arrive at the speed they always did.
    #[test]
    fn every_speed_cap_still_arrives() {
        let level = test_level();
        let stand = Stance::Stand.speed();
        let cases = [
            (
                MoveInput {
                    forward: 1.0,
                    ..Default::default()
                },
                stand,
                "stand",
            ),
            (
                MoveInput {
                    forward: 1.0,
                    sprint: true,
                    ..Default::default()
                },
                SPRINT_SPEED,
                "sprint",
            ),
            (
                MoveInput {
                    forward: 1.0,
                    stance: Stance::Crouch,
                    ..Default::default()
                },
                Stance::Crouch.speed(),
                "crouch",
            ),
            (
                MoveInput {
                    forward: 1.0,
                    stance: Stance::Prone,
                    ..Default::default()
                },
                Stance::Prone.speed(),
                "prone",
            ),
            (
                MoveInput {
                    forward: 1.0,
                    walk: true,
                    ..Default::default()
                },
                stand * 0.52,
                "walk",
            ),
            // Raising the sights drops the cap mid-stride, which is the case
            // the friction rule exists for.
            (
                MoveInput {
                    forward: 1.0,
                    aim: true,
                    ..Default::default()
                },
                AIM_SPEED,
                "aiming down sights",
            ),
            (
                MoveInput {
                    forward: 1.0,
                    speed_scale: 0.72,
                    ..Default::default()
                },
                stand * 0.72,
                "a heavy weapon",
            ),
        ];

        for (input, want, what) in cases {
            let mut p = spawn();
            run(&mut p, &level, &input, TARGET_STEP, 120);
            let got = p.horizontal_speed();
            assert!((got - want).abs() < 0.01, "{what}: {got} m/s, want {want}");
        }
    }

    /// A landing must not scrub the speed the body arrives with. The cap binds
    /// in the air as well as on the ground, so a body that comes down at the cap
    /// is already inside it — the touchdown itself has to leave that alone.
    #[test]
    fn a_landing_does_not_scrub_speed() {
        let level = test_level();
        let forward = MoveInput {
            forward: 1.0,
            ..Default::default()
        };
        let cap = Stance::Stand.speed();
        let mut p = spawn();
        // Drop in from three metres up, already at the cap and pushing forward
        // with it.
        p.pos.y = 3.0;
        p.vel = vec3(0.0, -2.0, -cap);

        let mut landed = None;
        for frame in 0..600 {
            // One physics tick per frame: the grace is counted in substeps, so
            // it needs a frame that is exactly one of them.
            p.update(&level, TARGET_STEP, &forward);
            if p.on_ground {
                landed = Some(frame);
                break;
            }
        }
        assert!(landed.is_some(), "the body never landed");
        let speed = p.horizontal_speed();
        assert!(
            (speed - cap).abs() < 1e-3,
            "the landing scrubbed speed: {speed}"
        );

        // And it holds while the key stays down.
        run(&mut p, &level, &forward, TARGET_STEP, 60);
        let after = p.horizontal_speed();
        assert!((after - cap).abs() < 1e-3, "the run decayed: {after}");
    }

    /// The cap binds in the air too, so the classic strafe jump banks nothing.
    /// That is the point: a hop is a way over something, not a faster way to
    /// travel, and a held jump key must not turn into free speed. The air still
    /// steers — the velocity turns toward the view while the strafe is held —
    /// it just cannot come out of the hop faster than it went in.
    #[test]
    fn the_air_cannot_bank_speed() {
        let level = test_level();
        let forward = MoveInput {
            forward: 1.0,
            ..Default::default()
        };
        let idle = MoveInput::default();
        let mut p = spawn();
        run(&mut p, &level, &idle, TARGET_STEP, 60);
        run(&mut p, &level, &forward, TARGET_STEP, 120);
        let grounded = p.horizontal_speed();
        assert!(
            (grounded - Stance::Stand.speed()).abs() < 0.01,
            "the run-up did not reach the cap: {grounded}"
        );

        // Jump, and hold forward and right while the view turns: the input that
        // used to compound.
        let input = MoveInput {
            forward: 1.0,
            right: 1.0,
            jump: true,
            ..Default::default()
        };
        let turn = 90.0f32.to_radians() / 240.0;
        let heading = |p: &Player| vec2(p.vel.x, p.vel.z).normalize_or_zero();
        let mut best = grounded;
        let mut steered = false;
        let mut last = heading(&p);
        for _ in 0..240 {
            p.yaw += turn;
            p.update(&level, TARGET_STEP, &input);
            best = best.max(p.horizontal_speed());
            let now = heading(&p);
            if now != Vec2::ZERO && now.dot(last) < 0.9999 {
                steered = true;
            }
            last = now;
            if p.on_ground {
                // Back on the floor: one hop is over.
                break;
            }
        }
        assert!(
            best <= grounded + 1e-3,
            "the hop banked speed: {grounded} -> {best}"
        );
        assert!(steered, "the air stopped steering");
    }

    /// The other half of "a hop is not a way to travel": letting go of the keys
    /// in mid-air slows the body down instead of leaving it to sail on. The
    /// model brakes in the air on the same rule it uses on the ground, only
    /// gentler.
    #[test]
    fn letting_go_in_the_air_brakes() {
        let level = test_level();
        let forward = MoveInput {
            forward: 1.0,
            ..Default::default()
        };
        let jump = MoveInput {
            forward: 1.0,
            jump: true,
            ..Default::default()
        };
        let idle = MoveInput::default();
        let mut p = spawn();
        run(&mut p, &level, &forward, TARGET_STEP, 120);
        p.update(&level, TARGET_STEP, &jump);
        let from = flat(p.pos);
        let mut frames = 0;
        while !p.on_ground && frames < 300 {
            p.update(&level, TARGET_STEP, &idle);
            frames += 1;
        }
        let drift = (flat(p.pos) - from).length();
        assert!(
            p.horizontal_speed() < 0.05,
            "still moving in the air: {} after {frames} frames",
            p.horizontal_speed()
        );
        assert!(drift < 1.5, "the body sailed {drift} m with no key held");
    }

    /// Not an assertion: the movement model's actual numbers — the steady speed
    /// on each kind of input, the stop distance after letting go, what a hop
    /// does to a held key. The constants are tuned against this table.
    /// `cargo test -- --ignored --nocapture`.
    #[test]
    #[ignore = "prints the movement model's numbers; run with --ignored --nocapture"]
    fn show_the_movement_numbers() {
        let level = test_level();
        let dt = 1.0 / 60.0;
        let cap = Stance::Stand.speed();
        let go = MoveInput {
            forward: 1.0,
            ..Default::default()
        };
        let sprint = MoveInput {
            forward: 1.0,
            sprint: true,
            ..Default::default()
        };
        let idle = MoveInput::default();
        println!("standing cap = {cap:.4}, sprint cap = {SPRINT_SPEED:.4}");

        // Straight run: peak and mean over the last second.
        let mut p = spawn();
        run(&mut p, &level, &idle, dt, 30);
        let (mut peak, mut sum) = (0.0f32, 0.0f32);
        for i in 0..90 {
            p.update(&level, dt, &go);
            let s = p.horizontal_speed();
            peak = peak.max(s);
            if i >= 30 {
                sum += s;
            }
        }
        println!("straight W    : peak={peak:.4}  mean={:.4}", sum / 60.0);

        // Diagonal.
        let mut p = spawn();
        let diag = MoveInput {
            forward: 1.0,
            right: -1.0,
            ..Default::default()
        };
        run(&mut p, &level, &diag, dt, 90);
        {
            let v = vec2(p.vel.x, p.vel.z);
            let fwd = vec2(-p.yaw.sin(), -p.yaw.cos());
            let right = vec2(p.yaw.cos(), -p.yaw.sin());
            let wish = (fwd - right).normalize();
            let lag = v
                .normalize_or_zero()
                .dot(wish)
                .clamp(-1.0, 1.0)
                .acos()
                .to_degrees();
            println!(
                "diagonal W+D  : speed={:.4} lag={lag:.1}deg pos=({:.2},{:.2})",
                p.horizontal_speed(),
                p.pos.x,
                p.pos.z
            );
        }

        // Turning while holding W, at three rates.
        for deg in [30.0f32, 60.0, 120.0] {
            let mut p = spawn();
            let (mut peak, mut sum) = (0.0f32, 0.0f32);
            for i in 0..90 {
                p.yaw += deg.to_radians() * dt;
                p.update(&level, dt, &go);
                let s = p.horizontal_speed();
                peak = peak.max(s);
                if i >= 30 {
                    sum += s;
                }
            }
            let v = vec2(p.vel.x, p.vel.z);
            let fwd = vec2(-p.yaw.sin(), -p.yaw.cos());
            let lag = v
                .normalize_or_zero()
                .dot(fwd)
                .clamp(-1.0, 1.0)
                .acos()
                .to_degrees();
            println!(
                "turn {deg:>5.0} deg/s: peak={peak:.4}  mean={:.4} lag={lag:.1}deg",
                sum / 60.0
            );
        }

        // Stop distance after a released run-up.
        for (label, input, frames) in [("walk", &go, 60u32), ("sprint", &sprint, 48)] {
            let mut p = spawn();
            run(&mut p, &level, input, dt, frames);
            let v0 = p.horizontal_speed();
            let from = flat(p.pos);
            let mut t = 0.0;
            while p.horizontal_speed() > 0.05 && t < 5.0 {
                p.update(&level, dt, &idle);
                t += dt;
            }
            println!(
                "{label}: release at {v0:.3} -> stop after {:.3} m / {t:.2} s",
                flat(p.pos).distance(from)
            );
        }

        // Airborne with everything released.
        let mut p = spawn();
        run(&mut p, &level, &go, dt, 60);
        let jump = MoveInput {
            forward: 1.0,
            jump: true,
            ..Default::default()
        };
        p.update(&level, dt, &jump);
        let from = flat(p.pos);
        let start = p.horizontal_speed();
        let mut steps = 0;
        while !p.on_ground && steps < 200 {
            p.update(&level, dt, &idle);
            steps += 1;
        }
        println!(
            "air, released : {start:.3} -> {:.3} after {} frames, drift {:.2} m, grounded={}",
            p.horizontal_speed(),
            steps,
            flat(p.pos).distance(from),
            p.on_ground
        );

        // Holding jump (the game reads it as held, so it auto-hops) while
        // turning: the bunny-hop chain.
        let mut p = spawn();
        let hop = MoveInput {
            forward: 1.0,
            jump: true,
            ..Default::default()
        };
        run(&mut p, &level, &hop, dt, 60);
        println!("hop+turn start: {:.3}", p.horizontal_speed());
        for second in 1..=4 {
            for _ in 0..60 {
                p.yaw += 60.0f32.to_radians() * dt;
                p.update(&level, dt, &hop);
            }
            println!("   after {second}s: {:.3}", p.horizontal_speed());
        }
    }
}
