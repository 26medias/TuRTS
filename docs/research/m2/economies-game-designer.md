# M2 two-economy design (game-designer, answer mode, 2026-10-26)

Spec = RTS-Engine-Project-Specifications.md (S). M1 = docs/superpowers/specs/2026-10-26-m1-foundation-design.md.
All numbers are TEST VALUES (proposals). 16.2 leaves statistics, timings, costs, repair
accounting, construction restrictions, power multiplier, simultaneous destruction OPEN (S:751).

## 0. Status facts used

- M2 acceptance: "Traditional global-balance economy and physical-item economy work without
  privileged default-game code" (S:677). Proposed milestone table (S:672).
- 16.2 roster and rules: Decided, default mod only (S:726-749, D44 S:67, D55 S:78).
- 4.3 split Economy: engine = storage, balances, reservations, transactions; mod = resource
  types, upkeep, population (S:157). Construction: engine = placement, footprints, progress,
  occupancy; mod = prerequisites, worker-built vs instant, costs (S:158). Decided (S:151).
- 7.1 three resource kinds (S:394-398). 7.2 three payment policies; cancel/refund/interrupt/
  blocked/destroy must be explicit (S:406). 15 Production API: "do not impose one universal
  policy" (S:707). 8: research policies open (S:428). Research is NOT in M2 loops.
