VERDICT=fail ITEMS=112 MISSING=46 PARTIAL=51 CONFLICT=3 RUNS=7 NOT_TRACED=3

# Gap audit: simulation-core-engineer (spec 6.1, 7, 8, 13.5; second on 4.1, 4.2, 9 to 11)

Scope: yardstick A areas OR (orders), unit state machine, health and death, abilities, veterancy,
CO (construction), PR (production and technology), EC (economy), transports and garrisons, event
hooks. Yardstick B items in these areas are named in the note column of the matching A row
(B-n = `inventory-player.md` Part A2/A2b item n). Own gaps are `simcore-N`.

Milestone labels in the "proposed" column:

- `CTL` = a new core-control milestone, before M4 closes. Reason: M4 adds group `MoveTo` and a
  new earthwork order store (M4D:27 K1, M4D:146). An order model that comes after M4 must re-cut M4.
- `PLAY` = a milestone for the 16.2 first playable mod. Section 14 has none (S:674-688). S:694
  lists "playable default mod against a conventional bot" only as "additional proposed validation".
- M5, M6, M7, M12: the existing milestones (S:680-687).

Code paths are under `crates/`. M2D, M3D, M4D are the milestone designs under
`docs/superpowers/specs/` (2026-10-26-m2, 2026-10-27-m3, 2026-10-28-m4).

