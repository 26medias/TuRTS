// turts-test-role: pure
"use strict";

const assert = require("node:assert/strict");
const fs = require("node:fs");

function floatFromBits(bits) {
    if (!Number.isInteger(bits) || bits < 0 || bits > 0xffffffff) throw new Error(`invalid audit f32 bits: ${bits}`);
    const bytes = new ArrayBuffer(4);
    const view = new DataView(bytes);
    view.setUint32(0, bits, true);
    return view.getFloat32(0, true);
}

function contentId(value) {
    if (!value || typeof value !== "object" || typeof value.package !== "string" || typeof value.local !== "string") {
        throw new Error(`audit ContentId must be the typed package/local object: ${JSON.stringify(value)}`);
    }
    return `${value.package}:${value.local}`;
}

function worldView(state) {
    const world = state?.economy;
    if (!world || !Number.isInteger(world.tick) || !Array.isArray(world.units) || !Array.isArray(world.nodes) || !world.economy) {
        throw new Error("audit state has no complete economy WorldView");
    }
    return world;
}

function sortBy(items, selector) {
    return items.sort((left, right) => selector(left).localeCompare(selector(right)));
}

function wireGatherStage(stage) {
    if (typeof stage === "string") {
        const names = {
            Idle: "Idle",
            MovingToNode: "ToNode",
            Gathering: "Gathering",
            MovingToReceiver: "ToDropOff",
            Unloading: "Unloading",
        };
        if (names[stage]) return names[stage];
    }
    if (stage?.Waiting) {
        const reasons = {
            NoDeposit: "no_deposit",
            NoReceiver: "no_drop_off",
            DestinationFull: "destination_full",
            DestinationOverflow: "destination_overflow",
        };
        const reason = reasons[stage.Waiting.reason];
        if (!reason) throw new Error(`unknown audit gather wait reason: ${JSON.stringify(stage)}`);
        return { Waiting: { reason } };
    }
    throw new Error(`unknown audit gather stage: ${JSON.stringify(stage)}`);
}

function wireCostLines(lines) {
    return sortBy(lines.map((line) => ({
        source: line.source,
        resource: contentId(line.resource),
        amount: line.amount,
    })), (line) => `${line.resource}|${line.source}`);
}

function projectAuditOwnerWorld(state, slot) {
    if (!Number.isInteger(slot) || slot < 0 || slot > 255) throw new Error(`invalid owner slot ${slot}`);
    const world = worldView(state);
    const economy = world.economy;
    const units = world.units.map((unit) => ({
        unit_id: unit.unit_id,
        content_id: String(unit.content_id),
        owner_slot: unit.owner_slot,
        facing: unit.facing ?? null,
        pos: unit.pos.map(floatFromBits),
    }));
    const nodes = world.nodes.map((node) => ({
        id: node.id,
        content_id: contentId(node.content_id),
        pos: { x: floatFromBits(node.pos[0]), y: floatFromBits(node.pos[1]) },
        remaining: node.remaining,
    }));
    const balances = economy.balances.filter((item) => item.slot === slot).map(({ resource, on_hand, reserved }) => ({
        resource: contentId(resource), on_hand, reserved,
    }));
    const inventories = economy.inventories.filter((item) => item.owner_slot === slot).map((inventory) => ({
        unit: inventory.unit,
        port: String(inventory.port ?? "main").toLowerCase(),
        items: inventory.items.map(({ resource, on_hand, reserved }) => ({ resource: contentId(resource), on_hand, reserved })),
    }));
    const orders = economy.orders.filter((item) => item.owner_slot === slot).map((order) => ({
        unit: order.unit,
        node: order.node,
        receiver: order.receiver,
        stage: wireGatherStage(order.stage),
    }));
    const queues = economy.queues.filter((item) => item.owner_slot === slot).map((queue) => ({
        producer: queue.producer,
        enabled: queue.enabled ?? true,
        // Job FIFO is meaningful and must remain in the source order.
        jobs: queue.jobs.map((job) => ({
            id: job.id,
            recipe: contentId(job.recipe),
            stage: String(job.stage).replace(/[A-Z]/g, (letter) => `_${letter.toLowerCase()}`).replace(/^_/, ""),
            elapsed_ticks: job.elapsed_ticks,
            duration_ticks: job.duration_ticks,
            charged: wireCostLines(job.charged),
            reserved: wireCostLines(job.reserved),
        })),
    }));
    return {
        tick: world.tick,
        units: sortBy(units, (item) => String(item.unit_id).padStart(12, "0")),
        nodes: sortBy(nodes, (item) => String(item.id).padStart(12, "0")),
        economy: {
            balances: sortBy(balances, (item) => item.resource),
            inventories: sortBy(inventories, (item) => String(item.unit).padStart(12, "0")),
            orders: sortBy(orders, (item) => String(item.unit).padStart(12, "0")),
            queues: sortBy(queues, (item) => String(item.producer).padStart(12, "0")),
        },
        belts: world.belts ?? [],
        extractors: world.extractors ?? [],
        health_trace: world.health_trace ?? [],
        healths: world.healths ?? [],
        inserters: world.inserters ?? [],
        research: world.research ?? [],
        weapons: world.weapons ?? [],
    };
}

