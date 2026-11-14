# Stage 1 model measurement

Date: 2026-11-13. Source engine: `617a0f8b3d73955677ef4cfb9778fa285889ef74`.
The measurement ran before the skirmish content was finished.

Machine: Beast, Intel(R) Core(TM) i9-14900K, 125.5 GiB RAM.
OS: linux 7.0.0-42-generic.
Browser: Chrome/157.0.8103.38. Viewport: 1280 by 720.
Adapter: ANGLE (NVIDIA, Vulkan 1.4.312 (NVIDIA NVIDIA GeForce RTX 3080 (0x00002206)), NVIDIA-580.178.4.0).
WebGPU was enabled. The adapter reported no software fallback.
The browser used a fresh profile and `--alsa-output-device=null`.

All model scenes used `high`: PBR textures, one directional shadow cascade at 2048 px,
and the engine LOD selector. The plane stayed plain in each scene to isolate model cost.
All units were visible and idle. No computer player ran.
The detailed fixture has exactly 20,000 triangles and three embedded 2048 px PNG maps:
base colour, normal, and metallic-roughness. LOD 1 has 800 triangles. LOD 2 has 128 triangles.
The skirmish soldier used scale 0.88 to pass the footprint fit check.

| Scene | Units | Frame p50, ms | Frame p95, ms | Mean FPS | Wasm high-water, MiB | Drawn LOD counts |
| --- | ---: | ---: | ---: | ---: | ---: | --- |
| shapes | 400 | 16.70 | 16.80 | 59.00 | 74.6 | {} |
| lowpoly | 400 | 16.70 | 16.80 | 58.80 | 100.2 | {'0': 400} |
| detail | 400 | 16.70 | 16.70 | 58.90 | 334.1 | {'0': 400} |
| detail_lod | 400 | 16.70 | 16.70 | 58.80 | 334.4 | {'2': 400} |
| detail_lod_2000 | 2000 | 16.70 | 16.80 | 58.11 | 339.0 | {'2': 2000} |

Each row followed a five-second warm-up. A ten-second `requestAnimationFrame` sample
measured presentation intervals. This is browser presentation time, not an isolated GPU timer.
The display limits the result to about 60 FPS. These numbers show that the tested scenes
fit that limit. They do not measure the GPU's maximum throughput.
Wasm memory never shrinks. The reported linear-memory size is its high-water mark.
Draw-call counts and GPU memory were not available through this driver. They are not estimated.
The renderer shares mesh handles and one body material per source material. Team materials
are shared for each slot and quality setting.

Both detailed scenes at 400 units stayed above 30 FPS. The stop rule did not apply.
LOD reduced geometry. It did not reduce the measured memory high-water. Stage 1 still loads
all authored LOD files and their embedded textures. Texture streaming and crowd animation
remain later work. This measurement does not claim either capability.

Evidence:

- `/home/julien/Projects/TuRTS/target/mc-evidence/models-measurement/result.json`: raw intervals,
  browser version, adapter data, memory, and every row.
- The same folder: one screenshot and package/server logs for each scene.
- `/home/julien/Projects/TuRTS/target/mc-evidence/codex-models-measurement-3/receipt.json`:
  command exit 0, wrapper exit 0, source unchanged.
- `scripts/qa/models/generate_benchmark.py`: reproducible test geometry and packages.
- `/home/julien/Projects/TuRTS/target/mc-evidence/models-measurement/driver.js`: browser driver.

Two setup attempts ended before a complete measurement. The first fixture lacked its
construction-site limit. The second rejected the supplied soldier at scale 1.0.
The final run used the valid fixture and scale 0.88. No failed frame sample was discarded.
The detailed models are test assets. They are not part of `mods/skirmish`.

## Stage P2: animated crowds

Date: 2026-11-13. Tested game source: `1a6febb7`. Same machine and browser as stage 1.
The client uses shared bone poses for matching clip phases and blend weights.
Each unit keeps its own movement clock, world transform, and skin.
High quality updates every visible unit. The reference phase buckets are 1/30 s wide.

| Scene | Initial units | Frame p50, ms | Frame p95, ms | Mean FPS | Wasm high-water, MiB |
| --- | ---: | ---: | ---: | ---: | ---: |
| All walking | 200 | 16.70 | 16.80 | 57.81 | 148.9 |
| All walking | 400 | 33.30 | 33.40 | 39.07 | 180.4 |
| Mixed movement and idle; 20 scripted removals each second | 400 | 16.70 | 33.40 | 41.43 | 178.6 |
| All walking | 1000 | 50.00 | 66.70 | 17.81 | 278.1 |

