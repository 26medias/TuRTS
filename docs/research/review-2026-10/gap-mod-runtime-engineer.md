VERDICT=fail ITEMS=70 MISSING=27 PARTIAL=39 CONFLICT=0 RUNS=9 NOT_TRACED=4

# Gap audit: mod-runtime-engineer (4.3, 5.1 to 5.5, 5.7, 6.5)

Seat: mod-runtime-engineer. Mode: review. What I changed in the repository: none.
"S:n" = `RTS-Engine-Project-Specifications.md` line n (v0.7). "P-xx" = a case in the schema probe
(output: `probe-modrt/probe-output.txt` in this directory). "playable" = a proposed new milestone
that ships the 16.2 default mod with player controls (S:724 to S:751). No milestone holds 16.2 today
(M2 design `docs/superpowers/specs/2026-10-26-m2-economy-design.md:598-599`).

Verdict counts (Python count of the table rows): present 4, partial 39, missing 27, conflict 0.
Five rows also carry wrong-milestone in "note" (MD11, MD14, MD17, BT01, mre-1).

| id | item | verdict | evidence (S:line or terms searched) | mechanism/policy | milestone now | milestone proposed | depends on | M1-M3 change? | note |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| A-MD01 | Unit definition | partial | S:179 word list only. `crates/turts_mod_api/src/schema.rs:327-359`: id, name, footprint, 15 optional capability blocks. P-U: `hotkey`, `icon`, `sight`, `abilities`, `orders`, `cost`, `prerequisites`, `armor`, `sounds`, `cursor`, `selection_priority` all REJECTED (deny_unknown_fields) | both | M1-M3 (partial) | playable | MD20, PR07 | Y: `UnitDef` grows; engine API version bump | Cost is on `ProductionDef` (schema.rs:409-417), not on the unit. |
| A-MD02 | Weapon and projectile definition | partial | S:159 table cell. schema.rs:257-264: range, cooldown, damage, ammo, slot filter. P-W: `splash`, `damage_type`, `min_range`, `projectile`, `targets_air` REJECTED | both | M3 | playable | MD07, CB10 | Y: `WeaponDef` | `TargetFilter` has only slot relation (schema.rs:237-247). |
| A-MD03 | Building definition | partial | S:179. Footprint is `Circle` only (schema.rs:28-30). No placement rule, no construction block, no power | both | none | playable | CO03, M4 terrain slope | Y: `Footprint` enum | 16.2 construction (S:745) has no schema. |
| A-MD04 | Ability definition | missing | No schema type. S:160 is one table row. Terms: ability, autocast (0 hits), mana (hits are "manage"/"management") | both | none | after MD20, playable | MD20, script runtime for custom effects | N | Needs D (common shapes) plus S (custom). |
| A-MD05 | Upgrade and tech definition | partial | S:424-428 Baseline. schema.rs:296-306. `EffectDef` has one variant: `WeaponModifier{stat: Damage, add}` (schema.rs:281-293) | both | M3 | grow at playable | MD20 | Y: `EffectDef` | S:426 lists unlock units, buildings, abilities, detectors: none expressible. |
| A-MD06 | Faction definition | missing | S:163 word "Factions". P6 `exports.factions` REJECTED | policy | none | playable (one faction) | MD12, lobby | N | |
| A-MD07 | Damage and armour tables | missing | S:159 word only. No schema type | both | none | playable | MD02 | N | 16.2 has an "anti-vehicle soldier" (S:730): needs bonus damage or armour type. |
| A-MD08 | UI definitions | partial | S:179 "UI data"; S:164 Decided: mod owns command panels, tooltips, screens. P5 `exports.ui` REJECTED. Client builds panels in engine code, English labels: `crates/turts_client/src/economy_ui.rs:130`, `:319-321`, `:337` | both | none | playable | UI language decision (own gap mre-8) | Y: `economy_ui.rs` must read mod data | needs decision: data layout vs client scripts vs engine default UI with mod data slots. |
| A-MD09 | Audio definitions | missing | "audio" only at S:122 (module cell). sound, voice, music: 0 hits | both | none | playable | AU01 engine playback | N | |
| A-MD10 | Victory definitions | partial | S:163 policy; S:748 Decided rule. No schema (P-M `victory` REJECTED). Code: `rtk proxy grep -i 'victory|defeat|winner|game_over|match_end' crates` = 0 hits | policy + engine events | none | playable | mre-5 events, mre-6 conditions, CB01 death | N (new) | 16.2 rule fits a declarative primitive. Union bar (regicide, wonder, relics, score, time) needs a script or a large primitive set. |
| A-MD11 | AI definitions | partial | S:759 Decided: libraries supply machinery, mods supply knowledge. S:207: libraries use the script runtime. Dependencies rejected in code: `crates/turts_mod_runtime/src/manifest.rs:44-49` (P1) | both | M12 (S:687) | playable | script runtime, mre-2, observation API (S:780 Open) | Y: manifest dependencies | wrong-milestone: S:694 wants the playable mod against a bot; S:755 wants bots for manual testing. |
| A-MD12 | Lobby settings schema | partial | S:290 Baseline (types, defaults, ranges, validation). No schema type. P-M `players` REJECTED | both | M9 (S:684) | M9; minimum at playable | MF04, mre-6 | N | Victory and team options (S:290) cannot be expressed until it exists. |
| A-MD13 | String tables | missing | Terms locali, translat, string table: hits S:37, 86, 466, 556, 566 are all "localized" (spatial). `name: String` (schema.rs:126, 329) | both | none | before MD08 | none | Y: name fields become keys or gain a table | Also yardstick UI30. |
| A-MD14 | Event scripting API | partial | S:201 names four events; S:163 "event hooks". M2 design:469-475: typed event batch is a seam; no subscriber, no handler | both | M7 assumes it (S:682) | before the first S item | S:705 language decision | N | wrong-milestone: see mre-1. |
| A-MD15 | Fast iteration | missing | hot reload, iteration: 0 hits. S:722 proposes headless scenarios only | mechanism | none | as need appears (16.1 Decided) | none | NOT TRACED | |
| A-MD16 | Data inheritance | missing | "inherit" S:147 is class trees. P-U `parent` REJECTED | mechanism | none | before MD19 | none | N | |
| A-MD17 | Asset pipeline | partial | S:225-231 Baseline. `VisualDef` = `Capsule` or `Cuboid` only (schema.rs:48-51, 491-496) | mechanism | M13 (S:688) | playable needs models and icons | D61 | Y: `VisualDef` | wrong-milestone: M13 is after every gameplay milestone. |
| A-MD18 | Validation messages | present | S:209, S:229. Probe output names package, file, field: `probe/manifest.ron: dependencies: ...` | mechanism | M1 | none | none | none | |
| A-MD19 | Mod layering | missing | S:197 and S:708: override semantics Open. One map per package: manifest.rs:51-58. No dependencies: manifest.rs:44 | mechanism | none | M8 (S:683) | MD16, mre-2 | Y | |
| A-MD20 | Effect system | partial | S:160 "effects" engine mechanism; S:426. One effect variant (schema.rs:287-293) | both | M3 (one variant) | playable | none | Y: `EffectDef` | The lowest-layer rule (S:177-183) needs this before scripts. |
| A-UN07 | Abilities | partial | S:160 table row. No schema | both | none | playable | MD04, MD20 | N | |
| A-OR20 | Ability order | missing | Wire has six commands: `crates/turts_net_protocol/src/messages.rs:54-78` (MoveTo, Gather, Produce, CancelProduction, Remove, SetEnabled) | mechanism | none | playable | UN07 | Y: protocol | |
| A-OR21 | Autocast toggle | missing | "autocast": 0 hits | both | none | after OR20 | OR20 | N | OPTIONAL tier. |
| A-UI01 | Command card | partial | S:164 "command panels" (Decided, mod policy). Engine derives controls from capabilities: economy_ui.rs:310-340 | both | M2-M3 (engine-built) | playable | MD08, IN05 | Y | |
| A-IN05 | Hotkey per command button | missing | "hotkey", "keybind": 0 hits ("binding" S:382 is key binding to identity). P-U `hotkey` REJECTED | both | none | playable | IN02 engine action map, UI01 | N | B-12 same gap. |
| A-IN06 | Grid hotkey layout | missing | as IN05 | mechanism | none | after IN05 | IN05 | N | OPTIONAL. |
| A-AC01 | Rebindable keys | missing | "rebind": 0 hits | mechanism | none | playable | IN02 | N | Engine side; mod gives defaults. |
| A-UI27 | Mod UI layout | partial | S:164, S:179 | both | none | after playable | MD08 decision | Y (as MD08) | |
| A-UI23 | Cursor set | missing | "cursor": 0 hits. P-U `cursor` REJECTED | both | none | playable | PC21 | N | |
| A-PC21 | Context cursor | missing | "cursor": 0 hits | both | none | playable | mre-11 order model | N | |
| A-AU01 | Unit responses | missing | S:122 only | both | none | playable | MD09 | N | |
| A-AU02 | Alerts | partial | S:338, S:634: alerts are replicated data. No alert definition | both | none | playable | mre-5 | N | |
| A-AU08 | Repeat-click lines | missing | 0 hits | policy | none | later | AU01 | N | OPTIONAL. |
| A-MP04 | Triggers and map scripts | missing | "trigger" hits S:470, 692, 774, 792 are other senses. P-M `triggers` REJECTED | both | none | after script runtime | MD14, mre-5, mre-6 | N | COMMON tier. |
| A-MP10 | Custom game modes | partial | S:163 "modes" policy. D06 Baseline (S:29) "not arbitrary genres" | both | none | after MP04 | MP04 | N | Tension with D06 for WC3-style modes; D06 is Baseline, not Decided. |
| A-MF16 | Campaign and scenarios | missing | campaign: 0 hits; "mission" hits are admission/permission or bot squads (S:770) | both | none | after MP04 | MP04, MD13 | N | COMMON tier. |
| A-MF08 | Victory and defeat | partial | S:163, S:748 | policy | none | playable | MD10 | N | |
| A-MF10 | Team victory | partial | S:748 "player or team". No team type in schema | both | none | playable | TM01, MD12 | N | |
| A-MF05 | AI in slots | partial | S:301, S:786 Decided. No runtime | both | M12 | playable | MD11 | Y (deps) | |
| A-BT01 | Skirmish AI | partial | S:755-776 Decided | both | M12 (S:687) | playable | script runtime, mre-2 | Y | wrong-milestone (see MD11). |
| A-BT02 | Difficulty levels | partial | S:776, S:780 Open | policy | M12 | with BT01 | BT01 | N | |
| A-BT03 | Same command path | present | S:761, S:774 Baseline | mechanism | M12 | none | mre-11 | N | Only six wire commands exist today. |
| A-BT05 | Unit micro AI | missing | 0 hits | both | none | later | OR21 | N | OPTIONAL. |
| A-FV03 | Sight radius | partial | S:162 policy. P-U `sight` REJECTED | both | M5 (S:680) | playable | FV02 | N | |
| A-FV05 | Shared vision | partial | S:162 | both | M5 | M5 | TM01 | N | |
| A-FV06 | Detection | partial | S:162, S:426 | both | M5 | M5 | FV03 | N | |
| A-FV11 | Fog lobby options | missing | explored, shroud: 0 hits | policy | none | M9 | MD12 | N | |
| A-CA01 | Classic camera | partial | S:60 (D37), S:101; S:701 camera API Open. P-M `camera` REJECTED | both | none | playable | mre-8 | N | |
| A-CA13 | Camera bounds | partial | S:101 "motion constraints". `MapDef.size` only (schema.rs:455-457). P-M `bounds` REJECTED | both | none | playable | MP09 | Y: `MapDef` | |
| A-CA05 | Zoom limits | partial | S:101 | both | none | playable | CA01 | N | |
| A-UI11 | Tooltips | partial | S:164 | both | none | playable | MD13, description field | N | |
| A-UI30 | Localisation | missing | as MD13 | both | none | before MD08 | MD13 | Y | |
| A-MP07 | Map distribution | present | S:276 Decided, S:278 Baseline. M1 acquisition exists (`crates/turts_client/src/acquire.rs`, not read in detail) | mechanism | M1 | none | none | NOT TRACED | |
| A-SV05 | Mod save data | partial | S:714 "dynamic mod-state serialization" Open | both | none | with script runtime | mre-1 | N | Data-only mods keep state in engine components. |
| A-SV04 | Save version safety | present | S:65 (D42) | mechanism | none | none | none | N | |
| B-12 | Hotkeys (player list) | missing | as IN05 | both | none | playable | IN02 | N | |
| B-15 | Command card or build menu | partial | as UI01 | both | M2-M3 | playable | MD08 | Y | |
| B-43 | Spells, target cursor, mana, autocast | partial | S:160 | both | none | playable | UN07, UI23 | N | |
| B-78 | Custom player game modes | partial | S:163 | policy | none | after MP04 | MD10, MP04 | N | |
| mre-1 | Script runtime has no build milestone | missing | S:705 Open; S:682 M7 lists "scripts" as a workload; M2 design:600 defers until "a concrete rule needs scripts" | mechanism | none | before the first S item (victory or AI) | language decision (S:193, D34) | N | wrong-milestone: M7 measures a runtime that no milestone builds. |
| mre-2 | Library dependencies refused | partial | D33, D35 Decided (S:56, S:58, S:207-209). `manifest.rs:44-49` rejects any dependency (P1) | mechanism | M8 (S:683) | before AI libraries and generator libraries (S:213) | resolver, lockfile entries | Y: `manifest.rs` | Code lags Decided items; not a spec conflict. |
| mre-3 | Requested capabilities not checked | partial | S:209 field; S:708 names Open. `schema.rs:112` `Vec<String>`; P2 `["net.raw_socket","fs.write"]` ACCEPTED | mechanism | none | with script runtime | mre-1 | Y: reject unknown names | The field exists and grants nothing yet, so the risk is low today. |
| mre-4 | No cosmetic-only package kind | partial | S:203; S:708 Open. `PackageKind` = Game, Library (schema.rs:13-16). P3 `Cosmetic` REJECTED | both | none | before UI, audio, cursor mods | S:203 rule | Y | Without it a HUD mod changes the gameplay lockfile. |
| mre-5 | No event hook catalogue | partial | S:201 (four events), S:163. M2 events are production only (M2 design:462-463) | mechanism | none | playable | none | Y: extend the event batch | Missing: entity destroyed, slot defeated, under attack, area entered, timer. |
| mre-6 | No declarative condition vocabulary | missing | S:767, S:771 "mod-supplied conditions", S:290 "validation": no form | mechanism | none | playable | mre-5 | N | Victory, settings validation, AI conditions and triggers share it. |
| mre-7 | No entry-point or script export | missing | S:209 "exports/entry points". `Exports` (schema.rs:55-66) lists data files only. P4 `exports.scripts` REJECTED | mechanism | none | with runtime | mre-1 | Y | |
| mre-8 | No client-context runtime or presentation API | missing | `ExecutionContext::Client` exists (schema.rs:20-24) and runs nothing. S:173 "mod API describes presentation capabilities"; S:701 Open | mechanism | none | playable | MD08 decision | N | needs decision. |
| mre-9 | Interface split has no API | partial | S:164 Decided. Engine panel: economy_ui.rs:130-337. Default-mod content ids in client code are only under `#[cfg(test)]` (lib.rs:284, scenario.rs:674, ingest.rs:1024) | both | M2-M3 | playable | mre-8 | Y | Generic engine UI, not a privileged default-mod shortcut. |
| mre-10 | Exactly one map per package | partial | `manifest.rs:51-58` (M1 placeholder). MF03 map choice needs map packages | both | M1 | M9 | mre-2 | Y | |
| mre-11 | Closed engine order set | partial | S:160 "Orders" engine mechanism. messages.rs:54-78: no Attack, Stop, Hold, Patrol, Build, Ability; `Gather` takes one unit (`:59-62`) | mechanism | none | playable | OR11 | Y: protocol | S:747 needs attack and attack-move. |

