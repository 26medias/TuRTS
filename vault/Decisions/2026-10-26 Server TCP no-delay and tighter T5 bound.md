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
Disable Nagle's algorithm on the server's WebSocket sockets by vendoring aeronet_websocket 0.22.0-rc.1 with a four-line `set_nodelay(true)` after accept; disable it on the native client too; tighten T5's lag p99 limit from 2.0 to 1.5 ticks.

**Who:** orchestrator (studio call) on a traced investigation by network-engineer (Opus).
**Evidence (localhost, 20 Hz):** strace showed no TCP_NODELAY on the accepted server socket; server send to client receive p90 15.8 ms, p99 39.2 ms (one client frame plus Linux delayed ACK). aeronet exposes a Nagle option only on the client. With the patch: lag p99 1.33 to 1.35 over 12 runs (was 1.62 to 1.96 over 16); delivery max 19.4 ms (was 48.9).
**Why tighten the test:** with the old 2.0 limit no run ever went red on the Nagle build, so T5 could not catch this regression. 1.5 goes red on the old build and passes on the fixed one.
**Follow-up for the CEO:** file an upstream aeronet issue asking for a server-side no-delay option, then drop the vendored copy. Filing publishes on the CEO's behalf, so it waits for approval.
