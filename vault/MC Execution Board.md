---
type: task
date: 2026-10-29
tags: [turts, board, mc]
ai-first: true
source: codex
---

# MC execution board

State: ACTIVE on the desktop: 17/25 MC lanes complete; 80/170 build runs registered. Task10 merged at ea943bf and all integration native/QA/frozen/web gates passed. The actual HD1 process also passed in the assigned integration cache; no duplicate cache. Task15 RUN78 owns the sole compiler/cache/runtime. Two actual AUD1 pairs expose corrected detector calibration and the designed old-session rejoin race; C229 advances bounded retry/countdown from Task16: the compiled named red101 is followed by9 draft native passes0. Final ingest/driver controls, WASM check and browser validation remain. Actual alert playback remains unproved and the driver route is being repaired. Task17 RUN80 independently passed all C227/C231 source repairs; native/browser gates wait for the cache. RUN79 remains source-only until its later runtime lease. Wave7 push/CI follows Tasks15 and17. Main bb611bd is unchanged. CEO PT1/PT2 stop remains at Task23.

Plan: `docs/superpowers/plans/2026-10-29-mc-player-control.md`.
Design of record: `docs/superpowers/specs/2026-10-29-mc-player-control-design.md`, revision14 at mainf050e42.
Brief: [[Reports/2026-10-29 MC Codex brief]].
Original base: `a0a3076`; rebased on CEO instruction to `main` at `f503cff`; integration: `codex/mc-integration`, worktree `/home/julien/Projects/TuRTS-mc-integration`.
Task 1 lane: `codex/mc-t1`, worktree `/home/julien/Projects/TuRTS-mc-t1`, `CARGO_TARGET_DIR=target/mc-t1`.
Main base is `f050e42` (CEO O13/design14); do not merge or push main. The pre-existing untracked disk audit stays in the main worktree.

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

- Design revision14 at mainf050e42 wins over the plan; O13 requires VIS1 >=20 normal and >=10 each Machado simulation at severity1.0 with the unchanged Okabe-Ito palette; O12 requires all N legal primary slots, with zero or more spares. O10 requires ShellAction/client_shell.rs; O11 uses the Shell Press for PT1 step6. Apply the brief section4 decision rights and record each Codex ruling C<n> on this board and in the rulings note.
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

24 numbered tasks; task2 has two branches, for25lanes. Waves1–5 complete14lanes; Task5 adds the15th complete lane. Task13 RUN67 owns the sole cache/runtime, with RUN68 independent source review. Wave6 push/read follows Task13 acceptance and integration re-gate.

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
| T6 | 5 | 4 | codex/mc-t6 | COMPLETE ed793bb; merge450c023; final review0High,21probes/191tests;20GREF/8plants/ownplant; integration12/12 gates0,1211/0/18,T7,M3; lane removed with compressed evidence/cache preserved. |
| T7 | 5 | 2b, 4 | codex/mc-t7 | COMPLETE c190c909; merge7e575c9; independent0H/0M/30tests/ownplant, integration12/12gates0 (1231/0/19,T7,M3); lane removed, evidence and branch retained. |
| T11 | 5 | 9, 19, 20 | codex/mc-t11 | COMPLETE c996463; merge aff4d63; independent 0H/0M/0L, 11 probes, CAM1 2/2 and both plants restored; integration 12/12 gates0,1266/0/19; completed artifacts archived and lane removed, branch retained. |
| T12 | 5 | 3, 9, 19, 20 | codex/mc-t12 | COMPLETE candidate77ca599; mergeaf567c4. Independent0High/0remaining,7probes and own production101→0. FreshHUD1six/HUD4two pass; required runtime-default mutation1→restore0. Integration11/11gates0,1306/0/19,frozen,T7. All28removed tests mapped; registered3panelplants remain planned-window NOT CHECKED. Lane removed, evidence/cache/branch preserved. |
| T5 | 6 | 4, 6, 19 | codex/mc-t5 | COMPLETE mergee214ccc from98147ae. RUN65 PASS0High,15independentprobes/ownplant0-101-0; alltaskgates/19GREF/2requiredplants; integration11/11gates0,1419/0/19,T7 137frames/53moving. Lane removed after provenance/hold checks; branch retained. |
| T13 | 6 | 4, 6, 9, 11, 19, 20 | codex/mc-t13 | COMPLETE merge60082da; candidate9ae04f4, independent0High/0Medium, normal/requiredplants/native19/actualforeignsolid closed, fresh5000VIS2record; native10integrationgates0 (1446/0/19), T7 137frames/53moving exit0; lane removed, branch/evidence retained. |
| T10 | 7 | 5, 9, 11, 19, 20 | codex/mc-t10 | COMPLETE b051e04, merged ea943bf; RUN77 0High/0unresolvedMedium; all integration gates and actual non10 native proof0; lane retired. |
| T15 | 7 | 4, 7, 12, 13 | codex/mc-t15 | ACTIVE RUN78 sole runtime; focused native green; AUD1 pair2samePIDreloadfail; C229 boundedretry/redfirst and lawfulpixel/audio-route repairs pending. RUN79 independent source review; no candidate/fullrowgreen yet. |
| T17 | 7 | 3, 12, 13, 19, 20 | codex/mc-t17 | SOURCE READY UNCOMPILED; RUN76 ended102WIP, root C227 bounded repairs/RUN80 sourcePASS; no target. New builder row needed after15merge/rebase/cachehandoff. |
| T14 | 8 | 5, 12, 13, 15 | codex/mc-t14 | NOT STARTED |
| T16 | 8 | 7, 12, 15 | codex/mc-t16 | OPEN; volume/id subsets advanced underC189/C225; C229 retry/countdown advanced forAUD1. FullTask16rows/threewindows/backendreview remain unstarted. |
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

CEO approved about 212 total runs. Build stop line: 170 (106 planned task runs plus 64 repairs). Gate estimate: 42 spent before this build. Build dispatches used: 63/170. Root preflight is recorded separately and is not a builder/reviewer dispatch. Twice a task estimate requires a recorded continuation decision; four times is a hard stop. T18 estimate2: soft4, hard8. No dispatch at global170 or a task hard limit.

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
| 49 | T6 finite source findings repair and task completion | simulation-core-engineer | gpt-6-luna | high | ffb21b3+seven-file WIP | COMPLETE ed793bb; all normal gates0,20GREF/87commands,8validplants/27executions; source exact restored; merge450c023 and integration12/12gates0. Task6run7/6. |
| 50 | T6 independent finite finding closure and final review | performance-architect | gpt-6.1-sol | high | ffb21b3+RUN49 repairs | COMPLETE PASS ed793bb:0High,C114Mediumdeferred,C121scope;21probes/191tests, all20GREF/8plants verified, ownGM40 0/101/0 exactrestore; finalcache returned. Task6run8/6. |
| 51 | T7 server session implementation continuation | backend-engineer | gpt-6-luna | high | integration450c023+preserved5pathprep | COMPLETE builder c190c909; compiled reds/greens, f9native1228/0/19 andACC17/0/0,196GREF2/12commands, C125boundedfinal253/0/18+QA+process1/1; lease returned toRUN52. Task7run2/4; independentreview/mergepending. |
| 52 | T7 independent session/script/log review | network-engineer | gpt-6.1-sol | high | Task7 base63c8ab4 plus RUN51 | COMPLETE PASS c190c909;0H/0M,30focusedgreens, ownfreshnessplant2named101s/restored0,16hashesexact, lease/readholdreturned. Task7run3/4. |
| 53 | T11 camera and light implementation continuation | bevy-rendering-engineer | gpt-6-luna | high | d568edb preparation, rebased6f73ed5 | COMPLETE c996463; all normal gates0, E9 GREF green/semantic1/green, PointerLeft native101 and CAM1 red2/2, exact restore/native0; C131 web green independently supplied by RUN54. Task11 run2/4. |
| 54 | T11 independent camera and light review | ux-designer | gpt-6.1-sol | high | c996463 | COMPLETE PASS 0H/0M/0L;11 independent probes, first CAM1 2/2 supplied C131 restore and independent pixels; own drag-speed plant0/101/0, exact source restored, lease/read hold returned. Task11 run3/4. |
| 55 | T12 widget, cursor and guide implementation continuation | bevy-rendering-engineer with ux-designer | gpt-6-luna | high | ffff0a3 after T7 rebase | Registered BEFORE dispatch. Implementation completed through77ca599 finalgates and fresh8browsergreens. INTERRUPTED when its proposed fixture plant contradicted Task12step4; no plantcredit. Root restored and registered RUN60 before continuation. Actual native9selected/7semantic failures exit101; C134 HUD1 all6 named missing-widget/panel failures exit1 with valid owned selection and exact viewport/DPR. Runtime implementation active; preserve legacy28 until covered. Task12 run3/estimate8, soft16/hard32. |
| 56 | T12 independent widget runtime and data-source review | mod-runtime-engineer | gpt-6.1-sol | high | f6577b9 plus RUN55 preparation | COMPLETE final77ca599 PASS0High/0remaining. Independent7/7native probes and42/42 pure instrument probes; own pending-clear production plant selected1 semantic101, exactrestore selected1green0. Full source/gate/browser attribution explicit; final report SHAe05466c5. All holds/processes released; root read raw receipts/hashes before merge. Task12run4/estimate8. |
| 57 | T12 parallel asset loader implementation | bevy-rendering-engineer with ux-designer | gpt-6-luna | high | f6577b9 plus active RUN55 source | COMPLETE source-only handoff:390-line assets.rs with qualified image/cursor/font maps, separate CPU-retained cursor decode, nine-slice and reload controls. No compiler/browser/cache execution; tests NOT TRACED. RUN55 integrates/runs and repairs; RUN56 independently reviews. Task12 run5/estimate8, soft16/hard32. |
| 58 | T12 parallel permitted source semantics | bevy-rendering-engineer with ux-designer | gpt-6-luna | high | f6577b9 plus active RUN55 source | COMPLETE source-only handoff f6fc7731 sources.rs with inline semantic tests, qualified assets and exact typed actions. No compiler/browser/cache execution; RUN55 executes/integrates and owns later repairs. RUN56 continues independent review, including H7 target-aware capability. Task12 run6/estimate8, soft16/hard32. |
| 59 | T12 authored widget lifecycle coverage | bevy-rendering-engineer with ux-designer | gpt-6-luna | high | f6577b9 plus RUN55 repaired runtime | COMPLETE source-only handoff6f56c184, initial lifecycle3/3 executed by RUN55. RUN55 then extended to5/5 in native-final-v004: exact Produce/disabled action, retained press/cancel/job cleanup, DPR2 fifth-job scroll, actual CoreAdapter inside/outside world input and first-frame/clip/inactive-camera snapshot. Initial helper world suppression was NOT TRACED; new controls are separate RUN55 proof. Handoff preserved in main t12-lifecycle/run59. Task12run7/estimate8, soft16/hard32. |
| 60 | T12 required production cursor mutation and handoff | bevy-rendering-engineer with ux-designer | gpt-6-luna | high | 77ca599 | COMPLETE. Registered before resuming interrupted RUN55. Correct production context override only: actual headed HUD4 nondefault move→default mismatch, command1; exact cursorSHAae02fe39 restore and HUD4two0. FixtureSHA14246814 unchanged. Clean77ca599 handoff, no acceptance claim before RUN56 PASS; mergedaf567c4 and root11gates0. Task12run8/estimate8; build60/170. |
| 61 | T5 weapons, attack/attack-move and audited policies | simulation-core-engineer | gpt-6-luna | high | Wave5 ad0eef4 plus registration docs | Registered BEFORE dispatch. Task5run1/estimate4, soft8/hard16. Branch codex/mc-t5 and own worktree; sole compiler/cache lease after root transfer, all five variables. Must get compiled server-envelope/actual rush semantic red before production implementation; preserve frozen Forge replay and Task6 movement/queues. |
| 62 | T13 visuals, palette and pixel instruments | bevy-rendering-engineer | gpt-6-luna | high | Wave5base, O13/rebaseca0c36b3 | INTERRUPTED at CEO travel pause after provisional4dfe0b3; historical evidence retained. Earlier phase: Source/pureNode prep+RUN66 source review complete with routinecompileprep pending. Root solecache/runtimegrant afterTask5integration11/11gates. Nativepalettebaseline101:4selected/1referencepass/3contractfail. HeadedVIS1baseline pending. No productionapply beforebothvalidsemanticreds. Task13run1/estimate5, soft10/hard20. |
| 63 | T5 independent combat/policy/replay review | qa-lead | gpt-6.1-sol | high | Task5 base12e435c plus RUN61 stable WIP | Registered BEFORE dispatch. Task5run2/estimate4, soft8/hard16. SOURCE-ONLY: inspect current stable interfaces and prepare finite independent probes outside lane; no compiler/cache/browser/source edits until root lease. Check actual lifecycle, all fire/engage axes, immutable catalog policy, plain Move identity audit, grid/determinism, frozen evidence and required plants. |
| 64 | T5 repair continuation after O13 | simulation-core-engineer | gpt-6-luna | high | 12ded296, O13 mainf050e42 | COMPLETE clean98147ae; all10native/QA/frozen0,ACC0,headedT7 0;19selectedGREF,requiredJob/rushplants semantic101 thenrestored0. Finalhandoff/run64. Task5run3/estimate4. |
| 65 | T5 independent recheck after O13 | qa-lead | gpt-6.1-sol | high | O13-rebased Task5 plus WIP | COMPLETE PASS0High on98147ae. Original15independentprobes0; ownfocus-preferenceplant0/101/0, exactrestore+all16sourcehashes; allnormal/GREF/requiredplant evidence raw/hashrechecked. Allholdsreleased; merged/regated/laneremoved. Task5run4/estimate4. |
| 66 | T13 independent visual/acceptance source review | ux-designer | gpt-6.1-sol | high | T13 prep2bcb9ae plus off-lane candidate | INTERRUPTED at CEO travel pause; source reports/probes retained, no final runtime PASS. Registered BEFORE original dispatch. Task13run2/estimate5; soft10/hard20. SOURCE-ONLY until explicit root lease. Review design14/O13/C161/C163/C167; full VIS1 pixels+first-frame state, SEL2/3, palette thresholds, shared assets, hover/bar/marker/blocked lifetimes and actual VIS2 instrumentation. Candidate not yet compiled/applied; prepare finite independent probes/own four-gap visual plant outside lane. T5 RUN65 owns sole runtime. |
| 67 | T13 resume, C178 projection and final visuals proof | bevy-rendering-engineer | gpt-6-luna | high | 4dfe0b3, integration640fa20 | Registered BEFORE dispatch2026-11-09. Task13run3/estimate5, soft10/hard20. Resume C178 real-command regression red then read-only view repair; fresh fullVIS1, final gates/plants/VIS2. Sole compiler/browser/cache owner target/mc-t13. Prior interrupted4dfe wrapper is notPASS. Preserve full scope and frozen values; independent review remains required.  PAUSED by CEO travel request2026-11-09; no active lease. Resume from laptop handoff and raw pause checkpoint; Task13 not accepted. |
| 68 | T13 resumed independent visual and projection review | ux-designer | gpt-6.1-sol | high | 4dfe0b3 plus RUN67 C178 | Registered BEFORE dispatch2026-11-09. Task13run4/estimate5. SOURCE-ONLY until explicit runtime lease. Rebind historical RUN66 evidence/probes to actual final source; review C175-C178, actual pixel/first-frame/waypoint proofs, final native initialize and own foreign-solid plant. No compiler/cache/browser/lane edits until handoff. |
| 69 | T13 final source and runtime review continuation | ux-designer | gpt-6.1-sol | high | 1b649dc plus C179/C181 repairs | Registered BEFORE resuming completed RUN68. Task13run5/estimate5, soft10/hard20. Review stable actual repair source and raw replay, then exclusive native initialization/own actual foreign-solid plant after builder final gates. SOURCE-ONLY until explicit lease; full acceptance NOT TRACED.  PAUSED by CEO travel request2026-11-09; no active lease. Resume from laptop handoff and raw pause checkpoint; Task13 not accepted. |
| 70 | T13 desktop continuation after CEO travel pause | bevy-rendering-engineer | gpt-6-luna | high | 40b9d90 | COMPLETE9ae04f4. C186causaldiagnostic/isolatedSEL2, no-screen/palette requiredplants, finalactual5000VIS2 redrecord/green/commit. Rootmerged60082da. Allholds released. Global71/170. |
| 71 | T13 desktop final independent review continuation | ux-designer | gpt-6.1-sol | high | 40b9d90/9ae04f4 | COMPLETE APPROVED0High/0unresolvedMedium. Native19/19; actualfullVIS1baseline2green/ownforeignsolid2red/exactrestore2green; finalsource/rawGPU5000/docs audited. Map/filehashlabel corrected and originalreceipt preserved. Allholds released. |
| 72 | T10 headless actions implementation | bevy-rendering-engineer | gpt-6-luna | high | currentintegration afterWave6 | Registered BEFORE dispatch. Task10run1/estimate2 soft4/hard8. Solelocalruntime/cache in target/mc-t10. Actualredfirst, HD1native/browser, gates/plants thenindependentreview. | COMPLETE builder handoff; final product b051e04 accepted/merged after C216.
| 73 | T15 feedback/audio source preparation | bevy-rendering-engineer | gpt-6-luna | high | c4afbc9 | SOURCE PREP COMPLETE, UNCOMPILED/UNACCEPTED. 98 paths archived/hashed at run73/handoff-20261109; no target. Resume only under new ledger row/explicit lease. Actual stub reds preserved; pure checks25/25. |
| 74 | T10 independent early-to-final review | qa-lead | gpt-6.1-sol | high | c4afbc9 + RUN72candidate | Registered BEFORE dispatch. Task10run2/estimate2 soft4/hard8; global74/170. SOURCEONLY until rootlease. Review realcore/actions/native-browserparity/groupproof and C187 operationalreuse proof, then actualindependent tests/ownplant and finalcandidate0High afterbuildergates. | COMPLETE original311 review0High/0Medium, ownGroupDrop101/restore0 and C196 controls0/2/2; retained original identity.
| 75 | T17 source/test/asset preparation | ux-designer | gpt-6-luna | high | Current integration after C211 | Registered before dispatch. Task17 run1/estimate4, soft8/hard16; global75/170. Own lane/branch, SOURCEONLY; no target/compiler/browser/server or Forge HUD before actual baseline reds. | COMPLETE source prep only, continued by RUN76.
| 76 | T17 builder continuation | ux-designer | gpt-6-luna | high | RUN75 five-path source handoff | Registered BEFORE continuation. Task17 run2/estimate4, soft8/hard16; global76/170. SOURCEONLY SEL1/HUD2 baseline preparation until explicit sole-cache lease after T10 merge/regate. No Forge HUD/assets before actual baseline reds. |
| 77 | T10 independent cache-routing repair re-check | qa-lead | gpt-6.1-sol | high | Approved311d486 plus C216 bounded instrument repair | Registered BEFORE continuation. Task10 run3/estimate2, soft4/hard8; global77/170. Re-check only assigned-cache repair and affected HD1 proof. Original RUN74 approval/plants preserved for unchanged product source; no new broad matrix. | COMPLETE b051e04 approval0High/0unresolvedMedium;3LowC209. Root merged/regated/actualnon10proof0.
| 78 | T15 source and runtime continuation | bevy-rendering-engineer | gpt-6-luna | high | Preserved RUN73 WIP rebased to ea943bf | Registered BEFORE dispatch. Task15 run2/estimate5, soft10/hard20; global78/170. SOURCEONLY compatibility audit while Task17 has baseline lease; no Cargo/target/server/browser until explicit sole-cache handoff. Then compile/gates/actual browser acceptance on the existing scope. Current: sole runtime/cache15. Focused native152/ORD2four/camera33/WASM0 partial; actual AUD1pair2fail1/sourceimmutable beforeplaybackreload. C228 foreigndetector64 correction/sourcecheck thenfreshpair. No candidatecommit/fullacceptance yet. |
| 79 | T15 independent source-to-final review | ux-designer | gpt-6.1-sol | high | ea943bf plus RUN78 preserved implementation | Registered BEFORE dispatch. Task15 run3/estimate5, soft10/hard20; global79/170. SOURCEONLY until explicit final runtime lease. Consolidate feedback/audio/real-input acceptance findings before costly final gates, then independent own plant/final review after builder gates/plants. Current: consolidated1High/5Medium C220 repaired/sourcePASS; C225 PID/ALR sourcePASS refreshedactualoracle236aa20. C228 exactPNGhelperfix recheckpending. No runtime; ownvoiceplant/finalreview stillrequired. |
| 80 | T17 independent source-to-final review | mod-builder | gpt-6.1-sol | high | ea943bf plus RUN76 102-path source candidate | Registered BEFORE dispatch. Task17 run3/estimate4, soft8/hard16; global80/170. SOURCEONLY schema/content/actual-adapter/pixel-instrument review until root lease; no compiler/target/runtime. Independent final own plant/review after later builder continuation and gates. Current: C2271High/5Medium repaired/rootboundedfivefilepatch andindependentsourcePASS. Actualpurewrongcolour oldfalsepass0→repairednamedfail1/bluegreen0;63fileclone62identical/oneapprovedmap. Controlsactualscroll/native/gates/ownhealthplant pending. Proof t17-review/run80-source/. |

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


