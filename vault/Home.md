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

**Phase:** M1 closed on 2026-10-26. M2 economy design revision 2 passed gate 1 and was approved by the user. The 16-task plan passed gate 2 after six focused reviews and repairs; the user approved execution on 2026-10-26. Implementation is underway in the managed worktree; Tasks 1 and 2 are complete after review. Task 3 is ready next. The Minor AuditState coverage from Task 2 remains deferred to Task 8/final review. Luna handles implementation; Sol handles review. See [[Tasks]], [[Team]], and the detailed [plan review record](../docs/research/m2/plan-reviews.md). Latest report: [[Reports/2026-10-26]].

**Repository:** https://github.com/26medias/TuRTS (public). CI green.

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
