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
