---
type: task
date: 2026-10-29
tags: [turts, board, mc]
ai-first: true
source: codex
---

# MC execution board

State: ACTIVE: O12 released Task 6. Integration and all four open lanes rebased onto main721e56e (design revision13); dirty source hashes preserved.10of25lanes complete;50/170runs; Task6run8/6, soft12/hard24. C1-C116. RUN49 repairs the exact RUN47 final source findings, then commits and gates; no repaired acceptance yet. No main merge.

Plan: `docs/superpowers/plans/2026-10-29-mc-player-control.md`.
Design of record: `docs/superpowers/specs/2026-10-29-mc-player-control-design.md`, revision12 at63788d7, included by mainc3f09d0.
Brief: [[Reports/2026-10-29 MC Codex brief]].
Original base: `a0a3076`; rebased on CEO instruction to `main` at `f503cff`; integration: `codex/mc-integration`, worktree `/home/julien/Projects/TuRTS-mc-integration`.
Task 1 lane: `codex/mc-t1`, worktree `/home/julien/Projects/TuRTS-mc-t1`, `CARGO_TARGET_DIR=target/mc-t1`.
Main stays at `c3f09d0`. The pre-existing untracked disk audit stays in the main worktree.

## 1. Roles and dispatch rules

| Role | Seat | Model and effort | Work |
| --- | --- | --- | --- |
| Orchestrator | root | Current Codex session; no child dispatch | Verify, merge, gate, track budget, keep vault. |
| Builder | Task seat in budget table | gpt-6-luna; high | Show red, then green; commit; gate; plants. |
| Reviewer | Task reviewer in budget table | gpt-6.1-sol; high | Separate agent; read diff, run tests and one plant; cite path:line. |
| Validator | qa-lead | Model from task; high | Validate integration for tasks 21 to 24 and plants. |
| Player | player and CEO | Sol; high for agent | CEO plays PT1 and PT2; log "not blind". |

Each dispatch must state model, effort, task, base, branch, worktree, target directory, owned paths, design sections, commands, report path and stop conditions. Count each dispatch and resumed work turn before it runs. Agent summaries are leads, not proof.

User workspace instructions take precedence over the brief's inherited role-file instructions. Do not read or import CLAUDE.md, .claude, Claude settings or marketplaces. No such file was read. Use Codex instructions and the task/design for the seat contract. The plan's required superpowers skills are not in the available Codex skill catalog. No external skill was loaded or enabled.

## 2. Standing rules

- Design revision12 at63788d7, included by main c3f09d0, wins over the plan. O10 requires ShellAction/client_shell.rs; O11 uses the Shell Press for PT1 step6. Apply the brief section4 decision rights and record each Codex ruling C<n> on this board and in the rulings note.
- K1 to K27 stand as recommended. K11(a) is retained on Chrome evidence under the CEO Firefox deferral. Full design; no scope cut without CEO.
- Each task branch starts from integration after all Needs merge; one worktree and target per lane. Build binaries one at a time. Remove the lane and its target only after merge.
- Red must compile and fail semantically. Show the command and `; echo exit=$?`. Do not pipe away output before the exit code.
- Order: red, green, commit, gate, plants. Restore plants, then prove changed paths are clean. Never restore uncommitted or new files with git checkout.
- Stage explicit paths. Vault-only commits use [skip ci]. End each commit with `Co-Authored-By: Codex <noreply@openai.com>`.
- Verify path:line. Mark absent evidence NOT TRACED. Use ASD-STE100 prose and four spaces per indent.
- Do not edit the design, product spec or Claude configuration. Leave the plan's Doc updates section to its owner.
- Decide listed name collisions, instrument/fixture details, uncovered steps, same-shape mechanism substitutions, wave/cache layout, Medium-or-lower findings and overruns above twice the estimate. Rename the engine side for a collision; never rename protected Forge/Muster content. Repair and recheck every High.
- Stop only for required frozen-value change; suspect Decided spec item or K call; a player-visible design change (new key, removed step or changed acceptance row); global170 or task4x estimate; Task23 URL/human play; plant retirement; or main merge. See brief section4:47-55. Do not edit design/spec or choose protected content changes.
- Before task 23, send the CEO the serve-only URL and wait. Tester identity is decided: CEO, not blind.

## 3. Gates and CI

Task gate sets are the plan's Local gate commands. T1 requires G-QA. After each task merge, run G-RUST, G-QA and G-FROZEN; also G-WEB for client/server edits. After each wave, push integration only and read hosted check, agents and browser jobs. Never merge or push main.

Frozen M2 SHA256: `bcbf72c2206251f635a7c7f5b6c8f573d69a5fc05a5bd77716e98ebfb08e370b`. Keep forge fixed point and refutation plants. Known browser failure: M2 browser proof; trace in T20, repair in owner task/T21. Wave1 hosted source8e95e42: check/agents success; browser failed at the unchanged M2 economy timeout after T7 passed. Native M2 refutations, all17M3 plants and acceptance succeeded. Receipt target/mc-evidence/wave1-ci-terminal-jobs.json. Wave2 check/agents passed; known browser timeout retained. Wave3 pushed6d2c46e with check/agents passed and the same browser failure.

CI trigger: `.github/workflows/ci.yml:2-5` runs pushes to main and pull requests. After Wave1 gates pass, push integration and open a draft PR to main to run CI. Do not merge it. Keep the pushed code head free of a skip-ci marker; record CI results in a later vault-only commit.

## Known red window

| Evidence | Current result | Cause and owner |
| --- | --- | --- |
| Historical CI 40960227409 | E9-browser-base-economy times out after 15 s. Server completed production at tick175; captured client stayed at tick87. | Underlying cause NOT TRACED. Historical artifact and log retained under target/mc-evidence/t20-builder. |
| T20 clean local baseline at22a4600 | Selection, economy and recovery pass; E9-browser-base-public-convergence fails its shared-tick public-projection assertion (tests/browser/m2.js:430-444), exit=1. | Forced-software-profile diagnostic; mechanism NOT TRACED. Isolated control and active-lifecycle cases both pass with both pages visible. A full instrumented NVIDIA run passes all15 cases; clean original-source normal-native baseline also passed all15 cases, exit0. No engine owner assigned; hosted cause remains NOT TRACED. |

## 4. Tasks

24 numbered tasks; task2 has two branches, for25lanes. T1/T2a/T3/T20, T8/T18/T19, T2b,T4 andT9 are complete. Wave5 is active: T6 repair owns the compiler; T7/T11/T12 preparations are preserved until their implementation lease.

| Task | Wave | Needs | Branch | State |
| --- | --- | --- | --- | --- |
| T1 | 1 | - | codex/mc-t1 | COMPLETE; merge8f612a3; G-RUST/G-QA/G-FROZEN all0; lane removed |
| T2a | 1 | - | codex/mc-t2a | COMPLETE; merge17286b1; all gates0; lane removed |
| T3 | 1 | - | codex/mc-t3 | COMPLETE; merge aceef85; all integration gates pass; T7 115 frames |
| T20 | 1 | - | codex/mc-t20 | COMPLETE; merge8e95e42; all integration gates0; task lane removed |
| T8 | 2 | 2a, 3 | codex/mc-t8 | COMPLETE merge461d105; review0High/0Medium; integration G-RUST/G-QA/G-FROZEN0; lane removed |
| T18 | 2 | 3 | codex/mc-t18 | COMPLETE candidate179ada7; mergeb1b3834; review0High/0Medium; all integration gates0 including C12 T7; lane removed |
| T19 | 2 | 3 | codex/mc-t19 | COMPLETE merge98135d3; review0High/0Medium; all integration gates0; lane removed |
| T2b | 3 | 2a, 3, 18 | codex/mc-t2b | COMPLETE64cd400; merge6d2c46e; all gates/10GREF0; review0H/0M; lane removed |
| T4 | 4 | 2b, 3 | codex/mc-t4 | COMPLETE merge0490ebd; review0H/0M,36GREF,all12rootgates0; lane removed |
| T9 | 4 | 1, 8, 2b | codex/mc-t9 | COMPLETE mergefb5e2d6; independent0High/0unresolvedMedium,69tests,7GREF,FOV/modifier plants,all12integrationgates0; archived lane removed, branch retained. |
| T6 | 5 | 4 | codex/mc-t6 | ACTIVE RUN49: repair finite C115/C116 replay/index findings, commit and final proof. Seven-file coherent WIP retained; sole compiler lease; no acceptance. |
| T7 | 5 | 2b, 4 | codex/mc-t7 | PREPARED RUN40; source/tests preserved, implementation and semantic red pending; no compiler lease. |
| T11 | 5 | 9, 19, 20 | codex/mc-t11 | PREPARED RUN41; source/tests preserved, native/browser red pending; no compiler lease. |
| T12 | 5 | 3, 9, 19, 20 | codex/mc-t12 | Analysis28legacytests +HUD1instrument8650131 ready; actualnative/browserred/widgetruntime/HUD4/allfinalproofpending. No legacy deletion. |
| T5 | 6 | 4, 6, 19 | codex/mc-t5 | NOT STARTED |
| T13 | 6 | 4, 6, 9, 11, 19, 20 | codex/mc-t13 | NOT STARTED |
| T10 | 7 | 5, 9, 11, 19, 20 | codex/mc-t10 | NOT STARTED |
| T15 | 7 | 4, 7, 12, 13 | codex/mc-t15 | NOT STARTED |
| T17 | 7 | 3, 12, 13, 19, 20 | codex/mc-t17 | NOT STARTED |
| T14 | 8 | 5, 12, 13, 15 | codex/mc-t14 | NOT STARTED |
| T16 | 8 | 7, 12, 15 | codex/mc-t16 | NOT STARTED |
| T22 | 8 | 6, 7, 10, 11, 13, 19, 20 | codex/mc-t22 | NOT STARTED |
| T21 | 9 | 14, 16, 17, 18, 20 | codex/mc-t21 | NOT STARTED |
| T23 | 10 | 14, 15, 21, 22 | codex/mc-t23 | NOT STARTED |
| T24 | 11 | 23 | codex/mc-t24 | NOT STARTED |

## 5. Waves

Tasks in one wave share no file, except the refutation registries (merge in task-number order, G-REF).

| Wave | Tasks | Order reason |
| --- | --- | --- |
| 1 | 1, 2a, 3, 20 | no needs |
| 2 | 8, 18, 19 | 8 needs 2a, 3; 18 and 19 need 3 |
| 3 | 2b | needs 2a, 3, 18 (R3) |
| 4 | 4, 9 | 4 needs 2b, 3; 9 needs 1, 8, 2b |
| 5 | 6, 7, 11, 12 | 6 needs 4; 7 needs 2b, 4 (`--order-log` reads the order state); 11, 12 need 9, 19, 20 (12 also 3) |
| 6 | 5, 13 | 5 needs 4, 6, 19; 13 needs 4, 6, 9, 11, 19, 20 |
| 7 | 10, 15, 17 | 10 needs 5, 9, 11, 19, 20; 15 needs 4, 7, 12, 13; 17 needs 3, 12, 13, 19, 20 |
| 8 | 14, 16, 22 | 14 needs 5, 12, 13, 15 (`AlertPings` for the step 18 ping, re-check 1 player A); 16 needs 7, 12, 15; 22 needs 6, 7, 10, 11, 13, 19, 20 (`scripts/acceptance.sh` after 10) |
| 9 | 21 | needs 14, 16, 17, 18, 20 |
| 10 | 23 | needs 14, 15, 21, 22 (and so every task) |
| 11 | 24 | needs 23 |

## 6. Hot files

| File | Order of tasks | Note |
| --- | --- | --- |
| `crates/turts_net_protocol/src/messages.rs` | 2a, 2b | |
| `crates/turts_mod_api/src/schema.rs` | 3 | |
| `crates/turts_mod_api/src/input.rs` (new) | 3, 9 | 9 adds the P1 rows to `RESERVED_BROWSER_KEYS` |
| `crates/turts_client/src/economy_ui.rs` (removed) | 2b, 9, 12 | |
| `crates/turts_client/src/render.rs` | 9, 11, 13, 14 | 9 removes the fit and the fixed start; 11 the fixed light |
| `crates/turts_client/src/lib.rs` | 2b, 9, 12, 16 | R5 |
| `crates/turts_client/src/debug.rs` | 2b, 9 | no later edit (R5) |
| `crates/turts_client/src/debug_feed.rs` (new) | 9 | typed resources; each later module writes its own resource |
| `crates/turts_client/src/input.rs` | 2b, 9 | |
| `crates/turts_interaction/src/*.rs` | 8, 9 | except `camera.rs` (8 stub, 11 logic) and `minimap.rs` (8 stub, 14 logic) |
| `crates/turts_server/src/session.rs` | 2b, 4, 7 | 4 inverts the `NotMobile` group tests only if they fail |
| `crates/turts_server/src/orders.rs` (new) | 2b, 4, 5 | |
| `crates/turts_server/src/outbound.rs` | 2b, 4 | 4 adds the sim reasons to `wire_reason` (`outbound.rs:55-90`) |
| `crates/turts_server/src/export.rs` | 2b, 4 | owner-only `UnitOrderRecord`; `export.rs:431` maps reasons |
| `crates/turts_server/src/main.rs`, `config.rs` | 2b, 7 | `mod orders;`, then `--command-script` and `--order-log` |
| `crates/turts_sim/src/economy/views.rs`, `crates/turts_server/src/economylog.rs` | 4, 6 | `AuditState`, validator (K19 a) |
| `crates/turts_sim/src/economy/transaction.rs` | 4 | phase mark on `OperationAudit` |
| `crates/turts_sim/src/command.rs` | 4, 6 | per-unit validation, clamp (K6) |
| `crates/turts_sim/src/orders.rs` (new), `crates/turts_sim/src/movement.rs` | 4, 6, 5 | |
| `crates/turts_client_web/index.html`, `crates/turts_client_web/src/lib.rs` | 2b (`lib.rs` only), 9 (`lib.rs:35`; the input sequence listener in `index.html`), 12, 15, 16 | |
| `scripts/boundary.sh`, `crates/turts_checks/tests/boundary.rs` | 8 | |
| `crates/turts_checks/tests/support/mod.rs` | 3 | |
| `scripts/qa/driver_rule.py`, `tests/qa/driver/cases.tsv` | 20, 21 | role `harness`, `--ledger-limit` (default 0) |
| `scripts/mc-browser.sh` | 20, 21 | 21 builds `params.bindings` with the task 9 example |
| `tests/browser/legacy-bridge.tsv` | 12, 21 | R6 |
| `scripts/acceptance.sh` | 7, 10, 22 | |
| `.github/workflows/ci.yml` | 21, 24 | |
| `docs/measurements/mc.md` (new) | 13, 22, 24 | R9 |
| `mods/forge/manifest.ron` | 18, 17 | |
| `Cargo.lock` | 8, 9, 15 | 8 adds the crate and glam; 9 adds the `turts_client` to `turts_interaction` edge (`--locked` refuses a new path dependency, gate 2 td probe); 15 adds the Bevy `audio` feature dependencies |
| `crates/turts_client/Cargo.toml` | 9, 15 | |
| `crates/turts_interaction/Cargo.toml` | 8, 9 | 9 adds `turts_mod_runtime` as a dev-dependency for the example `effective_bindings` (not in `cargo tree -e normal`, so T1 holds) |
| `crates/turts_client_web/Cargo.toml` | 16 | `web-sys` features only |

## 7. Budget and seats

CEO approved about 212 total runs. Build stop line: 170 (106 planned task runs plus 64 repairs). Gate estimate: 42 spent before this build. Build dispatches used: 29/170. Root preflight is recorded separately and is not a builder/reviewer dispatch. Twice a task estimate requires a recorded continuation decision; four times is a hard stop. T18 estimate2: soft4, hard8. No dispatch at global170 or a task hard limit.

One run is one agent dispatch (builder or reviewer). Count every run on the board.

| Task | Runs | Builder seat (Luna) | Reviewer seat (Sol) |
| --- | --- | --- | --- |
| 1 | 3 | web-platform-engineer | ux-designer |
| 2 (2a 2, 2b 4) | 6 | network-engineer | mod-runtime-engineer |
| 3 | 6 | mod-runtime-engineer | mod-builder |
| 4 | 8 | simulation-core-engineer | qa-lead |
| 5 | 4 | simulation-core-engineer | qa-lead |
| 6 | 6 | simulation-core-engineer | performance-architect |
| 7 | 4 | backend-engineer | network-engineer |
| 8 | 2 | technical-director | qa-lead |
| 9 | 6 | bevy-rendering-engineer (ux-designer writes the headless core tests) | technical-director |
| 10 | 2 | bevy-rendering-engineer | qa-lead |
| 11 | 4 | bevy-rendering-engineer | ux-designer |
| 12 | 8 | bevy-rendering-engineer with ux-designer | mod-runtime-engineer |
| 13 | 5 | bevy-rendering-engineer | ux-designer |
| 14 | 3 | bevy-rendering-engineer | simulation-core-engineer |
| 15 | 5 | bevy-rendering-engineer | ux-designer |
| 16 | 6 | web-platform-engineer with ux-designer | backend-engineer |
| 17 | 4 | ux-designer | mod-builder |
| 18 | 2 | mod-builder | mod-runtime-engineer |
| 19 | 3 | mod-builder | ux-designer |
| 20 | 3 | qa-lead | web-platform-engineer |
| 21 | 8 | qa-lead | web-platform-engineer |
| 22 | 3 | qa-lead with performance-architect | simulation-core-engineer |
| 23 | 3 | player agent and the person | ux-designer |
| 24 | 2 | orchestrator (Sol); qa-lead runs the REG1 plants | - |
| Subtotal (tasks) | 106 | | |
| Gate 1 (spent: 23 agent turns, resumed turns included; on disk 17 report files `docs/research/mc/gate1-*.md`: 5 lens reports and 12 re-checks; re-check 1 td N2) | 23 | | |
| Gate 2 (spent: 6 round 1 reports, 6 re-check 1 reports, 1 plan draft, 4 repair passes; planned: 2 for a re-check 2 of bevy and qa-lead) | 19 | | |
| Repair allowance (60 % of the subtotal) | 64 | | |
| **Total** | **about 212** | | |

## 8. Run ledger

One row per builder/reviewer dispatch or resumed agent work turn. Register before dispatch. Do not add fake red, green or reviewer results.

