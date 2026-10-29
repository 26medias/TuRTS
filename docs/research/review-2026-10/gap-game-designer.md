VERDICT=fail ITEMS=88 MISSING=35 PARTIAL=30 CONFLICT=1 RUNS=9 NOT_TRACED=4

# Gap audit: game-designer seat

Areas: spec 3, 3.2, 16.2 (Primary); 7, 8, 6.6 settings schema (Second), per
`docs/superpowers/specs/2026-10-25-studio-agent-team-design.md:131,145,151,163`.
Yardsticks: A = `inventory-engine.md` (my Phase 1 file), B = `inventory-player.md`. S = spec v0.7.
Terms searched for each `missing` row: `inventory-engine-greps.txt` and `gap-game-designer-greps.txt` in this
directory. Noise hits are not counted: "mission" = permission/admission, "air" = repair/pair, "tribute" =
distribute, "draw" = draw counts, "rally" = literally (S:366).

Milestone keys used below (proposals, direction only):
- **PC** = a new player-control milestone. It makes the M3 and M4 demos playable (lens answer 3). It goes
  before M4 closes, or M4 K1 grows into it.
- **FP** = a new first-playable milestone for the 16.2 default mod. A person finishes a match against a bot.
  It goes right after PC, before M5. Today 16.2 has no milestone (game-designer-3).
- **later** = after FP. The spec review sets the slot.

