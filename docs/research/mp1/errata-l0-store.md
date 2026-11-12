# L0-STORE: errata and rulings for the design text (branch `mp1/l0-store`)

Status: the builder's list for the orchestrator. Each item names its two facts with `path:line`, the
reading that the build takes, and what needs a decision. Nothing here changes a frozen M2/M3 value.

## 1. The job elapsed ticks: the ruling and an old test input disagree (needs decision)

Facts.

- Ruling (`docs/research/mp1/errata-pending-6.3.md`, "From the second L0-WIRE re-check", item 1):
  `done`, `total` and `per_tick` of `RateRecord` count raw units of 1/Q tick, and the client derives
  ticks with `base_ticks_done(units, duration_ticks, total)` (`crates/turts_rules/src/progress.rs:82`).
- The test `widgets::sources::tests::build_menu_reports_reserved_duplicate_costs_queue_capacity_and_missing_research`
  (`crates/turts_client/src/widgets/sources.rs:2535-2557`) seeds a record of 120 of 200 for a job of 80 ticks and
  expects the progress 1.0 (a clamp). 200 is a Q of 2.5. With `base_ticks_done` the value is 48 ticks
  and the progress 0.6, so the test goes red. Plant `CLAMP1/(a)` (`docs/research/mp1/plants/l0-wire.json`)
  runs this test: with a consistent record (`total` = 80) the progress cannot be above 1.0 (`base_ticks_done` of
  `done <= total` is at most the duration), so the plant would stay green.

Reading taken. `Replica::job_elapsed_ticks` derives the ticks with `base_ticks_done` when `total` is a whole
multiple of the duration of the job in the queue (a valid Q), and shows the raw value when it is not (the
reading before the ruling; the same for Q = 1, so the HUD and the debug state show the same values). The
test input and the plant stay as they are. Tests: `the_elapsed_ticks_of_a_job_are_derived_from_raw_units_of_one_over_q_tick`
(Q = 2: 20 raw units are 10 ticks) and `a_record_that_is_not_a_whole_q_shows_its_raw_value`.

Needs decision. Option (a): keep this reading. Option (b): refuse the record (a stricter store) and change
the test input to `total: 80`, and retire or re-pin `CLAMP1/(a)`: that changes an assertion input and a
plant of another task, so the builder did not do it.

## 2. `RateRecord` stalled reading: `held` and `WaitingForFunds` keep the value

`crates/turts_rules/src/progress.rs:12-20` has `stalled` ("the job state is `WaitingForFunds`"). The old
`Replica::job_elapsed_ticks` always passed `stalled: false`. The store passes
`stalled = held || state == WaitingForFunds` (`replica_store::rate_record`), the reading of 6.4 and
erratum E21 ("its value does not advance"). The real writer sends the states `Running` and `OutputBlocked`
and `held: false` today (`crates/turts_server/src/export_jobs.rs:84-88`), so no shown value changes. Test
`a_waiting_or_held_job_keeps_its_value`. `L-PAY` should write `per_tick` 0 for such a record as well.

## 3. `MatchClock` needs the start tick, which the replica does not hold

The plan names a getter for `MatchClock`. The start tick is `Welcome.start.start_tick`, which lives in
`ReplicaState.match_start` (`crates/turts_client/src/ingest.rs:73-76`), not in the replica, and the end tick
is in `MatchResult` (a message of its own, not a store record). The getter is
`Replica::match_clock_ticks(start_tick)`: the last applied tick minus the start tick. `UI-RESULT` passes the
start tick and must stop the clock at `MatchResult.end_tick`; NOT TRACED whether the end of the match
needs a store record (needs decision for `UI-RESULT` / `L-MATCH-CLIENT`).

## 4. New refusals of the store (a Baseline refinement of 6.8; the writers must respect them)

A message with one of these records is refused as a whole (the delta asks for a resync; the Snapshot is
`Invalid`), as `merge_unit_orders` refuses a bad `orders_cleared`. Each has a case in
`a_record_that_names_a_gone_or_wrong_unit_is_refused` and a plant in `docs/research/mp1/plants/l0-store.json`.

- a `SiteProgress` for a unit that is not an own site, that the same message removes, or that appears twice;
- a `sites_completed` entry that is not a known site, that the same message removes, or that appears twice (the
  writer `export_sites.rs:20-24` already leaves out a removed site);
- a `ConstructionQueue` for a unit that is not an own unit, that the same message removes, with a non-finite
  point or a content that is not a unit of the catalog;
- a `SupplyRecord` of another slot, of a resource that is not a `Supply` resource of the catalog, or twice;
- a `MatchState` with a slot twice in `connections`;
- a record of the job table for a producer that the message removes, or twice (as before the move).

A job record for a job that no queue holds is dropped, as before. A site that completes in the delta that
carries its progress record loses the record with the site.

## 5. The store and the candidate (plan "One apply path")

Plan text (line 734): "the store change set is staged with the candidate through `stage_snapshot` and
through each `stage_delta`, and `commit` applies both". Build: `stage_snapshot` returns the candidate and a
whole staged store; `stage_delta` returns the candidate and a `StoreStep` (the changes of that delta only);
a resync applies each replayed step to the staged store of its Snapshot, and `commit` replaces or steps the
live store (`crates/turts_client/src/replica.rs:230-290`, `:395-452`, `:471-600`). The lines named in the
plan moved: `stage_snapshot` `:379` is now `:395`, `candidate_at` `:432` is `:454`, `stage_delta` `:449` is
`:471`, `commit` `:553` is `:598`.

The candidate is still cloned twice for each applied delta (`candidate_at` and `current.clone()` in
`stage_delta`): D:2257 keeps that cost. Only the job table left the candidate.

## 6. The three-client pattern: what has a writer today

Only the job records have a writer (`export_jobs::queue_job_records`). `job_progress_clients.rs` keeps its
three clients and its expected values; its table assertion reads `client.store().job_keys()` (an observation
path change: the field `Replica::job_progress` is gone) and it now also asserts that the store tick equals the
tick of each client. The whole store is not compared with `PartialEq` there: a `Snapshot` record and a chain
record of one job differ in `done` and `at_tick` by design (records are sent on change), though they give the
same value. The other tables (sites, supply, construction queues, match state) have no writer, so the test
`a_late_join_and_a_lossy_client_end_with_the_store_of_a_continuous_client` (`tests/mp1_store.rs`) builds the
messages from a truth model: a continuous client, a late joiner and a lossy client hold, at every tick, the
store of a Snapshot of the truth.

## 7. Observation path changes in other tests (permitted kind: observation change)

The field `Replica::job_progress` moved into the store. Four test lines changed their path, with no change of
an expected value: `crates/turts_client/tests/economy_replica.rs` (two `is_empty` reads become
`store().job_keys().next().is_none()`), `crates/turts_client/tests/job_progress_clients.rs` (the key read),
`crates/turts_client/tests/widgets_lifecycle.rs` and `crates/turts_client/src/widgets/sources.rs` (two seeds
become `store_for_tests().seed_job_progress(..)`). `Replica::store_for_tests` is a `#[doc(hidden)]` seam on a
production type, because the seeding tests are integration tests of another file set; no production code calls it.

## 8. For `REC`, not done here

`crates/turts_net_protocol/src/records.rs:56-60` still says "The replica does this today in
`merge_job_progress`". That function is gone: the rule is in `replica_store::held_jobs`. The branch
`mp1/l0-wire-fu` edits the doc comments of `records.rs`, so this task leaves the file alone.
