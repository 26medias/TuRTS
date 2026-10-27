---
type: note
date: 2026-10-26
tags:
  - turts
  - dashboard
ai-first: true
source: mcp
---

## For future Claude
Dashboard for the TuRTS studio. Read this first, then [[Tasks]] and the latest note in Reports.

# TuRTS studio

**Product:** mod-first RTS engine in Rust and Bevy. Browser WebAssembly client first, authoritative headless server, the official game is a default mod on the public API. Spec: `RTS-Engine-Project-Specifications.md` (v0.7, 2026-10-26).

**Current milestone:** M1 done. Next: M2, mod API proven by the default mod plus an alternative economy mod.

**Phase:** M1 is closed. M2 Tasks 1–9 are complete after review and canonical integration; original plan progress is 9/16. Task 9 final canonical gates passed: 390 workspace tests passed, 0 failed, 12 ignored across 68 groups; fmt/check/strict Clippy/boundary/locked wasm passed. Process acceptance and Chrome T7 passed (52/52 frames, `acme:scout`, final `[20,8]`). Task 10 is next; protocol 1 remains selected pending the atomic protocol switch. Current board, report and ledger are in the managed worktree at `/home/julien/.codex/worktrees/ee40/TuRTS`.** https://github.com/26medias/TuRTS (public). CI green.

## Where to look

- [[Tasks]]: the board.
- [[Team]]: the sixteen agents, model tiers, review process.
- `Decisions/`: one note per decision. Start with [[Decisions/2026-10-26 Bevy 0.20]].
- `Work Log/`: one note per day.
- `Reports/`: the daily report for the CEO.
- `Knowledge/`: verified facts the studio relies on.

## Standing CEO items

- License: TBD (CEO's words). No LICENSE file until chosen.
- Desktop delivery (D02, native or Tauri wrapper): the M1 acceptance row names a secondary executable. The studio proposes to accept the browser part of M1 now and block the executable part on this decision.
