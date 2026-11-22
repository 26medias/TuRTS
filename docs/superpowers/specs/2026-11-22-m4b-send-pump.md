# M4b send pump: no state backlog when a frame runs many ticks

Status: Baseline (reviewed by Viktor, technical director: PASS-WITH-CONDITIONS, option B plus the
loss timeout floor of section 8; review `.swarm/reports/T-1-pump-design-review.md`). Date: 2026-11-22. Track:
Foundation (server send path of the restricted-state transactions). Author: Lena (net-eng).
Approved as M4b work by the CEO through the PM, 2026-11-22. Base: `56a008b4`. Branch:
`codex/m4b-send-pump`. Worktree: `/home/julien/Data/work-dir/TuRTS/m4b-send-pump`.
Source: `.swarm/reports/T-1-lag-trace.md`.

Fixed by the PM, not open in this design: `WINDOW` stays 64. The wire format, `NET_PROTOCOL_VERSION`,
`Time<Virtual>::max_delta` (250 ms) and the tick rate do not change. If the work shows that one of
them must change, the work stops and the PM gets a report.

## 1. Problem

Measured in the GM4 browser run and in a server-only replay (report sections 3.1 to 3.3):

- When a tick costs more than the tick period, one application frame runs up to 5 fixed ticks
  (`max_delta` 250 ms at 20 Hz).
- Each fixed tick queues one state transaction for each session (`outbound.rs`, `send_deltas`,
  `FixedLast` → `outbound_knowledge.rs`, `queue_tick`). A transaction is `chunk_count + 2` frames, so
  3 frames or more.
- `pump` runs one time for each application frame (`outbound_knowledge.rs:250`, `PostUpdate`, before
  `MessageSystems::Send`) and sends at most `FRAMES_PER_PUMP` = 16 frames for each session.
- So a 5-tick frame makes 15 frames or more and sends 16 at most. The margin is 0.33 tick for each
  frame. A backlog of 57 ticks drained at 1.2 ticks per second. The client showed a world 2.0 to 2.9 s
  older than the server world. Order results use `ReliableChannel` and arrived before the state.

The limit of 16 was written for 1 ms frames with 0 or 1 tick. It does not hold for a frame with many ticks.

Not in scope: the 55 ms tick cost (simulation, Kenji). The repair must be correct with and without it.

## 2. Requirements

- R1. A session with an empty queue at the start of a frame, in which the frame runs N ticks (1 to 5),
  each with a transaction of at most 16 frames: all the frames of the N transactions go to the
  transport in that frame, if the window permits it.
- R2. Order: frames leave in transaction order, and in frame order in a transaction. A cancel keeps its
  place against the frames (unchanged rule: `emit_cancel` before the restart).
- R3. Backpressure: `outstanding` never goes above `WINDOW` (64). The retained-byte limits
  (`MAX_PENDING_STATE_BYTES`, `MAX_POST_BASE_PATCH_BYTES`), the 5 s transaction time-out and the restart
  window do not change.
- R4. One slow session does not delay another session (unchanged; test
  `bounded_registered_send_window_does_not_block_a_second_connection`).
- R5. A frame with 0 ticks still pumps: a large baseline, the acks and the time-out need it.
- R6. A trace of the send queue exists as a local test hook, so the 3.6 s stall (report 3.2, NOT TRACED)
  can be traced.

## 3. Options

### A. Pump one time for each fixed tick

Add a second call of `pump` in `FixedLast` after `send_deltas`. Keep the `PostUpdate` call (R5).

- Send limit for a frame with N ticks: 16 × (N + 1), then `WINDOW`.
- Cost: `pump_one` runs N + 1 times for each session and frame. Each run reads the transport acks, the two
  receivers and `Instant::now()`. lightyear receives one time for each frame (`PreUpdate`), so runs 2 to
  N + 1 get no new ack. The window can only fill in a frame.
- Risk: the time-out and cancel logic runs in the middle of a fixed tick burst. A restart in `FixedLast`
  of tick k is followed by `queue_tick` of tick k + 1 in the same frame. This path exists today only
  across frames. It is new behaviour to prove.
- Two call sites with one shared budget rule to keep in step.

### B. One pump for each frame, budget scaled by the ticks of the frame (recommended)

Keep the one `PostUpdate` call. The frame budget becomes `FRAMES_PER_PUMP × max(1, ticks_in_frame)`.
`ticks_in_frame` is the difference of the simulation `Tick` between two pump calls (a `Local<u32>` of
the last value; a resource with the same content if a test must set it).

