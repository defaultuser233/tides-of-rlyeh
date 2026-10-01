//! The one place that knows where a file lives.
//!
//! Every asset the game ships is compiled into the binary, so a copied
//! executable runs from anywhere with nothing beside it. That choice has a
//! known cost — changing one texture rebuilds and relinks the whole binary, and
//! a large enough payload is awkward to ship — and this module is what keeps it
//! reversible. Nothing outside here names a path, so moving the files out to
//! disk, or into a packed archive beside the executable, is a change to
//! [`bytes`] and nothing else.
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
    SfxSpawn,
}

impl Asset {
    /// Every variant, for tests and for anything that wants to walk the set.
    #[cfg(test)]
    pub const ALL: [Asset; 26] = [
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
        Asset::SfxSpawn,
    ];
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
        Asset::SfxSpawn => include_bytes!("../assets/sounds/spawn.ogg"),
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
}
