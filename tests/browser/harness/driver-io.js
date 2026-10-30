// turts-test-role: harness
"use strict";

const fs = require("node:fs/promises");
const path = require("node:path");
const crypto = require("node:crypto");

const IO_KEYS = Object.freeze(["click", "drag", "key", "move", "wheel", "screenshot", "save", "wait", "reload", "use"]);

function wallTime() { return Date.now(); }

function createDriverIo(pages, { outputDir, inputLog = [], initialCursor = { x: 0, y: 0 } } = {}) {
    if (!Array.isArray(pages) || pages.length === 0) throw new TypeError("at least one page is required");
    if (!Number.isFinite(initialCursor.x) || !Number.isFinite(initialCursor.y)) throw new TypeError("initialCursor must have finite x and y");
    const state = { selected: 0 };
    const nextSeq = pages.map(() => 0);
    const cursor = pages.map(() => ({ x: initialCursor.x, y: initialCursor.y }));
    const page = () => pages[state.selected];
    async function record(call, args, events, action) {
        const started = wallTime();
        const entry = { call, args: structuredClone(args), wall_time_ms: started, window: state.selected + 1, event_types: [], event_seq: [], actual_events: [] };
        inputLog.push(entry);
        const traceStart = await page().evaluate(() => ({ actual: window.__turts_io_actual_events?.length || 0, downstream: window.__turts_io_downstream_events?.length || 0 }));
        const send = async (kind, operation, expected = {}) => {
            if (!events.includes(kind)) throw new TypeError(`${call} cannot send ${kind}`);
            const seq = ++nextSeq[state.selected];
            await page().evaluate(({ value, type, match }) => { window.__turts_io_seq = value; window.__turts_io_dispatching = true; window.__turts_io_expected_event = { seq: value, type, ...match, matched: false }; }, { value: seq, type: kind, match: expected });
            entry.event_seq.push(seq);
            entry.event_types.push(kind);
            try { return await operation(); }
            finally { await page().evaluate(() => { window.__turts_io_dispatching = false; window.__turts_io_seq = null; window.__turts_io_expected_event = null; }); }
        };
        try { return await action(send); }
        catch (error) {
            entry.error = String(error);
            throw error;
        } finally {
            const trace = await page().evaluate((offset) => ({ actual: (window.__turts_io_actual_events || []).slice(offset.actual), downstream: (window.__turts_io_downstream_events || []).slice(offset.downstream) }), traceStart);
            entry.actual_events = trace.actual;
            entry.downstream_events = trace.downstream;
        }
    }
    const noEvent = (operation) => async () => operation();
    const io = {
        async click(x, y, button = "left") {
            return record("click", [x, y, button], ["pointermove", "pointerdown", "pointerup"], async (send) => {
                await movePointer(x, y, send);
                await send("pointerdown", () => page().mouse.down({ button }), { client_x: cursor[state.selected].x, client_y: cursor[state.selected].y });
                await send("pointerup", () => page().mouse.up({ button }), { client_x: cursor[state.selected].x, client_y: cursor[state.selected].y });
            });
        },
        async drag(x0, y0, x1, y1, button = "left") {
            return record("drag", [x0, y0, x1, y1, button], ["pointerdown", "pointermove", "pointerup"], async (send) => {
                await movePointer(x0, y0, send);
                await send("pointerdown", () => page().mouse.down({ button }), { client_x: cursor[state.selected].x, client_y: cursor[state.selected].y });
                await movePointer(x1, y1, send, { steps: 8, present: true });
                await send("pointerup", () => page().mouse.up({ button }), { client_x: cursor[state.selected].x, client_y: cursor[state.selected].y });
            });
        },
        async key(code, modifiers = [], opts = {}) {
            return record("key", [code, modifiers, opts], ["keydown", "keyup"], async (send) => {
                const holdMs = Number(opts.hold_ms || 0);
                const action = opts.action || "press";
                if (!Number.isInteger(holdMs) || holdMs < 0 || holdMs > 60000) throw new RangeError("opts.hold_ms must be an integer from 0 to 60000");
                if (!["press", "down", "up"].includes(action)) throw new TypeError("opts.action must be press, down, or up");
                if (action !== "press" && modifiers.length > 0) throw new TypeError("opts.action down/up cannot include modifiers; hold each key with its own io.key call");
                if (action !== "press") {
                    const type = action === "down" ? "keydown" : "keyup";
                    if (opts && typeof opts.key === "string") {
                        const session = await page().context().newCDPSession(page());
                        try { await send(type, () => session.send("Input.dispatchKeyEvent", { type: action === "down" ? "keyDown" : "keyUp", code, key: opts.key, modifiers: 0 }), { code, key: opts.key }); }
                        finally { await session.detach(); }
                    } else {
                        await send(type, () => page().keyboard[action](code), keyExpectation(code));
                    }
                    return;
                }
                if (opts && typeof opts.key === "string") {
                    const session = await page().context().newCDPSession(page());
                    try {
                        let mask = 0;
                        for (const modifier of modifiers) {
                            const physical = modifierPhysicalKey(modifier);
                            mask |= physical.mask;
                            await send("keydown", () => session.send("Input.dispatchKeyEvent", { type: "keyDown", code: physical.code, key: physical.key, modifiers: mask }), { code: physical.code, key: physical.key });
                        }
                        await send("keydown", () => session.send("Input.dispatchKeyEvent", { type: "keyDown", code, key: opts.key, modifiers: mask }), { code, key: opts.key });
                        if (holdMs) await page().waitForTimeout(holdMs);
                        await send("keyup", () => session.send("Input.dispatchKeyEvent", { type: "keyUp", code, key: opts.key, modifiers: mask }), { code, key: opts.key });
                        for (const modifier of [...modifiers].reverse()) {
                            const physical = modifierPhysicalKey(modifier);
                            mask &= ~physical.mask;
                            await send("keyup", () => session.send("Input.dispatchKeyEvent", { type: "keyUp", code: physical.code, key: physical.key, modifiers: mask }), { code: physical.code, key: physical.key });
                        }
                    } finally { await session.detach(); }
                } else {
                    for (const modifier of modifiers) await send("keydown", () => page().keyboard.down(modifier), keyExpectation(modifier));
                    await send("keydown", () => page().keyboard.down(code), keyExpectation(code));
                    if (holdMs) await page().waitForTimeout(holdMs);
                    await send("keyup", () => page().keyboard.up(code), keyExpectation(code));
                    for (const modifier of [...modifiers].reverse()) await send("keyup", () => page().keyboard.up(modifier), keyExpectation(modifier));
                }
            });
        },
        async move(x, y) {
            return record("move", [x, y], ["pointermove"], async (send) => movePointer(x, y, send));
        },
        async wheel(dy) {
            return record("wheel", [dy], ["wheel"], async (send) => {
                const scale = Number(await page().evaluate(() => window.devicePixelRatio || 1)) || 1;
                return send("wheel", () => page().mouse.wheel(0, dy * scale), { delta_y: dy });
            });
        },
        async screenshot() { return record("screenshot", [], [], noEvent(async () => page().screenshot({ type: "png", scale: "css" }))); },
        async save(name, bytes) {
            const safe = safeName(name);
            const contents = Buffer.from(bytes);
            const identity = {
                byte_length: contents.length,
                sha256: crypto.createHash("sha256").update(contents).digest("hex"),
                saved_as: safe,
            };
            return record("save", [name, identity], [], noEvent(async () => {
                if (!outputDir) throw new Error("save requires an output directory");
                const target = path.resolve(outputDir, safe);
                if (!target.startsWith(path.resolve(outputDir) + path.sep)) throw new Error("unsafe artefact name");
                await fs.mkdir(path.dirname(target), { recursive: true });
                await fs.writeFile(target, contents);
            }));
        },
        async wait(ms) { return record("wait", [ms], [], noEvent(async () => page().waitForTimeout(ms))); },
        async reload() { return record("reload", [], [], noEvent(async () => { await page().reload({ waitUntil: "domcontentloaded" }); })); },
        async use(n) {
            if (!Number.isInteger(n) || n < 1 || n > pages.length) throw new RangeError(`window must be in 1..${pages.length}`);
            state.selected = n - 1;
            const started = wallTime();
            const entry = { call: "use", args: [n], wall_time_ms: started, window: n, event_types: [], event_seq: [], actual_events: [], downstream_events: [] };
            inputLog.push(entry);
            const downstreamStart = await page().evaluate(() => window.__turts_io_downstream_events?.length || 0);
            try {
                await page().bringToFront();
                await page().waitForTimeout(100);
                entry.downstream_events = await page().evaluate((offset) => (window.__turts_io_downstream_events || []).slice(offset), downstreamStart);
            } catch (error) {
                entry.error = String(error);
                throw error;
            }
        },
    };

    async function movePointer(x, y, send, { steps = 1, present = false } = {}) {
        const start = cursor[state.selected];
        if (start.x === x && start.y === y) return;
        for (let step = 1; step <= steps; step += 1) {
            const next = {
                x: start.x + (x - start.x) * step / steps,
                y: start.y + (y - start.y) * step / steps,
            };
            await send("pointermove", () => page().mouse.move(next.x, next.y), { client_x: next.x, client_y: next.y });
            cursor[state.selected] = next;
            if (present) await page().waitForTimeout(16);
        }
    }
    return { io: Object.freeze(io), inputLog, state };
}

function safeName(name) {
    if (typeof name !== "string" || !name || name.includes("\0")) throw new TypeError("save name must be a non-empty string");
    return name;
}

function keyExpectation(code) {
    const alias = { Shift: { code: "ShiftLeft", key: "Shift" }, Control: { code: "ControlLeft", key: "Control" }, Alt: { code: "AltLeft", key: "Alt" }, Meta: { code: "MetaLeft", key: "Meta" } };
    return alias[code] || { code };
}

function modifierPhysicalKey(name) {
    const keys = { Alt: { code: "AltLeft", key: "Alt", mask: 1 }, Control: { code: "ControlLeft", key: "Control", mask: 2 }, Meta: { code: "MetaLeft", key: "Meta", mask: 4 }, Shift: { code: "ShiftLeft", key: "Shift", mask: 8 } };
    const physical = keys[name];
    if (!physical) throw new TypeError(`unsupported modifier: ${name}`);
    return physical;
}

function modifierMask(modifiers) {
    return modifiers.reduce((value, name) => value | modifierPhysicalKey(name).mask, 0);
}

module.exports = { createDriverIo, IO_KEYS };
