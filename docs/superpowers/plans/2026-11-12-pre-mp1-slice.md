# Pre-MP1 slice: implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Plan status:** DRAFT for gate 2. Author: `producer` (build mode), 2026-11-12. Root saves this text unchanged as `docs/superpowers/plans/2026-11-12-pre-mp1-slice.md`.

**Design of record:** `docs/superpowers/specs/2026-11-12-pre-mp1-slice-design.md`, revision 4 (commit `431c3ad`, branch `mp1-design`), "D" below.
- Section 9 of D is binding. It replaces the lines it names.
- Where D says "as in revision 1", the text is D revision 1 (commit `a29ce67`).
- Sources: the ruling `docs/research/mp1/ruling-td-one-step-function.md` ("RUL"); the gate reports `docs/research/mp1/gate-slice-qa.md` ("GQA") and `docs/research/mp1/gate-slice-simulation.md` ("GSIM"); the input `docs/research/mp1/input-technical-director.md` ("TDI").
- When this plan and D disagree, D wins. Report the conflict to Root. Section 2 lists each place where this plan refines the order of D. Each refinement needs Root's acceptance at gate 2.

**Goal:** the four CEO decisions of 2026-11-11 (`vault/Decisions/2026-11-11 MC architecture audit.md:47-48`) and the audit debt of `:33`, before MP1:
- the economy log validator checks ledger conservation only;
- the validator lives in a `turts_audit` crate, with one file for each domain;
- one movement step function, where a pool is optional data on the unit;
- one re-freeze of the M2 stream;
- a neighbour grid cell that does not depend on the largest unit;
- split files, so that MP1 lanes do not wait for each other.

**Scope rule:** the slice adds no mod API. It changes one thing that a player can see (D section 1, `:30`). It changes one frozen value one time, at A2c. The items of D section 7 are not in this slice.

## 0. Line numbers

- All `path:line` in this plan are at `431c3ad` (TuRTS-mp1), or at the commit that the line names.
- The depletion repair (`codex/mc-fix-deplete`, `43282fe`) changes `orders.rs` (6 lines), `economy/gather.rs` (10), `economylog.rs` (87) and `turts_server/tests/economy.rs` (196). Run `git diff --stat 8d6902c 43282fe`.
- S1 moved the validator from `economylog.rs` to `crates/turts_audit/src/replay.rs` (lane `pre-mp1/s1-audit`, commit `c1ebb23`).
- So every line number after `:1000` in `orders.rs`, and every validator line number, moves.
- **Verify first** means: the producer read the line at `431c3ad`, and the builder must find the item again by its name or its text before the edit.
- **NOT TRACED** means: the producer did not read it.

## 1. Execution contract

- **Integration branch.**
    - `pre-mp1/integration`, from `codex/mc-integration` at `8d6902c`. That is the base of both lanes in work: `git merge-base` gives `8d6902c` for `pre-mp1/s1-audit` and for `pre-mp1/s45-client`.
    - Never merge into `main` or `codex/mc-integration`.
    - Root merges `codex/mc-integration` into `pre-mp1/integration` after the depletion repair lands there (task D0p).
- **One branch per task.**
    - The branch name is `pre-mp1/<id>`.
    - A task branch starts from `pre-mp1/integration` after every task in its "Inputs" has merged there.
    - A task starts the moment its inputs merge. There are no waves (ruling C391, `vault/Decisions/2026-10-30 MC Codex rulings.md:25-43`).
- **One worktree and one build cache per lane.**
    - Create the worktree: `git worktree add ../TuRTS-pre-<id> pre-mp1/<id>`.
    - Set `CARGO_TARGET_DIR=target/pre-<id>` inside it.
    - The two lanes in work keep their caches: `target/mc-pre-s1-audit` and `target/pre-s45-client`.
    - Delete the worktree and its target directory after the merge.
- **Reviewers.**
    - Each reviewer works in its own worktree, `../TuRTS-pre-<id>-review`, at the candidate commit.
    - It runs the task's fast gate. It runs one fault of its own (red, exact restore, fresh green).
    - It cites `path:line`.
- **Step order in every task** (MC plan `:53`): red, green, commit, gate, plants.
    - A compile error is not a red.
    - Restore each plant with `git checkout -- <file>`. Then `git status --porcelain -- <file>` must print nothing.
    - Never pipe a red or a green run through `tail`, `head` or `grep` before you read its exit code. Write `; echo exit=$?` after each command.
- **Pure moves.**
    - Prove a pure move with `item_equal.py` (D `:49-52`; sha256 `7b14c917…` at `/home/julien/Projects/TuRTS/target/mc-evidence/root-regate/item_equal.py`, copy at `docs/research/mp1/probes/item_equal.py`).
    - Also prove that `cargo test --workspace -- --list` gives the same names before and after (command in section 4).
    - The colour-moved diff is a reading aid only (GQA M6).
- **Frozen value.**
    - `sha256sum tests/m3/m2_operation_stream.json` prints `bcbf72c2206251f635a7c7f5b6c8f573d69a5fc05a5bd77716e98ebfb08e370b` (`tests/m3/refutations.py:18`) until the Q2 commit.
    - A task other than A2c and Q2 that gets a red on `m2_operation_stream_matches_baseline` (`crates/turts_pack/tests/m3_m2_stream.rs:350`) stops and reports. Run that task again only after a new ruling (D `:146`).
- **Git.**
    - Stage explicit paths. Never `git add -A`.
    - Build each binary with its own `cargo build -p <crate>`.
    - Do not edit `CLAUDE.md`, `.claude/agents/`, D, the MC design or the product spec. Put each doc change in section 13.
- **Records.**
    - Root counts each dispatch on the board (`vault/Pre-MP1 Execution Board.md`; Root creates it; NOT TRACED that it exists).
    - The record for each task: red, green, plant outputs, changed paths, commit, run count.
    - A task that uses twice its run estimate is reported. It is not halted (C391 item 7).
- **Stop and report, do not decide:**
    - a frozen value changes outside Q2;
    - a step contradicts D;
    - a fault has no red after its change (D `:81`);
    - the classifier finds a difference outside P3 (D `:146`);
    - the turts_audit compile check of R2 fails.

## 2. Plan refinements of D (Root accepts or refuses each one at gate 2)

| # | Refinement | Reason (evidence) | Cost if refused |
| --- | --- | --- | --- |
| R1 | G1 starts now. It runs in parallel with S1, S2 and S3. Its stream counter run (step 4) waits for D0p. | G1 edits `grid.rs` only (`grid.rs:61-70`). S1, S2 and S3 do not edit `grid.rs`. The counter must see the trips of the depletion repair (GSIM L2). | none on the path (G1 is not on the critical path) |
| R2 | A2a runs in parallel with A1b and A1c. A2b waits for A1c. | The replay validator uses only these movement items: `step_frame_preserving_resting`, `MovementFrame`, `MovementFrameUnit`, `MoveProgress`, `PlainMoveProgress`, `PlainGatherGoal`, `spread_destinations`, `FollowMoveOrigin`, `MovementActivity` (grep of `turts_sim::movement::` in `economylog.rs` below `:8084`). A2a keeps their signatures, and its output is bit-equal (D `:43`). D section 8 item 4 says only A2b touches what the validator reads. Condition: `cargo check -p turts_audit --locked` passes on the A2a branch. If it fails, A2a waits for A1c. | +4 runs on the critical path (24 to 28; scratch model) |
| R3 | A4 starts after A2b, in parallel with A2c and Q2. A4 and A3 merge only after the Q2 re-freeze commit. Each runs G-FROZEN again with the new hash. | A4 edits `movement/frame.rs`. A2c edits the dispatch (`orders.rs:2822-2900`, `command.rs:488-495`). The rule keeps the classified pair of Q2 free of A4 and A3. | +5 runs on the critical path |
| R4 | A1 is four tasks: A1a (live reader, outcome assertions), A1b (TDI step 2 and the side-by-side exit commit), A1c (TDI step 3), A1d (TDI step 4). A1d is not an input of A2b. | TDI `:77-81`; D section 4 items 1 to 7; D section 9 items 1 and 2. | +2 runs |
| R5 | New task D0p: port the validator hunks of the depletion repair into `crates/turts_audit/src/replay.rs`. | The repair edits `economylog.rs` in the validator at `:3470` and `:5541` (`git diff -U0 8d6902c 43282fe`). S1 moved those lines to `replay.rs` (`c1ebb23`: `economylog.rs` −18,880 lines). | the merge of D0 into the slice has no owner |
| R6 | Q1b commits the list of allowed differences, `tests/m3/refreeze_allowed.json`, before the A2c commit. | D section 5 item 3 P1: "committed before A2c". | none |
| R7 | The "CI grep" of D section 9 item 5 is a `turts_checks` test. So it runs in `cargo test --workspace` (`.github/workflows/ci.yml:33`), with no `ci.yml` edit. | `ci.yml` stays out of the hot files. | none |
| R8 | Q2 is the lane of `qa-lead` on top of the A2c branch. A2c and Q2 merge together, after the CEO approval. | A2c alone is red at G-FROZEN by design (D section 5 item 2). | none |
| R9 | A2a builds the bit-equal reference harness. A4 and A3 use it again, each with a reference copy taken from its own base. | D `:54-59`. | +2 runs |

## 3. Dependency graph

Model: scratch file `slice.py`. Cycle check: no cycle.
- Run estimates come from the MC actuals of the most similar MC task (MC board ledger, 135 runs).
- Lane hours = runs × 2 h. The MC range per run is 1.4 to 2.9 h (MC T6: 8 runs, about 23 h; MC T16: 22 runs, about 30 h; from the merge commit times).
- "In work" counts only the remaining runs.

