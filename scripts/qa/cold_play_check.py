#!/usr/bin/env python3
"""Check a cold-play evidence folder (task Q-COLD; docs/qa/cold-play.md).

    python3 -I -B scripts/qa/cold_play_check.py <folder> [--also <folder> ...] [--receipt lock-receipt.json]
                                                  [--no-lock-check] [--partial] [--json]

A readiness run has three folders: the first-click probe (cold-play.sh --probe first-click), the buttons probe
(--probe buttons) and the agent session. Give one of them first and the others with --also. A folder of a
positive control (--control close-layer) is judged on its probe only.

The result line is NOT-READY when any machine check or probe fails or a part of the run is missing, and
MACHINE-PASS (manual checks open) otherwise. The checker never reads the answer of an agent: an agent reads every
text and obeys it, a person clicks what they see (input-ceo-second-play-test.md; errata-pending-6.3.md, last
section). --partial skips the missing-part rule and prints PARTIAL-PASS (tests and a look at one folder).

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
  P1  the first-click probe passed (every own unit click gave a selection and a visible change)
  U0  the UI records exist (before the first input; after the first click)
  F9  no visible text block of more than 40 words after the first click that no input opened
  F3  no layer of more than half of the page that takes pointer input and is not opaque, after the first click
  F6  no alert line before the first input
  F8  the buttons probe: the first click on each visible button changed the UI record or the page

Exit: 0 every machine check passes; 1 a machine check fails; 2 a folder cannot be read.
Python 3 standard library only.
"""

import argparse
import hashlib
import json
import re
import sys
from pathlib import Path

STATEMENT_KEYS = ("GOAL", "GOAL_SHOT", "FIRST_ACTION", "FIRST_ACTION_SHOT", "WORKED", "WORKED_SHOT")
SHOT_NAME = re.compile(r"^(?:0000-start|\d{4}-[a-z]+-p(?:200|1000)|probe-(?:[a-z]+-)?\d{3}-[ab])\.png$")
KEY_NAME = re.compile(r"^[A-Za-z0-9]{1,24}$")
# "do not know" anywhere in a part is a fail (protocol check C3); an empty-answer word fails when it starts the part.
NO_ANSWER = re.compile(r"\b(?:do not know|don['\u2019]?t know|no idea)\b|^(?:unknown|none|n/a|not sure|unsure|unclear|nothing)\b", re.IGNORECASE)
MAX_LIMIT_MS = 1500 * 1000
LIMIT_GRACE_MS = 10000
WRAPPER_HOLD_S = 1800
# The slack for a screenshot: it is taken after the wait; late is the fault.
SHOT_LATE_MS = {200: 500, 1000: 700}
MAX_WORDS = 40
MAX_TOTAL_WORDS = 60
GOAL_LINE = re.compile(r"\b(goal|objective|mission|destroy|win|capture|defend|survive|research)\b", re.IGNORECASE)
# The files of the tool: their sha256 is written to session.json and compared by check C5.
TOOL_FILES = ("tests/browser/cold-probe.js", "tests/browser/cold-pixels.js", "tests/browser/cold-session.js", "tests/browser/cold-play.js",
              "tests/browser/harness/png.js", "scripts/qa/cold_play_check.py")
REQUIRED_SIZES = ("1280x720", "1024x768")
SEE_THROUGH_SHARE = 0.05
# The times are whole milliseconds, so a screenshot can start 1 ms before its mark; more than that is early.
EARLY_JITTER_MS = 3

