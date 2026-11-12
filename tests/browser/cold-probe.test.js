// turts-test-role: unit
"use strict";

// Q-COLD pass 2: the scripted probes of the cold-play tool (no agent) and the UI-record checks.
//
// The probes run against small fixture pages in a real headless Chrome (cold-fixture.js). A page is a
// "game" with a start overlay, three own units, one enemy, an optional guide layer, optional alert
// lines, and the five bridges that the tool reads. The tests show, for each check, a page where the
// fact is true (red) and a page where it is false (green). The plants of docs/research/mp1/plants/q-cold.json
// edit the probe and run this file. Skipped when there is no Chrome binary.
const assert = require("node:assert/strict");
const { spawn, spawnSync } = require("node:child_process");
const fs = require("node:fs");
const http = require("node:http");
const net = require("node:net");
const os = require("node:os");
const path = require("node:path");
const { deflateSync } = require("node:zlib");
const test = require("node:test");
const { changeStats, nonflatShare } = require("./cold-pixels.js");
const { decodePng } = require("./harness/png.js");
const { ColdSession } = require("./cold-session.js");
const { SETTLE_MS, runProbe, wordCount, ownUnitsOnScreen, visibleButtons, normalisedUi } = require("./cold-probe.js");
const { fixtureGame } = require("./cold-fixture.js");
const { silentEnv, SILENT_ARGS } = require("./harness/silent-chrome.js");

const REPO = path.resolve(__dirname, "../..");
const CHECKER = path.join(REPO, "scripts/qa/cold_play_check.py");
const TEMP = [];
function tempDir(label) {
    const dir = fs.mkdtempSync(path.join(os.tmpdir(), `cold-probe-${label}-`));
    TEMP.push(dir);
    return dir;
}
test.after(() => { for (const dir of TEMP) fs.rmSync(dir, { recursive: true, force: true }); });

// ---- Pixels ------------------------------------------------------------------------------------

function crc32(bytes) {
    let value = 0xffffffff;
    for (const byte of bytes) {
        value ^= byte;
        for (let bit = 0; bit < 8; bit += 1) value = value & 1 ? (value >>> 1) ^ 0xedb88320 : value >>> 1;
    }
    return (value ^ 0xffffffff) >>> 0;
}
function chunk(type, data) {
    const name = Buffer.from(type);
    const length = Buffer.alloc(4);
    length.writeUInt32BE(data.length);
    const crc = Buffer.alloc(4);
    crc.writeUInt32BE(crc32(Buffer.concat([name, data])));
    return Buffer.concat([length, name, data, crc]);
}
// An RGB PNG whose pixel (x, y) is `color(x, y)`.
function png(width, height, color) {
    const raw = Buffer.alloc(height * (width * 3 + 1));
    for (let y = 0; y < height; y += 1) {
        for (let x = 0; x < width; x += 1) {
            const [r, g, b] = color(x, y);
            raw.set([r, g, b], y * (width * 3 + 1) + 1 + x * 3);
        }
    }
    const header = Buffer.alloc(13);
    header.writeUInt32BE(width, 0);
    header.writeUInt32BE(height, 4);
    header.set([8, 2, 0, 0, 0], 8);
    return Buffer.concat([Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]), chunk("IHDR", header), chunk("IDAT", deflateSync(raw)), chunk("IEND", Buffer.alloc(0))]);
}
const image = (width, height, color) => decodePng(png(width, height, color));

test("changeStats counts the changed pixels in a box and in the page, and ignores a change below the threshold", () => {
    const a = image(100, 100, () => [10, 10, 10]);
    const b = image(100, 100, (x, y) => (x >= 40 && x < 60 && y >= 40 && y < 60 ? [200, 10, 10] : [10, 10, 10]));
    const stats = changeStats(a, b, [30, 30, 40, 40]);
    assert.deepEqual(stats, { page_changed: 400, page_share: 0.04, box_changed: 400, box_px: 1600 });
    assert.equal(changeStats(a, b, [0, 0, 20, 20]).box_changed, 0, "a box away from the change");
    const faint = image(100, 100, () => [12, 12, 12]);
    assert.equal(changeStats(a, faint, [0, 0, 100, 100]).page_changed, 0, "a change of 6 in the sum is noise");
    assert.equal(changeStats(a, b, [90, 90, 50, 50]).box_px, 100, "a box is clamped to the page");
});