| id | item | verdict | evidence (S:line or terms searched) | mechanism/policy | milestone now | milestone proposed | depends on | M1-M3 change? | note |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| A-OR01 | Move | partial | S:747 (Decided, first mod), S:160 "Orders" engine cell; no design. Code `turts_sim/src/command.rs:163-164,212-253` | M | M1 (built) | CTL | OR11 | yes: becomes one order kind; inline gather reset `command.rs:241-251` moves into the order layer | B-6 |
| A-OR02 | Stop | missing | "stop" 0 hits (`simcore/spec-sweep.txt`). Code: `gather.rs:180` `interrupt_order` exists, no `CommandKind` for it (`command.rs:30-55`) | M | none | CTL | OR11 | no (new) | B-9 |
| A-OR03 | Attack target | partial | S:747 "attack" (Decided, first mod only); S:159 "Target queries". Code: no attack order; weapon picks nearest in range each tick `economy/combat.rs:165-199` | M | none | CTL | OR11, CB02, CB03 | yes: `choose_target` must read an order target before auto-target | B-5 |
| A-OR04 | Attack-move | partial | S:747 (Decided, first mod). No mechanism text | M | none | CTL | OR01, OR03, CB03 | yes: combat must know the unit's order mode | B-8 |
| A-OR05 | Force move | missing | "force" hits S:60,151,368,551,568,608,782: none an order | M | none | CTL | OR11, OR26 | no | armed mobile units fire while moving with no rule (`combat.rs:74-118` ignores `MoveTarget`) |
| A-OR06 | Force attack (ground, ally) | missing | "force", "attack ground" 0. Code: `SlotRelation::Any` is data, not an order (`turts_mod_api/src/schema.rs:237-241`) | B | none | CTL | OR03, target kind "point" | no | B-74 |
| A-OR07 | Patrol | missing | "patrol" 0 | M | none | CTL | OR04, OR11 | no | B-39 |
| A-OR08 | Hold position | missing | "hold" 0 real hits (S hits are "threshold") | M | none | CTL | OR26, CB03 | no | B-9 |
| A-OR09 | Stances | missing | "stance" 9 hits, all "instance" | B | none | CTL | OR26, CB05 | no | B-41 |
| A-OR10 | Guard and follow | missing | "guard", "follow" 8 hits, all "following"/"follow-on" | M | none | CTL | OR03, unit target kind | no | |
| A-OR11 | Order queue per unit | missing | "order queue", "queued order" 0. Nearest: S:342 "persistent intentions" (Proposed, cost paragraph). Code: one `MoveTarget` component `world.rs:31-33`; no queue | M | none | CTL | none | yes: order state is split over `MoveTarget`, `EconomyOrders` resource (`lib.rs:51`), `ProductionQueues`; M4 adds an earthwork store (M4D:146) | root gap; see lens answer |
| A-OR12 | Shift-queued orders | missing | "shift" 0; "queue" hits S:142,342,406,410,630,653 are production, commands, paths | M | none | CTL | OR11 | no | B-10 |
| A-OR13 | Waypoint display | partial | S:251 "unit orders and outcomes" must replicate. Wire: `UnitSnapshot`/`UnitDelta` carry position only (`turts_net_protocol/src/messages.rs:36-47`) | M | none | CTL | OR11, simcore-8 | yes: protocol carries no order state | presentation draws; sim must publish the queue |
| A-OR14 | Edit queued orders | missing | "waypoint" 0 | M | none | later (OPTIONAL) | OR13 | no | |
| A-OR15 | Rally point | missing | "rally" 1 hit S:366 ("literally") | B | none | CTL | OR11, PR05 | yes: `ProducerDef` has only `spawn_offsets` (`schema.rs:167-171`) | B-20 |
| A-OR16 | Build order | partial | S:140 Worker "construction ... orders"; S:745 (Decided, first mod). Excluded M2D:46, M3D:28 (K2), M4D:21 | B | none | PLAY | CO03, CO04, OR11 | no (new) | deferred three milestones in a row |
| A-OR17 | Gather order | present | S:140, S:743; S:157. Code `economy/gather.rs:88-178`, stages `gather.rs:33-40` | B | M2 (built) | CTL (re-home) | EC02 | yes: one unit per command (`messages.rs:59-62`); state lives in a resource, not in an order | B-33 |
| A-OR18 | Repair order | partial | S:742 (Decided, first mod only). M2D:46 excludes repair | B | none | PLAY | CO11, OR11 | no | B-51 |
| A-OR19 | Load and unload | partial | S:180 "transporting passengers" (composition example only) | M | none | PLAY or later | UN05, OR11 | no | B-53 |
| A-OR20 | Ability order | partial | S:160 "Orders, targeting, timers, effects" (engine cell) | B | none | after CTL | OR11, UN07 | no | B-43 |
| A-OR21 | Autocast toggle | missing | "autocast" 0 | B | none | later | OR20 | no | B-43 |
| A-OR22 | Order validation feedback | partial | S:782 "rejected-command reasons" (Proposed, bots); S:243 ack. Code has stable reasons `command.rs:78-152`, wire `messages.rs:92-119` | B | M1-M3 (built) | CTL | OR24 | yes: one reason per command; a group needs per-unit reasons | B-17 |
| A-OR23 | Order acknowledgement | partial | S:243 "immediate input acknowledgement" (Baseline). Code `CommandAck` `messages.rs:80-83` | B | M1 (built) | CTL | OR22 | no | presentation owns marker and voice |
| A-OR24 | Group order fan-out | conflict | S:342 "Support group orders" (Proposed). Code is all-or-nothing: one foreign or static unit rejects the whole list (`command.rs:274-283`); test `mixed_ownership_in_one_command_is_not_owner_and_moves_nothing` passes (run 2) | M | M1 (all-or-nothing) | CTL | OR11, OR22 | yes: per-unit acceptance; M4 K1 group `MoveTo` inherits the M1 rule | yardstick: "each unit that cannot do it ignores it"; B-6 |
| A-OR25 | Group destination spread | partial | S:156 "formation primitives"; S:630 shared destinations. Code: every unit gets the same `MoveTarget` (`command.rs:237-240`) | M | none (M7 flow fields) | CTL | OR24, GM01 | yes | group-movement seat owns; listed as a dependency |
| A-OR26 | Order state machine (interruption) | missing | "interrupt" S:72,406,678 (AI, production, M3); "state machine" S:759,771 (AI only). Code: interruption is pairwise inline code: `command.rs:241-251`, `gather.rs:162`, `gather.rs:188`; M4D:146 adds pairs for earthwork | M | none | CTL | OR11, CB03, CB05 | yes: replace the pairwise rules | see lens answer |
| A-OR27 | Deploy and transform | partial | S:160 "transformations" (mod-policy cell) | B | none | later | OR20, simcore-4 | no | UN18 |
| A-OR28 | Scatter | missing | "scatter" 0 | M | none | later (OPTIONAL) | GM04 | no | B-75 |
| A-OR29 | Assist | missing | "assist" 0 | M | none | later (OPTIONAL) | OR10 | no | B-56 |
| A-OR30 | Repeat queue | missing | S:428 "repeatable technologies" is research and Open; no queue repeat | M | none | later (OPTIONAL) | OR11, PR01 | no | B-56 |
| A-OR31 | Command rate limit | partial | S:342 "budgets for commands per tick" (Proposed). Code: only `MAX_UNITS_PER_COMMAND = 256` (`command.rs:19`); no per-tick cap in `turts_server/src/` (run 5) | M | none | CTL (cap), M12 (values) | OR22, simcore-2 | yes: per-command cost is O(N) (run 4) | D29 high AI APM |
| A-CB01 | Health and death | partial | "health" 0 hits; S:159 "damage events". Code M3: `Healths` resource `combat.rs:42-44`, `HealthDef { max }` `schema.rs:231-233`, kill = immediate `remove_unit` `combat.rs:260-262` | B | M3 (built) | PLAY | none | yes: see simcore-3 (a unit with a queue cannot have health) | B-28 |
| A-UN13 | Death and removal | missing | "death", "kill" 0; "destroy" 1 hit S:774 (AI). S:751 names "simultaneous-destruction edge cases" as open | B | M3 (instant removal) | PLAY | CB01 | yes: no dying state; `Destroyed` event only (M3D:363) | |
| A-CO20 | Destruction clears footprint | partial | S:158 "occupancy" (engine cell). Code: units are points, no occupancy grid (M4D:36 K10) | B | none | PLAY | CO03 | no | |
| A-CB18 | Wrecks and corpses | missing | "wreck", "corpse", "rubble", "husk" 0 | B | none | later | UN13 | no | B-57 reclaim |
| A-CB24 | Kill credit | partial | spec 0 ("kill" 0). Code records `RemoveCause::Killed { by: weapon }` `combat.rs:261` | M | M3 (built) | after PLAY | UN09 | no | code is ahead of spec |
| A-UN17 | Self-repair and regeneration | missing | "regenerat" hits S:215, S:480 are geology/maps | B | none | later | CB01, simcore-6 | no | |
| A-UN09 | Veterancy | partial | S:180 "gaining experience" (composition example only) | B | none | later | CB24, PR15 | no | B-50 |
| A-UN07 | Abilities | partial | S:160 engine "Orders, targeting, timers, effects"; mod "Spells, mana, cooldowns, buffs" | B | none | after CTL | OR20, simcore-6 | no | B-43 |
| A-MD04 | Ability definition | partial | S:179 definitions list has no abilities; S:160 only | B | none | after CTL | UN07 | no | |
| A-MD20 | Effect system | partial | S:160 "effects"; S:426 research effects. Code `EffectDef` has one variant `WeaponModifier` (`schema.rs:287-293`) | B | M3 (one effect) | after CTL | simcore-6 | yes: closed enum | |
| A-UN18 | Deployables | partial | S:160 "transformations" | B | none | later | OR27 | no | |
| A-CO01 | Placement ghost | partial | S:158 "Placement, footprints"; "ghost" 0 | M | none | PLAY | CO03 (sim query) | no | presentation primary; sim gives the validity query |
| A-CO02 | Footprint validity per cell | partial | S:158, S:460 | M | none | PLAY | CO03 | no | same query as CO03 |
| A-CO03 | Placement rules | partial | S:158; S:460; S:470; S:712 "placement constraints" open for default game | B | M4 adds `ForbidEdit` only (M4D:35 K9) | PLAY | terrain grid (M4) | no | B-18 |
| A-CO04 | Construction model | partial | S:158 "worker-built versus instant placement" is mod policy; S:745 (Decided, first mod) | B | none | PLAY | CO03, OR16 | no | |
| A-CO05 | Worker build queue | missing | "build queue" 0 | M | none | PLAY | OR12, CO04 | no | B-55 |
| A-CO06 | Multi-worker build | missing | "multiple workers", "several workers" 0 | B | none | later (OPTIONAL) | CO04 | no | |
| A-CO07 | Under-construction state | partial | S:158 "progress"; S:745 | B | none | PLAY | CB01, CO04 | no | |
| A-CO08 | Cancel and refund of a site | partial | S:406 refunds explicit (production only) | B | none | PLAY | PR02 primitives | no | reuse `CancelRefunds` (`production.rs:152-240`) |
| A-CO09 | Wall drag | missing | "drag" 0 | M | none | later | CO03 batch query | no | B-54 |
| A-CO10 | Gates | missing | "gate" (wall) 0 | B | none | later (OPTIONAL) | CO09 | no | |
| A-CO11 | Repair | partial | S:742 (Decided, first mod); S:751 "repair accounting" open | B | none | PLAY | CB01, transactions | no | B-51 |
| A-CO12 | Sell and deconstruct | missing | "sell", "deconstruct", "reclaim" 0. Code: `Remove` with `ContentsPolicy::Destroy` only (`schema.rs:268-277`, M3D K3) | B | M3 (remove, no refund) | PLAY | CO08 | yes: add refund policy values (M3D K3 plans enum growth) | B-51 |
| A-CO13 | Building rotation | partial | Code `Facing` (`schema.rs:175-179`) at map placement only | M | M3 (map only) | PLAY | CO01 | no | |
| A-CO14 | Foundation and flatten | present | S:444 "Level a site for construction"; S:470 | B | M4 | M4 | terrain grid | no | |
| A-CO15 | Upgrade in place | missing | "morph" 0; "upgrade" S:161,490,678 are research | B | none | later | PR08, simcore-4 | no | |
| A-CO16 | Grid overlay | missing | "grid overlay" 0 | M | none | PLAY | CO03 | no | presentation |
| A-CO17 | Line drag placement | missing | "drag" 0. M3 belts are map-placed (`proving_ground.ron` `belts`) | M | none | later | CO09 | no | |
| A-CO18 | Blueprints | missing | "blueprint", "template" 0 | M | none | later | CO03 batch | no | B-59 |
| A-CO19 | Capture | missing | "capture", "convert" 0 | B | none | later | simcore-5 | yes: `Owner` is set at spawn only (`world.rs:16-18`) | B-52 |
| A-CO21 | Placement vs units | missing | "placement" hits do not cover units on the footprint | M | none | PLAY | CO03, unit clearance (M4D:36 K10) | no | |
| A-CO22 | Build radius | missing | "build radius", "adjacency" 0 | B | none | PLAY | CO03 query hook | no | |
| A-PR01 | Production queue | present | S:142; S:410. Code `ProducerDef.queue_capacity` `schema.rs:167-171`; `production.rs` | B | M2 (built) | - | none | no | B-19 |
| A-PR02 | Cancel and refund | present | S:406; S:707 (Open API). Code stage refunds `production.rs:152-240` | B | M2 (built) | - | PR03 | no | B-37 |
| A-PR03 | Payment policy | partial | S:406 lists three policies. Code has two: `PaymentPolicy { ChargeOnQueue, ReserveThenCharge }` `schema.rs:390-393`; consume-over-time deferred (M3D K6) | P | M2 (2 of 3) | PLAY | EC01 | yes: add the third value | CNC sidebar needs consume-over-time |
| A-PR04 | Parallel production | partial | S:410 "without assuming identical queue semantics". Code: one queue per producer only | B | M2 (per building) | later | PR01 | no | CNC shared category queue absent |
| A-PR05 | Spawn and exit | partial | S:142 "unit-spawn output". Code `spawn_offsets` and `OutputBlocked` `production.rs:357-361` | M | M2 (built) | - | PF04 | no | |
| A-PR06 | Rally applied | missing | "rally" 0 | M | none | CTL | OR15, OR11 | yes: spawned unit gets no order (`production.rs:476-518`) | B-20 |
| A-PR07 | Prerequisites | partial | S:158, S:161, S:424. Code: research only (`research.rs:114-115`) | B | M3 (research) | PLAY | MD05 | no | units/buildings need prerequisites too |
| A-PR08 | Research and upgrades | present | S:422-428 (graph; definitions apart from state). Code `economy/research.rs` | B | M3 (built) | - | PR07 | no | |
| A-PR09 | Ages and tiers | missing | "age", "tier" 0 real hits (S:9 is a game name) | B | none | later | PR08, unlock effects | no | B-65 |
| A-PR10 | Unavailable reason | partial | S:782 (bots, Proposed). Code: reasons only at submit (`command.rs:78-112`); no "why not" query | B | M3 (submit-time) | CTL | OR22 | no | |
| A-PR11 | Repeat production | missing | S:428 "repeatable technologies" Open (research only) | M | none | later (OPTIONAL) | PR01 | no | |
| A-PR12 | Batch queue | missing | `Produce` queues one job (`messages.rs:63-66`) | M | none | CTL | PR01 | yes (small): count field | |
| A-PR13 | Multi-building production | missing | `Produce` names one `producer_id` | M | none | CTL | PR01, OR24 | yes | |
| A-PR14 | Supply block | missing | S:157 "population rules" mod policy; no mechanism | B | none | PLAY | EC08 | no | B-22 |
| A-PR15 | Modifier application | partial | S:161; S:428 stacking Open. Code: damage add per shot from research state `combat.rs:143-150` | B | M3 (one stat) | later | MD20 | yes: one stat (`WeaponStat::Damage`, `schema.rs:281-283`) | |
| A-PR16 | Power effect | partial | S:746 (Decided, first mod); S:420 | B | none | PLAY / M6 | EC09 | no | utility seat |
| A-PR17 | Asymmetric factions | partial | S:163 "Factions" (mod policy); no faction definition | P | none | PLAY | MD06 | no | |
| A-PR18 | Recipes | present | S:410. Code `RecipeDef` `schema.rs:214-221`; `machine.rs` | B | M3 (built) | - | EC01 | no | |
| A-PR19 | Exclusive branches | partial | S:428 Open policy | B | none | later | PR07 | no | Open: needs decision when a mod needs it |
| A-EC01 | Resource types | present | S:392-400; S:157 | B | M2 (built) | - | none | no | |
| A-EC02 | Gather cycle | present | S:140; S:743. Code stages `gather.rs:33-40` | B | M2 (built) | - | OR17 | no | B-33 |
| A-EC03 | Drop-off choice | partial | "drop-off" 0; S:743 delivers to command center. Code `DropOffDef` exists; nearest rule NOT TRACED | M | M2 | - | EC02 | NOT TRACED | |
| A-EC04 | Node depletion | present | S:679 (M4); code `NodeDepleted` `gather.rs:126-128` | B | M2 (built) | - | EC02 | no | |
| A-EC05 | Auto-resume and seek | partial | no spec text. Code `WaitReason::NoDeposit` `gather.rs:25-30`; seek of a new node NOT TRACED | M | M2 | CTL | EC04, OR26 | NOT TRACED | |
| A-EC06 | Node occupancy | missing | "occupan" hits S:158,270,330,414,451: none on nodes | B | none | PLAY | EC02 | no | |
| A-EC07 | Generators | present | S:396-398. Code: extractor on a node (M3) | B | M3 (built) | - | EC01 | no | |
| A-EC08 | Population cap | partial | S:157 "population rules" (mod policy) | B | none | PLAY | EC01 | no | B-22 |
| A-EC09 | Power | partial | S:398 network quantity; S:746 | B | none | PLAY / M6 | EC01 | no | B-23; utility seat |
| A-EC10 | Insufficient funds | partial | S:406. Code reject `InsufficientBalance` (`command.rs:95`); "pause until funds" needs consume-over-time (PR03) | P | M2 (reject) | PLAY | PR03 | no | |
| A-EC11 | Tribute and transfer | missing | "tribute" 0; "transfer" hits are inventory transfers (S:404, S:414) | B | none | later | simcore-5 | no | |
| A-EC12 | Market | missing | "market" 0 | B | none | later (OPTIONAL) | EC01 | no | |
| A-EC13 | Storage caps | partial | S:404 inventory capacities; no balance cap | B | M2 (inventories) | later | EC01 | no | |
| A-EC14 | Regrowing resources | missing | "regrow", "spread" 0 | B | none | later (OPTIONAL) | EC04 | no | S:480 forbids regrowth of mined ore only |
| A-EC15 | Upkeep | partial | S:157 "upkeep" (mod policy); no periodic-charge mechanism | P | none | later | EC08 | no | B-64 |
| A-EC16 | Physical items and logistics | present | S:397; S:412-420 | B | M2/M3 (built) | - | EC01 | no | |
| A-EC17 | Income rate display | missing | "income" 1 hit S:768 (AI) | M | none | later | stats query | no | |
| A-UN05 | Transports | partial | S:180 "transporting passengers" | B | none | later | OR19, simcore-4 | no | B-53 |
| A-UN06 | Garrison | missing | "garrison" 0 | B | none | later | OR19 | no | B-53 |
| A-MD14 | Event scripting API | partial | S:123; S:181; S:201 (production, research, discovery, lifecycle). Scripts Open (S:705). Code: event list `economy/events.rs` (no unit-died hook for mods, no order events) | B | none | after CTL | simcore-7 | no | |
| simcore-1 | One order layer instead of one store per activity | missing | Stores: `MoveTarget` `world.rs:31-33`; `EconomyOrders` `lib.rs:51`; `ProductionQueues`; M4 earthwork store M4D:146 | M | M4 adds a 4th store | CTL | OR11 | yes: M4 draft must not lock in its own order store | lens answer |
| simcore-2 | Unit id to entity lookup is a full scan | missing | S:616 (Baseline) asks for spatial indexes. Code scans all entities per command and per weapon: `command.rs:220-235`, `command_m3.rs:19-22`, `combat.rs:120-129,165-199`, `production.rs:368-381`. Run 4: 51 us, 371 us, 1096 us per 1-unit `MoveTo` at 1,000, 10,000, 40,000 units | M | none | CTL (id index), M7 (spatial) | none | yes | at 20 Hz (50 ms tick) and 40,000 units, about 45 orders fill one tick |
| simcore-3 | A unit with a queue cannot be killed | conflict | Load rules `turts_mod_runtime/src/content_m3_validation.rs:374-376` (health needs removal) and `:424-428` (a queue forbids removal). Run 6: lab with `health` is rejected "a unit with a queue cannot be removed". M3D:154-157 labels it a prototype limit | B | M3 (built) | PLAY | UN13, S:406 destruction policy | yes | blocks S:744 + S:748 (Decided, first mod): the command center produces units and must die for victory |
| simcore-4 | At most one work capability per unit | conflict | M3D:154-157; `content_m3_validation.rs:396-422`. Weapon counts as work | B | M3 (built) | PLAY | S:136-147 (4.2) | yes | blocks armed producers, producer + research (town center), armed harvesters |
| simcore-5 | Owner change, neutral owner, ally relation | missing | "neutral" 0. Code `Owner(SlotId)` set at spawn (`world.rs:16-18`); `SlotRelation { Other, Same, Any }` has no ally (`schema.rs:237-241`) | M | none | PLAY | teams (S:163) | yes | capture, conversion, tribute of units, creeps, shared vision targets |
| simcore-6 | Timers and status effects (buffs) | partial | S:160 "timers", "buffs". Code: weapon cooldown only (`combat.rs:30-36`) | M | none | after CTL | MD20 | no | abilities, regeneration, slows |
| simcore-7 | Unit and order events for mods and bots | partial | S:201 lists production, research, discovery, lifecycle; no unit-died, order-done, order-failed. S:771 tactical state machines need them | B | none | CTL | OR26 | no | |
| simcore-8 | Order and unit state on the wire | partial | S:251. Wire carries position (and facing at snapshot) only (`messages.rs:36-47`) | M | none | CTL | OR11 | yes: protocol bump | needed for waypoints, rally flags, health bars |
| simcore-9 | Seeded gameplay random streams | partial | S:211 seeded streams for map generation; S:239 randomness rules. Run 7: no RNG in `turts_sim/src` | M | none | PLAY (first random rule) | none | no | accuracy (CB20), spread, critical hits need one seeded stream per purpose |
| simcore-10 | First playable mod has no milestone | wrong-milestone | D44 (Decided), D55 (Decided), S:726-751; not in S:674-688; S:694 "additional proposed validation" only. Construction excluded M2D:46, M3D:28, M4D:21 | B | none | PLAY | CTL, simcore-3, simcore-4 | no | combat orders, construction, repair, power are Decided rules with no slot |

