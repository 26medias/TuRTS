# MC Rendering Measurements

## VIS2

Measured 2026-11-09 from source head `40b9d90effc3f20e3e830f8018cabd9925ad4849` using the headed Chrome profile under an isolated Xvfb display (`:100`). The unchanged VIS2 server row loaded 515 authored Muster units plus 4,485 synthetic units; the browser reported 5,000 replicated world units and 5,000 rendered scene units. Chrome 157.0.8103.38 used Vulkan WebGPU flags with SwiftShader disabled.

The host inventory identifies an Intel Core i9-14900K and NVIDIA GA102 GeForce RTX 3080 (`10de:2206`), driver 580.178.04. Bevy reported `BrowserWebGpu` / `Other` with an empty adapter name (redacted by the browser); WebGPU API metadata for the configured application device reported vendor `nvidia`, architecture `ampere`. The application acquired and configured its device, recorded 89 current-texture acquisitions and 1,529 submitted draw calls, and reported no instrumentation errors. `configuredContexts` is a count of configuration events, not a claim about distinct contexts.

| Profile | Metric | Value | Unit |
| --- | --- | ---: | --- |
| VIS2 | p50 | 133.3 | ms |
| VIS2 | p99 | 199.9 | ms |
| VIS2 | draw calls | 1529 | calls over 10 s |
| VIS2 | memory | 505872384 | bytes |

The capture recorded 86 frame intervals, a 17.375 mean submitted draw calls per RAF interval, and a 5,611 ms first frame. No acceptance threshold is specified here. GCP is pending (D59).

The raw measurement is `target/mc-evidence/t13-vis2-final-40b9d90-20261109/vis2-final-actual.json` (SHA-256 `d58dd8ed430a7a2b39564f137e98c33aac9dec0118103b5474b7807e9f792df0`). Its command receipt binds the headed browser argv, served URL, source hashes, and configured-device proof. The same evidence directory contains the server run, exact grep red/green logs, and raw `lscpu`, `lspci -nn`, and `nvidia-smi` inventories.

### Historical measurement

The earlier 2026-11-09 measurement used source head `9cf23638214e79a2800bbe1bf8a84d9a4f92b45a` on `DISPLAY=:1`. It recorded p50 133.3 ms, p99 183.3 ms, 1,530 total submitted draw calls over 10 seconds (17.191 mean per RAF interval), 558628864 bytes of WebAssembly memory, and an initial frame at 4,983 ms. This is retained as historical evidence and is not the final-source result. Its raw JSON remains at `target/mc-evidence/t13-vis2-9cf2363-20261109/vis2-actual-measurement.json` with the original source-bound receipts.
