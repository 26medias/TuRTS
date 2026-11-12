# Erratum: deltas of one server flush can arrive out of order

Status: erratum for two documents that stay unchanged (decided and recorded documents are not edited
here). Branch `codex/mc-fix-reorder`. Tick rate and transport are Open (spec section 15).

## 1. What is wrong

1. `docs/measurements/m1-network-spike.md:128`: "Over WebSocket (TCP) the order is kept, and
   `deltas_in_order` stayed true in every run."
   This is false for a flush that holds several deltas of different sizes. TCP keeps the order of the
   bytes it is given. lightyear 0.31.0 chooses that order when it builds a flush: with priority off
   (the default) it sorts the candidates of one flush with `packing_order`, fragments first, then
   singles by ascending byte size (`lightyear_transport-0.31.0/src/packet/priority_manager.rs:97-157`;
   candidates are collected singles before fragments in `channel/send.rs:379-410`).
   Test that pins it: `crates/turts_net_lightyear/tests/spike.rs`, `p8_one_flush_reorders_deltas_by_size`.
   Three deltas sent in one tick (314 B, 116 B, 2,110 B; ticks 22, 23, 24) arrived as 24, 23, 22 over
   WebSocket on loopback, with no loss. The P5 runs kept the order because each flush held one delta.
2. `docs/superpowers/specs/2026-10-26-m1-foundation-design.md:288-292`: "When `base_tick` differs from the
   last tick it applied, the client sends one `ResyncRequest` and enters resync." and "A break in the chain
   starts one new resync."
   With a server that runs several ticks in one frame (a stall: `Time<Virtual>` clamps a frame to 250 ms,
   so up to 5 ticks at 20 Hz), the deltas of those ticks wait in one flush and can arrive out of order.
   A strict chain rule then asks for a snapshot though no delta is lost. Measured: a 0.6 s server stall gave
   one resync per stall for the affected client (`reorder-proc-red` in the MC evidence).

## 2. The refinement (client only, no wire change)

`Replica::apply_delta` (`crates/turts_client/src/replica.rs`) holds a delta that is ahead of the chain and
applies it when its predecessor arrives. A resync starts only when `highest_observed_tick - last_applied_tick`
is above the grace, `max_catchup_ticks(tick_hz)` = `1 + floor((250 ms - 1 ns) / tick duration)` (5 at 20 Hz).
The recovery tail keeps the same tolerance. The bound is computed from a real Bevy app in
`crates/turts_client/tests/catchup_bound.rs`. The facts it rests on are pinned in
`crates/turts_server/src/main.rs` (`frame_tests`): `Time<Virtual>` max_delta 250 ms, the fixed timestep, and
one lightyear flush for each frame (the transport send runs once in `PostUpdate`,
`lightyear_transport-0.31.0/src/plugin.rs:674-679`). A truly lost delta still gives exactly one resync, at the
grace (T8 keeps its meaning).

## 3. R2: send in order on the server (not built)

- `prioritize` with priority off sets every effective priority to 0 and sorts by `packing_order`.
- `MessageSender::send_with_priority` exists (`lightyear_messages-0.31.0/src/send.rs:99`), but a priority only
  counts when `PriorityConfig.enabled`. `PriorityConfig::new(q)` enables priority and a bandwidth limiter
  together (`priority_manager.rs:40-47`). A server-only option exists: `PriorityConfig::new(u32::MAX)` and a
  priority that falls with the index of the tick in the frame, on each delta. No wire change.
  Not proven: that the limiter never binds at `u32::MAX`, and how to set the config on the client entities that
  lightyear creates (NOT TRACED).
- Recommended only together with a datagram transport (WebTransport or WebRTC), as defence in depth.
- Other options: `OrderedReliable` for deltas (protocol change on both ends, retransmit and head-of-line
  blocking; rejected by the M1 spike, `m1-network-spike.md:120`); one delta for each flush or a merged delta
  (a wire-rule change at design `:285`).

## 4. Open decision: the transport

The grace is "the most ticks a delta can be overtaken without a loss" = sender bound (frame catch-up, 5 at
20 Hz; 0 with R2) + transport bound (0 on TCP; to be measured on datagrams). When the transport is chosen
(Open: WebSocket, WebTransport, WebRTC), the grace becomes a parameter of the transport. Needs decision then.
A smaller grace on the current sender re-opens false resyncs: the margin is 0 (the worst observed span was
exactly 5 in every reordering run).
