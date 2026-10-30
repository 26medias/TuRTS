# MC Player Control and Client Foundation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Plan status:** DRAFT for gate 2 (2026-10-29). Author: `producer`. Commit of record for every `path:line`: `0b1a250` (the code is identical to `7a90f97`, the commit of the design citations: `git diff --stat 7a90f97 c1a5fd1 -- crates mods scripts tests` is empty, and `git diff --stat c1a5fd1 0b1a250` touches only the design). Orchestrator rulings O1 to O8 (2026-10-29) are applied. The critical path of this plan, 51 runs, supersedes the 46 runs of D section 15.2 (ruling O6: R1 to R11 accepted).

**Goal:** A person plays the M3 industrial demo (`mods/forge`) in a desktop browser with mouse and keyboard only, and 40 units sent to one point arrive as a group (S:722).

**Architecture:** A new crate `turts_interaction` holds the input map, selection, groups, smart command, command composition and camera state, with no `bevy*` dependency (K1). The wire goes to protocol 4 with an open order envelope and per-unit results (D73, D65). The simulation gets `SimSet::Orders`, a per-unit order queue and flat-ground group movement. Every HUD element during play is a mod widget (D67).

**Tech Stack:** Rust 2024, Bevy `=0.20.0`, lightyear `=0.31.0`, RON `=0.12.2`, `glam = "=0.33.12"` (K26), Python 3 standard library, Node 20 on CI (v24.10.0 local), playwright-core in `tests/browser`.

**Design:** `docs/superpowers/specs/2026-10-29-mc-player-control-design.md`, revision 6 (commit `0b1a250`), passed gate 1, "D" below (D section N; D:line at `0b1a250`). Product spec `RTS-Engine-Project-Specifications.md` v0.8, "S" below. **CEO decisions (2026-10-29):** K1 to K27 as recommended; K11 stays conditional on probe P1; K20 applied in revision 5; budget option 1 (the full design, no cut, about 184 runs). The design is the contract. When this plan and the design disagree, the design wins; report the conflict.

## Global Constraints

- `ENGINE_API_VERSION` `"0.2.2"` to `"0.2.3"` (`crates/turts_mod_api/src/lib.rs:33`). Append `"0.2.3"` to `RELEASED_ENGINE_APIS` (`lib.rs:36`). Additive: every 0.2.2 package loads unchanged (K13).
- `NET_PROTOCOL_VERSION` `3` to `4` (`crates/turts_net_protocol/src/lib.rs:13`, K14). The netcode `protocol_id` lines (`crates/turts_client/src/join.rs:275`, `crates/turts_server/src/net.rs:44`) do not change, but they read `NET_PROTOCOL_VERSION`, so the netcode id also becomes 4. A version 3 client then fails at the netcode handshake, before `Hello`. The test hook `--protocol-version 3` still reaches `judge_hello` (`crates/turts_server/src/session.rs:97`).
- Four new `VERSION_TABLE` rows (`crates/turts_mod_runtime/src/version_table.rs:20-31`), each `since_api: "0.2.3"`, present when the list is not empty: `exports.orders`, `exports.interface`, `exports.input`, `exports.assets`. `VersionTableRow.present` takes the parsed package content, not only `&Manifest` (`version_table.rs:6-10`; D section 5.2, 11.4).
- Every new schema type has `#[serde(deny_unknown_fields)]`. The four new `Exports` lists have `#[serde(default, skip_serializing_if = "Vec::is_empty")]` (D section 11.1).
- No Bevy type in `turts_mod_api`, `turts_net_protocol` or `turts_interaction` (D15). `turts_interaction` forbids `bevy*`, `wgpu`, `winit`, `turts_sim` on host and wasm32 (K1, D section 2.4). `turts_client` and `turts_client_web` never depend on `turts_sim` (T1, `scripts/boundary.sh:38-42`).
- Frozen evidence, no re-freeze (K19 a): `sha256sum tests/m3/m2_operation_stream.json` prints `bcbf72c2206251f635a7c7f5b6c8f573d69a5fc05a5bd77716e98ebfb08e370b` (pinned at `tests/m3/refutations.py:18`). `m2_operation_stream_matches_baseline` (`crates/turts_pack/tests/m3_m2_stream.rs:350`) and `forge_spec_13_4_fixed_point_and_ten_vs_five_shots` (`crates/turts_pack/tests/forge_fixed_point.rs:10`) stay green with no edit to their expected values. A task that must change either one stops and reports (the CEO approves a re-freeze, K19).
- Numbers from the design, verbatim: envelope cap 4,096 units, the client splits above it with one `group` (K4 b). Rate buckets per slot: 64 envelopes with refill 640 per second; 16,384 unit orders with refill 163,840 per second (K25). Spreading: at most 2,048 units per tick for all slots; pending list 16,384 units per slot (D section 6.3). `queue_depth` from 1 to 255, default 16 (K24). Spacing s = 2 x r_max + 0.1; separation reach r_i + r_j + 0.05; landing within one step or 0.025; stall 1 s; at most 3 slot claims (D sections 6.3 to 6.5). R_near = r_max x (sqrt(N / 0.6) + 1), 4.58 for 40 units of radius 0.5 (K17).
- K12 caps: decoded image bytes at most 256 MiB (width x height x 4 from the PNG IHDR) per package and per locked set; each image at most 2048 x 2048 px; each cursor image at most 128 x 128 px; font files at most 8 MiB; sound files at most 32 MiB; sounds at most 300 s in total and 30 s each. The decoder does not run on the server.
- K18: `turts-pack` warns with exit 0 below 11 px text and below 24 x 24 px buttons.
- Client numbers: double click 400 ms and 4 px; drag threshold 4 px; hover tooltip 0.5 s; edge band 8 logical px; UI scale 0.75 to 2.0; alert cooldown at least 1 s, at most 2 alert sounds, last 5 lines; move marker until rest, limit 60 s; `Blocked` mark 2 s; rejoin retry each 1 s, countdown 15 s; netcode timeout 10 s (`crates/turts_client/src/join.rs:278`). Palette Okabe-Ito `#0072B2`, `#E69F00`, `#009E73`, `#D55E00`, `#56B4E9`, `#CC79A7`, `#F0E442`, `#000000`; CIEDE2000 distance at least 20 for each pair under protan, deutan, tritan (Machado 2009).
- Storage keys (D section 10.4): `turts.v1.player_id` (`sessionStorage`), `turts.v1.bindings.<mod id>.<line>`, `turts.v1.settings`, `turts.v1.groups.<match_id>.<slot>`.
- New dependencies: only `glam = "=0.33.12"` (task 8) and the Bevy feature `audio` on `turts_client` (task 15). Any other new crate or npm package needs the orchestrator.
- No task edits `CLAUDE.md`, `.claude/agents/`, the design, or the product spec. Doc changes go to "Doc updates for the orchestrator".
- Code style: 4 spaces per indentation level in Rust, Python, JavaScript, RON, TSV and shell examples. Write prose in ASD-STE100.

## Review Focus

1. A page reload while the old session still lives (up to the 10 s netcode timeout): the player expects the countdown and the same slot, not "no open slot". Pinned in task 16 (SLOT1 steps 3 and 4).
2. A box drag that holds only own static units, or own and foreign units together: the player expects the static units when no mobile unit is in the box, and never a foreign unit. Pinned in task 13 (SEL2 step 2, cases `static_only` and `mixed_owner`).
3. An order to a point outside the map, or a NaN or infinite target from a bad client: the player expects a move to the edge, and the server expects `target_not_finite`. Pinned in task 22 (GMU).
4. A 0.2.2 package with no orders (an old mod): the player expects "This game defines no orders" once, not a silent client. Pinned in task 18 (step 1, `old_acme_copy_loads_with_no_orders`).
5. A corrupt `localStorage` value or a duplicated browser tab: the player expects defaults with one notice, and the refusal "This game is open in another window". Pinned in task 16 (PER1 step 5, SLOT1 step 4).

## Execution contract

- Base: `main` at `0b1a250` or later. Create the integration branch `codex/mc-integration` from it before wave 1. One branch per task, `codex/mc-t<N>` (task 2 has two: `codex/mc-t2a`, `codex/mc-t2b`). Merge each task into `codex/mc-integration` only after its local gate is green.
- Codex does not load `.claude/agents/`. Each dispatch tells the agent to read, in this order: its role file (the path in the task), `CLAUDE.md`, this plan task, and the design sections the task cites (`vault/Reports/2026-10-29 QA0 Codex brief.md` section 3).
- Models (`vault/Team.md:42`): the builder runs on Luna (execution); the reviewer runs on Sol (review); the orchestrator runs on Sol. State the model and the reasoning effort on each dispatch.
- One target directory per lane: `CARGO_TARGET_DIR=target/mc-t<N>`. Build each binary with its own `cargo build -p <crate>`. Delete a lane target directory after its merge.
- Stage explicit paths. Never `git add -A`. Never pipe a red or a green run through `tail`, `head` or `grep` before you read its exit code: write `; echo exit=$?` after the command.
- "Red" means a test that compiles and fails, or a tool that exits 1 with its failure line. A compile error is not a red. When a test names a new item, add the item first as a stub that compiles (signature first).
- Two kinds of red. **Red first:** the owner writes the test before the feature, and the test fails on the build before the feature. **Red by plant:** the feature merged in an earlier task, so the owner applies the red build that D section 14 names as a plant, runs the test, and restores. Every acceptance row of D section 14 has one of the two, in the table "Acceptance ownership".
- Plants, trust runs and mutation runs come after the task commit. Commit the task files first. Then edit, run, and restore with `git checkout -- <file>` (for a file that the run created: `rm <file>`). Then `git status --porcelain -- <the changed files>` must be empty. Never use `git checkout --` on a new or uncommitted file.
- Record each task's red output, green output, changed paths, commit and run count in `vault/Work Log/` and on the board `vault/MC Execution Board.md`.
- Stop and report to the orchestrator, do not decide: P1 is red on Firefox (task 1 step 4); a frozen value changes; a plan step contradicts the design or the spec; a Decided item looks wrong; the no-content scan is red on a name that the design fixes; a task uses twice its run estimate; the budget reaches 184 runs.

### Local gate commands

Each task names the sets it runs. Run them in the task branch, after the commit, in this order.

- **G-RUST:** `cargo fmt --all --check; echo exit=$?`, `cargo clippy --workspace --all-targets --locked -- -D warnings; echo exit=$?` (`.github/workflows/ci.yml:29`), `cargo check --workspace --exclude turts_server --target wasm32-unknown-unknown --locked; echo exit=$?` (`ci.yml:31`), `cargo test --workspace --locked; echo exit=$?` (`ci.yml:33`), `bash scripts/boundary.sh; echo exit=$?` (`ci.yml:35`). Each prints `exit=0`.
- **G-QA:** `python3 scripts/qa/yardstick.py --check; echo exit=$?` (prints `yardstick matches inventory`), `python3 scripts/qa/completeness.py --self-test && python3 scripts/qa/completeness.py --enforce; echo exit=$?` (prints `self-test: 21/21 cases passed; failures=0` and `checked specs: 2`), `python3 scripts/qa/driver_rule.py --self-test && python3 scripts/qa/driver_rule.py tests/browser; echo exit=$?`. Each prints `exit=0`.
- **G-ACC:** `bash scripts/acceptance.sh; echo exit=$?` prints `exit=0`.
- **G-WEB:** `(cd crates/turts_client_web && trunk build --release); echo exit=$?`, then `bash scripts/t7.sh; echo exit=$?`. Both print `exit=0`. After task 21, `t7.sh` runs `t7-driver.js` and `t7-oracle.js`.
- **G-FROZEN:** `sha256sum tests/m3/m2_operation_stream.json` (the value of Global Constraints), `cargo test -p turts_pack --locked --test m3_m2_stream --test forge_fixed_point --test forge_cases; echo exit=$?` prints `exit=0`.
- **G-REF(files):** the refutation cases that plant into the files the task changed (table below). Run `bash scripts/m2-refutations.sh --case <ids>; echo exit=$?` and `bash scripts/m3-refutations.sh --case <ids>; echo exit=$?`. Each prints `exit=0`. A plant anchor must occur exactly once (`tests/m2/refutations.py:270-280`). When the task moved the anchor code, the task edits that case's `anchor` and `replacement` in `tests/m2/refutations.json` or `tests/m3/refutations.json`, so the plant breaks the same behaviour in the new code. `qa-lead` reviews each such edit. Two tasks of one wave that edit the same registry merge in task-number order; the second rebases.
- **G-PIX(rows):** `bash scripts/mc-pixels.sh <row file>...; echo exit=$?` (task 20) runs each row twice; prints `exit=0`.

Refutation cases by planted file (from `tests/m2/refutations.json` and `tests/m3/refutations.json` at `0b1a250`; browser cases have `"runtime": "browser"`):

| Planted file | M2 cases | M3 cases | MC tasks that edit it |
| --- | --- | --- | --- |
| `crates/turts_sim/src/command.rs` | E6-carried-item-loss | - | 4, 6 |
| `crates/turts_server/src/session.rs` | E7-refused-hello-claims-slot, E8-rejection-ack-send-reversal | - | 2, 7 |
| `crates/turts_server/src/export.rs` | E9-owner-private-raw-dto, E9-output-in-snapshot-and-delta | - | 2, 4 |
| `crates/turts_client/src/ingest.rs` | E9-midtrace-rearm-ignored; browser: E9-browser-duplicate-outstanding, E9-browser-ignore-midtrace-rearm | - | 2 |
| `crates/turts_client/src/replica.rs` | E9-duplicate-spawn-tail-replay; browser: E9-browser-position-only-snapshot, E9-browser-unbounded-buffer | - | 2 |
| `crates/turts_client/src/economy_ui.rs` | browser: E2-browser-hardcoded-label, E9-browser-missing-button, E9-browser-debug-only-ui | - | 2, 12 (removed) |
| `crates/turts_client/src/render.rs` | browser: E9-browser-missing-output-mesh | - | 9, 11, 13, 14 |
| `crates/turts_sim/src/economy/gather.rs` | E5-conversion-cargo-separation, E6-node-replenishment, E6-duplicate-conversion, E6-self-store, E6-no-op-economy, E3-declared-conversion-coefficient, E8-foreign-gather-owner-check | - | 4 |
| `crates/turts_sim/src/economy/transaction.rs` | E4-local-capacity, E6-mixed-cost-partial-commit, E6-arithmetic-overflow, E6-duplicate-refund, E6-repeated-spawn, E6-capacity-bypass, E6-event-only-spawn, E8-mixed-cost-partial-commit | C10-inserter-state-survives-despawn | 4 |
| `crates/turts_sim/src/plugin.rs` | - | C15-combat-before-production | 4 |
| `crates/turts_sim/src/economy/combat.rs` | - | C13-ammo-debit-saturates, C14-health-zero-does-not-destroy, C16-double-range | 5 |
| `crates/turts_checks/tests/support/mod.rs` | E1-derived-scan | - | 3 |

### Known red window

`main` already has one red step. Task 12 adds two. Task 21 makes all three green. No other step may be red on `codex/mc-integration`.

