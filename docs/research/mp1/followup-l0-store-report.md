RESULT=PASS (items 1 to 6 and 8 done; item 7, the merge of the integration tree, is NOT DONE on purpose: the orchestrator has not said that `mp1/forward-mc` is in `mp1/integration`)

Head: `d4c94308` on `mp1/l0-store` (pushed to origin). Worktree `/home/julien/Projects/TuRTS-mp1-l0-store`. Evidence: `/home/julien/Projects/TuRTS/target/mc-evidence/l0store-fu-*/command.log`. Scratch: `/tmp/claude-1000/-home-julien-Projects-TuRTS/bf418660-b272-4bb3-8628-8d9ef2677d1d/scratchpad/l0-store/`.

Commits since `fda534de` (each ends with the Co-Authored-By line):
- `d195cc22`: code, tests, errata, measurement note.
- `c92e6c28`: messages on two assertions that plants read.
- `d4c94308`: 12 new plant records (row STORE5) and two re-pinned expected-red texts.

## Items

### 1. Instrument (findings 1, 2, 3)
All in `crates/turts_client/tests/mp1_store.rs` unless noted. Each plant ran red, was restored byte-exact and ran green through `mp1_plants.py --plants-dir <folder with only plants/l0-store.json>`. Result: 30 red-restored in `l0store-fu-plants-1`, 2 failed (STORE2/(h) and STORE4/(a), because their red text named the old oracle); those two have new texts and are red-restored in `l0store-fu-plants-2`.

| Reviewer fault | New test | Plant (file edited) | Red text |
| --- | --- | --- | --- |
| F1 Snapshot keeps an empty construction record | `a_snapshot_with_an_empty_construction_record_stores_no_entry` | STORE5/(a) (`replica_store.rs`) | `the Snapshot kept an empty construction record` |
| F3 replay staged against the live store | `a_replayed_delta_stages_against_the_store_of_its_snapshot` | STORE5/(b) (`replica.rs`) | `the replay is staged against the store of the Snapshot` |
| F4 values at `highest_observed_tick` | `values_are_not_evaluated_past_the_applied_tick` | STORE5/(c) (`replica.rs`) | `a value is evaluated past the last applied tick` |
| F6 stale retained delta writes the staged store | `a_retained_delta_that_is_not_newer_than_the_snapshot_does_not_write_the_staged_store` | STORE5/(d) (`replica.rs`) | `a stale retained delta wrote the staged store` |
| R1 (new store refusal keeps the old tick, one resync) | `a_store_refusal_keeps_the_old_tick_and_asks_one_resync` | STORE5/(k) (`replica_store.rs`) | `a delta with a record that a store rule refuses asks for a resync` |
| R2 (replayed delta refused by a store rule) | `a_replayed_delta_refused_by_a_store_rule_changes_nothing` | STORE5/(l) (`replica_store.rs`) | `a replayed delta that a store rule refuses fails the resync` |

Lossy client:
- Model test `a_late_join_and_a_lossy_client_end_with_the_store_of_a_continuous_client` (`mp1_store.rs`): the lossy client loses deltas 26 to 31. It then sees delta 32 (`ResyncRequested`) and buffers 33 to 35 (`Buffered`, 4 batches retained). It takes the Snapshot of tick 33 with `Applied { replayed: 2 }`. Plant STORE5/(e) (replay does not apply its steps) is red: `lossy: store tick`.
- Real-sim test `base_producer_jobs_equal_the_sim_through_disable_enable_cancel` and its `forge` sibling in `crates/turts_client/tests/job_progress_clients.rs`: the same pattern, loss then gap then Snapshot of the second tick after the loss with 2 deltas replayed. The test asserts `replayed_seen`. Plant STORE5/(g) is red: `lossy at tick 263: the store tick`.
- The oracle is now `assert_matches_truth`: expected rows come from the truth model (sites from `truth.units`, `truth.sites`, `truth.queues`, `truth.supply`, `truth.match_state`, `truth.jobs`) and are compared with the getters. It no longer uses `stage_snapshot_store`. The test also still compares the three stores with each other. Plant STORE5/(f), a fault in the helper `construction_rows` that both paths share, is red: `continuous at tick 11: the held orders of unit 2`.
- Older plants STORE2/(h) and STORE4/(a) now print `...differs from the store of the continuous client` and `the sites`; `expected_red` re-pinned.

### 2. Finding 4, ruling (a)
- `Replica::set_match_start`; `ingest::on_welcome` calls it (`crates/turts_client/src/ingest.rs:400-402`). `Replica::match_clock_ticks()` has no argument (`replica.rs:822-824`). There is no stop record.
- Test `the_match_clock_counts_from_the_start_tick_and_stops_at_the_end_tick`: 0 before the start, 6 after the Snapshot of tick 10, 15 at tick 19, 16 at the end tick 20. After it, a stale Snapshot at 20 is `IgnoredStale`, a late delta at 20 is `Discarded`, and the clock stays 16. A start tick above the applied tick gives 0. Plant STORE5/(h) is red: `the match clock counts from the start tick`.
- Test `welcome_gives_the_replica_the_start_tick_of_the_match_clock` (`tests/ingest.rs`): a `Welcome` with `start_tick` 7 and a Snapshot at 101 give 94. Plant STORE5/(i) (call removed) is red: `...start tick of Welcome`.
- Errata item 3 is rewritten. `L-MATCH-CLIENT` does not exist; the consumers are `UI-RESULT` and `UI-HUD`.

