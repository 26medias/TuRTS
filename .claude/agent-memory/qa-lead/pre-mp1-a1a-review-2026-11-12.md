---
name: pre-mp1-a1a-review-2026-11-12
description: A1a review at 89bf39b - reader keeps partial batches with no open boundary; :469 misses wrong destination after validator off; sink order not pinned; cheap economy reruns
metadata:
  type: project
---

Verified 2026-11-12 in worktree TuRTS-pre-a1a-review at 89bf39b (parent 70121e3), cache mc-pre-a1a-review.
Evidence dirs: target/mc-evidence/pre-a1a-review-* ; plants: scratchpad qa-a1a/plant.py.

- `complete_prefix_end` (economy.rs:1894) cuts only when a CommandBoundary has no Operation of its
  `operation_id`. It keeps: a full batch with no Tick, an ops-only tail (every tick has Other ops),
  and an order span with the first op only (end op missing). Rule "cut at first Boundary/Operation
  after the last Tick, keep Claim/Admission" passes the reader test and all 7 scratch cases.
- Live lines are 6-12 KB each; sink BufWriter flushes mid-batch, so partial batches are reachable.
- Reader mutants: cut-1 red :157 (3 of 4); cut+1 red :157 (validator on: Err text; off: 5 of 4);
  any->all red (expect panic :1928); drop tail Claims red :262. 10/10 green; the open cut is built
  from bytes (forced). :151-155 is a fixture self-check, it never calls the real reader.
- Sink order swap (economylog.rs:169-189) stays green: the fixture copies the order (economy.rs:84-92).
- Validator off (unconditional Ok): six mc_orders replay tests + rejoin green, no plant.
  :469 spread point x0.5 green off, red on at :566 -> a red lost at A1c. Stack-all plant red :583.
  :634 focus fires from any range: green on AND off (pursuit never checked).
  rejoin: wrong player id red :1065; Claim tick+5 green off.
- Incremental economy/mc_orders reruns take 3-30 s on a warm cache.

See [[pre-mp1-slice-gate-2026-11-11]].

Re-check at 86c0cda (labels pre-a1a-recheck-*), validator off:
- half-point red :594; stack red :600; spread point x1.03 (shift 0.25 / 0.62) GREEN: reach 1.2 = two
  rings of spacing 0.6, so a shift under about 0.6 is not seen.
- reader first-Operation-only red (cases A,B,C,E,G and :319). Sink swap red at the pin :129.
  Tick-before-commands (T,B,O) passes the pin; red only on fixture counts :171 and :291.
- Claim tick 0 red at session.rs:1078 (0 vs 3). Gates fmt 0, turts_server tests 0, sha256 equal.
