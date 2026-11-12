VERDICT=FAIL HIGH=1 FINDINGS=8 RUNS=25 NOT_CHECKED=7

# Review of L0-WIRE (backend-engineer, review mode)

Candidate: `c634efb6` on `mp1/l0-wire`. Worktree: `/home/julien/Projects/TuRTS-mp1-l0-wire-review` (clean at the end,
`git status --porcelain -uall` gives 0 lines). Integration head at fetch: `9ff4f9c7` (the L0-DBG follow-up merge;
the candidate contains `dbdd0efa`). Combat fixes: `315d5c74`.

The candidate's own gates are green. The High is a merge defect: after L0-WIRE and `mp1/combat-fixes` both merge,
the combat-fixes plant 1d cannot go red. It blocks the second of the two merges until 1d is re-pinned.

## Findings

1. **High (merge).** `crates/turts_server/src/export.rs:520-523` (the `last_target` filter) and `:488-497` (the
   resend) hide the stall defect from the process test `mc_kill_replica`. On the merged tree (L0-WIRE plus
   combat-fixes) with plant 1d applied (the sim keeps `last_target` for an ordered attacker), the test passes
   (exit 0). With the export filter and the resend also removed, the same test is red: "the owner replica asked for
   37 resync(s) across the kill of unit 46 (longest applied-tick stall 1.035933185s)". So the export is the guard
   that the process test sees, not the sim. Fix: merge combat-fixes first. Then the forward merge of L0-WIRE re-pins
   1d: the plant edits both the sim clear (`transaction_m3.rs`) and the export filter, or 1d retires and the sim
   tests 1a to 1c own the sim half. Add an export test that stays red without the filter (the builder's REMOVE1 (a)
   and (b) are that test).
2. **Medium.** `export.rs:512`: `ready_tick = tick + cooldown`. For a ready weapon (cooldown 0) the value is the
   tick of the write. A Snapshot at tick N gives N; the delta chain gives the tick of the last change (1 in my run).
   The probe "join at tick N" found raw weapon tables different in 16 of 32 comparisons; by ticks left they are
   equal in 32 of 32. The derived cooldown is equal, but a resynced client and a continuous client hold different
   records. Fix before the freeze: one canonical value (`0`, or the stored ready tick) when the cooldown is 0, and
   keep the probe as a test in `ordw1.rs`.
3. **Medium.** `crates/turts_sim/src/orders/writer.rs:153`: the change-list input `gather_changed` has no red
   test. With the line removed, `turts_sim` (380 tests, test-probes on), `turts_pack` (282, probes on) and four
   server suites (29) stay green. Either the input is redundant, or no test has a record that changes by the gather
   stage only (`crates/turts_sim/src/economy/views.rs:555-564` derives `Gathering` from the gather order). Fix: a
   test with a gatherer whose stage goes to `Idle` with no `UnitOrders` write; do the same check for each of the
   four `Changed<>` inputs (`writer.rs:89-125`, NOT TRACED one by one).
4. **Medium.** The writer scan misses more than the alias and the inferred `get_mut()`. My fault 1
   (`world.query::<&mut UnitOrders>()` and `*orders = UnitOrders::default()` in the Stop path of
   `crates/turts_sim/src/orders/apply.rs:763`) left the scan green. By reading the scanner: the turbofish check
   covers only `get_mut` and `remove` (`crates/turts_checks/tests/order_writers.rs:228`); it does not cover `query`,
   `query_filtered`, `take`, `entry`; `try_insert` is not in the insert list (`:235`); a struct literal of a bundle
   with a `UnitOrders` field is not walked (`:90`); a binding from `if let`, `match`, `for` or a closure is not
   bound (`:196-205` binds only `let` and fn parameters); macro bodies are not parsed; the roots are
   `crates/turts_sim/src` only (`:344`), while `UnitOrders` and its fields are `pub`
   (`crates/turts_sim/src/orders/mod.rs:120-124`). The backstop works: the `test-probes` check panicked on fault 1
   ("the order change list misses the units [1]"), and `cargo test --workspace` builds `turts_sim` with the feature
   on (`scripts/qa/probe_neutral.sh:5-6`; the dev edge `crates/turts_pack/Cargo.toml:19`). A fast
   `cargo test -p turts_sim` without `--features test-probes` does not run the probe. Fix: add the names above to the
   scan, scan `turts_server/src` and `turts_pack/src`, and write in F-RUST that the probe gate is the workspace run
   or the `--features test-probes` run.
