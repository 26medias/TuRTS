// turts-test-role: oracle
"use strict";

const assert = require("node:assert/strict");
const { decodePng, pixelAt } = require("./harness/png.js");
const { unprojectGround } = require("./hud4-oracle.js");
const { assertConnectionHeld, assertNoResync, readServerEvidence } = require("./checkpoint-lib.js");

async function judge(view, params) {
    const input = view.inputLog();
    const checkpoints = JSON.parse(await view.serverFile("scene-checkpoints.json"));
    // A run whose client lost its connection is red here, with its cause told from the evidence of the run (host
    // fault or slow server tick), not at a later order check that shows a stale replica (run
    // rereview-2df2393-hold-a: ORD4 red 2 of 2 at two different checks after stalls of 11 s and 18 s).
    const evidence = await readServerEvidence(view);
    assertConnectionHeld(checkpoints, evidence, params.row);
    // ORD4 always kills its target (the accepted Attack runs until the target is removed). A replica that asked for a
    // resync stalled after that kill (RUN142, repair 50eb827): red, with the first count and the nearest removal.
    if (params.row === "ORD4") assertNoResync(checkpoints, "ORD4", evidence);
    const steps = JSON.parse(await view.serverFile(`${params.row.toLowerCase()}-driver-steps.json`));
    const context = makeContext(view, input, checkpoints, steps, params);
    const before = await context.shot("before");
    const own = identifyUnit(before.checkpoint, steps.pixels.trooper, 0);
    assert.equal(own.unit.content_id, "muster:trooper", "actual screenshot ray identifies the selected exposed trooper");
    const selected = await context.shot("selected");
    assert((selected.checkpoint.debug.selection || []).includes(own.id), "pixel-selected trooper is the actual selected identity");
    if (params.row === "ORD3") return judgeOrd3(context, params, own);
    if (params.row === "ORD4") {
        assertNoUncommandedOrders(context, [own.id], [
            ["attack-selected-unit", "muster:order_attack", [own.id]], ["rush-target", "muster:rush", [own.id]],
            ["force-safe-approach", "muster:order_move", [own.id]], ["force-move-destination", "muster:order_move", [own.id]],
        ]);
        return judgeOrd4(context, params, own);
    }
    if (params.row === "ORD5") return judgeOrd5(context, params, own);
    throw new Error(`order oracle has no route for ${params.row}`);
}

async function judgeOrd5(ctx, params, trooper) {
    const worker = identifyUnit((await ctx.shot("before")).checkpoint, stepsWorker(ctx.steps), 0);
    assert.equal(worker.unit.content_id, "muster:worker", "screenshot-seeded pixel binds the actual own worker identity");
    const selected = await ctx.shot("selected");
    assert.deepEqual(new Set(selected.checkpoint.debug.selection), new Set([worker.id, trooper.id]),
        "one physical Shift selection retains exactly the actual worker and trooper IDs");
    assertNoUncommandedOrders(ctx, [worker.id, trooper.id], [
        ["smart-ground", "muster:order_move", [worker.id, trooper.id]], ["smart-node", "muster:order_move", [trooper.id]],
        ["smart-node", "muster:order_gather", [worker.id]], ["smart-foreign", "muster:order_move", [worker.id]],
        ["smart-foreign", "muster:order_attack", [trooper.id]],
    ]);
    const foreignAction = ctx.actionFor("smart-foreign");
    const foreignClick = ctx.inputFor("smart-foreign");
    const foreignWorld = minimapPoint(foreignAction, selected.checkpoint, params.map_size);
    const targetUnit = (selected.checkpoint.debug.replicated_world?.units || []).filter((unit) => unit.owner_slot === 1)
        .sort((a, b) => distance(a.pos, foreignWorld) - distance(b.pos, foreignWorld))[0];
    assert(targetUnit && distance(targetUnit.pos, foreignWorld) < 0.5,
        "screenshot-visible minimap target pixel lands inside the actual authored slot 1 unit footprint");

    const cases = [
        { name: "ground", target: { point: params.stages.ground }, expected: [[worker.id, "muster:order_move"], [trooper.id, "muster:order_move"]] },
        { name: "node", target: { node: 1, point: params.stages.node }, expected: [[worker.id, "muster:order_gather"], [trooper.id, "muster:order_move"]] },
        { name: "foreign", target: { unit: targetUnit.unit_id }, expected: [[worker.id, "muster:order_move"], [trooper.id, "muster:order_attack"]] },
    ];
    for (const item of cases) {
        const shot = await ctx.shot(`smart-${item.name}`);
        const action = `smart-${item.name}`;
        const click = item.name === "foreign" ? foreignClick : ctx.inputFor(action);
        assertPointer(click, "right", `ORD5 ${item.name} smart target`);
        const released = pointerEvent(click, "pointerup");
        assert.equal(released.shift_key, false, "smart command is one physical right click after the mixed selection");
        const orders = item.expected.map(([unit, order]) => assertOrder(shot.checkpoint.debug, released.seq, unit, order));
        assert(orders.every((entry) => entry.result.status === "accepted"), `both per-unit ORD5 ${item.name} orders are accepted`);
        const point = item.name === "node" ? params.stages.node : minimapPoint(ctx.actionFor(action), shot.checkpoint, params.map_size);
        for (const [unit, order] of item.expected) {
            const active = unitOrder(shot.checkpoint.debug, unit);
            assert.equal(active?.current?.order, order, `actual typed per-unit current state matches the ${item.name} resolver outcome`);
            assert(active?.current, `actual typed ${item.name} order is active for unit ${unit}`);
            if (item.name === "ground") assertPoint(active.current.target, point, "actual Move current target from minimap pixel");
            if (item.name === "node" && unit === worker.id) assert.equal(active.current.target?.Node ?? active.current.target?.node, 1,
                "only the worker receives the actual ore-node Gather target");
            if (item.name === "node" && unit === trooper.id) assertPoint(active.current.target, item.target.point,
                "trooper Move fallback resolves the ore click to the authored node point");
            if (item.name === "foreign") assert.equal(targetUnitId(active.current.target), targetUnit.unit_id,
                "actual active Unit target is preserved for each typed per-unit resolution");
        }
    }
}

