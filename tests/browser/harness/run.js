// turts-test-role: harness
"use strict";

const fs = require("node:fs/promises");
const path = require("node:path");
const { createDriverIo } = require("./driver-io.js");
const { createView } = require("./view.js");

function deepFreeze(value) {
    if (value && typeof value === "object" && !Object.isFrozen(value)) {
        Object.freeze(value);
        for (const key of Object.keys(value)) deepFreeze(value[key]);
    }
    return value;
}

function parseArgs(args) {
    if (args.length < 4) throw new Error("usage: run.js <url> <out-dir> <driver.js> <oracle.js> --params <params.json> --viewport <viewport>");
    const [url, outDir, driverPath, oraclePath, ...flags] = args;
    let paramsPath = "";
    let viewport = "";
    let windows = null;
    for (let index = 0; index < flags.length;) {
        const flag = flags[index++];
        if (flag === "--params" && flags[index]) paramsPath = flags[index++];
        else if (flag === "--viewport" && flags[index]) viewport = flags[index++];
        else if (flag === "--windows") {
            if (windows !== null || !flags[index] || !/^[1-9]\d*$/.test(flags[index]) || !Number.isSafeInteger(Number(flags[index]))) {
                throw new Error("--windows requires a positive integer");
            }
            windows = Number(flags[index++]);
        }
        else throw new Error(`unknown or incomplete argument: ${flag}`);
    }
    if (!paramsPath || !viewport) throw new Error("both --params and --viewport are required");
    const match = /^(\d+)x(\d+)(?:@([0-9]+(?:\.[0-9]+)?))?$/.exec(viewport);
    if (!match) throw new Error(`invalid viewport: ${viewport}`);
    return { url, outDir, driverPath, oraclePath, paramsPath, width: Number(match[1]), height: Number(match[2]), deviceScaleFactor: Number(match[3] || 1), windows };
}