| Task | What | Builder seat | Reviewer seat | Branch | Inputs (merged first) | Hot files (shared with) | GPU | Runs | Lane h |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| S1 | `turts_audit` crate, replay validator moved unchanged (in work: `c1ebb23`, `5ce21d2`) | backend-engineer | qa-lead | `pre-mp1/s1-audit` | none | `crates/turts_audit/*` (D0p, A1b-A1d), `economy.rs` (D0p, A1a-A1c) | no | 2 | 4 |
| S45 | Client splits S4 and S5 (in work: 6 commits to `d047433`) | bevy-rendering-engineer | ux-designer | `pre-mp1/s45-client` | none | `tests/m2/refutations.json` (S2, A1b, A1c) | slow set only | 2 | 4 |
| G1 | `within` finds every point inside `reach` | simulation-core-engineer | qa-lead | `pre-mp1/g1-grid` | none (step 4: D0p) | `grid.rs` (A3) | no | 3 | 6 |
| Q1a | Classifier and its self-test | qa-lead | simulation-core-engineer | `pre-mp1/q1a-classifier` | none | `tests/m3/refreeze_classify.py` (Q1b, Q2) | no | 2 | 4 |
| D0 | Depletion repair merged into `codex/mc-integration` (MC, not a slice task) | (MC) | (MC) | `codex/mc-fix-deplete` | MC | — | — | 0 | — |
| D0p | `codex/mc-integration` merged into `pre-mp1/integration`; repair hunks ported into `replay.rs` | backend-engineer | simulation-core-engineer | `pre-mp1/d0-port` | D0, S1 | `replay.rs`, `economy.rs`, `orders.rs`, `gather.rs` | no | 2 | 4 |
| S2 | `orders.rs` to `orders/` | simulation-core-engineer | technical-director | `pre-mp1/s2-orders` | D0p | `orders/` (A2b, A2c, A3), `refutations.json` | no | 2 | 4 |
| S3 | `movement.rs` to `movement/` | simulation-core-engineer | technical-director | `pre-mp1/s3-movement` | D0p | `movement/` (Q1b, A2a, A2b, A4, A3) | no | 2 | 4 |
| A1a | Live reader; outcome assertions for the "strictly replays" tests | backend-engineer | qa-lead | `pre-mp1/a1a-reader` | S1 | `economy.rs` (D0p: other hunks), `mc_orders.rs`, `session.rs` | no | 3 | 6 |
| Q1b | Trace emitter, `separated` source, the counts, the allowed list | qa-lead | simulation-core-engineer | `pre-mp1/q1b-trace` | S3, G1, Q1a | `movement/frame.rs`, `turts_sim/Cargo.toml` | no | 2 | 4 |
| A1b | `LedgerAudit`, domain files, both validators side by side, domain plants, writer scan, exit commit | backend-engineer | qa-lead | `pre-mp1/a1b-ledger` | A1a, D0p, S2, S3 | `turts_audit/*`, `economy/ledger_view.rs` (new), `refutations.json`, `economy.rs`, `el1.rs`, `mc_orders.rs` | yes (ci-browser 8) | 4 | 8 |
| A2a | One step function (bit-equal), phase counters | simulation-core-engineer | qa-lead | `pre-mp1/a2a-step` | Q1b, G1, S3 | `movement/frame.rs` | no | 4 | 8 |
| A1c | Replay validator deleted; E8 re-pin; self-checks become plants; sink writes `LedgerAudit` | backend-engineer | qa-lead | `pre-mp1/a1c-delete` | A1b | `turts_audit/*`, `economy.rs`, `el1.rs`, `refutations.json` | yes (ci-browser 8) | 2 | 4 |
| A1d | `common/conservation.rs` replaced by `turts_audit` | backend-engineer | qa-lead | `pre-mp1/a1d-conservation` | A1c | `crates/turts_pack/tests/common/*`, `tests/m3/refutations.json` | no | 2 | 4 |
| A2b | Fallback makes a pool of one; plain model deleted | simulation-core-engineer | qa-lead | `pre-mp1/a2b-fallback` | A2a, A1c, S2 | `movement/*`, `orders/plain.rs`, `views.rs`, `ledger_view.rs`, `el1.rs` | no | 3 | 6 |
| A2c | The plain move and `MoveTo` take the pool path | simulation-core-engineer | qa-lead | `pre-mp1/a2c-pool` | A2b, Q1b | `orders/apply.rs`, `command.rs` | no | 3 | 6 |
| Q2 | Re-freeze classification, technical-director review, CEO approval, fixture commit | qa-lead (technical-director reviews the difference) | technical-director | `pre-mp1/q2-refreeze` (on `pre-mp1/a2c-pool`) | A2c | `tests/m3/m2_operation_stream.json`, `tests/m3/refutations.py:18` | no | 3 | 6 + CEO wait |
| A4 | Rest of the frame as functions on `&mut FrameState` | simulation-core-engineer | performance-architect | `pre-mp1/a4-frame` | A2b (merge after Q2) | `movement/frame.rs` (A3) | no | 2 | 4 |
| A3 | Two neighbour grids; combat cell from the smallest weapon range | simulation-core-engineer | performance-architect | `pre-mp1/a3-grids` | A4, G1 (merge after Q2) | `movement/*`, `grid.rs`, `economy/combat.rs`, `orders/run.rs` | no | 3 | 6 |
| REG | Slice regression on the final merged tree | qa-lead | (Root reads) | `pre-mp1/reg` (evidence only) | Q2, A3, S45, A1d | — | yes | 2 | 4 |
| TDN | Architecture note before MP1 lanes start on the slice tree | technical-director | (Root reads) | none (text to Root) | REG | — | no | 1 | 2 |
| | **Total (remaining)** | | | | | | | **49** | **98 lane h** |

**Critical path** (scratch `slice.py`): S1 → D0p → S3 → Q1b → A2a → A2b → A2c → Q2 → REG → TDN = **24 runs**.
- That is 48 h at 2 h per run (range 34 to 70 h).
- Two waits come on top: the depletion repair (D0, MC) and the CEO approval in Q2.
- A second path of equal length ends at A2b: S1 → D0p → S2 → A1b → A1c → A2b. A slip on either path delays A2b.
- If R2 is refused, the path is 28 runs.

**Lanes at each stage** (offset in runs from now; D0 assumed at once):

| Offset | Lanes | Tasks | Headed GPU need |
| --- | --- | --- | --- |
| 0-2 | 4 | S1, S45, G1, Q1a | S45 slow rows only |
| 2-4 | 3 | G1, D0p, A1a | no |
| 4-6 | 3 | S2, S3, A1a | no |
| 6-8 | 2 | Q1b, A1b | A1b (8 browser faults) |
| 8-12 | 2 | A2a, then A1b / A1c | A1b, A1c |
| 12-15 | 2 | A2b, A1d | no |
| 15-18 | 2 | A2c, A4 / A3 | no |
| 18-21 | 2 | Q2, A3 | no |
| 21-24 | 1 | REG, TDN | REG |

The peak is 4 lanes. Headed work queues on the one GPU lock. Only S45, A1b, A1c and REG need it.

## 4. Gate sets

### Fast gate for one task (in the lane, after the commit)

Use the sets that the task names. The commands come from the MC plan `:62-73` unless the line says otherwise.

- **F-RUST**
    - `cargo fmt --all --check; echo exit=$?`
    - `cargo clippy --workspace --all-targets --locked -- -D warnings; echo exit=$?`
    - `cargo test -p <crate> --locked; echo exit=$?`. Run it for each changed crate and each crate that depends on it: `turts_sim` → `turts_pack`, `turts_server`, `turts_checks`; `turts_audit` → `turts_server`, `turts_pack`.
    - `bash scripts/boundary.sh; echo exit=$?`
    - Each prints `exit=0`.
- **F-WASM** (client tasks only): `cargo check --workspace --exclude turts_server --target wasm32-unknown-unknown --locked; echo exit=$?` prints `exit=0`.
- **F-QA**: the three G-QA commands (MC plan `:63`). Each prints `exit=0`.
- **F-FROZEN**
    - `sha256sum tests/m3/m2_operation_stream.json` prints the pinned hash.
    - `cargo test -p turts_pack --locked --test m3_m2_stream --test forge_fixed_point --test forge_cases; echo exit=$?` prints `exit=0`.
- **F-MOVE** (pure moves). Write evidence to `target/pre-<id>/evidence/`.
    1. `git show <base>:<old path> > target/pre-<id>/evidence/old/<name>` for each old file.
    2. `python3 docs/research/mp1/probes/item_equal.py --old <old files> --new <new files>; echo exit=$?` prints `RESULT EQUAL old_items=N new_items=N` and `exit=0`.
    3. `cargo test --workspace --locked -- --list > target/pre-<id>/evidence/list-<sha>.txt 2>&1; echo exit=$?`. Run it on the base and on the candidate.
    4. `grep -E ': (test|bench)$' <list file> | sort > <list file>.names` for each file.
    5. `cmp <base>.names <candidate>.names; echo exit=$?` prints `exit=0`.
- **F-REF** (faults that plant in the changed files)
    - Use the selector of the MC plan `:69-70`, with `pre-mp1/integration` in place of `codex/mc-integration`.
    - Run the selected non-headed cases with the parallel runner (argv from `runs/slice-gate-native/run.json`): `python3 /home/julien/Projects/TuRTS/target/mc-evidence/root-regate/parallel_refutations.py --set ci-native --cases <ids> --worktree <lane worktree> --evidence-root /home/julien/Projects/TuRTS/target/mc-evidence/root-regate/runs --build-root <lane worktree>/target/par --label pre-<id>-ref --workers 8 --alsa; echo exit=$?`
    - Expected: `summary.json` holds `"failed": []`.
    - Browser cases go to the slow set.

