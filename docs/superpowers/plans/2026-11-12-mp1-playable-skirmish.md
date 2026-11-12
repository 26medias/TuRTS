# MP1: Playable skirmish for two people: implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Plan status:** DRAFT, before gate 2. Author: `producer` (build mode), 2026-11-12. Root commits this text.

**Design of record:** `docs/superpowers/specs/2026-11-12-mp1-playable-skirmish-design.md`, revision 6 (branch `mp1-design`, head `a9a5fea`), "D" below. `D:n` is line n of D.
- Gate 1 is closed at revision 6. The Medium and Low items of the last checks are binding on this plan (section 12).
- When this plan and D disagree, D wins. Report the conflict to Root. Section 2 lists each place where this plan refines D. Each refinement needs Root's acceptance at gate 2.
- The slice plan `docs/superpowers/plans/2026-11-12-pre-mp1-slice.md` on `pre-mp1/integration` ("SP") is the model for form. Its gate sets (SP section 4) apply here unless a line says otherwise.

**Goal (D section 1):** two people finish a match with mouse and keyboard; both build, power and fight; both see the result screen. Two stages: stage 1 is the thin loop up to checkpoint C1; stage 2 is the breadth, then the play test PT3.

**Scope rule:** this plan adds no scope. Each task names its scope items (D section 4, X0 to X26) and its rows (D section 12). The check script (section 3.5) fails when a scope item or a row part has no task.

### CEO decisions this plan assumes

Every line of D section 2 is "CEO to confirm". The plan uses the working text. The last column names the plan tasks that a different CEO answer changes. Lane 0 freezes the schema at `L0-EXIT` (offset 10, section 3.2). A CEO answer that differs after that point is a change request (section 5.2). So Root should get the CEO answers before `L0-EXIT`: **needs decision before offset 10**.

| Id | Working text the plan uses | Tasks that a different answer changes |
| --- | --- | --- |
| P1 | (b) models in stage 2, cap 9 models, shapes in stage 1 | (a) or (c): `PRES-MODELS`, `PRES-ART` out (-9 runs, about -11 with the smaller `PRES-MEASURE` and `MEAS-EXIT`), `PRES-MEASURE` primitives only, `I2-PIX`, `MEAS-EXIT` |
| D-M | (a) caps of D 8.1 now (8,000 triangles, 4 MiB, 16 MiB, 2 materials, 32 MiB decoded) | (b): `PRES-MODELS` sets the caps after `PRES-MEASURE`; `L0-API1` row V1 numbers provisional |
| P2 | (a) two slots, ready and colour, team = slot | (b): `L-LOBBY2-SRV`, `L-LOBBY2-UI` drop colour, row LOB2 out; (c): also `L-MATCH-SIM` (teams), `L-ROOM0` |
| P3 | (a) keep all | (b): `L-RLY` out, `UI2-SMART` rally part, `C2-CONTENT` H3; (c): `L-CON3` cancel part; (d): `L-REP` out (changes Decided S:798) |
| P4 | (b) pay over time | (a): `L-PAY`, `L-CON1`, `L-INC`, `UI-HUD` texts, `UI-ALERT` no-credits state, `C-UNITS` |
| D-A | (a) `floor(cost x progress)` difference, u128, coalesced events | (b) or (c): `L-PAY` (rows PAY1, PAY2) |
| P5 | minimal sound set, stage 2 | none: `FX2` sound part and `C2-SOUND` out (row AUD2); voices: `C2-SOUND` grows |
| P6 | "Lantern Company", names of D section 5 | `C-UNITS`, `C-UI` texts |
| P7 | two people, one not the CEO | `PT3-PEOPLE` |
| P8 | (b) defeat after 120 s disconnected, mod clause | (a): `L-MATCH-SIM2` (`DisconnectedFor`), `L-MATCH-SRV2` disconnect cases, `C2-CONTENT` |
| P9 | (a) free approach point and placement gap | (b): `L-OBS` becomes an M4 route task (400 to 700 lines, D:1432-1433) |
| D-B | (1) seat is a reservation; start step claims | (2): `L-ROOM0`, `L-MATCH-SRV1`, `L-LOBBY-C1`, `L-LOBBY2-SRV` |
| D-C | (a) one match for each process | (b): `L-MATCH-SRV1` (world reset), row LEAVE1 |
| D-D | (a) sim inputs in the command log | (b) is not a valid option (D:178) |
| D-E | (a) site entity owns progress | (b): `L-CON1`, `UI-HUD` |
| D-E2 | (a) open-site cap as mod data | (b) or (c): `L-CON1`, `L0-API2` (row K6) |
| D-F | (a) start order on a map placement | (b): `L-STO` placement part out, `C-UI` hint H1, `I-HEAD` row STO1 |
| D-G | (a) idle retaliates; hold fires back in range, never moves | (b): `L-TGT` |
| D-H | (a) sessionStorage | (b): `L-LOBBY-C1` |
| D-I | (a) full 0.2.4 table in D | (b): `L0-API1`, `L0-API2` |
| D-J | (a) index, debug gate, grids, A3 entry | (b): `L0-SIM`, `L0-DBG`, `L-TGT`, `C1-FINAL` (A3 input) |
| D-K | (a) no rematch | (b): `L-MATCH-SRV1`, `UI-RESULT` |
| D-L | (a) free income 3 credits each second | (b): `L-INC` out (row INC1), `C-UNITS`, `C-UI` role line |
| D-N | (a) VIS3 floor 50 ms p95, 33 ms p50 before PT3 | (b): `MEAS-EXIT`, `PT3-PEOPLE` start condition |
| Q1 | (a) no named producer queues | (b): `L-CON1`, `L0-API1`, `UI-HUD` |
| S1 | edit S:723, S:809, S:801, S:727 after MP1 | keep: `REC` |
| 6.7.14 item 1 | `all_disconnected: Continue` default | `Shutdown`: `L-MATCH-SRV2` |
| 6.7.14 item 4 | keep the 10 s netcode timeout | lower it: `L-ROOM0` trace, `network-engineer` change, MC 10.5 |
| N1 | (a) bare and qualified ids | (b): `I-NC1` |
| Y1 | (a) keep the edge MP07 to MP01 | (b): no task; `REC` updates the coverage note |
| MC hosted CI (D:76-79) | option (b): MP1 pixel rows on the local GPU gate with receipts | `I-PIX1`, `REG2-1`, `I2-PIX`, `REG2-2` |

## 0. Line numbers and labels

- Code lines in this plan are at the slice head `85f34cb` (worktree `/home/julien/Projects/TuRTS-pre-integration`, branch `pre-mp1/integration`). D cites the older `mp1-design` tree. Between the two trees only 13 code files differ (`git diff --stat a9a5fea 85f34cb -- crates scripts mods`); the D anchors outside `economylog.rs` hold (anchor script, section 18).
- S1 moved the replay validator from `economylog.rs` to `crates/turts_audit/src/replay.rs` (`validate_records` at `replay.rs:97`). A1c deletes that file.
- **Verify first** means: the producer read the line at `85f34cb`, and a slice task will move it before the MP1 task starts. The builder finds the item again by its name before the edit. S2 moves `crates/turts_sim/src/orders.rs` to `orders/`. S3 moves `movement.rs` to `movement/` (lane `pre-mp1/s3-movement`, `a6f3f66`, in review). S45 splits `turts_interaction/src/lib.rs`, `widgets/mod.rs`, `visuals.rs`, `client_shell.rs` and `core_adapter.rs` (lane `pre-mp1/s45-client`, `d047433`).
- **NOT TRACED** means: the producer did not read it. Section 17 lists every such item.
- Offsets are in runs from now (2026-11-12). One run is one agent dispatch (build, review or repair). The D0 wait is not in the offsets (section 14.3).

## 1. Execution contract

- **Integration branch (refinement R10).**
    - Create `mp1/integration` from `pre-mp1/integration`. Never merge MP1 work into `pre-mp1/integration`, `codex/mc-integration` or `main`.
    - After each slice merge, Root merges `pre-mp1/integration` forward into `mp1/integration`. A conflict there goes to the MP1 owner of the file (section 3.4). The slice never waits for MP1.
    - MP1 merges to `main` only after MC merges to `main` (D:2548-2549) and after `AUDIT`.
- **One branch, one worktree and one build cache per task.**
    - Branch `mp1/<task id in lower case>`, from `mp1/integration` after every input of the task has merged there.
    - `git worktree add ../TuRTS-mp1-<id> mp1/<id>`; `CARGO_TARGET_DIR=target/mp1-<id>`. Delete both after the merge.
    - A task starts the moment its inputs merge (ruling C391). There are no waves in execution; the waves of section 3.2 are a reading aid.
- **Merge after (refinement R8).** A task with a "Merge after" cell builds against the frozen contract of lane 0 from its inputs. It rebases and runs its rows only after the named tasks merge. Root may start it at any offset inside its start window (section 3.1, the slack column).
- **Reviewers.** Each reviewer works in `../TuRTS-mp1-<id>-review` at the candidate commit. It runs the task's fast gate and one fault of its own (red, exact restore, fresh green). It cites `path:line`. Lane-0 tasks and `L0-EXIT` are foundation work: `L0-EXIT` is the second lens for every lane-0 task.
- **Step order in every task** (SP section 1): red, green, commit, gate, plants.
    - A compile error is not a red. A `todo!()` body that panics in a test is a red.
    - Restore each plant with `git checkout -- <file>`. Then `git status --porcelain -- <file>` prints nothing.
    - Write `; echo exit=$?` after each command. Never pipe a red or a green through `tail`, `head` or `grep` before you read its exit code.
    - Use `cmp` or `rtk proxy diff` for file comparisons, never the filtered `diff`.
- **Assertions.** SP section 1a applies: a builder never changes an assertion or an expected value on his own decision. The four permitted kinds are inversion, observation change, deletion and registry re-pin. This plan names each place in the task block. Any other change stops the task.
- **Frozen value.** `tests/m3/m2_operation_stream.json` keeps its hash. Until Q2 merges forward, it is the SP pin `bcbf72c2…e08e370b`; after Q2, the new hash of the Q2 vault note. No MP1 task changes it (D:2289). A red of `m2_operation_stream_matches_baseline` in an MP1 task stops the task.
- **GPU lock and headed batches (refinement R9).** Headed browser runs take the one GPU lock through `/home/julien/Projects/TuRTS/target/mc-evidence/gate-runner/mc_headed_v4.sh` (sha256 `66c105e5904357c5916b4a015517a1542ea2f8f16ea0128b3d2051a554cec235`, checked 2026-11-12).
    - Each client task writes its pixel row files (driver, oracle, row JSON, plants). It passes `python3 scripts/qa/driver_rule.py tests/browser` in its lane.
    - The pixel rows of stage 1 run in batch B1 inside `I-PIX1`. The pixel rows of stage 2 run in batch B2 inside `I2-PIX`. A red in a batch goes back to the owner task of the row.
    - Only the tasks with a "Headed" count above 0 in the task table take the lock (26 headed runs, section 14.2).
- **Model tiers.** The Tier cell is "builder/reviewer": S is Sonnet 5.5, H is Haiku 5.5, O is Opus 5.5. Planned code goes to S or H. Reviews, `L0-EXIT` and the audit go to O.
- **Git.** Stage explicit paths. Never `git add -A`. Build each binary with its own `cargo build -p <crate>`. Builders do not edit `CLAUDE.md`, `.claude/agents/`, D, the MC design or the product spec; they put each doc change in their report for `REC`.
- **Records.** Root counts each dispatch on a board (`vault/MP1 Execution Board.md`; NOT TRACED that it exists). The record of each task: red, green, plant outputs, changed paths, commit, run count. A task that uses twice its estimate is reported; it is not halted (C391 item 7).
- **Stop and report, do not decide:** a frozen value changes; a step contradicts D; a plant has no red; an open decision of D section 2 or S 15 is needed; a lane-0 contract item must change (section 5.2).

## 2. Plan refinements of D (each needs Root's acceptance at gate 2)

| # | Refinement | Reason (evidence) | Cost if refused |
| --- | --- | --- | --- |
| R1 | Lane-0 parts that touch no `turts_sim` file and no file of an in-flight slice task start now: `L0-RAW`, `L0-RULES`, `L0-DBG`, `L-ROOM0`, `C0-PACE`, `E-GPU`. D:2304 puts lane 0a at E1. | The SP task table names no `turts_mod_runtime`, `plugin.rs`, `replica.rs` or `debug.rs` file (grep count 0, section 18). The S45 diff `8d6902c..d047433` touches none of `turts_client/src/lib.rs`, `debug.rs`, `replica.rs`, `turts_client_web/src/lib.rs`. | lane 0 ends 2 runs later; path unchanged (the slice to A3 dominates) |
| R2 | `L0-API1` takes `L0-RULES` as input and replaces the five exhaustive `SlotRelation` matches by `turts_rules::relation` (X10). | New variants `Ally`, `Enemy` break `orders.rs:1126`, `economy/combat.rs:313`, `core_adapter.rs:340`, `widgets/cursor.rs:430`, `:619` (grep). D:767 wants one function. | two copies of the relation rule |
| R3 | Lane 0 builds the whole Validation column of D 6.10, except rows V1 and V2 (`L0-API2`). Feature lanes then edit only `unbuilt.rs` in `turts_mod_runtime`. | D:1388-1393 freezes shapes in lane 0; ten feature lanes would otherwise share the validation files. | about 10 concurrent edits of the validation files |
| R4 | Lane 0 creates one stub file for each feature at each seam: export writers (`export_sites.rs`, `export_supply.rs`, `export_jobs.rs`, `export_effects.rs`, `export_match.rs`), behaviour handlers (`L0b-ORD`), sim system slots (`L0-SIM`), client source stubs (`L0-CORE`). | D:1388-1393 ("writers empty"); the conflict list of section 3.4 | the conflict list grows by about 40 pairs |
| R5 | UnitIndex site 6 (`export.rs:299`) moves to `L0-WIRE`; the parse at `combat.rs:265` moves to `L-DMG`; hot site 3 (`gather.rs:1206`) moves from lane 0b to `L-OBS`. | export.rs is the wire lane's file; `combat.rs` waits for A2c and A3 (D:2304); `gather.rs` waits for A2b2 (D:2316-2317) | lane 0b waits for A2b2 (offset 15) |
| R6 | `L0b-ORD` also builds the engine order source (`Engine` through `apply_order`, the order-log source). | `L-CON1`, `L-STO`, `L-RLY` all need it (D:737-751). | three lanes build it |
| R7 | The sim parts of X11 and X10 go to `L-MATCH-SIM` (`simulation-core-engineer`). `L-MATCH-SRV1` (`backend-engineer`) keeps the room and builds the server-side `LobbyState` writer of X12. | D:2203 (sim owner); D:257, D:258 | one seat edits two crates |
| R8 | Merge-after rule (section 1) for the client tasks, `L-TGT` (A3, `L-PWR`), `L-FX-WIRE` and `L-STO` (A2b2), `PRES-ART`, `UI2-RESULT`, `UI2-CONN`, `C2-CONTENT`. | Without it the path is 69 runs; with it, 52 (section 18). | +17 runs on the path |
| R9 | Pixel rows run in headed batches B1 and B2. | CEO direction: pixel rows late and batched; one GPU lock. | about 12 more headed sessions |
| R10 | Integration branch `mp1/integration` with forward merges only. | SP section 1 forbids other merges into the slice branch. | MP1 and the slice share one tree; the slice REG tests MP1 code |
| R11 | The Countdown state moves from X11b rest (`L-MATCH-SRV2`) to `L-LOBBY2-SRV` (X12 rest). | The countdown serves only the lobby; the stage-2 lobby chain is then parallel to the surrender chain. | +3 runs in stage 2 |
| R12 | Open point 11 (a pack check that each building can be placed): dropped from MP1, moved to M4 with yardstick item CO03 (`later:M4`, D:2850). | CON1 and GOAL1 show that `frontier` buildings place; the map rules are M4 work. | `L0-API2` +1 to 2 runs |
| R13 | Open point 12 (replica cost): moved to M7 with the recorded numbers (`MEAS-EXIT` records native `apply_delta` p50 and p99). | Measured 0.029 ms (400 units) and 0.363 ms (5,000 units) for one clone; `replace_keyed` 0.247 ms (`docs/research/mp1/regate-performance.md:16`). MP1 runs 2 x 200 units. | `L0-WIRE` +1 to 2 runs |
| R14 | The `Credited` effect and the "+N" float move from X14 rest to stage 1 (`L-FX-WIRE`, `L-FX-PRES`). | GOAL1 (stage 1) needs the first "+300" (D:2461, plant (e)); X14 stage 1 lists only `Fired`, `Destroyed`, flash and death (D:260, D:2347). Design question Q3. | GOAL1 cannot go green in stage 1 |
| R15 | Rows STO1 and ALR2 are split into a stage-1 part and a stage-2 part. | Design questions Q1 and Q2. | STO1 or ALR2 cannot go green in stage 1 |
| R16 | No MP1 task edits the fault registry in stage 1. The plants run in the lane and are recorded. `I2-PIX` registers the MP1 set once. | The registry files are one hot file (`tests/m2/refutations.json`, SP:147). | a shared hot file for 20 tasks |
| R17 | The sim gate of D 6.7.5 (D:900-905) is built in lane 0 (`L0-SIM`): the sim inserts a default sim `MatchConfig` (team = slot, start tick 1) unless the host removes it for a lobby policy. | D:2521 (section 14 risk "the lobby stops the instruments") puts the `Immediate` default in lane 0. The count of test harnesses that tick the sim without a server is NOT TRACED; the default form changes none of them. | every sim test harness inserts the config |

## 3. Dependency graph

The table below is the single source. The script reads it. Stage `0` is lane 0, `1` stage 1, `2` stage 2, `slice` a remaining slice task (inputs and runs from SP section 3; S1, Q1a, G1 and A1a are merged and are not listed). "Runs" is the point estimate; "Range" is low-high. "Headed" counts the runs that take the GPU lock.

