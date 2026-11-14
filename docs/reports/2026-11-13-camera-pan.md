# Camera panning delivery

Engine: `f5d7d316`. Integration branch: `codex/p2-p3-presentation`.
Main was not merged or pushed.

The skirmish pans quickly enough to follow units with the Arrow keys.
Edge scrolling pauses while the pointer covers a unit or resource.
Open edges still scroll. Explicit keyboard panning works over a target.
The camera uses the same visible-body picking as selection.

A real 300 ms Arrow-key press moved the view by 10.7 map units, or 154 pixels.
Space then returned to the selected unit.
The first attempt to raise pan speed during P3 was reverted after a failed edge click.
This separate fix adds the missing protection and passes that same 1024-pixel probe.

## Executed checks

Paths below are relative to `target/mc-evidence/`.

| Check | Result | Evidence |
| --- | --- | --- |
| Edge unit and resource regression | The old code failed both checks. The fixed code passes. | `camera-pan-native-red.log`; `camera-pan-native-green.log` |
| Interaction suite | 143 passed | `camera-pan-interaction-full.log` |
| Client suite | 632 passed; five existing tests skipped | `camera-pan-client-full.log` |
| Strict Clippy | Passed | `camera-pan-clippy.log` |
| Browser Arrow pan and Space | Passed; 154 pixel movement | `camera-pan-browser/result.json` |
| First clicks at 1280 and 1024 | Each passed three selections, six orders, and four extra clicks | `camera-pan-cold-1280/probe-first-click.json`; `camera-pan-cold-1024/probe-first-click.json` |
| Buttons probe | Passed | `camera-pan-cold-buttons/probe-buttons.json` |
| Browser batch | Command/wrapper 0; source unchanged; no compiler | `camera-pan-final-checks/receipt.json` |
| Actual launcher | Passed; models loaded; sound off; fresh profile removed | `camera-pan-launcher/result.json`; `camera-pan-launcher-run/receipt.json` |

The cold checker has zero failures. Its label is PARTIAL-PASS because the older manual
agent and reference stages were not repeated. No new human playtest was done.
Every test Chrome used a fresh profile and `--alsa-output-device=null`.

Removed two obsolete alert-no-op assertions from stub tests. Real alert tests remain.

## Performance

All 400 soldiers remained visible and shooting throughout both samples.

| Quality | FPS | Frame p50 ms | Frame p95 ms |
| --- | ---: | ---: | ---: |
| High | 43.33 | 16.70 | 50.00 |
| Low | 51.62 | 16.70 | 33.40 |

Raw intervals, memory, and particle counts: `camera-pan-400-guard/result.json`.
The renderer and particle counts are unchanged. The test pointer is in the viewport center.
This is not a measurement of the cost of picking over a crowded edge.

## Play

Run `/home/julien/Projects/TuRTS/target/mc-evidence/play-skirmish.sh`.
Use the Arrow keys to pan, or move the pointer to an open screen edge.
Hover a unit at the edge and click it. The unit must stay under the pointer.
