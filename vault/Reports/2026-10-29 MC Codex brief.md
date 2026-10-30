---
type: report
date: 2026-10-29
tags: [turts, mc, codex, brief]
---

## For future Claude
The brief for the Codex orchestrator that builds MC (player control and client foundation). Claude (Fable) wrote the design and the plan and ran both gates. Codex builds, reviews, validates and keeps the vault during the build. Claude takes over again for the "Doc updates for the orchestrator" after the merge. The QA0 brief ([[2026-10-29 QA0 Codex brief]]) is the model; this brief repeats what changed.

# MC build: brief for the Codex orchestrator

## 1. What to build

- Plan: `docs/superpowers/plans/2026-10-29-mc-player-control.md`. Passed gate 2 on 2026-10-29 (six lenses, two repair passes, three re-checks). Build it as written: 24 tasks (task 2 is 2a and 2b) in 11 waves, in the wave order of the plan. The CEO chose the full design: no cut-line item slips without the CEO (K21 a).
- Design: `docs/superpowers/specs/2026-10-29-mc-player-control-design.md` (revision 10, passed gate 1). Read the sections each task cites. The plan wins on a conflict with the design; report the conflict.
- Product spec: `RTS-Engine-Project-Specifications.md` version 0.8. Decided items are not open.
- CEO decisions in force: K1 to K27 as recommended, full design ([[Decisions/2026-10-29 MC calls K1-K27 and budget]]); the orchestrator rulings O1 to O8 and the gate 2 rulings ([[Decisions/2026-10-29 MC plan rulings]]). K11 (Ctrl+digit) is conditional on probe P1 (task 1).
- Base: `main` at the commit the CEO names (the plan says `f9353bb` or later). Create `codex/mc-integration` from it before wave 1. One branch per task, `codex/mc-t<N>` (`codex/mc-t2a`, `codex/mc-t2b`), each from `codex/mc-integration` after every task of its Needs has merged there.

## 2. Task 1 first, and its stop rule

Task 1 is probe P1: real key input in headed Chrome and Firefox on the dev machine. Chrome first. Firefox may not start under a sandbox (snap); try outside it for at most 10 minutes. If Firefox does not start, mark Firefox `NOT TRACED` in the P1 table and stop: the CEO picks K11 with the Chrome table. If P1 is red on a browser, stop: the CEO picks. No screen task (9 and later) starts before the P1 result is recorded, because the default keys of design section 3 depend on it.

## 3. Roles, models and budget

Follow `vault/M3 Execution Board.md` sections 1 to 3 and the plan "Execution contract". Codex does not load `.claude/agents/`: every dispatch tells the agent to read, in this order, its role file, `CLAUDE.md`, the plan task, and the design sections the task cites.

- Orchestrator (root): the main Codex session, Sol. Dispatches, merges into `codex/mc-integration`, runs the gate sets, keeps the board and the vault.
- Builder: the seat the plan "Budget" table names, Luna. Red first, then green.
- Reviewer: the seat the "Budget" table names, Sol. Reads the diff, runs the tests and one plant, cites `path:line`.
- Validator: `qa-lead` on the integration branch for tasks 21 to 24 and the plants.
- Task 23 (PT1, PT2): the `player` agent through the harness `io` only, with the goals only and no read under `docs/` or `mods/`; and a person. The CEO names the person (someone who has not read the design); when the CEO plays, the log records "not blind". Stop and ask the CEO who plays before task 23 starts.
- State the model and the reasoning effort on each dispatch.
- Budget: about 212 agent runs in all (106 task runs, 64 repair allowance, the gates already spent). Count every run on the board. Stop and report to the CEO at 170 build runs (106 + 64) before you spend more. A task that reaches twice its estimate stops and reports.

## 4. Standing rules for every dispatch

- Verify, never assume. Cite `path:line`. Label what you did not trace `NOT TRACED`. List what you ran. Agent summaries are leads, not proof.
- Red, then green, with the exit code on the same line (`; echo exit=$?`). A compile error is not a red. Never pipe a run through `tail`, `head` or `grep` before its exit code.
- Step order in every task: red, green, commit, gate, plants (gate 2 ruling 12). Never `git checkout --` a new or uncommitted file. Restore each plant and show `git status --porcelain` empty for the changed files.
- Stage explicit paths. Never `git add -A`.
- One worktree and one target directory per lane (`CARGO_TARGET_DIR=target/mc-t<N>`). Build binaries one at a time. Delete the worktree and its target directory after the merge.
- Gate sets (plan "Local gate commands"): G-RUST, G-QA, G-ACC, G-WEB, G-FROZEN, G-REF, as each task lists them. After each merge, re-gate the integration branch (G-RUST, G-QA, G-FROZEN; G-WEB when a client or server file changed). After each wave, push `codex/mc-integration` and read the hosted run.
- The frozen values: `tests/m3/m2_operation_stream.json` keeps its hash; the forge fixed point stays green; the refutation plants keep their red (G-REF, with the known red window of the plan).
- Do not edit `CLAUDE.md`, `.claude/agents/`, the design, or the product spec. The plan section "Doc updates for the orchestrator" is for Claude after the merge; do not do it.
- Write in ASD-STE100 Simplified Technical English. Four spaces per indent level.
- Stop and report to the CEO, do not decide: P1 red or Firefox absent (section 2); a frozen value must change; a plan step contradicts the design or the spec; a Decided item looks wrong; a content id of the task 18 table is red in the no-content scan; the budget stop; who plays task 23; a refutation plant that must retire (it needs a decision note, plan task 21).

## 5. The vault

- Create `vault/MC Execution Board.md` from the QA0 board: roles, dispatch rules, the 24 tasks with builder and reviewer, the 11 waves, the hot files (plan "Shared files and their order"), the budget table, and a ledger with one row per run.
- After every task: a ledger row, and an entry in `vault/Work Log/<date>.md` with the red output, the green output, the plant outputs, the changed paths and the commit.
- Every call the CEO makes: a note in `vault/Decisions/<date> <title>.md`, with the frontmatter the existing notes use.
- At the end of each working session: the daily report `vault/Reports/<date>.md`; update "Current milestone" in `vault/Home.md` and the MC line in `vault/Tasks.md`.
- Commit vault files with explicit paths, with `[skip ci]` when the commit holds only documents.

## 6. CI and done

- The `browser` job fails on `main` at "M2 browser proof" since before QA0. Task 20 step 1 traces the cause; task 21 fixes the step. Until then, read the `check` and `agents` jobs, and record the browser job state after each wave.
- Done means: the 24 tasks merged into `codex/mc-integration`; every acceptance row of design section 14 green with its red recorded (plan "Acceptance ownership"); the ledger `tests/browser/legacy-bridge.tsv` at 0 rows; PT1 and PT2 signed off by the person; the `check`, `agents` and `browser` jobs green on the integration branch; `docs/measurements/mc.md` written; the board ledger full; the daily report written.
- Do not merge into `main` and do not push `main`. Report to the CEO with the integration commit, the run count, the CI link, and what is not done. The CEO approves the merge.

## 7. Commit messages

End every commit message with the attribution line the Codex session uses, and keep the subject in STE.
