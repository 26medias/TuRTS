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

## Binding amendments after the re-check of revision 2

Design revision 6, section 9 items 17 to 23, is binding on this plan and replaces the plan lines it names:
the `Blocked` pool of a fallback keeps `Blocked` (17); the A2b1 harness is bit-equal only on seeds with no
fallback (18); the collection rule reads `MoveProgress.gather_goal` (19); the `test-probes` proof in Q1b (20);
the stricter assertion rule of section 1a (21); the source counts count code only (22); the M4 start point
(23). Root accepts refinements R10, R11 (with item 20), R12 (with item 22) and R13 (with item 17). The plan
gate is passed for the tasks up to and including A2a. A2b1 gets a short re-check before its builder starts.

**From the A2b1 stop ruling (2026-11-12, technical-director; `docs/research/mp1/ruling-slice-a2b1-stop.md`).** Binding for A2b1 to A3 and for the orchestrator's gates:
- CEO touch point 1 (section 6) was NOT sent before A2b1 started (an orchestrator fault). It is sent on 2026-11-12. A2b1 does not merge before the answer.
- The two probe values of `scripts/qa/probe_neutral.sh` change at A2b1 by design (D 9.18 `:255-258`, D 9.8, D `:44`). Values at the A2b1 head f119dc39: `frame_chains=e6e4417f9cbe5fa5`, `muster_world=49fc63ffd1d8c57d` (before: d0d1b1c572163424, aa9db740578ed5c1). Each task compares with the values of its own base commit. A2b2 commit 1 and A2c may change them; A2b2 commit 2, A4 and A3 must keep them equal. The frozen stream sha256 is the only value that needs the CEO, at Q2.
- The five test changes outside the A2b1 list (`plain_movement_tests.rs:187`, `:394`; `orders/tests.rs:2692`; `frame_hash.rs:211`; `probe_flags.rs:117`; `ledger_logs.rs:530`) are allowed observation changes or inversions under the D 9.21 conditions. The plan names them here.
- From A2b2 on the 9.18 cut-off of `kernel_reference` is void: the live and reference fallback requests must be bit-equal and all ticks of all 128 chains are compared. The present harness fails at `kernel_reference.rs:904-917` when the reference is refreshed. `plain_movement_tests.rs:394` is a third port at A2b2.

**From the A1d ruling (2026-11-12, technical-director; `docs/research/mp1/ruling-slice-a1d.md`; review `docs/research/mp1/review-slice-a1d.md`).** Binding for A1d and for the orchestrator's gates:
- Ruling (ii). `crates/turts_pack/tests/common/conservation.rs` stays in the test tree as the independent source oracle. D `:73-74` is replaced by D section 9 item 25. The A1d exit "one oracle" is void.
- A1d keeps from `a921096b`: `assert_run_constants` (`crates/turts_server/tests/economy.rs:138-156`) and its two call sites, the removal of the `bevy*` dependencies of `turts_audit`, the three `A1d-run-constants-*` rows (plants moved to real state), and their test in `tests/m2/test_refutations.py`.
- A1d drops: `crates/turts_audit/src/source.rs`, `pub mod source`, `ron` in `turts_audit`, the `turts_audit` dev-dependency of `turts_pack`, and the import edits.
- Review M1 is fixed in A1d by a writer scan in `turts_checks` for `MapBounds`, `ProductionDurations`, `UnitTypes` and the removal of the resources of `AuditResourcePresence`. The scan visitor gets a `ResMut<T>` / `Mut<T>` / `NonSendMut<T>` arm, and the movement writer scan gets the same arm. No log format change.
- The no-content scan does not see compound names (`ore_items`): a recorded gap, owner `qa-lead`, before the first MP1 package with new content ids merges.
- The `tests/m2/refutations.json` conflict with `pre-mp1/a2b1-fallback`: the task that merges second keeps both row sets.

**From the A1c re-check (2026-11-12, qa-lead; A1c merged with 0 High; report `docs/research/mp1/rereview-slice-a1c.md`).** These bind A2b1 (its first steps) and A1d:
- A2b1: the sweep test `movement::ecs::tests::the_movement_system_writes_only_movement_state` does not reach the physical-escape branch of the write loop (`crates/turts_sim/src/movement/ecs.rs:113-115`): the reviewer's plant S2 (a unit in a physical escape is given to slot 1) stays green there and is red only by chance in `group_movement.rs:2230`. Add a blocked plain move to the sweep scenario and a count that asserts that each branch of the write loop runs; show S2 red in the sweep.
- A1d: read `docs/research/mp1/a1c-handover.md` on the slice branch (the log-based accounting oracle cannot see a change of `bounds`, `resources`, `compiled_durations`, `unit_types`; the E8 reorder; the unused `bevy*` dependencies of `turts_audit` with their three `Cargo.lock` lines).
- The parallel registry runner `target/mc-evidence/root-regate/parallel_refutations.py` still uses the headed wrapper v4 by default (`:42`) and passes no `--mode` (`:274`). Until it is changed, a headed registry run goes through a shim that calls `mc_headed_v6.sh --mode pre-merge`. Owner `qa-lead`.

**From the A2a review (2026-11-12, qa-lead, six own faults; 1 High in the harness reach, in repair; report `docs/research/mp1/review-slice-a2a.md`).** These bind A2b1, A2b2, A2c, A4, A3 and REG:
- The fast form `cargo test -p turts_sim --locked` runs 0 tests of `kernel_reference` and of `probe_flags` (the `test-probes` feature is off). Each of these tasks runs `cargo test -p turts_sim --locked --features test-probes` and the release form of `kernel_reference` that its test header names; the merge gate of the orchestrator runs both.
- `reference.rs` calls live helpers. A task that edits a movement helper must first keep the reference independent (the A2a repair says how).
- From the A2a re-check (`docs/research/mp1/rereview-slice-a2a.md`; A2a merged with 0 High): the shared helper `angle` (`movement/mod.rs:123-125`, called through `spread.rs:236`) has no pin; a negated `angle` stays green in `kernel_reference` and in `group_movement`. The first step of A2b1: pin `fn angle` and `fn step` by item (a hash of each function text, not of the whole `mod.rs`), red first on a negated `angle`. In A2b1, A2b2, A4, A3 and REG: `git diff <task base> HEAD` on the pinned files and items is empty, or the same commit puts the old text under `movement/reference*` and makes the reference use it (A3 edits `grid.rs`, so A3 is the first task that copies a helper). The reviewer of each task checks this.
- The `probe.rs` sha256 logged at plan `:80` (`aaa45aaa…`) is now `a0e861062cb9bebf7528dad354da1d5ef00bc108c05ec3e27dc7beb1b60a0295` after A2a; it changes again with the A2a repair. No checker pins it; the board log carries it.

