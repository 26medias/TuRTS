RULING=DONE

# Ruling: the A2b1 stop items and the two probe values

Seat: `technical-director`, answer mode. Date: 2026-11-12. Candidate: `pre-mp1/a2b1-fallback`, head `3e85f88b`,
base `afb9c6de` (worktree `/home/julien/Projects/TuRTS-pre-a2b1`). Plan: "P" =
`docs/superpowers/plans/2026-11-12-pre-mp1-slice.md` (TuRTS-mp1). Design: "D" =
`docs/superpowers/specs/2026-11-12-pre-mp1-slice-design.md` revision 6 (in the worktree). All code lines are at
`3e85f88b` unless the line names the base. No gated or decided text must change; every change below is a plan
erratum that Root writes.

## 0. Governing texts and their status

- D section 9 is binding (P `:8`, `:25-30`). D 9.17 (Blocked keeps Blocked), 9.18 (the A2b1 harness rule), 9.19
  (collection by `gather_goal`) and 9.21 (the stricter assertion rule) govern A2b1.
- P section 1a (`:214-223`): four permitted kinds; "Any other assertion change, and any extra red test, stops the
  task." R13 (`:261`) listed eight `group_movement.rs` tests only. The five stop items are R13 omissions. The
  builder was right to stop (P `:910`).
- Conditions for every allowed item below (D 9.21, `:268-272`): the scenario, the tick count, the observation tick
  and the message stay the same; only the read expression changes, unless the item says "inversion"; the review
  shows each changed hunk with its kind, old and new side by side; a new expected value comes from D text or from
  an independent calculation that the test states, never from a run of the code under test; the reviewer shows
  one plant red in each changed test.

## 1. The stop items

**Item 1. `movement/tests/plain_movement_tests.rs:187` and `:394`. ALLOWED, observation change (kind 2).**
- Tests: `plain_move_respects_reserved_spacing_before_using_a_zero_primary_point` (`:146`) and
  `plain_move_with_zero_legal_primary_slots_blocks_before_moving` (`:359`). Both call the kernel on a frame with
  no injected plain progress, so the fallback fires inside the step. The test reads the fallback result.
- Property: a unit with no pool whose goal is blocked, and whose pool of one has no legal primary slot, stops in
  the same step: `Blocked`, target `None`, position unchanged, slot equal to the point, no spare slot.
- Mapping: `output.plain_progress` -> `output.progress` filtered by `is_fallback()`. `activity` and `slot` keep
  their expected values (`Blocked`; `(1.0, 1.0)` at `:396`). `pool_slots.is_empty()` -> the request pool of that
  unit from `step_frame_with_fallbacks(..).fallbacks` (`frame.rs:85-89`): `pool.slots.len() <= 1`. This is the
  exact equivalent: the base built `pool_slots` as `pool.slots.into_iter().skip(1).take(6)` (base
  `movement/plain.rs:297`). Do not replace it with `primary_count == 0`; that is another property. The builder
  can add it as a second assertion, with the reason `fallback.rs:54-58`.
- Stop condition: if the slot is not `(1.0, 1.0)`, the slot rule changed. That is not an observation change. Stop.
- A2b2: P `:958` deletes `plain_movement_tests` and ports two tests. `:187` is one of the two ports. `:394` is
  not, but after A2b1 it tests the fallback, which still exists, so kind 3 ("the tested item no longer exists")
  does not apply. Ruling: A2b2 ports `:394` as a third port with its values unchanged.

**Item 2. `orders/tests.rs:2692`. ALLOWED, observation change (kind 2), with a fixed injection shape.**
- Test: `authored_plain_blocked_identity_is_acknowledged_while_legacy_multi_waits` (`:2620`). It injects a
  `PlainMoveProgress` (`:2646-2656`). After A2b1 the order phase reads only the fallback `MoveProgress`
  (`orders/run.rs:136-143`, `fallback_progress` at `:429-445`), so the acknowledgement does not happen and
  `:2692` panics.
- Property: when a new move replaces an authored plain move whose trip is `Blocked` and not yet acknowledged, the
  order phase records one `Blocked` failure for the prior order at the replacement tick, marks it as recorded,
  and keeps the legacy multi-move pending.
