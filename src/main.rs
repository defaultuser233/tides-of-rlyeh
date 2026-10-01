mod assets;
mod audio;
mod campaign;
mod combat;
mod effects;
mod enemy;
mod fog;
mod grenade;
mod hud;
mod i18n;
mod level;
mod maps;
mod mesh;
#[cfg(test)]
mod mesh_tests;
mod pickup;
mod player;
mod pointer;
mod surface;
mod util;
mod viewmodel;
mod weapon;
#[cfg(windows)]
mod win32;

use audio::Sfx;
use campaign::{Mission, Objective};
use enemy::{Attack, Enemy, Kind as EnemyKind};
use grenade::{GrenadeKind, Grenades};
use i18n::Key;
use level::Level;
use macroquad::prelude::*;
use pickup::{Pickup, PickupKind};
use player::{MoveInput, Player, Stance};
use util::{Rng, ray_aabb};
use viewmodel::ViewModel;
use weapon::{Arsenal, SlotClass, WeaponKind};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Phase {
    Menu,
    MissionSelect,
    Settings,
    Playing,
    Paused,
    Result,
}

/// Full-screen overlays that suspend the simulation but keep the world drawn.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Overlay {
    None,
    Inventory,
    Map,
    Mission,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum WheelKind {
    Grenade,
    Med,
}

struct Wheel {
    kind: WheelKind,
    selected: usize,
    held: f32,
}

const GRENADE_KINDS: [GrenadeKind; 2] = [GrenadeKind::Frag, GrenadeKind::Smoke];
/// (health restored, seconds channelled, starting count)
const MED_KINDS: [(f32, f32, u32); 3] = [(25.0, 1.2, 3), (60.0, 2.5, 1), (35.0, 1.8, 1)];

fn med_key(i: usize) -> Key {
    match i {
        0 => Key::MedBandage,
        1 => Key::MedKit,
        _ => Key::MedPainkiller,
    }
}

#[derive(Clone, Copy)]
struct Settings {
    sensitivity: f32,
    volume: f32,
    aim_toggle: bool,
    lean_toggle: bool,
    invert_y: bool,
    show_fps: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            sensitivity: 1.60,
            volume: 0.80,
            aim_toggle: true,
            lean_toggle: true,
            invert_y: false,
            show_fps: false,
        }
    }
}

const ARMOR_MAX: f32 = 50.0;

struct Game {
    phase: Phase,
    overlay: Overlay,
    level: Level,
    player: Player,
    arsenal: Arsenal,
    viewmodel: ViewModel,
    grenades: Grenades,
    enemies: Vec<Enemy>,
    pickups: Vec<Pickup>,
    effects: effects::Effects,
    /// The fog material, or `None` if the driver refused the shader.
    fog: Option<fog::Fog>,
    /// The six world textures. Held here rather than in `Level` because they
    /// outlive any one level — a mission switch rebuilds the level, not the
    /// GPU uploads.
    textures: surface::Textures,
    /// Reused by `render_world` for every per-frame mesh. `draw_mesh` copies
    /// the vertices into macroquad's own batch the moment it is called, so one
    /// buffer serves the whole frame — and, kept here rather than built per
    /// frame, it holds on to the capacity it last needed.
    scratch: Mesh,
    /// The same idea for the damage readouts: rebuilt every frame from the
    /// effects list, so a firefight does not allocate a vector per frame for
    /// labels that expire in three quarters of a second.
    damage_labels: Vec<(Vec2, f32, bool)>,
    mission: &'static Mission,
    mission_index: usize,
    rng: Rng,

    armor: f32,
    stance: Stance,
    ammo_grenades: [u32; 2],
    ammo_meds: [u32; 3],
    wheel: Option<Wheel>,
    /// Previous pointer position in window pixels, owned here so that
    /// recentring the pointer does not contaminate the delta.
    last_mouse: Vec2,
    mouse_ready: bool,
    /// True once the fire button has been released since the mission began.
    trigger_armed: bool,
    /// Latched state for toggle-style aiming.
    aim_on: bool,
    /// Sprint, as a mode rather than a held key. See `sprint_state`.
    sprint: bool,
    /// Commanded lean, -1 left .. +1 right. In toggle mode this is latched.
    lean_cmd: f32,
    heal_t: f32,
    heal_index: usize,

    spawned: u32,
    killed: u32,
    score: u32,
    shots_fired: u32,
    shots_hit: u32,
    elapsed: f32,
    spawn_timer: f32,
    flow_t: f32,
    survive_left: f32,
    objective_done: bool,
    extraction_open: bool,
    dead_timer: f32,
    /// Seconds of hit-stop left after a kill. While this is positive the world
    /// is advanced at `HITSTOP_SCALE`, and the timers that belong to the
    /// simulation (this one included) keep the real clock so the pause cannot
    /// extend itself.
    hitstop: f32,

    banner: Option<(String, f32)>,
    toast: Option<(String, f32)>,
    unlocked: usize,
    menu_sel: usize,
    result_sel: usize,
    settings_sel: usize,
    settings: Settings,
}

impl Game {
    fn new() -> Self {
        let mission = campaign::get(0);
        let mut level = Level::from_layout(&maps::campaign()[mission.layout]);
        // Only once a window exists: this uploads textures and fixes up their
        // sampling, neither of which works from a headless test.
        let textures = surface::Textures::load();
        level.dress(&textures);
        let player = Player::new(level.player_start);
        let arsenal = Arsenal::new(mission.loadout);
        let survive_left = mission.survive_seconds;
        Self {
            phase: Phase::Menu,
            overlay: Overlay::None,
            level,
            player,
            arsenal,
            viewmodel: ViewModel::new(),
            grenades: Grenades::new(),
            enemies: Vec::new(),
            pickups: Vec::new(),
            effects: effects::Effects::new(),
            textures,
            fog: fog::Fog::load(),
            scratch: mesh::empty_mesh(),
            damage_labels: Vec::new(),
            mission,
            mission_index: 0,
            rng: Rng::new(0x9E37_79B9),
            armor: 0.0,
            stance: Stance::Stand,
            ammo_grenades: [2, 1],
            ammo_meds: [MED_KINDS[0].2, MED_KINDS[1].2, MED_KINDS[2].2],
            wheel: None,
            last_mouse: Vec2::ZERO,
            mouse_ready: false,
            trigger_armed: false,
            aim_on: false,
            sprint: false,
            lean_cmd: 0.0,
            heal_t: 0.0,
            heal_index: 0,
            spawned: 0,
            killed: 0,
            score: 0,
            shots_fired: 0,
            shots_hit: 0,
            elapsed: 0.0,
            spawn_timer: 0.0,
            flow_t: 0.0,
            survive_left,
            objective_done: false,
            extraction_open: false,
            dead_timer: 0.0,
            hitstop: 0.0,
            banner: None,
            toast: None,
            unlocked: 1,
            menu_sel: 0,
            result_sel: 0,
            settings_sel: 0,
            settings: Settings::default(),
        }
    }

    fn start(&mut self, index: usize) {
        self.mission_index = index.min(campaign::count() - 1);
        self.mission = campaign::get(self.mission_index);
        let layouts = maps::campaign();
        self.level = Level::from_layout(&layouts[self.mission.layout]);
        // A retry or the next mission rebuilds the geometry; the textures are
        // already uploaded, so only the handle has to be reattached.
        self.level.dress(&self.textures);
        self.player = Player::new(self.level.player_start);
        self.player.alive = true;
        self.arsenal = Arsenal::new(self.mission.loadout);
        self.grenades = Grenades::new();
        self.enemies.clear();
        self.pickups.clear();
        self.effects.clear();
        self.armor = 0.0;
        self.stance = Stance::Stand;
        // You go in with what the mission issues; there is no resupply.
        self.ammo_grenades = [2, 1];
        self.ammo_meds = [MED_KINDS[0].2, MED_KINDS[1].2, MED_KINDS[2].2];
        self.wheel = None;
        self.mouse_ready = false;
        self.trigger_armed = false;
        self.aim_on = false;
        self.sprint = false;
        self.lean_cmd = 0.0;
        self.heal_t = 0.0;
        self.overlay = Overlay::None;
        self.spawned = 0;
        self.killed = 0;
        self.score = 0;
        self.shots_fired = 0;
        self.shots_hit = 0;
        self.elapsed = 0.0;
        self.spawn_timer = 0.5;
        self.flow_t = 0.0;
        self.survive_left = self.mission.survive_seconds;
        self.objective_done = false;
        self.extraction_open = false;
        self.dead_timer = 0.0;
        self.hitstop = 0.0;
        self.result_sel = 0;

        let s = self.level.player_start;
        self.pickups.push(Pickup::new(
            vec3(s.x + 2.0, 0.0, s.z + 1.0),
            PickupKind::Ammo,
            40.0,
        ));
        self.pickups.push(Pickup::new(
            vec3(s.x - 2.0, 0.0, s.z + 1.5),
            PickupKind::Health,
            25.0,
        ));

        self.banner = Some((level_name(&self.level), 2.8));
        self.phase = Phase::Playing;
        // Re-assert in case a focus change slipped in since startup.
        disable_ime();
    }

    fn alive_enemies(&self) -> usize {
        self.enemies.iter().filter(|e| !e.dead).count()
    }

    fn remaining(&self) -> u32 {
        self.mission.total_enemies.saturating_sub(self.spawned) + self.alive_enemies() as u32
    }

    /// Hostiles from the mission's own budget that have been killed.
    ///
    /// `killed` cannot be used for this: it also counts the spawnlings that
    /// burst out of Tentacle corpses, which were never part of
    /// `total_enemies`, so the mission panel could read `59 / 26`. Whatever the
    /// spawnlings do, this stays inside the budget.
    fn planned_kills(&self) -> u32 {
        self.spawned.saturating_sub(self.alive_enemies() as u32)
    }

    /// Returns to a playable state, swallowing whatever input the menu left
    /// behind.
    ///
    /// Two separate leaks, same cause. The click that pressed RESUME is never
    /// seen by the re-arm check in `simulate`, so the button is still down on
    /// the first frame back and the weapon fires. And the mouse baseline is
    /// stale — the pointer moved freely while the menu was up — so the first
    /// frame back applies the whole accumulated displacement as one turn.
    fn resume_play(&mut self) {
        self.phase = Phase::Playing;
        self.trigger_armed = false;
        self.mouse_ready = false;
        // Latched input is dropped on the way back in, for the same reason the
        // trigger is: whatever the player was holding when they opened the menu
        // is not something they are still asking for now.
        self.sprint = false;
    }

    fn say(&mut self, text: impl Into<String>) {
        self.toast = Some((text.into(), 1.6));
    }