| id | item | verdict | evidence (S:line or terms searched) | mechanism/policy | milestone now | milestone proposed | depends on | M1-M3 change? | note |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| game-designer-1 | 3.1 goal states the union bar | missing | S:95 "different combinations of combat, resource collection, ..."; D06 S:29 "broad range of RTS mechanics" (Baseline); "ultimate" hits S:700, S:774 only (other senses) | policy (product goal) | none | spec edit now | none | no | Lens answer 2 gives the sentence. |
| game-designer-2 | Reference games named in 1 | partial | S:9 names StarCraft, Warcraft, C&C, Age of Empires, Factorio; Supreme Commander absent ("Supreme" 0 hits) | policy | none | spec edit now | none | no | Add SupCom; it is the scale reference for D29. |
| game-designer-3 | 16.2 first playable has a milestone | wrong-milestone | S:674-688 has no 16.2 row; S:694 "playable default mod against a conventional bot" sits in an unnumbered "Additional proposed validation" list; M2 S:677 is economy only | policy (plan) | none | FP | PC, A-CO01, game-designer-14..16 | no | The Decided game (D44, D55) has no date. |
| game-designer-4 | Human playability gate per milestone | missing | M1-M3 acceptance is automated (S:676-678); CEO: "could identify no objects" (`vault/Reports/2026-10-28 M3 human playtest repair.md:3`, `:25`); M4 K1/F9 adds one gate (`docs/superpowers/specs/2026-10-28-m4-terrain-design.md:27`) | policy (process) | M4 draft only | every milestone from M4 | PC | no | One sentence in 14: each demo must be finished by a person with mouse and keyboard. |
| game-designer-5 | 16.2 harvester-loss dead end | missing | S:743, S:749: one harvester, credits "enough to establish power"; no rule for a player with no harvester and too few credits; no surrender (A-UI19) | policy | none | FP | A-UI19, A-EC10 | no | Match can stall forever: a CC survives, no income. Needs a rule or a surrender exit. |
| game-designer-6 | 16.2 anti-vehicle soldier has a reason to exist | missing | S:730 roster; rules table S:740-749 has no damage-against-armour row; S:159 types are mod policy only | policy (needs A-CB07 mechanism) | none | FP | A-CB07, A-CB25 | NOT TRACED whether `WeaponDef` has a damage type | Without bonus damage the rifleman and the anti-vehicle soldier are the same unit. |
| game-designer-7 | 16.2 command center queues: one or two | missing | S:744 "Produces units and manages building construction"; S:751 lists no queue rule | policy | none | FP | A-PR01, A-PR04 | no | C&C uses one structure queue and one unit queue. Needs decision. |
| game-designer-8 | 16.2 settings schema entries | missing | S:290 examples (teams, map, starting resources, research speed, victory); S:738-751 lists none for 16.2 | policy (schema per S:290) | none (M9 builds rooms S:684) | FP | A-MD12, A-MF04 | no | Teams, map, start credits, bot slots, fog mode. Each needs type, default, range. |
| game-designer-9 | 16.2 map: player count, start positions, ore per base | missing | S:749 starting assets only; S:332 start positions are mod policy; "start position" in 16.2: 0 hits | policy | none | FP | A-MP01, A-MF06 | no | A 1v1 map and a 2v2 map are the minimum to test team victory (S:748). |
| game-designer-10 | 16.2 fog rule | missing | S:740-749 no visibility row; S:162 sight is mod policy | policy | none | FP | A-FV01, A-FV02, A-UI14 | NOT TRACED whether M1-M3 filter by visibility | C&C has a black shroud. Pick one and put it in the rules table. |
| game-designer-11 | 16.2 ore depletion and regrowth | missing | S:732 "One type collected from ore fields"; no amount, no regrowth rule ("regrow", "spread" 0 hits in the 16.2 sense) | policy | none | FP | A-EC04, A-EC14 | no | Finite ore plus turrets gives a stalemate risk. Proposal with a measured match length. |
| game-designer-12 | Air units | partial | S:113 "Flying units remain a possible RTS capability; ... require a separate design" | both | none | later | A-PF03, A-GM08 | no | Union bar: SC, CNC, WC3, SUP have air. Needs a design slot. |
| game-designer-13 | Naval units and water movement | missing | "naval", "ship" (unit sense), "boat" 0 hits; "water" hits are utilities S:398, S:522 | both | none | later | A-PF03 | no | Union bar: CNC, AOE, WC3, SUP. |
| game-designer-14 | Consume-over-time payment for 16.2 | partial | S:406 names three policies; code has two: `crates/turts_mod_api/src/schema.rs:390-393` (`ChargeOnQueue`, `ReserveThenCharge`) | both | M2 (two of three) | FP | A-PR03 | yes: add a variant, `schema.rs:390` | C&C charges over time. 16.2 must pick one; needs decision. |
| game-designer-15 | Power for 16.2 (balance or network) | wrong-milestone | S:746 power rule (Decided, D55); S:398 network quantity; M6 S:681 is underground utilities; "power" 0 hits in `crates/` and `mods/` | both | M6 (utilities) | FP | A-EC09, A-PR16 | yes: no power code exists | 16.2 needs power before M6. A global balance (C&C) is enough. |
| game-designer-16 | Attack and attack-move orders | wrong-milestone | S:747 Decided rule; code: auto-target only, `crates/turts_sim/src/economy/combat.rs:165-191`; "AttackMove", "Attack" 0 hits in code | both | none | PC (attack), FP (attack-move) | A-OR03, A-OR04, A-OR26 | yes: new command kinds | A Decided rule with no milestone. |
| game-designer-17 | Player construction | wrong-milestone | S:745 Decided rule; M4 puts it out of scope (`2026-10-28-m4-terrain-design.md:22`); "Construct" 0 hits in code | both | none | FP | A-CO01, A-CO02, A-CO03 | yes: no construction code | 16.2 cannot run without it. |
| A-PR01 | Production queue | present | S:142, S:406, S:410 | both | M2 | — | — | no | — |
| A-PR02 / B-37 | Cancel and refund | present | S:406, S:707 (Open policy); code `crates/turts_sim/src/command.rs:44` | both | M2 | — | — | no | 16.2 refund ratio is open (S:751). |
| A-PR03 | Payment policy | present | S:406, S:707 | policy | M2 | — | — | see game-designer-14 | — |
| A-PR04 | Parallel production | partial | S:410 "without assuming identical queue semantics" | both | none | FP | A-PR01 | no | See game-designer-7. |
| A-PR05 | Spawn and blocked exit | partial | S:142, S:406 "output blockage" | mechanism | none | FP | A-PF04 | NOT TRACED | — |
| A-PR06 / B-20 | Rally applied to new units | missing | "rally" 0 hits (S:366 is "literally") | both | none | FP | A-OR15 | no | — |
| A-OR15 | Rally point | missing | as A-PR06 | both | none | FP | A-PR05 | no | — |
| A-PR07 | Prerequisites | present | S:161, S:424 | both | M3 | — | — | no | — |
| A-PR08 | Research | present | S:424-428 | both | M3 | — | — | no | — |
| A-PR09 / B-65 | Ages and tiers, with announcement | missing | "tier", "age", "era" 0 hits (S:9 "Age" is the game name) | policy (on A-PR15) | none | later | A-PR15 | no | — |
| A-PR10 | Unavailable reason | missing | no term; M3 repair added a missing-stock text (`vault/Reports/2026-10-28 M3 human playtest repair.md:5`) | both | none (M3 ad hoc) | PC | A-UI01 | NOT TRACED | Make it a general rule, not one panel. |
| A-PR11 | Repeat production | missing | "repeat" S:428 is research only; "loop", "infinite" 0 hits for production | mechanism | none | later | A-PR01 | no | — |
| A-PR12 | Batch queue | missing | "batch" hits are replication/rendering (S:340, S:566, S:596) | mechanism | none | FP | A-PR01 | no | — |
| A-PR13 | Multi-building production | missing | no term | mechanism | none | later | A-OR24 | no | — |
| A-PR14 / B-22 | Supply block | partial | S:157 "population rules" (mod policy cell) | policy | none | later | A-EC08 | no | 16.2 has no pop cap. |
| A-PR15 | Modifier application | present | S:426, S:428 (stacking Open) | both | M3 | — | — | no | — |
| A-PR16 | Power effect on production | present | S:410, S:746 | both | none | FP | game-designer-15 | yes (no power code) | Power-shortage multiplier is open (S:751). |
| A-PR17 | Asymmetric factions | partial | S:163 "Factions" | policy | none | later | A-MD06 | no | 16.2 has one faction (S:726). |
| A-PR18 | Recipes | present | S:410 | both | M3 | — | — | no | — |
| A-PR19 | Exclusive branches | partial | S:428 Open policy | both | none | later | A-PR07 | no | Do not fix in passing. |
| A-EC01 | Resource types | present | S:394-400 | both | M2 | — | — | no | — |
| A-EC02 / B-33 | Gather cycle | partial | S:140, S:180, S:743 | both | M2 | — | — | NOT TRACED (cycle code in `gather.rs`, not read) | Rule exists in 16.2; cycle behaviour not written in S. |
| A-EC03 | Drop-off choice | partial | S:743 | both | M2 | — | — | no | — |
| A-EC04 | Depletion | partial | geology S:478-480, S:542; surface ore fields not stated | both | M4 (geology) | FP (ore fields) | — | no | See game-designer-11. |
| A-EC05 | Auto-resume and seek | missing | no term ("idle" S:618, S:655 are systems) | mechanism | none | FP | A-EC04 | NOT TRACED | — |
| A-EC06 | Node occupancy | missing | "saturat" 0 hits; "occupancy" hits are placement/belts | both | none | later | A-EC02 | no | — |
| A-EC07 | Generators | partial | S:398, S:522 | both | none | later | A-EC09 | no | — |
| A-EC08 | Population cap | partial | S:157 | policy | none | later | — | no | — |
| A-EC09 / B-23 | Power, and power shown | partial | S:398, S:522, S:746; no power display term | both | M6 | FP | game-designer-15 | yes | Display goes with A-UI04. |
| A-EC10 | Insufficient funds | present | S:406 | policy | M2 | — | — | no | — |
| A-EC11 | Tribute and transfer | missing | "tribute", "gift" 0 hits in this sense; S:428 "team sharing" is research | both | none | later | A-TM02 | no | — |
| A-EC12 | Market | missing | "market", "trade", "exchange" 0 hits | policy | none | later | A-EC01 | no | — |
| A-EC13 | Storage caps | present | S:404 capacity checks | both | M2 | — | — | no | — |
| A-EC14 | Regrowing resources | missing | "regrow", "spread" 0 hits; S:480 forbids regrowth by fill, a different rule | policy | none | FP | A-EC04 | no | See game-designer-11. |
| A-EC15 / B-64 | Upkeep | partial | S:157 | policy | none | later | A-EC08 | no | — |
| A-EC16 | Physical items and logistics | present | S:396, S:412-420 | both | M3 | — | — | no | — |
| A-EC17 / B-63 | Income rate and power graph | missing | no term | mechanism (UI) | none | later | A-UI04 | no | — |
| A-CO03 | Placement rules | present | S:158, S:460, S:470, S:745 | both | M4 (foundations) | — | — | no | — |
| A-CO04 | Construction model | present | S:158, S:745 | policy | none | FP | game-designer-17 | yes | — |
| A-CO07 | Under construction state | partial | S:158, S:745 | both | none | FP | A-CB01 | yes | — |
| A-CO08 / B-37 | Construction cancel and refund | partial | S:406 production only | policy | none | FP | A-CO07 | no | — |
| A-CO11 / B-51 | Repair | partial | S:742 rule; accounting Open S:751 | both | none | FP | A-CB01 | no | Engineer repair is a 16.2 Decided rule. |
| A-CO12 / B-51 | Sell and deconstruct | missing | "sell", "deconstruct", "reclaim", "salvage" 0 hits | both | none | later | A-CO08 | no | M3 `Remove` exists (`vault/M3 Playtest Guide.md:15`); no refund rule. |
| A-CO15 | Upgrade in place | missing | no term | both | none | later | A-PR08 | no | — |
| A-CO22 | Build radius | partial | S:751 "construction restrictions" Open | policy | none | FP | A-CO03 | no | Proposal with a measurement. |
| A-CB01 | Health and death | partial | "health" 0 hits in S; code has it: `crates/turts_sim/src/economy/combat.rs:44,261` | both | M3 (code only) | PC (display) | — | no | Spec must name it; code is ahead of spec. |
| A-CB03 / B-32 | Auto-target | partial | S:747; code `combat.rs:165-191` (nearest in range) | both | M3 | — | — | no | Priority rule (A-CB04) missing. |
| A-CB07 | Damage and armour types | partial | S:159, S:733 | both | none | FP | MD07 | NOT TRACED | See game-designer-6. |
| A-CB21 | Defensive buildings | partial | S:144, S:746 | both | M3 | — | — | no | — |
| A-CB25 | Bonus damage | missing | "damage type" 0 hits; S:159 "damage types" cell only | both | none | FP | A-CB07 | NOT TRACED | — |
| A-MF04 | Game settings | present | S:290 | both | M9 | FP (16.2 entries) | A-MD12 | no | — |
| A-MF06 | Start conditions | present | S:317, S:332, S:749 | policy | none | FP | game-designer-9 | no | — |
| A-MF08 / B-78 | Victory rules | present | S:163, S:290, S:748 | policy | none | FP | — | yes: "victory" 0 hits in code | Simultaneous destruction Open (S:751). |
| A-MF09 | Defeated player observes | missing | S:328 spectating is for non-players; no defeat rule | both | none | FP | A-MF08 | no | — |
| A-MF10 | Team victory | partial | S:748 "player or team" | policy | none | FP | A-TM01 | no | — |
| A-MF11 | In-game diplomacy | partial | S:163 "diplomacy" | policy | none | later | A-TM02 | no | — |
| A-MF12 / B-36 / B-70 | Win and lose screen, score | missing | "defeat", "score", "end screen" 0 hits; S:272 "results" names a room | both | none | FP | A-MF08 | no | A person cannot finish a match without it. |
| A-MF16 | Campaign and scenarios | missing | "campaign", "tutorial" 0 hits; "mission" hits are other words | both | none | later | MP04 | no | — |
| A-MF17 | Handicap | missing | "handicap" 0 hits | policy | none | later | A-MD12 | no | — |
| A-MF18 | Random faction and map | missing | "random" hits are seeds (S:213, S:239, S:492) | policy | none | later | A-MD12 | no | — |
| A-MF19 | Co-op against AI | partial | S:301, S:755 bots in slots; team play not stated | policy | none | FP | A-TM01 | no | — |
| A-MD05 | Tech definitions | present | S:426 | both | M3 | — | — | no | — |
| A-MD06 | Faction definitions | partial | S:163 | policy | none | later | — | no | — |
| A-MD10 | Victory definitions | partial | S:163 | policy | none | FP | A-MF08 | no | — |
| A-MD12 | Settings schema | present | S:290 (Baseline) | both | M9 | FP | — | no | Server validates (S:290). |
| A-TM01 | Teams | partial | S:163, S:290 | both | none | FP | A-MD12 | no | — |
| A-TM02 | Alliance effects | partial | S:162 shared vision only | both | none | FP | A-TM01 | NOT TRACED whether combat skips allies (`combat.rs:182` uses a slot relation) | — |
| A-FV11 | Fog lobby options | missing | "fog" S:162, S:634 only; not in the S:290 example list | policy | none | FP | game-designer-10 | no | — |
| A-UI19 / B-38 | Surrender, leave, pause | missing | "surrender", "concede", "forfeit", "resign" 0 hits; S:303 pause is server policy when all disconnect | both | none | FP | A-MF08 | no | Exit for game-designer-5. |
| B-62 | Machine-problem alerts | missing | S:406 output blockage is accounting only; no alert term | both | none | PC | A-UI16 | no | The M3 playtest needed it (guide `:29` asks for "a factory that stops without an explanation"). |
| B-66 | Tech tree screen | missing | no term | both (UI) | none | later | A-PR07 | no | — |

