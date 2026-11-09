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


## C189: Close Task15 audio, menu and fixture seams

Conflict: AUD1 belongs to Task15 and requires real category gain changes that survive reload. Design9.5/10.1 places the four volumes in the F10/Esc menu, but Task16 implements that menu and persistence after Task15. A hidden test setter would not exercise the designed player path. AudioFeed also lacks an entity count for the explicit no-player-while-suspended assertion, and Muster has damage/destruction rules but lacks definitions for the other required alert triggers.

Choice:

- Advance only the already designed audio-volume portion of client_shell.rs and persist.rs into Task15. The same existing F10/Esc menu action opens the real designed menu with master/voice/alerts/effects controls; Task16 extends that one menu with the remaining designed controls and retains all its acceptance ownership. Store volumes under the exact turts.v1.settings/native settings.json contract, preserve unrelated settings, use isolated native test stores, and obey defaults plus one notice on corrupt settings. No new key, screen, separate settings path or removed step. Task16 baseline may now fail at the missing remaining controls rather than the plan's literal no-menu/no-store diagnostic; record inherited green coverage honestly and do not manufacture a red.
- Add a measured audio-player count to the observational audio feed/web debug bridge, derived from actual audio entities. Real context state, real OGG playback and actual gain remain the browser proof. A dedicated pre-WASM resume script and a pure Node unit test are allowed instruments; the Node fake-context red is not the actual AUD1 browser result.
- Add a controlled browser fixture package copied from Muster, with explicit alert rules and existing real assets/sounds, if no existing fixture overlay provides the rules. Keep mods/muster and protected Forge content unchanged. Use a nested fixture whose package-directory basename matches its manifest id. Drivers act through io; do not force client debug/replica fields from a driver. Native forced-record tests remain permitted as written. Browser alerts must derive from accepted fixture content plus real authoritative state/transitions.

Evidence: design9.5:917-931, menu10.1:945, persistence10.4:963-978, AUD1:1259; plan Task15 precedes Task16 and Task16 needs15. Existing client_shell.rs/persist.rs are empty registration stubs. debug_feed.rs:38-43 has context_state/started/categories but no entity count. Muster assets/interface provide OGG responses and two existing damage/destruction rules (mods/muster/interface/hud.ron:18-20); the fixture extends remaining trigger coverage. This is an uncovered fixture/instrument/dependency mechanism resolved by advancing the minimum existing designed behavior; full scope and player-visible shape stay intact. Root owns the small cross-task ownership change and later Task16 handoff.

Alternative not taken: defer or weaken AUD1, create an acceptance cycle, add a hidden debug setter or extra player control, or declare audible/persistent behavior from feed values alone. RUN73 remains source-preparation only until its exclusive runtime lease; actual compiled native red precedes corresponding native production behavior.

## C190: Retain the established HD1 CLI error wording

Conflict: Task10 Step1 describes unknown option --actions, but the current parser reports unknown argument --actions. Baseline c4afbc9 crates/turts_client/src/options.rs:219 returns unknown argument for unrecognized input.

Choice: retain that established CLI diagnostic. RUN72's compiled real-server HD1 test failed with unknown argument --actions and exit101 before implementation. This is the same missing-feature failure; do not alter unrelated CLI wording to match plan prose. Preserve the actual output/exit and require real action behavior for green. No acceptance or player control changes.


C189 evidence correction: root initially carried an unverified source-summary claim that Muster had no alert rules. Direct inspection confirms OwnUnitDamaged and OwnUnitDestroyed at hud.ron:19-20. Corrected the ruling to require only missing trigger coverage; preserve those existing rules. The volume-menu/persistence and observed-player-count decisions are unchanged. No runtime or mod content was changed by this correction.


## C191: Complete HD1 semantic orders through the real core

Conflict: design2.3:279-290 names an order by id without a physical key. RUN72's initial hook required an unrelated key and could execute the wrong order. Full-behavior coverage also needs Content/Job targets; core explicit_target currently has no such pointer conversion (interaction/lib.rs:645). Existing browser Produce/Cancel widgets dispatch validated ClientCommand directly (widgets/mod.rs:1514-1588), unlike ordinary Order widgets, which already enqueue UiAction (1505-1508). A private physical-key map alone would not cover unbound UI-only orders or those targets. Plan Task10's narrow file list cannot meet the design through the existing stub alone.

