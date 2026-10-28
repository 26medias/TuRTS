# M3 acceptance evidence

All20 implementation entries, checkpoints C1–C4 and final technical/player/mod-author reviews are accepted after the final repair. Ready for local Forge playtesting. Hosted CI, push and merge to main remain pending approval. Evidence paths are relative to the original repository root.

C4 tested exact source `dae673986a84277dc8eb33417881066f1e71b0f4`; all574 tracked SHA256 values match and its worktree is clean. Final repair07936e8 additionally rejects invalid negative/nonfinite geometric scalars at package loading and direct bootstrap, and corrects documentation. Gameplay, wire, UI and valid authored fixture behavior are unchanged. Historical C4 evidence is qualified reuse, not a claim of fresh execution on07936e8. The authoritative requirement/evidence map is `target/m3-evidence/c4/proof-index.json`; verdict is `report.md` beside it.

| Requirement | Accepted local evidence |
| --- | --- |
| F1 content independence | Clean source scan, three recipe/technology/unit negative fixtures, renamed industrial trace, native E1/E2 mutation proofs. |
| F2 unchanged M2 behavior | Immutable stream SHA `bcbf72c2206251f635a7c7f5b6c8f573d69a5fc05a5bd77716e98ebfb08e370b`; native33, browser15 and process regression suites. |
| F3 public package and boot | Forge/Cedar public package validation, pack smoke and serial boot/process acceptance. |
| F4 authored timing | Literal10/20/30Hz timing and phase suites in the exact-source760/0/18 workspace run. |
| F5 conservation | C1–C17 scenarios and all17 exact formal fault baselines, named semantic reds and exact restored greens. |
| F6 fixed point | Source-driven industrial fixed-point scenarios and restored fault checks. |
| F7 research combat effect | Real F11 audit and both client traces: A100→0 in10×10; B100→0 in5×20. |
| F8 privacy/lifecycle | Owner-only research, raw owner projections, public health/removal; leaked slot0/1/2 research fixture is rejected. |
| F9 registered faults | Independent C4 native M2 33/33 and M3 17/17; all8 browser faults independently audited with exact source/reuse limits below. |
| F10 real browser controls | Fresh C4 seven-stage Forge: Produce applied1170, Disable1179, Remove1197; both press ports, screenshots and real receipts. Fresh T7:135 frames, final(20,8), oracle PASS. |
| F11 state-triggered two-client path | C4 owner0 research private; opponent1 moves1364 after research1361. Early release causes actual Move16 before research1771 and semantic rejection; exact restore passes. |

`target/m3-evidence/c4/report.md`, `final-source-guard.json`, `causal-probe-index.json`, and `artifact-provenance.json` retain detailed scope, source and binary identity, raw logs and process cleanup. The783-function inventory adds10 versus C3 and3 versus accepted UI; all three new F11 functions have causal proofs. No performance budget is claimed.

Browser proof reuse is explicit: first5 mutations ran on35355 with the older runner and unchanged case routes; last3 ran onf3940ce with the reviewed recovery helper. All have baselineGREEN, named semanticRED and byte-restoredGREEN. They do not imply the older five exercised final cleanup code. Current Python15, fresh C4 native33 and dedicated interruption/mtime probes cover that contract. Root aggregate: `target/m3-evidence/t16-root-browser-eight-index.json`; C4 audit: `browser-eight-independent-audit.json`.

Positive browser15 ran onf3940ce across9 invocations (base4, haul2, acme2, cedar2, five loss families). C4 independently audited raw source, receipts, screenshots, complete-state comparisons and cleanup as the joint F2 gate, rather than rerunning identical cases. Source/UI/runner bytes match the frozen C4 source. Root index: `target/m3-evidence/t16-root-browser-positive15-index.json`; independent audit: `target/m3-evidence/c4/m2-positive-independent-audit.json`.

Earlier wrong-branch, substituted C6/C10/C11, compiler-error, zero-selection, aborted and infrastructure attempts remain excluded. Initial Task16 R1 findings were repaired and independently closed (`target/m3-evidence/t16-review/recheck.md`). Decisions and costs are in [[M3 Orchestrator Rulings]]. Historical checkpoint records remain in `target/m3-evidence/{c1,c2,c3}/root-acceptance.md`.

## Final repair acceptance

Current code07936e8:763 passed,0 failed,18 ignored across90 result groups; fmt, strict Clippy, wasm, boundaries and frozen M2 stream pass. Fresh formal17, public packer19, optimized Forge browser7 and direct-bootstrap guard causal proof pass. Final scoped Sol review reports PASS, zero findings. Evidence: `target/m3-evidence/final/fix/report.md`, `proof-index.json`, `source-manifest.json`; review: `target/m3-evidence/final/technical/recheck.md`. Root confirmed acceptance server/static/Xvfb PIDs are gone; only the intentional playtest remains. See [[Reports/2026-10-28 M3 ready to test]].

Human-playtest follow-up: prior usability verdict superseded by user feedback. Named selection/world labels, role guidance and research feedback repaired; fresh workspace765/0/18 and human research/mobile/existing7 browser flows pass. [[Reports/2026-10-28 M3 human playtest repair]].
