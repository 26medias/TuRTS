# Slice S1 review: technical-director

Source: subagent hand-back, MP1 brainstorm input, 2026-11-11. Agent output, not reviewed; claims are leads until verified.

VERDICT=PASS-WITH-FINDINGS FINDINGS=6 HIGH=0 RUNS=27 NOT_CHECKED=3

**Summary.** S1 is a pure move. I found no behaviour change and no lost test. I found no High finding. Nothing blocks the merge. The worktree is clean at 5ce21d2. I changed no repository file.

## Findings

1. **Medium, for the A1 step 3 plan; not an S1 defect.** `crates/turts_audit/src/replay.rs:73-86`: `read_jsonl` calls `validate_records` at `:84`. So the JSONL decoder is also a call to the replay validator. When step 3 deletes replay.rs, it also deletes the decoder. These call sites use it: `el1.rs:61`, `economy.rs` (for example `:302`), `session.rs:1053` and the server writer tests. Repair: in step 2, move `read_jsonl` and `read_jsonl_file` to format.rs as decode-only functions. Make each call site run the validator explicitly.
2. **Low.** `crates/turts_server/src/economylog.rs:247-334`: `validator_accepts_an_orders_operation_before_movement` stays with the writer, but it is a validator test. At `:332` it pins the replay-only text "movement transition changed nonmovement state". Step 3 must re-pin this test or delete it with a reason. All 7 writer tests compile in 3 binaries: `main.rs:7`, the `#[path]` include at `economy.rs:9`, and the one at `mc_orders.rs:3`. Both includes exist only for `EconomyLogWriter`.
3. **Low.** `crates/turts_audit/Cargo.toml:12`: the `bevy` facade dependency is not in the list of design section 3. It is not a forbidden edge. On host and on wasm32 it adds 6 crates that the turts_sim tree does not have: bevy, bevy_internal, bevy_diagnostic, bevy_input, bevy_shape, bevy_transform. It adds no render, winit or pbr crate. I accept it until step 3 deletes replay.rs. Repair: record it in the design as a temporary refinement, then delete it in step 3.
4. **Low.** The crate-layout table in `CLAUDE.md` has no `turts_audit` row. Repair: add one row with the responsibility and the forbidden list of `boundary.sh:50`.
5. **Low (information).** The 7 rows "X no turts_audit" at `boundary.sh:54-60` cannot be the first red:
    - turts_sim -> turts_audit is a cargo "cyclic package dependency" error (probe).
    - turts_client -> turts_audit fails first with "forbidden edge: turts_client -> turts_sim" (probe).
    - The rows stay useful only if turts_audit stops depending on turts_sim.
    - The red proofs test the rule mechanism, not each row. `t1_boundary_script_passes` proves that each row is present.
6. **Low (instrument).** Two gaps in the builder's `partition.py`:
    - `norm()` reads `,)` as `)`, so it cannot see a change between a 1-tuple and a parenthesised expression.
    - It prints the leftover text of the server `economylog.rs`, but it does not assert it.

    Repair: assert the leftover string, and list the one trailing comma explicitly. My own token check closes both gaps.

## What I ran (each exit code is the raw exit code)
- `partition.py a29ce67 5ce21d2`: exit 0, PASS, 19,082 of 19,082 old lines claimed. Builder plants `PLANT=replay` and `PLANT=drop`: exit 1 each.
- My plant in a scratch clone (replay.rs:4214, `!=` changed to `==`, committed in the clone only):
    - `partition.py`: exit 1, "replay.rs body differs at char 105138".
    - My `tokcheck.py`: exit 1, one new hunk "A replace `!=` -> `==`".
- `item_equal.py`, whole files: exit 1, DIFFERENT. After `unwrap_tests.py`: exit 1. The only differences are the reflow of `read_jsonl_file` (old :360-367, new replay.rs:88-93), the `use super` list without `EconomyLogWriter` (old :8086-8091, new replay.rs:7813-7818), and `fn initial()` (economylog.rs:242-245). The original `fn initial()` stays at replay.rs:7823.
- My own check `tokcheck.py`: exit 1. It strips pub and use items, then diffs the old tokens against format.rs + replay.rs + lib.rs, then diffs the deleted runs against the server `economylog.rs`. The whole residual is:
    - the `from_tick` impl, moved into format.rs with the same tokens;
    - the one trailing comma of `read_jsonl_file`;
    - the new test head `#[cfg(test)] mod tests {`, the 4-line copy of `fn initial()` and its closing `}`;
    - the `use` lists;
    - `mod format; mod replay;`.

    So the 3 differences that the builder reported are all real, and there is no other difference.
