# Weapon range (API 0.2.8)

Use `range_mode: Surface` for melee weapons. Their `range` is the gap between the two circular footprints.

```ron
weapon: Some((
    range: 0.6,
    range_mode: Surface,
    cooldown_ms: 800,
    damage: 25,
    ammo: None,
    targets: (slots: Enemy),
))
```

Declare `engine_api: "^0.2.8"` when you write `range_mode`, including `Centre`.
Omit the field to keep the existing center-distance behavior.

The server uses `range + source radius + target radius` for Surface weapons.
Firing, attack approach, and attack-move use the same reach.
The selected range ring includes the source radius. A target footprint can intersect that ring to be in range.

This field does not change damage, collision, or target priority.
The current engine has no attack damage reduction mechanic.
