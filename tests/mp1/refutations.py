#!/usr/bin/env python3
"""The MP1 harness of the parallel refutation runner (task Q-PLANTS; set `mp1`).

The cases come from the plant records in docs/research/mp1/plants/*.json, not from one registry
file. parallel_refutations_v2.py loads this module and calls `load_cases`, `select_cases`,
`group_key`, `copy_source_tree` and `run_group`. A worker runs one command group: the plants that
share a command run one after the other in one scratch copy with one cargo target folder, with one
baseline and one final green run for the group (the engine is run_group of
scripts/qa/mp1_plants.py, the same code as the serial runner, but in a scratch copy with no .git).

From tests/m2/refutations.py this harness reuses `run` (cargo environment and source receipts),
`copy_source_tree`, `create_run_workspace` and `write_case_evidence` (the case.json format).
The record `env` becomes an `env K=V` prefix of the command. Python 3 standard library only.
"""
import argparse
import importlib.util
import json
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


def command_of(rec):
    if not rec["env"]:
        return list(rec["command"])
    return ["env", *[f"{key}={value}" for key, value in rec["env"].items()], *rec["command"]]


def case_of(rec):
    return {
        "id": plants.label_of(rec),
        "kind": "mp1",
        "runtime": "headed" if rec["headed"] else "native",
        "record": rec,
        "command": command_of(rec),
        "expected_failure": rec["expected_red"],
        "plant_note": f"{rec['row']} pinned at {rec['commit']}",
    }


def load_cases():
    return [case_of(rec) for rec in plants.load_records(REGISTRY)]


def registry_bytes():
    return json.dumps(load_cases(), sort_keys=True).encode()


def group_key(case):
    return plants.group_key(case["record"])


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


def run_group(cases, scratch, build_dir, evidence_dirs):
    """Run the cases of one command group in the scratch copy. Write case.json in each evidence
    folder. Return one (verdict, error, seconds) for each case; verdict is RED-RESTORED or FAIL."""
    recs = [case["record"] for case in cases]

    def execute(rec):
        done = run(command_of(rec), scratch, build_dir)
        return done.returncode, done.stdout

    try:
        results = plants.run_group(scratch, recs, execute=execute, use_git=False)
    except plants.Refused as exc:
        results = [{"reason": str(exc), "baseline_sha256": None, "restored_sha256": None,
                    "baseline_output": "", "red_output": "", "restored_output": "", "red_line": None,
                    "seconds": 0.0} for _ in cases]
    outcomes = []
    for case, result, evidence in zip(cases, results, evidence_dirs):
        failed = bool(result["reason"])
        logs = {"baseline.green.log": result["baseline_output"], "plant.red.log": result["red_output"],
                "restored.green.log": result["restored_output"]}
        detail = result["reason"] if failed else f"semantic assertion: {result['red_line']}"
        base.write_case_evidence(evidence, case["id"], result["baseline_sha256"], result["restored_sha256"],
                                 logs, "failed" if failed else "passed", None, detail, {}, case["plant_note"])
        print(f'{case["id"]}: ' + (f"FAIL {detail.splitlines()[0]}" if failed else f"RED assertion={result['red_line']!r}"))
        outcomes.append(("FAIL" if failed else "RED-RESTORED", detail if failed else None, result["seconds"]))
    return outcomes


def main():
    parser = argparse.ArgumentParser(description="Run MP1 plant records in a scratch copy, one command group at a time.")
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
    groups = {}
    for case in cases:
        groups.setdefault(group_key(case), []).append(case)
    failed = 0
    for group in groups.values():
        dirs = []
        for case in group:
            evidence = run_root / "cases" / re.sub(r"[^A-Za-z0-9_.-]", "_", case["id"])
            evidence.mkdir(parents=True, exist_ok=False)
            dirs.append(evidence)
        failed += sum(verdict != "RED-RESTORED" for verdict, _, _ in run_group(group, scratch, build_dir, dirs))
    print(f"evidence: {run_root}")
    return 1 if failed else 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except Exception as exc:
        print(f"refutations: {exc}", file=sys.stderr)
        raise SystemExit(1)
