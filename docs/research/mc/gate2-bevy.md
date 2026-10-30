VERDICT=fail FINDINGS=8 HIGH=4 RUNS=9 NOT_CHECKED=5

# Gate 2, engine/presentation lens (bevy-rendering-engineer)

Artefact: `docs/superpowers/plans/2026-10-29-mc-player-control.md` (716 lines, commit `f69badf`).
Scope: tasks 1, 9, 10, 11, 12, 13, 14, 15, 16, 21. Bevy sources: `B=~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f`.
Scratch: `/tmp/claude-1000/-home-julien-Projects-TuRTS/0e0457bb-045c-4b6a-945f-1db6a0753ee2/scratchpad/mc/gate2/bevy/`.
Note: the orchestrator asked for this file at `docs/research/mc/gate2-bevy.md`. My review-mode rule forbids writes to the repository, so it is here. Copy it if you want it in the tree.

## Findings

### F1 High: two owners of the camera from task 9 to task 11; task 11 cannot edit the core
- Plan:173 gives `crates/turts_interaction/src/*.rs` to tasks 8 and 9 only. Plan:449 gives task 9 "7 (state only)". Plan:492 tells task 11 to put pan, edge band, zoom limits, bounds and Space centre in `crates/turts_client/src/camera.rs`, but its test is `crates/turts_interaction/tests/camera.rs` (headless, "speed 2x gives 2x distance", plan:495). D2.2 (design:254-256) and D7 (design:672) put the camera state in the core. So the task 11 builder must edit core files outside its list, or put camera logic in Bevy, where the headless test cannot reach it.
- Task 9 copies `CameraState` into the Bevy `Transform` (plan:451), but `fit_scene_camera` still rewrites the `Transform` each frame in `PostUpdate` (`crates/turts_client/src/render.rs:281`, `:454-458`) while `UiPanelRoot` exists (`render.rs:251-253`; the panel goes only in task 12). The core picks with its own projection (design:368), so from task 9 to task 11 the browser picks against a camera that is not drawn.
- Task 10 (HD1 pixels half) runs in wave 5 in parallel with task 11 (plan:122), so its browser half runs with the fit camera.
- Fix: move the removal of `fit_scene_camera` and of the fixed start (`render.rs:245`, `:336-346`, `:454-458`) into task 9 (it already edits `render.rs`). Give task 11 `crates/turts_interaction/src/camera.rs`. Make task 10 need 11.

### F2 High: the `InputEvent` contract cannot carry a minimap point, or a click that a widget took
- Task 8 fixes `InputEvent` "exactly as D section 2.2" (plan:369). Its variants are listed at design:245-247. `UiAction { name }` has no point.
- MM1 (task 14, plan:552) needs a left click or drag on the minimap to move the look-at point, and a right click to send the smart command at a world point (design:688-689, :1206). The camera and the order composition are in the core (design:231, :257). No event can carry a world point, and task 14 cannot edit `turts_interaction/src` (plan:173).
- No task says what happens to a `PointerDown` on a mod button. The core reads it as a ground click, and "Left click on the ground: clear the selection" (design:367). The card then hides (`visible: [SelectionNotEmpty, ...]`, design:782). Today the click path skips the panel rectangle (`crates/turts_client/src/input.rs:29-37`, `:52-60`). A grep of the design and the plan for a click-through rule found none.
- Needs decision: (a) add `MinimapPoint { button, world }` and an `over_widget` flag (or `PointerDown` suppression in the adapter) to D2.2, done in task 8 or 9; (b) the adapter makes up screen events for the minimap (this fails for a point outside the screen); (c) move MM1 to a later milestone.