- Condition: inject the state that the production writer makes, not a free shape. `MoveProgress` with
  `fallback: Some(MoveFallback { failure_recorded: false })`, a `pool_id` taken from
  `PendingMovementOrders::next_pool_id` (increment the counter), and a `MovementPools` entry with that id,
  point `(20, 20)`, and at most one slot (the equivalent of `pool_slots: Vec::new()`). Keep `Blocked`,
  `stalled_ticks 20`, `claim_count 3`, `best_distance`. Reason: with no pool entry, `fallback_progress` gives
  the point `None`, and `run.rs:111` then compares `Some(slot) != None`, which is always true. A test with no
  pool entry tests a state that production never makes. The read at `:2696-2701` becomes
  `MoveProgress.fallback.failure_recorded`. All other values stay.

**Item 3. `crates/turts_sim/tests/frame_hash.rs:211` (`plain > 0`). ALLOWED, observation change, plus one repair.**
- Property: the 16 chains reach the obstacle fallback, so the neutrality hash covers the fallback code. New read:
  the count of fallback requests is above 0. Message: "no unit reached a fallback pool".
- Repair (required): the chain must hold states that production can reach. Today the chain calls `step_frame`
  (`frame_hash.rs:189`), which drops the requests (`frame.rs:68-70`). Each fallback unit then carries pool id 0
  into later frames with no pool in `frame.pools` (`frame.rs:640-644`). Production never has this state
  (`movement/ecs.rs:115` gives the ids in the same system run). Use `step_frame_with_fallbacks` and
  `assign_fallback_pools` with a chain counter, and add the pools to `frame.pools`, as
  `kernel_reference.rs:936-944` does.

**Item 4. `crates/turts_sim/tests/probe_flags.rs:117`. NOT an observation change. It is an inversion (kind 1) that
the design allows. It needs a plan erratum row (Root), not the CEO.**
- Test: `a_unit_stepped_into_a_neighbour_reads_projected_then_plain_and_separated` (`:105`). A single Muster
  `MoveTo` (no pool until A2c) is projected on tick 1 and took a plain progress on the base. D `:21` and row A2b
  (`:44`, "sim behaviour for a blocked trip") give it a pool of one now.
- Tick 1: `was_projected Some(true)` and `separated false` stay (the step with no pool runs before the fallback,
  `frame.rs:919-937`). `unit_state` "plain" -> "pool"; `activity` "none" -> "moving". Sources: D `:21`;
  `fallback.rs:54-58` (`Moving` when the pool has a primary slot). The test states why a primary slot exists
  (no static near the point `(60, 18)`).
- Ticks 2 to 12: the old pins describe the plain branch, which has no creator after A2b1. Do not take new tick
  numbers from a run. Replace them with a property that the geometry gives and the test states: the line from the
  unit to `(60, 18)` passes through trooper 2 at `(26.1, 18)`, so over ticks `2..=N` the pool branch reports
  `Some(true)` on one or more ticks and `Some(false)` on a later tick, and `unit_state` stays "pool" until
  arrival. This keeps the purpose of the test (`:96-103`: a branch that stores a constant reads wrong). Plant: the
  pool branch stores a constant `was_projected`; the test goes red.
- No rename in A2b1 (the REG test-name compare, P `:1089`). Update the doc comment. A2b2 deletes the "plain" arm
  of `unit_state` (P `:952`) and can rename the test with a REG row.

**Item 5. `crates/turts_server/tests/ledger_logs.rs:530`. ALLOWED, inversion (kind 1), with one added assertion.**
- Cause: D `:22` and P `:873` ("After A2b1, no code creates a `PlainMoveProgress`"). On the base, the tick-12
  `MoveTo` of unit 41 (`:413-421`) filled `plain_movement_progress` in the Muster fixture (`MUSTER_EMPTY`,
  `:474-486`, does not list it). Now no fixture can fill it.
- Change: add `plain_movement_progress` to `MUSTER_EMPTY` and to `NEVER_FILLED` (`:490`). The doc line of
  `NEVER_FILLED` gives the reason: "no creator after A2b1; A2b2 deletes the field (P `:949`)".
