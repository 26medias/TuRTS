VERDICT=fail ITEMS=35 MISSING=11 PARTIAL=19 CONFLICT=0 RUNS=10 NOT_TRACED=4

Seat: ai-engineer. Areas: AI opponents, neutral creeps and shops, bot difficulty, the command and observation interface a bot needs.
S = `RTS-Engine-Project-Specifications.md` v0.7. Other verdicts: present 3, wrong-milestone 2.
Raw run output: `/tmp/claude-1000/-home-julien-Projects-TuRTS/0e0457bb-045c-4b6a-945f-1db6a0753ee2/scratchpad/review/ai-engineer/terms.out`.

| id | item | verdict | evidence (S:line or terms searched) | mechanism/policy | milestone now | milestone proposed | depends on | M1-M3 change? | note |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| A-BT01 | Skirmish AI | wrong-milestone | Architecture Decided S:755-776. M12 acceptance names only "high-rate AI commands" (S:687). "playable default mod against a conventional bot" is "Additional proposed validation" with no milestone (S:694). | both | M12 (scale only) | New "bot-playable" milestone right after the player-control milestone and the 16.2 mod | ai-engineer-1, -3, -6, -12, -19, -20 | no | The behaviour bot is not acceptance evidence of any milestone. |
| A-BT02 | Difficulty levels | partial | S:776, S:780 "difficulty design ... remain open" | policy (engine hook: ai-engineer-18) | none | bot-playable milestone, one profile first (S:782) | A-BT01 | no | Open item. Keep it open; one balanced profile first. |
| A-BT03 | Same command path; cheats explicit | present | S:761, S:774, S:780 (Baseline) | mechanism | M1 (command validation exists) | n/a | player command set | no | Principle is present. The path is too thin: see ai-engineer-1 to -5. |
| A-BT04 | Ally AI cooperation (pings, requests) | missing | "ping" 0 hits, "chat" 0 hits (terms.out) | both | none | after a ping/chat item exists (UI owner) | UI07 ping | no | OPTIONAL. Needs a slot-scoped ping event a bot can read. |
| A-BT05 | Unit micro AI (retreat, focus fire, autocast) | partial | S:771 "retreat" in tactical state machines; "autocast" 0 hits | policy | none | after abilities (OR20, OR21) | ai-engineer-2 | no | OPTIONAL. Retreat needs a move that ignores enemies. |
| A-MD11 | AI definitions (roles, priorities, build orders) | present | S:759 Decided, S:165 | policy | none | bot-playable milestone | ai-engineer-12 | no | Format of the mod data is not stated; depends on bot runtime. |
| A-MF05 | AI in slots | present | S:301 Decided, S:780, S:786 | both | M9 (rooms) implied | bot-playable milestone may use server config before M9 | ai-engineer-14 | no | Creator selects mod bots for unclaimed lobby slots (Decided). |
| A-MF14 | AI takeover on disconnect | partial | S:332 rejects automatic takeover as engine rule; S:786 temporary control is mod policy | both | none | M10 (slots) | delegation of control (PC18) | no | Policy is Decided. No mechanism lets a mod bot command a human slot's units. |
| A-MF19 | Co-op vs AI | partial | Teams word only: S:163, S:290, S:748 | both | none | bot-playable milestone | TM01, TM02 | NOT TRACED | Code `SlotRelation` is `Other/Same/Any` (crates/turts_mod_api/src/schema.rs:237-241): no ally relation. |
| A-UN14 | Neutral creeps and critters | missing | "neutral" 0, "creep" 0 (terms.out) | both | none | after ai-engineer-16 | ai-engineer-16 | yes, `SlotRelation` (schema.rs:237) | Needs an owner that is not a player slot, plus leash and camp behaviour as mod policy. |
| A-UN15 | Capturable neutral objects | missing | "neutral" 0, "capture" 0 | both | none | after ai-engineer-16, -17 | ai-engineer-16, -17 | NOT TRACED | Ownership change of an entity is not stated. |
| B-49 | Neutral creeps and shops | missing | "neutral", "creep", "shop", "merchant", "market", "trade" 0 hits (terms.out) | both | none | after ai-engineer-16, -17 and hero inventory | ai-engineer-16, -17, hero items (B-42) | NOT TRACED | CORE for a WC3-like mod. |
| ai-engineer-1 | Attack-target and attack-move orders for any slot | partial | S:747 (first-mod rule), S:160 "Orders, targeting". Code `CommandKind` has MoveTo, Gather, Produce, CancelProduction, Remove, SetEnabled only (crates/turts_sim/src/command.rs:30-56; crates/turts_net_protocol/src/messages.rs:54-79) | mechanism; targeting rules policy | none (M3 design:56 puts combat at scale in M7) | player-control milestone, before any bot | CB01, CB03 (M3 has health and turret auto-target, combat.rs:74-261) | yes, new command variants (command.rs:30) | A bot cannot "attack" or "defend" (S:770, S:776) with today's command set. |
| ai-engineer-2 | Stop, hold position, force move | missing | "stop" 0, "hold position" 0 (terms.out); S:771 needs "retreat" | mechanism | none | player-control milestone | OR11 order queue | yes (command.rs:30) | Retreat without force move gets units killed by auto-target. |
| ai-engineer-3 | Place structure / construction order | partial | S:158, S:745 "select a valid site". Excluded: M2 design:46, M3 design:51, M4 design:22. No row in S:674-688 names construction (run 2) | both | none at all | player-control or 16.2 milestone | CO03 placement rules, footprints | yes (command.rs:30) | Economy/construction controller (S:768) has no order to issue. |
| ai-engineer-4 | Placement validity query (dry run, same rule as the ghost) | missing | S:436 only says rendering must not hide rules; "valid site" S:745 is a rule, not a query | mechanism | none | with ai-engineer-3 | ai-engineer-3 | no | Bot site search and the player's green/red ghost need one shared query. |
| ai-engineer-5 | Repair order | partial | S:742 engineer repairs; "repair" also S:515, S:751; no command in code (command.rs:30-56) | both | none | 16.2 milestone | ai-engineer-3, CB01 | yes (command.rs:30) | Repair accounting is open (S:751). |
| ai-engineer-6 | Slot-scoped event catalogue: unit destroyed, unit attacked, construction done, production done, research done, low power, unit idle | partial | S:201 lists production, research, discovery, lifecycle only; S:159 "damage events"; S:338, S:634 "alerts". Code has `EconomyEventKind::Destroyed`, `Fired`, `ProductionCompleted` (crates/turts_sim/src/economy/events.rs:27-90), server-internal; no event type in `crates/turts_net_protocol/src/*.rs` (run 10) | mechanism | none | player-control milestone (alerts) | FV04 for "attacked by" | yes, wire protocol | Recovery (S:774) needs "harvester destroyed". Players need the same events as alerts (UI16). |
| ai-engineer-7 | Unit activity state query (idle, busy, current task) | missing | "idle" hits only factories S:618, S:655. Code has `GatherStage::Idle` for gatherers only (crates/turts_sim/src/economy/gather.rs:34) | mechanism | none | player-control milestone | OR11 | NOT TRACED | Bot "replace losses, assign units" (S:768, S:770) and the idle-worker button use one query. |
| ai-engineer-8 | Order queue and current-order observation for own units | missing | "order queue" 0, "queued order" 0 (terms.out); S:171 "orders" in an example only | mechanism | none | player-control milestone | OR11 | NOT TRACED | Pending-task inspection (S:782) needs it. |
| ai-engineer-9 | Observation interface equal to the slot's permitted view | partial | S:780 "Detailed observation/memory API ... open"; S:338; S:342 "controlled observation" | mechanism | none | bot-playable milestone | FV04; per-slot filter `export::delta_for(.., SlotId)` used in crates/turts_server/tests/export_m3.rs:372 | no | Needs decision: in-process bot reads the same per-slot filter as the wire, or connects as a client (S:755). |
| ai-engineer-10 | Remembered sightings for humans too | partial | S:780 states memory for bots only; FV10 last-seen for players has no line (inventory-engine.md:580) | mechanism | none | M5 (fog) | FV02, FV04 | no | If only bots get memory, the bot knows more than a human in its slot. |
| ai-engineer-11 | Map strategic metadata: candidate start locations, resource sites, expansion points | missing | "start location/position" hits S:50, S:332 only; S:332 rejects mandatory start positions as engine rules; "metadata" hits are not map fields (S:169, S:209, S:219, S:225, S:270, S:504) | both (schema mechanism, values policy) | none | bot-playable milestone | MP05 | NOT TRACED | Scouting (S:770, S:776) needs a list of where to look. Must not contradict S:332. |
| ai-engineer-12 | Bot runtime: where controller code runs, language, per-decision budget | partial | S:761 "bots run on the authoritative server"; scripting language Open (D34 S:57, S:705); packages use the portable runtime only (S:280) | mechanism | none | decide before bot-playable milestone | scripting decision (S:705) or engine-shipped data-driven bot library | no | Needs decision: Rust bot library driven by mod data, or sandboxed script. A downloaded package cannot be native Rust (S:280). |
| ai-engineer-13 | Bot registration, named profiles, bot package kind | partial | S:780 "bot registration remain open"; package kind values Open (S:708) | both | none | bot-playable milestone | ai-engineer-12 | no | Open. |
| ai-engineer-14 | Bot participant identity and slot state | partial | S:786 "implementation details"; claim needs cryptographic identity (S:328, S:382); "Claimed: owned by one player" (S:324) | mechanism | none | M9/M10 | ai-engineer-12 | NOT TRACED | Needs decision: which slot state a bot-held slot takes and whether it is terminal (S:326). |
| ai-engineer-15 | Per-slot command budget, the same rule for bots and humans | partial | S:342 Proposed, values open; S:687 "fair technical admission". Code: only `MAX_UNITS_PER_COMMAND = 256` (crates/turts_sim/src/command.rs:19); no per-tick per-slot limit in turts_sim or turts_server (run 6) | mechanism | M12 | value open; mechanism with the player-control milestone | OR31 | yes, add a per-slot limit | D29 makes high AI command rates a target workload. |
| ai-engineer-16 | Neutral participant: entities owned by no player slot | missing | "neutral" 0 hits; slot states S:319-324 have no neutral owner; `SlotRelation` Other/Same/Any (schema.rs:237-241) | mechanism | none | before UN14, B-49 | slot model S:317 | yes (schema.rs:237) | S:317 allows slots with no entities; it does not give entities with no slot. |
| ai-engineer-17 | Orders that target a foreign-owned entity (buy from shop, capture, garrison in ally) | missing | No spec line on target ownership; code rejects by actor ownership `NotOwner` (messages.rs:99) | both | none | after ai-engineer-16 | ai-engineer-16, TM02 | NOT TRACED | Transaction rules for a purchase between owners are not stated (S:404 covers stock, not owners). |
| ai-engineer-18 | Difficulty and explicit advantage hooks (per-slot income multiplier, reaction delay, command cap) | partial | S:780 "special knowledge or economic advantages must be explicit mod policy"; S:161 modifiers; settings schema is room-level (S:290) | both | none | bot-playable milestone | S:161 modifiers, ai-engineer-15 | no | Mod needs per-slot settings to express "hard bot gets +20 % income". |
| ai-engineer-19 | Victory, defeat, and match-end event | partial | S:748 rule, S:163; "victory" 0 hits in code (run 5); excluded M2 design:46, M3 design:55 | both | none | 16.2 milestone | ai-engineer-6 | yes, new event | Bot and headless scenario both need an end condition. |
| ai-engineer-20 | Power supply/demand observation | partial | S:746 rule; S:398 network quantity; excluded M3 design:51 | both | none | 16.2 milestone | M6 network or a simple per-slot power model | NOT TRACED | First-mod bot rule "maintain power" (S:776) cannot run. |
| ai-engineer-21 | Headless bot scenarios, decision traces, rejected-command reasons | partial | S:782 Proposed. Reject reasons exist in code (messages.rs:92-122) | mechanism | none | bot-playable milestone | ai-engineer-12 | no | Rejection reasons are built; traces and scenarios are not. |
| ai-engineer-22 | Playable default mod (16.2) as a milestone | wrong-milestone | No row in S:674-688 names 16.2, construction, or victory (run 2); only S:694 "Additional proposed validation" | both | none | between M3 and M5 | ai-engineer-1, -3, -19, -20 | no | Lens answer. |
| ai-engineer-23 | Seeded randomness for bot decisions | partial | S:239 repeatable server tests useful | mechanism | none | bot-playable milestone | ai-engineer-12 | no | Needed for red-before-green bot scenarios. |

