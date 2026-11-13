---
name: mp1-sound-off-rereview
description: Sound-off review pass 2 (314e2722) and pass 3 (ee9bd0dd), 2026-11-12: H25 names; press_release repair; menu key rule gap (shadowed key); probe not silent by itself
metadata:
  type: project
---

Pass 2 at 314e2722 (PASS): H25 names pack at ^0.2.4 and are refused at ^0.2.3; the strip helper `as_edition_0_2_3` gives
the 54c39fdd bytes exactly; trace bodies and the stream did not change.

Pass 3 at ee9bd0dd (FAIL HIGH=1). Verified by runs:
- `menu_keeps_a_key` counts keys only. With `menu` on `KeyM` (also `key_move`; `action_for_key` takes the first match in
  BTreeMap name order) or on `ShiftLeft`, `cancel` unbound and the mod switch hidden by `visible: [GuideOpen, Not(GuideOpen)]`,
  the package packs and the core gives no `menu` on any key. The pass-2 one-frame map-press case is now a red test.
- press_release.rs (now after pointer_events): `insert(Pressed)` panics if an Activate observer despawns the button between
  two clicks in one frame; a disabled button keeps a stale Pressed. Faults "Cancel arm removed" and "one pointer key" stay green.
- `tests/browser/click-without-hover-probe.js` launches Chrome with no SILENT_ARGS and no silentEnv; it is silent only under
  mc_headed_v6.sh. The scans read only `*.test.js` and `scripts/`.
- A ^0.2.3 package can bind `toggle_sound` or `leave_match` in `engine_overrides` and it packs (no version row).
- Bevy 0.20 picking event name: `PointerOver`. The default error handler panics on a failed command.
- The review cache folder can vanish (disk 95 % full): recreate it as an empty folder before you use the runner.

**Why:** consumer lens on the first engine control that a mod places (D67) and on the CEO silent-default rule.
**How to apply:** at the next pass, re-run the a1 and a5 packages and the core probe, and check the probe for SILENT_ARGS.
Related: [[mp1-l0-api2-review]].
