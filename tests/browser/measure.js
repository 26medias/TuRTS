// turts-test-role: instrument
// Browser measurements for the M1 client (design sections 4.5 and 7; plan Tasks 12 and 13).
//
// Usage: node measure.js <page url with ?server=..&ws=..> <label> [extra chrome args...]
//
// Opens the page in headless Chrome (CHROME_BIN, default /usr/bin/google-chrome) at 1280x720,
// waits for the first frame (window.turts_debug() returns a non-empty string), and then
// records for 10 s. It prints one JSON object:
//   - firstFrameMs: from the start of the navigation to the first frame;
//   - bytesAtFirstFrame: Resource Timing transferSize and decodedBodySize of each file, and
//     their sum, read at the first frame (the download size, with brotli on);
//   - frames: count, p50, p99 and max of the requestAnimationFrame intervals, in ms. The
//     browser calls requestAnimationFrame once per frame that Bevy runs on wasm32;
//   - wasmMemoryBytes: samples taken each second from window.turts_memory_bytes(), and the
//     last one (wasm memory never shrinks, so the last sample is the high-water mark). It is
//     null while the web crate does not export that hook (Task 13 adds it);
//   - phase: the phase in the debug JSON at the end;
//   - gl: the WebGL2 renderer string, so a run says whether it used SwiftShader or a GPU.
// Default flags give software rendering (SwiftShader). Performance runs on a dev machine pass
// `--enable-gpu --use-angle=gl`. The two kinds of run are never compared (design 4.5).
const { chromium } = require("playwright-core");
const os = require("node:os");
const { installWebGpuTelemetry } = require("./measure-webgpu-hook.js");
const { buildVis2LaunchOptions } = require("./harness/measure-launch-options.js");
const { adapterEvidence, vis2Ready: isVis2Ready } = require("./vis2-validation.js");

const FIRST_FRAME_TIMEOUT_MS = 60000;
const RECORD_MS = 10000;
const SAMPLE_MS = 1000;

const percentile = (sorted, p) => sorted[Math.floor(p * (sorted.length - 1))];

