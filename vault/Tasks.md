---
type: task
date: 2026-10-26
tags:
  - turts
  - board
ai-first: true
source: mcp
---

## For future Claude
The task board. Move items between sections; keep owner and evidence link on each line.

# Task board

## In progress

- [ ] **M2 implementation (16 tasks).** Owner: Codex orchestrator; Sol reasoning/review, Luna execution. Tasks 1–9 complete after review and canonical integration; original progress 9/16. Task 9 canonical gate: 390 workspace tests passed, 0 failed, 12 ignored across 68 groups; fmt/check/strict Clippy/boundary/locked wasm passed; acceptance and T7 browser passed. Canonical product checkpoint `1f004e6ad363a5f94f47cb52e4bf98ce5c95aa08` in `/home/julien/.codex/worktrees/ee40/TuRTS`. Task 10 protocol switch and Task 14 public proof have prepared isolated worktrees; both await root dispatch. Protocol 1 remains selected.

## Backlog (M1)

- [ ] M1 plan (writing-plans), gate 2.
- [ ] Cargo workspace and first crates (per the plan).
- [ ] Networking prototype: lightyear over WebSocket; fallback bevy_replicon + renet2.
- [ ] Simulation core: fixed tick, one unit, move command.
- [ ] Mod package v0: manifest, one unit definition, lockfile with one entry, hash check.
- [ ] Headless host binary with advertisement and lockfile endpoint.
- [ ] Browser client: join, present server results, interpolation.
- [ ] Acceptance tests T1 to T6 (producer's list), each shown red first.

## Done

- [x] 2026-10-26: User approved M2 economy design revision 2 and its economy-only/no-script scope. Decision: [[Decisions/2026-10-26 M2 scope and no script layer]].

- [x] 2026-10-26: Codex handoff; sixteen roles and shared skills verified. M2 design and complete authoring example written; six-seat gate 1 passed after consumer/simulation repairs and re-reviews. Evidence: `docs/research/m2/design-reviews.md`.

- [x] 2026-10-26: M2 research by producer, game-designer, mod-runtime-engineer.

- [x] 2026-10-26: **M1 closed** (3dbbe16). See [[Decisions/2026-10-26 M1 closed]].

- [x] 2026-10-26: M1 verify run. Third-party package works end to end; 5 of 18 QA plants stayed green (now being fixed).

- [x] 2026-10-26: all 18 M1 tasks done (bcbc5cd). T7 browser join passes in CI. Server TCP no-delay fix (021c522): T5 lag p99 about 1.34 ticks, limit tightened to 1.5.

- [x] 2026-10-26: M1 Tasks 7c, 9, 10 done; all eight acceptance tests pass (35e44b3).

- [x] 2026-10-26: M1 Tasks 6b, 7a, 7b, 8a-1, 8a-2 done; duplicate-player transport bug fixed (09d72bc). CI green.

- [x] 2026-10-26: networking spike passed; spike gate kept lightyear (df47cb0, ba8af70).

- [x] 2026-10-26: M1 Tasks 1, 2, 3, 4, 5, 8b done and verified (commits cc48dee to 6db7445).

- [x] 2026-10-26: M1 plan approved after gate 2 and re-gate (revision 2b, commit b890c3e). 18 tasks.

- [x] 2026-10-26: M1 plan drafted (Fable planner) and gated (five lenses; qa-lead fail). Design gate 2 edits, commit 7b5b2e1.

- [x] 2026-10-26: M1 gate 1 passed after two repair passes; design revision 3 (commit fb6e1e4). Reviewers: spec-rigour-reviewer, technical-director (Fable); network-engineer, mod-runtime-engineer, mod-builder (Opus); web-platform-engineer research.

- [x] 2026-10-26: M1 design drafted (`docs/superpowers/specs/2026-10-26-m1-foundation-design.md`), orchestrator.
- [x] 2026-10-26: spec 0.7 merged from the CEO's upload (commit b317d9e).

- [x] 2026-10-25: CLAUDE.md for the repository (orchestrator).
- [x] 2026-10-25: studio agent team designed, gated, built, dry-run tested (orchestrator + 3 implementer agents + 4 reviewers). Commits 38a947c to 10feb7f.
- [x] 2026-10-26: twenty third-party skills vetted by seven seats and installed (`docs/agents/skills.md`).
- [x] 2026-10-26: spec 0.6, two defects fixed, gated and re-gated (commit 4188d82).
- [x] 2026-10-26: toolchain, CI, scripts, public GitHub repository, CI green.
- [x] 2026-10-26: M1 research by network-engineer, producer, technical-director, mod-runtime-engineer (decisions logged; reports in scratchpad `m1/`).

- [x] Task 8 — Accounting oracle / audit — complete after oracle rereview-2 and raw-capture rereview. Canonical gates passed: workspace 376/0/12 across 65 groups, named economy_accounting 42/42, fmt, locked check, strict Clippy, boundary and wasm.
- [x] Task 9 — Base/haul package integration and positive economy loops — Sol-approved package integration; public-loader base/haul loops and acme no-economy check passed under the Task 8 oracle. Final canonical tests 390/0/12 across 68 groups; acceptance passed and T7 Chrome passed 52/52 frames. Evidence: `/tmp/turts-m2-task9-final-workspace-tests.log`, `/tmp/turts-m2-task9-final-acceptance.log`, `/tmp/turts-m2-task9-final-t7.log`; review artifacts under canonical private SDD.

[2026-10-27 checkpoint] Original M2 progress remains 7/16. Task 8 full review requested changes after 112 + 2 tests and the ten-plant checkpoint; Oracle work in `/home/julien/.codex/worktrees/4f65/TuRTS` is fixing automatic gather/output/owner/honest-path false rejects. Sol approved capture fix `49d0c2b` from `c68d` (raw queue endpoints/token IDs and resource flags), and coordinator authorized integration into `4f65`/model projection. Preview `e58a` passed package boot 5, public path 2, CLI 5, bad packages 7, native acceptance process 1 + 2 + 8 + convergence 1, wasm/Trunk build, and Chrome T7 (Chrome 156.0.8078.47; `acme:scout`; 53/53 frames; oracle 1/1; final `[20,8]`; artifacts `e58a/target/t7`). Preview is preflight, not Task 9 completion; all canonical B/data integration waits for Task 8. Client formatter-only fix for `crates/turts_client/tests/commands.rs` was integrated as `562bf60`; the preview's variant example remains excluded.

- [ ] Final closeout (conditional user authorization): only after all 16 tasks complete and final required checks pass, update the existing repository README (resolve actual casing at completion) with accurate final M2 scope and evidence, merge into `main`, and push. Authorization is already explicit; no redundant permission request is needed. Do not do these actions early. Current progress is 7/16; Task 8 oracle fixes and accepted capture integration remain ongoing. Preview native/T7 is not Task 9 completion. Preserve original-checkout uncommitted work.


[2026-10-27 Task 9 closeout] Original M2 progress is 9/16. Canonical product checkpoint `1f004e6ad363a5f94f47cb52e4bf98ce5c95aa08`; Task 9 canonical data/loop commit `51ff356`, scanner and fixture corrections `a712a12`, `49c445f`, `019d80e`, strict-Clippy cleanup `1f004e6`. Final gate results: workspace 390 passed/0 failed/12 ignored across 68 groups; fmt, locked check, strict Clippy, boundary and wasm passed; process acceptance 1 + 2 + 8 + repeated convergence 1; T7 Chrome 156, acme:scout, 52/52 frames, final [20,8], oracle 1/1. Task 10 is next; Task 14 prep is isolated and held. Protocol 1 remains selected.