All walking scenes kept every unit moving and visible throughout the samples.
The mixed scene starts with 200 moving and 200 idle units. The fixed-tick server script
removes one idle unit each tick for 200 ticks. Those real committed destruction events
exercise the death presentation at 20 deaths each second. Bodies peak at the required limit of 64.
These removals isolate presentation cost. They do not measure battle AI or targeting.

The fixture soldiers move at 0.4 map units each second to keep walking through the sample.
They use the supplied full skinned Crossbowman, with no LOD. The map stays plain.
After five seconds of animated warm-up, each row records ten seconds of RAF presentation intervals.
No compiler ran during the final timing run. All rows use high quality and the real WebGPU adapter.
Every scene uses a fresh Chrome profile and `--alsa-output-device=null`.

The 400-unit walking and mixed scenes exceed 30 FPS. The 1,000-unit stress row is 17.81 FPS.
The 1,000-unit row is recorded, not accepted as a smooth gameplay target.
No isolated frame-work timer, draw-call counter, or GPU-memory counter was available in this harness.

The first method ran each skeleton independently. Its 400-unit sample reached 21.13 FPS.
That run also had native compilers present. It is not an isolated comparison of algorithms.
An intermediate shared-pose sample reached 37.08 FPS, but 31 units stopped at occupied destinations.
That fixture failed its movement check. The final fixture puts goals beyond the whole formation.

Evidence under `target/mc-evidence/`:
- `p2-final-measurement/result.json`: raw frame intervals, adapter, memory, visible counts, and walking counts.
- `codex-p2-final-measure/receipt.json`: completed; command 0, wrapper 0, source unchanged.
- `p2-measurement/result.json`: initial cost failure.
- `p2-shared-measurement/result.json`: rejected intermediate movement fixture.
- `p2-stride.json`: stride measured from 241 samples of each walking clip.
- `scripts/qa/animation/measure_stride.py`: the stride measurement tool.


## P3 initial combat measurement: performance stop

2026-11-13. Engine `6d5a92a0`. This development build is incomplete.
The launcher still opens the accepted P2 build.

RTX 3080, i9-14900K, Chrome 157, WebGPU, 1280×720, High quality.
Fresh browser profile and null ALSA output for each row. No compiler ran during sampling.
Five seconds of shooting warm-up, then ten seconds of RAF intervals.
The fixture uses high health and longer weapon range to sustain shooting.
These are render workloads, not gameplay balance tests.

| Scene | Quality | FPS | Frame p50 ms | Frame p95 ms | Peak sampled particles | WASM high-water MiB |
| --- | --- | ---: | ---: | ---: | ---: | ---: |
| 100 against 100 | High | 50.12 | 16.70 | 33.40 | 1,800 | 156.4 |
| 200 against 200 | High | **26.67** | 33.30 | 83.30 | 3,200 | 196.1 |

All 400 soldiers stayed visible and in their ranged attack state throughout the second row.
The initial run stopped at the CEO's 30 FPS limit. The CEO then authorized automatic fixes.
Low quality and 400 against 400 were not measured.

The attempted burning-building row is **rejected**. It measured 51.02 FPS, but six of ten
buildings had died. Only four remained at 95/500 health. Repair the fixture before reuse.
The P3 section 6 table and FX13 remain incomplete.

Evidence under `target/mc-evidence/`:

- `p3-first-measurement/result.json`: raw intervals, unit counts, effects, adapter, and memory.
- `p3-first-effects-measure/receipt.json`: exit 3 at the performance limit; source unchanged;
  no compiler at start.
- `p3-first-measurement/battle400-high.png`: the 400-unit scene.
- `skirmish-effects-first-prepared/manifest.json`: build inputs and artifact hashes.

Drivers: `scripts/qa/effects/generate_benchmark.py` and `tests/browser/effects-measure.js`.


### P3 performance recovery

The CEO resumed work and directed Codex to fix future performance failures without waiting for approval.

The CPU profile identifies skin extraction, shadow/material specialization, UI snapshots,
and scene observation as the largest costs. The named profiling copy uses a different Binaryen
version and retains function names. Its timing is diagnostic only.