## Conflicts with the goal

No Decided item in section 2 blocks an order model, a construction mechanism, or a death state.
The conflicts are in Baseline text and in built code, not in Decided rows:

- simcore-3 is a built M3 rule. It contradicts S:744 and S:748. These are Decided rules for the
  first mod (D55). M3D:154-157 labels the rule a prototype limit. It must go before PLAY.
- simcore-4 is a built M3 rule. It contradicts S:136-147 (4.2, composition). M3D:157 says 4.2
  "stays the long-term rule".
- A-OR24: the all-or-nothing group rule (M1) contradicts the genre rule "each unit that cannot do
  it ignores it". S:342 (Proposed) names group orders but gives no rule. Needs decision: atomic
  group (current) or per-unit acceptance (genre). Evidence: `command.rs:274-283`, run 2.
- Tension, not conflict: D39 (Decided direction, "avoid arbitrary gameplay caps") and the engine
  cap `MAX_UNITS_PER_COMMAND = 256` (`command.rs:19`). A 300-unit selection needs two commands.
- D44 and D55 (Decided) have no milestone (simcore-10). This is a schedule gap, not a conflict.

## Lens answer

What an RTS order model needs (direction only, no design):

1. One order queue per unit (OR11). Each entry has an order kind, a target, and a queue mode
   (replace, append). Target kinds: none (stop, hold), point (move, attack-move, patrol, build
   site), unit (attack, follow, repair, load, gather from a node entity), area (abilities).
