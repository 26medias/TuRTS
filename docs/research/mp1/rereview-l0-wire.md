VERDICT=FAIL HIGH=1

# Re-check of the L0-WIRE repair (backend-engineer, review mode)

Candidate `f739c9b1` (`mp1/l0-wire`). Integration at fetch: `54c39fdd`. Receipts:
`/home/julien/Projects/TuRTS/target/mc-evidence/l0wire-rerev-*`. Scratch:
`/tmp/claude-1000/-home-julien-Projects-TuRTS/bf418660-b272-4bb3-8628-8d9ef2677d1d/scratchpad/l0-wire-review/`.
The worktree was clean at the end (`git status --porcelain -uall`: 0 lines).

The first-review findings 1 to 8 are closed. The one new High comes from the largest change of this pass (ruling
(b)): the client derives the progress of a disabled producer's job wrongly.

## Findings

1. **High.** The client value of `elapsed_ticks` is wrong while a producer is disabled, and it stays wrong until
   the next change of the queue.
   - The sim freezes a disabled producer and keeps the stage `Running`
     (`crates/turts_sim/src/economy/production.rs:299-305`; the same skip is in `machine.rs:47-52` and
     `research.rs:262-267`).
   - The export writes `per_tick = (stage == Running)` and ignores the disabled set
     (`crates/turts_server/src/export_jobs.rs`, `queue_job_records`).
   - A delta writes job records only for queues that the sim changed (`crates/turts_server/src/export.rs:225-228`).
     A `SetEnabled` change resends only the `QueueState` (`export.rs:380-414`).
   - The probe (a real `base` sim, then real `Replica` values) ran three clients over 700 ticks: 4 jobs, disable at
     tick 30, enable at tick 90, a cancel at tick 120. It found 180 differences in 3,114 comparisons. All are in
     the continuous and the lossy clients, from tick 30 to tick 119. Example: at tick 119 the sim holds 58 and the
     client 100 (the HUD shows the job complete). The stream shows the `QueueState` at tick 30 and no job record
     until the cancel at tick 120.
   - Running, queued, cancelled, late-join and resync cases are equal at every other tick.
   - The toggle is a product path (`crates/turts_client/src/commands.rs:38`, `SetEnabled`). Protocol 4 sent the
     exact value in each tick, so this is a regression.
   - The independent Snapshot oracle in `crates/turts_server/tests/economy.rs` copies the same `per_tick` rule, so
     it cannot find this.
   - Fix (no shape change): write `per_tick` 0 for each job of a disabled producer. Write the job records of each
     producer in `batch.enabled`. Do the same in the Snapshot. Add a test that disables a producer while its job
     runs (the probe is that test).
2. **Medium.** `a_snapshot_at_tick_n_equals_the_tables_built_from_the_deltas` (`crates/turts_server/tests/ordw1.rs:647`)
   does not fail when a writer has no Snapshot form.
   - It runs `muster`, which has no running job, and it checks only that the job lists are empty
     (`ordw1.rs:719-729`). The jobs writer now exists.
   - My fault 4 removed the jobs Snapshot form (`export.rs:126-129` set to `Vec::new()`). `ordw1` stayed green
     (13 passed).
   - My probe was red: the lossy client held 0 at tick 261 while the sim held 99.
   - Fix: run a package with a producer and compare the job tables.
3. **Low.** The scan does not see `modify_component::<UnitOrders, _>`. In my fault 3 (Stop path), the scan stayed
   green and the probe check panicked (red). Add the name to the scan list.
4. **Low.** The net-id test (`crates/turts_net_lightyear/tests/registers.rs`) pins only the order after
   `ResyncRequest`. My fault swapped `Hello` and `Welcome`: 5 passed (green). With that swap, a protocol-4
   `Hello` decodes as another message, so the clear refusal is lost. Fix: pin the full list.

## Item by item

1. Plant 1d (sim clear reverted AND export filter and resend off): red-restored, "20 resync(s) across the kill of
   unit 46". `REMOVE1/(11)`: red-restored, "tick 296: the sim hands the export the weapon of unit 40 that names the
   missing unit 46".
2. The join probe of my first review (32 checks): raw weapon tables differ in 0 of 32 (16 before the repair).
   `JOINN1/(12)` is red-restored. My own fault for this item is finding 2.
3. My fault 2 (`writer.rs:153`) is red: `input_gather_stage_is_listed`, exit 101. The sampled `INPUT1` plants
   (d), (i) and (j) are red-restored with "the order change list misses the units [..]".