## Conflicts with the goal (Decided items)

- No Decided item in 4.3, 5, or 6.5 blocks the goal. D04, D05, D33, D34, D35, D53 support it.
- Tension, not a block: S:720 (16.1, Decided) "Do not precommit to a large manual editor suite".
  SC2 and WC3 modders expect a map and trigger editor (MP02, MP04). The text allows tools that
  emerge from need. Name it; do not reopen it.
- Tension: D06 (S:29, Baseline) "not arbitrary game genres" against WC3-style custom modes (MP10).
- Outside my seat: D41 (S:64) "no standalone solo mode or local game saves" against SV01. For the
  owner seat.

## Lens answer

Full table: `core-classification-mod-runtime.md` in this directory (my judgement; produced by
`probe-modrt/classify.py`). Of 181 CORE items:

- 78 need only an engine mechanism (input, selection, groups, camera input, picking, pathing).
  No mod surface is needed, except default values.
- 98 need a declarative data field. Today the schema expresses 10 fully (OR17, PR01, PR02,
  PR05, PR08, EC01, EC02, EC04, CB01, CB21) and 20 partly. 72 mod-surface items have no field.
- 15 need a script or event hook for the union bar: OR20, UI16, UI19, CO03, CB04, UN07, MF05,
  MF08, MF10, SV05, MD04, MD10, MD11, BT01, BT02.
