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

**Phase:** M1 is closed. M2 Tasks 1–12 and 14 are accepted after review and canonical integration; original plan progress is 13/16. Task 12 product checkpoint is `f17ff7762ce63cc3f2a7a29dda28beb9b5378b78`. Combined canonical gates passed: workspace 427/0/13 across 69 groups; fmt, locked workspace check, strict Clippy, boundary, workspace and excluded spike-web wasm; process acceptance 1 + 2 + 8 + repeated T5 1; Chrome T7 45/45 scene checks and oracle 1/1. Headless public-package startup passed for base, haul and cedar with no recovery requests. The client report omits economy/node details, so the smoke supports initial acceptance through successful presentation but does not independently report those records or prove full interactions. Task 13 UI is starting from this checkpoint. Task 15 has approved standalone JSONL preparation and parser work underway, but is not complete. Task 16 final browser/native acceptance remains. Protocol 2 is selected. README/merge/push closeout remains deferred until all 16 tasks and final gates pass. Current board, report and ledger are in `/home/julien/.codex/worktrees/ee40/TuRTS`.** https://github.com/26medias/TuRTS (public). CI green.

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
