# SEL1 Forge fixture

This fixture is a byte-for-byte copy of `mods/forge` with one test-only map placement added:
an existing `forge:turret` at `(26, 8)` owned by slot 1. The turret uses the existing Forge unit
definition, including its authored weapon range. No production Forge map, unit, order, or content
id changes for this fixture.

The placement gives the pixel-only SEL1 row a foreign unit with a real weapon range circle. Keep
every other package file identical to `mods/forge`. When the Forge HUD and assets are added, copy
those files into this fixture byte-for-byte so the map remains its only package delta.
