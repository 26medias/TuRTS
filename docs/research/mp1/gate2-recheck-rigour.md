# MP1 plan revision 2 re-check: rigour

Source: subagent report. Agent output; claims are leads until verified.

VERDICT=FAIL HIGH=1

The repair closes all 10 of my gate-2 findings and every High of the other four lenses. One new High remains, and it is a decision: Q11 sits on the zero-slack path at the same offset as the lane-0 freeze. Revision 2 at f7090aa, design 6.1, slice head 03aef38.

**My gate-2 findings**

| Id | State | Plan line |
| --- | --- | --- |
| H1 | CLOSED | P:762, P:136; D:1048 and D:88 edited; "three slots" has no hit left in the D body |
| H2 | CLOSED | P:185 `A3^`, P:189 A3 as a plain input, `mp1-tree` P:412-413 |
| M1 | CLOSED | +1 rule at `mp1_plan_check.py:133-136`, P:85-86 |
| M2 | CLOSED | E1-E5 applied in D; ALR2/s2 plant at P:373 |
| M3 | PARTIAL | P:20-61; see L1 and N1 below |
| M4 | PARTIAL | C-UI CLOSED (P:1020-1023: `frontier_texts` gives `exit=101`, plant on H1); C-UNITS plant is defective (Medium M2) |
| L1 | CLOSED | P:89 |
| L2 | CLOSED | P:65-66, P:838 |
| L3 | CLOSED | P:933 (threshold), P:671 (export scan), P:943 (changed counter) |
| L4 | CLOSED | `mp1_plan_check.py:247-251`; mutation m4 is RED |

**Other lenses' Highs** (each closed by a task step whose check can go red)

- qa H1: CLOSED. `lane+px` plants with Headed ≥1, F-HEAD (P:426). Example red: GHO1 (a)-(e) in `UI-PLACE`.
- qa H2: CLOSED. `Q-PLANTS` (P:585-588): todo self-test `exit=1`; plant "compare the exit only".
- qa H3: CLOSED. `I2-PIX` (P:1194) re-runs the stage-1 rows with models on; the finder re-calibration needs a red before and after.
- TD H1: CLOSED. `L0-STORE` contract test (`exit=101`); plants (a) commit order, (b) clone counter.
- TD H2: CLOSED. Stacked edges, and the `weapon_enabled` stub with plant (c) (P:656).
- TD H3: CLOSED. `order_writers.rs` scan with a planted direct write (P:671, P:679).
- pres H1: CLOSED. `FX2` with "turret does not turn".
- pres H2: CLOSED. `PRES-PROBE` probe 2 red today, then the `UI-HUD` counter (P:943).
- pres H3: CLOSED by file list. No plant on the spawn path itself; the red comes only from the `I2-PIX` model re-run and REG2-2.
- ux H1: CLOSED. `UI-LABEL` "labels off" plus GOAL1 (i).
- ux H2: CLOSED. `WEB-NOGPU` NOGPU1 red on today's build.
- ux H3: CLOSED. `C-UI` steps 1 and 4.

**Schedule** (my own `cpm2.py`, same edge rules as the plan)
- Path 60, `L0-EXIT` 14, C1 40, `L0-EXIT2` 41, PT3 59, REC 60. Same path as P:244.
- 74 tasks; runs 39 / 97 / 71 for lane 0 / stage 1 / stage 2; 41 headed. No cycle, no reversed edge.
- `L0-SPLIT` option: I agree with 59 by reasoning, not run. The MP1 chain loses 2 runs, but the slice chain has only 1 run of slack. It saves 1 run for 1 more task and 1 more review, so not applying it is right.

**Coverage** (my `cov.py` against D 6.1)
- 42 rows, 52 parts. Each part has one owner with the D 6.1 stage. NOGPU1 and MCX1/s1 are stage 1; NC1 is stage 2.
- 30 scope items, 21 open points. E1-E11 each have a fate (P:1389-1401).

**New findings**

