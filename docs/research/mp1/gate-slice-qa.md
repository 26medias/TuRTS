# Pre-MP1 slice gate: qa-lead lens

Source: subagent hand-back, MP1 brainstorm input, 2026-11-11. Agent output, not reviewed; claims are leads until verified.

VERDICT=FAIL FINDINGS=8 HIGH=4 RUNS=12 NOT_CHECKED=3

Artefact: revision 1 at a29ce67. Revision 2 (4177fc0) landed while I worked. Section 4 is unchanged in revision 2 (:69-83). The section 5 gaps below are still in revision 2 (:84-101). Status: DRAFT. CEO decisions b and c are Decided. I do not reopen them.

## Findings

**H1. The live-log reader needs the replay validator. Section 4 does not list it.** `economy.rs:1792-1870` (`read_live_complete_tick_prefix`) finds an incomplete tail from the validator's exact error text "command operation id N has no matching operation" (`economylog.rs:1085`, match at `economy.rs:1846`). With the validator off, an incomplete tick reaches the oracle. The run showed this: the E9-duplicate-spawn-tail-replay baseline went red at `economy.rs:2235` ("automatic operation 6744: gather state differs…"). E9-midtrace-rearm-ignored runs the same test, and its baseline was green. So the result depends on timing. The E8 test reads the log through the same path (`:3184`). After A1, the E9 and E8 rows can go flaky.
Add to section 4: "The live reader `economy.rs:1792-1870` finds the incomplete tail itself (the last `Tick` record, or an open `CommandBoundary`) and does not read validator text. Land this before A1 step 3. Proof: E9 loss green 5 times in a row."

**H2. The E8 claim is incomplete.** The three E8 cases share `e8_commands_are_authoritative_and_acked`. That test holds validator self-checks (`economy.rs:3293-3345`, `.unwrap_err()`). With the validator off, all three baselines are red at `:3294`. With those self-checks off too, E8-foreign and E8-rejection gave red-restored. The E8-mixed plant was also red, but from `economy.rs:3197` ("rejected seq 5 is write neutral"), which is not in the validator.
New text: "Re-pin E8-mixed to the `economy.rs:3197` text. The self-checks `:3293-3345` become `reservations.rs` plants with the new messages, in the same commit that deletes the replay validator."

**H3. `LedgerAudit` breaks the readers of the log.** The browser audit oracle reads `units[].pos`, `facing`, the gather order `stage` and the node `pos` from the private log (`tests/browser/m2-audit.js:67-77`). Its red text is at `:196`. The `LedgerAudit` field list (input `:59`) drops these fields. EL1 reads `movement_progress`, `plain_movement_progress`, `physical_escapes` and `unit_orders` (`el1.rs:188-215`).
New text: "`LedgerAudit` keeps every field that `m2-audit.js:67-120` and `el1.rs:188-215` read. A1 step 3 runs ci-browser (8 cases) and EL1." The browser rows were not run (NOT TRACED by run).

**H4. The classifier can accept a wrong new stream.** The stream holds, for each tick: `operation_count`, and the ordered `operation_id`, `event_kinds` and `before_equals_after` of each operation (`m3_m2_stream.rs:19-36, 108-126, 361-374`). In the fixture, acme has 1 operation; base has 897, and 755 of them have no event kind. These defects pass P0 to P3:
- (a) A new scout operation with a wrong after-state. Examples: a leaked pool, or `current` not cleared. P1 compares only positions.
- (b) A conserving change of an event amount. P2 compares kinds only, and the trace has no payloads.
- (c) Two empty or truncated traces. P0 is then true without a check.
- (d) A trace emitter that changes between the two runs.
- (e) A fixture written by `UPDATE_M3_BASELINE` (`:384`) that is not the stream the classifier read.
- (f) The escape clause "until it has a cause" (`:101`).

Fixes:
- P1 compares every trace field in acme, with an advance list of the allowed differences.
- P2 compares the full events.
- Assert 4 packages × 400 ticks × the initial unit count.
- Both runs log the emitter sha256.
- The committed fixture must hash to the stream the classifier read.
- Replace "until it has a cause" with: "stops A2c; a new ruling and a re-run self-test are needed".
- Add self-test plants: a reorder of two operations in one acme tick; a flip of `before_equals_after` on the tick-1 operation; an empty trace; 399 ticks.

**M5. Faults that lose cover after A1.**
- Wrong dispatch: `mc_orders.rs:469` asserts nothing after its 80 ticks except the validator, and it stayed green with the validator off.
- A refused unit whose state outside the ledger changes: the validator checks the full state at `economylog.rs:2346-2350`. The rule "changes no account" is weaker. The only other cover is `orders.rs:1041-1061` and `economy.rs:3197`.
- A unit that moves with no order: the scan in section 4 names only `Position` and `UnitOrders`. `gather.rs:1261` inserts `MoveTarget`, and spawns write `Position` (`transaction.rs:699`, `map.rs:163`).

