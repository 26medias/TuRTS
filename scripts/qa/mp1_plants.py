#!/usr/bin/env python3
"""MP1 plant registry: record checks and the runner (task Q-PLANTS, ruling R-B).

A plant is one planted fault. Its record lives in docs/research/mp1/plants/<task id>.json
(format: docs/research/mp1/plants/README.md). The runner does this for each selected record:

    1. check the record fields, then apply the edit in memory (each anchor occurs exactly once);
    2. run the command on the clean source: it must be green (exit 0);
    3. for each plant of the command group: write the planted source, run the command (it must be
       red with the recorded exit and text, see the README), restore the source, check every byte
       and the git status, and run the command again: that green run must be green, and it is the
       baseline of the next plant (a group of N plants: N red runs and N + 1 green runs).

The runner restores the source in a `finally` block with SIGINT, SIGTERM and SIGHUP held, so a
failed, timed-out or interrupted command cannot leave a plant in the tree. Before the edit it
writes a journal in the git folder (record, file, sha256 of the original and of the planted bytes).
SIGKILL cannot be caught: the next run reads the journal first and restores a file only when its
bytes are the planted bytes. A file that has other bytes, a moved or missing file, or a commit
that holds the plant makes the run refuse (exit 2) with the record and the file; the runner never
overwrites a file that it does not recognise.

The pattern is the one of tests/m2/refutations.py: the anchor rule (:270-282), the semantic red
text that is not an echo line (:149-156), no compile error as red (:145), a source hash before
and after (:284-296). The parallel runner reaches the same code through tests/mp1/refutations.py.

Exit codes: 0 all selected plants are red and restored (or none selected); 1 a plant failed;
2 a record or an edit is refused (no command ran); 130 or 143 interrupted.
`--check` (with --all or --files) is the fast mode: it runs the `prepare` step of every selected
record and nothing else (no command, no file written). Exit 0 all apply; 2 one STALE line for each
stale record.
Python 3 standard library only.
"""

import argparse
import contextlib
import fcntl
import hashlib
import json
import os
import re
import shutil
import signal
import subprocess
import sys
import tempfile
import time
from pathlib import Path, PurePosixPath

ROOT = Path(__file__).resolve().parents[2]
PLANTS_DIR = Path("docs/research/mp1/plants")
FIELDS = ("row", "plant", "files", "command", "env", "headed", "expected_red", "exit", "commit")
OPTIONAL = ("expected_selected",)
JOURNAL_NAME = "mp1-plants.journal"
LOCK_NAME = "mp1-plants.lock"
HEADED_WRAPPER = "mc_headed_v6.sh --mode pre-merge|table"
# Lines that echo a test name or a harness heading are not a red (tests/m2/refutations.py:149-156).
ECHO_PREFIXES = ("thread '", "failures:", "test ")
COMPILE_ERROR = re.compile(r"could not compile|error\[E\d+\]")
HUNK_HEADER = re.compile(r"^@@ -(\d+)(?:,(\d+))? \+(\d+)(?:,(\d+))? @@")
COMMIT = re.compile(r"^[0-9a-f]{7,40}$")
CARGO_TEST = re.compile(r"\bcargo\b(\s+\S+)*?\s+test\b")
HEX64 = re.compile(r"^[0-9a-f]{64}$")
FAILURE_MARKER = re.compile(r"assert|panick|fail|error|expected|must|should|mismatch", re.I)
DEFAULT_TIMEOUT = 1200


class Refused(Exception):
    """The record or its edit cannot be used. No command ran."""


class PlantFail(Exception):
    """A phase of the plant did not give the expected result."""


class Interrupted(BaseException):
    """A termination signal arrived. The source is restored before this leaves run_record."""

    def __init__(self, message, code):
        super().__init__(message)
        self.code = code


def label_of(rec):
    return f"{rec.get('row')}/{rec.get('plant')}"


# ---------------------------------------------------------------------------- record checks

def safe_path(text):
    if not isinstance(text, str) or not text:
        return False
    path = PurePosixPath(text)
    return (not path.is_absolute() and ".." not in path.parts
            and os.path.normpath(text) == text and "\\" not in text)


def check_edit(edit):
    items = edit if isinstance(edit, list) else [edit]
    if not items:
        return ["edit is an empty list"]
    errors = []
    for item in items:
        if not isinstance(item, dict) or set(item) != {"old", "new"}:
            errors.append("each edit is an object with exactly the fields old and new")
        elif not isinstance(item["old"], str) or not item["old"] or not isinstance(item["new"], str):
            errors.append("edit old must be a non-empty string and edit new a string")
        elif item["old"] == item["new"]:
            errors.append("edit old and new are equal")
    return errors


def is_cargo_test(command):
    return bool(CARGO_TEST.search(" ".join(command)))


def check_record(rec, where):
    """Return the list of problems of one record (an empty list: the record is well formed)."""
    if not isinstance(rec, dict):
        return [f"{where}: the record is not an object"]
    errors = []
    missing = [name for name in FIELDS if name not in rec]
    unknown = [name for name in rec if name not in FIELDS + OPTIONAL]
    if missing:
        errors.append(f"{where}: missing field(s) {', '.join(missing)}")
    if unknown:
        errors.append(f"{where}: unknown field(s) {', '.join(unknown)}")
    for name in ("row", "plant", "expected_red"):
        if name in rec and not (isinstance(rec[name], str) and rec[name].strip()):
            errors.append(f"{where}: {name} must be a non-empty string")
    command = rec.get("command")
    if "command" in rec and not (
            isinstance(command, list) and command and all(isinstance(x, str) and x for x in command)):
        errors.append(f"{where}: command must be a non-empty list of strings (no shell; "
                      "use [\"bash\", \"-c\", ...] when a shell is needed)")
    env = rec.get("env")
    if "env" in rec and not (isinstance(env, dict) and all(
            isinstance(k, str) and k and isinstance(v, str) for k, v in env.items())):
        errors.append(f"{where}: env must be an object of strings ({{}} when none)")
    if "headed" in rec and not isinstance(rec["headed"], bool):
        errors.append(f"{where}: headed must be true or false")
    code = rec.get("exit")
    if "exit" in rec and (isinstance(code, bool) or not isinstance(code, int) or code == 0):
        errors.append(f"{where}: exit must be an integer other than 0 (the exit of the red)")
    selected = rec.get("expected_selected")
    if "expected_selected" in rec and not (
            (isinstance(selected, int) and not isinstance(selected, bool) and selected > 0)
            or (isinstance(selected, str) and selected.strip())):
        errors.append(f"{where}: expected_selected must be a count above 0 or a test name")
    if isinstance(command, list) and is_cargo_test([c for c in command if isinstance(c, str)]) \
            and "expected_selected" not in rec:
        errors.append(f"{where}: a cargo test command needs expected_selected: add the count of the tests "
                      "that it runs (for example 1) or the full name of the test, so that a red of "
                      "another test does not count")
    if "commit" in rec and not (isinstance(rec["commit"], str) and COMMIT.match(rec["commit"])):
        errors.append(f"{where}: commit must be a lower-case hex commit id of 7 to 40 digits")
    files = rec.get("files")
    if "files" in rec:
        if not (isinstance(files, list) and files):
            errors.append(f"{where}: files must be a non-empty list")
        else:
            names = []
            for index, entry in enumerate(files):
                here = f"{where}: files[{index}]"
                if not isinstance(entry, dict) or "file" not in entry:
                    errors.append(f"{here} must be an object with a file field")
                    continue
                names.append(entry["file"])
                if not safe_path(entry["file"]):
                    errors.append(f"{here}: file must be a clean relative path inside the repository")
                forms = [k for k in ("patch", "edit") if k in entry]
                extra = [k for k in entry if k not in ("file", "patch", "edit")]
                if len(forms) != 1 or extra:
                    errors.append(f"{here}: needs exactly one of patch or edit and no other field")
                    continue
                try:
                    if forms[0] == "edit":
                        errors += [f"{here}: {m}" for m in check_edit(entry["edit"])]
                    else:
                        replacements_of(entry)
                except Refused as exc:
                    errors.append(f"{here}: {exc}")
            if len(names) != len(set(names)):
                errors.append(f"{where}: a file is listed twice (one entry per file)")
    return errors


def load_records(plants_dir):
    """Read every <task>.json of plants_dir. Raise Refused with all problems when one is bad."""
    records, errors, seen = [], [], {}
    plants_dir = Path(plants_dir)
    for path in sorted(plants_dir.glob("*.json")) if plants_dir.is_dir() else []:
        try:
            data = json.loads(path.read_text(encoding="utf-8"))
        except (OSError, ValueError) as exc:
            errors.append(f"{path.name}: {exc}")
            continue
        if not isinstance(data, list):
            errors.append(f"{path.name}: the top level must be a list of records")
            continue
        for index, rec in enumerate(data):
            where = f"{path.name}[{index}]"
            found = check_record(rec, where)
            errors += found
            if found:
                continue
            if label_of(rec) in seen:
                errors.append(f"{where}: duplicate plant {label_of(rec)} (first at {seen[label_of(rec)]})")
                continue
            seen[label_of(rec)] = where
            records.append({**rec, "_where": where})
    if errors:
        raise Refused("\n".join(errors))
    return records


def select_records(records, files=None, native_only=False):
    """--all is files=None. --files selects the records that edit one of the given files."""
    wanted = None if files is None else {os.path.normpath(f) for f in files}
    chosen = []
    for rec in records:
        if native_only and rec["headed"]:
            continue
        if wanted is not None and not any(os.path.normpath(e["file"]) in wanted for e in rec["files"]):
            continue
        chosen.append(rec)
    return chosen


# ---------------------------------------------------------------------------- the edit

def parse_patch(text, file):
    """Return the (old, new) text of each hunk of a one-file unified diff. Hunk numbers are
    checked against the hunk lines, but the place of the hunk is its old text (exactly once)."""
    lines = text.split("\n")
    if lines and lines[-1] == "":
        lines.pop()
    hunks, current = [], None
    for line in lines:
        header = HUNK_HEADER.match(line)
        if header:
            current = {"old": int(header.group(2) or 1), "new": int(header.group(4) or 1), "rows": []}
            hunks.append(current)
            continue
        if current is None:
            for prefix in ("--- ", "+++ "):
                if line.startswith(prefix):
                    name = line[len(prefix):].split("\t")[0]
                    if name != "/dev/null" and re.sub(r"^[ab]/", "", name) != file:
                        raise Refused(f"patch names {name}, the entry file is {file}")
            continue
        if line.startswith("\\"):
            if not current["rows"]:
                raise Refused("patch has a no-newline marker before any line")
            current["rows"][-1][2] = True
        elif line[:1] in (" ", "+", "-"):
            current["rows"].append([line[0], line[1:], False])
        elif line == "":
            current["rows"].append([" ", "", False])
        else:
            raise Refused(f"patch line is not a hunk line: {line[:40]!r}")
    if not hunks:
        raise Refused("patch has no hunk")
    pairs = []
    for hunk in hunks:
        old_rows = [r for r in hunk["rows"] if r[0] in " -"]
        new_rows = [r for r in hunk["rows"] if r[0] in " +"]
        if len(old_rows) != hunk["old"] or len(new_rows) != hunk["new"]:
            raise Refused("patch hunk line counts do not match its @@ header")
        if not old_rows:
            raise Refused("patch hunk has no context line and no removed line")
        old = "".join(r[1] + ("" if r[2] else "\n") for r in old_rows)
        new = "".join(r[1] + ("" if r[2] else "\n") for r in new_rows)
        if old == new:
            raise Refused("patch hunk changes nothing")
        pairs.append((old, new, True))
    return pairs


