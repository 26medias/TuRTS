---
type: decision
date: 2026-11-09
tags: [turts, mc, codex, ruling]
---

# MC Codex rulings

## C179: measure the actual hardware WebGPU adapter

Conflict: tests/browser/measure.js:178-198 requires nvidiaAdapter for VIS2 readiness and includes a NVIDIA-specific metadata fallback. Design9.1 requires headed Chrome WebGPU measurements at5000units with no budget; it does not set a GPU vendor. The plan Task13step5 names the desktop RTX3080 as the machine record. The laptop handoff requires honest actual hardware metadata.

Choice: remove the vendor as a pass condition. Keep actual complete adapter metadata, a configured acquired device/context, actual submitted draw calls and texture acquisitions, WebGPU backend, a non-software adapter, headed mode,5000replicated and rendered units, and finite positive frame/memory metrics. NVIDIA may remain diagnostic data. Test alternate hardware vendors and rejected software/incomplete/device-less evidence with bounded pure controls; actual headed measurement is still mandatory. No performance threshold, simulated hardware or fallback measurement is authorized.

Evidence: design9.1 draw-method measurement clause; planTask13step5; measure.js:178-198; [[Reports/2026-11-01 MC laptop handoff]]. Alternative not taken: reject valid Intel/AMD hardware, disguise software rendering as GPU proof, delete adapter metadata, or change the acceptance row.68/170runs;15/25complete.


## C180: retain both work-log histories when importing external docs

Conflict: main advanced externally fromf050e42 tobb611bd through two documentation-only commits. A read-only merge-tree check found one content conflict in vault/Work Log/2026-11-01.md, which would prevent the draft PR merge ref. The incoming side adds CI/transfer history; integration has the full execution log.

Choice: merge main INTO codex/mc-integration, preserve both work-log additions and the externally committed disk audit, then continue the active Task13 lane unchanged. Runtime/design/plan/frozen files are byte-identical before and after this merge. This is branch/document maintenance under brief4, not a merge into main. Normal integration re-gate still follows the later Task13 code merge, before Wave6 push/read. No existing runtime observation is relabelled.

Evidence: main commits eb8d482 andbb611bd; git diff f050e42..main changes only the work log and disk audit; merge-tree reports that one conflict. Alternative not taken: drop either history, alter main, rebase an immutable active runtime tree, or push a PR with an unresolved documentation conflict.68/170runs.


### C179 actual browser schema clarification

Root traced the browser adapter field to turts_scene().render_adapter, not turts_debug().render_adapter. Web scene serialization exposes backend/name/device_type; the richer vendor/device native live-view schema is a different path. Actual historical0c26fab C84 confirms debug has no adapter and scene gives BrowserWebGpu/Other with an empty name. Do not require a fabricated Bevy name. Read the real scene backend and retain its honest fields, then bind hardware identity to the independent actual configured-device WebGPU API adapterInfo already captured by the prototype hook. Reject explicit Cpu/software/SwiftShader/llvmpipe/lavapipe and missing independent hardware metadata. No new debug fields or guessed names. Evidence: crates/turts_client_web/src/lib.rs:615-619; measure.js current bevyRenderAdapter assignment; root t13-root/resume-20261109/actual-web-adapter-schema.json. This clarifies the instrument repair before actual measurement; it is not a measurement pass.


## C181: keep the Gather dot visible on its resource node

Conflict: design9.1 requires a Gather dot at the node in the click frame. The prior full VIS1 driver did not test Gather. New1b649dc actual headed captures show worker2 Gathering node1 at25,10 with fresh intent3 in appliedseq38, but the ordinary depth-tested dot is below the node's larger cuboid. The full row itself stops earlier on an older-move-marker oracle binding error; it does not earn a full-row Gather failure.

Root ran the unchanged Gather pixel helper over the actual saved same-pose images, using only an appended private export to call the existing function. No assertion/image/C84 body was changed. It fails exit1 with observed0changed dot pixels. Pre-Gather and immediate PNG bytes are identical; node/camera state is equal; root manually viewed the unobscured screenshot. This is a focused actual-capture regression, not a passing row or a relabelled earlier failure.

Choice: use one globally shared marker-colour ExtendedMaterial overlay with the existing C175 depthAlways/writefalse mechanism, existing bar mesh, exact0.16scale and exact target/colour. No per-unit assets, extra mesh, content/input/acceptance changes or fabricated marker. Add immediate AND pointer-away pixel checks, real selected worker identity and node pre/post identity/position controls. Repair the separate waypoint instrument binding to the marker for that physical click and fresh intent rather than the first retained same-kind marker. Fresh full VIS1 remains mandatory after the production repair.

