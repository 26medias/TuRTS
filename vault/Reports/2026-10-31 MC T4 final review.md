---
type: report
date: 2026-10-31
tags: [turts, mc, review, task4]
ai-first: true
source: codex
---

# Task4 independent review — final

Verdict: **PASS — 0 High, 0 unresolved Medium** on exact candidate `2c45b8311906caf14793e7eb75b0bbb84462bdb1`. Task4 candidate is approved for root integration. Required per-merge integration gates and any main merge/push authorization remain with root.

Reviewer: RUN34, independent QA lead. Builder/compiler owner: RUN32. Lane: `/home/julien/Projects/TuRTS-mc-t4`, branch `codex/mc-t4`.

Final candidate: `2c45b8311906caf14793e7eb75b0bbb84462bdb1`. Full builder gates/C55/plan plants and the reused refutations actually executed on `0240e9ae059e1e022ab5acb9dc8b4900f58ea640`; C58 permits their reuse because the sole descendant change is one expected-failure diagnostic in the M2 registry. No execution source is relabeled. Registered base: `ab6ffd8b32a504ff078f26e6e4c49bab604468f1`.

Reviewed against MC design revision12 sections4.5,5.2,5.3,6.1; K4(b), K19(a), K24; Task4 execution/local gates and Codex rulings. Pinned Bevy0.20 source takes precedence over older skill examples. Codex Rust best-practices/testing and Bevy ECS/systems/testing skills were read independently. No Claude configuration, role files or skills were imported.

## Scope and design coverage

| Contract | Source/evidence reviewed | Result |
| --- | --- | --- |
| Orders precedes Movement; stable unit-id iteration | sim plugin/orders; real fixed-schedule capture and phase plant | Matches required schedule and tick t landing → Orders t+1 queued activation |
| Per-unit validation and results | server orders; sim orders/command; exact300 mixed case | Existence → owner → capability → authored filter → target → queue; 290 mobile move, five static and five foreign remain unchanged |
| Command envelope / wire | all envelopes become boxed CommandKind::Order; MAX4096; eight new reasons; owner-only orders in export | Required route preserved; C19 malformed/wholly-invalid Produce admissions retained |
| Queue data and lifecycle | public u8 queue depth, validated originating-package map; UnitOrders current/VecDeque/failure | Min1/max255/default16, true fresh plain Move then Append, barriers, Stop, depletion and queued failures covered |
| Move targets and plain compatibility | target-kind authority, Point clamp, Unit live-follow/targetgone/completion | Fresh single Point Replace/group0 writes legacy MoveTarget only; Unit replacement remains tracked; direct legacy outside-map behavior retained |
| Immediate effects | Produce/Cancel/Remove/SetEnabled and independent state/event/refusal models | No unit-order queue entry; deterministic first successful JobId; research/machine/reservation/refund/removal/toggle families checked |
| Audited writes and boundary integrity | CommandAudit spans; OperationAudit phase/subject; automatic Orders transitions | Full-envelope before/after; malformed/overlapping spans fail; per-unit writes and repeated automatic transitions distinguished |
| EL1 policy and movement replay | immutable captured catalog authority, semantic result/write models, shared pure movement step | Joint command/policy forgery, catalog transition, actor teleport and economic corruption negatives reviewed |
| Preserved M2/M3 contracts | independent M2 E8 bridge, full accounting oracle, Forge/frozen checks; C44 registry reanchors | Old oracle retained; accepted Order bridge is bounded/fail-closed; no frozen expected-value change |

Attack and group spreading/avoidance remain Tasks5/6. Task4 owns unit-test coverage, not final ORD2/ORD3/EL1 browser acceptance rows.

## Findings and dispositions

All reported High implementation/integrity findings were repaired and rechecked in source and targeted evidence. No open source High or Medium remains. C32 count-only mixed-envelope proof (Medium) was repaired with independent targets/movement/refused-state checks. All required candidate review obligations below are complete; no source finding was waived.