test("nonflatShare: an opaque layer with text is flat, a see-through layer is not", () => {
    const text = (x, y) => y % 20 < 6 && x % 8 < 5 && x > 20 && x < 180;
    const opaque = image(200, 200, (x, y) => (text(x, y) ? [255, 255, 255] : [20, 30, 40]));
    const seeThrough = image(200, 200, (x, y) => (text(x, y) ? [255, 255, 255] : x > 100 && y > 100 ? [60, 90, 60] : [20, 30, 40]));
    assert.ok(nonflatShare(opaque, [0, 0, 200, 200]) < 0.01, `opaque ${nonflatShare(opaque, [0, 0, 200, 200])}`);
    assert.ok(nonflatShare(seeThrough, [0, 0, 200, 200]) > 0.15, `see-through ${nonflatShare(seeThrough, [0, 0, 200, 200])}`);
    assert.equal(nonflatShare(opaque, [500, 500, 10, 10]), 0, "a rect outside the page");
});

test("the probes wait 8 s for a person's first look by default", () => {
    assert.equal(SETTLE_MS, 8000);
});

test("wordCount, visibleButtons, ownUnitsOnScreen and normalisedUi", () => {
    assert.equal(wordCount("  one two\nthree\t four "), 4);
    assert.equal(wordCount(null), 0);
    assert.equal(wordCount(""), 0);
    const ui = [
        { id: "b2", kind: "button", visible: true, enabled: true, rect: [100, 50, 40, 20], text: "B" },
        { id: "b1", kind: "button", visible: true, enabled: true, rect: [10, 10, 40, 20], text: "A" },
        { id: "hid", kind: "button", visible: false, enabled: true, rect: [10, 90, 40, 20], text: "H" },
        { id: "off", kind: "button", visible: true, enabled: true, rect: [0, 900, 40, 20], text: "O" },
        { id: "dis", kind: "button", visible: true, enabled: false, rect: [10, 120, 40, 20], text: "D" },
        { id: "w", kind: "widget", visible: true, enabled: true, rect: [10, 150, 40, 20], text: "W" },
        { id: "focus_overlay", kind: "focus_overlay", visible: true, enabled: true, rect: [0, 0, 1280, 720] },
    ];
    assert.deepEqual(visibleButtons(ui, { width: 1280, height: 720 }).map((button) => button.id), ["b1", "b2"], "visible, enabled, on the page, top to bottom then left to right");
    const world = { slot: 0, units: [
        { unit_id: 3, owner_slot: 0, content_id: "a", center: [100, 100] }, { unit_id: 1, owner_slot: 0, content_id: "a", center: [200, 100] },
        { unit_id: 2, owner_slot: 1, content_id: "b", center: [300, 100] }, { unit_id: 4, owner_slot: 0, content_id: "c", center: null },
        { unit_id: 5, owner_slot: 0, content_id: "c", center: [5000, 100] }, { unit_id: 6, owner_slot: 0, content_id: "c", center: [400, 100] }] };
    const own = ownUnitsOnScreen(world, { width: 1280, height: 720 }, 60);
    assert.deepEqual(own.map((unit) => unit.unit_id), [1, 6, 3], "own, on screen; one of each content id first, then the rest, by id");
    assert.equal(ownUnitsOnScreen(world, { width: 1280, height: 720 }, 2).length, 2, "a cap");
    assert.equal(ownUnitsOnScreen({ slot: 0, units: [] }, { width: 1280, height: 720 }, 60).length, 0);
    const before = normalisedUi([{ id: "x", visible: true, text: "a", rect: [1.2, 2, 3, 4] }, { id: "focus_overlay", kind: "focus_overlay", visible: true, rect: [0, 0, 9, 9] }]);
    assert.deepEqual(Object.keys(before), ["x"], "the start overlay is not part of the record that a click is judged by");
    assert.deepEqual(Object.keys(normalisedUi([{ id: "b", visible: true, rect: [0, 0, 1, 1] }, { id: "b::tooltip", visible: true, text: "tip", rect: [0, 0, 1, 1] }])), ["b"], "a tooltip is hover feedback, not a click effect");
});

