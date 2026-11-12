# MP1 design revision 4 re-gate: rigour

Source: subagent report. Agent output; claims are leads until verified.

VERDICT=FAIL HIGH=2

Spec rigour re-gate of design revision 4 (branch mp1-design, head 5158f96). Mode: review-spec. What I changed: none. My scratch files are in /tmp/claude-1000/-home-julien-Projects-TuRTS/bf418660-b272-4bb3-8628-8d9ef2677d1d/scratchpad/regate-rigour/ (xcheck.py, xcheck.out, tokens.py, selftest.out). Line numbers below are design lines. `S:n` is a specification line.

Both High findings are small text repairs, but H2 needs a CEO decision first.

## My gate-1 findings

| # | Finding (gate 1) | Status | Evidence in revision 4 |
| --- | --- | --- | --- |
| 1 | CON1 red build was the PRD1 green build | CLOSED | 6.1:378-399 (the queue holds orders only, the site owns progress); CON1 plant (1) :2031; PRD1 :2055 |
| 2 | P4 did not quote S:801; P3 cut a Decided item | CLOSED | P4 :143 quotes S:801 word for word (checked). P3 (d) :142 needs the CEO; S:798 checked |
| 3 | A leave reopened a claimed slot | CLOSED (CEO to confirm D-B) | 6.7.3:727-743; LOB1 run 3 red :2039. A new defect sits next to it (M2) |
| 4 | CB05 pointed to a list that does not exist | CLOSED | :2350 `in TGT2`; OR26 :2287 |
| 5 | TGT1 and TGT2 disagreed on hold | CLOSED | D-G :156; 6.3:546-547; TGT1 and TGT2 plants |
| 6 | Reds that cannot fail | PARTIAL | CON2, DMG1, DES1, REP1, MAT1, RES1, TGT1, TGT2 and HUD5 now have real plants. My probe confirms the DMG1 and CON3 numbers. GOAL1 is still open: see H2 |
| 7 | Exit item 1 and the plants of REG2 and PT3 | PARTIAL | :49-55 is fixed. The REG2 plant lines match the code. The PT3 plant does not touch the field that it judges: see M3 |
| 8 | Gate checks only that a path exists; CA13, PE04, MP07 | PARTIAL | The rows are fixed (:2301, :2429, :2389), and 67/67 tokens are in their files (my run). The gate is still form-only: R7b is not applied (15.9, 14:2125), and Y1 needs a decision |
| 9 | The "Changes" column was not complete | CLOSED | Section 2 Changes cells now list rows, coverage items and Player path steps |
| 10 | "Done" and the end of the 60 s were not defined | CLOSED | 1.1:115-119, 7.1:1470-1474 |
| 11 | Terms with two meanings | PARTIAL | 1.1 exists, but "seat" still has two meanings (L1) |
| 12 | Status labels and citations | CLOSED | D70 is Proposed at :1010 and :1611. 8.1:1754. 6.3:518-522 (invariant 15 is S:581, checked). The `economy/` path is fixed. 8.4:1811. No settings schema (:1372). Open point 7 |

## New findings

**H1 (High): the reason key `queue_full` has two texts.**
- Lines: 6.1 :444-445 and 7.2 :1499 give `queue_full` = "Construction queue is full (5/5)". HUD6 :2060 plant (c) and 7.3 :1536 give "Queue is full (5/5)" for the unit queue.
- The conflict: today `QueueFull` is also the producer and research refusal (`crates/turts_sim/src/economy/production.rs:117`, `research.rs:133`, wire key `messages.rs:241`). A mod `reasons` map gives one text for each key, so HUD6 and 7.2 cannot both pass. Lane 0 freezes `REASON_KEYS`, so a later split changes the API.
- Smallest repair: add the key `construction_queue_full` to 6.1 item 6, the 7.2 table, CON1, PRD1, the 6.8 list of new reasons and 6.10 row H1. The other option: use one text in both places.

**H2 (High, needs decision): stage 1 runs under `Immediate`, so GOAL1 depends on a delay that no one controls.**
- Lines: :1965 (stage 1 uses `Immediate`); 6.7.4 :752 (under `Immediate`, Boot goes straight to Running); GOAL1 :2052 ("+300" before 40 s from the press, plant (e)); C1 :1967.
- The conflict: the start tick is the boot, not the press. Each window joins later, at a different tick, so the "+300" timing and plant (e) depend on the boot-to-press delay. A window that joins after the first delivery never sees its "+300". Slot 1 also earns credits before its player exists.
- 6.7.4 :753 says that `frontier` uses `AllSeatedReady(3)`. That policy is stage 2, and lane 0 refuses unbuilt features (`UNBUILT_FEATURES`, :1213). The design does not say what start `frontier` writes in stage 1.
- Options:
    - (a) a server test hook `--start-when-seated N`;
    - (b) build `AllSeated(0)` in stage 1, which reverses 6.7.14 item 2;
    - (c) measure GOAL1 from the Welcome tick of each window, and require a join before tick 100.

**M1: the stage-1 hint list names stage-2 actions.**
- Hint H3 (:2460) needs a rally. Rally is X9, stage 2 (:227).
- Under the selection rule of 7.8 (:1677-1679), H3 can start but can never be done, so H4 to H7 never show in stage 1.
- Hints X1 and X3 (:2466, :2468) tell the player to cancel a job, but PAY2 is stage 2.
- Repair: in stage 1, the `frontier` list leaves out H3, and H4 starts when H2 is done. Also give the production cancel a stage-1 row.

