// turts-test-role: oracle
"use strict";

// M3 F10 oracle (MC design 13.3; replaces the asserts of tests/browser/m3.js). It reads the
// harness checkpoints of the screenshots that m3-driver.js saved, the server tick log and the
// trusted economy log (`--economy-log`, scripts/m3-browser.sh).
const assert = require("node:assert/strict");
const { checkpointForSave, readCheckpoints, replicaUnit, unitTrack, widget } = require("./checkpoint-lib.js");
const { parseCompleteAuditText } = require("./m2-oracle-lib.js");

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

// A weak conservation check on the trusted log: the ticks are continuous (validateAuditRecords),
// no amount is negative, and an amount changes only in a tick that records an economy event.
function assertConservation(records) {
    let ticks = 0;
    for (const record of records) {
        if (record.record !== "tick") continue;
        ticks += 1;
        const before = resourceTotals(record.data.before);
        const after = resourceTotals(record.data.state);
        for (const [resource, amount] of after) {
            assert(Number.isInteger(amount) && amount >= 0, `conservation: ${resource} is ${amount} at tick ${record.data.tick}`);
        }
        const changed = [...new Set([...before.keys(), ...after.keys()])].filter((key) => (before.get(key) ?? 0) !== (after.get(key) ?? 0));
        assert(changed.length === 0 || record.data.events.length > 0,
            `conservation: ${changed.join(", ")} changed at tick ${record.data.tick} with no economy event`);
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

    const pressShot = at("m3-02-press-two-port-inventory");
    const press = selectedOwn(pressShot, "forge:press", "owner machine is absent from actual rendered state");
    const pressLines = inventoryLines(pressShot.debug);
    assert(portHasCapacity(pressLines, "Main", 6) && portHasCapacity(pressLines, "Output", 4),
        `actual press panel omitted a labelled port or its catalog capacity: ${pressLines.join("\n")}`);

    const ready = at("m3-03-lab-research-ready");
    const lab = selectedOwn(ready, "forge:lab", "owner research station is absent from actual rendered state");
    const inventory = ready.debug.replicated_world.economy.inventories.find((row) => row.unit === lab.unit_id && row.port === "main");
    const shells = inventory?.items?.find((item) => item.resource === "forge:shell");
    const control = widget(ready.debug, (row) => row.id.startsWith("research_entries/") && row.enabled === true &&
        row.widget_action?.content_id === "forge:hardened_shells");
    assert(shells && shells.on_hand - shells.reserved >= 10 && control, "replica inventory and visible enabled research control");

    const researched = at("m3-04-research-applied");
    const researchResults = appliedAfter(researched.debug, maxSeq(ready.debug));
    assert(researchResults.length >= 1 && labQueue(researched.debug, lab.unit_id)?.jobs?.some((job) => job.recipe === "forge:hardened_shells"),
        "research command Acked, Applied and present in replicated queue");

    const disabled = at("m3-05-lab-disabled");
    assert(appliedAfter(disabled.debug, maxSeq(researched.debug)).length >= 1 && labQueue(disabled.debug, lab.unit_id)?.enabled === false,
        "SetEnabled control Acked and Applied with disabled replica state");
    const enabled = at("m3-06-lab-enabled");
    assert(appliedAfter(enabled.debug, maxSeq(disabled.debug)).length >= 1 && labQueue(enabled.debug, lab.unit_id)?.enabled === true,
        "SetEnabled(true) Applied with the enabled replica state");

    const beltShot = at("m3-07-belt-selected");
    const belts = beltShot.debug.replicated_world.belts ?? [];
    assert(belts.length > 0, "replica has no belt piece to remove");
    const piece = beltShot.debug.selection?.[0];
    assert(beltShot.debug.selection?.length === 1 && belts.some((belt) => belt.piece === piece),
        "belt piece selected from rendered scene");
    assert(beltShot.scene.units.some((unit) => unit.unit_id === piece), "belt piece has no actual rendered mesh");
    const removed = at("m3-08-belt-removed");
    assert(appliedAfter(removed.debug, maxSeq(beltShot.debug)).length >= 1 &&
        !(removed.debug.replicated_world.units ?? []).some((unit) => unit.unit_id === piece),
    "Remove control Acked and Applied with belt removed from replica");

    const droneShot = at("m3-10-drone-selected");
    assert.equal(droneShot.window, 2, "the drone order must come from the second window");
    const drone = selectedOwn(droneShot, "forge:drone", "the drone window must select its own drone");
    const ordered = at("m3-11-drone-ordered");
    assert(appliedAfter(ordered.debug, maxSeq(droneShot.debug)).length >= 1, "the drone move into range must be applied");
    const orderTick = droneShot.debug.replicated_world.tick;
    const track = unitTrack(await view.serverFile("server-tick.log"), drone.unit_id).filter((point) => point.tick > orderTick);
    assert(track.some((point) => Math.hypot(point.x - TURRET.x, point.y - TURRET.y) <= TURRET.range),
        `the drone must move into turret range ${TURRET.range} after tick ${orderTick} (server tick log)`);

    const records = parseCompleteAuditText(await view.serverFile("economy.jsonl"));
    assertConservation(records);
}

module.exports = { judge, assertConservation, portHasCapacity };