    /// Starts a healing channel for the given medical item.
    fn begin_heal(&mut self, index: usize, audio: &audio::Audio) {
        if index >= MED_KINDS.len() || self.ammo_meds[index] == 0 {
            self.say(i18n::t(Key::NoMeds));
            audio.play(Sfx::UiBack);
            return;
        }
        if self.player.health >= self.player.max_health {
            self.say(i18n::t(Key::HealthFull));
            audio.play(Sfx::UiBack);
            return;
        }
        if self.heal_t > 0.0 {
            return;
        }
        self.heal_index = index;
        self.heal_t = MED_KINDS[index].1;
        self.say(format!(
            "{} — {}",
            i18n::t(Key::Healing),
            i18n::t(med_key(index))
        ));
        audio.play(Sfx::Reload);
    }

    fn throw_grenade(&mut self, index: usize, audio: &audio::Audio) {
        if index >= GRENADE_KINDS.len() || self.ammo_grenades[index] == 0 {
            self.say(i18n::t(Key::NoGrenades));
            audio.play(Sfx::UiBack);
            return;
        }
        let eye = self.player.eye();
        let dir = self.player.look_dir();
        self.grenades.throw(GRENADE_KINDS[index], eye, dir);
        self.ammo_grenades[index] -= 1;
        self.viewmodel.trigger_grenade_throw();
        audio.play(Sfx::UiMove);
    }

    fn select_slot(&mut self, i: usize) {
        self.arsenal.select(i);
    }
}

fn level_name(level: &Level) -> String {
    match i18n::lang() {
        i18n::Lang::Zh => level.name_zh.to_string(),
        i18n::Lang::En => level.name_en.to_string(),
    }
}

/// How far forward the player has to be pushing for a sprint to keep itself on.
const SPRINT_FORWARD: f32 = 0.4;

/// Hit-stop: how long a kill slows the world, and what fraction of real time
/// the world gets while it lasts. Short enough to read as impact rather than as
/// a stutter, and never a full stop — a frozen frame in an FPS reads as a
/// hitch, not as weight.
const HITSTOP_TIME: f32 = 0.05;
const HITSTOP_SCALE: f32 = 0.25;

/// How far a tracer is kept away from the camera plane, in metres.
///
/// A beam has a ring of vertices at each end, and a ring that lands on the
/// camera plane projects through the near plane and smears into a wedge across
/// half the screen. The player's own shots start at the eye, and an enemy shot
/// that connects ends at it, so both ends have to be pushed out by this much.
const TRACER_NEAR: f32 = 0.55;

/// Whether the player is sprinting this frame.
///
/// Sprint is a mode rather than a held key: Shift latches it on and it drops
/// the moment the player stops asking to run — forward released, or sights up.
/// Holding the key is worse than it sounds on this keyboard, because Shift is
/// also the IME's language toggle (see `disable_ime`) and the shortcut Windows
/// uses to offer Sticky Keys, so a key that only has to be tapped is a key that
/// stops fighting both of them.
fn sprint_state(on: bool, toggled: bool, forward: f32, aiming: bool) -> bool {
    let latched = if toggled { !on } else { on };
    latched && forward > SPRINT_FORWARD && !aiming
}

/// Stop the Windows IME from eating the game's keys.
///
/// On a Chinese Windows install Left Shift is the IME's default
/// Chinese/English toggle, and Shift is sprint here — so every sprint flipped
/// the input method and the IME started swallowing WASD. Games solve this by
/// detaching the IME context from their window; miniquad exposes exactly that
/// as `set_ime_enabled(false)`, which calls
/// `ImmAssociateContextEx(hwnd, NULL, 0)` and sets a flag it re-checks on
/// `WM_SETFOCUS` / `WM_IME_SETCONTEXT`, so the setting survives focus changes.
///
/// There is no text input in this game, so the IME stays off for the whole
/// session. A no-op off Windows.
fn disable_ime() {
    macroquad::miniquad::window::set_ime_enabled(false);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Sprint had to become a mode rather than a held key, and the parts that
    /// matter are all in the transitions: a tap has to latch, a second tap has
    /// to drop it, and letting go of forward has to drop it too — otherwise you
    /// walk away from the keyboard still flagged as running.
    #[test]
    fn sprint_latches_and_releases_itself() {
        // A tap latches it, and it stays latched while running.
        assert!(sprint_state(false, true, 1.0, false));
        assert!(sprint_state(true, false, 1.0, false));
        // A second tap drops it.
        assert!(!sprint_state(true, true, 1.0, false));
        // Releasing forward drops it, and it stays dropped: the tap was a
        // command to run *now*, not a mode that waits for the next step.
        assert!(!sprint_state(true, false, 0.0, false));
        // Nor does it survive the sights coming up.
        assert!(!sprint_state(true, false, 1.0, true));
        // Backing up is not running.
        assert!(!sprint_state(true, false, -1.0, false));
        // A tap while standing still has nothing to attach to.
        assert!(!sprint_state(false, true, 0.0, false));
    }

    /// The damage readouts are the only thing the game projects from world
    /// space back into pixels, and a mirrored or shifted projection would pass
    /// a test that only checked for a finite number. A point on the ground five
    /// metres dead ahead has to land on the centre line and below the middle,
    /// at the angle the camera's own fov puts it; the player's right has to
    /// come out right; and a point behind the camera has to project to nothing.
    #[test]
    fn the_projection_puts_the_ground_ahead_below_the_crosshair() {
        let (w, h) = (1280.0f32, 720.0f32);
        let fovy = 62.0f32.to_radians();
        let eye = vec3(0.0, 1.8, 0.0);
        let vp = view_projection(eye, eye + Vec3::NEG_Z, Vec3::Y, fovy, w, h);

        let ground = project_to_screen(&vp, vec3(0.0, 0.0, -5.0), w, h).expect("in front");
        assert!(
            (ground.x - w * 0.5).abs() < 0.5,
            "not on the centre line: {ground:?}"
        );
        // Eye 1.8 m up, 5 m out: the point sits atan(1.8 / 5) below the horizon
        // and the camera puts `f * tan(angle)` of that down the screen.
        let f = (h * 0.5) / (fovy * 0.5).tan();
        let want = h * 0.5 + f * (1.8 / 5.0);
        assert!(
            (ground.y - want).abs() < 1.0,
            "ground point at {ground:?}, expected y {want}"
        );

        // Five metres ahead and 2.5 m to the player's right: right of centre.
        // Straight out to the side would be culled, correctly — it sits on the
        // eye plane, where the perspective divide has no answer.
        let right = project_to_screen(&vp, vec3(2.5, 1.8, -5.0), w, h).expect("in front");
        assert!(right.x > w * 0.5, "the player's right came out left");

        assert!(
            project_to_screen(&vp, vec3(0.0, 1.8, 5.0), w, h).is_none(),
            "a point behind the camera was projected anyway"
        );
    }
}

fn window_conf() -> Conf {
    Conf {
        window_title: "STEEL FRONT / 钢铁前线".to_owned(),
        window_width: 1280,
        window_height: 720,
        high_dpi: true,
        sample_count: 4,
        window_resizable: true,
        ..Default::default()
    }
}

/// The colour the sky reaches at the horizon.
///
/// Shared with the fog so that distant geometry fades into the sky rather than
/// crossing it with a hard edge — two expressions that had to agree would
/// eventually stop agreeing.
fn horizon_colour(sky: Color) -> Color {
    Color::new(
        (sky.r * 0.55 + 0.12).min(1.0),
        (sky.g * 0.50 + 0.10).min(1.0),
        (sky.b * 0.45 + 0.09).min(1.0),
        1.0,
    )
}

fn draw_sky(sky: Color) {
    let h = screen_height();
    let w = screen_width();
    let bands = 40.0;
    let top = Color::new(sky.r * 0.30, sky.g * 0.32, sky.b * 0.42, 1.0);
    let bottom = horizon_colour(sky);
    for i in 0..bands as i32 {
        let t = i as f32 / bands;
        let c = Color::new(
            top.r + (bottom.r - top.r) * t,
            top.g + (bottom.g - top.g) * t,
            top.b + (bottom.b - top.b) * t,
            1.0,
        );
        draw_rectangle(0.0, h * t, w, h / bands + 1.0, c);
    }
}

/// Ray against the player's own body box, for enemy fire. Uses the current
/// stance height so crouching genuinely presents a smaller target.
fn ray_player(player: &Player, origin: Vec3, dir: Vec3) -> Option<f32> {
    use player::PLAYER_RADIUS;
    ray_aabb(
        origin,
        dir,
        vec3(
            player.pos.x - PLAYER_RADIUS,
            player.pos.y,
            player.pos.z - PLAYER_RADIUS,
        ),
        vec3(
            player.pos.x + PLAYER_RADIUS,
            player.pos.y + player.body_height(),
            player.pos.z + PLAYER_RADIUS,
        ),
    )
}

fn apply_player_damage(
    player: &mut Player,
    armor: &mut f32,
    audio: &audio::Audio,
    amount: f32,
    from: Vec3,
) {
    if !player.alive {
        return;
    }
    let absorbed = (amount * 0.65).min(*armor);
    *armor -= absorbed;
    player.damage(amount - absorbed);
    // Where the hit came from, for the HUD's directional indicator. `hurt_from`
    // reads a direction from the player out toward the source and ignores a
    // flat zero, and this is the one place damage reaches the player, so the
    // bearing can never be recorded twice for one hit.
    player.hurt_from(from - player.pos);
    audio.play(Sfx::PlayerHurt);
}

