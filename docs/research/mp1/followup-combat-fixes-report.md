RESULT=PASS

# Follow-up of mp1/combat-fixes (simulation-core-engineer, build mode)

Branch `mp1/combat-fixes-fu` from 315d5c74, pushed, head 7be9364c, not merged. Worktree
`/home/julien/Projects/TuRTS-mp1-combat-fixes`. Evidence: `/home/julien/Projects/TuRTS/target/mc-evidence/combatfu-*/command.log`.
Scratch: `scratchpad/combat-fixes/`.

## Commits (on 315d5c74)

- 72a90eca fix(sim): `weapon_stand_off` is exact by construction
- 12df5e12 docs: `docs/research/mp1/errata-combat.md` and the corrected note in `docs/measurements/mp1.md`
- 297af0c9 test: per-coordinate "not past the target" check, step count message
- 2743aa91 test: `step_toward` unit test
- 81f3777b plants 3a to 3f
- 7be9364c plants: 2a, 2b, 2c, 2e re-pinned to the new code, 2d dropped (its tuned bound is gone)

## 1. Exact stand-off

Files: `crates/turts_sim/src/economy/combat.rs` only (code and tests).

- Red first. The reviewer's reproduction and the new case-class test were run on the code at
  315d5c74 (`combatfu-red1`): the reproduction gives `stand-off Vec2(3713.1174, 3960.1672) is out of
  range 0.001 of Vec2(3713.1167, 3960.168)` (the reviewer's point exactly), and the case classes give
  `stand-off ... is past the target ... (range 0.001 ...)`. The old 2M-ring test stayed green.
- The form (`stand_off_steps`, `step_toward`, `combat.rs:~350-390`): the ring point is returned
  unchanged when `within_weapon_range` holds. If not, each coordinate that is not the target
  coordinate moves one quantum toward the target, until the rule holds. The quantum is the `f32`
  spacing at the larger magnitude of the two coordinates (the reviewer wrote "one ulp"; the ulp of the
  point coordinate alone is too fine when the ring coordinate is near 0 and the target coordinate is
  large, plant 3c shows 36 steps instead of 3). A coordinate within one quantum of the target lands on
  it, so a step never passes it.
- Why it ends (comment on `weapon_stand_off`): each step only lowers the two offsets; the rule is
  monotone in the offsets (rounded difference, square and sum do not fall when an offset grows); at
  offset 0 the rule holds for every range above 0 (`0 <= range * range`, also when the square rounds
  to 0). `STAND_OFF_MAX_STEPS` = 64 is the hard bound; `debug_assert!` on the final point.
- The step bound by the review's error bound is about 25 (derivation in the comment). Measured: at
  most 3 steps over 1.35 million rings of the case classes (`combatfu-green2`, line `most stand-off
  steps: 3`). The test asserts at most 25.
- `STAND_OFF_BOUNDS` and the "margin of four" text are removed.
- Tests: `weapon_stand_off_is_in_range_for_a_range_far_below_the_coordinates` (the reproduction);
  `weapon_stand_off_is_in_range_for_the_reviewed_case_classes` (fixed seeds: M in 64, 4096, 2^20; r in
  0.001, 0.05, 1, 4, 40; 4 axes, 4 diagonals, random bearings; 10,000 rings each = 1.35 M);
  `weapon_stand_off_is_in_range_for_random_rings` (kept, 2 M); `step_toward_reaches_the_target_coordinate_and_never_passes_it`
  (binade boundary, up and down). Each ring is checked for: in range, not past the target in each
  coordinate, unchanged when the ring point passes, step count below the bound.
  The 27.8 M search of the review is not run in a unit test (cost); the classes are.
- Plants (`docs/research/mp1/plants/combat-fixes.json`, row COMBAT-FIXES-FU), all RED-RESTORED
  (`combatfu-plants4`, `-plants5`): 3a no step (reproduction), 3b no step (classes), 3c quantum of the
  coordinate itself (`the largest step count is 36`), 3d a step may pass the target (the `step_toward`
  test), 3e bound 1 (`the stand-off took 1 steps`), 3f range rule widened (pin test, item 2).
  Plants 2a, 2b, 2c, 2e of the first pass were re-pinned (their anchors were gone; they now remove the
  loop and the `debug_assert`) and are RED-RESTORED (`combatfu-plants6`). 2d is dropped. A first run of
  3d on the first test (case classes) stayed green: the case where a step passes the target does not
  occur in those samples, so 3d is tested by the direct `step_toward` test.
- Check of the review items in code: LOW 1 (comment text) fixed; LOW 2 (degenerate regime) fixed and
  reproduced; LOW 3 (range below the radii) traced and pinned (item 2).

## 2. Errata and the pin test

- `docs/research/mp1/errata-combat.md`:
  (1) minimum range: content validation `content_m3_validation.rs:33` accepts any finite positive
  range; with the exact form a minimum range is moot for the approach; the remark that a range below the
  `f32` spacing at the coordinate can put the stand-off on the target point stays; status: needs
  decision only if a mod can author it.
  (2) centre to centre or edge to edge: open, for `game-designer` (X2). Two options stated.
- Pin test `a_range_below_the_two_radii_never_fires_and_the_unit_rests_beside_the_target` (radii 0.5 and
  0.5, ranges 0.4 and 0.9). What the unit does today: it does not stop at contact. It walks to a free
  slot beside the target (observed centre distance 1.5 and 2.0), rests there outside the range, the
  Attack order stays current, no failure is recorded, and the weapon never fires in 400 ticks. At range
  1.0 (equal to the radii sum) it stops at contact and fires at tick 27 (probe only, not pinned). The
  test pins "at least the radii sum and above the range", not the two numbers. The task text expected
  "stops at contact"; the fact differs, and the errata says so. Plant 3f shows the test red.

## 3. Measurement note

`docs/measurements/mp1.md`, subsection "Combat": stop distance 4.0000005 (squared 16.000004), the first
figure was wrong, root cause traced (b4012fc1, head 315d5c74), the repair named, the measurement taken
before the repair.

## Frozen values and gates (head 7be9364c; code last changed in 2743aa91)

- `tests/m3/m2_operation_stream.json` bcbf72c2206251f635a7c7f5b6c8f573d69a5fc05a5bd77716e98ebfb08e370b; re-emitted
  stream `cmp`-equal. Trace bodies equal: base d78dc093..., haul 887ce95f..., acme ba1c849a..., cedar 09f63732...
  (`combatfu-hashes-trace`, on 81f3777b; code unchanged after). `probe_neutral`: `frame_chains=d0d1b1c572163424`,
  `muster_world=aa9db740578ed5c1`. No re-freeze.
- fmt, clippy `-D warnings`, wasm32 check, `boundary.sh`, `mp1_plants.py --check --all` (272 records, 0 stale): exit 0.
- `cargo test --workspace --locked`: 1997 passed, 0 failed, 38 ignored. The formula ratchet is green with
  no count raised (the new code has no `distance_squared` or `length_squared`).
- `acceptance.sh`, `MC_JOBS=4`: exit 0, `source_unchanged` true, one run; `mc_kill_replica` passed in it.

## NOT TRACED

- The 27.8 M all-input search of the review was not re-run. The review's bound formula was used for the
  step estimate; the test measures at most 3.
- Negative coordinates (not reachable, the review says clamp) are not in the case classes.
- `clamp_target` after the stand-off (unchanged from the first pass).
- No headed run.
- The ring-on-target case (range below the `f32` spacing): the stand-off can equal the target point;
  what the unit then does is not tested.
