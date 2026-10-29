VERDICT=pass-with-changes FINDINGS=3 HIGH=1 RUNS=5 NOT_CHECKED=2

## Original findings

1. Near drone: fixed. K22 (a), design:106, Player path:1549-1550, PT1 one drone (1158).
2. Reload rejection: fixed with a new defect (N2). Design 10.5:914-920, path step 15 before the drone dies.
3. No damage shown: fixed. Design 8.4:715, path step 8.
4. PT1 not blind: fixed. The tester gets goals only (1158). The path is the oracle list (1547-1548).
5. Tooltip and text: fixed. Design 4.1:364, 9.1:773, 9.3:818 (forge "Cannot move").
6. Range sign: fixed. Design 9.1:775. I still see no shot (MP1 scope, decided).
7. Resource line: fixed. Design 12.1:1008-1009, 12.3:1032.
8. PT2: fixed with a new defect (N3). Rings now use the footprint radius (771-772). Camera on the troopers (12.3).
Double tap: fixed (15.3:1221-1222).

## New findings

N1. Design 9.4:834, path:1562. High (instrument). "Research ready" fires only when the selected producer changes from disabled to enabled. If I select the belt while the lab fills, I get no alert. If the lab is already full at my first click, I also get no alert. The oracle then fails step 7 on a correct build. Fix: fire on the lab itself, not on the selection, or drop the alert from the oracle.

N2. Design 10.5:916-918; `crates/turts_server/src/net.rs:36-37`; design:1495. Medium. Step 15: I reload my only window. The old session can stay alive for up to 10 s, so I can read "This game is open in another window". That is false, and I do not know what to do. I want the game to take me back in, or tell me to wait.

N3. Design 9.1:787 against PT2 (1159). Medium. The marker goes after 10 s. Troopers at speed 3 that travel more than 30 units rest after the marker is gone, so I cannot judge "near the marker".

The 60 s rule: window A can show "0/10" for about 40 s at the start (NOT TRACED: which arm loader gets the shells first). No other step waits over 10 s.

## What I ran

Play log: `/tmp/claude-1000/-home-julien-Projects-TuRTS/0e0457bb-045c-4b6a-945f-1db6a0753ee2/scratchpad/mc/gate1-player-recheck-playlog.md`.

| run | claim | refute | observed |
| --- | --- | --- | --- |
| read 17.1 and the eight fix sites | 17.1:1300-1306 | a fix missing | 8 present |
| paper play of steps 1-19 with the map | PT1 passes | a step that cannot be done | step 7 at risk |
| read the trigger at 834 | path:1562 | the trigger is on the lab | it uses the selection |
| grep server timeout | 10.5 rejoin | a fast close | 10 s token |
| marker line 787 against 1159 | PT2 marker | no time cap | 10 s cap |

Not checked: P1 keys; arm loader order.

## What I changed

none
