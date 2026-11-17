# Utility Yard

API 0.2.18 (M6) underground utility fixture. It uses only the public mod API. It is a copy of
Terrain Stockyard with one power utility added.

## Content

- `power` is a `Supply` resource with scope `Network`. Only connected networks count it.
- `power_line` carries `power`. Its ports are on the Generator and the Workshop. Depth is 1 to 4
  elevation steps (default 2). A segment costs 1 loam. The longest segment is 16 world units.
  Exposure policy: `Disconnect`.
- The Generator provides 10 power. The Workshop consumes 6 power. Without power its production
  runs at one quarter of the full rate.
- The Material depot is the only unit that can lay a line (order `order_route`).

## Player path

1. Press U. The terrain fades. The two ports show as grey squares.
2. Select the depot. Click "Power line". Click the Generator port, then a point on the ridge to
   the north-east. Press Escape.
3. Click "Power line" again. Click the Workshop port, then a point to the north-west. The two
   lines cross in plan view, but the Workshop port stays red ("Underpowered").
4. Click "Power line" again. Press `[` once for depth 1. Click the first line on the ridge, then
   the second line. This route makes two junctions. The Workshop port turns green ("Powered").
5. Press U to see normal terrain. Select a Porter. Dig the ridge cell under the depth-1 route down
   to the flat level. Press U: the route is exposed, dashed and disconnected. The Workshop is short.
6. Fill that cell back to the ridge level with the Leveler (it costs loam). The route conducts
   again. No line ever moves: its elevation is fixed at placement.

Hover over a line, junction or port in the view to see its state, the network supply and demand,
the depth, the elevation and the cover from the terrain that you can see.

## Limits

No water, sewage or heat model. No line removal by the player. Bots do not use utilities.
Models and ground texture are copied unchanged from Stockyard and Skirmish. See
`assets/CREDITS.md` for the Kay Lousberg and Quaternius CC0 credits. No artwork was regenerated.
