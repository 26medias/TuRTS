# MC: two browser clients saturate one thread of the Chrome GPU process

Status: recorded, not judged (spec 15: the budgets stay open). The performance seat takes it
from here. Sources: the pacing probes of branch `codex/mc-fix-pacing` (2026-11-12) and probe
P-A of the pacing review (`target/mc-evidence/reviews/probe-a-gpu-thread.md` in the main
worktree; outside the repository). Both probes and their raw output are in
`docs/measurements/probes/mc-gpu-process/` with `SHA256SUMS`.
Change after the measurement (CEO sound rule, 2026-11-12): `probe.js` and `probe-a.js` now always give Chrome `--alsa-output-device=null` and the silent environment (`tests/browser/harness/silent-chrome.js`), and `SHA256SUMS` holds the new sums. The figures below came from the files with the old sums: `probe.js` `c395df413c46dfab8ce36284f9b66c92cfd03995dc20f4c403056184a7049868`, `probe-a.js` `4e071efaf9e6f99be1027f9cfe732b162a6fbf5c6a05c9ff3d3fce9a3df57036`.

## Conclusion in plain words

On the MC test host, most of the work of the Chrome GPU-process main thread for each frame is
the copy of each frame to the virtual display (Xvfb), not the work of the game client. A page
that only clears and shows a 1280 x 720 canvas costs that thread 7.9 to 10.7 ms for each
frame; the real client costs 12.3 ms. With two clients at full rate the thread is full, and a
third page waits a long time for its GPU device. On a real desktop display this copy is
expected to be absent; that is NOT measured. The frame limit that row SLOT1 stores (20) is
therefore a workaround for a host artefact, mostly.

## Finding 1: the thread saturates (pacing probe)

Two headed browser clients of the `forge` package, each at the default frame limit (no limit
when focused), hold the main thread of the Chrome GPU process at 93 to 96 % of one core. GPU
utilization stays at 24 to 35 % (about 34 % with no test client running: the desktop and other
work use the GPU too). A third page then waits 6.3 s, 9.1 s, 17.5 s or more than 30 s for its
WebGPU `requestDevice` (6 phases: 3 still pending at 30 s). The cause of the wait is the
saturated GPU-process thread, not the GPU.

The `forge` scene is small: 15 to 16 units in each replica (SLOT1 scene checkpoints,
`gameplay_entity_count` 15 in `server-stats.json`). The pacing review gives about 515 units for
MM1 (`muster`); the GPU-process thread was not measured with that scene.

Each value is one probe phase (`pacing-probe/probe-*.jsonl`). "Owners" are the two clients
that joined first.

| Owners | Owner frames per second | Owner frame p50 / p95 (ms) | GPU process CPU | GPU use | Third `requestDevice` |
|---|---|---|---|---|---|
| No limit (default) | 31.8 to 47.8 | 20.1 to 29.2 / 26.1 to 44.9 | 93 to 96 % (main thread) | 24 to 35 % | 6.3 s, 9.1 s, 17.5 s, > 30 s (x3) |
| Frame limit 30 (simulated in the page) | 29.0 to 29.8 | 33.3 to 33.5 / 35.9 to 41.7 | 77 to 80 % (main thread) | 21 to 36 % | 115, 178, 415, 474 ms |
| Frame limit 30 (client build) | 28.8 to 29.8 | 33.0 to 33.4 / 36.1 to 52.7 | 73 to 77 % (main thread) | 31 % | 101, 404 ms |
| Not focused, 10 (client build) | 9.7 to 9.8 | 100.0 to 100.2 / 101.6 to 113 | 25 to 27 % (main thread) | 35 to 38 % | 92, 104 ms |
| Frame limit 20 / 15 (simulated) | 19.5 to 19.9 / 14.7 | 50 / 67 | 54 to 68 % / 40 % (process total; type by start order, not read) | 25 to 37 % | 113, 141 ms / 96 ms |

