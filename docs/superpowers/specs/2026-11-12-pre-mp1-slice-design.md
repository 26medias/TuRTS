# Pre-MP1 slice: audit crate, one movement model, neighbour grid, file splits: design

Date: 2026-11-12. Revision 3. Status: DRAFT. Revision 3: the qa-lead lens FAILED revision 1 with 4 High (docs/research/mp1/gate-slice-qa.md); sections 2, 4 and 5 take its text changes; a re-check by both lenses follows. Gate round 1: the simulation lens FAILED revision 1 (one High: a third movement branch); the technical-director ruled (docs/research/mp1/ruling-td-one-step-function.md); sections 1, 2, 5 and 6 are replaced. The qa-lead lens is in progress.
Author: the orchestrator, from the technical-director input `docs/research/mp1/input-technical-director.md`
(parts A and C) and the producer input `docs/research/mp1/input-producer.md` (part 2). The inputs are agent
output; each claim below that this design depends on carries its `path:line` and is checked again at the gate.

## 1. Why this slice exists

The CEO decided on 2026-11-11 (`vault/Decisions/2026-11-11 MC architecture audit.md`):

1. The economy log validator checks ledger conservation only, and moves to a `turts_audit` crate with one file for each domain.
2. One movement model: a plain move is a pool of one. The M2 stream is frozen again one time.
3. Both before MP1.

"One movement model" means this (technical-director ruling, 2026-11-12):

- One step function moves every unit that has a `MoveTarget`. It does arrival, steering, projection, stall and `Blocked`, and the obstacle fallback. M4 routes and the M4b push go into this function only.
- A pool (`MoveProgress` and a `MovementPool`) is optional data on the unit. It chooses the destination slot and turns on the group rules (MC design 6.3 to 6.5).
- An order makes a pool: every move envelope, the plain move of MC design 6.1 (`:568`), and `CommandKind::MoveTo`.
- An engine-made trip has no pool: gather trips, queued starts, the attack approach, follow, resume and `--synthetic-units`. When such a trip finds its goal occupied (`movement.rs:1882-1892`), or is pushed and does not arrive (`:2052`), the step function gives it a pool of one with the same spread code.
- `PlainMoveProgress` is deleted.

The slice also holds three items of the MC architecture audit and its gate:

- the neighbour grid cell must not depend on the largest unit;
- `within` must find every point inside `reach` (`grid.rs:63`);
- the files that every MC task edited are split, so that MP1 lanes do not wait for each other.

The slice changes one behaviour that a player can see, and no other. Before: when a unit was sent to a point that another moving unit went to, the first unit gave the point up. After: the first unit keeps the point, and the later unit takes a free slot next to it (MC design 6.3 step 2, `:612-614`). An engine trip that finds its goal blocked uses the same free-slot rule. Above 2,048 single moves in one tick, the rest start one tick later (6.3 work cap). The CEO is told before step A2b.

The slice adds no mod API. It is not part of MC. The MC frozen stream hash `bcbf72c2206251f635a7c7f5b6c8f573d69a5fc05a5bd77716e98ebfb08e370b` stays until step A2c.

## 2. Parts and order

Order: the depletion repair (`codex/mc-fix-deplete`) is merged -> G1 -> S1 -> S2, S3 -> A1 -> A2a -> A2b -> A2c with the one re-freeze -> A4 -> A3. S4 and S5 run in parallel with all other parts.

