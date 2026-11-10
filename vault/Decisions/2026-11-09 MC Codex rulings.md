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


## C202: Record WebGPU adapter metadata without inventing hardware identity

Conflict: Task10 added a nonempty adapter-name assertion before HD1 parity. Actual headed Chrome returns Bevy render_adapter backend BrowserWebGpu, device_type Other and an empty name. Design13.5/HD1 requires real browser/headless result parity; it does not require a vendor name. C179's hardware measurement belongs to VIS2 and was separately fulfilled in Task13. This extra assertion hides the still-unassessed parity result.

Choice: remove the nonempty-name requirement and record the actual metadata. Other/empty is not proof of hardware; mark hardware identity NOT TRACED unless an actual independent instrument identifies it. Chrome flags or GPU process arguments alone are not that proof. Keep actual headed rendering, real physical input, source/report binding and exact native/browser results. No broad GPU measurement hook is added to HD1. Both oracle-mismatch attempts remain failed instrument attempts, with gameplay parity unassessed. Alternative not taken: fabricate a name, call Other proof of hardware, weaken VIS2's separate measurement, or add an unrelated vendor acceptance to HD1. Brief section4 instrument detail;74/170runs unchanged.


## C203: Keep ALR1 ping derivation and minimap drawing with their planned owners

Conflict: draft Task15 ALR1 oracle asserts visible minimap ping pixels, but client/minimap.rs:1-7 is the Task14 empty plugin. Plan Task14 explicitly needs15 for AlertPings. Task15 produces that feed; Task14 renders it. Advancing the whole minimap to15 would invert the written dependency. The draft also expects one retained damage line after7s of repeated damage, which cannot establish a1s cooldown.

Choice: Task15 proves all forced alert derivations, actual sound, actual ping feed/rule/unit/position, visible alert line and two real health drops within1s with no second emitted alert. Bind cooldown proof to actual frame emissions/ticks, not the total retained last-five list after a long wait. Add the actual rule id to ping telemetry where needed; retain its existing unique numeric id. No invented ping positions or synthetic health transitions count as pixel proof.

Task14 owns the actual minimap ping drawing and visible pixel proof. After Task14 implements it, rerun ALR1 with that pixel predicate enabled together with its MM1 ping checks. Keep the visible-ping obligation explicitly OPEN on the board until then; Task15 must not claim that part green. Task24 requires the complete ALR1 row including visible ping before final acceptance. Do not hide the obligation behind a permanent skip or count a stub as a rendered minimap. No designed player behavior or final acceptance is removed.

This is the brief section4 uncovered instrument/nearest existing mechanism and ownership resolution. Alternative not taken: advance the whole minimap into Task15, fake pixels from debug metadata, treat a list of identical retained entries as a cooldown measurement, or claim complete ALR1 before the minimap exists. Global74/170; no new agent run.


## C204: Conditional exact-binary carry for the same-type Clippy repair

Conflict: committed Task10 gate stops at Clippy101 because id.clone().into() converts a value to its own type (interaction/lib.rs:167). Removing that redundant conversion is required. Unconditionally repeating the84.88s browser pair for an unchanged compiled program adds no coverage; a source-only equivalence claim is insufficient proof.

Choice: after the one-line identity-conversion repair, run all required normal gates and fresh native execution. Carry the historical successful browser pair only if the actual rebuilt release client, server and test executable hashes ALL exactly equal the previously browser-tested native binding, and the action/package/driver/oracle bytes are unchanged. Record the exact patch and each before/after hash. Keep the old execution HEAD/dirty identity; never relabel the old native receipt current under the new HEAD. Any differing runtime byte or further runtime/input change requires affected browser recheck. No native, gate or plant is waived.

This is a brief section4 instrument/redundancy decision consistent with the CEO's test-grouping instruction. Alternative not taken: waive Clippy, infer binary equality without measurement, reuse proof after a changed binary/input, or repeat a known identical compiled program solely for source identity metadata. Conditional proof is NOT TRACED until actual comparison. Global74/170; no new agent run.


C204 clarification: the exact gate-warning patch also removes equivalent len/PathBuf conversions, an unused import and test-only style warnings; preserve its fullSHA and explicitdiff. Browser carry additionally requires rebuilt WASM/glue/HTML bytes unchanged, because native executables alone are not the browser program. Root preserved a clearly labeled POST-RUN current-dist snapshot before any newTrunkbuild; it is not relabeled as at-launch capture. If directproof is not met, rerun the affected pair. No broader source cleanup is authorized.


