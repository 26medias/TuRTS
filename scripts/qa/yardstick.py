#!/usr/bin/env python3
"""Extract and verify the Part A yardstick table."""

from __future__ import annotations

import argparse
import re
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SOURCE = ROOT / "docs/research/review-2026-10/inventory-engine.md"
OUTPUT = ROOT / "docs/qa/yardstick-a.tsv"
HEADER = "id\tarea\tname\ttier\ttier_note\tkind\tdeps\n"
ITEM_ID = re.compile(r"^[A-Z]{2}[0-9]{2}$")
DEPENDENCY = re.compile(r"[A-Z]{2}[0-9]{2}")


def extract() -> bytes:
    text = SOURCE.read_text(encoding="utf-8")
    lines = text.splitlines()
    in_part = False
    area = None
    rows = [HEADER]
    for line in lines:
        if line.startswith("# Part A"):
            in_part = True
            continue
        if in_part and line.startswith("# Part B"):
            break
        if not in_part:
            continue
        heading = re.match(r"^## (A[0-9]+)\.", line)
        if heading:
            area = heading.group(1)
            continue
        if not line.startswith("| "):
            continue
        cells = [cell.strip() for cell in line.strip().strip("|").split("|")]
        if len(cells) != 7 or not ITEM_ID.fullmatch(cells[0]):
            continue
        if area is None:
            raise ValueError(f"item row before area heading: {line}")
        tier_parts = cells[5].split(None, 1)
        tier = tier_parts[0] if tier_parts else ""
        note = tier_parts[1] if len(tier_parts) > 1 else ""
        deps = ",".join(DEPENDENCY.findall(cells[6])) or "-"
        values = (cells[0], area, cells[1], tier, note, cells[4], deps)
        rows.append("\t".join(values) + "\n")
    if not in_part or not any(line.startswith("# Part B") for line in lines):
        raise ValueError("could not find Part A and Part B boundaries")
    return "".join(rows).encode("utf-8")


def first_difference(expected: bytes, actual: bytes) -> str:
    left = expected.decode("utf-8").splitlines()
    right = actual.decode("utf-8").splitlines()
    for number, pair in enumerate(zip(left, right), 1):
        if pair[0] != pair[1]:
            return f"line {number}: expected {pair[0]!r}; found {pair[1]!r}"
    if len(left) != len(right):
        number = min(len(left), len(right)) + 1
        a = left[number - 1] if number <= len(left) else "<EOF>"
        b = right[number - 1] if number <= len(right) else "<EOF>"
        return f"line {number}: expected {a!r}; found {b!r}"
    return "no difference"


def main() -> int:
    parser = argparse.ArgumentParser()
    group = parser.add_mutually_exclusive_group(required=True)
    group.add_argument("--extract", action="store_true")
    group.add_argument("--check", action="store_true")
    args = parser.parse_args()
    try:
        data = extract()
    except (OSError, UnicodeError, ValueError) as error:
        print(f"yardstick: {error}", file=sys.stderr)
        return 2
    if args.extract:
        OUTPUT.parent.mkdir(parents=True, exist_ok=True)
        OUTPUT.write_bytes(data)
        print(f"wrote {OUTPUT.relative_to(ROOT)}")
        return 0
    if not OUTPUT.exists():
        print(f"yardstick: missing {OUTPUT.relative_to(ROOT)}", file=sys.stderr)
        return 1
    with tempfile.NamedTemporaryFile() as temporary:
        temporary.write(data)
        temporary.flush()
        temporary.seek(0)
        extracted = temporary.read()
        actual = OUTPUT.read_bytes()
        if extracted != data:
            print("yardstick: temporary extraction changed bytes", file=sys.stderr)
            return 2
        if actual == data:
            print("yardstick matches inventory")
            return 0
        print(f"yardstick differs: {first_difference(data, actual)}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