### Slow gate for one merged tree (Root, on `pre-mp1/integration`)

Root runs this set one time for each merged tree that differs from a tree that passed it (C391 item 3). Two merges that land during one slow run share the next run.

1. G-RUST, full (MC plan `:62`), with `cargo test --workspace --locked`.
2. G-QA (`:63`).
3. G-ACC: `bash scripts/acceptance.sh; echo exit=$?`. It includes EL1 (`scripts/acceptance.sh:140`), E8 and E9 (`:104-106`) and DET1.
4. G-WEB: `(cd crates/turts_client_web && trunk build --release); echo exit=$?`, then `bash scripts/t7.sh; echo exit=$?`. T7 keeps the unit within 0.011 of the ordered point (D `:161`).
5. G-FROZEN: the pinned hash until Q2, the new hash after Q2.
6. Fault sets with the parallel runner (RUN125: 33 native faults in 1,666 s):
    - `--set ci-native --workers 8 --alsa`
    - `--set m3-all --workers 9 --alsa`
    - `--set ci-browser --workers 8 --wasm-slots 2 --alsa`. This set takes the GPU lock.
    - Use `--label pre-int-<sha>-<set>` and `--build-root <integration worktree>/target/par-<set>`.
    - Expected: `"failed": []` in each `summary.json`. A1b adds domain rows to ci-native, so the expected count grows from 33.
7. Pixel rows, two runs each (`scripts/mc-pixels.sh`; 34 row files in `tests/browser/rows/`):
    - CI rows: `bash scripts/mc-pixels.sh tests/browser/rows/*.json; echo exit=$?`.
    - Headed rows (MM1, ORD3, ORD4, ORD5, PT1, VIS2): run them through the headed wrapper, which holds the GPU lock. Command: `bash /home/julien/Projects/TuRTS/target/mc-evidence/gate-runner/mc_headed_v4.sh --lane <worktree> --target <target dir> --head <sha> --label pre-int-<sha>-pix -- bash scripts/mc-pixels.sh <row files>; echo exit=$?`.
    - Wrapper sha256 now: `66c105e5904357c5916b4a015517a1542ea2f8f16ea0128b3d2051a554cec235`. It changed after C391 (c667b08).
    - The flags come from `exec_in_lane_v4.py --help`: **verify first**.
    - A browser run that fails with a timing symptom during a parallel build runs again one time before it is judged (C391 item 8).
8. From the merge of A1a on: E9 loss green five times in a row (A1a step 4 command).
9. Hosted CI on each push of `pre-mp1/integration`. Root reads the result when it ends (C391 item 5).

## 5. Points where the slice can stop early with value

| Stop | After | Value for MP1 | What stays open, and its deadline |
| --- | --- | --- | --- |
| E1 | S1, S45, G1, D0p, S2, S3 (about 6 runs on the path) | Split files, so MP1 lanes stop queueing on them. The audit crate exists. The grid defect is repaired. | The replay validator is still a second model. An MP1 behaviour would need a validator branch (TDI E1 refuses it). So MP1 lanes that add orders or movement must wait. |
| E2 | A1a to A1c (about 12 runs) | The validator checks conservation only. MP1 behaviours need no validator branch. | Two step branches and the plain model stay. MP1 must add no test that pins a plain move (audit `:48`: at the latest before M4). |
| E3 | A2c and Q2 (about 21 runs) | One movement model. The one re-freeze is done. | A4 (audit `:36`: split before M4). A3 (audit `:33`: before MP1 content adds a command centre or a long-range tank). |

## 6. CEO touch points (no others)

1. **Before A2b starts** (D `:30`; RUL `:52`). Root sends this sentence:
    > "After the slice, when you send a unit to a point that another moving unit already goes to, the new unit stops in a free place next to the point and the first unit keeps the point (before, the first unit gave the point up); a harvester or other engine trip that finds its point blocked uses the same free-place rule."

    Root adds D section 1: "Above 2,048 single moves in one tick, the rest start one tick later." The CEO answers. If the CEO refuses, A2b and A2c stop for a new ruling.
2. **In Q2, step 6.** The CEO approves these items in one vault note:
    - the old sha256 `bcbf72c2…e08e370b`;
    - the new sha256;
    - the classifier output;
    - the player-visible change.

    Nothing else in this slice needs the CEO.

## 7. Tasks

### Task S1: `turts_audit` crate, validator moved unchanged (in work)

**Seat:** `backend-engineer`. Review: `qa-lead`: is the move pure, and does each boundary row go red?

**Branch / inputs:** `pre-mp1/s1-audit` (worktree `../TuRTS-pre-s1-audit`, target `target/mc-pre-s1-audit`); none. **Design:** D section 3; TDI A1 step 1 (`:78`). **Done so far:**
- `c1ebb23`: `crates/turts_audit/src/{lib.rs,format.rs,replay.rs}`. `economylog.rs` keeps the writer and the sink.
- `5ce21d2`: `scripts/boundary.sh:51-60` has the `turts_audit` rows.

- [ ] **Step 1: Pure-move proof.** F-MOVE.
    - Old file: `economylog.rs` at `8d6902c`, lines 1 to 19,082.
    - New files: `crates/turts_audit/src/format.rs`, `crates/turts_audit/src/replay.rs` and `crates/turts_server/src/economylog.rs`.
    - Expected: `RESULT EQUAL` and `exit=0`.
    - Test names: compare after the map `economylog::tests::` → `replay::tests::` for the tests that moved. Write the map as `sed 's/^economylog::tests::/replay::tests::/'` on the base list, before `sort`.
    - Expected: `cmp` gives `exit=0`, or only the names of the seven writer tests that stay in `economylog.rs` differ. Those seven are listed in the commit message.
- [ ] **Step 2: Boundary reds.** For each new row, run `bash scripts/boundary.sh --rule forbid --crate <crate> --target <target> --manifest <fixture manifest> --names turts_audit; echo exit=$?` (`scripts/boundary.sh:25-31`) against a fixture that has the edge.
    - Expected: the forbid message and `exit=1`.
    - How `tests/boundary-fixture` selects its rows is NOT TRACED by the producer (D `:70`). The builder records it.
- [ ] **Step 3: Gate.** F-RUST, F-QA, F-FROZEN, F-REF.
    - Plant files of this change: `turts_server/src/session.rs` (M2 E7, E8 cases).
    - Expected: `"failed": []`.
- [ ] **Step 4: Reviewer fault.** Plant one: in `replay.rs`, change a validator comparison (for example `!=` to `==` in the refusal rule, text "refused unit {unit_id} changed state inside its order operation", `economylog.rs:2349` at `8d6902c`).
    - Run: `cargo test -p turts_audit --locked; echo exit=$?`.
    - Expected: red, `exit=101`. Restore, then green.

**Exit:** both proofs are EQUAL. All boundary rows are red on their fixtures. **Runs left: 2.**

### Task S45: client splits (in work)

**Seat:** `bevy-rendering-engineer`. Review: `ux-designer`.

**Branch / inputs:** `pre-mp1/s45-client` (worktree `../TuRTS-pre-s45-client`); none. **Design:** D revision 1 rows S4 and S5; TDI C4 to C7 (`:269-287`). **Done so far** (commits `34b7c4f` to `d047433`):
- core_adapter tests out;
- visuals tests out;
- `turts_interaction` `lib.rs` split;
- `visuals.rs` split into materials, markers, health and world_tooltip;
- `client_shell.rs` split;
- `widgets/mod.rs` split.

`tests/m2/refutations.json` changed 2 lines. Case `E2-browser-hardcoded-label` now plants `crates/turts_client/src/visuals/world_tooltip.rs`.

- [ ] **Step 1: Pure-move proof for each commit.** F-MOVE: `item_equal.py` on the old and new files of each split, and the test list `cmp`.
    - `Core::step` (`turts_interaction/src/lib.rs:137`) is not split (D `:167`).
    - Expected: `RESULT EQUAL` six times, and `cmp` `exit=0`.
- [ ] **Step 2: Registry anchors.** For each browser case whose plant file moved, run `python3 -c` with the selector of MC plan `:70`. Then check that the anchor occurs exactly once in the new `plant_path` (`tests/m2/refutations.py:270-280`, MC plan `:72`; **verify first**).
    - Case `E9-browser-debug-only-ui` still names `crates/turts_client/src/visuals.rs`. Check that its anchor is still in that file.
- [ ] **Step 3: Gate.** F-RUST, F-WASM, F-QA.
    - The browser cases of the changed files run in the slow set. They are "not checked" in the lane, not a pass.
- [ ] **Step 4: Reviewer fault.** Move one statement in a moved function (a swap of two lines). `item_equal.py` must print `RESULT DIFFERENT` and `exit=1`. Restore.

**Exit:** all proofs are EQUAL. The slow set on the merged tree is green, with the pixel rows that read the shell and the HUD (SH1, SH1-dpr2, KM1, KM2, PER1, SEL4, HUD1, HUD2, HUD4, VIS1). **Runs left: 2.**

### Task G1: `within` finds every point inside `reach`

**Seat:** `simulation-core-engineer`. Review: `qa-lead`: can the property test fail, and does the counter fire?

**Branch / inputs:** `pre-mp1/g1-grid`. No input for steps 1 to 3. Step 4 needs D0p (R1). **Design:** D section 2 row G1, section 5 item 0, section 9 items 3 and 4; RUL ruling 3 (`:56-65`).

**Files:**
- Modify `crates/turts_sim/src/grid.rs:61-70`. Today `rings = (reach / self.cell_size).ceil()` (`:63`).
- The test module is at `grid.rs:127-170`.

