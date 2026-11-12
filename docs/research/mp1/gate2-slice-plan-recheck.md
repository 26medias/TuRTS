# Pre-MP1 slice gate 2, re-check of plan revision 2: simulation-core-engineer

Source: subagent hand-back, MP1 brainstorm input, 2026-11-11. Agent output, not reviewed; claims are leads until verified.

VERDICT=FAIL FINDINGS=8 HIGH=2

The plan closes all 12 of my gate 2 findings. It fails on two new High findings in A2b1: one rule is not true, and one harness rule cannot be met. Both make A2b1 stop. I made no new run. The checks are reads of `bfd5097`, plus my gate 2 probe outputs. `git status --porcelain` in TuRTS-mp1 prints nothing.

## (1) My gate 2 findings: all closed by plan text

| Finding | Closed at | What closes it |
|---|---|---|
| H1 | plan `:683-688` and `:800-803` | "selects each of the three expressions by the presence of a pool"; A2b2 commit 1 |
| H2 | plan `:823`, A2c commit 2 `:849` | "A2c deletes it after the reroute" |
| M1 | `:742-747`, `:749`, `:777` | pool-id source, collection rule, fallback counter (see M-a below) |
| M2 | S2 `:432-441`, S3 `:478-481` | item lists; my count of the S2 list is 17 |
| M3 | `:461-465`, `:497-501` | item_equal plus a behaviour plant, first shown red |
| M4 | A2b1 and A2b2, files `:751-759` | two tasks |
| M5 | R12, `:705`, `:827` | the source counts |
| M6 | `:690`, section 1a row | "A2a changes no assertion" |
| L1 | G1 | bounding-box form |
| L2 | R2 | `cargo test -p turts_audit` and EL1 |
| L3 | `:646` | `was_projected` |
| L4 | `:846` | `command.rs:173-174` |

## New findings

**High**

**H-a. R13 "the expected value stays the same" is false for at least 2 of the 8 tests. Needs decision.**
- Today a `Blocked` plain progress with no target stays `Blocked` for ever (`movement.rs:718-736`).
- A `Blocked` pool clears its progress and its target in the next frame when the body has no overlap (`movement.rs:1670-1697`).
- `group_movement.rs:2757` runs 200 ticks, then expects `Blocked` and `failure_recorded`.
- `group_movement.rs:2975` expects `MoveProgress == None` and also a `Blocked` plain progress (`:3019-3030`). R10 writes a `Blocked` pool there, so `MoveProgress` becomes `Some`. That contradicts the first assertion.
- Tests `:2005` and `:2156` take the other `Blocked` recovery, so their values can change too (NOT TRACED).
- Options:
    - (i) D rule: "A `Blocked` pool of one from a fallback or a legacy failure keeps `Blocked` until a new order or a new trip. It is not cleared at `:1678-1697`." Then R13 holds.
    - (ii) Accept the behaviour of the pool model, and list `:2757`, `:2975` (and `:2005`, `:2156` if they go red) as inversions, with values from D.
- I recommend (i). It keeps A2b1 small and is closest to today.

**H-b. The A2b1 harness rule cannot be met (plan `:785`).**
- The plan says a frame with a fallback "differs only in `unit_state` and in the slot". That is not true:
    - after the fallback, the unit has a pool, so it takes the pool expressions;
    - `step_plain_progress` uses other expressions: `:769`, `:785`, `:803-804`;
    - it also has other stall and spare logic (`:856-898` against `:2272-2343`);
    - the seeds run as chains of 300 ticks, so the difference spreads to the neighbours.
- Text: "The harness is bit-equal on every seed whose reference run has no fallback. It counts the seeds with a fallback and lists them. Those seeds are proved by the guards of A2b1, not by bit equality."

**Medium**

**M-a. The collection rule needs a pure input.**
- `step_frame_preserving_resting` cannot see the gather stage.
- Text: "A fallback pool is not collected while `MoveProgress.gather_goal` is `Some`. `run_orders` clears `gather_goal` when the gather is reached or its endpoint changes (`orders.rs:771-783`)." Then the kernel needs no economy input.

**M-b. R11 needs a proof that the feature changes no result. Text for Q1b:**
- "Code under `cfg(feature = "test-probes")` only writes counters and probe fields. No `cfg(not(...))`. No probe call inside a condition or an expression whose value the sim uses."
- "A test `frame_hash` runs 16 seeded frame chains and prints one hash. Run it with and without `--features test-probes`. `cmp` of the two outputs gives `exit=0`."
- "The stream hash is the same in `cargo test -p turts_pack` (feature on through the dev-dependency) and in a run of `m3_m2_stream` with that dev-dependency feature removed."
- Note that `cargo test --workspace` unifies the feature into every test build.

**M-c. Section 1a is not strict enough, and one label is wrong.**
- Add to kind 2: "The scenario, the tick count, the observation tick and the message stay the same. Only the read expression changes. The review shows each changed hunk with its kind."
- Kind 1: "a gate probe" is a value from a run of the code. Text: "D text, or an independent calculation that the test states. A gate probe value only where D cites it."
- The hex-ring value at `:761` is an inversion. Label it as an inversion.

**Low**

- **L-a.** R12: count only code. Strip comments and string literals first, so a doc line cannot match the regex.
- **L-b.** Section 5: the M4 row says "A4 (19)". R3 merges A4 only after Q2, so the offset is at least 24.
- **L-c. Question (3).** I found no deletion that a later step still calls:
    - A2b2 removes `PlainMovePriorState.plain_progress` (`:3208`, `:3249`);
    - A2c deletes the 17 items and lists their callers, `prepare_legacy_move_to_waiters :603` included.

R10 and R12 are sound for the engine. R11 is sound once M-b is in the plan. R13 is sound only after H-a is decided.

## What I changed

None in the repository.
