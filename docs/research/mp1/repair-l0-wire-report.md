RESULT=PASS

# Repair of L0-WIRE after the independent review (network-engineer, build mode)

Branch `mp1/l0-wire`, head f739c9b1 (pushed, not merged). I merged `origin/mp1/integration` 3ebf7e2a first, with a merge commit (b3c724e5, no conflict). Labels: `l0wire-fix-*`. Receipts: `/home/julien/Projects/TuRTS/target/mc-evidence/l0wire-fix-*`. Scratch: `/tmp/claude-1000/-home-julien-Projects-TuRTS/bf418660-b272-4bb3-8628-8d9ef2677d1d/scratchpad/l0-wire/`.

## Per item

1. **High (stall defect hidden by the export): done.**
   - (a) Plant 1d in `docs/research/mp1/plants/combat-fixes.json` is re-pinned: it reverts the sim clear AND turns off the export resend and the export filter. On the merged tree it is red and restored through `--plants-dir`: "the owner replica asked for 18 resync(s) across the kill of unit 46 (longest applied-tick stall 1.02 s)" (`l0wire-fix-plants`).
   - (b) New test `the_sim_never_hands_the_export_a_weapon_that_names_a_missing_unit` (`crates/turts_server/tests/ordw1.rs`). It runs a real ordered attack through a kill (trooper 40 at dummy 46) and reads the SIM state before the filter. New plant `REMOVE1/(11)` (the sim clear reverted, the filter left in place) is red: "names the missing unit". The export rule stays.
2. **Medium (`ready_tick` for a ready weapon): done.**
   - A weapon with cooldown 0 is written with `ready_tick` 0. Reason: `tick + 0` is the tick of the write, so a Snapshot at tick N and a chain of deltas gave different records. The doc comment of `WeaponRecord.ready_tick` says so, and says that `Cooling` is sent (item 7).
   - Test `a_snapshot_at_tick_n_equals_the_tables_built_from_the_deltas`: for 120 ticks of a fight with a move, the tables built from the deltas (orders and weapons, with removals and `orders_cleared`) equal the tables of the Snapshot of that tick. It also fails when a writer starts to write a site, job, queue or supply record with no Snapshot form. Plant `JOINN1/(12)` (tick plus cooldown) is red.
3. **Medium (inputs of the change list): done.**
   - One red test and one plant for each input: `Changed<>` of `MoveTarget`, `MoveProgress`, `PlainMoveProgress`, `PhysicalEscape` (a to d); `RemovedComponents<>` of the same four (e to h); the pending table (i); the gather order, `gather_changed` (j, a record that changes by the gather stage only); the spawned units (k); the first-tick rule (l). Tests `input_*` in `crates/turts_sim/src/economy/combat.rs`. Plants `INPUT1/(a)` to `(l)`, all red and restored.
   - The erratum text of the Baseline refinement is in `docs/research/mp1/errata-l0-wire.md` (section 1).
