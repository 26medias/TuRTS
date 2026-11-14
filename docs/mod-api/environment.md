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
