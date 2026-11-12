# Paper play of the MP1 design rev 3 (player seat, gate 1). Numbers: input-game-designer.md. Runs: play.py -> play.out, fights.py.

## First 60 s (design 661-670, GOAL1 376, ALR2 381)
- 0 s: "Click to play". Camera on my CC, CC selected. Top line: "Destroy the enemy command centre". Credits 1200. "Low power" (0/40).
- 0 s: ALR2 says a forced state gives one line and one sound. Low power and "Harvester idle" are both true at tick 0
  (the hauler has no start order: schema.rs:440-447 Placement has no order field; design step 6 has the tester order it).
  So the first two sounds I hear are warnings, before I touch anything.
- 2-8 s: I read the hint "Build a Power Station: press P, then click near your command centre." I press P.
- 8-14 s: I click close to the CC. Valid only 6.0 to 12.5 from the CC centre (2.5 + 1.5 + gap 2.0; build distance 10 from edge).
  Closer: "Something is in the way". I see nothing in the way. I try once more, farther. It works.
- 14 s: site shows. Progress bar. The tooltip said 12 s. It takes 24 s, because I have low power (the station itself is slowed).
- 14-38 s: dead time. The hint text after placement is not written in the design. The units queue is free but nothing names it.
  If the hauler waits for my order, no money comes. 24 s of watching a bar.
- 38 s: "Construction complete" + sound. Low power goes. "The next hint" - its text is not in the design.
- First money: +200 at 31 s only if the hauler starts at 0 s. If I find the hauler at 50 s (step 6 order), first +200 at 81 s.

## First 5 minutes (play.out)
- S1 (hint follower, hauler sent at 50 s; PS, 3 riflemen, turret, tank, 2 riflemen, 2 launchers):
  on hold from 73 s on, 7 hold spans of up to 31 s; tank queued at 70 s, done at 158 s (18 s unit, 88 s wall).
- S2 (same, hauler from 0 s): 200 more harvested; tank done at 108 s; same hold rhythm.
- S3 (hauler never sent): on hold from 73 s to the end. Silent soft loss.
- S4 (second hauler first): holds of up to 23 s, 13 hold spans in 250 s.
- One hauler = 200 credits every 36 s. After about 75 s, "On hold: no credits" is the normal state.
  Each delivery clears "no credits", the next job empties it in about 5 s, the state comes back:
  under ALR2 that is one "Not enough credits" sound every 20 to 36 s for the whole match.

## Fights (fights.py)
- bullet vs vehicle or structure = 2 per shot. 1 rifleman needs 999 s for a CC, 249 s for a tank. Health bar moves 0.4 % per hit.
- tank -> CC 98 s alone; 2 tanks about 50 s. turret -> tank 18 s; turret -> rifleman 4.5 s; tank -> turret 28 s.
- Hauler: 3 launchers kill it in about 8 s; 750 credits, built by 42 s, walk 80 at 2.0 = 40 s: a raid lands at about 85-95 s.
- Earliest end: 2 tanks done at 111 s (S5), walk 31 s, kill a lone turret, then CC ~50-100 s: about 4-5 min.
- Ore per slot: own fields 7500, centre 16000 shared. Own fields last 22.7/11.4/7.5 min with 1/2/3 haulers.
  If both players turtle, the ore ends at roughly 20-25 min and only surrender ends the match.

## Hauler loss
- Bank in S1/S2 after 75 s is 0-200. A new hauler is 800, paid over time: "On hold" forever. No sell. No warning.
- ALR2 list has "Base under attack" and "Harvester idle", not "Harvester under attack" or "no harvester".
