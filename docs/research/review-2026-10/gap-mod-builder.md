VERDICT=fail ITEMS=41 MISSING=28 PARTIAL=9 CONFLICT=0 RUNS=43 NOT_TRACED=5

# Gap audit: mod-builder (consumer lens, sections 4.3, 5, 6.5)

Workspace: HEAD 5461486, `ENGINE_API_VERSION = "0.2.1"` (`crates/turts_mod_api/src/lib.rs:33`), ron =0.12.2
(`Cargo.toml`). Probe tool: `target/release/turts-pack`, rebuilt at HEAD with `cargo build --release -p turts_pack`.
Sample mod (a C&C 16.2 package that packs): `.../scratchpad/review/modprobe/cnc/`. Probe driver:
`.../scratchpad/review/modprobe/probe.sh`. Each probe copies `cnc`, changes ONE field, and runs `turts-pack`.
Messages below are quoted as the tool printed them.

## Part 1. Sample mod 1: StarCraft-like skirmish (22 content items)

Score: present = 1, partial = 0.5, missing = 0.

| # | Item | API field (path:line) | Spec promise (S:line) | Grade | Evidence |
| --- | --- | --- | --- | --- | --- |
| SC1 | Faction A (roster, start units, tech) | none; per-slot `Placement` only (schema.rs:432-439) | S:163 "Factions" mod policy | partial | probe `map-factions`: "unknown field `factions` ... (allowed: id, size, placements, nodes, slot_economy, belts)". Faction = fixed by map slot; no player choice. |
| SC2 | Faction B | same | S:163 | partial | same |
| SC3 | Worker gathers | `GathererDef` schema.rs:142-147, `DropOffDef` :162 | S:140, S:743 | present | `mods/base/units/harvester.ron:5`; `cnc` packs |
| SC4 | Worker builds structures | none | S:140, S:158 | missing | probe: "output_unit: output must be mobile and cannot be a producer"; no place command (`turts_net_protocol/src/messages.rs:54-78`) |
| SC5 | Barracks (producer, destructible) | `ProducerDef` schema.rs:167-171 | S:142 | partial | producer present; built only by map placement; cannot take damage (own gap mod-builder-2) |
| SC6 | Ranged infantry | `movement` + `health` + `weapon` schema.rs:332, :354, :356 | S:159, S:747 | partial | probe `weapon-on-mobile`: "weapon: weapons must be static". Unit moves, cannot fight. |
| SC7 | Melee infantry | same | S:159 | partial | same |
| SC8 | Armoured vehicle | same | S:159 | partial | same; no armour (probe `armor-field`) |
| SC9 | Flyer | `MovementDef { speed }` only, schema.rs:131-133 | S:113 (separate design), S:156 "movement categories" | missing | probe `flying-layer`: "unknown field `layer` ... (allowed: speed)" |
| SC10 | Caster (mana) | none | S:160 "mana" | missing | probe `abilities-field` unknown field |
| SC11 | Ability 1: timed self buff | none | S:160 "buffs", S:201 | missing | probe `abilities-field`; `EffectDef` one variant (schema.rs:287-293) |
| SC12 | Ability 2: targeted area damage | none | S:160, S:180 "area damage" | missing | same |
| SC13 | Hero (XP, levels, revive) | none | S:161 "hero levels", S:180 "gaining experience" | missing | probe `hero-xp` unknown field `experience` |
| SC14 | Control groups (and hotkey per button) | none | no hit for "control group" or "hotkey" in spec | missing | probe `hotkey` unknown field; S:164 promises only "UI primitives" |
| SC15 | Attack-move | none; 6 engine commands, messages.rs:54-78 | S:747 (default mod only) | missing | no `Attack`/`AttackMove` variant |
| SC16 | Fog and sight | none | S:162 Visibility | missing | probes `sight-field`, `fog-shroud` unknown field |
| SC17 | Second resource | `ResourceDef` schema.rs:123-128, `NodeDef` :371-376 | S:157, S:396-398 | present | `mods/forge/resources/` has two |
| SC18 | Supply / population cap | none | S:157 "population rules" | missing | probe `supply-cost`: "unknown field `supply` ... (allowed: id, name, output_unit, duration_ms, costs, payment, cancel_refunds)" |
| SC19 | Tech requirement (building unlocks unit) | none on `ProductionDef` (schema.rs:409-417) | S:161, S:426 "unlock ... units, buildings" | missing | probes `production-prereq`, `effect-unlock`: "unknown variant `UnlockProduction` ... (allowed: WeaponModifier)" |
| SC20 | Weapon upgrade | `TechnologyDef` schema.rs:297-306, `WeaponModifier` :288-292 | S:161, S:426 | partial | packs (`tech-ok`); only static weapons, only `Damage` (probe `weaponstat-range`: allowed: Damage) |
| SC21 | Victory: destroy all buildings | none | S:163, S:290 | missing | probe `map-victory`; producers cannot be destroyed |
| SC22 | Starting assets | `MapDef.placements`, `slot_economy` schema.rs:455-464 | S:749 | present | `mods/base/maps/flat.ron` |

