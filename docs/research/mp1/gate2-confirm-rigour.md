# MP1 plan revision 3 confirmation: rigour (gate 2 closes)

Source: subagent report. Agent output; claims are leads until verified.

VERDICT=PASS-WITH-FINDINGS HIGH=0

The patch closes all four Highs, and my schedule and coverage scripts agree with revision 3. I found no new High. Head b62d2ff; the slice head the plan cites is d7d7e6d.

a. The four Highs are closed. Each has a check that can go red.
- My H1 (`held`): CLOSED. E21 is consistent across D 6.4 :637-642, the 6.8 `SiteProgress` and job records, H14 `JobOnHold`, 7.3, and PAY1 (the 1-in-12 case and plant (6)). HUD5 still reads correctly. 6.7.13 holds no job record, so no change is due there. P:464 and P:711 (`L0-WIRE`) and P:692 (`L0-SIM`) list `held` among the frozen shapes. `L-PAY` has `L0-EXIT` (offset 12) as a plain input, so the field is frozen before `L-PAY` starts.
- qa N1 (GPU lock queue): CLOSED. `Q-LOCK` self-check cases (a) to (d) start at `exit=1`. The plant "remove `timeout`" turns case (a) red.
- TD H-B (store and resync): CLOSED. `L0-STORE` has three resync cases and plant (c), "store applied per message", which turns case (1) red.
- TD H-A (cross-branch stacks): CLOSED. The edge is defined in section 1, D E24 and the slice amendment. The script rule "a cross stack must have a row in `mp1-tree`" is a check that can go red.

b. My Medium and Low items:
- M1 is CLOSED: E22 gives the `LabelStyle` fields and defaults, and the `C-UI` RON at P:1073 has all of them.
- M2 is CLOSED: the `C-UNITS` plant is now the misspelt field `footprnt` (P:1065), and the K2 plant moved to `C1-FINAL`.
- L1 is CLOSED (E25).
- L2 is CLOSED: D-B is due at 3, D-F at 14, and P8 is now kind C.
- L3 is CLOSED: the `mp1-moves` block now carries the D 6.1 stages, and the errata count is fixed (P:1694).
- L4 is CLOSED: n1 and n3 now go RED (my p1 and p4). The new pixel rule covers n2.

c. Schedule: `cpm2.py` gives path 57, `L0-EXIT` 12, C1 37, PT3 56 and REC 57, with 76 MP1 tasks and 42 headed runs. The path runs through the slice again (A3 → `L-TGT` → `C1-FINAL`). The re-sequencing holds:
- `L-PWR` builds against the `rate_factor` and `weapon_enabled` stubs and runs PWR1 after it rebases on `L-CON1`, which writes the constructor's call line.
- `L-OBS` on `A3^` gives the tree with A3 that its probe needs.
- `L0-SPLIT` comes before `L0-CORE`, and the `core_adapter.rs` pair with `L0-API1` is in 3.4 (P:312).
- I found no reversed edge and no cycle.

d. Coverage against design 6.2: 42 rows give 52 parts. Each part has one owner with the 6.2 stage. 30 scope items, 21 open points. `BAD 0`: the gap repairs did not unown or double-own any row.

e. Design 6.2 against 6.1: every hunk maps to E21 to E26 or to their records (the Status line, the revision history, the errata table). I found no broken gate-1 closure and no stage list that disagrees. `xcheck.py` gives `FAILURES: 0` (49 keys, 42 rows, 30 items).

f. Runs:
- Plan check GREEN, `exit=0`. `--self-test` `exit=0`, with 20 of 20 mutations RED.
- My p1 (tree row of `L-TGT` deleted, A3 made a merge-after) and p4 (`I-PLAY1` removed from the inputs of `C1-PLAYER1`) are RED.
- My p2 (PAY1 plant (6) dropped from the plant cell) and p3 (`L0-SPLIT` removed from the inputs of `L0-CORE`) stay GREEN, `exit=0`. This is Low L-a below.
- `completeness.py --self-test` 21/21 and `--enforce`: both `exit=0`.
- The tree stayed clean (`git status --porcelain` empty).

g. Descent is enough. The rule reads `up_in` (plain and stacked inputs, `mp1_plan_check.py:377`). A plain input is merged before the task starts. A stacked input starts only after its own inputs merged. So `Q-LOCK` exists before the first headed run of every descendant. A direct input adds nothing. The text should say so (Low L-b).

Low items (binding notes):
- L-a. The plant label rule needs only one D label per cell, so it cannot see a missing plant (p2). The script also does not judge the conflict pairs it prints (p3). Note for `L-PAY`: the reviewer counts the PAY1 plants (1) to (6) against D:2444.
- L-b. P:111 and the `Q-LOCK` Exit say "has `Q-LOCK` in its Inputs", but the script checks descent. Change the text to "descends from".
- L-c. The slice and MC headed runs still use v4 on the same `gpu-runtime-2.lock`. The v6 queue order and hold limit do not cover them; only the daily receipt read sees their waits. Note for Root.
- L-d. The slice-plan diff also adds a "From the Q1b review" block (Q1b Step 5 marked void, new hashes, a CI step). That is outside the stated amendment, and this gate did not review it.

What I changed: none. Scratch files (p1.md to p4.md, r3_check.out, r3_self.out) are in /tmp/claude-1000/-home-julien-Projects-TuRTS/bf418660-b272-4bb3-8628-8d9ef2677d1d/scratchpad/gate2-rigour/.