Fixes: keep "refused ⇒ before == after over the full logged view" in `lib.rs` (it needs no movement model). The scan also covers `MoveTarget`, `MoveProgress` and the pools, with an allowlist. Give `:469` an outcome assert.

**M6. The pure-move diff hides a swap of two statements.** The probe showed it. A changed constant or comparison shows as plain lines, so a reviewer sees it. The visibility-line exemption also lets a type change on that line pass.
Replace with `item_equal.py` (a multiset of the top-level items, visibility removed). Also require the same `cargo test -- --list` names before and after.

**M7. "Bit-equal" is not defined (A2a, A4, A3).** `MovementFrameOutput` uses `PartialEq`, where 0.0 == -0.0. But `movement.rs:481,964` use `atan2` and `total_cmp`, and these give different results for -0.0. Required:
- Compare `to_bits`.
- 64 fixed seeds, each chained for 300 ticks.
- 1 to 5,000 movers, 0 to 20 statics of radius 0.5 to 8, `preserve_resting` not empty.
- A counter for each phase, which must be above 0.
- Plants that must go red: a phase swap; a reset `FrameState` field; a -0.0; statics queried with reach `r` only; an unsorted merge; speed `Some(0.0)`.

**L8. Open points.**
- Point 2: a probe compared a plain group-0 move with a group-7 pool of one. Position bits were equal for 400 ticks, the arrival tick was the same, and the pool count was 0 after arrival.
- Point 3: `within` drops a point at exactly `reach`. Case: qx 1.8, px -7.2000003, reach 9, cell 0.9. Add this case to the G1 test.
- "activity" has two meanings. Define it.
- The plants for each domain must be CI registry rows.
- Running both validators side by side has no exit condition.

## Item 1 result

M3: all 17 red. They cannot reach the validator: every case runs `-p turts_pack`, which has no dependency on `turts_server`. Native: 29 of 33 red; the 4 failures are H1 and H2. Browser: not run.

`cargo test -p turts_server` with the validator off, exit 101:
- Red: 48 inline tests in each binary that includes `economylog.rs`, plus `economy.rs:238`, `economy.rs:486` and `live_audit_reader…`.
- Green, so they prove nothing about the validator: the six "strictly replays" tests in `mc_orders.rs` and `session.rs:1021`.

## What I ran
| command | claim | refute | observed |
|---|---|---|---|
| m3-all validator off (`slice-gate-m3`) | :75 | a case not red | 17/17, exit 0 |
| ci-native validator off (`slice-gate-native`) | :73-75 | a case not red | 29/33, exit 1 |
| E8 ×3, self-checks off (`slice-gate-e8-v2`) | :73 | E8-mixed red with its text | 2 red, E8-mixed red at :3197, exit 1 |
| `cargo test -p turts_server --no-fail-fast` | :81 | strictly-replays tests go red | they pass, exit 101 |
| color-moved probe ×2 | :49 | swap shown | swap hidden |
| `item_equal.py` | M6 | a plant shows EQUAL | the pure move and the visibility-only change gave EQUAL; 4 plants gave DIFFERENT |
| `gridprobe` 20 M / 60 M | :130 | 0 drops | 1 drop / 0 drops |
| plain vs pool of one | :128 | bits differ | equal |
| python signed zero | M7 | atan2 the same | ±π |
| greps of the writers and readers | M5, H3 | — | cited |

Evidence (all in `/tmp/claude-1000/-home-julien-Projects-TuRTS/bf418660-b272-4bb3-8628-8d9ef2677d1d/scratchpad/qa-gate/`): `native-run.log`, `m3-run.log`, `e8v2-run.log`, `server-test-disabled.full.log`, `server-test-disabled.summary.txt`, `item_equal.py`, `gridprobe/`, `probe-out/`. Run receipts: `/home/julien/Projects/TuRTS/target/mc-evidence/root-regate/runs/slice-gate-{native,m3,e8-v2}`.

## Not checked
- The ci-browser set (a headed run is not allowed).
- The revision 2 ruling document.
- How the boundary fixture selects its rows.

## What I changed
- No repository file. The scratch worktrees are removed, and `git status --porcelain` in TuRTS-mp1 is empty.
- For about 2 minutes, a probe test file was in the runner worktree. Workers that copied the tree then got an extra `turts_sim/tests` file. No case command compiles that file.
- Memory: `/home/julien/Projects/TuRTS/.claude/agent-memory/qa-lead/pre-mp1-slice-gate-2026-11-11.md` and `MEMORY.md`.
- `target/mp1-qa-gate` stays on disk.

## Open questions
- H2 re-pin target: `economy.rs:3197`, or a new ledger rule? Needs a decision. Option a has an independent red today.
