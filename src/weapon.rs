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

const SEMI: &[FireMode] = &[FireMode::Single];
const AUTO_SEMI: &[FireMode] = &[FireMode::Auto, FireMode::Single];
const RIFLE_MODES: &[FireMode] = &[FireMode::Auto, FireMode::Burst, FireMode::Single];
const SMG_MODES: &[FireMode] = &[FireMode::Auto, FireMode::Burst, FireMode::Single];

#[derive(Clone, Copy)]
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

    pub fn class(self) -> SlotClass {
        self.stats().class
    }

    pub fn stats(self) -> WeaponStats {
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
        match self {
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
}

pub struct Weapon {
    pub kind: WeaponKind,
    pub mag: u32,
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
}

impl Weapon {
    pub fn new(kind: WeaponKind) -> Self {
        let s = kind.stats();
        Self {
            kind,
            mag: s.mag_size,
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
        }
    }

    pub fn stats(&self) -> WeaponStats {
        self.kind.stats()
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

    pub fn add_ammo(&mut self, rounds: u32) -> u32 {
        if self.stats().infinite_ammo {
            return 0;
        }
        let want = self.stats().mag_size.saturating_sub(self.mag).min(rounds);
        self.mag += want;
        want
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
        if self.stats().infinite_ammo || self.is_reloading() || self.mag >= self.stats().mag_size {
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
                self.mag = s.mag_size;
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
    pub fn new(loadout: [Option<WeaponKind>; 2]) -> Self {
        let mut ars = Self {
            slots: [None, None, None, None],
            active: 0,
            switch_t: 0.0,
        };
        for (i, k) in loadout.into_iter().enumerate() {
            // A melee weapon never takes a long-gun slot — that is what slot 4
            // is for — and the sidearm is always carried.
            if let Some(k) = k.filter(|k| k.class() != SlotClass::Melee) {
                ars.slots[i] = Some(Weapon::new(k));
            }
        }
        ars.slots[2] = Some(Weapon::new(WeaponKind::Pistol));
        ars.slots[3] = Some(Weapon::new(WeaponKind::Knife));
        ars
    }

    pub fn slot(&self, i: usize) -> Option<&Weapon> {
        self.slots.get(i).and_then(|w| w.as_ref())
    }

    /// The active weapon, falling back to the first occupied slot.
    pub fn active_weapon(&self) -> &Weapon {
        self.slots
            .get(self.active)
            .and_then(|w| w.as_ref())
            .or_else(|| self.slots.iter().flatten().next())
            .expect("arsenal always carries at least a sidearm")
    }

    pub fn active_weapon_mut(&mut self) -> &mut Weapon {
        let i = if self.slots[self.active].is_some() {
            self.active
        } else {
            self.slots.iter().position(|w| w.is_some()).unwrap_or(0)
        };
        self.slots[i].as_mut().expect("slot checked above")
    }

    pub fn select(&mut self, i: usize) {
        if i >= self.slots.len() || i == self.active || self.slots[i].is_none() {
            return;
        }
        self.active = i;
        self.switch_t = 0.34;
        if let Some(w) = self.slots[i].as_mut() {
            w.reload_t = 0.0;
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

#[cfg(test)]
mod tests {
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
    /// a mission asks for.
    #[test]
    fn the_sidearm_and_knife_are_always_present() {
        for mission in crate::campaign::MISSIONS.iter() {
            let ars = Arsenal::new(mission.loadout);
            assert_eq!(
                ars.slot(SlotClass::Pistol as usize).map(|w| w.kind),
                Some(WeaponKind::Pistol),
                "mission without a sidearm"
            );
            assert_eq!(
                ars.slot(SlotClass::Melee as usize).map(|w| w.kind),
                Some(WeaponKind::Knife),
                "mission without a knife"
            );
        }
    }
}
