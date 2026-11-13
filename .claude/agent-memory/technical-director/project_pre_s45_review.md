---
name: pre-s45-review-facts
description: S45 client split review facts (d047433): TEST_MODULE_FILES ungated-mod hole, T7 audio flag override, vis strip result, surface probe method, stale cites
metadata:
  type: project
---

Verified 2026-11-12 on d047433 (base a29ce67):
- Own pure-move instrument (lexer, member-level item multiset, impl split, vis-token strip) leaves only: 5 `../` in include_str (client_shell tests), 7 trailing-comma reflows, new `//!` module docs. Item counts equal: 153/54/114/78/62. Swap of two statements shows at item level only (char multiset equal).
- Interaction group = old lib.rs + four placeholder files (selection, groups, compose, smart), not lib.rs alone.
- 99 `pub(super)` lines added (not 188). All 99 needed: strip-and-compile took 4 stages (E0624 methods appear at stage 1; E0603 and E0425 cascade stage 2; `private type` errors have `Type<'_>` in backticks, so a name regex misses them; private_interfaces warning is only a warning, but BindingConflictPrompt still gives an error at client_shell.rs:135).
- Public surface: probe `use crate::M::N as _` for 356 client and 135 interaction names; run each crate alone (a failed interaction check cancels the client check). Reachable 18 client, 11 interaction, failing sets equal at base and head.
- TEST_MODULE_FILES (crates/turts_checks/tests/support/mod.rs:522) skips by path only. Plant: remove `#[cfg(test)]` from `mod tests;` in visuals.rs and add a content id to visuals/tests.rs: `no_content` stays green (exit 0). Plant of a content id in non-listed visuals/health.rs: red in `real_source_scan_is_clean` only.
- scripts/t7.sh:105 replaces MC_CHROME_ARGS_JSON with M2_CHROME_ARGS_JSON; a T7 run that passes the M2 list loses `--alsa-output-device=null`. PT1 and rows browser-profile.json show the flag; T7 has no profile file.
- Stale cites after the split: tests/browser/{m2,m3,t7}-driver.js, t7-oracle.js, pt1-oracle.js:61-72, harness/run.js:69 (client_shell.rs symbols), m2-driver.js:28 (widgets/mod.rs), crates/turts_client/tests/forge_research_tooltip.rs:103, widgets/sources.rs:792.
- Tests: `cargo test --workspace -- --list` 1796 tests both revs, name sets equal. Runner log: target/mc-evidence/<label>/command.log.
- Hot spots after S45: draw_world_visuals 527 lines (visuals.rs:279), drive_core_adapter 421 (core_adapter.rs:730, effect match; not in plan 3.4 list), step 400 (interaction lib.rs:143), spawn_widget 340, activate_widget 138 (activation.rs), sources.rs 2455 lines (1233 production).
