# 3D models and textures

Date: 2026-11-13. Status: stage 1 ready to build after the CEO answers section 9.
Track: Foundation for the package contract (sections 4 and 5); light path for the client work.
Process: this spec, the build, one independent review that runs code. No plan document.

## 1. Direction (CEO, 2026-11-13)

- After play: "what I missed most was having actual 3D models & textures. It's hard for a human
  to play with just geometric shapes in the same color & texture."
- "Right now I'm cool with basic low-poly models. But the long term goal is production-level
  'Ultimate RTS', so we're going to let mods use detailed models & textures and we need to
  support that and design for that."

Thus two results are necessary:

1. Now: the skirmish shows low-poly models with textures, in a few days.
2. Always: the package contract and the renderer path accept detailed models, PBR textures and
   animation later with no rework of what stage 1 builds. A mod author decides the quality. The
   engine scales it to the machine of each player.

## 2. Facts, traced

- The mod API 0.2.4 has the fields already: `AssetsDef.models`
  (`crates/turts_mod_api/src/assets.rs:11-12`) and `VisualDef.model` with `asset`,
  `team_material`, `turret_node`, `muzzle_node` (`crates/turts_mod_api/src/schema.rs:763-785`).
- No client code reads `model`. The client draws only `Shape::Capsule` and `Shape::Cuboid`
  (`crates/turts_client/src/render.rs:236-249`).
- The pack-time model checks are not built (`crates/turts_mod_runtime/src/mp1_validation.rs:19`
  names them as later work).
- The client crate does not enable a glTF feature
  (`crates/turts_client/Cargo.toml:15`: `3d`, `ui`, `default_font`, `bevy_audio`, `vorbis`).
- The MP1 design section 8.1
  (`docs/superpowers/specs/2026-11-12-mp1-playable-skirmish-design.md:2104-2147`) defines an
  "MP1 validation profile": 8,000 triangles, 2 materials, PNG only, no skins, no animations.
  Its own text says that this is not the final engine format. This spec replaces that profile.
- The skirmish package uses shapes only (`mods/skirmish/visuals/*.ron`).

NOT TRACED: the Bevy 0.20 behaviour named in section 6 (automatic instancing, visibility
ranges, KTX2 on WebGPU, skinning cost). Section 7 measures it before anybody relies on it.

## 3. Design principle: quality is content, scaling is the engine

A production RTS shows hundreds of detailed units because the engine does four things, not
because the models are simple:

1. **Instancing.** All units of one type share one mesh and one material. The GPU draws them in
   one call or a few calls.
2. **Level of detail (LOD).** A unit that covers few pixels on the screen draws a simpler mesh.
   At the usual RTS camera height most units are small on screen.
3. **Compressed textures.** Textures stay compressed in GPU memory (about 4 to 8 times smaller
   than raw RGBA).
4. **Cheap animation for crowds.** Near units animate fully. Far units use a cheaper method.

So the rule for this engine:

- The package states the art at full quality: LOD levels, PBR materials, animation clips.
- The engine selects what to draw for each machine: the LOD, the texture size, the shadow and
  animation quality. A graphics setting of the client controls it. A mod does not.
- The pack-time limits protect loading and memory. They are not a statement about art quality.
  They are measured numbers that can increase by a version-table row, with no format change.

## 4. Package contract

All additions are optional fields with defaults, so the packages of today load unchanged. Each
new field gets a row in `crates/turts_mod_runtime/src/version_table.rs`. Increase
`ENGINE_API_VERSION` as the version rules require.

### 4.1 Model files

- Format: glTF 2.0 binary (`.glb`), all data embedded, no external `uri`.
- Geometry: positions, normals, tangents (optional), UV set 0, vertex colours (optional).
- Materials: the glTF metallic-roughness PBR model. Accepted maps: base colour, normal,
  metallic-roughness, occlusion, emissive. Alpha modes `OPAQUE` and `MASK`. `BLEND` is refused
  in stage 1 with a clear error (sorting cost; open it later).
- Number of materials for one model: at most 8.
- Embedded images, stage 1: PNG and JPEG. Stage 3 adds KTX2 (Basis Universal) with no change to
  the fields.
- `extensionsRequired`: refused, except the entries that a later stage builds. The error names
  the extension.
- Skins and animations: accepted by the validator from stage 1 (within the limits of 4.4) and
  ignored by the stage 1 client, which draws the bind pose. Stage 2 plays them. Reason: a mod
  author must not have to re-export all models when stage 2 arrives.
- Cameras and lights in the file: ignored, with a pack-time warning.

### 4.2 `ModelDef` additions

```
model: Some((
    asset: "skirmish:soldier_glb",           // LOD 0, exists today
    team_material: Some("team"),             // exists today
    turret_node: None, muzzle_node: None,    // exist today
    lods: [                                  // new, default []
        (asset: "skirmish:soldier_lod1_glb", below_screen_height: 0.08),
        (asset: "skirmish:soldier_lod2_glb", below_screen_height: 0.03),
    ],
    scale: 1.0,                              // new, default 1.0
    animations: None,                        // new, stage 2 (section 6.2)
))
```

