# Damage, armour, and two combat units

Status: implementation and native checks pass. Release acceptance is still open.
This report describes the current damage branch. It does not certify the final release build.

Evidence root: `/home/julien/Projects/TuRTS/target/mc-evidence/damage-system/`.
All evidence paths below are relative to that directory.
The specification is `docs/superpowers/specs/2026-11-14-damage-system.md`.

## What changed

The server now applies one shared integer damage rule from `turts_rules`.
It combines bonuses, type ratios, optional armour curves, flat armour, and a minimum hit.
The table and curve share one rounding step. Overflow rejects the shot without consuming ammunition.
Missing table cells, absent armour, and untyped weapons retain their documented defaults.

Armour research affects existing and later units of its owner.
The server records completion order for armour upgrades.
This preserves `FlatByType` semantics when a new type row captures the current general flat value.
The server and HUD use the same effective-armour helper.
Enemy armour stays hidden because another player's research is private.
The weapon tooltip shows damage, type, and bonuses.
Reduced hits select `hit_reduced` when the visual defines it.

The mod API is **0.2.12**. The wire protocol is **7**.
The owner-only research record adds `armour_order`.
The fired effect adds the `reduced` flag. Older protocol peers are refused before claiming a slot.
Manifest maps now serialize as maps. The former `UniqueMap` wrapper produced RON that its reader rejected.
Manifest parse/write/read tests cover that fix.

The Skirmish adds the Warrior and Guard tower from the supplied CC0 art.
The Barracks trains Warriors. The Castle builds towers within its build area.
The computer uses normal paid orders to train Warriors and build a tower.
Infantry can acquire enemies, chase within a leash, select a reachable target, and return home.
Player orders take priority. Towers never move.

The Warrior uses scale **0.875**. Its model radius is **1.494 times its footprint radius**.
Its measured walk stride is **0.45921269 map units per loop** from 241 pose samples.
Source hashes: `supplied-art.json`. Stride: `warrior-stride-final.json`.
Package validation and bounds: `skirmish-content-valid.log`.
Static portraits come from the supplied models. The source meshes were not regenerated.

## Acceptance evidence

“PASS” below applies to the executed native check or preview check named in the row.
Preview browser results still need confirmation on the final release build.

| Row | Result and evidence |
| --- | --- |
| DM1 | PASS. Forty-six independent Python `Fraction` cases cover single steps, combined steps, overflow, and health clamping. `rules-first.log`; `native-final/native-checked.log`. Inputs: `crates/turts_rules/tests/fixtures/damage_pipeline.tsv`; generator: `scripts/qa/damage/generate_cases.py`. The single-floor case returns 5. |
| DM2 | PASS. The published hit counts are 10, 20, 31, 40, and 334. `rules-first.log`; `documented-cases.log`. |
| DM3 | PASS. The server removes 5 health for 15 damage, cell 1/2, and flat 2. The four default weapons are checked against all three classes. `sim-pipeline-checked.log`; `native-final/native-checked.log`. |
| DM3b | PASS. TargetSurface range 1.2 hits radius 1.5 at distance 2.6 and refuses 2.8. `target-surface.log`; `native-final/native-checked.log`. |
| DM4 | PASS for the executed legacy checks. Plain legacy combat remains covered. The M2 frozen operation stream matches its baseline. Forge fixed-point and renamed-package equality checks pass. `native-final/native-checked.log`. No changed frozen stream was observed. This does not claim that every historical stream was rerun. |
| DM5 | PASS. Actual research completion changes existing and later-trained units of its owning slot. Another slot stays unchanged. Ordered Flat, FlatByType, and later Flat upgrades retain general flat 7 and type flat 5. `ordered-armour-research-checked.log`; `native-final/native-checked.log`. |
| DM6 | PASS. Two matching bonus rows add. An unmatched row adds nothing. Overflow preserves health and ammunition. `sim-pipeline-checked.log`; `native-final/native-checked.log`. |
| DM7 | PASS. Actionable refusal cases and sparse/default cases pass. Five complete style packages pass the actual pack CLI with exit 0. `native-final/runtime-final.log`; `styles-complete/result.json` and its five style logs. |
| DM8 | PASS. Two runs produce identical health at every tick with a type table, curve, and actual completed armour upgrade. `modifier-determinism.log`; `native-final/native-checked.log`. |
| DM9 | PASS on preview. Mouse-driven research changes an existing Warrior from armour 2 to 3. A later-trained Warrior shows 3. An unarmoured lab has no armour row. Enemy armour stays hidden. `research-browser/result.json` and screenshots in that folder. Final-release rerun pending. |
| DM10 | PASS on preview. Mouse input trains a Warrior and builds a tower. Walking, melee damage, attack animation, and a tower shot are recorded. The unfinished tower has no active weapon. `preview-loop/result.json` and screenshots. Training took 6,501 ms; construction took 15,117 ms. Final-release rerun pending. |
| DM11 | PASS. All 25 battle rows pass with the proposed starting statistics. No balance tuning was needed. `native-final/balance.json`; `native-final/pack-presentation-balance-final.log`. See the table below. |
| DM12 | Preview normal-stat win PASS: `win-browser/result.json`, `win-browser/enemy-warrior-and-tower.png`, and `win-browser/terminal.png`. Victory appears at 154.332 seconds. Three Villagers fund eight Swordsmen through timber deliveries. The enemy has its normally funded Warrior and completed 400-health tower before the fight. The player castle retains 1,000 health. Final-release win and the three cold probes remain pending. |
| DM13 | PASS. `docs/modding/damage.md` documents the rule and five game styles. Tests read all 14 worked-hit rows and five kill-count rows directly from that page. `documented-cases.log`; `native-final/native-checked.log`. |