RUN49/RUN50 repair checkpoint: root verified the C116 cross-slot compiled panic (1 selected, exit101), repaired cross-slot/index/lookup/queued-self component greens, and full sim207 plus integration suites and server92/92 checkpoint exits0. These package results predate final test-only additions and are not formal acceptance. Per-slot immutable prefix maxima matched a linear insertion oracle in12,000 generated cases (root evidence t6-root/run49-promotion-index); C116 also has Rust nonmonotone/interior/equal-boundary controls. Source-only RUN50 is positive on the finite production repairs; full acceptance and repaired runtime remain pending. The phase test was tightened after an insensitive mutant passed. The matched-tick modern command prefix then gave actual0/101/0: disabling the span guard accepted a forged premature grant. Exact restoration and raw logs/observed exits are retained. This development mutant gives no postgate plant credit. Final3 source snapshot is explicit seven-path copies; economylog SHA1b912f7a20c9c8acdfdc22afc0645e2ab9e9ea362198761b858152bfd4b7d31f. Group/Unit prior restoration and forged negatives pass2/2. Old-authored Blocked identity has producer and component replay controls; the latter is not a regenerated end-to-end scheduler trace. Combined-capacity proof uses63 real accepted reservations with constructed16,320 pending entries, not a reconstructed live K25 history. No source commit, formal repaired gates, acceptance or merge yet.50/170 runs; Task6run8/6, soft12/hard24. Disk667GiB free; one compiler cache.

## Task6 committed repair and native gates

RUN49 committed754a1f5 explicit seven paths and then lint-only7898b267 explicit three paths; both have Codex trailers and clean state. RUN50 independently verified final3 source copies and Git blobs:0 known open production-source Highs, not acceptance PASS. Official reviewed native runner on7898b267 passed10/10 commands: exact Clippy, wasm, workspace1207 passed/0 failed/18 ignored across107 summaries, boundary, allQA, frozen hash and frozen tests. Root read all logs/exits/environment and wrote main target/mc-evidence/t6-root/native-7898b267-root-check.json. Source scalar accounting oracle remains unchanged. ACC and direct T7/M3 follow; C105 runtime, selected20native GREF, required plants and final independent acceptance remain.50/170; Task6run8/6.

An earlier ad hoc gate wrapper omitted process environment for Clippy/WASM and produced an undefined status. Those runs are not accepted. Root verified quiescence and /proc cwd/exe/fd/maps references, acquired Cargo locks, preserved inventory/provenance/logs, then deleted only inactive default target/debug and target/wasm32-unknown-unknown outputs.7.45GiB physically freed;671GiB free. Sole target/mc-t6 and clean source7898b267 remained. Receipt: main target/mc-evidence/t6-root/run49-wrong-env-cleanup/receipt.json. All compiler commands must set explicit per-process target/profile/incremental/job environment; shell environment does not persist between tool calls. The approved native helper environment.json and actual live Cargo /proc environment were both verified before accepting results. No new run, merge or push.


C105 preliminary runtime handoff: unchanged clean7898b267 now has all normal gates green. Root verified native10/10 and1207/0/18, ACC16/0 (E9loss179.02s/F1180.93s), T7oracle1/0 with136frames53moving, and M3actual produce/disable/remove accepted,7 full/canvas pairs, cleanup0. Root receipts are main t6-root/native-7898b267-root-check.json, acc-7898b267-root-check.json and web-7898b267-root-check.json. T7 and F11 evidence copied off-lane/hash verified; M3 wrote directly off-lane. The first M3 setup was interrupted130 before captures to relocate output; it is not a semantic red or gate pass. Valid second run is the accepted proof. Builder explicitly released quiescent sole cache; root authorized active RUN50 C105 unmutated19-probe batch, historical cap/escape/follow first. Reviewer verified all prerequisites and wrote run50/preliminary-lease.json before any compiler use. RUN49 is read-only; no source mutation, GREF or own plant under this lease. All20 selected native refutations, required builder plants and separate reviewer own plant/final acceptance remain.50/170; Task6run8/6; no merge/push.

## C117: retain and consume a completed Unit-follow cohort result

Conflict: independent RUN50 compiled and ran the mixed-radius stationary Unit-follow cohort on clean7898b267, exit101. Actor1 reaches its exact assigned slot(1.6500001,10.527757) at tick44 with no target, but MoveProgress has already been removed while current remains Move(Unit3). At tick45 it moves to(1.5366107,10.625955) and reopens a target. movement.rs:2387-2390 clears all Resting progress when no member is moving; orders.rs:2093-2125 needs the follow_origin/Resting result to complete through Orders. The completion capture later clears current/target but assumes movement already removed progress. Design6.3-6.5, C108 and C111 require legal slot arrival to finish the unchanged follow move.

Choice: retain the terminal follow-origin result until the actual captured Orders completion consumes it. Explicitly clear the consumed movement metadata on completion even with no queued successor. Keep the per-member Unit identity and own-radius canonical goal; a live moved target must resume correctly and a vanished target must keep TargetGone semantics. Preserve Point/ordinary legacy cleanup, exact free arithmetic/default data, claim/pool release and physical escape. Reconstruct the retention and captured consumption in strict replay; never use the after-state as an oracle. Add actual same-target mixed-radius and distinct-target merged-cohort arrival→completion/stable controls, no-successor cleanup, moved-target control and forged replay negatives. No implementation until the reviewer returns its immutable runtime lease; then red/green, commit, all final-source normal gates, independent recheck, selected refutations/plants. This changes the internal result lifetime; player keys, timing, failure rules and acceptance rows do not change.

Evidence: main target/mc-evidence/t6-review/run50/preliminary-runtime/member_follow_origin_completion_probe/{result.json,output.log}, compile0/run101, raw SHA2565df60b9e5fef364e70893750f9eaf3ed02b2051b6b232777a153a940c09ea3e1. Root read the raw arrival/next-Orders proof and source. The earlier retention-only assertion is superseded by this physical-motion counterexample. Alternative not taken: mark the trace green because metadata was removed, require followers to overlap the target body centre, write order operations in Movement, reopen an unchanged completed goal, or retain terminal metadata forever. Normal High repair under brief4; no CEO stop.50/170 runs; Task6run8/6, soft12/hard24.

## C118: bound the replay context for one atomic member-dispatch span

Conflict: a successful group dispatch pops its whole pending request in the first captured per-unit operation. Later members still need their own validated accepted order and follow origin. Reusing the first member current order loses distinct targets; using each actor prior current can substitute an old order. The new ephemeral replay context must also reject foreign/out-of-order members and an incomplete group at a phase/Tick/end boundary. The prior validator checked incomplete command and queued-capacity-failure spans but had no such member-context end check (economylog.rs970-1036; cached dispatch fallback around3930 and4227).

Choice: retain the independently validated per-member pending request for the contiguous atomic dispatch span. Accept only the next unstarted member and its exact accepted order/target. Reject an interrupting record or an unfinished span at log end; never silently discard it. Keep complete command-only prefixes valid when no automatic member span has started. Apply actual activation cleanup to every member, including last_failure and Gather, while preserving post-replacement queue work. Use validated before-state/history; no after-state oracle. This follows the existing complete-span checks and C97/C111. No serialized game field, player action, timing rule or acceptance row changes.

Evidence: RUN50 immutable C117 source review and RUN49 c117/server-c117-final-validation.log/.exit (focused1/1 exit0): actual mixed-radius/distinct-target cohort lifecycle, second-member prior failure kept while Pending and cleared on activation, wrong-member/retained-failure negatives, and truncation after only the first dispatch rejected. Root source lead was the missing context boundary/member check; the runtime game repair remains C117. Alternative not taken: copy the first member order, trust the current actor old order or an unvalidated after snapshot, or accept a completed-log claim with a partially captured atomic dispatch. Normal implementation/replay decision under brief4; no CEO stop.50/170 runs; Task6run8/6.

## C119: use a cursor for the atomic replay member span

Conflict: the new C118 replay context finds the next member by scanning request IDs against all prior progress, then repeats a nested every-member-started scan after each operation. Across N members this adds O(N^3) validation work. This is offline full-state audit replay, not the D6.7 movement-frame kernel, and no runtime timing failure was measured. The source lead is Medium; the full-state snapshots already require at least quadratic total work for N captured members.

Choice: repair before commit with a small context containing the validated request and next_index. Require the exact next unit ID; advance only after complete modeled-state equality succeeds. Drop the context when the cursor reaches the validated member count. Preserve interruption, truncation, membership, order/target and cleanup checks. Remove the repeated prefix/progress completion scans; no broad validator rewrite, new serialized state or claim of universal linear replay. Existing strict positive/negative controls must remain green.

Evidence: RUN50 c117-source-review2 identifies new loops near economylog.rs395,3968,4328; builder confirms the per-member nested scans. Alternative not taken: label an unmeasured offline cost lead as a movement-frame High, defer a small avoidable cubic loop, or relax full-state comparison. This is a Medium repair decision under brief4 and C112/C118. No CEO stop.50/170 runs; Task6run8/6.


C117-C119 repair committed as570a1593, with equivalent lint-only follow-uped793bb4635cb529ec10e1b809d0c9b9648bd717. Four explicit paths: movement terminal follow-result retention, captured successful follow cleanup (including self/no successor), strict per-member replay context with exact contiguous membership/complete-span checks, and an indexed replay cursor. Sim focused6/6 and strict lifecycle/forged-negative controls passed before commit. RUN50 final source disposition0knownproductionHigh and independently verified the committed/lint delta; runtime recheck pending. Official native helper on cleaned793bb passed10/10 commands, workspace107summaries/1211passed/0failed/18ignored and unchanged frozen values. Root receipt main t6-root/native-ed793bb-root-check.json. A reported1207count was corrected from raw logs; that count belongs to the earlier7898candidate. New-head ACC is running; new-head WEB/C105recheck/20selectednativeGREF/plants/finalreview still required. Existing7898greens remain historical.50/170 runs; Task6run8/6; no CEO stop, integration merge or push.

## C120: bind each gate to its checkout and repair only contaminated local artifacts

Conflict: the first ed793bb acceptance command used the approved Task6 target but ran the main checkout/script. Raw build paths identify /home/julien/Projects/TuRTS; filtered economy test counts and the repeated convergence run also differ from the MC lane. That exit0 is not candidate acceptance. The first corrected-cwd run then failed before tests because cached pre-MC API/protocol release artifacts lacked the MC exports. This is setup/source attribution, not a product semantic red.

Choice: reject both attempts as acceptance evidence. Use the evidence-only gate-runner/run_gate.py, which pins the actual subprocess cwd, clean expected HEAD, script realpath/hash, five explicit Cargo environment values, unique outputs and actual exit status. Run corrected ACC/T7/M3 from the lane. With all compilers quiescent, clean only the 11 local workspace packages release artifacts, keeping the third-party cache, native profiles, source and evidence. Actual cleanup removed222files/920.2MiB; metadata proves all manifests are in Task6. Native gates previously run from the verified helper remain valid on unchanged source and do not repeat solely for cache repair. The new ACC build now reports correct T6 source paths. No extra cache.

Evidence: run49/acceptance-ed793bb/G-ACC.log (wrong source), run49/acc-ed793bb-verified-cwd/{receipt.json,gate.log} (setup compile failure), main t6-root/release-cache-repair-ed793bb/{metadata.json,receipt.json,clean.log}, and gate-runner/run_gate.py. RUN50 independently reviewed the runner. Alternative not taken: accept a main-checkout result as Task6 proof, weaken imports/tests, clean every dependency, use another large target, or ask the CEO about reversible cache handling. Cache/task execution decision under brief4. No frozen value or player behavior changes; no CEO stop.50/170 runs.


Correct-source normal gates on clean ed793bb are now all verified: native10/10 (1211/0/18), corrected ACC16/0 with explicit lane cwd and no main compilation paths, T7oracle1/0 and136frames53moving, M3actual3acceptedactions/7full-canvas pairs/cleanup0. Root receipt main t6-root/normal-gates-ed793bb-root-check.json. Wrong-source and stale-artifact attempts remain rejected setup evidence. RUN50 renewed C105 in versioned preliminary-runtime-ed793bb4; all21 external probes and191 focused Rust tests passed, zero known open Highs. C117 actual same/distinct member slot arrival is consumed next Orders without body movement; self/no-successor passes. Supporting cost medians5000:spread3.337ms,dispersed5.647ms,occupied-spare10.033ms; not Task22 acceptance. Source stayed clean/unchanged; reviewer explicitly returned the quiescent lease at2026-11-01T04:16:11Z. Root authorized active RUN49 to execute all20selectednativeGREF and required precise plants. First6/18M2 cases are restored-green;2M3 cases and final independent ownplant/PASS remain.50/170 runs; Task6run8/6, no merge or push.

C120 cache handoff follow-up: when this cache moves to another worktree, invalidate only local workspace package artifacts in the needed profiles before building there, while retaining third-party cache/evidence. This prevents stale embedded manifest paths or old API exports from being reused across checkout roots. Verify actual build source paths as well as git HEAD and target. No extra large cache is needed.

## C121: use the sensitive final-head member-ID counterplant and state its scope

Conflict: root asked for C97 distinct valid Move IDs and mentioned disjoint unit types. The existing final-head test merged_group_spread_keeps_each_member_order_identity uses two distinct accepted definitions with one mobile type. Collapsing per-member order data to the first anchor produced a compiled semantic current-order failure for actor2; exact restoration returned green. Changing that fixture after normal gates solely to add another type would repeat all gates without improving detection of this specific ID-collapse fault.

Choice: accept the actual sensitive final-head C97 counterplant for per-member ID preservation. Record that the fixture uses one type; a combined disjoint-type/definition runtime fixture is NOT TRACED here. Keep separate type-authority tests and the source-verified type-independent member map; do not claim the combined fixture passed. No acceptance row or design rule is removed. C97 still requires preserving each accepted definition for every valid member.

Evidence: run49/plants/C97-member-order-identity/ baseline0/semantic101/restored0 on the exact orders::tests::merged_group_spread_keeps_each_member_order_identity path; the earlier untyped empty-map compile failure is setup-only. Alternative not taken: count compilation as a red, invent disjoint-type evidence, weaken the ID assertion, or edit a gated test to chase redundant mutation coverage. Instrument/coverage decision under brief4; no CEO stop.50/170 runs.

## Task6 complete and Task7 continuation

Task6 final review PASS ed793bb,0High;21externalprobes/191focusedtests, all20GREF/87commands,8buildercycles/27selectedruns and reviewerGM40own0/101/0 with exactrestore. Root merged450c023, verified identical engine/script/test/mod source, moved the sole cache atomically to integration and cleaned only local package artifacts across profiles. Integration12/12commands passed:1211/0/18, allQA/frozen exact,T7and7M3controls. Root receipt main t6-root/integration-450c023-root-check.json. Task6evidence preserved off-lane; remaining3.95GB generated evidence compressed losslessly to4.22MB with CRC check, manifest and archiveSHA58282c8c7de262936e831db5f75ad5bfd25d1a4d086b9c5160d174755920bff4. Clean merged worktree removed only after proc cwd/exe/fd/maps checks; cache preserved,~3.77GiB physically freed. No Claude-prefixed file read or archived.

T7/T11/T12 rebased onto450c023;10dirty prepared paths preserved byte-for-byte with explicit-path stash backups retained. Receipt main wave5-after-t6-rebase/receipt.json. RUN51 registered before new Luna/high T7 continuation;51/170 total, Task7run2/4. Wave5stillneedsT7/T11/T12; push/readCIafterwave. No main merge or push.

RUN52 registered before Sol/high network-engineer review. Source-only contract preparation may overlap RUN51; only RUN51 owns the compiler/cache.52/170 runs, Task7run3/4. RUN51 has compiled K27 and exact-order-log semantic reds; K25/implementation remain active. No new CEO stop.


## C122: reject command-script tick zero before installation

Conflict: the new script loader accepts tick0, but the fixed schedule advances Tick before applying script entries. A tick0 entry would remain unexecuted without an error. The plan does not define tick0 behavior.

Choice: reject the whole script at load time if any entry has tick0. The error identifies the file, entry index and invalid tick. Validate before installing the Script resource or its system. Keep the existing first executable tick1; do not retime an entry or silently drop it.

Evidence: crates/turts_server/src/command_script.rs:23-40 loads and groups entries before installation; :44-47 removes only the current tick. crates/turts_sim/src/plugin.rs:93,115,118 orders Advance before Commands, and :157 implements the tick advance. This is an instrument detail under brief section4, with no K, frozen value or player action change. Alternative not taken: silently lose tick0, move the simulation clock, or reinterpret zero as one. RUN51 implements the loader negative; runtime proof remains NOT TRACED until it runs.52/170 runs; Task7run3/4.

RUN53 registered before T11 continuation: independent source/test preparation overlaps Task7 gates/review, with no compiler/browser/cache use until root hands off the lease.53/170 runs; Task11run2/4. Production implementation waits for its compiled native and actual browser reds.


## C123: migrate the existing E8 plant anchor without changing its fault

Conflict: Task7 adds RateLimitBuckets initialization. Existing E8 replacement1 still anchors the old ReliableTraffic terminator, so it matches zero times. Its other three anchors each match once. Running that plant unchanged would fail setup instead of testing Ack/rejection order.

Choice: after the active clean-head ACC finishes, update only the E8 anchor and its replacement to retain RateLimitBuckets initialization. Preserve all other replacement bytes, pre-commands, test command, expected named failure and guards. Commit the registry separately, run affected QA and both selected GREF cases, then final plants. Do not retire the plant. Carry f9cca40 native/ACC production evidence to the registry-only candidate only after exact production-tree equivalence is verified; record both heads. Full integration re-gating after merge remains required.

Evidence: tests/m2/refutations.json:670-680 defines E8 and the stale initializer anchor. crates/turts_server/src/session.rs:108-114 adds the bucket resource. Root counted E8 anchors1,0,1,1 on clean f9cca40. This is an instrument applicability ruling under brief4, preserving the fault and expected red. Alternative not taken: count setup as red, remove E8, drop bucket initialization from the planted program, interrupt a valid ACC run, or repeat unchanged production gates for a registry-only edit.53/170 runs; no CEO stop.


## C124: log current order outcomes without replaying failure history

Conflict: orderlog maps any Idle row with last_failure to Failed, even if the failure belongs to an earlier order. Append can start a new move without erasing that history; after the move ends, the logger can report the old failure again. Blocked activity can also arise from physical movement while unrelated failure history remains.

Choice: Idle becomes Failed only for a failure stamped at the current fixed tick. A later successful move must finish as Done even when older failure history remains. Keep each terminal outcome stable without repeated rows. Blocked activity always uses the stable reason blocked, independent of historical failure reason or age; do not produce a reason-only transition to '-' on the next tick. This matches the exact design/plan log example. Preserve simulation history and audit behavior.

Evidence: crates/turts_server/src/orderlog.rs:61-88 reads any retained failure. crates/turts_sim/src/orders.rs:3055-3063 permits Append into an idle current slot. crates/turts_sim/src/economy/views.rs:531-600 derives physical/terminal Blocked independently of historical failure; UnitOrderFailureView at:168-172 includes its tick. RUN52 supplies an installed logger regression; actual compiled red and real Append path remain required, not assumed. This is a logger instrument/mechanism repair under brief4. Alternative not taken: erase sim history, report stale TargetGone as a new failure/block reason, or change the reason on an unchanged blocked unit.53/170 runs; normal repair, no CEO stop.


## C125: validate the logger repair by impact and retain exact earlier controls

Conflict: H4 changes only server orderlog.rs and tests/orderlog.rs after full native/QA,17-case ACC and two GREF controls passed. Repeating unrelated simulation and long no-order-log process scenarios adds no coverage of this flag-only repair. The CEO approved removal/grouping of redundant checks.

Choice: commit the two-path repair, then run fmt, exact workspace/all-target Clippy with -D warnings, the cached wasm check, all nonignored server tests, and all G-QA checks. Rebuild the actual release server and run the exact ignored command-script process acceptance (selected1), which enables the logger. Carry the other16 ACC cases and unrelated workspace results from f9cca40 only with exact all-other-path equivalence. Carry the two GREF controls from196ec8d only with unchanged session, registry, command test and provenance paths. State both original and repaired heads; do not claim the whole ACC/workspace command ran on the new head. Reviewer must verify impact, real Append regression, runtime controls and own postgate logger plant. Full integration re-gating after merge remains required.

