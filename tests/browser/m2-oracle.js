// turts-test-role: oracle
"use strict";

// M2 browser oracle (MC design 13.3; replaces the asserts of tests/browser/m2.js). It reads the
// harness checkpoints of the m2-driver.js screenshots, the harness record of the lost-delta
// fault plans (m2-ingest-private.json), the trusted economy log of base, and, for the
// public-convergence cases, the live read-only bridges of both windows. An assert failure is a
// semantic failure of the case: scripts/m2-browser.sh turns it into M2_CASE_SEMANTIC_FAIL.
const assert = require("node:assert/strict");
const { decodePng } = require("./harness/png.js");
const { assertNoDriverStop, checkpointBefore, checkpointForSave, readCheckpoints, replicaUnit, widgets } = require("./checkpoint-lib.js");
const { auditStateAt, projectAuditOwnerWorld, worldChangeMatchesAudit } = require("./m2-audit.js");
const {
    appliedCommandCount,
    assertFinalChangeQuiescence,
    assertPublicProjectionSchema,
    compareBrowserSnapshotToAudit,
    hasSingleDebugSelection,
    lastAuditTick,
    lossTransitionAt,
    readAuditRecords,
    projectGroundPoint,
    publicProjection,
} = require("./m2-oracle-lib.js");
const { visibleRingPixels } = require("./vis-full-oracle.js");
const { validateTranslationReceipt } = require("./harness/m2-fixture.js");

// mods/<package>/units/*.ron names; scripts/m2-browser.sh renames the two cedar units.
const UNIT_NAMES = {
    "base:command_center": "Command Center", "base:harvester": "Harvester", "base:rifleman": "Rifleman",
    "haul:depot": "Depot", "haul:worker": "Worker", "acme:scout": "Scout",
    "cedar:depot": "Task16 Cedar Yard", "cedar:worker": "Task16 Carrier",
};
const PRODUCER = { base: "base:command_center", haul: "haul:depot", cedar: "cedar:depot", acme: "acme:scout" };
const WORKER = { base: "base:harvester", haul: "haul:worker", cedar: "cedar:worker" };
const OFFSET = { base: 18, haul: 18, cedar: 18, acme: 0 };

function unitNear(debug, contentId, x, y) {
    return (debug.replicated_world?.units ?? []).find((unit) => unit.content_id === contentId && unit.owner_slot === 0 &&
        Math.abs(unit.pos[0] - x) < 1e-3 && Math.abs(unit.pos[1] - y) < 1e-3) ?? null;
}

function notTimedOut(step, label) {
    assert(step, `the harness did not record ${label}`);
    assert(!step.timed_out, `timeout waiting for ${step.timed_out}`);
    return step;
}

function assertJoined(joined, packageName) {
    assert(joined.debug.phase?.startsWith("Joined { slot: 0"), `expected first owner slot, saw ${joined.debug.phase}`);
    assert((joined.debug.units ?? []).some((unit) => unit.content_id.startsWith(`${packageName}:`)), `${packageName} join and rendered scene`);
    const adapter = joined.scene.render_adapter;
    assert(adapter && typeof adapter.backend === "string" && adapter.backend.toLowerCase().includes("webgpu"),
        `Bevy did not select a WebGPU adapter: ${JSON.stringify(adapter)}`);
    assert(typeof adapter.name === "string" && ["Cpu", "IntegratedGpu", "DiscreteGpu", "VirtualGpu", "Other"].includes(adapter.device_type),
        `actual WebGPU adapter metadata is incomplete: ${JSON.stringify(adapter)}`);
    assert.equal(joined.scene.cameras_3d, 1, "one active primary 3D camera must render the scene");
    assert(joined.scene.clip_from_world?.length === 16, "actual camera projection metadata is missing");
    assert(joined.scene.units?.length > 0, "no rendered unit meshes in the actual scene");
}

// The pick circle of the selected unit stays clear of each visible HUD widget (the M2 engine
// panel became the plain mod HUD of MC design 12.2).
function assertPickCircleClearOfHud(scene, debug, entry) {
    const radius = entry.pick_radius;
    assert(Number.isFinite(radius) && radius > 0, "selected unit has no catalog pick radius");
    const [wx, , wz] = entry.translation;
    const scale = scene.scale_factor;
    const samples = [];
    for (let index = 0; index < 32; index += 1) {
        const angle = index * Math.PI / 16;
        const point = projectGroundPoint(scene, [wx + Math.cos(angle) * radius, wz + Math.sin(angle) * radius]);
        assert(point, `pick-circle sample ${index} is outside the active camera viewport`);
        samples.push([point[0] * scale, point[1] * scale]);
    }
    const circle = [Math.min(...samples.map((p) => p[0])), Math.min(...samples.map((p) => p[1])),
        Math.max(...samples.map((p) => p[0])), Math.max(...samples.map((p) => p[1]))];
    for (const row of widgets(debug, (item) => item.kind === "button" && Array.isArray(item.rect))) {
        const [x, y, width, height] = row.rect;
        const overlaps = circle[0] < x + width && circle[2] > x && circle[1] < y + height && circle[3] > y;
        assert(!overlaps, `catalog pick circle ${JSON.stringify(circle)} overlaps actual Bevy panel ${JSON.stringify(row.rect)}`);
    }
    return circle;
}

