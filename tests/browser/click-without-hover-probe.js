// turts-test-role: instrument
// Review M1 of `mp1/sound-off-default`: a click on an engine button that the pointer reached in the
// same frame (no hover frame before it) must act. The cause was in `bevy_picking` (see
// `crates/turts_client/src/press_release.rs`). This probe makes that order on purpose: the pointer
// rests far away for 300 ms, then the move, the press and the release go to the browser in one
// batch. It then waits 500 ms and reads one debug record (the request keeps its 4 Hz cap: the
// debug request ruling allows an interval only to the SH1 phase capture). The effect must show in
// that record. It is not a row: it reads the debug bridge to find the buttons (it is an
// instrument, not a driver).
//
// Pass 3 adds `--map-press-first`: before the click check, one frame with a press on the map and a
// release over the sound switch (a box select that ends on a button inside one frame) must NOT
// turn the sound on: a click needs a press and a release at the same button.
//
// Usage: node click-without-hover-probe.js <game url> [--guide-close] [--map-press-first]
//   The url is the `URL:` line of `scripts/mc-browser.sh <package> --serve-only --prepared DIR`
//   (`scripts/click-without-hover.sh` runs both). The probe starts its own Chrome with the browser
//   profile of the MC scripts (`MC_CHROME_ARGS_JSON`, as `egpu-run.js`), so run it under the headed
//   wrapper. Checks: the sound switch (the button of the package or the engine fallback) turns
//   the sound on with one such click; with `--guide-close` the Close button of the open guide hides
//   the guide with one such click (a second kind of engine button).
// Output: one JSON object on stdout; each check has `expected` and `acted`. Exit 0: every check
// came out as expected; 1: one did not.
"use strict";

const { chromium } = require("playwright-core");

const WAIT_MS = 500;

async function record(page) {
    return page.evaluate(async () => JSON.parse(await (window.turts_debug_request ? window.turts_debug_request() : window.turts_debug())));
}

async function until(page, predicate, millis) {
    const started = Date.now();
    for (;;) {
        const debug = await record(page);
        if (predicate(debug)) return { debug, ms: Date.now() - started };
        if (Date.now() - started > millis) return { debug, ms: null };
    }
}

// Rest the pointer far from the target for 300 ms, then send move, press and release together.
async function quickClick(page, x, y) {
    await page.mouse.move(2, 2);
    await page.waitForTimeout(300);
    await Promise.all([page.mouse.move(x, y), page.mouse.down(), page.mouse.up()]);
}

function centre(row, scale) {
    const [left, top, width, height] = row.rect;
    return [(left + width / 2) / scale, (top + height / 2) / scale];
}

