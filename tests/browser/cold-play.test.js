// turts-test-role: unit
"use strict";

// Q-COLD: the cold-play step tool. The session (cold-session.js) runs here against a fake page with
// a fake clock, so the 60 s clock and the session limit need no wait. Each test names the build that
// makes it red: the plants of docs/research/mp1/plants/q-cold.json edit the tool and run this file.
// One case runs a real headless Chrome (skipped when there is none): it shows that a click that a
// page takes gives a different +200 ms picture, and that a click that a full-screen layer takes
// gives the same picture (the failure signs F2 and F3 are visible to the tool).
const assert = require("node:assert/strict");
const { execFile, spawn, spawnSync } = require("node:child_process");
const fs = require("node:fs");
const http = require("node:http");
const net = require("node:net");
const os = require("node:os");
const path = require("node:path");
const test = require("node:test");
const { ColdSession, listen, parseArgs, parseStatement, readSounds, MAX_LIMIT_S } = require("./cold-session.js");
const { readWorld } = require("./cold-probe.js");
const play = require("./cold-play.js");

const REPO = path.resolve(__dirname, "../..");
const CHECKER = path.join(REPO, "scripts/qa/cold_play_check.py");
const CLIENT = path.join(__dirname, "cold-play.js");
const SESSION_JS = path.join(__dirname, "cold-session.js");
const SCRIPT = path.join(REPO, "scripts/cold-play.sh");
const SENTINEL = "SENTINEL_SOUND_ID";
const BRIDGE_SENTINEL = "SENTINEL_BRIDGE_VALUE";
const UI_SENTINEL = "SENTINEL_UI_TEXT";

// ---- A fake page and a fake clock ------------------------------------------------------------

// A sleep advances the clock in steps of 500 ms and calls `hook` at each step: the hook is the sampler
// of the session, which a real session runs on a 500 ms timer.
function fakeClock() {
    let ms = 5000;
    const clock = { hook: null, now: () => ms };
    clock.sleep = async (delay) => {
        for (let left = delay; left > 0;) {
            const step = Math.min(500, left);
            ms += step;
            left -= step;
            if (clock.hook) await clock.hook();
        }
    };
    return clock;
}

// `observations` is the list that the fake debug bridge publishes; a test pushes to it.
function fakePage(observations) {
    const calls = [];
    let shots = 0;
    const window = {
        turts_debug: () => "",
        turts_debug_request: () => Promise.resolve(JSON.stringify({ audio: { context_state: BRIDGE_SENTINEL, sink_observations: observations },
            bindings: BRIDGE_SENTINEL, slot: 0, selection: [], alerts: [], replicated_world: { units: [] } })),
        turts_scene: () => JSON.stringify({ units: [] }),
        turts_ui: () => JSON.stringify([{ id: "hud", kind: "widget", rect: [0, 0, 10, 10], text: UI_SENTINEL, visible: true, enabled: true, capture_root: true }]),
    };
    const page = {
        calls,
        mouse: {
            move: async (x, y) => calls.push(["move", x, y]),
            down: async (options) => calls.push(["down", options.button]),
            up: async (options) => calls.push(["up", options.button]),
            wheel: async (dx, dy) => calls.push(["wheel", dx, dy]),
        },
        keyboard: {
            down: async (key) => { if (!/^(?:Key[A-Z]|F\d{1,2}|Escape|Space|Digit\d)$/.test(key)) throw new Error(`Unknown key: "${key}"`); calls.push(["keydown", key]); },
            up: async (key) => calls.push(["keyup", key]),
        },
        screenshot: async ({ path: file, scale }) => { shots += 1; calls.push(["shot", scale]); fs.writeFileSync(file, Buffer.from(`PNG ${shots}`)); },
        evaluate: async (fn) => {
            if (fn === readSounds || fn === readWorld) return new Function("window", `return (${fn.toString()})()`)(window);
            return { width: 1280, height: 720 };
        },
    };
    return page;
}

const TEMP = [];
function tempDir(label) {
    const dir = fs.mkdtempSync(path.join(os.tmpdir(), `cold-play-${label}-`));
    TEMP.push(dir);
    return dir;
}
test.after(() => { for (const dir of TEMP) fs.rmSync(dir, { recursive: true, force: true }); });

async function makeSession(options = {}) {
    const clock = fakeClock();
    const observations = [];
    const page = fakePage(observations);
    const dir = path.join(tempDir("session"), "evidence");
    const session = new ColdSession({ page, dir, now: clock.now, sleep: clock.sleep, viewport: { width: 1280, height: 720 }, meta: { build: "test" },
        ...options });
    clock.hook = () => session.sample();
    await session.start();
    return { session, page, clock, observations, dir, call: (cmd, ...args) => session.handle({ cmd, args }) };
}

function allFiles(dir) {
    return fs.readdirSync(dir, { withFileTypes: true }).flatMap((entry) => (entry.isDirectory() ? allFiles(path.join(dir, entry.name)) : [path.join(dir, entry.name)]));
}
const events = (dir) => fs.readFileSync(path.join(dir, "events.jsonl"), "utf8").trim().split("\n").map((line) => JSON.parse(line));
const sentinelFiles = (dir, needle) => allFiles(dir).filter((file) => fs.readFileSync(file).includes(needle));

const STATEMENT = (shot) => [`GOAL: win the match`, `GOAL_SHOT: ${shot}`, `FIRST_ACTION: click the big button`, `FIRST_ACTION_SHOT: ${shot}`,
    `WORKED: the panel opened`, `WORKED_SHOT: ${shot}`].join("\n");

// ---- Inputs and the input log ----------------------------------------------------------------

