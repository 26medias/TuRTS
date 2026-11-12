#!/usr/bin/env python3
"""Self-test of the re-freeze classifier (pre-MP1 slice, task Q1a).

Each plant changes one thing in a clean synthetic pair of runs. The
classifier must give FAIL, and the named check must be one of the failed
checks. The clean pair must give PASS. Python 3 standard library only.

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
EMITTER = "ab" * 32
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
        "format": "turts-refreeze-trace/1",
        "package": package,
        "emitter_sha256": EMITTER,
        "ticks": TICKS,
        "unit_count": len(units),
        "units": units,
    }


def unit_record(package, tick, unit, x, y, target=None, activity="none",
                state="none", separated=False, was_projected=False):
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
        "separated": separated,
        "was_projected": was_projected,
    }


def op_record(package, tick, op_id, unit, phase, events, before_equals_after):
    return {
        "record": "op",
        "package": package,
        "tick": tick,
        "op_id": op_id,
        "unit": unit,
        "phase": phase,
        "events": events,
        "before_equals_after": before_equals_after,
    }


def event(tick, ordinal, kind, payload):
    return {"tick": tick, "ordinal": ordinal, "owner_slot": 0,
            "kind": {kind: payload}}


def economy_trace(package):
    """A gather package: a worker that moves, a static building, ops with
    economy events. In `haul` a third unit appears at tick 100."""
    units = [
        {"unit": 1, "content_id": package + ":worker", "owner_slot": 0},
        {"unit": 2, "content_id": package + ":depot", "owner_slot": 0},
    ]
    records = [header(package, units)]
    op_id = 0
    ordinal = 0
    for tick in range(1, TICKS + 1):
        x = 10.0 + 0.25 * min(tick, 80)
        records.append(unit_record(package, tick, 1, x, 12.0,
                                   target=(30.0, 12.0) if tick < 80 else None))
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
    units = [{"unit": SCOUT, "content_id": "acme:scout", "owner_slot": 0}]
    records = [header("acme", units)]
    op_id = 0
    for tick in range(1, TICKS + 1):
        x, y = scout_position(tick)
        moving = tick < ARRIVAL
        records.append(unit_record(
            "acme", tick, SCOUT, x, y,
            target=(30.0, 30.0) if moving else None,
            activity="Moving" if new and moving else "none",
            state="pool" if new and moving else "none"))
        ops = []
        if tick == 1:
            ops.append((SCOUT, "Command", [], False))
            if new:
                ops.append((SCOUT, "Orders", [], False))
        if new and tick == ARRIVAL + 1:
            ops.append((SCOUT, "PostMovementOrders", [], False))
        for unit, phase, events, flag in ops:
            op_id += 1
            records.append(op_record("acme", tick, op_id, unit, phase,
                                     events, flag))
    return records


def clean_allowed():
    return {
        "schema": "turts-refreeze-allowed/1",
        "package": "acme",
        "note": "synthetic list of the self-test",
        "unit_fields": [
            {"unit": SCOUT, "field": "activity", "ticks": [1, ARRIVAL - 1],
             "old": "none", "new": "Moving"},
            {"unit": SCOUT, "field": "state", "ticks": [1, ARRIVAL - 1],
             "old": "none", "new": "pool"},
        ],
        "operations": [
            {"change": "added", "tick": 1, "index": 1, "unit": SCOUT,
             "phase": "Orders", "events": [], "before_equals_after": False},
            {"change": "added", "tick": ARRIVAL + 1, "index": 0,
             "unit": SCOUT, "phase": "PostMovementOrders", "events": [],
             "before_equals_after": False},
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
        self.mode = "a2c"
        self.old_dir_given = True

    def materialise(self, root):
        root = Path(root)
        paths = {}
        for side, traces in (("old", self.old), ("new", self.new)):
            directory = root / side
            directory.mkdir()
            for package, records in traces.items():
                if records is None:
                    continue
                text = "".join(
                    json.dumps(r, sort_keys=True, separators=(",", ":")) + "\n"
                    for r in records)
                (directory / (package + ".jsonl")).write_bytes(text.encode())
            paths[side] = directory
        old_stream = stream_bytes(project_stream(self.old))
        new = project_stream(self.new)
        if self.new_stream_patch is not None:
            self.new_stream_patch(new)
        new_stream = stream_bytes(new)
        fixture = new_stream
        if self.fixture_patch is not None:
            fixture = self.fixture_patch(new_stream, old_stream)
        (root / "old_stream.json").write_bytes(old_stream)
        (root / "new_stream.json").write_bytes(new_stream)
        (root / "fixture.json").write_bytes(fixture)
        (root / "allowed.json").write_text(json.dumps(self.allowed, indent=2))
        return {
            "old_dir": paths["old"] if self.old_dir_given else None,
            "new_dir": paths["new"],
            "old_stream": root / "old_stream.json",
            "new_stream": root / "new_stream.json",
            "allowed": root / "allowed.json",
            "fixture": root / "fixture.json",
            "old_sha256": hashlib.sha256(old_stream).hexdigest(),
            "repo": REPO,
        }


def find(records, **keys):
    for record in records:
        if all(record.get(k) == v for k, v in keys.items()):
            return record
    raise KeyError(keys)


def find_stream_op(stream, package, tick, index):
    entry = next(s for s in stream if s["package"] == package)
    return entry["ticks"][tick - 1]["operations"][index]


# Each plant: (name, expected failed check, function that changes a Case).

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
    record = find(case.new["acme"], record="op", phase="PostMovementOrders")
    records = case.new["acme"]
    records.remove(record)
    record["tick"] = ARRIVAL
    last_unit = records.index(find(records, record="unit", tick=ARRIVAL))
    records.insert(last_unit + 1, record)
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


def plant_other_emitter(case):
    for records in case.new.values():
        records[0]["emitter_sha256"] = "cd" * 32


def plant_fixture_not_classified_stream(case):
    case.fixture_patch = lambda new_stream, old_stream: old_stream


def plant_acme_field_not_allowed(case):
    record = find(case.new["acme"], record="unit", tick=10, unit=SCOUT)
    record["separated"] = True


def plant_flip_was_projected_acme(case):
    record = find(case.new["acme"], record="unit", tick=20, unit=SCOUT)
    record["was_projected"] = True


def plant_flip_was_projected_haul(case):
    record = find(case.new["haul"], record="unit", tick=60, unit=1)
    record["was_projected"] = True


def plant_stale_allowed_entry(case):
    case.allowed["unit_fields"].append({
        "unit": SCOUT, "field": "separated", "ticks": [300, 300],
        "old": False, "new": True})


def plant_allowed_position_entry(case):
    case.allowed["unit_fields"].append({
        "unit": SCOUT, "field": "x_bits", "ticks": [5, 5],
        "old": "00000000", "new": "00000001"})


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
    # --p0 mode (A2a, A2b): the four packages, old against old.
    case.mode = "p0"
    case.new = copy.deepcopy(case.old)
    record = find(case.new["acme"], record="unit", tick=77, unit=SCOUT)
    record["x_bits"] = flip_low_bit(record["x_bits"])


PLANTS = [
    # (name, expected failed check, text that the detail must hold, plant)
    ("flipped position bit in base", "P0", "base: bytes differ",
     plant_flip_position_bit_base),
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
    ("different emitter sha256", "EMITTER", "differ", plant_other_emitter),
    ("fixture does not hash to the classified stream", "FIXTURE",
     "does not hash", plant_fixture_not_classified_stream),
    ("acme field difference not in the allowed list", "P1",
     "field separated", plant_acme_field_not_allowed),
    ("flipped was_projected in acme", "P1", "field was_projected",
     plant_flip_was_projected_acme),
    ("flipped was_projected in haul", "P0", "haul: bytes differ",
     plant_flip_was_projected_haul),
    ("allowed entry that does not occur (stale list)", "P1",
     "does not occur", plant_stale_allowed_entry),
    ("allowed entry for a position field", "ALLOWED", "x_bits",
     plant_allowed_position_entry),
    ("arrival one tick after the calculation", "ARRIVAL", "124",
     plant_arrival_one_tick_late),
    ("no old trace given", "COUNTS-OLD", "no trace given",
     plant_no_old_trace),
    ("--p0 mode: flipped position bit in acme", "P0", "acme: bytes differ",
     plant_p0_mode_acme_bit),
]


def classify(classifier, case):
    with tempfile.TemporaryDirectory() as tmp:
        args = case.materialise(tmp)
        if case.mode == "p0":
            return classifier.run_p0(args["old_dir"], args["new_dir"])
        return classifier.run_a2c(**args)


def run_plants(classifier):
    """Return (rows, clean_ok, clean report). Each row: (name, expected
    check, red, failed checks). A plant is red only when the expected check
    fails and its detail holds the expected text."""
    clean = classify(classifier, Case())
    clean_ok = clean.passed
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

    def test_clean_pair_passes(self):
        report = classify(self.classifier, Case())
        self.assertTrue(report.passed, "\n".join(report.lines()))

    def test_clean_p0_pair_passes(self):
        case = Case()
        case.mode = "p0"
        case.new = copy.deepcopy(case.old)
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