The cost of the thread is per presented frame, not per second: thread % x 10 / total frames
per second gives 12.1 to 13.9 ms for each frame in every phase with thread data (no limit, 30
and 10). So 3 windows at 30 fps need 109 to 125 % of the thread, and 3 windows at 20 fps need
73 to 83 %. Row SLOT1 stores 20 for this reason.

The calls of one owner window at no limit, per second: 907 to 935 draw calls, 809 to 826 buffer
writes (`GPUQueue.writeBuffer`), 95 to 97 submits, 238 to 243 render passes. At about 47 frames
per second that is about 20 draw calls, 17 buffer writes, 2 submits and 5 render passes in each
frame. With a limit, the calls go down in proportion to the frame rate.

With a limit, the replica did not fall behind: each owner advanced 19.7 to 20.2 ticks per
second (the server runs at 20 Hz). The tick gap between the two owners was 0, except 1 tick in
2 limited phases (`probe-base-b`: the first 30 and the first 20 phase). The native test
`crates/turts_client/tests/background_pace.rs` drives the client once each 100 ms (the lowest
stored limit, 10) against a real server: tick gap 0, 20.00 ticks per second, no resync request.

## Finding 2: the cost is the copy to the virtual display (probe P-A)

From `probe-a/probe-a.jsonl` (field `gpu_main_ms_per_frame`, `gpu_main.pct`, `pages[].fps`).
ms for each present = CPU time of the GPU-process main thread in the 5 s window / frames shown
by all pages in the window. One lock hold, 9 phases, a new Chrome for each phase.

| Phase | Pages | Content | Frames per second of each page | GPU main thread | ms for each present |
|---|---|---|---|---|---|
| e | 2 | real client, `forge`, 1280x720 | 38.7, 37.7 | 94.9 % | 12.30 |
| a (run 1) | 2 | clear only, 1280x720 | 56.8, 56.6 | 90.3 % | 7.92 |
| b | 2 | clear only, 640x360 | 59.9, 59.9 | 27.5 % | 2.28 |
| c (run 1) | 2 | 20 draws, 17 writes (610,228 B), 5 passes, 2 submits, 1280x720 | 59.1, 58.9 | 84.6 % | 7.12 |
| d | 1 | clear only, 1280x720 | 59.3 | 53.2 % | 8.90 |
| f | 2 | a without `--disable-vulkan-surface` | 13.1, 13.3 | 94.2 % | 34.60 |
| e1 | 1 | real client, 1 window | 57.7 | 89.4 % | 15.37 |
| a (run 2) | 2 | clear only, 1280x720 | 41.5, 41.7 | 89.6 % | 10.67 |
| c (run 2) | 2 | as c | 23.3, 23.3 | 54.2 % | 11.46 |

- The clear-only page (a, d) costs 7.9 to 10.7 ms for each present: 64 to 87 % of the real
  client (12.3 ms, phase e). At 1/4 of the pixels (b) it costs 2.28 ms: the cost follows the
  pixels.
- A synthetic copy of the client's calls (c: the same draws, writes and passes) costs no more
  than a clear-only page in the same conditions (c run 1 7.12 ms against a run 1 7.92 ms). The
  calls are not the lever.
- The other present path (f, no `--disable-vulkan-surface`) is worse on Xvfb: 34.6 ms.
- The result file reads the perf profile of the thread as a CPU copy of each frame (kernel
  code and `memset`/`memmove`), and shows 12.2 to 14.9 % of the real client's samples in a
  kernel module (an inference: the nvidia module) against 1.0 to 2.3 % for the clear-only
  and synthetic pages (`probe-a-gpu-thread.md`, table column "perf").

## Finding 3: the client uploads about 610 KB each frame for a 15-unit scene

The real client writes 602 to 617 KB with 17 `writeBuffer` calls in each frame (`probe-a.jsonl`,
phase e, `calls_per_frame[].writeBytes`). The median frame has these write sizes, in call
order: 12, 512, 4096, 112, 2048, 8, 32, 4, 565760, 32640, 8, 112, 4, 48, 144, 80, 4608 B
(`median_frame.writes`).

