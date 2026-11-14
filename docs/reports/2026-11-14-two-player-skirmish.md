# Two-player skirmish

Status: the full local two-player match passes. All scripted cold probes pass.
Implementation: `59459914`. Main is unchanged.

Add `--human` to `scripts/skirmish-play.sh --prepared DIR --silent`.
The host prints a `JOIN_COMMAND`. Run that command in another terminal.
Each command starts its own fresh browser. Both commands use null sound in agent runs.
The default command still starts a computer opponent.

| Row | Result | Evidence under `target/mc-evidence/two-player/` |
| --- | --- | --- |
| VS1 | Human host prints its join command. Both human slots are available. | `match/host.log`; `match/result.json` |
| VS2 | PASS. Same match ID, slots 0 and 1, separate profiles, ready models. | `match/result.json`; `match/ready-0.png`; `match/ready-1.png` |
| VS3 | PASS. Both players move their own soldier. Both replicas observe each move. Enemy castle selection exposes no build button. | `match/result.json`, `moves` and `enemy_selection` |
| VS4 | PASS. Player two harvests 180 credits, builds Barracks, and trains a mixed army. | `match/result.json`; `match/army.png` |
| VS5 | PASS. Player two wins. Player one loses. Both results occur at tick 2470. | `match/result.json`; `match/terminal-0.png`; `match/terminal-1.png` |
| VS6 | PASS. Fresh profiles removed. Both first-click probes and the buttons probe pass. | `match/result.json`; `cold-1280/`; `cold-1024/`; `cold-buttons/` |

The package keeps normal stats. Both castles start with 1,000 health.
The winning castle retains 1,000 health. No wire commands or game-state mutations were injected.
The browser driver uses mouse input and observes debug records. Its command exits 0.
The client, server, and packer binaries are byte-identical to the compressed build. Only package text changes.
No authority, wire, determinism, or mod API code changes were needed.

This delivery supports two browsers on one computer. Internet and LAN hosting are not verified.
There is no ready lobby. The first player can act before the second player joins.
There is no shared rematch button. Restart the host to start another human match.
No new human playtest or two-browser FPS measurement was done.

Cold wrapper receipt: `target/mc-evidence/two-player-cold-checks/receipt.json`.
Command and wrapper exit 0. Source stayed unchanged.
Human launcher: `/home/julien/Projects/TuRTS/target/mc-evidence/play-two-player.sh`.
The computer launcher remains `target/mc-evidence/play-skirmish.sh`.

The updated computer launcher also passes the real launcher check.
Evidence: `target/mc-evidence/two-player/launcher/result.json`.
It downloads gzip, loads models and the map, keeps sound off, and removes its profile.
