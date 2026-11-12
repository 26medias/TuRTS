#!/usr/bin/env python3
"""Re-freeze classifier for the one change of the M2 operation stream.

Pre-MP1 slice, task Q1a. Design: docs/superpowers/specs/
2026-11-12-pre-mp1-slice-design.md, section 5 and section 9 items 6, 7
and 14 (revision 5). Python 3 standard library only.

The classifier reads two runs: "old" (the build before the change) and
"new" (the build after the change). For each run it reads one trace for each
package and the operation stream. It prints one line for each check, the
sha256 of each file that it read, and "RESULT PASS" (exit 0) or
"RESULT FAIL" (exit 1). A check that has no input gives FAIL, never PASS.

Modes:
    --p0 OLD_DIR NEW_DIR --stream F --emitter FILE [FILE ...]
        A2a and A2b. The stream does not change at these steps, so F is
        the frozen stream. Checks: COUNTS, EMITTER, OLD-HASH, STREAM-OLD
        and STREAM-NEW (F is the stream of both traces), P0 on all four
        packages.
    --a2c --stage before-fixture-commit|after-fixture-commit
          --old-trace OLD_DIR --new-trace NEW_DIR --old-stream F
          --new-stream F --allowed F --fixture F --emitter FILE [FILE ...]
        A2c and Q2. All checks below.
        - before-fixture-commit: A2c has landed and the builder did not
          touch the fixture (design section 5 item 2). The fixture must
          still have the frozen sha256.
        - after-fixture-commit: the Q2 commit wrote the fixture. The
          fixture must have the bytes of the classified new stream.
        In both stages the fixture and the new stream must be two
        different files. The new stream is the stream that the new run
        wrote; keep it outside the repository tree.
    --self-test
        Run the plants of tests/m3/test_refreeze_classify.py.
    Option --g1-in-a2c: G1 lands in A2c (design section 5 item 0, 9.16).
        Only then can a trace hold edge_hit and the list allow it.

EMITTER: the emitter sha256 is the sha256 of the bytes of the --emitter
files, joined in the order of the command line. The classifier calculates
it and compares it with the header of each trace.

TRACE FORMAT "turts-refreeze-trace/2" (task Q1b writes the emitter)
-------------------------------------------------------------------
One file for each package: <dir>/<package>.jsonl, for base, haul, acme and
cedar. UTF-8. One JSON object on each line. Each line ends with "\\n". No
blank line. Integers are JSON integers, flags are JSON true or false. The
order of keys in an object is free; the emitter must give the same bytes
for the same state (P0 compares bytes).

Line 1, the header:
    {"record": "header", "format": "turts-refreeze-trace/2",
     "package": P, "emitter_sha256": H, "ticks": 400, "unit_count": N,
     "units": [{"unit": U, "content_id": C, "owner_slot": S}, ...]}
    - units: the units that exist after load_package and before the first
      command, in ascending unit id. unit_count is the length of units.
    - emitter_sha256: 64 lowercase hex digits (see EMITTER above).
    - No line holds a value that changes from one run to the next (no
      time, no path, no commit), because P0 compares the bytes.

Then for each tick T = 1 to 400, in this order:
    1. One world record:
       {"record": "world", "package": P, "tick": T, "pools": N,
        "pending_movement_orders": N}
       - pools: the number of pools in the MovementPools resource.
       - pending_movement_orders: the number of pending movement orders.
    2. One unit record for each unit that exists after the update of tick
       T, in ascending unit id. Each initial unit has a record at each tick.
       {"record": "unit", "package": P, "tick": T, "unit": U,
        "x_bits": X, "y_bits": Y, "target": null | {"x_bits": X,
        "y_bits": Y}, "activity": A, "state": S, "order": O,
        "separated": B, "was_projected": B [, "edge_hit": B]}
       - x_bits, y_bits: f32::to_bits of the position, 8 lowercase hex.
       - target: the MoveTarget point as bits, or null when there is none.
       - activity: the MovementActivity of the MoveProgress of the unit
         ("Pending", "Moving", "Resting", "Blocked"), or "none" when the
         unit has no MoveProgress. Not the order or the gather stage
         (design section 9 item 7).
       - state: "plain" when the unit has a PlainMoveProgress
         (crates/turts_sim/src/movement.rs:58 at 94a50d3), "pool" when it
         is in a movement pool, else "none".
       - order: the current order of the unit as "<Kind>:<stage>", with
         "-" for an order that has no stage, or "none".
       - separated, was_projected: the two flags of the movement step
         (movement.rs:770, :1973, :2134 and :2008 at 94a50d3). The fallback
         of an engine trip starts on "separated || was_projected" (:2052).
         Both are false for a unit that does not step and on the snap
         paths. Design revision 5, section 9 item 14.
       - edge_hit: only with --g1-in-a2c; then in every unit record.
    3. One operation record for each operation that the stream puts at
       tick T, in capture order (the rule of
       crates/turts_pack/tests/m3_m2_stream.rs:53-128: the operations drained
       after the update of tick T, with the command operations of the
       start in tick 1):
       {"record": "op", "package": P, "tick": T, "op_id": N,
        "unit": U | null, "phase": "Command" | "Orders" |
        "PostMovementOrders" | "Other", "events": [E, ...],
        "before_equals_after": B}
       - op_id: the operation_id. In each file the ids are 1, 2, 3, ...
       - unit: OperationAudit.unit_id, or null.
       - events: each EconomyEvent as serde JSON (tick, ordinal,
         owner_slot, kind), with the full payload.
       - before_equals_after: OperationAudit.before == after.

ALLOWED LIST FORMAT "turts-refreeze-allowed/2" (tests/m3/refreeze_allowed.json)
--------------------------------------------------------------------------------
Task Q1b writes the list before the A2c commit (plan refinement R6). It
names each acme trace difference that the design predicts. Every entry must
occur; a stale entry is a FAIL.
    {"schema": "turts-refreeze-allowed/2", "package": "acme",
     "note": "free text (optional)",
     "unit_fields": [
        {"unit": U, "field": F, "ticks": [FIRST, LAST], "old": V,
         "new": V}],
     "world_fields": [
        {"field": W, "ticks": [FIRST, LAST], "old": V, "new": V}],
     "operations": [
        {"change": "added", "tick": T, "index": I, "unit": U | null,
         "phase": P, "events": [...], "before_equals_after": B}]}
    - F is one of target, activity, state, order, separated,
      was_projected, and edge_hit (only with --g1-in-a2c). W is pools or
      pending_movement_orders. A position field (x_bits, y_bits) cannot be
      allowed: the ruling P1 is "every position is bitwise equal over 400
      ticks" (docs/research/mp1/ruling-td-one-step-function.md, section 5).
    - LAST must be before the calculated arrival tick. From the arrival
      tick on, the acme trace must be equal to the old trace: the scout
      has no pool, no target and no leaked order.
    - An entry covers each tick from FIRST to LAST. At each of these ticks
      the old value must be "old" and the new value "new".
    - "added": the new operation at position I (0-based) in the list of
      tick T of the new trace.
    - There is no "changed" entry. Design section 5 item 3 (P3) admits an
      added or changed operation. In the stream, a changed operation can
      differ only in before_equals_after, because its event kinds must be
      empty; and the design requires that a flipped before_equals_after on
      the tick-1 operation is red (design lines 143-144). So every change
      of an old operation is a FAIL here. If A2c changes an old operation,
      the stop rule of design line 146 applies (needs a new ruling).
"""
import argparse
import hashlib
import importlib.util
import json
import math
from pathlib import Path
import re
import struct
import sys

