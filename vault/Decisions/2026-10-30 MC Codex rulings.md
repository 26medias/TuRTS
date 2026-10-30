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

## C4: extend the old-package RON test dispatcher

Conflict: the Task18 packages now contain MC orders/input/interface/assets files. The pre-MC schema test visits every RON file but recognizes only older directories, so old_ron_still_loads panics on mods/base/orders/order_move.ron. Evidence: crates/turts_mod_api/tests/schema_m3.rs:16-43 and :113-117; actual workspace gate exit101 in target/mc-evidence/t18-builder/repair-run26/g-rust-workspace-test.log.

Choice: add typed parsing for the new directories in this test helper: OrderDef, OrderRulesDef for orders/rules.ron, InputMapDef, InterfaceDef and AssetsDef. Keep the existing legacy parsing and all default-serialization assertions. Keep the unexpected-path failure for genuinely unknown paths. Run the named schema test red then green, and re-gate the repaired candidate. This is a dependent instrument/fixture update under brief section4.

Alternative not taken: skip the new directories, delete old-package coverage, change package schemas or weaken unknown-field checks. Typed parsing preserves the intent and checks all package definitions now present.

## C5: align dependent package metadata assertions with the planned minor bump

Conflict: Task18 explicitly changes Acme/Base/Cedar/Haul from0.2.0 to0.3.0 and their engine requirement to^0.2.3, while older tests assert0.2.0 archive names/advertisements or mutate the old version anchor. Evidence: plan Task18:425; crates/turts_server/tests/unit_boot.rs:119-147; crates/turts_mod_runtime/tests/t2c_package_rejects.rs:99; crates/turts_pack/tests/economy_package_boot.rs:136,274,381,404; crates/turts_pack/tests/economy_variants.rs:436. The Cedar file/export enumeration at economy_package_boot.rs:405-470 likewise predates the mandated MC additions.

Choice: capture named targeted failures, then update only dependent metadata expectations and edit anchors to the planned0.3.0/^0.2.3 values. Retain every economy/world invariant, original Cedar file and legacy export assertion; extend the explicit expected file/export set with approved MC files rather than drop coverage. Keep independent old-version fixture tests on their intended versions. Leave archive naming and advertisement mechanisms unchanged. This is an instrument/fixture alignment, not an acceptance-row or frozen-economy change.

Alternative not taken: retain old package versions contrary to Task18, weaken semantic economy assertions, or broadly replace every0.2.0 fixture. All unrelated old-version compatibility tests remain intact.

## C6: support MC exports in the existing M2 variant instrument

Conflict: the existing variant instrument recognizes only pre-MC exports, so the Task18 packages' orders/input/interface/assets RON is treated as unexported. Evidence: crates/turts_checks/examples/m2_variants.rs:1109-1134 (exported/category), :1380-1391 (renamed manifest paths), and transform_export at:1395 onward. The package plans require these new exports while existing package/variant proofs must still run.

Choice: first capture the actual existing generator/test failure on a Task18 package, then extend the instrument along its existing typed deterministic rewrite pattern for MC exports, paths and content references. Keep original economy/resource/rename semantics, all rejection guards, complete mapping provenance and deterministic output. Validate generated archives and cross-references, including renamed package/unit/order references and relevant asset/input/interface references. Do not change original package gameplay files or the frozen operation stream. Run the existing generator contract/variant tests and meaningful new MC-reference coverage before the full gate set.

Alternative not taken: strip the new definitions from real packages, silently skip them in a supposedly complete rename, or remove variant proofs. Those choices would make the existing instrument incomplete or violate Task18 content requirements. This is an uncovered instrument extension under the nearest existing pattern, not a new game mechanism.


C6 namespace detail: preserve structural widget/layout IDs and engine namespaces (engine override names, cursor context keys, response action keys and colors); rename the corresponding mod action/asset/content references. Widget IDs are explicitly layout names, not gameplay tokens (plan Task3:360; scanner support/mod.rs:171-194). New nonempty reference fixtures must cover balance sources, presentation sound responses and overlapping engine/mod names. Do not infer coverage from empty interface maps.

## C7: run selected refutations with the lane's one compiler cache

Conflict: C6 and the metadata test update touch planted files. The selector lists E2-rename-canonicalizer and E3-policy-binding. The standard M2 wrapper creates a fresh cargo-target for each invocation (tests/m2/refutations.py:407-418), which conflicts with this build's disk/cache policy when repeatedly compiling the same workspace.

Choice: after final commit and normal gates, invoke the unchanged registered run_case function from tests/m2/refutations.py for the two selected registry entries, in the clean committed task lane with its existing target/mc-t18. Keep exclusive compiler/plant ownership, the exact registered precommands/commands, selected-test and semantic-failure guards, baseline/planted/restored logs, source hash provenance and finally restoration (refutations.py:298-390). Record this in-place layout in the receipt, verify full source status before/after and restore hashes even on failure. Do not modify the runner, registry or any guard merely for cache reuse.

