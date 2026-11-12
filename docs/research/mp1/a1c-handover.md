# A1c hand-over (pre-MP1 slice, task A1c: the replay validator is deleted)

Branch `pre-mp1/a1c-delete`. This file is for A1d, A2b1, A2b2 and the reviewers. The review that
asked for it is `docs/research/mp1/review-slice-a1c.md` (written on the integration worktree; the
repair report is `docs/research/mp1/repair-slice-a1c-report.md`).

## 1. For A1d: the log-based oracle cannot see four fields (review finding 4)

- The economy log holds the ledger view (`LedgerAudit`). It has no `bounds`, `resources`,
  `compiled_durations` and `unit_types`, and no `movement_dispatch` and `units`.
- The accounting oracle (`crates/turts_pack/tests/common/accounting.rs`) models the full
  `AuditState`. The server tests rebuild that state from a logged view with `full_state`
  (`crates/turts_server/tests/economy.rs`).
- `full_state` takes `bounds`, `resources`, `compiled_durations` and `unit_types` from the
  independent public model (`template`). So on the log path the immutability rule of the oracle
  (`accounting.rs`, "immutable clock, bounds, catalog, durations, type registry, or resource
  presence changed") compares the template with itself. **A change of `bounds`, `resources`,
  `compiled_durations` or `unit_types` inside a run is not seen by the log-based oracle tests**
  (reviewer plant N12: bounds grow in each movement step; the in-process oracle and
  `full_state_rebuilds_the_audit_state` are red, the log-based tests stay green).
- `units` is rebuilt from the logged `economy.units`, so it is real data. `movement_dispatch` is the
  default value (it is the default at every logged snapshot of these tests).
- The property is held by the in-process oracle (the `turts_pack` accounting tests read
  `AuditState` from the trail), by `full_state_rebuilds_the_audit_state`, and for the bounds by the
  movement sweep test (`movement::ecs::tests::the_movement_system_writes_only_movement_state`).
- The log was not widened. A1d decides: move the oracle to `LedgerAudit` (then `full_state` goes) or
  keep this blind spot and name it in the oracle.

## 2. Refinement of the plan (section 1a, kind: test order, no assertion changed)

- E8 (`e8_commands_are_authoritative_and_acked`, `economy.rs`): the plan names only the re-pin of
  the row `E8-mixed-cost-partial-commit` to "rejected seq 5 is write neutral". With the ledger audit
  in the reader, the refusal rule gives its own text first. So the test now decodes the log with
  `decode_live_complete_tick_prefix` (no validation), runs its own checks, and then calls
  `validate_ledger`. The assertions are the same; only the order of the validation changed. The red
  of the row is the text of the test (`economy.rs`, "rejected seq N is write neutral").

## 3. For A1d: unused dependencies of `turts_audit`

`crates/turts_audit/Cargo.toml` lists `bevy`, `bevy_app` (dev) and `bevy_math` (dev). No source of
the crate uses them after the replay validator went. Removing them needs the same three lines in
`Cargo.lock` (the `turts_audit` entry: `"bevy"`, `"bevy_app"`, `"bevy_math"`); else `--locked` fails.
`scripts/boundary.sh` passes with and without them.

## 4. What the A1c repair added (so that the deletion of the replay loses no cover)

| Replay rule that went | Now held by |
| --- | --- |
| `validate_order_policy`: the order command equals its catalog definition (behaviour, target kind, unit list, queue depth, policy id, behaviour, target, fire, engage, units, queue depth, unit capabilities, behaviour and target pair, Remove capability, one definition, no duplicate capability entries) | `crates/turts_audit/src/orders.rs` (ledger rule on the log) and `to_sim_copies_every_field_of_the_order_definition` (`crates/turts_server/src/orders.rs`, the conversion itself) |
| The resting set of the movement system (point attack keeps its progress) | `crates/turts_sim/tests/movement_resting.rs` |
| A movement transition changes no non-movement state | `movement::ecs::tests::the_movement_system_writes_only_movement_state` and the field list in the module doc of `crates/turts_sim/src/movement/ecs.rs` |
| An operation is not relabeled to the phase `Other` | the capture site `economy/transaction.rs` (`capture_unit_order_operation`) and the sim tests that assert the phase `Orders` (`orders::tests::depleted_gather_writes_last_failure`, `tests/orders.rs` `mixed_order_command_audit_covers_every_per_unit_operation`); the ledger does not check a phase label of an operation with no command |

Known gap of the sweep: a write of a constant that already holds at every snapshot after tick 0
is not seen; the outcome tests of the orders and of the group movement hold those values.
