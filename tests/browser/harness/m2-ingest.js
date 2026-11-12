// turts-test-role: harness
"use strict";

// M2 lost-delta fault plans (MC task 21; they replace the in-page calls of the legacy
// tests/browser/m2.js). The fault is a test hook of the client, not player input, so the
// harness sets it, as it calls turts_debug.frames_clear before drive (MC design 13.2). The driver
// acts between the harness steps through io only. Each harness step writes the debug record and
// the ingest status that it saw to m2-ingest-private.json; the oracle reads that file with
// view.serverFile and checks the M2 assertions there.
const fs = require("node:fs/promises");
const path = require("node:path");

const PLANS = {
    "E9-browser-base-recovery": "recovery",
    "E9-browser-base-loss-conversion": "conversion",
    "E9-browser-base-loss-charge": "charge",
    "E9-browser-base-loss-refund": "refund",
    "E9-browser-base-loss-spawn": "spawn",
    "E9-browser-base-loss-final_change": "final_change",
};
// The wait for the dropped delta of each family (tests/browser/m2.js runParameterLossProof).
const DROP_WAIT_MS = { conversion: 60000, spawn: 20000, charge: 10000, refund: 10000, final_change: 10000 };

function queueJobs(snapshotValue, producer) {
    return snapshotValue.debug?.replicated_world?.economy?.queues?.find((queue) => queue.producer === producer)?.jobs ?? [];
}

async function snapshot(page) {
    // The client builds the debug state only when asked (design D 9, row DBG1): ask and wait for one
    // record. A page with no request bridge is an older build that publishes in each frame.
    const raw = await page.evaluate(async () => ({
        debug: typeof window.turts_debug !== "function" ? null
            : await (window.turts_debug_request?.() ?? window.turts_debug()),
        status: typeof window.turts_ingest_config_status === "function" ? window.turts_ingest_config_status() : null,
        error: window.turts_error || null,
    }));
    const parse = (text) => { try { return typeof text === "string" && text ? JSON.parse(text) : null; } catch { return null; } };
    return { wall_time_ms: Date.now(), debug: parse(raw.debug), status: parse(raw.status), error: raw.error };
}

async function waitFor(page, predicate, timeoutMs, description) {
    const deadline = Date.now() + timeoutMs;
    let latest = await snapshot(page);
    while (Date.now() < deadline) {
        if (latest.error) throw new Error(`HARNESS M2 browser error during ${description}: ${latest.error}`);
        if (predicate(latest)) return latest;
        await page.waitForTimeout(100);
        latest = await snapshot(page);
    }
    return { ...latest, timed_out: description };
}

async function configure(page, config) {
    await page.evaluate((value) => {
        if (typeof window.turts_configure_ingest !== "function") throw new Error("turts_configure_ingest is unavailable");
        window.turts_configure_ingest(value);
    }, config);
    const applied = await waitFor(page, (current) => current.status?.state === "applied" || current.status?.state === "rejected",
        5000, "queued browser ingest configuration application");
    if (applied.status?.state !== "applied") throw new Error(`HARNESS M2 browser ingest config not applied: ${JSON.stringify(applied.status)}`);
    return { config, ...applied };
}

async function holdWindow(page, ms) {
    const records = [];
    const deadline = Date.now() + ms;
    while (Date.now() < deadline) {
        records.push(await snapshot(page));
        await page.waitForTimeout(50);
    }
    return records;
}

function planFor(params) {
    return PLANS[params.m2?.case_id] ?? null;
}

