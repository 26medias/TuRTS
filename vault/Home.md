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

**Phase:** M1 closed on 2026-10-26. M2 design revision 2 and the 16-task plan were approved. Tasks 1–7 are complete after review and canonical integration; the combined gate passed (325 workspace tests, 0 failed, 12 ignored across 63 groups; fmt, locked check, strict Clippy, boundary and wasm passed). Task 8 is active in /home/julien/.codex/worktrees/4f65/TuRTS on codex/m2-accounting from product base 1ec683a. R9 freezes its oracle/capture scope and R11 permits only narrow Bevy Math serialization support; AuditTrail/owned records are trusted Rust diagnostics, capture helpers private. Client queue prep is integrated as 1726c4f from 422d1fa (4/4 named tests and client wasm passed) without activation. Role tool 409cc7a is 25 Python + 6 Rust tests green, Sol review pending. M1 prep and role packages remain held; protocol 1 remains selected, no B integration.

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