// ---- A real Chrome and the fixture pages -------------------------------------------------------------

const CHROME = process.env.CHROME_BIN || ["google-chrome", "chromium"].map((name) => spawnSync("which", [name], { encoding: "utf8" }).stdout.trim()).find(Boolean);
const skip = !CHROME && "no Chrome binary";

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

let shared = null;
async function chrome() {
    if (shared) return shared;
    const { chromium } = require("playwright-core");
    const server = http.createServer((request, response) => {
        const options = JSON.parse(decodeURIComponent(new URL(request.url, "http://x").search.slice(1) || "%7B%7D"));
        response.setHeader("content-type", "text/html");
        response.end(fixtureGame(options));
    });
    await new Promise((resolve) => server.listen(0, "127.0.0.1", resolve));
    const port = await freePort();
    const profile = tempDir("chrome");
    const child = spawn(CHROME, [`--user-data-dir=${profile}`, `--remote-debugging-port=${port}`, "--no-first-run", "--no-default-browser-check", "--headless=new", "--disable-gpu", ...SILENT_ARGS, "about:blank"], { stdio: "ignore", env: silentEnv(tempDir("quiet")) });
    await until(async () => (await fetch(`http://127.0.0.1:${port}/json/version`)).json(), "the debugging port");
    const browser = await chromium.connectOverCDP(`http://127.0.0.1:${port}`);
    shared = { browser, server, child, base: `http://127.0.0.1:${server.address().port}/` };
    return shared;
}
test.after(async () => {
    if (!shared) return;
    try { await shared.browser.close(); } catch { /* gone */ }
    shared.child.kill();
    await new Promise((resolve) => { if (shared.child.exitCode !== null) resolve(); else shared.child.once("exit", resolve); });
    shared.server.close();
});

// Run one probe on a fixture page; returns { dir, record }.
async function probe(kind, options, probeOptions = {}) {
    const { browser, base } = await chrome();
    const page = await browser.contexts()[0].newPage();
    try {
        await page.goto(`${base}?${encodeURIComponent(JSON.stringify(options))}`);
        await page.setViewportSize({ width: 1280, height: 720 });
        const dir = path.join(tempDir(`run-${kind}`), "evidence");
        const session = new ColdSession({ page, dir, viewport: { width: 1280, height: 720 }, limitMs: 120000, mode: `probe-${kind}`, control: probeOptions.control ?? null,
            meta: { build: "fixture" } });
        await session.start();
        // The sampler of the sound records runs on a timer in a real session (cold-session.js main).
        const timer = setInterval(() => { session.sample(); }, 500);
        let record;
        try { record = await runProbe(session, kind, { settleMs: 0, ...probeOptions }); } finally { clearInterval(timer); }
        await session.finish("probe-done");
        return { dir, record, session };
    } finally { await page.close(); }
}
const read = (dir, name) => JSON.parse(fs.readFileSync(path.join(dir, name), "utf8"));
function check(dirs, extra = []) {
    const [first, ...rest] = dirs;
    const args = [first, ...rest.flatMap((dir) => ["--also", dir]), "--no-lock-check", ...extra];
    const run = spawnSync("python3", ["-I", "-B", CHECKER, ...args], { encoding: "utf8" });
    return { status: run.status, out: run.stdout, err: run.stderr };
}

const GREEN = { startOverlay: true, guide: null, alerts: 0 };
const GUIDE = { startOverlay: true, guide: "seethrough", guideWords: 80, alerts: 2 };