async function judgeOrd3(ctx, params, own) {
    const stages = params.stages;
    // Bind repeated identical ShiftLeft actions monotonically to the actual input log.
    // Without this, a later modifier assertion can read an earlier route's key pair.
    for (const action of ctx.steps.actions.filter((entry) => entry.call === "key" && entry.code === "ShiftLeft"))
        ctx.inputForKey(action.label);
    const gatherContext = await ctx.shot("gather-worker-context");
    const worker = identifyUnit(gatherContext.checkpoint, stages.gather_worker_pixel, 0);
    assert.equal(worker.unit.content_id, "muster:worker", "screenshot ray binds the actual worker for Gather clauses");
    const workerBeforeGather = unitOrder(gatherContext.checkpoint.debug, worker.id);
    assert.equal(workerBeforeGather?.activity, "Idle", "the worker's actual published activity is Idle before Gather");
    assert.equal(workerBeforeGather?.current, null, "the worker has no current order before the physical Gather hotkey and target");
    assert.deepEqual(workerBeforeGather?.queued || [], [], "the worker has no queued order before Gather");
    assert.equal(workerBeforeGather?.last_failure, null, "the fresh worker has no prior order failure before Gather");
    const nodeBeforeGather = (gatherContext.checkpoint.debug.replicated_world?.nodes || []).find((node) => node.id === 1);
    assert.equal(nodeBeforeGather?.remaining, 2, "the authored ore node begins with the isolated positive two-unit test quantity");
    assertNoUncommandedOrders(ctx, [own.id, worker.id], ord3CommandLabels(own.id, worker.id));
    const routeQueued = await ctx.shot("route-queued");
    const routeOrders = [];
    for (let i = 0; i < 3; i += 1) {
        const click = ctx.inputFor(`route-append-${i + 1}`);
        assertPointer(click, "right", `ORD3 Append ${i + 1}`);
        assert.equal(pointerEvent(click, "pointerup").shift_key, true, "Shift is physically held through each appended release");
        routeOrders.push(assertOrder(routeQueued.checkpoint.debug, pointerEvent(click, "pointerup").seq, own.id, "muster:order_move"));
        assert.equal(routeOrders.at(-1).result.status, "accepted", `Append ${i + 1} is actually accepted`);
        const clickWorld = minimapPoint(ctx.actionFor(`route-append-${i + 1}`), routeQueued.checkpoint, params.map_size);
        assertPoint(routeOrders.at(-1).emitted.target, clickWorld, `emitted waypoint ${i + 1} from actual minimap input pixel`);
    }
    const routeWorlds = routeOrders.map((entry) => targetPoint(entry.emitted.target));
    const ticks = parseTickLog(await ctx.view.serverFile("server-tick.log"));
    assertCompleteTicks(ticks, own.id);
    const actorRows = ticks.filter((row) => row.unit === own.id);
    // The authoritative tick log says when the unit lands at A. A screenshot read after that tick
    // sees B as the current order. That is a host stall in the driver, not a queue fault.
    assertReadBeforeLanding(actorRows, routeWorlds[0], routeQueued.checkpoint.debug.replicated_world?.tick,
        "the route-queued checkpoint is read before the unit lands at A");
    const routed = unitOrder(routeQueued.checkpoint.debug, own.id);
    assert(routed?.current && routed.current.order === "muster:order_move", "first idle Append starts actual current Move A");
    assertPoint(routed.current.target, routeWorlds[0], "actual current A");
    assert.deepEqual((routed.queued || []).map((order) => order.target?.Point || order.target?.point),
        routeWorlds.slice(1).map(([x, y]) => ({ x, y })), "actual B/C queue follows A in order");

    const beforeQueueImage = beforePng(await ctx.shot("before"));
    const queuedImage = routeQueued.image;
    const lines = waypointSegments(routeQueued.checkpoint, own.id, routeWorlds);
    assert.equal(lines.length, 3, "renderer consumes the selected unit's actual current plus queued waypoints");
    const beforePixels = countUniqueLinePixels(beforeQueueImage, routeQueued.checkpoint.scene, lines);
    const queuedPixels = countUniqueLinePixels(queuedImage, routeQueued.checkpoint.scene, lines);
    assert(beforePixels.total === 0 && queuedPixels.perSegment.every((count) => count >= 3)
        && queuedPixels.total >= 12,
        "actual Gizmos waypoint lines render along each projected segment interior; the pre-queue PNG is negative");

    const aTick = actorRows.find((row) => close([row.x, row.y], routeWorlds[0], 0.001))?.tick;
    assert(Number.isSafeInteger(aTick), "authoritative tick log records exact landing at A");
    requireCompleteTickInterval(ticks, own.id, routeOrders[0].result.tick, aTick);
    assert(actorRows.some((row) => row.tick === aTick + 1 && distance([row.x, row.y], routeWorlds[0]) > 0.01
        && dot([row.x - routeWorlds[0][0], row.y - routeWorlds[0][1]],
            [routeWorlds[1][0] - routeWorlds[0][0], routeWorlds[1][1] - routeWorlds[0][1]]) > 0),
        "actual Movement begins toward B in exactly t+1 after A landing");
    const bTick = actorRows.find((row) => row.tick > aTick + 1 && close([row.x, row.y], routeWorlds[1], 0.001))?.tick;
    const cTick = actorRows.find((row) => Number.isSafeInteger(bTick) && row.tick > bTick && close([row.x, row.y], routeWorlds[2], 0.001))?.tick;
    assert(Number.isSafeInteger(bTick) && Number.isSafeInteger(cTick) && aTick < bTick && bTick < cTick,
        "tick log reaches actual waypoint A, then B, then C in order");
    assert(routeOrders[1].result.tick <= aTick && routeOrders[2].result.tick <= aTick,
        "actual B and C Append results arrive before the server tick that lands on A");
    requireCompleteTickInterval(ticks, own.id, aTick, cTick);

    const replaceBefore = await ctx.shot("replace-before");
    const preReplace = unitOrder(replaceBefore.checkpoint.debug, own.id);
    assert(preReplace?.queued?.length > 0, "plain replacement begins with an actual nonempty tail");
    const replaceClick = ctx.inputFor("plain-replace");
    const replaceRelease = pointerEvent(replaceClick, "pointerup");
    const replaceAfter = await ctx.shot("replace-after");
    const replaceOrder = assertOrder(replaceAfter.checkpoint.debug, replaceRelease.seq, own.id, "muster:order_move");
    assert.equal(replaceOrder.result.status, "accepted");
    const replacementPoint = minimapPoint(ctx.actionFor("plain-replace"), replaceAfter.checkpoint, params.map_size);
    assertPoint(replaceOrder.emitted.target, replacementPoint, "plain replacement emits its clicked physical minimap point");
    const replacementState = unitOrder(replaceAfter.checkpoint.debug, own.id);
    assertPoint(replacementState?.current?.target, replacementPoint, "plain replacement becomes the exact active Move target");
    assert.equal(replacementState?.queued?.length, 0,
        "plain replacement clears the previously nonempty queued tail");

    const stopBefore = await ctx.shot("stop-before");
    const stopState = unitOrder(stopBefore.checkpoint.debug, own.id);
    assert(stopState?.current && stopState.queued?.length > 0, "Stop starts with actual current movement and nonempty tail");
    const stopKey = ctx.inputForKey("stop-order");
    const stopAfter = await ctx.shot("stop-after");
    const stopResult = assertKeyOrder(stopAfter.checkpoint.debug, stopKey, own.id, "muster:order_stop");
    assert.equal(stopResult.result.status, "accepted");
    const firstStopMove = assertOrder(stopBefore.checkpoint.debug,
        pointerEvent(ctx.inputFor("stop-prequeue-first"), "pointerup").seq, own.id, "muster:order_move");
    const stopStart = actorRows.find((row) => row.tick === firstStopMove.result.tick);
    const stopEnd = actorRows.find((row) => row.tick === stopResult.result.tick);
    assert(stopStart && stopEnd && stopResult.result.tick > firstStopMove.result.tick,
        "Stop interval has actual accepted Move and Stop tick boundaries");
    requireCompleteTickInterval(ticks, own.id, firstStopMove.result.tick, stopResult.result.tick);
    assert(actorRows.some((row) => row.tick > firstStopMove.result.tick && row.tick <= stopResult.result.tick
        && distance([row.x, row.y], [stopStart.x, stopStart.y]) > 0.01),
        "actual tick log shows the actor moving during the nonempty queued Move interval before Stop");
    const afterStopState = unitOrder(stopAfter.checkpoint.debug, own.id);
    assert.equal(afterStopState?.current, null, "Stop clears current order");
    assert.deepEqual(afterStopState?.queued || [], [], "Stop clears queued tail");
    const holdContext = await ctx.shot("hold-context");
    const holdStartActor = unitPosition(holdContext.checkpoint.debug, own.id);
    const holdHealth = healthSnapshot(holdContext.checkpoint.debug);
    const holdEnemy = (holdContext.checkpoint.debug.replicated_world?.units || [])
        .filter((unit) => unit.owner_slot === 1 && holdHealth[unit.unit_id] > 0)
        .sort((a, b) => distance(a.pos, holdStartActor) - distance(b.pos, holdStartActor))[0];
    assert(holdEnemy && distance(holdEnemy.pos, holdStartActor) < 4,
        "the pre-H screenshot binds a living foreign target inside actual weapon range");
    const holdBaseline = holdHealth[holdEnemy.unit_id];
    const holdShot = await ctx.shot("hold-after");
    const holdApproachClick = ctx.inputFor("hold-approach");
    const holdApproach = assertOrder(holdShot.checkpoint.debug, pointerEvent(holdApproachClick, "pointerup").seq, own.id, "muster:order_move");
    assert.equal(holdApproach.result.status, "accepted");
    const stationary = actorRows.filter((row) => row.tick >= stopResult.result.tick && row.tick < holdApproach.result.tick);
    assert(stationary.length >= 4 && stationary.every((row) => row.x === stationary[0].x && row.y === stationary[0].y),
        "actual tick log shows the stopped actor stationary over subsequent complete ticks");
    requireCompleteTickInterval(ticks, own.id, stopResult.result.tick, holdApproach.result.tick - 1);

    const holdKey = ctx.inputForKey("hold-order");
    const holdOrder = assertKeyOrder(holdShot.checkpoint.debug, holdKey, own.id, "muster:order_hold");
    assert.equal(holdOrder.result.status, "accepted");
    const holdActor = unitPosition(holdShot.checkpoint.debug, own.id);
    assert(equalF32(holdActor[0], holdStartActor[0]) && equalF32(holdActor[1], holdStartActor[1]),
        "Hold remains at its actual pre-H position while its target is damaged");
    const holdDamage = strictHealthDrops(holdShot.checkpoint.debug.replicated_world?.health_trace || [], holdEnemy.unit_id, holdOrder.result.tick);
    assert(holdDamage.some((sample) => sample.current < holdBaseline), "actual stationary Hold fires at and damages the in-range foreign target");
    const holdTicks = ticks.filter((row) => row.unit === own.id && row.tick >= holdOrder.result.tick
        && row.tick <= holdShot.checkpoint.debug.replicated_world.tick);
    requireCompleteTickInterval(ticks, own.id, holdOrder.result.tick, holdShot.checkpoint.debug.replicated_world.tick);
    assert(holdTicks.length >= 4 && holdTicks.every((row) => equalF32(row.x, holdActor[0]) && equalF32(row.y, holdActor[1])),
        "actual Hold stays exactly stationary while its in-range target takes damage");
    const held = ctx.inputFor("append-behind-hold");
    const holdRefusalShot = await ctx.shot("hold-refusal");
    const refusal = assertOrder(holdRefusalShot.checkpoint.debug, pointerEvent(held, "pointerup").seq, own.id, "muster:order_move");
    assert.equal(refusal.result.status, "refused");
    assert.equal(refusal.result.reason, "not_queueable", "actual Append behind Hold is rejected as not_queueable");

    const queueShot = await ctx.shot("queue-overflow-final");
    const queueActions = Array.from({ length: 17 }, (_, i) => ctx.inputFor(`queue-overflow-${i + 1}`));
    const outcomes = queueActions.map((click) => assertOrder(queueShot.checkpoint.debug, pointerEvent(click, "pointerup").seq, own.id, "muster:order_move").result);
    assert(outcomes.slice(0, 16).every((result) => result.status === "accepted"), "the actual authored queue accepts 16 pending appends");
    assert.equal(outcomes[16].status, "refused");
    assert.equal(outcomes[16].reason, "queue_full", "the seventeenth queued append is actually refused queue_full");
    const full = unitOrder(queueShot.checkpoint.debug, own.id);
    assert.equal(full?.queued?.length, 16);
    const queueStartAction = ctx.actionFor("queue-start");
    const queueTarget = minimapPoint(queueStartAction, queueShot.checkpoint, params.map_size);
    assertPoint(full.current.target, queueTarget, "long active queue-overflow Move remains current throughout 17 physical appends");
    const firstQueueTick = assertOrder(queueShot.checkpoint.debug,
        pointerEvent(ctx.inputFor("queue-start"), "pointerup").seq, own.id, "muster:order_move").result.tick;
    const lastQueueTick = outcomes[16].tick;
    requireCompleteTickInterval(ticks, own.id, firstQueueTick, lastQueueTick);
    const queueStartTickRow = actorRows.find((row) => row.tick >= firstQueueTick && close([row.x, row.y], queueTarget, 0.001));
    assert(!queueStartTickRow || queueStartTickRow.tick > lastQueueTick,
        "the deliberately distant active point cannot complete and free a queue slot during overflow input");

    const gatherAccepted = await ctx.shot("gather-accepted");
    const gatherClick = ctx.inputFor("gather-node");
    const gatherKey = ctx.inputForKey("gather-order");
    const workerPixel = params.stages.gather_worker_pixel;
    assert.equal(worker.id, identifyUnit(gatherContext.checkpoint, workerPixel, 0).id);
    assert.equal(worker.unit.content_id, "muster:worker");
    const gather = assertOrder(gatherAccepted.checkpoint.debug, pointerEvent(gatherClick, "pointerup").seq, worker.id, "muster:order_gather");
    assert.equal(gather.result.status, "accepted");
    assert.equal(gather.emitted.target?.Node ?? gather.emitted.target?.node, 1, "gather reaches actual authored ore node");
    assert(gatherKey && pointerEvent(gatherClick, "pointerup").seq > keyEvent(gatherKey, "keydown").seq,
        "the actual explicit Gather key precedes the physical world target click");
    const gatherQueueClick = ctx.inputFor("append-behind-gather");
    const appendDown = ctx.inputForKey("gather-shift-down");
    const appendUp = ctx.inputForKey("gather-shift-up");
    assertPointer(gatherQueueClick, "right", "Gather Append input");
    assert.equal(pointerEvent(gatherQueueClick, "pointerup").shift_key, true,
        "the actual Shift modifier remains held through the Append release");
    assert(keyEvent(appendDown, "keydown").seq < pointerEvent(gatherQueueClick, "pointerup").seq
        && keyEvent(appendUp, "keyup").seq > pointerEvent(gatherQueueClick, "pointerup").seq,
        "physical Shift down and up bracket the Gather Append release");
    const gatherRefusal = assertOrder(gatherAccepted.checkpoint.debug, pointerEvent(gatherQueueClick, "pointerup").seq, worker.id, "muster:order_move");
    assert.equal(gatherRefusal.result.status, "refused");
    assert.equal(gatherRefusal.result.reason, "not_queueable",
        "the immediate same-worker Append is refused while its only accepted typed order is Gather");
    assertPoint(gatherRefusal.emitted.target,
        minimapPoint(ctx.actionFor("append-behind-gather"), gatherAccepted.checkpoint, params.map_size),
        "the refused Append retains the actual clicked minimap point");
    assert(gather.result.tick <= gatherRefusal.result.tick,
        "the physical Gather acceptance is observed before or in the same Orders phase as its Append refusal");
    const gatherSeq = pointerEvent(gatherClick, "pointerup").seq;
    const appendSeq = pointerEvent(gatherQueueClick, "pointerup").seq;
    assert(gatherSeq < appendSeq, "the physical Gather target release precedes the immediate held-Shift Append release");
    const workerOrders = new Map();
    for (const frame of gatherAccepted.checkpoint.debug.frames?.retained_input || []) {
        for (const emitted of frame.emitted_orders || []) if (emitted.units?.includes(worker.id))
            workerOrders.set(`${emitted.seq}|${emitted.order}`, emitted);
    }
    assert.deepEqual([...workerOrders.values()].map((entry) => [entry.seq, entry.order]).sort((a, b) => a[0] - b[0]),
        [[gather.emitted.seq, "muster:order_gather"], [gatherRefusal.emitted.seq, "muster:order_move"]].sort((a, b) => a[0] - b[0]),
        "the worker has no earlier or intervening emitted Hold/order beyond this physical Gather and its refused Append");
    const gatherRefusalShot = await ctx.shot("gather-depleted");
    const laterGatherRefusal = assertOrder(gatherRefusalShot.checkpoint.debug, pointerEvent(gatherQueueClick, "pointerup").seq, worker.id, "muster:order_move");
    assert.equal(laterGatherRefusal.result.reason, "not_queueable");
    const depleted = (gatherRefusalShot.checkpoint.debug.replicated_world?.nodes || []).find((node) => node.id === 1);
    assert.equal(depleted?.remaining, 0, "ordinary gathering exhausts the isolated positive ore node");
    const depletedOrder = unitOrder(gatherRefusalShot.checkpoint.debug, worker.id);
    assert.equal(depletedOrder?.last_failure?.order, "muster:order_gather",
        "the same worker records failure of its actual Gather order");
    assert.equal(depletedOrder?.last_failure?.reason, "node_depleted",
        "same worker records actual node_depleted after exhaustion");
    assert(Number.isSafeInteger(depletedOrder.last_failure.tick)
        && depletedOrder.last_failure.tick >= gather.result.tick
        && depletedOrder.last_failure.tick >= gatherRefusal.result.tick
        && depletedOrder.last_failure.tick <= gatherRefusalShot.checkpoint.debug.replicated_world.tick,
        "node depletion is an actual server tick at/after the typed Gather acceptance and Append refusal and no later than capture");
    const depletionVisible = await ctx.shot("gather-depletion-visible");
    const notice = (depletionVisible.checkpoint.debug.ui_snapshot || []).find((row) => row.kind === "widget"
        && row.id === "last_notice" && row.visible === true && row.text === "Worker: Resource node is depleted");
    assert(notice && Array.isArray(notice.rect) && notice.rect.length === 4 && notice.rect.every(Number.isFinite),
        "the authored visible last_notice widget reports this worker's actual node_depleted feedback");
    const noticePixels = whitePixelsInRect(depletionVisible.image, notice.rect,
        Number(depletionVisible.checkpoint.scene.scale_factor) || 1);
    assert(noticePixels >= 8,
        `the node_depleted notice has actual rendered glyph pixels in its computed widget rectangle (found ${noticePixels})`);
}

