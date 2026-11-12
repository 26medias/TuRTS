// turts-test-role: unit
"use strict";

// Plan task L0-DBG, step 2: the capture tool `mp1-capture.js` and the play script
// `scripts/mp1-play.sh`. The real-Chrome test starts a Chrome with a free remote debugging port and
// a new profile folder, opens a page with the three records, and reads it through the port. It is
// headless unless MP1_CAPTURE_HEADED=1 (the headed form is run under the GPU lock wrapper). It is
// skipped when no Chrome binary exists (CI has one).
const assert = require("node:assert/strict");
const { execFile, spawn, spawnSync } = require("node:child_process");
const fs = require("node:fs");
const http = require("node:http");
const net = require("node:net");
const os = require("node:os");
const path = require("node:path");
const test = require("node:test");
const { capture, parseArgs, parsePort } = require("./mp1-capture.js");
const { silentEnv, SILENT_ARGS } = require("./harness/silent-chrome.js");

const REPO = path.resolve(__dirname, "../..");
const CAPTURE = path.join(__dirname, "mp1-capture.js");
const PLAY = path.join(REPO, "scripts/mp1-play.sh");
const RECORD = { phase: "Joined { slot: 0 }", hints: { shown: ["F"] }, audio: { played: [{ cue: "alert" }] },
    effects: { recent: [{ kind: "hit", tick: 7 }] } };

// A fake `chromium.connectOverCDP`: the windows are `{ url, evaluate }` pages in one context.
function fakeChromium(pages) {
    const log = { endpoints: [], closed: 0, evaluated: [] };
    const chromium = { connectOverCDP: async (endpoint) => {
        log.endpoints.push(endpoint);
        return {
            contexts: () => [{ pages: () => pages.map((page) => ({ url: () => page.url,
                evaluate: async (fn) => { log.evaluated.push(page.url); return page.evaluate(fn); } })) }],
            close: async () => { log.closed += 1; },
        };
    } };
    return { chromium, log };
}

// Run the page function against a window, as the browser does.
function inWindow(window) {
    return (fn) => new Function("window", `return (${fn.toString()})()`)(window);
}

test("the capture returns hints, audio and effects.recent of each window, from a fresh record", async () => {
    const asked = [];
    const window = { turts_debug: () => "", turts_debug_request: (...args) => { asked.push(args); return Promise.resolve(JSON.stringify(RECORD)); } };
    const { chromium, log } = fakeChromium([{ url: "http://127.0.0.1:5000/?server=a", evaluate: inWindow(window) },
        { url: "chrome://newtab/", evaluate: () => assert.fail("a browser page is not read") }]);
    const result = await capture(chromium, 41234);
    assert.deepEqual(log.endpoints, ["http://127.0.0.1:41234"], "loopback only, the port that the caller gave");
    assert.deepEqual(asked, [[]], "one request with the default 4 Hz cap");
    assert.equal(result.windows.length, 1);
    assert.deepEqual(result.windows[0], { url: "http://127.0.0.1:5000/?server=a", available: true, hints: RECORD.hints,
        audio: RECORD.audio, effects_recent: RECORD.effects.recent, error: null });
    assert.equal(log.closed, 1, "the tool disconnects once");
});

test("a part that the build does not publish is null; a window with no bridge or a failing request is reported", async () => {
    const old = { turts_debug: () => JSON.stringify({ phase: "Joined", audio: { played: [] } }) };
    const noBridge = {};
    const rejecting = { turts_debug: () => "", turts_debug_request: () => Promise.reject(new Error("no frame served the request")) };
    const { chromium, log } = fakeChromium([{ url: "http://a/", evaluate: inWindow(old) },
        { url: "http://b/", evaluate: inWindow(noBridge) }, { url: "http://c/", evaluate: inWindow(rejecting) }]);
    const result = await capture(chromium, 1);
    assert.deepEqual(result.windows.map(({ available, hints, audio, effects_recent, error }) => ({ available, hints, audio, effects_recent, error })), [
        { available: true, hints: null, audio: { played: [] }, effects_recent: null, error: null },
        { available: false, hints: null, audio: null, effects_recent: null, error: null },
        { available: false, hints: null, audio: null, effects_recent: null, error: "no frame served the request" },
    ]);
    assert.equal(log.closed, 1);
});