Engine commit `14405dff` changes Rust and Binaryen optimization from size to speed.
The same High 400-unit fixture now measures **36.84 FPS**, p50 **16.70 ms**, p95 **50.00 ms**.
All 400 soldiers remain visible and shooting. No model, effect-count, or quality reduction was made.
This is a 38% FPS increase over the initial 26.67 FPS sample.

The tradeoff is a larger download. WASM grows from 29,683,233 to 59,363,833 bytes.
At gzip level 6, it grows from 9,442,563 to 14,746,429 bytes.
The gzip values describe compressibility. They do not claim that the server serves gzip.

Evidence under `target/mc-evidence/`:

- `p3-combat-cpu-profile/`: profile of the unchanged failing artifact.
- `p3-combat-named-profile/`: named CPU profile and inclusive system costs.
- `p3-combat-speed-measure/result.json`: unprofiled speed-build sample.
- `p3-combat-speed-measure/receipt.json`: exit 0, source unchanged, no compiler at start.
- `p3-build-size-comparison.json`: raw and gzip byte counts.

Further work removes unnecessary UI scans and unchanged render-state writes.
The full P3 High/Low table and final player acceptance are still incomplete.

## P3 final combat measurement

2026-11-13. Engine `f6b686e1`. All eight workload rows passed.
The runtime HUD-only follow-up moves warnings below credits.
The attempted camera speed change was reverted after the 1024-pixel probe failed.
Compiled engine inputs are unchanged by that follow-up.

RTX 3080, i9-14900K, Chrome 157, WebGPU, 1280x720.
Each row uses a fresh silent browser. No compiler ran during this table.
Five seconds of sustained shooting warm-up precede ten seconds of RAF samples.
The fixtures raise health and range to keep every soldier shooting.
The burning fixture keeps ten damaged buildings alive throughout the sample.

| Scene | Quality | FPS | Frame p50 ms | Frame p95 ms | Live particles, max sampled | WASM high-water MiB | Drops Small / Medium / Large |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- |
| 100 against 100 | High | 57.51 | 16.70 | 16.80 | 1,800 | 156.1 | 0 / 0 / 0 |
| 100 against 100, 10 burning buildings | High | 58.11 | 16.70 | 16.80 | 1,909 | 154.9 | 0 / 0 / 0 |
| 200 against 200 | High | 44.03 | 16.70 | 50.00 | 3,600 | 196.3 | 0 / 0 / 0 |
| 400 against 400 | High | 22.25 | 33.40 | 100.00 | 7,200 | 276.4 | 0 / 0 / 0 |
| 100 against 100 | Low | 59.20 | 16.70 | 16.80 | 400 | 141.1 | 2016 / 0 / 0 |
| 100 against 100, 10 burning buildings | Low | 59.50 | 16.70 | 16.80 | 416 | 140.7 | 1514 / 3 / 0 |
| 200 against 200 | Low | 50.32 | 16.70 | 33.40 | 512 | 170.3 | 12951 / 0 / 0 |
| 400 against 400 | Low | 35.48 | 33.30 | 50.00 | 512 | 227.3 | 39666 / 0 / 0 |

The High 400-unit result is **44.03 FPS**, compared with the original **26.67 FPS**.
The Low 400-unit result is **50.32 FPS**.
The High 800-unit result is **22.25 FPS**. That workload remains below 30 FPS.
Low uses its fixed 512-particle pool. Informational lines and hit marks remain when effects are dropped.
Drop counts are cumulative totals at the last sampled frame. They include warm-up.
Live particle values are sampled peaks, not the pool's lifetime peak counter.

Evidence under `target/mc-evidence/`:

- `p3-contract-final-table/result.json`: raw frame intervals, adapter, memory, shooting counts, and per-cost drops.
- `p3-contract-final-table/receipt.json`: command/wrapper 0, source unchanged, no compiler at start.
- `p3-final-hud-reuse.json`: compiled-input equality and inspection of the rebuilt runtime package.
- `skirmish-effects-final-prepared/manifest.json`: current build and package hashes.

The earlier FX3 pixel PASS was rejected because it counted attack markers.
The final acceptance uses a marker-free fixed-tick attack and a temporal spark mask.

### Final terminal-fix performance guard

Engine `85658237`. The terminal-only fix does not run in the sustained workload fixtures,
which disable victory. The full eight-row table above records engine `f6b686e1`.
The final High/Low 400-unit guard checks the complete delivered engine again.

