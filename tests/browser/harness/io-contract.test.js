// turts-test-role: unit
"use strict";

const assert = require("node:assert/strict");
const crypto = require("node:crypto");
const fs = require("node:fs/promises");
const os = require("node:os");
const path = require("node:path");
const test = require("node:test");
const { createDriverIo } = require("./driver-io.js");
const EXPECTED_IO_KEYS = ["click", "drag", "key", "move", "wheel", "screenshot", "save", "wait", "reload", "use"];
const { createView } = require("./view.js");

function fakePage(name, trace) {
    return {
        name,
        evaluate: async (fn, value) => {
            const source = String(fn);
            if (source.includes("__turts_io_seq")) { trace.push([name, "seq", typeof value === "object" ? value.value : value]); return undefined; }
            if (source.includes("__turts_io_actual_events") && source.includes("__turts_io_downstream_events")) return { actual: 0, downstream: 0 };
            if (source.includes("__turts_io_actual_events")) return value === undefined ? 0 : [];
            if (source.includes("__turts_io_downstream_events")) return value === undefined ? 0 : [];
            return undefined;
        },
        mouse: {
            move: async (...args) => trace.push([name, "move", ...args]),
            down: async (...args) => trace.push([name, "down", ...args]),
            up: async (...args) => trace.push([name, "up", ...args]),
            wheel: async (...args) => trace.push([name, "wheel", ...args]),
        },
        keyboard: {
            down: async (...args) => trace.push([name, "keyboard-down", ...args]),
            up: async (...args) => trace.push([name, "keyboard-up", ...args]),
        },
        context: () => ({ newCDPSession: async () => ({
            send: async (...args) => trace.push([name, "cdp-send", ...args]),
            detach: async () => trace.push([name, "cdp-detach"]),
        }) }),
        screenshot: async () => Buffer.from("png"),
        waitForTimeout: async (ms) => trace.push([name, "wait", ms]),
        reload: async () => ({ frame: () => ({ page: () => ({ secret: "raw-page" }) }) }),
        bringToFront: async () => trace.push([name, "front"]),
    };
}

test("io is frozen with exactly the ten approved functions and exposes no browser handle", () => {
    const { io } = createDriverIo([fakePage("a", [])]);
    assert.equal(Object.isFrozen(io), true);
    assert.deepEqual(Object.keys(io).sort(), [...EXPECTED_IO_KEYS].sort());
    for (const key of Object.keys(io)) {
        assert.equal(typeof io[key], "function");
        for (const forbidden of ["page", "evaluate", "context", "browser", "__turts_io_seq"]) {
            assert.equal(forbidden in io[key], false);
            assert.equal(Object.prototype.hasOwnProperty.call(io[key], forbidden), false);
        }
    }
    assert.equal("__turts_io_seq" in io, false);
});

test("interior cursor priming does not suppress an explicit edge move", async () => {
    const trace = [];
    const { io, inputLog } = createDriverIo([fakePage("a", trace)], { initialCursor: { x: 640, y: 360 } });
    await io.move(0, 0);
    assert.deepEqual(inputLog[0].event_seq, [1]);
    assert.deepEqual(inputLog[0].args, [0, 0]);
    assert.deepEqual(trace.find((row) => row[1] === "move").slice(2), [0, 0]);
});

test("DOM event sequences start at one in each window and have no gaps", async () => {
    const trace = [];
    const { io, inputLog } = createDriverIo([fakePage("a", trace), fakePage("b", trace)]);
    await io.click(10, 20);
    await io.drag(1, 2, 3, 4);
    await io.move(8, 9);
    await io.use(2);
    await io.wheel(120);
    assert.deepEqual(inputLog.map((entry) => entry.event_seq), [[1, 2, 3], [4, 5, ...Array.from({ length: 8 }, (_, index) => index + 6), 14], [15], [], [1]]);
    assert.deepEqual(inputLog.map((entry) => entry.call), ["click", "drag", "move", "use", "wheel"]);
    assert.deepEqual(inputLog.map((entry) => entry.args), [[10, 20, "left"], [1, 2, 3, 4, "left"], [8, 9], [2], [120]]);
    for (const entry of inputLog) assert.equal(Number.isFinite(entry.wall_time_ms), true);
    const sequenceValues = trace.filter((row) => row[1] === "seq" && row[2] > 0).map((row) => [row[0], row[2]]);
    assert.deepEqual(sequenceValues, Array.from({ length: 15 }, (_, index) => ["a", index + 1]).concat([["b", 1]]));
});

