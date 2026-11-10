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
