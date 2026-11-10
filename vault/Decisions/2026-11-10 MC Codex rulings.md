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


## C246: Keep the full failure and verify the existing cooldown predicates directly

Conflict: the effective-zero cooldown mutant ec1e5d7b makes both actual headed ALR1 runs fail, but the first full oracle stops at its earlier camera-centre assertion before the cooldown-spacing assertion. An earlier row failure alone does not prove the intended cooldown fault. The second full run reaches and fails the exact existing cooldown-spacing assertion.

Choice: preserve the distinct actual full-row errors without relabelling them. RUN79 extracts the EXACT unchanged original spacing loop and five-part suppression predicate from feedback-oracle.js SHA9149e42d and evaluates them against both actual captured first/late checkpoints. The original observers, row, oracle and acceptance criteria stay unchanged; no browser repeat is needed. Both original predicates reject both real captures: same unit24 at authoritative20Hz emits at72,74,...142 (36events) and73,75,...141 (35events), a2tick/0.1s gap instead of20ticks/1s. The pure Node controls exit0 because they explicitly require the two named semantic throws; they are not reported as new browser runs or as process exit1. The actual full browser pair exits1 on both attempts.

Root verifies actual headed profiles, immutable source and actual full-row log errors, then restores the exact committed feedback.rs preimage and runs the UNCHANGED complete ALR1 pair. All camera, audio, ping, suppression and spacing checks must pass on restoration. The first camera failure does not replace those checks. C203 minimap pixels still belong to Task14/24.

Evidence: root-t15-cooldown-mutant-01 outerrawb190d129; t15-root/runtime/cooldown-mutant-01/runs/run-20261110T052632Z-eHYtAC and run-20261110T053003Z-5Xh1g7; t15-reviewer/run79-source/plant-06-cooldown-direct-audit/ contains exact original guard fragments, programs, actual input hashes and results for both. Root plant-06-cooldown-root-progress.json records distinct full-row failures and pending full restoration. This is the brief section4 instrument/evidence handling decision. Alternative not taken: infer cadence from camera failure, edit or weaken the full oracle, change the minimum, fabricate a named failure, repeat an already conclusive mutant pair, or retire a plant.80/170 runs;17/25 lanes complete.


## C247: Report the narrow late debug-wiring exception to plan R5

Conflict: plan R5 at118 says no task edits debug.rs after Task9. The later actual ALR1/AUD1 work needs authoritative tick rate, lossless full-u64 identity and real bounded-rejoin state. A rounded JavaScript numeric ID or an omitted fixture parameter cannot prove those design13.2/14 observations. C210/C225/C229 already chose the underlying instruments and advanced designed persistence/rejoin subsets; this note reports the shared-file ownership conflict explicitly.

Choice: retain only the reviewed read-only wiring in debug.rs: actual Replica.tick_hz (33), actual PlayerId decimal string (125), real RejoinController state (275), and corresponding native report metadata (399/461). Preserve the old numeric field and replicated_world/wire DTOs. The owning typed resources still supply every value; no test-manufactured feed, changed K call, slot rule or player action. Design-of-record observability wins over the plan's file-ownership restriction. Task16 reuses the advanced subset and still owns its complete rows.

Root read the exact debug diff and final177-path delta; RUN79 final review covers the actual code with0High/0Medium. Tests include lossless u64::MAX capture/rebuild/report checks, actual authoritative timing, exact server decimal ID/slot comparisons and current headed row pairs. Evidence: debug.rs committed52ecf3c; t15-reviewer/run79-final/review.json and root final-acceptance-audit.json. No source change is made by this ruling. Alternative not taken: round an identity, use an absent or invented rate, weaken real rejoin evidence, change a wire DTO, or hide the ownership deviation.80/170 runs;17/25 lanes complete before merge.


## C248: Refresh learned HUD2 image-search seeds for the authored camera

Conflict: both actual headed HUD2 attempts stop at the required screenshot safeguard. The old learned seed for the lab is657,307 and turret658,418 (tests/browser/hud2-driver.js:21-22). Current saved images show those units outside the12px local search neighbourhood after the authored camera changes. RUN80 classifies M9 as Medium. The design requires actual image-driven selection; a stale learned seed cannot complete the step.

