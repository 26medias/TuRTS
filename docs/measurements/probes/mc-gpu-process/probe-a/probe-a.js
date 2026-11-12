"use strict";
// Probe A (scratch, not in the repository). Splits the GPU-process main-thread cost per presented
// frame between the present path and the client work. Each phase launches its own Chrome (fresh GPU
// process) with the v6 wrapper flags (MC_CHROME_ARGS_JSON), opens 1 or 2 pages, measures 5 s of CPU
// per thread from /proc, then records 5 s of `perf record -t <gpu main tid>`.
// Usage: node probe-a.js <out dir> <dist dir> <server bin> <package tar> <phase>...
// Phases: e (real client x2), a (clear 1280x720 x2), b (clear 640x360 x2), c (work 1280x720 x2),
//         d (clear 1280x720 x1), f (a without --disable-vulkan-surface), e1 (real client x1).
const http = require("node:http");
const fs = require("node:fs");
const path = require("node:path");
const net = require("node:net");
const { spawn, spawnSync } = require("node:child_process");
const { chromium } = require(process.env.PW_CORE || "/home/julien/Projects/TuRTS-mc-pacing-review/tests/browser/node_modules/playwright-core");

const [outDir, distDir, serverBin, packageTar, ...phases] = process.argv.slice(2);
const pageDir = path.join(__dirname, "page");
const HZ = 100;
const MEASURE_S = 5;
const PERF_S = 5;
let measuredWrites = null;

const PHASES = {
    e: { kind: "client", pages: 2, w: 1280, h: 720 },
    e1: { kind: "client", pages: 1, w: 1280, h: 720 },
    a: { kind: "probe", mode: "clear", pages: 2, w: 1280, h: 720 },
    b: { kind: "probe", mode: "clear", pages: 2, w: 640, h: 360 },
    c: { kind: "probe", mode: "work", pages: 2, w: 1280, h: 720 },
    d: { kind: "probe", mode: "clear", pages: 1, w: 1280, h: 720 },
    f: { kind: "probe", mode: "clear", pages: 2, w: 1280, h: 720, dropArgs: ["--disable-vulkan-surface"] },
};

function clientInstrument() {
    const p = { frames: 0, times: [], calls: { draw: 0, writeBuffer: 0, writeBytes: 0, submit: 0, passes: 0 }, perFrame: [], cur: null };
    window.__probe = p;
    const gct = GPUCanvasContext.prototype.getCurrentTexture;
    GPUCanvasContext.prototype.getCurrentTexture = function (...args) {
        p.frames += 1; p.times.push(performance.now());
        if (p.times.length > 20000) p.times.splice(0, 10000);
        if (p.cur) { p.perFrame.push(p.cur); if (p.perFrame.length > 600) p.perFrame.splice(0, 300); }
        p.cur = { writes: [], draws: 0, passes: 0, submits: 0 };
        return gct.apply(this, args);
    };
    const wrap = (proto, name, fn) => { const f = proto[name]; if (typeof f !== "function") return; proto[name] = function (...a) { fn(a); return f.apply(this, a); }; };
    for (const n of ["draw", "drawIndexed", "drawIndirect", "drawIndexedIndirect"]) wrap(GPURenderPassEncoder.prototype, n, () => { p.calls.draw += 1; if (p.cur) p.cur.draws += 1; });
    wrap(GPUCommandEncoder.prototype, "beginRenderPass", () => { p.calls.passes += 1; if (p.cur) p.cur.passes += 1; });
    wrap(GPUQueue.prototype, "submit", () => { p.calls.submit += 1; if (p.cur) p.cur.submits += 1; });
    wrap(GPUQueue.prototype, "writeBuffer", (a) => {
        const data = a[2]; const el = data && data.BYTES_PER_ELEMENT ? data.BYTES_PER_ELEMENT : 1;
        const total = data ? data.byteLength / el : 0;
        const bytes = ((a[4] !== undefined ? a[4] : total - (a[3] || 0))) * el;
        p.calls.writeBuffer += 1; p.calls.writeBytes += bytes;
        if (p.cur) p.cur.writes.push(bytes);
    });
}