/// Returns true only when a shot was actually fired, so the caller can keep
/// the gunshot audio in step with the real rate of fire.
fn fire_player_shot(game: &mut Game, audio: &audio::Audio, pressed: bool, held: bool) -> bool {
    let s = game.arsenal.active_weapon().stats();
    // `try_fire` already applies the fire-mode rule — it takes both the rising
    // edge and the sustained state. The extra latch that used to guard this was
    // not merely redundant, it was wrong: the caller only reaches here while
    // the button is down, so the latch could never be cleared and every
    // semi-automatic weapon fired exactly once per mission.
    if !game.arsenal.active_weapon_mut().try_fire(pressed, held) {
        return false;
    }

    let eye = game.player.eye();
    let spread = game.arsenal.active_weapon().spread(
        game.player.move_ratio(),
        game.player.ads,
        game.player.spread_scale(),
    );
    let headshot = s.headshot_mult;

    // A knife is a swing, not a shot: short reach, no falloff, no tracers.
    let (rays, range) = if s.melee {
        (3, 2.6)
    } else {
        (s.pellets, weapon::MAX_RANGE)
    };
    if !s.melee {
        game.shots_fired += s.pellets;
    }

    for _ in 0..rays {
        let dir = game.rng.cone(game.player.look_dir(), spread);
        let hit = combat::hitscan(&game.level, &game.enemies, eye, dir, range);
        match hit.kind {
            combat::HitKind::World => {
                if !s.melee {
                    let n = (eye - hit.point).normalize_or_zero();
                    game.effects.spark(
                        hit.point,
                        n,
                        Color::new(1.0, 0.85, 0.5, 1.0),
                        4,
                        &mut game.rng,
                    );
                    audio.play_at(Sfx::HitWorld, hit.dist);
                }
            }
            combat::HitKind::Enemy { index, head } => {
                game.shots_hit += 1;
                let dmg = game.arsenal.active_weapon().damage_at(hit.dist)
                    * if head { headshot } else { 1.0 };
                let died = game.enemies[index].take_damage(dmg);
                game.effects.blood(
                    hit.point,
                    dir,
                    Color::new(0.55, 0.10, 0.10, 1.0),
                    &mut game.rng,
                );
                game.effects.damage_number(hit.point, dmg, head);
                if head {
                    game.effects.head_marker = 1.0;
                    audio.play(Sfx::HitHead);
                } else {
                    game.effects.hit_marker = 1.0;
                    audio.play(Sfx::HitFlesh);
                }
                if died {
                    kill_enemy(game, index, audio);
                }
                // A knife only strikes the first thing in its arc.
                if s.melee {
                    break;
                }
            }
        }
        if !s.melee {
            // Outgoing fire is the thin, pale strand. The two tracer looks are
            // built so they are told apart without colour: narrow and washed
            // out against the incoming one's wide, deep red — colour alone is
            // the first thing a colour-blind player or a washed-out monitor
            // loses. It starts out along the barrel rather than at the eye;
            // see `TRACER_NEAR`. The reach clamp is for a shot into something
            // the muzzle is already touching, where the offset would otherwise
            // start the strand beyond its own impact.
            let reach = (hit.point - eye).length();
            game.effects.tracer(
                eye + dir * TRACER_NEAR.min(reach * 0.9),
                hit.point,
                Color::new(1.0, 0.97, 0.80, 1.0),
                0.012,
            );
        }
    }

    if !s.melee {
        let jitter = game.rng.signed();
        game.player
            .add_recoil(s.recoil_pitch, s.recoil_yaw * jitter);
    }
    // Firing breaks a healing channel.
    game.heal_t = 0.0;
    true
}

fn kill_enemy(game: &mut Game, index: usize, audio: &audio::Audio) {
    game.killed += 1;
    let score = game.enemies[index].score;
    game.score += score;
    // A kill is the one moment worth a flash and a beat of slow motion. The
    // flash used to be a field that only ever decayed — nothing set it, so the
    // frame it was built for never came; it decays in `effects.update`.
    game.effects.kill_flash = 1.0;
    game.hitstop = HITSTOP_TIME;
    audio.play(Sfx::EnemyDeath);
    let at = vec3(game.enemies[index].pos.x, 0.0, game.enemies[index].pos.z);
    let spawns = game.enemies[index].spawns_on_death;
    let roll = game.rng.f32();
    if roll < 0.16 {
        game.pickups.push(Pickup::new(at, PickupKind::Health, 30.0));
    } else if roll < 0.42 {
        game.pickups.push(Pickup::new(at, PickupKind::Ammo, 45.0));
    } else if roll < 0.52 {
        game.pickups.push(Pickup::new(at, PickupKind::Armor, 35.0));
    }

    // Body horror: the corpse bursts open and smaller things crawl out, so
    // killing a Tentacle horror at point-blank range is a mistake.
    if spawns > 0 {
        let wave = game.mission.wave;
        for i in 0..spawns {
            let a = i as f32 / spawns as f32 * std::f32::consts::TAU;
            let offset = vec3(a.cos() * 1.05, 0.0, a.sin() * 1.05);
            let spot = at + offset;
            let spot = if game.level.point_free(spot, 0.26, 0.72) {
                spot
            } else {
                at
            };
            let mut spawned = Enemy::spawn(EnemyKind::Spawnling, spot, wave, &mut game.rng);
            // Otherwise the hatchling stands where it crawled out, never having
            // seen anything, and still counts as alive.
            spawned.alert_to(at, 20.0);
            game.enemies.push(spawned);
        }
        game.effects.blood(
            at + vec3(0.0, 0.7, 0.0),
            Vec3::Y,
            Color::new(0.42, 0.05, 0.16, 1.0),
            &mut game.rng,
        );
    }
}

fn pick_spawn(level: &Level, player: &Player, rng: &mut Rng) -> Vec3 {
    // Far enough that nothing materialises in your face, but close enough that
    // the fight actually arrives. Preferring the *farthest* spawn was fine on
    // the small arenas and disastrous on R'LYEH, where the nearest spawn is
    // 19 m and the farthest 65 m through a deliberate labyrinth — the first
    // fourty seconds passed with nothing happening at all.
    const MIN: f32 = 14.0;
    const IDEAL: f32 = 24.0;
    let mut best = level.spawns[0];
    let mut best_score = f32::MIN;
    for _ in 0..10 {
        let cand = level.spawns[(rng.u32() as usize) % level.spawns.len()];
        let d = vec3(cand.x - player.pos.x, 0.0, cand.z - player.pos.z).length();
        if d < MIN {
            continue;
        }
        let hidden = !level.line_of_sight(
            vec3(cand.x, 1.6, cand.z),
            vec3(player.pos.x, player.pos.y + 1.6, player.pos.z),
        );
        // Arriving unseen is worth a lot; the rest is closeness to IDEAL.
        let score = if hidden { 30.0 } else { 0.0 } - (d - IDEAL).abs();
        if score > best_score {
            best_score = score;
            best = cand;
        }
    }
    best
}

fn menu_nav(sel: &mut usize, len: usize, audio: &audio::Audio) {
    if len == 0 {
        return;
    }
    let before = *sel;
    if is_key_pressed(KeyCode::Down) {
        *sel = (*sel + 1) % len;
    }
    if is_key_pressed(KeyCode::Up) {
        *sel = (*sel + len - 1) % len;
    }
    if *sel != before {
        audio.play(Sfx::UiMove);
    }
}

fn hovered_rect(rect: &(f32, f32, f32, f32)) -> bool {
    let (mx, my) = mouse_position();
    let (x, y, w, h) = *rect;
    mx >= x && mx <= x + w && my >= y && my <= y + h
}

fn clicked_rect(rect: &(f32, f32, f32, f32)) -> bool {
    is_mouse_button_pressed(MouseButton::Left) && hovered_rect(rect)
}

/// Row `idx`, if it exists. Out-of-range rows are never hit.
fn hovered(rects: &[(f32, f32, f32, f32)], idx: usize) -> bool {
    rects.get(idx).is_some_and(hovered_rect)
}

fn clicked(rects: &[(f32, f32, f32, f32)], idx: usize) -> bool {
    rects.get(idx).is_some_and(clicked_rect)
}

