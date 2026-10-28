"use strict";

const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const recordedAuditWorld = JSON.parse(fs.readFileSync(
    path.join(__dirname, "fixtures/task16-owner-audit-world-view.json"), "utf8",
)).state;
const { test } = require("node:test");
const {
    assertOwnerWorldMatchesAudit,
    auditStateAt,
    projectAuditOwnerWorld,
    validateAuditRecords,
    worldChangeMatchesAudit,
} = require("./m2-audit.js");

function floatBits(value) {
    const bytes = new ArrayBuffer(4);
    const view = new DataView(bytes);
    view.setFloat32(0, value, true);
    return view.getUint32(0, true);
}

function contentId(local) { return { package: "base", local }; }

function auditState(tick, overrides = {}) {
    const job = overrides.job ?? {
        id: 1,
        recipe: contentId("train_rifleman"),
        stage: "Running",
        elapsed_ticks: 1,
        duration_ticks: 100,
        charged: [{ source: "SlotBalance", resource: contentId("credits"), amount: 100 }],
        reserved: [],
    };
    return {
        economy: {
            tick,
            units: [
                { unit_id: 1, content_id: "base:command_center", owner_slot: 0, pos: [floatBits(8), floatBits(8)] },
                { unit_id: 2, content_id: "base:harvester", owner_slot: 0, pos: [floatBits(12), floatBits(8)] },
                { unit_id: 3, content_id: "base:command_center", owner_slot: 1, pos: [floatBits(56), floatBits(8)] },
            ],
            nodes: [{ id: 1, content_id: contentId("ore_deposit"), pos: [floatBits(24), floatBits(8)], remaining: overrides.node ?? 1000 }],
            economy: {
                balances: [
                    { slot: 0, resource: contentId("credits"), on_hand: overrides.credits ?? 500, reserved: 0 },
                    { slot: 1, resource: contentId("credits"), on_hand: 500, reserved: 0 },
                ],
                inventories: [
                    { unit: 2, owner_slot: 0, capacity: 10, items: [{ resource: contentId("ore"), on_hand: overrides.ore ?? 0, reserved: 0 }] },
                ],
                orders: [
                    { unit: 2, owner_slot: 0, node: 1, receiver: 1, stage: overrides.orderStage ?? "Gathering" },
                ],
                queues: (overrides.jobs ?? [job]).length ? [
                    { producer: 1, owner_slot: 0, jobs: overrides.jobs ?? [job] },
                ] : [],
            },
        },
    };
}

function record(record, data) { return { record, data }; }

test("projects a complete owner-only audit world into the browser snapshot schema", () => {
    const state = auditState(4);
    const actual = projectAuditOwnerWorld(state, 0);
    assert.deepEqual(actual, {
        tick: 4,
        units: [
            { unit_id: 1, content_id: "base:command_center", owner_slot: 0, facing: null, pos: [8, 8] },
            { unit_id: 2, content_id: "base:harvester", owner_slot: 0, facing: null, pos: [12, 8] },
            { unit_id: 3, content_id: "base:command_center", owner_slot: 1, facing: null, pos: [56, 8] },
        ],
        nodes: [{ id: 1, content_id: "base:ore_deposit", pos: { x: 24, y: 8 }, remaining: 1000 }],
        economy: {
            balances: [{ resource: "base:credits", on_hand: 500, reserved: 0 }],
            inventories: [{ unit: 2, port: "main", items: [{ resource: "base:ore", on_hand: 0, reserved: 0 }] }],
            orders: [{ unit: 2, node: 1, receiver: 1, stage: "Gathering" }],
            queues: [{ producer: 1, enabled: true, jobs: [{
                id: 1, recipe: "base:train_rifleman", stage: "running", elapsed_ticks: 1, duration_ticks: 100,
                charged: [{ source: "SlotBalance", resource: "base:credits", amount: 100 }], reserved: [],
            }] }],
        },
        belts: [],
        extractors: [],
        health_trace: [],
        healths: [],
        inserters: [],
        research: [],
        weapons: [],
    });
    assert.equal(actual.units.some((unit) => unit.owner_slot === 1), true, "public world units from both owners remain visible");
    assert.equal(actual.economy.balances.some((balance) => balance.slot === 1), false, "foreign owner economy must be excluded");
});