### F3 Medium: PROJ1 trace (plan:454, :676, :716 mark it NOT TRACED): no GPU adapter is needed, but "no render plugin" gives a false red
- `camera_system` is in `bevy_render` (`$B/bevy_render-0.20.0/src/camera.rs:354`), not in `bevy_camera`. Only `bevy_render::camera::CameraPlugin` registers it (`camera.rs:66-79`), and `RenderPlugin` adds that plugin (`$B/bevy_render-0.20.0/src/lib.rs:406-407`). `camera_system` reads `Assets<Image>` and `ManualTextureViews` only (`camera.rs:354-364`, `:289-296`).
- Probe: with no render plugin, all 60 points give `NoViewportSize`. A builder can read that as "needs a GPU adapter" and send PROJ1 to an ignored local test (plan:454) and a task 24 `browser` step (plan:661).
- Probe: `RenderPlugin` with `WgpuSettings { backends: None }` (`settings.rs:270-272`; the render sub-app is skipped at `lib.rs:393`), or `camera_system` and `ManualTextureViews` added by hand, gives at most 0.0001 px on all three viewports. The plant (field of view not copied) gives 132.99 px.
- Probe: `Backends::empty()` panics with "Unable to find a GPU!" (panic at `renderer/mod.rs:308`, text at `:160`). The default backends on this GPU host give the right result but exit with 139 (a segmentation fault at teardown).
- Fix: write into step 9.2 that PROJ1 uses `backends: None` and calls `app.finish(); app.cleanup();` before `update()`. The image keeps `MAIN_WORLD` (`camera.rs:334`). Scale 2 uses `ImageRenderTarget { scale_factor: 2.0 }` on a 2560 x 1440 image (`$B/bevy_camera-0.20.0/src/camera.rs:987-993`). PROJ1 then runs in the `check` job; remove the task 24 PROJ1 branch.

### F4 High: the "a frame late" plants of SEL2 and ORD7 cannot go red in the browser
- Plan:536 and plan:538 (SEL2), and plan:568-569 (ORD7), read "the first presented frame" from the frame log. The frame log is a native file (`crates/turts_client/src/framelog.rs:27-33`). It records unit positions only (`framelog.rs:3-4`). `crates/turts_client_web/src/lib.rs` has no frame-log hook (grep: no hit).
- A screenshot taken after the mouse-up cannot show a delay of 16 ms. So "selection applied one frame later (SEL2 fails)" stays green.
- Fix: in task 9 (R5), add a browser frame record, for example `turts_debug.frames`: a ring of (frame index, time, selection count, ring count, marker count). Or put the plant in the core and gate it with a headless test that requires the effect in the same `step`.

### F5 High: CAM2 "Esc ends the lock" cannot go green in headless Chrome
- Probe (`webprobe/probe2.js`): in headless Chrome, a click takes the lock, and `movementX` and `movementY` carry the motion while `clientX` stays fixed. After CDP Escape, the lock stays (`LOCKED_AFTER_ESC true`), and the keydown reaches the page. In a real browser, the first Esc ends the lock (design:319).
- The CAM2 row (design:1200, plan:239, owner task 16) runs in the `browser` job. It is red on a correct build.
- Fix: the CAM2 pixels row checks only the lock start, the cursor, the edge band and F10. The Esc clause comes from the P1 headed table (task 1 step 3, plan:279).

### F6 Medium: HD1 (task 10): the plan gives the pixels-only driver no way to map a world point to the screen
- `hd1-driver.js` has role `driver` (plan:480), so it cannot read `turts_scene`. The actions hold `world_rect`, `world_point` and `tick` (design:268-271).
- The plan does not give: the projection literals (camera start, viewport 1280 x 720 at scale 1, a `pure` helper); how a browser waits for a server tick; how the edge band is kept quiet (a point within 8 px of the edge pans the camera).
- Fix: give these in task 10, after F1.

### F7 Medium: no green test exercises the world click path from task 9 until task 13
- Plan:458 says T7 "still selects with a left click and moves with a right click". `tests/browser/t7.js` has no mouse call (grep `mouse|click|keyboard`: no hit, exit 1). It moves through `window.turts_move` (`t7.js:16`, `:214`).
- `m2.js` clicks (`m2.js:87`, `:107-112`), but its CI step is already red (plan:88).
- Fix: correct the step text, and add a small click row to task 9, or accept the gap in writing.