<!-- mp1-tasks:start -->
| Task | Stage | Builder | Reviewer | Tier | Inputs | Merge after | Runs | Range | Headed | Rows | Scope | Hot files |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| S45 | slice | bevy-rendering-engineer | ux-designer | - | - | - | 1 | 1-2 | 1 | - | - | interaction/, widgets/, visuals/, client_shell/, core_adapter.rs, support/mod.rs |
| S3 | slice | simulation-core-engineer | technical-director | - | - | - | 1 | 1-1 | 0 | - | - | movement/ |
| D0 | slice | (MC lane) | (MC) | - | - | - | 0 | 0-0 | 0 | - | - | - |
| D0p | slice | backend-engineer | simulation-core-engineer | - | D0 | - | 2 | 2-2 | 0 | - | - | replay.rs, gather.rs, orders/, economy-test.rs |
| S2 | slice | simulation-core-engineer | technical-director | - | D0p | - | 2 | 2-2 | 0 | - | - | orders/ |
| Q1b | slice | qa-lead | simulation-core-engineer | - | S3 | - | 2 | 2-2 | 0 | - | - | movement/, sim-Cargo.toml |
| A1b | slice | backend-engineer | qa-lead | - | D0p, S2, S3 | - | 4 | 4-4 | 1 | - | - | replay.rs, turts_audit/, session.rs, economy-test.rs |
| A2a | slice | simulation-core-engineer | qa-lead | - | Q1b, S3 | - | 4 | 4-4 | 0 | - | - | movement/ |
| A1c | slice | backend-engineer | qa-lead | - | A1b | - | 2 | 2-2 | 1 | - | - | replay.rs, turts_audit/, economy-test.rs |
| A1d | slice | backend-engineer | qa-lead | - | A1c | - | 2 | 2-2 | 0 | - | - | pack-tests-common/ |
| A2b1 | slice | simulation-core-engineer | qa-lead | - | A2a, A1c, S2 | - | 3 | 3-3 | 0 | - | - | movement/, orders/, gather.rs |
| A2b2 | slice | simulation-core-engineer | qa-lead | - | A2b1 | - | 2 | 2-2 | 0 | - | - | movement/, orders/, gather.rs, views.rs |
| A2c | slice | simulation-core-engineer | qa-lead | - | A2b2, Q1b | - | 4 | 4-4 | 0 | - | - | orders/, combat.rs, transaction.rs, views.rs |
| Q2 | slice | qa-lead | technical-director | - | A2c | - | 3 | 3-3 | 0 | - | - | - |
| A4 | slice | simulation-core-engineer | performance-architect | - | A2b2 | Q2 | 2 | 2-2 | 0 | - | - | movement/ |
| A3 | slice | simulation-core-engineer | performance-architect | - | A4 | Q2 | 3 | 3-3 | 0 | - | - | movement/, grid.rs, combat.rs, orders/ |
| REG | slice | qa-lead | (Root) | - | Q2, A3, S45, A1d | - | 2 | 2-2 | 2 | - | - | - |
| TDN | slice | technical-director | (Root) | - | REG | - | 1 | 1-1 | 0 | - | - | - |
| L-ROOM0 | 0 | backend-engineer | network-engineer | S/O | - | - | 3 | 2-4 | 0 | - | X11b, X12 | room.rs |
| L0-RAW | 0 | mod-runtime-engineer | mod-builder | S/O | - | - | 2 | 1-2 | 0 | - | X0, X19 | version_table.rs, content_mc_validation.rs |
| L0-RULES | 0 | simulation-core-engineer | technical-director | S/O | - | - | 2 | 2-3 | 0 | - | X0, X10 | Cargo.toml, boundary.sh |
| L0-DBG | 0 | bevy-rendering-engineer | performance-architect | S/O | - | - | 2 | 1-2 | 1 | DBG1 | X0 | client-lib.rs, debug.rs, replica.rs, web-lib.rs |
| C0-PACE | 0 | game-designer | player | H/O | - | - | 1 | 1-1 | 0 | - | X15 | probes/pacing/ |
| E-GPU | 0 | qa-lead | performance-architect | S/O | - | - | 1 | 1-1 | 1 | - | X20 | mp1.md |
| L0-API1 | 0 | mod-runtime-engineer | mod-builder | S/O | L0-RAW, L0-RULES, S2, S3, S45, D0p | - | 3 | 3-4 | 0 | - | X0, X19, X10 | schema.rs, version_table.rs, unbuilt.rs, orders/, combat.rs, replay.rs, session.rs, core_adapter.rs, cursor.rs |
| L0-API2 | 0 | mod-runtime-engineer | mod-builder | S/O | L0-API1 | - | 3 | 2-4 | 0 | VAL1 | X0, X19 | mp1_validation.rs |
| L0-SIM | 0 | simulation-core-engineer | performance-architect | S/O | S2, S3, D0p | - | 4 | 3-5 | 1 | - | X0, X0e | plugin.rs, production.rs, transaction_m3.rs, orders/, transaction.rs |
| L0-WIRE | 0 | network-engineer | backend-engineer | S/O | S2, S3, D0p, L0-DBG | - | 4 | 3-5 | 0 | - | X0, X19 | messages.rs, protocol_plugin.rs, events.rs, orders/, export.rs, session.rs, replica.rs, join.rs, ingest.rs |
| L0-CORE | 0 | bevy-rendering-engineer | ux-designer | S/O | S45, L0-API1 | - | 2 | 2-3 | 0 | - | X0, X5 | interaction/, widgets/ |
| L0-EXIT | 0 | technical-director | mod-builder | O/O | L0-API1, L0-WIRE, L0-RULES, L0-SIM, L0-CORE | - | 1 | 1-2 | 0 | - | X0, X19 | - |
| L0b-ORD | 0 | simulation-core-engineer | technical-director | S/O | A1c, L0-SIM, L0-EXIT | - | 2 | 2-3 | 0 | - | X0 | orders/, orderlog.rs |
| L-MATCH-SIM | 1 | simulation-core-engineer | backend-engineer | S/O | L0-EXIT | - | 3 | 3-4 | 0 | - | X11, X10 | plugin.rs, events.rs, match_state.rs |
| L-MATCH-SRV1 | 1 | backend-engineer | network-engineer | S/O | L-ROOM0, L-MATCH-SIM | - | 5 | 4-7 | 0 | IMM1, MAT1/s1 | X11b, X12 | session.rs, main.rs, config.rs, stats.rs, outbound.rs, room.rs, mods/standoff/, unbuilt.rs |
| L-LOBBY-C1 | 1 | network-engineer | web-platform-engineer | S/O | L0-EXIT | L-MATCH-SRV1 | 3 | 3-5 | 1 | LOB1/s1, LEAVE1/s1 | X12 | join.rs, rejoin.rs, actions.rs, client-main.rs, web-lib.rs, client_shell/ |
| L-PAY | 1 | simulation-core-engineer | qa-lead | S/O | L0-EXIT, A1d | - | 3 | 3-5 | 0 | PAY1, PAY2 | X7 | production.rs, turts_audit/, unbuilt.rs |
| L-INC | 1 | simulation-core-engineer | qa-lead | S/O | L-PAY | - | 2 | 1-2 | 0 | INC1 | X26 | unbuilt.rs |
| L-DMG | 1 | simulation-core-engineer | qa-lead | S/O | L0-EXIT | - | 2 | 2-3 | 0 | DMG1 | X2 | combat.rs, unbuilt.rs |
| L-TGT | 1 | simulation-core-engineer | performance-architect | S/O | L-DMG, L-MATCH-SIM, A1c | A3, L-PWR | 4 | 3-5 | 0 | TGT1, TGT2 | X2 | combat.rs, orders/, unbuilt.rs |
| L-CON1 | 1 | simulation-core-engineer | qa-lead | S/O | L0b-ORD, L-PAY | - | 6 | 5-8 | 0 | CON1, PRD1 | X4 | orders/, production.rs, server-orders.rs, unbuilt.rs |
| L-PWR | 1 | simulation-core-engineer | qa-lead | S/O | L-CON1, L-DMG | - | 3 | 2-4 | 0 | PWR1 | X6 | production.rs, combat.rs, construction.rs, unbuilt.rs |
| L-DES | 1 | simulation-core-engineer | qa-lead | S/O | L-CON1 | - | 3 | 2-4 | 0 | DES1 | X1 | transaction_m3.rs, production.rs, construction.rs, unbuilt.rs |
| L-OBS | 1 | simulation-core-engineer | performance-architect | S/O | A2b2, L-CON1, L0-API2 | A3 | 3 | 2-4 | 0 | OBS1 | X0e, X4 | gather.rs, mp1.md |
| L-STO | 1 | simulation-core-engineer | network-engineer | S/O | L0b-ORD, L-MATCH-SRV1 | A2b2 | 2 | 2-3 | 0 | - | X24 | orders/, gather.rs, unbuilt.rs |
| L-FX-WIRE | 1 | network-engineer | bevy-rendering-engineer | S/O | L-DMG | A2b2 | 2 | 1-2 | 0 | - | X14 | combat.rs, export.rs, ingest.rs, gather.rs |
| L-FX-PRES | 1 | bevy-rendering-engineer | performance-architect | S/O | L0-EXIT | L-FX-WIRE | 3 | 2-4 | 0 | FX1/s1 | X14 | visuals/ |
| UI-PLACE | 1 | bevy-rendering-engineer | ux-designer | S/O | L0-EXIT | L-CON1 | 5 | 4-6 | 0 | CON2, GHO1 | X5 | interaction/, visuals/, boundary.sh, cursor.rs |
| UI-HINT | 1 | bevy-rendering-engineer | ux-designer | S/O | L0-EXIT | UI-PLACE | 3 | 2-3 | 0 | - | X25 | interaction/, debug.rs |
| UI-HUD | 1 | bevy-rendering-engineer | ux-designer | S/O | L0-EXIT | L-PAY, L-CON1, L-PWR | 5 | 4-6 | 0 | HUD5, HUD6/s1, KEY1 | X6, X7 | widgets/ |
| UI-ALERT | 1 | bevy-rendering-engineer | ux-designer | S/O | L0-EXIT | L-PWR, L-STO, L-PAY, L-CON1 | 4 | 3-5 | 0 | ALR2/s1 | X15 | feedback.rs, audio.rs, interaction/ |
| UI-RESULT | 1 | bevy-rendering-engineer | ux-designer | S/O | L0-EXIT | L-LOBBY-C1 | 2 | 2-3 | 0 | RES1/s1 | X13 | widgets/, client_shell/ |
| C-UNITS | 1 | game-designer | mod-builder | S/O | L0-API2, C0-PACE | - | 3 | 3-4 | 0 | - | X15, X24, X26 | mods/frontier/ |
| C-UI | 1 | ux-designer | player | S/O | C-UNITS | - | 3 | 2-3 | 0 | - | X15, X25 | mods/frontier/ |
| I-NC1 | 1 | qa-lead | mod-runtime-engineer | S/O | L0-API1, C-UNITS | - | 2 | 1-2 | 0 | NC1 | X20 | support/mod.rs |
| C1-FINAL | 1 | game-designer | mod-builder | S/O | C-UI, L-DMG, L-TGT, L-PAY, L-INC, L-CON1, L-DES, L-PWR, L-OBS, L-STO, L-MATCH-SRV1, UI-HINT, UI-HUD, UI-ALERT, UI-RESULT, UI-PLACE, L-FX-PRES, A3 | - | 2 | 1-2 | 0 | - | X15, X0e | mods/frontier/, mp1.md |
| I-HEAD | 1 | qa-lead | network-engineer | S/O | C1-FINAL, L-LOBBY-C1 | - | 3 | 2-4 | 0 | ORDW1, STO1/s1 | X20 | mp1.md |
| I-PIX1 | 1 | qa-lead | bevy-rendering-engineer | S/O | C1-FINAL, L-LOBBY-C1 | - | 4 | 3-5 | 4 | GOAL1 | X20 | tests/browser/ |
| REG2-1 | 1 | qa-lead | technical-director | S/O | I-PIX1, I-HEAD, I-NC1, L0-API2, L0-DBG | - | 2 | 2-3 | 2 | REG2/s1 | X22 | - |
| C1-PLAYER | 1 | player | (none) | O/- | REG2-1 | - | 1 | 1-1 | 1 | - | X21 | - |
| C1-PERSON | 1 | qa-lead | (person) | S/- | C1-PLAYER | - | 1 | 1-2 | 1 | - | X21 | - |
| L-CON3 | 2 | simulation-core-engineer | qa-lead | S/O | C1-PERSON | - | 2 | 2-3 | 0 | CON3 | X4b | construction.rs, orders/, unbuilt.rs |
| L-REP | 2 | simulation-core-engineer | qa-lead | S/O | C1-PERSON | - | 3 | 3-4 | 0 | REP1 | X8 | orders/, combat.rs, unbuilt.rs |
| L-RLY | 2 | simulation-core-engineer | network-engineer | S/O | C1-PERSON | - | 3 | 2-4 | 0 | RLY1, STO1/s2 | X9, X24 | orders/, gather.rs, unbuilt.rs |
| L-MATCH-SIM2 | 2 | simulation-core-engineer | backend-engineer | S/O | C1-PERSON | - | 3 | 3-4 | 0 | - | X11c, X13 | match_state.rs, events.rs, orders/, unbuilt.rs |
| L-MATCH-SRV2 | 2 | backend-engineer | network-engineer | S/O | L-MATCH-SIM2, L-LOBBY2-SRV | - | 3 | 3-4 | 0 | MAT1/s2 | X11b, X11c | session.rs, room.rs, config.rs, server-orders.rs, mods/standoff/ |
| L-LOBBY2-SRV | 2 | backend-engineer | network-engineer | S/O | C1-PERSON | - | 3 | 2-4 | 0 | LEAVE1/s2 | X12, X11b | room.rs, session.rs, unbuilt.rs |
| L-LOBBY2-UI | 2 | bevy-rendering-engineer | ux-designer | S/O | L-LOBBY2-SRV | - | 4 | 3-6 | 1 | LOB1/s2, LOB2 | X12 | client_shell/, actions.rs, join.rs |
| UI2-RESULT | 2 | bevy-rendering-engineer | ux-designer | S/O | C1-PERSON | L-MATCH-SRV2 | 2 | 2-3 | 0 | RES1/s2 | X13 | widgets/ |
| UI2-CONN | 2 | bevy-rendering-engineer | ux-designer | S/O | C1-PERSON | L-MATCH-SRV2 | 2 | 2-3 | 0 | SUR1, ALR2/s2 | X11c | client_shell/, feedback.rs |
| UI2-HUD | 2 | bevy-rendering-engineer | ux-designer | S/O | L-CON3, L-RLY | - | 3 | 2-4 | 0 | HUD6/s2 | X4b, X9 | widgets/, interaction/ |
| UI2-SMART | 2 | bevy-rendering-engineer | ux-designer | S/O | L-REP, UI2-HUD | - | 2 | 2-3 | 0 | SMT1 | X8, X9 | interaction/, cursor.rs |
| FX2 | 2 | bevy-rendering-engineer | performance-architect | S/O | C1-PERSON | - | 3 | 3-4 | 0 | FX1/s2 | X14 | visuals/, audio.rs |
| C2-SOUND | 2 | game-designer | player | S/O | FX2 | - | 2 | 2-3 | 0 | AUD2 | X17, X14 | mods/frontier/ |
| PRES-MEASURE | 2 | bevy-rendering-engineer | performance-architect | S/O | C1-PERSON | - | 2 | 2-2 | 2 | - | X16 | mp1.md |
| PRES-MODELS | 2 | bevy-rendering-engineer | mod-runtime-engineer | S/O | PRES-MEASURE | - | 5 | 4-6 | 0 | - | X16 | visuals/, Cargo.toml, mp1_validation.rs |
| PRES-ART | 2 | bevy-rendering-engineer | ux-designer | S/O | PRES-MEASURE | PRES-MODELS | 4 | 3-8 | 0 | MDL1 | X16 | mods/frontier/ |
| UI2-MC | 2 | bevy-rendering-engineer | ux-designer | S/O | C1-PERSON | - | 4 | 3-6 | 0 | MCX1 | X18 | widgets/, client_shell/, feedback.rs |
| WEB-NOGPU | 2 | web-platform-engineer | bevy-rendering-engineer | S/O | C1-PERSON | - | 2 | 1-2 | 0 | NOGPU1 | X18 | web-lib.rs |
| C2-CONTENT | 2 | game-designer | mod-builder | S/O | L-RLY, L-REP, L-CON3, L-MATCH-SIM2 | L-MATCH-SRV2, UI2-RESULT | 2 | 2-3 | 0 | - | X15 | mods/frontier/ |
| I2-L | 2 | web-platform-engineer | backend-engineer | S/O | L-MATCH-SRV2 | - | 1 | 1-1 | 1 | - | X20 | mp1.md |
| I2-PIX | 2 | qa-lead | bevy-rendering-engineer | S/O | C2-CONTENT, C2-SOUND, PRES-ART, UI2-MC, WEB-NOGPU, L-LOBBY2-UI, UI2-SMART, UI2-CONN, UI2-RESULT, I2-L | - | 4 | 3-5 | 4 | - | X20 | tests/browser/ |
| MEAS-EXIT | 2 | qa-lead | performance-architect | S/O | I2-PIX | - | 2 | 2-3 | 2 | VIS3 | X20 | mp1.md |
| REG2-2 | 2 | qa-lead | technical-director | S/O | I2-PIX | - | 2 | 2-3 | 2 | REG2/s2 | X22 | - |
| AUDIT | 2 | technical-director | (none) | O/- | REG2-2 | - | 2 | 2-2 | 0 | - | X23 | - |
| PT3-PLAYER | 2 | player | (none) | O/- | REG2-2 | - | 1 | 1-1 | 1 | - | X21 | - |
| PT3-PEOPLE | 2 | qa-lead | (people) | S/- | PT3-PLAYER, MEAS-EXIT | - | 1 | 1-2 | 1 | PT3 | X21 | - |
| REC | 2 | qa-lead | (Root) | H/O | PT3-PEOPLE, AUDIT | - | 1 | 1-1 | 0 | - | X22 | mp1.md |
<!-- mp1-tasks:end -->

### 3.1 Critical path and start windows

Script: `python3 -I docs/research/mp1/probes/plan/mp1_plan_check.py docs/superpowers/plans/2026-11-12-mp1-playable-skirmish.md docs/superpowers/specs/2026-11-12-mp1-playable-skirmish-design.md; echo exit=$?` (exit 0; output in section 18).