Evidence: visuals.rs Gather branch; design9.1; actual1b649dc run20261109T154904Z-nklVLy; root t13-root/resume-20261109/gather-actual-pixel-red.json records input/source/image hashes and exact failure. Alternative not taken: accept marker metadata without pixels, change the node or target/shape, skip Gather, weaken the pixel requirement, or claim the full row reached a later assertion.68/170runs;15/25complete.


## C182: carry exact-input visual and measurement evidence through record-only changes

Conflict: the VIS2 capture/record cycle ran before Task13 final gates/plants, while the plan lists the record cycle in step5. The actual9cf2363 fullVIS1 pair and actual5000-unit measurement are valid, but later commits change measurement documentation and SEL2 instruments. Repeating identical renders would not exercise those changed paths. The earlier broad-grep green is explicitly superseded; it never proves the required record schema.

Choice: retain the actual9cf source identity and treat its measurement as provisional until gates/plants are complete. After exact plant restoration, verify that runtime, measurement program/hook/validator/profile/fixture inputs still match9cf; then replay the exact record red against the committed empty VIS2 baseline and green against the final document, and finalize its validation record. If those runtime/measurement inputs change, the carry expires and a fresh measurement is required. No old run is relabelled as execution on a newer commit.

The same exact-input check may carry9cf fullVIS1 through docs and SEL2-only edits, saving a duplicate normal pair if its impact batch has not started. The independent actual foreign-solid production plant still needs its compiled red and actual restored fullVIS1 green on the final candidate. SEL2/SEL3 changed instruments, affected CAM1/HUD1/HUD4, all normal task gates, required plants, independent review, integration re-gate and Wave6 hosted CI remain mandatory.

Evidence: root git diff9cf2363..59f2dbe lists only docs/measurements/mc.md, sel-driver.js and sel-oracle.js; runtime/design/fixture/VIS1/measurement/shared-harness input diff exits0. Existing C150/C151 source-impact carry and the CEO-approved test grouping apply. Actual fullVIS1 two0 and actualVIS2 data retain original receipts/hashes; corrected exact table has four rows. Alternative not taken: silently reorder/claim acceptance early, skip mutation restoration or final source binding, rerun unaffected renders only to change the printed commit, or carry across a runtime change.69/170runs;15/25complete.


## C183: distinguish the initial Scout silhouette from a small HUD glyph

Conflict: CAM1 recognition fails despite real Joined Scout1 at448.68,213.67. The shared recognizer already accepts warm and blue materials. Actual diagnostic PNGs show a183pixel14x16Scout plus a14pixel5x5blueHUDglyph; its old12pixel/3x3floor makes the unique-component rule ambiguous. Failure was setup, not a camera semantic red.

Choice: retain both colour predicates, unique eligible-component rule and exact silhouette-foot targeting; raise the eligible initial body floor to50pixels and8x8. Both callers CAM1/CLICK9 use it only at the same Acme initial camera, before zoom. Add the named glyph regression and preserve failure screenshots. Keep all camera/selection/timing assertions and fixture/content unchanged. Run CAM1 and the shared-helper-affected CLICK9 defined viewport checks; no unrelated image matrix is added.

Evidence: actual failure PNGs under t13-cam1-recognition-diagnostic-fb3d5eb-20261109; helper/callers; pure oldred1/new7pass; clean41987afa8e287c9750df436a9196351b90f004f4 CAM1two0 and CLICK9two0 independently audited. Source has no coordinate shortcut or debug input targeting. Alternative not taken: widen colours without evidence, choose the largest of two real unit-sized candidates, change the camera/content, weaken oracles, or accept a recognition failure as semantic red.69/170runs.

### C182 native lint changes expire the exact VIS2 carry

The later strict Clippy gate found routine lints in runtime visuals/debug source. It failed101 before subsequent gates; that is not a semantic red. Any runtime Rust source repair expires C182 exact-input VIS2 carry. Run fresh actual5000-unit measurement AFTER final gates and restored plants on final source, then finalize the record. Preserve old9cf measurement as historical. The earlier provisional capture is not relabelled. No benchmark rerun is needed merely for a record-format change, but the runtime-source change is a new reason.


## C184: allow the required external material field as an identifier only

Conflict: full workspace tests at203ec2a reach a real-source no-content failure: external Bevy0.20 ExtendedMaterial requires a field literally named base, which collides with the derived base package token. The pinned API has only public base/extension fields and Default, with no equivalent positional constructor. Renaming an engine local can fix local collisions, but cannot rename that external field. O10 kept the scanner unchanged for its ShellAction resolution; this is a new external-API collision under the widened brief4 mechanism rights.