4. My fault 1 is red (the scan, exit 101); `SCAN1/(a)` is red-restored. The new form of my own is finding 3.
5. Ruling (b), job progress:
   - (i) Every case of the item is equal at every tick except the disabled producer (finding 1).
   - (ii) `a_running_job_costs_no_bytes_between_its_changes` passes. My 200-delta measure: 3 of 200 deltas carry
     `QueueState` (97 B each; before: 200 of 200 at 106 B). The whole delta is 83 B a tick (before: 187 B). An
     empty `DeltaRecords` is 8 B.
   - (iii) `replica_m3.rs` adds a check, so it is stronger. `widgets_lifecycle.rs` keeps "progress changes, the
     button keeps its identity" (0 to 1 in place of 1 to 2). The `sources.rs` test keeps its observations (1.0 and
     0 s), but `value_at` now caps at the total, so the clamp branch in `production_queue` has no test. In
     `economy.rs`, the client-report oracle takes the sim `elapsed_ticks` from the audit: an independent check. The
     Snapshot oracle copies the export rule for `per_tick`: it is not independent for the rate (finding 1).
   - (iv) `JobState.charged` changes only when a job starts or is queued (`production.rs:333`, `research.rs:291`,
     `machine.rs:312`). The frozen shape can serve `L-PAY` with no protocol step: the per-tick charge rides
     `RateRecord.charged`. `L-PAY` must move the two client readers of `JobState.charged` to it: the validation
     (`crates/turts_client/src/replica.rs:1078`) and the charge feedback (`crates/turts_client/src/ingest.rs:646-705`).
   - (v) Accept `job_progress` in the candidate with a note for `L0-STORE`. The rule of D:2262-2263 exists for the
     clone cost (perf M2). The map holds one entry for each queued job, so the clone cost is small. `L0-STORE` moves
     the map into the side store, and its store-clone counter counts it. No wire change.
6. `every_snapshot_carries_a_match_state` passes in the workspace run. The `ContentAtPoint` finite check exists
   (`crates/turts_client/tests/ordw1_replica.rs:290-310`; `CAP1/(a)` is in the records). The net-id test is my
   probe: same method, same result. See finding 4.
7. The frozen stream is `bcbf72c2…`. The trace bodies are base `d78dc09392e5b969` (2,898 lines), haul
   `887ce95f69801702` (4,820), acme `ba1c849aaee46dd4` (802), cedar `09f637325c511972` (4,712). `probe_neutral`
   printed `frame_chains=d0d1b1c572163424` and `muster_world=aa9db740578ed5c1`. All are equal.
8. Gates:
   - workspace: 2,057 passed, 0 failed, 38 ignored.
   - `turts_sim --features test-probes`: 395 passed.
   - clippy, wasm32 and `boundary.sh`: exit 0.
   - `--check --all`: 308 apply, 0 stale.
   - `git merge-tree` against `54c39fdd`: no conflict.
   - On a scratch merge (a detached worktree, now removed): `--check --all` gives 315 apply, 0 stale. The merged
     `combat-fixes.json` holds 1d as this branch re-pinned it and 2a to 4b as integration re-pinned them, 17
     records in all.
   - On the merged sources: `turts_sim` with probes 400 passed, `order_writers` and `ordw1` green.
9. Frozen shapes: no stage-1 lane must change a shape. The fix of finding 1 is in the writer. The `L-PAY` rule
   keeps `JobState.charged`. A queued job reads its stage from `JobState.stage`.

## What I ran

| Run | Claim tested | Refuting result | Observed |
| --- | --- | --- | --- |
| workspace, probes, clippy, wasm, boundary, `--check` | repair head green | any red | green (see item 8) |
| `m2_trace`, `probe_neutral` | no behaviour change | another hash | equal |
| 7 sampled plants through `--plants-dir` | 1d, (11), (12), INPUT1 d/i/j, SCAN1 (a) red | a green plant | 7 red-restored |
| job probe, server stream + client `Replica` | derived elapsed equals sim | any difference | 180 / 3,114 (finding 1) |
| fault 4: jobs Snapshot form removed | join-at-N test red | green | green (finding 2); probe red |
| fault 2, fault 1, fault 3 | change list / scan red | green | red, red, scan green and probe red |
| Hello/Welcome swap | net-id test red | green | green (finding 4) |
| scratch merge with `54c39fdd` | records apply, tests green | stale or red | 315 apply, 0 stale; green |
| fresh green after the faults | restore exact | red | `cmp` equal, exit 0 |

## Not checked

Headed runs. `acceptance.sh` (the builder ran it). The sources of `L-PAY`, `L-PWR` and `L-CON1` (not written).
A disabled machine and a disabled research station at run time: only by reading (same skip as the producer).

## What I changed

None in the repository. Temporary probes and faults in the review worktree, each restored (`cmp` equal). A scratch
commit object `5dab49ab` (no ref) for the merge worktree, which I removed.
