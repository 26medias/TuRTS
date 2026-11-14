# Browser download compression

Status: delivered. All ZIP1–ZIP5 checks pass.
Engine/tool head: `113e86f0`. The stable play worktree stays at this head.
Main is unchanged.

## Player result

Chrome receives 14,821,363 encoded WASM bytes instead of 59,686,193 raw bytes.
The recorded wire transfer is 14,821,608 bytes, including HTTP overhead.
This is about 75 percent fewer WASM bytes. The decoded hash is unchanged.
The JavaScript sidecar is 29,467 bytes instead of 194,386 bytes.
The game keeps the tested minimap renderer, models, effects, controls, and sound default.

The prepare step creates deterministic gzip sidecars. Original files remain available.
The native static server and the local New skirmish host negotiate gzip.
The local host keeps MIME, length, Vary, HEAD, query, and conditional behavior.
A request that excludes gzip receives the original file.
Package archives, game messages, and simulation are unchanged.

## Acceptance evidence

Paths below are relative to `target/mc-evidence/compression/`.

| Row | Result | Evidence |
| --- | --- | --- |
| ZIP1 | PASS, deterministic sidecars; exact decoded bytes; unhelpful sidecar removed | `local-green-final.log`; `review-artifacts.json`; `size.json` |
| ZIP2 | PASS, real local HTTP negotiation, GET/HEAD, query, MIME, length, Vary, 304, identity preference, exclusions, and retry security | `local-green-final.log`; `edges-green-final.log`; `retry-final.log` |
| ZIP3 | PASS, real native gzip and identity/q=0 requests preserve WASM hash; Chrome header works | `native-green/result.json`; `review-native-edges.json` |
| ZIP4 | PASS, actual launcher in fresh Chrome; gzip transfer; loaded models and map; sound off; profile removed | `launcher/result.json`; `launcher/game.png` |
| ZIP5 | PASS, both first-click probes and buttons | `cold-1280/probe-first-click.json`; `cold-1024/probe-first-click.json`; `cold-buttons/probe-buttons.json` |

The old local host fails the new HTTP checks: `local-red.log`.
The old native server fails the actual gzip response check: `native-red.log`; `native-red/result.json`.
The fixed paths pass. All 37 prepare tests pass in `prepare-tests.log`.
Strict server Clippy passes in `clippy.log`.
The final local tests, five independent edge tests, and retry process/security test pass.
No desktop sound was played. Every test browser used a fresh profile and null audio.

Wrappers: `target/mc-evidence/compression-cold-checks/receipt.json` and `target/mc-evidence/compression-launcher-check/receipt.json`.
Both commands and wrappers exit 0. Author source stays unchanged during each run.
The cold checker reports PARTIAL-PASS because its older manual and reference stages were not repeated.
All scripted probes pass.

## Independent review and fixes

One independent reviewer ran the HTTP code and native server. The review passes at `113e86f0`.
The reviewer found two issues. File errors could label plain HTML as gzip. The launcher predicate could accept zero models after a timeout.
Both fixes are independently verified. The edge cases remain in `tests/qa/test_web_compression_edges.py`.
See `docs/reports/2026-11-14-browser-compression-independent-review.md`.

Cold probes ran at `9b889655`. The two later fixes affect the local-host error path and a test guard.
The native HTTP server used by those probes, the browser payloads, and game content remain byte-identical at the final head.
The final real-launcher run tests the fixed local host at `113e86f0`.
No repeated GPU benchmark was needed: the complete original dist and renderer match the measured minimap build.

## Limits and next work

This proves smaller HTTP transfers. It does not measure a startup-time improvement on another network or machine.
The browser recorder attaches and disables its cache, then reloads the same fresh test profile to capture the transfer.
No new human playtest or physical sound test was done.

Run `/home/julien/Projects/TuRTS/target/mc-evidence/play-skirmish.sh`.
Models and map controls must load with sound off.

Next: run full normal-stat win and loss matches. Earlier quick combat probes used a 30-HP castle.
P4 environment work and broader MP1 features remain open.