### F8 Low (four items)
- (a) glam 0.33.12 marks `Mat4::perspective_infinite_reverse_rh` deprecated. Strict clippy then fails on a stub that uses it (probe). Use `glam::camera::rh::proj::directx::perspective_infinite_reverse`. Bevy uses the same math (`$B/bevy_camera-0.20.0/src/projection.rs:339`).
- (b) winit 0.30.13 already sets `tabindex="0"` (`winit-0.30.13/src/platform_impl/web/web_sys/canvas.rs:100-108`). It also focuses the canvas on each pointer down when prevent-default is on (`web_sys/pointer.rs:116-120`). The task 16 work for these two items is redundant. For KM2, a driver click at (-5, -5) blurs the canvas while `document.hasFocus()` stays true (probe `focus.js`). So the shell must use the canvas blur (Bevy `WindowFocused`), not the window blur.
- (c) `scripts/native-view-proof.sh:143-213` reads `selection == unit_id` and the `ui` rows. Task 9 (a list selection) and task 12 (the panel removal) break it. The script is not in CI, and the plan does not name it.
- (d) MM1 "Expected: no minimap widget" (plan:552) is wrong after task 12, because HUD1 renders a `MinimapView` at bottom right (plan:507). The red message changes.

## What I ran

| command | claim tested | result that would refute | observed |
| --- | --- | --- | --- |
| `cargo build` of `proj1probe` (Bevy `=0.20.0`, the client feature set `3d`, `ui`, `default_font`; repo `Cargo.lock`; offline) | plan:454 PROJ1 is buildable on 0.20 | compile error on the API | exit 0 (one fix: `RenderCreation::Automatic` takes a `Box`) |
| `proj1probe bare` | plan:454 "no render plugin" works | errors = 0 | exit 1, `NoViewportSize` x60 |
| `proj1probe manual`, `proj1probe render_none` | camera_system needs no adapter | errors > 0 or > 1 px | exit 0, worst 0.0001 px on 1280x720, 1920x1080, 1280x720@2 |
| `proj1probe manual --plant`, `render_none --plant` | PROJ1 plant plan:457 goes red | worst <= 1 px | exit 1, 132.99 px |
| `proj1probe render_empty`, `render_auto` | default RenderPlugin needs an adapter | no panic with no backend | 101 "Unable to find a GPU!"; auto: result ok, exit 139 |
| `cargo check` of a `turts_interaction` stub, host and wasm32, plus `cargo tree` | plan:371 deps build with no Bevy | non-zero exit; bevy, wgpu or winit in the tree | exit 0 and 0; count 0; deprecation warning |
| `node webprobe/probe.js`, `probe2.js` (playwright-core 1.64.1, Chrome headless) | CAM1 pointer-out (plan:494); CAM2 lock (design:1200) | no pointerout; Esc ends the lock | pointerout at y=-5; lock on click; Esc keydown reaches the page, lock stays |
| `node webprobe/focus.js` | KM2 focus-loss step is drivable | no blur | click (-5,-5) blurs the canvas; `hasFocus` true |
| `driver_rule.py` on a planted and a clean scratch `t7-driver.js` | DRV1 plant plan:633 | exit 0 on the plant | plant exit 1 (DR2 A1, A3); clean exit 0 |

Greps: the users of `economy_ui` (`input.rs:5`, `render.rs:17,247,540`, `debug.rs:66,113`, `live_view.rs:22,404`, `turts_client_web/src/lib.rs:35`); `Selection` init only in `EconomyUiPlugin` (`economy_ui.rs:1291`), so R5 must move the `init_resource` with the type; the CI steps (`ci.yml:108-124`); the `custom_cursor` feature is in `3d`, through `default_platform` (`$B/bevy-0.20.0/Cargo.toml:2725-2731`, `:2887-2898`), so HUD4 needs no new feature; the `UiScale` citation holds (`bevy_ui-0.20.0/src/update.rs:57-62`); `UiTransform` `Val::Percent` is a percent of the node size (`ui_transform.rs:23-29`), so a pivot can be expressed.

## Not checked
- `bash scripts/t7.sh` and the trunk release build: not run (time). NOT TRACED.
- The wasm32 build with the Bevy feature `audio` (task 15) and the resume hook: not run. NOT TRACED.
- VIS2, palette.rs, the ten primitives beyond placement, the RENDER_WORLD-only image path at run time: read only.
- The task 21 rewrites, except the DRV1 plant.
- The CSS cursor limit of 128 px in Chrome and Firefox. NOT TRACED.

## What I changed
none (scratch files only).

## Open questions for the orchestrator
1. F2: which contract change for minimap points and widget click-through (options a, b, c)? It changes D2.2, the design of record.
2. F1: accept the move of the camera removal into task 9, and "task 10 needs 11"? The critical path changes.
