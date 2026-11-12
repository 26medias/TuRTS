#!/usr/bin/env python3
"""Self-test of the re-freeze classifier (pre-MP1 slice, task Q1a).

Each plant changes one thing in a clean synthetic pair of runs. The
classifier must give FAIL, the named check must be one of the failed
checks, and its detail must hold the named text. The clean pairs must give
PASS. Python 3 standard library only.

Run: python3 tests/m3/test_refreeze_classify.py
Or:  python3 tests/m3/refreeze_classify.py --self-test
"""
import copy
import hashlib
import importlib.util
import json
from pathlib import Path
import struct
import sys
import tempfile
import unittest

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[1]

PACKAGES = ("base", "haul", "acme", "cedar")
TICKS = 400
EMITTER_BYTES = b"// synthetic trace emitter of the self-test\n"
EMITTER = hashlib.sha256(EMITTER_BYTES).hexdigest()
# The arrival tick of the acme scout, from the gate probe (design section 5
# item 3, "arrival 123, operation 124"). It is a constant here on purpose:
# the classifier calculates it from the package files, and the clean pair
# passes only when the two numbers agree.
ARRIVAL = 123
SCOUT = 1


def load_classifier():
    spec = importlib.util.spec_from_file_location(
        "refreeze_classify", HERE / "refreeze_classify.py")
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    spec.loader.exec_module(module)
    return module


def f32_bits(value):
    return "%08x" % struct.unpack("<I", struct.pack("<f", value))[0]


def flip_low_bit(bits):
    return "%08x" % (int(bits, 16) ^ 1)


def header(package, units):
    return {
        "record": "header",
        "format": "turts-refreeze-trace/4",
        "package": package,
        "emitter_sha256": EMITTER,
        "ticks": TICKS,
        "unit_count": len(units),
        "units": units,
    }


def world_record(package, tick, pools=0, pending=0):
    return {"record": "world", "package": package, "tick": tick,
            "pools": pools, "pending_movement_orders": pending}


def unit_record(package, tick, unit, x, y, target=None, activity="none",
                state="none", order="none", separated=False,
                was_projected=False, queue=None, last_failure=None,
                gather_stage="none"):
    return {
        "record": "unit",
        "package": package,
        "tick": tick,
        "unit": unit,
        "x_bits": f32_bits(x),
        "y_bits": f32_bits(y),
        "target": None if target is None else {
            "x_bits": f32_bits(target[0]), "y_bits": f32_bits(target[1])},
        "activity": activity,
        "state": state,
        "order": order,
        "gather_stage": gather_stage,
        "queue": [] if queue is None else queue,
        "last_failure": last_failure,
        "separated": separated,
        "was_projected": was_projected,
    }


def op_record(package, tick, op_id, unit, phase, events, before_equals_after,
              move_to_route=None):
    record = {
        "record": "op",
        "package": package,
        "tick": tick,
        "op_id": op_id,
        "unit": unit,
        "phase": phase,
        "events": events,
        "before_equals_after": before_equals_after,
    }
    if move_to_route is not None:
        record["move_to_route"] = move_to_route
    return record


def event(tick, ordinal, kind, payload):
    return {"tick": tick, "ordinal": ordinal, "owner_slot": 0,
            "kind": {kind: payload}}


def economy_trace(package):
    """A gather package: a worker that moves (with a PlainMoveProgress up to
    tick 79), a static building, ops with economy events. In `haul` a third
    unit appears at tick 100."""
    units = [
        {"unit": 1, "content_id": package + ":worker", "owner_slot": 0},
        {"unit": 2, "content_id": package + ":depot", "owner_slot": 0},
    ]
    records = [header(package, units)]
    op_id = 0
    ordinal = 0
    for tick in range(1, TICKS + 1):
        moving = tick < 80
        records.append(world_record(package, tick, pending=1 if moving else 0))
        x = 10.0 + 0.25 * min(tick, 80)
        records.append(unit_record(
            package, tick, 1, x, 12.0,
            target=(30.0, 12.0) if moving else None,
            state="plain" if moving else "none",
            order="Gather:MovingToNode" if moving else "Gather:Gathering",
            gather_stage="MovingToNode" if moving else "Gathering"))
        records.append(unit_record(package, tick, 2, 40.0, 40.0))
        if package == "haul" and tick >= 100:
            records.append(unit_record(package, tick, 3, 41.0, 40.0))
        ops = []
        if tick == 1:
            ops.append((1, "Command", [], False))
        if tick % 10 == 0:
            ordinal += 1
            ops.append((1, "Other", [event(tick, ordinal, "Extracted", {
                "node": 7, "cargo": 1, "item": package + ":ore",
                "amount": 5})], False))
        if tick % 25 == 0:
            ordinal += 1
            ops.append((1, "Other", [event(tick, ordinal, "Stored", {
                "from": 1, "to": 2, "item": package + ":ore",
                "amount": 5})], False))
        if tick % 3 == 0:
            ops.append((None, "Orders", [], True))
        for unit, phase, events, flag in ops:
            op_id += 1
            records.append(op_record(package, tick, op_id, unit, phase,
                                     events, flag))
    return records


def scout_position(tick):
    """The scout goes in a line from (4, 4) to (30, 30) at 0.3 per tick and
    rests on (30, 30) from the arrival tick."""
    if tick >= ARRIVAL:
        return 30.0, 30.0
    dx, dy = 26.0, 26.0
    length = (dx * dx + dy * dy) ** 0.5
    travelled = 0.3 * tick
    return 4.0 + dx / length * travelled, 4.0 + dy / length * travelled


