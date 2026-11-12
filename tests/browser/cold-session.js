// turts-test-role: instrument
"use strict";

// Q-COLD: the cold-play step tool, the session side (docs/qa/cold-play.md; protocol of
// docs/research/mp1/input-ceo-second-play-test.md section 5).
//
// A person plays in a headed Chrome (scripts/mp1-play.sh starts it). An agent that has only a
// URL plays the same page one input at a time through `cold-play.js`. This file is the one process
// that holds the Chrome connection. It does each input with real mouse and keyboard events, takes
// the screenshots, and writes ONE evidence folder:
//
//   session.json      what the session was (limits, viewport, Chrome version)
//   events.jsonl      every event in order, with the time in ms since the session start
//   inputs.jsonl      the inputs only (one line for each call that the agent made)
//   shots/            every screenshot: 0000-start.png, then NNNN-<input>-p200.png and -p1000.png
//   statement.txt     the 60 s statement, stored when the agent calls `statement`
//   sound-records.json the sound records of the page debug state (see the gate below)
//   ui-records.json   the UI record (turts_ui) before the first input and after the first click, with the
//                     measures that the checker needs (words, alert lines, big layers); held back like the sounds
//   probe-*.json      the record of a scripted probe (cold-probe.js); a probe session has no agent
//   end.json          why and when the session ended
//   internal/         the control port and the logs of the start script (not for the agent)
//
// THE STATEMENT GATE (protocol check C3). The sound records come from the page debug state, which
// the agent must not read before it has said what it saw. The sampler keeps them in memory. They
// reach the disk in only two places: `releaseSounds`, called by `storeStatement` AFTER statement.txt
// is written, and by `finish` at the end of the session. `storeStatement` refuses before 60 s after
// the first click, so a junk statement cannot open the gate early. The order is also in events.jsonl
// (`statement_stored` before `sounds_released`) and the checker reads it.
//
// THE TOOL FINDS NOTHING FOR THE AGENT. What the agent reads is built in `reply`: the input it gave,
// the file paths of the screenshots, the clocks, and the refusal texts. No bridge value, no
// binding, no coordinate of a thing and no element lookup is ever in a reply.
const crypto = require("node:crypto");
const fs = require("node:fs");
const net = require("node:net");
const path = require("node:path");
const { nonflatShare } = require("./cold-pixels.js");
const { readWorld } = require("./cold-probe.js");
const { decodePng } = require("./harness/png.js");

const STATEMENT_AFTER_MS = 60000;
const SHOT_OFFSETS_MS = [200, 1000];
const SAMPLE_MS = 500;
// 300 s: the faults of the first cold session came in its first 60 s (docs/qa/cold-play.md).
const DEFAULT_LIMIT_S = 300;
// The headed wrapper holds the GPU lock for 1,800 s at most (mc_headed_v6.sh); the session must end first.
const MAX_LIMIT_S = 1500;
const MAX_WAIT_S = 30;
const PRESS_MS = 60;
const INPUT_KINDS = new Set(["move", "click", "drag", "wheel", "key"]);
const STATEMENT_KEYS = ["GOAL", "GOAL_SHOT", "FIRST_ACTION", "FIRST_ACTION_SHOT", "WORKED", "WORKED_SHOT"];
const SHOT_NAME = /^(?:0000-start|\d{4}-[a-z]+-p(?:200|1000))\.png$/;
const KEY_NAME = /^[A-Za-z0-9]{1,24}$/;
const FINDING_CLASSES = ["no-response", "wrong-response", "stale-text", "hidden-control", "covered-control", "unclear-text", "too-much-text", "sound", "other"];
const REPORT_WORDS = 12;

const USAGE = [
    "inputs (one for each call):",
    "  move X Y                 move the pointer to X Y (pixels of the screenshot)",
    "  click X Y left|right     press and release a mouse button at X Y",
    "  drag X1 Y1 X2 Y2         press the left button at X1 Y1, move to X2 Y2, release",
    "  wheel DY                 turn the wheel by DY at the pointer (positive is down)",
    "  key CODE                 press one key, for example KeyA, Digit1, Escape, F1, Space",
    "  wait SECONDS             do nothing for 0 to 30 s (wait 0 only takes a screenshot)",
    "each call returns the paths of two screenshots, 200 ms and 1000 ms after the input.",
    "other calls:",
    "  status                   the clocks",
    "  statement FILE           store your statement (see below); it opens after 60 s from your first click",
    "  report FILE              store your report (the form below) after the statement",
    "  end                      end the session",
    "statement file, six lines, each KEY: value on one line:",
    "  GOAL: what you think the goal is",
    "  GOAL_SHOT: the screenshot file that shows it",
    "  FIRST_ACTION: the first thing you would do",
    "  FIRST_ACTION_SHOT: the screenshot file that shows it",
    "  WORKED: how you know that the action worked",
    "  WORKED_SHOT: the screenshot file that shows it",
    "At 60 s after your first click or drag you must store the statement. No input is accepted before that.",
    "report file, after the statement (no yes or no line; the form is fixed):",
    "  FINDING 1: CLASS | SHOT | INPUT | EXPECTED | SEEN      one line for each finding, numbered from 1",
    `    CLASS is one of: ${FINDING_CLASSES.join(", ")}`,
    `    SHOT a screenshot file; INPUT the number of the call that caused it, or none; EXPECTED and SEEN ${REPORT_WORDS} words or fewer`,
    "  STUCK: CALL | SHOT | TEXT     the call and the screenshot where you did not know what to do, or: STUCK: none",
].join("\n");