## Conflicts with the goal

No Decided item in section 2 blocks the goal for my areas. CONFLICT=0.

- D25, D28, S:332 reject automatic AI takeover as an engine rule. S:786 keeps temporary control as mod policy. No conflict, but the delegation mechanism is missing (A-MF14).
- S:324 "Claimed: owned by one player" and S:328 "authenticates identity" against S:301 bots in slots. Not a conflict with the goal. Needs decision on bot slot state (ai-engineer-14).
- D44 one faction is a first-mod scope only (S:751). No conflict.

## Lens answer

Orders, queries, and events a bot needs that no spec section defines for players either:

- Orders: attack target, attack-move as a player command (only a first-mod rule, S:747); stop, hold, force move (0 hits); place structure (S:745 rule, no order, no milestone); repair (S:742 rule, no order); orders on foreign entities (0 hits).
- Queries: placement validity dry run; unit activity and idle state; own order queue; map strategic metadata (start candidates, resource sites); power supply and demand; per-slot remembered sightings for humans.
- Events: a slot-scoped catalogue with unit destroyed, unit attacked, construction done, low power, idle, match end. S:201 lists only production, research, discovery, lifecycle. The code has `Destroyed` and `Fired` (events.rs:27-90), but no event type is on the wire.
- Owners: a neutral non-slot owner for creeps and shops (0 hits).

