---
type: decision
date: 2026-10-26
tags:
  - turts
  - decision
  - networking
ai-first: true
source: mcp
status: active
---

## For future Claude
Spike gate passed: lightyear 0.31, messages only over WebSocket, is kept for M1. The UnitDeltas channel moved from sequenced-unreliable to unordered-unreliable. Spec 6.10 stays "not selected" for the long term; this is the M1 choice.

**Who:** orchestrator at the spike gate, on the spike report by an Opus implementer (`docs/measurements/m1-network-spike.md`).
**Evidence (idle machine, release build):** 196 dependency licences, all permissive. Server tick p99 about 0.36 to 0.43 ms; unchanged after a client is killed and during a 5 s client stall. Stalled client: kernel send queue peak about 1 MB, server memory rise 0. Bytes per tick to one client: 127 at 1 unit (14 payload), 10,390 at 1,000 units (9,854 payload). P5 after the channel change: 0 deltas lost in 3 of 3 runs at 1 and 1,000 units.
**Finding that drove the change:** lightyear's sequenced-unreliable receiver keeps only the newest unread message, so bursts dropped deltas although TCP lost nothing (1 of 3 runs before the change). The client's base-tick chain and resync remain the guard against real loss.
**Open:** P7 still lost 1 delta in one run after the change; cause not traced (possibly a fragment set cut by the pause). Recorded, not judged.
