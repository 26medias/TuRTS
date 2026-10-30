# MC Player Control and Client Foundation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Plan status:** Passed gate 2 (2026-10-29): six lenses (`spec-rigour-reviewer`, `technical-director`, `bevy-rendering-engineer`, `player`, `simulation-core-engineer`, `qa-lead`); round 1 21 High; repair 1, re-check 1 (2 High left); repair 2, re-check 2 (1 new High); orchestrator fix, re-check 3 by `bevy-rendering-engineer`: pass, 0 High. Reports in `docs/research/mc/gate2-*.md`. Design of record: revision 10 (`488b3d3`). CEO authorized the build (full design, K1 to K27); Codex brief in `vault/Reports/2026-10-29 MC Codex brief.md`.

**Goal:** A person plays the M3 industrial demo (`mods/forge`) in a desktop browser with mouse and keyboard only, and 40 units sent to one point arrive as a group (S:722).

**Architecture:** A new crate `turts_interaction` holds the input map, selection, groups, smart command, command composition and camera state, with no `bevy*` dependency (K1). The wire goes to protocol 4 with an open order envelope and per-unit results (D73, D65). The simulation gets `SimSet::Orders`, a per-unit order queue and flat-ground group movement. Every HUD element during play is a mod widget (D67).

**Tech Stack:** Rust 2024, Bevy `=0.20.0`, lightyear `=0.31.0`, RON `=0.12.2`, `glam = "=0.33.12"` (K26), Python 3 standard library, Node 20 on CI (v24.10.0 local), playwright-core in `tests/browser`.

**Design:** `docs/superpowers/specs/2026-10-29-mc-player-control-design.md`, revision 9 (commit `f9353bb`), passed gate 1, "D" below (D section N; D:line at `f9353bb`). Product spec `RTS-Engine-Project-Specifications.md` v0.8, "S" below. **CEO decisions (2026-10-29):** K1 to K27 as recommended; K11 stays conditional on probe P1; K20 applied in revision 5; budget option 1 (the full design, no cut). The design is the contract. When this plan and the design disagree, the design wins; report the conflict.

## Global Constraints

- `ENGINE_API_VERSION` `"0.2.2"` to `"0.2.3"` (`crates/turts_mod_api/src/lib.rs:33`). Append `"0.2.3"` to `RELEASED_ENGINE_APIS` (`lib.rs:36`). Additive: every 0.2.2 package loads unchanged (K13).
- `NET_PROTOCOL_VERSION` `3` to `4` (`crates/turts_net_protocol/src/lib.rs:13`, K14). The netcode `protocol_id` lines (`crates/turts_client/src/join.rs:275`, `crates/turts_server/src/net.rs:44`) do not change, but they read `NET_PROTOCOL_VERSION`, so the netcode id also becomes 4. A version 3 client then fails at the netcode handshake, before `Hello`. The test hook `--protocol-version 3` still reaches `judge_hello` (`crates/turts_server/src/session.rs:97`).
- Four new `VERSION_TABLE` rows (`crates/turts_mod_runtime/src/version_table.rs:20-31`), each `since_api: "0.2.3"`, present when the list is not empty: `exports.orders`, `exports.interface`, `exports.input`, `exports.assets`. `VersionTableRow.present` takes the parsed package content, not only `&Manifest` (`version_table.rs:6-10`; D sections 5.2, 11.4).
- Every new schema type has `#[serde(deny_unknown_fields)]`. The four new `Exports` lists have `#[serde(default, skip_serializing_if = "Vec::is_empty")]` (D section 11.1).
- No Bevy type in `turts_mod_api`, `turts_net_protocol` or `turts_interaction` (D15). `turts_interaction` forbids `bevy*`, `wgpu`, `winit`, `turts_sim` on host and wasm32 (K1, D section 2.4). `turts_client` and `turts_client_web` never depend on `turts_sim` (T1, `scripts/boundary.sh:38-42`).
- Tests that call `turts_server` code live in `crates/turts_server/tests/` and include the source file with `#[path = "../src/<file>.rs"]` and `#[allow(dead_code)] mod <file>;`, as `crates/turts_server/tests/economy_export.rs:1-2` does (gate 2 ruling 1). The `#[allow(dead_code)]` is on each such `mod` line, because a test crate that calls only `to_sim` makes strict clippy refuse the unused `to_wire` (re-check 1 rigour N1). So `crates/turts_server/src/orders.rs` and `orderlog.rs` use only external crate paths (no `crate::`). `turts_server` stays a binary; no new crate.
- Frozen evidence, no re-freeze (K19 a): `sha256sum tests/m3/m2_operation_stream.json` prints `bcbf72c2206251f635a7c7f5b6c8f573d69a5fc05a5bd77716e98ebfb08e370b` (pinned at `tests/m3/refutations.py:18`). `m2_operation_stream_matches_baseline` (`crates/turts_pack/tests/m3_m2_stream.rs:350`) and `forge_spec_13_4_fixed_point_and_ten_vs_five_shots` (`crates/turts_pack/tests/forge_fixed_point.rs:10`) stay green with no edit to their expected values. A task that must change either one stops and reports (the CEO approves a re-freeze, K19). No separate frozen M3 operation stream exists (D:1250 names one; grep finds only the M2 fixture and the forge fixed point).
- Plain move (gate 2 ruling 6): a plain move (one `Move` for one unit, no queue, no group above one unit) writes `MoveTarget` only. It writes no `UnitOrders.current` and no order operation; `Moving` comes from `MoveTarget`. From task 4, `orders::to_sim` sends every envelope as `CommandKind::Order`. The frozen stream keeps its direct `CommandKind::MoveTo` and `CommandKind::Gather` calls (`m3_m2_stream.rs:258-266,326-334`); task 4 adds the test that the `Order` path gives the same operation count.
- Content local ids: no package local id may equal an identifier or a whole string literal of non-test engine source (the no-content scan, `crates/turts_checks/tests/support/mod.rs:403-410,451-540`). The ids of the table in task 18 hold this at `d21c9ee` (code unchanged at `f9353bb`) (scratch probe `scanprobe`, the real `scan_rust`). A later task whose engine code hits a content id renames its engine identifier, not the content id.
- Numbers from the design, verbatim: envelope cap 4,096 units, the client splits above it with one `group` (K4 b). Rate buckets per slot: 64 envelopes with refill 640 per second; 16,384 unit orders with refill 163,840 per second (K25). Spreading: at most 2,048 units per tick for all slots; pending list 16,384 units per slot (D section 6.3). `queue_depth` from 1 to 255, default 16 (K24). Spacing s = 2 x r_max + 0.1; separation reach r_i + r_j + 0.05; landing within one step or 0.025; stall 1 s; at most 3 slot claims (D sections 6.3 to 6.5). R_near = r_max x (sqrt(N / 0.6) + 1), 4.58 for 40 units of radius 0.5 (K17).
- K12 caps: decoded image bytes at most 256 MiB (width x height x 4 from the PNG IHDR) per package and per locked set; each image at most 2048 x 2048 px; each cursor image at most 128 x 128 px; font files at most 8 MiB; sound files at most 32 MiB; sounds at most 300 s in total and 30 s each. The decoder does not run on the server.
- K18: `turts-pack` warns with exit 0 below 11 px text and below 24 x 24 px buttons.
- Client numbers: double click 400 ms and 4 px; drag threshold 4 px; hover tooltip 0.5 s; edge band 8 logical px; UI scale 0.75 to 2.0; alert cooldown at least 1 s, at most 2 alert sounds, last 5 lines; move marker until rest, limit 60 s; `Blocked` mark 2 s; rejoin retry each 1 s, countdown 15 s; netcode timeout 10 s (`crates/turts_client/src/join.rs:278`); `turts_debug.frames` holds the last 64 presented frames and, apart from them, every frame record whose `applied_seq` is not empty since the last `turts_debug.frames_clear()` call, at most 4,096 such records (D 13.2 revision 10; the oracle reads after `drive` ends, so a record with input must not fall out of the ring). Palette Okabe-Ito `#0072B2`, `#E69F00`, `#009E73`, `#D55E00`, `#56B4E9`, `#CC79A7`, `#F0E442`, `#000000`; CIEDE2000 distance at least 20 for each pair under protan, deutan, tritan (Machado 2009).
- Storage keys (D section 10.4): `turts.v1.player_id` (`sessionStorage`), `turts.v1.bindings.<mod id>.<line>`, `turts.v1.settings`, `turts.v1.groups.<match_id>.<slot>`.
- New dependencies: only `glam = "=0.33.12"` (task 8) and the Bevy feature `audio` on `turts_client` (task 15). Any other new crate or npm package needs the orchestrator.
- No task edits `CLAUDE.md`, `.claude/agents/`, the design, or the product spec. Doc changes go to "Doc updates for the orchestrator".
- Code style: 4 spaces per indentation level in Rust, Python, JavaScript, RON, TSV and shell examples. Write prose in ASD-STE100. Executor commands use plain `grep`, not `rtk`.

## Review Focus

1. A page reload while the old session still lives (up to the 10 s netcode timeout): the player expects the countdown and the same slot, not "no open slot". Pinned in task 16 (step 2, SLOT1).
2. A box drag that holds only own static units, or own and foreign units together: the player expects the static units when no mobile unit is in the box, and never a foreign unit. Pinned in task 13 (step 2, cases `static_only` and `mixed_owner`).
3. An order to a point outside the map, or a NaN or infinite target from a bad client: the player expects a move to the edge, and the server expects `target_not_finite`. Pinned in task 22 (step 4, GMU).
4. A 0.2.2 package with no orders (an old mod): the player expects "This game defines no orders" once, not a silent client. Task 18 step 1 (`old_acme_copy_loads_with_zero_orders`) and task 15 step 1 (`no_orders_notice_shows_once`).
5. A corrupt `localStorage` value, a duplicated browser tab, or a rebind to a key that is in use: the player expects defaults with one notice, the refusal "This game is open in another window", and the swap-or-cancel prompt. Pinned in task 16 (step 2).

## Execution contract

- Base: `main` at `f9353bb` or later. Create the integration branch `codex/mc-integration` from it before wave 1.
- One branch per task, `codex/mc-t<N>` (task 2 has two: `codex/mc-t2a`, `codex/mc-t2b`). Each task branch starts from `codex/mc-integration` after every task of its Needs has merged there (gate 2 ruling 20).
- One worktree per lane: `git worktree add ../TuRTS-mc-t<N> codex/mc-t<N>`, and `CARGO_TARGET_DIR=target/mc-t<N>` inside it. The trunk `dist/` and the `tests/browser/node_modules` are per worktree. Delete the worktree and its target directory after the merge.
- Merge a task into `codex/mc-integration` only after its gate is green. After each merge, re-gate the integration branch: G-RUST, G-QA, G-FROZEN, and G-WEB when the task touched a client or server file. After each wave, push `codex/mc-integration` and read the hosted run (`gh run list --branch codex/mc-integration --limit 1`); record the run id on the board.
- Codex does not load `.claude/agents/`. Each dispatch tells the agent to read, in this order: its role file (the path in the task), `CLAUDE.md`, this plan task, and the design sections the task cites (`vault/Reports/2026-10-29 QA0 Codex brief.md` section 3).
- Models (`vault/Team.md:42`): the builder runs on Luna (execution); the reviewer runs on Sol (review); the orchestrator runs on Sol. State the model and the reasoning effort on each dispatch.
- Build each binary with its own `cargo build -p <crate>`. Stage explicit paths. Never `git add -A`. Never pipe a red or a green run through `tail`, `head` or `grep` before you read its exit code: write `; echo exit=$?` after the command.
- "Red" means a test that compiles and fails, or a tool that exits 1 with its failure line. A compile error is not a red. When a test names a new item, add the item first as a stub that compiles (signature first).
- **Step order in every task (gate 2 ruling 12): red, green, commit, gate, plants.** The plants come last, after the commit and the gate. Restore each plant with `git checkout -- <file>` (for a file that the run created: `rm <file>`), then `git status --porcelain -- <the changed files>` must be empty. Never use `git checkout --` on a new or uncommitted file. A plant that needs a source edit in another task's file edits only the committed tree and restores it.
- Two kinds of red. **Red first:** the owner writes the test before the feature, and the test fails on the build before the feature. **Red by plant:** the feature merged in an earlier task, so the owner applies the red build that D section 14 names as a plant (last step) and records the failure. Every acceptance row has one or both (table "Acceptance ownership").
- Record each task's red output, green output, plant outputs, changed paths, commit and run count in `vault/Work Log/` and on the board `vault/MC Execution Board.md`.
- Stop and report to the orchestrator, do not decide: P1 is red on a browser, or Firefox does not start (task 1 step 4); a frozen value changes; a plan step contradicts the design or the spec; a Decided item looks wrong; the no-content scan is red on a content id of the task 18 table; a task uses twice its run estimate; the run count reaches the budget total.

### Local gate commands

Each task names the sets it runs, in this order, after its commit.

- **G-RUST:** `cargo fmt --all --check; echo exit=$?`, `cargo clippy --workspace --all-targets --locked -- -D warnings; echo exit=$?` (`.github/workflows/ci.yml:29`), `cargo check --workspace --exclude turts_server --target wasm32-unknown-unknown --locked; echo exit=$?` (`ci.yml:31`), `cargo test --workspace --locked; echo exit=$?` (`ci.yml:33`; it includes `real_source_scan_is_clean`), `bash scripts/boundary.sh; echo exit=$?` (`ci.yml:35`). Each prints `exit=0`.
- **G-QA:** `python3 scripts/qa/yardstick.py --check; echo exit=$?` (prints `yardstick matches inventory`), `python3 scripts/qa/completeness.py --self-test && python3 scripts/qa/completeness.py --enforce; echo exit=$?` (prints `self-test: 21/21 cases passed; failures=0` and `checked specs: 2`), `python3 scripts/qa/driver_rule.py --self-test && python3 scripts/qa/driver_rule.py tests/browser; echo exit=$?` (after task 21 the default ledger limit is 0, so the same command holds the ledger at 0 rows). Each prints `exit=0`.
- **G-ACC:** `bash scripts/acceptance.sh; echo exit=$?` prints `exit=0`. The script lists each test by name (`scripts/acceptance.sh:84-120`), so a task that adds an ignored process test adds its line there.
- **G-WEB:** `(cd crates/turts_client_web && trunk build --release); echo exit=$?`, then `bash scripts/t7.sh; echo exit=$?`. Both print `exit=0`. After task 21, `t7.sh` runs `t7-driver.js` and `t7-oracle.js`.
- **G-FROZEN:** `sha256sum tests/m3/m2_operation_stream.json` (the value of Global Constraints), `cargo test -p turts_pack --locked --test m3_m2_stream --test forge_fixed_point --test forge_cases; echo exit=$?` prints `exit=0`.
- **G-REF:** the refutation cases that plant into the files that the task changed. List them with:

      git diff --name-only codex/mc-integration...HEAD > /tmp/mc-changed.txt
      python3 -c 'import json; ch=set(open("/tmp/mc-changed.txt").read().split()); [print(f.split("/")[1], c["id"], c.get("runtime","rust")) for f in ("tests/m2/refutations.json","tests/m3/refutations.json") for c in json.load(open(f)) if ch & (({c["plant_path"]} if isinstance(c.get("plant_path"),str) else set(c.get("plant_path") or [])) | {p["path"] for p in c.get("plants",[])})]'

  The selector reads `plant_path` and `plants[].path` (multi-file cases; re-check 1 qa and rigour N2). Scratch run on `transaction_m3.rs`, `machine.rs`, `command.rs`: it lists E4-local-capacity, E6-capacity-bypass, C1-capacity-compound-bypass and C5-outputblocked-refund, `exit=0`. Run each listed case with `bash scripts/m2-refutations.sh --case <ids>; echo exit=$?` or `bash scripts/m3-refutations.sh --case <ids>; echo exit=$?`. Each prints `exit=0`. A plant anchor must occur exactly once (`tests/m2/refutations.py:270-280`). When the task moved the planted code, it edits the case's `anchor`, `replacement`, `plant_path`, `expected_failure` or `command` in the registry, so the plant breaks the same behaviour in the new code (gate 2 sim 7). `qa-lead` reviews each such edit. Two tasks of one wave that edit the same registry merge in task-number order; the second rebases. Browser cases (`"runtime": "browser"`) run through `scripts/m2-browser.sh` locally; they are mandatory for tasks 2b, 9, 11, 13 and 14 from the first commit at which task 20 step 1 has made `bash scripts/m2-browser.sh; echo exit=$?` give `exit=0` on a clean tree (gate 2 ruling 8). Before that, the browser cases are "not checked", not a pass: the task records the clean-tree failure text that task 20 step 1 found. A check that "the plants add no other failure text" is valid only after the M2 red is fixed, because a defect after the failing point is not reached (re-check 1 qa). If task 20 step 1 finds that the cause is not in the harness or a tool, it names the owner task on the board, and that task fixes it first in its own red, green, commit cycle.
- **G-PIX(rows):** `bash scripts/mc-pixels.sh <row file>...; echo exit=$?` (task 20) runs each row twice; prints `exit=0`. A row with `"runs": "headed"` prints `SKIP <row> headed` and does not count as a pass.

Plant files that both registries name and that MC edits (JSON scan at `f9353bb`; the G-REF command finds the rest):

