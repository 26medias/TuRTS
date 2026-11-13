---
name: mp1-architecture-input-facts
description: Verified 2026-11-11 facts for the pre-MP1 slice and MP1 design input (TuRTS-mp1 at 8d6902c): refutation dependency on economylog, stream before==after trap, grid within filters by distance, conservation.rs oracle
metadata:
  type: project
---

Verified 2026-11-11 in /home/julien/Projects/TuRTS-mp1 (branch mp1-design, 8d6902c):
- Of 41 M2 and 17 M3 refutation cases, only E8-mixed-cost-partial-commit gets its red from
  economylog.rs (message at economylog.rs:2349). E6/E2-E5 reds come from
  turts_pack/tests/common/accounting.rs; C1-C17 from common/conservation.rs (a ledger oracle,
  1,088 lines: BAL, RES, SHOT, DMG, REFUND, DANGLING). E8/E9 others come from tests/economy.rs.
- Validator users: tests economy.rs:9, el1.rs:6, mc_orders.rs:3 include src/economylog.rs by
  #[path]; session.rs:1053-1059 unit test; main.rs:7,109-129 sink.
- Frozen stream test compares op.before == op.after over the FULL AuditState
  (m3_m2_stream.rs:119). Removing fields from AuditState can flip it: shrink the log format
  separately, or count it in the one re-freeze.
- grid.rs `within` filters by exact distance and sorts by (id, index) (grid.rs:85-87,103-109):
  results do not depend on cell size. Movement callers pass reach r + max_radius(all units);
  only refresh_slot_clearance (movement.rs:1110-1117) uses max_radius as a semantic cap, and
  its clearances feed only the prefilter has_potential_free_slot (movement.rs:1166-1204).
- Death has one path: transaction_m3.rs:354 remove_unit, called from combat.rs:405.
- No match state, lobby, teams: SLOT_COUNT = 2 const (slots.rs:7). NET_PROTOCOL_VERSION 4
  (turts_net_protocol/src/lib.rs:13). ENGINE_API_VERSION 0.2.3 (turts_mod_api/src/lib.rs:52).
- Hot files by MC commits (bb611bd..HEAD): economylog 16, orders 12, movement 12,
  widgets/mod.rs 12, interaction lib 11, visuals 11, core_adapter 10, session 9.

- CORRECTION (gate 2026-11-12, verified): movement has THREE branches: plain_progress
  (movement.rs:1843-1879), no-progress bare MoveTarget (:1880-2085, all engine trips + MoveTo;
  the M2 stream runs only here), pooled (:2086+). Lone pool is collected on arrival
  (:2383-2404, :2497-2508). Float differs: bare `normalize()*step` (:2003) vs pooled
  `*= step/len`. 27 within() sites in movement.rs prod. Ruling: one step function, pool optional.

**Why:** the pre-MP1 slice and MP1 gates cite these.
**How to apply:** re-check line numbers after the slice lands. See [[mc-architecture-audit-facts]].