## C205: Update the independent E9 expected report for existing public fields

Conflict: G-ACC on clean Task10 candidate1d7c816 fails the native E9 late-join test at economy.rs:1558. The client report now includes owner-public unit_orders and per-unit max_health, added for Task13 under C165, but owner_report_projection_from_audit:1487-1513 omits both. Actual tick241 data otherwise matches the independent audit. This native projection mismatch is NOT the approved Task12-to21 browser-panel red window.

Choice: repair only the legacy test's expected report projection now. Include snapshot.unit_orders from the existing independent owner_unit_orders_from_audit helper (:1199). For this explicitly M2-only fixture, retain the no-M3-state invariant and expected null max_health; fail on unexpected health state rather than copy values from the client. Canonicalize the new owner rows by unit while preserving order/queue semantics. Keep full actual==expected comparison, private-owner filtering and all old fields. Never drop the new fields from actual or derive expected values from client output.

Preserve the compiled E9 red and prove the focused case green. Batch the independently found native tick:null parser disagreement with this repair: reject explicit null as required by C194, retain omission semantics, and show a named malformed-tick red then restored valid/invalid controls. Commit the bounded changes, rerun final gates/G-ACC, and rerun actual HD1 native/browser acceptance because C204 runtime hashes differed. No frozen audit/stream value, wire shape or gameplay changes are needed.

This is the brief section4 instrument/report-field/incomplete ownership resolution. Alternative not taken: call every legacy failure an approved window, postpone a required native gate to21, weaken full-state equality, or change the frozen baseline. Global74/170; no new agent run.


## C206: Enforce command environments and remove duplicate default caches

Conflict: a focused Cargo command omitted the required environment and compiled into Task10 target/release. A second default target/debug cache also exists. The semantic test stopped at missing turts_client, so that attempt is setup failure, not the independent-projection red. Repeating manual environment entry wastes disk and time.

Choice: stop all compiler/runtime processes and preserve raw logs, both dirty source files, inventories, timestamps and workspace Cargo fingerprint metadata. Root confirmed only standard rebuildable Cargo outputs and no active Cargo/rustc/Trunk/server/client process. Remove only target/release and target/debug; keep target/mc-t10 and all evidence. Actual free space rose from688043413504 to690931560448 bytes, about2.69GiB reclaimed. Receipt: t10-root/default-cache-audit/cleanup-receipt.json.

All future Cargo, Trunk and native/browser runtime commands must use the environment-enforcing exec_in_lane.py wrapper or an already-approved gate helper that enforces the same environment/cache. No naked Cargo command. The new wrapper resolves the expected short commit to the actual full commit, requires the existing named MC cache, assigns all5Cargo variables, rejects an existing default cache or competing compiler, requires --locked for Cargo compilation, records actual argv/env/source/raw exit, and rejects concurrent source drift. The --audio flag supplies the verified C197 pkg-config prefix. Pure source-only Node/syntax checks remain allowed.

Root positive control verified all5environment values. Negative controls rejected unlocked Cargo and dirty source without a grant before command execution; a temporary Git fixture with a deliberately changed source produced command0/wrapper2. The helper never reads Claude path contents. Wrapper SHA65d28e4d81b3e5c6e326a451d23225ca39ef50153df97b480f25e834d38b1df8. Proof: t10-root/environment-wrapper-controls. Root holds runtime until explicit return; no new source work is permitted during this cleanup.

This is the brief section4 cache/lane/instrument decision and follows the CEO disk constraint. Alternative not taken: keep duplicate caches, delete active outputs or unpreserved work, credit setup failure as semantic red, or rely on repeated hand-entered environments. Global74/170; no new agent run.


C206 measured hardening: independent reviewer found direct Cargo target/config flags can override the environment. Wrapper now rejects --target-dir, --build-dir and --config forms before execution. Actual two negative controls passed; current SHA4fffdc82e7900d9d44c69dc13fb1d067606893c72d980123d23a1dba5c81d4b0. Actual null-tick and E9 greens used this version and recorded all5environment values/source stability/no default cache. Current source and the hash-verified inverse-patch reconstruction of65d28e4d are archived with honest provenance under environment-wrapper-controls/source. Reviewer accepted the prescribed-command scope.


## C207: Use the existing group allocator for large typed UI orders

Conflict: independent review found the new UiOrder path always calls send_explicit_units with group0 (interaction/lib.rs:195-202). That helper splits more than4096 units. A valid Content action for4097 eligible producers therefore becomes two group0 envelopes, contrary to K4. Installed production widgets use one producer, so the current HD1 fixture cannot detect this boundary.

