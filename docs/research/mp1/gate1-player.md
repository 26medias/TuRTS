# MP1 gate 1: player lens

Source: subagent hand-back, MP1 brainstorm input, 2026-11-11. Agent output, not reviewed; claims are leads until verified.

VERDICT=PASS-WITH-FINDINGS FINDINGS=11 HIGH=4 RUNS=6 NOT_CHECKED=4

The game has all four parts. I control the units. I want the enemy command centre. Turrets, power and credits stop me. The match ends with a win or a loss. The loop works on paper. But the first five minutes repeat three parts of the MC failure, and those parts are not yet fixed in the design text. Do not close gate 1 until the four High findings are fixed. The full play log is at /tmp/claude-1000/-home-julien-Projects-TuRTS/bf418660-b272-4bb3-8628-8d9ef2677d1d/scratchpad/mp1-player/playlog.md. The run output is in play.out and fights.py in the same folder.

**Would I play a second match?** Yes, if the High findings are fixed. The best moments are real. One example: I kill the enemy power station and their turrets go dark. Another: three launchers catch the enemy hauler. As written, I would stop at about minute 2. I would be waiting for 200 credits and hearing "Not enough credits" again and again.

## Findings

1. **High. The hauler starts idle and the hints do not name it** (design 676, 664; `crates/turts_mod_api/src/schema.rs:440-447`). A map placement has no order field, so the hauler waits for me. Step 6 comes after the turret ghost. At tick 0, two warnings sound ("Low power", "Harvester idle"), and I have done nothing yet. In S3 (I never find the hauler), every job is on hold from 73 s until the end. If I send the hauler at 50 s, the first "+200" comes at 81 s. If it starts at 0 s, the first "+200" comes at 31 s. Needs decision. Option (a): the hauler gathers from tick 0. Option (b): the first hint is "Send your Ore Hauler to the ore", and the power station comes second.

2. **High. Only the first hint is written** (664, 669; GOAL1 376). "The next hint shows", but no list of hints exists in the design or in the inputs. From 14 s to 38 s the power station builds at half speed. If the hint stays until the station is complete, I see "Build a Power Station" after I already built one. Fix: write the ordered hint list into the Player path, with the event that starts each line and the event that ends it. Placing the station must change the hint to a parallel action, for example "Train a Rifleman: press I".

3. **High. "Not enough credits" comes back at each delivery** (ALR2 381, 429). With one hauler, all jobs are on hold from about 75 s. Each delivery of 200 clears the state, and about 5 s later the state starts again. That gives one sound every 20 to 36 s (7 hold spans in S1, 13 in S4). This is the "beeps non stop" problem again. Fix: add a red build to ALR2 where the state clears and comes back at each delivery. The sound must not repeat inside a window that the mod sets.

4. **High. Placement reasons describe a wrong cause** (7.2 line 244; 6.9 line 217; GD 152-153). The station is valid only 6.0 to 12.5 from the CC centre. Where only the 2.0 gap blocks the site, the ghost says "Something is in the way", but I see nothing there. A site 1.9 from ore says "Cannot build on ore". The rings that show the build area (GD 153) are gone from the design. Fix: add the reason "Too close to a building" and a ring that shows the gap, change the ore text to "Too close to ore", and bring back the build-area rings, each with a red build in GHO1.

5. **Medium. Losing my only hauler loses the game, with no warning** (ALR2 381). After 75 s my bank is 0 to 200 credits. A new hauler costs 800, so its job stays on hold for ever. Three launchers kill a hauler in about 8 s, and a raid can land at about 90 s. Fix: add "Harvester under attack" to ALR2.

6. **Medium. The screen does not show who beats whom** (HUD6 382; 6.3). A bullet does 2 damage per shot to a tank or a building, which is 0.4 % of a tank's health. If I send riflemen at a tank, I think the riflemen are broken. Fix: add a role line to each tooltip in HUD6, for example "Strong against tanks", and a smaller hit effect when the hit is reduced (FX1).

