# Desktop browser adapter startup — 2026-10-28

User reported `No available adapters` followed by Bevy renderer panic at the7140 preview. Reproduced on DISPLAY=:1 using private headed Chrome on the actual desktop: default arguments expose navigator.gpu but requestAdapter returns null. Same browser with the test flags returns an NVIDIA adapter. This is a browser/backend availability difference; loopback is a secure context and the server is healthy.

Opened a separate Playwright-managed headed Chrome window with --enable-gpu --use-angle=vulkan --enable-features=Vulkan --enable-unsafe-webgpu. Existing browser profiles/settings untouched. Verified joined scene with15 live rendered units, zero page errors, and an actual visible factory screenshot. The in-range drone was already destroyed while the existing simulation ran. This window occupies the first factory-owner slot. A second client must use the same browser settings; slots remain permanent until server restart.

Evidence: target/m3-evidence/browser-startup/adapter-probe.json, desktop-playtest.json, desktop-playtest.png, desktop-launch.log. Detached desktop controller PID2816402 intentionally retained until its window closes. No engine/source/build changes; this is a local launch workaround, not general Linux browser support. Ordinary Chrome startup remains unsupported on this host without working WebGPU settings. Future UI should present an actionable adapter error rather than a blank canvas/panic.

The ready-to-test verdict is qualified by the tested Vulkan/WebGPU browser environment. Push/main approval still pending.
