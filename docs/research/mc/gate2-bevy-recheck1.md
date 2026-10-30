VERDICT=fail HIGH=2

Recheck 1 of gate 2 (engine and presentation lens). Plan commit `9910798`, design commit `89e1fa3`. In this file, "plan:N" is a line of the plan at `9910798`.

Original findings:
- F1 is fixed. Task 9 removes the fit and the fixed start (plan:481, :117). Task 11 owns `turts_interaction/src/camera.rs` (plan:171, :527). Task 10 needs task 11 (plan:151; wave 7).
- F2 is fixed. Task 8 adds `Minimap` and `over_widget` (plan:387). Task 12 adds `hit.rs` (plan:540). Task 14 fills the core `minimap.rs` (plan:599). Task 9 adds the test `click_over_widget_is_not_a_ground_click` (plan:483).
- F3 is not fixed (High, new). The plugin list for PROJ1 at plan:483 is `MinimalPlugins`, `AssetPlugin` and `RenderPlugin` (no backend). In my probe it panics at `bevy_render-0.20.0/src/mesh/mod.rs:64` (`Assets<Mesh>` is missing; exit 101). The probe passes when I add `ImagePlugin`, `WindowPlugin { primary_window: None }`, `TransformPlugin` and `MeshPlugin`: at most 0.0001 px, exit 0. With those plugins, the plant at a field of view of 0.9 gives 55.90 px. The number 132.99 px at plan:487 is out of date (it came from a field of view of 0.6).
- F4 is not fixed (High). Each `frames` record holds no time and no input (plan:479, design:1145-1147). So the oracle cannot find "the first presented frame after the mouse-up", and the "one frame late" plants (plan:573, :617) stay green. Fix: add the wall time and the input events that the core used in that frame to each record.
- F5 is fixed (plan:301, :645).
- F6 is fixed (plan:584).
- F7 is fixed: CLICK9 (plan:481, :484).
- `Selection`: R5 moves it to `ui_shared.rs` with its `init_resource` (plan:117). Low: the plan does not name the plugin that sets it up. After task 9 replaces `input.rs`, no task writes `Selection`, which `render.rs:540` reads.

New defect (Medium): task 9 removes the fixed start (plan:117, :481). The start that comes from `CameraDef` arrives only in task 11 (plan:527). The plan gives no default `CameraState` for task 9, but T7 (G-WEB) and CLICK9 need the units on the screen. Fix: in task 9, use today's pose (`render.rs:336-342`) as the default.

Runs:
- `proj1probe plan`: exit 101 (the panic above).
- `proj1probe plan_img`: exit 101.
- `proj1probe plan_img_win`: exit 101.
- `proj1probe plan_full`: exit 0.
- `proj1probe plan_full --plant`: exit 1, 55.90 px.
