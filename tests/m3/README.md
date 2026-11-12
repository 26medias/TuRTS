# tests/m3

- `m2_operation_stream.json`: the frozen M2 operation stream. Its test is
  `crates/turts_pack/tests/m3_m2_stream.rs`. Its sha256 is in
  `refutations.py:18`. It changes one time only, at the pre-MP1 re-freeze
  (task Q2).
- `refutations.py`, `refutations.json`: the M3 fault registry and runner.
- `refreeze_classify.py`: the re-freeze classifier (pre-MP1 slice, tasks Q1a
  and Q1b).
- `test_refreeze_classify.py`: its self-test (synthetic runs and plants).
- `refreeze_allowed.json`: the list of the allowed acme differences (task Q1b;
  committed before the A2c commit, plan refinement R6). Its sha256 is logged
  with the sha256 of the emitter `crates/turts_pack/tests/m2_trace.rs`.

## Re-freeze classifier

Design: `docs/superpowers/specs/2026-11-12-pre-mp1-slice-design.md`, section 5
and section 9 items 6, 7, 14 and 24 (revision 6).

### Commands

    python3 tests/m3/refreeze_classify.py --self-test
    python3 tests/m3/test_refreeze_classify.py

    # Write the traces of one build (the emitter, task Q1b). The test needs the
    # cargo feature test-probes of turts_sim; the dev-dependency of turts_pack
    # turns it on. DIR must be an absolute path (the test refuses a relative
    # one) and receives base.jsonl, haul.jsonl, acme.jsonl,
    # cedar.jsonl and m2_operation_stream.json (the stream of the same run).
    TURTS_M2_TRACE_OUT=DIR cargo test -p turts_pack --locked --test m2_trace \
        -- --ignored --exact m2_trace_writes_four_packages

    # The counts of design 9.6 and 9.8 for one run, and (e) the unit-ticks of
    # each gather_stage. FAIL when a unit has state plain, or (b) or (c) is
    # above 0.
    python3 tests/m3/refreeze_classify.py --counts DIR

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
| `ALLOWED` | The allowed list follows schema `turts-refreeze-allowed/4`. A position field cannot be allowed. Each entry ends before the calculated arrival tick; for `order`, `queue` and `last_failure` it can end on the arrival tick (design 9.24a). Of an old operation, only `move_to_route` can change. `edge_hit` only with `--g1-in-a2c`. |
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

## Trace format `turts-refreeze-trace/4`

The reference text is the docstring at the top of `refreeze_classify.py`. In
short:

- One JSONL file for each package: `<dir>/base.jsonl`, `haul.jsonl`,
  `acme.jsonl`, `cedar.jsonl`. Each line ends with `\n`.
- Line 1: `{"record":"header","format":"turts-refreeze-trace/4","package":P,
  "emitter_sha256":H,"ticks":400,"unit_count":N,"units":[{"unit":U,
  "content_id":C,"owner_slot":S},...]}`.
- For each tick 1 to 400: one world record, then the unit records in
  ascending unit id, then the operation records in capture order.
- World record: `{"record":"world","package":P,"tick":T,"pools":N,
  "pending_movement_orders":N}`. `pending_movement_orders` counts units: the
  sum of `PendingMovementOrders.pending_units` (`crates/turts_sim/src/orders.rs`,
  struct at `:75-82`).