| CI step (`.github/workflows/ci.yml`) | Red from | Green at | Cause |
| --- | --- | --- | --- |
| "M2 browser proof" (`:115-116`; `scripts/m2-browser.sh:326` runs `tests/browser/m2.js`) | before QA0 (run 40943729334; D section 1.6) | task 21 | NOT TRACED (15 s timeout after `Joined { slot: 0 }`) |
| "M3 browser controls" (`:117-122`, `scripts/m3-browser.sh:82` runs `tests/browser/m3.js`) | task 12 | task 21 | `m3.js` reads the engine panel through `turts_ui`; task 12 removes the panel |
| "M2 browser-only semantic refutations" (`:123-124`) | task 12 | task 21 | three cases plant into `economy_ui.rs`, which task 12 removes; the step also needs `m2-browser.sh` |

The `browser` job stops at its first red step, so T7 (`:109-114`), which comes first, must stay green in every task.

## Plan refinements of the design

The design wins on a conflict. These refinements fill gaps; none changes a Decided item, a K call, or a number. Gate 2 can refuse each one.

| # | Refinement | Reason (evidence) |
| --- | --- | --- |
| R1 | The K5 lift (`content_m3_validation.rs:380-383`) moves from task 5 to task 3. Task 19 then needs 3 only. | `muster` troopers have a weapon and movement (D section 12.3); the package cannot load before the lift. Task 19 can then serve tasks 5, 10 to 14 and 22 earlier. The sim already fires from any unit with a weapon (`combat.rs:165-200`). |
| R2 | Task 5 needs task 6. | Attack on a point moves to the slot of the point (D section 5.2) and target choice uses the neighbour grid of D section 6.4; both are task 6. |
| R3 | Task 2 lands in two merges: 2a (new wire types, additive, no version change) and 2b (the switch to protocol 4). 2b needs 3 and 18. | After the switch, `--move` sends "the first `OrderDef` of the package whose behaviour is `Move`" (D section 5.5). T7 uses `acme` (`scripts/t7.sh:30`) and `scripts/acceptance.sh:120` runs an F11 process test on `forge`. Neither package has orders before task 18. |
| R4 | Task 18 migrates the orders part of all five packages (`acme`, `base`, `cedar`, `haul`, `forge`): `orders/`, `orders/rules.ron`, `input/bindings.ron`, `engine_api: "^0.2.3"`, new version. Task 17 adds the forge interface, assets and guide only. | Same as R3: task 2b needs forge orders; task 17 needs the widget runtime (task 12) and comes much later. |
| R5 | Task 9 creates stub modules in `turts_client` for tasks 10 to 16 (`actions.rs`, `camera.rs`, `widgets/mod.rs`, `visuals.rs`, `minimap.rs`, `feedback.rs`, `audio.rs`, `shell.rs`, `persist.rs`) and registers them in `build_app`. It also moves `Selection` and `UiPanelRoot` from `economy_ui.rs` to a new `ui_shared.rs`, and adds every new `turts_debug` key and `--out` key with an empty value. | `crates/turts_client/src/lib.rs` is shared by 2, 9, 12, 16, and `debug.rs` by 2, 9, 12 (D section 15.2 lists 9, 10, 12, 16 for `lib.rs` in order, but the Needs column puts 10 and 12 in one wave). `render.rs:17,247,540`, `debug.rs:66`, `live_view.rs:22`, `input.rs:5` import `economy_ui` types; the move lets task 12 remove `economy_ui.rs` without an edit to `render.rs` (owner order 9, 11, 13, 14). |
| R6 | Task 12 also writes the HUD1 driver `hud-driver.js` and `hud-oracle.js`, removes `tests/browser/m3-mobile.js` and its ledger row (5 to 4 rows). Task 21 rewrites the other four ledger files. | HUD1 can go red first only before the panel removal, which is task 12. D section 13.3 gives `m3-mobile.js` to `hud-driver.js` + `hud-oracle.js`. |
| R7 | Task 12 changes the page canvas to 100 % of the viewport (`crates/turts_client_web/index.html:8`). Task 16 adds `tabindex` and focus. | HUD1 and PROJ1 test 1920 x 1080; the fixed `#wrap{width:1280px;height:720px}` makes that viewport meaningless. |
| R8 | Pixels rows run through one row file each, `tests/browser/rows/<ROW>.json`, and one runner `scripts/mc-pixels.sh` (task 20). Task 24 adds one CI step for the runner. | D section 14: hosted CI runs each row except "headed" rows. One file per row means no two tasks edit one CI file. |
| R9 | The tick-cost record (D section 6.7) and VIS2 write `docs/measurements/mc.md` in the order 13, 22, 24. Task 22 needs 13. | One file, three writers. |
| R10 | Acceptance rows with a pixels half are owned by the first client task in which every half can be built (table "Acceptance ownership"). The simulation tasks prove their parts with unit tests. | D section 15.1 names no owner for the pixels halves. |
| R11 | `--actions`, `--out` and per-unit results use the debug keys that task 9 declares (R5); task 10 edits no shared client file except `options.rs`. | Same as R5. |

## Waves

Tasks in one wave share no file, except the refutation registries (merge in task-number order, G-REF).

| Wave | Tasks | Order reason |
| --- | --- | --- |
| 1 | 1, 2a, 3, 20 | no needs |
| 2 | 8, 18, 19 | 8 needs 2a, 3; 18 and 19 need 3 |
| 3 | 2b | needs 3, 18 (R3) |
| 4 | 4, 7, 9 | 4 needs 2b, 3; 7 needs 2b; 9 needs 1, 8, 2b |
| 5 | 6, 10, 11, 12 | 6 needs 4; 10, 11 need 9, 19, 20; 12 needs 3, 9, 19, 20 |
| 6 | 5, 13 | 5 needs 4, 6, 19 (R2); 13 needs 9, 11, 19, 20 |
| 7 | 14, 15, 17, 22 | 14 needs 5, 12, 13; 15 needs 7, 12, 13; 17 needs 3, 12, 13, 19, 20; 22 needs 6, 7, 11, 13, 19, 20 |
| 8 | 16 | needs 7, 12, 15 |
| 9 | 21 | needs 14, 16, 17, 18, 20 |
| 10 | 23 | needs 14, 15, 21, 22 (and so every task) |
| 11 | 24 | needs 23 |

Needs, after the refinements (the source for the waves; scratch `deps.py` checks it for cycles):

```text
1: -            2a: -            3: -              20: -
8: 2a 3         18: 3            19: 3             2b: 2a 3 18
4: 2b 3         7: 2b            9: 1 8 2b
6: 4            10: 9 19 20      11: 9 19 20       12: 3 9 19 20
5: 4 6 19       13: 9 11 19 20
14: 5 12 13     15: 7 12 13      17: 3 12 13 19 20 22: 6 7 11 13 19 20
16: 7 12 15     21: 14 16 17 18 20                 23: 14 15 21 22     24: 23
```

Critical path (scratch run `deps.py`, no cycle): 3, 18, 2b, 9, 11, 13, 15, 16, 21, 23, 24 = 6 + 2 + 4 + 6 + 4 + 5 + 5 + 6 + 8 + 3 + 2 = **51 runs in series**. This supersedes the design path of 46 runs (D section 15.2, reproduced by the same script; ruling O6). The 5 extra runs come from R3 (2b after 18) and from 13 after 11. The total does not change.

### Shared files and their order

| File | Order of tasks | Note |
| --- | --- | --- |
| `crates/turts_net_protocol/src/messages.rs` | 2a, 2b | D section 15.2 |
| `crates/turts_mod_api/src/schema.rs` | 3 | D section 15.2 |
| `crates/turts_client/src/economy_ui.rs` (removed) | 2b, 9, 12 | 9 moves two types out (R5) |
| `crates/turts_client/src/render.rs` | 9, 11, 13, 14 | 9 changes one import (R5) |
| `crates/turts_client/src/lib.rs` | 2b, 9, 12, 16 | R5 |
| `crates/turts_client/src/debug.rs` | 2b, 9, 12 | R5, R11 |
| `crates/turts_client/src/input.rs` | 2b, 9 | 9 replaces the click path |
| `crates/turts_server/src/session.rs` | 2b, 7 | D section 15.2 |
| `crates/turts_server/src/orders.rs` (new) | 2b, 4, 5 | envelope mapping |
| `crates/turts_server/src/export.rs` | 2b, 4 | owner-only `UnitOrderRecord` |
| `crates/turts_sim/src/economy/views.rs`, `crates/turts_server/src/economylog.rs` | 4, 6 | `AuditState`, validator (K19 a) |
| `crates/turts_sim/src/command.rs` | 4, 6 | per-unit validation, clamp (K6) |
| `crates/turts_client_web/index.html`, `crates/turts_client_web/src/lib.rs` | 2b, 12, 15, 16 | D section 15.2 adds 12 (R7, `turts_scene.widgets`) |
| `scripts/boundary.sh`, `crates/turts_checks/tests/boundary.rs` | 8 | D section 15.2 |
| `crates/turts_checks/tests/support/mod.rs` | 3 | token derivation of the new lists |
| `scripts/qa/driver_rule.py` | 20, 21 | role `harness`, then `LEDGER_LIMIT` |
| `tests/browser/legacy-bridge.tsv` | 12, 21 | R6 |
| `.github/workflows/ci.yml` | 21, 24 | |
| `docs/measurements/mc.md` (new) | 13, 22, 24 | R9 |
| `Cargo.lock` | 8, 15 | the two new dependencies |
| `crates/turts_client/Cargo.toml` | 9, 15 | `turts_interaction`; Bevy `audio` |
| `scripts/acceptance.sh` | 7, 10, 22 | new ignored process tests |
| `mods/forge/manifest.ron` | 18, 17 | orders lists, then interface and assets lists (R4) |
| `crates/turts_server/src/main.rs` | 2b, 7 | `mod orders;`, then `--command-script` |
| `crates/turts_sim/src/orders.rs` (new), `crates/turts_sim/src/movement.rs` | 4, 6, 5 | |
| `crates/turts_interaction/src/*.rs` | 8, 9 | later tasks plant there and restore |

## Budget

The estimate is the design estimate (D section 15.1). One run is one agent dispatch (builder or reviewer). Count every run on the board. Stop and report to the CEO at 184 runs before you spend more.

| Task | Runs | Builder seat (Luna) | Reviewer seat (Sol) |
| --- | --- | --- | --- |
| 1 | 3 | web-platform-engineer | ux-designer |
| 2 (2a 2, 2b 4; plan split) | 6 | network-engineer | mod-runtime-engineer |
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
| 23 | 3 | player agent and a person | ux-designer |
| 24 | 2 | orchestrator (Sol); qa-lead runs the REG1 plants | - |
| Subtotal | 106 | | |
| Gate 1 and gate 2 | 14 | | |
| Repair allowance (60 %) | 64 | | |
| **Total** | **about 184 (range 165 to 205)** | | |

Risk to the estimate: tasks 13, 14, 15 and 16 own 4, 4, 5 and 7 acceptance rows (R10). Expect the repair allowance there.

## Acceptance ownership

Each row of D section 14 has one owner. "First" is red first; "plant" is red by plant (Execution contract). "CI" is hosted CI; "check" is the `check` job; "browser" is the `browser` job.

| Row | Owner | Red build (D section 14) | How the owner shows red | Runs on |
| --- | --- | --- | --- | --- |
| KM1 | 16 | core matches the logical key; or no store write | first (no store before 16); plant: match `key`, not `code` | browser + check |
| KM2 | 16 | `prevent_default_event_handling: false` (`lib.rs:254`); or no focus handling | plant `lib.rs:254`; first (no overlay) | browser; P1 table headed local (task 1) |
| KM3 | 9 | double-click window 0; Shift ignored | first | check |
| SEL1 | 17 | pick radius 0; or a foreign click selects it for orders | plant x2 | browser |
| SEL2 | 13 | world rectangles; or a frame late | plant x2 | browser + check |
| SEL3 | 13 | type select ignores the screen | plant | browser |
| SEL4 | 16 | groups not stored; or dead ids kept | first; plant | browser |
| SLOT1 | 16 | today's `judge_hello`; or no retry | plant (revert the task 7 branch); first (no retry) | browser + check |
| ORD1 | 5 | no wire form for a behaviour; or id strings in server code | plant x2 | check |
| ORD2 | 15 | all-or-nothing `validate_move_to` (`command.rs:255-294`) | plant | check + browser |
| ORD3 | 14 | append as replace; stop keeps the queue; or no completion | plant x3 | check + browser |
| ORD4 | 14 | attack-move as move; or `choose_target` ignores the ordered target | plant x2 | check + browser |
| ORD5 | 14 | smart command resolved once for the selection | plant | browser + check |
| ORD6 | 15 | reason texts unmapped; or no rate limit | plant x2 | browser + check |
| ORD7 | 15 | marker on `OrderResult`; or one more tick | plant x2 | browser + check |
| GM40 | 22 | spreading off | plant | check + browser |
| GMA | 22 | avoidance off; or a `Blocked` unit rests inside another | plant x2 | check |
| GMU | 22 | clamp off; NaN kept; or slots reused | plant x3 | check |
| DET1 | 22 | neighbour iteration in `HashMap` order | plant, run first (D section 14) | check (`scripts/acceptance.sh`) |
| EL1 | 22 | missing `AuditState` input; a write outside; or today's validator | plant x3 | check |
| CAM1 | 11 | the `PostUpdate` fit (`render.rs:455-456`); or no `PointerLeft` | first | browser |
| CAM2 | 16 | no engine cursor under lock | plant | browser |
| PROJ1 | 9 | field of view not copied into `Projection` | plant | check, or local `--ignored` (step 9.2) |
| HUD1 | 12 | an engine-docked panel (`economy_ui.rs:916-934`) | first | browser + check |
| HUD2 | 17 | stale or empty source; or a tooltip with no key | plant x2 | browser |
| HUD3 | 3 | validator accepts; cap reads file bytes; or no version row | first | check |
| HUD4 | 12 | one cursor for all contexts | plant | browser |
| MM1 | 14 | the minimap click does not move the camera | first | browser |
| VIS1 | 13 | one material (`render.rs:403-406`); or a palette pair under 20 | first; plant | browser + check |
| VIS2 | 13 | no record | first (the row check reads `mc.md`) | headed local |
| ALR1 | 15 | no derivation; or no cooldown | first; plant | check + browser |
| AUD1 | 15 | no resume hook | plant | browser |
| SH1 | 16 | scroll speed ignored; or `UiScale` times the device factor | plant x2 | browser |
| PER1 | 16 | no write; or a corrupt value stops the client | first; plant | browser + check |
| T1X | 8 | the core depends on Bevy; or a row deleted | first | check |
| HD1 | 10 | the headless build has no core (`lib.rs:278-280`) | first | check + browser |
| DRV1 | 21 | a driver that reads a bridge | plant | agents job |
| MOD1 | 18 | a missing version row | plant | check |
| REG1 | 24 | a single unit off its point; or the ordered target alters the forge stream | plant x2 | check + CI |
| PT1 | 23 | an oracle step the tester cannot reach | plant (guide page 5 removed) | person and `player` agent |
| PT2 | 23 | spreading or arrival off | plant (the GM40 plant) | person and `player` agent |
| CI1 | 24 | any red job | the hosted run | CI |

