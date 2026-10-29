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

**Latest closeout:** All20 implementation entries, checkpoints C1–C4 and final technical/player/mod-author reviews are accepted after the final repair. Ready for local Forge playtesting. Hosted CI, push and merge to main remain pending approval. Current code07936e8:763/0/18;75/75 approved agent runs used. [[Reports/2026-10-28 M3 ready to test]] supersedes historical status entries below.
Dashboard for the TuRTS studio. Read this first, then [[Tasks]] and the latest note in Reports.

# TuRTS studio

**Product:** mod-first RTS engine in Rust and Bevy. Browser WebAssembly client first, authoritative headless server, the official game is a default mod on the public API. Spec: `RTS-Engine-Project-Specifications.md` (v0.7, 2026-10-26).

**Current milestone:** QA0 (instrument and schema groundwork). Design and plan passed both gates on 2026-10-29; the CEO chose the Codex handoff for the build (6 tasks, 4 waves, 22 to 26 runs, must tier). Brief: [[Reports/2026-10-29 QA0 Codex brief]]. Run sheet: [[QA0 Execution Board]] (created; no agent runs yet; the T6 mutation count awaits the CEO). M3 is done and merged ([[Reports/2026-10-28 M3 integration]]).

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
