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
M1 prototypes lightyear 0.31 over WebSocket with replication on and prediction and deterministic off; fallback is bevy_replicon 0.45 + bevy_replicon_renet2 1.0. Spec 6.10 stays "not selected" until the bytes-per-tick probe runs.

**Who:** network-engineer (Fable) researched; orchestrator accepts as a studio call inside spec 6.10's "first prototype recommendation". Not a CEO decision.
**Evidence:** crates.io fetched 2026-10-26: lightyear 0.31.0 (Bevy ^0.20, MIT/Apache, wasm websocket and webtransport features), bevy_replicon 0.45.4 (Bevy ^0.20), naia 0.25 (Bevy 0.18, WebRTC only: out). Scratchpad `m1/network.md`.
**Rules that follow:**
- The mod-facing and spectator protocol is our own envelope with content ids and stable unit ids. Library frames carry Bevy entity ids and never cross the public boundary (spec 5.2, D15).
- Never enable lightyear's `deterministic` feature (spec rejects lockstep, D18).
- Pin exact versions; lightyear 0.31 depends on an aeronet release candidate (risk accepted for a prototype).
- Tick rate assumption for M1: 20 Hz, configurable, recorded as an open input for M7.