- Code today: no economy code. grep for ore|credit|harvest|command_center|power_station|
  inventory|balance in crates/*.rs returned nothing. Mod API has UnitDef{id,name,speed,
  footprint}, MapDef{placements}, VisualDef only (crates/turts_mod_api/src/schema.rs:52-116).
  Only command: MoveTo (crates/turts_net_protocol/src/messages.rs:79).
- No-content test pattern names only base|acme|rifleman|mods/
  (crates/turts_checks/tests/no_content.rs:7).
- Scope conflict: M1 design assigns "economy and the 16.2 roster (M2)" but "combat ... (M7)"
  (M1:45-46). 16.2 victory, turret, tank, anti-vehicle, engineer all need combat.

## 1. Loop A: default mod `base`, global balance (16.2)

Resource kinds (7.1):
- ore: PHYSICAL ITEM (in deposits, in harvester cargo). 7.1 lists ore as item (S:397).
- credits: ABSTRACT BALANCE, per slot. Team sharing NOT decided -> mod policy, M2 uses per slot.
- electricity: NETWORK QUANTITY. M2 proposal: one implicit network per slot, aggregate
  supply vs demand (11.3 "aggregate supply and demand per connected network", S:521).
  needs decision: per-slot aggregate at M2, or wait for M6 topology.

Content (test values):
| Entity | Role | Cost | Time | Power | Other |
| --- | --- | --- | --- | --- | --- |
| command_center | producer + construction manager + ore drop-off | start asset | - | demand 10 | converts 1 ore -> 1 credit on delivery |
| power_station | building | 300 cr | 10 s | supply 100 | footprint 3x3 |
| harvester | gatherer | 600 cr | 15 s | - | cargo 100 ore, gather 20 ore/s, unload 50 ore/s, speed 2.0 |
| rifleman | infantry | 100 cr | 5 s | - | combat stats only if combat in M2 |
| anti_vehicle | infantry | 200 cr | 8 s | - | combat only |
| engineer | infantry | 300 cr | 10 s | - | repair only (needs damage) |
| tank | vehicle | 600 cr | 15 s | - | combat only |
| turret | building | 400 cr | 15 s | demand 25 | disabled when supply < demand |
| ore deposit | world | - | - | - | 2 per side x 1000 ore, 16 wu from CC, mirrored |
Map: 64 x 64 (as M1 `base`), 2 slots, mirrored.
Start (per slot): 1 CC, 1 harvester, 500 credits (power station 300 = "enough to establish power").

Rules and payment (7.2), proposals:
- Production at CC: CHARGE WHEN QUEUED. Cancel: 100% refund. Queue size 5. Blocked output
  (no free spawn cell): finished unit waits at queue head, queue stops. CC destroyed: queue
  lost, no refund. Low power: progress rate x0.5 (open multiplier, S:751).
- Construction: CHARGE WHEN ORDERED (16.2 "pay credits, select a valid site, then progresses",
  S:745). Before placement: cancel = 100% refund. After placement: foundation entity, progress
  over time; cancel = 50% refund (proposal); destroyed foundation = no refund. Valid site:
  in map, no overlap, within 20 wu of an own CC or completed building (MOD rule, not engine; D28).
  Construction counts as CC production, so low power slows it too (proposal).
- Harvesting: harvester auto-returns to nearest own drop-off; on delivery, atomic: remove ore
  from cargo, add credits. Harvester destroyed: cargo lost. Deposit depleted: stays depleted
  (invariant 3 spirit, S:541).
- Electricity: power state = supply >= demand per slot. Evaluated each tick or on change.
- Engineer: repairs 10 HP/s, costs 1 credit per 2 HP (CONSUME OVER TIME). Repair accounting open.
- Victory: lose when owning zero command_center; last slot standing wins; simultaneous last
  losses in one tick = draw (proposal; open S:751).

Walk (arrow -> capability or GAP):
1. harvester -> deposit: Gather order + gatherer behaviour + deposit extraction. GAP (only MoveTo).
2. ore -> CC: drop-off receiver + inventory transfer. GAP.
3. ore -> credits: on-delivery conversion transaction (item out, balance in). GAP.
4. credits -> units: production queue + cost from balance + spawn output. GAP.
5. credits -> buildings: construction order + placement validation + foundation progress. GAP.
6. power -> production/turrets: network quantity aggregate + powered-state modifier hook. GAP.
7. army -> combat: health, weapons, attack/attack-move/auto-target, damage, destroy. GAP (M7?).
8. combat -> victory: entity-destroyed event + ownership query + match-end. GAP.
First minute (arithmetic, test values): t0 pay for power station (500 -> 200) and place it;
progress 10 s base, x0.5 under low power -> done ~t20. Harvester trip = 8 s out + 5 s fill +
8 s back + 2 s unload = 23 s -> 100 cr per trip (~4.3 cr/s); deliveries at t23, t46 ->
400 credits at t60 if the harvester gathers from t0.
First choice (~t60-90): second harvester (600, payback ~140 s) or army. First loss: none
without combat. Dead ends without combat: engineer, tank, anti_vehicle, turret, rifleman
have no reason to exist; victory never resolves. Ore finite -> after depletion, credits have
no source; stalemate if no army (no deadline: D28 forbids engine deadline; mod time limit = open).
Sinks: credits -> units, buildings, repair. Ore -> credits. Electricity: constraint, no sink.

## 2. Loop B: alternative mod (say `haul`), physical items, no balance, no power

Resource kinds: ore = PHYSICAL ITEM only. No balance, no network. Proves power optional (S:420).
Content (test values):
| Entity | Role | Cost | Time | Other |
| --- | --- | --- | --- | --- |
| depot | start building, storage 2000 ore, producer of workers | start asset | - | drop-off |
| worker | gatherer + hauler | 50 ore from depot inventory | 6 s | cargo 10, gather 5 ore/s, speed 3.0 |
| stockpile | forward drop-off, storage 300 | 100 ore delivered to site | progress = delivered/required | |
| monument | goal building | 1500 ore delivered to site | progress = delivered/required | |
| ore deposit | world | - | - | 3 per side x 1500, 12 wu and 24 wu from depot |
Map 48 x 48 (as M1 `acme`), 2 slots, mirrored. Start: 1 depot holding 100 ore, 2 workers.
Rules, proposals:
- Worker production: RESERVE at queue (from depot inventory), CHARGE at start. Cancel before
  start: release; after start: 100% refund into depot; depot full: refund blocked -> cancel
  rejected (explicit). Depot inventory is the ONLY source: ore in a stockpile cannot pay.
- Construction: CONSUME AS DELIVERED. Site holds an inventory that accepts only its material;
  progress = delivered/required; no delivery = no progress. Cancel: delivered ore stays at
  the site as a pickup pile (proposal).
- Hauling: Supply order: worker takes from a storage (reserved) and carries to a site, repeats.
- Victory: first slot with a completed monument wins. Same tick both: draw (proposal).
Walk: worker -> deposit (Gather, shared with A); ore -> depot/stockpile (drop-off + transfer,
shared); stockpile -> site (take with reservation + carry + deliver; GAP: Supply order and
reservation); depot ore -> worker (production queue with cost source = producer inventory);
site inventory -> progress (construction progress source = delivered fraction); monument ->
victory (completion event + match-end).
First minute: worker trip 8 s travel + 2 s fill = 10 s -> ~1 ore/s per worker; 2 workers ->
~120 ore at t60. First choice t0: spend 100 ore on 2 more workers or save for a stockpile.
First loss: none (no combat); contention loss = a worker that finds a reserved pile empty.
Sinks: ore -> workers, stockpile, monument.
Reservation test (invariant 5, S:543): stockpile holds 10 ore; two workers with cargo 10
both ordered to take: exactly one receives 10, the other gets a rejection or zero.

## 3. Rule ownership (4.3)

| Rule | A | B | Owner | Shared engine capability |
| --- | --- | --- | --- | --- |
| resource ids, kinds | ore item, credits balance, power network | ore item | mod | resource registry with kind |
| gather loop | harvester | worker | mod data, engine behaviour | Gatherer + Deposit extraction |
| delivery destination | CC | depot, stockpile | mod | DropOff receiver |
| delivery effect | convert to credits | store | mod | atomic transfer; optional conversion |
| unit cost source | slot balance | producer inventory | mod | Cost{resource, amount, source} |
| payment policy | charge on queue | reserve then charge | mod | queue with policy enum + refund fraction |
| construction | prepay, timed | delivered material | mod | foundation with progress source |
| site validity | radius 20 | free footprint | engine: footprint/occupancy; mod: radius | placement check + mod restriction data |
| power effect | x0.5 rate, turret off | none | mod | network aggregate + powered state + modifier |
| victory | last CC | first monument | mod | match end + events + queries |
| starting assets | CC, harvester, 500 cr | depot+100 ore, 2 workers | mod (D28) | map placements with initial balance/contents |
| no double spend | yes | yes | engine (S:404, S:543) | transactions + reservations |

## 4. Engine capabilities M2 adds (mechanisms, no values)

1. Resource definitions with kind item|balance|network (definitions layer).
2. Per-slot balance accounts: credit/debit transactions, non-negative unless mod allows.
3. Inventory component: capacity, item filter, owner access; atomic transfer; reservations.
4. Deposit component: finite amount, extraction transaction, never refills.
5. Gatherer behaviour (composition): gather -> carry -> return to nearest accepting drop-off.
6. DropOff receiver with optional conversion (items -> balance at a mod rate).
7. Production queue: Cost{resource, amount, source: balance|own inventory}, policy
   ChargeOnQueue|ReserveThenCharge|ConsumeOverTime, refund fractions, blocked-output hold,
   rate modifier input, spawn output. Events: started, completed, cancelled.
8. Construction: place order, footprint/occupancy check, mod restriction hook (range data),
   foundation entity, progress source time|delivered-inventory, completion event.
9. Network quantity, per-slot aggregate (pending decision vs M6): supply, demand, satisfied flag;
   modifiers "rate x k" and "disable capability" when unsatisfied.
10. Orders + validation: Gather, Supply (take-and-deliver), Produce, CancelProduction, Place,
    CancelConstruction. CommandRejected reasons (exists in M1 protocol).
11. Replication: own balance, own inventories, queue and progress, power state; secret to owner.
12. Match end: Victory/Defeat/Draw outcome; entity-destroyed and completed events; ownership
    count query.
13. Package schema: buildings, resources, production entries, initial balance and contents on
    placements. Extend no-content pattern to new content names.
14. If combat is in M2: Health, Weapon, Attack/AttackMove/auto-target, damage, destroy.

## 5. Declarative vs script (5.1)

Definitions: all costs, times, capacities, rates, conversion, supply/demand, deposits,
starting assets, maps.
Composition: gather, drop-off(+conversion), production queue, construction(time|delivery),
power consumer/producer, unpowered modifiers, repair (if repair costs a balance per HP).
Script candidates (event-driven, low rate): victory "last owner of tag" and "first completed X"
unless the engine has built-in condition types; simultaneous-loss draw; repair accounting if
not composition. Event rate (arithmetic, test values): deliveries ~1 per 23 s per harvester;
10 harvesters ~0.43 events/s. No per-unit per-tick script is needed (S:183).
Conclusion: both loops CAN be fully declarative + composition. Then M2 proves layer 3 only if
a rule is deliberately written as a script. needs decision.

## 6. Player view: unclear or unfair

A: low power slows production with no visible cause (needs supply/demand display); prepaid
building with no valid site locks credits (needs cancel + rejection reason); harvester cargo
lost on death is invisible; credits locked by queued items; without combat, 5 of 9 roster
rows are dead and the match never ends; ore depletion -> stalemate with no deadline.
B: "I have 300 ore" but in the wrong inventory -> cannot build (needs per-inventory display
and where-payable hint); stalled site with no supplier looks like a bug; reservation
contention looks like a bug; race with no interaction = parallel solitaire, fairness only
from map symmetry.