Choice: repair the implementation. After ownership/capability filtering, use the same existing alloc_group rule as ordinary orders: more than one valid unit gets one nonzero group id, shared by every split chunk; a single producer keeps group0. Add one focused4097-unit Content test that checks chunk bounds, exact unit union and one group, plus retain the single-producer/Replace guard. No new key, schema, player action or K4 change is needed.

Let the already-active e200 native gate finish without source drift. Batch this repair and any remaining consolidated review findings before the next final gate run. Preserve prior outputs as historical checks, not final-source acceptance. This is a brief section4 review repair and nearest existing mechanism decision. Alternative not taken: keep an uncovered K4 violation, reject otherwise valid multi-unit typed input, add a second allocator, or change the single-producer widget behavior. Global74/170; no new agent run.


## C208: Check visible HUD colour before each HD1 button click

Conflict: design13.4 explicitly requires a driver to check HUD button colour in a screenshot before clicking. The HD1 driver uses the allowed authored layout literals but, before its final checkpoint, clicks without inspecting pixels. The successful result comparison does not replace this explicit instrument requirement.

Choice: add a narrow pure screenshot-colour check before each authored UI-button click. Use the existing PNG decoder, literal colours learned from the controlled HUD and the authored click region. The driver reads only io.screenshot bytes, never package files, turts_scene/widgets or semantic bridges. Preserve the checked PNG and colour observation through io.save. A missing or wrong visible colour must stop before the click. Keep the same action list, physical inputs, native/browser result oracle and all expected outcomes.

Use a small pure helper/control if needed: actual known colour passes; absent/wrong colour rejects. Queue buttons without icons use their authored visible button colour; do not claim icon identity there. Batch with C207 before the next final committed gate/acceptance run. This follows the design over the incomplete plan instrument detail under brief section4. Alternative not taken: exempt HD1 without design support, read widget rectangles semantically, or call a blind fixed-coordinate click a pixel check. Global74/170; no new agent run.


## C209: Defer three Low invalid-input/provenance hardening notes

Independent review completed the existing diff source audit with no remaining High/Medium beyond C207 and the C208 instrument requirement. Root accepts three Low notes for later instrument hardening: (1) a malformed Content/Job headless action with only a world point should give a clearer early diagnostic; current real core emits no invalid command. (2) the native binding validator could explicitly compare binding.source with execution_evidence.source_after. (3) creation could explicitly cross-check the recorded release launcher against the derived executable profile.

The trusted launcher already requires actual successful named execution, matching before/after/current source, actual argv/env/binary hashes and raw status. The actual accepted fixture uses typed targets and actual release executables; no contradictory or forged receipt was accepted. These notes do not affect valid action semantics, player behavior, source identity or the measured native/browser result comparison. Preserve them as known Low limits, not silent fixes or extra gates. Required missing/stale reference checks, C207/C208 and all final runtime/gates/plants remain.

This is the brief section4 Medium-or-lower finding decision. Alternative not taken: broaden the current repair into malformed-script UX or adversarial receipt security, or claim the extra checks already exist. A new actual contract failure overrides this deferral. Global74/170; no new agent run.


## C210: Put observed tick rate beside the debug snapshot

Conflict: ALR1 needs the actual tick rate to measure a1s cooldown. The Task15 draft put tick_hz only in one replicated_world publication path, creating inconsistent captures and another strict legacy owner-projection mismatch like C205. Tick rate is observation metadata, not an added unit/world-state DTO field.

Choice: publish the actual Replica.tick_hz as top-level turts_debug.tick_hz and native report metadata, in all capture/rebuild paths; use null when no actual replica exists. Keep replicated_world unchanged. ALR1 reads the actual top-level rate. Add capture/rebuild consistency assertions. Root inspected the current debug.rs diff and verified no tick_hz remains under replicated_world. This avoids changing an unrelated server test or frozen value for read-only timing metadata.

This is the brief section4 log-field/instrument placement decision. Alternative not taken: hardcode20Hz, update only one publication path, or expand the replicated-world DTO with observation-only metadata. Source checks pass; actual native/browser proof remains NOT TRACED until Task15 receives its runtime lease. Global74/170; no new agent run.


## C211: Rotate the completed source-prep seat into Task17 preparation

Conflict: four agent slots include root. Task15's source work is ready but cannot compile while Task10 holds the sole cache. Keeping that source-only seat idle prevents parallel preparation of Task17, whose declared Needs are already merged.

