// turts-test-role: instrument
"use strict";

// Q-COLD pass 2: the scripted probes of the cold-play tool. They run with no agent (docs/qa/cold-play.md).
//
// Why they exist: a fresh agent read the whole guide, closed it, and answered "yes" on the build that a person
// could not play. An agent obeys every text; a person clicks what they see. The verdict of a readiness run
// therefore comes from these probes and from the UI record, never from the answer of an agent.
//
//   first-click  One click to start (the start control that the UI record shows; the page centre when it shows
//                none), then one left click at the screen centre of EACH own unit that is on screen. No key, no
//                other input. For each click it records the selection after the click and the pixel change at
//                +200 ms in a box around the point and in the whole page. PASS only when every click gives a
//                selection that holds the clicked unit AND a visible change (condition M1).
//   buttons      No start click. For each button that the UI record shows as visible, top to bottom: the pointer
//                moves onto it, rests 300 ms (its hover feedback, such as a tooltip, is not the click), and one
//                left click follows. A button "acted" when the UI record changed (the start overlay left out) or the page
//                changed. A button whose first click is lost (a layer takes it) is the failure sign F8.
//
// The probes read the bridges (turts_debug, turts_scene, turts_ui). This file is role `instrument`: nothing of
// this goes to an agent, and a probe session has no agent.
const fs = require("node:fs");
const path = require("node:path");
const { changeStats } = require("./cold-pixels.js");
const { decodePng } = require("./harness/png.js");

const READY_TIMEOUT_MS = 90000;
const UNIT_CAP = 60;
const BUTTON_CAP = 20;
const BOX_HALF = 48;
const MIN_BOX_PIXELS = 40;
const PAGE_SHARE = 0.01;
const PAGE_MARGIN = 2000;

// Runs in the page: the three bridges as one compact record. `units` joins the owner (from the replica) with the
// screen centre (from the scene). Nothing here is sent to an agent.
async function readWorld() {
    const parse = (raw) => (typeof raw === "string" ? JSON.parse(raw) : raw);
    if (typeof window.turts_debug !== "function") return { available: false };
    const debug = parse(await (window.turts_debug_request?.() ?? window.turts_debug()));
    const scene = typeof window.turts_scene === "function" ? parse(window.turts_scene()) : null;
    const ui = typeof window.turts_ui === "function" ? parse(window.turts_ui()) : null;
    const centers = new Map(((scene && scene.units) || []).map((unit) => [unit.unit_id, unit.ground_screen_center ?? null]));
    const replica = (debug.replicated_world && debug.replicated_world.units) || debug.units || [];
    const compact = (widget) => ({
        id: widget.id, kind: widget.kind ?? null, rect: widget.rect ?? null, text: typeof widget.text === "string" ? widget.text.slice(0, 4000) : null,
        visible: widget.visible === true, enabled: widget.enabled !== false, capture_root: widget.capture_root === true,
        focus_policy_block: widget.focus_policy_block === true, text_nodes: Array.isArray(widget.text_nodes) ? widget.text_nodes.slice(0, 4) : null,
    });
    return {
        available: true, phase: debug.phase ?? null, slot: debug.slot ?? null, selection: Array.isArray(debug.selection) ? debug.selection : [],
        alerts: Array.isArray(debug.alerts) ? debug.alerts.length : 0,
        units: replica.map((unit) => ({ unit_id: unit.unit_id, owner_slot: unit.owner_slot ?? null, content_id: unit.content_id ?? null, center: centers.get(unit.unit_id) ?? null })),
        ui: Array.isArray(ui) ? ui.map(compact) : [],
    };
}

function wordCount(text) {
    return typeof text === "string" ? text.split(/\s+/).filter(Boolean).length : 0;
}

const inside = (point, viewport) => point && point[0] >= 0 && point[1] >= 0 && point[0] < viewport.width && point[1] < viewport.height;

