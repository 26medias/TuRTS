---
type: report
date: 2026-10-30
tags: [turts, mc, review]
ai-first: true
source: codex
---

MC T3 second recheck: FAIL on 08b72a16c567165a3762a7e9bb84b82ff73077a2, with 1 High and 1 Medium remaining. Verified HEAD and clean status; all CLI probes used the immutable candidate-08b72a1/turts-pack snapshot. Its SHA256 matched 76ce441231e65915baa1c4f58655548493cef5292a75b61b4d4ec8bc53ff5b20 before and after probing. Read manifest.txt (clean source HEAD, cargo build -p turts_pack --bin turts-pack --locked, build_exit=0, fixture_tree_sha256 eda46fdba9e0e9f003b138999a3571d1676938224c5fc9d9b5b81f7c87c2d24f). No reviewer compiler invocation, source edit, vault edit, merge or push.

Remaining findings:

- High: OGG cap guard remains bypassable through same-serial metadata after EOS. crates/turts_mod_runtime/src/asset_headers.rs:50-51 overwrites an already observed final granule; :40-41 groups pages by serial without tracking EOS or stream reuse. The immutable CLI rejects a header-valid 31s OGG with exit 2, but accepts that exact stream followed by a same-serial EOS page with granule 0 and sequence 2, exit 0. It also accepts a 31s stream followed by a second 1s stream reusing the same serial, exit 0. These are malformed stream continuations, generated with correct page CRCs; the header validator should refuse them rather than let later metadata erase a proven over-cap duration. This finding does not require decoder validation or new formats: reject invalid continuation/reuse/descending metadata, or otherwise ensure known duration cannot be understated. Actual probes: ogg-single31, ogg-descending-after-eos and ogg-reused-serial in recheck-08b72a1-probes.log/json. The distinct-serial chained fix works, but this High header-cap item is not fully closed.
- Medium: OrderRules/OrderDef diagnostic selection still misclassifies legal RON comments and raw strings. crates/turts_mod_runtime/src/archive.rs:320-325 skips only whitespace between a field identifier and colon, so `(queue_depth /* depth */ : 16, smart /* rules */ : [], typo: true)` reports `unknown field queue_depth (allowed: id, name, behaviour,...)`, exit 2, instead of the actual rules field typo. At :293-304 the classifier handles ordinary strings but treats a quote inside a raw description as its end; an otherwise valid OrderDef description `r#"queue_depth: \" smart: "#` plus `typo: true` reports `unknown field id (allowed: queue_depth, smart)`, exit 2. Removing only `typo: true` makes both exact packages pack exit 0, confirming their comment/raw-string syntax is valid. Use parsed RON field information or a lexer that correctly handles the RON grammar rather than approximate scanning. Actual probes: rule-comments-typo, order-raw-description and their valid-control observations.

Closed since d805669:

- KNOWN_KEY_CODES now includes all 194 plain variants of installed Bevy 0.20 KeyCode plus MetaLeft/MetaRight aliases; direct source comparison found no omissions. Fresh CLI accepts BrowserBack, WakeUp, Convert, KanaMode, Lang1, Hiragana, Katakana, F35, MetaLeft and SuperLeft (each exit 0), while KeyFlorp rejects exit 2.
- Whitespace-only malformed OrderRules now reports the actual typo field and correct queue_depth/smart allowed fields, exit 2. Ordinary quoted strings containing field-like words preserve OrderDef diagnostics. The comments/raw-string cases above keep the broader MD18 item open.

Prior closure checks repeated against the immutable snapshot:

- Undeclared namespace refs in order filters, widget orders, selection priority, unit presentation, smart rules, nested Balance and Balance sources reject exit 2. Local wrong-kind unit/order/balance refs reject 2. Existing dependency resolver/MC single-package behavior remains the scope; no MP2 linker expansion requested.
- Cursor dimensions and hotspots reject 2 through both widget and asset routes.
- Canonical/permuted reserved chords and duplicate modifiers reject 2.
- Primary portrait/icon Image bindings pack 0 with corresponding UnitPresentation. Non-image Health binding rejects 2.
- Item(Icon) button action rejects 2; Item(CancelAction) packs 0.
- Single31, distinct-serial chained32, and mixed-rate chained31 OGG reject 2. Mixed-rate chained30 packs 0.
- Guide 10px Text and 20x20 Button pack 0 with both expected package/file/guide-field warnings.
- Unknown nested Value, missing asset, unknown widget and invalid CancelProduction(Point) still reject 2. Baseline packs 0. The same missing-icon scratch package, restored from its original order file after the rejecting probe, packs 0 and reproduces baseline archive bytes/hash.
- No-content engine-override exclusion remains present, as does the regression asserting that select produces no token hit while mod action/order/asset IDs remain covered. This is reviewed source plus the prior numeric nine-test green receipt; no reviewer Cargo run.

Command: `python3 /home/julien/Projects/TuRTS/target/mc-evidence/t3-review/recheck-08b72a1.py` (orchestrator exit 0) runs the immutable binary on scratch package copies, recording each actual child exit in recheck-08b72a1-probes.log/json. Additional valid syntax controls are recorded in recheck-08b72a1-controls.json. The script hard-checks the snapshot SHA before execution. Source and snapshot remained unchanged. Own scratch payloads can be recreated from the script and are removed after per-file hash capture; snapshots are root-owned and untouched.

Read builder focused receipts: review-keycode-final.log, review-rules-whitespace-final.log and review-rules-quoted-final.log each contain named passing tests and EXIT_CODE=0. These narrower tests miss the concrete cases above. Preserve historical limitations from the earlier reports: the original 22 HUD3 first-red files lack numeric exit markers, while the reserved/chain red receipts have actual101. The isolated signature-stub instrument repeat/restored green receipt and final G-REF29 remain pending; no all-gates acceptance is claimed.
