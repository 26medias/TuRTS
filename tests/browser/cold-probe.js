// turts-test-role: instrument
"use strict";

// Q-COLD: the scripted probes of the cold-play tool. They run with no agent (docs/qa/cold-play.md).
//
// Why they exist: a fresh agent read the whole guide, closed it, and answered "yes" on the build that a person
// could not play. An agent obeys every text; a person clicks what they see. The verdict of a readiness run
// therefore comes from these probes and from the UI record, never from the answer of an agent.
//
//   first-click  Wait for the page, watch it 8 s (the UI record each 1 s), then ONE click to start (the start control
//                that the UI record shows; the page centre when it shows none), then one left click at the screen centre
//                of EACH own unit on screen. No key. For each click: the selection after the click and whether the
//                click was SEEN: a pixel change in a box around the unit (at least 1.5 times its size on screen), or
//                a change of a panel in the UI record (alert lines, tooltips and what changes by itself are left out).
//                A change elsewhere on the page does not count. PASS only when every click gives a selection that
//                holds the unit AND is seen (P1).
//                Then, for the first unit of each content id, the order buttons of its selection are clicked one time
//                each (P3): each needs a visible result within 1 s, a change of the panel or state, or a new notice.
//                Then four more clicks (P4): an enemy unit (its panel shows), a resource node (a name shows), open
//                ground after a selection (the selection clears), a right click on open ground with a unit selected
//                (a notice, an order mark or a pixel mark within 1 s).
//   buttons      Watch 20 s with no input (F6). Then F8-start: the first visible button under the start overlay gets
//                one click before any start click; a lost click is a finding of its own. Then the start click, and one
//                click on each visible button (F8): the pointer rests 300 ms, then one left click.
//
// The probes read the bridges (turts_debug, turts_scene, turts_ui). This file is role `instrument`: nothing of
// this goes to an agent, and a probe session has no agent.
const fs = require("node:fs");
const path = require("node:path");
const { changeStats } = require("./cold-pixels.js");
const { decodePng } = require("./harness/png.js");

const READY_TIMEOUT_MS = 90000;
// A person looks at the page for a few seconds before the first input. The first-click probe waits SETTLE_MS (the P1
// result does not change with time: a layer that opens at the join is there at any time). The buttons probe watches
// BUTTONS_SETTLE_MS for alert lines (F6; ALR2 forbids one at any time before the first input).
const SETTLE_MS = 8000;
const BUTTONS_SETTLE_MS = 20000;
const WATCH_STEP_MS = 1000;
const UNIT_CAP = 60;
const BUTTON_CAP = 20;
const ORDER_CAP = 40;
const MIN_BOX_PIXELS = 40;
const BOX_MIN_HALF = 48;
const BOX_SCALE = 1.5;
const RESULT_MS = 1000;
const DESTRUCTIVE = /remove|delete|sell|demolish|dismantle|destroy/i;

