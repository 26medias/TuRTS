# MP1 plan gate 2: presentation (bevy-rendering-engineer)

Source: subagent report. Agent output; claims are leads until verified.

VERDICT=PASS-WITH-FINDINGS HIGH=3
(Contract line: VERDICT=pass-with-changes FINDINGS=10 HIGH=3 RUNS=14 NOT_CHECKED=5)

The plan is buildable on Bevy 0.20 with the `webgpu` feature. Three High findings need repair text before their tasks start. None of the three touches lane 0.

## Findings

**H1. P:189, P:887 (High). Design section 8.2 has no owner.**
- D 8.2 (D:2142-2146) covers yaw from movement with a turn rate, turret aim from `Fired`, the infantry bob, and the site that rises with its progress. No task names it (grep "8.2|yaw|aim|turret_node" in P: 0 owners).
- FX1/s2 needs "an enemy turret turns to its target" (D:2450, D:3052). It needs `turret_node` from `PRES-MODELS`, but `FX2` has no edge to `PRES-MODELS`.
- Today the yaw comes only from the authored map facing (`render.rs:385-394`, `:443-451`).
- Repair: add to `FX2`: "D 8.2 in `render.rs` `sync_units`: yaw from movement with a turn rate; a turret aims from `Fired` on `turret_node`, else the whole turret shape turns; infantry bob". Add `PRES-MODELS` as a merge-after of `FX2`. `PRES-MODELS` owns the site rise.

**H2. P:735, P:737, P:907 (High). The client cost rule cannot go red.**
- After S45, `sync_widget_tree` is the exclusive system at `widgets/tree.rs:114`. It clones at `:136`, `:140`, `:144` and `:147`.
- The per-frame writes moved to `widgets/values.rs:39` (`*node =`), `:65` (`current.0 = text`), `:124` (`*current = widget_visibility`) and `widgets/tooltip.rs:97`.
- Why this costs time each frame: `DerefMut` marks the component changed (`bevy_ecs-0.20.0/src/change_detection/traits.rs:496-497`). A changed `Text` is measured again (`bevy_ui-0.20.0/src/widget/text.rs:325`).
- `UI-HUD` names `tree.rs` only and has no red for the rule. So the old per-frame path stays under every new widget.
- VIS3 "HUD on and off" has no threshold. No HUD-off hook exists (grep `hud_off|no_hud|hide_hud`: 0 hits), and no task builds one.
- Repair for `UI-HUD`:
    - Add `values.rs` and `tooltip.rs` to its files.
    - Red: a headless counter test with a static replica for 60 frames. The count of widget entities with `Ref<Text|Node|Visibility>::is_changed()` must be 0 after frame 2. Today's tree is red.
    - Plant: put `current.0 = text` back.
    - Build the hook `hud=off`.
- Repair for `PRES-MEASURE`: record HUD on and off, and the debug state on and off, on the C1 tree. Do this before any model exists.

**H3. P:192, P:890 (High). The `PRES-MODELS` file list misses the real spawn path.** These are the files it must change:
- `render.rs:356-434` (`sync_units`: one shape mesh for each content id; the slot material at `:414`).
- The shape catalogue: `content.rs:44`, `acquire.rs:146-148`, and `turts_mod_runtime/src/visuals.rs:10` (`visuals_by_content`).
- `lib.rs:298`: the memory source must be registered before `DefaultPlugins` (`bevy_asset-0.20.0/src/lib.rs:576-578`, `:628-630`).
- `turts_client_web/src/lib.rs:392`: `publish_scene` keeps only units `With<Mesh3d>`. A model root with no mesh empties `turts_scene.units`.
- Repair: add these files. Add the rule "a unit with no `model` keeps the same entity layout, so the MC rows stay green (D:2113)".

**M1. P:890 (Medium). Bevy 0.19 traps.**
- `SceneRoot` is now `WorldAssetRoot` (`bevy_world_serialization-0.20.0/src/components.rs:15-23`).
- `GltfPrimitive.material` is `Handle<GltfMaterial>`, not a `StandardMaterial` (`bevy_gltf-0.20.0/src/assets.rs:174`). Use the `Material{n}/std` labelled asset (`bevy_pbr-0.20.0/src/gltf.rs:126-138`) or `standard_material_from_gltf_material` (`:33`).
- Repair text: "shared mesh and material handles for each model; one slot material for each slot; no material for each unit". In the validator, use `gltf` with `default-features = false, features = ["names", "utils"]`. My probe built with these features.

