# Presentation roadmap: what a modern game expects

Date: 2026-11-13. Status: direction, confirmed by the CEO on 2026-11-13. Stage P1 and P2 have their own
specs or will get one. This document sets the contract rules and the order.

## 1. Direction (CEO, 2026-11-13)

"We will need all that is expected of modern games: Shaders effects, particles, ..."
The goal is a production-level "Ultimate RTS" (spec 3.1, G1). A mod must be able to ship
detailed models, textures and effects, and the engine must show them well.

## 2. Where we are, traced

- Units and buildings: one capsule or cuboid each (`crates/turts_client/src/render.rs:236-249`).
- Effects: the mod API has five effect kinds, `Flash`, `Spark`, `Tracer`, `Burst`, `Smoke`
  (`crates/turts_mod_api/src/schema.rs:832-838`). No client code draws them. The skirmish
  package declares none.
- Models: fields exist in the API, nothing reads them (see the models spec).
- Lighting: one directional light and ambient, from the package
  (`crates/turts_client/src/render.rs:339-342`).
- The spec keeps these presentation decisions Open: importer features, animation formats, the
  crowd path, the camera and lighting API (`RTS-Engine-Project-Specifications.md`, section 15).

## 3. The contract rules for all visual content

These rules are the Foundation part. They decide whether later stages need rework.

1. **A mod describes; the engine draws.** A mod names assets and sets parameters in data files.
   It never sees the renderer, a Bevy type, or a frame callback (D15, spec 5.2). The engine can
   then change its renderer and all mods continue to work.
2. **Quality is content, scaling is the engine.** A package ships its art at full quality. The
   client `quality` setting (low, medium, high, and later ultra) selects the LOD, texture size,
   particle count, shadow and post-processing level for each machine. A mod cannot force it.
3. **Presentation never changes the result of a match.** Effects, particles and animation are
   client side. The server does not read them. A player with effects off gets the same match.
4. **Data first, code last.** Three levels of power, opened in this order:
   - Level A: engine effect and material types with parameters (colour, size, texture, rate).
   - Level B: composition in data: a particle system built from modules, a material built from
     layers. No code.
   - Level C: custom shader code from a mod (WGSL), inside fixed entry points that the engine
     defines, checked at pack time, with a cost limit. A mod author gets full creative power; the
     engine keeps the frame structure.
   Level A and B cover most games. Level C is what makes it "ultimate". Level C is designed in
   stage P5 after A and B exist, because its entry points depend on what A and B need.
5. **Everything has a budget and a fallback.** Each effect type states its cost class. Each
   feature has a defined result when it is off (for example: no shadow, a simpler material). A
   package that works only at `ultra` is refused at pack time.
6. **WebGPU is the reference.** The browser client on WebGPU gets all features. A feature that
   a fallback path cannot do is switched off there, never faked.
7. **Additive versions.** Each new field is optional, has a default, and gets a version-table
   row. The packages of today always load.

## 4. The capability list

"All that is expected" as a list, so that nothing is forgotten and nothing is built by habit.

| Area | Capabilities | Stage |
| --- | --- | --- |
| Models | glTF, PBR materials, LOD, team colour, instancing | P1 |
| Lighting | directional sun with shadows, ambient, image-based ambient (sky), point and spot lights on units and effects, day-time presets for a map | P1 basic, P4 |
| Animation | skinned clips by unit state, turret and muzzle nodes, building construction and damage states, crowd animation path | P2 |
| Effects | particles (sparks, smoke, fire, dust, debris, explosions), tracers and beams, trails, decals (scorch marks, tracks, craters as pictures only), screen shake, light flashes | P3 |
| Combat readability | muzzle flash, hit, death, damaged-building smoke and fire, attack warning | P3 (first items) |
| Materials and shaders | engine material types (standard, team colour, emissive pulse, scrolling, dissolve for build and death, shield, cloak), then layered materials, then custom WGSL | P3 level A, P5 levels B and C |
| Ground and terrain | textured ground with blended layers, normal maps, cliffs and ramps from the heightfield, grass and small props, roads and creep-like overlays | P4 |
| Water | a water surface with reflection approximation, shore foam, depth colour | P4 |
| Atmosphere | sky, distance fog, weather particles (rain, snow, dust) as presentation | P4 |
| Fog of war | explored and visible states drawn on the ground and the minimap | with the fog-of-war rules (no decision register row yet) |
| Post-processing | tone mapping, bloom, anti-aliasing, colour grading by map, ambient occlusion, outline for selection and hover | P4 |
| Interface art | themed HUD skins, animated widgets, portraits, 3D unit previews, cursors | with the HUD work (D67) |
| Sound | positional effect sounds, music layers, ambience, unit voices | with the audio work (D69) |
| Cinematic | camera paths, letterbox, in-engine cutscenes from triggers | late (with D71 tooling) |
| Performance tools | frame budget overlay, per-package cost report in `turts-pack`, quality presets | each stage adds its part |

Terrain rule that stays: the visual mesh is never the authoritative terrain, and the terrain is
a heightfield (D09 to D13). Craters and tracks in P3 are pictures on the ground, not terrain
edits.

## 5. Stages

Each stage ends with something the CEO can see in the skirmish. No stage is only groundwork.

| Stage | Content | What the player sees | Spec |
| --- | --- | --- | --- |
| P1 | Models, textures, LOD, team colour, facing, shadows, ground texture, quality setting, first measurement | Real soldiers, workers and buildings on textured ground | `2026-11-13-models-and-textures.md` |
| P2 | Skinned animation by unit state, turret aim, construction rise, death | Units walk, aim, work and fall | to write after P1 measurement |
| P3 | Particle and effect system (levels A and B), decals, engine material types, combat feedback | Muzzle flashes, tracers, explosions, smoke from damaged buildings, scorch marks | to write; can run in parallel with P2 |
| P4 | Environment: terrain materials, water, sky, fog, post-processing, more lights | The map looks like a place, not a plane | to write |
| P5 | Layered materials and custom mod shaders (level C), with the pack-time checks and budgets | Mods ship their own look: shields, cloaks, energy weapons | to write; a Foundation spec with a gate |

Order reasons:

- P1 before all: every other stage draws on or around models.
- P3 early: combat feedback was already the next gameplay item, and effects give the largest
  visible gain for each day of work after models.
- P5 last: custom shader entry points are costly to change after mods use them. They must be
  designed from real needs that P1 to P4 show.

## 6. Measurement rule

Each stage records its numbers in `docs/measurements/` on the real GPU in the browser: frame
time p50 and p95 at 400 and 2,000 units with the stage's features on, and the memory high-water.
The numbers are recorded, not judged, until the CEO sets budgets (spec 15). If a stage result
is below 30 frames each second at 400 units on the play-test machine at `high`, the stage is not
complete: the fallback or the scaling is missing.

## 7. What this roadmap does not do

- It does not delay gameplay. Rally and exit, combat feedback, army selection, a second human
  player, and more units continue beside it.
- It does not select final art. The skirmish stays low-poly now (CEO).
- It does not add process: one spec for each stage, built straight through, one review that
  runs code. Only P5 and the contract rules of section 3 get a gate.

## 8. Confirmed by the CEO (2026-11-13)

1. The contract rules of section 3, with rule 4: mods get data-driven effects and materials
   first, and custom shader code later, inside engine-defined entry points.
2. The order P1, then P2 and P3 together, then P4, then P5.