// Runs in the page: the sound onsets of the debug state. The page publishes them; the tool keeps
// only these five fields of each.
async function readSounds() {
    if (typeof window.turts_debug !== "function") return { available: false };
    const raw = await (window.turts_debug_request?.() ?? window.turts_debug());
    const debug = typeof raw === "string" ? JSON.parse(raw) : raw;
    const audio = debug.audio ?? {};
    return {
        available: true,
        context_state: audio.context_state ?? null,
        observations: (audio.sink_observations ?? []).map((o) => ({
            sound_id: o.sound_id ?? null, category: o.category ?? null, play_id: o.play_id ?? null,
            player_entity: o.player_entity ?? null, request_frame_index: o.request_frame_index ?? null,
        })),
    };
}

// The statement text as fields. Each problem is one sentence for the agent. `shotNames` is the set
// of screenshot files of this session.
function parseStatement(text, shotNames) {
    const fields = {};
    const problems = [];
    text.split(/\r?\n/).forEach((line, index) => {
        if (!line.trim()) return;
        const match = /^([A-Z_]+):[ \t]*(.*)$/.exec(line);
        if (!match) { problems.push(`line ${index + 1} is not "KEY: value"`); return; }
        const [, key, value] = match;
        if (!STATEMENT_KEYS.includes(key)) problems.push(`line ${index + 1}: unknown key ${key}`);
        else if (key in fields) problems.push(`line ${index + 1}: ${key} is given two times`);
        else fields[key] = value.trim();
    });
    for (const key of STATEMENT_KEYS) {
        if (!(key in fields)) problems.push(`${key} is missing`);
        else if (!fields[key]) problems.push(`${key} is empty`);
    }
    for (const key of STATEMENT_KEYS.filter((name) => name.endsWith("_SHOT") && fields[name])) {
        const name = path.basename(fields[key]);
        if (!SHOT_NAME.test(name) || !shotNames.has(name)) problems.push(`${key}: ${fields[key]} is not a screenshot file of this session`);
    }
    return { fields, problems };
}

// The report of an agent session in its fixed form. Each problem is one sentence. `calls` is the number of calls made.
function parseReport(text, shotNames, calls) {
    const problems = [];
    const findings = [];
    let stuck = null;
    text.split(/\r?\n/).forEach((line, index) => {
        if (!line.trim()) return;
        const where = `line ${index + 1}`;
        if (/^\s*(yes|no|answer|verdict|ready|not ready)\b/i.test(line)) { problems.push(`${where}: the form has no yes or no line`); return; }
        let match = /^FINDING (\d+): (.*)$/.exec(line);
        if (match) {
            const parts = match[2].split("|").map((part) => part.trim());
            if (Number(match[1]) !== findings.length + 1) problems.push(`${where}: findings are numbered 1, 2, 3 in order`);
            if (parts.length !== 5) { problems.push(`${where}: a finding has five parts: CLASS | SHOT | INPUT | EXPECTED | SEEN`); return; }
            const [cls, shot, input, expected, seen] = parts;
            if (!FINDING_CLASSES.includes(cls)) problems.push(`${where}: CLASS ${cls} is not one of ${FINDING_CLASSES.join(", ")}`);
            if (!SHOT_NAME.test(path.basename(shot)) || !shotNames.has(path.basename(shot))) problems.push(`${where}: SHOT ${shot} is not a screenshot file of this session`);
            if (input !== "none" && !(/^\d+$/.test(input) && Number(input) >= 1 && Number(input) <= calls)) problems.push(`${where}: INPUT is a call number from 1 to ${calls}, or none`);
            for (const [name, value] of [["EXPECTED", expected], ["SEEN", seen]]) {
                if (!value) problems.push(`${where}: ${name} is empty`);
                else if (value.split(/\s+/).length > REPORT_WORDS) problems.push(`${where}: ${name} has more than ${REPORT_WORDS} words`);
            }
            findings.push({ cls, shot, input, expected, seen });
            return;
        }
        match = /^STUCK: (.*)$/.exec(line);
        if (match) {
            if (stuck !== null) problems.push(`${where}: STUCK is given two times`);
            else if (match[1].trim() === "none") stuck = "none";
            else {
                const parts = match[1].split("|").map((part) => part.trim());
                if (parts.length !== 3 || !/^\d+$/.test(parts[0]) || Number(parts[0]) < 1 || Number(parts[0]) > calls || !shotNames.has(path.basename(parts[1])) || !parts[2]) problems.push(`${where}: STUCK is "none" or CALL | SHOT | TEXT (a call number, a screenshot of this session, a text)`);
                stuck = parts;
            }
            return;
        }
        problems.push(`${where} is not FINDING n: ... or STUCK: ...`);
    });
    if (stuck === null) problems.push("STUCK is missing (write STUCK: none when you were not stuck)");
    return { findings, stuck, problems };
}