| Scene | Quality | FPS | Frame p50 ms | Frame p95 ms | Live particles, max sampled | WASM high-water MiB |
| --- | --- | ---: | ---: | ---: | ---: | ---: |
| 200 against 200 | High | 41.43 | 16.70 | 50.00 | 3600 | 196.3 |
| 200 against 200 | Low | 54.71 | 16.70 | 33.30 | 512 | 170.4 |

All 400 soldiers remained visible and shooting in every sampled state.
Receipt: `target/mc-evidence/p3-terminal-400-guard/receipt.json`.
Raw intervals and states: `target/mc-evidence/p3-terminal-400-guard/result.json`.
Command/wrapper 0. Source unchanged. No compiler at start.

### Camera follow-up guard

Engine `f5d7d316` adds responsive panning and protects edge-target clicks.
Both 400-unit samples pass. Every soldier stayed visible and shooting.

| Quality | FPS | Frame p50 ms | Frame p95 ms |
| --- | ---: | ---: | ---: |
| High | 43.33 | 16.70 | 50.00 |
| Low | 51.62 | 16.70 | 33.40 |

Evidence: `target/mc-evidence/camera-pan-400-guard/result.json` and
`target/mc-evidence/camera-pan-final-checks/receipt.json`.
No compiler ran. The source stayed unchanged.

## Swordsman mixed combat, 2026-11-14

Engine source: `d2214625`. The later launcher head changes browser drivers only.
The build source, packed content, and binary hashes remain unchanged at `e917050f`.
Machine: local RTX 3080, i9-14900K, Linux, Chrome WebGPU, 1280x720.
Fresh profile and `--alsa-output-device=null` for each run. No compilers ran during the samples.

The scene has 200 Swordsmen and 200 Crossbowmen in opposing pairs.
It uses shipped weapon ranges and cooldowns. Health is raised to sustain combat.
All 400 models stay visible. All 400 units lose health during each ten-second timing window.
Both attack types are observed. A Swordsman correctly returns to idle between strikes.
The test checks sustained damage instead of requiring a continuous melee attack clip.
The camera is pulled back to include the whole formation. The pointer is in the viewport center.
This is a render/combat workload, not a measurement of moving a 400-unit army.

| Quality | FPS | Frame p50 ms | Frame p95 ms | WASM high-water bytes |
| --- | ---: | ---: | ---: | ---: |
| High | 45.23 | 16.70 | 33.40 | 217710592 |
| Low | 53.31 | 16.70 | 33.40 | 190447616 |

Evidence: `target/mc-evidence/swordsman/400-guard-verified/result.json`.
The raw frame intervals, state samples, screenshots, and GPU metadata are retained there.
Wrapper: `target/mc-evidence/swordsman-verified-checks/receipt.json`.
Its later player-test assertion failed, after both measurement rows passed.
The final player run is separate. The earlier `400-guard` run is not an accepted measurement.
Its old continuous-attack assertion was incorrect for melee.

## Minimap, 2026-11-14

Engine: `85873b77`; driver-only follow-up: `5a40481d`.
Same 400-unit mixed combat protocol and machine as the swordsman row above.
All 400 actors remain visible. All 400 lose health in both timing windows.
The authored minimap and its live unit markers are enabled.
Stationary markers do not force another UI layout each frame.

| Quality | FPS | Frame p50 ms | Frame p95 ms | WASM high-water bytes |
| --- | ---: | ---: | ---: | ---: |
| High | 45.43 | 16.70 | 33.40 | 221577216 |
| Low | 52.32 | 16.70 | 33.30 | 194052096 |

Evidence: `target/mc-evidence/minimap/400-guard/result.json`.
Receipt: `target/mc-evidence/minimap-browser-checks/receipt.json` (command and wrapper 0; unchanged source; no compilers).
The sample measures sustained combat with a stationary camera. It does not measure panning a 400-unit scene.


## Final lobby client, 2026-11-14

Client source: `e6319f1d`. Prepared server source: `0dcd05b2`.
Only the native computer policy differs between the two prepared builds.
The browser, packer, and package hashes match exactly.
The benchmark disables the computer player.
Same machine and 400-unit mixed combat workload as the minimap row above.
All 400 actors remain visible. All 400 take damage in each timing window.
No compiler or other test browser runs during the samples.
Every test browser has a fresh profile and --alsa-output-device=null.