test("first-click probe, green page: every own unit click selects it and changes the picture", { skip, timeout: 120000 }, async () => {
    const { dir, record } = await probe("first-click", GREEN);
    assert.equal(record.summary.verdict, "PASS", JSON.stringify(record.summary));
    assert.equal(record.clicks.length, 3, "the three own units, not the enemy");
    for (const click of record.clicks) {
        assert.ok(click.selection.includes(click.unit_id), JSON.stringify(click));
        assert.ok(click.visible_change && click.ok, JSON.stringify(click));
        assert.ok(click.box_changed > 40, `box ${click.box_changed}`);
    }
    assert.deepEqual(record.start, { found: "focus_overlay", point: [640, 360] }, "the start click goes to the start overlay");
    const inputs = fs.readFileSync(path.join(dir, "inputs.jsonl"), "utf8").trim().split("\n").map((line) => JSON.parse(line));
    assert.deepEqual(inputs.map((input) => input.cmd), ["click", "click", "click", "click"], "the start click and one click for each unit: no key, no other input");
    assert.ok(inputs.every((input) => input.args.button === "left"));
    assert.deepEqual(read(dir, "probe-first-click.json").summary, record.summary, "the record is on the disk");
});

test("first-click probe, page with a see-through full-size guide: it FAILS at the first unit (the layer takes the click)", { skip, timeout: 120000 }, async () => {
    const { record } = await probe("first-click", GUIDE);
    assert.equal(record.summary.verdict, "FAIL");
    assert.equal(record.summary.failed, 3, "every click fails");
    assert.equal(record.summary.first_failure.unit_id, record.clicks[0].unit_id);
    for (const click of record.clicks) {
        assert.deepEqual(click.selection, [], "no selection");
        assert.equal(click.visible_change, false);
        assert.equal(click.selection_ok, false);
    }
    assert.match(record.summary.reason, /first click on unit \d+ \(fixture:own\d\): selection \[\], no visible change/);
});

test("first-click probe, opaque guide: still FAILS (the click is taken), but it is not a see-through layer", { skip, timeout: 120000 }, async () => {
    const { record } = await probe("first-click", { startOverlay: true, guide: "opaque", guideWords: 12, alerts: 0 });
    assert.equal(record.summary.verdict, "FAIL");
});

test("first-click probe, positive control: after the tool closes the guide the probe passes", { skip, timeout: 120000 }, async () => {
    const { dir, record } = await probe("first-click", GUIDE, { control: "close-layer" });
    assert.equal(record.summary.verdict, "PASS", JSON.stringify(record.summary));
    assert.equal(record.layer_close.closed, true);
    assert.ok(record.layer_close.clicks >= 1 && record.layer_close.clicks <= 3);
    assert.equal(read(dir, "session.json").control, "close-layer");
});

test("first-click probe: a ring around the unit alone (no panel) is a visible change, by the box rule", { skip, timeout: 120000 }, async () => {
    const { record } = await probe("first-click", { ...GREEN, noPanel: true });
    assert.equal(record.summary.verdict, "PASS", JSON.stringify(record.clicks[0]));
    assert.ok(record.clicks[0].page_changed < 2000, `the first ring alone is under the page rule: ${record.clicks[0].page_changed}`);
    for (const click of record.clicks) assert.ok(click.box_changed >= 40, `box ${click.box_changed}`);
});

test("first-click probe on a page with no own unit on screen is a FAIL, not a pass", { skip, timeout: 120000 }, async () => {
    const { record } = await probe("first-click", { ...GREEN, units: 0 });
    assert.equal(record.summary.verdict, "FAIL");
    assert.match(record.summary.reason, /no own unit on screen/);
});

test("first-click probe: a click that selects but shows nothing is a FAIL; a visible change with no selection is a FAIL", { skip, timeout: 120000 }, async () => {
    const quiet = (await probe("first-click", { ...GREEN, selectShows: false })).record;
    assert.equal(quiet.summary.verdict, "FAIL");
    assert.ok(quiet.clicks.every((click) => click.selection_ok && !click.visible_change));
    const wrong = (await probe("first-click", { ...GREEN, selectsNothing: true })).record;
    assert.equal(wrong.summary.verdict, "FAIL");
    assert.ok(wrong.clicks.every((click) => click.visible_change && !click.selection_ok), JSON.stringify(wrong.clicks[0]));
});

// ---- UI-record checks (F9, F3, F6) and the buttons probe (F8) -----------------------------------------------

