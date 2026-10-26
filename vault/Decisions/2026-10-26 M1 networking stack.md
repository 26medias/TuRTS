---
type: decision
date: 2026-10-26
tags:
  - turts
  - decision
  - networking
ai-first: true
source: mcp
status: proposed
---

## For future Claude
lightyear 0.31.0, messages only, over WebSocket, is the proposed M1 networking stack. It passed P1 to P7 in the final spike run. It is selected for M1 only after P8 (Task 6b) and the spike gate. Spec 6.10 stays "not selected" beyond M1.

**Who:** Task 6a implementer (network spike). The spike gate (orchestrator) decides.
**Evidence:** `docs/measurements/m1-network-spike.md`. The final run passed 7 of 7. Every plant went red.
**Open point for the spike gate:** P5 lost 1 delta in 1 of 3 runs. The cause is traced: a lightyear 0.31 `SequencedUnreliable` channel keeps only the newest unread message. Thus, deltas that arrive in a burst are dropped on the client (P7 lost 109 and 113 after a 5 s stall). The design 5 resync path recovers each loss. The gate decides if this fails P5, which would require the fallback (bevy_replicon 0.45 + bevy_replicon_renet2 1.0) to run the same list.
**Measured:** bytes per tick to one client: 128.4 for 1 unit and 10,387.7 for 1,000 units (payload 13.8 and 9,853.5). Tick p99: P6 405 us baseline and 127 us after the kill; P7 427 us before the pause and 396 us during the pause. P7 Send-Q at mid-pause 562,971 bytes. P7 RSS rise 0 bytes.
**Rules that follow:**
- Client timeout 10 s, set in the client's connect token. The server keeps the default.
- Kick with `Unlink` 300 ms after `JoinRejected`. Count a client disconnect only after `Connected`.
- Wait for `Linked` on the server entity, not `Started`, before a client connects.
- Expect `UnitDeltas` loss without network loss. Each loss is a chain break and starts one resync.