## Conflicts with the goal (Decided items)

I do not reopen these. I name the conflict with the union bar.

1. **D41 (S:64, S:253):** "no standalone solo mode or local game saves". All six reference games have offline
   skirmish and local save. Server-hosted bot matches are allowed (S:253), so skirmish is possible. Offline play
   and local save are blocked. Status: Decided. Conflict: yes, partial. The CEO decides if "Ultimate" includes
   offline play.
2. **D43 / S:720:** "Do not precommit to a large manual editor suite." SC1, SC2, WC3 and AOE modding grew from
   their editors (A-MP02, CORE for a moddable engine). The statement does not forbid an editor. Tension, not a
   block. Owner: qa-lead (16.1).
3. **D06 (S:29), Baseline:** "a broad range of RTS mechanics". It is not Decided, so it is not a conflict. It is
   weaker than the goal. See game-designer-1.

No other Decided item in section 2 blocks a row above. D28 and S:332 reject engine spawn rules and AI takeover.
These stay possible as mod policy (S:332, S:786), so they do not block the bar.

## Lens answer

**1. Is 16.2 enough to prove a C&C-like game?** Its rules are enough. A person cannot finish a match.

- The roster and rules (S:729-748) cover the C&C core: economy loop, power, production, a defence, repair, and
  victory. I walked the loop below. It has three dead ends and one missing reason.
  - Harvester to ore: rule S:743. Gather code exists (M2). OK.
  - Ore to credits: S:743. OK.
  - Credits to units: the CC produces units (S:744). It has no rally point, so new units stand still (A-OR15).
  - Credits to buildings: S:745 needs construction, and no milestone schedules it (game-designer-17).
  - Power to production and turrets: S:746. There is no power code, and the plan places power in M6
    (game-designer-15).
  - Army to combat: the move, attack and attack-move rule (S:747) has no attack or attack-move command in code
    (game-designer-16). The anti-vehicle soldier has no damage rule (game-designer-6).
  - Combat to victory: S:748 states the rule. There is no end screen (A-MF12) and no surrender (A-UI19). The
    harvester-loss stall never resolves (game-designer-5).
