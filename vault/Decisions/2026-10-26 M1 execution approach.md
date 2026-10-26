---
type: decision
date: 2026-10-26
tags:
  - turts
  - decision
  - process
ai-first: true
source: mcp
status: active
---

## For future Claude
M1 tasks run one implementer subagent per task (Sonnet at xhigh effort; Opus for the networking spike and the replica logic). The orchestrator verifies each task by running its tests; qa-lead reviews at the spike gate and at the end of M1.

**Who:** orchestrator (studio call), applying the CEO's model tiers and the rule to size the Anvil to the task.
**Why:** the plan carries the design decisions and every test names the build it fails on; a second reviewer per task doubles cost for little gain once the orchestrator runs the tests itself. The risky points get a full review: the spike gate (networking choice) and the end of M1 (all acceptance tests).
**Order:** 1 → {2, 5} → {3, 4, 6a, 8b} → 6b → spike gate → {7a, 8a-1} → {7b, 8a-2} → {7c, 10} → 9 → 11 → 12 → 13.
