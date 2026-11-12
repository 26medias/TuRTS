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
const { SETTLE_MS, BUTTONS_SETTLE_MS, runProbe, wordCount, ownUnitsOnScreen, visibleButtons, buttonsOutsidePage, orderButtons, normalisedUi, medianShift, boxHalf, openGround } = require("./cold-probe.js");
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

test("the probes wait 8 s (first click) and watch 20 s (buttons, F6) by default", () => {
    assert.equal(SETTLE_MS, 8000);
    assert.equal(BUTTONS_SETTLE_MS, 20000);
});

test("orderButtons: visible enabled order buttons, the destructive ones last; boxHalf scales with the unit; medianShift sees a moved camera, not one walker", () => {
    const view = { width: 1280, height: 720 };
    const button = (id, text, y, kind = "order", extra = {}) => ({ id, kind: "button", visible: true, enabled: true, rect: [1000, y, 200, 28], text, action: { kind, id: `p:${id}` }, ...extra });
    const ui = [button("remove", "Remove this machine [X]", 300), button("disable", "Disable [D]", 255), button("move", "Camera", 100, "center_camera"), button("off", "Off", 400, "order", { enabled: false }),
        button("hold", "Hold [H]", 330), button("out", "Out", 900)];
    assert.deepEqual(orderButtons(ui, view).map((entry) => entry.id), ["disable", "hold", "remove"], "order kind only, on the page, enabled; Remove last");
    assert.deepEqual(buttonsOutsidePage(ui, view).map((entry) => entry.id), ["out"]);
    assert.equal(boxHalf(0), 48);
    assert.equal(boxHalf(30), 48, "1.5 x 30 = 45: the minimum holds");
    assert.equal(boxHalf(100), 150, "a large object gets a box 1.5 times its size");
    const a = { units: [1, 2, 3, 4].map((id) => ({ unit_id: id, center: [id * 100, 100] })) };
    const camera = { units: a.units.map((unit) => ({ unit_id: unit.unit_id, center: [unit.center[0] + 40, unit.center[1]] })) };
    const walker = { units: a.units.map((unit) => ({ unit_id: unit.unit_id, center: unit.unit_id === 1 ? [unit.center[0] + 90, 100] : unit.center })) };
    assert.equal(medianShift(a, camera), 40);
    assert.equal(medianShift(a, walker), 0);
    assert.equal(medianShift(a, { units: [] }), 0);
});