- The first minute has no choice to make. The player cannot select a group (A-PC02), build (game-designer-17),
  or see power (A-EC09).
- **To be a game a person finishes, 16.2 must add these (FP):**
  - Orders: stop, rally, and an order for a selected group.
  - HUD: resource and power bar, build menu with placement ghost, queue display, health bars, and alerts for
    low power, base under attack, construction complete and insufficient funds.
  - Lobby: 16.2 settings entries (game-designer-8), a map with start positions (game-designer-9), bot slots
    (S:301).
  - Victory: a win and lose screen with the reason, surrender, and a defeat rule for "no CC".
- Balance numbers stay Open (S:751). Write each one as a proposal with a measurement.

**2. Does 3.1 state "Ultimate"?** No. "ultimate" appears only at S:700 ("ultimate capacity") and at S:774
("ultimately pass"). S:95 states combinations of systems. D06 (S:29, Baseline) says "broad range". Proposed
direction only, for the CEO to approve:

> "The engine must let a mod reproduce the player-facing mechanics of every reference RTS (StarCraft I and II,
> Command & Conquer to Red Alert 2, Age of Empires II, Warcraft III, Supreme Commander, and Factorio-style
> industry) through the public mod API, and the default game must be playable by a person with mouse and
> keyboard."

**3. The 30 items for the first player-control milestone (PC)** (A ids):

