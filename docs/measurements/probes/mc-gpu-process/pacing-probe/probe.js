"use strict";
// Pacing probe (scratch, not in the repository). Two real game clients join; then a third
// window opens. The probe records the presented frame times of each window (one
// getCurrentTexture call per Bevy frame), the requestDevice time of the third window, and the
// replica tick that each window presents.
// Usage: node probe.js <out.jsonl> <dist dir> <server binary> <package tar> <variant>...
// Variant names: base, raf2, raf4, blur (see VARIANTS).
const http = require("node:http");
const fs = require("node:fs");
const path = require("node:path");
const net = require("node:net");
// The sound isolation of the CEO sound rule (2026-11-12): the null ALSA device as an argument and the silent
// environment, from the shared helper. Added after the measurement; the figures came from the old file.
const { silentEnv, SILENT_ARGS } = require(path.join(__dirname, "../../../../tests/browser/harness/silent-chrome.js"));
const { spawn, spawnSync } = require("node:child_process");
const gpuUtil = () => { try { return spawnSync("nvidia-smi", ["--query-gpu=utilization.gpu", "--format=csv,noheader,nounits"], { encoding: "utf8" }).stdout.trim(); } catch (e) { return String(e); } };
const { chromium } = require(process.env.PW_CORE || "/home/julien/Projects/TuRTS-mc-fix-pacing/tests/browser/node_modules/playwright-core");

const [outPath, distDir, serverBin, packageTar, ...variantNames] = process.argv.slice(2);

const VARIANTS = {
    base: { rafDiv: 0, blur: false },
    cap30: { rafDiv: 33.3, blur: false },
    cap20: { rafDiv: 50, blur: false },
    cap15: { rafDiv: 66.7, blur: false },
    cap10: { rafDiv: 100, blur: false },
    blur: { rafDiv: 0, blur: true },
    // "blur3": all three windows blurred is not possible before window 3 starts; owners only.
};

function instrument(rafDiv) {
    const frames = [];
    window.__pf = frames;
    window.__rd = [];
    window.__ra = [];
    const gct = GPUCanvasContext.prototype.getCurrentTexture;
    GPUCanvasContext.prototype.getCurrentTexture = function (...args) {
        frames.push(performance.now());
        if (frames.length > 40000) frames.splice(0, 20000);
        return gct.apply(this, args);
    };
    const rd = GPUAdapter.prototype.requestDevice;
    GPUAdapter.prototype.requestDevice = function (...args) {
        const rec = { start: performance.now(), end: null, state: "pending" };
        window.__rd.push(rec);
        const p = rd.apply(this, args);
        p.then(() => { rec.end = performance.now(); rec.state = "fulfilled"; },
            (e) => { rec.end = performance.now(); rec.state = "rejected: " + e; });
        return p;
    };
    const ra = GPU.prototype.requestAdapter;
    GPU.prototype.requestAdapter = function (...args) {
        const rec = { start: performance.now(), end: null };
        window.__ra.push(rec);
        const p = ra.apply(this, args);
        p.then(() => { rec.end = performance.now(); }, () => { rec.end = performance.now(); });
        return p;
    };
    window.__calls = { draw: 0, writeBuffer: 0, writeTexture: 0, submit: 0, passes: 0 };
    const count = (proto, name, key) => {
        const f = proto && proto[name];
        if (typeof f !== "function") return;
        proto[name] = function (...args) { window.__calls[key] += 1; return f.apply(this, args); };
    };
    count(GPURenderPassEncoder.prototype, "draw", "draw");
    count(GPURenderPassEncoder.prototype, "drawIndexed", "draw");
    count(GPURenderPassEncoder.prototype, "drawIndirect", "draw");
    count(GPURenderPassEncoder.prototype, "drawIndexedIndirect", "draw");
    count(GPUQueue.prototype, "writeBuffer", "writeBuffer");
    count(GPUQueue.prototype, "writeTexture", "writeTexture");
    count(GPUQueue.prototype, "submit", "submit");
    count(GPUCommandEncoder.prototype, "beginRenderPass", "passes");
    if (rafDiv > 0) {
        // Simulates a frame cap: a callback runs at the first native frame that is at least
        // rafDiv ms after the last delivered frame. Robust to cancel and re-request.
        const native = window.requestAnimationFrame.bind(window);
        const nativeCancel = window.cancelAnimationFrame.bind(window);
        let nextId = 1;
        let last = -1e9;
        const tickets = new Map();
        window.requestAnimationFrame = (cb) => {
            const id = nextId++;
            const t = { native: null };
            const step = (ts) => {
                if (ts - last >= rafDiv - 2) { tickets.delete(id); last = ts; cb(ts); }
                else { t.native = native(step); }
            };
            t.native = native(step);
            tickets.set(id, t);
            return id;
        };
        window.cancelAnimationFrame = (id) => {
            const t = tickets.get(id);
            if (t) { nativeCancel(t.native); tickets.delete(id); } else { nativeCancel(id); }
        };
    }
}

