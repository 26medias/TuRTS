// turts-test-role: oracle
"use strict";

// M3 F10 oracle (MC design 13.3; replaces the asserts of tests/browser/m3.js). It reads the
// harness checkpoints of the screenshots that m3-driver.js saved, the server tick log and the
// trusted economy log (`--economy-log`, scripts/m3-browser.sh).
const assert = require("node:assert/strict");
const { checkpointForSave, readCheckpoints, replicaUnit, unitTrack, widget } = require("./checkpoint-lib.js");

const TURRET = { x: 17, y: 12, range: 8 };

function appliedAfter(debug, seq) {
    return (debug?.command_results ?? []).filter((result) => result.seq > seq && Number.isInteger(result.applied_tick));
}

function maxSeq(debug) {
    return Math.max(0, ...(debug?.command_results ?? []).map((result) => result.seq).filter(Number.isInteger));
}

function selectedOwn(checkpoint, contentId, label) {
    const selection = checkpoint.debug.selection;
    assert(Array.isArray(selection) && selection.length === 1, `${label}: the click must select exactly one unit`);
    const unit = replicaUnit(checkpoint.debug, selection[0]);
    assert(unit && unit.content_id === contentId && unit.owner_slot === checkpoint.debug.slot,
        `${label}: the selected unit must be the own ${contentId}, saw ${JSON.stringify(unit)}`);
    return unit;
}

function inventoryLines(debug) {
    const row = widget(debug, (item) => item.id === "info_inventory");
    assert(row && typeof row.text === "string", "actual rendered press panel shows no inventory text");
    return row.text.replace(/^Inventory: /, "").split("\n");
}

function portHasCapacity(lines, port, capacity) {
    return lines.some((line) => line.startsWith(`${port}: `) && line.endsWith(`(capacity ${capacity})`));
}

function labQueue(debug, labId) {
    return debug?.replicated_world?.economy?.queues?.find((row) => row.producer === labId) ?? null;
}

function resourceTotals(state) {
    const totals = new Map();
    const add = (resource, amount) => {
        const key = `${resource?.package}:${resource?.local}`;
        totals.set(key, (totals.get(key) ?? 0) + amount);
    };
    const economy = state?.economy?.economy;
    for (const balance of economy?.balances ?? []) add(balance.resource, balance.on_hand);
    for (const inventory of economy?.inventories ?? []) for (const item of inventory.items ?? []) add(item.resource, item.on_hand);
    return totals;
}

// The checkpoints of the series `<label>-NN` that the driver took after one action, in order.
function seriesOf(input, checkpoints, label) {
    return input.filter((entry) => entry.call === "save" && new RegExp(`^${label}-\\d{2}\\.png$`).test(entry.args?.[0] ?? ""))
        .map((entry) => checkpointForSave(input, checkpoints, entry.args[0], "M3"));
}

// The first checkpoint of a series that passes, within the bound of the series (m3.js waited
// up to 10 s for each result).
function firstPassing(input, checkpoints, label, predicate, message) {
    const shots = seriesOf(input, checkpoints, label);
    assert(shots.length > 0, `${message} (no screenshot after the action)`);
    const found = shots.find(predicate);
    assert(found, message);
    return found;
}

// The click after `<label>-card.png` must land on the card row of `orderId` (ui_snapshot).
function assertCardClick(input, checkpoints, label, orderId) {
    const card = checkpointForSave(input, checkpoints, `${label}-card.png`, "M3");
    const clickIndex = input.findIndex((entry, index) => index > card.screenshotIndex && entry.call === "click");
    assert(clickIndex > 0, `${label}: no click on the command card`);
    const [x, y] = input[clickIndex].args;
    const scale = card.scene.scale_factor || 1;
    const row = widget(card.debug, (item) => item.kind === "button" && item.id.startsWith("command_rows/") && Array.isArray(item.rect) &&
        x * scale >= item.rect[0] && x * scale <= item.rect[0] + item.rect[2] && y * scale >= item.rect[1] && y * scale <= item.rect[1] + item.rect[3]);
    assert(row?.widget_action?.kind === "order" && row.widget_action.id === orderId, `${label}: the click is not on the ${orderId} button of the forge command card`);
    return card;
}

// The conservation checks on the trusted economy log, streamed (the M3 log reaches 1.5 GB: run
// t21-run118-m3-2). The checks: the initial record comes first; the ticks are continuous and each
// tick starts from the state that the last tick ended with (m2-audit.js validateAuditRecords);
// no amount is negative; and a resource total changes only in a tick that logged at least one
// operation (EL1: each state write goes through capture_operation). The M3 machines write
// operations with no economy event (tick 15 of that run: ore moves, events []), so "a change
// needs an event" was wrong. Operation lines are counted, not parsed. A torn last line is cut.
async function assertConservationStream(lines) {
    let first = true;
    let previous = null;
    let previousTick = null;
    let operations = 0;
    let ticks = 0;
    let pendingError = null;
    for await (const line of lines) {
        if (pendingError) throw pendingError;
        if (!line.trim()) continue;
        if (line.startsWith('{"record":"operation"')) { operations += 1; continue; }
        let record;
        try { record = JSON.parse(line); }
        catch (error) { pendingError = new Error(`invalid economy audit line: ${error.message}`); continue; }
        if (first) {
            assert.equal(record.record, "initial", "conservation: the first economy audit record must be initial");
            previous = JSON.stringify(record.data.economy);
            previousTick = record.data.economy?.tick;
            first = false;
            continue;
        }
        if (record.record !== "tick") continue;
        const { tick, before, state } = record.data;
        assert(tick === previousTick + 1, `conservation: economy audit tick ${tick} is discontinuous after ${previousTick}`);
        assert(JSON.stringify(before.economy) === previous, `conservation: economy audit tick ${tick} before-state differs from the previous completed tick`);
        const totalsBefore = resourceTotals(before);
        const totalsAfter = resourceTotals(state);
        for (const [resource, amount] of totalsAfter) {
            assert(Number.isInteger(amount) && amount >= 0, `conservation: ${resource} is ${amount} at tick ${tick}`);
        }
        const changed = [...new Set([...totalsBefore.keys(), ...totalsAfter.keys()])].filter((key) => (totalsBefore.get(key) ?? 0) !== (totalsAfter.get(key) ?? 0));
        assert(changed.length === 0 || operations > 0, `conservation: ${changed.join(", ")} changed at tick ${tick} with no logged operation`);
        previous = JSON.stringify(state.economy);
        previousTick = tick;
        operations = 0;
        ticks += 1;
    }
    assert(ticks > 0, "conservation: the economy log has no completed tick");
    return ticks;
}