function validateAuditRecords(records) {
    if (!Array.isArray(records) || records.length === 0 || records[0]?.record !== "initial") {
        throw new Error("first economy audit record must be initial");
    }
    let previousState = records[0].data;
    let previousTick = worldView(previousState).tick;
    const seenTicks = new Set();
    for (const [index, item] of records.entries()) {
        if (!item || typeof item.record !== "string" || !item.data || typeof item.data !== "object") {
            throw new Error(`invalid economy audit record at line ${index + 1}`);
        }
        if (index > 0 && item.record === "initial") throw new Error("economy audit contains multiple initial records");
        if (item.record !== "tick") continue;
        const { tick, before, state, events } = item.data;
        if (!Number.isInteger(tick) || tick !== previousTick + 1 || seenTicks.has(tick)) {
            throw new Error(`economy audit tick ${tick} is discontinuous after ${previousTick}`);
        }
        if (worldView(before).tick !== previousTick || worldView(state).tick !== tick || !Array.isArray(events)) {
            throw new Error(`economy audit tick ${tick} has a discontinuous before/state clock`);
        }
        if (JSON.stringify(worldView(before)) !== JSON.stringify(worldView(previousState))) {
            throw new Error(`economy audit tick ${tick} before-state differs from the previous completed tick`);
        }
        if (events.some((event) => event.tick !== tick)) throw new Error(`economy audit tick ${tick} contains an event from another tick`);
        seenTicks.add(tick);
        previousTick = tick;
        previousState = state;
    }
    return records;
}

function auditStateAt(records, tick) {
    validateAuditRecords(records);
    if (worldView(records[0].data).tick === tick) return records[0].data;
    const entry = records.find((record) => record.record === "tick" && record.data.tick === tick);
    if (!entry) throw new Error(`completed economy audit tick ${tick} is missing`);
    return entry.data.state;
}

function canonicalOwnerWorld(value) {
    const copy = JSON.parse(JSON.stringify(value));
    const by = (key) => (left, right) => String(left[key]).localeCompare(String(right[key]), "en", { numeric: true });
    copy.units.sort(by("unit_id"));
    copy.nodes.sort(by("id"));
    copy.economy.balances.sort(by("resource"));
    copy.economy.inventories.sort(by("unit"));
    for (const item of copy.economy.inventories) item.items.sort(by("resource"));
    copy.economy.orders.sort(by("unit"));
    copy.economy.queues.sort(by("producer"));
    for (const queue of copy.economy.queues) {
        // Preserve job FIFO; only each job's unordered cost lines are canonicalized.
        for (const job of queue.jobs) {
            const costOrder = (left, right) => `${left.resource}|${left.source}`.localeCompare(`${right.resource}|${right.source}`);
            job.charged.sort(costOrder);
            job.reserved.sort(costOrder);
        }
    }
    return copy;
}

function assertOwnerWorldMatchesAudit(actual, state, slot) {
    const tick = worldView(state).tick;
    const expected = projectAuditOwnerWorld(state, slot);
    assert.deepStrictEqual(canonicalOwnerWorld(actual), canonicalOwnerWorld(expected),
        `owner replica mismatch at authoritative completed audit tick ${tick}`);
    return tick;
}

async function readCompleteAuditLog(filePath, timeoutMs = 2000) {
    const deadline = Date.now() + timeoutMs;
    while (true) {
        const bytes = fs.readFileSync(filePath);
        if (bytes.length > 0 && bytes[bytes.length - 1] === 0x0a) {
            const lines = bytes.toString("utf8").split("\n").slice(0, -1);
            if (lines.some((line) => line.trim() === "")) throw new Error("economy audit JSONL contains an empty completed line");
            const records = lines.map((line, index) => {
                try { return JSON.parse(line); } catch (error) { throw new Error(`invalid complete economy audit line ${index + 1}: ${error.message}`); }
            });
            return validateAuditRecords(records);
        }
        if (Date.now() >= deadline) throw new Error("economy audit log ended with a torn record past the bounded wait");
        await new Promise((resolve) => setTimeout(resolve, 5));
    }
}