Choice: RUN73 saves its98-path WIP archive/patch/hash handoff outside Git and ends uncompiled/unaccepted, with its branch/worktree unchanged and no target. Register RUN75 as Task17 builder source/test/asset preparation only in its own branch/worktree from current integration. Do not compile, create another target, start browser/server, or add the Forge HUD before the required actual baseline reds. Source preparation may write the baseline tests, drivers/oracles and asset/provenance recipes. Root retains the sole runtime lease.

When Task15 gets its compiler lease, resume the preserved builder under a new ledger row before it runs; count that continuation toward Task15's estimate/cap. No new task acceptance, budget expansion or wave reordering occurs. This is the brief section4 lane/cache/concurrency decision. Alternative not taken: a second compiler/cache, losing uncommitted audio work, or leaving an available preparation slot unused. RUN73 handoff: t15-builder/run73/handoff-20261109/receipt.json. Global75/170 registered before dispatch; Task17 run1/estimate4, soft8/hard16.


## C212: Test the actual effective bindings and preserve the Press R plant

Conflict: Task17's guide test must use effective bindings, but turts_pack has no dev-dependency on the existing turts_interaction resolver (input_map.rs:93-120). Reimplementing the resolver in the test risks a self-confirming duplicate. The plan also exempts bare single-letter labels in step1 while step4 explicitly requires the Press R plant to fail.

Choice: authorize only a test-only turts_interaction path dependency in crates/turts_pack/Cargo.toml. Use the real effective resolver as the independent source of key labels. Root will resolve the minimal lock edge under controlled metadata preparation before the actual baseline; no hand-edited lock, broad update, compiler or new target is permitted during source-only prep. The current edit is one dev-dependency line.

For guide pages1-4, retain whole-word multi-character/special-key checks. A bare article A/a remains ordinary prose. A single-letter label used as an explicit key instruction or chord, such as Press R, is a key mention and must fail. Include article-positive and imperative-negative controls. Keep the required Press R plant and the page5 real Bindings source; do not weaken or retire it. This resolves the instrument contradiction in favour of the design's key-free first pages while preserving the plan's article safeguard.

This is the brief section4 omitted dependency and test-instrument choice. Alternative not taken: duplicate the binding resolver, ban ordinary articles, allow explicit Press R through the guide check, or remove a required plant. Global75/170; no new agent run.


## C213: Correct only the production-button palette and reuse unchanged native gates

Conflict: C208's actual headed pair stops at the production button because root supplied the Dummy portrait colour. The controlled make_dummy definition actually outputs muster:beacon and names Build Beacon. Both saved pre-click PNGs show the real Beacon button; actual RGB128,212,255 matches413 interior pixels in each, while the wrong Dummy colour matches0. This is root's calibration error, not an absent/disabled control or a changed gameplay result.

Choice: change only the pure helper's producer colour/name and its unit-test label to Beacon. Keep action/order ids, click points, package, threshold5, tolerance3, native code and oracle outcomes unchanged. Preserve both failed attempts and the actual pixel calibration in final-e349/beacon-palette-calibration.json.

To avoid repeating the197.81s native suite and447.22s G-ACC for two JS-only calibration edits, carry the e349 native results only after verifying the complete diff is those two pure-JS files, all native source/test/build/config/package/action inputs are byte-identical, and freshly rebuilt native executable bytes still match. Keep original e349 execution identity. Run current G-QA and pure controls, fresh actual native binding, and a new headed pair with every pre-click check. All plants/reviewer and full integration re-gates after merge remain required. Any other change or native byte difference cancels carry and requires affected gates.

This is the brief section4 instrument-detail/test-grouping decision under the CEO's speed instruction. Alternative not taken: weaken colour checks, infer content from its id, call the wrong portrait correct, relabel old execution as new, or rerun unchanged native programs without a new concern. Conditional carry is NOT TRACED until measured. Global75/170; no new agent run.


C213 measurement: complete Git diff is exactly the2authorized pure-JS files on311d486. Root compared all1369 original source files and verified freshly built/executed client/server/test binary hashes, actions and package bytes equal e349. Carry conditions passed; original native/G-ACC receipts remain labelled e349. Current G-QA/pure controls and fresh bound named native execution each exit0. Full fresh report bytes differ with live timing and remain independently bound; report-byte equality is not a gate-carry condition. The new actual headed pair remains pending.


## C214: Run Task17 baselines before the Task15 build lease