- Condition: in the same test, assert that one or more sampled Muster states hold a `movement_progress` entry
  with the key `fallback`. Without it, the new `MoveProgress` fields `gather_goal` and `fallback`
  (`movement/mod.rs:70-81`, serde skip when `None`) are never checked by the keep test. If no sampled state has
  it, stop and report.

**The two reads `group_movement.rs:477` and `:3022`. ALLOWED.** They are in tests that P names (`:373` at P
`:899`; `:2975` at P `:906`). The P line numbers were stale. Both apply the P `:898` map. One more note for the
reviewer: the hunk at `:2083` is "tightened", not "observation". The old `pool_slots.len() <= 6` was always true
(`take(6)`, base `plain.rs:297`). The new `slots - primary_count <= 6` can fail. It matches the message. Accept
it and label it.

**Plant (c), P `:927`. The named instruments cannot see it.** `kernel_reference` compares no pool id: the ids are
given after the compare (`kernel_reference.rs:904-940`), from the harness counter (`:892`). The stream packages
have 0 fallbacks (P `:920`). The red instrument is `fallback_pool_ids_come_from_the_order_counter_in_unit_id_order`
(`group_movement.rs:3825`, assertions `:3865-3891`), registry row `tests/m2/refutations.json:1591-1607`. Accept it.
The erratum replaces the P `:927` text. The reviewer shows the plant red in that test.

## 2. The two probe values

**(a) Expected by the design, not a defect, under one condition.**
- D answers the question. A unit with no pool does not move to the old positions after its fallback. D 9.18
  (`:255-258`): "after a fallback the unit has a pool, takes the pool expressions and other stall and spare
  logic ..., and in a chain of 300 ticks the difference spreads to its neighbours". D 9.8 (`:210-212`): the pool
  expressions win. D `:44`: "sim behaviour for a blocked trip". The slot rule also changed: the base took
  `pool.slots.first()` (base `plain.rs:296`); the fallback takes the spread assignment (`fallback.rs:46-50`).
