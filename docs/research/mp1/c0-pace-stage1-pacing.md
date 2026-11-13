# MP1 stage-1 pacing rerun (game-designer, plan task C0-PACE, open point 19)

Source: subagent report. Agent output; claims are leads until verified.

RESULT=RECOMMEND-c (start credits 1,200 to 1,400), plus d for the small tail. Option a fails. Option b works but leaves the hint line blank.
Files: /tmp/claude-1000/-home-julien-Projects-TuRTS/bf418660-b272-4bb3-8628-8d9ef2677d1d/scratchpad/c0-pace/ (call it C0/). Status: all numbers are Open (16.2, S:807); they are proposals from a model, not measurements.

CONSTANTS CHECK (C0/consts.py, exit 0, 0 mismatches). `docs/research/mp1/probes/pacing/pacing.py` `new` set equals the design (the spec file, `docs/superpowers/specs/2026-11-12-mp1-playable-skirmish-design.md` in the TuRTS-mp1 worktree):
- Start 1200 (l.304); free income 3/s (l.309); hauler 500 credits, 15 s (l.305); load 300, gather 15/s, unload 60/s, speed 2.4 (l.306).
- Rifleman 7 s (l.310); Launcher 10 s (l.311); Engineer 250 (l.312); Tank 20 s (l.313); Turret 600 (l.314); Power Station 12 s (l.323).
- Not in the section 5 table, so taken from `docs/research/mp1/input-game-designer.md` (the GD file): Rifleman 100, Launcher 250, Tank 700 (GD:38), Power Station 300 (GD:56), Turret 20 s (GD:57).
- Power rule `40+30*tur > 100*ps`, half speed: l.323 and 6.2 (`Completed` counts). Alert: cooldown 90 s, clear 15 s (7.9 table, l.2064); hold5.py uses the same.
- cmp of pacing.py against the design copy: identical.

MODEL: C0/sim.py has the pacing.run arithmetic with a hint-follower driver. Its self-test (exit 0) shows B 7.2 s, C 14.5 s, D 4.0 s equal to pacing.run.
(1) TIMELINE: C0/timeline.out. Key rows of C (F: 2 Riflemen and a Tank):
| t | action | credits | income/s | hold |
| 8 | queue PS | 1223 | 11.6 | no |
| 15 | queue hauler | 1150 | 11.6 | no |
| 34 | queue Turret | 1065 | 11.6 | no |
| 40-42 | queue Rifle, Rifle, Tank | 582 | 11.6 | no |
| 54 | Turret done | 35 | 20.3 | no |
| 57.2-64.4 | HOLD 7.2 s (ends on a +300 delivery) | 1 | 20.3 | yes |
| 84.1-98.6 | HOLD 14.5 s (Tank, 35 credits/s against 20.3) | 3 | 20.3 | yes |
B (3 Rifles) has only the first hold. One "Not enough credits" sound, at 57 s. Cause: 1,800 spent by 54 s against about 1,660 earned, and income comes in lumps of 300.

(2) OPTIONS (follower; R3 = 3 Rifles, RRT = 2 Rifles + a Tank; "+10" = every step 10 s slower; "w30" = worst over 0 to 30 s slower; MC = 600 runs, each step 0 to 12 s slower, four unit lists):
| option | R3 once/+10/w30 | RRT once/+10/w30 | first Tank (s) | 4 units (s) | MC worst, share above 4 s |
| d, rev 6 as is | 7.0/0/7.0 | 14.4/0/14.4 | 98.7 | 71.7 | 15.0 s, 47 % |
| a, H4 after PS complete | 1.6/0/1.6 | 25.1/0/25.3 | 70.3 | 51.1 | 29.8 s, 43 % |
| b1, F after 2nd hauler complete | 0/0/0 | 14.4/0/14.4 | 98.7 | 72.1 | 10.4 s, 13 % |
| b2, F after 2nd hauler delivered | 0/0/0 | 0/0/0 | 114.8 | 101.8 | 0 s, 0 % |
| b3, F after Turret complete | 0/0/0 | 5.1/0/5.1 | 98.7 | 81.0 | 2.8 s, 0 % |
| c1, start 1300 | 0/0/0 | 2.8/0/3.6 | 82.6 | 66.1 | 12.1 s, 20 % |
| c2, start 1400 | 0/0/0 | 0.7/0/0.7 | 79.7 | 66.1 | 7.7 s, 1.3 % |
| c3, income 4/s | 1.4/0/1.4 | 4.0/0/4.0 | 83.3 | 67.1 | 12.9 s, 24 % |
| c4, hauler cost 400 | 0/0/0 | 2.8/0/3.6 | 82.6 | 66.1 | 12.1 s, 20 % |
- Option a: F shows from 14 s, so the player trains units before the hauler. The second hauler is done at 67.5 s against 45.1 s. It is worse.
- b2 needs a new `HintCondition`: row H14 (l.1589) has no "delivered" and no "credits above N". b1 and b3 use `OwnUnits(.., Complete, ..)`, so they need no new mechanism. But F is `hint_fallback` (a plain string) today. To gate it, F becomes a situation hint (starts and done by `StepCurrent`). The hint line is then blank from 15 to 32 s and from 34 to 54 s. That repeats the PT1 failure "no first action" (l.2962).
- The "+10" column is 0 in every option: a player 10 s slow at each step never holds. The hold hits the fast follower, so a first-time tester rarely meets it; the CEO may.
- Sweeps (C0/sweep_c.out): the hold is a cliff. +100 start removes the R3 hold. The Turret at 500 would also work (not on your list).
- Content-number options leave the hint table as it is.
Match length (C0/match_opts.out; control reproduces l.341-345): start 1400 gives 11.5 min (was 11.4); 20.2, 20.2 and 3.5 min unchanged; two equal careful players: no end, ore gone at 30.4 min. Income 4/s and hauler 400 move rows by at most 0.1 min. The macro model is crude (l.359).
Rows that change at start 1400 (C0/s5rows.out; control reproduces S1 445, S5 25, S8 30): S1 credits at 180 s: 445 becomes 645. S7: the 4 s hold goes. L1: 333 s becomes 266 s (C0/run_l1.py). S4, S5 (25 s) and S8 (30 s) are unchanged, so the alert-sound test keeps its overspend case.