def acme_trace(new):
    """The old MoveTo resets UnitOrders to the default (orders.rs:633-638), so
    the old trace has order none on every tick. The new run keeps the order
    through the arrival tick, because the Orders phase that clears it runs
    before Movement in the next tick (design 9.24a)."""
    units = [{"unit": SCOUT, "content_id": "acme:scout", "owner_slot": 0}]
    records = [header("acme", units)]
    op_id = 0
    for tick in range(1, TICKS + 1):
        x, y = scout_position(tick)
        moving = tick < ARRIVAL
        records.append(world_record("acme", tick,
                                    pools=1 if new and moving else 0))
        records.append(unit_record(
            "acme", tick, SCOUT, x, y,
            target=(30.0, 30.0) if moving else None,
            activity="Moving" if new and moving else "none",
            state="pool" if new and moving else "none",
            order="Move:-" if new and tick <= ARRIVAL else "none",
            # Format /4: both branches store the real value, so the old and the
            # new run read false while the scout moves and null from the
            # arrival tick (a snap to the target runs no projection loop).
            was_projected=False if moving else None))
        ops = []
        if tick == 1:
            ops.append((SCOUT, "Command", [], False,
                        "envelope" if new else "direct"))
            if new:
                ops.append((SCOUT, "Orders", [], False, None))
        if new and tick == ARRIVAL + 1:
            ops.append((SCOUT, "PostMovementOrders", [], False, None))
        for unit, phase, events, flag, route in ops:
            op_id += 1
            records.append(op_record("acme", tick, op_id, unit, phase,
                                     events, flag, route))
    return records


def clean_allowed():
    return {
        "schema": "turts-refreeze-allowed/4",
        "package": "acme",
        "note": "synthetic list of the self-test",
        "unit_fields": [
            {"unit": SCOUT, "field": "activity", "ticks": [1, ARRIVAL - 1],
             "old": "none", "new": "Moving"},
            {"unit": SCOUT, "field": "state", "ticks": [1, ARRIVAL - 1],
             "old": "none", "new": "pool"},
            {"unit": SCOUT, "field": "order", "ticks": [1, ARRIVAL],
             "old": "none", "new": "Move:-"},
        ],
        "world_fields": [
            {"field": "pools", "ticks": [1, ARRIVAL - 1], "old": 0, "new": 1},
        ],
        "operations": [
            {"change": "added", "tick": 1, "index": 1, "unit": SCOUT,
             "phase": "Orders", "events": [], "before_equals_after": False},
            {"change": "added", "tick": ARRIVAL + 1, "index": 0,
             "unit": SCOUT, "phase": "PostMovementOrders", "events": [],
             "before_equals_after": False},
        ],
        "operation_fields": [
            {"tick": 1, "index": 0, "field": "move_to_route",
             "old": "direct", "new": "envelope"},
        ],
    }


def event_kind(value):
    kind = value["kind"]
    return kind if isinstance(kind, str) else next(iter(kind))


def project_stream(traces):
    """The stream that m3_m2_stream.rs would write for these traces."""
    out = []
    for package in PACKAGES:
        ticks = {tick: [] for tick in range(1, TICKS + 1)}
        for record in traces[package] or []:
            if record.get("record") == "op" and record["tick"] in ticks:
                ticks[record["tick"]].append({
                    "operation_id": record["op_id"],
                    "event_kinds": [event_kind(e) for e in record["events"]],
                    "before_equals_after": record["before_equals_after"],
                })
        out.append({"package": package, "ticks": [
            {"tick": tick, "operation_count": len(ops), "operations": ops}
            for tick, ops in ticks.items()]})
    return out


def stream_bytes(stream):
    # The same bytes as serde_json::to_string_pretty of the fixture plus a
    # newline (checked against tests/m3/m2_operation_stream.json).
    return (json.dumps(stream, indent=2, sort_keys=True) + "\n").encode()


def trace_bytes(records):
    return "".join(json.dumps(r, sort_keys=True, separators=(",", ":")) + "\n"
                   for r in records).encode()


