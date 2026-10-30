---
type: decision
date: 2026-10-30
tags: [turts, mc, codex, rulings]
ai-first: true
source: codex
status: active
---

# MC Codex rulings

Authority: CEO decision at main c3f09d0, MC Codex brief sections 3 and 4. The orchestrator reviews these rulings after each wave and can reverse one. A reversal is repaired in the next wave. Design revision 12 at 63788d7 is current. O11 already changes PT1 to the Shell Press; it is an orchestrator ruling, not a new Codex ruling. Its computed timing remains NOT TRACED until PT1.

## C1: repair the dependent HUD3 fixture

Conflict: Task18 updates the real Acme manifest, but the older HUD3 token fixture inserts duplicate orders/input/assets fields and overwrites bindings used by the copied package. The plan does not list this dependent fixture update. Evidence: crates/turts_checks/tests/no_content.rs:304-331; Task18 test failure is retained in target/mc-evidence/t18-builder/logs/g-rust-workspace-test.log; original source is in git commit 0e83be5. Design11.3 requires token coverage to remain strict.

Choice: repair only fixture construction so it preserves a valid copied package and adds its probe definitions without duplicate exports or orphaned bindings/assets. Preserve all token assertions and scanner implementation. Follow the existing fixture pattern; rerun the real source scan and the token fixture. This is an instrument/fixture repair under brief section4, not a scanner exception.

Alternative not taken: remove or weaken the token assertion, change the scanner, or freeze Acme on its old manifest solely for this test. Those choices would hide the regression or lose the required token proof.

## C2: repair the Task18 Medium findings and subject coverage

Conflict: candidate 0e83be5 misses authored policy and capability coverage. Evidence: mods/forge/orders/order_move.ron:1 lacks Some(Never), required by design5.2 (:503); Cedar and Haul units/depot.ron:7 have producers but their manifest.ron:10 lacks production/cancel orders; Base/Cedar/Haul orders/rules.ron:5-6 places generic node movement before accepted-node gathering, contrary to design4.3 (:400-410). Forge orders/rules.ron:2-5 omits node movement. Forge order_enable.ron:1 and order_disable.ron:1 omit lab/turret, although command_m3.rs:73-78 and economy_ui.rs:309-315 support research/weapon toggling.

Choice: repair all findings before merge. Add meaningful catalog and policy tests, named semantic reds, then fix data. Use Some(Never) for Forge Move, provide Produce/CancelProduction for each producer package with the already approved order_produce/order_cancel IDs, order Gather before generic Node Move, add Forge Node Move and include all existing toggleable Forge subjects. Keep belts excluded under O11. Keep maps, units, recipes, technologies, scanner and frozen values unchanged. The table's producer rule and design12.2 cover these necessary package definitions; no new mechanism or key is introduced.

Alternative not taken: defer the findings until protocol/HUD tasks. Deferral would carry incomplete package policies into their consumers and remove existing production or toggle controls after migration.

## C3: continue Task18 beyond twice its estimate

Conflict: Task18 estimate is 2 runs. Its first preparation, implementation and initial review consumed runs19/23/25. The next repair run26 reaches 4 runs, twice the estimate. Brief section3 at c3f09d0 permits continuation and stops only at four times the estimate.

Choice: continue with a bounded repair and independent recheck, registered before dispatch. The current defects are known data/fixture issues; no frozen value, K call or player-visible contract changes. Expected next two runs bring Task18 to 5 runs. Record the overrun in the daily report. Hard task stop is 8 runs; global stop remains 170. Reassess each further run and stop at either hard limit.

Alternative not taken: stop again at 4 runs under the old rule or waive validation. The CEO replaced that old stop rule; the repaired data still needs independent review and all gates.