Coverage: every `in` row of `## Yardstick coverage` (81 rows, D:1442-1632) names one of 27 tests, and every one of the 27 is a row above (scratch run `coverage.txt`: "tests in coverage not in acceptance: []"). By owner: task 3 HUD3 (MD08); 9 KM3 (IN03, IN04); 11 CAM1 (CA01, CA02, CA03, CA13, CA14); 12 HUD4 (PC21, UI23); 13 SEL2 (PC02, PC04, PE04), SEL3 (PC05), VIS1 (PC23, UI12, TM03, AN06, AN07); 14 MM1 (CA08, CA09, UI05, UI08), ORD3 (OR02, OR08, OR11, OR12, EC04), ORD4 (OR03, OR04, OR05, CB03), ORD5 (PC22, OR01, OR17, EC02); 15 ORD2 (PC17, OR24), ORD6 (OR22, OR31), ORD7 (OR23, PF05, PE01, PE07), ALR1 (UI16, AU02), AUD1 (MD09, AU01, AU06); 16 KM1 (IN02), KM2 (IN08), SEL4 (PC08, PC09, PC10, PC19), CAM2 (AC05), SH1 (CA16, UI22); 17 SEL1 (IN01, PC01, PC16, PC20), HUD2 (IN05, UI01, UI02, UI03, UI04, UI09, UI10, UI11, PR10); 18 MOD1 (MD01); 22 GM40 (OR25, GM01, GM09, PF01), GMA (GM04, GM06), GMU (GM10, PF04, MP09).

---

## Wave 1

### Task 1: Probe P1 (keys, pointer lock, audio resume)

**Seat and model:** `.claude/agents/web-platform-engineer.md`, Luna. Review: `ux-designer` (Sol) reads the table and the K11 rule.

**Branch / needs / wave:** `codex/mc-t1`; none; wave 1. **Design:** sections 3.2, 3.3, 3.4, 9.5; K9, K11. **Owns rows:** none. Task 16 attaches its table to the browser-capture row; tasks 3 and 9 wait for its K11 result.

**Files (create):** `docs/research/mc/p1-probe/page.html` (logs each `keydown`: `code`, `key`, modifiers, `defaultPrevented` after the handler, `document.hasFocus()`, pointer lock state, `AudioContext.state`; it calls `preventDefault()` on every `keydown` and `contextmenu`, as winit does with `prevent_default_event_handling: true`, `crates/turts_client/src/lib.rs:254`); `docs/research/mc/p1-probe/server.py` (standard library; serves the page and appends each POSTed record to a JSONL file); `docs/research/mc/p1-probe/run.sh` (private Xvfb display, headed browser, `xdotool` keys); `docs/research/mc/p1-keys.md` (the table); `docs/research/mc/p1-probe/SHA256SUMS`.

**Probe facts:** P1 runs on the dev machine, not on hosted CI or GCP (ruling O2). `/usr/bin/google-chrome`, `/usr/bin/firefox`, `/usr/bin/xdotool` and `/usr/bin/Xvfb` exist on the dev machine (plan run). `firefox --version` did not return within 120 s in the plan run: record the Firefox version from `navigator.userAgent` in the page log. CDP input bypasses browser accelerators, so the probe never uses CDP (D section 3.4). Safari stays NOT TRACED.

- [ ] **Step 1: The control rows first.** Write `run.sh` to start a new browser for each control key and send, in each browser, Ctrl+T and Ctrl+W (reserved in Chromium and Firefox, D section 3.2). Run `bash docs/research/mc/p1-probe/run.sh chrome control; echo exit=$?` and the same for `firefox`. Expected: for each control key, the row says "browser acted: yes" (a new tab or a closed window, read with `xdotool search --name`) and `exit=0`. If a control row says "browser acted: no", the probe cannot see a browser action: stop and fix the probe. This is the red of the instrument.
- [ ] **Step 2: The key table.** Send every row of D table 3.3, Ctrl+Digit1 to Ctrl+Digit9, Shift+Digit1 to Shift+Digit9, Digit1 to Digit9, F1 to F12, Space, Tab, Esc, and Esc while the pointer is locked (a click calls `requestPointerLock()` first). Write `docs/research/mc/p1-keys.md`: one row per key and browser, columns `key`, `browser`, `version` (the Chrome and Firefox versions of the dev machine, ruling O2), `reaches the page`, `default prevented`, `browser acted`.
- [ ] **Step 3: Pointer lock and audio.** Record: the lock starts on a click; the first Esc ends the lock and whether a `keydown` reaches the page; `AudioContext.state` is `suspended` before the first click and `running` after `resume()` in the click handler (D section 9.5).
- [ ] **Step 4: The K11 rule.** K11 (a) holds only when, in both browsers, every Ctrl+Digit1..9, Shift+Digit1..9, Digit1..9, F1, F10, Space and Tab row has "reaches the page: yes", "default prevented: yes", "browser acted: no". If a Firefox row (or a Chrome row) fails, **stop**: send the table to the orchestrator. The CEO picks K11 with the table. Tasks 3 (step 8) and 9 (step 3) wait for the pick. Write the result line at the top of `p1-keys.md`: `K11 (a) holds: yes` or `K11 (a) holds: no; rows: <list>`.
- [ ] **Step 5:** `cd docs/research/mc/p1-probe && sha256sum page.html server.py run.sh > SHA256SUMS && sha256sum -c SHA256SUMS; echo exit=$?` prints `exit=0`. Gate: G-QA. Commit the five files. **Runs: 3.**

### Task 2: Protocol 4 (2a: types; 2b: the switch)

**Seat and model:** `.claude/agents/network-engineer.md`, Luna. Review: `mod-runtime-engineer` (D73 seat), Sol, for 2a and 2b.

**Branch / needs / wave:** 2a `codex/mc-t2a`, none, wave 1. 2b `codex/mc-t2b`, needs 2a, 3, 18 (R3), wave 3. **Design:** sections 5.1, 5.4 (reasons only), 5.5; K4, K14. **Owns rows:** none (ORD1 is task 5, ORD2 task 15). 2b keeps T7, G-ACC and G-FROZEN green.

**Interfaces (produces, `turts_net_protocol::messages`):**

    pub struct OrderEnvelope { pub order: ContentRef, pub units: Vec<UnitId>, pub target: OrderTarget,
                               pub queue: QueueMode, pub group: u32 }
    pub enum OrderTarget { None, Point(Pos), Unit(UnitId), Node(NodeId), Job(JobId), Content(ContentRef) }
    pub enum QueueMode { Replace, Append }
    pub struct UnitRefusal { pub unit: UnitId, pub reason: CommandRejectReason }
    pub struct OrderResult { pub seq: u32, pub tick: u32, pub accepted: Vec<UnitId>, pub refused: Vec<UnitRefusal>,
                             pub envelope: Option<CommandRejectReason>, pub job_id: Option<JobId> }
    pub enum Activity { Idle, Pending, Moving, Attacking, Holding, Gathering, Blocked }
    pub struct OrderSummary { pub order: ContentRef, pub target: OrderTarget }
    pub struct OrderFailure { pub order: ContentRef, pub reason: CommandRejectReason, pub tick: u32 }
    pub struct UnitOrderRecord { pub unit: UnitId, pub activity: Activity, pub current: Option<OrderSummary>,
                                 pub queued: Vec<OrderSummary>, pub last_failure: Option<OrderFailure> }

2b: `PlayerCommand { seq: u32, order: OrderEnvelope }`; `Welcome { slot_id, tick, tick_hz, match_id: u64 }`; `Snapshot.unit_orders` and `WorldDelta.unit_orders: Vec<UnitOrderRecord>`; `crates/turts_server/src/orders.rs`: `pub fn to_sim(slot, &PlayerCommand, &OrderCatalog) -> Result<Vec<turts_sim::Command>, OrderResult>` and `pub fn to_wire(...) -> OrderResult`. `ContentRef` is `pub type ContentRef = String;` at `crates/turts_net_protocol/src/economy.rs:16`.

**2a files:** Modify `crates/turts_net_protocol/src/messages.rs` (new types; append the eight reasons `unknown_order`, `not_applicable`, `target_invalid`, `target_gone`, `out_of_range`, `rate_limited`, `not_queueable`, `blocked` to `CommandRejectReason` at `:92-122`, to `ALL` at `:124` and to `as_str` at `:155`), `crates/turts_net_protocol/tests/reasons_m3.rs` (the `ALL` list). Create `crates/turts_net_protocol/tests/wire4.rs`.

- [ ] **2a Step 1: Signature first.** Add the types and the eight reason variants. Let `as_str` return `"internal"` for the eight new variants. Write `wire4.rs`: `every_order_target_round_trips` (postcard round trip of each of the six `OrderTarget` values in an `OrderEnvelope`), `order_result_round_trips_with_refusals`, `new_reason_names_are_stable` (asserts the eight names of D section 5.1).
- [ ] **2a Step 2: Red.** `cargo test -p turts_net_protocol --test wire4 --locked; echo exit=$?`. Expected: `new_reason_names_are_stable` fails with ``assertion `left == right` failed`` and `left: "internal"`, `right: "unknown_order"`; `exit=101`.
- [ ] **2a Step 3: Green.** Give the eight names. Re-run: `exit=0`. Gate: G-RUST, G-QA. Commit. **Runs: 2.**

**2b files:** Modify `crates/turts_net_protocol/src/lib.rs:13` (4), `messages.rs` (`PlayerCommand`, `Welcome`, `Snapshot`, `WorldDelta`; remove `PlayerCommandKind`, `CommandApplied`, `CommandRejected`), `crates/turts_net_protocol/tests/wire.rs`, `git mv tests/protocol3.rs tests/protocol4.rs`; `crates/turts_net_lightyear/src/protocol_plugin.rs:36-43` (register `OrderResult`, drop `CommandApplied` and `CommandRejected`), `src/channels.rs`, `src/bin/spike_client.rs`, `src/bin/spike_server.rs`, `tests/registers.rs`; `crates/turts_server/src/orders.rs` (new), `main.rs` (`mod orders;`), `session.rs` (`admit_kind`, `:339-419`, calls `orders::to_sim`; `Welcome.match_id`), `outbound.rs:17-54` (`send_command_results` sends `OrderResult`), `export.rs` (owner-only `unit_orders`, filtered as `:87-93`; empty until task 4), `tests/hello.rs`, `tests/support/wire_sender.rs`, `tests/acceptance.rs`, `tests/economy.rs`, `tests/m3_process.rs`; `crates/turts_client/src/commands.rs`, `ingest.rs` (`OrderResult`, `match_id`), `join.rs:356-410` (`--move` and `--move-unit` send an envelope with the first `OrderDef` whose behaviour is `Move`), `input.rs:131-141` (right click sends that envelope), `economy_ui.rs` (each `UiAction` sends an envelope with the `OrderDef` of its behaviour), `debug.rs:103,175,193-200,258,284`, `options.rs:26,119` (help text), `lib.rs:295-362` (test); `crates/turts_client_web/src/lib.rs:190-210,670` (`turts_move` sends a `Move` envelope); the client tests that use `PlayerCommandKind` (`tests/commands.rs`, `tests/economy_ui.rs`, `tests/scenario.rs`).

**2b rules:**
- Before task 4, `to_sim` maps by `OrderDef.behaviour` from the package catalog (task 3): `Move` to `CommandKind::MoveTo`, `Gather` to one `CommandKind::Gather` per unit, `Produce` with `Content(recipe or technology)`, `CancelProduction` with `Job`, `Remove`, `SetEnabled(bool)`. `Stop`, `Hold`, `Attack` give `not_applicable` for each unit until tasks 4 and 5. An unknown order id gives `unknown_order` for each unit. `to_wire` copies one command outcome to every unit of the envelope (all accepted, or all refused with the one reason) until task 4.
- The server code holds no order id string (ORD1).
- `debug.rs` keeps the key `command_results`. Each entry keeps `seq`, `applied_tick` when `accepted` is not empty, `rejected_tick` and `rejected` when every unit is refused, and adds `accepted` and `refused`. So the legacy `tests/browser/t7.js` stays green until task 21 (QA0D:424-425).
- `match_id`: the server draws one `u64` at process start (a `std::collections::hash_map::RandomState` hash of the process id and the start time) and writes it in `--stats`. Plan choice; D gives no source.

- [ ] **2b Step 1: Trace (NOT TRACED item 1).** Read the lightyear 0.31.0 source for the largest message on the reliable channel that `PlayerCommand` uses (`crates/turts_net_lightyear/src/channels.rs`). Record `path:line` in the work log. An envelope of 4,096 units is at most 4,096 x 5 bytes of postcard varints plus the header. If the limit is below that, stop and report (K4 b assumes it fits).
- [ ] **2b Step 2: Red first.** Add the unit test `protocol_3_hello_is_refused` in `crates/turts_server/src/session.rs` (`mod tests`): `judge_hello` with `protocol_version: 3` returns `Err("protocol version mismatch")`. Run `cargo test -p turts_server --bin turts_server --locked session::tests::protocol_3_hello_is_refused; echo exit=$?` while the version is 3. Expected: `called `Result::unwrap_err()` on an `Ok` value`, `exit=101`. Add `move_hook_sends_the_first_move_order` in `crates/turts_client/tests/commands.rs`: the `--move 20,8` hook on `acme` queues an envelope with `order: "acme:<the Move OrderDef id of task 18>"`, `target: Point(20, 8)`, `queue: Replace`. Expected red: the hook still queues `MoveTo`.
- [ ] **2b Step 3: Green.** Make the switch. Re-run both tests: `exit=0`. Run `cargo test -p turts_net_protocol -p turts_net_lightyear -p turts_server -p turts_client --locked; echo exit=$?`: `exit=0`.
- [ ] **2b Step 4: Gate.** Commit. Then G-RUST, G-QA, G-ACC, G-WEB, G-FROZEN, G-REF(`session.rs`, `export.rs`, `ingest.rs`, `replica.rs`, `economy_ui.rs`; rust cases only, the browser cases are in the red window). **Runs: 4.**

### Task 3: Mod API 0.2.3

**Seat and model:** `.claude/agents/mod-runtime-engineer.md`, Luna. Review: `mod-builder` (Sol): can a modder fix each error from its text alone (MD18)?

**Branch / needs / wave:** `codex/mc-t3`; none (step 8 waits for task 1); wave 1. **Design:** sections 5.2, 8.2 to 8.4, 11.1 to 11.4, 3.2 (reserved keys); K2, K5 (R1), K12, K13, K18, K24. **Owns rows:** HUD3.

**Interfaces (produces):** in `turts_mod_api`: `OrderDef`, `Behaviour` (`Move | Stop | Hold | Attack | Gather | Produce | CancelProduction | Remove | SetEnabled(bool)`), `TargetKind`, `FirePolicy`, `EngagePolicy`, `OrderRulesDef` (`queue_depth: u8`, `smart`), `SmartTarget`, `Capability`, `InputMapDef`, `Binding`, `Mod`, `Button`, `AssetsDef`, `InterfaceDef`, `Widget` (the ten primitives of D section 8.2), `Placement` (`at`, `size`, `layer`, `visible`), `Source`, `Value`, `Predicate`, `CameraDef`, `LightDef`, `WorldStyle`, `AlertDef`, `AlertTrigger`, `UnitPresentation`, `GuidePage`, `RESERVED_BROWSER_KEYS`, `KNOWN_KEY_CODES`, `DEFAULT_QUEUE_DEPTH = 16`. In `turts_mod_runtime`: `PackageArchive::{orders, order_rules, input_maps, interfaces, assets}` and `ContentCatalog.orders: BTreeMap<ContentId, OrderDef>`. The shapes are those of D section 11.2, field for field.

