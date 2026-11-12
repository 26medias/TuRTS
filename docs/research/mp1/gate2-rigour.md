# MP1 plan gate 2: rigour

Source: subagent report. Agent output; claims are leads until verified.

VERDICT=FAIL HIGH=2

The plan's coverage holds: my own script and the plan's script agree. Two defects block execution as written: one stage-1 row part has no decision, and the path model is wrong at A3. Both repairs are small.

**High**

H1. P:1076 (Q4) and P:563, P:580, P:839. The MAT1/s1 three-slot sim test (D:2447, stage 1) cannot be built: `SLOT_COUNT` is 2 (`crates/turts_sim/src/slots.rs:7`), and D:281 says "more than two slots" is not in MP1. Your working answer (a) fixes only the exit-item-5 scenes (D:82-83). It says nothing about MAT1. C1 needs every stage-1 row part green (P:839, D:2365), so C1 cannot pass. Needs decision for MAT1:
- (b) a test-only slot count in the sim, about +1 run in `L-MATCH-SIM`;
- (c) drop the case and edit D:2447. My inference, not run: with 2 slots, the first defeat ends the match, so `slot_defeated` cannot occur in MP1.

Also move ownership of the case to `L-MATCH-SIM`, which builds it (P:563).

H2. P:156, P:160, P:211-214. A3 must be a real input of `L-OBS` and `L-TGT`, not a "merge after":
- `L-OBS` step 1 runs the perf probe "on the tree with A3" (P:677).
- `L-TGT` builds the per-tick frame "verify first after A2c, A3" (P:633). D:2224-2225 says this part rebases on A2c and A3, which edit the same `combat.rs` and `orders/` code.

With A3 as a plain input of both, the plan's own script gives a 56-run path, C1 at 39 and `REG2-2` at 53. The plan claims 52, 35 and 49. Repair: move A3 to the Inputs cell of both rows. Then restate 3.1, 3.2, 14 and the R8 reason.

**Medium**

M1. P:72 and the script's `schedule()` (`mp1_plan_check.py:103-107`). A merge-after task finishes at the same offset as the task it merges after. Rebase, row runs and review cost 0 runs, but P:72-73 says they happen after the merge. 16 edges have this property (list in `detail.txt`). With +1 run per edge the path is 54 and C1 is at 37. Repair: `e = max(s + runs, end[a] + 1)`.

M2. R14, R15 (ALR2) and Q4 change gated statements of D: the X14 stage-1 part (D:260, D:2347), ALR2 listed whole in stage 1 (D:2427), and exit item 5 (D:82-83). P:9 says "D wins", so D needs a revision-6.1 note before execution. Also, ALR2/s2 (`UI2-CONN`, P:884) has no red build in D or in the plan. Repair: add a plant, "`SlotDisconnected` not wired: a killed client gives no 'lost connection' line".

M3. P:18 says every D section 2 answer is needed before offset 10. That overstates it. 5.2 item 4 lets an unreleased protocol 5 and API 0.2.4 change in place:
- Answers that delete a feature can stay behind `UNBUILT` at small cost.
- Answers that add a type or record need the decision before `L0-EXIT`: P2 c, D-K b, Q1 b, D-E b, D-I b, D-J b, D-B 2.
- These change only content or later lanes: D-M, P6, P7, P9, D-C, D-G, D-H, D-N, S1, 6.7.14, N1, Y1, MC CI.

Full split in `detail.txt`. Repair: replace the "needs decision before offset 10" sentence with this list.

M4. P:767 and P:771-777. The `C-UNITS` gate cannot fail on a content error: the packer stops at the first `ModError`, so "exactly one `UNBUILT` refusal" hides every later check. `C-UI` has no step and no check. Repair: pack a copy without the unmerged-lane fields, expect `exit=0`, and add one plant (gap below the K2 minimum gives exit 2 with the K2 text).

**Low**

