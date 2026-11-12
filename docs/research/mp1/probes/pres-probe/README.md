# PRES-PROBE: two presentation risk probes (MP1, lane 0)

Source: agent output (bevy-rendering-engineer, test mode). Every number comes from a run listed below. The raw output
is in `raw/`. `NOT TRACED` marks what no run tested.

Machine: this laptop, Linux 7.0, 28 logical CPUs. The load average was 12 to 34 during the runs (other lanes were
building). Timings are therefore noisy. Bevy 0.20.0 (`bevy = "=0.20.0"`; the registry also holds 0.20.1, which the
lock does not use). Rust 1.99.1.

## Results

| Question | Answer |
| --- | --- |
| Probe 1: a model loads from package bytes on Bevy 0.20, native, headless? | **YES** (no GPU, no window, no `RenderPlugin` needed) |
| Probe 1: the same code compiles for wasm32? | **YES** (`cargo check --target wasm32-unknown-unknown`, with and without `bevy_pbr`) |
| Probe 1: can ONE team material be replaced for each slot with one shared handle? | **YES** (shown on a spawned scene and on a flat template) |
| Probe 2: does the HUD change `Text`, `Node` and `Visibility` in each frame on today's tree? | **YES**, in every frame (red) |
| Probe 2: can the counter be a normal headless test? | **YES** (`cargo test`, no GPU, 1.4 s) |
| Probe 2: does the change cost time? | **YES**: `app.update()` takes 2.8 times as long in the dev profile (43 to 44 ms against 15 to 16 ms) and 1.6 times as long in release (2.8 to 3.0 ms against 1.7 to 1.8 ms) |

## Probe 1: a model from package bytes

Sources: `glb_load/Cargo.toml`, `glb_load/src/main.rs`. GLB: `make_tank.py` of the gate (sha256
`6acfa9c6...e660`, 6,628 bytes); the bad pivot: `make_tank_bad.py` (sha256 `6c454a93...7fb6`).

How the probe pins Bevy: its own `[workspace]`. `Cargo.lock` is a copy of the workspace `Cargo.lock` of the lane
(`51f1223`); `cargo build --offline` prunes it. The result has `bevy 0.20.0` and `gltf 1.4.1` and no 0.20.1 package
(`glb_load/Cargo.lock`). The commands: `cargo build --offline [--features pbr]`; `cargo check --offline --target
wasm32-unknown-unknown [--features pbr]` (exit 0 for both; raw in `raw/p1_wasm_check_*.txt`). The `--locked` flag
cannot work in a pruned lock (first try: exit 101, "cannot update the lock file").

### Plugin sets that work (both load `tank.glb` in 1 update after `finish()`)

1. **No render types** (feature set `bevy_gltf`, `bevy_asset`, `bevy_world_serialization`, `bevy_mesh`,
   `reflect_auto_register`, `debug`): `TaskPoolPlugin`, `AssetPlugin`, `TransformPlugin`, `MeshPlugin`,
   `WorldSerializationPlugin`, `GltfPlugin`. `bevy_gltf` does not depend on `bevy_render` or `bevy_pbr`
   (`bevy_gltf-0.20.0/Cargo.toml:56-166`). Result: `named_nodes`, `named_materials`, meshes, `GltfMaterial` assets, and a
   spawned hierarchy, but **no `StandardMaterial` and no `Material{n}/std` asset**: the extension handler that makes
   them is in `bevy_pbr` (`bevy_pbr-0.20.0/src/gltf.rs:101-138`; installed at `bevy_pbr-0.20.0/src/lib.rs:250-252`).
2. **With `bevy_pbr`** (add `bevy_pbr`, `bevy_render`): the set of 1, plus `WindowPlugin { primary_window: None }`,
   `CameraPlugin`, `ImagePlugin`, `RenderPlugin { render_creation: WgpuSettings { backends: None, .. }.into() }` and
   `PbrPlugin`. No GPU is touched. This is the set that gives `Material{n}/std`.

Three ordering and setup rules the probe found (each one made it panic or hang first):