test("opts.key is sent only through CDP Input.dispatchKeyEvent", async () => {
    const trace = [];
    const { io, inputLog } = createDriverIo([fakePage("a", trace)]);
    await io.key("KeyA", ["Shift"], { key: "q" });
    const cdp = trace.filter((row) => row[1] === "cdp-send");
    assert.deepEqual(cdp.map((row) => row.slice(2)), [
        ["Input.dispatchKeyEvent", { type: "keyDown", code: "ShiftLeft", key: "Shift", modifiers: 8 }],
        ["Input.dispatchKeyEvent", { type: "keyDown", code: "KeyA", key: "q", modifiers: 8 }],
        ["Input.dispatchKeyEvent", { type: "keyUp", code: "KeyA", key: "q", modifiers: 8 }],
        ["Input.dispatchKeyEvent", { type: "keyUp", code: "ShiftLeft", key: "Shift", modifiers: 0 }],
    ]);
    assert.deepEqual(inputLog[0].event_seq, [1, 2, 3, 4]);
    assert.deepEqual(inputLog[0].event_types, ["keydown", "keydown", "keyup", "keyup"]);
    assert.equal(trace.some((row) => row[1] === "keyboard-down" || row[1] === "keyboard-up"), false);
});

test("key args are snapshotted so driver mutation cannot rewrite the input log", async () => {
    const trace = [];
    const { io, inputLog } = createDriverIo([fakePage("a", trace)]);
    const modifiers = ["Shift"];
    const opts = { key: "q" };
    await io.key("KeyA", modifiers, opts);
    modifiers[0] = "Control";
    opts.key = "x";
    assert.deepEqual(inputLog[0].args, ["KeyA", ["Shift"], { key: "q" }]);
    const dispatched = trace.filter((row) => row[1] === "cdp-send").map((row) => row[3].key);
    assert.deepEqual(dispatched, ["Shift", "q", "q", "Shift"]);
});

test("modifier key events are sequenced, and use(n) foregrounds the selected window", async () => {
    const trace = [];
    const { io, inputLog } = createDriverIo([fakePage("a", trace), fakePage("b", trace)]);
    await io.key("KeyA", ["Shift"], { hold_ms: 1000 });
    await io.use(2);
    assert.deepEqual(inputLog.map((entry) => entry.event_seq), [[1, 2, 3, 4], []]);
    assert.deepEqual(inputLog[0].event_types, ["keydown", "keydown", "keyup", "keyup"]);
    assert.equal(trace.some((row) => row[0] === "b" && row[1] === "front"), true);
    const held = trace.map((row) => row[1]);
    assert.ok(held.indexOf("wait", held.lastIndexOf("keyboard-down")) < held.lastIndexOf("keyboard-up"));
});

test("key down/up spans separate calls and every emitted key event is sequenced", async () => {
    const trace = [];
    const { io, inputLog } = createDriverIo([fakePage("a", trace)]);
    await io.key("Shift", [], { action: "down" });
    await io.click(5, 6);
    await io.key("Shift", [], { action: "up" });
    assert.deepEqual(inputLog.map((entry) => entry.event_seq), [[1], [2, 3, 4], [5]]);
    assert.deepEqual(inputLog[0].event_types, ["keydown"]);
    assert.deepEqual(inputLog[2].event_types, ["keyup"]);
    assert.ok(trace.indexOf(trace.find((row) => row[1] === "keyboard-down")) < trace.findIndex((row) => row[1] === "move"));
    assert.ok(trace.findIndex((row) => row[1] === "up") < trace.findIndex((row) => row[1] === "keyboard-up"));
});

