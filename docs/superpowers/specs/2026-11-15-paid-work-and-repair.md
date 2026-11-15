# Paid work and repair

Status: Anvil spec and plan reviews approved. Both final clarifications are incorporated.
Baseline: operating resources at `4eb91f1c`; MP1 design sections 6.4, 6.5, and public schema rows P1, K5, O2, R1.
Public contract: engine API `0.2.14`, network protocol `8`. Older protocol peers fail compatibility before state admission.

## Result and scope

Mods can charge while training units or constructing buildings, cancel construction safely, and define repairers.
A player can observe payment stalls, cancel an unfinished building, and order a repair through ordinary controls.
The existing Skirmish economy and balance stay unchanged. A separate mod fixture proves the new behavior.

Research ChargeOverTime remains refused, as the existing public design requires.
Research still gains safe destruction refunds for its existing payment policies.
Machine recipe-input payment remains unchanged. This feature does not add another payment model for machines.
Individual cancellation of one held construction request is deferred until that queue has stable public addresses.
Stop cancels all held requests now. No command guesses a held request from a shifting row index.
Repair of unfinished sites or zero-health units is deferred. No resurrection is introduced.
New artwork, repair animations, surrender, and match statistics are outside this feature.
These deferrals remain unfinished engine scope. This feature does not complete the engine.

## Payment while working

ChargeOverTime applies to unit production and construction productions only.
Accepting a job, placing a valid site, or holding a construction request charges nothing under this policy.
Existing placement validation, site limits, production queues, and authorization still apply.
For each authored cost entry, define `paid(done) = floor(cost * done / total)` using exact wide integer arithmetic.
An advancing tick charges `paid(next_done) - paid(done)`. `next_done` cannot exceed `total`.
All cost differences and progress commit together, or none commit.
Retain a site's original constructor-inventory endpoint after constructor removal. Never substitute another source.
An absent payment source holds the site without mutation. A manual refund to an absent destination is refused atomically.
Zero-difference ticks can advance without funds. No balance or inventory becomes negative.
Insufficient funds leave progress, health, charged amounts, reservations, and allocations unchanged.
The per-tick state identifies the first unaffordable resource in authored cost order.
Work retries automatically. Manual Disable pauses progress and payment without losing fractions.
Output that waits for a legal exit is fully paid once. Retrying its exit does not charge again.

Keep the installed global RateQ and the exact minimum shortage rate for each capability.
Site health gains use the same scaled progress difference as construction today.
The final progress step, its charge, health gain, and operating-resource activation are one atomic operation.
Admission overflow freezes that final step before payment, using the existing typed operating hold.
Normal geometric output blocking retains its existing behavior.
Commands still spend before Income. Income still runs before work payment. Repair runs after work payment and before combat.
Producer and site payment competition uses the existing shared physical-source order and queue FIFO.
Exact source ties receive equal priority and are held together. Unit IDs never choose a visible winner.
This replaces the older MP1 instruction to pay in unit-ID order.

Preserve the public hold hysteresis: first funding failure sets `held_since_tick` and the display hold flag.
Clear that flag only after one full second of consecutive affordable advancing ticks, rounded up to server ticks.
A failed tick resets the recovery streak. Paused ticks do not count toward recovery.
The display hold flag is not a progress veto. Affordable ticks advance while that flag remains set.
Per-tick WaitingForFunds and its zero rate remain distinct from the display hold flag.
Client prediction follows actual state and per_tick, including advancing Running/Slowed records with held=true.
Change the old replica_store interpretation that unconditionally stalls when held=true.
Coalesce accounting events at completion, cancellation, destruction, and entry into a funding wait.
Raw authoritative charged amounts must still explain every balance change between those events.

## Cancellation and removal

CancelProduction retains its existing explicit job identity and ownership checks.
CancelConstruction with target None cancels each selected own site. A completed building is refused.
Grouped site cancellation evaluates eligible sites by physical source coordinates, with an atomic refund/removal for each site.
Exact source ties are all refused as Blocked. Other per-unit outcomes remain independent; selection/ID order never chooses a refund winner.
Stop on a constructor clears all of its unplaced held Construct requests and refunds zero for them.
Its already placed sites continue independently. Stop does not cancel the producer queue of a hybrid constructor.
Explicit site cancellation removes its occupancy, work state, operating holds, and claims in the same transaction.
An already cancelled site or job cannot refund twice. A repeated request returns the ordinary refusal.
Cancellation remains available while a source is manually disabled or cannot pay its next work tick.

Refund each cost from the amount actually charged, using the authored stage ratio and one final floor.
A queued ChargeOverTime job refunds zero. A queued reservation is released without manufacturing a credit.
A queued ChargeOnQueue job retains its existing queued-refund policy because it already paid.
A running site uses `cancel_refunds.running`; a blocked completed output uses `output_blocked`.
Return a manual refund to its original source. If any refund cannot fit, refuse the whole cancellation without mutation.