// The tool commit and the sha256 of each tool file, for session.json (check C5 compares them with the tool under review).
const TOOL_FILES = ["tests/browser/cold-probe.js", "tests/browser/cold-pixels.js", "tests/browser/cold-session.js", "tests/browser/cold-play.js",
    "tests/browser/harness/png.js", "scripts/qa/cold_play_check.py"];
function toolRecord(root = path.resolve(__dirname, "../..")) {
    const git = (...args) => require("node:child_process").spawnSync("git", ["-C", root, ...args], { encoding: "utf8" });
    const files = {};
    for (const name of TOOL_FILES) {
        const file = path.join(root, name);
        if (fs.existsSync(file)) files[name] = crypto.createHash("sha256").update(fs.readFileSync(file)).digest("hex");
    }
    const commit = git("rev-parse", "HEAD");
    const dirty = git("status", "--porcelain", "--", ...TOOL_FILES);
    return { commit: commit.status === 0 ? commit.stdout.trim() : null, dirty: dirty.status === 0 ? dirty.stdout.trim().length > 0 : null, files };
}

function refuse(exit, ...lines) { return { exit, lines }; }

class ColdSession {
    // `page` is a Playwright page (a fake in the unit tests). `now` is a monotonic clock in ms and
    // `sleep` waits for ms: the tests give a fake pair. `viewport` is { width, height } of the page.
    constructor({ page, dir, now = () => performance.now(), sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms)),
        limitMs = DEFAULT_LIMIT_S * 1000, viewport, meta = {}, mode = "agent", control = null }) {
        if (!(limitMs >= 1000 && limitMs <= MAX_LIMIT_S * 1000)) throw new Error(`the session limit must be 1 to ${MAX_LIMIT_S} s`);
        Object.assign(this, { page, dir, now, sleep, limitMs, viewport, meta, mode, control });
        this.uiRecords = [];
        this.uiBeforeDone = false;
        this.firstClickShots = false;
        this.shotsDir = path.join(dir, "shots");
        this.seq = 0;
        this.calls = 0;
        this.firstClickMs = null;
        this.statementMs = null;
        this.reportMs = null;
        this.dueRecorded = false;
        this.pointer = null;
        this.ended = null;
        this.sounds = new Map();
        this.sampling = false;
        this.sampleCount = 0;
        this.sampleFailures = 0;
        this.bridgeMissing = 0;
        this.lastSampleError = null;
        this.contextStates = [];
        this.queue = Promise.resolve();
        this.finishing = null;
        this.releases = 0;
    }

    t() { return Math.round(this.now() - this.t0); }

    record(kind, fields = {}) {
        this.seq += 1;
        const event = { seq: this.seq, t_ms: this.t(), kind, ...fields };
        fs.appendFileSync(path.join(this.dir, "events.jsonl"), JSON.stringify(event) + "\n");
        return event;
    }

    async start() {
        fs.mkdirSync(this.shotsDir, { recursive: true });
        fs.mkdirSync(path.join(this.dir, "internal"), { recursive: true });
        this.t0 = this.now();
        const size = this.viewport ?? await this.page.evaluate(() => ({ width: window.innerWidth, height: window.innerHeight }));
        this.size = size;
        fs.writeFileSync(path.join(this.dir, "session.json"), JSON.stringify({
            session_version: 2, mode: this.mode, control: this.control, started_iso: new Date().toISOString(), limit_ms: this.limitMs, statement_after_ms: STATEMENT_AFTER_MS,
            sample_ms: SAMPLE_MS, shot_offsets_ms: SHOT_OFFSETS_MS, viewport: size, ...this.meta,
        }, null, 2) + "\n");
        this.record("ready");
        await this.shot("0000-start", 0);
        await this.sample();
    }

    // One screenshot in CSS pixels, so the agent's points are the pixels of the picture.
    async shot(name, targetMs) {
        const file = `${name}.png`;
        const full = path.join(this.shotsDir, file);
        const tStart = this.t();
        await this.page.screenshot({ path: full, scale: "css" });
        const data = fs.readFileSync(full);
        this.record("shot", { file, target_ms: targetMs, t_start_ms: tStart, t_done_ms: this.t(),
            sha256: crypto.createHash("sha256").update(data).digest("hex"), bytes: data.length });
        return full;
    }

    // The screenshots 200 ms and 1000 ms after `endMs`.
    async shotsAfter(n, kind, endMs, hook = null, firstClick = false) {
        const paths = [];
        for (const offset of SHOT_OFFSETS_MS) {
            // A timer can wake a millisecond early: wait again until the time is reached.
            for (let wait = endMs + offset - this.t(); wait > 0; wait = endMs + offset - this.t()) await this.sleep(wait);
            const file = await this.shot(`${String(n).padStart(4, "0")}-${kind}-p${offset}`, endMs + offset);
            paths.push(file);
            if (firstClick) await this.captureUi(`after_first_click_p${offset}`, file);
            if (hook) await hook(offset, file);
        }
        return paths;
    }

    // The three bridges as one record (cold-probe.js). Used by the probes and the UI records; never sent to an agent.
    readWorld() { return this.page.evaluate(readWorld); }

    // One UI record: the widgets, the alert lines, and for each layer of more than half of the page whether the
    // world shows through it. `file` is a screenshot that was just taken; with none, a picture is taken for the measure.
    async captureUi(label, file = null) {
        const world = await this.readWorld().catch(() => ({ available: false }));
        if (!world.available) { this.uiRecords.push({ label, t_ms: this.t(), available: false }); return; }
        const size = this.size;
        const big = world.ui.filter((widget) => widget.visible && widget.rect && (widget.rect[2] * widget.rect[3]) / (size.width * size.height) > 0.5);
        let picture = null;
        if (big.length) {
            try { picture = decodePng(file ? fs.readFileSync(file) : await this.page.screenshot({ scale: "css" })); } catch { picture = null; }
        }
        this.uiRecords.push({
            label, t_ms: this.t(), available: true, viewport: size, alerts: world.alerts,
            alert_lines: world.ui.filter((widget) => widget.visible && /(^|\/)alert_line/.test(widget.id) && widget.kind !== "widget" && (widget.text ?? "").trim()).length,
            widgets: world.ui.filter((widget) => widget.visible).map((widget) => ({ id: widget.id, kind: widget.kind, rect: widget.rect, text: widget.text,
                capture_root: widget.capture_root, focus_policy_block: widget.focus_policy_block })),
            big_widgets: big.map((widget) => ({ id: widget.id, kind: widget.kind, rect: widget.rect, area_share: Math.round((widget.rect[2] * widget.rect[3]) / (size.width * size.height) * 1000) / 1000,
                capture_root: widget.capture_root, focus_policy_block: widget.focus_policy_block, nonflat_share: picture ? nonflatShare(picture, widget.rect) : null })),
        });
    }

    // One read of the sound onsets. A failed read is counted, not thrown.
    async sample() {
        if (this.sampling || this.ended) return;
        this.sampling = true;
        const at = this.t();
        try {
            let timer;
            const result = await Promise.race([this.page.evaluate(readSounds),
                new Promise((_, reject) => { timer = setTimeout(() => reject(new Error("no debug record within 5 s")), 5000); })])
                .finally(() => clearTimeout(timer));
            this.sampleCount += 1;
            if (!result.available) { this.bridgeMissing += 1; return; }
            const state = result.context_state ?? null;
            if (!this.contextStates.length || this.contextStates[this.contextStates.length - 1].state !== state) this.contextStates.push({ t_ms: at, state });
            for (const o of result.observations) {
                const key = `${o.play_id}:${o.player_entity}:${o.request_frame_index}:${o.sound_id}`;
                if (!this.sounds.has(key)) this.sounds.set(key, { ...o, first_seen_ms: at });
            }
        } catch (error) {
            this.sampleFailures += 1;
            this.lastSampleError = String(error?.message || error).slice(0, 200);
        } finally { this.sampling = false; }
    }

    // THE GATE: the only writer of sound-records.json. Callers: storeStatement, finish.
    releaseSounds(reason, final) {
        const records = [...this.sounds.values()];
        fs.writeFileSync(path.join(this.dir, "sound-records.json"), JSON.stringify({
            released_at_ms: this.t(), reason, final,
            sampling: { interval_ms: SAMPLE_MS, samples: this.sampleCount, failures: this.sampleFailures, bridge_missing: this.bridgeMissing,
                last_error: this.lastSampleError },
            context_states: this.contextStates, records,
        }, null, 2) + "\n");
        fs.writeFileSync(path.join(this.dir, "ui-records.json"), JSON.stringify({ records: this.uiRecords }, null, 2) + "\n");
        this.releases += 1;
        this.record("sounds_released", { reason, final, records: records.length });
    }

    // The clock lines that every reply carries.
    clockLines() {
        const lines = [];
        const now = this.t();
        if (this.firstClickMs === null) lines.push("clock: no click yet. The 60 s clock starts at your first click or drag.");
        else if (this.statementMs !== null) lines.push("clock: your statement is stored.");
        else if (this.isDue()) {
            lines.push(`STATEMENT DUE: ${Math.round((now - this.firstClickMs) / 1000)} s have passed since your first click. Input is refused until you store your statement.`,
                "Write a file with six lines (GOAL, GOAL_SHOT, FIRST_ACTION, FIRST_ACTION_SHOT, WORKED, WORKED_SHOT; each KEY: value) and call: statement FILE");
        } else lines.push(`clock: ${((now - this.firstClickMs) / 1000).toFixed(1)} s since your first click. Your statement is due at 60 s.`);
        lines.push(`session time left: ${Math.max(0, Math.round((this.limitMs - now) / 1000))} s`);
        return lines;
    }

    isDue() { return this.mode === "agent" && this.firstClickMs !== null && this.statementMs === null && this.t() >= this.firstClickMs + STATEMENT_AFTER_MS; }

    reply(exit, lines) {
        if (this.isDue() && !this.dueRecorded) {
            this.dueRecorded = true;
            this.record("statement_due", { due_ms: this.firstClickMs + STATEMENT_AFTER_MS });
        }
        return { exit, lines: [...lines, ...this.clockLines()] };
    }

    handle(request) {
        const run = this.queue.then(() => this.dispatch(request));
        this.queue = run.catch(() => {});
        return run;
    }

    async dispatch({ cmd, args = [], text, hook = null }) {
        if (this.ended) return refuse(4, `the session has ended (${this.ended}). The evidence folder is complete.`);
        try {
            if (cmd === "status") return this.reply(0, ["status"]);
            if (cmd === "end") { await this.finish("end"); return refuse(0, "the session is over. The evidence folder is complete."); }
            if (cmd === "statement") return await this.storeStatement(text ?? "");
            if (cmd === "report") return await this.storeReport(text ?? "");
            if (cmd === "wait") return await this.wait(args);
            if (!INPUT_KINDS.has(cmd)) return refuse(2, `unknown call "${cmd}"`, USAGE);
            return await this.input(cmd, args, hook);
        } catch (error) {
            this.record("error", { cmd, message: String(error?.message || error).slice(0, 300) });
            return this.reply(6, [`the tool could not do "${cmd}": ${String(error?.message || error).split("\n")[0].slice(0, 200)}`]);
        }
    }

    number(text) { return /^-?\d+(?:\.\d+)?$/.test(String(text)) ? Number(text) : null; }

    point(x, y) {
        if (x === null || y === null) return "X and Y must be numbers";
        if (x < 0 || y < 0 || x >= this.size.width || y >= this.size.height) return `the point ${x} ${y} is outside the page, which is ${this.size.width} x ${this.size.height} pixels`;
        return null;
    }

    async input(cmd, args, hook = null) {
        if (this.isDue()) {
            this.record("refused", { cmd, reason: "statement_due" });
            return this.reply(3, [`refused: "${cmd}" is an input, and your statement is due.`]);
        }
        const { page, sleep } = this;
        // The UI record just before the first input (a wait is no input): alert lines before any input count here.
        if (!this.uiBeforeDone && cmd !== "wait") { this.uiBeforeDone = true; await this.captureUi("before_first_input"); }
        const nums = args.map((arg) => this.number(arg));
        const startMs = this.t();
        const detail = {};
        let downMs = null;
        const press = async (button) => {
            await sleep(40);
            downMs = this.t();
            await page.mouse.down({ button });
            if (this.firstClickMs === null) { this.firstClickMs = downMs; this.record("first_click", { t_down_ms: downMs }); }
            await sleep(PRESS_MS);
        };
        if (cmd === "move") {
            if (args.length !== 2) return refuse(2, "usage: move X Y");
            const bad = this.point(nums[0], nums[1]);
            if (bad) return refuse(2, bad);
            await page.mouse.move(nums[0], nums[1]);
            this.pointer = { x: nums[0], y: nums[1] };
            Object.assign(detail, { x: nums[0], y: nums[1] });
        } else if (cmd === "click") {
            if (args.length !== 3 || !["left", "right"].includes(args[2])) return refuse(2, "usage: click X Y left|right");
            const bad = this.point(nums[0], nums[1]);
            if (bad) return refuse(2, bad);
            await page.mouse.move(nums[0], nums[1]);
            this.pointer = { x: nums[0], y: nums[1] };
            await press(args[2]);
            await page.mouse.up({ button: args[2] });
            Object.assign(detail, { x: nums[0], y: nums[1], button: args[2] });
        } else if (cmd === "drag") {
            if (args.length !== 4) return refuse(2, "usage: drag X1 Y1 X2 Y2");
            const bad = this.point(nums[0], nums[1]) ?? this.point(nums[2], nums[3]);
            if (bad) return refuse(2, bad);
            await page.mouse.move(nums[0], nums[1]);
            await press("left");
            const steps = 10;
            for (let i = 1; i <= steps; i += 1) {
                await page.mouse.move(nums[0] + ((nums[2] - nums[0]) * i) / steps, nums[1] + ((nums[3] - nums[1]) * i) / steps);
                await sleep(30);
            }
            await page.mouse.up({ button: "left" });
            this.pointer = { x: nums[2], y: nums[3] };
            Object.assign(detail, { x1: nums[0], y1: nums[1], x2: nums[2], y2: nums[3] });
        } else if (cmd === "wheel") {
            if (args.length !== 1 || nums[0] === null || nums[0] === 0 || Math.abs(nums[0]) > 2000) return refuse(2, "usage: wheel DY (a number, not 0, at most 2000 either way)");
            if (!this.pointer) return refuse(2, "move the pointer first: the wheel turns where the pointer is");
            await page.mouse.wheel(0, nums[0]);
            Object.assign(detail, { dy: nums[0] });
        } else if (cmd === "key") {
            if (args.length !== 1 || !KEY_NAME.test(args[0])) return refuse(2, "usage: key CODE (one key name, for example KeyA, Digit1, Escape, F1)");
            try { await page.keyboard.down(args[0]); } catch (error) { return refuse(2, `unknown key "${args[0]}"`); }
            await sleep(PRESS_MS);
            await page.keyboard.up(args[0]);
            Object.assign(detail, { code: args[0] });
        }
        const endMs = this.t();
        this.calls += 1;
        const n = this.calls;
        const event = this.record("input", { n, cmd, args: detail, t_start_ms: startMs, t_down_ms: downMs, t_end_ms: endMs, counts_as_input: true });
        fs.appendFileSync(path.join(this.dir, "inputs.jsonl"), JSON.stringify(event) + "\n");
        const firstClick = !this.firstClickShots && downMs !== null && this.firstClickMs === downMs;
        if (firstClick) this.firstClickShots = true;
        const shots = await this.shotsAfter(n, cmd, endMs, hook, firstClick);
        return this.reply(0, [`input ${n}: ${cmd} ${args.join(" ")}`, `screenshot +200 ms: ${shots[0]}`, `screenshot +1000 ms: ${shots[1]}`]);
    }

    // `wait` is not an input of the player: it is time with no input. It takes the same two screenshots.
    async wait(args) {
        const seconds = this.number(args[0]);
        if (args.length !== 1 || seconds === null || seconds < 0 || seconds > MAX_WAIT_S) return refuse(2, `usage: wait SECONDS (0 to ${MAX_WAIT_S})`);
        const startMs = this.t();
        const left = this.limitMs - startMs;
        const wanted = Math.min(seconds * 1000, Math.max(0, left));
        if (wanted > 0) await this.sleep(wanted);
        const endMs = this.t();
        this.calls += 1;
        const n = this.calls;
        const event = this.record("input", { n, cmd: "wait", args: { seconds }, t_start_ms: startMs, t_down_ms: null, t_end_ms: endMs, counts_as_input: false });
        fs.appendFileSync(path.join(this.dir, "inputs.jsonl"), JSON.stringify(event) + "\n");
        const shots = await this.shotsAfter(n, "wait", endMs);
        return this.reply(0, [`input ${n}: wait ${seconds}${wanted < seconds * 1000 ? " (cut short by the session limit)" : ""}`, `screenshot +200 ms: ${shots[0]}`, `screenshot +1000 ms: ${shots[1]}`]);
    }

    async storeStatement(text) {
        if (this.statementMs !== null) return this.reply(3, ["refused: the statement is stored already. It is not changed."]);
        if (this.firstClickMs === null) return this.reply(3, ["refused: you have made no click or drag yet. The statement is for 60 s after your first click."]);
        const left = this.firstClickMs + STATEMENT_AFTER_MS - this.t();
        if (left > 0) return this.reply(3, [`refused: too early. The statement opens ${Math.ceil(left / 1000)} s from now (60 s after your first click).`]);
        const shotNames = new Set(fs.readdirSync(this.shotsDir));
        const { problems } = parseStatement(text, shotNames);
        if (problems.length) {
            this.record("statement_refused", { problems: problems.length });
            return this.reply(3, ["refused: the statement is not stored.", ...problems.map((problem) => `  - ${problem}`)]);
        }
        await this.sample();
        fs.writeFileSync(path.join(this.dir, "statement.txt"), text, { flag: "wx" });
        this.statementMs = this.t();
        this.record("statement_stored", { sha256: crypto.createHash("sha256").update(text).digest("hex"), bytes: Buffer.byteLength(text) });
        this.releaseSounds("statement", false);
        return this.reply(0, ["statement stored.", `sound records: ${path.join(this.dir, "sound-records.json")} (they are released now; they grow until the session ends)`]);
    }

    // The report: the fixed form, after the statement, one time. It is read by the reviewer; the answer of the agent is not.
    async storeReport(text) {
        if (this.mode !== "agent") return this.reply(3, ["refused: a probe session has no report."]);
        if (this.statementMs === null) return this.reply(3, ["refused: store the statement first."]);
        if (this.reportMs !== null) return this.reply(3, ["refused: the report is stored already. It is not changed."]);
        const { problems } = parseReport(text, new Set(fs.readdirSync(this.shotsDir)), this.calls);
        if (problems.length) {
            this.record("report_refused", { problems: problems.length });
            return this.reply(3, ["refused: the report is not stored.", ...problems.map((problem) => `  - ${problem}`)]);
        }
        fs.writeFileSync(path.join(this.dir, "report.txt"), text, { flag: "wx" });
        this.reportMs = this.t();
        this.record("report_stored", { sha256: crypto.createHash("sha256").update(text).digest("hex"), bytes: Buffer.byteLength(text) });
        return this.reply(0, ["report stored."]);
    }

    // End the session: one more read of the sounds, the release if the gate has not opened, end.json.
    finish(reason) {
        if (!this.finishing) {
            this.finishing = (async () => {
                await this.sample();
                this.ended = reason;
                this.releaseSounds(this.mode !== "agent" ? "probe" : this.statementMs === null ? "end-without-statement" : "end", true);
                const end = { reason, t_ms: this.t(), limit_ms: this.limitMs, calls: this.calls, first_click_ms: this.firstClickMs, statement_stored: this.statementMs !== null };
                fs.writeFileSync(path.join(this.dir, "end.json"), JSON.stringify(end, null, 2) + "\n");
                this.record("end", { reason });
                fs.rmSync(path.join(this.dir, "internal", "control.json"), { force: true });
            })();
        }
        return this.finishing;
    }
}

