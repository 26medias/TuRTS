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

**Phase:** M1 is closed. M2 Tasks 1–14 are accepted after review and canonical integration; original plan progress is 14/16. Task 13 product checkpoint is `435f74011f0aa577566f132791f596fa1a96dd19`. Combined canonical gates passed: workspace 445/0/13 across 70 groups; fmt, locked workspace check, strict Clippy, boundary, workspace and excluded spike-web wasm, and Trunk release build. Process acceptance passed unit_boot 1 + hello 2 + acceptance 8 + repeated T5 1. Chrome T7 passed with 52/52 scene checks and oracle 1/1 (`acme:scout`, final `[20,8]`). Task 15 native process validation is still underway. Task 16 must verify actual native-window controls and browser UI text/clicks; T7 is not that acceptance, and a preview screenshot is not proof of its selection interaction. Protocol 2 is selected. README/merge/push closeout remains deferred until both remaining tasks and final gates pass. Current board, report and ledger are in `/home/julien/.codex/worktrees/ee40/TuRTS`.** https://github.com/26medias/TuRTS (public). CI green.

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
