# M1 networking spike: P1 to P7

Task 6a of `docs/superpowers/plans/2026-10-26-m1-foundation.md`. Pass list: design section 8.
Task 6b adds P8.

**Stack under test:** lightyear 0.31.0, messages only, WebSocket, netcode `Manual` tokens
(`default-features = false`, features `std`, `websocket`, `netcode`, plus `client` and `server`).
Bevy 0.20.0. Channels and messages: `turts_net_lightyear::ProtocolPlugin` (Task 5).

**Code:** the commit that adds this file. Its parent is `887391f`. Spike code:
`crates/turts_net_lightyear/src/bin/{spike_server,spike_client}.rs`,
`crates/turts_net_lightyear/tests/spike.rs`, `tests/net/relay.py`.

**Machine:** Linux 7.0.0-38-generic, 28 logical CPUs, rustc 1.99.1, Python 3.12.4. Release
build. No other build ran during P5 to P7. Date: 2026-10-26.

**Command (all items):**

```
cargo test --release -p turts_net_lightyear --features spike --test spike -- --ignored --test-threads=1 --nocapture
```

Final run: `test result: ok. 7 passed; 0 failed`, 209.81 s. The raw JSON of each run goes to
`target/spike/` (`licences.txt`, `p5.json`, `p6.json`, `p7.json`, the stats files).

## Results

| Item | Result | Numbers |
| --- | --- | --- |
| P1 lockfile pinned, licences permissive | pass | `cargo metadata --locked` succeeds; every `[workspace.dependencies]` version starts with `=`; 196 registry packages, all permissive. List: `target/spike/licences.txt` (written by the test) |
| P2 server without render crates; client on wasm32 | pass | `cargo tree -p turts_net_lightyear --features server` has no `bevy_render`, `bevy_winit`, `bevy_pbr`; `cargo check --features client --target wasm32-unknown-unknown` succeeds |
| P3 Hello, Welcome; bad hash rejected | pass | right hash: `welcomed: true`; wrong hash: `rejected_reason: "lockfile hash mismatch"`, `disconnected_after_connect: true`, inside a 2 s client run |
| P4 1,000 `MoveTo`, one `CommandAck` each | pass | `acks` 1000, `ack_seqs_unique` 1000 |
| P5 60 s of `UnitDeltas` in order | pass in the final run; **1 lost delta in 1 of 3 runs** (see "Delta loss") | see the P5 table |
| P6 killed client, tick p99 | pass | baseline p99 405 us (max 444, 341 samples); after the kill p99 127 us (max 250, 160 samples); bound 1,000 us |
| P7 stalled reader for 5 s | pass | see the P7 table |

### P5: bytes per tick (final run, 60 s client run, through the relay without `--rcvbuf`)

`bytes per tick` = relay bytes to the client / deltas received. This includes the WebSocket
frames, lightyear and netcode headers, pings, `Welcome`, and the handshake. `payload` =
postcard bytes of `UnitDeltas` sent / server ticks. `overhead` = bytes per tick - payload.

| Units | Deltas | First, last tick | Lost | To client bytes | Bytes per tick | Payload per tick | Overhead per tick |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 1 | 1197 | 4, 1200 | 0 | 153,677 | 128.4 | 13.8 | 114.6 |
| 1,000 | 1197 | 4, 1200 | 0 | 12,434,084 | 10,387.7 | 9,853.5 | 534.2 |

All P5 runs:

| Run | Units 1: deltas, lost, bytes per tick | Units 1,000: deltas, lost, bytes per tick |
| --- | --- | --- |
| 1 | 1196, **1**, 125.2 (test red: `deltas 1196 != last_tick - first_tick + 1 = 1197`) | not run (the test stops at the first failure) |
| 2 | 1197, 0, 136.7 | 1197, 0, 10,389.2 |
| 3 (final) | 1197, 0, 128.4 | 1197, 0, 10,387.7 |

Payload accuracy: the plan's formula divides by all server ticks (1,220). This includes the
ticks before `Welcome` (no send) and the ticks after the client stopped (the server sends until
it detects the disconnect). One delta for 1,000 units at a tick of two varint bytes is 9,878
bytes (computed), so the server sent about 12,021,277 / 9,878 = 1,217 deltas. The payload per
sent delta is thus 0.25 % above the payload per tick in the table. The server computes the
postcard size with a function that agrees with `postcard::to_allocvec` 1.1.3 in 36 cases (units
0 to 20,000, ticks 1 to `u32::MAX`).

### P7: client stalled for 5 s (final run)

Server with 1,000 units behind `relay.py --rcvbuf 4096 --pause-after 10 --pause-secs 5`.
Send-Q is sampled each 500 ms with `ss -tni` on the server's WebSocket port.

