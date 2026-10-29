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

**Latest closeout:** All20 implementation entries, checkpoints C1–C4 and final technical/player/mod-author reviews are accepted after the final repair. Ready for local Forge playtesting. Hosted CI, push and merge to main remain pending approval. Current code07936e8:763/0/18;75/75 approved agent runs used. [[Reports/2026-10-28 M3 ready to test]] supersedes historical status entries below.
The task board. Move items between sections; keep owner and evidence link on each line.

# Task board

## In progress

- [ ] **M3 industrial loop implementation (20/20 accepted; final milestone judgments active).** T0 freezes real M2 operation streams; T1 adds the additive API0.2.1 schema, package readers, resolved catalog and source diagnostics, with old-reader serialization compatibility. T2 portable validation and T3a state/reasons are accepted; T4 public boot is accepted; checkpoint C1 passed (all88 added tests have semantic red evidence), T6/T7 are accepted, and T8/T9/T10/T11/T12 are accepted, T13a is accepted, and C2 passed on frozen b87c111. T13b export is accepted; T14 oracle/scenario work is accepted while T15a client ingest is accepted and T15b presentation and browser gates are accepted on `codex/m3-integration`. Board: [[M3 Execution Board]]. Approved scope: [[Decisions/2026-10-27 M3 scope and orchestrator calls]]. F11 privacy resolution: [[Decisions/2026-10-27 M3 F11 private research trigger]]. Extraction, transport, inserters, machines, research and combat are accepted; Task16 and independent C4 passed; final technical, player and mod-author judgments remain before the local test handoff and integration approvals.

No M2 tasks remain in progress; all 16 were accepted on 2026-10-27.

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

- [x] **2026-10-27: M2 implementation (Tasks 1–16) accepted.** Final canonical source: `601e71286e56bd2cbed0ecf9f0240ebcecd63047`; task closeout follows all 41 native/browser semantic refutations, six Task16 source/consumer review lenses, the final guarded release acceptance, tracked T7, and all 15 default M2 browser positives. Global workspace validation: 497 passed, 0 failed, 17 ignored across 72 groups; fmt, strict Clippy, wasm and boundary gates passed. Browser mutation audit: 8/8 semantic RED/restored GREEN; native: 33/33; final no-argument M2 browser suite: 15/15. Evidence and hashes: `docs/measurements/m2.md` and `target/task16-canonical-union/final-601e712/`. GitHub Actions was not run; CI-host measurements and performance budgets remain open. The prototype is a multiplayer graybox; gameplay systems beyond the accepted economy slice remain future work.

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


- [x] **2026-10-27: Task 14 — public authoring and public proof.** Canonical commits `45d8c85`, `7f128d1`, `8bf0bda`, lint cleanup `74e2c10`; final Task 14 review accepted. Scoped gates: 42 accounting + 7 variant + 2 public-path + 4 no-content tests passed; Python variants 26/26 passed; formatting and strict Clippy passed. Mutation evidence aggregates 18 previously verified cases and seven new actual-product cases, each with semantic RED and restored GREEN; source hashes and restoration were verified. E8/E9 remain deferred to Tasks 15/16. Evidence: `.superpowers/sdd/2026-10-26-m2-economy/progress.md`, `docs/measurements/m2-public-api.md`, `/tmp/task-14-report.md`, `/tmp/task-14-public-proof-final-review.md`.


- [x] **2026-10-27: Task 10 — protocol 2 and shared command sequencing.** Accepted at product checkpoint `f75488082f87c829c3e210b3eccb6b3420a0994c`; `NET_PROTOCOL_VERSION=2` is selected. Canonical gates: workspace 401 passed/0 failed/13 ignored across 67 groups; fmt, locked workspace check, strict Clippy, boundary, workspace wasm and excluded spike-web wasm passed. Process gates: unit_boot 1/1, hello 2/2, acceptance 8/8, repeated convergence 1/1. Chrome T7: `acme:scout`, 53/53 scene-checked frames, final `[20,8]`, oracle 1/1. Excluded spike-web lock additions were needed for postcard and introduced no version upgrades. Logs: `/tmp/task10-canonical-workspace-tests.log`, `/tmp/task10-canonical-acceptance.log`, `/tmp/task10-canonical-t7.log`.


- [x] **2026-10-27: Task 11 — completed-tick owner-filtered export.** Accepted at product checkpoint `2a186f01bc185eb320145ac52bddf8b399ec6c2b`. Source commits `c1bf9a6aaf73e1b1f05338277ef730d801e84704`, `bdb0f1756cd5cd23d0c65062cf121e9f0fbd54b5`, and `aefc69746e5e8846b292a7dbdefd54f64fbec7e4` integrated as `ff10ec5`, `e69fd96`, `2a186f0`. Focused tests: 262 passed/0 failed/12 ignored across 30 groups, including export 5/5, boot 10 passed/1 ignored, accounting 42/42, variants 7/7. Fmt, locked sim/server check, strict Clippy and sim wasm passed. No combined workspace/process/browser gates were claimed; these are deferred until Task 12 integration. Evidence: `/tmp/task11-canonical-tests.log` (SHA256 `9e11fcbc7868e29517a72f7be37540c5f0e2245da5cd6531c46ac2a7346684fb`) and `.superpowers/sdd/2026-10-26-m2-economy/task-11-full-review.md`.

**M2 closeout:** All 16 tasks are accepted. Current scope and evidence are summarized in [[Home]] and the [M2 measurements](../docs/measurements/m2.md); the final dated report is [[Reports/2026-10-27]].

Human usability amendment1354e33: catalog-derived principal labels/named local selection, stationary/mobile guidance and research cost/progress. Fresh workspace765/0/18, strict checks and wasm/boundaries pass; real-pointer research completion+damage20, second-client movement and existing Forge7 pass. Historical C4/fault evidence remains qualified unchanged-engine reuse. See vault report “2026-10-28 M3 human playtest repair”; no additional agents dispatched, hosted CI/push/main pending.

**M3 scope closed by user:** technical implementation accepted for integration; human demo clarity remains inadequate. User explicitly authorized merge to main and push. Further demo polish is deferred; integration/CI receipts will be retained in the closeout evidence.