- C25–C26: queue-depth package authority and public u8/private raw-u16 diagnostics preserve accepted data, default16 and exact0/256 diagnostics.
- C27–C30: plain cleanup applies only to prior state on Point fast path; mixed Produce reaches per-unit domain checks while C19 admissions remain; first successful sorted JobId is retained; representable target errors occur after subject priority.
- C31–C35: complete command spans/refusal evidence, mixed state effects, overlap/incomplete-span rejection, principal-subject tags with behavioral models, and actual emitter-derived validator fixtures. Tags alone are insufficient proof.
- C36: declared Move Unit/PointOrUnit semantics implemented through arrival; replacing current+queue with group0 Move(Unit) is tracked, with Point-only cleanup guard and captured regression proof.
- C37–C38: M3 one-cache wrapper preserves original CLI/semantic/frozen/provenance guards; immutable catalog order definitions are replay authority, including resolved policy/capabilities.
- C39: acceptance T5 dedup preserves one named aggregate pass or exact fallback; shell-stub missing/failing/duplicate/zero-test cases inspected. Stub artifacts were explicitly rejected as real F11 proof; fresh final F11 is retained separately. Root Trunk dedup retains T7 build/runtime/oracle.
- C40–C42: Box is Rust layout only; trace canonicalization retains every authority field; process fixtures now assert the new finite-clamp/not_applicable/owner-order contract without weakening full DTO/frozen input/M2 accounting evidence. Spawn initialization bug was fixed in product code.
- C43–C45: authored repeated unit-filter entries retain membership semantics; Remove honors by_command before filter/target. Existing spawn/Gather plants are reanchored without retirement. Remove replay now models materials/events/state removal, belt splits and dependent inserter/gather/weapon links.
- C46–C49 and C52 concern separate Task9 preparation/interface decisions; this report does not claim Task9 behavior or runtime completion.
- C50–C51: machine Cancel restored without widening Produce. Batch fixes cover reserve canonical ordering, research eligibility/all stations/ID priority, late refund failure, exact no-write refusals, idempotent carry and event allocation. Machine actual Running/OutputBlocked fixtures refund one item from input2×1/2 and reject missing credit.
- C53–C54: exact-commit receipt auditor was corrected for M3 map hashes and full source/selector coverage; source scanner collision fixed by local identifier renames only. Candidate0240 differs from6056 only in those names/format.
- C55: seven missing controlled-mutation provenance records were rerun after stable gates; exact patch/base/mutated/restored hashes and raw named RC101/restored RC0 were independently reconstructed/checked. Corrected composite maps the seventh test from its raw command; original erroneous instrument attempts remain separate.
- C56: root integration helper now explicitly adds unchanged M3 controls when requested with-web+with-m3. Prior Task2b postmerge helper did not separately run M3; that process gap is disclosed, not relabeled as a pass.
- C57: split actual M2 components may be reused only after identical source, disjointness and exact union against both plant-path forms are proven by the read-only composite auditor. Actual final split/M3 receipts have been independently audited, including exact union and recorded execution source.

## Audited candidate gate evidence

These gates actually ran on0240e9a. Exact git-tree comparison confirms final2c45b83 changes only tests/m2/refutations.json, and only E8-mixed-cost-partial-commit.expected_failure; all runtime/test/data blobs are identical. Root C58 authorizes metadata-only reuse; Independent tests below have now passed on2c45b83; per-merge integration gates remain required.

Builder evidence root: `/home/julien/Projects/TuRTS/target/mc-evidence/t4-builder/run32`.

| Gate | Saved evidence reviewed | Result |
| --- | --- | --- |
| G-RUST | g-rust-*-final-0240e9a.log | fmt/strict Clippy/wasm RC0; workspace984 passed/0 failed/18 ignored across99 summaries, RC0 |
| G-QA | yardstick/completeness/driver logs and their selftests; boundary | All actualRC0; completeness21/21 and driver47 selfcases |
| G-ACC | g-acc-final-0240e9a.log + acceptance-final-0240e9a | Required process selectors pass; T5 named aggregate pass exactlyonce; genuine F11 pass |
| F11 provenance | retained provenance + artifact-sha256 manifest + summary | Exact candidate clean source/no diff; all18 retained artifact hashes independently checked (~837MB); ordered owner/opponent health/research proof present |
| G-WEB | g-web-t7-0240e9a.log/result + g-web-m3-browser-0240e9a.log/result/cleanup | Real Trunk; T7 138frames/54moving, exact oracle RC0; M3 Produce/disable/remove receipts, seven captures/twoport panel, cleanupRC0 |
| G-FROZEN | g-frozen-final-0240e9a.log | Forge26 + fixedpoint1 + frozen stream1 pass; RC0 |
| G-REF | Actual18M2/78commands on0240 + actual4M2/36commands on2c45 + actual14M3/42commands on0240 | PASS: exact22M2+14M3 union,156 actual commands,36 native semantic RC101 reds/72 phase greens; full original guards/source/frozen/cardinality provenance verified |

