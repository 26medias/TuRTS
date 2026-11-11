// turts-test-role: oracle
"use strict";

// M2 browser oracle (MC design 13.3; replaces the asserts of tests/browser/m2.js). It reads the
// harness checkpoints of the m2-driver.js screenshots, the harness record of the lost-delta
// fault plans (m2-ingest-private.json), the trusted economy log of base, and, for the
// public-convergence cases, the live read-only bridges of both windows. An assert failure is a
// semantic failure of the case: scripts/m2-browser.sh turns it into M2_CASE_SEMANTIC_FAIL.
const assert = require("node:assert/strict");
const { decodePng } = require("./harness/png.js");
const { checkpointForSave, readCheckpoints, replicaUnit, widgets } = require("./checkpoint-lib.js");
const {
    appliedCommandCount,
    assertPublicProjectionSchema,
    compareBrowserSnapshotToAudit,
    hasSingleDebugSelection,
    lossTransitionAt,
    parseCompleteAuditText,
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
    assert(appliedCommandCount(command) > appliedCommandCount(selected), `${packageName} real Gather command and rendered status`);
    const later = input.map((entry) => entry).filter((entry) => entry.call === "save" && /^m2-gather-\d+\.png$/.test(entry.args?.[0] ?? ""))
        .map((entry) => checkpointForSave(input, checkpoints, entry.args[0], "M2"));
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
    const deadline = Date.now() + 10000;
    while (Date.now() < deadline) {
        const debug0 = await view.bridge("turts_debug", 1);
        const debug1 = await view.bridge("turts_debug", 2);
        const world0 = debug0?.replicated_world;
        const world1 = debug1?.replicated_world;
        if (world0?.tick === world1?.tick && world0?.tick !== null && world0?.tick !== undefined &&
            JSON.stringify(publicProjection({ debug: debug0 })) === JSON.stringify(publicProjection({ debug: debug1 }))) {
            return { tick: world0.tick };
        }
        await new Promise((resolve) => setTimeout(resolve, 50));
    }
    assert.fail("public world projections differ between owners at a shared coherent tick");
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

async function judgeConversion(view, record, records, joined) {
    const kind = "conversion";
    const steps = record.steps;
    const worker = unitNear(joined.debug, "base:harvester", 12 + OFFSET.base, 8);
    assert(worker, `${kind} loss fixture lacks its real owner-0 target mesh`);
    const before = notTimedOut(steps.before, "the selected Harvester");
    assert.deepEqual(before.debug.selection, [worker.unit_id], `${kind} target selection uses actual Bevy UI`);
    assert.equal(appliedCommandCount(before), appliedCommandCount(joined), `${kind} selection must not send a PlayerCommand`);
    const armed = notTimedOut(steps.armed, "the loss configuration");
    assert.equal(appliedCommandCount(armed), appliedCommandCount(before), `${kind} loss config must not allocate a PlayerCommand`);
    assert.equal(ingestOf(armed).max_buffered_batches, 3, `${kind} loss must set the live cap to three`);
    assert.deepEqual(ingestOf(armed).armed_drop_world_changes?.kinds, [kind]);
    assert.equal(armed.status?.state, "applied", `${kind} arm must apply on the Bevy main thread`);
    const armTick = armed.status.arm_tick;
    assert(Number.isInteger(armTick), `${kind} live ingest arm must publish its applied authoritative tick`);
    const dropped = notTimedOut(steps.dropped, "the conversion loss");
    assert(appliedCommandCount(dropped) > appliedCommandCount(armed), `${kind} real right-pointer Gather application`);
    const matching = (dropped.debug.dropped_world_changes ?? []).filter((drop) => drop.kind === kind && drop.arm_tick === armTick);
    assert.equal(matching.length, 1, `${kind} fixture must lose exactly one matching delta`);
    lossTransitionAt(kind, matching[0], records, 0);
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
    compareBrowserSnapshotToAudit(recovered, records, 0, `${kind}-recovered`);
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
    if (caseId === "E9-browser-base-recovery" || caseId === "E9-browser-base-loss-conversion") {
        const record = JSON.parse(await view.serverFile("m2-ingest-private.json"));
        const records = parseCompleteAuditText(await view.serverFile("economy-audit-private.jsonl"));
        return caseId.endsWith("recovery") ? judgeRecovery(view, record, records, joined) : judgeConversion(view, record, records, joined);
    }
    if (caseId.endsWith("-public-convergence")) return judgeConvergence(view, at, joined, packageName);
    await judgeSelection(view, at, joined, packageName);
    if (caseId === "E9-browser-haul-economy" || caseId === "E9-browser-cedar-economy") judgeGather(input, checkpoints, at, joined, packageName);
    return null;
}

async function judge(view, params) {
    try {
        return await judgeCase(view, params);
    } catch (error) {
        if (error instanceof assert.AssertionError) error.task16_semantic = { row: "M2", clause: params.m2?.case_id ?? "unknown" };
        throw error;
    }
}

module.exports = { judge, assertPickCircleClearOfHud, UNIT_NAMES };