Totals: present 3, partial 7, missing 12. Score (3 + 3.5) / 22 = **29.5 %**. Fully present: 3/22 = **14 %**.

## Part 2. Sample mod 2: C&C default mod, spec 16.2 (S:729-749), 17 items

| # | Item (S:line) | API field (path:line) | Grade | Evidence |
| --- | --- | --- | --- | --- |
| CC1 | Rifleman (S:730) | movement, health; weapon blocked | partial | `cnc/units/rifleman.ron` packs only without `weapon`; "weapons must be static" |
| CC2 | Anti-vehicle soldier (S:730) | same; no damage class | partial | probes `armor-field`, `weapon-target-class`: "unknown field `classes` ... (allowed: slots)" |
| CC3 | Engineer (S:730) | movement, health | partial | packs as a unit; repair missing (R1) |
| CC4 | Ore resource (S:731) | `ResourceDef` Item, `NodeDef` | present | `cnc/resources/ore.ron`, `cnc/nodes/ore.ron` |
| CC5 | Harvester (S:732) | `GathererDef`, `InventoryDef` | present | `cnc/units/harvester.ron` |
| CC6 | Tank (S:733) | movement, health; weapon blocked | partial | as CC1 |
| CC7 | Turret (S:734) | static `weapon` + `health` + `removal` | present | `cnc/units/turret.ron` packs |
| CC8 | Command center (S:735) | `producer` + `drop_off` | partial | probe `producer-health`: "health: a unit with health requires removal"; with removal: "removal: a unit with a queue cannot be removed". CC is indestructible. |
| CC9 | Power station (S:736) | none functional | missing | probes `power-field` (unknown field `power`), `network-resource`: "unknown variant `Network` ... (allowed: Balance, Item)" |
| R1 | Engineer repairs (S:742) | none | missing | probe `repair-field` unknown field |
| R2 | Harvest to CC for credits (S:743) | `DropOffEffect::Convert` schema.rs:152 | present | `cnc/units/command_center.ron` |
| R3 | CC produces units, manages construction (S:744) | `ProducerDef` (mobile output only) | partial | units yes; buildings: "output must be mobile and cannot be a producer" |
| R4 | Construction: pay, site, progress (S:745) | none | missing | no place command (messages.rs:54-78); no construction def |
| R5 | Power shortage slows production, disables turrets (S:746) | none; `SetEnabled` is a manual toggle (messages.rs:74-77) | missing | no network quantity |
| R6 | Move, attack, attack-move, auto-target (S:747) | `MoveTo` messages.rs:55; auto-target for static weapons (M3 design:329-341) | partial | no attack or attack-move order |
| R7 | Last command center wins (S:748) | none | missing | probe `map-victory`; CC cannot be destroyed (CC8) |
| R8 | Starting assets (S:749) | `placements`, `slot_economy` | present | `cnc/maps/duel.ron` |

Totals: present 5, partial 7, missing 5. Score (5 + 3.5) / 17 = **50 %**. Fully present: 5/17 = **29 %**.
The packed `cnc` mod is a game with no combat between mobile units, no buildable buildings, and no end.

## Part 3. Gap table (yardstick items a modder touches, plus own gaps)

