VERDICT=fail FINDINGS=7 HIGH=2 RUNS=8 NOT_CHECKED=2

MCD = `docs/superpowers/specs/2026-10-29-mc-player-control-design.md` (revision 2).

## Original findings

1. PT1: fixed with a new defect (High). The goals (MCD:1158) do not ask for a reload, a queued move or a double tap. The oracle still wants them (MCD:1573, 1577, 1578).
2. GM40: fixed with a new defect (High). The rules hold now: the reach is 1.05 and the spacing is 1.1 (MCD:568, 587, 602). But the screen check still compares `turts_scene` rects (MCD:1135, 1067). At exact slots, 32 to 64 rect pairs overlap.
3. Done rows: fixed with a new defect (Medium). EC02 stays `done:` (MCD:1454), but its dependency PF01 is now `later:M4`. OR17 leans on EC02.
4. Cut line: fixed (MCD:1228-1235). Low: slip 1 also breaks step 18 (the minimap ping, MCD:1584).
5. Assumes column: fixed (MCD:1118).
6. F-G1, D39 note, queue cap: fixed (K24).
7. macOS: not fixed (Low). MCD:213 and 326 say "not an MC target", but MCD:75 says Open. My S:759 citation was wrong; the correct line is S:751.
8. Terms and STE: fixed (MCD:554, 900). Long sentences fell from 33 to 7. Passive sentences rose from 18 to 26.

Open questions: SLOT1 now tests invariant 11. GMA still counts ticks (MCD:1136; Low).

## New findings

- MCD:1206. Medium. The text says "these six take 30 runs", but it lists seven tasks with 36 runs. The longest Needs path is 3, 8, 9, 12, 15, 16, 21, 23, 24: 45 runs.

## What I ran

| command | claim | refutes | observed |
| --- | --- | --- | --- |
| `--enforce` | gate green | exit 1 | exit 0 |
| `--self-test` | 21/21 | a fail | 21/21 |
| deps.py | MCD:1344 counts | other counts | match; EC02 has a later dependency |
| gm40_r2.py | GM40 can pass | overlap | discs pass, rects fail |
| path script | MCD:1206 | not 30 | 36; 45 |
| budget sum | MCD:1195 | not 104 | 104 |
| ste_r2.py | MCD:1282 | 33 or more | 7 |
| grep | items 6 and 7 | hits | 213, 326, 1136 |

Not checked: the code citations in K19 to K27, and the model runs at MCD:1345-1348 (NOT TRACED).

## What I changed

none