Acquisition checks also pass for chase/return, zero leash, static towers, blocked-target replacement, and manual order priority.
Evidence: `acquisition-checked.log` and `native-final/native-checked.log`.
Coordinate-priority checks swap spawn identity and retain the same physical result.
Evidence: `coordinate-priority-checked.log` and `native-final/native-checked.log`.

## Recorded balance

Every run uses shipped statistics, idle acquisition, and ordinary movement and combat.
The legal formation anchors start 12 map units apart. Formation spacing is 1.35.
The rotations are 0°, 90°, 180°, −90°, and 45°.
The simulation runs at 20 ticks per second. No health or damage overrides are used.

For Barracks destruction, TTK starts at the first actual damage tick and includes the destruction tick.
Full elapsed time also includes the approach. This basis was fixed before reading results.
The Barracks has its existing 500 health.

| Rotation | 6 Crossbowmen vs 4 Warriors | 3 Crossbowmen vs tower | 5 Warriors vs tower | Barracks: 8 Crossbowmen, TTK / full seconds | Barracks: 8 Warriors, TTK / full seconds | TTK ratio |
| --- | --- | --- | --- | --- | --- | --- |
| 0° | Crossbowmen win; 2 survive; 241 ticks | Tower wins; 341 ticks | 5 Warriors survive; 206 ticks | 16.55 / 17.70 | 7.35 / 10.00 | 2.25 |
| 90° | Crossbowmen win; 2 survive; 241 ticks | Tower wins; 341 ticks | 4 Warriors survive; 279 ticks | 19.20 / 20.35 | 5.95 / 8.65 | 3.23 |
| 180° | Crossbowmen win; 2 survive; 241 ticks | Tower wins; 341 ticks | 5 Warriors survive; 206 ticks | 23.20 / 24.35 | 5.90 / 8.60 | 3.93 |
| −90° | Crossbowmen win; 2 survive; 240 ticks | Tower wins; 341 ticks | 5 Warriors survive; 207 ticks | 19.30 / 20.45 | 5.15 / 7.80 | 3.75 |
| 45° | Crossbowmen win; 2 survive; 244 ticks | Tower wins; 341 ticks | 5 Warriors survive; 207 ticks | 23.20 / 24.35 | 6.65 / 9.35 | 3.49 |

Each equal-cost army starts at 240 credits. The surviving Crossbowmen cost 80 credits, below half the starting cost.
The tower wins all five light raids. Warriors destroy the tower in all five runs.
The Barracks TTK ratio exceeds 2 in every placement.
The JSON retains all first-hit ticks, elapsed ticks, survivors, costs, winners, and timeout flags.

Final default values remain unchanged from the proposed table:

| Unit | Health | Damage / type | Cooldown | Range | Speed | Cost | Duration | Flat armour |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Crossbowman | 120 | 15 pierce | 700 ms | 6.0 TargetSurface | 4.0 | 40 | 4 s | 0 |
| Warrior | 160 | 20 blade | 900 ms | 1.2 TargetSurface | 3.6 | 60 | 6 s | 2 |
| Guard tower | 400 | 18 pierce | 800 ms | 7.5 TargetSurface | static | 100 | 15 s | 0 |

Pierce ratios are 1, 3/4, and 1/4 against light, heavy, and structure.
Blade ratios are 1, 1, and 3/4. Minimum damage is 1. The default game uses no armour curve.
The existing Swordsman keeps 300 health, 25 blade damage, and its previous Surface range.

## Verification and remaining work

The broad native run covers API, runtime, rules, simulation, protocol, server, and packer.
Its ten stale fixture/version targets were repaired. Their focused reruns pass all 475 tests.
The simulation library passes 327 tests with one existing ignored test.
Commands, exit codes, and limits are in `native-final/summary.json` and `native-final/focused-results.json`.
The client and web suite passes **686 tests**, with **5 existing ignored tests**: `client-final.log`.
Those ignored tests were not executed. These results do not replace browser acceptance.

The browser evidence above uses the diagnostic preview distribution.
Each driver uses its own fresh Chrome profile, null ALSA output, and a separate Xvfb display.
The completed preview drivers record profile removal. The CEO's browser and system sound settings were not used.

The following delivery checks are still pending:

- The sealed release build and final package validation.
- Final strict Clippy after the reported lint fixes.
- Detailed-model GPU measurements at 400 and 2,000 units, including Warrior content and the 400-unit camera pan.
- Final-release DM9 and DM10 browser confirmation.
- All three final cold-play probes and the normal-stat full win required by DM12.
- One independent foundation review that runs code, followed by fixes for its findings.
- The global launcher starting the accepted final build.

No release performance result, final cold-play result, full-win result, or independent-review approval is claimed in this draft.
No CEO play request is made.

## Spec corrections and parked scope

The current API already had Centre and Surface range contracts.
TargetSurface adds the requested target-edge rule without changing either old mode.
The existing Swordsman remains a distinct unit.
The correction appendix also preserves sparse combat tables and units without armour or damage types.

The CEO's arrival and identity rule overrides the spec's old unit-ID targeting tie-break.
Equal-distance target choice and sequential firing order now use physical coordinates.
This does not introduce simultaneous fire or change cooldowns.
The old planted-fault and repeated-review machinery is superseded by the current working rules.

Role captions remain parked because the client role field has no implementation.
The current panel shows the unit name, armour, health, portrait, and weapon details.
An armour research building is an acceptance fixture. It is not added to the default game.
Penetration, directional armour, negative armour, damage over time, area damage, shields, healing, and veterancy remain outside this milestone.
