# Shared skin palettes

The client shares a GPU bone palette between rigs that already share the same evaluated pose.
The current 2,000-unit High battle reaches 25.12 FPS. Low reaches 31.85 FPS.
The post-rig diagnostic profile spends about 10 percent of CPU time extracting skin matrices.
It spends about 16 percent propagating transforms and marking transform trees.
Evidence: `target/mc-evidence/stable-model-transforms/post-rig-profile/`.
That profile uses the optimized preview. Final comparisons use the release build.

This is an engine implementation change. Public API 0.2.10 and protocol 6 stay unchanged.
The server, game rules, package files, animation detail, animation cadence, and effects stay unchanged.
A mod describes its model and clips. It does not select this renderer path.

## Rules

- Share only poses that the existing crowd evaluator already groups together.
  Preserve every unit's movement clock, facing, transitions, blend weights, death pose, and declared anchors.
- The shared palette is relative to the mesh anchor. Each instance retains its own world transform.
  For a joint J, bind matrix B, and anchor A, the palette entry is inverse(A) * J * B.
  The vertex shader applies the instance anchor before that entry.
  Use the matching inverse-transpose normal transform. Preserve tangent handedness and all vertex attributes.
- Preserve PBR materials, team colours, hit flashes, transparency, shadows, and depth rendering.
  The hit flag occupies one tag bit. Palette addressing must not overwrite it.
- Keep authored hierarchy and named anchors. A rendered pose and its muzzle use the same animation time.
- Bound the atlas to 65,536 matrices and the supported device limit, whichever is smaller.
  Unsupported layouts, morphs, invalid palettes, nonfinite transforms, absent dependencies, and overflow use the existing skin path.
  Unsupported render passes also use the existing path. An optimization must not reject an accepted package.
- Keep source meshes, inverse-bind assets, and source materials unchanged.
  New mesh data and renderer materials are private copies.
- A model is ready only when its actual mesh, actual material type, and pipeline can draw.
- Provide a renderer reference option for automated tests. It affects presentation only.
  Do not add an implementation switch to the player HUD.

## Acceptance

1. Native tests compare instance positions, normals, tangents, palette indices, and tag bits with the existing path.
   Include different root transforms, scales, joint poses, and fallback cases.
2. A real GPU check runs the production palette math. It agrees with the native reference.
3. Real rendered combat retains models, team colours, bolts, sparks, hit flashes, and team-colour return.
   Walking, muzzle anchors, death, quality changes, and fallback remain correct.
4. Record release High/Low results at 400 and 2,000 visible, damaged actors with the existing 500 ms scene-read protocol.
   Record a real 400-unit pan. High stays at or above 30 FPS at 400 units.
   Report the 2,000-unit result and remaining limits. Do not hide actors or reduce quality to improve it.
5. One independent review runs code after the build. Fix its findings without another review loop.
6. Final scripted player probes and the actual launcher pass before delivery.

No main merge or push. Every test browser uses a fresh profile and `--alsa-output-device=null`.
Keep `target/mc-evidence`. Reuse the existing native and WASM caches.
