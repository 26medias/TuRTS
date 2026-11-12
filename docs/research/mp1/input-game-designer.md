# MP1 input: game-designer (the first playable mod)

Source: subagent hand-back, MP1 brainstorm input, 2026-11-11. Agent output, not reviewed; claims are leads until verified.

VERDICT=INPUT

The report runs past my 600-word limit. Your nine-part brief asks for full stat tables, so I put the whole design here and the long run output in scratchpad files (paths at the end).

**Status of the work.**
- The roster and the basic rules are Decided: S:782-805, D44, D55.
- All names, statistics, timings, costs, repair accounting, placement rules, the power multiplier and the simultaneous-destruction case are Open (S:807, S:768). Below they are proposals that need a measurement.
- MP1 content stays Proposed until its spec passes gate 1 (S:701).
- I put the content in a new package (working id `frontier`), not in `base`. The frozen M2 stream test uses `base` (`crates/turts_pack/tests/m3_m2_stream.rs:350-366`).
- All ids below pass my scan probe against engine identifiers and literals. `light` and `shell` collide, so I do not use them.

## 1. Roster and statistics

| id | Working name | Role |
| --- | --- | --- |
| `rifleman` | Rifleman | Cheap line infantry. Kills infantry. Screens. |
| `launcher` | Launcher | Anti-vehicle infantry. Kills tanks and haulers. Weak against riflemen. |
| `engineer` | Engineer | Repairs vehicles and buildings. Has no weapon. |
| `ore_hauler` | Ore Hauler | The harvester. Brings ore to the command center. |
| `tank` | Tank | Main battle unit and siege unit. |
| `guard_turret` | Guard Turret | Static defence. Needs power. |
| `command_center` | Command Center | Trains all units, builds all buildings, takes ore. Victory key. |
| `power_station` | Power Station | Supplies power. |

Resources:
- `ore` is an Item.
- `credits` is a Balance.
- `power` is a Network quantity (S:424-426). This kind is new.
- The node type is `ore_field`.

**Mobile units.** Speed, radius and range are in world units. Build times are at full power.

| Stat | Rifleman | Launcher | Engineer | Ore Hauler | Tank |
| --- | --- | --- | --- | --- | --- |
| Cost | 100 | 250 | 300 | 800 | 700 |
| Build time | 6 s | 9 s | 10 s | 20 s | 18 s |
| Health | 120 | 90 | 70 | 600 | 500 |
| Armour class | infantry | infantry | infantry | vehicle | vehicle |
| Speed | 2.4 | 2.0 | 2.2 | 2.0 | 2.6 |
| Radius | 0.4 | 0.4 | 0.4 | 0.9 | 0.9 |
| Sight | 9 | 9 | 7 | 6 | 10 |
| Idle acquire range | 7 | 9 | repair 6 | none | 8 |
| Weapon | 12 bullet, range 5, 1000 ms | 60 rocket, range 7, 2500 ms | none | none | 40 cannon, range 6, 2000 ms |
| Power | 0 | 0 | 0 | 0 | 0 |
| Made by | CC units lane | CC units lane | CC units lane | CC units lane | CC units lane |
| Orders | move, stop, hold, attack | move, stop, hold, attack | move, stop, hold, repair | move, stop, gather | move, stop, hold, attack |

**Buildings.**

| Stat | Command Center | Power Station | Guard Turret |
| --- | --- | --- | --- |
| Cost | not buildable (map start only) | 300 | 500 |
| Build time | none | 12 s | 20 s |
| Health | 2000 | 600 | 600 |
| Armour class | structure | structure | structure |
| Radius | 2.5 | 1.5 | 1.0 |
| Sight | 10 | 6 | 10 |
| Weapon | none (one work capability: `content_m3_validation.rs:392-419`) | none | 40 cannon, range 8, 1500 ms |
| Power | uses 40 | makes 100 | uses 30 |
| Made by | not made | CC structures lane | CC structures lane |
| Extends build area | yes, 10 | yes, 10 | no |
| Orders | produce, cancel, place, set rally | cancel (while it is a site) | stop, hold, attack (a unit) |

**Damage table.** Each hit does `max(1, floor(damage x percent / 100))`.

