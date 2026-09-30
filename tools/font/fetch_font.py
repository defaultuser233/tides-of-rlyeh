#!/usr/bin/env python3
"""Build the bundled Simplified-Chinese UI font for rust-fps.

Produces ``assets/fonts/ui_zh.ttf``: a subset of LXGW WenKai that carries the
characters the game UI needs, in TrueType ``glyf`` outline format (never
CFF/PostScript outlines), plus the matching OFL license file.

Source font
-----------
    LXGW WenKai (霞鹜文楷) 1.511 -- lxgw <calxgw2018@gmail.com>
    https://github.com/lxgw/LxgwWenKai
    SIL Open Font License 1.1 (OFL-1.1), original TrueType ``glyf`` outlines.

Upstream GitHub releases are unreachable from this machine, so the font is
taken from the Debian package ``fonts-lxgw-wenkai`` (1.511+repack-1) served by
public Debian mirrors.  Debian ships the unmodified upstream TTFs, so no
outline conversion is involved: the result stays natively ``glyf``.

Why ``glyf`` matters
--------------------
macroquad rasterizes text through ``fontdue``.  Keeping the outlines in the
native TrueType ``glyf`` table avoids any CFF/PostScript round-trip and is the
format fontdue's outline builder is built around.  ``--verify`` asserts the
table is present, so a regression cannot slip through.

OFL compliance
--------------
OFL-1.1 reserves the font names "霞鹜" and "LXGW".  A subset is a Modified
Version, so this script renames the family (see ``rename_for_ofl``) and leaves
no reserved name in any identity name record.  The original copyright notice
and license records are preserved, and the full license text is written next
to the font as ``LICENSE-OFL-1.1.txt`` -- both are obligations of OFL-1.1.

Usage
-----
    python -m pip install fonttools brotli
    python tools/font/fetch_font.py

The 60 MB Debian package is cached in ``tools/font/.cache/`` (self-ignoring),
so re-runs with the same options are offline and fast.
"""

from __future__ import annotations

import argparse
import hashlib
import io
import lzma
import tarfile
import urllib.request
from pathlib import Path

from fontTools import subset
from fontTools.ttLib import TTFont

REPO_ROOT = Path(__file__).resolve().parents[2]
OUT_FONT = REPO_ROOT / "assets" / "fonts" / "ui_zh.ttf"
OUT_LICENSE = REPO_ROOT / "assets" / "fonts" / "LICENSE-OFL-1.1.txt"
CACHE_DIR = Path(__file__).resolve().parent / ".cache"

# --- source package ---------------------------------------------------------
DEB_NAME = "fonts-lxgw-wenkai_1.511+repack-1_all.deb"
DEB_SHA256 = "5a4fe76b63927f01129ec8e0be48f511e4ca6b430188e7555982eb0b60d9236b"
DEB_MIRRORS = (
    "https://mirrors.tuna.tsinghua.edu.cn/debian/pool/main/f/fonts-lxgw-wenkai/",
    "https://mirrors.aliyun.com/debian/pool/main/f/fonts-lxgw-wenkai/",
    "https://mirrors.ustc.edu.cn/debian/pool/main/f/fonts-lxgw-wenkai/",
)
# The Debian package carries six faces; Regular is the one the UI uses.
DEB_TTF_MEMBER = "./usr/share/fonts/truetype/lxgw-wenkai/LXGWWenKai-Regular.ttf"
DEB_COPYRIGHT_MEMBER = "./usr/share/doc/fonts-lxgw-wenkai/copyright"
TTF_SHA256 = "be1d5ee371b25f23ca613c3142d6293328db6253ef0b0c09857e9f30657514a8"

SRC_FONT_NAME = "LXGW WenKai"
SRC_FONT_VERSION = "1.511"
SRC_FONT_URL = "https://github.com/lxgw/LxgwWenKai"

# Debian ships 1.511 while upstream is further along; the script pins the exact
# artefact it was verified against so a mirror update cannot silently change
# the output.
DEB_VERSION = "1.511+repack-1"

# --- output identity (OFL: no "LXGW" / "霞鹜" in a Modified Version) --------
NEW_FAMILY = "RustFPS UI"
NEW_STYLE = "Regular"
NEW_FULL_NAME = f"{NEW_FAMILY} {NEW_STYLE}"
NEW_PS_NAME = "RustFPSUI-Regular"