| Run | Task | Seat | Model | Effort | Base | Result |
| --- | --- | --- | --- | --- | --- | --- |
| 1 | T1 build | web-platform-engineer | gpt-6-luna | high | b563624 | d524fa3; Chrome controls/keys exit 0; Firefox no window exit 10; stop. |
| 2 | T1 evidence review | ux-designer | gpt-6.1-sol | high | d524fa3 | PASS for evidence handoff; no High findings; hashes/G-QA exit 0; scratch hash plant red1/restored0; overall T1 incomplete. |
| 3 | T1 supplementary input records | web-platform-engineer | gpt-6-luna | high | d524fa3 | 45c7eab; all supplementary Chrome rows traced; hashes/G-QA exit0; 184KiB evidence. |
| 4 | T2a build | network-engineer | gpt-6-luna | high | f8b204e | e8f9e02; semantic red101, scoped green3/3; G-QA exit0; full G-RUST/G-QA passed in fresh-local gate. |
| 5 | T2a review | mod-runtime-engineer | gpt-6.1-sol | high | e8f9e02 | PASS, no findings; named plant red101/restoredgreen0, clean; full G-RUST/G-QA passed by root. |
| 6 | T3 build | mod-runtime-engineer | gpt-6-luna | high | 8f612a3 | cac1d76; all22HUD3cases semanticred; scopedgreens API/runtime/pack39/no-content; fullgates running. |
| 7 | T20 build | qa-lead | gpt-6-luna | high | 17286b1 | Read-only contract preparation complete; no code or baseline run; waits for compiler slot. |
| 8 | T3 review | mod-builder | gpt-6.1-sol | high | cac1d76 | FAIL49d42a:5High/5Medium; detailedrepros; repairscommitted d805669 pendingrecheck. |
| 9 | T3 review recheck | mod-builder | gpt-6.1-sol | high | d805669 | FAIL d805669:0High/2Medium; sourcefixes and immutablebinaryrecheck pending. |
| 10 | T3 second recheck | mod-builder | gpt-6.1-sol | high | 08b72a1 | FAIL08b72a1:1High OGGpost-EOS/serialreuse,1MediumRONcomments/rawstrings; fixed67307d1 pendingrecheck. |
| 11 | T3 final functional recheck | mod-builder | gpt-6.1-sol | high | 67307d1 | FunctionalPASS67307d1:0High/0Medium;56CLIobservations; G-REF/numericreceiptpending. |
| 12 | T20 draft/build continuation | qa-lead | gpt-6-luna | high | 17286b1 | Current base22a4600; clean normal-native M2 baseline15/15 exit0; temporary diagnostics restored; Candidate9a3365a committed; actual DPR2 row twice and direct DPR1 pass; task gates and plant running. Hosted software cause remains NOT TRACED. |
| 13 | T20 independent review | web-platform-engineer | gpt-6.1-sol | high | 9a3365a | FAIL9a3365a:0High/2Medium; actual held-modifier loss with opts.key at bothDPRs; missing --windows N. Independent plant1/restored0; clean. |
| 14 | T20 repair | qa-lead | gpt-6-luna | high | 9a3365a | Candidatea8a375c committed; named reds1, repaired Node15/15 and actual DPR1/rowDPR2 twice pass; postcommit gates/plant running. |
| 15 | T20 independent recheck | web-platform-engineer | gpt-6.1-sol | high | a8a375c | FAIL a8a375c:0High/1Medium. Original two findings closed; CDP-held modifier disappears from later normal key/click, actual game bothDPRs. Clean. |
| 16 | T20 final authorized repair | qa-lead | gpt-6-luna | high | a8a375c | 72ddb0a committed; G-QA/G-WEB0, Node17/17; actual DPR1 + twice DPR2 pass; page plant1/restored0; clean. STOP6/6; no further dispatch/merge/push without CEO ruling. |
| 17 | T20 CEO-authorized independent recheck | web-platform-engineer | gpt-6.1-sol | high | 72ddb0a | PASS72ddb0a:0High/0Medium; fresh actual gameDPR1/2 and CLI override0, Node17/17; clean/idle. CEO conditional merge/re-gate/continue is satisfied. |
| 18 | T8 build | technical-director | gpt-6-luna | high | Wave2 base | 91d6db7; T1X red101/green0, fmt/clippy/wasm0; full testsNoContentShellcollision101; STOP before G-QA/finalplant, clean. |
| 19 | T18 build | mod-builder | gpt-6-luna | high | adfb662 | STOP under T8 ruling; API/package prep only, no edits/Cargo. Icon-assets file-list omission clarified by root; HUD stays text-only. |
| 20 | T19 build | mod-builder | gpt-6-luna | high | adfb662 | INTERRUPTED on global stop; untracked muster.rs and external package drafts retained/archived. No Cargo. |
| 21 | T8 O10 continuation | technical-director | gpt-6-luna | high | 36a86ae | COMPLETE01621d0: O10 rename and lock-seed repair; fullgates0,861/0/18; semantic reds101/greens0, clean. |
| 22 | T8 independent review | qa-lead | gpt-6.1-sol | high | 3d345f6 | PASS01621d0:0High/0Medium; lock-seed Medium closed, independent boundary13/no-content9 pass0; host-row plant101/restoredgreen0, clean. |
| 23 | T18 build continuation | mod-builder | gpt-6-luna | high | f564f88 | STOP0e83be5: focused4green, packs/scans0; fullgate staleHUD3fixture failed (outerRC notcaptured), GACC interrupted130; no repairs/plants after global stop. |
| 24 | T19 build continuation | mod-builder | gpt-6-luna | high | f564f88 | STOP test prep finalSHA3fc474607e563d0d3d902b8d32592964cf68f691ab3291e6cb9f7f2fe0046acc; drafts preserved; no Cargo/scan/package implementation. |
| 25 | T18 independent review | mod-runtime-engineer | gpt-6.1-sol | high | 0e83be5 | STOP noPASS:0High/4Medium plus lab/turret coverage concern; inherited belt contract conflict escalated; read-only only, no Cargo/plant. |
| 26 | T18 C1-C10 repair | mod-builder | gpt-6-luna | high | c244c0a | COMPLETE179ada7: fullgates0,870/0/18; GACC corrected0, registered E2/E3 red101/restored0; input-row red101/green0; clean. |
| 27 | T18 independent recheck | mod-runtime-engineer | gpt-6.1-sol | high | ead4fef | PASS179ada7:0High/0Medium;36 independenttests0, own input-rowred101/restored0; source/PNG/frozen/gates/C7 audited; clean/idle. |
| 28 | T19 implementation | mod-builder | gpt-6-luna | high | 62bb518 | COMPLETE39a74e3: missing-package/repair named reds101, 8tests0, strict scan9/9, G-RUST/GQA0; merge98135d3. |
| 29 | T19 independent review | ux-designer | gpt-6.1-sol | high | 39a74e3 | PASS0High/0Medium; 8+9tests0; saved-copy offset plant0/101/0, exact restoration; task4runs/hard12. |
| 30 | T2b implementation | network-engineer | gpt-6-luna | high | 363d02d | COMPLETE64cd400;14gate receipts0,886/0/18,10GREF(69commands) namedred/restoredgreen; merge6d2c46e. |
| 31 | T2b independent review | mod-runtime-engineer; qa-lead registry lens | gpt-6.1-sol | high | 64cd400 | PASS0High/0Medium;26focused tests0, capplant0/101/0, all14gates/10GREF audited; sourceclean; task2runs/estimate4. |
| 32 | T4 implementation | simulation-core-engineer | gpt-6-luna | high | ab6ffd8 | COMPLETE2c45b83:984/0/18,all gates0,36GREF/156acceptedcommands,2planplants+7C55; merge0490ebd. |
| 33 | T9 core-test preparation | ux-designer | gpt-6-luna | high | ab6ffd8 | COMPLETE draft-only; external tests/contract, SHA76817063...a2d629, T9 clean, no Cargo, no observed red/green. |
| 34 | T4 independent review | qa-lead | gpt-6.1-sol | high | 2c45b83 | PASS0High/0Medium;65independenttests0,ownsemanticplant0/101/0; exactrestore/cacheidle. |
| 35 | T9 implementation preparation | bevy-rendering-engineer | gpt-6-luna | high | 3203f6b | COMPLETE preparation only: imported tests, API stubs, PROJ1/CLICK9 drafts; uncompiled/unexecuted/uncommitted; task2/estimate6/soft12/hard24. |
| 36 | T9 implementation continuation | bevy-rendering-engineer | gpt-6-luna | high | 61535fc | COMPLETE native implementation handoff only; initial semantic reds, C49/C69-C73 repairs and native proofs. Task verification continues in RUN38. |
| 37 | T9 independent review | technical-director | gpt-6.1-sol | high | abe0841 | COMPLETE PASS0High/0unresolvedMedium;69 independent checks,7/7 GREF audited,FOV3 audited, own modifier0/101/0,63 source hashes restored, clean/cache ROOT. |
| 38 | T9 pixel proof and final verification continuation | bevy-rendering-engineer | gpt-6-luna | high | abe0841 | COMPLETE: fresh base/CLICK9, all standard gates0,7/7 GREF45commands,FOV0/101/0, clean final index. T9run5/estimate6/soft12/hard24. Historical setup failures preserved. |
| 39 | T6 group movement builder | simulation-core-engineer | gpt-6-luna | high | d3ef732+dirty | STOPPED handoffcomplete pendingCEOdesignruling; validtiny-mapred101,other18movementtests0withprobeexcluded; fourdirtyfilespreserved/cacheidle. No finalgates/PASS. |
| 40 | T7 server session builder | backend-engineer | gpt-6-luna | high | 487f6c9 | COMPLETE RED-READY source preparation only; scoped hashes/stubs/inventory, no compiled red or behavior acceptance; estimate4/soft8/hard16. |
| 41 | T11 camera and light preparation | bevy-rendering-engineer | gpt-6-luna | high | 30fc272 | COMPLETE source preparation only; scoped snapshots, static0; native/browser red and behavior NOTTRACED; estimate4/soft8/hard16. |
| 42 | T6 independent performance/determinism review | performance-architect | gpt-6.1-sol | high | 487f6c9+working | COMPLETE early SOURCE review only;14High/1Medium stableH1-H15 with9-file exactsnapshot/17areas; finalruntime/ownplant/PASS pending, new run count before reviewer continuation; T6run2/estimate6/soft12/hard24. |
| 43 | T12 widget/guide/cursor preparation | bevy-rendering-engineer with ux-designer | gpt-6-luna | high | 2b22fa8 | COMPLETE analysis only; no code/stubs/rows created, notREDREADY; rootverified28testinventory/3actualplants; native/browser NOTTRACED; estimate8/soft16/hard32. |
| 44 | T12 HUD1 instrument preparation continuation | bevy-rendering-engineer with ux-designer | gpt-6-luna | high | 2b22fa8 | COMPLETE HUD1 instrument preparation:8650131,Node75/0 with13oraclecases, strictDPRregression/plant0/1/0,QA0; actualbrowser/nativeHUD1NOTTRACED; T12run2/estimate8/soft16/hard32. |
| 45 | T6 final independent review/recheck | performance-architect | gpt-6.1-sol | high | b4/d3+separatebuilderdirtyproof | COMPLETE SOURCE-ONLY understop; confirmsgeometryandbuildersemanticred;17areas/openHfindingsretained; no runtime/ownplant/PASS. |
| 46 | T6 O12 repair continuation | simulation-core-engineer | gpt-6-luna | high | d4a8f97+preserved dirty | COMPLETE WIP handoff only: C106-C110 simulation/replay focused proofs; six files uncommitted, no final gates/GREF/plants/PASS. Continued by RUN48. Task6run4/6, soft12/hard24. |
| 47 | T6 independent O12 and retained-finding recheck | performance-architect | gpt-6.1-sol | high | rebased d4a8f97+active repair | COMPLETE SOURCE-ONLY, no PASS; final finite report c115-c116-final-source-boundary. After C105 preliminary runtime: ffb21b3 verified gates; H4/H14/H12 compiled semantic101, other probes pass. Lease returned; repaired runtime/own plant/final PASS pending. Task6run5/6, soft12/hard24. |
| 48 | T6 C106-C110 repair completion and final proof | simulation-core-engineer | gpt-6-luna | high | ffb21b3+six-file coherent WIP | COMPLETE WIP only: seven-path coherent boundary, full sim/server green, final source review has unresolved findings. No commit or formal repaired gates. Originally registered before continuation. Finish focused/replay controls, commit, all required final gates, C105 handoff, selected GREF and semantic plants. Task6run6/estimate6, soft12/hard24. |

| 49 | T6 finite source findings repair and task completion | simulation-core-engineer | gpt-6-luna | high | ffb21b3+seven-file coherent WIP | Registered before continuation: fix exact RUN47 final report, meaningful controls, commit, normal gates, C105 runtime handoff, GREF/plants. Task6run7/estimate6, soft12/hard24. |
| 50 | T6 independent finite finding closure and final review | performance-architect | gpt-6.1-sol | high | ffb21b3+RUN49 repairs | Registered before reviewer continuation. Source-only until stable clean candidate, all normal gates and explicit C105 runtime lease. Recheck finite report, retained actual counterexamples and own final plant. Task6run8/estimate6, soft12/hard24. |

## 9. Preflight ledger and stop

| Event | Evidence | Result |
| --- | --- | --- |
| Root setup | git status; worktree list; git show and SHA256 of design at 488b3d3 and a0a3076 | Design bytes match: 82d49b19c7e483793569cf4cd80d0b47108590e78913a8ecfa948fbed5929508. |
| P1 conflict | Design:363-364; plan:300,303 | Design requires default prevented in the output table. Plan says it is not used and omits it. STOP before implementation. |
| Precedence error | Brief section 1 versus user instruction and plan | User explicitly resolved: design revision 10 wins. Do not follow the brief's plan-wins sentence. |
| CEO decisions | Current user instruction | 212 approved; stop at 170 build runs; CEO plays both tests, not blind; exact base a0a3076. |

Historical preflight proposal: retain `defaultPrevented` in the raw records and the table, in addition to `cancelable`; use reaches-page and browser-acted for K11. No probe ran before this initial stop. O9 accepted and released this proposal below. The current stop is the Firefox result, not the column conflict.


## 10. O9 release, 2026-10-30

CEO relayed ruling O9 at f503cff and released the P1 preflight stop. The P1 table keeps `default prevented` (event.defaultPrevented read after the handler) next to `cancelable`. Record it; do not judge it. K11 uses only `reaches the page` and `browser acted`. Both integration and T1 were rebased on main at f503cff. Rebase kept both work-log entries when their append points conflicted. Prior proposal above is now accepted. No other task is released.


## 11. P1 result stop, 2026-10-30

Evidence commit: `d524fa317a91236482ac0be26c7c4b070a65605c` on `codex/mc-t1`. Table: `docs/research/mc/p1-keys.md`. First line: `Firefox NOT TRACED`. Chrome version: Google Chrome 156.0.8078.47. All 31 required Chrome K11 keys reached the page and had no observed browser action. The table records `cancelable` and `default prevented` next to each other per O9; neither is a K11 judgement input. Ctrl+9 uses the separate nonlast-tab retest, not the invalid same-tab observation.

Controls: Ctrl+T visibly opened New Tab; Ctrl+W closed its only browser window. Both controls passed, exit 0. Chrome keys and the corrected Ctrl+9 retest exit 0. Fresh first-click audio: suspended to running. Real Esc under pointer lock: lock true to false, no Escape keydown at the page. Root independently matched all 31 table rows to raw key events, checked F10/Ctrl+9 screenshots, ran bash -n and verified all five SHA256 receipt entries, exit 0.

Firefox effort: 04:17:20 to about 04:19:38 UTC (2m18s), unrestricted session. Version command stalled in snap-confine D state. Headed control attempt returned no window, exit 10 after 11.38s. All Firefox rows NOT TRACED. Safari and seven supplementary Chrome pointer-input rows NOT TRACED. No browser experiments after this stop. Snap child PID 132330 remained in D state at builder handoff; no system files or other browser sessions were changed.

The CEO must pick K11 with the Chrome table. No pick is inferred. No task merge, re-gate, wave completion, push or hosted CI is claimed. Both worktrees remain for review. All tasks except T1 are NOT STARTED.

Run2 final: ux-designer PASS for the stopped evidence handoff, no High findings; T1 overall incomplete. G-QA/hash checks exit 0; scratch-only copied-log corruption red exit 1, restored exit 0. Review saved in [[Reports/2026-10-30 MC P1 review]]. Root re-ran G-QA on integration and git diff --check: all exit 0.


## 12. CEO release and disk control, 2026-10-30

[[Decisions/2026-10-30 MC Firefox deferral and continue]] releases the Firefox stop. K11(a) stays; Firefox remains NOT TRACED. Prior stopped-state entries above are historical. Remaining P1 pointer rows were completed in run3 before merge.

No active Cargo, Rust, Trunk or TuRTS process was present at cleanup. Removed only obsolete generated gate2-player/{debug,wasm-bindgen,wasm-opt,wasm-release,wasm32-unknown-unknown}, gate2-sim/debug, claude-verify/debug, and main debug/incremental. Reclaimed 26,027,208,704 bytes (24.24GiB); free space rose from557GiB to581GiB. Retained old evidence and source worktrees. Receipt: main target/cleanup-archive/20261030-mc-cache-cleanup.json. New build lanes use debug=0 and incremental=0. Delete task targets/worktrees after each verified merge; retain only compact evidence.

Warm cache rule: only one full workspace compiler cache is kept. Root transfers it by rename between idle lane targets; there is no symlink/hardlink sharing and no concurrent mutation. Required CARGO_TARGET_DIR remains specific to the executing lane. Scoped test outputs are disposable once their durable logs exist.


## 13. T1 gate and T2a ready

AfterT1 all ten required gate commands passed, frozen hash unchanged; source8f612a3. Logs: target/mc-evidence/after-t1/results.json. Removed the clean merged T1 worktree; no T1 build cache exists. Branch retained.

T2a sourcee8f9e02: independent review PASS, plant red101/restored0, taskG-RUST/G-QA all0. Warm-cache relocation initially exposed a stale local net-protocol artifact: Clippy failed to find the newtypes despite correct committed source. Invalidating all workspace packages with cargo clean -p (keeping third-party dependencies) fixed this; fresh-local strictclippy/wasm/workspacetests/boundary/G-QA allpassed. No source change was needed. Preserve failed gate log as an artifact, not semantic-red evidence. Logs: target/mc-evidence/task-t2a andtask-t2a-fresh-local; reviewer target/mc-evidence/t2a-review/report.md. Future cache transfers invalidate localworkspacepackages before verification.

T2a merged17286b1; after-t2a G-RUST/G-QA/G-FROZEN all0, workspace799/0/18, frozenhash unchanged. CleanmergedT2alane removed. Onecache moved toT3, tenlocalworkspace packages invalidated before fullslot release. T3 now ownsfullcompiler slot; T20 mayread/draftexternally but mustpreservecleanworktree forstep1 baseline untilslotrelease.


## 14. T3 review and repair

Candidatecac1d76 plusclippy49d42a: scopedAPI/runtime/pack/no-content greens andordinaryfullgates passed. Independent run8 found5High/5Medium functional/diagnostic failures withrealCLIprobes. Initialreport preserved in [[Reports/2026-10-30 MC T3 initial review]]. G-REF29cases washeldbeforeexpense.

Repaird805669: packbad_packages56/56, runtime120, API14, no_content9 green; ordinaryG-RUST/G-QA/G-FROZEN all0. Run9 freshCLIrecheck closedall5High and3Medium;2Mediumremain: completephysicalkeycodes andwhitespace-safeOrderRuleserrors. Buildcaps/foreignrefs/dynamicimagebindings/reservedchords fixed. NoT3merge.

Oneevidencelimit: original22HUD3semanticredlogshavecompilednamedfailures butno durable numericexitmarker; do notinventhistory. Reserved/chainedOggregressionreds recordactual101. Afterfullrequiredgates, builderwillrepeat signature-stub instrument proof inisolatedscratch withactualred101/restoredgreen0. Finalreviewwilluseoneimmutablereviewbinarybuiltfromcleancommit withSHA/source/fixtureprovenance; liveCargooutputs changehashonrebuild. Snapshotbinaryisremovedafterreview, hash/logskept.