| id | item | verdict | evidence (S:line or terms searched) | mechanism/policy | milestone now | milestone proposed | depends on | M1-M3 change? | note |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| A-MD01 | Unit definition | partial | `UnitDef` schema.rs:327-359; cost/time in `ProductionDef` :409-417; no prereq, model, icon, voice, hotkey, abilities (probes `hotkey`, `icon-visual`, `gltf-visual`, `sound`); S:179 | both | M1-M3 (stats) | core-loop milestone after M3 | MD04, MD17 | yes: schema.rs (additive fields) | Expose the missing fields through the same `UnitDef`. |
| A-MD02 | Weapon and projectile | partial | `WeaponDef` schema.rs:258-264, hitscan only; static only (probe); S:159 "projectile support" | both | M3 (static), M7 (combat, M1 design:46) | core-loop milestone | mod-builder-1 | yes: weapon validation (M3 design:139) | Allow weapons on mobile units first. |
| A-MD03 | Building definition | partial | footprint `Circle` only (schema.rs:28-30; probe `rect-footprint`: "allowed: Circle"); no power, no placement rules; S:158 | both | none | core-loop milestone | mod-builder-3 | yes: schema.rs | Buildings need a placement path. |
| A-MD04 | Ability definition | missing | terms "abilit", "ability" in schema.rs: 0 fields; S:160 | both | none | after core loop, with MD20 | MD20, mod-builder-4 | no | Engine supplies orders, timers, effects (S:160). |
| A-MD05 | Upgrade and tech | partial | `TechnologyDef` schema.rs:297-306; effects: `WeaponModifier` `Damage` only (:281-293); S:426 | both | M3 | core-loop milestone (unlock effects) | MD20 | yes: `EffectDef` | Add unlock and generic stat modifiers. |
| A-MD06 | Faction definition | missing | probe `map-factions`; S:163 | policy (+ lobby mechanism) | none | M9 (lobby choice) | MD12 | no | Map slot placement is the only proxy. |
| A-MD07 | Damage and armour tables | missing | probe `armor-field`; `TargetFilter` has `slots` only (schema.rs:245-247); S:159 | both | none | core-loop milestone | MD02 | yes: schema.rs | 16.2 anti-vehicle soldier needs it. |
| A-MD08 | UI definitions | missing | no UI type in schema.rs; S:164, S:179 "UI data" | both | none | core-loop milestone (command card) | A-UI items | NOT TRACED (client UI) | No mod-visible UI surface. |
| A-MD09 | Audio definitions | missing | probe `sound`: "unknown field `sound` ... (allowed: target, shape)"; S:122 only names audio | both | none | M13 | MD17 | no | Spec has no mod audio promise. |
| A-MD10 | Victory definitions | missing | probe `map-victory`; S:163, S:748 | both | none (M2 design:47 excludes) | core-loop milestone | MD14 or declarative victory, mod-builder-2 | yes: producers must be destructible | |
| A-MD11 | AI definitions | missing | no AI type in schema.rs; S:759 Decided | both | none | M12 | MD14 | no | |
| A-MD12 | Lobby settings schema | missing | no settings type in schema.rs; S:290 Baseline | both | none | M9 | none | no | |
| A-MD13 | String tables | missing | terms "locali", "string table", "translation" in spec: only terrain "localized" hits (S:466, S:556); `name: String` single text (schema.rs:329) | mechanism | none | M8 | none | no | |
| A-MD14 | Event scripting API | missing | Exports has no scripts (probe `scripts-export`: allowed list has no `scripts`); S:181, S:201, D34 Decided, language Open S:193; M2 design:4 "no script layer" | both | none | before victory/abilities | D34 language choice | no | Biggest extension gap after combat. |
| A-MD15 | Fast iteration | partial | `turts-pack` reports in under 1 s per probe (43 runs); hot reload NOT TRACED | mechanism | M1 | M8 | none | no | |
| A-MD16 | Data inheritance | missing | terms "inherit", "override": S:197 "override semantics remain open"; no parent field | mechanism | none | M8 | MD19 | no | |
| A-MD17 | Asset pipeline | missing | `Shape` = `Capsule`, `Cuboid` (schema.rs:48-51); probe `gltf-visual`: "allowed: Capsule, Cuboid"; S:225-229 Baseline | mechanism | none | M13 | none | no | |
| A-MD18 | Validation messages | present | every probe names file, field, cause, and the allowed list (Part 1, 2) | mechanism | M1-M3 | n/a | none | no | Sharp edges: mod-builder-5, -7. |
| A-MD19 | Mod layering | missing | probe `dependency-base`: "dependencies: dependencies are not supported at M1"; S:207 D33 Decided | mechanism | M8 | M8 | MD16 | yes: dependency rejection | |
| A-MD20 | Effect system | missing | `EffectDef` one variant (schema.rs:287-293); S:160 | both | none | after core loop | MD04 | yes: `EffectDef` | |
| B-A3-1 | Hotkeys | missing | probe `hotkey` | both | none | core-loop milestone | MD08 | NOT TRACED | |
| B-A3-2 | Command card layout | missing | no UI type; S:164 | both | none | core-loop milestone | MD08 | NOT TRACED | |
| B-A3-3 | Unit cost and build time | present | `ProductionDef` schema.rs:409-417 | policy | M2 | n/a | none | no | Only for mobile outputs. |
| B-A3-4 | Selection flags and priority | missing | no field in `UnitDef` | both | none | core-loop milestone | A-PC items | NOT TRACED | |
| B-A3-5 | Resource gathering | present | schema.rs:142-164, :371-376 | both | M2 | n/a | none | no | |
| B-A3-6 | Power or supply | missing | probes `power-field`, `supply-cost`, `network-resource`; S:398, S:157 | both | M6 (networks) | core-loop milestone for 16.2 power | none | yes: `ResourceKind` (schema.rs:117-120) | 16.2 power is a match-level balance, not an M6 utility network. |
| B-A3-7 | Fog and sight | missing | probes `sight-field`, `fog-shroud`; S:162 | both | M5 (M1 design:46) | core-loop milestone | none | no | |
| B-A3-8 | Alerts and voice | missing | probe `sound` | both | none | M13 | MD09 | no | |
| B-A3-9 | Win condition | missing | as A-MD10 | both | none | core-loop milestone | A-MD10 | yes | |
| B-A3-10 | UI layout | missing | as A-MD08 | both | none | later | MD08 | NOT TRACED | |
| mod-builder-1 | Armed mobile units | missing | probe `weapon-on-mobile`: "weapon: weapons must be static"; M3 design:139 "A weapon is static." | mechanism | M7 | core-loop milestone | none | yes: weapon validation and targeting system (code path NOT TRACED by rule) | Blocks 10 of 39 sample-mod items (lens answer). |
| mod-builder-2 | Destructible producers | missing | probes: "a unit with health requires removal" + "a unit with a queue cannot be removed" | mechanism | M3 rule | core-loop milestone | none | yes: removal validation | CC and barracks cannot die, so S:748 victory cannot happen. |
| mod-builder-3 | Building placement / construction | missing | probe: "output_unit: output must be mobile and cannot be a producer"; M2 design:46, M4 design:22 exclude | both | none | core-loop milestone | MD03 | yes: production validation | No path, not even instant, to make a building in play. |
| mod-builder-4 | Mod-declared orders | missing | 6 fixed variants, messages.rs:54-78; S:160 | mechanism | none | core-loop milestone | none | yes: protocol | Attack, attack-move, stop, repair, build, ability all absent. |
| mod-builder-5 | Capability names checked | partial | probe `unknown-capability` (`["scripts.server"]`) exit 0; S:209 Baseline, S:708 names Open | mechanism | M1 | M8 | S:708 decision | yes: manifest check | Unknown capability is accepted in silence. |
| mod-builder-6 | Library must not activate gameplay | partial | probe `library-kind`: a `Library` with units and a map packs, exit 0; S:207 Decided | mechanism | M1 | M8 | MD19 | yes: kind check | Server behaviour with it NOT TRACED. |
| mod-builder-7 | Dependency diagnostic text | partial | message says "not supported at M1" at M3 HEAD | mechanism | M1 | M8 | MD19 | yes: message | Stale milestone in a user message. |
| mod-builder-8 | A milestone owns 16.2 (D44, D55 Decided) | wrong-milestone | S:670-688: no row; S:694 "playable default mod" is "additional proposed"; `mods/base/manifest.ron:8` has 3 units | policy (default mod) | none | the next milestone after M3 | mod-builder-1..4 | no | M4 to M6 build terrain, geology, utilities before the Decided first game can fight or end. |
| mod-builder-9 | Movement categories (air, naval) | missing | probe `flying-layer`; S:156 Baseline table; S:113 | mechanism | none | after core loop | none | yes: schema.rs | |
| mod-builder-10 | Teams in targeting and match | missing | `SlotRelation` Other/Same/Any (schema.rs:237-241); probe `map-teams`; S:163 | mechanism | none | M9 | none | yes: `TargetFilter` | Allies attack allies under `Other`. |
| mod-builder-11 | In-tree id names free of engine tokens | partial | `mods/forge/NAMES.md:3-5, :14, :21`: ids `inserter`, `range` renamed by the no-content scan | mechanism (test) | M3 | M8 | none | no | The default mod cannot pick names a third party can. Low. |

