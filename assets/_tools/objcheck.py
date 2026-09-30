"""Minimal Wavefront OBJ sanity checker.

Counts v/vt/vn/f records and reports whether a same-named .png sits beside the
file. No third-party deps: the goal is to confirm the bytes macroquad's loader
will see, not to render anything.
"""

import sys
from pathlib import Path


def scan(path: Path) -> dict:
    counts = {"v": 0, "vt": 0, "vn": 0, "f": 0, "o": 0, "g": 0, "usemtl": 0, "mtllib": 0}
    groups: list[str] = []
    tri_faces = 0
    ngon_faces = 0
    bad_faces = 0
    with path.open("r", encoding="utf-8", errors="replace") as fh:
        for raw in fh:
            line = raw.strip()
            if not line or line[0] == "#":
                continue
            key, _, rest = line.partition(" ")
            if key in ("v", "vt", "vn"):
                counts[key] += 1
            elif key == "f":
                counts["f"] += 1
                n = len(rest.split())
                if n == 3:
                    tri_faces += 1
                elif n > 3:
                    ngon_faces += 1
                else:
                    bad_faces += 1
            elif key == "o":
                counts["o"] += 1
                groups.append(rest.strip())
            elif key == "g":
                counts["g"] += 1
                groups.append(rest.strip())
            elif key == "usemtl":
                counts["usemtl"] += 1
            elif key == "mtllib":
                counts["mtllib"] += 1

    png = path.with_suffix(".png")
    mtl = path.with_suffix(".mtl")
    return {
        "path": path,
        "counts": counts,
        "groups": groups,
        "tri": tri_faces,
        "ngon": ngon_faces,
        "bad": bad_faces,
        "png": png.exists(),
        "mtl": mtl.exists(),
        "kb": path.stat().st_size / 1024,
    }


def main(argv: list[str]) -> int:
    targets: list[Path] = []
    for a in argv:
        p = Path(a)
        if p.is_dir():
            targets.extend(sorted(p.rglob("*.obj")))
        else:
            targets.append(p)
    if not targets:
        print("no .obj files found")
        return 1

    bad = 0
    print(f"{'file':<58} {'v':>6} {'vt':>6} {'vn':>6} {'f':>6} {'png':>4} {'mtl':>4}")
    print("-" * 96)
    for t in targets:
        r = scan(t)
        c = r["counts"]
        ok = c["v"] > 0 and c["f"] > 0
        bad += 0 if ok else 1
        label = str(t).replace("\\", "/")
        if len(label) > 56:
            label = "..." + label[-53:]
        print(
            f"{label:<58} {c['v']:>6} {c['vt']:>6} {c['vn']:>6} {c['f']:>6} "
            f"{'yes' if r['png'] else '-':>4} {'yes' if r['mtl'] else '-':>4}"
            f"{'' if ok else '   <-- NO VERTICES/FACES'}"
        )
    print("-" * 96)
    print(f"{len(targets)} obj file(s), {bad} failing (v==0 or f==0)")
    return 0 if bad == 0 else 2


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