- Send limit for a frame with N ticks: 16 × max(1, N), then `WINDOW`. With N = 5: 80, so the window
  (64) is the limit. That is 21 ticks of 3 frames: 16 ticks of margin for each frame against 0.33 today.
- Acks, cancels, the time-out and the restart run one time for each frame, as today. No new path.
- A frame with 0 or 1 tick has exactly the behaviour of today. The normal 20 Hz case does not change.
- One call site. The change is the loop bound of `pump_one` and one counter.

### Comparison

| | A: for each fixed tick | B: scaled budget |
| --- | --- | --- |
| R1 | yes | yes |
| R2 order | yes (one queue) | yes (one queue, code path of today) |
| R3 window | yes | yes |
| R5 | needs both call sites | unchanged |
| New control-flow paths | restart in a tick burst | none |
| Work for each frame | N + 1 pump runs | 1 pump run |
| Bytes to the socket in one frame | at most 64 KiB | at most 64 KiB |

In the two options the frames reach the socket at the same time: lightyear sends one time for each
frame, after `PostUpdate`. A does not put a frame on the wire earlier than B.

Recommendation: **B**.

## 4. Order and backpressure (option B)

- Order. The queue is `pending`, then `patches` front to back. `pump_one` takes the first transaction that
  has `end_sent == false`. A larger loop bound does not change the choice. `KnowledgeChannel` is ordered
  reliable, so the wire order is the send order.
- Window. The loop stops when `outstanding.len() >= WINDOW`. An id leaves `outstanding` only on a transport
  ack, read at the start of `pump_one`. The larger budget cannot go above the window.
- Client receive. The client assembler gets at most 64 frames of 1024 bytes in one server frame. It gets
  the same number today across four 1 ms frames at the join. No new client limit is reached. To verify:
  the join and resync acceptance tests.
- Burst size. At most 64 KiB for each session and frame, bounded by the window. Today the same 64 frames
  can leave in 4 ms.
- Starvation. The budget is for each session. Sessions do not share it.
- Remaining limit, stated: if the transactions of one frame need more than 64 frames (5 ticks of more than
  12 frames each), a backlog still grows until the acks of the next frame. Then the limit is the window,
  which the PM fixed. The trace (section 5) shows this case. It is a report, not a silent stall.

## 5. Send-queue trace (test hook)

New option `--send-trace PATH` (listed in `--help`, local file, not served, as `--stats`). One CSV line
for each session and pump call in which the queue is not empty or a frame was sent:

`wall_ns,sim_tick,slot,ticks_in_frame,budget,frames_sent,outstanding,queued_transactions,queued_bytes,oldest_tick,tip_tick,event`

`oldest_tick` is the tick of the oldest transaction that is not acknowledged. `tip_tick − oldest_tick` is
the backlog in ticks. `event` is empty, or `restart_timed_out`, `restart_cancelled`, `restart_superseded`,
`restart_buffer_limit`, `window_full`. No player data is in the line. Off by default: no cost in a normal run.

## 6. Tests (through the server path)

1. Red then green, app level (`turts_server`, real `send_deltas` and `pump`, a session with a real
   lightyear `Transport`, manual time): one frame of 250 ms runs 5 ticks; each tick changes state so each
   transaction has 4 frames. Assert that the 20 frames are in the transport after the frame and that the
   queue has no unsent frame. On `56a008b4` this is red (16 frames).
2. Window: the same with transactions that need more than 64 frames in the frame. Assert `outstanding`
   is 64, the order of the frames is the queue order, and the rest leaves after the acks.
3. No change at 20 Hz: a frame with 1 tick and a baseline of more than 16 frames sends exactly 16.
4. The existing tests of `outbound_knowledge.rs` and `outbound_knowledge_pipeline_tests.rs` stay green
   with no edit of an expected value.
5. Process level: the server-only replay (`replay.sh`) before and after: tick rate and tick cost must be
   equal in the two builds (the repair does not touch the simulation). The red and the green outputs are kept.
6. GM4 with a client and `--send-trace` (planned with Marco; one browser run, outside the GM7 slots):
   compare, before and after the 34-unit order, the tick rate, the input latency (order emitted to result)
   and `applied_tick − replica_tick`. Expected after the repair: the backlog stays below 6 ticks while the
   server runs 5-tick frames. The trace also answers the 3.6 s stall.
7. Gates: fmt, clippy, the three crate test sets, `boundary.sh`, `acceptance.sh`.

## 7. Questions for the reviewer (answered: Q1 B; Q2 yes for M4b; Q3 a separate option)

