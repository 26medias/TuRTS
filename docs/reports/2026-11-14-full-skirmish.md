# Full normal-stat skirmish checks

Status: full browser win, loss, and both real restarts pass.
Game engine and launcher: `113e86f0`. Driver: `bc68ca11`.
No game stats, map data, or commands were injected. Main is unchanged.

The driver opens the real launcher in a fresh silent Chrome.
It uses mouse input, HUD buttons, box selection, and the minimap.
Debug hooks observe state. They do not submit orders.
Both castles start with 1,000 health. Swordsmen keep 300 health.

| Check | Result | Evidence under `target/mc-evidence/full-skirmish/` |
| --- | --- | --- |
| Harvest and production | PASS. Earn 180 credits from timber. Build Barracks. Train two Swordsmen and three Crossbowmen. | `win-final/result.json`; `win-final/mixed-army.png` |
| Mixed army selection and fight | PASS. Select six combat units with a box. Defeat the computer's defenders and destroy its normal castle. | `win-final/result.json`; `win-final/selected-army.png` |
| Normal Victory | PASS at 132.4 seconds. The Victory title is visible. | `win-final/terminal.png`; `win-final/result.json` |
| Restart after Victory | PASS. A new match ID, both full-health castles, and ready models appear. | `win-final/restart.png`; `win-final/result.json` |
| Normal Defeat | PASS at 217.3 seconds. The computer attacks and destroys the idle player's castle. | `loss/terminal.png`; `loss/result.json` |
| Restart after Defeat | PASS. A new match ID, both full-health castles, and ready models appear. | `loss/restart.png`; `loss/result.json` |
| Cleanup | PASS. Both fresh browser profiles were removed. | Both `result.json` files |

The win wrapper command and wrapper both exit 0. Source stayed unchanged.
Receipt: `target/mc-evidence/full-skirmish-win-final/receipt.json`.
The loss command exits 0 on its own private Xvfb display. It is a logical player check, not an FPS measurement.
Command log: `target/mc-evidence/full-skirmish/loss-command.log`.
Every browser has `--alsa-output-device=null`. User Chrome and system sound are untouched.

The earlier `win` and `win-verified` folders are failed driver development runs.
The first clicked occluded feet inside a clustered army. The driver now uses normal box selection.
The second tried to click a target that died between observation and input. The driver now selects a live target again.
Neither failed run is an accepted full-win result.

The current launcher remains `/home/julien/Projects/TuRTS/target/mc-evidence/play-skirmish.sh`.
This closes the normal-stat match gap left by the earlier 30-HP victory fixture.
No new human playtest or other-hardware performance test was done.
Next: two-player host and join mode. Full lobby and public hosting acceptance remain open.
