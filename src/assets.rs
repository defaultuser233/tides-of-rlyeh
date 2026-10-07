//! The one place that knows where a file lives.
//!
//! Every asset the game ships is compiled into the binary, so a copied
//! executable runs from anywhere with nothing beside it. That choice has a
//! known cost — changing one texture rebuilds and relinks the whole binary, and
//! a large enough payload is awkward to ship — and this module is what keeps it
//! reversible. Nothing outside here names a path, so moving the files out to
//! disk, or into a packed archive beside the executable, is a change to
//! [`bytes`] and nothing else. The one exception is [`override_path`], which
//! names where a replacement *would* be found; it is here rather than in the
//! mixer for exactly the reason above.
//!
//! # The payload has a budget
//!
//! Embedding is what makes the distribution one file, and it stops being the
//! right answer somewhere around a few tens of megabytes: changing one texture
//! relinks the whole binary, and a download that size is awkward to hand to
//! anyone. The total is therefore pinned by
//! `tests::the_payload_stays_within_its_budget`, which fails with the size and
//! the largest contributor when it is passed. That failure is not a bug — it is
//! the moment to choose between the two ways out, both of which are the change
//! to [`bytes`] described above:
//!
//! - a **packed archive** beside the executable, so what ships is still two
//!   files and nothing has to be laid out on disk;
//! - an **asset directory** beside it, whose paths must be resolved from the
//!   executable's own directory — never from the working directory, which is
//!   the trap this project fell into once already (see the note below).
//!
//! Raising the budget is also allowed. Doing it without writing down why is not.
//!
//! `Asset` has one variant per file rather than taking a path as a string, so a
//! renamed or missing file stays a *compile* error. The alternative is a silent
//! fallback at runtime, which this project has already been through once: the
//! font and the sounds used to be loaded from a relative `assets/` path, which
//! works under `cargo run` and quietly fails for a released executable launched
//! from anywhere else.

/// Every file the game embeds, one variant each.
///
/// The sound variants are never constructed when the `sound` feature is off —
/// that build has no mixer to load them into. It is the only configuration in
/// which an arm of this enum can be dead, and it is a compile check rather than
/// a shipping one, so the attribute is narrower than the alternative of
/// threading `#[cfg]` through nineteen variants and their match arms.
#[cfg_attr(not(feature = "sound"), allow(dead_code))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Asset {
    /// The UI font, a subset of 霞鹜文楷 covering the strings in `i18n`.
    UiFont,

    /// The six world textures, one per `Surface`.
    TexConcreteFloor,
    TexConcreteTiles,
    TexMetalPlate,
    TexMetalRusty,
    TexMetalCorrugated,
    TexCrateSide,

    /// The curated sound effects. The effects without an asset are synthesised
    /// at load time and never touch this enum.
    SfxShotRifle,
    SfxShotShotgun,
    SfxShotSmg,
    SfxShotSniper,
    SfxShotLmg,
    SfxEnemyShot,
    SfxHitFlesh,
    SfxHitWorld,
    /// A round landing on a material that answers for itself, chosen from the
    /// surface the wall is dressed in. `SfxHitWorld` is what a wall with no
    /// material of its own still plays.
    SfxHitConcrete,
    SfxHitMetal,
    SfxHitMetalHeavy,
    SfxHitWood,
    SfxHitGlass,
    SfxHitHead,
    SfxEnemyDeath,
    SfxReload,
    SfxReloadEmpty,
    SfxFootstep,
    SfxPlayerHurt,
    SfxUiClick,
    SfxUiMove,
    SfxUiBack,
    SfxExplosion,
    /// The same bang at the other end of the arena. A report heard across the
    /// map has lost its crack and kept its tail, which is a thing attenuation
    /// alone cannot do.
    SfxExplosionDistant,
    SfxExplosionLarge,
    SfxSpawn,
    SfxHealing,
    SfxEliteArrival,
    SfxBossArrival,
    SfxSanityCritical,
    SfxPickupHealth,
    SfxPickupAmmo,
    SfxObjectiveDone,
    SfxMissionComplete,
    SfxMissionFailed,
}