Evidence: RUN51 limited diff crates/turts_server/src/orderlog.rs and crates/turts_server/tests/orderlog.rs; the loader installs only with --order-log, orderlog.rs:19-23. scripts/acceptance.sh final exact command-script case is the scenario that enables that flag; tests/command_script.rs:42-53 supplies it. The frozen stream, movement, session/buckets and other acceptance paths are unchanged. Alternative not taken: omit affected logger/process tests, discard valid old controls, label old commands as new-head execution, or repeat hundreds of unchanged simulation seconds. Instrument/test/cache decision under brief4 and approved test grouping.53/170 runs; no new run or stop.


C124/C125 evidence clarification: RUN52 actual Point-Append probe confirms movement allocation clears last_failure before terminal completion (orders.rs:2012). The real Append test is a valid green lifecycle control, but does not prove stale failure survives to completion. H4's compiled red is the installed-logger completed-view input regression. Natural stale-terminal reachability is NOT TRACED; do not claim a proven live game failure from the Point case. The logger repair preserves correct state semantics for supplied history. Reviewer ownplant must name exactly which probe detects it. Two reviewer failed fixture attempts remain setup evidence, with no plant-red credit. Root also corrected immutable RUN51handoff ACCignored9 typo: actual17/0/0 across9summaries. Errata main t7-root/RUN51-handoff-errata.json; original report kept.


Task11 RUN53 actual compiled reds: corecamera21selected,5passed16failed, exit101; clientcamera4selected,1passed3failed, exit101. Root read raw outputs/exit files, excluded an earlier zero-selection client filter; rootreceipt main t11-root/native-red-read-check.json. ActualCAM1 browserred pending, no greenproduction yet. RUN54 source-only startup review traced Welcome before Snapshot and Replica.snapshot_tick as readiness; valid empty-owned snapshot fallback must work, later resync must not resetcamera. Seven independent Core probes prepared outsidebranch, NOTTRACED untilreviewlease. Ledger verified54 consecutiveunique runrows.

Disk cleanup complete outsideactivecache: losslessarchives retain28913 completedtmp files and19 copiedF11artifactfiles. zstdCRC and tar source-byte comparison exit0; no activeprocessrefs; inventory+SHA+mapping in main t7-root/completed-artifact-archive/receipt.json. Raw2.2GiB fileblocks replaced by~12MBarchives; no source/evidencecontent lost. Originalcachetmp staging and copiedF11directory removed onlyafterverification; immutable reports' oldF11paths map to f11-evidence.tar.zst. ActiveT11cache unaffected; third-partydeps retained.


## C126: wait for the first accepted snapshot before Core construction

Conflict: camera OwnedCentre must use the player's actual units. Welcome creates a replica with no snapshot, but the existing adapter can construct Core from that empty frame and latch the map-centre fallback. The Task11 owned file list omits core_adapter.rs, where first construction happens.

Choice: extend Task11 scope by a narrow initial-construction guard in core_adapter.rs. Create the first Core only after Replica.snapshot_tick is Some, with authored camera configuration set before construction. A valid accepted snapshot with zero own units must still create Core and use the OwnedCentre fallback. Do not use a nonempty-unit test, gate an existing Core on later resync, or reset an existing camera. Add actual whole-client lifecycle controls for Welcome before snapshot, owned and empty accepted snapshots, and later recovery.

Evidence: design section7 requires OwnedCentre and mod start selection. core_adapter.rs:520-531 constructs Core once and reads the initial_pose fallback; ingest.rs:394-404 creates the replica on Welcome; replica.rs:120,156 starts snapshot_tick asNone, and :270-272 sets it only after accepted commit. Root read all three paths. This is a necessary same-shape startup mechanism under brief4, with no K, frozen value or acceptance-row change. Alternative not taken: latch the pre-snapshot fallback, wait forever for a nonempty owned set, re-centre after each resync, or reset Core state. RUN53 implements; RUN54 independently checks.54/170 runs; no CEO stop.


## C127: apply player pan speed to incremental middle drag

Conflict: the early camera draft uses a fixed ground anchor for middle drag, but applies the player scroll-speed setting only to key and edge pan. Design section7 states the pan-speed rule without a middle-drag exception. A fixed-anchor correction multiplied repeatedly would also depend on frame partition.

Choice: apply the player scroll-speed multiplier to all three pan modes. At speed1, middle drag keeps direct ground-grab behavior. At other settings, scale incremental projected ground deltas. Projection supplies height scaling. Do not multiply the remaining error from one initial anchor on every frame. Add speed1 anchor, speed2 distance and equal-input/frame-partition controls.

Evidence: design section7 pan rule; early interaction/camera.rs:53-66 applies anchor minus current ground directly, while :131-136 applies player speed only in the key/edge path. Root read the draft. RUN54 independently prepares signed-yaw speed and frame-partition probes outside the lane. This is implementation of the existing pan rule under brief4, with no new setting or key. Alternative not taken: silently exempt drag, make speed depend on pointer event frequency, or repeatedly amplify an initial-anchor error. Runtime proof is NOT TRACED until the revised source runs.54/170 runs; no CEO stop.

RUN55 is registered before Task12 continuation. Its source/test work overlaps Task11 implementation and source review. It has no compiler, browser or cache lease. The lane is clean at ffff0a3 after rebase; all four accepted HUD1 instrument files match their prior bytes. Receipt: main target/mc-evidence/t12-root/rebase-after-t7/receipt.json.55/170 runs; no new stop condition.


## C128: make camera comments and the pre-snapshot test describe the final code

Conflict: RUN54's Low source finding identifies four comments that still call implemented camera functions preparation stubs. The configuration test name implies a ready pose before the adapter, but asserts None before a snapshot; its fixture is Point while its assertion text says OwnedCentre.

Choice: repair the stale comments and rename/reword that test to state the first-snapshot requirement for its actual Point fixture. Keep the stronger whole-adapter owned/empty/recovery tests. Wait for any active browser row to finish before editing. No production behavior or assertion is removed.

Evidence: crates/turts_client/src/camera.rs:33,75,87,105 stale comments; :223 test name and :248 message, verified by root. This is a Low repair under brief4. Alternative not taken: leave misleading completion/readiness documentation, claim this fixture proves the whole startup order, or replace the stronger lifecycle controls.55/170 runs; no stop.


## C129: remove inline-canvas overflow before judging the edge band

Conflict: CAM1 arrows passed, but its right-edge stage had zero camera-matrix change. At the required1280x720 viewport, page scrollbars covered x1265-1279; the driver point x1278 hit the scrollbar rather than the canvas. The existing fixed wrapper and inline canvas create overflow. Enlarging the test viewport would leave the product obstruction and a later coordinate debt.

Choice: extend Task11 by adding only display:block to #turts in crates/turts_client_web/index.html. Keep the fixed wrapper and CAM1 viewport/assertions unchanged. Preserve widget-hit suppression. Task12 still owns changing the wrapper to full viewport size.

Evidence: index.html:8 fixed wrapper/style; pre-fix browser-green/03-right-edge-band.png and RUN54 cam1-first-green-attempt-inspection.json show scrollbars and matrix delta0. Root viewed the failed screenshot. After the one declaration, both actual headed Vulkan runs in main t11-builder/run53/browser-green-c129 passed CAM1; root read raw PASS markers and viewed the second screenshot, which has no scrollbars. RUN54 c129-real-row-inspection.json records matrix deltas0.45710/0.45827 and actual exit0. This is a same-shape layout/input repair under brief4. Alternative not taken: enlarge the window, remove widget suppression, weaken the edge assertion or defer a required camera row.55/170 runs; no stop.

## C130: use visible fixture silhouettes and captured cursor state for HUD4

Conflict: Task12 must prove five cursor contexts before Task13 supplies slot-colored materials and own/foreign rings. Current rendering uses one material. Also the current frame record has no cursor history, and Muster has no interface target image entry, although its explicit move order declares a cursor asset.

Choice: use a dedicated muster_hud4 test fixture with isolated, visibly distinct primitive silhouettes for own attacker, own worker, foreign target and node. Reuse Muster behavior/order/asset data; leave production Muster and Task13 visual work unchanged. The driver uses screenshots only. The oracle must verify actual hits/selections and public owner/capability data from captured debug/scene snapshots, and reject a wrong-role/ownership control. Role claims or fixed coordinates alone are not proof.

Use the C84 checkpoint instrument already on Task11: capture_scene_checkpoints records the full debug cursor alongside scene at each screenshot, keyed to input_call/window. HUD4 reads these through view.serverFile. No new CursorFeed/FrameRecord history is needed. In explicit target mode, context is target and the image uses the selected OrderDef.cursor override, with interface target/default fallback as applicable. Muster move already declares muster_cursor_move; no new production asset is required. Keep resolution generic and data-driven.

Evidence: design13.4 assumes slot colors/rings, while plan Task13 supplies them after Task12; design13.2 and HUD4 require actual context/image/applied CursorIcon. Task11 tests/browser/harness/scene-checkpoints.js:14-21 captures full debug/scene; tests/browser/harness/run.js:140-150 persists it. Current Core UiAction emits declared cursor requests at crates/turts_interaction/src/lib.rs:184-187; crates/turts_client/src/core_adapter.rs exposes CoreRuntime.effects():97-99 and CoreEffects is typed. mods/muster/orders/order_move.ron:1 declares the image override; mods/muster/interface/hud.ron:13 lacks a target mapping. crates/turts_client/src/debug.rs:164-177 serializes public ownership. This is a fixture/instrument substitution under brief4, keeping all five acceptance roles and real applied cursor checks. Alternative not taken: wait across the wave dependency, claim ownership from equal-color pixels, read debug in a driver, invent cursor history, add a content-specific engine name, or omit a context. Native/browser proof remains NOT TRACED until it runs.55/170 runs; no stop.


## C131: share the restored CAM1 row with independent pixel review

Conflict: Task11 requires a PointerLeft native/browser fault and restored green, while the reviewer must independently execute the same clean CAM1 row. Running two identical restored rows on the same bytes adds no distinct observation. The CEO approved grouping redundant tests.

Choice: after all normal gates and unchanged GREF guards pass, RUN53 runs the PointerLeft native and real CAM1 reds, restores exact source bytes, and runs the native restored green. It returns the clean lease with restored CAM1 explicitly pending. RUN54 runs the unchanged full CAM1 row first, before installing probes or another plant. Those two clean browser passes provide both the PointerLeft restored-web green and the independent pixel review, with exact candidate/hash/profile attribution. RUN54 then runs its focused probes and own native plant. Complete every required observation before merge; do not claim RUN53 executed RUN54's row.

Evidence: plan Task11 step4 requires native and CAM1 failure under ignored PointerLeft, then restore. The reviewer contract requires independent tests and one plant. Both clean CAM1 purposes use the same committed candidate, row, viewport and approved browser profile. C131 changes execution grouping only; registered GREF baseline/plant/restore guards stay unchanged. Alternative not taken: omit restored browser proof, omit independent runtime, count a pending check as PASS, or repeat an identical clean row unnecessarily.55/170 runs; no new dispatch or stop.


## C132: establish HUD1 selection from pixels and judge the same captured state

Conflict: the accepted source-only HUD1 instrument assumed exactly one unit component, but the real Muster fixture has many. A dense component at DPR2 was not a selectable unit. An adaptive candidate oracle also accepted any rendered selection without ownership and could combine an earlier selected scene with final UI state.

Choice: the driver may derive a selection box from visible unit-material pixels and use the existing owned-box mechanism. No debug/scene query enters the driver. The oracle must verify real dispatched input, a post-drag screenshot identified by actual input-log order, and a nonempty rendered/replicated selection owned by the actual joined slot. Read the legacy-panel check from that checkpoint's debug.ui_snapshot alongside its scene/debug; preserve useful final selection/phase consistency checks. Recognition retries must not use a fixed screenshot ordinal. Keep all three viewport/DPR cases and strict1px assertions.

Evidence: HUD1 driver’s old single-component requirement and actual failed DPR2 clicks; hud-oracle.js:99-127 previously checked only replicated/rendered IDs. crates/turts_client/src/debug.rs:95-98 clones UiSnapshotFeed into ui_snapshot and publishes it in debug JSON. C84 already captures it. Design8.5 requires an owned selected card, not a unique click target; the existing core box filter is an available pixel-driven mechanism. This is a fixture/instrument repair under brief4. Alternative not taken: treat an empty selection as red evidence, trust a driver role claim, read a bridge in the driver, or combine different selected states. Source/pure-Node checks do not replace actual browser proof.56/170 runs; no stop.

## C133: apply the planned canvas sizing prerequisite before DPR2 widget red

Conflict: Task12 Step1 needs a valid selected HUD1 fixture at all three viewports, but the existing fixed canvas reports logical640x360 at device scale2 while screenshot/input coordinates are CSS1280x720. The sizing repair is listed in Task12 Step2, so the intended widget red cannot be reached faithfully first at DPR2. Root read four actual C84 records with window_logical_size[640,360], viewport[0,0,640,360], scale_factor2.

Choice: implement only the already-owned R7 canvas/window sizing prerequisite while widget runtime remains stubbed and legacy UI/tests remain present. Then rerun the actual HUD1 baseline. Require logical1280x720 at DPR2 with the proper physical render target and unmodified CSS input coordinates. Preserve the old mismatch as precondition/legacy-sizing evidence. Keep native defaults unless a concrete shared repair is necessary; do not divide driver coordinates or relax C86/DPR assertions.

Evidence: design13.1:1146 defines device scale2, and design10.3 requires a viewport-filling canvas. Actual main t12-builder/run55/browser-hud1-{ground-box,cluster-box,checkpoint,box}-probe records confirm the mismatch. Plan Task12 assigns canvas sizing to its Step2. Design wins on this instrument/mechanism dependency. Player UiScale remains separate; later SH1 verifies menu scale changes and native widget geometry must handle it. Alternative not taken: label failed selection as widget red, distort pixels, weaken the viewport assertion, or implement/remove widgets before their meaningful red.56/170 runs; no CEO stop.


## C134: match Chrome physical scale to each MC context DPR

Conflict: at context DPR2, headed Chrome reports CSS1280x720 and devicePixelRatio2, but ResizeObserver.devicePixelContentBoxSize is1280x720. winit trusts this box as physical pixels. Bevy therefore publishes logical640x360, and the unchanged pixel driver cannot establish the required selected fixture. Both experimental startup resolution seeds are overwritten by the resize event.

Choice: remove only the failed startup seed/API experiments, preserve the planned responsive CSS, and set --force-device-scale-factor to the same effective DPR as browser.newContext for each harness invocation, including1. Reject conflicting or duplicate explicit flags; record the effective DPR, viewport and arguments. Keep all row, coordinate, selection and oracle assertions. Add pure profile tests and repeat the full unchanged HUD1 matrix before widget implementation. Do not use a global force2 flag.

Evidence: main target/mc-evidence/t12-builder/run55/dpr2-device-pixel-box-probe.log records the wrong physical box. The matching force2 probe reports2560x1440; actual hud1-dpr2-force-profile/runs/run-20261101T072921Z-DZjDvu records logical1280x720, scale2 and two selected owned units, then reaches the intended missing-widget/present-panel oracle red. Local winit-0.30.13/src/platform_impl/web/web_sys/resize_scaling.rs:231-263 trusts devicePixelContentBoxSize; bevy_winit-0.20.0/src/state.rs:1065-1068 applies it. The primary report https://github.com/microsoft/playwright/issues/18591 describes this same emulation mismatch. Task12 tests/browser/harness/run.js:42-84 now derives and validates the launch profile; launch-profile.test.js supplies five pure controls. The complete real six-run baseline is still running, not claimed green or complete.

This is an instrument/profile repair under brief4, with no player-visible design or acceptance-row change. Alternative not taken: distort driver coordinates, patch ResizeObserver, retain ineffective product startup seeds, relax DPR geometry checks, or apply force2 to DPR1 contexts.56/170 runs; no new dispatch or CEO stop.


## C135: prove guide close through authored button content

Conflict: Task12 requires guide close-button handling, but the existing Muster guide is a Text widget and supplies no close Button. The engine must not invent HUD layout or text. Forge guide content and its close-button pixel acceptance belong to Task17.

Choice: Task12 implements the generic authored Button Input(toggle_guide) path and GuideOpen visibility. An installed-runtime test supplies a small authored interface with a close button, dispatches and handles the button interaction, and checks guide state and rendered visibility. A direct toggle helper call alone is insufficient. Keep production Muster content unchanged for this proof. Task17 supplies the actual Forge guide button and proves its HUD2 pixel path as planned.

Evidence: plan Task12 Files at:542 requires the guide close button; Task17 Step2 at:617 requires F1 and the button to close Forge's guide. mods/muster/interface/hud.ron:10 defines the guide as Text(Bind(Guide)); no close Button is present. Design8:719-721 and8.1:726-727 assign HUD layout/content to mod widgets, and design8.4:794 defines Input actions for buttons. This is a fixture and ownership decision under brief4; no player-visible step or acceptance row changes. Alternative not taken: draw an engine-owned close button, claim a nonexistent authored control was exercised, omit installed dispatch proof, or move Forge's pixel acceptance into this lane.56/170 runs; no new dispatch or CEO stop.


Disk pass2026-11-01:24 completed economy.jsonl logs (14,598,330,399 bytes) are preserved as sibling .jsonl.zst files (18,042,501 bytes). Root checked no open process file references, source unchanged, compression/CRC/decompression exit0, and exact decompressed SHA-256 before each raw removal. Reclaimed13.58GiB; MC evidence now2.3GiB,684GiB filesystem free. Active Task12/cache/source and all screenshots/reports remain. Old raw path mappings and restoration instructions: main target/mc-evidence/completed-log-compression-20261101/README.md; per-file hashes/exits in receipt.json. These are lossless storage changes, not evidence deletion or a new run. RUN55 implements; RUN56's package-local asset collision High is being repaired with a two-package regression before gates.


## C136: implement the asset loader in parallel with the widget tree

Conflict: Task12 has independent image/font asset work and substantial source/tree integration work. Waiting for one builder to write both delays the full runtime, while a second compiler would duplicate cache and violate the standing lease.

Choice: register RUN57, Luna/high, for widgets/assets.rs only in the Task12 worktree. RUN55 keeps sources, tree, module registration, integration and the sole compiler/browser/cache lease. RUN57 writes source and meaningful tests against the agreed qualified WidgetAssets API, then RUN55 executes them with its normal validation. RUN56 independently reviews both. No additional lane or target is created.

Evidence: design8.1:733-737 requires render-only normal images and separate CPU-retained cursor assets. RUN55 supplied the disjoint API before dispatch: qualified image/cursor/font maps, nine-slice metadata, handle accessors and load_widget_assets. Brief4 permits lane/cache layout decisions and the CEO approved safe parallelism. Alternative not taken: run a second compiler, overlap edits to the renderer, reduce review, or add more analysis-only runs.57/170 runs; Task12 run5/8; no CEO stop.


## C137: retain physical UI rows and derive window CSS scene rectangles

Conflict: the first Task12 snapshot draft divides every UiSnapshotFeed rect by a DPR read from the first Window. This changes the existing turts_ui physical-coordinate contract, although design13.2 keeps that bridge. C86 requires only the new turts_scene.widgets rectangles to be window-logical/CSS. A nonzero camera viewport or a different target camera also makes the first-Window shortcut wrong.

Choice: capture actual post-layout widget geometry once. Keep turts_ui/UiSnapshotFeed rects in camera-viewport physical pixels. Derive the new scene widget rects in window-logical pixels using each node's actual target camera, target scaling and physical viewport origin. Preserve player UiScale and all strict row assertions. A separate derived rect field is permitted; do not reuse one differently scaled array for both contracts.

Evidence: design13.2:1171-1175 adds scene widgets and says turts_ui stays. web lib.rs:488-492 declares window-logical scene coordinates and camera-viewport-physical UI rect coordinates; existing bridge contract tests:550,564-565 assert both. Legacy economy_ui.rs:1097-1108 defines the computed UI row. C86 already records the CSS scene requirement. Current Task12 snapshot.rs:12-30 reads the first Window and divides the feed geometry. Root read all paths. RUN56 will independently test actual target-camera/nonzero-viewport cases.

This is an instrument-coordinate repair under brief4, with no player-visible layout or acceptance-row change. Alternative not taken: silently change turts_ui, cancel player scale, trust an unrelated Window, or omit the viewport origin.57/170 runs; no new dispatch or CEO stop.


## C138: complete permitted source semantics in a disjoint module

Conflict: the widget tree/input/cursor integration and the table8.4 source semantics can proceed independently after their action contract is fixed. Keeping them on one writer delays runtime proof; overlapping module edits would risk conflicts.

Choice: register RUN58, Luna/high, for widgets/sources.rs and inline tests only. RUN55 explicitly froze SourceContext/ItemData and supplied the dispatcher JSON contract for Order, Select, Produce(producer/content), CancelProduction(producer/job), and Input. RUN55 owns tree/actions/cursor and all compiler/cache/browser execution. RUN56 independently reviews. RUN57 remains completed; no resumed run or second cache is hidden.