| Part | What | Kind | Proof |
| --- | --- | --- | --- |
| G1 | `within` scans the cells of the bounding box of the query disc, made larger by 1e-4 x max(1, reach) (`grid.rs:61-70`) | defect repair | Property test against a brute-force scan (24,000 queries, whole-number reach/cell included): 0 misses, 0 cell-size-dependent results. An edge counter on the stream and the golden tests: 0 hits, or G1 moves into A2c (section 5) |
| S1 to S5 | As in revision 1 | pure move | As in revision 1 |
| A1 | As in revision 1 | test oracle | As in revision 1 |
| A2a | One step function for the no-progress branch (`movement.rs:1932-2085`) and the pool branch (`:2086-2380`). Where they compute one value with different float expressions, keep the no-progress expression (for example `:2003-2004`). No writer changes | refactor | Reference copy of the old kernel, bit-equal on seeded frames (no-progress, plain-progress, pooled, escape, `Blocked`, follow, Attack-Point units). Stream unchanged. GM40, GMA, GMU and `group_movement.rs` green; each changed assertion has a reason. The call-counter test |
| A2b | The obstacle fallback (`:1893`, `:2053`) makes a pool of one with `spread_destinations_with_context`. Delete `step_plain_progress` (`:671-905`), `make_plain_progress` (`:925`), `PlainMoveProgress` (`:58`) and the plain reservations in `orders/`. `PlainGatherGoal` (`:63,79-82`) moves to `MoveProgress` | sim behaviour for a blocked trip | Stream unchanged (the streams spend 0 unit-ticks on `PlainMoveProgress`). Guards of section 6 |
| A2c | The plain move (`orders.rs:2822-2900`) and `CommandKind::MoveTo` (`command.rs:488-495`) take the pool path at dispatch | player-visible change of section 1; audit operations for a single move | Section 5 |
| A4 | The rest of `step_frame_preserving_resting` becomes functions on `&mut FrameState`, called in one loop in unit-id order | refactor | Reference copy, bit-equal; stream unchanged |
| A3 | Two neighbour grids (movers, statics); combat grid cell from the smallest weapon range | performance only | Reference copy, bit-equal; property test; RUN126 probe time recorded |

A pure move is checked with `item_equal.py` (`docs/research/mp1/probes/item_equal.py`): the multiset of the
top-level items of the old files, with visibility removed, equals that of the new files, except `mod` and `use`
items. The names of `cargo test -- --list` are the same before and after. The colour-moved git diff is a
reading aid only: the gate showed that it hides a swap of two statements.

"Bit-equal" in parts A2a, A4 and A3 means: every f32 of the frame output compared with `to_bits` (0.0 and
-0.0 differ, because `movement.rs:481,964` use `atan2` and `total_cmp`); 64 fixed seeds, each chained for 300
ticks; 1 to 5,000 movers; 0 to 20 statics of radius 0.5 to 8; a `preserve_resting` set that is not empty; a
counter for each phase that must be above 0. Plants that must go red: a swap of two phases; a `FrameState`
field that is reset; a -0.0; statics queried with reach `r` only; a merge that is not sorted; a speed of
`Some(0.0)`.

## 3. `turts_audit` boundary

- `turts_audit` (library) depends on `turts_sim` (audit types), `turts_net_protocol` (`CommandRejectReason`,
  `economylog.rs:14,43`), `turts_mod_api`, serde, serde_json.
- Users: `turts_server` (the sink writes `EconomyLogRecord`); dev-dependencies of the `turts_server` and
  `turts_pack` tests.
- New rows in `scripts/boundary.sh`: `turts_audit` may not depend on `bevy_render`, `bevy_winit`, `bevy_pbr`,
  the client crates, `turts_server`, `turts_pack`, `turts_mod_runtime`, `turts_net_lightyear`; `turts_sim`,
  `turts_client`, `turts_client_web` and `turts_interaction` may not depend on `turts_audit`. Each new row has a
  fixture case that goes red. How `tests/boundary-fixture` selects its rows is NOT TRACED; the builder traces it.
- Files: `lib.rs`, `format.rs`, `clock.rs`, `slots.rs`, `balances.rs`, `inventories.rs`, `reservations.rs`,
  `production.rs`, `industry.rs`, `combat.rs`. The writer and the sink stay in `turts_server`.
- The ledger oracle of the tests (`crates/turts_pack/tests/common/conservation.rs`) is the seed of the domain
  files, and is then replaced by `turts_audit`, so one oracle remains.
- `AuditState` (`views.rs:374`) keeps all its fields in A1: the frozen stream compares the full `AuditState`
  before and after each operation (`m3_m2_stream.rs:119`). The log writes a new `LedgerAudit` view; a sim
  structure enters the log format only through an explicit edit of `LedgerAudit`.