- 9 need a UI description language: UI01, UI02, UI03, UI04, UI09, UI10, UI11, MF01, MD08.

Does the absence of a script layer block a CORE item? Yes, for AI. S:759 (Decided) puts the
bot machinery in libraries. S:207 (Decided) runs libraries on the script runtime. The code also
refuses dependencies (`manifest.rs:44-49`). So MF05, BT01, BT02, and MD11 cannot exist as
specified until a runtime and a resolver exist. The other path is an engine-side Rust bot
framework exposed to all mods (S:189 allows this). That path needs decision.

Victory (MF08, MD10, MF10) is not blocked for the 16.2 rule. A declarative "last slot or team
with a surviving unit of type X" plus a death event is enough. The union bar (regicide, wonder,
relics, score, timed) needs a script or a growing primitive set.

The larger blocker is not scripts. It is the data layer: 72 CORE mod-surface items have no
field, the composition layer has one effect variant, and no event catalogue exists. Section 5.1
says to use the lowest layer that works. That layer is thin today.

## What I ran

| command | claim tested | result that would refute | observed |
| --- | --- | --- | --- |
| Probe `probe-modrt/src/main.rs` (cargo run; 35 cases; output `probe-modrt/probe-output.txt`) | Schema cannot express hotkey, icon, sight, abilities, orders, sounds, cursor, armour, splash, victory, bounds, camera, triggers, factions, scripts, ui (schema.rs:27, 103, 326) | Any of these ACCEPTED | All REJECTED. Controls U0, W0, M0, P0 ACCEPTED, so the instrument can go green and red. |
| Same probe, P1 | D33 libraries usable (S:207) | P1 ACCEPTED | VALIDATE-REJECTED "dependencies are not supported at M1" |
| Same probe, P2 | Capabilities are validated (S:280 "apply capability limits") | P2 REJECTED | ACCEPTED with invented names |
| Same probe, P3 | A cosmetic kind exists (S:203) | P3 ACCEPTED | PARSE-REJECTED |
| `python3 -I probe-modrt/classify.py inventory-engine.md` | Yardstick has 181 CORE items; my table covers each once | assert fails | 181; assert passed; counts above |
| `rtk proxy grep -rn -i -E 'victory\|defeat\|winner\|game_over\|match_end' crates` | Some victory code exists | any hit | 0 hits |
| `rtk proxy grep -rn -E '"(forge\|base\|haul\|acme\|cedar):' crates/*/src` plus `cfg(test)` lines | Engine code has a default-mod shortcut (S:97, S:189) | a hit outside `#[cfg(test)]` | All hits after `#[cfg(test)]` (lib.rs:284, scenario.rs:674, ingest.rs:1024) |
| `rtk proxy grep -n -i -E 'script\|rhai\|sandbox'` on M2, M3, M4 designs | A milestone builds the script runtime | a build commitment | M2: "no script layer" (:4, :595); M3:56 none; M4 none |
| Spec term sweeps (`rtk proxy grep -n -i -o -E`): locali, inherit, hot reload, effect, trigger, cosmetic, hotkey, cursor, audio, camera, faction, armour, autocast, campaign, condition, entry point, explored, mana, icon | Each "missing" row has no spec line | a hit in the right sense | Hits listed in rows are other senses |

## NOT TRACED

- `crates/turts_client/src/acquire.rs` behaviour for A-MP07 (not read in detail).
- Fast-iteration cost (A-MD15): I did not time a server restart with changed data.
- Whether `turts_pack` or the server rejects unknown capability names later than
  `validate_manifest`. A crate grep found no other use of `capabilities` outside tests.
- The classification of each CORE item into E, D, S, U is my judgement, not a run.
