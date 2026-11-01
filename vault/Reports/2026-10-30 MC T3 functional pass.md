---
type: report
date: 2026-10-30
tags: [turts, mc, review]
ai-first: true
source: codex
---

MC T3 final functional recheck: PASS, 0 High and 0 Medium findings remaining on 67307d1a633495c6ae7c1565acc627af4c9762d6. This clears the functional review for final gates; it is not a declaration that pending gates or evidence receipts have completed.

Provenance: verified candidate HEAD and clean source status before and after probing. Every CLI invocation used only /home/julien/Projects/TuRTS/target/mc-evidence/t3-review/candidate-67307d1/turts-pack. The snapshot SHA256 matched 5b4a5828917a26b35f97570d202b0235e4f38f64b3debbe3130bc413c748b95c before and after, matching manifest.txt. The manifest records clean source HEAD, the exact build command, build_exit=0 and fixture tree hash eda46fdba9e0e9f003b138999a3571d1676938224c5fc9d9b5b81f7c87c2d24f. No live Cargo-target executable was used. No reviewer source edits, Cargo invocations, vault edits, merge or push.

Closed final High: OGG metadata cannot erase an already observed over-cap duration. asset_headers.rs:51-60 now validates stream serial lifecycle, BOS and page sequence; :80-85 refuses decreasing granules; EOS and complete sequential chains are checked later. Actual immutable CLI observations: same-serial post-EOS zero granule, same-serial stream reuse, and descending granule before EOS each reject exit 2. Single31s, distinct-serial chained32s and mixed-rate chained31s reject exit 2. A valid increasing-granule 5s stream and mixed-rate sequential 30s chain each pack exit 0. Thus the malformed cases are rejected without breaking the exercised valid streams.

Closed final Medium: archive.rs:293-295 and :324-337 use actual RON MapAccess keys and IgnoredAny values for schema classification. Rules with whitespace and comments before colons now report the actual unknown typo field with the rules schema; ordinary and raw OrderDef descriptions containing field-like text report the actual typo with the order schema. All reject exit 2. Removing only the typo field from the exact commented rules/raw-string order packages makes each pack exit 0, preserving valid RON syntax. Queue depth256 still rejects exit 2 with the field and 1..255 range.

All earlier High/Medium closures repeated against the same snapshot:

- Seven undeclared-namespace reference probes and three local wrong-kind unit/order/balance probes reject exit 2 with package/file/referring field. The existing resolver/MC single-package scope is preserved; no MP2 linker functionality was added or required.
- Widget and AssetsDef cursor dimensions/hotspots each reject exit 2.
- Canonical and permuted reserved modifier chords reject exit 2 with the same normalized text; duplicate modifiers reject exit 2.
- Primary portrait/icon Image templates with matching UnitPresentation each pack exit 0. Non-image Health binding and Item(Icon) action reject exit 2; Item(CancelAction) packs exit 0.
- Guide10px Text and20x20 Button pack exit 0 with both expected guide-field warnings.
- Complete key vocabulary confirmed statically: all194 plain installed Bevy KeyCode variants present, with only MetaLeft/MetaRight additional aliases. Representative BrowserBack, WakeUp, Convert, KanaMode, Lang1, Hiragana, Katakana, F35, MetaLeft and SuperLeft probes pack exit 0; KeyFlorp rejects exit 2.
- Engine-override token exclusion and its select/no-hit regression remain in source; read prior no_content nine-test log with numeric EXIT_CODE=0. Mod order/action/asset tokens remain covered. No reviewer compiler run.

Negative controls retained: unknown nested Value, missing image asset, unknown widget and CancelProduction(Point) reject exit 2 with the expected diagnostics. Baseline packs exit 0. Missing-icon scratch plant rejects exit 2; restoring that exact package's original order file packs exit 0 and reproduces baseline archive bytes/hash.

Actual reviewer command: python3 /home/julien/Projects/TuRTS/target/mc-evidence/t3-review/recheck-67307d1.py. Script orchestration exited 0; it hard-checks the snapshot SHA, invokes the immutable CLI through subprocess.run, and records56 actual child-process observations in recheck-67307d1-probes.log/json. These are API/package validation probes, not rendered-client acceptance. Scratch source payloads are removed after per-file hash capture, while the compact reproducer, logs, JSON and provenance remain. Snapshot removal belongs to root.

Read current builder evidence: review-runtime-final4.log contains123 passing tests and EXIT_CODE=0; review-pack-final4.log includes bad_packages58 and the complete pack suite, EXIT_CODE=0. Focused comment/raw-string/malformed-OGG logs each contain passing named tests and numeric EXIT_CODE=0. These are read receipts, not reviewer reruns.

Pending: final ordinary gates, G-REF29 cases, and the explicit isolated signature-stub instrument repeat with actual exit101/restored0 receipts remain builder/root-owned until their final logs are checked. Preserve the historical distinction: original22 HUD3 first-red logs have named semantic failures but no numeric exit marker; their historical numeric exit remains NOT TRACED. Later reserved/chained-specific red logs have actual101, and the earlier one-failure HUD3 run does not prove every new regression went red. No fabricated historical receipt and no all-gates completion claim.