(3) IS A HOLD A PROBLEM? Yes, at the length of the two cases I measured. A wait of 4 s or less with the visible line "Not enough credits" (one sound only, cooldown 90 s) teaches that credits limit the player. The repeat of the sound was the PT1 failure (l.2962); the 7.9 rule fixes that, so one beep is fine. A wait of 14.5 s in the first 100 s is dead time, not teaching. I would defend: no hold above 4 s for a hint follower in the first 120 s (the existing target); a hard limit of 10 s, the X3 trigger. A hold with a cause shown may reach 8 s once. NOT TRACED: whether X3 will ever show. My model has the held state flicker, because free income pays 1 tick in about 12. In the 14.5 s span the longest unbroken run is 1.3 s (C0/x3.py), and the l.326 merge applies only to the table. If `JobOnHold(10)` needs 10 s unbroken, X3 never shows. That needs a decision (owner game-designer with ux-designer).

(4) RECOMMENDATION c2, exact design text:
- l.304 becomes: `| Starting credits | 1,200 | 1,400 | S:805 ("enough credits to establish power"); on the stage-1 hint order a player who follows hint F at once holds 0 s (3 Rifles) or 0.6 s (a Tank) where 1,200 gave 7.2 s and 14.5 s; the other changes remove the holds |`
- l.330 "445 credits at 180 s" becomes "645". l.332 "one hold of 4 s at 95 s" becomes "no hold". l.336 "333 s" becomes "266 s".
- l.349-354: replace with "The first 3 minutes: with 1,400 starting credits, no hold above 4 s in S1, S4, S7 and on the stage-1 hint order when the player follows hint F at once. In 600 runs with each step 0 to 12 s slower and four unit lists, 1.3 % hold above 4 s; the worst is 7.7 s (one Tank, one hold, one sound). A hold comes when the player overspends (S5, S8); the alert sounds once (7.9) [player H3]. This is a proposal (C0-PACE)."
- Hint table: no row changes. Exit item l.2507: delete "of the revision-4 order (open point 19)".
- l.2692: "closed by C0-PACE, section 5". Open point 19 (l.2608-2612) becomes: "19. Closed (C0-PACE): the model was run on the stage-1 hint order; starting credits 1,400 (section 5). Residual, needs decision: how `JobOnHold(10)` counts a flickering hold (6.4; NOT TRACED). Owner: game-designer with ux-designer. It closes at gate 2."
- Text to follow: l.3009 "Credits 1200" becomes "Credits 1400"; l.2451 (HUD5) too, if its scenario uses the start value (NOT TRACED).
- Acceptance rows: GOAL1 and ALR2 pass conditions: no change. ALR2 case A forces 0 credits, and S8 still holds 30 s. PT3 (oracle list): only the step-1 text in l.3009. INC1: no change (the 166 s floor uses start 0; income and hauler cost are unchanged).
Caveat: 1,400 is a cliff move of about 200 credits. If the CEO wants more margin, 1,500 gives MC worst 4.9 s, but that also leaves the macro model flat. A headless duel must check the early-rush balance (not run).

WHAT I RAN (all exit 0, in C0/): consts.py; `sim.py` self-test (refuted if B, C, D differ from pacing.run: they are equal); timeline.py; run_opts.py, final.py, mc.py, run_c.py (options and sweeps); run_match.py (refuted if the control differs from l.341-345: it matches); run_s5rows.py and run_l1.py (control matches l.330-336); x3.py; static14.py. Not run: any engine code (NOT TRACED: engine `JobOnHold`).