# --- size budget ------------------------------------------------------------
SIZE_BUDGET = 2_900_000  # bytes; the stated ceiling is 3 MB

# --- character set ----------------------------------------------------------
# ASCII printable, required by the UI (menus, HUD numbers, key names).
ASCII_PRINTABLE = range(0x0020, 0x007F)

# CJK punctuation explicitly required by the spec.
REQUIRED_PUNCT = "，。！？：；、（）「」《》…—"

# Extra punctuation and symbols that Chinese game UI text tends to need.
# Codepoints missing from the source font are dropped automatically.
EXTRA_SYMBOLS = (
    "　"  # ideographic space
    "·×÷±°′″§¶©®™€¥£¢№"
    "–―‘’“”†‡•‰‱※〜"
    "←→↑↓↔⇒⇔"
    "■□▲△▼▽◆◇○●★☆♠♣♥♦✓✗"
    "①②③④⑤⑥⑦⑧⑨⑩"
    "＋－＝＜＞％＆＃＠＊／＼｜～＾＿｀｛｝［］（）：；，．！？"
)
# Fullwidth ASCII block (U+FF01-U+FF5E) is a compact, well-defined add-on.
FULLWIDTH_ASCII = range(0xFF01, 0xFF5F)

# GB2312-80 hanzi rows, extracted from the Python codec rather than hardcoded:
#   Level 1 (常用字): rows 0xB0-0xD7 -> 3755 characters
#   Level 2 (次常用字): rows 0xD8-0xF7 -> 3008 characters
GB2312_LEVEL1_ROWS = range(0xB0, 0xD8)
GB2312_LEVEL2_ROWS = range(0xD8, 0xF8)


def log(msg: str) -> None:
    print(msg, flush=True)


def gb2312_hanzi(rows) -> list[int]:
    """Codepoints of the GB2312 hanzi in the given row range, in GB2312 order."""
    out: list[int] = []
    for hi in rows:
        for lo in range(0xA1, 0xFF):
            try:
                out.append(ord(bytes([hi, lo]).decode("gb2312")))
            except UnicodeDecodeError:
                pass  # 5 of the 3760 Level-1 slots are unassigned
    return out