- [ ] **Step 1: Red.** Add `within_matches_brute_force_scan` to the `grid.rs` test module. `within` is `pub(crate)`, so the test must be inside the crate.
    - The test runs 24,000 seeded queries. It includes whole-number `reach / cell`, negative coordinates, and the gate case: qx 1.8, px -7.2000003, reach 9, cell 0.9 (D `:189-190`).
    - Assertion message: `within missed {n} points inside reach`.
    - A second check: the same query at 3 cell sizes gives the same ids, with the message `within result depends on the cell size`.
    - Run: `cargo test -p turts_sim --locked --lib grid::; echo exit=$?`.
    - Expected: red on the miss message. GSIM saw 43 misses in 24,000 queries; the count here depends on the generator. `exit=101`.
- [ ] **Step 2: Green.** Scan the cells of the bounding box of the query disc, made larger by `1e-4 × max(1, reach)`. The distance filter does not change.
    - Rerun step 1: `exit=0`.
    - If the bounding-box form fails, use `ceil + 1` and record its tick cost (RUL `:59`).
- [ ] **Step 3: Commit, gate.** Commit. F-RUST, F-FROZEN.
- [ ] **Step 4: Edge counter (not committed; after D0p).**
    - Write a patch that counts the points that the fix adds, as a test-only counter (the pattern of `movement.rs:13-21`).
    - First, show at least 1 hit on the gate case.
    - Then run, each with `; echo exit=$?`:
        - `cargo test -p turts_pack --locked --test m3_m2_stream --test forge_fixed_point`
        - `cargo test -p turts_sim --locked`
        - `cargo test -p turts_pack --locked`
        - `cargo test -p turts_server --locked`
        - `bash scripts/acceptance.sh`
    - Put these under `docs/research/mp1/probes/g1-edge-counter/`: the patch, the run log (commit, command, hits for each test) and `SHA256SUMS`. Check with `sha256sum -c SHA256SUMS; echo exit=$?`, expected `exit=0` (D `:191-194`).
- [ ] **Step 5: Decide.**
    - 0 hits on the stream and the golden tests: G1 merges.
    - Hits on the stream: G1 does not merge. Its commit goes into A2c (D `:116`). Q1b adds `edge_hit` to the trace.
    - Write the result on the board.
- [ ] **Step 6: Plant.** Restore `ceil` without the margin. `within_matches_brute_force_scan` goes red with `within missed`. Restore.

**Exit:** 0 misses and no result that depends on the cell size. The counter result is recorded. **Runs: 3.**

### Task Q1a: classifier and self-test

**Seat:** `qa-lead`. Review: `simulation-core-engineer`: does each P rule match D section 5?

**Branch / inputs:** `pre-mp1/q1a-classifier`; none. **Design:** D section 5 (items 3 and 4, `:131-145`), section 9 items 6 and 7.

**Files:** create `tests/m3/refreeze_classify.py` (Python 3 standard library). It fixes the trace format (JSONL, one file for each package) that Q1b writes.
- Unit record:
    - `package`, `tick`, `unit`;
    - `x_bits`, `y_bits` (u32 hex);
    - `target` (bits or null);
    - `activity`: `Pending`, `Moving`, `Resting`, `Blocked` or `none` (D `:204-205`);
    - `state`: `none` or `pool`;
    - `separated`;
    - `edge_hit` (only when G1 is in A2c).
- Operation record:
    - `package`, `tick`, `op_id`, `unit`, `phase`;
    - `events`: the full payloads;
    - `before_equals_after`.
- Header: `emitter_sha256`.

Modes:
- `--p0 OLD NEW`: byte-equal for each package; it uses `cmp`-equivalent bytes.
- `--a2c OLD NEW --allowed LIST --stream STREAM`: P0 for base, haul and cedar; P1, P2 and P3 for acme; the arrival-tick check; the count assertion (4 packages × 400 ticks × the initial unit count); equal emitter sha256; the stream sha256 that it read, printed.

- [ ] **Step 1: Red.** Write the self-test first, with a stub classifier that accepts everything.
    - Run: `python3 tests/m3/refreeze_classify.py --self-test; echo exit=$?`.
    - Expected: `self-test: 0/12 plants red`, `exit=1`.
    - The 12 plants (D `:128`, `:143-144`):
        1. one flipped position bit;
        2. one added `Stored` event;
        3. one changed operation in `base`;
        4. one scout operation on the arrival tick itself;
        5. two operations of one acme tick in the other order;
        6. a flipped `before_equals_after` on the tick-1 operation;
        7. an empty trace;
        8. a trace of 399 ticks;
        9. a changed event payload with the same kind;
        10. a different emitter sha256;
        11. a stream sha256 that is not the classified stream;
        12. an acme field difference that is not in the allowed list.
- [ ] **Step 2: Green.** Implement. Rerun: `self-test: 12/12 plants red; clean pair green`, `exit=0`.
- [ ] **Step 3: Commit, gate.** F-QA.

**Exit:** 12 of 12 plants are red and the clean pair is green. **Runs: 2.**

### Task D0p: depletion repair into the slice

**Seat:** `backend-engineer`. Review: `simulation-core-engineer` (gather).

**Branch / inputs:** `pre-mp1/d0-port`; D0 (the repair merged into `codex/mc-integration`), S1. **Design:** D section 2 order ("the depletion repair is merged"); GSIM L2.

**Files:**
- The merge of `codex/mc-integration` into `pre-mp1/d0-port`.
- Port the two validator hunks of `43282fe` (old `economylog.rs:3470` and `:5541`; `validate_automatic_order_operation` and `set_audited_gather_cycle`) into `crates/turts_audit/src/replay.rs`.
- Port the four test hunks (old `:8860-8945`) into the tests in `replay.rs`.
- `crates/turts_server/tests/economy.rs`: the repair hunks are at old `:470`, `:1134`, `:1978` and `:2016`. They are disjoint from A1a's `:1792-1870`.

- [ ] **Step 1: Trial merge.** `git merge --no-commit --no-ff codex/mc-integration; echo exit=$?`, then `git diff --name-only --diff-filter=U`. Record the conflicted files.
- [ ] **Step 2: Port.**
    - Apply each hunk to the moved code.
    - Proof: `item_equal.py --old <economylog.rs at the repair head> --new crates/turts_audit/src/format.rs crates/turts_audit/src/replay.rs crates/turts_server/src/economylog.rs`.
    - Expected: `RESULT EQUAL`, `exit=0`.
- [ ] **Step 3: Green.** Run each with `; echo exit=$?`, each expected `exit=0`:
    - `cargo test -p turts_sim --locked --test gather_depletion`
    - `cargo test -p turts_audit --locked`
    - `cargo test -p turts_pack --locked --test mc_order_rows`
- [ ] **Step 4: Commit, gate.** F-RUST, F-FROZEN, F-REF. Case `E8-foreign-gather-owner-check` plants `gather.rs` and `orders.rs`: check that each anchor is in its file exactly once.

**Exit:** the repair's tests are green on the slice tree. The port is EQUAL. **Runs: 2.**

### Task S2: `orders.rs` to `orders/`

**Seat:** `simulation-core-engineer`. Review: `technical-director`: are the module paths kept, and is the move pure?

**Branch / inputs:** `pre-mp1/s2-orders`; D0p. **Design:** D revision 1 row S2; TDI C2 (`:254-261`).

**Files:** `crates/turts_sim/src/orders.rs` (7,036 lines at `431c3ad`; tests from `:3673`) becomes `crates/turts_sim/src/orders/`. All ranges are **verify first**: they move by +6 lines after the repair, and items, not line numbers, decide.

| New file | TDI range |
| --- | --- |
| `mod.rs` | types `:417-507`; `pub(crate) use` re-exports, so callers keep `crate::orders::*` |
| `pending.rs` | `:49-416`, `:1134-1189`, `:1358-1416`, `:1652-2185` |
| `plain.rs` | the plain code, which A2b deletes: `:96-178`, `:1189-1358`, `:3101-3286`, `:3614-3660` (TDI A2 `:92`) |
| `run.rs` | `:697-1134` |
| `apply.rs` | `:2610-3101`, `:3331-3673` |
| `behaviours/attack.rs`, `follow.rs`, `queue.rs` | `:2185-2610` |
| `tests.rs` | the test module, declared as `#[cfg(test)] mod tests;` in `mod.rs`, so the test names stay `orders::tests::*` |

`crates/turts_sim/src/lib.rs` keeps `mod orders;`, so it needs no edit. Registry: `E8-foreign-gather-owner-check` gets its `plant_path` for `orders.rs` changed to the new file. The anchor text does not change.

- [ ] **Step 1: Move.** One commit. No item body changes.
- [ ] **Step 2: Proof.** F-MOVE (old: `orders.rs` at the base; new: every file under `orders/`). Expected: `RESULT EQUAL`, and `cmp` `exit=0`.
- [ ] **Step 3: Gate.** F-RUST, F-FROZEN, F-REF (`E8-foreign-gather-owner-check`; expected `"failed": []`).
- [ ] **Step 4: Reviewer fault.** Swap two statements in `run.rs`. `item_equal.py` must print `RESULT DIFFERENT`. Restore.

**Exit:** EQUAL, the same test names, and the stream unchanged. **Runs: 2.**

### Task S3: `movement.rs` to `movement/`

**Seat:** `simulation-core-engineer`. Review: `technical-director`.

**Branch / inputs:** `pre-mp1/s3-movement`; D0p. It runs in parallel with S2. The second to merge rebases. They share no file. **Design:** D revision 1 row S3; TDI C3 (`:262-268`).

**Files:** `crates/turts_sim/src/movement.rs` (3,873 lines; test modules from `:2511`) becomes `movement/`. All ranges are **verify first**.

