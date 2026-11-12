#!/usr/bin/env python3
"""Check a cold-play evidence folder (task Q-COLD; docs/qa/cold-play.md).

    python3 -I -B scripts/qa/cold_play_check.py <session folder> [--receipt lock-receipt.json]
                                                  [--no-lock-check] [--json]

The checker reads the files that tests/browser/cold-session.js wrote. It starts no browser. It
verifies what a machine can verify of the person-readiness protocol (input-ceo-second-play-test.md
section 5) and prints the checks that stay manual. It never prints PERSON-READY: that verdict needs
the manual checks and the reference red (C5).

Machine checks:
  E1  events.jsonl is complete and in order; inputs.jsonl equals the input events
  E2  every screenshot file exists and has the recorded sha256; every call has its two screenshots
      at +200 ms and +1000 ms, taken in time (a late screenshot can miss the effect of a click)
  L1  the session limit: at most 1,500 s (the lock wrapper holds 1,800 s), ended cleanly, on time
  L2  the lock marker of the headed wrapper is in session.json (--no-lock-check skips it); with
      --receipt: the wrapper receipt says ok and its hold is inside its limit
  C2a each key input names a key (the keys are listed for the manual match of C2)
  C3  the statement: it exists; it was stored before the sound records were released; it was stored
      60 s or more after the first click; no input came between the 60 s mark and the statement; it
      has the three parts, none of them "do not know"; each part names a screenshot file of the
      session that was taken before the statement
  S1  the sound records are complete and final, and the page debug bridge answered
  F5a no sound id more than 3 times in 60 s
  F5b no alert sound before the first input

Exit: 0 every machine check passes; 1 a machine check fails; 2 the folder cannot be read.
Python 3 standard library only.
"""

import argparse
import hashlib
import json
import re
import sys
from pathlib import Path

STATEMENT_KEYS = ("GOAL", "GOAL_SHOT", "FIRST_ACTION", "FIRST_ACTION_SHOT", "WORKED", "WORKED_SHOT")
SHOT_NAME = re.compile(r"^(?:0000-start|\d{4}-[a-z]+-p(?:200|1000))\.png$")
KEY_NAME = re.compile(r"^[A-Za-z0-9]{1,24}$")
# "do not know" anywhere in a part is a fail (protocol check C3); an empty-answer word fails when it starts the part.
NO_ANSWER = re.compile(r"\b(?:do not know|don['\u2019]?t know|no idea)\b|^(?:unknown|none|n/a|not sure|unsure|unclear|nothing)\b", re.IGNORECASE)
MAX_LIMIT_MS = 1500 * 1000
LIMIT_GRACE_MS = 10000
WRAPPER_HOLD_S = 1800
# The slack for a screenshot: it is taken after the wait; late is the fault.
SHOT_LATE_MS = {200: 500, 1000: 700}
# The times are whole milliseconds, so a screenshot can start 1 ms before its mark; more than that is early.
EARLY_JITTER_MS = 3

MANUAL = (
    ("C0", "the prompt holds only the start command or URL and the one question (the reviewer greps the prompt)"),
    ("C1", "the agent read no repository file, memory note or earlier report before the statement (the transcript)"),
    ("C2", "each click, drag and wheel point and each key above was seen in an earlier screenshot (match inputs.jsonl to shots/)"),
    ("C3m", "the goal, the first action and the proof are true to the screenshots that they name (read the three files)"),
    ("C4", "none of F1 to F4 and F6 to F10 is seen (read the screenshots; F6 is the alert LINE, F5b above is the alert SOUND)"),
    ("C5", "the reference red: the same tool on the 9b3a7bd forge build shows F2 and F3 at the first building click (run once for each tool version)"),
)


class Unusable(Exception):
    pass


def load_json(path):
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (OSError, ValueError) as error:
        raise Unusable(f"cannot read {path.name}: {error}")


def load_events(folder):
    path = folder / "events.jsonl"
    try:
        lines = path.read_text(encoding="utf-8").splitlines()
    except OSError as error:
        raise Unusable(f"cannot read events.jsonl: {error}")
    events = []
    for number, line in enumerate(lines, 1):
        try:
            events.append(json.loads(line))
        except ValueError:
            raise Unusable(f"events.jsonl line {number} is not JSON")
    return events


