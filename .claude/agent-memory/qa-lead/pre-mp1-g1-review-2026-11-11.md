---
name: pre-mp1-g1-review-2026-11-11
description: G1 grid within repair review (1755e69) - property test misses high-side margin and negative reach; nothing pins scanned cells; edge counter reproduced 14/0
metadata:
  type: project
---

Verified 2026-11-11 on TuRTS-pre-g1-review, head 1755e69 (code 46a740c), runner labels pre-g1-review-*.

- Seen red: `within_matches_brute_force_scan` on the reverted range ("missed 14", exit 101), on
  EDGE_MARGIN 0.0 ("missed 2", only the 2 gate points; the 24,000 random queries give 0), on
  margin low side only removed (2).
- Stays green (gaps): margin removed on the HIGH side only (exit 0). Real high-side case:
  cell 0.7, reach 3*0.7, q (-3.5,0), p (-1.4,0) - probe red under that plant, green at head.
  `reach.abs()` -> `reach` green (no negative reach query). Range one cell wider everywhere:
  all 226 turts_sim lib tests green; nothing pins visited cells on the dense path.
- Scanned cells at reach/cell 1 on a lattice of queries: mean 9.23, worst 12 (head); 25.4/30 wide.
  "9 cells or fewer" (gate2-slice-plan-simulation.md:59) is a mean, not a bound.
- Edge counter patch reproduced: gate control 1 START 14 HIT added 14; frozen pack tests 3 START 0 HIT.
  Base lacks D0p 43282fe.
- f32 probe: G1 formula 0 low-side misses for |x| up to 4096, cells 0.1-5, reach 0.1-20 (sampled).