- `GltfPlugin` before `PbrPlugin`. `PbrPlugin::build` reads the resource `GltfExtensionHandlers`, which
  `GltfPlugin::build` inserts (`bevy_pbr-0.20.0/src/gltf.rs:26`; `bevy_gltf-0.20.0/src/lib.rs:284`). The other order
  panics: "Requested resource ...GltfExtensionHandlers does not exist". `DefaultPlugins` has the right order
  (`bevy_internal-0.20.0/src/default_plugins.rs:77`, `:79`).
- `app.finish(); app.cleanup();` before the first `update()` in a test that does not call `app.run()`.
  `GltfPlugin` registers the real loader in `finish()` (`bevy_gltf-0.20.0/src/lib.rs:287-313`); until then the load
  stays in `LoadState::Loading` (200 updates, no result: first try).
- To spawn a scene (`WorldAssetRoot`) the type registry must hold every component type. With the Bevy default
  features off, the spawn panics: "world contains the unregistered type `Transform`"
  (`bevy_world_serialization-0.20.0/src/world_asset_spawner.rs:635`). The feature `reflect_auto_register` cures it.
  `3d` enables it through `default_app` (`bevy-0.20.0/Cargo.toml:2873-2879`), so the client has it. On wasm32 it uses
  `inventory` (`bevy_reflect-0.20.0/src/lib.rs:525`); whether it works in a browser is `NOT TRACED`. The flat
  template of D 8.1 never spawns a scene, so it does not need this.

### The memory source and the late bytes

- Register before `AssetPlugin`: `app.register_asset_source("pkg", AssetSourceBuilder::new(move ||
  Box::new(MemoryAssetReader { root: dir.clone() })))` (`bevy_asset-0.20.0/src/lib.rs:580`, `:624-642`;
  `io/source.rs:153-155`; `io/memory.rs:290-293`). The rule text is at `lib.rs:576-579`; the `error!` for a late
  registration is at `lib.rs:631`.
- `Dir` is `Arc<RwLock<..>>` (`io/memory.rs:40-41`), so a clone that the app keeps can receive bytes AFTER the app is
  built: `dir.insert_asset(Path::new("models/<lock>/<pkg>/tank.glb"), bytes)` (`memory.rs:64`, `From<Vec<u8>>` at
  `:333`). The probe built and updated the app 5 times with an empty `Dir`, then inserted the bytes, then loaded: it
  worked (`raw/p1_run_*_late.txt`). This answers the order problem: the client builds its app (`lib.rs:298`) before
  the package is acquired. Keep the `Dir` clone in a resource and insert the model bytes at acquire time.
- Load: `asset_server.load::<Gltf>("pkg://<path>")` (`server/mod.rs:364`); wait with
  `AssetServer::is_loaded_with_dependencies(&handle)` (`server/mod.rs:1163`). It was true after 1 update.
- A path is cached by the asset server. Put the lock hash in the path, or a second package with the same file name
  gets the first file. (Reason from the cache design; the probe did not test a clash: `NOT TRACED`.)

### What the loaded `tank.glb` holds (raw: `raw/p1_run_pbr.txt`)