Section 17 assumed these as "ordinary command validation" (S:780). The player side never defined them. So the bot design rests on a player command set that M1 to M3 built as six commands (command.rs:30-56).

CORE player items that M12 needs first: OR03 attack, OR04 attack-move, OR02 and OR05 stop and force move, OR16 build with CO03 placement, OR24 group orders (built for MoveTo only, command.rs:30-34), OR22 reject reasons (built), CB01 health and death (built in M3), CB03 auto-target (turret only), UI16 alerts as slot events, MF08 victory, MF05 bot in slot, OR31 per-slot command budget. FV02 fog is NOT a prerequisite: with no fog a human sees all, so a bot that reads the same per-slot filter stays fair.

Does M12 come too late? Yes. M12 joins two jobs: worst-case scale and a behaviour bot. The scale job fits late. The behaviour bot does not. It depends on none of M4 terrain, M6 utilities, M8 index, M10 slots, M11 spectators. S:755 says bots exist "for playable games and manual testing"; every milestone after M3 loses that test partner. The 16.2 playable mod has no milestone at all (S:694). Direction: add a player-control milestone, then a 16.2 milestone with a bot-playable acceptance (bot vs idle opponent, replaced harvester, commands per tick logged), before M5. Keep high-rate AI load in M12. The bot runtime decision (ai-engineer-12) gates this and depends on the open scripting choice (S:705).