Evidence: design8.4:775-795 defines allowed sources and actions; existing sources.rs is an isolated adapter with explicit replica/catalog/owner inputs. RUN55 confirmed the exact disjoint ownership/API before dispatch. Brief4 permits lane/cache layout choices and the CEO approved safe parallelism. Alternative not taken: change the data surface, let two writers edit one module, run concurrent compilers, skip semantic source tests, or count an uncompiled helper as green.58/170 runs; Task12run6/8; no CEO stop.


## C139: read targeting state from the interaction core

Conflict: the first cursor draft keeps a second pending-order state from raw keyboard/UI requests and guesses an order from a unique cursor image. Two real Muster orders share that image; modifier/action resolution and target cancellation/consumption already belong to Core. A client guess can retain or choose the wrong target mode.

Choice: extend Task12 by one narrow read-only Core pending-order-id accessor. After the actual Core step, cursor presentation reads the authoritative pending order through CoreRuntime, then resolves its package-local override. Remove raw-key request tracking and unique-image inference. Add real-core/installed controls with shared cursor IDs, no-target action, cancellation and consumed targeting. No direct presentation mutation of Core state.

Evidence: interaction/lib.rs:31 stores pending_order; :139 and:202 set it, :162 cancels, :263 takes it on target consumption. Existing public accessors:56-70 expose camera/selection/groups but no pending order. Current Task12 cursor.rs:27-63 mirrors requests and:88-94 guesses from image identity. mods/muster/orders/order_attack.ron:1 and rush.ron:1 both declare muster_cursor_attack with different target kinds. Root read all paths. This is a same-shape readonly integration seam under brief4; no new key, behavior, asset or acceptance row.

Alternative not taken: infer authority from a shared image, duplicate effective-binding/modifier rules, retain stale targeting state, or weaken HUD4 to the unique Move case.58/170 runs; no new dispatch or CEO stop.


## C140: format Guide text from authored page strings

Conflict: production Muster authors Text(Bind(Guide)), but the initial source adapter returns structured item arrays and the Text renderer serializes them as JSON. The guide would show internal field/enum syntax rather than the authored player text. RUN56 M1 requests disposition.

Choice: repair M1. When Guide is a Text/Format value, return readable authored page title/body strings joined by newlines. Keep Source::Guide list rows structured for templates. Add no instructional copy, content-specific rule, page layout or new schema. Empty guide text is empty.

Evidence: mods/muster/interface/hud.ron:10 binds Guide in a Text widget. Current Task12 sources.rs:125 uses source_items_value for Value::Guide; mod.rs text_value/display_value serializes nonstrings to JSON. Design8.2 permits text binding and8.4:791 defines Guide as authored pages; design8:719-721 keeps copy/layout in mod content. This is a Medium repair and uncovered value-format detail under brief4. Runtime readability is NOT TRACED until the real renderer runs.

Alternative not taken: expose item JSON to players, invent engine help copy, alter production guide content to hide the adapter defect, or remove structured List(Guide) support.58/170 runs; no new run or CEO stop.


## C141: send the existing guide key through the same core action flow

Conflict: Core resolves the approved F1 toggle_guide action but its KeyDown dispatch has no matching branch. The initial widget button handler directly toggles guide state and also queues UiAction. A future effect consumer would then toggle the button twice, while F1 remains ineffective. The Task12 file list omits this necessary Core dispatch hunk.

Choice: extend Task12's interaction/lib.rs scope to emit ShellAction(toggle_guide) from the existing key action, as menu already does. Both authored Button Input(toggle_guide) and F1 flow through Core, then one widget effect consumer changes GuideOpen after CoreAdapterSet. Remove direct duplicate button mutation. Prove each action in an actual adapter frame and actual GuideOpen-dependent retained visibility. Preserve the same entity across toggles.

Evidence: interaction/input_map.rs:63 binds toggle_guide to F1; the initial interaction/lib.rs KeyDown menu/cancel branch:154-177 drops the resolved guide action. UiAction's fallback already emits ShellAction. Initial Task12 mod.rs activate_widget toggles directly and queues the action; no CoreEffects consumer existed. RUN56 independently traced both paths. The current repair now adds the guide branch next to menu and a consumer. C135 requires real generic activation and visibility, not just queued-event/state assertions.

This is a same-shape mechanism/file-ownership repair under brief4, not a new key or acceptance row. Alternative not taken: add close_guide to the schema, duplicate key handling in the UI, count a direct observer-only test as the full path, or allow a double toggle.58/170 runs; no new dispatch or CEO stop.


## C142: give each cursor image/hotspot pair its own cache identity

Conflict: accepted cursor declarations may use the same image with different hotspots. Bevy0.20's Winit custom cursor cache key includes image handle/atlas/flip/rect but omits hotspot. Switching only CustomCursorImage.hotspot can therefore reuse the first OS cursor, although the component/feed appears changed.

Choice: load stable distinct CPU-retained cursor handles for each qualified image-id/hotspot pair, including interface, order and nested primitive declarations. Select the exact pair at runtime; repeated lookup of one pair reuses its handle, while a different hotspot has a different cache identity. Refresh/remove variants on package replacement. Keep normal UI images render-only and preserve authored image/hotspot values. Add same-image/different-hotspot and replacement controls; independently inspect actual handle selection. OS runtime remains NOT TRACED until exercised.

Evidence: local bevy_winit-0.20.0/src/cursor/mod.rs:136-149 builds CustomCursorCacheKey::Image without hotspot and takes CustomCached when present; :90-105 reuses cached OS cursors. cursor/custom_cursor.rs:15-33 defines the same key fields. Root read these primary source paths and RUN56 independently found the issue. Current Task12 asset/runtime lookup was image-id-only. Design8.2 Cursor and8.1 require authored hotspots and separate CPU cursor assets.

This is an internal mechanism/cache repair under brief4, keeping the same player-visible cursor design. Alternative not taken: patch vendored Bevy, crop/shift images to alter the key, report component equality as OS hotspot proof, or forbid valid shared-image declarations.58/170 runs; no new run or CEO stop.


## C143: do not invent a command-card cell for an unplaced order

Conflict: the first actual1280x720 diagnostic passes HUD1's outer rectangles, but the screenshot shows overlapping command buttons. Captured order_disable and order_gather both occupy[76,76,64,64]. Disable declares card:None; Gather explicitly owns row1/column1. The generic sequential fallback puts an unplaced order in an authored cell.

Choice: Grid(Source::CommandCard) draws only rows with a valid authored CardCell, filtering before capacity limiting, in initial spawn and retained reconciliation. Do not auto-place None entries. Other Grid sources keep normal sequential flow; an authored List(CommandCard) may still show the complete source. Keep all commands, mod data, keys and typed actions unchanged. Add Some/None, capacity and generic-flow controls.

Evidence: actual diagnostic main t12-builder/run55/browser/runs/run-20261101T082739Z-5gtren/hud1-after-selection-box.png and scene-checkpoints.json; root viewed/read both. Fixture orders/order_disable.ron:1 has card:None and order_gather.ron:1 has Some(row1,column1), matching production. Root receipt t12-root/c143-card-layout.json retains exact rows, authoring and hashes. Design8:719-721 and8.3 assign layout to the mod;8.4 supplies the card-cell field. The diagnostic is not final-source acceptance because its build overlapped edits.

This is an uncovered placement-detail repair under brief4, not a cut of an order or a changed acceptance row. Alternative not taken: invent free cells, overwrite authored cells, rename protected content, or accept correct outer rectangles while inner controls overlap.58/170 runs; no new run or CEO stop.


## C144: pass the declared HUD4 row metadata to the harness

Conflict: real HUD4 recognition now succeeds, but its oracle receives no fixture_catalog, stages, hash or pick epsilon. The launcher currently copies only operational row/window/DPR/capture fields, so it drops the C130 public fixture contract before the harness freezes params.

Choice: copy exactly fixture_tree_sha256, fixture_catalog, pick_world_epsilon and stages from the row into generated params. Preserve runtime URLs/package/window/DPR handling and read-only deep freezing. Exercise the actual Python generator and keep the row/oracle checks unchanged. No source/debug access enters the driver.

Evidence: scripts/mc-browser.sh:120-132 builds params from a field subset; repaired :126-127 adds the four metadata fields. HUD4.json already declares them and hud4-oracle validates them. Actual failed rows reached validateFixtureCatalog with missing params. Root independently ran the old and new production generators at all3 HUD1 viewports: bytes match exactly; HUD4 metadata survives; injected row URL/package fields cannot replace pinned runtime values. Receipt main t12-root/c144-row-params/root-check.json; builder params-contract tests exercise the real heredoc. This is an instrument-field repair under brief4 and C130.

Alternative not taken: invent missing catalog claims in the oracle, drop its role/geometry checks, let row metadata replace server URLs, or rerun unchanged HUD1 solely for these HUD4-only fields.58/170 runs at decision; no new dispatch or CEO stop.


## C145: add isolated authored-widget lifecycle tests in parallel

Conflict: current HUD1/HUD4 passes do not prove the old panel's exact command, pressed-control, job-retirement, scroll and clipping invariants. RUN55 still has source High repairs. The public authored-tree/action seam is now stable enough for disjoint coverage.

Choice: register RUN59, Luna/high, for tests/widgets_lifecycle.rs only after the frozen matrix completes. RUN55 owns runtime/source repairs and the sole compiler/browser/cache; RUN59 writes tests but runs no compiler. Test the real authored tree and public WidgetsPlugin/CoreAdapter where needed. Authored ButtonAction is Order/Input/Item; resolved Produce/Cancel are WidgetAction values and enqueue PendingCommands. Preserve exact producer/job/content identity and actual input/geometry behavior. RUN56 independently reviews all replacements before legacy deletion.

Evidence: RUN55 supplied the public setup/ownership contract and five concrete gaps; legacy28 map explicitly marks partial coverage. Existing tests/widgets.rs has10 small placement/guide/hit cases and cannot substitute for installed lifecycle tests. The matrix is complete before the new writer starts; no source freeze or compiler lease overlaps are hidden. Brief4 permits cache/lane layout decisions; CEO approved safe parallelism.

Alternative not taken: delete old tests with future-only replacements, let two writers edit runtime modules, add a second cache, or call unexecuted tests green.59/170 runs; Task12run7/8; no CEO stop.


## C146: repair the remaining widget Medium and Low findings now

Conflict: RUN56 v005 identifies off-viewport tooltips (M2), Auto/Fit containers with absolute children collapsing (M3), a new package drawing against retained old assets after decode failure (M4), missing documented pan cursor contexts (M5), and pixel conversions outside placement.rs (L1). These are Task12 runtime/instrument gaps, not reasons to change the design or stop.

Choice: repair all five now. M2 flips/clamps automatic tooltips against the actual target viewport with authored content/style unchanged. M3 supports natural Fit containers using real content/child measurement, with nested tests rather than guessed glyph metrics. M4 uses the existing Phase::Failed path on new presentation decode failure and clears/hides old widget drawing, capture, actions and cursor claims atomically; do not mix revisions. M5 gets pan direction/context from the actual camera controller's logical pan state through a read-only seam, using up/down/left/right and diagonal pan_* names with declared/default image fallback; PointerLeft, focus, UI and edge-disable cases must work. L1 routes conversions through placement and makes the required grep clean.

Evidence: immutable main t12-review/run56/source-findings-v005.json and source-pass-v005/ cite tooltip.rs:41, placement/mod absolute-child layout, assets.rs:76 replacement path and cursor context omissions. join.rs:37-46 already provides Phase::Failed. Design8.1-8.3 gives mod style/layout, Fit and pan_* contexts; plan Task12 requires the pixel-conversion grep. Root read the inventory and source. The H6 camera timing repair also follows the explicit plan: cursor derivation/feed in Last before FrameCloseSet and CursorSystems::Update, after PostUpdate camera propagation. Local bevy_winit0.20 cursor/mod.rs:36 registers its processing in Last.

Alternative not taken: defer incomplete planned primitives, claim leaf-only Fit proves containers, show stale accepted bytes, duplicate raw camera binding logic, omit pan cursors, or relax grep.59/170 runs; no extra dispatch or CEO stop.


## C147: resolve item-dependent container tooltips in each row scope

Conflict: D67 and protected Muster put tooltip Item(Reason) on the command-card Grid, while the template Button has no tooltip. The initial renderer resolves it on the Grid with no item and never passes it to the row, so disabled reasons cannot be displayed. Field presence in the source is not rendered tooltip proof.

Choice: for List/Grid, an item-dependent tooltip expression, including nested Format/Item, becomes the template row's default unless that template overrides it. Resolve it in the row's own item context and do not also bind the unbound Item expression on the container. Item-independent container tooltips remain container-scoped. Preserve nested row scope and override behavior. Supply public OrderDef.description in existing contextual ItemField::Body so authored tooltip formats can use name/key/body/reason; invent no missing fields for data types without one.

Evidence: mods/muster/interface/hud.ron:4 places Some(Item(Reason)) on Grid and template Button.tooltip is absent. Design8.5:802-810 does the same in the D67 probe;8.2 permits tooltip on any widget and names description/key/reason content. RUN56 confirmed the parent-grid item=None path in the frozen renderer. Existing ItemField::Body and OrderDef.description provide the needed data; no schema or protected content edit is required.

This is a scope/instrument mechanism repair under brief4. Alternative not taken: change protected Muster layout, render an empty parent tooltip, overwrite nested item scopes, fabricate description text, or call source Reason coverage a rendered tooltip pass.59/170 runs; no new dispatch or CEO stop.


## C148: replace gather panel copy through the designed source boundary

Conflict: the removed panel test combines selected inventory assertions with the exact engine-generated line `Gather: waiting · destination_full`. Copying that line into the new runtime would add an unlisted widget query and engine HUD text. The initial legacy map incorrectly asks to retain that exact line.

Choice: retain the inventory assertions through the actual SelectionPrimary.Inventory source, including both ports and each capacity. Retire only the old panel-copy assertion under the explicit panel removal. The mandatory authored GathererWaiting alert remains owned by Task15 and is NOT CHECKED in Task12. Do not invent a direct gather-status source. Foreign inspection, private-data exclusion and removed-selection pruning still require current source/Core regression evidence; this disposition does not waive them.

Evidence: design8.1:731-733 removes economy_ui and its English text;8.4:775-796 lists the permitted widget queries and excludes direct gather-stage access;9.4:891-911 maps GathererWaiting to GatherStage::Waiting. Plan Task15:601 explicitly requires that derivation. Legacy economy_ui.rs test:785-855 combines inventory with the old exact line. Current widgets/sources.rs:1526 tests both ports and authored capacity; interaction/tests/groups.rs:74-99 asserts removed units leave active selection in the same step. Tests must execute on the final candidate before replacement credit.

This resolves a replacement-test instrument conflict by following the design; it changes no design rule or acceptance row. Alternative not taken: preserve forbidden engine copy, expose a new query, silently drop the wait alert, or claim future Task15 behavior is already green.59/170 runs; no new dispatch or CEO stop.


Task12 removal checkpoint: RUN56 and root cleared the exact28-row replacement map734614b3…b54b5f after v004 native proof. RUN55 removed economy_ui source/test, the module declaration, m3-mobile.js and its ledger row. Root verified remaining legacy ledger is exactly4 rows. All removed tests and replacements/dispositions are listed in today's work log. The three registered panel plants remain NOT CHECKED / planned red window; no retirement. Final source freeze, HUD1/HUD4 matrix, gates, mutation and independent installed review remain pending.59/170 runs,13/25 complete.


## C149: choose actual visible ground outside captured HUD for the cursor instrument

Conflict: final HUD4 added a positive default-context stage after the seven existing stages. Both executions fail because selection is not cleared. Root traced the actual ground click256,201 inside the authored card rectangle8,8,272,204. The pixel helper accepts dark near-neutral backdrop instead of visible green playfield. UI capture correctly suppresses that clearing click; these failures do not prove a runtime cursor fault.

Choice: repair the pixel-only open-ground recognizer to accept actual visible playfield and reject backdrop, unit/node silhouettes and HUD pixels. Preserve the cleared-selection and applied-cursor checks, all prior stages, and the driver/oracle boundary. Add positive/negative pixel controls, then rerun HUD4 against the frozen repaired candidate. The required default-for-every-hover plant must fail a nondefault hover assertion; the added default stage is positive coverage, not a prerequisite for that plant.

Evidence: tests/browser/harness/hud4-pixels.js:86-109 accepts high<150 and spread<45; tests/browser/hud4-driver.js:90-93 uses that coordinate for selection clearing. Both final run completion.json files095246-xorzKn and095313-tBe1CY report exit1 with actual selection1. Their params/input/checkpoints are retained under main t12-builder/run55/browser-final/runs. The six preceding HUD1 completions all exit0; root separately checked profile, CSS PNG dimensions, all three computed root rectangles and absence of engine panel in final-hud1-captured-geometry.json.

Alternative not taken: weaken empty-selection proof, disable actual UI capture, let the driver read scene/debug rectangles, claim these setup failures as the mutation red, or repeat unaffected rows solely for a pixel helper edit. A newly found nested Fit reflow repair remains under C146; if its production path affects HUD1, run the affected rows again.59/170 runs; no new dispatch or CEO stop.


## C150: carry HUD1 through the isolated Fit-axis repair

Conflict: the final-phase6 HUD1 runs pass, but the nested Fit repair changes the common placement helper after that build. Blindly calling those results a new-head run would be false; repeating unchanged HUD1 paths would add no coverage for the repaired nested Fit behavior.

Choice: retain those six actual HUD1 passes as impact-carried evidence with their original source manifest. Run the repaired HUD4 twice and require the installed nested Fit grow/shrink/convergence test plus independent geometry probe. Final committed workspace/QA/WEB gates remain required. This carry expires if further production or HUD1 instrument/fixture changes affect the paths.

Evidence: root matched the original mod.rs SHAce7081bc…9008 to RUN56 snapshot, then diffed it against the repair. The diff only adds per-axis Dimension::Fit guards at the two apply_node_placement calls/helper. False/false keeps the previous assignments. The HUD1 authored fixture has no Fit dimensions; Grid rows resolve to fixed cells. Other changed paths are lifecycle tests and HUD4-only pixels/oracle tests. Root receipts: main t12-root/nested-fit-impact.diff and nested-fit-hud1-impact.json. RUN56 rechecked the source repair; actual independent nested geometry execution remains pending.

This applies the CEO-approved test grouping and brief4 instrument judgment. Alternative not taken: pretend an old-source run executed new source, skip nested runtime proof, weaken a cursor assertion, or rerun all six HUD1 cases solely to exercise a branch their fixture never reaches.59/170 runs; no new dispatch or CEO stop.


Task12 candidate5feedd5548b8c4805c82849395b5368b226c51b2 committed with explicit staging and attribution. Root verified clean lane and unchanged frozen M2 hash. Fresh HUD4 pair exits0 at10:03:00/10:03:28UTC; root independently rejudged both actual captures through immutable oracleSHA4a3f95c8, all8stages/47calls each. Pre/post manifestf7005940 matches all109 live entries; both copied fixtures match63/63 source files. Receipts main t12-root/fresh-hud4-verified and fresh-hud4-source-fixture-binding.json. Six HUD1 passes carry only under C150's exact impact/source contract. Earlier HUD4 selection/input diagnostics remain preserved and are not semantic mutation reds.

RUN55 owns committed locked RUST/QA/WEB gates and the required default-hover mutation. RUN56 independent installed probes and own production mutation remain pending; Task12 is not yet merged.59/170 runs,13/25 complete; work continues without a checkpoint pause.


## C151: retain browser observations through equivalent lint refactoring

Conflict: strict Clippy rejects the initial committed widget candidate5feedd5. The necessary lint repair changes seven source/test files, so source hashes alone cannot label old browser observations as executions of new commit0c96cd7.

Choice: carry the existing HUD1/HUD4 observations with their original source/impact receipts through the exact lint-only diff. Root read every hunk: nested conditions become equivalent short-circuit let chains; Bevy Query tuple/filter signatures become identical aliases; chunks_exact(2) becomes as_chunks::<2>().0 (same complete pairs/remainder handling); clone-to-slice and borrow scopes affect test setup only; allow(too_many_arguments) is scoped to test constructors. No widget, cursor, input, geometry or acquisition behavior changes. Require final locked workspace/QA/WEB gates and independent runtime/plant on0c96cd7.

Evidence: git commit0c96cd73698a82d6271a11103637839284d1fded; root full exact diff saved at main t12-root/lint-impact-0c96cd7.diff. First nine lint diagnostics/exit101 remain in run55/final-gates-v001 and are not semantic red evidence. Strict Clippy now passes; the remaining gates are still running.

This is the CEO-approved grouping with explicit provenance, not a dropped gate. Alternative not taken: claim earlier runs executed the new hash, suppress production warnings globally, or repeat the full browser matrix for type aliases/equivalent conditions alone.59/170 runs; no new dispatch or CEO stop.


## C152: use the declared unit role on harness unit tests

Conflict: G-QA rejects three new HUD4/launch-profile test files because their role comment says test, which is not a supported role. The first attempted header edit temporarily left duplicate role lines; that failed attempt remains recorded. The files are unit tests, not driver/oracle entry points.

