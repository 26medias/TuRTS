---
type: task
date: 2026-10-27
tags:
  - turts
  - board
  - m3
ai-first: true
source: mcp
---

## For future Claude
The run sheet for M3 implementation by Codex agents. It says who builds, who reviews, who validates, what depends on what, and what runs in parallel. The task text is in the plan; this note does not repeat it. Update the ledger table at the bottom after every task.

# M3 execution board

**Spec:** `docs/superpowers/specs/2026-10-27-m3-industrial-design.md`. **Plan:** `docs/superpowers/plans/2026-10-27-m3-industrial.md`. **Calls K1 to K8:** confirmed by the user on 2026-10-27 ([[Decisions/2026-10-27 M3 scope and orchestrator calls]]). **Base:** `main` at `fc6931a` (includes the execution board; product source unchanged from `9e1987c`).

## 1. Roles

| Role | Who | Model (rule from [[Team]]) | Does |
| --- | --- | --- | --- |
| Orchestrator (root) | main Codex session | Sol | Dispatches, merges into `codex/m3-integration`, runs gates, keeps this ledger and the vault. Only the orchestrator edits the shared hot files (section 5). |
| Builder | a domain seat from `.claude/agents/` | Luna | Implements one task. Reads its role file, `CLAUDE.md`, the plan task, the spec sections the task cites. Writes the failing test first. |
| Reviewer | a different seat, named per task | Sol | Reads the diff, runs the tests and one plant, cites `path:line`. Level R0, R1 or R2 (section 3). |
| Validator | `qa-lead` seat | Luna runs, Sol judges at checkpoints | Re-runs on the merged branch. Proves each test can go red, and on which build. Not the builder, never the same agent as the reviewer. |
| Judges at the end | `player`, `mod-builder`, `technical-director` | Sol | Milestone close only (section 7). |

Codex does not load `.claude/agents/` roles by itself. Every dispatch tells the agent to read its role file first.

## 2. Standing rules for every dispatch

