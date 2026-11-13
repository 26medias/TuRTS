---
name: mp1-reconcile-facts
description: Verified 2026-11-11 facts from the MP1 design rev 4 reconcile pass (TuRTS-mp1, mp1-design d65a4cb + uncommitted edit): code anchors for alert triggers, Welcome, CardCell, reason keys, tick log, build menu, replica clones; choices written
metadata:
  type: project
---

Verified 2026-11-11 in /home/julien/Projects/TuRTS-mp1 (branch mp1-design, head d65a4cb):
- `AlertTrigger` has `OwnUnitDamaged`, `OwnUnitDestroyed`, `ProductionComplete`, `GathererWaiting(f32)` today
  (crates/turts_mod_api/src/interface.rs:387-399). `AlertDef` has cooldown, coalesce_radius, no kind (:373-384).
- `Welcome` already has `slot_id, tick, tick_hz, match_id` (turts_net_protocol/src/messages.rs:26-31).
- `CardCell { row, column }` exists for orders (turts_mod_api/src/orders.rs:18, :25-28); ProductionDef has no card/hotkey.
- Wire keys: queue_full messages.rs:241, unknown_order :256, not_applicable :257.
- `--tick-log` holds positions only; `--order-log` exists (turts_server/src/config.rs:49, :162).
- Build menu rows follow `producer.recipes` order (turts_client/src/widgets/sources.rs:395-397).
- Replica clones the candidate per delta at replica.rs:432-443 and :451; replace_keyed at :1029; :1090 is not a clone.
- Every section-12 row in the 4 repair files is verbatim in the design except ORDW1 (qa-lead's newer, longer row).
- Choices written in rev 4 part 5: reason keys site_outside_map, site_out_of_area, site_blocked, site_near_building,
  site_near_resource, queue_full (no site_cap); both site caps make an order WAIT; 6.7.4 MatchDef is the match-file
  authority (+ open_sites_per_slot); 6.7.13 is the lifecycle wire authority; terms list in design 1.1; job "stage"
  renamed "job state" (wire field `state`).

**Why:** the gate-1 re-check and lane 0 cite these.
**How to apply:** re-check line numbers after the pre-MP1 slice moves files. See [[mp1-design-repair-facts]].
