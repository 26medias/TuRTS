# Shared presentation payloads and borrowed cursor state

Status: delivered on the integration branch. All PS1–PS5 rows pass.

Accepted model, image, font, and sound payloads now use shared immutable byte storage.
Presentation metadata clones retain those bytes instead of deep-copying the payloads.
Cursor selection borrows the current presentation, catalog, replica, interpolation, and selection.
It caches only ECS query metadata. It reads current camera and window values every update.
Equal cursor icons no longer trigger repeated platform updates.

A decoder that requires an owned buffer still receives one at asset load.
Bevy's memory reader gets its Vec once per model. AudioSource shares the accepted sound bytes.
The renderer, art, unit detail, animation, gameplay, and data schemas stay unchanged.
Public API 0.2.11 and protocol 6 stay unchanged.

Implementation: `1efd54f1`. Retained independent regression: `ec91bcf4`.
Spec: `docs/superpowers/specs/2026-11-14-presentation-payload-sharing.md`.
Evidence root: `/home/julien/Projects/TuRTS/target/mc-evidence/cursor-borrows/`.

## Executed checks

The combined client/web suite passes 684 checks, with five existing ignored tests.
Evidence: `shared-native-tests-complete.log`.
The accepted-byte test reads real Skirmish and Muster fixtures.
It checks qualified IDs, exact archive bytes, shared ownership, independent metadata, and hashes after original-owner drops.
Five retained cursor tests pass, including camera motion and camera/window replacement after query initialization.
Evidence: `retained-camera-tests.log`.
Strict native client/web Clippy passes for all targets: `final-clippy.log`.

The preview renders real combat: bolt 153 pixels, sparks 42, and restored team colour.
High/Low/High controls, model readiness, map bounds, and outside construction pass.
Evidence: `shared-preview-pixels-checked/result.json` and `shared-preview-quality/result.json`.
The first effects-probe setup lacks the unpacked package seed. The private diagnostic copy now contains it.
The accepted global ground build was not modified.

## Independent review

One independent review runs code in an isolated checkout from `1efd54f1`.
Its native result is 684 passes and five ignored tests, plus one independent camera regression.
Both real GPU sanity probes pass. It finds no actionable production issue.
Report: `review/review.md`. The independent test is retained and passes in the author checkout.
No second review pass runs.

## Performance evidence and limits

Preview samples do not show a speed gain. They are not release comparisons.
The previous preview reaches 30.45 FPS High at 2,000 units.
Cursor borrowing alone reaches 29.16 FPS. Shared payloads plus borrowing reach 28.45 FPS.
All actors stay visible and take damage.
The ordinary release supplies the final performance table below.
Evidence: `preview-2000/`, `shared-preview-2000/`, and `../crowd-targets/preview-baseline/`.

The earlier animation-target membership experiment remains parked on `codex/crowd-cpu` at `c53a8516`.
Its matched preview slows down. It is absent from this build.
The accepted game package, server, and packer remain byte-identical: `native-package-reuse.json`.
No quality, effect-count, animation-cadence, acceptance-threshold, or gameplay reduction is made.

Cleanup removes 1.74 GiB of finished server/package test executables and 0.18 GiB of old incremental cache.
Source patches, hashes, and the deleted-file list remain in `finished-cache-cleanup/`.
Active dependencies, normal server/packer binaries, and all `target/mc-evidence` remain.

No new human playtest, full normal-stat match, shared-human match, or other-machine measurement runs for this change.
Main is unchanged. No branch is pushed.

## Final acceptance

| Row | Result and evidence under the evidence root |
| --- | --- |
| PS1 | PASS: `shared-native-tests-complete.log`; real archive ownership and byte tests. |
| PS2 | PASS: `shared-native-tests-complete.log`, `retained-camera-tests.log`, `final-clippy.log`. |
| PS3 | PASS: `final/pixels/result.json`, `final/quality/result.json`, `final/player/cold-1024/probe-first-click.json`, `final/player/cold-1280/probe-first-click.json`, `final/player/cold-buttons/probe-buttons.json`. |
| PS4 | PASS: `final/battle400/result.json`, `final/battle2000/result.json`, `final/pan/result.json`. |
| PS5 | PASS: `review/review.md`, `final/launcher/result.json`. |

Release source: `ec91bcf4`. RTX 3080, i9-14900K, Linux, Chrome WebGPU, 1280×720.
The source stays unchanged during the final checks. The wrapper exits 0.
Receipt: `../payload-sharing-final-checks/receipt.json`.

| Scene | Quality | Mean FPS | p50 ms | p95 ms | WASM high-water MiB |
| --- | --- | --- | --- | --- | --- |
| battle400 | High | 58.70 | 16.70 | 16.80 | 193.2 |
| battle400 | Low | 59.40 | 16.70 | 16.80 | 172.1 |
| battle2000 | High | 33.15 | 33.30 | 50.00 | 388.8 |
| battle2000 | Low | 39.14 | 16.80 | 33.40 | 348.7 |
| pan | High | 59.70 | 16.70 | 16.70 | 193.9 |
| pan | Low | 59.40 | 16.70 | 16.80 | 172.8 |

All measured actors stay visible and take damage. Shared GPU skin palettes stay enabled.
The ground-blending baseline records 31.00 FPS High and 38.24 FPS Low at 2,000 units.
These are single release samples. They do not establish a repeatable percentage gain.
High p95 remains 50 ms at 2,000 units. Retained WASM memory does not materially decrease.
The earlier preview samples do not show a gain.

The actual single-player launcher passes. It serves gzip WASM and removes its fresh test profile.
Raw WASM: 60,959,792 bytes. Gzip: 15,091,809 bytes. Observed encoded transfer: 15,092,054 bytes.
All three global launchers point to `skirmish-payload-sharing-prepared`.
The shared-human and two-player launcher paths are updated but not rerun for this client-only change.
No new normal-stat win/loss match, human playtest, other-machine measurement, or detailed-model benchmark runs.
No spec item changes. No main merge or push.

To play: run `/home/julien/Projects/TuRTS/target/mc-evidence/play-skirmish.sh`; build, harvest, and defeat the enemy castle.