Counts: present 3, partial 9, missing 28, wrong-milestone 1, conflict 0. Total 41.

## Conflicts with the goal (Decided items)

No Decided item in section 2 blocks the goal for a modder. One tension: S:183 (no status label) says a mod
must not run a script per unit per frame. Abilities, buffs, heroes, and power must then be engine
composition, so the engine must ship a large composition library (S:180). That is scope, not a conflict.
The blockers are M3 design rules (weapon static, producer not removable, production output mobile),
not spec decisions.

## Lens answer

- StarCraft-like: 29.5 % by weighted score, 14 % fully present (3 of 22).
- C&C 16.2: 50 % by weighted score, 29 % fully present (5 of 17).
- The single missing API that blocks the most: **combat on mobile units** (weapon on a moving unit plus
  the attack and attack-move orders). It blocks 10 of 39 items: SC6, SC7, SC8, SC15, SC20 (partly), CC1,
  CC2, CC6, R6, and by extension any army. Second: construction (6 items). Third: script/event hooks (5).
- Engine code: yes. Every missing item needs a new field or variant in `turts_mod_api` (all types are
  `deny_unknown_fields`, e.g. schema.rs:326) and a new simulation system. No script layer exists (D34;
  language Open, S:193). A third party must change engine Rust, which 4.3 and 5.2 assign to the engine.
  The default mod has no privilege: `mods/base` is blocked in the same way and has only 3 units.