5. **Medium, needs decision.** The protocol-4 `QueueState` rides each delta while a job runs, because
   `JobState.elapsed_ticks` changes in each tick (`views.rs:739`). Probe (`base`, one producer, 3 jobs, 200 ticks):
   200 of 200 deltas carry it, 106 B each, 57 % of a 187 B delta. The new on-change `JobProgress`
   (`crates/turts_net_protocol/src/records.rs:52-58`) carries the same progress, so the rate rule of design 6.8
   (D:1353-1358) does not hold for production. Options, before L0-EXIT freezes the shapes: (a) keep it, and I-HEAD
   records the cost; (b) protocol 5 removes `elapsed_ticks` (and the per-tick `charged`) from `JobState`, and
   `JobProgress` is the only progress source. (b) is a shape change: it costs least now.
6. **Medium.** Two store rules are not written. `SnapshotRecords.match_state: Option` (`records.rs:316`) has no
   meaning for `None` in a Snapshot. `JobProgress` has no removal signal for a job that completes or is cancelled.
   No wire change is needed. Ruling to record: every Snapshot carries `Some` from `L-MATCH-SIM` on; `L0-STORE`
   reads `None` as the empty state; it drops a `JobProgress` whose `(producer, job)` is not in the producer's
   `QueueState`, and a `SiteProgress` at `sites_completed` or `removed`.
7. **Low.** `crates/turts_net_protocol/src/messages.rs:365` says that the server "never sends `Cooling`".
   `export.rs:516` sends it. Fix the comment. The replica checks a finite point for `OrderTarget::Point` only
   (`crates/turts_client/src/replica.rs:701`), not for `ContentAtPoint`. Add the check with `L-CON1`.
8. **Low.** `LobbyAction` and `LeaveMatch` have no receiver and cost no token. Lightyear clears unread messages
   each frame (`lightyear_messages-0.31.0/src/plugin.rs:203-223`, `:312`), so nothing grows now. `L-MATCH-SRV1` must
   take the player id from the connection, never from the message, and must limit these messages. `Fired.target`
   is not an `Option` (`records.rs:101`). In MP1 every unit is visible; fog (M5) will need a new protocol step.

## Shape table (consumer, fits or not)

| Record or message | Consumer | Fits |
| --- | --- | --- |
| `LobbyState{revision, you, seats, settings, start}` | L-MATCH-SRV1 (stage-1 `start: Waiting`), L-LOBBY-C1 (`Seated` at the first `you`) | yes (6.7.13) |
| `Seat`, `SeatOccupant` (no player id) | both | yes (D-H, 6.8) |
| `LobbyAction{revision, action}` | stage 2 | yes; no slot field, so no action for another slot |
| `LobbyRefused{revision, reason}` | stage 2, `lobby_refusals` | yes |
| `Welcome.start: MatchStart{start_tick, teams, colours}` | start step, Immediate, late join, reconnect | yes (`session.rs:141-148`, `:295`) |
| `MatchState{connections[since_tick], outcomes}` | `match_states`, banner count, result screen | yes; Snapshot `None` needs the rule of finding 6 |
| `MatchResult{end_tick, slots[label], stats, duration_ticks}` | Ended, reconnect in Ended, RES1 | yes |
| `LeaveMatch` (no field) | LEAVE1 | yes; cannot name another player |
| `REASON_MATCH_OVER` | Ended `JoinRejected` | yes |
| 9 reject keys | `match_not_started`, `slot_defeated`, `match_over`, 6 site keys | yes, appended after `Blocked` |
| `SiteProgress`, `JobProgress`, `RateRecord` | L-CON1, L-PAY (E21 fields) | yes; job removal rule (finding 6) |
| `ConstructionQueue{constructor, orders[HeldOrder]}` | L-CON1 | yes, complete record |
| `SupplyRecord` | L-PWR | yes |
| `UnitSnapshot.site`, `sites_completed` | L-CON1 | yes; the Snapshot form is the flag |
| `Effect` `Fired`/`Destroyed`/`Credited` | L-FX-WIRE | yes for MP1 (finding 8 for M5) |
| `UnitOrderRecord.rally` | L-RLY | yes |