2. One executor per unit runs the current order. Activities (gather, earthwork, build, attack)
   are executors, not separate stores.
3. One engine interruption rule set (OR26): a new order, a queued order, auto-target, and
   retaliation. Mods select stance values (OR09). The engine owns the mechanism.
4. Group fan-out with a per-unit result (OR24), and per-unit reasons (OR22).
5. Producers hold an order template (rally, OR15). A spawned unit takes it (PR06).
6. Order state replicates to the owner (S:251; simcore-8).

How far the current path is: far. There is no order concept. `CommandKind` (`command.rs:30-55`)
and `PlayerCommandKind` (`messages.rs:54-78`) are one-shot verbs. Each verb writes its own store:
`MoveTarget` (`world.rs:31-33`), `EconomyOrders` (`gather.rs:163-176`), production queues, and in
M4 an earthwork store (M4D:146). Interruption is pairwise inline code (`command.rs:241-251`,
`gather.rs:162`, `gather.rs:188`). N activities need N x N rules. M4D:146 already shows the
growth: it states earthwork against `MoveTo` and `Remove`, but not against `Gather`. Combat has no
order input; the weapon picks the nearest unit in range (`combat.rs:165-199`). There is no target
of kind unit or none. Group validation is atomic (`command.rs:274-283`). Each lookup scans all
entities (simcore-2).

