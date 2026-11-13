# MP1 design revision 4 re-gate: player

Source: subagent report. Agent output; claims are leads until verified.

VERDICT=PASS-WITH-FINDINGS HIGH=3 FINDINGS=8 RUNS=7 NOT_CHECKED=4

The repair is real. Revision 4 closes my four gate-1 High findings with true changes, and I checked each one at its line. There are three new High findings, and each one stops a new player in the first two minutes. Do not close gate 1 until they are fixed. Each fix is small: hint data or a text change, or one rule for new units. I cannot play MP1 because it is not built. Everything below is on paper. I used the repair-pass model and my own scripts in /tmp/claude-1000/-home-julien-Projects-TuRTS/bf418660-b272-4bb3-8628-8d9ef2677d1d/scratchpad/regate-player/ (walk.out, pacing-new.out, match.out, ids.out). What I changed: none.

## My gate-1 findings
1. The hauler starts idle. CLOSED. Decision D-F (a) adds a start order for a map unit (design 155, 237, 280). The STO1 test checks it (2053), and the GOAL1 test fails if the start order is missing (2052). Nothing sounds at the start (1698-1699).
2. Only the first hint was written. CLOSED. The hint list H1 to H7, F and X1 to X3 is in the Player path, and each hint has a start and an end (2456-2468). Placing a site moves the hint on at once (1681-1684). H2 now gives a second task while the station builds.
3. "Not enough credits" sounded at each delivery. CLOSED. This alert now sounds once when the state starts, then waits 90 s before it can sound again (1698-1702, 1716). ALR2 forced case A tests this (2059). My rerun with the 90 s and 15 s values of the 7.9 table gives the same holds as the design.
4. Placement gave the wrong reason. CLOSED. The new texts are "Too close to a building" and "Too close to ore". Rings show the build area, the gap and the ore clearance (1481-1498). GHO1 plants (a), (b) and (c) test them (2054).
5. Losing my only hauler lost the game. PARTIAL. The alerts "Ore Hauler under attack" and "Ore Hauler destroyed" exist (1718-1719). Hint X1 tells me what to do, and the free income is a floor. But the replacement hauler does not gather (new finding N2).
6. I could not see who beats whom. CLOSED in the design (1726-1736). Both parts come in stage 2 (2021-2022; see M2).
7. The tooltip time was wrong under low power. CLOSED (1737-1738, HUD6 plant a). This also comes in stage 2.
8. The goals text gave away the loop. CLOSED (2077-2084). The word "harvester" no longer shows on the screen (264).
9. The end screens said too little. CLOSED. Result reasons are in 1642-1647. Decision D-K (a) is recorded. The "Leave" screen text is in 1652-1653.
10. The players had no reason to meet. PARTIAL. The build area is now around the Command Centre only (289). But two equal careful players still reach no end in 40 min in the match model (317). In stage 1 there is no surrender.
11. Matches 2 and 3 had no instructions. CLOSED (2085-2091).

## The walk: a person who read nothing (full build; times from S7, the hint order)
- 0:00. I see my Command Centre, already selected. I see the goal line, hint H1, "Credits 1200" and "Low power: 40 used / 0 made" (2472-2475). Nothing sounds. If I wait 30 s, the hauler drives to the ore and "+300" shows at 29.7 s. That is good.
    - BAD B1: I click the hauler or the ground first, as people do. The Command Centre is no longer selected. Now P does nothing, and nothing tells me why. N3.
- 0:08. I press P. The ghost, the rings and the reasons show. I click, and H2 shows in the same frame. The station takes 24 s. Good.
- 0:14. I press I. The Rifleman is done at 0:28. H3 shows: "right-click the ground: new units walk there."
    - BAD B2: in stage 1 there is no rally, because X9 is stage 2 (227, 1973). The hint list can never pass H3. Each time I try, I get a refusal, probably with the error sound. N1.
- H3, then H4. I press O. My second hauler is done at 0:46. It walks to the flag and stands there.
    - BAD B3: it earns nothing, and no alert tells me. N2.
- 0:32. The Power Station is complete, so H5 starts. I place a turret, done at 1:10. Then hint F shows until I have 4 units, at 1:39. Then H6: attack-move. My units walk 33 s, and the first fight comes at about 2:15. H7 shows for 30 s. After that, only the goal line shows. That is acceptable.
- If I do nothing at any step, nothing sounds and nothing gets worse. The hint stays. That is good.

## New findings
- N1 High (2460; 227; 1677-1679). The stage-1 hint list stops at H3 because there is no rally in stage 1. H4 to H7 never show, and hint F never shows. GOAL1 checks only H1 to H2 (2052).
    - Smallest fix: leave H3 out of the stage-1 hint data.
    - Add to GOAL1: a test driver that does only what the hints say reaches H7. Add a plant where H3 never ends.