class Case:
    """One synthetic pair of runs. A trace value of None means "no file";
    the value [] means "an empty file"."""

    def __init__(self):
        self.old = {p: economy_trace(p) for p in PACKAGES if p != "acme"}
        self.old["acme"] = acme_trace(new=False)
        self.new = {p: economy_trace(p) for p in PACKAGES if p != "acme"}
        self.new["acme"] = acme_trace(new=True)
        self.allowed = clean_allowed()
        self.new_stream_patch = None
        self.fixture_patch = None
        self.fixture_is_new_stream = False
        self.stage = "after-fixture-commit"
        self.emitter_bytes = EMITTER_BYTES
        self.mode = "a2c"
        self.p0_stream = True
        self.old_dir_given = True
        self.g1_in_a2c = False

    def to_counts(self):
        """--counts mode: the counts of one run. The synthetic gather worker
        has a plain progress (it exercises the P0 rules), so the clean counts
        case turns it into no progress, as in the real stream."""
        self.mode = "counts"
        for records in self.new.values():
            for record in records or []:
                if record.get("state") == "plain":
                    record["state"] = "none"

    def to_p0(self):
        """--p0 mode (A2a, A2b): the four packages, old against old."""
        self.mode = "p0"
        self.new = copy.deepcopy(self.old)

    def materialise(self, root):
        root = Path(root)
        paths = {}
        for side, traces in (("old", self.old), ("new", self.new)):
            directory = root / side
            directory.mkdir()
            for package, records in traces.items():
                if records is not None:
                    (directory / (package + ".jsonl")).write_bytes(
                        trace_bytes(records))
            paths[side] = directory
        old_stream = stream_bytes(project_stream(self.old))
        new = project_stream(self.new)
        if self.new_stream_patch is not None:
            self.new_stream_patch(new)
        new_stream = stream_bytes(new)
        if self.stage == "before-fixture-commit":
            fixture = old_stream
        else:
            fixture = new_stream
        if self.fixture_patch is not None:
            fixture = self.fixture_patch(new_stream, old_stream)
        (root / "old_stream.json").write_bytes(old_stream)
        (root / "new_stream.json").write_bytes(new_stream)
        (root / "fixture.json").write_bytes(fixture)
        (root / "allowed.json").write_text(json.dumps(self.allowed, indent=2))
        (root / "emitter.rs").write_bytes(self.emitter_bytes)
        if self.mode == "counts":
            return {"directory": paths["new"], "g1_in_a2c": self.g1_in_a2c}
        if self.mode == "p0":
            return {
                "old_dir": paths["old"] if self.old_dir_given else None,
                "new_dir": paths["new"],
                "stream": root / "old_stream.json" if self.p0_stream else None,
                "emitter": [root / "emitter.rs"],
                "old_sha256": hashlib.sha256(old_stream).hexdigest(),
                "g1_in_a2c": self.g1_in_a2c,
            }
        return {
            "old_dir": paths["old"] if self.old_dir_given else None,
            "new_dir": paths["new"],
            "old_stream": root / "old_stream.json",
            "new_stream": root / "new_stream.json",
            "allowed": root / "allowed.json",
            "fixture": root / ("new_stream.json" if self.fixture_is_new_stream
                               else "fixture.json"),
            "emitter": [root / "emitter.rs"],
            "stage": self.stage,
            "old_sha256": hashlib.sha256(old_stream).hexdigest(),
            "repo": REPO,
            "g1_in_a2c": self.g1_in_a2c,
        }


def find(records, **keys):
    for record in records:
        if all(record.get(k) == v for k, v in keys.items()):
            return record
    raise KeyError(keys)


def find_stream_op(stream, package, tick, index):
    entry = next(s for s in stream if s["package"] == package)
    return entry["ticks"][tick - 1]["operations"][index]


def renumber_ops(records):
    number = 0
    for record in records:
        if record.get("record") == "op":
            number += 1
            record["op_id"] = number


def scout_units(case, first, last):
    return [find(case.new["acme"], record="unit", tick=t, unit=SCOUT)
            for t in range(first, last + 1)]


# Each plant changes one Case. The PLANTS table names the expected check.

def plant_flip_position_bit_base(case):
    record = find(case.new["base"], record="unit", tick=200, unit=1)
    record["x_bits"] = flip_low_bit(record["x_bits"])


def plant_flip_position_bit_acme(case):
    record = find(case.new["acme"], record="unit", tick=50, unit=SCOUT)
    record["y_bits"] = flip_low_bit(record["y_bits"])


def plant_added_stored_event(case):
    # A conserving extra Stored event in haul, in the trace and the stream.
    record = find(case.new["haul"], record="op", tick=30)
    record["events"].append(event(30, 999, "Stored", {
        "from": 1, "to": 2, "item": "haul:ore", "amount": 0}))


def plant_changed_operation_base(case):
    record = find(case.new["base"], record="op", tick=9)
    record["before_equals_after"] = not record["before_equals_after"]


def plant_scout_op_on_arrival_tick(case):
    # The added op falls on the arrival tick itself. The allowed list is
    # changed to agree, so only P3 can see it.
    records = case.new["acme"]
    record = find(records, record="op", phase="PostMovementOrders")
    records.remove(record)
    record["tick"] = ARRIVAL
    unit = records.index(find(records, record="unit", tick=ARRIVAL))
    records.insert(unit + 1, record)
    case.allowed["operations"][1]["tick"] = ARRIVAL


def plant_reorder_acme_ops_with_ids(case):
    # Swap the two tick-1 operations, ids and all, in the new stream. The
    # trace stays valid, so P3 must see the order itself.
    def patch(stream):
        ops = next(s for s in stream if s["package"] == "acme")["ticks"][0][
            "operations"]
        ops[0], ops[1] = ops[1], ops[0]
    case.new_stream_patch = patch


def plant_reorder_acme_ops_keep_ids(case):
    # Swap the content of the two tick-1 operations, but keep the ids in
    # order. The stream is not changed: both have no event and flag false.
    one = find(case.new["acme"], record="op", op_id=1)
    two = find(case.new["acme"], record="op", op_id=2)
    one["phase"], two["phase"] = two["phase"], one["phase"]


def plant_flip_flag_tick1_op(case):
    # The accepted MoveTo operation shows before == after, in the trace and
    # in the stream (a run that agrees with itself).
    record = find(case.new["acme"], record="op", op_id=1)
    record["before_equals_after"] = True


def plant_flip_flag_tick1_stream_only(case):
    def patch(stream):
        op = find_stream_op(stream, "acme", 1, 0)
        op["before_equals_after"] = not op["before_equals_after"]
    case.new_stream_patch = patch


def plant_empty_traces_base(case):
    case.old["base"] = []
    case.new["base"] = []


def plant_empty_trace_new_acme(case):
    case.new["acme"] = []


def plant_399_ticks(case):
    for side in (case.old, case.new):
        side["cedar"] = [r for r in side["cedar"] if r.get("tick") != 400]


def plant_changed_payload_same_kind(case):
    record = find(case.new["cedar"], record="op", tick=40)
    record["events"][0]["kind"]["Extracted"]["amount"] += 1


