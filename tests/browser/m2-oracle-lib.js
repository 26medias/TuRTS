// turts-test-role: oracle
"use strict";

// Oracle helpers of the M2 browser cases (MC design 13.3). The legacy file tests/browser/m2.js
// held these functions together with the code that drove the browser; this file keeps only the
// functions that read captured bridge snapshots, the harness captures and the server files.
// The pure screen-geometry helpers live in harness/png.js.
const assert = require("node:assert/strict");
const { finitePair, projectGroundPoint } = require("./harness/png.js");
const {
    assertOwnerWorldMatchesAudit,
    auditStateAt,
    projectAuditOwnerWorld,
    validateAuditRecords,
    worldChangeMatchesAudit,
} = require("./m2-audit.js");

function projectRenderedEntity(scene, entry) {
    assert(entry && finitePair(entry.ground_screen_center), "rendered entity has no in-viewport ground center");
    assert(Number.isFinite(entry.pick_radius) && entry.pick_radius > 0, "rendered entity has no declared pick radius");
    const translation = entry.translation;
    assert(Array.isArray(translation) && translation.length === 3 && translation.every(Number.isFinite));
    const projected = projectGroundPoint(scene, [translation[0], translation[2]]);
    assert(projected, "the rendered entity ground position must project inside the active viewport");
    const distance = Math.hypot(projected[0] - entry.ground_screen_center[0], projected[1] - entry.ground_screen_center[1]);
    assert(distance <= 1.25, `Rust camera center differs from JS projection by ${distance.toFixed(3)} logical pixels`);
    return entry.ground_screen_center;
}

function uiActionWindowPoint(scene, rect) {
    const [x, y, width, height] = rect ?? [];
    const [viewportX, viewportY, viewportWidth, viewportHeight] = scene?.viewport_physical_rect ?? [];
    const scale = scene?.scale_factor;
    if (
        ![x, y, width, height, viewportX, viewportY, viewportWidth, viewportHeight, scale].every(Number.isFinite) ||
        width <= 0 || height <= 0 || scale <= 0 || x < 0 || y < 0 ||
        x + width > viewportWidth || y + height > viewportHeight
    ) {
        return null;
    }
    return [
        (viewportX + x + width / 2) / scale,
        (viewportY + y + height / 2) / scale,
    ];
}

function findVisibleUiAction(rows, type, predicate = () => true) {
    const normalizedType = String(type).toLowerCase();
    return rows.find((row) =>
        row.kind === "text" && row.visible === true && row.enabled === true &&
        row.action?.type === normalizedType && predicate(row.action, row.text, row)
    );
}

function findPressedUiAction(rows, type, predicate = () => true) {
    return rows.find((row) =>
        row.button_pressed === true && row.action?.type === String(type).toLowerCase() &&
        findVisibleUiAction([row], type, predicate) === row
    );
}

function findVisibleUiText(rows, predicate) {
    return rows.find((row) => row.kind === "text" && row.visible === true && predicate(row.text, row));
}

function hasSingleDebugSelection(snapshot, unitId) {
    const selected = snapshot?.debug?.selection;
    return Array.isArray(selected) && selected.length === 1 && selected[0] === unitId;
}

// MC adds `unit_orders` to the replicated world and `max_health` to each unit (design 9.1 and
// 9.2). The M2 key list stays exact otherwise, so a new private field still turns this red.
const PUBLIC_WORLD_KEYS = ["belts", "economy", "extractors", "health_trace", "healths", "inserters", "nodes", "research", "tick", "unit_orders", "units", "weapons"];
const PUBLIC_UNIT_KEYS = ["content_id", "facing", "max_health", "owner_slot", "pos", "unit_id"];

