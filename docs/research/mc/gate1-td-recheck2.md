VERDICT=pass-with-changes FINDINGS=2 HIGH=1 RUNS=7 NOT_CHECKED=2

D = the MC design, revision 3.

Items
- N1: fixed with a new defect (N1b).
- N2: fixed (D:940-950). The 15 s retry is longer than the 10 s timeout (join.rs:278). SLOT1 (D:1158) can go red.
- N3: fixed (D:196, D:1223).
- N4: fixed. The 46 runs are correct (D:1239).
- N5: fixed (D:1251).
- Budget: correct. 106 + 14 + 64 (60 % = 63.6) = 184.

New findings
1. N1b, D:546 against D:553. High. Under 6.1, the start and the completion of an order are captured operations. The frozen stream moves the acme scout from (4,4) to (30,30) at speed 6 (m3_m2_stream.rs:263). That is 36.8 units, which the scout covers inside 400 ticks at any tick rate up to 65 Hz. The completion therefore adds an operation, the stream fails, and REG1 (D:1189) is false. The field rule at D:548 protects bytes only, not the operation count. Fix: a plain move writes no audited order, and the movement function completes it. Gather (:331) has the same risk: NOT TRACED.
2. Low, D:1240. The longest simulation path is 2, 4, 5, 19, 22, 23, 24 (29 runs), not the 28-run path.

Cheapest honest split (my estimate). These cuts touch no S:722 item:
- pointer lock: about 2 runs;
- audio, with text alerts kept: about 4;
- rebinding and stored groups: about 2;
- `LightDef`: about 1;
- rejoin (K27 c): about 3.
That is about 12 task runs. With the 60 % repair allowance it is about 19 runs, so the total falls to about 165. Take the cuts before gate 2, because each slip after gate 2 needs a re-gate (D:15.3).

What I ran
- read the stream test and the acme map | N1b | no move | arrives
- movement.rs:30-58 | snap rule | change | none
- path script | D:1239 | not 46 | 46
- sum script | D:15.1 | not 184 | 184
- read 10.5 and SLOT1 | N2 | no retry | retry
- grep D:196 | N3 | absent | present
- read the 15.2 table | N5 | absent | present

Not checked: the gather order path; the removal of the server session at timeout.

What I changed: none in the repository. I updated my memory.
