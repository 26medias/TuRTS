VERDICT=fail FINDINGS=8 HIGH=4 RUNS=6 NOT_CHECKED=3

The M3 demo has no win and no loss, and the drone side wants nothing. That scope is decided (S:722; MP1 adds match flow), so I review the walk.

## Findings

Blocking:

1. `mods/forge/maps/proving_ground.ron:11,13`, `units/turret.ron:11`. High. The near drone is 7.0 from the turret (range 8). It dies before window B opens. Steps 10, 12, 13 and 15 ("two drones") fail. Needs decision (questions).
2. `crates/turts_sim/src/slots.rs:45-55`, `crates/turts_client/src/lib.rs:76`. High. Step 18 reloads. The page gets a new random id, both slots are taken, and I see the rejection screen. My last drone is also dead by step 17, so I cannot test the new key or group 1. SEL4 and PER1 reload too. Fix: define a rejoin, or drop the reload. Put step 18 before step 15.
3. Design 8.4:596, path:1300-1301. High. The info panel has no weapon fields, so "damage 20" cannot show. I never see what the research gave me. Fix: show damage, range and ammo.
4. Design 14:955. High (instrument). The tester "follows the Player path", which names every key. With an empty guide and no tooltips, PT1 still passes. That is not blind (S:708-709). Fix: the tester gets goals only. The path is the oracle's list.

Non-blocking:

5. Path:1292,1302; design 4.1:327, 4.3:355-356, 9.3:680-682. Medium. The design has no world tooltip ("Bore Drill"). `no_applicable_order` has no text ("Cannot move"). Fix: define both, or change the path.
6. Path:1313-1316; design 1.3:64-67, 9.5:716. Medium. I cannot see the turret range, so I guess. When it fires, I see no shot and hear nothing. Only a bar drops. I want a range circle on a selected weapon unit.
7. `proving_ground.ron:19`; design 8.4:595, 12.1:838. Medium. Forge has no slot balances. The resource line has no source.
8. Design 1.4:93, 9.1:650, 7:528, 12.2:849-853. Medium. In PT2 I see meshes, and K17 says meshes overlap at rest. Ring size and marker lifetime are not given, so "near the point" is a guess. The muster camera starts between the troopers and the far block. Fix: judge by ground rings, keep the marker until rest, start on the troopers.

Control (3, 4) matches my habits: Shift toggle, type select, Ctrl+N, Shift queue, S, H, A-click, marker on the click frame. Missing, not blocking: Tab sub-groups, select-all-army, idle button. Alerts (9.4) suit the machines. I want "research ready" and "turret firing" for slot 0.

Cut line (15.2): PT1 and PT2 pass without each item. No minimap: step 11 changes; the map fits one screen. No audio: alerts are easy to miss. No stored groups: step 18 drops. No K5: no change, because forge drones have no weapon. So no person tries attack-move in MC. The double tap is in neither list (15.2:1003,1011-1019).

## What I ran

Log: `/tmp/claude-1000/-home-julien-Projects-TuRTS/0e0457bb-045c-4b6a-945f-1db6a0753ee2/scratchpad/mc/gate1-player-playlog.md`.

| run | claim | refute | observed |
| --- | --- | --- | --- |
| read map, turret, drone | path:1304 | drone out of range | 7.0 < 8.0 |
| grep server for a join wait | drone lives to step 10 | a wait | none (`stats.rs:131`) |
| read slots, `session.rs:103` | path:1318 | a reclaim | none |
| grep design for four texts | path:1292,1301,1302,1313 | a source | none |
| 8.4 against `slot_economy` | 12.1:838 | item source | none |
| paper play, steps 1-18 | PT1 passes | all doable | fails 10,12,13,18 |

## Not checked

P1 keys (NOT TRACED). Shell timing (estimated). Muster placements (no file).

## What I changed

none

## Open questions for the orchestrator

- F1 needs decision: (a) one-drone path; (b) drone steps on `muster`; (c) new forge map, which breaks the M3 fixed point (12.1:832-833).
- Is rejoin after reload in MC scope (D25)?