Snapshot form for each delta record: `unit_orders` and `orders_cleared` (full table), `weapons`, `sites`, `jobs`,
`construction_queues`, `supply`, `match_state` (`SnapshotRecords`), `sites_completed` (the `site` flag). Effects
have no Snapshot form, by design. Join-at-N probe: the order table equal in 32 of 32 checks; weapons see finding 2.
A save (M10) is sim state, not the wire; it is not in this task.

## Integrity and visibility

- Every new client field: `ContentAtPoint` becomes `OrderTargetSim::Invalid` (`crates/turts_server/src/orders.rs:54`),
  so any point, also a huge or a non-finite one, is refused (the reason key at run time is NOT TRACED).
  `LobbyAction` and `LeaveMatch` carry no slot or player. No new code writes `Slots` (grep of the diff). D25 holds.
- The bucket runs first (`session.rs:406`), keyed by `session.slot` from the receive system (`:388`), so a client
  cannot charge another slot.
- Visibility probe (two slots, 160 ticks, moves, a gather, a fight): slot 1 received 0 order records, 0 cleared
  entries and 0 weapon records of slot-0 units, and the reverse. Site, job, queue, supply and `Credited` are owner
  only by the builder's tests (`crates/turts_server/tests/ordw1.rs:365-470`), which inject rows (the sim writers are
  empty in lane 0).

## Rulings proposed

- Item 4, `OrderTarget::Unit` in an order record: rule (a), keep. The replica does not check unit targets
  (`replica.rs:668-711`); the marker lookup returns none for a missing unit (`crates/turts_client/src/visuals/markers.rs:13-18`);
  protocol 4 had the same reference; a queued order can name a dead unit for many ticks, so (b) needs an edit of
  every order queue at each removal and changes the sim order lifecycle (F-FROZEN risk). Write the removal rule as
  "a state record that the client checks against its unit table"; an order target is a reference.
- The weapon writer's `None`: correct as the wire rule (one visibility filter in export, 6.8), but it hides the sim
  defect from the strict client (finding 1). The sim tests of combat-fixes own the sim half.
- The change list: accept the builder's inputs as a Baseline refinement of 6.8 ("the change list holds the units
  that the writer noted and the units whose other record inputs changed"). A unit without `UnitOrders` has a record
  from movement and gather only (`views.rs:558-579`), so the writer alone cannot cover it. Condition: each input has
  a red test (finding 3).
- Invented shapes: `SettingEntry{key, value}` (strings; M9 may need typed values, a later step), `RecordState`
  (the five states of design 1.1, D:123-124), `HeldOrder`, `RateRecord` (the field list of D:1308), `MatchState`,
  `MatchResult` (6.7.13): accept all.

## My faults

- Fault 1 (a scan-invisible write, `apply.rs:763` Stop path): scan `no_production_write_of_unit_orders_outside_the_writer`
  green (exit 0); `a_stop_order_lists_the_cleared_record` red without probes (`left: [] right: [1]`, exit 101) and
  with probes (panic at `writer.rs:241`, exit 101). Restore: `cmp` equal to `c634efb6`. Fresh green: exit 0.
- Fault 2 (`writer.rs:153` gather input dropped): green everywhere (finding 3). Restore: `cmp` equal. Fresh green
  (`economy::combat::tests`, 44 passed).

## Merge order

`git merge-tree` against `origin/mp1/integration` (`9ff4f9c7`): no conflict. Against `origin/mp1/combat-fixes`
(`315d5c74`): no textual conflict; combat-fixes tests use the `set_order` helper that L0-WIRE routes through the
writer. Semantic conflict: finding 1. Merge combat-fixes first, then L0-WIRE with the 1d re-pin.

## Browser files that read a renamed or moved field

`tests/browser/orders-oracle.js:457-459` (`weapon.cooldown`, now derived in `debug.rs`; `status`),
`hud2-oracle.js:130` (`weapon.damage`), `m2-audit.js:129` (weapons projection). The order table is now merged, not
replaced: `hud2-oracle.js:309,359`, `m2-oracle-lib.js:100,171`, `mm1-oracle.js:288`, `orders-oracle.js:647`,
`pt1-oracle.js:495`, `sel1-oracle.js:69`, `vis-full-oracle.js:493`. Headed before the merge of a stage-1 lane: T7
(protocol 5 in wasm), the MC orders rows, HUD2, SEL1, MM1, PT1 driver checks, VIS-full and the M2 browser oracle.

## Cost (recorded, not judged)