// Runs in the page: the three bridges as one compact record. `units` joins the owner (from the replica) with the
// screen centre (from the scene). Nothing here is sent to an agent.
async function readWorld() {
    const parse = (raw) => (typeof raw === "string" ? JSON.parse(raw) : raw);
    if (typeof window.turts_debug !== "function") return { available: false };
    const debug = parse(await (window.turts_debug_request?.() ?? window.turts_debug()));
    const scene = typeof window.turts_scene === "function" ? parse(window.turts_scene()) : null;
    const ui = typeof window.turts_ui === "function" ? parse(window.turts_ui()) : null;
    const entries = new Map(((scene && scene.units) || []).map((unit) => [unit.unit_id, unit]));
    const replica = (debug.replicated_world && debug.replicated_world.units) || debug.units || [];
    const compact = (widget) => ({
        id: widget.id, kind: widget.kind ?? null, rect: widget.rect ?? null, text: typeof widget.text === "string" ? widget.text.slice(0, 4000) : null,
        visible: widget.visible === true, enabled: widget.enabled !== false, capture_root: widget.capture_root === true,
        focus_policy_block: widget.focus_policy_block === true, text_nodes: Array.isArray(widget.text_nodes) ? widget.text_nodes.slice(0, 4) : null,
        action: widget.widget_action ?? null,
    });
    const noticeText = (notice) => (typeof notice === "string" ? notice : notice.text ?? notice.message ?? "");
    return {
        available: true, phase: debug.phase ?? null, slot: debug.slot ?? null, selection: Array.isArray(debug.selection) ? debug.selection : [],
        alerts: Array.isArray(debug.alerts) ? debug.alerts.length : 0,
        notices: (Array.isArray(debug.notices) ? debug.notices : []).map(noticeText),
        markers: Array.isArray(debug.markers_shown) ? debug.markers_shown.length : 0,
        units: replica.map((unit) => ({ unit_id: unit.unit_id, owner_slot: unit.owner_slot ?? null, content_id: unit.content_id ?? null,
            center: (entries.get(unit.unit_id) || {}).ground_screen_center ?? null, pick_radius: (entries.get(unit.unit_id) || {}).pick_radius ?? null })),
        nodes: ((scene && scene.nodes) || []).map((node) => ({ node_id: node.node_id, center: node.ground_screen_center ?? null, pick_radius: node.pick_radius ?? null })),
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
    const seenIds = new Set();
    const first = [];
    const rest = [];
    for (const unit of own) {
        if (seenIds.has(unit.content_id)) rest.push(unit);
        else { seenIds.add(unit.content_id); first.push(unit); }
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

const CLOSE_TEXT = /^(close|dismiss|ok|got it|x|×)$/i;

// The buttons that a person cannot reach: a visible button whose centre is not on the page, and a Close-like button that
// the UI record marks as not visible because its place is outside the page (the client clips what is outside: at 1024x768
// the "Close" of the forge guide has a fixed offset of 1176 px).
function buttonsOutsidePage(ui, viewport) {
    return ui.filter((widget) => widget.kind === "button" && widget.rect && widget.rect[2] > 0 && widget.rect[3] > 0 && !inside(center(widget.rect), viewport)
        && (widget.visible || CLOSE_TEXT.test((widget.text ?? "").trim())));
}

// The order buttons of the selection: visible, enabled, an action of kind "order"; the destructive ones last.
function orderButtons(ui, viewport) {
    const list = visibleButtons(ui, viewport).filter((widget) => widget.action && widget.action.kind === "order");
    return [...list.filter((w) => !DESTRUCTIVE.test(`${w.text} ${w.action.id}`)), ...list.filter((w) => DESTRUCTIVE.test(`${w.text} ${w.action.id}`))];
}

// The record that a click is judged by: id to the state of the widget. Left out: the start overlay (it leaves on the
// first click, whatever that click was); tooltips (hover feedback) unless `tooltips`; alert lines and alert panels (they
// come by themselves) when `alerts` is false.
function normalisedUi(ui, { tooltips = false, alerts = true } = {}) {
    const record = {};
    for (const widget of ui) {
        const id = String(widget.id);
        if (widget.kind === "focus_overlay" || id.startsWith("focus_overlay")) continue;
        if (!tooltips && id.endsWith("::tooltip")) continue;
        if (!alerts && /(^|\/)alert_/.test(id)) continue;
        record[id] = JSON.stringify([widget.visible, widget.enabled, widget.text ?? null, (widget.rect ?? []).map(Math.round)]);
    }
    return record;
}

const changedIds = (before, after, ignore = new Set()) => [...new Set([...Object.keys(before), ...Object.keys(after)])].filter((id) => before[id] !== after[id] && !ignore.has(id)).sort();

// The median shift, in pixels, of the unit centres that two worlds both show: a moved camera shifts all of them;
// one unit that walks does not move the median.
function medianShift(a, b) {
    const known = new Map((a.units || []).filter((unit) => unit.center).map((unit) => [unit.unit_id, unit.center]));
    const shifts = (b.units || []).filter((unit) => unit.center && known.has(unit.unit_id))
        .map((unit) => Math.hypot(unit.center[0] - known.get(unit.unit_id)[0], unit.center[1] - known.get(unit.unit_id)[1])).sort((x, y) => x - y);
    return shifts.length ? shifts[Math.floor(shifts.length / 2)] : 0;
}

// The half side of the box around an object: at least 48 px and 1.5 times its radius on screen.
const boxHalf = (radius) => Math.max(BOX_MIN_HALF, Math.round(BOX_SCALE * (radius || 0)));
const boxAround = (x, y, radius) => { const half = boxHalf(radius); return [x - half, y - half, half * 2, half * 2]; };

// A point of open ground: near the own units, away from every unit and node (2 times its radius, at least 60 px) and
// under no visible widget that covers less than half of the page. `avoid` are points already used.
function openGround(world, viewport, avoid = []) {
    const own = (world.units || []).filter((unit) => unit.owner_slot === world.slot && inside(unit.center, viewport));
    if (!own.length) return null;
    const cx = own.reduce((sum, unit) => sum + unit.center[0], 0) / own.length;
    const cy = own.reduce((sum, unit) => sum + unit.center[1], 0) / own.length;
    const things = [...(world.units || []), ...(world.nodes || [])].filter((thing) => thing.center);
    const widgets = (world.ui || []).filter((w) => w.visible && w.rect && w.kind !== "focus_overlay" && (w.rect[2] * w.rect[3]) / (viewport.width * viewport.height) < 0.5);
    const covered = (x, y) => widgets.some((w) => x >= w.rect[0] && x < w.rect[0] + w.rect[2] && y >= w.rect[1] && y < w.rect[1] + w.rect[3]);
    const free = (x, y) => inside([x, y], viewport) && !covered(x, y)
        && things.every((t) => Math.hypot(t.center[0] - x, t.center[1] - y) >= Math.max(60, 2 * (t.pick_radius || 0)))
        && avoid.every((p) => Math.hypot(p[0] - x, p[1] - y) >= 60);
    for (let ring = 1; ring <= 12; ring += 1) {
        for (let k = 0; k < 16; k += 1) {
            const angle = (k * Math.PI * 2) / 16;
            const x = Math.round(cx + Math.cos(angle) * ring * 40);
            const y = Math.round(cy + Math.sin(angle) * ring * 40);
            if (free(x, y)) return [x, y];
        }
    }
    return null;
}

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

const alertLines = (ui) => ui.filter((w) => w.visible && /(^|\/)alert_line/.test(w.id) && w.kind !== "widget" && (w.text ?? "").trim()).length;

// Look at the page for `ms`, one UI record each second, with no input. Each record: the alert lines and debug alerts.
async function watchFor(session, ms, record) {
    record.watch = [];
    let world = null;
    for (let spent = 0; spent < ms; spent += WATCH_STEP_MS) {
        await session.sleep(Math.min(WATCH_STEP_MS, ms - spent));
        world = await session.readWorld().catch(() => ({ available: false }));
        if (world.available) record.watch.push({ t_ms: session.t(), alert_lines: alertLines(world.ui), alerts: world.alerts });
    }
    return world && world.available ? world : null;
}

// The big layer of a UI record: visible, not the start overlay, more than half of the page.
function bigLayers(ui, viewport) {
    return ui.filter((widget) => widget.visible && widget.kind !== "focus_overlay" && widget.rect
        && (widget.rect[2] * widget.rect[3]) / (viewport.width * viewport.height) > 0.5);
}


// The positive control: close the layer that the first click met, by the button that the UI record shows for it.
async function closeLayer(session) {
    const viewport = session.size;
    const record = { found: false, clicks: 0, closed: false, off_page: [] };
    for (let i = 0; i < 3; i += 1) {
        const world = await session.readWorld();
        if (bigLayers(world.ui, viewport).length === 0) { record.closed = true; break; }
        const button = visibleButtons(world.ui, viewport).find((widget) => CLOSE_TEXT.test((widget.text ?? "").trim()));
        if (!button) {
            record.off_page = buttonsOutsidePage(world.ui, viewport).filter((widget) => CLOSE_TEXT.test((widget.text ?? "").trim())).map((widget) => widget.id);
            break;
        }
        record.found = true;
        const [x, y] = center(button.rect);
        await session.handle({ cmd: "click", args: [String(x), String(y), "left"] });
        record.clicks += 1;
        await session.sleep(300);
    }
    if (!record.closed) record.closed = bigLayers((await session.readWorld()).ui, viewport).length === 0;
    return record;
}

// One look before a click: two pictures 200 ms apart (the second is the picture to compare with, the pair gives the
// idle noise) and the two UI records.
async function lookBefore(session, tag, n) {
    const worldA = await session.readWorld();
    const a = await session.shot(`probe-${tag}${pad(n)}-a`, session.t());
    await session.sleep(200);
    const worldB = await session.readWorld();
    const b = await session.shot(`probe-${tag}${pad(n)}-b`, session.t());
    return { worldA, worldB, a, b, idleIds: new Set(changedIds(normalisedUi(worldA.ui), normalisedUi(worldB.ui))) };
}

// A click through the session with the picture at `offset` ms kept. Returns the path of that picture and the world after.
async function clickKeeping(session, x, y, button, offset = 200) {
    let file = null;
    await session.handle({ cmd: "click", args: [String(x), String(y), button], hook: (o, f) => { if (o === offset) file = f; } });
    return { file, world: await session.readWorld() };
}

// Was the click seen? A pixel change in the box around the object, or a change of a panel (not an alert line, not a
// tooltip, not what changes by itself) in the UI record. `tooltips` lets a tooltip count (a node shows its name so).
function seen(look, afterWorld, afterFile, point, radius, { tooltips = false } = {}) {
    const box = boxAround(point[0], point[1], radius);
    const idle = changeStats(image(look.a), image(look.b), box);
    const change = changeStats(image(look.b), image(afterFile), box);
    const before = normalisedUi(look.worldB.ui, { tooltips, alerts: false });
    const after = normalisedUi(afterWorld.ui, { tooltips, alerts: false });
    const idleIds = new Set(changedIds(normalisedUi(look.worldA.ui, { tooltips, alerts: false }), before));
    const changed = changedIds(before, after, idleIds);
    const boxSeen = change.box_changed >= Math.max(MIN_BOX_PIXELS, 3 * idle.box_changed);
    const newNotice = afterWorld.notices.filter((text) => text && !look.worldB.notices.includes(text));
    return { box_changed: change.box_changed, box_px: change.box_px, idle_box_changed: idle.box_changed, changed_ids: changed, notice: newNotice[0] ?? null,
        box_seen: boxSeen, panel_seen: changed.length > 0, visible_change: boxSeen || changed.length > 0 };
}

async function firstClick(session, options) {
    const viewport = session.size;
    const record = { kind: "first-click", control: options.control ?? null, viewport, ready_ms: null, settle_ms: null, watch: [], start: null, layer_close: null,
        buttons_outside_page: [], clicks: [], orders: [], extras: {}, summary: null };
    const ready = await waitReady(session);
    record.ready_ms = ready.waited_ms;
    record.settle_ms = options.settleMs ?? SETTLE_MS;
    if (ready.world && record.settle_ms > 0) ready.world = (await watchFor(session, record.settle_ms, record)) ?? ready.world;
    const fail = (reason) => { record.summary = { verdict: "FAIL", clicks: 0, failed: 0, first_failure: null, reason, orders: { verdict: "SKIPPED" }, extras: { verdict: "SKIPPED" } }; return record; };
    if (!ready.world) return fail(`the page was not ready within ${READY_TIMEOUT_MS / 1000} s`);
    record.buttons_outside_page = buttonsOutsidePage(ready.world.ui, viewport).map((widget) => widget.id);
    // The start click: the start control of the UI record, else the page centre.
    const overlay = ready.world.ui.find((widget) => widget.kind === "focus_overlay" && widget.visible);
    const node = overlay && overlay.text_nodes && overlay.text_nodes[0] && overlay.text_nodes[0].rect;
    const point = node ? center(node) : [Math.floor(viewport.width / 2), Math.floor(viewport.height / 2)];
    record.start = { found: overlay ? "focus_overlay" : "viewport-centre", point };
    await session.handle({ cmd: "click", args: [String(point[0]), String(point[1]), "left"] });
    await session.sleep(300);
    if (options.control === "close-layer") record.layer_close = await closeLayer(session);
    const world = await session.readWorld();
    record.buttons_outside_page = [...new Set([...record.buttons_outside_page, ...buttonsOutsidePage(world.ui, viewport).map((widget) => widget.id)])];
    const units = ownUnitsOnScreen(world, viewport);
    if (units.length === 0) return fail("no own unit on screen after the start click: nothing to click");

    // P1: one click on each own unit.
    let number = 0;
    const selected = new Map(); // content id -> the first unit that a click selected and showed
    for (const unit of units) {
        number += 1;
        const fresh = (await session.readWorld()).units.find((entry) => entry.unit_id === unit.unit_id);
        if (!fresh || !inside(fresh.center, viewport)) { record.clicks.push({ n: number, unit_id: unit.unit_id, content_id: unit.content_id, skipped: "not on screen at the click", ok: true }); continue; }
        const [x, y] = fresh.center.map(Math.round);
        const look = await lookBefore(session, "", number);
        const { file, world: after } = await clickKeeping(session, x, y, "left");
        const result = seen(look, after, file, [x, y], fresh.pick_radius);
        const selectionOk = after.selection.includes(unit.unit_id);
        if (selectionOk && result.visible_change && !selected.has(unit.content_id)) selected.set(unit.content_id, unit);
        record.clicks.push({ n: number, unit_id: unit.unit_id, content_id: unit.content_id, point: [x, y], selection: after.selection, selection_ok: selectionOk, ...result,
            ok: selectionOk && result.visible_change });
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
        orders: { verdict: "SKIPPED", reason: "no unit was selected" }, extras: { verdict: "SKIPPED", reason: "no unit was selected" },
    };
    if (selected.size === 0) return record;

    // P3: the order buttons of each selection. P4: an enemy, a node, open ground, a right click.
    await orderPass(session, record, [...selected.values()]);
    await extraClicks(session, record, [...selected.values()][0]);
    return record;
}

async function selectUnit(session, unit) {
    const fresh = (await session.readWorld()).units.find((entry) => entry.unit_id === unit.unit_id);
    if (!fresh || !inside(fresh.center, session.size)) return null;
    const [x, y] = fresh.center.map(Math.round);
    const { world } = await clickKeeping(session, x, y, "left");
    return world.selection.includes(unit.unit_id) ? { x, y, radius: fresh.pick_radius, world } : null;
}

async function orderPass(session, record, units) {
    const viewport = session.size;
    let counter = 0;
    for (const unit of units) {
        let mark = await selectUnit(session, unit);
        if (!mark) continue;
        await session.sleep(RESULT_MS);
        const list = orderButtons((await session.readWorld()).ui, viewport).slice(0, ORDER_CAP);
        for (const listed of list) {
            counter += 1;
            if (!(await session.readWorld()).selection.includes(unit.unit_id)) { mark = await selectUnit(session, unit); if (!mark) break; await session.sleep(RESULT_MS); }
            const world = await session.readWorld();
            const button = orderButtons(world.ui, viewport).find((widget) => widget.id === listed.id);
            if (!button) { record.orders.push({ unit_id: unit.unit_id, content_id: unit.content_id, id: listed.id, text: listed.text, skipped: "not visible at its turn", acted: true }); continue; }
            const [bx, by] = center(button.rect);
            // The pointer rests on the button, then the pictures before the click, then the click; the result is read 1 s later.
            await session.handle({ cmd: "move", args: [String(bx), String(by)] });
            await session.sleep(300);
            const look = await lookBefore(session, "ord-", counter);
            const { file, world: after } = await clickKeeping(session, bx, by, "left", RESULT_MS);
            const result = seen(look, after, file, [mark.x, mark.y], mark.radius);
            record.orders.push({ unit_id: unit.unit_id, content_id: unit.content_id, id: button.id, text: button.text, order: button.action.id, acted: result.visible_change || result.notice !== null,
                changed_ids: result.changed_ids, notice: result.notice, box_changed: result.box_changed });
        }
    }
    const lost = record.orders.filter((order) => !order.acted);
    const tried = record.orders.filter((order) => !order.skipped);
    record.summary.orders = {
        verdict: tried.length > 0 && lost.length === 0 ? "PASS" : "FAIL", buttons: tried.length, failed: lost.length,
        reason: tried.length === 0 ? "no order button was visible after any selection: the orders may be hotkeys only"
            : lost.length ? `no visible result within 1 s for ${lost.map((order) => `${order.text} (${order.content_id})`).join(", ")}`
                : `${tried.length} of ${tried.length} order buttons showed a result`,
    };
}

async function extraClicks(session, record, firstUnit) {
    const viewport = session.size;
    const results = {};
    const used = [];
    const world0 = await session.readWorld();
    // 1: an enemy unit: its panel (name or owner) shows.
    const enemy = (world0.units || []).find((unit) => unit.owner_slot !== null && unit.owner_slot !== world0.slot && inside(unit.center, viewport));
    if (enemy) {
        const [x, y] = enemy.center.map(Math.round);
        const look = await lookBefore(session, "ext-", 1);
        const { file, world } = await clickKeeping(session, x, y, "left", RESULT_MS);
        const result = seen(look, world, file, [x, y], enemy.pick_radius);
        results.enemy = { unit_id: enemy.unit_id, content_id: enemy.content_id, point: [x, y], selection: world.selection, ok: result.visible_change, ...result };
    } else results.enemy = { skipped: "no enemy unit on screen", ok: true };
    // 2: a resource node: its name shows as a panel or a tooltip.
    const nodeNow = ((await session.readWorld()).nodes || []).find((entry) => inside(entry.center, viewport));
    if (nodeNow) {
        const [x, y] = nodeNow.center.map(Math.round);
        await session.handle({ cmd: "move", args: [String(x), String(y)] });
        await session.sleep(500);
        const look = await lookBefore(session, "ext-", 2);
        const { file, world } = await clickKeeping(session, x, y, "left", RESULT_MS);
        const result = seen(look, world, file, [x, y], nodeNow.pick_radius, { tooltips: true });
        results.node = { node_id: nodeNow.node_id, point: [x, y], ok: result.visible_change, ...result };
    } else results.node = { skipped: "no resource node on screen", ok: true };
    // 3: open ground after a selection: the selection must clear.
    let ground = openGround(await session.readWorld(), viewport, used);
    let mark = await selectUnit(session, firstUnit);
    if (ground && mark) {
        used.push(ground);
        const after = (await clickKeeping(session, ground[0], ground[1], "left", RESULT_MS)).world;
        results.ground_clear = { point: ground, selection_before: mark.world.selection, selection_after: after.selection, ok: after.selection.length === 0 };
    } else results.ground_clear = { ok: false, reason: ground ? "the own unit could not be selected" : "no open ground found" };
    // 4: a right click on open ground with a unit selected: a notice, an order mark or a pixel mark within 1 s.
    mark = await selectUnit(session, firstUnit);
    ground = openGround(await session.readWorld(), viewport, used);
    if (ground && mark) {
        const look = await lookBefore(session, "ext-", 4);
        const { file, world } = await clickKeeping(session, ground[0], ground[1], "right", RESULT_MS);
        const result = seen(look, world, file, ground, 0);
        const markShown = world.markers > look.worldB.markers;
        results.right_click = { point: ground, ok: result.notice !== null || markShown || result.box_seen, notice: result.notice, marker_added: markShown, box_changed: result.box_changed };
    } else results.right_click = { ok: false, reason: ground ? "the own unit could not be selected" : "no open ground found" };
    record.extras = results;
    const bad = Object.entries(results).filter(([, value]) => !value.ok).map(([name]) => name);
    record.summary.extras = {
        verdict: bad.length === 0 ? "PASS" : "FAIL", failed: bad,
        reason: bad.length ? `no visible response: ${bad.join(", ")}` : `all ${Object.keys(results).length} extra clicks answered (${Object.entries(results).filter(([, v]) => v.skipped).map(([k]) => `${k} skipped`).join(", ") || "none skipped"})`,
    };
}

async function buttons(session, options) {
    const viewport = session.size;
    const record = { kind: "buttons", viewport, ready_ms: null, settle_ms: null, watch: [], idle_changed_ids: [], start_overlay_buttons: [], f8_start: null, buttons: [], summary: null };
    const ready = await waitReady(session);
    record.ready_ms = ready.waited_ms;
    record.settle_ms = options.settleMs ?? BUTTONS_SETTLE_MS;
    if (ready.world && record.settle_ms > 0) ready.world = (await watchFor(session, record.settle_ms, record)) ?? ready.world;
    if (!ready.world) {
        record.summary = { verdict: "FAIL", failed: 0, reason: `the page was not ready within ${READY_TIMEOUT_MS / 1000} s`, start: { verdict: "SKIPPED" } };
        return record;
    }
    // One button, one try: the pointer rests, the pictures, the click, then what changed.
    const idleSet = new Set();
    const tryButton = async (button, n, tag) => {
        const [x, y] = center(button.rect);
        await session.handle({ cmd: "move", args: [String(x), String(y)] });
        await session.sleep(300);
        const look = await lookBefore(session, tag, n);
        const before = look.worldB;
        const { file, world: after } = await clickKeeping(session, x, y, "left");
        const overlayLeft = before.ui.some((w) => w.kind === "focus_overlay" && w.visible) && !after.ui.some((w) => w.kind === "focus_overlay" && w.visible);
        const changed = changedIds(normalisedUi(before.ui), normalisedUi(after.ui), new Set([...idleSet, ...look.idleIds]));
        const box = boxAround(x, y, Math.max(button.rect[2], button.rect[3]) / 2);
        const actedBox = !overlayLeft && changeStats(image(look.b), image(file), box).box_changed >= Math.max(MIN_BOX_PIXELS, 3 * changeStats(image(look.a), image(look.b), box).box_changed);
        const cameraMoved = medianShift(before, after) > 3;
        return { n, id: button.id, text: button.text, point: [x, y], acted: changed.length > 0 || actedBox || cameraMoved, acted_ui: changed.length > 0, acted_box: actedBox,
            camera_moved: cameraMoved, overlay_left: overlayLeft, changed_ids: changed };
    };
    // What changes by itself in 200 ms, with no input: it is not the effect of a click.
    const first = normalisedUi(ready.world.ui);
    await session.sleep(200);
    changedIds(first, normalisedUi((await session.readWorld()).ui)).forEach((id) => idleSet.add(id));
    record.idle_changed_ids = [...idleSet].sort();
    // F8-start: a button that shows under the start overlay gets one click before any start click.
    const overlay = ready.world.ui.find((widget) => widget.kind === "focus_overlay" && widget.visible);
    const under = visibleButtons(ready.world.ui, viewport);
    record.start_overlay_buttons = overlay ? under.map((widget) => widget.id) : [];
    if (overlay && under.length) {
        record.f8_start = await tryButton(under[0], 0, "s-");
        record.summary = { start: { verdict: record.f8_start.acted ? "PASS" : "FAIL", button: under[0].id,
            reason: record.f8_start.acted ? `${under[0].id} acted on its first click` : `the first click on ${under[0].id} changed nothing: the start overlay took it` } };
    } else record.summary = { start: { verdict: "PASS", reason: overlay ? "no button under the start overlay" : "no start overlay" } };
    // The start click if the overlay is still there, then F8 for each visible button.
    let world = await session.readWorld();
    const stillThere = world.ui.find((widget) => widget.kind === "focus_overlay" && widget.visible);
    if (stillThere) {
        const node = stillThere.text_nodes && stillThere.text_nodes[0] && stillThere.text_nodes[0].rect;
        const [sx, sy] = node ? center(node) : [Math.floor(viewport.width / 2), Math.floor(viewport.height / 2)];
        await session.handle({ cmd: "click", args: [String(sx), String(sy), "left"] });
    }
    await session.sleep(1000);
    world = await session.readWorld();
    const list = visibleButtons(world.ui, viewport).slice(0, BUTTON_CAP);
    let number = 0;
    for (const listed of list) {
        number += 1;
        const fresh = await session.readWorld();
        const button = fresh.ui.find((widget) => widget.id === listed.id);
        if (!button || !visibleButtons([button], viewport).length) { record.buttons.push({ n: number, id: listed.id, skipped: "not visible at its turn", acted: true }); continue; }
        record.buttons.push(await tryButton(button, number, ""));
    }
    const lost = record.buttons.filter((button) => !button.acted);
    Object.assign(record.summary, {
        verdict: lost.length === 0 ? "PASS" : "FAIL", buttons: record.buttons.length, failed: lost.length,
        reason: record.buttons.length === 0 ? "no visible button after the start click: nothing to click"
            : lost.length ? `the first click on ${lost.map((button) => button.id).join(", ")} changed nothing` : `${record.buttons.length} of ${record.buttons.length} buttons acted on the first click`,
    });
    return record;
}

async function runProbe(session, kind, options = {}) {
    if (!["first-click", "buttons"].includes(kind)) throw new Error(`unknown probe ${kind}`);
    const record = kind === "first-click" ? await firstClick(session, options) : await buttons(session, options);
    fs.writeFileSync(path.join(session.dir, `probe-${kind}.json`), JSON.stringify(record, null, 2) + "\n");
    return record;
}

module.exports = { SETTLE_MS, BUTTONS_SETTLE_MS, runProbe, readWorld, wordCount, ownUnitsOnScreen, visibleButtons, buttonsOutsidePage, orderButtons, normalisedUi, bigLayers,
    center, medianShift, boxHalf, openGround, changedIds };