Choice: ordinary semantic order ids use the existing UiAction InputEvent and real core dispatch, followed by projected physical pointer events for world targets. No extra physical key is required; reject or validate any optional key rather than silently execute a different order. Read accepted public catalogue data for validation if needed; avoid a duplicate private binding catalogue in CoreRuntime when the existing semantic UI action already does the work.

For Content/Job targets, add a minimal typed UI-order InputEvent and common core composition route, carrying the existing UI semantic target and the intended unit(s). Route the existing Produce/Cancel widget activation through that same core event after preserving its existing capability/ownership/queue guards and selected-producer semantics. The headless script maps named Content/Job actions into the same event; world-point actions retain actual projection/hit-test input. All actual envelopes still come from the core and go through submit_order/network/server/results. Do not directly fabricate envelopes/results or select hidden entities. Extra owned paths are the required narrow interaction/core_adapter/widgets seams and their regression tests. Reviewer must verify unchanged player behavior and actual native/browser parity.

Use a controlled copy under tests/browser/fixtures/hd1/muster when actual producer/job controls and eligible content are needed for the each-behavior fixture. Add only necessary authored producer/recipe/queue/interface data; keep mods/muster and Forge untouched. The fixture must genuinely exercise Move, Stop, Hold, Attack, Gather, Produce, CancelProduction, Remove and both SetEnabled states through real player paths. Include exact accepted and refused sets per envelope, including the empty opposite set, and actual stored-group membership/recall selection. Choose real refusals that reach the server, such as a validated target or queue refusal; a core-filtered ineligible unit produces no envelope and cannot be claimed as a server refusal.

Order ids follow the actual accepted public catalogue, not an illustrative example spelling. Muster currently declares order_attack in orders/order_attack.ron; validate and use its actual qualified public id rather than rename protected content or invent an alias for the prose muster:attack example. The API remains id-based and supports any accepted mod order.

This is the brief section4 same-player-shape existing-mechanism/instrument resolution; no new key, player control, removed acceptance or frozen value. Root owns the small cross-task composition seam. Alternative not taken: require an extra key absent from the design, leave the id unused, silently skip Produce/Cancel, call an unrelated dispatch helper outside the core, or fake empty parity/result arrays. Require real tests for this seam and rerun affected widget/core checks before final acceptance.


## C192: Pass the shared HD1 action fixture through runner params

Conflict: HD1 needs the same action list in the headless file and pixel driver, but mc-browser currently allows stages and not an external action fixture in params. A driver must not read source/JSON files directly. Duplicating the list in stages creates drift risk.

Choice: add the narrow optional row field actions_file. The runner/parameter helper resolves a JSON file strictly within tests/browser/fixtures (including resolved-path containment), validates that it is an action list, and passes it as immutable params.actions. Native --actions reads the same file. Reject conflicting inline actions plus actions_file. Keep existing rows unchanged and record the fixture path/hash in run provenance. This is permitted shared-runner/test-parameter plumbing, not a driver capability expansion. Native/schema validation remains responsible for action semantics; drivers remain io-only. Add real positive and invalid-path/conflicting-field/invalid-list controls to the existing parameter contract tests.

Alternative not taken: duplicate the actions in stages, add driver filesystem/debug access, or silently omit actions. Root authorizes the narrow scripts/mc-browser.sh and mc_browser_params.py/parameter-test seam outside Task10's incomplete file list under brief section4. No player-visible behavior or acceptance row changes.


C187 enforcement addendum: NEWv2 now intrinsically captures a clean HEAD/status and tracked input hashes before execution, then rejects HEAD/status/byte drift before frozen reuse. Includes vendor/.cargo/rust-toolchain compiler inputs; actualcleanintegration940files. Output labels are unique. Originalhelper remains unchanged. Rootpositive/9parser+6immutability controls and RUN74 independent15suite mutations+3assume-unchanged compiler-input byte-drift controls passed. RUN74 accepted currenthelper source; receipt t10-review/run74/c187-final-helper-audit.json. This strengthens the original caller immutability requirement and removes no tests.

C188terminal outcome: all3jobsread/classified, bothfailuresexactapprovedTask21window; mergeholdreleased for otherwiseapprovedWave7candidates. No newruntimefailure class was observed in this run.