function stats(times, fromMs, toMs) {
    const t = times.filter((x) => x >= fromMs && x <= toMs);
    const d = [];
    for (let i = 1; i < t.length; i += 1) d.push(t[i] - t[i - 1]);
    d.sort((a, b) => a - b);
    const q = (p) => (d.length ? Number(d[Math.min(d.length - 1, Math.floor(p * d.length))].toFixed(2)) : null);
    return { frames: t.length, fps: Number(((t.length - 1) / ((toMs - fromMs) / 1000)).toFixed(1)), p50_ms: q(0.5), p95_ms: q(0.95), max_ms: d.length ? Number(d[d.length - 1].toFixed(1)) : null };
}

function descendants(root) {
    const parent = {};
    for (const pid of fs.readdirSync("/proc")) {
        if (!/^\d+$/.test(pid)) continue;
        try { const st = fs.readFileSync(`/proc/${pid}/stat`, "utf8"); parent[pid] = st.slice(st.lastIndexOf(")") + 2).split(" ")[1]; } catch {}
    }
    const set = new Set([String(root)]);
    let grew = true;
    while (grew) { grew = false; for (const [pid, pp] of Object.entries(parent)) if (!set.has(pid) && set.has(pp)) { set.add(pid); grew = true; } }
    return set;
}

let ROOT_PID = null;
function cpuSample() {
    const out = {};
    const tree = descendants(ROOT_PID);
    for (const pid of tree) {
        try {
            const cmd = fs.readFileSync(`/proc/${pid}/cmdline`, "utf8").split("\0").join(" ");
            const m = cmd.match(/--type=(\S+)/);
            const util = cmd.match(/--utility-sub-type=(\S+)/);
            const type = m ? m[1] + (util ? "/" + util[1] : "") : "browser";
            for (const tid of fs.readdirSync(`/proc/${pid}/task`)) {
                try {
                    const st = fs.readFileSync(`/proc/${pid}/task/${tid}/stat`, "utf8");
                    const comm = st.slice(st.indexOf("(") + 1, st.lastIndexOf(")"));
                    const f = st.slice(st.lastIndexOf(")") + 2).split(" ");
                    out[`${pid}/${tid}`] = { type, comm, ticks: Number(f[11]) + Number(f[12]) };
                } catch {}
            }
        } catch {}
    }
    return out;
}

function cpuDelta(a, b, secs) {
    const hz = 100;
    const proc = {};
    const threads = [];
    for (const key of Object.keys(b)) {
        if (!a[key]) continue;
        const pct = ((b[key].ticks - a[key].ticks) / hz / secs) * 100;
        const pid = key.split("/")[0];
        const pk = `${b[key].type}:${pid}`;
        proc[pk] = (proc[pk] || 0) + pct;
        if (pct >= 15) threads.push(`${b[key].type}:${pid}:${b[key].comm}=${pct.toFixed(0)}`);
    }
    for (const k of Object.keys(proc)) { if (proc[k] < 2) delete proc[k]; else proc[k] = Number(proc[k].toFixed(1)); }
    return { proc, threads };
}

async function freePorts(n) {
    const servers = [];
    const ports = [];
    for (let i = 0; i < n; i += 1) {
        const s = net.createServer();
        await new Promise((r) => s.listen(0, "127.0.0.1", r));
        ports.push(s.address().port);
        servers.push(s);
    }
    for (const s of servers) await new Promise((r) => s.close(r));
    return ports;
}

