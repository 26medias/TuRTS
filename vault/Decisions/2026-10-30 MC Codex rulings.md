

## C389: replace the prepared P6 plant by P6-prime at the group pool's own reserved-cell test (2026-11-11)

Conflict: the prepared P6 (slots reused for the second group) makes conflicts_with_reserved return false (crates/turts_sim/src/movement.rs:209). Root verified that this function has two callers only, both for plain moves (movement.rs:714, 1883). Group pools use an inline reserved-cell test in free_slot_grid (movement.rs:461-473). So the prepared plant cannot change the second group's pool and cannot make the O14 slot check red.

Choice: P6-prime removes the inline reserved clause of free_slot_grid (files and hashes under target/mc-evidence/t22-run121/P6-adapted). It is the same fault class that row GMU names, at the code that the row judges. Expected named red: the GMU slot check reports a second-order slot at distance 0 from a first-order slot. The runtime run may also run the prepared bytes once to record that they stay green. The Task 22 reviewer judges the plant.

Evidence: movement.rs:209-221, 461-473, 714, 1883; RUN121 reply; plan Task 22 step 4; brief section 4 line 49.

Alternative not taken: keep the prepared P6 and record that it cannot go red. Then the red build "slots reused" of row GMU has no proof.

### C390: a second lane for native Cargo work only (2026-11-11)

Conflict: the handoff rule "One compiler/browser/GPU lease at a time" makes all remaining work serial. The CEO asked for more speed with no loss of quality. Root checked the reasons for the rule against the machine on 2026-11-11: disk 528 GiB free (the rule came from low disk), 28 CPU threads with CARGO_BUILD_JOBS=4 for each build, 45 GiB memory available. The valid reasons that stay are timing (headed browser runs, measurements) and the one GPU.

Choice: a second lane may run native `cargo test`, `cargo check` or `cargo clippy` (no `--target`, no browser, no GPU, no measurement) at `nice 19`, with its own cache `target/<mc-name>-native`, while another lane holds the browser/GPU lease. Runner: target/mc-evidence/gate-runner/exec_in_lane_v3_native.py with `--native-coexist`, sha256 3aaa138d32ee766f7f53970d1225eeb01272e7bdad00d9df25eb4317f49edcc4; it is exec_in_lane_v2.py with that one branch, and the receipt records `native_coexist` and the other compilers at the start. All headed browser runs, the GPU and the Task 22 measurements stay exclusive. A browser run that fails while a native coexist run is active is repeated alone before it is judged. Acceptance gates after a merge stay on exec_in_lane_v2.py with no other compiler.

Limit: exec_in_lane_v2.py refuses to start while any compiler is active (line 23-24). So a native coexist run must end before the other lane starts its next v2 command. Root runs it only inside one long command of the other lane.

Evidence: exec_in_lane_v2.py:23-24, 40; brief section 4 line 52 (the orchestrator decides the number of lanes, cache and disk handling); `nproc` 28, `free -g`, `df -h` on 2026-11-11.

Alternative not taken: keep one lease for all work. It keeps the estimate of 20 to 28 hours and finds a compile error in the Task 22 native check hours later.

### C391: process rules reviewed; lanes run in parallel (CEO order, 2026-11-11)

The CEO read the single-lease rule and ordered: review every process rule; remove each rule that limits speed and does not protect quality. Root reviewed the rules of the handoff, the brief and the board.

Changed:
1. Single compiler/browser/GPU lease: removed. Each lane has its own cache and compiles in parallel. Headed browser runs stay one at a time through the existing GPU lock (`gpu-runtime.lock` in the headed wrapper). Timing measurements run only when no build is active. Runners: gate-runner/exec_in_lane_v4.py (sha256 8eefe1c0c51b5a6da95a6dad264f0dd4b82cf9ad904256e8147adaf7109da8cd) and mc_headed_v4.sh (ab0062c4f96a2030f7b060dbfba468e966d9358759892f889475c58d89937a9a). They are v2/v3 without the other-compiler check and with 10 build jobs (was 4); every receipt field stays, and `other_compilers_at_start` is added. C390 is superseded.
2. One cache that moves between lanes: removed. Disk 528 GiB free.
3. Local re-gate after each merge: required only when the merged tree differs from a tree that passed the full gate set. Root compares the tree hashes. Hosted CI stays for each merge.
4. Serial task order: Tasks 21 and 22 run their runtime in parallel. The Task 24 regression (REG1) runs during the CEO play of Task 23, not after it.
5. Hosted CI: work continues while a hosted run is active; Root reads the result when it ends.
6. Run ledger: one row per run stays (it is the cost record). The row is written at the dispatch, not as a separate step before it.
7. Budget stops (170 runs, 4 times a task estimate): Root keeps the count and reports when a limit is passed, but does not halt the build. The CEO can restore the halt with one word.
8. A browser run that fails with a timing symptom during a parallel build is repeated one time before it is judged; the repeat is recorded.

Kept, because they protect quality or belong to the CEO: an independent reviewer who runs the tests and one fault of its own before each merge; red, exact restore, fresh green for each fault; receipts with head, clean source and source hashes; the frozen stream hash; no design change that the player sees without the CEO; no fault retirement without a decision note; the CEO plays PT1 and PT2; no merge or push to main; explicit git paths; Simplified Technical English.

Evidence: CEO messages of 2026-11-11 in the Root session; exec_in_lane_v2.py:23-24, 40; mc_headed_v3.sh (flock line).

Alternative not taken: keep C390 only (native work in a second lane). It keeps all browser work and both tasks serial.
