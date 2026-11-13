VERDICT=PASS HIGH=0 FINDINGS=6 RUNS=22 NOT_CHECKED=4

# Review of pre-MP1 slice task A1b (qa-lead, 2026-11-12)

Candidate: c536c8c (7425aa1, c536c8c on 708c335). Lane `/home/julien/Projects/TuRTS-pre-a1b-review`, cache
`target/mc-pre-a1b-review`. Evidence: `/home/julien/Projects/TuRTS/target/mc-evidence/a1b-rev-*`. Plant script:
`/tmp/claude-1000/-home-julien-Projects-TuRTS/bf418660-b272-4bb3-8628-8d9ef2677d1d/scratchpad/qa-a1b-review/plant.py`.

Lens: is the ledger audit a real instrument on its own? Each rule family of the builder has a red with a message
that names the fault (the builder's 40 plants and the 9 registry rows; I read the rows and the rule code). My
faults show three places where the ledger is blind, and where cover after A1c depends on other tests or on
nothing. None of them loses cover at the A1b commit, because the replay validator still runs. They are owed
before A1c deletes the replay.

## Findings

1. **Medium, needs decision.** `crates/turts_audit/src/lib.rs:401-436` (refusal rule) compares `LedgerAudit`,
   not the full logged view. `LedgerAudit` drops `pending_movement_orders` and `movement_dispatch`
   (`crates/turts_sim/src/economy/ledger_view.rs:38-46`); the sim calls the first one "audited command state"
   (`crates/turts_sim/src/orders/pending.rs:32`). Probe (F4, a temporary test in `ledger_logs.rs`, restored, cmp 0):
   the refused command (operation 5) of the base log, with `pending_movement_orders.next_pool_id + 1` in its
   `after`: `ledger=Ok(())`, `replay=Err("movement transition changed nonmovement state")`. A refused unit that
   still gets a pending spread entry moves later, and no ledger rule sees it. Design D section 4 item 4 says
   the rule covers "the full logged view"; TDI `:59` leaves these fields out of `LedgerAudit`. Options: (a) add
   `pending_movement_orders` to `LedgerAudit`; (b) accept, and pin a refused-group outcome test before A1c.
2. **Medium.** `crates/turts_checks/tests/movement_writers.rs:49-98,288-309`: the allow list is by whole file, so
   a new writer in an allowed file is invisible. It is not brittle (line numbers are only in the `why` text). F2:
   `world.get_mut::<crate::world::Position>(entity)` plus `x += 0.25` for a weapon with a target in
   `economy/combat.rs:203` (an allowed file). Scan: green. Ledger (replay off): green on the Forge log. Replay
   on: red, "operation before snapshot is discontinuous from the prior modeled state" (`ledger_logs.rs:197`), and
   `mc_orders.rs:791`, `:1071` red only with the replay. With the replay off, the only reds are incidental
   (`mc_orders.rs:934`, `:1185` position equality; `ledger_logs.rs:184` "no Fired event"). Fix: pin the multiset
   of (file, kind, type) with counts, for example `combat.rs: method remove MoveTarget x1`.
3. **Medium.** Three validator self-checks are red with the replay off and no plant (run
   `a1b-rev-baseline-off-server`): `crates/turts_server/src/economylog.rs:453` (Claim at slot 1 with slot 0
   open: the removed "first open slot" cover), `economylog.rs:331` (phase Other on a movement operation), and
   `crates/turts_server/tests/economy.rs:1145` (1 ulp). The A1c plan names only the 1-ulp test and the E8
   self-checks. `:331` was a Low of the S1 review; `:453` is named nowhere. Fix: name both in A1c. F1 shows that
   `session.rs:840`, `:1125` and `economy.rs:4596` hold the first-open-slot property, so `:453` can go.
4. **Low.** `ledger_view.rs:134-198`: the field-equality test uses one unit with default orders, so each field
   that is empty in that fixture is not checked. F5 (`unit_orders: Vec::new()`): the test stays green. The
   red comes only from `economy.rs:908` (the depletion rule reads `unit_orders`). At A1c, EL1 reads
   `unit_orders` from the log. Fix: compare on a Forge `Tick` state, and assert that each kept field is not empty.
5. **Low (by decision).** F3 (`production.rs:107` `cost.amount - 1`): the ledger is green. The charge and the
   charge line agree, and no rule compares a charge with the catalog cost. The refund, by contrast, is compared
   with the catalog ratio (`production.rs` of turts_audit, `:141-189`). Held by `economy.rs:4717`
   ("accepted Produce differs from exact requested recipe, ledger and unrelated-state model"). CEO rule:
   "ledger conservation only".
6. **Low.** Repeated validation: `economy.rs:2455-2470` validates the full prefix with both validators on each
   10 ms poll; `economy.rs:2502-2503` validates twice; `economy.rs:552-553` validates twice. See the time cost.

## My faults (replay off unless stated; each restored, cmp 0, `git status` empty)

| Fault | Ledger | Other red (replay off) | Replay on |
| --- | --- | --- | --- |
| F1 `slots.rs` `rposition` (last open slot) | green | `session.rs:840` `Ok(SlotId(1))` vs `Ok(SlotId(0))`; `:1125`; `economy.rs:4596` | not run |
| F2 Position write in `combat.rs` | green; scan green | incidental only (above) | red, 3 sites |
| F3 cost - 1 | green | `economy.rs:4717` | not run |
| F4 refused command changes pending orders (log probe) | green | none | red |
| F5 `LedgerAudit.unit_orders` empty | field test green | `economy.rs:908` | not run |

## Removed rules and cover

- "a started job has no active claim": dead code. `reservations.rs:83-91` returns "names no queued job" first.
- "no shot at a removed unit": a single-site plant cannot reach it. `tx.health` returns `IntegrityViolation`
  for a missing row (`crates/turts_sim/src/economy/transaction_m3.rs:773-781`), and `choose_target` needs a health
  row (`combat.rs:305-309`). With two sites (health `Ok(0)` and no second removal), the ledger accepts a shot with
  `applied 0` (`turts_audit/src/combat.rs:120-122`). NOT TRACED by a run. Low risk.
- "first open slot": not in the ledger; kept by outcome tests (F1). `economylog.rs:453` must move (finding 3).

## Plant (b), E8-mixed, time cost

- Plant (b): agreed that it cannot go red before A1c. The sink writes `AuditState` (`economylog.rs:33,92,124-125`).
  `facing` is inside `WorldView`, so the plant edits `AuditState` too. Move the plant to A1c.
- E8-mixed, release, replay off: clean green (4.47 s); plant red at `economy.rs:3808`, "refused unit 1 changed
  state inside its order operation" (the ledger text, `lib.rs:425`). The receipt does not record
  `TURTS_AUDIT_REPLAY` (`exec_in_lane_v4.py:44`); the baseline-off debug run proves that the variable reaches the
  test process.
- Time: Forge log only 25.3 s, log plus ledger 20.8 s, log plus both 22.1 s (debug). The ledger cost is below
  the noise. Building the 2,400-tick log in debug is the cost of `ledger_logs` (about 23-26 s). The economy
  binary took 26.8-47.1 s over six of my runs (builder base 28.2 s), so the +9 s is noise. E9 200 to 265 s:
  NOT TRACED.

## Gates

Fast gate clean: audit 69, sim 229 + tests, checks, server all green (`a1b-rev-final-gate`, exit 0). Traces and
stream: all five sha256 equal to the pinned values.

## Not checked

- ci-browser 8 cases: they need the headed wrapper (`parallel_refutations.py:18-19,274`). Owed.
- EL1 and E9 release with the replay off.
- The builder's 40 plants: not re-run; I read the 9 registry rows and `plants.tsv`.
- The two-site "shot at removed unit" plant.

## What I changed

This file only. No commit.
