# MP1: Playable skirmish for two people: implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Plan status:** revision 3. **Gate 2 is closed** (2026-11-12): the rigour lens confirmed the patch with 0 High (`docs/research/mp1/gate2-confirm-rigour.md`); its Low items L-a to L-d are binding notes (L-a: the `L-PAY` reviewer counts the PAY1 plants (1) to (6) against D; L-c: the slice and MC headed runs still use the v4 wrapper on the same lock, so Root reads their receipts each day until they end). Revision 3 was a patch after the re-check of revision 2. Author: `producer` (build mode), 2026-11-12. Root commits this text.

**Review record.** Gate 2 ran on revision 1 (`24a18e1`) with five lenses. Reports in `docs/research/mp1/`: `gate2-qa-lead.md` (FAIL, 3 High), `gate2-rigour.md` (FAIL, 2 High; detail in `gate2-rigour-detail.txt`), `gate2-technical-director.md` (3 High), `gate2-presentation.md` (3 High), `gate2-ux-designer.md` (3 High). Total 14 High. This revision applies the orchestrator rulings R-A to R-S (R-R: the C0-PACE result; R-S: the S45 layout advice). Section 18 gives the disposition of every finding. Section 22 lists the D edits that revision 2 needed (errata E1 to E20 of D revision 6.1; Root applied them at `f7090aa`).

**Re-check of revision 2** (three lenses, reports in `docs/research/mp1/`): `gate2-recheck-qa-lead.md` (instruments, 1 High: N1, the GPU lock queue), `gate2-recheck-technical-director.md` (architecture and sequencing, 2 High: H-A, the cross-branch stack; H-B, the store commit on resync), `gate2-recheck-rigour.md` (rigour, 1 High: H1, the held job). Total 4 High. Every gate-2 finding of revision 1 is closed or partly closed. Revision 3 is a patch for the findings of the re-check only. It applies the orchestrator rulings R-T to R-Z (section 18, second table) and the errata E21 to E26 (D revision 6.2; this revision writes them into D, section 22). It also adds one binding amendment block to the slice plan (cross stack A3 on A4, and the order-writer rule).

**Design of record:** `docs/superpowers/specs/2026-11-12-mp1-playable-skirmish-design.md`, revision 6.2 (branch `mp1-design`; revision 6.1 at `f7090aa` holds E1 to E20; revision 6.2 adds E21 to E26 of this revision), "D" below. `D:n` is line n of D revision 6.1 unless the line says 6.2.
- Gate 1 is closed at revision 6. The Medium and Low items of the last checks are binding on this plan (section 12).
- When this plan and D disagree, D wins. Report each conflict to Root. Section 2 lists each place where this plan refines D; the column "Kind" says if the refinement changes a gated D statement (DCH, with its erratum) or not (REF).
- The slice plan `docs/superpowers/plans/2026-11-12-pre-mp1-slice.md` on `pre-mp1/integration` ("SP") is the model for form. Its gate sets (SP section 4) apply here unless a line says otherwise.

**Goal (D section 1):** two people finish a match with mouse and keyboard; both build, power and fight; both see the result screen. Two stages: stage 1 is the thin loop up to checkpoint C1; stage 2 is the breadth, then the play test PT3.

**Scope rule:** this plan adds no scope without a ruling. Each task names its scope items (D section 4, X0 to X26) and its rows (D section 12). The scope that the gate-2 rulings add (name labels R-E, the store R-H, the order writer R-I, the plant runner R-B, the second freeze R-J) and the re-check rulings add (the lock wrapper R-T, the pure-move task R-X, the held flag R-Y) is in section 22 as errata or in section 2 as refinements. The check script (section 3.5) fails when a scope item or a row part has no task.

### CEO decisions this plan assumes (ruling R-L)

Every line of D section 2 is "CEO to confirm". The plan uses the recommended option of D for each. The CEO does not need to answer all of them before lane 0 ends. Only an answer that is different from the recommendation costs work, and the cost depends on the kind of the answer:

- **Kind A, a new type or record of a stage-1 shape.** The lane-0 freeze `L0-EXIT` (offset 12, section 3.1) fixes it. A different answer after `L0-EXIT` is a change request with rework above the change-request budget (5.2). Needed before offset 12 **only if the CEO picks a non-recommended option**. Where a lane-0 task builds the shape before `L0-EXIT`, the latest time is that task (for example the seat table of `L-ROOM0`).
- **Kind B, a new type or record of a stage-2 shape.** The second freeze `L0-EXIT2` (offset 38, ruling R-J) fixes it. Needed before offset 38 under the same condition.
- **Kind C, a feature removed.** The feature stays behind its `UNBUILT_FEATURES` line (5.3) and its lane goes out. No wire or API change (5.2 item 4). Needed before the lane starts.
- **Kind D, content, a number or a later lane only.** Needed before the named task starts.

| Id | The question in plain words | Options (D section 2) | Recommended (the plan uses it) | If the CEO picks another option: cost; latest time |
| --- | --- | --- | --- | --- |
| P1 | Do units get 3D models in MP1? | (a) no models; (b) models in stage 2, at most 9; (c) other forms of D | (b) | (a) or (c), kind C: `PRES-MODELS`, `PRES-ART` out, `PRES-MEASURE` primitives only (about -11 runs); before `L0-EXIT2` (38) |
| D-M | Which model size caps? | (a) the caps of D 8.1 now; (b) caps set after the measure | (a) | (b), kind D: `PRES-MODELS` sets them after `PRES-MEASURE`; before `PRES-MODELS` (39) |
| P2 | Lobby: ready and colour? Teams? | (a) two slots, ready and colour, team = slot; (b) no colour choice; (c) also teams | (a) | (b), kind C: `L-LOBBY2-SRV` and `L-LOBBY2-UI` lose colour, row LOB2 out; before 38. (c), kind A: teams in the sim `MatchConfig` (`L-MATCH-SIM`) and in the seat table (`L-ROOM0`, built at offsets 0 to 3); before `L-ROOM0` merges (3) |
| P3 | Rally, cancel of construction, repair? | (a) keep all; (b) no rally; (c) no cancel; (d) no repair | (a) | (b), (c), (d), kind C: `L-RLY`, `L-CON3` or `L-REP` out (about -3 each); (d) also changes Decided S:798; before 38 |
| P4 | Pay at the start of a job, or while it builds? | (a) pay at once; (b) pay over time | (b) | (a), kind A: `ChargeOverTime`, the job `charged`, `held` and `Paid` fields change (`L-PAY`, `L-CON1`, `L-INC`, `UI-HUD`, `UI-ALERT`, `C-UNITS`); before 12 |
| D-A | How the charge of a tick is computed | (a) `floor(cost x progress)` difference, u128, coalesced events; (b), (c) of D | (a) | (b) or (c), kind A: the job record (`L-PAY`, rows PAY1, PAY2); before 12 |
| P5 | Which sounds? | minimal set in stage 2; voices; none | minimal set | none, kind C: `FX2` sound part and `C2-SOUND` out (row AUD2); voices, kind D: `C2-SOUND` grows; before `FX2` (38) |
| P6 | Faction and unit names | "Lantern Company", names of D section 5; other names | as D | kind D: `C-UNITS`, `C-UI` texts; before `C-UNITS` (11) |
| P7 | Who plays PT3? | two people, one not the CEO; other | as D | kind D: `PT3-PEOPLE`; before 55 |
| P8 | Is a player who stays disconnected defeated? | (a) no; (b) defeat after 120 s, a mod clause | (b) | (a), kind C (a feature removed): `DisconnectedFor` stays behind its `UNBUILT_FEATURES` line (`L-MATCH-SIM2`, `L-MATCH-SRV2`, `C2-CONTENT`); before `L-MATCH-SIM2` starts (38) |
| P9 | Units in the way of harvesters | (a) free approach point and placement gap; (b) the M4 route step in MP1 | (a) | (b), kind D: `L-OBS` becomes a route task of 400 to 700 lines (D:1432-1433); before `L-OBS` (21) |
| D-B | When does a lobby seat become a slot? | (1) the seat is a reservation, the start step claims; (2) the seat claims at once | (1) | (2), kind A: `LobbyState` and the claim (`L-ROOM0` builds the seat table at offsets 0 to 3; `L-MATCH-SRV1`, `L-LOBBY-C1`, `L-LOBBY2-SRV`); before `L-ROOM0` merges (3) |
| D-C | One match for each server process? | (a) yes; (b) a world reset in place | (a) | (b), kind D: `L-MATCH-SRV1` world reset, row LEAVE1; before 15 |
| D-D | Where do surrender and disconnect enter the sim? | (a) sim inputs in the command log; (b) of D | (a) | (b) is not a valid option (D:178) |
| D-E | Who owns construction progress? | (a) the site entity; (b) the constructor | (a) | (b), kind A: site records, `CommittedTick` fields (`L-CON1`, `UI-HUD`); before 12 |
| D-E2 | The open-site cap | (a) mod data; (b), (c) of D | (a) | (b) or (c), kind A: row K6 field (`L-CON1`, `L0-API2`); before 12 |
| D-F | Does the first hauler gather with no input? | (a) a start order on the map placement; (b) no | (a) | (b), kind C: row M3 stays `UNBUILT`, `L-STO` placement part out, `C-UI` hint H1 and GOAL1 plant (e) change, `I-HEAD` row STO1; before `C-UI` starts (14) |
| D-G | Idle units and the hold stance | (a) idle retaliates, hold fires back in range and never moves; (b) of D | (a) | (b), kind D: `L-TGT`; before 20 |
| D-H | Where the browser keeps the player id | (a) sessionStorage; (b) of D | (a) | (b), kind D: `L-LOBBY-C1`; before 12 |
| D-I | Is the full 0.2.4 table in D? | (a) yes; (b) a part | (a) | (b), kind A: `L0-API1`, `L0-API2`; before `L0-API1` (4) |
| D-J | Index, debug gate, grids, A3 entry | (a) as D; (b) of D | (a) | (b), kind A: `L0-SIM`, `L0-DBG`, `L-TGT`, `C1-FINAL`; before `L0-DBG` merges (3) |
| D-K | A rematch button? | (a) no; (b) yes | (a) | (b), kind A: a rematch record (`L-MATCH-SRV1`, `UI-RESULT`); before 12 |
| D-L | Free income of the Command Centre | (a) 3 credits each second; (b) none | (a) | (b), kind C: `L-INC` out (row INC1), `C-UNITS`, `C-UI` role line; the pacing of R-R and section 5 of D (the floor L2, 166 s) then has no floor; before 12 |
| D-N | The frame-time floor of the play-test machine | (a) p95 50 ms and p50 33 ms before PT3; (b) of D | (a) | (b), kind D: `MEAS-EXIT`, `PT3-PEOPLE` start condition; before 51 |
| Q1 | Named producer queues? | (a) no; (b) yes | (a) | (b), kind A: `L-CON1`, `L0-API1`, `UI-HUD`; before 12 |
| S1 | Edit the specification lines S:723, S:809, S:801, S:727? | edit after MP1; keep | edit | keep, kind D: `REC`; before 56 |
| 6.7.14 item 1 | What the server does when every player is disconnected | `Continue`; `Shutdown` | `Continue` | `Shutdown`, kind D: `L-MATCH-SRV2`; before 41 |
| 6.7.14 item 4 | The 10 s netcode timeout | keep; lower | keep | lower, kind D: `L-ROOM0` trace, a `network-engineer` change, MC 10.5; before `L-ROOM0` merges (3) |
| N1 | Ids in the no-content scan | (a) bare and qualified; (b) of D | (a) | (b), kind D: `I-NC1`; before 37 |
| Y1 | The yardstick edge MP07 to MP01 | (a) keep; (b) of D | (a) | (b), kind D: no task; `REC` updates the coverage note; before 56 |
| MC hosted CI (D:76-79) | Where the MP1 pixel rows run while the hosted runner has no GPU | the options of the MC note; (b) the local GPU gate with receipts | (b) | another option, kind D: `I-PIX1`, `REG2-1`, `I2-PIX`, `REG2-2`; before 28 |

Q11 (the held job) is not a CEO line: the orchestrator decided it as a design call (ruling R-Y, erratum E21; kind A, so it is in place before `L0-EXIT` at 12).

Summary: with the recommended options, no CEO answer blocks a task. Answers of kind A are due before offset 12 (or before `L-ROOM0` merges, offset 3, for D-B (2) and P2 (c)) only when the CEO wants a non-recommended option. Root sends this table to the CEO now.

## 0. Line numbers and labels

- Code lines in this plan are at the slice head `03aef38` (worktree `/home/julien/Projects/TuRTS-pre-integration`, branch `pre-mp1/integration`). S3 merged at `7b23b37`; S45 merged at `03aef38` (2026-11-12, during this repair). `git diff --stat 85f34cb 7b23b37 -- crates scripts mods` touches only the movement files (10 files); `git diff --stat 7b23b37 03aef38 -- crates` touches 29 client and check files and none of `turts_client/src/lib.rs`, `debug.rs`, `replica.rs`, `render.rs`, `feedback.rs`, `turts_client_web/src/lib.rs`, `turts_mod_runtime`, `turts_server`, `turts_sim` (section 20).
- Q1b merged after this text was first written: the slice head is `d7d7e6d` (S1, Q1a, G1, A1a, S3, S45, Q1b; 7 of 21 slice tasks; D0p in work). `git diff --stat 03aef38 d7d7e6d -- crates scripts` lists 13 files: `turts_checks/tests/boundary.rs`, `turts_pack/Cargo.toml`, `turts_pack/tests/m2_trace.rs`, `turts_sim/Cargo.toml` (the feature `test-probes`, `:11`), `movement/ecs.rs`, `frame.rs`, `mod.rs` (+2 lines, so the counter pattern is now at `movement/mod.rs:34-42`), `plain.rs`, `probe.rs`, `turts_sim/tests/frame_hash.rs`, `probe_flags.rs`, `scripts/boundary.sh`, `scripts/qa/probe_neutral.sh`. No client, server or mod-runtime file that this plan cites is in the list. The revision-3 lines that say `d7d7e6d` were read there.
- `movement.rs` is now `crates/turts_sim/src/movement/` (the `cfg(test)` counter pattern is at `movement/mod.rs:32-37`). `widgets/mod.rs` is now `widgets/tree.rs`, `values.rs`, `activation.rs` and more; `turts_interaction/src/lib.rs` keeps `step` at `:143`, and the smart rules are in `smart.rs`.
- S1 moved the replay validator from `economylog.rs` to `crates/turts_audit/src/replay.rs`. A1c deletes that file.
- **Verify first** means: the producer read the line at `03aef38`, and a slice task will move it before the MP1 task starts. The builder finds the item again by its name before the edit. S2 moves `crates/turts_sim/src/orders.rs` to `orders/`.
- **NOT TRACED** means: the producer did not read it. Section 19 lists every such item.
- Offsets are in runs from now (2026-11-12). One run is one agent dispatch (build, review or repair). The D0 wait is not in the offsets (section 14.4).

## 1. Execution contract

- **Integration branch (refinement R10).**
    - Create `mp1/integration` from `pre-mp1/integration`. Never merge MP1 work into `pre-mp1/integration`, `codex/mc-integration` or `main`.
    - After each slice merge, Root merges `pre-mp1/integration` forward into `mp1/integration`. A conflict there goes to the MP1 owner of the file (section 3.4). The slice never waits for MP1.
    - Root also merges the MC PT1 repair `codex/mc-fix-pt1` (head `9b3a7bd`; not an ancestor of `codex/mc-integration`, `mp1-design` or `pre-mp1/integration`, section 20) forward into `mp1/integration` before `UI-ALERT` starts. If MC merges it into `codex/mc-integration` first, the forward merge of MC carries it. The conflict owner is `UI-ALERT` (section 3.4).
    - MP1 merges to `main` only after MC merges to `main` (D:2548-2549) and after `AUDIT`.
- **One branch, one worktree and one build cache per task.**
    - Branch `mp1/<task id in lower case>`, from `mp1/integration` after every plain input of the task has merged there.
    - `git worktree add ../TuRTS-mp1-<id> mp1/<id>`; `CARGO_TARGET_DIR=target/mp1-<id>`. Delete both after the merge.
    - A task starts the moment its inputs merge (ruling C391). There are no waves in execution; the waves of section 3.2 are a reading aid.
- **Three kinds of edge (refinement R8, ruling R-G).**
    - A plain input `X`: the task branches after X merges.
    - A stacked input `X^` inside one integration branch: the task branches from the head of the X branch when the build of X ends, before X merges. It rebases when X merges, and it merges at least 1 run after X. Use: `UI-ALERT` on `UI-HUD` (same function `sync_widget_tree`, ruling R-S). In the slice: A3 on A4 (slice plan amendment of revision 3, TD proposal P-b).
    - **A cross stack (ruling R-W, TD H-A):** `A3^` in an MP1 task (`L-TGT`, `L-OBS`). The MP1 task branches from `mp1/integration` when its plain inputs have merged, and it merges the branch `pre-mp1/a3-grids` into its own branch at the end of the A3 build (a merge commit, not a rebase). A3 merges into `pre-mp1/integration` with a merge commit, not a rebase, so the A3 commits keep their ids and the forward merge of the slice into `mp1/integration` brings no second copy of them. The MP1 task merges at least 1 run after A3 merges into `pre-mp1/integration` and after Root merges the slice forward. If A3 must change after its build ends (a review repair), the MP1 task merges `pre-mp1/a3-grids` again. Block `mp1-tree` names each cross stack; the script checks it (3.5).
    - A merge-after task `X`: the task builds against the frozen contract of lane 0 from its plain inputs. It rebases and runs its rows only after X merges, and it merges at least 1 run after X. Root may start it at any offset inside its start window (section 3.1).
    - Two tasks are concurrent when no path of plain or stacked inputs joins them. A merge-after edge orders merges, not builds. Section 3.4 lists every concurrent pair that shares a hot file. D 6.2 names the three kinds (erratum E24).
- **The slice rule (SP:264, ruling R-K).** Until A2c merges forward, MP1 adds no test that pins a blocked engine trip or the operations of a single move. `L0b-ORD`, `L-CON1` and `L-STO` issue engine moves while A2b1 and A2c change those moves: until A2c, their CON1 and STO1 plants assert arrival and progress, not operation counts. The ORDW1 single-move case of `L0-WIRE` checks record bytes at accept and at arrival, not operations.
- **Reviewers.** Each reviewer works in `../TuRTS-mp1-<id>-review` at the candidate commit. It runs the task's fast gate and one fault of its own (red, exact restore, fresh green). It cites `path:line`. `L0-EXIT` is the second lens for the lane-0 tasks that are its inputs (`L0-API1`, `L0-API2`, `L0-WIRE`, `L0-STORE`, `L0-RULES`, `L0-SIM`, `L0-CORE`, `Q-PLANTS`); `L0b-ORD`, `L-ROOM0`, `E-GPU`, `PRES-PROBE`, `Q-LOCK` and `L0-SPLIT` have their own review only (rigour L1; `L0-SPLIT` is an independent review of two pure moves, ruling R-X).
- **Joint reviews (qa L1, ruling R-O).** `L-PAY` and `L-INC` build at the same offsets; the `L-PAY` review run (run 2) reads both diffs, and `L-INC` has no review run of its own. `UI2-RESULT` and `UI2-CONN` share one review run in `UI2-RESULT`. `UI2-SMART` is folded into `UI2-HUD` (one task, one review). The reviewer seat and the lens stay the same. **A joint reviewer runs one own fault for each task that it reviews (qa N7):** the `L-PAY` run 2 runs one own fault for INC1 (`L-INC`); the `UI2-RESULT` review runs one own fault for SUR1 and one for ALR2/s2 (`UI2-CONN`). Each fault: red, exact restore, fresh green.
- **Step order in every task** (SP section 1): red, green, commit, gate, plants.
    - A compile error is not a red. A `todo!()` body that panics in a test is a red.
    - Restore each plant with `git checkout -- <file>`. Then `git status --porcelain -- <file>` prints nothing.
    - Write `; echo exit=$?` after each command. Never pipe a red or a green through `tail`, `head` or `grep` before you read its exit code.
    - Use `cmp` or `rtk proxy diff` for file comparisons, never the filtered `diff`.
