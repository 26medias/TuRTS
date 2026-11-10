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


## C254: Reject disabled research pixels before the physical click

Conflict: full C253 offline oracle replay of both real C251 captures reaches the actual queue check and fails. The driver exits its enable-wait loop on an actually disabled button and clicks it; no job starts. hud2-driver.js:241-244 accepts pixels within Euclidean35 of enabled RGB33,66,77. The actual disabled pixel51,51,51 is exactly35 away (18²+15²+26²=1225; the earlier approximate report is corrected), so it falsely passes. The saved screenshot/checkpoint confirms enabled=false,0/10Shell and an empty queue. This is a pixel-predicate instrument error; no product failure is inferred.

Choice: calibrate the current-PNG predicate to separate enabled background from disabled grey, using a tighter colour distance and explicit distance margin from disabled. Retain pixel-only driver access, screenshot-before-click proof, the actual simulation wait cap and physical click. Independently replay both saved disabled images: original helper falsely accepts, repaired helper must reject with named failure. Then actual headed runs must wait for real visible enablement, and the oracle must independently prove same-checkpoint availability and the resulting real research job/seconds/progress/cancel. Synthetic colour controls do not replace that actual green. No Forge content/resources, simulation, acceptance row or player action changes.

Evidence: t17-builder/run81-c253-offline-replay.json and both original C251 captured enabled-labelled PNGs (their historical labels are preserved but do not prove availability). RUN80 source/pure image recheck precedes the actual rerun. Alternative not taken: accept a disabled click, inject inventory, infer a queue from intent, weaken queue checks or add semantic reads to the driver.81/170 runs;18/25 complete.


## C255: Find actual command-button pixels inside the authored list viewport

Conflict: both actual C254 HUD2 attempts pass real research/start/progress/cancel and fail the physical tooltip-target association. The first purported target1256,492 lies below the actual command list [1001,255,264,228]. Actual drill buttons [1001,255,264,28] and [1001,287,264,28] have black interiors because the preserved Forge order icons are opaque1×1black PNG backgrounds. The driver expects teal/grey fills, rejects real buttons and falsely accepts footer43,44,47 as disabled grey. The oracle correctly rejects a hover outside every actual order button.

Choice: calibrate current-image detection to actual black button rectangles plus visible label glyphs, within authored list bounds. Persist actual screenshot observations before physical hover. Controls must accept actual button rows and reject the captured blank/outside-list target. Preserve original icons, effective key labels, applicable-order coverage and independent exact button/action/hitbox/tooltip/key/pixel oracle checks. No semantic driver reads, hidden-UI acceptance or relaxed association. RUN80 source/pure image recheck precedes fresh actual HUD2. This is brief section4 instrument repair, not a missing-tooltip product claim.

Evidence: t17-builder/run81-browser-hud2-c254 captures; RUN80 same-screenshot rect/action/icon/pixel readback; hud2-driver.js captureOrderTooltips and hud2-oracle.js:425-450. Alternative not taken: change preserved artwork, blindly hover footer/void, remove point-in-button checks, count unrendered tooltips or change the row.81/170 runs;18/25 complete.

## C256: Sample the actual same-job progress widget rectangle

Conflict: RUN80 verifies the real queue screenshot has Hardened Shells job27,200ticks,4elapsed,9.8s remaining and actual progress rect24,650,164,12. The oracle's fixed22,648 sample overlaps it and passes, but is not precisely bound to that job's rendered widget. The cancel click229,658 is inside the actual button195,646,72,28 and next checkpoint has no jobs.

Choice: resolve the visible progress row for the same actual lab job from that checkpoint and use its computed rect for pixel sampling. Retain actual job duration/elapsed/seconds/cancel evidence; replay this bounded oracle-only change against real captured PNGs before another live run. No bar layout, job timing, content or acceptance changes. This is a Medium instrument precision repair under brief4.

Evidence: actual C254 queue captures and RUN80 queue-state/layout readback. Alternative not taken: keep a nearby unbound sample, accept arbitrary coloured pixels, or change the progress widget to fit the old constant.81/170 runs;18/25 complete.


## C257: Treat an explicit zero stock record as known empty ammo

Conflict: both actual C255 HUD2 attempts fail the static items.length===0 ammo assertion. RUN80 independently verifies selected owned forge:turret7, local/owner slot0, known main inventory unit7 with explicit forge:shell on_hand0/reserved0, and visible Shells:0 at19,399,248,20. An explicit zero record and no Shell record in a known empty inventory both represent zero available ammo. Neither is unknown/private state. The existing UI is correct.

Choice: bind unit and known main inventory to actual selection[0], required turret content and local owner. Require an actual inventory array and nonnegative integer stock quantities if a Shell entry exists. Compute available=max0(on_hand-reserved), using0 for an absent Shell entry ONLY in a known inventory. Preserve required available===0, exact visible Shells:0 and actual pixels. Nonzero ammo must not satisfy the empty-ammo step; unknown/private inventory must not be treated as known empty. RUN80 source recheck, complete offline oracle replay on both real C255 captures, then fresh actual HUD2 pair remain required. This is brief4 instrument repair, with no player/data/content/acceptance change.

Evidence: t17-review/run80-browser-review/turret-zero-stock-causality.json; actual t17-builder/run81-browser-hud2-c255 captures; hud2-oracle.js known ammo assertion. Alternative not taken: erase valid zero stock rows in the engine, accept any ammo count, infer hidden inventory, weaken the zero requirement or remove the pixel check.81/170 runs;18/25 complete.


## C258: Freeze formatted source before one final grouped row batch

Conflict: actual cargo fmt command exits0 but its enforcing wrapper correctly exits2 because formatting changes source. The following fmt check exits0 with source unchanged. Formatter touched only widgets/mod.rs, widgets/sources.rs and widgets/snapshot.rs (layout/import/test wrapping); previous SEL1/Muster passes preserve their original source identities. C252 reuse requires byte-identical engine inputs, so that condition no longer holds. The corrected full HUD2 oracle separately passes both complete C255 captures offline; their original live exits remain1.

Choice: run precommit focused strict Clippy now, repair any issues, format/check and independently review exact final bytes before expensive runtime. Then execute one grouped final batch for HUD2, SEL1 and HUD2-MUSTER, two actual headed attempts each, on frozen formatted source. This is cheaper and clearer than reconstructing formatter-equivalence proof for the old short pairs. Preserve every old run and its identity. Commit after semantic greens; full workspace gates run only after commit, then all causal plants. No redundant precommit full workspace test, no new agent run and no acceptance waiver. This is brief4 task/instrument ordering and the CEO's approved redundant-work reduction.

Evidence: RUN81 formatter receipts and actual three-path diff, subsequent stable fmt check; run81-c257-offline-replay.json actual saved-capture oracle passes2/2. Alternative not taken: claim unchanged byte identities after formatting, relabel old runs as final-source acceptance, reconstruct an expensive special equivalence proof, or discover avoidable lint repairs after the final browser batch.81/170 runs;18/25 complete.


## C259: Align test-only guide expectations with inherited visibility

Conflict: committed3d22996 workspace tests fail the old core_adapter::adapter_tests::guide_button_and_f1_both_toggle_once_through_core_shell_action enum expectation. H8 correctly makes predicate-true authored widgets Inherited, and actual C258 browser guide close/F1 checks plus the meaningful native hierarchy regression passed. The old test expects unconditional Visible in its initial, reopened and adapter-early-return states. The complete adapter_tests module is #[cfg(test)] at core_adapter.rs:1074.

Choice: update only those three stale test expectations to Inherited. Preserve hidden state, button/F1 toggle-once effects and entity identity assertions. Require exact focused native green, fmt check, independent bounded review, test-only follow-up commit and all required committed gates rerun. Keep the original3d22996 failing workspace proof as failed. Prove the production prefix and every non-test/browser input byte-identical to final six headed passes; carry their original run/head/status identities rather than rerunning unchanged browser code. A runtime change would invalidate this scoped reuse. This is brief4 instrument/test-detail repair; no new agent run or player behavior/acceptance change.

Evidence: crates/turts_client/src/core_adapter.rs:1074,1788-1789,1841-1843,1861-1863; actual committed workspace failure; C258 six actual0/headed records and t17-root/candidate-source-binding.json (1611input files bound to3d22996). Alternative not taken: restore the real hidden-parent bug to satisfy Visible, waive the failed native test, relabel the failed gate green, or repeat six unchanged runtime rows for three cfg(test)-only assertions.81/170 runs;18/25 complete.

C259 count correction: the complete function has three open-state enum assertions; the initial bounded read saw only two. All three actual hunks are inspected and approved. Focused exact test runs1/1green after all three are updated. Original successive failures remain preserved.


## C260: Overlap hosted CI with the next wave's local work

Conflict: Wave 7 is built, merged, locally re-gated and pushed at2c113836d42944eaf32a45a32fa0decdf7da029a. The required hosted run has been read: CI42789795527 has agents success; check and browser are still running. Waiting idle for cold hosted tool installation would delay local work. The execution contract requires push/read after a wave; it does not require a terminal result before the next local task starts. The CEO approved reducing redundant waits while keeping quality.

Choice: record the real run ID and actual current results, then start Wave 8 Task14 on its own branch/cache after all Needs are merged. Root continues to read all three hosted jobs and their logs to terminal state. No new task merges until those results are classified and any new regression is resolved. Only the two already-approved panel-migration failures may stay in the O13 Task12–21 window, owner21; a new failure is not assumed known. Preserve every task's normal gates, actual acceptance, plants, review and post-merge gates. Keep one local compiler/cache owner. This is pipeline/task ordering under brief4 and the execution contract, not a CI pass claim or waiver.

Evidence: plan execution contract lines46-48; push2c11383; actual GitHub run https://github.com/26medias/TuRTS/actions/runs/42789795527; jobs129740016726success,129740016986inprogress,129740017190inprogress. Alternative not taken: mark running jobs green, ignore terminal logs, overlap local compilers, merge past an unclassified failure, or idle through hosted setup. Build runs81/170 before new dispatch registration.


## C261: Assert the completion-observing Orders phase without an extra idle tick

Conflict: ORD3's short acceptance text says the next queued order starts in the same tick; Task14's plan says the tick after landing. Design4.5 lines429-432 gives the exact reading: Movement of tick t lands; Orders of t+1 observes completion, pops and starts the next order in that phase. Existing orders.rs test queued_order_starts_in_the_tick_after_landing at939 follows this schedule.

Choice: assert physical landing at Movement t and promotion/start in the immediately following completion-observing Orders t+1, with no extra idle Orders tick. Treat same-tick wording as the same phase that observes completion, not an instruction to reorder Orders after Movement or start twice. Preserve exact design phase order and player-visible no-idle behavior. Native and pixel instruments must record actual ticks/phases; runtime remains NOT TRACED until run. This is brief4 instrument/timing interpretation using the detailed design rule; no player-visible change, K change or acceptance waiver.

Evidence: design4.5:429-432; ORD3:1238; plan Task14:632; sim/plugin.rs:97-105; sim/tests/orders.rs:939 onward. Alternative not taken: impose contradictory same-physical-Movement-tick execution, add t+2 idle delay, rename an unrun check as passed, or ask CEO for an instrument conflict delegated by the brief.

## C262: Put the native alert-ping presentation test in the client layer

