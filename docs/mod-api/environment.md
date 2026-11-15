# Map environment (API 0.2.10)

`MapDef.environment` is optional. An absent value preserves the previous camera and ground defaults.
Use `engine_api: "^0.2.10"` when writing this field.

```ron
environment: Some((
    background: "#B8D4DF",
    ground_margin: 64.0,
    boundary_colour: Some("#A7966D"),
    fog: Some((colour: "#B8D4DF", start: 80.0, end: 220.0, opacity: 0.2)),
    antialiasing: Fxaa,
)),
```

Colours use opaque #RRGGBB values. Ground margin is finite, from 0 to 10000 world units.
The margin adds decorative ground outside each edge. It does not change the map bounds.
The boundary line uses the original map dimensions. Units and buildings obey those dimensions.
The existing ground texture keeps its world scale and origin when the margin changes.
The margin, map size, and tile size must produce finite UV coordinates.
Both content and model validation reject an overflowing combination.

Fog distances are finite world units. They satisfy 0 <= start < end <= 10000.
Opacity is finite, from 0 to 1. Its default is 0.2.
Ground margin defaults to 0. Boundary colour and fog default to None.
Antialiasing defaults to Fxaa. Off disables it.

High and Medium apply fog and FXAA. Low disables both effects.
All qualities keep the background, ground margin, and boundary.
An explicit environment uses one sample. FXAA never combines with multisampling.
Changing graphics quality applies during the match, without reconnecting.

The verified game package selects the map. Dimensions, ground, and environment come from that map.
Native fixtures without a verified root retain their first-map fallback.
The server validates these values. It does not use them to change simulation or visibility.
Protocol 6 and the wire layouts stay unchanged.

This is the first P4 slice. It does not implement water, heightfield visuals, a full sky, or weather.

## Ground overlay (API 0.2.11)

`ground.overlay` adds one bounded layer without changing ground geometry or match rules.
Use `engine_api: "^0.2.11"`. Omit the field to keep the original material.

```ron
overlay: Some((
    colour: "#9B8057", base_colour: None, tile_size: 8.0, roughness: 1.0,
    patches: [
        (centre: (x: 10.0, y: 24.0), size: (x: 14.0, y: 14.0),
         rotation_deg: 0.0, feather: 3.0, opacity: 1.0),
    ],
)),
```

The image, when present, multiplies the colour. Both colours enter lighting in linear space.
An absent image uses white, so the authored colour works alone.
`tile_size` uses world units and must be finite and positive. World UV ranges must be finite.
`roughness` defaults to 1 and stays between 0 and 1.
There are at most 16 patches. A patch is a rectangle in world map coordinates.
Size is the full width and depth, each positive and at most 10000 world units.
Centre coordinates range from -10000 to 10000. Rotation ranges from -360 to 360 degrees.
Feather is at least 0.00001 world units and no larger than half the smaller size.
This minimum matches the shader precision guard. Smaller values fail package validation.
It fades inward from the rectangle edge. Opacity defaults to 1 and ranges from 0 to 1.
Every numeric value must be finite. Overlaps use the maximum mask.

The layer blends base colour, roughness, and the base normal towards the geometric normal before lighting.
All qualities retain the masks. Optional images use the existing texture limits.
The shader uses at most 16 masks and one added image sample per ground fragment.
The server validates the data. It never uses the layer for navigation, visibility, or match results.
This slice does not add heightfield geometry, water, or custom mod shaders.
