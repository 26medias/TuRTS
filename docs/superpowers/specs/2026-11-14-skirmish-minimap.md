# Skirmish minimap

Status: planned follow-up to the swordsman. Use the existing MinimapView contract.
No server, wire, or mod API change is planned.

Show a compact map in the lower right. Show own and enemy unit colors, resource markers, and the camera frame.
Left-click or drag to look at another part of the map. Right-click to send the selected units there.
Show an attack ping when the existing offscreen warning fires. Remove old pings after two seconds.
Keep the map clear of sound controls, production rows, and the credits display at both tested window sizes.
Keep existing art and unit counts. Do not reduce quality to meet the frame target.

| Row | Acceptance |
| --- | --- |
| MAP1 | Browser pixels show both team colors, resource markers, and the camera frame in the authored map. |
| MAP2 | Left-click and drag change the main view to the requested map positions. Selection stays unchanged. |
| MAP3 | Right-click sends a selected unit to the requested map position. The main view stays unchanged. |
| MAP4 | A real offscreen attack shows a visible map ping. The ping expires after two seconds. |
| MAP5 | The 400-unit High combat measurement stays at least 30 FPS. Record p50, p95, and memory. |
| MAP6 | Both cold first-click probes and the buttons probe pass. The launcher starts the tested build. |

Run the browser checks on a fresh silent Chrome. Keep the previous launcher until the probes pass.
Fix performance issues without waiting for a user instruction.
