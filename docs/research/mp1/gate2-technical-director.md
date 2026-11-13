# MP1 plan gate 2: architecture and sequencing (technical-director)

Source: subagent report. Agent output; claims are leads until verified.

VERDICT=PASS-WITH-FINDINGS HIGH=3 FINDINGS=8 RUNS=13 NOT_CHECKED=4
Condition: repair H1 to H3 in the plan before L0-WIRE starts. The six "now" tasks (P:94) can run.

**Findings (High first)**

H1. P:146, P:502. No task owns the client store for the new 6.8 records: sites, SiteProgress, ConstructionQueue, SupplyRecord, MatchState and jobs. L0-WIRE touches replica.rs only for `unit_orders`. But UI-PLACE, UI-HUD, UI-HINT, UI-ALERT and UI-RESULT (P:715-755) all read these records. Five merge-after lanes would each add the storage or derive it. That is the MC-validator pattern, on the client side. There is also a design conflict. D:1381 says "replica is the only source", but D:2257 says "MP1 adds no field to the replica candidate". `Candidate` is at replica.rs:82, and `candidate_at` clones each of its fields (:432-446). **needs decision**: (a) add the fields to `Candidate`; or (b) a side store that commits with the candidate. Repair: L0-WIRE (+1-2 runs) builds each record's apply and one source getter for each `Value`. Add a contract test that applies a hand-built protocol-5 Snapshot and WorldDelta through `Replica::apply_delta` (replica.rs:278), as `crates/turts_client/tests/economy_replica.rs` does. This test also retires the R8 risk.

H2. P:156, P:260. L-TGT and A3 both define the combat spatial index, in the same lines: combat.rs:90-117 (the per-tick scan and the cell), inside `run_combat` :82-257. combat.rs has only 410 lines of code; tests start at :410. The conflict list cannot show this pair. The check script treats merge-after as an order (mp1_plan_check.py:122) and charges 0 runs for the rebase (:103-107). My script found 5 hidden pairs: L-TGT|A3, L-TGT|L-PWR, UI-HINT|UI-PLACE, UI-RESULT|L-LOBBY-C1, L-FX-WIRE|A2b2. Repair: (a) the script finds concurrency from build windows; (b) one owner for the unit frame and the cell: L-TGT builds on the A3 branch head, or A3's combat part moves to L-TGT by a slice change request; (c) L0-SIM adds a stub `power::weapon_enabled`, so that L-PWR does not edit combat.rs.

H3. P:499. L0-WIRE builds the `unit_orders` change list into "the order writers in orders/" at offsets 4-8. A grep finds about 62 order-state write lines in non-test orders.rs (pattern count, approximate). A2b1, A2b2 and A2c rewrite those lines at offsets 10-19. A2c adds new writers (`UnitOrders.current` from the accept tick, SP:872). If one writer is missed, a change drops with no error. ORDW1 (P:801) has no single-move case. Repair: one writer function that records the change, and a syn static check in F-RUST (the DES1 pattern, P:665), so that a forward merge with a direct write goes red. Add an ORDW1 case for a single `MoveTo`, at accept and at arrival.

M1. P:296-311. Lane 0 freezes the stage-2 shapes (lobby actions, rally, repair, surrender, stats, V1 and V2) at offset 10. Stage 2 uses them from offset 35, after C1 can change them. The change-request budget is 0-3 runs for 11 lanes, and it does not count the rebase of each open lane. Repair: L0-EXIT freezes the stage-1 shapes only. A new L0-EXIT2 (1 run, after C1) freezes the stage-2 shapes. This needs no second bump, because protocol 5 is not released before MP1 merges.

M2. The plan does not carry the slice rule SP:264: "until A2c, MP1 adds no test that pins a blocked engine trip or the operations of a single move." L0b-ORD (10-12), L-CON1 (15-21) and L-STO issue engine moves while A2b1 and A2c change those moves. Repair: copy the rule into P section 1. Until A2c merges forward, the CON1 and STO1 plants assert arrival and progress, not operation counts.

M3. P:298, P:335. The client evaluates rate records (D:1348-1351) and shows the item fields `Paid` and `Refund`. `turts_rules` has no "value at tick" function. UI-HUD, UI-HINT and the progress bar would each write that arithmetic. Repair: add `turts_rules::progress` and use it for Paid and Refund. Extend the CON2 grep (P:715) to charge, refund and progress arithmetic.

M4. P:110 (R17). The sim inserts a default config, and the host removes it. This puts a test convenience into production behaviour, and a late removal lets the sim tick in the lobby. The count that R17 marks NOT TRACED is now traced. `SimPlugin` is built at harness.rs:13 (`headless_app`, used by 51 files), at server main.rs:68, and in session.rs tests at :792, :963, :1116 and :1747. Repair: `SimPlugin` inserts nothing. `headless_app` inserts `Immediate` (one line). The server inserts the config by its policy. A harness without the config then gets 0 ticks and fails loudly.

L1. P:561: `KeyUnitsLost` is counted "at spawn, completion, death", but the completion hook has no owner: L-MATCH-SIM edits none of those files, and L-CON1 comes later. P:601: the L-PAY audit rule must check the conservation identity only (balance change = -ΣCharged, charged ≤ cost) and must not compute the charge schedule again. Add one plant.

**Shared files: present shape and judgement**

