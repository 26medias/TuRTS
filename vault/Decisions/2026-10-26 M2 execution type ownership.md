---
type: decision
date: 2026-10-26
tags:
  - turts
  - m2
  - execution
ai-first: true
source: mcp
---

# M2 execution type ownership

**Status:** studio-ruling for execution of the approved 16-task plan.

## Ruling R1

During Task 2, add `crates/turts_sim/src/economy/production.rs` to owned paths for data-only `ProductionQueues`, `ProductionJob`, and `JobStage`; introduce foundational `Endpoint`/reservation records in Task 2 `ledger.rs` if those job types require them. Task 3 adds their validated mutation operations. Task 6 extends the existing production module with queue/start/advance behavior rather than creating it anew. Keep empty order/queue capability state and immutable view types bootable without automatic production.

## Why

P:87 assigns the job types to `production.rs`; P:135 requires boot queues and canonical `AuditState`/`WorldView` before Tasks 3 and 6; P:250 assigns creation of the module to Task 6. Without R1, Task 2 has no canonical location for its bootable queue type. D:197–219 provides the data records. This moves only type scaffolding and adds no gameplay behavior.

## Cost if wrong

A few type declarations and module ownership edits move between early commits; runtime policy and final API remain unchanged.

## Source anchors

- Sol execution preflight: `docs/superpowers/plans/2026-10-26-m2-economy.md` anchors P:87, P:135, P:250 and design D:197–219.
- Full 16-task and 92-pair scan: `.superpowers/sdd/2026-10-26-m2-economy/progress.md` under “Execution preflight artifact”.