function procTable() {
    const t = {};
    for (const pid of fs.readdirSync("/proc")) {
        if (!/^\d+$/.test(pid)) continue;
        try { const st = fs.readFileSync(`/proc/${pid}/stat`, "utf8"); t[pid] = st.slice(st.lastIndexOf(")") + 2).split(" ")[1]; } catch {}
    }
    return t;
}
function descendants(root) {
    const parent = procTable(); const set = new Set([String(root)]); let grew = true;
    while (grew) { grew = false; for (const [pid, pp] of Object.entries(parent)) if (!set.has(pid) && set.has(pp)) { set.add(pid); grew = true; } }
    return set;
}
function cpuSample() {
    const out = {};
    for (const pid of descendants(process.pid)) {
        try {
            const cmd = fs.readFileSync(`/proc/${pid}/cmdline`, "utf8").split("\0").join(" ");
            const m = cmd.match(/--type=(\S+)/); const u = cmd.match(/--utility-sub-type=(\S+)/);
            const type = m ? m[1] + (u ? "/" + u[1] : "") : (cmd.includes("chrome") ? "browser" : "other");
            for (const tid of fs.readdirSync(`/proc/${pid}/task`)) {
                try {
                    const st = fs.readFileSync(`/proc/${pid}/task/${tid}/stat`, "utf8");
                    const comm = st.slice(st.indexOf("(") + 1, st.lastIndexOf(")"));
                    const f = st.slice(st.lastIndexOf(")") + 2).split(" ");
                    out[`${pid}/${tid}`] = { pid, tid, type, comm, ticks: Number(f[11]) + Number(f[12]) };
                } catch {}
            }
        } catch {}
    }
    return out;
}
function cpuDelta(a, b, secs) {
    const threads = [];
    for (const k of Object.keys(b)) {
        if (!a[k]) continue;
        const pct = ((b[k].ticks - a[k].ticks) / HZ / secs) * 100;
        threads.push({ ...b[k], pct: Number(pct.toFixed(1)) });
    }
    threads.sort((x, y) => y.pct - x.pct);
    return threads;
}
const gpuUtil = () => { try { return spawnSync("nvidia-smi", ["--query-gpu=utilization.gpu", "--format=csv,noheader,nounits"], { encoding: "utf8" }).stdout.trim(); } catch (e) { return String(e); } };
async function freePorts(n) {
    const ss = [], ports = [];
    for (let i = 0; i < n; i += 1) { const s = net.createServer(); await new Promise((r) => s.listen(0, "127.0.0.1", r)); ports.push(s.address().port); ss.push(s); }
    for (const s of ss) await new Promise((r) => s.close(r));
    return ports;
}
function staticServer(dir, port) {
    const types = { ".html": "text/html", ".js": "text/javascript", ".wasm": "application/wasm", ".css": "text/css" };
    const server = http.createServer((req, res) => {
        const url = new URL(req.url, "http://x");
        let file = path.join(dir, decodeURIComponent(url.pathname));
        if (!file.startsWith(dir)) { res.writeHead(403); res.end(); return; }
        if (fs.existsSync(file) && fs.statSync(file).isDirectory()) file = path.join(file, "index.html");
        fs.readFile(file, (err, data) => { if (err) { res.writeHead(404); res.end(); return; } res.writeHead(200, { "content-type": types[path.extname(file)] || "application/octet-stream" }); res.end(data); });
    });
    return new Promise((r) => server.listen(port, "127.0.0.1", () => r(server)));
}
function perfRun(tid, file) {
    const rec = spawnSync("perf", ["record", "-F", "999", "-t", String(tid), "-o", file, "--", "sleep", String(PERF_S)], { encoding: "utf8" });
    const out = { record_exit: rec.status, record_stderr: (rec.stderr || "").split("\n").filter((l) => /samples|error|Error|warn/i.test(l)).slice(0, 5) };
    const dso = spawnSync("perf", ["report", "-i", file, "--stdio", "--sort", "dso", "-q"], { encoding: "utf8", maxBuffer: 1 << 26, timeout: 120000 });
    out.by_dso = (dso.stdout || "").split("\n").map((l) => l.trim()).filter(Boolean).slice(0, 15);
    const sym = spawnSync("perf", ["report", "-i", file, "--stdio", "--sort", "dso,sym", "-q"], { encoding: "utf8", maxBuffer: 1 << 26, timeout: 120000 });
    out.by_sym = (sym.stdout || "").split("\n").map((l) => l.trim()).filter(Boolean).slice(0, 25);
    return out;
}
function frameStats(times, from, to) {
    const t = times.filter((x) => x >= from && x <= to); const d = [];
    for (let i = 1; i < t.length; i += 1) d.push(t[i] - t[i - 1]);
    d.sort((x, y) => x - y);
    const q = (p) => (d.length ? Number(d[Math.min(d.length - 1, Math.floor(p * d.length))].toFixed(2)) : null);
    return { frames: t.length, fps: Number(((t.length - 1) / ((to - from) / 1000)).toFixed(1)), p50_ms: q(0.5), p95_ms: q(0.95) };
}

