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

**Phase:** M1 is closed. M2 Tasks 1–10 and 14 are complete after review and canonical integration; original plan progress is 11/16. Task 10 selected protocol 2 atomically and passed 401 workspace tests (0 failed, 13 ignored across 67 groups), formatting, locked workspace check, strict Clippy, boundary, both wasm checks, process acceptance and Chrome T7. T7 checked 53/53 frames for `acme:scout` at `[20,8]`, oracle 1/1. Tasks 11 and 12 are approved to proceed in parallel from product checkpoint `f75488082f87c829c3e210b3eccb6b3420a0994c`, reusing worktrees `61b7`/`target/m2-export` and `8bdc`/`target/m2-protocol`. Tasks 13 and 15–16 remain. Final README/merge/push closeout remains deferred until all 16 tasks and final gates pass. Current board, report and ledger are in the managed worktree at `/home/julien/.codex/worktrees/ee40/TuRTS`.** https://github.com/26medias/TuRTS (public). CI green.

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
