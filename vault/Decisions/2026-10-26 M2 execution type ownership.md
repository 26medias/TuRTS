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


## Ruling R4 — user-authorized parallel execution

The user authorized isolated dependency-based worktrees for the remaining M2 plan. Lane ownership and frozen interfaces are in `docs/superpowers/plans/2026-10-26-m2-parallel-execution.md`. Canonical integration remains `/home/julien/.codex/worktrees/ee40/TuRTS`; review precedes integration. Preparation does not mark the original plan tasks complete.

**Reason:** Parallel lanes can reduce time after the Task 1–2 foundations are complete.

**Cost if wrong:** Integration conflicts may require reconciliation and revalidation.

Ruling R5: Task 3 additionally owns `crates/turts_sim/src/economy/bootstrap.rs` to add a validated `accepted_items` set to the Inventory ledger and bootstrap constructor, preserving inventory filter checks. No lane conflict; cost if wrong is small constructor/test rework. No gameplay policy change.


## Ruling R7 — Task 5 typed gather-stage completion

Task 5 replaces the current placeholder `String` in `OrderView.stage` and `GatherOrder.stage` with the approved typed `GatherStage`, including `Waiting { reason: WaitReason }`, matching design §§5/7 and immutable structured export. Allow the narrow bootstrap Idle constructor and command/test literal migration; read capabilities through `InstalledCatalog`. Gathering-specific staging operations are allowed. No transaction refactor. Task 7 adopts typed Idle after union integration.

**Reason:** the current string is a placeholder that loses the approved typed shape and structured waiting reason. This completes the already approved Task 5 interface without changing gameplay policy.

**Cost if wrong:** small constructor/view-pattern migrations.

## Ruling R8 — Task 6 queue/job/event atomicity and CostSource

Task 6 completes the private `WriteSet` across queue changes, checked `JobId` allocation, and event effects so payment, queue, id and event changes commit atomically. No new public API or transaction-architecture refactor. `CostView` gains schema `CostSource` because the current placeholder drops the endpoint source required by design §7. The coordinator composes shared gather/production hunks during integration.

**Reason:** production payment and queue lifecycle must commit consistently with checked allocation, emitted events, and their true endpoint sources.

**Cost if wrong:** small view/constructor and shared-commit migrations; the later Task 10 exporter will use the actual source.


## Ruling R9 — Task 8 accounting instrumentation declarations

R9 freezes the Task 8 declaration proposal in canonical .superpowers/sdd/2026-10-26-m2-economy/task-8-context.md: test-only AutomaticPhase/AutomaticSubject and TickBoundary; full TickAudit; crate-private capture helpers; complete raw diagnostic serialization including units and SimClock.tick_hz; and movement verification by an independent tick model. Expose the planned AuditTrail/owned records as trusted Rust diagnostics for external tests and Task 15; keep capture helpers private and add no player-facing API. Task 8 owns the previously deferred AuditState Minor: bounds, catalog, compiled durations and resource-presence metadata, plus necessary raw order/inventory/event-cursor snapshots.

**Reason:** the accounting oracle must observe complete operation/tick boundaries and independently derive movement, rate and duration expectations without trusting production events or post-state.

**Cost if wrong:** narrow extra ownership in economy views, gather/production boundaries, plugin tick capture, and serialization of complete snapshots; no gameplay policy or public API change.

**Execution status:** approved by root on 2026-10-27; Task 8 is complete after oracle rereview-2 and capture-evidence rereview passed. Source tip 7532c6715272a1d02b763d21a05b0abacf65b9d0 is integrated canonically through 333070c44ab7de4cf062822488768775db389d36. Post-integration verification: workspace 376 passed/0 failed/12 ignored, economy_accounting 42/42, fmt/check/Clippy/boundary/wasm passed.

## Ruling R10 — role-transform package preparation

For the approved role-transform preparation, a structural rename keeps exactly one producer per owner/map. Derived physical inventory receives synthesized capacity (base 2500); each slot retains a zero-valued declared Balance account, rather than omitting the account. Acme’s zero-economy transform is a no-op. No engine changes are allowed, and this preparation does not claim E3 behavior before the accounting oracle exists.

**Reason:** preserve declared ownership, synthesized runtime capacity, and zero-valued account invariants while preparing packages without changing engine behavior.

**Cost if wrong:** package mapping and fixture adjustments.

**Execution status:** role tool `409cc7a80d7dcab232594c5ec4f2edb2d5011df8` and collision fix `6c75eff218ebc30ee4af77a33cf279865bd7d45b` approved by Sol in /home/julien/.codex/worktrees/b8a3/TuRTS/.superpowers/sdd/2026-10-26-m2-economy/lane-role-variants-rereview-1.md. Original role suite: 25 Python + 6 Rust green; collision proof: RED/restored plus role/rename 2 and Rust 5 regression tests green. This remains structural preparation only; package integration is held and full E3 proof waits for Tasks 8/14.

## Ruling R11 — minimal diagnostic serialization support

Task 8 may enable the pinned Bevy Math 0.20 serialize feature in the simulation manifest and make only the matching Cargo.lock feature-edge change; the feature adds serde and glam/serde without renderer dependencies or version upgrades. Add narrow serde derives and private economy/audit.rs support as needed. Test-only existing serde_json in the pack manifest is allowed only if compilation requires it. The planned AuditTrail and owned records are public trusted Rust diagnostics for external tests and Task 15; capture helpers stay private, and this adds no player-facing API.

**Reason:** complete raw accounting snapshots need stable serialization of engine Vec2 and owned diagnostic records.

**Cost if wrong:** a small dependency-feature and lockfile/derive adjustment; no runtime renderer or protocol change.

**Execution status:** approved by root on 2026-10-27 for Task 8 only.
