---
type: decision
date: 2026-10-26
tags:
  - turts
  - decision
  - networking
ai-first: true
source: mcp
status: accepted
---

## For future Claude
lightyear 0.31.0, messages only, over WebSocket, is the M1 networking stack (spike gate, 2026-10-26). P1 to P8 pass. This decision is final for M1. Spec 6.10 stays "not selected" beyond M1.

**Who:** spike gate (orchestrator, studio call), on the Task 6a evidence.
**Evidence:** `docs/measurements/m1-network-spike.md`. P1 to P8 pass. Every plant went red.
**P8 (Task 6b):** a wasm32 client (`tests/spike-web`, trunk 0.22.0, wasm-bindgen 0.2.129) in headless Chrome 156.0.8078.47 (`playwright-core` 1.64.1, no extra flags) gets `Welcome` from `spike_server` in 265 to 294 ms, 4 of 4 runs. Plant (the page sends `sha256:other`): `rejected`, exit 1. The `getrandom` backend flag was not necessary.
**Gate change:** `UnitDeltas` travel on `ChannelMode::UnorderedUnreliable`. A lightyear 0.31 sequenced channel keeps only the newest unread message, so deltas in a burst were lost inside the library (P5: 1 lost in 1 of 3 runs; P7: 109 and 113 lost after a 5 s stall). After the change: P5 lost 0 in 3 of 3 runs at 1 and 1,000 units; P7 lost 1 (cause NOT TRACED).
**Measured after the change:** bytes per tick to one client 126.4 to 127.0 (1 unit) and 10,388.1 to 10,397.4 (1,000 units); payload 13.8 and 9,853.5. P6 tick p99 405 us baseline, 127 us after the kill (sequenced channel run). P7 tick p99 362 us before and 407 us during the pause; Send-Q at mid-pause 552,867 bytes; RSS rise 0 bytes.
**Rules that follow:**
- Client timeout 10 s, set in the client's connect token. The server keeps the default.
- Kick with `Unlink` 300 ms after `JoinRejected`. Count a client disconnect only after `Connected`.
- Wait for `Linked` on the server entity, not `Started`, before a client connects.
- Do not put `UnitDeltas` on a sequenced channel. The client's `base_tick` chain and resync guard against real loss.
