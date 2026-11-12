// turts-test-role: instrument
// E-GPU (MP1 task E-GPU, design section 14 "One GPU for a two-client match"): two browser windows
// of one match on one GPU, with combat, and the frame record of each window.
//
// Usage: node egpu-run.js <page url with ?server=..&ws=..> <out dir> [--secs 600]
//
// It uses the browser profile of the MC scripts: MC_CHROME_ARGS_JSON, MC_CHROME_IGNORE_DEFAULT_ARGS_JSON,
// MC_BROWSER_HEADED (1: a headed window, as under mc_headed_v6.sh), CHROME_BIN. The harness
// (harness/run.js:129-150) reads the same variables.
//
// Frame figures: an init script wraps window.requestAnimationFrame in each window and pushes the
// timestamp of every callback while it is active. The wasm client runs one Bevy frame for each
// requestAnimationFrame callback (measure.js, header). The wrapper only observes: it adds no frame
// loop of its own. The figures are the intervals between those timestamps (egpu-stats.js).
// Measurement effect: this instrument also loads the page and drives it. It reads the page (frame
// array, turts_scene, turts_debug) once every 10 s, and it sends mouse and key input. The input and
// the reads cost the page a few ms each. No screenshot is taken during the measured period.
// The instrument reads the bridges turts_scene and turts_debug (role instrument: it is not a driver
// of the pixels-only rule).
//
// Combat: the package gives one armed slot (mods/muster/units/trooper.ron); the other slot owns
// unarmed targets. The slot-0 window selects one trooper and orders it to attack one target about
// every 50 s. The slot-1 window moves one of its targets about every 40 s. Both windows zoom.
//
// Exit: 0 PASS, 1 FAIL (the task's fail condition), 3 the run could not be set up (no verdict).
"use strict";

const fs = require("node:fs/promises");
const os = require("node:os");
const path = require("node:path");
const { chromium } = require("playwright-core");
const { summarize } = require("./egpu-stats.js");

const VIEW = { w: 1280, h: 720 };
// tests/browser/rows/MM1.json: map_size [64, 48], minimap_inset 8, minimap_size 200.
const MAP = { w: 64, h: 48 };
const MINIMAP = { inset: 8, size: 200 };
const SAMPLE_EVERY_S = 10;
const ATTACK_EVERY_S = 50;
const DUMMY_MOVE_EVERY_S = 40;
const ZOOM_EVERY_S = 30;
const FRAME_RECORDER = `(() => {
    const native = window.requestAnimationFrame.bind(window);
    const state = { frames: [], active: false };
    window.__egpu = state;
    window.requestAnimationFrame = (callback) => native((timestamp) => {
        if (state.active) state.frames.push(timestamp);
        return callback(timestamp);
    });
})();`;

const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
const nowIso = () => new Date().toISOString();

function parseArgs(argv) {
    const [url, outDir, ...rest] = argv;
    let secs = 600;
    for (let i = 0; i < rest.length; i += 1) {
        if (rest[i] === "--secs") { secs = Number(rest[i + 1]); i += 1; } else throw new Error(`unknown argument ${rest[i]}`);
    }
    if (!url || !outDir || !Number.isSafeInteger(secs) || secs < 10) throw new Error("usage: egpu-run.js <url> <out dir> [--secs N (at least 10)]");
    return { url, outDir, secs };
}

// A point that a click can reach: inside the view and outside the HUD zones (mm1-driver.js isHud).
function reachable(point) {
    if (!point) return false;
    const [x, y] = point;
    if (!(x > 20 && x < VIEW.w - 20 && y > 20 && y < VIEW.h - 20)) return false;
    return !((x < 300 && y < 330) || (x > VIEW.w - 320 && y < 180) || (x > VIEW.w - 220 && y > VIEW.h - 220) || y > VIEW.h - 40);
}