#[macroquad::main(window_conf)]
async fn main() {
    // The window exists by the time this body runs, so the IME can be detached
    // before the first keystroke is read.
    disable_ime();
    // Same reason, other direction: Shift is sprint here, and five taps of it is
    // how Windows asks whether you want Sticky Keys.
    #[cfg(windows)]
    let _quiet_keys = win32::QuietKeys::new();
    i18n::set_lang(i18n::Lang::Zh);

    let fonts = hud::Fonts::load().await;
    let mut audio = audio::Audio::load().await;
    audio.set_master(0.8);

    let mut game = Game::new();
    let mut menu_rects: Vec<(f32, f32, f32, f32)>;

    loop {
        let dt = get_frame_time().min(0.05);
        // The HUD takes its accent from the arena, so this has to land before
        // any panel is drawn — menus included, where the arena is the last one
        // played.
        hud::set_accent(game.level.accent);

        match game.phase {
            Phase::Menu => {
                pointer::clip(false);
                pointer::show(true);
                let items = [
                    hud::MenuItem {
                        label: i18n::t(Key::NewCampaign),
                        enabled: true,
                    },
                    hud::MenuItem {
                        label: i18n::t(Key::SelectMission),
                        enabled: true,
                    },
                    hud::MenuItem {
                        label: i18n::t(Key::Settings),
                        enabled: true,
                    },
                    hud::MenuItem {
                        label: i18n::t(Key::Quit),
                        enabled: true,
                    },
                ];
                menu_nav(&mut game.menu_sel, items.len(), &audio);
                clear_background(Color::new(0.03, 0.04, 0.06, 1.0));
                draw_sky(Color::new(0.06, 0.07, 0.11, 1.0));
                menu_rects = hud::draw_menu(
                    &fonts,
                    i18n::t(Key::Title),
                    Some(i18n::t(Key::Subtitle)),
                    &items,
                    game.menu_sel,
                );
                for i in 0..menu_rects.len() {
                    if hovered(&menu_rects, i) {
                        game.menu_sel = i;
                    }
                }
                let enter = is_key_pressed(KeyCode::Enter) || is_key_pressed(KeyCode::KpEnter);
                let choose = |i: usize| (enter && game.menu_sel == i) || clicked(&menu_rects, i);
                if choose(0) {
                    audio.play(Sfx::UiClick);
                    game.start(0);
                } else if choose(1) {
                    audio.play(Sfx::UiClick);
                    game.phase = Phase::MissionSelect;
                    game.menu_sel = 0;
                } else if choose(2) {
                    audio.play(Sfx::UiClick);
                    game.phase = Phase::Settings;
                    game.settings_sel = 0;
                } else if choose(3) {
                    break;
                }
            }

            Phase::MissionSelect => {
                pointer::clip(false);
                pointer::show(true);
                let layouts = maps::campaign();
                let names: Vec<String> = (0..campaign::count())
                    .map(|i| {
                        let m = campaign::get(i);
                        let l = &layouts[m.layout];
                        let n = match i18n::lang() {
                            i18n::Lang::Zh => l.name_zh,
                            i18n::Lang::En => l.name_en,
                        };
                        format!("{}. {}", i + 1, n)
                    })
                    .collect();
                let items: Vec<hud::MenuItem> = names
                    .iter()
                    .enumerate()
                    .map(|(i, n)| hud::MenuItem {
                        label: n.as_str(),
                        enabled: i < game.unlocked,
                    })
                    .chain(std::iter::once(hud::MenuItem {
                        label: i18n::t(Key::Back),
                        enabled: true,
                    }))
                    .collect();

                menu_nav(&mut game.menu_sel, items.len(), &audio);
                clear_background(Color::new(0.03, 0.04, 0.06, 1.0));
                draw_sky(Color::new(0.07, 0.08, 0.12, 1.0));
                menu_rects = hud::draw_menu(
                    &fonts,
                    i18n::t(Key::SelectMission),
                    None,
                    &items,
                    game.menu_sel,
                );
                for (i, (rect, item)) in menu_rects.iter().zip(items.iter()).enumerate() {
                    if item.enabled && hovered_rect(rect) {
                        game.menu_sel = i;
                    }
                }
                let enter = is_key_pressed(KeyCode::Enter) || is_key_pressed(KeyCode::KpEnter);
                let n = items.len();
                for (i, (rect, item)) in menu_rects.iter().zip(items.iter()).enumerate() {
                    if !item.enabled || !((enter && game.menu_sel == i) || clicked_rect(rect)) {
                        continue;
                    }
                    audio.play(Sfx::UiClick);
                    if i == n - 1 {
                        game.phase = Phase::Menu;
                        game.menu_sel = 0;
                    } else {
                        game.start(i);
                    }
                    break;
                }
            }

            Phase::Settings => {
                pointer::clip(false);
                pointer::show(true);
                let on = i18n::t(Key::On);
                let off = i18n::t(Key::Off);
                let aim_label = if game.settings.aim_toggle {
                    i18n::t(Key::AimToggle)
                } else {
                    i18n::t(Key::AimHold)
                };
                let rows = [
                    format!(
                        "{}  {:.2}",
                        i18n::t(Key::Sensitivity),
                        game.settings.sensitivity
                    ),
                    format!(
                        "{}  {:.0}%",
                        i18n::t(Key::Volume),
                        game.settings.volume * 100.0
                    ),
                    format!("{}  {}", i18n::t(Key::AimMode), aim_label),
                    format!(
                        "{}  {}",
                        i18n::t(Key::LeanMode),
                        if game.settings.lean_toggle {
                            i18n::t(Key::AimToggle)
                        } else {
                            i18n::t(Key::AimHold)
                        }
                    ),
                    format!(
                        "{}  {}",
                        i18n::t(Key::InvertY),
                        if game.settings.invert_y { on } else { off }
                    ),
                    format!(
                        "{}  {}",
                        i18n::t(Key::ShowFps),
                        if game.settings.show_fps { on } else { off }
                    ),
                    format!("{}  {}", i18n::t(Key::Language), i18n::lang().label()),
                    i18n::t(Key::Back).to_string(),
                ];
                let items: Vec<hud::MenuItem> = rows
                    .iter()
                    .map(|r| hud::MenuItem {
                        label: r.as_str(),
                        enabled: true,
                    })
                    .collect();

                menu_nav(&mut game.settings_sel, items.len(), &audio);
                let left = is_key_pressed(KeyCode::Left);
                let right = is_key_pressed(KeyCode::Right);
                clear_background(Color::new(0.03, 0.04, 0.06, 1.0));
                draw_sky(Color::new(0.07, 0.08, 0.12, 1.0));
                menu_rects = hud::draw_menu(
                    &fonts,
                    i18n::t(Key::Settings),
                    None,
                    &items,
                    game.settings_sel,
                );

                if left || right {
                    let d = if right { 1.0 } else { -1.0 };
                    match game.settings_sel {
                        0 => {
                            game.settings.sensitivity =
                                (game.settings.sensitivity + d * 0.10).clamp(0.30, 5.0)
                        }
                        1 => {
                            game.settings.volume =
                                (game.settings.volume + d * 0.05).clamp(0.0, 1.0);
                            audio.set_master(game.settings.volume);
                        }
                        2 => game.settings.aim_toggle = !game.settings.aim_toggle,
                        3 => game.settings.lean_toggle = !game.settings.lean_toggle,
                        4 => game.settings.invert_y = !game.settings.invert_y,
                        5 => game.settings.show_fps = !game.settings.show_fps,
                        6 => i18n::set_lang(i18n::lang().next()),
                        _ => {}
                    }
                    audio.play(Sfx::UiMove);
                }
                let enter = is_key_pressed(KeyCode::Enter) || is_key_pressed(KeyCode::KpEnter);
                if (enter && game.settings_sel == 7) || clicked(&menu_rects, 7) {
                    audio.play(Sfx::UiBack);
                    game.phase = Phase::Menu;
                    game.menu_sel = 0;
                }
            }

            Phase::Playing => {
                if is_key_pressed(KeyCode::Escape) && game.overlay == Overlay::None {
                    audio.play(Sfx::UiBack);
                    game.phase = Phase::Paused;
                    game.menu_sel = 0;
                } else {
                    handle_overlay_keys(&mut game, &audio);
                    if game.overlay == Overlay::None {
                        pointer::clip(true);
                        pointer::show(false);
                        // Hit-stop slows the world, not the game: the pointer,
                        // the input edges and the HUD all keep the real clock,
                        // and only `simulate` sees the scaled rate. Timed here
                        // rather than inside `simulate` because it is the one
                        // timer that must not be slowed by itself.
                        let world_dt = if game.hitstop > 0.0 {
                            dt * HITSTOP_SCALE
                        } else {
                            dt
                        };
                        game.hitstop = (game.hitstop - dt).max(0.0);
                        simulate(&mut game, &mut audio, world_dt);
                    } else {
                        // The world holds still while a screen is open.
                        pointer::clip(false);
                        pointer::show(true);
                        game.effects.update(dt, &game.level);
                    }
                    render_world(&mut game, &fonts);
                }
            }

            Phase::Paused => {
                pointer::clip(false);
                pointer::show(true);
                let items = [
                    hud::MenuItem {
                        label: i18n::t(Key::Resume),
                        enabled: true,
                    },
                    hud::MenuItem {
                        label: i18n::t(Key::Restart),
                        enabled: true,
                    },
                    hud::MenuItem {
                        label: i18n::t(Key::MainMenu),
                        enabled: true,
                    },
                ];
                menu_nav(&mut game.menu_sel, items.len(), &audio);
                clear_background(Color::new(0.02, 0.03, 0.04, 1.0));
                draw_rectangle(
                    0.0,
                    0.0,
                    screen_width(),
                    screen_height(),
                    Color::new(0.0, 0.0, 0.0, 0.55),
                );
                menu_rects =
                    hud::draw_menu(&fonts, i18n::t(Key::Paused), None, &items, game.menu_sel);
                for i in 0..menu_rects.len() {
                    if hovered(&menu_rects, i) {
                        game.menu_sel = i;
                    }
                }
                let enter = is_key_pressed(KeyCode::Enter) || is_key_pressed(KeyCode::KpEnter);
                for i in 0..3 {
                    if (enter && game.menu_sel == i) || clicked(&menu_rects, i) {
                        audio.play(Sfx::UiClick);
                        match i {
                            0 => game.resume_play(),
                            1 => game.start(game.mission_index),
                            _ => {
                                game.phase = Phase::Menu;
                                game.menu_sel = 0;
                            }
                        }
                        break;
                    }
                }
            }

            Phase::Result => {
                pointer::clip(false);
                pointer::show(true);
                let success = game.objective_done && game.player.alive;
                let rows = [
                    (i18n::t(Key::Kills), format!("{}", game.killed)),
                    (i18n::t(Key::Score), format!("{}", game.score)),
                    (i18n::t(Key::Time), format_time(game.elapsed)),
                    (
                        i18n::t(Key::Accuracy),
                        if game.shots_fired == 0 {
                            "—".to_string()
                        } else {
                            format!(
                                "{:.0}%",
                                100.0 * game.shots_hit as f32 / game.shots_fired as f32
                            )
                        },
                    ),
                ];
                let has_next = success && game.mission_index + 1 < campaign::count();
                let mut labels: Vec<hud::MenuItem> = Vec::new();
                let next_label;
                if has_next {
                    next_label = i18n::t(Key::NextMission);
                    labels.push(hud::MenuItem {
                        label: next_label,
                        enabled: true,
                    });
                }
                labels.push(hud::MenuItem {
                    label: i18n::t(Key::RetryMission),
                    enabled: true,
                });
                labels.push(hud::MenuItem {
                    label: i18n::t(Key::MainMenu),
                    enabled: true,
                });

                menu_nav(&mut game.result_sel, labels.len(), &audio);
                clear_background(Color::new(0.03, 0.04, 0.06, 1.0));
                draw_sky(Color::new(0.06, 0.07, 0.11, 1.0));
                let title = if success {
                    if has_next {
                        i18n::t(Key::MissionComplete)
                    } else {
                        i18n::t(Key::CampaignCleared)
                    }
                } else if game.player.alive {
                    i18n::t(Key::MissionFailed)
                } else {
                    i18n::t(Key::YouWereKilled)
                };
                menu_rects =
                    hud::draw_result(&fonts, title, success, &rows, &labels, game.result_sel);
                for i in 0..menu_rects.len() {
                    if hovered(&menu_rects, i) {
                        game.result_sel = i;
                    }
                }
                let enter = is_key_pressed(KeyCode::Enter) || is_key_pressed(KeyCode::KpEnter);
                for i in 0..labels.len() {
                    if !((enter && game.result_sel == i) || clicked(&menu_rects, i)) {
                        continue;
                    }
                    audio.play(Sfx::UiClick);
                    let is_next = has_next && i == 0;
                    let is_retry = i == if has_next { 1 } else { 0 };
                    if is_next {
                        game.unlocked = game.unlocked.max(game.mission_index + 2);
                        game.start(game.mission_index + 1);
                    } else if is_retry {
                        game.start(game.mission_index);
                    } else {
                        game.phase = Phase::Menu;
                        game.menu_sel = 0;
                    }
                    break;
                }
            }
        }

        if game.settings.show_fps {
            fonts.text(
                &format!("{} FPS", get_fps()),
                10.0,
                screen_height() - 10.0,
                18.0,
                Color::new(0.6, 0.7, 0.8, 1.0),
            );
        }
        next_frame().await
    }
}

fn format_time(secs: f32) -> String {
    let s = secs.max(0.0) as u32;
    format!("{}:{:02}", s / 60, s % 60)
}

/// Tab / M / J and the Esc-to-close behaviour for the full-screen overlays.
fn handle_overlay_keys(game: &mut Game, audio: &audio::Audio) {
    let was_open = game.overlay != Overlay::None;
    if is_key_pressed(KeyCode::Tab) {
        game.overlay = if game.overlay == Overlay::Inventory {
            Overlay::None
        } else {
            Overlay::Inventory
        };
        audio.play(Sfx::UiClick);
    }
    if is_key_pressed(KeyCode::M) {
        game.overlay = if game.overlay == Overlay::Map {
            Overlay::None
        } else {
            Overlay::Map
        };
        audio.play(Sfx::UiClick);
    }
    if is_key_pressed(KeyCode::J) {
        game.overlay = if game.overlay == Overlay::Mission {
            Overlay::None
        } else {
            Overlay::Mission
        };
        audio.play(Sfx::UiClick);
    }
    if is_key_pressed(KeyCode::Escape) && game.overlay != Overlay::None {
        game.overlay = Overlay::None;
        audio.play(Sfx::UiBack);
    }
    // The pointer roamed free while the overlay was up; start a fresh baseline
    // so closing it does not snap the view.
    if was_open && game.overlay == Overlay::None {
        game.mouse_ready = false;
    }
}