| Group | Items |
| --- | --- |
| Input (5) | IN01 height-aware picking, IN02 action mapping, IN03 modifier semantics, IN05 hotkey per command button, IN08 browser key capture |
| Selection (9) | PC01 single, PC02 box, PC04 shift add/remove, PC08 control groups, PC16 inspect foreign, PC19 selection hygiene, PC20 hover, PC22 smart right-click, PC23 selection visual |
| Orders (6) | OR02 stop, OR20 ability order with point and area target (Dig, Research), OR22 refusal reason, OR23 acknowledgement, OR24 group fan-out, GM01 destination spread |
| Camera (4) | CA02 edge scroll, CA03 key scroll, CA05 zoom, CA15 height-aware camera |
| Interface (6) | UI01 command card, UI02 selection panel, UI03 info panel, UI10 queue display, UI11 tooltips, UI12 health bars |

Why these fit the demos:
- M3 needs inspection, buttons, costs, queue progress, health, and a reason when a machine stops.
- M4 needs box select, group move, an area order for Dig, height picking, and refusal reasons. M4 K1 already
  plans PC02 and OR24 (`2026-10-28-m4-terrain-design.md:264-267`).
- The client today holds one selected id (`crates/turts_client/src/economy_ui.rs:31`). It picks on a flat plane
  (`crates/turts_client/src/input.rs:14-15`). It reads no key ("KeyCode" 0 hits in `crates/turts_client/src`).