// The control port: one JSON line in, one JSON line out, loopback only. `onEnd` runs after the
// reply to the call that ended the session.
function listen(session, onEnd) {
    const server = net.createServer((socket) => {
        let buffer = "";
        socket.on("error", () => {});
        socket.on("data", (chunk) => {
            buffer += chunk;
            const newline = buffer.indexOf("\n");
            if (newline < 0) return;
            const line = buffer.slice(0, newline);
            buffer = "";
            let request;
            try { request = JSON.parse(line); } catch { socket.end(JSON.stringify({ exit: 2, lines: ["bad request"] }) + "\n"); return; }
            session.handle(request).then((reply) => {
                socket.end(JSON.stringify(reply) + "\n", () => { if (session.ended) onEnd(); });
            }, (error) => socket.end(JSON.stringify({ exit: 6, lines: [String(error?.message || error)] }) + "\n"));
        });
    });
    return new Promise((resolve, reject) => server.once("error", reject).listen(0, "127.0.0.1", () => resolve(server)));
}

function parseArgs(argv) {
    const options = { port: null, session: null, limitSecs: DEFAULT_LIMIT_S, viewport: { width: 1280, height: 720 }, build: null, url: null, probe: null, control: null };
    for (let i = 0; i < argv.length; i += 2) {
        const [flag, value] = [argv[i], argv[i + 1]];
        if (value === undefined) throw new Error(`${flag} needs a value`);
        if (flag === "--port") options.port = Number(value);
        else if (flag === "--session") options.session = value;
        else if (flag === "--limit-secs") options.limitSecs = Number(value);
        else if (flag === "--build") options.build = value;
        else if (flag === "--url") options.url = value;
        else if (flag === "--probe") options.probe = value;
        else if (flag === "--control") options.control = value;
        else if (flag === "--viewport") {
            if (value === "none") options.viewport = null;
            else {
                const match = /^(\d{3,5})x(\d{3,5})$/.exec(value);
                if (!match) throw new Error("--viewport takes WIDTHxHEIGHT or none");
                options.viewport = { width: Number(match[1]), height: Number(match[2]) };
            }
        } else throw new Error(`unknown option ${flag}`);
    }
    if (!(Number.isInteger(options.port) && options.port >= 1 && options.port <= 65535)) throw new Error("--port <1-65535> is required (the DEBUG_PORT of scripts/mp1-play.sh)");
    if (!options.session) throw new Error("--session <folder> is required");
    if (!(Number.isInteger(options.limitSecs) && options.limitSecs >= 1 && options.limitSecs <= MAX_LIMIT_S)) throw new Error(`--limit-secs must be a whole number from 1 to ${MAX_LIMIT_S}`);
    if (options.probe !== null && !["first-click", "buttons"].includes(options.probe)) throw new Error("--probe takes first-click or buttons");
    if (options.control !== null && !(options.probe === "first-click" && options.control === "close-layer")) throw new Error("--control close-layer goes with --probe first-click");
    return options;
}