**Files:** Modify `crates/turts_mod_api/src/schema.rs:53-65` (four lists), `crates/turts_mod_api/src/lib.rs:33,36` and re-exports; create `crates/turts_mod_api/src/orders.rs`, `input.rs`, `interface.rs`, `assets.rs`. Modify `crates/turts_mod_runtime/src/archive.rs` (accessors; `archive.rs:56` `file()` already gives the bytes of any packed file, and `pack.rs:60-87` packs every regular file, so NOT TRACED item 4 is closed), `version_table.rs` (rows, `present` signature), `content_m3_validation.rs:380-383` (remove "weapons must be static", R1); create `crates/turts_mod_runtime/src/content_mc_validation.rs`, `asset_headers.rs` (PNG IHDR width and height; TTF `00 01 00 00` or OTF `OTTO` magic; OGG Vorbis identification header rate and last-page granule). Modify `crates/turts_pack/src/main.rs` (K18 warnings on stderr, exit 0), `crates/turts_checks/tests/support/mod.rs:88-95,118` (the four lists in the export set, so `stray_ron` accepts the files; tokens for `OrderDef.id`, mod action names, asset ids; widget ids are not tokens: they are layout names, not gameplay content). Tests: create `crates/turts_mod_runtime/tests/mc_content.rs`, `crates/turts_pack/tests/fixtures/hud3/` (a one-unit Game package with orders, input, interface and assets; `make_assets.py` writes header-valid PNG, TTF and OGG files with the standard library); modify `crates/turts_pack/tests/bad_packages.rs` (HUD3 cases through its exact-stderr helper), `crates/turts_mod_runtime/tests/content_m3.rs:339` (`weapon_must_be_static` becomes `mobile_weapon_is_accepted`).

- [ ] **Step 1: Signature first.** Add the types, the lists and the accessors. The validator returns `Ok(())`. Every 0.2.2 package still loads: `cargo test --workspace --locked; echo exit=$?` gives `exit=0`.
- [ ] **Step 2: Red first (HUD3).** Write one `bad_packages.rs` case per HUD3 item of D section 14: unknown widget kind; unknown source; duplicate widget id; `card` collision; missing asset id; PNG of 2049 x 1 px; cursor of 129 x 129 px; IHDR sum above 256 MiB in a package (65 images of 2048 x 2048, header-only files); the same sum across two packages given to the locked-set check function (a real two-package locked set is refused with LR3 until MP2, `CLAUDE.md` `turts-pack` entry); missing font file; fonts above 8 MiB; a sound of 31 s; sound files above 32 MiB; a reserved default binding (Ctrl+KeyW); `CancelProduction` with `target: Point`; text `size: 10` (warning, exit 0); `engine_api: "^0.2"` with `exports.interface`. Each case asserts exit 2 and the exact stderr `<package>: <file>: <field>: <text>`, or exit 0 and the warning line. Run `cargo test -p turts_pack --test bad_packages --locked; echo exit=$?`. Expected: each new case fails (`exit=101`); the refusal cases print the stub exit 0.
- [ ] **Step 3: Cap red on file bytes.** Add the case "65 images of 2048 x 2048 with 19,771-byte files": file bytes 1.3 MB, decoded 1.0 GiB (D K12 evidence). A cap that reads file bytes accepts it. Keep this case: it is the red of "the cap reads file bytes".
- [ ] **Step 4: Green.** Implement `content_mc_validation.rs` (D section 11.3, in that order) and `asset_headers.rs`. Run step 2 again: `exit=0`.
- [ ] **Step 5: Version rows.** Add the four rows; widen `present` to the parsed content. Unit tests in `version_table.rs`: each row refuses `^0.2.2` with its list present and accepts `^0.2.3`. Red first: the tests fail before the rows exist.
- [ ] **Step 6: K5 lift (R1).** Invert `crates/turts_mod_runtime/tests/content_m3.rs:339`; remove the rule; keep "a unit with health requires removal" (`:374`) and "a unit with a queue cannot be removed" (`:424-427`). Red first: the inverted test fails while the rule exists.
- [ ] **Step 7: Token derivation.** Run `cargo test -p turts_checks --test no_content --locked; echo exit=$?`: `exit=0` with the existing packages. Add a fixture test: a package with `orders/rush.ron` (`id: "rush"`) gives a hit for a source line `let s = "rush";`.
- [ ] **Step 8: Reserved keys (waits for task 1).** `RESERVED_BROWSER_KEYS` holds the rows of D table 3.2 and every row of `docs/research/mc/p1-keys.md` with "browser acted: yes". If task 1 has not merged, stop this step and go on with the rest.
- [ ] **Step 9: Gate.** Commit. G-RUST, G-QA, G-FROZEN, G-REF(`support/mod.rs`). **Runs: 6.**

### Task 20: Harness and `io`

**Seat and model:** `.claude/agents/qa-lead.md`, Luna. Review: `web-platform-engineer` (Sol).

**Branch / needs / wave:** `codex/mc-t20`; none; wave 1. **Design:** sections 13.1, 13.2 (the `view` side), 13.4; QA0D sections 4.1 to 4.4. **Owns rows:** none. It gives `io`, `view`, `scripts/mc-browser.sh` and `scripts/mc-pixels.sh` to tasks 10 to 23.

**Interfaces (produces):** `io.click(x, y, button)`, `io.drag(x0, y0, x1, y1, button)`, `io.key(code, modifiers, opts)` (`opts.key` gives another logical key through a CDP key event that the harness builds), `io.move(x, y)`, `io.wheel(dy)`, `io.screenshot()`, `io.save(name, bytes)`, `io.wait(ms)`, `io.reload()`, `io.use(n)` (window n of `--windows N`; plan choice, PT1 and SLOT1 need two or three windows). `view.bridge(name)` (refuses `turts_move`, QA0D:333), `view.inputLog()`, `view.screenshot()`, `view.serverFile(path)`. Row file `tests/browser/rows/<ROW>.json`: `{"row": "GM40", "package": "mods/muster", "driver": "gm40-driver.js", "oracle": "gm40-oracle.js", "viewports": ["1280x720"], "windows": 1, "runs": "ci"}` (`runs`: `ci` or `headed`).

**Files:** Create `tests/browser/harness/run.js`, `tests/browser/harness/driver-io.js`, `tests/browser/harness/view.js` (role `harness`); `scripts/mc-browser.sh` (packs the package copy, starts `turts_server` and a static server as `scripts/t7.sh:22-82` does, runs `run.js`); `scripts/mc-pixels.sh` (runs each row file twice; a row passes only when both runs pass, QA0D:441-443); `tests/qa/driver/harness-ok-driver.js`, `harness-ok-oracle.js`, `harness-fail-oracle.js`; modify `scripts/qa/driver_rule.py:12` (add `harness` to `ROLES`) and its self-test cases.

- [ ] **Step 1: Red first (role).** Give `run.js` the line `// turts-test-role: harness`. Run `python3 scripts/qa/driver_rule.py tests/browser; echo exit=$?`. Expected: a `RED DR1` line for `tests/browser/harness/run.js` (unknown role), `exit=1`.
- [ ] **Step 2: Green.** Add `harness` to `ROLES`; add a self-test case for a `harness` file. Re-run both driver-rule commands: `exit=0`.
- [ ] **Step 3: Red (oracle fail).** Run `bash scripts/mc-browser.sh mods/acme tests/qa/driver/harness-ok-driver.js tests/qa/driver/harness-fail-oracle.js 1280x720; echo exit=$?`. Expected: `FAIL harness-fail-oracle.js: <reason>`, `exit=1`. The pass case with `harness-ok-oracle.js` prints `PASS` and `exit=0`.
- [ ] **Step 4: Input log.** `harness-ok-oracle.js` asserts that `view.inputLog()` holds each `io` call of the driver with its arguments and wall time, and that `view.bridge("turts_move")` throws.
- [ ] **Step 5: Gate.** Commit. G-QA, G-WEB. **Runs: 3.**

## Wave 2

### Task 8: `turts_interaction` skeleton, contract types, T1 rows, T1X

**Seat and model:** `.claude/agents/technical-director.md`, Luna. Review: `qa-lead` (Sol): can each T1X case go red?

**Branch / needs / wave:** `codex/mc-t8`; needs 2a, 3; wave 2. **Design:** sections 2.1, 2.2, 2.4, 13.6; K1, K26. **Owns rows:** T1X.

**Interfaces (produces, `turts_interaction`):** `Core::step(&mut self, events: &[InputEvent], world: &dyn WorldView, view: &Viewport, now_ms: u64) -> Vec<Effect>`; `InputEvent`, `WorldView`, `Viewport`, `Effect`, `CameraState` (look-at point, height, pitch, yaw, vertical field of view), exactly as D section 2.2. The stub `step` returns `Vec::new()`.

**Files:** Create `crates/turts_interaction/Cargo.toml` (`turts_mod_api`, `turts_net_protocol`, `glam = "=0.33.12"`), `src/lib.rs`, `src/input_map.rs`, `src/selection.rs`, `src/groups.rs`, `src/smart.rs`, `src/compose.rs`, `src/camera.rs`, `src/projection.rs`. Modify `Cargo.lock`, `scripts/boundary.sh` (after line 37), `crates/turts_checks/tests/boundary.rs:92-95` (four `checked:` lines), and add the T1X tests to `crates/turts_checks/tests/boundary.rs`.

- [ ] **Step 1:** Add the crate. `cargo tree -p turts_interaction -e normal --prefix none --locked > <scratch>/tree.txt; echo exit=$?` prints `exit=0`; then `rtk proxy grep -c "^glam v0.33.12" <scratch>/tree.txt` prints `1` and `rtk proxy grep -c "^glam v0.34" <scratch>/tree.txt` prints `0`.
- [ ] **Step 2: Red first (T1X copy).** Write `t1x_copy_with_bevy_render_is_red`: it copies `crates/turts_interaction` to a temporary directory, writes its own `[workspace]` table, explicit package fields, absolute path dependencies and `bevy_render = "=0.20.0"`, then runs `bash scripts/boundary.sh --rule no-bevy-prefix --crate turts_interaction --manifest <copy>/Cargo.toml`. It asserts exit 1 and the exact stderr line `forbidden edge: turts_interaction -> bevy_app` (D section 13.6). Before the rows exist the full run does not print the new lines, so also write `t1x_full_run_prints_four_lines`. Run `cargo test -p turts_checks --test boundary --locked; echo exit=$?`. Expected: `t1x_full_run_prints_four_lines` fails with `missing "checked: turts_interaction x86_64-unknown-linux-gnu: no bevy*"`, `exit=101`.
- [ ] **Step 3: Green.** Add the four rows of D section 2.4. Add `t1x_real_crate_wasm32_is_red_for_mod_api` (`--rule forbid --crate turts_interaction --target wasm32-unknown-unknown --names turts_mod_api`, expects `forbidden edge: turts_interaction -> turts_mod_api`), as `boundary.rs:105-122` does. Re-run: `exit=0`. `bash scripts/boundary.sh` prints the four lines `checked: turts_interaction <host>: no bevy*`, `checked: turts_interaction wasm32-unknown-unknown: no bevy*`, `checked: turts_interaction <host>: no turts_sim wgpu winit`, `checked: turts_interaction wasm32-unknown-unknown: no turts_sim wgpu winit`.
- [ ] **Step 4: Plant.** After the commit, delete one new row from `scripts/boundary.sh`; `t1x_full_run_prints_four_lines` fails; restore. Gate: G-RUST, G-QA. **Runs: 2.**

### Task 18: Orders for the five packages (K3 b, R4)

**Seat and model:** `.claude/agents/mod-builder.md`, Luna. Review: `mod-runtime-engineer` (Sol).

**Branch / needs / wave:** `codex/mc-t18`; needs 3; wave 2. **Design:** sections 11.4, 12.1 (orders part), 12.2; K3 b, K13. **Owns rows:** MOD1.

**Files:** For each of `mods/acme`, `mods/base`, `mods/cedar`, `mods/haul`: `manifest.ron` (`engine_api: "^0.2.3"`, version `0.2.0` to `0.3.0`), `orders/*.ron` (the orders its units can use), `orders/rules.ron`, `input/bindings.ron`, `interface/hud.ron` (plain text buttons, no images; D section 12.2). For `mods/forge`: `manifest.ron` (`engine_api: "^0.2.3"`, `0.1.0` to `0.2.0`), `orders/*.ron` (move, stop, hold, research, cancel, enable, disable, remove; D section 12.1), `orders/rules.ron`, `input/bindings.ron` (M, S, H, D, E, R and the production keys, D section 3.3). Maps, units, recipes and technologies do not change. Create `crates/turts_pack/tests/mc_packages.rs`.

**Content ids:** choose local ids that the no-content scan does not find in engine code. Run `cargo test -p turts_checks --test no_content --locked real_source_scan_is_clean; echo exit=$?` after each package. On a hit, rename the local id in the package (for example `forge:order_research`), never the engine code. Plan run: the literal `"research"` occurs at `crates/turts_mod_runtime/src/content_m3_validation.rs:407` and `crates/turts_client/src/debug.rs:98`, so a local id `research` is red.

- [ ] **Step 1: Red first.** Write `mc_packages.rs`: `five_packages_load_with_0_2_3` (each loads with `^0.2.3` and has at least one `OrderDef` with behaviour `Move` when a unit has movement), `old_acme_copy_loads_with_no_orders` (a 0.2.2 copy of `acme` with no orders loads; the client content reports "This game defines no orders"), `each_version_row_refuses_0_2_2` (four copies, one list each, `^0.2.2`, exit 2 with the row name), `a_new_order_binds_a_behaviour_by_data` (a copy with `orders/dash.ron` bound to `Move` loads). Run `cargo test -p turts_pack --test mc_packages --locked; echo exit=$?`. Expected: `five_packages_load_with_0_2_3` fails (no orders yet), `exit=101`.
- [ ] **Step 2: Green.** Write the files. Re-run: `exit=0`. `cargo run -q -p turts_pack -- <scratch copy of each package>; echo exit=$?` prints the archive and hash, `exit=0`.
- [ ] **Step 3: Plant (MOD1 red).** After the commit, delete the `exports.input` row from `version_table.rs`; `each_version_row_refuses_0_2_2` fails; restore.
- [ ] **Step 4:** Gate: G-RUST, G-QA, G-FROZEN (the stream test loads `base`, `haul`, `acme`, `cedar`, `m3_m2_stream.rs:16`; the hash must not change), G-ACC. **Runs: 2.**

### Task 19: `muster` and the HUD1 fixture

**Seat and model:** `.claude/agents/mod-builder.md`, Luna. Review: `ux-designer` (Sol).