def replacements_of(entry):
    """The (old, new, whole_lines) list of one files[] entry."""
    if "patch" in entry:
        if not isinstance(entry["patch"], str) or not entry["patch"].strip():
            raise Refused("patch must be a non-empty string")
        return parse_patch(entry["patch"], entry["file"])
    edit = entry["edit"]
    return [(item["old"], item["new"], False) for item in (edit if isinstance(edit, list) else [edit])]


def apply_replacements(file, text, replacements):
    """The anchor rule of tests/m2/refutations.py:270-282: each old text occurs exactly once."""
    for old, new, whole_lines in replacements:
        count = text.count(old)
        if count != 1:
            raise Refused(f"{file}: anchor occurs {count} times, expected exactly one")
        at = text.find(old)
        if whole_lines and at > 0 and text[at - 1] != "\n":
            raise Refused(f"{file}: the patch text does not start at a line start")
        text = text[:at] + new + text[at + len(old):]
    return text


def apply_plant_case(original, case):
    """Same rule for a case of the parallel runner (tests/mp1/refutations.py)."""
    pairs = [(r["anchor"], r["replacement"], r.get("whole_lines", False)) for r in case["replacements"]]
    try:
        return apply_replacements(case.get("path", "?"), original, pairs)
    except Refused as exc:
        raise Refused(f"{case['id']}: {exc}") from None


# ---------------------------------------------------------------------------- run one command

def kill_group(proc):
    try:
        os.killpg(proc.pid, signal.SIGKILL)
    except (ProcessLookupError, PermissionError):
        pass


def run_command(command, cwd, env, timeout):
    """Run one command in its own process group. Return (exit code, output); the exit code is
    None on timeout. Output is stdout and stderr together."""
    try:
        proc = subprocess.Popen(command, cwd=cwd, env=env, stdin=subprocess.DEVNULL,
                                stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
                                start_new_session=True)
    except OSError as exc:
        return 127, f"cannot start {command[0]}: {exc}\n"
    try:
        out, _ = proc.communicate(timeout=timeout)
    except subprocess.TimeoutExpired:
        kill_group(proc)
        out, _ = proc.communicate()
        return None, out.decode("utf-8", "replace")
    except BaseException:
        kill_group(proc)
        proc.wait()
        raise
    return proc.returncode, out.decode("utf-8", "replace")


def tail(output, lines=12):
    return "\n".join(output.rstrip("\n").splitlines()[-lines:])[-1500:]


def changed_lines(original_text, planted_text):
    """The stripped lines that the edit added to a file: a command that prints the file prints them."""
    before = {line.strip() for line in original_text.splitlines()}
    return {line.strip() for line in planted_text.splitlines() if line.strip() and line.strip() not in before}


def is_edit_line(line, changed):
    text = line.strip()
    if not text:
        return False
    if text in changed:
        return True
    if FAILURE_MARKER.search(text):  # a failure message that quotes the edited line is a true red
        return False
    return any(len(c) >= 8 and c in text for c in changed)


def find_red_line(expected, output, changed=()):
    """The first line (or block of lines) that prints the expected text and is not an echo line
    and not a line of the plant's own edit. Multi-line text gets the same filter, line by line."""
    lines = output.splitlines()
    span = expected.count("\n")
    at = output.find(expected)
    while at != -1:
        first = output.count("\n", 0, at)
        block = lines[first:first + span + 1]
        if not any(line.lstrip().startswith(ECHO_PREFIXES) or is_edit_line(line, changed) for line in block):
            return block[0].strip()
        at = output.find(expected, at + 1)
    return None


def check_selected(rec, phase, output):
    """The guard of tests/m3/refutations.py:143: a red of another test does not count.
    expected_selected is a count (sum of the `running N tests` lines) or a full test name."""
    wanted = rec.get("expected_selected")
    if wanted is None:
        return
    if isinstance(wanted, int):
        found = sum(int(n) for n in re.findall(r"^running (\d+) tests?", output, re.M))
        if found != wanted:
            raise PlantFail(f"{phase}: {wanted} selected test(s) expected, the output says {found}")
        return
    state = "FAILED" if phase == "red" else "ok"
    if not re.search(rf"^test {re.escape(wanted)} \.\.\. {state}\b", output, re.M):
        raise PlantFail(f"{phase}: test {wanted} did not report {state}")
    others = [n for n in re.findall(r"^test (.+?) \.\.\. FAILED", output, re.M) if n != wanted]
    if others:
        raise PlantFail(f"{phase}: another test failed too: {', '.join(others)}")


def judge_green(phase, code, output, rec=None):
    if code is None:
        raise PlantFail(f"{phase}: the command timed out")
    if code != 0:
        raise PlantFail(f"{phase} is not green: exit {code}\n{tail(output)}")
    if rec is not None:
        check_selected(rec, phase, output)


def judge_red(rec, code, output, baseline_output="", changed=()):
    """Return the red line, or raise PlantFail. baseline_output is the output of the green run."""
    if code is None:
        raise PlantFail("red run: the command timed out")
    if COMPILE_ERROR.search(output):
        raise PlantFail(f"the plant caused a compile failure, not a red\n{tail(output)}")
    if code == 0:
        raise PlantFail("plant stayed green (exit 0)")
    if code != rec["exit"]:
        raise PlantFail(f"red exit {code}, the record says {rec['exit']}\n{tail(output)}")
    check_selected(rec, "red", output)
    if rec["expected_red"] in baseline_output:
        raise PlantFail(f"red text {rec['expected_red']!r} is also printed by the green run: it proves nothing")
    line = find_red_line(rec["expected_red"], output, changed)
    if line is None:
        raise PlantFail(f"red text {rec['expected_red']!r} is not printed on a line of the red run that is "
                        f"not an echo line and not a line of the plant's edit\n{tail(output)}")
    return line


# ---------------------------------------------------------------------------- journal, lock, restore

IN_FLIGHT = []  # files with a plant in them whose restore is not checked yet


def git(root, *args):
    return subprocess.run(["git", "-C", str(root), *args], text=True, stdout=subprocess.PIPE,
                          stderr=subprocess.PIPE)


def sha256_of(data):
    return hashlib.sha256(data).hexdigest()


def set_hash(blobs):
    """One file: its sha256. Several: sha256 of the sorted JSON map (tests/m2/refutations.py:284)."""
    hashes = {name: sha256_of(data) for name, data in blobs.items()}
    if len(hashes) == 1:
        return next(iter(hashes.values()))
    return sha256_of(json.dumps(hashes, sort_keys=True).encode())


def git_dir(root):
    done = git(root, "rev-parse", "--absolute-git-dir")
    return Path(done.stdout.strip()) if done.returncode == 0 else None


@contextlib.contextmanager
def signals_blocked():
    """The restore and its check run with SIGINT, SIGTERM and SIGHUP held; they arrive after it."""
    held = {signal.SIGINT, signal.SIGTERM, signal.SIGHUP}
    old = signal.pthread_sigmask(signal.SIG_BLOCK, held)
    try:
        yield
    finally:
        signal.pthread_sigmask(signal.SIG_SETMASK, old)


@contextlib.contextmanager
def run_lock(root, use_git):
    """One runner in one tree: flock on <git-dir>/mp1-plants.lock (a second runner would see the
    plant of the first one in its own red run)."""
    directory = git_dir(root) if use_git else None
    if directory is None:
        yield
        return
    path = directory / LOCK_NAME
    handle = os.open(path, os.O_CREAT | os.O_RDWR, 0o644)
    try:
        try:
            fcntl.flock(handle, fcntl.LOCK_EX | fcntl.LOCK_NB)
        except BlockingIOError:
            raise Refused(f"another mp1_plants.py run holds {path}; wait for it, or stop it") from None
        yield
    finally:
        os.close(handle)


def journal_write(root, entries):
    path = git_dir(root) / JOURNAL_NAME
    tmp = path.with_suffix(".tmp")
    with open(tmp, "w") as handle:
        handle.write(json.dumps(entries))
        handle.flush()
        os.fsync(handle.fileno())
    os.replace(tmp, path)


def journal_clear(root):
    (git_dir(root) / JOURNAL_NAME).unlink(missing_ok=True)


def journal_entries(path):
    """Read the journal. A wrong shape is refused with a clear text: never a traceback, never a delete."""
    def bad(why):
        return Refused(f"{path} is not a valid plant journal ({why}); the runner changed nothing. "
                       "Look at `git status`, put right any file that holds a plant by hand, "
                       "then delete the journal")
    try:
        data = json.loads(path.read_text())
    except (OSError, ValueError) as exc:
        raise bad(f"cannot read it: {exc}") from None
    if not isinstance(data, list) or not data:
        raise bad("it is not a non-empty list")
    for index, entry in enumerate(data):
        ok = (isinstance(entry, dict) and isinstance(entry.get("record"), str)
              and isinstance(entry.get("file"), str) and safe_path(entry.get("file"))
              and isinstance(entry.get("sha256"), str) and HEX64.match(entry["sha256"])
              and isinstance(entry.get("planted_sha256"), str) and HEX64.match(entry["planted_sha256"])
              and isinstance(entry.get("edit"), list))
        if not ok:
            raise bad(f"entry {index} lacks record, file, sha256, planted_sha256 or edit")
    return data


def plant_text(entry):
    parts = [f"{str(item.get('old'))[:120]!r} by {str(item.get('new'))[:120]!r}" for item in entry["edit"]]
    return "the plant replaced " + "; ".join(parts)


def head_bytes(root, name):
    done = subprocess.run(["git", "-C", str(root), "show", f"HEAD:{name}"], stdout=subprocess.PIPE,
                          stderr=subprocess.PIPE)
    return done.stdout if done.returncode == 0 else None


def recover_journal(root):
    """A run that died (SIGKILL) leaves its plant in the tree and its journal in the git folder.
    Restore a file only when its bytes are the planted bytes and HEAD has the original bytes.
    When the bytes are the original bytes, drop the entry. In every other case refuse (exit 2)
    and change nothing: the runner never overwrites a file that it does not recognise."""
    path = git_dir(root) / JOURNAL_NAME
    if not path.exists():
        return []
    plan, problems = [], []
    for entry in journal_entries(path):
        name, record, text = entry["file"], entry["record"], plant_text(entry)
        target = Path(root) / name
        if not target.is_file():
            problems.append(f"{record}: {name} is missing (moved or deleted?). A moved copy may hold the "
                            f"plant: look at it by hand. {text}")
            continue
        have = sha256_of(target.read_bytes())
        if have == entry["sha256"]:
            plan.append((entry, "drop"))
            continue
        at_head = head_bytes(root, name)
        head_sha = sha256_of(at_head) if at_head is not None else None
        if have != entry["planted_sha256"]:
            problems.append(f"{record}: {name} differs from both the plant and the original: look at it by "
                            f"hand. {text}")
        elif head_sha == entry["planted_sha256"]:
            commit = git(root, "rev-parse", "--short", "HEAD").stdout.strip()
            problems.append(f"{record}: the commit {commit} holds the plant: {name} at HEAD is the planted "
                            f"text. Revert that change by hand. {text}")
        elif head_sha != entry["sha256"]:
            problems.append(f"{record}: HEAD no longer has the original bytes of {name}: look at "
                            f"`git log -- {name}` by hand. {text}")
        else:
            plan.append((entry, "restore"))
    if problems:
        raise Refused(f"an interrupted run left a plant journal ({path}); the runner changed nothing:\n  "
                      + "\n  ".join(problems) + "\n  When the files are right, delete the journal.")
    lines = []
    for entry, action in plan:
        name, record = entry["file"], entry["record"]
        if action == "drop":
            lines.append(f"journal entry of {record} for {name} dropped: the file has its original bytes")
            continue
        git(root, "checkout", "HEAD", "--", name)
        if sha256_of((Path(root) / name).read_bytes()) != entry["sha256"]:
            raise Refused(f"cannot restore {name}, left by a plant of {record}: the bytes are still "
                          "not the original; look at it by hand")
        lines.append(f"a plant of {record} was left in {name} by an interrupted run; restored")
    path.unlink()
    return lines