| New file | TDI range |
| --- | --- |
| `spread.rs` | `:116-480` |
| `search.rs` | `:517-666` |
| `plain.rs` | `:671-990` |
| `pool_index.rs` | `:995-1345` |
| `frame.rs` | `:1346-2408` (`step_frame` `:1382`, `step_frame_preserving_resting` `:1388`) |
| `ecs.rs` | `:2409-2512` (`integrate_movement` `:2410`) |
| `mod.rs` | types (`:24-115`; `MovementActivity` `:24-29`, `PlainMoveProgress` `:58`, `PlainGatherGoal` `:79-82`); the test counter of `:13-21`; re-exports |
| `tests/` | `spread_tests` (`:2512`), `plain_movement_tests` (`:3471`) and the other modules, each at its old module path |

- [ ] **Step 1: Move.** One commit.
- [ ] **Step 2: Proof.** F-MOVE. Expected: `RESULT EQUAL`, `cmp` `exit=0`.
- [ ] **Step 3: Gate.** F-RUST, F-FROZEN, F-REF. No registry case plants `movement.rs` (JSON scan at `431c3ad`). Run the selector again and record "0 cases".
- [ ] **Step 4: Reviewer fault.** Change `normalize() * step_limit` (`:2003-2004`) to `* step_limit * 1.0001`. `cargo test -p turts_sim --locked --test group_movement; echo exit=$?` must go red. Restore.

**Exit:** EQUAL, the same test names, and the stream unchanged. **Runs: 2.**

### Task A1a: live reader and outcome assertions

**Seat:** `backend-engineer`. Review: `qa-lead`.

**Branch / inputs:** `pre-mp1/a1a-reader`; S1. D0p edits other hunks of `economy.rs`; the second to merge rebases. **Design:** D section 4 items 1 and 4 (third bullet), section 9 item 1; GQA H1, M5.

**Files:**
- `crates/turts_server/tests/economy.rs`:
    - `read_live_complete_tick_prefix` `:1792-1870`. Today it matches the validator text at `:1846-1849`. The text "command operation id {operation_id} has no matching operation" is at `economylog.rs:1085` at `8d6902c`, and in `replay.rs` after S1.
    - `live_audit_reader_ignores_only_unterminated_tail_and_waits_for_tick` `:54`.
- `crates/turts_server/tests/mc_orders.rs`: `:469`, `:574`, `:694`, `:824`, `:1089`, `:1094`. These are the "strictly replays" tests, named at those lines.
- `crates/turts_server/src/session.rs:1021` (`rejoin_reuses_slot_without_writing_a_second_economy_claim`).

- [ ] **Step 1: Red (reader).**
    - Extend `live_audit_reader_ignores_only_unterminated_tail_and_waits_for_tick` with a log cut inside an open `CommandBoundary`.
    - Add a planted reader `whole_tail_reader` in the test that returns the whole tail. The test asserts that the planted reader fails and the real reader returns the complete prefix.
    - Run: `cargo test -p turts_server --locked --test economy live_audit_reader_ignores_only_unterminated_tail_and_waits_for_tick -- --exact; echo exit=$?`.
    - Expected: red at the new assertion on the new reader stub (signature first), `exit=101`.
- [ ] **Step 2: Green (reader).**
    - The reader finds the incomplete tail itself: the records after the last `Tick` record, or an open `CommandBoundary`. It reads no validator text.
    - Rerun: `exit=0`.
- [ ] **Step 3: Red then green (outcomes).**
    - For each of the six `mc_orders.rs` tests and `session.rs:1021`, add an outcome assertion from the order log: the unit outcomes, the final order state, or the claim count.
    - Red by plant: GQA saw that `:469` stays green with the validator off (GQA `:38`). Plant a dispatch that drops the second unit in `orders/pending.rs`, or in `orders.rs` before S2. `cargo test -p turts_server --locked --test mc_orders queued_group_attack_spreads_and_strictly_replays -- --ignored --exact; echo exit=$?` must now go red on the outcome assertion. Restore.
    - Whether these tests are `#[ignore]` is **verify first**. Drop `--ignored` if they are not.
- [ ] **Step 4: Commit, gate, E9 ×5.**
    - Commit. F-RUST, F-FROZEN.
    - `cargo build --release -p turts_server --locked; echo exit=$?` and `cargo build --release -p turts_client --locked; echo exit=$?`. These are the pre-commands of the E9 registry row.
    - Then `for i in 1 2 3 4 5; do cargo test --release -p turts_server --locked --test economy e9_loss_and_small_buffer_converge -- --ignored --exact --nocapture > target/pre-a1a/evidence/e9-$i.log 2>&1; echo run=$i exit=$?; done`.
    - Expected: five `exit=0` (D `:89`). The test is at `economy.rs:3709`. The command is from `tests/m2/refutations.json`, case `E9-duplicate-spawn-tail-replay`.

**Exit:**
- The reader test is red on the planted reader and green on the new reader.
- Seven outcome assertions, each with a red by plant.
- E9 is green five times.

**Runs: 3.**

### Task A1b: `LedgerAudit`, domain validators, side by side, exit commit

**Seat:** `backend-engineer`. Review: `qa-lead`: does each domain plant go red with its own message?

**Branch / inputs:** `pre-mp1/a1b-ledger`; A1a, D0p, S2, S3. The writer scan names the `orders/` and `movement/` files. **Design:** D section 3, section 4 items 3, 4, 6 and 7; TDI A1 (`:28-61`, step 2 `:79`).

**Files:**
- Create `crates/turts_sim/src/economy/ledger_view.rs`: `LedgerAudit` and one function from `&AuditState`.
    - Fields: the list of TDI `:59`.
    - Add every field that `tests/browser/m2-audit.js:67-120` reads (`units[].pos`, `facing`, the gather order `stage`, the node `pos`).
    - Add every field that `crates/turts_server/tests/el1.rs:188-215` reads (`movement_progress`, `plain_movement_progress`, `physical_escapes`, `unit_orders`) (D `:95-98`).
    - Do not remove a field from `AuditState` (`views.rs:374`; D `:75-77`).
- Create in `crates/turts_audit/src/`: `clock.rs`, `slots.rs`, `balances.rs`, `inventories.rs`, `reservations.rs`, `production.rs`, `industry.rs` and `combat.rs`.
    - Seed them from `crates/turts_pack/tests/common/conservation.rs`: `stock` `:185`, `balance_expected` `:258`, `check_operation` `:370`, at `8d6902c`. **Verify first.**
- Extend `crates/turts_audit/src/lib.rs`:
    - the record shape (exactly one `Initial`, first);
    - the rule "a refused command or operation leaves the full logged view unchanged" (`economylog.rs:2346-2350`; D `:100-101`);
    - `validate_ledger`;
    - `validate_side_by_side`, which runs `validate_ledger` first and returns its error first, then runs the replay validator.
- Callers: every caller of `validate_records` calls `validate_side_by_side`. Callers per TDI `:324`: `economy.rs`, `el1.rs`, `mc_orders.rs`, `session.rs:1053-1059`. **Verify first.**
- Create `crates/turts_checks/tests/movement_writers.rs` and a planted fixture under `crates/turts_checks/tests/fixtures/movement_writers/`.
    - The scan covers writes to `Position`, `UnitOrders`, `MoveTarget`, `MoveProgress` and the pools.
    - The allow list holds `movement/`, `orders/`, `economy/gather.rs` (the insert at `:1261`), the spawn writes (`economy/transaction.rs:699`, `map.rs:163`), `command.rs:495` and `synthetic.rs:72,92` (D `:102-104`; GQA M5).
    - **Verify first:** each of these lines.
- `tests/m2/refutations.json`: one new ci-native row for each domain file and for the refusal rule. Each row has `plant_path` in sim code, `expected_failure` = the new ledger message, and `command` = the economy test that reads the log (D `:110`).

- [ ] **Step 1: Red.**
    - `cargo test -p turts_audit --locked; echo exit=$?`: each new domain test fails on its stub (signature first), with the domain message, for example `balance of slot 1 ore differs from its events`. `exit=101`.
    - `cargo test -p turts_checks --locked --test movement_writers; echo exit=$?`: red on the stub, `exit=101`.
- [ ] **Step 2: Green.** Implement. Rerun both: `exit=0`. Every caller runs `validate_side_by_side`, and both validators accept every clean log (F-RUST).
- [ ] **Step 3: Commit, gate.**
    - F-RUST, F-QA, F-FROZEN.
    - F-REF with the new rows: `--set ci-native --cases <new ids>`. Expected: `"failed": []`. Each new row is red with its ledger message.
    - EL1: `cargo test --release -p turts_server --locked --test el1 -- --ignored --test-threads=1 --exact el1_muster_movement_and_queued_gather_validate_every_captured_tick; echo exit=$?` (`scripts/acceptance.sh:140`). Expected: `exit=0`.
    - ci-browser (8 cases; the GPU lock), through the slow-set command with `--label pre-a1b-browser`. Expected: `"failed": []`.
- [ ] **Step 4: Plants.**
    - (a) A spawn write of `Position` in a file outside the allow list (the fixture): `movement_writers` goes red. Restore.
    - (b) Remove `facing` from `LedgerAudit`: `m2-audit.js` reads it (`:67-120`), and the browser case `E9-browser-…` that reads the audit log goes red. Which case reads it is NOT TRACED by the producer; the builder names it. Restore.
- [ ] **Step 5: The exit commit.**
    - On one commit, every item of D section 4 items 1 to 6 is green: the reader (A1a), E8 (it still gets its red from the validator, `economy.rs:3184`; D `:186-188`), the readers, the cover rules, EL1, the registry rows.
    - Record that commit on the board as "A1 exit" (D `:111-112`). The next commit (A1c) deletes the replay validator.

**Exit:** the A1 exit commit is recorded. **Runs: 4.**

### Task A1c: replay validator deleted

