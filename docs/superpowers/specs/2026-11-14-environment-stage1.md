# Environment stage 1: ground, haze, and antialiasing

## Player result

The medieval skirmish sits in a wider grass landscape.
A thin line marks the playable map boundary. The outside ground is decoration.
A light distance haze softens the far landscape. Unit colours remain readable.
The world uses a light background instead of the dark void.
High and Medium use FXAA. Low has a cheaper path.
The minimap, orders, map bounds, and match results keep their current rules.

This is the first P4 slice. It does not complete P4.
Water, heightfield visuals, blended terrain materials, a full sky, weather, and more lights remain later work.

## Mod API contract

Release API 0.2.10. Add optional `environment` to MapDef.
An absent environment preserves the existing ground extent, background, fog, and antialiasing behavior.
An explicit environment has these data fields. No field contains a renderer type.

- `background`: an opaque #RRGGBB colour.
- `ground_margin`: finite world units of decorative ground beyond each map edge; default 0; range 0 to 10000.
- `boundary_colour`: optional #RRGGBB colour for a thin line at the actual map boundary; default None.
- `fog`: optional record with `colour`, `start`, `end`, and `opacity`; default None.
  Colour uses #RRGGBB. Distances are finite world units, 0 <= start < end <= 10000.
  Opacity is finite, from 0 to 1; default 0.2.
- `antialiasing`: Off or Fxaa; default Fxaa.

The environment belongs to the same selected map as its placements and dimensions.
The engine draws it. The server never uses it for admission, simulation, navigation, or visibility.
Clients with different quality settings receive the same world state.
High and Medium apply authored fog and FXAA. Low keeps background, ground margin, and boundary, but disables fog and FXAA.
Switching quality during a game updates these choices without reconnecting.
FXAA uses one sample. It must not run together with multisampling.
Packages that name environment fields must require API 0.2.10 or later.
Old packages continue to load. Protocol 6 and the wire layouts stay unchanged.

Skirmish art stays unchanged. Reuse its existing grass texture.
Keep its haze opacity at or below 0.2 so the battle stays readable at every supported zoom.
Do not change the authoritative map dimensions or placements.

## Acceptance

| Row | Check |
| --- | --- |
| ENV1 | An old package without environment loads. Its presentation defaults stay unchanged. Environment fields fail below API 0.2.10 and pass at that version. |
| ENV2 | Invalid colours, nonfinite values, negative margins, reversed fog distances, excessive distances, out-of-range opacity, and overflowing expanded UV ranges fail validation. |
| ENV3 | Decorative ground extends past all four boundaries. The visible boundary uses the original map dimensions. Units cannot move or build outside those dimensions. |
| ENV4 | The real browser shows the new background, textured ground, boundary, and light haze. Models and unit colours stay readable. |
| ENV5 | Switching High to Low and back toggles fog and FXAA. A package without environment keeps its existing camera behavior. No duplicate post-process is installed. |
| ENV6 | Loading and stepping the same match with and without environment produces the same audited simulation state. The minimap still uses the authoritative map dimensions. |
| ENV7 | Record frame p50, p95, memory high-water, and GPU details at 400 and 2000 units on High and Low. High stays at or above 30 FPS at 400 units. Record a 400-unit pan check too. Fix a regression before delivery. |
| ENV8 | One independent review runs code after the build. Fix its findings. Final scripted cold probes and the real launcher pass. |

Do not merge or push main.
Every test browser uses a fresh profile and --alsa-output-device=null.
Keep target/mc-evidence. Reuse shared build caches.

## Map selection

Use the verified game package's map for dimensions, ground, and environment.
Unverified native fixtures retain their first-map fallback.
Select all three from one map definition. Do not borrow presentation from a later map.