def tree_state(root):
    return set(git(root, "status", "--porcelain", "-uall").stdout.splitlines())


def default_restore(root, originals, use_git):
    if use_git:
        git(root, "checkout", "HEAD", "--", *originals)
    else:
        for name, data in originals.items():
            (Path(root) / name).write_bytes(data)


def check_restored(root, originals, use_git, tree_before):
    """Return the problems of the restore and repair the bytes when they differ."""
    problems = []
    for name, data in originals.items():
        path = Path(root) / name
        if not path.is_file() or path.read_bytes() != data:
            path.write_bytes(data)
            problems.append(f"{name} was not byte-exact after the restore")
    if use_git:
        dirty = git(root, "status", "--porcelain", "--", *originals).stdout.strip()
        if dirty:
            problems.append(f"git status is not empty after the restore: {dirty}")
        if tree_before is not None:
            after = tree_state(root)
            new, gone = sorted(after - tree_before), sorted(tree_before - after)
            if new or gone:
                problems.append("the command changed files outside the plant: "
                                + ", ".join(line.strip() for line in new + gone))
    return problems


# ---------------------------------------------------------------------------- run records

def where_of(rec):
    return rec.get("_where") or "record"


def refusal(rec, text):
    return Refused(f"{label_of(rec)} [{where_of(rec)}]: {text}")


def missing_files(root, records):
    """One refusal text for each record that names a file that is not there."""
    errors = []
    for rec in records:
        for entry in rec["files"]:
            if not (Path(root) / entry["file"]).is_file():
                errors.append(str(refusal(rec, f"{entry['file']} is missing: the file was moved or "
                                               "deleted; re-pin the record")))
    return errors


def check_files_exist(root, records):
    """Every record of every file, in every mode: a moved or deleted file is a stale record."""
    errors = missing_files(root, records)
    if errors:
        raise Refused("\n".join(errors))


def known_path(root, relative, use_git):
    """A tracked file, or a path that a change deleted (in the index, or in a commit): raw
    `git diff --name-only` output lists deleted paths too."""
    if not use_git:
        return (Path(root) / relative).is_file()
    if relative in git(root, "ls-files", "--", relative).stdout.splitlines():
        return True
    if git(root, "diff", "--cached", "--name-only", "--diff-filter=D", "--", relative).stdout.strip():
        return True
    return bool(git(root, "log", "-1", "--no-renames", "--diff-filter=D", "--format=%h", "--",
                    relative).stdout.strip())


def normalize_files(root, files, records, use_git):
    """Repository-relative names of the --files arguments. An absolute path under the root is made
    relative. A path that is neither a tracked file nor in a record is a mistake: refuse it."""
    in_records = {entry["file"] for rec in records for entry in rec["files"]}
    chosen, bad = [], []
    for name in files:
        path = Path(name)
        if path.is_absolute():
            try:
                path = path.resolve().relative_to(Path(root).resolve())
            except ValueError:
                bad.append(f"{name}: outside the repository {root}")
                continue
        relative = os.path.normpath(str(path))
        known = known_path(root, relative, use_git)
        if relative in in_records or known:
            chosen.append(relative)
        else:
            bad.append(f"{name}: not a tracked file, not deleted by a change, and not in any record "
                       "(a mistyped path?)")
    if bad:
        raise Refused("--files: " + "; ".join(bad))
    return chosen


def prepare(root, rec, use_git):
    """Read the files and compute the planted text. Raise Refused when the plant cannot apply.
    A file must be tracked and clean, so that `git checkout` restores it exactly; this check comes
    before the anchor check."""
    originals, planted = {}, {}
    for entry in rec["files"]:
        name = entry["file"]
        path = Path(root) / name
        if not path.is_file():
            raise refusal(rec, f"{name} is missing: the file was moved or deleted; re-pin the record")
        if use_git:
            # --no-optional-locks: a read-only check must not refresh the stat cache in .git/index.
            if git(root, "--no-optional-locks", "ls-files", "--error-unmatch", "--", name).returncode != 0:
                raise refusal(rec, f"{name} is not tracked by git")
            if git(root, "--no-optional-locks", "status", "--porcelain", "--", name).stdout.strip():
                raise refusal(rec, f"{name} is not clean: git status shows a change in it")
        originals[name] = path.read_bytes()
        try:
            text = originals[name].decode("utf-8")
        except UnicodeDecodeError:
            raise refusal(rec, f"{name} is not UTF-8 text") from None
        try:
            planted[name] = apply_replacements(name, text, replacements_of(entry))
        except Refused as exc:
            reason = str(exc) if name in str(exc) else f"{name}: {exc}"
            raise refusal(rec, f"{reason}; when the file changed, re-pin the record") from None
    if all(planted[name].encode("utf-8") == originals[name] for name in originals):
        raise refusal(rec, "the plant changes nothing: the edits of the record cancel each other, so "
                           "the plant would stay green")
    return originals, planted


def group_key(rec):
    """Plants with the same command, environment and selection guard form a command group."""
    return json.dumps([rec["command"], rec["env"], rec.get("expected_selected")], sort_keys=True)


def run_group(root, recs, *, execute=None, restore=None, timeout=DEFAULT_TIMEOUT, extra_env=None,
              use_git=None):
    """Run the plants of one command group: a baseline, then for each plant the edit, the red run,
    a byte-checked restore and a green run. That green run is the baseline of the next plant, so a
    group of N plants has N red runs and N + 1 green runs. A green run that fails after a restore
    fails that plant and stops the group (the plants that did not run fail with "not run").
    Return one result for each record. Raise Refused when a plant cannot apply (no command ran)."""
    root = Path(root)
    use_git = (root / ".git").exists() if use_git is None else use_git
    restore = restore or default_restore
    prepared = [prepare(root, rec, use_git) for rec in recs]
    if execute is None:
        def execute(rec):
            env = {**os.environ, **(extra_env or {}), **rec["env"]}
            return run_command(rec["command"], root, env, timeout)
    tree_before = tree_state(root) if use_git else None
    results = [{"row": rec["row"], "plant": rec["plant"], "verdict": "PLANT-FAIL", "reason": None,
                "baseline_exit": None, "red_exit": None, "red_line": None, "red_output": "",
                "baseline_output": "", "restored_exit": None, "restored_output": "",
                "baseline_sha256": set_hash(originals), "restored_sha256": None, "seconds": 0.0}
               for rec, (originals, _) in zip(recs, prepared)]

    def finish(result, started):
        result["seconds"] = round(time.monotonic() - started, 1)
        result["verdict"] = "PLANT-FAIL" if result["reason"] else "RED-RESTORED"

    started = time.monotonic()
    code, green_out = execute(recs[0])
    try:
        judge_green("baseline", code, green_out, recs[0])
    except PlantFail as exc:
        for result in results:
            result.update(reason=str(exc), baseline_exit=code, baseline_output=green_out)
            finish(result, started)
        return results
    stopped = None
    for rec, (originals, planted), result in zip(recs, prepared, results):
        cycle = time.monotonic()
        if stopped:
            result["reason"] = f"not run: the group stopped after {stopped} failed its green run"
            finish(result, cycle)
            continue
        result.update(baseline_exit=code, baseline_output=green_out)
        failure = None
        try:
            changed = set()
            for name, text in planted.items():
                changed |= changed_lines(originals[name].decode("utf-8"), text)
            if use_git:
                journal_write(root, [
                    {"record": label_of(rec), "file": entry["file"],
                     "sha256": sha256_of(originals[entry["file"]]),
                     "planted_sha256": sha256_of(planted[entry["file"]].encode("utf-8")),
                     "edit": [{"old": old, "new": new} for old, new, _ in replacements_of(entry)]}
                    for entry in rec["files"]])
            IN_FLIGHT[:] = list(originals)
            for name, text in planted.items():
                (root / name).write_bytes(text.encode("utf-8"))
            red_code, red_out = execute(rec)
            result["red_exit"], result["red_output"] = red_code, red_out
            result["red_line"] = judge_red(rec, red_code, red_out, green_out, changed)
        except PlantFail as exc:
            failure = str(exc)
        finally:
            with signals_blocked():
                restore(root, originals, use_git)
                problems = check_restored(root, originals, use_git, tree_before)
                if use_git and IN_FLIGHT:
                    journal_clear(root)
                IN_FLIGHT.clear()
            result["restored_sha256"] = set_hash({n: (root / n).read_bytes() for n in originals})
        if problems:
            failure = "; ".join(([failure] if failure else []) + problems)
        code, green_out = execute(rec)   # the green run after the restore: the baseline of the next plant
        result.update(restored_exit=code, restored_output=green_out)
        try:
            judge_green("restored", code, green_out, rec)
        except PlantFail as exc:
            failure = "; ".join(([failure] if failure else []) + [f"{exc} (the green run after the restore)"])
            stopped = label_of(rec)
        result["reason"] = failure
        finish(result, cycle)
    return results


def run_record(root, rec, **kwargs):
    """One plant: a group of one (baseline, red, restore, green). Return its result."""
    return run_group(root, [rec], **kwargs)[0]


def describe(result):
    """One report line (the first line); the next lines hold the output tail."""
    name = f"{result['row']}/{result['plant']}"
    if result["verdict"] == "RED-RESTORED":
        return f"RED-RESTORED {name} exit={result['red_exit']} red={result['red_line']!r}"
    first, *rest = result["reason"].split("\n")
    return "\n".join([f"PLANT-FAIL {name}: {first}"] + [f"    {line}" for line in rest])


def public(result):
    return {k: v for k, v in result.items() if not k.endswith("_output")}


# ---------------------------------------------------------------------------- the command line

def refusals_of(root, records, use_git):
    """One refusal text for each record that does not apply to the tree: `prepare` is the one
    check of the run mode and of --check. It reads files and runs `git ls-files` and `git status`;
    it writes no file and runs no command of a record."""
    refused = []
    for rec in records:
        try:
            prepare(root, rec, use_git)
        except Refused as exc:
            refused.append(str(exc))
    return refused


def run_state_notes(root, use_git):
    """Why a "not clean" line of --check can be false: a journal that a killed run left (the next run
    mode recovers it), or a run in flight (it holds the lock; its plant is in the tree). Read-only:
    the lock file is opened for reading and tried with a shared, non-blocking flock; nothing is
    created, recovered or deleted."""
    directory = git_dir(root) if use_git else None
    if directory is None:
        return []
    notes = []
    if (directory / JOURNAL_NAME).exists():
        notes.append(f"{directory / JOURNAL_NAME} exists: a killed run left a plant, so a \"not clean\" "
                     "line below can be false. Run the run mode once to recover it (the run mode does "
                     "that first); --check does not recover.")
    try:
        handle = os.open(directory / LOCK_NAME, os.O_RDONLY)
    except OSError:
        return notes
    try:
        fcntl.flock(handle, fcntl.LOCK_SH | fcntl.LOCK_NB)
    except BlockingIOError:
        notes.append(f"{directory / LOCK_NAME} is held: a run in flight has a plant in the tree, so a "
                     "\"not clean\" line below can be false. Wait for that run to end, then check again.")
    finally:
        os.close(handle)
    return notes


def stale_records(root, records, chosen, use_git):
    """--check: return (the number of records of `chosen` that apply, one text for each stale
    record). A record that --files did not select but whose file is gone is stale too: the run mode
    refuses it in every mode (`check_files_exist`)."""
    stale = refusals_of(root, chosen, use_git)
    chosen_where = {rec["_where"] for rec in chosen}
    unselected = [r for r in records if r["_where"] not in chosen_where]
    return len(chosen) - len(stale), stale + missing_files(root, unselected)


