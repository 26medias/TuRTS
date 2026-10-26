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


## Ruling R2

Ruling: Task 2 additionally owns `crates/turts_pack/Cargo.toml` for a dev-only workspace bevy_app dependency — planned `common::scenario(tick_hz)->App` helper needs direct type dependency; avoid artificial turts_sim reexport — cost if wrong one devdependency/pathlist rework, no runtime/gameplay policy change.

**Why:** The planned `common::scenario(tick_hz) -> App` helper requires a direct type dependency in `turts_pack` tests. A dev-only workspace `bevy_app` dependency provides it directly and avoids an artificial `turts_sim` re-export.

**Cost if wrong:** One dev-dependency and path-list rework; no runtime or gameplay policy change.

**Execution status:** Task 2 owns this manifest path; `/root/m2_task_2` was informed the dependency is allowed.


## Ruling R3 (before Task 2 fix round 1)

Ruling: bootstrap balance initialization and owner/seed validation follow actual Slots resource count; host default and M2 fixtures remain two slots — honours every configured slot and prevents ghost/missing accounts — cost if wrong small count/getter/test rework; no new supported-capacity promise.

**Resolution of the slot-scope review item:** This ruling resolves the scope question by making the actual `Slots` resource authoritative. It is a scope decision, not a claim that a test passed. The host default and M2 fixtures remain two slots; this creates no capacity promise.
