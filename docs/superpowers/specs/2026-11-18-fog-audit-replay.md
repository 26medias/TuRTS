# Fog-safe replay of group movement dispatch

Date: 2026-11-18. Track: Foundation (audit log data model, auditor contract). Status: built on
branch `codex/fog-audit-replay`, not merged.

## Problem

Under restricted sight (fog), the live group spread leaves out each body of another slot that the
ordering slot may not see (`crates/turts_sim/src/orders/pending.rs`, the `active_basis` filter).
The economy-log auditor recomputed the spread from all units. The two results can differ, so the
auditor refused a correct fog log (`target/mc-evidence/cross-owner/REPORT.md`, limit 4).

## Facts that the design uses (traced in code)

1. The dispatch reads the Start basis of the tick. `refresh_start` runs after `advance_tick` and
   before the commands and all systems of the tick (`crates/turts_sim/src/plugin.rs`). No unit
   moves between the end of the prior tick and the dispatch: Movement runs after Orders.
2. For a slot S, a unit is permitted when it is in the basis and (its owner is S, or its cell is
   observed by S). An observed cell is in the sight circle of a source. A source is a unit with
   vision that is not a construction site and is alive, and whose owner shares sight with S
   (`VisibilityState::update`, `capture_units`). Memory of old sightings has no effect on this rule.
3. The auditor holds the audited state at the start of each tick (`tick_start` in
   `crates/turts_audit/src/replay.rs`). It checks that state against the prior `Tick` record.

Thus the hidden set of a dispatch is a function of data that the auditor already validates, plus
the constant inputs of sight. The log does not need to carry a sight result.

## Contract change

1. `AuditState` has a new optional field `raw_sight` (`SightAudit`): the policy (`grid`,
   `sharing`, `teams`), the `finalized` flag, and for each unit type in the basis its vision
   radius and whether it has health. It holds no observed cell, and no permitted or hidden unit.
2. The server writes `raw_sight` only when a `VisibilityState` is installed. An AllVisible log
   has no such key: its bytes do not change.
3. The auditor computes the hidden bodies itself (`crates/turts_audit/src/sight.rs`): it builds
   the observed cells of the ordering slot from the tick-start state with the grid geometry of
   `turts_rules`, then applies rule 2. It does not call the incremental coverage code of the
   simulation. A unit that is not in the tick-start state is hidden, as in the live basis. Both
   spread sites use the set: the dispatch and the `Blocked` envelope.
4. The sight inputs are constant: each snapshot of the log must repeat those of `Initial`.
   One change is permitted. A lobby server writes `Initial` before the room fixes the teams
   (`crates/turts_server/src/main.rs:160`, then `lobby.rs:254`), so `Initial` has the
   placeholder (`finalized: false`). The log may go one time from the placeholder to the final
   inputs, before the first `Tick` and the first `Operation` record. Only `finalized` and
   `teams` may change then. No dispatch can read the placeholder of such a log.
5. `validate_records` trusts the sight inputs of `Initial`, as it trusts the catalog and the
   geology there. `validate_records_with_sight(records, expected)` first compares the policy of
   the log with a `SightPolicy` that the caller builds from a trusted source (the locked package
   and the room). `None` means AllVisible. With a policy, it also refuses a log whose context is
   never finalized. A host that holds the package must use this function.
6. Fail closed: under fog, the auditor refuses a log with a pending movement order that has a
   Unit target. That path (the server prunes a hidden target, reason `TargetInvalid`) is not
   modelled.

## Why a forged log does not pass

The log has no hidden-unit list and no mode flag that the auditor obeys alone. A forged policy,
radius, team or `finalized` value changes the set that the auditor computes. When that changes a
spread, the recomputed slots differ from the recorded ones and the log is refused. When it changes
no spread, the recorded transitions are correct under both rules; `validate_records_with_sight`
still refuses the wrong policy.

## Versioning and migration

- No version number changes. The field is optional with a default.
- A log from before this change that has no `raw_sight` replays as AllVisible, as before.
- One class of old log is not valid: a fog log from a server at or after commit `89c6017d` (the
  fog filter of the dispatch, 2026-11-18) and before this change. It has no `raw_sight`, so the
  auditor uses the AllVisible rule and refuses it when a hidden body changed a spread. This is the
  defect that this change repairs. Such a log cannot be repaired; capture it again.
- A fog log from before `89c6017d` used all units in the live spread. It replays as before.

## Evidence

`crates/turts_audit/tests/fog_dispatch.rs`. Each log comes from a live headless simulation and
passes through its JSON line form.

- Fog capture with one hidden and one seen body on the two order points: replays.
- AllVisible capture of the same scene: replays, and has no `raw_sight` key.
- Forged mode in both directions, forged wide and narrow vision, forged shared sight, forged
  placeholder, inputs that change in one snapshot, a grid that differs from its geometry: refused.
- A fog log with a pending Unit-target group move: refused.
- Finalization at the start: accepted. Finalization after a tick, or with a changed radius: refused.

`crates/turts_audit/tests/fog_dispatch_scenes.rs` holds the live scenes of the independent
review: Team sharing, an order on tick 1, the placeholder, units with health, a source and a
body removed in the dispatch tick, 120 ticks of orders while sight changes, appended legs, a unit
born in the match, a hidden static body, the lobby start, and 14 more forgeries.

## Limits

1. Only the group spread is fog-exact. Other live rules that read the sight basis (command
   admission of a Unit or Node target, acquire, combat, gather, repair, extractor) are NOT TRACED
   in the auditor by this change.
2. Item 6 of the contract change: Unit-target group moves under fog are refused, not replayed.
3. The `Blocked` envelope path uses the same hidden set, but has no live fog test.
4. The vision radii, `has_health` and the type list in `raw_sight` have the trust of the
   `Initial` catalog. A host check of them against the package is not built. A forged radius
   that changes no spread passes.
5. The site rule and the health rule of a sight source have no live scene that separates them
   (cedar has no construction order). A planted fault in those rules, or in the `Blocked` site,
   turns no test red.
   Two guards of the start change (`!sight.finalized`, one sight value in the change record)
   have no test that needs them: the constancy check refuses the same logs.
6. A snapshot with no `raw_sight` before the start change is rewritten to the final inputs.
   The copy is then validated with the final rule, so no wrong spread passes.
7. A run of the real `turts_server` with a lobby start under fog is NOT TRACED. The test copies
   the server sequence in a headless simulation.