Choice: change exactly the three leading comments to turts-test-role: unit. Keep QA rules/limits unchanged. At committed c17d8e8, driver-rule selftest47/47 and enforcement pass; all104 Node harness tests pass. Root read the complete3-line metadata diff. Carry native RUST gates from committed0c96cd7 because no Rust/Cargo/build source changed; carry browser observations with their original source and C150/C151 impact records. WEB/T7 and mutation still run on the current candidate. Each command receipt must name its actual head and exit; no old command is relabeled as a c17 execution.

Evidence: c17d8e8c273329a55234527bc15a630e1845b407 changes only the first comment in harness/hud4-oracle.test.js, hud4-pixels.test.js and launch-profile.test.js. Root exact diff main t12-root/qa-role-impact-c17d8e8.diff. Failed role/duplicate-label logs and final47-case PASS remain run55/final-gates-v002/driver-rule-selftest*.log; actual104-test log is browser-harness-tests.log.

This is an instrument metadata-name correction under brief4. Alternative not taken: raise the ledger limit, classify executable drivers as unit tests, omit QA, discard failures, or repeat all native simulations for comment-only changes.59/170 runs; no new dispatch or CEO stop.


## C153: remove the interrupted default web cache and continue

Conflict: RUN55 started Trunk without exporting the five required Cargo cache/profile variables. It wrote partial host/wasm artifacts outside the sole leased cache, then interrupted before completion. This attempt is not WEB gate evidence. The builder correctly stopped compiler work for root cache inspection; this is a layout decision, not a narrowed CEO stop.

Choice: retain the raw interrupted log and a per-file path/size/time manifest; inspect active Cargo/Rust/Trunk/wasm-opt and all accessible process cwd/exe/fd links. After confirming no references or tracked files, remove only lane target/wasm-release and target/wasm32-unknown-unknown. Preserve target/mc-t12 and all evidence. Resume RUN55 with all five variables scoped on each actual command and explicit cwd. No new run or source change.

Evidence: root manifest covers2834 rebuildable files/875366961 logical bytes, process references empty, tracked paths empty, clean c17d8e8 lane. Main t12-root/cache-environment-incident-20261101 holds manifest.json, pre-deletion receipt.json and completion.json. Raw interrupted build remains run55/final-gates-v003/trunk-release.log. The intended cache is retained; no Trunk/T7 green is claimed from the interrupted run.

Alternative not taken: leave a duplicate cache, reuse unpinned output as proof, delete evidence/source, remove active cache, or ask the CEO to decide routine cache recovery.59/170 runs; no CEO stop.


## C154: batch independent runtime diagnostics before the final repair gates

Conflict: while WEB was running, RUN56 found an additional Fit rows-to-empty defect: a previously measured container skips update when no child measurement remains and can keep ghost capture bounds. Applying each late edge separately before running the prepared runtime probes would repeat expensive builds. T7 also used the default headless profile and failed GPU startup; its inner release build passed, but browser acceptance did not run.

Choice: after RUN55 stops all processes, give RUN56 the sole compiler/cache lease for one finite native diagnostic batch on committed c17d8e8: nested Fit geometry, DPR2/UiScale/nonzero camera/multiwindow geometry, cursor-free replacement, rows-to-empty-to-rows, and three Core controls. RUN55 prepares a repair outside the lane only. Reviewer restores exact source and returns all findings together. Builder then repairs/commits the batch and runs final gates and HUD4 plant; reviewer repeats the cheap native batch and its own production plant after final gates for final acceptance. No formal plant runs before the WEB gate is green.

Use the existing bound gate runner for final T7. It scopes all five Cargo variables, M2_HEADLESS=false, Vulkan GPU flags and ignored software fallback, and records actual cwd/head/exits. Do not improvise another default-profile invocation. Keep the first headless GPU failure as setup evidence, not semantic red. Root read scripts/t7.sh:83-98: the headed path uses xvfb-run and the oracle runs while the server remains alive.

Evidence: RUN55 reports no owned processes and quiescent source; RUN56 explicitly received the sole lease and has seven prepared native probes. The only lane changes during this lease are reviewer temporary tests. Repair sketch d3e278f3…7b93c6 stays outside source. Main run55/final-gates-v004 preserves the headless setup failure and successful inner build.

This is wave/instrument scheduling under brief4 and the CEO-approved grouping. Alternative not taken: skip the empty-container invariant, let two agents edit/compile together, run plants before final gates, call a GPU setup failure a product red, or restart every expensive build for each new diagnostic.59/170 runs; no new dispatch or CEO stop.


Task12 independent C154 batch: root read actual locked-argv/cwd/five-env/head/status/exit receipts and raw logs. Core3/3 pass (exit0); installed3/4 pass with named exit101 r56_actual_fit_queue_shrinks_to_empty_and_recovers_new_rows. Actual3-row size460x56/tops36,40,44 exposed stale retained row Binding.at; expected460x152/tops36,88,140. Empty retained460x56 visible capture; refill2 gave460x52 instead of460x100. Reviewer restored exact original lifecycle SHA7782a617…85eeb, removed temporary Core test and released the lease. Root verified clean c17 and no compiler processes before returning sole lease to RUN55.

Repair77ca599d41b1110263af37516c5c0c2ba34ad1ef updates retained binding geometry and resolves truly empty authored Fit containers to zero axes. Root read the exact15-line production diff and the permanent test's compiled named red; builder reports permanent lifecycle7/7 green. RUN56 source re-check confirms both diagnosed causes repaired. Final fmt/Clippy/wasm/QA green; fullworkspace running. Since shared retained-row handling changed, C150's isolated Fit-only carry no longer applies: freshfullHUD1/HUD4 matrix is required on77ca599, then required default-hover mutation/restoredgreen and final independent7/ownplant.59/170 runs,13/25complete; no stop or main change.


## C155: reject the fixture mutation and register the required production continuation

Conflict: after all eight fresh browser runs passed on77ca599, RUN55 proposed and began changing the HUD4 fixture default-image mapping. Task12step4 explicitly requires production to return default context for every hover. A fixture swap tests a different fault; an assertion failure from it cannot satisfy the required plant.

Choice: root interrupted RUN55, stopped only its verified owned process group730275, preserved the attempted diff/logs with no proof credit, and restored the fixture bytes from committed77ca599. Root checked actual SHA14246814…c99c matches, lane clean, no compiler processes. Register RUN60 before resuming that interrupted builder agent to perform the exact production fault in widgets/cursor.rs, require a named NONDEFAULT context assertion failure, restore exact production bytes and show both HUD4 runs green. No fixture edit is permitted for this plant.

Evidence: plan Task12step4 at:547 says return default context for every hover. cursor.rs:59-88 computes context and optional order cursor, then selected_cursor:89 applies it. Root receipt main t12-root/rejected-fixture-plant-20261101 records the rejected diff, exact restore and no-credit disposition. RUN55 fresh matrix has eight completion.json exits0 at browser-final-77ca599/runs.

Alternative not taken: accept a different plant, claim an interrupted setup run as semantic red, leave protected fixture content changed, or resume an interrupted run without a ledger row.60/170 runs,Task12run8/estimate8; no CEO stop.


## Task12 closeout and Wave5 checkpoint

Task12 COMPLETE at candidate77ca599d41b1110263af37516c5c0c2ba34ad1ef, mergeaf567c4f1afb3875ba8d9f474d10efda236b6032. Native initial9selected/7named semantic failures exit101; HUD1 original6named missing-widget/panel failures exit1. Final candidate RUST/QA all actual0; workspace1306/0/19, Node104/104. Fresh8browser runs all actual0, after the shared retained-row repair; the earlier C150/C151 carries are now superseded by same-candidate observations. Root separately checked every completion/profile, actual HUD1 root geometry, actual HUD4 oracle rejudgement,63-file fixture trees and configured served tar hashes.

Required Task12 production plant forces all contexts and overrides to default in cursor.rs only. Both actual headed runs fail the named nondefault select-attacker move context (command1); exact sourceSHAae02fe39 restores and both HUD4 runs pass0. The rejected fixture-only attempt has no credit. Independent RUN56 final report e05466c5/98fffac3 gives0High/0remaining: Core3/3+installed4/4 actual0, corrected Fit row tops36/88/140, size460x152→0x0→460x100, exact job302 action, camera/DPR/scale and asset replacement. Its own production pending-clear plant selects1, compiles and fails101, then exactSHA c9ced2bf restoration selects1/passes0. Root read the raw logs, actual argv/cwd/five env/HEAD/exits and final source hashes.

Integration re-gate af567c4:10native/QA/frozen commands all0 plus bound headed T7/Trunk/oracle0; workspace1306/0/19. Frozen M2 SHA remainsbcbf72c2206251f635a7c7f5b6c8f573d69a5fc05a5bd77716e98ebfb08e370b. T7 captured137frames/53moving; lag is recorded, not judged. No old-panel M3 browser run is claimed inside the planned red window. The3registered panel plants remain NOT CHECKED; no retirement. Task15 authored GathererWaiting and native-view-proof migration remain pending under their explicit dispositions.

Root moved the idle cache atomically to integration and invalidated only11workspace packages across profiles. Main cache-t12-to-integration-af567c4 holds the receipts. Task12 lane-local510files/28768653bytes are preserved in t12-closeout-preservation/lane-local-evidence.tar.zst (7735221bytes, SHA5f64b008…), with creation/CRC/byte-compare all0. Source branch remains; worktree removed after merge/gates/read/process holds cleared. Main target/mc-evidence retains all builder/reviewer/root proof. No Claude file was read or imported.

14/25lanes complete,60/170runs; Task12run8/estimate8, no overrun stop. Wave5 push/readCI next; continue Wave6 automatically. Main stays721e56e, including the pre-existing untracked disk audit.


## C156: standardize future MC row commands and source receipts

Conflict: earlier ad hoc WEB commands omitted cache or GPU variables, and the final fresh matrix lacked a separate pre-run file manifest (correctly marked false in its receipt). Repeating those command shapes would create avoidable setup failures and weak source attribution.

Choice: prepare a local bound row wrapper, main target/mc-evidence/gate-runner/run_mc_rows.py (SHAbd3eb4f05db970dd77249362ec61e8dfbcd5aff3af195b80b2003ddda09cc494). It requires explicit lane/cache/head/row/unique evidence label, pins five Cargo variables and the accepted headed Vulkan MC profile, records actual argv/env/cwd/exits/raw hashes, and compares pre/post allowed source SHA256 manifests. Clean baseline is required; a production mutation must declare its exact modified paths. It preserves driver/oracle code and IO boundaries. Claude paths are skipped and symlinks are recorded without dereferencing. T7 continues through its existing bound runner.

Evidence: the wrapper syntax and --help exit0; no end-to-end row through this new wrapper is claimed yet. Its first scheduled future row must supply that proof. Current Task12 acceptance uses its actual original runs/receipts plus root source/gate/artifact/fixture checks, not this future wrapper. The builder receipt field named commit_tree_sha256 is a Git object ID, not a SHA256 file digest; read it as the Git tree ID. Root does not relabel it or invent a missing pre-run manifest.

This is an instrument/command detail under brief4. Alternative not taken: change product code to hide setup failures, synthesize historic source snapshots, weaken an oracle, allow concurrent caches, or rerun current accepted rows merely to exercise a new wrapper.60/170runs; no new dispatch or CEO stop.


## C157: trigger the Wave5 pull-request CI without changing source

Conflict: vault-only closeout commits correctly carry [skip ci], but Wave5 must produce a hosted CI read. This workflow listens to main pushes and pull_request; the current integration PR is draft/open, so its synchronization uses the head commit. GitHub's official [skip instructions](https://docs.github.com/en/actions/how-tos/manage-workflow-runs/skip-workflow-runs) say a skip instruction on the pull-request head skips that run. There is no workflow_dispatch trigger in this workflow.

Choice: commit this vault ruling with [skip ci], then add one empty CI-trigger commit without a skip marker and push only codex/mc-integration with an exact force-with-lease for the known prior remote headfb5e2d6454fefd7a69564a1788eaa52d0ee505e4. Source stays identical to the locally gated mergeaf567c4 except vault records. Read the actual PR workflow jobs; do not claim green before completion.

Evidence: .github/workflows/ci.yml:2-5; connected GitHub PR2 is open/draft/unmerged with that remote head; git ls-remote confirms integrationfb5e2d6 and main721e56e. The integration11gates passed before this doc/empty-only step.

This is an instrument command/flag judgment under brief4. Alternative not taken: push main, remove the required vault skip marker, bypass a check, silently treat absent CI as green, or change the product to obtain a run.60/170runs; no new agent or CEO stop.


Wave5 pushed to origin codex/mc-integration atad0eef46c7a3514488ac95efcbb9798795d0668c using exact old-head leasefb5e2d6; main remains721e56e. Hosted run https://github.com/26medias/TuRTS/actions/runs/41300331651 was read through the connected GitHub app: agents124829747745SUCCESS, check124829747567RUNNING (fmt/strictClippy/wasm passed, native tests active), browser124829747692RUNNING (helper tests passed, T7 active). Final job results are not claimed; root continues monitoring. Raw initial/read receipts are main wave5-ci-ad0eef4/*.json. The wave push/read checkpoint is complete; reversible Wave6 work now proceeds while hosted jobs run.

RUN61/62 registered before dispatch. Task5 has the sole local compiler/cache after root transfer. Task13 can prepare independent source/pure-Node instruments and outside-lane implementation patches, but must preserve the old renderer until its actual VIS1/palette semantic red and receives no second cache. These lanes own disjoint Sim/server vs client/render/instrument files; registry merges remain task-number ordered. No test or final CI result is omitted.62/170runs,14/25complete.


## C158: resolve active and queued policies from the immutable audited catalog

Conflict: runtime Order stores identity/behaviour/target but not fire/engage. Adding duplicate policy fields to every active/queued Order would require many fixture literal changes. The design requires authored policy to survive queue activation and strict replay, not a particular internal duplicate field layout.

Choice approved before the stop: resolve by order ID only in the per-match immutable InstalledCatalog; capture the complete typed fire/engage values in CatalogOrderAudit and OrderPolicyAudit; strictly compare command policy to catalog and reject catalog changes. No changing/uncaptured lookup, unknown-ID silent fallback or weaker validator is allowed. Other runtime state affecting combat, including plain Move identity and shot history, must also be audited or purely derived. Preserve the plain Move rule: no UnitOrders.current or extra order operation.

Evidence: orders.rs:436-483 holds current/queued IDs and command policy; economy/views.rs:307 and:1015 captures catalog order definitions. tests/common/accounting.rs:412 already rejects catalog changes. Builder added queue/policy-tamper tests and reports frozen preflights unchanged. RUN63 still has four failing lifecycle/replay/spreading probes in WIP; this mechanism approval is not Task5 acceptance. Alternative not taken: read mutable definitions at activation, duplicate inconsistent partial policies, weaken accounting, or alter frozen expected values.63/170runs; no new dispatch.


## C159: add the typed synthetic-units launcher field

Conflict: Task13 VIS2 requires --synthetic-units4485, but mc-browser.sh:76 launches only config/tick-log and its parameter extractor does not retain synthetic_units. This is an instrument gap, not a player-visible design change.

Choice approved before the stop: allow Task13 to own a narrow optional synthetic_units JSON integer0..u32::MAX (excluding bool), validated before builds and forwarded by shell array as exactly --synthetic-units N. Absent means identical old argv/params; declared values are retained in params; actual server argv is recorded. Positive synthetic count has no tick log because the server disables it. No generic arbitrary server-argument pass-through. Root also accepted the separate palette module as a red-test seam while rendering remains unchanged.

Evidence: server/config.rs:50,157 defines/parses the existing generic flag; ticklog.rs:40 disables its log for positive counts. Root counted maps/field.ron directly:515actors, including460beacons+40troopers (the SEL2cluster); VIS2adds4485. Source-only Node/Python contract tests are reported green; actual Task13 runtime is NOT TRACED. The proposed WebGPU measurement-counter change and two-ring-topology mechanism remain unapproved/unexecuted at the stop.

The prepared bound row wrapper now has SHAd4f50077362c5fa01e76042f2ff74d9fb729bd89764187ee1b5adbe6ef73db1e: it also scopes MC_PIXELS_HEADED=1 and rejects a success without the required2runs/viewport completion records. Its --help/syntax pass; no actual row through it is claimed yet. Alternative not taken: silently SKIP a headed measurement as green, invent5000actors, break unchanged row bytes, or record nonexistent tick logs.63/170runs; no new dispatch.


## C160: move C16 to the effective weapon-range parameter

Conflict: Task5 now uses both grid.within(range) and a local/focus distance<=range_squared guard. C16's old narrow-predicate-only double remained green because the grid filtered candidates first. Moving only the grid predicate would leave the other guard intact.

Choice approved before the stop: keep C16-double-range's named selected1test and double-effective-range fault, but move its single anchor upstream within combat.rs weapon_parameters: range: weapon_def.range, becomes range: weapon_def.range * 2.0,. This feeds both range filters and leaves unrelated avoidance-grid callers unchanged. The edit must remain mechanical and earn actual named semantic red/exact restore before acceptance.

Evidence: current combat.rs:264 builds that parameter; choose_target:281-310 applies the local/focus check and then the grid query. RUN63 independently agreed with the fault meaning. The first remained-green attempt is retained as incomplete evidence. At the mandatory stop, the registry edit and rerun are NOT DONE. No plant is retired and no game expected value changes. Alternative not taken: retire C16, accept its remained-green result, corrupt all movement-grid queries, or count a compile failure as red.63/170runs; no new dispatch.


## STOP-PAL1: required CEO ruling

At2026-11-01UTC, root independently confirmed the fixed design9.1 Okabe–Ito palette fails VIS1's >=20 CIEDE2000 at Machado severity1.0: minima12.25/11.61/10.87. The severity is not specified in the design; no weaker definition is selected to force green. Both independent implementations pass the published2.0425reference. Brief4reserves player-visible/acceptance changes to CEO. All lanes quiescent; no palette/criterion/design change. See [[Reports/2026-11-01 MC palette stop]] for the exact conflict, raw reproducible proof, source/WIP preservation and pending CEO question.

14/25complete,63/170runs. Task5 HEAD7e010c2+8WIP files preserved; Task13 HEAD128ca44clean, production patches outside lane, no cache/runtime. RUN63probes preserved. Main721e56eunchanged. Final Wave5 CI41300331651:agentsSUCCESS;checkFAILUREon old-panel registry anchor FileNotFoundError;browserFAILUREon rebuilt owner-panel selection timeout5000ms. Both are planned Task21migration work, no three-job-green claim. The palette is the mandatory stop; task implementation and runtime remain stopped pending CEO ruling.

## CEO O13: release STOP-PAL1 and resume Wave6

CEO call at mainf050e429b38222c9917314bb23c1568dd7ad816c, design revision14: keep the exact Okabe-Ito defaults. VIS1 requires every pair >=20 without simulation and >=10 under each Machado2009 protan/deutan/tritan simulation at severity1.0. The native test must assert both thresholds; design VIS1 records21.73 normal,12.25/11.61/10.87 simulated. No palette change. Task12-to-21 hosted failures remain in the approved red window, owned byTask21.

Root rebased integration fromedba4a0 ontof050e42 tof9bacec; only upstream research/design/decision files and combined work-log documentation differ, no implementation source changed. Work-log conflicts retain both O13 and earlier history. Task5 rebased from7e010c2 to12ded296; all eight dirty files saved, stashed, restored and SHA256 matched byte-for-byte. Task13 rebased all five commits onto integration; source/candidate evidence retained. Back-up refs codex/mc-o13-pre-* and a named stash remain until merged/proven-safe. Evidence: target/mc-evidence/o13-rebase-20261101.

RUN62 stays active and source-only. RUN64/65 registered before resuming completed builder/reviewer:65/170. Task5 nowrun4/estimate4; no overrun ruling needed. Compiler/browser remains one owner; Task13 no second cache. STOP-PAL1 released; no main merge or push.

## C161: share two ring topologies without per-unit assets

Conflict: design9.1:864 says one shared ring mesh, while9.1:840-843 requires a solid own ring and a foreign ring with four gaps. The prepared Mesh3d/StandardMaterial renderer represents gaps in geometry; one fixed topology cannot represent both with the same per-slot material. Candidate visuals.rs:28-31,190-191 holds two handles and creates each once.

Choice: use two global shared ring meshes, one solid and one with four gaps, and one global shared bar mesh. Keep one unlit material per slot, no per-unit mesh/material creation, and identical visible geometry. This is the nearest existing render mechanism with the same player-visible shape under brief4. Add a native asset-sharing assertion; actual browser performance is NOT TRACED until VIS2. The design is not edited.

Alternative not taken: lose the foreign gaps, build per-unit assets, add a custom shader/material system only to encode the gap topology, or claim the candidate uses one topology. O13 palette/thresholds are unchanged.14/25complete;65/170runs, no new dispatch.

