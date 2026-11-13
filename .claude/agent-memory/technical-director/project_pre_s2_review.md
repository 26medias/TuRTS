---
name: pre-s2-review-facts
description: S2 orders split review (e196f3d, parent 8ba981d): leftovers, plant list, surface probe, ratchet rows, cycles; method notes
metadata:
  type: project
---

Verified 2026-11-12 on e196f3d (parent 8ba981d, pre-mp1/s2-orders):
- Own instrument: char multiset leftover = 2 commas (3 rustfmt trailing commas on re-wrapped fn signatures, 1 dropped by an unwrapped test call); item leftover = 4 reflows (record_dispatch_selection, record_plain_dispatch_selection, plain_move_reservation_capacity_available, test candidate_capacity_and_winner_indexes...). Also 9 new `//!` module-doc lines (not in the builder claim; comments only). A swapped statement shows at item level only.
- 50 visibility tokens (pub(super) and pub(in crate::orders)), stripped one at a time with `cargo test -p turts_sim --lib --no-run`: all 50 needed (E0603 36, E0616 11, E0624 3).
- Surface probe (92 names, `use crate::orders::NAME` appended to lib.rs): old 25 reachable, new 24; only `queue_movement_envelope` (pub(crate) in pending.rs, no re-export, used only in pending.rs and tests.rs) lost.
- Plant `return;` first in start_next_queued_order: 12 red at both builds, same names (4 lib orders::tests, 6 group_movement, 2 tests/orders.rs incl. completed_unit_follow_starts_the_next_queued_order_in_orders_phase). `cargo test -p turts_sim --locked --no-fail-fast`.
- Test list: 139 binaries, 1626 tests, equal at both commits.
- orders/ module graph: SCC {apply, pending, plain, queue, attack}; follow -> apply, queue; run -> all. run_orders 396 lines (run.rs:21-416), dispatch_pending_movement 531 lines (pending.rs:775-1306), apply_order 418, apply_unit_order 265, start_order 110.
- get_mut/remove::<UnitOrders> lines: apply 5, pending 7, run 4, plain 1, attack 2, follow 2, queue 2 = 23 + 1 test = 24 (equals the old file count).
- Forward merge with origin/mp1/integration (052beb1): clean; sim crate untouched there. Formula ratchet rows after merge: drop `orders.rs 0,3,1`; add `orders/behaviours/attack.rs 0,0,1` and `orders/run.rs 0,3,0`.
- Method: `bash -c` inside the runner works; strip loop script as one runner command; `tail`/`grep -c` are rtk-rewritten (use `rtk proxy`).