Conflict: Task14's four named baseline requirements include ping_shows_for_an_alert_ping, but the pure interaction core only owns camera/order minimap input and must not depend on Bevy (K1). Current baseline honestly has three failing core behaviors plus a passing unsupported/nonfinite guard. That guard cannot substitute for the planned ping requirement. The actual ping producer already lives in client/feedback.rs:369-378 and the empty minimap client plugin owns presentation.

Choice: keep the truthful 3-red/1-green core baseline and add the planned compiled semantic ping baseline at the client minimap presentation seam before implementing it, using a signature stub if needed. Its green must exercise the same presentation path consumed by the renderer; it is not proof of actual pixels. MM1 still must prove real slot1 damage-alert ping pixels in two windows under C203. Add asymmetric/non-square mapping cases so axis swaps cannot hide behind equal coordinates. Do not add Bevy to turts_interaction, fabricate a fourth red, or call metadata-only ping counters rendered acceptance. This is brief4 instrument-file ownership and nearest-design-layer choice.

Evidence: plan Task14 Step1 four names; core minimap.rs no-op signature; client/minimap.rs empty plugin; feedback.rs actual producer; RUN83 I1-I3 source review. No design/row/key/content changes.


## C263: Repair full MM1 instruments before qualified runtime evidence

Conflict: RUN83 finds High false positives in the draft MM1 oracle. A ping colour threshold of80 includes slot1 orange (distance51.478), and projected motion of an arbitrary moving unit cannot prove camera motion while combat units chase. Drag, right-click smart order, slot-colour unit dots and camera-frame clauses are also not fully asserted. No headed MM1 baseline has run; no pass is credited.

Choice: require actual same-window camera pose/matrix or a fixed static witness, tied independently to the physical minimap click and drag targets. Do not count moving-unit projection as camera proof. Bind a ping to the real slot1 damage event/unit/tick/world location and the actual minimap rect, then require its actual pixels/shape/temporal change; orange unit dots and a no-ping reference must fail controls. Use thresholds that distinguish the actual configured style, not an arbitrary broad yellow match. Implement all MM1 clauses: units/slot colours, camera frame, left click, drag, right smart command and two-window damage-ping pixels. Driver remains screenshot/io-only; semantic/geometry checks stay in the oracle. Repair and independently source/control-review before any qualified headed baseline or green. This is High instrument repair, not a changed row or renderer style.

Evidence: t14-review/run83-source reports; actual draft colour distance; native core3red/1green and client ping0vs1red receipts. Alternative not taken: accept incidental motion/orange pixels, drop MM1 clauses, fabricate camera/ping feeds or count a draft as full acceptance.

## C264: Minimap right click cancels active explicit target mode

Conflict: the new minimap handler calls smart command while leaving pending_order active. Design4.4 says right click or Esc cancels target mode; generic pointer Right already takes/cancels at interaction/lib.rs:286-297. Minimap right-click smart behavior describes the normal non-targeting path, with no explicit exception to cancellation.

Choice: reuse the same cancel/no-Send/default-cursor rule when minimap Right arrives during active explicit target mode. The next ordinary minimap Right, after cancellation, sends its smart command. Add a meaningful native regression for that state transition and preserve ordinary right-smart behavior. No new key, no new player step and no altered acceptance row. This follows the design's nearest existing input mechanism and brief4 instrument/mechanism authority.

Evidence: design4.4 and7; generic interaction Right branch; current handler pending_order source finding. Alternative not taken: issue a different world order and retain stale target mode, add a special new key, or modify design.

## C265: Preserve the draft and run a genuine original-behavior MM1 baseline

Conflict: native core3red/1green was recorded before implementation, but the core handler was then drafted before the headed MM1 baseline. The browser oracle needed C263 repair and no headed receipt exists. The client plugin is still empty and the C262 ping presentation helper is still a compiling stub. The missing browser red must not be invented or relabelled.

Choice: preserve all current changed/untracked draft bytes, binary Git diff, status and hashes in durable primary evidence. Temporarily write only the committed original core minimap stub bytes from HEAD, without checkout/discard of WIP. Correct/review MM1 test instruments under C263, bind the actual source manifest, and run the genuine original-behavior headed pair before further feature work. Keep the native ping stub red and add the asymmetric/target-mode tests as required. After the real camera-not-moving red is verified, restore the archived core draft byte-for-byte and continue implementation. Record actual chronology; this is a staged baseline correction, not proof the browser run happened before the draft was written. Do not edit during an immutable run. No data loss, acceptance waiver or new agent run.

Evidence: RUN82 confirms no MM1 receipt/runtime exists, only native baseline and client-ping red; current core WIP; RUN83 reports. Alternative not taken: throw away WIP, claim native tests prove browser behavior, fabricate an earlier timestamp, or continue feature work without the required physical baseline.84/170 registered runs;19/25 lanes complete.


## C266: Bind MM1 to actual exported schemas and driver IO

Conflict: repaired draft MM1 still references nonexistent scene cameras/pose fields, unit kind/id/map_pos/projected_ground_center and marker.order.target. Actual exporter lib.rs:659-675 has top-level clip_from_world and viewport_logical_rect; scene_entry at600-610 has unit_id or node_id, translation, ground_screen_center and pick_radius. Markers are actual JSON entries with an order string and x/y, not a nested target. Invented fields would fail before the intended camera guard or encourage a false bridge change.

Choice: use actual producer source and archived real captures as the typed schema contract. Reuse existing C84/HUD4 matrix unprojection for camera ground targets; join scene units to actual replicated unit IDs for owner/content. Read actual retained emitted_orders/results/alert producers for command and ping causality. Source/control-review the complete referenced field map against a real captured scene before qualified baseline. Use exactly the published driver-io.js IO_KEYS/signatures (click,drag,key,move,wheel,screenshot,save,wait,reload,use). Do not add a bridge or pose field merely to fit the draft, synthesize fixture values, weaken pre-camera guards, or give semantic reads to the driver. This is High test-instrument/schema repair under brief4; no player/schema/row change.

Evidence: actual exporter and scene_entry source; debug.rs feed exports; harness/scene-checkpoints.js uses input_call/window/wall_time_ms/scene/debug; harness/driver-io.js published methods. RUN83 draft2 schema review is durable under t14-review/run83-source. No current MM1 runtime pass is claimed.


## C267: Forward and test the actual MM1 public parameters

Conflict: the first headed original-stub pair failed before the oracle because the launcher dropped MM1 map, palette and minimap geometry fields. Physical click coordinates became null and Playwright rejected them. Both runs are setup failures, with no semantic baseline credit. The earlier source approval did not exercise the actual launcher generator.

Choice: add only the eight explicit MM1 public fields to the existing launcher whitelist. Keep the same values in driver and oracle params. Test the actual embedded generator with the actual MM1 row, require finite physical coordinates, and show the new contract test fails against the original whitelist before exact restoration and the same passing test. RUN83 must review the immutable source hashes and controls before retry. Put the enforcing wrapper inside outer xvfb-run so its recorded DISPLAY is the actual child display. Preserve both failed attempts. No feature code or acceptance guard changes; the original core stub remains until a genuine camera semantic red.

Evidence: scripts/mc-browser.sh:175-181; tests/browser/rows/MM1.json:11-18; t14-builder/mm1-original-baseline/baseline-classification.json SHAac2a72db; RUN83 actual generator control SHA675a4e6e and revised manifest SHA2a071bdb. Alternative not taken: count null-coordinate failures as gameplay reds, bypass the launcher in pure tests, forward arbitrary row data, or implement the feature before the corrected baseline. This is brief4 High instrument repair.85/170 runs;19/25 lanes complete.


## C268: Define the Task16 physical plan, authentic copied-tab fixture and private observations

Conflict: RUN85 finds four High readiness blockers in the source packet. Inline actions are rejected by the real parser; separate browser contexts do not copy sessionStorage and the all-Joined startup excludes refusal pages; generic debug equality is vacuous; public stats has no owner/session-close observations. These are instrument gaps, not accepted red results.

Choice: use strictly validated Task16 physical plans under the existing stages pass-through. Keep HD1 actions_file semantics and ordinary-row startup unchanged. Validate known IO methods, finite arguments, selected windows, unique phase/capture names, ordered capture indices and nonempty row-specific checks. Keep the preparation guard until the full instrument path is ready.

For SLOT1, use the exact three-phase fixture in t16-review/run85/review.md. A: actual distinct decimal A/B owners and genuinely unclaimed C refusal, with A above2^53. B: after a completed driver phase, close rejected C and open a fresh visible popup from live A; browser copies sessionStorage itself. Replace opaque handle3 only between phases; preserve one IO instance, monotonic logs and document epochs. Three windows means three simultaneous windows, with the replacement document recorded. Keep A/B live through the full clone countdown by ordinary IO window rotation only. C: close clone, then actual A reload within5s and same-slot recovery within15s. No reseeding, copied JSON, semantic driver reads, hidden fixture side effects in IO methods, forced keepalive, synthetic refusal or changed deadlines. Role-specific readiness captures wrong semantic outcomes; missing bridge/timeouts remain setup failures.

Use a private mode0600 local JSONL observer at a documented sibling of the explicit --stats path, outside served/package trees; public stats stays unchanged. Record actual Hello outcomes, actual Slots owners, actual session add/removal with entity generation/epoch, and separately any proven server transport-disconnect event. Capture lifecycle time at the actual event with host/monotonic clocks. No timeout/rejoin/client event may stand in for server close. Keep raw IDs/capabilities private and distinguish authoritative session removal, observed disconnect and peer Close; unproven WebSocket close remains NOT TRACED. No authority, protocol, timeout, dependency/vendor or player behavior changes. Actual Bevy0.20 API must later compile under the sole runtime lease.

All seven rows need nonvacuous clause-specific oracles, retained PNG/input/source provenance and positive/negative controls before baseline. Preserve already-green Task15 subsets; do not remove them to manufacture obsolete planned reds. Keep native config tests isolated from user files/global environment; add independent literal path expectations and real notice/preservation tests. Register a new source-only builder run86 before repair. No Cargo/cache/browser/feature implementation or acceptance credit until the later lease and real baseline.

Evidence: scripts/mc_browser_params.py:39-42; scripts/mc-browser.sh:176-182; tests/browser/harness/run.js:108,164-174; harness/driver-io.js:12-19,29-51,214; crates/turts_server/src/stats.rs:163-178; session.rs:150-156,210-270; complete independent review t16-review/run85/review.md and slot1-phase-map.md. Alternative not taken: pretend setup failures are reds, seed a fake copied tab, infer server close from client countdown, add public owner/identity data, relax acceptance clauses or stop for delegated instrument decisions.86/170 registered;Task16run3/estimate6,soft12/hard24;19/25complete.


## C269: Use the Task14 source-review handoff slot for Task22 preparation

Conflict: Task14's independent source/baseline review is complete, while its builder still implements the feature. Keeping that reviewer waiting would prevent useful independent source preparation under the four-seat limit. Task22's Needs6,7,10,11,13,19,20 are all merged; it shares no required runtime with source-only preparation.

Choice: finish RUN83 as source/baseline review only, preserving its findings and own-plant proposal; register a new independent final-review/plant run when Task14's candidate is ready. Use the freed seat for RUN87 Task22 source-only instruments on a fresh branch/worktree from current integration. Keep the sole cache/compiler/browser lease with RUN82. RUN87 must not run Cargo, browsers or server processes, create a target/cache, commit or claim runtime red/green. Full actual native/process/pixel baseline, gates, plants and measurements still follow under later explicit leases. Each later resumed dispatch is a new registered run. No review or task gate is waived.

