# Pre-MP1 slice: audit crate, one movement model, neighbour grid, file splits: design

Date: 2026-11-12. Revision 1. Status: DRAFT, before its gate (two lenses: qa-lead, simulation-core-engineer).
Author: the orchestrator, from the technical-director input `docs/research/mp1/input-technical-director.md`
(parts A and C) and the producer input `docs/research/mp1/input-producer.md` (part 2). The inputs are agent
output; each claim below that this design depends on carries its `path:line` and is checked again at the gate.

## 1. Why this slice exists

The CEO decided on 2026-11-11 (`vault/Decisions/2026-11-11 MC architecture audit.md`):

1. The economy log validator checks ledger conservation only, and moves to a `turts_audit` crate with one file
   for each domain.
2. One movement model: a plain move is a pool of one. The M2 stream is frozen again one time.
3. Both before MP1.

The slice also holds two items of the MC architecture audit: the neighbour grid cell must not depend on the
largest unit, and the files that every MC task edited are split, so that MP1 lanes do not wait for each other.

The slice changes no rule that a player sees. It adds no mod API. It is not part of MC: the MC frozen stream hash
`bcbf72c2206251f635a7c7f5b6c8f573d69a5fc05a5bd77716e98ebfb08e370b` stays until step A2 of this slice.

## 2. Parts and order

Order: S (pure moves) -> A1 -> A2 with the one re-freeze -> A4 -> A3. The S moves on client crates run in
parallel with all other parts.

| Part | What | Kind | Proof |
| --- | --- | --- | --- |
| S1 | `turts_audit` crate created; the validator moves to it unchanged (A1 step 1) | pure move | moved-code diff; all gates; boundary rows with a red fixture |
| S2 | `turts_sim` `orders.rs` to an `orders/` folder (types, pending, plain, run, apply, behaviours); tests out | pure move | moved-code diff; all gates; stream unchanged |
| S3 | `turts_sim` `movement.rs` to a `movement/` folder (spread, search, plain, pool_index, frame, ecs); tests out | pure move | as S2 |
| S4 | `turts_interaction/src/lib.rs` to selection, groups, smart, compose, bindings, world_view | pure move | moved-code diff; all gates |
| S5 | `turts_client` `visuals.rs`, `client_shell.rs`, `widgets/mod.rs`, `core_adapter.rs`: tests out, then the file splits of the input part C5 to C7 | pure move | moved-code diff; all gates; the pixel rows that read the shell and the HUD |
| A1 | `LedgerAudit` view; ledger validator with one file for each domain; both validators run side by side; then the replay validator is deleted; the fault list of section 4 | behaviour of the test oracle | one planted fault for each domain; E8 re-pinned; M3 C1 to C17 stay red |
| A2 | single moves take the pooled path; the plain model is deleted | sim behaviour for a move to a reserved point; audit operations for a single move | the classifier of section 5; the guard tests of section 6 |
| A4 | `step_frame_preserving_resting` split into phases with a `FrameState` | refactor | reference copy of the old kernel, bit-equal on seeded frames; stream unchanged |
| A3 | two neighbour grids (movers, statics); combat grid cell from the smallest weapon range | performance only | reference copy, bit-equal; a property test of `within` against a brute-force scan; the RUN126 probe time recorded |

A pure move is checked with
`git diff -M --color-moved=dimmed-zebra --color-moved-ws=allow-indentation-change`: no line outside `mod`, `use`
and visibility lines may show as added or removed.

S2 and S3 start only after the MC repair of gather at node depletion (branch `codex/mc-fix-deplete`) is merged,
because that repair edits `orders.rs` and `economy/gather.rs`.

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

1. Before A2, qa-lead adds a trace that is not frozen: for each tick and unit the position bits, the target and
   the activity; for each operation the unit and the phase. It runs on the build before A2; its sha256 is logged.
2. The builder lands A2 and does not touch the fixture.
3. A classifier (Python standard library) checks: P1 every position is bitwise equal for the four packages over
   400 ticks; P2 every economy event kind falls on the same tick; P3 every changed operation has no event kind,
   belongs to the `acme` scout, and falls on the accept tick or the arrival tick; and an independent calculation
   of the arrival tick, `ceil(|P - p0| / (speed x dt))`.
4. Classifier self-test: one flipped position bit gives red; one added `Stored` event gives red.
5. The technical-director reviews the classified difference. The CEO approves the old and the new sha256 in the
   vault. Then the fixture and `refutations.py:18` change in one commit.

Any difference outside P3 is a defect until it has a cause. "Regenerate with the current code" is refused.

## 6. Guards (A2)

- `one_unit_alone_takes_the_point` (`group_movement.rs:1309`) stays, and gains cases: through `MoveTo`; through
  an envelope with group 0; with a static unit of radius 4 far away; to a point off the map (the clamped point,
  bitwise).
- T7 (the unit rests within 0.011 of the ordered point) stays.
- The tests that pin plain moves are listed in the input (part A2); each one is kept, inverted with a reason, or
  deleted with a reason, in the plan.

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
