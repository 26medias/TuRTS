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

- [ ] **M2 implementation (16 tasks).** Owner: Codex orchestrator; Sol for reasoning/planning/reviews, Luna for execution. User authorized dependency-based parallel isolated worktrees after Tasks 1–2 passed review at base `9e383896`. Three lanes are initialized at reviewed base `9e383896`: A simulation (`/home/julien/.codex/worktrees/8b9c/TuRTS`, `codex/m2-sim`) completed Task 3 after approved rereview and canonical integration; Task 4 is complete after rereview and canonical integration (commits `eec8e8a`, `b1d06de`; test-only fixture fix `94156a6`); R6 routes Task 5 gather at `/home/julien/.codex/worktrees/8b9c/TuRTS` and Task 6 production at `/home/julien/.codex/worktrees/e814/TuRTS` from the same approved Task 4 source baseline `3e8025e`; production HEAD `0db7799` carries the same test-only fixture fix. Task 6 is Sol-approved and integrated canonically (`c66ec6d`, `1a04d0f`), with all required checks passing; it remains a completion candidate pending the reviewed Task 5 union and combined timing/atomicity witness. Task 7 checkpoint commits `44a4202` + `c89c18f` pass 11 completion + 16 production tests and overflow unit test, review pending; end-to-end income-overflow and depleted-node occupancy require the gather-union integration; Task 8 follows integrated/reviewed Tasks 5–7. E1 scanner fix `d8d7601` is accepted by Sol. Task 5 fix 1 commit `2dcb008d96e076469a2190f0ba3044490fef3331` passes 19 integration + 5 unit tests, but Sol rereview still has the two-gatherer partial-cargo race at `gather.rs:465-491` open; probe ongoing and not approved. B authored packages/Cedar (`/home/julien/.codex/worktrees/b8a3/TuRTS`, `codex/m2-data`, commits `5009f98` + `c8da658`) is approved and held until A8 and M1 migration; variant source/tool commits `bf03be14d21802033b7690dc9e1875e6b073810a`, `7e6eb1fb54ecd05444f00993f2bd4b1edf5cc3a9`, and fix `3d4a67975392aa5d7d893f6763ee9a7ec7049dca` are Approved; Rust 3 + Python 19 passed, optional P3 deferred. Roles/behavior and Task 14 remain incomplete. C protocol DTO/registration prep is integrated canonically; `NET_PROTOCOL_VERSION` remains 1 and Task 10 remains incomplete. Original plan completion remains 4/16 until the gather/production union witness passes. Canonical integration/vault stays `/home/julien/.codex/worktrees/ee40/TuRTS`; review each lane before integration. Preparation does not check off original tasks. Root plus three agents fills four slots. UI prep may use the free lane after Bboot only while independent; otherwise work switches to Task 14 preparation. Coordinator owns shared activation; Task 10 remains atomic. R1/R2/R3/R5/R6/R7/R8 and the Task 8/final-review AuditState Minor remain recorded in the execution ownership decision. Amendment: `docs/superpowers/plans/2026-10-26-m2-parallel-execution.md`.

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
