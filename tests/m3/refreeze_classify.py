#!/usr/bin/env python3
"""Re-freeze classifier for the one change of the M2 operation stream.

Pre-MP1 slice, task Q1a. Design: docs/superpowers/specs/
2026-11-12-pre-mp1-slice-design.md, section 5 and section 9 items 6, 7
and 14 (revision 5).
Python 3 standard library only.

The classifier reads two runs: "old" (the build before the change) and
"new" (the build after the change). For each run it reads one trace for each
package and the operation stream. It prints one line for each check, the
sha256 of each file that it read, and "RESULT PASS" (exit 0) or
"RESULT FAIL" (exit 1). A check that has no input gives FAIL, never PASS.

Modes:
    --p0 OLD_DIR NEW_DIR
        A2a and A2b: COUNTS, EMITTER and P0 on all four packages.
    --a2c --old-trace OLD_DIR --new-trace NEW_DIR --old-stream F
          --new-stream F --allowed F [--fixture F] [--old-sha256 H]
        A2c: all checks below.
    --self-test
        Run the plants of tests/m3/test_refreeze_classify.py.

TRACE FORMAT "turts-refreeze-trace/1" (task Q1b writes the emitter)
-------------------------------------------------------------------
One file for each package: <dir>/<package>.jsonl, for base, haul, acme and
cedar. UTF-8. One JSON object on each line. Each line ends with "\\n". No
blank line. The order of keys in an object is free; the emitter must give
the same bytes for the same state (P0 compares bytes).

Line 1, the header:
    {"record": "header", "format": "turts-refreeze-trace/1",
     "package": P, "emitter_sha256": H, "ticks": 400, "unit_count": N,
     "units": [{"unit": U, "content_id": C, "owner_slot": S}, ...]}
    - units: the units that exist after load_package and before the first
      command, in ascending unit id. unit_count is the length of units.
    - emitter_sha256: 64 lowercase hex digits; the sha256 of the emitter.
    - The header holds nothing that changes from one run to the next
      (no commit, no time), because P0 compares the bytes.

Then for each tick T = 1 to 400, in this order:
    1. One unit record for each unit that exists after the update of tick
       T, in ascending unit id. Each initial unit has a record at each tick.
       {"record": "unit", "package": P, "tick": T, "unit": U,
        "x_bits": X, "y_bits": Y, "target": null | {"x_bits": X,
        "y_bits": Y}, "activity": A, "state": S, "separated": B,
        "was_projected": B [, "edge_hit": B]}
       - x_bits, y_bits: f32::to_bits of the position, 8 lowercase hex.
       - target: the MoveTarget point as bits, or null when there is none.
       - activity: the MovementActivity of the MoveProgress of the unit
         ("Pending", "Moving", "Resting", "Blocked"), or "none" when the
         unit has no MoveProgress. Not the order or the gather stage
         (design section 9 item 7).
       - state: "pool" when the unit is in a movement pool, else "none".
       - separated: the separated flag of the movement step (bool).
       - was_projected: the was_projected flag of the movement step (bool;
         crates/turts_sim/src/movement.rs:2008 at 94a50d3). The fallback of
         an engine trip starts on "separated || was_projected" (:2052).
         Design revision 5, section 9 item 14.
       - edge_hit: only when G1 lands in A2c (design section 5 item 0); it
         is in all unit records of a file or in none.
    2. One operation record for each operation that the stream puts at
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

ALLOWED LIST FORMAT "turts-refreeze-allowed/1" (tests/m3/refreeze_allowed.json)
--------------------------------------------------------------------------------
Task Q1b writes the list before the A2c commit (plan refinement R6). It
names each acme trace difference that the design predicts. Every entry must
occur; a stale entry is a FAIL.
    {"schema": "turts-refreeze-allowed/1", "package": "acme",
     "note": "free text (optional)",
     "unit_fields": [
        {"unit": U, "field": F, "ticks": [FIRST, LAST], "old": V,
         "new": V}],
     "operations": [
        {"change": "added", "tick": T, "index": I, "unit": U | null,
         "phase": P, "events": [...], "before_equals_after": B}]}
    - field is one of target, activity, state, separated, was_projected,
      edge_hit. A
      position field (x_bits, y_bits) cannot be allowed: the ruling P1 is
      "every position is bitwise equal over 400 ticks"
      (docs/research/mp1/ruling-td-one-step-function.md, section 5).
    - A unit_fields entry covers each tick from FIRST to LAST. At each of
      these ticks the old value must be "old" and the new value "new".
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
TRACE_FORMAT = "turts-refreeze-trace/1"
ALLOWED_SCHEMA = "turts-refreeze-allowed/1"
# The frozen stream before the one re-freeze (tests/m3/refutations.py:18).
FROZEN_SHA256 = "bcbf72c2206251f635a7c7f5b6c8f573d69a5fc05a5bd77716e98ebfb08e370b"
# m3_m2_stream.rs:258-288 applies MoveTo before the update of tick 1, and
# :53-56 puts those operations in tick 1.
ACCEPT_TICK = 1
SCOUT_CONTENT_ID = "acme:scout"
ACTIVITIES = {"Pending", "Moving", "Resting", "Blocked", "none"}
STATES = {"none", "pool"}
PHASES = {"Command", "Orders", "PostMovementOrders", "Other"}
ALLOWED_UNIT_FIELDS = {"target", "activity", "state", "separated",
                       "was_projected", "edge_hit"}
UNIT_KEYS = {"record", "package", "tick", "unit", "x_bits", "y_bits",
             "target", "activity", "state", "separated", "was_projected"}
OP_KEYS = {"record", "package", "tick", "op_id", "unit", "phase", "events",
           "before_equals_after"}
HEADER_KEYS = {"record", "format", "package", "emitter_sha256", "ticks",
               "unit_count", "units"}
COMPARED_UNIT_FIELDS = ("x_bits", "y_bits", "target", "activity", "state",
                        "separated", "was_projected", "edge_hit")
HEX8 = re.compile(r"[0-9a-f]{8}\Z")
HEX64 = re.compile(r"[0-9a-f]{64}\Z")


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
        self.ops = []
        self.ops_by_tick = {tick: [] for tick in range(1, TICKS + 1)}
        self.op_by_id = {}

    def unit_ids_at(self, tick):
        return sorted(u for (t, u) in self.units if t == tick)


def check_target(value):
    if value is None:
        return True
    return (isinstance(value, dict) and set(value) == {"x_bits", "y_bits"}
            and all(isinstance(value[k], str) and HEX8.match(value[k])
                    for k in value))


def load_trace(directory, package):
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
    lines = raw.split(b"\n")[:-1]
    records = []
    for number, line in enumerate(lines, 1):
        try:
            record = json.loads(line)
        except ValueError as error:
            return None, "%s line %d: not JSON (%s)" % (package, number, error)
        if not isinstance(record, dict):
            return None, "%s line %d: not an object" % (package, number)
        records.append(record)
    head = records[0]
    if head.get("record") != "header" or set(head) != HEADER_KEYS:
        return None, "%s line 1: not a header with keys %s" % (
            package, sorted(HEADER_KEYS))
    if head["format"] != TRACE_FORMAT:
        return None, "%s: format %r, expected %r" % (
            package, head["format"], TRACE_FORMAT)
    if head["package"] != package:
        return None, "%s: header package %r" % (package, head["package"])
    if not (isinstance(head["emitter_sha256"], str)
            and HEX64.match(head["emitter_sha256"])):
        return None, "%s: emitter_sha256 is not 64 lowercase hex" % package
    if head["ticks"] != TICKS:
        return None, "%s: header ticks %r, expected %d" % (
            package, head["ticks"], TICKS)
    units = head["units"]
    if not isinstance(units, list) or not units:
        return None, "%s: header has no initial units" % package
    for unit in units:
        if (not isinstance(unit, dict)
                or set(unit) != {"unit", "content_id", "owner_slot"}
                or not is_int(unit["unit"])
                or not isinstance(unit["content_id"], str)
                or not is_int(unit["owner_slot"])):
            return None, "%s: bad header unit %r" % (package, unit)
    ids = [u["unit"] for u in units]
    if ids != sorted(set(ids)):
        return None, "%s: header unit ids are not ascending" % package
    if head["unit_count"] != len(units):
        return None, "%s: unit_count %r, but %d units" % (
            package, head["unit_count"], len(units))

    trace = Trace(package, raw, head)
    initial = set(trace.initial)
    tick = 0
    last_unit = None
    in_ops = False
    seen_initial = set()
    edge_hit = None
    for number, record in enumerate(records[1:], 2):
        where = "%s line %d" % (package, number)
        kind = record.get("record")
        if record.get("package") != package:
            return None, "%s: package %r" % (where, record.get("package"))
        rtick = record.get("tick")
        if not is_int(rtick):
            return None, "%s: tick is not an integer" % where
        if kind == "unit":
            keys = set(record)
            if keys - {"edge_hit"} != UNIT_KEYS:
                return None, "%s: unit keys %s" % (where, sorted(keys))
            has_edge = "edge_hit" in record
            if edge_hit is None:
                edge_hit = has_edge
            elif edge_hit != has_edge:
                return None, "%s: edge_hit in some unit records only" % where
            if has_edge and not isinstance(record["edge_hit"], bool):
                return None, "%s: edge_hit is not a bool" % where
            if rtick != tick:
                if rtick != tick + 1:
                    return None, "%s: tick %d after tick %d" % (
                        where, rtick, tick)
                if tick and seen_initial != initial:
                    return None, "%s tick %d: initial units %s have no record" % (
                        package, tick, sorted(initial - seen_initial))
                tick = rtick
                last_unit = None
                in_ops = False
                seen_initial = set()
            elif in_ops:
                return None, "%s: unit record after an op record" % where
            unit = record["unit"]
            if not is_int(unit) or (last_unit is not None and unit <= last_unit):
                return None, "%s: unit ids are not ascending" % where
            last_unit = unit
            if unit in initial:
                seen_initial.add(unit)
            for field in ("x_bits", "y_bits"):
                if not (isinstance(record[field], str)
                        and HEX8.match(record[field])):
                    return None, "%s: %s is not 8 lowercase hex" % (where, field)
            if not check_target(record["target"]):
                return None, "%s: bad target %r" % (where, record["target"])
            if record["activity"] not in ACTIVITIES:
                return None, "%s: activity %r" % (where, record["activity"])
            if record["state"] not in STATES:
                return None, "%s: state %r" % (where, record["state"])
            for field in ("separated", "was_projected"):
                if not isinstance(record[field], bool):
                    return None, "%s: %s is not a bool" % (where, field)
            trace.units[(tick, unit)] = record
        elif kind == "op":
            if set(record) != OP_KEYS:
                return None, "%s: op keys %s" % (where, sorted(record))
            if rtick != tick or tick == 0:
                return None, "%s: op of tick %d after unit records of tick %d" % (
                    where, rtick, tick)
            in_ops = True
            if record["op_id"] != len(trace.ops) + 1:
                return None, "%s: op_id %r, expected %d (ids 1, 2, 3, ...)" % (
                    where, record["op_id"], len(trace.ops) + 1)
            if record["unit"] is not None and not is_int(record["unit"]):
                return None, "%s: op unit %r" % (where, record["unit"])
            if record["phase"] not in PHASES:
                return None, "%s: phase %r" % (where, record["phase"])
            if not isinstance(record["before_equals_after"], bool):
                return None, "%s: before_equals_after is not a bool" % where
            if not isinstance(record["events"], list):
                return None, "%s: events is not a list" % where
            try:
                for value in record["events"]:
                    event_kind(value)
            except ValueError as error:
                return None, "%s: %s" % (where, error)
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
    if count != TICKS * len(initial):
        return None, "%s: %d initial-unit records, expected %d x %d" % (
            package, count, TICKS, len(initial))
    return trace, None


def load_traces(directory, packages, report, check):
    traces = {}
    errors = []
    details = []
    for package in packages:
        trace, error = load_trace(directory, package)
        if error:
            errors.append(error)
            continue
        traces[package] = trace
        report.hashes.append((check.lower() + "-" + package,
                              hashlib.sha256(trace.raw).hexdigest(),
                              Path(directory) / (package + ".jsonl")))
        initial = len(trace.initial)
        details.append("%s %d ticks x %d initial units = %d, %d unit records, "
                       "%d ops" % (package, TICKS, initial, TICKS * initial,
                                   len(trace.units), len(trace.ops)))
    if errors:
        report.add(check, False, "; ".join(errors))
    else:
        report.add(check, True, "; ".join(details))
    return traces


def check_emitter(report, old, new, packages):
    missing = [side + "/" + p for side, traces in (("old", old), ("new", new))
               for p in packages if p not in traces]
    if missing:
        report.add("EMITTER", False, "no valid trace for " + ", ".join(missing))
        return
    values = {side + "/" + p: traces[p].header["emitter_sha256"]
              for side, traces in (("old", old), ("new", new)) for p in packages}
    if len(set(values.values())) != 1:
        report.add("EMITTER", False, "emitter sha256 values differ: " + ", ".join(
            "%s=%s" % item for item in sorted(values.items())))
        return
    report.add("EMITTER", True, "all %d traces: %s" % (
        len(values), next(iter(values.values()))))


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


def load_allowed(report, path):
    """Return the allowed list, or None (ALLOWED is then FAIL)."""
    data, error = load_json(path, "allowed list")
    if error:
        report.add("ALLOWED", False, error)
        return None
    report.hashes.append(("allowed", sha256_file(path), path))
    problems = []
    if not isinstance(data, dict):
        report.add("ALLOWED", False, "the allowed list is not an object")
        return None
    extra = set(data) - {"schema", "package", "note", "unit_fields",
                         "operations"}
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
    ops = data.get("operations")
    if not isinstance(fields, list) or not isinstance(ops, list):
        problems.append("unit_fields and operations must be lists")
        fields, ops = [], []
    spans = {}
    for index, entry in enumerate(fields):
        where = "unit_fields[%d]" % index
        if not isinstance(entry, dict) or set(entry) != {
                "unit", "field", "ticks", "old", "new"}:
            problems.append("%s: keys must be unit, field, ticks, old, new" % where)
            continue
        if not is_int(entry["unit"]):
            problems.append("%s: unit is not an integer" % where)
        if entry["field"] not in ALLOWED_UNIT_FIELDS:
            problems.append("%s: field %r cannot be allowed (allowed: %s)" % (
                where, entry["field"], ", ".join(sorted(ALLOWED_UNIT_FIELDS))))
        ticks = entry["ticks"]
        if not (isinstance(ticks, list) and len(ticks) == 2
                and all(is_int(t) for t in ticks)
                and 1 <= ticks[0] <= ticks[1] <= TICKS):
            problems.append("%s: ticks must be [first, last] in 1..400" % where)
            continue
        if entry["old"] == entry["new"]:
            problems.append("%s: old equals new" % where)
        key = (entry["unit"], entry["field"])
        for first, last in spans.get(key, []):
            if ticks[0] <= last and first <= ticks[1]:
                problems.append("%s: ticks overlap another entry" % where)
        spans.setdefault(key, []).append(tuple(ticks))
    places = set()
    for index, entry in enumerate(ops):
        where = "operations[%d]" % index
        if not isinstance(entry, dict):
            problems.append("%s: not an object" % where)
            continue
        change = entry.get("change")
        keys = {"change", "tick", "index", "unit", "phase", "events",
                "before_equals_after"}
        if change != "added":
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
    report.add("ALLOWED", True, "%d unit field entries, %d operation entries" % (
        len(fields), len(ops)))
    return data


def op_body(record):
    return {k: record[k] for k in ("tick", "unit", "phase", "events",
                                   "before_equals_after")}


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
                differences[key + (field,)] = (x.get(field), y.get(field))
    used = set()
    for index, entry in enumerate(allowed["unit_fields"]):
        first, last = entry["ticks"]
        for tick in range(first, last + 1):
            key = (tick, entry["unit"], entry["field"])
            seen = differences.get(key)
            if seen != (entry["old"], entry["new"]):
                problems.append(
                    "allowed unit_fields[%d] does not occur at tick %d "
                    "(observed %r)" % (index, tick, seen))
                break
            used.add(key)
    for key in sorted(set(differences) - used):
        problems.append("acme tick %d unit %d field %s: %r -> %r is not in "
                        "the allowed list" % (key + differences[key]))
        if len(problems) > 12:
            problems.append("(more)")
            break

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


def load_stream(report, path, label, traces, check):
    data, error = load_json(path, label)
    if error:
        report.add(check, False, error)
        return None
    report.hashes.append((label.replace(" ", "-"), sha256_file(path), path))
    try:
        stream = stream_structure(data)
    except ValueError as error:
        report.add(check, False, "structure: %s" % error)
        return None
    counts = ", ".join("%s %d ops" % (p, sum(len(v) for v in stream[p].values()))
                       for p in PACKAGES)
    problems = []
    for package in PACKAGES:
        trace = traces.get(package)
        if trace is None:
            problems.append("%s: no valid trace to compare" % package)
            continue
        for tick in range(1, TICKS + 1):
            projected = [{
                "operation_id": r["op_id"],
                "event_kinds": [event_kind(e) for e in r["events"]],
                "before_equals_after": r["before_equals_after"],
            } for r in trace.ops_by_tick[tick]]
            if stream[package][tick] != projected:
                problems.append("%s tick %d: the stream is not the trace of the "
                                "same run" % (package, tick))
                break
    if problems:
        report.add(check, False, "structure ok (%s); trace: %s" % (
            counts, "; ".join(problems)))
    else:
        report.add(check, True, "structure ok (%s); equal to the trace" % counts)
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


def calculated_arrival(report, repo):
    """Return (arrival tick, target bits) or (None, None)."""
    try:
        inputs = arrival_inputs(repo)
    except (OSError, ValueError) as error:
        report.add("ARRIVAL", False, "inputs: %s" % error)
        return None, None
    (x0, y0), (x1, y1) = inputs["start"], inputs["target"]
    if not (0 <= x1 <= inputs["size"][0] and 0 <= y1 <= inputs["size"][1]):
        report.add("ARRIVAL", False, "the target is off the map; the "
                   "calculation does not hold")
        return None, None
    step = inputs["speed"] / inputs["tick_hz"]
    ratio = math.hypot(x1 - x0, y1 - y0) / step
    if abs(ratio - round(ratio)) < 1e-3:
        report.add("ARRIVAL", False, "distance / step = %.6f is within 0.001 "
                   "of a whole number; f32 rounding decides (needs decision)"
                   % ratio)
        return None, None
    calc = math.ceil(ratio)
    return (calc, inputs, ratio), (f32_bits(x1), f32_bits(y1))


def check_arrival(report, repo, old, new):
    calc, bits = calculated_arrival(report, repo)
    if calc is None:
        return None
    tick, inputs, ratio = calc
    text = ("ceil(|(%g, %g) - (%g, %g)| / (%g x 1/%d)) = ceil(%.4f) = %d" % (
        inputs["target"][0], inputs["target"][1], inputs["start"][0],
        inputs["start"][1], inputs["speed"], inputs["tick_hz"], ratio, tick))
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
    return tick


def match_in_order(old_ops, new_ops):
    """Find the old operations in the new list, in order. Return (missing
    old operations, new operations that are not old ones)."""
    rest = []
    position = 0
    missing = []
    for old in old_ops:
        while position < len(new_ops) and new_ops[position][1] != old[1]:
            rest.append(new_ops[position])
            position += 1
        if position == len(new_ops):
            missing.append(old)
        else:
            position += 1
    rest.extend(new_ops[position:])
    return missing, rest


def check_p3(report, old_stream, new_stream, old, new, arrival):
    """Every added or changed operation of the stream: in acme, of the scout,
    with no event kind, on the accept tick or on the tick after arrival."""
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
    for tick in range(1, TICKS + 1):
        ops = new_stream[CHANGED_PACKAGE][tick]
        for op in ops:
            if op["operation_id"] <= previous:
                problems.append("acme tick %d: operation %d after operation %d "
                                "(order)" % (tick, op["operation_id"], previous))
            previous = op["operation_id"]

        def keyed(stream_ops, trace):
            # The key of an operation: its unit and phase (from the trace of
            # the same run; STREAM-OLD and STREAM-NEW check that the stream
            # is that trace), its event kinds and its flag.
            out = []
            for op in stream_ops:
                record = trace.op_by_id.get(op["operation_id"])
                unit = record["unit"] if record else "unknown"
                phase = record["phase"] if record else "unknown"
                out.append((op, (unit, phase, tuple(op["event_kinds"]),
                                 op["before_equals_after"])))
            return out
        missing, rest = match_in_order(
            keyed(old_stream[CHANGED_PACKAGE][tick], a), keyed(ops, b))
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
            if reasons:
                problems.append("acme tick %d: new operation %d: %s" % (
                    tick, op["operation_id"], ", ".join(reasons)))
            else:
                added.append("tick %d op %d" % (tick, op["operation_id"]))
    if problems:
        report.add("P3", False, "; ".join(problems[:12]))
    else:
        report.add("P3", True, "base, haul, cedar streams equal; acme added "
                   "scout operations: %s" % (", ".join(added) or "none"))


def check_old_hash(report, path, expected):
    if path is None or not Path(path).is_file():
        report.add("OLD-HASH", False, "no old stream given")
        return
    value = sha256_file(path)
    report.add("OLD-HASH", value == expected,
               "old stream sha256 %s, expected %s" % (value, expected))


def check_fixture(report, fixture, new_stream_path):
    if fixture is None or not Path(fixture).is_file():
        report.add("FIXTURE", False, "no fixture given")
        return
    if new_stream_path is None or not Path(new_stream_path).is_file():
        report.add("FIXTURE", False, "no new stream given")
        return
    a, b = sha256_file(fixture), sha256_file(new_stream_path)
    report.hashes.append(("fixture", a, fixture))
    if a != b:
        report.add("FIXTURE", False, "the fixture %s (%s) does not hash to the "
                   "classified new stream (%s)" % (fixture, a, b))
    else:
        report.add("FIXTURE", True, "the fixture hashes to the classified new "
                   "stream: %s" % a)


def run_p0(old_dir, new_dir):
    report = Report()
    old = load_traces(old_dir, PACKAGES, report, "COUNTS-OLD")
    new = load_traces(new_dir, PACKAGES, report, "COUNTS-NEW")
    check_emitter(report, old, new, PACKAGES)
    check_p0(report, old, new, PACKAGES)
    return report


def run_a2c(old_dir, new_dir, old_stream, new_stream, allowed, fixture,
            old_sha256=FROZEN_SHA256, repo=ROOT):
    report = Report()
    allowed_list = load_allowed(report, allowed)
    old = load_traces(old_dir, PACKAGES, report, "COUNTS-OLD")
    new = load_traces(new_dir, PACKAGES, report, "COUNTS-NEW")
    check_emitter(report, old, new, PACKAGES)
    check_old_hash(report, old_stream, old_sha256)
    old_s = load_stream(report, old_stream, "old stream", old, "STREAM-OLD")
    new_s = load_stream(report, new_stream, "new stream", new, "STREAM-NEW")
    check_p0(report, old, new, UNCHANGED_PACKAGES)
    check_p1(report, old, new, allowed_list)
    check_p2(report, old, new)
    arrival = check_arrival(report, repo, old, new)
    check_p3(report, old_s, new_s, old, new, arrival)
    check_fixture(report, fixture, new_stream)
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
    if not clean_ok:
        print("\n".join(clean.lines()))
    print("self-test: %d/%d plants red; clean pair %s" % (
        red, len(rows), "green" if clean_ok else "red"))
    return 0 if red == len(rows) and clean_ok else 1


def main(argv):
    parser = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument("--self-test", action="store_true")
    mode.add_argument("--p0", nargs=2, metavar=("OLD_DIR", "NEW_DIR"))
    mode.add_argument("--a2c", action="store_true")
    parser.add_argument("--old-trace")
    parser.add_argument("--new-trace")
    parser.add_argument("--old-stream")
    parser.add_argument("--new-stream")
    parser.add_argument("--allowed")
    parser.add_argument("--fixture",
                        default=str(ROOT / "tests/m3/m2_operation_stream.json"))
    parser.add_argument("--old-sha256", default=FROZEN_SHA256)
    args = parser.parse_args(argv)
    if args.self_test:
        return self_test()
    if args.p0:
        report = run_p0(args.p0[0], args.p0[1])
    else:
        report = run_a2c(args.old_trace, args.new_trace, args.old_stream,
                         args.new_stream, args.allowed, args.fixture,
                         old_sha256=args.old_sha256)
    print("\n".join(report.lines()))
    return 0 if report.passed else 1


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