**M2. P:896, P:916 (Medium). No task runs the stage-1 pixel rows again on the model build, and no task owns the oracle re-calibration.**
- B2 runs the stage-2 rows only. `REG2-2` runs the MC rows only.
- The oracles test the slot colour at `ground_screen_center` (`tests/browser/pt1-oracle.js:406`, radius 10) or take a median hue (`vis-oracle.js:32-43`).
- The 15 % team-colour cover is advice, not a check (D:2137-2138).
- Repair: `I2-PIX` runs GHO1, FX1/s1 and GOAL1 again with models. `PRES-ART` changes the finders to count slot pixels inside `pick_radius`. `pick_radius` comes from the footprint (`render.rs:50-60`), so models do not change it.

**M3. P:893 (Medium). `WEB-NOGPU` is not concrete.**
- `start()` builds the app at `turts_client_web/src/lib.rs:144` and runs it at `:173`. Nothing probes for an adapter first.
- With the `webgpu` feature the only backend is `BROWSER_WEBGPU` (`bevy_render-0.20.0/src/settings.rs:78-79`). There is no WebGL2 fallback.
- Bevy panics at `renderer/mod.rs:308`. `panic = "abort"` (`Cargo.toml:43`), so nothing can catch the panic.
- Repair: before `:144`, read `navigator.gpu` through `js_sys::Reflect` (the pattern of `:181-183`) and await `requestAdapter()` (`wasm-bindgen-futures` is already a dependency, `crates/turts_client_web/Cargo.toml:17`). If it is absent or null, put the D 7.7 text in `#wrap` and do not build the app.

**M4. P:991 (Medium). Client per-frame sites that no task owns.**
- `ContentId::parse` runs for each unit in each frame at `visuals.rs:368`, `:446`, `:567`, `visuals/health.rs:74` and `visuals/world_tooltip.rs:28` (D:2232-2236).
- The clones at `widgets/cursor.rs:32-40` and `feedback.rs:393` (D:2266) also have no task.
- Repair: `UI-PLACE` owns the `cursor.rs` clones. `UI-ALERT` owns `feedback.rs:393`. `L-FX-PRES` owns the visuals index, built at install.

**L1. P:252-271 (Low). Gaps in the conflict list.**
- `L-FX-PRES` needs `turts_debug.effects.recent` (D:2450) but does not name `debug.rs`. `UI-HINT` edits `debug.rs` in the same window (P:216).
- The `visuals.rs` plugin is shared by `L-FX-PRES` and `UI-PLACE`.

**L2. P:887 (Low).** The FX2 positional gain should be computed as `Volume::Linear` and not use `SpatialAudioSink`. The rodio falloff to 0 at `audible_range` is NOT TRACED.

**L3 (Low).** `CLAUDE.md` names `visuals_by_unit`. The code name is `visuals_by_content` (`turts_mod_runtime/src/visuals.rs:10`). This is for `REC`.

## Presentation task table

| Task | Bevy 0.20 API it needs | Exists |
| --- | --- | --- |
| L-FX-PRES, FX2 | `Visibility::Hidden` pool | yes, `bevy_camera-0.20.0/src/visibility/mod.rs:83-90` |
| | `set_if_neq` | yes, `bevy_ecs .../traits.rs:221` |
| | `Gizmos::line` (tracer) | yes, `bevy_gizmos-0.20.0/src/gizmos.rs:403` |
| | `unlit` material | yes, `bevy_pbr .../pbr_material.rs:677` |
| | `world_to_viewport` ("+N") | yes, `bevy_camera .../camera.rs:580` |
| UI-PLACE | gizmo rings and the `RingOverlayMaterial` pattern | yes, `visuals.rs:59` |
| | stripes material | NOT TRACED |
| UI-HUD, UI2-HUD | `PartialEq` on `Text`, `Node`, `Visibility` | yes, `bevy_ui .../text.rs:128`, `ui_node.rs:485` |
| UI-ALERT, FX2 | `AudioPlayer`, `PlaybackSettings`, `Volume` | yes, `bevy_audio-0.20.0/src/audio.rs:57`; used at `audio.rs:451-452` |
| PRES-MODELS | `MemoryAssetReader` | yes, `memory.rs:290` |
| | `Dir::insert_asset` | yes, `:64`, `From<Vec<u8>>` at `:333` |
| | `register_asset_source` | yes, `bevy_asset lib.rs:580` |
| | `Gltf.named_materials`, `named_nodes` | yes, `assets.rs:30`, `:34` |
| | `is_loaded_with_dependencies` | yes, `server/mod.rs:1163` |
| | `bevy_gltf` is in the `3d` feature | yes, `bevy Cargo.toml:2743-2749` |
| | `gltf 1.4.1` | yes, `Cargo.lock:3929-3930` |
| | `SceneRoot` | no (0.19); it is `WorldAssetRoot` |
| WEB-NOGPU | none from Bevy (a JS probe) | the panic is at `renderer/mod.rs:308` |
| L0-DBG, UI-RESULT, UI-HINT, PRES-MEASURE | no new render API | - |