def plant_other_emitter_file(case):
    # The new run used another emitter: its headers say so, and they
    # differ from the old headers.
    case.emitter_bytes = b"// another emitter\n"
    other = hashlib.sha256(case.emitter_bytes).hexdigest()
    for records in case.new.values():
        records[0]["emitter_sha256"] = other


def plant_emitter_not_the_file_hash(case):
    # Reviewer plant n: all 8 headers declare one constant that is not the
    # sha256 of the emitter file.
    for side in (case.old, case.new):
        for records in side.values():
            records[0]["emitter_sha256"] = "ab" * 32


def plant_fixture_not_classified_stream(case):
    case.fixture_patch = lambda new_stream, old_stream: old_stream


def plant_fixture_is_the_new_stream_path(case):
    case.fixture_is_new_stream = True


def plant_fixture_touched_before_commit(case):
    case.stage = "before-fixture-commit"
    case.fixture_patch = lambda new_stream, old_stream: new_stream


def plant_acme_field_not_allowed(case):
    record = find(case.new["acme"], record="unit", tick=10, unit=SCOUT)
    record["separated"] = True


def plant_flip_was_projected_acme(case):
    record = find(case.new["acme"], record="unit", tick=20, unit=SCOUT)
    record["was_projected"] = True


def plant_flip_was_projected_haul(case):
    record = find(case.new["haul"], record="unit", tick=60, unit=1)
    record["was_projected"] = True


def plant_base_state_plain_to_none(case):
    # The base worker loses its PlainMoveProgress one tick early.
    find(case.new["base"], record="unit", tick=79, unit=1)["state"] = "none"


def plant_base_pending_orders(case):
    find(case.new["base"], record="world", tick=150)[
        "pending_movement_orders"] = 1


def plant_pool_kept_to_400_listed(case):
    # Reviewer plant c2: a leaked pool after arrival, and the list predicts
    # it.
    for record in scout_units(case, ARRIVAL, TICKS):
        record["activity"] = "Resting"
        record["state"] = "pool"
    case.allowed["unit_fields"] += [
        {"unit": SCOUT, "field": "activity", "ticks": [ARRIVAL, TICKS],
         "old": "none", "new": "Resting"},
        {"unit": SCOUT, "field": "state", "ticks": [ARRIVAL, TICKS],
         "old": "none", "new": "pool"}]


def plant_move_target_kept_listed(case):
    # Reviewer plant c3: the MoveTarget is not cleared after arrival, and
    # the list predicts it.
    target = {"x_bits": f32_bits(30.0), "y_bits": f32_bits(30.0)}
    for record in scout_units(case, ARRIVAL, TICKS):
        record["target"] = dict(target)
    case.allowed["unit_fields"].append({
        "unit": SCOUT, "field": "target", "ticks": [ARRIVAL, TICKS],
        "old": None, "new": target})


def plant_current_order_not_cleared(case):
    for record in scout_units(case, ARRIVAL, TICKS):
        record["order"] = "Move:-"


def plant_world_pool_kept(case):
    for tick in range(ARRIVAL, TICKS + 1):
        find(case.new["acme"], record="world", tick=tick)["pools"] = 1


def plant_world_record_missing(case):
    records = case.new["acme"]
    records.remove(find(records, record="world", tick=200))


def plant_extra_scout_op_listed(case):
    # Reviewer plant d2: a third added scout operation on tick 1, listed.
    records = case.new["acme"]
    second = records.index(find(records, record="op", op_id=2))
    records.insert(second + 1, op_record("acme", 1, 0, SCOUT, "Orders", [],
                                         False))
    renumber_ops(records)
    case.allowed["operations"].append({
        "change": "added", "tick": 1, "index": 2, "unit": SCOUT,
        "phase": "Orders", "events": [], "before_equals_after": False})


def plant_added_op_unit_null_listed(case):
    # Reviewer plant d3, in the form that only the owner rule can see: the
    # one added op of tick 1 has no unit, and the list says so.
    find(case.new["acme"], record="op", op_id=2)["unit"] = None
    case.allowed["operations"][0]["unit"] = None


def plant_added_op_before_command_listed(case):
    # Reviewer plant p: the added op comes before the Command op, listed at
    # index 0; the old op then has id 2.
    records = case.new["acme"]
    added = find(records, record="op", op_id=2)
    records.remove(added)
    records.insert(records.index(find(records, record="op", op_id=1)), added)
    renumber_ops(records)
    case.allowed["operations"][0]["index"] = 0


def plant_listed_new_value_wrong(case):
    # Reviewer plant q: the list gives a wrong new value on the real span.
    case.allowed["unit_fields"][0]["new"] = "Pending"


def plant_stale_allowed_entry(case):
    case.allowed["unit_fields"].append({
        "unit": SCOUT, "field": "separated", "ticks": [100, 100],
        "old": False, "new": True})


def plant_allowed_position_entry(case):
    case.allowed["unit_fields"].append({
        "unit": SCOUT, "field": "x_bits", "ticks": [5, 5],
        "old": "00000000", "new": "00000001"})


def plant_edge_hit_entry_without_g1(case):
    # Reviewer plant j2: edge_hit null -> false in the new acme trace, and
    # listed, in a run that is not declared as G1-in-A2c.
    for record in case.new["acme"]:
        if record.get("record") == "unit":
            record["edge_hit"] = False
    case.allowed["unit_fields"].append({
        "unit": SCOUT, "field": "edge_hit", "ticks": [1, ARRIVAL - 1],
        "old": None, "new": False})


def plant_edge_hit_in_trace_without_g1(case):
    for record in case.new["base"]:
        if record.get("record") == "unit":
            record["edge_hit"] = False