- `lods`: each entry is a simpler model and the screen height (fraction of the viewport height
  that the unit bounds cover) below which it shows. The values must decrease. An empty list is
  valid: the engine then draws LOD 0 at all distances.
- `scale`: uniform. The fit check of 4.4 uses the scaled bounds.
- `shape` stays required. It is the fallback while a model loads, the low-end fallback, and the
  pick volume of today.

### 4.3 Team colour

- A model names one material as `team_material`. The client draws that material with the slot
  colour of the owner: base colour texture multiplied by the slot colour, other maps unchanged.
- A model with no `team_material` gets no team colour on the mesh. The selection ring and the
  health bar still show the slot colour.
- Advice to authors (not checked): the team material covers at least 15 % of the top view.

### 4.4 Pack-time checks

Each refusal is a `ModError` with the package, the file and the field, in the style of the
asset checks of today (`crates/turts_mod_runtime/src/content_mc_validation.rs`). Use the `gltf`
crate at the version that `bevy_gltf` 0.20 uses, so the validator parses what the client parses.

| Check | Stage 1 limit (provisional until section 7 measures) |
| --- | --- |
| GLB version 2, all data embedded | fixed |
| Triangles of one model file | 100,000 |
| Bytes of one model file | 32 MiB |
| Bytes of all models of a package | 512 MiB |
| Image size | 4096 px on each side |
| Materials of one model | 8 |
| Joints of one skin | 128 |
| The `team_material`, `turret_node`, `muzzle_node` names exist in the file | fixed; the error lists the names that exist, with a `did you mean` hint |
| LOD list: values decrease; each LOD has no more triangles than the level before it | fixed |
| Fit: the scaled XZ radius of the model bounds is 0.5 to 1.5 times the footprint radius | fixed; a warning, not a refusal, between 1.2 and 1.5 |
| Pivot: the lowest point of the scaled bounds is within 0.05 of ground level | fixed |

Warnings (the pack succeeds, the text prints): a model above 20,000 triangles with no `lods`;
an image that is not a power of two; a model with cameras or lights.

The limits are high on purpose. The cost control is the LOD and the client budget of section 6,
not a small cap. Record the limits in one table in the code, so that one edit changes them.

## 5. Stage 1: models in the skirmish (build now)

### 5.1 Client

1. Enable glTF loading in the client. The browser loads each model from the package bytes that
   are already in memory. It makes no network request for a model file.
2. For each visual with a `model`: show the `shape` until the model is ready, then show the
   model. No frame has neither.
3. Draw all units of one type with shared mesh and material handles, so that the renderer can
   instance them. One shared material for each slot colour of the team material.
4. LOD: when `lods` is not empty, select the level by the screen height. Use the engine's
   visibility-range mechanism if it serves; else a simple system. A small hysteresis prevents
   flicker at a boundary.
5. Facing: a unit turns to face its movement direction and, when it attacks, its target. The
   turn has a rate limit (a full turn in about 0.4 s), so it does not snap. A building keeps its
   authored facing. Client only; no wire change.
6. Selection, picking, health bars and rings stay as today and stay correct for a model. The
   pick volume follows the visible model body as it does for shapes today (commit 4560700d).
7. Lighting: one directional light with shadows for units and buildings, and ambient light, so
   that the forms read. The package light settings of today continue to apply.
8. Ground: the map plane takes an optional tiling ground texture from the package (base colour;
   normal map optional). If the map file has no field for it, add one optional field.
9. A client graphics setting `quality` with three values (`low`, `medium`, `high`) exists from
   stage 1 and controls: shadows on or off, the LOD bias, the largest texture size used. The
   default is `high` on WebGPU. It is a client setting; a package cannot set it.

### 5.2 Skirmish content

The theme is medieval fantasy, low-poly (CEO, 2026-11-13). The six assets are ready, all CC0:
`target/mc-evidence/art/skirmish-fantasy/` (see its `README.md` for sizes, materials and the
team material of each file, and `CREDITS.md` for the sources). That folder is not in git; copy
the files into the package.

| Target | File | What it is | Team colour |
| --- | --- | --- | --- |
| `skirmish:worker` | `worker.glb` | A villager in a tunic, no weapon | the tunic |
| `skirmish:soldier` | `soldier.glb` | An armoured knight who aims a crossbow forward | cape and trim |
| `skirmish:base` | `base.glb` | A stone castle with a dome and flags | dome and flags |
| `skirmish:barracks` | `barracks.glb` | A timber hall with crossed swords above the door; the door side is forward | banners |
| `skirmish:trees` | `trees.glb` | A stand of pine trees | none |
| ground | `ground_grass.png` | A tiling grass and dirt texture, 1024 px | none |