- Unit record: `{"record":"unit","package":P,"tick":T,"unit":U,
  "x_bits":"8 hex","y_bits":"8 hex","target":null|{"x_bits":..,"y_bits":..},
  "activity":"Pending|Moving|Resting|Blocked|none","state":"none|plain|pool",
  "order":"none|<Behaviour>:<stage>","gather_stage":"none|<stage>",
  "queue":[<Behaviour>,...],
  "last_failure":null|{"order":C,"reason":R,"tick":N},"separated":bool,
  "was_projected":bool|null}`, plus `"edge_hit":bool` in every unit record
  only with `--g1-in-a2c`.
  - `activity` is the `MovementActivity` of the `MoveProgress`, or `none`
    (design section 9 item 7). `state` is `plain` for a `PlainMoveProgress`.
  - `order` is `UnitOrders.current`: the `Behaviour` name (never the content
    id) and the gather stage of the unit (`-` when it has none). The stage
    names are in the docstring. A legacy Gather command leaves `current`
    empty, so a gather unit reads `none`.
  - `gather_stage` (format /4) is the stage of the gather cycle, read through
    `turts_sim::order_view` (the economy view), or `none` when the unit has
    no gather cycle. It shows the stage of a legacy Gather, which `order`
    cannot. Task A2b1 changes the gather protocol, so the stage is a P0 field:
    a difference is red unless the list names it. The base, haul and cedar
    traces hold the stages Idle, MovingToNode, Gathering, MovingToReceiver and
    Unloading; the acme trace holds `none` only.
  - `was_projected` (format /4) is the real value of the hard projection in
    the last frame: `true` when at least one of its passes moved the unit,
    `false` when the unit ran the projection loop and no pass moved it. Each
    of the three branches (no pool, pool, plain) stores its own value after
    its loop. It is `null` when the unit did not reach a projection loop in
    that frame: no movement, a snap to the target before the loop, an
    escape step. In formats /1 to /3 it was `null` for a pool unit and `false`
    for a plain unit, which hid two real projections.
  - `move_to_route` (on the operation record of a `MoveTo`) is read as
    follows (`observe_move_to_route` in the emitter). Direct against pending
    comes from the world: `direct` when nothing is pending and the unit has its
    `MoveTarget` right after the command; `none` when nothing is pending and it
    has no `MoveTarget`; a pending count above 0 means a pending envelope.
    `per_unit` against `envelope` comes from the shape of the command, not from
    the world: more than one unit in `unit_ids` reads `per_unit`, one unit reads
    `envelope`. The pending count is global: it is the sum of
    `PendingMovementOrders.pending_units` over all slots, so a pending envelope
    of another slot or of another command would make a `direct` command read
    `per_unit` or `envelope`. The streams hold one `MoveTo` (acme), so the
    count is not mixed there.
- Operation record: `{"record":"op","package":P,"tick":T,"op_id":N,
  "unit":U|null,"phase":"Command|Orders|PostMovementOrders|Other",
  "events":[EconomyEvent as serde JSON],"before_equals_after":bool}`, plus
  `"move_to_route":"direct|per_unit|envelope|none"` on the operation of a
  `MoveTo` command only. The emitter reads the route from the world right
  after the command, not from the shape of the command.

## The emitter (task Q1b)

- `crates/turts_pack/tests/m2_trace.rs` repeats the run loop of
  `m3_m2_stream.rs` (the same commands and drain points) and writes the trace
  and the stream of the same run. The stream bytes equal the frozen fixture on
  the base build (sha256 `bcbf72c2...e370b`).
- The `emitter_sha256` of every header is the sha256 of `m2_trace.rs` alone.
  Pass that file to `--emitter`. The probe (`crates/turts_sim/src/movement/probe.rs`)
  is not part of the hash: its body changes at A2b2, and the emitter must be the
  same file on both builds of a P0 pair (plan R11).
- The emitter reads the movement state only through
  `turts_sim::movement::probe` (cargo feature `test-probes`). It never names
  `PlainMoveProgress`.
- The stream (`m3_m2_stream.rs`) never sets `separated` or `was_projected` and
  never uses a pool or a plain progress. A plant in the probe is therefore not
  seen by `--p0` on the stream. The crowd cases of
  `crates/turts_sim/tests/probe_flags.rs` (feature `test-probes`) are the
  instrument for the probe values: each of the three branches (no pool, pool,
  plain) has an assertion that reads both `true` and `false`.
- `crates/turts_sim/tests/frame_hash.rs` prints two hash lines.
  `scripts/qa/probe_neutral.sh` runs it with the feature off and with
  `--features test-probes`, compares the lines, and exits 1 on a difference
  (design 9.20). It is a step of the `check` job of `.github/workflows/ci.yml`.
  The hashes are not pinned: they use the libm of the platform. Since Q1b every
  workspace test builds `turts_sim` with the feature on, so this script is the
  only check of the build without it.
- `scripts/boundary.sh` checks that the shipped server and client builds do not
  turn the feature on (`cargo tree -p CRATE -e normal,build,features -i
  turts_sim`), and that the same grep finds the feature on the dev edge of
  `turts_pack` (the self-check that the grep can go red).
- `TURTS_M2_TRACE_OUT` must be an absolute path. The test refuses a relative
  path: it would resolve under `crates/turts_pack/` and write untracked files
  into the tree. Write the traces under `target/`.
- A failure of `--p0` names the first differing record: tick, unit (or world
  or operation) and field, with the old and the new value.

## Phases and routes found by the probe (Q1b)

- The operation on the tick after arrival has the phase `Orders` (a pooled
  single move of the scout, group 7: operation 3 at tick 124). The README of
  Q1a guessed `PostMovementOrders`.
- `UnitOrders.current` of that move is `Move` at ticks 1 to 123 and empty at
  tick 124. The old single `MoveTo` has an empty `current` on every tick.
