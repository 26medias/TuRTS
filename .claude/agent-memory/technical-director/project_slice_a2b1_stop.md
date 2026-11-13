---
name: slice-a2b1-stop-ruling
description: 2026-11-12 A2b1 stop ruling facts - probe values unpinned, kernel_reference live-fed chain, coverage-count trap, plant (c) instrument
metadata:
  type: project
---

Verified at `pre-mp1/a2b1-fallback` 3e85f88b (base afb9c6de). Ruling file:
`TuRTS-mp1/docs/research/mp1/ruling-slice-a2b1-stop.md` (RULING=DONE, no CEO).

- `frame_chains` / `muster_world` (probe_neutral.sh) are NOT pinned anywhere (frame_hash.rs:13-17); only
  research reports carry d0d1b1c5/aa9db740. Not frozen values; plan :414 "Nothing else needs the CEO".
- frame_chains hashes Debug of `progress` and `plain_progress` (frame_hash.rs:174-175): any output-type change
  moves it. Use muster_world (positions only) as the cross-task behaviour witness.
- kernel_reference feeds the LIVE output into both kernels each tick (`next_frame(&frame,&live,..)` :942), so a
  per-tick compare after a fallback is valid; D 9.18 "difference spreads" reason does not hold for it.
- Trap: `seen.count` / `seen.fallback` (:939-941) count every tick, not compared ticks; "kind > 0" checks pass on
  uncompared kinds. Header :44 "no random seed free" vs code :1006 counts pooled chains too (cannot go red).
- `step_frame` drops fallback requests (frame.rs:68-70); a chain that uses it carries pool id 0 forever.
- Plant (c) id source: red only in `fallback_pool_ids_come_from_the_order_counter_in_unit_id_order`.
- CEO touch point 1 answer: no vault record found (NOT TRACED) on 2026-11-12.

**Why:** later A2b2/A4/A3 reviews reuse these instruments. **How to apply:** demand full-tick compare from A2b2 on,
compare probe values to the task base, never to remembered constants. Related: [[mp1-l0-sim-order]].
