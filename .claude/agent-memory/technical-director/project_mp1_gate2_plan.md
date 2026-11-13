---
name: mp1-gate2-plan-facts
description: Verified 2026-11-12 facts from the MP1 plan gate 2 (plan 24a18e1, slice head a6f3f66): check-script merge-after trap, combat.rs shape, SimPlugin sites, replica Candidate, path numbers
metadata:
  type: project
---

Verified 2026-11-12 (plan TuRTS-mp1 24a18e1; code TuRTS-pre-integration pre-mp1/integration = a6f3f66, S3 merged, S45 not):
- mp1_plan_check.py:103-107 sets end = max(build end, merge-after ends) (0 runs for the rebase) and :122 counts merge-after as reach, so the conflict list hides build-parallel pairs: L-TGT|A3 (combat.rs, orders/), L-TGT|L-PWR, UI-HINT|UI-PLACE, UI-RESULT|L-LOBBY-C1, L-FX-WIRE|A2b2.
- combat.rs 2093 lines, code 1-410; run_combat :82-257 holds the per-tick scan :90-109 and the cell :110-117 (fold max weapon range).
- SimPlugin built at harness.rs:13 (headless_app, 51 files use it), server main.rs:68, session.rs tests :792 :963 :1116 :1747.
- Replica Candidate struct replica.rs:82; candidate_at :432-446 clones each field; D:2257 "MP1 adds no field to the replica candidate" vs D:1381 "replica is the only source". No plan task owns client storage of the new 6.8 records.
- Path: plan 52; A3 stacked on A4 branch 51; + L-PAY merge-after A1d 49 (scratch gate2-td/plan_alt*.md).
- Slice plan SP:264 rule (no MP1 test pins single-move ops or blocked engine trip until A2c) is not in the MP1 plan.

**Why:** the plan repair and its re-gate cite these.
**How to apply:** re-check lines after S2 and S45 merge. See [[mp1-rev6-facts]].