## 15. T3 accepted, 2026-10-30

Source67307d1; no High/Medium functional findings remain. FulltaskG-RUST/G-QA/G-FROZEN0 and pinnedhash unchanged. G-REFselectionindependentlycomputedfromactualchangedfilesmatches17M2+12M3; bothbatchcommands exit0. Rootreadall29caseJSONs and87baseline/red/restoredlogs, confirmednamedtestfailureswithoutcompilererrors, restoredblobhashesequalbaselineandcommittedGitblobs; auditedsource67307d1/provenance. Rootaudit target/mc-evidence/t3-root-verification.json.

Originalsignaturestubredlogsnumericlimitationisnotrewritten. Aftergatesanisolatedcopy repeatedtheinstrument: bad_packagesstub20passed/38failed actual101, restored58/58 actual0; headerstub1passed/4failed actual101, restored5/5 actual0. Exactsourceblobrestorationandallloghashesverifiedbyroot. Numericreceipt target/mc-evidence/t3-builder/signature-stub/provenance.json. Rootauditfirstmisclassified restored.green asred bysubstring; exactfilenameclassifiercorrectedandauditpassed, no sourcechanges.

BothGREFmutantcaches/scratchsourcesremovedafterhash/patch/commitreceipts (M2cache3.7GiB plus40MiBcopy; M3sizeinitscleanuprecord). Accepted30MiBreviewCLIandearlier30MiBsupersededCLIremovedafterSHAverification; manifests/reports/probesretained. Primarycachecontainsreusabledependencies; tenworkspaceartifactsinvalidatedafterstubrestoration.


## Forward checks for the owning tasks

- T18: the plan says to rename a colliding local id, but the user and brief require a stop on any red content-id table entry. The user stop rule wins. Do not rename that table entry without a CEO decision. No collision has been observed.
- T16 / SLOT1: audit the harness setup with the persistence and refusal rows. Current Task20 startup eagerly waits for every page to join, and uses a browser context per page. SLOT1 needs expected refused windows; design10.4 expects shared localStorage per origin and separate sessionStorage per window. The owning persistence test must set up those actual browser semantics and the copied-id case, and must not count a pre-drive timeout as the expected gameplay red. Current Task20 browser proof covers two joined windows and DOM delivery only.

- T24: existing CI Node step explicitly names only three old browser tests (ci.yml:108). Include the new harness contract and CLI tests when wiring final CI so the raw-page plant remains a hosted guard, then inspect actual Node step output.


## T20 task budget stop

T20 estimate3; runs7,12,13,14,15,16 total6. Run16 is the last authorized repair. Finish its concrete candidate and gate proof, then stop and report to the CEO before another run, merge, wave push or new task. The brief section3 says: "A task that reaches twice its estimate stops and reports." Global stop remains170; currentbuild16/170. An independent recheck after run16 will need a CEO extension.

Final stop receipt: target/mc-evidence/t20-builder/final-repair/root-stop-audit.json. Request: one additional independent review run (T20cap7, wholebuild17/170) and continuation only if that review passes. No decision or extension has been granted yet.


## CEO release: T20 cap7

The CEO granted exactly one additional independent web-platform-engineer review of72ddb0a, run17/global17of170. If PASS with0High, merge T20 into codex/mc-integration, re-gate and continue Wave1. If FAIL, STOP/report findings before any further run. This supersedes the earlier pending request and cap6. See [[Decisions/2026-10-30 MC T20 seventh review run]]. Other stops and the ban on main merge/push remain.

Run17 result: PASS0High/0Medium on72ddb0a. [[Reports/2026-10-30 MC T20 final review]]. CEO conditional authority now permits integration merge, re-gate and continuation. Root verified the actual report, candidate head and clean source.


## Wave1 integrated and pushed

Merge8e95e42346d260f24340b9e3b27c7b92e2ea820f includes T20. Integration after-t20 gates all0: G-RUST858passed/0failed/18ignored, G-QA, G-FROZEN (hashunchanged) and Node17/17. Evidence target/mc-evidence/after-t20. Merged T20 worktree removed after re-gate; single cache remains on integration. All local workspace artifacts were invalidated across dev/native-release/wasm-check/wasm-release profiles after transfer; third-party cache retained.

Pushed codex/mc-integration; draftPR https://github.com/26medias/TuRTS/pull/2 (main untouched). CI https://github.com/26medias/TuRTS/actions/runs/41097571682: agents success; check/native tests and browser/T7 running at last read. Wave2 preparation can proceed; no Wave2 integration merge before results are assessed.


## Wave2 design conflict stop

T8 real_source_scan_is_clean failed101: identifier Shell atinteraction/lib.rs:140 matches forge resource-name Shell (resources/shell.ron,name). Design2.2:268 requires Shell{action}; design11.3:1040 forbids forge content names in engine. Frozenhash unchanged. No scanner, content or contract workaround chosen. Proposed but UNAPPLIED: rename the effect variant ShellAction, preservefields/semantics, forge content and scanner. [[Reports/2026-10-30 MC T8 contract collision stop]]. CEO ruling is required before any resumed run or Wave2 integration action.

T18 clarification: design12.2:1099-1101 says the HUD has textbuttons/noimages; it does not prohibit iconassets outside that HUD. Mandatory OrderDef.icon can be backed by minimal image exports while the HUD stays text-only. The task file-list omitted those required dependencies; adding them on resume is necessary implementation, not a schema/design waiver. Forge minimal icons also need an early asset handoff before Task17 expands them. No T18 data was changed.

Read-only CI watcher for already-running Wave1 run41097571682: unifiedexec session24160, output target/mc-evidence/wave1-ci-watch.log. It records terminal status without a new agent/build run. Local implementation remains stopped; ShellAction proposal is unapplied and the CEO question is pending.


## O10 release and rebase

CEO relayed O10 at mainb29e3d8, designrevision11: Effect::ShellAction{action}; behavior/data/scanner unchanged; clientmoduleclient_shell.rs. This releases the Shell contract stop. Integration rebased with merges preserved; the sole work-log append conflict kept both mainO10 notes and all execution entries. Implementation paths are byte-identical to pre-rebaseintegration. Open lanes will be rebased onto this integration; T19 untracked test is backed up and must retain its SHA. Run21 T8 continuation registered. [[Decisions/2026-10-30 MC ruling O10]].

Rebase verification: integrationf564f88, T8 36a86ae, T18/T19f564f88 all descend from b29e3d8. T8 implementation bytes are unchanged by rebase. T19 untracked test retains SHA cc73d25ad772faed177b128ae42a94ede038fe001ca0375b2fb937b1c238f8eb. Receipt: target/mc-evidence/o10-rebase/after.json in the main worktree.

CI job124207191614 logs read: T7 oracle passed91frames; M2 economy failed with the unchanged E9-browser-base-economy 15000ms timeout, exit1. M3 controls and browser refutations were skipped after that failure. This is the existing failure assigned to Task21; its cause remains NOT TRACED. Agents passed; native check still runs. Receipt: target/mc-evidence/wave1-ci-browser-assessment.txt.

Disk correction: earlier T8 commands omitted the required debug0/incremental0 overrides. Root archived fingerprints, a31384file inventory, process/source state, then removed the rebuildable debug output while no compiler was active (65653958328 apparentbytes). No source or evidence removed. Remaining release/wasm cache retained. All further Cargo commands must explicitly set CARGO_PROFILE_DEV_DEBUG=0, CARGO_PROFILE_TEST_DEBUG=0, CARGO_INCREMENTAL=0, CARGO_BUILD_JOBS=4. Receipt: target/mc-evidence/o10-cache-cleanup.

Task18 run23 resumes under O10 release. The earlier icon concern was a file-list omission: design12.2 constrains the HUD to text buttons with no images, while each OrderDef requires an icon. Minimal icon assets/exports satisfy both existing contracts; HUD remains text-only. No schema/scanner/design change. Tests must first fail semantically on current packages. Any Task18 table-ID red stops for CEO; the brief overrides the older plan rename suggestion. No compiler use until T8 builder/reviewer finish.

RUN22 confirmed Medium at boundary.rs179: scratch copied Cargo.lock to temp/Cargo.lock, but its workspace root was temp/crates/turts_interaction, so Cargo ignored that seed and generated a new inner lock. Reproduced locate-project0, locked/offline101, offline tree0 with newlock generation; receipt target/mc-evidence/t8-review/lock-location-repro.json. Root accepted finding and handed the compiler back to still-active RUN21 for destination repair and offline fixture enforcement, then commit/gates/plant. RUN22 stays active for final independent recheck. No extra dispatch. Initial candidate fullgates0 with861passed/0failed/18ignored independently audited by root. No Task8 merge yet.

Task8 merged at461d10555ad3a34d2c87e81bb7bfcf7aa2496674 after final independent PASS01621d0,0High/0Medium. Root audited final eight logs and861/0/18 counts. Warm cache moved by rename to integration; workspace artifacts invalidated in all four profiles, exit0 each. Integration G-RUST/G-QA/G-FROZEN now running. Wave1 hosted assessment remains: agents/T7 pass, browser has unchanged known M2 economy timeout, native refutations still pending; no hosted success claimed. Local Task8 gates permit integration work while root continues reading that old run. Main untouched.

Task8 integration re-gate complete: all10commands exit0 at source461d105 (intervening commits only vault documents). Frozen hash bcbf72c2206251f635a7c7f5b6c8f573d69a5fc05a5bd77716e98ebfb08e370b retained; forge cases, fixed point and M2 stream pass. Evidence target/mc-evidence/after-t8. Clean/merged lane archived and removed. Sole cache moved by rename to target/mc-t18, workspace outputs invalidated in all four profiles, all0. T18 source test retained; run23 compiler handoff authorized. [[Reports/2026-10-30 MC T8 review]].

Task18 candidate0e83be525b3e7cfa237957b349dc56db77d5affb committed;69owned files. Original semantic red is the missing usable Move order101; other three tests already passed. All four now pass, all five package CLI packs exit0 and per-package scans green. Root read exact intermediate scan output: only order-name/fieldname Move and Stop hit, never a prescribed table ID. New descriptive labels fix that authoring error without ID/scanner/schema changes. RUN25 independent review registered; builder RUN23 retains compiler for full gates and plant.

RUN25 source audit found four Medium issues on0e83be5: Forge Move lacks explicit Never fire policy; Cedar/Haul producer depots lack Produce/Cancel orders; Base/Cedar/Haul Gather rules follow generic Node movement; Forge lacks Node movement rule. Root checked data and design4.3/5.2, accepted repairs. Authored-policy checks have9named failures, exit1; resolver execution remains NOT TRACED until its owning task. All21PNG icons decode; fourHUDs text-only;22frozen package files byte-identical.

Task18 workspace gate also found stale HUD3 test setup in no_content.rs:304+, which inserts duplicate order/input/assets exports into updated Acme. Root authorized only fixture construction repair, preserving scanner support code, token expectations and assertions; copied bindings/assets must remain valid. This is a dependent fixture update, not a scanner/schema/contract change. Builder records actual failure and repairs/tests within active RUN23; no extra dispatch. Evidence target/mc-evidence/t18-review/initial-findings.md and t18-builder/logs.

## Belt toggle contract stop

Revision11 PT1 step6 (:1693) requires belt pause/restart. Section5.3 (:532) and planTask2b (:341) retain existing M2/M3 SetEnabled, whose command_m3.rs:73-80 applicability excludes conveyors. No plan task assigns a conveyor toggle extension. Root verified primary evidence and stopped all lanes for CEO/orchestrator ruling, choosing no workaround. T18 source stayed0e83be5 clean; activeGACC interrupted130 with own child processes cleaned. T19 test/drafts archived/preserved; all compilers idle. Build25/170, no further dispatch before ruling. Full report [[Reports/2026-10-30 MC belt toggle contract stop]].

## Authority update and Codex rulings

CEO main c3f09d0 applied. O11 at63788d7/design12 already changes PT1 step6 to Shell Press, same D/E, alert and timing; computed8s remains NOT TRACED until PT1. Integration rebased with merge topology retained; one work-log conflict kept all main and execution entries. Open T18/T19 lanes rebased; implementation bytes and untracked test preserved (receipt target/mc-evidence/c3-authority-rebase/after.json). Wave1 CI41097571682 now check=success, agents=success, browser=known unchanged M2 economy timeout.

C1: repair stale HUD3 fixture construction only, retain scanner and token assertions. C2: repair all Task18 Medium policy/producer/subject coverage findings before merge, with semantic tests. C3: continue Task18 past soft4 with bounded repair/recheck; hard8/global170 remain. Conflict, evidence and alternatives are in [[Decisions/2026-10-30 MC Codex rulings]]. These rulings are subject to orchestrator review after the wave. Run26 registered before dispatch. Historical STOP entries remain evidence; current state is released.

Rebased merge mapping for current ancestry: T1e896448, T2a44a74d4, T3b9fb2e5, T206fa803a, T8cd38f38. Earlier hashes remain the historical test/run receipts. Integration code was byte-identical across the c3f09d0 rebase; no acceptance change was inferred from a hash rewrite.

C4: extend schema_m3.rs old-package RON test dispatcher with typed MC orders/rules/input/interface/assets parsing. Preserve legacy and default-serialization assertions plus unknown-path failure. Actual workspace gate exit101 was stale dispatcher, not package rejection. No schema/scanner/frozen changes. Builder RUN26 and reviewer RUN27 remain active for repair/recheck. [[Decisions/2026-10-30 MC Codex rulings]].

C5: align dependent0.2.0 metadata assertions/anchors and Cedar explicit file/export enumeration with planned0.3.0/^0.2.3 MC additions; preserve economic assertions and original files. C6: reproduce then extend M2 variant instrument typed export/reference/path handling for MC data, preserving deterministic/provenance/rejection contracts. Root found these stale assumptions by source audit before another full gate iteration. RUN26/27 remain active; no new dispatch. Full details and alternatives [[Decisions/2026-10-30 MC Codex rulings]].

C7: selected G-REF E2-rename-canonicalizer/E3-policy-binding will use unchanged registered run_case in the clean committed lane with its single target cache, exact commands and all semantic/provenance/restore guards. Exclusive ownership and before/after clean status required. No fresh giant cache, registry or runner edit. C6 namespace detail also recorded: structural layout IDs/engine contexts/colors stay fixed; actual mod references remap, covered by nonempty fixtures. [[Decisions/2026-10-30 MC Codex rulings]].

C5 now also covers Acme archive lock and Forge version/API assertion updates plus Cedar10+14MC=24file check, preserving old world/catalog/hash assertions. C8 repairs the conversion fixture that decodes generated Base PNG assets as UTF-8: mutate only RON, preserve binary bytes and all conversion guards/trace assertions. Actual named exit101 output retained. RUN26/27 continue; no new dispatch. [[Decisions/2026-10-30 MC Codex rulings]].

C9: renamed Forge fixture must discover exported OrderDef IDs and include them in its existing scratch-copy namespace/local-ID rewrite. All three tests currently fail loader validation on forge:order_move inside clone; fixed point/cases passed. Preserve all command/trace/conservation assertions and original Forge content. No new dispatch; RUN26/27 remain active. [[Decisions/2026-10-30 MC Codex rulings]].

C10: first GACC on179ada7 passed all test assertions but returned1 in final retention due relative-vs-absolute target-root lexical comparison. Rerun unchanged script using absolute path to the SAME lane target cache; guards/source unchanged, no extra cache. Gate accepted only after actualexit0. Builder RUN26 continues; reviewer RUN27 holds. [[Decisions/2026-10-30 MC Codex rulings]].

C11 forward requirement for Task24: add explicit cargo test -p turts_checks --example m2_variants --locked to hosted check job. Workspace tests omit this test target; local builder/reviewer7/7 proofs and E2/E3 retained now. CI plumbing deferred to planned .github/workflows/ci.yml owner, not waived; complete before CI1/REG1. Carry alongside prior harness Node test hook requirement. [[Decisions/2026-10-30 MC Codex rulings]].

C12: post-T18 integration Rust/QA/frozen and Trunk passed; default headlessT7 failed before join with noGPUadapter. NVIDIA/Chrome versions verified. Corrective T7 uses documented private-Xvfb headed Vulkan launch, unchanged oracle/viewport/source and samecache. Default adapter cause NOT TRACED. No new agent run. Task18 merge b1b3834 remains pending completed integration web gate. [[Decisions/2026-10-30 MC Codex rulings]].

Task18 final acceptance: candidate179ada7 mergedb1b3834. Root G-RUST/GQA/GFROZEN all0,870/0/18, Trunk0. Default headless T7 failed before GPU startup; original result retained. C12 corrective headed Vulkan T7 passed138frames, movement/scene oracle0, code unchanged. Composite receipt target/mc-evidence/after-t18/final-gates.json. Lane clean/merged and removed; sole cache moved to T19, four workspace-profile cleans0; untracked T19 test preserved and branch rebased to include T18. Task18 used5runs vs estimate2; C3 overrun reason applies, below hard8. [[Reports/2026-10-30 MC T18 final review]].

Task19 candidate010368f committed from prepared source. Five named tests compiled and failed on missing package/fixture before implementation, exit101; five scoped greens and nine NoContent tests now0. Production/fixture copies differ only HUD. Real media decode evidence saved; independent UX review RUN29 registered while RUN28 runs gates. No new minor stop.

RUN29 initial UX review found three Medium authored-content issues and a camera-proof gap. C13 moves Rush to free card(1,3), preserving key/ID and adding mixed-selection proof. C14 bundles full portable Apache2 terms and actual font attribution, preserving TTF bytes. C15 strengthens authored pitch/yaw projection checks, actual rendering stays NOTTRACED until owning tasks. C16 adds fullHUD LastNotice Text binding while keeping exact three-root HUD1 fixture. Root verified primary data/design/media evidence. RUN28/29 remain active; no new dispatch or CEO stop. [[Decisions/2026-10-30 MC Codex rulings]].

C17: Task2b hook test uses the private sender capture in src/commands.rs:348, driving auto_move (join.rs:360), rather than exposing internals for tests/commands.rs; actual sent envelope and first-Move selection must be exercised. Original helper red retained; helper-only proof is insufficient. See [[Decisions/2026-10-30 MC Codex rulings]].

C18: repair RUN31 Medium debug.rs:221 empty-envelope legacy rejection fields now; require explicit semantic red/green and partial-acceptance regression. See [[Decisions/2026-10-30 MC Codex rulings]].

C19: repair RUN31 High Produce admission regression via existing catalog-selected pre-domain checks; restore malformed-input fixture semantics. C20: repair High E8 delayed-Ack anchor and retain exact semantic failure text. Independent recheck required; no waiver/retirement. See [[Decisions/2026-10-30 MC Codex rulings]].

C21: retain valid empty Move through legacy simulation rejection/audit; refuse other empty fan-outs without pending hangs. Keep existing E8 command-boundary reachability assertions. See [[Decisions/2026-10-30 MC Codex rulings]].

C22: repair explicit shared-group loss on client splits; pin4097 ->4096+1 through actual sender and test sequence exhaustion. RUN31 Medium, same active runs. See [[Decisions/2026-10-30 MC Codex rulings]].