## 4. Faults, rows and readers that change in A1

A fault with no red after the change is a defect of this slice. The gate ran the fault sets with the replay
validator disabled (`docs/research/mp1/gate-slice-qa.md`): M3 17 of 17 red (they cannot reach the validator);
native 29 of 33 red; the 4 others are the first two items below.

1. The live-log reader. `economy.rs:1792-1870` (`read_live_complete_tick_prefix`) finds an incomplete tail from
   the exact error text of the validator (`economylog.rs:1085`, matched at `economy.rs:1846`). With the validator
   off, an incomplete tick reaches the oracle and the E9 and E8 rows can go flaky. The reader finds the
   incomplete tail itself (the last `Tick` record, or an open `CommandBoundary`) and reads no validator text.
   This lands before A1 step 3. Proof: the E9 loss cases green 5 times in a row.
2. The three E8 cases share `e8_commands_are_authoritative_and_acked`, which holds validator self-checks
   (`economy.rs:3293-3345`). `E8-mixed-cost-partial-commit` is pinned to the text of `economy.rs:3197`
   ("rejected seq 5 is write neutral"), which is not in the validator and is red today. The self-checks
   `:3293-3345` become plants of `reservations.rs` with the new messages, in the same commit that deletes the
   replay validator.
3. Readers of the log. The browser audit oracle reads `units[].pos`, `facing`, the gather order `stage` and the
   node `pos` (`tests/browser/m2-audit.js:67-120`); EL1 reads `movement_progress`, `plain_movement_progress`,
   `physical_escapes` and `unit_orders` (`el1.rs:188-215`). `LedgerAudit` keeps every field that these two
   read. A1 step 3 runs the browser fault set (8 cases) and EL1.
4. Cover that the replay validator gives today and that must stay:
    - A refused command or operation leaves the full logged view unchanged (`economylog.rs:2346-2350`). This
      rule stays in `lib.rs`; it needs no movement model. "Changes no account" alone is weaker.
    - A unit that moves with no order: the `turts_checks` scan covers writes to `Position`, `UnitOrders`,
      `MoveTarget`, `MoveProgress` and the pools, with an allow list of files, and a planted fixture that goes
      red.
    - `mc_orders.rs:469` asserts nothing but the validator today. It gets an outcome assertion from the order
      log before the replay validator is deleted; the same holds for the other "strictly replays" tests
      (`mc_orders.rs:574, 694, 824, 1089, 1094`) and `session.rs:1021`.
5. Row EL1 of the MC design: "movement input missing" and "the validator of today" become void; the row text
   changes to: the ledger validator accepts each tick, and the run records the log bytes.
6. The plants for each domain file are rows of the CI fault registry, not only local tests.
7. Both validators run side by side from A1 step 2 until every item above is green on one commit; that commit
   is the exit condition; the next commit deletes the replay validator.

## 5. The one re-freeze (A2)

0. If the G1 edge counter has hits on the stream, G1 lands in A2c. The trace then records `edge_hit` for each query, and P0 and P1 accept a difference only on a flagged query.
1. After A2b, `qa-lead` adds a trace that is not frozen:
    - for each tick and unit: the position bits, the target, the activity, the movement state (no pool or pool) and the `separated` flag;
    - for each operation: the unit and the phase.
   It runs on the build before A2c, and its sha256 is logged.
2. The builder lands A2c and does not touch the fixture.
3. A classifier (Python standard library) checks:
    - P0: for `base`, `haul` and `cedar`, the trace before and after is equal byte for byte.
    - P1: in `acme`, every position is bitwise equal over 400 ticks.
    - P2: in all four packages, every economy event kind falls on the same tick.
    - P3: every added or changed operation is in `acme`, belongs to the scout, has no event kind, and falls on the accept tick or on the tick after arrival (Orders runs before Movement, `plugin.rs:97-98`). Expected: one added operation at tick 1 and one at the tick after arrival (gate probe: arrival 123, operation 124).
    - An independent calculation of the arrival tick, `ceil(|P - p0| / (speed x dt))`, must equal the arrival tick in the trace.
