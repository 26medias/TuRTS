#!/usr/bin/env python3
"""Browser cases that the hosted runner does not run, and the ledger that keeps that list honest.

Hosted run 43214215645: the public-convergence cases need two windows that show the same tick.
The software renderer shows one new tick about every 28 ticks in each window, the two windows are
16 ticks out of phase, and they never share a tick (0 shared ticks in 1651 and 1656 polls). A
longer limit cannot help. These cases need a real GPU, so they run in the local gate.

A hosted job lists its local-only cases in the env value M2_LOCAL_ONLY_CASES (.github/workflows/ci.yml).
A listed case is never a silent skip: this script prints one plain line (the case, the reason, the
local gate command, the receipt place, the owner) and appends the same facts to $GITHUB_STEP_SUMMARY.
A case can be listed only when it has a row in tests/m2/local-only.tsv. `--check` fails when a
listed case has no row, when a row is not listed, or when a row names a case that is not registered.

Usage:
  local_only.py --decide CASE   exit 0: run the case; exit 10: local-only (notice printed); exit 2: error
  local_only.py --count         read case ids from stdin, print how many are listed
  local_only.py --check         static check of the ledger, the registered cases and ci.yml
"""
import argparse
import json
import os
from pathlib import Path
import re
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[2]
LEDGER = Path(__file__).with_name("local-only.tsv")
WORKFLOW = ROOT / ".github" / "workflows" / "ci.yml"
ENV_NAME = "M2_LOCAL_ONLY_CASES"
COLUMNS = ["case", "reason", "local_command", "receipt", "owner"]
CASE_ID = re.compile(r"E[0-9]-browser-[a-z0-9_-]+")
MARK_LINE = re.compile(rf'^\s*{ENV_NAME}:\s*"([^"]*)"\s*$')


class LedgerError(Exception):
    pass


def read_ledger(path=LEDGER):
    lines = [line for line in Path(path).read_text().splitlines() if line.strip() and not line.startswith("#")]
    if not lines or lines[0].split("\t") != COLUMNS:
        raise LedgerError(f"{path}: the first line must be the tab-separated header {COLUMNS}")
    rows = {}
    for number, line in enumerate(lines[1:], 2):
        fields = line.split("\t")
        if len(fields) != len(COLUMNS) or any(not field.strip() for field in fields):
            raise LedgerError(f"{path}: row {number} must have {len(COLUMNS)} non-empty tab-separated fields")
        row = dict(zip(COLUMNS, fields))
        if not CASE_ID.fullmatch(row["case"]):
            raise LedgerError(f"{path}: row {number} has a case id that is not a browser case id: {row['case']!r}")
        if row["case"] in rows:
            raise LedgerError(f"{path}: case {row['case']} appears twice")
        if row["case"] not in row["local_command"] and not row["local_command"].startswith("see:"):
            raise LedgerError(f"{path}: the local command of {row['case']} must name the case")
        rows[row["case"]] = row
    return rows


def marked(env=None):
    value = (os.environ if env is None else env).get(ENV_NAME, "")
    return [item.strip() for item in value.split(",") if item.strip()]


def registered_cases(root=ROOT):
    listed = subprocess.run(["bash", str(root / "scripts" / "m2-browser.sh"), "--list-cases"], cwd=root,
                            capture_output=True, text=True, check=True).stdout.split()
    refutations = json.loads((root / "tests" / "m2" / "refutations.json").read_text())
    return set(listed) | {case["id"] for case in refutations if case.get("runtime") == "browser"}


def workflow_marks(text):
    """Every case that ci.yml marks local-only. A comment line is not a mark."""
    found = []
    for line in text.splitlines():
        match = MARK_LINE.match(line)
        if match:
            found.extend(item.strip() for item in match.group(1).split(",") if item.strip())
    if ENV_NAME in re.sub(r"(?m)^\s*#.*$", "", text) and not found:
        raise LedgerError(f"ci.yml uses {ENV_NAME} in a form that this check cannot read; write it as {ENV_NAME}: \"a,b\"")
    return found


def check(ledger=LEDGER, workflow=WORKFLOW, root=ROOT):
    problems = []
    try:
        rows = read_ledger(ledger)
        marks = workflow_marks(Path(workflow).read_text())
    except LedgerError as error:
        return [str(error)]
    known = registered_cases(root)
    for case in marks:
        if case not in rows:
            problems.append(f"{case} is marked local-only in {Path(workflow).name} but has no row in {Path(ledger).name}")
        if case not in known:
            problems.append(f"{case} is marked local-only but is not a registered browser case")
    if len(marks) != len(set(marks)):
        problems.append("a case is marked local-only twice in ci.yml")
    for case in rows:
        if case not in known:
            problems.append(f"ledger row {case} is not a registered browser case")
        if case not in marks:
            problems.append(f"ledger row {case} is not marked in ci.yml ({ENV_NAME}); remove the row or mark the case")
    return problems


def notice(row):
    return (f"M2_CASE_LOCAL_ONLY {row['case']}: not run on this runner ({row['reason']}). "
            f"Local gate: {row['local_command']} Receipt: {row['receipt']} Owner: {row['owner']}")


def summary_markdown(row):
    return (f"### Not run on the hosted runner: `{row['case']}`\n"
            f"- Reason: {row['reason']}\n- Local gate: `{row['local_command']}`\n"
            f"- Receipt: {row['receipt']}\n- Owner: {row['owner']}\n\n")


def decide(case, env=None, ledger=LEDGER, summary_path=None):
    """Return 0 (run), 10 (local-only, notice printed) or 2 (error)."""
    listed = marked(env)
    if case in ("all", "product") and listed:
        print(f"local_only: the selector {case!r} cannot be used while {ENV_NAME} is set", file=sys.stderr)
        return 2
    if case not in listed:
        return 0
    try:
        row = read_ledger(ledger).get(case)
    except LedgerError as error:
        print(f"local_only: {error}", file=sys.stderr)
        return 2
    if row is None:
        print(f"local_only: {case} is marked local-only in {ENV_NAME} but has no row in {Path(ledger).name}", file=sys.stderr)
        return 2
    print(notice(row), flush=True)
    target = summary_path if summary_path is not None else (os.environ if env is None else env).get("GITHUB_STEP_SUMMARY")
    if target:
        with open(target, "a") as handle:
            handle.write(summary_markdown(row))
    return 10


def main():
    parser = argparse.ArgumentParser()
    group = parser.add_mutually_exclusive_group(required=True)
    group.add_argument("--decide", metavar="CASE")
    group.add_argument("--count", action="store_true")
    group.add_argument("--check", action="store_true")
    args = parser.parse_args()
    if args.decide:
        return decide(args.decide)
    if args.count:
        listed = set(marked())
        print(sum(1 for line in sys.stdin.read().split() if line in listed))
        return 0
    problems = check()
    for problem in problems:
        print(f"local-only check: {problem}", file=sys.stderr)
    if not problems:
        print(f"local-only check: ok ({len(read_ledger())} rows)")
    return 1 if problems else 0


if __name__ == "__main__":
    sys.exit(main())