// The own units that are on screen, one of each content id first (by id), then the others (by id), at most `cap`.
function ownUnitsOnScreen(world, viewport, cap = UNIT_CAP) {
    const own = (world.units || []).filter((unit) => unit.owner_slot === world.slot && inside(unit.center, viewport)).sort((a, b) => a.unit_id - b.unit_id);
    const seen = new Set();
    const first = [];
    const rest = [];
    for (const unit of own) {
        if (seen.has(unit.content_id)) rest.push(unit);
        else { seen.add(unit.content_id); first.push(unit); }
    }
    return [...first, ...rest].slice(0, cap);
}

const center = (rect) => [Math.floor(rect[0] + rect[2] / 2), Math.floor(rect[1] + rect[3] / 2)];

// The buttons that a person sees and can press, top to bottom, then left to right.
function visibleButtons(ui, viewport) {
    return ui.filter((widget) => widget.kind === "button" && widget.visible && widget.enabled && widget.rect && widget.rect[2] > 0 && widget.rect[3] > 0
        && inside(center(widget.rect), viewport))
        .sort((a, b) => a.rect[1] - b.rect[1] || a.rect[0] - b.rect[0]);
}

// The record that a click is judged by: id to the state of the widget. The start overlay is left out: it leaves on
// the first click whatever that click was.
function normalisedUi(ui) {
    const record = {};
    for (const widget of ui) {
        if (widget.kind === "focus_overlay" || String(widget.id).startsWith("focus_overlay")) continue;
        // A tooltip shows when the pointer rests on a control: hover feedback, not the effect of a click.
        if (String(widget.id).endsWith("::tooltip")) continue;
        record[widget.id] = JSON.stringify([widget.visible, widget.enabled, widget.text ?? null, (widget.rect ?? []).map(Math.round)]);
    }
    return record;
}

const changedIds = (before, after, ignore = new Set()) => [...new Set([...Object.keys(before), ...Object.keys(after)])].filter((id) => before[id] !== after[id] && !ignore.has(id)).sort();

const pad = (n) => String(n).padStart(3, "0");
const image = (file) => decodePng(fs.readFileSync(file));

async function waitReady(session) {
    const start = session.t();
    for (;;) {
        const world = await session.readWorld().catch(() => ({ available: false }));
        if (world.available && world.ui.length > 0 && world.units.length > 0 && (world.phase === null || /Joined/.test(world.phase))) return { world, waited_ms: session.t() - start };
        if (session.t() - start > READY_TIMEOUT_MS) return { world: null, waited_ms: session.t() - start };
        await session.sleep(500);
    }
}

// The big layer of a UI record: visible, not the start overlay, more than half of the page.
function bigLayers(ui, viewport) {
    return ui.filter((widget) => widget.visible && widget.kind !== "focus_overlay" && widget.rect
        && (widget.rect[2] * widget.rect[3]) / (viewport.width * viewport.height) > 0.5);
}

// The positive control: close the layer that the first click met, by the button that the UI record shows for it.
async function closeLayer(session) {
    const viewport = session.size;
    const record = { found: false, clicks: 0, closed: false };
    for (let i = 0; i < 3; i += 1) {
        const world = await session.readWorld();
        if (bigLayers(world.ui, viewport).length === 0) { record.closed = true; break; }
        const button = visibleButtons(world.ui, viewport).find((widget) => /^(close|dismiss|ok|got it|x|×)$/i.test((widget.text ?? "").trim()));
        if (!button) break;
        record.found = true;
        const [x, y] = center(button.rect);
        await session.handle({ cmd: "click", args: [String(x), String(y), "left"] });
        record.clicks += 1;
        await session.sleep(300);
    }
    if (!record.closed) record.closed = bigLayers((await session.readWorld()).ui, viewport).length === 0;
    return record;
}

