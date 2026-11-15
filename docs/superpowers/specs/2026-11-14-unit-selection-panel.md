# Skirmish selection information

Use the existing interface API. Do not change the engine or match rules.
Selecting a unit or building shows its portrait, name, and current / maximum health.
The health bar follows the selected primary unit. Enemy health also appears.
An empty selection hides the panel. A multiple selection shows its existing primary unit.
The panel sits between order buttons and the build list. It must not cover either.
Portraits are 256 px static renders of the five existing CC0 game models.
They do not replace slot colours on world models.

Acceptance:
- Select castle, villager, crossbowman, barracks, and swordsman with real browser clicks. Check the name, portrait pixels, and health text.
- Attack an enemy. Its selected health text must decrease with the replicated health.
- Clear selection. The panel hides. Select several units. The primary unit remains readable.
- Check panel bounds at 1280×720 and 1024×768. Run all three scripted cold-player probes.
- Run the actual single-player launcher. Record evidence and limits.