**Seat:** `backend-engineer`. Review: `qa-lead`.

**Branch / inputs:** `pre-mp1/a1c-delete`; A1b. **Design:** D section 4 items 2 and 5, section 9 item 2; TDI step 3 (`:80`), `:66-74`.

**Files:**
- Delete `crates/turts_audit/src/replay.rs`. Callers call `validate_ledger`. The sink in `economylog.rs` writes `LedgerAudit`.
- `crates/turts_server/tests/economy.rs`:
    - The self-checks `:3293-3345` (`validate_records(&missing_reservation)` … `(&leaked_claim)`) become plants of `reservations.rs`, with the new messages.
    - Delete `:486-564` (a position moved by 1 ulp; TDI `:67`).
    - Keep `:441`, `:457` and `:468` under the domains.
    - All four ranges: **verify first**.
- `tests/m2/refutations.json`: case `E8-mixed-cost-partial-commit` (`expected_failure` today: "refused unit 1 changed state inside its order operation") gets `expected_failure: "rejected seq 5 is write neutral"`. That is the text of `economy.rs:3197-3201`. Do it in the same commit as the deletion (D `:186-188`).
- `crates/turts_server/tests/el1.rs`: the assertions of the void EL1 faults ("movement input missing", "the validator of today") go. The test asserts: the ledger validator accepts each tick, and the run records the log bytes (D `:108-109`).

- [ ] **Step 1: Red.**
    - In the lane, before the deletion, re-pin the E8-mixed row only.
    - Run `--set ci-native --cases E8-mixed-cost-partial-commit`.
    - Expected: `"failed": ["E8-mixed-cost-partial-commit"]`, because its red still comes from the validator text. That shows why the re-pin waits for the deletion.
- [ ] **Step 2: Green.** Make the deletion and the re-pin in one commit. Rerun: `"failed": []`.
- [ ] **Step 3: Gate.**
    - F-RUST, F-QA, F-FROZEN.
    - The full ci-native set: 33 rows plus the A1b rows.
    - EL1 (A1b step 3 command).
    - ci-browser (8 cases).
    - E9 ×5 (A1a step 4 command).
    - Expected: each `exit=0`; `"failed": []`.
- [ ] **Step 4: Plant.** Restore one deleted self-check as a plant: a missing reservation. It must go red with the `reservations.rs` message. Restore.

**Exit:** no replay code remains. `grep -rn 'fn validate_movement_transition' crates; echo exit=$?` prints `exit=1`. All faults are red. **Runs: 2.**

### Task A1d: one ledger oracle

**Seat:** `backend-engineer`. Review: `qa-lead`.

**Branch / inputs:** `pre-mp1/a1d-conservation`; A1c. **Design:** D `:73-74`; TDI step 4 (`:81`).

**Files:**
- `crates/turts_pack/tests/common/conservation.rs` (1,088 lines) is replaced by calls to `turts_audit`, which becomes a dev-dependency of `turts_pack`.
- `tests/m3/refutations.json`: no edit if every `expected_failure` text keeps its bytes.

- [ ] **Step 1: Text check first.** For each M3 case, `grep -F` its `expected_failure` in `conservation.rs` and in `turts_audit`. Record which text comes from where. A text that would change needs a registry edit, and `qa-lead` reviews it (MC plan `:72`).
- [ ] **Step 2: Replace.** Rerun: `cargo test -p turts_pack --locked; echo exit=$?`, `exit=0`.
- [ ] **Step 3: Gate.** F-RUST, F-FROZEN. `--set m3-all --workers 9 --alsa`. Expected: 17 of 17 red, `"failed": []` (TDI `:81`; GQA `:63`).

**Exit:** one oracle. M3 C1 to C17 are red. **Runs: 2.**

### Task Q1b: trace emitter and allowed list

**Seat:** `qa-lead`. Review: `simulation-core-engineer`: does each trace field come from the state the design names?

**Branch / inputs:** `pre-mp1/q1b-trace`; S3, G1 (the counter result), Q1a. **Design:** D section 5 item 1 and section 9 items 6 and 7; RUL `:140-144`.

**Files:**
- `crates/turts_sim/Cargo.toml`: a feature `test-probes = []`. `turts_sim` has no `[features]` section at `431c3ad`.
- `crates/turts_sim/src/movement/frame.rs`: `MovementFrameOutput` gets `separated: bool` behind `#[cfg(feature = "test-probes")]`. It is filled from the local `separated` of each branch (`movement.rs:770`, `:1973`, `:2134` at `431c3ad`; **verify first** after S3). When G1 lands in A2c, it also gets `edge_hit`.
- `crates/turts_pack/Cargo.toml`: dev-dependency `turts_sim` with `features = ["test-probes"]`.
- Create `crates/turts_pack/tests/m2_trace.rs`: `#[ignore] m2_trace_writes_four_packages`. It uses the run loop of `m3_m2_stream.rs` (`TICKS` `:15`, `PACKAGES` `:16`, `run_package` `:240`) and writes `<dir>/<package>.jsonl` in the Q1a format to `TURTS_M2_TRACE_OUT`.
- Create `tests/m3/refreeze_allowed.json`: the acme differences that D allows.
    - The scout's `state` becomes `pool` from the accept tick.
    - The scout operations: one added at tick 1, and one at the tick after arrival. The gate probe gave arrival 123 and an operation at 124 (D `:126`).
    - Positions do not change.

- [ ] **Step 1: Run on the base.** `TURTS_M2_TRACE_OUT=target/pre-q1b/evidence/base cargo test -p turts_pack --locked --test m2_trace -- --ignored --exact m2_trace_writes_four_packages; echo exit=$?`. Run it two times, into `base` and `base2`. `cmp` each package pair: `exit=0`. That proves the trace is deterministic. `sha256sum` each file into the board record.
- [ ] **Step 2: Counts** (D `:202-203`; measured, not assumed). For each package, record the unit-ticks on `PlainMoveProgress`, `MoveProgress` and no progress, and the count of `separated = true`. GSIM measured acme 122, base 256, haul 230, cedar 289 unit-ticks with no progress, and 0 on the others. If a stream unit spends any tick on `PlainMoveProgress`, stop and report: the A2b claim of D `:44` fails.
- [ ] **Step 3: Feature leak check.** `cargo tree -p turts_server -e features -i turts_sim; echo exit=$?`. Expected: no `test-probes` in the normal build (resolver 3, `Cargo.toml:2`). This is NOT TRACED until this run.
- [ ] **Step 4: Commit, gate.** Commit the emitter, the feature and `refreeze_allowed.json`. Log the sha256 of `refreeze_allowed.json` on the board before A2c (R6). F-RUST, F-FROZEN.
- [ ] **Step 5: Plant.** Set `separated` to `false` in all branches. The step 2 count changes, or `--p0` against `base` goes red. Restore.

**Exit:** a deterministic base trace, the counts recorded, and the allowed list committed and hashed. **Runs: 2.**

### Task A2a: one step function

**Seat:** `simulation-core-engineer`. Review: `qa-lead`: can the counter go red, and is the reference copy the old kernel?

**Branch / inputs:** `pre-mp1/a2a-step`; Q1b, G1, S3 (R2). **Design:** D section 2 row A2a, `:54-59`, section 6 (the one-step guard), section 9 item 5; RUL ruling 1.

**Files** (after S3; ranges at `431c3ad`, **verify first**):
- `movement/frame.rs`:
    - the plain-progress branch `:1843-1879` stays until A2b;
    - the no-progress branch `:1880-2085`;
    - the pool branch `:2086-2380`.
- The no-progress and pool branches become one step function for one unit.
- Where one value has two float expressions, keep the no-progress expression. Example: `velocity.normalize() * step_limit` (`:2003-2004`) against `velocity *= step_limit / velocity_len` (pool branch, RUL `:25`).
- No writer changes. `step_frame_preserving_resting`, `MovementFrame`, `MovementFrameUnit`, `PlainMoveProgress`, `PlainGatherGoal` and `spread_destinations` keep their signatures (R2).
- Phase counters behind `test-probes`: arrival, steering, projection, stall, fallback; for each unit and tick (the pattern of `:13-21`).
- Create:
    - `crates/turts_sim/src/movement/reference.rs` (`#[cfg(feature = "test-probes")]`): a copy of the kernel at the A2a base.
    - `crates/turts_sim/tests/kernel_reference.rs`.
    - `crates/turts_sim/tests/one_step_function.rs`.

- [ ] **Step 1: Red (counter).**
    - `one_step_function.rs::each_moving_unit_passes_each_shared_phase_once`. The scenario: a gather trip, a queued start, an attack approach, a follow, a resume, a single envelope move, a group move and a synthetic unit (D `:155`).
    - It asserts one count for each moving unit, phase and tick, with the message `unit {id} tick {t} phase {p}: {n} calls`.
    - Put the counters in the code of the pool branch only. Then units on the no-progress branch show 0.
    - Run: `cargo test -p turts_sim --locked --features test-probes --test one_step_function; echo exit=$?`.
    - Expected: red (`0 calls` for the gather unit), `exit=101`. Record the run (D `:197-198`).
- [ ] **Step 2: Red (harness smoke).**
    - `kernel_reference.rs::kernel_matches_reference_bit_for_bit`:
        - 64 seeds, each chained for 300 ticks;
        - 1 to 5,000 movers, 0 to 20 statics of radius 0.5 to 8;
        - a `preserve_resting` set that is not empty;
        - frames that hold no-progress, plain-progress, pooled, escape, `Blocked`, follow and Attack-Point units (RUL `:75`);
        - every f32 compared with `to_bits`;
        - a counter for each phase, which must be above 0.
    - Plant a `-0.0` in the real kernel (a sign flip of a zero velocity).
    - Run: `cargo test --release -p turts_sim --locked --features test-probes --test kernel_reference; echo exit=$?`.
    - Expected: red on the bit compare, `exit=101`. Restore.
    - Then run the harness with the merged step on the pooled seeds only. Record the count of seeds that differ (risk 5). If the count is above 0, stop and report.
