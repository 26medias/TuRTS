# Visibility probe

This public API16 package provides a two-human scouting probe. It uses ordinary authored vision and orders.
It does not use a privileged engine path. Neither unit has a weapon or an AI controller.
The scene uses the existing primitive renderer and existing Acme order icons.

Slot 0 controls the survey scout. Slot 1 controls the survey contact.
Both units have sight radius 10 on a grid with cell size 2.
The deposit supplies a stale surface-resource observation without an extraction promise.
There is no victory rule, so the probe can finish its scouting sequence.

Serve this package with the accepted server and browser build. Leave both slots human.
Use the resulting game URL with this command:

```sh
node tests/browser/shared-visibility.js URL tests/browser/fixtures/shared-visibility.json OUTPUT
```

The driver opens two fresh silent Chrome profiles and its own Xvfb.
It moves the scout from (8,12) to (28,12), then back to (8,12).
It moves the hidden contact from (36,12) to (52,12).
It then moves the scout to the old empty location at (36,12).
The screenshots compare the same three knowledge states on ground and minimap.
This fixture does not change Skirmish or its AllVisible default.
