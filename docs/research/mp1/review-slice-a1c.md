VERDICT=FAIL HIGH=1 FINDINGS=6 HIGH=1 RUNS=41 NOT_CHECKED=5

# Review of pre-MP1 task A1c (the replay validator is deleted)

Candidate: b552c71 on `pre-mp1/a1c-delete` (base 8722f8e). Reviewer: qa-lead, review mode, 2026-11-12.
Lane: `/home/julien/Projects/TuRTS-pre-a1c-review`, cache `target/mc-pre-a1c-review`. Evidence:
`/home/julien/Projects/TuRTS/target/mc-evidence/a1c-rev-*`. Scripts: scratchpad `a1c-review/`
(`plant.py`, `run_plant.sh`, `run_base.sh`, `js_keys.js`, `serverlog.sh`, `e8.sh`, `deps_probe.py`).

Lens: after 18,677 lines of validator are gone, what fault can no instrument see?

## Findings

1. **High. Server order conversion has no cover (N10).** `crates/turts_server/src/orders.rs:100`
   `allowed_units: definition.units.clone()` changed to `Vec::new()` stays GREEN on all 842 tests of
   sim, audit, checks, pack and server at b552c71 (`a1c-rev-plant-n10-allowed`). At 8722f8e the replay
   is red in 8 tests with "order command 1 differs from its immutable catalog definition"
   (`a1c-rev-base-n10-on`; text only in the deleted `replay.rs:1871`, `validate_order_policy`
   `:1789`). The sim checks the unit list only when it is not empty (`orders/apply.rs:165`). A hostile
   client can then give an order to a unit type that the mod does not list for that order (example
   `mods/muster/orders/rush.ron` lists only `muster:trooper`); only the capability check stays. The
   exploit itself is NOT TRACED by a run. Fix: port the catalog comparison of `validate_order_policy`
   (behaviour, target kind, unit list, queue depth, policy: a pure catalog rule, no sim model) into the
   ledger audit, or add a `turts_server` unit test that `to_sim` copies each field from the
   definition. Show it red on N10 and N13.
2. **Medium. The resting set of the movement system has no cover (N3).** `movement/ecs.rs:70`
   `Behaviour::Attack` changed to `Behaviour::Move` stays GREEN on all 842 tests
   (`a1c-rev-plant-n3-resting`). At 8722f8e the replay is red at `mc_orders.rs:566` "movement
   transition changed nonmovement state" (`a1c-rev-base-n3-on`); with `TURTS_AUDIT_REPLAY=off` the
   only reds are the 3 known self-checks (`a1c-rev-base-n3-off`). Effect: point-attack members lose
   their resting progress, so the resume rule at `orders/run.rs:314-330` reads a different state, and
   Move members keep `MoveProgress` and pools after arrival (effect on play NOT TRACED by a run).
   `kernel_reference` cannot see it: it compares kernels, not this ECS glue. Fix: an outcome test
   that a resting point-attack group keeps `MoveProgress::Resting` at its slots and that a finished
   Move group holds no progress and no pool; show it red on N3.
3. **Low. The replaced self-check `economylog.rs:331` holds another property.** The A1b check was
   "an Orders operation relabeled `Other` is rejected". A1c tests a relabel to `Command`. Probe (test
   text back to `Other`): the ledger returns `Ok` (`economylog.rs:333` unwrap_err on Ok,
   `a1c-rev-probe-p331-dump`). The comment at `economylog.rs:245-248` says the movement tests and
   the writer scan hold it; the writer scan is static and cannot see a phase label. Fix: state the
   deletion in the plan (section 1a kind 3), or add a phase rule.