Name changes that fit the theme (display names only; the content ids stay): "Command centre"
becomes "Castle", "Soldier" becomes "Crossbowman", "Worker" becomes "Villager". "Barracks" and
"Timber stand" stay. `soldier_anim.glb` and `worker_anim.glb` are for stage P2.

Rules for the content:

- At the default camera a player tells the worker from the soldier in one look, without the
  selection panel. Silhouette and proportion carry the difference, not colour alone.
- Body colours are neutral. They are not close to a slot colour.
- Forward is glTF +Z, up is +Y, the pivot is at ground level in the footprint centre.
- Each file records its source and licence in `mods/skirmish/assets/CREDITS.md`. The repository
  is public: only files whose licence permits public redistribution are accepted.
- The packages of the older milestones keep their shapes. Their pixel checks do not change.

## 6. Later stages (design now, build later)

Stage 1 must not block these. Codex states in its report if a stage 1 choice would.

### 6.1 Stage 2: animation

- Skinned glTF animation, with clips named in the model file.
- `ModelDef.animations` maps unit states to clip names: `idle`, `move`, `attack`, `death`,
  `work` (gather and build). A missing entry uses `idle`. The engine selects the clip from the
  unit state that the client already has. A mod runs no script for each unit.
- The attack clip aligns with the fire event. The death clip plays, then the body stays a short
  time and sinks or fades.
- Crowd cost: beyond a unit count or below a screen height, units animate at a lower update
  rate or share poses. The method is selected after the stage 3 measurement.

### 6.2 Stage 3: scale

- KTX2 compressed textures; `turts-pack` can transcode PNG to KTX2 so that an author needs no
  extra tool.
- Mipmaps for all model textures.
- A texture memory budget for each `quality` value; the client drops the top mip levels to fit.
- Automatic LOD generation in `turts-pack` for a model that gives none (mesh simplification).
- Impostors or a simpler far level for very large armies, if the measurement asks for it.

## 7. Measurement (part of stage 1, before the content work finishes)

One browser run on the real GPU of the play-test machine, WebGPU, sound off, recorded in
`docs/measurements/models.md` with the machine, the browser and the adapter:

| Scene | Record |
| --- | --- |
| 400 units, shapes (the state of today) | frame time p50 and p95 |
| 400 units, the stage 1 low-poly soldier | frame time p50 and p95, draw calls if available |
| 400 units, a detailed test model: 20,000 triangles, 2048 px base colour + normal + metallic-roughness, no LOD | the same, and the wasm memory high-water and the GPU memory if available |
| The same detailed model with two LOD levels | the same |
| 2,000 units of the detailed model with LOD | the same |

The detailed test model is a test asset; it is not shipped in the skirmish. These numbers are
recorded, not judged. They set the limits of 4.4 and the order of stage 3. If the detailed
scene with LOD is below 30 frames each second at 400 units, report it to the orchestrator
before more content work: that changes the plan.

## 8. Acceptance for stage 1

Each row must be able to fail on the build before the change.

| Id | Check | Level |
| --- | --- | --- |
| MD1 | The skirmish loads in the browser with the six assets. The network log has no request for a `.glb` path. | browser |
| MD2 | With a test hook that delays model readiness by 500 ms, the shape shows at the unit's screen point in each frame of the delay, then the model shows. | browser, pixels |
| MD3 | The team material shows the slot colour: the soldier of slot 1 and the soldier of slot 2 differ in the team pixels and agree in the body pixels. | browser, pixels |
| MD4 | Each check of 4.4 refuses a planted bad package with its error text. A valid model with a skin and an animation packs. | pack test |
| MD5 | A unit that moves east faces east within 0.5 s; a unit that attacks faces its target. | client test |
| MD6 | Picking: a click on the visible body of a model selects it; a click beside it does not. | browser |
| MD7 | LOD: a test model with two levels shows level 1 when zoomed out below its threshold and level 0 when zoomed in. | client test or browser |
| MD8 | The scripted player probes of `docs/qa/cold-play.md` pass on the build with models. | browser |
| MD9 | The measurement table of section 7 is recorded. | document |
| MD10 | The packages of the older milestones still pack and load, unchanged. | existing tests |

Every test browser runs with `--alsa-output-device=null` and a fresh profile.

## 9. CEO decisions (2026-11-13)

1. The six stage 1 models come from free sources whose licence permits a public repository.
   The orchestrator finds them and delivers the files. If no good free model exists for a
   target, the orchestrator tells the CEO and they select another option.
2. Stage 2 (animation) starts directly after stage 1, together with stage P3 (effects).

The engine work of sections 4, 5.1 and 7 does not depend on the model files and starts with a
placeholder model.

## 10. Review

One reviewer, after the build. The reviewer runs MD1 to MD8, reads the measurement, and plants
three faults: (a) the team colour not applied, (b) the shape removed before the model is ready,
(c) a model fetched by URL. Each must turn a row red.
