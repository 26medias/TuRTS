# tests/m3

- `m2_operation_stream.json`: the frozen M2 operation stream. Its test is
  `crates/turts_pack/tests/m3_m2_stream.rs`. Its sha256 is in
  `refutations.py:18`. It changes one time only, at the pre-MP1 re-freeze
  (task Q2).
- `refutations.py`, `refutations.json`: the M3 fault registry and runner.
- `refreeze_classify.py`: the re-freeze classifier (pre-MP1 slice, task Q1a).
- `test_refreeze_classify.py`: its self-test (synthetic runs and plants).
- `refreeze_allowed.json`: the list of the allowed acme differences. Task Q1b
  writes it before the A2c commit. It does not exist before Q1b.

## Re-freeze classifier

Design: `docs/superpowers/specs/2026-11-12-pre-mp1-slice-design.md`, section 5
and section 9 items 6, 7 and 14 (revision 5).

Commands:

    python3 tests/m3/refreeze_classify.py --self-test
    python3 tests/m3/test_refreeze_classify.py
    python3 tests/m3/refreeze_classify.py --p0 OLD_DIR NEW_DIR
    python3 tests/m3/refreeze_classify.py --a2c --old-trace OLD_DIR \
        --new-trace NEW_DIR --old-stream OLD.json --new-stream NEW.json \
        --allowed tests/m3/refreeze_allowed.json \
        [--fixture tests/m3/m2_operation_stream.json] [--old-sha256 H]

The output has one `CHECK <name> PASS|FAIL <detail>` line for each check,
one `SHA256` line for each file that the classifier read, and `RESULT PASS`
(exit 0) or `RESULT FAIL` (exit 1). A check with no input is a FAIL.

| Check | What it checks |
| --- | --- |
| `ALLOWED` | The allowed list follows schema `turts-refreeze-allowed/1`. A position field cannot be allowed. |
| `COUNTS-OLD`, `COUNTS-NEW` | Each trace follows the format below: 400 ticks, a record for each initial unit at each tick (400 x the initial unit count), op ids 1, 2, 3, ... An absent, empty or short trace is a FAIL. |
| `EMITTER` | The emitter sha256 is equal in all 8 traces. |
| `OLD-HASH` | The old stream has the frozen sha256 (default `bcbf72c2...e370b`). |
| `STREAM-OLD`, `STREAM-NEW` | The stream has the structure of `m3_m2_stream.rs` and is the projection of the trace of the same run (count, ids, event kinds, `before_equals_after`). |
| `P0` | `--a2c`: the traces of base, haul and cedar are byte-equal. `--p0`: all four packages. |
| `P1` | acme: every trace field and every operation record is equal, except the entries of the allowed list. Each entry must occur. |
| `P2` | All four packages: the full events, with payloads, are equal on each tick. |
| `ARRIVAL` | `ceil(\|P - p0\| / (speed x 1/tick_hz))` from `mods/acme` and `m3_m2_stream.rs` equals the first tick where the scout is on P, in both acme traces. |
| `P3` | The streams of base, haul and cedar are equal. In acme, each old operation is in the new tick, in order, unchanged. Each new operation belongs to the scout, has no event kind, and is on tick 1 or on the tick after arrival. Operation ids increase. |
| `FIXTURE` | The fixture file has the same sha256 as the classified new stream. |

## Trace format `turts-refreeze-trace/1`

The reference text is the docstring at the top of `refreeze_classify.py`. In
short:

- One JSONL file for each package: `<dir>/base.jsonl`, `haul.jsonl`,
  `acme.jsonl`, `cedar.jsonl`. Each line ends with `\n`.
- Line 1: `{"record":"header","format":"turts-refreeze-trace/1","package":P,
  "emitter_sha256":H,"ticks":400,"unit_count":N,"units":[{"unit":U,
  "content_id":C,"owner_slot":S},...]}`. No value that changes between runs.
- For each tick 1 to 400: first the unit records, in ascending unit id, then
  the operation records, in capture order.
- Unit record: `{"record":"unit","package":P,"tick":T,"unit":U,
  "x_bits":"8 hex","y_bits":"8 hex","target":null|{"x_bits":..,"y_bits":..},
  "activity":"Pending|Moving|Resting|Blocked|none","state":"none|pool",
  "separated":bool,"was_projected":bool}`, plus `"edge_hit":bool` in every
  unit record only when G1 lands in A2c. `activity` is the `MovementActivity`
  of the `MoveProgress`, or `none` (design section 9 item 7).
- Operation record: `{"record":"op","package":P,"tick":T,"op_id":N,
  "unit":U|null,"phase":"Command|Orders|PostMovementOrders|Other",
  "events":[EconomyEvent as serde JSON],"before_equals_after":bool}`.
