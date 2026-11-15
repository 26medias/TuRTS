# Share immutable presentation payloads

## Result

Client frame snapshots must not copy model, image, font, and sound payloads.
The accepted package bytes stay immutable and can be shared by client presentation consumers.
Cursor selection reads the current state directly. It does not copy the accepted presentation or replica.
Cursor meaning, images, hotspots, lock behavior, orders, art, detail, quality, and animation stay unchanged.

## Contract

Asset IDs and bytes after verified package acquisition remain identical.
Cloning presentation metadata shares payload storage. Editing cloned metadata cannot change the original metadata.
Dropping an archive or metadata clone cannot invalidate payloads still used by another consumer.
Decoders and quality scaling retain their own derived buffers. They cannot modify the accepted bytes.
A boundary that needs an owned Vec can copy once when loading an asset. No frame consumer should deep-copy payloads.
Each cursor update reads current selection, replica, interpolated positions, camera, and window state.
Cached ECS query metadata must discover replacement windows and newly created archetypes.
Equal native cursor icons need no repeated platform update.
No server, mod data schema, package, wire, gameplay, cadence, or detail changes.
API 0.2.11 and protocol 6 remain unchanged.

## Acceptance

| Row | Check |
| --- | --- |
| PS1 | Real accepted packages retain qualified IDs and exact payload bytes. Metadata clones share byte ownership, survive owner drops, and retain independent metadata. |
| PS2 | Native cursor tests pass authored hotspot variants, lock/UI/device scaling, current state changes, and a replacement window. Image/font/model/sound loading and quality tests pass. |
| PS3 | Real browser models, effects, and High/Low/High quality restoration pass. Final first-click probes at both sizes and the buttons probe pass. |
| PS4 | Measure the ordinary release at 400 and 2000 fighting units on High and Low and a 400-unit pan. Record p50, p95, memory, and GPU. Keep all actors visible and damaged. High stays at least 30 FPS at 400 units. |
| PS5 | One independent review runs code after the build. Fix its findings. The actual global launcher passes. |

Use the accepted ground-blending build as the release baseline.
Record preview/profile timings as diagnostic results. Do not claim a speed gain from them.
Keep the current launcher until this feature passes. Do not merge or push main.
Every test browser uses a fresh profile and --alsa-output-device=null.