async function runPhase(name) {
    const v = PHASES[name];
    const out = { phase: name, spec: v, utc: new Date().toISOString() };
    const [sPort, hPort, wPort] = await freePorts(3);
    let server = null, stat = null, browser = null;
    const work = fs.mkdtempSync(path.join(outDir, `run-${name}-`));
    try {
        let url;
        if (v.kind === "client") {
            fs.writeFileSync(path.join(work, "server.ron"), `(\n    game_package: "${packageTar}",\n    http_listen: "127.0.0.1:${hPort}",\n    ws_listen: "127.0.0.1:${wPort}",\n    public_url: "http://127.0.0.1:${hPort}",\n    tick_hz: 20,\n    static_dir: "${distDir}",\n)\n`);
            const log = fs.openSync(path.join(work, "server.log"), "w");
            server = spawn(serverBin, [path.join(work, "server.ron")], { stdio: ["ignore", log, log] });
            stat = await staticServer(distDir, sPort);
            for (let i = 0; i < 100; i += 1) { try { const r = await fetch(`http://127.0.0.1:${hPort}/advertisement.json`); if (r.ok) break; } catch {} await new Promise((r) => setTimeout(r, 100)); }
            url = `http://127.0.0.1:${sPort}/?server=${encodeURIComponent(`http://127.0.0.1:${hPort}`)}&ws=${encodeURIComponent(`ws://127.0.0.1:${wPort}`)}`;
        } else {
            stat = await staticServer(pageDir, sPort);
            const writes = v.mode === "work" ? (measuredWrites || []).join(",") : "";
            out.writes = writes;
            url = `http://127.0.0.1:${sPort}/probe.html?w=${v.w}&h=${v.h}&mode=${v.mode}&writes=${writes}`;
        }
        const args = JSON.parse(process.env.MC_CHROME_ARGS_JSON).filter((a) => !(v.dropArgs || []).includes(a));
        out.chrome_args = args;
        browser = await chromium.launch({ executablePath: "/usr/bin/google-chrome", headless: false, args,
            ignoreDefaultArgs: JSON.parse(process.env.MC_CHROME_IGNORE_DEFAULT_ARGS_JSON || '["--enable-unsafe-swiftshader"]') });
        const pages = [];
        for (let n = 0; n < v.pages; n += 1) {
            const ctx = await browser.newContext({ viewport: { width: v.w, height: v.h } });
            if (v.kind === "client") await ctx.addInitScript(clientInstrument);
            const page = await ctx.newPage();
            await page.goto(url, { waitUntil: "domcontentloaded" });
            await page.bringToFront();
            if (v.kind === "client") {
                await page.waitForFunction(() => { try { const d = JSON.parse(window.turts_debug()); return String(d.phase).startsWith("Joined") && Boolean(d.last_frame); } catch { return false; } }, null, { timeout: 40000, polling: 100 });
            } else {
                await page.waitForFunction(() => window.__probe && (window.__probe.failed || window.__probe.frames > 30), null, { timeout: 30000, polling: 100 });
                const failed = await page.evaluate(() => window.__probe.failed);
                if (failed) throw new Error("probe page failed: " + failed);
            }
            pages.push(page);
        }
        await pages[0].waitForTimeout(2000);
        const t0 = await Promise.all(pages.map((p) => p.evaluate(() => performance.now())));
        const f0 = await Promise.all(pages.map((p) => p.evaluate(() => window.__probe.frames)));
        const c0 = cpuSample(); const w0 = Date.now();
        const k0 = v.kind === "client" ? await Promise.all(pages.map((p) => p.evaluate(() => ({ ...window.__probe.calls })))) : null;
        await pages[0].waitForTimeout(MEASURE_S * 1000);
        const c1 = cpuSample(); const secs = (Date.now() - w0) / 1000;
        const f1 = await Promise.all(pages.map((p) => p.evaluate(() => window.__probe.frames)));
        const t1 = await Promise.all(pages.map((p) => p.evaluate(() => performance.now())));
        out.gpu_util = gpuUtil();
        const times = await Promise.all(pages.map((p) => p.evaluate(() => window.__probe.times.slice())));
        out.pages = pages.map((_, i) => ({ ...frameStats(times[i], t0[i], t1[i]), presented: f1[i] - f0[i] }));
        const threads = cpuDelta(c0, c1, secs);
        const gpuThreads = threads.filter((t) => t.type === "gpu-process");
        const main = gpuThreads.find((t) => t.pid === t.tid) || null;
        const top = gpuThreads[0] || null;
        const totalFrames = out.pages.reduce((s, p) => s + p.presented, 0);
        out.window_s = Number(secs.toFixed(2));
        out.total_presented = totalFrames;
        out.gpu_main = main && { tid: main.tid, comm: main.comm, pct: main.pct };
        out.gpu_top_thread = top && { tid: top.tid, comm: top.comm, pct: top.pct };
        out.gpu_main_ms_per_frame = main && totalFrames ? Number(((main.pct / 100) * secs * 1000 / totalFrames).toFixed(2)) : null;
        out.gpu_threads = gpuThreads.filter((t) => t.pct >= 3).map((t) => `${t.tid}:${t.comm}=${t.pct}`);
        out.other_threads = threads.filter((t) => t.type !== "gpu-process" && t.pct >= 10).map((t) => `${t.type}:${t.tid}:${t.comm}=${t.pct}`);
        if (v.kind === "client") {
            const k1 = await Promise.all(pages.map((p) => p.evaluate(() => ({ ...window.__probe.calls }))));
            out.calls_per_frame = k1.map((k, i) => Object.fromEntries(Object.keys(k).map((key) => [key, Number(((k[key] - k0[i][key]) / Math.max(1, out.pages[i].presented)).toFixed(2))])));
            const per = await pages[0].evaluate(() => window.__probe.perFrame.slice(-200));
            per.sort((x, y) => x.writes.length - y.writes.length);
            const med = per[Math.floor(per.length / 2)];
            out.median_frame = med && { writes: med.writes, write_bytes: med.writes.reduce((s, b) => s + b, 0), draws: med.draws, passes: med.passes, submits: med.submits };
            if (name === "e" && med) measuredWrites = med.writes;
        }
        if (main) {
            const pf0 = await Promise.all(pages.map((p) => p.evaluate(() => window.__probe.frames)));
            out.perf = perfRun(main.tid, path.join(work, "perf.data"));
            const pf1 = await Promise.all(pages.map((p) => p.evaluate(() => window.__probe.frames)));
            out.perf_window_presented = pf1.reduce((s, x, i) => s + x - pf0[i], 0);
        }
        if (v.kind === "probe") out.page_errors = await Promise.all(pages.map((p) => p.evaluate(() => window.__probe.errors)));
        await pages[0].screenshot({ path: path.join(work, "page1.png") }).catch(() => {});
    } catch (e) {
        out.error = String(e && e.stack || e);
    } finally {
        if (browser) await browser.close().catch(() => {});
        if (stat) stat.close();
        if (server) server.kill("SIGTERM");
    }
    for (const f of ["perf.data", "perf.data.old"]) { try { fs.unlinkSync(path.join(work, f)); } catch {} }
    fs.appendFileSync(path.join(outDir, "probe-a.jsonl"), JSON.stringify(out) + "\n");
    console.log(JSON.stringify(out));
}

(async () => { for (const p of phases) await runPhase(p); })().catch((e) => { console.error(e); process.exit(1); });