## C162: reject fire policy on Attack definitions

Conflict: content_mc_validation.rs:1328 permits fire on Move, Hold and Attack, but design14 section5.2:488-498 lists fire only for Move/Hold and engage for Attack. combat.rs:151-160 correctly uses fire only for Move/Hold, so a formerly accepted Attack.fire value would be ignored.

Choice: restrict authored fire to Move/Hold. Reject Attack.fire Some(Never) and Some(InRange) with package/file/field fire; keep Attack.fire None and its required approach/fire mechanism unchanged. Add compiled red-then-green negative validation regression plus positive controls. Shipped Forge/Muster Attack definitions already have fire None; no content, frozen value, design or acceptance criterion changes. This repairs an instrument/schema mismatch under brief4.

Evidence: design5.2:488-498, crates/turts_mod_runtime/src/content_mc_validation.rs:1328-1334, crates/turts_sim/src/economy/combat.rs:151-160, mods/muster/orders/order_attack.ron:1. Reviewer RUN65 raised the edge; builder RUN64 owns the repair and all final gates. Alternative not taken: invent Attack.fire behaviour, silently ignore accepted policy, or weaken audit/validation. Actual new regression is NOT TRACED until builder runs it.14/25complete;65/170runs, no new dispatch.

## C163: connect click markers to real local order intents

Conflict: prepared visuals reads Effect::Marker, but the interaction core declares that variant without producing it. Actual clicks produce Effect::Send (interaction lib.rs:683,844); core_adapter.rs:707 enqueues the envelope, commands.rs:243-310 later assigns sequences. The wire UnitOrderRecord has no sequence. A marker based only on selected units or old Idle can disappear before ACK or never appear.

Choice: extend Task13 ownership narrowly to commands.rs/core_adapter.rs. Create marker intent from each actual Send envelope in the click frame, carrying exact units/target/behaviour and a stable local identity. Carry identity through pending/split sends and correlate actual sequences/results without changing the wire or adding sends. Gate terminal replica checks on accepted result tick, so pre-command Idle cannot end a new marker. Then apply the design unit-rest/Blocked rule and60s limit; handle refused fragments and reset bounded local state on session change. Preserve existing trace/repeat/legacy command semantics. Tests must prove same-frame appearance, stale Idle, split results/refusal and session reset; runtime is NOT TRACED until the Task13 lease.

Evidence: design9.1:856-858 and9.2:875-885; crates/turts_interaction/src/lib.rs:683,844,1091; crates/turts_client/src/core_adapter.rs:707-712; commands.rs:243-310; net_protocol messages.rs:131. Alternative not taken: require a protocol order-id change, show a marker after ACK only, listen to an unused variant, infer its units from a later selection, or invent an arbitrary grace delay. This is the nearest existing mechanism for the unchanged visible contract under brief4.14/25complete;65/170runs, no new dispatch.

## C164: fail closed on an unresolved active firing identity

Conflict: server orders.rs:25-32 rejects unknown authored IDs and captures resolved policy, and economylog.rs:2074 validates it against the immutable audited catalog. The lower-level native OrderCommand accepts trusted resolved requests (orders.rs:460-477); test fixtures intentionally use policy None. Combat's active-order lookup (combat.rs:141-160) conflates a missing definition with valid omitted fire, enabling the InRange default for an invalid identity.

Choice: retain the established trusted-server boundary and native mechanism fixtures. Add a defensive firing guard: an active authored order (including the plain-Move identity) must resolve to a catalog definition with matching behaviour; otherwise it emits no shot. Only a found Move/Hold definition with omitted fire gets InRange default. Units without an order keep the exact M3 rule. Prove unknown/mismatched IDs fail closed and a valid omitted policy still fires, with compiled semantic red before the fix. No new wire/rejection enum, command sends, audit bypass or frozen expected-value changes.

Evidence: crates/turts_server/src/orders.rs:25-32,60-70; crates/turts_server/src/economylog.rs:2074; crates/turts_sim/src/orders.rs:460-477; crates/turts_sim/tests/orders.rs:90-119; combat.rs:141-160. Alternative not taken: permissive fallback for missing identity, reject the existing trusted native fixture API broadly, or claim arbitrary manually forged internal state is a valid wire order. Actual regression is NOT TRACED until RUN64 executes it.14/25complete;65/170runs; no new dispatch.

## C165: non-firing policy does not stop elapsed cooldown

Conflict: combat.rs:160-176 early-continues for valid Move/Hold fire Never and AttackPoint engage None before its normal per-tick cooldown decrement at:228. A unit that fired, moved for one cooldown duration with fire Never, then resumed InRange would wait that duration again. Design5.2 defines fire/engage as targeting/firing policy; it does not pause the existing weapon timer mechanism. The no-order M3 transaction advances cooldown independently of a shot.

Choice: repair this Medium finding now. Valid non-firing policies suppress acquisition/shots but keep the existing fixed-tick timer/status transaction. Keep no-order M3 exact and invalid identity fail-closed. Add focused controls for MoveNever, HoldNever, AttackPoint None with a positive cooldown: timer advances, no forbidden shot, and a later valid firing policy does not incur an extra frozen cooldown. Use deterministic ticks, not wall-clock sleeps. No content, design, wire API or frozen expected value changes.

Evidence: design5.2:488-498; crates/turts_sim/src/economy/combat.rs:160-176,228-248; spec mechanism/policy split:180. Reviewer RUN65 independently classified the source edge, root chose repair within brief4 rights. Alternative not taken: silently add time-pausing semantics to Never/None or defer a simple visible timing defect. Actual new regression is NOT TRACED until RUN64 runs it.14/25complete;65/170runs, no new dispatch.

## C166: keep the Job-support plant compiled and semantic

Conflict: Task5step4 says drop the Job arm of server orders::to_sim. Literal removal from the exhaustive Rust OrderTarget match produces a compile error, which brief4 explicitly does not credit as red.

Choice: for the temporary required production plant only, replace the Job mapping with OrderTarget::Job(_) => OrderTargetSim::Invalid. The real Job envelope no longer has supported simulation Job meaning while the match still compiles. The unchanged every_behaviour_has_an_envelope test must select and fail on its Job target assertion; then exact source restore and selected green. Other arms, input fixtures, expected values, wire schema and production behavior after restore stay unchanged. This is a plant implementation detail under brief4, not retirement.

Evidence: docs/superpowers/plans/2026-10-29-mc-player-control.md:562; crates/turts_server/src/orders.rs:49; brief4 red qualification. Alternative not taken: credit a compiler failure, remove the Job test, weaken expected target, or defer the required plant. Actual plant is NOT TRACED until RUN64 executes it after all gates.14/25complete;65/170runs, no new dispatch.

## C167: reject the optional renderer-free visible-marker assertion

Conflict: the proposed extra baseline-marker-native.patch would assert FrameRecord.markers contains a visible move marker in acme_order_app, which has no RenderPlugin. Final VisualsPlugin intentionally does not render in that app, so the assertion would remain false after correct implementation. It is not a valid baseline regression.

Choice: reject before any run and give it no red credit. Required Task13 baseline reds remain the plan's real headed VIS1 one-colour failure and native palette contract failure. Native marker tests assert valid intent/state logic with deterministic time; actual first-frame visible markers require full VIS1 screenshots and matching C84 frames. The rejected optional patch stays labelled REJECTED_BEFORE_RUN in its manifest. No registered plant or acceptance requirement is removed.

Evidence: candidate/baseline-marker-native.patch calls the renderless acme_order_app; candidate VisualsPlugin returns without RenderPlugin. The source was reviewed before a compiler/browser lease. Alternative not taken: count a permanently failing renderer-free assertion as useful red, or fake visibility records in headless code just to make it pass.14/25complete;65/170runs, no new dispatch.

## Task5 gates complete; independent review and Task13 source review

RUN64 finalhandoff at98147ae is clean with allnormalgates0,19GREF restored, requiredJob/rushplants101->0, frozenhashunchanged. Root read rawT7 proof137frames/53moving and actualresult; no all-green CI claim. RUN64 released allholds and completed. Root granted RUN65 sole Task5cache/runtime for independent15andownplant; source/read/deletionhold remains until thatreviewends.

RUN66 registered before dispatch for independent Task13 ux-designer source review (Sol/high); no compiler/browser/server/cache or laneedits. Task13 prep2bcb9ae plus off-lane patch61cb464a…63baf remains uncompiled; requiredbaselineheadedVIS1/nativepalette reds pending root rebase/handoff afterTask5merge/regate.66/170runs;14/25complete.

## C168: use authoritative same-tick marker terminal state

Conflict: the draft changed marker retirement from replica_tick >= result_tick to strict > because an unrun native assertion assumed equal-tick Idle could be stale. Actual ordering disproves that premise. Session admits after Advance before Commands; FixedPostUpdate publishes completed worldN after Orders/Movement; FixedLast outbound stamps resultN and sends its same-tick delta. Client commits unit_orders and last_applied_tick together. Equal-tick Idle/Blocked can be a valid completed order.

Choice: retain original C163 inclusive >= and repair the invalid test. Pre-result-tick Idle holds a marker; authoritative equal-tick terminal state retires it; equal-tick Moving and unresolved fragments retain it. Protect initial click visibility through actual same-frame intent before ACK. No invented tick delay or wire field. The equal-tick High claim is retracted after source trace, not recorded as a production fix.

Evidence: server/session.rs:115-119; sim/plugin.rs:93-150; server/outbound.rs:14,40,60-75; client/ingest.rs:908-988; client/replica.rs:551-565. Alternative not taken: let an unrun assertion define timing, or leave a marker waiting for a nonexistent next tick after completion. Runtime is NOT TRACED until Task13 lease.66/170runs.

## C169: preserve change detection for unchanged health fills

Conflict: the fill loop passes &mut Bevy Mut<Transform> into a helper before checking whether the fraction differs. DerefMut marks the component changed even when the helper writes nothing. This causes avoidable retained-bar extraction work.

Choice: repair this Medium finding now. Check fraction equality through read access first; call the mutating helper only for an actual geometry change. Add a native Changed<Transform> control for unchanged retained fill and changed fraction, with actual child geometry. Unit-body guard and visible output stay unchanged.

Evidence: staged visuals.rs fill update loop and update_health_fill_transform; Bevy Mut change detection. Alternative not taken: claim an unchanged value proves no change-tracking churn. Actual regression is NOT TRACED until Task13 lease.66/170runs.

## C170: make full VIS1 oracle match visible state and waypoint geometry

Conflict: the expanded oracle did not judge its selection-cleared screenshot, counted a few green pixels as enough for a damaged bar, sampled a straight line to the queued target while production draws via the current target, and checked active marker metadata without its saved pixels. These are bounded Medium coverage/correctness findings.

Choice: repair all before Task13 final gates. Check absence of full-health unselected bars in default mode; compare damaged fill width/ratio with actual public health and full reference, plus native retained child scaling; derive the actual current->queued waypoint polyline from C84 and sample those segments; require actual active-marker pixels. Keep physical PNG-only driver targets and existing fixture/acceptance values.

Evidence: staged vis-full-oracle.js:81-90,238-254,284-300; staged visuals.rs:798-809. Alternative not taken: accept a permanently visible/full-width bar, reject a correct non-collinear polyline, or substitute debug metadata for saved pixels. Actual browser checks remain NOT TRACED.66/170runs.

### C170 scope clarification

Actual owner-public unit order DTOs are needed for the waypoint oracle. Task13 may add sorted replica.unit_orders under replicated_world.unit_orders in both debug.rs cached capture and direct rebuild, with native consistency checks. Catalog max health is public reference data for fill ratios. No private server state, prediction, or driver semantic reads.

## C171: prove the offscreen type-selection precondition

Conflict: the required SEL3 no-screen-test plant has no effect if all own same-type units are onscreen. The draft oracle compared visible candidates but did not require an offscreen peer, and the driver did no camera setup.

Choice: require actual C84 own same-type total > visible >=1 at Ctrl-click and double-click, with a valid visible clicked unit. Use physical PNG-derived zoom/pan when needed; preserve the existing Muster fixture, screen rule and required plant. The oracle may inspect actual public C84 state; the driver keeps PNG-only input targeting. An unexercised predicate or remained-green plant is not accepted.

Evidence: planTask13step4/type-selection plant; design4.1/SEL3 visible-only rule; Task13 sel-driver/oracle. Alternative not taken: retire the plant, invent a content change, or call an all-visible fixture proof of screen filtering. Actual runtime precondition is NOT TRACED until Task13 lease.66/170runs.

## Task5 complete and lane closed

Root verified all11integrationgates at e7e0e17 (code mergee214ccc):1419passed/0failed/19ignored in113groups; bound headedT7exit0,137frames/53moving and no browsererrors. RawWEB SHA1ef18235276351259ddf854d771f2886b9ece6077555234698795f340831ed3d. Evidence integration-t5-e7e0e17*, root summary t5-root-98147ae/integration-verified.json. WorktreeTask5clean98147ae had no activecwd/executable references, no cache left, nounique evidence; ignored rebuilt outputs inventory46,924,473bytes. Rootremovedworktree, kept codex/mc-t5/backups/stash/originalWIP/evidence. Receipt t5-closeout-preservation/completion.json.15/25complete;66/170runs.

Root rebased7Task13 commits onto e7e0e17 whileintegrationchecks ran, cleannewHEADca0c36b3a5f8500e9ea99842621adf58a3238a5a. No sourceconflicts; builder/reviewerrefreshcandidatepreimages beforeanyruntime. ThisisGit/sourceoverlap, no parallelcompiler. Wave6push/hostedread waitsTask13completion. Both hostedpanelmigrationfailuresremainTask21window.

## Task13 runtime lease and required baseline reds

Root transferred the sole idle cache integration->Task13 and cleaned11workspacepackages in5profiles, all0; receiptcache-integration-to-t13-ca0c36b. RUN62 owns target/mc-t13 withdev/testdebug0,incremental0,jobs4; RUN66source-only. Nativepalettebaseline atclean ca0c36b3exit101:4selected, published CIEDE reference passes, exactdefault/normaldistance/compositeMachado tests fail againstonewarm-redstub. Actualcombinedtoolstdoutpersistedaftercompletion in t13-builder/run62/baseline-native-palette; receipt honestly says no independentlypersistedpre-commandmanifest, currentcleanHEAD/codehashesverified. Rootreadactualassertions, notsummaryonly.

BoundheadedVIS1baseline active labelrun62-baseline-vis1-ca0c36b3-20261101 usingapprovedC156helperd4f50077…db1e. Source remainsoldonecolourrenderer; actualbrowserresultisNOTTRACEDuntilcompletes. Optionalrendererfreevisiblemarkerassertionwasrejectedunrun underC167. RoutineSystemParam<=16/PrimaryWindowimportcompilepreps remainofflane; a compilererror earns no redcredit.15/25complete;66/170runs; wave6CIpush/readafterTask13.

C168 source timing addendum: CoreAdapter intent/draw are Update; commands sequence/send is PostUpdate(lib.rs132-139), so a new click cannot have a result before its first presentation. Equal-result-tickcommittedIdle is post-command authoritative; firstframevisibilityneedsnodelay.

## C172: bind browser greens to provisional clean source commits

Conflict: the approved bound row helper accepts a clean committed tree or explicit modified tracked paths, but Task13 introduces new untracked driver/oracle/row files. Its full headed green phase cannot run with the exact source guard until those files are committed. The plan lists the first commit after the green phase.

Choice: after the required genuine native/browser baseline reds and focused native greens, permit an explicit-path provisional branch commit to obtain clean source identity for headed green rows. It is not acceptance or merge approval. All required SEL2/SEL3/VIS1/fullVIS1/palette greens remain mandatory; any repair has a new clean commit and bound rerun. Final normal gates and required plants follow stable greens, and independent review precedes integration merge. Keep the helper's source guards unchanged.

Evidence: C156helper run_mc_rows.py modified-path check; Task13 new VIS1-full files; both actual baseline reds atca0c36b3. Provisional715985a follows fullclient nativeexit0 and pureguards; headedrows remain pending. Alternative not taken: weaken source guards, call untracked code clean, or claim final green before actual browser runs. No player-visible/design/acceptance change.66/170runs.

### C163 related test fixture scope

Mechanical client-test adaptations caused by the new visual-intent field and required per-slot owner/material inputs are in Task13 scope. Keep existing assertions, add the required actual owner/material fixture data and exhaustive-pattern .. where unrelated to visual intent. These are newly required fixture adaptations, not claims that baseline fixtures were already failing. Includes tests/widgets_lifecycle.rs and tests/render_map.rs. Routine import/type/Bevy API compile repairs need no per-line root permission; compile failures earn no red credit.

## C173: derive dense selection targets from actual screenshot regions

Conflict: the first SEL driver required500 isolated pixel components, but the real rendered formation has merged/tiny bodies. It found14components despite515actualsceneunits. The mixed-owner orange pixels are RGB194,140,42, about59from authored230,159,0 after the existing output transform, outside its48tolerance. The mobile Shift selector picked a merged5Troopercolumn, giving7selected from2statics rather than the required3.

Choice: derive the mobile formation box from the largest contiguous PNG-own-colour cluster, calibrate the PNG recognizer tolerance to60 from actual saved pixels, and choose an isolated one-mobile silhouette for Shift operations. Keep expected500/2/3/2 counts, actual C84 own/foreign checks and every required plant. No scene/debug input targeting or map/content change. Root offline checked the new PNG box[576,206,901,391] contains exactly500actualowner0centres. This is an instrument repair; the complete row remains pending.

Evidence: firstSEL2 captures715985a/cfcd163/76e2bf5; source sel-driver.js; root t13-root/sel2-pixel-box-offline-check.json. The first76e2bf5 mouse-upseq13 actually appliesonce inframe163 with selection500/rings500; sourcehash-bound rootreceipt sel2-first-frame-partial-positive.json. This proves only that first case; complete row failed later instrumentation and gets no green credit. Alternative not taken: change500expectation to componentcount, accept a multiunit Shift target as one, or weaken owner/timing checks.66/170runs.

## C174: expose the actual local slot in C84 debug snapshots

Conflict: SEL/VIS1-full ownership oracles require actual local slot, but debug_json lacks slot while the direct final report already exposes WelcomeInfo.slot. A row owner_slot expectation alone cannot prove the actual joined client slot.

Choice: add actual WelcomeInfo.slot to debug_json in existing Task13 debug.rs ownership, with native capture/rebuild consistency. Preserve strict own/foreign oracle checks; join per-unit owners from actual replicated_world.units byunit_id because scene.units has no owner_slot. Keep driver inputs PNG-only and do not add a guessed slot or new wire field.

Evidence: debug.rs cached/direct report builders; actual76e2bf5 C84 top-levelkeys and sceneunitkeys; sel-oracle.js owner checks. Alternative not taken: trust a row constant as actual client state, classify missing scene owner fields as foreign, or relax the ownership assertion. Focused native debug consistency passed; full headed row remains pending.66/170runs.

## C175: show exact-footprint rings over larger rendered models

Conflict: actual VIS1-full atbdaacd8 selects foreignunit3 and records the real ringradius0.5, but all four gap probes see the same orange body. Fixture unitdummy has footprint0.5 and acquired Capsule visualradius0.9/height2.0, so a normal depth-tested ground ring is occluded. Enlarging by1.2wouldstillbehidden and would change the required footprint relation.

Choice: retain exactfootprintradius, ground projection, slotcolour and solid-own/four-gap-foreign geometry. Use the existing Bevy0.20 ExtendedMaterial mechanism for a shared ring overlay rendered after opaque models, depthcompareAlways/depthwritefalse, alpha1Blend, shadows/prepassoff. Keep one globally shared overlay material per slot beside its body material. Use the existing solid-ring mesh and one shared neutral contrast underlay for the foreign pattern: otherwise transparent gaps expose the same-slot-colour body and remain visually indistinct. Order underlay before coloured foreground; only selected foreground has RingMetadata, so counts stay one perselectedunit. Foreignhover preserves the gap topology. No per-unit meshes/materials, new inputs, radius changes or fixture edits.

This resolves the literal one-material-per-slot draw-method conflict with a bounded global render representation, under brief4 rights for the nearest existing mechanism that keeps the exact player-visible shape. The design is not edited. Native pipeline/composition/asset-sharing controls and actual four-gap pixels remain mandatory, followed by the independent foreign-solid plant.

Evidence: actualPNG/C84 run62-vis1-full-bdaacd8-20261101; fixture crates/turts_pack/tests/fixtures/muster_hud4/{units,visuals}/dummy.ron:1; pinned Bevy0.20 bevy_pbr/src/extended_material.rs:31,123 and pbr_material.rs:682-697. Root viewed screenshot and read footprint/visual data. Alternative not taken: changefootprintradius/palette, shrinkthefixturetohidebug, or count a frustum-visible ring entity as visible gap pixels. Runtime fix is provisional until fullVIS1green/ownedplant.66/170runs.


## C176 and C175 contrast clarification

