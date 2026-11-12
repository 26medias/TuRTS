# MP1 design revision 5 re-check: player

Source: subagent report. Agent output; claims are leads until verified.

VERDICT=PASS-WITH-FINDINGS HIGH=0

This is the re-check of revision 5. No finding blocks gate 1 for the player lens; the plan can take the Medium and Low items. I cannot play MP1 because it is not built, so everything below is on paper.

**Closure list (revision 5 lines)**
- N1 CLOSED. The stage-1 hint data leaves out H3 (1989-1992, 2932). GOAL1 plant (f) goes red if H3 is put back (2436).
- N2 CLOSED. A new Ore Hauler gets a spawn order to the nearest ore (700-715, row U8 1502, 300). A new alert `GathererIdle` fires when a hauler has no ore order (1557, 2040).
- N3 CLOSED. Each hint that needs the Command Centre now says "select your Command Centre" (1983-1985, 2930-2940). The new hint X4 shows when the Command Centre is not selected (2941). GOAL1 has a hint driver that starts with a click on open ground, and plant (g) tests it (2436).
- M1 CLOSED. There is a 4 s gap between alert sounds, by priority. Each family has its own sound. "Unit ready" sounds only when the queue becomes empty (2011-2025, 2034-2045).
- M2 CLOSED. The role line and the cost line are in stage 1 (2049-2051). The reduced-hit effect stays in stage 2, and that is acceptable.
- L1 CLOSED. Stage 1 starts with `AllSeated(0)` (815-830, 1901-1902, C1 2332-2333).
- L2 CLOSED, as a recorded limit. C1 now has an end instruction (2336-2339).
- L3 CLOSED. The stage-1 cancel and refund rule is stated (648-655).
- Gate-1 item 5 is now CLOSED, because the spawn order fixes the replacement hauler. Gate-1 item 10 stays PARTIAL (Low, a known limit).

**Stage-1 walk (C1, a person who read nothing; times from hold5.py and sound5.py)**
- Before the start: window A shows only "Waiting for the other player". When window B opens, both show "Click to play" in the same frame. The match runs from that frame, even if I have not pressed yet. That is acceptable.
- 0:00: My Command Centre is selected. I see the goal line, H1 "Build a Power Station: select your Command Centre, press P, then left-click inside the ring around it.", "Credits 1200" and "Low power: 40 used / 0 made". Nothing sounds.
    - If I do nothing for 30 s, my hauler drives out and "+300" shows at 29.7 s, with no sound in stage 1. That is good.
- If I click the ground first: X4 shows "Click your Command Centre: its build menu shows the next step." I click the Command Centre and H1 comes back. I get through.
- 0:08: I press P and click. H2 shows in the same frame: "While it builds, train a Rifleman: select your Command Centre and press I."
- 0:14: I press I. H4 shows: "Train a second Ore Hauler for more credits: select your Command Centre and press O." I press O. The hauler is done at 0:45 and goes to the nearest ore by itself. I get through.
- 0:15 to 0:32: F shows: "Train more units while you wait: ...".
- 0:32: "Power Station complete." shows, with a sound. H5 shows: "Build a Guard Turret near your ore field: select your Command Centre and press T."
    - STOP 1: my nearest ore is 14.1 to 15.1 from my Command Centre (GD 224-226). The build ring is 12.5 (1755). If I put the turret "near the ore field", I get "Too far from your base". I can recover, but it costs me time.
- After H5, F shows again until I have 4 units. Then H6 asks for a box, then A, then a click, and H7 follows. The first fight comes at about 2:15.
- If I follow F right away (at about 0:40), my jobs hold for 7 s with 3 Riflemen, or for 14.5 s with a tank. "Not enough credits" sounds once and X3 explains it. That is not a stop. But section 5 says "no hold above 4 s", and that was measured on the revision-4 hint order, not this one.
- If I do nothing for 30 s at any step, nothing sounds and nothing gets worse.

**Full build (stage 2) differences**
- The lobby lines L1 to L5 replace the one waiting line. H3 (the rally) comes back between H2 and H4.
- The delivery sound, the reduced-hit effect and the build time at low power come back.
- The new problem is in stage 2 only: a hauler with a ground rally (see N-M1).

**Sounds (sound5.py, with the 7.9 numbers)**
- Quiet run: 2, 1, 2 and 1 alert sounds in minutes 0 to 3.
- The stage-2 delivery sounds (1 to 4 a minute) are not alerts, so the gap does not hold them back.
- Fight minute: 12 alert sounds are due, 9 play and 3 drop. The gap allows at most 15 a minute.
- Weapon sounds (at most 8 at once) and error sounds are also outside the gap.
- I judge this as not noise. Each sound now means a different thing.

**Rally judgement (question 4)**
No, this is not what I expect. I put a ground rally at the front for my army. I expect a new hauler to go to work, not to walk to the front. Revision 5 makes it gather from "the node nearest to it" after the flag (711). That can be a centre field, 43.9 to 47.4 from both Command Centres (GD 230), next to the enemy. A rally on ore is fine.

**New findings**
- N-M1 Medium (stage 2; 709-712, 1819). A new hauler follows a ground rally to the front, then gathers near the flag.
    - Smallest change: a new Ore Hauler ignores a ground rally and does its spawn order at once (`frontier` `rally` rule).
    - Or: after the flag, it takes the ore nearest the Command Centre.
- N-L1 Low (2934). H5 sends me to "near your ore field", and that area is outside the ring.
    - Change the text to "Build a Guard Turret inside the ring, on the side of your ore field: ...".
- N-L2 Low (section 5 S7; hold5.out). The pacing model was not rerun for the stage-1 hint order.
    - Rerun it, or start H4 only when the Power Station is complete.
- N-L3 Low (2941). In X4, "its build menu shows the next step" is not quite true: the menu shows buttons, not the next step.
    - Change it to "Click your Command Centre to build and train."

**What I ran**
| command | claim tested | result that would refute | observed |
| --- | --- | --- | --- |
| sound5.py | 2016: at most 15 alert sounds a minute | more than 15 play | 9 in a fight minute; 1 to 2 in a quiet minute |
| hold5.py | section 5: no hold above 4 s | a longer hold on the stage-1 order | 7.2 s and 14.5 s when I follow F at once |
| distance check | 2934 H5 can be done near the ore field | ore inside 12.5 | 14.1 to 15.1 |
| grep and read of the diff | the N2 and N3 repairs | a rule or text missing | present |

**Not checked**
- Whether a box drag (H6) also picks up the Command Centre: NOT TRACED. If it does, the attack order may be partly refused.
- The ore positions in revision 5: I used GD 224-226; NOT TRACED in the design itself.

**What I changed:** none.

The scripts and outputs are in /tmp/claude-1000/-home-julien-Projects-TuRTS/bf418660-b272-4bb3-8628-8d9ef2677d1d/scratchpad/regate-player/:
- sound5.py
- sound5.out
- hold5.py
- hold5.out
- r5.diff