| Planted file | Cases | MC tasks that edit it |
| --- | --- | --- |
| `crates/turts_sim/src/command.rs` | M2 E6-carried-item-loss | 4, 6 |
| `crates/turts_sim/src/bootstrap.rs` | M2 E5-item-resource-as-balance-account; M3 C6-research-running-refund | 4 (`UnitOrders` at spawn) |
| `crates/turts_sim/src/economy/gather.rs` | M2 E5-conversion-cargo-separation, E6-node-replenishment, E6-duplicate-conversion, E6-self-store, E6-no-op-economy, E3-declared-conversion-coefficient, E8-foreign-gather-owner-check | 4 |
| `crates/turts_sim/src/economy/transaction_m3.rs`, `extractor.rs`, `machine.rs` | through `plants[].path`: M2 E4-local-capacity, E6-capacity-bypass; M3 C1-capacity-compound-bypass, C5-outputblocked-refund | none planned; G-REF lists them if a task edits one |
| `crates/turts_sim/src/economy/transaction.rs` | M2 E4-local-capacity, E6-mixed-cost-partial-commit, E6-arithmetic-overflow, E6-duplicate-refund, E6-repeated-spawn, E6-capacity-bypass, E6-event-only-spawn, E8-mixed-cost-partial-commit; M3 C10-inserter-state-survives-despawn | 4 (phase mark) |
| `crates/turts_sim/src/plugin.rs` | M3 C15-combat-before-production | 4 |
| `crates/turts_sim/src/economy/combat.rs` | M3 C13-ammo-debit-saturates, C14-health-zero-does-not-destroy, C16-double-range | 5 |
| `crates/turts_server/src/session.rs` | M2 E7-refused-hello-claims-slot, E8-rejection-ack-send-reversal (its `expected_failure` names `CommandRejected`, which 2b removes) | 2b, 7 |
| `crates/turts_server/src/export.rs` | M2 E9-owner-private-raw-dto, E9-output-in-snapshot-and-delta | 2b, 4 |
| `crates/turts_server/tests/economy.rs`, `crates/turts_server/tests/support/wire_sender.rs` | M2 E8 and E9 cases (`command`) | 2b, 4 |
| `crates/turts_client/src/ingest.rs` | M2 E9-midtrace-rearm-ignored; browser E9-browser-duplicate-outstanding, E9-browser-ignore-midtrace-rearm | 2b |
| `crates/turts_client/src/replica.rs` | M2 E9-duplicate-spawn-tail-replay; browser E9-browser-position-only-snapshot, E9-browser-unbounded-buffer | 2b |
| `crates/turts_client/src/economy_ui.rs` | browser E2-browser-hardcoded-label, E9-browser-missing-button, E9-browser-debug-only-ui | 2b, 9, 12 (removed) |
| `crates/turts_client/src/render.rs` | browser E9-browser-missing-output-mesh | 9, 11, 13, 14 |
| `crates/turts_mod_runtime/src/content.rs` | M2 E10-public-error-provenance | 3 (wires `content_mc_validation.rs`) |
| `crates/turts_checks/tests/support/mod.rs` | M2 E1-derived-scan | 3 |

The eight browser cases all run `scripts/m2-browser.sh`, and their `expected_failure` texts are `tests/browser/m2.js` asserts (`m2.js:662,671,798-811`; `M2_CASE_SUMMARY`, `scripts/m2-browser.sh:130-134`). Task 21 removes `m2.js`, so task 21 owns all eight (task 21 step 4).

### Known red window

On `main`, the `browser` job is red at "M2 browser proof", and the later steps "M3 browser controls" and "M2 browser-only semantic refutations" are skipped (run 40960227409, gate 2 qa 3). So no hosted run checks `m3.js` or a browser plant before task 21. Task 20 step 1 traces the cause (gate 2 ruling 8). Locally, G-WEB adds `bash scripts/m3-browser.sh; echo exit=$?` for tasks 2b, 4, 6, 9 and 11, the builds before task 12 on which `m3.js` can run (the engine panel exists). From task 12 to task 21 no build can run `m3.js`, because it reads the panel through `turts_ui`; task 21 is the first build after (re-check 1 qa). T7 (`ci.yml:109-114`) comes before the red step and must stay green in every task.

| CI step (`.github/workflows/ci.yml`) | Red from | Green at | Cause |
| --- | --- | --- | --- |
| "M2 browser proof" (`:115-116`; `scripts/m2-browser.sh:326` runs `tests/browser/m2.js`) | before QA0 (run 40943729334) | task 20 (if a harness or tool defect) or the owner task that task 20 step 1 names | task 20 step 1 records it |
| "M3 browser controls" (`:117-122`, `scripts/m3-browser.sh:82` runs `tests/browser/m3.js`) | skipped now; red from task 12 | task 21 | `m3.js` reads the engine panel through `turts_ui`; task 12 removes the panel |
| "M2 browser-only semantic refutations" (`:123-124`) | skipped now; red from task 12 | task 21 | three cases plant into `economy_ui.rs`, which task 12 removes |

## Plan refinements of the design

The design wins on a conflict. These refinements fill gaps; none changes a Decided item, a K call, or a number. Ruling O6 accepted R1 to R11; gate 2 ruling 16 changed R5.

| # | Refinement | Reason (evidence) |
| --- | --- | --- |
| R1 | The K5 lift (`crates/turts_mod_runtime/src/content_m3_validation.rs:380-383`) moves from task 5 to task 3. Task 19 then needs 3 only. | `muster` troopers have a weapon and movement (D section 12.3). The sim already fires from any unit with a weapon (`combat.rs:165-200`). |
| R2 | Task 5 needs task 6. | Attack on a point moves to the slot of the point (D section 5.2); target choice uses the neighbour grid of D section 6.4. Both are task 6. |
| R3 | Task 2 lands in two merges: 2a (new wire types, additive) and 2b (the switch to protocol 4). 2b needs 3 and 18. | After the switch, `--move` sends "the first `OrderDef` of the package whose behaviour is `Move`" (D section 5.5). T7 uses `acme` (`scripts/t7.sh:30`); `scripts/acceptance.sh:120` runs an F11 process test on `forge`. |
| R4 | Task 18 writes the orders part of all five packages. Task 17 adds the forge interface, assets and guide only. | Same as R3. |
| R5 | Task 9 creates stub modules for tasks 10 to 16 and registers them in `build_app`. It removes `fit_scene_camera` and the fixed camera start, and owns the core camera state (gate 2 ruling 16). It moves `Selection` from `economy_ui.rs` to a new `ui_shared.rs` with its `init_resource` (`economy_ui.rs:1291`). It adds every new `turts_debug` and `--out` key, each read from a typed resource that the owning module writes (gate 2 ruling 20). After task 9, no task edits `debug.rs`. | `lib.rs` and `debug.rs` are shared; `render.rs:17,247,540`, `debug.rs:66,113`, `live_view.rs:22,404`, `input.rs:5` and `crates/turts_client_web/src/lib.rs:35` import `economy_ui` items. |
| R6 | Task 12 writes `hud-driver.js` and `hud-oracle.js`, removes `tests/browser/m3-mobile.js` and its ledger row (5 to 4 rows). Task 21 rewrites the other four ledger files. | HUD1 can go red first only before the panel removal (task 12). |
| R7 | Task 12 changes the page canvas to 100 % of the viewport (`crates/turts_client_web/index.html:8`). winit already sets `tabindex="0"` and focuses the canvas on each pointer down when prevent-default is on (gate 2 bevy F8 b: `winit-0.30.13/src/platform_impl/web/web_sys/canvas.rs:100-108`, `pointer.rs:116-120`, NOT TRACED by the producer). Task 16 handles focus loss through the canvas blur (Bevy `WindowFocused`). | HUD1 and PROJ1 test 1920 x 1080. A click outside the canvas blurs it while `document.hasFocus()` stays true (bevy probe). |
| R8 | Pixels rows run through one row file each, `tests/browser/rows/<ROW>.json`, and one runner `scripts/mc-pixels.sh` (task 20). Task 24 adds one CI step for the runner. | D section 14: hosted CI runs each row except "headed" rows. |
| R9 | `docs/measurements/mc.md` is written in the order 13, 22, 24. | One file, three writers. |
| R10 | Acceptance rows with a pixels half are owned by the first client task in which every half can be built. The simulation tasks prove their parts with unit tests. | D section 15.1 names no owner for the pixels halves. |
| R11 | Task 10 edits no shared client file except `options.rs`; `--out` keys come from task 9 (R5). | Same as R5. |

## Waves

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

Needs (the source for the waves; scratch `deps.py` checks it for cycles):

```text
1: -            2a: -            3: -              20: -
8: 2a 3         18: 3            19: 3             2b: 2a 3 18
4: 2b 3         9: 1 8 2b
6: 4            7: 2b 4          11: 9 19 20       12: 3 9 19 20
5: 4 6 19       13: 4 6 9 11 19 20
10: 5 9 11 19 20                 14: 5 12 13 15    15: 4 7 12 13      17: 3 12 13 19 20
16: 7 12 15     22: 6 7 10 11 13 19 20
21: 14 16 17 18 20               23: 14 15 21 22   24: 23
```

Critical path (scratch `deps.py`, no cycle): 3, 18, 2b, 4, 6, 13, 15, 16, 21, 23, 24 = 6 + 2 + 4 + 8 + 6 + 5 + 5 + 6 + 8 + 3 + 2 = **55 runs in series**. The design path was 46 (D section 15.2); the first plan draft had 51. Repair 2 moves task 14 to wave 8; the path does not change (scratch `deps.py` rerun).

### Shared files and their order

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

## Budget

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

The total is above the design range (165 to 205, D section 15.1). Stop and report to the CEO at 212 runs, and report now that the plan is above the range (gate 2 ruling 21). Risk to the estimate: tasks 13 to 16 own 20 of the 42 acceptance rows (R10), and task 20 gains the trace, the `io` contract test and `--serve-only` (gate 2 rulings 8, 9).

## Acceptance ownership

Each row of D section 14 has one owner. "First" is red first; "plant" is red by plant. "CI" is hosted CI; "check" is the `check` job; "browser" is the `browser` job.

| Row | Owner | Red build (D section 14) | How the owner shows red | Runs on |
| --- | --- | --- | --- | --- |
| KM1 | 16 | core matches the logical key; or no store write | first (step 1); plant: match `key`, not `code` | browser + check |
| KM2 | 16 | `prevent_default_event_handling: false` (`lib.rs:254`); or no focus handling | first (step 1, no overlay); plant `lib.rs:254` | browser; P1 table headed local (task 1) |
| KM3 | 9 | double-click window 0; Shift ignored | first | check |
| SEL1 | 17 | pick radius 0; or a foreign click selects it for orders | plant x2 | browser |
| SEL2 | 13 | world rectangles; or a frame late | plant x2 (`turts_debug.frames`) | browser + check |
| SEL3 | 13 | type select ignores the screen | plant | browser |
| SEL4 | 16 | groups not stored; or dead ids kept | first (step 1); plant | browser |
| SLOT1 | 16 | today's `judge_hello`; or no retry | first (step 1); plant (revert the task 7 branch) | browser + check |
| ORD1 | 5 | no wire form for a behaviour; or id strings in server code | first; plant x2 | check |
| ORD2 | 15 | all-or-nothing `validate_move_to` (`command.rs:255-294`) | plant | check + browser |
| ORD3 | 14 | append as replace; stop keeps the queue; or no completion | plant x3 | check + browser |
| ORD4 | 14 | attack-move as move; or `choose_target` ignores the ordered target | plant x2 | check + browser |
| ORD5 | 14 | smart command resolved once for the selection | plant | browser + check |
| ORD6 | 15 | reason texts unmapped; or no rate limit | first (texts); plant x2 | browser + check |
| ORD7 | 15 | marker on `OrderResult`; or one more tick | plant x2 (`turts_debug.frames`) | browser + check |
| GM40 | 22 | spreading off | plant | check + browser |
| GMA | 22 | avoidance off; or a `Blocked` unit rests inside another | plant x2 | check |
| GMU | 22 | clamp off; NaN kept; or slots reused | plant x3 | check |
| DET1 | 22 | neighbour iteration in `HashMap` order | plant, run first (D section 14) | check (`scripts/acceptance.sh`) |
| EL1 | 22 | missing `AuditState` input; a write outside; or today's validator | plant x3 | check |
| CAM1 | 11 | the camera does not follow the core (task 9 removed the fit); or no `PointerLeft` | first; plant (ignore `PointerLeft`) | browser |
| CAM2 | 16 | no engine cursor under lock | plant | browser |
| PROJ1 | 9 | field of view not copied into `Projection` | plant | check |
| HUD1 | 12 | an engine-docked panel (`economy_ui.rs:916-934`) | first | browser + check |
| HUD2 | 17 | stale or empty source; or a tooltip with no key | first; plant x2 | browser |
| HUD3 | 3 | validator accepts; cap reads file bytes; or no version row | first | check |
| HUD4 | 12 | one cursor for all contexts | plant | browser |
| MM1 | 14 | the minimap click does not move the camera | first | browser |
| VIS1 | 13 | one material (`render.rs:403-406`); or a palette pair under 20 | first; plant | browser + check |
| VIS2 | 13 | no record | first (step 5 check command) | headed local |
| ALR1 | 15 | no derivation; or no cooldown | first; plant | check + browser |
| AUD1 | 15 | no resume hook | plant | browser |
| SH1 | 16 | scroll speed ignored; or `UiScale` times the device factor | first (step 1); plant x2 | browser |
| PER1 | 16 | no write; or a corrupt value stops the client | first (step 1); plant | browser + check |
| T1X | 8 | the core depends on Bevy; or a row deleted | first | check |
| HD1 | 10 | the headless build has no core | first (`--actions` unknown); plant (core plugin out of the headless app) | check + browser |
| DRV1 | 21 | a driver that reads a bridge | first (ledger default 0); plant | agents job |
| MOD1 | 18 | a missing version row | first; plant | check |
| REG1 | 24 | a single unit off its point; or the ordered target alters the forge stream | plant x2 | check + CI |
| PT1 | 23 | an oracle step the tester cannot reach | plant (guide page 5 removed) | person and `player` agent |
| PT2 | 23 | spreading or arrival off | plant (the GM40 plant) | person and `player` agent |
| CI1 | 24 | any red job | the hosted run | CI |

Coverage: every `in` row of `## Yardstick coverage` (81 rows, D:1488-1668) names one of 27 tests, and every one of the 27 is a row above (scratch `coverage.txt`: "tests in coverage not in acceptance: []"). By owner: task 3 HUD3 (MD08); 9 KM3 (IN03, IN04); 11 CAM1 (CA01, CA02, CA03, CA13, CA14); 12 HUD4 (PC21, UI23); 13 SEL2 (PC02, PC04, PE04), SEL3 (PC05), VIS1 (PC23, UI12, TM03, AN06, AN07); 14 MM1 (CA08, CA09, UI05, UI08), ORD3 (OR02, OR08, OR11, OR12, EC04), ORD4 (OR03, OR04, OR05, CB03), ORD5 (PC22, OR01, OR17, EC02); 15 ORD2 (PC17, OR24), ORD6 (OR22, OR31), ORD7 (OR23, PF05, PE01, PE07), ALR1 (UI16, AU02), AUD1 (MD09, AU01, AU06); 16 KM1 (IN02), KM2 (IN08), SEL4 (PC08, PC09, PC10, PC19), CAM2 (AC05), SH1 (CA16, UI22); 17 SEL1 (IN01, PC01, PC16, PC20), HUD2 (IN05, UI01, UI02, UI03, UI04, UI09, UI10, UI11, PR10); 18 MOD1 (MD01); 22 GM40 (OR25, GM01, GM09, PF01), GMA (GM04, GM06), GMU (GM10, PF04, MP09).

Player path steps with no acceptance row (gate 2 ruling 22; D:1677-1712): step 2 (the guide opens at the first join; the camera on the factory) and step 3 (F1 and the guide button close it): task 12 builds `GuideOpen` and the `toggle_guide` action, task 17 step 2 checks them on forge. Step 9 ("Shell Turret: Cannot move" and the error sound): task 15 builds the local notice, task 17 step 2 checks the forge text. Step 18 (the minimap ping): task 14 builds the ping, task 14 step 2 checks it. Step 14 (swap or cancel, and Reset): task 16 step 6.

---

## Wave 1

### Task 1: Probe P1 (keys, pointer lock, audio resume)

**Seat and model:** `.claude/agents/web-platform-engineer.md`, Luna. Review: `ux-designer` (Sol) reads the table and the K11 rule.

**Branch / needs / wave:** `codex/mc-t1`; none; wave 1. **Design:** sections 3.2, 3.3, 3.4, 9.5, 14 (CAM2 Esc clause); K9, K11. **Owns rows:** none. Task 16 attaches the P1 table to the browser-capture row; tasks 3 and 9 use its K11 result.

**Files (create):** `docs/research/mc/p1-probe/page.html` (logs each `keydown`: `code`, `key`, modifiers, `cancelable`, `document.hasFocus()`, pointer lock state, `AudioContext.state`; it calls `preventDefault()` on every `keydown` and `contextmenu`, as winit does with `prevent_default_event_handling: true`, `crates/turts_client/src/lib.rs:254`); `docs/research/mc/p1-probe/server.py` (standard library; serves the page and appends each POSTed record to `p1-log.jsonl`); `docs/research/mc/p1-probe/run.sh` (private Xvfb display, a new headed browser for each key group, `xdotool` keys); `docs/research/mc/p1-probe/p1-log.jsonl`; `docs/research/mc/p1-keys.md` (the table); `docs/research/mc/p1-probe/SHA256SUMS`.

**Probe facts:** P1 runs on the dev machine, not on hosted CI or GCP (ruling O2). `/usr/bin/google-chrome`, `/usr/bin/firefox`, `/usr/bin/xdotool` and `/usr/bin/Xvfb` exist. `firefox --version` did not return in 120 s in the plan run, and the snap gave "sanity timeout expired" in the player run (gate 2 player 3). CDP input bypasses browser accelerators, so the probe never uses CDP. Safari stays NOT TRACED. The table carries the column `default prevented` (D section 3.4, `event.defaultPrevented` read after the handler) next to `cancelable`. The page calls `preventDefault()` itself, so the column is always true (gate 2 qa); it is recorded, not judged. The K11 rule of step 4 judges `reaches the page` and `browser acted` only (ruling O9).

