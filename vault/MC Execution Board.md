---
type: task
date: 2026-10-29
tags: [turts, board, mc]
ai-first: true
source: codex
---

# MC execution board

State: ACTIVE: Waves1-2 complete. T19 merged98135d3; independent0High/0Medium; integration878passed/0failed/18ignored and G-RUST/G-QA/G-FROZEN0. Wave2 pushed98135d3; CI41148971118 queued. Build30/170 includes registered T2b run30. C1-C16 recorded. Mainc3f09d0 untouched; one cache, completed T19 lane removed.

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

Frozen M2 SHA256: `bcbf72c2206251f635a7c7f5b6c8f573d69a5fc05a5bd77716e98ebfb08e370b`. Keep forge fixed point and refutation plants. Known browser failure: M2 browser proof; trace in T20, repair in owner task/T21. Wave1 hosted source8e95e42: check/agents success; browser failed at the unchanged M2 economy timeout after T7 passed. Native M2 refutations, all17M3 plants and acceptance succeeded. Receipt target/mc-evidence/wave1-ci-terminal-jobs.json. Wave2 remains in progress and has not been pushed.

CI trigger: `.github/workflows/ci.yml:2-5` runs pushes to main and pull requests. After Wave1 gates pass, push integration and open a draft PR to main to run CI. Do not merge it. Keep the pushed code head free of a skip-ci marker; record CI results in a later vault-only commit.

## Known red window

| Evidence | Current result | Cause and owner |
| --- | --- | --- |
| Historical CI 40960227409 | E9-browser-base-economy times out after 15 s. Server completed production at tick175; captured client stayed at tick87. | Underlying cause NOT TRACED. Historical artifact and log retained under target/mc-evidence/t20-builder. |
| T20 clean local baseline at22a4600 | Selection, economy and recovery pass; E9-browser-base-public-convergence fails its shared-tick public-projection assertion (tests/browser/m2.js:430-444), exit=1. | Forced-software-profile diagnostic; mechanism NOT TRACED. Isolated control and active-lifecycle cases both pass with both pages visible. A full instrumented NVIDIA run passes all15 cases; clean original-source normal-native baseline also passed all15 cases, exit0. No engine owner assigned; hosted cause remains NOT TRACED. |

## 4. Tasks

24 numbered tasks; task 2 has two branches, for 25 lanes. T1 merged at8f612a3 after completion of supplementary Chrome receipts and CEO Firefox deferral. T2a and T3 are complete. T20 passed its clean local baseline and is implementing the harness.

| Task | Wave | Needs | Branch | State |
| --- | --- | --- | --- | --- |
| T1 | 1 | - | codex/mc-t1 | COMPLETE; merge8f612a3; G-RUST/G-QA/G-FROZEN all0; lane removed |
| T2a | 1 | - | codex/mc-t2a | COMPLETE; merge17286b1; all gates0; lane removed |
| T3 | 1 | - | codex/mc-t3 | COMPLETE; merge aceef85; all integration gates pass; T7 115 frames |
| T20 | 1 | - | codex/mc-t20 | COMPLETE; merge8e95e42; all integration gates0; task lane removed |
| T8 | 2 | 2a, 3 | codex/mc-t8 | COMPLETE merge461d105; review0High/0Medium; integration G-RUST/G-QA/G-FROZEN0; lane removed |
| T18 | 2 | 3 | codex/mc-t18 | COMPLETE candidate179ada7; mergeb1b3834; review0High/0Medium; all integration gates0 including C12 T7; lane removed |
| T19 | 2 | 3 | codex/mc-t19 | COMPLETE merge98135d3; review0High/0Medium; all integration gates0; lane removed |
| T2b | 3 | 2a, 3, 18 | codex/mc-t2b | REGISTERED run30; message-size trace first |
| T4 | 4 | 2b, 3 | codex/mc-t4 | NOT STARTED |
| T9 | 4 | 1, 8, 2b | codex/mc-t9 | NOT STARTED |
| T6 | 5 | 4 | codex/mc-t6 | NOT STARTED |
| T7 | 5 | 2b, 4 | codex/mc-t7 | NOT STARTED |
| T11 | 5 | 9, 19, 20 | codex/mc-t11 | NOT STARTED |
| T12 | 5 | 3, 9, 19, 20 | codex/mc-t12 | NOT STARTED |
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
| 30 | T2b implementation | network-engineer | gpt-6-luna | high | 98135d3 plus vault | REGISTERED: Lightyear reliable limit first, then protocol migration; task1/estimate4/hard16. |

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

C17: Task2b hook test lives beside auto_move in join.rs rather than tests/commands.rs; actual hook queue and first-Move selection must be exercised. Original helper red retained; helper-only proof is insufficient. See [[Decisions/2026-10-30 MC Codex rulings]].
