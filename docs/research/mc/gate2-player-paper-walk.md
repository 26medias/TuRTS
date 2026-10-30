# Paper walk of the D Player path (D:1644-1679) against the plan tasks

| Step | What I do and see | Task that builds it | Row that tests it before PT1 |
| --- | --- | --- | --- |
| 1 | load screen, phases, bar | 16 (shell.rs, plan:608) | SH1 (16) |
| 2 | "Click to play", guide opens at page 1, camera on factory | 16 overlay; guide open at first join: NO TASK NAMES IT; camera start: 11 (OwnedCentre default) | none for the guide open |
| 3 | close guide with F1 or button | toggle_guide is an engine action (D:342); no task step names F1 or the guide toggle | none |
| 4 | pan, zoom, hover ring, tooltip "Bore Drill" | 11, 13, 12 tooltip.rs, 17 | CAM1 (11), SEL1 (17) |
| 5 | lab info panel, disabled research, reason tooltip with "R" | 12, 17 | HUD2 (17) |
| 6 | disable belt, InserterWaiting alert with sound, enable | 15, 17 | ALR1 (15) headless; HUD2 |
| 7 | Research ready alert, R, queue bar | 15, 17 | ALR1, HUD2 |
| 8 | Research complete alert, turret info 20/8/ammo | 15, 17 | ALR1, HUD2 |
| 9 | turret right click: "Shell Turret: Cannot move" + error sound | 9 (local notice), 15 (feedback.rs), 17 (reasons) | no row checks the local notice text with the unit name |
| 10 | window B, camera on own drone in slot 1 colour | 11, 13 | VIS1 |
| 11 | turret on minimap, gapped ring, empty card, range circle | 13, 14, 17 | SEL1, MM1, VIS1 |
| 12 | box drag selects drone, panel with health bar | 13, 17 | SEL2, HUD2 |
| 13 | Ctrl+1, click ground, 1, 1 twice centres | 9, 11, 16 | KM3, SEL4 |
| 14 | F10 menu, scroll speed, rebind Stop | 16 | SH1, KM1 (no row for swap/cancel or Reset, D:928-930) |
| 15 | reload, same slot, group 1 and new Stop key | 7, 16 | SLOT1, SEL4, KM1 |
| 16 | Shift + two right clicks, waypoint lines, new Stop key | 4, 13, 14 | ORD3 |
| 17 | right click in range circle, marker at once, stays until rest | 13, 15 | ORD7, VIS1 |
| 18 | turret fires, bar drops, "Unit under attack" sound AND minimap ping | 15 (alert), 14 (minimap); PING: NO TASK NAMES IT | ALR1 says "with sound and ping" (D:1209) but plan task 15 step 1 (plan:566) has no ping; task 15 does not need 14 (plan:138) |
| 19 | drone destroyed, leaves selection and group, "Unit destroyed" | 9, 15, 17 | SEL4, ALR1 |

Guide "Controls": page 5 is a List on Bindings (plan:578). Pages 1 to 4 are free text; plan:581 forbids ticks, Applied, coordinates, studio words, not key names. No row checks that page 5 changes after a rebind.