**These can wait for FP:** OR15 rally, OR03/OR04 attack and attack-move, CO01/CO02 placement ghost, UI04
resource bar, UI05 minimap and CA08/CA09, UI16 alerts, UI17 chat, UI19 surrender, MF12 end screen, AU01-AU07
audio, OR12 shift queue, PC05 type select, PC09/PC10 group add and centre.

**These can wait for later:** formations, stances, patrol, subgroups, bookmarks, rebinding, colour-blind
support, and everything COMMON or OPTIONAL not named above.

## What I ran

| command | claim tested | result that would refute | observed |
| --- | --- | --- | --- |
| `rtk proxy grep -n -i -o -E` with 20 term groups on S (`gap-game-designer-greps.txt`) | each `missing` row has no spec line (table) | a hit in the item's sense | hits are other senses only (noted in the header) |
| `rtk proxy grep -n -i -o -E 'ultimate\|union\|broad range\|traditional RTS\|playable\|power'` on S | 3.1 does not state the union bar (game-designer-1) | "ultimate" or "union" in 3 | "ultimate" only at S:700 and S:774; "union" 0 |
| `rtk proxy grep -rn` in `crates`, `mods` for victory, health, cancel, rally, settings schema, construction, power, payment, auto-target, attack | code state cited in the M1-M3 change column | a hit that contradicts a "no code" cell | victory 0; construction 0; power 0; attack 0; health `combat.rs:44`; cancel `command.rs:44` |
| `rtk proxy grep -n -A6 'pub enum PaymentPolicy'` | code has two of the three S:406 policies (game-designer-14) | a third variant | `schema.rs:390-393`: two variants |
| `rtk proxy grep -n` on `combat.rs` | auto-target is nearest in range (A-CB03) | a priority or order-driven target | `choose_target` uses distance, `combat.rs:165-191` |
| `rtk proxy grep -n 'struct Selection'`, `ground_point`, `KeyCode::` in `crates/turts_client/src` | the client has one-unit selection, flat picking, no keys (lens 3) | a set type, a heightfield ray, or key handling | `economy_ui.rs:31` Option<u32>; `input.rs:15` plane; KeyCode 0 hits |
| Read M4 draft 1.1, 8, 9 | M4 K1 plans PC02 and OR24 only | hotkeys or groups in scope | `m4-terrain-design.md:23` excludes hotkeys, control groups, minimap, sound |
| Read playtest report and guide | M3 had named Select buttons and one-unit right-click | box or group select in the guide | guide `:9`, `:21`; report `:3`, `:25` |
| Python count of verdicts in this table | the counts on line 1 | a mismatch | see line 1 (filled by the script) |

## NOT TRACED

1. Whether `WeaponDef` has a damage-type or armour field (game-designer-6, A-CB07, A-CB25).
2. Whether M1-M3 filter replication by visibility (game-designer-10).
3. Gather cycle details and auto-resume in `crates/turts_sim/src/economy/gather.rs` (A-EC02, A-EC05). Also
   blocked spawn exit (A-PR05).
4. Whether combat skips allied slots (`combat.rs:182`) and whether A-PR10 is a general rule or one M3 panel.