Frozen M2 SHA256 remains `bcbf72c2206251f635a7c7f5b6c8f573d69a5fc05a5bd77716e98ebfb08e370b`. Frozen fixture and Forge expected values are unchanged. Existing hosted M2 economy timeout remains Task21; local browser proofs above are actual current-candidate executions, not a claim that hosted behavior is repaired.

## Mutation proofs and provenance

Both mandatory Task4 plants were audited from `plan-plants/plant-a` and `plant-b`. Exact candidate source hashes, saved patches, mutated hashes and restored hashes were independently reconstructed in memory. Raw named tests select exactlyone case per command.

- PlantA inserts current state in shared apply_move_to: plain-envelope test fails UnitOrders emptiness, frozen stream fails Acme operation total2 versus1; both RC101. Exact restoration then both named RC0.
- PlantB changes capture_unit_order_operation emitter Orders→Other: actual captured/decoded phase fixture fails with `unit 1 movement differs from its prior target and speed`, RC101. Exact restoration and named RC0.
- Seven additional C55 controlled proofs: duplicate filter, disabled removal capability, research priority, ID allocation, canonical reserve lines, disabled-extractor carry, groupzero Move(Unit). Independent mapping/hash/cardinality audit: `c55-independent-audit.json` beside this report.

Historical development was not untouched-base red-first: behavior/scaffolds preceded some ORD2/ORD3/EL1 test observations. Earlier controlled compiling regressions and later C55 exact-provenance reruns demonstrate the missing semantics. They do not rewrite that development order. Compile/setup errors, nonunique anchor attempts and an IndentationError are preserved as failed instrumentation attempts and excluded from semantic proof. No assertion inversion is counted as a valid red.

Actual app captures include mixed envelopes, true plain/Append lifecycle, captured emitter, live follow/queued Gather scheduler paths, research both payment policies, canonical reserve Produce→Cancel, machine stages and material-bearing Remove. Remove dependent-link augmentation and late-refund records are synthetic decoded fixtures with consistent projections; they are not described as actual dependent-link process captures. Late-refund Full/Overflow are additionally tied to passing original simulation implementation tests. Already-disabled nonzero extractor carry is a defensive synthetic state built over an actual idempotent command capture.

## Independent execution and handback — complete

Before explicit root handoff, this reviewer used only read-only source/evidence inspection, AST/shell syntax/static checks and external review artifacts. After exclusive compiler handoff, the approved focused checks and one semantic plant were executed below.

All65 focused tests passed on the exact final candidate:

| Independent command | Selected/pass | Actual RC |
| --- | --- | --- |
| cargo test --locked -p turts_sim --test orders -- --nocapture | 16/16 | 0 |
| cargo test --locked -p turts_server --test orders_plain -- --nocapture | 7/7 | 0 |
| cargo test --locked -p turts_server --bin turts_server economylog::tests -- --nocapture | 41/41 | 0 |
| cargo test --locked --release -p turts_server --test economy e8_commands_are_authoritative_and_acked -- --exact --ignored --nocapture | 1/1 real process | 0 |

Command logs, exact environment, log hashes and summaries are in `independent-tests.json`; executable orchestrator is `run_independent.py`. No full workspace/E9 gate was repeated by this reviewer.

Own semantic plant suppresses only `validate_automatic_order_operation(operation)?` in economylog.rs. The exact named `validator_rejects_a_forged_automatic_orders_completion` baseline passes0; with the call suppressed it compiles and fails101 because `unwrap_err()` receives Ok (forged automatic completion accepted); exact original bytes are restored, and the same named test passes0. Each phase selected exactlyone test. `automatic-plant-receipt.json` pins commands/environment/log hashes, patch hash and base/mutated/restored source SHA. Original/restored source SHA256: `de85bbaefe568681d7da32c73013c6c837e13ee813ea4e1ee5f04a2c991e7234`. No test assertion was changed.

Only `/home/julien/Projects/TuRTS-mc-t4/target/mc-t4` was used, with DEV_DEBUG0/TEST_DEBUG0/INCREMENTAL0/BUILD_JOBS4. Final git status is empty, HEAD exact2c45b83, frozen hash unchanged and a /proc inspection found no lane compiler/target processes. `idle-handback.json` records the handback at2026-10-31T05:53:26.764368Z. Compiler/cache ownership was explicitly released to root, and the builder was notified. No further reviewer Cargo/source mutation is planned.