async function firstClick(session, options) {
    const viewport = session.size;
    const record = { kind: "first-click", control: options.control ?? null, ready_ms: null, start: null, layer_close: null, clicks: [], summary: null };
    const ready = await waitReady(session);
    record.ready_ms = ready.waited_ms;
    if (!ready.world) {
        record.summary = { verdict: "FAIL", clicks: 0, failed: 0, first_failure: null, reason: `the page was not ready within ${READY_TIMEOUT_MS / 1000} s` };
        return record;
    }
    // The start click: the start control of the UI record, else the page centre.
    const overlay = ready.world.ui.find((widget) => widget.kind === "focus_overlay" && widget.visible);
    const node = overlay && overlay.text_nodes && overlay.text_nodes[0] && overlay.text_nodes[0].rect;
    const point = node ? center(node) : [Math.floor(viewport.width / 2), Math.floor(viewport.height / 2)];
    record.start = { found: overlay ? "focus_overlay" : "viewport-centre", point };
    await session.handle({ cmd: "click", args: [String(point[0]), String(point[1]), "left"] });
    await session.sleep(300);
    if (options.control === "close-layer") record.layer_close = await closeLayer(session);
    const world = await session.readWorld();
    const units = ownUnitsOnScreen(world, viewport);
    if (units.length === 0) {
        record.summary = { verdict: "FAIL", clicks: 0, failed: 0, first_failure: null, reason: "no own unit on screen after the start click: nothing to click" };
        return record;
    }
    let number = 0;
    for (const unit of units) {
        number += 1;
        // The unit may have moved: read its centre now.
        const fresh = (await session.readWorld()).units.find((entry) => entry.unit_id === unit.unit_id);
        if (!fresh || !inside(fresh.center, viewport)) { record.clicks.push({ n: number, unit_id: unit.unit_id, content_id: unit.content_id, skipped: "not on screen at the click", ok: true }); continue; }
        const [x, y] = fresh.center.map(Math.round);
        const a = await session.shot(`probe-${pad(number)}-a`, session.t());
        await session.sleep(200);
        const b = await session.shot(`probe-${pad(number)}-b`, session.t());
        let p200 = null;
        await session.handle({ cmd: "click", args: [String(x), String(y), "left"], hook: (offset, file) => { if (offset === 200) p200 = file; } });
        const after = await session.readWorld();
        const box = [x - BOX_HALF, y - BOX_HALF, BOX_HALF * 2, BOX_HALF * 2];
        const idle = changeStats(image(a), image(b), box);
        const change = changeStats(image(b), image(p200), box);
        const visible = change.box_changed >= Math.max(MIN_BOX_PIXELS, 3 * idle.box_changed)
            || (change.page_share >= PAGE_SHARE && change.page_changed >= 3 * idle.page_changed + PAGE_MARGIN);
        const selectionOk = after.selection.includes(unit.unit_id);
        record.clicks.push({
            n: number, unit_id: unit.unit_id, content_id: unit.content_id, point: [x, y], selection: after.selection, selection_ok: selectionOk,
            box_changed: change.box_changed, box_px: change.box_px, page_changed: change.page_changed, page_share: change.page_share,
            idle_box_changed: idle.box_changed, idle_page_changed: idle.page_changed, visible_change: visible, ok: selectionOk && visible,
        });
    }
    const made = record.clicks.filter((click) => !click.skipped);
    const failed = made.filter((click) => !click.ok);
    const first = failed[0] ?? null;
    record.summary = {
        verdict: made.length > 0 && failed.length === 0 ? "PASS" : "FAIL", clicks: made.length, failed: failed.length,
        first_failure: first && { n: first.n, unit_id: first.unit_id, content_id: first.content_id },
        reason: made.length === 0 ? "every own unit left the screen before its click"
            : first ? `first click on unit ${first.unit_id} (${first.content_id}): selection ${JSON.stringify(first.selection)}, ${first.visible_change ? "a visible change" : "no visible change"}`
                + (failed.length > 1 ? `; ${failed.length} of ${made.length} clicks failed` : "")
                : `${made.length} of ${made.length} clicks selected the unit and changed the picture`,
    };
    return record;
}