**Critical path: 52 runs from now**, plus the D0 wait:
D0p → S2 → A1b → A1c → A2b1 → A2b2 → A2c → Q2 → A4 → A3 → `L-TGT` → `C1-FINAL` → `I-PIX1` → `REG2-1` → `C1-PLAYER` → `C1-PERSON` → `L-RLY` → `UI2-HUD` → `UI2-SMART` → `I2-PIX` → `REG2-2` → `PT3-PLAYER` → `PT3-PEOPLE` → `REC`.

- The first 25 runs are slice runs up to A3 (merged after Q2). C1 ends at offset 35, `REG2-2` at 49, `PT3-PEOPLE` at 51.
- Lane 0 exits at offset 10 (`L0-EXIT`). The near-critical MP1 chain is `L0-EXIT` → `L0b-ORD` → `L-CON1` → `L-PWR` → `L-TGT`: 1 run of slack (`L-CON1` slack 1, `L-PWR` slack 1). A 2-run overrun of `L-CON1` puts MP1 on the path.
- Start windows (earliest start; latest start that does not move the path): `UI-PLACE` 10-20, `UI-HINT` 10-22, `UI-HUD` 10-20, `UI-ALERT` 10-21, `UI-RESULT` 10-23, `L-FX-PRES` 10-22, `L-LOBBY-C1` 10-22, `L-FX-WIRE` 12-23, `L-STO` 18-23, `C-UNITS` 10-19, `I-NC1` 13-29. Root can start a client task late to cut its rebase work.

### 3.2 Waves and lanes

Wave = topological level. "Lanes" = the largest number of builds that run at one time inside the wave span, MP1 and slice.

| Wave | Offset | Max lanes (MP1, slice) | MP1 tasks ([slice]) |
| --- | --- | --- | --- |
| W0 | 0-3 | 9 (6, 3) | `L-ROOM0`, `L0-RAW`, `L0-RULES`, `L0-DBG`, `C0-PACE`, `E-GPU`; [S45, S3, D0] |
| W1 | 0-3 | 9 (6, 3) | [D0p, Q1b] |
| W2 | 2-7 | 5 (3, 2) | [S2, A2a] |
| W3 | 4-8 | 5 (3, 2) | `L0-API1`, `L0-SIM`, `L0-WIRE`; [A1b] |
| W4 | 7-10 | 5 (4, 1) | `L0-API2`, `L0-CORE`; [A1c] |
| W5 | 9-13 | 13 (11, 2) | `L0-EXIT`, `C-UNITS`; [A1d, A2b1] |
| W6 | 10-16 | 13 (11, 2) | `L0b-ORD`, `L-MATCH-SIM`, `L-PAY`, `L-DMG`, `C-UI`, `I-NC1`; [A2b2] |
| W7 | 12-21 | 11 (10, 1) | `L-MATCH-SRV1`, `L-INC`, `L-CON1`, `L-FX-WIRE`; [A2c] |
| W8 | 10-24 | 13 (11, 2) | `L-LOBBY-C1`, `L-PWR`, `L-DES`, `L-STO`, `L-FX-PRES`, `UI-PLACE`; [Q2] |
| W9 | 10-17 | 13 (11, 2) | `UI-HINT`, `UI-HUD`, `UI-ALERT`, `UI-RESULT`; [A4] |
| W10 | 22-25 | 4 (3, 1) | [A3] |
| W11 | 13-27 | 10 (9, 1) | `L-TGT`, `L-OBS`; [REG] |
| W12 | 25-28 | 3 (2, 1) | `C1-FINAL`; [TDN] |
| W13 | 27-31 | 3 (2, 1) | `I-HEAD`, `I-PIX1` |
| W14-W16 | 31-35 | 1 | `REG2-1`, `C1-PLAYER`, `C1-PERSON` |
| W17 | 35-39 | 11 (11, 0) | `L-CON3`, `L-REP`, `L-RLY`, `L-MATCH-SIM2`, `L-LOBBY2-SRV`, `FX2`, `PRES-MEASURE`, `UI2-MC`, `WEB-NOGPU` |
| W18 | 37-42 | 8 | `L-MATCH-SRV2`, `L-LOBBY2-UI`, `UI2-HUD`, `C2-SOUND`, `PRES-MODELS` |
| W19 | 35-43 | 11 | `UI2-RESULT`, `UI2-CONN`, `UI2-SMART`, `PRES-ART`, `I2-L` |
| W20 | 38-40 | 8 | `C2-CONTENT` |
| W21-W25 | 43-52 | 1 to 2 | `I2-PIX`, then `MEAS-EXIT` with `REG2-2`, then `AUDIT` with `PT3-PLAYER`, `PT3-PEOPLE`, `REC` |

The peak of 11 MP1 lanes at offsets 10-12 comes from client tasks that start at their earliest offset. With the start windows of 3.1, Root can hold the peak at 6 to 7 MP1 lanes and the path does not move. Five of the peak lanes use the `bevy-rendering-engineer` seat.

### 3.3 Dependencies read from D, and inferred

- Read from D: E1 inputs of lane 0a (D:2304); E2 inputs of lane 0b and of the order lanes (D:2305); A3 before content with large footprints (X0e, D:244, D:1419-1425); A1d before PAY1, PAY2, CON3, DES1, REP1, INC1 (D:2421-2422, D:2533; inferred: the ledger oracle of A1d is the "turts_audit" that the rows need); the replica test before the end-tick step (D:2582-2590); point 21 before the seat release (`docs/research/mp1/recheck6-rigour.md:30`); stage 2 only after C1 passes (D:60, D:2366).
- Inferred by the producer: every other edge of the task table, from the files and the rows. The near-critical chain `L-CON1` → `L-PWR` comes from PWR1, which tests the constructor's `Rate` (D:2443).

### 3.4 Conflict list: files that two concurrent tasks touch

The script prints 112 concurrent pairs (section 18). They fall into these groups. "Forward merge" means: the slice task merges first, and the MP1 owner resolves the conflict when Root merges the slice forward.

| File | Concurrent tasks | Order or split |
| --- | --- | --- |
| `turts_mod_runtime/src/unbuilt.rs` | 16 feature tasks of stages 1 and 2 | One line per feature; each line names its owner task (section 5.3). A task deletes only its own lines. The later merge rebases; the resolution is the union of deletions. The reviewer counts the lines with `rtk proxy grep -c` against section 5.3. |
| `turts_sim/src/orders/` | lane 0: `L0-API1`, `L0-SIM`, `L0-WIRE`; stage 1: `L-CON1`, `L-STO`, `L-TGT`; stage 2: `L-CON3`, `L-REP`, `L-RLY`, `L-MATCH-SIM2`; slice A2b1, A2b2, A2c, A3 | Lane 0 merges in the order `L0-API1`, `L0-SIM`, `L0-WIRE`; each later task rebases (different hunks: literals and relation arms; `validate_target`; the order writers). Feature lanes edit only their own behaviour file (stubs of `L0b-ORD`). Slice tasks: forward merge. |
| `turts_sim/src/economy/combat.rs` | `L0-API1`, `L-DMG`, `L-FX-WIRE`, `L-PWR`, `L-TGT`; slice A2c (`:154`), A3 (`:111-117`) | Order: `L0-API1` → `L-DMG` → (`L-FX-WIRE`, `L-PWR`; the later rebases) → `L-TGT` (merges after A3). Slice: forward merge. |
| `turts_sim/src/economy/gather.rs` | `L-FX-WIRE`, `L-STO`, `L-OBS`; slice A2b1, A2b2 | All three merge after A2b2. `L-OBS` owns the approach-point and index edits. `L-STO` puts `NearestNode` in `orders/`; `L-FX-WIRE` reads the existing delivery event. If either needs `gather.rs`, it rebases on `L-OBS`. |
| `turts_server/src/session.rs` | `L0-API1` (test literals), `L0-WIRE` (bucket first, `Welcome.start`), `L-MATCH-SRV1`; slice A1b (tests `:1053-1059`, SP:571) | Order: `L0-API1` → `L0-WIRE` → `L-MATCH-SRV1`. The room logic lives in `room.rs`. A1b: forward merge. |
| `turts_audit/src/replay.rs` | `L0-API1` (struct literals); slice A1b, A1c | If A1c merges forward first, the file is gone: take the deletion. |
| `turts_sim/src/economy/transaction.rs` | `L0-SIM` (index insert and remove near `:710`, `:747`); slice A2c (`:736`) | Forward merge. |
| `production.rs`, `economy/construction.rs` | `L-DES`, `L-PWR` (both after `L-CON1`) | `L-DES` merges first (death path); `L-PWR` rebases (rate only). |
| `turts_interaction/src/` | `UI-PLACE`, `UI-HINT`, `UI-ALERT` | Own modules (`placement.rs`, `hints.rs`, `alerts.rs`); `L0-CORE` adds the shared dispatch arms. |
| `turts_client/src/widgets/` | `UI-HUD`, `UI-RESULT`; stage 2 `UI2-HUD`, `UI2-MC`, `UI2-RESULT` | Own source files from `L0-CORE`; the shared dispatch arms exist. Rebase on a residual hunk. |
| `turts_client/src/visuals/` | `L-FX-PRES`, `UI-PLACE`; stage 2 `FX2`, `PRES-MODELS` | Own files (`effects.rs`, `placement.rs`, `models.rs`). |
| `client_shell/`, `feedback.rs` (stage 2) | `L-LOBBY2-UI`, `UI2-CONN`, `UI2-MC` | Rebase; `UI2-CONN` merges first (it adds alert triggers that `UI2-MC` keeps). |
| `mods/frontier/` (stage 2) | `C2-CONTENT`, `C2-SOUND`, `PRES-ART` | Own subdirectories (`match/`, `interface/`; `sounds/`, `fx/`; `assets/models/`, `visuals/`). `manifest.ron` is shared: one export line each; rebase. |
| `docs/measurements/mp1.md` | 8 tasks | One section heading per task; append only. |

### 3.5 Coverage check

The same script checks: every row part of D section 12 has exactly one owner, with a stage that agrees with the D stage lists (D:2424-2430) and the splits of section 13.2; every scope item X0 to X26 of D section 4 has a task; every open point of D section 15 has a closing entry (section 11); every input exists; no cycle.

- `--self-test` applies six mutations (a row part unowned, a row part owned twice, a cycle, X0e dropped, open point 21 dropped, a stage-2 owner marked stage 1). Each gives RED; the unmutated plan gives GREEN (section 18).

## 4. Gate sets

Fast gate for one task, in the lane, after the commit. SP section 4 defines F-RUST, F-WASM, F-QA, F-FROZEN, F-MOVE and F-REF; use `mp1/integration` in place of `pre-mp1/integration`. MP1 adds:

- **F-PACK** (tasks that touch the mod API or a package): `for p in mods/*/; do cargo run -q -p turts_pack -- "$p"; echo "$p exit=$?"; done`. Each repository package prints `exit=0`; `frontier` prints only the refusal named in its task.
- **F-PROC** (server tasks): `cargo test -p turts_server --release --locked --test <file> -- --ignored --test-threads=1; echo exit=$?` for each process test that the task names.
- **F-DRV** (tasks that add `tests/browser/*.js`): `python3 scripts/qa/driver_rule.py --self-test; echo exit=$?`, then `python3 scripts/qa/driver_rule.py tests/browser; echo exit=$?`. Each prints `exit=0`.
- **F-PLAN** (any edit of this plan): the script of section 3.1 and its `--self-test`, each `exit=0`.

Slow gate for one merged tree (Root, on `mp1/integration`): SP section 4 "Slow gate", items 1 to 7 and 9, one time for each merged tree that differs from a tree that passed it (C391 item 3). Add the MP1 process tests that exist on the tree (F-PROC for every MP1 test file). Headed batches B1 and B2 run inside `I-PIX1` and `I2-PIX`; REG2 runs inside `REG2-1` and `REG2-2`.

## 5. Lane 0: the contract

Lane 0 is the contract for every later lane. It has 13 tasks (stage `0` in the table): six that start now (`L-ROOM0`, `L0-RAW`, `L0-RULES`, `L0-DBG`, `C0-PACE`, `E-GPU`), five after E1 (`L0-API1`, `L0-API2`, `L0-SIM`, `L0-WIRE`, `L0-CORE`), the freeze review `L0-EXIT`, and lane 0b (`L0b-ORD`, after E2).

### 5.1 What is frozen at `L0-EXIT`

1. The mod API 0.2.4: every type, field, default and enum variant of D 6.10 (rows A1 to V4), the edition rows T1 to T4 as names, `REASON_KEYS` (49 keys, D:1603-1615), `CURSOR_CONTEXTS` (9, D:1577), `STATE_KEYS` (22, D:1616-1625), `ENGINE_API_VERSION = "0.2.4"` and `RELEASED_ENGINE_APIS` with `"0.2.4"` (D:2280-2283), every version-table row with its detection kind, the `UNBUILT_FEATURES` lines and their text (D:1390).
2. Protocol 5: `NET_PROTOCOL_VERSION = 5`, every record of D 6.8 (D:1298-1321) and D 6.7.13 (D:1259-1272) with its fields and channel, the registration order of the five new messages after `ResyncRequest` (D:1336-1339), the nine new `CommandRejectReason` keys.
3. `turts_rules`: the public signatures of `placement::check`, `relation`, the damage formula, the charge, refund and heal difference functions, the site-health target, the shortage effect.
4. Sim seams: the new `CommittedTick` fields, the sim `MatchConfig` type and its gate, the empty system slots and their schedule order, the `UnitIndex` API.
5. Client seams: the `Core::step` split by input kind, the dispatch stubs for every new `Value`, `Source`, `Predicate`, `PrimaryField` and `ItemField` variant.
6. Lane 0b, frozen at its own merge: the behaviour handler table, the stub file of each new behaviour, `OrderSource::Engine`.

Not frozen: writers, systems and handlers (the feature lanes fill them), validation messages of `L0-API2` until it merges, content numbers (S:807).

### 5.2 How a later lane asks for a change

1. The lane stops the step and writes a change request on the board: `CR-<n>`, the item of 5.1, the reason, the D line, and the tasks that the conflict list (3.4) names for that file.
2. `technical-director` rules on it in one run (Opus). The CEO decides when the request touches a decision of D section 2.
3. The owner seat of the item makes the change in one commit on `mp1/integration`: `mod-runtime-engineer` for 5.1 item 1, `network-engineer` for item 2, `simulation-core-engineer` for items 3, 4 and 6, `bevy-rendering-engineer` for item 5.
4. A change request never bumps the protocol or the API again: MP1 has one step of each (X19, D:268). The change re-runs VAL1 (item 1) or the ORDW1 cases of `L0-WIRE` (item 2), and every open lane rebases.
5. Budget: 0 to 3 runs, counted under gates (section 14.1).

### 5.3 `UNBUILT_FEATURES` lines and the task that removes each

| Lines (6.10 rows) | Removed by |
| --- | --- |
| `exports.combat` (A1), `armour` (U1), `weapon.damage_type` (U2) | `L-DMG` |
| `acquire` (U4) | `L-TGT` |
| `kind: Supply` (S1), `provides`, `consumes` (U5), `*.when_short` (U6) | `L-PWR` |
| `payment: ChargeOverTime` (P1) | `L-PAY` |
| `income` (U7) | `L-INC` |
| `constructor` (K1), `placement` (K2), `build_area` (K3), production with a static output (K4), `Construct` (O1), `ContentAtPoint` (O5), `needs: Constructor` (O6) | `L-CON1` |
| `health without removal` (K7), `removal with a queue` (K8), `cancel_refunds.destroyed` (K5) | `L-DES` |
| `start_order` (M3), `spawn_order` (U8) with `ground_rally: Ignore` | `L-STO` |
| `exports.match` (A2) with `AllSeated(countdown_secs: 0)` and `KeyUnitsLost` (M1, M2) | `L-MATCH-SRV1` |
| `AllSeated` above 0, `AllSeatedReady` (M2) | `L-LOBBY2-SRV` |
| `DisconnectedFor` (M1), `Surrender` (O9) | `L-MATCH-SIM2` |
| `CancelConstruction` (O2) | `L-CON3` |
| `SetRally` (O3), `rally` (O7), `ground_rally: ApplyFirst` (U8), smart `Ground` with `needs: Producer` (O6) | `L-RLY` |
| `repairer` (R1), `Repair` (O4), `OwnDamaged`, `needs: Repairer` (O6) | `L-REP` |
| `hints`, `hint_fallback` (H14), `start_selection` (H18), `Value::Hint`, `Predicate::HintShown` | `UI-HINT` |
| `alerts[].kind`, `clear_after` (H6), `units` (H5), `select_on_click`, `{unit}` (H17), `focus` (H22), `sound_when` (H21), `alert_sound_gap_secs` (H20), triggers `ConstructionComplete`, `SupplyShort`, `SupplyRestored`, `InsufficientFunds`, `GathererIdle` (H4) | `UI-ALERT` |
| triggers `SlotDisconnected`, `SlotReconnected` (H4), `Value::Connection` | `UI2-CONN` |
| `world.placement`, `world.build_area_ring`, `world.gap_ring`, `world.progress_bar` (H11), cursor contexts `place` (H2), `Predicate::Placing` | `UI-PLACE` |
| `role` (H19), `hotkey` (O8), `card` (O10), `Value::Supply`, `ConstructionQueue`, `Sites`, `SelectionTypes`, `Notices`, `Predicate::SupplyShort`, `ItemHas`, item fields `Role`, `State`, `Paid`, `Refund`, `Badge` | `UI-HUD` |
| `Value::Outcome`, `OutcomeReason`, `MatchClock`, `Predicate::MatchOver`, `OutcomeIs`, `OutcomeReasonIs`, `{slot}` (H17, T4), `leave_match` (H23) | `UI-RESULT` |
| `Value::MatchStats` (H7) | `UI2-RESULT` |
| item fields `BuildTimeNow`, `SupplyChange`, `CausesShortage`, primary fields `Activity`, `Cargo`, `Armour` (H8, H9), `PrimaryHasValue` (H10), `world.cargo_bar`, `world.health_bar.scale_to_footprint` (H12) | `UI2-HUD` |
| `world.rally` (H11), cursor contexts `repair`, `rally` (H2) | `UI2-SMART` |
| `exports.visual_effects` (A3), visual `effects` with `fire`, `death` (V3, V4) | `L-FX-PRES` |
| `hit`, `hit_reduced`, `damaged` (V4), effect `sound` (V3), `world.float_text` sound (H11), `effects_voices`, `audible_range` (H16) | `FX2` |
| `Button.font`, `Button.size` (H13) | `UI2-MC` |
| `models` (V1), `model` (V2) | `PRES-MODELS` |

