---
type: report
date: 2026-10-30
tags: [turts, mc, stop, design-conflict]
ai-first: true
source: codex
---

# T8 contract collision stop

STOP for a CEO design ruling. No further build or agent run has started. Build20/170.

Task8 candidate91d6db7 on codex/mc-t8 is clean and unmerged. The new interaction contract exactly implements `Effect::Shell { action }`, as designrev10 section2.2 line268 requires. Full workspace tests compiled and failed `real_source_scan_is_clean`, exit101. The hit is `Shell`, resource-name from forge/resources/shell.ron fieldname, against interaction/src/lib.rs:140. Design11.3 line1040 requires that forge/muster content names do not appear in engine code. These requirements conflict on the required spelling.

Primary evidence: target/mc-evidence/t8-builder/g-rust-test.log:530-558 and root-stop-verification.json; candidate source line140; mods/forge/resources/shell.ron. Frozen M2 stream hash is unchanged. Task8 T1X red101/green0 and fmt/clippy/wasm passed; G-QA and final plant were not run after the stop.

Recommendation for CEO approval, not applied: rename the UI effect variant to `ShellAction { action }`, keeping fields and behavior, forge content, and the scanner unchanged. A one-line proposal is saved as target/mc-evidence/t8-builder/proposed-shell-action.patch. If a later private Rust module named shell also collides, the planned shell.rs file can use a non-colliding module alias. The proposed variant rename still requires a ruling because it changes the design's explicit contract spelling.

Task18 initially reported an icon/assets conflict. Root checked the primary text and corrected that classification: design12.2 lines1099-1101 limits images in the plain HUD, not all package assets. Required order icons can be supplied through minimal asset exports while the HUD remains text-only. This is a missing dependency in the task's file list; no contract waiver is needed. The same early icon dependency must be handed to Task17 for forge. No Task18 file was edited or compiled.

Task19 was interrupted on the global stop. Its untracked muster.rs test and external package drafts are retained. The test was archived with SHA under t19-builder/pre-stop; no compiler was used. No unmerged lane or source was deleted. All local workers/builds are stopped; the single warm compiler cache remains at target/mc-t8.

Wave1 is complete and pushed at8e95e42346d260f24340b9e3b27c7b92e2ea820f, draftPR https://github.com/26medias/TuRTS/pull/2. CI https://github.com/26medias/TuRTS/actions/runs/41097571682 remains running at last read: agents success, check on M2native refutations, browser on T7. Reading that already-running CI is the only continuing work. Main is unchanged.