async function image(view, file) {
    return decodePng(Buffer.from(await view.serverFile(`${file}.png.b64`), "base64"));
}

async function judgeSelection(view, at, joined, packageName) {
    const offset = OFFSET[packageName];
    const position = packageName === "acme" ? [4, 4] : [8 + offset, 8];
    const producer = unitNear(joined.debug, PRODUCER[packageName], ...position);
    assert(producer, `owner-0 ${PRODUCER[packageName]} is absent from actual received/rendered state`);
    const selected = at("m2-producer-selected");
    const mesh = selected.scene.units.find((unit) => unit.unit_id === producer.unit_id);
    assert(mesh, "the selected unit has no rendered mesh");
    assert(hasSingleDebugSelection(selected, producer.unit_id), "read-only debug selection must agree with the actual rendered selection");
    assert((selected.debug.rings_shown ?? []).some((ring) => ring.unit === producer.unit_id && ring.own === true) &&
        visibleRingPixels(await image(view, "m2-producer-selected"), selected.scene, { ...mesh, owner_slot: producer.owner_slot }, mesh.pick_radius),
    "selection must appear in actual visible Bevy rendering, not only in debug state");
    assert.equal(appliedCommandCount(selected), appliedCommandCount(joined), "unit selection must not submit a world command");
    const circle = assertPickCircleClearOfHud(selected.scene, selected.debug, mesh);
    const hover = at("m2-producer-hover");
    const expected = `${UNIT_NAMES[producer.content_id]} · Slot 1`;
    const tooltip = widgets(hover.debug, (row) => row.kind === "world_tooltip" && row.id === `world.tooltip.${producer.unit_id}`)[0];
    assert(tooltip && tooltip.text === expected, `rendered world tooltip must show the package unit name: want "${expected}", saw ${JSON.stringify(tooltip?.text ?? null)}`);
    return { unit_id: producer.unit_id, circle };
}

function inventoryOf(debug, unitId) {
    return (debug.replicated_world?.economy?.inventories ?? []).filter((row) => row.unit === unitId)
        .flatMap((row) => row.items ?? []).reduce((sum, item) => sum + item.on_hand, 0);
}

function judgeGather(input, checkpoints, at, joined, packageName) {
    const worker = unitNear(joined.debug, WORKER[packageName], 12 + OFFSET[packageName], 8);
    assert(worker, `${packageName} gather fixture is missing its worker or rendered node`);
    const selected = at("m2-worker-selected");
    assert(hasSingleDebugSelection(selected, worker.unit_id), `${packageName} gatherer selection`);
    const nodes = selected.debug.replicated_world.nodes ?? [];
    const command = at("m2-gather-command");
    const later = input.map((entry) => entry).filter((entry) => entry.call === "save" && /^m2-gather-\d+\.png$/.test(entry.args?.[0] ?? ""))
        .map((entry) => checkpointForSave(input, checkpoints, entry.args[0], "M2"));
    const before = appliedCommandCount(selected);
    const started = [command, ...later].find((shot) => appliedCommandCount(shot) > before);
    assert(started, `${packageName} real Gather command and rendered status`);
    // m2.js:1483-1486: the right click applies exactly one fresh command, with a non-idle order.
    assert.equal(appliedCommandCount(started), before + 1, "the economy Gather click must apply exactly one fresh command");
    assert((started.debug.replicated_world?.economy?.orders ?? []).some((order) => order.unit === worker.unit_id),
        "one applied Gather command with a non-idle rendered order");
    // m2.js:1506: no other client command follows the Gather.
    assert(later.every((shot) => appliedCommandCount(shot) <= before + 1), "Gather did not remain the latest applied client command");
    const gathered = later.find((shot) => inventoryOf(shot.debug, worker.unit_id) > inventoryOf(selected.debug, worker.unit_id) &&
        (shot.debug.replicated_world.nodes ?? []).some((node) => {
            const initial = nodes.find((item) => item.id === node.id);
            return initial && node.remaining < initial.remaining;
        }));
    assert(gathered, `${packageName} actual extraction and inventory increase`);
}

async function judgeConvergence(view, at, joined, packageName) {
    const owner1 = at("m2-owner1-joined");
    assert.equal(owner1.window, 2, "the second owner must use the second window");
    assert(owner1.debug.phase?.startsWith("Joined { slot: 1") && owner1.scene.units?.length > 0, `${packageName} second-owner late join`);
    const ownerUnits = (owner1.debug.replicated_world?.units ?? []).filter((unit) => unit.owner_slot === 1);
    if (ownerUnits.length > 0) {
        const selected = at("m2-owner1-selected");
        const unitId = selected.debug.selection?.[0];
        assert(selected.debug.selection?.length === 1 && replicaUnit(selected.debug, unitId)?.owner_slot === 1, `${packageName} owner-1 real selection`);
        assert.equal(appliedCommandCount(selected), appliedCommandCount(owner1), "owner-1 selection must not issue a world command");
        const mesh = selected.scene.units.find((unit) => unit.unit_id === unitId);
        assertPickCircleClearOfHud(selected.scene, selected.debug, mesh);
    }
    const shared = await sampleSharedTick(view);
    if (shared) return shared;
    assert.fail("public world projections differ between owners at a shared coherent tick");
}