- **Plant records (ruling R-B; qa N6).** Each task that has plants writes `docs/research/mp1/plants/<task id>.json`: a list of records `{ "row", "plant", "files" (a list: one entry `{ "file", "patch" (a unified diff, or "edit": the exact old and new text) }` for each edited file, so a plant that edits two files has one record), "command", "env" (the environment variables of the command, `{}` when none), "headed" (true when the red runs through the headed wrapper), "expected_red" (the exact text that the red prints), "exit", "commit" }`. The format and its checker come from `Q-PLANTS`; the lane-0 tasks that merge before `Q-PLANTS` write the same fields, and `Q-PLANTS` checks them at its merge. The plant set of each row part is in section 3.6 (block `mp1-plants`).
- **Forward merges and plant records (qa N5).** A forward merge can stop a record from applying: the slice tasks A2b1 to A3 (`orders/`, `combat.rs`, `gather.rs`) and, in stage 2, `PRES-MODELS` and `FX2` (`render.rs`, `visuals/`). The MP1 owner of the forward merge (section 3.4 names the owner of each file) re-pins each record whose patch no longer applies, and runs `python3 -I scripts/qa/mp1_plants.py --files <the merged paths> --native-only; echo exit=$?` red in the same merge. The pixel records of those files are re-pinned in the same merge and run red in the next REG2 table run.
- **Pixel rows in the lane (rulings R-A, R-U; qa N4).** A task that writes a pixel row runs, in its own lane and before merge, one pre-merge headed session through the wrapper (mode `pre-merge`, 1 headed run, counted in the Headed cell): the row green, then **every pixel plant of the task red**, each restored, then the row green again. Each red writes its output into the record of the plant. The task also adds `node --test tests/browser/<row>-oracle.test.js` with one red fixture for each plant (no GPU), as MC did (`tests/browser/pt1-oracle.test.js`). Each fixture is the recorded output of a real plant run of that session, never a hand-made file; the reviewer checks the fixture against the run receipt. The batch `I-PIX1` (B1) and `I2-PIX` (B2) re-run the rows.
    - **The 20-minute bound.** One pre-merge session holds the lock for at most 20 minutes (rule 1 of the lock queue below). The plan assumes that one row run takes at most 2 minutes (NOT TRACED; `E-GPU` and the `Q-LOCK` receipts measure it), so one session holds the green run, at most 8 plants and the last green run. A task with more than 8 pixel plants, or whose plants need more than 20 minutes, splits them over two pre-merge sessions (Headed 2).
    - **Pixel plants for each task** (from block `mp1-plants`; the split between pixel and headless plants of a mixed row is the producer's reading, the builder confirms it): `L-LOBBY-C1` 1 (run 4); `L-FX-PRES` 3 (flash, death, offline mark); `UI-PLACE` 5 (GHO1 (a) to (e)); `UI-HUD` 6 (HUD5 (a) to (c), HUD6 (b), (d), KEY1 (a)); `UI-ALERT` 5 (ALR2 (d), (i), "two alerts for one wait", the two MCX1 alert-list plants); `UI-RESULT` 1; `UI-LABEL` 1; `WEB-NOGPU` 1; `I-PIX1` 6 (GOAL1 (a), (b), (c), (e), (f), (g): plant (f) waits up to 300 s, so `I-PIX1` splits them over two pre-merge sessions, 3 and 3); `L-LOBBY2-UI` 4; `UI2-RESULT` 1; `UI2-CONN` 4; `UI2-MC` the MCX1 plants of the O15 list (count NOT TRACED; the builder counts them and splits above 8); `UI2-HUD` 5; `FX2` 2; `C2-SOUND` 3; `PRES-ART` 3; `PRES-MEASURE` 2; `MEAS-EXIT` 2. Each other headed task has no pixel plant of its own.
- **Assertions.** SP section 1a applies: a builder never changes an assertion or an expected value on his own decision. The four permitted kinds are inversion, observation change, deletion and registry re-pin. This plan names each place in the task block. Any other change stops the task.
- **Frozen value.** `tests/m3/m2_operation_stream.json` keeps its hash. Until Q2 merges forward, it is the SP pin `bcbf72c2…e08e370b`; after Q2, the new hash of the Q2 vault note. No MP1 task changes it (D:2289). Every task that changes `turts_sim`, `turts_pack` or `turts_server` runs F-FROZEN in its gate (qa L1); a red of `m2_operation_stream_matches_baseline` stops the task.
- **GPU lock and headed runs (refinement R9; ruling R-T, qa N1).** Headed browser runs take the one GPU lock through the wrapper of task `Q-LOCK`: `/home/julien/Projects/TuRTS/target/mc-evidence/gate-runner/mc_headed_v6.sh` (outside the repository; `Q-LOCK` writes it and records its sha256 here when it merges). Until `Q-LOCK` merges, no MP1 headed run starts. The wrapper of today is `mc_headed_v4.sh` (sha256 `66c105e5904357c5916b4a015517a1542ea2f8f16ea0128b3d2051a554cec235`, checked again 2026-11-12); its last command is `exec flock -x -o "$MC_EVIDENCE_BASE/gpu-runtime-2.lock"` with no queue order and no hold limit (read 2026-11-12), so it cannot enforce an order.
    - Each client task writes its pixel row files (driver, oracle, oracle unit test, row JSON, plant records). It passes F-DRV in its lane.
    - Only the tasks with a "Headed" count above 0 take the lock (42 MP1 headed runs, section 14.2). Every headed MP1 task descends from `Q-LOCK` (a plain or stacked input chain; the check script tests descent) (the script checks it, 3.5).
    - **The lock queue rule (four points):**
        1. A pre-merge F-HEAD run (wrapper mode `pre-merge`) holds the lock for its rows and its plants, at most 20 minutes. It builds every wasm build that it needs (the green build and one build for each pixel plant, each in its own output directory) before it takes the lock; under the lock the wrapper runs only the browser.
        2. Table runs (wrapper mode `table`: B1 in `I-PIX1`, B2 in `I2-PIX`, REG2 in `REG2-1` and `REG2-2`, and the pixel plants that REG2 runs) start a row only when no pre-merge run waits, and they free the lock between rows (one wrapper call for each row).
        3. The slow gate runs `mp1_plants.py --all --native-only`; pixel plants run only in REG2.
        4. The wrapper has a hold limit (`timeout "$MC_LOCK_MAX_SECS"`; 1,200 s in mode `pre-merge`) and writes the wait time and the hold time to the run receipt. Root reads the receipts each day and records a hold above the limit or a wait above 20 minutes on the board.
- **Model tiers.** The Tier cell is "builder/reviewer": S is Sonnet 5.5, H is Haiku 5.5, O is Opus 5.5. Planned code goes to S or H. Reviews, `L0-EXIT`, `L0-EXIT2`, the `player` checks and the audit go to O.
- **Git.** Stage explicit paths. Never `git add -A`. Build each binary with its own `cargo build -p <crate>`. Builders do not edit `CLAUDE.md`, `.claude/agents/`, D, the MC design or the product spec; they put each doc change in their report for `REC`.
- **Records.** Root counts each dispatch on a board (`vault/MP1 Execution Board.md`; NOT TRACED that it exists). The record of each task: red, green, plant record file, changed paths, commit, run count. A task that uses twice its estimate is reported; it is not halted (C391 item 7).
- **Stop and report, do not decide:** a frozen value changes; a step contradicts D or an applied erratum; a plant has no red; an open decision of D section 2 or S 15 is needed; a lane-0 contract item must change (section 5.2).

## 2. Plan refinements of D (each needs Root's acceptance)

"Kind": REF is a refinement that changes no gated D statement; DCH changes one, and section 22 has its erratum (rigour R and Q table, `gate2-rigour-detail.txt:4-25`).

| # | Kind | Refinement | Reason (evidence) | Cost if refused |
| --- | --- | --- | --- | --- |
| R1 | REF | Lane-0 parts that touch no `turts_sim` file and no file of an in-flight slice task start now: `L0-RAW`, `L0-RULES`, `L0-DBG`, `L-ROOM0`, `E-GPU`, `Q-PLANTS`, `PRES-PROBE`. D:2304 puts lane 0a at E1. | The SP task table names no `turts_mod_runtime`, `plugin.rs`, `replica.rs` or `debug.rs` file (grep count 0). S45 merged and touched none of `turts_client/src/lib.rs`, `debug.rs`, `replica.rs`, `turts_client_web/src/lib.rs` (section 20). | lane 0 ends 2 runs later |
| R2 | REF | `L0-API1` takes `L0-RULES` as input and replaces the five exhaustive `SlotRelation` matches by `turts_rules::relation` (X10). | New variants `Ally`, `Enemy` break `orders.rs:1126`, `economy/combat.rs:313`, `core_adapter.rs:340`, `widgets/cursor.rs:430`, `:619` (grep at `03aef38`). D:767 wants one function. | two copies of the relation rule |
| R3 | REF | Lane 0 builds the whole Validation column of D 6.10, except rows V1 and V2 (`L0-API2`). Feature lanes then edit only `unbuilt.rs` in `turts_mod_runtime`. | D:1388-1393 freezes shapes in lane 0. | about 10 concurrent edits of the validation files |
| R4 | REF | Lane 0 creates one stub file or function for each feature at each seam: export writers (`export_*.rs`), behaviour handlers (`L0b-ORD`), sim system slots and `power::weapon_enabled` (`L0-SIM`), client source and effect stubs and the split draw and effect functions (`L0-CORE`, ruling R-S). | D:1388-1393 ("writers empty"); section 3.4 | the conflict list grows by about 40 pairs |
| R5 | REF, D note E16 | UnitIndex site 6 (`export.rs:299`) moves to `L0-WIRE`; the parse at `combat.rs:265` moves to `L-DMG`; hot site 3 (`gather.rs:1206`, fn `find_unit` at `:1204`) moves from lane 0b to `L-OBS`. | export.rs is the wire lane's file; `combat.rs` waits for A2c and A3 (D:2304); `gather.rs` waits for A2b2 | lane 0b waits for A2b2 |
| R6 | REF | `L0b-ORD` also builds the engine order source (`Engine` through `apply_order`, the order-log source). | `L-CON1`, `L-STO`, `L-RLY` all need it (D:737-751). | three lanes build it |
| R7 | REF | The sim parts of X11 and X10 go to `L-MATCH-SIM` (`simulation-core-engineer`). `L-MATCH-SRV1` (`backend-engineer`) keeps the room and builds the server-side `LobbyState` writer of X12. | D:2203; D:257, D:258 | one seat edits two crates |
| R8 | DCH, E6, E24 | The merge-after and stacked edges of section 1, with the cost model `e = max(s + runs, end[a] + 1)` (ruling R-G). | D:2295 says "a task starts when its inputs are merged". Plain inputs only give a path of 73 runs (section 20); this model gives 60. | +13 runs on the path |
| R9 | REF, D note E15 | Pixel rows run green and red in their lane (R-A) and again in the batches B1 and B2. | D:2324: "Each feature lane owns its own pixel rows". The batches re-run them. | none |
| R10 | REF | Integration branch `mp1/integration` with forward merges only. | SP section 1 forbids other merges into the slice branch. | MP1 and the slice share one tree |
| R11 | REF | The Countdown state moves from X11b rest (`L-MATCH-SRV2`) to `L-LOBBY2-SRV` (X12 rest). | The countdown serves only the lobby. | +3 runs in stage 2 |
| R12 | REF | Open point 11 (a pack check that each building can be placed): dropped from MP1, moved to M4 with yardstick item CO03 (`later:M4`, D:2850). | D:2571 allows a drop with a reason. | `L0-API2` +1 to 2 runs |
| R13 | REF | Open point 12 (replica cost): moved to M7 with the recorded numbers (`MEAS-EXIT` records native `apply_delta` p50 and p99). | D:2576-2577 allows the move; measured 0.029 ms (400 units) and 0.363 ms (5,000 units) for one clone (`docs/research/mp1/regate-performance.md:16`). | `L0-WIRE` +1 to 2 runs |
| R14 | DCH, E4 | The `Credited` effect and the "+N" float move from X14 rest to stage 1 (`L-FX-WIRE`, `L-FX-PRES`), with the offline mark on an unpowered turret (ux L3). | GOAL1 (stage 1) needs the first "+300" (D:2461, plant (e)); the Player path step 10 shows the offline mark. | GOAL1 cannot go green in stage 1 |
| R15 | STO1 REF; ALR2 DCH, E5 | Rows STO1 and ALR2 are split into a stage-1 part and a stage-2 part. | D:2424 and D:2462 already split STO1; D:2427 lists ALR2 whole in stage 1. | STO1 or ALR2 cannot go green in stage 1 |
| R16 | REF | No MP1 task edits the shared fault registry `tests/m2/refutations.json`. Each task writes its own plant record file (ruling R-B); `Q-PLANTS` gives the runner and the `mp1` set; REG2-1 and REG2-2 run `--all`. | The registry is one hot file (SP:147). | a shared hot file for 40 tasks |
| R17 | REF, D note E12 | The sim gate of D 6.7.5 (D:900-905) is built in `L0-SIM`. `SimPlugin` inserts no `MatchConfig`. `headless_app` (`crates/turts_sim/src/harness.rs:11-13`, 51 files use it) inserts the `Immediate` config (team = slot, start tick 1). The server inserts the config by its start policy: at boot under `Immediate`, in the start step under a lobby policy. The four `SimPlugin` builders in `session.rs` tests (`:792`, `:963`, `:1116`, `:1747`) insert it too (ruling R-K, TD M4). | A harness with no config gets 0 ticks and fails loudly; no default runs in production. | the sim can tick in the lobby |
| R18 | DCH, E11, E23 | One client store for the new 6.8 records (`L0-STORE`, ruling R-H); it is staged with the candidate through the whole resync chain and commits with it (TD H-B). | TD H1, H-B: five UI lanes read these records; `apply_snapshot` stages, replays and commits once (`crates/turts_client/src/replica.rs:216-276`, `:268`). | five UI lanes build storage; a per-message store apply puts the store ahead of the replica |
| R19 | REF | One writer function for order-state changes, checked by a syn scan (`L0-WIRE`, ruling R-I). | TD H3: about 62 order-state write lines (pattern count, `gate2-technical-director.md:14`). | a missed writer drops a change with no error |
| R20 | DCH, E14 | `L0-EXIT` freezes the stage-1 shapes; `L0-EXIT2` (after C1) freezes the stage-2 shapes. No second protocol or API step (ruling R-J). | TD M1. | stage-2 shapes are fixed before C1 can teach anything |
| R21 | DCH, E7, E8, E9 | MCX1 alert-list part and NOGPU1 move to stage 1; NC1 moves to stage 2 (block `mp1-moves`, section 13.2). | ux M5, ux H2; TD stage note (rulings R-N, R-D, R-Q). | a person meets the defect in the first 3 minutes |
| R22 | DCH, E10 | Name labels over units in stage 1, as a generic option of the `world` style (6.10 row H24; engine request E3; ruling R-E). | ux H1: the stage-1 shapes alone do not name the type. | a person cannot tell the types apart at C1 |
| R23 | DCH, E1, E2, E3 | No three-slot MAT1 case; exit-item-5 scenes at 2 slots (ruling R-F). | rigour H1: `SLOT_COUNT` is 2 (`crates/turts_sim/src/slots.rs:7`). | C1 cannot pass (a stage-1 row part cannot be built) |
| R24 | DCH, E19 | The `player` seat judges a recorded evidence set made by a scripted driver; it does not drive a browser (ruling R-D). | qa M3; MC PT1 used a scripted driver (MC design :1191). | the blind check is not repeatable |
| R25 | DCH, E13 | Start credits 1,400 (ruling R-R, from `C0-PACE`). | `docs/research/mp1/c0-pace-stage1-pacing.md`. | holds of 7.2 s and 14.5 s on the stage-1 hint order |
| R26 | REF | Task `Q-LOCK` (ruling R-T): the wrapper with the queue rule, the hold limit and the receipt fields; every headed MP1 task takes it as an input. | qa N1: `mc_headed_v4.sh` ends in `exec flock -x -o …/gpu-runtime-2.lock` with no order and no limit. | short pre-merge runs can starve behind a table run |
| R27 | DCH, E26 | Each lane runs all its pixel plants red in its one pre-merge session; every red fixture is the output of a real plant run (ruling R-U). | qa N4: revision 2 ran one plant red and did not say what made the other fixtures. | the first real red of most pixel plants is in `REG2-1` |
| R28 | REF | Task `L0-SPLIT` (ruling R-X): the two pure moves of `L0-CORE` become a task with its own review and a behaviour oracle; `L0-CORE` falls to 3 runs. | TD M-B: a moved early `return` keeps the joined-body text equal; `draw_world_visuals` returns early at `crates/turts_client/src/visuals.rs:343` and shares `desired` (`:437`), `ring_metadata` (`:438`), `health_fractions` (`:621`) (read at `d7d7e6d`). | a control-flow change in a pure move has no red |
| R29 | REF | `L-PWR` takes `L0-EXIT` and `L-DMG` as inputs and merges after `L-CON1`; `L0-SIM` adds the stub `power::rate_factor` (the `weapon_enabled` pattern); `L-OBS` takes A3 as a cross stack (TD sequencing proposal, rulings R-W, R-X). | TD scheduler run on copies: 57 runs (section 3.1). | path 60 runs |
| R30 | DCH, E21 | The job record gains `held` and `held_since_tick` (ruling R-Y); the 6.4 job state stays per tick. | rigour H1: the free income pays 1 tick in about 12, so the per-tick state flickers (`c0-pace-stage1-pacing.md` section 3). | hint X3 never shows, or Q11 blocks `L-PAY` |

## 3. Dependency graph

The table below is the single source. The script reads it. Stage `0` is lane 0, `1` stage 1, `2` stage 2, `slice` a remaining slice task (inputs and runs from SP section 3; S1, Q1a, G1, A1a, S3, S45 and Q1b are merged and are not listed; Q1b merged during this revision, so A2a and A2c lose it as an input; A2a takes D0p, from the Q1b amendment of SP: "After D0p merges, and before A2a starts: emit the base trace again"). "Runs" is the point estimate from now; "Range" is low-high. "Headed" counts the runs that take the GPU lock. An input with `^` is stacked (section 1). `C0-PACE` is done (1 run spent; output `docs/research/mp1/c0-pace-stage1-pacing.md`).

<!-- mp1-tasks:start -->
| Task | Stage | Builder | Reviewer | Tier | Inputs | Merge after | Runs | Range | Headed | Rows | Scope | Hot files |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| D0 | slice | (MC lane) | (MC) | - | - | - | 0 | 0-0 | 0 | - | - | - |
| D0p | slice | backend-engineer | simulation-core-engineer | - | D0 | - | 2 | 2-2 | 0 | - | - | replay.rs, gather.rs, orders/, economy-test.rs |
| S2 | slice | simulation-core-engineer | technical-director | - | D0p | - | 2 | 2-2 | 0 | - | - | orders/ |
| A1b | slice | backend-engineer | qa-lead | - | D0p, S2 | - | 4 | 4-4 | 1 | - | - | replay.rs, turts_audit/, session.rs, economy-test.rs |
| A2a | slice | simulation-core-engineer | qa-lead | - | D0p | - | 4 | 4-4 | 0 | - | - | movement/ |
| A1c | slice | backend-engineer | qa-lead | - | A1b | - | 2 | 2-2 | 1 | - | - | replay.rs, turts_audit/, economy-test.rs |
| A1d | slice | backend-engineer | qa-lead | - | A1c | - | 2 | 2-2 | 0 | - | - | pack-tests-common/ |
| A2b1 | slice | simulation-core-engineer | qa-lead | - | A2a, A1c, S2 | - | 3 | 3-3 | 0 | - | - | movement/, orders/, gather.rs |
| A2b2 | slice | simulation-core-engineer | qa-lead | - | A2b1 | - | 2 | 2-2 | 0 | - | - | movement/, orders/, gather.rs, views.rs |
| A2c | slice | simulation-core-engineer | qa-lead | - | A2b2 | - | 4 | 4-4 | 0 | - | - | orders/, combat.rs, transaction.rs, views.rs |
| Q2 | slice | qa-lead | technical-director | - | A2c | - | 3 | 3-3 | 0 | - | - | - |
| A4 | slice | simulation-core-engineer | performance-architect | - | A2b2 | Q2 | 2 | 2-2 | 0 | - | - | movement/ |
| A3 | slice | simulation-core-engineer | performance-architect | - | A4^ | Q2 | 3 | 3-3 | 0 | - | - | movement/, grid.rs, combat.rs, orders/ |
| REG | slice | qa-lead | (Root) | - | Q2, A3, A1d | - | 2 | 2-2 | 2 | - | - | - |
| TDN | slice | technical-director | (Root) | - | REG | - | 1 | 1-1 | 0 | - | - | - |
| L-ROOM0 | 0 | backend-engineer | network-engineer | S/O | - | - | 3 | 2-4 | 0 | - | X11b, X12 | room.rs |
| L0-RAW | 0 | mod-runtime-engineer | mod-builder | S/O | - | - | 2 | 1-2 | 0 | - | X0, X19 | version_table.rs, content_mc_validation.rs |
| L0-RULES | 0 | simulation-core-engineer | technical-director | S/O | - | - | 2 | 2-3 | 0 | - | X0, X10 | Cargo.toml, boundary.sh |
| L0-DBG | 0 | bevy-rendering-engineer | performance-architect | S/O | Q-LOCK | - | 2 | 1-2 | 1 | DBG1 | X0 | client-lib.rs, debug.rs, replica.rs, web-lib.rs |
| C0-PACE | 0 | game-designer | player | H/O | - | - | 0 | 0-0 | 0 | - | X15 | probes/c0-pace/ |
| E-GPU | 0 | qa-lead | performance-architect | S/O | Q-LOCK | - | 1 | 1-2 | 1 | - | X20 | mp1.md |
| Q-PLANTS | 0 | qa-lead | performance-architect | S/O | - | - | 2 | 2-3 | 0 | - | X20, X22 | mp1_plants.py, tests/mp1/ |
| PRES-PROBE | 0 | bevy-rendering-engineer | performance-architect | S/O | - | - | 1 | 1-2 | 0 | - | X16 | probes/pres/ |
| Q-LOCK | 0 | qa-lead | performance-architect | S/O | - | - | 1 | 1-2 | 0 | - | X20 | gate-runner/ |
| L0-SPLIT | 0 | bevy-rendering-engineer | technical-director | S/O | - | - | 2 | 2-3 | 0 | - | X0, X5 | visuals.rs, visuals/, core_adapter.rs, core_adapter/ |
| L0-API1 | 0 | mod-runtime-engineer | mod-builder | S/O | L0-RAW, L0-RULES, S2, D0p | - | 4 | 3-5 | 0 | - | X0, X19, X10 | schema.rs, version_table.rs, unbuilt.rs, orders/, combat.rs, replay.rs, session.rs, core_adapter.rs, cursor.rs, values.rs, sources.rs, feedback.rs |
| L0-API2 | 0 | mod-runtime-engineer | mod-builder | S/O | L0-API1 | - | 3 | 2-4 | 0 | VAL1 | X0, X19 | mp1_validation.rs |
| L0-SIM | 0 | simulation-core-engineer | performance-architect | S/O | S2, D0p, Q-LOCK | - | 4 | 3-5 | 1 | - | X0, X0e | plugin.rs, harness.rs, production.rs, transaction_m3.rs, orders/, transaction.rs, power.rs |
| L0-WIRE | 0 | network-engineer | backend-engineer | S/O | S2, D0p, L0-DBG | - | 5 | 4-6 | 0 | - | X0, X19 | messages.rs, protocol_plugin.rs, events.rs, orders/, export.rs, session.rs, replica.rs, join.rs, ingest.rs, checks-tests/ |
| L0-STORE | 0 | network-engineer | bevy-rendering-engineer | S/O | L0-WIRE | - | 2 | 2-3 | 0 | - | X0, X19 | replica.rs, replica_store.rs |
| L0-CORE | 0 | bevy-rendering-engineer | technical-director | S/O | L0-API1, L0-SPLIT | - | 3 | 2-4 | 0 | - | X0, X5 | interaction/, widgets/, core_adapter/, feedback.rs |
| L0-EXIT | 0 | technical-director | mod-builder | O/O | L0-API1, L0-API2, L0-WIRE, L0-STORE, L0-RULES, L0-SIM, L0-CORE, Q-PLANTS | - | 1 | 1-2 | 0 | - | X0, X19 | - |
| L0b-ORD | 0 | simulation-core-engineer | technical-director | S/O | A1c, L0-SIM, L0-EXIT | - | 2 | 2-3 | 0 | - | X0 | orders/, orderlog.rs |
| L-MATCH-SIM | 1 | simulation-core-engineer | backend-engineer | S/O | L0-EXIT | - | 3 | 2-4 | 0 | - | X11, X10 | plugin.rs, events.rs, match_state.rs |
| L-MATCH-SRV1 | 1 | backend-engineer | network-engineer | S/O | L-ROOM0, L-MATCH-SIM | - | 5 | 4-7 | 0 | IMM1, MAT1/s1 | X11b, X12 | session.rs, main.rs, config.rs, stats.rs, outbound.rs, room.rs, mods/standoff/, unbuilt.rs, server-tests-support/ |
| L-LOBBY-C1 | 1 | network-engineer | web-platform-engineer | S/O | L0-EXIT, Q-LOCK | L-MATCH-SRV1 | 3 | 3-5 | 1 | LOB1/s1, LEAVE1/s1 | X12 | join.rs, rejoin.rs, actions.rs, client-main.rs, web-lib.rs, client_shell/ |
| L-PAY | 1 | simulation-core-engineer | qa-lead | S/O | L0-EXIT, A1d | - | 3 | 3-5 | 0 | PAY1, PAY2 | X7 | production.rs, turts_audit/, unbuilt.rs |
| L-INC | 1 | simulation-core-engineer | qa-lead (joint review in L-PAY) | S/O | L0-EXIT, A1d | L-PAY | 1 | 1-2 | 0 | INC1 | X26 | income.rs, turts_audit/, unbuilt.rs |
| L-DMG | 1 | simulation-core-engineer | qa-lead | S/O | L0-EXIT | - | 2 | 2-3 | 0 | DMG1 | X2 | combat.rs, unbuilt.rs |
| L-TGT | 1 | simulation-core-engineer | performance-architect | S/O | L-DMG, L-MATCH-SIM, A1c, A3^ | L-PWR | 4 | 3-5 | 0 | TGT1, TGT2 | X2 | combat.rs, acquire.rs, orders/, unbuilt.rs |
| L-CON1 | 1 | simulation-core-engineer | qa-lead | S/O | L0b-ORD, L-PAY | - | 6 | 5-8 | 0 | CON1, PRD1 | X4 | orders/, production.rs, construction.rs, server-orders.rs, unbuilt.rs |
| L-PWR | 1 | simulation-core-engineer | qa-lead | S/O | L0-EXIT, L-DMG | L-CON1 | 3 | 2-4 | 0 | PWR1 | X6 | power.rs, export_supply.rs, unbuilt.rs |
| L-DES | 1 | simulation-core-engineer | qa-lead | S/O | L-CON1 | - | 3 | 2-4 | 0 | DES1 | X1 | transaction_m3.rs, production.rs, construction.rs, unbuilt.rs |
| L-OBS | 1 | simulation-core-engineer | performance-architect | S/O | A2b2, L-CON1, L0-API2, A3^ | - | 3 | 2-4 | 0 | OBS1 | X0e, X4 | gather.rs, mp1.md |
| L-STO | 1 | simulation-core-engineer | network-engineer | S/O | L0b-ORD, L-MATCH-SRV1 | A2b2 | 2 | 2-3 | 0 | - | X24 | orders/, gather.rs, unbuilt.rs |
| L-FX-WIRE | 1 | network-engineer | bevy-rendering-engineer | S/O | L-DMG | A2b2 | 2 | 1-2 | 0 | - | X14 | combat.rs, export.rs, ingest.rs, gather.rs |
| L-FX-PRES | 1 | bevy-rendering-engineer | performance-architect | S/O | L0-EXIT, Q-LOCK | L-FX-WIRE | 4 | 3-5 | 1 | FX1/s1 | X14 | visuals.rs, visuals/, debug.rs |
| UI-PLACE | 1 | bevy-rendering-engineer | ux-designer | S/O | L0-EXIT, Q-LOCK | L-CON1 | 6 | 5-7 | 1 | CON2, GHO1 | X5 | interaction/, visuals.rs, visuals/, core_adapter/, boundary.sh, cursor.rs |
| UI-HINT | 1 | bevy-rendering-engineer | ux-designer | S/O | L0-EXIT | UI-PLACE | 3 | 2-4 | 0 | - | X25 | interaction/, core_adapter/, debug.rs |
| UI-HUD | 1 | bevy-rendering-engineer | ux-designer | S/O | L0-EXIT, Q-LOCK | L-PAY, L-CON1, L-PWR | 6 | 5-8 | 1 | HUD5, HUD6/s1, KEY1 | X6, X7 | widgets/ |
| UI-ALERT | 1 | bevy-rendering-engineer | ux-designer | S/O | L0-EXIT, UI-HUD^, Q-LOCK | L-PWR, L-STO, L-PAY, L-CON1 | 5 | 4-6 | 1 | ALR2/s1, MCX1/s1 | X15, X18 | feedback.rs, audio.rs, interaction/, core_adapter/, widgets/ |
| UI-RESULT | 1 | bevy-rendering-engineer | ux-designer | S/O | L0-EXIT, L-LOBBY-C1, Q-LOCK | - | 3 | 2-4 | 1 | RES1/s1 | X13 | widgets/, client_shell/, core_adapter/ |
| UI-LABEL | 1 | bevy-rendering-engineer | ux-designer | S/O | L0-EXIT, Q-LOCK | - | 2 | 2-3 | 1 | - | X15 | visuals.rs, visuals/, unbuilt.rs |
| WEB-NOGPU | 1 | web-platform-engineer | bevy-rendering-engineer | S/O | L0-DBG, Q-LOCK | - | 2 | 1-3 | 1 | NOGPU1 | X18 | web-lib.rs |
| C-UNITS | 1 | game-designer | mod-builder | S/O | L0-API2, C0-PACE | - | 3 | 3-4 | 0 | - | X15, X24, X26 | mods/frontier/ |
| C-UI | 1 | ux-designer | player | S/O | C-UNITS | - | 4 | 3-5 | 0 | - | X15, X25 | mods/frontier/, frontier_texts.rs |
| C1-FINAL | 1 | game-designer | mod-builder | S/O | C-UI, L-DMG, L-TGT, L-PAY, L-INC, L-CON1, L-DES, L-PWR, L-OBS, L-STO, L-MATCH-SRV1, UI-HINT, UI-HUD, UI-ALERT, UI-RESULT, UI-PLACE, UI-LABEL, L-FX-PRES, WEB-NOGPU, A3 | - | 3 | 2-4 | 0 | - | X15, X0e | mods/frontier/, mp1.md |
| I-HEAD | 1 | qa-lead | network-engineer | S/O | C1-FINAL, L-LOBBY-C1 | - | 3 | 2-4 | 0 | ORDW1, STO1/s1 | X20 | mp1.md |
| I-PIX1 | 1 | qa-lead | bevy-rendering-engineer | S/O | C1-FINAL, L-LOBBY-C1, Q-LOCK | - | 4 | 3-5 | 3 | GOAL1 | X20 | tests/browser/ |
| I-PLAY1 | 1 | qa-lead | web-platform-engineer | S/O | C1-FINAL, L-LOBBY-C1, Q-LOCK | - | 3 | 2-4 | 2 | - | X20, X21 | tests/browser/, mp1-play.sh |
| C1-PLAYER1 | 1 | player | (none) | O/- | I-PLAY1 | - | 1 | 1-1 | 0 | - | X21 | - |
| REG2-1 | 1 | qa-lead | technical-director | S/O | I-PIX1, I-HEAD, I-PLAY1, L0-API2, L0-DBG, Q-LOCK | - | 3 | 2-4 | 3 | REG2/s1 | X22 | - |
| C1-PLAYER | 1 | player | (none) | O/- | REG2-1, C1-PLAYER1 | - | 1 | 1-1 | 0 | - | X21 | - |
| C1-PERSON | 1 | qa-lead | (person) | S/- | C1-PLAYER, Q-LOCK | - | 1 | 1-2 | 1 | - | X21 | - |
| L0-EXIT2 | 2 | technical-director | mod-builder | O/O | C1-PERSON | - | 1 | 1-2 | 0 | - | X0, X19 | - |
| I-NC1 | 2 | qa-lead | mod-runtime-engineer | S/O | C1-PERSON | - | 2 | 1-2 | 0 | NC1 | X20 | support/mod.rs |
| L-CON3 | 2 | simulation-core-engineer | qa-lead | S/O | L0-EXIT2 | - | 2 | 2-3 | 0 | CON3 | X4b | construction.rs, orders/, unbuilt.rs |
| L-REP | 2 | simulation-core-engineer | qa-lead | S/O | L0-EXIT2 | - | 3 | 3-4 | 0 | REP1 | X8 | orders/, combat.rs, unbuilt.rs |
| L-RLY | 2 | simulation-core-engineer | network-engineer | S/O | L0-EXIT2 | - | 3 | 2-4 | 0 | RLY1, STO1/s2 | X9, X24 | orders/, gather.rs, unbuilt.rs |
| L-MATCH-SIM2 | 2 | simulation-core-engineer | backend-engineer | S/O | L0-EXIT2 | - | 3 | 3-4 | 0 | - | X11c, X13 | match_state.rs, events.rs, orders/, unbuilt.rs |
| L-LOBBY2-SRV | 2 | backend-engineer | network-engineer | S/O | L0-EXIT2 | - | 3 | 2-4 | 0 | LEAVE1/s2 | X12, X11b | room.rs, session.rs, unbuilt.rs |
| L-MATCH-SRV2 | 2 | backend-engineer | network-engineer | S/O | L-MATCH-SIM2, L-LOBBY2-SRV | - | 3 | 3-4 | 0 | MAT1/s2 | X11b, X11c | session.rs, room.rs, config.rs, server-orders.rs, mods/standoff/, actions.rs |
| L-LOBBY2-UI | 2 | bevy-rendering-engineer | ux-designer | S/O | L-LOBBY2-SRV, Q-LOCK | - | 4 | 3-6 | 1 | LOB1/s2, LOB2 | X12 | client_shell/, actions.rs, join.rs |
| UI2-RESULT | 2 | bevy-rendering-engineer | ux-designer | S/O | L0-EXIT2, Q-LOCK | L-MATCH-SRV2 | 2 | 2-3 | 1 | RES1/s2 | X13 | widgets/, actions.rs |
| UI2-CONN | 2 | bevy-rendering-engineer | ux-designer (joint review in UI2-RESULT) | S/O | L0-EXIT2, Q-LOCK | L-MATCH-SRV2 | 2 | 1-3 | 1 | SUR1, ALR2/s2 | X11c | client_shell/, feedback.rs |
| UI2-MC | 2 | bevy-rendering-engineer | ux-designer | S/O | L0-EXIT2, Q-LOCK | - | 3 | 3-5 | 1 | MCX1/s2 | X18 | widgets/, client_shell/, feedback.rs |
| UI2-HUD | 2 | bevy-rendering-engineer | ux-designer | S/O | L-CON3, L-RLY, L-REP, UI2-MC, Q-LOCK | - | 4 | 3-6 | 1 | HUD6/s2, SMT1 | X4b, X8, X9 | widgets/, interaction/, cursor.rs |
| PRES-MEASURE | 2 | bevy-rendering-engineer | performance-architect | S/O | C1-PERSON, Q-LOCK | - | 2 | 2-3 | 2 | - | X16 | mp1.md |
| PRES-MODELS | 2 | bevy-rendering-engineer | mod-runtime-engineer | S/O | PRES-MEASURE, L0-EXIT2 | - | 5 | 4-7 | 0 | - | X16 | visuals.rs, visuals/, render.rs, content.rs, acquire.rs, client-lib.rs, web-lib.rs, Cargo.toml, mp1_validation.rs |
| FX2 | 2 | bevy-rendering-engineer | performance-architect | S/O | L0-EXIT2, Q-LOCK | PRES-MODELS | 4 | 3-5 | 1 | FX1/s2 | X14 | visuals.rs, visuals/, audio.rs, render.rs |
| C2-SOUND | 2 | game-designer | player | S/O | L0-EXIT2, Q-LOCK | FX2 | 2 | 2-3 | 1 | AUD2 | X17, X14 | mods/frontier/ |
| PRES-ART | 2 | bevy-rendering-engineer | ux-designer | S/O | PRES-MEASURE, L0-EXIT2, Q-LOCK | PRES-MODELS | 4 | 3-6 | 1 | MDL1 | X16 | mods/frontier/, scripts/mp1_models.py |
| C2-CONTENT | 2 | game-designer | mod-builder | S/O | L-RLY, L-REP, L-CON3, L-MATCH-SIM2 | L-MATCH-SRV2, UI2-RESULT, UI2-CONN, UI2-HUD, UI2-MC | 3 | 2-4 | 0 | - | X15 | mods/frontier/ |
| I2-L | 2 | web-platform-engineer | backend-engineer | S/O | L-MATCH-SRV2, Q-LOCK | - | 1 | 1-1 | 1 | - | X20 | mp1.md |
| I2-PIX | 2 | qa-lead | bevy-rendering-engineer | S/O | C2-CONTENT, C2-SOUND, PRES-ART, UI2-MC, L-LOBBY2-UI, UI2-HUD, UI2-CONN, UI2-RESULT, I2-L, FX2, Q-LOCK | - | 5 | 4-7 | 5 | - | X20 | tests/browser/ |
| MEAS-EXIT | 2 | qa-lead | performance-architect | S/O | I2-PIX, Q-LOCK | - | 2 | 2-3 | 2 | VIS3 | X20 | mp1.md |
| REG2-2 | 2 | qa-lead | technical-director | S/O | I2-PIX, I-NC1, Q-LOCK | - | 3 | 2-4 | 3 | REG2/s2 | X22 | - |
| AUDIT | 2 | technical-director | (none) | O/- | REG2-2 | - | 2 | 2-2 | 0 | - | X23 | - |
| PT3-PLAYER | 2 | player | (none) | O/- | REG2-2 | - | 1 | 1-1 | 0 | - | X21 | - |
| PT3-PEOPLE | 2 | qa-lead | (people) | S/- | PT3-PLAYER, MEAS-EXIT, Q-LOCK | - | 1 | 1-2 | 1 | PT3 | X21 | - |
| REC | 2 | qa-lead | (Root) | H/O | PT3-PEOPLE, AUDIT | - | 1 | 1-1 | 0 | - | X22 | mp1.md |
<!-- mp1-tasks:end -->

### 3.1 Critical path and start windows

Script: `python3 -I docs/research/mp1/probes/plan/mp1_plan_check.py docs/superpowers/plans/2026-11-12-mp1-playable-skirmish.md docs/superpowers/specs/2026-11-12-mp1-playable-skirmish-design.md; echo exit=$?` (exit 0; output in section 20).

**Critical path: 57 runs from now**, plus the D0 wait (revision 2: 60):
D0p → S2 → A1b → A1c → A2b1 → A2b2 → A2c → Q2 → A4 → A3 → `L-TGT` → `C1-FINAL` → `I-PIX1` → `REG2-1` → `C1-PLAYER` → `C1-PERSON` → `L0-EXIT2` → `L-MATCH-SIM2` → `L-MATCH-SRV2` → `UI2-RESULT` → `C2-CONTENT` → `I2-PIX` → `REG2-2` → `PT3-PLAYER` → `PT3-PEOPLE` → `REC`.

- The path goes through the slice again, up to A3, with 0 slack (the TD measured 57, 37 and 56 on copies; this run gives the same). Lane 0 exits at offset 12 (`L0-EXIT`). `C1-PLAYER1` (the first-minute check) ends at 32, `REG2-1` at 35, **C1 (`C1-PERSON`) at 37**, `L0-EXIT2` at 38, `I2-PIX` at 51, `REG2-2` at 54, **PT3 (`PT3-PEOPLE`) at 56**, `REC` at 57.
- Revision 2 said 60, 40 and 59. The differences (TD sequencing proposal, rulings R-W, R-X): `L0-SPLIT` (2 runs, from now) with `L0-CORE` at 3 runs; `L-PWR` builds from `L0-EXIT` and `L-DMG` and merges after `L-CON1`; `L-OBS` takes A3 as a cross stack. `Q-LOCK` (1 run, from now) is not on the path. Q1b is merged.
- **The slice chain is the path:** D0p → S2 → A1b → A1c → A2b1 → A2b2 → A2c → Q2 → A4 → A3 → `L-TGT` has 0 slack. A slice overrun of 1 run moves C1 and PT3 by 1 run. Root watches the slice board each day (14.4 risk 1).
- One-run-slack MP1 chains in stage 1: `L0-API1` → `L0-CORE` or `L0-API2` → `L0-EXIT`; `L0-WIRE` → `L0-STORE`; `L-PAY` → `L-CON1` → `L-PWR` → `UI-HUD` → `UI-ALERT` → `C1-FINAL`; `L-MATCH-SIM` → `L-MATCH-SRV1` → `L-LOBBY-C1`; `L-DES`, `UI-RESULT`. Zero slack in stage 1 outside the slice: `L-TGT`, `L-OBS`, `C1-FINAL`, `I-PIX1`, `REG2-1`, `C1-PLAYER`, `C1-PERSON`.
- Start windows (earliest start-merge; slack in runs): `Q-LOCK` 0-1/2, `L0-SPLIT` 0-2/7, `UI-PLACE` 12-22/2, `UI-HINT` 12-23/2, `UI-RESULT` 21-24/1, `L-FX-PRES` 12-17/8, `L-LOBBY-C1` 12-21/1, `L-FX-WIRE` 14-16/8, `L-STO` 20-22/2, `L-DMG` 12-14/6, `L-PWR` 14-22/1, `L-INC` 12-16/9, `UI-LABEL` 12-14/11, `C-UNITS` 11-14/7, `C-UI` 14-18/7, `WEB-NOGPU` 3-5/20, `L-DES` 21-24/1, `L-MATCH-SIM` 12-15/1, `L-MATCH-SRV1` 15-20/1. Root can start a client task late to cut its rebase work and to hold the lanes (3.2).

### 3.2 Waves and lanes

Wave = topological level. "Lanes" = the largest number of builds that run at one time inside the wave span, MP1 and slice (ASAP schedule).

| Wave | Offset | Max lanes (MP1, slice) | MP1 tasks ([slice]) |
| --- | --- | --- | --- |
| W0-W1 | 0-4 | 9 (7, 2) | `L-ROOM0`, `L0-RAW`, `L0-RULES`, `Q-PLANTS`, `PRES-PROBE`, `Q-LOCK`, `L0-SPLIT`, `L0-DBG`, `E-GPU`; [D0, A2a, D0p] |
| W2-W4 | 2-11 | 5 (4, 1) | `WEB-NOGPU`, `L0-API1`, `L0-SIM`, `L0-WIRE`, `L0-API2`, `L0-STORE`, `L0-CORE`; [S2, A1b, A1c] |
| W5 | 10-14 | 13 (12, 1) | `L0-EXIT`, `C-UNITS`; [A1d, A2b1] |
| W6 | 12-18 | 13 (12, 1) | `L0b-ORD`, `L-MATCH-SIM`, `L-PAY`, `L-DMG`, `UI-LABEL`, `C-UI`; [A2b2] |
| W7 | 12-21 | 13 (12, 1) | `L-MATCH-SRV1`, `L-INC`, `L-CON1`, `L-FX-WIRE`; [A2c] |
| W8 | 12-24 | 13 (12, 1) | `L-LOBBY-C1`, `L-PWR`, `L-DES`, `L-STO`, `L-FX-PRES`, `UI-PLACE`; [Q2] |
| W9 | 12-24 | 13 (12, 1) | `UI-HINT`, `UI-HUD`, `UI-RESULT`; [A4] |
| W10-W11 | 17-26 | 7 (6, 2) | `UI-ALERT`, `L-TGT`, `L-OBS`; [A3, REG] |
| W12 | 25-28 | 2 | `C1-FINAL`; [TDN] |
| W13 | 28-32 | 3 | `I-HEAD`, `I-PIX1`, `I-PLAY1` |
| W14-W16 | 31-37 | 2 | `C1-PLAYER1`, `REG2-1`, `C1-PLAYER`, `C1-PERSON` |
| W17-W20 | 37-45 | 12 (12, 0) | `L0-EXIT2`, `I-NC1`, `PRES-MEASURE`, `L-CON3`, `L-REP`, `L-RLY`, `L-MATCH-SIM2`, `L-LOBBY2-SRV`, `UI2-MC`, `PRES-MODELS`, `L-MATCH-SRV2`, `L-LOBBY2-UI`, `UI2-HUD`, `FX2`, `PRES-ART`, `UI2-RESULT`, `UI2-CONN`, `C2-SOUND`, `I2-L` |
| W21 | 41-44 | 7 | `C2-CONTENT` |
| W22-W26 | 46-57 | 1 to 2 | `I2-PIX`, then `MEAS-EXIT` with `REG2-2`, then `AUDIT` with `PT3-PLAYER`, `PT3-PEOPLE`, `REC` |

The peaks of 12 MP1 builds at offsets 12-13 and 38-39 come from tasks that start at their earliest offset. With the start windows of 3.1, Root can hold stage 1 at 7 to 8 MP1 lanes and the path does not move. Seven of the stage-1 peak lanes and eight of the stage-2 peak lanes use the `bevy-rendering-engineer` seat; Root starts them in the order of their slack (lowest first).

### 3.3 Dependencies read from D, and inferred

- Read from D: E1 inputs of lane 0a (D:2304); E2 inputs of lane 0b and of the order lanes (D:2305); A3 before content with large footprints (X0e, D:244, D:1419-1425); A3 in the tree of `L-OBS` (D:2523) and of `L-TGT` (D:2224-2225) (rigour H2; both as cross stacks in revision 3, ruling R-W); A1d before PAY1, PAY2, CON3, DES1, REP1, INC1 (D:2421-2422; inferred: the ledger oracle of A1d is the `turts_audit` that the rows need); the replica test before the end-tick step (D:2582-2590); point 21 before the seat release (`docs/research/mp1/recheck6-rigour.md:30`); stage 2 only after C1 passes (D:60, D:2366).
- Inferred by the producer: every other edge of the task table, from the files and the rows. The block `mp1-tree` (3.6) and the plant rule of the script check the edges that a row needs.

### 3.4 Conflict list: files that two concurrent tasks touch

The script prints 161 concurrent pairs, 13 of them ordered by a merge-after edge only (section 20; revision 2 printed 160 and 12; revision 1 printed 112 and could not see the merge-after pairs, TD H2). They fall into these groups. "Forward merge" means: the slice task merges first, and the MP1 owner resolves the conflict when Root merges the slice forward. "Own function" means: `L0-CORE` or lane 0 made a stub function or file for each writer, so each later task fills its own body and adds at most one call line.

| File | Concurrent tasks | Order or split |
| --- | --- | --- |
| `turts_mod_runtime/src/unbuilt.rs` (49 pairs) | 18 feature tasks of stages 1 and 2 | One line per feature; each line names its owner task (section 5.3). A task deletes only its own lines. The resolution is the union of deletions. The reviewer counts the lines with `rtk proxy grep -c` against section 5.3. |
| `turts_sim/src/orders/` (42) | lane 0: `L0-API1`, `L0-SIM`, `L0-WIRE`, `L0b-ORD`; stage 1: `L-CON1`, `L-STO`, `L-TGT`; stage 2: `L-CON3`, `L-REP`, `L-RLY`, `L-MATCH-SIM2`; slice A2b1, A2b2, A2c, A3 | Lane 0 merges in the order `L0-API1`, `L0-SIM`, `L0-WIRE` (inputs); feature lanes edit only their own behaviour file (stubs of `L0b-ORD`). Every order-state write goes through the one writer of `L0-WIRE`; the syn check `order_writers` makes a forward merge with a direct write red, and the MP1 owner of the forward merge (`network-engineer`) routes it (R19). Slice: forward merge. |
| `turts_sim/src/economy/combat.rs` (10) | `L0-API1`, `L-DMG`, `L-FX-WIRE`, `L-TGT`, `L-REP`; slice A2c, A3 | `L-PWR` no longer edits it (`power::weapon_enabled` stub of `L0-SIM`, TD H2 (c)). `L-TGT` takes A3 as a cross stack (section 1, ruling R-W): it merges the A3 branch at the end of the A3 build, so A3 and `L-TGT` do not build the unit frame and the cell at the same time (TD H2 (b), H-A). `L-DMG` (the modifier `:371-376`) and `L-FX-WIRE` (`:393-402`) touch other functions than `run_combat` `:82-257`; the later one rebases. Slice A2c: forward merge. |
| `turts_sim/src/economy/gather.rs` (8) | `L-FX-WIRE`, `L-STO`, `L-OBS`, `L-RLY`; slice A2b1, A2b2 | `L-FX-WIRE` and `L-STO` merge after A2b2; `L-OBS` has A2b2 as an input and A3 as a cross stack. `L-OBS` owns the approach-point and index edits. `L-STO` puts `NearestNode` in `orders/`; `L-FX-WIRE` reads the existing delivery event. If either needs `gather.rs`, it rebases on `L-OBS`. Slice: forward merge. |
| `turts_client/src/visuals.rs` `draw_world_visuals` (`:279`, 527 lines) (8) | `UI-PLACE`, `L-FX-PRES`, `UI-LABEL`; stage 2 `FX2`, `PRES-MODELS` | Own function: `L0-SPLIT` splits the function into one function for each concern, called in the present order (pure move, rulings R-S, R-X). Each task adds its own file under `visuals/` and one call line. |
| `turts_client/src/core_adapter/` (6) | `UI-PLACE`, `UI-HINT`, `UI-ALERT`, `UI-RESULT` | Own function: `L0-SPLIT` moves the Effect match of `drive_core_adapter` (`core_adapter.rs:730`, the arms at `:1045-1123`) to `core_adapter/effects.rs`, one function for each effect family (pure move, rulings R-S, R-X). |
| `turts_client/src/core_adapter.rs` (1) | `L0-SPLIT` (the Effect match, `:1044-1123`), `L0-API1` (the `SlotRelation` match at `:340`) | Different functions. `L0-SPLIT` merges first (window 0-2); `L0-API1` (4-8) rebases. |
| `turts_client/src/widgets/` (4 stage 1, 4 stage 2) | `UI-HUD`, `UI-RESULT`, `UI-ALERT`; stage 2 `UI2-HUD`, `UI2-MC`, `UI2-RESULT` | `activate_widget` (`widgets/activation.rs:43`): `L0-CORE` gives each writer its own function file (`activation/cancel.rs` `UI-HUD`, `leave.rs` `UI-RESULT`, `place.rs` `UI-PLACE`, `rally.rs` `UI2-HUD`) and one arm each. `widgets/sources.rs` (2,455 lines): one stub function for each new `Value`, `Source`, `Predicate`, `PrimaryField`, `ItemField`, in a file of its owner (section 5.3). `widgets/tree.rs` `sync_widget_tree` (`:114`) has two writers that cannot be split (the cost rule of `UI-HUD`; the alert-list rows of `UI-ALERT`, read at `:188-210`): an input edge, `UI-ALERT` stacked on `UI-HUD`. Stage 2: `UI2-HUD` has `UI2-MC` as input (both touch `spawn_widget` `:275` and `sources.rs`). |
| `turts_interaction/src/` (3) | `UI-PLACE`, `UI-HINT`, `UI-ALERT` | `L0-CORE` splits `step` (`lib.rs:143`) by input kind and makes `placement.rs`, `hints.rs`, `alerts.rs`. |
| `turts_client/src/debug.rs` (1) | `L-FX-PRES` (`turts_debug.effects.recent`), `UI-HINT` (`turts_debug.hints`) | Own field function each; the later one rebases (presentation L1). |
| `turts_client/src/client_shell/`, `feedback.rs`, `actions.rs` (stage 2) | `L-LOBBY2-UI`, `UI2-CONN`, `UI2-MC`, `L-MATCH-SRV2` (`surrender` action), `UI2-RESULT` (`stats_first_seen_tick`) | Own match arm or report field each; `UI2-CONN` merges first (it adds alert triggers that `UI2-MC` keeps). |
| `turts_client/src/feedback.rs` (stage 1) | `UI-ALERT`; the MC PT1 repair (`codex/mc-fix-pt1`, `feedback.rs` +16: one alert for each wait) | Forward merge before `UI-ALERT` starts; `UI-ALERT` keeps the PT1 one-alert-per-wait test green and adds its plant (ux M3). |
| `turts_client_web/src/lib.rs` (1) | `L-LOBBY-C1` (the save at `Seated`, `:259`), `WEB-NOGPU` (the probe before `start()` builds the app, `:144`) | Different functions; `WEB-NOGPU` merges first (earlier window); `L-LOBBY-C1` rebases. |
| `turts_server/src/session.rs` (4) | `L0-API1` (test literals), `L0-WIRE` (bucket first, `Welcome.start`), `L-MATCH-SRV1`; slice A1b | Inputs order the MP1 writers; A1b: forward merge. |
| `turts_audit/src/replay.rs` (2) | `L0-API1`; slice A1b, A1c | If A1c merges forward first, the file is gone: take the deletion. |
| `turts_audit/` (1) | `L-PAY`, `L-INC` (merge-after) | Joint review; `L-INC` rebases on `L-PAY`. |
| `economy/transaction.rs` (1) | `L0-SIM` (index insert and remove); slice A2c | Forward merge. |
| `production.rs`, `construction.rs` (0) | `L-DES` | `L-PWR` no longer edits them: the rate goes through the stub `power::rate_factor` of `L0-SIM`, called in `production.rs` by `L0-SIM` and in `construction.rs` by `L-CON1`; `L-PWR` fills the body in `power.rs` (rulings R-W, R-X, TD sequencing proposal). |
| `render.rs`, `visuals.rs`, `visuals/` (3, merge-after) | `FX2`, `PRES-MODELS` | `FX2` merges after `PRES-MODELS` (turret node); `PRES-MODELS` owns the spawn path in `sync_units` (`render.rs:356`), `FX2` the yaw, aim and bob in their own functions. |
| `mods/frontier/` (stage 2) (3) | `C2-CONTENT`, `C2-SOUND`, `PRES-ART` | Own subdirectories (`match/`, `interface/`, `guide/`; `sounds/`, `fx/`; `assets/models/`, `visuals/`). `C2-CONTENT` owns `mods/frontier/interface/` in stage 2 (ux M7). `manifest.ron` and `assets/assets.ron` are shared: one export line each; rebase. |
| `tests/browser/` (1) | `I-PIX1`, `I-PLAY1` | Own files (`mp1-goal1-*.js`; `mp1-play-*.js`); shared finders in `tests/browser/mp1-finders.js` are written by `I-PIX1` first, and `I-PLAY1` rebases. |
| `docs/measurements/mp1.md` (8) | 8 tasks | One section heading per task; append only. |
| `scripts/acceptance.sh` (not in the hot cells) | every task that adds an `#[ignore]` test | One `run_cargo_test` line each (the form of `scripts/acceptance.sh:86-88`); append only; the union on merge. |

### 3.5 Coverage and order check

The same script checks: every row part of D section 12 has exactly one owner, with a stage that agrees with the D stage lists (D:2424-2430), the splits and the moves of section 13.2; every scope item X0 to X26 of D section 4 has a task; every open point of D section 15 has a closing entry (section 11); every input exists; no cycle; the reviewer is not the builder; every stage-2 task descends from `C1-PERSON` through inputs; every pair of block `mp1-tree` is a path of inputs; each row part has a plant entry; the owner descends from each plant task; a pixel plant needs a headed run. Revision 3 adds (qa N2, rigour L4): a row part whose D "Runs" cell names "pixels" needs a `lane+px` plant entry; each plant cell holds a D plant label of the row (a label "(x)" of the D Red build cell, or three words in sequence of that cell when it has no label); the task text names the row near the word "plant" and the first label of the cell; the pairs of the script's own list `REQUIRED_TREE` (the E1, E2, A1d and A3 needs, and `C1-PLAYER1` ← `I-PLAY1`, `C1-PLAYER` ← `REG2-1`, `PT3-PLAYER` ← `REG2-2`) are paths of inputs and rows of `mp1-tree`; each cross stack has a row in `mp1-tree`; every headed MP1 task descends from `Q-LOCK`.

- `--self-test` applies 13 mutations: the six of revision 1 (a row part unowned, a row part owned twice, a cycle, X0e dropped, open point 21 dropped, a stage-2 owner marked stage 1), the three of qa M1 (`I-PIX1` without `C1-FINAL`; the `L-STO` plant entries deleted; `I-HEAD` with only `L0-EXIT` as input), the three of rigour (m5: the reviewer equals the builder; m4: a stage-2 task before C1; m9: A3 a merge-after only of `L-OBS`), and one for R-A (a pixel plant in a task with no headed run). Revision 3 adds seven, so `--self-test` applies 20: qa m4 (the plant text deleted from the `L-STO` step), m5 (the GHO1 plant `lane` in place of `lane+px`), m6 (the HUD5 plant cell "none"); rigour n1 (the `L-TGT` tree row deleted and A3 a merge-after of `L-TGT`), n2 (the same escape as m5 on a mixed row: the FX1/s1 pixel plant of `L-FX-PRES` set to `lane`; the n2 text equals m5, so the producer moved it to a second row), n3 (`I-PLAY1` removed from the inputs of `C1-PLAYER1`); and one for R-T (`Q-LOCK` removed from the inputs of `L0-DBG`). Each gives RED; the unmutated plan gives GREEN (section 20).

### 3.6 Plant set, build-tree needs and row moves

The plants are the red builds of D section 12 at the D line of the row, and the plan plants in quotes. "Red run": `lane` means the plant runs red in the lane before merge; `lane+px` means it is a pixel plant and runs red through the headed wrapper in the lane (R-A). The records are in `docs/research/mp1/plants/<task>.json`.

<!-- mp1-plants:start -->
| Row part | Task | Plants (D labels, or plan labels in quotes) | Red run |
| --- | --- | --- | --- |
| DBG1 | L0-DBG | (a) trace pruned to 0; (b) build on every frame | lane |
| VAL1 | L0-RAW | VAL1 (2) part: parsed value for `orders`; t2c row removed | lane |
| VAL1 | L0-API2 | VAL1 (1) to (6) (D:2437) | lane |
| IMM1 | L0-SIM | IMM1: `muster` has 0 ticks with the gate and no `Immediate` insert in `headless_app` (red build 1) | lane |
| IMM1 | L-MATCH-SRV1 | IMM1 (D:2474): default policy a lobby; `--start-now` ignored; a version-table row missing | lane |
| MAT1/s1 | L-MATCH-SIM | (b) draw gives `Victory` to the lower slot; (i) logic: defeat with no clause | lane |
| MAT1/s1 | L-MATCH-SRV1 | (b) draw rule; (a) tick log has a tick after T | lane |
| LOB1/s1 | L-LOBBY-C1 | run 0: sim ticks before the second seat; start through `SnapshotDue`; run 4: seat bound to the player id | lane+px |
| LEAVE1/s1 | L-LOBBY-C1 | the 8 plants of D:2475: the server stays in Ended (no exit), a world reset in place, the script restarts after a non-zero exit, and the five others | lane |
| PAY1 | L-PAY | PAY1 (1) to (6) ((6): `held` follows the per-tick state, E21); "audit recomputes the schedule" | lane |
| PAY2 | L-PAY | PAY2 (1), (2) | lane |
| INC1 | L-INC | INC1 (a) to (c) (D:2465) | lane |
| DMG1 | L0-RULES | DMG1 (1) float multiplier: 29/100 gives 28 | lane |
| DMG1 | L-DMG | DMG1 (1) f64 and f32, (2) axes swapped, (3) `flat` before the ratio | lane |
| TGT1 | L-TGT | TGT1 (1) to (4) | lane |
| TGT2 | L-TGT | TGT2 (1) to (4) | lane |
| CON1 | L-CON1 | CON1 (1) to (6) | lane |
| PRD1 | L-CON1 | PRD1 (a) to (d) | lane |
| PWR1 | L-PWR | PWR1 (1) to (4) | lane |
| DES1 | L-DES | DES1 (1) to (6); "direct despawn outside the death path" | lane |
| OBS1 | L-OBS | OBS1 (1) to (3) | lane |
| FX1/s1 | L-FX-WIRE | FX1 (a) headless; (d) kept deltas replay effects | lane |
| FX1/s1 | L-FX-PRES | FX1 (b) pool; flash and death pixel plants; "offline mark missing" | lane+px |
| CON2 | UI-PLACE | CON2 (1) to (3) | lane |
| GHO1 | UI-PLACE | GHO1 (a) to (e) | lane+px |
| HUD5 | UI-HUD | HUD5 (a) to (c); "`current.0 = text` back" (changed counter) | lane+px |
| HUD6/s1 | UI-HUD | HUD6 (b), (d) | lane+px |
| KEY1 | L0-API2 | KEY1 (b), (c) packer refusals | lane |
| KEY1 | UI-HUD | KEY1 (a) | lane+px |
| ALR2/s1 | UI-ALERT | ALR2 (a) to (i) without the connection alerts; "two alerts for one wait" (PT1 repair) | lane+px |
| MCX1/s1 | UI-ALERT | each MC defect put back, alert-list part: the row pitch off by the gap; the first row clipped | lane+px |
| RES1/s1 | UI-RESULT | result screen without the reason; outcome shown before `end_tick` | lane+px |
| NOGPU1 | WEB-NOGPU | no probe: a panic with a console message only (the MC build 8d6902c) | lane+px |
| ORDW1 | L0-WIRE | ORDW1 (1) to (4); "single `MoveTo` change dropped"; "direct write of `UnitOrders`" | lane |
| ORDW1 | I-HEAD | ORDW1 (5); case C bytes above 8 B | lane |
| STO1/s1 | L-STO | STO1 (a) to (e) | lane |
| GOAL1 | UI-HINT | GOAL1 (d), (h) headless; "X3 never shows on S8" | lane |
| GOAL1 | UI-LABEL | GOAL1 (i) labels off: the label oracle is red | lane+px |
| GOAL1 | I-PIX1 | GOAL1 (a), (b), (c), (e), (f), (g) | lane+px |
| REG2/s1 | REG2-1 | camera plant (`camera.rs:358`); SEL1 projection shift; MP1 CON1 (1) through `mp1_plants.py --all` | lane+px |
| NC1 | I-NC1 | one literal for each new kind | lane |
| CON3 | L-CON3 | CON3 (1) to (3) (D:2442) | lane |
| REP1 | L-REP | REP1 (1) to (3) (D:2445); "direct health write" | lane |
| RLY1 | L-RLY | RLY1 (1) to (4) (D:2446) | lane |
| STO1/s2 | L-RLY | STO1 (f) | lane |
| MAT1/s2 | L-MATCH-SRV2 | MAT1 (c) to (k) plants of D:2447 | lane |
| LEAVE1/s2 | L-LOBBY2-SRV | a world reset in place: the same `match_id` and the old lobby seats after the restart (the lobby-seat form, D:2475) | lane |
| LOB1/s2 | L-LOBBY2-UI | a start with one ready (run 1); `unready` does not stop the countdown (run 2); the revision check off (run 2) (D:2448) | lane+px |
| LOB2 | L-LOBBY2-UI | the client takes the slot colour from the palette by slot index (pixels); the server does not check the colour (D:2472) | lane+px |
| RES1/s2 | UI2-RESULT | the sim view gives the statistics at each tick; one statistic in a new field of `MatchState`; a build that counts the free income in `Gathered` (D:2449) | lane+px |
| SUR1 | UI2-CONN | surrender with no confirmation; the other window shows no result; the outcome shows before the server end tick (D:2473) | lane+px |
| ALR2/s2 | UI2-CONN | ALR2 (j) `SlotDisconnected` not wired: a killed client gives no lost-connection line (rigour M2) | lane+px |
| MCX1/s2 | UI2-MC | each MC defect put back, without the alert-list part (D:2457) | lane+px |
| HUD6/s2 | UI2-HUD | HUD6 (a), (c) (D:2469) | lane+px |
| SMT1 | UI2-HUD | SMT1 (a) to (c) (D:2470) | lane+px |
| FX1/s2 | FX2 | FX1 (c) `hit_reduced` never chosen; "turret does not turn" (D 8.2) | lane+px |
| AUD2 | C2-SOUND | AUD2 (a), (b), (d) in the browser; (c) a packer refusal (D:2471) | lane+px |
| MDL1 | PRES-MODELS | MDL1 (b) for each check of 8.1 (pivot, scale, materials, nodes, bytes, images), a plant that packs with exit 0 | lane |
| MDL1 | PRES-ART | MDL1 (a) a URL fetch of the model (`.glb` request); (c) the shape removed at spawn before the model is ready; (d) the team material not replaced | lane+px |
| VIS3 | PRES-MEASURE | VIS3 (1) `frame_stall_ms=60` on every frame; (2) on 1 frame in 10 | lane+px |
| VIS3 | MEAS-EXIT | VIS3 (1) and (2) on the play-test machine | lane+px |
| REG2/s2 | REG2-2 | `clamp_to_map` returns at once (the camera plant); SEL1 shift; MP1 set through `mp1_plants.py --all` | lane+px |
| PT3 | I2-PIX | PT3 timing plants (1) to (3) (D:2459) | lane |
| PT3 | PT3-PEOPLE | PT3 (1) to (3) through the timing script on the capture of the people; the red condition of D:2459 (a stop that a tester says aloud) | lane |
<!-- mp1-plants:end -->

<!-- mp1-splits:start -->
| Row | Parts | Reason |
| --- | --- | --- |
| STO1 | STO1/s1, STO1/s2 | run 4 and plant (f) need a rally (X9, stage 2; D:2462); D 6.1 lists STO1 in both stage lists (E5, E20) |
| ALR2 | ALR2/s1, ALR2/s2 | the lost-connection and return alerts need `SlotConnection` (X11c, stage 2) (Q2; erratum E5) |
| MCX1 | MCX1/s1, MCX1/s2 | the alert-list row pitch and clip part shows with the first alert at about 40 s (ux M5, ruling R-N): `UI-ALERT`; the rest stays in `UI2-MC` (erratum E7) |
<!-- mp1-splits:end -->

<!-- mp1-moves:start -->
| Row part | D stage | Plan stage | Erratum |
| --- | --- | --- | --- |
| NOGPU1 | 1 | 1 | E8, applied in D 6.1: the C1 start can fail with no adapter (ux H2, ruling R-D) |
| NC1 | 2 | 2 | E9, applied in D 6.1: a source scan; C1 loses nothing (TD stage note, ruling R-Q) |
<!-- mp1-moves:end -->

<!-- mp1-tree:start -->
| Task | Needs in its build tree (through Inputs) | Reason |
| --- | --- | --- |
| L0-API1 | S2, D0p | struct literal sites after S2 (E1, D:2304) |
| L0-SIM | S2, D0p | E1 (D:2304) |
| L0-WIRE | S2, D0p | E1; the order writers in `orders/` |
| L0b-ORD | A1c | E2 (D:2305) |
| L-PAY | A1d | the ledger oracle (D:2421-2422) |
| L-INC | A1d | the ledger oracle (D:2421-2422) |
| L-TGT | A3 | the per-tick frame and the cell (D:2224-2225; TD H2); a cross stack `A3^` (ruling R-W) |
| L-OBS | A3 | the perf probe on the tree with A3 (D:2523; rigour H2); a cross stack `A3^` (ruling R-W) |
| C1-FINAL | A3 | C1 condition (D:2365, X0e) |
| C1-PLAYER1 | I-PLAY1 | it judges the evidence set of the `I-PLAY1` green run (R-D; rigour L4 n3) |
| C1-PLAYER | REG2-1 | it judges the evidence set that `REG2-1` writes on the C1 commit (R-D) |
| PT3-PLAYER | REG2-2 | it judges the evidence set that `REG2-2` writes (R-D) |
<!-- mp1-tree:end -->

## 4. Gate sets

Fast gate for one task, in the lane, after the commit. SP section 4 defines F-RUST, F-WASM, F-QA, F-FROZEN, F-MOVE and F-REF; use `mp1/integration` in place of `pre-mp1/integration`. MP1 adds or changes:

- **F-FROZEN** for every task that changes `turts_sim`, `turts_pack` or `turts_server` (qa L1), also where the task block does not repeat it.
- **F-REF (MP1 form, ruling R-B):** the SP selector on the changed files, then `python3 -I scripts/qa/mp1_plants.py --files <changed paths> --native-only; echo exit=$?`: every native MP1 plant whose file is in the changed set runs red, restores, and runs green. The pixel plants of the task run in F-HEAD. Until `Q-PLANTS` merges, F-REF is the SP form only.
- **F-PACK** (tasks that touch the mod API or a package): `for p in mods/*/; do cargo run -q -p turts_pack -- "$p"; echo "$p exit=$?"; done`. Each repository package prints `exit=0`; `frontier` prints only the refusal named in its task.
- **F-PROC** (server tasks): `cargo test -p turts_server --release --locked --test <file> -- --ignored --test-threads=1; echo exit=$?` for each process test that the task names, and the same test through `bash scripts/acceptance.sh` after the task adds its line.
- **F-DRV** (tasks that add `tests/browser/*.js`): `python3 scripts/qa/driver_rule.py --self-test; echo exit=$?`, then `python3 scripts/qa/driver_rule.py tests/browser; echo exit=$?`, then `node --test tests/browser/<row>-oracle.test.js; echo exit=$?`. Each prints `exit=0`.
- **F-HEAD** (tasks with a pixel row, rulings R-A, R-U, R-T): the wasm builds first (green and one for each pixel plant), then one wrapper call in mode `pre-merge` (at most 20 minutes): the row green, every pixel plant of the task red with its output written to its record, each restored (`cmp` of the file with its committed form), and the row green again. The receipt (wait time, hold time) goes into the task record.
- **F-PLAN** (any edit of this plan): the script of section 3.1 and its `--self-test`, each `exit=0`.

Slow gate for one merged tree (Root, on `mp1/integration`): SP section 4 "Slow gate", items 1 to 7 and 9, one time for each merged tree that differs from a tree that passed it (C391 item 3), and `python3 -I scripts/qa/mp1_plants.py --all --native-only` from `Q-PLANTS` on (rule 3 of the lock queue: pixel plants run only in REG2). Add the MP1 process tests that exist on the tree (F-PROC for every MP1 test file). Headed batches B1 and B2 run inside `I-PIX1` and `I2-PIX`, and REG2 runs inside `REG2-1` and `REG2-2`, each as table runs (wrapper mode `table`, one call for each row, rule 2).

## 5. Lane 0: the contract

Lane 0 is the contract for every later lane. It has 18 tasks (stage `0` in the table): ten that start now or are done (`L-ROOM0`, `L0-RAW`, `L0-RULES`, `L0-DBG`, `C0-PACE` (done), `E-GPU`, `Q-PLANTS`, `PRES-PROBE`, `Q-LOCK`, `L0-SPLIT`), six after E1 (`L0-API1`, `L0-API2`, `L0-SIM`, `L0-WIRE`, `L0-STORE`, `L0-CORE`), the freeze review `L0-EXIT`, and lane 0b (`L0b-ORD`, after E2). The second freeze `L0-EXIT2` is a stage-2 task (after C1).

### 5.1 What is frozen at `L0-EXIT` (the stage-1 shapes) and at `L0-EXIT2` (the stage-2 shapes)

Lane 0 builds every type and record of MP1. `L0-EXIT` freezes the stage-1 shapes. The stage-2 shapes exist from lane 0 on, but a C1 finding can change them with no change request until `L0-EXIT2` (ruling R-J). Protocol 5 and API 0.2.4 are not released before MP1 merges, so this needs no second step (X19).

1. The mod API 0.2.4 (`L0-EXIT`): every type, field, default and enum variant of D 6.10 that a stage-1 row uses (row H24 in the form of erratum E22: `LabelStyle`, `LabelShow`), the edition rows T1 to T4 as names, `REASON_KEYS` (49 keys, D:1603-1615), `CURSOR_CONTEXTS` (9, D:1577), `STATE_KEYS` (22, D:1616-1625), `ENGINE_API_VERSION = "0.2.4"` and `RELEASED_ENGINE_APIS` with `"0.2.4"` (D:2280-2283), every version-table row with its detection kind, the `UNBUILT_FEATURES` lines and their text (D:1390), and the new row H24 `world.unit_labels` (errata E10, E22).
2. Protocol 5 (`L0-EXIT`): `NET_PROTOCOL_VERSION = 5`, every record of D 6.8 (D:1298-1321) and D 6.7.13 (D:1259-1272) that a stage-1 row uses, with its fields and channel (the job records and `SiteProgress` with `held: bool` and `held_since_tick: u32`, erratum E21, ruling R-Y), the registration order of the five new messages after `ResyncRequest` (D:1336-1339), the nine new `CommandRejectReason` keys.
3. `turts_rules` (`L0-EXIT`): the public signatures of `placement::check`, `relation`, the damage formula, the charge, refund and heal difference functions, `progress` (the one "value at tick" function for rate records, `Paid` and `Refund`; ruling R-K), the site-health target, the shortage effect.
4. Sim seams (`L0-EXIT`): the new `CommittedTick` fields, the sim `MatchConfig` type and its gate, the empty system slots and their schedule order, the `UnitIndex` API, `power::weapon_enabled` and `power::rate_factor`, the order writer function of R19 and its `test-probes` check, the job fields `held` and `held_since_tick` and the engine constant of 1,000 ms (E21).
5. Client seams (`L0-EXIT`): the `Core::step` split by input kind, the split `draw_world_visuals` and `core_adapter/effects.rs` (`L0-SPLIT`, R-S, R-X), the dispatch stubs for every new `Value`, `Source`, `Predicate`, `PrimaryField` and `ItemField` variant, the `WidgetAction` variants `Place { constructor, content }` and `FocusAlert`, the store getters of `L0-STORE` (one for each `Value`).
6. Lane 0b, frozen at its own merge: the behaviour handler table, the stub file of each new behaviour, `OrderSource::Engine`.
7. The stage-2 shapes (`L0-EXIT2`): `LobbyAction`, `LobbyRefused`, the `set_colour` and ready fields, `SetRally`, `UnitOrderRecord.rally`, `CancelConstruction`, `Repair` and `repairer`, `Surrender`, `DisconnectedFor`, `SlotConnection`, `MatchResult.stats`, the models V1 and V2, H8, the stage-2 parts of H9, H11 and H12, H13, H16, and the stage-2 sound fields (`gate2-technical-director.md:16`).

Not frozen: writers, systems and handlers (the feature lanes fill them), validation messages of `L0-API2` until it merges, content numbers (S:807).

### 5.2 How a later lane asks for a change

1. The lane stops the step and writes a change request on the board: `CR-<n>`, the item of 5.1, the reason, the D line, and the tasks that the conflict list (3.4) names for that file.
2. `technical-director` rules on it in one run (Opus). The CEO decides when the request touches a decision of D section 2.
3. The owner seat of the item makes the change in one commit on `mp1/integration`: `mod-runtime-engineer` for 5.1 item 1, `network-engineer` for items 2 and the store of item 5, `simulation-core-engineer` for items 3, 4 and 6, `bevy-rendering-engineer` for the rest of item 5. The reviewer of the requesting lane reviews that commit (one run; rigour, lane-0 freeze gap).
4. A change request never bumps the protocol or the API again: MP1 has one step of each (X19, D:268). The change re-runs VAL1 (item 1) or the ORDW1 cases of `L0-WIRE` and the store contract test of `L0-STORE` (item 2), and every open lane rebases.
5. Budget: 0 to 3 change requests, each 2 runs (ruling and review), counted under gates (section 14.1).

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
| `world.unit_labels` (H24, errata E10, E22) | `UI-LABEL` |
| `role` (H19), `hotkey` (O8), `card` (O10), `Value::Supply`, `ConstructionQueue`, `Sites`, `SelectionTypes`, `Notices`, `Predicate::SupplyShort`, `ItemHas`, item fields `Role`, `State`, `Paid`, `Refund`, `Badge` | `UI-HUD` |
| `Value::Outcome`, `OutcomeReason`, `MatchClock`, `Predicate::MatchOver`, `OutcomeIs`, `OutcomeReasonIs`, `{slot}` (H17, T4), `leave_match` (H23) | `UI-RESULT` |
| `Value::MatchStats` (H7) | `UI2-RESULT` |
| item fields `BuildTimeNow`, `SupplyChange`, `CausesShortage`, primary fields `Activity`, `Cargo`, `Armour` (H8, H9), `PrimaryHasValue` (H10), `world.cargo_bar`, `world.health_bar.scale_to_footprint` (H12), `world.rally` (H11), cursor contexts `repair`, `rally` (H2) | `UI2-HUD` |
| `exports.visual_effects` (A3), visual `effects` with `fire`, `death` (V3, V4) | `L-FX-PRES` |
| `hit`, `hit_reduced`, `damaged` (V4), effect `sound` (V3), `world.float_text` sound (H11), `effects_voices`, `audible_range` (H16) | `FX2` |
| `Button.font`, `Button.size` (H13) | `UI2-MC` |
| `models` (V1), `model` (V2) | `PRES-MODELS` |

`L0-EXIT` checks with a script that every row name of 6.10 is either built in lane 0 or has exactly one line here (step 2 of `L0-EXIT`).

## 6. Lane 0 tasks

### Task L-ROOM0: room module and the connection-end trace (now)

**Seat:** `backend-engineer` (S). **Review:** `network-engineer` (O): does each cell of the state table hold, and does the trace show the real end path?

**Branch / inputs:** `mp1/l-room0` from `pre-mp1/integration` at `03aef38`; no input. **Design:** D 6.7.2, 6.7.3, 6.7.5 (table D:982-988), 6.7.6, section 15 point 21 (D:2615-2619), section 14 row "The server detects a disconnect late" (D:2528).

**Files:**
- Create `crates/turts_server/src/room.rs`: pure types and functions, no Bevy system. `RoomState { Boot, Lobby, Countdown, Running, Ended }`; `SeatTable` (seat count = player-slot count, `crates/turts_sim/src/slots.rs:7`; lowest open seat; lowest free palette index; label `Player N`; release); `judge_join(state, id, has_live_connection)` for each cell of D:982-988; the rule "a second `Hello` on a connection with a seat is ignored" (D:957). Add `mod room;` in `crates/turts_server/src/main.rs`.
- Create `crates/turts_server/tests/connection_end.rs` (`#[ignore]` process test) and its `run_cargo_test` line in `scripts/acceptance.sh`.

- [ ] **Step 1: Trace (open point 21).** Find the code that ends a `ClientOf` at the netcode timeout and at `Unlink`. Lead (verified at `03aef38`): today the only handler of a session end is the observer `on_session_removed` on `On<Remove<Session>>` (`crates/turts_server/src/session_diagnostics.rs:383-384`), registered only with `--stats` (`crates/turts_server/src/stats.rs:104-105`); the refused-connection path triggers `Unlink` (`session.rs:710-719`). Which lightyear schedule despawns the entity is NOT TRACED. Write the process test: release `turts_server` with `muster` and `--stats`, one headless client with `--player-id`, then a SIGKILL of the client, then a clean exit in a second run. Read the private observer file for `session_authority_removed` and its time after the kill. Expected: one record within 11 s for each case. Record L for a release build in `docs/measurements/mp1.md` (section "Disconnect notice L, native release"). Write the trace note in the task report: the hook that `L-MATCH-SRV1` uses for the seat release, with `path:line`.
- [ ] **Step 2: Red.** Unit tests of `room.rs` for each table cell, the lowest-seat and lowest-colour rules, the release and the second `Hello`, with `todo!()` bodies. `cargo test -p turts_server --locked --bin turts_server room::; echo exit=$?` gives `exit=101`.
- [ ] **Step 3: Green.** Fill the bodies. Same command gives `exit=0`.
- [ ] **Step 4: Gate.** F-RUST (`turts_server`), F-FROZEN, F-PROC (`connection_end`).
- [ ] **Step 5: Plants.** (a) Remove `app.add_observer(session_diagnostics::on_session_removed)` (`stats.rs:105`): the trace test is red ("no removal record"). (b) The colour rule by seat index: the lowest-free-colour test is red. Restore each. Write `docs/research/mp1/plants/L-ROOM0.json`.

**Exit:** the trace note names the end hook; L (release) is recorded; the room tests are green. **Rows:** none. **Runs: 3 (2-4).**

### Task L0-RAW: RAW presence for the existing version-table rows (now)

**Seat:** `mod-runtime-engineer` (S). **Review:** `mod-builder` (O): does an explicit empty field now get the version refusal?

**Branch / inputs:** `mp1/l0-raw`; none. **Design:** D 6.10 "Version-table detection" (D:1469-1493); VAL1 plant (2) (D:2437); D:2284-2286.

**Files:** `crates/turts_mod_runtime/src/version_table.rs` (rows from `:52`; the refusal from `:96`); `crates/turts_mod_runtime/src/content_mc_validation.rs:65` (`orders_present: !orders.is_empty() || !order_rules.is_empty()`, the parsed-value presence); create `crates/turts_mod_runtime/src/raw_presence.rs` and `crates/turts_mod_runtime/tests/raw_presence.rs`.

- [ ] **Step 1: Red.** A `^0.2.2` package that writes `orders: []` packs with exit 0 today (mod-builder gate probe, D:1480). Write the test that expects the refusal of its row. `cargo test -p turts_mod_runtime --locked --test raw_presence; echo exit=$?` gives `exit=101`.
- [ ] **Step 2: Green.** Parse each file a second time as `ron::Value` (`ron` is `=0.12.2`, rigour run) and walk its keys for every RAW row of 0.2.2 and 0.2.3. Variant rows keep VAR or COMBO detection (`ron::Value` drops variant names, D:1474).
- [ ] **Step 3: Gate.** F-RUST (`turts_mod_runtime` → `turts_pack`, `turts_server`, `turts_checks`), F-QA, F-PACK (every repository package requires `^0.2.3`, rigour run), F-FROZEN.
- [ ] **Step 4: Plants.** (a) Back to the parsed value for the `orders` row: the explicit-empty test is red. (b) In `crates/turts_mod_runtime/tests/t2c_package_rejects.rs`, remove one row from the table: the t2c test of that row is red (D:2285-2286). Restore each. Plant records.

**Exit:** explicit-empty packages are refused for every existing RAW row. `ENGINE_API_VERSION` stays `"0.2.3"` (`crates/turts_mod_api/src/lib.rs:52`); the bump is in `L0-API1`. **Rows:** VAL1 plant (2) in part (VAL1 is owned by `L0-API2`). **Runs: 2 (1-2).**

### Task L0-RULES: the crate `turts_rules` (now)

**Seat:** `simulation-core-engineer` (S). **Review:** `technical-director` (O): pure, no workspace dependency, one copy of each rule?

**Branch / inputs:** `mp1/l0-rules`; none. **Design:** D section 9 crate table (D:2194), "One copy of each rule" (D:2206-2210); 6.1 check order (D:467-483) and the pure function (D:491-496); 6.6 relation (D:757-767); 6.3 formula (D:551-556); 6.4 charge (D:626-632) and refunds (D:648-656); 6.5 heal (D:691-694); 6.2 shortage (D:523-528); row K2 `site_health` (D:1531); the rate records (D:1348-1351).

**Files:** create `crates/turts_rules/` (`Cargo.toml`, `src/lib.rs`, `placement.rs`, `relation.rs`, `damage.rs`, `charge.rs`, `heal.rs`, `supply.rs`, `progress.rs`, `tests/`); workspace `Cargo.toml` members; `scripts/boundary.sh` rows for host and wasm32 (forbid every `bevy*` crate, every workspace crate, `wgpu`, `winit`) with fixture cases.

- [ ] **Step 1: Red.** Tests first: the check order (two failing causes give the first); squared distances only (a source test: no `sqrt`, `mul_add`, `sin`, `cos`, `atan` in `placement.rs`); the relation table `Same`, `Ally`, `Enemy`, `Other`, `Any` with two slots in each team; damage cells 100 x 29/100 = 29, 50 x 106/100 = 53, flat 5 on 100 x 29/100 = 24 (DMG1 plants (1), (3)); the charge sum equals the cost for a seeded set of costs up to `u64::MAX` and five rates; refund `floor(charged x n / d)`; the heal difference; the site-health target at progress 0 and at completion; `progress::value_at(rate_record, tick)` (ruling R-K, TD M3): the value at the tick of the record equals the record value, it never passes the end value, and `Paid` and `Refund` from it equal the charge and refund functions for a seeded set.
- [ ] **Step 2: Green.**
- [ ] **Step 3: Gate.** F-RUST; `cargo check -p turts_rules --target wasm32-unknown-unknown --locked; echo exit=$?`; `bash scripts/boundary.sh; echo exit=$?`; each new fixture row red with the forbid message and `exit=1`.
- [ ] **Step 4: Plants.** (a) A float multiplier in `damage.rs`: the 29/100 cell gives 28. (b) `relation` that returns `Enemy` for `Other`: the two-slot team test is red. (c) `progress::value_at` without the clamp: the "never passes the end" test is red. Restore each. Plant records.

**Exit:** the crate builds on host and wasm32; the boundary rows are red on their fixtures. **Rows:** none (DMG1, TGT1, CON2, HUD6 use it). **Runs: 2 (2-3).**

### Task L0-DBG: the debug state on request (now)

**Seat:** `bevy-rendering-engineer` (S). **Review:** `performance-architect` (O): can the counter go red on today's tree?

**Branch / inputs:** `mp1/l0-dbg`; none. **Design:** D section 9 "The client debug state" (D:2243-2259); row DBG1 (D:2454); D 8.5 (D:2178-2181).

**Files:** `crates/turts_client/src/lib.rs:147` (`app.add_systems(Last, (rebuild_debug_state, exit_when_done).chain())`); `crates/turts_client/src/debug.rs:441` (`rebuild_debug_state`); `crates/turts_client/src/replica.rs:1182` (`health_trace`); `crates/turts_client_web/src/lib.rs:166`, `:371-372` (`publish_debug`). S45 touched none of these files (section 20).

- [ ] **Step 1: Red.** DBG1 on today's tree, which is its red build: a headless client at 400 units (`turts_server --synthetic-units 400`) for 600 frames; the build counter shows 600; the test expects 0. A 10-minute process test in combat: the trace count is above 64. Both `exit=101`.
- [ ] **Step 2: Green.** Build the debug state only on a harness request, at most 4 times each second, for the last presented frame; the web bridge copies it only on a change; keep the last 64 entries of `health_trace` for each unit. The request form (ruling R-V, qa N3): in the browser, the harness asks through the page bridge; the page itself posts nothing to the server or to any other address. A person's Chrome is read the same way: `scripts/mp1-play.sh` starts Chrome with `--remote-debugging-port=9222` (the port listens on localhost by default and needs the non-default `--user-data-dir` that the sheet already gives; both NOT TRACED for the CEO's Chrome version, the builder checks them), and the capture tool `tests/browser/mp1-capture.js` (role `instrument`; `playwright-core` `connectOverCDP`, the dependency of `tests/browser/package.json`) reads `turts_debug.hints`, `turts_debug.audio` and `turts_debug.effects.recent` of each window through that port. The native client keeps `--out`. The builder states the bridge getter in the task report and adds one test: a capture through the port of a headed window returns the three records.
- [ ] **Step 3: Gate.** F-RUST, F-WASM. Headed (1 run): the MC browser rows that read `turts_debug` (SP slow set item 6, `--set ci-browser`) stay green.
- [ ] **Step 4: Plants.** (a) Prune the trace to 0 entries: the MC test of `OwnUnitDestroyed` that reads the last entry (`crates/turts_client/src/feedback.rs:643-647`) is red. (b) Build on every frame again: the counter test is red. Restore each. Plant records.