4. **Low. `full_state` makes four oracle checks compare the model with itself.**
   `crates/turts_server/tests/economy.rs:57-104` takes `bounds`, `resources`,
   `compiled_durations`, `unit_types` from `template`. In the log-based oracle path, the
   immutability rule `crates/turts_pack/tests/common/accounting.rs:449-456` then compares the
   template with itself. `template` is an in-process sim boot (`economy.rs:1304-1318`), not a model
   independent of the sim. Plant N12 (bounds grow each tick): the log-based oracle tests
   (`e9_base_gather_trace_replays_node_depletion_in_process`, `decoded_moving_gatherer_…`) stay
   green; `full_state_rebuilds_the_audit_state` (`economy.rs:606`) and the in-process `turts_pack`
   oracle (34 tests) are red. So the property is held, but not by the oracle that reads the log.
   `units` is rebuilt from the logged `economy.units`, so it stays real data. Fix: say this in the
   `full_state` doc comment; no code change needed.
5. **Low. The E8 reorder is not a named change.** `economy.rs:3943` decodes without validation and
   `:3979` validates after the test's own checks. Not an assertion change, but section 1a lists only
   the re-pin. Fix: record it as a refinement of the plan.
6. **Low. Unused dependencies.** `crates/turts_audit/Cargo.toml` keeps `bevy`, `bevy_app`,
   `bevy_math`; no source uses them. The boundary rule (`scripts/boundary.sh:71-75`) does not care:
   `boundary ok` with them (17/17 tests) and without them (`a1c-rev-deps-boundary`). Removal needs
   the 3 lock lines `Cargo.lock:8608-8610`, else `--locked` fails ("cannot update the lock file",
   `a1c-rev-deps-nolock`). Fix: remove them in A1d with the lock lines.

## Fault table (head b552c71; combined set 842 tests, green baseline `a1c-rev-combo-green1`)

| Fault | Red at b552c71 (first sites) | Replay at 8722f8e |
| --- | --- | --- |
| F1 first open slot `rposition` (`slots.rs:53`) | `session.rs:840`, `:1125`; `economy.rs:4722`; `t3_claim.rs:72`; 11 tests | (A1b: red) |
| F2 `Position` write in `combat.rs` | `movement_writers.rs:401` (named); 17 tests | (A1b: red) |
| F3 cost - 1 (`production.rs:107`) | accounting oracle, 38 tests; ledger green | not run |
| F4 NotOwner refusal changes `next_pool_id` (sim, `apply.rs:101`) | `session.rs:2673` only; ledger has no real-log refusal on this path | not run |
| F5 `LedgerAudit.unit_orders` empty | `ledger_logs.rs:518`, `economy.rs:2826`, `mc_orders.rs:612` | not run |
| N1 ECS speed x1.25 | 32 tests (oracle movement model, `group_movement`, `m3_m2_stream`) | not run |
| N2 ECS radius x0.5 | 13 tests (`group_movement`, `gather_depletion`, `economy.rs:1142`) | not run |
| N3 resting set Attack->Move | **GREEN** | red `mc_orders.rs:566` |
| N4 spawn +0.5 x | 11 tests (`accounting.rs:157` in process, `unit_boot.rs:367`); ledger green | not run |
| N5 spawn type id 0 | same 11 tests; ledger green | not run |
| N6 dispatch cursor +2 | `orders/tests.rs:1724` only | not run |
| N7 gather reach x3 | 19 tests incl. log-based `e9_base_gather_trace…` | not run |
| N8 facing `None` (plant b) | `ledger_logs.rs:559`, `economy_packages.rs:67`, `export_m3.rs:222` | not run |
| N9 NotOwner -> NotApplicable | 7 tests (`session.rs:2663`, `orders_plain.rs:198`, `orders.rs:172`) | not run |
| N10 server `allowed_units` empty | **GREEN** | red, 8 tests |
| N12 bounds grow (a dropped field) | 34 in-process tests, `economy.rs:606`; log-based oracle green | not run |
| N13 server queue depth 255 | `session.rs:1750`, `:2760` | red, 10 tests |
| P331 phase relabel `Other` | ledger `Ok` (probe) | (A1b: red) |

Each plant: anchor count 1, restore by copy, `git status --porcelain` empty after each run.

## Judgements