// Each window builds its debug state on a request, at most 4 times a second (design D 9, row DBG1;
// `view.bridge` makes the request). Two reads one after the other leave the two builds one round
// trip apart, and the 250 ms cap keeps that gap the same in every round, so a pair that straddles a
// tick boundary (20 Hz) straddles it in every round and the ticks never meet. Idle the windows for
// more than the cap, so that a request builds in the next frame, ask both windows at once, and
// change the idle time in each round so that the pair does not fall on the same phase of the tick.
// The pair must show one tick and one public projection (the M2 convergence claim, unchanged).
const DEBUG_CAP_MS = 250;
async function sampleSharedTick(view, { deadlineMs = 10000, now = Date.now, sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms)),
    project = (debug) => publicProjection({ debug }) } = {}) {
    const deadline = now() + deadlineMs;
    for (let round = 0; now() < deadline; round += 1) {
        await sleep(DEBUG_CAP_MS + 20 + (round * 17) % 50);
        const [debug0, debug1] = await Promise.all([view.bridge("turts_debug", 1), view.bridge("turts_debug", 2)]);
        const world0 = debug0?.replicated_world;
        const world1 = debug1?.replicated_world;
        if (world0?.tick === world1?.tick && world0?.tick !== null && world0?.tick !== undefined &&
            JSON.stringify(project(debug0)) === JSON.stringify(project(debug1))) {
            return { tick: world0.tick };
        }
    }
    return null;
}

function ingestOf(step) {
    return step.debug?.ingest ?? {};
}