- L1. P:73 says "`L0-EXIT` is the second lens for every lane-0 task". That is false: `L0b-ORD` runs after it, and `L0-API2`, `L-ROOM0`, `C0-PACE` and `E-GPU` are not its inputs.
- L2. P:633 cites `movement.rs:13-21`, which is stale after S3 (now `crates/turts_sim/src/movement/mod.rs:32-37`). P:56, P:120 and P:1056 still say head 85f34cb and S3 remaining; the slice head is 7b23b37 with S3 merged.
- L3. These items have no red: P:992 (M5, reviewers read only), P:984 (N1, HUD per-frame rule), and P:727 (the `UI-HINT` counter has no threshold).
- L4. The script does not check the rule that stage 2 starts after C1 (P:872). Mutation m4 (`L-CON1`'s input `C1-PERSON` replaced) stays GREEN. The table holds the rule today (my script: 0 violations).

**R and Q table** (REF = allowed refinement; DCH = changes a gated D statement)

R1 REF · R2 REF · R3 REF · R4 REF · R5 REF, D:2310-2317 note · R6 REF · R7 REF · R8 DCH (D:2295 "starts when its inputs are merged"; M1) · R9 REF, D:2324 note · R10 REF · R11 REF · R12 REF (D:2571 allows the drop) · R13 REF (D:2576-2577 allows the move) · R14 DCH (stage boundary of X14) · R15: STO1 REF (D:2424 and D:2462 already split it), ALR2 DCH · R16 REF (REG2/s1 holds no MP1 plant) · R17 REF, D:900-905 note · Q1 REF · Q2 DCH · Q3 DCH · Q4 DCH plus H1 · Q5 REF · Q6 REF · Q7 REF (one D line) · Q8 REF · Q9 REF · Q10 REF.

**Path and waves:** 52 runs with the plan's model; same path, same 26 waves, same C1 at 35. 61 with plain inputs, 54 with +1 per merge-after, 56 with H2 fixed. No cycle. No builder reviews his own task. Stage sums are 30, 84 and 68; 26 headed runs. The slice inputs agree with the slice plan.

**Lane-0 freeze:** P:296-301 freezes items 1 to 6. The change-request path (P:307-311) is defined. Gap: step 3 commits on `mp1/integration` and no review run follows it.

**Coverage:** 42 rows, 51 parts, each part with one owner and a correct stage. 30 scope items. 21 open points. Every binding note is in D revision 6 or in plan section 12.

**What I ran**
- `cov.py` (own coverage check) on the plan: exit 0.
- Plan script: GREEN, exit 0. `--self-test`: exit 0, 6 of 6 mutations RED.
- Mutations: m1 (DES1 owner deleted), m2 (`UI2-CONN` stage moved), m3 (`L-TGT` stage moved), m6, m7 and m8 each RED, exit 1. m4 (C1 gate removed) and m5 (reviewer = builder) GREEN, exit 0.
- `cpm.py` (own path and wave script) with four models; m9 (A3 as a real input) through the plan script gave 56.
- `cites.py`: 46 citations at 7b23b37. 41 hold. 4 are off by one with the item present. 1 is stale. `git diff --stat 85f34cb 7b23b37` touches only movement files.
- `completeness.py --self-test` 21/21, exit 0; `--enforce` exit 0, 3 specs.
- `git merge-base --is-ancestor codex/mc-fix-deplete codex/mc-integration`: exit 1, so D0 is not merged.
- The ignored-test count is 31. Every repository package requires `^0.2.3`. `ron` is `=0.12.2`.

**Not checked:** the D design lines behind each plant; the 112-pair conflict list; S45 contents (NOT TRACED).

**Stray file:** untracked `docs/research/mp1/probes/plan/__pycache__/` in TuRTS-mp1, created at 01:37:55. I ran every script with `-I` as `__main__`, so I think another agent made it. Who made it is NOT TRACED.

What I changed: none.

Files are in /tmp/claude-1000/-home-julien-Projects-TuRTS/bf418660-b272-4bb3-8628-8d9ef2677d1d/scratchpad/gate2-rigour/:
- detail.txt
- cov.py
- cpm.py
- cites.py
- m1.md to m9.md (the mutated plan copies)