## C193: Observe each real browser audio context through the pre-WASM hook

Conflict: Bevy0.20 AudioOutput is pub(crate) with a private MixerDeviceSink (installed bevy_audio/src/audio_output.rs:16-17), so the client cannot read CPAL's underlying AudioContext through Bevy's public API. An aggregate context_state cannot prove AUD1's each-context rule. Guessing state from a resume request would be false observation.

Choice: the design9.5 pre-WASM constructor hook keeps the actual contexts and exposes a read-only snapshot of their actual state values. Add context_states to AudioFeed/debug while retaining aggregate context_state for compatibility. The browser oracle requires a nonempty list and all suspended before the first input, all running after actual resume. The wasm audio owner uses the same actual observation to gate audio-player creation; the debug publisher may merge the actual snapshot too. Never treat resume-called or its promise creation as success. Retain measured player_entities and active_by_category counts from actual ECS audio entities; real OGG playback and real sink/category gain remain required, not invented feed values. The pure Node fake-context tests cover hook logic only and do not count as actual browser AUD1.

This is a log-field/read-only observation bridge under the brief section4 and the design's existing hook mechanism. No extra player control or Bevy-private access. Missing/unavailable actual contexts stay unavailable/locked, not optimistically running. Alternative not taken: private Bevy-field access, a test-only state setter, a guessed running flag, or checking just one of several contexts.


## C194: Accept the design's optional HD1 tick field

Conflict: the current draft Rust Action requires tick and the Python actions_file helper rejects its absence, but design2.3:283-286 explicitly gives smart_command/order/group_assign examples without tick.

Choice: tick is optional. A missing tick uses the preceding effective tick, or0 for the first action, preserving list order without an invented delay. Explicit ticks remain nonnegative integers in nondecreasing order; booleans are invalid. Native parsing and the browser parameter helper use identical normalization. The source fixture remains unchanged, and its raw hash still identifies the authored file; params may carry normalized actions. Add positive controls for all-omitted and mixed explicit10 followed by omitted fields, plus existing invalid/decreasing controls. Explicitly timed fixtures retain their schedules. This is the design-nearest instrument timing default under brief section4, not a player-visible timing change.

Alternative not taken: require an extra field absent from design examples, treat each missing tick as0 after a later scheduled action and reject it, reorder actions, or add a new automatic delay.


## C195: Reproject each authored headless action after prior core input

Conflict: the draft hook batches same-tick actions and projects their world points using the pre-batch camera. A double group recall can centre the camera inside Core::step, making a later already-projected command target the wrong world point. The design requires headless actions to follow the same core/camera semantics as browser input.

Choice: tick is an earliest-due threshold for this client test hook. Preserve list order and dispatch at most one authored action per CoreAdapter frame, projecting it from the fresh core state after the prior action was processed. Key/pointer events for one authored action remain atomic. Do not add sleeps or increment authored ticks; C194 omitted-tick normalization is unchanged. Add a focused actual-core/action regression: double group recall changes the camera, then a named world-point command must still emit the intended world target. Accepted/refused-set parity alone cannot detect a wrong point. The separate server --command-script/DET1 exact-tick contract stays unchanged.

This is the nearest existing input-frame/projection mechanism under brief section4, with unchanged player keys/targets and no new visible delay rule. Alternative not taken: use a stale camera for all events, invent direct world-target envelopes outside the core, or relax the server replay schedule.


## C196: Give the HD1 oracle a bound real native reference

Conflict: HD1 must compare actual browser accepted/refused sets with actual headless --out, but current runner plumbing exposes no native report to the oracle. Hardcoding the same expected sets twice is not parity. Giving the driver native results would violate the driver boundary.

Choice: a trusted native launcher produces the real --out plus a binding receipt for actual argv/env/profile, original execution HEAD/dirty status, runtime/source byte identity, action-file hash, controlled package hash and raw report hash. The trusted browser launcher copies that report and receipt into each browser evidence directory through an explicit operator reference-directory option (MC_HD1_NATIVE_REFERENCE_DIR or the equivalent documented narrow option). The oracle reads them through serverFile and compares exact per-envelope accepted/refused sets, including empty opposites and missing/extra envelopes. The driver receives neither the native result data nor new filesystem/debug capability.