4. **Medium (writer scan): done.**
   - The scan now sees `query::<&mut UnitOrders>()` and `query_filtered`, `take`, `entry`, `try_insert`, a bundle struct with a `UnitOrders` field, a binding from `if let`, a closure parameter typed `&mut UnitOrders`, and macro arguments that parse as expressions. It reads `turts_sim/src`, `turts_server/src` and `turts_pack/src` (non-test code). Fixtures: `the_widened_write_patterns_have_fixtures_that_the_scan_finds`.
   - Plants `SCAN1/(a)` (the reviewer's fault 1, a query write in the Stop path) and `SCAN1/(b)` (a direct write in `turts_server/src`) are red.
   - The test header lists what the scan cannot see. The fast gate for the `test-probes` check is `cargo test -p turts_sim --locked --features test-probes` (header and errata file). It ran green: `l0wire-fix-probes`.
5. **Medium (job state in `QueueState`): done.**
   - `JobState.elapsed_ticks` is removed from the wire. `JobState.charged` stays (see "Decision" below).
   - The sim queue diff ignores `elapsed_ticks` (`economy/views.rs` `queue_replacements`). The export writes a `JobProgress` for each job of a changed queue, and in the Snapshot (`export_jobs::queue_job_records`).
   - The client keeps `Replica::job_progress` and derives the elapsed ticks with `value_at` (`Replica::job_elapsed_ticks`). The drop rules are in `merge_job_progress`.
   - Readers of the removed field (`rtk proxy grep`): `widgets/sources.rs` (HUD progress and remaining time) moved to the derived value; `debug.rs` keeps `economy.queues[].jobs[].elapsed_ticks` as a derived value for `tests/browser/m2-audit.js` (and the copy `tests/qa/driver/m2-audit-copy.js`). No browser file was edited. Test literals: about 27 `elapsed_ticks:` lines were deleted. Three tests were changed in the permitted "observation change" form: `replica_m3.rs`, `widgets_lifecycle.rs`, `sources.rs` unit test.
   - Measured, `a_running_job_costs_no_bytes_between_its_changes` (`docs/measurements/mp1.md`): 3 jobs of 100 ticks, 260 ticks. Before (the queue diff counts elapsed again, plant `JOBQ1/(a)`): queue and job records in 260 of 260 ticks, 29,557 B, about 114 B in a steady tick. After: 5 of 260 ticks (1, 101, 102, 202, 203), 477 B, 0 B in the other 255 ticks.
   - Frozen stream bcbf72c2... equal; the four trace bodies equal (see Hash checks); `probe_neutral` equal.
   - Headed rows before a stage-1 lane merges (in `docs/measurements/mp1.md`): T7, the MC orders rows (`orders-oracle.js`), HUD2, SEL1, MM1, PT1 driver checks, VIS-full, the M2 browser oracle (`m2-audit.js`). None was run here.
6. **Medium (store rulings): done.** `errata-l0-wire.md` section 5 and doc comments at `SnapshotRecords`, `JobProgress`, `SiteProgress`. Every Snapshot carries `Some(MatchState::default())` (writer `export::snapshot_for`, test `every_snapshot_carries_a_match_state`, plant `MATCH1/(a)`). The replica drops a `JobProgress` whose queue no longer holds the job and one whose producer is removed, and refuses one that names a producer that the same delta removes (test `job_progress_derives_the_elapsed_ticks_and_follows_the_queue`, plants `JOBQ1/(b)` to `(d)`). The `SiteProgress` rule is documented only (no writer yet).
7. **Low: done.** The comment at `messages.rs` now says that `Cooling` is sent. The replica refuses a `ContentAtPoint` with a point that is not finite (test `a_content_at_point_record_needs_a_finite_point`, plant `CAP1/(a)`).
8. **Low: done.** Doc comments at `LobbyAction` and `LeaveMatch`: the receiver takes the player from the connection, never from the message; no receiver in lane 0; owner `L-MATCH-SRV1`. The run-time net-id probe is in `crates/turts_net_lightyear/tests/registers.rs`: both ends give the same ids, and the five messages follow `ResyncRequest`.
9. **Held order naming a removed unit: done, ruling (a), keep.** Stated as an exception in the doc comment of `UnitOrderRecord` (`messages.rs`) and in the errata file section 6.

## Gates on the repaired head f739c9b1

- fmt: exit 0. Clippy `--workspace --all-targets --locked -D warnings`: exit 0 (`l0wire-fix-clippy2`).
- `cargo test --workspace --locked --no-fail-fast`: exit 0 (`l0wire-fix-ws`). `cargo test -p turts_sim --locked --features test-probes`: exit 0 (`l0wire-fix-probes`).
- wasm32 check: exit 0. `boundary.sh`: exit 0. The formula ratchet is green in the workspace run, and I raised no count.
- Hash checks (`l0wire-fix-trace`, `l0wire-fix-neutral`):
  - Stream `tests/m3/m2_operation_stream.json`: bcbf72c2206251f6... equal. The stream that `m2_trace` writes is equal to it.
  - Trace bodies (`tail -n +2`): base d78dc09392e5b969 (2,898 lines), haul 887ce95f69801702 (4,820), acme ba1c849aaee46dd4 (802), cedar 09f637325c511972 (4,712): all equal. The header `emitter_sha256` 04caf579... is the sha256 of `m2_trace.rs`.
  - `probe_neutral`: `frame_chains=d0d1b1c572163424`, `muster_world=aa9db740578ed5c1`: equal (read from the log).
- `mp1_plants.py --self-test`: exit 0. `--check --all`: 308 records apply, 0 stale, exit 0.
- Plants through `--plants-dir` (my `l0-wire.json` and `combat-fixes.json`): 51 plants in 44 command groups, 51 red and restored, 0 failed (`l0wire-fix-plants`). 1d is among them.
- `scripts/acceptance.sh` with `MC_JOBS=4`, load average 10 to 11. Run 1 failed at `e9_two_owners_late_join_and_output_lifecycle`: the independent Snapshot oracle of `economy.rs` did not expect the match state and the job records. Run 2 failed at `e9_loss_and_small_buffer_converge`: the client report oracle did not expect the derived `elapsed_ticks`. I fixed both oracles (observation change: the expected values follow the protocol 5 shapes). The four ignored `economy` tests then passed alone (`l0wire-fix-econ`), and run 3 passed all stages, exit 0 (`l0wire-fix-accept3`). The failures were my fault, not load.

## Decision and items for the orchestrator

- **Needs decision: `JobState.charged` stays in protocol 5.** The ruling says to remove `elapsed_ticks` and the per-tick `charged`. No per-tick `charged` exists today: `charged` changes at a charge event only (M2 and M3 charge at queue). The client uses it for validation (`replica.rs`) and for the charge feedback (`ingest.rs`). Removing it is a larger change, so I kept it. Rule recorded for `L-PAY`: a per-tick payment rides `RateRecord.charged` and must not be written into `JobState.charged`. If `L-PAY` cannot keep that, `charged` must leave `JobState` before the freeze.
- **Needs decision: where `Replica::job_progress` lives.** `L0-STORE` (ruling R-H) is the side store, and D:2257 says "MP1 adds no field to the replica candidate". The field was needed now because the HUD and the debug state read the elapsed value. It is one `BTreeMap` in `Candidate` and `Replica` and commits with the candidate. `L0-STORE` may move it.
- Until `L-PAY`, the job rows come from the queues, not from `CommittedTick::job_progress`. A job that is queued but not started has `per_tick` 0 and state `Running`, because the five states of design 1.1 have no "waiting its turn" state.

## NOT TRACED

- No browser run (T7, `orders-oracle.js`, `m2-audit.js`, the other rows of item 5).
- The "before" byte figure uses the new wire shape, so protocol 4 was a few bytes larger. The reviewer's figure for protocol 4 is 106 B in 200 of 200 deltas.
- The medium workload of D57.

## Commits (after the merge b3c724e5)

`fix(wire): ready_tick 0, join-at-N test, sim state check`; `test(wire): one red test for each input of the order change list`; `test(plants): input plants, re-pin 1d`; `test(wire): the writer scan reads ...`; `feat(wire): protocol 5 JobState has no elapsed_ticks ...`; `feat(wire): job records from queues, Snapshot match_state Some, ContentAtPoint finite point, net-id probe, docs of the rulings`; `docs(wire): errata and measurements of the L0-WIRE repair`; three test-oracle commits for `economy.rs` and the plant records. Head f739c9b1.