(async () => {
    const [url, label, ...extra] = process.argv.slice(2);
    if (!url || !label) {
        console.error("usage: node measure.js <page url> <label> [extra chrome args...]");
        process.exit(2);
    }
    const executablePath = process.env.CHROME_BIN || "/usr/bin/google-chrome";
    const vis2 = label === "vis2";
    const launchOptions = vis2
        ? buildVis2LaunchOptions(executablePath, extra, process.env)
        : { executablePath, headless: true, args: extra };
    const browser = await chromium.launch(launchOptions);
    const page = await browser.newPage({ viewport: { width: 1280, height: 720 } });
    if (vis2) await page.addInitScript(installWebGpuTelemetry);
    const logs = [];
    page.on("console", (m) => logs.push(m.type() + ": " + m.text()));
    page.on("pageerror", (e) => logs.push("pageerror: " + e.message));

    const startedAt = Date.now();
    await page.goto(url);
    const gl = vis2 ? null : await page.evaluate(() => {
        const context = document.createElement("canvas").getContext("webgl2");
        return context ? context.getParameter(context.RENDERER) : "no webgl2";
    });

    let firstFrameMs = null;
    let actualUnits = null;
    while (Date.now() - startedAt < FIRST_FRAME_TIMEOUT_MS) {
        const ready = vis2
            ? await page.evaluate(() => {
                if (typeof window.turts_debug !== "function" || typeof window.turts_scene !== "function") return null;
                try {
                    const debug = JSON.parse(window.turts_debug());
                    const scene = JSON.parse(window.turts_scene());
                    const worldCount = debug?.replicated_world?.units?.length ?? null;
                    const sceneCount = Array.isArray(scene?.units) ? scene.units.length : null;
                    return { ok: worldCount === 5000 && sceneCount === 5000, units: { replicatedWorld: worldCount, renderedScene: sceneCount } };
                } catch {
                    return null;
                }
            }).catch(() => null)
            : await page.evaluate(() => typeof window.turts_debug === "function" ? window.turts_debug() : "")
                .then((text) => ({ ok: Boolean(text), units: null }))
                .catch(() => ({ ok: false, units: null }));
        if (ready?.ok) {
            firstFrameMs = Date.now() - startedAt;
            actualUnits = ready.units;
            break;
        }
        await page.waitForTimeout(50);
    }

    let report = { label, chrome: browser.version(), gl, firstFrameMs };
    if (vis2) {
        const cpus = os.cpus();
        report.machine = {
            platform: process.platform,
            architecture: os.arch(),
            release: os.release(),
            cpu_model: cpus[0]?.model || null,
            logical_processors: cpus.length,
            total_memory_bytes: os.totalmem(),
        };
        report.display = process.env.DISPLAY || null;
        report.chromeArgs = launchOptions.args;
        report.chromeIgnoreDefaultArgs = launchOptions.ignoreDefaultArgs;
        report.browserMode = "headed";
    }
    if (firstFrameMs !== null) {
        const bytesAtFirstFrame = await page.evaluate(() => {
            const entries = performance
                .getEntriesByType("navigation")
                .concat(performance.getEntriesByType("resource"))
                .map((e) => ({ name: new URL(e.name).pathname.split("/").pop() || "/", transfer: e.transferSize, decoded: e.decodedBodySize }));
            return { files: entries, transferTotal: entries.reduce((sum, e) => sum + e.transfer, 0) };
        });
        const gpuStart = await page.evaluate((measureGpu) => {
            window.__rafIntervals = [];
            let previous = performance.now();
            const step = (now) => {
                window.__rafIntervals.push(now - previous);
                previous = now;
                requestAnimationFrame(step);
            };
            requestAnimationFrame(step);
            return measureGpu ? { ...window.__mcGpuMetrics } : null;
        }, vis2);
        const memory = [];
        for (let waited = 0; waited < RECORD_MS; waited += SAMPLE_MS) {
            await page.waitForTimeout(SAMPLE_MS);
            memory.push(
                await page.evaluate(() =>
                    typeof window.turts_memory_bytes === "function" ? window.turts_memory_bytes() : null
                )
            );
        }
        const intervals = await page.evaluate(() => window.__rafIntervals.slice(1));
        const sorted = intervals.slice().sort((a, b) => a - b);
        const phase = await page.evaluate(() => {
            const text = window.turts_debug();
            return text ? JSON.parse(text).phase : null;
        });
        const vis2Measurements = vis2 ? await page.evaluate((start) => {
            const current = window.__mcGpuMetrics;
            const currentTextureAcquisitions = current.currentTextureAcquisitions - start.currentTextureAcquisitions;
            const submittedDrawCalls = current.submittedDrawCalls - start.submittedDrawCalls;
            const debug = JSON.parse(window.turts_debug());
            const scene = JSON.parse(window.turts_scene());
            return {
                actualUnits: scene.units.length,
                worldUnits: debug.replicated_world?.units?.length ?? null,
                renderAdapter: scene.render_adapter ?? null,
                webgpu: {
                    adapterRequested: current.adapterRequested,
                    adapterInfo: current.adapterInfo,
                    activeDeviceId: current.activeDeviceId,
                    acquiredDeviceCount: current.acquiredDeviceCount,
                    deviceAcquired: current.deviceAcquired,
                    configuredDeviceAcquired: current.configuredDeviceAcquired,
                    configuredContexts: current.configuredContexts,
                    currentTextureAcquisitions,
                    submittedDrawCalls,
                    meanSubmittedDrawCallsPerRafInterval: window.__rafIntervals.length > 1
                        ? submittedDrawCalls / (window.__rafIntervals.length - 1) : null,
                    submissions: current.submissions - start.submissions,
                    executedBundles: current.executedBundles - start.executedBundles,
                    instrumentationErrors: current.instrumentationErrors,
                },
            };
        }, gpuStart) : null;
        report = {
            ...report,
            ...(vis2 ? { actualUnits, vis2: vis2Measurements } : {}),
            bytesAtFirstFrame,
            frames: {
                count: sorted.length,
                p50_ms: sorted.length ? percentile(sorted, 0.5) : null,
                p99_ms: sorted.length ? percentile(sorted, 0.99) : null,
                max_ms: sorted.length ? sorted[sorted.length - 1] : null,
            },
            wasmMemoryBytes: { samples: memory, last: memory[memory.length - 1] },
            phase,
        };
    } else {
        report.logs = logs.slice(0, 12);
    }
    if (vis2 && report.vis2) {
        report.vis2.adapterEvidence = adapterEvidence(report);
    }
    const vis2Ready = !vis2 || isVis2Ready(report);
    console.log(JSON.stringify(report, null, 2));
    await browser.close();
    process.exit(firstFrameMs === null || !vis2Ready ? 1 : 0);
})().catch((e) => {
    console.error(e);
    process.exit(1);
});
