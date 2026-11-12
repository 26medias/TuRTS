# G1 edge counter: run log

Task G1 step 4 (plan `docs/superpowers/plans/2026-11-12-pre-mp1-slice.md`, task G1), design section 9 item 4.

## Base

- Commit under test: `46a740c` (branch `pre-mp1/g1-grid`), the G1 repair of `crates/turts_sim/src/grid.rs`.
- The depletion repair (`43282fe`, task D0p) is NOT in this base: `git merge-base --is-ancestor 43282fe HEAD` gives exit 1.
  So these runs are the runs before the depletion repair. Root repeats the stream run after D0p.
- Runner: `exec_in_lane_v4.py --target mc-pre-g1-grid --head 46a740c --audio --allow-dirty`, with
  `TURTS_G1_EDGE_LOG=<log file>` in the environment. The receipts are in
  `/home/julien/Projects/TuRTS/target/mc-evidence/pre-g1-counter-<n>/receipt.json`.

## Instrument

- `g1-edge-counter.patch` (apply on `46a740c`). It is not committed to the code.
- For each `within` call, it takes the cell range of the old code (`ceil(reach / cell)` rings around the
  cell of the query). `added` counts the points in the new result that are outside the old range.
  `removed` counts the points that the old range had, that the filter accepts, and that are outside the new range.
- It appends one `START` line for each process (the first call) and one `HIT` line for each query with
  `added > 0` or `removed > 0`. The thread name is the test name in a libtest thread.
- `START` lines prove that the counter was live in that process. `summarize.py <log>` counts the lines.

## Runs

| n | command | exit | processes (`START`) | hit queries | added | removed |
| --- | --- | --- | --- | --- | --- | --- |
| 00 | `cargo test -p turts_sim --locked --lib grid::tests::within_matches_brute_force_scan` | 0 | 1 | 14 | 14 | 0 |
| 01 | `cargo test -p turts_pack --locked --test m3_m2_stream --test forge_fixed_point --test forge_cases` | 0 | 3 | 0 | 0 | 0 |
| 02 | `cargo test -p turts_sim --locked` | 0 | 6 | 14 | 14 | 0 |
| 03 | `cargo test -p turts_pack --locked` | 0 | 11 | 0 | 0 | 0 |
| 04 | `cargo test -p turts_server --locked` | 0 | 8 | 0 | 0 | 0 |
| 05 | `bash scripts/acceptance.sh` | 0 | 30 (26 `turts_server`) | 0 | 0 | 0 |

- Run 00 is the positive control. It shows the gate case: `q=(1.8,0.0) reach=9.0 cell=0.9` and
  `q=(1.8,-3.6) reach=9.0 cell=0.9`, `added=1` each (`edge-00-gate-case.log:2-3`). The 14 added points are
  the 14 misses of the red run on the old code.
- All 14 hits of run 02 are in the thread `grid::tests::within_matches_brute_force_scan` (the property test).
  0 hits come from any other test.

## Result

0 hits on the stream, the golden tests and the process tests. So no golden value can change from G1, and
G1 lands alone (design section 2 row G1; plan task G1 step 5). This is the result before D0p.

## Cost (task G1, recorded, not judged)

`within-cost-bench.rs`, release, best of 5, 5,000 units over 200 x 200, 200,000 queries, cell 1.2
(2 x max radius 0.6). Machine: local, Intel(R) Core(TM) i9-14900K, 28 logical CPUs, 7.0.0-42-generic. Receipt: `/home/julien/Projects/TuRTS/target/mc-evidence/pre-g1-bench/`.
Other `cargo`, `rustc` and `trunk` processes ran at the start of the run (receipt field `other_compilers_at_start`), so the times are noisy.

| reach / cell | cells per query before | cells per query after | ns per query before | ns per query after |
| --- | --- | --- | --- | --- |
| 1 | 9.000 | 9.001 | 426.4 | 256.2 |
| 2 | 25.000 | 25.004 | 894.2 | 896.3 |

The results per query are equal (0.5617 and 2.2406 points). The `ceil + 1` form was not needed.
