VERDICT=PASS

# Re-check of the A1c repair (pre-MP1 slice)

Head 3c53b96 on `pre-mp1/a1c-delete` (holds the merge of 86a5324). Reviewer: qa-lead, review mode,
2026-11-12. Lane `/home/julien/Projects/TuRTS-pre-a1c-review`, cache `target/mc-pre-a1c-review`,
labels `a1c-rerev-*` in `/home/julien/Projects/TuRTS/target/mc-evidence/`. Scripts are in the
scratchpad `a1c-review/` (`plant.py`, `run_plant2.sh`, `chain2.sh`, `headed_v6_premerge.sh`).
Line 2 of the summary: FINDINGS=4 HIGH=0 RUNS=24 NOT_CHECKED=3.

## Findings

1. **Medium. The sweep does not reach the physical-escape branch.** Plant S2 at
   `crates/turts_sim/src/movement/ecs.rs:113-115` (a unit in a physical escape gets `Owner(SlotId(1))`)
   leaves `movement::ecs::tests::the_movement_system_writes_only_movement_state` green. Only one
   other test is red, and only by chance: `group_movement.rs:2230` "normal queue/replacement
   continuation remains immediate" (`a1c-rerev-plant-s2`). The Muster scenario of the sweep
   (`ecs.rs:173-...`) makes no physical escape. So the module doc claim (`ecs.rs:10-12`: "a write
   to any other audited field is red") is true only for the branches that the scenario reaches.
   The stated gap (a constant write) is a different gap. Fix: add a blocked plain move to the sweep
   scenario (the fixture of `group_movement.rs:2230` makes an escape), and add a count that asserts
   the scenario reaches each branch of the write loop (target set or removed, progress, plain
   progress, escape). Show S2 red there.
2. **Low. The parallel runner defaults to the v4 wrapper.**
   `/home/julien/Projects/TuRTS/target/mc-evidence/root-regate/parallel_refutations.py:42`
   `HEADED = .../mc_headed_v4.sh`, and the call at `:274` has no `--mode`. The rule says never v4. I
   passed `--headed` with a shim that runs `mc_headed_v6.sh --mode pre-merge "$@"`. Fix: make v6 the
   default and pass `--mode` in the runner.
3. **Low. Host load.** The browser run started at load 9.7. During my parallel gates and the wasm
   rebuilds of the plant phases the load went to 13 to 18 (rule: below 12). All 8 cases still
   gave the expected verdicts.
4. **Low (info).** 11 of the 24 headed receipts list one survivor, the `Xvfb :100` of that call. The
   wrapper ended it. No process of the run was left at the end.

## 1. N10, N13, N3 and the ported rule

- N10 (`crates/turts_server/src/orders.rs:100` unit list empty): red in 16 tests (`a1c-rerev-plant-n10`).
  The ledger text is "orders: order command 1 differs from its immutable catalog definition in allowed
  units" (6 `mc_orders` log tests, `ord3_full_scene_…`, `e9_base_gather_trace…`,
  `muster_log_passes_the_ledger_audit`). The conversion test (`orders.rs:364` "muster:order_attack:
  allowed units") also fails, in 4 binaries.
- N13 (queue depth 255): red in 18 tests, the same families plus `session.rs:1750` and `:2760`
  (`a1c-rerev-plant-n13`).
- N3: red at `movement_resting.rs:184` "unit 21 landed after an attack-move with no MoveProgress" and
  `:232` (`a1c-rerev-plant-n3`).
- Field-by-field comparison with `8722f8e:crates/turts_audit/src/replay.rs:1789-1927` against
  `crates/turts_audit/src/orders.rs`: the new rule has every condition of the old rule. These are: the
  catalog must be present; exactly one definition; no duplicate capability entry; the Remove
  capability against `removal.by_command`; behaviour; target kind; unit list as a set; queue depth;
  the policy must be present; the policy id, behaviour, target, fire, engage, units as a set, queue
  depth and capabilities as a set; the behaviour/target pair. One change: the Remove subject finds its
  content id in `economy.units` instead of `raw_units.unit_type` and `unit_types`. That is the same
  registry source, because the ledger view has no `unit_types`. The rule runs for every command
  boundary (`lib.rs:167`). Neither rule compares the order target, so I made fault N14 for that field:
  the server swaps the axes of a point target (`orders.rs:44`). N14 is red at `mc_orders.rs:253`
  ("muster:rush target adapter must preserve Point") and at `:624` (`a1c-rerev-plant-n14`). So a
  named test holds the target field.

## 2. The Muster fixture with the server conversion

This is not a comparison of the conversion with itself. The rule takes its expected values from the
catalog snapshot in the log (`before.catalog`, built by `turts_sim/src/economy/views.rs:1034-1069`
from the installed catalog). It does not take them from `to_sim`. N10 is red in the ledger on this
fixture (`muster_log_passes_the_ledger_audit`) and on the `mc_orders` logs, not only in the unit test.
The capability formula has three copies: `to_sim` (`orders.rs:70-91`), `views.rs:1049-1069` and the
unit test (`orders.rs:380-401`). The sim does not read `policy` (`orders/mod.rs:140` is its only
field), so a fault there has no effect on play. Verdict: enough.

## 3. The sweep

- S1 (common branch; a unit with no target gets `Owner(SlotId(0))`, `ecs.rs:97-99`): red at
  `ecs.rs:244` "tick 0: the movement system changed an audited field that is not movement state", and
  in 84 other tests (`a1c-rerev-plant-s1`).
- S2 (the rare escape branch): green in the sweep (finding 1).
- The stated gap (a constant write that holds at each snapshot after tick 0): I accept it. The tick-0
  call on the installed world catches an owner write (S1 above, and the builder's own plant). The
  real gap is branch reach.

## 4. Sample of the builder table (combined set, 865 tests)

| Plant | Red tests (builder) | Red tests (this run) |
| --- | --- | --- |
| F1 slots | 11 | 11 (`a1c-rerev-plant-f1`) |
| F3 cost - 1 | 38 | 38 (`-f3`) |
| F5 `unit_orders` empty | 3 | 3 (`-f5`) |
| N1 ECS speed | 32 | 32 (`-n1`) |
| N7 gather reach | 19 | 19 (`-n7`) |
| N9 refusal reason | 7 | 7 (`-n9`) |

For each plant the anchor occurs once, I restored the file by copy, and `git status --porcelain` was
empty after each run.

## 5. Equal values (whole files)

The trace sha256 values are equal: base d4c73ebe…, haul 805309b2…, acme 19dcf9a3…, cedar 0db10e01…
(`a1c-rerev-m2-trace`). The stream bcbf72c2… is equal, and `cmp` with the copy of the emitter gives 0.
`probe_neutral` printed `frame_chains=d0d1b1c572163424` and `muster_world=aa9db740578ed5c1` and exited
with 0 (`a1c-rerev-probe-neutral`).

## 6. Gates

- Combined set (`-p turts_sim -p turts_audit -p turts_checks -p turts_pack -p turts_server --locked
  --no-fail-fast --features turts_sim/test-probes`): 865/865, 310 s (`a1c-rerev-combo-green`).
- Release `kernel_reference`: 2/2, 57.4 s (`a1c-rerev-kernel-release`).
- `cargo clippy --workspace --all-targets --locked -- -D warnings`: exit 0 (`a1c-rerev-clippy`).
- `bash scripts/boundary.sh`: "boundary ok", exit 0 (`a1c-rerev-boundary`).

## 7. The 8 headed ci-browser cases

I ran `parallel_refutations.py --set ci-browser` with 4 workers, 6 seed jobs and 2 wasm slots, the
lane target `mc-pre-a1c-review` and the head 3c53b96. The headed wrapper was v6 in pre-merge mode,
through the shim; the hash of `mc_headed_v6.sh` is 1d3a0000…, which is the hash in the errata. The
builds ran outside the lock. Result: 8/8 RED-RESTORED, `"failed": []`, wall 3,369 s, seed 437 s; the
longest hold was 53 s. Evidence: `/home/julien/Projects/TuRTS/target/mc-evidence/root-regate/runs/a1c-rerev-browser`.
The base cases read the private economy log (`m2-oracle.js:522`), so the new log format passed the
real browser oracles. I deleted the build root `target/par-browser` after the run.

## NOT TRACED

- `scripts/acceptance.sh` (the builder reports exit 0); `cargo test -p turts_client`.
- The play effect of S2, N3 and N10.
- Plants N2, N4, N5, N6, N8 and N12 on this head (not sampled).

What I changed: none in the repository; the lane is clean at 3c53b96. This report file only, on
request.
