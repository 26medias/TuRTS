# Human playtest repair — 2026-10-28

User feedback supersedes the previous human-usability verdict: they could identify no objects and discover no action besides Disable/Remove. The industrial acceptance demonstrated engine behavior but did not establish understandable human onboarding. Do not equate the original automated gates with usable gameplay.

Implemented in1354e33 on the integration branch: catalog-driven principal object/resource labels; named selection buttons; stationary/mobile/automatic role guidance; research costs, missing-stock explanation and replicated queue progress. Selection remains local and ownership checked. M2 layout is preserved for non-industrial catalogs. Transport pieces receive a world label when selected to avoid dense belt text. No AI, construction or new authoritative mechanics.

Regression evidence: existing UI suite before implementation23 passed/2 new discovery tests failed. Full client suite after repair161 passed/0 failed/1 ignored; strict workspace Clippy passed. First new-test invocation accidentally selected0 and is excluded. An intermediate full test found the new Selection resource absent from a manual observer fixture; the fixture now initializes it and proves local selection adds no command and rejects enemy ownership. Intermediate M2 scroll regression led to restricting the industrial onboarding to catalogs with industrial capabilities, preserving its exact existing layout.

The optimized browser build, human-facing real-pointer research completion flow, second-client movement and existing seven-stage browser suite all passed. Previous763/0/18 and source07936e8 refer to pre-usability code. All75 agent runs remain used; root is executing this repair without additional agents. Evidence: target/m3-evidence/browser-startup/.

## Verified result

Full workspace: 765 passed,0 failed,18 ignored across90 result groups. Optimized browser build passed. New human-facing real-pointer flow passed: named lab selection sends no command; costs shown; actual research Applied1164; visible queue progress and completion; named turret selection reveals Damage20. Separate second-client test passed: no enemy factory selection controls; named drone selection explains right-click; command applied and observed position moved from(30,4) to x<29.5. Existing seven-stage Forge browser suite also passes (Produce1169, Disable1178, Remove1195). Captures and cleanup in human/, mobile/, existing-seven/. No user/browser default flags changed; desktop still requires the previously proven Vulkan flags.

New tests m3-human.js and m3-mobile.js are runnable against fresh servers using node with <fresh-owner-url> <artifact-dir>. Each closes its test browsers; the independent driver owns and stops its temporary server. Source/artifact hashes in usability-source-manifest.json. Prior full mutation evidence remains historical unchanged-engine evidence; no fresh independent review or entire mutation sweep claimed for the new UI.

## Current desktop preview

Self-contained run: `target/m3-playtest/20261028-1354e33/`; server PID2846901, desktop controller2847108. Same loopback7140/7141 URL, two named tabs with lab/distant drone selected. No automatic research or movement command was sent by setup. Factory tab waits for real stock; Research becomes available normally. Copied server/package are unchanged; new optimized client assets are SHA-pinned in playtest-manifest.json. Old explicitly owned preview/controller were stopped; existing user browser profiles and7000/7001 untouched. Initial replacement launch raced the old socket shutdown and exited on bind; retry after confirmed free ports is healthy. No player slot was consumed by the failed startup. Current preview consumes both slots intentionally; use its existing two tabs. Reset via this run's restart.sh, then reload Factory before Drones. Earlier preview directories are retained evidence and not the active server.

Fmt, strict workspace Clippy, full workspace tests, wasm workspace check and boundary checks all passed. Three temporary test servers were independently confirmed gone. The only intentional game/browser setup retained is this desktop preview.

## User scope closure and integration authorization

The user explicitly judged the demonstration inadequate for human evaluation despite the working systems, asked to consider M3 done and move on, and authorized “Merge to main, push.” This closes further demo polish under M3; it does not establish a successful human usability verdict. Technical acceptance remains765/0/18 with the qualified browser/fault evidence above. No new gameplay or UX work is authorized by this closeout. Both main integration and publication are explicitly approved; no additional confirmation is required.
