---
name: client-control-gap-facts
description: Verified 2026-10-28 code and spec facts on the missing RTS control loop (selection, group orders, camera, audio, mod UI schema) for the full spec review phase 3
metadata:
  type: project
---

Verified 2026-10-28 (full spec review, phase 2 gap audit; my file is
scratchpad review/gap-technical-director.md, scratchpad is per session):
- Selection(pub Option<u32>) at crates/turts_client/src/economy_ui.rs:31; test at :1405 "selection is local, never a server command".
- ground_point uses plane y=0, crates/turts_client/src/input.rs:14-21.
- PlayerCommandKind closed enum, 6 variants, crates/turts_net_protocol/src/messages.rs:54-78; MoveTo carries Vec<UnitId>.
- validate_move_to rejects the whole group if one unit is unknown, foreign or static (crates/turts_sim/src/command.rs:274-282); MAX_UNITS_PER_COMMAND = 256 (:19).
- Sim has one MoveTarget(pub Vec2) per unit (crates/turts_sim/src/world.rs:33), no order queue.
- turts_mod_api has 0 presentation fields (camera, hotkey, icon, sound, cursor). bevy_audio is not in the turts_client tree (native and wasm32).
- M2 design line 521-522: "command verbs/status strings are engine UI" (drifts from spec 4.3 Interface row S:164).
- No "M2D/M3D" decision IDs exist; the vault writes "M2D:98" as a line reference into the M2 design.
- docs/agents/check.sh:18 has a fixed NAMES list; unknown names fail (:155-159). A new seat edits it.
- Spec 4.1 Presentation row (S:122) does not name camera; 4.3 has 11 rows, none for input, camera, audio, selection.

**Why:** Phase 3 asks me to review the consolidated gap register; these are the anchors.
**How to apply:** Re-check each line before citing; the M4 or control milestone will move them.