async function judgeRecovery(view, record, records, joined) {
    const steps = record.steps;
    const harvester = unitNear(joined.debug, "base:harvester", 12 + OFFSET.base, 8);
    assert(harvester, "base recovery fixture lacks its owner-0 Harvester or ore node");
    const before = notTimedOut(steps.before, "the state before the loss configuration");
    const initialUnit = replicaUnit(before.debug, harvester.unit_id);
    const initialNode = (before.debug.replicated_world?.nodes ?? []).find((node) => Math.abs(node.pos.x - (24 + OFFSET.base)) < 1e-3 && Math.abs(node.pos.y - 8) < 1e-3);
    assert(initialUnit && Array.isArray(initialUnit.pos), "recovery worker lacks its actual initial replicated position");
    assert(initialNode, "recovery proof node lacks its initial actual replicated amount");
    assert(hasSingleDebugSelection(before, harvester.unit_id),
        "recovery proof requires the Harvester selected through the actual rendered UI before loss configuration");
    // m2.js:796-797 read the panel text "Selected: Harvester"; the plain HUD shows the selection ring.
    assert((before.debug.rings_shown ?? []).some((ring) => ring.unit === harvester.unit_id && ring.own === true),
        "recovery proof requires the actual visible Harvester selection ring");
    // m2.js:798.
    assert.equal(appliedCommandCount(before), appliedCommandCount(joined), "selecting the recovery worker must not issue a PlayerCommand");
    const armed = notTimedOut(steps.armed, "the loss configuration");
    assert.equal(appliedCommandCount(armed), appliedCommandCount(before), "configuring a loss must not issue a PlayerCommand");
    assert.equal(ingestOf(armed).max_buffered_batches, 3, "browser cap must update the already joined Replica");
    assert.deepEqual(ingestOf(armed).armed_drop_world_changes?.kinds, ["extraction"]);

    const overflowed = notTimedOut(steps.overflowed ?? steps.held, "the held overflow");
    notTimedOut(steps.held, "the held Snapshot");
    assert(appliedCommandCount(overflowed) > appliedCommandCount(before), "real Gather command before intentional extraction loss");
    const held = ingestOf(overflowed);
    assert(held.buffer_overflows > 0, "real cap=3 loss did not produce an observed buffer overflow");
    assert(held.buffered_batches <= 3, "retained real delta tail exceeded configured cap=3");
    assert.equal(held.max_outstanding_requests, 1, "only one actual resync request may be outstanding at a time");
    // The time of a second request depends on the run: before the overflow (m2.js:810 in
    // t2b-refutations-run30) or only at the release (t21-run118). One check over every
    // recorded step gives one failure text for both cases. The later checks stay.
    const outstandingPeak = Math.max(0, ...Object.values(steps).flat().map((sample) => ingestOf(sample).max_outstanding_requests ?? 0));
    assert.equal(outstandingPeak, 1, "only one actual resync request may be outstanding at a time");
    assert.equal(held.held_snapshots, 1, "only one actual resync response may remain held for one outstanding request");
    assert.equal(held.max_buffered_batches, 3, "the real retained delta tail never exceeds the configured cap");
    assert.equal(held.outstanding_request, true);
    const requestsDuringHold = held.resync_requests_sent;
    for (const sample of steps.hold_window ?? []) {
        assert.equal(ingestOf(sample).resync_requests_sent, requestsDuringHold, "only one actual resync request may be outstanding at a time");
        assert.equal(ingestOf(sample).max_outstanding_requests, 1);
    }
    const originalArm = (overflowed.debug.dropped_world_changes ?? []).find((drop) => drop.kind === "extraction");
    assert(originalArm, "actual extraction delta was not dropped by the browser-armed predicate");
    const firstLoss = lossTransitionAt("extraction", originalArm, records, 0);

    const heldAppliedTick = overflowed.debug.replicated_world.tick;
    const release = notTimedOut(steps.release, "the release configuration");
    assert(release.debug.replicated_world.tick >= heldAppliedTick, "release configuration must not roll back the applied replica tick");
    assert.equal(release.status.arm_tick, heldAppliedTick,
        "same-kind re-arm while the first response is held must anchor to the current coherent applied tick");
    assert.equal(appliedCommandCount(release), appliedCommandCount(overflowed),
        "reconfiguration and held Snapshot application must not allocate a command sequence");
    const recovered = ingestOf(release).held_snapshots === 0 && !ingestOf(release).outstanding_request ? release : notTimedOut(steps.recovered, "the recovery");
    assert.equal(ingestOf(recovered).successor_requests, 1, "one successor request must be observed only after the prior held response was released");
    assert.equal(ingestOf(recovered).max_outstanding_requests, 1, "the real browser Replica must never have more than one outstanding request");
    assert.equal((recovered.debug.dropped_world_changes ?? []).filter((drop) => drop.kind === "extraction" && drop.arm_tick === originalArm.arm_tick).length, 1,
        "the first configured extraction loss must occur exactly once");
    assert.equal(ingestOf(recovered).resync_active, false, "coherent held response must close recovery");
    const recoveryAudit = compareBrowserSnapshotToAudit(recovered, records, 0, "extraction-recovery-observation");
    const releasedWorker = replicaUnit(recovered.debug, harvester.unit_id);
    assert(releasedWorker, "released full Snapshot omitted the active Harvester");
    assert(Math.hypot(releasedWorker.pos[0] - initialUnit.pos[0], releasedWorker.pos[1] - initialUnit.pos[1]) > 0.25,
        "released Snapshot did not converge the Harvester's moved position");
    assert(inventoryOf(recovered.debug, harvester.unit_id) > 0, "released full Snapshot did not converge the Harvester inventory");
    const releasedNode = (recovered.debug.replicated_world.nodes ?? []).find((node) => node.id === initialNode.id);
    assert(releasedNode && releasedNode.remaining < initialNode.remaining, "released full Snapshot did not converge the actual node depletion");

    const rearmTick = release.status.arm_tick;
    assert.equal(rearmTick, heldAppliedTick, "same-kind re-arm must anchor to the applied authoritative tick");
    const active = release.debug.ingest?.armed_drop_world_changes?.arm_tick === rearmTick;
    const consumed = (release.debug.dropped_world_changes ?? []).some((drop) => drop.kind === "extraction" && drop.arm_tick === rearmTick && drop.dropped_tick > rearmTick);
    assert(active || consumed, "same-kind re-arm must install a fresh matcher at its applied authoritative tick");
    const redropped = notTimedOut(steps.redropped, "the re-armed loss");
    const redrop = (redropped.debug.dropped_world_changes ?? []).find((drop) => drop.kind === "extraction" && drop.arm_tick === rearmTick);
    assert(redrop && redrop.dropped_tick > redrop.arm_tick, "re-armed extraction must drop a later tick exactly once");
    const secondLoss = lossTransitionAt("extraction", redrop, records, 0);
    assert.notEqual(rearmTick, originalArm.arm_tick, "the re-armed extraction must use a distinct arm tick");
    const final = notTimedOut(steps.rearm_recovered, "the re-armed recovery");
    const drops = (final.debug.dropped_world_changes ?? []).filter((drop) => drop.kind === "extraction");
    assert.equal(drops.length, 2, "the scenario must observe exactly the original and re-armed extraction loss");
    assert.deepEqual(drops.map((drop) => drop.arm_tick), [originalArm.arm_tick, rearmTick]);
    assert.equal(ingestOf(final).resync_requests_sent, requestsDuringHold + 2,
        "the held response successor and the later re-armed loss must each issue exactly one request");
    assert.equal(ingestOf(final).successor_requests, 1, "the first held response must cause exactly one successor request");
    assert.equal(ingestOf(final).max_outstanding_requests, 1, "the real browser Replica must never have more than one outstanding request");
    const rearmAudit = compareBrowserSnapshotToAudit(final, records, 0, "extraction-rearm-recovery");
    return { first_loss_tick: firstLoss.after.economy.tick, rearm_loss_tick: secondLoss.after.economy.tick, recoveryAudit: recoveryAudit.tick, rearmAudit: rearmAudit.tick };
}