**Branch / needs / wave:** `codex/mc-t19`; needs 3 (R1); wave 2. **Design:** sections 7 (camera start), 8.5, 12.3; K15, K24. **Owns rows:** none (content for SEL2, SEL3, GM40, GMA, GMU, ORD1 to ORD7, MM1, VIS1, VIS2, HUD1, HUD4, DET1, EL1).

**Names (ruling O1 a, D revision 6, section 12.3):** the slot 1 unit is `dummy` and the renamed `Attack` order is `rush`. The old names collided with the scan: `target` is an engine identifier (for example `crates/turts_sim/src/command.rs:239`, `MoveTarget(target)`) and a literal at `crates/turts_mod_runtime/src/visuals.rs:31`; `"charge"` is a literal at `crates/turts_client/src/scenario.rs:134` and `crates/turts_checks/examples/m2_variants.rs:121`. Step 2 runs the scan as the proof that the new names pass. The scan checks every identifier and every whole string literal under `crates/` except `tests` directories (`crates/turts_checks/tests/support/mod.rs:451-540`), and an id gives its qualified and its local token (`support/mod.rs:403-410`).

**Files:** Create `mods/muster/` (`manifest.ron`, `maps/field.ron` 64 x 48 with one ore node, `units/*.ron`, `visuals/*.ron`, `resources/ore.ron`, `nodes/*.ron`, `orders/*.ron` (the forge set plus attack, gather and `rush`, which binds `Attack`, D section 5.2), `orders/rules.ron`, `input/bindings.ron`, `interface/hud.ron`, `assets/assets.ron` and the asset files). `--synthetic-units` adds units of the first registered unit type with movement (`crates/turts_server/src/synthetic.rs:39-50`). The registration order is NOT TRACED, so `manifest.ron` lists `units/marker.ron` first and the test `marker_is_the_first_unit_type` checks that the synthetic units are markers (VIS2). Create `crates/turts_pack/tests/fixtures/muster_hud1/` (a copy of `muster` with the tree of D section 8.5) and `crates/turts_pack/tests/muster.rs`.

**Placement facts:** slot 0: 40 troopers (radius 0.5, speed 3.0, weapon range 4, damage 5, cooldown 1 s, health 50, removal), 2 workers (gatherer), 1 depot, 1 post (static, radius 2), 460 markers in a far block. One screen box holds the 460 markers and the 40 troopers and no other unit (SEL2: 500 rings). Slot 1: 10 `dummy` units (mobile, health 40, no weapon), 1 post. Slot 0 has an ore balance. Total 515 units. `CameraDef.start: Point` on the troopers.

- [ ] **Step 1: Red first.** `muster.rs`: `muster_loads_with_515_units`, `fixture_hud1_loads`, `marker_is_the_first_unit_type`. Run `cargo test -p turts_pack --test muster --locked; echo exit=$?`: fails (no package), `exit=101`.
- [ ] **Step 2: Green.** Write the package and the fixture. Re-run: `exit=0`. `cargo test -p turts_checks --test no_content --locked; echo exit=$?`: `exit=0` (the proof for `dummy` and `rush`; a hit here stops the task).
- [ ] **Step 3:** Gate: G-RUST, G-QA. Commit. **Runs: 3.**

## Wave 3

Task 2b (in the task 2 section above).

## Wave 4

### Task 4: `SimSet::Orders`, queue, behaviours, audited writes, validator (K19 a)

**Seat and model:** `.claude/agents/simulation-core-engineer.md`, Luna. Review: `qa-lead` (Sol): which build makes each test red?

**Branch / needs / wave:** `codex/mc-t4`; needs 2b, 3; wave 4. **Design:** sections 4.5, 5.2 (server check order), 5.3, 6.1; K4 b, K19 a, K24. **Owns rows:** none (ORD2, ORD3, EL1 are later; this task builds their unit tests).

**Interfaces (produces):** `SimSet::Orders`, first in the `FixedUpdate` chain (`crates/turts_sim/src/plugin.rs:88-101`). Component `UnitOrders { current: Option<Order>, queue: VecDeque<Order> }` on each unit with `movement` or `weapon`. `CommandKind::Order(OrderCommand { order: ContentId, behaviour: Behaviour, units: Vec<u32>, target: OrderTargetSim, queue: QueueMode, group: u32 })`. `CommandResult` gains `units: UnitOutcomes { accepted: Vec<u32>, refused: Vec<(u32, RejectReason)> }`; the sim `RejectReason` (`crates/turts_sim/src/command.rs:79-112`) gets the eight new reasons. Pure function `turts_sim::movement::step(&MovementInput, dt: f32) -> MovementOutput`, called by `integrate_movement` and by the validator. `MAX_UNITS_PER_COMMAND` 256 to 4,096 (`command.rs:19`).

**Files:** Modify `crates/turts_sim/src/plugin.rs:31-42,88-101`, `command.rs` (per-unit validation in the order of D section 5.2; `apply_move_to` `:212-253` stays the plain-move path), `movement.rs:12-58` (calls `step`; same float step for a unit with no neighbour, `:44-48`), `economy/views.rs:300` (`AuditState` fields with `#[serde(default, skip_serializing_if = ...)]`), `economy/gather.rs` (gather order and stop); create `crates/turts_sim/src/orders.rs`; modify `crates/turts_server/src/orders.rs` (Stop, Hold; per-unit `to_wire`), `export.rs` (fill `unit_orders`), `economylog.rs:578-663` (`validate_movement_transition` calls `step`; keep `movement_pending`, `:326,553-556`). Tests: create `crates/turts_sim/tests/orders.rs`; invert `crates/turts_sim/tests/command.rs:89,116,233,322`, `crates/turts_pack/tests/economy_ordering.rs:120-175` (`NotMobile` at `:153`) and `:217-260` (`:247`), and the group-move cases in `crates/turts_server/tests/economy.rs:2089-2236` (inside `e8_commands_are_authoritative_and_acked`, `:1875`, run by `scripts/acceptance.sh:106`).

**Plain orders write no operation (D section 6.1):** a plain move is one `Move` for one unit, no queue, no group above one unit. It writes only today's command operation (`apply_command` calls `capture_command`, `command.rs:155-159`). A gather order writes no order operation. Every other order write runs inside `capture_operation` (`crates/turts_sim/src/economy/transaction.rs:90`), one unit per operation.