| Damage type | infantry | vehicle | structure |
| --- | --- | --- | --- |
| bullet | 100 | 20 | 20 |
| rocket | 30 | 100 | 50 |
| cannon | 75 | 100 | 100 |

**The ratios that carry the balance.** Source: `balance2.out`. The model is a focus-fire duel with no approach and no range, so it is a proposal to measure.
1. **Time to kill.**
    - A rifleman kills a rifleman in 9 s. A tank kills a rifleman in 6 s. Fights take seconds, so a player can read them.
    - At equal cost, 7 riflemen lose to 1 tank, and the tank keeps 66 of 500 health. 8 riflemen win.
    - 3 launchers kill a tank in 7.5 s and lose 1.
    - 5 riflemen kill 2 launchers in 3 s and lose none.
    - My first pass had bullet at 25% against vehicles and rocket at 75% against structures. Then 7 riflemen beat a tank and 4 launchers beat a turret, so I lowered both cells (`balance.out`).
2. **Hauler income against unit cost.**
    - One hauler on the main field makes about 330 credits/min: 365 in the model, minus my 10% guess for overhead.
    - A hauler pays back its cost in about 2.4 min. A tank costs about 2.1 hauler-minutes.
3. **Turret against tank.**
    - 1 turret (500) beats 1 tank (700) and keeps 200 health.
    - 2 tanks kill the turret in 14 s.
    - A turret with 2 engineers beats 2 tanks.
    - The turret outranges the tank (8 against 6) and the launcher (8 against 7).
4. **Power against production.**
    - One station (100) covers the CC (40) and 2 turrets (60).
    - A third turret causes a shortage, so the player must build a second station first.

**Measurement.** MP1 adds headless duel scenarios for each ratio row and a 10-trip hauler timing. The numbers stay Open until they are measured.

## 2. Economy

**Hauler.**
- It carries 200 ore and gathers 10 ore/s (20 s to fill). It unloads 40 ore/s (5 s).
- 1 ore converts to 1 credit through the existing `Convert` drop-off rule (`schema.rs:158-172`).
- When a node is empty, the hauler goes to the nearest live node on its own (`gather.rs:333-379`).

**Trips.** Reach distances come from `gather.rs:438,652`.

| Field | One trip | Income | Size |
| --- | --- | --- | --- |
| Main | 32.8 s | about 365/min | 3 nodes x 1,500 |
| Side | 40.8 s | about 294/min | 2 nodes x 1,500 |
| Centre | 62.6 s | about 192/min | 4 nodes x 2,000, twice |

**Income curve.**
- 1 hauler makes about 330/min. 2 make about 660/min. 3 make about 1,000/min.
- The main field lasts 13.6 min with 1 hauler, 6.8 min with 2 and 4.5 min with 3. So the mid-game forces players onto the side fields, then the centre fields.
- NOT TRACED: whether two gatherers on one node slow each other.

**Starting credits: 1,200.** That pays for power (300) plus one real first choice: a second hauler (800), or a turret and some army. Source: `economy.out`.
- **Plan A (power, then a second hauler).** By 3:00 the player has 1,800 harvested, 950 in the bank, a turret, 2 riflemen and a launcher.
- **Plan B (power, turret, army).** By 3:00 the player has 1,000 harvested, a turret, 3 riflemen, a launcher and a tank, with 150 in the bank.

**Payment: consume over time** (S:434 names it).
- **Rate.** The total charge is `floor(cost x progress)`. Each tick charges the difference, so the total at completion equals the cost.
- **Low power.** Progress runs at half speed, and spending runs at half speed with it.
- **Credits run out.** The job does not advance in a tick that it cannot pay. It shows "Waiting for credits" and starts again on its own. The balance never goes below 0.
- **Payment order.** Producer id order, then lane order as the file declares it. Both are deterministic.
- **Cancel.** It refunds 100% of what the job spent: `cancel_refunds` all 1/1.
- **Blocked output.** The job waits complete and charges nothing more. Cancel still refunds 100%.
- **Destruction.** A destroyed site or CC loses everything it spent. There is no refund.

## 3. Construction