Alternative not taken: allocate another full cold compiler cache or run ad-hoc commands without the registered runner's semantic checks. The layout changes; the tested plants and acceptance criteria do not.


C5 additional evidence: named red gates found current-package metadata expectations in turts_mod_runtime/tests/archive.rs:152-156 and forge_package.rs:19-20; align Acme lock version/API to0.3.0/^0.2.3 and Forge manifest version/API to0.2.0/^0.2.3. The Forge bump is explicitly required by Task18. economy_variants.rs:505 also expects Cedar's old10 files; retain those10 and assert the14 approved MC additions (24 total). Do not change catalog/economy/hash assertions or independent old-version fixtures.

## C8: keep binary assets out of the conversion fixture's text mutation

Conflict: declared_conversion_coefficient_is_used_by_public_trace decodes every file of a generated Base package as UTF-8 (crates/turts_pack/tests/economy_variants.rs:542-543). Approved MC packages now include valid binary PNG assets, so the existing test panics before its conversion assertions. Named failure exit101 is recorded in repair-run26/g-rust-workspace-test-ed13ac3.log. The independent Cedar file-count failure in the same log is covered by C5.

Choice: inspect/mutate only RON text definitions, preserve binary files byte-for-byte, and keep the one-conversion-coefficient guard plus all subsequent public-loader and positive economic trace assertions. Add explicit original Cedar plus approved MC file coverage when updating its count. Then targeted greens, commit and final gates. No source package gameplay or binary payload changes are authorized by this test repair.

Alternative not taken: decode binary assets lossily, drop them from packages, remove conversion coverage or relax economic assertions. The fixture must support the new approved package format while proving the same conversion behavior.

## C9: include the new order IDs in the renamed Forge fixture

Conflict: forge_renamed_clone.rs:29-63 enumerates only old Forge gameplay IDs when rewriting the copied package. New smart rules still refer to forge:order_move after the copy is renamed clone, so all three existing tests fail at loader helper:72 with unknown dependency namespace forge. Actual named exit101 output is in repair-run26/g-rust-workspace-test-6dbe394.log. Original Forge fixed point and cases passed before this fixture failure.

Choice: extend the fixture's rename-ID collection with the actual exported OrderDef IDs read through PackageArchive::orders, then apply the existing prefix/local-ID rewrite to the copied package. Keep original gameplay ID mappings, all command/operation-trace equality assertions and both conservation negative cases unchanged. Preserve original Forge content and expected frozen results. Run all three fixture tests, then commit and re-gate.

Alternative not taken: delete MC orders/rules from the clone, weaken trace/conservation assertions or change original Forge data. Typed order discovery keeps the renamed fixture valid as approved order content is added, without introducing engine hardcoded IDs.

## C10: pass the same lane target directory as an absolute path to acceptance

Conflict: all acceptance tests on179ada7 passed, but the final artifact-retention guard returned1. Its absolute F11 server marker is compared lexically with CARGO_TARGET_DIR/tmp/server_* (scripts/acceptance.sh:13,30-31), while this invocation supplied relative target/mc-t18. The marker points inside that same physical lane cache. Actual output is in repair-run26/g-acc-command-179ada7.log.

Choice: rerun the unchanged acceptance script with CARGO_TARGET_DIR set to the canonical absolute path of that same target/mc-t18. Use absolute target paths for future gate wrappers, retaining one cache per lane and all private-directory guards. Record the failed first top-level invocation as exit1 despite passing tests; accept the gate only after an actual top-level exit0. No source or cache duplication is needed.

Alternative not taken: weaken the artifact-retention guard, accept a nonzero gate, or allocate another target directory. This is an execution-path detail within the authorized cache/worktree decision rights.

## C11: wire the MC variant example tests into CI in Task24

Conflict: the new MC reference tests live under the m2_variants example test target. Current CI runs cargo test --workspace at .github/workflows/ci.yml:33, and the actual workspace log confirms it does not execute that example's test target. The seven example tests passed explicitly under the builder and independent reviewer, but need a permanent hosted entry.

Choice: defer the CI hook to Task24, the planned .github/workflows/ci.yml owner (plan:724-726), and require cargo test -p turts_checks --example m2_variants --locked in its check job. Keep the explicit local/example proof now and the selected E2/E3 hosted refutation coverage. Track this with the existing Task24 harness-test hook note; it must be complete before CI1/REG1 signoff. No test or gate is waived for Task18.

Alternative not taken: claim the workspace command runs these seven tests, omit their hosted guard from the finished build, or change the hot CI file ahead of its planned owner. This is a reasoned deferral of CI plumbing, not a change to the acceptance row or game behavior.