async function run(options) {
    const loadedParams = JSON.parse(await fs.readFile(options.paramsPath, "utf8"));
    const windows = options.windows ?? loadedParams.windows ?? 1;
    if (!Number.isSafeInteger(windows) || windows < 1) throw new Error("params.windows must be a positive integer");
    const params = deepFreeze({ ...loadedParams, windows });
    const { chromium } = require("playwright-core");
    const driver = require(path.resolve(options.driverPath));
    const oracle = require(path.resolve(options.oraclePath));
    if (typeof driver.drive !== "function") throw new Error("driver must export drive(io, params)");
    if (typeof oracle.judge !== "function") throw new Error("oracle must export judge(view, params)");
    const browser = await chromium.launch({
        executablePath: process.env.CHROME_BIN || undefined,
        headless: process.env.MC_BROWSER_HEADED !== "1",
        args: JSON.parse(process.env.MC_CHROME_ARGS_JSON || '["--enable-gpu","--use-angle=vulkan","--enable-features=Vulkan","--enable-unsafe-webgpu"]'),
        ignoreDefaultArgs: JSON.parse(process.env.MC_CHROME_IGNORE_DEFAULT_ARGS_JSON || '["--enable-unsafe-swiftshader"]'),
    });
    const pages = [];
    const inputLog = [];
    const serverDir = options.outDir;
    try {
        const windows = Number(params.windows || 1);
        for (let n = 0; n < windows; n += 1) {
            const context = await browser.newContext({ viewport: { width: options.width, height: options.height }, deviceScaleFactor: Number(params.device_scale_factor || options.deviceScaleFactor) });
            await context.addInitScript(() => {
                window.__turts_io_actual_events = [];
                window.__turts_io_dispatching = false;
                window.__turts_io_expected_event = null;
                window.__turts_io_downstream_events = [];
                const ambientKey = "__turts_io_ambient_events";
                let ambient = [];
                try { ambient = JSON.parse(sessionStorage.getItem(ambientKey) || "[]"); } catch {}
                window.__turts_io_ambient_events = Array.isArray(ambient) ? ambient : [];
                const types = ["pointerdown", "pointerup", "pointermove", "keydown", "keyup", "wheel"];
                for (const type of types) document.addEventListener(type, (event) => {
                    const canvas = document.querySelector("canvas");
                    const expected = window.__turts_io_expected_event;
                    const observed = {
                        type,
                        target: event.target?.tagName || null,
                        canvas_target: Boolean(canvas && (event.target === canvas || canvas.contains(event.target))),
                        shift_key: Boolean(event.shiftKey),
                        ctrl_key: Boolean(event.ctrlKey),
                        alt_key: Boolean(event.altKey),
                        meta_key: Boolean(event.metaKey),
                        wall_time_ms: Date.now(),
                        client_x: Number.isFinite(event.clientX) ? event.clientX : null,
                        client_y: Number.isFinite(event.clientY) ? event.clientY : null,
                        code: typeof event.code === "string" ? event.code : null,
                        key: typeof event.key === "string" ? event.key : null,
                        delta_y: Number.isFinite(event.deltaY) ? event.deltaY : null,
                    };
                    const matches = window.__turts_io_dispatching && !expected?.matched && Number.isInteger(expected?.seq) && expected?.type === type &&
                        (expected.client_x === undefined || Math.abs(observed.client_x - expected.client_x) < 0.01) &&
                        (expected.client_y === undefined || Math.abs(observed.client_y - expected.client_y) < 0.01) &&
                        (expected.code === undefined || observed.code === expected.code) &&
                        (expected.key === undefined || observed.key === expected.key) &&
                        (expected.delta_y === undefined || Math.abs(observed.delta_y - expected.delta_y) < 0.01);
                    if (matches) {
                        expected.matched = true;
                        window.__turts_io_seq = expected.seq;
                        window.__turts_io_actual_events.push({ ...observed, seq: expected.seq });
                    } else {
                        window.__turts_io_seq = null;
                        window.__turts_io_ambient_events.push({ ...observed, seq: null, during_dispatch_seq: window.__turts_io_dispatching ? expected?.seq ?? null : null, expected_type: expected?.type || null });
                        if (window.__turts_io_ambient_events.length > 1000) window.__turts_io_ambient_events.shift();
                        try { sessionStorage.setItem(ambientKey, JSON.stringify(window.__turts_io_ambient_events)); } catch {}
                    }
                }, true);
                for (const type of types) document.addEventListener(type, (event) => {
                    window.__turts_io_downstream_events.push({ type, seq: window.__turts_io_seq, wall_time_ms: Date.now(), client_x: Number.isFinite(event.clientX) ? event.clientX : null, client_y: Number.isFinite(event.clientY) ? event.clientY : null, code: typeof event.code === "string" ? event.code : null, key: typeof event.key === "string" ? event.key : null, delta_y: Number.isFinite(event.deltaY) ? event.deltaY : null, shift_key: Boolean(event.shiftKey) });
                }, true);
            });
            const page = await context.newPage();
            await page.mouse.move(options.width / 2, options.height / 2);
            pages.push(page);
            await page.goto(options.url, { waitUntil: "domcontentloaded" });
        }
        for (const page of pages) {
            await page.bringToFront();
            await page.waitForFunction(() => {
                if (window.turts_error) return true;
                if (typeof window.turts_debug !== "function" || typeof window.turts_scene !== "function") return false;
                try {
                    const debug = JSON.parse(window.turts_debug());
                    const scene = JSON.parse(window.turts_scene());
                    return typeof debug.phase === "string" && debug.phase.startsWith("Joined") && Boolean(debug.last_frame) && scene.cameras_3d === 1;
                } catch { return false; }
            }, null, { timeout: 30000 });
            const bootError = await page.evaluate(() => window.turts_error || null);
            if (bootError) throw new Error(`browser client failed: ${bootError}`);
            await page.evaluate(() => {
                window.__turts_io_seq = 0;
                if (typeof window.turts_debug === "function" && typeof window.turts_debug.frames_clear === "function") window.turts_debug.frames_clear();
            });
        }
        await pages[0].bringToFront();
        await pages[0].waitForTimeout(100);
        for (const page of pages) await page.evaluate(() => { window.__turts_io_actual_events = []; window.__turts_io_downstream_events = []; window.__turts_io_seq = null; window.__turts_io_dispatching = false; window.__turts_io_expected_event = null; });
        const { io, state } = createDriverIo(pages, { outputDir: options.outDir, inputLog, initialCursor: { x: options.width / 2, y: options.height / 2 } });
        const view = createView(pages, inputLog, serverDir, state);
        try {
            await driver.drive(io, params);
        } finally {
            await fs.writeFile(path.join(options.outDir, "input-log.json"), JSON.stringify(inputLog) + "\n");
            const ambient = await Promise.all(pages.map(async (page) => {
                try { return await page.evaluate(() => ({ ambient: window.__turts_io_ambient_events || [], downstream: window.__turts_io_downstream_events || [] })); }
                catch (error) { return { ambient: [{ kind: "capture-error", message: String(error) }], downstream: [] }; }
            }));
            await fs.writeFile(path.join(options.outDir, "ambient-events.json"), JSON.stringify(ambient) + "\n");
        }
        await oracle.judge(view, params);
        console.log(`PASS ${path.basename(options.oraclePath)}`);
    } catch (error) {
        console.error(`FAIL ${path.basename(options.oraclePath)}: ${error && error.message ? error.message : String(error)}`);
        process.exitCode = 1;
    } finally {
        await browser.close();
    }
}

if (require.main === module) run(parseArgs(process.argv.slice(2))).catch((error) => {
    console.error(`FAIL harness: ${error.message || String(error)}`);
    process.exitCode = 1;
});

module.exports = { deepFreeze, parseArgs };
