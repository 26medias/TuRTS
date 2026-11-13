# Review of L0-STORE (the client record store), candidate fda534de

VERDICT=PASS HIGH=0 FINDINGS=8 RUNS=31 NOT_CHECKED=4

Seat: `bevy-rendering-engineer`, consumer lens (the HUD and the widgets read the store). Date: 2026-11-12.
Worktree: `/home/julien/Projects/TuRTS-mp1-l0-store-review`, detached at `fda534de`, clean at the end
(`git status --porcelain=v1 -uall`: 0 lines). Cache: `target/mc-mp1-l0-store-review`. Evidence directories:
`/home/julien/Projects/TuRTS/target/mc-evidence/l0store-rev-*`. Scratch:
`/tmp/claude-1000/-home-julien-Projects-TuRTS/bf418660-b272-4bb3-8628-8d9ef2677d1d/scratchpad/l0-store-review/`.

Base note: `fda534de` is not a descendant of `4553acc4`. Its merge base with `origin/mp1/integration` is
`dc72fa4d`. `git diff --stat dc72fa4d 4553acc4 -- crates Cargo.lock Cargo.toml` is empty, so the base runs at
`4553acc4` measure the same Rust tree as the true base. `git merge-tree` against `origin/mp1/integration` is clean.

## Findings

No High finding. The code is correct on every path that I traced. The defects are in the test instrument,
in the getter contract for `UI-RESULT`, and in the merge order.

1. **Medium (instrument). A replay staged against the live store is not seen.**
   `crates/turts_client/src/replica.rs:261` stages each replayed delta against `staged_store`. My fault F3 stages
   it against `self.store`. The whole `turts_client` suite stays green (`l0store-rev-f3`: 1 red test, mine only).
   A real effect: a Snapshot that brings a new own site, then a retained delta with its `SiteProgress`, is refused,
   so the resync fails each time the gap holds a site (a resync loop during construction). This is the plan
   property "staged with the whole chain" (plan :734, TD H-B). Fix: add my test R3 (below).
2. **Medium (instrument). "No client prediction" has no red test.** `replica.rs:649-651` (`applied_tick`) feeds every
   value getter. My fault F4 evaluates at `highest_observed_tick`. Green on the builder suite; red only on my R4.
   Design D:1385-1386: "never later" than the last applied delta. Fix: add R4 (values while deltas wait in a resync).
3. **Medium (instrument). The model test shares the code under test, and the lossy client never resyncs.**
   - Oracle: `crates/turts_client/tests/mp1_store.rs:1179-1200` compares each client with `fresh.store()`, the
     Snapshot path of the same module. Both paths share `site_progress_rows`, `construction_rows`, `supply_rows`
     and `held_jobs` (`crates/turts_client/src/replica_store.rs:302-374`). A fault in a shared helper is invisible.
   - Lossy client: `mp1_store.rs:1167-1177` does not call `apply_delta` for the lost ticks, then applies an
     unrequested Snapshot. No gap is seen, no delta is buffered, no delta is replayed. The same pattern is in
     `crates/turts_client/tests/job_progress_clients.rs:168-173`. Resync replay is tested only by the two-delta case
     `mp1_store.rs:551-595`.
   - Snapshot/delta disagreement: F2 (match state, delta path) is red in the model test; builder plant STORE2/(h)
     (construction, delta path) is red. My F1 (construction, Snapshot path: an empty record kept) is green on the
     whole builder suite, because the truth never puts an empty record in a Snapshot. My F6 (a stale retained
     delta writes the staged store) is also green on the builder suite; red only on my R2.
   - Fix: feed the lossy client its gap (`apply_delta` of the tick after the loss, then the server Snapshot with
     the retained tail); add R2 and R5; give the model test an expected store built from the truth, not from
     `stage_snapshot_store`.
