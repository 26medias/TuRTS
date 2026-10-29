---
type: report
date: 2026-10-29
tags: [turts, qa0, codex, brief]
---

## For future Claude
The brief for the Codex orchestrator that builds QA0. Claude (Fable) wrote the design and the plan and ran both gates. Codex builds, reviews, validates and keeps the vault during the build. Claude takes over again for the "Doc updates for the orchestrator" after the merge. The CEO chose this path on 2026-10-29.

# QA0 build: brief for the Codex orchestrator

## 1. What to build

- Plan: `docs/superpowers/plans/2026-10-29-qa0-instrument.md`. Passed gate 2 on 2026-10-29. Build it as written: six tasks, four waves, the "must" tier only. Do not add the "can slip" items (they are deferred to MC in the plan header).
- Design: `docs/superpowers/specs/2026-10-29-qa0-instrument-design.md` (revision 3, passed gate 1). Read the sections each task cites. The plan wins on a conflict with the design; report the conflict.
- Product spec: `RTS-Engine-Project-Specifications.md` version 0.8. Decided items are not open.
- CEO decisions in force: K1 to K6 as recommended (K1 = O1), must tier only ([[Decisions/2026-10-29 QA0 calls K1-K6 and must tier]]). Only the CEO changes the CORE list (K6).
- Base: `main` at `7a148ba` or later. Create `codex/qa0-integration` from it before wave 1. One branch per task, `codex/qa0-t<N>`.

## 2. Task 1 sources

Task 1 copies the probes and receipts from the Claude session scratchpad into `docs/qa/probes/`. The plan names the source directories `QP` and `CP` under
`/tmp/claude-1000/-home-julien-Projects-TuRTS/0e0457bb-045c-4b6a-945f-1db6a0753ee2/scratchpad/`. A full copy (3,413 files) is also at `~/.cache/turts-qa0-probes/` (`qa0/` and `review/qa-probe/`). Use the scratchpad when it exists; use the cache when `/tmp` was cleared. Run Task 1 first in wave 1, so the plan no longer depends on `/tmp`.

## 3. Roles, models and budget

Follow `vault/M3 Execution Board.md` sections 1 to 3 as the model. Codex does not load `.claude/agents/` roles by itself: every dispatch tells the agent to read its role file first, then `CLAUDE.md`, the plan task, and the design sections the task cites.

- Orchestrator (root): the main Codex session, Sol. Dispatches, merges into `codex/qa0-integration`, runs the gate commands, keeps the board and the vault.
- Builder: the seat the plan names for the task (`qa-lead` T1 and T6, `backend-engineer` T2, `web-platform-engineer` T3, `mod-runtime-engineer` T4 and T5), Luna. Writes the failing test first.
- Reviewer: a different seat, Sol. Reads the diff, runs the tests and one plant, cites `path:line`. The plan names which tasks get a reviewer; T1 gets none (the orchestrator reads the `cmp` lines).
- Validator: `qa-lead`, on the merged branch, for T6 and the mutation runs.
- State model and reasoning effort on each dispatch.
- Budget: 22 to 26 agent runs. Count every run on the board. Stop and report to the CEO at 26 before you spend more.

## 4. Standing rules for every dispatch

- Verify, never assume. Cite `path:line` for each claim. Label what you did not trace `NOT TRACED`. List what you ran. Agent summaries are leads, not proof.
- Red, then green. Show the failing run and the passing run with the exit code on the same line (`; echo exit=$?`). A compile error is not a red (plan "Execution contract"). Never pipe a run through `tail`, `head` or `grep` before its exit code is read.
- Commit task files before any in-tree trust or mutation run. Never `git checkout --` a new or uncommitted file (plan "Execution contract", gate 2 run G1).
- Stage explicit paths. Never `git add -A`.
- One target directory per lane: `CARGO_TARGET_DIR=target/qa0-<slot>`. Build binaries one at a time. Delete a lane's target directory after its merge.
- Do not edit `CLAUDE.md`, `.claude/agents/`, the design, or the product spec (plan "Global Constraints"). The plan section "Doc updates for the orchestrator" is for Claude after the merge; do not do it.
- Write in ASD-STE100 Simplified Technical English. Four spaces per indent level in Rust, Python, JavaScript and TSV examples.
- Gate commands before a Rust task merges: the five commands in the plan "Execution contract" (`fmt`, `clippy --locked -D warnings`, `test --workspace --locked`, `boundary.sh`, the wasm32 check). Before a Python task merges: its CI step command, locally, exit 0.
- Stop and report to the CEO, do not decide: a frozen interface must change; a plan step contradicts the design or the spec; a Decided item looks wrong; the CORE list needs a change; the budget reaches 26 runs.

## 5. The vault

- Create `vault/QA0 Execution Board.md` from the M3 board: roles, dispatch rules, review levels, the six tasks with builder and reviewer, waves, hot files (`.github/workflows/ci.yml` in waves 2 and 3), the budget table, and a ledger with one row per task (branch, commit, red evidence, green evidence, runs used, reviewer verdict).
- After every task: a ledger row on the board, and an entry in `vault/Work Log/<date>.md` with the red output, the green output, the changed paths and the commit (plan "Execution contract").
- Every call the CEO makes: a note in `vault/Decisions/<date> <title>.md`, with the frontmatter the existing notes use.
- At the end of each working session: the daily report `vault/Reports/<date>.md` (what landed, runs used, what is red, what waits on the CEO). Update the "Current milestone" line in `vault/Home.md` and the QA0 line in `vault/Tasks.md`.
- Commit vault files with explicit paths, with `[skip ci]` when the commit holds only documents.

## 6. CI and done

- Push `codex/qa0-integration` after each wave and read the hosted run (`gh run list --branch codex/qa0-integration`). The `browser` job fails on `main` since before QA0 at "M2 browser proof" with a 15 s timeout after `Joined { slot: 0 }` (cause NOT TRACED). Record it; do not chase it unless its failure text changes. The `check` and `agents` jobs must be green.
- Done means: the six tasks merged into `codex/qa0-integration`; T6 ticks A1 to A21 with the 22 mutation runs recorded; the `check` and `agents` jobs green on the integration branch; the board ledger full; the daily report written.
- Do not merge into `main` and do not push `main`. Report to the CEO with the integration commit, the run count, the CI link, and what is not done. The CEO approves the merge, as for M3.

## 7. Commit messages

End every commit message with the attribution line the Codex session uses, and keep the subject in STE.