- **`full_state`:** the oracle stays independent for the 22 kept fields and for `units` (rebuilt
  from logged `economy.units`). For the four immutable fields and `movement_dispatch` it reads the
  template or a default, so its immutability rule is a tautology on the log path (finding 4). The
  spawn plants N4 and N5 are red in the in-process oracle; no non-ignored log-based test spawns a
  unit (E8 and E9 process tests are ignored; E8 green in release, see below).
- **E8-mixed:** independent. The check "rejected seq 5 is write neutral" is at 8722f8e too, with the
  same text and comparison (logged boundary `before == after`); it reads the sink snapshots, not
  `turts_audit`. Design 9.2 names it. Run: plant red at `economy.rs:3956` with that text
  (`a1c-rev-e8-plant-test`); clean run green in release (`a1c-rev-e8-green-test`; this run also
  executes the three reservation self-checks with their exact `reservations.rs` texts).
- **Log format:** a file format change, not a wire change. No reader outside the tests reads a
  dropped field. Real sink log (debug `turts_server`, forge, 300 ticks, `a1c-rev-serverlog-forge`):
  its snapshots hold none of the six dropped fields; the readers `m2-audit.js`
  (`validateAuditRecords`, `auditStateAt`, `projectAuditOwnerWorld`, `worldChangeMatchesAudit`),
  `m2-oracle-lib.js` (`readAuditRecords`, `lastAuditTick`) and `m3-oracle.js`
  (`assertConservationStream`, 302 ticks) pass, and a Proxy records that they read only the key
  `economy`. Red probe: with `economy` removed the same run fails ("audit state has no complete
  economy WorldView"). `stats.rs:284` does not read the log; `scripts/m2-browser.sh` and
  `scripts/m3-browser.sh` only pass the path. The 8 headed `ci-browser` cases were not run.
- **Equal values:** trace sha256 base d4c73ebe…, haul 805309b2…, acme 19dcf9a3…, cedar 0db10e01…
  equal; stream bcbf72c2… equal (`cmp` 0 against the emitter copy; `m3_m2_stream` green);
  `probe_neutral`: `frame_chains=d0d1b1c572163424`, `muster_world=aa9db740578ed5c1`, feature off
  equals on.
- **Merge:** `git merge-tree` against 86a5324: no conflict, tree fac2f7fd. Scratch copy of that
  tree: `cargo test -p turts_sim --locked --features test-probes` 348/348; release
  `kernel_reference` 2/2 (49.38 s); `turts_server` + `turts_audit` 185/185. Copy deleted.

## Commands (all through `exec_in_lane_v4.py --audio`, labels `a1c-rev-*`)

Green: `cargo test -p turts_server --locked` (`server-green`), `-p turts_audit -p turts_checks`,
`-p turts_sim --features test-probes`, `-p turts_pack`, combined set (`combo-green1`, 842/842).
Plants: `cargo test -p turts_sim -p turts_audit -p turts_checks -p turts_pack -p turts_server
--locked --no-fail-fast --features turts_sim/test-probes` once per plant (`plant-F1` … `plant-n13-depth`).
Base: `cargo test -p turts_server --locked --no-fail-fast` at 8722f8e (lane detached there, then back
to b552c71): `base-clean-on`, `base-n3-on`, `base-n3-off`, `base-n10-on`, `base-n13-on`.
Others: `probe-p331-dump`, `build-bins`, `serverlog-forge`, `m2-trace`, `probe-neutral`,
`deps-nolock`, `deps-lock`, `deps-boundary`, `merged-sim`, `merged-kernel-release`,
`merged-server`, `e8-plant-*`, `e8-green-*`, `fmt` (exit 0).

## NOT TRACED

- The 8 headed `ci-browser` cases; EL1 and E9 in release; `acceptance.sh`; clippy; the registry
  runner itself (the builder's "20 rows red with the pinned text").
- The play effect of N3 and the exploit of N10 (no scenario run).
- Whether a measurement document pins the EL1 log byte count, which the smaller log changes.
- The builder's full plant list (not in the hand-over text).

What I changed: none in the repository (the lane was moved to 8722f8e and back, detached; tree
clean). This report file only, on request.
