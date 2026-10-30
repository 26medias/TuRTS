---
type: report
date: 2026-10-30
tags: [turts, mc, review]
ai-first: true
source: codex
---

MC T3 recheck: FAIL, 0 High and 2 Medium remaining on d805669169fdcb095242e525f5f10c77843925f1. Verified HEAD and clean status before/after probes. No reviewer source changes or Cargo runs. Re-read the 49d42a..d805669 diff, new regression tests and actual evidence logs.

Remaining findings:

- Medium, KNOWN_KEY_CODES remains incomplete and regresses formerly valid keys (crates/turts_mod_api/src/input.rs:210-231). Fresh CLI rejects BrowserBack and WakeUp with exit 2 and `unknown physical key code`; both were accepted by 49d42a and are in installed bevy_input-0.20.0/src/keyboard.rs:570 and :614, plus winit. Convert now accepts exit 0, but its fix deleted all seven Browser* codes and WakeUp. A direct Bevy KeyCode enum comparison also finds Meta, Hyper, Turbo, Abort, Resume, Suspend, Again, Copy, Cut, Find, Open, Paste, Props, Select, Undo, Hiragana, Katakana and F25..F35 absent. Complete the intended physical-code vocabulary rather than special-casing only the first reported omission; preserve MetaLeft/MetaRight aliases where needed. Actual regression probes: known-browserback and known-wakeup.
- Medium, OrderRules classification still depends on exact raw substring spelling (crates/turts_mod_runtime/src/archive.rs:271-272, consumed at :100 and :116). The legal RON whitespace in `(queue_depth : 16, smart : [], typo: true)` makes the new classifier return false, so CLI exits 2 with `unknown field queue_depth (allowed: id, name, behaviour, ...)`, hiding the true unknown `typo` field and presenting OrderDef fields. The normal no-space overflow and typo cases are fixed. Classify from syntax/schema information, handling whitespace/comments and avoiding string/comment false matches. Actual regression probe: rule-whitespace-typo.

Prior items closed with actual current CLI observations:

- High references: seven undeclared-namespace probes (unit filter, widget order, selection priority, unit presentation, smart rule, nested Balance and Balance source) now exit 2 with referring package/file/field. Local unit/order/balance wrong-kind probes also exit 2. Existing resolver refuses unavailable declared dependencies; no MP2 cross-library linking scope is requested here.
- High cursor caps: widget and AssetsDef cursor dimension/hotspot probes each exit 2 with actionable fields.
- High reserved chords: canonical and permuted Ctrl+Shift+W both exit 2 with the same normalized diagnostic; duplicate Ctrl rejects 2.
- High primary images: real Image templates binding SelectionPrimary(Portrait) and SelectionPrimary(Icon), with a corresponding UnitPresentation, each pack exit 0. Bind(SelectionPrimary(Health)) rejects 2.
- High OGG duration: single 31s, chained 32s and chained mixed-rate 31s reject 2; mixed-rate 30s accepts 0. This exercises independent sample rates, not only the builder's same-rate regression.
- Medium action kinds: Item(Icon) rejects 2, Item(CancelAction) accepts 0. IconField/ActionField and validate_icon now constrain image/action expressions.
- Medium no-content derivation: engine_overrides iteration is removed; updated test asserts no hit for engine action `select` while retaining rush/launch/rocket hits. Read review-no-content-green.log: nine tests, EXIT_CODE=0. No separate reviewer compiler run.
- Medium guide warnings: guide 10px Text and 20x20 Button pack exit 0 and print both package/file/guide-field warnings.
- Medium OrderRules diagnostics partially fixed as described above; public order_rules no longer silently skips malformed entries and has accessor regressions.

Retained negative controls: unknown nested Value, missing icon, unknown widget and CancelProduction(Point) all reject 2. Baseline packs 0. A missing-icon scratch plant rejects 2, restoring that exact package's original order file packs 0 and reproduces the baseline archive hash. No candidate source plant was made.

Reproduction: `python3 /home/julien/Projects/TuRTS/target/mc-evidence/t3-review/recheck-d805669.py` invokes `target/mc-t3/debug/turts-pack <scratch-package>` for each case with subprocess.run and records actual return codes. Script itself exited 0. Full observations and archive hashes are in recheck-d805669-probes.log and recheck-d805669-probes.json. Observed binary SHA256 was 2b47e472582a41573dcf46b9b38cccdde217ad9d98870e250a3ecb9460f4c7af, differing from root's supplied d1850a4e...; root/builder notified for build provenance confirmation. Candidate HEAD remained d805669. Own scratch payloads are removed only after hashes are captured; script/logs remain reproducible and compact.

Evidence limitations: read explicit EXIT_CODE=101 and named semantic failures in review-reserved-red.log and review-chain-red.log, with corresponding green logs. review-hud3-semantic-red.log has EXIT_CODE=101 but only one failing cursor test caused by fixture interference; it does not demonstrate every new regression went red. Original 22-case historical red logs still lack numeric exit markers; numeric exits remain historically NOT TRACED. Read numeric EXIT_CODE=0 in review-runtime-all-green.log (120 tests), review-pack-bad-green.log (56), review-api-green.log (14), review-no-content-green.log (9) and review-workspace-green.log. None of these green receipts close the two confirmed Medium findings. Final corrected-candidate G-REF and the planned isolated instrument repeat/restored green receipt remain pending and builder-owned.