- [ ] **Step 3: Green.** Merge the two branches. Rerun steps 1 and 2: `exit=0`.
- [ ] **Step 4: Commit, gate.**
    - Commit. F-RUST, F-FROZEN.
    - Q1a `--p0` on all four packages: run the emitter on the A2a base and on A2a, then `python3 tests/m3/refreeze_classify.py --p0 <base dir> <a2a dir>; echo exit=$?`, `exit=0` (D `:200-201`).
    - `cargo test -p turts_sim --locked --test group_movement --test gmu_same_point; echo exit=$?` and `cargo test --release -p turts_server --locked --test mc_group -- --ignored; echo exit=$?`. These hold GM40, GMA and GMU; which test holds which row is **verify first**. Each `exit=0`. Each changed assertion has a reason in the commit message (D `:43`).
    - `cargo check -p turts_audit --locked; echo exit=$?` is `exit=0` (R2).
- [ ] **Step 5: Plants.** Each goes red, then restore:
    - (a) speed `Some(0.0)` in the merged step: `kernel_reference` goes red.
    - (b) swap arrival and steering in the merged step: `kernel_reference` goes red.
    - (c) the counter call in a second copy of the steering code: `one_step_function` goes red with `2 calls`.

**Exit:** one step function. Bit-equal. P0 equal on four packages. The counter is red before A2a and green after; both runs are recorded. **Runs: 4.**

### Task A2b: the fallback makes a pool of one; the plain model goes

**Seat:** `simulation-core-engineer`. Review: `qa-lead`.

**Branch / inputs:** `pre-mp1/a2b-fallback`; A2a, A1c, S2. **CEO touch point 1 comes before this task starts.** **Design:** D section 2 row A2b, section 6 (blocked engine trip, writer table); RUL ruling 1.

**Files** (at `431c3ad`, **verify first**):
- The fallback calls (`movement.rs:1893`, `:2053`) make a pool of one with `spread_destinations_with_context`.
- Delete:
    - `step_plain_progress` (`:671-905`);
    - `make_plain_progress` (`:925`);
    - `PlainMoveProgress` (`:58`);
    - the plain branch (`:1843-1879`);
    - `orders/plain.rs` (the plain reservations and identities; TDI `:92`). The plain spread context at `:1885-1960` is NOT in the delete list (RUL `:88`).
- `PlainGatherGoal` (`:63`, `:79-82`) moves into `MoveProgress`.
- `AuditState.plain_movement_progress` (`views.rs`) goes. `LedgerAudit` and `el1.rs:188-215` drop it (D `:178-179`).
- Unit tests `movement.rs:3471-3700` (`plain_movement_tests`) are deleted. The reason: the model is gone. Two cases move to the pool-of-one path and are kept as tests:
    - `unobstructed_step_keeps_the_legacy_target_until_within_one_step` (`:3527`);
    - `plain_move_respects_reserved_spacing_before_using_a_zero_primary_point` (`:3618`).
- Add `crates/turts_sim/tests/one_step_function.rs::no_second_step_model`: the source of `crates/turts_sim/src` holds no `fn step_plain_progress` and no `PlainMoveProgress` (R7).

- [ ] **Step 1: Red.**
    - `no_second_step_model`: red on the base, with the message `step_plain_progress still defined at <path>`.
    - `group_movement.rs::two_gatherers_deliver_to_one_receiver_in_one_tick`: both deliver; no footprints overlap; each delivers within `2 × (distance / speed) + 5 s`. Add a second case: a delivery after node depletion (D `:158`).
    - Run: `cargo test -p turts_sim --locked --test one_step_function --test group_movement; echo exit=$?`. Expected: `no_second_step_model` red, `exit=101`. The gatherer guard can be green on the base; its red is by plant (step 4).
- [ ] **Step 2: Green.** Make the edits. Rerun: `exit=0`.
- [ ] **Step 3: Commit, gate.**
    - Commit. F-RUST, F-FROZEN. The stream must stay unchanged: Q1b counted 0 unit-ticks on `PlainMoveProgress`. A red stops the task (D `:146`).
    - `--p0` on four packages against the A2b base: `exit=0` (D `:200-201`).
    - EL1 (A1b command): `exit=0`.
- [ ] **Step 4: Plant.** The fallback returns no destination (the unit stays `Blocked`). `two_gatherers_deliver_to_one_receiver_in_one_tick` goes red on the deadline. Restore.

**Exit:** no plain model. The stream and P0 are unchanged. **Runs: 3.**

### Task A2c: the plain move and `MoveTo` take the pool path

**Seat:** `simulation-core-engineer`. Review: `qa-lead`.

**Branch / inputs:** `pre-mp1/a2c-pool`; A2b, Q1b (the allowed list committed). If G1 held its commit (G1 step 5), cherry-pick it here. **Design:** D section 2 row A2c, section 6 (guards); GSIM M3 (the cap).

**Files:**
- `orders/apply.rs`: the plain branch (`orders.rs:2822-2900` at `431c3ad`) takes the deferred path (`:2772-2820`).
- `command.rs:488-495`: `CommandKind::MoveTo` maps to an envelope with a pool of one (`queue_legacy_move_to_envelope`, `orders.rs:551`).
- Tests (dispositions; each one **verify first**, and each reason goes in the commit message):

| Test | Disposition |
| --- | --- |
| `crates/turts_sim/tests/orders.rs:496` `accepted_plain_move_projects_without_unit_orders_and_legacy_move_has_no_identity` | Invert: the pool path writes order operations (`orders.rs:2874`) |
| `orders.rs:988` `plain_move_writes_no_order_operation` | Invert (TDI `:123`) |
| `group_movement.rs:282` `unopposed_authored_singleton_keeps_its_command_only_audit_shape` | Invert |
| `orders.rs:555` `stopped_or_removed_plain_move_identity_does_not_leak_into_the_view` | Rewrite for the pool collection. Plain identities are gone since A2b |
| `orders.rs:330`, `:383`, `:607` | Keep (queue rules) |
| `group_movement.rs:109`, `:325`, `:1895`, `:2005`, `:2156`, `:2757`, `:2876` | Keep. `:2005` and `:2156` are renamed (no "plain") |
| `crates/turts_server/tests/orders_plain.rs:118` `plain_move_envelope_has_the_moveto_operation_count` | TDI says "inverts". If both paths gain the same operations, the test stays. Record which |
| `command.rs:341` `legacy_move_to_clamps_each_edge_and_corner` | Keep |
| The `MoveTo` users in `crates/turts_pack/tests/economy_ordering.rs` and `economy_accounting.rs` | Keep. A red is a difference that the classifier must explain |

- [ ] **Step 1: Red.** In `group_movement.rs`:
    - `second_move_to_a_held_point_keeps_the_first_unit_on_it` (gate case S12). Unit 1 rests on P, bitwise. Unit 2 rests at least `s - 0.00001` from P (`s = 2 × r_max + 0.1`). No footprints overlap.
    - `one_unit_alone_takes_the_point` (`:1309`) gains four cases: through `MoveTo`; through an envelope with group 0; with a static unit of radius 4 far away; to a point off the map (the clamped point, bitwise).
    - `single_move_pool_is_collected_on_arrival`: 0 `MoveProgress` and 0 pools after arrival (`movement.rs:2383-2404`, `:2497-2508`).
    - `three_thousand_single_moves_spread_over_two_ticks`: 2,048 units start at T and 952 at T+1 (GSIM M3).
    - Run: `cargo test -p turts_sim --locked --test group_movement; echo exit=$?`.
    - Expected: S12 red (the first unit gives up P, GSIM `:78`); the cap test red; `exit=101`. Record the run (D `:153`).
- [ ] **Step 2: Green.** Make the edits. Rerun: `exit=0`.
- [ ] **Step 3: Commit, gate.**
    - Commit. Do not touch the fixture (D `:121`).
    - F-RUST, without the stream test.
    - G-FROZEN is red by design: `m2_operation_stream_matches_baseline` fails. Record its first difference and hand over to Q2.
    - G-WEB is not run in the lane. T7 runs in the slow set after Q2.
- [ ] **Step 4: Plant.** `free_slot_grid` always false. `one_unit_alone_takes_the_point` goes red. Restore.

**Exit:** the guards are green. The stream is red only at the stream test. Hand-over to Q2. **Runs: 3.**

### Task Q2: the one re-freeze

**Seat:** `qa-lead`. `technical-director` reviews the classified difference (one run).

**Branch / inputs:** `pre-mp1/q2-refreeze`, on top of `pre-mp1/a2c-pool`; A2c. **Design:** D section 5 items 3 to 5, `:131-146`.

- [ ] **Step 1: Traces.** Run the emitter (Q1b step 1 command):
    - on the A2c base, into `target/pre-q2/evidence/before`;
    - on the A2c head, into `target/pre-q2/evidence/after`.
