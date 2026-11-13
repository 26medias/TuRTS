---
name: l0-core-review-facts
description: L0-CORE review (ec88338, 2026-11-12): FocusAlert unit variant High, post-loop hook too late for placement, shared-type edit points, step_equal.py blind spots
metadata:
  type: project
---

Verified 2026-11-12 on ec88338 (base 9044820), worktree TuRTS-mp1-l0-core-review. Verdict FAIL HIGH=1.
Report: /home/julien/Projects/TuRTS-mp1/docs/research/mp1/review-l0-core.md (uncommitted at write time).

- `WidgetAction::FocusAlert` is a unit variant (primitives.rs:41-42); alert rows today carry `CenterCamera{x,y}` (sources.rs:1031-1036). A unit variant cannot identify the row: High, needs decision (I proposed `FocusAlert{x,y,unit}`).
- Hooks `placement/hints/alerts::update` run after the event loop (lib.rs:237-241). A probe showed 27 hook calls that already held `Send(...)` in effects on PointerUp, so GHO1 (d) cannot be met without a pre-dispatch intercept.
- Shared edit points that remain: the Core struct and new, InputEvent plus the one exhaustive match (lib.rs:164), the Effect enum plus the drive_core_adapter match, WorldView plus FrameWorld (core_adapter.rs:279), SourceContext (10 literals, one in sync_widget_tree tree.rs:201), VisualInputs (visuals.rs:89).
- step_equal.py drops call arguments and maps every `return` to `continue`. Its blind mutations: inner continue->return, an argument changed to a literal, `&mut Vec::new()` in place of an accumulator, hook calls moved. The tests catch the wheel accumulator (rebound_actions.rs:708) and the inner return (groups.rs:188).
- Harness trap: when you exec step_equal.py source, `.replace("sys.exit(main(sys.argv))","")` also changes the string literal on its line 30. Cut at `rindex` instead.

Re-review 2026-11-12 of the repair 0816a87: H1, M1, M2 (concurrent pairs), M3, L1, L2 and L4 are closed. The proof is red on all 13 of my mutations. Report: rereview-l0-core.md (same folder).
- What remains (ordered, not High): `facts::*::build(replica, content)` runs every frame (core_adapter.rs:567-568) and is stateless, so the per-delta rule (plan:977, :990) needs a shared edit. `leave::activate` cannot send LeaveMatch (a wire message). The text "takes every input" (placement.rs:24) would swallow KeyUp/FocusLost/Placement.
- The run_proof grep `taking_input\s*=[^=]` does not see `taking_input: true` in a Default impl. The tests do see it (92 red).
- l0-split/run_proof.sh at HEAD: move B prints NOT-CALLED for the new WorldView methods, but its tokens are equal. With the 4 repair lines removed, move A is EQUAL.

**Why:** L0-EXIT freezes these shapes; a later change request costs 2 runs each.
**How to apply:** at L0-EXIT, or at a repair of L0-CORE, check that these items are fixed or ruled. See [[l0-split-review-facts]].