impl Asset {
    /// Every variant, for tests and for anything that wants to walk the set.
    #[cfg(test)]
    pub const ALL: [Asset; 42] = [
        Asset::UiFont,
        Asset::TexConcreteFloor,
        Asset::TexConcreteTiles,
        Asset::TexMetalPlate,
        Asset::TexMetalRusty,
        Asset::TexMetalCorrugated,
        Asset::TexCrateSide,
        Asset::SfxShotRifle,
        Asset::SfxShotShotgun,
        Asset::SfxShotSmg,
        Asset::SfxShotSniper,
        Asset::SfxShotLmg,
        Asset::SfxEnemyShot,
        Asset::SfxHitFlesh,
        Asset::SfxHitWorld,
        Asset::SfxHitConcrete,
        Asset::SfxHitMetal,
        Asset::SfxHitMetalHeavy,
        Asset::SfxHitWood,
        Asset::SfxHitGlass,
        Asset::SfxHitHead,
        Asset::SfxEnemyDeath,
        Asset::SfxReload,
        Asset::SfxReloadEmpty,
        Asset::SfxFootstep,
        Asset::SfxPlayerHurt,
        Asset::SfxUiClick,
        Asset::SfxUiMove,
        Asset::SfxUiBack,
        Asset::SfxExplosion,
        Asset::SfxExplosionDistant,
        Asset::SfxExplosionLarge,
        Asset::SfxSpawn,
        Asset::SfxHealing,
        Asset::SfxEliteArrival,
        Asset::SfxBossArrival,
        Asset::SfxSanityCritical,
        Asset::SfxPickupHealth,
        Asset::SfxPickupAmmo,
        Asset::SfxObjectiveDone,
        Asset::SfxMissionComplete,
        Asset::SfxMissionFailed,
    ];
}

/// Where a replacement for one of the sounds sits on disk, if the player has
/// put one there.
///
/// This is the one path in the game that is not a compile-time constant, and it
/// is deliberate: the shipped effects are embedded, but a file dropped into
/// `assets/sounds/` still wins over the compiled-in copy, so the sounds stay
/// replaceable without a rebuild. It lives here because this module is the only
/// one that knows the layout — the mixer used to build the string itself, which
/// is how the rule this module states came to have an exception nobody had
/// written down (review R10).
///
/// Only a build with the mixer in it has anything to look this up for.
#[cfg(feature = "sound")]
pub fn override_path(name: &str) -> String {
    format!("assets/sounds/{name}")
}