Conflict: Task17 RUN75 prepared the native guide and HUD2 baseline but has no SEL1 driver and no actual baseline execution. The sole cache is still Task10-owned. Task15 source is ready for compilation. Waiting for the entire Task15 runtime batch would leave Task17 content preparation blocked on its red-first obligation.

Choice: register RUN76 as Task17 builder continuation now, source-only for the missing baseline instrument. After Task10 passes review, merges and its full integration re-gates, move the sole cache under the existing preservation/cleaning procedure to Task17 for a bounded pre-HUD native/browser red batch. Then move the cache to Task15 while Task17 writes HUD/assets/guide source. Do not add the production HUD before actual baseline reds. All normal gates, plants and independent review remain. Task15 needs a new ledger row before its builder resumes.

Evidence: plan Task17 step1 (2026-10-29-mc-player-control.md:614-619) requires both HUD2 and SEL1 before HUD content; RUN75 handoff states SEL1 and actual reds pending. The brief section4 permits task order inside a wave and cache/lane handling. Alternative not taken: a second cache/compiler, calling syntax checks semantic red, or keeping Task17 idle through Task15's full validation. Global76/170 registered before continuation; Task17 run2/estimate4, soft8/hard16.


## C215: Give SEL1 a foreign weapon unit in a controlled fixture

Conflict: SEL1 must show a selected foreign unit's range circle (design section14:1231); section9.1:845 requires a weapon. The actual Forge map has only foreign drones, and the actual Muster map has only foreign dummies/posts. These types have no weapon. A conditional range assertion would leave the acceptance obligation untested. Forge maps and units must keep the fixed point.

Choice: create tests/browser/fixtures/sel1/forge as a controlled byte-copy of the actual Forge package, with only one extra map placement of the existing forge:turret for owner1 at a visible empty authored point (26,8), empty contents. The existing definition supplies range8 (mods/forge/units/turret.ron:11-17); no new weapon or engine behavior is needed. Original mods/forge maps/units stay unchanged. HUD2 uses actual Forge; SEL1 uses the controlled fixture and unconditionally verifies the foreign turret's range, read-only selection and empty command card, plus own click/ground clear/hover obligations.

Keep the clone's content/assets byte-identical to actual Forge except the documented map addition, including later HUD updates, and preserve the equality/delta evidence. Run the real pre-HUD baseline before adding green HUD content. Evidence: mods/forge/maps/proving_ground.ron:14-15 and mods/muster/maps/field.ron:49-59 show weaponless foreign placements; design9.1 and SEL1 are compatible once the instrument has a weapon-bearing target. This is a brief section4 fixture/instrument choice. Alternative not taken: make range optional, edit the frozen Forge map/unit types, or report a circle for a weaponless unit. Global76/170; no new agent run.


## C216: Bind native HD1 to the assigned cache in every lane

Conflict: root's cross-lane preparation found scripts/hd1-native-reference.sh:5 unconditionally sets target/mc-t10. scripts/hd1_reference.py:81-83 also requires that literal. G-ACC calls the script at acceptance.sh:138. Task10 proof is valid in lane10, but a later lane would compile a duplicate cache or reject its correct binding. This contradicts the CEO disk rule and the per-lane target contract. RUN74's approval of311d486 remains historical; hold the merge for repair.

Choice: root repairs only these two orchestration instruments. Require and normalize the caller's explicit CARGO_TARGET_DIR within this worktree's target/mc-* namespace, preserve it for both Cargo subprocesses, and validate the launcher/runtime against that assigned directory. Do not fall back to a different lane. Use one early-stopped Cargo-stub control to prove the current misroute then the fixed route without compiling another cache. This is instrument red/green, not gameplay credit.

Register RUN77 for the independent bounded repair re-check. Compare complete source delta and actual native bytes: carry prior native G-RUST, unaffected G-ACC cases and unchanged product plants with their original identities only if native code, fixtures and executable bytes are unchanged. Re-run the affected actual native-reference component, current QA and headed browser pair on the final source. Root will additionally run the actual named native binding in integration's non10 cache after merge, alongside all mandatory integration re-gates. If an input or binary changes, re-run the affected gate; do not relabel old executions. No retired plant or player change is needed.

This is the brief section4 instrument/cache/Medium repair decision and the CEO test-grouping instruction. Alternative not taken: create the second cache, weaken source/binary binding, accept a hardcoded lane in a shared acceptance script, or repeat all unchanged acceptance cases without a changed input. Global77/170 registered before dispatch; Task10run3/estimate2, belowsoft4/hard8.


## C217: Split HUD2's two package checks into companion instruments