def run_selected(root, records, args):
    use_git = (Path(root) / ".git").exists()
    refused = refusals_of(root, records, use_git)
    if refused:
        for message in refused:
            print(f"REFUSED {message}", flush=True)
        print(f"mp1_plants: {len(refused)} record(s) refused, no command ran", flush=True)
        return 2
    native = [rec for rec in records if not rec["headed"]]
    for rec in records:
        if rec["headed"]:
            print(f"SKIPPED {label_of(rec)} (headed: it runs only through {HEADED_WRAPPER}, never here)",
                  flush=True)
    groups = {}
    for rec in native:
        groups.setdefault(group_key(rec), []).append(rec)
    results = []
    for group in groups.values():
        for result in run_group(root, group, timeout=args.timeout, use_git=use_git):
            results.append(result)
            print(describe(result), flush=True)
    failed = [r for r in results if r["verdict"] != "RED-RESTORED"]
    print(f"mp1_plants: {len(results)} plant(s) in {len(groups)} command group(s), "
          f"{len(results) - len(failed)} red-restored, {len(failed)} failed", flush=True)
    if args.out:
        Path(args.out).write_text(json.dumps([public(r) for r in results], indent=2) + "\n")
    return 1 if failed else 0


def terminate(signum, _frame):
    raise Interrupted(f"signal {signum}", 128 + signum)


def parse(argv):
    parser = argparse.ArgumentParser(
        description="Run the MP1 plant records: red, restore, green (see the module text).")
    parser.add_argument("--self-test", action="store_true", help="run the self-test cases")
    parser.add_argument("--all", action="store_true", help="run every record")
    parser.add_argument("--files", nargs="*", metavar="FILE",
                        help="run the records that edit one of these repository paths")
    parser.add_argument("--native-only", action="store_true",
                        help="kept for old command lines: this runner never runs headed records")
    parser.add_argument("--list", action="store_true", help="print the selected records and run nothing")
    parser.add_argument("--check", action="store_true",
                        help="fast: apply the same checks as the run mode, run no command and change no "
                             "file; exit 2 with one STALE line for each record that does not apply")
    parser.add_argument("--root", type=Path, default=ROOT, help="the git worktree (default: this one)")
    parser.add_argument("--plants-dir", type=Path, help="default: <root>/" + str(PLANTS_DIR))
    parser.add_argument("--timeout", type=int, default=DEFAULT_TIMEOUT, help="seconds for each command run")
    parser.add_argument("--out", help="write the results as JSON to this path")
    args = parser.parse_args(argv)
    if not args.self_test and (args.all == (args.files is not None)):
        parser.error("give exactly one of --all or --files")
    if args.check and (args.list or args.self_test):
        parser.error("--check cannot be used with --list or --self-test")
    return args


def select_from(root, plants_dir, args, use_git, require_files=True):
    records = load_records(plants_dir)
    if require_files:
        check_files_exist(root, records)
    files = None if args.all else normalize_files(root, args.files, records, use_git)
    return records, select_records(records, files, args.native_only)


def interrupted_text():
    if IN_FLIGHT:
        return ("interrupted while a plant was in " + ", ".join(IN_FLIGHT)
                + "; the next run restores it from the journal")
    return "interrupted; every plant file is restored and checked"


def main(argv=None):
    args = parse(argv)
    if args.self_test:
        return self_test()
    # A parent that started this process with SIGINT ignored must not disable the restore.
    signal.signal(signal.SIGINT, signal.default_int_handler)
    for number in (signal.SIGTERM, signal.SIGHUP):
        signal.signal(number, terminate)
    root = args.root.resolve()
    plants_dir = args.plants_dir or root / PLANTS_DIR
    use_git = (root / ".git").exists()
    try:
        if args.list:
            _, chosen = select_from(root, plants_dir, args, use_git)
            for rec in chosen:
                print(f"{label_of(rec)} headed={rec['headed']} "
                      f"files={','.join(e['file'] for e in rec['files'])}")
            return 0
        if args.check:
            records, chosen = select_from(root, plants_dir, args, use_git, require_files=False)
            applying, stale = stale_records(root, records, chosen, use_git)
            for note in run_state_notes(root, use_git):
                print(f"NOTE {note}", flush=True)
            for message in stale:
                print(f"STALE {message}", flush=True)
            print(f"mp1_plants: --check: {applying} record(s) apply, {len(stale)} stale", flush=True)
            return 2 if stale else 0
        with run_lock(root, use_git):
            if use_git:
                for line in recover_journal(root):
                    print(line, flush=True)
            records, chosen = select_from(root, plants_dir, args, use_git)
            if not chosen:
                print(f"mp1_plants: no plant selected ({len(records)} record(s) in {plants_dir})")
                return 0
            return run_selected(root, chosen, args)
    except Refused as exc:
        print(f"REFUSED {exc}", file=sys.stderr)
        return 2
    except KeyboardInterrupt:
        print(f"mp1_plants: {interrupted_text()}", file=sys.stderr)
        return 130
    except Interrupted as exc:
        print(f"mp1_plants: {interrupted_text()} ({exc})", file=sys.stderr)
        return exc.code


# ---------------------------------------------------------------------------- the self-test

CHECK = (
    "import os, sys\n"
    "a = open('a.txt').read()\n"
    "b = open('b.txt').read()\n"
    "bad = []\n"
    "if 'GOOD' not in a: bad.append('RED a lost GOOD')\n"
    "if 'two' not in b: bad.append('RED b lost two')\n"
    "for line in bad: print(line)\n"
    "sys.exit(1 if bad else 0)\n"
)
CHECK_EXIT3 = CHECK.replace("sys.exit(1 if bad else 0)", "sys.exit(3 if bad else 0)")
CHECK_COMPILE = CHECK.replace("print(line)", "print('error[E0425]: ' + line)")
CHECK_ENV = ("import os, sys\nif os.environ.get('MP1_X') != '1':\n    print('ENV MISSING')\n"
             "    sys.exit(3)\n" + CHECK)
CHECK_ECHO = CHECK.replace("print(line)", "print('test x::GOOD ... FAILED')")
CHECK_SLEEP = ("import os, time\nif 'GOOD' not in open('a.txt').read():\n"
               "    open('child.pid', 'w').write(str(os.getpid()))\n    time.sleep(60)\n")
CHECK_LINGER = ("import os, sys\nn = len(open('count.txt').read()) if os.path.exists('count.txt') else 0\n"
                "open('count.txt', 'a').write('x')\nif n >= 1:\n    print('RED lingering')\n    sys.exit(1)\n")
CHECK_TOUCH = "open('b.txt', 'a').write('side effect\\n')\n" + CHECK
CHECK_FLAG = "open('ran.flag', 'w').write('x')\n"
CHECK_RED = "import sys\nprint('RED always')\nsys.exit(1)\n"
CASES = []


def case(function):
    CASES.append((function.__name__, function))
    return function


def expect(condition, message):
    if not condition:
        raise AssertionError(message)


def expect_refused(function, text):
    try:
        function()
    except Refused as exc:
        expect(text in str(exc), f"refusal text lacks {text!r}: {exc}")
        return str(exc)
    raise AssertionError(f"not refused (wanted {text!r})")


class Fixture:
    """A small git repository in a temporary folder: a.txt and b.txt, committed."""

    FILES = {"a.txt": "alpha GOOD\nbeta\n", "b.txt": "one\ntwo\n"}

    def __init__(self, base):
        self.base = Path(base)
        self.root = self.base / "repo"
        self.plants = self.base / "plants"
        self.root.mkdir()
        self.plants.mkdir()
        for name, text in self.FILES.items():
            (self.root / name).write_text(text)
        # Files that a command may write on purpose are ignored; any other new file is a finding.
        (self.root / ".gitignore").write_text("count.txt\nchild.pid\nran.flag\nruns.log\nheaded.ran\nstarted.flag\nignored/\n")
        self.git("init", "-q")
        self.git("add", ".gitignore", "a.txt", "b.txt")
        self.commit("fixture")

    def commit(self, message):
        self.git("-c", "user.name=t", "-c", "user.email=t@example.com", "-c", "commit.gpgsign=false",
                 "commit", "-q", "-m", message)

    def install_git_shim(self):
        """A git that sleeps before `checkout`, so a signal can land inside the restore."""
        real = shutil.which("git")
        shim = self.base / "bin" / "git"
        shim.parent.mkdir()
        shim.write_text("#!/bin/sh\nfor a in \"$@\"; do\n    if [ \"$a\" = checkout ]; then\n"
                        "        : > \"$MP1_MARK\"; sleep 2\n    fi\ndone\nexec " + real + " \"$@\"\n")
        shim.chmod(0o755)
        mark = self.base / "checkout.mark"
        return {**os.environ, "PATH": f"{shim.parent}:{os.environ['PATH']}", "MP1_MARK": str(mark)}, mark

    def journal(self):
        return Path(self.git("rev-parse", "--absolute-git-dir").strip()) / "mp1-plants.journal"

    def git(self, *args):
        done = git(self.root, *args)
        expect(done.returncode == 0, f"git {args}: {done.stderr}")
        return done.stdout

    def write_records(self, records, name="t.json"):
        (self.plants / name).write_text(json.dumps(records))

    def cli(self, *argv, timeout=120):
        done = subprocess.run([sys.executable, "-B", str(Path(__file__).resolve()), *argv,
                               "--root", str(self.root), "--plants-dir", str(self.plants)],
                              text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, timeout=timeout)
        return done.returncode, done.stdout

    def assert_restored(self):
        for name, text in self.FILES.items():
            expect((self.root / name).read_text() == text, f"{name} is not restored")
        expect(self.git("status", "--porcelain", "--untracked-files=no") == "", "git status is not empty")


def make_record(**over):
    rec = {"row": "T1", "plant": "a",
           "files": [{"file": "a.txt", "edit": {"old": "GOOD", "new": "BAD"}}],
           "command": [sys.executable, "-c", CHECK], "env": {}, "headed": False,
           "expected_red": "RED a lost GOOD", "exit": 1, "commit": "d7d7e6d"}
    rec.update(over)
    return rec


@case
def missing_expected_red_is_refused(fx):
    rec = make_record()
    del rec["expected_red"]
    fx.write_records([rec])
    expect_refused(lambda: load_records(fx.plants), "expected_red")
    fx.write_records([make_record(expected_red="")])
    expect_refused(lambda: load_records(fx.plants), "expected_red")
    fx.write_records([make_record()])
    expect(len(load_records(fx.plants)) == 1, "a good record is not loaded")


@case
def record_fields_are_checked(fx):
    bad = {
        "exit 0": make_record(exit=0), "exit text": make_record(exit="1"),
        "commit": make_record(commit="zzz"), "headed": make_record(headed="yes"),
        "env value": make_record(env={"A": 1}), "no files": make_record(files=[]),
        "unknown": make_record(extra=1), "shell string": make_record(command="make test"),
        "both forms": make_record(files=[{"file": "a.txt", "patch": "x", "edit": {"old": "a", "new": "b"}}]),
        "absolute": make_record(files=[{"file": "/etc/passwd", "edit": {"old": "a", "new": "b"}}]),
        "dotdot": make_record(files=[{"file": "../a.txt", "edit": {"old": "a", "new": "b"}}]),
        "same file twice": make_record(files=[{"file": "a.txt", "edit": {"old": "a", "new": "b"}}] * 2),
        "old equals new": make_record(files=[{"file": "a.txt", "edit": {"old": "a", "new": "a"}}]),
    }
    for name, rec in bad.items():
        fx.write_records([rec])
        try:
            load_records(fx.plants)
        except Refused:
            continue
        raise AssertionError(f"a record with a bad {name} is not refused")
    fx.write_records([make_record(), make_record()])
    expect_refused(lambda: load_records(fx.plants), "duplicate plant T1/a")