- N2 High (1541, 1720; `crates/turts_sim/src/economy/transaction.rs:710-719`, `gather.rs:237-239`, `crates/turts_client/src/feedback.rs:821`). A new hauler starts with no ore order and does nothing. "Ore Hauler idle" fires only when a hauler that already has an ore order waits. So a hauler from H4, from X1, or with no rally sits still, and nothing tells me. The pacing model assumes it gathers at once (pacing.py:82).
    - Smallest fix: a new Ore Hauler gathers from the nearest ore on its own. Use mod data, the same mechanism as the D-F start order.
    - Make "Ore Hauler idle" also fire when a hauler has had no ore order for 5 s.
    - Add a test: a new hauler with no rally, and one with a ground rally, both deliver.
- N3 High (2458; `crates/turts_client/src/widgets/sources.rs:372-379`). The build keys work only while the Command Centre is selected. H1, H2, H4, H5 and F say "press X" but not "select". H3 is the only hint that says "Select your Command Centre".
    - Smallest fix: start each of these texts with "Click your Command Centre, then press X". Or add a situation hint that shows when the current step needs the Command Centre and it is not selected.
    - Add a GOAL1 plant: the driver clicks the ground first.
- M1 Medium (1712-1722). Nothing limits the total number of alert sounds. A quiet minute in S7 has up to 9 sounds. A fight minute has up to 24 alert and screen sounds by the cooldown arithmetic, plus weapon sounds. 7.9 does not say that the alerts use different sounds. MC failed on one sound shared by many rules.
    - Fix: give "Unit ready" a line with no sound, or a sound only when the queue is empty.
    - Add a gap of a few seconds between any two alert sounds, by priority.
    - Name one sound for each family of alerts.
- M2 Medium (2021-2022; 10, C1 1967). C1 asks a person to judge fights, but stage 1 has no role lines, no cost tooltip, no reduced-hit effect and no sound set.
    - Fix: move the role line and the cost line of the HUD6 tooltip to stage 1. They are text only.
- L1 Low (1965). Stage 1 starts at once (`Immediate`), so the match runs before anyone joins. A late window misses the first "+300", and the second window joins later.
    - Fix: C1 opens both windows before the server starts.
- L2 Low (2081, 317). Two equal players can stall. In stage 1 they cannot surrender, so the 40-minute stop ends with no result screen. I record this; I propose no change.
- L3 Low (2466, 2468; 2021). Hints X1 and X3 tell me to cancel a job. But cancel with a refund of what the job paid is stage 2 (PAY2). The stage-1 refund: NOT TRACED.

## Stage 1: enough to play?
Yes, after N1 to N3 are fixed. A person can build power, harvest, train, fight, win and see the reason. Without N1 the person is stuck at minute 1, the same way as in the MC play. M2 holds back the parts that a person needs to judge fights.

## Sounds
At most 1 "Not enough credits" sound in 300 s, even in the greedy script S8. The repair fixed that MC failure. The new risk is the total: 9 to 24 sounds in a minute (M1).

## Arithmetic (walk.py, pacing.py new, match.py; all exit 0)
- First delivery 29.7 s. A trip takes 34.7 s, so one hauler earns 519 credits a minute.
- First Rifleman at 28 s. The station is complete at 32 s.
- First tank: 35.5 s in a rush (S5), 98.7 s in the hint order (S7).
- The two Command Centres are 80 apart. Walk times: tank 31 s, Rifleman 33 s, Launcher 40 s.
- First contact: about 67 s in a rush, about 2:15 in the hint order.
- A tank does 25 damage a second to a building (40 x 125 % every 2 s). One tank kills a Command Centre (3,000) in 120 s, three tanks in 40 s. "Base under attack" sounds at the first hit, so the warning is fair.
- Holds: S1 none, S4 none, S7 one of 4 s. S8 holds up to 29.9 s, with 1 sound.
- Match length: 3.5 to 20.2 min when the players are not equal. Two equal careful players reach no end in 40 min (313-317).

## What I ran
| command | claim tested | result that would refute | observed |
| --- | --- | --- | --- |
| pacing.py new | 302-306: no hold above 4 s; 1 sound | a hold above 4 s in S1, S4 or S7 | same as design |
| walk.py | 1716 values (90 s, 15 s) | different holds or sounds | same |
| match.py | 313-317 match lengths | different end times | same |
| ids.py | 1465-1466, 1710-1722 | a Player path hint or alert missing from 7.8/7.9 | none missing; ORD4, SH1, SLOT1 are MC rows |
| grep feedback.rs, gather.rs, transaction.rs | 1720 "Ore Hauler idle" | the trigger counts a hauler with no ore order | only waiting haulers (N2) |
| grep sources.rs:372 | 2458: P works from the start | build keys work with any selection | Command Centre only (N3) |
| grep design for rally and stage | 2460 H3 | rally in stage 1 | stage 2 (N1) |

## Not checked
- Where the hint line sits on the screen: NOT TRACED.
- Which sound file the stage-1 alerts play: NOT TRACED.
- The stage-1 cancel refund: NOT TRACED.
- Sound and feel: paper only, because there is no build to play.

## What I changed
none

## Open questions for the orchestrator
- N2 needs a decision: a new hauler gathers on its own, by mod data (recommended), or by an engine default.
- Does stage 1 use the MC cancel refund rule under pay-over-time?