function minimapPoint(worldX, worldY) {
    const x0 = VIEW.w - MINIMAP.inset - MINIMAP.size;
    const y0 = VIEW.h - MINIMAP.inset - MINIMAP.size;
    return { x: Math.round(x0 + MINIMAP.size * (worldX / MAP.w)), y: Math.round(y0 + MINIMAP.size * (1 - worldY / MAP.h)) };
}

async function readState(page) {
    return page.evaluate(() => {
        const out = { hidden: document.hidden, visibility: document.visibilityState, error: window.turts_error || null };
        try {
            const debug = JSON.parse(window.turts_debug());
            const recent = debug.frames?.recent || [];
            out.phase = debug.phase ?? null;
            out.slot = Number.isSafeInteger(debug.slot) ? debug.slot : null;
            out.tick = debug.last_frame?.latest_tick ?? null;
            out.frame_index = recent.length ? recent[recent.length - 1].frame_index : null;
            out.has_last_frame = Boolean(debug.last_frame);
        } catch (error) { out.debug_error = String(error?.message || error); }
        try {
            const scene = JSON.parse(window.turts_scene());
            out.cameras_3d = scene.cameras_3d;
            out.render_adapter = scene.render_adapter ?? null;
            out.units = scene.units.map((unit) => ({
                id: unit.unit_id, x: unit.translation[0], y: unit.translation[2], screen: unit.ground_screen_center,
            }));
        } catch (error) { out.scene_error = String(error?.message || error); }
        return out;
    });
}

async function waitJoined(page, label, timeoutMs = 180000) {
    const started = Date.now();
    let last = null;
    while (Date.now() - started < timeoutMs) {
        last = await page.evaluate(() => typeof window.turts_debug === "function" && typeof window.turts_scene === "function").then(
            (present) => (present ? readState(page) : { phase: null }), () => ({ phase: null }));
        if (last.error) throw new Error(`${label}: browser client failed: ${last.error}`);
        if (typeof last.phase === "string" && last.phase.startsWith("Joined") && last.has_last_frame && last.cameras_3d === 1) return last;
        await sleep(250);
    }
    throw new Error(`${label}: not joined after ${timeoutMs} ms; last ${JSON.stringify({ ...last, units: undefined })}`);
}

