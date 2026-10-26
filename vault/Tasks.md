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

- [ ] **M2 implementation (16 tasks).** Owner: Codex orchestrator; Sol for reasoning/planning/reviews, Luna for execution. The user approved execution on 2026-10-26. Plan: `docs/superpowers/plans/2026-10-26-m2-economy.md`; active managed worktree: `/home/julien/.codex/worktrees/ee40/TuRTS`; SDD ledger: `/home/julien/.codex/worktrees/ee40/TuRTS/.superpowers/sdd/2026-10-26-m2-economy/progress.md`. Task 1 is complete after fix round 1 review (commits `ddd9c64..2015cec`); Task 2 is ready next. Sol re-review `/root/m2_task_1_rereview_1` approved with no new breakage. Task 1 report: `.superpowers/sdd/2026-10-26-m2-economy/task-1-report.md`. Baseline workspace tests passed (211 passed, 0 failed, 12 ignored).

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