**From the A1b review (2026-11-12, qa-lead, five own faults; A1b merged with 0 High open; report `docs/research/mp1/review-slice-a1b.md`).** Task A1c must do these before it deletes the replay validator:
- Ruling (orchestrator): add `pending_movement_orders` to `LedgerAudit` (`ledger_view.rs:38-46`); the sim names it audited command state (`orders/pending.rs:32`). A refused command that changes `next_pool_id` must be red in the ledger (the reviewer's probe F4 is the plant).
- The movement-writers allow list (`crates/turts_checks/tests/movement_writers.rs:49-98`) pins the count of each (file, kind, type) entry, so a new writer in an allowed file is red (plant: a `get_mut::<Position>` write in `combat.rs`).
- With the replay off, three replay self-checks are red with no plant: `economylog.rs:453` (delete; other tests keep the property), `economylog.rs:331`, `economy.rs:1145`. A1c moves or replaces all three, not only the 1-ulp test.
- The field test in `ledger_view.rs:134-198` runs on a Forge `Tick` state, so a field that is empty in the small fixture is checked.
- Plant (b) "remove facing" moves to A1c (the sink writes `AuditState` until then).
- Validate one time for each poll result, not the whole prefix two times (`economy.rs:2455-2470`, `:2502-2503`, `:552-553`).
- The run receipt records `TURTS_AUDIT_REPLAY`.
- Owed from A1b: the 8 `ci-browser` cases (headed) and EL1 and E9 in release with the replay off.

**From the A1a review (2026-11-12, qa-lead, two rounds; A1a merged with 0 High open).** Task A1b must do these:

- The destination check of `queued_group_attack_spreads…` (`crates/turts_server/tests/mc_orders.rs`, `first_logged_destinations`)
  uses a radius of 1.2 and does not see a shift below about 0.6. Also assert that the nearest destination is the order
  point (slot zero, `movement.rs:293`), after a check that slot zero is free in this fixture.
- The message of "gives every member a destination" must say that the destination comes from `raw_units.move_target`
  and that a change of the movement model (part A2) means an update of the helper, not a sim defect.
- `assert_sink_record_order` (`crates/turts_server/tests/economy.rs`) must also assert that the boundaries and the
  operations of a batch come before its `Tick`.
- Weak checks after the validator goes (leads): `mc_orders.rs:634` (pursuit and range: a focus target hit from any
  range stays green today, with the validator on or off); `economy.rs:299` and `:547` (`decoded_*`); `economy.rs:3425`,
  `:3780`, `:3925` (`replay_public_log`, which validates at `:1998`).
- `read_live_complete_tick_prefix` still calls `validate_records`. Task A1c removes that call and runs the A1a plants again.
- The A1a exit "E9 green five times in a row" moves to task D0p: one run in six was red at `economy.rs:2306` from the
  gather-at-depletion defect, whose repair is not in the slice base before D0p.

**From the S3 review (2026-11-12, technical-director; S3 merged, 0 High).**

- Task S3 Step 4(b): the named test `forty_units_rest_with_no_overlap_and_no_blocked` stays green under the named plant on
  both builds. The test that goes red is `second_group_same_point` (`crates/turts_sim/tests/group_movement.rs:1181`), with
  the plant `let push = Vec2::ZERO;` at `movement/frame.rs:835`. Later tasks that reuse this fault (A2a, A2b2, A4) name
  that test.
- Task S3 Step 2: the count regex is `grep -cE '^(pub(\([a-z]+\))? )?(fn|struct) '` (it gives 3 on `plain.rs`).
- Stale `movement.rs:NNN` citations stay in comments (`crates/turts_sim/tests/gmu_same_point.rs:6`,
  `crates/turts_server/tests/mc_orders.rs:578`, `tests/m3/README.md:110`, `tests/m3/refreeze_classify.py:78`) and in the
  A1b amendment above (`movement.rs:293` is now in `movement/spread.rs`). The next task that edits each file corrects it.
- `step_frame_preserving_resting` is one function at `movement/frame.rs:60-1078`. A2a and A4 work in `frame.rs` only;
  Q1b adds `movement/probe.rs`. The use-cycles frame/plain, frame/search and frame/pool_index come from
  `MovementFrameUnit` and `MovementFrameOutput` in `frame.rs`; A4 can move those types to `mod.rs` (optional).

**From the Q1b review (2026-11-12, simulation-core-engineer, two rounds; Q1b merged, 0 High).**

- The plan text of Q1b Step 5 is void: the stream cannot see `separated`. The red instrument for the probe flags is
  `crates/turts_sim/tests/probe_flags.rs`. The plan count "13/13 plants" is void: the classifier self-test has 74 plants.
- `was_projected` is `Some(value)` in all three loops and `null` only for a unit that did not reach a projection loop
  (this replaces "null for a pool unit"). `gather_stage` is a trace field (format /4).
- Hashes to log before A2c (R6): `tests/m3/refreeze_allowed.json` a431dbd826cbbd3e130b9a8820911711c41fa4a8d7efa82dca90d49214165bc7;
  the emitter `crates/turts_pack/tests/m2_trace.rs` e5152ece49414eaf46800e1c0d29f7144ef38c51531336bdb9b84a779988f924;
  `crates/turts_sim/src/movement/probe.rs` aaa45aaad4d8deea4526311fba1d01c12cd660130345e6f8219356322c2a4cae.
- After D0p merges, and before A2a starts: emit the base trace again and log its sha256. The depletion repair changes
  `gather.rs`, and `gather_stage` may move at node depletion with the stream sha unchanged (NOT TRACED).
- What the Q2 approval can rest on: single-unit moves, gather timing and the order of economy operations, bit for bit
  (three packages byte-equal; acme differs only as listed, with equal positions). What it cannot rest on: crowds. On
  the stream no unit separates, is projected, claims a slot or moves in a group. Those behaviours are held by the A2a
  kernel reference, `probe_flags`, `group_movement`, `gmu_same_point`, `mc_group` and T7. The Q2 request to the CEO
  gives both parts.
- `scripts/qa/probe_neutral.sh` (CI step) holds the sim equal with the probe feature off and on; `scripts/boundary.sh`
  has a no-probes row for the server and client binaries. A task that adds a gated probe line runs both.
- Low, open: `per_unit` against `envelope` in `move_to_route` comes from the command shape; a gather carry error below
  5 is invisible on the stream content.

**From the re-check of MP1 plan revision 2 (2026-11-12, technical-director H-A and M-A; MP1 plan revision 3,
rulings R-W and R-I).** Tasks A3, A2b1, A2b2 and A2c must do these:

- **A3 builds stacked on A4** (TD option P-b). A3 branches from the head of `pre-mp1/a4-frame` when the A4 build ends,
  before A4 merges (G1 is merged). This replaces "A4, G1" as plain inputs (section 3 row A3) and the Task A3 line "It rebases on
  A2c": A3 takes A4 and A2c into its branch with merge commits, not with a rebase. A3 still merges only after Q2 and
  at least 1 run after A4. If A4 must rebase after A3 branched (the R3 case: A2c edits a `movement/` file), A4 merges
  `pre-mp1/integration` in place of the rebase, and A3 merges the new A4 head.
- **A3 merges into `pre-mp1/integration` with a merge commit, not a rebase**, so the A3 commits keep their ids.
- **Two MP1 tasks merge the A3 branch.** MP1 tasks `L-TGT` and `L-OBS` take A3 as a cross stack: each branches
  from `mp1/integration` and merges `pre-mp1/a3-grids` into its own branch at the end of the A3 build. So a rebase
  or a history rewrite of `pre-mp1/a3-grids` after the A3 build ends breaks those two lanes. A review repair of A3
  goes on top as a new commit; the A3 builder tells Root, and Root tells the owners of `L-TGT` and `L-OBS`.
- **The order writer rule (for A2b1, A2b2, A2c and A3).** MP1 adds one writer of `UnitOrders`,
  `set_unit_orders` (MP1 plan, task `L0-WIRE`), and a check that is red on any other production write: a syn scan
  (`crates/turts_checks/tests/order_writers.rs`) of `get_mut::<UnitOrders>()`, `remove::<UnitOrders>()`, `insert` of
  a `UnitOrders` value, a field assignment, and a helper that takes `&mut UnitOrders` or `&mut Order`; and a
  `test-probes` system in `SimSet::Collect` that compares `Changed<UnitOrders>` and `RemovedComponents<UnitOrders>`
  with the change list. The slice does not have this code, so no slice gate changes. A slice task that adds or moves
  a write of `UnitOrders` names each such line (`path:line`) in its report. At the forward merge, the MP1 owner
  (`network-engineer`) routes it through `set_unit_orders` (about 1 run for each of A2b1, A2b2 and A2c).

**From the D0p and S2 reviews (2026-11-12; both merged, 0 High).**

- D0p: the plan block was out of date (three validator hunks, eleven test hunks, nine `economy.rs` hunks; one semantic
  conflict that git merged without a mark: two names in `crates/turts_server/tests/economy.rs` that S1 had moved). The
  port is verified hunk by hunk (16 of 16 found in `crates/turts_audit/src/replay.rs`; no extra edit). E9 was green six
  times in a row after the repair, so the moved A1a exit is met.
- A1b: the guard `!audited_gather_delivers_cargo` at `crates/turts_audit/src/replay.rs:3251` has no negative test: with
  its helper forced to `false`, every test stays green. Add one test that edits a real depletion log so the order fails
  while cargo is held, and assert that the audit rejects it. Keep this property in the ledger audit.
- The base traces after D0p (and unchanged by S2), to compare before A2a: `base.jsonl`
  d4c73ebee90e0aef84e58df4e31787ef3756b7e5d3a589ffdde2224ccfcdd7b2, `haul.jsonl`
  805309b26e717dfe3daa0e5597ebd6f21adde7ef3f93ec71a3610bd97bff3148, `acme.jsonl`
  19dcf9a3759096c312c11a8e89c2c0ac272dd3ccd868aacf9ac8a4357b91d1d2, `cedar.jsonl`
  0db10e012cd1c3a41c68fb054a7ab39ec325451f4db4fb912f72dbddd5f33718. The 400-tick traces never reach a node depletion,
  so they do not test the depletion repair; its own tests do.
- S2 layout, for A2b1, A2b2 and A2c: the files are `orders/{mod, plain, pending, run, apply, tests}.rs` and
  `orders/behaviours/{attack, follow, queue, mod}.rs`. `plain.rs` is what A2c deletes. Four functions are the meeting
  points of the later tasks: `run_orders` (`run.rs:21-416`), `dispatch_pending_movement` (`pending.rs:775-1306`),
  `apply_order` and `apply_unit_order` (`apply.rs`), `start_order` (`behaviours/queue.rs`). The use graph has one cycle
  group {apply, pending, plain, queue, attack}: the A2c deletion of `plain.rs` edits pending, apply and queue together;
  count it as one change.
- S2, the review fault for later tasks: `return;` as the first statement of `start_next_queued_order`
  (`orders/behaviours/queue.rs`) turns 12 tests of `turts_sim` red on both builds (4 in `orders::tests`, 6 in
  `group_movement`, 2 in `tests/orders.rs`, with `queued_order_starts_in_the_tick_after_landing`).
- S2, Low: `queue_movement_envelope` (`orders/pending.rs:295`) is `pub(crate)` with no re-export; the next task that
  edits `pending.rs` makes it `pub(super)`. Stale comment citations of `orders.rs` stay in
  `crates/turts_sim/tests/gmu_same_point.rs:198`, `crates/turts_sim/src/movement/probe.rs:127`, `tests/m3/README.md:103`,
  `tests/m3/refreeze_classify.py:171`, `tests/m3/test_refreeze_classify.py:175`.
- The MP1 branch carries a formula ratchet (`crates/turts_checks/tests/formula_ratchet.rs`) with a per-file table. A slice
  task that moves or splits a counted file names the new rows in its report; the forward merge writes them.
- The writer rule of MP1 lane 0 (one function `set_unit_orders` for order-state changes and a static check that forbids a
  direct write of `UnitOrders` outside it) lands on the MP1 branch, not on the slice. A2b1, A2b2 and A2c keep their
  direct writes on the slice; the MP1 owner of each forward merge moves the new writes into the writer function.

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

## 1a. Rule for assertions and expected values

A builder never changes an assertion or an expected value on his own decision. There are four permitted kinds of change, and this plan names each one.

1. **Inversion.** A design decision changes the behaviour. The plan names the test and its new expected value. The source of the value is D, or a gate probe.
2. **Observation change.** The type that the test reads is deleted. The test reads the replacement field. The expected value stays the same.
3. **Deletion.** The tested item no longer exists. The plan names the test, and D or TDI gives the reason.
4. **Registry re-pin.** D names the new text.

Any other assertion change, and any extra red test, stops the task. The builder reports it to Root.

Each place in this plan where an assertion or an expected value can change:

| Place | Kind | Why it stays, or how it is tightened |
| --- | --- | --- |
| S1 step 1, test names | tightened | The builder lists the seven writer tests that stay in `economylog.rs` by name on the board before the compare. The compare must give exactly those seven names, and no other difference. |
| A1c: E8-mixed `expected_failure` to "rejected seq 5 is write neutral" | re-pin | D section 9 item 2 names the text (`economy.rs:3197-3201`). |
| A1c: delete `economy.rs:486-564` (a position moved by 1 ulp) | deletion | TDI `:67`. The movement tests own positions. |
| A1c: `el1.rs`, the two void EL1 faults | deletion | D section 4 item 5. |
| A1d: a changed M3 failure text | tightened | No registry edit in A1d. If a text would change, A1d stops. |
| A2a: "each changed assertion has a reason" | removed | D 9.9: A2a changes no assertion. |
| A2b1: eight tests that read `PlainMoveProgress` | observation change | The mapping is in Task A2b1. A pinned fallback slot gets a new value only from an independent calculation in the test (MC design 6.3 hex ring), never from a run. |
| A2b2: `plain_movement_tests` (`movement.rs:3471-3700`) | deletion, plus two ports | The tested model is gone. The two ported tests keep their expected values. |
| A2c: four tests on the audit shape of a single move | inversion | The new values come from D section 5 P3 and the gate probe (operations at ticks 1, 1 and 124; GSIM `:61`). |
| A2c: tests that read `PlainMoveOrderIdentities` | observation change | They read `UnitOrders`. The order id stays the same. |
| A2c: `orders_plain.rs:118` | no change | Both paths take the pool path, so their counts stay equal. A red stops A2c. |
| A2c: the `MoveTo` users in `economy_ordering.rs` and `economy_accounting.rs` | no change | A red stops A2c. |
| A2c: renames of `group_movement.rs:2005` and `:2156` | removed | No rename. Then the test-name compare in REG stays clean. |

## 2. Plan refinements of D

Root accepted R1 to R9 after gate 2. R2 carries the gate condition. R10 to R13 are new. They need Root's acceptance at the re-check.

| # | Refinement | Reason (evidence) | Cost if refused |
| --- | --- | --- | --- |
| R1 | G1 starts now and runs in parallel with S1, S2 and S3. Its stream counter run (step 4) waits for D0p. | G1 edits only `grid.rs` (`grid.rs:61-70`). The counter must see the trips of the depletion repair (GSIM L2). | none on the path |
| R2 | A2a runs in parallel with A1b and A1c. A2b1 waits for A1c. Condition (gate 2 L2): `cargo test -p turts_audit --locked` and EL1 pass on the A2a branch. If either fails, A2a waits for A1c. | The replay validator uses `step_frame_preserving_resting`, `MovementFrame`, `MovementFrameUnit`, `MoveProgress`, `PlainMoveProgress`, `PlainGatherGoal`, `spread_destinations`, `FollowMoveOrigin` and `MovementActivity` (grep of `turts_sim::movement::` in `economylog.rs` before `:8084`). A2a keeps their signatures, and its output is bit-equal. | +4 runs (path 27 to 31) |
| R3 | A4 starts after A2b2, in parallel with A2c and Q2. A4 and A3 merge only after the Q2 commit. Each runs G-FROZEN again with the new hash. | A4 edits only `movement/frame.rs`. The A2c file list holds no `movement/` file. If A2c must edit a `movement/` file, A4 rebases after A2c. | +5 runs |
| R4 | A1 is four tasks: A1a to A1d. | TDI `:77-81`; D section 4; D section 9 items 1 and 2. | +2 runs |
| R5 | Task D0p ports the validator hunks of the depletion repair into `turts_audit/src/replay.rs`. | `git diff -U0 8d6902c 43282fe`: hunks at `economylog.rs:3470` and `:5541`. | the merge has no owner |
| R6 | Q1b commits `tests/m3/refreeze_allowed.json` before the A2c commit. | D section 5 item 3. | none |
| R7 | The source checks of D section 9 items 5 and 13 are tests in `crates/turts_sim/tests/one_step_function.rs`, so `cargo test --workspace` runs them (`.github/workflows/ci.yml:33`). | `ci.yml` stays out of the hot files. | none |
| R8 | Q2 is the lane of `qa-lead` on top of the A2c branch. A2c and Q2 merge together. | A2c alone is red at G-FROZEN by design. | none |
| R9 | A2a builds the bit-equal harness. A2b1, A2b2, A4 and A3 use it again, each with a reference copy from its own base. After A2b2, the harness has no plain-progress frames (gate 2, on R9). | D `:54-59`. | +2 runs |
| R10 | A2b1 also replaces the second creator of `PlainMoveProgress`: the failure of a legacy envelope (`orders.rs:2044-2058`, `request.plain`). Then A2b2 deletes a type that no code creates. | D 9.11 lists `:2044-2058` in the A2b file list but does not say which part owns it. If A2b2 owned it, A2b2 would change behaviour (a legacy envelope that fails would no longer show `Blocked`; `group_movement.rs:3027`). | A2b2 is not a pure deletion |
| R11 | The trace emitter and the reference harness read the movement state only through `turts_sim::movement::probe` (feature `test-probes`). They never name `PlainMoveProgress`. | A2b2 deletes the type. An emitter that names it would change between the two runs of a P0 pair, and the rule "the emitter sha256 is equal" (D `:140`) would fail. | the P0 pair at A2b2 cannot be compared |
| R12 | The counts of D 9.13 are checked in two stages. At A2a: one `for _ in 0..8` and one call `step(` + `&MovementInput` in `movement/` outside `movement/plain.rs`, `movement/reference.rs` and `movement/tests/`. At A2b2: one of each in `movement/`, outside `reference.rs` and `tests/`. | Today the count is 3 of each (`movement.rs:807, 2009, 2192`; calls at `:793, 1993, 2174`). One of each is in `step_plain_progress`, which stays until A2b2. The reference copy holds the old kernel. The text `step(&MovementInput` occurs 0 times as written, because rustfmt puts the argument on the next line. So the check uses the regex `\bstep\(\s*&MovementInput` (python probe: 3 hits today). | the D 9.13 check cannot go green at A2a |
| R13 | The tests that read `PlainMoveProgress` go red at A2b1, not at A2b2. So A2b1 lists them and gives each one its new observation (section 1a). | A2b1 removes the creators of `PlainMoveProgress`. Eight tests in `group_movement.rs` read it (list in Task A2b1). | the builder changes assertions with no rule |

## 3. Dependency graph

Model: scratch file `slice.py`, revision 2. Cycle check: no cycle.

- Run estimates come from the MC actuals of the most similar MC task.
- Lane hours = runs × 2 h. The MC range per run is 1.4 to 2.9 h.
- "In work" counts only the remaining runs.
- MP1-E1, MP1-E2 and MP1-M4 are start markers for MP1 lanes (section 5). They take 0 runs.

| Task | What | Builder seat | Reviewer seat | Branch | Inputs (merged first) | Hot files (shared with) | GPU | Runs | Lane h |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| S1 | `turts_audit` crate, validator moved unchanged (in work) | backend-engineer | qa-lead | `pre-mp1/s1-audit` | none | `turts_audit/*` (D0p, A1b-A1d), `economy.rs` (D0p, A1a-A1c) | no | 2 | 4 |
| S45 | Client splits (in work) | bevy-rendering-engineer | ux-designer | `pre-mp1/s45-client` | none | `tests/m2/refutations.json` (S2, A1b, A1c) | slow set only | 2 | 4 |
| G1 | `within` finds every point inside `reach` | simulation-core-engineer | qa-lead | `pre-mp1/g1-grid` | none (step 4: D0p) | `grid.rs` (A3) | no | 3 | 6 |
| Q1a | Classifier and self-test | qa-lead | simulation-core-engineer | `pre-mp1/q1a-classifier` | none | `tests/m3/refreeze_classify.py` (Q1b, Q2) | no | 2 | 4 |
| D0 | Depletion repair in `codex/mc-integration` (MC) | (MC) | (MC) | `codex/mc-fix-deplete` | MC | — | — | 0 | — |
| D0p | Repair ported into the slice | backend-engineer | simulation-core-engineer | `pre-mp1/d0-port` | D0, S1 | `replay.rs`, `economy.rs`, `orders.rs`, `gather.rs` | no | 2 | 4 |
| S2 | `orders.rs` to `orders/` | simulation-core-engineer | technical-director | `pre-mp1/s2-orders` | D0p | `orders/` (A2b1, A2c, A3), `refutations.json` | no | 2 | 4 |
| S3 | `movement.rs` to `movement/` | simulation-core-engineer | technical-director | `pre-mp1/s3-movement` | D0p | `movement/` (Q1b, A2a, A2b1, A2b2, A4, A3) | no | 2 | 4 |
| A1a | Live reader; outcome assertions | backend-engineer | qa-lead | `pre-mp1/a1a-reader` | S1 | `economy.rs`, `mc_orders.rs`, `session.rs` | no | 3 | 6 |
| Q1b | Trace emitter, probe API, counts, allowed list | qa-lead | simulation-core-engineer | `pre-mp1/q1b-trace` | S3, G1, Q1a | `movement/` (probe module), `turts_sim/Cargo.toml` | no | 2 | 4 |
| A1b | `LedgerAudit`, domains, side by side, exit commit | backend-engineer | qa-lead | `pre-mp1/a1b-ledger` | A1a, D0p, S2, S3 | `turts_audit/*`, `ledger_view.rs`, `refutations.json`, `economy.rs`, `el1.rs` | yes | 4 | 8 |
| A2a | One step function, bit-equal | simulation-core-engineer | qa-lead | `pre-mp1/a2a-step` | Q1b, G1, S3 | `movement/frame.rs` | no | 4 | 8 |
| A1c | Replay validator deleted | backend-engineer | qa-lead | `pre-mp1/a1c-delete` | A1b | `turts_audit/*`, `economy.rs`, `el1.rs` | yes | 2 | 4 |
| A1d | One ledger oracle | backend-engineer | qa-lead | `pre-mp1/a1d-conservation` | A1c | `turts_pack/tests/common/*` | no | 2 | 4 |
| A2b1 | The fallback pool, the gather protocol, the guards | simulation-core-engineer | qa-lead | `pre-mp1/a2b1-fallback` | A2a, A1c, S2 | `movement/*`, `orders/run.rs`, `orders/pending.rs`, `orders/behaviours/follow.rs`, `economy/gather.rs` | no | 3 | 6 |
| A2b2 | The switch goes; the plain movement model is deleted | simulation-core-engineer | qa-lead | `pre-mp1/a2b2-delete` | A2b1 | `movement/*`, `orders/*`, `views.rs`, `ledger_view.rs`, `el1.rs`, `command.rs`, `gather.rs` | no | 2 | 4 |
| A2c | Single moves take the pool path; plain reservations deleted | simulation-core-engineer | qa-lead | `pre-mp1/a2c-pool` | A2b2, Q1b | `orders/*`, `command.rs`, `economy/combat.rs:154`, `economy/transaction.rs:736`, `views.rs:512` | no | 4 | 8 |
| Q2 | The one re-freeze | qa-lead (technical-director reviews) | technical-director | `pre-mp1/q2-refreeze` | A2c | the fixture, `refutations.py:18` | no | 3 | 6 + CEO wait |
| A4 | The frame as functions on `&mut FrameState` | simulation-core-engineer | performance-architect | `pre-mp1/a4-frame` | A2b2 (merge after Q2) | `movement/frame.rs` | no | 2 | 4 |
| A3 | Two grids; combat cell | simulation-core-engineer | performance-architect | `pre-mp1/a3-grids` | A4, G1 (merge after Q2) | `movement/*`, `grid.rs`, `combat.rs:111-117`, `orders/run.rs` | no | 3 | 6 |
| REG | Slice regression | qa-lead | (Root) | `pre-mp1/reg` | Q2, A3, S45, A1d | — | yes | 2 | 4 |
| TDN | Architecture note | technical-director | (Root) | none | REG | — | no | 1 | 2 |
| | **Total (remaining)** | | | | | | | **52** | **104 lane h** |

**Critical path:** S1 → D0p → S3 → Q1b → A2a → A2b1 → A2b2 → A2c → Q2 → REG → TDN = **27 runs**.
- That is 54 h at 2 h per run (range 38 to 78 h). The D0 wait and the CEO wait in Q2 come on top.
- A second path reaches A2b1 at the same offset: S1 → D0p → S2 → A1b → A1c.
- If R2 fails, the path is 31 runs.

**What changed against revision 1 of this plan:**
- A2b (3 runs) is now A2b1 (3) and A2b2 (2).
- A2c grows from 3 to 4 runs, because it takes the deletion of the plain reservations and their callers.
- The path grows from 24 to 27 runs. The total grows from 49 to 52 runs.
- A4 now starts after A2b2.

**Lanes at each stage** (offset in runs; D0 assumed at once):

| Offset | Slice lanes | Tasks | MP1 lanes that can start |
| --- | --- | --- | --- |
| 0-2 | 4 | S1, S45, G1, Q1a | lobby (server; D 9.15) |
| 2-4 | 3 | G1, D0p, A1a | — |
| 4-6 | 3 | S2, S3, A1a | — |
| 6 | — | — | MP1-E1: damage and armour, power, effects, match lifecycle |
| 6-8 | 2 | Q1b, A1b | |
| 8-12 | 2 | A2a, A1b / A1c | |
| 12 | — | — | MP1-E2: construction, rally, repair approach, automatic targeting |
| 12-14 | 2 | A2b1, A1d | |
| 14-15 | 1 | A2b1 | |
| 15-17 | 1 | A2b2 | |
| 17-19 | 2 | A2c, A4 | |
| 19 | — | — | MP1-M4: the M4 route follower (after A4) |
| 19-21 | 2 | A2c, A3 | |
| 21-24 | 2 | Q2, A3 | |
| 24-27 | 1 | REG, TDN | |

The slice uses at most 4 lanes. From offset 6, MP1 lanes add to that count. Headed GPU work queues on the one lock: S45, A1b, A1c, REG, and the pixel rows of the MP1 lanes.

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

## 5. Start points for MP1 lanes, and early stops

D section 9 item 15 is binding. GSIM gate 2, "Answer to 3": the replay validator lets only movement change the state between operations. It checks an Other-phase operation for phase continuity only (`economylog.rs:540-556` at `8d6902c`).

| Point | After (offset) | MP1 work that can start | Condition | Open debt and its deadline |
| --- | --- | --- | --- | --- |
| Lobby | now (0) | the two-slot lobby (`turts_server` room) | It is on the server and does not touch the sim. | — |
| E1 | S1, S2, S3, S45, D0p (6) | damage and armour, power, effects, match lifecycle | Every sim write is inside a captured operation (`capture_operation`, `economy/transaction.rs:90`). Put new code in new files (TDI B: `economy/damage.rs`, `economy/power.rs`, `match_state.rs`). An MP1 edit of `economy/combat.rs` rebases on A2c (`:154`) and A3 (`:111-117`). | The replay validator is still the second model. Until E2, no MP1 lane adds an order or writes `MoveTarget`. |
| E2 | A1c (12) | construction, rally, the repair approach, automatic targeting | These add orders. They write `MoveTarget` in the same way before and after A2, so they need no rework. | Two exceptions (gate 2): a test that pins a blocked engine trip changes at A2b1, and a test that pins the operations of a single move, or case S12, changes at A2c. Until A2c, MP1 adds no test of either kind (audit `:48`). |
| E3 | Q2 (24) | everything in MP1 | — | A4 (audit `:36`: before M4). A3 (audit `:33`: before MP1 content adds a command centre or a long-range tank). |
| M4 | A4 (19) | the M4 route follower | It goes into the one step function only (D section 1). | — |

Early stops with value, if MP1 must take the lanes:
- **Stop after E1:** the files are split and the audit crate exists. MP1 lanes that add orders wait.
- **Stop after E2:** the validator checks conservation only, and all MP1 lanes can run. Two step branches remain until A2b2.
- **Stop after Q2:** one movement model. A4 and A3 stay open with the deadlines above.

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

**Branch / inputs:** `pre-mp1/s1-audit` (worktree `../TuRTS-pre-s1-audit`, target `target/mc-pre-s1-audit`); none. **Design:** D section 3; TDI A1 step 1 (`:78`). **Done so far:** `c1ebb23` (the crate: `lib.rs`, `format.rs`, `replay.rs`) and `5ce21d2` (`scripts/boundary.sh:51-60`). Root has the review report (`f7e7ccb`). Each step below that the review already ran is recorded, not run again.

- [ ] **Step 1: Pure-move proof.** F-MOVE.
    - Old file: `economylog.rs` at `8d6902c`.
    - New files: `crates/turts_audit/src/format.rs`, `crates/turts_audit/src/replay.rs` and `crates/turts_server/src/economylog.rs`.
    - Expected: `RESULT EQUAL`, `exit=0`.
    - Test names: first write the seven writer tests that stay in `economylog.rs` on the board, by name. Then compare after the map `sed 's/^economylog::tests::/replay::tests::/'` on the base list.
    - Expected: `cmp` gives `exit=0`, or a difference of exactly the seven named tests (section 1a).
- [ ] **Step 2: Boundary reds.** For each new row, run `bash scripts/boundary.sh --rule forbid --crate <crate> --target <target> --manifest <fixture manifest> --names turts_audit; echo exit=$?` (`scripts/boundary.sh:25-31`). Expected: the forbid message, `exit=1`. How the fixture selects its rows is NOT TRACED by the producer.
- [ ] **Step 3: Gate.** F-RUST, F-QA, F-FROZEN, F-REF (the M2 E7 and E8 cases of `session.rs`). Expected: `"failed": []`.
- [ ] **Step 4: Reviewer fault** (D 9.12).
    - (a) Swap two statements in one moved function. `item_equal.py` prints `RESULT DIFFERENT`, `exit=1`.
    - (b) Behaviour plant: invert the comparison of the refusal rule (text "refused unit {unit_id} changed state inside its order operation", `economylog.rs:2349` at `8d6902c`). First show it red on the base build with `cargo test -p turts_server --locked --bin turts_server economylog::tests; echo exit=$?`. Then show it red on the moved build with `cargo test -p turts_audit --locked; echo exit=$?`. Each gives `exit=101`. Restore each and get a fresh green.

**Exit:** both proofs are EQUAL, the boundary rows are red on their fixtures, and the plant is red on both builds. **Runs left: 2.**

### Task S45: client splits (in work)

**Seat:** `bevy-rendering-engineer`. Review: `ux-designer`.

**Branch / inputs:** `pre-mp1/s45-client` (worktree `../TuRTS-pre-s45-client`); none. **Design:** D revision 1 rows S4 and S5; TDI C4 to C7. **Done so far:** commits `34b7c4f` to `d047433`. Case `E2-browser-hardcoded-label` now plants `crates/turts_client/src/visuals/world_tooltip.rs`.

- [ ] **Step 1: Pure-move proof for each commit.** F-MOVE: `item_equal.py` on each split, and the test-list `cmp`. `Core::step` is not split (D `:167`). Expected: `RESULT EQUAL` six times, and `cmp` `exit=0`.
- [ ] **Step 2: Registry anchors.** Each moved browser plant has its anchor exactly once in its new `plant_path` (MC plan `:72`). Case `E9-browser-debug-only-ui` still names `crates/turts_client/src/visuals.rs`. Check that its anchor is still there.
- [ ] **Step 3: Gate.** F-RUST, F-WASM, F-QA. The browser cases run in the slow set.
- [ ] **Step 4: Reviewer fault** (D 9.12).
    - (a) Swap two statements in a moved function: `RESULT DIFFERENT`, `exit=1`.
    - (b) Behaviour plant: the plant of the registry case `E2-browser-hardcoded-label`. First show it red on the base build. Then show it red on the split build, through the slow set (`--set ci-browser --cases E2-browser-hardcoded-label`, the GPU lock). Expected: `"failed": []`.

**Exit:** all proofs are EQUAL, the plant is red on both builds, and the slow set is green with the pixel rows SH1, SH1-dpr2, KM1, KM2, PER1, SEL4, HUD1, HUD2, HUD4 and VIS1. **Runs left: 2.**

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

**Seat:** `simulation-core-engineer`. Review: `technical-director`.

**Branch / inputs:** `pre-mp1/s2-orders`; D0p. **Design:** D revision 1 row S2; D 9.12; TDI C2.

**Files:** `crates/turts_sim/src/orders.rs` becomes `crates/turts_sim/src/orders/`. Each file gets a list of items, not line ranges (D 9.12). The line numbers below are the item starts at `bfd5097`. D0p moves them by up to +6 lines. **Verify first** each item by its name.

- **`plain.rs`:** only the items that A2c deletes.
    - `PlainMoveOrderIdentities` (`:96`), `PlainMoveReservations` (`:99`) and its `impl` (`:109`), `decrement_slot_count` (`:140`; used only by that impl, `:112`, `:122`), `PlainMoveReservation` (`:155`), `PlainMovePriorState` (`:167`).
    - `arbitrate_plain_move_reservations` (`:1189`), `record_plain_dispatch_selection` (`:1390`), `reservation_precedes_request` (`:1406`).
    - `plain_move_has_no_prior_order_state` (`:3101`), `plain_move_reservation_exists` (`:3115`), `plain_move_reservation_capacity_available` (`:3121`), `capture_plain_move_prior` (`:3198`), `restore_plain_move_prior` (`:3221`).
    - `remember_plain_move_identity` (`:3614`), `record_plain_move_reservation` (`:3622`), `forget_plain_move_identity` (`:3652`).
- **Shared items that stay out of `plain.rs`** (D 9.12):
    - `UnitMoveNeedsAllocation` (`:152`), `SpreadIndexCache` (`:178`), `record_dispatch_selection` (`:1358`);
    - `movement_envelope_capacity_available_after_replacing` (`:3151`; A2c needs it, `:559`);
    - `unit_has_pending_move` (`:3286`), `clear_gather` (`:3661`);
    - the test counters `:22-47` (`count_plain_candidate_index_step` is called at `:1743` and `:1763` in the shared dispatch).
- **`mod.rs`:** the counters `:22-47`, the types (`PendingMovementOrder` `:49` to `OrderUnitCapabilitiesAudit` `:490`, `UnitOutcomes` `:503`, `MovementEnvelopeRequest` `:642`), and `pub(crate) use` re-exports, so that callers keep the `crate::orders::*` paths.
- **`pending.rs`:**
    - `PendingMovementOrders` and its impls (`:75`, `:191`), `MovementDispatchState` (`:86`, `:185`), `UnitMoveNeedsAllocation`, `SpreadIndexCache`;
    - `decrement_member_count` (`:380`), `pending_member_target_is_live` (`:389`), `pending_live_member_count` (`:401`);
    - `queue_movement_envelope` (`:533`), `queue_legacy_move_to_envelope` (`:551`), `prepare_legacy_move_to_waiters` (`:590`), `reset_legacy_move_to_unit` (`:617`);
    - `PromotionHeaderIndex` (`:654`), `insert_promoted_request` (`:679`), `finish_movement_dispatch_tick` (`:1134`), `dispatch_start_slot` (`:1174`), `record_dispatch_selection` (`:1358`), `queue_unallocated_move_orders` (`:1416`), `resolved_move_point` (`:1595`), `refresh_spread_reservation` (`:1616`), `dispatch_pending_movement` (`:1652`).
- **`run.rs`:** `run_orders` (`:697`), `attack_point_has_target_in_range` (`:1076`).
- **`behaviours/attack.rs`:** `attack_goal` (`:2185`), `run_unit_attack` (`:2213`).
- **`behaviours/follow.rs`:** `run_unit_follow_move` (`:2309`).
- **`behaviours/queue.rs`:** `start_next_queued_order` (`:2465`), `start_order` (`:2497`).
- **`apply.rs`:**
    - `unit_order_capable` (`:508`), `unit_toggleable` (`:524`), `apply_order` (`:2610`), `validate_target` (`:3028`);
    - `movement_envelope_capacity_available_after_replacing` (`:3151`), `unit_has_pending_move` (`:3286`), `unit_orders_empty` (`:3294`), `unit_radius` (`:3312`), `clamp_target` (`:3320`), `apply_unit_order` (`:3331`), `clear_replaced_movement` (`:3596`), `clear_gather` (`:3661`).
- **`tests.rs`:** the test module (`:3673`), declared as `#[cfg(test)] mod tests;` in `mod.rs`, so the test names stay `orders::tests::*`.
- **Registry:** case `E8-foreign-gather-owner-check` changes its `plant_path` for `orders.rs` to the file that now holds its anchor. The anchor text does not change.

- [ ] **Step 1: Move.** One commit. No item body changes.
- [ ] **Step 2: Proof.** F-MOVE. Expected: `RESULT EQUAL`, `cmp` `exit=0`. Also check that `plain.rs` holds exactly the items of the list above: `grep -cE '^(pub(\(crate\))? )?(fn|struct|impl)' crates/turts_sim/src/orders/plain.rs` against the count of the list (17 items, with the impl).
- [ ] **Step 3: Gate.** F-RUST, F-FROZEN, F-REF (`E8-foreign-gather-owner-check`). Expected: `"failed": []`.
- [ ] **Step 4: Reviewer fault** (D 9.12).
    - (a) Swap two statements in `run.rs`: `RESULT DIFFERENT`, `exit=1`.
    - (b) Behaviour plant: make `start_next_queued_order` (`behaviours/queue.rs`; `orders.rs:2465` on the base) return at once.
    - First show it red on the base build: `cargo test -p turts_sim --locked --test orders queued_order_starts_in_the_tick_after_landing; echo exit=$?` gives `exit=101` (`crates/turts_sim/tests/orders.rs:939`).
    - Then show it red on the moved build: `exit=101`.
    - Restore each and get a fresh green.

**Exit:** EQUAL, the same test names, the plain.rs list exact, the plant red on both builds, and the stream unchanged. **Runs: 2.**

### Task S3: `movement.rs` to `movement/`

**Seat:** `simulation-core-engineer`. Review: `technical-director`.

**Branch / inputs:** `pre-mp1/s3-movement`; D0p. It runs in parallel with S2. **Design:** D revision 1 row S3; D 9.12; TDI C3.

**Files:** `crates/turts_sim/src/movement.rs` becomes `movement/`. Each file gets a list of items. **Verify first** each item by its name.

- **`plain.rs`:** only `step_plain_progress` (`:671`) and `make_plain_progress` (`:925`), with `PlainMoveProgress` (`:58`) and its derives. All three go at A2b2.
- **Shared items that stay out of `plain.rs`** (D 9.12):
    - `minimum_unseen_hex_radius` (`:907`), `finite_hex_ring_limit` (`:913`), `nearest_angle_order` (`:957`), `target_key` (`:969`), `update_target_claims` (`:973`). These go to `spread.rs`.
    - `angle` (`:480`), `MovementInput` (`:486`), `MovementOutput` (`:494`) and `step` (`:500`). These go to `mod.rs`.
- **`mod.rs`:**
    - the test counter `:13-21`;
    - `MovementActivity` (`:24`), `PlainGatherGoal` (`:79`; A2b1 moves it into `MoveProgress`), `MoveProgress` (`:34`), `PhysicalEscape`, `MovementPool`, `MovementPools`, `SpreadMember`, `StaticFootprint` and the other type items before `:116`;
    - `angle`, `MovementInput`, `MovementOutput`, `step`;
    - re-exports that keep every `crate::movement::*` path.
- **`spread.rs`:** the items from `:116` to `:480` (before `angle`), and the five shared helpers above.
- **`search.rs`:** `MovementSearch` (`:517`) and its impl (`:525`), `body_overlaps_position` (`:650`).
- **`pool_index.rs`:** `MovementPoolIndex` (`:995`) up to the item before `:1346`.
- **`frame.rs`:** `:1346` up to `integrate_movement`, which includes `step_frame` (`:1382`) and `step_frame_preserving_resting` (`:1388`).
- **`ecs.rs`:** `integrate_movement` (`:2410`) up to the first test module.
- **`tests/`:** `spread_tests` (`:2512`), `plain_movement_tests` (`:3471`) and the other test modules, each at its old module path.

- [ ] **Step 1: Move.** One commit.
- [ ] **Step 2: Proof.** F-MOVE: `RESULT EQUAL`, `cmp` `exit=0`. `plain.rs` holds exactly 3 items with bodies (`grep -cE '^(pub )?(fn|struct) '` gives 3).
- [ ] **Step 3: Gate.** F-RUST, F-FROZEN, F-REF (the selector lists 0 cases for `movement/`; record it).
- [ ] **Step 4: Reviewer fault** (D 9.12; the `× 1.0001` plant of the old text stayed green in four test files, gate 2 M3).
    - (a) Swap two statements in `frame.rs`: `RESULT DIFFERENT`, `exit=1`.
    - (b) Behaviour plant: set the push of the pool branch to zero (`let push = (limit - separation) * offset / separation;`, `movement.rs:2163`; change it to `let push = Vec2::ZERO;`).
    - First show it red on the base build: `cargo test -p turts_sim --locked --test group_movement forty_units_rest_with_no_overlap_and_no_blocked; echo exit=$?` gives `exit=101` (`group_movement.rs:1283`).
    - Then show it red on the moved build. Restore each.

**Exit:** EQUAL, the same test names, the plant red on both builds, and the stream unchanged. **Runs: 2.**

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

**Branch / inputs:** `pre-mp1/a1d-conservation`; A1c. **Design:** D `:73-74`; TDI step 4.

**Files:** `crates/turts_pack/tests/common/conservation.rs` stays (A1d ruling). `crates/turts_server/tests/economy.rs` (`assert_run_constants`), `crates/turts_audit/Cargo.toml` (no `bevy*`), `tests/m2/refutations.json` and `tests/m2/test_refutations.py` (three `A1d-run-constants-*` rows), `crates/turts_checks/tests/` (the run-constant writer scan and the `ResMut` arm). `tests/m3/refutations.json` is not edited. `tests/m3/refutations.json` is not edited.

- [ ] **Step 1: Text check first.** For each M3 case, `grep -F` its `expected_failure` in `conservation.rs`. Record the source line of each one. Each text must come out of `turts_audit` with the same bytes. If a text cannot keep its bytes, stop and report. A1d does not edit the registry (section 1a).
- [ ] **Step 2: Replace.** `cargo test -p turts_pack --locked; echo exit=$?` gives `exit=0`.
- [ ] **Step 3: Gate.** F-RUST, F-FROZEN, then `--set m3-all --workers 9 --alsa`. Expected: 17 of 17 red, `"failed": []`.

**Exit:** the same 17 M3 texts, `conservation.rs` byte-equal to `afb9c6de`, the A1c hand-over items closed, the run-constant writer scan red on its plants. **Runs: 2.**

### Task Q1b: trace emitter, probe API and allowed list

**Seat:** `qa-lead`. Review: `simulation-core-engineer`.

**Branch / inputs:** `pre-mp1/q1b-trace`; S3, G1 (the counter result), Q1a. **Design:** D section 5 item 1; D 9.6, 9.7 and 9.14; R11.

**Files:**
- `crates/turts_sim/Cargo.toml`: a feature `test-probes = []`. There is no `[features]` section at `bfd5097`.
- Create `crates/turts_sim/src/movement/probe.rs` (`#[cfg(feature = "test-probes")]`). It is the only API that the emitter and the harness read (R11).
    - `unit_state(world, unit) -> &'static str`: `"none"`, `"pool"` (a `MoveProgress`) or `"plain"` (a `PlainMoveProgress`).
    - `activity(world, unit)`: the `MovementActivity` of the `MoveProgress`, or `none` (D 9.7).
    - The last frame flags of each unit:
        - `separated`, from the local of each branch (`movement.rs:770`, `:1973`, `:2134`);
        - `was_projected`, only for a unit with no pool (`movement.rs:2008`, set at `:2036`). It is `null` for a pool unit, because the pool branch has no such local. So a P0 pair across A2a compares the same field;
        - `edge_hit`, only when G1 is held for A2c.
    - The flags reach the probe through a `#[cfg(feature = "test-probes")]` field on `MovementFrameOutput`.
    - Other code can call the probe; A2b2 changes only its body.
- `crates/turts_pack/Cargo.toml`: dev-dependency `turts_sim` with `features = ["test-probes"]`.
- Create `crates/turts_pack/tests/m2_trace.rs` with `#[ignore] m2_trace_writes_four_packages`. It uses the run loop of `m3_m2_stream.rs` (`TICKS` `:15`, `PACKAGES` `:16`, `run_package` `:240`). It writes `<dir>/<package>.jsonl` in the Q1a format to `TURTS_M2_TRACE_OUT`. It adds two fields:
    - `state` can be `plain`;
    - each `MoveTo` command operation gets `move_to_route`: `per_unit` when `unit_ids.len() > 1` (`command.rs:173-174`), and `direct` otherwise (`:488-495`).
- Extend `tests/m3/refreeze_classify.py` (Q1a) to read `plain`, `was_projected` and `move_to_route`. The self-test gains plant 13, a flipped `was_projected`, which must give red.
- Create `tests/m3/refreeze_allowed.json` with the acme differences that D allows:
    - the scout `state` goes from `none` to `pool` from the accept tick;
    - `move_to_route` goes from `direct` to the envelope route (the scout command, `m3_m2_stream.rs:263`);
    - one scout operation is added at tick 1, and one at the tick after arrival (gate probe: 124; D `:126`);
    - no position changes.

- [ ] **Step 1: Run on the base.** `TURTS_M2_TRACE_OUT=target/pre-q1b/evidence/base cargo test -p turts_pack --locked --test m2_trace -- --ignored --exact m2_trace_writes_four_packages; echo exit=$?`. Run it again into `base2`. `cmp` each package pair: `exit=0`. Log each sha256 on the board.
- [ ] **Step 2: Counts** (D 9.6 and 9.8). For each package, record:
    - (a) the unit-ticks with state `none`, `pool` and `plain`;
    - (b) the unit-ticks with `state == none` and `separated == true`. A2b2 records this count with its P0;
    - (c) the unit-ticks with `was_projected == true`;
    - (d) the `move_to_route` of each `MoveTo`.

    GSIM saw 122 / 256 / 230 / 289 unit-ticks with no progress, and 0 with plain progress. If any stream unit has state `plain`, or if (b) or (c) is above 0, stop and report: P0 at A2b2 would then not be equal (D 9.8).
- [ ] **Step 3: Feature leak check.** `cargo tree -p turts_server -e features -i turts_sim; echo exit=$?`. Expected: no `test-probes` in the normal build. This is NOT TRACED until this run.
- [ ] **Step 4: Commit, gate.** Commit the probe, the feature, the emitter, the classifier extension and `refreeze_allowed.json`. Log the sha256 of the allowed list and the emitter on the board before A2c (R6).
    - F-RUST, F-FROZEN.
    - `python3 tests/m3/refreeze_classify.py --self-test; echo exit=$?` prints `13/13 plants red; clean pair green`, `exit=0`.
- [ ] **Step 5: Plant.** Make `separated` always `false` in the probe. The step 2 count (b) changes, or `--p0` against `base` goes red. First show it red, then restore.

**Exit:** a deterministic base trace, the four counts recorded, the probe API in use, and the allowed list committed and hashed. **Runs: 2.**

### Task A2a: one step function

**Seat:** `simulation-core-engineer`. Review: `qa-lead`.

**Branch / inputs:** `pre-mp1/a2a-step`; Q1b, G1, S3 (R2). **Design:** D row A2a; D `:54-59`; D 9.5, 9.8, 9.9 and 9.13; R12.

**Float rule (D 9.8, binding).** The two branches compute three values with different float expressions:
- the start velocity: `movement.rs:1968-1972` against `:2130`;
- the push: `:1985` against `:2163`;
- the speed cap: `:2003-2004` against `:2184-2185`.

The merged step selects each of the three expressions by the presence of a pool. A unit with a pool takes the pool expression. A unit with no pool takes the no-progress expression. The output is bit-equal on every frame. The gate probe proved that one expression for all units cannot be bit-equal: with the no-progress expressions, 46 of 64 pooled seeds differ.

**Assertions (D 9.9).** A2a changes no assertion and no expected value. A changed assertion, or a red suite test, stops A2a. A green GM40, GMA or GMU proves nothing about equality, because no suite test pins pooled positions bitwise. The reference copy is the proof.

**Files** (at `bfd5097`; **verify first** after S3):
- `movement/frame.rs`:
    - the no-progress branch (`:1880-2085`) and the pool branch (`:2086-2380`) become one step function for one unit, with the switch of the float rule;
    - the plain-progress branch (`:1843-1879`) and `step_plain_progress` stay until A2b2;
    - no writer changes. The R2 items keep their signatures.
- Phase counters behind `test-probes`: arrival, steering, projection, stall, fallback; for each unit and tick.
- Create:
    - `movement/reference.rs` (`test-probes`): the kernel at the A2a base, through the probe API (R11);
    - `crates/turts_sim/tests/kernel_reference.rs`;
    - `crates/turts_sim/tests/one_step_function.rs`.

- [ ] **Step 1: Red (counter and source counts).**
    - (a) `each_moving_unit_passes_each_shared_phase_once`. The scenario holds a gather trip, a queued start, an attack approach, a follow, a resume, a single envelope move, a group move and a synthetic unit (D `:155`). The counters are in the pool-branch code only. Message: `unit {id} tick {t} phase {p}: {n} calls`.
    - (b) `step_loop_and_step_call_occur_once_outside_plain` (R12). The test reads the production files of `crates/turts_sim/src/movement/`, except `plain.rs`, `reference.rs` and `tests/`. It counts `for _ in 0..8` and the regex `\bstep\(\s*&MovementInput`, and expects 1 of each. Message: `{pattern}: {n} occurrences, expected 1`.
    - Run: `cargo test -p turts_sim --locked --features test-probes --test one_step_function; echo exit=$?`.
    - Expected: (a) red with `0 calls` for the gather unit; (b) red with `2 occurrences` (the no-progress and pool branches; python probe at `bfd5097`); `exit=101`. Record the run (D 9.5).
- [ ] **Step 2: Red (harness).** `kernel_reference.rs::kernel_matches_reference_bit_for_bit` runs:
    - 64 seeds × 300 ticks;
    - 1 to 5,000 movers, 0 to 20 statics of radius 0.5 to 8, and a `preserve_resting` set that is not empty;
    - frames with no-progress, plain-progress, pooled, escape, `Blocked`, follow and Attack-Point units (D row A2a; RUL `:75`);
    - every f32 compared with `to_bits`;
    - a counter for each phase, which must be above 0.

    Plant a `-0.0` (a zero velocity with its sign flipped). Run `cargo test --release -p turts_sim --locked --features test-probes --test kernel_reference; echo exit=$?`. Expected: red on the bit compare, `exit=101`. Restore.
- [ ] **Step 3: Green.** Merge the two branches with the switch. Rerun steps 1 and 2: `exit=0`. A seed that differs stops A2a.
- [ ] **Step 4: Commit, gate.**
    - Commit. F-RUST, F-FROZEN.
    - `--p0` on all four packages, base against A2a: `python3 tests/m3/refreeze_classify.py --p0 <base dir> <a2a dir>; echo exit=$?`, `exit=0`.
    - `cargo test -p turts_sim --locked --test group_movement --test gmu_same_point; echo exit=$?` and `cargo test --release -p turts_server --locked --test mc_group -- --ignored; echo exit=$?`. Each `exit=0`, with no assertion edit.
    - R2 condition: `cargo test -p turts_audit --locked; echo exit=$?` and EL1 (`cargo test --release -p turts_server --locked --test el1 -- --ignored --test-threads=1 --exact el1_muster_movement_and_queued_gather_validate_every_captured_tick; echo exit=$?`). Each `exit=0`.
- [ ] **Step 5: Plants.** First show each red, then restore:
    - (a) speed `Some(0.0)`: `kernel_reference` goes red;
    - (b) arrival and steering swapped: `kernel_reference` goes red;
    - (c) the switch gives the no-progress expressions to a pool unit: `kernel_reference` goes red on a pooled seed;
    - (d) a second copy of the steering loop (`for _ in 0..8`) outside `plain.rs`: test (b) of step 1 goes red with `2 occurrences`.

**Exit:** one step function with the switch, bit-equal on every frame, P0 equal on four packages, both counters red before and green after (both runs recorded), and the R2 condition green. **Runs: 4.**

### Task A2b1: the fallback pool, the gather protocol and the guards

**Seat:** `simulation-core-engineer`. Review: `qa-lead`: can each guard go red, and is the pool-id source deterministic?

**Branch / inputs:** `pre-mp1/a2b1-fallback`; A2a, A1c, S2. **CEO touch point 1 comes before this task starts** (section 6). **Design:** D row A2b; D 9.10, 9.11; R10, R13; gate 2 M1.

**What A2b1 does.** It replaces every creator of `PlainMoveProgress` with a pool of one. It deletes nothing. After A2b1, no code creates a `PlainMoveProgress`. The creators today:
- `make_plain_progress`, called at `movement.rs:1893` and `:2053`;
- the failure of a legacy envelope, `orders.rs:2044-2058` (R10).

Neither the pool expression switch nor the plain types change in A2b1.

**Pool-id source.** The step function is pure, and `next_pool_id` lives in `PendingMovementOrders` (`orders.rs:258-259`). So:
- the step returns a fallback request for each unit (unit id, the point, the spread result of `spread_destinations_with_context`);
- `integrate_movement` (`movement/ecs.rs`) takes the ids from `PendingMovementOrders::next_pool_id`, in unit-id order, after the step;
- the legacy failure path takes its id from the same counter.

There is one id space and one order.

**Collection rule.** A fallback pool is not collected while the gather stage of its unit still targets the goal of the pool (`MovingToNode` or `MovingToReceiver`). This is the same rule as for `follow_origin` and `preserve_resting` at `movement.rs:2399-2400`. The gather protocol in `run_orders` clears the pool when the gather is reached or the endpoint changes. Today this protocol does the same for plain progress (`orders.rs:771-783`).

**Files** (at `bfd5097`; **verify first** after S2 and S3):
- `movement/frame.rs` `:1893`, `:2053`: the fallback request in place of `make_plain_progress`.
- `movement/ecs.rs`: the id allocation.
- `movement/mod.rs`: `MoveProgress` (`:34`) gains `gather_goal: Option<PlainGatherGoal>` (`:79`), and `failure_recorded` if the legacy failure needs it (**verify first**: `PlainMoveProgress` has it, `orders.rs:2056`).
- The collection at `movement.rs:2383-2404` and `:2497-2508`.
- `orders/run.rs` (`orders.rs:734-860`): the gather protocol reads the fallback `MoveProgress`.
- `orders/pending.rs` (`orders.rs:2044-2058`): the legacy failure writes a `Blocked` pool of one.
- `orders/behaviours/follow.rs` (`orders.rs:2389-2458`, `same_goal_fallback`): the same protocol for a follow trip.
- `economy/gather.rs:1246-1263` (`set_target`; it reads `PlainMoveProgress` at `:1248`): it writes `MoveProgress.gather_goal`.
- The probe (`unit_state`) is unchanged.
- **Tests that read `PlainMoveProgress` (R13; section 1a, observation change).** Each test reads the same property from the fallback `MoveProgress` and its `MovementPool`. The expected value stays the same. Map: `activity` → `MoveProgress.activity`; `original_point` → the pool point; `slot` → `MoveProgress.slot`; `is_some()` / `is_none()` of the fallback → the same on `MoveProgress`. A pinned exact slot of a fallback unit gets its new value only from an independent hex-ring calculation in the test (MC design 6.3), never from a run.
    - `unit_target_move_waits_for_cap_and_samples_live_target_at_activation` (`group_movement.rs:373`; it reads at `:482`, `:491`);
    - `pending_replace_allows_prior_completion_without_advancing_append` (`:858`; `:929`);
    - `plain_singleton_blocked_by_a_full_width_post_reports_failure_by_gmu_deadline` (`:2005`; `:2077`, `:2083`);
    - `blocked_plain_move_escape_survives_queued_and_pre_ack_holds` (`:2156`; `:2193`);
    - `unit_follow_move_completes_at_its_reached_legal_fallback` (`:2298`; `:2323`, `:2383`);
    - `moved_follow_target_reopens_motion_instead_of_consuming_old_fallback` (`:2653`; `:2678`, `:2751`);
    - `idless_legacy_move_reports_blocked_without_inventing_an_order_id` (`:2757`; `:2817`);
    - `insufficient_primary_slots_block_the_whole_envelope_without_movement` (`:2975`; `:3027`).

    `crates/turts_sim/tests/orders.rs:383`, `:496` and `:555` read `PlainMoveOrderIdentities` or `PlainMoveProgress` (**verify first** which type each one reads). Lines that read `PlainMoveProgress` belong here. Lines that read identities belong to A2c.

    A red test outside this list stops A2b1.

- [ ] **Step 1: Red.**
    - `group_movement.rs::two_gatherers_deliver_to_one_receiver_in_one_tick`: both deliver, no footprints overlap, and each delivers within `2 × (distance / speed) + 5 s`. A second case covers a delivery after node depletion (D `:158`).
    - `group_movement.rs::each_trip_makes_at_most_one_fallback_pool`. A test-probes counter counts the fallback pools of each trip. A trip ends when the `MoveTarget` of its unit is removed or replaced, or when its gather leg ends. The test asserts at most 1 for each trip, in the two-gatherer scenario and in a crowd of 20 gatherers on one receiver. Message: `unit {id} trip {k}: {n} fallback pools`. The deadline guard alone passes an oscillation that delivers in the end (gate 2 M1).
    - `one_step_function.rs::no_plain_progress_is_created`: over the scenario of A2a step 1, `unit_state` is never `plain`.
    - Run: `cargo test -p turts_sim --locked --features test-probes --test group_movement --test one_step_function; echo exit=$?`.
    - Expected: `no_plain_progress_is_created` red (the fallback makes plain progress today); the fallback counter red on its stub (signature first); `exit=101`. The deadline guard can be green on the base; its red comes by plant (step 4).
- [ ] **Step 2: Green.** Make the edits. Apply the observation changes of the list. Rerun: `exit=0`.
- [ ] **Step 3: Commit, gate.**
    - Commit. F-RUST, F-FROZEN. A stream red stops the task: Q1b counted 0 plain-progress unit-ticks, so no stream unit reaches a fallback.
    - `--p0` on four packages, A2b1 base against A2b1: `exit=0`.
    - `kernel_reference` against the A2b1 base copy: bit-equal on every frame with no fallback. A frame where a fallback fires differs only in `unit_state` (`plain` to `pool`) and in the slot that the pool index chooses. The harness asserts both rules.
    - EL1 (A2a command): `exit=0`.
- [ ] **Step 4: Plants.** First show each red, then restore:
    - (a) the fallback returns no destination: the deadline of `two_gatherers…` goes red;
    - (b) collect the fallback pool on arrival while the gather stage still targets the goal: `each_trip_makes_at_most_one_fallback_pool` goes red;
    - (c) take the fallback pool id from a second counter: `kernel_reference` goes red, or `--p0` on a seed with a fallback goes red, because the ids depend on the order.

**Exit:** no creator of `PlainMoveProgress`, the three guards green, P0 equal, and the eight tests changed by observation only. **Runs: 3.**

### Task A2b2: the switch goes; the plain movement model is deleted

**Seat:** `simulation-core-engineer`. Review: `qa-lead`.

**Branch / inputs:** `pre-mp1/a2b2-delete`; A2b1. **Design:** D row A2b; D 9.8 (the switch), 9.11 (the deletion is driven by the compiler, with P0), 9.13; R11, R12.

**Commit 1: the switch goes.** The merged step uses the pool expressions for every unit (D 9.8).
- The harness compares against the A2b1 copy. A difference is allowed only on a unit-tick where, in the reference, the unit has no pool and is `separated`. The harness asserts this rule. The gate probe saw 1 of 64 no-progress seeds differ, always on the first contact tick, which enters the fallback (`movement.rs:2052`).
- `--p0` on four packages, A2b1 against commit 1: `exit=0`. Record it next to the Q1b count (b) of separated no-progress unit-ticks in the stream packages (D 9.8).
- If P0 differs while the count is 0, stop.

**Commit 2: the deletion.** Delete `PlainMoveProgress` and let the compiler list each user. After A2b1, every user is dead code. Grep at `bfd5097`:
- `PlainMoveProgress`: 36 references in `orders.rs`, 16 in `movement.rs`, 12 in `group_movement.rs` (now gone through A2b1), 5 in `economy/views.rs`, 1 in `economy/gather.rs`, 1 in `command.rs` (`:494`);
- `plain_progress`: 61 in `movement.rs` and 12 in `orders.rs`.

Delete:
- `step_plain_progress` and `make_plain_progress` (`movement/plain.rs`), and the plain branch (`movement.rs:1843-1879`);
- the `plain_progress` field of `MovementFrameUnit` and of its outputs;
- `AuditState.plain_movement_progress` and `RawPlainMovementProgressAudit` (`views.rs:442`, `:530-549`, `:1110`), and their reader in `el1.rs:188-215` and in `LedgerAudit`;
- the field `PlainMovePriorState.plain_progress` (`orders.rs:171`) and its two uses (`:3208`, `:3249`). It is always `None` after A2b1;
- the `.remove::<PlainMoveProgress>()` lines in `orders/` and `command.rs:494`;
- the `PlainMoveProgress` arm of `probe::unit_state` (R11).

Also:
- `PlainGatherGoal` stays as the type of `MoveProgress.gather_goal`;
- the reference copy loses its plain branch in the same commit;
- the harness drops the plain-progress frames (R9);
- `plain_movement_tests` (`movement.rs:3471-3700`) are deleted (section 1a, deletion). Two tests move to the pool-of-one path with their expected values unchanged: `unobstructed_step_keeps_the_legacy_target_until_within_one_step` (`:3527`) and `plain_move_respects_reserved_spacing_before_using_a_zero_primary_point` (`:3618`).

Not in A2b2: `orders/plain.rs` (the reservations and identities). A2c deletes it after the reroute (D 9.10).

- [ ] **Step 1: Red.** `one_step_function.rs::no_second_step_model`:
    - the production source of `crates/turts_sim/src` holds no `fn step_plain_progress` and no `PlainMoveProgress`;
    - in `movement/`, except `reference.rs` and `tests/`, `for _ in 0..8` and `\bstep\(\s*&MovementInput` each occur 1 time (R12; D 9.13);
    - messages: `step_plain_progress still defined at {path}` and `{pattern}: {n} occurrences, expected 1`.

    Run: `cargo test -p turts_sim --locked --features test-probes --test one_step_function no_second_step_model; echo exit=$?`. Expected: red with `2 occurrences`, `exit=101`.
- [ ] **Step 2: Commit 1 (the switch).** Harness rule and P0 as above. `cargo test -p turts_sim --locked; echo exit=$?`: `exit=0`, with no assertion edit.
- [ ] **Step 3: Commit 2 (the deletion).** `cargo build -p turts_sim --locked` until it is clean. Then F-RUST. `no_second_step_model` is green. `--p0` on four packages, commit 1 against commit 2: `exit=0`. `kernel_reference`: bit-equal on every remaining frame.
- [ ] **Step 4: Gate.** F-FROZEN (unchanged hash), and EL1 (A2a command). Each `exit=0`.
- [ ] **Step 5: Plant.** Restore one `for _ in 0..8` copy in a new function in `frame.rs`. `no_second_step_model` goes red. Restore.

**Exit:** one step function with no switch, no plain movement model, P0 equal on four packages for both commits, and the count recorded. **Runs: 2.**

### Task A2c: single moves take the pool path; the plain reservations go

**Seat:** `simulation-core-engineer`. Review: `qa-lead`.

**Branch / inputs:** `pre-mp1/a2c-pool`; A2b2, Q1b (the allowed list committed). If G1 held its commit (G1 step 5), cherry-pick it here. **Design:** D row A2c; D 9.10, 9.14; section 6 guards of D; GSIM M3.

**Commit 1: the reroute.**
- `orders/apply.rs`: the plain branch (`orders.rs:2822-2900`) takes the deferred path (`:2772-2820`).
- `command.rs:173-174` (`unit_ids.len() > 1`) and `:488-495`: a single `CommandKind::MoveTo` takes the envelope path (`queue_legacy_move_to_envelope`, `orders.rs:551`) with a pool of one.
- `PendingMovementOrder.plain` (`orders.rs:60`) stays. It marks the legacy idless envelope (`:2017`, `:2122`, `:2161`, `:2171`), whose behaviour `group_movement.rs:2757` pins.

**Commit 2: the deletion, after the reroute** (D 9.10). Delete `orders/plain.rs` (the 17 items of the S2 list), and fix each user that the compiler lists. Grep at `bfd5097`; **verify first**:
- `PlainMoveReservations` at `orders.rs:603`, `:1141`, `:1197-1397`, `:1669`, `:1737`, `:1783`, `:2124`, `:2795`, `:2917`, `:2998`, `:3117-3176`, `:3633-3656`;
- `reservation_precedes_request` at `:1773`;
- `forget_plain_move_identity` at 16 places in `orders.rs` and 1 in `command.rs`;
- `plain_move_reservation_exists` and `plain_move_has_no_prior_order_state` in `command.rs`;
- `PlainMoveOrderIdentities` at `economy/combat.rs:154`, `economy/transaction.rs:736`, `economy/views.rs:512` and `command.rs:268`.

The identity removal at `views.rs:512` changes `AuditState`. The Q2 classifier sees it.

**Tests** (section 1a). A red test that is not in this table stops A2c.

| Test | Kind | New expected value |
| --- | --- | --- |
| `crates/turts_sim/tests/orders.rs:988` `plain_move_writes_no_order_operation` | inversion | The command operation and one order operation at the accept tick; one order operation at the tick after arrival (D section 5 P3; gate probe at ticks 1, 1, 124). |
| `orders.rs:496` `accepted_plain_move_projects_without_unit_orders_and_legacy_move_has_no_identity` | inversion of the plain-move half only | `UnitOrders.current` holds the order from the accept tick to arrival. The legacy half is unchanged. |
| `group_movement.rs:282` `unopposed_authored_singleton_keeps_its_command_only_audit_shape` | inversion | As for `:988`. |
| `orders.rs:555` `stopped_or_removed_plain_move_identity_does_not_leak_into_the_view` | observation change | A stopped or removed single move leaves 0 `MoveProgress` and 0 pools in the view. |
| `orders.rs:383`, `crates/turts_pack/tests/mc_order_rows.rs:650`, `crates/turts_server/tests/mc_orders.rs:459` (they read `PlainMoveOrderIdentities`) | observation change | They read `UnitOrders.current`. The order id is the same. |
| `orders.rs:330`, `:607`; `group_movement.rs:109`, `:325`, `:1895`, `:2005`, `:2156`, `:2757`, `:2876`; `command.rs:341`; `crates/turts_server/tests/orders_plain.rs:118`; the `MoveTo` users in `economy_ordering.rs` and `economy_accounting.rs` | no change | Not renamed. A red stops A2c. |

- [ ] **Step 1: Red.** In `group_movement.rs`:
    - `second_move_to_a_held_point_keeps_the_first_unit_on_it` (S12): unit 1 rests on P, bitwise. Unit 2 rests at least `s - 0.00001` from P. No footprints overlap.
    - `one_unit_alone_takes_the_point` (`:1309`) gains four cases: through `MoveTo`; through an envelope with group 0; with a static unit of radius 4 far away; to a point off the map (the clamped point, bitwise).
    - `single_move_pool_is_collected_on_arrival`: 0 `MoveProgress` and 0 pools after arrival.
    - `three_thousand_single_moves_spread_over_two_ticks`: 2,048 at T and 952 at T+1.
    - Run: `cargo test -p turts_sim --locked --test group_movement; echo exit=$?`. Expected: S12 red and the cap test red, `exit=101`. Record the run.
- [ ] **Step 2: Commit 1 (the reroute).** Rerun step 1: `exit=0`. Apply the three inversions and the observation changes of the table. `--p0` for `base`, `haul` and `cedar` against the A2c base: `exit=0` (D 9.6).
- [ ] **Step 3: Commit 2 (the deletion).** Compiler-driven. F-RUST, without the stream test. Do not touch the fixture. Then run `grep -rnE 'PlainMove(Reservation|OrderIdentities|PriorState)|plain_move_(reservation|identity)' crates/turts_sim/src; echo exit=$?`. Expected: `exit=1` (no hit).
- [ ] **Step 4: Gate.** G-FROZEN is red by design. Record its first difference and hand over to Q2. G-WEB runs in the slow set after Q2.
- [ ] **Step 5: Plant.** Make `free_slot_grid` always false. `one_unit_alone_takes_the_point` goes red. Restore.

**Exit:** the guards are green, there are no plain reservations, the table is applied exactly, and only the stream test is red. Hand-over to Q2. **Runs: 4.**

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

**Branch / inputs:** `pre-mp1/a4-frame`; A2b2. It merges only after Q2 (R3). If A2c edits a `movement/` file, A4 rebases after A2c. **Design:** D row A4; GSIM L1; RUL `:71`.

**Files:** `movement/frame.rs`. The rest of `step_frame_preserving_resting` (the index build, the escapes, the write-back, the resting collection) becomes functions on `&mut FrameState`, called in one loop in unit-id order. The phase ranges of TDI `:165` are **verify first**, because A2a and A2b2 changed this function.

- [ ] **Step 1: Reference.** Refresh `movement/reference.rs` from the A4 base.
- [ ] **Step 2: Red by plant before the change.** Swap two phases in the real kernel: `kernel_reference` goes red. Restore.
- [ ] **Step 3: Change, then green.** `cargo test --release -p turts_sim --locked --features test-probes --test kernel_reference; echo exit=$?` gives `exit=0`. No assertion edit (section 1a).
- [ ] **Step 4: Commit, gate.** F-RUST, `--p0` on four packages, and `no_second_step_model`. After the rebase onto Q2: F-FROZEN with the new hash.
- [ ] **Step 5: Plant.** Reset one `FrameState` field between units: `kernel_reference` goes red. Restore.

**Exit:** bit-equal, the stream unchanged at the new hash, and one step function. **Runs: 2.**

### Task A3: two neighbour grids, combat cell

**Seat:** `simulation-core-engineer`. Review: `performance-architect`.

**Branch / inputs:** `pre-mp1/a3-grids`; A4, G1. It merges only after Q2. It rebases on A2c, because both edit `economy/combat.rs` (A2c at `:154`, A3 at `:111-117`). **Design:** D row A3; TDI A3; audit `:33`.

**Files:**
- `movement/frame.rs`: a movers grid and a statics grid (`speed` None, `movement.rs:1414`), with `within_pair` merged and sorted by `(id, index)`.
- `economy/combat.rs:111-117` and `orders/run.rs` (`orders.rs:722-728`; today `fold(f32::EPSILON, f32::max)`): the cell is the smallest weapon range.
- `refresh_slot_clearance` (`movement.rs:1110-1117`): the cap is the largest mover radius.

- [ ] **Step 1: Before.** Run the RUN126 probe (1 static of radius 4 and 5,000 movers; audit `:33`) and record the time. Where its source is, is NOT TRACED. If it is not in the tree, build it again from audit `:33`.
- [ ] **Step 2: Reference, change, green.** Refresh the reference from the A3 base. `kernel_reference` (1,000 to 5,000 movers) and `within_matches_brute_force_scan` give `exit=0`. No assertion edit.
- [ ] **Step 3: Commit, gate.** F-RUST, F-FROZEN (new hash), the GM tests (A2a step 4 commands), and `--p0` on four packages. Run the probe again and record the time. It is not judged.
- [ ] **Step 4: Plants.** First show each red, then restore: (a) the statics are queried with reach `r` only; (b) the merge is not sorted. In each case `kernel_reference` goes red.

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

| # | Risk | Early check |
| --- | --- | --- |
| 1 | The depletion repair conflicts with the S1 move. | D0p step 1: a trial merge as soon as D0 lands. |
| 2 | An orphan display holds the GPU lock (c667b08: 36 minutes). | Before each headed batch, confirm that no headed run is active. Root asks the CEO to stop an orphan. |
| 3 | A stream unit is on plain progress, or is separated or projected with no pool. Then P0 at A2b2 cannot be equal (D 9.8). | Q1b step 2, counts (a) to (c), on the base build. |
| 4 | The G1 counter has hits on the stream. | G1 step 4, right after D0p. |
| 5 | The switch of A2a is not bit-equal on some frame type (for example escape or Attack-Point). | A2a step 3: the full seed set, before the commit. |
| 6 | The fallback pool oscillates: it is collected and made again on each leg. Gate 2 M1; NOT TRACED whether this happens today. | A2b1 step 1: the counter test on a crowd of 20 gatherers. |
| 7 | More suite tests than the A2b1 and A2c lists go red. | At the start of each task: build with the first edit only, run `cargo test -p turts_sim -p turts_pack -p turts_server --locked --no-fail-fast; echo exit=$?`, and compare the red names with the list. |
| 8 | An MP1 lane started at E1 edits `economy/combat.rs` and conflicts with A2c and A3. | MP1 code goes in new files (section 5). Root checks `git diff --name-only` of each MP1 lane against the A2c and A3 file lists. |
| 9 | A1d changes an M3 failure text. | A1d step 1. |
| 10 | The R2 condition fails at A2a. | A2a step 4, at the first commit. |

## 9. Budget

- Remaining: **52 runs**, and **27 runs** on the critical path (54 h at 2 h per run; range 38 to 78 h), plus the D0 and CEO waits.
- Revision 1 of this plan had 49 runs and a path of 24. The changes: A2b (3 runs) is now A2b1 (3) and A2b2 (2), and A2c grows from 3 to 4.
- Root counts each dispatch. A task that uses twice its estimate is reported (C391 item 7).

## 10. Not traced by the producer

- How `tests/boundary-fixture` selects its rows.
- Which browser case reads `facing`.
- Where the speed and `dt` of the arrival-tick check are defined.
- The source of the RUN126 probe.
- Whether the `mc_orders.rs` tests are `#[ignore]`.
- The headed wrapper flags beyond its `--help` text.
- Whether `vault/Pre-MP1 Execution Board.md` exists.
- Whether `MoveProgress` needs a `failure_recorded` field for the legacy failure.
- Whether `UnitOrders.current` holds the order of a single move after A2c.
- Whether the fallback oscillation of gate 2 M1 occurs.
- Which type each of `crates/turts_sim/tests/orders.rs:383`, `:496` and `:555` reads.

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
| grep of `turts_sim::movement::` in `economylog.rs` | The validator uses a small API | It uses the step branches directly | 9 items |
| JSON scan of both registries | Which plants S2 and S45 must move | None | `orders.rs` 1; `visuals.rs` 2 |
| `sed -n` on the gate 2 anchors at `bfd5097` (`movement.rs:480, 907, 957, 1968, 1985, 2003, 2008, 2130, 2163, 2184, 2399`; `orders.rs:152, 178, 258, 1358, 2044, 2842, 3115, 3151, 3286, 3622, 3661`; `command.rs:173, 494`; `gather.rs:1246`) | The gate lines | Another item at a line | All match |
| grep of each plain symbol in `crates` and `tests`, with counts for each file | Ordering defect: a step deletes something that a later step still calls | — | Six defects found; see the table below |
| python regex count of `\bstep\(\s*&MovementInput` and `for _ in 0..8` in production `movement.rs` | D 9.13 "today 3" | Another count | 3 and 3 (`:793, 1993, 2174`; `:807, 2009, 2192`). The plain text `step(&MovementInput` gives 0, because rustfmt puts the argument on the next line |
| grep `PlainMoveProgress` and `PlainMoveOrderIdentities` by test function | Which tests change, and in which part | — | 8 tests in `group_movement.rs` (A2b1); `orders.rs` 3; `mc_order_rows.rs:650`; `mc_orders.rs:459` |
| `slice.py` revision 2 | No cycle; the path | A cycle | No cycle; 27 runs; 31 if R2 fails |

Ordering checks (Root's item 3). A "defect" is a step that deletes something that a later step still calls, or a check that cannot go green at its step.

| Item | Used later by | Result |
| --- | --- | --- |
| The plain reservations and identities (`orders/plain.rs`) | The plain move branch `:2842-2900`, until A2c | Defect, the gate's H2. Fixed: deleted in A2c commit 2. |
| `PlainMoveProgress`, created at `orders.rs:2044-2058` (legacy failure) | The live legacy envelope path | Defect, new. Fixed: A2b1 replaces the creator (R10). |
| `PlainMovePriorState.plain_progress` (`:171`, `:3208`, `:3249`) | The plain move code, which A2c deletes | No defect after A2b1 (the value is always `None`). A2b2 deletes the field. |
| `PlainMoveProgress` read by the trace emitter | The P0 pair at A2b2 and the pair of Q2 | Defect, new. Fixed: the probe API (R11). |
| `PlainMoveProgress` in the reference copy | The harness at A2b2 | Defect, new. Fixed: the copy changes in the same commit (A2b2). |
| One `for _ in 0..8` and one `step(` call in `step_plain_progress` | The D 9.13 guard at A2a | Defect, new. Fixed: the count in two stages (R12). |
| Eight suite tests read `PlainMoveProgress` | They go red when A2b1 removes the creators | Defect, new. Fixed: listed in A2b1 (R13). |
| `PlainMoveOrderIdentities` at `combat.rs:154`, `transaction.rs:736`, `views.rs:512`, `command.rs:268`, and in 3 test files | A2c commit 2 | No defect. Added to the A2c file list. A3 rebases on A2c for `combat.rs`. |
| `PlainGatherGoal` (replay validator 8 times, `gather.rs`, `orders.rs`) | A2b1 | No defect. The validator is gone at A1c, and A2b1 waits for A1c. The type survives inside `MoveProgress`. |
| `command.rs:494` `.remove::<PlainMoveProgress>()` on the direct `MoveTo` path | A2c reroute | No defect. A2b2 deletes the line; the path is dead after A2b1. |
| The `movement_envelope_capacity_available_after_replacing` call at `:559` | A2c | No defect. The item is kept out of `plain.rs` (S2). |
| The A2a counter and the harness need `test-probes` | A2b1, A2b2, A4, A3 | No defect. Q1b adds the feature first. |

## 14. Open questions for Root

- Accept or refuse R1 to R9 at gate 2.
- The re-check of the simulation lens is still open (D `:3`). This plan assumes that it passes as revision 4.
