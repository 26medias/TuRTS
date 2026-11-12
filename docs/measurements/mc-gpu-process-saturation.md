# MC: two browser clients saturate one thread of the Chrome GPU process

Status: recorded, not judged (spec 15: the budgets stay open). The performance seat takes it
from here. Source: the pacing probes of branch `codex/mc-fix-pacing` (2026-11-12).

## Finding

Two headed browser clients of the `forge` package, each at the default frame limit (no limit
when focused), hold the main thread of the Chrome GPU process at 93 to 96 % of one core. GPU
utilization stays at 24 to 35 % (about 34 % with no test client running: the desktop and other
work use the GPU too). A third page then waits 6.3 s, 9.1 s, 17.5 s or more than 30 s for its
WebGPU `requestDevice` (6 phases: 3 still pending at 30 s). The cause of the wait is
the saturated GPU-process thread, not the GPU.

The scene in these runs is small: 15 to 16 units in each replica (SLOT1 scene checkpoints,
`gameplay_entity_count` 15 in `server-stats.json`). A unit count of about 515 was not observed
in these runs.

## Numbers

Each value is one probe phase. "Owners" are the two clients that joined first.

| Owners | Owner frames per second | Owner frame p50 / p95 (ms) | GPU process CPU | GPU use | Third `requestDevice` |
|---|---|---|---|---|---|
| No limit (default) | 31.8 to 47.8 | 20.1 to 29.2 / 26.1 to 44.9 | 93 to 96 % (main thread) | 24 to 35 % | 6.3 s, 9.1 s, 17.5 s, > 30 s (x3) |
| Frame limit 30 (simulated in the page) | 29.0 to 29.8 | 33.3 to 33.5 / 35.9 to 41.7 | 77 to 80 % (main thread) | 21 to 36 % | 115, 178, 415, 474 ms |
| Frame limit 30 (client build) | 28.8 to 29.8 | 33.0 to 33.4 / 36.1 to 52.7 | 73 to 77 % (main thread) | 31 % | 101, 404 ms |
| Not focused, 10 (client build) | 9.7 to 9.8 | 100.0 to 100.2 / 101.6 to 113 | 25 to 27 % (main thread) | 35 to 38 % | 92, 104 ms |
| Frame limit 20 / 15 (simulated) | 19.5 to 19.9 / 14.7 | 50 / 67 | 54 to 68 % / 40 % (process total; type by start order, not read) | 25 to 37 % | 113, 141 ms / 96 ms |

The calls of one owner window at no limit, per second: 907 to 935 draw calls, 809 to 826 buffer
writes (`GPUQueue.writeBuffer`), 95 to 97 submits, 238 to 243 render passes. At about 47 frames
per second that is about 20 draw calls, 17 buffer writes, 2 submits and 5 render passes in each
frame. With a limit, the calls go down in proportion to the frame rate.

The replica did not fall behind with a limit: each owner advanced 19.7 to 20.1 ticks per second
(the server runs at 20 Hz), with no tick gap between the two owners.

## Host and method

- Host: Intel Core i9-14900K (28 threads), 125 GB, NVIDIA GeForce RTX 3080 (driver
  580.178.04), Linux 7.0.0-42-generic.
- Browser: Google Chrome 157.0.8103.38, headed under Xvfb (`1280x720x24`), the flags of
  `mc_headed_v6.sh`: `--enable-gpu --use-angle=vulkan --enable-features=Vulkan
  --enable-unsafe-webgpu --disable-vulkan-surface --alsa-output-device=null`. Playwright
  emulates the focus, so every window counts as focused.
- Server: `turts_server` release, `tick_hz: 20`, package `mods/forge`; client: the release
  wasm of `trunk build --release`.
- Probe: a scratch Playwright script (not in the repository). It opens two client windows
  (1280 x 720), waits until each has joined, measures 5 s, then opens a third window and times
  its `requestDevice` (limit 30 s). An init script records each `getCurrentTexture` call (one in
  each Bevy frame) and counts the WebGPU calls. CPU is read from `/proc/<pid>/task/<tid>/stat`
  of the Chrome processes under the probe; GPU use is one `nvidia-smi` sample. The simulated
  limit holds each animation frame callback until the time of the limit has passed.
- Runs (each under the GPU lock): `pacing-probe-base-a`, `pacing-probe-base-b`,
  `pacing-probe-base-d` (client without the setting), `pacing-probe-cand-a` (client with
  limits 30 and 10). Raw output: the probe scratch folder of the task (`probe-*.jsonl`).
- Row evidence: SLOT1 with the default fails 2 of 2 (the third window gets no role in 30 s);
  SLOT1 with the stored frame limit 30 passes 2 of 2.

## NOT TRACED

- Which client work causes the GPU-process load: the draw call count, the buffer uploads in
  each frame, the render passes (5 in each frame, for example the UI pass or shadow passes),
  or the compositor work of `--disable-vulkan-surface` (frames read back for the X server).
- The same measurement on a real desktop display (no Xvfb, a Vulkan or GL surface), on another
  GPU, or in another browser.
- The load with more units (D57 medium workload) and with more than two clients.
- Why the audio threads of each renderer and the audio service use 46 to 77 % CPU with the
  ALSA null device in these runs.
