---
type: decision
date: 2026-10-28
status: implementation-ruling
tags: [turts, m3, boot]
---

# M3 boot binding interpretations

These are orchestrator interpretations of the approved prototype contract, not new CEO decisions or long-term API policies. T4 implementation and review must verify them.

- SPEC7 says nearest node in range: sort geometrically eligible nodes by distance, then NodeId; require the selected node's item to be accepted. An incompatible nearest node rejects instead of selecting a farther accepted node. Forge has one node, so its demonstration is unchanged.
- SPEC4.1 declares conveyor lines explicitly in map.belts. A conveyor in ordinary placements rejects on placements[i].unit. A one-piece line uses pieces:1. No implicit line or adjacency rule is added.
- No produced-unit facing default is selected. The current production schema has no facing field, so a recipe producing an orientation-required inserter must reject with a source-aware output_unit diagnostic. Map placement with explicit facing remains supported. A later API may add an explicit facing input; this rule does not settle that design.
- Existing M2 production already requires mobile, non-producer output units (runtime content.rs). Preserve that restriction. SPEC7's static extractor/main-inventory requirement and SPEC4's static conveyors are also checked on the portable path.
- All creation paths prepare capability rows and target bindings before mutation, then apply them infallibly. Synthetic creation must not silently use empty bindings for a targeted capability. Main inventory preload is preserved; output inventory, health and the other new rows must not disappear on non-map creation paths.

If an interpretation is wrong, expected rework is limited to those validation/binding rules and their tests. No new public schema field or wire version is introduced by this note.
