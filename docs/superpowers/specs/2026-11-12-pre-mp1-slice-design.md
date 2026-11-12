# Pre-MP1 slice: audit crate, one movement model, neighbour grid, file splits: design

Date: 2026-11-12. Revision 2. Status: DRAFT. Gate round 1: the simulation lens FAILED revision 1 (one High: a third movement branch); the technical-director ruled (docs/research/mp1/ruling-td-one-step-function.md); sections 1, 2, 5 and 6 are replaced. The qa-lead lens is in progress.
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

A pure move is checked with `git diff -M --color-moved=dimmed-zebra --color-moved-ws=allow-indentation-change`. No line outside `mod`, `use` and visibility lines may show as added or removed.

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

## 4. Faults and rows that change in A1

A fault with no red after the change is a defect of this slice.

- `E8-mixed-cost-partial-commit` (`tests/m2/refutations.json:768`): its red comes from `economylog.rs:2349`
  today. Pin it to the new rule "a refused command or operation changes no account".
- The other 40 M2 cases and the 17 M3 cases keep their reds (sources: `accounting.rs`, `conservation.rs`,
  `turts_server/tests/economy.rs`, `m2-oracle.js`). The gate checks this claim by a run of the three fault sets.
- Row EL1 of the MC design: "movement input missing" and "the validator of today" become void. "A write to
  `Position` or `UnitOrders` outside the movement function and `capture_operation`" becomes a `turts_checks` scan
  with a planted fixture. The row text changes to: the ledger validator accepts each tick, and the run records
  the log bytes.
- `mc_orders.rs` tests that say "strictly replays" must assert the order outcome from the order log. If they
  do not do so today (NOT TRACED), A1 adds the assertions before the replay validator is deleted.

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

Any difference outside P3 is a defect until it has a cause. "Regenerate with the current code" is refused. G1, A2a and A2b change no frozen value. If the stream test goes red on one of them, that step stops for a new ruling.

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
