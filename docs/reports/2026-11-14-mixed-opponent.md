# Mixed computer army

Status: BOT1–BOT4 pass. Policy implementation: `f61f7468`.
The prepared build also contains the final ready-lobby client. Main is unchanged.

The computer now trains the least represented available fighting type.
It replaces a missing gatherer first. Paid jobs count toward the type population.
Equal populations use the package recipe order. Unit ids do not rank these choices.
The opening delay, six-unit army cap, attack waves, and ordinary paid orders remain.

| Row | Result | Evidence under `target/mc-evidence/mixed-opponent/` |
| --- | --- | --- |
| BOT1 | PASS. The simulation builds Barracks, harvests, trains three Crossbowmen and three Swordsmen, and defeats an idle player. | `native.log`; `server.log` |
| BOT2 | PASS. Barracks and the trained army cost at least 420 credits against 200 starting credits. The ordinary production path charges these costs. Timber deliveries fund the difference. | `native.log`; `server.log`; normal package production files |
| BOT3 | PASS. Repeated joins produce identical audited states. No computer economy starts before a human joins. | `native.log` |
| BOT4 | PASS. Browser Victory at 129.98 seconds and Defeat at 209.61 seconds. Both New skirmish buttons start a fresh match. Both first-click probes and the buttons probe pass. | `win/result.json`; `loss/result.json`; `cold-1280/`; `cold-1024/`; `cold-buttons/` |

Both games use the normal package and real launcher commands.
The winning driver harvests, builds, trains, selects, and fights through mouse input.
No game stats, map placements, or wire commands are injected.
Both fresh browser profiles are removed.

All 113 server tests and strict Clippy pass.
Only the server binary changes from the final ready-lobby build.
Browser, packer, and package hashes match exactly. See `artifact-reuse.json`.
The protocol and public mod API remain at 6 and 0.2.9.
This is an opponent policy change. It does not complete the MP2 bot framework.

The final browser runs the 400-unit mixed combat guard at 46.42 FPS High and 56.91 FPS Low.
All 400 actors stay visible and take damage in both timing windows.
No other test browser or compiler runs during the samples.
Evidence: `400-guard/result.json`. Measurements are in `docs/measurements/models.md`.
This workload disables the computer player. It measures rendering and combat, not bot planning cost.
No new 2,000-unit or other-hardware measurement was run.

Cold wrapper: `target/mc-evidence/mixed-opponent-player-checks/receipt.json`.
Command and wrapper exit 0. Source stays unchanged.
The checker reports PARTIAL-PASS because manual checks remain open.
No new human playtest or full manual readiness claim is made.
Every test browser uses a fresh profile and --alsa-output-device=null.
