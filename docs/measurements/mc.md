# MC Rendering Measurements

## VIS2

Measured 2026-11-09 with the headed `measure.js` WebGPU profile on `DISPLAY=:1`. The server ran the unchanged VIS2 row: 515 authored Muster units plus 4,485 synthetic units, for 5,000 replicated and rendered units. Chrome 157.0.8103.38 ran with Vulkan WebGPU flags and SwiftShader defaults disabled.

The machine was Linux x86_64, kernel 7.0.0-42-generic, Intel Core i9-14900K, 28 logical processors, and 134,726,823,936 bytes of system memory. Bevy reported `BrowserWebGpu` / `Other` with an empty adapter name; the browser redacted that label. The configured WebGPU adapter API independently reported vendor `nvidia`, architecture `ampere`. The measured run acquired and configured the device, acquired 90 current textures, submitted 1,530 draw calls, and recorded no instrumentation errors. The actual adapter and measurement JSON are retained in the evidence directory below.

| Metric | Measured value | Unit |
| --- | ---: | --- |
| Frame interval p50 | 133.3 | ms |
| Frame interval p99 | 183.3 | ms |
| Submitted draw calls | 1,530 | calls over 10 s |
| WebAssembly memory high-water mark | 558,628,864 | bytes |

First frame: 4,983 ms. The run recorded 87 frame intervals and a mean 17.19 submitted draw calls per RAF interval. No acceptance threshold is specified here; the measurement is a hardware-specific record.

Raw measurement: `target/mc-evidence/t13-vis2-9cf2363-20261109/vis2-actual-measurement.json` (SHA-256 `3f6b6e862e9d6fa29867dd32300e4bc2a28e4be67f4092f84695efd67498a6cd`). The evidence also contains the actual server/browser logs, the headed profile, machine details, adapter/device/context/draw counters, and completion status.
