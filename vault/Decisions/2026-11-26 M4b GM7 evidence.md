---
type: decision
date: 2026-11-26
tags: [turts, m4b, gm7]
---

# M4b GM7 evidence: b3-2 plus the pinned run on the final code (operator, 2026-11-26)

**Status:** Decided by the operator, 2026-11-26 ("whate you recommend", option a).

GM7 for M4b is accepted on two runs:

1. b3-2, the operator's B3 sudo run, 1000 intervals per row, all four rows PASS
   (`target/mc-evidence/t7-flocking/gm7-b3-candidate/runs/b3-2/`). Code a6d54cee plus the test-only workload
   layout of 854e01e8.
2. The pinned run without sudo on the final game code c1dc1e60, 200 intervals per row, all four rows PASS, figures
   equal to b3-2 (`target/mc-evidence/t7-flocking/gm7-diag-c1dc1e60/runs/A1/`).

No third sudo run. The change between the two is the 3-line E1 yield rule and the V6 place rule.

## Known limits accepted (operator, 2026-11-26: "ok, those are fine")

1. A move or a queued route can end after 2 s at a standing enemy or a standing crowd, up to 19-24 m before its goal, with no reason shown (T-59).
2. A pushed unit does not always go back exactly to its place: usually 0.5-0.9 m off, up to 3.7 m when a large own unit takes the spot (T-59).
3. Dig with Walkers and Crawlers together orders only the Walkers; the tooltip says so (merged Dig, first after M4b).
4. A Dig or Fill drag that ends behind a wall can take the wall cell; the preview shows it.
5. A refused building spot shows only a red circle, no reason text (F7).
6. A right-click on ground under or behind own units gives no order.
7. At far zoom in a packed crowd, a click can pick the unit in front.
8. A 1,000-unit join fills the send window briefly at the join, ticks 8-16 (T-58).