Extend or replace: **extend the intake, replace the order layer.**

- Keep: `CommandQueue`, the drain in `FixedPreUpdate` after `advance_tick` (`plugin.rs:88,110-112`),
  `seq`, `CommandResult` and the stable `RejectReason` names (`command.rs:57-152`), the transaction
  capture (`command.rs:155-159`), the movement integrator as the low-level locomotion goal
  (`movement.rs:10-58`), the gather stage machine as the Gather executor (`gather.rs:33-40`), and
  the production queues. Spec 7.3 (S:410) says unit production and unit orders need not share
  queue semantics.
- Replace: the per-verb order stores; the pairwise interruption code; the wire verb shapes (some
  take a unit list, some one unit: `messages.rs:55-62`); atomic group validation; combat target
  selection without an order input; the full-scan lookups.

What M1 to M3 built that must change: `command.rs:212-253` (MoveTo writes `MoveTarget` and resets
gather inline); `gather.rs:88-178` (state in a resource keyed by id); `combat.rs:165-199` (no order
target); `content_m3_validation.rs:374-376,396-428` (simcore-3, simcore-4); `messages.rs:36-78`
(no order state on the wire; verb shapes); `schema.rs:390-393` (two of three payment policies).
The M4 draft must not add a fourth store (M4D:146). Do the order layer before M4 is gated.