ROOT = Path(__file__).resolve().parents[2]
PACKAGES = ("base", "haul", "acme", "cedar")
UNCHANGED_PACKAGES = ("base", "haul", "cedar")
CHANGED_PACKAGE = "acme"
TICKS = 400
TRACE_FORMAT = "turts-refreeze-trace/2"
ALLOWED_SCHEMA = "turts-refreeze-allowed/2"
STAGES = ("before-fixture-commit", "after-fixture-commit")
# The frozen stream before the one re-freeze (tests/m3/refutations.py:18).
FROZEN_SHA256 = "bcbf72c2206251f635a7c7f5b6c8f573d69a5fc05a5bd77716e98ebfb08e370b"
# m3_m2_stream.rs:258-288 applies MoveTo before the update of tick 1, and
# :53-56 puts those operations in tick 1.
ACCEPT_TICK = 1
SCOUT_CONTENT_ID = "acme:scout"
ACTIVITIES = {"Pending", "Moving", "Resting", "Blocked", "none"}
STATES = {"none", "plain", "pool"}
PHASES = {"Command", "Orders", "PostMovementOrders", "Other"}
ALLOWED_UNIT_FIELDS = {"target", "activity", "state", "order", "separated",
                       "was_projected"}
WORLD_FIELDS = ("pools", "pending_movement_orders")
UNIT_KEYS = {"record", "package", "tick", "unit", "x_bits", "y_bits",
             "target", "activity", "state", "order", "separated",
             "was_projected"}
OP_KEYS = {"record", "package", "tick", "op_id", "unit", "phase", "events",
           "before_equals_after"}
WORLD_KEYS = {"record", "package", "tick"} | set(WORLD_FIELDS)
HEADER_KEYS = {"record", "format", "package", "emitter_sha256", "ticks",
               "unit_count", "units"}
COMPARED_UNIT_FIELDS = ("x_bits", "y_bits", "target", "activity", "state",
                        "order", "separated", "was_projected", "edge_hit")
HEX8 = re.compile(r"[0-9a-f]{8}\Z")
HEX64 = re.compile(r"[0-9a-f]{64}\Z")
ORDER = re.compile(r"(none|[A-Za-z][A-Za-z0-9_]*:[A-Za-z0-9_-]+)\Z")


class Report:
    """The verdict lines of one classification."""

    def __init__(self):
        self.checks = []
        self.hashes = []

    def add(self, name, ok, detail):
        self.checks.append((name, bool(ok), detail))

    @property
    def passed(self):
        return bool(self.checks) and all(ok for _, ok, _ in self.checks)

    def failed(self):
        return {name: detail for name, ok, detail in self.checks if not ok}

    def lines(self):
        out = ["CHECK %-10s %s %s" % (name, "PASS" if ok else "FAIL", detail)
               for name, ok, detail in self.checks]
        out += ["SHA256 %s %s %s" % row for row in self.hashes]
        out.append("RESULT " + ("PASS" if self.passed else "FAIL"))
        return out