function balanceIncrease(previous, current, slot) {
    const before = previous.economy.economy.balances.filter((item) => item.slot === slot);
    return current.economy.economy.balances.some((item) => item.slot === slot && item.on_hand >
        (before.find((old) => contentId(old.resource) === contentId(item.resource))?.on_hand ?? 0));
}

function inventoryIncrease(previous, current, slot) {
    const before = previous.economy.economy.inventories.filter((item) => item.owner_slot === slot);
    return current.economy.economy.inventories.some((inventory) => inventory.owner_slot === slot &&
        inventory.items.some((item) => item.on_hand > (before.find((old) => old.unit === inventory.unit)?.items
            .find((old) => contentId(old.resource) === contentId(item.resource))?.on_hand ?? 0)));
}

function inventoryDecrease(previous, current, slot) {
    const after = current.economy.economy.inventories.filter((item) => item.owner_slot === slot);
    return previous.economy.economy.inventories.filter((item) => item.owner_slot === slot).some((inventory) =>
        inventory.items.some((item) => item.on_hand > (after.find((old) => old.unit === inventory.unit)?.items
            .find((old) => contentId(old.resource) === contentId(item.resource))?.on_hand ?? 0)));
}

function balanceDecrease(previous, current, slot) {
    const after = current.economy.economy.balances.filter((item) => item.slot === slot);
    return previous.economy.economy.balances.filter((item) => item.slot === slot).some((item) =>
        item.on_hand > (after.find((old) => contentId(old.resource) === contentId(item.resource))?.on_hand ?? 0));
}

function jobsFor(state, slot) {
    return state.economy.economy.queues.filter((queue) => queue.owner_slot === slot).flatMap((queue) =>
        queue.jobs.map((job) => ({ producer: queue.producer, job })));
}

function newChargedLine(previous, current, slot) {
    const beforeJobs = jobsFor(previous, slot);
    return jobsFor(current, slot).some(({ producer, job }) => job.charged.some((line) => {
        const priorJob = beforeJobs.find((entry) => entry.producer === producer && entry.job.id === job.id)?.job;
        const priorLine = priorJob?.charged.find((old) => old.source === line.source && contentId(old.resource) === contentId(line.resource));
        return !priorLine || line.amount > priorLine.amount;
    }));
}

function worldChangeMatchesAudit(kind, previousState, currentState, slot) {
    const previous = projectAuditOwnerWorld(previousState, slot);
    const current = projectAuditOwnerWorld(currentState, slot);
    const balancesUp = balanceIncrease(previousState, currentState, slot);
    const inventoryUp = inventoryIncrease(previousState, currentState, slot);
    const inventoryDown = inventoryDecrease(previousState, currentState, slot);
    const balanceDown = balanceDecrease(previousState, currentState, slot);
    switch (kind) {
        case "extraction":
            return previous.nodes.some((node) => current.nodes.find((item) => item.id === node.id)?.remaining < node.remaining) && inventoryUp;
        case "conversion":
            return inventoryDown && balancesUp;
        case "charge":
            return newChargedLine(previousState, currentState, slot) && (inventoryDown || balanceDown);
        case "refund": {
            const previousQueues = previous.economy.queues;
            const currentQueues = current.economy.queues;
            const queueShrank = previousQueues.some((queue) => queue.jobs.length >
                (currentQueues.find((item) => item.producer === queue.producer)?.jobs.length ?? 0));
            return queueShrank && (balancesUp || inventoryUp);
        }
        case "spawn":
            return current.units.some((unit) => !previous.units.some((old) => old.unit_id === unit.unit_id));
        case "final_change":
            return JSON.stringify(previous.economy) !== JSON.stringify(current.economy);
        default:
            throw new Error(`unknown WorldDelta loss family ${kind}`);
    }
}

module.exports = {
    assertOwnerWorldMatchesAudit,
    auditStateAt,
    projectAuditOwnerWorld,
    readCompleteAuditLog,
    validateAuditRecords,
    worldChangeMatchesAudit,
};