test("each input is done with real mouse and keyboard calls and is logged with its times", async () => {
    const ctx = await makeSession();
    assert.equal((await ctx.call("move", "10", "20")).exit, 0);
    assert.equal((await ctx.call("click", "100", "200", "left")).exit, 0);
    assert.equal((await ctx.call("click", "300", "400", "right")).exit, 0);
    assert.equal((await ctx.call("drag", "50", "60", "150", "160")).exit, 0);
    assert.equal((await ctx.call("wheel", "-120")).exit, 0);
    assert.equal((await ctx.call("key", "F1")).exit, 0);
    assert.equal((await ctx.call("wait", "2")).exit, 0);
    const calls = ctx.page.calls.filter((call) => call[0] !== "shot");
    assert.deepEqual(calls.slice(0, 7), [["move", 10, 20], ["move", 100, 200], ["down", "left"], ["up", "left"], ["move", 300, 400], ["down", "right"], ["up", "right"]]);
    const drag = calls.slice(7, 7 + 13);
    assert.deepEqual(drag[0], ["move", 50, 60]);
    assert.deepEqual(drag[1], ["down", "left"]);
    assert.deepEqual(drag.slice(2, 12).at(-1), ["move", 150, 160]);
    assert.deepEqual(drag[12], ["up", "left"]);
    assert.deepEqual(calls.slice(20), [["wheel", 0, -120], ["keydown", "F1"], ["keyup", "F1"]]);
    const inputs = fs.readFileSync(path.join(ctx.dir, "inputs.jsonl"), "utf8").trim().split("\n").map((line) => JSON.parse(line));
    assert.deepEqual(inputs.map((input) => [input.n, input.cmd]), [[1, "move"], [2, "click"], [3, "click"], [4, "drag"], [5, "wheel"], [6, "key"], [7, "wait"]]);
    assert.deepEqual(inputs.map((input) => input.counts_as_input), [true, true, true, true, true, true, false]);
    assert.deepEqual(inputs[2].args, { x: 300, y: 400, button: "right" });
    assert.deepEqual(inputs[5].args, { code: "F1" });
    for (const input of inputs) {
        assert.ok(Number.isInteger(input.t_start_ms) && input.t_end_ms >= input.t_start_ms, `times of call ${input.n}`);
        if (input.cmd === "click") assert.ok(input.t_down_ms > input.t_start_ms && input.t_down_ms < input.t_end_ms, "the press is inside the click");
    }
    assert.ok(inputs.every((input, index) => index === 0 || input.t_start_ms >= inputs[index - 1].t_end_ms), "the calls do not overlap");
    assert.equal(events(ctx.dir).filter((event) => event.kind === "first_click").length, 1, "one first click");
});

test("a move or a key that is not valid is refused, logged as no input and does not start the clock", async () => {
    const ctx = await makeSession();
    assert.equal((await ctx.call("click", "1280", "10", "left")).exit, 2, "a point outside the page");
    assert.equal((await ctx.call("click", "10", "10", "middle")).exit, 2);
    assert.equal((await ctx.call("click", "ten", "10", "left")).exit, 2);
    assert.equal((await ctx.call("wheel", "50")).exit, 2, "the wheel needs a pointer");
    assert.equal((await ctx.call("key", "NoSuchKey")).exit, 2, "an unknown key");
    assert.equal((await ctx.call("key", "Control+KeyA")).exit, 2, "one key, not a combination");
    assert.equal((await ctx.call("wait", "31")).exit, 2);
    assert.equal((await ctx.call("hover", "1", "1")).exit, 2);
    assert.equal(ctx.session.firstClickMs, null);
    assert.equal(fs.existsSync(path.join(ctx.dir, "inputs.jsonl")), false, "nothing was logged as an input");
    assert.deepEqual(ctx.page.calls.filter((call) => call[0] !== "shot"), [], "no event reached the page");
});

test("each call returns two screenshots, 200 ms and 1000 ms after the input ended, in CSS pixels", async () => {
    const ctx = await makeSession();
    const reply = await ctx.call("click", "100", "100", "left");
    const shots = reply.lines.filter((line) => line.startsWith("screenshot"));
    assert.deepEqual(shots.map((line) => line.slice(0, 20)), ["screenshot +200 ms: ", "screenshot +1000 ms:"]);
    const files = shots.map((line) => line.slice(line.indexOf(": ") + 2));
    for (const file of files) assert.ok(fs.statSync(file).size > 0, file);
    assert.match(files[0], /0001-click-p200\.png$/);
    assert.match(files[1], /0001-click-p1000\.png$/);
    const all = events(ctx.dir);
    const input = all.find((event) => event.kind === "input");
    const taken = all.filter((event) => event.kind === "shot" && event.file.startsWith("0001-"));
    assert.deepEqual(taken.map((shot) => shot.t_start_ms - input.t_end_ms), [200, 1000], "the screenshots start at +200 ms and +1000 ms");
    assert.ok(ctx.page.calls.filter((call) => call[0] === "shot").every((call) => call[1] === "css"));
    assert.ok(fs.existsSync(path.join(ctx.dir, "shots", "0000-start.png")), "a start picture before any input");
});

// ---- The statement gate ----------------------------------------------------------------------

