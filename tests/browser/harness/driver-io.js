// turts-test-role: harness
"use strict";

const fs = require("node:fs/promises");
const path = require("node:path");
const crypto = require("node:crypto");
const { qualifyNavigationReceipt } = require("./navigation-receipt.js");
const { waitForSync } = require("./wait-sync.js");

// The longest wait for the page bridges after a reload that has scene checkpoints.
const BRIDGES_READY_MS = 20000;
const IO_KEYS = Object.freeze(["click", "drag", "key", "move", "wheel", "screenshot", "save", "wait", "reload", "use"]);

function wallTime() { return Date.now(); }

function createDriverIo(pages, { outputDir, inputLog = [], initialCursor = { x: 0, y: 0 }, afterScreenshot, documentEpochs = [], storageAudit = null, beforeReload = null, navigationArchive = null, screenshotScale = "css", retainScreenshots = false, observeReloadLoad = false } = {}) {
    if (!Array.isArray(pages) || pages.length === 0) throw new TypeError("at least one page is required");
    if (!["css", "device"].includes(screenshotScale)) throw new TypeError("invalid harness screenshot scale");
    if (!Number.isFinite(initialCursor.x) || !Number.isFinite(initialCursor.y)) throw new TypeError("initialCursor must have finite x and y");
    const state = { selected: 0 };
    const nextSeq = pages.map(() => 0);
    const cursor = pages.map(() => ({ x: initialCursor.x, y: initialCursor.y }));
    const heldModifiers = pages.map(() => new Map());
    const page = () => pages[state.selected];
    const heldModifierMask = (windowIndex = state.selected, excludingCode = null) => {
        let mask = 0;
        for (const [code, physical] of heldModifiers[windowIndex]) if (code !== excludingCode) mask |= physical.mask;
        return mask;
    };
    const setModifierHeld = (physical, held, windowIndex = state.selected) => {
        if (held) heldModifiers[windowIndex].set(physical.code, physical);
        else heldModifiers[windowIndex].delete(physical.code);
    };
    async function record(call, args, events, action) {
        const started = wallTime();
        const entry = { call, args: structuredClone(args), wall_time_ms: started, window: state.selected + 1,
            document_epoch: documentEpochs[state.selected] ?? 0, event_types: [], event_seq: [], actual_events: [] };
        inputLog.push(entry);
        await page().evaluate(({ call, args, callIndex }) => {
            window.__turts_io_active_call = { call, args, call_index: callIndex };
        }, { call, args: entry.args, callIndex: inputLog.length - 1 });
        const callIndex = inputLog.length - 1;
        const traceStart = await page().evaluate(() => ({ document_time_origin: performance.timeOrigin, document_url: document.URL, actual: window.__turts_io_actual_events?.length || 0,
            downstream: window.__turts_io_downstream_events?.length || 0, contextmenu: window.__turts_io_contextmenu_events?.length || 0 }));
        if (storageAudit) await collectStorageAudit(page(), storageAudit, state.selected + 1,
            documentEpochs[state.selected] ?? 0, "before_physical_action");
        let navigationTrace = null;
        const recoverNavigation = async (error) => {
            if (!Array.isArray(navigationArchive) || entry.call !== "click" ||
                (error && !/Execution context was destroyed|Cannot find context with specified id/.test(String(error)))) throw error || new Error("unqualified navigation");
            if (navigationTrace) return navigationTrace;
            await page().waitForLoadState("domcontentloaded");
            await waitForSync(page(), () => typeof window.turts_debug === "function", null,
                { timeout: 30000, polling: 20 });
            const nextDocument = await page().evaluate(() => ({ document_url: document.URL,
                document_time_origin: performance.timeOrigin }));
            const qualified = qualifyNavigationReceipt({ archives: navigationArchive, entry, traceStart, callIndex,
                afterEpoch: documentEpochs[state.selected] ?? 0, nextDocument });
            entry.navigation = qualified.navigation;
            navigationTrace = qualified.trace;
            return navigationTrace;
        };
        const send = async (kind, operation, expected = {}) => {
            if (!events.includes(kind)) throw new TypeError(`${call} cannot send ${kind}`);
            const seq = ++nextSeq[state.selected];
            await page().evaluate(({ value, type, match }) => { window.__turts_io_seq = value; window.__turts_io_dispatching = true; window.__turts_io_expected_event = { seq: value, type, ...match, matched: false }; }, { value: seq, type: kind, match: expected });
            entry.event_seq.push(seq);
            entry.event_types.push(kind);
            try { return await operation(); }
            finally {
                try { await page().evaluate(() => { window.__turts_io_dispatching = false; window.__turts_io_seq = null; window.__turts_io_expected_event = null; }); }
                catch (error) { await recoverNavigation(error); }
            }
        };
        try { return await action(send, entry); }
        catch (error) {
            try { await recoverNavigation(error); }
            catch { entry.error = String(error); throw error; }
        } finally {
            if (storageAudit) await collectStorageAudit(page(), storageAudit, state.selected + 1,
                documentEpochs[state.selected] ?? 0, "after_physical_action");
            let trace;
            try {
                if (navigationTrace || (navigationArchive && entry.call === "click" && (documentEpochs[state.selected] ?? 0) > entry.document_epoch)) {
                    trace = await recoverNavigation();
                } else trace = await page().evaluate(async (offset) => {
                await new Promise((resolve) => setTimeout(resolve, 0));
                return {
                    actual: (window.__turts_io_actual_events || []).slice(offset.actual),
                    downstream: (window.__turts_io_downstream_events || []).slice(offset.downstream),
                    contextmenu: (window.__turts_io_contextmenu_events || []).slice(offset.contextmenu),
                    contextmenu_overflow: window.__turts_io_contextmenu_overflow === true,
                };
                }, traceStart);
            } catch (error) { trace = await recoverNavigation(error); }
            entry.actual_events = trace.actual;
            entry.downstream_events = trace.downstream;
            entry.contextmenu_events = trace.contextmenu;
            entry.contextmenu_overflow = trace.contextmenu_overflow;
            entry.completed_wall_time_ms = wallTime();
            try { await page().evaluate(() => { window.__turts_io_active_call = null; }); }
            catch (error) {
                const recovered = await recoverNavigation(error);
                entry.actual_events = recovered.actual;
                entry.downstream_events = recovered.downstream;
                entry.contextmenu_events = recovered.contextmenu;
                entry.contextmenu_overflow = recovered.contextmenu_overflow;
                entry.completed_wall_time_ms = wallTime();
            }
        }
    }
    const noEvent = (operation) => async () => operation();
    const io = {
        async click(x, y, button = "left", options = undefined) {
            if (options !== undefined && (!options || typeof options !== "object" || Array.isArray(options) || Object.keys(options).some((key) => key !== "count"))) {
                throw new TypeError("click options may contain only count");
            }
            const count = options === undefined ? 1 : options?.count ?? 1;
            if (!Number.isInteger(count) || count < 1 || count > 100) throw new RangeError("click count must be an integer from 1 to 100");
            const args = options === undefined ? [x, y, button] : [x, y, button, structuredClone(options)];
            return record("click", args, ["pointermove", "pointerdown", "pointerup"], async (send, entry) => {
                await movePointer(x, y, send);
                let frameHold = false;
                if (count > 1) {
                    await page().evaluate(() => {
                        if (typeof window.__turts_begin_bounded_input !== "function" || typeof window.__turts_end_bounded_input !== "function") {
                            throw new Error("bounded input frame hook is unavailable");
                        }
                        window.__turts_begin_bounded_input();
                    });
                    frameHold = true;
                    entry.frame_hold = { requested: true, count };
                }
                try {
                    for (let index = 0; index < count; index += 1) {
                        await send("pointerdown", () => page().mouse.down({ button }), { client_x: cursor[state.selected].x, client_y: cursor[state.selected].y });
                        await send("pointerup", () => page().mouse.up({ button }), { client_x: cursor[state.selected].x, client_y: cursor[state.selected].y });
                    }
                } finally {
                    if (frameHold) {
                        const released = await page().evaluate(() => window.__turts_end_bounded_input());
                        entry.frame_hold = { requested: true, count, ...released };
                    }
                }
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
                const windowIndex = state.selected;
                const keyModifier = maybeModifierPhysicalKey(code);
                if (action !== "press") {
                    const type = action === "down" ? "keydown" : "keyup";
                    if (opts && typeof opts.key === "string") {
                        if (keyModifier) requirePlaywrightKeyboardState(page());
                        const session = await page().context().newCDPSession(page());
                        const codeForDispatch = keyModifier?.code || code;
                        const mask = action === "down"
                            ? heldModifierMask(windowIndex) | (keyModifier?.mask || 0)
                            : heldModifierMask(windowIndex, keyModifier?.code || null);
                        try { await send(type, () => session.send("Input.dispatchKeyEvent", { type: action === "down" ? "keyDown" : "keyUp", code: codeForDispatch, key: opts.key, modifiers: mask }), { code: codeForDispatch, key: opts.key }); }
                        finally { await session.detach(); }
                    } else {
                        const codeForDispatch = keyModifier?.code || code;
                        await send(type, () => page().keyboard[action](codeForDispatch), keyExpectation(code));
                    }
                    if (keyModifier) {
                        setModifierHeld(keyModifier, action === "down", windowIndex);
                        if (opts && typeof opts.key === "string") syncPlaywrightModifierState(page(), heldModifiers[windowIndex]);
                    }
                    return;
                }
                if (opts && typeof opts.key === "string") {
                    if (modifiers.length > 0 || heldModifiers[windowIndex].size > 0) requirePlaywrightKeyboardState(page());
                    const session = await page().context().newCDPSession(page());
                    const temporary = [];
                    try {
                        for (const modifier of modifiers) {
                            const physical = modifierPhysicalKey(modifier);
                            if (heldModifiers[windowIndex].has(physical.code)) continue;
                            const mask = heldModifierMask(windowIndex) | physical.mask;
                            await send("keydown", () => session.send("Input.dispatchKeyEvent", { type: "keyDown", code: physical.code, key: physical.key, modifiers: mask }), { code: physical.code, key: physical.key });
                            setModifierHeld(physical, true, windowIndex);
                            syncPlaywrightModifierState(page(), heldModifiers[windowIndex]);
                            temporary.push(physical);
                        }
                        const codeForDispatch = keyModifier?.code || code;
                        await send("keydown", () => session.send("Input.dispatchKeyEvent", { type: "keyDown", code: codeForDispatch, key: opts.key, modifiers: heldModifierMask(windowIndex) | (keyModifier?.mask || 0) }), { code: codeForDispatch, key: opts.key });
                        if (holdMs) await page().waitForTimeout(holdMs);
                        await send("keyup", () => session.send("Input.dispatchKeyEvent", { type: "keyUp", code: codeForDispatch, key: opts.key, modifiers: heldModifierMask(windowIndex) }), { code: codeForDispatch, key: opts.key });
                    } finally {
                        for (const physical of [...temporary].reverse()) {
                            if (!heldModifiers[windowIndex].has(physical.code)) continue;
                            const mask = heldModifierMask(windowIndex, physical.code);
                            await send("keyup", () => session.send("Input.dispatchKeyEvent", { type: "keyUp", code: physical.code, key: physical.key, modifiers: mask }), { code: physical.code, key: physical.key });
                            setModifierHeld(physical, false, windowIndex);
                            syncPlaywrightModifierState(page(), heldModifiers[windowIndex]);
                        }
                        await session.detach();
                    }
                } else {
                    const temporary = [];
                    try {
                        for (const modifier of modifiers) {
                            const physical = modifierPhysicalKey(modifier);
                            if (heldModifiers[windowIndex].has(physical.code)) continue;
                            await send("keydown", () => page().keyboard.down(physical.code), keyExpectation(modifier));
                            setModifierHeld(physical, true, windowIndex);
                            temporary.push(physical);
                        }
                        await send("keydown", () => page().keyboard.down(keyModifier?.code || code), keyExpectation(code));
                        if (holdMs) await page().waitForTimeout(holdMs);
                        await send("keyup", () => page().keyboard.up(keyModifier?.code || code), keyExpectation(code));
                    } finally {
                        for (const physical of [...temporary].reverse()) {
                            if (!heldModifiers[windowIndex].has(physical.code)) continue;
                            await send("keyup", () => page().keyboard.up(physical.code), keyExpectation(physical.code));
                            setModifierHeld(physical, false, windowIndex);
                        }
                    }
                }
            });
        },
        async move(x, y) {
            return record("move", [x, y], ["pointermove"], async (send) => movePointer(x, y, send));
        },
        async wheel(dy) {
            return record("wheel", [dy], ["wheel"], async (send) => {
                return send("wheel", () => page().mouse.wheel(0, dy), { delta_y: dy });
            });
        },
        async screenshot() {
            const input_call = inputLog.length;
            const window = state.selected + 1;
            const document_epoch = documentEpochs[state.selected] ?? 0;
            const screenshotPage = page();
            const bytes = await record("screenshot", [], [], noEvent(async () => {
                const bytes = await screenshotPage.screenshot({ type: "png", scale: screenshotScale });
                // PT1 (ruling C386): the harness keeps each screenshot under its input-log index.
                if (retainScreenshots && outputDir) await fs.writeFile(path.join(outputDir, `harness-shot-${input_call}.png`), bytes);
                if (afterScreenshot) await afterScreenshot({ input_call, window, document_epoch, wall_time_ms: wallTime() });
                return bytes;
            }));
            Object.defineProperties(bytes, {
                input_call: { value: input_call },
                window: { value: window },
                document_epoch: { value: document_epoch },
            });
            return bytes;
        },
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
        async reload() { return record("reload", [], [], async (_send, entry) => {
            const before = documentEpochs[state.selected] ?? 0;
            entry.document_epoch_before = before;
            if (storageAudit) await collectStorageAudit(page(), storageAudit, state.selected + 1,
                documentEpochs[state.selected] ?? 0, "before_reload");
            if (beforeReload) await beforeReload(page(), state.selected);
            await page().reload({ waitUntil: "domcontentloaded" });
            if (observeReloadLoad) {
                // PT1: record the load screen of the reload in the input log (at most 20 s).
                entry.load_observations = [];
                const deadline = wallTime() + 20000;
                while (wallTime() < deadline) {
                    let observed = null;
                    try {
                        observed = await page().evaluate(async () => {
                            let debug = null;
                            // Ask for one record (design D 9, row DBG1); an older page publishes in each frame.
                            try {
                                debug = typeof window.turts_debug !== "function" ? null : JSON.parse(
                                    await (window.turts_debug_request?.() ?? window.turts_debug()));
                            } catch { debug = null; }
                            const rows = (debug?.ui_snapshot ?? []).filter((row) => String(row.id ?? "").startsWith("shell.load") || row.kind === "focus_overlay");
                            return { phase: debug?.phase ?? null, joined: typeof debug?.phase === "string" && debug.phase.startsWith("Joined") && Boolean(debug.last_frame), rows };
                        });
                    } catch { observed = null; }
                    if (observed) entry.load_observations.push({ wall_time_ms: wallTime(), ...observed });
                    if (observed?.joined) break;
                    await page().waitForTimeout(150);
                }
            }
            if (afterScreenshot && !observeReloadLoad) {
                // A screenshot of a page that has no bridge yet cannot get its scene checkpoint
                // ("missing scene checkpoint bridge", RUN142 AUD1). The wait is in the harness, not
                // in the driver, and it is bounded. A page that stays without bridges after the bound
                // fails the reload with a text that names the fault (see below), so the red does not
                // blame a later wait that already used its time on the dead page.
                entry.bridges_ready_after_reload = await page().waitForFunction(() => {
                    try {
                        return typeof window.turts_scene === "function" && typeof window.turts_debug === "function" &&
                            Boolean(window.turts_scene()) && Boolean(window.turts_debug());
                    } catch { return false; }
                }, null, { timeout: BRIDGES_READY_MS, polling: 100 }).then(() => true, () => false);
            }
            const after = documentEpochs[state.selected] ?? 0;
            entry.document_epoch_after = after;
            if (!Number.isSafeInteger(before) || before < 1 || !Number.isSafeInteger(after) || after <= before) {
                throw new Error(`reload did not advance the actual document epoch: ${before} -> ${after}`);
            }
            if (entry.bridges_ready_after_reload === false) {
                throw new Error(`reload: page bridges absent after ${BRIDGES_READY_MS / 1000} s`);
            }
        }); },
        async use(n) {
            if (!Number.isInteger(n) || n < 1 || n > pages.length) throw new RangeError(`window must be in 1..${pages.length}`);
            state.selected = n - 1;
            const started = wallTime();
            const entry = { call: "use", args: [n], wall_time_ms: started, window: n,
                document_epoch: documentEpochs[state.selected] ?? 0, event_types: [], event_seq: [], actual_events: [], downstream_events: [] };
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
    async function flushStorageAudit() {
        if (!storageAudit) return null;
        const finalState = [];
        for (let index = 0; index < pages.length; index += 1) {
            const snapshot = await pages[index].evaluate(() => {
                const audit = window.__task16_storage_audit;
                const sent = Number.isSafeInteger(window.__task16_storage_audit_sent) ? window.__task16_storage_audit_sent : 0;
                const operations = Array.isArray(audit) ? audit.slice(sent) : [];
                if (Array.isArray(audit)) window.__task16_storage_audit_sent = audit.length;
                const entries = (storage) => Object.fromEntries(Array.from({ length: storage.length }, (_, i) => {
                    const key = storage.key(i);
                    return [key, storage.getItem(key)];
                }));
                return { operations, sessionStorage: entries(sessionStorage), localStorage: entries(localStorage) };
            });
            for (const entry of snapshot.operations) storageAudit.push({ ...entry, window: index + 1,
                document_epoch: documentEpochs[index] ?? 0, input_call: null, flush_point: "end_of_run" });
            finalState.push({ window: index + 1, document_epoch: documentEpochs[index] ?? 0,
                sessionStorage: snapshot.sessionStorage, localStorage: snapshot.localStorage });
        }
        return finalState;
    }
    return { io: Object.freeze(io), inputLog, state, storageAudit, flushStorageAudit };
}

async function collectStorageAudit(page, output, window, document_epoch, flush_point) {
    const entries = await page.evaluate(() => {
        const audit = window.__task16_storage_audit;
        if (!Array.isArray(audit)) return [];
        const sent = Number.isSafeInteger(window.__task16_storage_audit_sent) ? window.__task16_storage_audit_sent : 0;
        window.__task16_storage_audit_sent = audit.length;
        return audit.slice(sent);
    });
    for (const entry of entries) output.push({ ...entry, window, document_epoch, input_call: null, flush_point });
}

function safeName(name) {
    if (typeof name !== "string" || !name || name.includes("\0")) throw new TypeError("save name must be a non-empty string");
    return name;
}

function keyExpectation(code) {
    return maybeModifierPhysicalKey(code) || { code };
}

function modifierPhysicalKey(name) {
    const physical = maybeModifierPhysicalKey(name);
    if (!physical) throw new TypeError(`unsupported modifier: ${name}`);
    return physical;
}

function maybeModifierPhysicalKey(name) {
    const keys = {
        Alt: { code: "AltLeft", key: "Alt", mask: 1 }, Option: { code: "AltLeft", key: "Alt", mask: 1 }, AltLeft: { code: "AltLeft", key: "Alt", mask: 1 }, AltRight: { code: "AltRight", key: "Alt", mask: 1 },
        Control: { code: "ControlLeft", key: "Control", mask: 2 }, Ctrl: { code: "ControlLeft", key: "Control", mask: 2 }, ControlLeft: { code: "ControlLeft", key: "Control", mask: 2 }, ControlRight: { code: "ControlRight", key: "Control", mask: 2 },
        Meta: { code: "MetaLeft", key: "Meta", mask: 4 }, Cmd: { code: "MetaLeft", key: "Meta", mask: 4 }, Command: { code: "MetaLeft", key: "Meta", mask: 4 }, MetaLeft: { code: "MetaLeft", key: "Meta", mask: 4 }, MetaRight: { code: "MetaRight", key: "Meta", mask: 4 },
        Shift: { code: "ShiftLeft", key: "Shift", mask: 8 }, ShiftLeft: { code: "ShiftLeft", key: "Shift", mask: 8 }, ShiftRight: { code: "ShiftRight", key: "Shift", mask: 8 },
    };
    return keys[name] || null;
}

function requirePlaywrightKeyboardState(page) {
    const clientPage = page?.keyboard?._page;
    const connection = clientPage?._connection;
    if (!clientPage || typeof connection?.toImpl !== "function") {
        throw new Error("held CDP modifiers require the in-process Playwright page implementation bridge");
    }
    let implementationPage;
    try { implementationPage = connection.toImpl(clientPage); }
    catch (error) { throw new Error(`cannot resolve Playwright's in-process page implementation: ${error.message || String(error)}`); }
    const keyboard = implementationPage?.keyboard;
    const mouseKeyboard = implementationPage?.mouse?._keyboard;
    if (!(keyboard?._pressedModifiers instanceof Set) || !(keyboard?._pressedKeys instanceof Set) || mouseKeyboard !== keyboard) {
        throw new Error("unsupported Playwright backend: keyboard modifier sets and the page mouse keyboard link are required");
    }
    return keyboard;
}

function syncPlaywrightModifierState(page, held) {
    const keyboard = requirePlaywrightKeyboardState(page);
    const activeNames = new Set([...held.values()].map((physical) => physical.key));
    for (const name of ["Alt", "Control", "Meta", "Shift"]) {
        if (activeNames.has(name)) keyboard._pressedModifiers.add(name);
        else keyboard._pressedModifiers.delete(name);
    }
    for (const code of ["AltLeft", "AltRight", "ControlLeft", "ControlRight", "MetaLeft", "MetaRight", "ShiftLeft", "ShiftRight"]) {
        if (held.has(code)) keyboard._pressedKeys.add(code);
        else keyboard._pressedKeys.delete(code);
    }
}

module.exports = { createDriverIo, IO_KEYS };