- [ ] **Step 1: Red (the instrument).** Run Chrome first. `bash docs/research/mc/p1-probe/run.sh chrome control; echo exit=$?` sends Ctrl+T and Ctrl+W, each in a new browser. Expected: for each, "browser acted: yes" (a new tab or a closed window, read with `xdotool search --name`), `exit=0`. A control row with "browser acted: no" means the probe cannot see a browser action: fix the probe.
- [ ] **Step 2: Green (Chrome table).** Send every row of D table 3.3, Ctrl+Digit1 to Ctrl+Digit9, Shift+Digit1 to Shift+Digit9, Digit1 to Digit9, F1 to F12, Space, Tab, Esc, and Esc while the pointer is locked (a click calls `requestPointerLock()` first; record whether the first Esc ends the lock: this is the CAM2 Esc clause, gate 2 ruling 18). Record `AudioContext.state` before the first click (`suspended`) and after `resume()` in the click handler (`running`). Write `p1-keys.md`: one row per key and browser, columns `key`, `browser`, `version` (`google-chrome --version`), `reaches the page`, `cancelable`, `default prevented`, `browser acted`.
- [ ] **Step 3: Firefox.** Run step 1 and step 2 with `firefox`. If Firefox does not start under the agent sandbox, try it outside the sandbox (the snap can fail only inside it). Record the version from `navigator.userAgent`. If Firefox does not start after 10 minutes of effort, mark every Firefox row `NOT TRACED` and go to step 4 (gate 2 ruling 15).
- [ ] **Step 4: The K11 rule.** K11 (a) holds only when, in each browser with rows, every Ctrl+Digit1..9, Shift+Digit1..9, Digit1..9, F1, F10, Space and Tab row has "reaches the page: yes" and "browser acted: no". Write the first line of `p1-keys.md`: `K11 (a) holds: yes`, or `K11 (a) holds: no; rows: <list>`, or `Firefox NOT TRACED`. On "no" or "Firefox NOT TRACED": **stop** and send the table to the orchestrator; the CEO picks K11 with it (with the Chrome table only when Firefox is NOT TRACED). Task 9 step 3 waits for the pick.
- [ ] **Step 5: Commit.** `cd docs/research/mc/p1-probe && sha256sum page.html server.py run.sh p1-log.jsonl ../p1-keys.md > SHA256SUMS && sha256sum -c SHA256SUMS; echo exit=$?` prints `exit=0`. Commit the six files.
- [ ] **Step 6: Gate.** G-QA. **Runs: 3.**

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

2b: `PlayerCommand { seq: u32, order: OrderEnvelope }`; `Welcome { slot_id, tick, tick_hz, match_id: u64 }`; `Snapshot.unit_orders` and `WorldDelta.unit_orders: Vec<UnitOrderRecord>`; `crates/turts_server/src/orders.rs`: `pub fn to_sim(slot: turts_sim::SlotId, command: &PlayerCommand, catalog: &turts_mod_api::ContentCatalog) -> Result<Vec<turts_sim::Command>, OrderResult>` and `pub fn to_wire(results: &[turts_sim::CommandResult], command: &PlayerCommand) -> OrderResult` (external crate paths only). `ContentRef` is `pub type ContentRef = String;` at `crates/turts_net_protocol/src/economy.rs:16`.

**2a files:** Modify `crates/turts_net_protocol/src/messages.rs` (new types; append the eight reasons `unknown_order`, `not_applicable`, `target_invalid`, `target_gone`, `out_of_range`, `rate_limited`, `not_queueable`, `blocked` to `CommandRejectReason` at `:92-122`, to `ALL` at `:124` and to `as_str` at `:155`), `crates/turts_net_protocol/tests/reasons_m3.rs` (the `ALL` list). Create `crates/turts_net_protocol/tests/wire4.rs`.

- [ ] **2a Step 1: Red.** Add the types and the eight variants; let `as_str` return `"internal"` for the eight. Write `wire4.rs`: `every_order_target_round_trips` (postcard round trip of each of the six `OrderTarget` values in an `OrderEnvelope`), `order_result_round_trips_with_refusals`, `new_reason_names_are_stable`. Run `cargo test -p turts_net_protocol --test wire4 --locked; echo exit=$?`. Expected: `new_reason_names_are_stable` fails with ``assertion `left == right` failed``, `left: "internal"`, `right: "unknown_order"`; `exit=101`.
- [ ] **2a Step 2: Green.** Give the eight names. Re-run: `exit=0`.
- [ ] **2a Step 3: Commit, gate.** Commit. G-RUST, G-QA. **Runs: 2.**

**2b files:** Modify `crates/turts_net_protocol/src/lib.rs:13` (4), `messages.rs` (`PlayerCommand`, `Welcome`, `Snapshot`, `WorldDelta`; remove `PlayerCommandKind`, `CommandApplied`, `CommandRejected`), `crates/turts_net_protocol/tests/wire.rs`, `git mv crates/turts_net_protocol/tests/protocol3.rs crates/turts_net_protocol/tests/protocol4.rs`; `crates/turts_net_lightyear/src/protocol_plugin.rs:36-43` (register `OrderResult`, drop `CommandApplied` and `CommandRejected`), `src/channels.rs`, `src/bin/spike_client.rs`, `src/bin/spike_server.rs`, `tests/registers.rs`; create `crates/turts_server/src/orders.rs`; modify `crates/turts_server/src/main.rs` (`mod orders;`), `session.rs` (`admit_kind`, `:339-419`, calls `orders::to_sim`; `Welcome.match_id`), `outbound.rs:17-54` (`send_command_results` sends `OrderResult`), `export.rs` (owner-only `unit_orders`, filtered as `:87-93`; empty until task 4), `tests/hello.rs`, `tests/support/wire_sender.rs`, `tests/acceptance.rs`, `tests/economy.rs`, `tests/m3_process.rs`; `crates/turts_client/src/commands.rs`, `ingest.rs` (`OrderResult`, `match_id`), `join.rs:356-410` (`--move` and `--move-unit` send an envelope with the first `OrderDef` whose behaviour is `Move`), `input.rs:131-141` (right click sends that envelope), `economy_ui.rs` (each `UiAction` sends an envelope with the `OrderDef` of its behaviour), `debug.rs:103,175,193-200,258,284`, `options.rs:26,119` (help text), `lib.rs:295-362` (test); `crates/turts_client_web/src/lib.rs:190-210,670` (`turts_move` sends a `Move` envelope); the client tests that use `PlayerCommandKind` (`tests/commands.rs`, `tests/economy_ui.rs`, `tests/scenario.rs`); `tests/m2/refutations.json` (E8-rejection-ack-send-reversal: new `expected_failure` text for `OrderResult`).

**2b rules:**
- Before task 4, `to_sim` maps by `OrderDef.behaviour` from the package catalog (task 3): `Move` to `CommandKind::MoveTo`, `Gather` to one `CommandKind::Gather` per unit, `Produce` with `Content(recipe or technology)`, `CancelProduction` with `Job`, `Remove`, `SetEnabled(bool)`. `Stop`, `Hold`, `Attack` give `not_applicable` for each unit until tasks 4 and 5. An unknown order id gives `unknown_order` for each unit. `to_wire` copies one command outcome to every unit of the envelope until task 4.
- The server code holds no order id string (ORD1).
- `debug.rs` keeps the key `command_results`. Each entry keeps `seq`, `applied_tick` when `accepted` is not empty, `rejected_tick` and `rejected` when every unit is refused, and adds `accepted` and `refused`. So the legacy `tests/browser/t7.js` stays green until task 21 (QA0D:424-425).
- `match_id`: the server draws one `u64` at process start (a `std::collections::hash_map::RandomState` hash of the process id and the start time) and writes it in `--stats`. Plan choice; D gives no source.

- [ ] **2b Step 1: Trace (NOT TRACED item 1).** Read the lightyear 0.31.0 source for the largest message on the reliable channel that `PlayerCommand` uses (`crates/turts_net_lightyear/src/channels.rs`). Record `path:line` in the work log. An envelope of 4,096 units is at most 4,096 x 5 bytes of postcard varints plus the header. If the limit is below that, stop and report (K4 b).
- [ ] **2b Step 2: Red.** Add `protocol_3_hello_is_refused` in `crates/turts_server/src/session.rs` (`mod tests`): `judge_hello` with `protocol_version: 3` returns `Err("protocol version mismatch")`. Run `cargo test -p turts_server --bin turts_server --locked session::tests::protocol_3_hello_is_refused; echo exit=$?` while the version is 3. Expected: ``called `Result::unwrap_err()` on an `Ok` value``, `exit=101`. Add `move_hook_sends_the_first_move_order` in `crates/turts_client/tests/commands.rs`: the `--move 20,8` hook on `acme` queues an envelope with `order: "acme:order_move"` (task 18 table), `target: Point(20, 8)`, `queue: Replace`. Expected red: the hook still queues `MoveTo`.
- [ ] **2b Step 3: Green.** Make the switch. Re-run both tests: `exit=0`. Run `cargo test -p turts_net_protocol -p turts_net_lightyear -p turts_server -p turts_client --locked; echo exit=$?`: `exit=0`.
- [ ] **2b Step 4: Commit, gate.** Commit. G-RUST, G-QA, G-ACC, G-WEB, G-FROZEN, G-REF (browser cases per the G-REF rule). **Runs: 4.**

### Task 3: Mod API 0.2.3

**Seat and model:** `.claude/agents/mod-runtime-engineer.md`, Luna. Review: `mod-builder` (Sol): can a modder fix each error from its text alone (MD18)?

**Branch / needs / wave:** `codex/mc-t3`; none; wave 1. **Design:** sections 5.2, 8.2 to 8.4, 11.1 to 11.4, 3.2 (reserved keys); K2, K5 (R1), K12, K13, K18, K24. **Owns rows:** HUD3.

**Interfaces (produces):** in `turts_mod_api`: `OrderDef`, `Behaviour` (`Move | Stop | Hold | Attack | Gather | Produce | CancelProduction | Remove | SetEnabled(bool)`), `TargetKind`, `FirePolicy`, `EngagePolicy`, `OrderRulesDef` (`queue_depth: u8`, `smart`), `SmartTarget`, `Capability`, `InputMapDef`, `Binding`, `Mod`, `Button`, `AssetsDef`, `InterfaceDef`, `Widget` (the ten primitives of D section 8.2), `Placement` (`at`, `size`, `layer`, `visible`), `Source`, `Value`, `Predicate`, `CameraDef`, `LightDef`, `WorldStyle`, `AlertDef`, `AlertTrigger`, `UnitPresentation`, `GuidePage`, `RESERVED_BROWSER_KEYS` (the rows of D table 3.2; task 9 adds the P1 rows), `KNOWN_KEY_CODES`, `DEFAULT_QUEUE_DEPTH = 16`. In `turts_mod_api`: `ContentCatalog.orders: BTreeMap<ContentId, OrderDef>` (`ContentCatalog` is at `crates/turts_mod_api/src/content.rs:156`; re-check 1 rigour N5). In `turts_mod_runtime`: `PackageArchive::{orders, order_rules, input_maps, interfaces, assets}`; `check_locked_set_caps(&[&PackageArchive]) -> Result<(), ModError>`. The shapes are those of D section 11.2, field for field.

**Files:** Modify `crates/turts_mod_api/src/schema.rs:53-65` (four lists), `crates/turts_mod_api/src/content.rs:156` (`ContentCatalog.orders`), `crates/turts_mod_api/src/lib.rs:33,36` and re-exports; create `crates/turts_mod_api/src/orders.rs`, `input.rs`, `interface.rs`, `assets.rs`. Modify `crates/turts_mod_runtime/src/archive.rs` (accessors; `archive.rs:56` `file()` gives the bytes of any packed file, and `pack.rs:60-87` packs every regular file), `content.rs` (calls the new validation; E10 anchor, G-REF), `version_table.rs` (rows, `present` signature), `content_m3_validation.rs:380-383` (remove "weapons must be static", R1); create `crates/turts_mod_runtime/src/content_mc_validation.rs`, `asset_headers.rs` (PNG IHDR width and height; TTF `00 01 00 00` or OTF `OTTO` magic; OGG Vorbis identification header rate and last-page granule). Modify `crates/turts_pack/src/main.rs` (K18 warnings on stderr, exit 0), `crates/turts_checks/tests/support/mod.rs:88-95,118` (the four lists in the export set, so `stray_ron` accepts the files; tokens for `OrderDef.id`, mod action names, asset ids; widget ids are not tokens: they are layout names, not gameplay content). Tests: create `crates/turts_mod_runtime/tests/mc_content.rs`, `crates/turts_pack/tests/fixtures/hud3/` (a one-unit Game package with orders, input, interface and assets; `make_assets.py` writes header-valid PNG, TTF and OGG files with the standard library); modify `crates/turts_pack/tests/bad_packages.rs` (HUD3 cases through its exact-stderr helper), `crates/turts_mod_runtime/tests/content_m3.rs:339` (`weapon_must_be_static` becomes `mobile_weapon_is_accepted`), `crates/turts_mod_runtime/tests/t2c_package_rejects.rs:57` (the message names `0.2.3`, gate 2 td 3).

- [ ] **Step 1: Red (HUD3).** Signature first: add the types, the lists and the accessors with a validator that returns `Ok(())`. Write one `bad_packages.rs` case per HUD3 item of D section 14: unknown widget kind; unknown source; duplicate widget id; `card` collision; missing asset id; PNG of 2049 x 1 px; cursor of 129 x 129 px; IHDR sum above 256 MiB in one package (17 images of 2048 x 2048, 272 MiB, header-only files); the same sum across two packages given to `check_locked_set_caps` (a real two-package locked set is refused with LR3 until MP2, `CLAUDE.md` `turts-pack` entry); "65 images of 2048 x 2048 with 19,771-byte files" (file bytes 1.3 MB, decoded 1.0 GiB: the red of "the cap reads file bytes"); missing font file; fonts above 8 MiB; a sound of 31 s; sound files above 32 MiB; a reserved default binding (Ctrl+KeyW); `CancelProduction` with `target: Point`; text `size: 10` (warning, exit 0); `engine_api: "^0.2"` with `exports.interface`. Each case asserts exit 2 and the exact stderr `<package>: <file>: <field>: <text>`, or exit 0 and the warning line. Version rows: unit tests in `version_table.rs`, each row refuses `^0.2.2` with its list present and accepts `^0.2.3`. K5: invert `content_m3.rs:339`. Run `cargo test -p turts_pack --test bad_packages --locked; echo exit=$?` and `cargo test -p turts_mod_runtime --locked; echo exit=$?`. Expected: each new case fails, `exit=101`.
- [ ] **Step 2: Green.** Implement `content_mc_validation.rs` (D section 11.3, in that order), `asset_headers.rs`, the four rows, the lift (keep "a unit with health requires removal", `content_m3_validation.rs:375`, and "a unit with a queue cannot be removed", `:427`), the token derivation, the 0.2.3 bump. Add a fixture test in `crates/turts_checks/tests/no_content.rs`: a package with `orders/rush.ron` (`id: "rush"`) gives a hit for a source line `let s = "rush";`. Re-run step 1, then `cargo test -p turts_checks --test no_content --locked; echo exit=$?`: `exit=0`.
- [ ] **Step 3: Commit, gate.** Commit. G-RUST, G-QA, G-FROZEN, G-REF. **Runs: 6.**

### Task 20: Harness, `io`, the M2 red trace

**Seat and model:** `.claude/agents/qa-lead.md`, Luna. Review: `web-platform-engineer` (Sol).

**Branch / needs / wave:** `codex/mc-t20`; none; wave 1. **Design:** sections 13.1, 13.2 (the `view` side), 13.4; QA0D sections 4.1 to 4.4. **Owns rows:** none. It gives `io`, `view`, `scripts/mc-browser.sh` and `scripts/mc-pixels.sh` to tasks 10 to 23.

**Interfaces (produces):** a driver sets `exports.drive = async (io, params) => ...`. `io` is a frozen object with exactly the keys `click`, `drag`, `key`, `move`, `wheel`, `screenshot`, `save`, `wait`, `reload`, `use`: `io.click(x, y, button)`, `io.drag(x0, y0, x1, y1, button)`, `io.key(code, modifiers, opts)` (`opts.key` gives another logical key; the harness uses it only in a CDP `Input.dispatchKeyEvent` with a string `key`), `io.move(x, y)`, `io.wheel(dy)`, `io.screenshot()`, `io.save(name, bytes)`, `io.wait(ms)`, `io.reload()`, `io.use(n)` (window n of `--windows N`). **Input sequence (design revision 9, 13.2):** the harness numbers each DOM-level event that an `io` call sends (a click is a down and an up; a drag is a down, the moves and an up), from 1 in each window. Before it sends event n, the harness sets the page global `window.__turts_io_seq = n` with its own `page.evaluate` (the driver never sees it). The input log records each `io` call with the sequence numbers of its events. `params` is a frozen JSON object that the harness reads from `--params <file>`; `scripts/mc-browser.sh` builds it. From task 21, `params.bindings` is the effective input map (engine defaults merged with the package `input/bindings.ron`), printed by the example of task 9 (re-check 1 player D). `view.bridge(name)` (refuses `turts_move`, QA0D:333), `view.inputLog()`, `view.screenshot()`, `view.serverFile(path)`. Row file `tests/browser/rows/<ROW>.json`: `{"row": "GM40", "package": "mods/muster", "driver": "gm40-driver.js", "oracle": "gm40-oracle.js", "viewports": ["1280x720"], "windows": 1, "runs": "ci"}` (`runs`: `ci` or `headed`).

**Files:** Create `tests/browser/harness/run.js`, `tests/browser/harness/driver-io.js`, `tests/browser/harness/view.js` (role `harness`); `tests/browser/harness/io-contract.test.js` (role `unit`); `scripts/mc-browser.sh` (packs the package copy, starts `turts_server` and a static server as `scripts/t7.sh:22-82` does, builds `params`, runs `run.js`; `--serve-only` prints the page URL and waits, for the person of task 23); `scripts/mc-pixels.sh` (runs each row file twice; a row passes only when both runs pass, QA0D:441-443; `"runs": "headed"` prints `SKIP <row> headed`, unless `MC_PIXELS_HEADED=1` is set on a machine with a display); `tests/qa/driver/harness-ok-driver.js`, `harness-ok-oracle.js`, `harness-fail-oracle.js`; modify `scripts/qa/driver_rule.py:12` (add `harness` to `ROLES`) and `tests/qa/driver/cases.tsv` (a `harness` case).