test("the sound records are not on the disk before the statement is stored; the statement is stored first", async () => {
    const ctx = await makeSession();
    ctx.observations.push({ sound_id: SENTINEL, category: "alerts", play_id: 1, player_entity: 11, request_frame_index: 5 });
    await ctx.session.sample();
    await ctx.call("click", "100", "100", "left");
    await ctx.session.sample();
    assert.deepEqual(sentinelFiles(ctx.dir, SENTINEL), [], "no file holds a sound id before the statement");
    assert.deepEqual(sentinelFiles(ctx.dir, UI_SENTINEL), [], "no file holds a UI text before the statement");
    assert.equal(fs.existsSync(path.join(ctx.dir, "sound-records.json")), false);
    const early = await ctx.session.handle({ cmd: "statement", text: STATEMENT("0001-click-p200.png") });
    assert.equal(early.exit, 3, "before 60 s after the first click the statement is refused");
    assert.match(early.lines[0], /too early/);
    assert.equal(fs.existsSync(path.join(ctx.dir, "statement.txt")), false);
    assert.deepEqual(sentinelFiles(ctx.dir, SENTINEL), [], "a refused statement opens nothing");
    await ctx.call("wait", "30");
    await ctx.call("wait", "30");
    const bad = await ctx.session.handle({ cmd: "statement", text: "GOAL: win\nGOAL_SHOT: shots/nothing.png" });
    assert.equal(bad.exit, 3);
    assert.ok(bad.lines.some((line) => /FIRST_ACTION is missing/.test(line)) && bad.lines.some((line) => /GOAL_SHOT: .* is not a screenshot file of this session/.test(line)));
    assert.deepEqual(sentinelFiles(ctx.dir, SENTINEL), [], "a statement with problems opens nothing");
    const ok = await ctx.session.handle({ cmd: "statement", text: STATEMENT("0001-click-p200.png") });
    assert.equal(ok.exit, 0, ok.lines.join("\n"));
    assert.ok(sentinelFiles(ctx.dir, SENTINEL).some((file) => file.endsWith("sound-records.json")), "the records are on the disk after the statement");
    assert.ok(sentinelFiles(ctx.dir, UI_SENTINEL).some((file) => file.endsWith("ui-records.json")), "the UI records are released with them");
    const log = events(ctx.dir);
    const stored = log.find((event) => event.kind === "statement_stored");
    const released = log.find((event) => event.kind === "sounds_released");
    assert.ok(stored && released && stored.seq < released.seq, "statement_stored comes before sounds_released");
    assert.equal(released.reason, "statement");
    assert.equal(fs.readFileSync(path.join(ctx.dir, "statement.txt"), "utf8"), STATEMENT("0001-click-p200.png"), "the statement is stored as given");
    const again = await ctx.session.handle({ cmd: "statement", text: STATEMENT("0001-click-p200.png") });
    assert.equal(again.exit, 3, "a second statement is refused");
});

test("a statement before any click is refused, and an end with no statement still releases at the end", async () => {
    const ctx = await makeSession();
    ctx.observations.push({ sound_id: SENTINEL, category: "effects", play_id: 1, player_entity: 11, request_frame_index: 5 });
    await ctx.session.sample();
    const reply = await ctx.session.handle({ cmd: "statement", text: STATEMENT("0000-start.png") });
    assert.equal(reply.exit, 3);
    assert.match(reply.lines[0], /no click or drag yet/);
    assert.deepEqual(sentinelFiles(ctx.dir, SENTINEL), []);
    assert.equal((await ctx.call("end")).exit, 0);
    const released = events(ctx.dir).find((event) => event.kind === "sounds_released");
    assert.equal(released.reason, "end-without-statement", "the log says that no statement came first");
    assert.ok(sentinelFiles(ctx.dir, SENTINEL).length === 1);
});

test("after 60 s from the first click every input is refused until the statement; wait and status are not", async () => {
    const ctx = await makeSession();
    await ctx.call("click", "10", "10", "left");
    const sixty = await ctx.call("wait", "30");
    assert.match(sixty.lines.join("\n"), /clock: \d+\.\d s since your first click\. Your statement is due at 60 s\./);
    const due = await ctx.call("wait", "30");
    assert.match(due.lines.join("\n"), /STATEMENT DUE: 6\d s have passed/, "the tool says that the 60 s have passed");
    for (const args of [["move", "5", "5"], ["click", "5", "5", "left"], ["drag", "1", "1", "9", "9"], ["wheel", "10"], ["key", "KeyA"]]) {
        const reply = await ctx.call(...args);
        assert.equal(reply.exit, 3, `${args[0]} is refused`);
        assert.match(reply.lines[0], /refused:/);
    }
    assert.equal((await ctx.call("status")).exit, 0);
    assert.equal((await ctx.call("wait", "1")).exit, 0, "wait is time, not input");
    const before = ctx.page.calls.length;
    assert.equal((await ctx.call("key", "KeyA")).exit, 3);
    assert.equal(ctx.page.calls.length, before, "a refused input never reaches the page");
    assert.equal((await ctx.session.handle({ cmd: "statement", text: STATEMENT("0001-click-p200.png") })).exit, 0);
    assert.equal((await ctx.call("key", "KeyA")).exit, 0, "input goes on after the statement");
    assert.equal(events(ctx.dir).filter((event) => event.kind === "statement_due").length, 1);
});

// ---- The session limit -----------------------------------------------------------------------

test("a session ends at its limit with the evidence complete, and later calls are refused", async () => {
    const ctx = await makeSession({ limitMs: 20000 });
    ctx.observations.push({ sound_id: SENTINEL, category: "effects", play_id: 1, player_entity: 11, request_frame_index: 5 });
    const waited = await ctx.call("wait", "30");
    assert.match(waited.lines[0], /cut short by the session limit/);
    await ctx.session.finish("limit");
    const end = JSON.parse(fs.readFileSync(path.join(ctx.dir, "end.json"), "utf8"));
    assert.equal(end.reason, "limit");
    assert.ok(end.t_ms >= 20000 && end.t_ms < 20000 + 5000, `ended at ${end.t_ms}`);
    assert.equal(events(ctx.dir).at(-1).kind, "end");
    const released = JSON.parse(fs.readFileSync(path.join(ctx.dir, "sound-records.json"), "utf8"));
    assert.equal(released.final, true);
    assert.equal(released.records.length, 1);
    const after = await ctx.call("click", "1", "1", "left");
    assert.equal(after.exit, 4);
    assert.match(after.lines[0], /session has ended \(limit\)/);
    assert.equal(ctx.page.calls.filter((call) => call[0] === "down").length, 0, "no input after the limit");
});