- [ ] **Step 2: Stream.** On the A2c head, run `UPDATE_M3_BASELINE=1 cargo test -p turts_pack --locked --test m3_m2_stream m2_operation_stream_matches_baseline; echo exit=$?`. The variable is read at `m3_m2_stream.rs:386`; D says `:384`. Then `sha256sum tests/m3/m2_operation_stream.json`. Copy the file to `target/pre-q2/evidence/stream.json`.
- [ ] **Step 3: Classify.**
    - `python3 tests/m3/refreeze_classify.py --self-test; echo exit=$?` gives `12/12`, `exit=0`.
    - Then `python3 tests/m3/refreeze_classify.py --a2c target/pre-q2/evidence/before target/pre-q2/evidence/after --allowed tests/m3/refreeze_allowed.json --stream target/pre-q2/evidence/stream.json; echo exit=$?`.
    - Expected:
        - P0 equal for base, haul and cedar;
        - P1 within the allowed list;
        - P2 the full events on the same ticks;
        - P3: one operation added at tick 1 and one at the tick after arrival, both for the acme scout, with no event kind;
        - the arrival tick equal to `ceil(|P - p0| / (speed × dt))`. Where the speed and `dt` are defined is NOT TRACED; the classifier prints the inputs that it used;
        - the counts asserted;
        - the emitter sha256 equal;
        - `exit=0`.
    - Any difference outside P3 stops the task (D `:146`).
- [ ] **Step 4: Evidence.** Put the traces, the stream, the classifier output and `SHA256SUMS` under `docs/research/mp1/probes/refreeze/`. `sha256sum -c SHA256SUMS; echo exit=$?` gives `exit=0`.
- [ ] **Step 5: Technical-director review.** `technical-director` reads the classified difference and the S12 guard. Verdict on the board.
- [ ] **Step 6: CEO touch point 2.** Root writes the vault note with the old and new sha256, the classifier output and the player-visible change. The CEO approves.
- [ ] **Step 7: One commit.** `tests/m3/m2_operation_stream.json` (`sha256sum` equals the classified stream, D `:141-142`) and `tests/m3/refutations.py:18` (the new hash).
- [ ] **Step 8: Gate.** F-FROZEN with the new hash, `exit=0`. A2c and Q2 then merge into `pre-mp1/integration`.

**Exit:** the new hash is committed, approved, and equal to the classified stream. **Runs: 3.**

### Task A4: the frame as functions on `&mut FrameState`

**Seat:** `simulation-core-engineer`. Review: `performance-architect`.

**Branch / inputs:** `pre-mp1/a4-frame`; A2b. Merge only after Q2 (R3). **Design:** D row A4; GSIM L1; RUL `:71`.

**Files:** `movement/frame.rs`. The rest of `step_frame_preserving_resting` (index build, escapes, write-back, resting collection) becomes functions on `&mut FrameState`, called in one loop in unit-id order. TDI phase ranges `:165`: **verify first**.

- [ ] **Step 1: Reference.** Refresh `movement/reference.rs` from the A4 base.
- [ ] **Step 2: Red by plant before the change.** Swap two phases in the real kernel. `kernel_reference` goes red. Restore.
- [ ] **Step 3: Change, then green.** `cargo test --release -p turts_sim --locked --features test-probes --test kernel_reference; echo exit=$?` gives `exit=0`.
- [ ] **Step 4: Commit, gate.** F-RUST, F-FROZEN (after the rebase onto Q2), `--p0` on four packages.
- [ ] **Step 5: Plant.** Reset one `FrameState` field between units. `kernel_reference` goes red. Restore.

**Exit:** bit-equal, and the stream unchanged at the new hash. **Runs: 2.**

### Task A3: two neighbour grids, combat cell

**Seat:** `simulation-core-engineer`. Review: `performance-architect`.

**Branch / inputs:** `pre-mp1/a3-grids`; A4, G1. Merge only after Q2. **Design:** D row A3; TDI A3 (`:138-160`); audit `:33`.

**Files:**
- `movement/frame.rs`: movers and statics grids (`speed` None, `movement.rs:1414`), with `within_pair` merged and sorted by `(id, index)`.
- `economy/combat.rs:111-117`: cell = the smallest weapon range.
- `orders/run.rs` (engagement grid `orders.rs:722-728`; today `fold(f32::EPSILON, f32::max)`): cell = the smallest weapon range.
- `refresh_slot_clearance` (`movement.rs:1110-1117`) caps at the largest mover radius (TDI `:144-146`).

- [ ] **Step 1: Before.** Run the RUN126 probe (1 static of radius 4; 5,000 movers; audit `:33`: 16.7 ms to 56.2 ms) and record the time. Where the probe source is, is NOT TRACED. If it is not in the tree, A3 builds it again from audit `:33` and records it.
- [ ] **Step 2: Reference.** Refresh it from the A3 base. Change, then green: `kernel_reference` (1,000 to 5,000 movers) `exit=0`; `within_matches_brute_force_scan` `exit=0`.
- [ ] **Step 3: Commit, gate.** F-RUST, F-FROZEN (new hash). GM40, GMA, GMU (A2a step 4 commands). `--p0` on four packages. Run the probe again and record the time; it is not judged.
- [ ] **Step 4: Plants.** Each goes red, then restore:
    - (a) query the statics with reach `r` only: `kernel_reference` goes red;
    - (b) a merge that is not sorted: `kernel_reference` goes red.

**Exit:** bit-equal and the probe time recorded. **Runs: 3.**

### Task REG: slice regression

**Seat:** `qa-lead`. Inputs: Q2, A3, S45, A1d.

- [ ] **Step 1.** The full slow set of section 4 on the final `pre-mp1/integration` head, every item including the headed rows.
- [ ] **Step 2.** The test list of the head against the list at `8d6902c`. Every name that is added or removed maps to a task line in this plan.
- [ ] **Step 3.** Record the receipts (head, clean source, source hashes) on the board.

**Exit:** all green with receipts. **Runs: 2.**

### Task TDN: architecture note

**Seat:** `technical-director`. Inputs: REG. The seat reads the diff `8d6902c..pre-mp1/integration` and runs:
- `bash scripts/boundary.sh`;
- the `MoveTarget` writer grep against the allow list of A1b;
- `wc -l` of the former hot files.

It writes the note text for Root, who saves it as `docs/research/mp1/arch-note-pre-mp1-slice.md`. The note says what MP1 lane 0 (D section 7) can build on. **Runs: 1.**

## 8. Risks and early checks

| # | Risk | Early check (day 1) |
| --- | --- | --- |
| 1 | The merge of the depletion repair conflicts with the S1 move. The repair edits validator code that S1 moved. | D0p step 1 trial merge in a scratch worktree, as soon as D0 lands. |
| 2 | An orphan display holds the GPU lock and blocks every headed run. It happened at c667b08 for 36 minutes. | Before each headed batch, confirm that no headed run is active. Root asks the CEO to stop an orphan; Root does not go round a permission refusal. |
| 3 | The stream units are not all on the no-progress branch, so the A2b claim "stream unchanged" fails. | Q1b step 2 counts on the base build. |
| 4 | The G1 counter has hits on the stream, so G1 moves into A2c and the trace needs `edge_hit`. | G1 step 4, right after D0p. |
| 5 | A2a is not bit-equal for pooled units, because of the float expression (`:2003-2004` against `:2183-2185`). Many GM tests would change. | A2a step 2: the count of differing pooled seeds, before the rest of A2a. |
| 6 | A1d changes an M3 failure text, so an M3 fault stops being red for its text. | A1d step 1 `grep -F` of each text, before the replacement. |
| 7 | R2 fails: A2a must change a movement item that the replay validator uses. | `cargo check -p turts_audit --locked` at the first A2a commit. |

## 9. Budget

- Remaining: **49 runs** in total (section 3 table); 24 runs on the critical path. Root counts each dispatch.
- The early producer input estimated the slice at 11 to 18 runs (`docs/research/mp1/input-producer.md`, part 3).
- The design grew after that: G1, four splits, the trace, the classifier, A4 and the A1 amendments. So 49 is the estimate at the MC rate. It is not a promise.

## 10. Not traced by the producer

- How `tests/boundary-fixture` selects its rows.
- Which browser case reads `facing`.
- Where the speed and `dt` of the arrival-tick check are defined.
- The source of the RUN126 probe.
- Whether the `mc_orders.rs` tests are `#[ignore]`.
- The flags of the headed wrapper beyond its `--help` text.
- Whether `vault/Pre-MP1 Execution Board.md` exists.

## 11. Doc updates for the orchestrator

- MC design, row EL1: new text "the ledger validator accepts each tick, and the run records the log bytes" (D `:108-109`).
- `CLAUDE.md`:
    - a `turts_audit` row in the crate layout, with its forbidden dependencies (D section 3);
    - the `turts_interaction` row (audit `:38`);
    - the movement and orders folders.
- D `:142`: the line of `UPDATE_M3_BASELINE` is `m3_m2_stream.rs:386`, not `:384`.

## 12. What I changed

None. Scratchpad: /tmp/claude-1000/-home-julien-Projects-TuRTS/bf418660-b272-4bb3-8628-8d9ef2677d1d/scratchpad/slice.py (the graph model).

## 13. What I ran

| Command | Claim tested | Result that would refute | Observed |
| --- | --- | --- | --- |
| `sed -n` on the line anchors at `431c3ad` | D and TDI line numbers | Another item at the line | Match. One drift: `UPDATE_M3_BASELINE` is at `:386` |
| `git merge-base`, `git log` on the lanes | The lanes start from `codex/mc-integration` | Another base | Both at `8d6902c` |
| `git diff -U0 8d6902c 43282fe` | The repair touches the moved validator | No hunk in the validator | Hunks at `:3470` and `:5541` |
| grep of `turts_sim::movement::` in `economylog.rs` | R2: the validator uses a small API | It uses the step branches directly | 9 items, listed in R2 |
| JSON scan of both registries | Which plants S2 and S45 must move | No plant in the moved files | `orders.rs` 1 (E8-foreign); `visuals.rs` 2 |
| `slice.py` | The graph has no cycle; critical path | A cycle | No cycle; 24 runs, 28 if R2 is refused |

## 14. Open questions for Root

- Accept or refuse R1 to R9 at gate 2.
- The re-check of the simulation lens is still open (D `:3`). This plan assumes that it passes as revision 4.