C176 keeps the exact outer footprint and uses inner0.88/midpoint0.94 for a readable stroke and guarded same-pose reference samples. Four coloured quadrants and independent foreign-solid plant remain required. C175 uses at most two shared neutral backers selected from actual slot colour; black/white native controls pass. Full row db3a3ca failed before the visual oracle because the initial saved PNG was absent from the checkpoint index; this earns no semantic red credit. Index fixed in clean7702c8b; bound full VIS1 running. Details: [[Decisions/2026-11-01 MC Codex rulings]].66/170 runs;15/25 complete.


Task13 instrument update:7702c8b pair failed on missing initial base64 artifact before visual checks; no red credit. Clean0c26fab shares screenshot persistence, preserves input/checkpoint association and measures damaged health after moving pointer away from the overlapping tooltip. C176 uses four interior quadrant sample groups at radius1 neighbourhood, with solid/missing-quadrant/gaps-only controls. Fresh headed full row active; final gates/plants/VIS2/review still pending. Root manually read actual7702 tooltip text "Training Target · Slot 2"; provisional source-bound receipt t13-root/tooltip-manual-7702c8b.json.66/170runs;15/25complete.


C177: actual0c26fab saved captures expose attack cross occlusion despite first-frame marker metadata. Root viewed PNG. Keep exact target/cross/colour/timing; use dedicated marker gizmo depth_bias=-1, preserving unrelated gizmo configuration. Actual repaired full-row proof remains pending. The preceding helper-wrapper TypeError is instrument failure, not semantic red. Details in today's Codex rulings.66/170runs;15/25complete.


C178: real Moving/Pending plain moves project current:null despite captured authored identity; queued record alone cannot draw the required two-leg path. Task13 may repair read-only sim/economy/views.rs to use existing PlainMoveOrderIdentities as fallback, with real-command native red/green and legacy/stale/precedence controls. No wire/schema/frozen/audit change. Wait for immutable4dfe full row to end before modifying source; final gates and real pixels remain required.66/170runs;15/25complete.


## C179 actual hardware measurement

RUN68 found VIS2 readiness hard-requires NVIDIA. Root decides this instrument conflict: design9.1 requires actual Chrome WebGPU5000 measurement, no vendor restriction. Require complete actual nonsoftwarehardware metadata/configureddevice/context/submitteddraws and positivefinite metrics; record the actual machine. No inventedbudget or fakeGPU. See [[Decisions/2026-11-09 MC Codex rulings]].68/170runs.


C180: imported external documentation-only mainbb611bd INTO integration, preserving both work-log histories and the disk audit. No runtime/design/frozen changes and no merge/push INTO main. Task13 active lane untouched. Required integration re-gate remains after its later code merge.68/170runs.


C181: actual1b649dc two-headed row fails the waypoint oracle because it chooses retained moveintent1 instead of newqueuedintent2; C178 records now contain both correct targets. Gather physical stage still executed with actualworker2/node1/intent3. Root unchanged Gather pixel helper on real images fails observed0changedpixels (focusedexit1, NOT full-rowGatherfailure), and manualPNGshowsnodot. Authorize one shared markeroverlay with existingexactgeometry/target/colour, immediate+unobscured proof and corrected click-specific marker binding; freshfullrow mandatory. See today's Codex rulings and t13-root/resume-20261109/gather-actual-pixel-red.json.68/170runs.

C182 carries exact unchanged-input9cf VIS1/VIS2 evidence through docs/SEL2-only edits, with originalsource attribution. Final post-plant source/record validation and actual restored independentfullVIS1 remain required; any runtime change expires carry. No normal gate/requiredplant is dropped. See today's rulings.69/170runs.


C183: actual CAM1 failure was a14pixelHUDglyph competing with183pixelScout. Initial-only body floor50pixels/8x8 preserveswarm/blue/footgeometry/ambiguity rejection; nativecamera/content unchanged. CAM1twoheaded0 and affectedCLICK9two0 verified byRUN69. Normal native gate then stopped at strictClippy101 in Task13-owned visuals/debug, no semantic red and no later gatecredit. Narrowrepairs underway; runtime-source change expires exactVIS2carry, so freshpostgate/postplantmeasurement is required perC182.69/170runs.


C184: required external ExtendedMaterial.base collides with derived base package identifier. Existing provenance-backed generic exception may allow identifiers_only:true; all bare string/bytes/fullIDs/paths remain scanned, replaceable locals renamed. Explicit bare-literal controls + true→false boundary plant/restoration and newlyaffected registered E1-derived-scan required. O10ShellAction decision is unchanged; thisnewexternalAPIcollision is separatelyreported. Native203 gate failedscanner, ae6030a native/QA/frozen gates all0 provisional; finaltest/localrename patch and finalgate rerun remain.69/170runs.


C185: one-frame plant actualfirstcases prove seq13 frames163/148 release0rings/0selection and EXACT nextframe500/500, preselection0. Existing named first-release ring assertion0!==500 is equivalent to plan's prose message. Credit only that firstnonShiftcase; take-based plant's laterShift side effect explicitly unclaimed. Exactrestore/selectedgreen still pending. RootrawSHA-boundwitness saved. World-box plantalreadytwoactualreds→exactrestore→twoSEL2greens; no sourcefaultretired.69/170runs.


## CEO travel pause 2026-11-09

PAUSED by CEO for travel/laptop transfer on 2026-11-09. 15/25 complete; 69/170 runs. Task13 at40b9d90 is unfinished and not accepted. Normal gates passed; plants/review/finalVIS2 remain, including an unresolved restored-source Shift-add failure. See [[Reports/2026-11-09 MC laptop handoff]]. No main merge/push. RUN67 and RUN69 must preserve evidence and restore temporary mutations. No further implementation dispatch until CEO resume. Next runtime state is in their pause receipts.

## CEO desktop resume2026-11-09

ACTIVE on the desktop after explicit CEO resume2026-11-09. 15/25complete;70/170runs registered. Task13 at40b9d90 remains unaccepted. RUN70 builder owns sole runtime to resolve restored-source SEL2 Shift-add failure, then remaining plants. No laptop work occurred. No main merge/push.


C186: dedicated headed X display for remaining Task13 browser checks, unchanged assertions and actual hardware WebGPU proof required. Old Shift-add failure retained; actual synthesized release in oldrun is inference, bounded diagnostic and isolated results NOT TRACED until they run. See [[Decisions/2026-11-09 MC Codex rulings]].


C186 measured update: controlled actual unsequenced Shift=false move reproduced original Shift-add1!=3 in2/2 headed runs at40b9d90. Before selection[43,44], release frames300/272 selection[41]/rings1, actual Shift=true down/up, unchangedcamera,1/6 actual frames after injectedfalse before nextpointer. Exact harness restored SHA021e888d… and clean. Root independent audit t13-root/desktop-resume-20261109/shift-add-causal-runtime-audit.json. This is diagnostic, not required plant credit; historical synthesizedKeyUp remains inferred. Clean isolatedSEL2 control is pending.


Task13 RUN70 required plants complete: no-screen actual red2/2 exit1 with candidates40/visible39/selected40 (offscreenownTrooper40 selected), exact CoreSHA c9ced2… restored then green2/2 exit0 selecting39/ID40excluded. Palette nearpair red101 with normal0.377<20/protan0.372<10; exact paletteSHA71728a… restored and4/4green0. Root raw audits saved under t13-root/desktop-resume-20261109/{no-screen-required-plant-audit,palette-required-plant-audit}.json. RUN70 released allholds/processes; root verifiedclean/no compiler and granted RUN71 exclusive samecache runtime. No duplicated palette execution needed; reviewer audits receipt.

Task13 completion: ACTIVE:16/25MC tasks complete;71/170buildruns. Task13 merged60082da, independent0High/0Medium; native10gates0 (1446/0/19) and headedT7exit0 (137frames/53moving). Final5000-unitVIS2record9ae verified; GCPpendingD59. Lane removed, branch/evidence retained, solecache on integration. Wave6 push/hostedCI next, then Wave7. No main merge/push. See t13-root/desktop-resume-20261109/acceptance-audit.json and RUN71final-review.md. C186hardwareclosure: actualconfiguredGPU1 NVIDIA/Ampere, source40b, Xvfb100,5000public/rendered,89textureacquisitions,178submissions,0instrumenterrors. The2configuredContexts are configurationevents. FreshVIS2 p50133.3ms/p99199.9ms/1529calls over10s/505872384bytes; exactrecordred0/1 then fourrows4/0, docscommit9ae. Historical9cf stays narrative. Native/WEBregates on60082da all0; frozenSHAexact. Metadatareceipt had canonicalmapdigest labeled filehash; corrected fileSHA22ff1a07…/separatemapdigestcc168db7…, all916actualentries verified, oldreceipt preserved. No source/rawregeneration. C187allows same-workspace frozenproofreuse with9negativecontrols, no gate/plant removed.

ACTIVE Wave7:16/25complete;73/170runs registered. Wave6 pushed588dcd3; CI42681727530 agents success, check/browser RUNNING (not green). RUN72 Task10 sole localruntime; RUN73 Task15 red-test/source prep only under C188. NoWave7merge before terminalWave6jobs read/classified. Main unchangedbb611bd. C188records overlapped preparation with hostedCI; terminalresults stillrequired. C187helperuses explicit reuse-frozen-workspace proof, allothergates remain.


C189resolvesTask15 AUD1 dependency: advanceonlydesignedvolume-menu/persistence subset from16, actualmeasuredAudioPlayercount, controlledMuster-copyalertfixture; nohidden setter/newplayercontrol, fullrealbrowserproof remains. Task16extends same menu andinheritscoveredvolume tests, no manufacturedred. C190retains actualHD1 unknownargument diagnostic: RUN72 compiledreal-server semanticred101 beforeimplementation. RUN73 pureNode resume-hook red1 is sourceunitproof only, no browser/Cargo claim. See Nov9rulings.


C191resolvesHD1id-only/publicorder and Content/Job gaps: existingsemanticUiAction forordinaryorders, narrowtypedUIorder/corecomposition sharedbyrealProduce/Cancel widgets andheadlesshook; preserveUIguards/playerbehavior, no directenvelopebypass. ControlledHD1Muster-copyfixture permitted foractualall-behavior coverage, exactaccepted/refusedsets andrealgroups. Actualpublicorderidspreserved; proseexampledoesnotrenamecontent. RUN74earlyreviewcaughtmissingkeysemantics/fullcoverage andqueue/groupmodifierdefects; RUN72repairs beforegreen.


C192: HD1rowactions_file resolvesonevalidatedfixturesJSON intoimmutableparams.actions; nativeandbrowser sharethefile. No driverfilesystem/debug access or duplicatedstages. Narrowexistingrunner/parameterhelper/testedit authorized; requirecontainment/type/conflictcontrols andfixturehashprovenance.


Wave6terminalCI42681727530 on588dcd3: agentsSUCCESS; check format/clippy/wasm/native/boundarypassed, Pythonbrowserregistry testfailsFileNotFoundremoved economy_ui.rs; browserT7passed89frames thenexactlegacyCommandCenter selection/rebuiltownerpanel5000ms timeoutJoinedslot0. BothapprovedO13Task12→21window, ownerTask21. Rootread/preservedall3rawlogs+terminalclassification inmain target/mc-evidence/wave6-ci; no generic unchangedM2timeout claim. C188terminalholdreleased. C187helperacceptedbyindependentRUN74: intrinsiccleanHEAD/status/940trackedinputbyteguard includingsource/vendor/.cargo/toolchain, uniqueevidenceoutput, 9root+15independentfrozen-suite negativecontrols and6head/source+3compilerinputdriftcontrols; oldhelperunchanged.


C193actualaudioobservation: verifiedBevyAudioOutputprivate; preWASMhookread-onlysnapshot ofALLrealcontextstates feedsowner gating/debug. Nonemptyall-suspended→all-running AUD1, no guessedresumesuccess, actualentity/categorycounts andrealOGG/gains stillrequired. PureNode fakecontexts are unitproofonly.


C194: HD1tickoptionalperdesignexamples; missinginheritsprior effective tick(0first), orderpreserved, explicitnonnegative/nondecreasing integer andboolrejection. Native/browsernormalizationmatch; positiveall-omitted/mixedcontrols required. Rootcaughtmandatorytickdraftbeforecandidate.


C195HD1freshprojection: atmostoneauthoredaction perCoreAdapterframe, tickearliestdue/orderpreserved, noadditional sleep/tickincrement. ReprojectafterpriorCorecamera effects; sameactioneventsatomic. Actualdoublegrouprecall→worldtarget regression required. C194defaults andserverDET1exactticksunchanged.


Cache interleave: RUN72explicitlyreleasedidlecompiler whilecontinuingHD1fixture/browser sourceauthoring. RootpausedRUN73writes, preserved84explicitownedWIPfilesinstashde16c846d4878c3a49b6cddb07f68eabd4b5818e, movedsolecache10→15/all5cleans0, reappliedstashandverifiedEVERYhash/statusidentical. RUN73nowhasboundedcompiledstub-redleaseonly foralerts/notices/volume_settings/audio_readiness; noproductgreen/body/audio-dependencydownload/browser/gates. Afterbatchreleaserootreturnssolecacheto10withsameWIPpreservation. Stashretained, noextraagentrun; global74. Receipt t15-root/borrowed-baseline-20261109/restoration-and-lease.json.


C196HD1actualparity: trustedlauncher copiesboundrealnative--out+receipt intoeachbrowserevidencedir, oracle.serverFile only, nodriverresults/capability. Separatefreshservers/samefixture; reuseONEprovennativeexecutionfor2browserattemptsifruntime/action/package identitiesmatch. Exactper-envelopeaccepted/refused inclmissing/extra/emptyopposites. Sharedfixtureordinarygroups/staticcamera; C195double-recallstress focusednativeonly.


Cache return and native progress: RUN73 completed actual compiled alerts/notices/volume/audio-readiness stub reds (101), unknown-sound validator green0. Root verified exact84path post-red archive/patch against25primarysourcehashes before green implementation. Cache returned15→10 after explicit82path RUN72 WIP stash5a786ee0…; all5workspacecleans0 and every82path hash/status restored. RUN72 sole runtime, RUN73 green SOURCEONLY, RUN74 SOURCEONLY. Receipts t15-root/borrowed-baseline-20261109 and t10-root/cache-return-after-t15-reds-20261109. Native HD1 actual1test green6.28s: Hold then Shift+Move produces authoritative not_queueable refusal for39livingTroopers, allrequiredbehaviors/groups asserted. Prior ownDepotAttack accepted and UI-disabled DepotStop hypotheses retained; neither is final parity proof. Bound native report, actual browser pair, committed gates/plants and independent final review remain. Global74/170;16/25complete. Disk658GiBfree, one local MC cache.


C197: nativeaudio ALSA development dependency is missing and sudo needs a password. Root verified official115kB aptpackage SHA against aptmetadata, extracted realheaders/linkerfile to localevidenceprefix, linked onlylocaldevelopmentpath to matching installedruntime and verified pkg-config/actualversion1.2.11. Explicit PKG_CONFIG_PATH+PKG_CONFIG_SYSROOT_DIR will be recorded for nativeaudiobuilds; no systemwrite/productchange/compilation claim. RUN72 keeps solecompilerlease. See Nov9rulings and native-prerequisites/alsa/receipt.json.74/170runs.


Task10 provisional runtime proof: valid installedwidget eligible-order mutation101→exactrestore0 now inspects actual PendingCommands envelope (qualified order/producer7/Replace/group0); earlier drainedCoreInputQueue assertions are test-instrument failures and are not redcredit. C195 old-batching101→restored0 confirms actualtarget32,22 forbothworkers. Bound release-native run01 passednamed1test6.12s/exit0; root independently verified1367current sourceentries,3executablehashes, rawlog/hash, before==after identity and reportSHA dbd382fcb545092c756258193daafafc933c12c83920f3f6493908cc6f1832dc. Actual204per-unitresults/39not_queueable refusals, livingtroopergroup39/workergroup2. Evidence t10-root/native-binding-run01-audit.json. Headedbrowserpair now compilingWASM withsamefrozen source/native reference; no browser/candidate acceptance yet. Global74/170;16/25complete.


C198 binds required realerroraudio via existing authored UnitPresentation.responses[error], categoryeffects, affectedunitpreferred/deterministicauthoredfallbackforunitlesserror; noassetnameinference/hardcodedmod/schemaexpansion. Narrow missingerrorentries authorizedinMusterinterface/controlfixture; Task17Forgewillauthoritsentries. Accepted-onlyresult mustneverplayerror. Realrefusal/failure timing/category/sound proof andvalidation remain. See Nov9rulings.74/170runs.


C199 allows RUN73 one bounded two-target cargo fetch to resolve plannedaudio Cargo.lock/dependencies during sourceprep. Preserveoldlock/actualargv-env-log-exit/lockdiff/no-targetproof; no update/generate-lockfile/compiler/test/server/browser. RUN72 sole compiler/cache unchanged. ThenRUN73 returnsSOURCEONLY.74/170runs.


C200 resolves ORD6physicalburst instrument: optional bounded count1..100 on existingio.click, samepixel/realtrustedCDPevents/per-release trace. Trustedharness may holdonlyapplicationRAFcallbacks duringboundedburst, thenrealflush/finallyrelease, no driverpauseAPI/game-state setter/newchannel. Requireactual100releases→100realcoreenvelopes/serverresults and separatelybound101stacceptedafter1s; actualrefusaltext/error timing remains. K25bucket/refill/tickrate unchanged. No realbrowsercredit fromhelperunit tests. See Nov9rulings.74/170runs.


C201 corrects root earlierTask10-existing-menu assumption: actualclient_shell.rs empty, staleFutureTask10comment; Task10headlessownsnomenu. C189volume/shellsubset thereforeimplementedNOWinTask15: singleexisting-keyF10/Esc overlay +fourvolumes viarealShellActionmenu effect, realUIcapture. Task16extendsSAMEoverlaylater. No key/acceptance/player-screen change, no hiddenvolumesetter. Rootownsincorrectpriorassumption; seeNov9rulings.74/170runs.


C202: actualHD1headedChromeBevymetadata BrowserWebGpu/Other/emptyname defeats an extraunplannednonempty-nameassertion, not HD1gameplay. Recordvalues/hardwareidentityNOTTRACED; noflag/process-arg hardwareclaim. Removeextrapredicate, keeprealheaded/parity/sourcebindings; Task13VIS2actualhardwaremeasurementunchanged. Firstphase-mismatchpairandsecondadapter-mismatchpairretainedasfailedinstruments, no parityacceptance.74/170runs.


C203: Task15ALR1provesactualderivation/sound/pingfeed+rule/unit/position, visiblealertline andrealwithin1shealthdrops/frame-emissioncooldown; nottotalretainedlistafter7s. OPEN acceptanceobligation: actualMINIMAPpingpixels ownedTask14(needs15AlertPings), thenrerunfullALR1+MM1. Task24mustrequirefullALR1visibleping; no permanent skip/fullrowgreenclaimnow. MinimapTask14stub cannotcountasdrawn.74/170runs.


Task10provisionalHD1green: rootfixture-only5centerfixes gave native0/headed2of2exit0;204results/39not_queueable each, exactreason/status/groups/selection/native, actualXvfb103 preflight0, rootrawaudit cell-center-repair/runtime-run01/root-parity-audit.json. Candidatefe5311 committed88files. FirstnormalgateClippy101 same-type id.clone().into(); narrowidentityrepair pending. C204 permits historicalbrowsercarryONLYafteractual3rebuiltruntimehashesALLidentical +unchangedactions/package/driver/oracle; originalexecutionidentityretained. Otherwiseaffectedbrowserrepeat. Allnormalgates/nativeGACC/plants/reviewremain.74/170runs;16/25complete.


C205: native G-ACC1d7c816 fails E9 full owner report at tick241 because expected projection omits existing public unit_orders and null max_health. This is NOT approved browser-panel red window. Repair expected from existing independently validated audit/snapshot helper, keep full equality/M2 no-health invariant, never copy/drop actual fields. Focused E9 red→green then final gates/GACC. Batch C194 explicit-null rejection (native Option currently accepts null, Python rejects), actual red/green controls. C204 actual native client/test hashes differ, so headed recheck required after batch. No frozen change.74/170runs;16/25complete.


C206: root stopped runtime after omitted Cargo environment created default cache. Preserved raw logs, dirty sources, inventories and workspace fingerprints; no active compilers/server/client. Removed only rebuildable target/release and target/debug; reclaimed2.69GiB; retained38GiB soleMC cache. New exec_in_lane.py (SHA65d28e4d…) or approved enforcing gate helpers REQUIRED for allfuture Cargo/Trunk/runtime commands.5env/locked/existingcache/shortHEAD/sourceimmutability/rawexit are enforced; actualpositive/3negativecontrols passed. ROOTowns runtime untilreturn. Correct-target E9 red and null-tick red preserved; wrong-target attempt is uncredited setup.74/170runs.