function assertPublicProjectionSchema(snapshot) {
    const view = snapshot.debug?.replicated_world;
    assert(view && Number.isInteger(view.tick), "the coherent public replicated-world view is missing");
    const worldKeys = Object.keys(view).sort();
    // The saved Task 16 fixture predates MC and has neither MC field.
    const legacyWorld = !worldKeys.includes("unit_orders");
    assert.deepEqual(worldKeys, legacyWorld ? PUBLIC_WORLD_KEYS.filter((key) => key !== "unit_orders") : PUBLIC_WORLD_KEYS,
        "browser replicated-world debug must contain the permitted M2/M3 client state only");
    assert.deepEqual(Object.keys(view.economy).sort(), ["balances", "inventories", "orders", "queues"],
        "browser economy debug must expose the complete permitted owner projection");
    for (const balance of view.economy.balances) assert.deepEqual(Object.keys(balance).sort(), ["on_hand", "reserved", "resource"]);
    for (const inventory of view.economy.inventories) assert.deepEqual(Object.keys(inventory).sort(), ["items", "port", "unit"]);
    for (const order of view.economy.orders) assert.deepEqual(Object.keys(order).sort(), ["node", "receiver", "stage", "unit"]);
    for (const queue of view.economy.queues) assert.deepEqual(Object.keys(queue).sort(), ["enabled", "jobs", "producer"]);
    for (const unit of view.units) {
        assert.deepEqual(Object.keys(unit).sort(), legacyWorld ? PUBLIC_UNIT_KEYS.filter((key) => key !== "max_health") : PUBLIC_UNIT_KEYS);
    }
    for (const node of view.nodes) {
        assert.deepEqual(Object.keys(node).sort(), ["content_id", "id", "pos", "remaining"]);
    }
    assert(!Object.hasOwn(snapshot.debug, "raw_world_dtos"));
    assert(!Object.hasOwn(snapshot.debug, "foreign_economy"));
    // MC design 5.1 (design lines 477-478): order state goes to its owner only, as the server
    // filters it (crates/turts_server/src/export.rs:96-100 snapshot, :190-194 delta).
    for (const order of view.unit_orders ?? []) {
        const unit = view.units.find((item) => item.unit_id === order.unit);
        assert(unit && unit.owner_slot === snapshot.debug.slot,
            `browser replica holds the order state of unit ${order.unit}, which slot ${snapshot.debug.slot} does not own`);
    }
    return view;
}

function publicProjection(snapshot) {
    const view = assertPublicProjectionSchema(snapshot);
    return {
        units: view.units.map(({ unit_id, content_id, owner_slot, pos }) => ({ unit_id, content_id, owner_slot, pos }))
            .sort((a, b) => a.unit_id - b.unit_id),
        nodes: view.nodes.map((node) => ({ id: node.id, content_id: node.content_id, pos: node.pos, remaining: node.remaining }))
            .sort((a, b) => a.id - b.id),
    };
}

// The text of a server JSONL file up to its last complete line. The server still runs while the
// oracle reads, so a torn last line is cut, never parsed.
function parseCompleteAuditText(text) {
    assert.equal(typeof text, "string", "economy audit log must be text");
    const end = text.lastIndexOf("\n");
    assert(end >= 0, "economy audit log has no complete record");
    const lines = text.slice(0, end).split("\n");
    assert(!lines.some((line) => line.trim() === ""), "economy audit JSONL contains an empty completed line");
    return validateAuditRecords(lines.map((line, index) => {
        try { return JSON.parse(line); } catch (error) { throw new Error(`invalid complete economy audit line ${index + 1}: ${error.message}`); }
    }));
}

// The complete records of a server JSONL audit log, streamed (view.serverLines). Operation
// records are skipped unparsed: validateAuditRecords and auditStateAt read only the initial and
// tick records, and the operations are most of the bytes (an M3 log reached 1.5 GB in run
// t21-run118-m3-2). A torn last line is cut; a torn line before another is an error.
async function readAuditRecords(lines) {
    const records = [];
    let pendingError = null;
    for await (const line of lines) {
        if (pendingError) throw pendingError;
        if (!line.trim() || line.startsWith('{"record":"operation"')) continue;
        try { records.push(JSON.parse(line)); }
        catch (error) { pendingError = new Error(`invalid complete economy audit line ${records.length + 1}: ${error.message}`); }
    }
    return validateAuditRecords(records);
}

function lastAuditTick(records) {
    return records.filter((entry) => entry.record === "tick").reduce((latest, entry) => Math.max(latest, entry.data.tick),
        records[0]?.data?.economy?.tick ?? -1);
}