- [ ] **Step 1: Red first.** `orders.rs` tests: `per_unit_results_for_a_mixed_envelope` (300 units: 290 own mobile accepted, 5 own static `not_applicable`, 5 foreign `not_owner`), `shift_appends_and_plain_replaces`, `stop_clears_current_and_queue`, `queue_beyond_depth_is_queue_full`, `append_behind_hold_or_gather_is_not_queueable`, `queued_order_starts_in_the_completion_tick`, `depleted_gather_writes_last_failure`, `plain_move_writes_no_order_operation`. Run `cargo test -p turts_sim --test orders --locked; echo exit=$?`. Expected: `per_unit_results_for_a_mixed_envelope` fails with `left: 0` accepted (today's all-or-nothing `validate_move_to`, `command.rs:255-294`), `exit=101`.
- [ ] **Step 2: Green.** Implement. Invert the listed tests. Re-run: `exit=0`.
- [ ] **Step 3: Frozen.** G-FROZEN: `exit=0` and the hash unchanged. If `m2_operation_stream_matches_baseline` fails, stop and report (K19).
- [ ] **Step 4: Validator.** Add the unit test `validator_accepts_plain_and_queued_moves` in `crates/turts_server/src/economylog.rs` (`mod tests`): a 400-tick log of `acme` with one plain move and one queued move of two waypoints. Red first: it fails on today's `validate_movement_transition` with `unit <id> snapshots disagree after movement` (`economylog.rs:660`). Green after the switch to `step`: `exit=0`.
- [ ] **Step 5: Gate.** Commit. G-RUST, G-QA, G-ACC, G-WEB, G-REF(`command.rs`, `plugin.rs`, `gather.rs`, `transaction.rs`, `export.rs`). **Runs: 8.**

### Task 7: Server session: rate buckets (K25), rejoin (K27), `--command-script`

**Seat and model:** `.claude/agents/backend-engineer.md`, Luna. Review: `network-engineer` (Sol).

**Branch / needs / wave:** `codex/mc-t7`; needs 2b; wave 4. **Design:** sections 5.4, 10.5 (server side), 14 DET1 (the hook); K25, K27. **Owns rows:** none (SLOT1 is task 16, ORD6 task 15, DET1 task 22).

**Files:** Modify `crates/turts_server/src/session.rs` (buckets before the command queue; `judge_hello` `:92-106`: when `claim` gives `AlreadyClaimed` and no live session holds that slot, give the same slot back with a new `Welcome` and a snapshot; while the old session lives, refuse with `This game is open in another window`), `config.rs:146-150` (`--command-script <path>`), `main.rs` (load the script: a JSON list of `{"tick": u32, "slot": u8, "envelope": OrderEnvelope}`, applied at fixed ticks with no client), `crates/turts_server/tests/help.rs` (the help text). `Slots::claim` (`crates/turts_sim/src/slots.rs:46-57`) does not change.

- [ ] **Step 1: Red first.** Unit tests in `session.rs`: `bucket_refuses_the_65th_envelope_in_one_burst` (`rate_limited` for each unit), `bucket_refills_after_one_second` (accept again after 1,000 ms of refill), `unit_bucket_refuses_above_16384`, `reload_reclaims_a_held_slot_with_no_live_session`, `reload_while_the_old_session_lives_is_refused`. Expected red: `reload_reclaims_a_held_slot_with_no_live_session` fails with `Err("player already in match")` (`session.rs:32,103-105`); `exit=101`.
- [ ] **Step 2: Green.** Implement. Re-run: `exit=0`. Add `crates/turts_server/tests/command_script.rs` (ignored process test; add it to `scripts/acceptance.sh`): a script that moves one `acme` unit at tick 10 gives a tick log in which the unit moves from tick 10.
- [ ] **Step 3: Gate.** Commit. G-RUST, G-QA, G-ACC, G-REF(`session.rs`). **Runs: 4.**

### Task 9: Core logic, Bevy adapter, PROJ1, KM3

**Seat and model:** `.claude/agents/bevy-rendering-engineer.md`, Luna; `ux-designer` writes the headless core tests. Review: `technical-director` (Sol).

**Branch / needs / wave:** `codex/mc-t9`; needs 1, 8, 2b; wave 4. **Design:** sections 2.2, 2.3, 3.1, 3.3, 4.1 to 4.4, 7 (state only); K11, K16. **Owns rows:** KM3, PROJ1.

**Files:** Modify `crates/turts_interaction/src/*.rs` (logic). Modify `crates/turts_client/Cargo.toml` (`turts_interaction`), `lib.rs` (the module stubs of R5; the core plugin in both headless and windowed apps, `lib.rs:262-283`), `input.rs` (Bevy input to `InputEvent`, including `CursorLeft` to `PointerLeft`, `bevy_winit-0.20.0/src/state.rs:323-325`), `render.rs:17,540` (import `Selection` and `UiPanelRoot` from `ui_shared.rs`), `debug.rs` (keys `selection`, `groups`, `results`, `notices`, `alerts`, `audio`, `contextmenu`, `focus`, `cursor`, `player_id` with empty values, R5), `economy_ui.rs` (import the moved types), `live_view.rs:22,404`; create `crates/turts_client/src/ui_shared.rs`, `core_adapter.rs` (the `WorldView` from the replica; copies `CameraState` into the Bevy `Transform` and the vertical field of view into `Projection`), and the stub files of R5. Tests: create `crates/turts_interaction/tests/km3.rs`, `selection.rs`, `groups.rs`, `smart.rs`, `crates/turts_client/tests/proj1.rs`.

- [ ] **Step 1: Red first (KM3).** `km3.rs`: `shift_click_toggles`, `shift_drag_adds`, `ctrl_click_selects_type_on_screen`, `two_clicks_within_400ms_and_4px_double`, `two_clicks_500ms_apart_are_two_singles`. Run `cargo test -p turts_interaction --test km3 --locked; echo exit=$?`. Expected: each fails on the stub `step` (`left: []`), `exit=101`.
- [ ] **Step 2: PROJ1 trace and red.** `proj1.rs`: 20 world points, viewports 1280 x 720, 1920 x 1080, and 1280 x 720 at scale 2; the core projection and `Camera::world_to_viewport` differ by at most 1 px, in a Bevy app with an `Image` render target and no window. First try the app with no render plugin (the camera size comes from the image asset). If `camera_system` needs a GPU adapter, mark the test `#[ignore = "PROJ1 needs a wgpu adapter"]`, run it locally with `cargo test -p turts_client --test proj1 --locked -- --ignored; echo exit=$?`, and task 24 adds it to the `browser` job (mesa Vulkan, `ci.yml:100`). Record which case holds (D section 14 marks this NOT TRACED).
- [ ] **Step 3: Defaults (waits for task 1).** The engine default bindings are D table 3.3 when `p1-keys.md` says `K11 (a) holds: yes`. Else use the CEO pick.
- [ ] **Step 4: Green.** Implement the core (D sections 3.1, 4.1 to 4.4: the dense content-id index of D section 4.1; no selection cap; foreign units inspect only, K16). Re-run steps 1 and 2: `exit=0`.
- [ ] **Step 5: Plant (PROJ1).** After the commit, stop the copy of the field of view into `Projection` in `core_adapter.rs`; `proj1.rs` fails at 1920 x 1080; restore.
- [ ] **Step 6: Gate.** G-RUST, G-QA, G-WEB (T7 still selects with a left click and moves with a right click), G-REF(`render.rs`; the browser case only records). **Runs: 6.**

## Wave 5

### Task 6: Group movement: spreading, avoidance, arrival, fallback

**Seat and model:** `.claude/agents/simulation-core-engineer.md`, Luna. Review: `performance-architect` (Sol): cost and determinism.

**Branch / needs / wave:** `codex/mc-t6`; needs 4; wave 5. **Design:** sections 6.3 to 6.7; K4 merge rule, K6, K17. **Owns rows:** none (GM40, GMA, GMU, DET1, EL1 are task 22).

**Files:** Modify `crates/turts_sim/src/movement.rs` (`step` grows: separation, same-group yield, hard projection, arrival), `orders.rs` (merge key (slot, `group`, tick); pending lists; round robin; cap 2,048 per tick; head envelope in full; `Pending`), `command.rs:284-292` (non-finite: `target_not_finite`; finite outside: clamp inset by the unit radius), `economy/views.rs` (pool, spares, stall timer, best distance, claim count, pass mode in `AuditState`), `crates/turts_server/src/economylog.rs` (inputs of `step`); create `crates/turts_sim/src/grid.rs` (uniform grid, cell 2 x r_max, iteration in unit id order). Tests: create `crates/turts_sim/tests/group_movement.rs`; invert `crates/turts_sim/tests/command.rs:243,268`.

- [ ] **Step 1: Red first.** `group_movement.rs`: `forty_units_rest_with_no_overlap_and_no_blocked` (model case A: farthest at most 4.58), `ring_to_point` (B), `second_group_same_point` (C2), `new_order_while_moving` (D), `cases_f_and_i_rest_with_no_overlap`, `one_unit_alone_takes_the_point`, `outside_target_is_clamped`, `nan_target_is_refused`, `post_target_gives_a_ring`, `pending_above_2048_per_tick`. Run `cargo test -p turts_sim --test group_movement --locked; echo exit=$?`. Expected: `forty_units_rest_with_no_overlap_and_no_blocked` fails with an overlap count above 0 (straight line, `movement.rs:12-58`; model: 780), `exit=101`.
- [ ] **Step 2: Green.** Implement. Re-run: `exit=0`. G-FROZEN: unchanged (a unit with no neighbour keeps the float step of `movement.rs:44-48`).
- [ ] **Step 3: Gate.** Commit. G-RUST, G-QA, G-ACC, G-WEB (T7 puts the unit within 0.011 of the ground point), G-REF(`command.rs`). **Runs: 6.**

### Task 10: Headless `--actions` hook

**Seat and model:** `.claude/agents/bevy-rendering-engineer.md`, Luna. Review: `qa-lead` (Sol).

**Branch / needs / wave:** `codex/mc-t10`; needs 9, 19, 20; wave 5. **Design:** sections 2.3, 13.5. **Owns rows:** HD1.

**Files:** Modify `crates/turts_client/src/options.rs` (`--actions <path>`; help text), `crates/turts_client/tests/options.rs`, `crates/turts_client/tests/help.rs`; fill `crates/turts_client/src/actions.rs` (each action to `InputEvent`s on a fixed 1280 x 720 virtual viewport). Create `crates/turts_client/tests/hd1.rs` (process test, `#[ignore]`, added to `scripts/acceptance.sh`), `tests/browser/hd1-driver.js` (role `driver`), `tests/browser/hd1-oracle.js` (role `oracle`), `tests/browser/rows/HD1.json`, `tests/browser/fixtures/hd1-actions.json` (one action per behaviour on `muster`, and `group_assign`, `group_recall`).

- [ ] **Step 1: Red first.** `hd1.rs` runs `turts_client --headless --actions tests/browser/fixtures/hd1-actions.json --out <p>` against a `muster` server and asserts the per-unit results in `--out`. Run `cargo test -p turts_client --test hd1 --locked -- --ignored; echo exit=$?`. Expected: `unknown option --actions`, the test fails, `exit=101`.
- [ ] **Step 2: Green.** Implement. Re-run: `exit=0`. Then `bash scripts/mc-pixels.sh tests/browser/rows/HD1.json; echo exit=$?`: the oracle compares the browser accepted and refused sets with the headless `--out`; `exit=0`.
- [ ] **Step 3:** Gate: G-RUST, G-QA, G-ACC. Commit. **Runs: 2.**

### Task 11: Camera controller and `LightDef` (K23)

**Seat and model:** `.claude/agents/bevy-rendering-engineer.md`, Luna. Review: `ux-designer` (Sol).

**Branch / needs / wave:** `codex/mc-t11`; needs 9, 19, 20; wave 5. **Design:** section 7; K9, K23; D68. **Owns rows:** CAM1.

**Files:** Modify `crates/turts_client/src/render.rs` (remove `fit_scene_camera` `:245` and its `PostUpdate` system `:454-458`, the fixed start `:336-342`, the fixed light `:343-346`, the `UiPanelRoot` import `:17`); fill `crates/turts_client/src/camera.rs` (pan by keys, edge band 8 px, middle drag; speed scales with height; zoom between mod limits; bounds plus margin; Space centres; `OwnedCentre`, `MapCentre`, `Point(x, y)`; `LightDef` or today's light by default). Create `tests/browser/cam1-driver.js`, `tests/browser/cam1-oracle.js`, `tests/browser/rows/CAM1.json`, `crates/turts_interaction/tests/camera.rs`.

- [ ] **Step 1: Red first.** `cam1-driver.js` holds ArrowRight 1 s, moves the pointer into the right edge band 1 s, wheels in and out past the limits, presses Space with a selection, and moves the pointer out of the top edge. `cam1-oracle.js` reads `clip_from_world` from `turts_scene` before and after each step. Run `bash scripts/mc-pixels.sh tests/browser/rows/CAM1.json; echo exit=$?`. Expected: `FAIL cam1-oracle.js: camera did not move` (the fit rewrites the camera each frame, `render.rs:455-456`), `exit=1`.
- [ ] **Step 2: Green.** Implement. Re-run: `exit=0`. The clause "the scroll speed setting changes the pan distance per second" is tested here headless in `camera.rs` (speed 2x gives 2x distance). The menu path is SH1 (task 16).
- [ ] **Step 3:** Gate: G-RUST, G-QA, G-WEB, G-REF(`render.rs`). Commit. **Runs: 4.**

### Task 12: Widget runtime, cursor, removal of the engine panel

**Seat and model:** `.claude/agents/bevy-rendering-engineer.md` with `ux-designer`, Luna. Review: `mod-runtime-engineer` (Sol): does the runtime read only table 8.4?

**Branch / needs / wave:** `codex/mc-t12`; needs 3, 9, 19, 20; wave 5. **Design:** sections 8.1 to 8.6, 10.3 (canvas size, R7); D67. **Owns rows:** HUD1, HUD4.

**Files:** Fill `crates/turts_client/src/widgets/` (`mod.rs`, `placement.rs`, `primitives.rs`, `sources.rs`, `tooltip.rs`, `cursor.rs`, `snapshot.rs` for the `turts_ui` bridge). Images load with `RENDER_WORLD` usage only; cursor images keep the CPU copy and load as their own asset (D section 8.1). `UiScale` takes the player scale only (`bevy_ui-0.20.0/src/update.rs:57-62`). Remove `crates/turts_client/src/economy_ui.rs` and `crates/turts_client/tests/economy_ui.rs`; list each removed test and the MC test that covers its behaviour in the work log. Modify `lib.rs:7,37,279`, `debug.rs:113`, `live_view.rs`, `scenario.rs`, `crates/turts_client_web/src/lib.rs:35` (`turts_ui` from `widgets::snapshot`; `turts_scene.widgets`: id and rect of each rendered widget), `crates/turts_client_web/index.html:8` (canvas 100 % of the viewport, R7). Create `crates/turts_client/tests/widgets.rs`, `tests/browser/hud-driver.js`, `hud-oracle.js`, `hud4-driver.js`, `hud4-oracle.js`, `tests/browser/rows/HUD1.json` (three viewports), `rows/HUD4.json`. Remove `tests/browser/m3-mobile.js` and its row in `tests/browser/legacy-bridge.tsv` (R6).

- [ ] **Step 1: Red first (HUD1).** Run the HUD1 row on the fixture `crates/turts_pack/tests/fixtures/muster_hud1` before the runtime: `bash scripts/mc-pixels.sh tests/browser/rows/HUD1.json; echo exit=$?`. Expected: `FAIL hud-oracle.js: widget card missing` and `engine panel present` (the panel of `economy_ui.rs:916-934`), `exit=1`. `widgets.rs` `placement_resolves_each_anchor` fails on the stub.
- [ ] **Step 2: Green.** Implement. Re-run: card top left, minimap bottom right, text bottom centre, each within 1 px at 1280 x 720, 1920 x 1080 and 1280 x 720 at scale 2; no engine panel; `exit=0`. The grep check: `rtk proxy grep -rnE "px\(|Val::Px" crates/turts_client/src/widgets --include=*.rs` finds hits only in `placement.rs`.
- [ ] **Step 3: HUD4.** `hud4-driver.js` points at an own unit, a foreign unit, a node, an invalid target and a point in target mode. `hud4-oracle.js` reads `turts_debug.cursor` (context, mod image id, applied `CursorIcon`). Plant after the commit: return the `default` context for every hover; the oracle fails; restore.
- [ ] **Step 4: Gate.** G-RUST, G-QA, G-WEB, G-REF(`economy_ui.rs`: the three browser cases now enter the red window; record it). The ledger has 4 rows; `python3 scripts/qa/driver_rule.py tests/browser; echo exit=$?` gives `exit=0`. Commit. **Runs: 8.**

## Wave 6

### Task 5: Mobile weapons, attack, attack-move, grid target choice (K5); ORD1

**Seat and model:** `.claude/agents/simulation-core-engineer.md`, Luna. Review: `qa-lead` (Sol).

**Branch / needs / wave:** `codex/mc-t5`; needs 4, 6, 19 (R2); wave 6. **Design:** sections 5.2, 5.3 (attack rows), 6.2; K2, K5. **Owns rows:** ORD1.

**Files:** Modify `crates/turts_sim/src/economy/combat.rs:165-200` (`choose_target` gets the ordered target; the `fire` and `engage` policy decide the rest; a unit with no order keeps the M3 rule; candidates from `grid.rs`, not `combat.rs:89-199`), `crates/turts_sim/src/orders.rs` (`Attack` on a unit and on a point; `Hold` fires in range), `crates/turts_server/src/orders.rs` (`Attack`). Create `crates/turts_pack/tests/mc_orders.rs`.

- [ ] **Step 1: Red first (ORD1).** `mc_orders.rs`: `every_behaviour_has_an_envelope` (one envelope per `Behaviour`, nine in all, through `orders::to_sim`), `every_target_round_trips` (the six `OrderTarget`), `rush_order_attacks` (`muster:rush` binds `Attack`, D section 5.2), `version_3_hello_is_refused`. Run `cargo test -p turts_pack --test mc_orders --locked; echo exit=$?`. Expected: `rush_order_attacks` fails (`not_applicable`, task 2b rule), `exit=101`.
- [ ] **Step 2: Green.** Implement. Re-run: `exit=0`. `cargo test -p turts_checks --test no_content --locked; echo exit=$?`: `exit=0` ("rush" is not in engine code).
- [ ] **Step 3: Plants.** After the commit: (a) add `if order == "muster:rush"` in `crates/turts_server/src/orders.rs`; `real_source_scan_is_clean` fails; restore. (b) drop the `Job` arm of `to_sim`; `every_behaviour_has_an_envelope` fails; restore.
- [ ] **Step 4: Frozen.** G-FROZEN: unchanged (no forge unit gets an `Attack` order; the turret keeps the M3 rule, D section 6.2).
- [ ] **Step 5: Gate.** G-RUST, G-QA, G-ACC, G-REF(`combat.rs`: C13, C14, C16; re-anchor C16 if its anchor `(distance_squared <= range_squared).then_some((distance_squared, id))` moved). **Runs: 4.**

### Task 13: World visuals; SEL2, SEL3, VIS1, VIS2

**Seat and model:** `.claude/agents/bevy-rendering-engineer.md`, Luna. Review: `ux-designer` (Sol).

**Branch / needs / wave:** `codex/mc-t13`; needs 9, 11, 19, 20; wave 6. **Design:** sections 4.1, 9.1; D69 (presentation). **Owns rows:** SEL2, SEL3, VIS1, VIS2.

**Files:** Fill `crates/turts_client/src/visuals.rs` (rings: solid for own, four gaps for foreign; hover ring; range circles; health bars with the three modes; markers until rest, limit 60 s; waypoint lines from `UnitOrderRecord`; `Blocked` mark 2 s; one shared ring mesh and one bar mesh; one unlit material per slot colour; bar fill as a child with x scale; transform writes only on change). Modify `crates/turts_client/src/render.rs:383,390,403-406` (one material per slot; writes on change). Create `crates/turts_client/tests/palette.rs` (CIEDE2000 under Machado 2009 matrices), `tests/browser/sel-driver.js`, `sel-oracle.js`, `vis-driver.js`, `vis-oracle.js`, `rows/SEL2.json`, `rows/SEL3.json`, `rows/VIS1.json`, `rows/VIS2.json` (`"runs": "headed"`), `docs/measurements/mc.md` (section "VIS2").

- [ ] **Step 1: Red first (VIS1).** `vis-oracle.js` finds two slot colours in the screenshot. Run `bash scripts/mc-pixels.sh tests/browser/rows/VIS1.json; echo exit=$?`. Expected: `FAIL vis-oracle.js: one colour for all units` (`render.rs:403-406`), `exit=1`. `palette.rs` fails on the stub palette.
- [ ] **Step 2: SEL2 and SEL3.** `sel-driver.js` box-selects the 500 units; drags a box with only `post` and `depot` (case `static_only`: the static units are selected); drags a box over own and slot 1 units (case `mixed_owner`: no foreign unit); Shift+drag adds; Shift+click removes; Ctrl+click and a double click on a trooper. `sel-oracle.js` checks the rings in the first presented frame after the mouse-up (frame log) and `turts_debug.selection`.
- [ ] **Step 3: Green.** Implement. Re-run the rows SEL2, SEL3, VIS1: `exit=0`. `cargo test -p turts_client --test palette --locked; echo exit=$?`: `exit=0`.
- [ ] **Step 4: Plants.** After the commit: box test in world space (SEL2 fails); selection applied one frame later (SEL2 fails); type select with no screen test (SEL3 fails); one palette pair set to `#0072B2` and `#0073B3` (`palette.rs` fails). Restore each.
- [ ] **Step 5: VIS2 (headed, local).** Start the `muster` server with `--synthetic-units 4485` (515 + 4,485 = 5,000). Run `node tests/browser/measure.js <page url> vis2 <Chrome WebGPU flags>` in `tests/browser`. Write p50 and p99 frame time, draw calls and memory, the machine (`lscpu` model name), the Chrome version and the adapter in `docs/measurements/mc.md`. The row check fails while a number is missing. GCP is pending (D59); record the local machine.
- [ ] **Step 6: Gate.** G-RUST, G-QA, G-WEB, G-REF(`render.rs`). **Runs: 5.**

## Wave 7

### Task 14: Minimap view; MM1, ORD3, ORD4, ORD5

**Seat and model:** `.claude/agents/bevy-rendering-engineer.md`, Luna. Review: `simulation-core-engineer` (Sol) for the order rows.

**Branch / needs / wave:** `codex/mc-t14`; needs 5, 12, 13; wave 7. **Design:** sections 4.3 to 4.5, 7 (minimap click), 8.2 (`MinimapView`), 9.1 (waypoints); K5. **Owns rows:** MM1, ORD3, ORD4, ORD5.

**Files:** Fill `crates/turts_client/src/minimap.rs`; modify `crates/turts_client/src/render.rs` (minimap camera or texture only if needed). Create `tests/browser/mm1-driver.js`, `mm1-oracle.js`, `orders-driver.js`, `orders-oracle.js`, `rows/MM1.json`, `rows/ORD3.json`, `rows/ORD4.json`, `rows/ORD5.json`; `crates/turts_pack/tests/mc_order_rows.rs` (the headless halves on `muster`).

- [ ] **Step 1: Red first (MM1).** `mm1-driver.js` left-clicks and drags on the minimap and right-clicks a minimap point with troopers selected. Run `bash scripts/mc-pixels.sh tests/browser/rows/MM1.json; echo exit=$?`. Expected: `FAIL mm1-oracle.js: no minimap widget`, `exit=1`.
- [ ] **Step 2: Green (MM1).** Implement. Re-run: `exit=0`.
- [ ] **Step 3: ORD3, ORD4, ORD5.** `mc_order_rows.rs` and `orders-driver.js` follow D section 14 rows ORD3 to ORD5 word for word (three Shift waypoints; replace; S; H fires in range; `queue_full`; `not_queueable`; same-tick start; `node_depleted`; attack out of range; attack-move stops to fire; force move with no shot; smart command on ground, node and a slot 1 `dummy`). Run both: `exit=0`.
- [ ] **Step 4: Plants.** After the commit: `Append` treated as `Replace`; `Stop` keeps the queue; no completion rule (ORD3 fails each time); attack-move as move; `choose_target` ignores the ordered target (ORD4); smart command resolved once for the selection in `crates/turts_interaction/src/smart.rs` (ORD5). Restore each.
- [ ] **Step 5: Gate.** G-RUST, G-QA, G-WEB, G-REF(`render.rs`). Commit. **Runs: 3.**

### Task 15: Notices, alerts, audio and the resume hook; ORD2, ORD6, ORD7, ALR1, AUD1

**Seat and model:** `.claude/agents/bevy-rendering-engineer.md`, Luna. Review: `ux-designer` (Sol).

**Branch / needs / wave:** `codex/mc-t15`; needs 7, 12, 13; wave 7. **Design:** sections 9.2 to 9.5; K7, K8; D69. **Owns rows:** ORD2, ORD6, ORD7, ALR1, AUD1.

**Files:** Fill `crates/turts_client/src/feedback.rs` (timeline of D table 9.2; reason texts with engine English defaults and mod overrides; notices that name the unit and the count) and `audio.rs` (OGG Vorbis; categories `master`, `voice`, `alerts`, `effects`; one voice at a time; no audio player entity while a context is suspended); alert derivation from `HealthRecord`, job and research records, `ExtractorStatus` (`crates/turts_net_protocol/src/messages.rs:226-231`), `JobStage::OutputBlocked` (`economy.rs:83-87`), `WeaponStatus::OutOfAmmo` (`messages.rs:239-245`), `InserterStage`, `GatherStage::Waiting`. Modify `crates/turts_client/Cargo.toml` (Bevy feature `audio`), `Cargo.lock`, `crates/turts_client_web/index.html` (a page script, loaded before the wasm, records each `AudioContext` and calls `resume()` on the first pointer down or key down), `crates/turts_client_web/src/lib.rs` (`turts_debug.audio`). Create `crates/turts_client/tests/alerts.rs`, `tests/browser/feedback-driver.js`, `feedback-oracle.js`, `rows/ORD2.json`, `rows/ORD6.json`, `rows/ORD7.json`, `rows/ALR1.json`, `rows/AUD1.json`; `crates/turts_pack/tests/mc_feedback.rs` (ORD2 and ORD6 headless halves).

- [ ] **Step 1: Red first (ALR1).** `alerts.rs` feeds forced records: own unit damaged, research ready, research complete, extractor full, output blocked, out of ammo, inserter waiting above 5 s; a second damage inside the cooldown; three alerts with sounds at once. Run `cargo test -p turts_client --test alerts --locked; echo exit=$?`. Expected: no alert lines (stub), `exit=101`.
- [ ] **Step 2: ORD2 (D revision 6 row, rulings O4 and O7).** Headless half: `mc_feedback.rs` sends one envelope with 300 units (290 own mobile, 5 own static, 5 foreign); the result is 290 accepted, 5 `not_applicable`, 5 `not_owner`, and the 290 move. Pixels half: on `muster`, the driver drags a box over the 40 troopers and the own `post`, then right-clicks the ground. By D section 4.1 the box holds the 40 troopers only. The oracle reads `turts_debug.results`: the reply has 40 accepted and 40 per-unit results; the 40 move and the `post` does not (tick log). `not_applicable` and `not_owner` are headless only, because a box drops own static units and cannot hold a foreign unit.
- [ ] **Step 3: Green.** Implement. ORD6: 100 envelopes at once give `rate_limited` beyond the bucket, and an envelope after 1 s is accepted; every refusal shows a player text within one frame, with the error sound. ORD7: marker and response sound in the first presented frame after the click, before `OrderResult` (frame log, input log); the server tick log shows the first move in the draining tick. AUD1: each context `suspended` before the first click and `running` after; select and order responses in `voice`, one at a time; each volume changes its category gain and survives a reload; an unknown sound id fails `turts-pack`. Run the five rows: `exit=0`.
- [ ] **Step 4: Plants.** After the commit: restore the whole-list return in `crates/turts_sim/src/command.rs` (ORD2); show the reason code (ORD6); bucket size `u32::MAX` (ORD6); spawn the marker on `OrderResult` (ORD7); delay the order one tick (ORD7); cooldown 0 (ALR1); remove the resume script from `index.html` (AUD1). Each row fails; restore each.
- [ ] **Step 5: Gate.** G-RUST, G-QA, G-WEB, G-ACC, G-REF(`command.rs` for the ORD2 plant only). **Runs: 5.**

### Task 17: Forge interface content and guide; SEL1, HUD2

**Seat and model:** `.claude/agents/ux-designer.md`, Luna. Review: `mod-builder` (Sol): can a modder write this file from the schema alone?

**Branch / needs / wave:** `codex/mc-t17`; needs 3, 12, 13, 19, 20; wave 7. **Design:** sections 8.6, 9.3, 9.4 (forge alerts), 12.1 (interface part, R4); K16, K22. **Owns rows:** SEL1, HUD2.

**Files:** Modify `mods/forge/manifest.ron` (`interface`, `assets` lists); create `mods/forge/interface/hud.ron` (selection panel; info panel with the inventory lines and, for the own turret, damage, range and ammo; command card; the lab research entry with its reason "Needs 10 Shell (n/10)"; production queue with a bar in seconds; alert lines; minimap; guide of five pages, page 5 a `List` on `Bindings`; a groups row on `ControlGroups`; reasons with `no_applicable_order` "Cannot move"; alerts `ResearchReady`, `ResearchComplete`, `OutOfAmmo`, `OutputBlocked`, `InserterWaiting(5)` for slot 0 and `OwnUnitDamaged`, `OwnUnitDestroyed` for slot 1), `mods/forge/assets/assets.ron` and the files (icons, one portrait per unit type, one UI font, cursors, sounds; OGG from `ffmpeg` with `libvorbis`, present on the dev machine; the font license is NOT TRACED: record it in `mods/forge/NAMES.md`). Create `tests/browser/hud2-driver.js`, `hud2-oracle.js`, `sel1-driver.js`, `sel1-oracle.js`, `rows/HUD2.json`, `rows/SEL1.json`.

- [ ] **Step 1: Red first.** Run `bash scripts/mc-pixels.sh tests/browser/rows/HUD2.json tests/browser/rows/SEL1.json; echo exit=$?` before the forge HUD exists. Expected: `FAIL hud2-oracle.js: no selection panel`, `exit=1`.
- [ ] **Step 2: Green.** Write the content. The text has no "ticks", no "Applied", no coordinate and no studio word (D section 12.1). Re-run: `exit=0`. `muster` resource line: the HUD2 oracle also checks the ore balance on `muster`.
- [ ] **Step 3: Plants.** After the commit: `SelectionUnits` returns an empty list (HUD2 fails); the tooltip drops the key label (HUD2 fails); pick radius 0 in the core (SEL1 fails); a foreign click fills the command card (SEL1 fails). Restore each.
- [ ] **Step 4: Gate.** G-RUST, G-QA, G-ACC, G-FROZEN (forge maps and units do not change). **Runs: 4.**

### Task 22: GM40, GMA, GMU, EL1, DET1 instruments; tick-cost record

**Seat and model:** `.claude/agents/qa-lead.md` with `performance-architect`, Luna. Review: `simulation-core-engineer` (Sol).

**Branch / needs / wave:** `codex/mc-t22`; needs 6, 7, 11, 13, 19, 20 (R9); wave 7. **Design:** sections 6.1, 6.3 to 6.7, 14 rows GM40, GMA, GMU, DET1, EL1; K17, K19 a. **Owns rows:** GM40, GMA, GMU, DET1, EL1.

**Files:** Create `crates/turts_server/tests/mc_group.rs` (process tests on `muster` with `--tick-log`, `#[ignore]`, added to `scripts/acceptance.sh`), `crates/turts_server/tests/det1.rs` (two server processes with `--command-script tests/browser/fixtures/gm40-script.json --tick-log`; `cmp` of the two logs), `crates/turts_server/tests/el1.rs` (`--economy-log` with GM40, GMA and a queued gather; the validator accepts each tick), `crates/turts_sim/tests/mc_cost.rs` (`#[ignore]`; tick time of spreading, avoidance and combat at 1,000 and 5,000 units), `tests/browser/gm40-driver.js`, `gm40-oracle.js`, `rows/GM40.json`, `tests/browser/fixtures/gm40-script.json`. Modify `scripts/acceptance.sh`, `docs/measurements/mc.md` (section "Tick cost").

- [ ] **Step 1: DET1 red first (D section 14).** Commit the tests. Plant: iterate the grid neighbours through a `HashMap` with a new `RandomState`. Run `cargo test --release -p turts_server --test det1 --locked -- --ignored; echo exit=$?`. Expected: `cmp` reports `differ`, `exit=101`. Restore; re-run: `exit=0`.
- [ ] **Step 2: GM40.** Tick log: at rest (no position change above 0.001 for 2 s), 0 `Blocked`, no two discs overlap (distance below the radius sum minus 0.001), each unit within 4.58 of the target. Pixels: the driver box-selects the 40 troopers (the camera starts on them) and right-clicks one point; the oracle takes each disc centre from `ground_screen_center` back to y = 0 with `clip_from_world`, radius `pick_radius`. Plant: every slot is P. Both halves fail; restore.
- [ ] **Step 3: GMA, GMU.** GMA: two groups of 20 cross at a right angle; no cross-group pair above 20 % of the smaller radius for more than 0.5 s; 0 overlaps at rest; cases F and I 0 overlaps at rest; record (not gate) the `Blocked` count and the largest overlap in motion. GMU: clamp, `target_not_finite`, ring round the `post`, second group with 0 overlaps and 0 `Blocked`, rest or `Blocked` within 2 x (distance / speed) + 5 s. Plants: avoidance off; a `Blocked` unit rests inside another; clamp off (`target_outside_map`); a clamp that keeps NaN; slots reused for the second group. Each fails; restore.
- [ ] **Step 4: EL1.** Plants: drop one `step` input from `AuditState`; write a position outside `step`; restore the validator of `economylog.rs:578-663`. Each fails; restore.
- [ ] **Step 5: Cost record.** `cargo test --release -p turts_sim --test mc_cost --locked -- --ignored --nocapture; echo exit=$?`. Write the figures, the machine and the commit in `docs/measurements/mc.md`. No budget (S section 15 stays Open).
- [ ] **Step 6: Gate.** G-RUST, G-QA, G-ACC, G-FROZEN. Commit the record. **Runs: 3.**

## Wave 8

### Task 16: Shell, menu, rebinding, persistence, focus, rejoin client; KM1, KM2, SEL4, SLOT1, CAM2, SH1, PER1

**Seat and model:** `.claude/agents/web-platform-engineer.md` with `ux-designer`, Luna. Review: `backend-engineer` (Sol) for the rejoin path.

**Branch / needs / wave:** `codex/mc-t16`; needs 7, 12, 15; wave 8. **Design:** sections 3.1 (labels), 3.2, 10.1 to 10.5, 7 (pointer lock); K9, K10, K27; D70 client side, D72. **Owns rows:** KM1, KM2, SEL4, SLOT1, CAM2, SH1, PER1.

**Files:** Fill `crates/turts_client/src/shell.rs` (load, rejection, focus overlay, menu on F10 and Esc, connection lost on `reconnect_required`, `input.rs:46`), `persist.rs` (the keys of D table 10.4; `<line>` rule; native files under `$XDG_CONFIG_HOME/turts` or `$HOME/.config/turts`; a value that does not parse gives defaults and one notice; delete the groups of other `match_id`s at join). Modify `crates/turts_client_web/src/lib.rs` (`sessionStorage` player id into `ClientOptions.player_id`; `getLayoutMap()` once at load in Chrome; `tabindex`; focus at join and on pointer down; `contextmenu` count; bridge keys `focus`, `contextmenu`, `player_id`), `crates/turts_client_web/Cargo.toml` (`web-sys` features `Storage`, `Navigator` if needed; no new crate), `crates/turts_client_web/index.html`, `crates/turts_client/src/lib.rs` (only if the player id path needs it). Create `tests/browser/shell-driver.js`, `shell-oracle.js`, `rows/KM1.json`, `rows/KM2.json`, `rows/SEL4.json`, `rows/SLOT1.json` (`"windows": 3`), `rows/CAM2.json`, `rows/SH1.json`, `rows/PER1.json`, `crates/turts_client/tests/persist.rs`.

- [ ] **Step 1: Red first.** Run `bash scripts/mc-pixels.sh tests/browser/rows/SLOT1.json tests/browser/rows/PER1.json; echo exit=$?`. Expected: `FAIL shell-oracle.js: reload got "no open slot"` (a reload makes a new random id, `crates/turts_client/src/lib.rs:76`), `exit=1`.
- [ ] **Step 2: Green.** Implement.
- [ ] **Step 3: SLOT1.** The driver reloads within 5 s; the oracle sees the countdown and the same slot within 15 s, and the server stats show no owner change. A second window with another id gets "no open slot".
- [ ] **Step 4: SLOT1 copy.** A live window with a copy of the stored id gets "This game is open in another window" after the countdown (a duplicated tab copies `sessionStorage`).
- [ ] **Step 5: PER1, KM1, KM2, SEL4, CAM2, SH1.** As D section 14, word for word. KM1 also runs `turts-pack` on a copy with a reserved default binding (exit 2). KM2 attaches the P1 table of task 1 (`docs/research/mc/p1-keys.md`). Run the seven rows: `exit=0`.
- [ ] **Step 6: Plants.** After the commit: revert the task 7 reclaim branch in `judge_hello` (SLOT1); no retry (SLOT1); `prevent_default_event_handling: false` at `lib.rs:254` (KM2); match `key`, not `code` (KM1); keep dead ids in groups (SEL4); no engine cursor under lock (CAM2); ignore the scroll speed (SH1); multiply `UiScale` by the device factor (SH1); a corrupt value panics (PER1). Each row fails; restore each.
- [ ] **Step 7: Gate.** G-RUST, G-QA, G-WEB, G-ACC, G-REF(`session.rs` for the SLOT1 plant only). **Runs: 6.**

## Wave 9

### Task 21: Four ledger rewrites, ledger to 0, the M2 browser proof; DRV1

**Seat and model:** `.claude/agents/qa-lead.md`, Luna. Review: `web-platform-engineer` (Sol).

**Branch / needs / wave:** `codex/mc-t21`; needs 14, 16, 17, 18, 20; wave 9. **Design:** sections 13.3, 13.4; QA0D:422-436. **Owns rows:** DRV1.

**The four files** (`tests/browser/legacy-bridge.tsv:2-5` after task 12): `t7.js` to `t7-driver.js` + `t7-oracle.js` (QA0D conditions 1 to 4; the driver finds the unit by its slot colour; condition 2 reads `results`); `m3-human.js` to `pt1-driver.js` + `pt1-oracle.js` (the Player path steps 1 to 19, from screenshots, two windows); `m3.js` to `m3-driver.js` + `m3-oracle.js` (research, enable and disable, remove, a drone move into range, through the forge HUD; conservation from the server files); `m2.js` removed: its pure helpers to `tests/browser/harness/png.js` (role `pure`), its oracle helpers to `tests/browser/m2-oracle-lib.js` (role `oracle`), the M2 cases to `m2-driver.js` + `m2-oracle.js` on the migrated M2 packages. The fifth file, `m3-mobile.js`, went in task 12 (R6).

**Files:** the files above; modify `tests/browser/m2-projection.test.js:17` (require `m2-oracle-lib.js`, not `m2.js`), `scripts/t7.sh:82`, `scripts/m2-browser.sh:326`, `scripts/m3-browser.sh:82`, `.github/workflows/ci.yml:108` (the unit test list), `tests/browser/rows/PT1.json` (`"windows": 2`, `"runs": "headed"`), `tests/browser/legacy-bridge.tsv` (header only), `scripts/qa/driver_rule.py:13` (`LEDGER_LIMIT = 0`), `tests/m2/refutations.json` (re-target every browser plant that has an MC equivalent, from the three `economy_ui.rs` plants and `E9-browser-missing-output-mesh`, to the MC code that holds the same defect; ruling O3).

- [ ] **Step 1: Red first.** Set `LEDGER_LIMIT = 0` first. Run `python3 scripts/qa/driver_rule.py tests/browser; echo exit=$?`. Expected: `RED DR7 - tests/browser/legacy-bridge.tsv:1: legacy bridge ledger exceeds limit 0`, `exit=1`.
- [ ] **Step 2: Green.** Rewrite the four files; empty the ledger. Re-run both driver-rule commands: `exit=0`. Run each new pair twice: `bash scripts/t7.sh; echo exit=$?`, `bash scripts/m2-browser.sh; echo exit=$?`, `bash scripts/m3-browser.sh; echo exit=$?`, twice each, `exit=0`.
- [ ] **Step 2b: Retired plants (ruling O3).** A plant with no MC equivalent retires only with a decision note in `vault/Decisions/` that names the plant id, the failure class it covered, and the MC plant that now covers that class. Write the list in the work log for the orchestrator; `qa-lead` reviews the list at gate 2. Run `bash scripts/m2-refutations.sh --case ci-browser; echo exit=$?` on the result: `exit=0`.
- [ ] **Step 3: Plant (DRV1).** After the commit, add `globalThis.turts_debug()` to `t7-driver.js`; the driver rule exits 1 with a Part A line; restore.
- [ ] **Step 4: CI fix.** Push `codex/mc-integration` after the merge; `gh run list --branch codex/mc-integration` shows the `browser` job green at "M2 browser proof", "M3 browser controls" and "M2 browser-only semantic refutations" (the red window closes). Record the run id.
- [ ] **Step 5: Gate.** G-RUST, G-QA, G-WEB, G-REF(all browser cases: `bash scripts/m2-refutations.sh --case ci-browser; echo exit=$?`). **Runs: 8.**

## Wave 10

### Task 23: PT1 and PT2

**Seat and model:** `.claude/agents/player.md` (Sol) through the harness `io` only, and a person in a normal browser. Review: `ux-designer` (Sol) reads the play log. The CEO is the person who plays and signs off PT1 and PT2, unless the CEO names someone else (G12, S:703; ruling O5).

**Branch / needs / wave:** `codex/mc-t23`; needs 14, 15, 21, 22; wave 10. **Design:** section 14 rows PT1, PT2; `## Player path` (D:1634-1676); K22, K27. **Owns rows:** PT1, PT2.

**Files:** Create `docs/research/mc/pt1-playlog.md`, `docs/research/mc/pt2-playlog.md` (the agent and the person; time of each goal; each place the tester stopped for 60 s).

- [ ] **Step 1: Red (instrument).** After a commit of an empty play log, plant: remove guide page 5 from `mods/forge/interface/hud.ron`. Run `bash scripts/mc-pixels.sh tests/browser/rows/PT1.json; echo exit=$?` (row file from task 21). Expected: the oracle fails at the step that needs the group keys, `exit=1`. Restore.
- [ ] **Step 2: PT1, agent.** The `player` agent gets the build, the goals of D section 14 row PT1 and nothing else, and plays through the harness. The oracle checks the 19 Player path steps.
- [ ] **Step 3: PT1, person.** The CEO (or the person the CEO names) plays forge in two browser windows side by side with the goals only. The test fails when the person cannot find an object, an action or a reason, or when 60 s pass with no sign of progress. That person signs off in the play log.
- [ ] **Step 4: PT2.** That person and the agent open `muster`, box-select the 40 troopers and right-click one point; they judge from the screen, by the ground rings, no overlap, near the marker, no jitter; the marker stays until rest. Plant for the agent run: spreading off (the GM40 plant); the agent reports overlap; restore.
- [ ] **Step 5:** Commit the play logs. **Runs: 3.**

## Wave 11

### Task 24: CI wiring, measurements file, vault; REG1, CI1

**Seat and model:** the orchestrator (Sol); `qa-lead` (Luna) runs the REG1 plants in test mode.

**Branch / needs / wave:** `codex/mc-t24`; needs 23; wave 11. **Design:** section 14 rows REG1, CI1; section 15. **Owns rows:** REG1, CI1.

**Files:** Modify `.github/workflows/ci.yml` (a `browser` job step `bash scripts/mc-pixels.sh tests/browser/rows/*.json` with every `"runs": "ci"` row; the PROJ1 step if task 9 step 2 marked it `--ignored`), `docs/measurements/mc.md` (header: commit, machine, the P1 summary line, links to the play logs), `vault/MC Execution Board.md`, `vault/Work Log/<date>.md`, `vault/Reports/<date>.md`.

- [ ] **Step 1: REG1 plants (qa-lead).** After a commit of the CI change: (a) give a single unit an offset slot (`one_unit_alone_takes_the_point` and T7 fail); (b) in the branch of `choose_target` for a unit with no order, take the highest unit id in range, not the nearest (`nearest_target_wins_tie_lowest_unit_id` in `combat.rs:501` and `forge_spec_13_4_fixed_point_and_ten_vs_five_shots` fail). Restore each.
- [ ] **Step 2: REG1 green.** On `codex/mc-integration`: G-RUST, G-QA, G-ACC, G-WEB, G-FROZEN, `bash scripts/m2-refutations.sh --case ci-native; echo exit=$?`, `bash scripts/m3-refutations.sh --case all; echo exit=$?`, `bash scripts/mc-pixels.sh tests/browser/rows/*.json; echo exit=$?`. Each prints `exit=0`.
- [ ] **Step 3: CI1.** Push; `gh run list --branch codex/mc-integration --limit 1` shows `check`, `browser` and `agents` green. Record the run id.
- [ ] **Step 4:** Tick each row of D section 14 with its command, exit code and the red seen. Do not merge into `main`: report to the CEO with the integration commit, the run count and the CI link. **Runs: 2.**

---

## Risks

1. **Name collisions in the no-content scan.** Engine identifiers and literals block some content ids (ruling O1 renamed `target` and `charge`). Tasks 18, 19 and 17 run the scan after each package change.
2. **The refutation registries move with the code.** 41 cases of `tests/m2/refutations.json` and 17 of `tests/m3/refutations.json` anchor on exact text (`tests/m2/refutations.py:270-280`); MC edits 12 planted files (table G-REF). A re-anchor is a test-instrument edit; `qa-lead` reviews each one.
3. **Red window from task 12 to task 21.** Two more `browser` job steps go red (table "Known red window"). T7 must stay green; it comes first in the job.
4. **Task 2b is wide.** It touches 6 crates and about 30 files in one merge. Mitigation: 2a lands the types first; 2b keeps per-unit results as a copy of the command outcome until task 4.
5. **PROJ1 on hosted CI: NOT TRACED.** Task 9 step 2 decides between a normal test and an ignored local test plus the `browser` job.
6. **P1 can stop the plan (K11).** Tasks 3 step 8 and 9 step 3 wait for the CEO pick if P1 is red.
7. **Acceptance load on tasks 13 to 16 (R10).** They own 20 of 42 rows with 19 runs of estimate.
8. **Determinism.** Every new iteration is in unit id order (D section 6.1); DET1 is the guard.

## Doc updates for the orchestrator

1. `CLAUDE.md`, crate table: add `turts_interaction` (responsibility: D section 2.1 row; forbidden: `bevy*`, `wgpu`, `winit`, `turts_sim` on host and wasm32). `turts_client` row: the new modules (`core_adapter.rs`, `ui_shared.rs`, `actions.rs`, `camera.rs`, `widgets/`, `visuals.rs`, `minimap.rs`, `feedback.rs`, `audio.rs`, `shell.rs`, `persist.rs`); `economy_ui.rs` removed. `turts_client_web` row: `turts_ui` from `widgets::snapshot`, `turts_scene.widgets`, the bridge keys of D section 13.2, the audio resume script.
2. `CLAUDE.md`, "Build and test commands": `ENGINE_API_VERSION` is 0.2.3; `NET_PROTOCOL_VERSION` 4; the four version table rows; `turts_client --actions <path>`; `turts_server --command-script <path>`; `scripts/mc-browser.sh`, `scripts/mc-pixels.sh`, `tests/browser/rows/`; the role `harness`; the ledger has 0 rows and `LEDGER_LIMIT` is 0; the measurements file `docs/measurements/mc.md`; "M1 to M3, QA0 and MC are done" when the CEO merges.
3. `CLAUDE.md`, "Decisions that are still open": the player control lines that MC designed (control model, input map, HUD, camera controls, client shell client side, order envelope, interaction core) change with the spec rows below.
4. Spec section 2: the status of D65, D66, D67, D68, D69, D70 (client side), D72, D73, D74 from Proposed (design Proposed) to the status that the CEO gives after MC. D71 does not change.
5. Spec section 15: S:755 "Player input" (closed by D section 3 and P1); S:756 "HUD" open items: the widget set, the mod schema, image formats and size limits, the image reference form (closed by D section 8 and K12); S:757 "Order model": the queue, the envelope fields, the protocol bump (closed by K24, D section 5.1, K14); S:750 "Presentation": camera controls (D68) and audio format OGG Vorbis (K7); the lighting API keeps only `LightDef` (K23).
6. Spec S:722, MC row, acceptance evidence: "D section 14 of `docs/superpowers/specs/2026-10-29-mc-player-control-design.md`" (the producer confirms it, D section 17.3 question 1; ORD2 per D revision 6).
7. `docs/qa/specs.tsv`: the MC row from `gated` to `done` in the merge commit. `docs/qa/milestones.tsv` needs no change (MC is order 5).
8. The M4 draft amendment takes protocol 5 (D section 17.3 question 4).
9. The design, stale citations (for a later amendment): `content_m3.rs:339` is `crates/turts_mod_runtime/tests/content_m3.rs:339` (not `src/`); `ContentRef` is at `crates/turts_net_protocol/src/economy.rs:16` (not `:15`); D section 5.5 "the protocol id of netcode stays": the line stays, the value becomes 4; D section 6.2 "No forge unit gets an order in the M3 cases": `crates/turts_pack/tests/forge_fixed_point.rs:82` and `forge_cases.rs:516,1017` send a plain `MoveTo` (a plain move, so the stream still holds, D section 6.1).
10. `scripts/boundary.sh:4-8`: the comment about the no-content grep is out of date (the scan derives tokens from `mods/`).
11. Vault: `vault/MC Execution Board.md`; a decision note for the rulings O1 to O8 (2026-10-29) and for each retired plant of task 21; the daily report.

## Open questions for the orchestrator

Rulings of 2026-10-29: O1 (a), design revision 6 (`dummy`, `rush`); O2 confirmed (P1 on the dev machine); O3 re-target, retire only with a decision note (task 21 step 2b); O4 design revision 6 (ORD2 row); O5 the CEO signs off PT1 and PT2 unless the CEO names someone else; O6 R1 to R11 accepted, critical path 51 runs; O7 the ORD2 pixels half holds the 40 troopers only (40 accepted, 40 per-unit results; `not_applicable` and `not_owner` headless only; commit `0b1a250`); O8 the ORD1 row says "rush" (commit `0b1a250`). No question is open.

## Evidence for this plan

| Command | Claim tested | Result that would refute | Observed |
| --- | --- | --- | --- |
| `git diff --stat 7a90f97 c1a5fd1 -- crates mods scripts tests` | the design code citations hold at `c1a5fd1` | a changed code file | empty |
| `git diff --stat c1a5fd1 b8b3a8f`; `git diff --stat b8b3a8f 0b1a250` | revision 6 changes only the design | another file | 1 file each: the design (11 and 8 lines; 2 and 2 lines) |
| `sed -n` of each cited range (boundary, client, render, input, protocol, server, sim, mod API, mods) | every `path:line` in this plan | a moved line | as cited; two stale design citations (Doc update 9) |
| Python over D section 14 and `## Yardstick coverage` (scratch `coverage.txt`) | 42 rows; 81 `in` rows map to 27 tests, all in section 14 | a test outside section 14 | 42 rows; 81 `in`, 15 `done:`, 85 `later:`; none outside |
| Python longest path and cycle check (scratch `deps.py`) | design path 46 (D section 15.2); plan waves | another sum, or a cycle | design 46 reproduced; plan 51, no cycle |
| JSON scan of `tests/m2/refutations.json`, `tests/m3/refutations.json` | plants anchor in files MC edits | no plant in an MC file | 12 planted files edited by MC (table G-REF) |
| `rtk proxy grep -n` for `"charge"`, `"target"`, `"research"` in non-test `crates/` | content ids collide with the scan | no literal | `scenario.rs:134`, `m2_variants.rs:121`, `visuals.rs:31`, `content_m3_validation.rs:407`, `debug.rs:98` |
| `sed -n 403-410,451-540 crates/turts_checks/tests/support/mod.rs` | the scan checks qualified and local tokens on every identifier and literal | local tokens skipped | both added; identifiers and whole literals checked |
| `sha256sum tests/m3/m2_operation_stream.json` | the frozen hash of `refutations.py:18` | another hash | equal |
| `yardstick.py --check`, `completeness.py --self-test` then `--enforce`, `driver_rule.py --self-test` then `tests/browser`, `bash scripts/boundary.sh` | the baseline gates are green at `c1a5fd1` | a non-zero exit | all `exit=0`; `21/21 cases passed`; `checked specs: 2`; 11 `checked:` lines and `boundary ok` |
| `sed -n 56,90 archive.rs`, `pack.rs:60-87` | NOT TRACED item 4 (asset bytes in the archive) | no raw accessor | `file()` at `archive.rs:56`; every regular file packed |
| `which firefox xdotool google-chrome Xvfb`; `ffmpeg -encoders` | P1 and OGG can run locally | a missing tool | all present; `libvorbis` encoder present; `firefox --version` did not return in 120 s |

**NOT TRACED:** the lightyear message size limit (task 2b step 1); whether `camera_system` needs a GPU adapter for an `Image` target (task 9 step 2); whether the server sees the WebSocket close at once (K27, task 16 step 3 measures it); the Firefox version; the font license of the forge and muster UI font; the run time of `bash scripts/m3-refutations.sh --case all` after MC; whether `move` as a local id is a scan hit (syn keyword handling; tasks 18 and 19 run the scan); a separate frozen M3 operation stream (only the M2 stream fixture and the forge fixed point were found).