// Click helpers for the plain HUD lists (ruling C384). The driver saves `<label>-button.json`
// and then clicks; the checkpoint of its last screenshot before the click holds the rows.
function hudClick(input, checkpoints, label) {
    const saveIndex = input.findIndex((entry) => entry.call === "save" && entry.args?.[0] === `${label}-button.json`);
    assert(saveIndex >= 0, `${label}: the driver did not check the HUD button before the click`);
    const clickIndex = input.findIndex((entry, index) => index > saveIndex && entry.call === "click" && entry.args?.[2] === "left");
    assert(clickIndex > saveIndex, `${label}: no click after the HUD button check`);
    const before = checkpointBefore(checkpoints, clickIndex, 1);
    assert(before?.debug, `${label}: no checkpoint before the HUD click`);
    const [x, y] = input[clickIndex].args;
    return { before, clickIndex, x, y };
}

function rowAt(debug, x, y, predicate) {
    const scale = 1;
    return widgets(debug, (row) => row.kind === "button" && Array.isArray(row.rect) && predicate(row) &&
        x * scale >= row.rect[0] && x * scale <= row.rect[0] + row.rect[2] && y * scale >= row.rect[1] && y * scale <= row.rect[1] + row.rect[3])[0] ?? null;
}

function produceRow(debug, producerId, recipe) {
    return widgets(debug, (row) => row.kind === "button" && row.id.startsWith("m2_build_rows/") && row.enabled === true &&
        row.widget_action?.kind === "produce" && row.widget_action.producer === producerId &&
        (recipe === null || row.widget_action.content_id === recipe))[0] ?? null;
}

function cancelRows(debug, producerId) {
    return widgets(debug, (row) => row.kind === "button" && row.id === "m2_job_cancel" &&
        row.widget_action?.kind === "cancel_production" && row.widget_action.producer === producerId);
}

function newJobId(debug, appliedBefore) {
    const applied = (debug.command_results ?? []).filter((result) => Number.isInteger(result.applied_tick));
    return applied.length > appliedBefore ? applied.at(-1).job_id ?? null : null;
}

function queueOf(debug, producerId) {
    return debug.replicated_world?.economy?.queues?.find((queue) => queue.producer === producerId) ?? null;
}

function credits(debug) {
    return (debug.replicated_world?.economy?.balances ?? []).find((balance) => balance.resource === "base:credits")?.on_hand ?? null;
}

// Produce through the plain HUD: an enabled row of the recipe for the producer is under the
// click, the server applies the order, and the job shows in the queue rows.
// The first later snapshot (a checkpoint after the click, or a harness step) in which the server
// applied one more order; m2.js waited 5 s for it.
function appliedAfter(checkpoints, click, extra = []) {
    const appliedBefore = appliedCommandCount(click.before);
    const clickTime = click.before.wall_time_ms ?? 0;
    const later = [...checkpoints.filter((shot) => shot.window === 1 && shot.input_call > click.clickIndex)
        .sort((left, right) => left.input_call - right.input_call), ...extra];
    return later.find((shot) => appliedCommandCount(shot) > appliedBefore && (shot.wall_time_ms ?? clickTime) - clickTime <= 6000) ?? null;
}

function judgeProduce(input, checkpoints, label, producerId, recipe, message, extra = []) {
    const click = hudClick(input, checkpoints, label);
    const row = produceRow(click.before.debug, producerId, recipe);
    assert(row, message);
    assert(rowAt(click.before.debug, click.x, click.y, (item) => item.id === row.id), "the real visible production control was not clicked");
    const queued = appliedAfter(checkpoints, click, extra);
    assert(queued, "production did not apply within 5 s of the click");
    const jobId = newJobId(queued.debug, appliedCommandCount(click.before));
    assert(Number.isSafeInteger(jobId), "applied production result did not expose its JobId");
    return { click, queued, jobId };
}

function judgeCancel(input, checkpoints, label, producerId, jobId) {
    const click = hudClick(input, checkpoints, label);
    const control = rowAt(click.before.debug, click.x, click.y, (row) => row.id === "m2_job_cancel");
    assert(control && control.text === "Cancel" && control.widget_action?.kind === "cancel_production" &&
        control.widget_action.producer === producerId && control.widget_action.job === jobId,
    `cancel must target the exact applied JobId ${jobId}`);
    return click;
}