Builder figures re-run: idle 0 B of order, cleared and weapon records (702 units); one move 6 B at tick 1 and 6 B at
tick 41; 12 troopers 81 records, 627 B in 100 ticks; `Fired` 6 B. An empty `DeltaRecords` is 8 B in each delta.
Estimate for the first playable match (2 slots, 1 open site for each slot, D:327; up to 5 held orders): a
`SiteProgress` or `JobProgress` is about 37 to 56 B, sent at start, end and each state change; under the flicker
income (1 tick in 12 pays; plan `2026-11-12-mp1-playable-skirmish.md:861`) about 2 changes in 12 ticks, so about 9 B a tick for each starved job; supply about
21 B for each building change; a full construction queue about 155 B for each change. No new record is sent in each
tick while nothing changes. The old `QueueState` is (finding 5). A held cooldown is a change in each tick
(`ordw1.rs:574-577`); the sim counts down every tick (`crates/turts_sim/src/economy/combat.rs:234-236`), so NOT
TRACED where a cooldown can hold.

## What I ran

All through `exec_in_lane_v4.py`, receipts under `/home/julien/Projects/TuRTS/target/mc-evidence/l0wire-rev-*`.

| Command | Claim tested | Refuting result | Observed |
| --- | --- | --- | --- |
| `cargo test --workspace --locked` | all tests green | any failure | 2,031 passed, 0 failed, 37 ignored |
| `cargo test -p turts_sim --features test-probes` | probe check green | failure | 380 passed |
| clippy `-D warnings`; wasm32 check; `boundary.sh` | gates | exit not 0 | 0; 0; 0 |
| `mp1_plants.py --check --all` | records apply | STALE | 269 apply, 0 stale |
| `mp1_plants.py --all --plants-dir <only l0-wire.json>` | 19 plants red | a green plant | 19 red-restored, 0 failed |
| `m2_trace` with `TURTS_M2_TRACE_OUT`; `sha256` | stream and bodies equal | other hash | stream `bcbf72c2`; bodies base `d78dc093`, haul `887ce95f`, acme `ba1c849a`, cedar `09f63732`; header = sha of `m2_trace.rs` (`04caf579`) |
| `probe_neutral.sh` | values | other value | `frame_chains=d0d1b1c572163424`, `muster_world=aa9db740578ed5c1` |
| `p4.sh`: real server and client, `--protocol-version 4` | protocol 4 refused clearly | welcomed | client exit 1, `protocol version mismatch`, disconnected; a protocol-5 client welcomed, slot 0 |
| net-id probe (`MessageRegistry.kind_map`) | equal order on both ends | different lists | equal; `ResyncRequest` 10, then `LobbyState` 11 to `LeaveMatch` 15 (lightyear `Ping`/`Pong` at 0, 1) |
| join-at-N and visibility probe | Snapshot equals deltas; no cross-slot record | difference | orders equal; weapons raw differ 16/32 (finding 2); 0 cross-slot records |
| queue cost probe | QueueState on change | per-tick record | 200/200 deltas (finding 5) |
| fault 1, fault 2 | scan and change list red | green | see "My faults" |
| merged tree + plant 1d; + filter removed | 1d red after merge | red | green, then red with the filter removed (finding 1) |
| `git merge-tree` x2 | conflicts | conflict | none |

The proposed run-time check for item 6 is the net-id probe above. `TypeMapper::kind` and `TypeMapper::name` are
public (`lightyear_utils-0.31.0/src/registry.rs:49-59`). It is about 40 lines in
`crates/turts_net_lightyear/tests/registers.rs`. Scratch copy:
`/tmp/claude-1000/-home-julien-Projects-TuRTS/bf418660-b272-4bb3-8628-8d9ef2677d1d/scratchpad/l0-wire-review/zz_rev_netid.rs`.

## Not checked

- Headed browser runs (excluded by the task).
- `scripts/acceptance.sh` and the 31 ignored process tests, except `mc_kill_replica` on the merged tree.
- The refusal reason key of a `ContentAtPoint` envelope at run time.
- Each of the four `Changed<>` inputs and the pending-table input one by one.
- A release build, and the D58 cost on 4 vCPU and 8 GB.
- The lobby and result messages at run time (no writer exists in lane 0).
- Saves (M10; MP1 keeps no data, D:1191-1213).

## What I changed

none (the probes and faults were temporary edits in the review worktree, each restored; `cmp` equal; final tree clean).