Conflict: HUD2 requires the Forge HUD and the Muster ore resource line (design section14:1252). The runner gives each row exactly one package (mc-pixels.sh:12-25); extra windows share that package. The plan names only HUD2.json, so one invocation cannot exercise both authored packages.

Choice: keep HUD2.json on actual Forge and add HUD2-MUSTER.json on mods/muster with a unique instrument ID and an explicit known-scenario parameter to the same pixel-only driver/oracle. Both instruments belong to acceptance row HUD2 and must pass in the Task17/final batch. The Muster case must observe the real visible ore line with actual replicated balance, not merely find a schema entry. Preserve baseline/green evidence for the relevant obligation; do not count an unrelated missing Forge HUD as a Muster red.

This is a brief section4 fixture/file/instrument split, consistent with existing companion visual cases. Alternative not taken: omit one package, pretend two windows load different packages, add product behavior to switch packages, or change the acceptance row. Global77/170; no new agent run.


C216 compatibility: RUN77 traced CI check/browser env at .github/workflows/ci.yml:11,63,68 to target/m3-t16. Preserve this exact explicitly assigned legacy CI basename as well as local MC names; do not change CI cache strategy or introduce a fallback. Actual stopped-stub controls prove the old script overrides a different assigned MC target (red02 assertion1, stub79), then the patch preserves it (green0) and preserves the declared legacy CI target (green0), with no compiler or probe cache. First control attempt had a stub quoting SyntaxError and is setup-only/uncredited. Candidate83e6228 is a two-instrument-file change; affected final native/browser checks remain pending.


C216 consolidated compatibility repair also preserves the assigned compiler budget: CI declares jobs2, whereas the new helper previously forced4 and required4 in its binding. Keep caller jobs1..4 (default4 if absent), bind the actual recorded value, and retain all debug/incremental guards. The local wrapper still requires jobs4. Actual stopped CI stub observes both target/m3-t16 and jobs2, exit0 without compiler/cache. Candidateb051e04 changes only the same2instrumentfiles; earlier83native proof remains historical, final bound process/browser check is new. RUN77 found no other material source issue in its consolidated audit before this bounded correction.


## C218: Observe actual world-tooltip text through the existing UI bridge

Conflict: SEL1 requires the unit name/owner world tooltip after0.5s (design9.1:844 and section14:1231). Design13.4:1198 says text is checked through turts_ui, with no OCR. The actual tooltip is Bevy WorldTooltip+Text (visuals.rs:1282-1357), while snapshot.rs:12-20 ignores every entity without AuthoredWidgetId. A pixel panel/glyph check alone cannot establish the required text.

Choice: authorize Task17's narrow observation repair in visuals.rs and widgets/snapshot.rs. Expose the existing marker/unit field within the crate for the test seam. Extend turts_ui with actual world-tooltip rows, kind world_tooltip, observation id world.tooltip.<unit> and unit id, real Bevy Text, computed rectangle and actual layout visibility. Never attach AuthoredWidgetId to an engine tooltip or derive text from the catalog in the observer. Keep authored rows and scene_widgets unchanged. No new driver channel, OCR, input, style, delay or gameplay change.

Prepare a focused actual-marker/Text semantic red before implementing the snapshot branch; the marker visibility change alone is allowed for that test seam. Run it in the bounded baseline lease. Browser SEL1 must pair absence before0.5s with actual tooltip pixels and exact rendered name/owner through turts_ui after0.5s. Required text cannot remain NOT TRACED at acceptance. This is the brief section4 instrument/log-field and nearest existing mechanism choice. Alternative not taken: waive the text, infer it from package declarations, mark an engine entity as authored, use OCR, or expose semantics to the driver. Global77/170; no new agent run.


## C219: Calibrate Task17 baseline instruments against their actual inputs

Conflict: SEL1's first browser attempt expects arbitrary top-level row facts, but mc-browser.sh:173-179 forwards only an existing whitelist. HUD2-MUSTER's scenario field has the same issue. These are setup failures, not missing-feature reds. A later SEL1 attempt reaches selection clear, with actual empty selection/rendered rings, but the colour-only absence predicate mistakes the blue unit body for a remaining selection ring.

Choice: keep the runner whitelist unchanged. SEL1 may use the approved literal foreign-turret fixture facts or the existing fixture_catalog parameter. HUD2-MUSTER branches on the already-forwarded row/package identity; remove ignored scenario metadata. Preserve the unsupported-field attempts as uncredited setup.

