---
name: l0-split-review-facts
description: L0-SPLIT review facts (e0569d1): own inline instrument, oracle gaps, commuting concern pairs, layout for later visuals tasks
metadata:
  type: project
---

Verified 2026-11-12 on e0569d1 (base d7d7e6d, oracle commit 149f1ed):
- Own token-inline instrument (scratchpad inline_check.py, strips `&`/`&mut`/`*` of args): move A old 3120 vs 3112 tokens, only reference glue plus `let hovered =` and tail `hovered`; move B 2954 = 2954 EQUAL (6 calls). Red on swapped calls, one constant (0.021 to 0.022), swapped lets.
- Scene oracle green at 149f1ed and e0569d1, fixture byte-equal (git diff empty). `TURTS_BLESS_SCENES` returns green without compare; set nowhere in .github or scripts.
- Oracle misses: delete `visuals.active = false;` stays green (initial state is false, no full-to-exit scene). Also unreached: hover on own unit, hover leave, blocked 2 s expiry, marker expiry, Selected health mode, missing-content continues, second camera.
- Concern pairs: draw_selection_rings/draw_hover_ring and intake_markers/track_blocked commute (green when swapped). Red when swapped: order_markers/blocked (markers_shown order), health collect/fills, sync_session/hover.
- Effect::Marker and Effect::Sound: no producer in turts_interaction (only enum at lib.rs:887-891).
- Test list: 1608 at d7d7e6d, 1614 at e0569d1, diff exactly the 6 added tests (parse binary header, `rtk proxy diff`).
- merge-tree with origin/mp1/integration 8ba981d: clean (exit 0, tree b0391b3); integration touches no client or refutations files.
- Runner trap: a `( ... ) &` subshell is killed when the launching Bash call ends; use run_in_background on the runner itself.
