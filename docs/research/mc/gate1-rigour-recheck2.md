VERDICT=pass-with-changes FINDINGS=3 HIGH=0 RUNS=8 NOT_CHECKED=1

D = `docs/superpowers/specs/2026-10-29-mc-player-control-design.md` (revision 3).

## Items

- PT1: fixed with a new defect (Medium). No goal asks for the scroll speed (D:1628) or the drill tooltip (D:1608). D:1190 says each step follows from a goal.
- GM40: fixed (D:1166; fields at `crates/turts_client_web/src/lib.rs:415-416`). Probe: 0 overlaps; 780 with spreading off.
- EC02/OR17: fixed (D:1446, 1506).
- Step 18: fixed (D:1264). Low: slip 5 also breaks step 16.
- macOS: fixed (D:75, 215, 330).
- Critical path: fixed (D:1238): 46 runs.
- GMA: fixed (D:1167).
- Passive voice: fixed: 26 to 15.
- DET1 (D:1169): it goes red on `HashMap` order. No run shows it red (NOT TRACED). The plan must show it red first.
- EL1 (D:1170): it goes red on a write outside the movement function and on today's validator. A defect inside the function passes, as D:103 says.

## What I ran

| command | claim | refutes | observed |
| --- | --- | --- | --- |
| `--enforce` | gate green | exit 1 | exit 0 |
| `--self-test` | 21/21 | a fail | 21/21 |
| deps.py | honest done rows | later dependency | none |
| gm40_r3.py | GM40 can go red | other counts | 0; 780 |
| path script | D:1238 | not 46 | 46 |
| ste_r2.py | D:1351 | 26 or more | 15 |
| grep bridge | D:1096 | absent | present |
| grep goals | D:1190 | none missing | 2 |

Not checked: the model runs at D:1395-1400.

## What I changed

none
