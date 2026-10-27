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

**Phase:** M1 is closed. M2 Tasks 1–11 and 14 are complete after review and canonical integration; original plan progress is 12/16. Task 11 product checkpoint is `2a186f01bc185eb320145ac52bddf8b399ec6c2b`; focused simulation/server/pack tests passed 262/0/12 across 30 groups, including exporter 5/5, unit boot 10 passed/1 ignored, accounting 42/42, and variants 7/7. Formatting, locked sim/server check, strict Clippy, and sim wasm passed. Combined full workspace/process/browser gates are deferred until Tasks 11 and 12 are both integrated. Task 12 core/resync work is underway in `/home/julien/.codex/worktrees/8bdc/TuRTS`; Task 15 has Sol-reviewed JSONL preparation in `/home/julien/.codex/worktrees/b54a/TuRTS`, with live implementation still incomplete. Task 13 UI and Task 16 final browser acceptance remain. Protocol 2 is selected. README/merge/push closeout remains deferred until all 16 tasks and final gates pass. Current board, report and ledger are in `/home/julien/.codex/worktrees/ee40/TuRTS`.** https://github.com/26medias/TuRTS (public). CI green.

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
