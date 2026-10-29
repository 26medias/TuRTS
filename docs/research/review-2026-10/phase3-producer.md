VERDICT=pass-with-changes FINDINGS=8 HIGH=4 RUNS=4 NOT_CHECKED=3

Artefact: `docs/research/review-2026-10/00-gap-register.md` (R:line). Spec v0.7 = S:line. Mode: review-plan.
Dependency tags: (R) = read from the register; (S) = read from the spec; (I) = inferred by me.

## Findings

1. R:67-86, R:61. HIGH. Scripting has no position in the order and no G item. M7 needs "scripts on both targets"
   (S:681). Scripts are Open (S:705, D34). F7 says 15 CORE items need scripts (R:40), so MB needs them too.
   Run P1 shows 2 order violations: "SCR HAS NO POSITION". Fix: add a G13 for the script runtime. Put a selection
   spike before MB. State that M7 does not choose the runtime.
2. R:74 vs R:76. HIGH. The MP gate is "A person finishes a match". M4b (spreading, arrival) comes after MP. Units
   that stack look broken (inventory-player A2 item 7, CORE). Probe P2 shows the violation "MP needs M4b". Fix: put
   flat-ground GM01/GM04/GM09 in a slice before MP (GMb). Keep GM10, GM11 and PF06 in M4b, because they need terrain
   or obstacles.
3. R:74. HIGH. MP is too large for one milestone. Identity duplicates M10 (R:81; backend-4 says "with or before
   M10"). The per-slot lobby duplicates M9 (R:81). MP has no reconnect, so identity is not needed there. Fix: split
   MP into MP1 and MP2. MP1 is a playable two-human skirmish: construction, power, combat, repair, victory and rally.
   MP2 is the first bot (G8). Move identity to M10 and the lobby to M9.
4. R:101. HIGH. Silent closure. "fix S:352 (Lightyear is selected and pinned)". S:352 says "not selected". S:704
   and D18 (S:41) keep the networking library Open. `CLAUDE.md` lists it as open. Needs decision: record Lightyear
   as selected (a CEO decision), or keep the item Open with "in use for prototypes".
5. R:54. MEDIUM. The G1 text says "every reference RTS". It conflicts with D10 (bridges, S:34, S:113) and with D41
   unless G2 reopens D41. It also drops the release boundary in S:111. Fix: G1 names its exceptions, or it splits
   engine scope from release scope.
6. R:74. MEDIUM. MP has no engineer repair. Repair is a Decided 16.2 rule (S:742). Fix: add it to MP1.
7. R:73. MEDIUM. The MC row lists G6 and G7 as included before the CEO decides them. Fix: write "per the G6/G7
   outcome".
8. R:40, R:89-93. MEDIUM. No milestone or owner holds these items:
    - F7 schema growth.
    - Library packages (`manifest.rs:44-49`). The bot libraries are Decided (S:759).
    - The completeness gate and the driver/oracle split.
   Fix: put the hotkey, icon, cursor and selection-priority fields in MC. Put the cost, prerequisite, armour and
   sound fields in MP1. Add library support before MP2. Add QA0 (owner `qa-lead`) before MC, because the MC gate
   needs the driver/oracle split.

Low notes:
- R:98: the new rows D65 to D73 must be Proposed until the CEO approves them. Baseline is for retained direction.
- R:102: S:790 "desktop-first" agrees with D54 (S:77). I do not see a contradiction with D02.

## Q1. Merged order and size

The order holds for every R and S edge except the scripting edge (P1). MB after M6 is not a dependency. MB needs
MP, M5 (cloak and detection need unit fog), M4b (air separation) and scripting. MB does not need M6. Direction:
M1-M3, QA0, MC, GMb, MP1, MP2, M4, M4b, M5, SCR spike, MB, M6, M7, then M8-M13. P3: 0 violations. MB can run in
parallel with M6. MB is also open-ended: one StarCraft-like mod does not prove the union. Fix: bound MB with a list
of CORE and COMMON items taken from the coverage matrix (R:89).

Re-estimate. These are estimates, not facts. Assumption A: the M3 rate. M3 was 20 entries for 75 runs (about
3.75 runs per entry), with reviews, checkpoints and judges included (`vault/M3 Execution Board.md:145,147`).
Assumption B: the same model routing. The M3 overrun of 1.6 times is inside the high band.

| Slice | Entries (my count) | Runs |
| --- | --- | --- |
| QA0 | 2-3 | 8-12 |
| MC (merged scope, about 19 entries) | 16-22 | 60-100 |
| GMb | 4-6 | 15-25 |
| MP1 + MP2 (identity and lobby moved out) | 25-32 | 95-150 |
| M4 + M4b | 22-30 | 85-125 |
| M5 with unit fog | 16-24 | 60-90 |
| SCR spike + MB | 30-60 | 115-225 |
| M6 | 13-20 | 50-75 |
| M7 | 10-20 | 35-75 |
| **Total to M7** | | **525-875 (7-12 times M3)** |

Before M4 (QA0 to MP2): 180-285 runs. My phase 2 figure of 115-225 used my own, smaller scope.

## Q2. Is G1-G12 complete (scope control)?

Missing: (a) script runtime and when it is selected (Finding 1). (b) The networking selection record (Finding 4).
(c) D10 bridges. The register names D41, D52 and S:720, but not D10. (d) A token allowance for QA0 and MC.
The CEO rule is to propose a budget for each milestone first (memory token-budget). (e) Approval of the statuses
of D65 to D73. (f) Does "playable" include terrain? The MP-before-M4 order is a risk choice, not a dependency (S:751).

## Q3. Silent closures

There is one: S:352/S:704 (Finding 4). Near-closures: G6/G7 are treated as done in the MC row (Finding 7). G1 would
reopen D10 by implication (Finding 5). G9 adds a determinism requirement, but it is labelled as a CEO choice, so
it is not silent.

## Q4. What the CEO decides first

1. G10 with G1: the bar and the order, with an exception list. Every plan waits on these two.
2. G5: the engine/mod line for the HUD and controls. The MC design waits on it.
3. G7 with G6: the order envelope and per-unit acceptance. The MC protocol, MP and G8 wait on it.
A token allowance for QA0 and MC is a precondition to start, not a design decision.

## What I ran

| id | command | claim tested | refute | observed |
| --- | --- | --- | --- | --- |
| P1 | `python3 -I producer/deps2.py` | The R:71-82 order satisfies its dependencies | 0 violations | 2: SCR has no position |
| P2 | same, extra edge MP->M4b | MP's blind gate is fine with M4b after M4 | 0 new violations | 1 new: "MP needs M4b". The checker can go red. |
| P3 | repaired order check (inline) | The repaired order is valid | any violation | 0 |
| P4 | `sed -n 352p,790p S`; grep repair/script in register | S:352 says "not selected"; MP omits repair; no G for scripts | the text says otherwise | "First prototype recommendation, not selected"; repair only in F3; scripts only in F7 and G8 |

Output: `producer/deps2.out`.

## Not checked

Seat probe numbers (NOT TRACED by the orchestrator, R:122). The entry counts per slice are my reading of R:73-82.
The token share for each run at the current model mix.

## What I changed

none (repository). Scratchpad: this file, `producer/deps2.py`, `producer/deps2.out`.