## What I ran

| command | claim tested | result that would refute | observed |
| --- | --- | --- | --- |
| `python3 -I ai-engineer/terms.py` (36 term groups over S) | Neutral, creep, shop, stop, hold, idle, health, ping, chat, order queue are absent from S | Any hit in the right sense | 0 hits for neutral, creep, shop, handicap, stop, hold, rally, health, ping, chat, order queue; idle only S:618, S:655 (factories) |
| Python scan of S:674-694 for construct, 16.2, default mod, bot, AI, victory | No milestone demonstrates construction, 16.2, victory, or a behaviour bot | A milestone row with one of these | construct, 16.2, victory: none; "bot" only S:694 ("both" false hit at S:682 discarded); AI only S:687 |
| `rtk proxy grep` on `crates/turts_sim/src/command.rs:30` and `messages.rs:54` | Player command set lacks attack, place, stop, repair | A variant with these names | Six variants only |
| `rtk proxy grep -c Attack|Place|Build|Stop|Repair` on command.rs, command_m3.rs, messages.rs | Same | Any variant hit | One hit, `MatchStopped` (command.rs:199), not a command |
| `rtk proxy grep -c visib|fog|sight` over crates | No fog in turts_sim or turts_server | Any hit in those crates | 0 in turts_sim and turts_server |
| `rtk proxy grep -c victory|defeat|winner` over crates | No victory code | Any hit | 0 |
| `rtk proxy grep` for per-tick or rate limit in turts_sim, turts_server | No per-slot command budget | A limit per slot per tick | Only `MAX_UNITS_PER_COMMAND = 256` (command.rs:19) |
| `rtk proxy grep` bots in milestone designs | Bots are deferred in every milestone | A milestone design that includes a bot | M1 design:47 "bots (M12)"; M2 design:48 "bot spending" excluded; M3 design:55 "bots" excluded; M4 none |
| `rtk proxy grep enum SlotRelation` | No neutral or ally relation | A Neutral or Ally variant | `Other, Same, Any` (schema.rs:237-241) |
| `rtk proxy grep EconomyEvent|events` in turts_net_protocol | Events do not reach the wire | Any event type | 0 hits |

## NOT TRACED

1. Whether removed unit ids reach clients through `CommittedTick` deltas (events.rs:117 says publication drains `RemovedThisTick`). A client bot may infer deaths from removals.
2. Whether the M3 client UI exposes any order other than MoveTo, Gather, Produce, Remove, SetEnabled.
3. The M7 milestone content: no M7 design exists in `docs/superpowers/specs/`, so attack orders may be planned there.
4. Whether any per-slot command limit exists in the lightyear layer (`turts_net_lightyear`); I searched turts_sim and turts_server only.

What I changed in the repository: none.