**Exit:** DBG1 green with its recorded red. **Rows:** DBG1. **Runs: 2 (1-2). Headed: 1.**

### Task C0-PACE: pacing scripts and open point 19 (DONE)

**Seat:** `game-designer` (H). **Review:** `player` (O). **Status: done** (1 run spent; ruling R-R). **Output:** `docs/research/mp1/c0-pace-stage1-pacing.md`; scripts and outputs in `docs/research/mp1/probes/c0-pace/`.

**Result.** On the stage-1 hint order, a player who follows hint F at once holds 7.2 s (3 Riflemen) or 14.5 s (a Tank) with start credits 1,200. With start credits 1,400 the holds are 0 s and 0.7 s; in 600 varied runs 1.3 % hold above 4 s, worst 7.7 s. The option "H4 starts after the Power Station is complete" is worse; to gate hint F leaves the hint line blank. Ruling R-R: start credits 1,400 is the working content number (content numbers are Open, S:807; a proposal value, not a CEO decision). The D text changes are erratum E13. The residual question (how `JobOnHold(10)` counts a hold that flickers) is open point 19 residual (section 11) and design question Q11.

**Rows:** none. **Runs: 0 from now.**

### Task E-GPU: two headed clients on one GPU (now)

**Seat:** `qa-lead` (S). **Review:** `performance-architect` (O).