Evidence: RUN83 run83-progress-index.json SHA2de3afb8 and qualified baseline audit0b7e1cd5; Task22 plan:652-668 and board Needs. Root preflight t22-root/source-preflight-handoff.md lists actual existing log/script helpers. Alternative not taken: keep a reviewer waiting for feature implementation, start a second compiler/cache, skip final independent review, or count source prep as completed acceptance. This is brief4 lane/task ordering.87/170registered;Task22run1/estimate3,soft6/hard12;Task14run2/estimate3;19/25complete.


## C270: Match the native minimap tests to their explicit fixtures

Conflict: RUN83 found two Medium native-test mistakes before the next compile. The right-click test expected48,36 while TestWorld defaults to64x64. The target-mode test expected an attack cursor while the fixture order has cursorNone and the existing core falls back to target. These expectations do not describe their fixtures.

Choice: give the right-click test an explicit64x48 map and retain its expected48,36 point; retain separate80x40 asymmetric left/drag cases. Expect target for the fixture's cursorNone, then test cancellation sends no order/default cursor and the next ordinary Right resumes smart command. Preserve the actual C264 player behavior and real native baseline chronology. Repair these tests before focused native compile; no runtime pass is inferred.

Evidence: crates/turts_interaction/tests/common/mod.rs:129,217; tests/minimap.rs:72-75,115 onward; interaction/src/lib.rs:404. RUN83 final index684768e8 reports both findings; root read the actual corrected source. Alternative not taken: change production map/default cursor to fit wrong tests, weaken the world-point assertion, count compile/setup error as red, or omit the second normal click. This is brief4 Medium instrument repair.87/170runs;19/25complete.


## C271: Route real minimap input through the existing physical adapter

Conflict: the core minimap handler and client drawing compile, but root source preflight finds no client producer of InputEvent::Minimap. The current plugin only registers sync_minimap and widgets only add render metadata. The physical adapter still emits ordinary PointerDown/Up/Move, so native green alone cannot make minimap interaction reachable. The plan's file list does not name this necessary adapter seam.

Choice: extend the nearest existing physical input/capture mechanism, including core_adapter.rs when needed, to emit the typed Minimap event from the actual visible authored widget rectangle and physical click/drag. Retain event ordering, proper UI viewport/scale/clipping and inherited visibility, focus-loss cleanup, drag capture and ordinary widget/world behavior. One minimap gesture must not also emit a world selection/order. Keep core policy pure. Add meaningful adapter coverage, then focused checks before the next candidate browser pair. If a pair already runs, finish it unchanged and preserve its actual result before edits. This adds no player step/key/row and changes no protocol.

Evidence: crates/turts_client/src/core_adapter.rs:627-681 ordinary window events; minimap.rs:55-63 registers only sync_minimap; widgets/mod.rs:706-725 attaches config. Root notified RUN82 before its announced next headed pair. Alternative not taken: count unreachable native behavior as implemented, spend a known-failing browser pair, route gameplay through oracle/debug hooks, or bypass UI capture. This is brief4 nearest-mechanism/file ownership and mandatory implementation repair.87/170runs;19/25complete.


## C272: Preserve private observations across repeated stats runs

Conflict: the Task16 private observer draft opens the fixed stats sibling with create_new. A previous diagnostic file would make a normal repeated --stats PATH startup fail. The draft also allocates owner snapshots while diagnostics is disabled. Existing stats permits repeated use of PATH.

Choice: append to a validated regular private sibling, enforce mode0600, preserve previous records, and scope every record/alias/epoch to the actual new run nonce. Reject unsafe file kinds and do not truncate/delete old evidence. Harness/oracle must select the exact current boot and reject cross-boot timing/identity joins. Create owner snapshots only with diagnostics present. Keep event-driven bounded writes and unchanged public stats/authority. Add source controls and later actual observer tests under the runtime lease; no compile/runtime success is inferred now.

Evidence: crates/turts_server/src/session_diagnostics.rs:47-56 create_new draft; session.rs:223-228 unconditional owner snapshot; RUN86 agreed source correction. Alternative not taken: fail normal restarts on an old optional trace, silently erase evidence, mix boots, or add idle per-tick logs. This is brief4 instrument/mechanism handling within C268.87/170runs;19/25complete.


## C273: Keep minimap local coordinates top-down and release capture safely

Conflict: the first C271 adapter draft inverted local Y before passing it to the core, which already converts top-down widget Y to world Y. Bevy UI local Y is negative at the top. That draft would send a click at75%,25% to the wrong map half. A release without a current cursor could also leave minimap drag capture set.

Choice: normalize actual UI-local coordinates to top-down fractions without a second inversion, using the actual Bevy0.20 transform/normalize contract. Retain the core world conversion. Assert an asymmetric adapter-to-core example: local(.75,.25) on80x40 maps to world(60,30). Cover cursor-left/release-without-position/focus-loss cleanup so re-entry cannot continue a released minimap drag. Preserve actual event order and no duplicate world input. Repair/recheck before another candidate pair; runtime remains unproven until executed.

Evidence: cached bevy_ui0.20 ui_node.rs:211-242 chooses top for local_point.y<0 and exposes normalize_point; widgets/snapshot.rs:145-154 uses center-size/2 as top-left; interaction/minimap.rs:34 converts1-localY. RUN82 confirms corrected normalization and asymmetric control. Alternative not taken: change the oracle to expect mirrored clicks, flip the core/renderer to fit an adapter error, or ignore stuck capture. This is brief4 geometry/instrument and implementation repair within C271.87/170runs;19/25complete.


## C274: Capture the real order log for GM40 pixels

Conflict: GM40 requires zero Blocked transitions from --order-log. The browser launcher currently adds only --tick-log; client replicated activity at a later checkpoint cannot replace the transition history. This is missing instrument data, not a passing pixel clause.

Choice: add a narrowly validated optional capture_order_log:true for the GM40 row. The launcher adds the existing --order-log hook with a fixed run-local server-order.log path and forwards the reviewed capture flag/filename to public params. Require real file provenance, expected nonempty ordered transitions for the selected units, and actual no-Blocked assertion in the readonly oracle. Wrong type, incompatible row or unavailable required log fails setup; absence of the flag keeps ordinary launcher argv/params unchanged. The driver gets no semantic access. Test the exact real launcher generator/server-argv path with positive and negative source controls before runtime. No new server protocol/behavior, public stats or player step.

Evidence: scripts/mc-browser.sh:112-122 server argv; existing crates/turts_server/src/orderlog.rs hook and design14GM40:1243; harness/view.js:27 readonly serverFile. RUN87 reported the gap before browser execution. Alternative not taken: omit the pixel Blocked clause, infer no prior Blocked from current state, fabricate a log, or edit another active lane. Rebase later must preserve C267 and C268 launcher contracts. This is brief4 log/flag instrument detail.87/170runs;19/25complete.


## C275: Respect higher UI capture above a minimap

Conflict: C271's first routed adapter finds any visible minimap at the pointer, even behind a higher capturing overlay. The existing Settings overlay is full viewport, UiCaptureRoot, FocusPolicy::Block and ZIndex MAX-1; the minimap can remain inherited-visible behind it. Sending Minimap input there would bypass the existing modal capture.

Choice: resolve minimap routing against the actual visible hit capture order on its target UI camera. A higher covering capture must prevent camera/order input to a lower minimap; an ancestor below or unrelated non-overlapping capture must not disable an exposed minimap. Use the actual Bevy0.20 computed stack/picking order, not raw global Z guesses or a new shell special case. Add a real computed-widget regression with a covering visible capture and uncovered control before the next pair. Preserve hidden/clipped/focus/release safeguards and no duplicate world orders. This keeps current menu/player behavior.

Evidence: client_shell.rs:158-173 full modal capture; core_adapter.rs minimap_at draft only searches minimap roots; cached bevy_ui0.20 stack.rs:17,30-34,130-132 gives ComputedStackIndex/UI order. Root source review found this before approving the next candidate. Alternative not taken: allow click-through behind Settings, disable minimap globally whenever any UI exists, rely on an unrelated parent ZIndex, or waive capture. This is brief4 existing-mechanism implementation repair within C271.87/170runs;19/25complete.


## C276: Keep private connection and match identities lossless

Conflict: the Task16 observer draft emits Entity::to_bits and match_id as JSON numbers, while JavaScript cannot represent every u64 above2^53. Public debug currently has numeric match_id but no decimal companion. A rounded identity can join the wrong lifecycle/match evidence even though player_id_decimal is already exact under C247.

Choice: use exact decimal-string identities for private entity bits (including generation) and match IDs, scoped by boot/epoch. Join oracle evidence with those exact values; never convert through Number. Add minimal actual match_id_decimal to the existing readonly debug capture, rebuild and native report if needed for browser correlation, retaining old fields for compatibility. Verify greater-than2^53 and u64::MAX serialization/rebuild/report controls. This changes observation encoding only, not authority, wire DTOs, player steps or timing. All actual runtime evidence remains NOT TRACED until run.

Evidence: session_diagnostics.rs draft:184,188,204,208; debug.rs:294,466 numeric match_id and:125-127,316 existing exact player_id_decimal; cached bevy_ecs0.20 entity/mod.rs:566 u64 to_bits includes generation. Alternative not taken: round identities, infer equality from a lossy number, add public capabilities, or change server policy. This is brief4 typed log/field instrument detail following C247.87/170runs;19/25complete.


## C277: Use an exposed trooper to create the real damage stimulus

Conflict: the first C271/C275 MM1 attempt reaches camera/drag/right/zoom/dot/frame checks, then has no OwnUnitDamaged ping. Root read the actual captured history: the screenshot nearest-center choice selected interior trooper21 at29.4,20.2; its attack on dummy46 at55.2,34 was accepted at tick98, but by tick322 the actor only reached29.5028,20.2762 and the target still had40health. At tick409 health is unchanged. It is surrounded by stationary formation members. An emitted/accepted attack is not actual damage.

Choice: after the frozen pair ends, select an exposed frontier trooper from actual current screenshot pixels toward the enemy, excluding the large post and beacon cluster with real PNG positive/negative controls. The oracle independently verifies exact actor/content/owner, actual movement, real target damage and the causal ping. Do not read semantic state in the driver, change movement/production content/timeouts, fabricate damage or weaken the ping clause. Preserve failed attempts and the distinction between accepted order and effect. Use deliberate unjammed fixtures for other order clauses unless the jam itself is under test.

Evidence: t14-builder/mm1-feature-candidate-c271c275/runs/run-20261110T105051Z-QXnTHb actual input26/window1, input55/window2 and retained accepted seq1; root mm1-candidate-run1-attack-stimulus-audit.json; driver selectBody nearest-center. Alternative not taken: blame alert delivery before damage exists, lengthen a wait for an unmoving unit, add pathfinding/push, or count missing stimulus as full alert acceptance. This is brief4 physical-driver/fixture instrument repair preserving the player-visible scenario.87/170runs;19/25complete.


## C278: Put the DET1 hash-order fault at the effective neighbor consumer

Conflict: the grid's sparse HashMap iteration is followed by an unconditional unit-ID sort, and dense queries walk coordinates. Merely replacing grid storage/hasher would not change effective movement neighbor order. DET1 needs the actual nondeterministic iteration fault, not an inert edit.

