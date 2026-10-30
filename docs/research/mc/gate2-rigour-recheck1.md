VERDICT=pass HIGH=0

# Gate 2 rigour re-check 1: plan 9910798, design 89e1fa3 (revision 8)

## Original findings
- H1 fixed: plan:22 (`#[path]` rule), plan:557 (`crates/turts_server/tests/mc_orders.rs`), plan:615 (ORD6 headless = `session::tests::bucket`).
- H2 fixed: plan:425 (zero `OrderDef`), plan:612,614 (task 15 `no_orders_notice_shows_once`).
- H3 fixed: plan:53. I checked every task: each commit step comes before its plants.
- M1 fixed: plan:642 (KM1, KM2, SEL4 red in step 1), plan:529,532 (PointerLeft), plan:601 (MM1 design red), plan:591 (HD1 plant).
- M2 fixed: plan:151, plan:582 (10 needs 5; 15 needs 4).
- M3 fixed: plan:481, plan:485 (task 9 adds the P1 rows).
- M4 fixed: plan:483 (0.9 rad), plan:487 (all viewports).
- M5 fixed: plan:574 (grep check). See N6.
- M6 fixed: plan:674, plan:709.
- M7 fixed: plan:746-747.
- M8 fixed: plan:664.

## New defects from the repair
- N1 Medium. plan:22,462,557,612: three test crates include `orders.rs` with `#[path]`. A crate that does not call both `to_sim` and `to_wire` fails `clippy --all-targets -D warnings`. My scratch probe gave `error: function `to_wire` is never used`. The existing pattern, `economy_export.rs`, calls both of its functions. Fix: write `#[allow(dead_code)]` on the `mod` line.
- N2 Medium. plan:70: the G-REF selector reads only `plant_path`. Multi-file cases keep their paths in `plants[].plant_path`. A dry run on `transaction.rs`, `transaction_m3.rs` and `machine.rs` did not list E4-local-capacity, E6-capacity-bypass, C1 or C5. Fix: also read `plants`.
- N3 Low. plan:5,13 cite design revision 7 at `d21c9ee`. Revision 8 moves the lines by up to 16 (Player path: 1651 becomes 1667). G1 (plan:762) has an answer in revision 8 but is still open.
- N4 Low. plan:740 gives "22 runs" for tasks 13 to 16. The budget gives 5 + 3 + 5 + 6 = 19.
- N5 Low. plan:355 lists `ContentCatalog.orders` under `turts_mod_runtime`. The code has `ContentCatalog` in `crates/turts_mod_api/src/content.rs:156`, which agrees with plan:328.
- N6 Low. plan:574: the VIS2 red expects `0` and exit 1. If `mc.md` does not exist yet, grep exits 2. Task 13 is the first writer of that file.
- N7 Low. plan:642-648: task 16 runs its green rows in steps 4 to 6, after its commit and gate.

## Runs
- `own2.py`: 42 of 42 rows have one owner each; 81 of 81 coverage rows map to tests; no mismatch.
- Waves check: the waves agree with the Needs column. Critical path 55; subtotal 106.
- Citation sample: 25 new `path:line` citations. All hold.
- Clippy probe: red, as stated in N1.
- G-REF dry run: 4 cases missed, as stated in N2.
- `diff` of design revisions 7 and 8: the task 18 table, task 4 and task 6 agree with revision 8.
