#!/usr/bin/env python3
"""Fail-closed longer recipe time of the packed copy of one M2 economy browser case.

Hosted run 43214215645: on the software renderer one screenshot costs about 3.3 s, and the steps
between Produce and Cancel took 8.1 to 9.6 s. The recipes last 5 s (base) and 6 s (cedar, haul),
so the job was done when the Cancel click came. The method is the one of T7_SCOUT_SPEED in
scripts/t7.sh:37-56: edit the packed copy only, fail when the anchor is not there exactly once,
and leave the oracle requirement as it is. `mods/` never changes.

Only the three economy cases are scaled. The recovery and loss cases count ticks, not seconds.
The receipt is written in every case. The oracle reads it, reads the true recipe time from
`mods/` by itself, and checks that the job in the received world has the packed duration.
"""
import argparse
import json
from pathlib import Path
import re
import sys

ROOT = Path(__file__).resolve().parents[2]
# case id -> (package, recipe file of the row that the driver clicks, BUILD_ROW(0))
ECONOMY_RECIPES = {
    "E9-browser-base-economy": ("base", "production/rifleman.ron"),
    "E9-browser-haul-economy": ("haul", "production/worker.ron"),
    "E9-browser-cedar-economy": ("cedar", "production/hire.ron"),
}
# Loss cases that take a per-case factor (M2_RECIPE_DURATION_SCALE_CASES) and never the general one. The refund job
# (5 s) was gone before the queue shot (7.3 s after the click) and before the Cancel click (12.1 s to 12.6 s) in hosted
# run 43228094477. The other loss cases count ticks or keep the job alive (final_change blocks the output): no scale.
PER_CASE_RECIPES = {
    "E9-browser-base-loss-refund": ("base", "production/rifleman.ron"),
}
DURATION = re.compile(r"duration_ms:\s*(\d+)\s*,")


def duration_ms(text, where):
    found = DURATION.findall(text)
    if len(found) != 1:
        raise ValueError(f"{where}: expected exactly one `duration_ms: N,`, found {len(found)}")
    return int(found[0])


def parse_scale(raw):
    if not re.fullmatch(r"[1-9][0-9]*", raw):
        raise ValueError(f"M2_RECIPE_DURATION_SCALE must be a whole number above 0, got {raw!r}")
    scale = int(raw)
    if scale > 10:
        raise ValueError(f"M2_RECIPE_DURATION_SCALE above 10 is refused, got {scale}")
    return scale


def parse_case_scales(raw):
    """`CASE=N,CASE=N`: only the cases of PER_CASE_RECIPES, each once, N a whole number 1 to 10."""
    scales = {}
    for item in [part for part in (raw or "").split(",") if part != ""] if raw else []:
        name, separator, value = item.partition("=")
        if not separator or name not in PER_CASE_RECIPES:
            raise ValueError(f"M2_RECIPE_DURATION_SCALE_CASES entry {item!r}: only {sorted(PER_CASE_RECIPES)} take a per-case factor, written CASE=N")
        if name in scales:
            raise ValueError(f"M2_RECIPE_DURATION_SCALE_CASES lists {name} twice")
        scales[name] = parse_scale(value)
    if raw and not scales:
        raise ValueError(f"M2_RECIPE_DURATION_SCALE_CASES has no entry: {raw!r}")
    return scales


def apply(source_package, packed_package, package, case_id, raw_scale, receipt_path, case_scales=""):
    scale = parse_scale(raw_scale)
    per_case = parse_case_scales(case_scales)
    receipt = {"package": package, "case_id": case_id, "scale_requested": scale, "scale": 1, "scale_source": "none",
               "applied": False, "recipe_file": None, "source_duration_ms": None, "packed_duration_ms": None}
    recipe = ECONOMY_RECIPES.get(case_id) or PER_CASE_RECIPES.get(case_id)
    if case_id in PER_CASE_RECIPES:
        scale = per_case.get(case_id, 1)
        receipt.update(scale_requested=scale)
        source_kind = "case" if case_id in per_case else "none"
    else:
        source_kind = "general"
    if recipe:
        economy_package, relative = recipe
        if economy_package != package:
            raise ValueError(f"case {case_id} belongs to package {economy_package}, not {package}")
        source_text = (source_package / relative).read_text()
        packed_file = packed_package / relative
        packed_text = packed_file.read_text()
        if packed_text != source_text:
            raise ValueError(f"{packed_file}: the packed copy differs from mods/ before the edit")
        source_ms = duration_ms(source_text, str(source_package / relative))
        packed_ms = source_ms * scale
        if scale != 1:
            packed_file.write_text(DURATION.sub(f"duration_ms: {packed_ms},", packed_text, count=1))
            if duration_ms(packed_file.read_text(), str(packed_file)) != packed_ms:
                raise ValueError(f"{packed_file}: the edit did not give duration_ms {packed_ms}")
        receipt.update(scale=scale, applied=scale != 1, recipe_file=relative, scale_source=source_kind,
                       source_duration_ms=source_ms, packed_duration_ms=packed_ms)
    receipt_path.write_text(json.dumps(receipt, sort_keys=True) + "\n")
    return receipt


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--source", required=True, type=Path, help="mods/<package>")
    parser.add_argument("--packed", required=True, type=Path, help="packed copy of the package")
    parser.add_argument("--package", required=True)
    parser.add_argument("--case", required=True)
    parser.add_argument("--scale", default="1")
    parser.add_argument("--case-scales", default="", help="CASE=N list for the cases of PER_CASE_RECIPES")
    parser.add_argument("--receipt", required=True, type=Path)
    args = parser.parse_args()
    try:
        receipt = apply(args.source, args.packed, args.package, args.case, args.scale or "1", args.receipt, args.case_scales)
    except (ValueError, OSError) as error:
        print(f"recipe_duration: {error}", file=sys.stderr)
        return 1
    if receipt["applied"]:
        print(f"M2_RECIPE_DURATION_SCALE={receipt['scale']} ({receipt['recipe_file']}: "
              f"{receipt['source_duration_ms']} ms to {receipt['packed_duration_ms']} ms in the packed copy; mods/ is unchanged)")
    else:
        print(f"M2_RECIPE_DURATION_SCALE=1 (true recipe times; scale requested {receipt['scale_requested']}, "
              f"case {args.case})")
    return 0


if __name__ == "__main__":
    sys.exit(main())
