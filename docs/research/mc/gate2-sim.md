VERDICT=pass-with-changes FINDINGS=8 HIGH=3 RUNS=8 NOT_CHECKED=5

# Gate 2, simulation lens: MC plan `f69badf`

Artefact: `docs/superpowers/plans/2026-10-29-mc-player-control.md`. Scope: tasks 2a, 2b, 4, 5, 6, 7, 18, 19, 22.
Evidence: `/tmp/claude-1000/-home-julien-Projects-TuRTS/0e0457bb-045c-4b6a-945f-1db6a0753ee2/scratchpad/mc/gate2/sim/`.

## Findings

1. HIGH. Plan task 4 files list (`economylog.rs:578-663`, "keep `movement_pending`, `:326,553-556`"). The first non-command operation of a tick consumes `movement_pending` (`crates/turts_server/src/economylog.rs:369-372`). `SimSet::Orders` writes operations before `Movement`, so the validator applies the movement transition to the wrong state and rejects a correct log. Probe: one no-op operation before `SimSet::Movement` gives `Err("unit 1 movement differs from its prior target and speed")`; control gives `Ok(())`. Step 4 (a queued move) cannot go green from a switch to `step` alone. Its predicted red (`:660`, "snapshots disagree") is also not the message the probe gives. Fix: state the rule. Validate Orders-phase operations by clock continuity, and apply the movement transition after the last Orders operation. That needs a phase mark on `OperationAudit` or an equal rule.
2. HIGH. Task 22 step 1 (DET1, `cmp` of two `--tick-log` files). Each line holds `wall_ns` from `SystemTime::now()` (`crates/turts_server/src/ticklog.rs:64-66,82`). Two acme runs of 20 ticks: `cmp` says "differ: byte 12"; with column 2 removed, `cmp` exits 0. So DET1 is red on every build and "restore; re-run: exit=0" cannot happen. Fix: compare `cut -d, -f1,3-`, or add a deterministic log. The design row says "byte-equal" (D section 14 DET1), so the orchestrator must rule on that text.
3. HIGH. Task 22 steps 2 and 3 read "0 `Blocked`" (GM40, GMU) and the `Blocked` count (GMA) from the tick log. The tick log has no activity column (`ticklog.rs:82`). The `Blocked` half cannot go red. Fix: name the source, for example `AuditState` activity in the EL1 economy log, a `--stats` counter, or the owner `UnitOrderRecord` through a headless client.
4. MEDIUM. Task 4 does not say how a plain move is stored. If a plain move sets `UnitOrders.current`, then its completion is an order write. In `capture_operation`, that write adds an operation and the acme stream goes red (the fixture holds 1 operation for acme). Outside `capture_operation`, EL1 goes red. Fix: a plain move writes `MoveTarget` only, `UnitOrders` stays default, and `Moving` comes from `MoveTarget`. Also say whether `to_sim` sends a plain move as `CommandKind::MoveTo` or as `Order`. The frozen stream calls `MoveTo` and `Gather` directly (`crates/turts_pack/tests/m3_m2_stream.rs:244-253,302-311`), so it covers only the path that the server really uses.
5. MEDIUM. Task 6 `pending_above_2048_per_tick` and task 4 `queued_order_starts_in_the_completion_tick` have names but no assertion. The plan does not say if (slot, group, tick) merges before the cap, if `group` 0 merges, or if a waiting envelope is atomic. Fix: give the expected tick schedule for two slots. Give the tick of a completion: the unit lands in `Movement` of tick t, then `Orders` of tick t+1 completes and starts the next order.
6. MEDIUM. Plant sensitivity, NOT TRACED. The EL1 plant "drop one `step` input" stays green if the input is the stall timer, the pass mode or the claim count, because GM40 has no stall. Name the group id or the slot. The DET1 `HashMap` plant changes nothing if no unit has 2 or more neighbours in reach in one tick. Add an instrument self-check, or run the GMA script too.
7. MEDIUM. The G-REF re-anchor rule allows only `anchor` and `replacement`. Task 5 moves the candidate scan to `grid.rs`, so C16 (`combat.rs:191`) can need a new `plant_path`. E8-rejection-ack-send-reversal expects a `CommandRejected` text that task 2b removes. Fix: also allow `plant_path`, `expected_failure` and `command`, with a `qa-lead` review.
8. LOW. Task 7: the bucket clock (tick or wall time), its place (receipt or `admit_player_commands`, `session.rs:271-307`), and its order relative to the seq check are not given. The plan also does not say if `--command-script` passes the buckets. A `rate_limited` refusal through `record_admission_rejection` writes an admission record (`session.rs:433-510`). Recommendation: refill per tick, 640 / `tick_hz` (D18 keeps the tick rate open).

Confirmed: `SimSet::Commands` is in `FixedPreUpdate` and the `FixedUpdate` chain starts at `Movement` (`crates/turts_sim/src/plugin.rs:88-101`), so the plan's insertion point is correct. A plain move and a gather order write no operation today: `apply_move_to` (`command.rs:212-253`), and no `transact` in `gather.rs:88-175`. The fixture holds acme 1 operation in 400 ticks. In the frozen scenarios, every mover keeps a gap of at least 0.5 to every other unit (more than the reach of 0.05). The smallest step is 0.1 (more than 0.025), and no target needs the clamp. So "no neighbour keeps today's step" holds, if "neighbour" means inside the separation reach. In base, haul and cedar, movers pass inside one grid cell.

## What I ran

| command | claim tested | result that would refute | observed |
| --- | --- | --- | --- |
| `cargo test -p turts_sim --locked` | baseline green (plan G-RUST) | a fail | 176 passed, 0 failed, 10 suites, exit=0 |
| `cargo test -p turts_pack --locked --test m3_m2_stream --test forge_fixed_point` | G-FROZEN green at base | a fail | 2 passed, exit=0 |
| `sha256sum tests/m3/m2_operation_stream.json` | hash = `refutations.py:18` | another hash | `bcbf72c2...e370b`, equal |
| `python3 tests/m3/refutations.py --help`, m2 too | plant runner, `--case` | no list mode | `--case` only; wrapper takes `ID[,ID...]`; anchors must occur once (`tests/m2/refutations.py:270-280`) |
| scratch clearance probe (`probe.log`) | task 6 step 2, "no neighbour keeps the float step" | a gap below 0.05, a step below 0.025, or a clamp | min gap 0.5; min step 0.1; 0 clamps; exit=0 |
| scratch validator probe (`probe_el_*.log`) | task 4: keep `movement_pending` | an Orders-phase operation still validates | control `Ok(())` exit 0; plant `Err(...movement differs...)` exit 101 |
| two `turts_server --tick-log --run-ticks 20` on acme, `cmp` | task 22 DET1 byte-equal | equal on a correct build | differ at byte 12; `cut -f1,3-` equal |
| `awk` for `transact` in `gather.rs:88-175` | design 6.1 citation | a match | none |

## Not checked

The lightyear message size (task 2b step 1). The K4 split in the client. The GM40 pixels oracle (`clip_from_world`, discs); this is the rendering seat. The task 7 rejoin. The tasks 18 and 19 package contents and the scan collisions (`move`).

## What I changed

This file only. The probes are in the scratchpad.

## Open questions for the orchestrator

1. The DET1 wording "byte-equal" (design section 14) needs a ruling: compare the logs without `wall_ns`, or add a new log.
2. Where does the `blocked` activity come from in the process tests?