Choice: preserve exact grid.within membership in the plant, route it through a fresh HashMap with RandomState at the group-avoidance consumer in movement.rs:2132, then iterate directly into the existing order-sensitive velocity accumulation without a following sort. Leave other grid callers and the independent hard projection query unchanged. Normal production code stays canonical. At runtime require actual two-process tick projections through400 to differ, complete tick/unit coverage, nonzero >=2-neighbor work, and exact-restored equality/order logs. The geometry self-check alone is insufficient. This is a source proposal only: sensitivity remains NOT TRACED until executed; no plant retires or changes failure class.

Evidence: grid.rs:61-109 canonical sort; movement.rs:2132-2171 avoidance accumulation,2194 separate projection; RUN87 source-phase.md:52 traces callers. Alternative not taken: plant a cancelled HashMap reorder, infer sensitivity from neighbor counts, edit unrelated queries, or claim a source proposal as red. This is brief4 nearest equivalent test-fault mechanism.88/170registered after new Task14 reviewer;19/25complete.


## C279: Require the server result for each physical minimap command

Conflict: RUN88 finds that MM1 assertOrderFromInput only joins a physical pointer release to a client-emitted order. A request can be emitted then rejected, so that is insufficient smart-command acceptance. Root actual candidate showed an accepted attack, but the oracle itself did not require it.

Choice: bind the actual retained emitted sequence and unit to the real per-unit accepted server result, with exact intended order/target from the same intent and actual current-order/movement effect when applicable. Reject missing/rejected/wrong-sequence/wrong-unit controls. Keep real attack movement/damage/ping causality underC277. Use existing actual producer fields: emitted_orders has intent_id,seq,order,units,target, not group/queue. Do not require invented fields or add a bridge merely to fit a draft. If later full order clauses need unavailable envelope data, propose a minimal actual observation export at the owning producer. Preserve failure/success chronology and existing guards.

Evidence: tests/browser/mm1-oracle.js:242-258 emitted-only assertion; core_adapter.rs emitted_order_values actual producer; RUN88 Medium instrument finding; run1 retained seq1/unit21 accepted attick98. Alternative not taken: accept intent as execution, match unrelated accepted results, weaken real damage, or fabricate metadata. This is brief4 Medium instrument repair.88/170runs;19/25complete.


## C280: Clip all minimap glyphs to the authored widget

Conflict: RUN88 finds that clipping camera centerlines to map bounds does not clip the actual child glyph rectangles. A2x2 frame point at boundary0 has a negative left/top; dots and ping crosses can likewise draw outside a root with default Overflow::Visible.

Choice: use the existing Overflow::clip() on the authored MinimapView root so unit/node/ping/frame children obey its real widget boundary, with the authored border and inherited visibility preserved. Add a focused boundary control and retain full real MM1 pixels. Make changes only after any active immutable run ends. This does not change production maps, keys, alert source, authored style or acceptance row.

Evidence: widgets/mod.rs MinimapView setup; minimap.rs:428-430 frame glyph offset and cross/square placement helpers; RUN88 Medium source finding. Alternative not taken: allow glyphs to escape, weaken the interior frame/ping checks, or clamp only the center while ignoring glyph extent. This is brief4 Medium presentation repair using existing layout clipping.88/170runs;19/25complete.


## C281: Join damage samples to their actual alert observation tick

Conflict: the MM1 draft requires a changed-health sample at exactly ping.tick. Actual producers have different clocks: replica records changed health at its server-delta tick, ingest can drain several deltas in one rendered frame, and feedback stamps the alert with last_applied_tick of the observed snapshot. RUN88 proves the mismatch in authentic Task15 evidence: unit8 health drops50@555 to49@614; the exact same retained feedback event and ping observe tick615; later health48@618. No changed-health sample615 exists.

Choice: retain exact ping-to-retained-event rule, unit, world position and observer replica tick. Require the same unit's actual strict health drop after the qualified pre-damage/no-ping baseline and at/after the real accepted attack tick, no later than the ping observation tick. The latest eligible sample must be lower than baseline; actual capture health must also be lower, with later real damage allowed. Reject wrong unit, pre-stimulus/old drop, future sample, no drop or mismatched event. Test authentic614-to615 and exact-time positives plus these negatives. Do not infer an expired immediate previous frame, invent a615sample, change feedback timestamps or add a bridge. C263's causal same-tick rule refers to the actual observer frame, not a fabricated equality of distinct producer clocks.

Evidence: replica.rs:1181-1190 changed-health trace; ingest.rs:937-939 batch drain; feedback.rs:369-373 snapshot tick; t15-builder/run78/browser-c235-headed/runs/run-20261110T024151Z-scY6Si checkpoint92/window1; RUN88 real-batched-health-ping-timestamp.json and batched-health-causal-controls.json. Alternative not taken: reject authentic batched events, accept any old lower health, forge history, weaken real damage or alter player timing. This is brief4 actual log/timing instrument interpretation; no row change.88/170runs;19/25complete.


## C282: Measure real admitted Muster work at both unit counts

Conflict: root source review finds Task22 cost prep creates a synthetic walker with modified weapon range256/cooldown50 instead of the design's Muster workload. It also submits5000 units in one OrderCommand, but command.rs:19,308-309,526-527 and orders.rs:2624-2625 reject more than4096. Thus a nominal5000 timing could measure rejected/idle work. At most2048 units spread per tick, so one update also does not complete a valid5000-unit spread. No metric has run or been accepted.

Choice: the next Task22 builder must repair this before measurements. Use actual acquired Muster definitions in an isolated benchmark map sized for1000/5000 legal placements; preserve production files and authored movement/weapon values. Submit legal <=4096 parts with a common group through the existing merge mechanism, observe actual admitted outcomes, and account for all spreading dispatch ticks until the full count is active. Warm up pending dispatch before avoidance samples. Assert real active movement and actual eligible combat/target work, record actual admitted/active/sample counts and per-tick/aggregate method. Label whole-workload tick duration honestly; do not claim an isolated phase CPU measurement without instrumentation. Keep the budget Open and machine/commit/fixture provenance exact.

Evidence: mc_cost.rs:46-126 synthetic definition and:129-162 single oversized queue/update; design6.7 requires Muster at1000/5000; command/orders limits above, orders.rs:1190,1657 spreading cap. Original RUN87 packet remains preserved, not final accepted. Root readiness note t22-root/cost-readiness-findings.json. Alternative not taken: publish idle/rejected timings, silently change authored combat to amplify cost, bypass admission/cap, or add an invented performance threshold. This is brief4 fixture/instrument repair; runtime remains NOT TRACED.88/170runs;19/25complete.


## C283: Keep the ordinary target-attack stimulus before considering a workaround

Conflict: RUN82 proposed adding Rush-to-point before damage because authored Unit Attack has engageNone and was assumed not to approach. That inference contradicts the detailed design and current code. The only actual failed attack still used jammed interior actor21; exposed frontier actor40 has not been tried. Replacing the mechanism now could hide an ORD4 chase defect.

Choice: reject the Rush substitution for this trial and keep C277 exposed frontier actor40 plus ordinary target Attack. Design behavior table says Unit Attack approaches weapon range; engage StopAndFire applies to point targets. Current unit-attack branch calls attack_goal and inserts MoveTarget out of range regardless of engage. Synchronize driver/oracle back to that stimulus. If actual exposed target Attack still fails, preserve and diagnose the real result, then repair under existing rules; full ORD4 out-of-range chase remains mandatory. Do not infer no-chase from None, fabricate successful movement, weaken the row or use pre-attack damage as target-attack causality.

Evidence: design5 behavior table:498, ORD4:1239; orders.rs:2184-2210 attack_goal,2537-2564 UnitAttack start,945 run_unit_attack; actual authored order_attack.ron targetUnit/engageNone; only failed actor21 history underC277. Alternative not taken: silently add Rush to bypass an untested valid chase, change content/engine policy, or request CEO input for an incorrect source assumption. This is brief4 fixture/mechanism interpretation using the specific design rule; no player change.88/170runs;19/25complete.


## C284: Handle valid subpixel minimap dimensions without a panic

Conflict: RUN88 finds sync_minimap accepts finite positive widget dimensions such as0.5, but map_local clamps to an upper bound widget_size-1. A0.5dimension gives-0.5 and f32::clamp(0,-0.5) panics. Existing placement permits this finite nonnegative authored size.

Choice: bound the maximum local coordinate at zero for dimensions below one, or use equivalent total finite mapping, so valid small widgets render/clip safely. Keep normal200px mapping, world bounds and invalid/nonfinite guards unchanged. Add a meaningful subpixel regression and observe the actual former panic then restored passing check; no extra browser matrix for this pure numeric edge. Do not reject otherwise valid content or loosen bounds.

Evidence: minimap.rs sync positive-size guard and map_local max=widget_size-1/clamp; placement.rs:197-208 allows finite nonnegative Px/Frac; RUN88 Medium finding. Alternative not taken: leave a legal-layout crash, change schema acceptance, or clamp to an invalid interval. This is brief4 Medium implementation repair.88/170runs;19/25complete.


## C285: Give Task16 acceptance clauses fixed product semantics

Conflict: RUN89 executes the real producer/validator/driver/oracle and finds remaining High false acceptance. A KM1 executable plan can satisfy required clause IDs using only KeyQ, synthetic PNGs and debug.phase=Joined, without defaults, override/reload, reserved notice or pack rejection. PER1 is contradictory: browser-key-names is required to have source.kind=input, then required to have source.kind=private-json/storage-audit.json. Neither legal choice can pass. Named IDs and caller-selected truthiness are not product assertions.

Choice: use fixed row/clause-specific semantic evaluators and producer-backed evidence contracts. Physical plans and pixel calibration remain declarative, but the caller cannot choose an unrelated source/operator to satisfy a clause. Separate physical-trigger evidence from storage-key evidence; PER1 key evidence uses the actual private storage audit and its required physical IO is independently checked. Define one consistent schema in validator and oracle. Observe every required behavior in each of the seven design rows; reject unrelated phase-truthy and missing/replaced clause controls through the actual launcher-to-oracle path. Keep preparation guards until later source-bound plans/runtime baseline; mocks are controls only.

Evidence: mc_browser_params.py:77,274-276,303-307 contradictory PER1 checks; RUN89 actual PER1 input/private rejection controls and KM1-vacuous-executable actual producer/whole-oracle pass; t16-review/run89/parser-results.json/source-results.json. Alternative not taken: rename generic truth checks, accept clause presence as behavior, remove the persistence clause or count preparation/setup as semantic red. This is mandatory High instrument repair under brief4.89/170runs;19/25complete.

## C286: Fail closed on incomplete or wrong-boot SLOT1 observations

Conflict: RUN89's unmodified private SLOT1 oracle accepts sequence gaps,100ms Hello cadence, missing countdown captures and no owner-frame observations. The writer discards I/O errors, aliases are collected across all appended boots and the first old A alias selects the wrong boot. Missing or stale lifecycle data can therefore appear as uninterrupted authority and unchanged ownership.

Choice: write a real boot/start record with an exact run nonce, bind the fixture to that current boot before building any alias/epoch map, and require contiguous complete records. Propagate private writer failure to a failed diagnostic run; never silently accept a partial prefix. Under the existing explicit stats hook, retain bounded private authority/owner checkpoints at phase boundaries or at the existing low-frequency stats interval (at most about1Hz), with actual active epochs and current Slots. Require healthy current-boot coverage through each phase's final capture; missing coverage is instrument failure, not proof of liveness. No per-tick/world bulk trace and no new public owner endpoint. Keep prior boots intact and public stats unchanged.

