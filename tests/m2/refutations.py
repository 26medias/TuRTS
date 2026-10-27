#!/usr/bin/env python3
"""Run exact source mutations in an isolated Task14 scratch copy."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[2]
TASK_TARGET = Path("/home/julien/Projects/TuRTS/target/m2-public-proof")
SCRATCH = TASK_TARGET / "refutation-scratch"
BUILD = TASK_TARGET / "refutation-build"
REGISTRY = ROOT / "tests/m2/refutations.json"

def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()

def run(command, cwd):
    env = os.environ.copy()
    env["CARGO_TARGET_DIR"] = str(BUILD)
    env["CARGO_BUILD_JOBS"] = "2"
    env["RUST_BACKTRACE"] = "0"
    return subprocess.run(command, cwd=cwd, env=env, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)

def selected(output):
    match = re.search(r"running (\d+) tests?", output)
    return int(match.group(1)) if match else 0

def run_case(case):
    def execute():
        if case.get("pre_command"):
            built = run(case["pre_command"], SCRATCH)
            if built.returncode != 0:
                raise RuntimeError(f'{case["id"]}: prerequisite build failed\n{built.stdout[-5000:]}')
        return run(case["command"], SCRATCH)

    path = SCRATCH / case["plant_path"]
    anchor = case["anchor"]
    original = path.read_text()
    if original.count(anchor) != 1:
        raise RuntimeError(f'{case["id"]}: anchor occurs {original.count(anchor)} times, expected exactly one')
    baseline_hash = digest(path)
    green = execute()
    if green.returncode != 0 or selected(green.stdout) == 0 or "test result: ok" not in green.stdout:
        raise RuntimeError(f'{case["id"]}: baseline is not green with a selected test\n{green.stdout[-5000:]}')
    planted = original.replace(anchor, case["replacement"], 1)
    path.write_text(planted)
    red = execute()
    try:
        if selected(red.stdout) == 0:
            raise RuntimeError(f'{case["id"]}: zero selected tests after plant\n{red.stdout[-5000:]}')
        if "could not compile" in red.stdout or re.search(r"error\[E\d+\]", red.stdout):
            raise RuntimeError(f'{case["id"]}: plant caused a compile failure\n{red.stdout[-5000:]}')
        if red.returncode == 0:
            raise RuntimeError(f'{case["id"]}: planted test remained green\n{red.stdout[-5000:]}')
        semantic_lines = [line.strip() for line in red.stdout.splitlines()
                          if case["expected_failure"] in line
                          and not line.lstrip().startswith("thread '")
                          and not line.lstrip().startswith("failures:")
                          and not line.lstrip().startswith("test ")]
        if not semantic_lines:
            raise RuntimeError(f'{case["id"]}: red lacked semantic failure {case["expected_failure"]!r}\n{red.stdout[-5000:]}')
        semantic_line = semantic_lines[0]
        print(f'{case["id"]}: RED selected={selected(red.stdout)} assertion={semantic_line!r}')
    finally:
        path.write_text(original)
    restored_hash = digest(path)
    if restored_hash != baseline_hash:
        raise RuntimeError(f'{case["id"]}: source restore hash differs: {baseline_hash} != {restored_hash}')
    restored = execute()
    if restored.returncode != 0 or selected(restored.stdout) == 0 or "test result: ok" not in restored.stdout:
        raise RuntimeError(f'{case["id"]}: restored source is not green\n{restored.stdout[-5000:]}')
    print(f'{case["id"]}: RESTORED GREEN source_sha256={restored_hash}')

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--case", required=True)
    args = parser.parse_args()
    cases = json.loads(REGISTRY.read_text())
    if args.case == "all":
        selected_cases = cases
    elif args.case == "product":
        selected_cases = [c for c in cases if c.get("kind") == "product"]
    elif "," in args.case:
        wanted = set(args.case.split(","))
        selected_cases = [c for c in cases if c["id"] in wanted]
    else:
        selected_cases = [c for c in cases if c["id"] == args.case]
    if not selected_cases:
        raise SystemExit(f"unknown refutation case: {args.case}")
    if SCRATCH.exists():
        shutil.rmtree(SCRATCH)
    if BUILD.exists():
        shutil.rmtree(BUILD)
    SCRATCH.parent.mkdir(parents=True, exist_ok=True)
    shutil.copytree(ROOT, SCRATCH, ignore=shutil.ignore_patterns(".git", "target", ".superpowers", ".claude", ".codex", ".agents", "__pycache__"))
    for case in selected_cases:
        run_case(case)
    return 0

if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except Exception as exc:
        print(f"refutations: {exc}", file=sys.stderr)
        raise SystemExit(1)