- Q1. A or B?
- Q2. Is the stated remaining limit (section 4, more than 64 frames in one frame) acceptable for M4b with
  `WINDOW` fixed?
- Q3. Is `--send-trace` as a separate option correct, or must the fields go into the `--stats` session file?

## 8. Addendum after the trace: the packet loss timeout (decided by the reviewer, 2026-11-22)

### 8.1 What the trace showed

The limit of 16 does not explain the first backlog: at 5 ticks of 3 frames the input is 15 frames.
Counted with the old limit, the GM4 34-unit order and the native headless client
(`.swarm/reports/T-1-lag-trace.md` section 6b): from the first slow frame no state frame is acknowledged,
the window of 64 is full in 1.3 s, the pump sends nothing for 6.1 s, the backlog reaches 111 ticks, and the
5 s time-out makes one full restart.

Cause, lightyear_transport 0.31.0: a sent packet is declared lost after max(10 ms, 1.5 x RTT)
(`packet/nack.rs:27-47`). The declaration removes the record that maps the packet to its messages
(`plugin.rs:326`), so the later acknowledgement of the packet gives no message acknowledgement and the pump
gets no window credit. A slow server frame is 55 to 277 ms and an acknowledgement is seen 2 frames after
the send. The RTT estimate was 11.6 ms.

### 8.2 Repair

`SERVER_PACKET_LOSS_TIMEOUT_FLOOR` = 4 x 250 ms = 1 s (`outbound_knowledge.rs`), set with the public
`Transport::set_packet_nack_settings` on each new transport that has `ClientOf` (a server session link),
in `PreUpdate` before `TransportSystems::Receive`. The lightyear maximum (3 s) does not change.

- Basis of 1 s: `max_delta` is 250 ms, so a frame that runs fixed ticks is at most about one `max_delta`
  of simulation work plus the frame overhead. The acknowledgement is seen 2 frames after the send
  (measured maximum: 2 frames, 554 ms, section 8.4). 4 x `max_delta` gives a margin of about 2 frames.
- Server links only. A client transport keeps the lightyear default. The production client sends only
  orders and transaction acknowledgements; they are small, and the client was not the cause in the trace.
  A browser frame can be longer than 10 ms, so the client can declare false losses too: not measured, not
  in the scope of M4b (server only, PM).
- Real packet loss: for a reliable channel `receive_nack` does nothing (`channel/send.rs:310-314`); a
  reliable message is resent on a timer (`rtt_resend_min_delay` 50 ms, `protocol_plugin.rs:30-33`). So the
  longer floor does not delay the recovery of a lost reliable message. It only delays the loss
  declaration, which nothing in the server uses for a reliable channel. The unreliable `StateChannel`
  path does not use acknowledgements.
- Transport dependency: on WebSocket no packet is lost, so the floor has no cost. The transport is an
  Open decision (spec 15). Review the value if the transport changes.
- No wire change. `WINDOW`, `max_delta` and the tick rate do not change.
- The experiment switch `TURTS_NACK_MIN_MS` is removed. It is not in the branch.

### 8.3 Tests added (`outbound_knowledge_pipeline_tests.rs`, real lightyear transports, frames of 250 ms with 5 ticks, 4 frames for each tick)

| Pump limit | Loss timeout floor | Peak send lag | Result |
| --- | --- | --- | --- |
| 16 (old) | 10 ms (default), the state of 56a008b4 | 284 ticks | RED |
| scaled (B) | 10 ms (default) | 284 ticks | RED |
| 16 (old) | 1 s | 60 ticks | RED |
| scaled (B) | 1 s | 0 ticks | GREEN |

The bound is 5 ticks: the ticks of one slow frame may wait for the next pump; more is a backlog. The bound
is checked at the peak (after 4 slow frames) and at the end (60 slow frames). At 1 tick for each frame
the same harness leaves a lag of 0 before the slow phase, and the existing 50 and 100 ms RTT test
and the non-returning viewer test run with the new floor and stay green.
Other tests: a frame with 0 ticks or 1 tick sends exactly 16 frames of a larger baseline; 5 ticks that
need more than 64 frames stop at the window and keep the order; mutation run: with the window check
removed that test is red (the harness asserts `outstanding <= WINDOW`); unit tests of `frame_budget`,
of a tick that goes back (C1) and of the `ClientOf` filter (N2).

### 8.4 Process runs (GM4 34-unit order, native headless client; numbers in the report)

See `.swarm/reports/T-1-pump.md`.