fn simulate(game: &mut Game, audio: &mut audio::Audio, dt: f32) {
    // A dead player is a spectator. `alive` gates every input path below —
    // look, stance, movement, weapon handling, firing, grenades and medical
    // items — because the two seconds before the result screen used to be two
    // seconds of running, shooting and scoring kills as a corpse. Everything
    // from the spawning pass down still runs, so the world keeps ticking: the
    // death animation plays out, in-flight grenades land, and the death timer
    // still reaches the result screen.
    let alive = game.player.alive;
    let alt = is_key_down(KeyCode::LeftAlt) || is_key_down(KeyCode::RightAlt);
    let aiming = if !alive {
        false
    } else if game.settings.aim_toggle {
        if is_mouse_button_pressed(MouseButton::Right) {
            game.aim_on = !game.aim_on;
        }
        game.aim_on
    } else {
        is_mouse_button_down(MouseButton::Right)
    };
    let free_look = alive && is_mouse_button_down(MouseButton::Middle);

    // --- look ---------------------------------------------------------------
    // Deltas are computed here rather than with `mouse_delta_position`, because
    // the pointer is recentred every frame and owning the baseline is what lets
    // us rebase it to the centre we just moved to. Cancelling the movement
    // instead would feed the cancel back in as the next frame's delta and the
    // view would drift until it pointed at the sky.
    // A radial menu owns the pointer while it is open. Recentring — which is
    // what makes mouse look work at all — parks the cursor a few pixels from
    // the middle, so a sector test with any dead zone at all could never fire
    // and the grenade and medical wheels were unusable.
    let wheel_open = game.wheel.is_some();

    let (mx, my) = mouse_position();
    let cur = vec2(mx, my);
    if !game.mouse_ready {
        game.last_mouse = cur;
        game.mouse_ready = true;
    }
    let delta = cur - game.last_mouse;
    game.last_mouse = cur;
    if !wheel_open && let Some(landed) = pointer::recenter(cur) {
        // Rebase to where the pointer really ended up, not to a nominal centre.
        game.last_mouse = landed;
    }

    let invert = if game.settings.invert_y { -1.0 } else { 1.0 };
    // Kept in pixel units so the recentre stays exact, but normalised by the
    // window width so the feel does not change with resolution.
    let sens = game.settings.sensitivity * 2.0 / screen_width().max(1.0)
        * (1.0 - game.player.ads * 0.5)
        * match game.player.stance() {
            Stance::Stand => 1.0,
            Stance::Crouch => 0.88,
            Stance::Prone => 0.72,
        };
    // Moving right must turn right: increasing yaw turns left in this
    // convention, so both axes subtract.
    // While a menu is open the mouse steers the menu, not the view. A corpse
    // does not turn its head either.
    let look = if wheel_open || !alive {
        Vec2::ZERO
    } else {
        vec2(-delta.x * sens, -delta.y * sens * invert)
    };
    if free_look {
        game.player.free_yaw = (game.player.free_yaw + look.x).clamp(-2.6, 2.6);
        game.player.free_pitch = (game.player.free_pitch + look.y).clamp(-1.2, 1.2);
    } else {
        game.player.yaw += look.x;
        game.player.pitch = (game.player.pitch + look.y).clamp(-1.53, 1.53);
        // Pulling down against the climb has to eat the accumulated recoil, or
        // the automatic fallback stacks on top of the player's own correction
        // and lands the aim below where the burst started. Only here: in free
        // look the player is turning their head, not fighting the muzzle.
        game.player.compensate_recoil(look.y, look.x);
    }

    // --- stance -------------------------------------------------------------
    if alive && is_key_pressed(KeyCode::Z) {
        game.stance = if game.stance == Stance::Prone {
            Stance::Stand
        } else {
            Stance::Prone
        };
    }
    if alive && is_key_pressed(KeyCode::C) {
        game.stance = if game.stance == Stance::Crouch {
            Stance::Stand
        } else {
            Stance::Crouch
        };
    }

    // Lean follows the same rule as aiming, so the two never disagree: in
    // toggle mode a tap latches the lean and another tap drops it, in hold mode
    // it lasts as long as the key does. A dead body lies where it fell.
    if !alive {
        game.lean_cmd = 0.0;
    } else if game.settings.lean_toggle {
        if is_key_pressed(KeyCode::Q) {
            game.lean_cmd = if game.lean_cmd < -0.5 { 0.0 } else { -1.0 };
            audio.play(Sfx::UiMove);
        }
        if is_key_pressed(KeyCode::E) {
            game.lean_cmd = if game.lean_cmd > 0.5 { 0.0 } else { 1.0 };
            audio.play(Sfx::UiMove);
        }
    } else {
        game.lean_cmd = (is_key_down(KeyCode::E) as i32 - is_key_down(KeyCode::Q) as i32) as f32;
    }
    let lean = game.lean_cmd;

    let forward = (is_key_down(KeyCode::W) as i32 - is_key_down(KeyCode::S) as i32) as f32;
    let sprint_toggled =
        alive && (is_key_pressed(KeyCode::LeftShift) || is_key_pressed(KeyCode::RightShift));
    game.sprint = sprint_state(game.sprint, sprint_toggled, forward, game.player.ads > 0.4);
    // Sprinting forces you upright and drops any latched lean, otherwise a
    // toggled lean would silently block sprinting until you noticed.
    if alive && game.sprint && forward > SPRINT_FORWARD {
        game.stance = Stance::Stand;
        game.lean_cmd = 0.0;
    }

    // A corpse takes no input, but it is still simulated: gravity finishes
    // bringing it to the floor and the camera stops where the body is.
    let input = if alive {
        MoveInput {
            forward,
            right: (is_key_down(KeyCode::D) as i32 - is_key_down(KeyCode::A) as i32) as f32,
            jump: is_key_down(KeyCode::Space),
            sprint: game.sprint,
            aim: aiming,
            walk: is_key_down(KeyCode::CapsLock),
            lean,
            hold_breath: alt && aiming,
            free_look,
            stance: game.stance,
            speed_scale: game.arsenal.move_scale(),
        }
    } else {
        MoveInput {
            stance: game.stance,
            ..Default::default()
        }
    };
    game.player.ads_fov = game.arsenal.active_weapon().stats().ads_fov;
    game.player.update(&game.level, dt, &input);
    game.level.update_smoke(dt);

    if game.player.landing > 4.0 {
        audio.play(Sfx::Footstep);
    }

    // --- weapon selection ---------------------------------------------------
    for (i, k) in [KeyCode::Key1, KeyCode::Key2, KeyCode::Key3, KeyCode::Key4]
        .iter()
        .enumerate()
    {
        if alive && is_key_pressed(*k) {
            game.select_slot(i);
            audio.play(Sfx::UiMove);
        }
    }
    let (_, wheel_scroll) = mouse_wheel();
    if alive && wheel_scroll.abs() > 0.0 && game.wheel.is_none() {
        game.arsenal.cycle(if wheel_scroll > 0.0 { 1 } else { -1 });
    }

    // --- fire mode / magazine check / inspect --------------------------------
    if alive && is_key_pressed(KeyCode::B) {
        if alt {
            let mode = game.arsenal.active_weapon().mode();
            game.say(format!(
                "{}: {}",
                i18n::t(Key::FireModeIs),
                i18n::t(mode.label())
            ));
            audio.play(Sfx::UiClick);
        } else {
            let mode = game.arsenal.active_weapon_mut().cycle_fire_mode();
            game.say(format!(
                "{}: {}",
                i18n::t(Key::FireModeChanged),
                i18n::t(mode.label())
            ));
            audio.play(Sfx::UiClick);
        }
    }
    if alive && is_key_pressed(KeyCode::T) && alt {
        let key = game.arsenal.active_weapon().mag_estimate();
        let rounds = game.arsenal.active_weapon().mag;
        game.arsenal.active_weapon_mut().mag_check_t = 2.2;
        game.say(format!(
            "{}: {} ({})",
            i18n::t(Key::CheckingMag),
            i18n::t(key),
            rounds
        ));
        audio.play(Sfx::ReloadEmpty);
    }
    if alive && is_key_pressed(KeyCode::U) {
        game.arsenal.active_weapon_mut().start_inspect();
    }

    // --- grenades -----------------------------------------------------------
    if alive && is_key_pressed(KeyCode::G) {
        game.wheel = Some(Wheel {
            kind: WheelKind::Grenade,
            selected: 0,
            held: 0.0,
        });
    }
    // Dying with a wheel open would otherwise leave the pointer captured by a
    // menu nobody can close any more.
    if !alive {
        game.wheel = None;
    }
    if let Some(w) = &mut game.wheel {
        w.held += dt;
    }
    if is_key_down(KeyCode::G)
        && let Some(w) = &mut game.wheel
        && w.held > 0.28
        && w.kind == WheelKind::Grenade
        && let Some(s) = wheel_selection(GRENADE_KINDS.len())
    {
        w.selected = s;
    }
    if is_key_released(KeyCode::G)
        && let Some(w) = game.wheel.take()
    {
        // The pointer was free while the wheel was open; take a fresh baseline
        // so the travel does not snap the view.
        game.mouse_ready = false;
        if w.held > 0.28 {
            game.throw_grenade(w.selected, audio);
        } else {
            game.throw_grenade(0, audio);
        }
    }

    // --- medical ------------------------------------------------------------
    if alive && is_key_pressed(KeyCode::V) {
        game.wheel = Some(Wheel {
            kind: WheelKind::Med,
            selected: 0,
            held: 0.0,
        });
    }
    if is_key_down(KeyCode::V)
        && let Some(w) = &mut game.wheel
        && w.held > 0.28
        && w.kind == WheelKind::Med
        && let Some(s) = wheel_selection(MED_KINDS.len())
    {
        w.selected = s;
    }
    if is_key_released(KeyCode::V)
        && let Some(w) = game.wheel.take()
    {
        game.mouse_ready = false;
        let pick = if w.held > 0.28 {
            w.selected
        } else {
            best_med(game)
        };
        game.begin_heal(pick, audio);
    }

    // --- healing channel ----------------------------------------------------
    if game.heal_t > 0.0 {
        if !alive {
            // The channel dies with the player, and quietly: there is nobody
            // left to read a message about it.
            game.heal_t = 0.0;
        } else if game.player.horizontal_speed() > 2.5 {
            game.heal_t = 0.0;
            game.say(i18n::t(Key::HealInterrupted));
        } else {
            game.heal_t -= dt;
            if game.heal_t <= 0.0 {
                game.heal_t = 0.0;
                let amount = MED_KINDS[game.heal_index].0;
                game.player.heal(amount);
                game.ammo_meds[game.heal_index] = game.ammo_meds[game.heal_index].saturating_sub(1);
                audio.play(Sfx::PickupHealth);
            }
        }
    }

    // --- firing -------------------------------------------------------------
    // The click that left the menu is still held on the first frame of the
    // mission, which used to fire a burst into the wall every time.
    if !is_mouse_button_down(MouseButton::Left) {
        game.trigger_armed = true;
    }
    let held = is_mouse_button_down(MouseButton::Left) && game.trigger_armed;
    let pressed = is_mouse_button_pressed(MouseButton::Left) && game.trigger_armed;
    let melee = game.arsenal.active_weapon().stats().melee;
    if alive && (held || pressed) {
        if melee || game.arsenal.active_weapon().stats().infinite_ammo {
            // A knife's hit sound is played inside the shot resolution when it
            // actually connects; playing it here fired 40 times a second while
            // the button was held, hit or miss.
            fire_player_shot(game, audio, pressed, held);
        } else if game.arsenal.active_weapon().mag > 0 {
            let kind = game.arsenal.active_weapon().kind;
            // Only sound off when a round actually left the barrel. Otherwise
            // the shots come twice as fast as the weapon fires, and keep coming
            // all through a reload.
            if fire_player_shot(game, audio, pressed, held) {
                audio.play(match kind {
                    WeaponKind::Rifle => Sfx::ShotRifle,
                    WeaponKind::Smg => Sfx::ShotSmg,
                    WeaponKind::Sniper => Sfx::ShotSniper,
                    WeaponKind::Lmg => Sfx::ShotLmg,
                    _ => Sfx::ShotShotgun,
                });
            }
        } else if pressed {
            audio.play(Sfx::ReloadEmpty);
            game.arsenal.active_weapon_mut().start_reload();
        }
    }

    if alive && is_key_pressed(KeyCode::R) && game.arsenal.active_weapon_mut().start_reload() {
        audio.play(Sfx::Reload);
    }

    game.arsenal.update(dt);
    // The viewmodel's sway was tuned against `mouse_delta_position`'s
    // normalised scale, so convert back before handing it over.
    let sway_delta = delta * 2.0 / screen_width().max(1.0);
    game.viewmodel
        .update(dt, sway_delta, &game.player, &game.arsenal);

    // --- enemy spawning -----------------------------------------------------
    let mission = game.mission;
    if game.spawned < mission.total_enemies {
        game.spawn_timer -= dt;
        if game.spawn_timer <= 0.0 && game.alive_enemies() < mission.max_alive as usize {
            game.spawn_timer = (1.9 - 0.06 * mission.difficulty).max(0.7);
            let pos = pick_spawn(&game.level, &game.player, &mut game.rng);
            let roll = game.rng.u32();
            let kind: EnemyKind = campaign::roll_kind(mission, roll);
            let wave = mission.wave;
            // You should know something enormous has arrived before it sees you.
            if kind == EnemyKind::Colossus {
                game.banner = Some((i18n::t(Key::ColossusStirs).to_string(), 2.6));
            }
            let mut e = Enemy::spawn(kind, pos, wave, &mut game.rng);
            // Spawned out of sight and never having seen anything, a
            // reinforcement would otherwise stand at its post forever.
            e.alert_to(game.player.pos, 20.0);
            game.enemies.push(e);
            game.spawned += 1;
            game.effects.smoke(
                pos + vec3(0.0, 1.0, 0.0),
                Color::new(0.5, 0.8, 1.0, 0.5),
                10,
                &mut game.rng,
            );
            audio.play_at(Sfx::Spawn, 12.0);
        }
    }

    // --- enemy AI -----------------------------------------------------------
    let positions: Vec<Vec3> = game.enemies.iter().map(|e| e.pos).collect();
    let mut attacks: Vec<(Vec3, Attack)> = Vec::new();
    // A footfall is a pulse that `update` overwrites every frame, so it has to
    // be read here, on the frame it fires; the dust is emitted after the loop
    // because that is where the other fields of `game` are free again.
    let mut footfalls: Vec<Vec3> = Vec::new();
    for (i, e) in game.enemies.iter_mut().enumerate() {
        // The whole squad's positions, once — the enemy skips its own entry.
        // This used to allocate a filtered copy per enemy per frame.
        if let Some(atk) = e.update(&game.level, &game.player, &positions, i, dt, &mut game.rng) {
            attacks.push((e.pos, atk));
        }
        if e.take_footfall() {
            footfalls.push(e.pos);
        }
    }
    for at in footfalls {
        // A small, ground-hugging puff: enough that a unit crossing open ground
        // reads as walking, little enough that it never looks like a hit.
        game.effects.smoke(
            at + vec3(0.0, 0.05, 0.0),
            Color::new(0.45, 0.42, 0.36, 0.5),
            5,
            &mut game.rng,
        );
    }
    for (from, atk) in attacks {
        match atk {
            Attack::Shot {
                origin,
                dir,
                damage,
            } => {
                let wall = game
                    .level
                    .ray_world(origin, dir, weapon::MAX_RANGE)
                    .unwrap_or(weapon::MAX_RANGE);
                let mut end = origin + dir * wall;
                if let Some(t) = ray_player(&game.player, origin, dir)
                    && t > 0.02
                    && t < wall
                {
                    // `origin` is the eye the round left, not the feet `from`
                    // holds: both sit on the same column, so the flattened arc
                    // cannot tell them apart today, but the muzzle is the thing
                    // the indicator is naming.
                    apply_player_damage(&mut game.player, &mut game.armor, audio, damage, origin);
                    game.heal_t = 0.0;
                    // Stop the strand short of the eye rather than on it: a
                    // beam that ends on the camera plane smears the same way a
                    // player's own tracer does at its start (see `TRACER_NEAR`),
                    // and the last half metre is behind the crosshair anyway.
                    end = origin + dir * (t - TRACER_NEAR).max(0.05);
                }
                // Incoming fire is the wide, deep orange-red line, and it
                // carries a short bright head at the muzzle: width and shape
                // both say "that one is coming at you" before the colour does,
                // and the head marks the shooter even when the rest of the
                // tracer is lost against a bright wall.
                game.effects
                    .tracer(origin, end, Color::new(1.0, 0.32, 0.10, 1.0), 0.024);
                // The head is clamped to the shot itself, so a point-blank one
                // cannot leave a bright stub sticking out past its own hit.
                let head = (end - origin).length().min(0.6);
                game.effects.tracer(
                    origin,
                    origin + dir * head,
                    Color::new(1.0, 0.86, 0.55, 1.0),
                    0.034,
                );
                let d = (from - game.player.pos).length();
                audio.play_at(Sfx::EnemyShot, d);
            }
            Attack::Melee { damage } => {
                // A swing carries no origin of its own; the attacker's feet are
                // where it came from.
                apply_player_damage(&mut game.player, &mut game.armor, audio, damage, from);
                game.heal_t = 0.0;
            }
        }
    }

    // --- grenades -----------------------------------------------------------
    // --- sanity --------------------------------------------------------------
    // Only what you *look at* costs you: facing matters, and so does line of
    // sight. Turning away is a real defence, which is the point.
    let eye = game.player.eye();
    let look = game.player.look_dir();
    let mut drain = 0.0f32;
    for e in &game.enemies {
        if e.dead || e.sanity_drain <= 0.0 {
            continue;
        }
        let to = e.eye() - eye;
        let dist = to.length();
        if dist > 45.0 {
            continue;
        }
        let facing = to.normalize_or_zero().dot(look);
        if facing < 0.35 {
            continue;
        }
        if !game.level.line_of_sight(eye, e.eye()) {
            continue;
        }
        drain += e.sanity_drain * ((facing - 0.35) / 0.65);
    }
    if drain > 0.0 {
        game.player.sanity = (game.player.sanity - drain * dt).max(0.0);
    } else {
        game.player.sanity = (game.player.sanity + 4.0 * dt).min(100.0);
    }

    let blasts = game
        .grenades
        .update(dt, &mut game.level, &mut game.effects, &mut game.rng);
    for b in blasts {
        audio.play_at(Sfx::Explosion, (b.pos - game.player.pos).length());
        game.effects
            .smoke(b.pos, Color::new(0.9, 0.75, 0.5, 0.7), 26, &mut game.rng);
        game.effects.spark(
            b.pos,
            Vec3::Y,
            Color::new(1.0, 0.75, 0.3, 1.0),
            30,
            &mut game.rng,
        );
        // Fall off with distance and require line of sight, so corners work.
        for i in 0..game.enemies.len() {
            if game.enemies[i].dead {
                continue;
            }
            let centre = game.enemies[i].pos + vec3(0.0, game.enemies[i].height * 0.5, 0.0);
            let d = centre.distance(b.pos);
            if d > b.radius || !game.level.line_of_sight(b.pos, centre) {
                continue;
            }
            let falloff = 1.0 - (d / b.radius).powf(1.4);
            let dmg = b.damage * falloff.clamp(0.0, 1.0);
            if game.enemies[i].take_damage(dmg) {
                kill_enemy(game, i, audio);
            }
        }
        let eye = game.player.eye();
        let d = eye.distance(b.pos);
        if d < b.radius && game.level.line_of_sight(b.pos, eye) {
            let falloff = 1.0 - (d / b.radius).powf(1.4);
            // The blast has no single muzzle: the grenade's own position is the
            // source the player has to turn toward.
            apply_player_damage(
                &mut game.player,
                &mut game.armor,
                audio,
                b.damage * falloff,
                b.pos,
            );
            game.heal_t = 0.0;
        }
    }

    // --- pickups ------------------------------------------------------------
    for p in &mut game.pickups {
        p.update(dt);
    }
    let mut taken: Vec<usize> = Vec::new();
    for i in 0..game.pickups.len() {
        // Copy out of the pickup first: collecting one mutates the inventory,
        // which cannot happen while the list itself is borrowed.
        let (kind, amount, at) = {
            let p = &game.pickups[i];
            (p.kind, p.amount, p.pos)
        };
        let close = vec3(at.x - game.player.pos.x, 0.0, at.z - game.player.pos.z).length() < 1.1
            && (at.y - game.player.pos.y).abs() < 1.8;
        if !close {
            continue;
        }
        let label = match kind {
            PickupKind::Health => {
                // Same class of loss as the ammo crate below: `heal` reports how
                // much it actually restored, and at full health that is nothing,
                // so the crate used to be consumed for no gain.
                let restored = game.player.heal(amount);
                if restored <= 0.0 {
                    game.say(i18n::t(Key::HealthFull));
                    continue;
                }
                game.pickups[i].amount = amount - restored;
                audio.play(Sfx::PickupHealth);
                i18n::t(Key::PickHealth)
            }
            PickupKind::Ammo => {
                // `add_ammo` reports the rounds that actually fit. Taking none
                // of them — a full magazine, or a weapon that never consumes
                // ammunition — used to still play the sting and delete the
                // crate, quietly destroying its contents. There is no reserve
                // pool to put them in, so the crate stays where it is.
                let rounds = game.arsenal.active_weapon_mut().add_ammo(amount as u32);
                if rounds == 0 {
                    game.say(i18n::t(Key::AmmoFull));
                    continue;
                }
                // A partial take leaves the remainder behind for later.
                game.pickups[i].amount = amount - rounds as f32;
                audio.play(Sfx::PickupAmmo);
                i18n::t(Key::PickAmmo)
            }
            PickupKind::Armor => {
                let restored = (ARMOR_MAX - game.armor).clamp(0.0, amount);
                if restored <= 0.0 {
                    game.say(i18n::t(Key::ArmorFull));
                    continue;
                }
                game.armor += restored;
                game.pickups[i].amount = amount - restored;
                audio.play(Sfx::PickupAmmo);
                i18n::t(Key::PickArmor)
            }
        };
        game.say(label);
        // Ammo that could not all be carried keeps the crate in the world; the
        // rest are emptied on contact.
        if kind != PickupKind::Ammo || game.pickups[i].amount < 0.5 {
            taken.push(i);
        }
    }
    for i in taken.into_iter().rev() {
        game.pickups.remove(i);
    }

    // --- progression --------------------------------------------------------
    game.enemies.retain(|e| !e.is_removable());
    // Anything that walks into a bottomless pit is gone. Without this a fallen
    // enemy would keep falling forever and the mission could never complete.
    game.enemies
        .retain(|e| !game.level.swallowed_by_void(e.pos));
    if game.level.swallowed_by_void(game.player.pos) {
        game.player.health = 0.0;
        game.player.alive = false;
        if game.toast.is_none() {
            game.say(i18n::t(Key::AbyssTaken));
        }
    }
    game.effects.update(dt, &game.level);
    game.elapsed += dt;
    game.flow_t -= dt;
    if game.flow_t <= 0.0 {
        game.flow_t = 0.25;
        game.level.flow.update(game.player.pos);
    }

    let all_spawned = game.spawned >= mission.total_enemies;
    let cleared = all_spawned && game.alive_enemies() == 0;
    let at_extraction = game
        .level
        .extraction
        .map(|x| vec3(x.x - game.player.pos.x, 0.0, x.z - game.player.pos.z).length() < 2.2)
        .unwrap_or(false);

    match mission.objective {
        Objective::Eliminate => {
            if cleared {
                game.objective_done = true;
            }
        }
        Objective::EliminateThenExtract => {
            if cleared && !game.extraction_open {
                game.extraction_open = true;
                game.banner = Some((i18n::t(Key::ObjReach).to_string(), 2.4));
                audio.play(Sfx::ObjectiveDone);
            }
            if game.extraction_open && at_extraction {
                game.objective_done = true;
            }
        }
        Objective::Survive => {
            game.survive_left -= dt;
            if game.survive_left <= 0.0 {
                game.survive_left = 0.0;
                if !game.extraction_open {
                    game.extraction_open = true;
                    game.banner = Some((i18n::t(Key::ObjReach).to_string(), 2.4));
                    audio.play(Sfx::ObjectiveDone);
                }
                if at_extraction {
                    game.objective_done = true;
                }
            }
        }
    }

    tick_toast(game, dt);

    if game.objective_done && game.player.alive {
        game.score += mission.score_bonus;
        if game.mission_index + 1 < campaign::count() {
            game.unlocked = game.unlocked.max(game.mission_index + 2);
        }
        game.phase = Phase::Result;
        game.result_sel = 0;
        audio.play(Sfx::MissionComplete);
        return;
    }
    if !game.player.alive {
        game.dead_timer += dt;
        if game.dead_timer > 2.0 {
            game.phase = Phase::Result;
            game.result_sel = 0;
            audio.play(Sfx::MissionFailed);
        }
    }
}