- [ ] **Step 1: Trace the M2 red (gate 2 ruling 8).** On a clean tree run `bash scripts/m2-browser.sh; echo exit=$?` locally and read `target/m2-browser/runs/*/browser.log`. Record the cause with `path:line` in the work log and in the "Known red window" row of the board. If the cause is a harness or tool defect (Trunk, Chrome flags, timeouts, `m2-browser.sh`, `m2.js` waiting logic), fix it in this task and show `exit=0`. Else name the owner task in the board row and stop this step.
- [ ] **Step 2: Red.** Give `run.js` the line `// turts-test-role: harness`. Run `python3 scripts/qa/driver_rule.py tests/browser; echo exit=$?`. Expected: a `RED DR1` line for `tests/browser/harness/run.js` (unknown role), `exit=1`. Write `io-contract.test.js`: `Object.isFrozen(io)`; `Object.keys(io).sort()` equals the ten keys; no property of `io` or of any `io` function gives `page`, `evaluate`, `context` or `browser`; a spy on the CDP session sees `opts.key` only in `Input.dispatchKeyEvent`. Run `bash scripts/mc-browser.sh mods/acme tests/qa/driver/harness-ok-driver.js tests/qa/driver/harness-fail-oracle.js 1280x720; echo exit=$?`. Expected: `FAIL harness-fail-oracle.js: <reason>`, `exit=1`.
- [ ] **Step 3: Green.** Add `harness` to `ROLES` and the case. Implement the harness. Re-run the driver-rule commands: `exit=0`. `node --test tests/browser/harness/io-contract.test.js; echo exit=$?`: `exit=0`. The pass case with `harness-ok-oracle.js` prints `PASS`, `exit=0`; it asserts that `view.inputLog()` holds each `io` call with its arguments, its wall time and its event sequence numbers (strictly increasing, no gap), and that `view.bridge("turts_move")` throws. `io-contract.test.js` also asserts that `__turts_io_seq` is not reachable from `io`.
- [ ] **Step 4: Commit, gate.** Commit. G-QA, G-WEB.
- [ ] **Step 5: Plant.** Add `page` as an `io` property in `driver-io.js`; `io-contract.test.js` fails (key list and `page` reachable); restore. **Runs: 3.**

## Wave 2

### Task 8: `turts_interaction` skeleton, contract types, T1 rows, T1X

**Seat and model:** `.claude/agents/technical-director.md`, Luna. Review: `qa-lead` (Sol): can each T1X case go red?

**Branch / needs / wave:** `codex/mc-t8`; needs 2a, 3; wave 2. **Design:** sections 2.1, 2.2 (revision 7: `Minimap { button, local }`, `over_widget`), 2.4, 13.6; K1, K26. **Owns rows:** T1X.

**Interfaces (produces, `turts_interaction`):** `Core::step(&mut self, events: &[InputEvent], world: &dyn WorldView, view: &Viewport, now_ms: u64) -> Vec<Effect>`; `InputEvent` with the variants of D section 2.2 rev 7 (`PointerMove`, `PointerDown`, `PointerUp` each carry `over_widget: bool`; `Minimap { button, local: [f32; 2] }` with `local` in 0..1); `WorldView`, `Viewport`, `Effect`, `CameraState` (look-at point, height, pitch, yaw, vertical field of view). The stub `step` returns `Vec::new()`. Stub files `camera.rs` (task 11 fills) and `minimap.rs` (task 14 fills). Use `glam::camera::rh::proj::directx::perspective_infinite_reverse`, not the deprecated `Mat4::perspective_infinite_reverse_rh` (gate 2 bevy F8 a; strict clippy refuses the deprecated call).

**Files:** Create `crates/turts_interaction/Cargo.toml` (`turts_mod_api`, `turts_net_protocol`, `glam = "=0.33.12"`), `src/lib.rs`, `src/input_map.rs`, `src/selection.rs`, `src/groups.rs`, `src/smart.rs`, `src/compose.rs`, `src/camera.rs`, `src/minimap.rs`, `src/projection.rs`. Modify `Cargo.lock`, `scripts/boundary.sh` (after line 37), `crates/turts_checks/tests/boundary.rs:92-95` (four `checked:` lines) and its T1X tests.

- [ ] **Step 1: Red.** Add the crate. `cargo tree -p turts_interaction -e normal --prefix none --locked > /tmp/mc-tree.txt; echo exit=$?` prints `exit=0`; `grep -c "^glam v0.33.12" /tmp/mc-tree.txt` prints `1`; `grep -c "^glam v0.34" /tmp/mc-tree.txt` prints `0`. Write `t1x_copy_with_bevy_render_is_red`: it copies `crates/turts_interaction` and the root `Cargo.lock` to a temporary directory (so `cargo tree` resolves offline, gate 2 td 8), writes its own `[workspace]` table, explicit package fields, absolute path dependencies and `bevy_render = "=0.20.0"`, and runs `bash scripts/boundary.sh --rule no-bevy-prefix --crate turts_interaction --manifest <copy>/Cargo.toml`. It asserts exit 1 and the exact stderr line `forbidden edge: turts_interaction -> bevy_app` (D section 13.6). Write `t1x_full_run_prints_four_lines` and `t1x_real_crate_wasm32_is_red_for_mod_api` (`--rule forbid --crate turts_interaction --target wasm32-unknown-unknown --names turts_mod_api`, expects `forbidden edge: turts_interaction -> turts_mod_api`, as `boundary.rs:105-122`). Run `cargo test -p turts_checks --test boundary --locked; echo exit=$?`. Expected: `t1x_full_run_prints_four_lines` fails with `missing "checked: turts_interaction x86_64-unknown-linux-gnu: no bevy*"`, `exit=101`.
- [ ] **Step 2: Green.** Add the four rows of D section 2.4. Re-run: `exit=0`. `bash scripts/boundary.sh` prints `checked: turts_interaction <host>: no bevy*`, `checked: turts_interaction wasm32-unknown-unknown: no bevy*`, `checked: turts_interaction <host>: no turts_sim wgpu winit`, `checked: turts_interaction wasm32-unknown-unknown: no turts_sim wgpu winit`.
- [ ] **Step 3: Commit, gate.** Commit. G-RUST, G-QA.
- [ ] **Step 4: Plant.** Delete one new row from `scripts/boundary.sh`; `t1x_full_run_prints_four_lines` fails; restore. **Runs: 2.**

### Task 18: Orders for the five packages (K3 b, R4); the content id table

**Seat and model:** `.claude/agents/mod-builder.md`, Luna. Review: `mod-runtime-engineer` (Sol).

**Branch / needs / wave:** `codex/mc-t18`; needs 3; wave 2. **Design:** sections 11.4, 12.1 (orders part), 12.2; K3 b, K13. **Owns rows:** MOD1.

**Content ids (gate 2 ruling 2).** Hits are from the real `scan_rust` over `crates/` at `d21c9ee` (scratch probe `scanprobe`: a scratch package whose unit ids are the candidates, `derive_tokens` and `scan_rust` from `crates/turts_checks/tests/support/mod.rs`; exit 0). Ids with 0 hits today also take the prefix, because MC adds engine names: the engine action `cancel` (D table 3.3) and the cursor contexts `move`, `attack`, `gather` (D section 8.2).

| Package | Kind | Old id | Hits | New id | Hits |
| --- | --- | --- | --- | --- | --- |
| forge, muster, acme, base, cedar, haul | order | `move` | 0 | `order_move` | 0 |
| forge, muster, M2 packages that need it | order | `stop` | 0 | `order_stop` | 0 |
| forge, muster | order | `hold` | 0 | `order_hold` | 0 |
| forge, muster | order | `research` | 115 (first `crates/turts_client/src/debug.rs:98`) | `order_research` | 0 |
| forge, muster, base | order | `cancel` | 0 | `order_cancel` | 0 |
| forge, muster | order | `enable` | 2 (`crates/turts_server/src/main.rs:94`) | `order_enable` | 0 |
| forge, muster | order | `disable` | 5 (`crates/turts_sim/src/command_m3.rs:83`) | `order_disable` | 0 |
| forge, muster | order | `remove` | 37 (`crates/turts_client/src/economy_ui.rs:1002`) | `order_remove` | 0 |
| base (and any package with a producer) | order | `produce` | 6 (`crates/turts_client/src/options.rs:33`) | `order_produce` | 0 |
| muster, base, cedar, haul | order | `gather` | 17 (`crates/turts_server/src/export.rs:11`) | `order_gather` | 0 |
| muster | order | `attack` | 0 | `order_attack` | 0 |
| muster | order | `rush` | 0 | `rush` (design revision 6) | 0 |
| muster | unit | `marker` | 6 (`crates/turts_client/src/live_view.rs:268`) | `beacon` | 0 |
| every package | input action | one per order hotkey | - | `key_<verb>` (for example `key_move`) | 0 |

Unit ids `trooper`, `worker`, `depot`, `post`, `dummy` have 0 hits. `collect` (256), `build` (8), `bindings` (10) and `assets` (12) have hits: no package may use them as ids. Rule (also for tasks 17 and 19): after each package change run `cargo test -p turts_checks --test no_content --locked real_source_scan_is_clean; echo exit=$?`; on a hit, rename the local id and add the row to this table in the work log.

**Files:** For each of `mods/acme`, `mods/base`, `mods/cedar`, `mods/haul`: `manifest.ron` (`engine_api: "^0.2.3"`, version `0.2.0` to `0.3.0`), `orders/*.ron` (the orders its units can use, ids from the table), `orders/rules.ron`, `input/bindings.ron`, `interface/hud.ron` (plain text buttons, no images; D section 12.2). For `mods/forge`: `manifest.ron` (`engine_api: "^0.2.3"`, `0.1.0` to `0.2.0`), `orders/*.ron` (D section 12.1 set, ids from the table), `orders/rules.ron`, `input/bindings.ron` (M, S, H, D, E, R and the production keys, D section 3.3). Maps, units, recipes and technologies do not change. Modify `crates/turts_mod_runtime/tests/t2c_package_rejects.rs:50-66` (the anchor `engine_api: \"^0.2\"` becomes `engine_api: \"^0.2.3\"`, gate 2 td 3) and `crates/turts_server/tests/economy.rs:3180-3192` (`e10_cedar_uses_public_pack_and_boot`: the same anchor). Create `crates/turts_pack/tests/mc_packages.rs`.

- [ ] **Step 1: Red.** Write `mc_packages.rs`: `five_packages_load_with_0_2_3` (each loads with `^0.2.3` and has at least one `OrderDef` with behaviour `Move` when a unit has movement), `old_acme_copy_loads_with_zero_orders` (a 0.2.2 copy of `acme` with no orders loads, and its catalog has 0 `OrderDef`; the client notice is task 15, gate 2 ruling 11), `each_version_row_refuses_0_2_2` (four copies, one list each, `^0.2.2`, exit 2 with the row name), `a_new_order_binds_a_behaviour_by_data` (a copy with `orders/dash.ron` bound to `Move` loads). Run `cargo test -p turts_pack --test mc_packages --locked; echo exit=$?`. Expected: `five_packages_load_with_0_2_3` fails (no orders yet), `exit=101`.
- [ ] **Step 2: Green.** Write the files. Re-run: `exit=0`. `cargo run -q -p turts_pack -- <scratch copy of each package>; echo exit=$?`: archive and hash, `exit=0`. `real_source_scan_is_clean`: `exit=0`.
- [ ] **Step 3: Commit, gate.** Commit. G-RUST, G-QA, G-FROZEN (the stream test loads `base`, `haul`, `acme`, `cedar`, `m3_m2_stream.rs:16`; the hash must not change), G-ACC.
- [ ] **Step 4: Plant (MOD1).** Delete the `exports.input` row from `version_table.rs`; `each_version_row_refuses_0_2_2` fails; restore. **Runs: 2.**

### Task 19: `muster` and the HUD1 fixture

**Seat and model:** `.claude/agents/mod-builder.md`, Luna. Review: `ux-designer` (Sol).

**Branch / needs / wave:** `codex/mc-t19`; needs 3 (R1); wave 2. **Design:** sections 7 (camera start), 8.5, 12.3 (revision 6: `dummy`, `rush`); K15, K24. **Owns rows:** none (content for many rows).

**Names:** the ids of the task 18 table, including `beacon` for the far block and `dummy` for slot 1. Step 2 runs the scan as the proof (ruling O1).

**Files:** Create `mods/muster/` (`manifest.ron`, `maps/field.ron` 64 x 48 with one ore node, `units/*.ron`, `visuals/*.ron`, `resources/ore.ron`, `nodes/*.ron`, `orders/*.ron` (the forge set plus `order_attack`, `order_gather` and `rush`, which binds `Attack`, D section 5.2), `orders/rules.ron`, `input/bindings.ron`, `interface/hud.ron`, `assets/assets.ron` and the asset files). `--synthetic-units` adds units of the first registered unit type with movement (`crates/turts_server/src/synthetic.rs:39-50`). The registration order is NOT TRACED, so `manifest.ron` lists `units/beacon.ron` first and the test `beacon_is_the_first_mobile_unit_type` checks it (VIS2). Create `crates/turts_pack/tests/fixtures/muster_hud1/` (a copy of `muster` with the tree of D section 8.5) and `crates/turts_pack/tests/muster.rs`.

**Placement facts:** slot 0: 40 troopers (radius 0.5, speed 3.0, weapon range 4, damage 5, cooldown 1 s, health 50, removal), 2 workers (gatherer), 1 depot, 1 post (static, radius 2), 460 beacons in a far block. One screen box holds the 460 beacons and the 40 troopers and no other unit (SEL2: 500 rings). Slot 1: 10 `dummy` units (mobile, health 40, no weapon), 1 post. Slot 0 has an ore balance. `interface/hud.ron` has the alert `OwnUnitDamaged` with a minimap ping for slot 1 (MM1, task 14). Total 515 units. `CameraDef.start: Point` on the troopers, so the troopers and the own `post` fit in one screen box at 1280 x 720 (ORD2 pixels half).

- [ ] **Step 1: Red.** `muster.rs`: `muster_loads_with_515_units`, `fixture_hud1_loads`, `beacon_is_the_first_mobile_unit_type`. Run `cargo test -p turts_pack --test muster --locked; echo exit=$?`: fails (no package), `exit=101`.
- [ ] **Step 2: Green.** Write the package and the fixture. Re-run: `exit=0`. `cargo test -p turts_checks --test no_content --locked; echo exit=$?`: `exit=0` (the proof for the names; a hit stops the task).
- [ ] **Step 3: Commit, gate.** Commit. G-RUST, G-QA. **Runs: 3.**

## Wave 3

Task 2b (in the task 2 section).

## Wave 4

### Task 4: `SimSet::Orders`, queue, behaviours, audited writes, validator (K19 a)

**Seat and model:** `.claude/agents/simulation-core-engineer.md`, Luna. Review: `qa-lead` (Sol): which build makes each test red?

**Branch / needs / wave:** `codex/mc-t4`; needs 2b, 3; wave 4. **Design:** sections 4.5, 5.2 (server check order), 5.3, 6.1; K4 b, K19 a, K24. **Owns rows:** none (unit tests for ORD2, ORD3, EL1).

**Interfaces (produces):** `SimSet::Orders`, first in the `FixedUpdate` chain (`crates/turts_sim/src/plugin.rs:88-101`). Component `UnitOrders { current: Option<Order>, queue: VecDeque<Order> }` on each unit with `movement` or `weapon`, inserted at spawn (`crates/turts_sim/src/bootstrap.rs`). `CommandKind::Order(OrderCommand { order: ContentId, behaviour: Behaviour, units: Vec<u32>, target: OrderTargetSim, queue: QueueMode, group: u32 })`. `CommandResult` gains `units: UnitOutcomes { accepted: Vec<u32>, refused: Vec<(u32, RejectReason)> }`; the sim `RejectReason` (`crates/turts_sim/src/command.rs:79-112`) gets the eight new reasons, and `wire_reason` (`crates/turts_server/src/outbound.rs:55-90`) and `export.rs:431` map them. `OperationAudit` gains `phase: AuditPhase` (`Command`, `Orders`, `Other`; `#[serde(default)]` = `Other`). Pure function `turts_sim::movement::step(&MovementInput, dt: f32) -> MovementOutput`, called by `integrate_movement` and by the validator. `MAX_UNITS_PER_COMMAND` 256 to 4,096 (`command.rs:19`). `orders::to_sim` sends every envelope as `CommandKind::Order` (gate 2 ruling 6).

**Tick schedule (gate 2 sim 5):** a unit lands in `Movement` of tick t; `Orders` of tick t+1 sees the completion, pops the queue and starts the next order in that phase. This is "the next queued order starts in the same tick" of D section 4.5, read as the tick in which `Orders` sees the completion.

**Files:** Modify `crates/turts_sim/src/plugin.rs:31-42,88-101`, `command.rs` (per-unit validation in the order of D section 5.2; `apply_move_to` `:212-253` stays the plain-move path: `MoveTarget` only), `movement.rs:12-58` (calls `step`; same float step for a unit with no neighbour in reach, `:44-48`), `bootstrap.rs` (`UnitOrders` at spawn), `economy/views.rs:300` (`AuditState` fields with `#[serde(default, skip_serializing_if = ...)]`), `economy/transaction.rs` (phase mark), `economy/gather.rs` (gather order and stop); create `crates/turts_sim/src/orders.rs`; modify `crates/turts_server/src/orders.rs` (`Stop`, `Hold`; per-unit `to_wire`; `Order` for every envelope), `outbound.rs:55-90`, `export.rs` (fill `unit_orders`; `:431`), `economylog.rs` (`:326,360-375,553-556,578-663`: the phase rule below). Tests: create `crates/turts_sim/tests/orders.rs`, `crates/turts_server/tests/orders_plain.rs` (`#[path = "../src/orders.rs"] mod orders;`); invert `crates/turts_sim/tests/command.rs:89,116,233,322`, `crates/turts_pack/tests/economy_ordering.rs:120-175` (`NotMobile` at `:153`) and `:217-260` (`:247`), the group-move cases in `crates/turts_server/tests/economy.rs:2089-2236` (inside `e8_commands_are_authoritative_and_acked`, `:1875`, run by `scripts/acceptance.sh:106`), and `crates/turts_server/src/session.rs` tests that assert `NotMobile` for a group (`:1195-1201`; NOT TRACED by the producer, gate 2 td), only if they fail. Add the field `phase` to the `OperationAudit` struct literals at `crates/turts_pack/tests/forge_cases.rs:213,240,268,319,371,436` and `crates/turts_pack/tests/forge_renamed_clone.rs:330` (the field only; no expected value changes; re-check 1 sim N1).

**Validator rule (gate 2 ruling 5):** today the first non-command operation of a tick consumes `movement_pending` (`economylog.rs:369-372`), so an `Orders` operation before `Movement` gets the movement check. New rule: an operation with `phase: Orders` gets the clock-continuity check only; the movement transition is checked after the last `Orders` operation of the tick and before the first `Other` operation.

**Plain orders write no operation (D section 6.1):** a plain move writes only today's command operation (`apply_command` calls `capture_command`, `command.rs:155-159`). A gather order writes no order operation. Every other order write runs inside `capture_operation` (`crates/turts_sim/src/economy/transaction.rs:90`) with `phase: Orders`, one unit per operation.

