#!/usr/bin/env python3
"""Fail-closed translation of test-only packed M2 browser fixtures."""
import argparse
from decimal import Decimal
import hashlib
import itertools
import json
from pathlib import Path
import re

ROOT = Path(__file__).resolve().parents[2]
OFFSET_X = Decimal("18.0")
MAPS = {
    "base": ("maps/flat.ron", "flat", Decimal("64.0"), Decimal("64.0")),
    "haul": ("maps/field.ron", "field", Decimal("48.0"), Decimal("48.0")),
    "cedar": ("maps/clearing.ron", "clearing", Decimal("64.0"), Decimal("64.0")),
}
NUMBER = r"[+-]?(?:\d+\.\d+|\d+)"
SIZE = re.compile(rf"(size:\s*\(x:\s*)({NUMBER})(\s*,\s*y:\s*)({NUMBER})(\s*\))")
POSITION = re.compile(rf"(pos:\s*\(x:\s*)({NUMBER})(\s*,\s*y:\s*)({NUMBER})(\s*\))")
PLACEMENT = re.compile(
    rf'\(\s*unit:\s*"([^"\n]+)"\s*,\s*owner_slot:\s*(\d+)\s*,\s*'
    rf'pos:\s*\(x:\s*({NUMBER})\s*,\s*y:\s*({NUMBER})\s*\)'
)
NODE = re.compile(
    rf'\(\s*node:\s*"([^"\n]+)"\s*,\s*'
    rf'pos:\s*\(x:\s*({NUMBER})\s*,\s*y:\s*({NUMBER})\s*\)'
)


def sha256(data):
    return hashlib.sha256(data).hexdigest()


def decimal(value):
    try:
        result = Decimal(value)
    except Exception as error:
        raise ValueError(f"invalid coordinate {value!r}") from error
    if not result.is_finite():
        raise ValueError(f"non-finite coordinate {value!r}")
    return result


def one_decimal(value):
    return format(value.quantize(Decimal("0.1")), ".1f")


def parse_map(text, package, translated=False):
    if package not in MAPS:
        raise ValueError(f"unsupported translated package {package!r}")
    relative, expected_id, expected_width, expected_height = MAPS[package]
    sizes = list(SIZE.finditer(text))
    if len(sizes) != 1:
        raise ValueError(f"expected one map size in {package}, found {len(sizes)}")
    size = sizes[0]
    map_id_matches = re.findall(r'id:\s*"([^"]+)"', text)
    if map_id_matches != [expected_id]:
        raise ValueError(f"unexpected {package} map id: {map_id_matches!r}")
    width, height = decimal(size.group(2)), decimal(size.group(4))
    if translated:
        expected_width += OFFSET_X
    if (width, height) != (expected_width, expected_height):
        raise ValueError(f"unexpected {package} dimensions {(width, height)!r}")
    placements = list(PLACEMENT.finditer(text))
    nodes = list(NODE.finditer(text))
    position_count = len(list(POSITION.finditer(text)))
    if not placements or not nodes:
        raise ValueError(f"{package} fixture must contain placements and nodes")
    if text.count("(unit:") != len(placements) or text.count("(node:") != len(nodes):
        raise ValueError(f"{package} map contains an unparsed placement or node")
    if position_count != len(placements) + len(nodes):
        raise ValueError(f"{package} map contains an unparsed position field")
    records = []
    for match in placements:
        x, y = decimal(match.group(3)), decimal(match.group(4))
        records.append({"kind": "unit", "id": match.group(1), "owner_slot": int(match.group(2)), "x": x, "y": y})
    for match in nodes:
        x, y = decimal(match.group(2)), decimal(match.group(3))
        records.append({"kind": "node", "id": match.group(1), "x": x, "y": y})
    for record in records:
        if not (Decimal(0) <= record["x"] <= width and Decimal(0) <= record["y"] <= height):
            raise ValueError(f"{package} coordinate outside original bounds: {record!r}")
    return {"id": expected_id, "width": width, "height": height, "records": records}


def add_expected_case_override(source_text, package, case_id):
    if package == "base" and case_id == "E9-browser-base-loss-final_change":
        anchor = '        (unit: "base:harvester", owner_slot: 1, pos: (x: 52.0, y: 8.0), contents: []),'
        blockers = (
            '\n        (unit: "base:command_center", owner_slot: 0, pos: (x: 8.0, y: 12.0), contents: []),'
            '\n        (unit: "base:command_center", owner_slot: 0, pos: (x: 8.0, y: 4.0), contents: []),'
        )
        if source_text.count(anchor) != 1:
            raise ValueError("base final_change source placement anchor must occur exactly once")
        return source_text.replace(anchor, anchor + blockers, 1)
    return source_text


def pairwise_distance_hash(records):
    points = [(record["x"], record["y"]) for record in records]
    squared = [str((ax - bx) ** 2 + (ay - by) ** 2) for (ax, ay), (bx, by) in itertools.combinations(points, 2)]
    return sha256(json.dumps(squared, separators=(",", ":")).encode())