Run native and browser against separate fresh servers with the same controlled fixture, so slots and world state do not contaminate parity. One successful bound native report may serve both browser attempts when all runtime/action/package identities match; invalidate it after relevant changes. Retain original execution identity rather than relabel an old report as a newer run. Test launcher missing-reference/stale-binding controls; do not count a missing artifact as a gameplay red. This is the required test-artifact transport under brief section4; no acceptance is removed and no player behavior changes.

C195 fixture clarification: keep ordinary assign/recall in the shared HD1 fixture and the plan's fixed authored-camera projection. The double-recall camera stress remains in the focused actual native drive_actions/Core regression. No browser screenshot camera estimator is required.


## C197: Use a local ALSA development prefix for native audio

Conflict: Task15 adds native Bevy audio (plan:601; design9.5). Desktop pkg-config cannot find alsa, libasound2-dev is not installed, and sudo requires a password. The matching runtime libasound2t64 1.2.11-1ubuntu0.3 is installed. Stopping for a system package is outside the narrowed stop list.

Choice: download the official apt libasound2-dev 1.2.11-1ubuntu0.3 package, verify its SHA256 against apt metadata, and extract its real headers/development linker file under primary target/mc-evidence/native-prerequisites/alsa/sysroot. The development linker file resolves through a local symlink to the installed matching runtime. Use the two explicit PKG_CONFIG_PATH and PKG_CONFIG_SYSROOT_DIR variables in native audio builds and record them with the execution receipt. No system files, product source, frozen data or player controls change. Do not inject these desktop paths into committed Cargo configuration. Keep the existing5Cargo profile variables.

Evidence: packageSHA b83795b1ba0e40cfa8363359fd17a06260838d8ae0f315531771a62f7c7e215a matches apt. pkg-config returns the local include/library paths and -lasound. Loading the actual development library path reports ALSA1.2.11. Real native compile/link remains NOT TRACED until RUN73 receives the sole compiler lease; no competing compiler was started. Receipt and env.sh are under target/mc-evidence/native-prerequisites/alsa.

Alternative not taken: ask the CEO for sudo, modify system libraries, disable native audio, fabricate headers, or claim successful native compilation from a pkg-config check. This is the brief section4 local cache/environment handling decision. Global74/170; no new agent run.


## C198: Bind error feedback through existing authored unit responses

Conflict: design9.2:878, ORD6:1241 and HUD2 require an error sound. AssetsDef only gives sound ids and paths; the design gives no explicit global error-event field. Muster already exports its error OGG (mods/muster/assets/assets.ron:28), but its six unit presentations only bind select/order (interface/hud.ron:24-29). Inferring a sound from its asset name would make arbitrary mod ids significant.

Choice: use the existing UnitPresentation.responses string-key map (crates/turts_mod_api/src/interface.rs:410-413) with the event key error. Play the actual authored response in category effects for a real local refusal, per-unit/server envelope refusal or later last_failure transition. Prefer the affected unit presentation. For a unitless error or unavailable unit, use the first authored error list in deterministic unit-id order. No error request occurs for an accepted-only result. All normal sound-id validation applies (content_mc_validation.rs:667-675). Do not infer ids from names or hardcode a package id.

Authorize only the missing error-response entries in production mods/muster/interface/hud.ron and the controlled feedback fixture; all existing ids, rules, keys, texts and order behavior stay. Task17 authors the corresponding Forge error-response entries with its new interface. The existing Muster asset is reused. This implements the already-designed audible feedback; it adds no player control or schema field. Keep real sound/category/timing proof and missing-sound validator controls.

Alternative not taken: a new global schema field, an engine-side Muster/Forge sound id, suffix guessing, silent real refusal in the shipped package, or an error sound on every accepted result. This is the nearest existing mechanism/incomplete instrument choice under brief section4. Global74/170; no new agent run.


## C199: Resolve audio dependencies during source preparation

Conflict: RUN72 owns the sole compiler/cache and must keep it for HD1. Task15 has added the planned Bevy audio/vorbis features but its new dependency lock/download preparation can run without compiling or creating another target. Waiting for a compiler handoff to discover missing locked dependencies adds avoidable delay.