- [ ] **Step 1: Red.** `crates/turts_sim/tests/orders.rs`: `per_unit_results_for_a_mixed_envelope` (300 units: 290 own mobile accepted, 5 own static `not_applicable`, 5 foreign `not_owner`), `shift_appends_and_plain_replaces`, `stop_clears_current_and_queue`, `queue_beyond_depth_is_queue_full`, `append_behind_hold_or_gather_is_not_queueable`, `queued_order_starts_in_the_tick_after_landing` (lands in `Movement` of tick t; the next order is current after `Orders` of tick t+1), `depleted_gather_writes_last_failure`, `plain_move_writes_no_order_operation` (one operation, the command operation, as `MoveTo`). `crates/turts_server/tests/orders_plain.rs`: `plain_move_envelope_has_the_moveto_operation_count` (a one-unit `Move` envelope through `orders::to_sim` and `apply_command` gives the same operations and `before == after` values as `CommandKind::MoveTo`; gate 2 qa 7). `economylog.rs` `mod tests`: `validator_accepts_an_orders_operation_before_movement`. Run `cargo test -p turts_sim --test orders --locked; echo exit=$?`, `cargo test -p turts_server --test orders_plain --locked; echo exit=$?`, `cargo test -p turts_server --bin turts_server --locked economylog::tests; echo exit=$?`. Expected: `per_unit_results_for_a_mixed_envelope` fails with `left: 0` accepted (today's all-or-nothing `validate_move_to`, `command.rs:255-294`); `validator_accepts_an_orders_operation_before_movement` fails with `Err("unit 1 movement differs from its prior target and speed")` (gate 2 sim probe); `exit=101`.
- [ ] **Step 2: Green.** Implement. Invert the listed tests. Re-run step 1: `exit=0`.
- [ ] **Step 3: Commit, gate.** Commit. G-RUST, G-QA, G-ACC, G-WEB, G-FROZEN (if `m2_operation_stream_matches_baseline` fails, stop and report, K19), G-REF.
- [ ] **Step 4: Plants.** (a) A plain move also writes `UnitOrders.current` inside `capture_operation`: `plain_move_envelope_has_the_moveto_operation_count` and `m2_operation_stream_matches_baseline` fail. (b) Give every non-command operation `phase: Other`: `validator_accepts_an_orders_operation_before_movement` fails. Restore each. **Runs: 8.**

### Task 9: Core logic, Bevy adapter, camera state, PROJ1, KM3

**Seat and model:** `.claude/agents/bevy-rendering-engineer.md`, Luna; `ux-designer` writes the headless core tests. Review: `technical-director` (Sol).

**Branch / needs / wave:** `codex/mc-t9`; needs 1, 8, 2b; wave 4. **Design:** sections 2.2 (rev 7), 2.3, 3.1, 3.3, 4.1 to 4.4, 7 (camera state), 13.2 (rev 7: `frames`); K11, K16. **Owns rows:** KM3, PROJ1.

**Interfaces (produces):** typed resources in `crates/turts_client/src/debug_feed.rs`, each written by one module, each read by `debug_json` and `report_json` (`crates/turts_client/src/debug.rs:31,212`): `SelectionFeed` (9), `GroupsFeed` (9), `ResultsFeed` (9, per-unit results of each envelope), `FramesFeed` (9: two lists as D section 13.2 revision 10: a ring of the last 64 presented frames, and the frames with a non-empty `applied_seq` since the last `frames_clear()`, at most 4,096 (the oldest record leaves at the cap), closed by a system in `Last` after the task 13 visuals write `RingsShown` and `MarkersShown`, so the mouse-up record survives until the oracle reads it after `drive`; the harness calls `frames_clear` with its own `page.evaluate` (`turts_debug` is a function, `t7.js:135`), never through a driver; a record holds unit positions, so one read can be several MB at 500 units (accepted, oracle only); a `frames_retained_until_read` test fills 200 frames after one click and finds the click record; each record holds the frame index, the wall time in ms, `applied_seq` (the sequence numbers of the input events that the core applied in that frame), the latest replicated tick, the presented unit positions, `MarkersShown` (13 writes), `RingsShown` (13 writes) and the order results received in the frame), `CursorFeed` (12), `UiSnapshotFeed` (12; until then `economy_ui.rs`), `NoticesFeed`, `AlertsFeed`, `AudioFeed` (15), `ContextMenuFeed`, `FocusFeed`, `PlayerIdFeed` (16), `AlertPings` (15 writes, 14 reads). **How `applied_seq` is filled:** a capture-phase listener in `crates/turts_client_web/index.html` copies `window.__turts_io_seq` (task 20) onto each canvas `pointerdown`, `pointerup`, `pointermove`, `keydown`, `keyup` and `wheel` event in a queue `window.__turts_seq_queue` (sequence, event type). At the start of each frame, the web entry drains the queue into the frame record: winit and the queue see the DOM events in one order, so the drained numbers are the events that the core applies in that frame. "The first presented frame after the mouse-up" is the first record whose `applied_seq` holds the sequence number of the mouse-up. Keys of `turts_debug`: `selection`, `groups`, `results`, `frames`, `cursor`, `notices`, `alerts`, `audio`, `contextmenu`, `focus`, `player_id`. `--out` adds `selection`, `groups` and the per-unit results (D section 2.3).

**Files:** Modify `crates/turts_interaction/src/*.rs` except `camera.rs` logic and `minimap.rs` (the core: input map, selection, groups, smart command, composition, projection; the core ignores a pointer event with `over_widget: true` as a ground click). Modify `crates/turts_mod_api/src/input.rs` (`RESERVED_BROWSER_KEYS` gets every P1 row with "browser acted: yes"). Modify `crates/turts_client/Cargo.toml` (`turts_interaction`), `Cargo.lock`, `lib.rs` (stub modules `actions.rs`, `camera.rs`, `widgets/mod.rs`, `visuals.rs`, `minimap.rs`, `feedback.rs`, `audio.rs`, `shell.rs`, `persist.rs`; the core plugin in both apps, `lib.rs:262-283`), `input.rs` (Bevy input to `InputEvent`, including `CursorLeft` to `PointerLeft`, `bevy_winit-0.20.0/src/state.rs:323-325`), `render.rs` (remove `fit_scene_camera` `:245` and its `PostUpdate` system `:454-458`, the fixed start `:336-342`, the `UiPanelRoot` import `:17`; `Selection` from `ui_shared.rs` at `:540`), `debug.rs` (the keys above), `economy_ui.rs` (moved types; writes `UiSnapshotFeed`), `live_view.rs:22,404`, `crates/turts_client_web/src/lib.rs:35` (`turts_ui` reads `UiSnapshotFeed`; the frame drain of `__turts_seq_queue`), `crates/turts_client_web/index.html` (the capture listener), `crates/turts_interaction/Cargo.toml` (dev-dependency `turts_mod_runtime`); create `crates/turts_client/src/ui_shared.rs`, `debug_feed.rs`, `core_adapter.rs` (`CoreAdapterPlugin`: the `WorldView` from the replica; copies `CameraState` into the Bevy `Transform` and the vertical field of view into `Projection` each frame; `init_resource::<Selection>()` and, each frame, writes `ui_shared::Selection` from the primary unit of the core selection, so `render.rs:540` keeps a writer after `input.rs` changes and after task 12 removes `EconomyUiPlugin`; re-check 1 bevy Low), `crates/turts_interaction/examples/effective_bindings.rs` (prints the effective input map of a package directory as JSON: engine defaults merged with `input/bindings.ron`). **Camera default (re-check 1 bevy):** until task 11 reads `CameraDef`, the default `CameraState` is today's pose: look-at at the map centre, from `CAMERA_HEIGHT` and `CAMERA_BACK` (`render.rs:336-342`), so T7 and CLICK9 see the units. Tests: create `crates/turts_interaction/tests/km3.rs`, `selection.rs`, `groups.rs`, `smart.rs`, `crates/turts_client/tests/proj1.rs`, `tests/browser/click9-driver.js`, `click9-oracle.js`, `tests/browser/rows/CLICK9.json` (a task check, not a D row: left click on the `acme` scout, right click on the ground; the oracle reads `results` and the tick log; gate 2 bevy F7: `t7.js` has no mouse call, `t7.js:16,214` use `turts_move`).

- [ ] **Step 1: Red.** `km3.rs`: `shift_click_toggles`, `shift_drag_adds`, `ctrl_click_selects_type_on_screen`, `two_clicks_within_400ms_and_4px_double`, `two_clicks_500ms_apart_are_two_singles`, `click_over_widget_is_not_a_ground_click`. `proj1.rs` (gate 2 ruling 19; re-check 1 bevy F3): a Bevy app with `MinimalPlugins`, `AssetPlugin`, `ImagePlugin`, `WindowPlugin { primary_window: None, ..default() }`, `TransformPlugin`, `MeshPlugin`, and `RenderPlugin { render_creation: RenderCreation::Automatic(Box::new(WgpuSettings { backends: None, ..default() })), ..default() }`; call `app.finish(); app.cleanup();` before `app.update()`. Without `MeshPlugin` the app panics at `bevy_render-0.20.0/src/mesh/mod.rs:64` (`Assets<Mesh>` missing; bevy probe, exit 101). The camera targets an `Image` with usage `MAIN_WORLD` (`bevy_render-0.20.0/src/camera.rs:334`); scale 2 uses `ImageRenderTarget { scale_factor: 2.0 }` on a 2560 x 1440 image (`bevy_camera-0.20.0/src/camera.rs:987-993`). The core field of view is 0.9 rad (not the Bevy default π/4), so a missing copy shows. 20 world points on 1280 x 720, 1920 x 1080 and 1280 x 720 at scale 2: the core projection and `Camera::world_to_viewport` differ by at most 1 px. Run `cargo test -p turts_interaction --test km3 --locked; echo exit=$?` and `cargo test -p turts_client --test proj1 --locked; echo exit=$?`. Expected: each `km3` test fails on the stub (`left: []`); `proj1` fails on the stub projection; `exit=101`. `examples/effective_bindings.rs` test (`crates/turts_interaction/tests/effective_bindings.rs`): for `mods/forge` the output holds `group_assign_1` = `Ctrl+Digit1`, `menu` = `F10`, `toggle_guide` = `F1` and the forge order keys; red on the stub (empty map).
- [ ] **Step 2: Green.** Implement the core (D sections 3.1, 4.1 to 4.4; the dense content-id index of D section 4.1; no selection cap; foreign units inspect only, K16), the adapter and the feeds. Re-run step 1: `exit=0`. `bash scripts/mc-pixels.sh tests/browser/rows/CLICK9.json; echo exit=$?`: `exit=0`.
- [ ] **Step 3: Defaults (waits for task 1).** The engine default bindings are D table 3.3 when `docs/research/mc/p1-keys.md` says `K11 (a) holds: yes`; else the CEO pick. Add the P1 rows to `RESERVED_BROWSER_KEYS`.
- [ ] **Step 4: Commit, gate.** Commit. G-RUST, G-QA, G-WEB, G-REF (browser cases per the G-REF rule).
- [ ] **Step 5: Plant (PROJ1).** Stop the copy of the field of view into `Projection` in `core_adapter.rs`; `proj1.rs` fails on all three viewports (re-check 1 bevy probe at a field of view of 0.9: 55.90 px; the 132.99 px of round 1 came from a field of view of 0.6); restore. **Runs: 6.**

## Wave 5

### Task 6: Group movement: spreading, avoidance, arrival, fallback

**Seat and model:** `.claude/agents/simulation-core-engineer.md`, Luna. Review: `performance-architect` (Sol): cost and determinism.

**Branch / needs / wave:** `codex/mc-t6`; needs 4; wave 5. **Design:** sections 6.3 to 6.7; K4 merge rule, K6, K17. **Owns rows:** none (GM40, GMA, GMU, DET1, EL1 are task 22).

**Merge and cap rules (gate 2 sim 5; D section 6.3):** envelopes with one key (slot, `group` above 0, tick) merge before the cap; `group` 0 never merges. A waiting envelope is atomic: it spreads in full or waits. Schedule for two slots that each send one 3,000-unit envelope in tick t: in tick t slot A is first in the turn, its head envelope spreads in full (above the cap of 2,048), and slot B waits with `Pending`; in tick t+1 the turn starts one slot later, so slot B spreads.

**Files:** Modify `crates/turts_sim/src/movement.rs` (`step` grows: separation, same-group yield, hard projection, arrival), `orders.rs` (merge, pending lists, round robin, cap, `Pending`), `command.rs:284-292` (non-finite: `target_not_finite`; finite outside: clamp inset by the unit radius), `economy/views.rs` (pool, spares, stall timer, best distance, claim count, pass mode in `AuditState`), `crates/turts_server/src/economylog.rs` (inputs of `step`); create `crates/turts_sim/src/grid.rs` (uniform grid, cell 2 x r_max, iteration in unit id order). Tests: create `crates/turts_sim/tests/group_movement.rs`; invert `crates/turts_sim/tests/command.rs:243,268`.

- [ ] **Step 1: Red.** `group_movement.rs`: `forty_units_rest_with_no_overlap_and_no_blocked` (model case A: farthest at most 4.58), `ring_to_point` (B), `second_group_same_point` (C2), `new_order_while_moving` (D), `cases_f_and_i_rest_with_no_overlap`, `one_unit_alone_takes_the_point`, `outside_target_is_clamped`, `nan_target_is_refused`, `post_target_gives_a_ring`, `two_slots_of_3000_spread_in_ticks_t_and_t_plus_1`, `group_0_never_merges`. Run `cargo test -p turts_sim --test group_movement --locked; echo exit=$?`. Expected: `forty_units_rest_with_no_overlap_and_no_blocked` fails with an overlap count above 0 (straight line, `movement.rs:12-58`; model: 780), `exit=101`.
- [ ] **Step 2: Green.** Implement. Re-run: `exit=0`.
- [ ] **Step 3: Commit, gate.** Commit. G-RUST, G-QA, G-ACC, G-WEB (T7 puts the unit within 0.011 of the ground point), G-FROZEN (in the frozen scenarios every mover keeps a gap of at least 0.5 to every unit, more than the reach of 0.05; gate 2 sim probe), G-REF. **Runs: 6.**

### Task 7: Server session: rate buckets (K25), rejoin (K27), `--command-script`, `--order-log`

**Seat and model:** `.claude/agents/backend-engineer.md`, Luna. Review: `network-engineer` (Sol).

**Branch / needs / wave:** `codex/mc-t7`; needs 2b, 4; wave 5. **Design:** sections 5.4, 10.5 (server side), 13.2 rev 7 (`--order-log`), 14 DET1 (the hook); K25, K27. **Owns rows:** none (SLOT1 is task 16, ORD6 task 15, DET1 task 22).

**Bucket rules (gate 2 sim 8; re-check 1 sim N2):** the tokens are whole numbers scaled by `tick_hz`, so no fraction accumulates at any tick rate (for example 30 Hz). Envelope bucket: capacity 64 x `tick_hz`, refill 640 for each tick, cost `tick_hz` for each envelope. Unit-order bucket: capacity 16,384 x `tick_hz`, refill 163,840 for each tick, cost `tick_hz` for each unit order. At any `tick_hz` this is 64 envelopes with 640 per second and 16,384 unit orders with 163,840 per second (K25; the tick rate stays Open, D18). The check runs in `admit_player_commands` (`session.rs:271-307`) after the sequence check and before the command queue. A `rate_limited` refusal writes its admission record as other refusals do (`record_admission_rejection`, `session.rs:433-510`). `--command-script` is not a session, so it does not pass the buckets.

**Order log (gate 2 ruling 4):** `--order-log <path>` writes one line `tick,unit_id,state,reason` for each change of a unit order state: `Pending`, `Moving`, `Blocked`, `Done`, `Failed` (`reason` is the `CommandRejectReason` name or `-`). Lines are in tick order, then unit id order. It is created in `crates/turts_server/src/orderlog.rs` (external crate paths only).

**Files:** Modify `crates/turts_server/src/session.rs` (buckets; `judge_hello` `:92-106`: when `claim` gives `AlreadyClaimed` and no live session holds that slot, give the same slot back with a new `Welcome` and a snapshot; while the old session lives, refuse with `This game is open in another window`), `config.rs:146-150` (`--command-script <path>`, `--order-log <path>`), `main.rs` (load the script: a JSON list of `{"tick": u32, "slot": u8, "envelope": OrderEnvelope}`, applied at fixed ticks with no client; install the order log), `crates/turts_server/tests/help.rs`, `scripts/acceptance.sh`; create `crates/turts_server/src/orderlog.rs`, `crates/turts_server/tests/orderlog.rs` (`#[path = "../src/orderlog.rs"]`), `crates/turts_server/tests/command_script.rs` (ignored process test). `Slots::claim` (`crates/turts_sim/src/slots.rs:46-57`) does not change.

- [ ] **Step 1: Red.** Unit tests in `session.rs`: `bucket_refuses_the_65th_envelope_in_one_burst` (`rate_limited` for each unit), `bucket_refills_after_one_second_of_ticks`, `unit_bucket_refuses_above_16384`, `reload_reclaims_a_held_slot_with_no_live_session`, `reload_while_the_old_session_lives_is_refused`. `orderlog.rs` test: a unit that goes `Moving`, `Blocked` gives two lines with the exact text `12,7,Moving,-` and `40,7,Blocked,blocked`. Run `cargo test -p turts_server --bin turts_server --locked session::tests; echo exit=$?` and `cargo test -p turts_server --test orderlog --locked; echo exit=$?`. Expected: `reload_reclaims_a_held_slot_with_no_live_session` fails with `Err("player already in match")` (`session.rs:32,103-105`); the order-log test fails on the stub; `exit=101`.
- [ ] **Step 2: Green.** Implement. Re-run: `exit=0`. `command_script.rs`: a script that moves one `acme` unit at tick 10 gives a tick log in which the unit moves from tick 10 and an order log with `10,<id>,Moving,-`; add its line to `scripts/acceptance.sh`.
- [ ] **Step 3: Commit, gate.** Commit. G-RUST, G-QA, G-ACC, G-REF. **Runs: 4.**

### Task 11: Camera logic and `LightDef` (K23)

**Seat and model:** `.claude/agents/bevy-rendering-engineer.md`, Luna. Review: `ux-designer` (Sol).

**Branch / needs / wave:** `codex/mc-t11`; needs 9, 19, 20; wave 5. **Design:** section 7; K9, K23; D68. **Owns rows:** CAM1.

**Files:** Fill `crates/turts_interaction/src/camera.rs` (pan by keys, edge band 8 px, middle drag; speed scales with height and with the scroll speed setting; zoom between mod limits; bounds plus margin; Space centres; `OwnedCentre`, `MapCentre`, `Point(x, y)`; `PointerLeft` stops the edge band), `crates/turts_client/src/camera.rs` (`CameraDef` from the interface file to the core; `LightDef` or today's light by default), `crates/turts_client/src/render.rs:343-346` (the fixed light goes). Create `crates/turts_interaction/tests/camera.rs`, `tests/browser/cam1-driver.js`, `cam1-oracle.js`, `rows/CAM1.json`.

- [ ] **Step 1: Red.** `camera.rs` tests: `arrow_pans`, `edge_band_pans`, `pointer_left_stops_the_edge_band`, `zoom_stays_in_limits`, `bounds_hold`, `space_centres`, `scroll_speed_2x_gives_2x_distance` (the menu path is SH1, task 16). `cam1-driver.js` holds ArrowRight 1 s, moves the pointer into the right edge band 1 s, wheels past the limits, presses Space with a selection, and moves the pointer out of the top edge. `cam1-oracle.js` reads `clip_from_world` before and after each step. Run `cargo test -p turts_interaction --test camera --locked; echo exit=$?` and `bash scripts/mc-pixels.sh tests/browser/rows/CAM1.json; echo exit=$?`. Expected: the tests fail on the stub; `FAIL cam1-oracle.js: camera did not move`; `exit=101` and `exit=1`.
- [ ] **Step 2: Green.** Implement. Re-run: `exit=0`, `exit=0`.
- [ ] **Step 3: Commit, gate.** Commit. G-RUST, G-QA, G-WEB, G-REF.
- [ ] **Step 4: Plant.** Ignore `PointerLeft` in the core: `pointer_left_stops_the_edge_band` and the CAM1 row fail; restore. **Runs: 4.**

### Task 12: Widget runtime, cursor, guide state, removal of the engine panel

**Seat and model:** `.claude/agents/bevy-rendering-engineer.md` with `ux-designer`, Luna. Review: `mod-runtime-engineer` (Sol): does the runtime read only table 8.4?

**Branch / needs / wave:** `codex/mc-t12`; needs 3, 9, 19, 20; wave 5. **Design:** sections 8.1 to 8.6, 10.3 (canvas size, R7), 12.1 (guide open at the first join); D67. **Owns rows:** HUD1, HUD4.

**Files:** Fill `crates/turts_client/src/widgets/` (`mod.rs`, `placement.rs`, `primitives.rs`, `sources.rs`, `tooltip.rs`, `cursor.rs` (writes `CursorFeed`), `guide.rs` (`GuideOpen`, open at the first join, the `toggle_guide` action and the guide close button), `snapshot.rs` (writes `UiSnapshotFeed`), `hit.rs` (sets `over_widget` on pointer events)). Images load with `RENDER_WORLD` usage only; cursor images keep the CPU copy and load as their own asset (D section 8.1). `UiScale` takes the player scale only (`bevy_ui-0.20.0/src/update.rs:57-62`). The `custom_cursor` feature is already in `3d` (gate 2 bevy). Remove `crates/turts_client/src/economy_ui.rs` and `crates/turts_client/tests/economy_ui.rs`; list each removed test and the MC test that covers its behaviour in the work log. Modify `lib.rs:7,37,279`, `live_view.rs`, `scenario.rs`, `crates/turts_client_web/src/lib.rs` (`turts_scene.widgets`: id and rect of each rendered widget), `crates/turts_client_web/index.html:8` (canvas 100 % of the viewport, R7). Create `crates/turts_client/tests/widgets.rs`, `tests/browser/hud-driver.js`, `hud-oracle.js`, `hud4-driver.js`, `hud4-oracle.js`, `tests/browser/rows/HUD1.json` (three viewports), `rows/HUD4.json`. Remove `tests/browser/m3-mobile.js` and its row in `tests/browser/legacy-bridge.tsv` (R6). `scripts/native-view-proof.sh:143-213` reads the old selection and `ui` rows; it is not in CI; record it in the work log (gate 2 bevy F8 c).

- [ ] **Step 1: Red.** Run the HUD1 row on `crates/turts_pack/tests/fixtures/muster_hud1` before the runtime: `bash scripts/mc-pixels.sh tests/browser/rows/HUD1.json; echo exit=$?`. Expected: `FAIL hud-oracle.js: widget card missing` and `engine panel present` (the panel of `economy_ui.rs:916-934`), `exit=1`. `widgets.rs`: `placement_resolves_each_anchor`, `guide_opens_at_first_join`, `toggle_guide_closes_and_opens`, `pointer_over_a_button_sets_over_widget`; they fail on the stub (`exit=101`).
- [ ] **Step 2: Green.** Implement. Re-run: card top left, minimap bottom right, text bottom centre, each within 1 px at 1280 x 720, 1920 x 1080 and 1280 x 720 at scale 2; no engine panel; `exit=0`. `grep -rnE "px\(|Val::Px" crates/turts_client/src/widgets --include=*.rs` finds hits only in `placement.rs`. HUD4: `hud4-driver.js` points at an own unit, a foreign unit, a node, an invalid target and a point in target mode; `hud4-oracle.js` reads `turts_debug.cursor` (context, mod image id, applied `CursorIcon`); `exit=0`.
- [ ] **Step 3: Commit, gate.** Commit. G-RUST, G-QA (the ledger has 4 rows), G-WEB, G-REF (the three `economy_ui.rs` browser cases enter the red window; record it).
- [ ] **Step 4: Plant (HUD4).** Return the `default` context for every hover; the HUD4 row fails; restore. **Runs: 8.**

## Wave 6

### Task 5: Mobile weapons, attack, attack-move, grid target choice (K5); ORD1

**Seat and model:** `.claude/agents/simulation-core-engineer.md`, Luna. Review: `qa-lead` (Sol).

**Branch / needs / wave:** `codex/mc-t5`; needs 4, 6, 19 (R2); wave 6. **Design:** sections 5.2, 5.3 (attack rows), 6.2; K2, K5. **Owns rows:** ORD1.

**Files:** Modify `crates/turts_sim/src/economy/combat.rs:165-200` (`choose_target` gets the ordered target; `fire` and `engage` decide the rest; a unit with no order keeps the M3 rule; candidates from `grid.rs`, not the scan of `combat.rs:89-199`), `crates/turts_sim/src/orders.rs` (`Attack` on a unit and on a point; `Hold` fires in range), `crates/turts_server/src/orders.rs` (`Attack`), `tests/m3/refutations.json` (C16: new `plant_path` if the range test moves to `grid.rs`). Create `crates/turts_server/tests/mc_orders.rs` (`#[path = "../src/orders.rs"] mod orders;`, gate 2 ruling 1).

- [ ] **Step 1: Red.** `mc_orders.rs`: `every_behaviour_has_an_envelope` (one envelope per `Behaviour`, nine in all, through `orders::to_sim` on `muster`), `rush_order_attacks` (`muster:rush` binds `Attack`, D section 5.2). The other ORD1 halves are `every_order_target_round_trips` (`crates/turts_net_protocol/tests/wire4.rs`, task 2a) and `protocol_3_hello_is_refused` (`session.rs`, task 2b). Run `cargo test -p turts_server --test mc_orders --locked; echo exit=$?`. Expected: `rush_order_attacks` fails (`not_applicable`, the task 2b rule), `exit=101`.
- [ ] **Step 2: Green.** Implement. Re-run, then `cargo test -p turts_net_protocol --test wire4 --locked; echo exit=$?`, `cargo test -p turts_server --bin turts_server --locked session::tests::protocol_3_hello_is_refused; echo exit=$?`, `cargo test -p turts_checks --test no_content --locked; echo exit=$?`: each `exit=0` ("rush" is not in engine code).
- [ ] **Step 3: Commit, gate.** Commit. G-RUST, G-QA, G-ACC, G-FROZEN (no forge unit gets an `Attack` order; the turret keeps the M3 rule, D section 6.2), G-REF (C13, C14, C16).
- [ ] **Step 4: Plants.** (a) Add `if command.order.order == "muster:rush"` in `crates/turts_server/src/orders.rs`; `real_source_scan_is_clean` fails. (b) Drop the `Job` arm of `to_sim`; `every_behaviour_has_an_envelope` fails. Restore each. **Runs: 4.**

### Task 13: World visuals; SEL2, SEL3, VIS1, VIS2

**Seat and model:** `.claude/agents/bevy-rendering-engineer.md`, Luna. Review: `ux-designer` (Sol).

**Branch / needs / wave:** `codex/mc-t13`; needs 4, 6, 9, 11, 19, 20 (gate 2 ruling 21: `UnitOrderRecord` and `Blocked`); wave 6. **Design:** sections 4.1, 9.1; D69 (presentation). **Owns rows:** SEL2, SEL3, VIS1, VIS2.

**Files:** Fill `crates/turts_client/src/visuals.rs` (rings: solid for own, four gaps for foreign; hover ring; range circles; health bars with the three modes; markers until rest, limit 60 s; waypoint lines from `UnitOrderRecord`; `Blocked` mark 2 s; one shared ring mesh and one bar mesh; one unlit material per slot colour; bar fill as a child with x scale; transform writes only on change; writes `MarkersShown` and `RingsShown`). Modify `crates/turts_client/src/render.rs:383,390,403-406` (one material per slot; writes on change). Create `crates/turts_client/tests/palette.rs` (CIEDE2000 under Machado 2009 matrices), `tests/browser/sel-driver.js`, `sel-oracle.js`, `vis-driver.js`, `vis-oracle.js`, `rows/SEL2.json`, `rows/SEL3.json`, `rows/VIS1.json`, `rows/VIS2.json` (`"runs": "headed"`), `docs/measurements/mc.md` (section "VIS2").

- [ ] **Step 1: Red.** `vis-oracle.js` finds two slot colours in the screenshot. Run `bash scripts/mc-pixels.sh tests/browser/rows/VIS1.json; echo exit=$?`. Expected: `FAIL vis-oracle.js: one colour for all units` (`render.rs:403-406`), `exit=1`. `cargo test -p turts_client --test palette --locked; echo exit=$?` fails on the stub palette, `exit=101`.
- [ ] **Step 2: Green.** Implement. SEL2 and SEL3: `sel-driver.js` box-selects the 500 units; drags a box with only the own `post` and `depot` (case `static_only`: they are selected); drags a box over own and slot 1 units (case `mixed_owner`: no foreign unit); Shift+drag adds; Shift+click removes; Ctrl+click and a double click on a trooper. `sel-oracle.js` takes the sequence number of the mouse-up from `view.inputLog()` and reads `turts_debug.frames`: the first record whose `applied_seq` holds that number has `RingsShown` 500 and `selection` 500. Run the rows SEL2, SEL3, VIS1 and `palette.rs`: `exit=0`.
- [ ] **Step 3: Commit, gate.** Commit. G-RUST, G-QA, G-WEB, G-REF.
- [ ] **Step 4: Plants.** Box test in world space (SEL2 fails); selection applied one frame later (the core keeps the box result and applies it in the next `step`): the first record with the mouse-up in `applied_seq` has `RingsShown` 0, so SEL2 fails with `FAIL sel-oracle.js: rings 0 in the mouse-up frame` (re-check 1 bevy F4, qa 6); type select with no screen test (SEL3 fails); one palette pair set to `#0072B2` and `#0073B3` (`palette.rs` fails). Restore each.
- [ ] **Step 5: VIS2 (headed, local; a second red, green, commit cycle for the record only).** Task 13 is the first writer of `docs/measurements/mc.md`: write the file with its title and an empty section "VIS2" first. Red: `grep -cE '^\| VIS2 \| (p50|p99|draw calls|memory) \| [0-9]' docs/measurements/mc.md; echo exit=$?` prints `0` and `exit=1` (with no file, grep exits 2; re-check 1 rigour N6). Start the `muster` server with `--synthetic-units 4485` (515 + 4,485 = 5,000). Run `node tests/browser/measure.js <page url> vis2 <Chrome WebGPU flags>` in `tests/browser`. Write four rows `| VIS2 | p50 | <ms> |`, `| VIS2 | p99 | <ms> |`, `| VIS2 | draw calls | <n> |`, `| VIS2 | memory | <bytes> |`, the machine (`lscpu` model name, the RTX 3080 of `lspci`), the Chrome version and the adapter. Green: the same `grep -cE` prints `4`, `exit=0`. GCP is pending (D59). Commit the record. **Runs: 5.**

## Wave 7

### Task 10: Headless `--actions` hook; HD1

**Seat and model:** `.claude/agents/bevy-rendering-engineer.md`, Luna. Review: `qa-lead` (Sol).

**Branch / needs / wave:** `codex/mc-t10`; needs 5, 9, 11, 19, 20 (gate 2 ruling 16; Attack is task 5, gate 2 rigour M2); wave 7. **Design:** sections 2.3, 13.5. **Owns rows:** HD1.

**Driver mapping (gate 2 bevy F6):** `hd1-driver.js` (role `driver`) cannot read `turts_scene`. It turns each world point of the action list into a pixel with `tests/browser/harness/project.js` (role `pure`): the `muster` camera start of `CameraDef` (literals in the file, as a person learns a HUD), viewport 1280 x 720 at scale 1. It waits for a tick with `io.wait(ticks x 50 + 200)` ms (20 Hz in `server.ron`). It keeps each pointer point at least 16 px from the canvas edge, so the 8 px edge band does not pan.

**Files:** Modify `crates/turts_client/src/options.rs` (`--actions <path>`; help text), `crates/turts_client/tests/options.rs`, `crates/turts_client/tests/help.rs`, `scripts/acceptance.sh`; fill `crates/turts_client/src/actions.rs` (each action to `InputEvent`s on a fixed 1280 x 720 virtual viewport). Create `crates/turts_client/tests/hd1.rs` (ignored process test), `tests/browser/hd1-driver.js`, `hd1-oracle.js`, `tests/browser/harness/project.js`, `tests/browser/rows/HD1.json`, `tests/browser/fixtures/hd1-actions.json` (one action per behaviour on `muster`, and `group_assign`, `group_recall`).

- [ ] **Step 1: Red.** `hd1.rs` runs `turts_client --headless --actions tests/browser/fixtures/hd1-actions.json --out <p>` against a `muster` server and asserts the per-unit results in `--out` (an `order_attack` on a `dummy` is accepted). Run `cargo test -p turts_client --test hd1 --locked -- --ignored; echo exit=$?`. Expected: `unknown option --actions`, `exit=101`.
- [ ] **Step 2: Green.** Implement. Re-run: `exit=0`. `bash scripts/mc-pixels.sh tests/browser/rows/HD1.json; echo exit=$?`: the oracle compares the browser accepted and refused sets with the headless `--out`; `exit=0`.
- [ ] **Step 3: Commit, gate.** Commit. G-RUST, G-QA, G-ACC.
- [ ] **Step 4: Plant.** Remove the core plugin from the headless app in `crates/turts_client/src/lib.rs`; `hd1.rs` fails (no results); restore. **Runs: 2.**

### Task 15: Notices, alerts, audio, the resume hook; ORD2, ORD6, ORD7, ALR1, AUD1

**Seat and model:** `.claude/agents/bevy-rendering-engineer.md`, Luna. Review: `ux-designer` (Sol).

**Branch / needs / wave:** `codex/mc-t15`; needs 4, 7, 12, 13; wave 7. **Design:** sections 9.2 to 9.5, 11.4 (the "no orders" notice); K7, K8; D69. **Owns rows:** ORD2, ORD6, ORD7, ALR1, AUD1.

**Files:** Fill `crates/turts_client/src/feedback.rs` (timeline of D table 9.2; reason texts with engine English defaults and mod overrides; notices that name the unit and the count, including the local `no_applicable_order` notice "Shell Turret: Cannot move" of forge; the notice "This game defines no orders" once for a package with 0 `OrderDef`, gate 2 ruling 11; writes `NoticesFeed`, `AlertsFeed`, `AlertPings`) and `audio.rs` (OGG Vorbis; categories `master`, `voice`, `alerts`, `effects`; one voice at a time; no audio player entity while a context is suspended; writes `AudioFeed`); alert derivation from `HealthRecord`, job and research records, `ExtractorStatus` (`crates/turts_net_protocol/src/messages.rs:226-231`), `JobStage::OutputBlocked` (`economy.rs:83-87`), `WeaponStatus::OutOfAmmo` (`messages.rs:239-245`), `InserterStage`, `GatherStage::Waiting`. Modify `crates/turts_client/Cargo.toml` (Bevy feature `audio`), `Cargo.lock`, `crates/turts_client_web/index.html` (a page script, loaded before the wasm, records each `AudioContext` and calls `resume()` on the first pointer down or key down), `crates/turts_client_web/src/lib.rs` (the `audio` key from `AudioFeed`). Create `crates/turts_client/tests/alerts.rs`, `crates/turts_client/tests/notices.rs`, `crates/turts_server/tests/mc_feedback.rs` (`#[path = "../src/orders.rs"] mod orders;`), `tests/browser/feedback-driver.js`, `feedback-oracle.js`, `rows/ORD2.json`, `rows/ORD6.json`, `rows/ORD7.json`, `rows/ALR1.json`, `rows/AUD1.json`.

- [ ] **Step 1: Red.** `alerts.rs` feeds forced records: own unit damaged, research ready, research complete, extractor full, output blocked, out of ammo, inserter waiting above 5 s; a second damage inside the cooldown; three alerts with sounds at once; each alert with a ping flag writes `AlertPings`. `notices.rs`: `no_orders_notice_shows_once`, `each_reason_has_a_player_text` (no raw code), `local_notice_names_the_unit`. Run `cargo test -p turts_client --test alerts --test notices --locked; echo exit=$?`. Expected: no alert lines and no notices on the stub, `exit=101`.
- [ ] **Step 2: Green.** Implement. ORD2 (D revision 7 row, rulings O4 and O7): headless half `mc_feedback.rs` boots `muster`, sends one envelope with 300 units (290 own mobile, 5 own static, 5 foreign) through `orders::to_sim` and `apply_command`: 290 accepted, 5 `not_applicable`, 5 `not_owner`, and the 290 move. Pixels half: on `muster`, the driver drags a box over the 40 troopers and the own `post`, then right-clicks the ground; by D section 4.1 the box holds the 40 troopers only; the oracle reads `turts_debug.results`: 40 accepted and 40 per-unit results; the 40 move and the `post` does not (tick log). ORD6: headless half is the task 7 bucket tests (`cargo test -p turts_server --bin turts_server --locked session::tests::bucket`; gate 2 rigour H1); pixels half: 100 envelopes at once give `rate_limited` beyond the bucket, an envelope after 1 s is accepted, every refusal shows a player text within one frame with the error sound. ORD7: `feedback-oracle.js` takes the sequence number of the right-click mouse-up from `view.inputLog()` and reads `turts_debug.frames`: the first record whose `applied_seq` holds it has `MarkersShown` at least 1 and the response sound, and it comes before the record that holds the `OrderResult`; the server tick log shows the first move in the draining tick. AUD1: each context `suspended` before the first click and `running` after; select and order responses in `voice`, one at a time; each volume changes its category gain and survives a reload; an unknown sound id fails `turts-pack`. Run the five rows and the tests: `exit=0`.
- [ ] **Step 3: Commit, gate.** Commit. G-RUST, G-QA, G-WEB, G-ACC, G-REF.
- [ ] **Step 4: Plants.** Restore the whole-list return in `crates/turts_sim/src/command.rs` (ORD2 both halves); show the reason code (ORD6); bucket size `u32::MAX` (ORD6); spawn the marker on `OrderResult` (ORD7: the mouse-up frame has `MarkersShown` 0); delay the order one tick (ORD7); cooldown 0 (ALR1); remove the resume script from `index.html` (AUD1). Each fails; restore each. **Runs: 5.**

### Task 17: Forge interface content and guide; SEL1, HUD2

**Seat and model:** `.claude/agents/ux-designer.md`, Luna. Review: `mod-builder` (Sol): can a modder write this file from the schema alone?

**Branch / needs / wave:** `codex/mc-t17`; needs 3, 12, 13, 19, 20; wave 7. **Design:** sections 8.6, 9.3, 9.4 (forge alerts), 12.1 (interface part, R4); K16, K22. **Owns rows:** SEL1, HUD2.

**Files:** Modify `mods/forge/manifest.ron` (`interface`, `assets` lists); create `mods/forge/interface/hud.ron` (selection panel; info panel with the inventory lines and, for the own turret, damage, range and ammo; command card; the lab research entry with its reason "Needs 10 Shell (n/10)"; production queue with a bar in seconds; alert lines; minimap with `ping`; guide of five pages, open at the first join, page 5 a `List` on `Bindings`; a groups row on `ControlGroups`; reasons with `no_applicable_order` "Cannot move"; alerts `ResearchReady`, `ResearchComplete`, `OutOfAmmo`, `OutputBlocked`, `InserterWaiting(5)` for slot 0 and `OwnUnitDamaged`, `OwnUnitDestroyed` for slot 1), `mods/forge/assets/assets.ron` and the files (icons, one portrait per unit type, one UI font, cursors, sounds; OGG from `ffmpeg` with `libvorbis`, present on the dev machine; record the font licence in `mods/forge/NAMES.md`, NOT TRACED until then). Create `crates/turts_pack/tests/forge_guide.rs`, `tests/browser/hud2-driver.js`, `hud2-oracle.js`, `sel1-driver.js`, `sel1-oracle.js`, `rows/HUD2.json`, `rows/SEL1.json`.

- [ ] **Step 1: Red.** `forge_guide.rs`: `guide_pages_1_to_4_name_no_key` (no word of pages 1 to 4 equals a key label of two or more characters from the effective input map (for example `Digit1` shown as "1" is excluded, `F10` is in) or a named special key: Ctrl, Shift, F1, F10, Esc, Space, Tab; single letters are not checked, because the English word "A" is plain text; gate 2 ruling 22, re-check 1 player C), `guide_page_5_lists_bindings`. Run `cargo test -p turts_pack --test forge_guide --locked; echo exit=$?` and `bash scripts/mc-pixels.sh tests/browser/rows/HUD2.json tests/browser/rows/SEL1.json; echo exit=$?` before the forge HUD exists. Expected: `exit=101`; `FAIL hud2-oracle.js: no selection panel`, `exit=1`.
- [ ] **Step 2: Green.** Write the content. The text has no "ticks", no "Applied", no coordinate and no studio word (D section 12.1). HUD2 oracle also checks: the guide is open at the first join and the camera shows the factory (Player path step 2); F1 and the close button close it (step 3); a right click with the turret selected gives the notice "Shell Turret: Cannot move" and the error sound (step 9); the `muster` resource line shows the ore balance. Re-run step 1: `exit=0`, `exit=0`. `real_source_scan_is_clean`: `exit=0`.
- [ ] **Step 3: Commit, gate.** Commit. G-RUST, G-QA, G-ACC, G-FROZEN (forge maps and units do not change).
- [ ] **Step 4: Plants.** `SelectionUnits` returns an empty list (HUD2 fails); the tooltip drops the key label (HUD2 fails); pick radius 0 in the core (SEL1 fails); a foreign click fills the command card (SEL1 fails); "Press R" in guide page 2 (`guide_pages_1_to_4_name_no_key` fails). Restore each. **Runs: 4.**

## Wave 8

### Task 14: Minimap view and ping; MM1, ORD3, ORD4, ORD5

**Seat and model:** `.claude/agents/bevy-rendering-engineer.md`, Luna. Review: `simulation-core-engineer` (Sol) for the order rows.

**Branch / needs / wave:** `codex/mc-t14`; needs 5, 12, 13, 15 (`AlertPings`, re-check 1 player A); wave 8. **Design:** sections 2.2 rev 7 (`Minimap { button, local }`), 4.3 to 4.5, 7 (minimap click), 8.2 (`MinimapView` with `ping`), 9.1 (waypoints), 9.4 (alert ping); K5. **Owns rows:** MM1, ORD3, ORD4, ORD5.

**Files:** Fill `crates/turts_interaction/src/minimap.rs` (a `Minimap` event moves the look-at point on left click and drag; a right click sends the smart command at the world point of `local`), `crates/turts_client/src/minimap.rs` (the widget sends `Minimap` events; draws units and nodes in slot colours, the camera frame, and the pings of `AlertPings`); modify `crates/turts_client/src/render.rs` only if the minimap needs a camera. Create `crates/turts_interaction/tests/minimap.rs`, `tests/browser/mm1-driver.js`, `mm1-oracle.js`, `orders-driver.js`, `orders-oracle.js`, `rows/MM1.json`, `rows/ORD3.json`, `rows/ORD4.json`, `rows/ORD5.json`; `crates/turts_pack/tests/mc_order_rows.rs` (the headless halves on `muster`, through `CommandKind::Order`).

- [ ] **Step 1: Red.** `minimap.rs` tests: `left_click_moves_look_at`, `drag_moves_look_at`, `right_click_orders_at_world_point`, `ping_shows_for_an_alert_ping`. `mm1-driver.js` left-clicks and drags on the minimap and right-clicks a minimap point with troopers selected. Run `cargo test -p turts_interaction --test minimap --locked; echo exit=$?` and `bash scripts/mc-pixels.sh tests/browser/rows/MM1.json; echo exit=$?`. Expected: the tests fail on the stub; `FAIL mm1-oracle.js: camera did not move after the minimap click` (the `MinimapView` of task 12 renders, the click does nothing; D section 14 MM1 red); `exit=101`, `exit=1`.
- [ ] **Step 2: Green.** Implement. ORD3, ORD4, ORD5: `mc_order_rows.rs` and `orders-driver.js` follow D section 14 rows ORD3 to ORD5 word for word (three Shift waypoints; replace; S; H fires in range; `queue_full`; `not_queueable`; start in the tick after landing; `node_depleted`; attack out of range; attack-move stops to fire; force move with no shot; smart command on ground, node and a slot 1 `dummy`). Player path step 18: the MM1 row uses two windows (`"windows": 2`); in window 0 the troopers attack a `dummy`; window 1 (slot 1) shows the `OwnUnitDamaged` ping of the `muster` interface (task 19 writes that alert) on its minimap, and the oracle checks the ping pixels there. The ping comes from `AlertPings` (task 15). Run all: `exit=0`.
- [ ] **Step 3: Commit, gate.** Commit. G-RUST, G-QA, G-WEB, G-REF.
- [ ] **Step 4: Plants.** `Append` treated as `Replace`; `Stop` keeps the queue; no completion rule (ORD3 fails each time); attack-move as move; `choose_target` ignores the ordered target (ORD4); smart command resolved once for the selection in `crates/turts_interaction/src/smart.rs` (ORD5). Restore each. **Runs: 3.**

### Task 16: Shell, menu, rebinding, persistence, focus, rejoin client; KM1, KM2, SEL4, SLOT1, CAM2, SH1, PER1

**Seat and model:** `.claude/agents/web-platform-engineer.md` with `ux-designer`, Luna. Review: `backend-engineer` (Sol) for the rejoin path.

**Branch / needs / wave:** `codex/mc-t16`; needs 7, 12, 15; wave 8. **Design:** sections 3.1 (labels), 3.2, 10.1 to 10.5, 7 (pointer lock), 14 CAM2 rev 7; K9, K10, K27; D70 client side, D72. **Owns rows:** KM1, KM2, SEL4, SLOT1, CAM2, SH1, PER1.

**Files:** Fill `crates/turts_client/src/client_shell.rs` (not `shell.rs`: the module token `shell` is a `forge` resource name, ruling O10) (load, rejection, focus overlay on the canvas blur (Bevy `WindowFocused`, R7), menu on F10 and Esc, connection lost on `reconnect_required`, `input.rs:46`; rebinding with the swap-or-cancel prompt for a key in use and Reset to the mod default, D section 10.2; writes `ContextMenuFeed`, `FocusFeed`), `persist.rs` (the keys of D table 10.4; the `<line>` rule; native files under `$XDG_CONFIG_HOME/turts` or `$HOME/.config/turts`; a value that does not parse gives defaults and one notice; delete the groups of other `match_id`s at join; writes `PlayerIdFeed`). Modify `crates/turts_client_web/src/lib.rs` (`sessionStorage` player id into `ClientOptions.player_id`; `getLayoutMap()` once at load in Chrome; the `contextmenu` count), `crates/turts_client_web/Cargo.toml` (`web-sys` features `Storage` and `Navigator` if needed), `crates/turts_client_web/index.html`, `crates/turts_client/src/lib.rs` (only if the player id path needs it). Create `tests/browser/shell-driver.js`, `shell-oracle.js`, `rows/KM1.json`, `rows/KM2.json`, `rows/SEL4.json`, `rows/SLOT1.json` (`"windows": 3`), `rows/CAM2.json`, `rows/SH1.json`, `rows/PER1.json`, `crates/turts_client/tests/persist.rs`.

- [ ] **Step 1: Red.** Run all seven rows and `persist.rs` before the shell: `bash scripts/mc-pixels.sh tests/browser/rows/KM1.json tests/browser/rows/KM2.json tests/browser/rows/SEL4.json tests/browser/rows/SLOT1.json tests/browser/rows/CAM2.json tests/browser/rows/SH1.json tests/browser/rows/PER1.json; echo exit=$?` and `cargo test -p turts_client --test persist --locked; echo exit=$?`. Expected failures: KM1 `binding lost after reload`; KM2 `no focus overlay`; SEL4 `group 1 empty after reload`; SLOT1 `reload got "no open slot"` (a reload makes a new random id, `crates/turts_client/src/lib.rs:76`); CAM2 `no engine cursor`; SH1 `no menu`; PER1 `no stored value`; `exit=1` and `exit=101`.
- [ ] **Step 2: Green.** Implement. Run the seven rows and `persist.rs`: `exit=0`, as D section 14, word for word, with the checks of the items below (re-check 1 rigour N7: all green checks come before the commit).
    - Rows. KM1 also runs `turts-pack` on a copy with a reserved default binding (exit 2). KM2 attaches `docs/research/mc/p1-keys.md`; its driver blurs the canvas with a click at (-5, -5). CAM2 checks lock start, the cursor, the summed motion, the edge band and F10; "Esc ends the lock" is the P1 row (D section 14 CAM2 rev 7).
    - SLOT1. The driver reloads within 5 s; the oracle sees the countdown and the same slot within 15 s; the server stats show no owner change; a second window with another id gets "no open slot"; a live window with a copy of the stored id gets "This game is open in another window" after the countdown (a duplicated tab copies `sessionStorage`). Record the time from the reload to the server's session close (NOT TRACED: whether the server sees the WebSocket close at once, K27).
    - SH1 rebinding, PER1. SH1: bind Stop to a key in use; the prompt offers swap or cancel; swap moves the other action to the old key; cancel keeps both; Reset restores the mod default; the guide page 5 updates after the rebind. PER1: a corrupt `turts.v1.settings` value gives the defaults and one notice.
- [ ] **Step 3: Commit, gate.** Commit. G-RUST, G-QA, G-WEB, G-ACC.
- [ ] **Step 4: Plants.** Revert the task 7 reclaim branch in `judge_hello` (SLOT1); no retry (SLOT1); `prevent_default_event_handling: false` at `lib.rs:254` (KM2); match `key`, not `code` (KM1); keep dead ids in groups (SEL4); no engine cursor under lock (CAM2); ignore the scroll speed (SH1); multiply `UiScale` by the device factor (SH1); a corrupt value panics (PER1). Each row fails; restore each. **Runs: 6.**

### Task 22: GM40, GMA, GMU, EL1, DET1 instruments; tick-cost record

**Seat and model:** `.claude/agents/qa-lead.md` with `performance-architect`, Luna. Review: `simulation-core-engineer` (Sol).

**Branch / needs / wave:** `codex/mc-t22`; needs 6, 7, 10, 11, 13, 19, 20 (R9); wave 8. **Design:** sections 6.1, 6.3 to 6.7, 14 rows GM40, GMA, GMU, DET1 (rev 7), EL1; K17, K19 a. **Owns rows:** GM40, GMA, GMU, DET1, EL1.

**Files:** Create `crates/turts_server/tests/mc_group.rs` (process tests on `muster` with `--tick-log` and `--order-log`, `#[ignore]`), `crates/turts_server/tests/det1.rs` (two server processes with `--command-script tests/browser/fixtures/gm40-script.json --run-ticks 420 --tick-log --order-log`), `crates/turts_server/tests/el1.rs` (`--economy-log` with GM40, GMA and a queued gather; the validator accepts each tick), `crates/turts_sim/tests/mc_cost.rs` (`#[ignore]`; tick time of spreading, avoidance and combat at 1,000 and 5,000 units), `tests/browser/gm40-driver.js`, `gm40-oracle.js`, `rows/GM40.json`, `tests/browser/fixtures/gm40-script.json` (GM40 and the GMA crossing, so many units have 2 or more neighbours in reach). Modify `scripts/acceptance.sh` (one line each for `mc_group`, `det1`, `el1`), `docs/measurements/mc.md` (sections "Tick cost" and "EL1 log size").

**DET1 comparison (gate 2 ruling 3; D rev 7):** with N = 400, `awk -F, -v n=400 '$1<=n {print $1","$3","$4","$5}' <log> > <log>.cut` for each tick log, then `cmp a.cut b.cut`; the order logs (`awk -F, -v n=400 '$1<=n'`) are compared byte for byte with `cmp`.

- [ ] **Step 1: Red.** Write the tests. `det1.rs` also counts the ticks in which a unit has 2 or more neighbours in reach (from the tick log positions) and fails when the count is 0, so the plant has work to change (gate 2 sim 6). `mc_group.rs`: GM40 (at rest: no position change above 0.001 for 2 s; 0 `Blocked` lines in the order log; no two discs overlap, distance below the radius sum minus 0.001; each unit within 4.58 of the target), GMA, GMU as D section 14. Run them on the build before the instruments' data exist: `cargo test --release -p turts_server --test mc_group --test det1 --test el1 --locked -- --ignored; echo exit=$?`. Expected: the tests fail on missing helpers (stub `todo` replaced by `assert!(false, "instrument stub")`), `exit=101`.
- [ ] **Step 2: Green.** Implement the instruments. Re-run: `exit=0`. GM40 pixels half: `gm40-driver.js` box-selects the 40 troopers (the camera starts on them) and right-clicks one point; `gm40-oracle.js` takes each disc centre from `ground_screen_center` back to y = 0 with `clip_from_world`, radius `pick_radius`; `bash scripts/mc-pixels.sh tests/browser/rows/GM40.json; echo exit=$?`: `exit=0`.
- [ ] **Step 3: Commit, gate.** Commit. G-RUST, G-QA, G-ACC, G-FROZEN.
- [ ] **Step 4: Plants.** DET1: iterate the grid neighbours through a `HashMap` with a new `RandomState`; `det1.rs` fails with `cmp` reporting `differ` on the `.cut` files; restore; `det1.rs` passes (`exit=0`). GM40: every slot is P; both halves fail. GMA: avoidance off; a `Blocked` unit rests inside another. GMU: clamp off (`target_outside_map`); a clamp that keeps NaN; slots reused for the second group. EL1: drop the group id from `AuditState`; write a position outside `step`; restore the validator of `economylog.rs:578-663`. Each fails; restore each.
- [ ] **Step 5: Records.** `cargo test --release -p turts_sim --test mc_cost --locked -- --ignored --nocapture; echo exit=$?`. Write the figures, the machine and the commit in `docs/measurements/mc.md`. Write the EL1 log size per unit and per pool slot (D section 6.1 estimate: about 60 and 20 bytes). No budget (S section 15 stays Open). Commit the record. **Runs: 3.**

## Wave 9

### Task 21: Four ledger rewrites, ledger to 0, the eight browser plants; DRV1

**Seat and model:** `.claude/agents/qa-lead.md`, Luna. Review: `web-platform-engineer` (Sol).

**Branch / needs / wave:** `codex/mc-t21`; needs 14, 16, 17, 18, 20; wave 9. **Design:** sections 13.3, 13.4; QA0D:422-436. **Owns rows:** DRV1.

**The four files** (`tests/browser/legacy-bridge.tsv:2-5` after task 12): `t7.js` to `t7-driver.js` + `t7-oracle.js` (QA0D conditions 1 to 4; the driver finds the unit by its slot colour; condition 2 reads `results`); `m3-human.js` to `pt1-driver.js` + `pt1-oracle.js` (the Player path steps 1 to 19, from screenshots, two windows; the driver reads `params.bindings` (the effective input map, engine defaults merged with `mods/forge/input/bindings.ron`) once at start and takes every key from it, never from literals; after its own rebind of Stop it uses the new key; an oracle step checks that guide page 5, as rendered (`turts_ui`), matches `params.bindings`; gate 2 ruling 14); `m3.js` to `m3-driver.js` + `m3-oracle.js` (research, enable and disable, remove, a drone move into range, through the forge HUD; conservation from the server files); `m2.js` removed: its pure helpers to `tests/browser/harness/png.js` (role `pure`), its oracle helpers to `tests/browser/m2-oracle-lib.js` (role `oracle`), the M2 cases to `m2-driver.js` + `m2-oracle.js` on the migrated M2 packages.

**The eight browser plants (gate 2 ruling 8, qa 5; ruling O3):**

| Case | Plant file | Disposition |
| --- | --- | --- |
| E2-browser-hardcoded-label | `economy_ui.rs` (removed) | re-target to `crates/turts_client/src/widgets/sources.rs` (the unit name from content) |
| E9-browser-missing-button | `economy_ui.rs` (removed) | re-target to `widgets/sources.rs` (`CommandCard`, `BuildMenu` items) |
| E9-browser-debug-only-ui | `economy_ui.rs` (removed) | re-target to `widgets/snapshot.rs`, or retire with a note |
| E9-browser-missing-output-mesh | `render.rs` | re-anchor in `render.rs` or `visuals.rs` |
| E9-browser-position-only-snapshot | `replica.rs` | keep the plant; `expected_failure` from `m2-oracle.js` |
| E9-browser-unbounded-buffer | `replica.rs` | keep the plant; `expected_failure` from `m2-oracle.js` |
| E9-browser-duplicate-outstanding | `ingest.rs` | keep the plant; `expected_failure` from `m2-oracle.js` |
| E9-browser-ignore-midtrace-rearm | `ingest.rs` | keep the plant; `expected_failure` from `m2-oracle.js` |

A plant with no MC equivalent retires only with a decision note in `vault/Decisions/` that names the plant id, the failure class it covered and the MC plant that now covers it. Task 21 lists the retired plants for the orchestrator; `qa-lead` reviews the list at gate 2.

**Files:** the files above; `tests/browser/rows/PT1.json` (`"windows": 2`, `"runs": "headed"`); modify `tests/browser/m2-projection.test.js:17` (require `m2-oracle-lib.js`), `scripts/t7.sh:82`, `scripts/m2-browser.sh:326` (and its `M2_CASE_SUMMARY` handling, `:130-134`), `scripts/m3-browser.sh:82`, `.github/workflows/ci.yml:108` (the unit test list; `:149` does not change), `tests/browser/legacy-bridge.tsv` (header only), `scripts/qa/driver_rule.py:13,308-309,386` (`--ledger-limit N`, default 0; the self-test passes 5 explicitly to the fixture directories; so the design line "the ledger cannot grow back" (D:1173) holds with no CI flag; re-check 1 qa), `tests/qa/driver/cases.tsv` (a case for the default limit on a directory with one ledger row: `DR7`), `scripts/mc-browser.sh` (`params.bindings` from `cargo run -q -p turts_interaction --example effective_bindings -- <package copy>`), `tests/m2/refutations.json`.

- [ ] **Step 1: Red.** Add `--ledger-limit` with default 0 first. Run `python3 scripts/qa/driver_rule.py tests/browser; echo exit=$?` with the four rows still present. Expected: `RED DR7 - tests/browser/legacy-bridge.tsv:1: legacy bridge ledger exceeds limit 0`, `exit=1`. `python3 scripts/qa/driver_rule.py --self-test; echo exit=$?`: `exit=0` (the self-test passes 5 to the fixtures; the `dirs/dr6-repaired` case keeps `DR6`).
- [ ] **Step 2: Green.** Rewrite the four files; empty the ledger; handle the eight plants. Re-run step 1: `exit=0`. Run each new pair twice: `bash scripts/t7.sh; echo exit=$?`, `bash scripts/m2-browser.sh; echo exit=$?`, `bash scripts/m3-browser.sh; echo exit=$?`, twice each, `exit=0`. `bash scripts/m2-refutations.sh --case ci-browser; echo exit=$?`: `exit=0`.
- [ ] **Step 3: Commit, gate.** Commit. G-RUST, G-QA, G-WEB, G-REF.
- [ ] **Step 4: CI.** Push `codex/mc-integration` after the merge; `gh run list --branch codex/mc-integration --limit 1` shows the `browser` job green at "M2 browser proof", "M3 browser controls" and "M2 browser-only semantic refutations". Record the run id.
- [ ] **Step 5: Plants.** Add `globalThis.turts_debug()` to `t7-driver.js`: `python3 scripts/qa/driver_rule.py tests/browser` exits 1 with a Part A line (DRV1). Set the default limit back to 5 and add one ledger row: `driver_rule.py tests/browser` exits 0, so the default is the guard (record it). Replace one `params.bindings` key in `pt1-driver.js` with a literal `"KeyS"` after the rebind: the PT1 oracle fails at step 16 (headed, local). Restore each. **Runs: 8.**

## Wave 10

### Task 23: PT1 and PT2

**Seat and model:** `.claude/agents/player.md` (Sol) through the harness `io` only, and the person. Review: `ux-designer` (Sol) reads the play logs. **Who signs off (gate 2 ruling 13; D section 14 PT1 rev 7):** a person who has not read the design, named by the CEO, plays and signs off. When the CEO plays, the play log records "not blind". The `player` agent gets the build URL, the goals of D section 14 PT1 and nothing else; its dispatch tells it to read nothing under `docs/` or `mods/` for PT1, and its play log records every file it read.

**Branch / needs / wave:** `codex/mc-t23`; needs 14, 15, 21, 22; wave 10. **Design:** section 14 rows PT1, PT2; `## Player path` (D:1670-1712); K22, K27. **Owns rows:** PT1, PT2.

**Files:** Create `docs/research/mc/pt1-playlog.md`, `docs/research/mc/pt2-playlog.md` (the agent and the person; time of each goal; each place the tester stopped for 60 s; blind or not blind; files the agent read).

- [ ] **Step 1: Red (instrument).** Commit the empty play logs. Plant: remove guide page 5 from `mods/forge/interface/hud.ron`. Run `bash scripts/mc-pixels.sh tests/browser/rows/PT1.json` with headed runs allowed (`MC_PIXELS_HEADED=1`); expected: the oracle fails at the guide page 5 step (task 21), `exit=1`. Restore.
- [ ] **Step 2: PT1, agent.** The `player` agent plays through the harness with the goals only. The oracle checks the 19 Player path steps.
- [ ] **Step 3: PT1, person.** Start the game with `bash scripts/mc-browser.sh mods/forge --serve-only` (task 20); give the person the URL and the goals only. The person opens window A, and opens window B 30 s later (the near drone dies at about tick 544, 27.2 s at 20 Hz, gate 2 player run; K22). The test fails when the person cannot find an object, an action or a reason, or when 60 s pass with no sign of progress. The person signs off in the play log.
- [ ] **Step 4: PT2.** The person and the agent open `muster`, box-select the 40 troopers and right-click one point; they judge from the screen, by the ground rings, no overlap, near the marker, no jitter; the marker stays until rest. For the agent run, plant spreading off (the GM40 plant); the agent reports overlap; restore.
- [ ] **Step 5: On a failure (gate 2 player 8).** Record the step and the cause; the orchestrator assigns the fix to the owner task of that step (table "Acceptance ownership" and the Player path note). The re-run needs a new person who has not played MC, or the log records "not blind". Commit the play logs. **Runs: 3.**

## Wave 11

### Task 24: CI wiring, measurements file, vault; REG1, CI1

**Seat and model:** the orchestrator (Sol); `qa-lead` (Luna) runs the REG1 plants in test mode.

**Branch / needs / wave:** `codex/mc-t24`; needs 23; wave 11. **Design:** section 14 rows REG1, CI1; section 15. **Owns rows:** REG1, CI1.

**Files:** Modify `.github/workflows/ci.yml` (a `browser` job step `bash scripts/mc-pixels.sh tests/browser/rows/*.json`; headed rows print `SKIP`), `docs/measurements/mc.md` (header: commit, machine, the P1 summary line, links to the play logs), `vault/MC Execution Board.md`, `vault/Work Log/<date>.md`, `vault/Reports/<date>.md`. PROJ1 runs in the `check` job through `cargo test --workspace` (task 9); no extra step.

- [ ] **Step 1: Commit.** Commit the CI change and the measurements header.
- [ ] **Step 2: REG1 green.** On `codex/mc-integration`: G-RUST, G-QA, G-ACC, G-WEB, G-FROZEN, `bash scripts/m2-refutations.sh --case ci-native; echo exit=$?`, `bash scripts/m2-refutations.sh --case ci-browser; echo exit=$?`, `bash scripts/m3-refutations.sh --case all; echo exit=$?`, `bash scripts/mc-pixels.sh tests/browser/rows/*.json; echo exit=$?`. Each prints `exit=0`.
- [ ] **Step 3: CI1.** Push; `gh run list --branch codex/mc-integration --limit 1` shows `check`, `browser` and `agents` green. Record the run id.
- [ ] **Step 4: REG1 plants (qa-lead).** (a) Give a single unit an offset slot: `one_unit_alone_takes_the_point` and T7 fail. (b) In the branch of `choose_target` for a unit with no order, take the highest unit id in range, not the nearest: `nearest_target_wins_tie_lowest_unit_id` (`combat.rs:501`) and `forge_spec_13_4_fixed_point_and_ten_vs_five_shots` fail. Restore each.
- [ ] **Step 5:** Tick each row of D section 14 with its command, exit code and the red seen. Do not merge into `main`: report to the CEO with the integration commit, the run count and the CI link. **Runs: 2.**

---

## Risks

1. **Name collisions in the no-content scan.** MC engine code can add an identifier equal to a content id (for example `post`, `worker`). G-RUST runs the scan in every task; the engine name changes.
2. **The refutation registries move with the code.** 41 cases of `tests/m2/refutations.json` and 17 of `tests/m3/refutations.json` anchor on exact text (`tests/m2/refutations.py:270-280`). A re-anchor is a test-instrument edit; `qa-lead` reviews each one.
3. **Browser checks hidden until task 20 step 1.** No hosted run checks `m3.js` or a browser plant today (Known red window).
4. **Task 2b is wide.** It touches 6 crates and about 30 files. 2a lands the types first; 2b copies one outcome to every unit until task 4.
5. **P1 can stop the plan (K11).** Task 9 step 3 waits for the CEO pick if P1 is red or Firefox is NOT TRACED.
6. **Acceptance load on tasks 13 to 16 (R10).** They own 20 of 42 rows with 19 runs of estimate (5 + 3 + 5 + 6).
7. **Determinism.** Every new iteration is in unit id order (D section 6.1); DET1 is the guard, with the neighbour self-check.
8. **Budget.** The total (about 212) is above the design range.

## Doc updates for the orchestrator

1. `CLAUDE.md`, crate table: add `turts_interaction` (responsibility: D section 2.1 row; forbidden: `bevy*`, `wgpu`, `winit`, `turts_sim` on host and wasm32). `turts_client` row: `core_adapter.rs`, `ui_shared.rs`, `debug_feed.rs`, `actions.rs`, `camera.rs`, `widgets/`, `visuals.rs`, `minimap.rs`, `feedback.rs`, `audio.rs`, `shell.rs`, `persist.rs`; `economy_ui.rs` removed. `turts_client_web` row: `turts_ui` from `UiSnapshotFeed`, `turts_scene.widgets`, the bridge keys of D section 13.2 (with `frames`), the audio resume script. `turts_sim` row: `orders.rs`, `grid.rs`, `SimSet::Orders`, `movement::step`. `turts_server` row: `orders.rs`, `orderlog.rs`, the rate buckets, rejoin in `judge_hello`, `--command-script`, `--order-log`. `turts_mod_api` row: `orders.rs`, `input.rs`, `interface.rs`, `assets.rs`. `turts_mod_runtime` row: `content_mc_validation.rs`, `asset_headers.rs`, `check_locked_set_caps`.
2. `CLAUDE.md`, "Build and test commands": `ENGINE_API_VERSION` is 0.2.3; `NET_PROTOCOL_VERSION` 4; the four version table rows; the `turts-pack` K18 warnings with exit 0 and the K12 caps; `turts_client --actions <path>`; `--move X,Y` now sends one `Move` envelope with the first `OrderDef` of behaviour `Move` (not "one MoveTo"); the `--out` and `command_results` shape (`accepted`, `refused`; `selection`, `groups`); `turts_move` sends a `Move` envelope; `turts_server --command-script <path>` and `--order-log <path>`; `scripts/mc-browser.sh` (with `--serve-only`), `scripts/mc-pixels.sh`, `tests/browser/rows/`; the role `harness`; `driver_rule.py --ledger-limit N` (default 0; the ledger has 0 rows); the measurements file `docs/measurements/mc.md`; "M1 to M3, QA0 and MC are done" when the CEO merges.
3. `CLAUDE.md`, "Decisions that are still open": the player control lines that MC designed change with the spec rows below.
4. Spec section 2: D65, D67 and D73 have Decided lines (S:94, S:96, S:102); only their design part changes status. D66, D68, D69, D70 (client side), D72 and D74 change from Proposed to the status that the CEO gives after MC. D71 does not change.
5. Spec section 15: S:755 "Player input" (closed by D section 3 and P1); S:756 "HUD" open items: the widget set, the mod schema, image formats and size limits, the image reference form (closed by D section 8 and K12); S:757 "Order model": the queue, the envelope fields, the protocol bump (closed by K24, D section 5.1, K14); S:750 "Presentation": camera controls (D68) and audio format OGG Vorbis (K7); the lighting API keeps only `LightDef` (K23).
6. Spec S:722, MC row, acceptance evidence: "D section 14 of `docs/superpowers/specs/2026-10-29-mc-player-control-design.md`" (the producer confirms it, D section 17.3 question 1).
7. `docs/qa/specs.tsv`: the MC row from `gated` to `done` in the merge commit (what `completeness.py` enforces for a `done` row is NOT TRACED). `docs/qa/milestones.tsv` needs no change.
8. The M4 draft amendment takes protocol 5 (D section 17.3 question 4).
9. The design, stale citations: `content_m3.rs:339` is `crates/turts_mod_runtime/tests/content_m3.rs:339`; `ContentRef` is at `crates/turts_net_protocol/src/economy.rs:16`; D section 5.5 "the protocol id of netcode stays": the line stays, the value becomes 4; D section 6.2 "No forge unit gets an order in the M3 cases": `crates/turts_pack/tests/forge_fixed_point.rs:82` and `forge_cases.rs:516,1017` send a plain `MoveTo`; D:1250 names an "M3 operation stream" that does not exist.
10. `scripts/boundary.sh:4-8`: the comment about the no-content grep is out of date. `scripts/native-view-proof.sh:143-213` breaks after tasks 9 and 12 (not in CI).
11. Vault: `vault/MC Execution Board.md`; a decision note for the rulings O1 to O8 and the 22 gate 2 rulings (2026-10-29), and for each retired plant of task 21; the daily report.

## Open questions for the orchestrator

Rulings of 2026-10-29: O1 (a), design revision 6 (`dummy`, `rush`); O2 confirmed; O3 re-target, retire only with a decision note (task 21); O4 and O7 the ORD2 rows of revisions 6 and 7; O5 replaced by gate 2 ruling 13; O6 R1 to R11 accepted; O8 the ORD1 row says "rush"; gate 2 rulings 1 to 22 applied in repair 1; G1 answered by design revision 8 (D sections 12, 4.5, 6.3); the re-check 1 items 1 to 13 applied in repair 2.

- **G2.** The critical path is now 55 runs and the total about 212, above the design range (165 to 205). The CEO decides whether to go on.

## Evidence for this plan

| Command | Claim tested | Result that would refute | Observed |
| --- | --- | --- | --- |
| `git diff --stat 7a90f97 f9353bb -- crates mods scripts tests` | the code citations hold at `f9353bb` | a changed code file | empty |
| G-REF selector on a scratch list (`transaction_m3.rs`, `machine.rs`, `command.rs`) | the selector reads `plants[].path` | E4, E6-capacity-bypass, C1 or C5 missing | all four listed, `exit=0` |
| `python3 -c` over both registries for `plants` | the multi-file key | another key | `plants[].path` for E4-local-capacity, E6-capacity-bypass, C1, C5 |
| `sed -n` of each cited range (boundary, client, render, input, protocol, server, sim, mod API, mods, ticklog, config, economylog) | every `path:line` in this plan | a moved line | as cited |
| Python over D section 14 and `## Yardstick coverage` (scratch `coverage.txt`) | 42 rows; 81 `in` rows map to 27 tests | a test outside section 14 | none outside |
| Python longest path and cycle check (scratch `deps.py`, new Needs) | waves and path | a cycle | no cycle; path 55 |
| JSON scan of both registries | plant files that MC edits | a missing file | table of the Execution contract; 8 browser cases |
| Scratch crate `scanprobe` (`#[path]` to `support/mod.rs`, a package with the candidate ids as unit ids, `derive_tokens` then `scan_rust` over `crates/`) | content ids of the task 18 table | a hit for a new id | `exit=0`; hits only for `gather` 17, `enable` 2, `disable` 5, `remove` 37, `research` 115, `produce` 6, `marker` 6, `collect` 256, `build` 8, `bindings` 10, `assets` 12; 0 for every new id and for `move` |
| `sed -n 360-375 economylog.rs`, `sed -n 60-85 ticklog.rs`, `sed -n 45-52 config.rs` | gate 2 sim 1, sim 2, qa 1 facts | other code | `movement_pending` consumed by the first non-command operation; `wall_ns` in column 2; `--run-ticks` may run past N |
| `sed -n 48-66 t2c_package_rejects.rs`, `sed -n 3180-3192 economy.rs` | gate 2 td 3 | no `^0.2` anchor | anchors `engine_api: \"^0.2\"` at `:52,:64`, `0.2.2` at `:57`, cedar anchor at `:3182` |
| `sha256sum tests/m3/m2_operation_stream.json` | the frozen hash | another hash | equal |

**NOT TRACED:** the lightyear message size limit (task 2b step 1); the cause of the red "M2 browser proof" (task 20 step 1); whether the server sees the WebSocket close at once (task 16 step 5); the Firefox start outside the sandbox (task 1 step 3); the font licence; the registration order of unit types (task 19 test); `session.rs:1195-1201` after task 4; the winit focus citations of R7 (gate 2 bevy, not re-read); what `completeness.py` enforces for a `done` row.
