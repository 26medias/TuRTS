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
// Usage: node click-without-hover-probe.js <debug port> [--guide-close]
//   The port is the `DEBUG_PORT` that `scripts/mp1-play.sh` printed (`scripts/click-without-hover.sh`
//   runs both). Checks: the sound switch (the button of the package or the engine fallback) turns
//   the sound on with one such click; with `--guide-close` the Close button of the open guide hides
//   the guide with one such click (a second kind of engine button).
// Output: one JSON object on stdout. Exit 0: every check acted within 500 ms; 1: one did not.
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

async function run(port, wantGuide) {
    const browser = await chromium.connectOverCDP(`http://127.0.0.1:${port}`);
    const page = browser.contexts().flatMap((context) => context.pages()).find((candidate) => /^https?:/.test(candidate.url()));
    if (!page) throw new Error("no game page on the debug port");
    const result = { checks: [] };
    try {
        const ready = await until(page, (debug) => typeof debug.phase === "string" && debug.phase.startsWith("Joined") &&
            (debug.ui_snapshot || []).some((row) => row.kind === "focus_overlay" && row.root_count === 1), 60000);
        if (ready.ms === null) throw new Error("the game did not join and show its start overlay in 60 s");
        // A key is a gesture: it removes the start overlay and resumes the audio contexts.
        await page.keyboard.press("F1");
        const scale = await page.evaluate(() => window.devicePixelRatio || 1);
        const settled = await until(page, (debug) => (debug.ui_snapshot || []).some((row) => row.kind === "focus_overlay" && row.root_count === 0) &&
            debug.audio?.context_states?.every((state) => state === "running"), 10000);
        if (settled.ms === null) throw new Error("the start overlay stayed or the audio contexts do not run");
        const isSwitch = (row) => row.kind === "sound_switch" || (row.kind === "button" && row.widget_action?.name === "toggle_sound");
        const sound = settled.debug.ui_snapshot.find((row) => isSwitch(row) && row.visible === true && Array.isArray(row.rect));
        if (!sound) throw new Error("no visible sound switch in the ui snapshot");
        if (settled.debug.audio.sound_on !== false) throw new Error("the sound is not off at the start");
        const [sx, sy] = centre(sound, scale);
        await quickClick(page, sx, sy);
        await page.waitForTimeout(WAIT_MS);
        const on = await record(page);
        result.checks.push({ name: "sound switch, one click with no hover frame", kind: sound.kind, acted: on.audio?.sound_on === true, within_ms: WAIT_MS });
        if (wantGuide) {
            const close = (settled.debug.ui_snapshot || []).find((row) => row.id === "guide_close" && row.visible === true && Array.isArray(row.rect));
            if (!close) throw new Error("no visible guide_close button (the guide is open at the join in this package)");
            const [cx, cy] = centre(close, scale);
            await quickClick(page, cx, cy);
            await page.waitForTimeout(WAIT_MS);
            const closed = await record(page);
            result.checks.push({ name: "guide Close button, one click with no hover frame", acted: (closed.ui_snapshot || []).some((row) => row.id === "guide_close" && row.visible === false), within_ms: WAIT_MS });
        }
    } finally {
        await browser.close().catch(() => {});
    }
    return result;
}

async function main() {
    const [port, ...flags] = process.argv.slice(2);
    if (!/^\d+$/.test(port || "")) {
        console.error("usage: click-without-hover-probe.js <debug port> [--guide-close]");
        process.exit(2);
    }
    const result = await run(Number(port), flags.includes("--guide-close"));
    console.log(JSON.stringify(result));
    process.exit(result.checks.every((check) => check.acted) ? 0 : 1);
}

if (require.main === module) main().catch((error) => { console.error(error.stack || String(error)); process.exit(1); });
module.exports = { quickClick, centre };