async function buttons(session, options) {
    const viewport = session.size;
    const record = { kind: "buttons", ready_ms: null, idle_changed_ids: [], buttons: [], summary: null };
    const ready = await waitReady(session);
    record.ready_ms = ready.waited_ms;
    if (!ready.world) {
        record.summary = { verdict: "FAIL", failed: 0, reason: `the page was not ready within ${READY_TIMEOUT_MS / 1000} s` };
        return record;
    }
    // What changes by itself in 200 ms, with no input: it is not the effect of a click.
    const first = normalisedUi(ready.world.ui);
    await session.sleep(200);
    const second = normalisedUi((await session.readWorld()).ui);
    const idle = new Set(changedIds(first, second));
    record.idle_changed_ids = [...idle].sort();
    const list = visibleButtons(ready.world.ui, viewport).slice(0, BUTTON_CAP);
    let number = 0;
    for (const listed of list) {
        number += 1;
        const world = await session.readWorld();
        const button = world.ui.find((widget) => widget.id === listed.id);
        if (!button || !visibleButtons([button], viewport).length) { record.buttons.push({ n: number, id: listed.id, skipped: "not visible at its turn", acted: true }); continue; }
        const [x, y] = center(button.rect);
        // The pointer rests on the button first, as a person's does: what the hover shows is not the click.
        await session.handle({ cmd: "move", args: [String(x), String(y)] });
        await session.sleep(300);
        const before = normalisedUi((await session.readWorld()).ui);
        const a = await session.shot(`probe-${pad(number)}-a`, session.t());
        await session.sleep(200);
        const b = await session.shot(`probe-${pad(number)}-b`, session.t());
        let p200 = null;
        await session.handle({ cmd: "click", args: [String(x), String(y), "left"], hook: (offset, file) => { if (offset === 200) p200 = file; } });
        const afterWorld = await session.readWorld();
        const changed = changedIds(before, normalisedUi(afterWorld.ui), idle);
        const idlePixels = changeStats(image(a), image(b), [0, 0, 1, 1]);
        const pixels = changeStats(image(b), image(p200), [x - BOX_HALF, y - BOX_HALF, BOX_HALF * 2, BOX_HALF * 2]);
        // The start overlay dims the whole page; when it leaves on this click, the page changes for that reason.
        const overlayLeft = world.ui.some((widget) => widget.kind === "focus_overlay" && widget.visible) && !afterWorld.ui.some((widget) => widget.kind === "focus_overlay" && widget.visible);
        const actedPixels = !overlayLeft && pixels.page_share >= PAGE_SHARE && pixels.page_changed >= 3 * idlePixels.page_changed + PAGE_MARGIN;
        record.buttons.push({ n: number, id: button.id, text: button.text, point: [x, y], acted: changed.length > 0 || actedPixels, acted_ui: changed.length > 0,
            acted_pixels: actedPixels, overlay_left: overlayLeft, changed_ids: changed, page_share: pixels.page_share });
    }
    const lost = record.buttons.filter((button) => !button.acted);
    record.summary = {
        verdict: lost.length === 0 ? "PASS" : "FAIL", buttons: record.buttons.length, failed: lost.length,
        reason: record.buttons.length === 0 ? "no visible button: nothing to click"
            : lost.length ? `the first click on ${lost.map((button) => button.id).join(", ")} changed nothing` : `${record.buttons.length} of ${record.buttons.length} buttons acted on the first click`,
    };
    return record;
}

async function runProbe(session, kind, options = {}) {
    if (!["first-click", "buttons"].includes(kind)) throw new Error(`unknown probe ${kind}`);
    const record = kind === "first-click" ? await firstClick(session, options) : await buttons(session, options);
    fs.writeFileSync(path.join(session.dir, `probe-${kind}.json`), JSON.stringify(record, null, 2) + "\n");
    return record;
}

module.exports = { runProbe, readWorld, wordCount, ownUnitsOnScreen, visibleButtons, normalisedUi, bigLayers, center };
