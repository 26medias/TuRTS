VERDICT=PASS

FINDINGS=2 HIGH=0 RUNS=14 NOT_CHECKED=3

# Re-check of the A2a repair (1bff266 on ab37cea)

Reviewer: qa-lead, 2026-11-12. Scope: only what changed (`frame.rs` +5, `reference.rs` 34 lines changed, `kernel_reference.rs`
+388). Receipts: `/home/julien/Projects/TuRTS/target/mc-evidence/a2a-rerev-*/`. Scripts:
`/tmp/claude-1000/-home-julien-Projects-TuRTS/bf418660-b272-4bb3-8628-8d9ef2677d1d/scratchpad/a2a-review/`.

The High of the first review is closed. The late-arrival, same-point and slow-clause branches now run, and a
fault in each one is red in release. F4 is also red in the debug CI form. One Medium stays open. It is a risk for
the tasks after A2a, and it does not block A2a.

## Findings

1. **Medium (for A2b1 to A3).** `crates/turts_sim/src/movement/mod.rs:123-125` `angle` has no pin. It is shared:
   `spread.rs:5` imports it and `spread.rs:236` calls it, and the reference and the live kernel both call
   `spread`. My fault F7 negates `angle`. `kernel_reference` stays **GREEN**, and `group_movement` (38 tests) also
   stays GREEN.
    - The live `step` is not a hole. F8 changes the operation order in the live `step`. It is RED at seed 1, tick 3,
      because the merged step calls the live `step` (`frame.rs:714`) and the reference calls its local copy.
      `plain.rs:160` still calls the live `step` on both sides. That is safe only while the direct call stays.
    - The types in `mod.rs` hold data, not behaviour. A2b1 adds a field to `MoveProgress` (plan `:880`), so a pin on
      the whole of `mod.rs` would be bumped at A2b1 for a reason that is not real.
    - What I require:
        - (a) Pin `angle` and `step` by item: hash the text of `fn angle` and `fn step` that a small scanner takes
          from `mod.rs`, not the whole file. Or copy `angle` into `reference.rs`, the same way as `step`. A copy alone
          is not sufficient, because `spread.rs` calls the live `angle` on both sides.
        - (b) Add a gate rule against a blind bump, because the pin test cannot see one. In each later task (A2b1,
          A2b2, A4, A3) and in REG, `git diff <task base> HEAD -- <pinned files and items>` must be empty, or the same
          commit must add the old text under `movement/reference*` and point the reference at it. The reviewer of each
          task runs this check. A3 edits `grid.rs` (plan row A3), so A3 is the first task that must copy a helper.
2. **Low.** Each anchor raises its count once or twice (`late_arrival_pooled: 1`, `late_arrival_free: 1`,
   `same_point: 2`, `slow_clause_*: 1`). That is enough: a fault in each branch is red on tick 0 of its anchor. The
   test asserts each count in its own run (`kernel_reference.rs` anchors loop), so a random seed cannot stand in for
   an anchor.

## Checks

1. `rtk proxy git diff ab37cea 1bff266 -- crates/turts_sim/src/movement/frame.rs`: one hunk of 5 added lines at
   `:921`, all `//` comments. No code line changed.
2. Faults in release (restored each time with `git checkout`; `cmp` against `git show 1bff266:` = 0; `git status`
   empty):

| Fault | Result | First difference |
| --- | --- | --- |
| F4: late arrival without `Resting` (`frame.rs:800-802`) | RED | anchor "late arrival, pool" tick 0 unit 1, progress Moving vs none |
| F5: `unit_id >` (`:766`) | RED | anchor "same point" tick 0 unit 1, position |
| (h): `probe.separated = separated` (`:792`) | RED | anchor "late arrival, pool" tick 0, probe.separated true vs false |
| F6 (new): `0.025` to `0.02` at `:650` | RED | anchor "slow clause, no pool" tick 0 |
| F7 (new): `angle` negated (`mod.rs:124`) | **GREEN** (also `group_movement` 38/38) | none (finding 1) |
| F8 (new): live `step`, `offset * step / distance` | RED | seed 1 tick 3 unit 11 |

3. Reach with my counters (`plant_cov.py`, release): same point 35 = 1 and 36 = 1; late arrival 37 = 1 and 38 = 1;
   with `separated` 39 = 1 and 56 = 1; slow clause 59 = 2. Before the repair, each of these was 0.
4. Debug CI form (`cargo test --workspace --locked --test kernel_reference`): F4 RED (same message, 6.70 s). Fresh
   green: 145,400 unit frames, every anchor count above 0, 11.29 s.
5. Pins: the five pinned sha256 values equal `git show 8722f8e:` for `plain.rs`, `pool_index.rs`, `search.rs`,
   `spread.rs` and `grid.rs`. `reference_helpers_are_unchanged` is green. See finding 1 for the hole.
6. Copy proof: `rtk proxy diff` of old `frame.rs:61-1104` against `reference.rs:61-` (1bff266) shows the same two
   hunks as before (the name and doc comment, `..Default::default()`). The local `fn step` is byte-equal (`cmp` = 0)
   to `mod.rs:141-156` at 8722f8e, except `pub`.
7. Equal values at 1bff266: the four traces and the stream are byte-equal (`cmp`) to the traces from the 8722f8e
   movement files (`d4c73ebe…`, `805309b2…`, `19dcf9a3…`, `0db10e01…`, stream `bcbf72c2…`). `probe_neutral`:
   `frame_chains=d0d1b1c572163424`, `muster_world=aa9db740578ed5c1`, feature off equals on.
8. Clean release run: 5,684,000 unit frames, 0 differences, 2 tests ok. `one_step_function` 3 ok.

## Commands (labels `a2a-rerev-*`)

release-green-1, f4-release, f5-release, h-release, f6-slow-clause, f7-angle-shared, f8-live-step, f4-debugws,
cov-release, trace-emit, probe-neutral, debugws-green (cargo through `exec_in_lane_v4.py --head 1bff266`), plus the
git diff and the `cmp` copy checks.

## NOT TRACED

- Whether a later task plans to edit `angle` or `step`.
- The builder's report text: I read only the commit.
- fmt and clippy on 1bff266.

What I changed: none in the repository. I wrote this file only (not committed).
