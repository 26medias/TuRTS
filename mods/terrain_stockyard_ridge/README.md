# Terrain Stockyard ridge

M4b public group demonstration for Terrain Stockyard. This standalone Game package has one map with the frozen GM2-narrow layout of `tests/browser/terrain-group/scenes.json`. A Game package exports exactly one map, so the default `mods/terrain_stockyard` stays unchanged (decision 2026-11-23, `vault/Decisions/2026-11-23 M4b ridge demo as standalone packages.md`).

Map: 64 x 48 cells. A ridge of four height steps runs along X[30,34) and is closed for the large units. A narrow opening at Y[23,25) admits the small units only. You own 24 Porters (radius 0.4) and 12 Carriers (radius 1.1).

How to play the person check:

1. Drag a selection box around all your units. Right-click the far side of the ridge, near (50, 24).
2. The Porters go through the narrow opening. The Carriers wait with a status.
3. Select one Porter. Choose the Scraper at the bottom left. Click an observed cell to capture its reference level. Drag a rectangle over the ridge around the opening (X 30 to 34, Y 20 to 28) to level 0.
4. When the passage is wide enough, the waiting Carriers go on with their first order. Do not give a new order.

Frozen fixture values (from the generator, not the default Terrain Stockyard balance): Scraper and Leveler 64 work per second, compact and broad classes max step 0, four resource units per world cube. Dug loam goes to the slot balance. Visibility is off on this map.

Models and the ground texture are byte-equal copies of `mods/terrain_stockyard/assets`. See `assets/CREDITS.md`.
