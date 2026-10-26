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
    const browser = await chromium.launch({ executablePath, headless: true, args: extra });
    const page = await browser.newPage({ viewport: { width: 1280, height: 720 } });
    const logs = [];
    page.on("console", (m) => logs.push(m.type() + ": " + m.text()));
    page.on("pageerror", (e) => logs.push("pageerror: " + e.message));

    const startedAt = Date.now();
    await page.goto(url);
    const gl = await page.evaluate(() => {
        const context = document.createElement("canvas").getContext("webgl2");
        return context ? context.getParameter(context.RENDERER) : "no webgl2";
    });

    let firstFrameMs = null;
    while (Date.now() - startedAt < FIRST_FRAME_TIMEOUT_MS) {
        const text = await page
            .evaluate(() => (typeof window.turts_debug === "function" ? window.turts_debug() : ""))
            .catch(() => "");
        if (text) {
            firstFrameMs = Date.now() - startedAt;
            break;
        }
        await page.waitForTimeout(50);
    }

    let report = { label, chrome: browser.version(), gl, firstFrameMs };
    if (firstFrameMs !== null) {
        const bytesAtFirstFrame = await page.evaluate(() => {
            const entries = performance
                .getEntriesByType("navigation")
                .concat(performance.getEntriesByType("resource"))
                .map((e) => ({ name: new URL(e.name).pathname.split("/").pop() || "/", transfer: e.transferSize, decoded: e.decodedBodySize }));
            return { files: entries, transferTotal: entries.reduce((sum, e) => sum + e.transfer, 0) };
        });
        await page.evaluate(() => {
            window.__rafIntervals = [];
            let previous = performance.now();
            const step = (now) => {
                window.__rafIntervals.push(now - previous);
                previous = now;
                requestAnimationFrame(step);
            };
            requestAnimationFrame(step);
        });
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
        report = {
            ...report,
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
    console.log(JSON.stringify(report, null, 2));
    await browser.close();
    process.exit(firstFrameMs === null ? 1 : 0);
})().catch((e) => {
    console.error(e);
    process.exit(1);
});