4. Classifier self-test. Each of these must give red: one flipped position bit; one added `Stored` event; one changed operation in `base`; one scout operation on the arrival tick itself.
5. The technical-director reviews the classified difference. The CEO approves the old and the new sha256 in the vault, together with the player-visible change of section 1. Then the fixture and `refutations.py:18` change in one commit.

The classifier compares more than positions, because the stream holds, for each tick, the operation count
and the ordered operation id, event kinds and the before-equals-after flag of each operation
(`m3_m2_stream.rs:19-36, 108-126, 361-374`):

- P1 compares every trace field in `acme`, against a list of the allowed differences that is written before
  A2c lands.
- P2 compares the full events with their payloads, not only the kinds.
- The trace has 4 packages x 400 ticks x the initial unit count; the classifier asserts these counts, so two
  empty or short traces cannot pass.
- Both runs log the sha256 of the trace emitter; the two must be equal.
- The committed fixture must hash to the stream that the classifier read; a fixture written by
  `UPDATE_M3_BASELINE` (`m3_m2_stream.rs:384`) from another run is refused.
- More self-test plants that must give red: two operations of one `acme` tick in the other order; a flipped
  before-equals-after flag on the tick-1 operation; an empty trace; a trace of 399 ticks.

A difference outside P3 stops A2c. A new ruling and a new run of the self-test are needed before it continues. "Regenerate with the current code" is refused. G1, A2a and A2b change no frozen value. If the stream test goes red on one of them, that step stops for a new ruling.

## 6. Guards (A2)

- `one_unit_alone_takes_the_point` (`group_movement.rs:1309`) stays. It gains these cases: through `MoveTo`; through an envelope with group 0; with a static unit of radius 4 far away; to a point off the map (the clamped point, bitwise).
- Second move to a held point (gate case S12). Unit 1 moves to P; unit 2 gets a move to P while unit 1 still moves.
    - After A2c: unit 1 rests on P, bitwise. Unit 2 rests at least s - 0.00001 from P (s = 2 x r_max + 0.1). No footprints overlap.
    - The guard is red on the build before A2c. Both runs are recorded.
- One step function (A2a). A counter only in test builds (the pattern of `movement.rs:13-21`) counts the calls to the step function for each unit and tick.
    - The scenario has: a gather trip, a queued start, an attack approach, a follow, a resume, a single envelope move, a group move and a synthetic unit.
    - It asserts one call for each moving unit in each tick.
    - It is red on a build that keeps a second step branch.
- Blocked engine trip (A2b). Two gatherers deliver to one receiver in the same tick: both deliver, no footprints overlap, and each delivers within 2 x (distance / speed) + 5 s. A delivery after node depletion is also checked (gate L2).
- Pool collection. After a single move arrives: 0 `MoveProgress` and 0 pools (`movement.rs:2383-2404`, `:2497-2508`).
- G1. The property test against a brute-force scan.
- T7 (the unit rests within 0.011 of the ordered point) stays.
- The tests that pin plain moves are listed in the input (part A2). Each one is kept, inverted with a reason, or deleted with a reason, in the plan.

## 7. Not in this slice

- `turts_rules` (pure placement, relation and damage rules) and the schema and wire freeze: MP1 lane 0.
- The behaviour handler table of `orders/` and the split of `Core::step`: MP1 lane 0, after the pure moves.
- The UnitId-to-Entity index: MP1 (automatic targeting needs it).
- `WorldDelta.unit_orders` changes only: the MP1 protocol bump.

## 8. Open points for the gate

1. Is the claim "only E8 gets its red from the validator" true? Run the three fault sets against a build where
   the replay validator accepts everything.
2. Does a pooled single move dispatch in the same tick, and is the pool of a lone unit collected after arrival
   (both NOT TRACED)? If not, P1 fails and A2 needs a design change.
3. The f32 cell-edge case of the grid (`grid.rs:63,120-124`): can `within` drop a point at exactly `reach`?
4. Is the order "A1 before A2" enough to keep the validator out of A2, or does A2 still touch `LedgerAudit`
   (`AuditState.plain_movement_progress` goes away)?
