# Skirmish presentation assets

The five static models and the ground texture come from the supplied fantasy art set.
See [CREDITS.md](CREDITS.md) for the CC0 sources and changes.

The models use +Y up and +Z forward. Their pivots are at ground level.
The soldier uses scale 0.88. Its crossbow reaches beyond the body.
That scale passes the model fit check without changing the collision radius.
The other models use scale 1.0. The grass texture repeats every eight map units.
The team material is named `team`. The trees have no team material.

The Villager and Crossbowman now use the animated variants. Walking stride is measured from the embedded foot-joint motion. The measurements are in `target/mc-evidence/p2-stride.json`.