Root C205 focused verification is green under enforced wrapper4fffdc82…: native explicit-null rejection1/1 exit0, parameters/projection13/13 exit0, E9 audit-projection1/1 exit0 (53.24s test,70.87s including compilation). Each wrapper receipt records unchanged source and absent default caches. Existing correct-target semantic reds remain; wrong-target missing-client run is setup only. RUN72 may format/document/commit4bounded code paths, then freezes. ROOT will execute final committed gates/GACC/native+headed acceptance.16/25complete;74/170runs.


C207 repairs a K4 implementation gap: typed UiOrder must use existing alloc_group after valid-unit filtering when count>1, and share it across4096-unit chunks; single-producer group0 stays. One4097-unit Content boundary control required. Current e200 gate remains frozen until it ends; batch review fixes before next final run. K4/design unchanged, no CEO stop.74/170runs.


C208 applies design13.4 to HD1: every authored HUDbutton click first checks visible screenshotcolour with purePNG decoding/literalHUDpalette, savesactualcheckPNG/observation, failsbeforeclick onwrong/missingcolour. No driverpackage/debug/widgetread/newcapability. BatchwithC207beforefinalgates/acceptance; rawresultoracle unchanged.74/170runs.


C209 explicitly defers3Low instrument notes: early malformed Content/Job-worldpoint diagnostic; extra binding.source/source_after equality; release-label/profile cross-check. Actual trusted validfixture execution/source/runtime/status proof remains mandatory. No remainingHigh/Medium beyond current C207/C208 repairs; no extra invalid-input/security matrix. Anyactualcontractfailure overridesdeferral.74/170runs.


C210 places actual replica tick_hz at TOP-LEVEL debug/native report metadata in all capture paths; ALR1 reads that actualrate. No replicated_world schema expansion or hardcoded20Hz; no second C205-style legacy projection mismatch. Capture/rebuild source controls added; actualruntimeproof pendingTask15lease.74/170runs.



C211: RUN73 source preparation ended uncompiled/unaccepted with98owned paths archived/hashed, no target; resume later under a NEW ledger row. RUN75 uses freed slot for Task17 baseline tests/driver/asset recipes, all Needs merged. ROOT still sole runtime; no other wave or budget expansion. Task15 handoff t15-builder/run73/handoff-20261109/receipt.json.75/170registered;16/25complete.


C212 authorizes only test dev-dependency turts_interaction for actual effective key labels; minimal Cargo.lock resolution remains pending before Task17 baseline. Bare A/a prose allowed; explicit single-key instruction Press R must fail, preserving the required plant. No duplicate resolver or relaxed guide requirement. RUN75 branch/worktree created from integration0bee132; SOURCEONLY.75/170runs.


C213 fixes root producer-colour calibration: make_dummy actually builds Beacon. Both real pre-click17 PNGs show413pixels atRGB128,212,255; wrongDummyRGB0. Only purepalette/testlabel maychange; no input/package/Rust/oracleexpectation change. Carry e349native/GACC onlyifexactnativeinputs+freshbinariesidentical; currentGQA/purecontrols/freshnativebinding/actualheadedpair andplants/review/fullintegrationregates required. Preservefailedpixel attempts.75/170runs.


C213 measured: clean Task10 commit311d486 changes only hd1-ui-pixels.js and its pure test. All1369 original source files compared: only those2JS hashes differ. Fresh actual named native process exit0; client/server/test executable hashes, package tree and action bytes match e349 exactly. Carry e349 G-RUST/G-ACC accepted, retaining original execution identity. Current5G-QA commands and3pure controls each exit0. Fresh native report is independently bound to311d486; live timing means its full bytes differ, so no report-byte equality is claimed. Actual headed pair is active; no browser acceptance yet. Proof t10-root/final-311d486/c213-source-identity.json/current-gqa-results.json/native-reference-driver.75/170runs; ROOT sole runtime.


C214: register RUN76 before Task17 continuation. Prepare the missing SEL1 baseline source now. After Task10 acceptance, merge and full integration re-gates, give the sole cache to Task17 for its actual pre-HUD native/browser baseline batch, then to Task15 while Task17 writes content source. Keep one compiler/cache/runtime owner and preserve/hash every WIP path before a cache move. No green content before actual baseline reds, no second cache, and no gate omission. Task15 continuation needs its own later ledger row.76/170runs; Task17run2/4.


Task10 final311d486 local acceptance evidence: headed HD1 pair2/2 exit0, every11button check passed per run,204unit results/status/reasons exactly match the fresh native process,39held-Move refusals, groups39+2 and finalselection39. Hardware adapter identity NOT TRACED (actual BrowserWebGpu/Other/empty name). Required NoCore plant compiled and joined actual Muster but returned0results; named HD1 failed Remove40vs39, exit101. Exact source hash restored, status empty, same named process exit0. RUN72 ended clean. RUN74 now owns exclusive runtime for C196 controls and its own GroupDrop plant/finalreview. No Task10 merge yet. See t10-root/final-311d486/{root-final-parity-audit.json,no-core-plant/receipt.json}.76/170runs.


C215 resolves SEL1 instrument target: actualForge foreigners are weaponless drones. Add one existing foreign forge:turret only in a controlled sel1/forge fixture copy, owner1 at visible empty(26,8), emptycontents. Keep original Forge maps/units fixed; keep clone content byte-identical except documented placement and mirror later HUD/assets. Require the range assertion unconditionally; HUD2 remains actualForge. Preserve clone/delta proof and actual preHUD reds.76/170; RUN76 source-only.


C216: root caught native-reference.sh forcing target/mc-t10 and hd1_reference.py enforcing the same literal. In another lane this would make a duplicate build cache. Hold Task10 merge; repair only assigned-target routing/validation in these2instruments. Use an actual stopped Cargo-stub control (not gameplay red), then fresh native binding/browser proof/currentQA and byte-identity carry of unaffected native gates/plants. RUN77 re-check is registered before dispatch; Task10run3/2 belowsoft4/hard8. Prove actual non10 routing after integration merge with its sole cache.77/170runs.


C217: HUD2 uses two companion instruments: HUD2.json actualForge and HUD2-MUSTER.json actualMuster, same pixel-only driver/oracle with explicit scenario. Both required for acceptance HUD2/finalbatch. The runner supports onepackage perrow, so no omitted resource-line obligation or fake mixedpackage windows.77/170runs.


C216 compatibility includes exact explicitly assigned legacyCI target/m3-t16, traced in ci.yml. Stopped-stub red02 proves oldtargetoverride; patched MCtarget and legacyCIcontrols both0, no compiler/newcache. FirststubSyntaxError preserved as uncreditedsetup. Clean candidate83e6228 changes onlytwoinstrumentfiles; fresh affectedproof pending. RUN77 source-only; ROOT sole runtime.77/170.


C218: SEL1 world-tooltip text uses existing turts_ui actual-entity observation. Add kindworld_tooltip rows from WorldTooltip+Text/computedlayout, not AuthoredWidgetId spoofing/catalog-derived expectations. NarrowTask17 visuals marker visibility +snapshotinstrument ownership, actual semantic baseline beforegreen, paired0.5s pixels+text browserproof required. No OCR/driverchannel/tooltipbehavior change.77/170.

Task10 COMPLETE at merge ea943bf: all11 postmerge stages0, native1454/0/21 in115groups, exact frozen26+1+1 reused from that workspace execution, headed T7/pass137frames. Actual shared HD1 native process in target/mc-integration passed0; all three executables are in that assigned cache and no target/mc-t10 appeared. Root verified product tree equal reviewed b051e04, retired the clean inactive Task10 worktree, preserved evidence and branch. Root acceptance audit: t10-merge-b051e04/acceptance-audit.json.17/25complete.

Task17 C212 offline lock resolution passed0 with only the existing turts_interaction test-dependency edge; no package refresh or target created. First helper ps-no-process exit1 was an uncredited preflight setup issue, fixed before Cargo. Task17 rebase preserved all54 WIP hashes/status. Task15 rebase preserved97 files exactly and the98th core_adapter is the verified conflict-free three-way merge of saved WIP and Task10. Both archives/stashes retained outside lanes. RUN78 registered before source-only dispatch.78/170runs.


Task17 runtime handoff measured: C212 offline fetch0, only expected existing pack→interaction test edge, no target/package refresh; sourcehelper ps empty-result exception was setup-only. Root archived/stashed55 paths, moved the sole cache integration→17 with all5workspace cleans0, restored EVERY WIP hash/status and kept stash3208cb0. RUN76 now has exclusive bounded baseline lease under wrapper; no HUD/assets/snapshot green before actual named reds. RUN78 /root/mc_t15_build2 was dispatched only after ledger commitff46bbb, SOURCEONLY.78/170. Receipts t17-root/{c212-lock-resolution,cache-lease-baseline}/receipt.json.


RUN76 native baselines root-audited: guide3tests1pass2fail101 at missingForgeinterface; C218focusedlibtest1actualfailure101 at missingrealWorldTooltip row. InitialValue.iter compilefailure and exactshort0tests run are setup-only/uncredited. HUD2 actualheaded2/2 named no-selection-panel reds1 after C84 proves realownedselection[1]. SEL1 unsupported rowfacts attempt issetup-only; currentclear-pixel predicate needsC219body-colour calibration despite correctempty Coreselection/renderedrings. Keep runnerwhitelist, use existingrow/package/fixturefacts, correctpixelcomparison withactualinitialimage +samecamera/staticpositions +actualselectionringmetadata. Re-runaffectedbaseline beforeproductgreen.78/170. Rootnativeaudit t17-root/baseline-native-audit.json.


Task17 bounded baseline COMPLETE: nativeguide2fail101/1lexicalcontrolpass, actualWorldTooltip snapshotlib1fail101; headedHUD2 no-panel2reds1 afterrealownedselection; SEL1v4 missingworldtooltipobservation2reds1 withactualdelayedglyphs; MusterexistingOre5002green0. Root audited rawreceipts/C84/PNG; no hoverproductdefect. C219addendum confirms selected-onlyRingsShown. Root archived55T17and98T15paths, movedsolecache17→15/all5cleans0, restoredEVERYhash/status; stash5943947retained. RUN78 nowexclusivefocusednative --audio wrapper; RUN76 SOURCEGREEN only, no target/compiler/browser. RUN79 registeredbeforeindependentsourceaudit.79/170;17/25complete.


C220 records RUN79 consolidated source batch: H blank rendered alertTitle; M slot-wide research readiness, missing alert-click camera route, None-interface/prejoin no-orders, earlygesture/latecontext audio deadlock, and request-only/Update-before-playback ORD7 evidence. ALLREPAIR, no deferral. Root grants narrow commonCore camera widget route, generic no-interface notice fallback +actualUI observation; coordinate snapshotfilewithTask17. Actualsameframe sink proof/playback-delay plant required. Current4targetnativegroup green6+6+7+3 ispartial only; ORD2 stillfixture-preflight overlap aftercorrectingSlotId type, notsemanticred. Float/dual-techfixture corrections scoped; no gates/commit beforebatchrepair.79/170;17/25complete.


C220 addendum: keep actual ORD7 result-bearing causal frames in existing4096bounded retained_input history and preserve actual sink onset perentity, with separately observed currentgains; recent64/rolling96same-sink samples are FPS-flaky. C221 resolves plant conflict: actualtypedatomic-return mutation mustREDthe300mixednativecase;40validpixelcase isGREENcontrol asdesignrequires, notfictitiousred. No namedGREFplantretired. C222 authorizes genericNeeds<required><name>(available/required) and inventoryTextformatting whilepreservingtypedarrays/ownership/eligibility; noForgehardcodes/schemaaddition.79/170.


C223 resolves knownemptyturretammo: actualownammo definition+presentmainInventoryState withnomatchingitem→0; missing/unknown/foreignremainNull. Task17sourceonlygenericresolver, actualHUD2proofrequired; no frozenForgecontentchange. RUN78 C220batch nowinsource, RUN79 independentreadbackbeforegates.79/170.


C224: HUD2tooltipkey proof reuses actualvisibleAuthoredWidgetId owner::tooltip Text/rect alreadypublished; no newowner-intent tooltip_textfield. Pairphysicalhover/glyphpixels withactualrenderedkey andrequiredkey-dropplant. C218WorldTooltip observation remainsseparateengineentity.79/170.


C220browserproof: actualPNGbytes/losslessbase64, noUTF8binarydecode; causalhistoryalsoalertframes, exactpingrule/unit/tick/pre-attackidentity andclickedactualrect/action/targetcenter; controlledpackagegeometrypreflightoutsideGit, no setupcredit, freshHD1bindingafterfixturechange; still2attempts/viewport. C225advancesONLYdesignedsessionStorageturts.v1.player_id/fullu64 stringstoreatJoinneededforAUD1reload. NativeCLI/slotownershipunchanged;16stillownsretry/countdown/secondwindows. Actualsameid/slotserverdecimalproofrequired; immediateSocketClose NOTTRACEDuntilmeasured.79/170.


C226: learnedlab/turretcoords are localPNGcolour/shape searchseeds ONLY, no blindunitclick. Requireactualcurrentpixels/sampleproof plusactualC84selection; no driversemantics. RUN76source81pathreadybutthisinstrumentcorrectionpendingreceiptrefresh; stilluncompiled.79/170.


RUN76 ended SOURCEPREP at102WIPpaths, stilluncompiled/unaccepted andtargetabsent. Source-green receipt t17-builder/run76-source-green.json, SEL1clone63files/62identical/oneapprovedmapdelta. Any builderresume needsnewledgerrow. RUN80 registeredbefore independentmod-builder source-to-finalreview, usefulwhileTask15exclusiveactualruntimepending. C225canonicalCargoofflinefetchresolvedONLYexistingclient→wasm-bindgen edge; earliermanualedgecheckwasreverted andnotfinalcredit, freshlockednative/WASMchecks0. HD1A/Boriginalmapandshiftedbothpack0; unneeded460-beaconshiftrevertedexactHEAD, authorizedHUDButtonmirrorretained/currentpack0. Task15latestPID/ALRproofsourcefrozenawaitquick79readbackbeforeheaded.80/170;17/25complete.


C227 repairs RUN80 oneHigh/fiveMedium source findings before Task17 runtime: required Bindings scrollingList/all51actions, factual droneguide, actualselectedhealth and reserved-awareinventoryoracle, exactSEL1HUDclone, correctslot/radius/colour+reference ringproof. Root boundedsourcepatch archived; reviewer80 recheckpending. RUN79 finalC225sourcePASS0H/M; RUN78 GO sixheadedpairsAUD1first, sole runtime.80/170;17/25complete.


C228: AUD1 firstheadedpair bothactual1 beforeplayback/reload, sourceunchanged. RootactualPNG proves authoredorange tolerance48 rejects shadedorange(bestdistance57.56;1086pixelswithin70). Calibrateonlypixelinstrument withsamePNGoldfail/newpass controls; preservebothfailures. No audio/reloadacceptanceclaimed; RUN78solelease continues. Task17 C227 allsixsourcefindingsindependentlyrecheckedPASS; freshcloneparity63/62oneapprovedmap, wrongredpixelcontrol1/slotcolourcontrol0, Controlsphysicalscrollproofsourceprepared.80/170;17/25.


C229: freshAUD1pair2fails1/sourceimmutable nowreachesreload andTRACESoldlive-sessionrefusal forSAMEexactPID. AdvanceonlydesignedK27one-secondHello/15scountdown/retry subsetfrom16; noSlots/serverreplacement change. Meaningfulnative lifecycleRED→GREEN thenactualimmutableAUD1pairrequired. FullTask16SLOT1threewindows/otherrowsremainopen. Actualpre-reloadsinks7/gains106arepartialaudioevidence, notrowgreen.80/170;17/25complete.


C229mechanism: rejectedHello connection kicks300ms/ignoresresends; useactualreconnect andcurrentattemptguards. CountdownneedsactualprejoinUIcamera/Text becauseallcurrentHUD/camerasJoined-only. RUN79actualvoice/effects7onsets106/101gains, alerts0; combatblocked+statictargetwrong. Batchlawfulpixel/controlledfixturefixeswithretrybeforeanotherfullbrowserpair; noenginepathfinding/palettechange. Still80/170;17/25.


C230: keepstrictAUD1firstloadallSuspended→realgesture/allRunning; actualbrowserreloadC40isRunning/noGesture/zeroPlayers inbothpairs. Removeextraunsupportedreload-must-suspendassert; observeactualrunning/suspendedcontexts+noNewGesture/zeroPlayersthenNEWrealcategorysinks/gainsaftergesture. Designrowunchanged; C229rejoinfailurestillfails.80/170.


C231: RUN80 currentTask17groundpickeractualofflineoutput1232,216 isvoidRGBA43,44,47 andC84outside32x16floor. Rootrepairs withcurrentPNGopaquegreen25sample17x17patch plusoriginalHUD/bodyexclusions; nocoordinates/semanticreadsindriver. Preimage/sourcehasharchived, reviewer80recheckpending. Historicalbaselinevoidtarget32,568preserved/labeled; missingpanel/tooltipobserverredsunaffected; nobrowsergreenclaimed. C229actualnativecompiledred101 rootauditedexactsource21769d73/raw5254d4f3; broadergreenpending.80/170;17/25.


C231closedsource: actualwholeoldhelper/SAMEPNG/C84floornegative named1; currentca65ac7ehelper/SAMEPNGinsidefloorpositive0, historicalvoidpatchesfalse. Rootverifiedall9currentreviewhashes/exactpreimage/rawcontrols andupdatednextbuilderhandoff. NO Task17native/browsergreenclaimed. C229firstnativeactualcompilednamedred101 andexactpreimage21769d73/raw5254d4f3 rootverified; broaderlifecycle/sourceGREENpending.80/170;17/25.


C229draftnativeGREEN actual9pass0/sourceimmutable/raw3f03702c independentlyrootaudited, includingoriginalnamedredcase plusactualHello-cadence/delayedconnect/disconnect/deadline/success/staleevents/shellcameraText. Finalactual-ingestguard andsourcebatch/nativeWASM/sourcehashreview/freshAUD1pending. No candidatetaskcommit/merge/acceptanceclaimed; alloriginalbrowserfailuresretained.80/170;17/25.


C232: actualK27immediateWelcomeorobservedretry/countdownbothvalid; draftoracleextra2ndcountdown/mandatoryrefusal/prepaintHelloassertionsremovedinboundedJSbatch, keepingactualsamePIDslot/15s/Hello cadence/realUIwhenwaiting/allcategorygains. RootownsJSwhile78finishesRust; sourcefreezeand79recheckbeforecommands.80/170.


C233: real15s window startsFIRSTactualstoredtransport; actualHello cadence separate/noreset; absoluteTime<Real>guardbeforePreUpdateConnected/ingest cannotuseoldUpdateclock. Currentnative165pass0+a569/WASM0+c1a27517arepartialbeforefinalschedulefix. RootC232JSf42855f3/2db4faddindependentsourcePASS0H/M, exactactualauthoredreasonandcausalstaticeffect corrected; native/offlineproofNOTbrowseracceptance.80/170;17/25.


C234: actualC233headedAUD1pair2fails1 atINITIALsolidblackPNGs despiteJoined/515units/7UIrows/suspendedcontext/nogesture; causeNOTTRACED, noaudio/rejoincredit. NewAUD1-onlyscreenshotpoll≤10s waitsactualownbodypixelswithoutgesture, preservesfirstnegative/exactpositive;79sourcePASS8ebec6e6, Rust/oracleunchanged. FreshpairGO; no redundantnativeWASMJS-onlyrerun. C233finalnative167+WASM0+1515sourcehashidentity rootverified. ExtraC233postjoin-deadline nativeplant reservedAFTERcommit/gates, requiredallotherplantsremain.80/170;17/25.


C235profileauditCORRECTION: C233andC234pairsALLactualheaded0/headlessTRUE, despiteartifactlabels. FOURfailuresareheadlesssetup/readinessonly; noaudio/rejoinheadedacceptance. ForceMC_BROWSER_HEADED1/MC_PIXELS_HEADED1 OUTERdedicatedxvfb-run/actualxdpyinfo/inheritedDISPLAY, verifyactualfirstprofileheadlessFALSE+headed1. Onephysical:1headedattemptinterrupted/incompletepreserved; ownedprocesscleanupthenoneproperpairGO. Sourceunchanged8ebec6e6/2db4fadd; native167/WASM/sourcePASS stillvalid.80/170;17/25.


C236: validheadedC235pair actualsamePIDslot0rejoin10.509s/12.638s PARTIAL; fullAUD1failedwrongsliderstatesfirst/falseundefinedvoicecountsecond. Realpeak1/nooverlap, no mixervoicebugproved. RootJSvisibleper-sliderpre/postthumbackbeforeNEXTmove+actualframeproof; actualmaprequired/missingvoicekey0/null2reject. SamePNGcontrols0/79sourcePASSc08c7e83+16780932; Rust167/WASM0unchanged. FreshproperheadedXvfbpairGO; no redundantnative/WASMJS-onlyreruns.80/170;17/25.