### 3. Finding 6
- One getter: `Replica::supply_short(resource)` (`replica.rs:670`). It uses `turts_rules::supply::is_short` (consumed above provided, design line 521). False when there is no record, an unknown resource or no context. Test `supply_short_is_consumed_above_provided_and_has_one_getter` covers 40/100, 100/100 (not short), 101/100 (short), 101/150. Plant STORE5/(j) is red: `100 used of 100 made`.
- `State` word order and `MatchOver`: no meaning invented. Errata item 9 lists what the design leaves undefined, with design lines. The `State` word order is at design lines 1832-1835, 640 and `records.rs:16-22`. `MatchOver` is at lines 1590, 1903, 922 and 1134. `MatchClock` is at lines 1587 and 1903.

### 4. Finding 7
- `Replica::store_for_tests` and `RecordStore::seed_job_progress` are deleted (grep over `crates/`: 0 hits).
- `widgets_lifecycle.rs` and the `sources.rs` unit test now seed their job record through `apply_snapshot`. In `sources.rs` the Snapshot holds only the producer and its queue; the test asserts the production queue after it.
- In `widgets_lifecycle.rs` the Snapshot keeps only the `Balance` resources, which the test does not read after the seed.
- The `sources.rs` expected values are unchanged (progress 1.0, 0.0 seconds). Both tests are green.

### 5. Finding 8
Errata item 4 now says the refusals are binding for the writers of `L-CON1`, `L-PWR` and `L-MATCH-SIM` (`errata-pending-6.3.md:271`, ruling (2)). The plan revision adds them to those task blocks, each as a red test at the export boundary.

### 6. Measurement note
`docs/measurements/mp1.md` now says plainly that the before and after clone figures come from two counters. It adds the reviewer's allocation table (160 records: 443,548 B to 380,140 B per delta; Snapshot 338,254 B to 396,030 B; also the 1,600-record row), with the source `review-l0-store.md:90-112`, the scratch test `zz_l0store_review_alloc.rs`, base `4553acc4` against head `fda534de`.

### 7. Merge of the integration tree
NOT DONE. The orchestrator's instruction was to wait. `origin/mp1/forward-mc` now exists; I did not merge it or `origin/mp1/integration`. When it is in `mp1/integration`: merge it with `forward-mc` leading, and put my three changed lines inside `apply_chained` (`stage_delta(.., &self.store, ..)`, the step, `commit(.., StoreUpdate::Step(step), ..)`; plus the `Snapshot` replay path, which the review found merges clean). Then rerun the gates and the STORE plants on the result. NOT TRACED: any build or test of that resolved tree.

## Gates at `d4c94308` (all through `exec_in_lane_v4`, wrapper exit 0 unless stated)

| Gate | Result |
| --- | --- |
| `cargo fmt --all -- --check` | pass (`l0store-fu-gate-fmt`) |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | pass |
| `cargo test --workspace --locked` | pass, 165 `test result: ok` lines, 0 failed (includes the formula ratchet) |
| `cargo test -p turts_sim --locked --features test-probes` | pass |
| `cargo check --workspace --exclude turts_server --target wasm32-unknown-unknown --locked` | pass |
| `bash scripts/boundary.sh` | pass |
| `mp1_plants.py --self-test` | pass |
| `mp1_plants.py --check --all` | pass (run after the last commit; 0 stale) |
| `scripts/qa/probe_neutral.sh` | `frame_chains=d0d1b1c572163424`, `muster_world=aa9db740578ed5c1` |
| `m2_operation_stream.json` sha256 | `bcbf72c2…e08e370b`, unchanged |
| Trace bodies | not rerun this pass: `git diff fda534de..HEAD -- crates/turts_sim crates/turts_pack crates/turts_server tests/m3` is empty, and they matched at `fda534de` (base `d78dc093…`, haul `887ce95f…`, acme `ba1c849a…`, cedar `09f63732…`) |
| Node unit tests | pass: the CI list (53 files) and `tests/browser/*.test.js tests/browser/harness/*.test.js`, 338 tests each, 0 failed. The first attempt failed on `Cannot find module 'playwright-core'`; I ran `npm ci --ignore-scripts` in `tests/browser` (ignored by git), then both passed. |
| `scripts/acceptance.sh`, `MC_JOBS=4`, load 7.6 | one run, exit 0 (`l0store-fu-gate-accept`) |
| Local suites | `mp1_store` 24 tests, `ingest` 18, `job_progress_clients` 3, `economy_replica` 17, `widgets_lifecycle` 9, `mp1_store_cost` 2, lib 254: all green |

No headed run and no browser were used.

## NOT TRACED
- A build and a test of the `mp1/forward-mc` merge (item 7).
- The full plant set was not rerun as one run at the last head. The 30 red-restored plants of the first run and the 2 re-run plants ran on source identical to `d4c94308` (no source file changed after `c92e6c28`).
- The `l0-wire` records re-pinned in the first pass were run at `b1c22351` and `fda534de`, not here.
- The sound-off branch was not merged or tested.
- Any writer for sites, supply, construction queue or match state (the sim rows are empty in lane 0): the refusals are tested with hand-built messages only.
- No test fails if `on_welcome` is bypassed by another path that creates a `Replica`; `grep set_context crates/` shows `ingest.rs:399` as the only production caller.

## What I changed
Files in the branch: `crates/turts_client/src/{ingest.rs,replica.rs,replica_store.rs,widgets/sources.rs}`; `crates/turts_client/tests/{ingest.rs,job_progress_clients.rs,mp1_store.rs,widgets_lifecycle.rs}`; `docs/measurements/mp1.md`; `docs/research/mp1/errata-l0-store.md`; `docs/research/mp1/plants/l0-store.json`. Outside the branch: this report. `tests/browser/node_modules` was created (git-ignored by `.git/info/exclude`).