- H1 (High). Q11 and open point 19 residual (P:804, P:1380) block `L-PAY` (zero slack, starts at 14, P:249) with no recommended option. The "held over a window" rule changes D:635 and the PAY1 check ("A tick that cannot pay … `WaitingForFunds`"), and no erratum covers that. "The job record exports the held state" also adds or changes a stage-1 record that `L0-EXIT` freezes at the same offset. Repair:
  - decide Q11 now, and put it in the header table as kind A, due before 14;
  - add erratum E21: the job record gains `held: bool`, and the 6.4 state stays per tick;
  - list `held` in the `L0-WIRE` shapes and in 5.1 item 2.
- M1 (Medium). D:1597, row H24 (E10), gives no field names and no defaults for colour, size and font, and has no RON example. `C-UI` (P:1016) writes `Some((show: Always, own_only: false))`. The existing pattern `WorldTooltipStyle` (`crates/turts_mod_api/src/interface.rs:359-364`) has every field required, so that RON fails to parse unless defaults exist. `L0-API1` can pick names, but D-I (a) requires the table in D. Repair: E21 gives `colour: String = "#FFFFFF"`, `size: u16 = 14`, `font: Option<String> = None`, the defaults of `show` and `own_only`, and one RON line.
- M2 (Medium). P:1009-1010: the `C-UNITS` plant uses `placement.gap`, and K2 `placement` is `UNBUILT` until `L-CON1` merges (P:466; around offset 20, `C-UNITS` window 11-14). The gate copy leaves that field out, so the plant can give the K2 text only if validation runs before the `UNBUILT` refusal (NOT TRACED). Repair: plant on a lane-0 field instead (for example, an unknown field gives the "did you mean" text), or move the K2 plant to `C1-FINAL`.
- L1. The D:2349 stage table, row X21, still says the `player` agent "plays the stage-1 loop". E18 changed D:65 and X21 in section 4 but not this row. This is a second stage-table disagreement after X18. Repair: one E21 line.
- L2. The latest times are late for D-B (2) and P2 (c): `L-ROOM0` builds the seat table at 0-3, but the table says 14. D-F (b) changes `C-UI` H1 (at 14) and GOAL1 (e), but the table says 22. P8 (a) removes a feature, so it is kind C, not kind B (P:38).
- L3. Stale text: `mp1-moves` still gives NOGPU1 and NC1 their pre-E8/E9 D stage (P:399-400), and the STO1 split reason "omits STO1" is stale (P:391). The script accepts either stage (`:195`). P:1568 says "19 errata", but D 6.1 has E20.
- L4. My three mutations all stay GREEN:
  - n1: the `L-TGT`/A3 tree row deleted and A3 made a merge-after. The tree rule is the author's own claim.
  - n2: GHO1 `lane+px` changed to `lane`.
  - n3: `I-PLAY1` removed from the inputs of `C1-PLAYER1`.

**What I ran**
- Plan check: GREEN, `exit=0`. `--self-test`: `exit=0`, 13 of 13 RED.
- My n1, n2, n3 mutations: `exit=0` each (GREEN).
- `cov.py` `exit=0`. `cpm2.py` `exit=0`.
- `cites2.py`: 24 citations at 03aef38, all hold. Three needed a manual read: `web-lib.rs:144` (`build_app`), `values.rs:39`, `feedback.rs:477`.
- `git diff a9a5fea f7090aa -- D`: every hunk maps to E1-E20 except the Status line, the revision history and a budget note.
- `completeness.py --self-test` 21/21, `exit=0`; `--enforce` `exit=0`.
- I ran every script with `-I -B`. No `__pycache__` came from me. Two untracked files in the tree, `docs/research/mp1/gate2-recheck-qa-lead.md` and `docs/research/mp1/gate2-recheck-technical-director.md`, are other agents' reports.

What I changed: none. My scripts and outputs are in /tmp/claude-1000/-home-julien-Projects-TuRTS/bf418660-b272-4bb3-8628-8d9ef2677d1d/scratchpad/gate2-rigour/ (cpm2.py, cites2.py, cov.py, n1.md to n3.md, d61.diff, r2_check.out, r2_self.out).
