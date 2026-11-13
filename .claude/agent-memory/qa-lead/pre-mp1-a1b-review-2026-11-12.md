---
name: pre-mp1-a1b-review-2026-11-12
description: A1b review at c536c8c - ledger refusal rule blind to pending_movement_orders; writer allow list is per file; 3 replay-off baseline reds owed to A1c
metadata:
  type: project
---

Verified 2026-11-12 in worktree TuRTS-pre-a1b-review at c536c8c (base 708c335), cache mc-pre-a1b-review.
Evidence: target/mc-evidence/a1b-rev-*; plants: scratchpad qa-a1b-review/plant.py. Report:
TuRTS-mp1/docs/research/mp1/review-slice-a1b.md. Verdict PASS, 0 High, 3 Medium owed before A1c.

- Replay off (TURTS_AUDIT_REPLAY=off), no plant: 3 baseline reds = validator self-checks
  economylog.rs:453 (first open slot), economylog.rs:331 (phase Other), economy.rs:1145 (1 ulp).
- F1 slots rposition: ledger green; session.rs:840/:1125, economy.rs:4596 red (cover kept outside ledger).
- F2 Position write in combat.rs (allowed file): scan green, ledger green; replay red. Allow list is per file.
- F3 cost-1: ledger green; economy.rs:4717 accounting oracle red.
- F4 log probe: refused command changes pending_movement_orders -> ledger Ok, replay Err.
- F5 LedgerAudit.unit_orders empty: field-equality test green (fixture fields empty); economy.rs:908 red.
- E8-mixed release replay off: clean green, plant red at economy.rs:3808 with ledger text.
- exec_in_lane_v4 receipt does not record TURTS_AUDIT_REPLAY; prove propagation by the baseline reds.
- Time: forge log build in debug ~20-25 s dominates ledger_logs; ledger cost below noise; economy binary 27-47 s noise.
- Scratchpad is shared with builders (their plants.tsv lives there); use a subdir.

See [[pre-mp1-a1a-review-2026-11-12]], [[pre-mp1-slice-gate-2026-11-11]].