/// The bytes of an asset, compiled into the binary.
///
/// Deliberately the only function in the game that names an asset path.
pub const fn bytes(asset: Asset) -> &'static [u8] {
    match asset {
        Asset::UiFont => include_bytes!("../assets/fonts/ui_zh.ttf"),

        Asset::TexConcreteFloor => include_bytes!("../assets/textures/concrete_floor.png"),
        Asset::TexConcreteTiles => include_bytes!("../assets/textures/concrete_tiles.png"),
        Asset::TexMetalPlate => include_bytes!("../assets/textures/metal_plate.png"),
        Asset::TexMetalRusty => include_bytes!("../assets/textures/metal_rusty.png"),
        Asset::TexMetalCorrugated => include_bytes!("../assets/textures/metal_corrugated.png"),
        Asset::TexCrateSide => include_bytes!("../assets/textures/crate_side.png"),

        Asset::SfxShotRifle => include_bytes!("../assets/sounds/shot_rifle.ogg"),
        Asset::SfxShotShotgun => include_bytes!("../assets/sounds/shot_shotgun.ogg"),
        Asset::SfxShotSmg => include_bytes!("../assets/sounds/shot_smg.ogg"),
        Asset::SfxShotSniper => include_bytes!("../assets/sounds/shot_sniper.ogg"),
        Asset::SfxShotLmg => include_bytes!("../assets/sounds/shot_lmg.ogg"),
        Asset::SfxEnemyShot => include_bytes!("../assets/sounds/enemy_shot.ogg"),
        Asset::SfxHitFlesh => include_bytes!("../assets/sounds/hit_flesh.ogg"),
        Asset::SfxHitWorld => include_bytes!("../assets/sounds/hit_world.ogg"),
        // The four surface impacts. They were kept in `sounds/variants/` while
        // the level could not say what a wall was made of; now that it can, they
        // are shipped sounds like any other and live beside them.
        Asset::SfxHitConcrete => include_bytes!("../assets/sounds/impact_concrete.ogg"),
        Asset::SfxHitMetal => include_bytes!("../assets/sounds/impact_metal_light.ogg"),
        Asset::SfxHitMetalHeavy => include_bytes!("../assets/sounds/impact_metal_heavy.ogg"),
        Asset::SfxHitWood => include_bytes!("../assets/sounds/impact_wood.ogg"),
        // The bottle's shatter. It was the last take left over from the pack
        // downloads: nothing in the game broke until a molotov did.
        Asset::SfxHitGlass => include_bytes!("../assets/sounds/impact_glass.ogg"),
        Asset::SfxHitHead => include_bytes!("../assets/sounds/hit_head.ogg"),
        Asset::SfxEnemyDeath => include_bytes!("../assets/sounds/enemy_death.ogg"),
        Asset::SfxReload => include_bytes!("../assets/sounds/reload.ogg"),
        Asset::SfxReloadEmpty => include_bytes!("../assets/sounds/reload_empty.ogg"),
        Asset::SfxFootstep => include_bytes!("../assets/sounds/footstep.ogg"),
        Asset::SfxPlayerHurt => include_bytes!("../assets/sounds/player_hurt.ogg"),
        Asset::SfxUiClick => include_bytes!("../assets/sounds/ui_click.ogg"),
        Asset::SfxUiMove => include_bytes!("../assets/sounds/ui_move.ogg"),
        Asset::SfxUiBack => include_bytes!("../assets/sounds/ui_back.ogg"),
        Asset::SfxExplosion => include_bytes!("../assets/sounds/explosion.ogg"),
        // `explosionCrunch_000` is the shipped bang; these two are its siblings
        // from the same pack, picked for the ends of the range it cannot cover:
        // the longest crunch in the pack for a blast at arm's length, and the
        // sub-bass take for one heard across the arena.
        Asset::SfxExplosionDistant => include_bytes!("../assets/sounds/explosion_distant.ogg"),
        Asset::SfxExplosionLarge => include_bytes!("../assets/sounds/explosion_large.ogg"),
        Asset::SfxSpawn => include_bytes!("../assets/sounds/spawn.ogg"),
        Asset::SfxMissionFailed => include_bytes!("../assets/sounds/mission_failed.ogg"),
        Asset::SfxMissionComplete => include_bytes!("../assets/sounds/mission_complete.wav"),
        Asset::SfxObjectiveDone => include_bytes!("../assets/sounds/objective_done.wav"),
        Asset::SfxPickupAmmo => include_bytes!("../assets/sounds/pickup_ammo.wav"),
        Asset::SfxPickupHealth => include_bytes!("../assets/sounds/pickup_health.ogg"),
        Asset::SfxSanityCritical => include_bytes!("../assets/sounds/sanity_critical.wav"),
        Asset::SfxBossArrival => include_bytes!("../assets/sounds/boss_arrival.ogg"),
        Asset::SfxEliteArrival => include_bytes!("../assets/sounds/elite_arrival.ogg"),
        Asset::SfxHealing => include_bytes!("../assets/sounds/healing.ogg"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The realistic mistake in a table this long is an arm pointing at the
    /// wrong file: `TexMetalRusty` and `TexMetalPlate` are one word apart, and a
    /// copy-pasted line looks exactly like a correct one. Nothing else in the
    /// suite would notice — the textures all decode, and a sound is still a
    /// sound. Distinct contents catch it.
    #[test]
    fn every_asset_is_a_different_file() {
        for (i, a) in Asset::ALL.iter().enumerate() {
            assert!(!bytes(*a).is_empty(), "{a:?} is empty");
            for b in &Asset::ALL[i + 1..] {
                assert!(bytes(*a) != bytes(*b), "{a:?} and {b:?} are the same file");
            }
        }
    }

    /// Every byte the executable carries, whether or not anything loads it.
    fn embedded_bytes() -> usize {
        Asset::ALL.iter().map(|a| bytes(*a).len()).sum()
    }

    /// The embedded payload has a budget, and this is what makes it a decision
    /// rather than a surprise.
    ///
    /// Embedding is what makes the distribution one file. It stops being the
    /// right answer somewhere around a few tens of megabytes, and the failure
    /// this asserts is not a bug: it is the moment to pick a packed archive
    /// beside the executable (still two files to copy; the index is a change to
    /// `bytes` and nothing else) or an asset directory beside it (paths resolved
    /// from the executable's own directory, never the working directory — the
    /// trap this project has been through once). Raising the budget is allowed
    /// too; writing down why is the part that is not optional.
    #[test]
    fn the_payload_stays_within_its_budget() {
        const BUDGET: usize = 16 * 1024 * 1024;
        let total = embedded_bytes();
        let (biggest, largest) = Asset::ALL
            .iter()
            .map(|a| (a, bytes(*a).len()))
            .max_by_key(|(_, size)| *size)
            .expect("the asset table is not empty");
        assert!(
            total <= BUDGET,
            "the embedded payload is {total} B ({:.1} MiB) over a budget of {BUDGET} B, \
             and {biggest:?} alone is {largest} B ({:.0}% of it). This is the moment to \
             choose, not a bug: a packed archive beside the executable, or an asset \
             directory beside it whose paths are resolved from the executable's own \
             directory and never from the working directory. Raising the budget is the \
             third option — write down why.",
            total as f64 / 1048576.0,
            largest as f64 * 100.0 / total.max(1) as f64,
        );
        assert!(total > 0, "the payload measures zero");
    }
    /// Nothing sits in `assets/sounds/` that the executable does not carry.
    ///
    /// The directory means "shipped": every file in it is `include_bytes!`d and
    /// every one has a slot. A take parked there that nothing loads reads as a
    /// shipped sound to the next person, which is exactly how two takes that must
    /// *not* be wired back in ended up sitting in it for a day (MANIFEST §2d).
    ///
    /// The check is by content rather than by name, because the names are the
    /// slots' business (`audio::Sfx::file_name`) and `Asset` deliberately has no
    /// idea what it is called. A stray copy is the same problem as a stray take.
    #[test]
    fn every_sound_beside_the_others_is_shipped() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/sounds");
        let mut seen = 0;
        for entry in std::fs::read_dir(&dir).expect("assets/sounds is part of the repo") {
            let path = entry.expect("a readable directory entry").path();
            if !path.is_file() {
                continue;
            }
            let data = std::fs::read(&path).expect("a readable sound file");
            assert!(
                Asset::ALL.iter().any(|a| bytes(*a) == data.as_slice()),
                "{} is in assets/sounds/ and nothing embeds it",
                path.display()
            );
            seen += 1;
        }
        // How many assets *are* sounds, sniffed by container rather than counted:
        // a number here would have to be edited every time one is added, and the
        // font and the six textures are the only assets that are not Ogg or WAV.
        let sounds = Asset::ALL
            .iter()
            .filter(|a| {
                let bytes = bytes(**a);
                bytes.starts_with(b"OggS") || bytes.starts_with(b"RIFF")
            })
            .count();
        assert_eq!(
            seen, sounds,
            "assets/sounds/ has {seen} files for {sounds} sound assets"
        );
    }
}