Choice: allow RUN73 one bounded cargo fetch --target x86_64-unknown-linux-gnu --target wasm32-unknown-unknown preparation command in its own lane. Preserve the original Cargo.lock, record actual command/env/raw output/exit and the resulting lock diff, and confirm no target/compiler/server/browser was created. Accept only dependency resolution required by the planned features, with no broad cargo update or generate-lockfile. Other platforms are not downloaded. RUN72 remains the sole compiler/runtime owner. RUN73 returns to source-only after fetch.

This is the brief section4 cache/lane handling decision. No native/browser test or compilation credit is given. The exact archived post-red source remains historical and unchanged. Alternative not taken: a parallel compiler/cache, unnecessary all-platform downloads, a broad lock refresh, or waiting until compiler lease for network-only preparation. Global74/170; no new agent run.


## C200: Batch ORD6 real physical clicks in one client input frame

Conflict: ORD6 requires100 real envelopes at once, then an accepted envelope after1s. The draft loops100 awaited io.click calls. Each click has multiple CDP round trips (harness/driver-io.js:29-61), while the fixed server bucket has capacity64 and refills32 envelopes per50ms tick at20Hz (server/session.rs:60-72). Counting100 calls does not prove a burst or any real exhaustion. The oracle's any accepted result does not identify the post-refill command.

Choice: extend the existing physical io.click with a bounded optional count (1..100), rather than a new driver method or a socket/debug capability. All clicks use the same authored pixel/button and real trusted Playwright/CDP input, with a separate sequence and observed down/up trace for each. The old three-argument call remains unchanged. ORD6 uses count100, then its separate ordinary click after1s.

The trusted harness may hold only application requestAnimationFrame callbacks during that bounded input delivery and release them through the real scheduler afterward, so the real existing core consumes the100 physical releases in one input frame. Install the narrow timing hook before WASM. Do not synthesize DOM events, commands, results or core/debug state. Do not change server tick rate, bucket, refill or engine behavior. Always release the frame hold on failure; bind hook/source/input traces and actual frame/envelope evidence. No pause API is exposed to the driver. A tight dispatch loop alone is not enough if actual core/server traces show the commands spread below the bucket.

Require actual100 physical releases,100 corresponding emitted envelopes and authoritative accepted/refused results. Bind the separate101st release to its own accepted result after the observed1s wait/refill; an earlier accepted result cannot satisfy it. Every refusal/failure must still have its frame's player text and real error sound evidence. The native bucket controls and required u32::MAX mutation remain. Unit fake-frame/CDP tests cover the helper only; real browser proof remains mandatory.

This is the brief section4 instrument/fixture mechanism choice with the same real inputs and player-visible acceptance. Authorized extra files are the narrow existing harness driver-io/run/contract and timing-hook tests plus ORD6 driver/oracle/row plumbing. Alternative not taken: change K25 constants/tick rate, fabricate browser commands through a new channel, call100 slow inputs a burst, add an unbounded click method, or accept the wrong refill result. Global74/170; no new agent run.


## C201: Correct the menu ownership assumption in C189 composition

Conflict: root previously referred to an existing Task10 F10/Esc overlay and told Task15 to wait for it. Actual client_shell.rs:1-7 in both lanes is an empty plugin stub. Its FutureTask10 comment is stale; plan Task10:580 is the headless actions hook, while Task16 owns the shell. The existing core already emits ShellAction menu for the designed keys (interaction/lib.rs:306-318; input_map.rs:76-89), but no menu consumer exists. That earlier root dependency claim was wrong.

Choice: carry out C189's already-authorized Task16 subset directly in Task15: implement only the single designed F10/Esc overlay and its four volume controls in client_shell.rs, using the existing CoreEffects/ShellAction menu route. Task16 adds its remaining controls and screens to this same overlay later. Do not add a second menu, new key or global Escape handler that bypasses the existing cancel/menu priority. Prove actual menu input capture prevents world orders while sliders/buttons are used.

This corrects an unverified ownership assumption and resolves the incomplete stub through the nearest existing mechanism under brief section4. It does not change the design, keys, player screens or the wave. The minimal shell subset is already required by C189 for AUD1's real player volume adjustment/reload proof. No Task10 shell work is requested. Alternative not taken: wait for a task that does not own the overlay, leave AUD1 without real controls, or use hidden volume setters. Global74/170; no new agent run.