function staticServer(dir, port) {
    const types = { ".html": "text/html", ".js": "text/javascript", ".wasm": "application/wasm", ".css": "text/css" };
    const server = http.createServer((req, res) => {
        const url = new URL(req.url, "http://x");
        let file = path.join(dir, decodeURIComponent(url.pathname));
        if (!file.startsWith(dir)) { res.writeHead(403); res.end(); return; }
        if (fs.existsSync(file) && fs.statSync(file).isDirectory()) file = path.join(file, "index.html");
        fs.readFile(file, (err, data) => {
            if (err) { res.writeHead(404); res.end(); return; }
            res.writeHead(200, { "content-type": types[path.extname(file)] || "application/octet-stream" });
            res.end(data);
        });
    });
    return new Promise((r) => server.listen(port, "127.0.0.1", () => r(server)));
}

async function debugState(page) {
    return page.evaluate(() => {
        try {
            const d = JSON.parse(window.turts_debug());
            return { phase: d.phase, hi_tick: d.last_frame?.hi_tick ?? null, error: window.turts_error || null, now: performance.now(), focus: document.hasFocus(), active: document.activeElement?.id || document.activeElement?.tagName };
        } catch (e) { return { phase: null, error: window.turts_error || String(e), now: performance.now() }; }
    });
}

async function waitJoined(page, timeout) {
    await page.waitForFunction(() => {
        if (window.turts_error) return true;
        try { const d = JSON.parse(window.turts_debug()); return typeof d.phase === "string" && (d.phase.startsWith("Joined") || d.phase.startsWith("Failed")) && (d.phase.startsWith("Failed") || Boolean(d.last_frame)); } catch { return false; }
    }, null, { timeout, polling: 100 });
}