def sha256_file(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def is_int(value):
    return isinstance(value, int) and not isinstance(value, bool)


def is_count(value):
    return is_int(value) and value >= 0


def event_kind(value):
    """The kind name, as m3_m2_stream.rs:151-163 takes it."""
    if not isinstance(value, dict) or "kind" not in value:
        raise ValueError("event has no kind: %r" % (value,))
    kind = value["kind"]
    if isinstance(kind, str):
        return kind
    if isinstance(kind, dict) and len(kind) == 1:
        return next(iter(kind))
    raise ValueError("event kind is not a string or a one-key object")


class Trace:
    """One parsed trace file."""

    def __init__(self, package, raw, header):
        self.package = package
        self.raw = raw
        self.header = header
        self.initial = [u["unit"] for u in header["units"]]
        self.units = {}
        self.world = {}
        self.ops = []
        self.ops_by_tick = {tick: [] for tick in range(1, TICKS + 1)}
        self.op_by_id = {}


def check_target(value):
    if value is None:
        return True
    return (isinstance(value, dict) and set(value) == {"x_bits", "y_bits"}
            and all(isinstance(value[k], str) and HEX8.match(value[k])
                    for k in value))


def check_header(package, head):
    if head.get("record") != "header" or set(head) != HEADER_KEYS:
        return "%s line 1: not a header with keys %s" % (
            package, sorted(HEADER_KEYS))
    if head["format"] != TRACE_FORMAT:
        return "%s: format %r, expected %r" % (
            package, head["format"], TRACE_FORMAT)
    if head["package"] != package:
        return "%s: header package %r" % (package, head["package"])
    if not (isinstance(head["emitter_sha256"], str)
            and HEX64.match(head["emitter_sha256"])):
        return "%s: emitter_sha256 is not 64 lowercase hex" % package
    if not is_int(head["ticks"]) or head["ticks"] != TICKS:
        return "%s: header ticks %r, expected %d" % (
            package, head["ticks"], TICKS)
    units = head["units"]
    if not isinstance(units, list) or not units:
        return "%s: header has no initial units" % package
    for unit in units:
        if (not isinstance(unit, dict)
                or set(unit) != {"unit", "content_id", "owner_slot"}
                or not is_int(unit["unit"])
                or not isinstance(unit["content_id"], str)
                or not is_int(unit["owner_slot"])):
            return "%s: bad header unit %r" % (package, unit)
    ids = [u["unit"] for u in units]
    if ids != sorted(set(ids)):
        return "%s: header unit ids are not ascending" % package
    if not is_int(head["unit_count"]) or head["unit_count"] != len(units):
        return "%s: unit_count %r, but %d units" % (
            package, head["unit_count"], len(units))
    return None


def check_unit_record(where, record, g1_in_a2c):
    keys = set(record)
    if g1_in_a2c:
        if keys != UNIT_KEYS | {"edge_hit"}:
            return "%s: unit keys %s (edge_hit is required with " \
                   "--g1-in-a2c)" % (where, sorted(keys))
        if not isinstance(record["edge_hit"], bool):
            return "%s: edge_hit is not a bool" % where
    elif keys != UNIT_KEYS:
        if "edge_hit" in keys:
            return "%s: edge_hit in a run that is not --g1-in-a2c" % where
        return "%s: unit keys %s" % (where, sorted(keys))
    if not is_int(record["unit"]):
        return "%s: unit is not an integer" % where
    for field in ("x_bits", "y_bits"):
        if not (isinstance(record[field], str) and HEX8.match(record[field])):
            return "%s: %s is not 8 lowercase hex" % (where, field)
    if not check_target(record["target"]):
        return "%s: bad target %r" % (where, record["target"])
    if record["activity"] not in ACTIVITIES:
        return "%s: activity %r" % (where, record["activity"])
    if record["state"] not in STATES:
        return "%s: state %r" % (where, record["state"])
    if not (isinstance(record["order"], str) and ORDER.match(record["order"])):
        return "%s: order %r is not \"none\" or \"<Kind>:<stage>\"" % (
            where, record["order"])
    for field in ("separated", "was_projected"):
        if not isinstance(record[field], bool):
            return "%s: %s is not a bool" % (where, field)
    return None


def check_op_record(where, record, expected_id):
    if set(record) != OP_KEYS:
        return "%s: op keys %s" % (where, sorted(record))
    if not is_int(record["op_id"]) or record["op_id"] != expected_id:
        return "%s: op_id %r, expected %d (ids 1, 2, 3, ...)" % (
            where, record["op_id"], expected_id)
    if record["unit"] is not None and not is_int(record["unit"]):
        return "%s: op unit %r" % (where, record["unit"])
    if record["phase"] not in PHASES:
        return "%s: phase %r" % (where, record["phase"])
    if not isinstance(record["before_equals_after"], bool):
        return "%s: before_equals_after is not a bool" % where
    if not isinstance(record["events"], list):
        return "%s: events is not a list" % where
    try:
        for value in record["events"]:
            event_kind(value)
    except ValueError as error:
        return "%s: %s" % (where, error)
    return None


def load_trace(directory, package, g1_in_a2c=False):
    """Return (Trace, None) or (None, error text)."""
    if directory is None:
        return None, "%s: no trace given" % package
    path = Path(directory) / (package + ".jsonl")
    if not path.is_file():
        return None, "%s: no trace given (%s does not exist)" % (package, path)
    raw = path.read_bytes()
    if not raw:
        return None, "%s: empty trace (%s has 0 bytes)" % (package, path)
    if not raw.endswith(b"\n"):
        return None, "%s: the last line has no newline" % package
    records = []
    for number, line in enumerate(raw.split(b"\n")[:-1], 1):
        try:
            record = json.loads(line)
        except ValueError as error:
            return None, "%s line %d: not JSON (%s)" % (package, number, error)
        if not isinstance(record, dict):
            return None, "%s line %d: not an object" % (package, number)
        records.append(record)
    error = check_header(package, records[0])
    if error:
        return None, error

    trace = Trace(package, raw, records[0])
    initial = set(trace.initial)
    tick = 0
    last_unit = None
    in_ops = False
    seen_initial = set()
    for number, record in enumerate(records[1:], 2):
        where = "%s line %d" % (package, number)
        kind = record.get("record")
        if record.get("package") != package:
            return None, "%s: package %r" % (where, record.get("package"))
        rtick = record.get("tick")
        if not is_int(rtick):
            return None, "%s: tick is not an integer" % where
        if kind == "world":
            if rtick != tick + 1:
                return None, "%s: world record of tick %d after tick %d" % (
                    where, rtick, tick)
            if tick and seen_initial != initial:
                return None, "%s tick %d: initial units %s have no record" % (
                    package, tick, sorted(initial - seen_initial))
            if set(record) != WORLD_KEYS or not all(
                    is_count(record[f]) for f in WORLD_FIELDS):
                return None, "%s: world keys %s or a count that is not a " \
                    "whole number" % (where, sorted(record))
            tick = rtick
            last_unit = None
            in_ops = False
            seen_initial = set()
            trace.world[tick] = record
        elif kind == "unit":
            if rtick != tick or tick == 0:
                return None, "%s: unit record of tick %d, but tick %d has no " \
                    "world record" % (where, rtick, rtick)
            if in_ops:
                return None, "%s: unit record after an op record" % where
            error = check_unit_record(where, record, g1_in_a2c)
            if error:
                return None, error
            unit = record["unit"]
            if last_unit is not None and unit <= last_unit:
                return None, "%s: unit ids are not ascending" % where
            last_unit = unit
            if unit in initial:
                seen_initial.add(unit)
            trace.units[(tick, unit)] = record
        elif kind == "op":
            if rtick != tick or tick == 0:
                return None, "%s: op of tick %d after the records of tick %d" % (
                    where, rtick, tick)
            in_ops = True
            error = check_op_record(where, record, len(trace.ops) + 1)
            if error:
                return None, error
            trace.ops.append(record)
            trace.ops_by_tick[tick].append(record)
            trace.op_by_id[record["op_id"]] = record
        else:
            return None, "%s: record %r" % (where, kind)
    if tick == 0:
        return None, "%s: header only, 0 tick records" % package
    if seen_initial != initial:
        return None, "%s tick %d: initial units %s have no record" % (
            package, tick, sorted(initial - seen_initial))
    if tick != TICKS:
        return None, "%s: the trace ends at tick %d, expected %d" % (
            package, tick, TICKS)
    count = sum(1 for (t, u) in trace.units if u in initial)
    if count != TICKS * len(initial) or len(trace.world) != TICKS:
        return None, "%s: %d initial-unit records and %d world records, " \
            "expected %d x %d and %d" % (package, count, len(trace.world),
                                         TICKS, len(initial), TICKS)
    return trace, None


def load_traces(directory, report, check, g1_in_a2c):
    traces = {}
    errors = []
    details = []
    for package in PACKAGES:
        trace, error = load_trace(directory, package, g1_in_a2c)
        if error:
            errors.append(error)
            continue
        traces[package] = trace
        report.hashes.append((check.lower() + "-" + package,
                              hashlib.sha256(trace.raw).hexdigest(),
                              Path(directory) / (package + ".jsonl")))
        initial = len(trace.initial)
        details.append("%s %d ticks x %d initial units = %d, %d unit records, "
                       "%d world records, %d ops" % (
                           package, TICKS, initial, TICKS * initial,
                           len(trace.units), len(trace.world), len(trace.ops)))
    if errors:
        report.add(check, False, "; ".join(errors))
    else:
        report.add(check, True, "; ".join(details))
    return traces


def emitter_sha256(files):
    digest = hashlib.sha256()
    for path in files:
        digest.update(Path(path).read_bytes())
    return digest.hexdigest()


def check_emitter(report, old, new, emitter_files):
    if not emitter_files:
        report.add("EMITTER", False, "no emitter file given (--emitter)")
        return
    missing_files = [str(f) for f in emitter_files if not Path(f).is_file()]
    if missing_files:
        report.add("EMITTER", False, "emitter file does not exist: " +
                   ", ".join(missing_files))
        return
    expected = emitter_sha256(emitter_files)
    report.hashes.append(("emitter", expected, " + ".join(
        str(f) for f in emitter_files)))
    missing = [side + "/" + p for side, traces in (("old", old), ("new", new))
               for p in PACKAGES if p not in traces]
    if missing:
        report.add("EMITTER", False, "no valid trace for " + ", ".join(missing))
        return
    wrong = ["%s/%s=%s" % (side, p, traces[p].header["emitter_sha256"])
             for side, traces in (("old", old), ("new", new)) for p in PACKAGES
             if traces[p].header["emitter_sha256"] != expected]
    if wrong:
        report.add("EMITTER", False, "header emitter_sha256 is not the sha256 "
                   "%s of the emitter files, or the values differ: %s" % (
                       expected, ", ".join(wrong)))
        return
    report.add("EMITTER", True, "all 8 traces: %s (the sha256 of the emitter "
               "files)" % expected)


def first_line_difference(old, new):
    a = old.split(b"\n")
    b = new.split(b"\n")
    for number, (x, y) in enumerate(zip(a, b), 1):
        if x != y:
            return number
    return min(len(a), len(b)) + 1


def check_p0(report, old, new, packages):
    problems = []
    for package in packages:
        if package not in old or package not in new:
            problems.append("%s: no valid trace on both sides" % package)
            continue
        if old[package].raw != new[package].raw:
            problems.append("%s: bytes differ (first at line %d)" % (
                package, first_line_difference(old[package].raw,
                                               new[package].raw)))
    if problems:
        report.add("P0", False, "; ".join(problems))
    else:
        report.add("P0", True, "byte-equal: " + ", ".join(packages))


def load_json(path, label):
    if path is None:
        return None, "no %s given" % label
    path = Path(path)
    if not path.is_file():
        return None, "no %s given (%s does not exist)" % (label, path)
    try:
        return json.loads(path.read_bytes()), None
    except ValueError as error:
        return None, "%s is not JSON (%s)" % (label, error)


def check_span(problems, where, entry, spans, key, arrival):
    ticks = entry["ticks"]
    if not (isinstance(ticks, list) and len(ticks) == 2
            and all(is_int(t) for t in ticks)
            and 1 <= ticks[0] <= ticks[1] <= TICKS):
        problems.append("%s: ticks must be [first, last] in 1..400" % where)
        return
    if ticks[1] >= arrival:
        problems.append("%s: last tick %d is at or after the arrival tick %d; "
                        "from arrival on, acme must equal the old trace" % (
                            where, ticks[1], arrival))
    if entry["old"] == entry["new"]:
        problems.append("%s: old equals new" % where)
    for first, last in spans.get(key, []):
        if ticks[0] <= last and first <= ticks[1]:
            problems.append("%s: ticks overlap another entry" % where)
    spans.setdefault(key, []).append(tuple(ticks))


def load_allowed(report, path, arrival, g1_in_a2c):
    """Return the allowed list, or None (ALLOWED is then FAIL)."""
    data, error = load_json(path, "allowed list")
    if error:
        report.add("ALLOWED", False, error)
        return None
    report.hashes.append(("allowed", sha256_file(path), path))
    if arrival is None:
        report.add("ALLOWED", False, "no calculated arrival tick to check the "
                   "list against")
        return None
    if not isinstance(data, dict):
        report.add("ALLOWED", False, "the allowed list is not an object")
        return None
    problems = []
    extra = set(data) - {"schema", "package", "note", "unit_fields",
                         "world_fields", "operations"}
    if extra:
        problems.append("unknown keys %s" % sorted(extra))
    if data.get("schema") != ALLOWED_SCHEMA:
        problems.append("schema %r, expected %r" % (data.get("schema"),
                                                    ALLOWED_SCHEMA))
    if data.get("package") != CHANGED_PACKAGE:
        problems.append("package %r, expected acme" % data.get("package"))
    if "note" in data and not isinstance(data["note"], str):
        problems.append("note is not a string")
    fields = data.get("unit_fields")
    world = data.get("world_fields")
    ops = data.get("operations")
    if not all(isinstance(x, list) for x in (fields, world, ops)):
        problems.append("unit_fields, world_fields and operations must be lists")
        fields, world, ops = [], [], []
    allowed_fields = ALLOWED_UNIT_FIELDS | ({"edge_hit"} if g1_in_a2c else set())
    spans = {}
    for index, entry in enumerate(fields):
        where = "unit_fields[%d]" % index
        if not isinstance(entry, dict) or set(entry) != {
                "unit", "field", "ticks", "old", "new"}:
            problems.append("%s: keys must be unit, field, ticks, old, new" % where)
            continue
        if not is_int(entry["unit"]):
            problems.append("%s: unit is not an integer" % where)
        if entry["field"] not in allowed_fields:
            problems.append("%s: field %r cannot be allowed (allowed: %s%s)" % (
                where, entry["field"], ", ".join(sorted(allowed_fields)),
                "" if g1_in_a2c else "; edge_hit only with --g1-in-a2c"))
        check_span(problems, where, entry, spans,
                   ("unit", entry["unit"], entry["field"]), arrival)
    for index, entry in enumerate(world):
        where = "world_fields[%d]" % index
        if not isinstance(entry, dict) or set(entry) != {
                "field", "ticks", "old", "new"}:
            problems.append("%s: keys must be field, ticks, old, new" % where)
            continue
        if entry["field"] not in WORLD_FIELDS:
            problems.append("%s: field %r is not a world field" % (
                where, entry["field"]))
        check_span(problems, where, entry, spans, ("world", entry["field"]),
                   arrival)
    places = set()
    keys = {"change", "tick", "index", "unit", "phase", "events",
            "before_equals_after"}
    for index, entry in enumerate(ops):
        where = "operations[%d]" % index
        if not isinstance(entry, dict):
            problems.append("%s: not an object" % where)
            continue
        if entry.get("change") != "added":
            problems.append("%s: change must be \"added\"; a change of an old "
                            "operation cannot be allowed" % where)
            continue
        if set(entry) != keys:
            problems.append("%s: keys must be %s" % (where, sorted(keys)))
            continue
        if not (is_int(entry["tick"]) and 1 <= entry["tick"] <= TICKS
                and is_int(entry["index"]) and entry["index"] >= 0):
            problems.append("%s: bad tick or index" % where)
            continue
        if (entry["tick"], entry["index"]) in places:
            problems.append("%s: two entries for one place" % where)
        places.add((entry["tick"], entry["index"]))
        if entry["unit"] is not None and not is_int(entry["unit"]):
            problems.append("%s: bad unit" % where)
        if entry["phase"] not in PHASES:
            problems.append("%s: phase %r" % (where, entry["phase"]))
        if not isinstance(entry["events"], list):
            problems.append("%s: events is not a list" % where)
        if not isinstance(entry["before_equals_after"], bool):
            problems.append("%s: before_equals_after is not a bool" % where)
    if problems:
        report.add("ALLOWED", False, "; ".join(problems))
        return None
    report.add("ALLOWED", True, "%d unit field entries, %d world field "
               "entries, %d operation entries; all end before arrival %d" % (
                   len(fields), len(world), len(ops), arrival))
    return data


def op_body(record):
    return {k: record[k] for k in ("tick", "unit", "phase", "events",
                                   "before_equals_after")}


def apply_spans(problems, entries, differences, key_of, label):
    """Each entry must occur at each tick of its span. Return the keys that
    the entries cover."""
    used = set()
    for index, entry in enumerate(entries):
        first, last = entry["ticks"]
        for tick in range(first, last + 1):
            key = key_of(tick, entry)
            seen = differences.get(key)
            if seen != (entry["old"], entry["new"]):
                problems.append(
                    "allowed %s[%d] does not occur at tick %d (observed %r)" % (
                        label, index, tick, seen))
                break
            used.add(key)
    return used


def check_p1(report, old, new, allowed):
    package = CHANGED_PACKAGE
    if package not in old or package not in new:
        report.add("P1", False, "acme: no valid trace on both sides")
        return
    if allowed is None:
        report.add("P1", False, "no valid allowed list")
        return
    a, b = old[package], new[package]
    problems = []
    if a.header != b.header:
        problems.append("acme header differs")
    differences = {}
    for key in sorted(set(a.units) | set(b.units)):
        x, y = a.units.get(key), b.units.get(key)
        if x is None or y is None:
            problems.append("acme tick %d unit %d: record only in the %s trace"
                            % (key[0], key[1], "new" if x is None else "old"))
            continue
        for field in COMPARED_UNIT_FIELDS:
            if x.get(field) != y.get(field):
                differences[("unit",) + key + (field,)] = (x.get(field),
                                                           y.get(field))
    for tick in range(1, TICKS + 1):
        for field in WORLD_FIELDS:
            x, y = a.world[tick][field], b.world[tick][field]
            if x != y:
                differences[("world", tick, field)] = (x, y)
    used = apply_spans(problems, allowed["unit_fields"], differences,
                       lambda t, e: ("unit", t, e["unit"], e["field"]),
                       "unit_fields")
    used |= apply_spans(problems, allowed["world_fields"], differences,
                        lambda t, e: ("world", t, e["field"]), "world_fields")
    for key in sorted(set(differences) - used, key=repr)[:12]:
        old_value, new_value = differences[key]
        if key[0] == "unit":
            problems.append("acme tick %d unit %d field %s: %r -> %r is not "
                            "in the allowed list" % (key[1], key[2], key[3],
                                                     old_value, new_value))
        else:
            problems.append("acme tick %d world field %s: %r -> %r is not in "
                            "the allowed list" % (key[1], key[2], old_value,
                                                  new_value))

    # The expected new operations: the old ones with the listed additions,
    # then the ids 1, 2, 3, ... in order.
    expected = {tick: [op_body(r) for r in a.ops_by_tick[tick]]
                for tick in range(1, TICKS + 1)}
    added = sorted((e["tick"], e["index"], i)
                   for i, e in enumerate(allowed["operations"]))
    for tick, place, index in added:
        entry = allowed["operations"][index]
        ops = expected[tick]
        if place > len(ops):
            problems.append("allowed operations[%d] does not occur: index %d "
                            "is past the end of tick %d" % (index, place, tick))
            continue
        ops.insert(place, {"tick": tick, "unit": entry["unit"],
                           "phase": entry["phase"], "events": entry["events"],
                           "before_equals_after": entry["before_equals_after"]})
    flat = [body for tick in range(1, TICKS + 1) for body in expected[tick]]
    for op_id, body in enumerate(flat, 1):
        body["op_id"] = op_id
    actual = []
    for record in b.ops:
        body = op_body(record)
        body["op_id"] = record["op_id"]
        actual.append(body)
    if flat != actual:
        for index in range(max(len(flat), len(actual))):
            want = flat[index] if index < len(flat) else None
            got = actual[index] if index < len(actual) else None
            if want != got:
                problems.append("acme operation %d: expected %s, new trace has "
                                "%s (an operation difference that the allowed "
                                "list does not give)" % (
                                    index + 1, json.dumps(want, sort_keys=True),
                                    json.dumps(got, sort_keys=True)))
                break
    if problems:
        report.add("P1", False, "; ".join(problems))
    else:
        report.add("P1", True, "acme: %d field differences, all in the allowed "
                   "list; %d operations as listed" % (len(differences),
                                                      len(actual)))


def check_p2(report, old, new):
    problems = []
    total = 0
    for package in PACKAGES:
        if package not in old or package not in new:
            problems.append("%s: no valid trace on both sides" % package)
            continue
        for tick in range(1, TICKS + 1):
            x = [e for r in old[package].ops_by_tick[tick] for e in r["events"]]
            y = [e for r in new[package].ops_by_tick[tick] for e in r["events"]]
            total += len(y)
            if x != y:
                problems.append("%s tick %d: events differ: old %s, new %s" % (
                    package, tick, json.dumps(x, sort_keys=True),
                    json.dumps(y, sort_keys=True)))
                break
    if problems:
        report.add("P2", False, "; ".join(problems))
    else:
        report.add("P2", True, "%d events with payloads, equal on each tick in "
                   "all four packages" % total)


def stream_structure(data):
    """Return {package: {tick: [op, ...]}} or raise ValueError."""
    if not isinstance(data, list):
        raise ValueError("the stream is not a list")
    names = [entry.get("package") if isinstance(entry, dict) else None
             for entry in data]
    if names != list(PACKAGES):
        raise ValueError("packages %r, expected %r" % (names, list(PACKAGES)))
    out = {}
    for entry in data:
        package = entry["package"]
        if set(entry) != {"package", "ticks"}:
            raise ValueError("%s: keys %s" % (package, sorted(entry)))
        ticks = entry["ticks"]
        if not isinstance(ticks, list) or [t.get("tick") for t in ticks] != list(
                range(1, TICKS + 1)):
            raise ValueError("%s: ticks are not 1..400" % package)
        out[package] = {}
        for tick in ticks:
            if set(tick) != {"tick", "operation_count", "operations"}:
                raise ValueError("%s tick %s: keys" % (package, tick.get("tick")))
            ops = tick["operations"]
            if tick["operation_count"] != len(ops):
                raise ValueError("%s tick %d: operation_count %r, %d operations"
                                 % (package, tick["tick"],
                                    tick["operation_count"], len(ops)))
            for op in ops:
                if set(op) != {"operation_id", "event_kinds",
                               "before_equals_after"}:
                    raise ValueError("%s tick %d: operation keys" % (
                        package, tick["tick"]))
            out[package][tick["tick"]] = ops
    return out


def stream_bytes(data):
    """The bytes of serde_json::to_string_pretty plus a newline, as
    m3_m2_stream.rs:384-392 writes the fixture (checked on the real file)."""
    return (json.dumps(data, indent=2, sort_keys=True) + "\n").encode()


def projection(trace):
    return {"package": trace.package, "ticks": [{
        "tick": tick,
        "operation_count": len(trace.ops_by_tick[tick]),
        "operations": [{
            "operation_id": r["op_id"],
            "event_kinds": [event_kind(e) for e in r["events"]],
            "before_equals_after": r["before_equals_after"],
        } for r in trace.ops_by_tick[tick]],
    } for tick in range(1, TICKS + 1)]}


def load_stream(report, path, label, traces, check):
    """The stream must be, byte for byte, the projection of the trace of the
    same run. Return the stream structure, or None."""
    data, error = load_json(path, label)
    if error:
        report.add(check, False, error)
        return None
    raw = Path(path).read_bytes()
    report.hashes.append((label.replace(" ", "-"), sha256_file(path), path))
    try:
        stream = stream_structure(data)
    except ValueError as error:
        report.add(check, False, "structure: %s" % error)
        return None
    counts = ", ".join("%s %d ops" % (p, sum(len(v) for v in stream[p].values()))
                       for p in PACKAGES)
    missing = [p for p in PACKAGES if p not in traces]
    if missing:
        report.add(check, False, "structure ok (%s); trace: no valid trace to "
                   "compare for %s" % (counts, ", ".join(missing)))
        return stream
    projected = [projection(traces[p]) for p in PACKAGES]
    if raw != stream_bytes(projected):
        where = "the JSON types or the layout"
        for package, entry in zip(PACKAGES, projected):
            for tick in range(1, TICKS + 1):
                if stream[package][tick] != entry["ticks"][tick - 1]["operations"]:
                    where = "%s tick %d" % (package, tick)
                    break
            else:
                continue
            break
        report.add(check, False, "structure ok (%s); the stream bytes are not "
                   "the bytes of the trace of the same run (first at %s)" % (
                       counts, where))
        return stream
    report.add(check, True, "structure ok (%s); byte-equal to the projection "
               "of the trace" % counts)
    return stream


def source_value(path, pattern, label):
    text = Path(path).read_text()
    found = re.findall(pattern, text, re.S)
    if len(found) != 1:
        raise ValueError("%s: %d matches in %s, expected 1" % (
            label, len(found), path))
    return found[0]


def arrival_inputs(repo):
    """Read the arrival inputs from the package and the stream test, not from
    the trace or from the code under test."""
    repo = Path(repo)
    test = repo / "crates/turts_pack/tests/m3_m2_stream.rs"
    speed = float(source_value(
        repo / "mods/acme/units/scout.ron",
        r"movement:\s*Some\(\(speed:\s*([0-9.]+)\)\)", "scout speed"))
    start = source_value(
        repo / "mods/acme/maps/field.ron",
        r'unit:\s*"acme:scout"[^)]*?pos:\s*\(x:\s*([-0-9.]+),\s*y:\s*([-0-9.]+)\)',
        "scout placement")
    size = source_value(
        repo / "mods/acme/maps/field.ron",
        r"size:\s*\(x:\s*([0-9.]+),\s*y:\s*([0-9.]+)\)", "map size")
    target = source_value(
        test, r"CommandKind::MoveTo\s*\{[^}]*?target:\s*Vec2::new\(\s*"
              r"([-0-9.]+),\s*([-0-9.]+)\s*\)", "MoveTo target")
    tick_hz = int(source_value(test, r"headless_app\((\d+)\)", "tick rate"))
    return {
        "speed": speed,
        "start": (float(start[0]), float(start[1])),
        "size": (float(size[0]), float(size[1])),
        "target": (float(target[0]), float(target[1])),
        "tick_hz": tick_hz,
    }


def f32_bits(value):
    return "%08x" % struct.unpack("<I", struct.pack("<f", value))[0]


def calculate_arrival(repo):
    """Return ((tick, text, target bits), None) or (None, error text)."""
    try:
        inputs = arrival_inputs(repo)
    except (OSError, ValueError) as error:
        return None, "inputs: %s" % error
    (x0, y0), (x1, y1) = inputs["start"], inputs["target"]
    if not (0 <= x1 <= inputs["size"][0] and 0 <= y1 <= inputs["size"][1]):
        return None, "the target is off the map; the calculation does not hold"
    step = inputs["speed"] / inputs["tick_hz"]
    ratio = math.hypot(x1 - x0, y1 - y0) / step
    if abs(ratio - round(ratio)) < 1e-3:
        return None, ("distance / step = %.6f is within 0.001 of a whole "
                      "number; f32 rounding decides (needs decision)" % ratio)
    tick = math.ceil(ratio)
    text = ("ceil(|(%g, %g) - (%g, %g)| / (%g x 1/%d)) = ceil(%.4f) = %d" % (
        x1, y1, x0, y0, inputs["speed"], inputs["tick_hz"], ratio, tick))
    return (tick, text, (f32_bits(x1), f32_bits(y1))), None


def scout_id(trace):
    ids = [u["unit"] for u in trace.header["units"]
           if u["content_id"] == SCOUT_CONTENT_ID and u["owner_slot"] == 0]
    return ids[0] if len(ids) == 1 else None


def trace_arrival(trace, unit, target_bits):
    for tick in range(1, TICKS + 1):
        record = trace.units.get((tick, unit))
        if record and (record["x_bits"], record["y_bits"]) == target_bits:
            return tick
    return None


def check_arrival(report, calc, error, old, new):
    if calc is None:
        report.add("ARRIVAL", False, error)
        return
    tick, text, bits = calc
    problems = []
    for side, traces in (("old", old), ("new", new)):
        trace = traces.get(CHANGED_PACKAGE)
        if trace is None:
            problems.append("%s acme: no trace given" % side)
            continue
        unit = scout_id(trace)
        if unit is None:
            problems.append("%s acme: no single slot-0 scout" % side)
            continue
        seen = trace_arrival(trace, unit, bits)
        if seen != tick:
            problems.append("%s acme: the scout arrives at tick %s" % (side, seen))
    if problems:
        report.add("ARRIVAL", False, text + "; " + "; ".join(problems))
    else:
        report.add("ARRIVAL", True, text + "; old and new traces arrive at %d"
                   % tick)


def match_in_order(old_ops, new_ops):
    """Find the old operations in the new list, in order. Return (pairs of
    (old, new), missing old operations, new operations that are not old)."""
    rest = []
    pairs = []
    position = 0
    missing = []
    for old in old_ops:
        while position < len(new_ops) and new_ops[position][1] != old[1]:
            rest.append(new_ops[position])
            position += 1
        if position == len(new_ops):
            missing.append(old)
        else:
            pairs.append((old, new_ops[position]))
            position += 1
    rest.extend(new_ops[position:])
    return pairs, missing, rest


def check_p3(report, old_stream, new_stream, old, new, arrival):
    """Every added or changed operation of the stream: in acme, of the scout,
    with no event kind, on the accept tick or on the tick after arrival.
    Expected: one added operation at tick 1 and one at the tick after
    arrival (design section 5 item 3)."""
    if old_stream is None or new_stream is None:
        report.add("P3", False, "no valid stream on both sides")
        return
    if arrival is None:
        report.add("P3", False, "no arrival tick from the calculation")
        return
    problems = []
    for package in UNCHANGED_PACKAGES:
        if old_stream[package] != new_stream[package]:
            tick = next(t for t in range(1, TICKS + 1)
                        if old_stream[package][t] != new_stream[package][t])
            problems.append("the operation stream of %s changed (first at tick "
                            "%d); only acme can change" % (package, tick))
    a, b = old.get(CHANGED_PACKAGE), new.get(CHANGED_PACKAGE)
    if a is None or b is None:
        problems.append("acme: no valid trace on both sides to find the unit "
                        "of each operation")
        report.add("P3", False, "; ".join(problems))
        return
    scout = scout_id(b)
    allowed_ticks = (ACCEPT_TICK, arrival + 1)
    previous = 0
    added = []
    added_on = {tick: 0 for tick in allowed_ticks}

    def keyed(stream_ops, trace):
        # The key of an operation: its unit and phase (from the trace of
        # the same run; STREAM-OLD and STREAM-NEW check that the stream is
        # that trace), its event kinds and its flag.
        out = []
        for op in stream_ops:
            record = trace.op_by_id.get(op["operation_id"])
            unit = record["unit"] if record else "unknown"
            phase = record["phase"] if record else "unknown"
            out.append((op, (unit, phase, tuple(op["event_kinds"]),
                             op["before_equals_after"])))
        return out

    for tick in range(1, TICKS + 1):
        ops = new_stream[CHANGED_PACKAGE][tick]
        for op in ops:
            if op["operation_id"] <= previous:
                problems.append("acme tick %d: operation %d after operation %d "
                                "(order)" % (tick, op["operation_id"], previous))
            previous = op["operation_id"]
        pairs, missing, rest = match_in_order(
            keyed(old_stream[CHANGED_PACKAGE][tick], a), keyed(ops, b))
        for (old_op, _), (new_op, _) in pairs:
            if old_op["operation_id"] != new_op["operation_id"]:
                problems.append("acme tick %d: old operation %d has id %d in "
                                "the new stream; an old operation keeps its "
                                "operation_id (an added operation cannot come "
                                "before it)" % (tick, old_op["operation_id"],
                                                new_op["operation_id"]))
        for op, key in missing:
            problems.append("acme tick %d: old operation %d (unit %s, phase %s, "
                            "kinds %s, before_equals_after %s) is changed or "
                            "gone" % ((tick, op["operation_id"], key[0], key[1],
                                       list(key[2]), key[3])))
        for op, key in rest:
            reasons = []
            if key[0] != scout:
                reasons.append("unit %s is not the scout %s" % (key[0], scout))
            if key[2]:
                reasons.append("event kinds %s" % list(key[2]))
            if tick not in allowed_ticks:
                reasons.append("not on the accept tick %d or the tick after "
                               "arrival %d" % allowed_ticks)
            else:
                added_on[tick] += 1
            if reasons:
                problems.append("acme tick %d: new operation %d: %s" % (
                    tick, op["operation_id"], ", ".join(reasons)))
            else:
                added.append("tick %d op %d" % (tick, op["operation_id"]))
    for tick in allowed_ticks:
        if added_on[tick] != 1:
            problems.append("acme tick %d: %d added operations; exactly one is "
                            "expected on tick %d and one on the tick after "
                            "arrival %d" % (tick, added_on[tick], ACCEPT_TICK,
                                            arrival + 1))
    if problems:
        report.add("P3", False, "; ".join(problems[:12]))
    else:
        report.add("P3", True, "base, haul, cedar streams equal; acme added "
                   "scout operations: %s" % ", ".join(added))


def check_old_hash(report, path, expected):
    if path is None or not Path(path).is_file():
        report.add("OLD-HASH", False, "no old stream given")
        return
    value = sha256_file(path)
    report.add("OLD-HASH", value == expected,
               "old stream sha256 %s, expected %s" % (value, expected))


def check_fixture(report, fixture, new_stream_path, stage, old_sha256):
    if stage not in STAGES:
        report.add("FIXTURE", False, "no stage given (--stage %s)" % "|".join(
            STAGES))
        return
    if fixture is None or not Path(fixture).is_file():
        report.add("FIXTURE", False, "no fixture given")
        return
    if new_stream_path is None or not Path(new_stream_path).is_file():
        report.add("FIXTURE", False, "no new stream given")
        return
    if Path(fixture).resolve() == Path(new_stream_path).resolve():
        report.add("FIXTURE", False, "the fixture and the new stream are one "
                   "file (%s); FIXTURE cannot fail on one file" % fixture)
        return
    a, b = sha256_file(fixture), sha256_file(new_stream_path)
    report.hashes.append(("fixture", a, fixture))
    if stage == "before-fixture-commit":
        if a != old_sha256:
            report.add("FIXTURE", False, "%s: the fixture %s has sha256 %s, not "
                       "the frozen %s; the builder must not touch the fixture"
                       % (stage, fixture, a, old_sha256))
        else:
            report.add("FIXTURE", True, "%s: the fixture still has the frozen "
                       "sha256 %s; the classified new stream is %s" % (
                           stage, a, b))
        return
    if a != b:
        report.add("FIXTURE", False, "%s: the fixture %s (%s) does not hash to "
                   "the classified new stream (%s)" % (stage, fixture, a, b))
    else:
        report.add("FIXTURE", True, "%s: the fixture hashes to the classified "
                   "new stream: %s" % (stage, a))


def run_p0(old_dir, new_dir, stream, emitter, old_sha256=FROZEN_SHA256,
           g1_in_a2c=False):
    report = Report()
    old = load_traces(old_dir, report, "COUNTS-OLD", g1_in_a2c)
    new = load_traces(new_dir, report, "COUNTS-NEW", g1_in_a2c)
    check_emitter(report, old, new, emitter)
    check_old_hash(report, stream, old_sha256)
    load_stream(report, stream, "old stream", old, "STREAM-OLD")
    load_stream(report, stream, "new stream", new, "STREAM-NEW")
    check_p0(report, old, new, PACKAGES)
    return report


def run_a2c(old_dir, new_dir, old_stream, new_stream, allowed, fixture,
            emitter, stage, old_sha256=FROZEN_SHA256, repo=ROOT,
            g1_in_a2c=False):
    report = Report()
    calc, calc_error = calculate_arrival(repo)
    arrival = calc[0] if calc else None
    allowed_list = load_allowed(report, allowed, arrival, g1_in_a2c)
    old = load_traces(old_dir, report, "COUNTS-OLD", g1_in_a2c)
    new = load_traces(new_dir, report, "COUNTS-NEW", g1_in_a2c)
    check_emitter(report, old, new, emitter)
    check_old_hash(report, old_stream, old_sha256)
    old_s = load_stream(report, old_stream, "old stream", old, "STREAM-OLD")
    new_s = load_stream(report, new_stream, "new stream", new, "STREAM-NEW")
    check_p0(report, old, new, UNCHANGED_PACKAGES)
    check_p1(report, old, new, allowed_list)
    check_p2(report, old, new)
    check_arrival(report, calc, calc_error, old, new)
    check_p3(report, old_s, new_s, old, new, arrival)
    check_fixture(report, fixture, new_stream, stage, old_sha256)
    return report


def self_test():
    spec = importlib.util.spec_from_file_location(
        "test_refreeze_classify",
        Path(__file__).with_name("test_refreeze_classify.py"))
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    spec.loader.exec_module(module)
    rows, clean_ok, clean = module.run_plants(sys.modules[__name__])
    red = sum(1 for row in rows if row[2])
    for name, expected, is_red, failed in rows:
        print("plant %-60s expect %-10s %s failed=%s" % (
            name, expected, "RED" if is_red else "NOT-RED", ",".join(failed)))
    for name, report in clean:
        if not report.passed:
            print("clean pair %s:\n%s" % (name, "\n".join(report.lines())))
    print("self-test: %d/%d plants red; clean pairs %s" % (
        red, len(rows), "green" if clean_ok else "red"))
    return 0 if red == len(rows) and clean_ok else 1


def main(argv):
    parser = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument("--self-test", action="store_true")
    mode.add_argument("--p0", nargs=2, metavar=("OLD_DIR", "NEW_DIR"))
    mode.add_argument("--a2c", action="store_true")
    parser.add_argument("--stage", choices=STAGES)
    parser.add_argument("--stream", help="--p0: the frozen stream")
    parser.add_argument("--old-trace")
    parser.add_argument("--new-trace")
    parser.add_argument("--old-stream")
    parser.add_argument("--new-stream")
    parser.add_argument("--allowed")
    parser.add_argument("--fixture")
    parser.add_argument("--emitter", nargs="+", default=[])
    parser.add_argument("--old-sha256", default=FROZEN_SHA256)
    parser.add_argument("--g1-in-a2c", action="store_true")
    args = parser.parse_args(argv)
    if args.self_test:
        return self_test()
    if args.p0:
        report = run_p0(args.p0[0], args.p0[1], args.stream, args.emitter,
                        old_sha256=args.old_sha256, g1_in_a2c=args.g1_in_a2c)
    else:
        report = run_a2c(args.old_trace, args.new_trace, args.old_stream,
                         args.new_stream, args.allowed, args.fixture,
                         args.emitter, args.stage, old_sha256=args.old_sha256,
                         g1_in_a2c=args.g1_in_a2c)
    print("\n".join(report.lines()))
    return 0 if report.passed else 1


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