async function main(argv) {
    let options;
    try { options = parseArgs(argv); } catch (error) { console.error(`cold-session: ${error.message}`); return 2; }
    const { chromium } = require("playwright-core");
    const browser = await chromium.connectOverCDP(`http://127.0.0.1:${options.port}`);
    const pages = browser.contexts().flatMap((context) => context.pages()).filter((page) => /^https?:/.test(page.url()));
    if (pages.length !== 1) { console.error(`cold-session: expected one game page, found ${pages.length}`); await browser.close(); return 1; }
    const page = pages[0];
    await page.bringToFront();
    const session = new ColdSession({
        page, dir: options.session, limitMs: options.limitSecs * 1000, viewport: options.viewport,
        mode: options.probe ? `probe-${options.probe}` : "agent", control: options.control,
        meta: { url: options.url ?? page.url(), build: options.build, tool: toolRecord(), chrome: browser.version(), lock_marker: process.env.MC_LOCK_RUN ?? null, node: process.version },
    });
    if (options.viewport) await page.setViewportSize(options.viewport);
    await session.start();
    if (options.probe) {
        // A probe session: no agent, no control port. The probe runs, the session ends, the folder is complete.
        const { runProbe } = require("./cold-probe.js");
        const sampler = setInterval(() => { session.sample(); }, SAMPLE_MS);
        const limit = setTimeout(() => { session.finish("limit"); }, Math.max(0, options.limitSecs * 1000 - session.t()));
        let record;
        try { record = await runProbe(session, options.probe, { control: options.control }); } finally { clearInterval(sampler); clearTimeout(limit); }
        await session.finish(session.ended ?? "probe-done");
        console.log(`PROBE ${options.probe}${options.control ? ` (control ${options.control})` : ""}: ${record.summary.verdict} - ${record.summary.reason}`);
        try { await browser.close(); } catch { /* Chrome is gone */ }
        return 0;
    }
    let finished;
    const done = new Promise((resolve) => { finished = resolve; });
    const server = await listen(session, finished);
    fs.writeFileSync(path.join(options.session, "internal", "control.json"), JSON.stringify({ port: server.address().port, pid: process.pid }));
    const sampler = setInterval(() => { session.sample(); }, SAMPLE_MS);
    const limit = setTimeout(() => { session.handle({ cmd: "end" }).then(finished); }, Math.max(0, options.limitSecs * 1000 - session.t()));
    page.on("close", () => { session.finish("page-closed").then(finished); });
    console.log(`READY ${options.session}`);
    await done;
    clearInterval(sampler);
    clearTimeout(limit);
    // The reply of the last call goes out before the server closes.
    await new Promise((resolve) => setTimeout(resolve, 200));
    server.close();
    try { await browser.close(); } catch { /* Chrome is gone */ }
    return 0;
}

if (require.main === module) {
    main(process.argv.slice(2)).then((code) => { process.exit(code); },
        (error) => { console.error(`cold-session: ${error.stack || error}`); process.exit(1); });
}

module.exports = { ColdSession, listen, parseArgs, parseStatement, parseReport, toolRecord, FINDING_CLASSES, readSounds, USAGE, STATEMENT_KEYS, SHOT_OFFSETS_MS, STATEMENT_AFTER_MS, MAX_LIMIT_S, DEFAULT_LIMIT_S };