async function judge(view) {
    const input = view.inputLog();
    const checkpoints = await readCheckpoints(view);
    const at = (file) => checkpointForSave(input, checkpoints, `${file}.png`, "M3");

    const joined = at("m3-01-joined");
    assert(joined.debug.phase?.startsWith("Joined"), "forge joined in a real WebGPU browser context");
    assert.equal(joined.scene.cameras_3d, 1, "forge joined in a real WebGPU browser context");
    assert((joined.debug.units ?? []).some((unit) => unit.content_id.startsWith("forge:")), "forge joined in a real WebGPU browser context");

    firstPassing(input, checkpoints, "m3-02-press", (shot) => {
        try {
            const unit = selectedOwn(shot, "forge:press", "press");
            const lines = inventoryLines(shot.debug);
            return unit && portHasCapacity(lines, "Main", 6) && portHasCapacity(lines, "Output", 4);
        } catch { return false; }
    }, "actual rendered press panel shows Main and Output inventory capacity");

    const ready = at("m3-03-lab-research-ready");
    const lab = selectedOwn(ready, "forge:lab", "owner research station is absent from actual rendered state");
    const inventory = ready.debug.replicated_world.economy.inventories.find((row) => row.unit === lab.unit_id && row.port === "main");
    const shells = inventory?.items?.find((item) => item.resource === "forge:shell");
    const control = widget(ready.debug, (row) => row.id.startsWith("research_entries/") && row.enabled === true &&
        row.widget_action?.content_id === "forge:hardened_shells");
    assert(shells && shells.on_hand - shells.reserved >= 10 && control, "replica inventory and visible enabled research control");

    firstPassing(input, checkpoints, "m3-04-research", (shot) => appliedAfter(shot.debug, maxSeq(ready.debug)).length >= 1 &&
        labQueue(shot.debug, lab.unit_id)?.jobs?.some((job) => job.recipe === "forge:hardened_shells"),
    "research command Acked, Applied and present in replicated queue");

    const disableCard = assertCardClick(input, checkpoints, "m3-05-disable", "forge:order_disable");
    firstPassing(input, checkpoints, "m3-05-disabled", (shot) => appliedAfter(shot.debug, maxSeq(disableCard.debug)).length >= 1 &&
        labQueue(shot.debug, lab.unit_id)?.enabled === false, "SetEnabled control Acked and Applied with disabled replica state");
    const enableCard = assertCardClick(input, checkpoints, "m3-06-enable", "forge:order_enable");
    firstPassing(input, checkpoints, "m3-06-enabled", (shot) => appliedAfter(shot.debug, maxSeq(enableCard.debug)).length >= 1 &&
        labQueue(shot.debug, lab.unit_id)?.enabled === true, "SetEnabled(true) Applied with the enabled replica state");

    const beltShot = firstPassing(input, checkpoints, "m3-07-belt", (shot) => shot.debug.selection?.length === 1 &&
        (shot.debug.replicated_world.belts ?? []).some((belt) => belt.piece === shot.debug.selection[0]), "belt piece selected from rendered scene");
    const piece = beltShot.debug.selection[0];
    assert(beltShot.scene.units.some((unit) => unit.unit_id === piece), "belt piece has no actual rendered mesh");
    const removeCard = assertCardClick(input, checkpoints, "m3-08-remove", "forge:order_remove");
    firstPassing(input, checkpoints, "m3-08-removed", (shot) => appliedAfter(shot.debug, maxSeq(removeCard.debug)).length >= 1 &&
        !(shot.debug.replicated_world.units ?? []).some((unit) => unit.unit_id === piece),
    "Remove control Acked and Applied with belt removed from replica");

    const droneShot = firstPassing(input, checkpoints, "m3-10-drone", (shot) => {
        try { return shot.window === 2 && selectedOwn(shot, "forge:drone", "drone"); } catch { return false; }
    }, "the drone window must select its own drone");
    const drone = selectedOwn(droneShot, "forge:drone", "the drone window must select its own drone");
    firstPassing(input, checkpoints, "m3-11-drone-order", (shot) => appliedAfter(shot.debug, maxSeq(droneShot.debug)).length >= 1,
        "the drone move into range must be applied");
    const orderTick = droneShot.debug.replicated_world.tick;
    const track = unitTrack(await view.serverFile("server-tick.log"), drone.unit_id).filter((point) => point.tick > orderTick);
    assert(track.some((point) => Math.hypot(point.x - TURRET.x, point.y - TURRET.y) <= TURRET.range),
        `the drone must move into turret range ${TURRET.range} after tick ${orderTick} (server tick log)`);

    await assertConservationStream(view.serverLines("economy.jsonl"));
}

module.exports = { judge, assertConservationStream, firstPassing, portHasCapacity };
