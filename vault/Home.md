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

**Latest QA0 state:** QA0 is complete. All six tasks are merged; check and agents CI passed. All 23 mutation cases passed; 26/26 agent runs used. The known M2 browser failure remains. [[Reports/2026-10-29]] supersedes earlier status notes.
Dashboard for the TuRTS studio. Read this first, then [[Tasks]] and the latest note in Reports.

# TuRTS studio

**Product:** mod-first RTS engine in Rust and Bevy. Browser WebAssembly client first, authoritative headless server, the official game is a default mod on the public API. Spec: `RTS-Engine-Project-Specifications.md` (v0.8, 2026-10-29).

**Current milestone:** ACTIVE: Waves1-3 complete (8 of25 lanes), pushed6d2c46e. Task2b64cd400 accepted0High/0Medium; root re-gate12commands0,886/0/18,T7(138frames). Wave3 CI41157502932 queued. Runs32T4/33T9UX registered, build33/170. C1-C24 recorded. One cache reserved forT4; T9UX drafts only. Mainc3f09d0 untouched. [[MC Execution Board]]

**Phase:** API0.2.1/protocol3 industrial graybox is locally validated. Exact-source gates:760 passed,0 failed,18 ignored; strict Clippy, wasm, boundaries and frozen M2 stream pass. Native M2 faults33/33, M3 formal faults17/17, browser faults8/8, positive browser15/15, real private-research/combat F11, Forge browser7 and T7 pass. Final judgments and hosted CI/push/main merge approvals remain.

**M2 baseline:** M1 closed; M2 Tasks 1–16 accepted at canonical source `601e71286e56bd2cbed0ecf9f0240ebcecd63047`. The multiplayer graybox proves the public package path, two-owner server-authoritative economy interactions, browser/native rendered controls, and recovery. Final process acceptance, T7, 15 browser positives, 33 native mutation plants, and 8 browser mutation plants passed. Global workspace tests: 497 passed, 0 failed, 17 ignored across 72 groups. Exact artifacts and hashes are in `docs/measurements/m2.md`. Remote GitHub Actions was not run for this closeout; performance budgets remain open.

## Where to look

- [[Tasks]]: the board.
- [[Team]]: the sixteen agents, model tiers, review process.
- [[M3 Execution Board]]: M3 run sheet for the Codex agents (tasks, reviewers, validators, waves, ledger).
- `Decisions/`: one note per decision. Start with [[Decisions/2026-10-26 Bevy 0.20]].
- `Work Log/`: one note per day.
- `Reports/`: the daily report for the CEO.
- `Knowledge/`: verified facts the studio relies on.

## Standing CEO items

- License: TBD (CEO's words). No LICENSE file until chosen.
- Desktop delivery (D02, native or Tauri wrapper): the M1 acceptance row names a secondary executable. The studio proposes to accept the browser part of M1 now and block the executable part on this decision.

Human-playtest follow-up: prior usability verdict superseded by user feedback. Named selection/world labels, role guidance and research feedback repaired; fresh workspace765/0/18 and human research/mobile/existing7 browser flows pass. [[Reports/2026-10-28 M3 human playtest repair]].

**M3 scope closed by user:** technical implementation accepted for integration; human demo clarity remains inadequate. User explicitly authorized merge to main and push. Further demo polish is deferred; integration/CI receipts will be retained in the closeout evidence.

**Integrated:** M3 fast-forwarded into main and pushed; hosted CI running, human-demo limitations retained. See [[Reports/2026-10-28 M3 integration]].
