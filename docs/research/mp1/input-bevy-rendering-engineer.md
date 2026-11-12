# MP1 input: bevy-rendering-engineer (model path, effects, first real assets)

Source: subagent hand-back, MP1 brainstorm input, 2026-11-11. Agent output, not reviewed; claims are leads until verified.

VERDICT=INPUT

This is an input for the MP1 design. It is not a review. Scratch copies come from HEAD 8d6902c. HEAD moved to a29ce67 during the run, and that commit changed docs only. I wrote no repository file. The run log is in `/tmp/claude-1000/-home-julien-Projects-TuRTS/bf418660-b272-4bb3-8628-8d9ef2677d1d/scratchpad/mp1-render/runs.txt`.

**Main finding.** The web build already compiles the glTF loader, PBR, KTX2 and zstd. Bevy's `3d` feature includes `bevy_gltf`, `bevy_pbr`, `gltf_animation` and (through `3d_api`) `ktx2` and `zstd_rust` (bevy-0.20.0/Cargo.toml:2725-2753). `turts_client` enables `3d` (crates/turts_client/Cargo.toml:14). `cargo tree -i bevy_gltf` confirms this for wasm32. We already pay for the loader in download size.

## 1. Model path

**Schema**
- Add `models: Vec<FileAsset>` to `AssetsDef` (turts_mod_api/src/assets.rs:5-10), with `#[serde(default)]` so that old packages still parse.
- Add an optional `model` to `VisualDef` (schema.rs:501-504): `{ asset, team_material, turret_node, muzzle_node }`. Keep `shape` as a required field. The client uses it as the fallback while the model loads, and it is also the content cut line.
- Version-table rows `assets.models` and `visual.model`, since_api 0.2.4 (version_table.rs:21-57). The current API is 0.2.3 (turts_mod_api/src/lib.rs:52-55).

**Checks** (turts_mod_runtime, beside png_header in asset_headers.rs:11)
- Use `gltf = "=1.4.1"`, the version bevy_gltf uses (bevy_gltf Cargo.toml:122). The validator then parses what the client parses (spec 5.8:249).
- The file must be GLB v2. There must be no `uri` anywhere, no `extensionsRequired`, no skins, animations, cameras or lights.
- Triangles are counted from the index accessor counts. Bounds come from the POSITION min/max values.
- The named nodes and the team material must exist.
- Embedded PNGs go through the existing caps (content_mc_validation.rs:16-17).
- New caps (needs decision): 4 MiB per GLB and 16 MiB in total.

**Bytes in the browser** (no URL fetch). Bevy 0.20 has `MemoryAssetReader` and `Dir::insert_asset` (bevy_asset io/memory.rs:41-64, 290). That module is not gated for wasm32 (io/mod.rs:18). The source must be registered before `AssetPlugin` (lib.rs:576-579, 628-630). In our client that means before turts_client/src/lib.rs:298. Then load with `asset_server.load("pkg://<hash>/<path>")`.

**Native probe (passed).**
- The GLB bytes were inserted after startup and loaded after 1 update.
- `named_nodes=["body","turret"]` and `named_materials=["body","team"]`. The embedded PNG was decoded.
- The texture has `mips=1` and usage `MAIN_WORLD|RENDER_WORLD`.
- A truncated GLB gave exit 3. The probe can go red.
- The probe also passes `cargo check --target wasm32`. It did NOT RUN on wasm32.

**0.20 pitfalls**
- `GltfPrimitive.material` is `Handle<GltfMaterial>` (bevy_gltf assets.rs:174). The StandardMaterial is the label `<mat>/std` (bevy_pbr gltf.rs:120-158).
- Scenes are `WorldAsset`, spawned with `WorldAssetRoot` (bevy_world_serialization components.rs:23).

I recommend that the client does not spawn scenes. It builds one flat template for each model (mesh, material, transform, node name) and spawns its own entities.

