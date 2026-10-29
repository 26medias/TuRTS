VERDICT=fail FINDINGS=8 HIGH=3 RUNS=12 NOT_CHECKED=3

MCD = `docs/superpowers/specs/2026-10-29-mc-player-control-design.md`. Evidence: `/tmp/claude-1000/-home-julien-Projects-TuRTS/0e0457bb-045c-4b6a-945f-1db6a0753ee2/scratchpad/mc/gate1-rigour-evidence.md`.

## Findings

1. MCD:955, 1286. High. The PT1 tester follows the Player path script. S:708 allows only the build, the guide and the pixels, so this test is not blind. Fix: give the tester the goal only.
2. MCD:935, 477, 489, 497. High. GM40 goes red on a correct build. The screen rects of neighbour units overlap (32 to 65 pairs). A 0.05 arrival tolerance on a 0.05 gap allows overlap. The separation range (1.1) is larger than the slot spacing (1.05). Fix: test projected discs. Make the spacing larger than the range.
3. MCD:1199, 1217, 1241. High, needs decision. The cited files do not prove CB03, PF01 or MP09. Their dependencies are `later` rows. When I set the three rows to `later`, R4 gave five reds. PC16 (MCD:1115) is `later`, but SEL1 tests it. Options: change the yardstick dependencies, or move the five rows out of MC.
4. MCD:1011, 1018. Medium, needs decision. The cut line can drop S:722 items: the minimap, hold, attack and attack-move. No call covers this. Slips 1, 3 and 5 break Player path steps 6, 8, 9, 11, 16 and 18.
5. MCD:929. Medium. The rows assume the recommended options. Under K4(a), ORD2 fails on a correct build. ORD4, GMU and HUD3 need K5(a), K6(a) and K12(a). Fix: tag each row with its option.
6. MCD:1053, 1040, 163. Medium. F-G1 is stale: the self-test gives 21/21 at 7a90f97. MCD:88, 367 and 442 cite a "D39 note", but S:68 has no note. The queue cap of 16 has no call.
7. MCD:820-821. Medium. The text gives two rules for when the K3 fallback applies. F10 and Esc open two screens (MCD:276, 730). MCD:290 decides the macOS target, which S:759 keeps Open.
8. MCD:467, 752, 354. Low. MCD:467 uses "input" for a function argument, against S:21. "Mod major version" has no meaning for 0.x versions. 33 sentences have more than 25 words.

## What I ran

| command | claim | refutes | observed |
| --- | --- | --- | --- |
| `completeness.py --enforce` | coverage is green | exit 1 | exit 0 |
| `--self-test` | F-G1, MCD:1053 | 21/21 | 21/21 |
| deps.py | done rows, MCD:1069 | no later deps | 3 have later deps |
| planted `later` rows | done rows carry R4 | exit 0 | 5 reds |
| test names | done evidence | path test | none |
| gm40.py | GM40, MCD:935 | no overlap | overlap |
| `sed` spec lines | citations | text differs | match |
| grep D39 | MCD:367 | note found | none |
| term grep | MCD:467 | no hit | hit |
| ste.py | STE | 0 long | 33 long |
| grep forge | MCD:752 | major 1 | 0.1.0 |
| budget sum | MCD:990 | not 97 | 97 |

## Not checked

- The code citations in K1 to K18. Outside scope.
- The harness against QA0D. Outside scope.
- Invariant 11 (MCD:278). No test row names it. NOT TRACED.

## What I changed

none

## Open questions for the orchestrator

- Which yardstick dependencies change? Who owns the change?
- Can MC drop S:722 items?
- Does D39 permit the queue cap and the rate limits?
- Which version part keys the stored bindings of 0.x mods?
- Is macOS tested in MC?
- The tick rate is Open. Can the tick limits use seconds?