test("the UI records are taken before the first input and after the first click, and are not on the disk before the release", { skip, timeout: 120000 }, async () => {
    const { browser, base } = await chrome();
    const page = await browser.contexts()[0].newPage();
    try {
        await page.goto(`${base}?${encodeURIComponent(JSON.stringify(GUIDE))}`);
        await page.setViewportSize({ width: 1280, height: 720 });
        const dir = path.join(tempDir("ui"), "evidence");
        const session = new ColdSession({ page, dir, viewport: { width: 1280, height: 720 }, limitMs: 120000 });
        await session.start();
        await session.handle({ cmd: "wait", args: ["0.5"] });
        assert.equal(fs.existsSync(path.join(dir, "ui-records.json")), false, "a wait is no input and writes no record");
        await session.handle({ cmd: "click", args: ["640", "360", "left"] });
        assert.equal(fs.existsSync(path.join(dir, "ui-records.json")), false, "the records are held back, as the sound records are");
        await session.finish("end");
        const records = read(dir, "ui-records.json").records;
        assert.deepEqual(records.map((record) => record.label), ["before_first_input", "after_first_click_p200", "after_first_click_p1000"]);
        const before = records[0];
        assert.equal(before.alert_lines, 2, "two alert lines before the first input");
        assert.ok(before.widgets.some((widget) => widget.id === "guide"));
        const after = records[1];
        assert.ok(after.widgets.some((widget) => widget.id === "guide_copy" && wordCount(widget.text) === 80));
        const big = after.big_widgets.find((widget) => widget.id === "guide");
        assert.ok(big.area_share > 0.9 && big.nonflat_share > 0.05 && big.capture_root, JSON.stringify(big));
    } finally { await page.close(); }
});

test("checker, UI records: a long text (F9), a see-through blocking layer (F3) and an alert line before input (F6) are FAILS; the green page is not", { skip, timeout: 120000 }, async () => {
    const green = await probe("first-click", GREEN);
    const buttons = await probe("buttons", { ...GREEN, startOverlay: false, helpButton: true });
    const ok = check([green.dir, buttons.dir], ["--partial"]);
    assert.doesNotMatch(ok.out, /FAIL (F9|F3|F6)/, ok.out);
    assert.match(ok.out, /PASS F9 /);
    assert.match(ok.out, /PASS F3 /);
    assert.match(ok.out, /PASS F6 /);
    const guide = await probe("first-click", GUIDE);
    const bad = check([guide.dir], ["--partial"]);
    assert.equal(bad.status, 1, bad.out);
    assert.match(bad.out, /FAIL F9 .*guide_copy: 80 words/);
    assert.match(bad.out, /FAIL F3 .*guide/);
    assert.match(bad.out, /FAIL F6 .*2 alert lines/);
    assert.match(bad.out, /FAIL P1 .*first click on unit/);
    assert.match(bad.out, /NOT-READY/);
    // Each fact alone: a long text on an opaque layer; a see-through layer with little text; alerts only.
    const longText = check([(await probe("first-click", { ...GREEN, guide: "opaque", guideWords: 90, alerts: 0 })).dir], ["--partial"]);
    assert.match(longText.out, /FAIL F9 /);
    assert.match(longText.out, /PASS F3 /, "an opaque layer is not see-through");
    const seeThrough = check([(await probe("first-click", { ...GREEN, guide: "seethrough", guideWords: 10, alerts: 0 })).dir], ["--partial"]);
    assert.match(seeThrough.out, /PASS F9 /);
    assert.match(seeThrough.out, /FAIL F3 /);
    const alerts = check([(await probe("first-click", { ...GREEN, alerts: 1 })).dir], ["--partial"]);
    assert.match(alerts.out, /FAIL F6 .*1 alert line/);
    assert.match(alerts.out, /PASS F3 /);
});

