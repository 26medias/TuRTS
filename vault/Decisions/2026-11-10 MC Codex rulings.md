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


## C244: Use one verified runtime launcher and move its lease to root

Conflict: manual browser launch assembly repeatedly omitted the durable output path or headed flags. The first marker pair ran headless on parent display:1 and failed pixel readiness; it is setup evidence, not the marker red. Repeated setup runs cost time without acceptance coverage. The approved run_mc_rows.py already enforces durable output, all profile flags and exact modified-path grants for committed-file mutants.

Choice: root takes the sole Task15 compiler, browser and lane-source lease after RUN78's active canonical marker pair ends. RUN78 confirms no owned runtime remains and prepares the other five exact mutant files/patches outside Git only. RUN79 source-reviews those patches and saved artifacts; its separate voice-plant runtime lease still comes later. No new agent run, scope reduction or model change. Root runs all remaining runtime commands through outer dedicated xvfb-run, exec_in_lane.py with the assigned target/audio/immutable source receipts, then run_mc_rows.py with exact full HEAD, target/mc-t15 and explicit allowed modified paths. The helper always selects headed mode and a durable output directory; actual profile files and completion records are still checked.

The already completed canonical marker red is valid: both actual profiles are headless:false on dedicated display:110, both fail the named first-release-frame marker assertion, source mutant b561db3a unchanged. Root verifies those actual logs, restores the committed visuals.rs preimage exactly and confirms clean status before the restored pair. Do not rerun that red solely for the lease change. Preserve the two earlier headless attempts separately with no marker-acceptance credit. All remaining plants, exact restorations, same greens, current row coverage and independent final voice plant remain required.

Evidence: t15-builder/run78/plants/ord7-marker/mutant-headed/receipt.json and its two actual run profiles/completions; t15-root/plant-04-marker-root-progress.json; RUN78 lease handoff. This is the brief section4 cache/runtime layout and verification handling decision. Alternative not taken: start concurrent runtimes, trust a headed label without the actual profile, count pixel setup failures as marker reds, leave evidence in a retired lane, or waive a plant.80/170 runs;17/25 lanes complete.


## C245: Delay integration before command drain for the singleton fast path

Conflict: the first source-only order-delay proposal swaps Orders and Movement within FixedUpdate. RUN79 found that the ORD7 singleton Move path already calls apply_legacy_move_to during FixedPreUpdate Commands (orders.rs:2826-2860), which inserts MoveTarget before either FixedUpdate set. Actual integration reads that target directly. The proposed swap could leave first movement in the accepted tick, so it is not a reliable one-tick fault. The rejected draft was not run.

Choice: use a temporary plugin.rs mutation with the FixedPreUpdate chain Advance, Movement, Commands; remove Movement from the FixedUpdate chain; register the single integrate_movement system in FixedPreUpdate instead of FixedUpdate. Preserve tick advance, command/result tick, client marker and audio. This puts actual integration before the newly drained target, including the singleton path; a new move can first change position on the next tick. The ORD7 oracle must observe the real first-movement tick differing from the actual accepted-result tick, then pass after exact source restoration.

RUN78 prepared exact mutant3abf973a4062e62183407e6837a5da8ca88e9e8b88169ca42e026ba4fb6b1452 and patch9fa41b6f4194e76c70457b4e5b5e3f35e77ecbd04b51ccdba47bcc9933249df2 from committed preimageb9d35a69. Root read the singleton and scheduling paths. RUN79 source review approves the one integration registration, ordering and unchanged reports with0High/0Medium. Receipt: t15-reviewer/run79-source/c245-order-delay-source-pass/review.json. Actual red and exact-restored green remain required; no pass is claimed from source review.

This is the brief section4 actual-mechanism/instrument and review-finding repair, refining the C241 plant target without changing the product, the acceptance row or the plant count. Alternative not taken: run a schedule swap that misses the tested fast path, delay only client arrival, edit result ticks or logs, or waive the timing fault.80/170 runs;17/25 lanes complete.

Metadata correction: the actual complete wrapper source maps and C242 receipt have1,515 entries. Earlier prose said1,514. C242 checks every actual entry; no source or guard was omitted. Correct the prose only; all source hashes, raw logs, outcomes and identities stay unchanged.