async function judgeEconomy(view, input, checkpoints, at, joined) {
    const producer = unitNear(joined.debug, "base:command_center", 8 + OFFSET.base, 8);
    const selected = at("m2-producer-selected");
    assert(produceRow(selected.debug, producer.unit_id, "base:train_rifleman"), "no visible enabled rendered Produce control for the selected producer");
    const first = judgeProduce(input, checkpoints, "m2-econ-first", producer.unit_id, "base:train_rifleman",
        "no visible enabled rendered Produce control for the selected producer");
    const initial = new Set((joined.debug.replicated_world.units ?? []).map((unit) => unit.unit_id));
    const outputs = input.filter((entry) => entry.call === "save" && /^m2-econ-output-\d+\.png$/.test(entry.args?.[0] ?? ""))
        .map((entry) => checkpointForSave(input, checkpoints, entry.args[0], "M2"));
    const output = [...outputs].reverse().find((shot) => (shot.debug.replicated_world.units ?? []).some((unit) =>
        unit.content_id === "base:rifleman" && !initial.has(unit.unit_id)));
    assert(output, "actual completed rifleman output and rendered mesh");
    const riflemen = output.debug.replicated_world.units.filter((unit) => unit.content_id === "base:rifleman" && !initial.has(unit.unit_id));
    assert.equal(riflemen.length, 1, "exactly one new rifleman must appear in received state");
    assert.equal(output.scene.units.filter((unit) => unit.unit_id === riflemen[0].unit_id).length, 1, "the completed output must have exactly one rendered mesh");
    const before = credits(output.debug);
    assert.equal(before, 400, "first production should have charged the declared 100-credit cost");
    const second = judgeProduce(input, checkpoints, "m2-econ-second", producer.unit_id, "base:train_rifleman",
        "no visible enabled rendered Produce control for the selected producer");
    // The last screenshot before the Cancel click shows the row that the driver clicked.
    const queueShot = hudClick(input, checkpoints, "m2-econ-cancel").before;
    const job = queueOf(queueShot.debug, producer.unit_id)?.jobs?.find((item) => item.id === second.jobId);
    assert(job, `second JobId ${second.jobId} is not rendered in the visible queue`);
    assert(cancelRows(queueShot.debug, producer.unit_id).some((row) => row.widget_action.job === second.jobId),
        `second JobId ${second.jobId} is not rendered in the visible queue`);
    assert(["queued", "running", "output_blocked"].includes(job.stage), `unknown rendered job stage: ${job.stage}`);
    const refund = job.stage === "running" ? 50 : 100;
    const expected = before - 100 + refund;
    const cancel = judgeCancel(input, checkpoints, "m2-econ-cancel", producer.unit_id, second.jobId);
    const appliedAtCancel = appliedCommandCount(cancel.before);
    const later = checkpoints.filter((shot) => shot.window === 1 && shot.input_call > cancel.clickIndex &&
        shot.wall_time_ms - cancel.before.wall_time_ms <= 6000);
    const done = later.find((shot) => appliedCommandCount(shot) > appliedAtCancel &&
        !queueOf(shot.debug, producer.unit_id)?.jobs?.some((item) => item.id === second.jobId) &&
        !cancelRows(shot.debug, producer.unit_id).some((row) => row.widget_action.job === second.jobId) &&
        credits(shot.debug) === expected);
    assert(done, `exact ${job.stage} refund should yield ${expected} Credits`);
    const harvester = at("m2-worker-selected");
    const selectClick = input.findIndex((entry, index) => index > at("m2-econ-cancelled").screenshotIndex && entry.call === "click" && entry.args?.[2] === "left");
    const beforeSelect = checkpointBefore(checkpoints, selectClick, 1);
    assert(beforeSelect && appliedCommandCount(harvester) === appliedCommandCount(beforeSelect),
        "selecting a Harvester must not advance the applied command sequence");
    return { first_job_id: first.jobId, output_unit: riflemen[0].unit_id, second_job_id: second.jobId, stage: job.stage, final_credits: expected };
}

function judgeProductionCancel(input, checkpoints, at, joined, packageName) {
    const producer = unitNear(joined.debug, PRODUCER[packageName], 8 + OFFSET[packageName], 8);
    assert(produceRow(at("m2-producer-selected").debug, producer.unit_id, null), `${packageName} producer has no visible enabled Produce control`);
    const queued = judgeProduce(input, checkpoints, "m2-prod", producer.unit_id, null,
        `${packageName} producer has no visible enabled Produce control`);
    assert(cancelRows(hudClick(input, checkpoints, "m2-prod-cancel").before.debug, producer.unit_id).some((row) => row.widget_action.job === queued.jobId),
        `${packageName} Job ${queued.jobId} is absent from actual visible queue text`);
    const cancel = judgeCancel(input, checkpoints, "m2-prod-cancel", producer.unit_id, queued.jobId);
    const cancelled = appliedAfter(checkpoints, cancel);
    assert(cancelled && !queueOf(cancelled.debug, producer.unit_id)?.jobs?.some((job) => job.id === queued.jobId),
        `${packageName} applied exact-job cancellation`);
}

