# Two-player skirmish

Status: VS1–VS6 pass. See `docs/reports/2026-11-14-two-player-skirmish.md`.

## First delivery

Add `--human` to the skirmish launcher. This mode leaves both server slots for human players.
Keep the computer game as the default. Print a command that starts a second fresh browser at the same match.
Serve the client through the native HTTP server. Retain gzip downloads.
Use the existing authority, wire messages, slot claims, and immediate match start.
The HUD must give correct instructions to either player.

This first delivery supports two browsers on one computer. It does not claim Internet or LAN hosting.
WebGPU needs a secure browser context. Trusted HTTPS hosting is a later delivery.
There is no ready lobby yet. The first player can act before the second joins.
There is no shared rematch button yet. Restart the host for a new human match.

## Acceptance

| Row | Check |
| --- | --- |
| VS1 | The default launcher reserves the computer slot. `--human` leaves it free and prints a usable join command. |
| VS2 | Two fresh, silent browsers join the same match and receive different slots. Both load models and their own HUD. |
| VS3 | Each player selects and moves their own unit through mouse input. Both browsers observe each move. Enemy selection cannot expose own-unit orders. |
| VS4 | Player two builds Barracks, harvests, and trains through its own HUD. Both browsers observe the resulting units. |
| VS5 | A normal-stat fight ends with Victory on one browser and Defeat on the other. Both observe the same surviving castle. |
| VS6 | Scripted cold probes pass with the updated package. Both fresh profiles are removed after the two-player check. |

No server or protocol changes are planned. If implementation needs a foundation change, specify it and run one independent review.