Destruction uses the common removal transaction, including all queued jobs, sites, reservations, and held requests.
It applies `cancel_refunds.destroyed`, releases claims once, and removes occupancy in the same tick.
Remove by command still requires permission; permitted removal of a queue-bearing unit follows this destruction policy.
Lethal damage does not depend on permission to remove by command. Health without an optional removal policy is not immunity.
Destroying a constructor drops its held requests. Its placed sites retain their original rules and continue.

A refund failure must never cancel lethal damage or preserve its victim.
Aggregate destruction refunds per original destination before crediting them.
For a surviving balance, credit only the representable amount. Explicitly record the uncredited remainder as discarded.
For a surviving inventory, credit its complete refund batch only if all of that batch fits; otherwise discard that batch.
Refunds destined for an inventory destroyed in the same operation, or an absent destination, are discarded explicitly.
Accounting records distinguish desired, credited, and discarded amounts. No silent saturation or duplicate credit is permitted.
Discarding a refund does not create an item drop or change the existing contents-destruction policy.

## Repair

Resolve the existing RepairerDef through the public catalog. It remains independent of Bevy and game-specific unit names.
The resolved field is `ResolvedUnit.repairer: Option<RepairerDef>`, with validated qualified cost-resource and class references.
Repair is not a queue-owning work capability. A unit can have a weapon and a repairer.
The player owns the repairer. The target must satisfy its authored slot relation and optional armour-class filter.
An empty class list accepts otherwise eligible unarmoured targets too. OwnDamaged smart targeting remains own-slot only.
Targets must exist, be complete, have positive health, and be below their authored maximum health.
An explicit Repair order uses an ordinary Unit target. It approaches through the existing movement model.
Reach is the gap between the two gameplay footprints, rather than distance from the repairer's centre.
This clarifies the old target-edge wording and permits short reach without requiring physical overlap.
Approach obeys normal collision and unreachable handling. It introduces no separate movement or teleport path.

For each eligible affordable tick, heal the exact difference of `floor(rate_per_second * t / tick_hz)`.
The applied amount is capped by missing health. Approach and unaffordable ticks do not advance this repair clock.
An eligible zero-heal fractional tick advances the clock without a debit, so rates below tick_hz eventually heal.
Charge only applied healing. Every repair cost and the health gain commit together, or none commit.
Repair costs debit only the repairer's own slot Balance accounts. The existing schema has no alternative cost source.
For each repairer/resource, maintain the exact fractional charge remainder from its `amount / per_health` ratio.
Retain that remainder across Stop, order replacement, target changes, and automatic retargeting.
Otherwise repeatedly replacing an order could obtain free healing below the rounding boundary.
Removal discards the repairer's remainder. Counter or charge overflow holds the action without mutation; never wrap or silently clamp it.
Free repair through an empty cost list or zero amount remains valid. Duplicate repair resource rows are refused.
Full health completes the explicit order. Target loss or invalidation uses normal order failure and queue handling.
Unused healing is not banked or transferred to another target in the same tick.

An idle repairer with auto_range selects the nearest eligible damaged target within that footprint-gap radius.
Choose by gap, then target coordinates. Never choose by unit ID.
An exact tie for the best remaining rank acquires no target, symmetrically. Do not fall back to a farther target or iterator order.
Retain the automatic target until full, invalid, removed, or outside auto_range. Acquire another on a later tick.
Use bounded spatial queries through the existing acquisition facilities.
An accepted explicit Stop immediately ends repair and latches automatic-repair suppression until the next accepted non-Stop explicit order.
Rejected orders and engine-generated orders do not clear this latch. Keep it in authoritative audit state, omitted when false.
Accepted queued non-Stop orders clear the latch, but pending/current/queued explicit work suppresses automatic repair until it ends.
Hold suppresses automatic repair while current; it does not complete by itself. An appended Hold starts when earlier work ends.
A completed explicit Move can return to automatic repair. Stop remains immediate rather than waiting behind a queue.
An active repair action suppresses that repairer's automatic weapon action. Explicit replacement restores normal combat behavior.
Repair is unaffected by power. Multiple repairers act independently in physical-source order after the work-payment pass.
Each sees the already committed health, so health cannot exceed its cap or charge for healing another repairer already supplied.
An exact source tie is held without an ID fallback.

## Owner state and compatibility