`L0-EXIT` checks with a script that every row name of 6.10 is either built in lane 0 or has exactly one line here (step 2 of `L0-EXIT`).

## 6. Lane 0 tasks

### Task L-ROOM0: room module and the connection-end trace (now)

**Seat:** `backend-engineer` (S). **Review:** `network-engineer` (O): does each cell of the state table hold, and does the trace show the real end path?

**Branch / inputs:** `mp1/l-room0` from `pre-mp1/integration` at `85f34cb`; no input. **Design:** D 6.7.2, 6.7.3, 6.7.5 (table D:982-988), 6.7.6, section 15 point 21 (D:2615-2619), section 14 row "The server detects a disconnect late" (D:2528).

**Files:**
- Create `crates/turts_server/src/room.rs`: pure types and functions, no Bevy system. `RoomState { Boot, Lobby, Countdown, Running, Ended }`; `SeatTable` (seat count = player-slot count, `crates/turts_sim/src/slots.rs:7`; lowest open seat; lowest free palette index; label `Player N`; release); `judge_join(state, id, has_live_connection)` for each cell of D:982-988; the rule "a second `Hello` on a connection with a seat is ignored" (D:957). Add `mod room;` in `crates/turts_server/src/main.rs`.
- Create `crates/turts_server/tests/connection_end.rs` (`#[ignore]` process test).

- [ ] **Step 1: Trace (open point 21).** Find the code that ends a `ClientOf` at the netcode timeout and at `Unlink`. Lead (verified): today the only handler of a session end is the observer `on_session_removed` on `On<Remove<Session>>` (`crates/turts_server/src/session_diagnostics.rs:384`), registered only with `--stats` (`crates/turts_server/src/stats.rs:104-105`); the refused-connection path triggers `Unlink` (`session.rs:710-719`). Which lightyear schedule despawns the entity is NOT TRACED. Write the process test: release `turts_server` with `muster` and `--stats`, one headless client with `--player-id`, then a SIGKILL of the client, then a clean exit in a second run. Read the private observer file for `session_authority_removed` and its time after the kill. Expected: one record within 11 s for each case. Record L for a release build in `docs/measurements/mp1.md` (section "Disconnect notice L, native release"). Write the trace note in the task report: the hook that `L-MATCH-SRV1` uses for the seat release (an observer on the removal of a seat marker, or the despawn of the `ClientOf`), with `path:line`.
- [ ] **Step 2: Red.** Unit tests of `room.rs` for each table cell, the lowest-seat and lowest-colour rules, the release and the second `Hello`, with `todo!()` bodies. `cargo test -p turts_server --locked --bin turts_server room::; echo exit=$?` gives `exit=101`.
- [ ] **Step 3: Green.** Fill the bodies. Same command gives `exit=0`.
- [ ] **Step 4: Gate.** F-RUST (`turts_server`), F-PROC (`connection_end`).
- [ ] **Step 5: Plants.** (a) Remove `app.add_observer(session_diagnostics::on_session_removed)` (`stats.rs:105`): the trace test is red ("no removal record"). (b) The colour rule by seat index: the lowest-free-colour test is red. Restore each.

**Exit:** the trace note names the end hook; L (release) is recorded; the room tests are green. **Rows:** none. **Runs: 3 (2-4).**

### Task L0-RAW: RAW presence for the existing version-table rows (now)

**Seat:** `mod-runtime-engineer` (S). **Review:** `mod-builder` (O): does an explicit empty field now get the version refusal?

**Branch / inputs:** `mp1/l0-raw`; none. **Design:** D 6.10 "Version-table detection" (D:1469-1493); VAL1 plant (2) (D:2437); D:2284-2286.

**Files:** `crates/turts_mod_runtime/src/version_table.rs` (rows from `:52`; the refusal at `:96-108`, text "`{}` needs engine api {}; `{}` admits {version}"); `crates/turts_mod_runtime/src/content_mc_validation.rs:65` (`orders_present: !orders.is_empty() || !order_rules.is_empty()`, the parsed-value presence); create `crates/turts_mod_runtime/src/raw_presence.rs` and `crates/turts_mod_runtime/tests/raw_presence.rs`.

- [ ] **Step 1: Red.** A `^0.2.2` package that writes `orders: []` packs with exit 0 today (mod-builder gate probe, D:1480). Write the test that expects the refusal of its row. `cargo test -p turts_mod_runtime --locked --test raw_presence; echo exit=$?` gives `exit=101`.
- [ ] **Step 2: Green.** Parse each file a second time as `ron::Value` (ron 0.12.2, `Cargo.toml:15`, verify first) and walk its keys for every RAW row of 0.2.2 and 0.2.3. Variant rows keep VAR or COMBO detection (`ron::Value` drops variant names, D:1474).
- [ ] **Step 3: Gate.** F-RUST (`turts_mod_runtime` → `turts_pack`, `turts_server`, `turts_checks`), F-QA, F-PACK (every repository package requires `^0.2.3`, D:1480-1481; verify first).
- [ ] **Step 4: Plants.** (a) Back to the parsed value for the `orders` row: the explicit-empty test is red. (b) In `crates/turts_mod_runtime/tests/t2c_package_rejects.rs`, remove one row from the table: the t2c test of that row is red (D:2285-2286: no test turns into a no-op). Restore each.

**Exit:** explicit-empty packages are refused for every existing RAW row. `ENGINE_API_VERSION` stays `"0.2.3"` (`crates/turts_mod_api/src/lib.rs:52`); the bump is in `L0-API1`. **Rows:** VAL1 plant (2) closed in part (VAL1 is owned by `L0-API2`). **Runs: 2 (1-2).**

### Task L0-RULES: the crate `turts_rules` (now)

**Seat:** `simulation-core-engineer` (S). **Review:** `technical-director` (O): pure, no workspace dependency, one copy of each rule?

**Branch / inputs:** `mp1/l0-rules`; none. **Design:** D section 9 crate table (D:2194), "One copy of each rule" (D:2206-2210); 6.1 check order (D:467-483) and the pure function (D:491-496); 6.6 relation (D:757-767); 6.3 formula (D:551-556); 6.4 charge (D:626-632) and refunds (D:648-656); 6.5 heal (D:691-694); 6.2 shortage (D:523-528); row K2 `site_health` (D:1531).

**Files:** create `crates/turts_rules/` (`Cargo.toml`, `src/lib.rs`, `placement.rs`, `relation.rs`, `damage.rs`, `charge.rs`, `heal.rs`, `supply.rs`, `tests/`); workspace `Cargo.toml` members; `scripts/boundary.sh` rows for host and wasm32 (forbid every `bevy*` crate, every workspace crate, `wgpu`, `winit`) with fixture cases.

- [ ] **Step 1: Red.** Tests first: the check order (two failing causes give the first); squared distances only (a source test: no `sqrt`, `mul_add`, `sin`, `cos`, `atan` in `placement.rs`); the relation table `Same`, `Ally`, `Enemy`, `Other`, `Any` with two slots in each team; damage cells 100 x 29/100 = 29, 50 x 106/100 = 53, flat 5 on 100 x 29/100 = 24 (DMG1 plants (1), (3)); the charge sum equals the cost for a seeded set of costs up to `u64::MAX` and five rates; refund `floor(charged x n / d)`; the heal difference; the site-health target at progress 0 and at completion.
- [ ] **Step 2: Green.**
- [ ] **Step 3: Gate.** F-RUST; `cargo check -p turts_rules --target wasm32-unknown-unknown --locked; echo exit=$?`; `bash scripts/boundary.sh; echo exit=$?`; each new fixture row red with the forbid message and `exit=1`.
- [ ] **Step 4: Plants.** (a) A float multiplier in `damage.rs`: the 29/100 cell gives 28. (b) `relation` that returns `Enemy` for `Other`: the two-slot team test is red. Restore each.

**Exit:** the crate builds on host and wasm32; the boundary rows are red on their fixtures. **Rows:** none (DMG1, TGT1, CON2 use it). **Runs: 2 (2-3).**

### Task L0-DBG: the debug state on request (now)

**Seat:** `bevy-rendering-engineer` (S). **Review:** `performance-architect` (O): can the counter go red on today's tree?

**Branch / inputs:** `mp1/l0-dbg`; none. **Design:** D section 9 "The client debug state" (D:2243-2259); row DBG1 (D:2454); D 8.5 (D:2178-2181).

**Files:** `crates/turts_client/src/lib.rs:147` (`app.add_systems(Last, (rebuild_debug_state, exit_when_done).chain())`); `crates/turts_client/src/debug.rs:441` (`rebuild_debug_state`); `crates/turts_client/src/replica.rs:1180-1184` (`health_trace`); `crates/turts_client_web/src/lib.rs:166-170`, `:371-372` (`publish_debug`). The S45 diff touches none of these files.

- [ ] **Step 1: Red.** DBG1 on today's tree, which is its red build: a headless client at 400 units (`turts_server --synthetic-units 400`) for 600 frames; the build counter shows 600; the test expects 0. A 10-minute process test in combat: the trace count is above 64. Both `exit=101`.
- [ ] **Step 2: Green.** Build the debug state only on a harness request, at most 4 times each second, for the last presented frame; the web bridge copies it only on a change; keep the last 64 entries of `health_trace` for each unit. How the harness requests it (bridge getter or `--out`) is NOT TRACED; the builder reads the bridge first and states the form.
- [ ] **Step 3: Gate.** F-RUST, F-WASM. Headed (1 run): the MC browser rows that read `turts_debug` (SP slow set item 6, `--set ci-browser`) stay green.
- [ ] **Step 4: Plants.** (a) Prune the trace to 0 entries: the MC test of `OwnUnitDestroyed` that reads the last entry (`crates/turts_client/src/feedback.rs:643-647`) is red. (b) Build on every frame again: the counter test is red. Restore each.

**Exit:** DBG1 green with its recorded red. **Rows:** DBG1. **Runs: 2 (1-2).**

### Task C0-PACE: pacing scripts and open point 19 (now)

**Seat:** `game-designer` (H for the copy and the runs). **Review:** `player` (O): does the stage-1 hint order hold for a first-time player?

**Branch / inputs:** `mp1/c0-pace`; none. **Design:** D section 5 (D:364-369), open point 19 (D:2608-2612); `docs/research/mp1/recheck5-player.md:31`, `:56`.

**Files:** create `docs/research/mp1/probes/pacing/`: copy `pacing.py`, `fights2.py`, `match.py`, their outputs, from `/tmp/claude-1000/-home-julien-Projects-TuRTS/bf418660-b272-4bb3-8628-8d9ef2677d1d/scratchpad/mp1-pacing/`, and `hold5.py`, `hold5.out` from `.../scratchpad/regate-player/`; add `SHA256SUMS`.

- [ ] **Step 1.** `python3 -I pacing.py new > new.check; cmp new.check new.out; echo exit=$?` prints `exit=0` (the copy reproduces).
- [ ] **Step 2.** Run the model on the stage-1 hint order (H4 starts when H2 is done) and on the alternative of D:2611 (H4 starts when the Power Station is complete). Record the longest hold for each order.
- [ ] **Step 3.** Write the recommendation in the report: change the order, or accept the hold. `C-UI` writes the chosen order. Root records the choice on the board.

**Exit:** point 19 has a measured answer before C1. **Rows:** none. **Runs: 1 (1-1).**

### Task E-GPU: two headed clients on one GPU (now)

**Seat:** `qa-lead` (S). **Review:** `performance-architect` (O).

**Branch / inputs:** `mp1/e-gpu`; none. **Design:** section 14 row "One GPU for a two-client match" (D:2526).

- [ ] **Step 1.** Through the headed wrapper, run two headed clients of the MC build for 10 minutes with combat. Verify first: which MC package gives two armed slots (lead: `forge`, turret against drones, `vault/M3 Playtest Guide.md`). Record frame p50 and p95 of each window and the wait for the GPU lock.
- [ ] **Step 2.** Write the numbers, the machine and the GPU in `docs/measurements/mp1.md` (section "Two headed clients on one GPU, MC build"). No threshold (S 15).

**Exit:** the numbers exist before stage-1 pixel rows are batched. **Rows:** none. **Runs: 1 (1-1). Headed: 1.**

### Task L0-API1: the mod API 0.2.4 as types

**Seat:** `mod-runtime-engineer` (S). **Review:** `mod-builder` (O): can a modder write each 0.2.4 field from the table and the RON examples alone?

**Branch / inputs:** `mp1/l0-api1`; `L0-RAW`, `L0-RULES`, E1 (S2, S3, S45, D0p). **Design:** D 6.10 (D:1435-1742), the key lists (D:1600-1625), the defaults and RON examples (D:1627-1693), "The API bump" (D:2280-2286), `UNBUILT_FEATURES` (D:1388-1393), 6.7.4 match data (D:859-896).

**Files:**
- `crates/turts_mod_api/src/schema.rs`, `orders.rs`, `interface.rs`, `assets.rs`, `lib.rs` (`:52` `ENGINE_API_VERSION`, `:55` `RELEASED_ENGINE_APIS`); create `keys.rs` (`REASON_KEYS`, `CURSOR_CONTEXTS`, `STATE_KEYS`).
- `crates/turts_mod_runtime/src/version_table.rs` (one row for each 6.10 row, with RAW, VAR, COMBO or ED); create `unbuilt.rs` (section 5.3).
- Struct literal sites of `UnitDef`, `Placement`, `CancelRefunds`, `ProductionDef`, `InterfaceDef`, `AlertDef`, `WeaponDef` (none derives `Default`, `schema.rs:333`, `:408`, `:438`). At `85f34cb` they are in `turts_sim/src` (`orders.rs`, `registry.rs`, `command_m3.rs`, `economy/production.rs`, `machine.rs`, `combat.rs`, `research.rs`, `boot_m3.rs`), `turts_audit/src/replay.rs`, `turts_server/src/economylog.rs`, `session.rs` (tests), `turts_client/src`, and the test files (grep `UnitDef {`). **Verify first** after S2.
- The five exhaustive `SlotRelation` matches (`orders.rs:1126`, `economy/combat.rs:313`, `turts_client/src/core_adapter.rs:340`, `widgets/cursor.rs:430`, `:619`) call `turts_rules::relation` with team = slot (R2, X10).

- [ ] **Step 1: Red.** Tests: `acquire` in a `^0.2.3` package is refused with the text of D:1458-1460; a `^0.2.4` package is refused for the API without the bump (VAL1 plant (6)); a `^0.2.4` package with `constructor` is refused with "`constructor` is not built in this engine build". Each `exit=101`.
- [ ] **Step 2: Green.** The types with `#[serde(default)]` on new fields of existing structs (D:1461-1463); the bump in the same commit as the first 0.2.4 row (D:2284); the edition; every version-table row; the `UNBUILT_FEATURES` lines of section 5.3, each with its owner task in a comment.
- [ ] **Step 3: Gate.** F-RUST, F-WASM, F-QA, F-FROZEN, F-PACK (every repository package `exit=0`).
- [ ] **Step 4: Plants.** (a) Remove one version-table row: its `^0.2.3` package packs with exit 0. (b) Remove the bump: the `^0.2.4` package is refused for the API. Restore each.

**Exit:** 5.1 item 1 exists in code; the existing packages pack. **Rows:** none (VAL1 is `L0-API2`). **Runs: 3 (3-4).**

### Task L0-API2: the validation column and VAL1

**Seat:** `mod-runtime-engineer` (S). **Review:** `mod-builder` (O): does each red package give the exact text of its row?

**Branch / inputs:** `mp1/l0-api2`; `L0-API1`. **Design:** D 6.10 Validation and Error-text columns, the "did you mean" rule (D:1450-1456, `crates/turts_mod_runtime/src/parse.rs:23-29`, verify first), 6.7.4 pack checks (D:881-896), D 8.1 does not apply (V1, V2 go to `PRES-MODELS`); row VAL1 (D:2437).

**Files:** create `crates/turts_mod_runtime/src/mp1_validation.rs` (one function for each 6.10 group); `content_m3_validation.rs:374-376`, `:418`, `:420-424` (the two lifted rules become COMBO rows K7, K8 behind `UNBUILT`; the `constructor` beside `producer` exemption at `:418`); `content_mc_validation.rs:1495-1519` (`is_engine_action` gains `leave_match`, H23); create `crates/turts_mod_runtime/tests/val1.rs` and `tests/fixtures/val1/` (one red package for each RAW row written and written empty, each VAR row, each COMBO row; a `^0.2.3` and `^0.2.4` pair for each ED row T1 to T4).

- [ ] **Step 1: Red.** The VAL1 package set on the `L0-API1` build: the explicit-empty, key, placeholder and reserved-variant packages pack with exit 0. `cargo test -p turts_mod_runtime --locked --test val1; echo exit=$?` gives `exit=101`.
- [ ] **Step 2: Green.** Each check and its text; the packer refuses `Q` above 65,536 (D:629) and a gap below the largest mobile diameter plus 0.2 (K2); a package with a constructor and no `open_sites_per_slot` (K6); `forge` and `muster` copies pack (H13 detection on the parsed `Button`, D:1476-1479).
- [ ] **Step 3: Gate.** F-RUST, F-QA, F-PACK.
- [ ] **Step 4: Plants.** VAL1 plants (1) to (6) (D:2437). Restore each.

**Exit:** VAL1 green with its reds. **Rows:** VAL1. **Runs: 3 (2-4).**

### Task L0-SIM: UnitId index, sim gate and system slots

**Seat:** `simulation-core-engineer` (S). **Review:** `performance-architect` (O): is the index used only for lookup, and do the slots sit where D says?

**Branch / inputs:** `mp1/l0-sim`; E1 (S2, S3, D0p). **Design:** D section 9 "UnitId index in lane 0" (D:2212-2230), "String ids become indices" (D:2232-2236), D 6.7.5 gate (D:900-905), D 6.2 computation (D:519-522), D 6.7.8 outcome check place (D:1113), section 14 row 2 (D:2521).