| Quality | FPS | Frame p50 ms | Frame p95 ms | WASM high-water bytes |
| --- | ---: | ---: | ---: | ---: |
| High | 46.42 | 16.70 | 33.40 | 221249536 |
| Low | 56.91 | 16.70 | 33.30 | 193986560 |

Evidence: `target/mc-evidence/mixed-opponent/400-guard/result.json`.
Reuse proof: `target/mc-evidence/mixed-opponent/artifact-reuse.json`.
The sample measures sustained combat with a stationary camera.
No new 2,000-unit row or other-hardware performance claim is made.

## Environment stage 1, 2026-11-14

Source for these samples: `e95e628a`. Final validation and HUD follow-up: `cecc9a96`.
Machine: RTX 3080, Intel Core i9-14900K, Linux 7.0.0-42-generic, Chrome WebGPU, 1280x720.
Each browser uses a fresh profile and `--alsa-output-device=null`. No compiler or other test browser ran during samples.
The generated maps preserve the shipped grass, environment, and ground margin.
All actors stay visible and take damage. The workload disables the computer player.
The measurement driver reads the scene each animation frame. This includes test overhead.

| Units | Camera | Quality | FPS | Frame p50 ms | Frame p95 ms | WASM high-water bytes |
| ---: | --- | --- | ---: | ---: | ---: | ---: |
| 400 | stationary | High | 42.13 | 16.70 | 33.40 | 241696768 |
| 400 | stationary | Low | 49.82 | 16.70 | 33.40 | 209518592 |
| 2000 | stationary | High | 5.08 | 200.00 | 250.00 | 589889536 |
| 2000 | stationary | Low | 12.06 | 83.30 | 116.70 | 546701312 |
| 400 | pan | High | 41.86 | 16.70 | 33.40 | 241762304 |
| 400 | pan | Low | 50.32 | 16.70 | 33.30 | 209715200 |

Evidence: `target/mc-evidence/p4-environment/{400-measure,2000-measure,400-pan}/result.json`.
Each result retains raw frame intervals, state samples, browser details, and GPU adapter data.
Chrome reports a hardware adapter and an empty adapter name. The local GPU inventory identifies the RTX 3080.
The pan test sends real Arrow keys and checks distinct camera matrices.
The 400-unit High row passes 30 FPS. The 2,000-unit scene needs more performance work.
These samples do not prove performance on another GPU.

Final build `cecc9a96`, refreshed fixtures from the shipped package, 400-unit pan:

| Quality | FPS | Frame p50 ms | Frame p95 ms | WASM high-water bytes |
| --- | ---: | ---: | ---: | ---: |
| High | 42.23 | 16.70 | 33.40 | 242286592 |
| Low | 49.02 | 16.70 | 33.40 | 209846272 |

Evidence: `target/mc-evidence/p4-environment/final-400-pan/result.json`. All 400 remain visible and take damage.

## Large-army rendering, 2026-11-14

Engine: `a608d9e0`. Final browser driver correction: `325c5386`.
Same RTX 3080, i9-14900K, Linux, Chrome WebGPU, and 1280x720 viewport as the environment measurements.
Every test browser uses a fresh profile and `--alsa-output-device=null`. No compiler or other test browser runs during samples.
All required quality features stay enabled. Low retains its existing cheaper path.
The sustained mixed battle keeps every actor visible and checks damage on every actor.
The default timing protocol now reads scene state every 500 ms. It records every RAF interval.
Each row records `scene_sampling_interval_ms`. The earlier environment rows read scene state every RAF.

| Units | Camera | Quality | FPS | Frame p50 ms | Frame p95 ms | WASM high-water bytes |
| ---: | --- | --- | ---: | ---: | ---: | ---: |
| 400 | stationary | High | 59.40 | 16.70 | 16.80 | 208601088 |
| 400 | stationary | Low | 59.50 | 16.70 | 16.80 | 185532416 |
| 2000 | stationary | High | 21.49 | 50.00 | 66.70 | 452853760 |
| 2000 | stationary | Low | 28.05 | 33.30 | 50.00 | 414318592 |
| 400 | pan | High | 59.70 | 16.70 | 16.70 | 208732160 |
| 400 | pan | Low | 59.40 | 16.70 | 16.70 | 185663488 |