async function judgeOrd4(ctx, params, own) {
    const beforeAttack = await ctx.shot("before");
    const view = await ctx.shot("attack-view-ready");
    const target = identifyUnit(view.checkpoint, ctx.steps.pixels.attack_target, 1);
    assert.equal(target.unit.content_id, "muster:dummy", "screenshot attack target is an actual slot 1 dummy");
    const closer = (view.checkpoint.debug.replicated_world?.units || []).filter((unit) => unit.owner_slot === 1 && unit.unit_id !== target.id)
        .sort((a, b) => distance(a.pos, unitPosition(view.checkpoint.debug, own.id)) - distance(b.pos, unitPosition(view.checkpoint.debug, own.id)))[0];
    assert(closer && distance(closer.pos, unitPosition(view.checkpoint.debug, own.id)) < distance(target.unit.pos, unitPosition(view.checkpoint.debug, own.id)),
        "a real nearer eligible foreign distractor remains present for ordered-target focus");
    const attackClick = ctx.inputFor("attack-selected-unit");
    assertPointer(attackClick, "left", "explicit Unit Attack target");
    const attack = await ctx.shot("attack-issued");
    const attackResult = assertOrder(attack.checkpoint.debug, pointerEvent(attackClick, "pointerup").seq, own.id, "muster:order_attack");
    assert.equal(attackResult.result.status, "accepted");
    assert.equal(targetUnitId(attackResult.emitted.target), target.id, "the accepted ordered target is the screenshot-bound target");
    const activeAttack = unitOrder(attack.checkpoint.debug, own.id)?.current;
    assert.equal(activeAttack?.order, "muster:order_attack", "the accepted Unit Attack is the actor's actual typed current order");
    assert.equal(targetUnitId(activeAttack?.target), target.id, "actual current Attack retains the screenshot-bound Unit target");
    const attacked = await ctx.shot("attack-after-approach");
    const trace = attacked.checkpoint.debug.replicated_world?.health_trace || [];
    const attackBaseline = healthSnapshot(beforeAttack.checkpoint.debug)[target.id];
    const distractorBaseline = healthSnapshot(beforeAttack.checkpoint.debug)[closer.unit_id];
    const orderedDrops = strictHealthDrops(trace, target.id, attackResult.result.tick).filter((entry) => entry.current < attackBaseline);
    assert(orderedDrops.length > 0, "the accepted Unit Attack causes actual health loss to its ordered target");
    const tickRows = parseTickLog(await ctx.view.serverFile("server-tick.log"));
    const firstOrderedDrop = orderedDrops[0];
    const attackActorRows = tickRows.filter((row) => row.unit === own.id);
    const eligibleTick = attackActorRows.find((row) => row.tick >= attackResult.result.tick
        && distance([row.x, row.y], target.unit.pos) <= 4 && distance([row.x, row.y], closer.pos) <= 4
        && distance([row.x, row.y], closer.pos) < distance([row.x, row.y], target.unit.pos))?.tick;
    assert(Number.isSafeInteger(eligibleTick), "accepted ordered Attack reaches a tick where both the target and nearer distractor are eligible");
    const eligiblePositions = positionsAtTick(tickRows, eligibleTick);
    const actorAtEligibility = eligiblePositions.get(own.id);
    assert(actorAtEligibility, "authoritative tick log has the attacking actor at its first ordered-focus opportunity");
    const liveForeignAtEligibility = (beforeAttack.checkpoint.debug.replicated_world?.units || []).filter((unit) => unit.owner_slot === 1
        && healthAtOrBefore(trace, unit.unit_id, eligibleTick - 1, healthSnapshot(beforeAttack.checkpoint.debug)[unit.unit_id]) > 0)
        .map((unit) => ({ unit, position: eligiblePositions.get(unit.unit_id) }));
    assert(liveForeignAtEligibility.every((entry) => entry.position),
        "authoritative tick log contains every living foreign candidate at the first focus opportunity");
    const eligibleForeign = liveForeignAtEligibility.filter((entry) => distance(actorAtEligibility, entry.position) <= 4);
    assert(eligibleForeign.some((entry) => entry.unit.unit_id === target.id)
        && eligibleForeign.some((entry) => entry.unit.unit_id === closer.unit_id),
        "ordered target and nearer distractor are both alive and within actual weapon range at the focus opportunity");
    const targetDeathTick = orderedDrops.find((entry) => entry.current <= 0)?.tick
        ?? attacked.checkpoint.debug.replicated_world.tick;
    const eligibleDrops = eligibleForeign.flatMap(({ unit }) => {
        const baseline = healthAtOrBefore(trace, unit.unit_id, eligibleTick - 1,
            healthSnapshot(beforeAttack.checkpoint.debug)[unit.unit_id]);
        return strictHealthDrops(trace, unit.unit_id, eligibleTick)
            .filter((entry) => entry.current < baseline && entry.tick <= targetDeathTick)
            .map((entry) => ({ unit: unit.unit_id, ...entry }));
    }).sort((a, b) => a.tick - b.tick || a.unit - b.unit);
    assert(eligibleDrops.length > 0, "an actual eligible foreign target receives a strict health transition");
    const firstEligibleTick = eligibleDrops[0].tick;
    const firstEligibleHits = eligibleDrops.filter((entry) => entry.tick === firstEligibleTick);
    assert(firstEligibleHits.every((entry) => entry.unit === target.id),
        "every first-opportunity health transition is the ordered target, with no tied shot to another eligible unit");
    const positionsAtDrop = positionsAtTick(tickRows, firstEligibleTick);
    const actorAtDrop = positionsAtDrop.get(own.id), targetAtDrop = positionsAtDrop.get(target.id), distractorAtDrop = positionsAtDrop.get(closer.unit_id);
    assert(actorAtDrop && targetAtDrop && distractorAtDrop,
        "authoritative tick log binds actor, ordered target, and nearer distractor at the first eligible health transition");
    assert(distance(actorAtDrop, targetAtDrop) <= 4 && distance(actorAtDrop, distractorAtDrop) <= 4
        && distance(actorAtDrop, distractorAtDrop) < distance(actorAtDrop, targetAtDrop),
        "at the first eligible damage tick, the nearer distractor remains in range of the ordered actor");
    const allied = (beforeAttack.checkpoint.debug.replicated_world?.units || []).filter((unit) => unit.owner_slot === 0 && unit.unit_id !== own.id);
    assert(allied.every((unit) => eligibleForeign.every(({ unit: foreign }) => {
        const position = positionsAtDrop.get(foreign.unit_id);
        const alliedPosition = positionsAtDrop.get(unit.unit_id);
        return position && alliedPosition && distance(alliedPosition, position) > 4;
    })), "no other allied unit can attribute the first eligible hit to any in-range foreign candidate");
    const startActor = unitPosition(beforeAttack.checkpoint.debug, own.id);
    const endActor = unitPosition(attacked.checkpoint.debug, own.id);
    assert(distance(startActor, endActor) > 2 && distance(endActor, target.unit.pos) < distance(startActor, target.unit.pos),
        "the actual selected actor approaches the ordered Unit target");
    const finalTargetHealth = strictHealthDrops(trace, target.id, attackResult.result.tick).at(-1)?.current;
    const targetStillReplicated = (attacked.checkpoint.debug.replicated_world?.units || []).some((unit) => unit.unit_id === target.id);
    assert(finalTargetHealth === 0 || !targetStillReplicated || healthSnapshot(attacked.checkpoint.debug)[target.id] === 0,
        "the accepted Attack continues until the exact ordered target is removed or its authoritative health reaches zero");

    const rushIssued = await ctx.shot("rush-issued");
    const rushClick = ctx.inputFor("rush-target");
    const rush = assertOrder(rushIssued.checkpoint.debug, pointerEvent(rushClick, "pointerup").seq, own.id, "muster:rush");
    assert.equal(rush.result.status, "accepted");
    const rushTarget = unprojectGround(rushIssued.checkpoint.scene, ctx.steps.pixels.rush_ground);
    assertPoint(rush.emitted.target, rushTarget, "actual screenshot-projected Rush target");
    const activeRush = unitOrder(rushIssued.checkpoint.debug, own.id)?.current;
    assert.equal(activeRush?.order, "muster:rush", "the accepted Rush is the actor's actual current order");
    assertPoint(activeRush.target, rushTarget, "actual active Rush retains the screenshot-derived point target");
    const rushIssuedHealth = healthSnapshot(rushIssued.checkpoint.debug);
    const rushActorBefore = unitPosition(rushIssued.checkpoint.debug, own.id);
    const rushVictim = (rushIssued.checkpoint.debug.replicated_world?.units || [])
        .filter((unit) => unit.owner_slot === 1 && rushIssuedHealth[unit.unit_id] > 0
            && distance(unit.pos, rushActorBefore) <= 4)
        .sort((a, b) => distance(a.pos, rushActorBefore) - distance(b.pos, rushActorBefore))[0];
    assert(rushVictim && rushIssuedHealth[rushVictim.unit_id] > 0
        && distance(rushVictim.pos, rushActorBefore) <= 4,
        "the Rush-issued checkpoint binds a specific living foreign target already inside actual weapon range before damage");
    const rushBaseline = rushIssuedHealth[rushVictim.unit_id];
    const engagement = await ctx.shot("rush-engagement");
    const rushAfter = await ctx.shot("rush-paused-or-resumed");
    const engagementDrops = strictHealthDrops(engagement.checkpoint.debug.replicated_world?.health_trace || [], rushVictim.unit_id, rush.result.tick)
        .filter((entry) => entry.current < rushBaseline);
    assert(engagementDrops.length > 0, "authored Rush damages the same actual pre-damage eligible target");
    const rushActorEngaged = unitPosition(engagement.checkpoint.debug, own.id);
    assert(distance(rushActorBefore, rushActorEngaged) < 0.25,
        "StopAndFire pauses the actor while its bound in-range enemy takes damage");
    const rushTargetStillReplicated = (rushAfter.checkpoint.debug.replicated_world?.units || []).some((unit) => unit.unit_id === rushVictim.unit_id);
    assert(!rushTargetStillReplicated || healthSnapshot(rushAfter.checkpoint.debug)[rushVictim.unit_id] === 0,
        "Rush resumes only after the same pre-damage target is removed or reaches zero health");
    const rushActorAfter = unitPosition(rushAfter.checkpoint.debug, own.id);
    assert(distance(rushActorAfter, rushActorEngaged) > 0.5
        && dot(sub(rushTarget, rushActorEngaged), sub(rushActorAfter, rushActorEngaged)) > 0,
        "Rush resumes actual movement toward its point after removing the in-range target");

    const safeClick = ctx.inputFor("force-safe-approach");
    const safeMoveShot = await ctx.shot("force-safe-arrived");
    const safeMove = assertOrder(safeMoveShot.checkpoint.debug, pointerEvent(safeClick, "pointerup").seq, own.id, "muster:order_move");
    assert.equal(safeMove.result.status, "accepted", "a physical smart Move parks the actor outside live enemy range");
    const safePoint = minimapPoint(ctx.actionFor("force-safe-approach"), safeMoveShot.checkpoint, params.map_size);
    assert(distance(unitPosition(safeMoveShot.checkpoint.debug, own.id), safePoint) <= 0.001,
        "the actor reaches the actual screenshot-derived safe staging point");
    const forceContext = await ctx.shot("force-context");
    assert(distance(unitPosition(forceContext.checkpoint.debug, own.id), safePoint) <= 0.001,
        "the actor remains at the safe staging point before force Move");
    const forceTarget = identifyUnit(forceContext.checkpoint, ctx.steps.pixels.force_target, 1);
    assert(distance(forceTarget.unit.pos, unprojectGround(forceContext.checkpoint.scene, ctx.steps.pixels.force_target)) <= forceTarget.sceneUnit.pick_radius + 0.15,
        "force target screenshot ray lies inside the living enemy's real pick footprint");
    const forceClick = ctx.inputFor("force-move-destination");
    assertPointer(forceClick, "left", "explicit force Move target");
    const forceIssued = await ctx.shot("force-issued");
    const force = assertOrder(forceIssued.checkpoint.debug, pointerEvent(forceClick, "pointerup").seq, own.id, "muster:order_move");
    assert.equal(force.result.status, "accepted");
    const forcePoint = unprojectGround(forceContext.checkpoint.scene, ctx.steps.pixels.force_target);
    assertPoint(force.emitted.target, forcePoint, "force Move target is the actual screenshot ray under a living enemy");
    const forceDone = await ctx.shot("force-move-arrival");
    const active = unitOrder(forceDone.checkpoint.debug, own.id);
    assert.equal(active?.current, null, "force Move capture observes actual Idle/currentNone");
    assert.equal(active?.last_failure, null, "force Move did not fail, become Blocked, or get replaced");
    const ticks = parseTickLog(await ctx.view.serverFile("server-tick.log"));
    assertCompleteTicks(ticks, own.id);
    const actorRows = ticks.filter((row) => row.unit === own.id);
    const end = unitPosition(forceDone.checkpoint.debug, own.id);
    const idleTick = forceDone.checkpoint.debug.replicated_world.tick;
    const finalRow = actorRows.find((row) => row.tick === idleTick);
    assert(finalRow && equalF32(finalRow.x, end[0]) && equalF32(finalRow.y, end[1]),
        "the authoritative tick log ends at the captured force-Move actor position");
    requireCompleteTickInterval(ticks, own.id, force.result.tick, idleTick);
    const forceRows = actorRows.filter((row) => row.tick >= force.result.tick && row.tick <= idleTick);
    let suffixStart = forceRows.length - 1;
    while (suffixStart > 0 && forceRows[suffixStart - 1].x === finalRow.x && forceRows[suffixStart - 1].y === finalRow.y) suffixStart -= 1;
    const landing = forceRows[suffixStart];
    assert(landing && forceRows.at(-1).tick === idleTick
        && forceRows.slice(suffixStart).every((row) => row.x === finalRow.x && row.y === finalRow.y),
        "force Move has an exact-coordinate terminal suffix through the first captured Idle state");
    const damageAtStart = healthSnapshot(forceContext.checkpoint.debug);
    const damageTrace = forceDone.checkpoint.debug.replicated_world?.health_trace || [];
    const foreignIds = (forceContext.checkpoint.debug.replicated_world?.units || []).filter((unit) => unit.owner_slot === 1).map((unit) => unit.unit_id);
    for (const foreign of foreignIds) assertNoHealthDrop(damageTrace, foreign, force.result.tick, landing.tick, damageAtStart[foreign]);
    const weapon = (forceContext.checkpoint.debug.replicated_world?.weapons || []).find((entry) => entry.unit === own.id);
    assert(weapon?.enabled && weapon.cooldown === 0 && weapon.status !== "Cooling" && weapon.damage?.Ok !== undefined,
        "actual public weapon record proves the force-move actor starts ready with no cooldown");
    const cooldownTicks = params.stages.weapon_cooldown_ticks;
    const acceptedIndex = actorRows.findIndex((row) => row.tick >= force.result.tick);
    assert(cooldownTicks > 0 && acceptedIndex >= cooldownTicks, "actual actor history spans at least the authored weapon cooldown before force Move");
    const safePrefix = actorRows.slice(acceptedIndex - cooldownTicks, acceptedIndex);
    const liveForeign = (forceContext.checkpoint.debug.replicated_world?.units || []).filter((unit) => unit.owner_slot === 1);
    assert(safePrefix.length === cooldownTicks && safePrefix.every((row) => liveForeign.every((enemy) => distance([row.x, row.y], enemy.pos) > 4)),
        "the actor physically remained outside every live enemy range for a full authored cooldown before the force command");
    const safeArrivalTick = actorRows.find((row) => row.tick >= safeMove.result.tick && close([row.x, row.y], safePoint, 0.001))?.tick;
    assert(Number.isSafeInteger(safeArrivalTick) && safeArrivalTick < force.result.tick,
        "complete server tick positions witness arrival at the safe point before force Move");
    requireCompleteTickInterval(ticks, own.id, safeArrivalTick, force.result.tick);
    assert(actorRows.slice(acceptedIndex, actorRows.findIndex((row) => row.tick === landing.tick) + 1)
        .some((row) => distance([row.x, row.y], forcePoint) < 4),
        "the ready armed actor has a real eligible firing opportunity during force movement");
    assert(distance(unitPosition(forceIssued.checkpoint.debug, own.id), end) > 0.5,
        "force Move travels through the actual enemy's eligible firing range");
    const targetCenter = forceTarget.unit.pos;
    const contactRadius = own.sceneUnit.pick_radius + forceTarget.sceneUnit.pick_radius;
    const centerDistance = distance(end, targetCenter);
    const clickedOffset = distance(forcePoint, targetCenter);
    assert(centerDistance + 0.001 >= contactRadius,
        "force Move ends outside the actual actor-plus-target pick-footprint overlap distance");
    assert(clickedOffset <= forceTarget.sceneUnit.pick_radius + 0.15,
        "force Move remains bound to the actual clicked target footprint");
    assert(centerDistance <= contactRadius + forceTarget.sceneUnit.pick_radius + 0.25,
        "force Move ends within actual pick-footprint contact plus bounded legal occupied-target fallback of the enemy center");
}

