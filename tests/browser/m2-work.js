// turts-test-role: pure
"use strict";
const assert = require("node:assert/strict");
const RATE_KEYS = ["at_tick", "charged", "done", "held", "held_since_tick", "per_tick", "state", "total"];
function exact(value, keys, label) { assert.deepEqual(Object.keys(value).sort(), [...keys].sort(), label); }
function own(view, id, slot) { return view.units.some((u) => u.unit_id === id && u.owner_slot === slot); }
function assertWorkSchema(view, slot) {
    exact(view.progress, ["sites", "jobs", "construction_queues"], "complete owner progress schema");
    for (const [kind, rows] of Object.entries(view.progress)) {
        assert(Array.isArray(rows), `progress ${kind} must be an array`);
        for (const row of rows) {
            const id = kind === "jobs" ? row.producer : kind === "sites" ? row.unit : row.constructor;
            assert(own(view, id, slot), `progress of foreign or missing unit ${id}`);
            exact(row, kind === "jobs" ? ["producer", "job", "rate", "value"] : kind === "sites" ? ["unit", "production", "rate", "value"] : ["constructor", "orders"], `progress ${kind} row schema`);
            if (kind !== "construction_queues") {
                exact(row.rate, RATE_KEYS, "public rate schema");
                for (const line of row.rate.charged) exact(line, ["source", "resource", "amount"], "public charge schema");
            } else {
                assert(Array.isArray(row.orders));
                for (const order of row.orders) exact(order, ["content", "point"], "held order schema");
            }
        }
    }
    assert(Array.isArray(view.repairs));
    for (const row of view.repairs) {
        exact(row, ["repairer", "target", "state"], "public repair schema");
        assert(own(view, row.repairer, slot) && own(view, row.target, slot), "repair state must belong to the receiving owner");
    }
}
function number(value, label) { assert(Number.isSafeInteger(value) && value >= 0, `unsafe or invalid ${label}`); return BigInt(value); }
function checked(value) { assert(value <= BigInt(Number.MAX_SAFE_INTEGER), "browser work oracle numeric range exceeded"); return Number(value); }
function projectWork(actual, records, slot, helpers) {
    const { worldView, contentId, wireCostLines } = helpers;
    const states = new Map();
    for (const record of records) {
        if (record.record === "initial") states.set(worldView(record.data).tick, worldView(record.data));
        if (record.record === "tick") states.set(record.data.tick, worldView(record.data.state));
    }
    const tip = states.get(actual.tick);
    assert(tip, `missing work audit tick ${actual.tick}`);
    const cid = (value) => typeof value === "string" ? value : contentId(value);
    const held = (world, id) => (world.operating?.holds ?? []).find((r) => r.unit === id);
    const disabled = (world, id) => (world.m3?.disabled ?? []).includes(id);
    function facts(world, kind, id, jobId) {
        const q = number(world.operating?.rate_q ?? 1, "rate scale");
        assert(q > 0n);
        if (kind === "jobs") {
            const queue = world.economy.queues.find((r) => r.producer === id && r.owner_slot === slot);
            const job = queue?.jobs.find((r) => r.id === jobId);
            assert(job, `missing owned job ${id}/${jobId} at audit tick ${world.tick}`);
            const raw = (world.operating?.jobs ?? []).find((r) => r.unit === id && r.job === jobId);
            const hold = job.stage === "Queued" ? null : held(world, id);
            const pair = (world.operating?.rates ?? []).find((r) => r.unit === id)?.producer ?? [1, 1];
            const step = q * number(pair[0], "rate numerator") / number(pair[1], "rate denominator");
            const waiting = raw?.funding?.waiting_for_funds;
            const per_tick = disabled(world, id) || held(world, id) || waiting || job.stage !== "Running" ? 0 : checked(step);
            return { done: raw?.done ?? job.elapsed_ticks, total: raw?.total ?? job.duration_ticks, per_tick,
                state: waiting ? { WaitingForFunds: cid(waiting) } : hold ? "Held" : job.stage === "OutputBlocked" ? "OutputBlocked" : per_tick > 0 && BigInt(per_tick) < q ? "Slowed" : "Running",
                charged: raw?.funding ? wireCostLines(job.charged) : [], held: Boolean(raw?.held || hold), held_since_tick: hold?.since_tick ?? raw?.held_since_tick ?? 0 };
        }
        const site = world.construction?.sites?.[String(id)];
        assert(site && site.owner_slot === slot, `missing owned site ${id} at audit tick ${world.tick}`);
        let ratio = [1n, 1n];
        for (const rule of site.constructor_when_short ?? []) {
            const supply = (world.operating?.supply ?? []).find((r) => r.slot === slot && cid(r.resource) === rule.resource);
            if (!supply || supply.provided >= supply.consumed || !rule.effect?.Rate) continue;
            const other = [number(rule.effect.Rate.numerator, "site numerator"), number(rule.effect.Rate.denominator, "site denominator")];
            if (other[0] * ratio[1] < ratio[0] * other[1]) ratio = other;
        }
        const hold = held(world, id), waiting = site.funding?.waiting_for_funds;
        const per_tick = hold || waiting || disabled(world, id) || disabled(world, site.constructor) ? 0 : checked(q * ratio[0] / ratio[1]);
        return { done: site.done, total: site.total, per_tick,
            state: waiting ? { WaitingForFunds: cid(waiting) } : hold ? "Held" : ratio[0] < ratio[1] ? "Slowed" : "Running",
            charged: site.charged.map((r) => ({ source: Object.hasOwn(r.endpoint, "Slot") ? "SlotBalance" : "ProducerInventory", resource: cid(r.resource), amount: r.amount })),
            held: Boolean(hold || site.funding?.held_since_tick != null), held_since_tick: hold?.since_tick ?? site.funding?.held_since_tick ?? 0 };
    }
    function rate(row, kind, id, jobId) {
        const at = row.rate.at_tick;
        assert(Number.isInteger(at) && at >= 0 && at <= actual.tick, "invalid work anchor tick");
        const anchor = states.get(at); assert(anchor, `missing work anchor audit tick ${at}`);
        const expected = facts(anchor, kind, id, jobId);
        const canonical = (r) => ({ ...r, charged: [...r.charged].sort((a, b) => `${a.resource}|${a.source}`.localeCompare(`${b.resource}|${b.source}`)) });
        const { at_tick: _at, ...observed } = row.rate;
        assert.deepEqual(canonical(observed), canonical(expected), `work anchor differs from authoritative tick ${at}`);
        const current = facts(tip, kind, id, jobId);
        for (const field of ["total", "per_tick", "state", "held", "held_since_tick", "charged"]) {
            assert.deepEqual(canonical({ charged: [], [field]: expected[field] })[field], canonical({ charged: [], [field]: current[field] })[field], `stale work ${field}`);
        }
        const predicted = number(row.rate.done, "work done") + number(row.rate.per_tick, "work step") * BigInt(actual.tick - at);
        const total = number(row.rate.total, "work total");
        assert.equal(predicted > total ? total : predicted, number(current.done, "audited work value"), "retained work anchor predicts a different audited value");
        assert.equal(row.value, current.done, "browser work value differs from audit");
        // The retained anchor is copied only after both independent checks above.
        return { rate: { ...row.rate }, value: current.done };
    }
    const expectedJobs = tip.economy.queues.filter((q) => q.owner_slot === slot).flatMap((q) => q.jobs.map((j) => [q.producer, j.id]));
    const expectedSites = Object.entries(tip.construction?.sites ?? {}).filter(([, s]) => s.owner_slot === slot).map(([id]) => Number(id));
    assert.deepEqual(actual.progress.jobs.map((r) => `${r.producer}/${r.job}`).sort(), expectedJobs.map(([p, j]) => `${p}/${j}`).sort(), "complete owner job progress");
    assert.deepEqual(actual.progress.sites.map((r) => r.unit).sort((a, b) => a - b), expectedSites.sort((a, b) => a - b), "complete owner site progress");
    const jobs = actual.progress.jobs.map((row) => ({ producer: row.producer, job: row.job, ...rate(row, "jobs", row.producer, row.job) }));
    const sites = actual.progress.sites.map((row) => {
        const production = cid(tip.construction.sites[String(row.unit)].recipe);
        assert.equal(row.production, production, "site recipe differs from audit");
        return { unit: row.unit, production, ...rate(row, "sites", row.unit) };
    });
    const construction_queues = Object.entries(tip.construction?.held ?? []).filter(([id, orders]) => orders.length && own(tip, Number(id), slot)).map(([id, orders]) => ({
        constructor: Number(id), orders: orders.map((r) => ({ content: cid(r.recipe), point: { x: r.point[0], y: r.point[1] } })),
    }));
    const repairs = (tip.economy.repairs ?? []).filter((r) => r.owner_slot === slot).map((r) => ({ repairer: r.repairer, target: r.target,
        state: r.state?.WaitingForFunds ? { WaitingForFunds: cid(r.state.WaitingForFunds) } : r.state }));
    return { progress: { sites, jobs, construction_queues }, repairs };
}
module.exports = { assertWorkSchema, projectWork };