C23: bootstrap absent per-worktree browser node_modules with offline npm ci from unchanged pinned lock, then rerun T7; retain infrastructure failure1. See [[Decisions/2026-10-30 MC Codex rulings]].

C25: carry validated per-package queue-depth data into ContentCatalog (u8, default16). C26: public queue-depth type follows designu8; private wide parsing preserves exact overflow diagnostics and invalid-input proof. Task4 RUN32 owns implementation, independent review required. See [[Decisions/2026-10-30 MC Codex rulings]].

C27-C32: Task4 QA repair decisions cover stateful plain replacement/implicit movement, per-unit Produce admission, retained deterministic job_id, target-check priority, complete audit operation spans, and mixed290 actual movement proof. All High findings must be repaired/rechecked before commit/gates; C32 Medium repaired now. See [[Decisions/2026-10-30 MC Codex rulings]].

C33/C34: audit span overlap is High and must be rejected/tested; principal-unit operation metadata is defaulted/omitted for legacy compatibility and must be semantically validated. Task4 repairs remain active under RUN32/RUN34; no candidate/gate acceptance. See [[Decisions/2026-10-30 MC Codex rulings]].

C28 clarified: historical single-subject Produce admissions stay; multi-subject requests get unit-local domain results including all-invalid/malformed-content cases. C35 requires real emitter phase coverage and final post-gate repetition. C36 implements shipped Move Unit/PointOrUnit policy/follow semantics using existing Move completion. See [[Decisions/2026-10-30 MC Codex rulings]].

C37: M3 selected refutations use a one-cache wrapper retaining original CLI frozen/cardinality/provenance plus unchanged run_case guards; QA audit required before actual use. No runtime proof yet. See [[Decisions/2026-10-30 MC Codex rulings]].


C38: Task4 replay resolves policy from an immutable InstalledCatalog-backed CatalogAudit order_definitions field, with default/skip-empty legacy serialization. Command-local policy is insufficient. Valid baseline, jointly forged policy and catalog-transition negatives required; early frozen checks, no expected-value changes. RUN32/34 remain active, build34/170. See [[Decisions/2026-10-30 MC Codex rulings]].


CEO2026-10-31 approved C39 faster repair loop and removal of duplicate test/build work, retaining all required final gates, plants and per-merge checks. C40 boxes Order payload without serde change; C41 preserves strict renamed-catalog equality; C42 updates dependent process fixtures while retaining full independent replay. See [[Decisions/2026-10-30 MC Codex rulings]]. Task4 candidate6e25954 has focused E9 spawn repair green; full gates and independent runtime are pending. Build34/170.

C43-C46 record valid-data/Remove semantic repairs, preserved mutation detectors and Task9 source/test-stub preparation while Task4 owns the compiler. RUN35 registered before dispatch; global35/170. No Task4 merge accepted.


Wave3 terminal CI41157502932 on6d2c46e: check124407427372 and agents124407427127 success. Root read raw logs; all33registered native M2 cases and17M3 cases have semantic RED/restored GREEN, full acceptance/F11 passed. Browser retains unchanged M2 timeout after T7(97frames). Raw logs/terminal metadata: main target/mc-evidence/wave3-{check,agents,browser}-job.log and wave3-ci-terminal-jobs.json. Check SHA f3fba3e0de94fa0127d9a74493f09b16afdc8e62171e85f470c896a13aba05f7; agents SHA b9bedfbdb9695d38e1f5c67561123d24ae03e9a0ab28f1afbe09fc84779cfd73. C47-C51 record Task9 read-only interfaces/selection/action identity and batched Task4 model fidelity, no gate waiver. Build35/170.

Task9 RUN35 preparation complete; no Cargo/compiler/browser/cache use, no behavior accepted. Root fixture-source corrections and C52 instrument names are documented. Task4 combined validator preflight27/27 exit0 after family repairs; QA source batch recheck pending. Global35/170. Disk685GiB free; sole25GiB cache and4.5GiB retained evidence.


Task4 finalproof closure:31validator tests and15static M2 harness tests exit0; QA has no concrete source blocker on the named checklist. C36 Point-only exception and all3 C44 same-defect registry reanchors are repaired. Remove dependent-link augmentation is synthetic/canonical; basic material-bearing Remove, machine refunds and research runs are actual captures. Late-refusal model fixtures are synthetic with real implementation-test ties. Root indexed25 exact raw development proof paths/hashes/ACTUAL_RCs in main target/mc-evidence/t4-builder/run32/root-finalproof-index.json. Candidate49f556c followed by test lint-only6056fc0 is clean; full final gates/plants/36GREF and independent runtime remain pending. C53 adds a read-only saved-proof auditor with QA source review and prior10case/69command validation0. Global35/170, no new dispatch.


Stable Task4 source moved49f556c ->6056fc0 for test let-chain lint ->0240e9a for engine-local protected-token rename. Root inspected0240e9a; no content/scanner/behavior change. Format/strict Clippy/wasm0; workspace test is live PID1840544 at current observation, exact source0240e9a and compact sole cache. C54/C55 record the collision repair and honest missing dev-red mutation metadata; focused post-gate proofs will close it. QA auditor recheck passed source c20bbf34; final M3 runtime still pending. Global35/170.


Task4 exact0240e9ae059e1e022ab5acb9dc8b4900f58ea640: root read final fmt/Clippy/wasm/workspace raw logs, all actualRC0. Workspace99groups sum984passed/0failed/18ignored. Candidate source clean. Remaining boundaries/QA/frozen/acceptance/web, post-gate provenance repairs,2planplants,36registeredrefutations and independent runtime remain required. No Task4 merge accepted. Previous goal turn classified PROGRESS: Task9 scaffolds/source corrections and authoritative rulings/evidence changed the next actions; this turn adds reproducible exact-source audit and verified final Rust results. Goal remains active, build35/170, main unchanged.


C56 root-helper source QA passed, actual postmerge M3 runtime pending; prior post2b helper invocation gap remains disclosed. Task4 final GACC is live PID1850880, source0240e9a, currently recovery scenario; do not restart from an observation delay. Root found original combined GQA files did not retain self-test stdout despite a summary saying they did. Builder confirmed and reran cheap checks into split persistent files: completeness-selftest21/21 and driver-selftest47 cases bothactualRC0; enforce/tree also0. Root read the exact new logs. No new build run or source change, global35/170.


Task4 all15 final gate logs on0240e9a root-audited actualRC0 in root-final-gate-audit.json. Full acceptance includes E8/E9recovery/E10/F11 and exactly one T5; all18retained F11 hashes verified with exact0240 clean provenance. T7=138frames/54moving, M3 controls Produce/disable/remove+7captures and inventory panel passed. Boundary/QAselftests/frozen all0; frozen value unchanged. QA independently audited7 C55 semantic transformations and both actual planplants with source/patch/hash/red101/restored0. C55 seventh metadata test index is being corrected from rawcommand; failed instrumentation attempts retained honestly.

GREF is live: original21M2 wrapper has11completed; omitted foreign-Gather will run as separate guarded component, union22 plus14M3 exactselector in root-gref-selection-0240e9a.json. C57 parent audit source-reviewed, actualnew splitruntime pending. Task9 source-prep guards now prevent empty-stub falsegreens, accidental doubleclick and offviewport composition; source snapshotv2 preserves19files in71,364bytes SHAef5065aa421165b5a6777c8dbb4e845b59e324b426c36ac167cc6c488cf2f9c5. Still uncompiled/unexecuted, no feature accepted. Pinned Bevy WindowEvent unified input stream notes prepared for actual adapter. Build35/170, main unchanged.


C58 repaired the expected diagnostic of the same E8 partial-write mutation. Candidate2c45b83 differs from0240 ONLY that registry expected_failure; root/QA verified all other tracked blobs and reusedcase definitions identical. Original failed wrapper remains failed; root projected18actuallycompleted M2 cases/78commands and audited original guards/source hashes0. All14selected M3/42commands completed on0240 and root/QA actual-source/frozen/whole-source audits0; explicit final-source metadata mapping0. Remaining4M2 on2c45 has foreignGather and mixedcost complete, E9privacy/output pending at observation. Independent reviewer source/gate/plant report prepared, verdictPENDING ownruntime/GREF. Global35/170. No main action.


Task4 accepted candidate2c45b8311906caf14793e7eb75b0bbb84462bdb1; independent RUN34 PASS0High/0unresolvedMedium. Root verified65actual tests16orders+7plain+41validator+1realE8, loghashes/RC0; ownautomatic-validator suppression compiledsemantic101 betweenbaseline/restored0, exactbytehashde85...; clean/cacheidle/frozenexact. Root audited36registeredplants/156acceptedcommands (120zero+36native101, includingprerequisites) with originalguards, actualexecutionSHAs and C58registry-only mapping. Full report [[2026-10-31 MC T4 final review]].

Merged Task4 as0490ebdac49ddf09dfbe0d43935309ec75aaca10 withrequiredtrailer; no conflicts. Root diff against2c45 excludingvault is empty, so integratedruntime/test/data tree matchesreviewedcandidate. Integration re-gate is pending before lane removal or Task9 compiler transfer. All43changedpaths are in review; 13,137insertions/368deletions include comprehensiveauditmodels/tests. Historical development/proof limits and failedinstrumentation attempts remain disclosed. Global35/170, Task4 used2runs/estimate8; nooverrun. Main unchanged; Wave4 waitsTask9 beforepush.


Task4 integrated re-gate complete on61535fc (code merge0490ebd):12commands allactualexit0,984/0/18, T7=138frames/53moving, actual M3 controls0, frozen exact. Source/HEAD/status stayed stable acrossgates; source tree excludingvault matches2c45 exactly. Root audit main target/mc-evidence/after-t4/root-final-audit.json. C56 now has actualruntimeproof, not onlysyntax.

Clean/merged Task4 lane archived in t4-completion-provenance.json and removed; branchretained. Onecache moved byrename tointegration thenT9,4workspaceprofilecleans each0; firstmove reclaimed7.9GiB+691.7MiB+6.7MiB+161MiB rebuildableoutputs. No evidence/protected main audit deleted. T9 source stashed/rebased61535fc/restored with all19hashes preserved (run35/rebase-after-t4/receipt.json), later fixture improvements documented; sole cache nowtarget/mc-t9. Root source-only preflight strengthened KM3 independent time/distance boundaries, made PROJ1 three separate cases and corrected target-kind/plain/forceMove composition (C59). Still no Task9 observedsemanticred orfeatureclaim.

RUN36 registered before implementation resume, Luna/high, global36/170, T9run3/estimate6/soft12/hard24. FullTask9 follows red→green→commit→gates→plants; --actions remainsTask10 stub; legacy traces/panel inputs survive until owning tasks. No wave4push untilTask9 complete. Mainunchanged.


RUN36 compiled-red checkpoint root-read: six headless targets failed semantically, all three PROJ1 viewport tests failed numeric tolerance; no compilation error in accepted red logs. Raw logs main target/mc-evidence/t9-builder/run36/{interaction-red-all,proj1-red-semantic}.log. RUN37 registered before dispatch for independent technical-director source review, then runtime/plant after builder cache handoff. Global37/170, Task9run4/estimate6/soft12/hard24. Original setup errors remain retained.


C60: root verified selection.rs:12-21 geometry contradicts design381/1225. Build drag bounds from all three intended projected centres; retain expected mobileids1,2/staticid3 exclusion and outside-centre test. Original compiled red retained; production never selects out-of-box matching units. No new run.


C61: planP1→reserved-key instruction includes conditional Esc/wheel rows; design328/332/351/353/354 wins. ExistingCtrlT/W and requiredF11/F12 remain; no globalEsc ban or wheel pseudo-key; owningTask11/16 retain explicitdesign behavior. Root audited allfive browser-acted yes rows; table unchanged. RUN36 root verified27headlessgreen0 and current checkpoint hashes; no finalTask9 acceptance.


C62: retain selected IDs inside each Last-closed frame for Task13 mouse-up-frame selection500 proof; Task9 owns schema. Do not substitute later top-level selection. Design1150-1159/SEL2 and plan481/Task13step2; instrument only, no new run.


C63: design requires debug.selection list; legacy M2/M3 scalar oracle reads migrate shape only, exact one-ID selection guards and rendered UI checks retained. No scalar compatibility workaround or early driver migration. Run actual browser paths/refutations atstablecommit. No new agent run.


C64: Task9 resolves independently rebound add/queue/type held actions from effectivebindings; physicalmods stillmatchauthoredchords. Task16 owns menu/storage, not first runtime support. Focusedoverride-away/independentadd-queue tests required; defaults unchanged. No extra run.


C65: repair future feed object shapes and owner-only evidence now; debug formatter serializes fulltypedfeed with same-typedefault. Rawcoreeffects are requests, not startedaudio/appliedcursor. Task12/15/16futureowners filltruthfuldefaults withoutlaterdebug.rs edits. Medium dispositionrepair; build37/170.


C66: followTask9green→commitorder withfullprebuildsourcehashmap/snapshot, frozenbothCLICK9attempts, unchangedaftermap and exacttestedbytescommitproof. ActualHEADdirtymetadata stayshonest; nofalseexecutiononnewcommit. Finalcleancommitgates/plantsretained. Earliermixedpairfailedwasmcompile/noBrowserresult. No duplicateidenticalpairrequired.


Root verified nativepreflight exactexitfiles0:40interaction/0failed,185client/0failed/1existingignored;9adaptertests hold process-globalclear-testMutex. ProductiononeAppperWasm behavior unchanged. Reviewer named source checklist clear, verdictPENDING runtime. Prebuildmap44723dbe...d3ab5b coversall47changed/untrackedpaths, allstillmatch; base61535fc tree1fb73afa...cae. Browserpaircurrentlylive, no passclaimed. Evidence run36/root-current-native-audit.json and root-pre-click9-source-audit.json.