Publish exact scaled job/site counters, actual charged amounts, per-tick funding state, and stable hold timestamps to their owner.
Do not expose another slot's balances, costs paid, repair charge remainders, or funding state.
Arithmetic or exact-source-tie holds keep an in-range action in Working status.
WaitingForFunds names only a real unaffordable Balance resource.
Typed authoritative diagnostics explain arithmetic/source holds; no owner-visible wire reason is promised.
Add `RepairRecord { repairer: UnitId, target: UnitId, state: RepairState }` in the existing transport-free record module.
Freeze `RepairState::{Approaching, Working, WaitingForFunds(ContentRef)}`; the payload is the missing Balance resource.
Add `SnapshotRecords.repairs: Vec<RepairRecord>` as the complete owner-only set keyed by repairer.
Add `DeltaRecords.repairs: Vec<RepairRecord>` as absolute replacements and `repairs_cleared: Vec<UnitId>` as explicit removals.
Source removal uses WorldDelta.removed alone; omit repair rows and clear IDs for that removed source.
For target removal with a surviving repairer, clear its record in the same delta. No repair state row names a missing source or target.
Snapshots and new matches replace the store. Reject duplicate keys, missing units, and foreign-source repair rows atomically.
Completion, Stop, target loss, and source removal clear status without stale records.
Add PrimaryField::{RepairState, RepairTargetName, RepairResourceName} for Value::SelectionPrimary.
RepairState returns the stable key approaching, working, or waiting_for_funds; target/resource accessors return authored names.
They return null without an owned primary repair record; RepairResourceName is null outside WaitingForFunds.
Add Predicate::RepairStateIs(RepairStateKind), with RepairStateKind::{Approaching, Working, WaitingForFunds}.
PrimaryHas(Repairer) and PrimaryHasValue work normally. Labels and layout remain mod data.
Enemy viewers retain only their existing permitted unit and health information.
Use the existing order envelope. Add the new records at protocol 8 and the authoring accessors at API 0.2.14.
Packages without these capabilities retain their existing behavior and serialized audit shape.
Keep independent audit validation of actual debits, refunds, discarded refunds, progress, and applied healing.

## Acceptance

| Row | Required executed result |
| --- | --- |
| PW1 | A separate package authors both payment modes and repair through the public API. Wrong kinds, duplicate repair costs, invalid ratios/reach, and research ChargeOverTime fail with field paths. |
| PW2 | Production and site payment use exact cumulative floors at two tick rates and Q=1/Q=6, including zero-difference ticks and mixed balance/inventory costs. The charged sum equals the declared cost. |
| PW3 | One unaffordable or missing source freezes every debit and progress/health change. Income restores affordable work automatically. Retained inventory endpoints, manual pause, recovery hysteresis, and fractional progress remain exact. |
| PW4 | Final-step operating overflow takes no payment or health gain. Recovery succeeds once. Geometry-blocked output remains paid once. Swapping creation IDs preserves physical spending priority and FIFO. |
| PW5 | Own-site cancellation, repeated/disabled cancellation, queued reservations, ChargeOnQueue refunds, and absent refund destinations are exact. Grouped refund competition survives ID inversion and symmetric ties. Stop clears held requests; placed sites and hybrid production continue. |
| PW6 | Destruction of a producer, research station, site, and constructor releases claims and applies actual-paid refunds once. Full accounts, full/removed inventories, and health without removal permission never prevent lethal destruction. Discards are independently accounted for. |
| RP1 | Manual repair approaches a moving target without overlap, respects footprint reach, filters, and ownership, and refuses unfinished, full, missing, or zero-health targets correctly. |
| RP2 | Two tick rates, a heal rate below tick_hz, and fractional cost ratios produce exact health and debits. Wrong-kind repair costs fail. Unaffordable multi-cost repair heals nothing; free repair works. Order replacement cannot reset cost debt. |
| RP3 | Two repairers share a target without overheal or overcharge. Physical competition remains the same after ID inversion. Target removal, completion, and explicit replacement clear their state. |
| RP4 | Automatic repair obeys filters, exact symmetric target ties, and the persistent Stop latch. Rejected orders preserve the latch; accepted Move/queued work and Hold follow their defined lifetimes. Combat switching and power-independent repair work. |
| IO1 | Protocol 8 postcard round-trips and export/replica tests prove exact fractions, active held=true prediction, repair replacements/clears/resync, malformed-row rejection, and owner privacy. Old packages retain frozen audit streams. |
| IO2 | An isolated browser fixture uses real mouse input to place, stall, resume, and cancel a paid site, clear held construction with Stop, train a paid unit, and approach/repair a damaged target. Its resource, health, and funding displays agree with authoritative records. |
| IO3 | Repeated server runs agree at every tick. One independent review runs the completed foundation change. Required integration checks and the three final cold-play probes pass. |

Record evidence paths, exact verified limits, API/protocol changes, and the decisions above in one feature report.
Update README progress and remaining engine work. Keep the accepted launcher until final acceptance passes.
Every test Chrome uses a fresh profile and `--alsa-output-device=null`. Do not push main.