MANUAL = (
    ("C0", "the prompt holds only the start command or URL and the one question (the reviewer greps the prompt)"),
    ("C1", "the agent read no repository file, memory note or earlier report before the statement (the transcript)"),
    ("C2", "each click, drag and wheel point and each key above was seen in an earlier screenshot (match inputs.jsonl to shots/)"),
    ("C3m", "the goal, the first action and the proof are true to the screenshots that they name (read the three files)"),
    ("C4", "the failure signs that no machine check covers: F1, F4, F7, F10, and F2 by eye (F3, F5, F6, F8, F9 are machine-checked above when the probes and the UI records are in the run)"),
    ("C5", "the reference red: on the 9b3a7bd forge build the first-click probe FAILS and its control (--control close-layer) PASSES (run once for each tool version)"),
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
        self.label = ""

    def add(self, check, ok, text):
        self.rows.append({"check": check, "status": "PASS" if ok else "FAIL", "text": text, "folder": self.label})

    def info(self, check, text):
        self.rows.append({"check": check, "status": "INFO", "text": text, "folder": self.label})

    @property
    def failed(self):
        return [row["check"] if not row["folder"] else f"{row['check']}[{row['folder'].split()[0]}]" for row in self.rows if row["status"] == "FAIL"]


FINDING_CLASSES = ("no-response", "wrong-response", "stale-text", "hidden-control", "covered-control", "unclear-text", "too-much-text", "sound", "other")


def parse_report(text, calls, shots):
    """The fixed report form of an agent session. Returns (findings, stuck, problems); a yes or no line is a problem."""
    findings, problems, stuck = [], [], None
    for number, line in enumerate(text.splitlines(), 1):
        if not line.strip():
            continue
        if re.match(r"^\s*(yes|no|answer|verdict|ready|not ready)\b", line, re.IGNORECASE):
            problems.append(f"line {number}: the form has no yes or no line")
            continue
        match = re.match(r"^FINDING (\d+): (.*)$", line)
        if match:
            parts = [part.strip() for part in match.group(2).split("|")]
            if int(match.group(1)) != len(findings) + 1:
                problems.append(f"line {number}: findings are numbered in order")
            if len(parts) != 5:
                problems.append(f"line {number}: a finding has five parts")
                continue
            cls, shot, call, expected, seen = parts
            if cls not in FINDING_CLASSES:
                problems.append(f"line {number}: unknown class {cls}")
            if Path(shot).name not in shots:
                problems.append(f"line {number}: {shot} is not a screenshot of the session")
            if call != "none" and not (call.isdigit() and 1 <= int(call) <= calls):
                problems.append(f"line {number}: INPUT is a call number or none")
            if not expected or not seen or len(expected.split()) > 12 or len(seen.split()) > 12:
                problems.append(f"line {number}: EXPECTED and SEEN are 1 to 12 words")
            findings.append(parts)
            continue
        match = re.match(r"^STUCK: (.*)$", line)
        if match:
            if stuck is not None:
                problems.append(f"line {number}: STUCK twice")
            stuck = "none" if match.group(1).strip() == "none" else [part.strip() for part in match.group(1).split("|")]
            continue
        problems.append(f"line {number} is not FINDING or STUCK")
    if stuck is None:
        problems.append("STUCK is missing")
    return findings, stuck, problems


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


def probe_checks(folder, mode, control, report):
    """P1, P3, P4, P5 (first-click probe), PC (its control), F8s and F8 (buttons probe): the verdict of a probe record."""
    if mode not in ("probe-first-click", "probe-buttons"):
        return
    name = "probe-first-click.json" if mode == "probe-first-click" else "probe-buttons.json"
    path = folder / name
    if not path.is_file():
        report.add("PC" if control else ("P1" if mode == "probe-first-click" else "F8"), False, f"{name} is missing: the probe did not finish")
        return
    record = load_json(path)
    summary = record.get("summary", {})
    ok = summary.get("verdict") == "PASS"
    if mode == "probe-first-click":
        outside = record.get("buttons_outside_page", [])
        if control:
            layer = record.get("layer_close") or {}
            report.add("PC", ok and bool(layer.get("closed")), f"control ({control}): {summary.get('verdict')}; layer closed by the tool: {layer.get('closed')}"
                       + (f"; the close control is outside the page: {layer.get('off_page')}" if layer.get("off_page") else "") + f"; {summary.get('reason')}")
            # The order and extra clicks are shown, not judged: a control is judged on the layer only.
            for key, label in (("orders", "P3"), ("extras", "P4")):
                part = summary.get(key, {})
                report.info(label, f"control, not judged: {part.get('verdict')}: {part.get('reason')}")
            return
        report.add("P1", ok and summary.get("clicks", 0) >= 1, f"first-click probe {summary.get('verdict')}: {summary.get('reason')}")
        for key, label in (("orders", "P3"), ("extras", "P4")):
            part = summary.get(key, {"verdict": "SKIPPED"})
            if part.get("verdict") == "SKIPPED":
                report.info(label, f"skipped: {part.get('reason', 'the first-click part failed')}")
            else:
                report.add(label, part.get("verdict") == "PASS", f"{part.get('verdict')}: {part.get('reason')}")
        report.add("P5", not outside, "no visible button outside the page at this size" + (f"; outside: {outside}" if outside else ""))
    else:
        start = summary.get("start", {})
        report.add("F8s", start.get("verdict") in ("PASS", "SKIPPED"), f"a button under the start overlay: {start.get('verdict')}: {start.get('reason')}")
        lost = [button for button in record.get("buttons", []) if not button.get("acted")]
        report.add("F8", ok and not lost, f"buttons probe {summary.get('verdict')}: {summary.get('reason')}")


def words(text):
    return len(text.split()) if isinstance(text, str) else 0


def ui_checks(folder, mode, report, by_kind):
    """U0, F9, F9b, G1, F3, F6 from ui-records.json and the watch of the probe: the UI record before the first input and after the first click."""
    path = folder / "ui-records.json"
    if not path.is_file():
        report.add("U0", False, "ui-records.json is missing")
        return
    records = load_json(path).get("records", [])
    by_label = {record.get("label"): record for record in records}
    has_click = bool(by_kind.get("first_click"))
    has_input = bool(by_kind.get("input")) and any(e.get("counts_as_input") for e in by_kind.get("input", []))
    after = [by_label[label] for label in ("after_first_click_p200", "after_first_click_p1000") if label in by_label]
    unavailable = [r.get("label") for r in records if not r.get("available")]
    ok = (not has_input or "before_first_input" in by_label) and (not has_click or len(after) == 2) and not unavailable
    report.add("U0", ok, f"UI records: {[r.get('label') for r in records]}" + (f"; the debug bridge did not answer for {unavailable}" if unavailable else ""))
    if not records or unavailable:
        return
    # F9: a visible text block of more than 40 words; F9b: more than 60 words in all (a long text split into small widgets).
    long_texts, total = {}, 0
    for record in after:
        sum_now = 0
        for widget in record.get("widgets", []):
            count = words(widget.get("text"))
            if widget.get("kind") == "focus_overlay" or "tooltip" in str(widget.get("id")) or str(widget.get("id")).startswith("world"):
                continue
            sum_now += count
            if count > MAX_WORDS:
                long_texts[widget["id"]] = count
        total = max(total, sum_now)
    report.add("F9", not long_texts, f"no visible text of more than {MAX_WORDS} words after the first click"
               + (f"; {', '.join(f'{k}: {v} words' for k, v in sorted(long_texts.items()))}" if long_texts else ""))
    report.add("F9b", total <= MAX_TOTAL_WORDS, f"at most {MAX_TOTAL_WORDS} words of visible text in all after the first click ({total} words)")
    # The words of the first text that states a goal: recorded, not a verdict yet.
    goal = None
    for record in after[-1:]:
        for widget in record.get("widgets", []):
            if isinstance(widget.get("text"), str) and GOAL_LINE.search(widget["text"]) and widget.get("kind") != "focus_overlay":
                goal = widget
                break
    report.info("G1", f"goal line (recorded, not judged): {words(goal['text'])} words in {goal['id']}" if goal else "goal line (recorded, not judged): no visible text that states a goal")
    # F3: a layer of more than half of the page that takes pointer input and lets the world show through.
    layers = {}
    for record in after:
        for widget in record.get("big_widgets", []):
            if not (widget.get("capture_root") or widget.get("focus_policy_block")):
                continue
            share = widget.get("nonflat_share")
            if share is None or share >= SEE_THROUGH_SHARE:
                layers[widget["id"]] = (widget.get("area_share"), share)
    report.add("F3", not layers, "no layer of more than half of the page takes input and shows the world through, after the first click"
               + (f"; {', '.join(f'{k} (area {a:.0%}, see-through ' + ('unmeasured' if s is None else f'{s:.0%}') + ')' for k, (a, s) in sorted(layers.items()))}" if layers else ""))
    # F6: an alert line before the first input, at any second of the watch.
    before = by_label.get("before_first_input")
    watch = []
    for name in ("probe-first-click.json", "probe-buttons.json"):
        if (folder / name).is_file():
            watch = load_json(folder / name).get("watch", [])
    seen = [(w["t_ms"], w.get("alert_lines", 0), w.get("alerts", 0)) for w in watch if w.get("alert_lines", 0) or w.get("alerts", 0)]
    if before is not None:
        lines, alerts = before.get("alert_lines", 0), before.get("alerts", 0)
        if lines or alerts:
            seen.append((before.get("t_ms"), lines, alerts))
        report.add("F6", not seen, f"no alert line before the first input (watched {len(watch)} s + the record before the input)"
                   + (f"; first at {seen[0][0] / 1000:.1f} s: {seen[0][1]} alert line{'s' if seen[0][1] != 1 else ''} (debug alerts: {seen[0][2]})" if seen else ""))
    else:
        report.info("F6", "no input in the session: nothing to compare")


def size_of(session):
    view = session.get("viewport") or {}
    return f"{view.get('width')}x{view.get('height')}"


def tool_hashes_now():
    """The sha256 of each file of the tool under review (the repository that holds this checker)."""
    root = Path(__file__).resolve().parents[2]
    return {name: hashlib.sha256((root / name).read_bytes()).hexdigest() for name in TOOL_FILES if (root / name).is_file()}


def reference_checks(folders, report):
    """C5: the reference red, repeated for THIS tool version. The folders are runs on the reference build (9b3a7bd forge).
    Each must record the sha256 of the tool files that are here now; the probe must FAIL and the control must PASS."""
    now = tool_hashes_now()
    report.label = "reference " + ", ".join(str(f) for f in folders)
    stale, probe_fail, control_pass, builds = [], False, False, set()
    for folder in folders:
        session = load_json(folder / "session.json")
        tool = session.get("tool") or {}
        recorded = tool.get("files") or {}
        differ = sorted(name for name in now if recorded.get(name) != now[name])
        if differ or not recorded or tool.get("dirty"):
            stale.append(f"{folder.name}: " + ("no tool record" if not recorded else f"differs: {', '.join(Path(n).name for n in differ)}" if differ else "tool tree was dirty"))
        builds.add(str(session.get("build")))
        probe_path = folder / "probe-first-click.json"
        if probe_path.is_file():
            summary = load_json(probe_path).get("summary", {})
            if session.get("control"):
                layer = load_json(probe_path).get("layer_close") or {}
                control_pass = control_pass or (summary.get("verdict") == "PASS" and bool(layer.get("closed")))
            else:
                probe_fail = probe_fail or summary.get("verdict") == "FAIL"
    report.add("C5a", not stale, "every reference run was made with the tool files that are here (sha256 in session.json); otherwise run the reference again for this tool version"
               + (f"; {stale}" if stale else ""))
    report.add("C5b", probe_fail and control_pass, f"on the reference build the first-click probe FAILS ({probe_fail}) and its control PASSES ({control_pass})")
    report.add("C5c", bool(builds) and all("9b3a7bd" in b for b in builds), f"the reference runs name the build 9b3a7bd: {sorted(builds)}")


def check(folder, receipt=None, lock_check=True, report=None):
    """Check one folder. Returns what the verdict needs: the mode, whether it is a control, the probe verdict."""
    session = load_json(folder / "session.json")
    events = load_events(folder)
    mode = session.get("mode", "agent")
    control = session.get("control")
    report = report if report is not None else Report()
    report.label = f"{mode}{' control' if control else ''} {folder}"
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
    except FileNotFoundError:
        inputs_lines = []  # a session with no input writes no file
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
    clean = (end is not None and bool(end_events) and end.get("reason") in ("limit", "end", "probe-done")
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

    if control:
        probe_checks(folder, mode, control, report)
        return {"mode": mode, "control": control, "size": size_of(session)}
    if mode == "agent":
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
        # C6: the report in its fixed form. The reviewer reads the findings; no yes or no is read.
        report_path = folder / "report.txt"
        if report_path.is_file() and by_kind.get("report_stored"):
            findings, stuck, problems = parse_report(report_path.read_text(encoding="utf-8"), len(input_events), set(shot_events))
            report.add("C6", not problems, f"report.txt has the fixed form: {len(findings)} findings, stuck: {'no' if stuck == 'none' else 'yes'}" + (f"; PROBLEMS: {problems}" if problems else ""))
            for index, parts in enumerate(findings, 1):
                report.info("C6", f"FINDING {index}: {' | '.join(parts)}")
            if stuck not in (None, "none"):
                report.info("C6", f"STUCK: {' | '.join(stuck)}")
        else:
            report.add("C6", False, "no report.txt in the fixed form (the session call: report FILE)")

    # S1, F5: the sound records.
    records_path = folder / "sound-records.json"
    sound = load_json(records_path) if records_path.is_file() else None
    if sound is None:
        report.add("S1", False, "sound-records.json is missing")
        probe_checks(folder, mode, control, report)
        ui_checks(folder, mode, report, by_kind)
        return {"mode": mode, "control": control, "size": size_of(session)}
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
    if mode == "agent":  # the sounds of a probe are the sounds of its own scripted clicks
        report.add("F5a", not crowded, f"no sound id more than 3 times in 60 s ({len(records)} sounds, {len(by_id)} ids)" + (f"; {crowded}" if crowded else ""))
    else:
        report.info("F5a", "not judged in a probe: its clicks make the sounds")
    first_input = min((e["t_start_ms"] for e in input_events if e["counts_as_input"]), default=float("inf"))
    early = [f"{r.get('sound_id')} at {r['first_seen_ms'] / 1000:.1f} s" for r in records
             if (r.get("category") == "alerts" or "alert" in str(r.get("sound_id")).lower()) and r["first_seen_ms"] < first_input]
    report.add("F5b", not early, "no alert sound before the first input" + (f"; {early}" if early else ""))
    probe_checks(folder, mode, control, report)
    ui_checks(folder, mode, report, by_kind)
    return {"mode": mode, "control": control, "size": size_of(session)}


def verdict(report, infos, partial, reference_given=False):
    """The result word. It never depends on an answer of an agent."""
    failed = report.failed
    if infos and all(info["control"] for info in infos):
        return ("CONTROL-FAIL: " + ", ".join(failed)) if failed else "CONTROL-PASS (the probe can pass: it is not a readiness run)"
    have = {(info["mode"], info["size"]) for info in infos if not info["control"]}
    modes = {mode for mode, _ in have}
    missing = []
    if not partial:
        missing += [f"first-click probe {size}" for size in REQUIRED_SIZES if ("probe-first-click", size) not in have]
        missing += [name for name, key in (("buttons probe", "probe-buttons"), ("agent session", "agent")) if key not in modes]
        if not reference_given:
            missing.append("reference run (C5)")
    if failed or missing:
        parts = ([f"failed: {', '.join(failed)}"] if failed else []) + ([f"missing: {', '.join(missing)}"] if missing else [])
        return f"NOT-READY ({'; '.join(parts)})"
    if partial:
        return "PARTIAL-PASS (manual checks open; one folder is not a readiness run)"
    return "MACHINE-PASS (manual checks open)"


def render(folders, report, result):
    lines = ["cold_play_check: " + ", ".join(str(f) for f in folders), "MACHINE CHECKS"]
    label = None
    for row in report.rows:
        if row["folder"] != label:
            label = row["folder"]
            lines.append(f"  == {label}")
        lines.append(f"  {row['status']:<4} {row['check']:<4} {row['text']}")
    lines.append("MANUAL CHECKS (a person or an agent reads the evidence; this tool does not check them)")
    for check_id, text in MANUAL:
        lines.append(f"  TODO {check_id:<4} {text}")
    lines.append(f"RESULT: {result}")
    return "\n".join(lines)


def main(argv=None):
    parser = argparse.ArgumentParser(description="Check cold-play evidence folders (no browser).")
    parser.add_argument("folder")
    parser.add_argument("--also", action="append", default=[], help="another folder of the same readiness run (a probe or the agent session)")
    parser.add_argument("--reference", action="append", default=[], help="a folder of a run on the reference build (probe or control), made with this tool version (C5)")
    parser.add_argument("--receipt", help="lock-receipt.json of the headed wrapper (for the first folder)")
    parser.add_argument("--no-lock-check", action="store_true", help="the session did not run under the headed wrapper (a test)")
    parser.add_argument("--partial", action="store_true", help="do not require all three parts of a readiness run")
    parser.add_argument("--json", action="store_true", help="print the checks as JSON")
    args = parser.parse_args(argv)
    folders = [Path(args.folder)] + [Path(f) for f in args.also]
    report = Report()
    infos = []
    try:
        for index, folder in enumerate(folders):
            infos.append(check(folder, args.receipt if index == 0 else None, not args.no_lock_check, report))
    except Unusable as error:
        print(f"cold_play_check: {error}", file=sys.stderr)
        return 2
    except (KeyError, TypeError, AttributeError) as error:
        print(f"cold_play_check: a folder has an unexpected shape ({type(error).__name__}: {error})", file=sys.stderr)
        return 2
    if args.reference:
        reference_checks([Path(f) for f in args.reference], report)
    result = verdict(report, infos, args.partial, bool(args.reference))
    if args.json:
        print(json.dumps({"machine": report.rows, "manual": [{"check": c, "text": t} for c, t in MANUAL], "failed": report.failed, "result": result}, indent=2))
    else:
        print(render(folders, report, result))
    return 0 if result.split()[0] in ("MACHINE-PASS", "PARTIAL-PASS", "CONTROL-PASS") else 1


if __name__ == "__main__":
    sys.exit(main())
