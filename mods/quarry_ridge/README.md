# Quarry ridge

M4b public group demonstration for Quarry. This standalone Game package has one map with the frozen GM2-narrow layout of `tests/browser/terrain-group/scenes.json`. A Game package exports exactly one map, so the default `mods/quarry` stays unchanged (decision 2026-11-23, `vault/Decisions/2026-11-23 M4b ridge demo as standalone packages.md`).

Map: 64 x 48 cells. A ridge of four height steps runs along X[30,34) and is closed for the large units. A narrow opening at Y[23,25) admits the small units only. You own 24 Walkers (radius 0.4) and 12 Crawlers (radius 1.1).

How to play the person check:

1. Drag a selection box around all your units. Right-click the far side of the ridge, near (50, 24).
2. The Walkers go through the narrow opening. The Crawlers wait with a status.
3. Select one Walker. Choose the Spade at the bottom left. Drag over the ridge around the opening (X 30 to 34, Y 20 to 28) and release: the area is dug down to the lowest ground that you see round it. Right-click cancels.
4. When the passage is wide enough, the waiting Crawlers go on with their first order. Do not give a new order.

Frozen fixture values (from the generator, not the default Quarry balance): Walker and Crawler inventory 256, Spade and Backfill 64 work per second, light and wide classes max step 0. The dug soil goes into the Walker's inventory. Visibility is off on this map.

Models and the ground texture are byte-equal copies of `mods/quarry/assets`. See `assets/CREDITS.md`.