test("the probe options: --probe first-click|buttons, --control close-layer only with the first-click probe, in the script and in the session", () => {
    assert.equal(parseArgs(["--port", "5000", "--session", "x", "--probe", "buttons"]).probe, "buttons");
    assert.deepEqual([parseArgs(["--port", "5000", "--session", "x", "--probe", "first-click", "--control", "close-layer"]).probe, parseArgs(["--port", "5000", "--session", "x", "--probe", "first-click", "--control", "close-layer"]).control], ["first-click", "close-layer"]);
    assert.throws(() => parseArgs(["--port", "5000", "--session", "x", "--probe", "dance"]), /--probe takes first-click or buttons/);
    assert.throws(() => parseArgs(["--port", "5000", "--session", "x", "--probe", "buttons", "--control", "close-layer"]), /--control close-layer goes with --probe first-click/);
    assert.throws(() => parseArgs(["--port", "5000", "--session", "x", "--control", "close-layer"]), /goes with --probe first-click/);
    const root = fakeRoot();
    const run = (...extra) => spawnSync("bash", [path.join(root, "scripts/cold-play.sh"), "--session", path.join(root, `p-${extra.join("-")}`), "--url", "http://127.0.0.1:1/", ...extra], { encoding: "utf8", timeout: 20000 });
    const good = run("--probe", "first-click", "--control", "close-layer");
    assert.equal(good.status, 0, good.stderr);
    assert.match(fs.readFileSync(path.join(root, "p---probe-first-click---control-close-layer/internal/session.log"), "utf8"), /--probe first-click --control close-layer/);
    assert.equal(run("--probe", "dance").status, 2);
    assert.equal(run("--probe", "buttons", "--control", "close-layer").status, 2);
    assert.equal(run("--control", "close-layer").status, 2);
});

test("the session limit has a ceiling under the hold limit of the lock wrapper", async () => {
    assert.equal(MAX_LIMIT_S, 1500);
    assert.throws(() => new ColdSession({ page: {}, dir: "x", limitMs: 1501 * 1000 }), /1 to 1500 s/);
    assert.throws(() => new ColdSession({ page: {}, dir: "x", limitMs: 0 }), /1 to 1500 s/);
    assert.throws(() => parseArgs(["--port", "5000", "--session", "x", "--limit-secs", "1501"]), /1 to 1500/);
    assert.equal(parseArgs(["--port", "5000", "--session", "x"]).limitSecs, 600, "the default is 10 minutes");
    assert.equal(parseArgs(["--port", "5000", "--session", "x"]).viewport.width, 1280);
    assert.equal(parseArgs(["--port", "5000", "--session", "x", "--viewport", "none"]).viewport, null);
    assert.throws(() => parseArgs(["--session", "x"]), /--port/);
    // The start script has the same ceiling. It runs in a fake root, so a plant in the check cannot start a real Chrome.
    const root = fakeRoot();
    for (const bad of ["0", "1501", "abc", "-5", "99999"]) {
        const run = spawnSync("bash", [path.join(root, "scripts/cold-play.sh"), "--session", path.join(root, `s-${bad}`), "--url", "http://127.0.0.1:1/", "--limit-secs", bad], { encoding: "utf8", timeout: 20000 });
        assert.equal(run.status, 2, `${bad}: ${run.stdout}${run.stderr}`);
        assert.match(run.stderr, /--limit-secs must be a whole number from 1 to 1500/);
        assert.equal(fs.existsSync(path.join(root, "play.pid")), false, "no game and no Chrome were started");
    }
    for (const good of ["1", "1500"]) {
        const run = spawnSync("bash", [path.join(root, "scripts/cold-play.sh"), "--session", path.join(root, `s-${good}`), "--url", "http://127.0.0.1:1/", "--limit-secs", good], { encoding: "utf8", timeout: 20000 });
        assert.equal(run.status, 0, `${good}: ${run.stderr}`);
        assert.match(fs.readFileSync(path.join(root, `s-${good}/internal/session.log`), "utf8"), new RegExp(`--limit-secs ${good} `));
    }
});

// ---- What the agent reads --------------------------------------------------------------------

