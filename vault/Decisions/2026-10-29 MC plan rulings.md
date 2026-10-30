---
type: decision
date: 2026-10-29
tags:
  - turts
  - decision
  - mc
ai-first: true
source: mcp
status: approved
---

## For future Claude
Orchestrator rulings on the MC plan (`docs/superpowers/plans/2026-10-29-mc-player-control.md`) during the plan and gate 2. The CEO had decided K1 to K27 and the full design ([[2026-10-29 MC calls K1-K27 and budget]]); these rulings settle plan-level questions inside those decisions. Each ruling that changed the design text is a design revision (6 to 9).

**Producer questions O1 to O8.** O1 (a): `muster` ids renamed so the no-content scan passes (`dummy`, `rush`; revision 6). O2: probe P1 runs on the dev machine. O3: a refutation plant with an MC equivalent is re-targeted; a plant with none may retire only with a decision note that names the plant, the failure class and the MC plant that covers it; `qa-lead` reviews the list. O4 and O7: ORD2 gets a pixels half on `muster` (40 troopers in a box, the own `post` dropped by 4.1); `not_applicable` and `not_owner` are headless only (revision 6 and 7). O5: superseded by gate 2 ruling 13. O6: refinements R1 to R11 accepted (task 2 split into 2a and 2b; task 18 writes the forge orders; the G-REF gate for refutation plants). O8: the ORD1 row names "rush".

**Gate 2 round 1 rulings (22).** Tests that need `turts_server` code live in `crates/turts_server/tests/` with `#[path]`, no new crate. Content ids: `order_<verb>`, `key_<verb>`, `beacon` (revision 8). DET1 compares tick logs without the wall-clock column, up to tick N (revision 7). A server `--order-log` hook is the `Blocked` source (revision 7). The validator gets a phase mark on `OperationAudit`. A plain move writes `MoveTarget` only. `--ledger-limit` is a parameter (default 0 after re-check 1; fixtures pass 5). Task 20 traces the red "M2 browser proof" step first. Task 20 adds the `io` contract test. `turts_debug.frames` (revision 7; with wall time and applied input sequence in revision 9). Task 15 builds the "no orders" notice. Step order in every task: red, green, commit, gate, plants. PT1: the person is someone who has not read the design, named by the CEO, else the CEO with "not blind" in the log; the `player` agent reads nothing under `docs/` or `mods/`. The PT1 driver reads the effective input map, not literals. P1 runs on Chrome first; Firefox gets 10 minutes outside the sandbox, else NOT TRACED and the CEO picks K11. Task 9 owns the core camera state and removes `fit_scene_camera`; task 11 owns the camera logic; task 10 needs 11. `InputEvent` gains `Minimap { button, local }` and `over_widget` (revision 7). The CAM2 Esc clause moves to the P1 headed table. PROJ1 runs in the `check` job with `RenderPlugin` and no backend. The Codex contract gains: branch from `codex/mc-integration`, one worktree per lane, re-gate after each merge, push and read CI after each wave. Shared-file rows for `Cargo.lock`, the crate `Cargo.toml` files, `outbound.rs`, `session.rs` (task 4). Debug keys read typed resources. The 4.5 tick reading (t, then Orders of t+1) and the 6.3 merge rules (`group` 0 never merges; a waiting envelope is atomic) (revision 8).

**Budget.** The plan's total rose from about 184 to about 212 runs (critical path 55) after the gate 2 repairs. This is above the design range of 165 to 205 and goes to the CEO (plan G2).

Related: [[2026-10-29 MC calls K1-K27 and budget]]

## Ruling O9 (2026-10-30, Codex task 1 preflight stop)
Codex found a conflict: design 3.4 lists `default prevented` as a P1 table result; plan task 1 omitted the column (gate 2 qa: the page calls `preventDefault()` itself, so the value is always true). The design wins. Ruling: the P1 table carries `default prevented` next to `cancelable`; the value is recorded, not judged. The K11 rule (task 1 step 4) judges `reaches the page` and `browser acted` only. Plan task 1 "Probe facts" and step 2 updated. No design change.

## Ruling O10 (2026-10-30, Codex task 8 stop)
Design 2.2 names the effect `Shell { action }`. The `forge` resource `mods/forge/resources/shell.ron` has `id: "shell"`, `name: "Shell"`, `unit_name: "shell"`, so the no-content scan (`crates/turts_checks/tests/support/mod.rs`, exact token match on every Rust identifier in `crates/`) refuses the tokens `shell` and `Shell` in engine code (design 11.3). Ruling: the effect is `ShellAction { action }`; behaviour, data and the scanner do not change. Design revision 11. Also applied ahead of task 16: the module `crates/turts_client/src/shell.rs` of the plan is `client_shell.rs`, because `mod shell;` would hit the same token.

## Ruling O11 (2026-10-30, Codex belt toggle stop)
Design PT1 step 6 (design 17, player path) had the tester disable a belt piece. `SetEnabled` is kept from M3 unchanged (design 5.3) and refuses conveyors: `crates/turts_sim/src/command_m3.rs:73-80`, test `set_enabled_not_toggleable_for_belt`. Belt toggling is an economy mechanic, outside MC. Ruling: step 6 disables the Shell Press (a machine, toggleable). The press input (capacity 6, `mods/forge/units/press.ron`) fills from the arm loader at 2 items per second (swing 250 ms, return 250 ms) in at most 3 s; the loader waits with `DestinationFull`; the `InserterWaiting(5)` alert fires 5 s later: at most 8 s, within the 10 s of the step. The timing is computed from the content numbers, not run: NOT TRACED until PT1. Design revision 12. No task change: task 21 migrates the path as written.
