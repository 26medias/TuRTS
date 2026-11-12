# MP1 loop walk (game-designer, 2026-11-11)

Each arrow names one public mod API capability. EXISTS = in the API today (path:line).
GAP = new engine mechanism for all mods (MP1 engine work).

| # | Arrow | Capability | State |
| --- | --- | --- | --- |
| 1 | Harvester -> ore field | `Gather` order, `gatherer` on UnitDef | EXISTS: orders.rs:37, schema.rs:150; cycle gather.rs:202-304 |
| 2 | Field depleted -> next field | nearest live node of same item | EXISTS: gather.rs:333-379 |
| 3 | Ore -> command centre -> credits | `drop_off` rule `Convert(balance, per_item)` | EXISTS: schema.rs:158-172; base command_center.ron |
| 4 | Credits -> units | `ProducerDef` + `ProductionDef`, cost `SlotBalance` | EXISTS: schema.rs:175,417; production.rs:37-150 |
| 4a | Payment over time | `PaymentPolicy::ConsumeOverTime` | GAP: only ChargeOnQueue, ReserveThenCharge (schema.rs:398-401) |
| 4b | Units and buildings in parallel | producer lanes | GAP: one FIFO head per producer (production.rs:266-354) |
| 5 | Credits -> buildings | place order + construction site | GAP: no construction (spec 14 MP1 row, S:723) |
| 6 | Building -> power | network resource, supply and demand | GAP: ResourceKind is Balance or Item only (schema.rs:125-128) |
| 7 | Power -> production speed | shortage multiplier on work | GAP |
| 8 | Power -> turrets | shortage disables weapon | GAP (a `Disabled` set exists for SetEnabled: combat.rs:133) |
| 9 | Units -> army -> combat | weapon, target filter, Attack, StopAndFire | EXISTS: schema.rs:266-272, combat.rs:82-256 |
| 9a | Damage type x armour | damage table | GAP: damage is flat (combat.rs:371-381) |
| 9b | Team relation in targeting | Enemy/Ally filter | GAP: SlotRelation Other/Same/Any only (schema.rs:245-249) |
| 9c | Idle units answer fire | idle acquisition range | GAP: idle fires in weapon range only (combat.rs:297-339) |
| 10 | Engineer -> repair | `repairer` capability, `Repair` order | GAP |
| 11 | Combat -> destruction | health 0 removes unit | EXISTS for units without queue; GAP for CC (content_m3_validation.rs:420-424) |
| 12 | Destruction -> victory | match rule `LastTeamStanding(units)` | GAP: no match end |
| 13 | Victory -> result screen | `MatchResult` source, `MatchOver` predicate | GAP |
| 14 | New unit -> rally | `SetRally` order, producer rally point | GAP |

Dead-end check:
- Credits have sinks: every unit, building and repair. Ore has one sink (drop-off). Power has no
  stock, so it has no sink; it is a constraint, not a currency.
- Every unit has a counter and a role (see balance2.out). No unit has no reason to exist.
- The CC is not buildable, so the count of CCs only falls. The victory rule resolves when a
  side attacks. A passive match does not resolve: Surrender closes that hole.
