# Human playtest repair — 2026-10-28

User feedback supersedes the previous human-usability verdict: they could identify no objects and discover no action besides Disable/Remove. The industrial acceptance demonstrated engine behavior but did not establish understandable human onboarding. Do not equate the original automated gates with usable gameplay.

Repair in progress on the integration branch: catalog-driven principal object/resource labels; named selection buttons; stationary/mobile/automatic role guidance; research costs, missing-stock explanation and replicated queue progress. Selection remains local and ownership checked. M2 layout is preserved for non-industrial catalogs. Transport pieces receive a world label when selected to avoid dense belt text. No AI, construction or new authoritative mechanics.

Regression evidence: existing UI suite before implementation23 passed/2 new discovery tests failed. Full client suite after repair161 passed/0 failed/1 ignored; strict workspace Clippy passed. First new-test invocation accidentally selected0 and is excluded. An intermediate full test found the new Selection resource absent from a manual observer fixture; the fixture now initializes it and proves local selection adds no command and rejects enemy ownership. Intermediate M2 scroll regression led to restricting the industrial onboarding to catalogs with industrial capabilities, preserving its exact existing layout.

Optimized browser build and new human-facing real-pointer lab -> research -> completion -> weapon-damage flow are pending, followed by the existing seven-stage browser suite. Previous763/0/18 and source07936e8 refer to pre-usability code. All75 agent runs remain used; root is executing this repair without additional agents. Evidence: target/m3-evidence/browser-startup/.

## Verified result

Full workspace: 765 passed,0 failed,18 ignored across90 result groups. Optimized browser build passed. New human-facing real-pointer flow passed: named lab selection sends no command; costs shown; actual research Applied1164; visible queue progress and completion; named turret selection reveals Damage20. Separate second-client test passed: no enemy factory selection controls; named drone selection explains right-click; command applied and observed position moved from(30,4) to x<29.5. Existing seven-stage Forge browser suite also passes (Produce1169, Disable1178, Remove1195). Captures and cleanup in human/, mobile/, existing-seven/. No user/browser default flags changed; desktop still requires the previously proven Vulkan flags.

New tests m3-human.js and m3-mobile.js are runnable against fresh servers using node with <fresh-owner-url> <artifact-dir>. Each closes its test browsers; the independent driver owns and stops its temporary server. Source/artifact hashes in usability-source-manifest.json. Prior full mutation evidence remains historical unchanged-engine evidence; no fresh independent review or entire mutation sweep claimed for the new UI.
