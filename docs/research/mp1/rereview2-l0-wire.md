VERDICT=PASS

# Second re-check of L0-WIRE (backend-engineer, review mode)

Candidate `67a3f388` (`mp1/l0-wire`). It contains `origin/mp1/integration` `54c39fdd`, the integration head at
fetch. Receipts: `/home/julien/Projects/TuRTS/target/mc-evidence/l0wire-rerev2-*`. The worktree was clean at the end
(`git status --porcelain -uall`: 0 lines).

My High of the first re-check is closed. No new High. Four Medium items and one Low item are below; none blocks the
merge of L0-WIRE. Item 3 (delta gaps) needs a decision on when to port the reorder repair.

## 1. The disabled producer, and the "one function"

- The committed test `crates/turts_client/tests/job_progress_clients.rs` is green (3 passed).
- Fault A: the view ignores the disabled set (`production.rs`, `view_job_per_tick`). Red: "continuous at tick 221:
  job 3 of producer 4 (sim 7, a disabled producer holds a Running job: true)", and the `base` test also fails.
- Fault B: the sim does not skip a disabled machine (`crates/turts_sim/src/economy/machine.rs:47-52`). Red: the same
  forge case (sim 9).
- Each fault was restored (`cmp` equal), and a fresh run was green.
- **Medium (the claim is only partly true).** `job_step_per_tick` (`crates/turts_sim/src/economy/production.rs:390-400`)
  is one function for the stage rule and the rate. The freeze rule exists twice:
  - The sim freezes by an early `continue` before it calls the function: `production.rs:300-305`, `machine.rs:47-52`,
    `research.rs:262-267`.
  - Machines and stations pass the literal `false` (`machine.rs:94-99`, `research.rs:306-311`).
  - The producer passes a `frozen` value that is always `false` at the call (`production.rs:339`), because a frozen
    producer was skipped first.
  - The wire freezes through `view.m3.disabled` (`production.rs:407-410`).
  - So a new freeze cause added to the sim skip only (for example "unpowered" in `L-PWR`) does not reach the wire.
    The client test is the guard; it caught both faults.
  - Fix: one predicate `is_frozen(world | view, producer)` used by the skip and by the view, or pass the real value
    and remove the skips. Record the rule for `L-PWR`.

## 2. Rate 1/2 and the frozen shape (`L-PWR`): no High

Example: Q = 2, a job of 100 ticks, so `total` = 200 units. At full rate the step is 2 units a tick
(`turts_rules::progress::rate_step`); at rate 1/2 it is 1 unit a tick.