Choice: use the scanner's existing provenance-backed generic-word mechanism with exactly base and identifiers_only:true, as already supported for credit. This changes the exception catalog, not literal/full-ID/path derivation or scanning. Rename replaceable production locals to slot_material/first_vertex. Keep bare string and byte-string base, full IDs such as base:rifleman/base:credits, and resource paths forbidden. Add explicit bare literal category assertions; after final normal gates, flip only this exception's identifiers_only flag false and require that boundary test to fail, then exact SHA restore/green. Run newly affected registered E1-derived-scan through its unchanged controller and preserve existing plants. No mod/content/frozen value is renamed or changed.

Evidence: pinned bevy_pbr0.20 extended_material.rs:151-168; support/mod.rs generic-word exception/provenance and distinct check_ident/check literal paths; compiled real-source scanner failure; ae6030a focused no_content9green; RUN69 review requests bare-literal controls. Source/gates remain provisional until those controls and exact restoration complete. Alternative not taken: exempt literals/full IDs/paths, encode or hide the field spelling, rename protected content, make a blanket source-file bypass, or duplicate the upstream Material implementation solely to avoid a generic external field name. This preserves the engine's content-blind rule; it is not a decision that a K call is wrong.69/170runs.


## C185: judge the actual first-release ring assertion for the first box

Conflict: planTask13step4 spells the delay failure as rings0 in the mouse-up frame. The implemented oracle names the exact case/first pointer-release actual rendered ring count and then prints0!==500. The planted delay stores the result for the next Core.step but uses mem::take before box_select, which also changes later Shift-add behavior.

Choice: retain the existing precise assertion text and credit ONLY the first non-Shift500-unit box. Both actual C84 traces prove preselection0, release selection/rings0, and exact nextframe selection/rings500: seq13, releaseframes163/148. In this first box take and clone are equivalent; the later Shift side effect cannot explain or alter the already-recorded first-case transition and is not credited. No repeated GPU run is needed to test a side effect outside the asserted failure. Preserve the complete patch/captures, require exact original SHA restoration and selected SEL2 green, and make no claim about Shift under planted code.

Evidence: planTask13step4; sel-oracle.js ring assertion before selection assertion; CoreAdapter has one step call per Update; root t13-root/resume-20261109/one-frame-first-case-witness.json with raw checkpoint/input hashes; actual firststage data. Alternative not taken: count later500 as firstframe success, use a dropped selection instead of a real delay, weaken ring checks, claim later Shift correctness, or rewrite a valid error solely to match prose. This is an instrument/scope ruling under brief4; acceptance/timing/ring count do not change.69/170runs; exact restoredgreen still pending at this note.


## C186: Isolate headed browser input from the desktop

Conflict: a real restored-source SEL2 run failed Shift-add1!=3 while later runs on the same source and WASM passed. The raw ambient log shows unrelated unsequenced pointer moves with Shift=false during the Shift gesture. The intended input log has ShiftDown25 and Shift=true down27/up36. This is an unresolved failure; a retry does not accept it.

Choice: run the remaining headed Task13 browser checks on a dedicated X display through xvfb-run, with the same Chrome hardware WebGPU flags and all original driver/oracle/row assertions. The existing untracked bound runner already inherits DISPLAY (target/mc-evidence/gate-runner/run_mc_rows.py:86); no engine behavior, row, or modifier rule changes. Record the actual display, browser profile, configured-device metadata and scene adapter. Reject software/CPU adapters. Environment isolation is allowed by brief section4 instrument/cache/layout rights. The original failure and diagnostic stay in the evidence; do not relabel them green.

Evidence: tests/browser/harness/run.js:145 records unmatched physical input as ambient without consuming it. crates/turts_client/src/core_adapter.rs:596 consumes real or synthesized KeyboardInput and removes the released key. crates/turts_interaction/src/lib.rs:121 removes its held modifier action on KeyUp. Installed winit web event_loop/window_target.rs:264 sends ModifiersChanged from pointer moves; installed bevy_winit0.20 state.rs:815 reconciles recorded modifiers and can synthesize Released. On Web, unknown modifier sides cannot synthesize the corresponding Pressed solely from a later pointer event. Root preserved exact upstream source paths/hashes and actual failing-event chronology in t13-root/desktop-resume-20261109/shift-add-causal-source-lead.json; RUN71 independently reviewed the same source mechanism. The actual old-run synthesized KeyUp was not logged and remains an inference. RUN70 must complete the bounded actual untagged modifiers0 pointer experiment with frame separation, report the outcome honestly, then demonstrate clean isolated SEL2 and actual hardware proof. Those new measured results are NOT TRACED at this ruling.

