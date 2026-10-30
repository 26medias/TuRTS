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
- Task 23 (PT1, PT2): the `player` agent through the harness `io` only, with the goals only and no read under `docs/` or `mods/`; and a person. The CEO decided (2026-10-29): the CEO plays PT1 and PT2. The log records "not blind". Do not ask who plays; before task 23 starts, tell the CEO the page URL from `scripts/mc-browser.sh --serve-only` and wait.
- State the model and the reasoning effort on each dispatch.
- Budget: about 212 agent runs in all (106 task runs, 64 repair allowance, the gates already spent). The CEO approved this budget (2026-10-29). Count every run on the board. Stop and report to the CEO at 170 build runs (106 + 64) before you spend more. A task that reaches twice its estimate does not stop (CEO, 2026-10-30): the orchestrator decides whether to continue, records the reason on the board, and reports the overrun in the daily report. A task that reaches four times its estimate stops and reports.

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
- Decide yourself, do not stop (CEO, 2026-10-30, "allow it to take decisions"). Record each decision as a numbered Codex ruling (C1, C2, ...) on the board and in `vault/Decisions/<date> MC Codex rulings.md`, with the conflict, the choice, the `path:line` evidence, and the alternative you did not take. The orchestrator (Claude) reviews these rulings after each wave and can reverse one; a reversed ruling is a repair in the next wave. You may decide:
    - A name collision with the no-content scan or with an existing identifier: rename the engine side (a variant, module, file, test or field). Never rename content of `mods/forge` or `mods/muster` that a frozen value or an acceptance row names.
    - A plan step that disagrees with the design on an instrument detail (a column, a log field, a file name, a command flag, a test name, a fixture): the design wins; apply it and continue.
    - A plan step that the design does not cover: follow the design's nearest rule and the pattern the code already has.
    - A design step that an existing M2 or M3 mechanism cannot do as written (as the PT1 belt step, ruling O11): change the step to the nearest existing mechanism that keeps the same player-visible shape (the same keys, the same alert, the same timing bound), and record the computed timing as NOT TRACED until the step runs.
    - The order of tasks inside a wave, the number of lanes, cache and disk handling, worktree layout.
    - A review finding of Medium or lower: repair or defer with a reason on the board. A High finding is repaired and re-checked, always.
    - A task over twice its estimate (section 3).
- Stop and report to the CEO, do not decide: a frozen value must change (`tests/m3/m2_operation_stream.json`, the forge fixed point); a Decided item of the product spec or a K call looks wrong; a change to the design that changes what the player sees or does (a new key, a removed step, a changed acceptance row); the budget stop at 170 and a task at four times its estimate; the page URL for task 23 (the CEO plays); a refutation plant that must retire; a merge into `main`.

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