function makeContext(view, input, checkpoints, steps, params) {
    const consumed = new Set();
    const inputByLabel = new Map();
    return {
        view, input, checkpoints, steps,
        async shot(label) {
            const name = `${steps.row.toLowerCase()}-${label}.png`;
            const saveIndex = input.findIndex((entry) => entry.call === "save" && entry.args?.[0] === name);
            assert(saveIndex > 0, `missing actual saved screenshot ${name}`);
            let screenIndex = saveIndex - 1;
            while (screenIndex >= 0 && input[screenIndex].call !== "screenshot") screenIndex -= 1;
            const checkpoint = checkpoints.find((entry) => entry.input_call === screenIndex && entry.window === 1);
            assert(checkpoint?.scene && checkpoint?.debug, `${name} lacks its authentic C84 checkpoint`);
            return { checkpoint, image: decodePng(Buffer.from(await view.serverFile(`${name}.b64`), "base64")) };
        },
        inputFor(label, mark = true) {
            if (inputByLabel.has(label)) return inputByLabel.get(label);
            const action = this.actionFor(label);
            const index = input.findIndex((entry, i) => (!mark || !consumed.has(i)) && entry.call === action.call
                && JSON.stringify(entry.args) === JSON.stringify([action.point[0], action.point[1], action.button]));
            assert(index >= 0, `${label} has no exact actual harness input`);
            if (mark) consumed.add(index);
            inputByLabel.set(label, input[index]);
            return input[index];
        },
        inputForKey(label, mark = true) {
            if (inputByLabel.has(label)) return inputByLabel.get(label);
            const action = this.actionFor(label);
            assert.equal(action.call, "key", `${label} is a physical key action`);
            const index = input.findIndex((entry, i) => (!mark || !consumed.has(i)) && entry.call === "key" && entry.args?.[0] === action.code
                && JSON.stringify(entry.args?.[1] || []) === JSON.stringify(action.modifiers || [])
                && JSON.stringify(entry.args?.[2] || {}) === JSON.stringify(action.options || {}));
            assert(index >= 0, `${label} has an exact actual key input`);
            if (mark) consumed.add(index);
            inputByLabel.set(label, input[index]);
            return input[index];
        },
        actionFor(label) {
            const action = steps.actions.find((entry) => entry.label === label);
            assert(action, `driver lacks physical action ${label}`);
            return action;
        },
        params,
    };
}