The oracle must independently enforce actual countdown captures and duration, >=1s Hello cadence, the existing absolute15s first-attempt budget, original A/B epoch presence through clone refusal, exact unchanged owners, and actual reload/removal correlation. Bind all timing to the same boot/entity-generation/epoch and checked host/monotonic clocks. Reject missing events, gaps, old aliases, wrong epochs, rapid retries, no countdown and missing owner observations. Preserve actual Session removal versus NOTTRACED peer WebSocketClose. No timeout/authority/protocol changes.

Evidence: session_diagnostics.rs:112,190,210 ignored writes; shell-oracle.js:265-275 cross-boot alias/selection and increasing-only sequence; RUN89 SLOT1-private-counterexample.json/source-results.json actual counterexamples. Alternative not taken: infer no removal from incomplete logs, mix aliases from prior boots, use a client timestamp as server close, or label synthetic controls runtime proof. This is mandatory High evidence repair under C268/C272/C276 and brief4.89/170runs;19/25complete.

## C287: Establish owners before capacity refusal and keep document epochs monotonic

Conflict: current SLOT1 fixture navigates A/B/C before any role wait, so unclaimed C can win a slot before the intended A/B owners. It also resets handle3 epoch to0 before replacement; both original C and copied A then appear as epoch1. These are fixture races/identity collisions, not product failures.

Choice: establish actual A and B ownership through ordinary entry and retained real readiness/evidence before starting ordinary C. If A/B get a wrong product outcome, retain it for a semantic oracle result; missing bridge/timeout remains setup failure. Replace rejected C only after its completed phase and all IO flush. Allocate handle3 document epochs from one monotonically increasing counter across popup creation/navigation/reload; never reset or hardcode old/new epoch1. Preserve one IO instance and append-only input/checkpoint history, authentic opener copy, three simultaneous windows, and all C268 phase constraints. Test actual extracted epoch/ordering code with positive and collision/race controls before runtime; final browser setup remains unproven until run.

Evidence: harness/run.js:112-192 initial goto ordering,:390 reset,:418 hardcoded epoch; RUN89 actual tracker control shows oldCepoch1=newAepoch1. Alternative not taken: preclaim synthetic slots, overwrite C storage to imitate copy, discard old document traces, or blame a setup race on the game. This is mandatory High fixture repair under brief4; no player-step/row change.89/170runs;19/25complete.


## C288: Close the full ALR1 follow-up after real MM1 pixels

Conflict: C203 deferred rendered minimap ping pixels and explicitly requires full ALR1+MM1 after minimap implementation. The completed current MM1 pair proves real ping pixels, but current feedback-oracle judgeAlr1 ends with sound/cooldown/feed checks and has no rendered minimap assertion. Full C203 closure cannot be inferred from that older partial ALR1 oracle.

Choice: mark only the C203 actual-minimap-pixel evidence portion complete from the verified two-headed MM1 pair. Add the original ALR1 rendered-ping clause using its real alert unit/location, authored minimap rectangle/style and retained actual PNGs, with orange/no-ping negatives. Preserve every visible alert, click-center/no-selection-change, within-one-second damage suppression and actual OGG sink guard. Apply C281 actual observer-versus-health clock semantics where necessary. Independently source/control-review and run the actual full ALR1 headed pair. Reuse completed MM1 only with unchanged relevant engine/content/harness/driver/oracle inputs; do not relabel runs. ALR1-only oracle changes need no unrelated feedback matrix repeat with exact unchanged-branch proof.

Evidence: original boardC203:1808 then-rerunfullALR1+MM1; feedback-oracle.js judgeAlr1:181-299 has no pixel guard; ALR1 row expected_pingtrue; feedback fixture authored MinimapView200px/inset8/ping#ffcc00; current MM1 wrapperf04e765f and independent d64d30a3/root full-mm1-frontier-pair-audit. Alternative not taken: declare full ALR1 without rendered checks, leave permanent partial coverage, repeat unrelated rows, or fabricate a ping. This fulfills existing acceptance, not a changed row.89/170runs before new dispatch;19/25complete.


## C289: Use a bounded isolated node-depletion fixture for ORD3

Conflict: protected Muster's ore node has1000remaining; two2/s workers plus capacity/unload/travel would make repeated browser node_depleted checks take many minutes. ORD3 must observe real depletion/last_failure, not wait an arbitrary short interval or alter production content.

Choice: create an isolated test fixture copy retaining actual Muster IDs, policies, layouts, assets and all production unit/order definitions. Change only the fixture map's intended ore node remaining quantity to a small finite positive amount such as2, with exact original/copy diff and hashes. Use ordinary physical gather to exhaust that actual node, then assert real remaining0 and the same worker's authoritative node_depleted last_failure plus required visible feedback. Preserve current queued-order rules and all other ORD3 clauses; use17physical append inputs for depth16 without changing policy. No protected live map/resource/order change and no fake depletion event. Other fixture changes need a concrete separate proposal.

Evidence: mods/muster/maps/field.ron:521 remaining1000; actual worker gather/unload2/s/capacity10; design14ORD3:1238; RUN88 source planning recommendation. Alternative not taken: shorten a wait without actual depletion, change production quantity, inject failure metadata, or skip last_failure. This is brief4 fixture/mechanism detail preserving the required gameplay behavior.90/170runs;19/25complete.


## C290: Use the planned native pack test and minimal existing test dependencies

Conflict: the ORD draft points at a server binary test helper, but the plan requires crates/turts_pack/tests/mc_order_rows.rs and actual Core per-unit smart output through CommandKind::Order. The public Core types need protocol IDs; pack has no direct protocol dev-dependency. A new glam dependency is unnecessary because its existing bevy_math resolves the same locked glam0.33.12 as interaction.

Choice: keep the planned pack test file, use existing bevy_math::Vec2 and add only test-only path turts_net_protocol when required. Allow the minimal turts_pack dependency-list change in Cargo.lock, with no version/checksum/unrelated graph drift; subsequent tests stay --locked. Use real Core smart composition and actual catalog values with the public typed simulation command path, not a duplicated smart resolver or unsupported server-binary library import. Preserve production dependency boundaries.

Clarify fixture expectations from actual policy: three Appends from idle produce one current plus two queued; establish an active current order before17Appends to overflow a16queued depth. Smart-worker foreign fallback for authored PointOrUnit remains Move Unit(targetId), whereas node fallback is Point; explicit force Move uses the existing Point conversion. Keep all actual target/behavior checks.

Evidence: pack Cargo.toml existing bevy_math/interaction devdeps; Cargo.lock bevy_math→glam0.33.12 and interaction exactsame; lib.rs:854 Point conversion only for targetPoint,:695-699 explicit force conversion; RUN88 native/draft review. Alternative not taken: relocate the required test, add duplicate vector versions, copy smart logic, alter queue policy or expect a target form the source does not emit. This is brief4 test-file/dependency/instrument detail.90/170runs;19/25complete.

## C291: Prove the no-idle boundary with native phase probes and actual server ticks

Conflict: client snapshots can batch-skip ticks and the existing order-transition log lacks queue targets, so neither alone proves the exact C261 completion-observing Orders phase. Invented OrderLines or phase metadata is not valid. The existing authoritative tick log is complete and the simulation exposes ordered schedule sets.

Choice: native test-only observers after Orders/before Movement and after Movement/before Gather record exact actor tick/position/MoveTarget/current/queue. Witness Movement t landing A, Orders t+1 promoting B and setting its target in that phase, then Movement t+1 advancing toward B. Browser uses the actual complete server-tick.log, actual physical A/B point orders accepted and queued before A lands, no intervening replacement, exact unit IDs/targets, and no missing tick rows. Find landing t within0.001 and require meaningful nonzero displacement with positive dot toward B at exact t+1, with no idle tick. Use deliberate clear singleton waypoints. Together with actual Orders→Movement source binding and native phase probes, this measures the same mechanism without a new producer/CLI. Do not infer it from client screen motion or a coarse state line.

Evidence: sim/plugin.rs Orders→Movement→Gather chain; ticklog.rs sorted per-unit each completed tick; RUN88 C261 proposal and source field audit; design4.5/planTask14. Alternative not taken: demand same physical landing tick despite detailed design, tolerate an extra idle tick, fabricate a phase feed, or add unnecessary private server tracing. This is brief4 timing/instrument mechanism under C261, preserving exact behavior.90/170runs;19/25complete.


## C292: Expose minimal actual input and package observation facts

Conflict: fixed Task16 clauses need authored/default versus runtime bindings, actual named dispatch and exact root mod/version. Existing effective_bindings merges defaults and overrides, effects alone do not identify every matched action, and UI package identity is not necessarily the root game's identity. Generic caller metadata cannot replace those facts.

Choice: expose default/package bindings through the existing real resolver with an empty runtime override, separately from effective runtime bindings. Record bounded per-step actual named dispatch at the actual pure-Core dispatch branch, with input origin/code/index and matched action/order; export it through the owning adapter/frame/debug. Keep Core free of Bevy and action behavior/wire DTOs unchanged. Fixed oracles must still require actual resulting action/order/state, not dispatch presence alone.

Derive root mod ID/version from the successfully verified lockfile entry matching the advertised root, or the actual verified root archive; expose minimal readonly Acquisition metadata/getter. Do not substitute interface_package, a row constant or an unverified response. Missing new feature observations remain absent/default until real implementation; no fabricated successful counters. Avoid unnecessary new core dependencies; client exporter can serialize plain typed records. Compile/runtime behavior remains NOTTRACED pending lease and baseline.

Evidence: debug.rs:138-164 existing effective resolver; Core.step KeyDown resolves action then dispatch_named_action; dispatch_named_action handles order.hotkey and engine actions; acquire.rs Acquired holds content/interface not root version; join.rs verified lockfile/advertised root acquisition. RUN90 reported the concrete missing producer facts. Alternative not taken: keep caller-selected truthiness, infer root version from interface names, duplicate matching logic in a fake debug bridge, or implement persistence/rebinding green before baseline. This is brief4 minimal log/field observation detail underC285.90/170runs;19/25complete.


## C293: Measure force Move suppression through actual arrival

Conflict: force Move suppresses firing while its effective Move order is active; a later Idle capture can legitimately contain auto-fire. The browser tick log has positions, not exact MoveTarget lifecycle. A loose arrival tolerance can cut the observed interval early, and the requested point under a live enemy can require a nearby legal landing slot.

Choice: native phase observers identify actual MoveTarget removal on landing tick t and require zero Fired from the exact ready actor through t inclusive. Orders clears its effective Move identity on t+1; Idle auto-fire from t+1 is allowed. In browser evidence bind the actual accepted force Move point under a live enemy, real ready-weapon firing opportunity, complete authoritative tick coverage, no replacement/failure/Blocked, and the first captured Idle/currentNone. Infer landing from the first tick of the final exactly equal coordinate-pair suffix ending at that capture; do not use a loose distance tolerance to shorten the suppression interval. Validate the actual legal nearby endpoint rather than require an occupied enemy center. Reject every eligible foreign strict health drop from acceptance through landing t, using preceding health samples; do not check only the destination target. All native ORD coverage findings remain open until repaired and independently checked.