test("the tool takes the port as an argument and has no default", () => {
    assert.deepEqual(parseArgs(["40123"]), { port: 40123, out: null });
    assert.deepEqual(parseArgs(["40123", "--out", "x.json"]), { port: 40123, out: "x.json" });
    for (const bad of [[], [""], ["abc"], ["0"], ["65536"], ["-1"], ["9222x"], ["1.5"]]) {
        assert.throws(() => parseArgs(bad), /usage: mp1-capture\.js <port/, JSON.stringify(bad));
    }
    assert.throws(() => parseArgs(["40123", "--bogus"]), /unknown argument/);
    assert.equal(parsePort("65535"), 65535);
    const run = spawnSync(process.execPath, [CAPTURE], { encoding: "utf8" });
    assert.equal(run.status, 2, "no port: exit 2");
    assert.match(run.stderr, /usage/);
});

// ---- The play script and the sources: what they must never contain ---------------------------

const SOURCES = { "scripts/mp1-play.sh": PLAY, "tests/browser/mp1-capture.js": CAPTURE };
const text = (file) => fs.readFileSync(file, "utf8");
// The text with the comment lines removed: a comment may name what the code never does.
const code = (file) => text(file).split("\n").filter((line) => !/^\s*(?:#|\/\/)/.test(line)).join("\n");

test("neither file fixes the debugging port, mutes Chrome, or touches the system sound", () => {
    for (const [name, file] of Object.entries(SOURCES)) {
        assert.doesNotMatch(text(file), /9222/, `${name} names a fixed port`);
        assert.doesNotMatch(code(file), /mute-audio|pactl|pacmd|amixer|wpctl|pw-cli|alsamixer|--audio-output|set-sink-volume/, `${name} touches the sound`);
    }
});

test("the capture tool opens no page and no address other than 127.0.0.1", () => {
    const source = code(CAPTURE);
    assert.doesNotMatch(source, /\.(?:newPage|newContext|goto|launch|launchPersistentContext)\(/);
    assert.deepEqual([...source.matchAll(/https?:\/\/[^\s`"']*/g)].map((match) => match[0]).filter((url) => !url.startsWith("http://127.0.0.1:")), []);
});

test("the play script prints a free port, a non-default profile and the capture command", () => {
    assert.equal(spawnSync("bash", ["-n", PLAY]).status, 0, "bash syntax");
    const ports = new Set();
    for (let i = 0; i < 3; i += 1) {
        const run = spawnSync("bash", [PLAY, "--print-plan"], { encoding: "utf8", env: { ...process.env, CHROME_BIN: "" } });
        assert.equal(run.status, 0, run.stderr);
        const lines = Object.fromEntries(run.stdout.split("\n").filter(Boolean).map((line) => [line.slice(0, line.indexOf(":")), line.slice(line.indexOf(":") + 2)]));
        const port = Number(lines.DEBUG_PORT);
        assert.ok(Number.isInteger(port) && port > 1023 && port !== 9222, `port ${lines.DEBUG_PORT}`);
        assert.equal(lines.CAPTURE, `node tests/browser/mp1-capture.js ${port}`, "the capture line gives the printed port");
        assert.match(lines.CHROME, new RegExp(`--remote-debugging-port=${port}(?: |$)`));
        const profile = /--user-data-dir=(\S+)/.exec(lines.CHROME)?.[1];
        assert.ok(profile && profile === lines.PROFILE, "a non-default profile folder is named");
        assert.ok(!/\.config|google-chrome|chromium/.test(profile), `the profile is not a default Chrome profile: ${profile}`);
        assert.doesNotMatch(lines.CHROME, /mute-audio/);
        ports.add(port);
    }
    assert.ok(ports.size >= 2, `the port is picked at each start: ${[...ports]}`);
});

test("a free port that the play script prints can be bound", async () => {
    const run = spawnSync("bash", [PLAY, "--print-plan"], { encoding: "utf8" });
    const port = Number(/DEBUG_PORT: (\d+)/.exec(run.stdout)[1]);
    await new Promise((resolve, reject) => {
        const server = net.createServer().once("error", reject).listen(port, "127.0.0.1", () => server.close(resolve));
    });
});

test("the play script with a stale prepared folder starts no Chrome and exits with a code", () => {
    const folder = fs.mkdtempSync(path.join(os.tmpdir(), "mp1-play-stale-"));
    try {
        const marker = path.join(folder, "chrome-was-started");
        const fake = path.join(folder, "chrome");
        fs.writeFileSync(fake, `#!/bin/sh\ntouch '${marker}'\n`, { mode: 0o755 });
        const run = spawnSync("bash", [PLAY, "--prepared", path.join(folder, "none")], { encoding: "utf8", env: { ...process.env, CHROME_BIN: fake } });
        assert.notEqual(run.status, 0);
        assert.match(run.stderr, /the server did not start/);
        assert.equal(fs.existsSync(marker), false, "no Chrome is started when the server did not start");
    } finally { fs.rmSync(folder, { recursive: true, force: true }); }
});

// ---- A real Chrome ---------------------------------------------------------------------------

const CHROME = process.env.CHROME_BIN || ["google-chrome", "chromium"].map((name) => spawnSync("which", [name], { encoding: "utf8" }).stdout.trim()).find(Boolean);

async function freePort() {
    return new Promise((resolve, reject) => {
        const server = net.createServer().once("error", reject).listen(0, "127.0.0.1", () => {
            const { port } = server.address();
            server.close(() => resolve(port));
        });
    });
}

async function until(check, what, ms = 15000) {
    const end = Date.now() + ms;
    for (;;) {
        try { const value = await check(); if (value) return value; } catch { /* not ready */ }
        if (Date.now() > end) throw new Error(`timeout: ${what}`);
        await new Promise((resolve) => setTimeout(resolve, 100));
    }
}

test("a capture through the port of a real Chrome window returns the three records and leaves Chrome open", { skip: !CHROME && "no Chrome binary" }, async () => {
    const page = http.createServer((_request, response) => {
        response.setHeader("content-type", "text/html");
        response.end(`<!doctype html><title>fake game</title><script>
            window.turts_debug = () => "";
            window.turts_debug_request = () => Promise.resolve(${JSON.stringify(JSON.stringify(RECORD))});
        </script>`);
    });
    await new Promise((resolve) => page.listen(0, "127.0.0.1", resolve));
    const url = `http://127.0.0.1:${page.address().port}/`;
    const port = await freePort();
    const profile = fs.mkdtempSync(path.join(os.tmpdir(), "mp1-capture-profile-"));
    const headed = process.env.MP1_CAPTURE_HEADED === "1";
    const chrome = spawn(CHROME, [`--user-data-dir=${profile}`, `--remote-debugging-port=${port}`, "--no-first-run",
        "--no-default-browser-check", ...(headed ? [] : ["--headless=new", "--disable-gpu"]), ...SILENT_ARGS, url], { stdio: "ignore", env: silentEnv(profile) });
    try {
        const version = await until(async () => (await fetch(`http://127.0.0.1:${port}/json/version`)).json(), "the debugging port");
        assert.match(version.Browser, /Chrome/);
        await until(async () => (await (await fetch(`http://127.0.0.1:${port}/json/list`)).json()).some((item) => item.url === url), "the page");
        const out = path.join(profile, "capture.json");
        const result = await new Promise((resolve) => execFile(process.execPath, [CAPTURE, String(port), "--out", out],
            { encoding: "utf8", timeout: 40000 }, (error, stdout, stderr) => resolve({ status: error ? (error.code ?? 1) : 0, stdout, stderr })));
        assert.equal(result.status, 0, result.stderr);
        const captured = JSON.parse(result.stdout);
        assert.deepEqual(JSON.parse(fs.readFileSync(out, "utf8")), captured, "--out holds the same record");
        assert.equal(captured.port, port);
        assert.deepEqual(captured.windows.map(({ url: windowUrl, available, hints, audio, effects_recent, error }) => ({ windowUrl, available, hints, audio, effects_recent, error })),
            [{ windowUrl: url, available: true, hints: RECORD.hints, audio: RECORD.audio, effects_recent: RECORD.effects.recent, error: null }]);
        // The tool disconnected: Chrome and the window are still there.
        const after = await (await fetch(`http://127.0.0.1:${port}/json/list`)).json();
        assert.ok(after.some((item) => item.url === url), "the window is still open after the capture");
        assert.equal(chrome.exitCode, null, "Chrome is still running after the capture");
    } finally {
        chrome.kill();
        await new Promise((resolve) => { if (chrome.exitCode !== null) resolve(); else chrome.once("exit", resolve); });
        page.close();
        fs.rmSync(profile, { recursive: true, force: true });
    }
});