For selection-ring absence, compare cleared pixels with the actual initial unselected image at the same projected samples, only after verifying equal viewport/camera/static-unit position. Require the selected image to differ and the actual C84 rendered-ring kind/unit to prove the selection ring; hover/body colour alone is insufficient. Cleared Core selection and rendered rings must both remain empty. Preserve the false body-colour attempt and actual sampled PNG evidence as instrument calibration, not a product failure. Re-run only the affected baseline rows before any green HUD/snapshot content.

This is the brief section4 instrument-detail/Medium repair choice. Alternative not taken: broaden the semantic parameter channel, accept a false red, delete ring pixels from the requirement, or change production selection/rendering. Global78/170; no new agent run.


C219 source/measurement correction: RingsShown/collect_visible_rings observes only entities with RingMetadata, attached only to VisualKind::Selected (visuals.rs:1208-1234). Hover meshes exist at:833-900 and deliberately carry no RingMetadata. Empty rings_shown during unselected hover is expected; it is not a hover regression. Root viewed the v4 delayed PNG and confirmed actual tooltip glyphs/hover overlay. Keep selected-ring unit/slot/own/radius checks, with no invented kind field; final hover proof uses actual pixel differences and the delayed actual UI text. Native C218 red and headed v4 missing-observer red are valid; unsupported metadata/false hover-feed assertions remain instrument-only. No production hover/rendering change is authorized by this correction.


## C220: Repair the complete Task15 source-review batch before final gates

RUN79 found one High and five Medium gaps. Root read the relevant source and chooses repair for all; none is deferred. Existing focused native green is useful partial evidence, not Task15 acceptance.

1. High, visible alert text: feedback.rs writes text/message, but widgets/sources.rs:926-953 fills Title only from absent title, while Muster renders Item(Title). Make actual alert rows expose their real player text under the authored field and prove nonempty rendered UI text/glyphs, not just a feed.
2. Medium, ResearchReady: feedback.rs checks only one station's queue, while sim/economy/research.rs:121-128 blocks a same-slot technology in progress at any station. Use the same real slot-wide rule, with positive, same-slot other-station negative, and other-slot control. Preserve reserved/cost/prerequisite/capacity/enabled conditions.
3. Medium, alert click: design9.4 requires click-to-centre, but rows discard position/action and authored lines are plain Text. Add a narrow internal typed widget camera-centre action from the actual finite alert position, routed through the common Core camera controller and existing map clamp, never an order/network effect or direct bypass. Use generic authored Button Item(Action) without a new RON syntax/key or content id. Preserve selection/groups; headless guard plus physical alert-click camera proof. This fills the existing design, not a new player control.
4. Medium, no-orders: emit once for the actual joined package even with no authored interface, rather than beneath Some(interface) or before Welcome. Add a narrow generic client notice fallback when no authored interface can render it; no extra panel when an interface exists. Publish actual fallback Text/layout through turts_ui with an engine observation marker, not a forged AuthoredWidgetId. Use neutral observation names that do not collide with Forge tokens. Test pre-join/None-interface/once and actual visible text. Coordinate snapshot.rs with Task17's separate tooltip observation; keep both at merge.
5. Medium, early audio activation: the permanent handled flag is set before contexts may exist and blocks later real gestures. Keep observation truthful and retry suspended contexts on later real gestures; do not mark a resume request as running. Preserve RUN79's actual isolated-script early-click→late-context→next-click semantic red, then green; real AUD1 remains required.
6. Medium, ORD7 audio: Update observation precedes Bevy's PostUpdate playback. Observe actual sinks after playback/deferred component creation and before FrameClose/debug in that same presented frame, and make the oracle require the actual matching sink/request/core-frame identity, not only requested FrameFeedback. A deliberate playback-delay plant must fail the repaired browser proof. No synthetic sink or relaxed timing.

Also accept two bounded instrument corrections: distinguish simultaneously legal ResearchReady/Complete technologies in the forced fixture, and compare only f32 persistence numbers within1e-6 while preserving exact unrelated-field/object checks. The ORD2 controlled map overlap is fixture setup, not an order-policy red; space placements using real footprints without editing production units/maps, then obtain the actual300-unit result/movement proof.

This is the brief section4 review/instrument/nearest existing mechanism choice. The typed action, fallback observation and field ownership extensions fill existing design9.4/11.4 behavior; no K/frozen/player-visible design rule changes. Alternative not taken: defer a High, call blank feed values visible, drop click-to-centre/no-interface notice, invent a key, use fake playback evidence, or credit compile/fixture failures. Global79/170; no new agent run.