Evidence: movement.rs:756-764,844-853 removes MoveTarget at arrival; orders.rs:731-741,785-810,1010-1067 clears the active identity on the next Orders phase; combat.rs:151-177 reads that identity for Never. RUN88 source review also finds the required ready-weapon/cooldown nonvacuity guard. Alternative not taken: forbid legitimate later Idle fire, infer early completion from screenshot timing or proximity alone, require overlap with an occupied center, or accept a cooldown-only zero-shot result. This is brief4 fixture/timing measurement detail, preserving force Move behavior.90/170runs;19/25complete.


## C282 source clarification: the first atomic group may exceed 2048

Root inspected orders.rs:1958: the cap defers a group only when used>0 and used+next_count>2048. Thus an initial atomic merged5000 group may dispatch in one tick; the earlier statement that one update cannot cover5000 was too broad. Require actual accepted IDs, dispatch accounting, pending emptiness and all actual spread ticks, whether one or several. The synthetic non-Muster definition and rejected >4096 single envelope findings remain open. No metric has run or been published. Updated t22-root cost findings and next-builder handoff; no product or acceptance change, no new run.90/170runs.


## C286 boundary clarification: no shutdown change

RUN90 asked whether to add SIGTERM/AppExit to write run_end. Root keeps product shutdown unchanged. The fixed oracle needs a contiguous healthy current-boot stream through the retained phase final capture and a real later bounded owner_checkpoint, not proof of the server's whole lifetime. The fixture may wait for the next actual low-frequency private checkpoint with a bounded timeout before reading. Missing checkpoint, partial record, boot mismatch or writer failure remains instrument failure; EOF alone is no proof. Original owner liveness is bounded before the later intentional reload. No public endpoint, privileged driver read or synthetic barrier; peer WebSocketClose remains NOT TRACED. This clarifies existing C286 rather than changing a player row.90/170runs.


## C294: Bind browser evidence to actual document, transport and phase lifetimes

Conflict: RUN91 actual-helper controls show transient Joined-before-frame classified as a product failure, a stale retained KeyQ frame satisfying a later receipt, missing visible countdown accepted, and an observer stream ending before final rejoin capture accepted. Root and RUN91 also confirm the oracle wrongly requires one server connection epoch across retries, while existing rejoin.rs:184-195 and join.rs:267-285 reconnect on fresh transports; server session.rs:292-294 kicks refusals. One copied document is not one transport. Whole-boot A refusal selection can also mix the copied document with original A's later reload.

Choice: root makes a bounded instrumentation repair in harness/run.js and shell-oracle.js, then RUN91 rechecks exact new bytes. Wait for actual owner Joined+frame+scene readiness or actual terminal Failed; bind KM1 to the actual physical key sequence and contemporaneous dispatch/outcome frame. Bind the copied phase to actual browser-open and terminal-capture host bounds, exact copied document/identity, and each distinct observed connection entity/epoch, while retaining original A/B epochs. Require real changing visible countdown snapshots and a healthy actual owner/active-epoch checkpoint after final rejoin capture, as well as earlier phase boundaries. Keep all incomplete rows/pixel calibration guarded; tiny synthetic white patches are not product text proof. No server retry/timeout/ownership behavior, wire data, keys or player acceptance row changes.

Evidence: t16-review/run91/source-controls.js/source-results.json and slot1-real-retry-transport-control.json; shell-oracle.js:178-191,618-620,647-648, final newSession check; harness/run.js waitForTask16Role. Original35-path archive07538d46 and root c294/pre-edit preserve prior bytes. Alternative not taken: change reconnection behavior to fit a false oracle, accept stale evidence or incomplete streams, count a Hello as a visible countdown, or mark preparation as runtime green. This is brief4 High instrument repair under C285–C287;91/170runs,19/25complete.


## C295: Judge ordered-target priority when that target is eligible

Conflict: the ORD4 draft requires ordered46 to take damage before closer45 from command acceptance onward. Actual qualified MM1 actor40 attacks45 at238 while ordered46 is4.9143away and45 is3.8868away, then attacks46 at258 when46 is3.999999away and closer45 is2.9845away. The draft rejects valid approach behavior. Design6.2:596 explicitly prioritizes the ordered target when alive and in range.

Choice: retain real accepted Unit Attack, out-of-range approach, target death and exact native actor Fired focus proof. Browser focus starts when the actual ordered target is alive, matches the weapon relation and is in the actor's authored range. At the first ensuing actual firing opportunity, require ordered-target damage while the nearer eligible distractor does not take that actor's shot. Bind actual actor/target positions and strict health transitions at the eligible ticks; establish actor attribution by excluding other in-range own shooters or retain the exact native actor witness if browser health is ambiguous. Earlier legitimate damage to an in-range distractor while the ordered target is out of range is not a focus failure. Preserve all actual damage chronology rather than deleting early samples. No combat behavior or acceptance row change.

Evidence: design6.2:596; combat.rs:297-329 matches alive/relation/range before focus; run88/ord4-authentic-mm1-ordered-focus-control.json SHA9387b2db, actual MM1 ticks238/258. Alternative not taken: demand out-of-range fire, disable legitimate approach fire, infer focus merely from eventual death, or loosen exact native focus. This is brief4 instrument timing detail applying the design of record.91/170runs,19/25complete.


## C296: Rejudge stricter exact ORD5 emissions on complete retained runtime data

Conflict: after ORD5 pair started, RUN82/RUN88 found a draft guard using separate allowed order/unit sets. It can allow an extra cross-product envelope, although expected per-unit typed current outcomes are checked. The frozen5141b849 oracle's source approval is superseded for final acceptance. Its actual immutable headed attempts must finish without source edits.

Choice: archive both original executions and their exact original source/profile/results. Repair to exact allowed (order,unit) pairs with expected cardinality, no duplicates and no extra same-gesture emissions; run actual-fixture positive/negative controls and independent source recheck. A full strengthened oracle may then rejudge BOTH complete retained actual runs through a file-backed view if every consumed input, checkpoint, PNG and parameter is genuinely retained. orders-oracle.js reads inputLog/serverFile only for ORD5, so this is feasible in principle. Bind original runtime source hashes and new oracle hash separately, prove all production/driver/row/launcher bytes unchanged and all original guards retained, and label the result a semantic replay rather than a new browser execution. Missing artifacts, changed stimulus/production behavior or unresolved assertion means no carry; rerun the affected row normally. Never fabricate a full positive from partial fixtures or relabel old5141 completions as a new-helper run.

Evidence: RUN88 exact-pair finding; orders-oracle.js:9-11,356-369 file-backed inputs and screenshots; actual active t14-ord5-headed-pair-candidate1. Alternative not taken: mutate during a frozen run, accept extra unintended commands, repeat unaffected product work without need, or declare an incomplete replay green. This is brief4 instrument/evidence reuse detail under the CEO-approved grouping of redundant tests; all row clauses and two actual headed attempts remain mandatory.92/170runs,19/25complete.


## C297: Preserve the final source packet and record lost unaccepted intermediates

Conflict: RUN92 refreshed one post-edit tar path in place across cb24→371→f6. The intermediate cb24f1f5 and371d894a hashes were reported, but those tar bytes were overwritten and cannot be presented as retained evidence. No compiler, runtime, product green, High closure or acceptance relied on either intermediate.

Choice: record this Low provenance gap; intermediate tar bytes are NOT TRACED and are not acceptance evidence. Do not fabricate a reconstruction. Preserve final f6aedd075da31d5d9c702f6cc96a4b8be276ca9111fbd40f75da035033de8cf4, the earlier C2947bb8622f and original RUN9007538d46 archives, source manifests/diffs/controls and disclosure. Root independently hashes all36live files and exact final tar members against the final manifest before new review. Later source packets use versioned exclusive destinations and never overwrite prior source-bound reports/bytes. Continue a newly registered independent review of final f6; all real future red/green commands still need immutable actual source proof. No protected/frozen/player behavior changed and no narrowed stop condition applies.

Evidence: RUN92 explicit disclosure; t16-builder/run92/post-edit final tar/source-hashes; root live/archive36-path comparison. Alternative not taken: claim lost cb24/371 bytes exist, label hash-only records full archives, or stop valid work despite intact current/base source and no accepted evidence loss. This is brief4 Low evidence-handling decision.93/170runs,19/25complete.


## C298: Prove finite Gather activity at the actual refusal, not a later screenshot

Conflict: C289's node2 depletes quickly. Actual candidate2 IO had1.52–1.56s between G and the attempted Append; removing an intervening screenshot reduces it to about0.9s, but the following snapshot still arrives about2s after G. Actual worker reach is radius0.45+node1+gather1.5=2.95, so approach is about0.52s plus1s harvest. Requiring current Gather in that later snapshot rejects legitimate completion. The row requires real not_queueable and node_depleted, not an active-order table at an arbitrary later screenshot.

Choice: keep real physical Gather then immediate Shift Append before the screenshot. Bind the initial worker's actual Idle/no-current/no-queue state and absence of prior worker commands; exact physical G target release to accepted typed muster:order_gather Node1; then the exact next same-worker Move Append to its actual not_queueable server result. Validate physical modifier, wire sequence/unit/order/target, accepted/refused ticks and no intervening worker replacement or Hold. Orders.rs:2754-2763 makes that refusal direct Gather/queued-Gather evidence once other causes are excluded by the history. Later require the same node remaining0 and the same worker/order's actual node_depleted last_failure, real player text and glyph pixels. Retain actual result/failure ticks and permit same-tick phase ordering; do not infer a vanished active order from the late image. Keep the exact native Gather witness. No bridge, protected content, quantity, timing-policy or player acceptance change.

Evidence: RUN88 actual candidate2 input timings and worker/node/gather definitions; orders.rs:2754-2763; planTask14:632 and design14ORD3. Earlier wrong-UI Gather click remains fixture failure and cannot be turned into a valid Gather event by replay; run the affected full ORD3 pair after source approval. Alternative not taken: force a perpetual Gather, invent per-frame current state, enlarge the protected node, drop refusal/depletion checks or demand an active job after it legally ended. This is brief4 instrument timing/producer interpretation applying existing C279/C289.93/170runs,19/25complete.


## C299: Add actual group, cursor and DOM observations for shell rows

Conflict: Task16 rows require same-frame group state, real engine cursor position, DOM canvas focus/pointer lock and context-menu prevention. Current FrameRecord has selection/dispatch but no groups; CursorFeed has image/context but no position; ContextMenuFeed has no production writer and OS FocusFeed does not prove DOM focus. A generic populated feed would invent evidence.

Choice: add bounded read-only observations at the existing producers. close_core_frame records Core.group_members(1..=9) beside its actual selection/dispatch; cursor production records the actual primary Window cursor_position with its existing icon observation (None remains None, including under lock). Harness checkpoints may read document.activeElement/canvas focus and document.pointerLockElement. A web capture listener may retain bounded actual contextmenu receipts with sequence, target, cancelable and defaultPrevented read after event dispatch; it must not prevent the event or change focus. Read defaultPrevented after the handler, not during the capture callback. Missing/overflow observations fail the relevant evaluator. Do not infer default prevention from a truthy feed or add fake overlays. Missing focus overlay/settings/menu behavior stays explicit baseline failure until implemented. The new fields are private instruments, with unchanged keys, browser behavior and acceptance clauses.