| Item | Value |
| --- | --- |
| `Gltf.named_nodes` (`bevy_gltf-0.20.0/src/assets.rs:34`) | `hull`, `muzzle`, `tank`, `turret` |
| `Gltf.named_materials` (`assets.rs:30`) | `body`, `team` |
| `Gltf.scenes` | `Vec<Handle<WorldAsset>>`, 1 scene (`assets.rs:20`) |
| `GltfNode.transform` (`assets.rs:110`) | `turret` at (0, 0.4, 0); `muzzle` at (0, 0.12, 0.85), a child of `turret` |
| Triangles from `Mesh::indices().len() / 3` | 36 + 12 + 12 + 12 = **72** (same as the gate's `gltf` probe) |
| `GltfPrimitive.material` (`assets.rs:174`) | **`Option<Handle<GltfMaterial>>`**, not a bare `Handle` |
| Material labels (`bevy_gltf-0.20.0/src/label.rs:78-88`) | `Material0` = body, `Material1` = team. Standard material: `Material0/std`, `Material1/std` |
| Load a standard material | `asset_server.load::<StandardMaterial>("pkg://<path>#Material1/std")` loaded in the pbr set (`base_color` white for `team`). Without `PbrPlugin` there is none |

The bad pivot: the gate's `glb_probe` (the `gltf =1.4.1` D 8.1 check, `raw/p1_pivot_check.txt`) gives `min_y=-0.400
... pivot_ok=false` for `tank_bad.glb` and `min_y=0.000 ... pivot_ok=true` for `tank.glb`. The probe returns the
boolean, exit 0 both times; the `ModError` text of D 8.1 (`lowest point y = -0.40; the pivot must be at ground level`)
is the validator's job (`PRES-MODELS`). The Bevy load of the bad file also succeeds (`raw/p1_run_nopbr_bad.txt`): the
glTF loader does not check the pivot, so the pack-time check is the only gate.

### Spawn a unit, and the per-slot team material

`WorldAssetRoot(Handle<WorldAsset>)` (`bevy_world_serialization-0.20.0/src/components.rs:15-23`), with
`Gltf.default_scene`. After 4 updates the hierarchy of one unit is (pbr set):
`root [WorldAssetRoot, WorldInstance, Children]` > `Scene0` > `tank` > `hull` (`hull.body`, `hull.team`) and `turret`
(`turret.team`, `turret.body`, `muzzle`). A mesh entity has `Mesh3d`, `MeshMaterial3d<StandardMaterial>`, `Aabb`,
`GltfMeshName`, `GltfMaterialName`, `Name`. `GltfMaterialName` (`bevy_gltf-0.20.0/src/assets.rs:346`, set at
`loader/mod.rs:1741`) is the key to find the team parts. `muzzle` is an entity with only `Transform` and `Name`.

The mechanism (raw: `raw/p1_run_pbr.txt`, the lines after "after the swap"):

1. Spawn two units of one model. Result: the two units use the SAME `Handle<StandardMaterial>` on each mesh entity.
   `Assets<StandardMaterial>` held 3 assets before and 3 after the spawn: **no material for each unit**.
2. For each mesh entity with `GltfMaterialName == "team"`, insert `MeshMaterial3d(slot_handle)`. One handle is shared by
   all units of the slot. Result: team entities of unit A all on `slot_red`, of unit B all on `slot_blue`; the `body`
   entities of A and B still share one handle; the asset count rose by 2 (the two slot materials), not by the number
   of units.
3. The flat template (D 8.1, D:2118): build it once from `Assets<Gltf>`, `Assets<GltfNode>`, `Assets<GltfMesh>`:
   for each primitive keep `(node path or transform, Handle<Mesh>, is_team)`; `is_team` is
   `primitive.material == Gltf.named_materials["team"]`; the body material handle is the label handle
   `Material0/std`. Spawn one root and one child entity for each part with `Mesh3d(mesh)` and
   `MeshMaterial3d(slot or body handle)`. The probe did this: root + 4 children, 0 new materials for the unit.
   No `WorldAssetRoot`, no `reflect_auto_register`, no `Scene0` level.
   **Keep the `turret` node as its own entity** (and `muzzle` as a child with its local transform): the turret aim of
   D 8.2 and the muzzle point of `Fired` need them. The probe flattened nodes into world transforms to show the
   mechanism only; `PRES-MODELS` must keep the node tree for `turret_node` and `muzzle_node`.

A unit with no model keeps the present primitive path: `render.rs:356-434` `sync_units` spawns `(Mesh3d(mesh),
MeshMaterial3d(slot material), Transform, PresentedUnit)` at `render.rs:424`; the slot material is the shared handle
`SlotMaterials.0[owner_slot]` (`render.rs:269`, `:414`). So the primitive path already shares one material for each
slot. The model path must use the same `SlotMaterials` handle for the `team` parts, so own and enemy colours stay
the same across the two paths.
The web bridge keeps only entities `With<Mesh3d>` (`turts_client_web/src/lib.rs:392`): keep `Mesh3d` on the unit root
(for example the hull mesh), or the root is not in `turts_scene.units` (gate H3).

### Names that are wrong or incomplete in the plan or the design for Bevy 0.20

| Where | It says | What is true (Bevy 0.20.0) |
| --- | --- | --- |
| plan P:624 (PRES-PROBE) | "`AssetPlugin`, a memory asset source ..., and `GltfPlugin` loads `tank.glb`" | Also needs `TaskPoolPlugin`, `MeshPlugin`, `WorldSerializationPlugin` (`bevy_internal-0.20.0/src/default_plugins.rs:33`, `:35`, `:50`), and `finish()`/`cleanup()` before the first update. |
| plan P:624 | "Print ... the `Material0/std` labelled asset handle" | `Material0/std` exists only with `PbrPlugin` (`bevy_pbr gltf.rs:124-138`), which needs `RenderPlugin`. It works headless with `WgpuSettings.backends = None`. `GltfPlugin` must come before `PbrPlugin`. |
| plan P:1247 (PRES-MODELS) | "the model root is `WorldAssetRoot`" | D:2118 says the client spawns no scenes (flat template). `WorldAssetRoot` needs `reflect_auto_register` and adds a `Scene0` entity. Use the flat template; drop `WorldAssetRoot` from the row, or name it as the fallback. |
| plan P:1247, gate M1 | "not `GltfPrimitive.material`" (gate: `Handle<GltfMaterial>`) | The field is `Option<Handle<GltfMaterial>>` (`assets.rs:174`). The `GltfMaterial` handle is the key for `named_materials` equality; the `StandardMaterial` comes from the label `Material{n}/std`. |
| plan P:1247 | "a memory asset source registered before `DefaultPlugins` in `lib.rs:298`" | True, but at that point the package bytes do not exist. Register an empty shared `Dir`; insert the bytes at acquire time (shown). `lib.rs:298` is in the non-headless branch; the headless branch (`MinimalPlugins`, `lib.rs:293-296`) has no `AssetPlugin`, so a headless client has no models. |
| plan P:1247 | `bevy_asset-0.20.0/src/lib.rs:628-630` | The `error!` is at `lib.rs:630-632`; the rule text is at `:576-579`. |
| design D:2115 | `MemoryAssetReader` at `io/memory.rs:290` | Correct. |
| gate M1 (the source of the plan text) | `SceneRoot` is renamed `WorldAssetRoot` | Confirmed: `components.rs:15-23`. Scenes are `WorldAsset`, not `Scene` (`Gltf.scenes: Vec<Handle<WorldAsset>>`, `assets.rs:20`). The design text (D:2115-2118) names neither; it says "does not spawn scenes". |

The glTF scenes are `WorldAsset` (crate `bevy_world_serialization`), not the scenes of `bevy_scene`. The feature `scene`
turns on both crates (`bevy-0.20.0/Cargo.toml:2969-2972`). Whether `bevy_scene` has a type of the same old name is `NOT TRACED`.

## Probe 2: the HUD change counter on today's tree

Test source: `hud_change_counter.rs` (a scratch copy; the lane ran it as `crates/turts_client/tests/hud_change_counter.rs`,
untracked, removed before the commit). Patches: `patches/`.

Instrument. A headless client app in the pattern of `tests/widgets_lifecycle.rs:185-318`: `MinimalPlugins`, `AssetPlugin`,
`InputPlugin`, `WindowPlugin`, `TransformPlugin`, `ImagePlugin`, `TextureAtlasPlugin`, `MeshPlugin`, `TextPlugin`,
`CameraPlugin`, picking, `UiPlugin`, `ButtonPlugin`, `ScrollAreaPlugin`, `CoreAdapterPlugin`, `WidgetsPlugin`. No render
plugin, no GPU. The package is `mods/forge` (the HUD of `interface/hud.ron`, 108 widget entities with an
`AuthoredWidgetId`, 89 with `Text`). The replica is static (one own unit of each content id, the lab selected; no delta
arrives). A system in `First` reads `Option<Ref<Text>>`, `Option<Ref<Node>>`, `Option<Ref<Visibility>>` and counts
`is_changed()`; it runs once for each frame, so its count at the start of update k + 1 is the set of writes of frame k.
(Reading `Ref::is_changed` from `World` after `app.update()` is always false, because `clear_trackers` moves the
change tick to the end of the frame. The counter must be a system.) 61 updates give frames 1 to 60.

### The counts for each frame (raw: `raw/p2_*.log`)

All counts are entities with a changed component in that frame. "w" means the entity has an `AuthoredWidgetId`;
the tooltip views carry that id with the suffix `::tooltip`, so "other" entities with a `Node` and no id were 0 in all
runs.

| Build | Frames | Text | Node | Visibility | Test |
| --- | --- | --- | --- | --- | --- |
| Today (`51f1223`) | 1 | 89 | 108 | 108 | - |
| Today | 2 | 89 | 66 | 108 | - |
| Today | 3 to 60 (each frame) | **89** | **66** | **108** | RED, 58 of 58 frames, exit 101 |
| Step A: `values.rs` 3 sites guarded | 3 to 60 | 6 | 66 | 6 | red |
| Step B: A + `tooltip.rs` 2 sites | 3 to 60 | 0 | 66 | 0 | red |
| Step C: B + `tree.rs` Node and `UiTransform` sites | 2 | 0 | 61 | 0 | - |
| Step C | 3 to 60 | 0 | 0 | 0 | GREEN, exit 0 |

Sum over frames 3 to 60 today: Text 5,162; Node 3,828; Visibility 6,264. After step C: 0 and 0 and 0.
The 6 Text and 6 Visibility left after step A are the 6 tooltip views (`raw/p2_who-stepa.log`, `FRAME-5` lines).
The 66 Node left after step B are the list rows and the Fit containers (`apply_node_placement`, the Fit write).

### The time (dev profile, `app.update()`, 600 frames after 10 warm-up frames)

Command: `cargo test -p turts_client --test hud_change_counter --locked -- --ignored --nocapture --test-threads=1`
through the lane runner, profile `test` (opt-level 0, no debug info). Same machine, under a load of 12 to 34.

| Build | Run | 600 updates | Per update |
| --- | --- | --- | --- |
| Today | 1 | 25,895.9 ms | 43.2 ms |
| Today | 2 | 26,458.2 ms | 44.1 ms |
| Step C (all sites guarded) | 1 | 9,111.7 ms | 15.2 ms |
| Step C | 2 | 9,656.3 ms | 16.1 ms |

Three earlier runs of today's tree (their labels came from a shell loop; kept in the evidence folder but not used
above) gave 44.0, 40.6 and 37.0 ms. The runs interleave, so the difference is not a drift of the load.

Release profile (`cargo test --release`, same test, same order of runs; the first release build took 225 s):

| Build | Run | 600 updates | Per update |
| --- | --- | --- | --- |
| Today | 1 | 1,670.3 ms | 2.78 ms |
| Today | 2 | 1,784.3 ms | 2.97 ms |
| Step C | 1 | 1,077.0 ms | 1.79 ms |
| Step C | 2 | 1,029.6 ms | 1.72 ms |

So the guarded writes save about 1.1 ms (about 40 %) for each `update()` on native release. That is 6 to 7 % of a
16.7 ms frame, for one HUD with 108 widget entities and no render work. The remaining 1.7 ms is the cost that the
rule's first bullet names (the exclusive `sync_widget_tree` clones the replica, the content and the interface in each
frame, `tree.rs:136-147`) and the other per-frame work; step C does not touch it. The browser (wasm32) cost is
`NOT TRACED`: the run did not use a browser, as the task required. The test app has no render; the real client also
pays the render and the text layout on the GPU side.
The runs were not on a quiet machine (load average 12 to 34), so read the ratio (about 1.6 times native release,
about 2.8 times dev), not the milliseconds.

### Write sites that `UI-HUD` must change (today's lines, `51f1223`)

| File:line | Write | Why it counts |
| --- | --- | --- |
| `crates/turts_client/src/widgets/values.rs:39` | `*node = bar_fill_node(..)` | `DerefMut` marks `Node` changed (`bevy_ecs-0.20.0/src/change_detection/traits.rs:496-497`) |
| `values.rs:65` | `current.0 = text` | marks `Text` changed; a changed `Text` is measured again (`bevy_ui-0.20.0/src/widget/text.rs:325`) |
| `values.rs:124` | `*current = widget_visibility(..)` | marks `Visibility` changed |
| `crates/turts_client/src/widgets/tooltip.rs:97` | `current.0 = text` | one for each tooltip view (6 here) |
| `tooltip.rs:100` | `*visibility = ..` | same |
| `crates/turts_client/src/widgets/tree.rs:73`, `:76` | `node.width/height = ..` in `update_fit_layout` | the helper `update_fit_layout` runs each frame; not in the gate list |
| `tree.rs:96` -> `:869-886` | `apply_node_placement(&mut current, ..)` takes `&mut Node` | every call marks `Node` changed; callers `tree.rs:96` (Fit children) and `:840` (list rows) |
| `tree.rs:104`, `:848` | `*current = transform` (`UiTransform`) | not one of the three types, but the same pattern |

The gate listed `values.rs:39/65/124` and `tooltip.rs:97`. The run shows that those four are not enough: the
`Node` writes of `tree.rs` (`update_fit_layout` and the list-row reconcile) keep 66 `Node` changes in each frame.
`UI-HUD` must name `tree.rs` as well as `values.rs` and `tooltip.rs` (gate H2 said "`tree.rs` only", then "add
`values.rs` and `tooltip.rs`"; the run needs all three).
Other client systems take `&mut Text`, `&mut Node` or `&mut Visibility` (`client_screens.rs:256-257`,
`client_shell/settings.rs:558`, `client_shell/bindings_editor.rs:1042`, `:1065`, `client_shell/notices.rs:141`, `:166`,
`volume_settings.rs:227`, `visuals/world_tooltip.rs:21`). They were not in this test app, so whether they write in
every frame is `NOT TRACED`.

### Can the counter be a normal headless test?

Yes. It is a `cargo test` integration test of `turts_client`; it needs no GPU, no window and no browser. It built and
ran in 1.4 s after the crate was built. The count is 58 frames with changes today (exit 101), 0 after step C (exit 0).
Partial fixes (A, B) stay red, so each site is covered by the test. The ignored timing test is not part of the gate.
The test does not cover: a live replica (deltas), the `Presented` update, the feeds (`GroupsFeed`, `AlertsFeed`),
and the client screens: `NOT TRACED`.

## What the design or the plan needs (task or design line, what it says, what is true)

| Line | It says | What is true |
| --- | --- | --- |
| plan `UI-HUD` (P:735, P:737, P:907) | files: `tree.rs` only (gate: add `values.rs`, `tooltip.rs`) | Add `tree.rs:73, :76, :96, :104, :840, :848` (the Node and `UiTransform` writes) and `tooltip.rs:97, :100` and `values.rs:39, :65, :124`. Red: `hud_change_counter.rs` (58 of 58 frames today). |
| plan `UI-HUD` red/plant | "put `current.0 = text` back" | The test goes red for each of the three types separately (steps A and B), so a plant on any one site shows. |
| design D:2267-2276 (cost rule; the `set_if_neq` line is D:2274) | "A system writes a component only when its value changes (`set_if_neq`)" | Right, and `Text` is a `String` field: use a compare before the write (`if current.0 != text`) or `set_if_neq` with `Text(..)`; `Node` and `Visibility` derive `PartialEq`. A helper that takes `&mut Node` (`apply_node_placement`) must take `&mut Mut<Node>` and end with `set_if_neq`. |
| plan `PRES-MODELS` (P:1247) | root `WorldAssetRoot`; memory source in `lib.rs:298` | Use the flat template (D:2118); keep the `turret` and `muzzle` node entities; register an empty shared `Dir` before `DefaultPlugins` and insert the bytes at acquire time; keep `Mesh3d` on the unit root; the team material is the `SlotMaterials` handle. |
| plan `PRES-MODELS` tests | - | Add a headless test with the plugin set 1 or 2 above (copy from `glb_load/src/main.rs`): `app.finish()`, `GltfPlugin` before `PbrPlugin`, bytes in the `Dir`; assert `named_nodes` has `turret` and `muzzle`, 72 triangles for the tank, and that two units share one `team` handle. |
| plan PRES-PROBE (P:616-628) | "Print ... `Material0/std`" | Done; the label works as `pkg://<path>#Material0/std` in the pbr set only. |
| gate M1 | `GltfPrimitive.material` is `Handle<GltfMaterial>` | It is `Option<Handle<GltfMaterial>>`. |

## Commands and exit codes

Probe 1 (scratch folder; its own `target`; `--offline`; no lane runner):

| Command | Exit | Note |
| --- | --- | --- |
| `python3 -I make_tank.py tank.glb`; `python3 -I make_tank_bad.py tank_bad.glb` | 0, 0 | 6,628 bytes each |
| `cargo build --locked --offline` | 101 | "cannot update the lock file" (pruned lock); then without `--locked` |
| `cargo build --offline` (no pbr) | 101, then 0 | first a compile error in the probe (`inspect_entity` tuple), fixed |
| `glb_load tank.glb` (no pbr), try 1 | 3 | load stayed `Loading`: no `app.finish()` |
| `glb_load tank.glb`, try 2 and 3 | 101, 101 | spawn panic: `Transform` not registered; cured by `reflect_auto_register` |
| `glb_load tank.glb`, `LATE=1 glb_load tank.glb`, `glb_load tank_bad.glb` (no pbr) | 0, 0, 0 | `raw/p1_run_nopbr*.txt` |
| `cargo build --offline --features pbr` | 0 | |
| `glb_load tank.glb` (pbr), try 1 and 2 | 101, 101 | `GltfExtensionHandlers` missing (plugin order); then `WindowResized` message missing (add `WindowPlugin`) |
| `glb_load tank.glb`, `LATE=1 glb_load tank.glb` (pbr) | 0, 0 | `raw/p1_run_pbr*.txt` |
| `cargo check --offline --target wasm32-unknown-unknown` and `... --features pbr` | 0, 0 | `raw/p1_wasm_check_*.txt` |
| `glb_probe` (gate source, `gltf =1.4.1`) on `tank.glb` and `tank_bad.glb` | 0, 0 | `pivot_ok=true`, `pivot_ok=false` |

Probe 2 (lane `/home/julien/Projects/TuRTS-mp1-pres-probe`, head `51f1223`, cache `target/mc-mp1-pres-probe`, runner
`exec_in_lane_v4.py`, labels `mp1-presprobe-*` in `target/mc-evidence/`; `cargo ... --locked`):

| Label | Command | Exit |
| --- | --- | --- |
| `build-deps` | `cargo test -p turts_client --test widgets_lifecycle --no-run` | 0 |
| `counter-today` | `cargo test -p turts_client --test hud_change_counter -- --nocapture` | 101 (red: 58 of 58 frames) |
| `counter-stepa`, `who-stepa`, `counter-stepb` | same, with the patch steps A, A, B | 101, 101, 101 |
| `counter-stepc` | same, step C | 0 (green) |
| `time-today-dev-1`, `-2`; `time-fixed-dev-1`, `-2` | same with `-- --ignored` | 0 each |
| `release-build-fixed` | `cargo test --release ... --no-run` | cargo 0; the wrapper gave 2, because I made the `docs/` folder in the lane while it ran (source receipt changed) |
| `time-today-rel-1`, `-2`; `time-fixed-rel-1`, `-2` | `cargo test --release ... -- --ignored` | 0 each |

Deviation from the task: three early timing runs of today's tree used labels built in a shell loop
(`mp1-presprobe-time-today-dev-r1`, `-r2`, `-r3`). They are not used in the tables. The scratch changes and the scratch
test file were removed from the lane before the commit; the tree is `51f1223` plus this folder.
The patches in `patches/` were written again with `rtk proxy git diff` from the same edit script (the first copies were
cut short by the output filter) and checked with `git apply --check` on the clean lane; they were not run again.
Step A is the `values.rs` hunks of `stepAB_values_tooltip.patch`; step B is that whole file.

## Not checked

- A browser run, wasm32 runtime (`reflect_auto_register` on wasm, `bevy_tasks::block_on` in `bevy_pbr-0.20.0/src/gltf.rs:15`),
  and any GPU run. The task forbade them.
- A model with embedded textures (`Image` asset, PNG decode); `tank.glb` has none. The no-texture load did not need
  `ImagePlugin` or an `Image` asset (a run without `init_asset::<Image>` still loaded).
- A path clash between two packages in one `Dir`; removal of bytes (`Dir::remove_asset`, `memory.rs:91`) and reload.
- The live client with deltas: the counter ran on a static replica in the widgets plugin only.