**Files:**
- Create `crates/turts_sim/src/unit_index.rs`. Use it at site 1 `economy/production.rs:282`, site 2 `economy/transaction_m3.rs:635` and `:524` (a reverse set where ordered targets are set and cleared, or keep the scan and record its cost, D:2219-2220), site 7 `validate_target` (`orders.rs:3056`, `:3085`; verify first after S2). Insert at spawn (`economy/transaction.rs` spawn path near `:710`; the map spawn in `boot_m3.rs`), remove at `economy/transaction.rs:747`.
- Footprint and supply resource ids to indices at catalog install.
- Create `crates/turts_sim/src/match_config.rs`: the sim `MatchConfig { tick_hz, start_tick, teams, settings }` (D:1197-1200) and the run condition on each `SimSet` (`plugin.rs:31-44`), with the default insert of R17.
- `crates/turts_sim/src/plugin.rs`: empty systems for supply (start of the economy phase), income, construction placement, acquisition grids, and the outcome check (after the death path, before `Collect`), each in its own new file.

- [ ] **Step 1: Red.** Tests: a lookup of a removed id gives no entity; with no `MatchConfig` the tick does not advance; the slot order test (outcome check after removals). Each red on the base.
- [ ] **Step 2: Green.** Verify first how many test harnesses build `SimPlugin` without a server; state the count in the report.
- [ ] **Step 3: Gate.** F-RUST, F-FROZEN, F-REF (selector on the changed files). Early test (D:2521): the 31 ignored process tests (`grep -c '#\[ignore'` for each file of `crates/turts_server/tests`), hd1 and T7 (`bash scripts/t7.sh`) on this build; none waits for a lobby. Headed 1 (T7 under the lock, verify first whether T7 needs it).
- [ ] **Step 4: Plants.** (a) The gate without the default insert: `muster` with no client has 0 ticks after 3 s (IMM1 red build 1). (b) The index not updated at removal: the lookup test is red. Restore each.

**Exit:** 5.1 item 4 exists; the frozen stream is unchanged. **Rows:** none. **Runs: 4 (3-5).**

### Task L0-WIRE: protocol 5

**Seat:** `network-engineer` (S). **Review:** `backend-engineer` (O): does each record of D 6.8 exist with its channel and viewer class?

**Branch / inputs:** `mp1/l0-wire`; E1, `L0-DBG` (replica.rs order). **Design:** D 6.8 (D:1291-1393), D 6.7.13 (D:1257-1272), section 15 point 14 (D:2582-2590), row ORDW1 (D:2477).