Alternative not taken: weaken the selection oracle, accept retries as resolution, suppress real modifier release in production, or change player-visible modifier behavior. Native gates on unchanged runtime source can retain their original source identity; source/runtime changes require appropriate fresh gates. Isolated-display performance measurements must name that environment and use actual fresh metrics.


C186 measured result: two bounded actual headed reproductions at unchanged40b9d90 fail the expected Shift-add1!=3 assertion; actual injected false event, frame separation, unchangedcamera and true subsequent pointerflags are preserved and independently audited. Driver harness restored byte-for-byte, clean. The oldrun actual synthesizedKeyUp remains unlogged/inferred. Clean isolatedSEL2 and same-profile configured hardware proof remain pending. Rawreceipt t13-shift-ambient-diagnostic-40b9d90-20261109/receipt.json; rootaudit shift-add-causal-runtime-audit.json.


C186finalhardwareclosure: freshactualVIS2 under Xvfb100 used actualconfiguredNVIDIA/Ampere device1 with5000replicated/rendered units,89textureacquisitions/178submissions/0instrumenterrors. Source40b and record9ae. Root/RUN71verifiedraw; C186hardwarependingrequirement fulfilled.


## C187: Reuse frozen suites from the full workspace gate

Conflict: the native gate helper runs cargo test --workspace --locked, then runs the same forge_cases, forge_fixed_point and m3_m2_stream suites again for G-FROZEN on unchanged source. The CEO approved removal of redundant execution while retaining quality. The current Task13 integration run completed both forms successfully; the second invocation cost40.234s.

Choice: future native gates may use integration_gates_v2.py with reuse-frozen-workspace. The full workspace command still executes after every merge. G-FROZEN reuses only the three actual named suite results from that same successful workspace execution, requires each suite exactly once with positive passed count, zero failures and zero ignored tests, and checks the frozen file against the approved SHA. The existing frozen-hash gate still runs. A receipt records actual workspace log hash, suites, source file hash and reused command; it explicitly says no second execution occurred. Each caller must keep source/head unchanged across the gates as before. All other G-RUST/G-QA/G-WEB commands stay required.

Evidence: the real Task13 integration workspace log has forge_cases26/26, forge_fixed_point1/1 and m3_m2_stream1/1. The original repeated frozen command also passed. Actual positive verification plus nine negative controls reject omitted suites, zero executed tests, failed tests and ignored tests: t13-root/desktop-resume-20261109/frozen-reuse-proof-controls.json. New untracked helpers are integration_gates_v2.py and frozen_workspace_proof.py; the original helper and historical evidence stay unchanged.

This is test-instrument command grouping under the CEO speed approval and brief section4. It does not change frozen values or remove a gate or plant. Alternative not taken: skip the frozen assertions, accept filtered/empty/ignored execution, defer integration gates across task merges, or rerun the same deterministic tests without a source change.


## C188: Prepare Wave7 while hosted Wave6 checks run

Conflict: Wave6 is fully merged and locally re-gated, pushed588dcd3, and actual hosted run42681727530 has been read. Agents passed; check and browser are still running. Waiting without local work would leave the sole local compiler idle.

Choice: begin Wave7 in two lanes from the current integration after all Needs merged. RUN72 Task10 receives the sole local compiler/cache/runtime lease. RUN73 Task15 prepares red tests/instruments/source plans in its own worktree only, with no Cargo, browser, server or production behavior before its real red. Hosted workers are independent of the single local compiler. Read and record Wave6 terminal jobs as they finish; do not call pending jobs green. Do not merge any Wave7 lane before the terminal Wave6 results are read and classified. A new failure outside the approved Task12-to21 window gets priority repair and dependent rebasing. No gate, dependency, task, or wave push/read is omitted.

Evidence: Task13 merge60082da passed all10native gates and headedT7; push588dcd3 contains the identical runtime source plus vault records and a C157-pattern empty CI trigger. Live jobs were read at19:18UTC: agents success, check/browser in progress (target/mc-evidence/wave6-ci/initial-read.json). The brief section4 permits lane/cache layout decisions; the CEO approved safe concurrency and avoiding redundant waits. Alternative not taken: claim hosted green early, merge later-wave work before reading terminal results, or run parallel local compilers/caches.