## All changed paths

Compared exact candidate to the registered Task4 base; 43 paths. Paths include required adapters, compatibility fixtures and C44 registry work. No vault/document source was modified by this reviewer.

```text
crates/turts_client/src/content.rs
crates/turts_mod_api/src/content.rs
crates/turts_mod_api/src/orders.rs
crates/turts_mod_runtime/src/archive.rs
crates/turts_mod_runtime/src/content.rs
crates/turts_mod_runtime/src/content_mc_validation.rs
crates/turts_mod_runtime/tests/mc_content.rs
crates/turts_pack/tests/common/accounting.rs
crates/turts_pack/tests/economy_ordering.rs
crates/turts_pack/tests/economy_variants.rs
crates/turts_pack/tests/forge_cases.rs
crates/turts_pack/tests/forge_renamed_clone.rs
crates/turts_server/src/economylog.rs
crates/turts_server/src/export.rs
crates/turts_server/src/orders.rs
crates/turts_server/src/session.rs
crates/turts_server/tests/acceptance.rs
crates/turts_server/tests/economy.rs
crates/turts_server/tests/economy_export.rs
crates/turts_server/tests/orders_plain.rs
crates/turts_sim/src/boot_m3.rs
crates/turts_sim/src/bootstrap.rs
crates/turts_sim/src/command.rs
crates/turts_sim/src/command_m3.rs
crates/turts_sim/src/economy/combat.rs
crates/turts_sim/src/economy/extractor.rs
crates/turts_sim/src/economy/inserter.rs
crates/turts_sim/src/economy/machine.rs
crates/turts_sim/src/economy/production.rs
crates/turts_sim/src/economy/research.rs
crates/turts_sim/src/economy/transaction.rs
crates/turts_sim/src/economy/views.rs
crates/turts_sim/src/lib.rs
crates/turts_sim/src/map.rs
crates/turts_sim/src/movement.rs
crates/turts_sim/src/orders.rs
crates/turts_sim/src/plugin.rs
crates/turts_sim/tests/boot_geometric_validation.rs
crates/turts_sim/tests/command.rs
crates/turts_sim/tests/extractor.rs
crates/turts_sim/tests/orders.rs
scripts/acceptance.sh
tests/m2/refutations.json
```

## C58 metadata-only reuse audit

The original M2 wrapper selected21 and completed18 before its E8 mixed-cost plant failed the old expected-message guard. The raw test compiled, selected exactlyone and failed RC101 because the Task4 validator caught the injected partial debit earlier: `refused unit 1 changed state inside its order operation`. This is the original economic defect, not a new mutant or retired plant.

Final2c45b83 changes only that case's `expected_failure`. The updated C53/C57 auditors preserve actual execution sources; an optional completed-only projection pins the original receipt hash, retains exact original passed rows and command filtering, and resolves to original case directories. Parent reuse requires descendant source with exactly the one registry field difference, unchanged case order/every other case field/used case definitions, nonoverlap and full registered-base selector union.

Reviewer independently checked the current receipt mapping and exact tree/field diff; C53 read-only runs passed for actual18M2/78commands and full14M3/42commands, with M3 source/frozen/selector provenance. Reports: `m2-completed-independent-audit.json`, `m3-independent-audit.json`. Actual source remains0240e9a in both; root final-source M3 map to2c45b83 explicitly labels metadata equivalence. The failed/uncompleted three M2 plus the previously missing foreign-Gather case actually executed on2c45b83: four cases/36 commands, all original guards pass. Reviewer ran C53 independently on this component too (`m2-remaining-independent-audit.json`) and recomputed the complete final selector/component hashes/phase RCs. `final-refutations-independent-audit.json` records exact22M2+14M3,156 actual commands,36 native semantic reds and72 greens. All original failed/setup receipts remain separate and excluded from successful coverage.

## Review artifacts

The review directory contains the final report, provisional history, saved-proof audits, exact independent commands/raw logs, one source patch with restoration proof, and idle handback. `artifact-index.json` records their SHA256 hashes. External auditor sources inspected: C53 SHA061350c841152b7124ae7538fec531ebcd1abdf69181a52fcd318e8fc7efc31c; C57/C58 SHA9d404bc3e8c44a42c432b22e98a773ee7dc2802d5f26099a99de31de74b1e664. These are saved-proof checks; the actual test/plant executions remain attributed to their recorded source and owner.