fn tick_toast(game: &mut Game, dt: f32) {
    if let Some((_, t)) = &mut game.banner {
        *t -= dt;
        if *t <= 0.0 {
            game.banner = None;
        }
    }
    if let Some((_, t)) = &mut game.toast {
        *t -= dt;
        if *t <= 0.0 {
            game.toast = None;
        }
    }
}

/// Which sector of a radial menu the mouse is pointing at, if any.
fn wheel_selection(n: usize) -> Option<usize> {
    if n == 0 {
        return None;
    }
    let (mx, my) = mouse_position();
    let c = vec2(screen_width() * 0.5, screen_height() * 0.5);
    let d = vec2(mx, my) - c;
    if d.length() < 40.0 {
        return None;
    }
    // Sectors start at the top and run clockwise.
    let mut a = d.y.atan2(d.x) + std::f32::consts::FRAC_PI_2;
    if a < 0.0 {
        a += std::f32::consts::TAU;
    }
    let step = std::f32::consts::TAU / n as f32;
    Some(((a / step).floor() as usize).min(n - 1))
}

fn best_med(game: &Game) -> usize {
    // Prefer the cheapest item that will not waste healing.
    let missing = game.player.max_health - game.player.health;
    for (i, (amount, _, _)) in MED_KINDS.iter().enumerate() {
        if game.ammo_meds[i] > 0 && *amount <= missing + 10.0 {
            return i;
        }
    }
    MED_KINDS
        .iter()
        .enumerate()
        .find(|(i, _)| game.ammo_meds[*i] > 0)
        .map(|(i, _)| i)
        .unwrap_or(0)
}