async function run(url, wantGuide, mapPressFirst) {
    const chromeArgs = JSON.parse(process.env.MC_CHROME_ARGS_JSON || '["--enable-gpu","--use-angle=vulkan","--enable-features=Vulkan","--enable-unsafe-webgpu"]');
    if (!chromeArgs.some((arg) => arg.startsWith("--force-device-scale-factor"))) chromeArgs.push("--force-device-scale-factor=1");
    const browser = await chromium.launch({
        executablePath: process.env.CHROME_BIN || "/usr/bin/google-chrome",
        headless: process.env.MC_BROWSER_HEADED !== "1",
        args: chromeArgs,
        ignoreDefaultArgs: JSON.parse(process.env.MC_CHROME_IGNORE_DEFAULT_ARGS_JSON || '["--enable-unsafe-swiftshader"]'),
    });
    const result = { checks: [] };
    try {
        const context = await browser.newContext({ viewport: { width: 1280, height: 720 }, deviceScaleFactor: 1 });
        const page = await context.newPage();
        await page.goto(url);
        await page.waitForFunction(() => typeof window.turts_debug === "function", null, { timeout: 60000 });
        const ready = await until(page, (debug) => typeof debug.phase === "string" && debug.phase.startsWith("Joined") &&
            (debug.ui_snapshot || []).some((row) => row.kind === "focus_overlay" && row.root_count === 1), 60000);
        if (ready.ms === null) throw new Error("the game did not join and show its start overlay in 60 s");
        // A key is a gesture: it removes the start overlay and resumes the audio contexts. Shift: F1 would toggle the guide.
        await page.keyboard.press("ShiftLeft");
        const scale = await page.evaluate(() => window.devicePixelRatio || 1);
        const settled = await until(page, (debug) => (debug.ui_snapshot || []).some((row) => row.kind === "focus_overlay" && row.root_count === 0) &&
            debug.audio?.context_states?.every((state) => state === "running"), 10000);
        if (settled.ms === null) throw new Error("the start overlay stayed or the audio contexts do not run");
        const isSwitch = (row) => row.kind === "sound_switch" || (row.kind === "button" && row.widget_action?.name === "toggle_sound");
        const sound = settled.debug.ui_snapshot.find((row) => isSwitch(row) && row.visible === true && Array.isArray(row.rect));
        if (!sound) throw new Error("no visible sound switch in the ui snapshot");
        if (settled.debug.audio.sound_on !== false) throw new Error("the sound is not off at the start");
        const [sx, sy] = centre(sound, scale);
        if (mapPressFirst) {
            // The map point: away from every widget, in the middle of the screen. The browser can split
            // the batch over two frames, and then Bevy is right by itself; so the batch is sent 12 times.
            // With a repair that clicks a press-then-release at two places, one of them lands in one
            // frame and turns the sound on (plant 7i of `sound-off.json`).
            const mx = 640, my = 400;
            let acted = false;
            // A slow page (CPU throttle 20) has frames of about 300 ms: the batch of a few milliseconds
            // is then in one frame, as at a low frame rate (review M1).
            const cdp = await page.context().newCDPSession(page);
            await cdp.send("Emulation.setCPUThrottlingRate", { rate: 20 });
            for (let attempt = 0; attempt < 12 && !acted; attempt += 1) {
                await page.mouse.move(2, 2);
                await page.waitForTimeout(300);
                await Promise.all([page.mouse.move(mx, my), page.mouse.down(), page.mouse.move(sx, sy), page.mouse.up()]);
                await page.waitForTimeout(300);
                acted = (await record(page)).audio?.sound_on === true;
            }
            await cdp.send("Emulation.setCPUThrottlingRate", { rate: 1 });
            result.checks.push({ name: "press on the map, release over the sound switch, one frame (12 tries)", kind: sound.kind, expected: false, acted, within_ms: WAIT_MS });
        }
        await quickClick(page, sx, sy);
        await page.waitForTimeout(WAIT_MS);
        const on = await record(page);
        result.checks.push({ name: "sound switch, one click with no hover frame", kind: sound.kind, expected: true, acted: on.audio?.sound_on === true, within_ms: WAIT_MS });
        if (wantGuide) {
            const close = (settled.debug.ui_snapshot || []).find((row) => row.id === "guide_close" && row.visible === true && Array.isArray(row.rect));
            if (!close) throw new Error("no visible guide_close button (the guide is open at the join in this package)");
            const [cx, cy] = centre(close, scale);
            await quickClick(page, cx, cy);
            await page.waitForTimeout(WAIT_MS);
            const closed = await record(page);
            result.checks.push({ name: "guide Close button, one click with no hover frame", expected: true, acted: (closed.ui_snapshot || []).some((row) => row.id === "guide_close" && row.visible === false), within_ms: WAIT_MS });
        }
    } finally {
        await browser.close().catch(() => {});
    }
    return result;
}

async function main() {
    const [url, ...flags] = process.argv.slice(2);
    if (!/^http:\/\/127\.0\.0\.1:\d+\//.test(url || "")) {
        console.error("usage: click-without-hover-probe.js <game url on 127.0.0.1> [--guide-close] [--map-press-first]");
        process.exit(2);
    }
    const result = await run(url, flags.includes("--guide-close"), flags.includes("--map-press-first"));
    console.log(JSON.stringify(result));
    process.exit(result.checks.every((check) => check.acted === check.expected) ? 0 : 1);
}

if (require.main === module) main().catch((error) => { console.error(error.stack || String(error)); process.exit(1); });
module.exports = { quickClick, centre };
