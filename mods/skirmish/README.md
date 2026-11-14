# Browser skirmish

Build a base, harvest timber, train an army, and destroy the enemy castle.
The server owns the match result. The computer builds, harvests, trains, and defends.
Its first small attack starts after 180 seconds.

Use `bash scripts/skirmish-play.sh --prepare DIR --silent` to build it.
Use `bash scripts/skirmish-play.sh --prepared DIR --silent` to start a fresh game.
Every agent run must use `--silent`. The scripted player probes are in `docs/qa/cold-play.md`.

Select your castle. Click **Build Barracks**. Click a free point near the castle.
Select a villager. Right-click a timber stand to harvest and deliver credits.
Select the completed Barracks. Train a **Crossbowman** for 40 credits or a **Swordsman** for 80 credits.
Crossbowmen fire at range. Swordsmen have more health and attack close to the target.
Right-click the ground with Barracks selected to set its rally.
Select your army. Right-click an enemy to attack it.

Use the Arrow keys or an open screen edge to pan. Space centers the selection.
J returns to the latest offscreen attack warning. F10 opens settings.
Use the lower-right map to look around with a left-click or drag.
Right-click the map to send selected units there. A red cross marks an offscreen attack for two seconds.
After a match ends, click **New skirmish** to start again.

The package has low-poly models, textures, walking and attack clips, and combat effects.
Art sources and licenses are in `assets/CREDITS.md`. No sound files are included. Sound starts off.

## Two human players on one computer

Add `--human` to the prepared launcher. Copy its `JOIN_COMMAND` into a second terminal.
The second command starts a separate browser with a fresh profile. Keep the host open.
Both players can act immediately. There is no ready lobby yet.
Restart the host for a new match. This mode has no shared New skirmish button.
The address is local to this computer. Internet hosting is not included yet.

The map includes decorative grass beyond its boundary and light distance haze.
The line marks the playable area. Buildings and units stay inside it.
High and Medium use haze and FXAA. Low uses the cheaper visual path.
The existing artwork and grass texture are unchanged.
