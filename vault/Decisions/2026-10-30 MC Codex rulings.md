

## C389: replace the prepared P6 plant by P6-prime at the group pool's own reserved-cell test (2026-11-11)

Conflict: the prepared P6 (slots reused for the second group) makes conflicts_with_reserved return false (crates/turts_sim/src/movement.rs:209). Root verified that this function has two callers only, both for plain moves (movement.rs:714, 1883). Group pools use an inline reserved-cell test in free_slot_grid (movement.rs:461-473). So the prepared plant cannot change the second group's pool and cannot make the O14 slot check red.

Choice: P6-prime removes the inline reserved clause of free_slot_grid (files and hashes under target/mc-evidence/t22-run121/P6-adapted). It is the same fault class that row GMU names, at the code that the row judges. Expected named red: the GMU slot check reports a second-order slot at distance 0 from a first-order slot. The runtime run may also run the prepared bytes once to record that they stay green. The Task 22 reviewer judges the plant.

Evidence: movement.rs:209-221, 461-473, 714, 1883; RUN121 reply; plan Task 22 step 4; brief section 4 line 49.

Alternative not taken: keep the prepared P6 and record that it cannot go red. Then the red build "slots reused" of row GMU has no proof.
