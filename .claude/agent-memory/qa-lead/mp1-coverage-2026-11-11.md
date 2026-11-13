---
name: mp1-coverage-2026-11-11
description: MP1 yardstick coverage build - R4 dependency chains that force later rows, EC03/EC05 M2 tests seen red, no CORE item asks for a goal line
metadata:
  type: project
---

Verified 2026-11-11 on mp1-design 978921b (scratch worktree, removed after).

- MC coverage counts: 81 in, 15 done, 85 later (36 of them later:MP1).
- MP1 coverage I built: 55 in, 69 done, 57 later; gate --enforce exit 0, self-test 21/21.
  Files: scratchpad mp1cov/ (gen.py, coverage.md, rows.md, path.md, mp1-design-spliced.md).
- R4 forces `later` even where MP1 builds the flat-ground part (each probed red as `in`):
  CO03/CO02/MF06 need MP01 (M4); CO21 needs GM07 (M4b); MF02 needs MD12 which needs MF04 (M9);
  AN02 needs CB10 (MB); OR26 needs CB05; MF10/TM02 need TM01.
- EC03 and EC05 exist since M2: crates/turts_pack/tests/economy_gather.rs:471 and :656. Green on
  base; red under mutation (choose_receiver max_by, gather.rs:~997; no closest_deposit reseek).
- UI31 Objectives panel is COMMON (inventory-engine.md:205): the gate never asks for a goal line or
  a first-action hint. Only a new row (GOAL1) or PT3 can catch the CEO's 2026-11-11 failure.
- A `test` cell must be exactly one id (completeness.py:110); put second tests in a note column.
- `python3 -B` plus `sys.path.insert(0,'scripts/qa')` imports completeness.py with no pycache.

Related: [[pre-mp1-slice-gate-2026-11-11]], [[gap-audit-2026-10-28]].
