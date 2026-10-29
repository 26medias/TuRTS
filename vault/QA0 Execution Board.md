---
type: task
date: 2026-10-29
tags: [turts, board, qa0]
ai-first: true
source: codex
---

## For future Claude

This is the QA0 run sheet. Read the brief and the plan before a dispatch. T1 is starting. The CEO approved all 23 mutation cases. The integration base is `074fe7eed153ae30e56830e639daf84757c16344`. This base contains `7a148ba`.

# QA0 execution board

Plan: `docs/superpowers/plans/2026-10-29-qa0-instrument.md` (passed gate 2).
Design: `docs/superpowers/specs/2026-10-29-qa0-instrument-design.md` (revision 3).
Contract: [[Reports/2026-10-29 QA0 Codex brief]].
Calls: [[Decisions/2026-10-29 QA0 calls K1-K6 and must tier]].
Scope: six tasks, four waves, must tier only. Integration: `codex/qa0-integration`.

## 1. Roles

| Role | Seat | Model and effort | Work |
| --- | --- | --- | --- |
| Orchestrator | root | Sol; high | Dispatch, verify evidence, merge tasks, run gates, keep the vault. |
| Builder | task seat below | Luna; medium for T1, high for T2 to T6 | Read the role file first. Write the failing test, implement, verify, commit. |
| Reviewer | qa-lead; separate agent from builder | Sol; high | Read the diff, run tests and one plant, cite path:line. |
| Validator | qa-lead; separate from source builders | Luna; high | Run T6 on the merged branch. Record each mutation and restore. |
| Consumer judge | mod-builder | Sol; high | Judge the six error texts in T6. |

Every dispatch names the model, effort, role file, branch, exact base, worktree, owned paths, task text, design sections, commands, report path and standing rules. Each agent reads `.claude/agents/<seat>.md` first, then CLAUDE.md, its task, and the cited design sections. Do not inherit the root history when a model override is set.

## 2. Standing rules

- Verify, never assume. Cite path:line. Mark an untraced claim `NOT TRACED`. List commands run.
- Write ASD-STE100 Simplified Technical English. Use four spaces per indent.
- Show red then green, with the exit code on the same line. A compile error is not red.
- Commit task files before in-tree trust or mutation runs. Never restore a new or uncommitted file with git checkout.
- Stage explicit paths. Never use git add -A.
- Use a separate worktree and `CARGO_TARGET_DIR=target/qa0-<slot>` for each lane. Build binaries one at a time. Delete the lane target after the merge.
- Do not edit CLAUDE.md, .claude/agents/, the design or the product spec. Leave the plan's Doc updates section to Claude.
- Stop for a frozen interface change, a plan/design/spec conflict, a suspect Decided item, a CORE list change, or agent run 26. Ask the CEO.
- Keep ENGINE_API_VERSION at the planned 0.2.2 after T4. Do not change NET_PROTOCOL_VERSION or the lockfile format. Keep deny_unknown_fields on all 46 types.
- Commit vault-only changes with [skip ci] and explicit paths. End commit messages with the Codex attribution.

## 3. Review and gates

| Level | Use | Evidence |
| --- | --- | --- |
| R0 | T1 | Root reads every cmp result and the probe receipt. No gate agent. |
| R1 | T2 to T5 | One Sol qa-lead reviewer; diff, tests, one plant, path:line and verdict. |
| V0 | Every merge | Root verifies the task gate and restoration evidence. |
| V1 | T6 | Full integration commands, mutation sweep, A1 to A21 disposition and consumer judgment. |

Before a Rust merge: cargo fmt --all --check; cargo clippy --workspace --all-targets --locked -- -D warnings; cargo test --workspace --locked; bash scripts/boundary.sh; cargo check --workspace --exclude turts_server --target wasm32-unknown-unknown --locked.
Before a Python merge: its exact CI step command, locally, exit 0.
After each wave: push only codex/qa0-integration and inspect hosted CI. Check and agents must pass. The known browser failure at M2 browser proof is recorded; cause NOT TRACED. Do not chase it unless the text changes. No merge or push of main.

## 4. Tasks and waves

| Task | Wave | Builder (Luna) | Reviewer (Sol) | Needs | Scope |
| --- | --- | --- | --- | --- | --- |
| T1 | 1, first | qa-lead, medium | R0 root | none | Copy probes and receipts; yardstick extraction and TSV. |
| T4 | 1 | mod-runtime-engineer, high | qa-lead, high | none | Capabilities, table mechanism, typo hint, 0.2.2, client lockfile. |
| T2 | 2 | backend-engineer, high | qa-lead, high | T1 | Completeness R1 to R9, fixtures, registry, CI. |
| T5 | 2 | mod-runtime-engineer, high | qa-lead, high | T4 | Library rule, resolver, call sites, two table rows. |
| T3 | 3 | web-platform-engineer, high | qa-lead, high | T1, T2 | Driver rule, roles, ledger, samples, CI. |
| T6 | 4 | qa-lead, high | mod-builder, high (consumer) | T1 to T5 | Full gates, mutation sweep, acceptance receipt. |

T1 must run first to remove the dependency on /tmp. Use the scratchpad if present, else ~/.cache/turts-qa0-probes/. T4 can then run in its own lane. At most three child agents run with root.

