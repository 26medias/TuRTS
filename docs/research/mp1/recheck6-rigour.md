# MP1 design revision 6 confirmation: rigour

Source: subagent report. Agent output; claims are leads until verified.

VERDICT=PASS-WITH-FINDINGS HIGH=0

Revision 6 (head 7454a8e) closes N-H1, and the patch breaks no cross-check. Two items remain for the plan as binding notes. What I changed: none.

a. N-H1 is CLOSED. GOAL1 can now be built under the driver rule.
- The driver reads no text and touches no bridge (rule A1). The oracle reads `turts_debug.hints`.
- Plant (f) goes red through the oracle: at the step of H4 it sees H3.
- Plant (g) also goes red through the oracle: it sees the revision-4 H1 text, or no hint X4, where X4 is due.
- Section 15 point 17 is closed. My grep finds no other row or sentence that needs a driver to read text.
- Low: the second clause of plant (g) is wrong. The driver clicks its own Command Centre by its script, so P does start a placement. The oracle clause alone makes the plant red. Repair: delete "and the driver, which presses P … H1 shows for 300 s".

b. The rally rule is stated one way. 6.5 (:723-732), section 5, 6.10 rows U8 and M3, 7.3, STO1 run 4, RLY1 cases A to C and Player path step 7 all say the same rule: a ground rally does not apply by default, and a node rally applies in place of the spawn order. The text "queued after it" now occurs only in the `ApplyFirst` case (:731, U8, RLY1 case C).
- Medium: STO1 run 4 and its plant (f) need a rally on a node. Rally is X9, stage 2, but the stage-1 row list (:2424) has STO1 with no split. As written, stage 1 cannot make STO1 green. Repair: in STO1 and in the stage lists, write "run 4 and plant (f) are stage 2 (X9)".

c. Cross-checks on revision 6:
- xcheck: 42 rows, each in a stage list, no stage mismatch, all cited ids and decision ids exist.
- keys: `REASON_KEYS` is still 49, all unique, and it holds all 37 wire keys of today.
- tokens: 67/67.
- `completeness.py --self-test` exit 0 (21/21); `--enforce` exit 0 (3 specs).

d. The new open points:
- Point 19 (pacing): owner `game-designer`, closes before C1.
- Point 20 (box drag): owner `ux-designer`, closes at gate 2.
- Point 21 (connection-end schedule): owner `backend-engineer`, closes in lane 0.

None of them hides a High. Point 21 is behaviour that exists today without a traced handler, so the plan must trace it before L-LOBBY builds the seat release.

Runs: I read the diff 12bca5b..7454a8e and grepped the GOAL1, STO1, RLY1 and RES1 rows and section 15. Not checked: the plan diff, and the network texts F1 to F4 beyond reading them in the diff. My scratch files are in /tmp/claude-1000/-home-julien-Projects-TuRTS/bf418660-b272-4bb3-8628-8d9ef2677d1d/scratchpad/regate-rigour/ (r6.diff, xcheck6.out).