**Who builds: the CC.** There is no builder unit.
- 16.2 says the CC "manages building construction" (S:800).
- For a first-time player there is one place to look and no worker to lose.
- For the engine there is no builder order, no worker assignment and no worker travel.

**Flow.**
1. Select the CC, then pick a building in the build menu. A ghost follows the pointer.
2. Left-click a valid spot. A site appears at once with 10% health. Its health grows with progress, and it pays over time.
3. Esc or a right-click leaves placement mode at no cost.
4. The structures lane takes one site at a time (capacity 1).

**Placement rules.**
- The site is fully inside the map.
- Its centre is within 10 of the edge of an own completed CC or power station.
- It does not overlap any footprint, units included, with a 0.5 gap.
- It is at least 2.0 from the edge of an ore node.
- Footprints stay circles (`schema.rs:28-30`).

**Ghost.**
- Valid: `#56B4E9` outline with fill.
- Not valid: `#D55E00` with a hatch, the forbidden cursor, and a reason. The reasons are "Too far from your base", "Something is in the way", "Too close to ore" and "Outside the map". The meaning does not depend on colour alone.
- The build-area rings show while the player places.

**Cancel.** Select the site and press Cancel. This removes the site and refunds 100% of what it spent.

**Repair.**
- The engineer repairs an own vehicle or structure at 8 HP/s, with a reach of 1.5.
- It costs 1 credit for each 2 HP, charged over time. Without credits it stalls.
- Engineers stack.
- An idle engineer repairs on its own within 6.

## 4. Power

**The rule in one sentence:** "When your base uses more power than it makes, everything builds at half speed and your turrets switch off."
- Only completed buildings make or use power.
- MP1 has one network for each slot.
- At the start the player has 0 made and 40 used, so the first lesson is "build power".
- Repair is not slowed.

**What the HUD must show:**
- A power bar and the text "Power 100 / 70".
- On a shortage: a colour change, an icon and the text "LOW POWER: half speed, turrets off".
- "Half speed" on the CC panel.
- An "unpowered" icon over each turret.
- The alerts "Low power" (with a ping) and "Power restored".
- In each build-menu tooltip, the power of that building (+100 or -30), and the warning "This causes low power" when it applies.

## 5. Combat

**Orders.**
- Move: `fire: Never`, as in the current packages.
- Attack takes `PointOrUnit`, which is allowed (`content_mc_validation.rs:1298`). On a unit it attacks that unit. On a point it is attack-move with `StopAndFire`.
- Stop clears the order queue. The unit is then idle.
- Hold: the unit never moves and fires at targets in range.

**Automatic targeting.**
- An idle unit attacks the nearest enemy within its acquire range. It goes back when it is 6 from its idle point.
- The turret fires at the nearest enemy in range 8 when it has power.
- An ordered target comes first (`combat.rs:323-326`).
- Today an idle unit fires only at targets inside its weapon range (`combat.rs:297-339`). An outranged idle unit dies in place, so idle acquisition is engine work.

**Effects the mod asks for:**
- Muzzle flash and a tracer for each damage type.
- A hit spark, a rocket burst and a cannon burst.
- Infantry fall and fade.
- Vehicle explosion and wreck decal (10 s).
- Building explosion and scorch decal.
- Construction dust.
- Repair arc.
- Turret power-down.
- "+200" floating text on each delivery.
- Positional sounds.

A gap blocks all of these: no shot event goes on the wire today. There is no `Fired` in `crates/turts_net_protocol/src`, and the sim event goes to the shooter's slot only (`combat.rs:393-394`).

**Rally.**
- Right-click the ground with the CC selected to set the rally point. A flag and a line show.
- Right-click an ore node to make a gather rally for haulers.
- Combat units attack-move to the rally point.

## 6. The match

**Teams.** MP1 has two teams of one. The engine target filter today knows only slots: Other, Same, Any (`schema.rs:245-249`). For later, teams must support:
- more than one slot in a team;
- Enemy and Ally relations in target filters and auto-targeting;
- victory for each team;
- teams that are fixed at the start.

**Map "Twin Fields".** Flat, 112 x 72, point-symmetric about (56, 36).