- In base ticks (today's writer, Q = 1) the progress at rate 1/2 is 0.5 a tick. `per_tick: u64` cannot carry it.
- In raw units the record is `done` = units at `at_tick`, `total` = 200, `per_tick` = 1. All are integers.
  `value_at` gives units, and `base_ticks_done(units, duration_ticks, total)` (`progress.rs:82-88`) gives ticks
  (`duration_ticks` is in `JobState`).
- So `RateRecord` can carry the job with no protocol step. Only the writer and the client derivation change.
- **Medium (needs a ruling now):** the meaning of `done`, `total` and `per_tick` must not change silently inside
  protocol 5. Rule: they count raw units of 1/Q tick; today Q = 1, so they are equal to ticks. The client derives
  ticks with `base_ticks_done`.

## 3. The `mc_kill_replica` flake

What differs. With the dev edge `turts_server -> turts_client` added, the graph `cargo tree -p turts_server -e
normal,build,dev -e features` gains 893 feature lines. Among them: `bevy_ecs "multi_threaded"`,
`bevy_tasks "multi_threaded"`, `bevy "multi_threaded"` and `bevy "async_executor"`. So in that test build the server
binary runs its ECS and tasks on a thread pool, where it is single-threaded without the edge. Today:

| Build | `bevy_ecs "multi_threaded"` lines |
| --- | --- |
| server-only graph | 0 |
| workspace graph | 1 (from the client's normal edge) |

`scripts/acceptance.sh` builds and tests per package (`:50`, `:84`), so the process tests use the single-threaded
server.

Cause of the gap. The builder's log shows "last 495 delta tick 497 base 496" and "last 434 delta tick 436 base 435":
the delta after the next one arrived first. lightyear 0.31.0 sorts the messages of one flush by size
(`lightyear_transport-0.31.0/src/packet/priority_manager.rs:109`, `packing_order` at `:146`). The MC branch records
this defect and its repair: `docs/research/mc-reorder-errata.md` at `9d2ab86b` (the deltas of one flush can arrive out
of order when a frame runs several ticks). The MC repair is a client reorder window (`8365e8be`, merged `9d2ab86b`). It
is NOT on this branch and NOT on `mp1/integration` (`git merge-base --is-ancestor`: both no).

The multi-threaded server changes the frame timing, so frames with two or more ticks, and so flushes with two
deltas, come more often. The builder also saw a failure at load 15.6 without the edge. Protocol 5 makes delta sizes
vary more (records on change), and a size sort reorders only deltas of different sizes. This is a plausible increase;
it is NOT measured.

NOT TRACED: whether the missing delta arrived later (reorder) or never (loss). The log records only the gap. Over
WebSocket on loopback, a loss is unlikely; the size sort is the known mechanism.

Decision (needs decision, orchestrator):

- The port does not have to come before the merge of L0-WIRE: the defect is on `mp1/integration` today, and L0-WIRE
  adds no new cause.
- It must come before the first stage-1 lane that runs process rows under load (`L-MATCH-SRV1`: IMM1, MAT1; then
  LOB1, LEAVE1) and before C1.
- Until then, run `mc_kill_replica` and plant 1d on a quiet host.

The builder's rule "add no dev-dependency to `turts_server`" is correct as a guard. It hides the cause; it does not
remove it. Also, the client test now has dev edges on `turts_sim` and `turts_pack`. The crate table forbids
`turts_sim` for `turts_client`, but the boundary check reads only normal and build edges. Record this as an accepted
exception for test code.

## 4. The Snapshot oracle (`economy.rs`): Medium (instrument)

`measured_job_rates` (`crates/turts_server/tests/economy.rs:1847`) takes the rate from the audit, so it no longer
copies the export rule. The builder reports that no process scenario has a running job in a raw Snapshot, and that its
plant (wire rate + 1) stayed green. I did not re-run this process test (a server and client build per run). So the
oracle cannot go red today. The real guard is the client test (faults A and B were red).

Fix: give one raw-Snapshot scenario a running job, or write in the oracle that it cannot see a wrong rate.

## 5. Gates

- Workspace: 2,067 passed, 0 failed, 38 ignored. `turts_sim --features test-probes`: 402 passed.
- Clippy `-D warnings`: exit 0. `boundary.sh`: exit 0. `--check --all`: 324 apply, 0 stale.
- Frozen stream `bcbf72c2206251f6`.
- Trace bodies: base `d78dc09392e5b969`, haul `887ce95f69801702`, acme `ba1c849aaee46dd4`, cedar `09f637325c511972`.
- `probe_neutral`: `frame_chains=d0d1b1c572163424`, `muster_world=aa9db740578ed5c1`. All equal.

## 6. Merge check

- `origin/mp1/integration` (`54c39fdd`) is an ancestor of the candidate: nothing to merge.
- `origin/mp1/sound-off-default` (`314e2722`): `git merge-tree` is clean (tree `bb58d14a`). Its diff adds no literal
  of a shape that L0-WIRE changed (grep for `elapsed_ticks`, `.cooldown`, `JobState {`, `WeaponRecord {`, `Snapshot {`
  and the others: 0 lines).
- NOT TRACED: a build of that merge.
- Order: either side can merge first.

## Low

`view_job_per_tick` passes Q = 1 and the full rate (`production.rs:409`). It is correct in lane 0; `L-PWR` must
change it (see item 2).

## Not checked

Headed runs; `acceptance.sh`; the Snapshot oracle under a fault (item 4); the arrival order of the deltas in the
flake (item 3); a build of the sound-off merge.

## What I changed

none (two faults, each restored with `cmp` equal; a temporary dev edge in `crates/turts_server/Cargo.toml` and
`Cargo.lock` for `cargo tree`, restored from copies; the tree is clean).