Choice: repair the learned search seeds from the actual current screenshots. Retain a fresh local slot-colour/shape PNG search before each physical click and retain independent C84 oracle checks. The oracle's projected unit centre may diagnose the mismatch; it must not become a semantic read or a blind driver click. This follows C226 and the brief section4 instrument-detail and Medium-repair rights. All HUD2 acceptance checks and two headed attempts remain required. No production map, unit, order, camera setting, key, or acceptance row changes. The adjacent H8 widget hierarchy finding is a mandatory implementation repair, not a design change; it needs a real hierarchy red/green and independent recheck.

Evidence: t17-review/run80-browser-review/review.md and receipt.json; actual first batch t17-builder/run81-browser-green (raw SHA db30e045b464443796c1c1dd14173756bf8effd52ebe8c9b29bc55eb8176eca2). Root verified all six profiles are headed, source unchanged, HUD2/SEL1 each two failures and Muster two passes. Alternative not taken: disable the pixel safeguard, widen it to choose unrelated units, move the game camera to match old constants, change rows, or stop for a routine instrument repair.81/170 runs;18/25 lanes complete.


## C249: Check the visible ownership-appropriate HUD name row

Conflict: after the required H8 visibility repair, hidden foreign-info children correctly remain hidden. hud2-oracle.js:73-76 currently finds the first row with id info_name OR enemy_name, then checks visibility. Snapshot IDs sort enemy_name first, so an own selection can fail on an irrelevant hidden row even while its correct name is rendered. RUN80 found this deterministic Medium instrument defect during the active batch; no runtime pass is inferred.

Choice: use actual ownership to select the expected name row, and require that row to be visible with its actual text and pixels where required. A visible-row filter is also acceptable if all actual ownership and selected-unit identity/name/count checks remain. Preserve the fresh physical PNG click, replica owner identity, rendered unit target and public-health checks. Change source only after the active batch ends; retain any actual failure and repeat repaired HUD2. This follows brief section4 instrument and Medium-finding rights. No row, player behavior, key, HUD content or acceptance threshold changes.

Evidence: tests/browser/hud2-oracle.js:58-90; RUN80 source report and first repaired batch t17-builder/run81-browser-green-after-repair. Alternative not taken: make hidden panels visible again, accept any hidden name, remove rendered-name checks, or alter the acceptance row.81/170 runs;18/25 complete.


## C250: Measure the actual hover outline separately from selected rings

Conflict: both actual repaired SEL1 attempts pass ground clear and fail the hover pixel check with0changed slot-colour samples. RUN80 read the actual screenshot and source: hover is a smaller0.96×footprint ring at height0.032 with a neutral underlay at0.028 (crates/turts_client/src/visuals.rs:833-894). The oracle reuses selected-ring full-radius/height0.025 orange-change sampling. It misses the smaller hover annulus and rejects its intended neutral gaps. Actual selection and RingsShown correctly stay empty under C219.

Choice: use a distinct hover pixel check at the actual rendered hover annulus. Require actual coloured segments and visible neutral gaps/change from the unhovered reference, with meaningful pure negative controls on the same captured PNGs. Retain selected-ring metadata/slot-colour/shape tests unchanged. Retain actual delayed tooltip, range, real physical input and rendered text checks. Do not add hover metadata to RingsShown or change renderer geometry to satisfy a wrong sampler. This is brief section4 instrument-detail/Medium repair under existing design.

Evidence: current second batch t17-builder/run81-browser-green-after-repair, SEL1 run-20261110T065825Z-rVjLNM. RUN80 offline existing visibleForeignGaps at radius1.2×0.96 accepts the actual hover PNG and rejects its actual unhovered reference; it also finds coloured segments. Preserve exact controls and source recheck, then rerun actual SEL1 twice. Alternative not taken: waive hover pixels, fabricate metadata, sample any unrelated changed pixel, change product geometry, or stop for an instrument mismatch.81/170 runs;18/25 complete.


## C251: Compare foreign health to the same selected unit and checkpoint

