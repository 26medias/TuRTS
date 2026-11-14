# Ground blending: base clearings and timber paths

## Player result

The skirmish has soft dirt clearings around both castles and timber stands.
Two short dirt paths connect each base area to its timber.
Grass remains visible outside these areas. Edges blend without raised geometry.
Units, orders, building sites, and the minimap keep their current rules.
This is a second P4 slice. Water, heightfield geometry, sky, and weather remain later work.

## Contract

API 0.2.11 adds optional `overlay` to GroundDef. Absence preserves the existing material.
The overlay has an opaque `colour` (#RRGGBB), optional `base_colour` image reference,
`tile_size` (finite, positive), `roughness` (finite, 0 to 1, default 1), and `patches`.
An image multiplies the colour. Without an image the layer uses the colour alone.
A patch has `centre` (map x/y), `size` (full width/depth), `rotation_deg`,
`feather` (world units), and `opacity` (0 to 1, default 1).
Patch values are finite. Centres and sizes are bounded to 10000 world units in magnitude.
Sizes are positive. Feather is positive and no larger than half the smaller size.
Rotation is between -360 and 360 degrees. There are at most 16 patches.
Patches are rotated rectangles. The layer fades to zero inward from each rectangle edge.
Overlaps take the maximum mask, so a path does not become darker at a join.
World X/Z positions drive the mask and texture coordinates. Camera movement does not move the layer.
The layer blends base colour and roughness before PBR lighting.
The base normal blends towards the geometric normal inside the layer.
It adds no mesh, collider, navigation data, or frame callback for mods.
High, Medium, and Low keep all patches. Existing quality texture limits apply to its optional image.
A missing layer uses the original ground material and shader.
A missing optional image at runtime uses the authored colour. Pack-time validation rejects unknown image references.
The GPU cost is one bounded loop of at most 16 masks per ground fragment and at most one added texture sample.
The server validates presentation data but never uses it to decide a match.
Old packages load. Overlay fields require API 0.2.11. Protocol 6 stays unchanged.

Skirmish uses existing art only. Dirt uses a colour layer with no added texture download.
Do not change map dimensions, unit stats, placements, or resources.

## Acceptance

| Row | Check |
| --- | --- |
| GB1 | Old packages retain their ground. Overlay fields reject API 0.2.10 and load at 0.2.11. |
| GB2 | Validation rejects invalid colours, references, sizes, feathers, rotations, opacity, roughness, count, nonfinite data, and overflowing UV coordinates. |
| GB3 | Real GPU rendering shows clearings and paths. Compare the same scene without the layer. Check soft edges, world alignment, lighting, and no shader errors. |
| GB4 | High to Low and back preserves patches and applies texture limits. An optional textured fixture renders. |
| GB5 | The same match steps to the same audited state with and without the layer. Map bounds, movement, and minimap dimensions stay unchanged. |
| GB6 | Measure 400 and 2000 units on High and Low, including p50, p95, memory high-water, GPU, and a 400-unit pan. High stays at least 30 FPS at 400 units. Repair regressions. |
| GB7 | One independent review runs code after the build. Fix its findings. Scripted cold-player probes and the actual launcher pass on the final build. |

Do not merge or push main. Every test browser uses a fresh profile and --alsa-output-device=null.