async function run() {
    const { url, outDir, secs } = parseArgs(process.argv.slice(2));
    await fs.mkdir(outDir, { recursive: true });
    const chromeArgs = JSON.parse(process.env.MC_CHROME_ARGS_JSON || '["--enable-gpu","--use-angle=vulkan","--enable-features=Vulkan","--enable-unsafe-webgpu"]');
    if (!chromeArgs.some((arg) => arg.startsWith("--force-device-scale-factor"))) chromeArgs.push("--force-device-scale-factor=1");
    const headless = process.env.MC_BROWSER_HEADED !== "1";
    const browser = await chromium.launch({
        executablePath: process.env.CHROME_BIN || undefined,
        headless,
        args: chromeArgs,
        ignoreDefaultArgs: JSON.parse(process.env.MC_CHROME_IGNORE_DEFAULT_ARGS_JSON || '["--enable-unsafe-swiftshader"]'),
    });
    const events = [];
    const consoleNotes = [];
    let ending = false;
    let measuring = false;
    const windows = [];
    const result = { instrument: "tests/browser/egpu-run.js", started_utc: nowIso(), secs, headless, chrome_args: chromeArgs,
        chrome_version: browser.version(), display: process.env.DISPLAY ?? null, events, windows: [] };
    try {
        for (let n = 0; n < 2; n += 1) {
            const context = await browser.newContext({ viewport: { width: VIEW.w, height: VIEW.h }, deviceScaleFactor: 1 });
            await context.addInitScript(FRAME_RECORDER);
            const page = await context.newPage();
            const label = `window${n + 1}`;
            page.on("websocket", (ws) => {
                ws.on("close", () => events.push({ t_utc: nowIso(), window: n + 1, type: "websocket_close", url: ws.url().replace(/^(wss?:\/\/[^/]+).*/, "$1"), during_measure: measuring && !ending }));
                ws.on("socketerror", (error) => events.push({ t_utc: nowIso(), window: n + 1, type: "websocket_error", error: String(error), during_measure: measuring && !ending }));
            });
            page.on("pageerror", (error) => events.push({ t_utc: nowIso(), window: n + 1, type: "pageerror", text: String(error?.message || error).slice(0, 300) }));
            page.on("crash", () => events.push({ t_utc: nowIso(), window: n + 1, type: "page_crash", during_measure: measuring && !ending }));
            page.on("close", () => { if (!ending) events.push({ t_utc: nowIso(), window: n + 1, type: "page_close", during_measure: measuring }); });
            page.on("console", (message) => {
                if (["error", "warning"].includes(message.type()) && consoleNotes.length < 200) consoleNotes.push({ window: n + 1, type: message.type(), text: message.text().slice(0, 300) });
            });
            await page.goto(url, { waitUntil: "domcontentloaded" });
            await page.bringToFront();
            const joined = await waitJoined(page, label);
            windows.push({ n, page, label, slot: joined.slot, adapter: joined.render_adapter, start_state: joined, frames: [], origin: null, start_epoch: null, samples: [] });
        }
        // Setup is done. Reset the frame arrays and start the clock in both windows together.
        const startStates = await Promise.all(windows.map((w) => readState(w.page)));
        const initial = startStates.map((state) => state.units);
        await Promise.all(windows.map((w) => w.page.screenshot({ path: path.join(outDir, `${w.label}-start.png`) })));
        const t0 = Date.now();
        const activation = await Promise.all(windows.map((w) => w.page.evaluate(() => {
            window.__egpu.frames.length = 0;
            window.__egpu.active = true;
            return { origin: performance.timeOrigin, now: performance.now() };
        })));
        measuring = true;
        windows.forEach((w, i) => { w.origin = activation[i].origin; w.start_epoch = activation[i].origin + activation[i].now; w.start_frame_index = startStates[i].frame_index; });
        result.measure_start_utc = nowIso();

        const bySlot = (slot) => windows.find((w) => w.slot === slot);
        const attacker = bySlot(0) || windows[0];
        const mover = bySlot(1) || windows[1];
        result.roles = { attacker_window: attacker.n + 1, attacker_slot: attacker.slot, mover_window: mover.n + 1, mover_slot: mover.slot };
        // Unit sets, from the first scene read, by the muster map (mods/muster/maps/field.ron): slot-0 troopers
        // stand at x 25 to 33, y 18 to 23; slot-1 dummies and the post stand at x 54 and more.
        const first = initial[attacker.n];
        const foreignIds = first.filter((u) => u.x >= 50).map((u) => u.id).sort((a, b) => a - b);
        const trooperIds = first.filter((u) => u.x < 34 && u.y > 17 && u.y < 23.5).map((u) => u.id);
        const trooperId = first.filter((u) => trooperIds.includes(u.id)).sort((a, b) => b.x - a.x || a.id - b.id)[0]?.id ?? null;
        result.sets = { foreign_ids: foreignIds, trooper_count: trooperIds.length, chosen_trooper: trooperId };
        const combat = [];
        result.combat = combat;

        const front = async (w) => { await w.page.bringToFront(); };
        async function focusWorld(w, wx, wy) {
            const p = minimapPoint(wx, wy);
            await w.page.mouse.click(p.x, p.y, { button: "left" });
            await sleep(400);
        }
        async function attackNext(second) {
            await front(attacker);
            let state = await readState(attacker.page);
            const alive = foreignIds.filter((id) => state.units.some((u) => u.id === id));
            const entry = { second, foreign_alive_before: alive.length };
            if (alive.length === 0 || trooperId === null) { entry.outcome = alive.length === 0 ? "no foreign unit left" : "no trooper"; combat.push(entry); return; }
            const targetId = alive[0];
            let target = state.units.find((u) => u.id === targetId);
            let me = state.units.find((u) => u.id === trooperId);
            if (!reachable(me?.screen) && !reachable(target?.screen)) { await focusWorld(attacker, target.x, target.y); state = await readState(attacker.page); target = state.units.find((u) => u.id === targetId); me = state.units.find((u) => u.id === trooperId); }
            if (reachable(me?.screen)) { await attacker.page.mouse.click(me.screen[0], me.screen[1], { button: "left" }); await sleep(120); }
            else entry.select_skipped = "trooper not on screen";
            if (!reachable(target?.screen)) { await focusWorld(attacker, target.x, target.y); state = await readState(attacker.page); target = state.units.find((u) => u.id === targetId); }
            if (!reachable(target?.screen)) { entry.outcome = "target not reachable on screen"; combat.push(entry); return; }
            await attacker.page.mouse.click(target.screen[0], target.screen[1], { button: "right" });
            entry.target = targetId; entry.outcome = "attack ordered"; combat.push(entry);
        }
        async function moveDummy(second) {
            await front(mover);
            const state = await readState(mover.page);
            const dummy = state.units.filter((u) => foreignIds.includes(u.id) && reachable(u.screen))[0];
            const entry = { second, window: mover.n + 1 };
            if (!dummy) {
                const any = state.units.find((u) => foreignIds.includes(u.id));
                if (any) await focusWorld(mover, any.x, any.y);
                entry.outcome = any ? "camera focused on a target" : "no target of this slot left";
                combat.push(entry); return;
            }
            await mover.page.mouse.click(dummy.screen[0], dummy.screen[1], { button: "left" });
            await sleep(120);
            const dx = (second / DUMMY_MOVE_EVERY_S) % 2 === 0 ? 70 : -70;
            const dest = [Math.min(Math.max(dummy.screen[0] + dx, 40), VIEW.w - 40), Math.min(Math.max(dummy.screen[1] + 40, 40), VIEW.h - 60)];
            await mover.page.mouse.click(dest[0], dest[1], { button: "right" });
            entry.outcome = "dummy move ordered"; entry.unit = dummy.id; combat.push(entry);
        }
        async function zoom(second) {
            for (const w of windows) {
                await front(w);
                await w.page.mouse.move(VIEW.w / 2, VIEW.h / 2);
                await w.page.mouse.wheel(0, (second / ZOOM_EVERY_S) % 2 === 0 ? -240 : 240);
            }
        }
        async function drain() {
            for (const w of windows) {
                const got = await w.page.evaluate(() => window.__egpu.frames.splice(0));
                for (const ts of got) w.frames.push(w.origin + ts - w.start_epoch);
            }
        }
        async function sample(second) {
            const states = await Promise.all(windows.map((w) => readState(w.page)));
            states.forEach((state, i) => {
                const w = windows[i];
                const foreignAlive = state.units ? foreignIds.filter((id) => state.units.some((u) => u.id === id)).length : null;
                w.samples.push({ second, phase: state.phase, tick: state.tick, frame_index: state.frame_index, hidden: state.hidden,
                    visibility: state.visibility, units: state.units ? state.units.length : null, foreign_alive: foreignAlive,
                    debug_error: state.debug_error ?? null, scene_error: state.scene_error ?? null });
                if (typeof state.phase !== "string" || !state.phase.startsWith("Joined")) {
                    events.push({ t_utc: nowIso(), window: i + 1, type: "phase_not_joined", phase: state.phase ?? null, during_measure: true });
                }
            });
        }

        for (let second = 0; second < secs; second += 1) {
            const due = t0 + second * 1000;
            const wait = due - Date.now();
            if (wait > 0) await sleep(wait);
            // An action that throws is recorded and the run goes on: the frame record must not stop for a combat step.
            const guarded = async (name, action) => {
                try { await action(second); } catch (error) { combat.push({ second, step: name, error: String(error?.message || error).slice(0, 200) }); }
            };
            if (second % SAMPLE_EVERY_S === 0) {
                await drain(); await sample(second);
                console.log(`EGPU_PROGRESS second=${second} frames=${windows.map((w) => w.frames.length).join(",")} foreign_alive=${windows[0].samples.at(-1)?.foreign_alive}`);
            }
            if (second % ATTACK_EVERY_S === 1) await guarded("attack", attackNext);
            if (second % DUMMY_MOVE_EVERY_S === 21) await guarded("dummy_move", moveDummy);
            if (second % ZOOM_EVERY_S === 11) await guarded("zoom", zoom);
        }
        const tEnd = Date.now();
        // Stop the clock first, so that the final reads do not count as frames of the period.
        await Promise.all(windows.map((w) => w.page.evaluate(() => { window.__egpu.active = false; })));
        await drain();
        const endStates = await Promise.all(windows.map((w) => readState(w.page)));
        measuring = false;
        result.measure_end_utc = nowIso();
        result.measured_s = (tEnd - t0) / 1000;
        ending = true;
        await Promise.all(windows.map((w) => w.page.screenshot({ path: path.join(outDir, `${w.label}-end.png`) })));
        for (const [i, w] of windows.entries()) {
            const summary = summarize(w.frames, secs);
            const appFrames = endStates[i].frame_index != null && w.start_frame_index != null ? endStates[i].frame_index - w.start_frame_index : null;
            result.windows.push({
                window: w.n + 1, slot: w.slot, render_adapter: w.adapter, summary,
                app_frame_index_delta: appFrames, raf_callbacks: w.frames.length,
                end_phase: endStates[i].phase, visibility_samples: w.samples.map((s) => s.visibility),
                hidden_samples: w.samples.filter((s) => s.hidden).length, samples: w.samples,
            });
            await fs.writeFile(path.join(outDir, `${w.label}-frame-times-ms.json`), JSON.stringify(w.frames.map((v) => Math.round(v * 100) / 100)));
        }
        // Chrome GPU page, after the measured period (not counted).
        try {
            const gpu = await windows[0].page.context().newPage();
            await gpu.goto("chrome://gpu", { timeout: 15000 });
            await sleep(3000);
            await fs.writeFile(path.join(outDir, "chrome-gpu.txt"), await gpu.evaluate(() => document.body.innerText));
            await gpu.close();
        } catch (error) { result.chrome_gpu_error = String(error?.message || error).slice(0, 200); }
    } catch (error) {
        result.setup_error = String(error?.stack || error).slice(0, 2000);
    }
    ending = true;
    result.console_notes = consoleNotes;
    result.host = { cpu_model: os.cpus()[0]?.model, cpus: os.cpus().length, mem_gb: Math.round(os.totalmem() / 2 ** 30), os: `${os.type()} ${os.release()}` };
    result.ended_utc = nowIso();
    const disconnects = events.filter((e) => e.during_measure && ["websocket_close", "page_close", "page_crash", "phase_not_joined"].includes(e.type));
    result.disconnect_events = disconnects.length;
    const judged = result.windows.length === 2;
    result.verdict = !judged || result.setup_error ? "NO_VERDICT" : (result.windows.some((w) => w.summary.starved) || disconnects.length > 0 ? "FAIL" : "PASS");
    await fs.writeFile(path.join(outDir, "egpu-result.json"), JSON.stringify(result, null, 2) + "\n");
    await browser.close().catch(() => {});
    const line = { verdict: result.verdict, disconnect_events: result.disconnect_events, windows: result.windows.map((w) => ({ window: w.window, frames: w.summary.frames, p50: w.summary.interval_ms.p50, p95: w.summary.interval_ms.p95, lowest_5s_fps: w.summary.lowest_5s_fps })) };
    console.log(`EGPU_RESULT ${JSON.stringify(line)}`);
    console.log(`VERDICT=${result.verdict}`);
    process.exitCode = result.verdict === "PASS" ? 0 : (result.verdict === "FAIL" ? 1 : 3);
}

run().catch((error) => { console.error(error); process.exitCode = 3; });