- State model and reasoning effort in the brief. Suggested: Luna `medium` for tasks marked M, `high` for tasks marked H; Sol reviewers `high`.
- Minimum brief: task id, branch `codex/m3-t<N>`, base commit, worktree path, the plan task text, the spec sections it cites, owned paths, the gate commands, the report format (below), and these rules.
- **Verify, never assume.** Cite `path:line` for each claim. Label anything not traced `NOT TRACED`. List what was run. Agent summaries are leads, not proof.
- Write in Simplified Technical English (the standing rule).
- Test first. Show the test red, then green. A test that cannot go red is a defect.
- Stage explicit paths. Never `git add -A` while another agent runs.
- Build one binary at a time. Give each lane its own target directory: `CARGO_TARGET_DIR=target/m3-<slot>`. Delete a lane's target directory after its merge. (M2 left 155 GB of build output; disk was 85 percent full.)
- Do not change a frozen interface (plan "Interfaces" blocks). Stop and report to the orchestrator first.
- Do not edit a hot shared file that another running task owns (section 5).
- Report format (at most 400 words, long evidence in a file under the task's worktree): changed paths, tests added, red-to-green evidence, gate output summary, commit hash, deviations from the plan, open questions.
- Reviewers do not repair code. They report. The builder, or the orchestrator for a small fix, repairs. Only the reviewer that raised a blocking finding re-checks the repair, with a short message.

## 3. Review and validation levels

| Level | Meaning | When |
| --- | --- | --- |
| R0 | The orchestrator runs the task's gate commands and reads the diff. No reviewer agent. | Mechanical tasks with exact literal tests |
| R1 | One Sol reviewer with the named seat lens. | Risky or interface-heavy tasks |
| R2 | Two lenses. | Not planned. Ask the user first (token budget). |
| V0 | The orchestrator's gates after each merge. | Every task |
| V1 | `qa-lead` re-runs everything on `codex/m3-integration`, runs all plants so far (red, then restored green), checks that tests can go red. | Checkpoints C1 to C4 (section 6) |

Gate commands after every merge (all must pass): `cargo fmt --all --check`; `cargo clippy --workspace --all-targets --locked -- -D warnings`; `cargo check --workspace --all-targets`; `cargo test --workspace`; `bash scripts/boundary.sh`; and on a schema or wire task `cargo check --workspace --exclude turts_server --target wasm32-unknown-unknown`. Also keep `cargo test -p turts_pack --test m3_m2_stream` green from Task 0 on.

## 4. Tasks

Effort: M = medium, H = high (Luna). Paths are the owned paths; see the plan for the full list.

| Task | Builder seat (Luna) | Effort | Reviewer (Sol) | Validator | Depends on | Owned paths (short) |
| --- | --- | --- | --- | --- | --- | --- |
| T0 M2 operation-stream baseline | qa-lead | M | R1 `simulation-core-engineer` (is the stream real and the plant red?) | V0 | none | `crates/turts_pack/tests/m3_m2_stream.rs`, `tests/m3/m2_operation_stream.json` |
| T1 Schema API 0.2.1 | mod-runtime-engineer | H | R1 `technical-director` (versioning, boundaries, 66 literal sites) | V0 | T0 | `turts_mod_api`, `turts_mod_runtime` (archive, manifest, visuals), `scripts/boundary.sh`, struct-literal sites |
| T2 Portable validation | mod-runtime-engineer | M | R0 | V0 | T1 | `turts_mod_runtime/src/content_m3.rs`, its tests |
| T3a State types, ledger shape, reject reasons | simulation-core-engineer | M | R0 | V0 | T1 | `turts_sim/src/economy/{conveyor,inserter,extractor,combat,research}.rs` (types), `ledger.rs`, reason enums, `outbound.rs` |
| T5 Package `forge` + E1 scanner | game-designer (content) + qa-lead (scanner) | M | R0 | V1 at C1 (E1 plants) | T2 | `mods/forge/**`, `tests/m3/packages/forge_locked`, `turts_checks` scanner |
| T3 Transaction and removal | simulation-core-engineer | H | R1 `qa-lead` (atomicity, can failed writes leave state changed?) | V0 | T3a | `economy/transaction.rs` (hot), `plugin.rs` (hot) |
| T4 Boot | simulation-core-engineer | H | R1 `technical-director` (boot path, boundary, ids) | V1 at C1 | T3, T5 | `bootstrap.rs`, `registry.rs`, `map.rs`, `plugin.rs` (hot), `lib.rs` |
| T6 Extract phase | simulation-core-engineer | M | R0 | V0 | T4 | `economy/extractor.rs` |
| T7 Transport phase | simulation-core-engineer | M | R0 | V0 | T4 | `economy/conveyor.rs` |
| T9 Machines and recipes | simulation-core-engineer | H | R1 `game-designer` (recipe semantics, blocked output) | V0 | T4 | `economy/machine.rs`, `production.rs` (hot) |
| T8 Inserter phase | simulation-core-engineer | H | R1 `qa-lead` (held item, K7, tie-break tests can go red) | V0 | T6, T7 | `economy/inserter.rs` |
| T10 Research | simulation-core-engineer | M | R0 | V0 | T9 | `economy/research.rs`, `production.rs` (hot), `session.rs` (hot, admission only) |
| T11 Combat | simulation-core-engineer | H | R1 `qa-lead` (atomic shot, receiver-removal test) | V0 | T10, T3 | `economy/combat.rs` |
| T12 Commands Remove and SetEnabled | network-engineer | M | R0 | V1 at C2 | T7, T8, T11 | `command.rs` (hot), `messages.rs` (hot), `session.rs` (hot) |
| T13a Views and audit | simulation-core-engineer | H | R1 `network-engineer` (public Raw types, delta rule, killed-unit record) | V1 at C2 | T12 | `views.rs` (hot), `events.rs`, `plugin.rs` (hot), `economylog.rs`, `economy_export.rs` |
| T13b Protocol 3 and export | network-engineer | H | R1 `technical-director` (owner-only filter, versioning, privacy) | V1 at C3 | T13a | `turts_net_protocol`, `export.rs`, `outbound.rs` |
| T14 Oracle and scenario suite | qa-lead | H | R1 `simulation-core-engineer` (can the oracle be fooled by a matching event? is it independent of sim code?) | V1 at C3 | T13a (grows per case as phases land) | `turts_pack/tests/common/conservation.rs`, `forge_*.rs`, `tests/m3/packages/` |
| T15a Client replica and ingest | network-engineer | H | R0 | V0 | T13b | `turts_client` replica, ingest, debug |
| T15b Client UI, hooks, browser test | bevy-rendering-engineer (also reads the web-platform-engineer role for the wasm and `m3.js` parts) | H | R1 `web-platform-engineer` (wasm check, browser test, hook behavior) | V1 at C4 | T15a | `turts_client` render, ui, input, options, scenario, `turts_client_web`, `tests/browser/` |
| T16 Mutation sweep, process, browser acceptance | qa-lead | H | R1 `simulation-core-engineer` (do the plants model real defects?) | V1 at C4 (full F1 to F11 run) | T14, T15b | `tests/m3/`, `scripts/`, `crates/turts_server/tests/m3_process.rs`, `ci.yml`, `docs/measurements/m3.md` |

## 5. Hot shared files (orchestrator composes)

One task owns each file at a time. If two tasks need it in the same wave, the orchestrator merges one first.

| File | Touched by (in order) |
| --- | --- |
| `economy/transaction.rs` | T3a (match arms), T3, T4 (spawn-row hook), T9, T10, T11 (small hooks) |
| `economy/transaction_m3.rs` | T3 infrastructure; root composes later narrow phase accessors/hooks, one owner at a time |
| `plugin.rs` | T3, T4, T13a |
| `economy/production.rs` | T9, then T10 |
| `economy/views.rs` | T3a (match arms), T13a |
| `command.rs`, `messages.rs`, `turts_server/src/session.rs` | T3a (reasons), T10 (admission), T12 |
| `schema.rs` (`turts_mod_api`) | T1 only |
| `tests/browser/m2.js` | T15b only |
| `economylog.rs`, `economy_export.rs` | T13a only |

## 6. Waves, parallel lanes and checkpoints

Up to three builders run at once, plus one reviewer or validator. (M2 ran the root plus three lanes.) Each builder uses its own worktree and target directory.

| Wave | Runs in parallel | Needs | Notes |
| --- | --- | --- | --- |
| W0 | T0 | main | Must finish before any M3 source edit. Creates `codex/m3-integration`. |
| W1 | T1 | T0 | The large literal-fix sweep. Nothing else runs. |
| W2 | T2 ‖ T3a | T1 | Disjoint crates (`turts_mod_runtime` vs `turts_sim`, protocol, server reasons). |
| W3 | T5 ‖ T3 | T2 for T5; T3a for T3 | Package lane and simulation lane. |
| W4 | T4 | T3, T5 | Single task. Sets up the empty phases. |
| **C1** | V1 by qa-lead on the integration branch (E1 plants, M2 stream, all gates) | T4 merged | After W4, before W5. |
| W5 | T6 ‖ T7 ‖ T9 | T4 | Three builders. Disjoint files. T9 owns `production.rs` in this wave. |
| W6 | T8 ‖ T10 | T6+T7 for T8; T9 for T10 | |
| W7 | T11 | T10, T3 | T8 may still run. |
| W8 | T12 | T7, T8, T11 | |
| **C2** | V1 after T13a is merged | | |
| W9 | T13a | T12 | |
| W10 | T13b ‖ T14 | T13a | T14 starts the oracle and self-tests, then adds cases as phases are done. |
| **C3** | V1 after T13b and the T14 case suite are merged | | |
| W11 | T15a | T13b | T14 may still run. |
| W12 | T15b | T15a | |
| W13 | T16 | T14, T15b | Full acceptance. |
| **C4** | V1 full F1 to F11 run, then milestone close (section 7) | | |

```text
T0 → T1 → {T2, T3a}
T2 → T5 ; T3a → T3 ; {T3, T5} → T4
T4 → {T6, T7, T9} ; {T6, T7} → T8 ; T9 → T10 ; {T10, T3} → T11
{T7, T8, T11} → T12 → T13a → {T13b, T14}
T13b → T15a → T15b ; {T14, T15b} → T16
```

Critical path: T0 → T1 → T3a → T3 → T4 → T7 → T8 → T12 → T13a → T13b → T15a → T15b → T16. A delay on any of these delays the milestone; put the strongest builders there.

## 7. Milestone close (after C4)

1. `qa-lead` runs F1 to F11 and the full plant sweep. Record machine type and results in `docs/measurements/m3.md`. Claim no budget.
2. `technical-director`: one whole-branch review (boundaries, versioning, Open policies still Open).
3. `player` plays the forge demo in the browser and judges clarity in plain words. `mod-builder` tries to read `mods/forge` as a modder and lists any private gap.
4. Push `codex/m3-integration`, wait for CI to go green (M2 lesson: per-task tests missed a red fmt step for 7 pushes). Ask the user before the push and before the merge to `main`.
5. Daily report in `Reports/`, update [[Tasks]] and [[Home]].

## 8. Token budget (proposal, track here)

Estimate: 20 Luna builder runs plus at most one fix round each for 8 of them (about 28), 12 Sol R1 reviews, 4 `qa-lead` checkpoints, 3 milestone judges. About 47 agent runs. This is a proposal under the budget rule ([[Team]], memory "token budget"). If actuals pass 1.3 times the estimate, stop and ask the user. Mitigations: R0 on mechanical tasks; one reviewer per R1; re-checks by short message to the same reviewer; reports capped at 400 words; no probe done twice.

Running count after interruption recovery: builders 31, reviews 21, checkpoints 2 (7 execution runs), judges 0. Additional orchestration executor runs: 2. Total agent runs: 61. Two interrupted workers were replaced; their saved source and evidence are retained. Approval requested before any further run beyond the 61.1 threshold. C1 used three private lanes plus scoped repair proofs; no extra review lens.

## 9. Cut line (from the spec 16.2)

In order, only if the milestone proves too large: (1) `SetEnabled`; (2) drone B and `MoveTo`; (3) browser rendering of belt items; (4) `Remove` as a player command. Never cut: compact belt with backpressure, item recipe with blocked output, hand as inventory, atomic shot with removal, a research that changes a stat, conservation checks with plants, E1, the M2 regression. The orchestrator asks the user before any cut.

## 10. Ledger (update after every task)

| Task | Status | Branch | Worktree | Commit | Reviewer verdict | Validated | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- |
| T0 | complete | `codex/m3-t0` | `target/m3-worktrees/t0` | `e0e215f` → `fe6765f` | PASS, 0 findings | V0 PASS at `fe6765f`: 498/0/17 | Four 400-tick streams; extra-empty-op plant semantic RED, restored GREEN; independent comparison 1/1. |
| T1 | complete | `codex/m3-t1` | `target/m3-worktrees/t1` | `e534a03`, `87ef1f6` → `8cec0b9` | PASS after serialization fix | V0 PASS: 504/0/17; wasm | API 0.2.1 frozen; source provenance; old-reader round trip preserved. |
| T2 | complete | `codex/m3-t2` | `target/m3-worktrees/t2` | `c49b79b` → `76eef7e` | R0 PASS after repairs | V0 PASS: 539/0/17; wasm | 30 cases, exact diagnostics, syntax-presence version guard, corrected work-capability plant. |
| T3a | complete | `codex/m3-t3a` | `target/m3-worktrees/t3a` | `f3e844d` → `45e43af` | R0 PASS | V0 PASS: 509/0/17; wasm | Resource tables, output endpoint, reason tags; no systems or protocol bump. |
| T5 | complete | `codex/m3-t5` | `target/m3-worktrees/t5` | `dc3095a` → `0e29245` | R0 PASS after hand fix | V0 PASS: 548/0/17; C1 PASS | Forge/locked fixture, names, three E1 plants; no sim boot yet. |
| T3 | complete | `codex/m3-t3` | `target/m3-worktrees/t3` | `89be093` → `c9a8834` | R1 PASS, 0 open | V0 PASS: 563/0/17 | Strict M3/legacy split, catalog-correct extraction, all-store rollback plant. |
| T4 | complete | `codex/m3-t4` | `target/m3-worktrees/t4` | `a78fbf4`, `ea78099` → `73fc1cd` | R1 PASS incl. production-flow recheck | V0 PASS: 585/0/17; wasm | Public forge boot and shared rows; C1 accepted after capacity-test repair. |
| T6 | complete | `codex/m3-t6` | `target/m3-worktrees/t6` | `189fabe`, `e4b80fb` → `285a680` | R0 PASS after clipping-test supplement | V0 PASS: 605/0/17 | Exact rates, atomic extraction, full/depleted/disabled; combined with T7. |
| T7 | complete | `codex/m3-t7` | `target/m3-worktrees/t7` | `3b494f7` → `21dc877` | R0 PASS after boundary/split tests | V0 PASS: 597/0/17 | Compact movement/backpressure, exact boundaries, all split edges, empty clock. |
| T9 | complete | `codex/m3-t9` | `target/m3-worktrees/t9` | `3069b50` → `7a6249c` | R1 game-designer PASS, 0 findings | V0 PASS: 615/0/17 | Atomic item cycles, blocked output, half refund, K8, private derived status. |
| T8 | complete | `codex/m3-t8` | `target/m3-worktrees/t8` | `b6a8c7a`, `a1677ac` → `a5271b9` | R1 PASS after two timing guards | V0 PASS: 633/0/17 | K7, held stock, belt endpoints, literal timing; positive/bounded durations. |
| T10 | complete | `codex/m3-t10` | `target/m3-worktrees/t10` | `ae87c2c` → `4c7b8be` | R0 PASS | V0 PASS: 646/0/17 | Research payment/queue/completion, checked modifiers, server admission. |
| T11 | complete | `codex/m3-t11` | `target/m3-worktrees/t11` | `63f2f5a` → `b35365b` | R1 QA PASS, 0 findings | V0 PASS: 663/0/17 | Atomic shots/removal, references, research timing and receiver conservation. |
| T12 | complete | `codex/m3-t12` | `target/m3-worktrees/t12` | `01416ed` → `5848355` | R0 PASS | V0 PASS: 673/0/17; wasm | Ownership/no-op, atomic enable/reset, producer freeze, wire/admission. |
| T13a | complete | `codex/m3-t13a` | `target/m3-worktrees/t13a` | `0ea4ab1`, `db677d7` → `80fe968` | R1 PASS after removal delta fix | V0 PASS:685/0/17; wasm; C2 starts | Raw audit fidelity, facing retained, killed health0, removed disabled unit has no phantom enable. |
| T13b | complete | `codex/m3-t13b` | `target/m3-worktrees/t13b` | `df96a84`,`2779670` → `78d0ae7` | same R1 PASS after beltprivacy repair | V0 PASS700/0/17; wasm | Protocol3; authoritative per-piece ownership; private research, ports, facing, health0/removal. |
| T14 | complete | `codex/m3-t14` | `target/m3-worktrees/t14` | `dcddac0` → `0e28855` | Scoped R1 PASS, 0 findings | V0 PASS: 742/0/17; wasm | C1 retries, parsed resource balance, dangling state, full renamed industrial trace. |
| T15a | complete; queue follow-up accepted | `codex/m3-t15a` | `target/m3-worktrees/t15a` | `1eafa68` → `31c05d8` | R0 PASS | V0 PASS750/0/17 after queue follow-up; wasm | Atomic records, port keys, finalhealth0/removal, movingkill overlap, sharedmerge/debugtrace. |
| T15b | implementation and gates | `codex/m3-t15b` | `target/m3-worktrees/t15b` | next docs base | R1 web pending | pending | Presentation/actions/browser; coordinator interface frozen beforeconsumer. |
| T16 | not started | | | | | | |


## 11. Checkpoint ledger

| Checkpoint | Status | Source | Evidence |
| --- | --- | --- | --- |
| C1 | PASS | `f3b11e7` + test-only repair `32d1495` | `target/m3-evidence/c1/root-acceptance.md`; fresh gates `c1-final-v0/`; all88 tests refuted |
| C2 | PASS | `b87c111` | `target/m3-evidence/c2/root-acceptance.md`;685/0/17;99 new functions mapped;14 prior plants replayed |
| C3 | ready; run allowance approval pending | repaired docs freeze follows | Queue correction accepted; canonical 750/0/17 and all eight gates green |
| C4 | not started | | |

Execution refinement: independent T13b/T14 worktrees started alongside frozen C2; their merges remained gated on C2 acceptance. No checkpoint source was changed. This overlap avoids idle slots; a checkpoint repair would require dependent rebase.

W11 refinement: T15a integrated after T13b acceptance while T14 repairs its instrument, as the board permits. C3 will validate the larger union including client ingest after T14 merges. T15b builds in isolation; its merge stays gated on C3.