def plant_stream_flag_as_number(case):
    # Reviewer plant m: before_equals_after written as 0 or 1.
    def patch(stream):
        op = find_stream_op(stream, "base", 9, 0)
        op["before_equals_after"] = 1 if op["before_equals_after"] else 0
    case.new_stream_patch = patch


def plant_stream_ids_as_floats(case):
    # Reviewer plant m2: operation_id and operation_count as floats.
    def patch(stream):
        tick = next(s for s in stream if s["package"] == "cedar")["ticks"][9]
        tick["operation_count"] = float(tick["operation_count"])
        for op in tick["operations"]:
            op["operation_id"] = float(op["operation_id"])
    case.new_stream_patch = patch


def plant_trace_op_id_true(case):
    # Reviewer plant r: the trace op_id 1 written as true.
    find(case.new["acme"], record="op", op_id=1)["op_id"] = True


def plant_arrival_one_tick_late(case):
    # Both runs arrive one tick late, so P1 sees no difference.
    for side in (case.old, case.new):
        record = find(side["acme"], record="unit", tick=ARRIVAL, unit=SCOUT)
        before = find(side["acme"], record="unit", tick=ARRIVAL - 1,
                      unit=SCOUT)
        record["x_bits"] = before["x_bits"]
        record["y_bits"] = before["y_bits"]


def plant_no_old_trace(case):
    case.old_dir_given = False


def plant_p0_mode_acme_bit(case):
    case.to_p0()
    record = find(case.new["acme"], record="unit", tick=77, unit=SCOUT)
    record["x_bits"] = flip_low_bit(record["x_bits"])


def plant_p0_mode_wrong_op_capture(case):
    # Both traces capture one op with the wrong flag: P0 is equal, but the
    # traces are not the frozen stream.
    case.to_p0()
    stream = project_stream(case.old)
    case.new_stream_patch = None
    for side in (case.old, case.new):
        record = find(side["haul"], record="op", tick=9)
        record["before_equals_after"] = not record["before_equals_after"]
    case.frozen = stream


def plant_p0_mode_no_stream(case):
    case.to_p0()
    case.p0_stream = False


def plant_order_entry_after_arrival_listed(case):
    # The order is still in the new trace on the tick after arrival, and the
    # list says so: the span ends after the arrival tick (design 9.24a).
    find(case.new["acme"], record="unit", tick=ARRIVAL + 1,
         unit=SCOUT)["order"] = "Move:-"
    spans = [e for e in case.allowed["unit_fields"] if e["field"] == "order"]
    spans[0]["ticks"] = [1, ARRIVAL + 1]


def plant_state_entry_straddles_arrival_listed(case):
    # Reviewer plant (b) of the Q1a review round 2: the span begins before
    # arrival and ends ON the arrival tick. The trace agrees, so only the
    # check of the LAST tick can see it (a check of the first tick cannot).
    record = find(case.new["acme"], record="unit", tick=ARRIVAL, unit=SCOUT)
    record["state"] = "pool"
    spans = [e for e in case.allowed["unit_fields"] if e["field"] == "state"]
    spans[0]["ticks"] = [1, ARRIVAL]


def plant_flag_entry_straddles_arrival_listed(case):
    # The new run projects the scout on ticks 1 to ARRIVAL, and the list names
    # the span [1, ARRIVAL]: it ends ON the arrival tick, which only the order
    # followers may do.
    for record in scout_units(case, 1, ARRIVAL):
        record["was_projected"] = True
    case.allowed["unit_fields"].append({
        "unit": SCOUT, "field": "was_projected", "ticks": [1, ARRIVAL],
        "old": False, "new": True})


def plant_acme_last_failure_not_allowed(case):
    find(case.new["acme"], record="unit", tick=50, unit=SCOUT)[
        "last_failure"] = {"order": "acme:order_move", "reason": "blocked",
                           "tick": 49}


def plant_acme_queue_not_allowed(case):
    find(case.new["acme"], record="unit", tick=30, unit=SCOUT)["queue"] = [
        "Move"]


def plant_base_last_failure(case):
    find(case.new["base"], record="unit", tick=70, unit=1)["last_failure"] = {
        "order": "base:gather", "reason": "blocked", "tick": 69}


def plant_haul_was_projected_null(case):
    find(case.new["haul"], record="unit", tick=60, unit=1)[
        "was_projected"] = None


def plant_haul_move_to_route(case):
    find(case.new["haul"], record="op", tick=1)["move_to_route"] = "direct"


def plant_route_unchanged_in_new(case):
    # The new run still says "direct", but the list says "envelope".
    find(case.new["acme"], record="op", op_id=1)["move_to_route"] = "direct"


def plant_route_changed_not_listed(case):
    case.allowed["operation_fields"] = []


def plant_route_entry_wrong_old(case):
    case.allowed["operation_fields"][0]["old"] = "per_unit"


def plant_route_entry_other_field(case):
    case.allowed["operation_fields"][0]["field"] = "phase"


def plant_route_value_not_in_format(case):
    find(case.new["acme"], record="op", op_id=1)["move_to_route"] = "shortcut"


def plant_order_is_a_content_id(case):
    find(case.new["acme"], record="unit", tick=40, unit=SCOUT)[
        "order"] = "acme:order_move"


def plant_order_kind_is_not_a_behaviour(case):
    # A valid stage, but the kind is not a Behaviour name (a package name, as
    # a content id would give it).
    find(case.new["acme"], record="unit", tick=40, unit=SCOUT)[
        "order"] = "acme:-"


