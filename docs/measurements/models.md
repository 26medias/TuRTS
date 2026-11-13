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