def sha256_file(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as fh:
        for chunk in iter(lambda: fh.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


# ---------------------------------------------------------------------------
# source acquisition
# ---------------------------------------------------------------------------
def download_deb(dest: Path) -> None:
    last_err: Exception | None = None
    for base in DEB_MIRRORS:
        url = base + DEB_NAME.replace("+", "%2B")
        log(f"  GET {url}")
        try:
            req = urllib.request.Request(url, headers={"User-Agent": "rust-fps-font-fetch"})
            with urllib.request.urlopen(req, timeout=180) as resp:
                total = int(resp.headers.get("Content-Length") or 0)
                tmp = dest.with_suffix(".part")
                got = 0
                with tmp.open("wb") as fh:
                    while True:
                        chunk = resp.read(1 << 20)
                        if not chunk:
                            break
                        fh.write(chunk)
                        got += len(chunk)
                        if total:
                            pct = 100 * got / total
                            print(f"\r    {got/1048576:6.1f}/{total/1048576:.1f} MB ({pct:5.1f}%)",
                                  end="", flush=True)
                print()
            if DEB_SHA256 and sha256_file(tmp) != DEB_SHA256:
                raise ValueError(f"sha256 mismatch for {url}")
            tmp.replace(dest)
            log(f"  cached {dest.name} ({dest.stat().st_size/1048576:.1f} MB)")
            return
        except Exception as exc:  # try the next mirror
            print()
            log(f"  failed: {type(exc).__name__}: {exc}")
            last_err = exc
    raise SystemExit(f"could not download {DEB_NAME} from any mirror: {last_err}")


def read_ar_members(blob: bytes) -> dict[str, bytes]:
    """Split a Unix `ar` archive (what a .deb is) into {name: bytes}."""
    if blob[:8] != b"!<arch>\n":
        raise ValueError("not an ar archive")
    members: dict[str, bytes] = {}
    off = 8
    while off + 60 <= len(blob):
        header = blob[off:off + 60]
        name = header[0:16].decode("ascii", "replace").strip()
        size = int(header[48:58].decode("ascii", "replace").strip() or "0")
        members[name.rstrip("/")] = blob[off + 60:off + 60 + size]
        off += 60 + size + (size & 1)  # members are 2-byte aligned
    return members


def extract_from_deb(deb_path: Path, member: str) -> bytes:
    """Pull one file out of a .deb, decompressing data.tar.<xz|gz> as needed."""
    members = read_ar_members(deb_path.read_bytes())
    data_name = next(name for name in members if name.startswith("data.tar"))
    payload = members[data_name]

    if data_name.endswith(".xz"):
        payload = lzma.decompress(payload)
    elif data_name.endswith(".gz"):
        import gzip
        payload = gzip.decompress(payload)
    elif not data_name.endswith(".tar"):
        raise ValueError(f"unexpected data member: {data_name}")

    with tarfile.open(fileobj=io.BytesIO(payload)) as tf:
        handle = tf.extractfile(member)
        if handle is None:
            raise ValueError(f"{member} not in {data_name}")
        return handle.read()


def sha256_bytes(blob: bytes) -> str:
    return hashlib.sha256(blob).hexdigest()


def source_font_bytes(cache_dir: Path) -> tuple[bytes, bytes]:
    """Return (ttf_bytes, deb_copyright_bytes), downloading on a cold cache."""
    cache_dir.mkdir(parents=True, exist_ok=True)
    # A .gitignore of "*" keeps the 60 MB cache out of git without the project
    # having to know about it.
    (cache_dir / ".gitignore").write_text("*\n", encoding="utf-8")

    ttf_path = cache_dir / "LXGWWenKai-Regular.ttf"
    copyright_path = cache_dir / "copyright.txt"

    if (ttf_path.exists() and copyright_path.exists()
            and sha256_file(ttf_path) == TTF_SHA256):
        log("  reusing cached source font")
        return ttf_path.read_bytes(), copyright_path.read_bytes()

    deb_path = cache_dir / DEB_NAME
    if not deb_path.exists() or sha256_file(deb_path) != DEB_SHA256:
        log(f"downloading {DEB_NAME} ({DEB_VERSION})")
        download_deb(deb_path)

    log(f"  extracting {DEB_TTF_MEMBER.rsplit('/', 1)[-1]} from the .deb")
    ttf = extract_from_deb(deb_path, DEB_TTF_MEMBER)
    if sha256_bytes(ttf) != TTF_SHA256:
        raise SystemExit("extracted TTF does not match the pinned sha256")
    ttf_path.write_bytes(ttf)

    copyright_txt = extract_from_deb(deb_path, DEB_COPYRIGHT_MEMBER)
    copyright_path.write_bytes(copyright_txt)
    return ttf, copyright_txt


# ---------------------------------------------------------------------------
# character set assembly
# ---------------------------------------------------------------------------
def source_characters() -> set[int]:
    """Every non-ASCII character that appears in the game's Rust sources.

    A safety net: if the UI ever shows a character outside the standard sets
    below, it still ends up in the font.
    """
    found: set[int] = set()
    src = REPO_ROOT / "src"
    if not src.is_dir():
        return found
    for path in sorted(src.rglob("*.rs")):
        for ch in path.read_text(encoding="utf-8", errors="ignore"):
            if ord(ch) > 0x7F:
                found.add(ord(ch))
    return found


def spec_required_codepoints() -> set[int]:
    """What the spec demands: ASCII, the listed CJK punctuation, GB2312 Level 1."""
    cps = set(ASCII_PRINTABLE)
    cps |= {ord(c) for c in REQUIRED_PUNCT}
    cps |= set(gb2312_hanzi(GB2312_LEVEL1_ROWS))
    return cps


def optional_codepoints() -> set[int]:
    """Nice-to-have additions: fullwidth forms, UI symbols, the game's own text."""
    cps = set(FULLWIDTH_ASCII)
    cps |= {ord(c) for c in EXTRA_SYMBOLS}
    cps |= source_characters()
    return cps


def available(codepoints, src_cmap) -> set[int]:
    return {cp for cp in codepoints if cp in src_cmap}


# ---------------------------------------------------------------------------
# subsetting
# ---------------------------------------------------------------------------
def subset_options():
    opts = subset.Options()
    # The font has no hinting; turning it off documents that and is a no-op.
    opts.hinting = False
    opts.desubroutinize = False
    opts.glyph_names = False
    opts.post_format = 3.0
    opts.notdef_outline = True          # keep a visible fallback glyph
    opts.recalc_bounds = False
    opts.recalc_timestamp = False       # byte-reproducible output
    # The subsetter would otherwise drop name IDs 13/14 and the zh-CN records.
    # OFL-1.1 requires the copyright and license notices to survive.
    opts.name_IDs = ["*"]
    opts.name_languages = ["*"]
    opts.name_legacy = True
    # The UI is horizontal and fontdue does no complex shaping, so layout
    # tables are dead weight.  `kern` and vertical metrics are unused too.
    opts.layout_features = []
    opts.drop_tables += ["GSUB", "GPOS", "GDEF", "vhea", "vmtx", "gasp"]
    # Codepoints absent from the source are reported by verify_coverage.
    opts.ignore_missing_unicodes = True
    return opts


def rename_for_ofl(font) -> None:
    """Strip OFL Reserved Font Names by giving the subset its own identity."""
    name = font["name"]
    # Every record that names the font, across all platforms and languages.
    for name_id in (1, 2, 3, 4, 6, 16, 17, 18, 20, 21):
        name.removeNames(nameID=name_id)

    unique_id = f"{NEW_PS_NAME};subset"
    records = (
        (1, NEW_FAMILY),
        (2, NEW_STYLE),
        (3, unique_id),
        (4, NEW_FULL_NAME),
        (6, NEW_PS_NAME),
        (16, NEW_FAMILY),   # typographic family
        (17, NEW_STYLE),    # typographic subfamily
    )
    for name_id, value in records:
        name.setName(value, name_id, 3, 1, 0x409)  # Windows, Unicode BMP, en-US
        name.setName(value, name_id, 1, 0, 0)      # Macintosh, Roman, en


def build_subset(src_bytes: bytes, codepoints: list[int], out_path: Path | None):
    """Subset `src_bytes` to `codepoints`; returns (font, serialized_bytes)."""
    # Options.recalc_timestamp only reaches the subsetter's own writer, not the
    # TTFont used here -- the constructor would re-stamp head.modified on every
    # save and make the output depend on the clock.  Turn that off.
    font = TTFont(io.BytesIO(src_bytes), recalcTimestamp=False)
    opts = subset_options()
    subsetter = subset.Subsetter(options=opts)
    subsetter.populate(unicodes=codepoints)
    subsetter.subset(font)
    rename_for_ofl(font)

    buf = io.BytesIO()
    font.save(buf)
    blob = buf.getvalue()
    if out_path is not None:
        out_path.parent.mkdir(parents=True, exist_ok=True)
        out_path.write_bytes(blob)
    return font, blob


def fit_level2(src_bytes: bytes, base: list[int], level2: list[int], budget: int):
    """Grow the Level-2 prefix until the file would exceed `budget`.

    Level 2 is not part of the requirement, but leaving 1.2 MB of the budget
    unused would be wasteful.  Glyph data is monotonically increasing in the
    number of characters, so a binary search finds the largest prefix that
    still fits; the result is deterministic for a given budget.
    """
    log(f"filling the {budget/1e6:.2f} MB budget with GB2312 Level 2")
    lo, hi = 0, len(level2)
    best_n, best_size = 0, 0
    while lo <= hi:
        mid = (lo + hi) // 2
        _, blob = build_subset(src_bytes, base + level2[:mid], None)
        size = len(blob)
        log(f"    {mid:5d} L2 chars -> {size/1e6:5.2f} MB "
            f"{'ok' if size <= budget else 'over'}")
        if size <= budget:
            best_n, best_size = mid, size
            lo = mid + 1
        else:
            hi = mid - 1
    log(f"    largest fitting prefix: {best_n}/{len(level2)} L2 chars "
        f"({best_size/1e6:.2f} MB)")
    return best_n


# ---------------------------------------------------------------------------
# verification
# ---------------------------------------------------------------------------
def verify(font_path: Path, required: list[int], sample: str, budget: int) -> bool:
    from fontTools.pens.areaPen import AreaPen

    ok = True
    blob = font_path.read_bytes()
    font = TTFont(font_path)
    tags = sorted(font.keys())

    log("")
    log(f"== {font_path.relative_to(REPO_ROOT)}")
    log(f"   size          : {len(blob)} bytes ({len(blob)/1e6:.2f} MB, "
        f"budget {budget/1e6:.2f} MB)")
    log(f"   glyphs        : {font['maxp'].numGlyphs}")
    log(f"   family        : {font['name'].getDebugName(1)!r} "
        f"({font['name'].getDebugName(4)!r})")
    log(f"   tables        : {', '.join(tags)}")

    if "glyf" not in tags:
        log("   FAIL: no glyf table")
        ok = False
    else:
        log("   glyf table    : present")
    if "CFF " in tags or "CFF2" in tags:
        log("   FAIL: CFF outlines present")
        ok = False
    else:
        log("   CFF outlines  : absent")

    cmap = font.getBestCmap()
    missing = [cp for cp in required if cp not in cmap]
    if missing:
        log(f"   FAIL: {len(missing)} required codepoints missing: "
            f"{''.join(chr(c) for c in missing[:40])}")
        ok = False
    else:
        log(f"   coverage      : all {len(required)} required codepoints present")

    # OFL-1.1: a Modified Version may not use the reserved font names in an
    # identity name record.  Copyright/designer/URL records keep attribution.
    reserved = ("LXGW", "lxgw", "霞鹜", "文楷", "WenKai")
    identity_ids = (1, 2, 3, 4, 6, 16, 17, 18, 20, 21)
    offenders = []
    for rec in font["name"].names:
        if rec.nameID not in identity_ids:
            continue
        try:
            value = rec.toUnicode()
        except UnicodeDecodeError:
            continue
        if any(word in value for word in reserved):
            offenders.append((rec.nameID, value))
    if offenders:
        log(f"   FAIL: OFL reserved name in an identity record: {offenders}")
        ok = False
    else:
        log("   OFL naming    : no reserved name in any identity name record")

    # Real geometry, not blank placeholders.  AreaPen walks the glyf contours
    # and returns the signed area in font units, so a non-zero value proves
    # there is genuine ink to rasterize.
    glyf = font["glyf"]
    log("   outline probe :")
    for ch in sample:
        gname = cmap.get(ord(ch))
        glyph = glyf[gname] if gname else None
        if glyph is None or glyph.numberOfContours == 0:
            log(f"     {ch} U+{ord(ch):04X}: FAIL (empty glyph)")
            ok = False
            continue
        glyph.expand(glyf)
        coords, end_pts, flags = glyph.getCoordinates(glyf)
        points = len(coords)
        area = AreaPen(font.getGlyphSet())
        font.getGlyphSet()[gname].draw(area)
        if points == 0 or abs(area.value) == 0:
            log(f"     {ch} U+{ord(ch):04X}: FAIL "
                f"({points} points, area {area.value})")
            ok = False
        else:
            log(f"     {ch} U+{ord(ch):04X}: {points:3d} points, "
                f"{glyph.numberOfContours} contours, ink area {abs(area.value):7.0f}")

    for label in ("GSUB", "GPOS"):
        if label in tags:
            log(f"   note          : {label} still present")

    log(f"   result        : {'PASS' if ok else 'FAIL'}")
    return ok


# ---------------------------------------------------------------------------
# license sidecar
# ---------------------------------------------------------------------------
def write_license(deb_copyright: bytes, font_path: Path) -> None:
    """Write the OFL text + attribution next to the font, as OFL-1.1 requires."""
    lines = deb_copyright.decode("utf-8", "replace").splitlines()

    # A Debian copyright file carries several "License: <id>" stanzas: one per
    # Files block, then the full license texts at the end.  The OFL text is the
    # last "License: OFL-1.1" stanza, and it ends where the next stanza starts.
    starts = [i for i, line in enumerate(lines) if line.strip() == "License: OFL-1.1"]
    if not starts:
        raise SystemExit("could not find the OFL-1.1 stanza in the .deb copyright")
    start = starts[-1]
    end = len(lines)
    for i in range(start + 1, len(lines)):
        if lines[i].startswith("License:") and not lines[i].startswith(" "):
            end = i
            break

    # Debian indents the stanza body by one space and writes a blank line as
    # " ."; undo both to recover the license verbatim.
    body_lines = []
    for line in lines[start + 1:end]:
        text = line[1:] if line.startswith(" ") else line
        body_lines.append("" if text == "." else text)
    body = "\n".join(body_lines).strip()

    if "SIL OPEN FONT LICENSE Version 1.1" not in body or "GNU General Public" in body:
        raise SystemExit("extracted OFL text looks wrong; refusing to write it")

    header = f"""License and provenance for {font_path.name}
=====================================================

This font is a subset of {SRC_FONT_NAME} (霞鹜文楷) version {SRC_FONT_VERSION},
obtained from the Debian package "fonts-lxgw-wenkai" ({DEB_VERSION}).

    Upstream : {SRC_FONT_URL}
    Author   : lxgw <calxgw2018@gmail.com>
    Copyright: 2021-2025 LXGW
               Copyright 2020 The Klee Project Authors
               (https://github.com/fontworks-fonts/Klee)
    License  : SIL Open Font License 1.1

The file was subset to the characters the rust-fps UI needs and renamed to
"{NEW_FAMILY}" because OFL-1.1 reserves the font names "霞鹜" and "LXGW" for
the original, unmodified font.  No outlines were modified or converted.

The full license text follows.
"""
    OUT_LICENSE.write_text(header + "\n" + body + "\n", encoding="utf-8")
    log(f"wrote {OUT_LICENSE.relative_to(REPO_ROOT)}")


# ---------------------------------------------------------------------------
def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--budget", type=int, default=SIZE_BUDGET,
                    help=f"max output size in bytes (default {SIZE_BUDGET})")
    ap.add_argument("--skip-verify", action="store_true")
    args = ap.parse_args()

    log(f"source font: {SRC_FONT_NAME} {SRC_FONT_VERSION} (OFL-1.1), "
        f"from {DEB_NAME}")
    src_bytes, deb_copyright = source_font_bytes(CACHE_DIR)

    src_cmap = TTFont(io.BytesIO(src_bytes), lazy=True).getBestCmap()
    level2 = gb2312_hanzi(GB2312_LEVEL2_ROWS)
    level2_set = set(level2)

    required = spec_required_codepoints()
    optional = optional_codepoints()
    src_chars = source_characters()

    # The spec set is non-negotiable; fail loudly if the source lacks a char.
    absent_required = sorted(required - set(src_cmap))
    if absent_required:
        raise SystemExit("source font is missing required codepoints: "
                         + "".join(chr(c) for c in absent_required))

    # Optional extras that are unavailable here are dropped and reported.
    optional_ok = available(optional, src_cmap)
    dropped = sorted(optional - optional_ok)
    log(f"required (spec) : {len(required)} codepoints "
        f"- ASCII, CJK punctuation, GB2312 Level 1")
    log(f"optional extras : {len(optional_ok)}/{len(optional)} available "
        f"- fullwidth forms, UI symbols, characters used by src/*.rs")
    if dropped:
        log(f"  not in source font, skipped: {''.join(chr(c) for c in dropped)}")

    # Everything that must be present even without Level-2 filler: the spec
    # set, the non-Level-2 extras, and any Level-2 character the UI uses.
    base = required | (optional_ok - level2_set) | (src_chars & level2_set)
    base_cps = sorted(base)
    _, base_blob = build_subset(src_bytes, base_cps, None)
    log(f"base subset     : {len(base_blob)} bytes for {len(base_cps)} codepoints")
    if len(base_blob) > args.budget:
        raise SystemExit(f"required characters alone need {len(base_blob)} bytes, "
                         f"over the {args.budget} byte budget")

    n2 = fit_level2(src_bytes, base_cps, level2, args.budget)
    final_cps = sorted(base | set(level2[:n2]))

    log(f"building final subset: {len(final_cps)} codepoints")
    _, blob = build_subset(src_bytes, final_cps, OUT_FONT)
    log(f"wrote {OUT_FONT.relative_to(REPO_ROOT)} ({len(blob)} bytes)")

    write_license(deb_copyright, OUT_FONT)

    if not args.skip_verify:
        if not verify(OUT_FONT, sorted(required),
                      "游戏设置开始退出暂停继续Aa1，。", args.budget):
            return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
