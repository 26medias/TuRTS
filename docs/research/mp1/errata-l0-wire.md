# L0-WIRE repair: errata and rulings for the design text (branch `mp1/l0-wire`)

Status: orchestrator rulings of the independent review (`review-l0-wire.md`), written for
revision 6.3 and for the owners of later tasks. Nothing here changes a frozen M2/M3 value.

## 1. Change list of the order records: Baseline refinement of 6.8 (and of section 9)

Old text (6.8 and section 9): the delta carries the record of each unit "from the writer's change
list"; "each new record kind enters `CommittedTick` from the change list of the code that writes
it".

New text: "The change list of the order records holds the units that a writer of `UnitOrders` noted
and the units whose other record inputs changed in the tick: `MoveTarget`, `MoveProgress`,
`PlainMoveProgress` and `PhysicalEscape` (added, changed or removed), the pending move table, the
gather order, and a unit that the tick spawned. Each listed unit is a candidate: the publish step
compares its new record with the record of the previous tick, so a candidate that did not change
costs one comparison and sends nothing. A unit that the tick removes is never listed. When nothing
was published before the tick, every record is new."

Reason: a unit without `UnitOrders` has a record from its movement and gather state alone
(`economy/views.rs`, `unit_orders`), so a writer of `UnitOrders` cannot cover it.

Condition (met in this branch): each input has one red test and one plant (`INPUT1/(a)` to `(l)` in
`docs/research/mp1/plants/l0-wire.json`; tests `input_*` in `crates/turts_sim/src/economy/combat.rs`
and `the_first_tick_lists_every_record_when_nothing_was_published_before`).

## 2. The writer scan and the probe gate

- The scan `crates/turts_checks/tests/order_writers.rs` reads `turts_sim/src`, `turts_server/src`
  and `turts_pack/src`. Its header lists what it still cannot see (an inferred `get_mut()`, an alias
  of the type, a `for` or `match` binding, macros whose arguments are not expressions, other crates
  and test code).
- The `test-probes` check `check_order_changes` covers those at run time. It is NOT part of
  `cargo test -p turts_sim`. The fast gate is exactly:
  `cargo test -p turts_sim --locked --features test-probes`
  (the workspace run `cargo test --workspace --locked` also builds `turts_sim` with the feature,
  through the dev edge of `turts_pack`, `crates/turts_pack/Cargo.toml:19`). F-RUST names this command.

## 3. Weapon records

- `ready_tick` is 0 for a ready weapon (cooldown 0). Reason: `tick + 0` is the tick of the write,
  and a client that joined at tick N (a Snapshot) would hold a different record than a client that
  followed the deltas. Test `a_snapshot_at_tick_n_equals_the_tables_built_from_the_deltas`.
- The export filter (a target that is not in the view is written as `None`) and the resend stay: it
  is the wire rule (one visibility filter in export; fog of war needs it). The sim defect must stay
  visible, so the test `the_sim_never_hands_the_export_a_weapon_that_names_a_missing_unit` reads the
  sim state before the filter, and plant `combat-fixes.json` 1d disables the filter and the resend.

## 4. `JobState` has no `elapsed_ticks` (protocol 5)

`QueueState` rode every delta while a job ran (`elapsed_ticks` changed in each tick). Protocol 5
removes the field. The progress is `JobProgress` (`done`, `total`, `per_tick`, `at_tick`); the
client derives the elapsed ticks with `turts_rules::progress::value_at` at the last applied tick
(`Replica::job_elapsed_ticks`). The sim queue diff ignores `elapsed_ticks`
(`economy/views.rs` `queue_replacements`). `JobState.charged` stays: it changes at a charge event
only; the per-tick payment of `L-PAY` rides `RateRecord.charged`, and `L-PAY` must not write it into
`JobState.charged` (needs decision if `L-PAY` cannot keep that). `L-PAY` replaces the source of the
job rows (`export_jobs::queue_job_records`) with `CommittedTick::job_progress`.

## 5. Store rules for `L0-STORE` (no wire change)

- Every `Snapshot` carries `Some(match_state)`; the lane-0 value for "no match state yet" is the
  empty state. A reader that finds `None` reads the empty state.
- The store drops a `JobProgress` whose `(producer, job)` is no longer in the producer's
  `QueueState`, and one whose producer is removed (done in `merge_job_progress`).
- The store drops a `SiteProgress` at `sites_completed` or when its unit is removed.

## 6. Removal rule, and its one exception

A STATE record that the client checks against its unit table never names a unit that the same
message removes. An order TARGET is a reference: `OrderSummary.target` may name a unit that is gone
for about one tick, or longer for a queued order. Ruling (a), keep. Reason: the client does not check
unit targets in order records, and the marker lookup tolerates a missing unit.

## 7. Messages without a receiver

`LobbyAction` and `LeaveMatch` take the player from the connection, never from the message, have no
receiver in lane 0 and cost no bucket token. Owner `L-MATCH-SRV1`.

## 8. The wire rate of a job equals the rate that the sim applies (repair 2)

Rule: `JobProgress.progress.per_tick` is the rate that the sim applies to the job in that tick. One
function gives it: `turts_sim::economy::production::job_step_per_tick(stage, frozen, rate, q)`. The
advance steps (`run_production`, `run_machines`, `run_research`) call it, and the wire calls it through
`view_job_per_tick` (the view holds the disabled set). The table, from the sim code:

| Condition | What the sim does | Wire `per_tick` |
| --- | --- | --- |
| stage `Running`, producer enabled | advances by `rate_step(Q, rate)` (1 at Q = 1) | 1 |
| producer, machine or station disabled | skips it, stage stays `Running`, no progress | 0 |
| stage `Queued` (behind the head job, or the tick in which the head starts) | no progress | 0 |
| stage `OutputBlocked` | no progress, tries the output | 0 |
| machine with a missing input | has no job (it starts one when the input is there) | no record |
| producer without power (`L-PWR`) | `rate_factor` gives `Rate(ratio)` | the function reads it: the view must carry the rate and Q (raw units) |
| held job (`L-PAY`) | not written yet | `L-PAY` adds `held` to the function |

A record is written for each producer whose enabled state changed in the tick (`batch.enabled`), in
the delta, and the Snapshot form reads the same function. Limit, for `L-PWR`: the wire `done`,
`total` and `per_tick` are in base ticks (Q = 1). At Q above 1 the view needs the raw progress.

## 9. Notes accepted by the re-check

- `JobState.charged` stays: it changes at a charge event only. `L-PAY` moves the two client readers
  (`replica.rs` job validation, `ingest.rs` charge feedback) to `RateRecord.charged`.
- `Replica::job_progress` stays in the candidate. `L0-STORE` moves it into the side store and counts it
  in the store-clone counter.