test("openGround: a point near the own units, away from every unit and node and under no panel; none when there is no room", () => {
    const view = { width: 1280, height: 720 };
    const world = { slot: 0, units: [{ unit_id: 1, owner_slot: 0, center: [600, 300], pick_radius: 30 }, { unit_id: 2, owner_slot: 1, center: [640, 340], pick_radius: 30 }],
        nodes: [{ node_id: 5, center: [560, 300], pick_radius: 30 }], ui: [{ id: "panel", visible: true, rect: [0, 0, 300, 700], kind: "widget" }, { id: "big", visible: true, rect: [0, 0, 1280, 720], kind: "widget" }] };
    const point = openGround(world, view);
    assert.ok(point, "a point is found");
    for (const thing of [...world.units, ...world.nodes]) assert.ok(Math.hypot(thing.center[0] - point[0], thing.center[1] - point[1]) >= 60, `away from ${thing.center}`);
    assert.ok(point[0] >= 300, "not under the panel; a full-size widget does not block");
    assert.notDeepEqual(openGround(world, view, [point]), point, "a point that is used is avoided");
    assert.equal(openGround({ slot: 0, units: [], ui: [] }, view), null);
    assert.equal(openGround({ ...world, ui: [{ id: "all", visible: true, rect: [0, 0, 1280, 720], kind: "widget" }, ...[0, 1, 2, 3].flatMap((i) => [0, 1, 2, 3].map((j) => ({ id: `t${i}${j}`, visible: true, kind: "widget", rect: [i * 320, j * 180, 320, 180] })))] }, view), null, "covered everywhere");
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
        const viewport = probeOptions.viewport ?? { width: 1280, height: 720 };
        await page.setViewportSize(viewport);
        const dir = path.join(tempDir(`run-${kind}`), "evidence");
        const session = new ColdSession({ page, dir, viewport, limitMs: 240000, mode: `probe-${kind}`, control: probeOptions.control ?? null,
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

const GREEN = { startOverlay: true, guide: null, alerts: 0, units: 1 };
const FULL = { ...GREEN, units: 3 };
const GUIDE = { startOverlay: true, guide: "seethrough", guideWords: 80, alerts: 2, units: 1 };
const T = { skip, timeout: 240000 };
const inputsOf = (dir) => fs.readFileSync(path.join(dir, "inputs.jsonl"), "utf8").trim().split("\n").map((line) => JSON.parse(line));

// ---- The first-click probe: P1 (units), P3 (orders), P4 (extra clicks) ----------------------------------------

test("first-click probe, green page: units, orders and extra clicks all answer; one start click, no key", T, async () => {
    const { dir, record } = await probe("first-click", FULL);
    assert.equal(record.summary.verdict, "PASS", JSON.stringify(record.summary));
    assert.equal(record.clicks.length, 3, "the three own units, not the enemy");
    for (const click of record.clicks) {
        assert.ok(click.selection.includes(click.unit_id), JSON.stringify(click));
        assert.ok(click.visible_change && click.ok, JSON.stringify(click));
    }
    assert.deepEqual(record.start, { found: "focus_overlay", point: [640, 360] }, "the start click goes to the start overlay");
    assert.equal(record.summary.orders.verdict, "PASS", JSON.stringify(record.summary.orders));
    assert.deepEqual(record.orders.map((order) => order.text), ["Disable [D]", "Hold [H]", "Remove [X]"], "one unit of the content id; the destructive order last");
    assert.equal(record.summary.extras.verdict, "PASS", JSON.stringify(record.extras));
    assert.deepEqual(Object.keys(record.extras), ["enemy", "node", "ground_clear", "right_click"]);
    assert.equal(record.extras.ground_clear.selection_after.length, 0);
    const inputs = inputsOf(dir);
    assert.ok(inputs.every((input) => input.cmd !== "key"), "no key");
    assert.equal(inputs[0].cmd, "click", "the first input is the start click");
    assert.deepEqual(inputs.filter((input) => input.cmd === "click" && input.args.button === "right").length, 1, "one right click");
    assert.deepEqual(read(dir, "probe-first-click.json").summary, record.summary, "the record is on the disk");
});

test("first-click probe, see-through full-size guide: it FAILS at the first unit; orders and extras are skipped", T, async () => {
    const { record } = await probe("first-click", GUIDE);
    assert.equal(record.summary.verdict, "FAIL");
    assert.equal(record.summary.first_failure.unit_id, record.clicks[0].unit_id);
    for (const click of record.clicks) assert.deepEqual([click.selection, click.visible_change, click.selection_ok], [[], false, false]);
    assert.match(record.summary.reason, /first click on unit \d+ \(fixture:own\): selection \[\], no visible change/);
    assert.equal(record.summary.orders.verdict, "SKIPPED");
    assert.equal(record.summary.extras.verdict, "SKIPPED");
});

test("first-click probe, opaque guide: still FAILS (the click is taken), but it is not a see-through layer", T, async () => {
    const { record } = await probe("first-click", { startOverlay: true, guide: "opaque", guideWords: 12, alerts: 0, units: 1 });
    assert.equal(record.summary.verdict, "FAIL");
});

test("first-click probe, positive control: after the tool closes the guide the probe passes, with orders and extras", T, async () => {
    const { dir, record } = await probe("first-click", GUIDE, { control: "close-layer" });
    assert.equal(record.summary.verdict, "PASS", JSON.stringify(record.summary));
    assert.equal(record.layer_close.closed, true);
    assert.ok(record.layer_close.clicks >= 1 && record.layer_close.clicks <= 3);
    assert.equal(record.summary.orders.verdict, "PASS");
    assert.equal(read(dir, "session.json").control, "close-layer");
});

test("first-click probe: a ring around the unit alone is a visible change by the box rule, and the orders may be missing (P3 FAIL)", T, async () => {
    const { record } = await probe("first-click", { ...GREEN, noPanel: true, orders: false });
    assert.equal(record.summary.verdict, "PASS", JSON.stringify(record.clicks[0]));
    for (const click of record.clicks) assert.ok(click.box_seen && !click.panel_seen && click.box_changed >= 40, JSON.stringify(click));
    assert.equal(record.summary.orders.verdict, "FAIL");
    assert.match(record.summary.orders.reason, /no order button was visible after any selection/);
});

test("first-click probe on a page with no own unit on screen is a FAIL, not a pass", T, async () => {
    const { record } = await probe("first-click", { ...GREEN, units: 0 });
    assert.equal(record.summary.verdict, "FAIL");
    assert.match(record.summary.reason, /no own unit on screen/);
});

test("first-click probe: a click that selects but shows nothing is a FAIL; a visible change with no selection is a FAIL", T, async () => {
    const quiet = (await probe("first-click", { ...GREEN, selectShows: false })).record;
    assert.equal(quiet.summary.verdict, "FAIL");
    assert.ok(quiet.clicks.every((click) => click.selection_ok && !click.visible_change));
    const wrong = (await probe("first-click", { ...GREEN, selectsNothing: true })).record;
    assert.equal(wrong.summary.verdict, "FAIL");
    assert.ok(wrong.clicks.every((click) => click.visible_change && !click.selection_ok), JSON.stringify(wrong.clicks[0]));
});

test("a change elsewhere on the page does not make a click seen: a block and an alert line that appear by themselves are noise", T, async () => {
    const { record } = await probe("first-click", { ...GREEN, selectShows: false, noise: true });
    assert.equal(record.summary.verdict, "FAIL", "the selection shows nothing; the noise is far from the unit");
    const click = record.clicks[0];
    assert.ok(click.selection_ok && !click.box_seen && !click.panel_seen && !click.visible_change, JSON.stringify(click));
});

test("an order button with no visible result is a P3 FAIL; the other orders still pass", T, async () => {
    const { record } = await probe("first-click", { ...GREEN, ordersDead: true });
    assert.equal(record.summary.verdict, "PASS");
    assert.equal(record.summary.orders.verdict, "FAIL");
    assert.deepEqual(record.orders.filter((order) => !order.acted).map((order) => order.text), ["Disable [D]"]);
    assert.match(record.summary.orders.reason, /no visible result within 1 s for Disable \[D\] \(fixture:own\)/);
    assert.ok(record.orders.filter((order) => order.acted).every((order) => order.notice), "Hold and Remove answer with a notice within 1 s");
});

test("each extra click has a red case: an enemy with no panel, a node with no name, ground that keeps the selection, a right click that shows nothing", { skip, timeout: 400000 }, async () => {
    for (const [option, name] of [["enemyShows", "enemy"], ["nodeShows", "node"], ["groundKeeps", "ground_clear"], ["rightDead", "right_click"]]) {
        const { record } = await probe("first-click", { ...GREEN, [option]: option === "groundKeeps" || option === "rightDead" });
        assert.equal(record.summary.extras.verdict, "FAIL", `${option}: ${JSON.stringify(record.extras)}`);
        assert.deepEqual(record.summary.extras.failed, [name], option);
    }
});

test("a second window size: at 1024x768 the Close control of the guide is outside the page (P5), and the control cannot close the layer", T, async () => {
    const viewport = { width: 1024, height: 768 };
    const plain = (await probe("first-click", GUIDE, { viewport })).record;
    assert.deepEqual(plain.viewport, viewport);
    assert.ok(plain.buttons_outside_page.includes("guide_close"), JSON.stringify(plain.buttons_outside_page));
    const control = await probe("first-click", GUIDE, { viewport, control: "close-layer" });
    assert.equal(control.record.layer_close.found, false);
    assert.equal(control.record.layer_close.closed, false);
    assert.deepEqual(control.record.layer_close.off_page, ["guide_close"]);
    const out = check([control.dir]);
    assert.equal(out.status, 1, out.out);
    assert.match(out.out, /FAIL PC .*the close control is outside the page: \['guide_close'\]/);
    assert.match(out.out, /CONTROL-FAIL/);
    const ok = await probe("first-click", GREEN, { viewport });
    assert.deepEqual(ok.record.buttons_outside_page, []);
    assert.equal(read(ok.dir, "session.json").viewport.width, 1024);
});

// ---- UI-record checks (F9, F9b, F3, F6) --------------------------------------------------------------------

test("the UI records are taken before the first input and after the first click, and are not on the disk before the release", T, async () => {
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
        assert.equal(records[0].alert_lines, 2, "two alert lines before the first input");
        assert.ok(records[0].widgets.some((widget) => widget.id === "guide"));
        assert.ok(records[1].widgets.some((widget) => widget.id === "guide_copy" && wordCount(widget.text) === 80));
        const big = records[1].big_widgets.find((widget) => widget.id === "guide");
        assert.ok(big.area_share > 0.9 && big.nonflat_share > 0.05 && big.capture_root, JSON.stringify(big));
    } finally { await page.close(); }
});

test("checker, UI records: a long text (F9), a long text split in pieces (F9b), a see-through layer (F3) and an alert line (F6) are FAILS; the green page is not", T, async () => {
    const green = await probe("first-click", GREEN);
    const buttons = await probe("buttons", { ...GREEN, startOverlay: false, helpButton: true });
    const ok = check([green.dir, buttons.dir], ["--partial"]);
    assert.doesNotMatch(ok.out, /FAIL (F9|F9b|F3|F6)/, ok.out);
    for (const id of ["F9", "F9b", "F3", "F6"]) assert.match(ok.out, new RegExp(`PASS ${id} `));
    assert.match(ok.out, /INFO G1 .*no visible text that states a goal/);
    const guide = await probe("first-click", GUIDE);
    const bad = check([guide.dir], ["--partial"]);
    assert.equal(bad.status, 1, bad.out);
    assert.match(bad.out, /FAIL F9 .*guide_copy: 80 words/);
    assert.match(bad.out, /FAIL F9b /);
    assert.match(bad.out, /FAIL F3 .*guide/);
    assert.match(bad.out, /FAIL F6 .*2 alert lines/);
    assert.match(bad.out, /FAIL P1 .*first click on unit/);
    assert.match(bad.out, /NOT-READY/);
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

// ---- The buttons probe: F8-start and F8 ---------------------------------------------------------------------

test("buttons probe: a button under the start overlay loses its first click (F8-start, a finding of its own); after the start it acts (F8 passes)", T, async () => {
    const { dir, record } = await probe("buttons", { ...GREEN, helpButton: true });
    assert.equal(record.summary.start.verdict, "FAIL");
    assert.equal(record.f8_start.id, "help");
    assert.equal(record.f8_start.acted, false);
    assert.equal(record.f8_start.overlay_left, true, "the overlay took the click and left");
    assert.equal(record.summary.verdict, "PASS", "after the start the same button acts");
    assert.equal(record.buttons[0].acted, true);
    assert.deepEqual(inputsOf(dir).map((input) => input.cmd), ["move", "click", "move", "click"], "the pointer rests, one click under the overlay; then the same for the F8 list");
    const failed = check([dir], ["--partial"]);
    assert.match(failed.out, /FAIL F8s .*help/);
    assert.doesNotMatch(failed.out, /FAIL F8 /);
    assert.match(failed.out, /NOT-READY/);
});

test("buttons probe: with no start overlay a button that acts passes both; the checker says so", T, async () => {
    const { dir, record } = await probe("buttons", { ...GREEN, startOverlay: false, helpButton: true });
    assert.equal(record.summary.start.verdict, "PASS");
    assert.equal(record.summary.verdict, "PASS", JSON.stringify(record));
    assert.ok(record.buttons[0].changed_ids.includes("help_panel"));
    const passed = check([dir], ["--partial"]);
    assert.match(passed.out, /PASS F8 /);
    assert.match(passed.out, /PASS F8s /);
});

test("buttons probe: a start overlay that vanishes on the first click does not count as the button acting", T, async () => {
    const { record } = await probe("buttons", { ...GREEN, helpButton: true, helpDead: true });
    assert.equal(record.f8_start.acted, false);
    assert.deepEqual(record.f8_start.changed_ids, []);
    assert.equal(record.summary.verdict, "FAIL", "the dead button does nothing after the start either");
});

test("buttons probe: what the pointer resting on a button shows (a tooltip, a glow) is not the click acting", T, async () => {
    const { record } = await probe("buttons", { ...GREEN, startOverlay: false, helpButton: true, helpDead: true, hover: true });
    assert.equal(record.buttons[0].acted, false, JSON.stringify(record.buttons[0]));
    assert.deepEqual(record.buttons[0].changed_ids, []);
    const live = await probe("buttons", { ...GREEN, startOverlay: false, helpButton: true, hover: true });
    assert.equal(live.record.buttons[0].acted, true, "a button that acts is still seen to act");
    assert.deepEqual(live.record.buttons[0].changed_ids, ["help_panel"]);
});

test("the probes watch the page with no input: an alert line that comes in that time is F6; with no wait it is missed", T, async () => {
    const options = { ...GREEN, alerts: 2, alertsLateMs: 1500 };
    const hasty = await probe("first-click", options, { settleMs: 0 });
    assert.deepEqual(hasty.record.watch, []);
    assert.equal(read(hasty.dir, "ui-records.json").records[0].alert_lines, 0, "the first input came before the alerts");
    const patient = await probe("first-click", options, { settleMs: 2500 });
    assert.ok(patient.record.watch.length >= 2 && patient.record.watch[0].alert_lines === 0, JSON.stringify(patient.record.watch));
    assert.equal(patient.record.watch.at(-1).alert_lines, 2, "the alerts are seen by the watch");
    assert.equal(patient.record.settle_ms, 2500);
    assert.match(check([patient.dir], ["--partial"]).out, /FAIL F6 .*first at [\d.]+ s: 2 alert lines/);
    const buttons = await probe("buttons", { ...GREEN, startOverlay: false, helpButton: true, alerts: 1, alertsLateMs: 1500 }, { settleMs: 2500 });
    assert.ok(buttons.record.buttons.some((button) => /alert_line/.test(button.id)), "the button list is read after the watch");
    assert.match(check([buttons.dir], ["--partial"]).out, /FAIL F6 /);
});

// ---- The verdict line ----------------------------------------------------------------------------------

test("checker verdict: NOT-READY when a probe fails or a part is missing; a control is judged on its probe", T, async () => {
    const green = await probe("first-click", GREEN);
    const buttons = await probe("buttons", { ...GREEN, startOverlay: false, helpButton: true });
    const both = check([green.dir, buttons.dir], ["--partial"]);
    assert.equal(both.status, 0, both.out);
    assert.match(both.out, /RESULT: PARTIAL-PASS/);
    const missing = check([green.dir]);
    assert.equal(missing.status, 1, missing.out);
    assert.match(missing.out, /NOT-READY \(missing: first-click probe 1024x768, buttons probe, agent session, reference run \(C5\)\)/);
    assert.doesNotMatch(missing.out, /MACHINE-PASS/);
    const control = (await probe("first-click", GUIDE, { control: "close-layer" })).dir;
    const ctl = check([control]);
    assert.equal(ctl.status, 0, ctl.out);
    assert.match(ctl.out, /CONTROL-PASS/);
    assert.doesNotMatch(ctl.out, /FAIL F3|FAIL F9/, "a control run is judged on the probe only");
});