C220 ORD7 retention addendum: retain causal result-bearing frames as well as physical-input frames in the existing bounded retained_input history, with the same4096 total cap and clear semantics. Document the historical field name; recent64 alone cannot prove a later result after1000ms at high FPS. Match the actual emitted command sequence to that result, not an arbitrary result frame. Test retention beyond64idleframes.

Actual sink evidence must likewise be stable by playback identity: record one real onset per actual player entity, preserving its first observed/request/core frame in bounded history. Keep current actual sink/gain observations separately for volume checks. Do not increment play_id every frame or evict onset because an unchanged sink was sampled96times. Verify retention beyond96frames; real browser onset proof and playback-delay plant remain. No fake AudioSink or change to audio behavior is implied.

## C221: Put the atomic-return plant on the actual mixed-unit contract

Conflict: Task15 step4 says the whole-list-return ORD2 plant must fail both halves. Design revision7 section14:1237 explicitly makes the browser box drop static/foreign units, yielding40valid mobile units. Atomic and per-unit validation are equivalent for that envelope; only the300mixed-unit headless case distinguishes them. Further, command.rs:516 is the legacy validator; typed Order delegates via command.rs:187-218 to orders::apply_order and sim/orders.rs:2653onward. Planting the old validator would not exercise this contract.

Choice: preserve the whole-list atomic-return mutation and retarget it to the real typed ownership/applicability path. Require the actual300-unit headless case to fail semantically (rather than290/5/5), exact restore and same native green. Run the40valid browser case under the mutation as a positive green control, because the design explicitly excludes invalid units there. Record the plan's both-halves-red expectation as a conflict resolved in favor of the design. Do not manufacture a browser failure by changing its selection or fixture.

No named G-REF refutation plant is retired, and no acceptance row is weakened: the same atomic mutation is refuted on the design's capable mixed-unit clause; the valid-only clause stays checked. This is a brief section4 test/plant-target instrument decision. A genuinely required named G-REF plant retirement would still stop to the CEO. Alternative not taken: mutate a dead path, claim an impossible pixel red, add invalid units to the box, or change per-unit behavior. Global79/170; no new agent run.

## C222: Render generic inventory and shortage values as player text

Conflict: Forge HUD2 requires Needs10Shell(n/10), while affordability_reason returns Shell n/10; SelectionPrimary.Inventory returns useful typed array data but Text display_value prints raw JSON. Data-only Format would duplicate fragments and cannot turn inventory arrays into readable lines. These adapter presentations omit required design8.6/14 behavior.

Choice: authorize Task17's narrow generic presentation changes in widgets/sources.rs and the Text display formatter in widgets/mod.rs. Affordability wording uses the actual resolved cost/name and reserved-aware available count to produce Needs <required> <name> (<available>/<required>), with no Forge ids/names/count literals. Preserve cost aggregation and eligibility logic.

Keep SelectionPrimary.Inventory's typed array/ownership guards unchanged. Format its actual inventory rows into readable Text lines using existing port/name/on_hand/reserved/count/capacity fields; do not expose JSON or change unrelated value kinds. Share the display helper across direct Bind and Format paths rather than duplicate it. No new SelectionInventory schema/source is needed. Verify the actual lab UI text/values in HUD2 and retain structured-source tests; do not edit Forge frozen units/maps.

This is the brief section4 nearest existing presentation mechanism/instrument choice. Source ownership overlaps Task15's alert source changes in a separate worktree; root will preserve both at rebase/merge. Alternative not taken: content-specific strings in the engine, raw JSON in the product, a redundant nested reason phrase, or a larger schema extension. Global79/170; no new agent run.


## C223: Distinguish empty ammunition from unknown inventory

Conflict: actual Forge turret7 has an ammo definition and a replicated main inventory state with an empty items array. PrimaryField.Ammo currently returns null when no matching item row exists, so the required HUD2 ammo quantity is blank instead of the known zero. This is not absent/private state.

Choice: Task17 may resolve absent matching ammo item to available0 only when the own unit, ammo definition and actual main inventory state all exist. Keep null for an unknown/missing state or absent definition; keep the foreign guard and reserved-aware quantities. Do not change authored Forge contents or any unit definition. Add empty-versus-unknown/foreign source guards and verify actual empty-turret ammo0 in HUD2.

This is the brief section4 nearest existing source/presentation mechanism choice. Alternative not taken: fabricate an inventory, expose private ammo, change frozen unit data, or show blank for a known empty state. Global79/170; no new agent run.
