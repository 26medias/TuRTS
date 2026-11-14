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