def translate(source_path, packed_path, receipt_path, package, case_id, offset):
    if package not in MAPS or decimal(str(offset)) != OFFSET_X:
        raise ValueError("fixture translation requires base/haul/cedar with explicit X offset 18")
    relative, _, _, _ = MAPS[package]
    expected_source = ROOT / "mods" / package / relative
    expected_packed = Path("pack") / package / relative
    source_path = source_path.resolve()
    packed_path = packed_path.resolve()
    if source_path != expected_source.resolve():
        raise ValueError(f"unexpected source map path {source_path}")
    if packed_path.parts[-4:] != expected_packed.parts:
        raise ValueError(f"unexpected packed map path {packed_path}")
    source_bytes = source_path.read_bytes()
    packed_before = packed_path.read_bytes()
    source_text = source_bytes.decode("utf-8")
    expected_before_text = add_expected_case_override(source_text, package, case_id)
    if packed_before.decode("utf-8") != expected_before_text:
        raise ValueError("packed map differs from source plus the one registered case-specific fixture override")
    before = parse_map(expected_before_text, package)
    width_match = SIZE.search(expected_before_text)
    assert width_match is not None
    new_width = before["width"] + OFFSET_X
    translated_text, size_count = SIZE.subn(
        lambda match: match.group(1) + one_decimal(new_width) + match.group(3) + match.group(4) + match.group(5),
        expected_before_text,
        count=1,
    )
    if size_count != 1:
        raise ValueError("map width translation anchor must occur exactly once")

    def move_position(match):
        return match.group(1) + one_decimal(decimal(match.group(2)) + OFFSET_X) + match.group(3) + match.group(4) + match.group(5)

    translated_text, position_count = POSITION.subn(move_position, translated_text)
    if position_count != len(before["records"]):
        raise ValueError("expected to translate every placement and node exactly once")
    after = parse_map(translated_text, package, translated=True)
    if after["width"] != new_width or after["height"] != before["height"]:
        raise ValueError("translated map dimensions are invalid")
    if len(after["records"]) != len(before["records"]):
        raise ValueError("translation changed placement/node cardinality")
    for old, new in zip(before["records"], after["records"], strict=True):
        if old["kind"] != new["kind"] or old["id"] != new["id"] or old.get("owner_slot") != new.get("owner_slot"):
            raise ValueError("translation changed IDs, kind, owner, or ordering")
        if new["x"] != old["x"] + OFFSET_X or new["y"] != old["y"]:
            raise ValueError("translation changed more than X by the approved offset")
        if not (Decimal(0) <= new["x"] <= new_width and Decimal(0) <= new["y"] <= after["height"]):
            raise ValueError(f"translated coordinate outside map bounds: {new!r}")
    before_distances = pairwise_distance_hash(before["records"])
    after_distances = pairwise_distance_hash(after["records"])
    if before_distances != after_distances:
        raise ValueError("translation changed pairwise geometry")

    restored_text = SIZE.sub(
        lambda match: match.group(1) + one_decimal(before["width"]) + match.group(3) + match.group(4) + match.group(5),
        translated_text,
        count=1,
    )
    restored_text = POSITION.sub(
        lambda match: match.group(1) + one_decimal(decimal(match.group(2)) - OFFSET_X) + match.group(3) + match.group(4) + match.group(5),
        restored_text,
    )
    if restored_text != expected_before_text:
        raise ValueError("translation modified non-coordinate source text")

    translated_bytes = translated_text.encode("utf-8")
    packed_path.write_bytes(translated_bytes)
    receipt = {
        "package": package,
        "case_id": case_id,
        "source_map": str(source_path),
        "packed_map": str(packed_path),
        "map_relative_path": relative,
        "offset_x": 18,
        "source_map_sha256": sha256(source_bytes),
        "packed_before_translation_sha256": sha256(packed_before),
        "packed_after_translation_sha256": sha256(translated_bytes),
        "map_id": before["id"],
        "map_size_before": [str(before["width"]), str(before["height"])],
        "map_size_after": [str(after["width"]), str(after["height"])],
        "placement_count": sum(record["kind"] == "unit" for record in before["records"]),
        "node_count": sum(record["kind"] == "node" for record in before["records"]),
        "ordered_records_before": [
            {key: str(value) if isinstance(value, Decimal) else value for key, value in record.items()}
            for record in before["records"]
        ],
        "ordered_records_after": [
            {key: str(value) if isinstance(value, Decimal) else value for key, value in record.items()}
            for record in after["records"]
        ],
        "pairwise_squared_distance_sha256_before": before_distances,
        "pairwise_squared_distance_sha256_after": after_distances,
        "expected_case_override_sha256": sha256(expected_before_text.encode("utf-8")),
        "non_coordinate_source_text_preserved": True,
    }
    receipt_path.write_text(json.dumps(receipt, indent=2) + "\n")
    return receipt


def main(argv=None):
    parser = argparse.ArgumentParser()
    parser.add_argument("--source", type=Path, required=True)
    parser.add_argument("--packed", type=Path, required=True)
    parser.add_argument("--receipt", type=Path, required=True)
    parser.add_argument("--package", choices=tuple(MAPS), required=True)
    parser.add_argument("--case", default="")
    parser.add_argument("--offset-x", type=Decimal, required=True)
    args = parser.parse_args(argv)
    receipt = translate(args.source, args.packed, args.receipt, args.package, args.case, args.offset_x)
    print(json.dumps({key: receipt[key] for key in ("package", "case_id", "offset_x", "map_size_before", "map_size_after", "placement_count", "node_count", "packed_after_translation_sha256")}))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
