# L0-STORE: errata and rulings for the design text (branch `mp1/l0-store`)

Status: the builder's list for the orchestrator. Each item names its two facts with `path:line`, the
reading that the build takes, and what needs a decision. Nothing here changes a frozen M2/M3 value.

## 1. The job elapsed ticks: the derivation of `mp1/l0-wire-fu` is kept

The L0-WIRE follow-up (`origin/mp1/l0-wire-fu`, `32c148f8` and `dc72fa4d`) derives the ticks with
`base_ticks_done(units, duration_ticks, total)`, reads the duration from the `JobState` of the queue, removes
the display clamp of `production_queue`, and changes the seed of the `sources.rs` test to a record of 120 of 80.
This branch merged it and moved the derivation into `Replica::job_elapsed_ticks` over the side store with no
change of its expected values (`job_progress_in_raw_units_of_one_over_q_derives_base_ticks` is green). An
earlier build of this branch held a guard (a record whose `total` is not a whole multiple of the duration showed
its raw value) to keep the old 120-of-200 test; the merge made it unneeded and it is gone (test and plant
`STORE3/(c)` removed).

## 2. `RateRecord` stalled reading: `held` and `WaitingForFunds` keep the value

`crates/turts_rules/src/progress.rs:12-20` has `stalled` ("the job state is `WaitingForFunds`"). The old
`Replica::job_elapsed_ticks` always passed `stalled: false`. The store passes
`stalled = held || state == WaitingForFunds` (`replica_store::rate_record`), the reading of 6.4 and
erratum E21 ("its value does not advance"). The real writer sends the states `Running` and `OutputBlocked`
and `held: false` today (`crates/turts_server/src/export_jobs.rs:84-88`), so no shown value changes. Test
`a_waiting_or_held_job_keeps_its_value`. `L-PAY` should write `per_tick` 0 for such a record as well.

## 3. `MatchClock`: the replica takes the start tick at `Welcome` (orchestrator ruling (a), review finding 4)

The start tick is `Welcome.start.start_tick`. `ingest::on_welcome` now gives it to the replica
(`crates/turts_client/src/ingest.rs:400-402`, `Replica::set_match_start`), so `Replica::match_clock_ticks()` has
no argument: the last applied tick minus the start tick, 0 before the start. The clock needs no stop record: the
server sends the `Snapshot` of the end tick and then no delta (design 6.7, row "Ended", line 922; plan :833), so the
applied tick stops at the end tick. Tests: `the_match_clock_counts_from_the_start_tick_and_stops_at_the_end_tick`
(value at the end tick and after it; a stale Snapshot and a late delta leave it as it is) and
`welcome_gives_the_replica_the_start_tick_of_the_match_clock` (`tests/ingest.rs`). The task `L-MATCH-CLIENT`,
which an earlier text of this item named, does not exist in the plan; the consumers are `UI-RESULT` and `UI-HUD`.

## 4. New refusals of the store (a Baseline refinement of 6.8; binding for the writers)

These refusals are **binding for the writers of `L-CON1`, `L-PWR` and `L-MATCH-SIM`** (orchestrator ruling,
`docs/research/mp1/errata-pending-6.3.md:271`, item (2); review finding 8). The plan revision adds them to those task
blocks, each as a red test at the export boundary: a record that breaks one of them must fail there, not only in the client.

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
path change: the field `Replica::job_progress` is gone) and it asserts that the store tick equals the tick of
each client. Its lossy client now sees a gap: it loses the deltas, receives a later delta (`ResyncRequested`),
buffers the next three, and takes the Snapshot of the second tick after the loss with the retained deltas to
replay (`Applied { replayed: 2 }`). The whole store is not compared with `PartialEq` there: a `Snapshot` record and
a chain record of one job differ in `done` and `at_tick` by design (records are sent on change), though they give the
same value. The other tables (sites, supply, construction queues, match state) have no writer, so the test
`a_late_join_and_a_lossy_client_end_with_the_store_of_a_continuous_client` (`tests/mp1_store.rs`) builds the
messages from a truth model, with the same lossy path. Its oracle is `assert_matches_truth`: the expected rows come
straight from the model, compared with the getters one by one, not from the Snapshot path of the same module (review
finding 3). It also compares the stores of the three clients.

## 7. Observation path changes in other tests (permitted kind: observation change)

The field `Replica::job_progress` moved into the store. Test lines changed their path, with no change of an expected
value: `crates/turts_client/tests/economy_replica.rs` (two `is_empty` reads become
`store().job_keys().next().is_none()`) and `crates/turts_client/tests/job_progress_clients.rs` (the key read). The two
seeds of a job record (`crates/turts_client/tests/widgets_lifecycle.rs` and the unit test in
`crates/turts_client/src/widgets/sources.rs`) now go through a Snapshot with the job record
(`Replica::apply_snapshot`); `Replica::store_for_tests` and `RecordStore::seed_job_progress` are deleted, so the
store has no writer outside the staged apply path (review finding 7). The `widgets_lifecycle.rs` seed sets a context
on its hand-built replica and keeps the balances of the `Balance` resources; the `sources.rs` seed sets a context and
holds the producer and its queue only (the test asserts the production queue after it).

## 8. The doc comment of `JobProgress`

`crates/turts_net_protocol/src/records.rs:62-70` named `merge_job_progress`; it now names `replica_store.rs`
(`held_jobs`, `RecordStore::apply`). A comment change only.

## 9. What the design leaves undefined for the HUD and the result screen (no meaning invented)

The builder did not give these a meaning. Each needs a ruling by the owner of `UI-HUD` or `UI-RESULT`.

- **The order of the `State` words.** The row words are listed at design line 1832-1835 ("On hold: no credits",
  "Slowed: low power", "Waiting to start", "Waiting: exit blocked"). The record has the enum `RecordState`
  (`Running`, `Slowed`, `WaitingForFunds`, `OutputBlocked`, `Held`, `crates/turts_net_protocol/src/records.rs:16-22`) and a
  separate `held` flag (E21, design line 640 and 1833-1835). The design does not say which word wins when two hold
  (for example `held` and `Slowed`), and "Waiting to start" belongs to a held construction order, not to a job record.
  The store gives `Replica::job_progress()` (the record with `state` and `held`) and no word.
- **`MatchOver`.** The predicate is named at design lines 1590 (row H10) and 1903 and has no definition in 7.5 or 7.6.
  The only related text is the Ended state (line 922) and `MatchStopped { reason: MatchOver }` (line 1134). The store
  gives `Replica::match_state()` and `own_outcome()`; whether "over" means any outcome in `MatchState`, the own outcome,
  or the `MatchResult` message is not decided.
- **`MatchClock`.** Named at design lines 1587 (row H7) and 1903 with no definition (units, format, zero at start or at
  `Click to play`). The getter gives ticks from the start tick of `Welcome` (item 3); the text format is for `UI-RESULT`.