def plant_order_stage_with_brackets(case):
    find(case.new["base"], record="unit", tick=10, unit=1)[
        "order"] = "Gather:Waiting(NoDeposit)"


def plant_unit_record_without_queue(case):
    del find(case.new["base"], record="unit", tick=10, unit=1)["queue"]


def plant_queue_holds_a_content_id(case):
    find(case.new["base"], record="unit", tick=10, unit=1)["queue"] = [
        "base:order_move"]


def plant_was_projected_as_number(case):
    find(case.new["base"], record="unit", tick=10, unit=1)["was_projected"] = 0


def plant_last_failure_as_text(case):
    find(case.new["base"], record="unit", tick=10, unit=1)[
        "last_failure"] = "blocked"


def plant_flip_gather_stage_haul(case):
    # The legacy Gather leaves order at "none", so the stage is the only place
    # where a change of the gather protocol (task A2b1) shows in the trace.
    record = find(case.new["haul"], record="unit", tick=40, unit=1)
    record["gather_stage"] = "Unloading"


def plant_gather_stage_in_acme_not_allowed(case):
    find(case.new["acme"], record="unit", tick=60, unit=SCOUT)[
        "gather_stage"] = "Idle"


def plant_gather_stage_with_brackets(case):
    find(case.new["base"], record="unit", tick=10, unit=1)[
        "gather_stage"] = "Waiting(NoDeposit)"


def plant_gather_stage_is_a_behaviour_text(case):
    find(case.new["base"], record="unit", tick=10, unit=1)[
        "gather_stage"] = "Gather:Idle"


def plant_unit_record_without_gather_stage(case):
    del find(case.new["base"], record="unit", tick=10, unit=1)["gather_stage"]


def plant_counts_plain_unit_tick(case):
    case.to_counts()
    find(case.new["cedar"], record="unit", tick=77, unit=1)["state"] = "plain"


def plant_counts_separated_without_state(case):
    case.to_counts()
    find(case.new["haul"], record="unit", tick=88, unit=2)["separated"] = True


def plant_counts_was_projected(case):
    case.to_counts()
    find(case.new["base"], record="unit", tick=99, unit=1)[
        "was_projected"] = True


def plant_counts_empty_trace(case):
    case.to_counts()
    case.new["acme"] = []


def plant_counts_missing_trace(case):
    case.to_counts()
    case.new["cedar"] = None