// The lost-delta families of m2.js runParameterLossProof.
async function judgeLoss(view, input, checkpoints, at, kind, record, records, joined) {
    const steps = record.steps;
    const needsGather = kind === "conversion";
    const target = needsGather ? unitNear(joined.debug, "base:harvester", 12 + OFFSET.base, 8)
        : unitNear(joined.debug, "base:command_center", 8 + OFFSET.base, 8);
    assert(target, `${kind} loss fixture lacks its real owner-0 target mesh`);
    const before = notTimedOut(steps.before, "the selected target");
    assert.deepEqual(before.debug.selection, [target.unit_id], `${kind} target selection uses actual Bevy UI`);
    assert.equal(appliedCommandCount(before), appliedCommandCount(joined), `${kind} selection must not send a PlayerCommand`);
    let jobId = null;
    if (kind === "refund" || kind === "final_change") {
        const produced = judgeProduce(input, checkpoints, "m2-loss-produce", target.unit_id, "base:train_rifleman",
            `${kind} loss fixture Command Center has no actual enabled Produce control`, [steps.queued].filter(Boolean));
        jobId = produced.jobId;
        assert(Number.isSafeInteger(jobId), `${kind} actual applied Produce must identify its JobId`);
        const queueShot = at("m2-loss-queue");
        assert(cancelRows(queueShot.debug, target.unit_id).some((row) => row.widget_action.job === jobId), `${kind} real JobId ${jobId} must appear in rendered queue text`);
        if (kind === "refund") {
            const job = queueOf(queueShot.debug, target.unit_id)?.jobs?.find((item) => item.id === jobId);
            assert(job && ["queued", "running"].includes(job.stage), "refund loss requires the exact live job to remain cancelable");
        } else {
            const blocked = notTimedOut(steps.blocked, "the output_blocked job");
            notTimedOut(steps.blocked_stable, "the stable output_blocked interval");
            assert.equal(appliedCommandCount(blocked), appliedCommandCount(produced.queued), "waiting output_blocked sends no command");
            const blockedTick = blocked.debug.replicated_world.tick;
            const stableA = projectAuditOwnerWorld(auditStateAt(records, blockedTick + 1), 0);
            const stableB = projectAuditOwnerWorld(auditStateAt(records, blockedTick + 2), 0);
            delete stableA.tick;
            delete stableB.tick;
            assert.deepStrictEqual(stableA, stableB, "output_blocked fixture must have a completed unchanged owner economy interval before arming");
            assert.equal(auditStateAt(records, blockedTick + 2).economy.economy.queues.find((queue) => queue.producer === target.unit_id)?.jobs
                .find((job) => job.id === jobId)?.stage, "OutputBlocked");
        }
        judgeCancel(input, checkpoints, "m2-loss-cancel", target.unit_id, jobId);
    }
    const armed = notTimedOut(steps.armed, "the loss configuration");
    const beforeArm = notTimedOut(steps.blocked_stable ?? steps.queued ?? before, "the state before the loss configuration");
    assert.equal(appliedCommandCount(armed), appliedCommandCount(beforeArm), `${kind} loss config must not allocate a PlayerCommand`);
    assert.equal(ingestOf(armed).max_buffered_batches, 3, `${kind} loss must set the live cap to three`);
    assert.deepEqual(ingestOf(armed).armed_drop_world_changes?.kinds, [kind]);
    assert.equal(armed.status?.state, "applied", `${kind} arm must apply on the Bevy main thread`);
    const armTick = armed.status.arm_tick;
    assert(Number.isInteger(armTick), `${kind} live ingest arm must publish its applied authoritative tick`);
    if (kind === "charge" || kind === "spawn") {
        judgeProduce(input, checkpoints, "m2-loss-produce", target.unit_id, "base:train_rifleman",
            `${kind} loss fixture Command Center has no actual enabled Produce control`, [steps.dropped, steps.held].filter(Boolean));
    }
    const dropped = notTimedOut(steps.dropped, `the ${kind} loss`);
    if (needsGather) assert(appliedCommandCount(dropped) > appliedCommandCount(armed), `${kind} real right-pointer Gather application`);
    const matching = (dropped.debug.dropped_world_changes ?? []).filter((drop) => drop.kind === kind && drop.arm_tick === armTick);
    assert.equal(matching.length, 1, `${kind} fixture must lose exactly one matching delta`);
    const drop = matching[0];
    const transition = lossTransitionAt(kind, drop, records, 0);
    if (kind === "charge") {
        const previousJobs = transition.before.economy.economy.queues.find((queue) => queue.producer === target.unit_id)?.jobs ?? [];
        const chargedJobs = transition.after.economy.economy.queues.find((queue) => queue.producer === target.unit_id)?.jobs ?? [];
        const charged = chargedJobs.filter((job) => job.charged.some((line) => line.source === "SlotBalance" && line.resource.package === "base" &&
            line.resource.local === "credits" && line.amount === 100) && !previousJobs.some((previous) => previous.id === job.id));
        assert.equal(charged.length, 1, "charge loss audit tick must add one actually charged owner job");
    }
    let spawned = null;
    if (kind === "spawn") {
        const beforeIds = new Set(transition.before.economy.units.map((unit) => unit.unit_id));
        spawned = transition.after.economy.units.find((unit) => unit.owner_slot === 0 && !beforeIds.has(unit.unit_id));
        assert(spawned, "spawn loss audit tick must contain the real new owner unit");
    }
    const held = notTimedOut(steps.held, "the held response");
    assert.equal(ingestOf(held).max_buffered_batches, 3);
    assert(ingestOf(held).buffered_batches <= 3, "retained real delta tail exceeded configured cap=3");
    assert.equal(ingestOf(held).max_outstanding_requests, 1, "the real browser Replica must never have more than one outstanding request");
    const requestsDuringHold = ingestOf(held).resync_requests_sent;
    for (const sample of steps.hold_window ?? []) {
        assert.equal(ingestOf(sample).resync_requests_sent, requestsDuringHold,
            `${kind} must not issue another resync request while the actual response is held`);
        assert.equal(ingestOf(sample).max_outstanding_requests, 1, "only one actual resync request may be outstanding at a time");
    }
    const release = notTimedOut(steps.release, "the release");
    assert.equal(appliedCommandCount(release), appliedCommandCount(held), `${kind} release must not allocate a command`);
    const recovered = notTimedOut(steps.recovered, "the recovery");
    assert.equal(ingestOf(recovered).resync_requests_sent, requestsDuringHold + 1,
        `${kind} recovery must send one successor after releasing the held response`);
    assert.equal(ingestOf(recovered).max_outstanding_requests, 1);
    if (spawned) {
        notTimedOut(steps.spawn_output, "the spawned output");
        const outputShot = at("m2-spawn-output");
        assert((outputShot.debug.replicated_world.units ?? []).some((unit) => unit.unit_id === spawned.unit_id) &&
            outputShot.scene.units.some((unit) => unit.unit_id === spawned.unit_id), "spawned output did not render after recovery closed");
    }
    let final = recovered;
    if (kind === "final_change") {
        final = notTimedOut(steps.final, "the final heartbeats");
        assert(ingestOf(final).highest_delta_tick >= final.debug.replicated_world.tick,
            "empty-heartbeat recovery must show continuing received ticks through the applied tick");
    }
    compareBrowserSnapshotToAudit(final, records, 0, `${kind}-recovered`);
    if (kind === "final_change") {
        const actualTick = final.debug.replicated_world.tick;
        // m2.js read the audit four ticks past the browser tick; the oracle reads it after the run.
        const observationTick = actualTick + 4;
        assert(lastAuditTick(records) >= observationTick, `trusted economy audit did not complete tick ${observationTick}`);
        assertFinalChangeQuiescence(records, drop.dropped_tick, actualTick, observationTick, 0);
        const beforeDrop = auditStateAt(records, drop.dropped_tick - 1);
        const afterDrop = auditStateAt(records, drop.dropped_tick);
        assert(worldChangeMatchesAudit("refund", beforeDrop, afterDrop, 0), "terminal final_change must be the actual exact-job cancellation/refund mutation");
        assert.equal(afterDrop.economy.economy.queues.some((queue) => queue.producer === target.unit_id && queue.jobs.some((job) => job.id === jobId)), false,
            "terminal final_change audit state must remove the exact canceled JobId");
    }
}

