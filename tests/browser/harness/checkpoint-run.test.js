// turts-test-role: unit
"use strict";
const assert = require("node:assert/strict");
const fs = require("node:fs/promises");
const path = require("node:path");
const os = require("node:os");
const test = require("node:test");
const vm = require("node:vm");
const { createRequire } = require("node:module");

test("production runner overwrites driver-saved checkpoint forgeries before judging", async () => {
    const output = await fs.mkdtemp(path.join(os.tmpdir(), "mc-checkpoint-"));
    try {
        const paramsPath = path.join(output, "params.json");
        await fs.writeFile(paramsPath, JSON.stringify({ capture_scene_checkpoints: true }));
        const window = {
            turts_scene: () => JSON.stringify({ cameras_3d: 1, clip_from_world: [42] }),
            turts_debug: () => JSON.stringify({ phase: "Joined", last_frame: { frame_index: 7 } }),
        };
        const page = {
            on(event, handler) { if (event === "framenavigated") this.mainNavigation = handler; },
            mainFrame() { return this; },
            mouse: { move: async () => {} },
            goto: async () => { page.mainNavigation?.(page); },
            bringToFront: async () => {},
            waitForTimeout: async () => {},
            // Playwright does not await a predicate: a Promise result is true at once. The fake
            // refuses it, so an `async` predicate cannot pass here (review-l0-dbg.md, finding 1).
            waitForFunction: async (fn) => {
                const result = vm.runInNewContext(`(${fn.toString()})()`, { window });
                assert.equal(typeof result?.then, "undefined", "the predicate returned a Promise: Playwright would not await it");
                assert(result);
            },
            evaluate: async (fn, argument) => {
                if (String(fn).includes("window.turts_scene")) {
                    return {
                        scene: window.turts_scene(),
                        debug: window.turts_debug(),
                        browser: {
                            document_has_focus: true,
                            active_element: null,
                            canvas: null,
                            canvas_focused: false,
                            pointer_lock_element: null,
                            canvas_pointer_locked: false,
                        },
                    };
                }
                return vm.runInNewContext(`(${fn.toString()})(argument)`, { window, argument, setTimeout, clearTimeout,
                    performance: { timeOrigin: 1000 }, document: { URL: "http://fixture.invalid" } });
            },
            screenshot: async () => Buffer.from("actual screenshot pixels"),
        };
        let closed = false;
        let judged = false;
        const browser = {
            newContext: async () => ({ addInitScript: async () => {}, newPage: async () => page }),
            close: async () => { closed = true; },
        };
        const driver = { drive: async (io) => {
            assert.deepEqual(Object.keys(io), ["click", "drag", "key", "move", "wheel", "screenshot", "save", "wait", "reload", "use"]);
            await io.save("scene-checkpoints.json", Buffer.from("forged state"));
            assert.equal((await io.screenshot()).toString(), "actual screenshot pixels");
        } };
        const oracle = { judge: async (view) => {
            const rows = JSON.parse(await view.serverFile("scene-checkpoints.json"));
            assert.equal(rows.length, 1);
            assert.equal(rows[0].input_call, 1);
            assert.equal(rows[0].window, 1);
            assert.equal(rows[0].scene.clip_from_world[0], 42);
            assert.equal(rows[0].debug.last_frame.frame_index, 7);
            assert.equal(view.inputLog()[1].call, "screenshot");
            judged = true;
        } };
        const filename = path.join(__dirname, "run.js");
        const localRequire = createRequire(filename);
        const driverPath = path.join(output, "driver.js");
        const oraclePath = path.join(output, "oracle.js");
        const requireForRun = (name) => {
            if (name === "playwright-core") return { chromium: { launch: async () => browser } };
            if (name === driverPath) return driver;
            if (name === oraclePath) return oracle;
            return localRequire(name);
        };
        const errors = [];
        const sandbox = { __dirname: path.dirname(filename), require: requireForRun, module: { exports: {} }, process: { env: {} }, console: { log() {}, error(...args) { errors.push(args.join(" ")); } } };
        vm.runInNewContext((await fs.readFile(filename, "utf8")) + "\nmodule.exports.runForTest = run;", sandbox, { filename });
        await sandbox.module.exports.runForTest({ paramsPath, driverPath, oraclePath, outDir: output, url: "http://fixture.invalid", width: 1280, height: 720, deviceScaleFactor: 1 });
        assert.equal(sandbox.process.exitCode, undefined, `the production runner must not fail: ${errors.join("; ")}`);
        assert(judged, "the real runner must invoke the oracle after writing trusted snapshots");
        assert(closed, "the runner closes its owned browser");
    } finally {
        await fs.rm(output, { recursive: true, force: true });
    }
});
