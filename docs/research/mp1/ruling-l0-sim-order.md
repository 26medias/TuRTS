RULING=DONE

# Ruling: the tick order that L0-SIM freezes (technical-director, 2026-11-12)

Mode: answer, binding. Code: worktree `TuRTS-mp1-l0-sim` at 451f7b6. Design: revision 6.2 (D = the design file).
Status of every design line cited here: gated design.

## H1 and H2 (orchestrator rulings): I agree

- H1. D:520-523: one computation "at the start of the economy phase"; "A removal in tick t takes effect at the
  computation of tick t+1". `CommandKind::Remove` applies in `drain_commands` (`command.rs:264-272`), in
  `SimSet::Commands` (`plugin.rs:144-147`), before `Supply` (`plugin.rs:119`). Player orders apply only there:
  `apply_order` has two callers, both in `command.rs:201`, `:212`. So Supply before Commands is the only fix that
  changes no design text. The other removal path (death in Combat) and the completions (Production) come after it.
- H2. D:644: "Jobs pay in ascending unit id of their owner (producer or site), then in queue order." Today
  `run_production` walks `ProductionQueues`, a `BTreeMap<u32, _>` (`production.rs:49`, `:263-276`), so it is already
  ascending by producer id. One pass that merges site ids into that walk keeps the M2/M3 traces equal (no sites).

## 1. E-c: income and credits against the charges of tick t

- Rule: income of tick t is credited in `Income`, after the commands of tick t and before every per-tick charge
  of tick t (the pay pass in `Production`, repair in `Repair`).
- Position: `Income` stays the first set of `FixedUpdate` (`plugin.rs:120`).
- Consequence: a charge at the order (`ChargeOnQueue`, the reserve of `ReserveThenCharge`, `production.rs:130-138`)
  sees the balance before the income of tick t. A delivery in `Gather` (`gather.rs:947-948`, `:1098-1106`; set at
  `plugin.rs:188-193`) is also before the pay pass, so a starved job pays in the tick of the delivery.
- Design text: D is silent on income against commands (U7, D:1528). Row PAY1 (D:2451) says "resumes in the tick
  after the next delivery"; the frozen order gives the tick of the delivery. Erratum needed for PAY1 only.
- Erratum, paste into 6.4 "Order inside a tick" (D:644), after the sentence:
  "Inside a tick, credits come before the per-tick charges: the commands of tick t (a charge at the order), then
  the income of tick t, then the deliveries of tick t, then the pay pass. A unit destroyed in the combat of tick t
  has paid its income of tick t; it pays none from tick t+1."
- Erratum, row PAY1 (D:2451): old "the job resumes in the tick after the next delivery"; new "the job resumes in
  the tick of the next delivery".
- L0-SIM now: fix the comment `income.rs:3-4` (finding L3). Later: `L-INC` (the period), `L-PAY` (the PAY1 case).

## 2. E-d: a site completed in tick t

- Rule: a site completes in the pay pass of tick t (`Production`). From that point it is a complete unit: the
  sets after it in tick t (`Acquire`, `Repair`, `Combat`, `Outcome`) see it complete, so its weapon can acquire and
  fire in tick t and the `KeyUnitsLost` count changes in tick t (D:1120). Its supply counts from the computation
  of tick t+1, because the computation of tick t ran before it (D:520-523). Its producer, constructor and income
  act from tick t+1, because their sets ran before the pay pass in tick t. A unit spawned in tick t follows the
  same rule.
- Design text: no change. It follows from D:430-432 ("off until it is complete") and D:518-523. Optional
  clarification for 6.1 after D:432: "A site that completes in tick t is complete for the rest of tick t; its
  supply counts from tick t+1 (6.2)."
- L0-SIM now: the doc comment `construction.rs:4-6` is wrong (it says sites advance in `Construction`): sites
  advance in the pay pass. Later: `L-CON1` (completion in the pass), `L-PWR`, `L-MATCH-SIM`.

## 3. E-e: repair

- Rule: repair runs in a new set `Repair`, after `Acquire` and before `Combat`: heal before damage.
- Evidence: (a) auto-repair uses "the cadence and grid rule of 6.3" (D:708-709), and the grids are "built once for
  each tick" (D:593-595) in `Acquire`; repair before `Acquire` would need a second build. (b) The client "derives
  a hit from the health drop" (D:614-615). Heal after damage can cancel a hit inside one tick, so the published
  health shows no drop and the client shows no hit. Heal before damage keeps every hit visible. (c) The death path
  stays the last health writer of the tick, so no heal touches a removed unit.
- Effect to state: a heal in tick t comes before the damage of tick t, so a repaired unit can survive a hit that
  would kill it at its old health. Repairers pay after every job of the tick.
