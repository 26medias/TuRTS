---
name: mp1-rev6-facts
description: MP1 design revision 6 patch (2026-11-12): ground_rally field on StartOrder, GOAL1 driver/oracle split, open points 19-21, xcheck trap on "row X12"
metadata:
  type: project
---

Verified 2026-11-12 in /home/julien/Projects/TuRTS-mp1 (design file, uncommitted rev 6 over 12bca5b):
- Spawn-order ruling: `StartOrder` gets `ground_rally: Ignore or ApplyFirst` (default `Ignore`; placement `start_order` takes `Ignore` only). Node rally applies in place of the spawn order, no queued spawn order. Whether today's queue accepts a queued `Gather` is NOT TRACED (ApplyFirst case, lane L-RLY).
- GOAL1: driver reads no text; the oracle reads `turts_debug.hints`. Section 15 point 17 closed; new points 19 (pacing, game-designer), 20 (box drag, ux-designer), 21 (end-of-connection schedule, backend-engineer). Point 14 answered by reading.
- xcheck.py reports a cited "row X12" as a non-section-12 row: write "X12 item" for scope items, never "row X<n>".
- grep of crates/turts_server/src finds no handler for the end of a connection (session.rs:710 is about `kick_pending`).

**Why:** the rev-6 re-gate reads these.
**How to apply:** re-check lines before the next patch. See [[mp1-rev5-facts]].