| Item | Slot 0 | Slot 1 |
| --- | --- | --- |
| CC | (16, 36) | (96, 36) |
| Ore Hauler | (20, 32) | (92, 40) |
| Main field, 1,500 each | (14, 22), (17.5, 21), (21, 22.5) | (98, 50), (94.5, 51), (91, 49.5) |
| Side field, 1,500 each | (14, 58), (18, 60) | (98, 14), (94, 12) |

- Centre north, 2,000 each: (54, 14), (58, 14), (56, 11), (56, 17).
- Centre south is the mirror of centre north.
- The CCs are 80 apart. Each centre field is 43.9 to 47.4 from both CCs.

**Victory.**
- At the end of each tick, a team is alive if one of its slots has a CC with health above 0.
- When exactly one team is alive, it wins.
- When no team is alive in one tick, every team that was alive at the start of that tick gets **Draw**.
- Surrender defeats the slot at once. Without it, a passive match never ends.
- A disconnect changes nothing (D25, S:323; D28 keeps disconnect rules out of the engine).
- Shooters fire in unit id order (`combat.rs:131-144`), so the lower id fires first in a tie. This is an engine note.

**Result screen.**
- The outcome and its reason.
- The match time.
- For each slot: colour, team, outcome, credits harvested, credits spent, units trained, units lost, enemy units destroyed, buildings built, buildings lost.
- Two buttons: "Back to lobby" and "Leave".

**Lobby.**
- For each slot: Team (1 or 2), a colour from the mod palette (each colour used once), and a Ready toggle.
- The start needs two teams.
- Settings schema: none is needed for MP1. As an option, a "Starting credits" choice of 1,200, 2,500 or 5,000. The server checks that the value is in the list.

## 7. Mechanism or policy, and the new API

**Mod policy** (data in the package):
- every number above;
- the roster;
- the damage table values;
- the power rule values;
- the placement values;
- the map;
- the victory parameters;
- the guide pages;
- the HUD and result layout.

**Engine mechanisms.** Each line is one new item of public API.
1. `UnitDef.armour: Option<ContentRef>`.
2. `WeaponDef.damage_type: Option<ContentRef>`.
3. A `combat` export with a damage table (type, class, percent).
4. `ResourceKind::Network`.
5. `UnitDef.power: (supply, demand, shortage: Slow(num, den) | Disable)`.
6. `PaymentPolicy::ConsumeOverTime`, with a stall stage "WaitingForFunds".
7. `ProducerDef.lanes: [(id, recipes, capacity)]`.
8. Structure production: a `Place` behaviour with `Point` plus content.
9. A site entity whose health grows with progress.
10. `UnitDef.placement: (extends_area, clearance, node_clearance)`.
11. `UnitDef.repairer: (rate, reach, cost per HP, classes, auto_range)`, plus a `Repair` behaviour and a `Repairer` capability.
12. `UnitDef.acquire: (range, leash)` for idle units.
13. `SlotRelation` grows to Enemy, Ally and Own.
14. A `SetRally` behaviour and smart rows: Ground needs Producer; Node sets a gather rally.
15. Lift the rule `content_m3_validation.rs:420-424` ("a unit with a queue cannot be removed"), with an explicit destruction policy for queued jobs.
16. A `match` export with `victory: LastTeamStanding(units)` and Surrender.
17. A lobby definition with team count, palette and settings schema (S:314).
18. HUD sources: `Power`, `MatchResult`, `MatchClock`, `Rally`, lane queues, `SelectionPrimary.Progress` and `.Armour`.
19. HUD predicates: `PowerShort`, `Placing`, `MatchOver`.
20. Alert triggers: `LowPower`, `PowerRestored`, `ConstructionComplete`, `WaitingForFunds`, `NodeDepleted`, and `OwnUnitDamaged` with a unit filter.
21. Wire events visible to every client that may see them: Fired, Destroyed, Repaired, Delivered, SlotDefeated, MatchEnded.
22. Presentation: effects for each unit and weapon (flash, tracer, burst, decal, sound).

**Private shortcuts.** None is needed, if these rules hold:
- The victory key, power, damage names and build area are data. No content id goes into engine code.
- The result screen is a mod screen (D70, Proposed).
- The hauler's rally gather is the generic smart rule, not a hauler special case.

