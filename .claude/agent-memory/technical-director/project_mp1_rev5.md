---
name: mp1-rev5-facts
description: Verified 2026-11-12 code facts and choices from the MP1 design revision 5 repair (TuRTS-mp1, mp1-design faf70c5 + uncommitted edit): AllSeated choice, spawn order, alert priority field, key counts, cross-check script
metadata:
  type: project
---

Verified 2026-11-12 in /home/julien/Projects/TuRTS-mp1 (branch mp1-design, head faf70c5):
- `AlertDef` already has `sound: Option<String>` and `priority: i16` (turts_mod_api/src/interface.rs:375-382);
  higher priority wins in the alert voice cap (turts_client/src/audio.rs:436-462). No new priority field needed.
- A spawned gatherer gets a GatherOrder with node None, stage `Idle` (turts_sim/src/economy/transaction.rs:710-719);
  gather.rs:237-239 skips it; `GathererWaiting` reads only `GatherStage::Waiting` (turts_client/src/feedback.rs:821).
- Wire reason keys: 37 in `as_str` (turts_net_protocol/src/messages.rs:225-264); `Activity` 7 values (:104-112);
  `GatherStage` + wait reasons at turts_net_protocol/src/economy.rs:90-108.
- Engine input actions list: `is_engine_action` content_mc_validation.rs:1495-1519 (no `leave_match` today).
- ModError display: `pkg/file: field: msg (allowed: a, b)` (turts_mod_api/src/error.rs:46-62); parse hint
  "; did you mean `x`?" before allowed (turts_mod_runtime/src/parse.rs:29, :49-53).
- Stored-identity rejoin ends after 15 s without Welcome (turts_client/src/rejoin.rs:5, :317-342).
- `#[ignore` per file in turts_server/tests: acceptance 9, mc_group 8, economy 4, hello 3, t7_oracle 2, five files 1.
- Choices in rev 5: stage 1 uses `AllSeated(countdown_secs: 0)` (not the `--start-when-seated` fallback; needs
  only `LobbyState` writer, no LobbyAction/LobbyRefused); one `StartOrder` type for placement `start_order` and type
  `spawn_order` (NearestNode); `GathererIdle(f32)` new trigger; `construction_queue_full` key (49 REASON_KEYS, 22
  STATE_KEYS); PAY2 moved to stage 1; Ended exits when every claimed slot sent LeaveMatch or after_match_secs.
- Cross-check script (can go red; proven with 3 plants): scratchpad td-rev5/xcheck.py (reason keys, row ids, X
  stages in 4/10/11, table cell counts).

**Why:** the rev-5 re-gate and lane 0 cite these.
**How to apply:** re-check lines after the pre-MP1 slice moves files. See [[mp1-reconcile-facts]].