Task24 forward: add tests/browser/harness/capture-contract.test.js alongside io-contract.test.js in the explicit NodeCIhook (or use harness/*.test.js). It executes actualpagecapture script;18boundaryregressions cost~40ms. Sourceonlyactualred6fail/12pass thenfixed18pass retained; neither replacesrealbrowserproof nor changesledgerlimits.


Frozen47-file CLICK9pair FAILED twice at missing acceptedresult, snapshotsblack; no subsequentframechecksran. ExternalheadeddiagnosticPASS is diagnosticONLY: actualscout359.64/135.71, panel12/12/390/42; bothclicksoutsideUI; seqs1-6 consumedonce butnoorders. RUN37found HIGH catalogidentity mismatch (archive222-223 qualifiedKEY/localdef.id; adapter337 clonedvalues; core658/675 comparesqualifiedrules). C49authority; builderactualcatalogRMB+KeyM/LEFTconfirm semanticred101→green0, independentrecheckPENDING. NeverchangeRMBcancelbehavior (firstgreen-namedtestusedwrongRMBconfirmandfailedhonestly).

C67: genericCoreInputQueue eventproducer seam isTask9; Task10actionsstaysstub. JoinednoWindow/once-onlyorder regression required before refreshedfreeze. C68: add1500mspreinputwait+latePNG, sameinputs/oracleguards; headlessblackcauseNOTTRACED. Bothrecordedcentral, no newrun; sourcefreeze refreshneeded.


C69: source audit found real future dispatch/config gaps under sharedtable172/C47. Add no-op camera/minimap dispatch, optional map/config view, initial-pose resource, runtimebindings/settings resolver, validated RestoreGroups input, world-point smart helper and complete typed CoreEffects now; future feature logic stays with11/14/16. No mutable Core getter or fakeWindow. Tests/default preservation and refreshedsnapshot required; no extra run.


C70-C73 recorded: owner-defined per-frame feedback snapshot/FrameCloseSet; shared UI capture/helper/presentation set; compatibility input.rs decoupled from panel deletion; readonly accepted_archives getter for future feature-owned metadata/assets. No future feature implementation, player change, test retirement or extra run. All known future file-contract seams must pass final source audit before next browser freeze.


RUN36 ended after native implementation/handoff while root completed C74-C75 pixel instruments. Task9 is NOT complete. RUN38 registered before follow-up for the remaining required browser/commit/gates/plants/GREF; global38/170, task5/estimate6. No budget exception or new task start. RUN37 remains independent reviewer, runtime/Highrecheck/ownplant pending.

C74 explicit headed profile honored for CI rows without skips; C75 real pixels determine scout click. Root59Node/47selftest/tree/shell checks0, PNG12image Pillowbyte-differential0, oldprofile2semanticfails and invisibleclickold1/new0 guards retained. Rust source unchanged by these JS/shell edits. Root verified latest45interaction/189client/1ignored0. Task24 must use privateXvfb/MC_BROWSER_HEADED1 profile and full harness/*.test.js; Task21 reuses purecodec. Actual new headed pixel pair remains pending.


C76: the first browser GREF baseline hit a legacy panel covering the Cedar depot after required camera fit removal. Translate only generated base/haul/cedar fixture coordinates and width, preserving relative geometry and all assertions. Original mods, production camera and capture rules stay unchanged. Failed baseline retained; final proof pending. See [[Decisions/2026-10-30 MC Codex rulings]].

C76 fixture alignment also covers the six existing absolute M2 lookups through a validated test-only X offset. Keep every owner/content/oracle guard, and translate after case-specific blocker insertion.

C77: repair own-hit gating for Ctrl/double type expansion and clear foreign double-click history. Stop GREF cleanly for the confirmed selection defect, preserve evidence and restore source before repair. No keys, rows or K16 change. See [[Decisions/2026-10-30 MC Codex rulings]].

C78: finish the legacy M2 world click before polling its result. Core correctly acts on PointerUp; the old helper waited before releasing. Preserve all predicates, timeouts and UI/refutation guards. First2 GREF cases passed; third failed its unplanted baseline naturally. C77/C78 repairs remain in RUN38.

C79: move cached UI publication to Last before FrameCloseSet so it follows real PostUpdate layout/visibility. Keep the late Gather visibility assertion and timeout. Require actual UiPlugin geometry proof and real base/CLICK9 reruns. C77/C78/C79 remain in RUN38 before a single final commit.

C79 also orders actual web/native Last feed readers after FrameCloseSet (and native readers before exit), so a fresh producer cannot be read one frame early. This is part of the same snapshot fidelity repair.

Task9 source accepted abe0841 after independent PASS: 0 open High, 0 unresolved Medium; 69 independent checks, all7 guarded GREF cycles, FOV3 and own modifier semantic plants restored exactly. Integration merge and root gates pending; build38/170, no new run or main action.


Task9 integrated asfb5e2d6454fefd7a69564a1788eaa52d0ee505e4, code tree identical to reviewedabe0841. All12 postmerge gates actually exit0; workspace106summaries1053passed/0failed/18ignored, T7 136frames/53moving, actualM3 Produce/set_enabled/remove and7captures, frozen SHA unchanged. Root read/hash-checked logs in main target/mc-evidence/after-t9/root-final-audit.json; source/HEAD/status stable. Pushed integrationfb5e2d6; hostedrun41195343950 uses PR merge8c9856e onto mainc3f09d0. Read jobs: agents124520280631 success; check124520280779 andbrowser124520280856 running. No hosted browser success claim yet. DraftPR2 remains draft/unmerged, metadata updated. All170remaining lane target files matched their archived hashes before removal; clean merged T9 worktree removed, branch retained, solecacheintegration. Receipt t9-root/lane-cleanup-complete.json. Build38/170,10of25lanes complete; fullgoal remains active. C39 speedups retain every required final gate and permerge gate.


## C82: overlap Wave5 preparation with the hosted Wave4 run (2026-10-31)

Conflict: the Wave4 push and hosted read are complete, but check/browser jobs are still running. Waiting without source preparation wastes independent work. Brief section4:43 requires a push/read after each wave; section4:53 allows wave/cache layout decisions. Plan48 requires each merge re-gate.

Choice: start Task6 and Task7 after the recorded read of run41195343950, with all pending statuses explicit. One compiler/cache owner only. Independent source/test preparation may overlap hosted validation. Root must read the terminal Wave4 jobs and findings before the next integration merge; no hosted success is inferred. Keep every task/final/permerge gate, independent review and refutation. Use two builders so an independent reviewer still has a slot. Alternative not taken: idle until long hosted tests finish, or run competing caches/merge without CI review.

RUN39/40 registered before dispatch: build40/170, no overrun; same-wave source preparation with sole cache assigned toTask6 after root transfer. Terminal Wave4 CI review required before next integration merge.


Wave5 dispatch actualbase487f6c9: RUN39 Task6 owns solecachetarget/mc-t6 after rename/four successful profilecleans (t6-root/cache-from-integration); RUN40 Task7 source-only toRED-READY. Task6 initial partial2test run compiled and failed named40-unit overlap assertion101; a separate lone-unit test had an inadequate200tick fixture horizon, corrected300 before next run. Root rawlog audit t6-root/initial-red-audit.json; full11named fixtures/design boundaries remain pending before production implementation. First-run test bytes preserved honestly before correction; no fabricated pre-run archive. This is PROGRESS, not Task6 acceptance. Global40/170, no taskoverrun, main unchanged. Wave4 latestread agentsgreen, check native tests andbrowserT7 running; terminal review pending before any nextintegrationmerge.

RUN40 Task7 source preparation complete, uncompiled RED-READY; root strengthened ignoredprocess test to assert scoutstationaryticks1-9 and actualfirstmovementtick10 (exact20Hz/6unitssec step), not only a logstring. Source-onlydiffcheck0; no runtimeclaim. Root review notes pending for implementation. RUN41 Task11 Luna/high registered before source/test preparation, no compiler lease; build41/170. All future final gates/rows retained. Main unchanged.


## C83: pending-spread refusal uses the existing wire rate_limited reason (2026-10-31)

Conflict: Task6 design6.3:632-633 requires per-unit rate_limited above16,384 pending subjects, but sim RejectReason lacks it (command.rs:82-121). Wire CommandRejectReason already hasRateLimited (messages.rs:181), contrary to the initial builder report. Task6 file list omits two exhaustive server mappers.

Choice: Task6 may add only sim RejectReason::RateLimited and as_str rate_limited plus the matching cases in server orders.rs:190-234 and export.rs:439-480. Add direct refusal/mapping coverage. No protocol enum or wire-format change; Task7 owns its session bucket separately and must reuse existing wire value. Preserve frozen stream/fixedpoint and select GREF from the actual changed paths. Alternative not taken: use QueueFull/Internal, omit the pending limit, or add a duplicate wire variant.

## C84: harness captures camera checkpoints without exposing state to the driver (2026-10-31)

Conflict: Task11 plan529-533 requires clip_from_world before/after each camera step, but harness/run.js:142 invokes the oracle only after drive; io has no bridge, and current frames do not retain camera matrices. A final-only camera oracle would weaken acceptance. Existing view.serverFile (view.js:27) can read harness artifacts from the output directory.

Choice: add opt-in harness-owned scene/debug checkpoint capture at each io.screenshot, identified by input-call index/window/time, and write the trusted checkpoint artifact after driver completion, before oracle. Driver still receives only PNG bytes and the same ten IO calls; view retains its same four read-only methods. CAM1 opts in through row params and the oracle reads the artifact through serverFile to judge every step. No game debug/schema or player change. Root owns harness files in Task11 lane; builder owns camera/row/oracle. Add meaningful Node regressions for actual hook, unchanged PNG/IO surface, selected-window identity and readonly snapshots. Actual CAM1 red/green/two passes/PointerLeft plant remain required. Alternative not taken: expose bridge state to driver, add a driver capture/read API, change game debug history, or reduce the oracle to final movement.

RootC84instrument commit026759e inTask11lane only: actual prehooksemantic redexit1; Node68allgreen, QA3+driver47self/tree0; postcommit focusedcontrolledhookplant0/1/0 exactrestore. Actualproductionrunner withfakebrowser proves driversavedforgery overwrittenbeforeoracle; no actualgameacceptance claim. Receipt main target/mc-evidence/t11-root/checkpoints/root-hook-plant-receipt.json. Camera/sourcepreparationRUN41 complete uncompiled/redunobserved. Task6currentcodecheckscompile andA40partialpasses; fullscope/gatespending. RUN42 performance-architect Sol/high registered for independentTask6source review, no compiler lease; budget42/170. Wave4latestCIagentsgreen, checknativeGREF andbrowserT7running, nofailedstepyet; terminalreviewstillrequired beforemerge. Mainunchanged.

C84instrument finalheadcf87b0e inTask11lane: Node69allgreen; actualold026source+newidentityregressionfails1, fixed0. Finalcontrolledhookplant0/1/0 exactrestore, scopedharnessstatusclean; no camera/browser/runtime acceptance. Root corrected childtestTransformimport and wrote Task7/Task11 implementationreviewcontracts outsidevault with truthfulpreparation-onlystatus. RUN42sourceaudit found14High/1Mediumprototypeissues (cost/scheduler/arrival/queue/audit); builderrepairingbeforebroadgates, finalverdictpending. RUN43registeredTask12analysis/fixtures only, no compiler/no legacyremoval; global43/170, task12firstof8. Runtime/taskmerge order andrequiredM3proofs preserved.


## C85: repair the omitted pool origin and nearest-angle ordering (2026-10-31)

Conflict: independent RUN42 H11 is Medium: the multi-unit pool omitted P and stopped ring generation without proving all nearer candidates were enumerated (captured movement.rs74-108, source-findings.md). Design6.3:611-613 requires a hex grid round P ordered by distance then angle; checked-in model_lib.py hex_slots includes P.

Choice: repair H11 with the other source findings; do not defer it. Include a legal origin P, preserve N+max(6,ceil(N/4)) pool size, and prove nearest distance/angle ordering with independent small-pool vectors and adequate complete-shell enumeration for large N. Existing reserved/static/bounds exclusions still apply. No new player behavior or frozen-content change. Alternative not taken: accept a visibly shifted multi-unit formation or trust only an overlap test.


RUN42 source phase ended with14High/1Medium (not15High); root read the full stableH1-H15 report and17-area inventory, source-findings.{md,json}. The earlier unqualified15High count is corrected. Builderrun39 repairinventorytracksallfindings; no finalreview/independentruntime/ownplant accepted. Any later completed-reviewer followup is a new ledger run first. C85 repairsMediumH11 rather than defers. Task12RUN43 activeprepactualbase2b22fa8, no compiler/legacyremoval. Main unchanged.


## C86: new widget scene rectangles use the existing scene logical coordinate space (2026-10-31)

Conflict: Task12 preparation found existing legacy UI snapshot rectangles in camera-viewport physical pixels, while the scene bridge explicitly declares window-logical-pixels-top-left (web lib.rs:484-494). Plan Task12 requires widget id/rect and HUD1 at three viewport/DPR combinations; copying old physical rectangles would double scale at DPR2. Design8.3:751-768 defines logical placement and Bevy-owned DPR multiplication.

Choice: turts_scene.widgets entries use id and rect [left,top,width,height] in the existing top-left window-logical scene space. Convert actual computed Bevy geometry exactly once; player UiScale remains separate from DPR. Keep the existing legacy turts_ui physical-coordinate contract until its owner migrates it; do not treat the old rectangle units as the new widget bridge contract. Add all three actual HUD1 viewport assertions and native computed-geometry proof. Alternative not taken: unconverted physical widgets, a second DPR multiplier, or an oracle that changes its expected coordinates to fit output.


Wave4browserjob124520280856 terminalfailure: exactsame normalized M2_CASE_TIMEOUT E9-browser-base-economy textasWave3 (actualcompletedriflemanoutput/renderedmesh15000ms,lastphaseJoinedslot0). HostedT7passed91frames/47moving; M2base-selection passed, base-economy timedout, laterM3/browserplants skipped. Underlyinghostedcause remainsNOTTRACED, ownerTask21; no relaxedpredicate/timeout or mainaction. Root read/archived raw188952byte logSHA73af6e03b8addf080ccea5d9bd01834fae209b95c6fb09c83c2b7185a84676de and comparedprioractualerror inwave4-browser-failure-audit.json. Check124520280779 stillnativeGREF running, agents124520280631 success/rawlog retained. Firstlarge-log write exceededOS argumentlength; nofilewrite/process occurred, then safe16kquotedchunks archivedexacttooltext. Terminalcheckreadstillrequired before nextmerge.

Task12RUN43 did sourceanalysis only, no teststubs/HUDrows/sourcechanges despite broaderpreparation scope; notREDREADY. Root preserved authoritativeallowed-source hashes,28testsourceinventory/plannedreplacementownership andactualthreeGREF IDs inmain target/mc-evidence/t12-builder/run43/root-verified-preparation-inventory.json. Correctedagent shorthand: registered UIplants are E2-browser-hardcoded-label,E9-browser-missing-button,E9-browser-debug-only-ui; positivepackage E9base/haul/cedar cases are separatepaneldependencies, notthoseplantIDs. No plant retired/testdeleted. C86 chooseslogicalscene widgetrects perexistingdesign. All15remaininglanes/finalacceptance/legacy0/CEOplaytests/measurements/all3CIgreen remain unfinished; fullgoalactive43/170. Thisturn madePROGRESS throughsource/proofchanges andverifiedpendingCI; no blockedcondition.


## C87: convert authored field of view from degrees to core radians once (2026-10-31)

Conflict: the Task11 preparatory test invented direct .84 CameraDef→Core FOV copying. All accepted authored interfaces use field_of_view60.0 (mods/base/interface/hud.ron12, muster14, acme9, hud1fixture14), while Core projection.rs65-82 accepts vertical_fov only inside(0,pi) and passes it to glam perspective. The design7 names the field but does not state its unit.

Choice: infer degrees for authored CameraDef.field_of_view from the existing accepted data and convert once with to_radians into CameraState.vertical_fov. Keep pitch/yaw in their existing radians and do not edit authored content. Correct preparatory native test to48degrees→48.to_radians; controls fixture .9rad becomes .9.to_degrees authored. Root preserved exact before/after source in t11-root/fov-unit-preparation-correction.json; compile and actualsemanticred remain NOTTRACED. Require actual accepted-interface/native projection and CAM1 proof later. Alternative not taken: copy60radians, alter accepted mod values, or bless the invented .84-unit fixture.

Root verified c2-repair14 actual0/1testpass after correctsamegroupyield/hardprojection physics, nothorizontuning; nofullTask6acceptance. C87 corrects cameraunitfixturebeforeproduction. RUN44 registered before Task12completed-agentfollowup forconcreteHUD1instruments, secondof8estimate,global44/170. No extraTask6reviewerfollowupstarted, cacheownerTask6.


## C88: carry C84 capture opt-in through the real launcher (2026-10-31)

Conflict: root C84 tests passed the runner directly, but scripts/mc-browser.sh114-130 reconstructs params.json from an allowlist and dropped capture_scene_checkpoints. Root previously inferred that the whole row reached the runner; source inspection proves that inference was wrong. CAM1 would have lacked stage data before any camera assertion.

Choice: Task11 may add the one boolean field to the actual launcher parameter generator, validate its type, and test that exact PYPARAMS block with native Python (no browser/compiler). Keep all existing viewport/windows/device-scale precedence. Top-level CAM1 row flag reaches runner params. Root committed8937c98 after actual generator-test red1→fullNode72green0, shellsyntax/driverchecks0; postcommit field-drop plant0/1/0 with exactrestore in launcher-final-receipt.json. Same10driver/fourview APIs; no game behavior or acceptance change. ActualCAM1 remainsNOTTRACED. Alternative not taken: fake checkpoint data, silently skip per-step assertions, or activate captures globally.

Thisturn progress: Task6simpackagefullsuite7 actual0, movement16/orders16/command12 green, GM40/GMAactualaudits+corruption andqueuedGather auditcontrols0; finalserver suite stillred atindependentmoving-gatherer accounting. Root preservedproductionsource and diagnosed double-step arrival (post-stepdistancecomparedagainstfullstepcausesextrasnap), instructedfixproduction withoutweakening accountingoracle. EarlierrealGMAoverlapalert counted same-group pairs; correctedcross-groupfilter perdesign, finalall-pairsrestchecksretained. F/Istrongall80stable2s/noMoveTarget/nooverlapgreen; weakerfi-repair2 remainsunacceptedattempt. Costphasecontext/gridcache andsmall-radius/finite-ring fixes active, fullgates/commit/frozen/GREF/independentreview stillpending.

Task7rootprepared30Hz exactfractionalrefilltests21/21/22envelopes and5461/5461/5462units across3ticks, plusactualscriptmotiontick10; sourceonly, no compiler/semanticred. Task11rootstrengthened nativecamera boundary/playeredge/keyrelease/focuslost/height/group-centretests andC87FOVunits, stilluncompiled. C88launcherparaminstrument8937c98 hasrealgeneratorred1,Node72green0,postcommit0/1/0exactrestore. Task12HUD1instrument8650131 verified13oraclecases/allNode75green,actualDPRwrongfactorred1→green0andpostcommitplant0/1/0exactrestore; allQA0. These areinstrument proofs, notnative/browsergameacceptance. Global44/170, no newagentdispatchthisrecord, no stopcondition/mainaction.


## C89: observe synthetic retarget cadence before collision resolution (2026-10-31)

Conflict: unit_boot.rs388-438 compared MoveTarget after Movement against the synthetic requested point. Task6 collision avoidance may legitimately choose a free navigation destination; synthetic.rs93-120 still assigns exact authored targets in FixedFirst. World MoveTarget describes where the unit is going, so disabling avoidance would conflict with design6.4/6.6.

Choice: preserve synthetic production code and test exact cadence at its real scheduler boundary. Read-only FixedFirst observers before retarget and after its deferred writes check all scheduled IDs against target_point(previousTick,id), every unscheduled ID unchanged across that system, all50 observed and retargeted at least twice; retain map-unit/no-extra-target and actualmovement checks. Ordinary fullappupdates remain. Root authorized tests/unit_boot.rs change; oldsource/failure preserved; new actualobserver green andfullserver suite0. Afterstablecommit/gates, RETARGET_TICKS40→39 must fail this new test semantically andrestoreexactly. Alternative not taken: accept arbitrary legal targets, disable collision forsynthetics, or invoke retarget twice manually.

## C90: qualify repeated template instance IDs in the scene widget instrument (2026-10-31)

Conflict: authored widget IDs are validated unique once per definition tree (content_mc_validation.rs914-919), but a Grid/List template such as HUD1's id b expands into multiple rendered instances. New scene rows require an unambiguous id/rect per rendered instance; publishing b repeatedly makes oracle identity ambiguous. The design specifies template expansion and renderedwidget IDs but no instance naming rule.

Choice: preserve authored IDs for roots (card/minimap/ore), and give repeated template instances deterministic qualified identifiers based on the widget path plus item identity (or authored stable index when no identity exists). Do not edit content IDs or input/action bindings. Keep internal action identity and lifecycle bound to actual data. This is an instrument naming detail only. HUD1 oracle rejects duplicate rendered IDs; later runtime/native tests must prove correct template identity. Alternative not taken: publish indistinguishable repeated IDs, rename mod definitions, or weaken the oracle to arbitrary first matches.

Task6candidate55522c9 followedbylint/refactorcandidateb4bb0eb0ff169e5756b535d76fb8898b21974582; bothattributed, finalb4clean. Sim/server fullfunctional andfmtgreens atrecordedactualsources; resetformal-gateheads forb4. ExtraClippy--all-features wasgeneric-skillflagbroaderthanplan andenabledoldspike_serverfielderrors; preserveasdiagnostic/non-gate, no spikeedits. Exactplanclippy--workspace--all-targets--locked passedrepairedtree, rerunonb4formalrequired. Rootenforcesplanoverskillrecommendation. RUN45registeredbeforefinalindependentreviewerfollowup, global45/170, Task6thirdof6estimate; nooverrun. AllC89postgateplant/GREF5/frozen/actualM3/independentruntime remain. No mainchange.


## C91: rename the movement distance key that collides with Forge content (2026-10-31)

Conflict: the real_source_scan_is_clean workspace gate at b4bb0eb rejected the helper-local identifier shell in movement.rs as protected Forge content. Engine names may change; protected mod content and the scanner may not.

Choice: rename the engine locals to ring_key and minimum_unseen_ring_key, including their tests. Commit d3ef732 changes only those names/formatting. The exact named scanner test now passes1/1, exit0 (no-content-renamed-green.log), after the recorded real workspace failure101. No content or scanner change. Alternative not taken: rename Forge content, add a scanner exception, or ignore the gate. Later semantic changes remain uncommitted and require fresh gates.

STOPPED awaiting CEO pool-capacity ruling.10of25lanes complete; Task6 validtiny-map semanticred101 conflictswithmandatory7slotpool/loneunitarrival. No fallbackselected.45/170runs; Task6run3/6. C1-C91. Mainc3f09d0 unchanged. Full goal incomplete; awaiting the separate CEO movement-policy ruling.
See [[2026-10-31 MC T6 pool-capacity stop]] for exact conflictingrules, compilednativeproof, independentgeometry and concrete recommendedpolicy NOTimplemented. Root stoppedallbuild/repair/dispatch afterconfirmation; onlyread-onlyhandoff/provenance/vault/report actions followed. Firststopaudit=1; goal staysactive, notprematurelyblocked. Completedagentresumptionmustcountnewruns BEFORE followup.


## Wave 4 terminal CI and speedup approval

The CEO confirmed the speedups. C39 remains the execution rule: focused checks during repairs, grouped required final checks on the stable candidate, no duplicate external Trunk build or convergence run, one compiler cache, and all required independent reviews, plants, per-merge gates and per-wave push/CI reads. This approval does not select a Task 6 pool-capacity policy.

Root read the terminal hosted jobs for run 41195343950 at integration code fb5e2d6: agents passed; check failed; browser failed. Check failed in M2 native-only semantic refutations. The named test e9_loss_and_small_buffer_converge panicked at crates/turts_server/tests/economy.rs:2173:21: automatic operation 6744: gather state differs from the independent full transition projection. The test reports 0 passed, 1 failed, 53 filtered, 830.86 seconds. The exact refutation phase and cause are NOT TRACED. This code predates Task 6. M3 native refutations and acceptance were skipped. Raw log: main target/mc-evidence/wave4-check-job.log, SHA256 0cd6d28ebc0e54ee40ac5b25d3ab6249d3a6a68769841181ba0ed5e2772a099b.

Browser retains the exact known M2_CASE_TIMEOUT E9-browser-base-economy failure; its cause is NOT TRACED. Later M3/browser refutations were skipped. No hosted-green claim. CI: https://github.com/26medias/TuRTS/actions/runs/41195343950.

Task 6 remains stopped under brief section 4 pending the separate CEO movement policy ruling. No builds, source repairs, agent dispatches, merge or push were performed. Budget stays 45/170; completed lanes stay 10/25. Main was fetched and remains c3f09d0. Disk has about 677 GiB free. No outputs were deleted. The full MC objective remains incomplete.


## O12: resume Task 6 (2026-10-31)

CEO ruling O12, main721e56ee020cbb60c23d2b99acd1fdfd7367d509, design revision13: all N primary slots are required. Fewer than N legal slots fails the envelope atomically with blocked for each unit; no unit moves. Spares are the remaining legal slots, zero or more, up to the existing maximum. Section6.4 is unchanged: with no spare, one second without progress becomes Blocked at once. The earlier pool-capacity stop is released.

The CEO clarified brief section4: when an existing mechanism cannot meet a design rule and the same player-visible shape is kept, Codex decides the mechanism conflict as C<n>, records evidence and the rejected alternative, and continues. Do not escalate the next such conflict merely because the design wording needs an applicability ruling. Frozen/K/Decided changes, changed player-visible shape and the other explicit stop conditions remain.

Integration rebased onto721e56e. The only conflict was the daily work log add/add; both main O12 and integration history were retained. All open lanes T6/T7/T11/T12 rebased onto the resulting integration8eff3c4. All14 dirty or untracked source files are byte-identical before/after; explicit-path stashes retained as backup. Receipt: main target/mc-evidence/o12-rebase/receipt.json. No new cache was created. RUN46 registered before Task6 Luna/high continuation;46/170 total, Task6run4/estimate6. Task6 alone has the compiler lease. Review/source preparation in other lanes remains idle. No main merge or push.


O12 implementation checkpoint: RUN46 reproduced the original compiled singleton red101 on rebased source, then group-movement-o12-green2 passed21/21, exit0. Root read the raw logs and current assertions. This includes the reachable singleton, partial-spare four-unit pool, and impossible three-unit/two-slot atomic per-unit Blocked/no-movement case. The first attempted green retained two invalid fixture assertions (N2 actually fits; PendingMovementOrders::is_empty also checks scheduler IDs); root corrected the diagnosis and required test-only fixes, preserving the failed log. Current singleton proves fewer spares, not zero spares. Exact zero-spare1s deadline, actual replay, retained source findings, final gates/review/plants remain incomplete. No Task6 PASS or merge.

Wave4 check phase is now TRACED: case E9-midtrace-rearm-ignored failed baseline.green before any plant (planted_sha256=null, baseline/restored source hashes equal). Root downloaded the32.8MB existing diagnostic artifact13042513668 and extracted only that case receipt/logs; no build or broad scratch extraction. main target/mc-evidence/wave4-native-trace/root-phase-audit.json records hashes. Cause remains NOT TRACED because no failing operation AuditState was retained in the artifact. Full acceptance must prove the same loss test green without weakening the independent accounting oracle.


RUN47 registered before independent Sol/high reviewer continuation. Source review may overlap RUN46 repairs without cache/build/browser use. Review must cite exact captured bytes, distinguish repaired-source from runtime proof, recheck H1-H15 and O12, then wait for a stable committed candidate and explicit compiler handoff for independent runtime/own plant. No acceptance PASS before those steps. Budget47/170, Task6run5/estimate6; no overrun.


## C92: prune dead pending members before spreading

Conflict: RUN47 checkpoint1, orders.rs SHA06d6f4c128435f8b35573f17173da4e3899320970ec174e1b478ac9b2c59e3d5, crates/turts_sim/src/orders.rs:688-723 filters absent entities, then calls spread_destinations_with_context before its empty cleanup. crates/turts_sim/src/movement.rs:194-197 requires nonempty input. A pending envelope whose members die can panic; a partial-death envelope also needs a defined live membership. Design6.3 requires atomic envelopes and live legal slots; ordinary unit removal must not leave a permanent pending head.

Choice: before allocation, prune only IDs absent from the live unit set in a captured subjectless Orders operation. Remove an empty head so later envelopes can proceed; preserve the key, arrival order and atomicity of surviving members. Compute N and cap consumption from survivors. Independent replay must prove every removed ID absent from before.raw_units, recompute exact queue/member counts, and require no other state or event changes. Add actual all-dead/partial-death scheduler and replay proofs plus rejection of forged live-member pruning. Use the existing capture mechanism with a narrow Orders wrapper; reselect G-REF for every actually changed path. This is a mechanism decision under the CEO's O12 clarification; it adds no key or visible interaction.

Alternative not taken: spread empty input, retain dead envelopes indefinitely, fail all live survivors because another member died, or exempt arbitrary pending mutations from replay. Source finding is confirmed; runtime repair and independent acceptance remain pending.


## C93: pending Replace keeps the prior current order, not its queue

Conflict: RUN47 H12 found that retaining the old queue through deferred Move activation lets old queued C run after an accepted Replace B. The first review interpretation also required old C to continue while B waits. Design4.5:425-429 explicitly says Replace replaces the current order and queue; design6.3:639 says waiting units keep their previous order (singular), not the queue. The reviewer withdrew the stronger old-queue-preservation interpretation after root cited the exact text. Current captured orders.rs:845-849 replaces current without distinguishing old C from a later Append D.

Choice: after admission and capacity checks succeed, clear the old queue under the normal Replace rule, while retaining the prior CURRENT order, Gather state and physical movement until B dispatches. Appends accepted after pending B use B's Move behavior as their effective head, enter the now-empty queue, stay behind B, and survive B activation. A capacity refusal changes none of the old state. A later successfully admitted Replace clears the intervening Appends in the same way. This needs no prefix metadata and changes no key, failure reason or accepted Replace/Append behavior. Codex decides this mechanism applicability under O12/brief4.

Require actual scheduler and independent replay tests for refusal preserving old state; successful admission clearing C but retaining the old current/Gather; and later Append D accepted behind pending Move despite retained Gather, surviving dispatch and starting only after B completes or fails. H12 remains open until repair and independent recheck.

Alternative not taken: preserve canceled old queued C, clear all queues at dispatch and lose later D, advance D before B, or add queue-origin metadata for a prior-queue requirement the design does not impose.


## C94: initialize the dispatch cursor lazily for pristine legacy worlds

Conflict: root and RUN47 checkpoint1 confirm crates/turts_sim/src/orders.rs:480-507 changes MovementDispatchState and pending.next_slot on every idle tick, including worlds with no cohort envelope ever admitted. crates/turts_sim/src/economy/views.rs:1099-1105 clones both fields, while serde rules at378/385 require exact defaults for omission; PendingMovementOrders::is_empty atorders.rs:62-67 includes next_slot==0. Plain legacy audited states therefore gain new fields, contrary to design6.1 default-field/plain-path preservation. The frozen stream test projects operation counts/event kinds/before-equals-after and may not detect this raw-shape drift; no actual frozen fixture change is claimed.

Choice: keep pristine pre-cohort resources at their defaults so legacy raw AuditState serialization stays unchanged. Derive the first real cohort's turn lazily from the simulation clock, verifying the exact Tick phase; after initialization keep normal audited round-robin rotation on every tick, including idle ticks. This preserves the design's scheduling and plain-order rules through an internal mechanism change. Add actual pristine-world serialization after multiple idle ticks and first-cohort-after-idle scheduling regressions, then independent accounting/frozen checks. Never hide active pending fields, reset pool IDs, change the frozen fixture or omit runtime validation.

Alternative not taken: serialize new fields in all legacy worlds, stop idle rotation and favor a fixed first slot, or discard meaningful active scheduler state from audit. The implementation is repaired under Codex mechanism authority; no K/Decided/frozen value changes.


RUN46/RUN47 checkpoint: root verified raw O12 group22/0 and movement12/0; actual atomic-failure audit1/0 rejects forged missing failure/partial move. C92 all/partial dead-member scheduler+replay test exits0, with rotated-slot follow-up under repair. C93 first actual command/dispatch trace exits0 with named queue corruptions, but its synthetic Gather and no-wait setup do not prove a live gatherer during a cap-induced wait or later Append execution; those remain NOT TRACED until the stronger fixture runs. C94 pristine audit-state repair and H2 occupied-spare cost repair are active. Independent reviewer remains source-only until stable commit/full gates and explicit cache handoff. No Task6 PASS/merge.

Root recomputed provisional G-REF from actual changed paths after C92 added the transaction capture wrapper:14 native cases (12 M2,2 M3), not the previous5. All sequential plant anchors are unique in current working source. Receipts: main target/mc-evidence/t6-root/o12-working-gref-selector.json and o12-working-gref-anchor-preflight.json. This is source preflight only, not runtime refutation proof; recompute on final committed bytes. The first preflight missed the registry replacements-array shape and failed setup with KeyError; corrected preflight handles sequential replacements without changing registry/runner.

Disk correction: a RUN46 test omitted target/profile exports and built an unintended lane target/debug. Its raw stdout was not retained; builder reports exit0, which is not accepted as a verified gate. Root confirmed no /proc cwd/exe/maps references and acquired its Cargo lock, saved command/time/file-count/byte metadata, then removed only1100 inactive rebuildable files (957.6MiB allocated). Sole target/mc-t6 remains intact; no source or durable evidence removed. Receipt: main target/mc-evidence/t6-root/accidental-debug-cleanup.json. Earlier temporary downloaded native artifact ZIP was also removed after retaining the selected case receipt/logs and provenance. Available disk remains about677GiB. Budget47/170, Task6run5/6.


Root verified fresh RUN46 functional checkpoints: group-movement-full-c94.log/.exit has23passed/0failed, exit0; turts-sim-lib.log/.exit has176passed/0failed, exit0. Fairness fixture now uses7 genuine idle ticks with default scheduler state, then B atTick8, A atTick9 and B atTick10. No synthetic orphan cursor is needed.

H2 proof was strengthened after root/reviewer rejected invalid input and a misleading allowed-old-slot shortcut. Current canonical mixed-radius fixture has64 members,80 legal production slots, target==progress.slot, radii0.2/0.3/0.5, nonoverlapping starts/path blockers, physically free primaries, and16 spares occupied by other-group resting units. Legal speed0.05 makes the real one-second stall branch execute. clearance-canonical-mixed.log/.exit reports every commanded mover Blocked and0 spare-search candidate inspections, exit0; pre-run source SHA056620a8d55e0e7be703ef038197b7c0851037ce76573ae59f175cc4c322cd7c matches root-read movement.rs. This is a builder functional proof, not independent performance acceptance or Task22 completion. Invalid historical fixtures are not accepted as production-state evidence.

RUN47 prepared independent canonical1000/5000 cost probes and a real Gather/cap-wait/Append-after-arrival lifecycle probe. These remain uncompiled/unrun until the stable-candidate cache handoff. The lifecycle uses a real2049-unit busy head with audit disabled to avoid an O(N squared) full-state trace; separate small audit regressions cover replay. Server functional checks, full stable gates,14selected GREF cases, required postgate plants and independent final review remain incomplete. Budget47/170; no Task6 merge or new CEO stop.


## C95: queued groups use the design merge key and first unit-id start

Conflict: RUN47 orders.rs SHAdc1865932d666872df345c42d2e779010ac36b2632a94598d59f5e44cc86b46a, lines533-587, groups queued moves by authored point bits before merging. Design6.3 merges (slot, positive group, tick) regardless of point. The BTreeMap point order can choose a different P from the actual unit-id-ordered queued starts. Mixed radii also need one group-max inset point.

Choice: group queued starts by (slot, group, unique unit id only for group0), sort members by unit id, use the first member's authored P and clamp once by group maximum radius. This preserves the design merge key and the existing unit-id phase order. Actual body targets stay inset; retaining authored point until grouping is an internal instrument detail. Add different-P same-key and mixed-radius queued-point proofs. Alternative not taken: split one positive group by point, let float-bit sort choose P, or merge group0 envelopes.

## C96: fail queued activation atomically when pending capacity is full

Conflict: RUN47 orders.rs:594-610 admits the first member's queued envelope, but marks later members queued even if the first insertion failed at the16384 cap. Some members lose targets with no envelope; another can move through the plain path without spreading. The plan does not name this queued-activation failure mechanism. Design6.3 uses per-unit rate_limited on pending-cap refusal; design4.5 starts the next queued order in the same Orders phase after failure.

Choice: preflight the whole activated queued envelope. If it cannot enter the pending limit, fail every live member atomically using the existing OrderFailure with this order, RateLimited and current tick. No member moves, no partial insertion, and no unregistered straight-line target reaches Movement. Clear the failed current target/progress, preserve future queued work and apply normal same-phase continuation. Every next ready Move must also be atomically admitted or failed before Movement; use bounded queue-depth work rather than unbounded recursion. This differs from refusal of a NEW command, which must preserve its prior current/queue state. A queued order may fail at activation through the existing failure/continuation shape, as TargetGone already does.

Require real cap-pressure group regression, later-member and next-queued continuity checks, independent replay and rejection of forged partial failure/movement. Alternative not taken: partial admission, an unspread target, exceed the pending cap, or add a new pause/retry state. This is a Codex mechanism ruling under the CEO's O12 clarification; no new key, failure kind, frozen value or acceptance cut. H8 stays open until repair and independent recheck.


## C97: retain each member's accepted order identity in a merged pool

Conflict: RUN47 H13/C49 found PendingMovementOrder holds one order while same(slot, positive group, tick) merge appends unit IDs. Dispatch copies the first order to all members. Two legal Move definitions with disjoint allowed unit types can therefore share the required K4 physical pool but lose the accepted per-unit order/failure identity. Design6.3 requires the merge key; catalog authority and per-unit orders require the accepted definition to remain each member's own.

Choice: keep the physical key and first P, and retain per-member accepted Order metadata in the audited pending envelope (a default/skip-empty map is the minimal compatible representation). Both direct commands and queued grouping must populate actual per-unit orders. Merge, latest accepted replacement, cancellation, dead-member pruning, dispatch and failures preserve the actor's identity and exact metadata. Plain legacy envelopes with no order metadata still create no UnitOrders state. Independent replay models the map transitions; add two distinct legal Move IDs/allowed types sharing one pool and verify per-unit current/failure identities.

Alternative not taken: add order ID to the merge key, overwrite later members with the first definition, derive identity from an unrelated current order after deferred replacement, or omit new active metadata from audit. This is a Codex data/mechanism ruling under brief4/O12, with no new key, failure kind, frozen value or acceptance change. H13 stays open until repair/recheck.

Functional checkpoint before C97: server-bin-green2 has71/71, exit0; C95/C96 follow-up sim library177/177 and group movement23/23 are builder-reported fresh results pending final-source gate. These do not close later source findings or prove full Task6 acceptance. Run count stays47/170.


## C98: give stalled plain movement the full generic lifecycle

Conflict: RUN47 H9/H14 and actual compiled plain-post-red.log/.exit101 show a progress=None singleton can keep Moving forever. Valid7x5 map: radius0.5 worker at(0.5,2.5), radius2 post at(3.5,2.5), target(6.5,2.5); after200 ticks it retains its target. Design6.1:582-583 explicitly covers stalled plain movers/gatherers, and6.5/GMU require bounded fallback/Blocked. A temporary full-width-post detector passed only that geometry and was rejected and removed: it did not implement the required generic lifecycle and gave an immediate failure instead of the specified stall timing. Its green is superseded diagnostic evidence only.

Choice: add independent per-unit PlainMoveProgress with original point, current slot, a bounded own N1 pool of legal primary/spares, best distance, stall ticks, claim count, activity and failure-recorded marker. Carry optional state through the shared pure MovementFrame input/output and sorted default-omitted audit rows. Instantiate only on actual obstruction/retarget/no-progress, preserving original P before avoidance changes it. Reuse phase obstacle indexes; never share a sentinel group/pool ID across plain units. Apply the same1s no-progress, no-spare failure and at-most3 claims as6.3-6.5. A claimed spare is the new final slot; reaching it rests there, with no automatic return to the original goal. Unobstructed legacy arithmetic, omitted fields and operation counts stay exact.

Orders alone captures a Blocked failure acknowledgment once, Gather interruption where needed, and normal queue continuation. The kernel writes no UnitOrders or order operation. A legacy command with no authored Order ID remains idless: expose Blocked from its own progress and audit its acknowledgment; never invent a ContentId. If an authored identity exists, use the actual accepted order for OrderFailure. Store current authored plain identity in a separate Orders-only transient cache, not UnitOrders/kernel input; replay reconstructs it from validated CommandBoundary history. Narrow independent-oracle context support may model this new accepted-input history without weakening full-state checks. Pending replacements retain the old execution identity until activation; Appends do not replace it early; new active orders/removal clear old plain state/cache.

Require generic static and mobile obstruction/own-pool tests, stalled Gather coverage, authored correct/wrong-ID failure proofs, idless acknowledgment exactly once, new-order cleanup, exact plain legacy serialization/arithmetic/frozen checks and actual replay. Alternative not taken: special-case the full-width post, use unbounded waiting, add routes, return from a claimed spare to the old goal, assign unrelated movers one sentinel group, fabricate a content order ID, or serialize new fields into every free legacy move. This is a Codex mechanism/data ruling under brief4/O12, preserving the design's player-visible movement and failure shape. No CEO stop is needed.

Verified functional checkpoints before C98: C96 vacancy1/group2 sim and validator-span tests exit0; the span rejects missing/reordered members. C97 follow-up sim179/179 and server73/73 are builder-reported fresh results awaiting final-source verification. The plain-post generic regression remains intentionally red; Task6 is not accepted.47/170runs, Task6run5/6, no merge/push/main change.


## C99: let Gather reach checks resolve completed plain fallback

Conflict: RUN47 found Gather's approach setter overwrites a committed C98 slot every Gather phase, and movement arrival can erase the claim budget. An unreachable Gather can then restart forever. Keeping a completed fallback Resting without resolving the actual Gather target also leaves the order active forever. Design6.1 requires stalled Gather coverage;6.5 requires bounded visible failure; M2 Gather still owns interaction reach and semantic endpoint selection.

Choice: the existing real Gather reach check remains authoritative. Same-goal approach refresh must not overwrite an active committed plain fallback slot or reset its timer/claims. Retain the completed fallback result. In the next Orders phase after Gather checked reach, success at Gathering/Unloading consumes the metadata; if the unit is still moving to the node/receiver and outside actual reach, capture the existing Blocked failure once, interrupt Gather and apply normal queued continuation. Successful arrival may also consume metadata in its existing captured Gather stage transition when independent replay proves the actual reach/stage change. Movement writes no order operation. Free legacy paths with no fallback keep their shape.

An optional default-omitted semantic goal key (endpoint kind and node/receiver ID) may live in active PlainMoveProgress. The same key preserves fallback through approach refresh; a real M2 endpoint/stage change clears or reseeds it. Preserve NoReceiver, NodeDepleted and carry/accounting behavior. Replay must independently prove actor-only goal-key/metadata changes from the actual transition, never broadly ignore them.

Require actual inaccessible-Gather and reachable fallback tests, node/receiver changes, unchanged approach refresh, one captured failure and queue continuation. Alternative not taken: reset the budget every frame, return automatically to the original approach, leave an active Gather resting out of range forever, add routes, or make the kernel own economy reach rules. This is a delegated mechanism ruling with the same keys, interaction, timing bounds and existing failure reason.

C98 integration repairs remain mandatory: pending B preserves old execution A until activation; B success OR atomic failure clears old plain progress/identity. Common removal/death clears the transient identity entry. An independent legacy scalar AccountingOracle does not by itself prove new obstruction paths; K19(a) uses shared-step economylog replay there. Preserve all existing independent legacy full-state checks and label unexecuted secondary-model support NOT TRACED rather than replace that oracle with production code.


## C100: apply O12 to an initial plain N1 pool with no legal primary

Conflict: RUN47 found make_plain_progress used original P when its N1 pool was empty. Valid1.5x1.5 map: radius0.5 worker at(0.5,0.5), radius0.2 post at(1.2,1.2), P(1,1). Initial discs are legal and separate, all nonzero hex offsets leave the radius-inset map, and P lies inside the post exclusion. There is no legal primary; moving toward P would evade O12.

Choice: initial canonical N1 allocation with zero legal primaries uses existing Blocked atomically before any body step. It never invents an illegal slot or waits while moving. A valid primary with zero spares still uses the normal1s no-progress deadline. A previously allocated pool becoming obstructed uses its normal stall/claim rules. C99 Gather reach authority remains: when already inside actual interaction range no motion is needed, so a real successful Gather transition may consume an unacknowledged motion fallback instead of falsely failing Gather. Keep authored/idless identity and captured acknowledgment rules.

Root read c100-zero-primary/test.log/.exit: the exact valid geometry passes1/1, exit0, proving original position/no target on initial failure. Final whole-task acceptance remains pending. Alternative not taken: fall back to illegal P, add a special geometry exception, change the no-spare deadline, or fail a Gather that already reaches its real endpoint. This is a Codex applicability/mechanism ruling under O12/brief4, with no new key, failure reason or frozen change.

## Current Task6 checkpoint and gate status

Task6 source checkpoint a46b7c51a68c4893e78555a08b050deadacb049b is committed with the Codex trailer and11 explicit source paths. Root verified the Git commit/tree list. Its title/body explicitly say checkpoint, not acceptance. Later H3 pending-membership fast-path work and temporary restored diagnostics may remain dirty.

The workspace run failed101: economy_accounting41/42, e6_gather_conversion_and_interruptions_conserve_through_delivery fails for gatherer3 tick336 at the independent fixed-step boundary. Exact actor/source cause is not yet traced. Preserve the oracle; repair production after capturing actual before/after/predicted differences. No gate PASS or merge follows from earlier sim184/group25/server75 checkpoints.

Root authorized conditional compiler handoff to the already-active RUN47 only after builder explicitly releases a stable clean committed candidate with every required Task6 gate and selected refutation/plant restored-green receipt. Reviewer independently verifies HEAD/bytes/exits first; missing/failed/unknown prerequisites retain source-only mode. All final G-REF selection is based on actual ancestor8eff3c41537440aa49634339ed040906fd446984...HEAD, never old-fb5 triple-dot after rebase. Current selector is20native cases(18M2+2M3), all current sequential anchors unique; recompute on final source. Root still audits final review and re-gates before any integration merge.47/170runs, Task6run5/6. No main merge/push or scope cut.


## C101: preserve exact no-contact legacy arrival before applying tolerance

Conflict: actual independent accounting red for actor3 tick336 showed an ordinary no-contact move snapping after its step. Before(22.25925,9.274589), target(22.386324,9.181545), distanceabout0.1575, step0.15. The exact legacy next position is(22.380276,9.185973), with target retained. The post-step0.025 tolerance instead snapped to target and cleared it early. Design6.1/K19 preserves the default plain path;6.5's general arrival rule must not introduce an extra post-step snap there.

Choice: preserve original M1 step position/reached semantics for an unobstructed, nonprojected plain step. Apply the tolerance only in the appropriate new avoidance/fallback path, without a second legacy step or coordinate rounding. Root read compiled plain-step-red101 and plain-step-green0 with the exact coordinates. The original independent economy_accounting suite is now42/42, exit0; git diff8eff against current crates/turts_pack/tests/common/accounting.rs exits0 after the temporary diagnostic was restored. No oracle assertion, projection or tolerance was weakened.

Alternative not taken: accept one-tick-early motion, round the oracle coordinates, relax its equality, or change the frozen fixture. This is a Codex implementation/applicability ruling preserving the chosen K19/plain behavior. The hosted E9-midtrace-rearm-ignored baseline is still a separate process case: the same cause is only an inference until the exact ignored e9_loss_and_small_buffer_converge test passes in G-ACC.

Additional fresh evidence: C100 body-free/reservation-illegal target has compiled red then green in reserved-point-green; new legacy Gather accepted-state reset has a real sim/replay regression. Current sim library188/188 is builder-reported. C99 Gather replay now covers authored/idless failures, actual endpoint range, stable Blocked visibility, and authored-completion→new idless Gather without stale identity; root/reviewer are auditing exact receipt/source linkage. No Task6 acceptance or integration merge yet;47/170runs, Task6run5/6.


## C102: accepted idless Gather replaces prior current and queue

Conflict: after the C98 legacy Gather motion reset, prior UnitOrders.current and queued work still remained. A valid old cohort Move A with queued Hold C could cause C to start over a newly accepted legacy Gather. The legacy command has no queue modifier; treating it differently from the normal replacement rule would leave stale work active and incorrect exported status.

Choice: accepted legacy CommandKind::Gather uses the nearest design4.5 Replace rule: after all validation succeeds, clear prior current/queue/history, pending membership, target, group/plain progress and transient move identity; assign normal M2 Gather state without inventing an authored ID. Capture and independently model the nondefault order-state reset through the proper existing command/order span. Already-default free legacy input adds no extra order operation or fields. A refused Gather preserves every prior order, queue and motion field. Add valid Move A+queued Hold C interruption and refused-input preservation controls; exported status comes from real active Gather when no authored current exists.

Alternative not taken: let old C override the new accepted Gather, preserve an obsolete current Move, fabricate a content order ID, or clear old state before validation. This is a Codex legacy/new-mechanism applicability ruling under brief4, with the existing Gather interaction and failure rules. Source/runtime recheck remains required.

C101 further applies the pre-step arrival decision to retained plain and ordinary group progress: initial distance0.17 with step0.15 cannot take a step then snap an extra0.02. Normal tolerance eligibility is evaluated before movement; Blocked escape keeps its distinct at-most-one-step bound. Focused regressions and the source repair are being batched with C102/lints before the final gate head.

Current candidate c6e328865b9ff2e451ea28f3a68fde3e078c6282 is committed and reviewable, but is not accepted. First exact formal fmt passed; Clippy failed101 with13 Task6 lint findings, so no later gate was counted. Builder batches these repairs and C102 before recommitting. The earlier workspace fractional-timestep fixture used invalid radius0.5 start x0.01; it now starts at legal inset x0.5 while retaining the exact (0.5+3*dt).to_bits assertion. Focused proof is green; full final-source workspace still pending.

RUN47's current source matrix distinguishes confirmed source repair from independent runtime. New critical C96/C97/C99 behavior needs a valid compiled prefeature red or a postgate semantic plant plus restored green/clean bytes; root made this part of the conditional cache lease, alongside C89 and generic plain-lifecycle plants. Final G-REF remains actual-selector-driven(current20native cases), not a fixed historical count.47/170runs, Task6run5/6, no integration merge or main push.


## C103: batch gate groups on one immutable source head

Conflict: Task6 step3 lists ACC and WEB before FROZEN, while the existing native helper batches RUST, QA and FROZEN. Repeating the same passed frozen run solely to match list order adds no evidence. Brief4 delegates task order and cache handling; the CEO approved grouped checks without reduced quality.

Choice: batch independent RUST, QA and FROZEN before long ACC/WEB on the same clean committed source head. Every listed gate remains required before G-REF, postgate plants, reviewer runtime handoff or merge. A source repair requires final-source re-gating; an earlier candidate's green is historical evidence. Preserve red, green, commit, gate, plants. Evidence: plan Task6 step3 at505; brief4 gate/ordering rules at39-56. Alternative not taken: omit a gate, accept a dirty candidate, or rerun an unchanged gate solely for list order.

Verified checkpoint95d5d07: native helper final-rust-qa-frozen-4 has10 commands exit0, workspace1140 passed/0 failed/18 ignored; frozen fixture hash bcbf72c2206251f635a7c7f5b6c8f573d69a5fc05a5bd77716e98ebfb08e370b. G-ACC then failed at E8 operation167: authored singleton target65 on width64 maps to physical63.2 for radius0.8, while the legacy adapter expects64. Correct only the exact singleton Move/Replace/group0/Point adapter from independently audited radius/bounds; keep raw frozen input checks, full-state equality and strict default order metadata. No oracle tolerance relaxation. Task6 is not accepted.47/170 runs, Task6run5/6; no merge/push.


Task6 candidate cc6f91e77ffc72dd7df6089694815033a64a2fb0 commits only the E8 adapter and strict validator correction. Root verified clean source, unchanged independent common/accounting.rs and exact frozen SHA. Focused E8 process1/1 exit0; validator positive/forged-current/foreign-failure regression1/1 exit0. Root receipt: main target/mc-evidence/t6-root/e8-cc6f91e-root-check.json. Formal final-head gates running; no acceptance or merge. Ledger47 consecutive unique rows, no missing run.


## C104: use clear ground inside turret range in the F11 process fixture

Conflict: final-source G-ACC on cc6f91e failed at F11's 230-second final-health wait. Root read the retained reports: both clients welcomed, research completed at tick1361, opponent Move accepted/applied at1363, drone9 lost all health, drone10 kept100 health and rested at(15.05,2.874167). This was not a startup failure. F11 sends Move(17,4) at crates/turts_server/tests/m3_process.rs:301-302, exactly the lab centre at mods/forge/maps/proving_ground.ron:12. The lab radius1.2 plus drone radius0.6 makes that ground target obstructed. The new legal fallback is outside turret range8 at(17,12). The old fixture relied on movement through a static body.

Choice: change only this native process fixture's ground target to(20,6). It is within the existing radius-inset32x16 map and clear of the lab, press and arm loader; the segment from the drone's initial(30,4) is clear. Distance to turret is sqrt45, about6.708, below range8. Builder independently checked the geometry. Keep actual research completion before release, accepted move after research, private research projection, both clients' exact health steps, ten10-damage shots at drone9 and five20-damage shots at drone10, and the same230-second deadline. Actual timing and focused green remain NOT TRACED until the run finishes. Forge files, frozen fixture, design and gameplay rules do not change.

Evidence: retained failed acceptance at lane target/m3-evidence/t16-builder/acceptance/20261031T160856Z/f11-evidence; root selected report hashes/state in main target/mc-evidence/t6-root/f11-cc6f91e-failure-classification.json. Turret range: mods/forge/units/turret.ron:11; drone radius: mods/forge/units/drone.ron:4; exact assertions remain m3_process.rs:355-398. This is a test-fixture/mechanism applicability ruling under brief4:49-51. Alternative not taken: remove collision handling, change Forge content or its frozen values, widen the deadline, accept missing damage, or retry the same unreachable predicate. No CEO stop applies.

All ten native gates passed on cc6f91e; local exact E9loss passed1/1 after184.31s and E10 passed. Full G-ACC is exit1, not a pass. Required next steps: focused F11 green, commit the fixture correction, final-source gates, selected G-REF and postgate plants, then independent runtime review.47/170 runs; Task6run5/6. No merge/push.


C104 focused verification: root read f11-c104-green/test.log and exit.txt0: exact F11 process1/1 passed in81.12s. The only fixture diff17,4→20,6 is committed as ffb21b326f6b87e89d61997c7d69822ca216758a with the Codex trailer; source is clean. This replaces the earlier timing NOT TRACED status for this local focused run. All exact research/order/privacy/damage/health/shot assertions passed. Native final-head gates restarted on ffb21b3; Task6 acceptance still pending.47/170runs; no merge/push.


## C105: check independent native behavior before the long refutation batch

Conflict: the earlier Task6 cache handoff waits for all20 selected G-REF cases and builder plants before any independent runtime. A retained behavior/cost defect found then would require those long checks again. The CEO approved speedups with coverage kept; C39 already permits builder/reviewer cooperation with one compiler owner, and Task9 used an early independent native check before G-REF.

Choice: after all final normal gates pass on a clean committed head (RUST, QA, FROZEN, ACC, T7 and all seven M3 browser controls), the builder may explicitly release the sole cache for a preliminary independent native functional/cost check. RUN47 confirmed its six prepared external harnesses need no production, test or registry source mutation and use the same absolute lane target. Compiled library/API setup errors are not semantic reds. Preserve exact source hashes and selected branch-hit/geometry guards. No reviewer own plant, acceptance PASS or merge occurs in this preliminary phase.

After the probes, the reviewer explicitly releases the quiescent cache back to the builder. Every selected G-REF case and required postgate plant still runs, with semantic red/restored green and clean source. The reviewer then performs its own plant and final recheck under a new explicit lease. Final acceptance still needs all existing prerequisites; source repairs require the final-source normal gates again. This changes only the timing of an unmutated-source independent check, not gate coverage or red/green/commit/gate/plant order.

Evidence: brief4:42-43 and52, C39 in this note, Task9 early independent runtime in C81 (central rulings note:672); RUN47's source-only confirmation names native_cost, c93_lifecycle, plain_fallback, c99_gather, plain_escape and retained_progress_step harnesses. Alternative not taken: omit/refit registered refutations, mutate source before gates, share the cache concurrently, or claim an early PASS. No new agent run;47/170, Task6run5/6. Current ffb21b3 normal ACC is still running, so no lease has transferred.


Final normal-gate checkpoint: ffb21b326f6b87e89d61997c7d69822ca216758a is clean. Root verified all ten native commands exit0, workspace107 groups/1142 passed/0 failed/18 ignored, frozen exact. Full G-ACC also exit0: eight passing summaries/16 tests, E9 loss184.28s, corrected F11 81.01s, F11 retention passed. Root receipts: main target/mc-evidence/t6-root/native-ffb21b3-root-check.json and acc-ffb21b3-root-check.json. T7/M3 browser gates are pending.

The web helper accidentally repeated native checks on the same source. Builder identified its sole process, stopped only that invocation with SIGINT/exit130, confirmed no children remained, and recorded aborted redundant setup. It is not a web pass or semantic failure. Direct unchanged T7 and M3 scripts now run with the approved Vulkan environment and sole cache. No source change; completed native/ACC proofs remain valid.

RUN47 flagged an authored singleton cap edge case for its C105 preliminary probe. D6.3 has no authored Move-envelope exemption from the global cap; preserve legacy idless MoveTo separately. The source lead is not yet a confirmed runtime finding. Do not change source or claim a failure until the valid probe runs. Every selected refutation and final plant remains required.47/170 runs; Task6run5/6; no merge/push.


C105 preliminary handoff: all normal gates are now green on clean ffb21b3. Root verified T7 exit0,136 frames/53 moving and exact final(20,8), with its segment oracle1/1; M3 exit0 and cleanup0, all seven canvas/full capture pairs and accepted produce/disable/remove results. The first T7 attempt exited1 before browser assertions because playwright-core was absent; locked offline npm ci repaired setup, then unchanged-source retry passed. Both attempts remain recorded. Root web receipt: main target/mc-evidence/t6-root/web-ffb21b3-root-check.json.

Builder explicitly released the quiescent sole cache to RUN47. Reviewer verifies prerequisites before preliminary unmutated-source native/cost probes, prioritizing the authored singleton cap control. No production/test/registry mutation, own plant or acceptance PASS is authorized in this phase. All20 selected G-REF cases, required builder plants and reviewer final own plant remain pending.47/170 runs; Task6run5/6; no merge/push.


RUN47 preliminary H4 High confirmed on immutable ffb21b3: the authored singleton Move/Replace/group0/Point shortcut bypasses the shared work cap. The compiled external probe exits101 after a passing no-competition control. Competing Tick1 proves actual head progress2049 and MovementDispatchState.used2049; the next slot's accepted singleton still moves(8,8)→(8.15,8), activityMoving, Pending=false. Expected: whole-envelope Pending and exact unchanged body until next tick. Root read raw output/result/exit and matched current orders.rs SHA56e4b3d46b71c9ce45d73f13cffe22ed078136c0cda52ead3503720e63795e61. Raw SHA b70b1504574ae6ba84711e6b0585c0f09e989ae24f92ccf183b1be8631c37754; evidence main target/mc-evidence/t6-review/run47/preliminary-runtime/authored-singleton-cap-verified-budget/.

No authored-envelope exemption exists in D6.3. A repair must also preserve D6.1's plain one-command/no-extra-order-operation rule for uncontended moves and exact legacy idless/frozen behavior. Builder prepares a read-only mechanism proposal while reviewer batches remaining unmutated probes; source and cache stay leased to reviewer until explicit release. No source edit, plant or acceptance PASS yet. This is a repair requirement within Codex rights, not a CEO stop.47/170runs; Task6run5/6; no merge/push.


## C106: arbitrate authored plain moves with every movement envelope

Conflict: RUN47's compiled H4 counterexample on ffb21b3 proves a 2,049-unit head consumes the tick budget while a new singleton in the next slot moves 0.15 instead of waiting. The no-competition control passes. The shortcut at orders.rs:1633 and command.rs:188 does not enter the shared scheduler. Design6.3:629-645 gives no authored-singleton exemption; design6.1:565-571 also keeps the plain command-only audit path.

Choice: use a small transient reservation for each validated authored singleton Move/Replace/group0/Point until its next Orders arbitration. Keep accepted identity, slot and arrival order, plus only actor-local prior execution needed if it waits; never clone a world or AuditState per unit. Count reservations in the 16,384-unit admission limit before accepting input, with same-unit replacement/cancellation accounted for. Refusal preserves old state. A later accepted Replace is last-wins over unactivated reservations and keeps the original executing state; an accepted non-Move cancels the candidate. Append sees the candidate as its effective pending Move, retains the new suffix and cannot execute before it. Accepted Replace clears old future work as C93 requires.

The scheduling view merges reservations with real pending queues in slot/arrival order before the first dispatch. It must work for either command arrival order. Select against the exact common cap, round-robin turn and oversized-first-head rule. Direct winners count once but keep their existing command-only/plain state and arithmetic. Promote actual waiters through the normal captured Pending path, restoring previous current/Gather/physical motion and keeping only post-replacement Append work. Restore waiters before building spreading reservations, so an ungranted provisional target does not reserve ground. Early admission reservation prevents a late hidden refusal after accepted. Carry used budget through both dispatch passes; arbitrate newly ready work against that same budget. Do not discard direct-API commands merely because their accepted tick precedes the next Orders tick.

Rebuild transient reservation/history independently from validated accepted inputs and command boundaries, as C98 does for identity. Validate exact capacity, prior-state restoration, queue suffix, selected/waiting membership, used budget and cursor; no production flag or after-state is an oracle. Legacy idless MoveTo stays outside this new cache and its frozen stream stays exact. Require both command permutations, waiting/prior-Gather/Append, capacity/refusal/cancel controls, and no extra order operations for an ordinary granted plain move. Alternatives not taken: exempt authored singletons, check only earlier pending commands, let an unactivated reservation become prior execution, add order operations to every free plain grant, or weaken frozen/oracle checks. This is a mechanism ruling under brief4, with the same player-visible Pending/cap rules.

Evidence: main target/mc-evidence/t6-review/run47/preliminary-runtime/authored-singleton-cap-verified-budget/{result.json,output.log}; compile0/run101; raw SHA b70b1504574ae6ba84711e6b0585c0f09e989ae24f92ccf183b1be8631c37754. Runtime repair is not yet traced.

## C107: keep physical escape alive through order continuation

Conflict: on a valid7x5 map, an authored blocked move with queued Hold ends at(2.2790802,0.5), inside a radius2 post at(3.5,2.5): distance2.3432126 is less than radii sum2.5. The failure is acknowledged at tick147, but Hold cleanup removes all plain progress. The idless control escapes and rests clear. Design6.5:672-674 requires a blocked unit to leave every overlap before rest; start_order Hold at orders.rs:1424 clears the data that still drives escape.

Choice: make the unfinished physical escape an actor-local, default-omitted physical state distinct from current order/queue state. The pure movement result records it at the first real Blocked outcome with actual body/static penetration, before a new command can erase order progress. It survives failure acknowledgment, queued continuation, and newly accepted Hold/Stop/replacement cleanup. Movement uses the existing local escape mechanism at no more than one speed-times-delta step per tick, with no second normal step or order write. Keep the current/new order and destination intact while escape has priority; clear the physical obligation only after penetration is gone or the actor is removed. Queue continuation still occurs at its normal time.

Only an actual blocked mover's residual penetration creates this state. Reservation-only conflicts and ordinary idle/Hold units do not gain an escape obligation or move for others. Preserve exact failure identity and one acknowledgment. Carry every physical input through MovementFrame/AuditState with defaults omitted and strict replay. Require plain and cohort controls, queued/new Hold including input before failure acknowledgment, one-step bounds, clear-and-stable rest, and unchanged free/frozen data. Alternatives not taken: leave the body penetrating, delay the queue until escape ends, move every holding body, retain a stale order goal as the physical destination, or make Movement write order operations. This is a data/lifecycle mechanism ruling under brief4 preserving6.5 and queue behavior.

Evidence: main target/mc-evidence/t6-review/run47/preliminary-runtime/plain-fallback-probe-attempt2/{result.json,output.log}; compile0/run101; raw SHA bf97d48649d2d7f39959e55b06bd4c601a9b8f2fca73964286642c6627abd0c0. The initial world is legal; the penetration is produced by actual movement, not an invalid test start. Runtime repair is not yet traced.

## C108: complete a unit-target Move at its reached legal slot

Conflict: a valid Move(Unit) plus queued Hold reaches a legal fallback slot at tick37 but at tick300 retains the Move, untouched Hold, no target and Resting progress, exported Pending. The point-target control completes. run_unit_follow_move at orders.rs:1281-1354 tests equality with the other body's centre and refreshes that impossible destination. Design5.3:528 says Move completes when it rests on its slot.

Choice: consume a real reached/Resting fallback for the same current Move when its still-live target has the same canonical radius-inset goal. Complete through the existing captured order transition and same-phase queue continuation. Do not reopen an unchanged completed fallback every tick or require body-centre overlap. An actually moved target must update follow motion, not consume an old goal's result; preserve stall/claim state on a same-goal refresh and do not turn a refresh into a new accepted order. A changed order or vanished target uses existing replacement/TargetGone behavior, with exact identity. Clamp follow goals consistently by the follower radius. Validate goal/order identity and actual slot/rest state independently; do not ignore progress metadata.

Require the stationary-unit/queued-Hold regression, a moving-target control, target-gone behavior and radius-inset boundary control. Alternatives not taken: wait for impossible centre equality, snap into another body, drop the queued order, or consume a stale fallback after the target/order changed. This is completion/applicability under brief4 with the written5.3 behavior.

Evidence: main target/mc-evidence/t6-review/run47/preliminary-runtime/follow-move-probe/{result.json,output.log}; compile0/run101; raw SHA c8c2b9ae0fca0bc82222e5ff2b21072dda4a605923ace68f31883dfa7bb0cbf8. Runtime repair is not yet traced.

The C105 preliminary lease is returned: source remains clean ffb21b3, no build/server/browser child remains. Other probes passed, including corrected legal8-slot C93, C99 Gather, pure escape controls, all three arrival modes, and supporting1,000/5,000 cost checks. Those timings are review support only, not Task22 acceptance. Authorize one C106-C108 repair batch in active RUN46; source-only review continues in RUN47. After meaningful red/green proofs, commit and run final-source normal gates before the next preliminary runtime handoff, then every selected G-REF and required plant. No acceptance PASS, new agent run, merge or push.47/170 runs; Task6run5/6.


## C109: include initial unit-target Move work in the common cap

Conflict: the pending collector currently handles Point targets; a Move(Unit) or queued Unit-target activation can bypass the same cap that C106 repairs. This is a source scope lead, not a newly executed counterexample. Design5.2:494 keeps a live unit-follow reference,5.3:528 completes at the reached legal slot, and6.3:603-645 gives one shared initial spreading budget and whole-envelope Pending rule. Treating a Unit reference as an escape from admission/arbitration would leave that rule incomplete.

Choice: initial authored Move(Unit) and queued Unit-target Move activation enter the same global budget, capacity, merge/turn and whole-envelope Pending machinery. Resolve the live target's current radius-inset position for initial slot allocation, while preserving the accepted OrderTargetSim::Unit and per-member authored identity. Apply the written primary/spare/assignment rules to that initial allocation. Do not permanently turn follow into a Point order. Once active, ordinary updates while following a genuinely moved unit are motion under the same accepted order, not a new envelope or a per-tick command-budget charge.

C108's completion rule also respects a reached assigned cohort slot for the unchanged initial physical goal. A refresh must not immediately overwrite that slot with another body's centre, consume a stale reached result after real target/order change, or reset stall/claim state merely because the same goal is refreshed. Keep existing target-gone/queue behavior and exact identity. The independent validator derives initial target position/member identity from the live before-state and accepted input, not a producer flag. Include a Unit-target variant of the oversized-head control with a no-competition control, Pending/body invariance, and preserved Unit reference after activation; queued activation needs the same budget control.

Alternative not taken: exempt Unit references, recharge an already-active follow every tick, change the group key, snapshot the authored order permanently to Point, or require centre overlap. This is a mechanism/applicability ruling under brief4, with the same follow behavior and written cap/Pending shape. It expands the current C106-C108 repair batch before the next freeze, not after final refutations. Unit-target runtime proof is NOT TRACED until the probe runs.47/170 runs; Task6run5/6; no new agent run, merge or push.


## C110: keep prior execution live while its replacement is Pending

Conflict: RUN47's captured source (orders.rs SHA4d3ec1f3dad40b6785b78728c34842945abf26477b1b29dd19cbb3601042f5f4 atop ffb21b3) returns immediately for a pending member at run_orders:479, before the live-follow refresh at685/helper1686. Gather continues in its separate phase, but old Move(Unit) A can retain a stale target while replacement B waits. C93 and design6.3:639 keep the previous order; C109 keeps ongoing follow alive without charging a new envelope. This is a traced source lead; the new prepared probe has not yet run and is not an additional confirmed runtime High.

Choice: process the prior execution while B waits, with a separate queue-advance gate. An active A still refreshes its live canonical follow goal and retains unchanged-goal fallback/stall state. Completion, Blocked acknowledgment or TargetGone for A may finish/record A through the normal captured transition, but must not pop or start post-B Append work. B remains the pending head; D remains behind it. A completed prior execution may become idle while B waits. No new cap charge is created for an existing A refresh, and no new order identity is fabricated. Preserve C107 physical escape and actual old Gather/point execution too.

Use a pending-aware execution helper or explicit allow-queue-advance parameter rather than call the old helper unchanged: its completion/TargetGone branches start the queue. Require the prepared real-moving-target/old-A/B-Pending/D-gated control, with actual dispatched busy-head budget and a no-replacement positive control. Source and semantic red/green receipts must distinguish real execution from fixture setup. Alternative not taken: freeze A's live follow, activate D before B, suppress all old failure acknowledgments, or recharge A every tick. This is C93/C109 mechanism completion under brief4 with the existing player-visible Pending and queue behavior.

Evidence: main target/mc-evidence/t6-review/run47/pending-follow-source-lead/ and prepared pending_prior_follow_probe.rs. Add this to the existing repair batch before the next committed candidate/gates; no new agent run.47/170 runs, Task6run5/6; no merge/push.


RUN46 ended with a WIP repair handoff, not Task6 completion. Six explicit source files are dirty atop ffb21b3; root saved coherent exact bytes/hashes at main target/mc-evidence/t6-root/c106-c110-working-checkpoint/ (second hash pass matched; frozen SHA exact). C106 component promotion+Append replay selected1/1 exit0 with four forged-state negatives; focused group checkpoints are green but full repaired-source gates/refutations/plants remain. RUN48 registered before resuming the same Luna/high builder to finish the full authorized task.48/170 runs, Task6run6/estimate6, soft12/hard24; no new stop condition or merge/push. RUN47 remains source-only.


## C111: distinguish each follow origin from the shared pool anchor

Conflict: the coherent C106-C110 source compares shared pool.point (group maximum radius and first merged anchor) with the current member's own-radius Unit goal. A stationary target can therefore appear changed for a smaller member near a map edge, or for a distinct Unit reference merged under the same K4 key. Source: captured orders.rs:1753; report at main target/mc-evidence/t6-review/run47/c106-c110-source-review/report.md. This is a source lead, not a new executed runtime High.

Choice: add default-omitted per-member follow-origin metadata to MoveProgress: target unit ID and that member's canonical initial goal. Derive it at actual allocation from the accepted member Order, live before-state target position and member radius. It is distinct from the shared physical pool point/maximum-radius clamp. An unchanged member origin keeps the assigned pool slot, rest and stall state; real target/order change uses the live-follow path. Replay independently derives and verifies this metadata; it never trusts the producer field as authority. Point members keep the default and legacy data stays exact.

Require mixed radii at the map edge and distinct live Unit references in one merged group, with actual shared pool/slot retention and completion, plus the moving-target control. Alternatives not taken: compare unlike anchors, relax float equality, split the K4 group key, or discard an unchanged member's assigned slot. This is a data/mechanism detail under brief4 and C109.

## C112: maintain indexes for the new reservation paths

Conflict: source review finds new per-admission scans over unrelated pending/transient members (orders.rs:2464), and per-winner winners.iter/filter/min scans (orders.rs:1347). The latter visits W+(W-1)+... items. These reopen H3 cost concerns in a new path even though prior kernel cost probes passed. Evidence is source complexity; no new timing result is claimed.

Choice: maintain per-slot transient counts and per-unit membership/replacement indexes; use existing pending member indexes for incoming IDs. Build ordered per-slot reservation queues/indexes once and pop/select their heads, preserving real acceptance/FIFO order, merge/cap rules and both passes. Update every accepted Replace, Append classification, refusal, cancellation, promotion, grant and removal path consistently. Do not copy the whole world or AuditState per candidate. Keep canonical full-state replay checks; indexes are an implementation detail, not a producer authority.

Require a meaningful operation-count control for many singleton admissions with unrelated pending members and a many-direct-winner selection control, with actual branch-hit/cap assertions. Check counts/order invariants without rebuilding all indexes on each production admission. Alternatives not taken: suppress the source finding, keep quadratic selection because the tick cap is finite, or replace exact validation with cached after-state claims. This is execution/cache mechanism under brief4; source-only H3 remains open until repaired and rechecked.

## C113: fail only members whose own live target has gone

Conflict: captured dispatch at orders.rs:1391-1442 tests any missing Unit target in a merged request and TargetGone-fails every member. Under C95/C97/C109, that request can legally contain distinct Unit references and Point members. Replay already requires each failed member's own target to be absent (economylog.rs:3449), so the producer can report a false failure for a live target. This is a source/authority lead, not a new executed runtime claim.

Choice: when a pending head is considered for activation, independently identify members whose own Unit target has disappeared. Capture TargetGone only for those actors, with their actual accepted order IDs and normal failure/queue handling. A Point member or a member with a live Unit target must not inherit that failure. Remove failed members before evaluating the surviving work count/cap/pool. Keep the surviving envelope's slot/group/arrival position; it still spreads or waits atomically as one remaining envelope. Resolve the first eligible member's current goal with the surviving group maximum radius, and apply the same N-primary requirement and spare rules to live N. If no members remain, remove the head without a body step. Update reservation indexes after failed-member continuation before allocating survivor slots.

Replay proves each own target absence, exact failure identity/queue state, surviving member set/order, live goal and pool; it also covers both successful allocation and O12 failure from that live goal, not stale admission request.point. Require a mixed merged request containing a gone Unit reference, a live Unit reference and a Point member; check exact failures, atomic survivors and actual replay. Preserve FIFO and do not fail future queued work before it reaches the relevant head. Alternatives not taken: falsely fail live targets, freeze all valid members forever behind a vanished target, change the merge key, or silently drop missing-target actors without failure. This follows per-member accepted identity and C92's prune-before-cap pattern under brief4, preserving player-visible failure rules.

C111-C113 join the same active RUN48 repair batch before the next commit/final gates. RUN47 remains source-only; no new agent dispatch.48/170 runs, Task6run6/estimate6, soft12/hard24; no CEO stop, merge or push. Runtime proof of these new source leads is NOT TRACED until the valid controls run.


## C114: retain existing pending-cancellation cost as measured Medium debt

Conflict: after C112's new scans/winner selection/copies are repaired, cancel_units still walks canonical pending requests when an incoming ID already belongs to one. The new fresh/nonmember cost control does not establish a universal O(incoming) cancellation claim. A full stable-handle/queue representation rewrite would be a larger change; a reverse index alone still walks canonical queue headers.

Choice: retain this pre-existing cancellation path as Medium scalability debt, with an adversarial already-pending replacement workload for Task22 measurement/follow-up. It is not a Task6 High on current evidence: design6.7 explicitly bounds spreading and avoidance, timing remains Open, C106 cancels an already-pending ID once rather than multiplying that old path, and no measured tick-budget failure is shown. K25 bounds burst64 envelopes per slot and16,384 pending IDs. One full member pass would be about1.05M visits per slot burst; actual code makes several comparable passes and refill can sustain load. These are operation estimates, not measured CPU limits or a safety guarantee.

C112 remains required for genuinely new reservation preflights, ordered winner selection and full pending-state copies. Do not claim existing-member cancellation is O(incoming), hide it from the report, or mark a zero-unrelated-visit fresh test as proof of the old path. A future index should skip unrelated member arrays without changing FIFO/serialized queue semantics; a broader representation change needs evidence. Alternative not taken: stop the current freeze for an unmeasured old path, introduce a position-map rebuild that is still quadratic, or pretend a small index solves every case.

Evidence: RUN47 source assessment of captured cancel_units and its K25 applicability notes; C112/recorded six-file source report under main target/mc-evidence/t6-review/run47/c106-c113-coherent-source/. This is an explicit Medium defer under brief4:53; no design/cap/acceptance cut.48/170 runs; Task6run6/estimate6; no new agent run, merge or push.


## C115: give legacy MoveTo successful-only replacement semantics

Conflict: exact command.rs8e096e27 source has apply_move_to_per_unit clearing actual Gather at362-374 before queue admission can refuse RateLimited at379. Singleton apply_move_to at473 clears only plain progress/cache, leaving modern current/queue/cohort progress. Thus a refused command can stop Gather; accepted legacy C after provisional B plus Append D can retain D or old cohort motion. These are source-confirmed replacement/refusal gaps, not a change to frozen values or a new gameplay key.

Choice: as C102 does for legacy Gather, legacy MoveTo with no queue modifier is Replace. Complete validation and combined pending-capacity preflight before any destructive change. Refused units/envelopes preserve exact prior current, queue, Gather, pending/candidate state and motion. A successful direct legacy singleton cancels previous candidate/pending membership, current/queue/history and voluntary movement progress before installing its canonical target, preserving PhysicalEscape and ordinary free legacy arithmetic/default data. Never invent an authored ID.

For a successful legacy multi-unit envelope that must wait, clear only old future work on admission while previous current/Gather/physical execution continues. At actual activation, replace the previous current/motion, interrupt old Gather and preserve only Append work accepted after this replacement. Per-unit invalid recipients stay unchanged; whole capacity refusal changes none. Keep captured order resets where prior nondefault state requires them, but add no extra order operations to an ordinary default legacy path.

Do not put unconditional resets in shared target/Gather primitives used by authored or queued activation: C102 already exposed that mistake. Use the true legacy command entry or an explicit activation/reset mode so accepted suffixes of a currently activating modern order survive. Replay models exact successful reset/cancel and refusal preservation from validated input/before-state; no broad exception.

Require accepted B+Append D→legacy C with D cleared, old cohort target unable to overwrite C, refused legacy multi MoveTo with active Gather unchanged, and pending legacy activation preserving old execution/new suffix. Full free/frozen regressions remain mandatory. Alternatives not taken: mutate on refusal, retain cancelled future work, make legacy C resumable behind B, or reset every internal primitive call. This is the nearest existing Replace mechanism under brief4, preserving the command’s player-visible intent and K19.

## C116: cache new promotion insertion boundaries without changing FIFO

Conflict: C106 promotion still performs queue.iter().position for each new loser, walking a large old prefix repeatedly. This is introduced by C106 and is distinct from the pre-existing cancellation debt deferred by C114. Canonical pool IDs are not sorted after promotions; a binary search on raw pool ID or naive ascending append is unsafe.

Choice: include this new header-search cost in C112. Compute/cache immutable pre-promotion insertion boundaries (slot, accepted tick, predecessor/actual arrival relation) once, then retain the correct reverse promotion/FIFO placement, or use an equivalent bounded ordered merge that keeps captured per-actor transitions exact. A proven tail fast path may avoid old-prefix scans but must handle multiple reverse-processed losers correctly. Preserve real arrival, same-slot order, pool identity and exact replay. Do not undertake the larger canonical queue rewrite deferred by C114, or claim all vector maintenance is universally linear.

Require a many-older-requests/many-promotions operation-count control plus the existing arrival/FIFO and cap controls. Alternatives not taken: defer a newly introduced repeated header scan as if it were old debt, sort by seq/pool ID, or silently reverse new candidates. This is execution/index handling under brief4 and C112.

Evidence for both decisions: RUN47 seven-path boundary source review and exact captured command/Orders call stacks, main target/mc-evidence/t6-builder/run48/source-review-checkpoint/ and run47 reports. They join active RUN48 before the next commit/gates; no new agent run.48/170 runs, Task6run6/estimate6, soft12/hard24; no CEO stop, merge or push. New targeted runtime controls remain NOT TRACED until run.

## Resume after status handling error

Root ended status-only turns while authorized work remained. This caused an avoidable idle period. Status and ETA questions are steering; root must answer briefly and continue work until an explicit pause or a required stop condition. RUN48 ended at a seven-path uncommitted boundary; RUN47 source report has finite unresolved findings, not PASS. RUN49 is registered before builder continuation.49/170 runs; Task6run7/6, below soft12/hard24. No new CEO decision is needed.

RUN50 registered before independent Sol/high reviewer continuation. Source review may overlap RUN49 repair; no reviewer runtime/compiler/browser until explicit C105 lease after stable normal gates.50/170; Task6run8/6, below soft12/hard24.