async function runM2Fixture({ io, driver, params, pages, outDir }) {
    const plan = planFor(params);
    if (!plan) throw new Error(`HARNESS M2 has no ingest plan for ${params.m2?.case_id}`);
    const page = pages[0];
    const record = { case_id: params.m2.case_id, plan, steps: {} };
    const save = () => fs.writeFile(path.join(outDir, "m2-ingest-private.json"), JSON.stringify(record) + "\n", { mode: 0o600 });
    try {
        await driver.drive(io, { ...params, fixture_phase: "select" });
        record.steps.before = await snapshot(page);
        if (plan === "recovery") {
            record.steps.armed = await configure(page, { drop_world_changes: ["extraction"], hold_resync_snapshots: 512, max_buffered_batches: 3 });
            await driver.drive(io, { ...params, fixture_phase: "gather" });
            record.steps.held = await waitFor(page, (s) => s.debug?.ingest?.held_snapshots > 0 && s.debug.ingest.outstanding_request === true,
                15000, "actual held resync Snapshot with one outstanding request");
            record.steps.overflowed = await waitFor(page, (s) => s.debug?.ingest?.held_snapshots > 0 && s.debug.ingest.outstanding_request === true &&
                s.debug.ingest.buffer_overflows > 0, 10000, "real extraction-loss tail to overflow the live cap while the response remains held");
            record.steps.hold_window = await holdWindow(page, 200);
            record.steps.release = await configure(page, { drop_world_changes: ["extraction"], hold_resync_snapshots: 0, max_buffered_batches: 3 });
            const heldTick = record.steps.overflowed.debug?.replicated_world?.tick ?? 0;
            record.steps.recovered = await waitFor(page, (s) => s.debug?.ingest?.held_snapshots === 0 && s.debug.ingest.outstanding_request === false &&
                s.debug.replicated_world?.tick > heldTick, 5000, "recovered replica after releasing the pre-existing held Snapshot");
            const rearmTick = record.steps.release.status?.arm_tick;
            record.steps.redropped = await waitFor(page, (s) => (s.debug?.dropped_world_changes ?? [])
                .filter((drop) => drop.kind === "extraction" && drop.arm_tick === rearmTick).length === 1,
            20000, "same-kind re-arm while the first response was held drops one later real extraction batch");
            const redrop = (record.steps.redropped.debug?.dropped_world_changes ?? []).find((drop) => drop.kind === "extraction" && drop.arm_tick === rearmTick);
            record.steps.rearm_recovered = await waitFor(page, (s) => s.debug?.ingest?.resync_active === false && s.debug.ingest.held_snapshots === 0 &&
                s.debug.ingest.outstanding_request === false && s.debug.replicated_world?.tick > (redrop?.dropped_tick ?? Infinity),
            10000, "re-armed extraction loss recovers through its own successor Snapshot");
        } else {
            const kind = plan;
            const producer = record.steps.before.debug?.selection?.[0];
            if (kind === "refund" || kind === "final_change") {
                await driver.drive(io, { ...params, fixture_phase: "produce" });
                record.steps.queued = await waitFor(page, (s) => queueJobs(s, producer).length > 0, 5000, `${kind} real Produce application and rendered queue row`);
                if (kind === "final_change") {
                    record.steps.blocked = await waitFor(page, (s) => queueJobs(s, producer).some((job) => job.stage === "output_blocked"),
                        15000, "final_change exact job reaches actual stable output_blocked stage");
                    // Two more completed ticks give the oracle the stable audit interval of m2.js.
                    const blockedTick = record.steps.blocked.debug?.replicated_world?.tick ?? 0;
                    record.steps.blocked_stable = await waitFor(page, (s) => (s.debug?.replicated_world?.tick ?? 0) >= blockedTick + 3,
                        5000, "final_change stable output_blocked interval");
                }
                await driver.drive(io, { ...params, fixture_phase: "show-queue" });
            }
            record.steps.armed = await configure(page, { drop_world_changes: [kind], hold_resync_snapshots: 512, max_buffered_batches: 3 });
            await driver.drive(io, { ...params, fixture_phase: kind === "conversion" ? "gather" : kind === "refund" || kind === "final_change" ? "cancel" : "produce" });
            const armTick = record.steps.armed.status?.arm_tick;
            record.steps.dropped = await waitFor(page, (s) => (s.debug?.dropped_world_changes ?? [])
                .some((drop) => drop.kind === kind && drop.arm_tick === armTick), DROP_WAIT_MS[kind], `${kind} real ${kind} WorldDelta loss`);
            record.steps.held = await waitFor(page, (s) => s.debug?.ingest?.held_snapshots === 1 && s.debug.ingest.outstanding_request === true &&
                s.debug.ingest.buffer_overflows > 0, 20000, `${kind} real resync response held while bounded delta tail overflows`);
            record.steps.hold_window = await holdWindow(page, 200);
            record.steps.release = await configure(page, { drop_world_changes: [], hold_resync_snapshots: 0, max_buffered_batches: 3 });
            const heldTick = record.steps.held.debug?.replicated_world?.tick ?? 0;
            const successors = record.steps.held.debug?.ingest?.successor_requests ?? 0;
            record.steps.recovered = await waitFor(page, (s) => s.debug?.ingest?.held_snapshots === 0 && s.debug.ingest.outstanding_request === false &&
                s.debug.ingest.resync_active === false && s.debug.ingest.successor_requests >= successors + 1 && s.debug.replicated_world?.tick > heldTick,
            10000, `${kind} held Snapshot, one successor and full recovery close`);
            if (kind === "spawn") {
                const known = new Set((record.steps.before.debug?.replicated_world?.units ?? []).map((unit) => unit.unit_id));
                record.steps.spawn_output = await waitFor(page, (s) => (s.debug?.replicated_world?.units ?? []).some((unit) => !known.has(unit.unit_id) && unit.owner_slot === 0),
                    15000, "spawned output is both in the recovered browser replica and actual Bevy mesh scene");
                await driver.drive(io, { ...params, fixture_phase: "spawn-output" });
            }
            if (kind === "final_change") {
                const droppedTick = (record.steps.dropped.debug?.dropped_world_changes ?? []).find((drop) => drop.kind === kind && drop.arm_tick === armTick)?.dropped_tick ?? Infinity;
                record.steps.final = await waitFor(page, (s) => (s.debug?.replicated_world?.tick ?? 0) >= droppedTick + 8 &&
                    s.debug?.ingest?.resync_active === false && s.debug.ingest.outstanding_request === false,
                10000, "eight completed authoritative heartbeats after terminal final_change recovery");
            }
        }
        record.steps.after = await snapshot(page);
        await driver.drive(io, { ...params, fixture_phase: "after" });
    } finally {
        await save();
    }
}

module.exports = { planFor, runM2Fixture };
