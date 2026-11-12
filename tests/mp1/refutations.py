#!/usr/bin/env python3
"""The MP1 harness of the parallel refutation runner (task Q-PLANTS; set `mp1`).

The cases come from the plant records in docs/research/mp1/plants/*.json, not from one registry
file. The runner parallel_refutations_v2.py loads this module and calls `load_cases`,
`select_cases`, `copy_source_tree`, `pre_commands`, `run_case` and `require_phase_result`, as it
does for tests/m2/refutations.py and tests/m3/refutations.py.

Nothing here is a second style. The run of one case is `run_case` of tests/m2/refutations.py
(scratch copy, baseline green, plant, red, restore with a source hash, restored green, evidence
files). Two functions of that module are replaced, in this private copy only:
    require_phase_result  by the judge of scripts/qa/mp1_plants.py (exit, text, no compile error);
    apply_plant           by the anchor rule of scripts/qa/mp1_plants.py (also for patch hunks).
The record `env` becomes an `env K=V` prefix of the command. Python 3 standard library only.
"""
import argparse
import importlib.util
import json
import os
from pathlib import Path
import re
import sys

sys.dont_write_bytecode = True  # the loads below must leave no __pycache__
ROOT = Path(__file__).resolve().parents[2]


def _load(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


plants = _load("mp1_plants_core", ROOT / "scripts/qa/mp1_plants.py")
base = _load("mp1_m2_base", ROOT / "tests/m2/refutations.py")
if base.ROOT.resolve() != ROOT.resolve():
    raise RuntimeError(f"tests/m2/refutations.py ROOT {base.ROOT} is not {ROOT}")

# A folder of records. `load_cases` and `registry_bytes` read it; there is no registry file.
REGISTRY = ROOT / plants.PLANTS_DIR
copy_source_tree = base.copy_source_tree
pre_commands = base.pre_commands
digest = base.digest
run = base.run


def case_of(rec):
    command = list(rec["command"])
    if rec["env"]:
        command = ["env", *[f"{key}={value}" for key, value in rec["env"].items()], *command]
    return {
        "id": plants.label_of(rec),
        "kind": "mp1",
        "runtime": "headed" if rec["headed"] else "native",
        "plants": [{
            "path": entry["file"],
            "replacements": [{"anchor": old, "replacement": new, "whole_lines": whole}
                             for old, new, whole in plants.replacements_of(entry)],
        } for entry in rec["files"]],
        "command": command,
        "expected_failure": rec["expected_red"],
        "expected_exit": rec["exit"],
        "plant_note": f"{rec['row']} pinned at {rec['commit']}",
    }


def load_cases():
    return [case_of(rec) for rec in plants.load_records(REGISTRY)]


def registry_bytes():
    return json.dumps(load_cases(), sort_keys=True).encode()


def select_cases(cases, requested):
    if requested in ("all", "native"):
        selected = [case for case in cases if case["runtime"] == "native"]
        skipped = [case["id"] for case in cases if case["runtime"] != "native"]
        if skipped:
            print(f"mp1 harness: headed records skipped (they run through the wrapper): {skipped}",
                  file=sys.stderr)
    elif requested == "headed":
        raise ValueError("the parallel runner does not run headed plant records (F-HEAD and REG2 do)")
    else:
        wanted = [item.strip() for item in requested.split(",")]
        by_id = {case["id"]: case for case in cases}
        if not all(wanted) or len(wanted) != len(set(wanted)):
            raise ValueError("case list contains an empty or duplicate case ID")
        unknown = [item for item in wanted if item not in by_id]
        if unknown:
            raise ValueError(f"unknown refutation case(s): {', '.join(unknown)}")
        selected = [by_id[item] for item in wanted]
        if any(case["runtime"] != "native" for case in selected):
            raise ValueError("a headed plant record cannot run in the parallel runner")
    if not selected:
        raise ValueError("no refutation cases selected")
    return selected


def require_phase_result(case, phase, result):
    try:
        if phase in {"baseline.green", "restored.green"}:
            plants.judge_green(phase, result.returncode, result.stdout)
            return None
        record = {"exit": case["expected_exit"], "expected_red": case["expected_failure"]}
        return plants.judge_red(record, result.returncode, result.stdout)
    except plants.PlantFail as exc:
        raise RuntimeError(f'{case["id"]}: {exc}') from None


# Replace the two functions that run_case of the base module calls through its globals.
base.require_phase_result = require_phase_result
base.apply_plant = plants.apply_plant_case


def run_case(case, scratch, build_dir, evidence_dir):
    return base.run_case(case, scratch, build_dir, evidence_dir)


def main():
    parser = argparse.ArgumentParser(description="Run MP1 plant records one after the other in a scratch copy.")
    parser.add_argument("--case", default="all", help="all, or a comma list of row/plant ids")
    parser.add_argument("--list", action="store_true", help="print the selected case ids and run nothing")
    parser.add_argument("--evidence-root", type=Path, default=ROOT / "target/mp1-plants")
    parser.add_argument("--build-root", type=Path, default=ROOT / "target/mp1-plants-builds")
    args = parser.parse_args()
    cases = select_cases(load_cases(), args.case)
    if args.list:
        for case in cases:
            print(case["id"])
        return 0
    run_root = base.create_run_workspace(args.evidence_root, args.build_root)
    build_root = Path((run_root / "build-root.txt").read_text().strip())
    (run_root / "selection.json").write_text(json.dumps([case["id"] for case in cases], indent=2) + "\n")
    scratch = run_root / "scratch" / "source"
    build_dir = build_root / "cargo-target"
    copy_source_tree(ROOT, scratch)
    build_dir.mkdir(parents=True, exist_ok=False)
    for case in cases:
        evidence_dir = run_root / "cases" / re.sub(r"[^A-Za-z0-9_.-]", "_", case["id"])
        evidence_dir.mkdir(parents=True, exist_ok=False)
        run_case(case, scratch, build_dir, evidence_dir)
    print(f"evidence: {run_root}")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except Exception as exc:
        print(f"refutations: {exc}", file=sys.stderr)
        raise SystemExit(1)