PLANTS = [
    # (name, expected failed check, text that the detail must hold, plant)
    ("flipped position bit in base", "P0",
     "tick 200, unit 1, field x_bits", plant_flip_position_bit_base),
    ("flipped position bit in acme", "P1", "field y_bits",
     plant_flip_position_bit_acme),
    ("added Stored event", "P2", "haul tick 30",
     plant_added_stored_event),
    ("changed operation in base", "P3", "stream of base changed",
     plant_changed_operation_base),
    ("scout operation on the arrival tick", "P3", "tick 123",
     plant_scout_op_on_arrival_tick),
    ("two acme operations in the other order (stream, ids move)", "P3",
     "order", plant_reorder_acme_ops_with_ids),
    ("two acme operations in the other order (trace, ids stay)", "P1",
     "operation", plant_reorder_acme_ops_keep_ids),
    ("flipped before_equals_after on the tick-1 operation", "P3",
     "old operation", plant_flip_flag_tick1_op),
    ("flipped before_equals_after in the stream only", "STREAM-NEW",
     "acme tick 1", plant_flip_flag_tick1_stream_only),
    ("empty traces of base (both runs)", "COUNTS-NEW", "empty trace",
     plant_empty_traces_base),
    ("empty new trace of acme", "COUNTS-NEW", "empty trace",
     plant_empty_trace_new_acme),
    ("trace of 399 ticks (both runs)", "COUNTS-OLD", "ends at tick 399",
     plant_399_ticks),
    ("changed event payload, same kind", "P2", "cedar tick 40",
     plant_changed_payload_same_kind),
    ("other emitter file in the new run", "EMITTER", "not the sha256",
     plant_other_emitter_file),
    ("emitter constant that is not the file hash (n)", "EMITTER",
     "not the sha256", plant_emitter_not_the_file_hash),
    ("fixture does not hash to the classified stream", "FIXTURE",
     "does not hash", plant_fixture_not_classified_stream),
    ("fixture path is the new stream path", "FIXTURE", "one file",
     plant_fixture_is_the_new_stream_path),
    ("fixture touched before the fixture commit", "FIXTURE",
     "must not touch", plant_fixture_touched_before_commit),
    ("acme field difference not in the allowed list", "P1",
     "field separated", plant_acme_field_not_allowed),
    ("flipped was_projected in acme", "P1", "field was_projected",
     plant_flip_was_projected_acme),
    ("flipped was_projected in haul", "P0",
     "tick 60, unit 1, field was_projected",
     plant_flip_was_projected_haul),
    ("base PlainMoveProgress ends one tick early", "P0",
     "tick 79, unit 1, field state", plant_base_state_plain_to_none),
    ("base pending movement order count changed", "P0",
     "world record, tick 150, field pending_movement_orders",
     plant_base_pending_orders),
    ("pool kept to tick 400, listed (c2)", "ALLOWED",
     "at or after the arrival tick", plant_pool_kept_to_400_listed),
    ("MoveTarget kept after arrival, listed (c3)", "ALLOWED",
     "at or after the arrival tick", plant_move_target_kept_listed),
    ("current order not cleared after arrival", "P1", "field order",
     plant_current_order_not_cleared),
    ("world pool count kept after arrival", "P1", "world field pools",
     plant_world_pool_kept),
    ("world record missing at one tick", "COUNTS-NEW", "no world record",
     plant_world_record_missing),
    ("third added scout op on tick 1, listed (d2)", "P3", "exactly one",
     plant_extra_scout_op_listed),
    ("added op with unit null, listed (d3)", "P3", "is not the scout",
     plant_added_op_unit_null_listed),
    ("added op before the Command op, listed (p)", "P3", "keeps its",
     plant_added_op_before_command_listed),
    ("listed new value is wrong (q)", "P1", "does not occur",
     plant_listed_new_value_wrong),
    ("allowed entry that does not occur (stale list)", "P1",
     "does not occur", plant_stale_allowed_entry),
    ("allowed entry for a position field", "ALLOWED", "x_bits",
     plant_allowed_position_entry),
    ("edge_hit entry without --g1-in-a2c (j2)", "ALLOWED",
     "edge_hit only with", plant_edge_hit_entry_without_g1),
    ("edge_hit in a trace without --g1-in-a2c", "COUNTS-NEW",
     "edge_hit in a run", plant_edge_hit_in_trace_without_g1),
    ("stream flag written as 0 or 1 (m)", "STREAM-NEW", "bytes",
     plant_stream_flag_as_number),
    ("stream ids written as floats (m2)", "STREAM-NEW", "bytes",
     plant_stream_ids_as_floats),
    ("trace op_id written as true (r)", "COUNTS-NEW", "op_id",
     plant_trace_op_id_true),
    ("arrival one tick after the calculation", "ARRIVAL", "124",
     plant_arrival_one_tick_late),
    ("no old trace given", "COUNTS-OLD", "no trace given",
     plant_no_old_trace),
    ("--p0 mode: flipped position bit in acme", "P0", "acme: bytes differ",
     plant_p0_mode_acme_bit),
    ("--p0 mode: one wrong op capture in both traces", "STREAM-OLD",
     "haul tick 9", plant_p0_mode_wrong_op_capture),
    ("--p0 mode: no stream given", "STREAM-OLD", "no old stream given",
     plant_p0_mode_no_stream),
    # Task Q1b: design 9.24 and the format /3.
    ("order entry ends after the arrival tick, listed (a)", "ALLOWED",
     "still there on the arrival tick", plant_order_entry_after_arrival_listed),
    ("state entry straddles the arrival tick, listed (b)", "ALLOWED",
     "at or after the arrival tick",
     plant_state_entry_straddles_arrival_listed),
    ("was_projected entry straddles the arrival tick, listed (b)", "ALLOWED",
     "at or after the arrival tick",
     plant_flag_entry_straddles_arrival_listed),
    ("acme last_failure not in the allowed list", "P1",
     "field last_failure", plant_acme_last_failure_not_allowed),
    ("acme queue not in the allowed list", "P1", "field queue",
     plant_acme_queue_not_allowed),
    ("last_failure in base", "P0", "tick 70, unit 1, field last_failure",
     plant_base_last_failure),
    ("was_projected null instead of false in haul", "P0",
     "tick 60, unit 1, field was_projected", plant_haul_was_projected_null),
    ("move_to_route on a haul operation", "P0",
     "op record, tick 1, op_id 1, field move_to_route",
     plant_haul_move_to_route),
    ("move_to_route still direct in the new run, list says envelope", "P1",
     "operation difference", plant_route_unchanged_in_new),
    ("move_to_route changed in the new run, not listed", "P1",
     "operation difference", plant_route_changed_not_listed),
    ("operation_fields entry with a wrong old value", "P1",
     "does not occur", plant_route_entry_wrong_old),
    ("operation_fields entry for another field", "ALLOWED",
     "only move_to_route", plant_route_entry_other_field),
    ("move_to_route value that the format does not name", "COUNTS-NEW",
     "move_to_route", plant_route_value_not_in_format),
    ("order written as a content id", "COUNTS-NEW", "behaviours",
     plant_order_is_a_content_id),
    ("order kind that is not a behaviour name", "COUNTS-NEW", "behaviours",
     plant_order_kind_is_not_a_behaviour),
    ("gather stage with brackets in order", "COUNTS-NEW", "behaviours",
     plant_order_stage_with_brackets),
    ("unit record without queue", "COUNTS-NEW", "unit keys",
     plant_unit_record_without_queue),
    ("queue that holds a content id", "COUNTS-NEW", "queue",
     plant_queue_holds_a_content_id),
    ("was_projected written as 0", "COUNTS-NEW", "was_projected",
     plant_was_projected_as_number),
    ("last_failure written as text", "COUNTS-NEW", "last_failure",
     plant_last_failure_as_text),
    # Q1b review follow-up: gather_stage (format /4) and the decoded P0 detail.
    ("flipped gather_stage in haul (order stays none)", "P0",
     "tick 40, unit 1, field gather_stage", plant_flip_gather_stage_haul),
    ("gather_stage difference in acme not in the allowed list", "P1",
     "field gather_stage", plant_gather_stage_in_acme_not_allowed),
    ("gather_stage written with brackets", "COUNTS-NEW", "gather_stage",
     plant_gather_stage_with_brackets),
    ("gather_stage written as a behaviour text", "COUNTS-NEW",
     "gather_stage", plant_gather_stage_is_a_behaviour_text),
    ("unit record without gather_stage", "COUNTS-NEW", "unit keys",
     plant_unit_record_without_gather_stage),
    ("--counts: one unit-tick on a plain progress", "STOP", "plain 1",
     plant_counts_plain_unit_tick),
    ("--counts: separated with no movement state (b)", "STOP", "(b) 1",
     plant_counts_separated_without_state),
    ("--counts: was_projected true (c)", "STOP", "(c) 1",
     plant_counts_was_projected),
    ("--counts: empty trace", "COUNTS", "empty trace",
     plant_counts_empty_trace),
    ("--counts: no trace for one package", "COUNTS", "no trace given",
     plant_counts_missing_trace),
]


