# M6 underground utilities

Status: all eight M6 acceptance rows pass on branch `codex/m6-underground-utilities`.
The branch is not merged. Root integrates it and reconciles the version numbers.
Spec: `docs/superpowers/specs/2026-11-16-m6-underground-utilities.md`. Plan: `docs/superpowers/plans/2026-11-16-m6-underground-utilities.md`.
Evidence root: `/home/julien/Projects/TuRTS/target/mc-evidence/m6-utilities/`. Paths below are relative to it.

## What a player can do

The `utility_yard` package shows the whole path with mouse and keyboard.
Press U: the terrain fades, and own lines, junctions and ports show in mod colours.
Select the depot, choose "Power line" and click two points. The line goes in at a fixed depth.
Two lines that cross in plan view do not connect. The Workshop port stays red ("Underpowered").
A short route between the two lines makes two explicit junctions. The Workshop turns green ("Powered").
A worker digs the ridge under the depth-1 route. The route is exposed and disconnects (mod policy `Disconnect`).
A paid fill restores the cover, and the route conducts again. No line elevation ever changes.
Hover shows the state label, the network supply and demand, the depth, the elevation and the cover.
With the view off, normal selection, moves and terrain work are unchanged.

## Engine capability

- Public authoring (API 0.2.18): `exports.utilities`, `SupplyScope::Network`, ports on the utility type,
  depth range, reach, segment length, segment cost, exposure policy (`Operate`, `Disconnect`, `Remove`),
  colours, junction symbol and labels. `Behaviour::UtilityRoute`, `TargetKind::UtilityRoute`, the HUD source
  `UtilityRoutes`, and the engine actions `utility_view`, `utility_deeper`, `utility_shallower`.
- Topology: one logical layer per slot. Plan-view crossings never connect. Explicit endpoints only: port,
  junction, segment split, free point. Ids are counted per slot. Limits: 512 junctions and 512 segments per
  slot, 256 touched cells per segment.
- Placement: applied on admission in one audited transaction with the slot payment. Fixed elevation is the
  lowest touched surface minus the depth. Every touched cell must be currently observed by the slot.
- Burial: the pass after `Outcome` checks only segments on changed cells and new or split segments.
- Operation: aggregate electrical supply. A network is short when consumption is above supply. Every
  consumer on a short network gets its authored `when_short` effect in the next tick. No storage, no loss.
- Wire (protocol 11): owner-only `UtilityView` in snapshots and deltas, with a per-view dictionary of
  utility ids. A delta carries the view only on a tick where the owner rows changed.
- Client: utility view, route mode with snapping and depth keys, preview with a reason, inspection panel,
  faded terrain through blended material copies, and a test bridge for the oracle.

## Acceptance

| Row | Result | Evidence |
| --- | --- | --- |
| UT1 | PASS. `utility_yard` loads through public authoring. 13 runtime tests cover every refusal of the spec with field paths and the API floor. A mutation that skips the unit checks turns three tests red. | `crates/turts_mod_runtime/tests/utilities.rs`; `final-ae72ec92/gates/native-full.log` |
| UT2 | PASS. Crossing does not connect; a split route connects; the consumer is supplied in the next tick; port removal makes junctions. Real package: the server test drives the full player path. | `crates/turts_sim/src/utilities/tests.rs`; `server-utility-wire.log` |
| UT3 | PASS. Paid dig and paid fill through ordinary work orders change cover only. All three policies act as declared. Disconnect recovers after fill, also for the half of a split segment. The checked-segment counter proves that unchanged cells are not checked. | sim tests; `real_package_crossing_connection_exposure_and_paid_fill_keep_elevation_fixed` |
| UT4 | PASS. Every refusal leaves state and balances unchanged. Foreign and absent ids give identical results. The review found one gap (a split joined to an end of the same segment); it is now refused and tested. | sim UT4 tests; `review/review.md` |
| UT5 | PASS. Exact totals for chains, splits and several networks; a generator killed at tick T supplies nothing in T+1; reversed creation and command order give equal state; Network resources never enter slot totals or `SupplyRecord`. | sim UT5 tests |
| UT6 | PASS. Viewer 1 encoded snapshots and every encoded patch are byte-equal across paired worlds through lines, digging and exposure. Viewer 0 gets only its own rows. The committed-tick delta path carries the view only on change. Both wire tests go red on a leaking or silent export. | `crates/turts_server/src/utility_wire_tests.rs` |
| UT7 | PASS within the representative budgets (see below). | `ut7/ut7-release.json`, `ut7-wire-size.log` |
| UT8 | PASS on the final candidate build. Real pointer and key input in a fresh silent Chrome. Both zoom limits. All three scripted cold-play probes pass. | `final-ae72ec92/ut8/browser/result.json`; `final-ae72ec92/cold-*` |

## UT7 measurement

Release test binary, SHA-256 `7e56db5d2202d1bbe85fc4f185d9d004c7fa290742b64b5f5aa1684818cbe1d9`, source `89006a33`.
Run on the local workstation, CPUs 0, 2, 4 and 6, under the exclusive CPU lock. 2,000 ticks, 16 edited cells under lines per tick.