/// Seconds of a damage readout's life that are not drawn.
///
/// `hud::draw_damage_numbers` has no alpha and draws at a fixed opacity, so the
/// fade has to live here. A hard cut for the last stretch reads better than
/// nothing: by then the number is already rising away from the hit (the rise is
/// `effects.rs`'s, which lifts the world point every frame), so the cut looks
/// like the readout expiring rather than blinking out mid-flight.
const DAMAGE_NUMBER_CUTOFF: f32 = 0.3;

/// A perspective view-projection built from `Camera3D`'s own constructors and
/// defaults, for the CPU-side projection macroquad 0.4.16 does not offer for
/// 3D cameras.
fn view_projection(cam: Vec3, target: Vec3, up: Vec3, fovy: f32, width: f32, height: f32) -> Mat4 {
    let aspect = width / height.max(1.0);
    // The near and far planes are `Camera3D::default`'s; the near plane only
    // has to keep the divide away from zero.
    Mat4::perspective_rh_gl(fovy, aspect, 0.01, 10000.0) * Mat4::look_at_rh(cam, target, up)
}

/// Where a world point lands in window pixels, or `None` when it is behind the
/// camera. GL's clip space gives `w = -z_view`, so a `w` at or below the near
/// plane is the point being on or behind the eye plane, where the perspective
/// divide is meaningless and the result would smear across the screen.
fn project_to_screen(view_proj: &Mat4, p: Vec3, width: f32, height: f32) -> Option<Vec2> {
    let clip = *view_proj * p.extend(1.0);
    if clip.w <= 0.01 {
        return None;
    }
    let ndc = clip.truncate() / clip.w;
    if !ndc.x.is_finite() || !ndc.y.is_finite() {
        return None;
    }
    // NDC is y-up with 1 at the top of the screen; window pixels are y-down.
    Some(vec2(
        (ndc.x * 0.5 + 0.5) * width,
        (0.5 - ndc.y * 0.5) * height,
    ))
}

/// Projects this frame's damage readouts out of the world and hands them to the
/// HUD. The list is rebuilt in the buffer the game owns, so the projection —
/// not the allocation — is what runs per frame.
fn draw_damage_numbers(game: &mut Game, fonts: &hud::Fonts, view_proj: &Mat4) {
    game.damage_labels.clear();
    for &(at, amount, left, head) in &game.effects.damage_numbers {
        if left <= DAMAGE_NUMBER_CUTOFF {
            continue;
        }
        if let Some(screen) = project_to_screen(view_proj, at, screen_width(), screen_height()) {
            game.damage_labels.push((screen, amount, head));
        }
    }
    hud::draw_damage_numbers(fonts, &game.damage_labels);
}

fn render_world(game: &mut Game, fonts: &hud::Fonts) {
    let shake = game.player.shake_offset();
    let eye = game.player.eye();
    let dir = game.player.look_dir();
    let fovy = game.player.fovy();

    // Camera roll is applied by tilting the up vector, which is how leaning
    // reads as leaning rather than as the world rotating.
    let right = vec3(dir.z, 0.0, -dir.x).normalize_or_zero();
    let up = dir.cross(right).normalize_or_zero();
    let roll = game.player.lean_roll();
    let up_tilted = (up * roll.cos() + right * roll.sin()).normalize_or_zero();
    let cam = eye - right * shake.x * 0.0025 - up * shake.y * 0.0025;

    clear_background(game.level.sky);
    draw_sky(game.level.sky);
    // macroquad 0.4.16 has no `Camera3D::world_to_screen` — its projection
    // helpers are `Camera2D`-only — so the frame's view-projection is built
    // here, with the same constructors and defaults this camera is made of.
    // The damage readouts are the one thing on screen that has to come back
    // out of the world.
    let view_proj = view_projection(
        cam,
        cam + dir,
        up_tilted,
        fovy,
        screen_width(),
        screen_height(),
    );
    set_camera(&Camera3D {
        position: cam,
        target: cam + dir,
        up: up_tilted,
        fovy,
        ..Default::default()
    });
    if let Some(gl_fog) = &game.fog {
        gl_fog.begin(horizon_colour(game.level.sky), game.level.half);
    }
    for mesh in &game.level.meshes {
        // An arena that uses none of a material still gets a mesh for it, so
        // skip the empty batches rather than issue a draw call for nothing.
        if !mesh.indices.is_empty() {
            draw_mesh(mesh);
        }
    }
    // One scratch mesh for all of it: `draw_mesh` copies the vertices into
    // macroquad's batch as it is called, so nothing downstream still refers to
    // this buffer by the next iteration.
    for e in &game.enemies {
        if e.dead && e.death_t > 1.9 {
            continue;
        }
        e.write_mesh(&mut game.scratch);
        draw_mesh(&game.scratch);
    }
    for p in &game.pickups {
        p.write_mesh(&mut game.scratch);
        draw_mesh(&game.scratch);
    }
    game.grenades.write_mesh(&game.level, &mut game.scratch);
    draw_mesh(&game.scratch);
    game.effects.write_mesh(&mut game.scratch);
    draw_mesh(&game.scratch);
    // Everything in the world is behind us now; the viewmodel and the HUD are
    // drawn in the 2D pass and are not fogged — a weapon in your hands is not
    // twenty metres away.
    fog::Fog::end();
    set_default_camera();

    let madness = (1.0 - game.player.sanity_ratio()) * 0.7;
    hud::draw_vignette(
        game.player.hurt_flash.max(madness),
        game.player.health < 30.0 || game.player.sanity_ratio() < 0.25,
    );
    hud::draw_kill_flash(game.effects.kill_flash);
    // The one place the two yaw conventions meet. `hurt_dir` points from the
    // player at what hit them; the HUD's bearing is where that lands relative
    // to the view, 0 dead ahead and positive to the player's right. The game's
    // yaw grows to the left and the bearing grows to the right, so the
    // subtraction runs `cam_yaw - yaw_of(dir)`. Check it at yaw 0: the player
    // faces -Z and their right is +X, `yaw_of(+X)` is `atan2(-1, 0) = -PI/2`,
    // and `0 - (-PI/2) = +PI/2` — positive, i.e. right ✓.
    if let Some((dir, strength)) = game.player.hurt_dir() {
        let arcs = [(game.player.cam_yaw() - util::yaw_of(dir), strength)];
        hud::draw_hurt_arcs(&arcs);
    }
    draw_sanity_bar(fonts, game.player.sanity_ratio());
    if game.player.alive {
        game.viewmodel.draw(&game.player, &game.arsenal);
    }
    draw_damage_numbers(game, fonts, &view_proj);
    draw_hud_state(game, fonts);
    draw_wheel_overlay(game, fonts);

    match game.overlay {
        Overlay::Inventory => draw_inventory_overlay(game, fonts),
        Overlay::Map => draw_map_overlay(game, fonts),
        Overlay::Mission => draw_mission_overlay(game, fonts),
        Overlay::None => {}
    }
}