def classify(classifier, case):
    with tempfile.TemporaryDirectory() as tmp:
        args = case.materialise(tmp)
        frozen = getattr(case, "frozen", None)
        if frozen is not None:
            # The frozen stream is the one of the clean traces.
            Path(args["stream"]).write_bytes(stream_bytes(frozen))
            args["old_sha256"] = hashlib.sha256(
                stream_bytes(frozen)).hexdigest()
        if case.mode == "counts":
            return classifier.run_counts(**args)
        if case.mode == "p0":
            return classifier.run_p0(**args)
        return classifier.run_a2c(**args)


def clean_cases():
    after = Case()
    before = Case()
    before.stage = "before-fixture-commit"
    p0 = Case()
    p0.to_p0()
    g1 = Case()
    g1.g1_in_a2c = True
    for side in (g1.old, g1.new):
        for records in side.values():
            for record in records:
                if record.get("record") == "unit":
                    record["edge_hit"] = False
    counts = Case()
    counts.to_counts()
    return [("after-fixture-commit", after), ("before-fixture-commit", before),
            ("--p0", p0), ("--g1-in-a2c", g1), ("--counts", counts)]


def run_plants(classifier):
    """Return (rows, clean_ok, clean reports). Each row: (name, expected
    check, red, failed checks). A plant is red only when the expected check
    fails and its detail holds the expected text."""
    clean = [(name, classify(classifier, case)) for name, case in clean_cases()]
    clean_ok = all(report.passed for _, report in clean)
    rows = []
    for name, expected, needle, plant in PLANTS:
        case = Case()
        plant(case)
        report = classify(classifier, case)
        failed = report.failed()
        red = (not report.passed) and needle in failed.get(expected, "")
        rows.append((name, expected, red, sorted(failed)))
    return rows, clean_ok, clean


class RefreezeClassifierSelfTest(unittest.TestCase):
    classifier = None

    @classmethod
    def setUpClass(cls):
        cls.classifier = load_classifier()

    def test_clean_pairs_pass(self):
        for name, case in clean_cases():
            with self.subTest(name):
                report = classify(self.classifier, case)
                self.assertTrue(report.passed, "\n".join(report.lines()))

    def test_counts_report_gather_stage_counts(self):
        # (e): the synthetic gather worker is in MovingToNode on 79 ticks and
        # in Gathering on 321 ticks of each of the three gather packages.
        case = Case()
        case.to_counts()
        report = classify(self.classifier, case)
        details = {name: detail for name, _, detail in report.checks}
        for package in ("BASE", "HAUL", "CEDAR"):
            self.assertIn("(e) gather_stage unit-ticks Gathering 321, "
                          "MovingToNode 79", details["COUNT-" + package])
        self.assertIn("(e) gather_stage unit-ticks none",
                      details["COUNT-ACME"])

    def test_p0_detail_decodes_tick_unit_and_field(self):
        case = Case()
        case.to_p0()
        find(case.new["cedar"], record="unit", tick=321, unit=2)[
            "was_projected"] = True
        report = classify(self.classifier, case)
        self.assertIn("cedar: bytes differ (first at line ",
                      report.failed()["P0"])
        self.assertIn("unit record, tick 321, unit 2, field was_projected: "
                      "false -> true", report.failed()["P0"])

    def test_p0_detail_names_the_first_field_in_format_order(self):
        # Two fields of one record differ: the detail names the first in the
        # order of the format (position before gather_stage).
        case = Case()
        case.to_p0()
        record = find(case.new["haul"], record="unit", tick=40, unit=1)
        record["gather_stage"] = "Unloading"
        record["x_bits"] = flip_low_bit(record["x_bits"])
        report = classify(self.classifier, case)
        self.assertIn("unit record, tick 40, unit 1, field x_bits",
                      report.failed()["P0"])

    def test_a_listed_gather_stage_difference_passes(self):
        # gather_stage can be named in the allowed list (it is a P0 field, so a
        # difference is red unless listed).
        case = Case()
        for tick in range(10, 21):
            find(case.new["acme"], record="unit", tick=tick, unit=SCOUT)[
                "gather_stage"] = "Idle"
        case.allowed["unit_fields"].append({
            "unit": SCOUT, "field": "gather_stage", "ticks": [10, 20],
            "old": "none", "new": "Idle"})
        report = classify(self.classifier, case)
        self.assertTrue(report.passed, "\n".join(report.lines()))

    def test_synthetic_stream_bytes_match_fixture_format(self):
        fixture = REPO / "tests/m3/m2_operation_stream.json"
        raw = fixture.read_bytes()
        self.assertEqual(stream_bytes(json.loads(raw)), raw)


def _make_plant_test(name, expected, needle, plant):
    def test(self):
        case = Case()
        plant(case)
        report = classify(self.classifier, case)
        text = "\n".join(report.lines())
        self.assertFalse(report.passed, "plant gave PASS: " + name)
        self.assertIn(expected, report.failed(), text)
        self.assertIn(needle, report.failed()[expected], text)
    return test


for _index, (_name, _expected, _needle, _plant) in enumerate(PLANTS, 1):
    setattr(RefreezeClassifierSelfTest,
            "test_plant_%02d_%s" % (_index, _plant.__name__),
            _make_plant_test(_name, _expected, _needle, _plant))


if __name__ == "__main__":
    sys.exit(unittest.main())
