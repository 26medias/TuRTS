# MP1 gate 1: rigour lens

Source: subagent hand-back, MP1 brainstorm input, 2026-11-11. Agent output, not reviewed; claims are leads until verified.

VERDICT=FAIL FINDINGS=12 HIGH=4 RUNS=14 NOT_CHECKED=5

Findings

High
1. design:132-133 vs :250, :378, :680. Section 6.1 refuses "Progress inside the producer's queue". Section 7.3, row PRD1 and step 7 put a structure job in a command-centre queue. So the CON1 red build is the PRD1 green build. Fix for 6.1: "The structure queue holds the order until placement. Then the site entity owns progress and charge."
2. design:40 vs S:801. S:801 (16.2, Decided) says: "Pay credits, select a valid site, then construction progresses over time". P4 (b) cites only S:723 and S:434. S:723 is Proposed until gate 1 (S:701). Needs decision: P4 must quote S:801 and record the spec change. Same class: P3 (:39) lets the CEO cut repair, but S:798 is Decided.
3. design:364. Today the server claims a slot at Hello (`crates/turts_server/src/session.rs:838`). LOB1 says "a leave reopens the slot". That reopens a claimed slot (D25, invariant 11, S:350). Needs decision. Option 1: the claim happens at match start. Option 2: a leave closes the slot. Write the rule in 6.7.
4. design:557. CB05 is `later:MP2`, with the note "see the weak-cover list". No such list exists (grep). TGT2 (:387) is retaliation, and :436 says X2 takes it. Fix: `| CB05 | in | TGT2 |`. Then check OR26 again.

Medium
5. design:355 vs :387. TGT1 says "hold never moves". TGT2 lets a holding unit chase an attacker inside its leash. Fix for TGT2: "A holding unit fires back only inside weapon range." Red build: "a holding unit moves".
6. These reds cannot fail: CON2, DMG1 (float), DES1, REP1, MAT1, and RES1 (a pixels driver cannot see a server read). These reds only show that the feature is absent: TGT1, TGT2, HUD5, GOAL1. The detail file gives a plant for each.
7. design:23. Exit item 1 needs "each with its recorded red". VIS3 has no red, and REG2 and PT3 name no plant. Fix: "Every row except VIS3".
8. The completeness gate checks syntax only. A scratch copy with `done:Cargo.toml` and `CO04 in VIS3` exits 0. I checked a sample of 33 rows:
    - CA13 `done:CAM1` does not test the item: cam1-oracle.js:1-79 has no bounds assert.
    - PE04 `done:SEL2` does not test the item: I found no timing assert.
    - MP07 is `done:`, but its dependency MP01 is `later:M4`. This breaks the design's own rule at :451.
9. design:37-45. For every decision, the "Changes" column leaves out section 12, the Yardstick coverage section and the Player path. P2 option (a) says "ready only", but the recommendation keeps colour (LOB2, L4, TM03).
10. design:666-670. A hint goes "when the action is done". The design does not say if this is placement or completion. The first station builds at half speed (GD:123, :168). PT3 times only the site. Define "done" and the end of the 60 s.

Low
11. Terms with two meanings: slot (:49, :289), team (:289), seat (:180), site and pending site (:377), alert and notice, done. The definitions to add are in the detail file.
12. Status and citations:
    - 7.6 treats D70 (Proposed, S:99) as decided.
    - 8.1 fixes GLB features that S:247 keeps Open.
    - :149 cites invariant 15 for integer arithmetic, but invariant 15 is the replay rule (S:581).
    - The :151 path has no `economy/`.
    - 8.4 (:306) contradicts :439.
    - 7.6 needs a settings schema, but MD12 is `later:M9`.
    - S:727 still says M5 needs teams from MP1.

What I ran
- `completeness.py --self-test`: exit 0. `--enforce`: exit 0, 3 specs checked (MP1 included).
- `yardstick.py --check`: exit 0.
- Mutant gate run, to test the claim at :3 "the completeness gate passes": exit 0. Control run with a missing path: exit 1.
- 14 code citations checked with sed: 12 match, 2 paths are wrong.
- Counts: 55 in, 69 done, 57 later; 36 rows. TGT2, REP1 and CON2 cover no yardstick item.
- Float probe: floor(100*0.29) gives 28; the integer form gives 29.
- Oracle greps: cam1, sel, vis, orders, shell, feedback.

Detail files:
- /tmp/claude-1000/-home-julien-Projects-TuRTS/bf418660-b272-4bb3-8628-8d9ef2677d1d/scratchpad/rig-detail.txt (plants, the Changes omissions, term definitions)
- /tmp/claude-1000/-home-julien-Projects-TuRTS/bf418660-b272-4bb3-8628-8d9ef2677d1d/scratchpad/rig-sample30.tsv (the 33-row sample)

Not checked
- The balance numbers.
- Probe MP1-P1.
- The PE07, PF05 and CA16 oracles.
- The source of the "+N" text (NOT TRACED).
- Conflicts on the key P.

What I changed: none. During my run, other agents left untracked files in the worktree (`mods/frontier/`, then `crates/turts_sim/tests/zz_pa_sim.rs`). I did not make them.

Open questions for the orchestrator
Findings 2 and 3 need a CEO decision.