**Files:**
- `crates/turts_net_protocol/src/messages.rs` (`Welcome` `:26-31`, `last_failure` `:136`, `CommandRejectReason::as_str` `:225-264`), `economy.rs`, `lib.rs:13` (4 to 5).
- `crates/turts_net_lightyear/src/protocol_plugin.rs:26-43` (five messages after `ResyncRequest`), `channels.rs:5-6` (comment).
- `crates/turts_sim/src/economy/events.rs:138` (`CommittedTick`: the new fields, among them the order-change list), the order writers in `orders/` (verify first).
- `crates/turts_server/src/export.rs` (`unit_orders` as changes `:190-195`; `WeaponRecord.ready_tick` `:174-177`; `owner_of` by binary search `:299`, the view is sorted, `crates/turts_sim/src/world.rs:61`; the per-viewer visible set); create `export_sites.rs`, `export_supply.rs`, `export_jobs.rs`, `export_effects.rs`, `export_match.rs` with empty writers.
- `crates/turts_server/src/session.rs`: the bucket first (`:403-424`); `Welcome.start` under `Immediate` (`:268-283`).
- `crates/turts_client/src/replica.rs` (merge in chain order `:237-239`; `orders_cleared`; today's replacement `:523-525`), `ingest.rs:422-436`, `join.rs` (`Welcome.start`).

- [ ] **Step 1: Red.** ORDW1 idle case: 200 idle units for 100 ticks, `unit_orders` bytes of each delta are 0; today each delta carries the full table, so it is red (ORDW1 plant (2) is today's behaviour). The point-14 test: the replica applies a `Snapshot` that it did not request, and keeps `IgnoredStale` for `tick <= last_applied` (`replica.rs:217-221`, `:258-275`).
- [ ] **Step 2: Green.** If the point-14 test is red, make the replica accept the `Snapshot` (D:2589-2590).
- [ ] **Step 3: Gate.** F-RUST, F-WASM, F-QA, F-FROZEN, F-REF (`session.rs`, `export.rs`). Binding note (recheck5-network-host :36): no process test depends on refusals that skip the bucket; a red there stops the task.
- [ ] **Step 4: Plants.** ORDW1 plants (1) no `orders_cleared` (case A), (2) full table (idle bytes), (3) Snapshot merged into the old table (case B), (4) `WeaponRecord` in each tick (case C weapon part). Restore each.

**Exit:** 5.1 item 2 exists; ORDW1 cases A, B and the weapon part of C are green here; `I-HEAD` runs the whole row. **Rows:** none owned. **Runs: 4 (3-5).**

### Task L0-CORE: `Core::step` split and client stubs

**Seat:** `bevy-rendering-engineer` (S). **Review:** `ux-designer` (O).

**Branch / inputs:** `mp1/l0-core`; S45, `L0-API1`. **Design:** D section 9 crate table row `turts_interaction` (D:2199), D 7.5 core items (D:1910-1911).

**Files:** `crates/turts_interaction/src/lib.rs:137` (`pub fn step(`; S45 splits the file, verify first); create `placement.rs`, `hints.rs`, `alerts.rs` (stubs); `crates/turts_client/src/widgets/` dispatch arms for every new 6.10 `Value`, `Source`, `Predicate`, `PrimaryField`, `ItemField` variant, each in a stub file of its owner task (section 5.3).

- [ ] **Step 1.** Split `Core::step` by input kind, with no behaviour change. Proof: the `cargo test -p turts_interaction --locked -- --list` names are equal before and after (`cmp`), and the interaction tests are green.
- [ ] **Step 2.** The stubs return none; a 0.2.3 layout renders as before.
- [ ] **Step 3: Gate.** F-RUST, F-WASM.
- [ ] **Step 4: Plant.** Swap two input-kind arms: an interaction test is red (verify first which; lead: `crates/turts_interaction/tests/km3.rs`). Restore.

**Exit:** 5.1 item 5 exists. **Rows:** none. **Runs: 2 (2-3).**

### Task L0-EXIT: lane-0 freeze review

**Seat:** `technical-director` (O). **Review:** `mod-builder` (O): the sample mod.

**Branch / inputs:** none (a review of `mp1/integration`); `L0-API1`, `L0-WIRE`, `L0-RULES`, `L0-SIM`, `L0-CORE`. **Design:** D section 10 lanes (D:2310-2315), D 6.8 (D:1388-1393), section 5 of this plan.

- [ ] **Step 1.** A script reads D 6.10 and the code: each row name is built in lane 0 or has exactly one line of section 5.3. Each record of D 6.8 exists with its field names. Output: `docs/research/mp1/lane0-freeze.md` with the commit and the frozen list.
- [ ] **Step 2.** `mod-builder` packs the sample mod `docs/research/mp1/probes/regate-tide/tide/` against the frozen build: the only refusals are `UNBUILT` lines.
- [ ] **Step 3.** A plant for the script: delete one line of section 5.3 from `unbuilt.rs`: the script is red.

**Exit:** the freeze note exists; change requests follow 5.2 from here. **Runs: 1 (1-2).**

### Task L0b-ORD: behaviour handler table and engine orders

**Seat:** `simulation-core-engineer` (S). **Review:** `technical-director` (O).

**Branch / inputs:** `mp1/l0b-ord`; E2 (A1c), `L0-SIM`, `L0-EXIT`. **Design:** D section 9 "Behaviour handler table" (D:2277-2278), D 6.5 "Engine orders" (D:737-751); R6.

**Files:** `crates/turts_sim/src/orders/` (verify first after S2: `apply_order` at `orders.rs:2610` on the base); stub files for `Construct`, `CancelConstruction`, `SetRally`, `Repair`, `Surrender`; `OrderSource::Engine`; `crates/turts_server/src/orderlog.rs` (source field); `crates/turts_server/src/outbound.rs:26-31` (an engine order writes nothing into the `(slot, seq)` result list).

- [ ] **Step 1: Red.** Tests: a fake player envelope for an engine order is refused (TD part B5); an engine order gives no `OrderResult` and costs no token; its refusal shows in `last_failure` (`messages.rs:136`); the order log line has the source `Engine` and no seq.
- [ ] **Step 2: Green.**
- [ ] **Step 3: Gate.** F-RUST, F-FROZEN, F-REF (`orders/`).
- [ ] **Step 4: Plant.** Route an engine order through the player admission: the token test is red. Restore.

**Exit:** 5.1 item 6 exists. **Rows:** none. **Runs: 2 (2-3).**

## 7. Stage 1 tasks

Stage 1 is the thin loop (D:57-60). Each block names the rows that it makes green; the plants are the red builds of the row in D section 12, at the D line given. Pixel rows run in batch B1 (`I-PIX1`).

### Task L-MATCH-SIM: outcomes, teams and the end of the match (sim)

**Seat:** `simulation-core-engineer` (S). **Review:** `backend-engineer` (O). **Inputs:** `L0-EXIT`. **Design:** D 6.7.8 (D:1113-1134), 6.6 (D:757-771), 6.7.9 first bullets (D:1138-1141); X11, X10.

**Files:** create `crates/turts_sim/src/match_state.rs` (`MatchState { connections, outcomes }`, `KeyUnitsLost` counts per slot changed at spawn, completion and death, no world scan); the outcome-check slot of `L0-SIM`; `StopReason::MatchOver` (`economy/events.rs:183-197`) after the `Collect` chain of tick t (`plugin.rs:151`, `:157-160`); teams from the sim `MatchConfig`; `turts_server/src/export_match.rs` (public, on change).

- [ ] Red: sim tests for MAT1 (a) and (b) logic and the draw rule. Green. Gate: F-RUST, F-FROZEN. Plants: the draw rule gives `Victory` to the lower slot index ((b) red); a defeat with no clause ((i) logic). The three-slot sim test of MAT1 (`slot_defeated`): verify first, `SLOT_COUNT` is the constant 2 (`crates/turts_sim/src/slots.rs:7`); design question Q4.

**Rows:** none owned (MAT1/s1 is `L-MATCH-SRV1`). **Runs: 3 (3-4).**

### Task L-MATCH-SRV1: the room, the start step, Ended and the stage-1 hooks

**Seat:** `backend-engineer` (S). **Review:** `network-engineer` (O): does the stage-1 trace of recheck5-network-host §2 hold end to end?

**Branch / inputs:** `mp1/l-match-srv1`; `L-ROOM0`, `L-MATCH-SIM`. **Design:** D 6.7.3 to 6.7.12 stage-1 parts (D:805-1255), X11b stage 1 (D:257), X12 server writer (D:258), rows IMM1 (D:2474), MAT1 (D:2447).

**Files:** `crates/turts_server/src/room.rs` (wire in), `session.rs` (Hello by state, `:146-283`; the commands of a seated connection `:357-360`; after the end `:386-388`), `main.rs` (`:72`, `:78`, `:139-142`), `config.rs` (`after_match_secs`), `stats.rs` (`room_state`, `match_over_tick`, `deltas_after_end`), `outbound.rs` (`send_deltas` guard in Ended, `:51-84`), `command_script.rs`, `crates/turts_server/tests/help.rs` (reads `config.rs`, `:1-3`); create `mods/standoff/` (stage-1 form, D:1249-1254) and `scripts/mp1-serve.sh` (D:1173-1178).

- [ ] **Step 1: Red.** IMM1 on the `L0-SIM` build with a lobby default: `muster` has 0 ticks. MAT1 (a) with `--start-now --command-script` on `standoff`: no `match_over_tick`.
- [ ] **Step 2: Green.** Boot to Lobby or Running (D:912); the start step in one exclusive system, steps 1 to 4 (D:918-928), with the tick-0 `Snapshot` built in the step, not through `SnapshotDue` (`session.rs:683-685`); the seat release on the hook that `L-ROOM0` names; `LobbyState { you, seats, start: Waiting }`; `match_not_started` before the start (bucket of the seat index, D:1044-1048); `match_over` after the end; Ended: the end-tick `Snapshot` on the reliable channel, then `MatchResult` (`stats: []` in stage 1; design question Q7), then no deltas; `LeaveMatch` only in Ended, kick after `KICK_DELAY` (`session.rs:30`); `The match is over`; the exit rule (D:1163-1169); `--start-now`, `--input-log`, the three `--stats` fields; `after_match_secs` in `server.ron` and `--help`. Verify first whether `turts_pack::load_package` parses the interface file (the `slot_colours` length at boot, D:878).
- [ ] **Step 3: Gate.** F-RUST, F-QA, F-PROC (every ignored test of `crates/turts_server/tests`, the 31 of D:2521), hd1, F-PACK (`standoff`).
- [ ] **Step 4: Plants.** IMM1 plants (D:2474): the default policy a lobby; `--start-now` ignored; a version-table row missing. MAT1 stage-1 plants: the draw rule; the tick log has a tick after T. Restore each.

**Rows:** IMM1, MAT1/s1 (cases (a), (b), the three-slot sim test). **Runs: 5 (4-7).**

### Task L-LOBBY-C1: the client `Seated` phase and the stage-1 client hooks

**Seat:** `network-engineer` (S). **Review:** `web-platform-engineer` (O): does the browser keep the id from the seat on?

**Branch / inputs:** `mp1/l-lobby-c1`; `L0-EXIT`; merge after `L-MATCH-SRV1`. **Design:** D 6.7.7 (D:1054-1066), 6.7.12 client hooks (D:1229-1248), X12 stage 1 (D:258), 7.6 stage-1 lines (D:1925-1926, D:1936, D:1941-1942); rows LOB1 (D:2448), LEAVE1 (D:2475).

**Files:** `crates/turts_client/src/join.rs` (`Phase::Seated` at the first `LobbyState` with `you`, `advance_phase` `:107`; `:479`), `rejoin.rs` (`advance_to` `:306-311` counts `Seated` as joined), `actions.rs` (`on: { event, wait_ms }`; `leave`; `:107-118`), `main.rs` (`--rejoin` for a headless client, `:38-43`), the `--out` report fields (D:1243-1247, `end_snapshot_tick` on receipt), `crates/turts_client_web/src/lib.rs` (the save at `Seated`, `:259`), the shell screens (`client_shell/notices.rs` after S45, verify first).

- [ ] Red: LOB1 run 0 on today's client: no `lobby_states` entry. Green. Gate: F-RUST, F-WASM, F-PROC, headed 1 (the browser save at `Seated`, under the lock).
- [ ] Plants: LOB1 run 0 plants (the sim ticks before the second seat; the start step through `SnapshotDue` with two fixed steps) and run 4 plant (a seat bound to the player id); LEAVE1 plants (D:2475: no exit in Ended; the same `match_id`; the script restarts after a non-zero exit; an owner that did not leave gets `The match is over`; the revision-4 exit rule; the end `Snapshot` left out; no guard in Ended; `LeaveMatch` before Ended).

**Rows:** LOB1/s1 (run 0; run 4 in its stage-1 form), LEAVE1/s1. **Runs: 3 (3-5). Headed: 1.**

### Task L-PAY: charge over time and the production cancel

**Seat:** `simulation-core-engineer` (S). **Review:** `qa-lead` (O): does the ledger rule balance with `charged`?

**Inputs:** `L0-EXIT`, A1d. **Design:** D 6.4 (D:618-668); X7; rows PAY1 (D:2444), PAY2 (D:2466).

**Files:** `crates/turts_sim/src/economy/production.rs` (`done`, `total = duration_ticks x Q`, `try_ms_to_ticks` at `boot_m3.rs:591`; all costs or none; `WaitingForFunds`; owner unit id then queue order; coalesced `Charged`; `charged` on the job; the cancel refund from `charged`), `crates/turts_audit/src/` (the ledger rule in the A1b balances domain; verify first after A1d), `crates/turts_server/src/export_jobs.rs`.

- [ ] Red, green, gate (F-RUST, F-FROZEN, F-REF), plants: PAY1 (1) to (5), PAY2 (1), (2).

**Rows:** PAY1, PAY2. **Runs: 3 (3-5).**

### Task L-INC: free income

**Seat:** `simulation-core-engineer` (S). **Review:** `qa-lead` (O). **Inputs:** `L-PAY`. **Design:** D 6.10 row U7 (D:1523), section 5 (D:309); X26 (D-L, CEO to confirm).

**Files:** create `crates/turts_sim/src/economy/income.rs` (one ledger event for each period; `period_ms` to ticks at boot, rounding up, `boot_m3.rs:591-599`; only while complete and health above 0).

- [ ] Red, green, gate, plants INC1 (a) to (c) (D:2465).

**Rows:** INC1. **Runs: 2 (1-2).**

### Task L-DMG: damage table and armour

**Seat:** `simulation-core-engineer` (S). **Review:** `qa-lead` (O). **Inputs:** `L0-EXIT`. **Design:** D 6.3 formula (D:546-566); X2 damage; row DMG1 (D:2438).

**Files:** `crates/turts_sim/src/economy/combat.rs` (the modifier `:371-374`, the applied damage `:376`; the parse at `:265` becomes an index, R5); create `economy/damage.rs` (install with indices; calls `turts_rules`).

- [ ] Red: the two cells and the transposed table on a test package (not `frontier`). Green. Gate: F-RUST, F-FROZEN, F-REF, a source test that `combat.rs` calls no `ContentId::parse` in a tick system (regate-performance M4). Plants: DMG1 (1) f64 and f32, (2) axes swapped, (3) `flat` before the ratio.

**Rows:** DMG1 (the duels with `frontier` numbers are recorded by `C1-FINAL`; a failed duel is a retune, D:2438). **Runs: 2 (2-3).**

### Task L-TGT: acquisition, retaliation and hold

**Seat:** `simulation-core-engineer` (S). **Review:** `performance-architect` (O): does the counter show the cadence, and is the grid built once per tick?

**Inputs:** `L-DMG`, `L-MATCH-SIM`, A1c; merge after A3 and `L-PWR`. **Design:** D 6.3 (D:568-611); D section 9 sites 4 and 5 (D:2222-2225); rows TGT1 (D:2439), TGT2 (D:2476).

**Files:** `economy/combat.rs` (`choose_target` before the cooldown today, `:201`, `:229`; units without `acquire` keep it), the per-tick unit frame for `combat.rs:94` and `orders.rs:703` (verify first after A2c, A3), create `economy/acquire.rs` (one grid for each team; cell `max(2.0, smallest acquire.range / 2)`; `(t + unit_id) mod 4 = 0`; `Disable` stops acquisition too, PWR1 clause); the `cfg(test)` counter pattern of `crates/turts_sim/src/movement.rs:13-21` (in `movement/` after S3).

- [ ] Red, green, gate (F-RUST, F-FROZEN, F-REF), plants TGT1 (1) to (4), TGT2 (1) to (4) (test package: acquire 7 under a turret of range 8).

**Rows:** TGT1, TGT2. **Runs: 4 (3-5).**

### Task L-CON1: construction, held orders and sites

**Seat:** `simulation-core-engineer` (S). **Review:** `qa-lead` (O): can each cause go red alone, and does the cap make an order wait?

**Inputs:** `L0b-ORD`, `L-PAY`. **Design:** D 6.1 (D:402-505); X4, X5 server part; rows CON1 (D:2440), PRD1 (D:2464).

**Files:** `orders/` `Construct` stub (from `L0b-ORD`); create `economy/construction.rs` (the queue holds orders only; the two caps make an order wait; placement of held orders as engine orders in ascending constructor id, then queue order; the site entity with progress, charge and job through `L-PAY`; footprint blocks from its tick; `site_health`; the check through `turts_rules::placement::check`); `economy/production.rs` (exit points; a complete job waits for a free exit); `crates/turts_server/src/orders.rs` (`to_sim` for `ContentAtPoint`); `export_sites.rs` (`site`, `sites_completed`, `SiteProgress`, `ConstructionQueue`).

- [ ] Red, green, gate (F-RUST, F-FROZEN, F-REF), plants CON1 (1) to (6), PRD1 (a) to (d).

**Rows:** CON1, PRD1. **Runs: 6 (5-8).** Near-critical (section 3.1).

### Task L-PWR: supply and shortage effects

**Seat:** `simulation-core-engineer` (S). **Review:** `qa-lead` (O). **Inputs:** `L-CON1`, `L-DMG`. **Design:** D 6.2 (D:507-539); X6; row PWR1 (D:2443).

**Files:** create `economy/power.rs` (one computation at the start of the economy phase; `when_short` through `turts_rules`: `Rate` on producer and constructor, `Disable` on the weapon for fire and for today's targeting); `production.rs`, `construction.rs` (rate); `export_supply.rs`.

- [ ] Red, green, gate, plants PWR1 (1) to (4), with the second package `zeta:steam` and `Rate(1/3)`.

**Rows:** PWR1. **Runs: 3 (2-4).**

### Task L-DES: the one death path for units with health and queues

**Seat:** `simulation-core-engineer` (S). **Review:** `qa-lead` (O). **Inputs:** `L-CON1`. **Design:** D 6.4 "Destruction" (D:670-681); X1; row DES1 (D:2467).

**Files:** `economy/transaction_m3.rs:354` (`remove_unit`, called from `economy/combat.rs:405`); `production.rs`, `construction.rs`; create `crates/turts_checks/tests/sim_writers.rs` (static check: the only production despawn in `turts_sim` is `economy/transaction.rs:747`; it uses the syn scanner of `crates/turts_checks/tests/support/mod.rs`; no such check exists today, grep).

- [ ] Red, green, gate (F-RUST, F-FROZEN, F-REF), plants DES1 (1) to (6) (test package with `destroyed` 1/2 and `running` 3/4).

**Rows:** DES1. **Runs: 3 (2-4).**

### Task L-OBS: approach point, gap and the A3 measure

**Seat:** `simulation-core-engineer` (S). **Review:** `performance-architect` (O). **Inputs:** A2b2, `L-CON1`, `L0-API2`; merge after A3. **Design:** D 6.9 (D:1395-1433), row T1 (D:1713), section 14 rows 1 and 4 (D:2520, D:2523), open point 1; R5 (hot site 3).

**Files:** `crates/turts_sim/src/economy/gather.rs` (the approach point for edition 0.2.4; lead from the probe: `gather.rs:1126-1149`, `orders.rs:785-819`; verify first after A2b2; `find_unit` at `:1206` uses the index).

- [ ] Step 1 (early test, D:2523): the perf probe of perf H4 on the tree with A3; record the ratio in `docs/measurements/mp1.md`. Then red (today's tree: 0 harvester trips with a station 1 to 2 from the command centre), green, gate (F-RUST, F-FROZEN with the `base` stream unchanged, F-REF), plants OBS1 (1) to (3). Record the line count of the measure (open point 1: the estimate of 40 lines).

**Rows:** OBS1. **Runs: 3 (2-4).**

### Task L-STO: start orders of a placement and of a unit type

**Seat:** `simulation-core-engineer` (S). **Review:** `network-engineer` (O). **Inputs:** `L0b-ORD`, `L-MATCH-SRV1`; merge after A2b2. **Design:** D 6.5 "Start orders" (D:713-735); X24; row STO1 (D:2462).

**Files:** `orders/` (the engine order at the start tick and at the spawn tick; `NearestNode` by distance, then node index); the spawn of today gives a gatherer with no node and stage `Idle` (`economy/transaction.rs:710-719`).

- [ ] Red, green, gate, plants STO1 (a) to (e) on a test package (STO1 itself runs on `frontier` in `I-HEAD`).

**Rows:** none owned. **Runs: 2 (2-3).**

### Task L-FX-WIRE: fire, death and credit effects on the wire

**Seat:** `network-engineer` (S). **Review:** `bevy-rendering-engineer` (O). **Inputs:** `L-DMG`; merge after A2b2. **Design:** D 6.8 `Fired` and effects (D:1310-1311, D:1343-1346, D:1367-1373); R14.

**Files:** `economy/combat.rs:393-402` (stage `Fired` for every viewer group; today the shooter's slot only); `export_effects.rs`; `crates/turts_client/src/ingest.rs` (no effect from a resync batch; drop effects older than the presentation tick minus 4). `Credited`: verify first the name of the delivery event (D:1150, NOT TRACED).

- [ ] Red: FX1 (a) headless (window B gets no enemy `Fired`). Green, gate, plant FX1 (d) (kept deltas replay their effects).

**Rows:** none owned. **Runs: 2 (1-2).**

### Task L-FX-PRES: effect pool, tracer, flash, death, "+N"

**Seat:** `bevy-rendering-engineer` (S). **Review:** `performance-architect` (O). **Inputs:** `L0-EXIT`; merge after `L-FX-WIRE`. **Design:** D 8.3 (D:2148-2158), X14 stage 1, R14.

**Files:** create `crates/turts_client/src/visuals/effects.rs` (one pool of 256; the oldest instance is reused); the FX1 row files under `tests/browser/`.

- [ ] Red (headless pool counter), green, gate (F-RUST, F-WASM, F-DRV), plants FX1 (b) and the pixel plants for B1.

**Rows:** FX1/s1. **Runs: 3 (2-4).**

### Task UI-PLACE: placement mode, ghost and rings

**Seat:** `bevy-rendering-engineer` (S). **Review:** `ux-designer` (O). **Inputs:** `L0-EXIT`; merge after `L-CON1`. **Design:** D 7.2 (D:1772-1816), D 6.1 "One pure function" (D:491-501); X5; rows CON2 (D:2441), GHO1 (D:2463).

**Files:** `crates/turts_interaction/src/placement.rs` (the effect `Ghost { content, point, valid, reason }`; the footprint list rebuilt per applied delta, not per frame); `crates/turts_client/src/visuals/placement.rs`; `widgets/cursor.rs` (`place`, `forbidden`); `scripts/boundary.sh` (CON2: no placement function in `turts_client` other than the call); GHO1 row files.

- [ ] CON2 red, green, gate (F-RUST, F-WASM, F-DRV, boundary), plants CON2 (1) to (3); GHO1 plants (a) to (e) prepared for B1.

**Rows:** CON2, GHO1. **Runs: 5 (4-6).**

### Task UI-HINT: hint engine and the start selection

**Seat:** `bevy-rendering-engineer` (S). **Review:** `ux-designer` (O). **Inputs:** `L0-EXIT`; merge after `UI-PLACE`. **Design:** D 7.8 (D:1984-2019), 7.1 times (D:1766-1770); X25.

**Files:** `crates/turts_interaction/src/hints.rs` (evaluation once per applied delta plus timers; the selection rule; done through a pending site; `StepCurrent` reads steps only; `Selected`); `crates/turts_client/src/debug.rs` (`turts_debug.hints`: `click_ms`, `shown_ms`, `done_ms`, `site_ms`); `start_selection`.

- [ ] Red: GOAL1 plant (d) headless (the source evaluated only at the start: H2 never shows). Green. Gate. Plant (h) headless (`StepCurrent` through situation hints: more than 10 changes in 1 s). A counter test: hint evaluations per applied delta (regate-performance client-hint debt).

**Rows:** none owned (GOAL1 is `I-PIX1`). **Runs: 3 (2-3).**

### Task UI-HUD: HUD sources, row states, build menu keys, right-click cancel

**Seat:** `bevy-rendering-engineer` (S). **Review:** `ux-designer` (O). **Inputs:** `L0-EXIT`; merge after `L-PAY`, `L-CON1`, `L-PWR`. **Design:** D 7.3 (D:1818-1844), 7.4 (D:1846-1873), 7.5 (D:1875-1913), 7.10 stage-1 lines (D:2074-2085), section 9 client cost rule (D:2261-2275); rows HUD5 (D:2451), HUD6 part 1 (D:2469), KEY1 (D:2452).

**Files:** the stub source files of `L0-CORE`; `crates/turts_client/src/widgets/sources.rs` (build menu `:372-397`: the production `hotkey`, else the Produce or `Construct` key; constructor buildings after producer recipes); the queue row and entry right-click cancel (open point 16, section 11); `widgets/tree.rs` after S45 (no clones, `set_if_neq`; today `sync_widget_tree` at `widgets/mod.rs:79-90`, verify first).

- [ ] Red (HUD5 states in a scenario, headless where the text is a source value), green, gate (F-RUST, F-WASM, F-DRV), plants HUD5 (a) to (c), HUD6 (b), (d), KEY1 (a); KEY1 (b), (c) are packer refusals (`L0-API2`).

**Rows:** HUD5, HUD6/s1, KEY1. **Runs: 5 (4-6).**

### Task UI-ALERT: alert kinds, triggers and the sound gap

**Seat:** `bevy-rendering-engineer` (S). **Review:** `ux-designer` (O). **Inputs:** `L0-EXIT`; merge after `L-PWR`, `L-STO`, `L-PAY`, `L-CON1`. **Design:** D 7.9 (D:2021-2070); row ALR2 (D:2468).

**Files:** `crates/turts_client/src/feedback.rs` (state alerts with `clear_after`; `GathererIdle` from the idle gather stage; today `GathererWaiting` reads waiting orders only, `:815-825`; previous healths indexed by unit id, today a scan at `:618-632`; evaluation per applied delta); `audio.rs` (gap by priority; the live cap of two stays, `:795`); `crates/turts_interaction/src/alerts.rs` (`FocusAlert`).

- [ ] Red: ALR2 case A on today's rule (more than one sound in a window). Green. Gate. Plants ALR2 (a) to (i), except the connection alerts (ALR2/s2).

**Rows:** ALR2/s1. **Runs: 4 (3-5).**

### Task UI-RESULT: outcome and reason line

**Seat:** `bevy-rendering-engineer` (S). **Review:** `ux-designer` (O). **Inputs:** `L0-EXIT`; merge after `L-LOBBY-C1`. **Design:** D 7.6 result part (D:1949-1968), 6.7.7 step 5 (D:1031); X13 stage 1; row RES1 (D:2449).

**Files:** stub sources of `L0-CORE` for `Outcome`, `OutcomeReason`, `MatchClock`, `MatchOver`, `OutcomeIs`, `OutcomeReasonIs`, `{slot}`; the engine action `leave_match` (sends `LeaveMatch`, enabled only when the match is over); the "Leave" shell text.

- [ ] Red, green, gate. A test: when `MatchResult` arrives in a frame before the end `Snapshot`, no outcome shows until the replica tick is at or after `end_tick` (recheck5-network-host §3). Plant: the result screen without the reason (pixels, B1).

**Rows:** RES1/s1. **Runs: 2 (2-3).**

### Task C-UNITS: the `frontier` rules and map (stage 1)

**Seat:** `game-designer` (S). **Review:** `mod-builder` (O): can each field be written from 6.10 alone? **Inputs:** `L0-API2`, `C0-PACE`. **Design:** D section 5 (D:283-395), X15 stage 1 (D:2348), X24, X26.

**Files:** `mods/frontier/` (`manifest.ron`, `units/`, `resources/`, `production/`, `orders/`, `combat/`, `maps/twin_fields.ron`, `match/match.ron` with `AllSeated(countdown_secs: 0)` and `KeyUnitsLost`). Verify the ore positions against the ring of radius 12.5 (recheck5-player :29: 14.1 to 15.1 from GD 224-226, NOT TRACED in D).

- [ ] Write the package. Gate: `cargo run -p turts_pack -- mods/frontier; echo exit=$?` prints exactly one `UNBUILT` refusal of a stage-1 lane that has not merged (or `exit=0` when all merged).

**Rows:** none. **Runs: 3 (3-4).**

### Task C-UI: the `frontier` interface (stage 1)

**Seat:** `ux-designer` (S). **Review:** `player` (O). **Inputs:** `C-UNITS`. **Design:** D 7.2 to 7.10 texts, the Player path hint table (stage-1 data, no H3), 7.9 rules (D:2057-2070) without the two connection rules; X15, X25.

**Files:** `mods/frontier/interface/`, `mods/frontier/input/`. The `selection_priority` list (`crates/turts_mod_api/src/interface.rs:19`) puts the mobile units first, so a box with units leaves the Command Centre out (open point 20).

**Rows:** none. **Runs: 3 (2-3).**

### Task I-NC1: the no-content scan for 0.2.4

**Seat:** `qa-lead` (S). **Review:** `mod-runtime-engineer` (O). **Inputs:** `L0-API1`, `C-UNITS`. **Design:** row NC1 (D:2436), decision N1.

**Files:** `crates/turts_checks/tests/support/mod.rs` (kinds `:86-100`, `add_id` `:449-456`, exception table `:272-292`; S45 changed 9 lines, verify first), `no_content.rs:166`.

- [ ] Red: the literal `"muster_unit_damaged"` gives 0 hits today (D:2436). Green. Plants: one literal for each new kind.

**Rows:** NC1. **Runs: 2 (1-2).**

### Task C1-FINAL: `frontier` packs; duels recorded

**Seat:** `game-designer` (S). **Review:** `mod-builder` (O). **Inputs:** see the table (all stage-1 feature lanes and A3). **Design:** X0e (D:244), D section 5 results; open point 4.

- [ ] `turts-pack` on `mods/frontier` prints `exit=0`. A3 is merged: `git merge-base --is-ancestor pre-mp1/a3-grids mp1/integration; echo exit=$?` prints `exit=0`. The DMG1 duels with the `frontier` numbers run headless; the results go to `docs/measurements/mp1.md`; a failed ratio is a retune of S:807 numbers, recorded.

**Rows:** none. **Runs: 2 (1-2).**

### Task I-HEAD: the headless two-client match, ORDW1, STO1

**Seat:** `qa-lead` (S). **Review:** `network-engineer` (O). **Inputs:** `C1-FINAL`, `L-LOBBY-C1`. **Design:** X20 stage 1; rows ORDW1 (D:2477), STO1 runs 1 to 3 (D:2462); section 14 row 7 (D:2527).

- [ ] ORDW1 cases A, B, C in full (site, job and `Fired` at most 8 B); the bytes of each record kind to `docs/measurements/mp1.md`. STO1 runs 1 to 3 on `frontier`. The scripted headless match to its end: record the end time and the ore-out time.

**Rows:** ORDW1, STO1/s1. **Runs: 3 (2-4).**

### Task I-PIX1: stage-1 pixels driver, GOAL1, batch B1

**Seat:** `qa-lead` (S). **Review:** `bevy-rendering-engineer` (O). **Inputs:** `C1-FINAL`, `L-LOBBY-C1`. **Design:** X20 stage 1 (D:2349), row GOAL1 (D:2461), section 14 row 6 (D:2525).

**Files:** `tests/browser/mp1-*.js` (role lines; the driver reads no text and no bridge, QA0 A1; the oracle reads `turts_debug.hints`); `tests/browser/rows/mp1-*.json`; create `scripts/mp1-play.sh` (builds the release server and the web client, packs `frontier`, writes `target/mp1-play/server.ron`, prints the URL, runs `scripts/mp1-serve.sh`).

- [ ] Batch B1 under the lock: each stage-1 pixel row (GHO1, HUD5, HUD6/s1, KEY1, ALR2/s1 pixel part, FX1/s1, RES1/s1, GOAL1) green two times; each plant red; restore; green. One MP1 pixel row on the hosted runner, recorded.

**Rows:** GOAL1. **Runs: 4 (3-5). Headed: 4.**

### Task REG2-1: regression at the end of stage 1

**Seat:** `qa-lead` (S). **Review:** `technical-director` (O). **Inputs:** `I-PIX1`, `I-HEAD`, `I-NC1`, `L0-API2`, `L0-DBG`. **Design:** row REG2 (D:2460), D:2754-2755.

- [ ] All MC rows, the fault sets, the frozen values, and each `done:` row of the Yardstick coverage with its test token (the qa-lead token script until R7b). The camera plant: `clamp_to_map` returns at once (`crates/turts_interaction/src/camera.rs:358`); `bounds_hold` (`crates/turts_interaction/tests/camera.rs:322`) is red. The browser-row plant that D:2460 leaves to the plan: verify first; lead: shift the projection in `assertPhysicalClick` (`tests/browser/sel1-oracle.js:167`) and see SEL1 red.

**Rows:** REG2/s1. **Runs: 2 (2-3). Headed: 2.**

### Task C1-PLAYER and Task C1-PERSON: checkpoint C1

Section 8.

### Stage-1 items that this plan judges not needed for C1

The plan keeps them in stage 1, as D says. Root or the CEO can move them to stage 2:

- TGT2 (retaliation and the hold rule): no `frontier` weapon outranges an acquire range (D:2476, D:2747). About 1 run of `L-TGT`.
- The restart part of LEAVE1/s1 (`mp1-serve.sh` loop, a new `match_id`): C1 is one match. About 1 run.
- LOB1 run 4 (reload in the lobby): the C1 instruction has no reload. Under 1 run.
- The three-slot sim test of MAT1: it needs more than `SLOT_COUNT` 2 (design question Q4).
- The exit-blocked case of PRD1: the loop does not need it. Under 1 run.

## 8. Checkpoint C1

**Preconditions.** On one commit of `mp1/integration`: every stage-1 row part of section 13 is green with its recorded red; REG2/s1 is green; A3 is merged; open point 19 is closed (`C0-PACE`); the GPU machine of the person run has the E-GPU numbers.

**Step 1: the blind `player` seat (`C1-PLAYER`, Opus).**
- The `player` agent gets the goals text of D section 13, match 1 (D:2488-2490), the C1 end instruction (below), and the harness of the stage-1 pixels path (screenshots and input only, as for MC PT1: D of MC `:1192`). It gets no other document.
- It plays both windows to the result screen and reports each stop with its time and step.
- PASS: it reaches the result screen, and no stop is a "cannot find an object, an action or a reason". FAIL: the routing table below. Root does not start Step 2 on a FAIL.

**Step 2: the person (`C1-PERSON`).**
- Start command, on the play machine: `bash scripts/mp1-play.sh; echo exit=$?`. It prints `http://127.0.0.1:<port>/`. Use the tested Linux Chrome launch configuration (`vault/M3 Playtest Guide.md`, "Linux browser startup"; flags NOT TRACED).
- Open two browser windows side by side on that URL. Each shows "Waiting for the other player" until both are seated, then "Click to play". Do not reload a window before both show "Click to play" (recheck5-network-host F1).
- The instruction that the person gets, word for word (D:2361-2363): "Use one window to destroy the Command Centre of the other window. If no Command Centre falls after 40 minutes, stop: write 'no end' and the time in the vault note, close both windows, and stop the server with Ctrl+C."
- The person signs off C1 in the vault (a note under `vault/Decisions/`, with the commit, the date, PASS or FAIL, and the words of each stop). "No end" after 40 minutes is a known limit (D:2363-2364), not a FAIL.

**What a FAIL sends back:**

| Symptom | Task |
| --- | --- |
| No "Waiting for the other player", no "Click to play" in both windows, a new id gets a seat after the start | `L-MATCH-SRV1`; the client phase: `L-LOBBY-C1` |
| The Command Centre is not selected, the goal line or a hint is wrong or missing | `UI-HINT`; the texts and the order: `C-UI` |
| The ghost, a ring, or a reason text is wrong | `UI-PLACE`; a server refusal that differs: `L-CON1` |
| The Ore Hauler does not gather from the start, or a new one stays idle | `L-STO`; no trips near a building: `L-OBS`; the data: `C-UNITS` |
| Credits, power or row states are wrong | `UI-HUD`; the values: `L-PAY`, `L-PWR` |
| An alert repeats, sounds at tick 0, or the wrong alert shows | `UI-ALERT`; the rules: `C-UI` |
| Units do not fire, do not acquire, or the hold rule fails | `L-TGT`, `L-DMG` |
| No tracer, flash, death effect or "+N" | `L-FX-PRES`; the event is missing: `L-FX-WIRE` |
| The Command Centre dies and no outcome, or a wrong reason line | `L-MATCH-SIM`, `L-MATCH-SRV1`, `UI-RESULT` |
| The frame rate is too low to play | needs decision: not a stage-1 task (D-N is a PT3 condition); Root asks `performance-architect` and `bevy-rendering-engineer` |
| A panic or a crash | the last task that touched the crate; Root triages |

After a repair, the repaired task's rows and REG2/s1 run again, then Step 1, then Step 2.

## 9. Stage 2 tasks

Stage 2 starts only after C1 passes (D:2366). Every task has the input `C1-PERSON` directly or through another task. Code lines here are leads; every builder verifies first, because stage 1 moves them.

| Task | Seat (tier) / review | What (D reference) | Rows; plants (D line) | Runs |
| --- | --- | --- | --- | --- |
| `L-CON3` | sim (S) / qa-lead | `CancelConstruction`, refunds, held-order cancel, open-site count (D 6.1 :503-505, 6.4 :648-656) | CON3; (1) to (3) (:2442) | 2 |
| `L-REP` | sim (S) / qa-lead | `repairer`, `Repair`, charge by applied health, `auto_range` with the grid cadence (D 6.5 :685-702); static health-writer check on the syn scanner | REP1; (1) to (3) (:2445) | 3 |
| `L-RLY` | sim (S) / network-engineer | `SetRally`, `UnitOrderRecord.rally`, follow order as an engine order, `rules.ron rally`, `ground_rally` (D 6.5 :704-735). Step 1 traces whether the order queue accepts a queued `Gather` (D:731-732) | RLY1 (A) to (C), (1) to (4) (:2446); STO1/s2 run 4, (f) | 3 |
| `L-MATCH-SIM2` | sim (S) / backend-engineer | `SlotConnection` inputs, `DisconnectedFor` in ticks, `Surrender`, statistics counts (D 6.7.8, 6.7.9); verify first whether the owner of `by` is known when `Destroyed` is written (D:1149) | none | 3 |
| `L-LOBBY2-SRV` | backend (S) / network-engineer | `AllSeatedReady`, Countdown (R11), `LobbyAction`, `LobbyRefused`, ready and colour rules (D 6.7.5 :913-965, 6.7.6) | LEAVE1/s2 (:2475) | 3 |
| `L-MATCH-SRV2` | backend (S) / network-engineer | room writes `SlotConnection`; `connection` script kind; `all_disconnected`; the `to_sim` exception for an empty `Surrender` (`crates/turts_server/src/orders.rs:34-40`); stage-2 `standoff` (D 6.7.8 :1102-1110, 6.7.12) | MAT1/s2 (c) to (k) (:2447) | 3 |
| `L-LOBBY2-UI` | bevy (S) / ux-designer | lobby screen of 7.6; hooks `ready`, `unready`, `set_colour`, `lobby_move`, `stale_revision`; colours from `MatchStart.colours`. Early test (D:2529): the lobby stub of two clients is green within 4 runs, else report | LOB1/s2 runs 1-3, LOB2 (:2448, :2472) | 4, headed 1 |
| `UI2-RESULT` | bevy (S) / ux-designer | statistics on the result screen; merge after `L-MATCH-SRV2` | RES1/s2 (stats, wire check, `Gathered`) (:2449) | 2 |
| `UI2-CONN` | bevy (S) / ux-designer | surrender item and dialog (shell), connection banner, `SlotDisconnected`, `SlotReconnected` alerts | SUR1 (:2473), ALR2/s2 | 2 |
| `UI2-HUD` | bevy (S) / ux-designer | HUD6 part 2, cancel-construction controls, rally flag and line | HUD6/s2 (:2469) | 3 |
| `UI2-SMART` | bevy (S) / ux-designer | smart rules and cursors `rally`, `repair` | SMT1 (:2470) | 2 |
| `FX2` | bevy (S) / performance-architect | other effects of 8.3, `hit_reduced`, the sound engine part (8.4) | FX1/s2 (:2450) | 3 |
| `C2-SOUND` | game-designer (S) / player | the P5 sound set; sound assets: NOT TRACED how they are made | AUD2 (:2471) | 2 |
| `PRES-MEASURE` | bevy (S) / performance-architect | 8.5 first task: frame time at 400 units, primitives against placeholder models at the D-M caps, wasm memory high-water (open point 13); the VIS3 hook `frame_stall_ms` and its two plants | none | 2, headed 2 |
| `PRES-MODELS` | bevy (S) / mod-runtime-engineer | V1, V2 checks with `gltf = "=1.4.1"` (D 8.1), loader from package bytes, flat template, team material, shape fallback, the decoded-image cap (perf N3) | none | 5 |
| `PRES-ART` | bevy (S) / ux-designer | 9 models within the caps; merge after `PRES-MODELS`; the art path is NOT TRACED | MDL1 (:2455) | 4 |
| `UI2-MC` | bevy (S) / ux-designer | X18 items except the no-GPU page (D 7.7) | MCX1 (:2457) | 4 |
| `WEB-NOGPU` | web-platform (S) / bevy | the no-GPU page (D 7.7 :1979-1982) | NOGPU1 (:2458) | 2 |
| `C2-CONTENT` | game-designer (S) / mod-builder | guide, tuned numbers, stage-2 match file (`AllSeatedReady(3)`, `DisconnectedFor(120)`, `Surrender`), hint H3, rally rules | none | 2 |
| `I2-L` | web-platform (S) / backend-engineer | open point 10: the browser L for a reload and a tab close; step 19 breaks if L is above 13 s (D:1100-1101) | none | 1, headed 1 |
| `I2-PIX` | qa-lead (S) / bevy | the pixels-only two-player driver of the full path; batch B2 of all stage-2 pixel rows; the MP1 fault set in the registry (R16); the PT3 timing script with plants (1) to (3) (:2459) | none | 4, headed 4 |

## 10. Regression, play test, measurements, records

### Task MEAS-EXIT: the numbers of exit item 5

**Seat:** `qa-lead` (S). **Review:** `performance-architect` (O). **Inputs:** `I2-PIX`. **Design:** D:80-90, D 8.5, row VIS3 (D:2456).

- Server tick p50 and p99 over 200 or more ticks: the MP1 mixed scene at 2 x 200 units, and the idle tick (`turts_server --stats`, `--synthetic-units`). The 1,000-unit 4-slot and 5,000-unit 8-slot scenes are blocked by `SLOT_COUNT` 2: design question Q4, **needs decision**.
- Postcard bytes per tick per client by record kind, and the export time.
- Native `apply_delta` p50 and p99 (R13).
- Browser frame p50 and p95 and the wasm memory high-water after 10 minutes at 400 units, debug state off and on, HUD on and off (`node tests/browser/measure.js`, the M1 commands in `docs/measurements/m1.md`).
- The boot-to-ready time of the server with `frontier` (D:88-89).
- VIS3: the D-N floor on the play-test machine, with models on and the debug state off; plants `frame_stall_ms=60` on every frame and on 1 frame in 10.
- Each number with the machine, the GPU, the image and the package versions (S 13.2). Recorded, not judged (S 15). A VM is optional; with one, use the `default` gcloud configuration and delete the VM after the run.

**Rows:** VIS3. **Runs: 2 (2-3). Headed: 2.**

### Task REG2-2: the final regression

**Seat:** `qa-lead` (S). **Review:** `technical-director` (O). **Inputs:** `I2-PIX`. As `REG2-1`, on the stage-2 tree, and the hosted CI in the form of D:76-79. **Rows:** REG2/s2. **Runs: 2 (2-3). Headed: 2.**

### Task AUDIT: architecture audit

**Seat:** `technical-director` (O). **Inputs:** `REG2-2`. The audit of the whole MP1 change (exit item 3, X23), with the refused list of D:2288-2291. **Runs: 2.**

### Task PT3-PLAYER and Task PT3-PEOPLE: the play test

- `PT3-PLAYER` (Opus): the `player` agent plays the full build through the harness of `I2-PIX`, with the goals text of match 1 (D:2488-2490). PASS before people.
- `PT3-PEOPLE` (procedure owner `qa-lead`): start conditions: `MEAS-EXIT` shows the D-N floor on the play-test machine; `PT3-PLAYER` passed.
    1. Start: `bash scripts/mp1-play.sh; echo exit=$?` on the play machine. Two people, one not the CEO (P7). The observer has the Player path; the testers do not (D:2484).
    2. Match 1: both testers get the goals text of D:2488-2490.
    3. Match 2 and match 3: the texts of D:2494-2500. Each match is a new server process; between matches the testers reload the page.
    4. Record for each tester: `turts_debug.hints`, each spoken stop with its time and step, the result screen, and the answer to "Would you play a second match?" (D:2511-2512).
    5. The PT3 timing script judges the 60 s rule from `site_ms` against the server order log (D:2459).
    6. One person signs off in the vault. **Rows:** PT3. A FAIL goes to the owner of the step (Player path step to row to task, section 13).

### Task REC: records

**Seat:** `qa-lead` (H) for `docs/measurements/mp1.md` (one summary at the top). Root, not a builder, writes:
- `CLAUDE.md`: protocol 5 and API 0.2.4; the crate `turts_rules` and its row; `room.rs`; the new options `--start-now`, `--input-log`, `--rejoin`, the `--actions` kinds and the `server.ron` fields; `scripts/mp1-serve.sh`, `scripts/mp1-play.sh`; the packages `frontier` and `standoff`; `docs/measurements/mp1.md`.
- The vault: each decision, the work log, the C1 and PT3 sign-off notes, the CEO report.
- The specification edits of decision S1, after the CEO answer.
**Runs: 1.**

## 11. Open points of D section 15

<!-- mp1-open:start -->
| Point | Closing task | Stage | How |
| --- | --- | --- | --- |
| 1 | L-OBS | 1 | the measure lands with its line count; the 40-line estimate is replaced by the count |
| 2 | Root | - | the slice tasks are inputs of this graph (section 3); a slice finding that changes D section 9 is a change request (5.2) |
| 3 | MEAS-EXIT | 2 | numbers recorded, none promised; D-N is the CEO line of the header |
| 4 | C1-FINAL | 1 | duels recorded; a failed ratio is a retune |
| 5 | MP2 | - | a question for the MP2 design; no MP1 task |
| 6 | REG2-1, REG2-2 | 1, 2 | REG2 runs on the base that MP1 forward-merges; MP1 merges to main after MC |
| 7 | REC | 2 | the S:727 edit after the CEO answer to S1 |
| 8 | closed | - | D 6.4 and 6.5 decide; recorded additive items (D:1740-1742) |
| 9 | Root | - | R7b is the orchestrator's decision; until then the qa-lead token script runs in REG2-1 and REG2-2 |
| 10 | I2-L | 2 | the browser measure of L before the step-19 test |
| 11 | gate2 | - | R12: dropped from MP1, moved to M4 with CO03 |
| 12 | gate2 | - | R13: moved to M7 with the `apply_delta` numbers of MEAS-EXIT |
| 13 | PRES-MEASURE | 2 | measured before MDL1 runs |
| 14 | L0-WIRE | 0 | the replica test; if red, lane 0 makes the replica accept the Snapshot |
| 15 | MP2 | - | recorded for the MP2 design |
| 16 | UI-HUD | 1 | read by the producer: today the cancel is a separate `Button` with `action: Item(CancelAction)` (`mods/forge/interface/hud.ron:60`), fired by `On<Activate>` (`crates/turts_client/src/widgets/mod.rs:1517`); no widget row takes a right click (`MouseButton::Right` only in `input.rs:49`, `core_adapter.rs`, `client_shell.rs`). `UI-HUD` adds the right click; `ux-designer` confirms at gate 2 |
| 17 | closed | - | closed in D revision 6 |
| 18 | L-MATCH-SRV2 | 2 | read by the producer: A1c deletes all of `crates/turts_audit/src/replay.rs` (SP Task A1c), which holds the `EmptyCommand` rule (`replay.rs:836`, `:6605`). `L-MATCH-SRV2` starts after A1c, so only the `to_sim` exception is needed. Whether the A1b ledger validator has an `EmptyCommand` rule is NOT TRACED; `L-MATCH-SRV2` step 1 checks it |
| 19 | C0-PACE | 0 | the model on the stage-1 order and on the alternative; `C-UI` writes the chosen order before C1 |
| 20 | C-UI | 1 | read by the producer: `box_select` keeps one class, mobile or static, picked by the lowest `selection_rank` (`crates/turts_interaction/src/lib.rs:1038-1069`); `frontier` lists mobile units first in `selection_priority`; `ux-designer` confirms at gate 2 |
| 21 | L-ROOM0 | 0 | the trace of step 1, before `L-MATCH-SRV1` builds the seat release |
<!-- mp1-open:end -->

## 12. Binding notes from the last reviews

| Source | Note | Task |
| --- | --- | --- |
| recheck6-rigour (a), Low | GOAL1 plant (g) second clause | fixed in D revision 6 (D:2461 has no driver clause); `I-PIX1` uses D:2461 |
| recheck6-rigour (b), Medium | STO1 run 4 is stage 2 | D:2424 and D:2462 say so; the stage-2 list D:2428-2430 omits STO1: split STO1/s1 (`I-HEAD`), STO1/s2 (`L-RLY`); Q1 |
| recheck6-rigour (d) | trace point 21 before the seat release | `L-ROOM0` step 1; `L-MATCH-SRV1` input |
| recheck5-player N-L2, Low | pacing of the stage-1 order | `C0-PACE` (point 19) |
| recheck5-player, not traced | the box drag picks up the Command Centre | `C-UI` (point 20) |
| recheck5-player, not checked | ore positions | `C-UNITS` checks them against the ring |
| recheck5-player STOP 1, N-L1, N-L3 | H5 and X4 texts | fixed in D (Player path); `C-UI` writes them |
| recheck5-network-host F1 | reload before the other seat fills | the C1 instruction (section 8); LOB1 run 4 in `L-LOBBY-C1` |
| recheck5-network-host §3 | `MatchResult` before the end `Snapshot` in one frame | `UI-RESULT` test |
| recheck5-network-host §5 | no test depends on refusals that skip the bucket | `L0-WIRE` step 3 |
| recheck5-network-host point 14 | the replica accepts an unrequested Snapshot | `L0-WIRE` step 1 test |
| regate-performance N1, N5 | per-frame HUD work, quadratic alert | the client cost rule in each client task; `UI-ALERT` (health index); `MEAS-EXIT` HUD on and off |
| regate-performance N2 | on-change records | ORDW1 case C in `I-HEAD` |
| regate-performance N3 | decoded images | `PRES-MODELS` |
| regate-performance N4 | `health_trace` window | `L0-DBG` |
| regate-performance N6 | VIS3 plant on 1 frame in 10 | `PRES-MEASURE`, `MEAS-EXIT` |
| regate-performance N7 | grid cell | `L-TGT` |
| regate-performance M2 | replica cost | point 12, R13 |
| regate-performance M4 | string ids: no instrument | `L-DMG` source test; `L0-SIM` install indices |
| regate-performance M5 | records from writers: no instrument | each reviewer of a record-writing task checks that the record enters `CommittedTick` from the writer's change list |
| regate-performance, cost table | hint evaluation per frame | `UI-HINT` counter test |
| regate-performance, wire | bytes by record kind have no red | recorded only (`I-HEAD`, `MEAS-EXIT`) |

## 13. Acceptance rows and their owners

### 13.1 Owners

Read from the task table (Rows column). 51 row parts; each has one owner (script, section 3.5).

- Lane 0: DBG1 (`L0-DBG`), VAL1 (`L0-API2`).
- Stage 1: IMM1, MAT1/s1 (`L-MATCH-SRV1`); LOB1/s1, LEAVE1/s1 (`L-LOBBY-C1`); PAY1, PAY2 (`L-PAY`); INC1 (`L-INC`); DMG1 (`L-DMG`); TGT1, TGT2 (`L-TGT`); CON1, PRD1 (`L-CON1`); PWR1 (`L-PWR`); DES1 (`L-DES`); OBS1 (`L-OBS`); FX1/s1 (`L-FX-PRES`); CON2, GHO1 (`UI-PLACE`); HUD5, HUD6/s1, KEY1 (`UI-HUD`); ALR2/s1 (`UI-ALERT`); RES1/s1 (`UI-RESULT`); NC1 (`I-NC1`); ORDW1, STO1/s1 (`I-HEAD`); GOAL1 (`I-PIX1`); REG2/s1 (`REG2-1`).
- Stage 2: CON3 (`L-CON3`); REP1 (`L-REP`); RLY1, STO1/s2 (`L-RLY`); MAT1/s2 (`L-MATCH-SRV2`); LEAVE1/s2 (`L-LOBBY2-SRV`); LOB1/s2, LOB2 (`L-LOBBY2-UI`); RES1/s2 (`UI2-RESULT`); SUR1, ALR2/s2 (`UI2-CONN`); HUD6/s2 (`UI2-HUD`); SMT1 (`UI2-SMART`); FX1/s2 (`FX2`); AUD2 (`C2-SOUND`); MDL1 (`PRES-ART`); MCX1 (`UI2-MC`); NOGPU1 (`WEB-NOGPU`); VIS3 (`MEAS-EXIT`); REG2/s2 (`REG2-2`); PT3 (`PT3-PEOPLE`).

### 13.2 Row splits from planning

<!-- mp1-splits:start -->
| Row | Parts | Reason |
| --- | --- | --- |
| STO1 | STO1/s1, STO1/s2 | run 4 and plant (f) need a rally (X9, stage 2; D:2424, D:2462); the stage-2 list omits STO1 (Q1) |
| ALR2 | ALR2/s1, ALR2/s2 | the lost-connection and return alerts need `SlotConnection` (X11c, stage 2) (Q2) |
<!-- mp1-splits:end -->

## 14. Budget

### 14.1 Runs

From the task table (script output, section 18). D section 11 for comparison.

| Part | This plan: point (range) | D section 11 |
| --- | --- | --- |
| Lane 0 (13 tasks) | 30 (24-39) | inside stage 1 |
| Stage 1 (28 tasks) | 84 (66-109) | |
| Lane 0 + stage 1 | **114 (90-148)** | 62 to 106 |
| Stage 2 (27 tasks) | **68 (59-94)** | 39 to 66 |
| Gates outside tasks: gate 2 of this plan (4 to 6), the C1 gate lenses `player`, `bevy-rendering-engineer`, `ux-designer` (3), the final gate (4 to 6), re-gates of repairs (4 to 8), change requests (0 to 3) | 15 to 26 | 20 to 35 |
| **MP1 total** | **197 to 208 point; 164 to 268 range** | 132 to 225 |
| Slice, remaining (not MP1) | 40 | 11 to 18 (whole slice) |

Why the plan is above D section 11:
- Each task has its own review run, and D counted scope items, not tasks. Lane 0 alone is 13 tasks; D gave X0 and X19 about 10 to 17 runs.
- The 6.10 table has about 70 rows, each with a red package (`L0-API1` and `L0-API2`: 6 runs).
- Stage 2 holds the instruments and the close: `I2-PIX` 4, `MEAS-EXIT` 2, `REG2-2` 2, `AUDIT` 2, `PT3` 2, `REC` 1.

What would cut it (each is a CEO or Root decision, not a plan step): P3 (b) cuts rally (about -6); P1 (a) cuts models (about -11); D-L (b) cuts income (-2); moving the five items of section 7 "not needed for C1" to stage 2 lowers stage 1 by about 3 and does not lower the total.

### 14.2 Headed runs

26 MP1 headed runs: lane 0 3 (`L0-DBG`, `L0-SIM`, `E-GPU`), stage 1 9 (`L-LOBBY-C1` 1, `I-PIX1` 4, `REG2-1` 2, `C1-PLAYER` 1, `C1-PERSON` 1), stage 2 14 (`L-LOBBY2-UI` 1, `PRES-MEASURE` 2, `I2-L` 1, `I2-PIX` 4, `MEAS-EXIT` 2, `REG2-2` 2, `PT3-PLAYER` 1, `PT3-PEOPLE` 1). The slice adds 5 (S45, A1b, A1c, REG). In the ASAP schedule, 3 headed tasks overlap at offset 0 (S45, `L0-DBG`, `E-GPU`) and 2 at offsets 4-7 (A1b, `L0-SIM`), 41 (`L-LOBBY2-UI`, `I2-L`) and 47-48 (`MEAS-EXIT`, `REG2-2`). The lock serializes them. A wait for the lock moves the path only for a headed task on the path: `I-PIX1`, `REG2-1`, `C1-PLAYER`, `C1-PERSON`, `I2-PIX`, `REG2-2`, `PT3-PLAYER`, `PT3-PEOPLE`. At offsets 47-48, `MEAS-EXIT` goes after `REG2-2`, because `REG2-2` is on the path.

### 14.3 Model tiers

Of the 68 MP1 tasks: 62 have a Sonnet 5.5 builder (60 with an Opus review, plus `C1-PERSON` and `PT3-PEOPLE`, which a person judges), 2 a Haiku 5.5 builder (`C0-PACE`, `REC`), and 4 are Opus 5.5 work (`L0-EXIT`, `AUDIT`, `C1-PLAYER`, `PT3-PLAYER`). At the MC run mix (41 % build, 43 % review, 16 % repair; `docs/research/mp1/input-producer.md:70`), about 78 of the 182 task runs are Opus review runs.

### 14.4 The three largest risks to the schedule

| Risk | Early test that retires it |
| --- | --- |
| 1. The slice path sets the date of C1. The first 25 runs of the critical path are slice runs to A3; C1 is at least A3 + 10 runs. D0 (the depletion repair) is not in `codex/mc-integration` yet (`git merge-base --is-ancestor codex/mc-fix-deplete codex/mc-integration` gives no), so E1 waits for it. | Root checks the D0 merge each day; the six "now" tasks run in the wait. The A3 measure of D section 14 row 4 (D:2523) runs as `L-OBS` step 1 the day A3 merges forward. |
| 2. The near-critical MP1 chain `L-CON1` → `L-PWR` → `L-TGT` has 1 run of slack, and client work overran in MC (the client shell took 22 runs against 6, D:2529). | CON1 cause cases red in the first `L-CON1` run (a 6-run task that has no red after run 2 is reported). Stage 2: the lobby stub of D:2529 in `L-LOBBY2-UI` (more than 4 runs is red). |
| 3. One GPU lock for all headed work, and the hosted runner has no GPU (D:2525-2526). | `E-GPU` now: two headed clients for 10 minutes with combat, frame p95 and the lock wait. `I-PIX1`: one MP1 pixel row on the hosted runner, recorded. |

## 15. What can start now

The slice is at 4 of 21 tasks (S1, Q1a, G1, A1a merged into `pre-mp1/integration` at `85f34cb`; S3 and S45 not merged). E1 needs S2, S3, S45 and D0p; D0p needs D0, which is not merged into MC. So only tasks with no slice input can start. Each input check below was read at `85f34cb`.

| Task | Inputs met because | Evidence |
| --- | --- | --- |
| `L0-RAW` | no slice task edits `turts_mod_runtime` | SP task table: 0 hits for `version_table` and `turts_mod_runtime`; the presence of today is `crates/turts_mod_runtime/src/content_mc_validation.rs:65`; the refusal is `version_table.rs:96-108` |
| `L0-RULES` | new crate; `scripts/boundary.sh` has the S1 rows and no slice task edits it again | `git diff --stat a9a5fea 85f34cb` shows `scripts/boundary.sh` +13 (S1, merged); SP mentions it only in gate commands (SP:213, :301) |
| `L0-DBG` | the four files are outside S45 and every slice task | `crates/turts_client/src/lib.rs:147`, `debug.rs:441`, `replica.rs:1182`, `turts_client_web/src/lib.rs:166`, `:371`; the S45 diff file list `8d6902c..d047433` has none of them |
| `L-ROOM0` | new files only; no `session.rs` edit | no `room.rs` in `crates/turts_server/src/` (ls); the end hook lead `session_diagnostics.rs:384`, `stats.rs:104-105`; `judge_hello` at `session.rs:152-159`; `Slots::claim` at `crates/turts_sim/src/slots.rs:46` |
| `C0-PACE` | docs and probes only | the scripts exist in `.../scratchpad/mp1-pacing/` (`pacing.py`, `fights2.py`, `match.py`) and `.../scratchpad/regate-player/hold5.py` (ls) |
| `E-GPU` | runs on the MC build | wrapper `mc_headed_v4.sh` sha256 `66c105e5…cec235` equals SP:250 |

Not yet: `L0-API1`, `L0-SIM`, `L0-WIRE` need S2 (orders.rs struct literals, `crates/turts_sim/src/orders.rs:3767` and four more sites; the relation match at `:1126`); `L0-CORE` needs S45 (`crates/turts_interaction/src/lib.rs:137` moves). Gate 2 can run on the rest of the plan while these six run.

## 16. Design questions from planning

| # | D line | Question | Plan action |
| --- | --- | --- | --- |
| Q1 | D:2428-2430 against D:2424, D:2462 | The stage-2 row list omits STO1, but run 4 and plant (f) are stage 2. | Split STO1 (13.2). |
| Q2 | D:2468 against D:256, D:2428 | ALR2 is a stage-1 row, but its "return of a slot" and "lost connection" alerts need X11c (stage 2). | Split ALR2 (13.2). |
| Q3 | D:2461 (GOAL1 plant (e)) against D:260, D:2347 | GOAL1 (stage 1) needs the "+300" float and the `Credited` effect; X14 stage 1 names only `Fired`, `Destroyed`, flash and death. | R14. |
| Q4 | D:82-83, D:2447 against D:279-280 and `crates/turts_sim/src/slots.rs:7` | Exit item 5 asks for 1,000 units with 4 slots and 5,000 units with 8 slots, and MAT1 has a three-slot sim test; `SLOT_COUNT` is the constant 2, and "more than two slots" is not in MP1. **needs decision**: (a) measure with 2 slots; (b) a test-only slot count in the sim; (c) drop those scenes from MP1. | `MEAS-EXIT` and `L-MATCH-SIM` wait for the answer for those parts only. |
| Q5 | D:241-272 | The engine measure of D 6.9 (approach point, row T1, OBS1) has no scope item. | `L-OBS` maps to X0e and X4. |
| Q6 | D:274-277 | The alert engine items of D 7.9 (kinds, gap, `sound_when`, `focus`, `GathererIdle`) have no scope item. | `UI-ALERT` maps to X15. |
| Q7 | D:1268 against D:2346 | `MatchResult` has `stats`, but statistics are stage 2. D does not say what stage 1 sends. | Stage 1 sends `stats: []`; LEAVE1/s1 compares the whole record. |
| Q8 | D:2303 against D:257 | Section 10 starts L-LOBBY "now" as the server room, but the seat table, the start step and the sim gate are X11b items of L-MATCH (E1). | `L-ROOM0` now (pure module); wiring in `L-MATCH-SRV1`. |
| Q9 | D:900-905 | Each `SimSet` gets the run condition "the sim `MatchConfig` exists"; D does not say how test harnesses that tick the sim without a server get it. | R17. |
| Q10 | D:60, D:2366 | Stage 2 starts only after C1. Content and art (`PRES-MEASURE`, `C2-SOUND`) could start earlier with no code risk. | The plan keeps the CEO order; 11 stage-2 lanes start at offset 35. |

## 17. Not traced

- The D0 merge time (MC repair lane).
- Which lightyear schedule despawns a `ClientOf` at the netcode timeout (`L-ROOM0` step 1).
- How many test harnesses tick the sim without a server (R17, `L0-SIM` step 2).
- How the harness requests the debug state (`L0-DBG` step 2).
- The name of the credit delivery event (D:1150; `L-FX-WIRE`).
- Whether the order queue accepts a queued `Gather` (D:731-732; `L-RLY`).
- Whether `bevy_ui_widgets` `Activate` fires on a right click (point 16 lead).
- Whether `turts_pack::load_package` parses the interface file (D:878; `L-MATCH-SRV1`).
- Whether the A1b ledger validator has an `EmptyCommand` rule (point 18).
- The browser L (point 10; `I2-L`).
- The play-test machine and its GPU (D-N).
- The flags of the tested Linux Chrome launch configuration.
- How the sound set and the models are made (X17, X16).
- Which MC package gives two armed slots for `E-GPU`.
- Whether T7 needs the GPU lock.
- The `parse.rs:23-29` "did you mean" rule and `ron` 0.12.2 at `Cargo.toml:15` (read in D, not by the producer).
- The remaining run counts of S45 and S3 (the plan uses 1 each).
- The ore positions of Twin Fields (D section 5 gives counts only).
- That `vault/MP1 Execution Board.md` exists.

## 18. What I ran

| Command | Claim tested | Result that would refute | Observed |
| --- | --- | --- | --- |
| `git merge-base --is-ancestor <branch> HEAD` in the slice worktree for S1, Q1a, G1, A1a, S3, S45 | The slice is at 4 of 21 | another set merged | S1, Q1a, G1, A1a merged; S3, S45 not |
| `git merge-base --is-ancestor codex/mc-fix-deplete codex/mc-integration` | D0 is the E1 blocker | the repair is merged | not merged; also not in the slice |
| `git diff --stat a9a5fea 85f34cb -- crates scripts mods` | D anchors still hold at the slice head | many code files differ | 13 files; only `economylog.rs` anchors moved (to `turts_audit/src/replay.rs`) |
| `python3 -I anchors.py` (scratchpad `mp1-plan/anchors.py`, 101 D anchors) | each cited `path:line` holds the cited item | an item missing at its line | all files exist; 12 lines differ by one to three lines from the guessed text, the items are at the cited place |
| grep of the SP task table for 23 hot files | R1: no slice task touches the "now" files | a hit for `turts_mod_runtime`, `plugin.rs`, `replica.rs`, `debug.rs` | 0 hits each |
| `git diff --stat 8d6902c d047433 -- crates` in the S45 lane | `L0-DBG` files are outside S45 | one of the four files listed | none listed |
| grep `SlotRelation::` in `crates/*/src` | R2: new variants break exhaustive matches | no exhaustive match | five matches (orders.rs:1126, combat.rs:313, core_adapter.rs:340, cursor.rs:430, :619) |
| grep `UnitDef {`, `Placement {`, `CancelRefunds {` and the derives | `L0-API1` must wait for S2 | the literals use a default spread or no derive is missing | no `Default` derive; about 30 literal sites, 5 in `orders.rs` |
| grep of `turts_server/src` for `Disconnect`, `Unlink`, `On<Remove` | point 21 lead | a disconnect handler exists | only the `--stats` observer `session_diagnostics.rs:384` and the `Unlink` of refused connections |
| read `widgets/mod.rs:1236-1262`, `:1425-1445`, grep `MouseButton::Right` | point 16 lead | a right-click binding on a queue row | none; the cancel is a `Button` with `Item(CancelAction)` |
| read `turts_interaction/src/lib.rs:1038-1080` | point 20 lead | the box keeps every own unit | one class, by the lowest selection rank |
| read SP Task A1c | point 18 | A1c keeps part of `replay.rs` | it deletes the file |
| `python3 -I docs/research/mp1/probes/plan/mp1_plan_check.py <plan> <design>` | the graph numbers and coverage | RED, or a cycle | GREEN; path 52 runs; C1 at 35; 51 row parts, 30 scope items, 21 points; 112 concurrent pairs |
| the same with `--self-test` | the check can go red | a mutation stays GREEN | six mutations RED; unmutated GREEN |
| `python3 scripts/qa/completeness.py --self-test` and `--enforce` (in `/home/julien/Projects/TuRTS-mp1`) | the registry stays valid; the plan needs no row | exit 1 | self-test 21/21, exit 0; enforce exit 0, 3 specs |
| two file-level mutations of this plan (the STO1 split row deleted; an unknown input on `L-DMG`) | the check reads the plan file, not only memory | exit 0 | exit 1 each, with the expected messages |
| `cat docs/qa/specs.tsv` | does the registry hold plans | a plan row | designs only; the slice plan has no row, so this plan gets none |

The first model run (all client tasks with plain inputs) gave a path of 69 runs and C1 at 46; R8 and the restructured stage-2 lobby chain give 52 and 35.

## 19. What I changed

- `docs/superpowers/plans/2026-11-12-mp1-playable-skirmish.md` (this plan, new).
- `docs/research/mp1/probes/plan/mp1_plan_check.py` (the check script, new).
- No edit of D, `docs/qa/specs.tsv` or code. Scratch files: `/tmp/claude-1000/-home-julien-Projects-TuRTS/bf418660-b272-4bb3-8628-8d9ef2677d1d/scratchpad/mp1-plan/`.
