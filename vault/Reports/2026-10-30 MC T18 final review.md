---
type: report
date: 2026-10-30
tags: [turts, mc, review]
ai-first: true
source: codex
---

# RUN27 final independent Task18 review

Verdict: PASS, 0 High / 0 Medium. Candidate179ada763e9d5c87d62724cfa754da7776a7cd75. Scope: git diff codex/mc-integration...HEAD, 86 files, with repair-by-repair review. Root C1-C10 rulings and design revision12/O11 apply. No Claude files or role definitions were read.

## Resolved source findings

- Forge Move has explicit Some(Never), orders/order_move.ron:1. Its public catalog policy assertion is crates/turts_pack/tests/mc_packages.rs:312.
- Cedar/Haul have Produce/CancelProduction with correct Content/Job targets, depot subjects, key_produce P/key_cancel C, images, exports and text buttons: each manifest.ron:10; orders/order_produce.ron:1 and order_cancel.ron:1; interface/hud.ron:7. Producer coverage assertion: mc_packages.rs:327.
- Base/Cedar/Haul accepted-resource Gather precedes generic Node Move: orders/rules.ron:5. Ordered-policy assertion: mc_packages.rs:371.
- Forge Node Move exists: orders/rules.ron:5. Assertion: mc_packages.rs:407.
- Forge Enable/Disable include drill/press/arm_loader/lab/turret and exclude belt/drone: order_enable.ron:1 and order_disable.ron:1. Exact subject assertion: mc_packages.rs:426. Existing toggle mechanisms are unchanged. PT1 uses the Shell Press under O11.

C1 HUD3 fixture construction is isolated to valid probe exports (crates/turts_checks/tests/no_content.rs:306); all four token assertions and scanner/support are byte-identical. C4 schema dispatcher adds typed MC parsing and retains old parsing/default assertions and unknown-path panic. C5 metadata expectations match the planned versions while retaining hash, size, URL and economy checks. C8 explicitly checks all original10 Cedar files plus approved14 MC files and preserves binary assets when mutating conversion RON. C9 adds actual exported OrderDef IDs to the existing scratch clone mapping; loader/command/trace/conservation code after that helper is byte-identical.

C6 generator handles all four MC export lists, deterministic paths, order definitions/names, unit/order/resource references, asset and input IDs, nested interface references and mapping provenance. Engine override names, cursor contexts and minimap colors stay in their own namespaces. The enriched MC fixture has nonempty Balance sources/values, sound responses, fonts, cursors, unit presentation and Input buttons, including overlapping select mod action/engine override. Whole-MC inverse equality retains all old typed equality checks. Twenty-six original production functions are byte-identical, including build, role transforms, publication guards, discovery/hash and legacy transforms; original validate guards are verbatim before added MC guards. No rejection or economy assertion was weakened.

## Independent execution

All Cargo commands explicitly used CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_INCREMENTAL=0 CARGO_BUILD_JOBS=4 and CARGO_TARGET_DIR=/home/julien/Projects/TuRTS-mc-t18/target/mc-t18.

- cargo test -p turts_pack --test mc_packages --test economy_variants --locked: exit0, 9+7 tests.
- cargo test -p turts_checks --test no_content --example m2_variants --locked: exit0, 9+7 tests.
- cargo test -p turts_mod_api --test schema_m3 --locked: exit0, 4 tests.

These36 tests exercise public package loading, old0.2.2/no-orders compatibility, all four CLI version rejections, a new data-defined Move, package policy/subject coverage, full MC rename/inverse references, generator guards, scanner/token plants and typed schema loading. Actual command/env/RC records are in independent-results.json; raw logs have terminal exit0.

Required independent plant: remove only VERSION_TABLE exports.input row. cargo test -p turts_pack --test mc_packages each_version_row_refuses_0_2_2 --locked -- --exact --nocapture returned101, named each_version_row_refuses_0_2_2 and exports.input, actual Some(0) versus expected Some(2). Restored byte-identically, reran same command: exit0. Before/restored SHA256972342f7085a9f8f38a5d6f5336b17cf098906bfc66615c35ab587f82e6ed169. Evidence: independent-input-row-records.json and independent-input-row-{red,green}.log. This is a compiled semantic failure, not a compile error.

## Gate and invariant audit

Builder final179ada7 receipts/raw logs show G-RUST, G-QA and G-FROZEN actual exit0. First G-ACC invocation remains historical exit1 due relative-path artifact retention; corrected unchanged-script/same-cache absolute invocation has actual top-level exit0. All18 retained F11 artifact hashes were independently recomputed and match.

Root C7 exact registered E2/E3 cases each have actual RC sequence[0,0,0,101,0,0], named semantic failures and restored greens. All12 phase logs match their receipt hashes; source restored hashes equal the candidate; production generator binary restores to23ad2ab7075aca093e2fbe6d81639a79d81a4a4be24e7ce3867184c472c7b1f2. Builder prescribed input-row red101/green0 and exact restoration were also read directly. Evidence: builder-gate-audit.log, c7-audit.log and original referenced builder/root receipts.

Independent immutable-source audit passes:25 exact order IDs/behaviors/targets/subjects/hotkeys/image references, all25 production PNGs decode, four HUDs remain text-only, all22 maps/units/recipes/technologies files are byte-identical, scanner/assertions preserved. Frozen stream SHA256bcbf72c2206251f635a7c7f5b6c8f573d69a5fc05a5bd77716e98ebfb08e370b. Original historical no-content hits were order-name Move/Stop, not prescribed table IDs; no scanner exception or table-ID rename occurred.

Final state: candidate unchanged, git status --porcelain empty; git diff --check exit0; no active Cargo/rustc/rust-lld process. Reviewer releases compiler/plant ownership clean and idle. final-state.log records this check. No commit or staging by reviewer.

Limits: actual smart-resolver execution is NOT TRACED until its owning later task. This review proves package authoring/loading contracts. PT1 Shell Press timing remains NOT TRACED until PT1. No human usability or browser acceptance claim is made for this package-only task.

C11 follow-up: root defers the permanent CI hook for the seven m2_variants example tests to Task24, the planned .github/workflows/ci.yml owner. Task24 must add cargo test -p turts_checks --example m2_variants --locked before CI1/REG1; cargo test --workspace alone does not run this example test target. Local independent7/7 and E2/E3 proofs are retained now. This is an explicitly tracked follow-up, not a Task18 test waiver or source block.