| Load | p50 | p99 | Max | Budget |
| --- | --- | --- | --- | --- |
| Real pass, 2 slots at the limits | 51.1 µs | 141.0 µs | 214.2 µs | p99 ≤ 2 ms |
| Synthetic pass, 8 slots at the limits | 388.0 µs | 763.8 µs | 836.7 µs | p99 ≤ 2 ms |

One owner view at the slot limits, with 1,024 network rows, encodes to 31,227 bytes (budget 64 KiB).
Before the dictionary wire form it was 78,306 bytes, over the budget. The repair is in `b728e60f`.
This is an isolated utility measurement. It is not an M7 integrated-scale claim.

## Review

One independent code-running foundation review (technical-director seat): PASS-WITH-FIXES, 7 findings, 1 high.
The high finding (strict Clippy) and two medium findings (duplicate split pair; no test for the committed-tick
delta path) are repaired in `563a636a`. The same reviewer rechecked only those items: PASS.
Evidence: `review/review.md` and the logs beside it.

## Native and tooling checks

- First full native campaign on `89006a33`: 3,222 tests passed. Three targets had stale expectations
  (no-content scan without the utilities export, the HUD source list, the protocol version edge). They are
  repaired in `f45415fa`. Evidence: `native-full-1.log`.
- Final gates on `ae72ec92`: formatting, wasm32 check and dependency boundary pass. The full native campaign
  passes 3,241 tests with 0 failures (273 ignored). Evidence: `final-ae72ec92/gates/`.
- The acceptance script passes all 18 process test steps. Its last step, the HD1 native reference, first
  stopped on a shared-cache owner check: another session had built in the shared cache. After a rebuild set
  both owner markers to this worktree, the HD1 reference and the HD1 test pass. Evidence:
  `final-ae72ec92/gates/acceptance.log` and `final-ae72ec92/gates/hd1-rerun/`.
- Strict Clippy on all eleven changed crates passes (review recheck, `review/re_clippy.txt`).
- The QA0 completeness gate and the driver-rule self-test are red on the base commit `fc16df8e` too. The new
  spec adds one more "no registry row" line of the same kind as every spec since 2026-11-13. Evidence:
  `gates/completeness-base-fc16df8e.log`.
- The cold-play checker gives NOT-READY only for items outside the scripted probes: the lock marker of the
  `mc_headed_v6` wrapper (not present in this repository), the agent session and the reference run.
  Every scripted probe check passes (P1, P3, P4, P5, F3, F6, F9). Evidence: `final-ae72ec92/cold-check.log`.

## Version touchpoints for root

- `ENGINE_API_VERSION` 0.2.18 and `RELEASED_ENGINE_APIS` (`crates/turts_mod_api/src/lib.rs`); version-table rows
  `exports.utilities`, `SupplyScope::Network`, `behaviour UtilityRoute`, `target: UtilityRoute`,
  `Source::UtilityRoutes`, `engine action utility view` (`crates/turts_mod_runtime/src/version_table.rs`).
- `NET_PROTOCOL_VERSION` 11 (`crates/turts_net_protocol/src/lib.rs`); `SnapshotRecords.utilities`,
  `DeltaRecords.utilities`, `OrderTarget::UtilityRoute`, reasons `UtilityNotObserved`, `UtilityLimitReached`.
- Tests that pin these numbers: `turts_mod_api` lib, `turts_mod_runtime` `api_0_2_4.rs`, `raw_presence.rs`,
  `val1.rs`, `version_table.rs`; `turts_net_protocol` `protocol4.rs`, `protocol5.rs`, `repair.rs`, `wire.rs`;
  `turts_server` `session.rs`; `turts_client` `api_keys.rs`; `turts_mod_api` `keys.rs`.
- M5a also targets API18 and protocol 11 on its own branch. Root must renumber one of them at integration.

## Fixture decisions found during the build

- A work order digs only down to an observed reference level. So the demonstration exposes the depth-1 route
  on the ridge by digging the ridge cell down to the flat level, then fills it back.
- The inherited Hold, Pause and Resume buttons gave no visible result within 1 s in the cold-play probes.
  They are not part of the M6 path and are removed from `utility_yard` (`ae72ec92`).

## Limits and items not checked

- Only an aggregate electrical model. No water pressure, sewage gradient, heat loss or storage.
- No segment removal by the player, no corridor or clearance rules between lines, one layer per slot.
- The owner learns the exposure state of its own lines, also under fog. This is a stated rule (spec).
- Bots do not use utilities. The economy-log audit has no replay rule for `UtilityRoute`; it refuses such a log.
- A port removal gives each attached segment its own new junction. The segments do not join each other.
- `capture_live_ports` walks every entity each tick. The UT7 worlds are small; the cost at M7 scale is not measured.
- On a flat map with a fractional size, route observation is not checked (fail-open; flat ground reveals nothing).
- Browser frame time with the utility view on was not measured. Pointer picking during camera shake was not checked.
- No person has played this build. No hosted CI run. No integrated M7 workload.

## Launcher

`bash /home/julien/Projects/TuRTS/target/mc-evidence/play-utilities.sh` starts the prepared build of this branch
with sound off. Its prepared folder must match the worktree HEAD; `mp1-play.sh` refuses a stale folder.
The smoke result is in `launcher-smoke/`.