- Before the first fallback, positions must stay bit-equal (D 9.18: "bit-equal on every seed whose reference run
  has no fallback").
- Why the stream stays and Muster changes: the four stream packages spend 0 unit-ticks with plain progress
  (classifier `--counts`: plain 0; P `:920`; D `:44` "Stream unchanged (the streams spend 0 unit-ticks on
  `PlainMoveProgress`)"). No fallback fires there. Muster does reach the fallback: on the base a single Muster
  `MoveTo` took a plain progress on tick 1 (`probe_flags.rs:105-117`), and the Muster ledger fixture filled
  `plain_movement_progress` (item 5). `muster_world` sends 20 single `MoveTo` to points 0.5 apart
  (`frame_hash.rs:274-285`), and a trooper has a footprint radius of 0.5, so the goals overlap.
- `frame_chains` changes for a second reason: it hashes the `Debug` text of `progress` and `plain_progress`
  (`frame_hash.rs:174-175`). The change of representation alone changes it. So `frame_chains` is a neutrality
  witness only, never a "no behaviour change" witness across a change of the frame output types.
- Condition (A2b1 evidence, before merge): a differential run of the `muster_world` scenario, base against head,
  that writes the position bits of each unit and tick and, on the head, the fallback ticks and units. It must
  show: every tick before the first head fallback is bit-equal; at that tick, every unit with a lower id is
  bit-equal. Record the command, the first fallback (tick, unit) and the first difference (tick, unit) in the
  A2b1 report. A difference before the first fallback is an unintended change: stop.

**(b) What proves that the new positions are the intended ones.** The present guards prove liveness and safety,
not positions: `two_gatherers_deliver_to_one_receiver_in_one_tick` (deadline, no overlap, delivery;
`group_movement.rs:3711`), `each_trip_makes_at_most_one_fallback_pool` (no oscillation; `:3778`), the id test
(`:3825`), the eight R13 tests (state reads), and the phase counter of `one_step_function.rs`. No test pins a
fallback slot by the MC design 6.3 rule (grep: none). This is not enough. The builder adds, before merge:
1. `fallback_takes_the_nearest_free_hex_slot`. A trip with no pool (a `MoveTarget` written as an engine trip
   writes it) to P, with a resting unit on P. The test computes the expected slot from MC design 6.3
   (`2026-10-29-mc-player-control-design.md:611-616`): `s = 2 x r_max + 0.1`; the first hex ring at distance s;
   a slot within s of a reserved cell is skipped. Put statics on five of the six first-ring points, so that one
   slot is legal and the angle tie rule does not decide. Assert: the expected slot within 1e-4; the unit ends
   `Resting` with its position equal to its slot. Plant: the fallback takes `pool.point` as the slot; red.
2. The per-tick compare of section 3 (required from A2b2; recommended at A2b1).

A Muster scenario with expected positions derived in the test: no. 40 units over 120 ticks have no closed form,
and a value from a run is not permitted (D 9.21).

**(c) How the values are recorded, and the rule for later tasks.**
- No file pins them, on purpose (`frame_hash.rs:13-17`; `scripts/qa/probe_neutral.sh:13-16`). The check is
  feature off against feature on (`probe_neutral.sh:46`, `cmp`). Do not add a pin.
- The values after A2b1 are the values after the item-3 repair. The repair changes `frame_chains` again, so
  `67f1870e733807dc` is not the value to record.
- Who records: the A2b1 builder in its report; Root in the P binding block (a new entry "From the A2b1 stop
  ruling") and on the board; Root adds one line to the MP1 errata list, because MP1 lane 0 reports compare to
  `d0d1b1c572163424` and `aa9db740578ed5c1` (for example `docs/research/mp1/review-l0-sim.md:11`).
- Rule for every later task: compare to the values of its own base commit, on the same machine, never to a
  remembered constant. `probe_neutral.sh` exit 0 stays required.
    - A2b2 commit 1 (the switch goes): both values can change (D 9.8; P `:938`), only with the harness rule of P
      `:938` green. Record the new values.
    - A2b2 commit 2 (the deletion): `muster_world` must equal commit 1. `frame_chains` changes by representation
      (`frame_hash.rs:154`, `:175` read `plain_progress`). The builder runs commit 1 with those two hash lines
      removed (scratch edit, not committed) and shows `cmp` equal to commit 2.
    - A2c: `muster_world` can change (single moves take the pool path, D `:45`). Q2 records the new values next
      to the new stream sha.
    - A4 and A3: bit-equal refactors (D `:46-47`). Both values equal those of their own base, and after the merge
      onto Q2, those of the integration head.

**(d) No CEO approval for the probe values.** Quotes:
- P `:414`: "Nothing else in this slice needs the CEO."
- P `:196-197`: the frozen value is "`sha256sum tests/m3/m2_operation_stream.json` prints
  `bcbf72c2…e370b` ... until the Q2 commit". P `:208`: stop if "a frozen value changes outside Q2".
- D `:32`: "The MC frozen stream hash `bcbf72c2…` stays until step A2c." D `:146`: "G1, A2a and A2b change no
  frozen value." The stream did not change (section 5, run 1).
- `frame_hash.rs:13`: "There is no pinned value, on purpose". `:16-17`: "A later task that changes movement on
  purpose changes the hash."
- The behaviour change itself is CEO touch point 1 (P `:401-406`; D `:30` "The CEO is told before step A2b").
  NOT TRACED: I found no record of the CEO answer in `vault/`. Condition: Root confirms the recorded answer
  before A2b1 merges. With no answer, A2b1 does not merge (P `:406`).

## 3. `kernel_reference` after A2b1

**At A2b1.** The D 9.18 cut-off is binding and allowed. Two instrument defects, repair before merge:
- K1. The coverage counts include frames that the harness does not compare. `seen.fallback` (`:939`) and
  `seen.count` (`:941`) run on every tick; `compared` is separate. The checks "each kind above 0"
  (`:1043-1065`) can pass on a kind that no compare reached. With 55 of 64 random seeds cut at tick 0, this is
  real. Repair: count each kind on compared unit frames only, and assert each above 0 (the fallback count stays
  on the live run).
- K2. The header (`:43-44`) says the test "fails if no random seed is free of a fallback". The code
  (`:1006-1010`) counts random and pooled chains together, and the pooled chains have no movers with no pool
  (`pooled_frame`, `:811-818`), so it cannot go red. Repair: assert `fallback_seeds.len() < seeds()` and print
  the count of random seeds compared on all ticks.

**From A2b2 to A3.** The cut-off is void. D 9.18 is titled "The A2b1 harness rule" (`:255`); P `:969` says
"bit-equal on every remaining frame"; P `:1060`, `:1078` say bit-equal. With the reference refreshed from the
task base, both kernels make fallback pools, and the present code fails at `:904-917` (live first fallback
`Some`, reference `None`). Rule: the reference returns its fallback requests; the harness asserts that the live
and the reference requests are bit-equal on every tick (unit id, point, slots, primary count), compares all 300
ticks of all 128 chains, and keeps only the A2b2 commit-1 exception of P `:938`. Coverage counts on compared
frames; "fallback pool, compared" above 0.

Why a per-tick compare is valid after a fallback: the chain feeds the live output into both kernels on each tick
(`kernel_reference.rs:942`, `next_frame(&frame, &live, ..)`). The reference never runs a diverged chain of its
own. So the D 9.18 reason "the difference spreads to its neighbours" does not hold for this harness. At A2b1 the
builder can resume the compare on each tick, with exceptions only for: a unit that gets a new fallback in this
tick and the units after it in id order; a unit whose input is a `Blocked` fallback (D 9.17); a fallback with a
`gather_goal` (D 9.19).

## 4. Plan errata for Root

1. P 1a table: rows for items 1 to 5, the two reads and the `:2083` tightening, with the kinds above.
2. P `:927`: plant (c) is red in `fallback_pool_ids_come_from_the_order_counter_in_unit_id_order`.
3. Task A2b1: the hex-slot test, the Muster differential, K1, K2, the item-3 repair.
4. Task A2b2: `:394` is a third port; the full-compare harness rule; the `frame_chains` rule of commit 2.
5. Binding block: the probe value rule of 2(c). MP1 errata list: one line.

## 5. What I ran

| Command | Claim tested | Result that would refute | Observed |
| --- | --- | --- | --- |
| `git diff --quiet afb9c6de 3e85f88b -- tests/m3/`; `git show 3e85f88b:tests/m3/m2_operation_stream.json \| sha256sum` | builder: frozen stream unchanged; D `:146` | exit 1, or another sha | exit 0; `bcbf72c2…e370b` |
| grep `PlainMoveProgress {` and `make_plain_progress(` in `crates/turts_sim/src` | P `:873`: no creator after A2b1 | a hit outside the type, the fn body, `reference.rs`, tests | struct and fn body only; 0 calls |
| grep `d0d1b1c572163424`, `aa9db740578ed5c1` in plans, specs, scripts, research | builder: "the plan has no line on these values" | a pin in a plan, spec or script | hits only in 12 research reports |
| read `frame_hash.rs:13-17`, `probe_neutral.sh:13-16,46` | the values are pinned | a pinned constant | no pin; off/on `cmp` only |
| grep the touch-point-1 sentence in both `vault/` trees | the CEO answered touch point 1 | a vault record | no hit: NOT TRACED |
| grep `hex`, `ring` in the new `group_movement.rs` tests | a test pins a fallback slot by MC 6.3 | a hit | none |
| read `kernel_reference.rs:892-944`, `:1006-1065` | the coverage counts prove compared frames | counts inside the compare block | counts on every tick |
| `git diff afb9c6de 3e85f88b -- orders/run.rs`, `group_movement.rs`, `orders/tests.rs` | the mechanism of item 2 and the two reads | the ack still reads `PlainMoveProgress` | it reads the fallback `MoveProgress` |

Not checked: no `cargo` run (this mode permits no build), so the red states of the five items are the builder's
report plus my reading of the code; whether the item-4 property holds on the head; EL1 with an empty
`plain_movement_progress`; the A2b1 plants (a) and (b).
