# Swordsman delivery

Status: delivered. Native checks, independent review, performance, combat browser checks, scripted player probes, and the actual launcher pass.
Engine checkpoint: `d2214625`. Author branch: `codex/swordsman`.
Main is unchanged.

## Player change

Barracks now offer a Crossbowman for 40 credits and a Swordsman for 80 credits.
The Swordsman has 300 health and a sword and shield. It trains in six seconds.
Its attack reaches 0.6 map units beyond the two footprint edges.
Crossbowmen keep their old center-distance range.
The Swordsman has its own measured walking stride and melee attack clip.
Extra health gives protection. There is no damage reduction mechanic.

## Foundation change

API 0.2.8 adds optional `weapon.range_mode`. Omission means Centre.
Firing, approach, and attack-move share the new Surface geometry.
The broad phase includes large live targets. Exact checks use each target radius.
The audit catalog adds an optional field. Old audit records default to Centre.
The replay now reconstructs the existing bounded approach rounding correction.
The old replay could reject a legal approach goal.
No game command or snapshot wire layout changed.

## Checks

Paths below are relative to `target/mc-evidence/swordsman/`.

| Row | Result | Evidence |
| --- | --- | --- |
| SW1 | PASS | `api-test.log`; `native-full.log` |
| SW2 | PASS | `surface-green.log` |
| SW3 | PASS, 16 bearings for attack and attack-move; no overlap | `surface-green.log` |
| SW4 | PASS, both members of one mixed command fire | `surface-green.log`; `audit-live.log` |
| SW5 | PASS, 6.08 seconds, exact 80-credit charge, rally arrival | `browser-verified/result.json` |
| SW6 | PASS, walking, both attacks, hit flash, death, team art; native no-projectile check | `browser-verified/result.json`; `browser-verified/mixed-closeup.png`; `review-melee-fx.log` |
| SW7 | PASS, real mixed attack and attack-move replay; exact repeated records | `audit-live.log` |
| SW8 | PASS, High 45.23 FPS, Low 53.31 FPS; all 400 units visible and taking damage | `400-guard-verified/result.json` |
| SW9 | PASS, both window sizes, buttons, and actual launcher; sound off | `cold-1280/probe-first-click.json`; `cold-1024/probe-first-click.json`; `cold-buttons/probe-buttons.json`; `launcher/result.json` |

The full native run passed 1,485 tests. Seven existing tests were skipped.
The run covers API, runtime, simulation, audit, and client packages.
`surface-red.log` records the disabled range rule: three semantic failures, exit 101.
The restored rule passes all three tests. Strict Clippy passes in `clippy.log`.
The later index refactor passes the targeted simulation and live audit checks.

The original model failed the footprint fit check. Its sword prop is now 60 percent of source size.
The body remains 1.7 units tall before the visual scale.
Stride: `stride.json`. Art export: `art-export.log`. Attack motion: `attack-motion.log`.
The attack impact fraction is 0.6, at the end of the downward chop.

## Limits and scope

The browser fixture uses a durable castle and increased crossbow damage to observe a Swordsman death quickly.
It does not establish final game balance.
The 400-unit fixture keeps normal weapon reaches and raises health to sustain combat.
Every test Chrome must use a fresh profile and null audio.
No physical sound output or new manual CEO playtest is part of this delivery.
The computer keeps its current crossbow strategy. Mixed computer strategy is parked.

The independent foundation review passed on engine `d2214625`. Nine targeted tests passed.
See `docs/reports/2026-11-14-swordsman-independent-review.md` and `review.md`.
No findings required a fix. The review did not run a browser.

The first GPU instrument incorrectly required a permanent melee attack pose.
The corrected instrument checks sustained damage across the timing window.
The first browser range assertion used the initial Fired frame before movement interpolation caught up.
The corrected assertion checks the sustained attack after 250 ms.
The Swordsman holds a center distance of 3.2000001 across 113 samples.
Both corrections affect browser drivers only. Build and artifact hashes are unchanged.

## Play and remaining checks

Run `/home/julien/Projects/TuRTS/target/mc-evidence/play-skirmish.sh`.
Select Barracks, train a Swordsman, then right-click the enemy castle.
It must walk close and strike from outside the wall.

The launcher uses engine artifacts from `d2214625`, with driver-only follow-ups through `e917050f`.
The stable play worktree stays at `e917050f`. Integration documentation follows separately.
The actual launcher check passed. Its fresh profile was removed. Sound stayed off.
Receipt: `target/mc-evidence/swordsman-launcher-check/receipt.json`.
Final player receipt: `target/mc-evidence/swordsman-final-player-checks/receipt.json`.
Both commands and wrappers exited 0. Their author source stayed unchanged.
All scripted player probes pass. The old checker reports PARTIAL-PASS because its manual and reference stages were not repeated.
No new human playtest was done.

Next: show the existing minimap in the skirmish, then test navigation, orders, and warning pings.
P4 environment work, broader MP1 systems, and remote two-human acceptance remain open.
No user decision is needed for this follow-up.