4. **Medium (needs decision). `MatchClock` cannot be built from the getters.** `Replica::match_clock_ticks(start_tick)`
   (`replica.rs:805-807`) needs the start tick. `SourceContext` (`crates/turts_client/src/widgets/sources.rs:55-72`)
   has `replica` and no start tick. The start tick is in `ReplicaState.match_start`
   (`crates/turts_client/src/ingest.rs:76`, set at `:402`, in the same function that builds the replica at
   `:397-401`). So `UI-RESULT` must add a field to `SourceContext`, against plan :735 ("No interface task adds
   storage of its own"). Options: (a) the replica takes `start_tick` at `Welcome` (in `ReplicaContext` or a setter)
   and `match_clock_ticks()` has no argument (recommended: one line in `on_welcome`, owner L0-STORE now); (b)
   `UI-RESULT` adds `match_start` to `SourceContext`. The stop at the end needs no store record: after the end the
   server sends the end-tick Snapshot and no deltas (plan :833, "then no deltas"), so the applied tick stops at the
   end tick. `UI-RESULT` already has the test "no outcome before `end_tick`" (plan :1020); add the clock to it.
   Errata item 3 names the task `L-MATCH-CLIENT`; the plan has no such task (grep: `L-MATCH-SIM`, `-SIM2`,
   `-SRV1`, `-SRV2` only).
5. **Medium (merge). A content conflict with `mp1/forward-mc` (`785d08c6`) in `replica.rs`.** See "Merge check".
6. **Low (contract). Three HUD values have no getter, so the widgets derive them.** `SupplyShort` (design 6.2: short
   when consumed > provided) has three readers (HUD predicate, `UI-ALERT` triggers, the `L-FX-PRES` offline mark).
   The item field `State` reads `job_progress()` (`replica.rs:718-720`) and must choose among `held`, `Slowed`,
   `OutputBlocked` and `WaitingForFunds`; the order is not written. `MatchOver` has no definition in design 7.5.
   Recommendation: add `supply_short(resource)` and one row-state getter now.
7. **Low (seam). `Replica::store_for_tests` (`replica.rs:643-647`) and `RecordStore::seed_job_progress`
   (`replica_store.rs:167-172`) are `pub`.** Any crate can write the store outside the staged path. No production
   caller (grep: `sources.rs:2536` in `cfg(test)`, `tests/widgets_lifecycle.rs:770`). Cheaper honest form: seed
   both tests through a Snapshot with the job record, and delete the seam. A `test-seams` feature is the second
   choice.
8. **Low (policy text). The new refusals (errata item 4) are builder policy.** The writers in the tree respect them
   (table below), but the plan blocks of `L-CON1`, `L-PWR` and `L-MATCH-SIM` do not state them. Write them into
   those tasks as a red test at the export boundary.

## Atomicity trace (question 1)

| Path | Where | Store write | Result |
| --- | --- | --- | --- |
| Snapshot applied | `replica.rs:238`, `:286` | staged whole store, `commit` `StoreUpdate::Replace` `:610` | store and candidate in one call |
| Snapshot replay | `replica.rs:261-269` | each step on `staged_store` only | live store untouched until `:286` |
| Resync fails after its Snapshot | `replica.rs:280-284` | none (staged store dropped) | my R2: store, tick 10, retained deltas kept |
| Invalid Snapshot | `replica.rs:238-242` | none | builder `an_invalid_snapshot_changes_nothing` |
| Stale Snapshot | `replica.rs:230-235` | none | builder `a_stale_snapshot_and_a_discarded_delta_change_nothing` |
| Delta applied | `replica.rs:321-328` | `StoreUpdate::Step` in `commit` `:611` | one call |
| Delta refused (any rule) | `replica.rs:322-327` | none | my R1: a NEW store refusal with a valid position change: positions, tick and store stay at 10; `ResyncRequested` once, then `Buffered` |
| Discarded delta | `replica.rs:297-304` | none | builder test |
| Commit | `replica.rs:598-631` | no early return between the candidate fields and the store | readers see both or neither |

Readers: `SourceContext.replica` (`sources.rs:57`) and the debug state read `&Replica`; the only writer is
`ReplicaState` under `ResMut`. Two copies of one fact exist: `store.tick` and the `pub` field
`last_applied_tick`; only test code writes the field directly (`scenario.rs:724`, `lib.rs:349`, `ingest.rs:1171`,
all in `cfg(test)` modules).

## Cost (question 2)

The counter is honest for what it counts: `impl Clone for RecordStore` (`replica_store.rs:76-89`) is the only
`Clone`; `StoreStep` and `StoreUpdate` do not derive `Clone` (`:93-113`); no `mem::take` of a store table. A
`Replica` clone also counts (`ingest.rs:165`, `:780`, test hooks). The measurement note compares two different
counters (job-table clones before, store clones after). My own instrument: a counting global allocator, test
`zz_l0store_review_alloc.rs` (scratch copy), debug build, the `scale` world, 100 empty deltas, 7 runs.

| World | Build | Snapshot allocs / bytes | Per delta allocs / bytes | Per delta time, min (debug) |
| --- | --- | --- | --- | --- |
| 40 x 4 (160 records) | base 4553acc4 | 1,619 / 338,254 | 2,163 / 443,548 | 0.358 ms |
| 40 x 4 | head fda534de | 1,617 / 396,030 | 2,109 / 380,140 | 0.313 ms |
| 400 x 4 (1,600 records) | base | 15,909 / 3,327,766 | 21,365 / 4,379,710 | 4.32 ms |
| 400 x 4 | head | 15,802 / 3,780,550 | 20,835 / 3,757,806 | 2.92 ms |

The move saves 63,408 B for each delta at 160 records (-14 %). The two candidate clones stay (D:2257) and are
86 % of the delta bytes. A Snapshot now costs 17 % more bytes (+57,776 B at 160 records: the staged rows are
built in `Vec`s and then collected into maps, `replica_store.rs:386-422`). Time is from a loaded host (load 11 to
21) and a debug build: recorded, not judged.

`t5_convergence`, release, alone, 3 runs each (`target/mc-mp1-l0-store-review/tmp/acceptance-oracle.log`):
head lag_p99 1.341, 1.335, 1.339 (load 13.4 to 10.9); base 1.336, 1.340, 1.343 (load 21.2 to 16.1). All 6 pass.
The head is not slower.

## Getter table (question 3)

| Value (design 7.3 to 7.5, 6.10) | 6.8 record | Getter | What `UI-HUD` / `UI-RESULT` still derives |
| --- | --- | --- | --- |
| `Supply(res, side)` | `SupplyRecord` | `supply(&str, SupplySide)` `replica.rs:655-662` | `ContentRef` to id string; `None` vs 0 |
| `SupplyShort(res)` | `SupplyRecord` | none | consumed > provided (finding 6) |
| `ConstructionQueue` | `ConstructionQueue` | `construction_queue(id)` `:665-667` | none |
| `Sites` | `site`, `sites_completed` | `sites()`, `is_site()` `:670-676` | owner filter from `replica.units` (public list) |
| item `Paid`, `Refund` (site) | `SiteProgress` | `site_paid`, `site_refund` `:690-717` | cost per entry from content |
| item `Paid`, `Refund` (job) | job record | `job_paid`, `job_refund` `:754-781` | cost per entry, `CancelRefunds` from content |
| item `State` | job record `state`, `held` | `job_progress()` `:718-720` | the row word and its order (finding 6) |
| `Outcome`, `OutcomeReason` | `MatchState` | `outcome()`, `outcome_reason()` `:794-801` | wire kind to mod-API kind |
| `OutcomeIs`, `OutcomeReasonIs`, `{slot}` | `MatchState` | `outcome()`, `match_state()` | as above |
| `MatchOver` | `MatchState` | none | definition missing (finding 6) |
| `MatchClock` | none (`Welcome`) | `match_clock_ticks(start_tick)` `:805-807` | cannot be built (finding 4) |
| `Connection` (UI2-CONN) | `MatchState` | `match_state()` `:783-785` | none |

`crates/turts_client/src/widgets/sources/hud.rs` and `result.rs` are stubs that return null, no rows or false.

## Refusal table (question 4)

| Refusal | Rule source | Writer in the tree | Can a legal server send it? |
| --- | --- | --- | --- |
| `SiteProgress` not an own site, removed, twice | removal: ruling (`errata-pending-6.3.md:230`); own and site: builder | `export_sites.rs:27-38` filters owner and removed | No in one tick (completion and progress in one delta is accepted and dropped, `replica_store.rs:453-457`). NOT TRACED: a final record in the tick after completion (`L-CON1` sim rows are empty) |
| `sites_completed` not a site, removed, twice | builder | `export_sites.rs:17-24` filters removed | No |
| `ConstructionQueue` not own, removed, non-finite point, not a unit | builder | `export_sites.rs:41-66` filters owner and removed | Only if the server admits a non-finite point (NOT TRACED, `L-CON1`) |
| `SupplyRecord` other slot, not `Supply`, twice | design 6.8 "Owner" (D:1311) + builder | `export_supply.rs:10-21` filters the slot | No |
| `MatchState` slot twice | builder | `export_match.rs:12-14` returns `None` | No |
| job record twice, removed producer | existed at the base (`merge_job_progress`) | `export_jobs.rs:37-50`; `export.rs:226-233` chains two sources | Not new. Risk for `L-PAY` if it adds `batch.job_progress` beside `queue_job_records` (a duplicate key) |

## My faults (question 5)

All on the whole `cargo test -p turts_client --locked --no-fail-fast`, with my test file present. Restore by copy,
`cmp` with `git show HEAD:<path>`, `git diff --quiet HEAD -- crates/turts_client/src` after each. Fresh green after
all six: `l0store-rev-fresh-green`, 37 result lines, 0 failed. None is one of the 20 builder plants.

| Fault | Edit | Red tests | Seen by the builder suite? |
| --- | --- | --- | --- |
| F1 | Snapshot keeps an empty construction record (`replica_store.rs:401-403`) | my R5 | No |
| F2 | a delta `match_state: None` resets the state (`:203-205`) | builder: model test, `a_missing_match_state_...`, `each_table_...`; my R6 | Yes |
| F3 | replay staged against the live store (`replica.rs:261`) | my R3 | No |
| F4 | values at `highest_observed_tick` (`replica.rs:649-651`) | my R4 | No |
| F5 | step does not advance `store.tick` (`replica_store.rs:206`) | builder: 5 tests in `mp1_store`, 2 in `job_progress_clients`; my R1 | Yes |
| F6 | a stale retained delta is applied to the staged store (`replica.rs:252-254`) | my R2 | No |

## Seam (question 7)

See finding 7. Severity Low: no production caller, but nothing stops one.

## Merge check (question 9)

- `origin/mp1/integration` (`4553acc4`): clean.
- `origin/mp1/sound-off-default` (`314e2722`) and local `mp1/sound-off-default` (`861889b6`): conflict in
  `.github/workflows/ci.yml` only. Integration against sound-off gives the same conflict, so it is not from L0-STORE.
- `mp1/forward-mc` (`785d08c6`, local only; no `origin/mp1/forward-mc`): conflict in
  `crates/turts_client/src/replica.rs`, merged lines 458-502 (scratch `forward-mc-merged-replica.rs`). Forward-mc
  splits `apply_delta` into `apply_chained`, `drain_reorder`, `check_grace` and adds `start_resync(..,
  ResyncReason::ValidationFailure)`. L0-STORE changed the same 7 lines (`stage_delta` with `&self.store`, the
  step, `commit(.., StoreUpdate::Step(step), ..)`). Forward-mc must lead (it is the larger control-flow change);
  L0-STORE puts its three changed lines inside `apply_chained`. The Snapshot path merges with no conflict (merged
  :314-386): the replay uses `staged_store`, and the held `ahead` deltas stage later against the live store, which
  equals the staged store after the commit. NOT TRACED: a build and a test of the resolved tree.

## Gates at fda534de

fmt 0; clippy `--workspace --all-targets --locked -- -D warnings` 0; `cargo test -p turts_client --locked` 0
(35 binaries, 0 failed); `cargo test --workspace --locked` 0 (166 result lines, 2,086 passed, 0 failed, 38 ignored);
wasm32 check 0; `scripts/boundary.sh` 0; `mp1_plants.py --check --all` 0 (448 apply, 0 stale); 6 builder records
(STORE1/a, /c, STORE2/f, /h, STORE3/b, STORE4/e) through the plant runner: 6 red-restored, 0 failed. Frozen
stream: `sha256sum tests/m3/m2_operation_stream.json` = `bcbf72c2...e08e370b`, no change under `crates/turts_sim`,
`turts_pack`, `turts_server`, and `m2_operation_stream_matches_baseline ... ok` in the workspace run.

## NOT TRACED

- A build and a test of the forward-mc merge resolution.
- Whether `L-CON1` writes a site record after completion, or admits a non-finite held-order point.
- The wasm32 cost of the store (only native debug allocation counts above).
- The design meaning of `MatchClock` (design 6.10 H7 names it, gives no definition) and of `MatchOver`.

## What I changed

This file only. In the review worktree: temporary test files and fault edits, all removed; worktree clean at
`fda534de`.