## What I ran

| # | command | claim tested | result that would refute | observed |
| --- | --- | --- | --- | --- |
| 1 | `python3 -I simcore/sweep.py RTS-Engine-Project-Specifications.md <66 terms>` (output `simcore/spec-sweep.txt`) | yardstick B5: spec has no stop, patrol, hold, waypoint, rally, health, garrison, sell, capture | any real hit for those terms | 0 hits each; "stance", "guard", "follow", "regenerat" hits are noise (context printed) |
| 2 | `cargo test -p turts_sim --test command` | `command.rs:274-283` rejects a whole group for one foreign unit | test `mixed_ownership_..._moves_nothing` fails | 11 passed, including it |
| 3 | `sweep.py` over M1D, M2D, M3D, M4D (`simcore/milestone-sweep.txt`) | no milestone designs an order queue, attack order, rally, or consume-over-time payment | any hit for those terms | 0 hits each; construction excluded M2D:46, M3D:28, M4D:21 |
| 4 | probe `simcore/probe` (release): 200 single-unit `MoveTo` at 1k/10k/40k units | `command.rs:220-235` makes one order cost O(world size) | flat cost across sizes | 51.2, 370.5, 1095.6 us; second `MoveTo` replaces the first (no queue). Output `simcore/probe-out.txt` |
| 5 | `grep -rn "rate.?limit|commands_per|per_tick|max_commands|budget" turts_server/src` | no per-tick command cap exists (OR31) | a cap in the server | 0 relevant hits |
| 6 | probe `simcore/probe2` on a copy of forge with `health` on the lab | a queue-holding unit can be made killable | package loads | `ModError` "a unit with health requires removal", then "a unit with a queue cannot be removed" (`simcore/probe2-out.txt`) |
| 7 | `grep -rn "rng|Rng|rand::|thread_rng|SmallRng|StdRng" turts_sim/src` | sim has no gameplay random source (simcore-9) | any hit | 0 hits |

## NOT TRACED

1. A-EC03: the rule that picks the drop-off (nearest or other). I did not read `gather.rs` receiver selection.
2. A-EC05: whether a gatherer seeks a new node when its node is empty.
3. Kill of a lab with a running job inside combat (the rollback at `combat.rs:97`): the probe
   stopped at load validation (run 6), so the combat rollback path is read, not run. It is
   unreachable with shipped content.