**Branch / inputs:** `mp1/e-gpu`; none. **Design:** section 14 rows "One GPU for a two-client match" (D:2526) and "hosted runner" (D:2525).

- [ ] **Step 1.** Through the headed wrapper, run two headed clients of the MC build for 10 minutes with combat. Verify first: which MC package gives two armed slots (lead: `forge`, turret against drones, `vault/M3 Playtest Guide.md`; NOT TRACED). Record frame p50 and p95 of each window and the wait for the GPU lock.
- [ ] **Step 2.** The hosted probe (qa M4): run MC row MM1 (two windows, `tests/browser/mm1-driver.js`) on the hosted runner one time; record the result and the runner image.
- [ ] **Step 3.** Write the numbers, the machine and the GPU in `docs/measurements/mp1.md` (section "Two headed clients on one GPU, MC build"). No threshold (S 15).
- [ ] **Fail condition and decision rule (qa M4).** FAIL if a window shows under 1 frame each second for 5 s, or a client disconnects. On a FAIL, Root changes the form of the two-window headed runs (for example, one headed and one headless client for each pixel row, and two headed windows only for GOAL1 and the C1 evidence run) before `I-PIX1` starts, and records the change on the board.

**Exit:** the numbers and the verdict exist before the first stage-1 pixel row runs. **Rows:** none. **Runs: 1 (1-2). Headed: 1.**

### Task Q-PLANTS: plant records, runner and the `mp1` set (now; ruling R-B)

**Seat:** `qa-lead` (S). **Review:** `performance-architect` (O): can each record go red, and does `--files` select only the plants of the changed files?

**Branch / inputs:** `mp1/q-plants`; none. **Design:** D section 12 (the red builds), REG2 (D:2460); qa H2.

**Files:** create `docs/research/mp1/plants/README.md` (the record format of section 1); create `scripts/qa/mp1_plants.py` (Python 3 standard library only); create `tests/mp1/refutations.py` (the harness module that the parallel runner loads: `ROOT`, the case list from the records); in the runner `/home/julien/Projects/TuRTS/target/mc-evidence/root-regate/parallel_refutations.py` (outside the repository; its `SETS` table is at `:36-40`), add `"mp1": ("mp1", "all")`, and record its new sha256 in the task report.

- [ ] **Step 1: Red.** `python3 -I scripts/qa/mp1_plants.py --self-test; echo exit=$?` with `todo` bodies: `exit=1`. The self-test cases: a record with no `expected_red` is refused; a record whose patch does not apply is refused; a plant that stays green is reported RED; a plant that leaves the file changed after restore is reported RED; `--files` with a file of no record selects nothing; a record that edits two files applies and restores both; `--native-only` skips a `headed: true` record.
- [ ] **Step 2: Green.** `--files <paths>` runs every record with a `files[]` entry in the set; `--all` runs every record; `--native-only` skips the records with `headed: true` (the slow gate and F-REF use it; pixel records run in F-HEAD and in REG2, rule 3 of the lock queue); a record runs with its `env`; each run: apply, run `command`, compare the exit and the `expected_red` text, restore with `git checkout -- <file>`, `git status --porcelain -- <file>` empty, run green. It reads the records that the lane-0 tasks wrote before it merged, and checks their fields.
- [ ] **Step 3: Gate.** F-QA; `python3 -I scripts/qa/mp1_plants.py --all; echo exit=$?` on the records that exist (`exit=0`); the runner `--set mp1 --help` names the set.
- [ ] **Step 4: Plant.** Make the runner compare the exit only, not the text: a self-test case with the right exit and a wrong text stays GREEN, so the self-test is red. Restore.

**Exit:** F-REF of MP1 can run the plants of the changed files. **Rows:** none. **Runs: 2 (2-3).**

### Task PRES-PROBE: the two presentation risk probes (now; ruling R-M)

**Seat:** `bevy-rendering-engineer` (S). **Review:** `performance-architect` (O).

**Branch / inputs:** `mp1/pres-probe`; none. **Design:** D 8.1, D 9 client cost rule (D:2262-2276); presentation "Risks and early probes".

**Files:** create `docs/research/mp1/probes/pres/` only (no crate change). The GLB script of the gate probe is `docs/research/mp1/probes/gate2-glb/make_tank.py` (and `make_tank_bad.py`, `glb_probe_main.rs`, `glb_probe_Cargo.toml`).

- [ ] **Probe 1 (`PRES-MODELS` risk).** A headless app with `AssetPlugin`, a memory asset source registered before `DefaultPlugins` (`bevy_asset-0.20.0/src/lib.rs:576-578`), and `GltfPlugin` loads `tank.glb` from bytes. Print the `named_nodes` and the `Material0/std` labelled asset handle. Record the result and the Bevy 0.20 names (`WorldAssetRoot`, not `SceneRoot`).
- [ ] **Probe 2 (`UI-HUD` risk).** The H2 counter test on today's tree: a headless client with a static replica for 60 frames counts widget entities with `Ref<Text|Node|Visibility>::is_changed()` after frame 2. Expected today: above 0 in each frame (the writes at `widgets/values.rs:39`, `:65`, `:124`, `widgets/tooltip.rs:97`). This is the red that `UI-HUD` later turns green.

**Exit:** both results are in the report before `PRES-MODELS` and `UI-HUD` start. **Rows:** none. **Runs: 1 (1-2).**

### Task Q-LOCK: the GPU lock wrapper with the queue rule (now; ruling R-T, qa N1)

**Seat:** `qa-lead` (S). **Review:** `performance-architect` (O): can the hold limit and the queue order go red, and are the receipts complete?

**Branch / inputs:** none (the output is a file outside the repository; the plant record and the report go through Root); no input. First wave. **Design:** D section 10 "The GPU is the scarce resource" (D:2296-2297); section 1 of this plan (the lock queue rule).

**Files:** create `/home/julien/Projects/TuRTS/target/mc-evidence/gate-runner/mc_headed_v6.sh` from `mc_headed_v4.sh` (read 2026-11-12: it pins the headed profile, `PYTHONDONTWRITEBYTECODE=1`, the ALSA null device, and ends in `exec flock -x -o "$MC_EVIDENCE_BASE/gpu-runtime-2.lock" xvfb-run … python3 "$MC_LANE_RUNNER" --audio …`). `v5` is taken by `mc_headed_v5_alsa_null_experiment.sh`. Do not change v4: the MC lanes use it.

- [ ] **Step 1: Red.** `bash mc_headed_v6.sh --self-check; echo exit=$?` with an empty self-check body gives `exit=1`. The self-check uses its own lock file under `$MC_EVIDENCE_BASE/gpu-queue-selftest/`, never `gpu-runtime-2.lock`, and needs no GPU (the command is `sleep`). Cases: (a) a held lock past the limit: a `pre-merge` call of `sleep 30` with `MC_LOCK_MAX_SECS=2` ends within 5 s with exit 124, a second `flock -n` on the lock file gets it at once, and the receipt says `failed: hold limit`; (b) the queue order: while a `pre-merge` call waits, a `table` call does not take the lock; (c) a `table` call frees the lock after its row; (d) each receipt has `label`, `mode`, `wait_s`, `hold_s`, `limit_s`, `exit`, `start`, `end`.
- [ ] **Step 2: Green.** `--mode pre-merge` or `--mode table` (required). A `pre-merge` call writes a ticket file under `$MC_EVIDENCE_BASE/gpu-queue/` before it waits, and removes it when it holds the lock. A `table` call waits while a ticket exists, then holds the lock for one row only. Both run the command under `timeout "$MC_LOCK_MAX_SECS"` (default 1,200 s for `pre-merge`, 600 s for one table row) inside the flock of v4. Each call appends one JSON line to `$MC_EVIDENCE_BASE/gpu-queue/receipts.jsonl` and writes it into its label directory. The rest of v4 stays.
- [ ] **Step 3: Gate.** `bash -n mc_headed_v6.sh; echo exit=$?`; the self-check `exit=0`; `sha256sum mc_headed_v6.sh`.
- [ ] **Step 4: Plant.** Remove the `timeout` wrapper: case (a) is red (the call lasts 30 s and the lock stays held). Restore from the copy in the task record (`cmp` exit 0). The file is outside the repository, so the plant is in the task record, not in `mp1_plants.py`; the reviewer runs it again.

**Exit:** v6 and its sha256 in the report; Root writes the hash in section 1 and starts the daily receipt read on the board. Every headed MP1 task takes `Q-LOCK` as an input. **Rows:** none. **Runs: 1 (1-2).**

### Task L0-API1: the mod API 0.2.4 as types

**Seat:** `mod-runtime-engineer` (S). **Review:** `mod-builder` (O): can a modder write each 0.2.4 field from the table and the RON examples alone?

**Branch / inputs:** `mp1/l0-api1`; `L0-RAW`, `L0-RULES`, E1 (S2, D0p; S3 and S45 merged). **Design:** D 6.10 (D:1435-1742), the key lists (D:1600-1625), the defaults and RON examples (D:1627-1693), "The API bump" (D:2280-2286), `UNBUILT_FEATURES` (D:1388-1393), 6.7.4 match data (D:859-896); erratum E10 (row H24).

**Files:**
- `crates/turts_mod_api/src/schema.rs`, `orders.rs`, `interface.rs`, `assets.rs`, `lib.rs` (`:52` `ENGINE_API_VERSION`, `:55` `RELEASED_ENGINE_APIS`); create `keys.rs` (`REASON_KEYS`, `CURSOR_CONTEXTS`, `STATE_KEYS`); the H24 type `LabelStyle` (the label text is `UnitDef.name`, `schema.rs:337`).
- `crates/turts_mod_runtime/src/version_table.rs` (one row for each 6.10 row, with RAW, VAR, COMBO or ED); create `unbuilt.rs` (section 5.3).
- Struct literal sites of `UnitDef`, `Placement`, `CancelRefunds`, `ProductionDef`, `InterfaceDef`, `AlertDef`, `WeaponDef` (none derives `Default`). They are in `turts_sim/src` (`orders/` after S2, `registry.rs`, `command_m3.rs`, `economy/production.rs`, `machine.rs`, `combat.rs`, `research.rs`, `boot_m3.rs`), `turts_audit/src/replay.rs`, `turts_server/src/session.rs` (tests), `turts_client/src`, and the test files (grep `UnitDef {`). **Verify first** after S2.
- The five exhaustive `SlotRelation` matches (`orders.rs:1126`, `economy/combat.rs:313`, `turts_client/src/core_adapter.rs:340`, `widgets/cursor.rs:430`, `:619`) call `turts_rules::relation` with team = slot (R2, X10).
- **The matches with no `_` arm that new variants break (ux M1, ruling R-N):** `widgets/values.rs:201-233`, `widgets/sources.rs:64`, `:120`, `:184`, `:1215`, and `feedback.rs:620-837` (verify first; ux-designer grep at `d047433`). Step 2 adds an arm for each new variant that keeps the 0.2.3 behaviour (no output), so the crate compiles in lane-0 order; `L0-CORE` then turns the arms into owner stubs.

- [ ] **Step 1: Red.** Tests: `acquire` in a `^0.2.3` package is refused with the text of D:1458-1460; a `^0.2.4` package is refused for the API without the bump (VAL1 plant (6)); a `^0.2.4` package with `constructor` is refused with "`constructor` is not built in this engine build"; a `^0.2.4` package with `world.unit_labels` is refused with the `UNBUILT` text. Each `exit=101`.
- [ ] **Step 2: Green.** The types with `#[serde(default)]` on new fields of existing structs (D:1461-1463); the bump in the same commit as the first 0.2.4 row (D:2284); the edition; every version-table row; the `UNBUILT_FEATURES` lines of section 5.3, each with its owner task in a comment; the 0.2.3-behaviour arms in the listed matches.
- [ ] **Step 3: Gate.** F-RUST, F-WASM (the client crates compile with the new variants), F-QA, F-FROZEN, F-PACK (every repository package `exit=0`).
- [ ] **Step 4: Plants.** (a) Remove one version-table row: its `^0.2.3` package packs with exit 0. (b) Remove the bump: the `^0.2.4` package is refused for the API. Restore each. Plant records.

**Exit:** 5.1 item 1 exists in code; the existing packages pack; the workspace compiles on host and wasm32. **Rows:** none (VAL1 is `L0-API2`). **Runs: 4 (3-5).**

### Task L0-API2: the validation column and VAL1

**Seat:** `mod-runtime-engineer` (S). **Review:** `mod-builder` (O): does each red package give the exact text of its row?

**Branch / inputs:** `mp1/l0-api2`; `L0-API1`. **Design:** D 6.10 Validation and Error-text columns, the "did you mean" rule (D:1450-1456, `crates/turts_mod_runtime/src/parse.rs:23-29`, verify first), 6.7.4 pack checks (D:881-896); V1, V2 go to `PRES-MODELS`; row VAL1 (D:2437).

**Files:** create `crates/turts_mod_runtime/src/mp1_validation.rs` (one function for each 6.10 group, H24 included); `content_m3_validation.rs:374-376`, `:418`, `:420-424` (the two lifted rules become COMBO rows K7, K8 behind `UNBUILT`; the `constructor` beside `producer` exemption at `:418`); `content_mc_validation.rs:1495-1519` (`is_engine_action` gains `leave_match`, H23); create `crates/turts_mod_runtime/tests/val1.rs` and `tests/fixtures/val1/` (one red package for each RAW row written and written empty, each VAR row, each COMBO row; a `^0.2.3` and `^0.2.4` pair for each ED row T1 to T4).

- [ ] **Step 1: Red.** The VAL1 package set on the `L0-API1` build: the explicit-empty, key, placeholder and reserved-variant packages pack with exit 0. `cargo test -p turts_mod_runtime --locked --test val1; echo exit=$?` gives `exit=101`.
- [ ] **Step 2: Green.** Each check and its text; the packer refuses `Q` above 65,536 (D:629) and a gap below the largest mobile diameter plus 0.2 (K2); a package with a constructor and no `open_sites_per_slot` (K6); `forge` and `muster` copies pack (H13 detection on the parsed `Button`, D:1476-1479); KEY1 (b), (c) packer refusals.
- [ ] **Step 3: Gate.** F-RUST, F-QA, F-PACK, F-FROZEN.
- [ ] **Step 4: Plants.** VAL1 plants (1) to (6) (D:2437); KEY1 (b), (c). Restore each. Plant records.

**Exit:** VAL1 green with its reds. **Rows:** VAL1. **Runs: 3 (2-4).**

### Task L0-SIM: UnitId index, sim gate, system slots and the weapon gate

**Seat:** `simulation-core-engineer` (S). **Review:** `performance-architect` (O): is the index used only for lookup, and do the slots sit where D says?

**Branch / inputs:** `mp1/l0-sim`; E1 (S2, D0p). **Design:** D section 9 "UnitId index in lane 0" (D:2212-2230), "String ids become indices" (D:2232-2236), D 6.7.5 gate (D:900-905), D 6.2 computation (D:519-522), D 6.7.8 outcome check place (D:1113), section 14 row 2 (D:2521); R17 (ruling R-K).

**Files:**
- Create `crates/turts_sim/src/unit_index.rs`. Use it at site 1 `economy/production.rs:282`, site 2 `economy/transaction_m3.rs:635` and `:524` (a reverse set where ordered targets are set and cleared, or keep the scan and record its cost, D:2219-2220), site 7 `validate_target` (`orders.rs:3028`, calls at `:3056`, `:3085`; verify first after S2). Insert at spawn (`economy/transaction.rs` spawn path near `:710`; the map spawn in `boot_m3.rs`), remove at `economy/transaction.rs:747`.
- Footprint and supply resource ids to indices at catalog install.
- Create `crates/turts_sim/src/match_config.rs`: the sim `MatchConfig { tick_hz, start_tick, teams, settings }` (D:1197-1200) and the run condition on each `SimSet` (`plugin.rs:31-44`). `SimPlugin` (`plugin.rs:48-52`) inserts no config. `crates/turts_sim/src/harness.rs:11-13` (`headless_app`) inserts the `Immediate` config. `crates/turts_server/src/main.rs:68` inserts it at boot under `Immediate`; the four test builders `session.rs:792`, `:963`, `:1116`, `:1747` insert it (R17).
- `crates/turts_sim/src/plugin.rs`: empty systems for supply (start of the economy phase), income, construction placement, acquisition grids, and the outcome check (after the death path, before `Collect`), each in its own new file.
- Create `crates/turts_sim/src/economy/power.rs` with the stub `pub fn weapon_enabled(world, unit) -> bool { true }`; `economy/combat.rs` calls it at the fire check and at today's targeting (one call line each; TD H2 (c)). `L-PWR` fills the body; it does not edit `combat.rs`.
- In the same file, the stub `pub fn rate_factor(world, unit) -> Rate` (full rate; TD sequencing proposal, the `weapon_enabled` pattern); `economy/production.rs` calls it where a job advances (one call line); `L-CON1` calls it where a site advances. `L-PWR` fills the body; it does not edit `production.rs` or `construction.rs`.
- The job fields `held` and `held_since_tick` and the engine constant `HOLD_CLEAR_MS = 1_000` (erratum E21, ruling R-Y) as shapes; `L-PAY` writes them.

- [ ] **Step 1: Red.** Tests: a lookup of a removed id gives no entity; with no `MatchConfig` the tick does not advance; the slot order test (outcome check after removals); `headless_app` ticks (it inserts `Immediate`). Each red on the base.
- [ ] **Step 2: Green.** State in the report the count of `SimPlugin` builders (6 at `03aef38`: `harness.rs:13`, `main.rs:68`, `session.rs:792`, `:963`, `:1116`, `:1747`) and of `headless_app` users (51 files).
- [ ] **Step 3: Gate.** F-RUST, F-FROZEN, F-REF (selector on the changed files). Early test (D:2521): the 31 ignored process tests (`grep -c '#\[ignore'` for each file of `crates/turts_server/tests`), hd1 and T7 (`bash scripts/t7.sh`) on this build; none waits for a lobby. Headed 1 (T7 under the lock; whether T7 needs the lock is NOT TRACED, so it takes it).
- [ ] **Step 4: Plants.** (a) `headless_app` without the `Immediate` insert: `muster` with no client has 0 ticks after 3 s (IMM1 red build 1). (b) The index not updated at removal: the lookup test is red. (c) `weapon_enabled` called at fire only: a combat test with the stub returning `false` still sees a target chosen (test of the two call sites). (d) The `rate_factor` call removed from `production.rs`: a test with the stub returning 1/2 sees a job at full rate. Restore each. Plant records.

**Exit:** 5.1 item 4 exists; the frozen stream is unchanged. **Rows:** none. **Runs: 4 (3-5). Headed: 1.**

### Task L0-WIRE: protocol 5, the order writer and the record writers

**Seat:** `network-engineer` (S). **Review:** `backend-engineer` (O): does each record of D 6.8 exist with its channel and viewer class, and can the writer scan go red?

**Branch / inputs:** `mp1/l0-wire`; E1 (S2, D0p), `L0-DBG` (replica.rs order). **Design:** D 6.8 (D:1291-1393), D 6.7.13 (D:1257-1272), section 15 point 14 (D:2582-2590), row ORDW1 (D:2477); R19 (ruling R-I).

