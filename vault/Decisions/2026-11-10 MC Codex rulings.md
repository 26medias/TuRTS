---
type: decision
date: 2026-11-10
tags: [turts, mc, codex, ruling]
---

# MC Codex rulings

## C243: Reject a whole list before application, not by editing its result

Conflict: the first executed atomic plant has exact saved hash21fc9d2c, but its only change clears outcomes.accepted at the end of apply_order. Root and RUN79 read the exact executed bytes. Side effects already occurred: apply_unit_order near orders.rs:2900 and pending movement cancellation/queueing at2986-2988 precede the result at3015. The real 0-versus-290 failure therefore proves result corruption, not the required whole-list rejection. Earlier progress counted that runtime cycle too early.

Choice: preserve every original log, screenshot, source hash and restored green as a reporting-corruption control. Remove its credit for the required atomic plant. Do not retire the plant or change any acceptance row. Use a generic read-only validation pass after the unit/entity map and BEFORE any application, pending queue, group or inventory side effect. Inspect actual subject existence, owner, type and Move capability; if any subject fails, return an error with none accepted and all input subjects refused. No fixture IDs, counts or content names are permitted. The 40 valid browser subjects must still pass under the same exact mutant.

RUN78 prepared atomic-preflight-candidate.patch (41006c9a) and exact candidate mutant a2834480 from committed preimage daa197e0. Root read its42-line insertion before outcomes/application; RUN79 source approval is required before execution. Then run fresh compiled native semantic RED, both headed valid-unit browser controls under that exact mutant, exact restoration, and the same native and current browser greens. Keep native/browser mutant checks together before a single restore. The old reporting-corruption outputs cannot substitute. No repeat of normal gates is needed because this is a temporary fault and the committed product candidate52ecf3c is unchanged.

Evidence: t15-reviewer/run79-source/plant-01-atomic-audit/review.json contains the causal rejection and exact executed bytes; prior raw-only review is preserved. The replacement patch/bytes are in t15-builder/run78/plants/ord2-atomic/. Root corrects plant-01-atomic-root-audit.json to uncredited for the required fault. This is a brief section4 instrument/mechanism and review-finding repair. Alternative not taken: accept a result-count failure as proof of atomic execution, hide the wrong fault, waive or retire the named plant, or alter the product.80/170 runs;17/25 lanes complete.
