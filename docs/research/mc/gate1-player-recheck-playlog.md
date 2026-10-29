# Paper play, revision 2 Player path (design:1545-1586), forge map open
- Drone A (24,12) dies before B joins (turret 17,12 range 8): proving_ground.ron:11,13; turret.ron:11. Path says so (1549-1550). OK.
- Drone B (30,4), distance 15.3, alive at step 10. Speed 2.0 (drone.ron). Reaches range in about 4 s at step 17.
- Turret: cap 5 shells, 1 shell/shot, 20 damage after research -> 5 shots kill 100 hp drone B. Turret is full by then. Steps 18-19 take about 5 s.
- Window A start: loader to turret is placements line 9, loader to lab is line 10 (proving_ground.ron). If the turret loader wins each shell while drone A lives, the lab sits at 0/10 for about 20 s, then fills to 10 in about 20 s more (0.5 shell/s, make_shells.ron). Loader priority NOT TRACED. Sign of progress meanwhile: drone A health bar, if on screen.
- Step 7: ResearchReady fires only on a change of the PRIMARY producer entry (834). Lab not selected when it fills, or lab already full at first click -> no alert. Oracle step 7 red on a correct build.
- Step 15: rejoin needs "no live session" (916-918). Liveness rule not defined; token timeout 10 s (crates/turts_server/src/net.rs:36-37); disconnect handling later:M10 (1495). A quick reload can get "This game is open in another window".
- PT2: marker kept "until every unit rests or 10 s pass" (787) vs PT2 "until the units rest" (1159). Troopers 3.0/s on a 64 x 48 map: a move over 30 loses the marker before rest.
