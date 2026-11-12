# MP1 recorded measurements

Recorded, not judged. MP1 keeps its numeric limits in the design (section 6.7.8 and row LOB1); this file records what a run printed.

## Disconnect notice L, native release

Task `L-ROOM0`, 2026-11-12. L is the time from the end of a headless client process to the record `session_authority_removed` in the private session observer file (`<stats>.sessions.jsonl`). The record is written by the observer `on_session_removed` (`crates/turts_server/src/session_diagnostics.rs:383`), which fires when the `Session` component leaves the connection entity (the entity is despawned). Both clocks are the host unix clock of one machine.

- Machine: Intel Core i9-14900K, 28 logical CPUs, Linux 7.0.0-42-generic x86_64, `rustc 1.99.1`. Local machine, not the 4-vCPU target of D58. One server and one client, no other load from this test.
- Build: `cargo test --release --locked -p turts_server --test connection_end -- --ignored --test-threads=1 --nocapture` at the lane head `d7d7e6d` plus the new files. Server and client are release binaries.
- Tool: the process test `crates/turts_server/tests/connection_end.rs`. It prints `L_sigkill_ms` and `L_clean_exit_ms`.

| Case | Run | L (ms) |
| --- | --- | --- |
| SIGKILL of the client | `mp1-lroom0-conn-end-run1` | 9998.9 |
| SIGKILL of the client | `mp1-lroom0-conn-end-probe2` | 9996.8 |
| SIGKILL of the client | `mp1-lroom0-conn-end-probe3` | 9998.8 |
| Clean exit of the client (`--run-secs 4`) | `mp1-lroom0-conn-end-run1` | 9996.0 |
| Clean exit of the client (`--run-secs 4`) | `mp1-lroom0-conn-end-probe2` | 9996.9 |
| Clean exit of the client (`--run-secs 4`) | `mp1-lroom0-conn-end-probe3` | 9995.7 |
| SIGSTOP of the client (probe, not in the committed test) | `mp1-lroom0-conn-end-probe-sigstop` | 10000.3 |

The runs with `probe` in the label carried a temporary logging patch that is not in the commit (see `docs/research/mp1/trace-connection-end.md`). The patch only logs; the L values do not differ from the run without it.

What the figures show: L is the 10 s netcode timeout in every case, including a clean process exit. The transport signal on the same connection entity comes about 2 to 3 ms after the end (see the trace note). The record of this table is the despawn, not the transport signal.

Not measured: the browser (a tab close or reload), a release build on the 4-vCPU target, a server under load (the gate 1 probe ran at load 13 in a debug build, design 6.7.8).

## Transport signal on the connection entity, native release

Source: the network-engineer review probe of `L-ROOM0` (a temporary logging patch and a driver script, not a committed test): `target/mc-evidence/mp1-lroom0-review-probe-drive/command.log` (in the main worktree; the same log) (`probe-drive.log`). Time from the end of the client to `Add<Disconnected>` (lightyear) on the connection entity. The author probe of the first version (`docs/research/mp1/trace-connection-end.md`) agrees within about 1 ms.

| Case | Time to `Add<Disconnected>` | Reason text |
| --- | --- | --- |
| SIGKILL of the headless client | +4.3 ms | `ResetWithoutClosingHandshake` |
| Clean exit of the headless client | -3.9 ms (before the exit was read) | `ResetWithoutClosingHandshake` |
| SIGSTOP of the headless client | +9998.7 ms | `Disconnected by peer` (netcode timeout) |
| Raw WebSocket: close frame, then FIN | +0.4 ms | `Disconnected by peer` |
| Raw WebSocket: close frame, TCP held open | +0.9 ms | `Disconnected by peer` |
| Raw WebSocket: FIN, no close frame | +1.0 ms | `ResetWithoutClosingHandshake` |
| Raw WebSocket: RST | +0.9 ms | `ConnectionReset` |

A reload with the same id 500 ms after a SIGKILL is refused with "This game is open in another window" (today's code). Not measured: a real browser.

## UnitId lookup, scan and map

Task `L0-SIM`, 2026-11-12. Recorded, not judged. One lookup of a unit by its id: a scan of all entities (`world.iter_entities().find(..)`, the form that the sites used) against `UnitIndex::get`. The scan looks for the last unit, the worst case; the map takes the same id.

- Machine: Intel Core i9-14900K, 28 logical CPUs, Linux 7.0.0-42-generic x86_64. Local machine, not the 4-vCPU target of D58. Release build, `bevy_ecs` 0.20.0.
- Tool: the ignored test `unit_index::tests::lookup_cost` (`crates/turts_sim/src/unit_index.rs`): `cargo test -p turts_sim --release --locked --lib unit_index::tests::lookup_cost -- --ignored --nocapture` (run `mp1-l0sim-lookup-cost`). It times 200 scans and 200,000 map lookups of one id in a world of N units.

| Units | Scan (ns for each lookup) | `UnitIndex::get` (ns for each lookup) |
| --- | --- | --- |
| 1,000 | 8,062 | 12 |
| 5,000 | 29,937 | 13 |

The performance lens of the design measured 5.0 and 25.9 us for the scan and 0.026 us for the map (D section 9); this run is the same order. The map cost does not depend on the unit count in this range.

Sites that read the index after `L0-SIM`: `economy/production.rs` (`queue_production`, `cancel_production`, `run_production` for each active producer in each tick, `try_output`), `economy/transaction_m3.rs` `find_unit_entity`, `economy/transaction.rs` `inventory_owner`, and `validate_target` in `orders/apply.rs` (the two unit-target checks). The test of each site points the index entry at another entity and shows that the site follows the entry and does not scan.

The scan of the ordered attackers of a removed unit (`economy/transaction_m3.rs`, the `iter_entities().filter_map(..)` that builds `ordered_attackers`) is kept (design 9, site 2, second case: "if those writers are not in one place, lane 0 keeps the scan and records its cost"). The writers of `UnitOrders` are not in one place today (about 62 write lines, plan task `L0-WIRE`), so a reverse set has no one place to stay in step. It runs once for each removed unit and does one full pass of the world: the cost of the scan row at the unit count of the match (about 8 us at 1,000 units, about 30 us at 5,000; the pass gets two components in each entity, so it can cost more than the `find` of the table: NOT TRACED, not timed on its own). `L0-WIRE` writes every `UnitOrders` through one function; the reverse set belongs there.

