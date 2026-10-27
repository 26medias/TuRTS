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

**Phase:** M1 is closed. M2 Tasks 1–9 and 14 are complete after review and canonical integration; original plan progress is 10/16. Task 14's 49 Rust tests, six scanner/public-loader checks, and 26 Python tests passed; fmt and strict Clippy passed. The 25 mutation cases comprise 18 previously verified cases and seven new actual-product cases with semantic RED/restored GREEN evidence. Tasks 10–13 and 15–16 remain in progress/backlog as tracked on the board. Task 10 canonical integration and global gates are underway. Protocol 1 remains selected until Task 10's atomic switch passes all gates. Current board, report and ledger are in the managed worktree at `/home/julien/.codex/worktrees/ee40/TuRTS`.** https://github.com/26medias/TuRTS (public). CI green.

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
