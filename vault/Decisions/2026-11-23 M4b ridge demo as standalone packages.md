---
type: decision
date: 2026-11-23
tags:
  - turts
  - m4b
  - packages
ai-first: true
source: tower
---

## For future Claude
The M4b group demonstration is two standalone Game packages, `mods/quarry_ridge` and `mods/terrain_stockyard_ridge`. The default `mods/quarry` and `mods/terrain_stockyard` stay byte-identical. A Game package exports exactly one map.

# M4b ridge demo as standalone packages (Iris with Clara and Viktor, 2026-11-23)

**Status:** Decided by Iris (CEO) with Clara (game director) and Viktor (technical director) on 2026-11-23. Relayed by Tomas (pm) in the M4b task T-1. Their own words are not in this record.

**Question:** the M4b spec (`docs/superpowers/specs/2026-11-15-terrain-group-movement.md`, "Public demonstrations", lines 88 to 94) says to extend Quarry and Terrain Stockyard. The shipped maps give the player 3 units and cannot show a group. A second map in these packages is refused: a Game package exports exactly one map (`crates/turts_mod_runtime/src/manifest.rs:78`, `crates/turts_pack/src/lib.rs:28`), and the server has no map selection at launch.

**Options that were given:**
- A: two standalone Game packages with one ridge-group map each. No engine change.
- B: packages with several maps and a map-selection launch path. A public mod API change (foundation track).
- C: replace the map world of the default `field.ron`. This changes the default maps.

**Decision:** A.
- `mods/quarry_ridge` and `mods/terrain_stockyard_ridge`. Each has one map with the frozen GM2-narrow layout of `tests/browser/terrain-group/scenes.json`: 24 small units (radius 0.4), 12 large units (radius 1.1), a ridge at X[30,34) that is closed for the large units, and a narrow opening at Y[23,25).
- Public package files only. The models, textures and camera are the existing ones (byte-equal asset copies).
- An ordinary excavation order opens the ridge. The waiting large units resume with no replacement order.
- Launch through the normal public launcher: `scripts/mp1-play.sh --prepared DIR --package mods/quarry_ridge`.
- The default `mods/quarry` and `mods/terrain_stockyard` stay byte-identical.

**Owners and follow-up:**
- Hugo (lead-eng): the packages, the pack and launch proof, the bundles, the person-check sheet.
- Marco (client-eng): the direct browser checks after the timing window.
- If the frozen Quarry cargo needs extra map geometry, ask before a change to the approved layout.