## Asset proposal

**Stage 1.** Keep primitives (P1 working text). `Shape` has only `Capsule` and `Cuboid` (`schema.rs:48-51`). So `C-UNITS` gives each type a distinct size class: three capsule heights for the infantry, a low wide hauler, a tank cuboid, a tall thin turret, and large buildings. The slot material (`render.rs:414`) separates own units from enemy units.

**Stage 2 (`PRES-ART`, bevy-rendering-engineer; review ux-designer).**
- Use a Python script with the standard library only, in the repository, that writes GLB files deterministically. It makes 9 models, with materials `body` and `team` and nodes `turret` and `muzzle`, and no textures, so the decoded-image cap is 0 bytes.
- My probe built one tank this way: 72 triangles, 6,628 bytes. All the D 8.1 checks passed with `gltf =1.4.1` (pivot, scale 0.986 for footprint radius 0.9, 2 materials, node names). The pivot plant went red with "min_y=-0.400".
- The same script with a subdivision factor and an embedded PNG makes the placeholders at the caps for `PRES-MEASURE`.
- Blender 5.3.0 is on this machine (`/snap/bin/blender`). Use it only for better art; whether its output bytes are stable is NOT TRACED.

## Risks and early probes

1. **`PRES-MODELS`.** Probe: a headless app with `AssetPlugin`, a memory source and `GltfPlugin` loads `tank.glb` from bytes, then prints the `named_nodes` and the `Material0/std` handle.
2. **`UI-HUD`.** Probe: the H2 counter test on today's tree. It must show changed `Text` on each frame.

## What I ran

All scratch files are in `/tmp/claude-1000/-home-julien-Projects-TuRTS/bf418660-b272-4bb3-8628-8d9ef2677d1d/scratchpad/gate2-render/`.

| Command | Claim tested | Would refute | Observed |
| --- | --- | --- | --- |
| `git diff --stat 8d6902c..d047433` on lib, debug, replica, web-lib, render, audio, feedback | P:94: S45 does not touch them | any file listed | empty |
| `wc -l` on the S45 tree | P:268: visuals/ has own files | - | `visuals.rs` 934 lines stays; `render.rs` is not split |
| grep in `tree.rs`, `values.rs` | P:735: the fix is in `tree.rs` | writes in `tree.rs` only | writes are in `values.rs:39`, `:65`, `:124` |
| grep in `bevy_ecs` and `bevy_ui` 0.20 | D:2268: `set_if_neq` is needed | `DerefMut` does not mark changed | it marks changed (`traits.rs:497`); measure runs again (`text.rs:325`) |
| grep in `bevy_gltf`, `bevy_pbr`, `bevy_world_serialization` 0.20 | D:2109-2112 is buildable | API missing | present; `SceneRoot` renamed |
| grep in `bevy_render` settings and renderer | NOGPU1 can be caught | a WebGL2 fallback or a recoverable error | `BROWSER_WEBGPU` only; `expect` panic |
| grep `With<Mesh3d>` in the web bridge | the bridge does not depend on the spawn | no filter | filter at `lib.rs:392` |
| grep for a HUD-off hook | VIS3 HUD off is runnable | a hook exists | 0 hits |
| grep `8.2\|yaw\|aim` in P | 8.2 has an owner | an owner | none |
| oracle grep (`pt1-oracle.js`, `vis-oracle.js`) | models do not break the finders | a finder that does not depend on colour | centre-colour finder |
| `make_tank.py` and the `glb_probe` run (`--offline`; no lockfile in scratch) | the D 8.1 checks are computable | a check not computable | all OK, `exit=0` |
| the same probe on the pivot plant | the check can go red | `pivot_ok=true` | `false` |
| `which blender` | the art path exists | absent | 5.3.0 |

## Not checked

- Browser runs (not allowed).
- A Bevy load of the GLB from memory (it is risk probe 1).
- The rodio spatial falloff.
- The `UI-PLACE` stripe material.
- Whether `app.run()` works inside `spawn_local`.

## What I changed

none

## Open questions for the orchestrator

- Should `FX2` or `PRES-MODELS` own the turret aim of D 8.2? I recommend `FX2`, with `PRES-MODELS` as merge-after.