**Wasm size** (my `wasm-opt -Oz` flags, not trunk's flags)

| Build | cargo | wasm-opt | brotli |
| --- | ---: | ---: | ---: |
| today | 66,583,563 | 28,440,941 | 6,092,887 |
| no gltf | 65,132,644 | 27,694,366 | 5,941,682 |
| Δ | 1,450,919 | 746,575 | 151,205 |

`basis-universal` on wasm32: NOT TRACED.

## 2. Identity under models

**Team colour.** I recommend a named material slot. At spawn, the client replaces that material with the slot material from `SlotMaterials` (unlit, render.rs:269-301). A mask shader goes to M13.

**What must not change**
- The MC packages keep `shape` visuals. The drivers find units by hue (click9-pixels.js:6-9, hud4-pixels.js:107-120) and by silhouette aspect (hud4-pixels.js:60-70).
- Rings, bars and markers stay unlit (visuals.rs:551-554, 1372-1376).
- The ground stays green (render.rs:347; hud4-pixels.js:100-101).
- The `turts_scene` bridge queries `With<Mesh3d>` on the unit entity (turts_client_web/src/lib.rs:392). With models, the root entity needs a mesh, or that filter must change.

**What stays separate from the mesh.** Picking uses the declared footprint, not the mesh (render.rs:50-61), so the visual mesh never becomes the gameplay shape (4.1:152).

## 3. Orientation without animation

**Heading.** The wire has no heading. `UnitDelta` holds only `pos` (messages.rs:45-49). `facing` is a 4-way value set at spawn (messages.rs:41; schema.rs:183-188). The client can derive yaw from the replica samples (replica.rs:26), with a turn rate. It is cheap and needs no wire change.

**Forward axis.** glTF +Z matches `PosY` at yaw 0 (render.rs:27, 445), because both conversion flags are off by default (convert_coordinates.rs:63-83).

**Turret aim.** `last_target` (messages.rs:325) goes to the owner only (export.rs:174-177). An enemy turret cannot aim until we add an event (part 4).

**Harvester load.** Inventories are owner-only (export.rs:165). The client can scale a `cargo` node, for own units only.

**Cheap with static models:** heading, turret yaw, recoil and a walk bob.
**Needs skinning:** walking infantry. That goes to M13 (13.4:635).

## 4. Effects

**Events on the wire today**
- Health is public (export.rs:180).
- `removed` has no cause (messages.rs:357).
- No fire event exists, and weapon records go to the owner only.
- Damage is instant (combat.rs:371-381).

**Proposal.** Add `WorldDelta.effects: [Fired{unit,target,weapon}, Destroyed{unit}]`, public in MP1, as protocol 5 (lib.rs:13). The client derives a hit from the health drop.

**Smallest set and technique** (WebGPU and WebGL2 both work, no compute)

| Effect | Technique |
| --- | --- |
| Muzzle flash, hit spark | Pooled unlit meshes that shrink over their life (no alpha sort) |
| Tracer | A line in the existing gizmo group (visuals.rs:61) |
| Death burst | Expanding sphere and up to 8 debris pieces |
| Building damage | Smoke at a health threshold |

- Cap the pool at 256 instances (13.4:641). No Hanabi.
- Mod content: `exports.effects` with `EffectDef{id, kind, colour, size, ms, sound}`. The visual def binds `fire`, `hit` and `death` by content id. The server never reads them (4.4:191).
- Effect colours must not match the slot hues of the drivers (hud4-pixels.js:110).

## 5. Cost and risk

**Frame time.** VIS2 had 17.4 draw calls per frame and p50 133.3 ms at 5,000 units (mc.md:11-16). Batching works, so the cost is most likely on the CPU (NOT TRACED).
- 400 primitives: about 10.7 ms if the cost is linear in units.
- 400 models with about 4 entities each: 10.7 to 43 ms. This is an estimate, NOT MEASURED.

**Draw calls.** About 60 to 80 per frame. Shadows are off (bevy_light directional_light.rs:152; camera.rs:124-128).

**Download.** M1 was 4,815,934 B (m1.md:142). Nine GLBs could add 3 to 8 MB.

**Memory.** Textures need 1 to 5.3 MiB each. The CPU copy stays in memory unless we drop it. Wasm memory never shrinks (m1.md:122-124).

**WebGL fallback.** Still open (spec:760). Static PBR and pooled effects work on WebGL2. GPU mipmap generation is manual (bevy_core_pipeline mip_generation/mod.rs:236-245), so the client generates mipmaps on the CPU.

**Three risks**
1. The asset production loop has no upper bound.
2. The MP1 pixel instrument must find lit, textured units and must be shown to go red.
3. Frame time at 400 units with models is unmeasured, and it needs the GPU that is busy now.

## 6. Asset brief

- One GLB 2.0 file. Embedded PNG, sRGB. No URIs, no extensions, no skins or animations.
- At most two materials: `body` (textured) and `team` (flat white). The team area must cover at least 15% of the top view.
- Body colours: neutral greys and browns. No saturated green, and none of the slot hues.
- Scale: 1 unit = 1 m. The XZ radius is 0.8 to 1.2 × the footprint radius.
- Pivot at the ground centre, with min y = 0.
- Forward is glTF +Z (Blender −Y), and +Y is up.
- Triangles: infantry 1,500, vehicles 3,000, buildings 8,000.
- Textures: 512² for units, 1024² for the command centre.
- Node names: `turret` (pivot on the yaw axis) and `muzzle`.

## 7. Recommendation

| Option | Content | Runs |
| --- | --- | --- |
| A: thin model slice | Shared work plus models | 12 to 14 |
| B: better primitives | Shared work plus compound shapes | 9; models later cost 5 to 7 more |

**Shared work in both options (7 runs):** heading, fire event, effects, team slot, pixel finder, 400-unit browser measurement.
- Files: render.rs, visuals.rs, new effects.rs, replica.rs, messages.rs, export.rs, combat.rs, schema.rs, version_table.rs, tests/browser/harness.

**Option A adds (5 to 7 runs):**
- Files: assets.rs, asset_headers.rs, content_mc_validation.rs, turts_client lib.rs:298, turts_client_web lib.rs:392.
- About 3 runs for asset production.

**Option B adds (2 runs):**
- A `Shape::Compound` with part roles, plus `mesh_for` (render.rs:237).

**My choice: A.** The loader is already in the download, the probe passed, and the `shape` fallback makes the cut line a content cut, with no engine rework. The first plan task must be the 400-unit browser probe.

Needs decision: the model caps, and whether fire events are public.