## 5. Hot files

| Paths | Owners in order |
| --- | --- |
| .github/workflows/ci.yml | T2, wave 2; T3, wave 3 |
| tests/qa/completeness/green.md and mut-a.md to mut-e.md | T1 copy; T2 syntax port |
| tests/qa/driver/*.js | T1 copy; T3 role lines |
| turts_mod_runtime manifest.rs, lib.rs, version_table.rs; turts_pack main.rs, cli.rs, bad_packages.rs; t2c_package_rejects.rs | T4 then T5 |

## 6. Budget and run register

Count each dispatch and each resumed work turn as one run. Record it before dispatch. No additional run is allowed after 26 without a CEO answer. An agent run is separate from a mutation run.

| Allocation | Planned runs | Used |
| --- | --- | --- |
| T1 | 2 | 1 |
| T2 | 4 | 0 |
| T3 | 4 | 0 |
| T4 | 4 | 1 |
| T5 | 4 | 0 |
| T6 | 4 | 0 |
| Repair reserve | 4 | 0 |
| Total | 22 to 26 | 2 |

These are planning allowances, not required dispatch counts. Do not spend runs without a task.

| Run | Task | Seat | Model | Effort | Base | Result |
| --- | --- | --- | --- | --- | --- | --- |
| 1 | T1 build | qa-lead | gpt-6-luna | medium | 8d621f3 | dispatched |
| 2 | T4 build | mod-runtime-engineer | gpt-6-luna | high | 8d621f3 | dispatched |

## 7. Task ledger

| Task | Status | Branch | Commit | Red evidence | Green evidence | Runs used | Reviewer verdict |
| --- | --- | --- | --- | --- | --- | --- | --- |
| T1 | building | codex/qa0-t1 | - | NOT RUN | NOT RUN | 0 | pending R0 |
| T2 | pending | codex/qa0-t2 | - | NOT RUN | NOT RUN | 0 | pending |
| T3 | pending | codex/qa0-t3 | - | NOT RUN | NOT RUN | 0 | pending |
| T4 | building | codex/qa0-t4 | - | NOT RUN | NOT RUN | 0 | pending |
| T5 | pending | codex/qa0-t5 | - | NOT RUN | NOT RUN | 0 | pending |
| T6 | pending | codex/qa0-t6 | - | NOT RUN | NOT RUN | 0 | pending consumer |

## 8. Preflight findings

| Finding | Source | Status |
| --- | --- | --- |
| Done condition says 22 mutation runs; T6 lists 23 cases, including a new row 23 needed for A5. | brief:59; plan:404-419,432 | CEO approved all 23; see the decision note. |
| Design says 20 completeness cases; passed plan adds case 21 and explains it as a gate 2 addition. | plan:465 | Disclosed plan amendment; no document edit by root. |
| A8 is deferred to MC; A19 is a guard with no O1 semantic red. | plan:435-437 | Report these dispositions. Do not claim they have a new red proof. |
| Plan says inspect main CI in T6; brief and user name integration CI. | plan:438; brief:58-60 | Follow the explicit user instruction: inspect integration CI. |
| RTK.md is absent at the repo root and common parent paths; search under Projects and ~/.agents found no RTK.md. | cat and rg commands in setup | CEO said to ignore and uninstall RTK. Binary and command hook removed. |
| One pre-existing untracked report is present. | git status --short --branch | Preserve vault/Reports/2026-10-28 target disk audit.md; do not stage it. |

T1 branch: codex/qa0-t1, worktree /home/julien/.codex/worktrees/53d3/TuRTS. No product test or mutation has run yet.

### Preflight scan

| Task or task pair | Producer and consumer | Result |
| --- | --- | --- |
| T1 | Probe copies, yardstick and receipts | Source fallback is explicit. Hash receipt must be final before SHA256SUMS is made. Commit task files before in-tree trust runs; use scratch copies for hash red. |
| T2 | 21 fixtures, R1 to R9, CI | Plan explicitly adds case 21 after gate 2. Keep exact rule sets. |
| T3 | 28 sample role lines, 12 live role lines, DR1 to DR7 | Option A only. No harness creation. |
| T4 | Stub signatures, tests, then product rules | Signature-first makes semantic red possible. Empty real table belongs to T4. |
| T5 | Library rules, resolver and real table rows | Stub first. Rule call lines remain separate for T6. |
| T6 | Full gates, mutation sweep, receipt | Run 23 cases by CEO approval. A8 stays deferred; A19 stays a guard. Known browser failure is recorded. |
| T1 / T2 | Copied fixtures become ported fixtures | Sequential waves; original bytes remain in probe evidence. |
| T1 / T3 | Copied JS samples get role headers | Sequential waves. |
| T2 / T3 | Both edit agents CI steps | Wave 2 then wave 3. |
| T4 / T5 | Shared runtime, packer and test files | Wave 1 then wave 2; T5 starts after T4 merge. |
| T1 to T5 / T6 | Committed product and instruments become mutation targets | T6 runs after all merges; restore only committed files. |

CEO call: [[Decisions/2026-10-29 QA0 mutation count and RTK removal]].