async function phase(name) {
    const v = VARIANTS[name];
    const [staticPort, httpPort, wsPort] = await freePorts(3);
    const work = fs.mkdtempSync(path.join(path.dirname(outPath), `run-${name}-`));
    fs.writeFileSync(path.join(work, "server.ron"), `(
    game_package: "${packageTar}",
    http_listen: "127.0.0.1:${httpPort}",
    ws_listen: "127.0.0.1:${wsPort}",
    public_url: "http://127.0.0.1:${httpPort}",
    tick_hz: 20,
    static_dir: "${distDir}",
)
`);
    const serverLog = fs.openSync(path.join(work, "server.log"), "w");
    const server = spawn(serverBin, [path.join(work, "server.ron")], { stdio: ["ignore", serverLog, serverLog] });
    const stat = await staticServer(distDir, staticPort);
    const out = { name, variant: v };
    let browser;
    try {
        for (let i = 0; i < 100; i += 1) {
            try { const r = await fetch(`http://127.0.0.1:${httpPort}/advertisement.json`); if (r.ok) break; } catch {}
            await new Promise((r) => setTimeout(r, 100));
        }
        const url = `http://127.0.0.1:${staticPort}/?server=${encodeURIComponent(`http://127.0.0.1:${httpPort}`)}&ws=${encodeURIComponent(`ws://127.0.0.1:${wsPort}`)}`;
        browser = await chromium.launch({
            executablePath: "/usr/bin/google-chrome", headless: false,
            args: [...SILENT_ARGS, ...JSON.parse(process.env.MC_CHROME_ARGS_JSON)],
            env: silentEnv(work),
            ignoreDefaultArgs: JSON.parse(process.env.MC_CHROME_IGNORE_DEFAULT_ARGS_JSON || '["--enable-unsafe-swiftshader"]'),
        });
        ROOT_PID = process.pid;
        const pages = [];
        for (let n = 0; n < 2; n += 1) {
            const ctx = await browser.newContext({ viewport: { width: 1280, height: 720 } });
            await ctx.addInitScript(instrument, v.rafDiv);
            const page = await ctx.newPage();
            await page.mouse.move(640, 360);
            await page.goto(url, { waitUntil: "domcontentloaded" });
            await page.bringToFront();
            await waitJoined(page, 30000);
            pages.push(page);
        }
        if (v.blur) {
            for (const p of pages) await p.evaluate(() => { const c = document.getElementById("turts"); c.focus(); c.blur(); });
        }
        out.owner_device = await Promise.all(pages.map((p) => p.evaluate(() => window.__rd)));
        await pages[0].waitForTimeout(1000);
        const t0 = await Promise.all(pages.map((p) => p.evaluate(() => performance.now())));
        const s0 = await Promise.all(pages.map(debugState));
        const cpu0 = cpuSample();
        const c0 = await Promise.all(pages.map((p) => p.evaluate(() => ({ ...window.__calls }))));
        await pages[0].waitForTimeout(5000);
        const cpu1 = cpuSample();
        out.gpu_util_owners = gpuUtil();
        const c1 = await Promise.all(pages.map((p) => p.evaluate(() => ({ ...window.__calls }))));
        out.cpu_pct = cpuDelta(cpu0, cpu1, 5);
        out.calls_per_frame = null;
        const t1 = await Promise.all(pages.map((p) => p.evaluate(() => performance.now())));
        const s1 = await Promise.all(pages.map(debugState));
        const times = await Promise.all(pages.map((p) => p.evaluate(() => window.__pf.slice())));
        out.owners_before = pages.map((_, i) => ({ ...stats(times[i], t0[i], t1[i]), tick_rate: s1[i].hi_tick != null && s0[i].hi_tick != null ? Number(((s1[i].hi_tick - s0[i].hi_tick) / ((s1[i].now - s0[i].now) / 1000)).toFixed(1)) : null, phase: s1[i].phase, focus: s1[i].focus, active: s1[i].active }));
        out.calls_per_s = c1.map((c, i) => Object.fromEntries(Object.keys(c).map((k) => [k, Number(((c[k] - c0[i][k]) / 5).toFixed(1))])));
        out.owner_tick_gap = s1[1].hi_tick != null && s1[0].hi_tick != null ? s1[1].hi_tick - s1[0].hi_tick : null;
        // Window 3: never instrumented with a rAF divider; it is the new client.
        const ctx3 = await browser.newContext({ viewport: { width: 1280, height: 720 } });
        await ctx3.addInitScript(instrument, 0);
        const page3 = await ctx3.newPage();
        await page3.mouse.move(640, 360);
        const opened = Date.now();
        const o0 = await Promise.all(pages.map((p) => p.evaluate(() => performance.now())));
        const cpuT0 = cpuSample();
        const cpuT0ms = Date.now();
        await page3.goto(url, { waitUntil: "domcontentloaded" });
        await page3.bringToFront();
        let settled = false;
        try {
            await page3.waitForFunction(() => window.__rd.length > 0 && window.__rd.every((r) => r.state !== "pending"), null, { timeout: 30000, polling: 50 });
            settled = true;
        } catch {}
        out.third_host_ms = Date.now() - opened;
        out.cpu_during_third = cpuDelta(cpuT0, cpuSample(), (Date.now() - cpuT0ms) / 1000);
        out.gpu_util_during_third = gpuUtil();
        out.third_device = await page3.evaluate(() => ({ rd: window.__rd, ra: window.__ra }));
        out.third_settled = settled;
        if (settled) {
            try { await waitJoined(page3, 20000); } catch {}
            out.third_state = await debugState(page3);
            await page3.waitForTimeout(3000);
            const p3 = await page3.evaluate(() => [performance.now(), window.__pf.slice()]);
            out.third_frames = stats(p3[1], p3[0] - 3000, p3[0]);
        }
        const o1 = await Promise.all(pages.map((p) => p.evaluate(() => performance.now())));
        const times2 = await Promise.all(pages.map((p) => p.evaluate(() => window.__pf.slice())));
        const s2 = await Promise.all(pages.map(debugState));
        out.owners_during_third = pages.map((_, i) => ({ ...stats(times2[i], o0[i], o1[i]), phase: s2[i].phase, hi_tick: s2[i].hi_tick }));
    } catch (e) {
        out.error = String(e && e.stack || e);
    } finally {
        if (browser) await browser.close().catch(() => {});
        stat.close();
        server.kill("SIGTERM");
    }
    fs.appendFileSync(outPath, JSON.stringify(out) + "\n");
    console.log(JSON.stringify(out));
}

(async () => {
    for (const name of variantNames) await phase(name);
})().catch((e) => { console.error(e); process.exit(1); });