@case
def patch_that_does_not_apply_is_refused(fx):
    patch = "--- a/a.txt\n+++ b/a.txt\n@@ -1,2 +1,2 @@\n alpha MISSING\n-beta\n+gamma\n"
    rec = make_record(files=[{"file": "a.txt", "patch": patch}])
    expect_refused(lambda: run_record(fx.root, rec), "occurs 0 times")
    fx.assert_restored()


@case
def patch_that_applies_goes_red(fx):
    patch = "--- a/a.txt\n+++ b/a.txt\n@@ -1,2 +1,2 @@\n-alpha GOOD\n+alpha BAD\n beta\n"
    result = run_record(fx.root, make_record(files=[{"file": "a.txt", "patch": patch}]))
    expect(result["verdict"] == "RED-RESTORED", f"patch plant: {result['reason']}")
    fx.assert_restored()
    wrong_count = patch.replace("@@ -1,2 +1,2 @@", "@@ -1,3 +1,2 @@")
    expect_refused(lambda: load_records_from(fx, [make_record(files=[{"file": "a.txt", "patch": wrong_count}])]),
                   "line counts")
    other = patch.replace("+++ b/a.txt", "+++ b/other.txt")
    expect_refused(lambda: load_records_from(fx, [make_record(files=[{"file": "a.txt", "patch": other}])]),
                   "other.txt")


def load_records_from(fx, records):
    fx.write_records(records)
    return load_records(fx.plants)


@case
def anchor_that_occurs_zero_or_two_times_is_refused(fx):
    zero = make_record(files=[{"file": "b.txt", "edit": {"old": "zzz", "new": "y"}}])
    two = make_record(files=[{"file": "b.txt", "edit": {"old": "o", "new": "y"}}])
    expect_refused(lambda: run_record(fx.root, zero), "occurs 0 times")
    expect_refused(lambda: run_record(fx.root, two), "expected exactly one; when the file changed, re-pin the record")
    fx.assert_restored()


@case
def refused_record_runs_no_command(fx):
    good = make_record(command=[sys.executable, "-c", CHECK_FLAG], plant="ok")
    bad = make_record(plant="bad", files=[{"file": "b.txt", "edit": {"old": "zzz", "new": "y"}}])
    fx.write_records([good, bad])
    code, out = fx.cli("--all")
    expect(code == 2, f"exit {code}, wanted 2\n{out}")
    expect("REFUSED T1/bad" in out, f"no REFUSED line for T1/bad\n{out}")
    expect(not (fx.root / "ran.flag").exists(), "a command ran although a record was refused")


@case
def dirty_or_untracked_file_is_refused(fx):
    (fx.root / "a.txt").write_text("alpha GOOD\nbeta\nlocal edit\n")
    expect_refused(lambda: run_record(fx.root, make_record()), "not clean")
    (fx.root / "c.txt").write_text("GOOD\n")
    rec = make_record(files=[{"file": "c.txt", "edit": {"old": "GOOD", "new": "BAD"}}])
    expect_refused(lambda: run_record(fx.root, rec), "not tracked")


@case
def plant_that_stays_green_is_a_failure_with_row_and_plant(fx):
    rec = make_record(plant="stays", files=[{"file": "a.txt", "edit": {"old": "beta", "new": "gamma"}}])
    fx.write_records([rec])
    code, out = fx.cli("--all")
    expect(code == 1, f"exit {code}, wanted 1\n{out}")
    expect("PLANT-FAIL T1/stays: plant stayed green" in out, f"report line missing\n{out}")
    fx.assert_restored()


@case
def restore_that_leaves_bytes_changed_is_a_failure(fx):
    result = run_record(fx.root, make_record(), restore=lambda root, originals, use_git: None)
    expect(result["verdict"] == "PLANT-FAIL", "a restore that did nothing is not reported")
    expect("not byte-exact" in result["reason"], f"reason: {result['reason']}")
    fx.assert_restored()


@case
def files_with_no_record_selects_nothing(fx):
    fx.write_records([make_record()])
    records = load_records(fx.plants)
    expect(select_records(records, ["zzz.txt"]) == [], "a file with no record selected a plant")
    expect(select_records(records, []) == [], "an empty file list selected a plant")
    code, out = fx.cli("--files", "b.txt")
    expect(code == 0 and "no plant selected" in out, f"exit {code}\n{out}")
    expect(not (fx.root / "ran.flag").exists(), "a command ran")


@case
def files_selects_only_the_plants_of_the_changed_files(fx):
    a = make_record(row="T1", plant="a")
    b = make_record(row="T2", plant="b", files=[{"file": "b.txt", "edit": {"old": "two", "new": "2"}}],
                    expected_red="RED b lost two")
    both = make_record(row="T3", plant="c", files=[
        {"file": "a.txt", "edit": {"old": "GOOD", "new": "BAD"}},
        {"file": "b.txt", "edit": {"old": "two", "new": "2"}}])
    fx.write_records([a, b, both])
    records = load_records(fx.plants)
    names = lambda files: [label_of(r) for r in select_records(records, files)]
    expect(names(["a.txt"]) == ["T1/a", "T3/c"], f"a.txt selected {names(['a.txt'])}")
    expect(names(["./b.txt"]) == ["T2/b", "T3/c"], f"b.txt selected {names(['./b.txt'])}")
    expect(names(["a.txt", "b.txt"]) == ["T1/a", "T2/b", "T3/c"], "two files select all three")
    code, out = fx.cli("--list", "--files", "b.txt")
    expect(code == 0 and "T2/b" in out and "T1/a" not in out, f"--list --files b.txt\n{out}")


@case
def record_that_edits_two_files_applies_and_restores_both(fx):
    rec = make_record(files=[{"file": "a.txt", "edit": {"old": "GOOD", "new": "BAD"}},
                             {"file": "b.txt", "edit": {"old": "two", "new": "2"}}])
    result = run_record(fx.root, rec)
    expect(result["verdict"] == "RED-RESTORED", f"two-file plant: {result['reason']}")
    expect("RED a lost GOOD" in result["red_output"] and "RED b lost two" in result["red_output"],
           f"one of the two edits was not applied:\n{result['red_output']}")
    fx.assert_restored()
    expect(result["restored_sha256"] == result["baseline_sha256"], "the set hash differs after restore")


@case
def native_only_skips_headed_records(fx):
    native = make_record(plant="n")
    headed = make_record(plant="h", headed=True)
    fx.write_records([native, headed])
    records = load_records(fx.plants)
    expect([label_of(r) for r in select_records(records, None, True)] == ["T1/n"], "headed was not skipped")
    expect(len(select_records(records, None, False)) == 2, "without --native-only both must run")
    code, out = fx.cli("--list", "--all", "--native-only")
    expect("T1/h" not in out and "T1/n" in out, f"--list --native-only\n{out}")


@case
def exit_only_wrong_text_is_a_failure(fx):
    result = run_record(fx.root, make_record(expected_red="TEXT THAT THE RED DOES NOT PRINT"))
    expect(result["verdict"] == "PLANT-FAIL" and result["red_exit"] == 1, "right exit, wrong text passed")
    expect("is not printed" in result["reason"], f"reason: {result['reason']}")
    fx.assert_restored()


@case
def exit_other_than_the_record_is_a_failure(fx):
    result = run_record(fx.root, make_record(command=[sys.executable, "-c", CHECK_EXIT3]))
    expect(result["verdict"] == "PLANT-FAIL" and "red exit 3" in result["reason"], f"{result['reason']}")
    fx.assert_restored()


@case
def compile_error_is_not_a_red(fx):
    result = run_record(fx.root, make_record(command=[sys.executable, "-c", CHECK_COMPILE]))
    expect(result["verdict"] == "PLANT-FAIL" and "compile failure" in result["reason"], f"{result['reason']}")
    fx.assert_restored()


@case
def echo_line_is_not_a_red(fx):
    rec = make_record(command=[sys.executable, "-c", CHECK_ECHO], expected_red="x::GOOD")
    result = run_record(fx.root, rec)
    expect(result["verdict"] == "PLANT-FAIL" and "is not printed" in result["reason"], f"{result['reason']}")
    ok = make_record(command=[sys.executable, "-c", CHECK.replace("print(line)", "print('assertion: ' + line)")])
    expect(run_record(fx.root, ok)["verdict"] == "RED-RESTORED", "a real red line was not accepted")


@case
def baseline_that_is_not_green_is_a_failure(fx):
    result = run_record(fx.root, make_record(command=[sys.executable, "-c", CHECK_RED], expected_red="RED always"))
    expect(result["verdict"] == "PLANT-FAIL" and "baseline is not green" in result["reason"], f"{result['reason']}")
    expect(result["red_exit"] is None, "the plant was applied after a red baseline")
    fx.assert_restored()


@case
def env_of_the_record_reaches_the_command(fx):
    with_env = make_record(command=[sys.executable, "-c", CHECK_ENV], env={"MP1_X": "1"})
    result = run_record(fx.root, with_env)
    expect(result["verdict"] == "RED-RESTORED", f"with env: {result['reason']}")
    without = run_record(fx.root, make_record(command=[sys.executable, "-c", CHECK_ENV]))
    expect(without["verdict"] == "PLANT-FAIL" and "ENV MISSING" in without["reason"], "env was not needed")


@case
def restored_run_that_is_not_green_is_a_failure(fx):
    # Runs 1 (baseline) and 2 (plant, red text printed) are as wanted; run 3 (restored) stays red.
    result = run_record(fx.root, make_record(command=[sys.executable, "-c", CHECK_LINGER],
                                             expected_red="RED lingering"))
    expect(result["verdict"] == "PLANT-FAIL" and result["red_line"] == "RED lingering",
           f"the red run was not accepted: {result['reason']}")
    expect("restored is not green" in result["reason"], f"reason: {result['reason']}")
    fx.assert_restored()


@case
def command_that_changes_another_tracked_file_is_a_failure(fx):
    result = run_record(fx.root, make_record(command=[sys.executable, "-c", CHECK_TOUCH]))
    expect(result["verdict"] == "PLANT-FAIL" and "outside the plant" in result["reason"],
           f"{result['verdict']}: {result['reason']}")
    expect((fx.root / "a.txt").read_text() == Fixture.FILES["a.txt"], "the plant file is not restored")


@case
def timeout_is_a_failure_and_restores(fx):
    rec = make_record(command=[sys.executable, "-c", CHECK_SLEEP])
    started = time.monotonic()
    result = run_record(fx.root, rec, timeout=2)
    expect(result["verdict"] == "PLANT-FAIL" and "timed out" in result["reason"], f"{result['reason']}")
    expect(time.monotonic() - started < 30, "the timeout did not stop the command")
    fx.assert_restored()