7. **Medium. The tooltip time is wrong under low power** (HUD6). The tooltip says 12 s, and the first building takes 24 s. Fix: the tooltip shows the time at the current power.

8. **Medium. The goals text gives away the loop and uses other words** (391-395). "Get power, collect credits..., build a turret" tells the testers what the hints must teach them, so the hints are not tested. The goals text says "harvester", but the screen says "Ore Hauler". MC failed on this kind of word difference. Fix: keep only the goal and the test rules, and use the words on the screen.

9. **Medium. The end screens say too little** (703-709). After the result screen, the only way back is "Reload this page", and there is no "Play again". A win because the other player left shows only "Victory", with no reason. Fix: RES1 shows the reason ("Player 2 left", "Player 2 surrendered"). Needs decision: a rematch button in MP1.

10. **Low. Nothing makes the players meet before about 11 min.** With two haulers, my own fields last 11.4 min, so neither player needs the centre ore before then. If both players build defences, the ore runs out at about 20 to 25 min, and only surrender ends the match. I record this for the duel scenario in section 14. I propose no change.

11. **Low. Steps 18 and 19 need a second and a third match** (706-710), but the goals text does not ask for them. Fix: write a separate instruction for each, to give after the blind match.

## Answers in short

1. The first 60 s are 0 s warnings, 14 s place the station, then 24 s of nothing to do. Power first is a fair first lesson in placing a building, but only if money or a unit fills the wait.
2. With one hauler, the first wait starts at 73 s and each wait lasts up to 31 s. A tank takes 88 s from order to done. I choose when the first fight happens. The earliest end is 4 to 5 min with two tanks. A turret (500 plus 150 of a station's power) needs two tanks (1,400) to break it, so defence is about twice as cost-efficient. 10 to 20 min is possible when one player attacks. Two defending players can stall.
3. There is no comeback after I lose my hauler (finding 5). Yes, there is a reason to attack early: raid the hauler.
4. Power and the two queues are clear. The gap and armour rules are not (findings 4 and 6).
5. Three changes would help the feel most for little work: a sound with the "+N" on each delivery (it is not in the P5 sound set), a small "plink" for a reduced hit, and a building that rises while it builds.
6. See finding 9.
7. See finding 8.
8. There is no pause and no speed setting. For two players that is acceptable. A minimap, a surrender, cost tooltips and a click sound (MC ORD7) are present. Missing: build-area rings, "under attack" alerts for units, a rematch, a sell order.

## What I ran

| command | claim tested | result that would refute | observed |
| --- | --- | --- | --- |
| play.py, six scripts (exit 0) | GD 117: 1,200 credits pays for power plus one real choice. Design 110: 330 credits a minute | no hold spans with one hauler | holds from 73 s; up to 31 s each |
| play.py S3 | Design 676: the tester sends the hauler | the hauler earns with no order | no credits for 300 s |
| fights.py (exit 0) | Design 107-109: the balance ratios | turret→tank more than 28 s | 18 s; bullet against a vehicle does 2 |
| fights.py, ore count | GD 114: the mid-game moves to new fields | own fields last less than 8 min with 2 haulers | 11.4 min |
| grep of the design for 12 words (pause, rematch, Too close, Strong against and others) | Design 7.2: the reason texts and rings are complete | a gap reason or a ring present | none |
| grep of schema.rs Placement | the hauler can start with an order | an order field | none (440-447) |

Six runs: the four script runs and the two greps above. `git status --porcelain` in TuRTS-mp1 was empty.

## Not checked

- The ALR2 behaviour at tick 0 in code: NOT TRACED. I used the design text.
- The engine measure of 6.9 (about 40 lines): NOT TRACED.
- Whether two haulers on one node slow each other: NOT TRACED.
- Sound and feel: these are paper only, because there is no build to play.

## What I changed

none

## Open questions for the orchestrator

- Finding 1 needs a decision: the hauler starts on its own, or the first hint names it.
- Finding 9 needs a decision: a rematch button in MP1.