Conflict: HUD2 now reaches foreign public-health checks and fails an instrument constant. hud2-oracle.js:144-151 finds the first forge:drone and requires current100 plus literal Health100/100. Independent RUN80 checkpoint readback clarifies the exact identity: candidate02 selects drone9 and correctly shows90/100. Candidate03 selects drone10 and correctly shows100/100. The oracle retains the latter checkpoint, then incorrectly reads the first drone9, now60/100, instead of selected drone10. The first RUN81 description mixed those identities; its original report is preserved, and this correction governs the recorded evidence. The design requires actual public health, not an undamaged unit. Production map has both drones at mods/forge/maps/proving_ground.ron:13-14.

Choice: resolve the unit by the actual selection ID in that checkpoint. Require actual foreign owner/content, valid public current/max health, exact corresponding rendered text and actual pixels from the same checkpoint. Retain selected-row public-health checks, hidden owner command card and private inventory/ammo restrictions. Do not change combat, refill health, remove damage checks or compare a different drone. Change source after the active batch; retain the real mismatched-ID/60-versus100 failure and repeat actual HUD2 twice. This is brief section4 instrument-detail repair; no player-visible or acceptance-row change.

Evidence: t17-builder/run81-browser-green-final actual HUD2 captures and RUN81/RUN80 same-checkpoint review; oracle lines144-151. Alternative not taken: force full health in content, choose the other healthy drone, accept arbitrary text, disable real combat, or waive pixel checks.81/170 runs;18/25 complete.


## C252: Keep unaffected Task17 row greens across a HUD2-only oracle repair

Conflict: the completed headed batch has SEL1 and HUD2-MUSTER two actual passes each, while HUD2 alone fails the C251 identity lookup. Repeating every row after that bounded oracle repair would duplicate unaffected runtime work. The CEO approved grouping and removing redundant checks while keeping quality.

Choice: retain actual original SEL1 two-pass identities if its driver/oracle and all engine/content/harness inputs stay byte-identical. Repeat HUD2 twice on the repaired exact oracle. For Muster, its oracle shares the changed HUD2 file: require an explicit exact unchanged early Muster execution-path/source binding, or rerun its cheap pair if that proof is uncertain. Do not relabel original runs with a new commit or skip any assertion, gate, plant or reviewer check. A later engine/content change invalidates this scoped reuse and requires affected rows to run again. This is the C239 pattern applied to Task17 acceptance coverage and brief section4 instrument/order rights.

Evidence: t17-builder/run81-browser-green-final, actual frozen source and six headed completion records; SEL1 both0, Muster both0, HUD2 both1 at C251. Root verifies raw hashes/source binding before acceptance. Alternative not taken: repeat a complete expensive matrix after each isolated oracle repair, claim failed HUD2 green, relabel evidence, or waive affected cases.81/170 runs;18/25 complete.


## C253: Check retained real refusal playback at its exact physical-input frame

Conflict: both actual HUD2 C251 attempts reach the refusal sound assertion and fail because hud2-oracle.js:180-183 checks only audio.started at a later screenshot. audio.rs:349 clears that list every frame; it reports currently live sinks. The short actual error OGG has finished by the screenshot. RUN80 independently traced the physical right click input79/pointer-up seq31 to retained core frames398 and389 respectively. Those frames select turret7, show Shell Turret: Cannot move and emit no order. Each actual real-sink onset has configured forge:forge_effect_error, effects category, play_id6, entity12884901142, gain1 and request_frame_index exactly matching that causal frame. Pre-click checkpoint78 has no such error onset; checkpoint81 retains it, with running audio contexts. Current started/current_sinks truthfully show a later alert.

Choice: compare the retained actual sink observations before and after the exact physical click. Require a newly observed play identity, configured UnitPresentation error sound, effects category, positive gain and request frame EXACTLY equal to the uniquely retained pointer-up frame. Keep the actual notice, selected turret and no-world-order assertions. Do not count feedback intent, old/unrelated error, merely a later frame, an active alert, or a fabricated audible claim. RUN80 source-rechecks the bounded change before a fresh actual headed HUD2 pair. No product code, audio duration, row or player behavior changes; this is brief section4 instrument-detail repair.

Evidence: t17-builder/run81-browser-hud2-c251 actual two attempts; RUN80 same-checkpoint/input/sink causality review. Original failed runs and started/current_sinks are preserved. Alternative not taken: lengthen the OGG for the test, accept intent as playback, repeat unrelated rows, remove the sound check or claim silence as expected.81/170 runs;18/25 complete.
