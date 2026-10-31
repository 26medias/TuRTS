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

**Latest QA0 state:** QA0 is complete. Six tasks merged; check and agents CI passed; all 23 mutation cases passed. Closed at 26/26 agent runs. [[Reports/2026-10-29]].
The task board. Move items between sections; keep owner and evidence link on each line.

# Task board

## In progress

- [ ] **Full spec review (2026-10-28): 16/16 fail. CEO decided G1-G16 on 2026-10-29 (G1 with clause, rest as team lean).** Spec v0.8 amended and gated (2026-10-29). Stop and hold in 16.2 Decided (CEO, 2026-10-29). QA0 design and plan passed both gates (2026-10-29); K1-K6 decided, must tier. QA0 built by Codex and merged into `main` on 2026-10-29 (code `3e1b0ab`, 26/26 runs; [[QA0 Execution Board]]). Orchestrator doc pass done. Open: set the M4 registry row to `gated` and change `cases.tsv` row 4 when the amended M4 spec passes gate 1. MC design passed gate 1 (2026-10-29, revision 4, `cbe37f5`, five lenses, three repairs); CEO chose the full design (about 184 runs) and K1-K27 as recommended; K20 applied (revision 5, 19 rows to `in`, MC registry row `gated`). MC plan passed gate 2 (2026-10-29, six lenses, 21 high in round 1, three re-checks; plan `docs/superpowers/plans/2026-10-29-mc-player-control.md`; design revision 10). CEO approved about 212 total runs, stop at 170 build runs; CEO plays PT1/PT2, not blind; base a0a3076. O9 accepted the P1 table resolution; Codex rebased both branches on main f503cff. CEO deferred Firefox and released continuation. K11(a) retained on Chrome evidence; P1 completed supplementary Chrome inputs; T1/T2a merged and fully re-gated at17286b1 (799/0/18); both lanes removed. T3 merged at aceef85 and all integration gates passed, including headed Vulkan T7 (115 frames). T20 run17 PASS0High/0Medium; Wave1 merged/re-gated/pushed8e95e42, draftPR2 and CI41097571682 running (agents passed). O10 approved atmainb29e3d8/designrev11: ShellAction and client_shell.rs; integration and open lanes rebased. T8 complete at merge461d105, review0High/0Medium and all integration gates green; lane removed, T18 unmerged0e83be5 and T19 test/drafts preserved. CEO released stop at c3f09d0/O11, design12 uses Shell Press. ACTIVE under C1/C2 fixture/policy repair and C3 overrun continuation; Task18 complete atb1b3834, independent0High/0Medium and all integration gates green including C12T7(138frames); lane removed. T19 run28 owns single cache; build28/170. C1-C12 recorded. [[Decisions/2026-10-30 MC Codex rulings]]. [[Reports/2026-10-30 MC belt toggle contract stop]]. Main unchanged. Disk cleanup reclaimed24.24GiB plus about56GiB of avoidable debug output; compact profile settings enforced and targets cleaned after each merge. [[Reports/2026-10-30]]. [[MC Execution Board]]. Codex brief: [[Reports/2026-10-29 MC Codex brief]]. [[Decisions/2026-10-29 MC calls K1-K27 and budget]] Integration branch and [[QA0 Execution Board]] created from 074fe7e; all six tasks complete, 26/26 agent runs; check and agents CI passed; known M2 browser failure recorded; CEO approved all 23 mutation cases. `ux-designer` seat (G11) created 2026-10-29; D67 design constraint recorded (free placement, keep line a). M4 draft on hold until after MP2. [[Reports/2026-10-28 Full spec review]] · [[Decisions/2026-10-28 Goal statement and full spec review]]
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

- [x] **QA0 hosted CI closure.** All six tasks are complete. Final code 3e1b0ab; all 23 mutation cases passed; check and agents CI green. Browser has the unchanged known M2 timeout. Closed at 26/26 runs; no run 27 is authorised. [[QA0 Execution Board]] · [[Reports/2026-10-29]]

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

**Integrated:** M3 fast-forwarded into main and pushed; hosted CI running, human-demo limitations retained. See [[Reports/2026-10-28 M3 integration]].

MC update2026-10-31: ACTIVE: Waves1-3 complete (8 of25 lanes), pushed6d2c46e. Wave3 CI41157502932 check/agents passed; browser retains known M2 timeout. Task4 candidate6e25954 in gates after focused E9 spawn repair; no merge yet. Task9 source/test preparation complete; uncompiled, no feature accepted. Build35/170; C1-C52 recorded. One compiler/cache owner. Mainc3f09d0 untouched. [[Reports/2026-10-31]].