async function judgeCase(view, params) {
    const caseId = params.m2?.case_id;
    const packageName = params.m2?.package;
    assert(caseId && packageName, "M2 oracle needs params.m2.case_id and params.m2.package");
    const input = view.inputLog();
    const checkpoints = await readCheckpoints(view);
    const at = (file) => checkpointForSave(input, checkpoints, `${file}.png`, `M2 ${caseId}`);
    // The packed map copy is the source map moved by the declared offset (tests/m2/browser_fixture.py).
    validateTranslationReceipt(packageName, caseId, String(OFFSET[packageName]),
        JSON.parse(await view.serverFile("map-fixture-translation.json")));
    const joined = at("m2-joined");
    assertJoined(joined, packageName);
    assertPublicProjectionSchema(joined);
    if (caseId === "E9-browser-base-recovery" || caseId.startsWith("E9-browser-base-loss-")) {
        const record = JSON.parse(await view.serverFile("m2-ingest-private.json"));
        const records = await readAuditRecords(view.serverLines("economy-audit-private.jsonl"));
        if (caseId.endsWith("recovery")) return judgeRecovery(view, record, records, joined);
        return judgeLoss(view, input, checkpoints, at, caseId.slice("E9-browser-base-loss-".length), record, records, joined);
    }
    if (caseId.endsWith("-public-convergence")) return judgeConvergence(view, at, joined, packageName);
    await judgeSelection(view, at, joined, packageName);
    if (caseId === "E9-browser-base-economy") {
        await judgeEconomy(view, input, checkpoints, at, joined);
        judgeGather(input, checkpoints, at, joined, packageName);
    }
    if (caseId === "E9-browser-haul-economy" || caseId === "E9-browser-cedar-economy") {
        judgeProductionCancel(input, checkpoints, at, joined, packageName);
        judgeGather(input, checkpoints, at, joined, packageName);
    }
    return null;
}

async function judge(view, params) {
    try {
        const result = await judgeCase(view, params);
        // The run checks first, then the driver stop record (RUN123 L2).
        await assertNoDriverStop(view, `M2 ${params.m2?.case_id ?? "unknown"}`);
        return result;
    } catch (error) {
        if (error instanceof assert.AssertionError) error.task16_semantic = { row: "M2", clause: params.m2?.case_id ?? "unknown" };
        throw error;
    }
}

module.exports = { judge, appliedAfter, assertPickCircleClearOfHud, cancelRows, hudClick, produceRow, sampleSharedTick, UNIT_NAMES };