## What I ran

| command | claim tested | result that would refute | observed |
| --- | --- | --- | --- |
| `turts-pack cnc` (4 runs, edit after each) | a 16.2 package fits the public schema | exit 0 on run 1 | 3 rejections (building output, producer removal, static weapon), then exit 0 |
| `probe.sh` x 33 (one field each) | each missing item has no API path | exit 0 for a probe | 28 rejections; 5 exit 0: 3 controls (`base-ok`, `weapon-stat-range` no-op edit, `tech-ok`) and 2 real gaps (`unknown-capability`, `library-kind`). The controls prove a probe can go green. |
| `grep` 21 terms in spec | spec promise per item | hit/no hit as listed | "control group", "hotkey": 0 hits; others cited |
| `grep` command variants in `turts_net_protocol` | order set is closed | an attack or build variant | 6 variants, messages.rs:54-78 |
| `grep` M2, M3, M4 design | milestone owner per gap | a milestone includes the item | M2 design:46-48 and M4:22 exclude construction, combat, victory, power, fog |
| `cmp` wanted vs packed units | probe state matches report | files equal | 3 differ (weapon removed) |

## NOT TRACED

1. Server boot and play of `cnc.tar`: only `turts-pack` (which runs `load_package`) ran.
2. Validation code paths in `crates/` for the three M3 rules: not read (seat rule); cited by message and M3 design.
3. Server behaviour with a `Library` package that holds a map.
4. Hot reload.
5. Client UI support for control groups, hotkeys, selection (brief cites `economy_ui.rs:31`; I did not check).