The 565,760 B write is, by a size match, the Bevy UI vertex buffer, and the next write
(32,640 B) is the UI index buffer:
- Bevy UI writes its vertices and then its indices each frame
  (`bevy_ui_render-0.20.0/src/lib.rs:2423-2424`, `ui_meta.vertices.write_buffer`, then
  `ui_meta.indices.write_buffer`).
- One `UiVertex` (`lib.rs:1978-1995`) is 104 B (position 12, uv 8, color 16, flags 4,
  radius 32, border 16, size 8, point 8). 565,760 / 104 = 5,440 vertices.
- Each UI quad has 4 vertices and 6 `u32` indices (`lib.rs:2318-2343`). 32,640 / 4 = 8,160
  indices = 6 / 4 x 5,440. The two sizes agree: about 1,360 UI quads (boxes, borders and text
  glyphs) each frame.
- NOT TRACED at run time: the match is by size only, and which UI nodes give the 1,360 quads.

## Host and method

- Host: Intel Core i9-14900K (28 threads), 125 GB, NVIDIA GeForce RTX 3080 (driver
  580.178.04), Linux 7.0.0-42-generic.
- Browser: Google Chrome 157.0.8103.38, headed under Xvfb (`1280x720x24`), the flags of
  `mc_headed_v6.sh`: `--enable-gpu --use-angle=vulkan --enable-features=Vulkan
  --enable-unsafe-webgpu --disable-vulkan-surface --alsa-output-device=null`. Playwright
  emulates the focus, so every window counts as focused.
- Server: `turts_server` release, `tick_hz: 20`, package `mods/forge`; client: the release
  wasm of `trunk build --release`.
- Pacing probe (`pacing-probe/probe.js`): it opens two client windows (1280 x 720), waits until
  each has joined, measures 5 s, then opens a third window and times its `requestDevice` (limit
  30 s). An init script records each `getCurrentTexture` call (one in each Bevy frame) and
  counts the WebGPU calls. CPU is read from `/proc/<pid>/task/<tid>/stat` of the Chrome
  processes under the probe; GPU use is one `nvidia-smi` sample. The simulated limit holds each
  animation frame callback until the time of the limit has passed. The committed `probe.js` is
  the last version (runs `-d` and `cand-a`); runs `-a` and `-b` used earlier versions of the
  same script, without the thread names (`-a`, `-b`) and without the time-based limit (`-a`).
- Runs (each under the GPU lock): `pacing-probe-base-a`, `pacing-probe-base-b`,
  `pacing-probe-base-d` (client without the setting), `pacing-probe-cand-a` (client with
  limits 30 and 10; a probe build of a scratch clone, no commit: its exact source is NOT
  TRACED).
- Probe P-A: `probe-a/probe-a.js` and `probe-a/probe.html`; method and commands in the result
  file. Lock receipt: `pacing-rev-probe-a-hold1/lock-receipt.json`.
- Row evidence: SLOT1 with the default fails 2 of 2 (the third window gets no role in 30 s);
  SLOT1 with a stored frame limit passes 2 of 2.

## NOT TRACED

- A real desktop display (a GPU-composited surface, no Xvfb). That it has no CPU copy for each
  frame is expected, not measured. Also another GPU and another browser.
- Which client work causes the extra 1.6 to 6.5 ms for each frame over a clear-only page, and
  which client work calls into the kernel module (candidates: resources made each frame, map
  or readback, MSAA or depth attachments).
- Kernel symbol names (`kptr_restrict` is 1) and the CPU of the Xvfb process.
- The GPU-process load with MM1 (`muster`, about 515 units) and with more than two clients.
- Which UI nodes give the 1,360 UI quads of each frame (Finding 3).
- Why the audio threads of each renderer and the audio service use 46 to 77 % CPU with the
  ALSA null device in these runs.