**M2: the design gives two rules for a page reload in the lobby.**
- 6.7.5 :865 says "not possible: a seat is released when its connection ends". :839-841 says the other window gets "Player 2 left".
- 6.7.7 :926 and 7.6 :1624 say that the reload keeps the seat, and that the other window shows only "Player 2 — Not ready".
- With the 6.7.5 rule, the seat comes back only because it is the lowest open index, and the colour resets to the lowest free index (6.7.6 :874). No row tests a reload in the lobby.
- Repair: choose one rule, and add a LOB1 run for it.

**M3: the PT3 plants change `done_ms`, but the 60 s rule reads `site_ms`.**
- 7.1 :1473-1474 judges the time of the first server site (`site_ms`). PT3 :2050 shows its red with GOAL1 plants (b) and (d). Both plants change only `H1.done_ms`, so `site_ms` does not change.
- Repair: plant a client that writes `site_ms` at the pending site. Then the timing script is red against the server order log.

**M4: the stage lists in sections 4, 10 and 11 disagree.**
- 10 :1973 puts surrender and disconnect under X11. Section 4 :229-230 puts them in X11c.
- 11 :1992 has no X13 statistics in stage 2.
- 10 :1946-1963 and 11 :1991 have no X22 in stage 1, but 4 :245 and 12 :2020 run REG2 at the end of stage 1.
- The 10 stage-1 table has no X21 (the `player` agent).
- The 10 stage-2 list has no "X15 rest" and no "X20 rest".

**M5: 7.6 and Player path L3 give different lobby words.**
- 7.6 :1619 gives window B "Slot 2: You — Not ready". L3 :2446 gives both windows "Player 2 — Not ready".
- The LOB1 pixel check uses the L2 to L5 words (:2039).
- Repair: change the text of L3 to the 7.6 words.

**Low findings:**
- L1: "seat" also means an agent seat at :55, :162, :302, :698, :1119 and :1437. This breaks the one-meaning rule of 1.1 :75.
- L2: TGT2 :2067 uses a turret with range 8. The section 5 turret has range 7 (:286), and TGT2 names no package. With the `frontier` numbers, the case "attacker beyond the acquire range" does not occur (16 :2220).
- L3: 6.8 :1162 lists the reasons as "(key units lost, surrender, disconnect, draw)". That differs from 6.7.9 :995-996: `Draw` is an outcome, and `OpponentsDefeated` is missing. 7.6 has no text for the outcome `Draw` with the reason `Disconnected` (MAT1 case (c)).
- L4: 14 :2112 says "mc_group 19, ... t4, t5, t6b, t8". The code has 8 `#[ignore` lines in mc_group.rs, and t4 to t8 are in acceptance.rs, which has 9. The total of 31 is correct.
- L5: the PE04 note :2429 says that VIS3 records the box-select frame time. VIS3 :2047 does not say that.

## "Which build makes this red?" for 8 rows

- DMG1: the float build. Named. My probe: f64 gives 28 on 100 x 29/100, and f32 gives 52 on 50 x 106/100.
- CON1: the build where the site is only a marker. Named.
- CON3: the build that refunds the full cost. Named. My probe: charge 185, refund 138, destroyed 92.
- LOB1: the build that claims at `Hello` (run 3). Named.
- REG2: `clamp_to_map` returns at once. Named. `camera.rs:358` and `tests/camera.rs:322` and `:345` match the code. I did not run the mutation.
- STO1: plants (a) to (d). Named. `--order-log` exists (`config.rs:162`).
- GOAL1: plant (e) is unstable in stage 1 (H2).
- PT3: the named plants do not reach `site_ms` (M3).

## What I ran

| Command | Exit / result |
| --- | --- |
| `completeness.py --self-test` | exit 0, 21/21 cases |
| `completeness.py --enforce` | exit 0, 3 specs |
| `yardstick.py --check` | exit 0 |
| xcheck.py: section 12 rows (42) against the stage lists; cited row ids; coverage `in` cells; X-item stages in 4, 10 and 11; D- ids against section 2 | exit 0. Every row is in a stage list, and every coverage `in` cell names a real row. The ids not in section 12 are MC rows only. 8 stage mismatches give M4 (I removed 1 false positive by hand). All D- ids exist |
| tokens.py: 67 `done:` rows | 67/67 tokens found |
| Python probe of the DMG1 and CON3 numbers | the claims hold |
| `grep` of the reason keys; the `queue_full` producers | gives H1 |
| `grep -c '#[ignore'` in `crates/turts_server/tests` | 31 in total; the per-file split differs (L4) |
| `sed` of 38 code citations: session.rs:152-159 and :403-425, slots.rs:7 and :22-25, content_m3_validation.rs:374-376, :418 and :420-424, version_table.rs:52-56 and :101-108, combat.rs:393-405, :649 and :1076, transaction.rs:105 and :747, transaction_m3.rs:354, the client_web lib.rs lines, orders.rs:2610, mod_api lib.rs:52, messages.rs:21-31, :136 and :241, net lib.rs:13, views.rs:558, replica.rs:523-525, join.rs:345, completeness.py:118-121, add_id :449-456, no_content.rs:166, movement.rs:13-21 | all 38 match |
| `sed` of 30 S: lines (S:52 to S:809) | the quotes and status labels match. Invariant 15 exists at S:581 |

**Not traced:**
- I did not run the REG2 and NC1 mutations or any cargo build.
- I did not check the vault notes, the pacing scripts or the probe outputs.
- I did not check the balance numbers.
- I did not read the repair notes fully.
