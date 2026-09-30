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
const GROUND_ACCEL: f32 = 95.0;
const AIR_ACCEL: f32 = 17.0;
const BRAKE_ACCEL: f32 = 58.0;

/// How far the head swings sideways when leaning around cover.
const LEAN_DISTANCE: f32 = 0.42;
/// Seconds of held breath available, and how fast it drains / recovers.
const BREATH_DRAIN: f32 = 0.34;
const BREATH_RECOVER: f32 = 0.26;

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
    pub hurt_flash: f32,
    pub since_damage: f32,
    pub shake: f32,
    /// Distance fallen this frame, non-zero only on the frame of landing.
    pub landing: f32,
    /// 0..100. Drains while something cosmic is in view; recovers when it is
    /// not. Governs aim instability and how much the view writhes.
    pub sanity: f32,
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
            hurt_flash: 0.0,
            since_damage: 99.0,
            shake: 0.0,
            landing: 0.0,
            sanity: 100.0,
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
        self.pitch + self.free_pitch + self.recoil_pitch + self.sway().y + w.y
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

    pub fn add_recoil(&mut self, pitch: f32, yaw: f32) {
        self.recoil_pitch += pitch;
        self.recoil_yaw += yaw;
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

    /// Can the body occupy `stance` right here, or is there a ceiling in the way?
    pub fn can_assume(&self, level: &Level, stance: Stance) -> bool {
        level.point_free(self.pos, PLAYER_RADIUS, stance.height())
    }

    pub fn update(&mut self, level: &Level, dt: f32, input: &MoveInput) {
        self.landing = 0.0;
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

        // --- movement -------------------------------------------------------
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

        let target = vec2(wish.x, wish.z) * speed;
        let current = vec2(self.vel.x, self.vel.z);
        let delta = target - current;
        let accel = if !self.on_ground {
            AIR_ACCEL
        } else if wish == Vec3::ZERO {
            BRAKE_ACCEL
        } else {
            GROUND_ACCEL * if stance == Stance::Prone { 0.5 } else { 1.0 }
        };
        let max_delta = accel * dt;
        let new_h = if delta.length() > max_delta {
            current + delta.normalize() * max_delta
        } else {
            target
        };
        self.vel.x = new_h.x;
        self.vel.z = new_h.y;

        if input.jump && self.on_ground && stance == Stance::Stand {
            self.vel.y = JUMP_VELOCITY;
            self.on_ground = false;
        }

        self.vel.y = (self.vel.y - GRAVITY * dt).max(-MAX_FALL);

        // Deliberately unclamped: `move_and_collide` substeps internally, so a
        // fast body cannot tunnel and the caller does not need to know a safe
        // step size. The clamp that used to live here also capped the effective
        // fall speed at 0.85 / dt, which made the same drop take different
        // times at 30 fps and at 144 fps.
        let step = self.vel * dt;

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
                self.landing = -self.vel.y;
                self.vel.y = 0.0;
            }
            self.on_ground = true;
        } else {
            self.on_ground = false;
        }
        if col.ceiling && self.vel.y > 0.0 {
            self.vel.y = 0.0;
        }

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

        let decay = (-11.0 * dt).exp();
        self.recoil_pitch *= decay;
        self.recoil_yaw *= decay;
        self.hurt_flash = approach(self.hurt_flash, 0.0, dt * 2.2);
        self.shake = approach(self.shake, 0.0, dt * 2.0);

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