- Test lists. Base from a scratch clone at a29ce67, head from the lane:
    - Totals: 1,796 and 1,590. 1,796 - 296 + 67 + 21 + 2 = 1,590.
    - Base: 74 validator tests in each of 4 binaries, 296 entries.
    - Head: 67 tests in turts_audit, and the 7 writer tests in each of 3 binaries (21 entries). el1 no longer includes the file.
    - No leaf name vanished. The only new names are the 2 boundary tests. `list_cmp.py`: exit 0.
- Gates:
    - fmt: exit 0. clippy with `-D warnings`: exit 0. wasm32 check: exit 0.
    - `cargo test --workspace`: exit 0, 1,558 passed, 0 failed, 32 ignored.
    - `boundary.sh`: exit 0, 24 `checked:` lines.
    - Frozen tests: exit 0 (26 + 1 + 1 passed).
    - sha256 of the frozen fixture: bcbf72c2206251f635a7c7f5b6c8f573d69a5fc05a5bd77716e98ebfb08e370b.
- `cargo tree -p turts_audit -e normal`: host exit 0, wasm32 exit 0. No render, winit, pbr, wgpu, client or server crate.
- Boundary faults: `t1_boundary_script_passes` fails by name at boundary.rs:116 in both cases.
    - B1, `boundary.sh:57` deleted: exit 101.
    - B2, turts_mod_runtime removed from the list at `:50`: exit 101.
    - Probe in the clone: turts_audit -> turts_mod_runtime gives "forbidden edge: turts_audit -> turts_mod_runtime", exit 1.
- Validator fault R1. I inserted `expected.economy.balances = after.economy.balances.clone();` before replay.rs:6378, so the Cancel check accepts any balance.
    - `replay::tests::validator_rejects_fabricated_produce_cancel_and_set_enabled_operations` failed with exit 101: "called `Result::unwrap_err()` on an `Ok` value" at :18093.
    - Restore: `sha256sum -c` exit 0, status empty. The green re-run exited 0. The full boundary test binary exited 0.
- M3 fault set: exit 0, 17 of 17 RED-RESTORED, head 5ce21d2, status clean. Design section 4 says these faults cannot reach the validator, so this run is a regression check only.

## Not checked
- `scripts/acceptance.sh` and the M2 ci-native set. The E8 and E9 process tests reach the moved validator. These tests changed only by path renames (economy.rs). NOT TRACED.
- The browser fault set. The brief said no browser run.
- `completeness.py --enforce`. It is red on the base for a known reason (the MP1 draft lacks two sections), so I did not run it.

## Crate shape for A1
The shape is correct for step 2. lib.rs exposes only 4 items (lib.rs:1-5). The domain files can go next to replay.rs. Before step 3, the next builder must:
- move the decoder (Finding 1);
- deal with the writer test that pins replay text (Finding 2);
- remove `bevy` and the dev-dependencies `bevy_app` and `bevy_math` together with replay.rs;
- delete `#[allow(dead_code)]` on the items that are now pub (format.rs:40, replay.rs:88);
- decide if `EconomyLogWriter` stays behind the `#[path]` includes at economy.rs:9 and mc_orders.rs:3. Design section 3 keeps the writer in turts_server.

## Before the merge
Nothing blocks the merge. I recommend these three steps:
1. Add the CLAUDE.md row (Finding 4).
2. Record the bevy dependency in the design (Finding 3).
3. Run the M2 ci-native set. The red of `E8-mixed-cost-partial-commit` comes from the moved validator (design 9.2), and no run here reached that red.

## What I changed
None in the repository. Memory: `.claude/agent-memory/technical-director/project_pre_s1_audit.md` and one line in its `MEMORY.md`.

Files are in /tmp/claude-1000/-home-julien-Projects-TuRTS/bf418660-b272-4bb3-8628-8d9ef2677d1d/scratchpad:
- tokcheck.py
- tok.out
- tok-plant.out
- mylist.py
- partition-myplant.out
- faults.txt

The run logs are in /home/julien/Projects/TuRTS/target/mc-evidence/, in the folders pre-s1-review-*. The M3 evidence is in /home/julien/Projects/TuRTS/target/mc-evidence/root-regate/runs/pre-s1-review-m3.