## 8. The first-time player

**Order of learning in the first 2 minutes:**
1. The camera is on the CC and the guide opens.
2. Select the CC.
3. Place a power station. The player learns the ghost and its reasons.
4. Watch the "Low power" banner go away at about 0:24.
5. See "+200" at the first delivery, at about 0:31. Ore makes credits.
6. Make the first choice: a second hauler, or a turret and an army.
7. Set a rally point.
8. Move units, attack, and attack-move.
9. Scout the centre field.

**Guide pages:**
1. **Your goal:** destroy the enemy Command Center and keep yours.
2. **Credits:** your Ore Hauler fills at the ore field and unloads at the Command Center.
3. **Build power first:** select the Command Center, pick Power Station, and click a blue spot.
4. **Low power:** everything builds at half speed and your turrets switch off.
5. **Train units:** new units go to your rally point. Right-click the ground to set it.
6. **Who beats whom:** Riflemen beat Launchers. Launchers beat tanks. Tanks break buildings. A turret beats a lone tank.
7. **Repair:** right-click a damaged vehicle or building with an Engineer. Repair costs credits.
8. **Fight:** right-click to attack. Press A and click the ground to attack-move. H holds, S stops.
9. **Controls:** a list on the `Bindings` source.

## 9. Questions for the CEO, and cuts

**Questions** (my recommendation follows each):
- **Faction:** a frontier mining company's guard force. Working name "Lantern Company"; the match is a mirror match over a claim.
- **Tone:** grounded near-future, light, no gore.
- **Art:** low-poly, top-down silhouettes you can read at a glance, team colour on large surfaces. No glowing crystals.
- **Unit display names:** keep plain function names for MP1.

**The three cheapest cuts:**
1. Drop the lanes: one queue on the CC.
2. Drop idle acquisition: units fire only in weapon range.
3. Drop the settings schema and repair cost: repair is free at 5 HP/s.

## What I ran

| Command | Claim tested | Result that would refute it | Result |
| --- | --- | --- | --- |
| `idprobe.py` over `crates/**/src` | My ids do not collide with engine tokens (`no_content.rs:166-170`) | A HIT in production code | `light` and `shell` collide. `turret` and `ore` hit only in test code. Every id I chose: no hit |
| `balance.py`, then `balance2.py` | The four ratios in part 1 | Equal-cost riflemen beat a tank | The first pass failed. The tuned pass meets all four targets |
| `economy.py` | Trip times, income, and the 3-minute plans | A trip much faster or slower than the numbers above | 32.8 s, 40.8 s, 62.6 s. The map is symmetric (centre fields are the same distance from both CCs) |
| grep for `Attack` and `Fired` | Attack accepts PointOrUnit; no shot event reaches clients | A match in the protocol for `Fired` | Attack accepts PointOrUnit. No `Fired` in the protocol |

**Not checked (NOT TRACED):** node contention between gatherers, avoidance overhead on trips, and client-side event visibility in full. I did not load a package headless, because I wrote no repository file.

**Needs decision (small):** 16.2 orders the steps "pay, then place, then progress" (S:801). The MP1 row asks for charge-over-time payment (S:723). Option A: pay over time while the site builds (my recommendation). Option B: charge when the player places the site.

**What I changed:** none in the repository. Files are in /tmp/claude-1000/-home-julien-Projects-TuRTS/bf418660-b272-4bb3-8628-8d9ef2677d1d/scratchpad:
- loop-walk.md
- balance.py
- balance.out
- balance2.py
- balance2.out
- economy.py
- economy.out
- idprobe.py

**What I read:**
- `.claude/agents/game-designer.md` and `CLAUDE.md` (from the session context).
- The spec: S:28-104, 109-219, 310-356, 414-456, 700-819.
- `mods/forge`, `mods/muster` and `mods/base` (units, orders, maps, `hud.ron`).
- MC design sections 5, 6, 8, 9.1-9.4, 11 and 12.
- `schema.rs`, `orders.rs`, `content_m3_validation.rs:360-425`, `combat.rs:82-401`, `production.rs:100-399`, `gather.rs:300-389`.