**Files:**
- `crates/turts_net_protocol/src/messages.rs` (`Welcome` `:26-31`, `last_failure` `:136`, `CommandRejectReason::as_str` `:225-264`), `economy.rs`, `lib.rs:13` (4 to 5).
- `crates/turts_net_lightyear/src/protocol_plugin.rs:26-43` (five messages after `ResyncRequest`), `channels.rs:5-6` (comment).
- `crates/turts_sim/src/economy/events.rs:138` (`CommittedTick`: the new fields, among them the order-change list).
- The job records and `SiteProgress` carry `held: bool` and `held_since_tick: u32` (erratum E21, ruling R-Y); they are sent on change of rate, job state or `held`.
- **The one order writer (R19, TD H3).** In `orders/` (verify first after S2): `fn set_unit_orders(world, unit, new, changes)` writes `UnitOrders` and pushes the change to the `CommittedTick` change list. Every production write of `UnitOrders` goes through it (about 62 write lines at `a6f3f66`, pattern count, `gate2-technical-director.md:14`).
- **The writer scans.** Create `crates/turts_checks/tests/order_writers.rs` (the syn scanner of `crates/turts_checks/tests/support/mod.rs`, the DES1 pattern): (1) the only production write of `UnitOrders` outside `set_unit_orders` is none. The scan names the write patterns it must see (TD M-A; at `d7d7e6d` `crates/turts_sim/src/orders.rs` has 24 lines with `get_mut::<UnitOrders>` or `remove::<UnitOrders>`): `get_mut::<UnitOrders>()`, `remove::<UnitOrders>()`, `insert` of a `UnitOrders` value (a struct literal or a binding of that type), a field assignment on a `UnitOrders` binding, and a helper that takes `&mut UnitOrders` or `&mut Order`. A syn scan sees names only, so it cannot see a write through an inferred `get_mut()`, an `insert(var)` of an inferred type, or a despawn that must send `orders_cleared`. For those, a `test-probes` system in `SimSet::Collect` (the feature `test-probes`, `crates/turts_sim/Cargo.toml:11`; `Collect` at `crates/turts_sim/src/plugin.rs:43`) compares `Changed<UnitOrders>` and `RemovedComponents<UnitOrders>` of the tick with the change list of `CommittedTick` and panics on a difference; F-RUST runs the sim tests with `--features test-probes`; (2) the export writers `export_*.rs` read only `CommittedTick` fields, no `Query` of sim components (regate-performance M5, rigour L3). Both run in F-RUST.
- `crates/turts_server/src/export.rs` (`unit_orders` as changes `:190-195`; `WeaponRecord.ready_tick` `:174-177`; `owner_of` by binary search `:299`, the view is sorted, `crates/turts_sim/src/world.rs:61`; the per-viewer visible set); create `export_sites.rs`, `export_supply.rs`, `export_jobs.rs`, `export_effects.rs`, `export_match.rs` with empty writers.
- `crates/turts_server/src/session.rs`: the bucket first (`:403-424`); `Welcome.start` under `Immediate` (`:268-283`).
- `crates/turts_client/src/replica.rs` (merge in chain order `:237-239`; `orders_cleared`; today's replacement `:523-525`), `ingest.rs:422-436`, `join.rs` (`Welcome.start`).

- [ ] **Step 1: Red.** ORDW1 idle case: 200 idle units for 100 ticks, `unit_orders` bytes of each delta are 0; today each delta carries the full table, so it is red (ORDW1 plant (2) is today's behaviour). ORDW1 single-move case (R-I): one `MoveTo` on one unit gives exactly one changed record in the delta of the accept tick and one in the delta of the arrival tick (record bytes, not operations: slice rule, section 1). The point-14 test: the replica applies a `Snapshot` that it did not request, and keeps `IgnoredStale` for `tick <= last_applied` (`replica.rs:217-221`, `:258-275`). The writer scan with a direct write planted.
- [ ] **Step 2: Green.** The writer function and the change list; if the point-14 test is red, make the replica accept the `Snapshot` (D:2589-2590).
- [ ] **Step 3: Gate.** F-RUST (with `order_writers`), F-WASM, F-QA, F-FROZEN, F-REF (`session.rs`, `export.rs`, `orders/`). Binding note (recheck5-network-host :36): no process test depends on refusals that skip the bucket; a red there stops the task.
- [ ] **Step 4: Plants.** ORDW1 plants (1) no `orders_cleared` (case A), (2) full table (idle bytes), (3) Snapshot merged into the old table (case B), (4) `WeaponRecord` in each tick (case C weapon part); "the change of a single `MoveTo` dropped" (the arrival write bypasses the change list: the single-move case is red); "a direct write of `UnitOrders`" (the scan is red); "a `Query<&Health>` in `export_sites.rs`" (the scan is red). "a write through an inferred `get_mut()`" (the scan stays green, the `test-probes` check panics). Restore each. Plant records.

**Exit:** 5.1 item 2 and the writer of item 4 exist; ORDW1 cases A, B, the single move and the weapon part of C are green here; `I-HEAD` runs the whole row. A forward merge of a slice task that writes `UnitOrders` directly is red in F-RUST (the scan or the `test-probes` check); its MP1 owner (`network-engineer`) routes the write through `set_unit_orders`. Root counts 1 repair run for this at each forward merge of A2b1, A2b2 and A2c (3 runs, section 14.1). The slice builders know the rule from the slice plan amendment of revision 3. **Rows:** none owned. **Runs: 5 (4-6).**

### Task L0-STORE: the client store of the 6.8 records (ruling R-H)

**Seat:** `network-engineer` (S). **Review:** `bevy-rendering-engineer` (O): does every UI task get its values from one getter, and does the store commit with the candidate?

**Branch / inputs:** `mp1/l0-store`; `L0-WIRE`. **Design:** D 6.8 records (D:1298-1321), D:1381 ("the replica is the only source", erratum E11), D:2257 ("MP1 adds no field to the replica candidate"; it stays true); TD H1.

**Files:** `crates/turts_client/src/replica.rs` (`Candidate` `:82`, `candidate_at` `:432-446`, `apply_delta` `:278`; verify first); create `crates/turts_client/src/replica_store.rs`; create `crates/turts_client/tests/mp1_store.rs` (the pattern of `crates/turts_client/tests/economy_replica.rs`).

- The store holds sites, `SiteProgress`, `ConstructionQueue`, `SupplyRecord`, `MatchState` and the job records. It is a side store, not a field of `Candidate`. It is not cloned for each delta.
- One apply path, staged with the whole chain (TD H-B): the store change set is staged with the candidate through `stage_snapshot` (`crates/turts_client/src/replica.rs:379`) and through each `stage_delta` (`:449`) of the replay, and `commit` (`:553`) applies both. A resync that fails after its Snapshot (a retained delta that fails, a remainder, an open gap: `apply_snapshot` returns `ResyncRequested` at `:260-267` and keeps the old display) leaves the store unchanged. `IgnoredStale` (`:221`) and `Discarded` (`:287`) change nothing. The cost of the store apply on a Snapshot is one store replacement; a delta applies only its changes. (Lines read at `d7d7e6d`.)
- One source getter for each `Value` of 6.10 that reads a 6.8 record (`Supply`, `ConstructionQueue`, `Sites`, `Outcome`, `OutcomeReason`, `MatchClock`, the item fields `State`, `Paid`, `Refund` through `turts_rules::progress`). No interface task adds storage of its own (5.1 item 5).

- [ ] **Step 1: Red.** The contract test: apply a hand-built protocol-5 `Snapshot` and then a `WorldDelta` through `Replica::apply_delta`; each getter returns the record values; a delta that fails to apply leaves both the candidate and the store at the old tick; a counter shows 0 store clones for 100 deltas. Three more cases (TD H-B), each with the store unchanged or equal to the candidate tick: (1) a resync where a retained delta fails; (2) a stale Snapshot (`IgnoredStale`); (3) a resync with 2 replayed deltas (the store equals the candidate after the last one). `todo!()` bodies: `exit=101`.
- [ ] **Step 2: Green.**
- [ ] **Step 3: Gate.** F-RUST, F-WASM.
- [ ] **Step 4: Plants.** (a) Commit the store before the candidate apply: the failed-delta case shows a new store with an old candidate (red). (b) Clone the store in `candidate_at`: the clone counter is red. (c) Apply the store for each message in place of the staged chain: case (1) shows the store ahead of the candidate (red). Restore each. Plant records.

**Exit:** the store and its getters exist; the UI lanes read only the getters. **Rows:** none. **Runs: 2 (2-3).**

### Task L0-SPLIT: the two pure-move splits (now; rulings R-S, R-X)

**Seat:** `bevy-rendering-engineer` (S). **Review:** `technical-director` (O), an independent review of the two pure moves (as the slice did for S2, S3 and S45): the behaviour oracle, the item-equality proof, the test-list equality and one fault of its own.

**Branch / inputs:** `mp1/l0-split` from `pre-mp1/integration` at `d7d7e6d`; no input beyond the slice head (S45 is merged). It can start now. **Design:** D section 9 client cost rule (D:2261-2275); the S45 review advice (ruling R-S); TD M-B.

**Files:** `crates/turts_client/src/visuals.rs:279` (`draw_world_visuals`, read at `d7d7e6d`: the despawn-and-`return` at `:333-343` when the content, the `Welcome` or the camera is missing; the accumulators that the concerns share, `desired` `:437`, `ring_metadata` `:438`, `health_fractions` `:621`); new files under `crates/turts_client/src/visuals/`; `crates/turts_client/src/core_adapter.rs:730` (`drive_core_adapter`; `match effect {` at `:1044`, the arms at `:1045-1123`); create `crates/turts_client/src/core_adapter/effects.rs`; create the scene oracle `crates/turts_client/src/visuals/scene_tests.rs` (`cfg(test)`).

**Rules of the split (TD M-B):** (1) each early exit stays in the caller: `draw_world_visuals` keeps the `return` of `:343` and each other exit that ends the whole function; a concern function has no exit that the old body did not have at that place; (2) the shared accumulators stay in the caller and pass in by `&mut`, in the present order of writes; (3) the calls stay in the present order.

- [ ] **Step 1: The behaviour oracle first.** No headless test runs `draw_world_visuals` today: its only caller is the system tuple at `visuals.rs:175`, and `visuals/tests.rs` runs only test helpers (`:566-569`, `:614`) (grep at `d7d7e6d`). So write the scene oracle on the unsplit build: a headless app with `prepare_assets` and `draw_world_visuals` and fixed scenes (no camera; no `Welcome`; one own unit selected; a hover; a move marker; a blocked mark; queued targets; health bars at three fractions; two slots). After 3 updates it records the set of visual entities (`VisualId`, mesh handle, material, `Transform` rounded to 1e-4) and compares it with a committed fixture. Show it red on a planted change (the health-bar write removed: `exit=101`), restore, green.
- [ ] **Step 2: Pure move A.** Split `draw_world_visuals` into one function for each concern under the rules. Proof: the scene oracle green; `item_equal.py` (SP section 1 "Pure moves") on the moved items; `cargo test -p turts_client --locked -- --list` before and after, `cmp` (test-list equality); the joined bodies equal the old body except the call lines (scratch script, `cmp`).
- [ ] **Step 3: Pure move B.** Move the Effect match to `core_adapter/effects.rs`, one function for each effect family. The same proof, with the adapter tests of `core_adapter/adapter_tests.rs` as the behaviour oracle (verify first which test runs each family; if a family has no test, write one first and show it red on a planted change).
- [ ] **Step 4: Gate.** F-RUST, F-WASM, F-QA.
- [ ] **Step 5: Plants.** (a) Move the despawn-and-`return` of `:333-343` into the first concern function, with the caller going on to the later concerns: the joined-body proof can stay equal, and the scene oracle is red on the scene with no camera (the plant that the text proof cannot see). (b) Change the call order of two concerns: the joined-body proof is red. (c) Drop one effect-family call: an adapter test is red. Restore each. Plant records.

**Exit:** the two splits exist; the scene oracle runs in F-RUST. **Rows:** none. **Runs: 2 (2-3).**

### Task L0-CORE: `Core::step` split and client stubs (ruling R-S)

**Seat:** `bevy-rendering-engineer` (S). **Review:** `technical-director` (O). `ux-designer` reads the stub layout in the same review run.

**Branch / inputs:** `mp1/l0-core`; `L0-API1`, `L0-SPLIT`. **Design:** D section 9 crate table row `turts_interaction` (D:2199), D 7.5 core items (D:1910-1911); the S45 review advice (ruling R-S).

**Files:**
- `crates/turts_interaction/src/lib.rs:143` (`pub fn step(`); create `placement.rs`, `hints.rs`, `alerts.rs` (stubs).
- `crates/turts_client/src/core_adapter/effects.rs` (from `L0-SPLIT`): one stub function for each new effect family.
- `crates/turts_client/src/widgets/activation.rs:43` (`activate_widget`); create `widgets/activation/cancel.rs`, `leave.rs`, `place.rs`, `rally.rs` (stubs).
- `crates/turts_client/src/widgets/primitives.rs:13-20` (`WidgetAction`: add `Place { constructor, content }` and `FocusAlert`), `widgets/sources.rs`, `widgets/values.rs`, `feedback.rs`: the dispatch arms of `L0-API1` become one stub function for each new 6.10 `Value`, `Source`, `Predicate`, `PrimaryField`, `ItemField` variant, in a stub file of its owner task (section 5.3).

- [ ] **Step 1: Split `Core::step`** by input kind, with no behaviour change. Proof: `cargo test -p turts_interaction --locked -- --list` names equal before and after (`cmp`), and the interaction tests are green.
- [ ] **Step 2: Stubs.** The owner stubs above return none; a 0.2.3 layout renders as before (the scene oracle of `L0-SPLIT` stays green).
- [ ] **Step 3: Gate.** F-RUST, F-WASM, F-QA.
- [ ] **Step 4: Plants.** (a) Swap two input-kind arms: an interaction test is red (verify first which; lead: `crates/turts_interaction/tests/km3.rs`, `shift_click_toggles` `:9`). (b) A stub that returns a value for a 0.2.3 layout: the scene oracle is red. Restore each. Plant records.

**Exit:** 5.1 item 5 exists; each later client task adds its own file and at most one call line. **Rows:** none. **Runs: 3 (2-4).**

### Task L0-EXIT: lane-0 freeze review (stage-1 shapes)

**Seat:** `technical-director` (O). **Review:** `mod-builder` (O): the sample mod.

**Branch / inputs:** none (a review of `mp1/integration`); `L0-API1`, `L0-API2`, `L0-WIRE`, `L0-STORE`, `L0-RULES`, `L0-SIM`, `L0-CORE` (with `L0-SPLIT` in its tree), `Q-PLANTS`. **Design:** D section 10 lanes (D:2310-2315), D 6.8 (D:1388-1393), section 5 of this plan.

- [ ] **Step 1.** A script reads D 6.10 and the code: each row name is built in lane 0 or has exactly one line of section 5.3. Each record of D 6.8 exists with its field names. Output: `docs/research/mp1/lane0-freeze.md` with the commit, the frozen list (5.1 items 1 to 6) and the list of 5.1 item 7 (stage-2 shapes, not frozen until `L0-EXIT2`).
- [ ] **Step 2.** `mod-builder` packs the sample mod `docs/research/mp1/probes/regate-tide/tide/` against the frozen build: the only refusals are `UNBUILT` lines.
- [ ] **Step 3.** A plant for the script: delete one line of section 5.3 from `unbuilt.rs`: the script is red.

**Exit:** the freeze note exists; change requests of the stage-1 shapes follow 5.2 from here. **Runs: 1 (1-2).**

### Task L0b-ORD: behaviour handler table and engine orders

**Seat:** `simulation-core-engineer` (S). **Review:** `technical-director` (O).

**Branch / inputs:** `mp1/l0b-ord`; E2 (A1c), `L0-SIM`, `L0-EXIT`. **Design:** D section 9 "Behaviour handler table" (D:2277-2278), D 6.5 "Engine orders" (D:737-751); R6.

**Files:** `crates/turts_sim/src/orders/` (verify first after S2: `apply_order` at `orders.rs:2610` on the base); stub files for `Construct`, `CancelConstruction`, `SetRally`, `Repair`, `Surrender`; `OrderSource::Engine`; `crates/turts_server/src/orderlog.rs` (source field); `crates/turts_server/src/outbound.rs:26-31` (an engine order writes nothing into the `(slot, seq)` result list).

- [ ] **Step 1: Red.** Tests: a fake player envelope for an engine order is refused (TD part B5); an engine order gives no `OrderResult` and costs no token; its refusal shows in `last_failure` (`messages.rs:136`); the order log line has the source `Engine` and no seq. Slice rule: no test pins the operations of an engine move.
- [ ] **Step 2: Green.**
- [ ] **Step 3: Gate.** F-RUST (with `order_writers`), F-FROZEN, F-REF (`orders/`).
- [ ] **Step 4: Plant.** Route an engine order through the player admission: the token test is red. Restore. Plant record.

**Exit:** 5.1 item 6 exists. **Rows:** none. **Runs: 2 (2-3).**

## 7. Stage 1 tasks

Stage 1 is the thin loop (D:57-60). Each block names the rows that it makes green; the plants are the red builds of the row in D section 12, at the D line given, and section 3.6 lists them. Each task writes its plant record file. A task with a pixel row runs it green and every pixel plant red in one pre-merge session in its lane (F-HEAD, R-A, R-U); B1 (`I-PIX1`) re-runs the rows.

### Task L-MATCH-SIM: outcomes, teams and the end of the match (sim)

**Seat:** `simulation-core-engineer` (S). **Review:** `backend-engineer` (O). **Inputs:** `L0-EXIT`. **Design:** D 6.7.8 (D:1113-1134), 6.6 (D:757-771), 6.7.9 first bullets (D:1138-1141); X11, X10.

**Files:** create `crates/turts_sim/src/match_state.rs` (`MatchState { connections, outcomes }`, `KeyUnitsLost` counts per slot changed at spawn and death, no world scan; the count at site completion is added by `L-CON1`, TD L1); the outcome-check slot of `L0-SIM`; `StopReason::MatchOver` (`economy/events.rs:183-197`) after the `Collect` chain of tick t (`plugin.rs:151`, `:157-160`); teams from the sim `MatchConfig`; `turts_server/src/export_match.rs` (public, on change).

- [ ] Red: sim tests for MAT1 (a) and (b) logic and the draw rule. Green. Gate: F-RUST, F-FROZEN. Plants: the draw rule gives `Victory` to the lower slot index ((b) red); a defeat with no clause ((i) logic). Plant records.
- The three-slot sim test of MAT1 is dropped (ruling R-F, erratum E1): MP1 has exactly 2 slots (`SLOT_COUNT` 2, `crates/turts_sim/src/slots.rs:7`), and no test-only slot count goes into the sim.

**Rows:** none owned (MAT1/s1 is `L-MATCH-SRV1`). **Runs: 3 (2-4).**

### Task L-MATCH-SRV1: the room, the start step, Ended and the stage-1 hooks

**Seat:** `backend-engineer` (S). **Review:** `network-engineer` (O): does the stage-1 trace of recheck5-network-host §2 hold end to end?

**Branch / inputs:** `mp1/l-match-srv1`; `L-ROOM0`, `L-MATCH-SIM`. **Design:** D 6.7.3 to 6.7.12 stage-1 parts (D:805-1255), X11b stage 1 (D:257), X12 server writer (D:258), rows IMM1 (D:2474), MAT1 (D:2447, cases (a) and (b), erratum E1).

**Files:** `crates/turts_server/src/room.rs` (wire in), `session.rs` (Hello by state, `:146-283`; the commands of a seated connection `:357-360`; after the end `:386-388`), `main.rs` (`:72`, `:78`, `:139-142`), `config.rs` (`after_match_secs`), `stats.rs` (`room_state`, `match_over_tick`, `deltas_after_end`), `outbound.rs` (`send_deltas` guard in Ended, `:51-84`), `command_script.rs`, `crates/turts_server/tests/help.rs` (reads `config.rs`, `:1-3`); create `mods/standoff/` (stage-1 form, D:1249-1254) and `scripts/mp1-serve.sh` (D:1173-1178).
- **Process tests (qa M2, ruling R-O):** create `crates/turts_server/tests/support/match.rs` with `spawn_clients(n, options) -> Vec<ClientRun>` (the shared N-client helper; today the pattern is file-local: `crates/turts_server/tests/acceptance.rs:124`, `economy.rs:3579-3603`), `crates/turts_server/tests/imm1.rs` and `crates/turts_server/tests/mat1.rs` (`#[ignore]`), and one `run_cargo_test` line for each in `scripts/acceptance.sh`.

- [ ] **Step 1: Red.** IMM1 on the `L0-SIM` build with a lobby default: `muster` has 0 ticks. MAT1 (a) with `--start-now --command-script` on `standoff`: no `match_over_tick`.
- [ ] **Step 2: Green.** Boot to Lobby or Running (D:912); the start step in one exclusive system, steps 1 to 4 (D:918-928), with the tick-0 `Snapshot` built in the step, not through `SnapshotDue` (`session.rs:683-685`); the server inserts the sim `MatchConfig` by its policy (R17); the seat release on the hook that `L-ROOM0` names; `LobbyState { you, seats, start: Waiting }`; `match_not_started` before the start (bucket of the seat index, D:1044-1048); `match_over` after the end; Ended: the end-tick `Snapshot` on the reliable channel, then `MatchResult` (`stats: []` in stage 1; Q7, erratum E17), then no deltas; `LeaveMatch` only in Ended, kick after `KICK_DELAY` (`session.rs:30`); `The match is over`; the exit rule (D:1163-1169); `--start-now`, `--input-log`, the three `--stats` fields; `after_match_secs` in `server.ron` and `--help`. Verify first whether `turts_pack::load_package` parses the interface file (the `slot_colours` length at boot, D:878).
- [ ] **Step 3: Gate.** F-RUST, F-QA, F-FROZEN, F-PROC (every ignored test of `crates/turts_server/tests`, the 31 of D:2521, and `imm1`, `mat1`), hd1, F-PACK (`standoff`).
- [ ] **Step 4: Plants.** IMM1 plants (D:2474): the default policy a lobby; `--start-now` ignored; a version-table row missing. MAT1 stage-1 plants: the draw rule; the tick log has a tick after T. Restore each. Plant records.

**Rows:** IMM1, MAT1/s1 (cases (a), (b)). **Runs: 5 (4-7).**

### Task L-LOBBY-C1: the client `Seated` phase and the stage-1 client hooks

**Seat:** `network-engineer` (S). **Review:** `web-platform-engineer` (O): does the browser keep the id from the seat on?

**Branch / inputs:** `mp1/l-lobby-c1`; `L0-EXIT`; merge after `L-MATCH-SRV1`. **Design:** D 6.7.7 (D:1054-1066), 6.7.12 client hooks (D:1229-1248), X12 stage 1 (D:258), 7.6 stage-1 lines (D:1925-1926, D:1936, D:1941-1942); rows LOB1 (D:2448), LEAVE1 (D:2475).

**Files:** `crates/turts_client/src/join.rs` (`Phase::Seated` at the first `LobbyState` with `you`, `advance_phase` `:107`; `:479`), `rejoin.rs` (`advance_to` `:306-311` counts `Seated` as joined), `actions.rs` (`on: { event, wait_ms }`; `leave`; `:107-118`), `main.rs` (`--rejoin` for a headless client, `:38-43`), the `--out` report fields (D:1243-1247, `end_snapshot_tick` on receipt), `crates/turts_client_web/src/lib.rs` (the save at `Seated`, `:259`), `client_shell/notices.rs` (the "Waiting for the other player" line). **Process tests:** create `crates/turts_server/tests/lob1.rs` and `leave1.rs` on the helper of `L-MATCH-SRV1` (`tests/support/match.rs`), with their `acceptance.sh` lines.

- [ ] Red: LOB1 run 0 on today's client: no `lobby_states` entry. Green. Gate: F-RUST, F-WASM, F-PROC (`lob1`, `leave1`), F-HEAD (1 run: the browser save at `Seated`, run 4 in a window).
- [ ] Plants: LOB1 run 0 plants (the sim ticks before the second seat; the start step through `SnapshotDue` with two fixed steps) and run 4 plant (a seat bound to the player id); LEAVE1 plants (D:2475: no exit in Ended; the same `match_id`; the script restarts after a non-zero exit; an owner that did not leave gets `The match is over`; the revision-4 exit rule; the end `Snapshot` left out; no guard in Ended; `LeaveMatch` before Ended). Plant records.

**Rows:** LOB1/s1 (run 0; run 4 in its stage-1 form), LEAVE1/s1. **Runs: 3 (3-5). Headed: 1.**

### Task L-PAY: charge over time, the production cancel and the held flag

**Seat:** `simulation-core-engineer` (S). **Review:** `qa-lead` (O): does the ledger rule balance with `charged`? The review run (run 2) also reviews `L-INC` (joint review, qa L1).

**Inputs:** `L0-EXIT`, A1d. **Design:** D 6.4 (D:618-668) with erratum E21; X7; rows PAY1 (D:2444, with the E21 case), PAY2 (D:2466); open point 19 residual, closed by ruling R-Y.

**Files:** `crates/turts_sim/src/economy/production.rs` (`done`, `total = duration_ticks x Q`, `try_ms_to_ticks` at `boot_m3.rs:591`; all costs or none; `WaitingForFunds`; owner unit id then queue order; coalesced `Charged`; `charged`, `held` and `held_since_tick` on the job; the cancel refund from `charged`), `crates/turts_audit/src/` (the ledger rule in the A1b balances domain; verify first after A1d), `crates/turts_server/src/export_jobs.rs`.

- **The audit rule (TD L1, ruling R-K):** it checks the conservation identity only: the balance change of a slot in a tick equals minus the sum of its `Charged` events plus its credits, and `charged <= cost` for each job. It never computes the charge schedule again. Plant: "the audit recomputes the schedule": a ledger rule that compares each `Charged` amount with its own `floor(cost x done / total)`; a test with a job whose charge the sim paid in a later tick (a starved tick) shows the audit red where the identity holds.
- **The held flag (ruling R-Y, erratum E21; closes Q11 and open point 19 residual).** The job state of 6.4 stays per tick: `WaitingForFunds` on a tick that cannot pay in full. Under the free income it flickers (income pays 1 tick in about 12; the longest unbroken run in a 14.5 s span is 1.3 s, `docs/research/mp1/c0-pace-stage1-pacing.md`, section 3). The job record gains `held: bool` and `held_since_tick`: `held` becomes true at the first tick that cannot pay in full (and `held_since_tick` is that tick); it becomes false when the job has paid in full on every tick for 1,000 ms (the engine constant `HOLD_CLEAR_MS`; 20 ticks at 20 Hz). The hint condition `JobOnHold(secs)` and the HUD words "On hold: no credits" read `held` and the time since `held_since_tick`, not the per-tick state. So a hold that flickers because the free income pays one tick in twelve counts as one hold. The job record exports both fields (5.1 item 2).

- [ ] Red, green, gate (F-RUST, F-FROZEN, F-REF), plants: PAY1 (1) to (5), PAY1 (6) (E21: `held` follows the per-tick state), PAY2 (1), (2), "the audit recomputes the schedule". The E21 case of PAY1: income that pays 1 tick in 12 for 15 s gives `held` true for the whole span, and `held_since_tick` stays at the first starved tick; plant PAY1 (6) makes `held` follow the per-tick state, so `held` is false in 11 of 12 ticks (red). Plant records.

**Rows:** PAY1, PAY2. **Runs: 3 (3-5).**

### Task L-INC: free income

**Seat:** `simulation-core-engineer` (S). **Review:** joint, in `L-PAY` run 2. **Inputs:** `L0-EXIT`, A1d; merge after `L-PAY`. **Design:** D 6.10 row U7 (D:1523), section 5 (D:309); X26 (D-L).

Stage note (ruling R-Q): `L-INC` stays in stage 1. The pacing of section 5 of D and of `C0-PACE` needs the free income: the floor L2 (a slot with 0 credits and no hauler gets a hauler in 166 s, D section 5 row L2) and the C0-PACE timeline (income 11.6 credits each second at the start, of which 3 are free) assume it. Without it, C1 loses the floor.

**Files:** create `crates/turts_sim/src/economy/income.rs` (one ledger event for each period; `period_ms` to ticks at boot, rounding up, `boot_m3.rs:591-599`; only while complete and health above 0); the income event kind in `turts_audit/` (rebase on `L-PAY`).

- [ ] Red, green, gate (F-RUST, F-FROZEN, F-REF), plants INC1 (a) to (c) (D:2465). Plant records.

**Rows:** INC1. **Runs: 1 (1-2).**

### Task L-DMG: damage table and armour

**Seat:** `simulation-core-engineer` (S). **Review:** `qa-lead` (O). **Inputs:** `L0-EXIT`. **Design:** D 6.3 formula (D:546-566); X2 damage; row DMG1 (D:2438).

**Files:** `crates/turts_sim/src/economy/combat.rs` (the modifier `:371-374`, the applied damage `:376`; the parse at `:265` becomes an index, R5); create `economy/damage.rs` (install with indices; calls `turts_rules`).

- [ ] Red: the two cells and the transposed table on a test package (not `frontier`). Green. Gate: F-RUST, F-FROZEN, F-REF, a source test that `combat.rs` calls no `ContentId::parse` in a tick system (regate-performance M4). Plants: DMG1 (1) f64 and f32, (2) axes swapped, (3) `flat` before the ratio. Plant records.

**Rows:** DMG1 (the duels with `frontier` numbers are recorded by `C1-FINAL`; a failed duel is a retune, D:2438). **Runs: 2 (2-3).**

### Task L-TGT: acquisition, retaliation and hold

**Seat:** `simulation-core-engineer` (S). **Review:** `performance-architect` (O): does the counter show the cadence, and is the grid built once per tick?

**Inputs:** `L-DMG`, `L-MATCH-SIM`, A1c, A3 as a cross stack (`A3^`, ruling R-W: the task branches from `mp1/integration` and merges `pre-mp1/a3-grids` at the end of the A3 build; A3 merges into the slice with a merge commit; slice plan amendment of revision 3); merge after `L-PWR`. **Design:** D 6.3 (D:568-611); D section 9 sites 4 and 5 (D:2222-2225); rows TGT1 (D:2439), TGT2 (D:2476).

**Files:** `economy/combat.rs` (`choose_target` before the cooldown today, `:201`, `:229`; units without `acquire` keep it); the per-tick unit frame for `combat.rs:94` and `orders.rs:703` on the A3 code (verify first on the merged A3 branch); create `economy/acquire.rs` (one grid for each team; cell `max(2.0, smallest acquire.range / 2)`; `(t + unit_id) mod 4 = 0`; `power::weapon_enabled` false stops acquisition too, the PWR1 clause); the `cfg(test)` counter pattern of `crates/turts_sim/src/movement/mod.rs:32-37`. A3 and `L-TGT` do not edit the unit frame and the cell of `combat.rs:90-117` at the same time: `L-TGT` builds on A3's code (TD H2 (b), H-A).

- [ ] Red, green, gate (F-RUST, F-FROZEN, F-REF), plants TGT1 (1) to (4), TGT2 (1) to (4) (test package: acquire 7 under a turret of range 8). Plant records.

**Rows:** TGT1, TGT2. **Runs: 4 (3-5).**

### Task L-CON1: construction, held orders and sites

**Seat:** `simulation-core-engineer` (S). **Review:** `qa-lead` (O): can each cause go red alone, and does the cap make an order wait?

**Inputs:** `L0b-ORD`, `L-PAY`. **Design:** D 6.1 (D:402-505); X4, X5 server part; rows CON1 (D:2440), PRD1 (D:2464).

**Files:** `orders/` `Construct` stub (from `L0b-ORD`); create `economy/construction.rs` (the queue holds orders only; the two caps make an order wait; placement of held orders as engine orders in ascending constructor id, then queue order; the site entity with progress, charge and job through `L-PAY`; footprint blocks from its tick; `site_health`; the check through `turts_rules::placement::check`; the `KeyUnitsLost` count at site completion through `match_state.rs`, TD L1; one call line to the stub `power::rate_factor` of `L0-SIM` where a site advances, and `held` on the site job, E21); `economy/production.rs` (exit points; a complete job waits for a free exit); `crates/turts_server/src/orders.rs` (`to_sim` for `ContentAtPoint`); `export_sites.rs` (`site`, `sites_completed`, `SiteProgress`, `ConstructionQueue`).

- [ ] Red, green, gate (F-RUST, F-FROZEN, F-REF), plants CON1 (1) to (6), PRD1 (a) to (d), "a completed site does not count as a key unit". Slice rule (section 1): until A2c merges forward, the CON1 cases assert arrival and progress, not operation counts. A 6-run task with no red after run 2 is reported (14.5). Plant records.

**Rows:** CON1, PRD1. **Runs: 6 (5-8).** On the critical path (section 3.1).

### Task L-PWR: supply and shortage effects

**Seat:** `simulation-core-engineer` (S). **Review:** `qa-lead` (O). **Inputs:** `L0-EXIT`, `L-DMG`; merge after `L-CON1` (TD sequencing proposal, ruling R-X): it builds against the frozen stubs `power::weapon_enabled` and `power::rate_factor`, and it runs PWR1 after it rebases on `L-CON1`. **Design:** D 6.2 (D:507-539); X6; row PWR1 (D:2443).

**Files:** `economy/power.rs` (fill the bodies of the stubs `weapon_enabled` and `rate_factor` of `L0-SIM`; one supply computation at the start of the economy phase; `when_short` through `turts_rules`: `Rate` on producer and constructor through the `rate_factor` call lines (`L0-SIM` in `production.rs`, `L-CON1` in `construction.rs`), `Disable` on the weapon for fire and for today's targeting through the two call lines that `L0-SIM` put in `combat.rs`); `export_supply.rs`. `L-PWR` does not edit `combat.rs`, `production.rs` or `construction.rs`.

- [ ] Red, green, gate (F-RUST, F-FROZEN, F-REF), plants PWR1 (1) to (4), with the second package `zeta:steam` and `Rate(1/3)`. Plant records.

**Rows:** PWR1. **Runs: 3 (2-4).**

### Task L-DES: the one death path for units with health and queues

**Seat:** `simulation-core-engineer` (S). **Review:** `qa-lead` (O). **Inputs:** `L-CON1`. **Design:** D 6.4 "Destruction" (D:670-681); X1; row DES1 (D:2467).

**Files:** `economy/transaction_m3.rs:354` (`remove_unit`, called from `economy/combat.rs:405`); `production.rs`, `construction.rs`; create `crates/turts_checks/tests/sim_writers.rs` (static check: the only production despawn in `turts_sim` is `economy/transaction.rs:747`; it uses the syn scanner of `crates/turts_checks/tests/support/mod.rs`, as `order_writers.rs` of `L0-WIRE` does).

- [ ] Red, green, gate (F-RUST, F-FROZEN, F-REF), plants DES1 (1) to (6) (test package with `destroyed` 1/2 and `running` 3/4), "a direct despawn outside the death path" (the scan is red). Plant records.

**Rows:** DES1. **Runs: 3 (2-4).**

### Task L-OBS: approach point, gap and the A3 measure

**Seat:** `simulation-core-engineer` (S). **Review:** `performance-architect` (O). **Inputs:** A2b2, `L-CON1`, `L0-API2`, A3 as a cross stack (`A3^`, ruling R-W, the same form as `L-TGT`; rigour H2: A3 is in its build tree). **Design:** D 6.9 (D:1395-1433), row T1 (D:1713), section 14 rows 1 and 4 (D:2520, D:2523), open point 1; R5 (hot site 3).

**Files:** `crates/turts_sim/src/economy/gather.rs` (the approach point for edition 0.2.4; lead from the probe: `gather.rs:1126-1149`, `orders.rs:785-819`; verify first after A2b2; `find_unit` at `:1204` uses the index).

- [ ] Step 1 (early test, D:2523): the perf probe of perf H4 on the tree with A3; record the ratio in `docs/measurements/mp1.md`. Then red (today's tree: 0 harvester trips with a station 1 to 2 from the command centre), green, gate (F-RUST, F-FROZEN with the `base` stream unchanged, F-REF), plants OBS1 (1) to (3). Record the line count of the measure (open point 1: the estimate of 40 lines). Plant records.

**Rows:** OBS1. **Runs: 3 (2-4).**

### Task L-STO: start orders of a placement and of a unit type

**Seat:** `simulation-core-engineer` (S). **Review:** `network-engineer` (O). **Inputs:** `L0b-ORD`, `L-MATCH-SRV1`; merge after A2b2. **Design:** D 6.5 "Start orders" (D:713-735); X24; row STO1 (D:2462).

**Files:** `orders/` (the engine order at the start tick and at the spawn tick; `NearestNode` by distance, then node index); the spawn of today gives a gatherer with no node and stage `Idle` (`economy/transaction.rs:710-719`).

- [ ] Red, green, gate (F-RUST, F-FROZEN, F-REF), plants STO1 (a) to (e) on a test package (STO1 itself runs on `frontier` in `I-HEAD`). Slice rule: until A2c, the plants assert arrival at the node and the first trip, not operation counts. Plant records.

**Rows:** none owned (STO1/s1 is `I-HEAD`; its plants are here). **Runs: 2 (2-3).**

### Task L-FX-WIRE: fire, death and credit effects on the wire

**Seat:** `network-engineer` (S). **Review:** `bevy-rendering-engineer` (O). **Inputs:** `L-DMG`; merge after A2b2. **Design:** D 6.8 `Fired` and effects (D:1310-1311, D:1343-1346, D:1367-1373); R14 (erratum E4).

**Files:** `economy/combat.rs:393-402` (stage `Fired` for every viewer group; today the shooter's slot only); `export_effects.rs`; `crates/turts_client/src/ingest.rs` (no effect from a resync batch; drop effects older than the presentation tick minus 4). `Credited`: verify first the name of the delivery event (D:1150, NOT TRACED).

- [ ] Red: FX1 (a) headless (window B gets no enemy `Fired`). Green, gate (F-RUST, F-FROZEN, F-REF), plants FX1 (a), (d) (kept deltas replay their effects). Plant records.

**Rows:** none owned. **Runs: 2 (1-2).**

### Task L-FX-PRES: effect pool, tracer, flash, death, "+N", offline mark

**Seat:** `bevy-rendering-engineer` (S). **Review:** `performance-architect` (O). **Inputs:** `L0-EXIT`; merge after `L-FX-WIRE`. **Design:** D 8.3 (D:2148-2158), X14 stage 1, R14 (erratum E4).

**Files:** create `crates/turts_client/src/visuals/effects.rs` (one pool of 256; the oldest instance is reused); its call line in the split `draw_world_visuals` (`L0-CORE`); the visuals index built at install (presentation M4: today `ContentId::parse` runs for each unit in each frame at `visuals.rs:368`, `:446`, `:567`, `visuals/health.rs:74`, `visuals/world_tooltip.rs:28`; verify first after `L0-CORE`); the offline mark on an unpowered turret (one pooled instance, from the store getter of the supply record, ux L3); `crates/turts_client/src/debug.rs` (`turts_debug.effects.recent`, D:2450; own field function, presentation L1); the FX1 row files under `tests/browser/` with `mp1-fx1-oracle.test.js`.

- [ ] Red (headless pool counter; a headless counter of `ContentId::parse` calls for each frame at 400 units: today above 0, after the index 0), green, gate (F-RUST, F-WASM, F-DRV, F-HEAD), plants FX1 (b), the flash and death pixel plants, "offline mark missing", "`ContentId::parse` in a frame system". Plant records.

**Rows:** FX1/s1. **Runs: 4 (3-5). Headed: 1.**

### Task UI-PLACE: placement mode, ghost and rings

**Seat:** `bevy-rendering-engineer` (S). **Review:** `ux-designer` (O). **Inputs:** `L0-EXIT`; merge after `L-CON1`. **Design:** D 7.2 (D:1772-1816), D 6.1 "One pure function" (D:491-501); X5; rows CON2 (D:2441), GHO1 (D:2463).

**Files:** `crates/turts_interaction/src/placement.rs` (the effect `Ghost { content, point, valid, reason }`; the footprint list rebuilt per applied delta, not per frame); `crates/turts_client/src/visuals/placement.rs` (its call line in `draw_world_visuals`); `core_adapter/effects.rs` (the `Ghost` family function); `widgets/activation/place.rs` (`WidgetAction::Place`); `widgets/cursor.rs` (`place`, `forbidden`; and the clones at `cursor.rs:32-40` under the client cost rule, presentation M4); `scripts/boundary.sh` (CON2: no placement function in `turts_client` other than the call; the grep also covers charge, refund and progress arithmetic, which only `turts_rules` may hold, TD M3); GHO1 row files with `mp1-gho1-oracle.test.js`.

- [ ] Order inside the task (ux overrun note): `qa-lead` builds the GHO1 fixture (one point for each cause on a test map) first; the headless `Ghost` with CON2 is green by run 2; then the rings and the pixels.
- [ ] CON2 red, green, gate (F-RUST, F-WASM, F-DRV, F-HEAD, boundary), plants CON2 (1) to (3); GHO1 plants (a) to (e), all five red in the one pre-merge session through the headed wrapper (R-U). Plant records.

**Rows:** CON2, GHO1. **Runs: 6 (5-7). Headed: 1.**

### Task UI-HINT: hint engine and the start selection

**Seat:** `bevy-rendering-engineer` (S). **Review:** `ux-designer` (O). **Inputs:** `L0-EXIT`; merge after `UI-PLACE`. **Design:** D 7.8 (D:1984-2019), 7.1 times (D:1766-1770); row H14 `JobOnHold` with erratum E21; X25; open point 19 residual (closed by R-Y).

**Files:** `crates/turts_interaction/src/hints.rs` (evaluation once per applied delta plus timers; the selection rule; done through a pending site; `StepCurrent` reads steps only; `Selected`); `core_adapter/effects.rs` (the hint family function); `crates/turts_client/src/debug.rs` (`turts_debug.hints`: `click_ms`, `shown_ms`, `done_ms`, `site_ms`; own field function); `start_selection`.

- [ ] Red: GOAL1 plant (d) headless (the source evaluated only at the start: H2 never shows). Green. Gate (F-RUST, F-WASM). Plant (h) headless (`StepCurrent` through situation hints: more than 10 changes in 1 s). A counter test with a threshold (rigour L3): at most 1 hint evaluation for each applied delta, plus at most 4 timer evaluations each second; plant "evaluate in each frame": the counter is red. The X3 test (R-R item 4, ruling R-Y): on the overspend script S8 (C0-PACE `match.py` S8 inputs on a headless scenario), under income that pays about 1 tick in 12, hint X3 shows within 1 s after 10 s since `held_since_tick` of the held job (`JobOnHold(10)` reads `held`, E21); plant "X3 on the raw `WaitingForFunds`": X3 never shows (red). Plant records.

**Rows:** none owned (GOAL1 is `I-PIX1`). **Runs: 3 (2-4).**

### Task UI-HUD: HUD sources, row states, build menu keys, right-click cancel, the client cost rule

**Seat:** `bevy-rendering-engineer` (S). **Review:** `ux-designer` (O), with a half-way review after HUD5 (ux overrun note). **Inputs:** `L0-EXIT`; merge after `L-PAY`, `L-CON1`, `L-PWR`. **Design:** D 7.3 (D:1818-1844), 7.4 (D:1846-1873), 7.5 (D:1875-1913), 7.10 stage-1 lines (D:2074-2085), section 9 client cost rule (D:2261-2275); rows HUD5 (D:2451), HUD6 part 1 (D:2469), KEY1 (D:2452).

**Files:** the stub source files of `L0-CORE`, read through the `L0-STORE` getters (no storage of its own); `crates/turts_client/src/widgets/sources.rs` (build menu `:364`, `:392`: the production `hotkey`, else the Produce or `Construct` key; constructor buildings after producer recipes); `widgets/activation/cancel.rs` (the queue row and entry right-click cancel, open point 16); `widgets/tree.rs` (`sync_widget_tree` `:114`: no clones at `:136`, `:140`, `:144`, `:147`); `widgets/values.rs` (`:39`, `:65`, `:124`) and `widgets/tooltip.rs:97` (`set_if_neq`, presentation H2); the row state "On hold: no credits" reads `held` of the job record through the store getter (E21), so HUD5 (b) holds with no client timer; the hook `hud=off` (a test hook that hides the HUD root, for VIS3 and `PRES-MEASURE`; no hook exists today, grep `hud_off|no_hud|hide_hud` 0 hits).

- [ ] Run 1: probe point 16 (does `On<Activate>` fire on a right click; `activate_widget` at `widgets/activation.rs:43`; `MouseButton::Right` is read only in `input.rs:49` and the bindings editor). Then red: the HUD5 states in a scenario (headless where the text is a source value; the scenario forces "Credits 1200", so the start credits of R-R do not change HUD5); the changed counter (presentation H2): a headless client with a static replica for 60 frames, the count of widget entities with `Ref<Text|Node|Visibility>::is_changed()` is 0 after frame 2 (today red, `PRES-PROBE` probe 2). Green. Gate (F-RUST, F-WASM, F-DRV, F-HEAD). Plants HUD5 (a) to (c), HUD6 (b), (d), KEY1 (a), "`current.0 = text` back" (the counter is red). KEY1 (b), (c) are packer refusals (`L0-API2`). Plant records.

**Rows:** HUD5, HUD6/s1, KEY1. **Runs: 6 (5-8). Headed: 1.**

### Task UI-ALERT: alert kinds, triggers, the sound gap and the alert list

**Seat:** `bevy-rendering-engineer` (S). **Review:** `ux-designer` (O). **Inputs:** `L0-EXIT`, `UI-HUD` stacked (`UI-HUD^`: both write `sync_widget_tree`, ruling R-S); merge after `L-PWR`, `L-STO`, `L-PAY`, `L-CON1`; the MC PT1 repair is merged forward before it starts (section 1). **Design:** D 7.9 (D:2021-2070); rows ALR2 (D:2468), MCX1 alert-list part (D:2457; erratum E7).

**Files:** `crates/turts_client/src/feedback.rs` (state alerts with `clear_after`; `GathererIdle` from the idle gather stage; today `GathererWaiting` reads waiting orders only, `:815-825`; previous healths indexed by unit id, today a scan at `:618-632`; the clone at `:393` under the client cost rule, presentation M4; evaluation per applied delta; keep the PT1 rule "one alert for each wait"); `audio.rs` (gap by priority; the live cap of two stays, `:795`); `crates/turts_interaction/src/alerts.rs` (`FocusAlert`); `core_adapter/effects.rs` (the alert family function); the alert-list rows in `widgets/tree.rs` (read at `:188-210`; the row pitch equals the row height plus the gap within 1 px at UI scale 1.0 and 2.0, and no pixel of a row shows above the list, MCX1 text of D:2457).

- [ ] Red: ALR2 case A on today's rule (more than one sound in a window); the MCX1 alert-list pitch case on today's tree. Green. Gate (F-RUST, F-WASM, F-DRV, F-HEAD). Plants ALR2 (a) to (i), except the connection alerts (ALR2/s2); "two alerts for one wait" (the PT1 test is red); the MCX1 alert-list plants. Plant records.

**Rows:** ALR2/s1, MCX1/s1. **Runs: 5 (4-6). Headed: 1.**

### Task UI-RESULT: outcome and reason line

**Seat:** `bevy-rendering-engineer` (S). **Review:** `ux-designer` (O). **Inputs:** `L0-EXIT`, `L-LOBBY-C1` (a plain input: both edit `client_shell/notices.rs`; TD hidden pair). **Design:** D 7.6 result part (D:1949-1968), 6.7.7 step 5 (D:1031); X13 stage 1; row RES1 (D:2449).

**Files:** stub sources of `L0-CORE` for `Outcome`, `OutcomeReason`, `MatchClock`, `MatchOver`, `OutcomeIs`, `OutcomeReasonIs`, `{slot}` (through the `L0-STORE` getters); `widgets/activation/leave.rs` (the engine action `leave_match`: sends `LeaveMatch`, enabled only when the match is over); `core_adapter/effects.rs` (the result family function); the "Leave" shell text in `client_shell/notices.rs`; RES1/s1 row files with `mp1-res1-oracle.test.js`.

- [ ] Red, green, gate (F-RUST, F-WASM, F-DRV, F-HEAD). A test: when `MatchResult` arrives in a frame before the end `Snapshot`, no outcome shows until the replica tick is at or after `end_tick` (recheck5-network-host §3). Plants: the result screen without the reason (pixels, red in the lane); "outcome before `end_tick`". Plant records.

**Rows:** RES1/s1. **Runs: 3 (2-4). Headed: 1.**

### Task UI-LABEL: name labels over units (ruling R-E, engine request E3)

**Seat:** `bevy-rendering-engineer` (S). **Review:** `ux-designer` (O): can a person name each own unit type from one screenshot? **Inputs:** `L0-EXIT`. **Design:** errata E10, E22 (6.10 row H24 `world.unit_labels`), D 7.x world styles (row H11 pattern), the client cost rule (D:2261-2275).

**Files:** create `crates/turts_client/src/visuals/labels.rs` (one text label for each unit with a name, from `UnitDef.name`, `crates/turts_mod_api/src/schema.rs:337`; shown by the `show` option `Always` or `Selected` (no `Never`: `None` means no labels, E22), `own_only`; `colour`, `size` and `font` from the `LabelStyle` of E22; one entity per unit, written with `set_if_neq` only when the unit set or the text changes; the screen position from `world_to_viewport`); its call line in `draw_world_visuals`; remove the `world.unit_labels` line from `unbuilt.rs`; a label oracle `tests/browser/mp1-label-oracle.js` (role `oracle`) and `mp1-label-oracle.test.js`.

- [ ] Red: a headless test: with `world.unit_labels: Some((show: Always, own_only: false, colour: "#ffffff", font: None, size: 14))`, each unit entity has one label with its name; with `None`, none. Green. Gate (F-RUST, F-WASM, F-DRV, F-HEAD: in a pixels run the oracle finds a label box above each own unit's `ground_screen_center`). Plants: GOAL1 (i) "labels off" (the style ignored: the label oracle is red; the pixel plant of the pre-merge session); "a label written in each frame" (a changed counter on the label `Text` is red). Plant records.

**Rows:** a GOAL1 plant (section 3.6; erratum E10 adds the label check to GOAL1). **Runs: 2 (2-3). Headed: 1.**

### Task WEB-NOGPU: the no-GPU page (stage 1; rulings R-D, R-M)

**Seat:** `web-platform-engineer` (S). **Review:** `bevy-rendering-engineer` (O). **Inputs:** `L0-DBG` (both edit `turts_client_web/src/lib.rs`). **Design:** D 7.7 (D:1979-1982); row NOGPU1 (D:2458; moved to stage 1, erratum E8).

**Method (presentation M3).** With the `webgpu` feature the only backend is `BROWSER_WEBGPU` (`bevy_render-0.20.0/src/settings.rs:78-79`); Bevy panics at `renderer/mod.rs:308` and `panic = "abort"` (`Cargo.toml:43`) cannot catch it. So: in `crates/turts_client_web/src/lib.rs`, before `start()` builds the app (`:144`), read `navigator.gpu` through `js_sys::Reflect` (the pattern of `:181-183`) and await `requestAdapter()` (`wasm-bindgen-futures` is a dependency, `crates/turts_client_web/Cargo.toml:17`). If it is absent or null, put the D 7.7 text in `#wrap` and do not build the app.

- [ ] Red: NOGPU1 on today's build (Chrome with WebGPU off): a panic with a console message only (the MC build `8d6902c`, PT1 person run 1). Green. Gate (F-WASM, F-DRV, F-HEAD: the page text, and no panic in the console). Plant: no probe, the app builds and panics. Plant record.

**Rows:** NOGPU1. **Runs: 2 (1-3). Headed: 1.**

### Task C-UNITS: the `frontier` rules and map (stage 1)

**Seat:** `game-designer` (S). **Review:** `mod-builder` (O): can each field be written from 6.10 alone? **Inputs:** `L0-API2`, `C0-PACE`. **Design:** D section 5 (D:283-395) with erratum E13 (start credits 1,400), X15 stage 1 (D:2348), X24, X26; rulings R-E, R-R.

**Files:** `mods/frontier/` (`manifest.ron`, `units/`, `resources/`, `production/`, `orders/`, `combat/`, `maps/twin_fields.ron`, `match/match.ron` with `AllSeated(countdown_secs: 0)` and `KeyUnitsLost`). Verify the ore positions against the ring of radius 12.5 (recheck5-player :29: 14.1 to 15.1 from GD 224-226, NOT TRACED in D).

- **Start credits 1,400** (ruling R-R; `docs/research/mp1/c0-pace-stage1-pacing.md` section 4).
- **One distinct form and size class for each type (ruling R-E).** `Shape` has only `Capsule` and `Cuboid` (`schema.rs:48-51`), and the slot material (`render.rs:414`) separates own units from enemy units. The class table (from the presentation and ux reports; the numbers are content, Open, S:807):

| Type | Shape | Size class (radius x height) | Why it reads apart |
| --- | --- | --- | --- |
| Rifleman | Capsule | small: 0.3 x 0.9 | the shortest unit |
| Launcher | Capsule | small: 0.3 x 1.3 | taller than the Rifleman, same base |
| Engineer | Capsule | small-wide: 0.4 x 0.7 | short and wide |
| Ore Hauler | Cuboid | low wide vehicle: 0.9 x 0.6 | the only low wide box |
| Tank | Cuboid | vehicle: 0.9 x 1.0 | as wide as the hauler, twice as tall |
| Guard Turret | Cuboid | tall thin static: 0.6 x 2.0 | the only tall thin box |
| Power Station | Cuboid | large static: footprint x 1.5 | large and low |
| Command Centre | Cuboid | largest static: footprint x 2.5 | the largest and tallest |

- [ ] Write the package. **Gate (rigour M4):** pack a copy of `mods/frontier` with the fields of the lanes that have not merged left out; it prints `exit=0`. Then `cargo run -p turts_pack -- mods/frontier; echo exit=$?` prints exactly one `UNBUILT` refusal of a stage-1 lane that has not merged (or `exit=0` when all merged).
- [ ] **Plant (rigour M4, M2):** a copy with one misspelt field of a unit file (`footprnt` for `footprint`, `UnitDef.footprint` at `crates/turts_mod_api/src/schema.rs:338`) gives `exit=2` with the text `unknown field `footprnt`; did you mean `footprint`?` (the rule of `crates/turts_mod_runtime/src/parse.rs:19-30`, read at `d7d7e6d`; D:1450-1456). Reason for the choice (rigour M2 gave two options; the producer takes both halves): the K2 field `placement` is `UNBUILT` until `L-CON1` merges (offset 21, after the `C-UNITS` window 11-14), so a K2 plant here gives the `UNBUILT` refusal, not the K2 text; the "did you mean" rule exists today, so this plant can go red in the `C-UNITS` window. The K2 gap plant moves to `C1-FINAL`, where `L-CON1` has merged. Plant record.

**Rows:** none. **Runs: 3 (3-4).**

### Task C-UI: the `frontier` interface (stage 1)

**Seat:** `ux-designer` (S). **Review:** `player` (O). **Inputs:** `C-UNITS`. **Design:** D 7.2 to 7.10 texts, the Player path hint table (stage-1 data, no H3), 7.9 rules (D:2057-2070) without the two connection rules; X15, X25; the stage-1 hint order (C0-PACE: no change of the order).

**Files:** `mods/frontier/interface/`, `mods/frontier/input/`, `mods/frontier/assets/sounds/` and their `assets/assets.ron` entries (ux M2: one placeholder sound for each alert family of 7.9 and the GHO1 error sound, 6 sounds copied from `mods/forge`; D:2049, D:2348); `world.unit_labels: Some((show: Always, own_only: false, colour: "#ffffff", font: None, size: 14)),` (E10, E22: the form of the 6.10 row H24); `reasons.no_applicable_order: "The Command Centre cannot move. Select units, then right-click."` (ux M5; the key is shown through the notice at `crates/turts_client/src/feedback.rs:477-494` and is shared by every order that does not apply: `ux-designer` confirms the words in the review); create `crates/turts_pack/tests/frontier_texts.rs` (the pattern of `crates/turts_pack/tests/forge_guide.rs`). The `selection_priority` list (`crates/turts_mod_api/src/interface.rs:19`) puts the mobile units first, so a box with units leaves the Command Centre out (open point 20).

- [ ] **Step 1: Red (ux H3).** `frontier_texts.rs`: each hint, alert, reason, role and result text equals its D text, byte for byte (7.2, 7.6, 7.9, 7.10, the Player path); each step hint that needs the Command Centre contains "select your Command Centre"; each key letter in a hint equals the default binding of that action in `input/input.ron`; no text holds a raw id, "[]", "Slot 0", "x1 - HP" or a key list (first-60 check point 10). `cargo test -p turts_pack --locked --test frontier_texts; echo exit=$?` gives `exit=101` on the empty interface.
- [ ] **Step 2: Green.** Write the files.
- [ ] **Step 3: Gate.** F-PACK, the test of step 1.
- [ ] **Step 4: Plant.** Change one word of hint H1: the test is red. Restore. Plant record.

**Rows:** none. **Runs: 4 (3-5).**

### Task C1-FINAL: `frontier` packs; duels and the early-rush check recorded

**Seat:** `game-designer` (S). **Review:** `mod-builder` (O). **Inputs:** see the table (every stage-1 feature lane and A3). **Design:** X0e (D:244), D section 5 results; open point 4; R-R item 5.

- [ ] `turts-pack` on `mods/frontier` prints `exit=0`. **Plant (moved from `C-UNITS`, rigour M2):** a copy with `placement.gap` below the K2 minimum (1.8 + 0.2) gives `exit=2` with the K2 text of 6.10; plant record. A3 is merged: `git merge-base --is-ancestor pre-mp1/a3-grids mp1/integration; echo exit=$?` prints `exit=0`. The DMG1 duels with the `frontier` numbers run headless; the results go to `docs/measurements/mp1.md`; a failed ratio is a retune of S:807 numbers, recorded.
- [ ] **The early-rush check (R-R item 5).** (1) A constants check: a copy of `docs/research/mp1/probes/c0-pace/consts.py` reads the numbers from `mods/frontier` (not from D) and compares them with `pacing.py` `new` with start 1,400; 0 mismatches. (2) A headless scripted match on the built server with the S5 tank-rush inputs and the S1 ordinary script (`--start-now --command-script`): record the first-hold times and the hold lengths in the first 180 s, and the S5 result, next to the model values of the C0-PACE report. A hold above 4 s in S1 on the built content is a retune before C1 (recorded on the board).

**Rows:** none. **Runs: 3 (2-4).**

### Task I-HEAD: the headless two-client match, ORDW1, STO1

**Seat:** `qa-lead` (S). **Review:** `network-engineer` (O). **Inputs:** `C1-FINAL`, `L-LOBBY-C1`. **Design:** X20 stage 1; rows ORDW1 (D:2477), STO1 runs 1 to 3 (D:2462); section 14 row 7 (D:2527).

- [ ] ORDW1 cases A, B, C and the single `MoveTo` case in full (site, job and `Fired` at most 8 B); plant (5) of D:2477; the bytes of each record kind to `docs/measurements/mp1.md`. STO1 runs 1 to 3 on `frontier`. The scripted headless match to its end on the helper `tests/support/match.rs`: record the end time and the ore-out time. Plant record.

**Rows:** ORDW1, STO1/s1. **Runs: 3 (2-4).**

### Task I-PIX1: stage-1 pixels driver, GOAL1, batch B1

**Seat:** `qa-lead` (S). **Review:** `bevy-rendering-engineer` (O). **Inputs:** `C1-FINAL`, `L-LOBBY-C1`. **Design:** X20 stage 1 (D:2349), row GOAL1 (D:2461) with erratum E10 (labels), section 14 row 6 (D:2525).

**Files:** `tests/browser/mp1-goal1-*.js` (role lines; the driver reads no text and no bridge, QA0 A1; the oracle reads `turts_debug.hints`); `tests/browser/mp1-finders.js` (role `pure`; the shared unit finders, which count slot pixels inside `pick_radius`, ruling R-C; `pick_radius` comes from the footprint, `render.rs:50-60`); `tests/browser/rows/mp1-*.json`; `mp1-goal1-oracle.test.js`.

- [ ] GOAL1 red on the build without the hint engine, green, plants (a), (b), (c), (e), (f), (g), all six red on the real build in two pre-merge sessions of 3 plants each (plant (f) waits up to 300 s, so one session would pass the 20-minute bound; R-U). Batch B1 as a table run (wrapper mode `table`, one row for each call): each stage-1 pixel row (GHO1, HUD5, HUD6/s1, KEY1, ALR2/s1 pixel part, MCX1/s1, FX1/s1, RES1/s1, NOGPU1, GOAL1 with the label plant) green two times on the integrated tree, and one plant of each red; restore; green (`mp1_plants.py --files` of the rows). The hosted-runner probe is in `E-GPU` (qa M4). Plant records.

**Rows:** GOAL1. **Runs: 4 (3-5). Headed: 3** (two pre-merge sessions, one table run).

### Task I-PLAY1: the C1 evidence driver, `mp1-play.sh`, and the C1 instruction sheet (ruling R-D)

**Seat:** `qa-lead` (S). **Review:** `web-platform-engineer` (O): does the sheet start the CEO's browser as it did in MC, can the capture read it through the debugging port, and does the evidence set hold every item of 8.2? **Inputs:** `C1-FINAL`, `L-LOBBY-C1`. **Design:** D section 10 checkpoint C1 (D:2361-2366) with erratum E19; section 13 goals text (D:2488-2490); qa M3, ux H2, ux M6.

**Files:**
- Create `tests/browser/mp1-play-driver.js` (role `driver`): it plays both windows from "Click to play" to the result screen by the stage-1 Player path inputs, with the finders of `mp1-finders.js` (pixels only, QA0 A1 to A7), and destroys the other Command Centre.
- Create `tests/browser/mp1-play-evidence.js` (role `instrument`): it writes the evidence set of 8.2 for each window into `target/mp1-evidence/<run id>/`.
- Create `tests/browser/mp1-play-oracle.js` (role `oracle`) and `tests/browser/mp1-play-oracle.test.js` (qa N3). The oracle judges an evidence set and is red when an item of 8.2 is missing, a frame is all one colour, the first frame shows 0 own units (the finders of `mp1-finders.js`), a sound record comes before the first input, or `site_ms` is above 60,000. Plants: "units hidden" (the unit meshes not spawned: the first-frame unit count is 0, red); "audio record empty" (`turts_debug.audio` written as `[]` while the alerts show: red). The unit test runs the oracle on recorded sets (one green, one red for each plant, each the output of a real run) with no GPU.
- **How the runner loads the instrument** (qa N3): `scripts/mc-browser.sh` takes one driver and one oracle (`scripts/mc-browser.sh:15`, `:295-297`; it passes both to `tests/browser/harness/run.js`). The run is `scripts/mc-browser.sh mods/frontier tests/browser/mp1-play-driver.js tests/browser/mp1-play-oracle.js 1280x720 --windows 2`; the oracle `require`s `mp1-play-evidence.js`, which writes the set; then the oracle judges it. The driver never loads the instrument (QA0 rules A1 to A7 apply to the driver only).
- Create `tests/browser/mp1-capture.js` (role `instrument`): the capture for a person's Chrome (ruling R-V; `L0-DBG` states the form): it connects to `http://127.0.0.1:9222` with `playwright-core` `connectOverCDP`, reads the three debug records of each window, and writes them into the capture directory. The page posts nothing.
- Create `scripts/mp1-play.sh`: builds the release server and the web client, packs `frontier`, writes `target/mp1-play/server.ron` with `--order-log` and `--tick-log` for each match (`target/mp1-play/match-<n>/`), prints the URL and the Chrome command of the sheet, and runs `scripts/mp1-serve.sh`. It starts Chrome with the command of the sheet, which has `--remote-debugging-port=9222` (ruling R-V). A capture step `bash scripts/mp1-play.sh --capture <dir>` copies the logs, runs `mp1-capture.js` against the open windows, and then runs `mp1-play-oracle.js` on the captured set.
- Create `docs/research/mp1/c1-instructions.md` (the C1 instruction sheet, section 8.4).

- [ ] Red: the driver on the `C1-FINAL` tree with one plant of `UI-HINT` (H2 never shows): the evidence set shows the stop, and the driver run exits with the step number. Green: a full run to the result screen in both windows, under the lock (headed 2: one red, one green, each a pre-merge session). `I-PLAY1` runs the capture against a headed window: in the green session, `mp1-play.sh --capture` on the open windows writes a set that the oracle passes. Gate: F-DRV (with `node --test tests/browser/mp1-play-oracle.test.js`), `bash scripts/mp1-play.sh --dry-run; echo exit=$?`. Plants: the two oracle plants ("units hidden", "audio record empty") red in the green session. Plant records.

**Rows:** none. **Runs: 3 (2-4). Headed: 2.**

### Task REG2-1: regression at the end of stage 1

**Seat:** `qa-lead` (S). **Review:** `technical-director` (O). **Inputs:** `I-PIX1`, `I-HEAD`, `I-PLAY1`, `L0-API2`, `L0-DBG`. **Design:** row REG2 (D:2460), D:2754-2755.

- [ ] All MC rows, the fault sets, the frozen values, and each `done:` row of the Yardstick coverage with its test token (the qa-lead token script until R7b). `python3 -I scripts/qa/mp1_plants.py --all; echo exit=$?`: every MP1 plant red, restored and green on the merged tree. The red builds that show that REG2/s1 can go red: (1) the camera plant: `clamp_to_map` returns at once (`crates/turts_interaction/src/camera.rs:358`); `bounds_hold` (`crates/turts_interaction/tests/camera.rs:322`) is red (qa-lead reproduced it: green 0, red 101, restore `cmp` 0); (2) the SEL1 lead: shift the projection in `assertPhysicalClick` (`tests/browser/sel1-oracle.js:167`, verify first); (3) an MP1 red build (qa H2): CON1 plant (1) through `mp1_plants.py`. The pixel plant records (`headed: true`) run here as table runs (wrapper mode `table`, one row for each call, rule 2 of the lock queue). Then the C1 evidence run: the `I-PLAY1` driver on this commit writes the evidence set for `C1-PLAYER` (headed 1).

**Rows:** REG2/s1. **Runs: 3 (2-4). Headed: 3.**

### Task C1-PLAYER1, Task C1-PLAYER and Task C1-PERSON: checkpoint C1

Section 8.

### Stage-1 items that this plan judges not needed for C1

The plan keeps them in stage 1, as D says. Root or the CEO can move them to stage 2:

- TGT2 (retaliation and the hold rule): no `frontier` weapon outranges an acquire range (D:2476, D:2747). About 1 run of `L-TGT`.
- The restart part of LEAVE1/s1 (`mp1-serve.sh` loop, a new `match_id`): C1 is one match. About 1 run.
- LOB1 run 4 (reload in the lobby): the C1 instruction has no reload. Under 1 run.
- The exit-blocked case of PRD1: the loop does not need it. Under 1 run.

Moved by this revision (ruling R-Q): `I-NC1` (row NC1) to stage 2 (erratum E9): it is a source scan, and C1 loses nothing. `L-INC` stays (the pacing needs it, `L-INC` block). The three-slot MAT1 case is dropped (E1).

## 8. Checkpoint C1

### 8.1 Preconditions

On one commit of `mp1/integration`: every stage-1 row part of section 13 is green with its recorded red, and its plant records run red through `mp1_plants.py --all` (REG2-1); REG2/s1 is green; A3 is merged; the early-rush check of `C1-FINAL` is recorded; the `E-GPU` verdict is recorded, and the GPU machine of the person run has its numbers.

### 8.2 The evidence set (ruling R-D, qa M3)

The `I-PLAY1` driver writes it in `REG2-1` on the C1 commit; `mp1-play.sh --capture` writes the same set for the person run through the Chrome debugging port (ruling R-V). The oracle `mp1-play-oracle.js` checks each set before the `player` agent or the person signs off (qa N3). For each window:

1. The frames: the first presented frame after "Click to play"; one frame for each hint change; the frame of the first own site (the first client update with an own Power Station site, 7.1); the frame of the first "+N"; one frame each 10 s for the first 180 s; the frame of each alert line; the result screen.
2. The records: `turts_debug.hints` (`click_ms`, `shown_ms`, `done_ms`, `site_ms`), `turts_debug.audio` (the text record of each sound start: the `player` agent cannot hear), `turts_debug.effects.recent`.
3. The timings: the time of each step of the Player path from "Click to play", the match length, `site_ms`.
4. The server logs of the match: the `--order-log` and the `--tick-log` (`target/mp1-play/match-<n>/`).
5. The machine, the GPU, the browser version and the Chrome command used.

### 8.3 The first-60-seconds check list (ux M6, ruling R-D)

The text of the ux-designer (`gate2-ux-designer.md:48`), checked against the list of `input-mc-pt1-engine-requests.md:22-35`. An observer checks it from the screen (person run) and the `player` agent from the evidence set (agent run). Any miss is a FAIL, routed by the table of 8.6.

1. The person says the goal within 5 s.
2. The person points at the own Command Centre and at the Ore Hauler, and names each own unit type that is on screen (labels, R-E).
3. The person names the first action from the hint.
4. No alert line and no sound before the first input; in the first 60 s, no alert sound plays more than 3 times, and no sound comes again with no new event.
5. P, or a click on the entry, shows the ghost and the ring.
6. A red ghost shows a reason that the person can repeat.
7. The click shows the site and a new hint in the same frame.
8. "+300" and the credit change show by 40 s.
9. The person says the next action, and after 60 s says the goal, the first action and the win condition.
10. No raw id, "[]", "Slot 0", "x1 - HP" or key list is on screen.
11. A click on an alert line shows the thing that the alert names.
12. Each own unit differs from each enemy unit by colour and shape on the first frame; the person names the side of 3 units.

Check against the input list (`input-mc-pt1-engine-requests.md:26-35`): input 1 is points 1 and 3; input 2 is point 3 and the hint line (always on screen, no menu); input 3 is points 5 to 7; input 4 is point 12; input 5 is point 4; input 6 is point 11; input 7 is point 6 for placement only: in MP1 a refused order other than placement shows its reason in the notice line, not at the pointer (engine request E7 rest, sent to MP2, section 17); input 8 is point 8; input 9 is point 9; input 10 does not apply in MP1: the D hint texts name keys as plain letters (D:2990-2997), so the check is that each named key equals its default binding (`C-UI` test); a rebind-safe `{key:action}` placeholder is recorded for M8 (ux L2).

### 8.4 The C1 instruction sheet (a task output of `I-PLAY1`; ux H2, ruling R-D)

`docs/research/mp1/c1-instructions.md` holds, word for word:

1. Start: `bash scripts/mp1-play.sh; echo exit=$?` on the play machine. It prints `http://127.0.0.1:<port>/`.
2. The browser, the form that the CEO used with success, with the debugging port of the capture (ruling R-V): `google-chrome --user-data-dir=/tmp/turts-play-profile --remote-debugging-port=9222 --enable-unsafe-webgpu --enable-features=Vulkan --use-angle=vulkan "<URL>"`. Open a second window of the same profile on the same URL, side by side.
3. What to expect: each window shows "Waiting for the other player" until both are seated, then "Click to play". Do not reload a window before both show "Click to play" (recheck5-network-host F1). After the click: the own Command Centre is selected, the top line says "Destroy the enemy Command Centre", the hint line shows the first hint, and the Ore Hauler drives to the ore by itself. If a window shows the no-GPU text of 7.7, close it and start Chrome again with the command of item 2.
4. The instruction, word for word (D:2361-2363): "Use one window to destroy the Command Centre of the other window. If no Command Centre falls after 40 minutes, stop: write 'no end' and the time in the vault note, close both windows, and stop the server with Ctrl+C."
5. At the end: `bash scripts/mp1-play.sh --capture target/mp1-evidence/c1-person; echo exit=$?`.

### 8.5 The steps

**Step 1: the first-minute check (`C1-PLAYER1`, `player`, Opus; ux M4).** Input: the evidence set of the `I-PLAY1` green run (before `REG2-1`). The agent judges the first 60 s against 8.3 and answers "from one screenshot, name each own unit type" (ruling R-E). PASS or FAIL with the routing table. A FAIL is repaired before `REG2-1` ends.

**Step 2: the blind check (`C1-PLAYER`, `player`, Opus).** Input: the goals text of D section 13, match 1 (D:2488-2490), the C1 instruction sheet, and the evidence set that `REG2-1` writes on the C1 commit. The agent does not drive a browser (ruling R-D). It reports each stop with its time and step. PASS: the driver reached the result screen in both windows, 8.3 holds, and no stop is a "cannot find an object, an action or a reason". FAIL: the routing table. Root does not start Step 3 on a FAIL.

**Step 3: the person (`C1-PERSON`, procedure owner `qa-lead`).** The person follows the sheet of 8.4. The observer (or the person) checks 8.3. The person signs off C1 in the vault (a note under `vault/Decisions/`, with the commit, the date, PASS or FAIL, the words of each stop, the machine, the match length and `site_ms` from the capture). "No end" after 40 minutes is a known limit (D:2363-2364), not a FAIL.

### 8.6 What a FAIL sends back

| Symptom | Task |
| --- | --- |
| No "Waiting for the other player", no "Click to play" in both windows, a new id gets a seat after the start | `L-MATCH-SRV1`; the client phase: `L-LOBBY-C1` |
| The no-GPU text, or a blank page with the Chrome command of the sheet | `WEB-NOGPU`; the sheet: `I-PLAY1` |
| The Command Centre is not selected, the goal line or a hint is wrong or missing | `UI-HINT`; the texts and the order: `C-UI` |
| A unit type cannot be named, or a label is missing | `UI-LABEL`; the shapes and sizes: `C-UNITS` |
| The ghost, a ring, or a reason text is wrong | `UI-PLACE`; a server refusal that differs: `L-CON1`; the words: `C-UI` |
| The Ore Hauler does not gather from the start, or a new one stays idle | `L-STO`; no trips near a building: `L-OBS`; the data: `C-UNITS` |
| Credits, power or row states are wrong | `UI-HUD`; the values: `L-PAY`, `L-PWR`, `L0-STORE` |
| A hold with no hint X3, or a hold that is too long | `UI-HINT`, `L-PAY` (the held flag, E21); the numbers: `C-UNITS` |
| An alert repeats, sounds at tick 0, the wrong alert shows, or the alert list is clipped | `UI-ALERT`; the rules: `C-UI` |
| Units do not fire, do not acquire, or the hold rule fails | `L-TGT`, `L-DMG` |
| No tracer, flash, death effect, "+N" or offline mark | `L-FX-PRES`; the event is missing: `L-FX-WIRE` |
| The Command Centre dies and no outcome, or a wrong reason line | `L-MATCH-SIM`, `L-MATCH-SRV1`, `UI-RESULT` |
| The driver stops at a step that a person passes | `I-PLAY1` (the finders); `I-PIX1` (`mp1-finders.js`) |
| The frame rate is too low to play | needs decision: not a stage-1 task (D-N is a PT3 condition); Root asks `performance-architect` and `bevy-rendering-engineer` |
| A panic or a crash | the last task that touched the crate; Root triages |

After a repair, the repaired task's rows and REG2/s1 run again, then Step 2, then Step 3.

## 9. Stage 2 tasks

Stage 2 starts only after C1 passes (D:2366). Every stage-2 task descends from `C1-PERSON` through inputs (the script checks it). The code tasks take `L0-EXIT2` as input: they start from the frozen stage-2 shapes. Code lines here are leads; every builder verifies first, because stage 1 moves them. Gate sets: every task runs F-RUST and F-WASM for the crates it touches, F-FROZEN when it touches `turts_sim`, `turts_pack` or `turts_server` (qa L1), F-REF (MP1 form), F-PROC for its process tests, F-DRV and F-HEAD for its pixel rows, and writes its plant record file.

### Task L0-EXIT2: the second freeze (stage-2 shapes; ruling R-J)

**Seat:** `technical-director` (O). **Review:** `mod-builder` (O). **Inputs:** `C1-PERSON`. Read the C1 sign-off and the C1 findings. Change the stage-2 shapes of 5.1 item 7 that the findings need (one commit each, by the owner seat of 5.2 item 3), then freeze them: `docs/research/mp1/lane0-freeze.md` gains the stage-2 list and the commit. Plant: delete one stage-2 line of section 5.3 from `unbuilt.rs`: the freeze script is red. **Runs: 1 (1-2).**

| Task | Seat (tier) / review | What (D reference) | Rows; plants (D line) | Runs |
| --- | --- | --- | --- | --- |
| `I-NC1` | qa-lead (S) / mod-runtime-engineer | the no-content scan for 0.2.4 (moved to stage 2, erratum E9): `crates/turts_checks/tests/support/mod.rs` (kinds `:86-100`, `add_id` `:449-456`, exception table `:272-292`; verify first), `no_content.rs:166`; red: the literal `"muster_unit_damaged"` gives 0 hits today (D:2436); decision N1 | NC1; plants: one literal for each new kind | 2 |
| `L-CON3` | sim (S) / qa-lead | `CancelConstruction`, refunds, held-order cancel, open-site count (D 6.1 :503-505, 6.4 :648-656) | CON3; plants (1) to (3) (:2442) | 2 |
| `L-REP` | sim (S) / qa-lead | `repairer`, `Repair`, charge by applied health, `auto_range` with the grid cadence (D 6.5 :685-702); static health-writer check on the syn scanner | REP1; plants (1) to (3) (:2445); "direct health write" | 3 |
| `L-RLY` | sim (S) / network-engineer | `SetRally`, `UnitOrderRecord.rally`, follow order as an engine order through `set_unit_orders`, `rules.ron rally`, `ground_rally` (D 6.5 :704-735). Step 1 traces whether the order queue accepts a queued `Gather` (D:731-732) | RLY1 (A) to (C); RLY1 plants (1) to (4) (:2446); STO1/s2 run 4 and STO1 plant (f) | 3 |
| `L-MATCH-SIM2` | sim (S) / backend-engineer | `SlotConnection` inputs, `DisconnectedFor` in ticks, `Surrender`, statistics counts (D 6.7.8, 6.7.9); verify first whether the owner of `by` is known when `Destroyed` is written (D:1149) | none | 3 |
| `L-LOBBY2-SRV` | backend (S) / network-engineer | `AllSeatedReady`, Countdown (R11), `LobbyAction`, `LobbyRefused`, ready and colour rules (D 6.7.5 :913-965, 6.7.6); process test `lobby2.rs` on `tests/support/match.rs`, its `acceptance.sh` line | LEAVE1/s2 (:2475); plants: a world reset in place, the old lobby seats after the restart | 3 |
| `L-MATCH-SRV2` | backend (S) / network-engineer | room writes `SlotConnection`; `connection` script kind; `all_disconnected`; the `to_sim` exception for an empty `Surrender` (`crates/turts_server/src/orders.rs:34-40`); stage-2 `standoff` (D 6.7.8 :1102-1110, 6.7.12); the headless client action `surrender` in `actions.rs` (qa M2: MAT1 (e), (h) need it, D:1231); `mat1.rs` stage-2 cases | MAT1/s2; plants (c) to (k) (:2447) | 3 |
| `L-LOBBY2-UI` | bevy (S) / ux-designer | lobby screen of 7.6; hooks `ready`, `unready`, `set_colour`, `lobby_move`, `stale_revision`; the report field `lobby_refusals` (qa M2, D:1247); colours from `MatchStart.colours`. The lobby goal line: **needs decision** (ux L1, Q12). Early test (D:2529): the lobby stub of two clients is green within 4 runs, else report | LOB1/s2 runs 1-3, LOB2 (:2448, :2472); plants: LOB1 a start with one ready, `unready` does not stop the countdown, the revision check off; LOB2 the slot colour from the palette by slot index (pixels), the colour not checked; 4 pixel plants | 4, headed 1 |
| `UI2-RESULT` | bevy (S) / ux-designer (joint review with `UI2-CONN`) | statistics on the result screen; the report field `stats_first_seen_tick` (qa M2, D:1248); merge after `L-MATCH-SRV2` | RES1/s2 (stats, wire check, `Gathered`) (:2449); plants: the statistics at each tick, a statistic in `MatchState`, the free income in `Gathered`; 1 pixel plant (the statistics rows) | 2, headed 1 |
| `UI2-CONN` | bevy (S) / joint review in `UI2-RESULT` | surrender item and dialog (shell), connection banner, `SlotDisconnected`, `SlotReconnected` alerts | SUR1 (:2473), ALR2/s2; plants: SUR1 no confirmation, no result in the other window, the outcome before the end tick; ALR2 (j) "`SlotDisconnected` not wired: a killed client gives no lost-connection line" (rigour M2, erratum E5); 4 pixel plants | 2, headed 1 |
| `UI2-MC` | bevy (S) / ux-designer | X18 items except the no-GPU page and the alert-list part (both stage 1) (D 7.7) | MCX1/s2 (:2457); plants: each MC defect put back, without the alert-list part (the count is the O15 list, NOT TRACED) | 3, headed 1 |
| `UI2-HUD` | bevy (S) / ux-designer | HUD6 part 2, cancel-construction controls, rally flag and line (`widgets/activation/rally.rs`), and the smart rules and cursors `rally`, `repair` (the former `UI2-SMART`, folded: qa L1); input `UI2-MC` (both touch `spawn_widget` and `sources.rs`, ruling R-S) | HUD6/s2; plants HUD6 (a), (c) (:2469); SMT1; plants SMT1 (a) to (c) (:2470); 5 pixel plants | 4, headed 1 |
| `PRES-MEASURE` | bevy (S) / performance-architect | 8.5 first task, **on the C1 tree before any model exists** (presentation H2): frame time at 400 units with the HUD on and off (hook `hud=off` of `UI-HUD`) and the debug state on and off; primitives against placeholder models at the D-M caps, made by the GLB script of `PRES-ART` with a subdivision factor and an embedded PNG (presentation asset proposal); wasm memory high-water (open point 13); the VIS3 hook `frame_stall_ms` and its two plants | VIS3 plants (1) `frame_stall_ms=60` on every frame; (2) on 1 frame in 10; 2 pixel plants | 2, headed 2 |
| `PRES-MODELS` | bevy (S) / mod-runtime-engineer | V1, V2 checks with `gltf = "=1.4.1"`, `default-features = false, features = ["names", "utils"]` (D 8.1; presentation M1); loader from package bytes: a memory asset source registered before `DefaultPlugins` in `crates/turts_client/src/lib.rs:298` (`bevy_asset-0.20.0/src/lib.rs:576-578`, `:628-630`); the spawn path `render.rs:356-434` (`sync_units`, the slot material at `:414`); the shape catalogue `content.rs:44`, `acquire.rs:146-148`, `turts_mod_runtime/src/visuals.rs:10` (`visuals_by_content`); the web bridge `turts_client_web/src/lib.rs:392` (`publish_scene` keeps only `With<Mesh3d>`); the model root is `WorldAssetRoot` (0.20; `SceneRoot` is 0.19), the material is the `Material{n}/std` labelled asset (`bevy_pbr-0.20.0/src/gltf.rs:126-138`), not `GltfPrimitive.material`; shared mesh and material handles for each model, one slot material for each slot, no material for each unit; flat template, team material, shape fallback, the decoded-image cap (perf N3); the site model rises from the ground with its progress (D 8.2). **Rule (presentation H3):** a unit with no `model` keeps the same entity layout, so the MC rows stay green (D:2113) | MDL1 8.1 check plants: MDL1 (b) for each check (pivot, scale, materials, nodes, bytes, images) | 5 |
| `FX2` | bevy (S) / performance-architect | other effects of 8.3, `hit_reduced`, the sound engine part (8.4; the positional gain as `Volume::Linear`, not `SpatialAudioSink`, presentation L2; the rodio fall-off is NOT TRACED); **D 8.2 (presentation H1):** in `render.rs` `sync_units`, the yaw from movement with a turn rate, a turret aims from `Fired` on `turret_node` (else the whole turret shape turns), the infantry bob; merge after `PRES-MODELS` | FX1/s2 (:2450); plants FX1 (c) `hit_reduced` never chosen, "turret does not turn"; 2 pixel plants | 4, headed 1 |
| `C2-SOUND` | game-designer (S) / player | the P5 sound set; the sound assets are made by a deterministic script or copied from `forge` (NOT TRACED which); merge after `FX2` | AUD2 (:2471); plants AUD2 (a), (b), (d) in the browser, (c) a packer refusal; 3 pixel plants | 2, headed 1 |
| `PRES-ART` | bevy (S) / ux-designer | 9 models within the caps, written by a deterministic Python 3 script in the repository, `scripts/mp1_models.py` (standard library only; the probe `docs/research/mp1/probes/gate2-glb/make_tank.py` built one tank: 72 triangles, 6,628 bytes, materials `body` and `team`, nodes `turret` and `muzzle`, no textures; the pivot plant went red with "min_y=-0.400"); `cmp` of two runs of the script; merge after `PRES-MODELS` | MDL1 (:2455); browser plants MDL1 (a) a `.glb` URL fetch, (c) the shape removed before the model is ready, (d) the team material not replaced; 3 pixel plants | 4, headed 1 |
| `C2-CONTENT` | game-designer (S) / mod-builder | guide, tuned numbers, stage-2 match file (`AllSeatedReady(3)`, `DisconnectedFor(120)`, `Surrender`), hint H3, rally rules; **owns `mods/frontier/interface/` in stage 2** (ux M7): the HUD6/s2 layout, the RES1/s2 statistics rows, the connection banner; merge after `L-MATCH-SRV2`, `UI2-RESULT`, `UI2-CONN`, `UI2-HUD`, `UI2-MC` (TD Low, ux M7); `frontier_texts.rs` gains the stage-2 texts | none | 3 |
| `I2-L` | web-platform (S) / backend-engineer | open point 10: the browser L for a reload and a tab close; step 19 breaks if L is above 13 s (D:1100-1101) | none | 1, headed 1 |
| `I2-PIX` | qa-lead (S) / bevy | the pixels-only two-player driver of the full path (it writes the PT3 evidence set of 8.2 on the stage-2 tree); batch B2 of all stage-2 pixel rows; **every stage-1 MP1 pixel row and one plant of each again, with models on** (ruling R-C, qa H3); a finder change is a re-calibration commit of `mp1-finders.js` (count slot pixels inside `pick_radius`) that `bevy-rendering-engineer` reviews with the plant red before and after; the PT3 timing script with plants (1) to (3) (:2459). The reviewer runs one PT3-script plant and one registered plant (qa L1) | PT3 timing plants | 5, headed 5 |

## 10. Regression, play test, measurements, records

### Task MEAS-EXIT: the numbers of exit item 5

**Seat:** `qa-lead` (S). **Review:** `performance-architect` (O). **Inputs:** `I2-PIX`. **Design:** D:80-90 with erratum E3, D 8.5, row VIS3 (D:2456).

- Server tick p50 and p99 over 200 or more ticks: the MP1 mixed scene at 2 x 200 units, 1,000 units and 5,000 units with 2 slots (`--synthetic-units`), and the idle tick (`turts_server --stats`). The 4-slot and 8-slot scenes are recorded as "not measured in MP1": MP1 has 2 slots (`SLOT_COUNT` 2); they belong to the hosting benchmark milestones (D57, D58) (ruling R-F, erratum E3).
- Postcard bytes per tick per client by record kind, and the export time.
- Native `apply_delta` p50 and p99 (R13), and the store apply of `L0-STORE`.
- Browser frame p50 and p95 and the wasm memory high-water after 10 minutes at 400 units, debug state off and on, HUD on and off (hook `hud=off`) (`node tests/browser/measure.js`, the M1 commands in `docs/measurements/m1.md`).
- The boot-to-ready time of the server with `frontier` (D:88-89).
- VIS3: the D-N floor on the play-test machine, with models on and the debug state off; plants (1) `frame_stall_ms=60` on every frame and (2) on 1 frame in 10 (one pre-merge session).
- Each number with the machine, the GPU, the image and the package versions (S 13.2). Recorded, not judged (S 15). A VM is optional; GCP access is pending (D59), so the fallback is the local machine; with a VM, use the `default` gcloud configuration and delete the VM after the run.

**Rows:** VIS3. **Runs: 2 (2-3). Headed: 2.**

### Task REG2-2: the final regression

**Seat:** `qa-lead` (S). **Review:** `technical-director` (O). **Inputs:** `I2-PIX`, `I-NC1`. As `REG2-1`, on the stage-2 tree: all MC rows and all MP1 stage-1 rows (qa H3), `mp1_plants.py --all` (an MP1 red build named: CON1 plant (1)), and the hosted CI in the form of D:76-79. Then the PT3 evidence run: the `I2-PIX` driver on this commit writes the evidence set for `PT3-PLAYER` (headed 1). **Rows:** REG2/s2. **Runs: 3 (2-4). Headed: 3.**

### Task AUDIT: architecture audit

**Seat:** `technical-director` (O). **Inputs:** `REG2-2`. The audit of the whole MP1 change (exit item 3, X23), with the refused list of D:2288-2291. **Runs: 2.**

### Task PT3-PLAYER and Task PT3-PEOPLE: the play test

- `PT3-PLAYER` (Opus): the `player` agent judges the evidence set that `REG2-2` writes (8.2 items, on the full build), with the goals text of match 1 (D:2488-2490) and the check list of 8.3. It does not drive a browser (ruling R-D). PASS before people.
- `PT3-PEOPLE` (procedure owner `qa-lead`): start conditions: `MEAS-EXIT` shows the D-N floor on the play-test machine; `PT3-PLAYER` passed.
    1. Start: the C1 instruction sheet (8.4), with the stage-2 build. Two people, one not the CEO (P7). The observer has the Player path; the testers do not (D:2484).
    2. Match 1: both testers get the goals text of D:2488-2490. The observer checks 8.3 for each tester.
    3. Match 2 and match 3: the texts of D:2494-2500. Each match is a new server process; between matches the testers reload the page.
    4. Record for each tester: the capture of `mp1-play.sh --capture` (8.2), each spoken stop with its time and step, the result screen, and the answer to "Would you play a second match?" (D:2511-2512).
    5. The PT3 timing script, with its plants (1) to (3) (D:2459) run red on the capture of the people, judges the 60 s rule from `site_ms` against the server order log of the match.
    6. One person signs off in the vault. **Rows:** PT3. A FAIL goes to the owner of the step (Player path step to row to task, section 13).

### Task REC: records

**Seat:** `qa-lead` (H) for `docs/measurements/mp1.md` (one summary at the top). **Closing (qa L1, ruling R-O):** `qa-lead` writes the change of the MP1 row of `docs/qa/specs.tsv` from `gated` to `done` (present value `gated` in `/home/julien/Projects/TuRTS-mp1/docs/qa/specs.tsv`; `draft` in the slice worktree) and runs `python3 -I scripts/qa/completeness.py --enforce`; Root turns each `in` row of the Yardstick coverage section of D into `done:` with its test token. No task needs a new row in `docs/qa/yardstick-a.tsv` (qa L1). Root, not a builder, writes:
- `CLAUDE.md`: protocol 5 and API 0.2.4; the crate `turts_rules` and its row; `room.rs`; `replica_store.rs`; the new options `--start-now`, `--input-log`, `--rejoin`, the `--actions` kinds and the `server.ron` fields; `scripts/mp1-serve.sh`, `scripts/mp1-play.sh`, `scripts/qa/mp1_plants.py`, `scripts/mp1_models.py`; the packages `frontier` and `standoff`; `docs/measurements/mp1.md`; the name `visuals_by_content` in place of `visuals_by_unit` (presentation L3); `economy_ui.rs` does not exist in the client tree (ux L4).
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
| 13 | PRES-MEASURE | 2 | measured on the C1 tree before any model exists |
| 14 | L0-WIRE | 0 | the replica test; if red, lane 0 makes the replica accept the Snapshot |
| 15 | MP2 | - | recorded for the MP2 design |
| 16 | UI-HUD | 1 | read at `03aef38`: the cancel is a separate `Button` with `action: Item(CancelAction)` (`mods/forge/interface/hud.ron:60`), fired by `On<Activate>` in `activate_widget` (`crates/turts_client/src/widgets/activation.rs:43`); `MouseButton::Right` is read only in `input.rs:49` and the bindings editor (`client_shell/bindings_editor.rs:291`). `UI-HUD` run 1 probes whether `Activate` fires on a right click; `ux-designer` confirmed the lead at gate 2 |
| 17 | closed | - | closed in D revision 6 |
| 18 | L-MATCH-SRV2 | 2 | A1c deletes all of `crates/turts_audit/src/replay.rs` (SP Task A1c), which holds the `EmptyCommand` rule. `L-MATCH-SRV2` starts after A1c, so only the `to_sim` exception is needed. Whether the A1b ledger validator has an `EmptyCommand` rule is NOT TRACED; `L-MATCH-SRV2` step 1 checks it |
| 19 | C0-PACE, L-PAY, UI-HINT | 0, 1 | closed by `C0-PACE` (done): start credits 1,400 (R-R, erratum E13). The residual (Q11, how `JobOnHold(10)` counts a hold that flickers) is closed by ruling R-Y (erratum E21): `held` and `held_since_tick` on the job record; `L-PAY` builds them with the PAY1 case and plant (6); `UI-HINT` tests hint X3 on the overspend script S8 |
| 20 | C-UI | 1 | read at `03aef38`: `box_select` (`crates/turts_interaction/src/selection.rs:115`) keeps one class, mobile or static, by the lowest `selection_rank` (`world_view.rs:53`); `frontier` lists mobile units first in `selection_priority`; `ux-designer` confirmed at gate 2 |
| 21 | L-ROOM0 | 0 | the trace of step 1, before `L-MATCH-SRV1` builds the seat release |
<!-- mp1-open:end -->

## 12. Binding notes from the last reviews

| Source | Note | Task |
| --- | --- | --- |
| recheck6-rigour (a), Low | GOAL1 plant (g) second clause | fixed in D revision 6 (D:2461 has no driver clause); `I-PIX1` uses D:2461 |
| recheck6-rigour (b), Medium | STO1 run 4 is stage 2 | split STO1/s1 (`I-HEAD`), STO1/s2 (`L-RLY`); Q1, erratum E5 |
| recheck6-rigour (d) | trace point 21 before the seat release | `L-ROOM0` step 1; `L-MATCH-SRV1` input |
| recheck5-player N-L2, Low | pacing of the stage-1 order | `C0-PACE` (done; R-R) |
| recheck5-player, not traced | the box drag picks up the Command Centre | `C-UI` (point 20) |
| recheck5-player, not checked | ore positions | `C-UNITS` checks them against the ring |
| recheck5-player STOP 1, N-L1, N-L3 | H5 and X4 texts | fixed in D (Player path); `C-UI` writes them; `frontier_texts.rs` checks them |
| recheck5-network-host F1 | reload before the other seat fills | the C1 instruction sheet (8.4); LOB1 run 4 in `L-LOBBY-C1` |
| recheck5-network-host §3 | `MatchResult` before the end `Snapshot` in one frame | `UI-RESULT` test |
| recheck5-network-host §5 | no test depends on refusals that skip the bucket | `L0-WIRE` step 3 |
| recheck5-network-host point 14 | the replica accepts an unrequested Snapshot | `L0-WIRE` step 1 test |
| regate-performance N1, N5 | per-frame HUD work, quadratic alert | `UI-HUD` changed counter (a red now, presentation H2); `UI-ALERT` (health index); `MEAS-EXIT` and `PRES-MEASURE` HUD on and off |
| regate-performance N2 | on-change records | ORDW1 case C in `I-HEAD` |
| regate-performance N3 | decoded images | `PRES-MODELS` |
| regate-performance N4 | `health_trace` window | `L0-DBG` |
| regate-performance N6 | VIS3 plant on 1 frame in 10 | `PRES-MEASURE`, `MEAS-EXIT` |
| regate-performance N7 | grid cell | `L-TGT` |
| regate-performance M2 | replica cost | point 12, R13; `L0-STORE` adds no clone |
| regate-performance M4 | string ids: no instrument | `L-DMG` source test; `L0-SIM` install indices; `L-FX-PRES` frame counter |
| regate-performance M5 | records from writers: no instrument | `L0-WIRE` scan: the export writers read only `CommittedTick` (rigour L3) |
| regate-performance, cost table | hint evaluation per frame | `UI-HINT` counter test with a threshold (rigour L3) |
| regate-performance, wire | bytes by record kind have no red | recorded only (`I-HEAD`, `MEAS-EXIT`) |

## 13. Acceptance rows and their owners

### 13.1 Owners

Read from the task table (Rows column). 52 row parts; each has one owner (script, section 3.5).

- Lane 0: DBG1 (`L0-DBG`), VAL1 (`L0-API2`).
- Stage 1: IMM1, MAT1/s1 (`L-MATCH-SRV1`); LOB1/s1, LEAVE1/s1 (`L-LOBBY-C1`); PAY1, PAY2 (`L-PAY`); INC1 (`L-INC`); DMG1 (`L-DMG`); TGT1, TGT2 (`L-TGT`); CON1, PRD1 (`L-CON1`); PWR1 (`L-PWR`); DES1 (`L-DES`); OBS1 (`L-OBS`); FX1/s1 (`L-FX-PRES`); CON2, GHO1 (`UI-PLACE`); HUD5, HUD6/s1, KEY1 (`UI-HUD`); ALR2/s1, MCX1/s1 (`UI-ALERT`); RES1/s1 (`UI-RESULT`); NOGPU1 (`WEB-NOGPU`); ORDW1, STO1/s1 (`I-HEAD`); GOAL1 (`I-PIX1`); REG2/s1 (`REG2-1`).
- Stage 2: NC1 (`I-NC1`); CON3 (`L-CON3`); REP1 (`L-REP`); RLY1, STO1/s2 (`L-RLY`); MAT1/s2 (`L-MATCH-SRV2`); LEAVE1/s2 (`L-LOBBY2-SRV`); LOB1/s2, LOB2 (`L-LOBBY2-UI`); RES1/s2 (`UI2-RESULT`); SUR1, ALR2/s2 (`UI2-CONN`); HUD6/s2, SMT1 (`UI2-HUD`); FX1/s2 (`FX2`); AUD2 (`C2-SOUND`); MDL1 (`PRES-ART`); MCX1/s2 (`UI2-MC`); VIS3 (`MEAS-EXIT`); REG2/s2 (`REG2-2`); PT3 (`PT3-PEOPLE`).

### 13.2 Row splits and moves from planning

The splits and the moves are the blocks `mp1-splits` and `mp1-moves` of section 3.6. Each move and the MCX1 and ALR2 splits change a D stage list: errata E5, E7, E8, E9. Until Root applies them, the plan stages win for these rows only (header).

## 14. Budget

### 14.1 Runs

From the task table (script output, section 20). D section 11 for comparison. All counts are runs from now; `C0-PACE` (1 run) and gate 2 (5 runs) are spent.

| Part | This plan: point (range) | Revision 2 | D section 11 |
| --- | --- | --- | --- |
| Lane 0 (18 tasks, `C0-PACE` done) | 40 (32-55) | 39 (31-53) | inside stage 1 |
| Stage 1 (31 tasks) | 97 (76-130) | 97 (76-130) | |
| Lane 0 + stage 1 | **137 (108-185)** | 136 (107-183) | 62 to 106 |
| Stage 2 (27 tasks) | **71 (59-99)** | 71 (59-99) | 39 to 66 |
| Gates outside tasks: the re-gate of this revision (2 to 4), the C1 gate lenses `player`, `bevy-rendering-engineer`, `ux-designer` (3), the final gate (4 to 6), re-gates of repairs (4 to 8), change requests (0 to 3, 2 runs each: 0 to 6), the order-writer repair at the forward merges of A2b1, A2b2 and A2c (3; TD M-A) | 16 to 30 | 13 to 27 | 20 to 35 |
| **MP1 total from now** | **224 to 238 point; 183 to 314 range** | 220 to 234 point; 179 to 309 range | 132 to 225 |
| Slice, remaining (not MP1) | 36 | 38 | 11 to 18 (whole slice) |

What revision 3 changed (+1 task run, +3 gate runs; script, section 20): new tasks `Q-LOCK` +1 and `L0-SPLIT` +2; `L0-CORE` -2 (5 to 3 runs); the order-writer repair +3 in the gates. The slice remainder falls by 2 (Q1b merged). Revision 2 changed +25 point runs net from revision 1 (182 to 207; the detail is in revision 2 of this plan, `04139cc`).

What would cut it (each is a CEO or Root decision, not a plan step): P1 (a) cuts models (about -11); P3 (b) cuts rally (about -6 with the `UI2-HUD` part); the four items of section 7 "not needed for C1" lower stage 1 by about 3 and do not lower the total.

### 14.2 Headed runs

42 MP1 headed runs (revision 2: 41; revision 1: 26).
- Lane 0, 3: `L0-DBG` 1, `L0-SIM` 1, `E-GPU` 1. `Q-LOCK` takes no lock (its self-check uses its own lock file).
- Stage 1, 17: `L-LOBBY-C1` 1, `L-FX-PRES` 1, `UI-PLACE` 1, `UI-HUD` 1, `UI-ALERT` 1, `UI-RESULT` 1, `UI-LABEL` 1, `WEB-NOGPU` 1 (R-A: 8 lane sessions), `I-PIX1` 3 (two pre-merge sessions and the B1 table run), `I-PLAY1` 2, `REG2-1` 3, `C1-PERSON` 1.
- Stage 2, 22: `L-LOBBY2-UI` 1, `UI2-RESULT` 1, `UI2-CONN` 1, `UI2-MC` 1, `UI2-HUD` 1, `FX2` 1, `PRES-ART` 1, `C2-SOUND` 1 (new: the D Runs cell of AUD2 is "pixels", so its plants run in a headed session; qa N2 rule), `PRES-MEASURE` 2, `I2-L` 1, `I2-PIX` 5, `MEAS-EXIT` 2, `REG2-2` 3, `PT3-PEOPLE` 1.
- `C1-PLAYER1`, `C1-PLAYER` and `PT3-PLAYER` take no lock: they judge recorded evidence (R-D).
- The slice adds 4 (A1b, A1c, REG 2).
- In the ASAP schedule, up to 5 headed tasks overlap at offsets 12-13 (`L-LOBBY-C1`, `L-FX-PRES`, `UI-PLACE`, `UI-HUD`, `UI-LABEL`) and 6 at offsets 38-39 (stage-2 client lanes). Each lane task holds the lock for one pre-merge session of at most 20 minutes (section 1, the lock queue rule), so the lock serializes much less than 1 run of each. Pre-merge sessions get the lock before table runs (rule 2); among pre-merge sessions that wait at one time, Root starts the one with the lowest slack first. The headed tasks on the path: `I-PIX1`, `REG2-1`, `C1-PERSON`, `UI2-RESULT`, `I2-PIX`, `REG2-2`, `PT3-PEOPLE`. At offsets 51-53, `MEAS-EXIT` goes after `REG2-2`.

### 14.3 Model tiers

Of the 76 MP1 tasks: 66 have a Sonnet 5.5 builder with an Opus 5.5 review (`L-INC` and `UI2-CONN` with a joint review), 2 a Sonnet builder and a person judge (`C1-PERSON`, `PT3-PEOPLE`), 2 a Haiku 5.5 builder (`C0-PACE`, done, and `REC`), 2 are Opus builder and Opus review (`L0-EXIT`, `L0-EXIT2`), and 4 are Opus work with no review (`C1-PLAYER1`, `C1-PLAYER`, `AUDIT`, `PT3-PLAYER`). At the MC run mix (41 % build, 43 % review, 16 % repair; `docs/research/mp1/input-producer.md:70`), about 89 of the 208 task runs are Opus review runs, plus 7 Opus builder runs.

### 14.4 The three largest risks to the schedule

| Risk | Early test that retires it |
| --- | --- |
| 1. The slice chain D0p → S2 → A1b → A1c → A2b1 → A2b2 → A2c → Q2 → A4 → A3 → `L-TGT` is the path with 0 slack (3.1). The lane-0 chain `L0-API1` → `L0-CORE` or `L0-API2` → `L0-EXIT` and the stage-1 chain `L-PAY` → `L-CON1` → `L-PWR` → `UI-HUD` → `UI-ALERT` have 1 run of slack. D0 (the depletion repair) is not in `codex/mc-integration` yet (`git merge-base --is-ancestor codex/mc-fix-deplete codex/mc-integration` gave exit 1 in revision 2; not run again), so E1 waits for it. | Root checks the D0 merge and the slice board each day; the ten "now" tasks run in the wait. `L0-SPLIT` runs now with its scene oracle. The A3 cross stack lets `L-TGT` and `L-OBS` build at the end of the A3 build, not after the A3 merge. The A3 measure of D section 14 row 4 (D:2523) runs as `L-OBS` step 1. |
| 2. Client work overran in MC (the client shell took 22 runs against 6, D:2529); the stage-1 client tasks are 6 of the 10 zero-slack tasks. | CON1 cause cases red in the first `L-CON1` run (a 6-run task with no red after run 2 is reported). `UI-HUD` probes point 16 in run 1 and has a half-way review. `UI-PLACE` has the headless `Ghost` green by run 2. Stage 2: the lobby stub of D:2529 in `L-LOBBY2-UI` (more than 4 runs is red). |
| 3. One GPU lock for 42 headed runs, and the hosted runner has no GPU (D:2525-2526). | `Q-LOCK` now: the queue rule, the hold limit and the receipts (section 1). `E-GPU` now: two headed clients for 10 minutes with combat, the fail condition and the decision rule, and the hosted MM1 probe. Root reads the lock receipts each day. |

## 15. What can start now

The slice is at 7 of 21 tasks (S1, Q1a, G1, A1a, S3, S45, Q1b merged into `pre-mp1/integration`; head `d7d7e6d`; D0p is in work). E1 needs S2 and D0p. So only tasks with no slice input can start. Each check below was read at `03aef38`, and the rows of revision 3 at `d7d7e6d` (section 20). Q1b changed `scripts/boundary.sh` (+38 lines); it is merged, so the `L0-RULES` row holds.

| Task | Inputs met because | Evidence |
| --- | --- | --- |
| `L0-RAW` | no slice task edits `turts_mod_runtime` | SP task table: 0 hits for `version_table` and `turts_mod_runtime`; the presence of today is `crates/turts_mod_runtime/src/content_mc_validation.rs:65`; the refusal starts at `version_table.rs:96`; S45 touched no `turts_mod_runtime` file |
| `L0-RULES` | new crate; `scripts/boundary.sh` is not in any remaining slice task | SP mentions it only in gate commands (SP:213, :301); `git diff --stat 7b23b37 03aef38 -- scripts/boundary.sh` is empty |
| `L0-DBG` | the four files are outside every remaining slice task, and S45 did not touch them; after `Q-LOCK` (its T7 headed run) | `crates/turts_client/src/lib.rs:147`, `debug.rs:441`, `replica.rs:1182`, `turts_client_web/src/lib.rs:166`, `:371-372`, all at `03aef38`; `git diff --stat 7b23b37 03aef38` on them is empty |
| `L-ROOM0` | new files only; no `session.rs` edit | no `room.rs` in `crates/turts_server/src/`; the end hook lead `session_diagnostics.rs:383-384`, `stats.rs:104-105`; `Slots::claim` at `crates/turts_sim/src/slots.rs:46`; `SLOT_COUNT` at `:7` |
| `E-GPU` | runs on the MC build; after `Q-LOCK` (1 run) | wrapper `mc_headed_v4.sh` sha256 `66c105e5…cec235` (checked again 2026-11-12); `Q-LOCK` makes v6 |
| `Q-LOCK` | a file outside the repository; no input | `mc_headed_v4.sh` read 2026-11-12; `gate-runner/` holds v1 to v4 and `mc_headed_v5_alsa_null_experiment.sh`, so the new file is v6 |
| `L0-SPLIT` | the two functions are in no remaining slice task; S45 is merged | `draw_world_visuals` at `crates/turts_client/src/visuals.rs:279`, `drive_core_adapter` at `core_adapter.rs:730`, `match effect {` at `:1044` (read at `d7d7e6d`); `git diff --stat 03aef38 d7d7e6d -- crates` lists no client file |
| `Q-PLANTS` | new files; the runner is outside the repository | runner `SETS` at `parallel_refutations.py:36-40` (`ci-native`, `ci-browser`, `m3-all`); `tests/<harness>/refutations.py` is the harness form (`:45-52`) |
| `PRES-PROBE` | scratch probes only | `docs/research/mp1/probes/gate2-glb/` holds `make_tank.py`, `make_tank_bad.py`, `glb_probe_main.rs`, `glb_probe_Cargo.toml` |
| `WEB-NOGPU` | after `L0-DBG` (same file `turts_client_web/src/lib.rs`); no slice input | `start()` builds the app at `:144` (presentation M3; verify first) |

Done: `C0-PACE`. `WEB-NOGPU` waits for `L0-DBG` and `Q-LOCK`. Not yet: `L0-API1`, `L0-SIM`, `L0-WIRE` need S2 (`orders.rs` struct literals and the relation match at `orders.rs:1126`) and D0p; `L0-CORE` needs `L0-API1`.

## 16. Design questions from planning

| # | D line | Question | Plan action |
| --- | --- | --- | --- |
| Q1 | D:2428-2430 against D:2424, D:2462 | The stage-2 row list omits STO1, but run 4 and plant (f) are stage 2. | Split STO1 (13.2); erratum E5. |
| Q2 | D:2468 against D:256, D:2428 | ALR2 is a stage-1 row, but its connection alerts need X11c (stage 2). | Split ALR2 (13.2); erratum E5. |
| Q3 | D:2461 against D:260, D:2347 | GOAL1 (stage 1) needs the "+300" float and `Credited`; X14 stage 1 names only `Fired`, `Destroyed`, flash and death. | R14; erratum E4. |
| Q4 | D:82-83, D:2447 against D:279-280 and `crates/turts_sim/src/slots.rs:7` | Exit item 5 asks for 4-slot and 8-slot scenes, and MAT1 has a three-slot sim test; `SLOT_COUNT` is 2. | Decided by ruling R-F: drop the three-slot case; measure with 2 slots; the 4 and 8-slot scenes are recorded as not measured. Errata E1, E2, E3. |
| Q5 | D:241-272 | The engine measure of D 6.9 has no scope item. | `L-OBS` maps to X0e and X4. |
| Q6 | D:274-277 | The alert engine items of D 7.9 have no scope item. | `UI-ALERT` maps to X15 (E + UI work, TD Q6). |
| Q7 | D:1268 against D:2346 | `MatchResult` has `stats`, but statistics are stage 2. | Stage 1 sends `stats: []`; erratum E17. |
| Q8 | D:2303 against D:257 | Section 10 starts L-LOBBY "now", but the seat table, the start step and the sim gate are X11b items of L-MATCH (E1). | `L-ROOM0` now (pure module); wiring in `L-MATCH-SRV1`. |
| Q9 | D:900-905 | How do test harnesses that tick the sim without a server get the `MatchConfig`? | R17 (ruling R-K): `headless_app` inserts it; erratum E12. |
| Q10 | D:60, D:2366 | Content and art could start before C1. | The plan keeps the CEO order. The two presentation risk probes run now (`PRES-PROBE`). |
| Q11 | D 6.4, hint X3 (`JobOnHold(10)`) | The held state flickers under the free income; if `JobOnHold(10)` needs 10 s unbroken, X3 never shows. | **Decided** (ruling R-Y, an orchestrator design call; rigour re-check H1): the 6.4 job state stays per tick; the job record gains `held` (true from the first tick that cannot pay in full until the job has paid in full on every tick for 1,000 ms) and `held_since_tick`; `JobOnHold` and the HUD words read them. Erratum E21; `L-PAY`, `UI-HINT`, `UI-HUD`; kind A, in place before `L0-EXIT` (12). |
| Q12 | D:2972 against D:1986 | The lobby goal line is a shell screen, but D:1986 makes the goal line a mod widget (ux L1). | **needs decision** before `L-LOBBY2-UI` (offset 44): (a) a shell text in the lobby; (b) a mod widget in the lobby layout. Owner `ux-designer`. |

## 17. Engine requests E1 to E11 from the MC PT1 repair (ruling R-P)

Source: `docs/research/mp1/input-mc-pt1-engine-requests.md`. The test for a request that the rulings do not place: does a person at C1 meet it in the first three minutes?

| Request | Where | Reason |
| --- | --- | --- |
| E1 guide pages, one page at a time | MP2 | the hint line gives the first steps at C1; the guide is stage-2 content (`C2-CONTENT`) and a person at C1 does not open it in 3 minutes |
| E2 hide a no-op `SetEnabled` order; an "OFF" field | MP2 | `frontier` has no `SetEnabled` order in stage 1 |
| E3 name labels over units | stage 1, `UI-LABEL` | ruling R-E; erratum E10 |
| E4 objective marker on units of one content id | MP2 | H7 points at the minimap at about minute 3 and later; not met in the first 3 minutes |
| E5 `Format` drops an empty value | MP2 | `C-UI` writes no `Format` with an empty value; `frontier_texts.rs` and check 8.3 point 10 catch "[]" and empty fields |
| E6 alert-list defects (row pitch, clipped first row) | stage 1, `UI-ALERT` (MCX1/s1) | ruling R-N; the first alert shows at about 40 s |
| E6 plain inventory text | MP2 | the stage-1 Ore Hauler shows no inventory panel; the cargo bar is stage 2 (H12, `UI2-HUD`) |
| E7 refusal reason near the pointer, placement | stage 1, `UI-PLACE` (GHO1) | ruling R-P; the first placement is the first action |
| E7 refusal near the pointer, other orders | MP2 | a person meets one in 3 minutes (a right click with the Command Centre selected); stage 1 shows its reason in the notice line with the `frontier` text of `C-UI` ("The Command Centre cannot move. Select units, then right-click."); the pointer position is MP2 |
| E8 an `InserterWaiting` filter by reason | MP2 | `frontier` has no inserter; `GathererIdle` covers the hauler (`UI-ALERT`) |
| E9 control-group rows as one line in the Bindings list | stage 2 part, `UI2-MC` (Controls groups, X18); the one-line form MP2 | not met in the first 3 minutes |
| E10 shapes or models that show the type | stage 1 shapes and labels (`C-UNITS`, `UI-LABEL`); models stage 2 (`PRES-ART`) | ruling R-E |
| E11 a value for the player's own slot | MP2 | `PrimaryOwned` exists (`crates/turts_mod_api/src/interface.rs:235`); no stage-1 text needs the viewer's slot |

## 18. Gate-2 findings and where this revision closes them

"Plan" means this plan, revision 2. "E<n>" is an erratum of section 22. The second table below holds the findings of the re-check of revision 2 and where revision 3 closes them.

| Report | Finding | Closed in |
| --- | --- | --- |
| qa-lead | H1 pixel plants first red only in `I-PIX1` | R-A: section 1 "Pixel rows in the lane", F-HEAD (4), Headed 1 in `L-LOBBY-C1`, `L-FX-PRES`, `UI-PLACE`, `UI-HUD`, `UI-ALERT`, `UI-RESULT`, `UI-LABEL`, `WEB-NOGPU` and the stage-2 client tasks; oracle `node --test` files; budget 14.2 |
| qa-lead | H2 no MP1 plant in a registry before `I2-PIX` | R-B: task `Q-PLANTS`; plant records (1); F-REF MP1 form (4); `REG2-1`, `REG2-2` run `--all` and name CON1 plant (1); R16 |
| qa-lead | H3 no re-run of stage-1 pixel rows with models | R-C: `I2-PIX` re-runs every stage-1 MP1 pixel row and one plant with models on; the finder re-calibration commit reviewed by `bevy-rendering-engineer`; finders count slot pixels in `pick_radius` (`I-PIX1` `mp1-finders.js`); `REG2-2` includes the MP1 stage-1 rows |
| qa-lead | M1 the check cannot see order or missing plants | script: plant rule, tree rule, 3 qa mutations (3.5) |
| qa-lead | M2 process test files, N-client helper, `acceptance.sh` lines, missing hooks | R-O: `L-MATCH-SRV1` (`tests/support/match.rs`, `imm1.rs`, `mat1.rs`), `L-LOBBY-C1` (`lob1.rs`, `leave1.rs`), `L-ROOM0`, `L-LOBBY2-SRV`; 3.4 row `acceptance.sh`; `surrender` in `L-MATCH-SRV2`, `lobby_refusals` in `L-LOBBY2-UI`, `stats_first_seen_tick` in `UI2-RESULT` (the `surrender` owner differs from the report: `L-MATCH-SRV2` owns MAT1/s2, which needs it) |
| qa-lead | M3 C1 and PT3 evidence not defined | R-D: 8.2 evidence set, `I-PLAY1` (driver, capture, logs for each match), 8.5, PT3 (10) |
| qa-lead | M4 `E-GPU` has no fail condition; hosted probe late | R-O: `E-GPU` fail condition, decision rule, hosted MM1 probe |
| qa-lead | L1 F-FROZEN not named; no `done` closing; yardstick; joint reviews; `I2-PIX` reviewer | F-FROZEN rule (1, 4, 9); `REC` closing; no yardstick row (`REC`); joint reviews (1); `I2-PIX` reviewer runs one PT3 plant and one registered plant |
| rigour | H1 three-slot MAT1 case cannot be built | R-F: dropped (`L-MATCH-SIM`), E1, E2; Q4 decided |
| rigour | H2 A3 must be a real input of `L-OBS`, `L-TGT` | R-G: `L-OBS` plain input A3, `L-TGT` stacked `A3^`; tree rule; 3.1 restated |
| rigour | M1 merge-after costs 0 runs | R-G: `e = max(s + runs, end[a] + 1)` in the script; R8, E6 |
| rigour | M2 R14, R15, Q4 change gated D; ALR2/s2 no red | E4, E5, E1 to E3; plant in `UI2-CONN` and E5 |
| rigour | M3 "every answer before offset 10" overstates | R-L: the decision table of the header (kinds A to D) |
| rigour | M4 `C-UNITS` gate cannot fail; `C-UI` has no step | `C-UNITS` gate on a copy and the K2 plant; `C-UI` steps (R-N) |
| rigour | L1 `L0-EXIT` is not the second lens of every lane-0 task | section 1 "Reviewers" corrected |
| rigour | L2 stale `movement.rs` citation and old slice head | section 0 (`03aef38`), `L-TGT` cites `movement/mod.rs:32-37` |
| rigour | L3 items with no red (M5, N1, the hint counter) | `L0-WIRE` export scan (M5); `UI-HUD` changed counter (N1); `UI-HINT` counter threshold |
| rigour | L4 the script does not check "stage 2 after C1" | script rule and mutation (rigour m4) |
| rigour | the lane-0 change commit has no review | 5.2 item 3 |
| rigour | stray `__pycache__` | the cause is an in-process import: `importlib` `exec_module` writes bytecode also under `python3 -I`. This repair made one that way, found it with `rtk proxy git status --short` and deleted it (20). Rule for every agent: run the script as `python3 -I <script>`, or set `PYTHONDONTWRITEBYTECODE=1` before an import |
| technical-director | H1 no owner of the client store | R-H: task `L0-STORE`, contract test; E11 (D:1381); D:2257 stays true |
| technical-director | H2 `L-TGT` and A3 in the same lines; hidden pairs | R-G: P-b (A3 stacked on A4; `L-TGT` stacked on A3; `power::weapon_enabled` stub in `L0-SIM`; `L-PWR` drops `combat.rs`); the script finds concurrency from inputs and marks merge-after-only pairs; P-c not applied |
| technical-director | H3 one writer for order-state changes | R-I: `set_unit_orders`, `order_writers.rs` scan in F-RUST, ORDW1 single-move case (`L0-WIRE`) |
| technical-director | M1 stage-2 shapes frozen too early | R-J: 5.1 item 7, task `L0-EXIT2`; E14 |
| technical-director | M2 the slice rule is not copied | R-K: section 1 "The slice rule"; `L-CON1`, `L-STO`, `L0b-ORD` notes |
| technical-director | M3 no "value at tick" function | R-K: `turts_rules::progress` (`L0-RULES`), used by `L0-STORE`; CON2 grep extended (`UI-PLACE`) |
| technical-director | M4 the default config in production | R-K: R17 rewritten, `L0-SIM`; E12 |
| technical-director | L1 completion count has no owner; L-PAY audit rule | `L-CON1` counts at site completion; `L-PAY` audit rule checks the identity only, with a plant |
| technical-director | Low `C2-CONTENT` merges after the UI2 tasks | `C2-CONTENT` merge-after cell; owns `interface/` |
| technical-director | stage note: `L-INC`, `I-NC1` to stage 2 | R-Q: `I-NC1` moved (E9); `L-INC` kept (the pacing needs it) |
| technical-director | Q10 `PRES-MEASURE` now as a scratch probe | not applied: ruling R-M runs the two risk probes now (`PRES-PROBE`); `PRES-MEASURE` stays on the C1 tree (presentation H2) |
| presentation | H1 D 8.2 has no owner | R-M: `FX2` (yaw, aim, bob), merge after `PRES-MODELS`; `PRES-MODELS` owns the site rise |
| presentation | H2 the client cost rule cannot go red | R-M: `UI-HUD` files `values.rs`, `tooltip.rs`, the changed counter, the plant, the `hud=off` hook; `PRES-MEASURE` HUD on and off on the C1 tree; `PRES-PROBE` probe 2 |
| presentation | H3 `PRES-MODELS` misses the spawn path | R-M: the file list and the entity-layout rule |
| presentation | M1 Bevy 0.19 traps | R-M: `WorldAssetRoot`, `Material{n}/std`, shared handles, `gltf` features |
| presentation | M2 no re-run with models, no re-calibration owner | R-C (see qa H3) |
| presentation | M3 `WEB-NOGPU` not concrete | R-M: the probe method in `WEB-NOGPU`; moved to stage 1 (R-D) |
| presentation | M4 per-frame sites with no owner | `UI-PLACE` (`cursor.rs:32-40`), `UI-ALERT` (`feedback.rs:393`), `L-FX-PRES` (the visuals index) |
| presentation | L1 conflict-list gaps (`debug.rs`, `visuals.rs`) | 3.4 rows; `L-FX-PRES` hot files; R-S split |
| presentation | L2 `Volume::Linear` | `FX2` |
| presentation | L3 `visuals_by_unit` | `REC` |
| presentation | asset proposal and risk probes | `C-UNITS` size table; `PRES-ART` script; `PRES-PROBE` |
| ux-designer | H1 the types look the same | R-E: `C-UNITS` form and size table, `UI-LABEL`, E10; `C1-PLAYER1` "name each own unit type" |
| ux-designer | H2 the C1 start can fail | R-D: the Chrome command in the sheet (8.4) and printed by `mp1-play.sh`; `WEB-NOGPU` in stage 1 (E8) |
| ux-designer | H3 `C-UI` has no checks | R-N: `C-UI` steps, `frontier_texts.rs`, plant |
| ux-designer | M1 lane 0 does not compile in its own order | R-N: `L0-API1` arms; `L0-CORE` `WidgetAction` variants and files |
| ux-designer | M2 no writer of the stage-1 sounds | R-N: `C-UI` writes 6 sounds from `forge` |
| ux-designer | M3 the MC PT1 repair is not in the base | R-N: section 1 forward merge; 3.4 row |
| ux-designer | M4 the first person check comes late | R-N: `C1-PLAYER1` after `I-PLAY1`, before `REG2-1` ends |
| ux-designer | M5 alert list and right click in the first 3 minutes | R-N: MCX1/s1 in `UI-ALERT` (E7); the `no_applicable_order` text in `C-UI` |
| ux-designer | M6 no first-minute check | R-D: 8.3 in C1 and PT3 |
| ux-designer | M7 no stage-2 writer of `interface/` | R-N: `C2-CONTENT` |
| ux-designer | L1 lobby goal line | Q12, needs decision |
| ux-designer | L2 key letters in hints | 8.3 note; recorded for M8 (`REC` vault entry) |
| ux-designer | L3 offline mark in stage 2 but step 10 shows it | `L-FX-PRES` (stage 1); E4 |
| ux-designer | L4 `economy_ui.rs` does not exist | `REC` (`CLAUDE.md`) |
| ux-designer | point 16 confirmed; overrun notes | section 11 point 16; `UI-PLACE`, `UI-HUD` order notes |
| ux-designer | E1 to E10 and the first-60 list not found | the record is `input-mc-pt1-engine-requests.md`; section 17 and 8.3 |
| C0-PACE (R-R) | start credits, residual hold question, early-rush check | `C0-PACE` done; `C-UNITS` 1,400; open point 19 residual and Q11; `L-PAY` held window; `UI-HINT` X3 test; `C1-FINAL` early-rush check; E13 |
| S45 review (R-S) | hot spots after S45 | `L0-CORE` pure moves A and B (in revision 3: `L0-SPLIT`), owner stubs; `UI-ALERT` stacked on `UI-HUD`; `UI2-HUD` input `UI2-MC`; 3.4 |

**The re-check of revision 2 and where revision 3 closes it.** The orchestrator rulings of this revision: R-T (the lock queue rule and task `Q-LOCK`), R-U (each lane runs all its pixel plants red in its one headed session), R-V (the capture form: the Chrome debugging port), R-W (the cross stack A3 on A4 and `A3^` in MP1), R-X (`L0-SPLIT` and the TD sequencing proposal), R-Y (the held flag, a design call), R-Z (the `LabelStyle` fields of row H24).

| Report | Finding | Closed in |
| --- | --- | --- |
| qa-lead | N1 (High) the GPU lock has no queue rule | R-T: section 1 "The lock queue rule" (four points), F-HEAD and the slow gate (4), task `Q-LOCK` (wrapper v6, hold limit, receipt fields, modes `pre-merge` and `table`, self-check), `Q-LOCK` an input of every headed MP1 task (script rule and mutation m20), `REG2-1` table runs, 14.2 |
| qa-lead | N2 three green mutations | script: the pixel rule from the D Runs cell, the D-label rule, the task-text rule; mutations m14 to m16 (3.5); the new rules found and fixed real gaps: LOB2 and AUD2 had no `lane+px` entry (`C2-SOUND` now Headed 1), 23 plant cells held no D label, 17 plant entries (in 15 tasks) were not named in their task text |
| qa-lead | N3 nothing checks the evidence set; the capture path | R-V: `I-PLAY1` `mp1-play-oracle.js` with its unit test and the plants "units hidden", "audio record empty"; the runner loads the instrument through the oracle (`mc-browser.sh:15`, `:295-297`); `mp1-capture.js` through the debugging port; `L0-DBG` states the form; the sheet command (8.4) gains the flag; `I-PLAY1` captures a headed window |
| qa-lead | N4 how each red fixture is made | R-U: section 1 "Pixel rows in the lane": every pixel plant red in the one session, fixtures from real runs, the 20-minute bound, the per-task count; `I-PIX1` splits into two sessions; E26 |
| qa-lead | N5 forward merges stop records | section 1 "Forward merges and plant records": the owner re-pins and runs `--files` red in the same merge |
| qa-lead | N6 one `file` per record | section 1 "Plant records": `files[]`, `env`, `headed`; `Q-PLANTS` self-test case and `--native-only` |
| qa-lead | N7 joint reviews have no own fault | section 1 "Joint reviews": one own fault for INC1, one for SUR1, one for ALR2/s2 |
| technical-director | H-A (High) the cross-branch stack | R-W: section 1 "A cross stack"; `L-TGT`, `L-OBS` inputs; 3.4 combat row; `mp1-tree` rows and the cross-stack script rule; the slice plan binding amendment (A3 stacked on A4, merge commits, `L-TGT` and `L-OBS` merge the A3 branch); E24 |
| technical-director | H-B (High) the store commits with the whole staged chain | `L0-STORE`: staged through `stage_snapshot` and each `stage_delta`, one `commit`; three more contract cases; the Snapshot cost; plant (c); R18; E23 |
| technical-director | M-A the syn scan sees names only | `L0-WIRE`: the listed write patterns, the `test-probes` check in `Collect`, the plant "a write through an inferred `get_mut()`"; 3 repair runs in 14.1; the slice plan amendment |
| technical-director | M-B the pure-move proof cannot see control flow | R-X: task `L0-SPLIT` (own review; the exit and accumulator rules; the scene oracle first, red on a planted change; plant (a) the moved early exit); `L0-CORE` 3 runs |
| technical-director | L-A E6 names two kinds of edge | E24 |
| technical-director | sequencing proposal (three edits) | R-X, R-W: `L0-SPLIT`; `L-PWR` inputs `L0-EXIT`, `L-DMG`, merge after `L-CON1`, stub `power::rate_factor` (`L0-SIM`, `L-CON1` call line); `L-OBS` `A3^`. Script: path 57, C1 37, PT3 56 (3.1; the TD measured the same) |
| technical-director | H24 proposal | R-Z: E22 (with `show: Always` in the RON line, as the orchestrator set it) |
| rigour | H1 (High) Q11 blocks `L-PAY` | R-Y: E21; `L-PAY` held flag, PAY1 case and plant (6); `UI-HINT` X3 on S8; `UI-HUD` reads `held`; 5.1 items 2 and 4; `L0-WIRE` shapes; Q11 decided (16), point 19 closed (11) |
| rigour | M1 row H24 has no field names, defaults or RON | E22; `C-UI` and `UI-LABEL` use the RON line of E22 |
| rigour | M2 the `C-UNITS` plant cannot give the K2 text | the choice is stated in `C-UNITS`: a "did you mean" plant on an existing field (`parse.rs:19-30`); the K2 gap plant moves to `C1-FINAL` |
| rigour | L1 the stage table row X21 | E25 |
| rigour | L2 late "due before" offsets; P8 (a) kind | the CEO table: D-B (2) and P2 (c) before `L-ROOM0` merges (3); D-F (b) before `C-UI` (14); P8 (a) kind C; every offset recomputed for revision 3 |
| rigour | L3 stale `mp1-moves`, STO1 and errata count texts | 3.6 blocks; section 22 count line |
| rigour | L4 three green mutations | script: `REQUIRED_TREE` (with `C1-PLAYER1` ← `I-PLAY1`), the cross-stack rule; mutations m17 to m19 (n2 moved to the mixed row FX1/s1, because its text equals qa m5) |
| orchestrator | Q1b merged during the patch (head `d7d7e6d`) | section 0, the task table (Q1b row removed, A2a takes D0p from the SP Q1b amendment), section 15 |

## 19. Not traced

- The D0 merge time (MC repair lane).
- Which lightyear schedule despawns a `ClientOf` at the netcode timeout (`L-ROOM0` step 1).
- How the harness requests the debug state (`L0-DBG` step 2).
- The name of the credit delivery event (D:1150; `L-FX-WIRE`).
- Whether the order queue accepts a queued `Gather` (D:731-732; `L-RLY`).
- Whether `bevy_ui_widgets` `Activate` fires on a right click (point 16; `UI-HUD` run 1).
- Whether `turts_pack::load_package` parses the interface file (D:878; `L-MATCH-SRV1`).
- Whether the A1b ledger validator has an `EmptyCommand` rule (point 18).
- The browser L (point 10; `I2-L`).
- The play-test machine and its GPU (D-N).
- How the sound set is made (X17; `C2-SOUND`).
- Which MC package gives two armed slots for `E-GPU`.
- Whether T7 needs the GPU lock (`L0-SIM` takes it).
- The `parse.rs:23-29` "did you mean" rule (read in D, not by the producer).
- The ore positions of Twin Fields (D section 5 gives counts only).
- That `vault/MP1 Execution Board.md` exists.
- The function of the alert-list row pitch defect (`widgets/tree.rs`, read at `:188-210`; `UI-ALERT` verifies first). The plan orders `UI-ALERT` after `UI-HUD` for this reason.
- That one headed row run takes at most 2 minutes (the 20-minute bound of a pre-merge session; `E-GPU` and the `Q-LOCK` receipts measure it).
- Which plants of the mixed rows ALR2, LOB1, LOB2, RES1 and FX1 need pixels (section 1, the per-task count is the producer's reading).
- The count of MCX1 plants in the O15 list (`UI2-MC`).
- That the Chrome remote debugging port listens on localhost only and works with the CEO's Chrome version and profile (`L0-DBG`, `I-PLAY1`).
- Whether `driver_rule.py` lets an oracle `require` an instrument file (`I-PLAY1`; the rules A1 to A7 name drivers only).
- The rodio spatial fall-off at `audible_range` (`FX2`).
- The match arms of `L0-API1` that ux M1 lists (`widgets/values.rs:201-233`, `sources.rs:64`, `:120`, `:184`, `:1215`, `feedback.rs:620-837`): read by the ux-designer at `d047433`; the producer did not re-read each at `03aef38`.
- Whether the export writers can read only `CommittedTick` with today's export code (`L0-WIRE` scan rule; `L0-WIRE` verifies first and reports a change request if a record needs a world read).

## 20. What I ran

| Command | Claim tested | Result that would refute | Observed |
| --- | --- | --- | --- |
| `rtk proxy git log --oneline 85f34cb..HEAD` and `git rev-parse` in the slice worktree | section 0: S3 and S45 are merged; head | another set merged, or S45 not merged | head `03aef38`: S45 merge on top of `7b23b37` (S3 merge) |
| `rtk proxy git diff --stat 7b23b37 03aef38 --` on `turts_client/src/lib.rs`, `debug.rs`, `replica.rs`, `render.rs`, `feedback.rs`, `turts_client_web/src/lib.rs`, `turts_mod_runtime`, `turts_server`, `turts_sim`, `scripts/boundary.sh` | R1, 15: the "now" files are outside S45 | any of them listed | empty |
| `git merge-base --is-ancestor` of `codex/mc-fix-pt1` and `codex/mc-fix-deplete` against `codex/mc-integration`, `mp1-design`, the slice head | section 1 (PT1 forward merge), 14.4 (D0) | exit 0 | exit 1 for each |
| grep of `fn draw_world_visuals`, `fn drive_core_adapter`, `fn activate_widget`, `fn sync_widget_tree`, `fn spawn_widget`, `fn reconcile_list_rows`, `fn update_feedback`, `pub fn step` at `03aef38` and in the S45 worktree | ruling R-S anchors (3.4, `L0-CORE`) | a function at another line | `visuals.rs:279`, `core_adapter.rs:730`, `activation.rs:43`, `tree.rs:114`, `:275`, `:662`, `feedback.rs:117`, `turts_interaction/src/lib.rs:143` |
| `sed -n 1040,1050p core_adapter.rs` | the Effect match starts at `:1044` | no match there | `match effect {` at `:1044` |
| grep `SimPlugin {` and `headless_app` | R17 counts | other counts | 6 builders (`harness.rs:13`, `main.rs:68`, `session.rs:792`, `:963`, `:1116`, `:1747`); 51 files |
| grep `pub name` in `schema.rs` | `UI-LABEL` has a name source | no name on `UnitDef` | `UnitDef.name` at `:337` |
| read `feedback.rs:470-495`, grep `no_applicable_order` | the `C-UI` text reaches the screen | no mod text for the key | the notice reads `interface.reasons["no_applicable_order"]` (`:477-494`); the effect is made at `smart.rs:170` |
| read `parallel_refutations.py:25-60` | `Q-PLANTS` can add a set | sets in another form | `SETS` table `:36-40`, harness `tests/<name>/refutations.py` (`:45-52`) |
| `sha256sum mc_headed_v4.sh` | section 1 hash | another hash | `66c105e5…`, the same |
| `cat docs/qa/specs.tsv` (both trees) | `REC` closing value | no MP1 row | `gated` in TuRTS-mp1, `draft` in the slice worktree |
| `python3 -I docs/research/mp1/probes/plan/mp1_plan_check.py <plan> <design>` | 3.1, 3.2, 3.4, 13, 14 numbers | RED, or a cycle | GREEN; path 60; C1 40; PT3 59; 52 row parts, 30 scope items, 21 points, 64 plant entries; 160 pairs, 12 merge-after only; stage runs 39, 97, 71; headed 3, 17, 21; tasks 16, 31, 27 |
| the same with `--self-test` | 3.5: the check can go red | a mutation stays GREEN | 13 mutations RED; unmutated GREEN; exit 0 |
| two file-level mutations of this plan (A3 moved to the Merge-after cell of `L-OBS`; the NOGPU1 plant row deleted) | the check reads the plan file, not only the in-memory mutations | exit 0 | exit 1 each: "task L-OBS must have A3 in its build tree through Inputs"; "row part NOGPU1 has no plant entry" |
| a copy with every merge-after and stacked edge as a plain input (in-process) | R8 reason | the same path | 73 runs, C1 at 49, PT3 at 72 |
| a copy with `L0-SPLIT` (scratch `plan_alt_split.md`) | 3.1 option | no gain | 59 runs, C1 at 39, PT3 at 58 |
| `python3 -I scripts/qa/completeness.py --self-test` and `--enforce` (in `/home/julien/Projects/TuRTS-mp1`) | the registry stays valid; the plan needs no row | exit 1 | self-test 21/21 cases passed, exit 0; enforce exit 0, 3 specs checked |
| `rtk proxy git status --short` in `/home/julien/Projects/TuRTS-mp1` | no stray files | another path | first run: also `docs/research/mp1/probes/plan/__pycache__/` (from an in-process import of the script); deleted; second run: the plan and the script only |

Revision 3 runs (Python with `-I -B`; no import of the script):

| Command | Claim tested | Result that would refute | Observed |
| --- | --- | --- | --- |
| `git rev-parse --short HEAD` and `git diff --stat 03aef38 d7d7e6d -- crates scripts` in the slice worktree | section 0: Q1b merged; no cited client file changed | another head; a client file listed | `d7d7e6d`; 13 files, none in `turts_client`, `turts_client_web`, `turts_server`, `turts_mod_runtime` |
| read `replica.rs:214-290`, grep `fn stage_snapshot|fn stage_delta|fn commit|IgnoredStale|Discarded` at `d7d7e6d` | `L0-STORE` lines | other lines | `apply_snapshot` `:216`, `IgnoredStale` `:221`, `ResyncRequested` `:267`, `commit` call `:268`, `Discarded` `:287`, `stage_snapshot` `:379`, `stage_delta` `:449`, `fn commit` `:553` |
| grep `fn draw_world_visuals`, `return;`, the three accumulators; grep `draw_world_visuals` in `crates/` | R28, `L0-SPLIT`: the exits, the accumulators, no headless test | a test that runs the function | `:279`, `:343`, `:437`, `:438`, `:621`; the only caller is `visuals.rs:175` |
| grep `get_mut::<UnitOrders>|remove::<UnitOrders>` in `crates/turts_sim/src`; grep `test-probes` and `Collect` | `L0-WIRE` scan text | 0 hits; no feature | 24 lines, all in `orders.rs`; `Cargo.toml:11`; `plugin.rs:43` |
| read `mc_headed_v4.sh`; `sha256sum`; `ls gate-runner/` | section 1, `Q-LOCK` | another hash; a free name v5 | `66c105e5…cec235`; `exec flock -x -o …/gpu-runtime-2.lock`; v5 is taken by an experiment file |
| read `scripts/mc-browser.sh:15`, `:295-297`; `tests/browser/package.json` | `I-PLAY1` runner text; `connectOverCDP` | a third script slot; no playwright | one driver and one oracle; `playwright-core` 1.64.1 |
| read `crates/turts_mod_runtime/src/parse.rs:15-35`, `schema.rs:335-347` | `C-UNITS` plant gives the "did you mean" text today | no hint code | `NoSuchStructField` gives `unknown field` and `; did you mean` (edit distance at most 2); `UnitDef.footprint` `:338` |
| the plan check on revision 3 | 3.1, 3.2, 3.4, 14 numbers | RED, or a cycle | GREEN; path 57; `L0-EXIT` 12, C1 37, `L0-EXIT2` 38, PT3 56, `REC` 57; stage runs 40, 97, 71; headed 3, 17, 22; tasks 18, 31, 27; 161 pairs, 13 merge-after only; 52 row parts; 64 plant entries |
| the plan check before the plan cells were fixed (new rules on the revision-2 cells) | the new rules can go red on a real defect | GREEN | RED: 45 lines (LOB2 and AUD2 without `lane+px`, 23 cells without a D label, 17 entries not named in the task text, 3 tree pairs); all fixed in the plan |
| `--self-test` | 3.5: 20 mutations | a mutation GREEN | 20 of 20 RED; unmutated GREEN; exit 0 |
| six file-level mutations of this plan (LOB2 `lane`; HUD5 cell "none"; the `L-STO` plant text deleted; the `C1-PLAYER1` tree row deleted; A3 a merge-after of `L-OBS`; `Q-LOCK` deleted from the inputs of `UI-LABEL`) | the check reads the file | exit 0 on the first five | exit 1 on the first five; the sixth gives exit 0: `UI-LABEL` still descends from `Q-LOCK` through `L0-EXIT` → `L0-SIM` (the rule checks descent; mutation m20 uses `L0-DBG`, which has no other path) |
| the design cross-check `td-rev5/xcheck.py` on D 6.2; a column count of each edited D table row | E21 to E26 keep D consistent | FAILURES above 0; a column count change | FAILURES 0, exit 0; H24 7, H14 7, PAY1 4, X21 part 2, the 6.8 rows 4, the errata rows 3 (equal to 6.1) |
| `completeness.py --self-test`, `--enforce`; `yardstick.py --check` | the registries stay valid | exit 1 | see the reply of this revision |
| `rtk proxy git status --short` | only the four files changed | another path | see the reply of this revision |

Scratch files: `/tmp/claude-1000/-home-julien-Projects-TuRTS/bf418660-b272-4bb3-8628-8d9ef2677d1d/scratchpad/` (revision 2: `table2.md`, `blocks2.md`, `plants2.md`, `plan_try.md`, `plan_alt_split.md`, `rev2a.out`, `rev2a_self.out`; revision 3: `prod-r3/`, with the patch scripts, the six file mutations `f1_*.md` to `f6_*.md` and their `.out` files, and the revision-2 copies).

## 21. What I changed

- `docs/superpowers/plans/2026-11-12-mp1-playable-skirmish.md` (this plan, revision 3).
- `docs/research/mp1/probes/plan/mp1_plan_check.py` (revision 3: the pixel rule from the D Runs cell, the D-label rule, the task-text rule, `REQUIRED_TREE`, the cross-stack rule, the `Q-LOCK` rule; 20 self-test mutations).
- `docs/superpowers/specs/2026-11-12-mp1-playable-skirmish-design.md` (revision 6.2: errata E21 to E26, the Status line, the history bullet, the errata table of section 15).
- `docs/superpowers/plans/2026-11-12-pre-mp1-slice.md` (one binding amendment block after the Q1b block of the orchestrator; the Q1b block is kept).
- No edit of `docs/qa/specs.tsv` or code.

## 22. D errata: revision 6.1 (E1 to E19, applied by Root at `f7090aa`) and revision 6.2 (E21 to E26, applied by this revision)

Each line: the erratum, the D line, the old text, the new text, the ruling. Root applies them in one D commit (revision 6.1) and records it; after that, the plan stage blocks of 3.6 agree with D (the script accepts both states).

| E | D line | Old text | New text | Ruling |
| --- | --- | --- | --- | --- |
| E1 | 2447 (MAT1, Check) | `A sim test with three slots: an order from a defeated slot gets `slot_defeated`` | (delete the sentence) | R-F |
| E1 | 2447 (MAT1, Runs) | `Cases (a) and (b) and the three-slot sim test run in stage 1 with the stage-1 `standoff`` | `Cases (a) and (b) run in stage 1 with the stage-1 `standoff`` | R-F |
| E2 | 1049-1050 | `(only possible with more than two teams; a sim test covers it, row MAT1)` | `(only possible with more than two teams; MP1 has two slots, so no MP1 row tests it; the key stays in REASON_KEYS)` | R-F |
| E3 | 82-83 | `for an MP1 mixed scene at 2 x 200 units, at 1,000 units with 4 slots, and at 5,000 units with 8 slots, and the idle tick apart;` | `for an MP1 mixed scene at 2 x 200 units, at 1,000 units and at 5,000 units with 2 slots, and the idle tick apart; the scenes with 4 and 8 slots are recorded as not measured in MP1 (MP1 has two slots; they belong to the hosting benchmark milestones, D57, D58);` | R-F |
| E4 | 260 (X14) | `Effects: `Fired` and `Destroyed` on the wire (stage 1); flash and death effect (stage 1);` | `Effects: `Fired`, `Destroyed` and `Credited` on the wire (stage 1); flash, death effect, the "+N" float and the offline mark of an unpowered turret (stage 1);` | R14, ux L3 |
| E4 | 2347 | `\| X14 part \| the `Fired` record and one tracer line for each shot; death by removal \|` | `\| X14 part \| the `Fired` and `Credited` records, one tracer line for each shot, the "+N" float, the offline mark of an unpowered turret; death by removal \|` | R14 |
| E5 | 2427 | `KEY1, ALR2, FX1` | `KEY1, ALR2 (without the lost-connection and return alerts, which need X11c), FX1` | R15 |
| E5 | 2428 | `Stage 2 rows: CON3, REP1, RLY1,` | `Stage 2 rows: CON3, REP1, RLY1, STO1 (run 4 and plant (f)), ALR2 (the lost-connection and return alerts), NC1,` (NC1: E9) | R15, Q1, R-Q |
| E5 | 2468 (ALR2, Red build, end) | `... moves no camera, so the camera stays more than 2 world units from the Command Centre` | append `; (j) stage 2: `SlotDisconnected` not wired: a killed client gives no "lost connection" line` | rigour M2 |
| E6 | 2295 | `A dependency graph, not waves; a task starts when its inputs are merged (PR part 4).` | `A dependency graph, not waves; a task starts when its inputs are merged (PR part 4), or it builds against the frozen lane-0 contract and merges at least one run after its merge-after input (plan section 1).` | R8, R-G |
| E7 | 2427 | `KEY1,` (stage 1 list) | `KEY1, MCX1 (the alert-list row pitch and the clipped first row),` | R-N |
| E7 | 2430 | `MDL1, VIS3, MCX1, NOGPU1,` | `MDL1, VIS3, MCX1 (the rest),` (NOGPU1: E8) | R-N, R-D |
| E7 | 267 (X18) | `the grouped failure notice (O14), the no-GPU page \| UI \| 2 \|` | `the grouped failure notice (O14), the no-GPU page; the alert-list defects and the no-GPU page are stage 1 \| UI \| 1, 2 \|` | R-N, R-D |
| E8 | 2424 | (stage 1 list) | add `NOGPU1,` after `DBG1,` | R-D |
| E9 | 2424 | `Stage 1 rows: NC1, VAL1,` | `Stage 1 rows: VAL1,` | R-Q |
| E10 | after 1598 (6.10 table) | (new row) | `\| H24 \| `world.unit_labels` (engine request E3) \| `Option<LabelStyle>`: `show: Always or Selected or Never`, `own_only: bool`, colour `#RRGGBB`, size, font id \| `None`: no labels (the 0.2.3 behaviour) \| colours parse; size 8 to 32; the font exists \| `frontier/interface/hud.ron: world.unit_labels.size: 40 is outside 8 to 32` \| `world.unit_labels`, RAW \|` (the label text is `UnitDef.name`) | R-E |
| E10 | 2348 (X15 part) | `the stage-1 match file, simple shapes \|` | `the stage-1 match file, simple shapes, one form and size class for each type, a name label over each unit (6.10 row H24) \|` | R-E |
| E10 | 2461 (GOAL1) | Check: `... the hint line shows the H1 text of the Player path.` | append after it: ` Each own unit shows its name label (H24).` | R-E |
| E10 | 2461 (GOAL1, Red build, end) | `... the record shows more than 10 changes in 1 s` | append `; (i) the label style ignored: the label oracle finds no label over an own unit` | R-E |
| E11 | 1381 | `The replica is the only source of credits, progress, supply and outcomes (D18).` | append: ` The replica is the candidate and one side store of the 6.8 records that commits with it in one apply path; MP1 adds no field to the candidate (section 9).` | R-H |
| E12 | 902-903 | `Without it, the tick does not advance and no sim system runs.` | append: ` SimPlugin inserts no config. The test harness headless_app inserts the Immediate config; the server inserts the config by its start policy (at boot under Immediate, in the start step under a lobby policy).` | R-K (R17) |
| E13 | 304 | `\| Starting credits \| 1,200 \| 1,200 \| S:805 ("enough credits to establish power"); the other changes remove the holds \|` | `\| Starting credits \| 1,200 \| 1,400 \| S:805 ("enough credits to establish power"); on the stage-1 hint order a player who follows hint F at once holds 0 s (3 Rifles) or 0.7 s (a Tank) where 1,200 gave 7.2 s and 14.5 s; the other changes remove the holds \|` | R-R |
| E13 | 330 | `no hold; 445 credits at 180 s` | `no hold; 645 credits at 180 s` | R-R |
| E13 | 332 | `one hold of 4 s at 95 s` | `no hold` | R-R |
| E13 | 336 | `a new hauler at 333 s` | `a new hauler at 266 s` | R-R |
| E13 | 349-354 | the bullet "The first 3 minutes: ... Open point 19 of section 15 closes it." | `The first 3 minutes: with 1,400 starting credits, no hold above 4 s in S1, S4, S7 and on the stage-1 hint order when the player follows hint F at once. In 600 runs with each step 0 to 12 s slower and four unit lists, 1.3 % hold above 4 s; the worst is 7.7 s (one Tank, one hold, one sound). A hold comes when the player overspends (S5, S8); the alert sounds once (7.9) [player H3]. This is a proposal (C0-PACE).` | R-R |
| E13 | 2507 | `the numbers of section 5 give no hold above 4 s in the ordinary scripts of the revision-4 order (open point 19);` | `the numbers of section 5 give no hold above 4 s in the ordinary scripts;` | R-R |
| E13 | 2608-2612 | point 19 as written | `19. Closed (C0-PACE): the model was run on the stage-1 hint order; starting credits 1,400 (section 5). Residual, needs decision: how JobOnHold(10) counts a hold that flickers (6.4; NOT TRACED). Owner: game-designer with ux-designer. It closes before the payment lane builds the held state (plan L-PAY).` | R-R |
| E13 | 2692 | `\| player N-L2 (pacing of the stage-1 order) \| the measured values are in section 5; open point 19 \|` | `\| player N-L2 (pacing of the stage-1 order) \| closed by C0-PACE, section 5 \|` | R-R |
| E13 | 3009 | `"Credits 1200"` | `"Credits 1400"` (HUD5 at D:2451 keeps "Credits 1200": it is a forced state of a scenario) | R-R |
| E14 | 1388-1389 | `**Lane 0 freezes the shapes first** (section 10): every schema field, version-table row and wire record of 6.8 and 6.10, with the writers empty.` | append: ` The stage-1 shapes freeze at the lane-0 exit; the stage-2 shapes freeze at a second review after C1 (plan L0-EXIT2), with no second protocol or API step.` | R-J |
| E15 | 2324 | `- Each feature lane owns its own pixel rows; the final browser driver holds only the full match.` | `- Each feature lane owns its own pixel rows, and runs each green and one plant red before merge; the stage batches B1 and B2 run them again; the final browser driver holds only the full match.` | R-A, R9 |
| E16 | 2316-2317 | `- **Lane 0b** (E2): the behaviour handler table in `orders/`; hot site 3 (`gather.rs`, which rebases on slice parts A2b1 and A2b2).` | `- **Lane 0b** (E2): the behaviour handler table in `orders/`. Hot site 3 (`gather.rs`, which rebases on slice parts A2b1 and A2b2) is built with the approach point after A2b2 (plan L-OBS); the parse at `combat.rs:265` is built in L-DMG.` | R5 |
| E17 | 1268 (`MatchResult`) | the record cell | append: `; `stats` is empty in stage 1` | Q7 |
| E18 | 57-60 and 270 (X21) | `and the `player` agent plays the loop from the goals text of section 13 to the result screen` / `the `player` agent on the stage-1 loop` | `and the `player` agent judges the recorded evidence of a scripted driver that plays the loop to the result screen (frames, the debug records, the timings and the server logs of the match), from the goals text of section 13` / `the `player` agent on the recorded evidence of the stage-1 loop` | R-D |
| E19 | 2361-2366 (checkpoint C1) | `Conditions: lanes 0a and 0b, the stage 1 lanes and slice part A3 are merged;` | `Conditions: lanes 0a and 0b, the stage 1 lanes and slice part A3 are merged; the C1 instruction sheet names the browser command; the first-60-seconds check list (plan 8.3) holds for the agent and for the person; a headless run on the built content confirms the early-rush numbers of section 5;` | R-D, R-R |

| E21 | 635 (6.4, all costs or none) | `... It resumes on its own.` | append: the job state stays per tick; the job record holds `held` and `held_since_tick`; `held` true at the first tick that cannot pay in full, false after 1,000 ms of full pay (an engine constant; 20 ticks at 20 Hz); `JobOnHold(secs)` and "On hold: no credits" read `held`; a flicker counts as one hold | R-Y (rigour re-check H1) |
| E21 | 1301-1302 (6.8 records) | `SiteProgress{unit, done, total, per_tick, at_tick, state, charged}` ... `on change of rate or job state` (two rows) | add `held, held_since_tick`; `on change of rate, job state or `held`` | R-Y |
| E21 | 1587 (row H14) | `JobOnHold(secs: f32)` | `JobOnHold(secs: f32)` (an own job has `held` true and `secs` have passed since its `held_since_tick`) | R-Y |
| E21 | 1827 (7.3 row states) | `... so it does not flicker under the free income.` | append ` It reads `held` of the job record (6.4, E21).` | R-Y |
| E21 | 2444 (PAY1) | Check ends `... (cost 2^40, `total` 2^25) [perf M3]`; Red build ends `... not 1` | Check: add the case "income that pays 1 tick in 12 for 15 s: `held` true for the whole span; on S8 hint X3 shows after 10 s"; Red build: add (6) "`held` follows the per-tick state" | R-Y |
| E21 | 2607 (point 19) | `Residual, needs decision: ... (plan L-PAY).` | `The residual ... is closed by erratum E21, an orchestrator design call ...` | R-Y |
| E22 | 1597 (row H24) | `Option<LabelStyle>`: `show: Always or Selected or Never`, `own_only: bool`, colour `#RRGGBB`, size, font id | `WorldStyle` gets `unit_labels: Option<LabelStyle>`; `LabelStyle { show: LabelShow, own_only: bool = false, colour: String, font: Option<String> = None, size: u16 }`; `LabelShow { Always, Selected }`; checks `#RRGGBB`, size 8 to 32, the font exists; RON `unit_labels: Some((show: Always, own_only: false, colour: "#ffffff", font: None, size: 14)),` | R-Z (rigour M1, TD H24 proposal) |
| E23 | 1379 (6.8, the E11 sentence) | `... MP1 adds no field to the candidate (section 9).` | append: the store change set is staged with the candidate through the Snapshot stage and each replayed delta, one commit applies both; a failed resync, a stale Snapshot and a discarded delta change neither; a Snapshot replaces the store once | TD H-B |
| E24 | 2294 (10, the E6 sentence) | `... merges at least one run after its merge-after input (plan section 1).` | append: the third kind, the stacked edge, and its cross-branch form (merge of the slice branch; merge commit, not a rebase) | TD L-A, R-W |
| E25 | 2349 (stage table, X21 part) | `the `player` agent plays the stage-1 loop from the goals text (section 1)` | `the `player` agent judges the recorded evidence set of a scripted driver that plays the stage-1 loop, from the goals text of section 13 (E25)` | rigour L1 (completes E18) |
| E26 | 2322 (10, the E15 lane rule) | `runs each green and one plant red before merge` | `runs each green and every pixel plant red in one headed session before merge (E26)` | R-U (qa N4; completes E15) |

D revision 6.1 has 20 errata: E1 to E19 are the rows of this table above E21, and E20 is the orchestrator's ruling (D section 15, errata table; it completes E7 and E8). D revision 6.2 adds 6 errata (E21 to E26), in 11 rows. In all, 26 errata in 47 table rows.
