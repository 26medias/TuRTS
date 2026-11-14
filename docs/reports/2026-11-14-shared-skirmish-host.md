# Shared skirmish host

Status: SHARE1–SHARE5 pass. Combined code: `b047ccf0`. Main is unchanged.

One command starts the prepared authoritative game behind temporary HTTPS and WSS links.
Two players can open its game address. No account or system service is changed.
The command uses its own Cloudflare configuration. It stops only its own processes.
The local computer game and local two-player mode remain available.

| Row | Result | Evidence under `target/mc-evidence/shared-host/` |
| --- | --- | --- |
| SHARE1 | PASS. Startup, public package origin, failures, and child cleanup. | `tools-final.log`; `tools-merged.log` |
| SHARE2 | PASS. HTTPS join validation and local tool URL restrictions. Both real joins use the new command. | `tools-final.log`; `public/player-0.log`; `public/player-1.log` |
| SHARE3 | PASS. Real trusted HTTPS loads the package. WSS joins and ordinary orders succeed. | `public/result.json`; `public/player-0.png` |
| SHARE4 | PASS. Slots 0 and 1 share one match. Both clients observe both movement orders. | `public/result.json`; `public/player-1.png` |
| SHARE5 | PASS. Both fresh profiles, both tunnels, and the native server are gone after the test. | `public/result.json`, `profiles_removed` and `services_stopped` |

Both browsers use fresh profiles and `--alsa-output-device=null`.
No certificate checks were disabled. Both pages report a secure context.
The test connects through Cloudflare's public endpoints. Both browsers run on this computer.
Another computer, another GPU, and a full remote match played by two people were not tested.
The temporary test links are closed. This is development hosting, not a production service.

There is no ready lobby or shared rematch yet. Start another host for a new match.
The first delivery retried for about ten seconds after reload.
The fast rejoin delivery fixes that delay. Both new public reloads use one connection attempt.
See `2026-11-14-fast-rejoin.md`.

The prepared-build check no longer rejects a documentation-only commit.
It still verifies build source, package content, tools, environment, required files, and artifact hashes.
All 37 prepare tests pass: `prepare-tests.log`. No new game build was needed for this tool change.

Quick Tunnel behavior is documented at https://developers.cloudflare.com/tunnel/get-started/quick-tunnels/.

Launchers:

- Computer: `/home/julien/Projects/TuRTS/target/mc-evidence/play-skirmish.sh`.
- Two local players: `/home/julien/Projects/TuRTS/target/mc-evidence/play-two-player.sh`.
- Temporary shared game: `/home/julien/Projects/TuRTS/target/mc-evidence/play-shared-skirmish.sh`.

The shared command prints its HTTPS address. Each player can use:

```sh
bash /home/julien/Projects/TuRTS-presentation-play/scripts/mp1-play.sh --join HTTPS_GAME_ADDRESS --silent
```

The updated computer launcher also passes its real startup check.
Evidence: `target/mc-evidence/shared-host/launcher/result.json`.
It downloads gzip, loads models and the minimap, keeps sound off, and removes its profile.
