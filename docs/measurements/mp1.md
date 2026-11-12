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

Source: the network-engineer review probe of `L-ROOM0` (a temporary logging patch and a driver script, not a committed test): `/tmp/claude-1000/-home-julien-Projects-TuRTS/bf418660-b272-4bb3-8628-8d9ef2677d1d/scratchpad/l-room0-review/` (`probe-drive.log`). Time from the end of the client to `Add<Disconnected>` (lightyear) on the connection entity. The author probe of the first version (`docs/research/mp1/trace-connection-end.md`) agrees within about 1 ms.

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
