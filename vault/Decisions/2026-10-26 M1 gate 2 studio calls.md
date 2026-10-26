---
type: decision
date: 2026-10-26
tags:
  - turts
  - decision
  - m1
ai-first: true
source: mcp
status: active
---

## For future Claude
Studio calls made by the orchestrator when gate 2 returned eleven High findings on the M1 plan. None needed the CEO.

- **Join starts in resync state.** Deltas that arrive before the first Snapshot are buffered. Reason: the reliable Snapshot and unreliable deltas have no common order, so without this every join could trigger a wasted resync. Found by network-engineer with a model of T8.
- **Client timeout 10 s at M1.** A client stalled 5 s stays connected; a briefly hidden browser tab survives. A longer hide disconnects; reconnect is M10. Found by network-engineer: lightyear's netcode token timeout was 3 s.
- **T5 lag bound judged in a serial run.** Process tests are `#[ignore]` under `cargo test`; `scripts/acceptance.sh` runs them one at a time on a release build, also as a CI step. Found by qa-lead: parallel tests compete for CPU and turned a correct client red in its model.
- **P7 has no fixed memory bound.** 1,000 units through a relay with a small receive buffer; server tick p99 and post-stall memory are the checks.
- **Spike gate.** After the networking spike, the orchestrator reviews its report before any task builds on the networking choice. A failed pass item stops the plan for re-planning. The spike implementer does not choose the fallback alone.
- **Plan additions moved into the design:** crate `turts_net_lightyear` (lightyear assigns message ids by registration order, so both ends share one registration); `server.ron` gets separate HTTP and WebSocket listeners plus `public_url`.