test("buttons probe: a button that the start overlay covers loses its first click (F8); a button that acts passes", { skip, timeout: 120000 }, async () => {
    const lost = await probe("buttons", { ...GREEN, helpButton: true });
    assert.equal(lost.record.summary.verdict, "FAIL");
    assert.equal(lost.record.buttons.length, 1);
    assert.equal(lost.record.buttons[0].acted, false);
    assert.equal(lost.record.buttons[0].id, "help");
    const inputs = fs.readFileSync(path.join(lost.dir, "inputs.jsonl"), "utf8").trim().split("\n").map((line) => JSON.parse(line));
    assert.deepEqual(inputs.map((input) => input.cmd), ["move", "click"], "the pointer rests on the button, then one click; no start click first");
    const acts = await probe("buttons", { ...GREEN, startOverlay: false, helpButton: true });
    assert.equal(acts.record.summary.verdict, "PASS", JSON.stringify(acts.record));
    assert.equal(acts.record.buttons[0].acted, true);
    assert.ok(acts.record.buttons[0].changed_ids.includes("help_panel"));
    const failed = check([lost.dir, ], ["--partial"]);
    assert.match(failed.out, /FAIL F8 .*help/);
    assert.match(failed.out, /NOT-READY/);
    const passed = check([acts.dir], ["--partial"]);
    assert.match(passed.out, /PASS F8 /);
});

test("buttons probe: a start overlay that vanishes on the first click does not count as the button acting", { skip, timeout: 120000 }, async () => {
    const { record } = await probe("buttons", { ...GREEN, helpButton: true, helpDead: true });
    assert.equal(record.buttons[0].acted, false, "the overlay left, the button did nothing");
    assert.deepEqual(record.buttons[0].changed_ids, []);
});

test("buttons probe: what the pointer resting on a button shows (a tooltip, a glow) is not the click acting", { skip, timeout: 120000 }, async () => {
    const { record } = await probe("buttons", { ...GREEN, startOverlay: false, helpButton: true, helpDead: true, hover: true });
    assert.equal(record.buttons[0].acted, false, JSON.stringify(record.buttons[0]));
    assert.deepEqual(record.buttons[0].changed_ids, []);
    const live = await probe("buttons", { ...GREEN, startOverlay: false, helpButton: true, hover: true });
    assert.equal(live.record.buttons[0].acted, true, "a button that acts is still seen to act");
    assert.deepEqual(live.record.buttons[0].changed_ids, ["help_panel"]);
});

test("the probes wait for a person's first look: an alert line that comes in that time is F6; with no wait it is missed", { skip, timeout: 120000 }, async () => {
    const options = { ...GREEN, alerts: 2, alertsLateMs: 1500 };
    const hasty = await probe("first-click", options, { settleMs: 0 });
    assert.equal(read(hasty.dir, "ui-records.json").records[0].alert_lines, 0, "the first input came before the alerts");
    const patient = await probe("first-click", options, { settleMs: 2500 });
    const before = read(patient.dir, "ui-records.json").records[0];
    assert.equal(before.label, "before_first_input");
    assert.equal(before.alert_lines, 2, "after the wait the two alert lines are there");
    assert.equal(patient.record.settle_ms, 2500);
    assert.match(check([patient.dir], ["--partial"]).out, /FAIL F6 .*2 alert lines/);
    const buttons = await probe("buttons", { ...GREEN, startOverlay: false, helpButton: true, alerts: 1, alertsLateMs: 1500 }, { settleMs: 2500 });
    assert.ok(buttons.record.buttons.some((button) => /alert_line/.test(button.id)), "the button list is read after the wait");
});

// ---- The verdict line ----------------------------------------------------------------------------------

test("checker verdict: NOT-READY when a probe fails or is missing; MACHINE-PASS (manual checks open) only with all of them", { skip, timeout: 120000 }, async () => {
    const green = await probe("first-click", GREEN);
    const buttons = await probe("buttons", { ...GREEN, startOverlay: false, helpButton: true });
    const both = check([green.dir, buttons.dir], ["--partial"]);
    assert.equal(both.status, 0, both.out);
    assert.match(both.out, /RESULT: PARTIAL-PASS/);
    const missing = check([green.dir]);
    assert.equal(missing.status, 1, missing.out);
    assert.match(missing.out, /NOT-READY \(missing: buttons probe, agent session\)/);
    assert.doesNotMatch(missing.out, /MACHINE-PASS/);
    const control = (await probe("first-click", GUIDE, { control: "close-layer" })).dir;
    const ctl = check([control]);
    assert.equal(ctl.status, 0, ctl.out);
    assert.match(ctl.out, /CONTROL-PASS/);
    assert.doesNotMatch(ctl.out, /FAIL F3|FAIL F9/, "a control run is judged on the probe only");
});
