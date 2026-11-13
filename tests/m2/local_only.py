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
  local_only.py --check         static check of the ledger, the registered cases and ci.yml (hosted CI runs this)
  local_only.py --receipts --head SHA
                                freshness of the local receipts in tests/m2/local-receipts.tsv. It is a step of
                                the local gate and of the final MC regression. Hosted CI never runs it: the
                                receipts come from a local GPU run, which a hosted runner cannot make.
"""
import argparse
import fnmatch
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[2]
LEDGER = Path(__file__).with_name("local-only.tsv")
WORKFLOW = ROOT / ".github" / "workflows" / "ci.yml"
RECEIPTS = Path(__file__).with_name("local-receipts.tsv")
RECEIPTS_REL = "tests/m2/local-receipts.tsv"
ENV_NAME = "M2_LOCAL_ONLY_CASES"
COLUMNS = ["case", "reason", "local_command", "receipt", "owner"]
CASE_ID = re.compile(r"E[0-9]-browser-[a-z0-9_-]+")
MARK_LINE = re.compile(rf'^\s*{ENV_NAME}:\s*"([^"]*)"\s*$')


ABSOLUTE_PATH = re.compile(r"(^|[\s`'\"=(])(/|~/|[A-Za-z]:\\)[A-Za-z0-9_.-]")


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
        for name, field in row.items():
            if ABSOLUTE_PATH.search(field):
                raise LedgerError(f"{path}: row {number} field {name} holds an absolute path of one machine; "
                                  "name the file and say where the studio keeps it")
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


def strip_yaml_comment(line):
    """Cut a YAML comment (a # at the start or after a space, outside quotes) from one line."""
    quote = None
    for index, char in enumerate(line):
        if quote:
            if char == quote:
                quote = None
        elif char in "\"'":
            quote = char
        elif char == "#" and (index == 0 or line[index - 1] in " \t"):
            return line[:index]
    return line


def mark_occurrences(text):
    """(job, in_env, indent, line number, line) for every non-comment occurrence of the env name."""
    job, in_env, found = None, False, []
    for number, raw in enumerate(text.splitlines(), 1):
        line = strip_yaml_comment(raw)
        if not line.strip():
            continue
        indent = len(line) - len(line.lstrip(" "))
        if indent == 2 and re.fullmatch(r"  [A-Za-z0-9_-]+:\s*", line):
            job, in_env = line.strip()[:-1], False
        elif indent == 4:
            in_env = line.strip() == "env:"
        if ENV_NAME in line:
            found.append((job, in_env, indent, number, line))
    return found


def workflow_marks(text):
    """Every case that ci.yml marks local-only: exactly one occurrence of the env name (a comment is
    not one), in the canonical form, in the job-level env of the `browser` job."""
    found = mark_occurrences(text)
    if len(found) != 1:
        raise LedgerError(f"ci.yml must hold exactly one non-comment occurrence of {ENV_NAME}, it holds {len(found)}"
                          + "".join(f"; line {number}: {line.strip()}" for _, _, _, number, line in found))
    job, in_env, indent, number, line = found[0]
    if job != "browser" or not in_env:
        raise LedgerError(f"ci.yml line {number}: {ENV_NAME} must be in the env block of the browser job, it is in "
                          f"job {job!r}{'' if in_env else ' outside an env block'}")
    match = MARK_LINE.match(line)
    if not match or indent != 6:
        raise LedgerError(f"ci.yml line {number}: {ENV_NAME} is in a form that this check cannot read; "
                          f"write it as {ENV_NAME}: \"a,b\" with 6 spaces of indent")
    return [item.strip() for item in match.group(1).split(",") if item.strip()]


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


# ---- Local receipts: freshness by unchanged inputs (review follow-up M2) ----
RECEIPT_COLUMNS = ["case", "head_sha", "label", "result", "date", "receipt_sha256"]
PASS_RESULT = "pass"
PLANT_RESULT = "red-and-restored-green"
# Paths whose change after the receipt head makes the receipt stale. The receipts file itself is not an input.
INPUT_PREFIXES = ("tests/m2/", "tests/browser/", "mods/", "crates/turts_client/", "crates/turts_sim/",
                  "crates/turts_net_protocol/", "crates/turts_net_lightyear/", "crates/turts_server/",
                  "crates/turts_pack/", "crates/turts_mod_runtime/", "crates/turts_mod_api/")
INPUT_PATTERNS = ("scripts/m2-*.sh",)


def is_input(path):
    if path == RECEIPTS_REL:
        return False
    return path.startswith(INPUT_PREFIXES) or any(fnmatch.fnmatch(path, pattern) for pattern in INPUT_PATTERNS)


def read_receipts(path=RECEIPTS):
    lines = [line for line in Path(path).read_text().splitlines() if line.strip() and not line.startswith("#")]
    if not lines or lines[0].split("\t") != RECEIPT_COLUMNS:
        raise LedgerError(f"{path}: the first line must be the tab-separated header {RECEIPT_COLUMNS}")
    rows = []
    for number, line in enumerate(lines[1:], 2):
        fields = line.split("\t")
        if len(fields) != len(RECEIPT_COLUMNS) or any(not field.strip() for field in fields):
            raise LedgerError(f"{path}: row {number} must have {len(RECEIPT_COLUMNS)} non-empty tab-separated fields")
        row = dict(zip(RECEIPT_COLUMNS, fields))
        if not re.fullmatch(r"[0-9a-f]{40}", row["head_sha"]) or not re.fullmatch(r"[0-9a-f]{64}", row["receipt_sha256"]):
            raise LedgerError(f"{path}: row {number} needs a 40-digit head_sha and a 64-digit receipt_sha256 in lower-case hex")
        if not re.fullmatch(r"[0-9]{4}-[0-9]{2}-[0-9]{2}", row["date"]) or not re.fullmatch(r"[A-Za-z0-9._-]+", row["label"]):
            raise LedgerError(f"{path}: row {number} needs a date YYYY-MM-DD and a label of letters, digits, dot, dash")
        rows.append(row)
    return rows


def git(root, *args):
    result = subprocess.run(["git", "-C", str(root), *args], capture_output=True, text=True)
    return result.returncode, result.stdout.strip(), result.stderr.strip()


def evidence_root(root):
    override = os.environ.get("MC_EVIDENCE_DIR")
    if override:
        return Path(override)
    code, common, _ = git(root, "rev-parse", "--path-format=absolute", "--git-common-dir")
    return (Path(common).parent if code == 0 else Path(root)) / "target" / "mc-evidence"


def check_evidence(row, folder, plant):
    problems = []
    receipt = folder / "receipt.json"
    log = folder / "command.log"
    if not receipt.is_file() or not log.is_file():
        return [f"{row['case']}: evidence folder {row['label']} lacks receipt.json or command.log"]
    if hashlib.sha256(receipt.read_bytes()).hexdigest() != row["receipt_sha256"]:
        problems.append(f"{row['case']}: receipt_sha256 does not match {row['label']}/receipt.json")
    try:
        data = json.loads(receipt.read_text())
    except ValueError:
        return problems + [f"{row['case']}: {row['label']}/receipt.json is not JSON"]
    if data.get("expected_head") != row["head_sha"]:
        problems.append(f"{row['case']}: {row['label']}/receipt.json was made at head {data.get('expected_head')}, the row says {row['head_sha']}")
    if data.get("command_exit") != 0:
        problems.append(f"{row['case']}: {row['label']}/receipt.json has command_exit {data.get('command_exit')}")
    text = log.read_text(errors="replace")
    wanted = ([rf"^{re.escape(row['case'])}: RED assertion=", rf"^{re.escape(row['case'])}: RESTORED GREEN"] if plant
              else [rf"^M2_CASE_PASS {re.escape(row['case'])}$"])
    for pattern in wanted:
        if not re.search(pattern, text, re.M):
            problems.append(f"{row['case']}: command.log of {row['label']} has no line that matches {pattern}")
    return problems


def check_receipts(head, root=ROOT, ledger=LEDGER, receipts=RECEIPTS, evidence=None, plants=None):
    """Problems of the local receipts at `head` (the checked-out commit). Returns (problems, notes)."""
    problems, notes = [], []
    try:
        cases = list(read_ledger(ledger))
        rows = read_receipts(receipts)
    except LedgerError as error:
        return [str(error)], notes
    code, resolved, error = git(root, "rev-parse", "--verify", f"{head}^{{commit}}")
    if code != 0:
        return [f"--head {head} is not a commit: {error}"], notes
    code, current, _ = git(root, "rev-parse", "HEAD")
    if resolved != current:
        return [f"--head {head} is not the checked-out HEAD {current}; check out the commit that the gate runs at"], notes
    if plants is None:
        plants = {case["id"] for case in json.loads((Path(root) / "tests" / "m2" / "refutations.json").read_text())
                  if case.get("runtime") == "browser"}
    evidence = evidence if evidence is not None else evidence_root(root)
    latest = {}
    for row in rows:
        latest[row["case"]] = row
    for case in cases:
        row = latest.get(case)
        if row is None:
            problems.append(f"ledger row {case} has no receipt row in {Path(receipts).name}")
            continue
        plant = case in plants
        wanted = PLANT_RESULT if plant else PASS_RESULT
        if row["result"] != wanted:
            problems.append(f"{case}: the result is {row['result']!r}, it must be {wanted!r}")
        code, _, error = git(root, "cat-file", "-e", f"{row['head_sha']}^{{commit}}")
        if code != 0:
            problems.append(f"{case}: receipt head {row['head_sha']} is not a commit of this repository")
            continue
        code, changed, error = git(root, "diff", "--name-only", f"{row['head_sha']}..{resolved}")
        if code != 0:
            problems.append(f"{case}: git diff failed: {error}")
            continue
        touched = [path for path in changed.splitlines() if is_input(path)]
        if touched:
            problems.append(f"{case}: the receipt at {row['head_sha'][:12]} is stale, {len(touched)} input file(s) changed since: "
                            + ", ".join(touched[:5]) + (" ..." if len(touched) > 5 else ""))
        folder = Path(evidence) / row["label"]
        if folder.is_dir():
            problems.extend(check_evidence(row, folder, plant))
        else:
            notes.append(f"{case}: evidence folder {row['label']} is not on this machine; the hash and the pass lines were not recomputed")
    return problems, notes


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
    group.add_argument("--receipts", action="store_true")
    parser.add_argument("--head", help="the checked-out commit that the gate runs at (needed with --receipts)")
    args = parser.parse_args()
    if args.receipts:
        if not args.head:
            parser.error("--receipts needs --head SHA")
        problems, notes = check_receipts(args.head)
        for note in notes:
            print(f"local-only receipts: note: {note}", file=sys.stderr)
        for problem in problems:
            print(f"local-only receipts: {problem}", file=sys.stderr)
        if not problems:
            print(f"local-only receipts: ok ({len(read_ledger())} cases)")
        return 1 if problems else 0
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