Evidence: `target/mc-evidence/large-army-performance/{final-400,final-2000,final-400-pan}/result.json`.
The pan row sends actual Arrow inputs and checks distinct camera matrices.
The 2,000-unit battle remains below 30 FPS. It is a stress result, not a smooth-gameplay claim.

Paired High comparisons use the same 2,000-unit fixture and server:

| Scene-read protocol | Previous environment build | Optimized build | Evidence |
| --- | ---: | ---: | --- |
| Every 500 ms | 4.24 FPS | 21.49 FPS | `baseline-2000-new-poll/`, `final-2000/` |
| Every RAF | 5.08 FPS | 19.40 FPS | Previous `p4-environment/2000-measure/`, current `final-2000-legacy-poll/` |

The 500 ms comparison improves High by 5.06 times. Both builds use that protocol in this comparison.
The legacy row compares separate runs. All raw intervals remain available.

The original static model workload was also rerun on the final release.
The test model still has 20,000 triangles and 2048 px base-colour, normal, and metallic-roughness maps.
High uses the same quality settings and 2048 px shadow map. The ground stays plain to isolate model cost.
The models are idle. This is different from the sustained animated battle above.

| Scene | Units | FPS | Frame p50 ms | Frame p95 ms | WASM high-water bytes | LOD counts |
| --- | ---: | ---: | ---: | ---: | ---: | --- |
| shapes | 400 | 59.30 | 16.70 | 16.80 | 79429632 | {} |
| lowpoly | 400 | 58.90 | 16.70 | 16.80 | 106627072 | {'0': 400} |
| detail | 400 | 58.61 | 16.70 | 16.80 | 350814208 | {'0': 400} |
| detail_lod | 400 | 59.30 | 16.70 | 16.70 | 418185216 | {'2': 400} |
| detail_lod_2000 | 2000 | 58.90 | 16.70 | 16.80 | 426311680 | {'2': 2000} |

Evidence: `target/mc-evidence/large-army-performance/final-models/result.json`.
The copied driver and isolated fixture copies remain in the same evidence folder. The original evidence was not modified.
Both 400-unit detailed scenes pass 30 FPS. LOD 2 is actually selected in the LOD scenes.
The browser and GPU details, screenshots, raw intervals, and linear-memory samples are retained.
Draw-call counts and GPU-memory counters remain unavailable. `draw_meshes` counts entities, not draw calls.
The display limits these runs to about 60 FPS. The rows do not measure maximum GPU throughput.

## Rigid attachment batches: before the hit-origin fix

Date: 2026-11-14. Source: `ed8b563e`, with rotation guards from `207e7af4`.
Same machine, release profile, GPU, viewport, mixed fighting fixtures, and 500 ms scene-read protocol as the previous release.
All actors stay visible and take damage. No quality setting or animation cadence is reduced.

| Scene | Units | Quality | Frame p50, ms | Frame p95, ms | Mean FPS | Wasm high-water, MiB |
| --- | ---: | --- | ---: | ---: | ---: | ---: |
| Mixed fight | 400 | High | 16.70 | 16.70 | 59.30 | 195.6 |
| Mixed fight | 400 | Low | 16.70 | 16.80 | 59.60 | 175.0 |
| Mixed fight | 2000 | High | 33.30 | 50.10 | 27.26 | 406.1 |
| Mixed fight | 2000 | Low | 33.30 | 50.00 | 31.95 | 367.7 |
| Mixed fight and camera pan | 400 | High | 16.70 | 16.70 | 59.70 | 196.4 |
| Mixed fight and camera pan | 400 | Low | 16.70 | 16.70 | 59.50 | 175.1 |

The paired 2,000-unit High result increases from 21.49 to 27.26 FPS. Low increases from 28.05 to 31.95 FPS.
High at 2,000 units remains below 30 FPS. The 400-unit High threshold passes.
Browser presentation is capped near 60 FPS. Draw calls and GPU memory remain unavailable.
The `draw_meshes` observation counts entities, not draw calls.
Evidence: `target/mc-evidence/stable-model-transforms/final-battle400/result.json`,
`final-battle2000/result.json`, and `final-pan/result.json`.
Every browser uses a fresh profile and `--alsa-output-device=null`.
No compiler or other test browser runs during these timings.

## Rigid batches with visible hit sparks: final build