test("the replies hold no bridge value: every line is one of the fixed forms", async () => {
    const ctx = await makeSession();
    ctx.observations.push({ sound_id: BRIDGE_SENTINEL, category: BRIDGE_SENTINEL, play_id: 7, player_entity: 8, request_frame_index: 9 });
    await ctx.session.sample();
    const replies = [];
    for (const args of [["wait", "1"], ["click", "640", "360", "left"], ["click", "9999", "1", "left"], ["key", "NoSuch"], ["move", "5", "5"], ["drag", "1", "1", "20", "20"],
        ["wheel", "30"], ["status"], ["wait", "30"], ["wait", "30"], ["move", "1", "1"]]) replies.push(await ctx.call(...args));
    replies.push(await ctx.session.handle({ cmd: "statement", text: "GOAL: x" }));
    replies.push(await ctx.session.handle({ cmd: "statement", text: STATEMENT("0001-click-p200.png") }));
    replies.push(await ctx.call("end"));
    replies.push(await ctx.call("status"));
    const allowed = [/^input \d+: (?:wait|move|click|drag|wheel|key)\b[^\n]*$/, /^screenshot \+(?:200|1000) ms: \/\S+\/shots\/\d{4}-[a-z]+-p(?:200|1000)\.png$/,
        /^clock: /, /^session time left: \d+ s$/, /^STATEMENT DUE: /, /^Write a file with six lines \(GOAL, GOAL_SHOT, FIRST_ACTION, FIRST_ACTION_SHOT, WORKED, WORKED_SHOT;/,
        /^refused: /, /^  - [A-Z_]+ is missing$|^  - [A-Z_]+: /, /^statement stored\.$/, /^sound records: \/\S+\/sound-records\.json \(they are released now;/,
        /^the session is over\. The evidence folder is complete\.$/, /^the session has ended \((?:limit|end)\)\. The evidence folder is complete\.$/,
        /^the point 9999 1 is outside the page, which is 1280 x 720 pixels$/, /^unknown key "NoSuch"$/, /^status$/];
    for (const line of replies.flatMap((reply) => reply.lines)) {
        assert.ok(allowed.some((pattern) => pattern.test(line)), `a reply line of an unknown form: ${line}`);
        assert.doesNotMatch(line, new RegExp(`${BRIDGE_SENTINEL}|turts_|sink_obs|play_id|player_entity|sound_id|bindings|hints|selection`));
    }
});

test("parseStatement: six parts, each with a screenshot file of the session", () => {
    const names = new Set(["0000-start.png", "0003-click-p200.png"]);
    assert.deepEqual(parseStatement(STATEMENT("0003-click-p200.png"), names).problems, []);
    assert.deepEqual(parseStatement(STATEMENT("shots/0003-click-p200.png"), names).problems, [], "a path with the file name is accepted");
    assert.ok(parseStatement("GOAL: a\nGOAL: b", names).problems.includes("line 2: GOAL is given two times"));
    assert.ok(parseStatement(STATEMENT("0009-click-p200.png"), names).problems.some((problem) => /is not a screenshot file of this session/.test(problem)));
    assert.ok(parseStatement(STATEMENT("../../etc/passwd"), names).problems.some((problem) => /is not a screenshot file/.test(problem)));
    assert.ok(parseStatement("goal: x", names).problems.some((problem) => /not "KEY: value"/.test(problem)));
    assert.ok(parseStatement(STATEMENT("0000-start.png").replace("win the match", ""), names).problems.includes("GOAL is empty"));
});

// ---- The control port and the agent's command ------------------------------------------------

async function runClient(args) {
    return new Promise((resolve) => execFile(process.execPath, [CLIENT, ...args], { encoding: "utf8", timeout: 30000 },
        (error, stdout, stderr) => resolve({ status: error ? (error.code ?? 1) : 0, stdout, stderr })));
}

test("the agent's command talks to the session over the loopback port and exits with the code of the reply", async () => {
    const ctx = await makeSession();
    let ended = false;
    const server = await listen(ctx.session, () => { ended = true; });
    try {
        assert.equal(server.address().address, "127.0.0.1", "loopback only");
        fs.writeFileSync(path.join(ctx.dir, "internal", "control.json"), JSON.stringify({ port: server.address().port }));
        const click = await runClient([ctx.dir, "click", "100", "100", "left"]);
        assert.equal(click.status, 0, click.stderr);
        assert.match(click.stdout, /^input 1: click 100 100 left\nscreenshot \+200 ms: .*0001-click-p200\.png\nscreenshot \+1000 ms: .*0001-click-p1000\.png\nclock: /);
        assert.equal((await runClient([ctx.dir, "click", "5000", "1", "left"])).status, 2);
        const usage = await runClient([ctx.dir, "dance"]);
        assert.equal(usage.status, 2);
        assert.match(usage.stderr, /usage: node tests\/browser\/cold-play\.js/);
        assert.equal((await runClient([])).status, 2);
        const file = path.join(path.dirname(ctx.dir), "statement.txt");
        fs.writeFileSync(file, STATEMENT("0001-click-p200.png"));
        assert.equal((await runClient([ctx.dir, "statement", file])).status, 3, "too early: the exit code of the refusal");
        assert.equal((await runClient([ctx.dir, "statement", path.join(path.dirname(ctx.dir), "none.txt")])).status, 2);
        const end = await runClient([ctx.dir, "end"]);
        assert.equal(end.status, 0);
        assert.ok(ended, "the server was told to stop");
        assert.equal(fs.existsSync(path.join(ctx.dir, "internal", "control.json")), false, "the control file is removed at the end");
        const gone = await runClient([ctx.dir, "status"]);
        assert.equal(gone.status, 5);
        assert.match(gone.stderr, /no session runs at that folder/);
    } finally { server.close(); }
});

test("the client has no browser library and prints only the reply of the session", () => {
    const source = fs.readFileSync(CLIENT, "utf8");
    assert.doesNotMatch(source.split("\n").filter((line) => !/^\s*\/\//.test(line)).join("\n"), /playwright|chromium|turts_|evaluate|screenshot/);
});

// ---- Round trip with the checker -------------------------------------------------------------

function check(dir, extra = []) {
    const run = spawnSync("python3", ["-I", "-B", CHECKER, dir, "--no-lock-check", "--partial", ...extra], { encoding: "utf8" });
    return { status: run.status, out: run.stdout, err: run.stderr };
}

async function cleanSession(setup) {
    const ctx = await makeSession();
    if (setup) await setup(ctx);
    await ctx.call("wait", "1");
    await ctx.call("click", "640", "360", "left");
    await ctx.call("key", "KeyF");
    await ctx.call("wait", "30");
    await ctx.call("wait", "30");
    const stored = await ctx.session.handle({ cmd: "statement", text: STATEMENT("0002-click-p200.png") });
    assert.equal(stored.exit, 0, stored.lines.join("\n"));
    await ctx.call("wait", "2");
    await ctx.session.finish("end");
    return ctx;
}

test("round trip: a clean session passes the checker, which lists the checks that stay manual", async () => {
    const ctx = await cleanSession();
    const result = check(ctx.dir);
    assert.equal(result.status, 0, result.out + result.err);
    assert.match(result.out, /MACHINE CHECKS/);
    assert.match(result.out, /PASS E1 /);
    assert.match(result.out, /PASS C3c /);
    assert.match(result.out, /PASS F5a /);
    assert.match(result.out, /MANUAL CHECKS/);
    for (const id of ["C0", "C1", "C2", "C4", "C5"]) assert.match(result.out, new RegExp(`TODO ${id} `));
    assert.match(result.out, /RESULT: PARTIAL-PASS/);
    assert.doesNotMatch(result.out, /PERSON-READY/);
    assert.match(result.out, /MATCH BY HAND: key KeyF at /, "each key is flagged for the reviewer");
});

test("round trip: the checker goes red on a session with no statement", async () => {
    const ctx = await makeSession();
    await ctx.call("wait", "1");
    await ctx.call("click", "640", "360", "left");
    await ctx.call("wait", "30");
    await ctx.call("wait", "30");
    await ctx.session.finish("limit");
    const result = check(ctx.dir);
    assert.equal(result.status, 1, result.out);
    assert.match(result.out, /FAIL C3a /);
    assert.match(result.out, /FAIL C3c /);
    assert.match(result.out, /RESULT: NOT-READY \(failed: C3a\[agent\], C3c\[agent\]\)/);
});

test("round trip: the checker goes red when one sound id plays 4 times in 60 s, and not for 3", async () => {
    const four = await cleanSession(async (ctx) => {
        for (let i = 1; i <= 4; i += 1) {
            ctx.observations.push({ sound_id: "pkg:beep", category: "alerts", play_id: i, player_entity: 100 + i, request_frame_index: i });
            await ctx.session.sample();
            await ctx.clock.sleep(5000);
        }
    });
    const first = check(four.dir);
    assert.equal(first.status, 1, first.out);
    assert.match(first.out, /FAIL F5a .*pkg:beep: 4 times/);
    const three = await cleanSession(async (ctx) => {
        for (let i = 1; i <= 3; i += 1) {
            ctx.observations.push({ sound_id: "pkg:beep", category: "effects", play_id: i, player_entity: 100 + i, request_frame_index: i });
            await ctx.session.sample();
            await ctx.clock.sleep(5000);
        }
    });
    assert.equal(check(three.dir).status, 0, check(three.dir).out);
});

test("round trip: the checker goes red on an alert sound before the first input, not on one after it", async () => {
    const early = await cleanSession(async (ctx) => {
        ctx.observations.push({ sound_id: "pkg:alert", category: "alerts", play_id: 1, player_entity: 1, request_frame_index: 1 });
        await ctx.session.sample();
    });
    const result = check(early.dir);
    assert.equal(result.status, 1, result.out);
    assert.match(result.out, /FAIL F5b .*pkg:alert at /);
    const late = await makeSession();
    await late.call("wait", "1");
    await late.call("click", "640", "360", "left");
    late.observations.push({ sound_id: "pkg:alert", category: "alerts", play_id: 1, player_entity: 1, request_frame_index: 1 });
    await late.session.sample();
    await late.call("wait", "30");
    await late.call("wait", "30");
    await late.session.handle({ cmd: "statement", text: STATEMENT("0002-click-p200.png") });
    await late.session.finish("end");
    assert.equal(check(late.dir).status, 0, check(late.dir).out);
});

test("round trip: a stored screenshot that was changed afterwards is found", async () => {
    const ctx = await cleanSession();
    fs.appendFileSync(path.join(ctx.dir, "shots", "0002-click-p200.png"), "x");
    const result = check(ctx.dir);
    assert.equal(result.status, 1);
    assert.match(result.out, /FAIL E2 .*0002-click-p200\.png: bytes differ/);
});

// ---- The start script ------------------------------------------------------------------------

// A copy of the start script in a folder with a fake play script (it prints a URL and a port, and starts no Chrome) and a fake session process.
function fakeRoot() {
    const root = tempDir("script");
    fs.mkdirSync(path.join(root, "scripts"), { recursive: true });
    fs.mkdirSync(path.join(root, "tests/browser/node_modules/playwright-core"), { recursive: true });
    fs.copyFileSync(SCRIPT, path.join(root, "scripts/cold-play.sh"));
    fs.writeFileSync(path.join(root, "scripts/mp1-play.sh"), `#!/usr/bin/env bash
echo $$ > '${path.join(root, "play.pid")}'
echo "URL: http://127.0.0.1:1/?x=1"
echo "DEBUG_PORT: 4321"
exec sleep 60
`);
    fs.writeFileSync(path.join(root, "tests/browser/cold-session.js"), "console.log('args', process.argv.slice(2).join(' '));\n");
    return root;
}

test("the start script ends its game and Chrome with the session, refuses an old folder, and reports a failed start", () => {
    assert.equal(spawnSync("bash", ["-n", SCRIPT]).status, 0, "bash syntax");
    const root = fakeRoot();
    const marker = path.join(root, "play.pid");
    const run = spawnSync("bash", [path.join(root, "scripts/cold-play.sh"), "--session", path.join(root, "out"), "--url", "http://127.0.0.1:1/?x=1", "--limit-secs", "30", "--build", "b1"], { encoding: "utf8", timeout: 30000 });
    assert.equal(run.status, 0, run.stderr);
    assert.match(fs.readFileSync(path.join(root, "out/internal/session.log"), "utf8"), /args --port 4321 --session \S+out --url http:\/\/127\.0\.0\.1:1\/\?x=1 --limit-secs 30 --viewport 1280x720 --build b1/);
    const pid = Number(fs.readFileSync(marker, "utf8"));
    try { assert.throws(() => process.kill(pid, 0), "the play script is gone when the session ends"); } finally { try { process.kill(pid); } catch { /* gone */ } }
    const again = spawnSync("bash", [path.join(root, "scripts/cold-play.sh"), "--session", path.join(root, "out"), "--url", "http://127.0.0.1:1/"], { encoding: "utf8" });
    assert.equal(again.status, 2);
    assert.match(again.stderr, /exists; a session writes a new folder/);
    fs.writeFileSync(path.join(root, "scripts/mp1-play.sh"), "#!/usr/bin/env bash\necho 'mp1-play: the server did not start' >&2\nexit 66\n");
    const failed = spawnSync("bash", [path.join(root, "scripts/cold-play.sh"), "--session", path.join(root, "out2"), "--prepared", "/none"], { encoding: "utf8", timeout: 30000 });
    assert.equal(failed.status, 66, failed.stderr);
    assert.match(failed.stderr, /the game or Chrome did not start/);
    for (const bad of [[], ["--session", path.join(root, "o3")], ["--session", path.join(root, "o3"), "--url", "u", "--prepared", "p"], ["--session", path.join(root, "o3"), "--package", "x", "--url", "u"]]) {
        assert.equal(spawnSync("bash", [path.join(root, "scripts/cold-play.sh"), ...bad], { encoding: "utf8" }).status, 2, JSON.stringify(bad));
    }
});

test("mp1-play.sh --url takes only a loopback game address and starts no Chrome for another one", () => {
    const folder = tempDir("playurl");
    const marker = path.join(folder, "chrome-was-started");
    const fake = path.join(folder, "chrome");
    fs.writeFileSync(fake, `#!/bin/sh\ntouch '${marker}'\n`, { mode: 0o755 });
    const PLAY = path.join(REPO, "scripts/mp1-play.sh");
    for (const url of ["http://example.com/", "https://127.0.0.1.example.com/", "http://localhost@example.com/", "ftp://127.0.0.1/", "http://127.0.0.1"+"x/"]) {
        const run = spawnSync("bash", [PLAY, "--url", url], { encoding: "utf8", env: { ...process.env, CHROME_BIN: fake }, timeout: 20000 });
        assert.equal(run.status, 2, `${url}: ${run.stdout}${run.stderr}`);
        assert.match(run.stderr, /--url takes a loopback address/);
        assert.equal(fs.existsSync(marker), false, `no Chrome for ${url}`);
    }
    assert.equal(spawnSync("bash", [PLAY, "--url"], { encoding: "utf8" }).status, 2, "a missing address is a usage error");
    // A loopback address passes the check and gets as far as the Chrome start (the fake Chrome that does not listen).
    const ok = spawnSync("bash", [PLAY, "--url", "http://127.0.0.1:9/?a=1"], { encoding: "utf8", env: { ...process.env, CHROME_BIN: fake }, timeout: 30000 });
    assert.equal(fs.existsSync(marker), true, "a loopback address starts the Chrome");
    assert.match(ok.stderr, /Chrome did not listen on port/);
});

test("the tool never fixes the debugging port, mutes Chrome or touches the system sound; one writer of the sound file", () => {
    const text = (file) => fs.readFileSync(file, "utf8");
    const code = (file) => text(file).split("\n").filter((line) => !/^\s*(?:#|\/\/)/.test(line)).join("\n");
    for (const file of [SCRIPT, CLIENT, SESSION_JS, CHECKER]) {
        assert.doesNotMatch(text(file), new RegExp("92" + "22"), `${file} names a fixed port`);
        assert.doesNotMatch(code(file), /mute-audio|pactl|pacmd|amixer|wpctl|pw-cli|alsamixer|set-sink-volume/, `${file} touches the sound`);
    }
    const session = code(SESSION_JS);
    assert.equal([...session.matchAll(/writeFileSync\(path\.join\(this\.dir, "sound-records\.json"\)/g)].length, 1, "one writer of sound-records.json");
    assert.equal([...session.matchAll(/writeFileSync\(path\.join\(this\.dir, "ui-records\.json"\)/g)].length, 1, "one writer of ui-records.json");
    assert.equal([...session.matchAll(/this\.releaseSounds\(/g)].length, 2, "two callers of the gate: the statement and the end");
    assert.doesNotMatch(session, /\.(?:newPage|newContext|goto|launch|launchPersistentContext)\(/, "the tool opens no page of its own");
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

// A page with a big button in its middle. With `layer`, a full-screen layer that is nearly clear sits over it
// and takes every pointer event: the button never gets the click (the MC forge guide did this).
const gamePage = (layer) => `<!doctype html><meta charset="utf-8"><body style="margin:0;background:#224"><style>
    #b { position: absolute; left: 500px; top: 300px; width: 280px; height: 120px; background: #2a2; }
    #b.on { background: #e22; }
    #l { position: absolute; inset: 0; background: rgba(0,0,0,0.05); }
    </style><div id="b"></div>${layer ? '<div id="l"></div>' : ""}<script>
    window.__log = [];
    const record = (event) => window.__log.push([event.type, event.isTrusted, event.key ?? event.code ?? "", event.deltaY ?? 0, event.buttons ?? 0]);
    for (const type of ["mousedown", "mouseup", "keydown", "keyup", "wheel", "mousemove"]) addEventListener(type, record, true);
    document.getElementById("b").addEventListener("mousedown", () => document.getElementById("b").classList.add("on"));
    window.turts_debug = () => "";
    window.turts_debug_request = () => Promise.resolve(JSON.stringify({ audio: { context_state: "running", sink_observations: [] } }));
</script>`;

test("with a real Chrome: a click that the page takes changes the +200 ms picture; a click that a layer takes does not", { skip: !CHROME && "no Chrome binary" }, async () => {
    const { chromium } = require("playwright-core");
    const server = http.createServer((request, response) => {
        response.setHeader("content-type", "text/html");
        response.end(gamePage(request.url.startsWith("/layer")));
    });
    await new Promise((resolve) => server.listen(0, "127.0.0.1", resolve));
    const port = await freePort();
    const profile = tempDir("chrome");
    const chrome = spawn(CHROME, [`--user-data-dir=${profile}`, `--remote-debugging-port=${port}`, "--no-first-run", "--no-default-browser-check", "--headless=new", "--disable-gpu", "about:blank"], { stdio: "ignore" });
    let browser;
    try {
        await until(async () => (await fetch(`http://127.0.0.1:${port}/json/version`)).json(), "the debugging port");
        browser = await chromium.connectOverCDP(`http://127.0.0.1:${port}`);
        const results = {};
        for (const name of ["open", "layer"]) {
            const page = await browser.contexts()[0].newPage();
            await page.goto(`http://127.0.0.1:${server.address().port}/${name}`);
            await page.setViewportSize({ width: 1280, height: 720 });
            const dir = path.join(tempDir(`real-${name}`), "evidence");
            const session = new ColdSession({ page, dir, viewport: { width: 1280, height: 720 }, limitMs: 60000 });
            await session.start();
            const click = await session.handle({ cmd: "click", args: ["640", "360", "left"] });
            assert.equal(click.exit, 0, click.lines.join("\n"));
            for (const [cmd, args] of [["key", ["F1"]], ["move", ["600", "340"]], ["wheel", ["120"]], ["drag", ["100", "100", "200", "150"]]]) {
                assert.equal((await session.handle({ cmd, args })).exit, 0, cmd);
            }
            const log = await page.evaluate(() => window.__log);
            await session.finish("end");
            results[name] = { dir, log, same: fs.readFileSync(path.join(dir, "shots/0000-start.png")).equals(fs.readFileSync(path.join(dir, "shots/0001-click-p200.png"))) };
            const png = fs.readFileSync(path.join(dir, "shots/0001-click-p200.png"));
            assert.deepEqual([png.readUInt32BE(16), png.readUInt32BE(20)], [1280, 720], "a 1280 x 720 picture: the points of the agent are its pixels");
            await page.close();
        }
        for (const name of ["open", "layer"]) {
            const types = results[name].log.map((entry) => entry[0]);
            assert.ok(results[name].log.every((entry) => entry[1] === true), `${name}: every event is trusted (a real input)`);
            assert.ok(types.includes("mousedown") && types.includes("mouseup") && types.includes("wheel") && types.includes("keydown") && types.includes("keyup"), `${name}: ${types}`);
            assert.ok(results[name].log.some((entry) => entry[0] === "keydown" && entry[2] === "F1"));
            assert.ok(results[name].log.some((entry) => entry[0] === "wheel" && entry[3] === 120));
        }
        assert.equal(results.open.same, false, "the page took the click: the +200 ms picture differs from the start picture");
        assert.equal(results.layer.same, true, "a layer took the click: the +200 ms picture is the start picture (F2, F3)");
        // The checker reads the evidence of the real session too: the log, the files, and the time of each picture.
        const real = check(results.open.dir);
        for (const id of ["E1", "E1b", "E2", "E2b", "L1"]) assert.match(real.out, new RegExp(`PASS ${id} `), real.out);
    } finally {
        try { await browser?.close(); } catch { /* gone */ }
        chrome.kill();
        await new Promise((resolve) => { if (chrome.exitCode !== null) resolve(); else chrome.once("exit", resolve); });
        server.close();
    }
});

test("mp1-play.sh --url starts a Chrome that answers on the printed port, and ends it with the profile on TERM", { skip: !CHROME && "no Chrome binary" }, async () => {
    // A headless Chrome stands in for the headed one: the start logic is the same.
    const folder = tempDir("playreal");
    const shim = path.join(folder, "chrome");
    fs.writeFileSync(shim, `#!/bin/sh\nexec '${CHROME}' --headless=new --disable-gpu "$@"\n`, { mode: 0o755 });
    const server = http.createServer((_request, response) => { response.setHeader("content-type", "text/html"); response.end("<!doctype html><title>game</title>"); });
    await new Promise((resolve) => server.listen(0, "127.0.0.1", resolve));
    const url = `http://127.0.0.1:${server.address().port}/?a=1`;
    const play = spawn("bash", [path.join(REPO, "scripts/mp1-play.sh"), "--url", url], { env: { ...process.env, CHROME_BIN: shim, TMPDIR: folder }, stdio: ["ignore", "pipe", "pipe"] });
    let stdout = "";
    let stderr = "";
    play.stdout.on("data", (chunk) => { stdout += chunk; });
    play.stderr.on("data", (chunk) => { stderr += chunk; });
    try {
        await until(() => /DEBUG_PORT: \d+/.test(stdout) && /PROFILE: /.test(stdout), `the start lines (stderr: ${stderr})`, 45000);
        const lines = Object.fromEntries(stdout.split("\n").filter(Boolean).map((line) => [line.slice(0, line.indexOf(":")), line.slice(line.indexOf(": ") + 2)]));
        assert.equal(lines.URL, url);
        const version = await (await fetch(`http://127.0.0.1:${lines.DEBUG_PORT}/json/version`)).json();
        assert.match(version.Browser, /Chrome/, "the printed port is the debugging port of the Chrome that was started");
        assert.ok(fs.existsSync(lines.PROFILE), "the profile folder exists while Chrome runs");
        assert.ok(lines.PROFILE.startsWith(folder), "a new profile folder, not a default one");
        play.kill("SIGTERM");
        const code = await new Promise((resolve) => play.once("exit", resolve));
        assert.equal(code, 143);
        assert.equal(fs.existsSync(lines.PROFILE), false, "the profile folder is removed");
        await assert.rejects(fetch(`http://127.0.0.1:${lines.DEBUG_PORT}/json/version`), "Chrome is gone");
    } finally {
        // TERM first: the script ends its Chrome. KILL only if it does not end.
        if (play.exitCode === null) {
            play.kill("SIGTERM");
            await new Promise((resolve) => { const timer = setTimeout(() => { play.kill("SIGKILL"); resolve(); }, 10000); play.once("exit", () => { clearTimeout(timer); resolve(); }); });
        }
        server.close();
    }
});