Evidence: T16 core_adapter.rs:142 FrameRecord, :922 close_core_frame, :866 actual GroupsFeed producer; debug_feed.rs:20 CursorFeed; widgets/cursor.rs:178 actual Window::cursor_position; client_web/index.html:43 existing canvas input capture; harness/scene-checkpoints.js:14 actual per-page read-only bridge collection. Alternative not taken: treat OS focus as DOM focus, read defaultPrevented before the handler, fabricate a ContextMenuFeed count or a focus overlay, or relax the rows. This is brief4 instrument detail continuing C292.94/170runs,19/25complete.


## C300: Use the existing bounded Attack wait; judge the exact target from actual evidence

Conflict: ORD4 pair1 completed real accepted Attack but its driver timed out waiting for a five-by-five orange patch to disappear. Both actual targets46 were already removed, and actor40 was Idle with no failure. Surviving51 shares the screenshot column; geometry supports overlapping orange bodies, but post-death polling PNG bytes were not persisted, so exact residual-pixel provenance is NOT TRACED. The driver color patch is not a unit identity and must not overrule actual target evidence.

Choice: replace only waitForPixelGone for this Attack with existing stages.attack_wait_ms=18000 via public io.wait, then capture attack-after-approach and continue the physical driver. Keep the complete unchanged oracle: screenshot-ray exact target, actual accepted per-unit Attack, actual approach/focus/health loss and exact target removal are all mandatory. No semantic driver read, fake success, weaker target guard, product timing change or acceptance-row change. Preserve both original pixel-wait failures; affected new stimulus requires a fresh full ORD4 pair after source check, not replay of incomplete old drivers.

Evidence: orders-driver.js:143 current pixel wait and :239 matcher; ORD4.json:32 existing18000ms parameter; orders-oracle.js:374 actual exact-target removal guard. Root t14-root/ord4-first-pair-actual-kill-audit.json independently verifies real seq1 acceptance97/94, target46 actual40-to5 health histories, first actual absence421/419 (16.2/16.25s after acceptance at20Hz), final Idle/no failure. Wrapper4e0def78 exit1/sourceunchanged; RUN88 independently agrees. Alternative not taken: call a successful kill a product jam, attribute unretained residual pixels as proven, poll private state from the driver, or drop the actual kill guard. This is brief4 fixture/instrument detail, same player-visible Attack.94/170runs,19/25complete.


## C301: Bind the Rush target before damage removes it

Conflict: both complete ORD4 candidate2 drivers finish, but oracle picks nearestForeign from the later engagement screenshot. Actual Rush has already killed its first engaged target45 by then, so the helper selects untouched survivor50 and calls the missing damage a failure. The acceptance row requires stop/fire/resume against the actual engaged target, not whichever surviving enemy is nearest later.

Choice: bind the nearest actual living in-range foreign target from rush-issued before its damage, explicitly verify positive health and range there, then require that SAME target's strict post-acceptance health loss/removal while the actor stays still, followed by real resumed movement toward its point. Retain actual typed Rush, accepted result, all Attack/focus and force-Move clauses. Do not change physical driver/product/timing or choose an arbitrary damaged target after the fact. Freeze source and review positive/negative controls. Since both actual complete drivers retained all artifacts, existing C296 permits full corrected-public-judge replay on both; any missing evidence or failing remaining clause forbids carry. Original pair213d42d8 exit1 remains an instrument failure.

Evidence: orders-oracle.js:388 nearestForeign currently reads engagement; :391 baseline reads rush-issued. Root independently reads complete first-run C84: rush-issued487, actor[51.79256,32.19220], target45 health15 and range2.853; actual target45 health10@488,5@508,0@528; engagement533 has target45 removed and actor stationary; later859 actor moved over1 toward Rush point. Both actual final traces show target45 killed528/527. RUN88 independently confirms wrong survivor50 selected by old helper. Alternative not taken: treat an already killed target as no firing, weaken damage/resumption checks, read private semantics in the driver or rerun intact physical evidence without need. This is brief4 instrument event identity continuing C279/C296.94/170runs,19/25complete.


## C302: Rename an oracle assertion message and carry unaffected Rust gates

Conflict: after Task14 committed da4c9aee passed all G-RUST commands, G-QA driver-rule self-test rejects the English word `require` in feedback-oracle.js:427 (`pixel bounds require a visible component`). This is not a dynamic module load. The scanner remains unchanged.

Choice: replace that message word with `need`, commit the exact one-string change as3ab6600c, and run the full G-QA set on that clean final head. Root independently verifies the entire da4..3ab diff contains only that assertion text; the predicate, JS branches/exports/imports, Rust sources/tests, Cargo manifests/lock, gate scripts, fixtures and protected content are byte-identical. Carry the actual successful da4 G-RUST/boundary receipts with both heads/source delta recorded; do not spend a duplicate workspace build/test for an error-message word. G-WEB has not yet run and must run fresh on3ab; G-REF rebinds the unchanged86path/zero-selection result. MM1/ALR1 actual predicates and all order-row carries stay exact. This does not waive the mandatory fresh integration gates after merge, mutation cycles or final acceptance gates.

Evidence: actual3ab commit diff one string in feedback-oracle.js:427; scripts/qa/driver_rule.py:166 lexical require scan; actual da4 committed receipts t14-commit-{fmt,workspace-clippy,wasm-check,workspace-test,boundary} all exit0/sourceunchanged, raw workspace-test d93f147a, Clippy9497f97e, WASM47373134, boundary72047afc. Original G-QA f684e6ae exit1 retained; narrow dirty-source checkea99b38a exit0 is not final-head QA. Alternative not taken: weaken DR5, change static require calls, conceal the old QA failure, rerun unrelated unchanged Rust work or skip fresh G-WEB/integration gates. This is brief4 name/instrument collision plus CEO-approved grouping of redundant tests.94/170runs,19/25complete.


## C303: Use an isolated authored combat fixture for SEL4 death pruning

Conflict: SEL4 requires a real selected/grouped unit to be destroyed and pruned in the same frame. The preparation row picked default Forge, whose slot0 has no mobile health unit and whose turret needs ammo. Inventing a death bridge or treating a stale group edit as destruction would not test that row. The design row names no package or unit type.

Choice: prepare a separate exact Muster fixture copy, preserving all original unit definitions/assets/placements. Append one slot1 authored trooper at[18.5,12]; keep protected mods/muster and mods/forge unchanged. Select original workers41[10,12] and42[12,12], exercise assign/recall/add/center, then select41 alone and physically Move it to[16,10]. Actual existing enemy Idle combat can kill41;42 stays safe. Enemy initially sits4.5 from the closest depot[14,12], beyond actual center-distance range4; worker42 stays6.5 away, while the moved worker ends about3.20 away. Destination is about2.83 from depot, clear of its0.75+worker0.45 footprint sum. This corrects the initially considered[17,12] enemy (would shoot depot) and[15,12] destination (overlaps depot).

Capture bounded physical screenshots/C84 through real death; require consecutive actual Core frame identities/replica ticks and actual disappearance plus selection/group pruning in the same record, then recall/reload the surviving group. Verify actual joined unit identities/positions and source/fixture hashes during authorized diagnostic calibration; the driver uses published physical IO only. No forced kill, private driver read, altered health/damage, new observer seam or changed acceptance clause. Existing C299 frame observations suffice in source; actual phase/death/timing/pixels remain NOT TRACED until runtime.

Evidence: design14 rowSEL4:1234; Muster maps/field.ron existing worker/depot placements; units/trooper.ron range4/damage5/cooldown1000/Other; units/worker.ron HP30/radius0.45; depot.ron radius0.75/HP100; combat.rs:298-327 actual no-order target selection uses center distance <=range and Other owner. RUN94 source feasibility confirms same-frame frame/group fields. Six shots are computed, not observed. Alternative not taken: modify protected content, preload Forge economy, fabricate a death, select whichever unit later disappears or assert a timing not measured. This is brief4 fixture/mechanism detail with identical player group controls.94/170runs,19/25complete.


## C304: Export actual engine-menu UI through the existing snapshot

Conflict: Task16 must verify the real engine menu. Current widgets/snapshot.rs exports authored widgets and ClientNotice, but SettingsMenuOverlay lacks AuthoredWidgetId, so the existing bridge cannot report that actual tree. A boolean derived from a menu action would not prove visible UI.

Choice: extend only the existing UiSnapshotFeed snapshot path with a bounded record for the actual SettingsMenuOverlay component and its real descendants' Text and computed layout/visibility/capture state. Make the existing marker crate-visible if required. Keep actual existing physical-pixel rectangle semantics; missing ComputedNode/transform means null/false, not invented geometry. Bound traversal and report overflow explicitly for fail-closed evaluation; preserve all current authored-widget/notice rows. No new endpoint, semantic driver read, synthetic menu flag, product UI behavior or acceptance change. A menu action or OS focus feed alone cannot satisfy the browser menu/overlay clauses.

Evidence: T16 widgets/snapshot.rs:11 real ComputedNode/UiGlobalTransform export and :40 AuthoredWidgetId-only filter; client_shell.rs:20 SettingsMenuOverlay and :229 actual root/capture components; debug.rs:105 existing UiSnapshotFeed bridge. Alternative not taken: fabricate visible labels/rectangles from settings data, add another bridge, treat native tree tests as browser pixels or drop the actual menu checks. This is brief4 instrument detail continuing C292/C299.94/170runs,19/25complete.


## C305: Compare the saved audio value at its actual f32 precision

Conflict: the new native settings-preservation contract compares the JSON master-volume number with exact f64 literal0.6. Actual unchanged Task15 behavior starts from f32 volume0.6 and saves0.6000000238418579. This is a test precision mismatch, not lost volume or settings.

Choice: compare with exact `f64::from(0.6_f32)` while retaining exact unknown-field and other-category assertions. Do not change serialization, add an arbitrary tolerance, or count the old mismatch as a product red. Preserve original source/raw; rerun the same persist target. It now has5pass/9genuine missing-feature failures instead of4pass/10fail. No product behavior changed.

Evidence: crates/turts_client/tests/persist.rs:105 actual SettingsStore.set_volume and :111 corrected numeric assertion; initial native persist raw under t16-persist-baseline-run94-v1, rerunv2 rawac9aa4e621cbd323e906fea51492d1af73597fb4352931dbc5971d5a9f7dffd0. Alternative not taken: manufacture a Task15 regression, hide the old result, round stored data or loosen the acceptance values. This is brief4 instrument numeric precision detail, same volume behavior.95/170runs,20/25complete.


## C306: Retain bounded real browser setup diagnostics

Conflict: first guarded SLOT1 attempt runs headless and returns black screenshots; corrected headed attempt times out before unclaimed C readiness, with only A/B server additions. Current harness retains no console/pageerror/request-failure events, so it cannot distinguish a client startup, fetch, GPU or readiness problem. Neither attempt is a product red/green.

Choice: add bounded harness-owned per-page diagnostics for actual console, pageerror and failed-request events, bound to opaque window/document epoch and real host time. Record counts/limits/truncation and actual failure text/location; no bodies, credentials, new product bridge or driver method. Write outside the served tree with private mode0600 and flush at setup/driver failure and completion. If startup fails, retain actual screenshot and existing bridge/page-ready/visibility state when available, or the real capture/read error; never fabricate a blank image or completed checkpoint. Preserve original attempts/profiles and source identity. No product/browser behavior or acceptance clause changes. Correct actual headed/Vulkan launch remains the established contract; labels are not proof.

