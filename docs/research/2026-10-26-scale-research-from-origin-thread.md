# Scale research from the origin thread (condensed)

Source: the CEO's ChatGPT thread "Choosing an RTS Engine", read on 2026-10-26. The share shows
only its last three exchanges. The thread ends with "Updated the specs to v0.6", a file this
repository does not hold (see the CEO note at the end). Statements below are the thread's
findings, condensed. They are research, not measurements.

## Principle

Large-scale performance needs separate representations for simulation, networking, and
rendering. A unit is a compact simulation record on the server, a few replicated properties on
the wire, and an animated model on the client. The three must not share one cost.

## Assets and animation

- Mod authors supply glTF/GLB models with metadata: animation states, attachment points,
  materials, effects, collision footprint. A publishing pipeline validates and compiles them
  (gltfpack, Khronos validator). Gameplay nodes such as turret pivots must survive optimization.
- The server never loads visual skeletons. It uses footprint, position, heading, turret angle,
  and gameplay state.
- A declarative visual definition maps gameplay state to clips and effects: model, movable
  nodes, animation mappings, attachment points, effects mappings, team colour mask, detail
  variants, visual bounds. A mod can replace appearance without touching combat logic.
- Animation events may trigger cosmetic effects. They are never the authority for damage.
- Rendering paths differ by asset: static buildings with moving parts; instanced vehicles with
  turret and wheel transforms; instanced skeletal animation with baked bone data for infantry
  crowds; simplified meshes and icons at distance. Bevy's "Many Foxes" shows per-unit animation
  management cost remains even with shared assets; measure it.
- Bevy's custom-instancing example disables frustum culling; a crowd renderer needs correct
  batch bounds or per-instance culling. Hanabi (GPU particles) needs WebGPU; a WebGL2 fallback
  needs another effects path. The browser graphics baseline is an explicit open decision.

## Server simulation

- Reduce work before distributing it. Factorio evidence: sleeping machinery with explicit
  wake-ups; incremental radar coverage; belt parallelism limited by shared state.
- Compact contiguous hot data; spatial indexes; scheduled low-rate decisions; parallelism only
  where dependencies allow. Offscreen units obey the same rules.

## Navigation

- Flow fields for shared destinations; they do not solve congestion, formations, or avoidance.
  Candidate: bevy_flowfield_tiles_plugin (tiled cost, integration, flow fields, route cache,
  async requests; steering left to the game).
- Detour crowd patterns for the local layer: corridors, proximity grid, bounded neighbour lists.
- Combine hierarchical routing, shared routes, bounded local avoidance, queued requests with
  per-tick budgets, and versioned navigation regions for terrain edits.
- Test both "5,000 units to one destination" and "5,000 units to different destinations".

## Replication

- Planetary Annihilation model: replicate changing properties over time; clients reconstruct
  presentation between updates. Replicate positions, orientations, state changes, events; never
  bones or particles.
- Lightyear is worth evaluating: WASM and WebTransport, interpolation, custom serialization,
  replication priorities with bandwidth limits; uses Replicon for world replication. Fog
  filtering, map subscriptions, and compact mod state stay engine work.
- Scale note: 16 bytes per unit at 10 Hz for 5,000 units is 800 KB/s per client before
  overhead. Compact encoding, unchanged-state suppression, and interest management are
  architectural requirements. Visibility filtering enforces what a player may know; detail
  reduction comes after.

## Large maps

- Separate problems: coordinate range (big_space style floating origin), asset streaming,
  active simulation regions, navigation storage. Chunked storage; switching the viewed map must
  not require rendering the previous one, while its simulation may continue.

## Prototype tests the thread asks for

| Test | Establishes |
| --- | --- |
| 1,000 / 5,000 / 10,000 animated units | Animation CPU, GPU, draw submission |
| Concentrated battle with shadows and effects | Worst-case visible rendering |
| Shared orders, unique orders, chokepoints | Navigation and congestion |
| Terrain changes during movement | Invalidation and rebuild cost |
| Multiple maps with active production | Simulation cost off-camera |
| Eight clients observing a large battle | Serialization, bandwidth, fan-out |
| Mixed models from several mods | Whether batching survives real content |

Measure browser CPU and GPU frame time apart from server tick time, plus memory, bandwidth, and
long-frame spikes. The 4-vCPU/8-GB VM is a target, not a capacity.

## CEO note

The thread's last message says the spec was updated to v0.6 with these findings. The repository
holds the v0.5 file; the studio's own revision 0.6 of 2026-10-26 changes only the manifest and
lockfile text. When the ChatGPT v0.6 file is added to the repository, merge it as 0.7 and
re-number the studio's changes.