| Measure | Value |
| --- | --- |
| Pause length | 5,000 ms |
| Client | stayed connected (`disconnected_after_connect: false`); `last_tick` 400 > server tick 320 at the resume |
| Send-Q at the middle of the pause | 562,971 bytes (limit: at least 262,144) |
| Send-Q peak | 998,532 bytes |
| Tick p99 before the pause (own baseline) | 427 us |
| Tick p99 in the pause | 396 us (max 412 us); bound 1,000 us |
| RSS before, peak in the pause, after | 11,898,880; 11,898,880; 11,939,840 bytes |
| RSS rise in the pause | 0 bytes (limit 16 MiB) |
| Delta payload produced in the pause | 1,185,360 bytes |
| Deltas received, first, last tick | 284; 4; 400 |
| Lost deltas | 113 (recorded, not judged) |
| Relay bytes to client, to server | 4,122,225; 34,131 |

First P7 run (same code, before the instrument repair below): Send-Q at mid-pause 552,492,
peak 988,053 bytes; p99 397 us before, 410 us in the pause; RSS rise 0; lost deltas 109.

## Delta loss: cause traced

lightyear 0.31 keeps one ready message per `SequencedUnreliable` channel. When a newer message
arrives before the application reads the older one, the older one is dropped
(`lightyear_transport-0.31.0/src/channel/receive.rs`, `RecvState::UnreliableSequenced`,
`ready: Option<..>`, and `prepare_receive`: "A newer sequence invalidates an older completed
message even if the application has not drained it yet"). Thus, when two `UnitDeltas` arrive
between two client reads, the client loses the older one. TCP does not lose them.

Observed:
- P5 run 1, 1 unit, no small buffer: 1 delta lost in 1,197.
- P7 with `--rcvbuf 4096` and no pause (the P7 plant run): 5 lost.
- P7 after a 5 s stall: 109 and 113 lost. The backlog arrives at once and only the newest
  delta of each read survives.

This replaces the plan's "Cause NOT TRACED" for the P7 loss. Consequence for the design: a
client that receives deltas in bursts loses deltas without network loss, and each loss is a
chain break (design section 5). The resync path handles it, but the count of `ResyncRequest`
grows with bursts. The spike gate must judge P5 with this cause known.

## Plants (each one reverted; the final run above is on the clean code)

| Item | Plant | Result |
| --- | --- | --- |
| P3 | `Welcome` for any hash | red: `welcomed with the wrong hash` (left `true`, right `false`) |
| P4 | skip every second `CommandAck` | red: `acks != 1000` (left 500) |
| P5 | stop sending deltas after tick 200 (10 s) | red: `deltas 197 < 1140` |
| P6 | `sleep(2 ms)` in `FixedUpdate` when no client is connected | red: `tick p99 after the kill 2284 us > baseline 425 us + bound 1000 us` |
| P7 a | relay `--pause-secs 0` | red: `no back-pressure: Send-Q 10321 bytes at the middle of the pause` |
| P7 b | keep 24 MiB of non-zero bytes from the first frame where `/proc/net/tcp` `tx_queue` on the WebSocket port is at least 256 KiB | red: `RSS rise 25317376 bytes > 16777216 bytes` (allocation at `tx_queue` 271,803) |

P1 and P2 have no plant in the plan.

Instrument repair: the first P7 a run went red for a different reason (`no tick samples in the
window`), because a run with no pause has no tick samples in the pause window. The test now
uses NaN for an empty window. The back-pressure assertion comes before the p99 assertion, so
the plant goes red on "no back-pressure". The final P7 run is on the repaired test.

## Studio calls and refinements (confirmed by the orchestrator)

- Bound for P6 and P7 is one-sided: p99 <= baseline p99 + max(10 % of baseline, 1 ms).
- P6 uses 1,000 units. Baseline window: 1 s to 20 s after the client start. Kill window: 11 s
  to 20 s (client killed with SIGKILL at 10 s).
- P7 compares with its own window before the pause (2 s after the client start to the pause).
- `--stats` is JSON lines, one line per second. Each line adds `unix_ms` and `tick_us` (the
  raw tick samples of that second) to the fields in the plan. `resident_peak_bytes` is the
  peak RSS in the line's second.
- A tick sample is the wall time of one app frame that ran a fixed tick, from `First` to
  `Last`. It includes receive, the tick, and send.
- P7 b triggers on the kernel `tx_queue`, because lightyear and aeronet expose no count of
  pending sends (aeronet's native session queue is an unbounded `mpsc`).

## Facts found during the spike

- lightyear inserts `Started` when `Start` triggers, before the port is bound
  (`lightyear_netcode-0.31.0/src/server_plugin.rs:418`). `Linked` on the server entity marks
  the bound port (`lightyear_aeronet-0.31.0/src/endpoint.rs:55`). `spike_server` prints
  `listening` on `Linked`.
- `App::run` hands the world to the runner. Write any exit report before `AppExit`.
- With `--rcvbuf 4096` and no pause, Send-Q stays at or below 10,368 bytes.