fn draw_wheel_overlay(game: &Game, fonts: &hud::Fonts) {
    let Some(w) = &game.wheel else { return };
    if w.held <= 0.28 {
        return;
    }
    let centre = vec2(screen_width() * 0.5, screen_height() * 0.5);
    let radius = screen_height() * 0.19;
    match w.kind {
        WheelKind::Grenade => {
            let items: Vec<(String, u32, bool)> = GRENADE_KINDS
                .iter()
                .enumerate()
                .map(|(i, k)| {
                    (
                        i18n::t(match k {
                            GrenadeKind::Frag => Key::GrenadeFrag,
                            GrenadeKind::Smoke => Key::GrenadeSmoke,
                        })
                        .to_string(),
                        game.ammo_grenades[i],
                        game.ammo_grenades[i] > 0,
                    )
                })
                .collect();
            hud::draw_wheel(
                fonts,
                i18n::t(Key::GrenadeWheel),
                &items,
                centre,
                radius,
                wheel_selection(items.len()).unwrap_or(w.selected),
            );
        }
        WheelKind::Med => {
            let items: Vec<(String, u32, bool)> = MED_KINDS
                .iter()
                .enumerate()
                .map(|(i, _)| {
                    (
                        i18n::t(med_key(i)).to_string(),
                        game.ammo_meds[i],
                        game.ammo_meds[i] > 0,
                    )
                })
                .collect();
            hud::draw_wheel(
                fonts,
                i18n::t(Key::MedWheel),
                &items,
                centre,
                radius,
                wheel_selection(items.len()).unwrap_or(w.selected),
            );
        }
    }
}

fn draw_hud_state(game: &Game, fonts: &hud::Fonts) {
    let weapon = game.arsenal.active_weapon();
    let slots: Vec<(&str, bool)> = SlotClass::ALL
        .iter()
        .enumerate()
        .map(|(i, _)| {
            let tag = game.arsenal.slot(i).map(|w| w.kind.tag()).unwrap_or("--");
            (
                tag,
                i == game.arsenal.active && game.arsenal.slot(i).is_some(),
            )
        })
        .collect();
    let objective = i18n::t(game.mission.objective.key());
    let sub = game.mission.objective.sub_key().map(i18n::t).unwrap_or("");

    let state = hud::HudState {
        health: game.player.health,
        max_health: game.player.max_health,
        armor: game.armor,
        score: game.score,
        kills: game.killed,
        elapsed: game.elapsed,
        enemies_left: game.remaining() as usize,
        objective,
        objective_sub: if game.extraction_open {
            i18n::t(Key::ObjReach)
        } else {
            sub
        },
        weapon_name: i18n::t(weapon.kind.name()),
        mag: weapon.mag,
        mag_size: weapon.stats().mag_size,
        reload_progress: if weapon.is_reloading() {
            Some(weapon.reload_progress())
        } else {
            None
        },
        slots,
        spread: weapon.spread(
            game.player.move_ratio(),
            game.player.ads,
            game.player.spread_scale(),
        ),
        hit_marker: game.effects.hit_marker,
        headshot_marker: game.effects.head_marker,
        hurt: game.player.hurt_flash,
        low_health: game.player.health < 30.0,
        prompt: game.toast.as_ref().map(|(s, _)| s.as_str()),
        banner: game
            .banner
            .as_ref()
            .map(|(s, t)| (s.as_str(), (*t / 2.8).clamp(0.0, 1.0))),
        locked: !game.objective_done,
    };
    hud::draw_hud(fonts, &state);
}

/// Drawn by us rather than by `hud.rs`, whose `HudState` predates sanity.
fn draw_sanity_bar(fonts: &hud::Fonts, ratio: f32) {
    if ratio > 0.995 {
        return;
    }
    let (w, h) = (screen_width(), screen_height());
    let ui = hud::ui_scale();
    let bar_w = 240.0 * ui;
    let bar_h = 6.0 * ui;
    let x = w * 0.5 - bar_w * 0.5;
    let y = h - 92.0 * ui;
    let colour = hud::sanity_colour(ratio);
    draw_rectangle(
        x - ui,
        y - ui,
        bar_w + 2.0 * ui,
        bar_h + 2.0 * ui,
        Color::new(0.05, 0.06, 0.08, 0.8),
    );
    draw_rectangle(x, y, bar_w * ratio, bar_h, colour);
    fonts.text_c(
        i18n::t(Key::Sanity),
        w * 0.5,
        y - 8.0 * ui,
        17.0 * ui,
        colour,
    );
}

fn draw_inventory_overlay(game: &Game, fonts: &hud::Fonts) {
    let name_of =
        |i: usize| -> Option<&'static str> { game.arsenal.slot(i).map(|w| i18n::t(w.kind.name())) };
    let grenades: Vec<(String, u32)> = GRENADE_KINDS
        .iter()
        .enumerate()
        .map(|(i, k)| {
            let label = i18n::t(match k {
                GrenadeKind::Frag => Key::GrenadeFrag,
                GrenadeKind::Smoke => Key::GrenadeSmoke,
            });
            (label.to_string(), game.ammo_grenades[i])
        })
        .collect();
    let meds: Vec<(String, u32)> = MED_KINDS
        .iter()
        .enumerate()
        .map(|(i, _)| (i18n::t(med_key(i)).to_string(), game.ammo_meds[i]))
        .collect();
    let s = hud::InventoryState {
        primary: name_of(0),
        secondary: name_of(1),
        pistol: name_of(2),
        melee: name_of(3),
        active_slot: game.arsenal.active,
        grenades: &grenades,
        meds: &meds,
        armour: game.armor,
        health: game.player.health,
    };
    hud::draw_inventory(fonts, &s);
}

fn draw_map_overlay(game: &Game, fonts: &hud::Fonts) {
    let boxes: Vec<(Vec2, Vec2)> = game
        .level
        .solids
        .iter()
        .map(|b| (vec2(b.min.x, b.min.z), vec2(b.max.x, b.max.z)))
        .collect();
    let half = game
        .level
        .solids
        .iter()
        .fold(1.0f32, |acc, b| acc.max(b.max.x.abs()).max(b.max.z.abs()))
        .max(1.0);
    let mut markers: Vec<hud::MapMarker> = Vec::new();
    if let Some(x) = game.level.extraction {
        markers.push(hud::MapMarker {
            pos: vec2(x.x, x.z),
            color: Color::new(0.35, 0.80, 1.0, 1.0),
            size: 7.0,
            label: Some('E'),
        });
    }
    // Only hostiles that are actually aware of the player show up, so the map
    // does not hand you a free wallhack.
    for e in &game.enemies {
        if e.dead || e.aware_t > 6.0 {
            continue;
        }
        markers.push(hud::MapMarker {
            pos: vec2(e.pos.x, e.pos.z),
            color: Color::new(0.85, 0.18, 0.16, 1.0),
            size: 6.0,
            label: None,
        });
    }
    hud::draw_map(
        fonts,
        &boxes,
        half,
        (vec2(game.player.pos.x, game.player.pos.z), game.player.yaw),
        &markers,
    );
}

fn draw_mission_overlay(game: &Game, fonts: &hud::Fonts) {
    let objective = i18n::t(game.mission.objective.key()).to_string();
    let progress = match game.mission.objective {
        Objective::Survive => format!(
            "{} / {}",
            format_time(game.survive_left),
            format_time(game.mission.survive_seconds)
        ),
        _ => format!("{} / {}", game.planned_kills(), game.mission.total_enemies),
    };
    let obj_rows = vec![
        hud::PanelRow {
            label: i18n::t(Key::Objective),
            value: objective,
        },
        hud::PanelRow {
            label: i18n::t(Key::Hostiles),
            value: format!("{}", game.remaining()),
        },
        hud::PanelRow {
            label: i18n::t(Key::Time),
            value: progress,
        },
    ];
    let stat_rows = vec![
        hud::PanelRow {
            label: i18n::t(Key::Kills),
            value: format!("{}", game.killed),
        },
        hud::PanelRow {
            label: i18n::t(Key::Score),
            value: format!("{}", game.score),
        },
        hud::PanelRow {
            label: i18n::t(Key::Time),
            value: format_time(game.elapsed),
        },
        hud::PanelRow {
            label: i18n::t(Key::Accuracy),
            value: if game.shots_fired == 0 {
                "—".to_string()
            } else {
                format!(
                    "{:.0}%",
                    100.0 * game.shots_hit as f32 / game.shots_fired as f32
                )
            },
        },
    ];
    let sections: Vec<(&str, Vec<hud::PanelRow>)> = vec![
        (i18n::t(Key::Objectives), obj_rows),
        (i18n::t(Key::Statistics), stat_rows),
    ];
    let footer = format!(
        "{}   ·   {}   ·   {}",
        i18n::t(Key::HintTab),
        i18n::t(Key::HintMap),
        i18n::t(Key::HintClose)
    );
    hud::draw_panel(
        fonts,
        &format!(
            "{} — {}",
            i18n::t(Key::MissionPanel),
            level_name(&game.level)
        ),
        &sections,
        &footer,
    );
}