Date: 2026-11-14. Final content: `68f9d456`. Compiled engine: `ed8b563e`.
The client is byte-identical to the rig-only release. The package raises the authored spark origin by 0.65 map units.
This keeps sparks outside the armour. Particle size, count, lifetime, and quality settings stay unchanged.
The final fixtures include this content. Every actor stays visible and takes damage.

| Scene | Units | Quality | Frame p50, ms | Frame p95, ms | Mean FPS | Wasm high-water, MiB |
| --- | ---: | --- | ---: | ---: | ---: | ---: |
| Mixed fight | 400 | High | 16.70 | 16.70 | 59.50 | 196.0 |
| Mixed fight | 400 | Low | 16.70 | 16.80 | 59.60 | 174.8 |
| Mixed fight | 2000 | High | 33.40 | 50.10 | 25.12 | 405.5 |
| Mixed fight | 2000 | Low | 33.30 | 50.00 | 31.85 | 367.7 |
| Mixed fight and camera pan | 400 | High | 16.70 | 16.70 | 59.60 | 196.1 |
| Mixed fight and camera pan | 400 | Low | 16.70 | 16.70 | 59.50 | 175.1 |

Same RTX 3080, release profile, browser, viewport, five-second warm-up, ten-second sample, and 500 ms scene-read protocol as above.
High at 2,000 units remains below 30 FPS. The 400-unit High threshold passes.
The previous delivered package reaches 21.49 FPS High and 28.05 FPS Low at 2,000 units.
That package has the old spark origin. The rig-only comparison above isolates the engine change more closely.
This final comparison includes the improved visible impact effect. No effect quality is reduced.
Evidence: `target/mc-evidence/stable-model-transforms/visible-hit-final/`, with `result.json` in each row folder.
Receipt: `target/mc-evidence/rig-visible-final-checks/receipt.json`; command 0, wrapper 0, source unchanged.
All three final cold probes also pass in that batch. The checker retains open manual checks.
No compiler or other test browser runs during the timings. Every Chrome uses a fresh silent profile.

## Shared skin palettes: final release

Date: 2026-11-14. Engine: `66e0d0d3`; retained review test: `9e5f0f5f`.
Same game package and server as the previous delivery. Same 500 ms scene-read protocol.
Host: RTX 3080, driver 580.178.04. WebGPU reports NVIDIA/Ampere and no software fallback.
Every actor remains visible and takes damage. Animation detail and cadence stay unchanged.

| Scene | Units | Quality | Frame p50, ms | Frame p95, ms | Mean FPS | Wasm high-water, MiB |
| --- | ---: | --- | ---: | ---: | ---: | ---: |
| Mixed fight | 400 | High | 16.70 | 16.70 | 59.30 | 193.6 |
| Mixed fight | 400 | Low | 16.70 | 16.70 | 59.20 | 172.1 |
| Mixed fight | 2000 | High | 33.30 | 50.00 | 31.45 | 392.8 |
| Mixed fight | 2000 | Low | 33.20 | 33.40 | 39.14 | 350.8 |
| Mixed fight and camera pan | 400 | High | 16.70 | 16.80 | 59.60 | 193.3 |
| Mixed fight and camera pan | 400 | Low | 16.70 | 16.80 | 59.60 | 172.2 |
| Same release with reference renderer | 2000 | High | 33.30 | 50.10 | 27.21 | 408.1 |

At 2,000 units, all 5,000 skin draws use 33 palettes and 1,353 matrices.
The paired High comparison uses one release binary: 27.21 FPS reference, 31.45 FPS shared.
This is about a 16 percent increase. High p95 remains 50 ms. The stress scene still has slow frames.
The previous delivered build reaches 25.12 FPS High and 31.85 FPS Low.
That older-binary comparison is separate from the paired reference result.
The 400-unit High threshold passes. Both 2,000-unit mean results exceed 30 FPS.
The paired High Wasm high-water falls from 408.1 MiB to 392.8 MiB.
Presentation timing is not an isolated GPU timer. Draw calls and GPU memory remain unavailable.
The five-second warm-up and ten-second sample stay unchanged. No compiler or other test browser runs during timing.
Evidence: `target/mc-evidence/shared-skin-palettes/final/`, one `result.json` per row folder.
Receipt: `target/mc-evidence/shared-skins-final-checks/receipt.json`: command 0, wrapper 0, source unchanged.
No new static detailed-model measurement runs. This change affects skin rendering.