class Report:
    def __init__(self):
        self.rows = []

    def add(self, check, ok, text):
        self.rows.append({"check": check, "status": "PASS" if ok else "FAIL", "text": text})

    def info(self, check, text):
        self.rows.append({"check": check, "status": "INFO", "text": text})

    @property
    def failed(self):
        return [row["check"] for row in self.rows if row["status"] == "FAIL"]


def parse_statement(text):
    fields, problems = {}, []
    for number, line in enumerate(text.splitlines(), 1):
        if not line.strip():
            continue
        match = re.match(r"^([A-Z_]+):[ \t]*(.*)$", line)
        if not match:
            problems.append(f"line {number} is not KEY: value")
        elif match.group(1) not in STATEMENT_KEYS:
            problems.append(f"line {number}: unknown key {match.group(1)}")
        elif match.group(1) in fields:
            problems.append(f"line {number}: {match.group(1)} twice")
        else:
            fields[match.group(1)] = match.group(2).strip()
    for key in STATEMENT_KEYS:
        if key not in fields:
            problems.append(f"{key} is missing")
        elif not fields[key]:
            problems.append(f"{key} is empty")
    return fields, problems


def check(folder, receipt=None, lock_check=True):
    session = load_json(folder / "session.json")
    events = load_events(folder)
    report = Report()
    by_kind = {}
    for event in events:
        by_kind.setdefault(event.get("kind"), []).append(event)
    shots_dir = folder / "shots"
    end_events = by_kind.get("end", [])
    end = None
    if (folder / "end.json").exists():
        end = load_json(folder / "end.json")

    # E1: order and completeness of the event log.
    seqs = [event.get("seq") for event in events]
    times = [event.get("t_ms") for event in events]
    ordered = seqs == list(range(1, len(events) + 1)) and all(a <= b for a, b in zip(times, times[1:]))
    report.add("E1", ordered and bool(events) and events[0].get("kind") == "ready",
               f"events.jsonl: {len(events)} events, seq 1..n, time never goes back, starts with ready")
    inputs_path = folder / "inputs.jsonl"
    try:
        inputs_lines = [json.loads(line) for line in inputs_path.read_text(encoding="utf-8").splitlines()]
    except (OSError, ValueError):
        inputs_lines = None
    input_events = by_kind.get("input", [])
    report.add("E1b", inputs_lines == input_events, f"inputs.jsonl equals the {len(input_events)} input events")

    # E2: screenshots.
    shot_events = {event.get("file"): event for event in by_kind.get("shot", [])}
    bad_files = []
    for name, event in shot_events.items():
        path = shots_dir / str(name)
        if not SHOT_NAME.match(str(name)) or not path.is_file():
            bad_files.append(f"{name}: missing")
        elif hashlib.sha256(path.read_bytes()).hexdigest() != event.get("sha256") or path.stat().st_size != event.get("bytes"):
            bad_files.append(f"{name}: bytes differ from the record")
    report.add("E2", bool(shot_events) and not bad_files,
               f"{len(shot_events)} screenshots, each file present with the recorded sha256" + (f"; PROBLEMS: {bad_files[:4]}" if bad_files else ""))
    timing = []
    for event in input_events:
        for offset in (200, 1000):
            shot = shot_events.get(f"{event['n']:04d}-{event['cmd']}-p{offset}.png")
            if shot is None:
                timing.append(f"call {event['n']}: no +{offset} ms screenshot")
                continue
            late = shot["t_start_ms"] - event["t_end_ms"] - offset
            if late < -EARLY_JITTER_MS or late > SHOT_LATE_MS[offset]:
                timing.append(f"call {event['n']}: +{offset} ms screenshot started {late} ms off")
    report.add("E2b", not timing, f"each of the {len(input_events)} calls has its +200 ms and +1000 ms screenshots, in time"
               + (f"; PROBLEMS: {timing[:4]}" if timing else ""))

    # L1 and L2: the limits.
    limit_ms = session.get("limit_ms", 0)
    end_t = end.get("t_ms") if end else None
    clean = (end is not None and bool(end_events) and end.get("reason") in ("limit", "end")
             and events and events[-1].get("kind") == "end")
    report.add("L1", 0 < limit_ms <= MAX_LIMIT_MS and clean and end_t is not None and end_t <= limit_ms + LIMIT_GRACE_MS,
               f"limit {limit_ms / 1000:.0f} s (at most 1500 s); ended {end.get('reason') if end else 'NOT ENDED'} at "
               f"{end_t / 1000 if end_t is not None else float('nan'):.1f} s; the log ends with the end event")
    if lock_check:
        report.add("L2", bool(session.get("lock_marker")), f"lock marker in session.json: {session.get('lock_marker')}")
    else:
        report.info("L2", "lock marker not checked (--no-lock-check)")
    if receipt is not None:
        data = load_json(Path(receipt))
        report.add("L2b", data.get("status") == "ok" and data.get("hold_s", 1e9) <= data.get("limit_s", 0) and data.get("hold_s", 1e9) <= WRAPPER_HOLD_S,
                   f"wrapper receipt: status {data.get('status')}, hold {data.get('hold_s')} s of {data.get('limit_s')} s")
    else:
        report.info("L2b", "wrapper receipt not given (--receipt): hold time not checked")

    # C2a: keys.
    keys = [event for event in input_events if event["cmd"] == "key"]
    shots_by_time = sorted(shot_events.values(), key=lambda shot: shot["t_done_ms"])
    report.add("C2a", all(KEY_NAME.match(str(event["args"].get("code", ""))) for event in keys),
               f"{len(keys)} key inputs name a key; {sum(1 for e in input_events if e['cmd'] in ('click', 'drag', 'move', 'wheel'))} point inputs")
    for event in keys:
        before = [shot["file"] for shot in shots_by_time if shot["t_done_ms"] <= event["t_start_ms"]]
        report.info("C2a", f"MATCH BY HAND: key {event['args'].get('code')} at {event['t_start_ms'] / 1000:.1f} s; the last screenshot before it: {before[-1] if before else 'none'}")

    # C3: the statement.
    first_click = (by_kind.get("first_click") or [None])[0]
    stored = (by_kind.get("statement_stored") or [None])[0]
    released = by_kind.get("sounds_released", [])
    statement_path = folder / "statement.txt"
    report.add("C3a", statement_path.is_file() and stored is not None, "statement.txt exists and the log has statement_stored")
    if stored is not None and statement_path.is_file():
        text = statement_path.read_text(encoding="utf-8")
        report.add("C3b", hashlib.sha256(text.encode("utf-8")).hexdigest() == stored.get("sha256"), "statement.txt has the recorded sha256")
        first_release = released[0] if released else None
        report.add("C3c", first_release is not None and stored["seq"] < first_release["seq"],
                   f"the statement was stored (seq {stored['seq']}) before the sound records were released "
                   f"(seq {first_release['seq'] if first_release else 'none'})")
        if first_click is not None:
            due = first_click["t_down_ms"] + 60000
            report.add("C3d", stored["t_ms"] >= due, f"the statement came {(stored['t_ms'] - first_click['t_down_ms']) / 1000:.1f} s after the first click (60 s or more)")
            between = [e["n"] for e in input_events if e["counts_as_input"] and due < e["t_start_ms"] < stored["t_ms"]]
            report.add("C3e", not between, f"no input between the 60 s mark and the statement" + (f"; inputs {between}" if between else ""))
        else:
            report.add("C3d", False, "no first click in the log: the 60 s clock never started")
        fields, problems = parse_statement(text)
        report.add("C3f", not problems, "the statement has the six parts" + (f"; PROBLEMS: {problems}" if problems else ""))
        no_answer = [key for key in ("GOAL", "FIRST_ACTION", "WORKED") if NO_ANSWER.search(fields.get(key, ""))]
        report.add("C3g", not no_answer, "no part of the statement is \"do not know\"" + (f"; {no_answer}" if no_answer else ""))
        shot_problems = []
        for key in (k for k in STATEMENT_KEYS if k.endswith("_SHOT") and k in fields):
            name = Path(fields[key]).name
            shot = shot_events.get(name)
            if shot is None or not (shots_dir / name).is_file():
                shot_problems.append(f"{key}: {fields[key]} is not a screenshot of the session")
            elif shot["t_done_ms"] > stored["t_ms"]:
                shot_problems.append(f"{key}: {name} was taken after the statement")
        report.add("C3h", not shot_problems, "each part names a screenshot file of the session, taken before the statement" + (f"; PROBLEMS: {shot_problems}" if shot_problems else ""))
    else:
        report.add("C3c", False, "no statement: the sound records opened with no statement before them")

    # S1, F5: the sound records.
    records_path = folder / "sound-records.json"
    sound = load_json(records_path) if records_path.is_file() else None
    if sound is None:
        report.add("S1", False, "sound-records.json is missing")
        return report
    sampling = sound.get("sampling", {})
    expected = (end_t or 0) / sampling.get("interval_ms", 500) * 0.5
    samples = sampling.get("samples", 0)
    report.add("S1", bool(sound.get("final")) and samples >= max(1, expected) and sampling.get("bridge_missing", 0) == 0
               and sampling.get("failures", 0) <= 0.2 * max(1, samples),
               f"sound records final: {sound.get('final')}; {samples} samples (at least {max(1, expected):.0f}), "
               f"{sampling.get('failures')} failed, bridge missing {sampling.get('bridge_missing')}")
    records = sound.get("records", [])
    by_id = {}
    for record in records:
        by_id.setdefault(record.get("sound_id"), []).append(record["first_seen_ms"])
    crowded = []
    for sound_id, seen in by_id.items():
        seen.sort()
        for i in range(len(seen) - 3):
            if seen[i + 3] - seen[i] <= 60000:
                crowded.append(f"{sound_id}: 4 times from {seen[i] / 1000:.1f} s to {seen[i + 3] / 1000:.1f} s")
                break
    report.add("F5a", not crowded, f"no sound id more than 3 times in 60 s ({len(records)} sounds, {len(by_id)} ids)" + (f"; {crowded}" if crowded else ""))
    first_input = min((e["t_start_ms"] for e in input_events if e["counts_as_input"]), default=float("inf"))
    early = [f"{r.get('sound_id')} at {r['first_seen_ms'] / 1000:.1f} s" for r in records
             if (r.get("category") == "alerts" or "alert" in str(r.get("sound_id")).lower()) and r["first_seen_ms"] < first_input]
    report.add("F5b", not early, "no alert sound before the first input" + (f"; {early}" if early else ""))
    return report


