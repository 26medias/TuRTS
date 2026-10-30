---
type: task
date: 2026-10-29
tags: [turts, board, mc]
ai-first: true
source: codex
---

# MC execution board

State: STOPPED at task 1 preflight. No browser input was sent. No builder or reviewer was dispatched. P1 is NOT TRACED. No other task may start before the CEO receives the P1 result and releases the stop.

Plan: `docs/superpowers/plans/2026-10-29-mc-player-control.md`.
Design of record: `docs/superpowers/specs/2026-10-29-mc-player-control-design.md`, revision 10 at `488b3d3`.
Brief: [[Reports/2026-10-29 MC Codex brief]].
Base: `a0a3076`; integration: `codex/mc-integration`, worktree `/home/julien/Projects/TuRTS-mc-integration`.
Task 1 lane: `codex/mc-t1`, worktree `/home/julien/Projects/TuRTS-mc-t1`, `CARGO_TARGET_DIR=target/mc-t1`.
Main stays at `e2af4ab`. The pre-existing untracked disk audit stays in the main worktree.

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

- Design revision 10 wins over the plan. Report every conflict; stop on a task step that contradicts it or the product spec.
- K1 to K27 stand as recommended. K11 remains conditional on P1. Full design; no scope cut without CEO.
- Each task branch starts from integration after all Needs merge; one worktree and target per lane. Build binaries one at a time. Remove the lane and its target only after merge.
- Red must compile and fail semantically. Show the command and `; echo exit=$?`. Do not pipe away output before the exit code.
- Order: red, green, commit, gate, plants. Restore plants, then prove changed paths are clean. Never restore uncommitted or new files with git checkout.
- Stage explicit paths. Vault-only commits use [skip ci]. End each commit with `Co-Authored-By: Codex <noreply@openai.com>`.
- Verify path:line. Mark absent evidence NOT TRACED. Use ASD-STE100 prose and four spaces per indent.
- Do not edit the design, product spec or Claude configuration. Leave the plan's Doc updates section to its owner.
- Stop for P1 red, Firefox absent after at most 10 minutes outside the sandbox, frozen value change, design/spec conflict, suspect Decided item, task 18 content id collision, plant retirement without a decision, budget limit, or twice a task estimate.
- Before task 23, send the CEO the serve-only URL and wait. Tester identity is decided: CEO, not blind.

## 3. Gates and CI

Task gate sets are the plan's Local gate commands. T1 requires G-QA. After each task merge, run G-RUST, G-QA and G-FROZEN; also G-WEB for client/server edits. After each wave, push integration only and read hosted check, agents and browser jobs. Never merge or push main.

Frozen M2 SHA256: `bcbf72c2206251f635a7c7f5b6c8f573d69a5fc05a5bd77716e98ebfb08e370b`. Keep forge fixed point and refutation plants. Known browser failure: M2 browser proof; trace in T20, repair in owner task/T21. Current CI: NOT TRACED; no wave has completed and no push was made.

## 4. Tasks

24 numbered tasks; task 2 has two branches, for 25 lanes. All except T1 are NOT STARTED. T1 is STOPPED at preflight, with no probe result.

| Task | Wave | Needs | Branch | State |
| --- | --- | --- | --- | --- |
| T1 | 1 | - | codex/mc-t1 | STOPPED: preflight conflict |
| T2a | 1 | - | codex/mc-t2a | NOT STARTED |
| T3 | 1 | - | codex/mc-t3 | NOT STARTED |
| T20 | 1 | - | codex/mc-t20 | NOT STARTED |
| T8 | 2 | 2a, 3 | codex/mc-t8 | NOT STARTED |
| T18 | 2 | 3 | codex/mc-t18 | NOT STARTED |
| T19 | 2 | 3 | codex/mc-t19 | NOT STARTED |
| T2b | 3 | 2a, 3, 18 | codex/mc-t2b | NOT STARTED |
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

CEO approved about 212 total runs. Build stop line: 170 (106 planned task runs plus 64 repairs). Gate estimate: 42 spent before this build. Build dispatches used: 0/170. Root preflight is recorded separately and is not a builder/reviewer dispatch. T1 estimate 3; stop at 6 task dispatches. Do not dispatch another run at a stop.

One run is one agent dispatch (builder or reviewer). Count every run on the board.

| Task |Runs | Builder seat (Luna) | Reviewer seat (Sol) |
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

One row per builder/reviewer dispatch or resumed agent work turn. No runs have occurred. Do not add fake red, green or reviewer results.

| Run | Task | Seat | Model | Effort | Base | Result |
| --- | --- | --- | --- | --- | --- | --- |

## 9. Preflight ledger and stop

| Event | Evidence | Result |
| --- | --- | --- |
| Root setup | git status; worktree list; git show and SHA256 of design at 488b3d3 and a0a3076 | Design bytes match: 82d49b19c7e483793569cf4cd80d0b47108590e78913a8ecfa948fbed5929508. |
| P1 conflict | Design:363-364; plan:300,303 | Design requires default prevented in the output table. Plan says it is not used and omits it. STOP before implementation. |
| Precedence error | Brief section 1 versus user instruction and plan | User explicitly resolved: design revision 10 wins. Do not follow the brief's plan-wins sentence. |
| CEO decisions | Current user instruction | 212 approved; stop at 170 build runs; CEO plays both tests, not blind; exact base a0a3076. |

Proposed resolution for the CEO: retain `defaultPrevented` in the raw records and the table, in addition to the plan's `cancelable` column. Use reaches-page and browser-acted for K11. This proposal is not applied. The brief section 4 says to stop and report when a plan step contradicts the design. P1 remains NOT TRACED, not red or green. No merge, re-gate or wave CI is claimed.