test("validates completed audit tick continuity and selects an exact tick", () => {
    const initial = auditState(0);
    const tick1 = auditState(1);
    const tick2 = auditState(2);
    const records = [
        record("initial", initial),
        record("tick", { tick: 1, before: initial, state: tick1, events: [] }),
        record("tick", { tick: 2, before: tick1, state: tick2, events: [] }),
    ];
    assert.equal(auditStateAt(records, 2).economy.tick, 2);
    assert.throws(() => validateAuditRecords([
        records[0], records[1], record("tick", { tick: 4, before: tick2, state: auditState(4), events: [] }),
    ]), /discontinuous/);
});

test("requires an actual matching authoritative transition for each named loss family", () => {
    const baseline = auditState(4, { ore: 2, credits: 490 });
    const cases = [
        ["extraction", auditState(5, { ore: 3, credits: 490, node: 999 })],
        ["conversion", auditState(5, { ore: 1, credits: 491 })],
        ["charge", auditState(5, { ore: 2, credits: 390, job: { id: 2, recipe: contentId("train_rifleman"), stage: "Queued", elapsed_ticks: 0, duration_ticks: 100, charged: [{ source: "SlotBalance", resource: contentId("credits"), amount: 100 }], reserved: [] } })],
        ["refund", auditState(5, { ore: 2, credits: 540, jobs: [] })],
        ["spawn", auditState(5, { ore: 2, credits: 490, jobs: [] })],
        ["final_change", auditState(5, { ore: 2, credits: 491 })],
    ];
    cases[4][1].economy.units.push({ unit_id: 4, content_id: "base:rifleman", owner_slot: 0, pos: [floatBits(8), floatBits(12)] });
    for (const [kind, after] of cases) assert.equal(worldChangeMatchesAudit(kind, baseline, after, 0), true, kind);
    assert.equal(worldChangeMatchesAudit("extraction", baseline, auditState(5, { ore: 3, credits: 490 }), 0), false,
        "inventory gain without node depletion is not extraction proof");
    assert.equal(worldChangeMatchesAudit("extraction", baseline, auditState(5, { ore: 2, credits: 490, node: 999 }), 0), false,
        "a node decrement with unchanged inventory is not extraction proof");
    assert.equal(worldChangeMatchesAudit("conversion", baseline, auditState(5, { ore: 2, credits: 491 }), 0), false,
        "a balance increase with unchanged inventory is not conversion proof");
});


test("compares a captured browser snapshot only to its exact authoritative tick", () => {
    const state = auditState(4);
    const expected = projectAuditOwnerWorld(state, 0);
    assert.equal(assertOwnerWorldMatchesAudit(expected, state, 0), 4);
    assert.throws(() => assertOwnerWorldMatchesAudit({ ...expected, tick: 5 }, state, 0), /owner replica mismatch/);
    assert.throws(() => assertOwnerWorldMatchesAudit({ ...expected, economy: { ...expected.economy, balances: [] } }, state, 0), /owner replica mismatch/);
});


test("projects ContentId package/local objects from a sanitized real completed audit record", () => {
    const world = projectAuditOwnerWorld(recordedAuditWorld, 0);
    assert.equal(world.tick, 81);
    assert.equal(world.nodes[0].content_id, "base:ore_deposit");
    assert.equal(world.economy.balances[0].resource, "base:credits");
    assert.equal(world.economy.queues[0].jobs[0].recipe, "base:train_rifleman");
    assert.equal(world.economy.queues[0].jobs[0].charged[0].resource, "base:credits");
    assert.equal(world.economy.balances.some((balance) => balance.slot === 1), false);
});