| File | Shape at a6f3f66 | Judgement |
| --- | --- | --- |
| orders/ (17 writers) | orders.rs 7,036 lines, tests :3674; S2 makes 9 files (SP:441-464) | Feature lanes in own behaviour files: adequate. Lane 0 and the slice in apply.rs and pending.rs: H3. L0-SIM `validate_target` (:3028) sits next to the A2c deletions (:2998, :3117-3176); the compiler catches that conflict. |
| combat.rs (8) | 2,093 lines, code 1-410 | The rule only orders merges, and the 5 unordered writers include 3 in one function: H2. |
| session.rs (6) | 2,772 lines, code 1-725; `receive_hello` :201-342, admit :382-461 | Writers are ordered by inputs (L0-WIRE, L-MATCH-SRV1, L-LOBBY2-SRV, L-MATCH-SRV2); A1b edits tests only. Adequate. |
| export.rs (2) | 564 lines | R4 stub files already split it. Adequate. |
| mod schema | schema.rs 504, interface.rs 423 | One writer (L0-API1). Adequate. |
| unbuilt.rs (15) | new | Delete-only union with an owner on each line. A merge cannot duplicate logic. Adequate. |
| widgets/ (7) | mod.rs 3,241 (code to 1,681), sources.rs 2,455 | L0-CORE owner stubs and S45 split it first. Adequate. |
| frontier interface/hud.ron | new | The stage-2 HUD entries need an owner: C2-CONTENT should merge after UI2-HUD, UI2-SMART, UI2-CONN and UI2-MC (P:196). Low. |

No file needs a split before work beyond these items. combat.rs gets no extra split because H2 repair (c) removes one of its writers.

**Sequencing** (I edited copies of the plan table and recomputed with the plan's own script)
- Plan as written: 52 runs. If the L-TGT rebase on A3 costs 1 run: 53.
- P-a: A3 builds stacked on the A4 branch (offsets 17-20) and merges with Q2 at 22: **51**. The path then moves to L-PAY → L-CON1 → L-PWR.
- P-b: P-a plus the weapon-gate stub: **51** (path through L-DES). L-TGT stays off the path even with a 1-run rebase. Recommended.
- P-c: P-b plus L-PAY merge-after A1d: **49**, C1 at offset 32. Risk: L-PAY must move its ledger rule into the A1d oracle at the rebase, or it starts a third oracle.
- A3 before the A2 chain: no. A2a, A2b1, A2b2, A4 and A3 all edit frame.rs (SP:157-165), and A3 needs the A4 functions.

**Stage split.** Stage 1 is thin enough. Besides the P:827-835 items, L-INC and I-NC1 can move to stage 2 with no loss for C1.

**Q rulings**
- Q1, Q2: split; accept.
- Q3: accept R14. GOAL1 is a stage-1 row; REC records the D erratum.
- Q4: (a) measure with 2 slots, and record the D57 and D58 scenes as not measured. Drop the three-slot MAT1 test. No test-only slot count in the sim.
- Q5: accept.
- Q6: accept, but tag the alert engine work E+UI, not content.
- Q7: `stats: []`; accept.
- Q8: accept.
- Q9: as in M4.
- Q10: run PRES-MEASURE now as a scratch probe. It is the evidence for P1 and D-M, which are due before offset 10. Asset work starts after the V1 and V2 shapes freeze. Merges stay in stage 2.

**What I ran**
- The plan check script: exit 0, path 52, 112 pairs. Output: `/tmp/claude-1000/-home-julien-Projects-TuRTS/bf418660-b272-4bb3-8628-8d9ef2677d1d/scratchpad/gate2-td/check.out`.
- `td_sched.py`: build-parallel merge-after pairs, and the writers per file. It would refute H2 by finding 0 such pairs; it found 6, 5 real. Script: `/tmp/claude-1000/-home-julien-Projects-TuRTS/bf418660-b272-4bb3-8628-8d9ef2677d1d/scratchpad/gate2-td/td_sched.py`.
- Recomputed the path for four edited copies of the plan (plan_real, plan_alt1, plan_alt2, plan_alt3 in the same scratch folder): 53, 51, 51, 49.
- `wc -l` and grep for the first `#[cfg(test)]` on 18 hot files; read combat.rs :88-125 and :297-312.
- grep for `SimPlugin` (5 files, 8 lines) and `headless_app` (51 files).
- grep for `UnitOrders` writes (62 lines, approximate). File: `/tmp/claude-1000/-home-julien-Projects-TuRTS/bf418660-b272-4bb3-8628-8d9ef2677d1d/scratchpad/gate2-td/unitorders_writes.txt`.
- Read replica.rs `Candidate` and `candidate_at` (:425-455). grep of the plan for replica, hud.ron, export and the SP:264 terms (0 hits for the rule).
- grep of boundary.sh for the `turts_sim` and `turts_audit` rows (rows exist).

**Not checked**
- The bit-equality of A3's combat cell.
- The D 6.10 stance fields for D-G.
- The S45 split files (not merged).
- The client cost of D:2257 option (a).

**What I changed**
None in the repository. I added memory note `/home/julien/Projects/TuRTS/.claude/agent-memory/technical-director/project_mp1_gate2_plan.md` and its line in MEMORY.md.

**Open questions:** the D:2257 decision (H1). Whether Root accepts a slice change request that gives A3's combat part to L-TGT (H2 option b).
