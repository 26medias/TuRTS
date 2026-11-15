// turts-test-role: unit
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

test("maps the blocked gather wait of the audit to the wire stage and refuses an unknown reason", () => {
    const stage = (reason) => ({ Waiting: { reason } });
    const orders = (state) => projectAuditOwnerWorld(state, 0).economy.orders;
    assert.deepEqual(orders(auditState(4, { orderStage: stage("Blocked") })), [
        { unit: 2, node: 1, receiver: 1, stage: { Waiting: { reason: "blocked" } } },
    ]);
    assert.throws(() => orders(auditState(4, { orderStage: stage("Jammed") })), /unknown audit gather wait reason/);
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

function workFixture(cot = false) {
    const states = Array.from({ length: 4 }, (_, tick) => {
        const state = auditState(tick);
        const job = state.economy.economy.queues[0].jobs[0];
        job.elapsed_ticks = cot ? Math.floor(tick / 2) : tick;
        if (cot) {
            job.charged[0].amount = Math.floor(tick / 2);
            state.economy.operating = { rate_q: 2, rates: [{ unit: 1, producer: [1, 2], constructor: [1, 1] }], holds: [], supply: [],
                jobs: [{ unit: 1, job: 1, done: tick, total: 200, funding: {}, held: false, held_since_tick: 0 }] };
        }
        return state;
    });
    const records = [record("initial", states[0]), ...states.slice(1).map((state, i) => record("tick", { tick: i + 1, before: states[i], state, events: [] }))];
    const rate = { done: cot ? 2 : 1, total: cot ? 200 : 100, per_tick: 1, at_tick: cot ? 2 : 1,
        state: cot ? "Slowed" : "Running", charged: cot ? [{ source: "SlotBalance", resource: "base:credits", amount: 1 }] : [], held: false, held_since_tick: 0 };
    const actual = { ...projectAuditOwnerWorld(states[3], 0), progress: { sites: [], jobs: [{ producer: 1, job: 1, rate, value: 3 }], construction_queues: [] }, repairs: [] };
    return { records, states, actual };
}

test("checks a retained live progress anchor against its audited origin and current value", () => {
    const { projectAuditOwnerWork } = require("./m2-audit.js");
    const { records, states, actual } = workFixture();
    const expected = projectAuditOwnerWork(actual, records, 0);
    assert.equal(expected.progress.jobs[0].rate.at_tick, 1, "an active rate keeps its real old anchor");
    assert.equal(expected.progress.jobs[0].value, 3);
    assert.equal(assertOwnerWorldMatchesAudit(actual, states[3], 0, expected), 3);
    for (const [field, value] of [["done", 0], ["at_tick", 2], ["per_tick", 0], ["total", 99], ["state", "Held"], ["held", true]]) {
        const tampered = structuredClone(actual); tampered.progress.jobs[0].rate[field] = value;
        assert.throws(() => projectAuditOwnerWork(tampered, records, 0), /work anchor|work.*audit|retained work/, field);
    }
    const value = structuredClone(actual); value.progress.jobs[0].value = 4;
    assert.throws(() => projectAuditOwnerWork(value, records, 0), /browser work value/);
});

test("COT progress preserves raw Q and independently checks actual charged balances", () => {
    const { projectAuditOwnerWork } = require("./m2-audit.js");
    const { records, states, actual } = workFixture(true);
    const expected = projectAuditOwnerWork(actual, records, 0);
    assert.equal(expected.progress.jobs[0].value, 3);
    assert.equal(assertOwnerWorldMatchesAudit(actual, states[3], 0, expected), 3);
    const tampered = structuredClone(actual); tampered.progress.jobs[0].rate.charged[0].amount = 2;
    assert.throws(() => projectAuditOwnerWork(tampered, records, 0), /work anchor/);
    const missing = structuredClone(actual); missing.progress.jobs = [];
    assert.throws(() => projectAuditOwnerWork(missing, records, 0), /complete owner job progress/);
});

test("owner work schema rejects foreign progress, repair state and extra private fields", () => {
    const { assertWorkSchema } = require("./m2-work.js");
    const { actual } = workFixture();
    assertWorkSchema(actual, 0);
    const foreign = structuredClone(actual); foreign.progress.jobs[0].producer = 3;
    assert.throws(() => assertWorkSchema(foreign, 0), /foreign or missing/);
    const privateRate = structuredClone(actual); privateRate.progress.jobs[0].rate.raw_ledger = [];
    assert.throws(() => assertWorkSchema(privateRate, 0), /public rate schema/);
    const repair = structuredClone(actual); repair.repairs = [{ repairer: 2, target: 3, state: "Working" }];
    assert.throws(() => assertWorkSchema(repair, 0), /receiving owner/);
});

test("site work, held construction and repair rows come from owned audit records", () => {
    const { projectAuditOwnerWork } = require("./m2-audit.js");
    const { records, states, actual } = workFixture(true);
    for (const [tick, state] of states.entries()) {
        state.economy.units.push({ unit_id: 7, content_id: "base:building", owner_slot: 0, pos: [floatBits(20), floatBits(20)] });
        state.economy.operating.supply = [{ slot: 0, resource: contentId("beds"), provided: 0, consumed: 1 }];
        state.economy.construction = { sites: { 7: { owner_slot: 0, constructor: 1, recipe: contentId("build"), done: tick, total: 200,
            funding: { held_since_tick: 1 }, constructor_when_short: [{ resource: "base:beds", effect: { Rate: { numerator: 1, denominator: 2 } } }],
            charged: [{ endpoint: { Slot: 0 }, resource: contentId("credits"), amount: 3 }] } },
        held: { 1: [{ recipe: contentId("build"), point: [12, 14] }], 3: [{ recipe: contentId("secret"), point: [50, 50] }] } };
        state.economy.economy.repairs = [{ repairer: 2, target: 1, owner_slot: 0, state: { WaitingForFunds: contentId("credits") } }];
    }
    actual.units = projectAuditOwnerWorld(states[3], 0).units;
    actual.progress.sites = [{ unit: 7, production: "base:build", value: 3,
        rate: { done: 2, total: 200, per_tick: 1, at_tick: 2, state: "Slowed", charged: [{ source: "SlotBalance", resource: "base:credits", amount: 3 }], held: true, held_since_tick: 1 } }];
    actual.progress.construction_queues = [{ constructor: 1, orders: [{ content: "base:build", point: { x: 12, y: 14 } }] }];
    actual.repairs = [{ repairer: 2, target: 1, state: { WaitingForFunds: "base:credits" } }];
    const expected = projectAuditOwnerWork(actual, records, 0);
    assert.equal(expected.progress.sites[0].value, 3);
    assert.deepEqual(expected.progress.construction_queues, actual.progress.construction_queues);
    assert.equal(assertOwnerWorldMatchesAudit(actual, states[3], 0, expected), 3);
    const bad = structuredClone(actual); bad.repairs[0].state = "Working";
    assert.throws(() => assertOwnerWorldMatchesAudit(bad, states[3], 0, expected), /owner replica mismatch/);
});