test("CDP logical-key overrides preserve held modifiers per window", async () => {
    const trace = [];
    const { io } = createDriverIo([fakePage("a", trace), fakePage("b", trace)]);
    await io.key("Shift", [], { action: "down" });
    await io.key("KeyA", [], { key: "q" });
    await io.use(2);
    await io.key("KeyA", [], { key: "q" });
    await io.key("Ctrl", [], { action: "down", key: "Control" });
    await io.key("KeyA", ["ShiftLeft"], { key: "q" });
    await io.key("KeyA", [], { action: "down", key: "q" });
    await io.key("KeyA", [], { action: "up", key: "q" });
    await io.key("ControlLeft", [], { action: "up", key: "Control" });

    const dispatches = trace.filter((row) => row[1] === "cdp-send").map((row) => [row[0], row[3]]);
    assert.deepEqual(dispatches.map(([window, event]) => [window, event.type, event.code, event.key, event.modifiers]), [
        ["a", "keyDown", "KeyA", "q", 8], ["a", "keyUp", "KeyA", "q", 8],
        ["b", "keyDown", "KeyA", "q", 0], ["b", "keyUp", "KeyA", "q", 0],
        ["b", "keyDown", "ControlLeft", "Control", 2],
        ["b", "keyDown", "ShiftLeft", "Shift", 10],
        ["b", "keyDown", "KeyA", "q", 10], ["b", "keyUp", "KeyA", "q", 10],
        ["b", "keyUp", "ShiftLeft", "Shift", 2],
        ["b", "keyDown", "KeyA", "q", 2], ["b", "keyUp", "KeyA", "q", 2],
        ["b", "keyUp", "ControlLeft", "Control", 0],
    ]);
});

test("view has only four read-only methods and refuses the order bridge", async () => {
    const makePage = (windowNumber) => ({
        evaluate: async (_fn, name) => name === "turts_memory_bytes" ? 512 + windowNumber : JSON.stringify({ bridge: name, window: windowNumber }),
        screenshot: async () => Buffer.from(`png-${windowNumber}`),
    });
    const view = createView([makePage(1), makePage(2)], [], "/tmp");
    assert.equal(Object.isFrozen(view), true);
    assert.deepEqual(Object.keys(view).sort(), ["bridge", "inputLog", "screenshot", "serverFile"]);
    assert.deepEqual(await view.bridge("turts_debug"), { bridge: "turts_debug", window: 1 });
    assert.deepEqual(await view.bridge("turts_scene", 2), { bridge: "turts_scene", window: 2 });
    assert.equal((await view.screenshot(2)).toString(), "png-2");
    assert.equal(await view.bridge("turts_memory_bytes", 2), 514);
    await assert.rejects(view.bridge("turts_move"), /refuses/);
    await assert.rejects(view.bridge("turts_unlisted"), /unknown read-only bridge/);
    await assert.rejects(view.bridge("turts_scene", 3), /window must be in/);
    await assert.rejects(view.serverFile("../outside"), /escapes output directory/);
});


test("failed calls retain their original arguments and error metadata", async () => {
    const trace = [];
    const page = fakePage("a", trace);
    page.mouse.down = async () => { throw new Error("pointer dispatch failed"); };
    const { io, inputLog } = createDriverIo([page]);
    await assert.rejects(io.click(12, 14), /pointer dispatch failed/);
    assert.deepEqual(inputLog[0].args, [12, 14, "left"]);
    assert.deepEqual(inputLog[0].event_seq, [1, 2]);
    assert.match(inputLog[0].error, /pointer dispatch failed/);
});

test("reload discards the Playwright response and cannot leak its Page handle", async () => {
    const { io } = createDriverIo([fakePage("a", [])]);
    assert.equal(await io.reload(), undefined);
});

test("save logs compact argument identity that hashes to the saved artifact", async () => {
    const trace = [];
    const outputDir = await fs.mkdtemp(path.join(os.tmpdir(), "turts-io-save-"));
    try {
        const { io, inputLog } = createDriverIo([fakePage("a", trace)], { outputDir });
        const bytes = Buffer.from([0, 1, 2, 255]);
        await io.save("artifact.bin", bytes);
        const saved = await fs.readFile(path.join(outputDir, "artifact.bin"));
        const expectedHash = crypto.createHash("sha256").update(saved).digest("hex");
        assert.deepEqual(inputLog[0].args[1], { byte_length: bytes.length, sha256: expectedHash, saved_as: "artifact.bin" });
    } finally { await fs.rm(outputDir, { recursive: true, force: true }); }
});