function lossTransitionAt(kind, drop, records, slot = 0) {
    assert(Number.isInteger(drop?.dropped_tick) && Number.isInteger(drop?.arm_tick), `${kind} drop lacks real arm/drop tick evidence`);
    assert(drop.dropped_tick > drop.arm_tick, `${kind} drop tick must follow its live arm tick`);
    assert(lastAuditTick(records) >= drop.dropped_tick,
        `trusted economy audit did not complete tick ${drop.dropped_tick}; last complete tick=${lastAuditTick(records)}`);
    const before = auditStateAt(records, drop.dropped_tick - 1);
    const after = auditStateAt(records, drop.dropped_tick);
    assert(worldChangeMatchesAudit(kind, before, after, slot),
        `${kind} typed browser loss at tick ${drop.dropped_tick} lacks the matching authoritative owner transition`);
    return { before, after };
}

function compareBrowserSnapshotToAudit(snapshot, records, slot, label) {
    assertPublicProjectionSchema(snapshot);
    const tick = snapshot.debug.replicated_world.tick;
    assert(lastAuditTick(records) >= tick,
        `trusted economy audit did not complete tick ${tick}; last complete tick=${lastAuditTick(records)}`);
    const authoritative = auditStateAt(records, tick);
    // The audit projection has no MC order field; compare the M2 owner world only.
    const { unit_orders: _orders, ...world } = snapshot.debug.replicated_world;
    const units = world.units.map(({ max_health: _maxHealth, ...unit }) => unit);
    assertOwnerWorldMatchesAudit({ ...world, units }, authoritative, slot);
    return {
        label,
        slot,
        tick,
        expected_owner_world: projectAuditOwnerWorld(authoritative, slot),
        actual_browser_world: JSON.parse(JSON.stringify(snapshot.debug.replicated_world)),
        passed: true,
    };
}

function assertFinalChangeQuiescence(records, droppedTick, browserTick, observationTick, slot) {
    assert(browserTick >= droppedTick + 8,
        `terminal recovery applied tick ${browserTick} must be at least eight authoritative ticks after final change ${droppedTick}`);
    assert(observationTick >= browserTick,
        `bounded final audit observation tick ${observationTick} must reach the compared browser tick ${browserTick}`);
    const projection = (tick) => {
        const world = projectAuditOwnerWorld(auditStateAt(records, tick), slot);
        const { tick: _tick, ...withoutTick } = world;
        return withoutTick;
    };
    const finalState = projection(droppedTick);
    for (let tick = droppedTick + 1; tick <= observationTick; tick += 1) {
        assert.deepStrictEqual(projection(tick), finalState,
            `owner economy, unit/node lifecycle, queues and orders changed after terminal final_change at audit tick ${tick}`);
        const record = records.find((entry) => entry.record === "tick" && entry.data.tick === tick);
        assert(record, `authoritative quiet interval is missing completed tick ${tick}`);
        assert.equal(record.data.events.length, 0, `terminal final_change interval had an economy event at tick ${tick}`);
    }
    return { browser_applied_tick: browserTick, audit_observation_tick: observationTick,
        first_quiet_tick: droppedTick + 1, last_quiet_tick: observationTick, quiet_tick_count: observationTick - droppedTick };
}

// Count the client order envelopes that the server applied. The M2 engine panel line
// "Last command: applied seq N" has no MC source; the debug command results replace it.
function appliedCommandCount(snapshot) {
    return (snapshot?.debug?.command_results ?? []).filter((result) => Number.isInteger(result.applied_tick)).length;
}

module.exports = {
    appliedCommandCount,
    assertFinalChangeQuiescence,
    assertPublicProjectionSchema,
    compareBrowserSnapshotToAudit,
    findPressedUiAction,
    findVisibleUiAction,
    findVisibleUiText,
    hasSingleDebugSelection,
    lastAuditTick,
    lossTransitionAt,
    parseCompleteAuditText,
    readAuditRecords,
    projectGroundPoint,
    projectRenderedEntity,
    publicProjection,
    uiActionWindowPoint,
};