function stepsWorker(steps) { return steps.pixels.worker; }
function identifyUnit(checkpoint, pixel, owner) {
    const world = unprojectGround(checkpoint.scene, { x: pixel.x ?? pixel[0], y: pixel.y ?? pixel[1] });
    const matches = (checkpoint.scene.units || []).filter((unit) => Array.isArray(unit.translation) && Number.isFinite(unit.pick_radius))
        .map((unit) => ({ unit, d: Math.hypot(unit.translation[0] - world[0], unit.translation[2] - world[1]) }))
        .filter(({ unit, d }) => d <= unit.pick_radius + 0.15
            && (checkpoint.debug.replicated_world?.units || []).find((publicUnit) => publicUnit.unit_id === unit.unit_id)?.owner_slot === owner)
        .sort((a, b) => a.d - b.d);
    assert(matches.length, "current public screenshot ray must hit an actual C84 unit footprint");
    return { id: matches[0].unit.unit_id, sceneUnit: matches[0].unit,
        unit: (checkpoint.debug.replicated_world?.units || []).find((unit) => unit.unit_id === matches[0].unit.unit_id) };
}

function assertPointer(entry, button, label) {
    assert.equal(entry.args?.[2], button, `${label} uses the authored physical ${button} button`);
    const released = pointerEvent(entry, "pointerup");
    assert.equal(released.canvas_target, true, `${label} reaches the actual game canvas`);
}
function pointerEvent(entry, type) {
    const event = entry.actual_events?.find((candidate) => candidate.type === type && candidate.canvas_target);
    assert(Number.isSafeInteger(event?.seq), `actual canvas ${type} has a trusted input sequence`);
    return event;
}
function keyEvent(entry, type) {
    const event = entry.actual_events?.find((candidate) => candidate.type === type && candidate.canvas_target);
    assert(Number.isSafeInteger(event?.seq), `actual canvas ${type} has a trusted input sequence`);
    return event;
}
function ord3CommandLabels(trooper, worker) {
    const labels = [];
    for (let i = 1; i <= 3; i += 1) labels.push([`route-append-${i}`, "muster:order_move", [trooper]]);
    labels.push(["replace-tail-first", "muster:order_move", [trooper]], ["replace-tail-second", "muster:order_move", [trooper]],
        ["plain-replace", "muster:order_move", [trooper]], ["stop-prequeue-first", "muster:order_move", [trooper]],
        ["stop-prequeue-second", "muster:order_move", [trooper]], ["stop-order", "muster:order_stop", [trooper]],
        ["hold-approach", "muster:order_move", [trooper]], ["hold-order", "muster:order_hold", [trooper]],
        ["append-behind-hold", "muster:order_move", [trooper]], ["queue-start", "muster:order_move", [trooper]],
        ["gather-node", "muster:order_gather", [worker]], ["append-behind-gather", "muster:order_move", [worker]]);
    for (let i = 1; i <= 17; i += 1) labels.push([`queue-overflow-${i}`, "muster:order_move", [trooper]]);
    return labels;
}
function assertNoUncommandedOrders(ctx, units, actionOrders) {
    const allowedByInputSeq = new Map();
    const ordered = [...actionOrders].sort((a, b) => ctx.steps.actions.findIndex((entry) => entry.label === a[0])
        - ctx.steps.actions.findIndex((entry) => entry.label === b[0]));
    for (const [label, order, permittedUnits = units] of ordered) {
        const action = ctx.actionFor(label);
        const input = action.call === "key" ? ctx.inputForKey(label) : ctx.inputFor(label);
        const event = action.call === "key" ? keyEvent(input, "keydown") : pointerEvent(input, "pointerup");
        const rule = allowedByInputSeq.get(event.seq) || { pairs: new Set() };
        for (const unit of permittedUnits) rule.pairs.add(`${order}|${unit}`);
        allowedByInputSeq.set(event.seq, rule);
    }
    const frames = ctx.checkpoints.flatMap((checkpoint) => checkpoint.debug?.frames?.retained_input || []);
    assertEmissionsBelongToCommands(frames, allowedByInputSeq);
}
function assertEmissionsBelongToCommands(frames, allowedByInputSeq) {
    const uniqueFrames = new Map();
    for (const frame of frames) {
        assert(Number.isSafeInteger(frame.frame_index), "retained input frame has its actual producer frame identity");
        const serialized = JSON.stringify({ applied_seq: frame.applied_seq, emitted_orders: frame.emitted_orders, results: frame.results });
        const prior = uniqueFrames.get(frame.frame_index);
        if (prior !== undefined) {
            assert.equal(prior, serialized, `retained copies of frame ${frame.frame_index} are identical`);
            continue;
        }
        uniqueFrames.set(frame.frame_index, serialized);
    }
    const observed = new Set();
    for (const frame of frames.filter((entry, index, all) => all.findIndex((candidate) => candidate.frame_index === entry.frame_index) === index)) {
        const emitted = frame.emitted_orders || [];
        if (!emitted.length) continue;
        const rules = (frame.applied_seq || []).map((sequence) => allowedByInputSeq.get(sequence)).filter(Boolean);
        assert(rules.length > 0, "retained frames without an allowed physical command gesture emit no orders");
        for (const order of emitted) {
            assert((order.units || []).length > 0, "emitted order has actual per-unit targets");
            for (const unit of order.units) {
                const pair = `${order.order}|${unit}`;
                const matching = rules.filter((rule) => rule.pairs.has(pair));
                assert(matching.length > 0, "every exact order/unit pair is bound to an allowed physical command gesture");
                const inputSeq = (frame.applied_seq || []).find((sequence) => allowedByInputSeq.get(sequence)?.pairs.has(pair));
                const identity = `${inputSeq}|${unit}`;
                assert(!observed.has(identity), "one physical gesture emits at most one order for each unit");
                observed.add(identity);
            }
        }
    }
}
function frameFor(debug, sequence) {
    const matches = (debug.frames?.retained_input || []).filter((frame) => frame.applied_seq?.includes(sequence));
    assert.equal(matches.length, 1, `physical sequence ${sequence} is applied exactly once`);
    return matches[0];
}
function assertOrder(debug, releaseSeq, unit, orderName) {
    const frame = frameFor(debug, releaseSeq);
    const emitted = (frame.emitted_orders || []).find((entry) => entry.order === orderName && entry.units?.includes(unit));
    assert(emitted && Number.isSafeInteger(emitted.seq), `physical input emits ${orderName} for actual unit ${unit}`);
    const result = (debug.frames?.retained_input || []).flatMap((frame) => frame.results || [])
        .find((entry) => entry.seq === emitted.seq && entry.unit === unit);
    assert(result, `actual server result is retained for emitted sequence ${emitted.seq} and unit ${unit}`);
    return { frame, emitted, result };
}
function assertKeyOrder(debug, keyEntry, unit, orderName) {
    assert(keyEntry, `driver physically sends ${orderName} key`);
    const sequence = keyEntry.actual_events?.find((entry) => entry.type === "keydown")?.seq;
    assert(Number.isSafeInteger(sequence), `${orderName} hotkey is actual trusted input`);
    const frame = frameFor(debug, sequence);
    const emitted = (frame.emitted_orders || []).find((entry) => entry.order === orderName && entry.units?.includes(unit));
    assert(emitted, `${orderName} is emitted for the selected unit by its physical hotkey`);
    const result = (debug.frames?.retained_input || []).flatMap((frame) => frame.results || [])
        .find((entry) => entry.seq === emitted.seq && entry.unit === unit);
    assert(result, `${orderName} has an actual result`);
    return { frame, emitted, result };
}
function unitOrder(debug, unit) { return (debug.replicated_world?.unit_orders || []).find((entry) => entry.unit === unit); }
function targetUnitId(target) {
    const unit = target?.Unit ?? target?.unit;
    return Number.isSafeInteger(unit) ? unit : unit?.unit_id ?? unit?.[0];
}
function assertPoint(target, expected, label) {
    const point = target?.Point || target?.point;
    const x = expected[0];
    const y = expected[1];
    assert(point && Math.hypot(point.x - x, point.y - y) <= 0.001, `${label} matches the authored target within f32 tolerance`);
}
function minimapPoint(action, checkpoint, mapSize) {
    const widget = (checkpoint.scene.widgets || []).find((entry) => entry.id === "minimap");
    assert(widget && Array.isArray(widget.rect) && widget.rect.length === 4, "C84 screenshot exposes actual minimap geometry");
    const [left, top, width, height] = widget.rect;
    return [((action.point[0] - left) / width) * mapSize[0], (1 - (action.point[1] - top) / height) * mapSize[1]];
}
function targetPoint(target) {
    const point = target?.Point || target?.point;
    assert(point && Number.isFinite(point.x) && Number.isFinite(point.y), "actual emitted point target exists");
    return [point.x, point.y];
}
function unitPosition(debug, unit) {
    const position = (debug.replicated_world?.units || []).find((entry) => entry.unit_id === unit)?.pos;
    assert(Array.isArray(position) && position.length === 2 && position.every(Number.isFinite), `actual replicated position exists for unit ${unit}`);
    return position;
}
function strictHealthDrops(trace, unit, minTick) {
    const entries = (trace.find((entry) => entry.unit_id === unit)?.entries || [])
        .filter((entry) => Number.isSafeInteger(entry.tick) && Number.isFinite(entry.current))
        .sort((a, b) => a.tick - b.tick);
    const drops = [];
    for (let i = 1; i < entries.length; i += 1) {
        if (entries[i].tick >= minTick && entries[i].current < entries[i - 1].current)
            drops.push({ ...entries[i], previous: entries[i - 1] });
    }
    return drops;
}
function healthSnapshot(debug) {
    return Object.fromEntries((debug.replicated_world?.healths || []).map((entry) => [entry.unit, entry.current]));
}
function healthAtOrBefore(trace, unit, tick, fallback) {
    const entries = (trace.find((entry) => entry.unit_id === unit)?.entries || [])
        .filter((entry) => entry.tick <= tick && Number.isFinite(entry.current))
        .sort((a, b) => a.tick - b.tick);
    return entries.length ? entries.at(-1).current : fallback;
}
function assertNoHealthDrop(trace, unit, fromTick, throughTick, baseline) {
    assert(Number.isFinite(baseline), `foreign unit ${unit} has an actual pre-force health baseline`);
    const entries = (trace.find((entry) => entry.unit_id === unit)?.entries || []).filter((entry) => entry.tick >= fromTick && entry.tick <= throughTick);
    assert(entries.every((entry) => entry.current >= baseline), `forced mover causes no foreign health loss through landing for unit ${unit}`);
}
function nearestForeign(debug, position) {
    return (debug.replicated_world?.units || []).filter((unit) => unit.owner_slot === 1)
        .sort((a, b) => distance(a.pos, position) - distance(b.pos, position))[0];
}
function waypointSegments(checkpoint, unit, targets) {
    const record = unitOrder(checkpoint.debug, unit);
    const current = record?.current?.target?.Point || record?.current?.target?.point;
    assert(current, "selected unit has the renderer-consumed current route target");
    const from = (checkpoint.scene.units || []).find((entry) => entry.unit_id === unit)?.translation;
    assert(Array.isArray(from) && from.length === 3, "actual scene has the selected unit current presentation position");
    const points = [[from[0], from[2]], ...targets];
    return points.slice(0, 3).map((point, index) => [point, targets[index]]);
}
function countUniqueLinePixels(image, scene, segments) {
    const global = new Set(), perSegment = [];
    for (const [a, b] of segments) {
        const pa = project(scene, a), pb = project(scene, b);
        const length = Math.hypot(pb[0] - pa[0], pb[1] - pa[1]);
        const local = new Set();
        const steps = Math.max(1, Math.ceil(length));
        for (let i = 1; i < steps; i += 1) {
            const t = i / steps;
            if (t < 0.18 || t > 0.82) continue;
            const x = Math.round(pa[0] + (pb[0] - pa[0]) * t), y = Math.round(pa[1] + (pb[1] - pa[1]) * t);
            for (let dy = -1; dy <= 1; dy += 1) for (let dx = -1; dx <= 1; dx += 1) {
                const px = x + dx, py = y + dy;
                const pixel = pixelAt(image, px, py);
                if (pixel && pixel[3] >= 245 && Math.hypot(pixel[0] - 230, pixel[1] - 230, pixel[2] - 217) <= 70) {
                    const key = `${px},${py}`;
                    local.add(key);
                    global.add(key);
                }
            }
        }
        perSegment.push(local.size);
    }
    return { total: global.size, perSegment };
}
function project(scene, point) {
    const m = scene.clip_from_world, v = scene.viewport_logical_rect;
    assert(Array.isArray(m) && m.length === 16 && Array.isArray(v) && v.length === 4, "actual C84 camera projection exists");
    const clip = [0, 1, 2, 3].map((r) => m[r] * point[0] + m[4 + r] * 0.04 + m[8 + r] * point[1] + m[12 + r]);
    return [v[0] + (clip[0] / clip[3] + 1) * v[2] / 2, v[1] + (1 - clip[1] / clip[3]) * v[3] / 2];
}
function beforePng(shot) { return shot.image; }
function whitePixelsInRect(image, rect, scale) {
    const left = Math.max(0, Math.floor(rect[0] / scale));
    const top = Math.max(0, Math.floor(rect[1] / scale));
    const right = Math.min(image.width, Math.ceil((rect[0] + rect[2]) / scale));
    const bottom = Math.min(image.height, Math.ceil((rect[1] + rect[3]) / scale));
    let count = 0;
    for (let y = top; y < bottom; y += 1) for (let x = left; x < right; x += 1) {
        const pixel = pixelAt(image, x, y);
        if (pixel && pixel[3] > 0 && pixel[0] > 220 && pixel[1] > 220 && pixel[2] > 220) count += 1;
    }
    return count;
}
// True when the tick of a checkpoint is before the first tick of the actor at a point.
function readBeforeLanding(actorRows, point, checkpointTick) {
    const landed = actorRows.find((row) => close([row.x, row.y], point, 0.001));
    return !landed || checkpointTick < landed.tick;
}
function assertReadBeforeLanding(actorRows, point, checkpointTick, label) {
    assert(Number.isSafeInteger(checkpointTick), `${label}: the checkpoint has a replica tick`);
    const landed = actorRows.find((row) => close([row.x, row.y], point, 0.001));
    assert(readBeforeLanding(actorRows, point, checkpointTick),
        `${label}: the checkpoint tick ${checkpointTick} is after the landing tick ${landed.tick}`);
}
function parseTickLog(text) {
    return text.trim().split(/\r?\n/).filter(Boolean).map((line) => {
        const [tick, _wall, unit, x, y] = line.split(",").map(Number);
        assert([tick, unit, x, y].every(Number.isFinite), "authoritative server tick log contains valid rows");
        return { tick, unit, x, y };
    });
}
function positionsAtTick(rows, tick) {
    const sameTick = rows.filter((row) => row.tick === tick);
    const byUnit = new Map();
    for (const row of sameTick) {
        const existing = byUnit.get(row.unit) || [];
        existing.push([row.x, row.y]);
        byUnit.set(row.unit, existing);
    }
    const positions = new Map();
    for (const [unit, values] of byUnit) {
        assert.equal(values.length, 1, `unit ${unit} has one authoritative position at tick ${tick}`);
        positions.set(unit, values[0]);
    }
    return positions;
}
function assertCompleteTicks(rows, unit) {
    const values = [...new Set(rows.filter((entry) => entry.unit === unit).map((entry) => entry.tick))].sort((a, b) => a - b);
    assert(values.length > 2, `actual server tick log contains actor ${unit}`);
    for (let i = 1; i < values.length; i += 1) assert.equal(values[i], values[i - 1] + 1, "actor tick log has no gaps");
}
function requireCompleteTickInterval(rows, unit, start, end) {
    assert(Number.isSafeInteger(start) && Number.isSafeInteger(end) && end >= start, "required actor tick interval is bounded");
    const actor = rows.filter((entry) => entry.unit === unit);
    for (let tick = start; tick <= end; tick += 1) {
        const matches = actor.filter((entry) => entry.tick === tick);
        assert.equal(matches.length, 1, `authoritative actor tick ${tick} appears exactly once in the required interval`);
        assert(Number.isFinite(matches[0].x) && Number.isFinite(matches[0].y), `actor tick ${tick} has finite coordinates`);
    }
    assert(actor.some((entry) => entry.tick <= start) && actor.some((entry) => entry.tick >= end),
        "authoritative actor history covers both required interval boundaries");
}
function close(a, b, epsilon) { return distance(a, b) <= epsilon; }
function equalF32(a, b) { return Math.fround(a) === Math.fround(b); }
function distance(a, b) { return Math.hypot(a[0] - b[0], a[1] - b[1]); }
function sub(a, b) { return [a[0] - b[0], a[1] - b[1]]; }
function dot(a, b) { return a[0] * b[0] + a[1] * b[1]; }

module.exports = { judge, __test: { assertEmissionsBelongToCommands, strictHealthDrops, readBeforeLanding, parseTickLog } };
