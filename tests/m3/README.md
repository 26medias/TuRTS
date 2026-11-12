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

### Commands

    python3 tests/m3/refreeze_classify.py --self-test
    python3 tests/m3/test_refreeze_classify.py

    # A2a and A2b: the stream does not change; it must be the stream of
    # both traces.
    python3 tests/m3/refreeze_classify.py --p0 OLD_DIR NEW_DIR \
        --stream tests/m3/m2_operation_stream.json --emitter FILE [FILE ...]

    # A2c and Q2.
    python3 tests/m3/refreeze_classify.py --a2c --stage STAGE \
        --old-trace OLD_DIR --new-trace NEW_DIR \
        --old-stream OLD.json --new-stream NEW.json \
        --allowed tests/m3/refreeze_allowed.json \
        --fixture tests/m3/m2_operation_stream.json --emitter FILE [FILE ...]

Add `--g1-in-a2c` only when G1 lands in A2c (design section 5 item 0).
Then every unit record holds `edge_hit`, and the list can allow it.

The output has one `CHECK <name> PASS|FAIL <detail>` line for each check,
one `SHA256` line for each file that the classifier read, and `RESULT PASS`
(exit 0) or `RESULT FAIL` (exit 1). A check with no input is a FAIL.

### When FIXTURE runs (the stages of the design)

1. A2c lands. The builder does not touch the fixture (design section 5 item
   2). Run `--a2c --stage before-fixture-commit`. `--new-stream` is the
   stream that the new run wrote, kept outside the repository tree.
   FIXTURE passes only when the fixture still has the frozen sha256.
2. The technical-director reviews the output. The CEO approves the old and
   the new sha256 (design section 5 item 5).
3. Q2 copies the classified new stream to the fixture and changes
   `refutations.py:18`, in one commit. Run `--a2c --stage
   after-fixture-commit` with the same `--new-stream` file. FIXTURE passes
   only when the fixture has the bytes of that file.

In both stages the fixture and the new stream must be two different files.
On one file, FIXTURE cannot fail, so the classifier gives FAIL.

### Checks

| Check | What it checks |
| --- | --- |
| `ALLOWED` | The allowed list follows schema `turts-refreeze-allowed/2`. A position field cannot be allowed. Each entry ends before the calculated arrival tick. `edge_hit` only with `--g1-in-a2c`. |
| `COUNTS-OLD`, `COUNTS-NEW` | Each trace follows the format below: 400 ticks, one world record at each tick, a record for each initial unit at each tick (400 x the initial unit count), op ids 1, 2, 3, ... as JSON integers. An absent, empty or short trace is a FAIL. |
| `EMITTER` | The classifier calculates the sha256 of the `--emitter` files (their bytes, joined in the command-line order). Each of the 8 headers must hold that value. |
| `OLD-HASH` | The old stream has the frozen sha256 (default `bcbf72c2...e370b`). |
| `STREAM-OLD`, `STREAM-NEW` | The stream file is, byte for byte, the projection of the trace of the same run (the bytes that `m3_m2_stream.rs` writes). |
| `P0` | `--a2c`: the traces of base, haul and cedar are byte-equal. `--p0`: all four packages. |
| `P1` | acme: every unit field, every world field and every operation record is equal, except the entries of the allowed list. Each entry must occur. |
| `P2` | All four packages: the full events, with payloads, are equal on each tick. |
| `ARRIVAL` | `ceil(\|P - p0\| / (speed x 1/tick_hz))` from `mods/acme` and `m3_m2_stream.rs` equals the first tick where the scout is on P, in both acme traces. |
| `P3` | The streams of base, haul and cedar are equal. In acme, each old operation is in the new tick, in order, unchanged, with its old operation_id. Each new operation belongs to the scout, has no event kind, and is on tick 1 or on the tick after arrival. Exactly one is added on each of these two ticks. Operation ids increase. |
| `FIXTURE` | See "When FIXTURE runs". |

## Trace format `turts-refreeze-trace/2`

The reference text is the docstring at the top of `refreeze_classify.py`. In
short:

- One JSONL file for each package: `<dir>/base.jsonl`, `haul.jsonl`,
  `acme.jsonl`, `cedar.jsonl`. Each line ends with `\n`.
- Line 1: `{"record":"header","format":"turts-refreeze-trace/2","package":P,
  "emitter_sha256":H,"ticks":400,"unit_count":N,"units":[{"unit":U,
  "content_id":C,"owner_slot":S},...]}`.
- For each tick 1 to 400: one world record, then the unit records in
  ascending unit id, then the operation records in capture order.
- World record: `{"record":"world","package":P,"tick":T,"pools":N,
  "pending_movement_orders":N}`.
- Unit record: `{"record":"unit","package":P,"tick":T,"unit":U,
  "x_bits":"8 hex","y_bits":"8 hex","target":null|{"x_bits":..,"y_bits":..},
  "activity":"Pending|Moving|Resting|Blocked|none","state":"none|plain|pool",
  "order":"none|<Kind>:<stage>","separated":bool,"was_projected":bool}`, plus
  `"edge_hit":bool` in every unit record only with `--g1-in-a2c`.
  `activity` is the `MovementActivity` of the `MoveProgress`, or `none`
  (design section 9 item 7). `state` is `plain` for a `PlainMoveProgress`.
  `order` is the current order kind and its stage (`-` when it has none).
- Operation record: `{"record":"op","package":P,"tick":T,"op_id":N,
  "unit":U|null,"phase":"Command|Orders|PostMovementOrders|Other",
  "events":[EconomyEvent as serde JSON],"before_equals_after":bool}`.

## For task Q1b (the emitter)

From the review of Q1a (simulation-core-engineer):

- Write the trace inside `operation_stream()` of `m3_m2_stream.rs`, at the
  same drain points. Write the stream from that same run, so that STREAM can
  compare the two.
- Sort the units by UnitId. Never use the `iter_entities` order.
- Write each float as `to_bits`, 8 lowercase hex digits.
- No time, path or commit in any line (P0 compares bytes).
- `separated` is a local in three branches (`movement.rs:770`, `:1973`,
  `:2134` at `94a50d3`), and `was_projected` exists only at `:2008`. So the
  emitter adds `was_projected` in the pool branch and in the plain-progress
  branch. Both flags are false for a unit that does not step and on the snap
  paths.
- The phase of the operation on the tick after arrival comes from a probe of
  the real run. The synthetic self-test uses `PostMovementOrders`; the real
  phase is probably `Orders` (NOT TRACED).
- The EMITTER value is the sha256 of the emitter files, joined in the order
  that the classifier gets them with `--emitter`. Use the same files and
  the same order in the header.
- Each allowed-list entry must end before the arrival tick. If the real run
  differs from the old run at or after the arrival tick (for example, an
  order that is cleared one tick later), the classifier gives FAIL and the
  stop rule of design line 146 applies.