def render(folder, report):
    lines = [f"cold_play_check: {folder}", "MACHINE CHECKS"]
    for row in report.rows:
        lines.append(f"  {row['status']:<4} {row['check']:<4} {row['text']}")
    lines.append("MANUAL CHECKS (a person or an agent reads the evidence; this tool does not check them)")
    for check_id, text in MANUAL:
        lines.append(f"  TODO {check_id:<4} {text}")
    failed = report.failed
    if failed:
        lines.append(f"RESULT: MACHINE-FAIL: {', '.join(failed)}")
    else:
        lines.append("RESULT: MACHINE-PASS. The manual checks stay open. This is not PERSON-READY.")
    return "\n".join(lines)


def main(argv=None):
    parser = argparse.ArgumentParser(description="Check a cold-play evidence folder (no browser).")
    parser.add_argument("folder")
    parser.add_argument("--receipt", help="lock-receipt.json of the headed wrapper")
    parser.add_argument("--no-lock-check", action="store_true", help="the session did not run under the headed wrapper (a test)")
    parser.add_argument("--json", action="store_true", help="print the checks as JSON")
    args = parser.parse_args(argv)
    folder = Path(args.folder)
    try:
        report = check(folder, args.receipt, not args.no_lock_check)
    except Unusable as error:
        print(f"cold_play_check: {error}", file=sys.stderr)
        return 2
    except (KeyError, TypeError, AttributeError) as error:
        print(f"cold_play_check: the folder has an unexpected shape ({type(error).__name__}: {error})", file=sys.stderr)
        return 2
    if args.json:
        print(json.dumps({"machine": report.rows, "manual": [{"check": c, "text": t} for c, t in MANUAL], "failed": report.failed}, indent=2))
    else:
        print(render(folder, report))
    return 1 if report.failed else 0


if __name__ == "__main__":
    sys.exit(main())
