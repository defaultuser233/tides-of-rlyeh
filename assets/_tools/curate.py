"""Curate the downloaded Kenney / Poly Haven packs into assets/.

Sound files are named after the `Sfx::file_name()` slots in `src/audio.rs`, so
dropping them into `assets/sounds/` makes the engine pick them up automatically
(see `load_effect`). Slots whose built-in synth is a deliberate musical jingle
(`objective_done`, `mission_complete`, `mission_failed`, the three pickups) are
left alone on purpose.

Model files keep their original Kenney names; the role each one fills is
recorded in MANIFEST.md. Each model folder gets the pack's own `Textures/`
subfolder so the `.mtl` `map_Kd` references resolve without editing.
"""

import shutil
from pathlib import Path

ROOT = Path(r"D:/Documents/rust-fps/assets")
DL = ROOT / "_downloads"
BLASTER = DL / "kenney_blaster-kit_2.1/Models/OBJ format"
BLOCKY = DL / "kenney_blocky-characters_20/Models/OBJ format"
TEX = DL / "textures"

# Kenney blaster -> the game's WeaponKind
WEAPONS = {
    "blaster-a": "rifle",
    "blaster-b": "lmg",
    "blaster-h": "shotgun",
    "blaster-j": "smg",
    "blaster-p": "sniper",
}

# Kenney character -> the game's enemy Kind
CHARACTERS = {
    "character-b": "grunt",
    "character-c": "rusher",
    "character-h": "heavy",
    "character-j": "sniper",
    "character-q": "alt",
    "character-r": "alt",
}

PROPS = ["crate-small", "crate-medium", "crate-wide"]

# destination slot name -> (pack, source stem)
SOUNDS = {
    # --- guns -------------------------------------------------------------
    "shot_rifle": ("sci-fi", "laserSmall_000"),
    "shot_shotgun": ("sci-fi", "laserLarge_000"),
    "shot_smg": ("sci-fi", "laserSmall_002"),
    "shot_sniper": ("sci-fi", "laserLarge_004"),
    "shot_lmg": ("sci-fi", "laserRetro_002"),
    "enemy_shot": ("sci-fi", "laserRetro_000"),
    # --- impacts ----------------------------------------------------------
    "hit_flesh": ("impact", "impactPunch_medium_000"),
    "hit_world": ("impact", "impactPlate_medium_000"),
    "hit_head": ("impact", "impactMetal_light_001"),
    "enemy_death": ("impact", "impactSoft_heavy_003"),
    "player_hurt": ("impact", "impactPunch_heavy_000"),
    # --- gear -------------------------------------------------------------
    "reload": ("ui", "switch4"),
    "reload_empty": ("ui", "click3"),
    "footstep": ("impact", "footstep_concrete_000"),
    # --- interface --------------------------------------------------------
    "ui_click": ("ui", "click1"),
    "ui_move": ("ui", "rollover1"),
    "ui_back": ("ui", "click2"),
    # --- set pieces -------------------------------------------------------
    "explosion": ("sci-fi", "explosionCrunch_000"),
    "spawn": ("sci-fi", "forceField_000"),
}

# Extra material-specific one-shots the single `hit_world` slot cannot express.
# Deliberately *not* named after a slot, so nothing here is loaded by accident.
VARIANTS = {
    "impact_metal_light": ("impact", "impactMetal_light_000"),
    "impact_metal_heavy": ("impact", "impactMetal_heavy_000"),
    "impact_concrete": ("impact", "impactPlate_light_000"),
    "impact_wood": ("impact", "impactPlank_medium_000"),
    "impact_glass": ("impact", "impactGlass_medium_000"),
    "footstep_concrete_02": ("impact", "footstep_concrete_002"),
    "explosion_large": ("sci-fi", "explosionCrunch_003"),
    "explosion_deep": ("sci-fi", "lowFrequency_explosion_000"),
    "explosion_distant": ("sci-fi", "lowFrequency_explosion_001"),
    "enemy_shot_alt": ("sci-fi", "laserRetro_003"),
}

PACKS = {
    "sci-fi": DL / "kenney_sci-fi-sounds/Audio",
    "impact": DL / "kenney_impact-sounds/Audio",
    "ui": DL / "kenney_ui-audio/Audio",
}

TEXTURES = {
    "metal_plate.png": "metal_plate.png",
    "rusty_metal_02.png": "metal_rusty.png",
    "corrugated_iron.png": "metal_corrugated.png",
    "concrete_floor.png": "concrete_floor.png",
    "concrete_tiles_02.png": "concrete_tiles.png",
    "container_side.png": "crate_side.png",
}


def copy_model_set(stems, src_dir, dst_dir, texture_names):
    dst_dir.mkdir(parents=True, exist_ok=True)
    tdir = dst_dir / "Textures"
    tdir.mkdir(exist_ok=True)
    for stem in stems:
        for ext in (".obj", ".mtl"):
            src = src_dir / f"{stem}{ext}"
            if not src.exists():
                raise SystemExit(f"missing source: {src}")
            shutil.copy2(src, dst_dir / src.name)
    for name in texture_names:
        src = src_dir / "Textures" / name
        if not src.exists():
            raise SystemExit(f"missing texture: {src}")
        shutil.copy2(src, tdir / name)


def copy_sounds(mapping, dst_dir):
    dst_dir.mkdir(parents=True, exist_ok=True)
    copied = {}
    for dest_stem, (pack, src_stem) in mapping.items():
        src = PACKS[pack] / f"{src_stem}.ogg"
        if not src.exists():
            raise SystemExit(f"missing sound: {src}")
        dest = dst_dir / f"{dest_stem}.ogg"
        if dest.exists():
            raise SystemExit(f"sound name collision: {dest}")
        shutil.copy2(src, dest)
        copied[dest_stem] = (pack, src_stem)
    return copied


def main():
    copy_model_set(WEAPONS, BLASTER, ROOT / "models/weapons", ["colormap.png"])
    copy_model_set(
        CHARACTERS,
        BLOCKY,
        ROOT / "models/characters",
        [f"texture-{c.split('-')[1]}.png" for c in CHARACTERS],
    )
    copy_model_set(PROPS, BLASTER, ROOT / "models/props", ["colormap.png"])

    live = copy_sounds(SOUNDS, ROOT / "sounds")
    extra = copy_sounds(VARIANTS, ROOT / "sounds/variants")

    # Poly Haven ships 1024px PNGs that weigh ~5 MB each; 512px is plenty for
    # repeated tiling on arena surfaces and keeps the asset tree small.
    from PIL import Image

    tex = ROOT / "textures"
    tex.mkdir(exist_ok=True)
    for src_name, dst_name in TEXTURES.items():
        im = Image.open(TEX / src_name).convert("RGB")
        im.resize((512, 512), Image.LANCZOS).save(
            tex / dst_name, "PNG", optimize=True, compress_level=9
        )

    print(f"models: weapons={len(WEAPONS)} characters={len(CHARACTERS)} props={len(PROPS)}")
    print(f"sounds: {len(live)} engine slots + {len(extra)} variants")
    print(f"textures: {len(TEXTURES)}")


if __name__ == "__main__":
    main()