- Erratum, paste into 6.5 "Repair" after D:703: "Repair runs once for each tick after the acquisition grids and
  before combat: the heal of tick t comes before the damage of tick t. Repairers charge after the pay pass of the
  tick, in ascending repairer unit id."
- L0-SIM now: add `SimSet::Repair` with one empty system in a new file `economy/repair.rs`, in the pinned order and
  in the slot test. Later: `L-REP` fills it.

## 4. The spawn order of a unit spawned after Orders

- Rule: the engine applies the `spawn_order` (or the rally follow order) in the spawn step, in the pay pass of the
  spawn tick t, through `apply_order` with the source `Engine`. `NearestNode` is chosen in tick t. The order log
  line has tick t. The unit moves first in `Movement` of tick t+1.
- Evidence: D:731 and U8 (D:1529) "at the spawn tick"; STO1 run 3 (D:2469) "moves toward the nearest node in the
  tick after its spawn ... one `Engine` line for its spawn order at the spawn tick". `Movement` (`plugin.rs:122`)
  runs before `Production` (`:128`), so no write in tick t can move the unit in tick t.
- Design text: no change.
- L0-SIM now: nothing. Later: `L-STO`, `L-RLY`. NOT TRACED: that `apply_order` can run inside the
  `capture_operation` of `try_output` (`production.rs:367-371`); `L-STO` traces it.

## 5. The set list that L0-SIM freezes

    FixedPreUpdate  (chain): Advance, Supply, Construction, Commands
    FixedUpdate     (chain): Income, Orders, Movement, Gather, Extract, Transport, Inserters,
                             Production, Acquire, Repair, Combat, Outcome
    FixedPostUpdate:         Collect

- `Supply`: the one supply computation (H1).
- `Construction`: the placement of held `Construct` orders only (the plan names this slot "construction
  placement"). Before `Commands`, so a site that completes (pay pass, t), is destroyed (Combat, t) or is
  cancelled (Commands, t) frees its cap for the placement of tick t+1. That is the "next tick" of 6.1 (D:425-426)
  and of CON1 and CON3 (D:2447, D:2449). In `FixedUpdate`, a cancel in tick t would place in tick t: an erratum.
  A `Construct` order that the caps allow still places at once, in `Commands` (D:417).
- `Production`: the one pay pass. Today's research and machines first (`production.rs:259-262`, unchanged); then
  one walk over the owners (producers and open sites) in ascending unit id; for each owner: pay and advance the
  head job (a site: its one job; completion here), then the output and the spawn order. Construction progress and
  production progress are both here; there is no other progress set.
- `Acquire`, `Repair`, `Combat`, `Outcome` as above. `Outcome` after the death path, before `Collect` (D:1118).
- The old sets keep their relative order: Advance, Commands | Orders, Movement, Gather, Extract, Transport,
  Inserters, Production, Combat | Collect. The new sets are empty, so the M2/M3 traces stay byte-equal.

What L0-SIM changes now:
1. `plugin.rs:110-136`: the chains above; `Supply` and `Construction` move to `FixedPreUpdate`; add `Repair`.
2. `plugin.rs` tests: `TICK_ORDER` (17 sets) and the slot list of
   `each_slot_set_holds_a_system_in_the_fixed_update_schedule`: `Supply` and `Construction` are now in
   `FixedPreUpdate`, so the test must read that schedule for them (as written it reads `FixedUpdate` only,
   `plugin.rs:376`, and goes red). `mark` (`:342-355`) must put `Supply` and `Construction` in `FixedPreUpdate`.
3. Doc comments: `plugin.rs:32-41`, `income.rs:1-4`, `construction.rs:1-6`, `power.rs:19-21` (Supply runs before
   the commands), the test comment `plugin.rs:317-322`.
4. The server admission systems run `.after(SimSet::Advance).before(SimSet::Commands)` (`session.rs:120-121`,
   `:1123-1124`, `command_script.rs:45`). With two new sets in that gap, they are unordered with `Supply` and
   `Construction`. Change `.after(SimSet::Advance)` to `.after(SimSet::Construction)` so the order of
   `FixedPreUpdate` is total. No behaviour change today (the sets are empty).

Later tasks own: the pay pass with sites (`L-PAY` builds the owner walk; `L-CON1` adds sites, it already takes
`L-PAY` as input and edits `production.rs`), the held placement (`L-CON1`), supply (`L-PWR`), income (`L-INC`),
grids (`L-TGT`), repair (`L-REP`), the outcome (`L-MATCH-SIM`).

## NOT TRACED

- Whether `admit_player_commands` reads unit state that the held placement changes (item 5.4 removes the question).
- The income period start after a completion (`L-INC`).
- The errata above are not applied to D; the orchestrator applies them with revision 6.3.