def interrupted_run(fx, number):
    fx.write_records([make_record(command=[sys.executable, "-c", CHECK_SLEEP])])
    proc = subprocess.Popen([sys.executable, "-B", str(Path(__file__).resolve()), "--all", "--root",
                             str(fx.root), "--plants-dir", str(fx.plants), "--timeout", "120"],
                            stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    try:
        deadline = time.monotonic() + 30
        pid_file = fx.root / "child.pid"
        # The command writes its pid file only while the plant is in place.
        while not (pid_file.is_file() and pid_file.read_text()):
            expect(time.monotonic() < deadline and proc.poll() is None, "the plant was never applied")
            time.sleep(0.05)
        expect((fx.root / "a.txt").read_text() != Fixture.FILES["a.txt"], "the pid file came before the plant")
        proc.send_signal(number)
        code = proc.wait(timeout=30)
    finally:
        if proc.poll() is None:
            proc.kill()
    expect(code in (130, 143), f"exit {code} after signal {number}")
    fx.assert_restored()
    child = int((fx.root / "child.pid").read_text())
    deadline = time.monotonic() + 10
    while True:
        try:
            os.kill(child, 0)
        except ProcessLookupError:
            break
        expect(time.monotonic() < deadline, f"the command (pid {child}) still runs after the signal")
        time.sleep(0.05)


@case
def sigint_restores_the_source(fx):
    interrupted_run(fx, signal.SIGINT)


@case
def sigterm_restores_the_source(fx):
    interrupted_run(fx, signal.SIGTERM)


# --- review repairs (H1 to H3, M1 to M5): each case was red before its repair

CHECK_NOTE = CHECK.replace("sys.exit(1 if bad else 0)", "print('note: RED a lost GOOD')\nsys.exit(1 if bad else 0)")
CHECK_MULTI = ("import sys\nif 'GOOD' not in open('a.txt').read():\n"
               "    print('test x::GOOD ... FAILED\\nfailures:')\n    sys.exit(1)\n")
CHECK_FOUR = ("import sys\na = open('a.txt').read()\nb = open('b.txt').read()\nbad = []\n"
              "if 'GOOD' not in a: bad.append('RED GOOD')\nif 'beta' not in a: bad.append('RED beta')\n"
              "if 'one' not in b: bad.append('RED one')\nif 'two' not in b: bad.append('RED two')\n"
              "print(*bad, sep='\\n')\nsys.exit(1 if bad else 0)\n")


def check_cargo(count, name, red_count=None, red_name=None, extra=""):
    """A command that prints cargo test lines: the green run and the red run may differ."""
    return ("import sys\nbad = 'GOOD' not in open('a.txt').read()\n"
            f"print('running {red_count or count} tests' if bad else 'running {count} tests')\n"
            f"print('test {red_name or name} ... FAILED' if bad else 'test {name} ... ok')\n"
            f"{extra}if bad: print('RED a lost GOOD')\nsys.exit(1 if bad else 0)\n")


def expect_fail(result, text):
    expect(result["verdict"] == "PLANT-FAIL", f"verdict {result['verdict']}, wanted PLANT-FAIL")
    expect(text in result["reason"], f"reason lacks {text!r}: {result['reason']}")


@case
def h1_text_printed_by_the_green_run_is_no_red(fx):
    expect_fail(run_record(fx.root, make_record(command=[sys.executable, "-c", CHECK_NOTE])), "green run")
    fx.assert_restored()


@case
def h1_multi_line_text_of_echo_lines_is_no_red(fx):
    rec = make_record(command=[sys.executable, "-c", CHECK_MULTI],
                      expected_red="test x::GOOD ... FAILED\nfailures:")
    expect_fail(run_record(fx.root, rec), "is not printed")
    fx.assert_restored()


@case
def h1_shell_echo_of_the_command_is_no_red(fx):
    rec = make_record(command=["bash", "-c", "set -x; grep -q GOOD a.txt"], expected_red="grep -q GOOD a.txt")
    expect_fail(run_record(fx.root, rec), "green run")
    fx.assert_restored()


@case
def h1_text_printed_by_the_edit_itself_is_no_red(fx):
    rec = make_record(command=["bash", "-c", "cat a.txt; grep -q GOOD a.txt"], expected_red="alpha BAD")
    expect_fail(run_record(fx.root, rec), "edit")
    fx.assert_restored()


@case
def h2_moved_file_is_refused_loudly(fx):
    other = make_record(row="T2", plant="b", files=[{"file": "b.txt", "edit": {"old": "two", "new": "2"}}],
                        expected_red="RED b lost two")
    fx.write_records([make_record(), other])
    (fx.root / "sub").mkdir()
    fx.git("mv", "a.txt", "sub/a.txt")
    fx.commit("move")
    for args in (["--files", "sub/a.txt"], ["--files", "b.txt"], ["--all"]):
        code, out = fx.cli(*args)
        expect(code == 2, f"{args}: exit {code}, wanted 2\n{out}")
        expect("t.json[0]" in out and "re-pin" in out, f"{args}: no file, index and re-pin in\n{out}")


@case
def h2_unknown_path_is_refused_and_absolute_path_is_accepted(fx):
    fx.write_records([make_record()])
    for bad in ("a.tx", "/etc/passwd"):
        code, out = fx.cli("--list", "--files", bad)
        expect(code == 2, f"--files {bad}: exit {code}, wanted 2\n{out}")
    code, out = fx.cli("--list", "--files", str(fx.root / "a.txt"))
    expect(code == 0 and "T1/a" in out, f"absolute path: exit {code}\n{out}")


@case
def h3_sigkill_during_the_plant_run_is_recovered_by_the_next_run(fx):
    fx.write_records([make_record(command=[sys.executable, "-c", CHECK_SLEEP])])
    proc = subprocess.Popen([sys.executable, "-B", str(Path(__file__).resolve()), "--all", "--root",
                             str(fx.root), "--plants-dir", str(fx.plants), "--timeout", "120"],
                            stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    try:
        pid_file = fx.root / "child.pid"
        deadline = time.monotonic() + 30
        while not (pid_file.is_file() and pid_file.read_text()):
            expect(time.monotonic() < deadline and proc.poll() is None, "the plant was never applied")
            time.sleep(0.05)
        proc.kill()
        proc.wait()
    finally:
        if proc.poll() is None:
            proc.kill()
    os.kill(int(pid_file.read_text()), signal.SIGKILL)
    expect((fx.root / "a.txt").read_text() != Fixture.FILES["a.txt"], "SIGKILL left no plant to recover")
    expect(fx.journal().is_file(), "no journal after SIGKILL")
    fx.write_records([make_record()])
    code, out = fx.cli("--all")
    expect("a plant of T1/a was left in a.txt by an interrupted run; restored" in out, f"no recovery line\n{out}")
    expect(code == 0, f"exit {code} after the recovery\n{out}")
    fx.assert_restored()
    expect(not fx.journal().exists(), "the journal is still there")


@case
def h3_sigterm_inside_the_restore_does_not_leave_the_plant(fx):
    env, mark = fx.install_git_shim()
    fx.write_records([make_record()])
    proc = subprocess.Popen([sys.executable, "-B", str(Path(__file__).resolve()), "--all", "--root",
                             str(fx.root), "--plants-dir", str(fx.plants)], env=env,
                            stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
    deadline = time.monotonic() + 30
    while not mark.exists():
        expect(time.monotonic() < deadline and proc.poll() is None, "the restore never started")
        time.sleep(0.02)
    proc.send_signal(signal.SIGTERM)
    out, _ = proc.communicate(timeout=60)
    expect(proc.returncode == 143, f"exit {proc.returncode}, wanted 143\n{out}")
    fx.assert_restored()
    expect(not fx.journal().exists(), "the journal is still there")
    expect("restored" in out, f"no restored line\n{out}")


@case
def m1_headed_record_never_runs_in_the_serial_runner(fx):
    headed = make_record(plant="h", headed=True,
                         command=[sys.executable, "-c", "open('headed.ran', 'w').write('x')\n" + CHECK])
    fx.write_records([make_record(plant="n"), headed])
    code, out = fx.cli("--all")
    expect(code == 0, f"exit {code}\n{out}")
    expect(not (fx.root / "headed.ran").exists(), "the serial runner ran a headed record")
    expect("SKIPPED T1/h" in out, f"the skipped id is not listed\n{out}")


@case
def m2_expected_selected_count_and_name_are_checked(fx):
    def run(selected, **kwargs):
        cmd = [sys.executable, "-c", check_cargo(1, "m::t", **kwargs)]
        return run_record(fx.root, make_record(command=cmd, expected_selected=selected))
    expect_fail(run(2), "baseline: 2 selected")             # the green run selects 1 test
    expect_fail(run(1, red_count=2), "red: 1 selected")    # only the red run selects 2
    expect(run(1)["verdict"] == "RED-RESTORED", "the right count was refused")
    expect_fail(run("m::t", red_name="m::other"), "m::t did not report FAILED")
    expect_fail(run("m::t", extra="print('test m::zz ... FAILED') if bad else 0\n"), "another test failed")
    expect(run("m::t")["verdict"] == "RED-RESTORED", "the right test name was refused")


@case
def m3_untracked_file_written_by_the_command_is_a_failure(fx):
    cmd = [sys.executable, "-c", "open('stray.txt', 'a').write('x')\n" + CHECK]
    expect_fail(run_record(fx.root, make_record(command=cmd)), "stray.txt")
    expect((fx.root / "stray.txt").exists(), "the runner deleted a file that it does not own")


@case
def m4_second_runner_in_one_tree_is_refused(fx):
    slow = [sys.executable, "-c", "open('started.flag', 'w').write('x')\nimport time\ntime.sleep(6)\n"]
    fx.write_records([make_record(command=slow)])
    other = fx.base / "plants2"
    other.mkdir()
    (other / "t.json").write_text(json.dumps([make_record(row="T2", plant="b",
                                  files=[{"file": "b.txt", "edit": {"old": "two", "new": "2"}}],
                                  expected_red="RED b lost two")]))
    runner = [sys.executable, "-B", str(Path(__file__).resolve()), "--all", "--root", str(fx.root)]
    first = subprocess.Popen(runner + ["--plants-dir", str(fx.plants)], stdout=subprocess.DEVNULL,
                             stderr=subprocess.DEVNULL)
    try:
        deadline = time.monotonic() + 30
        while not (fx.root / "started.flag").exists():
            expect(time.monotonic() < deadline and first.poll() is None, "the first runner never started")
            time.sleep(0.05)
        second = subprocess.run(runner + ["--plants-dir", str(other)], text=True, stdout=subprocess.PIPE,
                                stderr=subprocess.STDOUT, timeout=60)
        expect(second.returncode == 2 and "another" in second.stdout,
               f"second runner: exit {second.returncode}\n{second.stdout}")
    finally:
        first.send_signal(signal.SIGINT)
        first.wait(timeout=60)
    fx.assert_restored()


@case
def m5_a_green_run_follows_each_restore(fx):
    cmd = [sys.executable, "-c", "open('runs.log', 'a').write('x')\n" + CHECK_FOUR]
    edits = [("p1", "a.txt", "GOOD", "RED GOOD"), ("p2", "a.txt", "beta", "RED beta"),
             ("p3", "b.txt", "one", "RED one"), ("p4", "b.txt", "two", "RED two")]
    fx.write_records([make_record(plant=p, command=cmd, expected_red=red,
                                  files=[{"file": f, "edit": {"old": old, "new": "ZZ"}}])
                      for p, f, old, red in edits])
    code, out = fx.cli("--all")
    expect(code == 0 and out.count("RED-RESTORED") == 4, f"exit {code}\n{out}")
    runs = len((fx.root / "runs.log").read_text())
    expect(runs == 9, f"{runs} runs of the command; a group of 4 needs 4 red + 5 green (the baseline and one "
                      "after each restore) = 9")
    fx.assert_restored()


@case
def m5_failed_green_after_a_restore_stops_the_group(fx):
    cmd = [sys.executable, "-c", CHECK_LINGER]
    fx.write_records([make_record(plant="p1", command=cmd, expected_red="RED lingering"),
                      make_record(plant="p2", command=cmd, expected_red="RED lingering",
                                  files=[{"file": "b.txt", "edit": {"old": "two", "new": "2"}}])])
    code, out = fx.cli("--all")
    expect(code == 1, f"exit {code}\n{out}")
    expect("PLANT-FAIL T1/p1: restored is not green" in out, f"p1\n{out}")
    expect("PLANT-FAIL T1/p2: not run" in out, f"p2 must say that the group stopped\n{out}")


# --- second review round: each case below was red at 38c6ea2 (except the one marked "guard")

CHECK_FLAKY = ("import os, sys\nn = len(open('runs.log').read()) if os.path.exists('runs.log') else 0\n"
               "open('runs.log', 'a').write('x')\n"
               "if 'GOOD' not in open('a.txt').read() or n == 2:\n    print('RED shared')\n    sys.exit(1)\n")
CHECK_RESIDUE = ("import os, sys\nm = 'ignored/marker'\nif os.path.exists(m):\n    os.remove(m)\n"
                 "    print('RED residue')\n    sys.exit(1)\nif 'GOOD' not in open('a.txt').read():\n"
                 "    os.makedirs('ignored', exist_ok=True)\n    open(m, 'w').write('x')\n"
                 "    print('RED residue')\n    sys.exit(1)\n")
CHECK_COUNTED = ("import os, sys\nn = len(open('runs.log').read()) if os.path.exists('runs.log') else 0\n"
                 "open('runs.log', 'a').write('x')\nprint('running 2 tests' if n == 2 else 'running 1 test')\n"
                 "bad = 'GOOD' not in open('a.txt').read()\nif bad: print('RED a lost GOOD')\nsys.exit(1 if bad else 0)\n")
CHECK_PRINTS_LINE = ("import sys\nl = open('a.txt').read().splitlines()[0]\nif 'GOOD' not in l:\n"
                     "    print(f\"assertion failed: line '{l}' must contain GOOD\")\n    sys.exit(1)\n")


def kill_in_plant(fx):
    """Start a run, kill it with SIGKILL while the plant is in a.txt, kill the command. Leave the
    plant and the journal in the tree. The next run uses the default record."""
    fx.write_records([make_record(command=[sys.executable, "-c", CHECK_SLEEP])])
    proc = subprocess.Popen([sys.executable, "-B", str(Path(__file__).resolve()), "--all", "--root",
                             str(fx.root), "--plants-dir", str(fx.plants), "--timeout", "120"],
                            stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    try:
        pid_file = fx.root / "child.pid"
        deadline = time.monotonic() + 30
        while not (pid_file.is_file() and pid_file.read_text()):
            expect(time.monotonic() < deadline and proc.poll() is None, "the plant was never applied")
            time.sleep(0.05)
        proc.kill()
        proc.wait()
    finally:
        if proc.poll() is None:
            proc.kill()
    os.kill(int(pid_file.read_text()), signal.SIGKILL)
    expect("BAD" in (fx.root / "a.txt").read_text(), "SIGKILL left no plant to recover")
    expect(fx.journal().is_file(), "no journal after SIGKILL")
    fx.write_records([make_record()])


@case
def j_hand_edit_after_a_kill_is_never_overwritten(fx):
    kill_in_plant(fx)
    fx.git("checkout", "--", "a.txt")
    (fx.root / "a.txt").write_text(Fixture.FILES["a.txt"] + "USER WORK\n")
    code, out = fx.cli("--all")
    expect(code == 2, f"exit {code}, wanted 2\n{out}")
    expect("USER WORK" in (fx.root / "a.txt").read_text(), "the runner overwrote the work of a person")
    expect("T1/a" in out and "a.txt" in out and "by hand" in out, f"no record, file and hint in\n{out}")
    expect(fx.journal().is_file(), "the journal was deleted")


@case
def j_moved_file_after_a_kill_is_refused(fx):
    kill_in_plant(fx)
    (fx.root / "sub").mkdir()
    fx.git("mv", "a.txt", "sub/a.txt")
    code, out = fx.cli("--all")
    expect(code == 2, f"exit {code}, wanted 2\n{out}")
    expect("T1/a" in out and "a.txt" in out, f"no record and file in\n{out}")
    expect("BAD" in (fx.root / "sub" / "a.txt").read_text(), "the runner changed the moved file")
    expect(fx.journal().is_file(), "the journal was deleted")


@case
def j_committed_plant_is_named(fx):
    kill_in_plant(fx)
    fx.git("-c", "user.name=t", "-c", "user.email=t@example.com", "-c", "commit.gpgsign=false",
           "commit", "-qam", "oops")
    code, out = fx.cli("--all")
    expect(code == 2, f"exit {code}, wanted 2\n{out}")
    expect("holds the plant" in out and "T1/a" in out, f"the commit of the plant is not named\n{out}")
    expect(fx.journal().is_file(), "the journal was deleted")


@case
def j_damaged_journal_is_refused_and_kept(fx):
    for text in ("[{}]", "{}", "{", '[{"record": 1, "file": "a.txt", "sha256": "x"}]'):
        fx.journal().write_text(text)
        code, out = fx.cli("--all")
        expect(code == 2, f"journal {text!r}: exit {code}, wanted 2\n{out}")
        expect("Traceback" not in out, f"journal {text!r}: a traceback\n{out}")
        expect(fx.journal().is_file() and fx.journal().read_text() == text, f"journal {text!r} was changed")
    fx.journal().unlink()


@case
def j_head_without_the_original_is_refused(fx):
    kill_in_plant(fx)
    planted = (fx.root / "a.txt").read_text()
    (fx.root / "a.txt").write_text("alpha OTHER\nbeta\n")
    fx.git("-c", "user.name=t", "-c", "user.email=t@example.com", "-c", "commit.gpgsign=false",
           "commit", "-qam", "other text")
    (fx.root / "a.txt").write_text(planted)    # the bytes are the planted bytes, HEAD has other bytes
    code, out = fx.cli("--all")
    expect(code == 2 and "no longer has the original" in out, f"exit {code}\n{out}")
    expect((fx.root / "a.txt").read_text() == planted, "the runner changed the file")


@case
def j_guard_file_restored_by_hand_drops_the_entry(fx):
    kill_in_plant(fx)
    fx.git("checkout", "--", "a.txt")
    code, out = fx.cli("--all")
    expect(code == 0, f"exit {code}\n{out}")
    fx.assert_restored()
    expect(not fx.journal().exists(), "the journal is still there")


@case
def m5_flaky_green_between_plants_is_not_a_red(fx):
    cmd = [sys.executable, "-c", CHECK_FLAKY]
    fx.write_records([make_record(plant="p1", command=cmd, expected_red="RED shared"),
                      make_record(plant="p2", command=cmd, expected_red="RED shared",
                                  files=[{"file": "b.txt", "edit": {"old": "one", "new": "ZZ"}}])])
    code, out = fx.cli("--all")
    expect(code == 1 and "RED-RESTORED T1/p2" not in out and "PLANT-FAIL T1/p2" in out,
           f"a plant with no effect was red (exit {code})\n{out}")
    fx.assert_restored()


@case
def m5_residue_in_an_ignored_folder_is_not_a_red(fx):
    cmd = [sys.executable, "-c", CHECK_RESIDUE]
    fx.write_records([make_record(plant="p1", command=cmd, expected_red="RED residue"),
                      make_record(plant="p2", command=cmd, expected_red="RED residue",
                                  files=[{"file": "b.txt", "edit": {"old": "one", "new": "ZZ"}}])])
    code, out = fx.cli("--all")
    expect(code == 1 and "RED-RESTORED T1/p2" not in out and "PLANT-FAIL T1/p2" in out,
           f"a plant with no effect was red (exit {code})\n{out}")
    fx.assert_restored()


@case
def m2_cargo_test_record_needs_expected_selected(fx):
    fx.write_records([make_record(command=["cargo", "test", "-p", "x"])])
    expect_refused(lambda: load_records(fx.plants), "expected_selected")
    fx.write_records([make_record(command=["cargo", "test", "-p", "x"], expected_selected="m::t")])
    expect(len(load_records(fx.plants)) == 1, "a cargo test record with expected_selected was refused")
    fx.write_records([make_record(command=["bash", "-c", "cd x && cargo test --lib"])])
    expect_refused(lambda: load_records(fx.plants), "expected_selected")


@case
def m2_selection_guard_is_part_of_the_group_key(fx):
    one, two = make_record(expected_selected=1), make_record(plant="b", expected_selected=2)
    expect(group_key(one) != group_key(two), "plants with another selection guard share a group")
    expect(group_key(one) == group_key(make_record(plant="c", expected_selected=1)), "equal plants are split")


@case
def m2_every_green_run_checks_expected_selected(fx):
    rec = make_record(command=[sys.executable, "-c", CHECK_COUNTED], expected_selected=1)
    expect_fail(run_record(fx.root, rec), "restored: 1 selected")   # run 3, the green after the restore
    fx.assert_restored()


@case
def l_true_red_that_prints_the_edited_line_is_accepted(fx):
    rec = make_record(command=[sys.executable, "-c", CHECK_PRINTS_LINE], expected_red="assertion failed: line")
    result = run_record(fx.root, rec)
    expect(result["verdict"] == "RED-RESTORED", f"a true red was refused: {result['reason']}")


@case
def l_deleted_path_is_accepted_in_files(fx):
    fx.write_records([make_record(row="T2", plant="b", files=[{"file": "b.txt", "edit": {"old": "two", "new": "2"}}],
                                  expected_red="RED b lost two")])
    fx.git("rm", "-q", "a.txt")
    code, out = fx.cli("--list", "--files", "a.txt")            # deleted in the index
    expect(code == 0, f"index delete: exit {code}\n{out}")
    fx.commit("delete a")
    code, out = fx.cli("--list", "--files", "a.txt", "b.txt")   # deleted in a commit
    expect(code == 0 and "T2/b" in out, f"committed delete: exit {code}\n{out}")
    code, out = fx.cli("--list", "--files", "never-existed.txt")
    expect(code == 2, f"a path that never existed: exit {code}, wanted 2\n{out}")


def tree_snapshot(fx):
    """Every file under the repository folder (the git folder excluded) with its bytes, the names
    that sit in the git folder that the runner could write (journal, lock), and the sha256 of
    .git/index (`git status` refreshes the stat cache there unless it runs with no optional locks)."""
    files = {str(p.relative_to(fx.root)): p.read_bytes()
             for p in sorted(fx.root.rglob("*")) if p.is_file() and ".git" not in p.relative_to(fx.root).parts}
    git_dir_path = Path(fx.git("rev-parse", "--absolute-git-dir").strip())
    index = hashlib.sha256((git_dir_path / "index").read_bytes()).hexdigest()
    return files, sorted(n for n in (JOURNAL_NAME, LOCK_NAME) if (git_dir_path / n).exists()), index


def stale_a(**over):
    """A record whose anchor does not occur in a.txt."""
    return make_record(row="T9", plant="stale", command=[sys.executable, "-c", CHECK_FLAG],
                       files=[{"file": "a.txt", "edit": {"old": "MISSING", "new": "x"}}], **over)


@case
def q3_check_clean_set_exits_0_and_runs_nothing(fx):
    fx.write_records([make_record(command=[sys.executable, "-c", CHECK_FLAG]),
                      make_record(plant="b", command=[sys.executable, "-c", CHECK_FLAG],
                                  files=[{"file": "b.txt", "edit": {"old": "two", "new": "2"}}])])
    before = tree_snapshot(fx)
    code, out = fx.cli("--check", "--all")
    expect(code == 0, f"clean set: exit {code}, wanted 0\n{out}")
    expect("2 record(s) apply, 0 stale" in out, f"no summary line\n{out}")
    expect(not (fx.root / "ran.flag").exists(), "--check ran a command")
    expect(tree_snapshot(fx) == before, "--check changed a file")


@case
def q3_check_stale_anchor_exits_2_and_names_the_record(fx):
    fx.write_records([make_record(), stale_a()])
    code, out = fx.cli("--check", "--all")
    expect(code == 2, f"stale anchor: exit {code}, wanted 2\n{out}")
    stale = [line for line in out.splitlines() if line.startswith("STALE ")]
    expect(len(stale) == 1, f"wanted one STALE line\n{out}")
    expect("1 record(s) apply, 1 stale" in out, f"the count line is wrong\n{out}")
    for part in ("T9/stale", "t.json[1]", "a.txt", "occurs 0 times"):
        expect(part in stale[0], f"STALE line lacks {part!r}: {stale[0]}")
    expect("T1/a" not in out.replace("STALE", ""), f"the clean record is named\n{out}")


@case
def q3_check_anchor_that_occurs_twice_exits_2(fx):
    fx.write_records([make_record(plant="two", files=[{"file": "b.txt", "edit": {"old": "o", "new": "y"}}])])
    code, out = fx.cli("--check", "--all")
    expect(code == 2, f"two occurrences: exit {code}, wanted 2\n{out}")
    expect("STALE T1/two" in out and "occurs 2 times" in out, f"STALE line lacks the reason\n{out}")


@case
def q3_check_stale_patch_exits_2(fx):
    patch = "--- a/a.txt\n+++ b/a.txt\n@@ -1,2 +1,2 @@\n alpha MISSING\n-beta\n+gamma\n"
    fx.write_records([make_record(plant="p", files=[{"file": "a.txt", "patch": patch}])])
    code, out = fx.cli("--check", "--all")
    expect(code == 2, f"stale patch: exit {code}, wanted 2\n{out}")
    expect("STALE T1/p" in out and "occurs 0 times" in out, f"STALE line lacks the reason\n{out}")
    good = "--- a/a.txt\n+++ b/a.txt\n@@ -1,2 +1,2 @@\n-alpha GOOD\n+alpha BAD\n beta\n"
    fx.write_records([make_record(plant="p", files=[{"file": "a.txt", "patch": good}])])
    code, out = fx.cli("--check", "--all")
    expect(code == 0, f"a patch that applies: exit {code}, wanted 0\n{out}")


@case
def q3_check_leaves_every_file_byte_equal_also_after_a_failure(fx):
    fx.write_records([make_record(command=[sys.executable, "-c", CHECK_FLAG]), stale_a()])
    before = tree_snapshot(fx)
    code, out = fx.cli("--check", "--all")
    expect(code == 2 and "STALE T9/stale" in out, f"exit {code}, wanted 2 with a STALE line\n{out}")
    expect(tree_snapshot(fx) == before, "--check changed a file or wrote a journal or a lock after a failure")
    expect(not (fx.root / "ran.flag").exists(), "--check ran a command")
    fx.assert_restored()


@case
def q3_check_and_run_mode_refuse_the_same_records(fx):
    """The two modes share one code path: the stale set of --check is the refused set of --all."""
    patch = "--- a/a.txt\n+++ b/a.txt\n@@ -1,2 +1,1 @@\n alpha MISSING\n-beta\n"
    (fx.root / "c.txt").write_text("GOOD\n")                    # untracked: the run mode refuses it
    fx.write_records([
        make_record(plant="ok"), stale_a(),
        make_record(plant="two", files=[{"file": "b.txt", "edit": {"old": "o", "new": "y"}}]),
        make_record(plant="patch", files=[{"file": "a.txt", "patch": patch}]),
        make_record(plant="untracked", files=[{"file": "c.txt", "edit": {"old": "GOOD", "new": "BAD"}}])])
    code_check, out_check = fx.cli("--check", "--all")
    code_run, out_run = fx.cli("--all")
    expect(code_check == 2 and code_run == 2, f"exit codes {code_check} and {code_run}, wanted 2 and 2")
    stale = [line[len("STALE "):] for line in out_check.splitlines() if line.startswith("STALE ")]
    refused = [line[len("REFUSED "):] for line in out_run.splitlines() if line.startswith("REFUSED ")]
    expect(len(stale) == 4 and stale == refused, f"STALE:\n{stale}\nREFUSED:\n{refused}")
    expect(not any("T1/ok" in line for line in stale), "the clean record is stale")


@case
def q3_check_missing_file_is_stale_with_the_text_of_the_run_mode(fx):
    gone = make_record(plant="gone", files=[{"file": "gone.txt", "edit": {"old": "a", "new": "b"}}])
    fx.write_records([make_record(), gone])
    code_check, out_check = fx.cli("--check", "--all")
    code_run, out_run = fx.cli("--all")
    expect(code_check == 2 and code_run == 2, f"exit codes {code_check} and {code_run}, wanted 2 and 2")
    stale = [line[len("STALE "):] for line in out_check.splitlines() if line.startswith("STALE ")]
    refused = [line[len("REFUSED "):] for line in out_run.splitlines() if line.startswith("REFUSED ")]
    expect(len(stale) == 1 and stale == refused and "gone.txt is missing" in stale[0],
           f"STALE:\n{stale}\nREFUSED:\n{refused}")
    # A record that --files does not select still counts: the run mode refuses it in every mode too.
    code, out = fx.cli("--check", "--files", "a.txt")
    expect(code == 2 and "STALE T1/gone" in out, f"--files a.txt must report the moved file\n{out}")


@case
def q3_check_with_files_selects_like_the_run_mode(fx):
    fx.write_records([make_record(), make_record(plant="b", files=[
        {"file": "b.txt", "edit": {"old": "zzz", "new": "y"}}])])
    code, out = fx.cli("--check", "--files", "a.txt")
    expect(code == 0 and "1 record(s) apply, 0 stale" in out, f"--files a.txt: exit {code}\n{out}")
    code, out = fx.cli("--check", "--files", "b.txt")
    expect(code == 2 and "STALE T1/b" in out and "0 record(s) apply, 1 stale" in out,
           f"--files b.txt: exit {code}\n{out}")
    code, out = fx.cli("--check", "--files", "zzz.txt")
    expect(code == 2, f"a mistyped path: exit {code}, wanted 2\n{out}")


@case
def q3_check_does_not_mix_with_list_and_list_stays_as_it_is(fx):
    fx.write_records([stale_a()])
    code, out = fx.cli("--check", "--list", "--all")
    expect(code == 2 and "--check cannot be used with --list" in out,
           f"--check with --list: exit {code}, wanted 2\n{out}")
    code, out = fx.cli("--list", "--all")
    expect(code == 0 and "T9/stale" in out and "STALE" not in out, f"--list changed: exit {code}\n{out}")


@case
def q3_check_names_a_leftover_journal_and_does_not_recover(fx):
    fx.write_records([make_record(command=[sys.executable, "-c", CHECK_FLAG])])
    (fx.root / "a.txt").write_text("alpha BAD\nbeta\n")        # looks like a plant that a kill left
    journal = fx.journal()
    journal.write_text("[]")
    before = tree_snapshot(fx)
    code, out = fx.cli("--check", "--all")
    expect(code == 2 and "STALE T1/a" in out, f"exit {code}\n{out}")
    notes = [line for line in out.splitlines() if line.startswith("NOTE ")]
    expect(len(notes) == 1 and JOURNAL_NAME in notes[0] and "run mode" in notes[0], f"journal note: {notes}\n{out}")
    expect(tree_snapshot(fx) == before, "--check recovered or changed a file")
    expect((fx.root / "a.txt").read_text() == "alpha BAD\nbeta\n", "--check restored a.txt")
    journal.unlink()
    (fx.root / "a.txt").write_text(Fixture.FILES["a.txt"])
    code, out = fx.cli("--check", "--all")
    expect(code == 0 and "NOTE" not in out, f"no journal: exit {code}\n{out}")


@case
def q3_check_names_a_run_in_flight_and_not_a_free_lock_file(fx):
    fx.write_records([make_record(plant="g", files=[{"file": "a.txt", "edit": {"old": "beta", "new": "gamma"}}])])
    fx.cli("--all")                                           # the run mode leaves a free lock file
    lock = Path(fx.git("rev-parse", "--absolute-git-dir").strip()) / LOCK_NAME
    expect(lock.exists(), "the run mode left no lock file: the case tests nothing")
    code, out = fx.cli("--check", "--all")
    expect(code == 0 and "NOTE" not in out, f"free lock file: exit {code}\n{out}")
    handle = os.open(lock, os.O_RDWR)
    try:
        fcntl.flock(handle, fcntl.LOCK_EX | fcntl.LOCK_NB)
        (fx.root / "a.txt").write_text("alpha GOOD\nbeta\nplanted by the other run\n")
        code, out = fx.cli("--check", "--all")
    finally:
        os.close(handle)
    notes = [line for line in out.splitlines() if line.startswith("NOTE ")]
    expect(code == 2 and len(notes) == 1 and "run in flight" in notes[0] and "wait" in notes[0].lower(),
           f"lock note: exit {code}, {notes}\n{out}")


@case
def q3_net_zero_plant_is_refused_in_both_modes(fx):
    net_zero = make_record(plant="zero", command=[sys.executable, "-c", CHECK_FLAG], files=[
        {"file": "a.txt", "edit": [{"old": "GOOD", "new": "GOOD2"}, {"old": "GOOD2", "new": "GOOD"}]}])
    fx.write_records([net_zero])
    code_check, out_check = fx.cli("--check", "--all")
    code_run, out_run = fx.cli("--all")
    expect(code_check == 2 and code_run == 2, f"exit codes {code_check} and {code_run}, wanted 2 and 2\n{out_run}")
    stale = [l[len("STALE "):] for l in out_check.splitlines() if l.startswith("STALE ")]
    refused = [l[len("REFUSED "):] for l in out_run.splitlines() if l.startswith("REFUSED ")]
    expect(len(stale) == 1 and stale == refused and "changes nothing" in stale[0], f"{stale}\n{refused}")
    expect(not (fx.root / "ran.flag").exists(), "a command ran")
    # A record that changes one file and leaves the other as it was still applies.
    two = make_record(files=[{"file": "a.txt", "edit": {"old": "GOOD", "new": "BAD"}},
                             {"file": "b.txt", "edit": [{"old": "two", "new": "2"}, {"old": "2", "new": "two"}]}])
    fx.write_records([two])
    code, out = fx.cli("--check", "--all")
    expect(code == 0, f"one file changed, one not: exit {code}\n{out}")


@case
def q3_check_does_not_write_the_git_index(fx):
    fx.write_records([make_record(command=[sys.executable, "-c", CHECK_FLAG])])
    for name in Fixture.FILES:
        os.utime(fx.root / name, (1_000_000_000, 1_000_000_000))   # same bytes, other stat: status wants to refresh
    index = Path(fx.git("rev-parse", "--absolute-git-dir").strip()) / "index"
    before = hashlib.sha256(index.read_bytes()).hexdigest()
    code, out = fx.cli("--check", "--all")
    expect(code == 0, f"exit {code}\n{out}")
    expect(hashlib.sha256(index.read_bytes()).hexdigest() == before, ".git/index changed during --check")


def self_test():
    failed = 0
    for name, function in CASES:
        base = tempfile.mkdtemp(prefix="mp1-plants-")
        try:
            function(Fixture(base))
            print(f"SELFTEST ok   {name}", flush=True)
        except Exception as exc:  # a todo body (NotImplementedError) is a red, not a crash
            failed += 1
            print(f"SELFTEST FAIL {name}: {type(exc).__name__}: {exc}", flush=True)
        finally:
            shutil.rmtree(base, ignore_errors=True)
    print(f"SELFTEST {len(CASES) - failed} of {len(CASES)} cases ok", flush=True)
    return 1 if failed else 0


if __name__ == "__main__":
    sys.dont_write_bytecode = True
    raise SystemExit(main())