Evidence: t16-builder/run94/physical-slot1-v3 actual86inputs/12blackcaptures and headless:true; t16-slot1-physical-run94-v3 guardedoracle rejection; corrected headed timeout `unclaimed-third readiness` after30s, serverobserver onlyA/B, no C startup boundary. Task16 harness/run.js owns page creation/startup and already collects private C84 checkpoints. Alternative not taken: call setup a K27/product failure, guess a shader fix, weaken readiness/deadline, add semantic driver reads or claim black pixels as UI proof. This is brief4 instrument detail continuing C268/C294.95/170runs,20/25complete.


## C307: Use the shared empty-aware native configuration root

Conflict: the new persistence helper ignores an empty XDG_CONFIG_HOME, but the actual production volume resolver accepts it and builds a relative turts/settings.json path. RUN96 reports Medium M96-1 against the frozen source. Normal nonempty roots are unchanged.

Choice: repair now. Route the production resolver through the shared empty-aware helper. Test the production resolver with injected empty XDG and an injected HOME; do not mutate process HOME/XDG. Preserve the failing contract and rerun it after the fix. This is a Medium repair under brief4; no player-visible design or acceptance change.

Evidence: crates/turts_client/src/volume_settings.rs:379-405 calls and tests native_user_config_dir_from; crates/turts_client/src/persist.rs:152 provides native_config_dir_from. Actual red raw run94-native-xdg-c307-red-v1/command.log SHA7c543233, exit101; same contract green run94-native-xdg-c307-green-v1, rawc9c57661, exit0. Full receipts and diff are in primary target/mc-evidence/t16-builder/run94/native-xdg-c307-v1. No process environment mutation; source unchanged within each command. Combined native lib later155pass/1expected missing-menu failure, so its persist target did not execute; prior isolated persist14/14 remains its own proof.

Alternative not taken: defer a known production path defect, count helper-only tests as production wiring proof, change ordinary roots, or mutate global environment for tests.96/170runs;20/25complete.


## C308: Observe the actual browser GPU device promise without settling it

Conflict: actual headed SLOT1 C times out before first Last-published state. C has a canvas and logs BrowserWebGPU adapter selection, but no Lightyear connection; the existing bridges still return their initial empty values. The healthy single-window headed HUD2 control does not establish the cause of this three-window startup failure. Device request settlement is NOT TRACED.

Choice: authorize a default-off, bounded harness diagnostic around the actual GPUAdapter.requestDevice call. Reflect.apply preserves the original receiver and arguments; return the exact original promise and rethrow synchronous errors unchanged. Do not attach then/catch handlers, read descriptor getters, change GPU flags/limits or await/replace/settle the promise. At setup failure/completion, Chromium CDP Runtime.evaluate with awaitPromise:false plus Runtime.getProperties may observe the actual promise internal [[PromiseState]]. Record call time, observation time and actual state or actual unsupported/error. Exact settlement time remains NOT TRACED. Keep private bounded records outside served files, mode0600, with explicit overflow; no new product bridge or driver method. Pure controls must verify arguments, receiver, return identity and rethrow behavior.

Evidence: tests/browser/harness/run.js:180-195 creates the actual windows and waits for typed startup. Current C306 diagnostics collect real console/pageerror/requestfailed events at:356-364 and setup-failure pages at:438-449. Actual run t16-builder/run94/physical-slot1-headed-v2/run-20261110T170016Z-8oU4Hl retains C white pixels and empty bridge reads, 27console events/no overflow/no pageerror/requestfailed, server A/B only. Unchanged crates/turts_client_web/src/lib.rs first publication occurs in Last; installed Bevy renderer logs adapter selection before awaiting request_device. This is a causal hypothesis to measure, not a product verdict.

Alternative not taken: raise readiness or reconnect bounds, guess a shader/network repair, add promise handlers that alter rejection observation, change device selection, or count setup failure as K27/product red. Brief4 instrument detail.96/170runs;20/25complete.


## C309: Keep the complete menu snapshot bounded at 256 text nodes

Conflict: the complete engine-and-mod action list makes the actual menu exceed the original C304 snapshot cap of128 text nodes. A native check exposes the truncation. Omitting engine actions would violate design10.2; an unbounded bridge would violate the evidence contract.

Choice: raise only MAX_SETTINGS_MENU_TEXT_NODES to256. Keep the four-root and512-descendant bounds, per-node actual layout/visibility, explicit text_truncated/descendants_truncated and root overflow records. The menu remains one screen with a bounded normal ScrollArea; every effective action remains present. No new driver method, product bridge, acceptance condition, key or tab.

Evidence: crates/turts_client/src/widgets/snapshot.rs:11-13 and:43-55; actual cap128 intermediate failure retained by RUN94, then current native library160/160 passes raw8e83bbecffe993fb7c983c9842feb083dbd759700afa887995f88121e9a3dbbc exit0, including root-overflow, full-list, first-snapshot and capture isolation checks. Native persistence15/15 raw0bd689c7 exit0, locked wasm check95b2358b exit0. Root verifies all three raw hashes/source_unchanged/default-cache-absent receipts. Browser geometry/scrolling and full Task16 acceptance remain NOT TRACED.

Alternative not taken: remove engine defaults, hide truncation, publish unbounded descendants, fabricate positive layout, or add menu tabs/keys. This is brief4 instrument-detail/nearest-design implementation choice continuing C304.96/170runs;20/25complete.


## C310: Preserve browser drafts and require actual semantic browser baselines

Conflict: Task16 plan step1 expects old no-menu/random-id/no-focus failures, but the current source preserves advanced Task15/rejoin behavior and now has genuine native red-to-green menu/persistence work. No genuine Task16 browser semantic baseline is recorded yet. Existing instrument_status guards reject before real assertions; NOT READY clauses and third-window startup failures cannot supply that missing baseline. RUN94 began preparing browser storage code too early.

Choice: stop only dependent browser-feature edits, preserve their unexecuted draft separately, and restore exact pre-web native source before actual headed browser baseline attempts. Keep legitimate native work tied to its real native reds; do not manufacture an older random-id/no-retry/no-menu regression. Run all seven actual physical plans against this stated source. Record the actual missing feature assertion and source/version, not the stale expected text. A guard, NOT READY placeholder, missing artifact, compile or startup failure remains setup/instrument-only. Qualify runnable baseline plans without dropping any final fixed clause. All seven complete semantic/pixel checks still precede commit/gates and the nine later required plants; no acceptance waiver is granted. If a current row first passes, record that fact and its actual existing proof lineage without inventing an initial red, and review its causal coverage explicitly.

Evidence: shell-oracle.js:110-115 rejects preparation status before assertions and:187-190 rejects unqualified fixed clauses; shell-driver.js:13-16 likewise guards execution. Every current row retains preparation/candidate status. Prior SLOT1 headedv2 has setup timeout before first Last state; HUD2/bare-device controls are not Task16 baselines. Root checks preserved web draft471e88ada82a0465bb2d6ccac486e988347ff1ba88b5e233a387bf4e4b7e81fa and actual restored persist.rs dd79df35f0fc8dd5ff9e71c046048fa3640fa425766f42a9fa337ba28452be82 against the retained baseline. Draft/diff/original are primary t16-builder/run94/web-storage-preserved-v1. No web draft runtime green or browser-red credit claimed.

Alternative not taken: relabel setup/guard failure as product red, erase legitimate native work, apply untested browser features before baseline, fabricate old behavior/history, or relax acceptance. This is brief4 instrument/step-order correction following the actual source; no player-visible design or K call changes.96/170runs;20/25complete.


## C311: Keep root Python checks from writing into an active lane

Conflict: root imported the lane parameter module for an external KM2 proposal without disabling Python bytecode. That wrote scripts/__pycache__/mc_browser_params...pyc during RUN94's immutable F10 diagnostic. The browser command succeeded, but the enforcing wrapper correctly reports source_unchanged=false/wrapperexit2. This was root's error, not a product failure.

Choice: preserve the original receipt/delta and archived bytecode, remove only that rebuildable file (RUN94 completed this), and run every future root repository Python import with PYTHONDONTWRITEBYTECODE=1 and sys.dont_write_bytecode=True before the import. Do not weaken the source manifest or relabel the old wrapper outcome. RUN94 already ran a fresh clean F10 diagnostic; use its actual result for calibration. No product baseline or acceptance credit from either diagnostic.

Evidence: run94-menu-physical-diagnostic-v2 innerexit0/wrapperexit2, raw457cb426e53716e7b8a06d261f3f652b012859047aaa6317256b30f1161ba299; archived bytecode under run94-ui-menu-diagnostic-v2/rebuildable-python-artifacts. Clean v3 receipt source_unchanged=true/command0/wrapper0 rawc36ef703ae6731e311a1594f4106669d8d10fd0fb74ffc6bcc140fa206d46e2d; actual settings-menu PNG47636414. Root verifies all raw hashes. Actual labels overlap/wrap in132px column; normal layout repair is in progress. Wheel diagnostic observes51action rows/51Change/51Reset,212textnodes/378descendants, no truncation; source bounds256/512 stand.

Alternative not taken: blame product/source drift, exclude arbitrary files from the immutable gate, delete evidence, or count a diagnostic as a Task16 row. This is brief4 instrument/cache hygiene;96/170runs;20/25complete.


## C312: Read actual GPU requirement data through CDP without invoking getters

Conflict: C308 now observes actual A/B requestDevice fulfilled and C pending after30s, while all three default bare requests fulfill. Required features/limits are NOT TRACED, so the default control cannot isolate a Bevy requirement or application-load difference. C308 intentionally excluded argument inspection.

Choice: extend only the same default-off guarded harness diagnostic. After Reflect.apply returns, retain the exact argument0 reference in the private bounded call record, without reading its fields or changing arguments/receiver/returned promise. Through CDP Runtime.getProperties ownProperties:true/generatePreview:false, inspect ONLY data-property descriptors for requiredFeatures and requiredLimits. Never call getters, iterate arbitrary iterables, inspect labels/defaultQueue/PromiseResult, attach promise handlers or mutate device selection/limits/features. Bound feature/limit entries and string lengths, record accessor/unsupported/error/overflow as NOT TRACED. Actual call/observation time and window/document identity remain bound. Release all object handles. Pure controls must prove no getter/iterator access and unchanged receiver/arguments/promise/error. A later external bare replay may use the captured plain primitive requirements to compare three devices without game code; it is diagnostic only, never a Task16 acceptance row.

Evidence: installed Cargo-locked wgpu30.0.1 backend/webgpu.rs:1780-1806 constructs GpuDeviceDescriptor::new, sets numeric limits and a feature-name vector, then calls request_device_with_descriptor. Its generated gen_GpuDeviceDescriptor.rs:136-138 creates plain js_sys::Object::new; map_js_sys_limits at:942 creates a plain typed object and writes numeric properties. Thus actual application descriptors are source-traced plain data; generic accessors still fail closed. Current actual SLOT1 audit run94-slot1-headed-c308-v2/runs/run-20261110T180056Z-1OPzLI/gpu-device-audit.json has installed/nooverflow/noerrors, A/B fulfilled/Cpending; immutable command receipt is run94-slot1-headed-current-source-c308-v2, raw7d32eb52, exit1 due readiness timeout. Actual settlement time/cause remain NOT TRACED.

Alternative not taken: change the renderer/profile on a guess, raise readiness/rejoin deadlines, read arbitrary request getters or silently treat default bare devices as equivalent to Bevy requests. This narrows C308's earlier no-argument-observation rule only for bounded getter-free private CDP data observation. No product code/acceptance/player-visible change; brief4 instrument detail.96/170runs;20/25complete.
