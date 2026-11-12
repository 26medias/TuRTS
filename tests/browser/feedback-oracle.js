// turts-test-role: oracle
"use strict";

const assert = require("node:assert/strict");
const { decodePng, pixelAt } = require("./harness/png.js");
const { assertSwitchClickByLabel } = require("./harness/sound-switch-proof.js");

async function judge(view, params) {
    const row = params.row;
    const input = view.inputLog();
    const checkpoints = JSON.parse(await view.serverFile("scene-checkpoints.json"));
    assert(Array.isArray(checkpoints), `${row} needs actual scene/debug checkpoints`);
    const final = await view.bridge("turts_debug");
    if (row === "NOORD") return judgeNoOrders(view, input, checkpoints, final);
    if (row === "ORD2") return judgeOrd2(view, input, checkpoints, params);
    if (row === "ORD6") return judgeOrd6(input, checkpoints, final);
    if (row === "ORD7") return judgeOrd7(view, input, checkpoints, final);
    if (row === "ALR1") return judgeAlr1(view, input, checkpoints, params);
    if (row === "AUD1") return judgeAud1(view, input, checkpoints, final);
    if (row === "SND1") return judgeSnd1(input, checkpoints, final);
    throw new Error(`feedback oracle has no route for ${row}`);
}

async function judgeNoOrders(view, input, checkpoints, final) {
    const visible = checkpointForSave(input, checkpoints, "noorders-visible.png");
    const row = (visible.debug.ui_snapshot || []).find((item) => item.kind === "client_notice"
        && item.text === "This game defines no orders" && item.visible === true);
    assert(row, "NOORD renders the joined package notice through the engine-owned visible text row");
    assert(Array.isArray(row.rect) && row.rect.length === 4 && row.rect.every(Number.isFinite),
        "NOORD reports the actual post-layout notice rectangle");
    assert.equal(final.notices?.filter((notice) => notice.text === "This game defines no orders").length, 1,
        "NOORD emits the player notice only once");

    const image = decodePng(Buffer.from((await view.serverFile("noorders-visible.png.b64")).trim(), "base64"));
    const scale = Number(visible.scene.scale_factor) || 1;
    const [x, y, width, height] = row.rect.map((value) => value / scale);
    const left = Math.max(0, Math.floor(x));
    const top = Math.max(0, Math.floor(y));
    const right = Math.min(image.width, Math.ceil(x + width));
    const bottom = Math.min(image.height, Math.ceil(y + height));
    let whiteGlyphPixels = 0;
    for (let py = top; py < bottom; py += 1) {
        for (let px = left; px < right; px += 1) {
            const offset = (py * image.width + px) * 4;
            if (image.rgba[offset + 3] > 0 && image.rgba[offset] > 220 && image.rgba[offset + 1] > 220 && image.rgba[offset + 2] > 220) whiteGlyphPixels += 1;
        }
    }
    assert(whiteGlyphPixels >= 8, `NOORD screenshot contains rendered white notice glyphs in its actual layout rectangle (found ${whiteGlyphPixels})`);
}

async function judgeOrd2(view, input, checkpoints) {
    const box = input.find((entry) => entry.call === "drag" && entry.args?.[4] === "left");
    const order = input.find((entry) => entry.call === "click" && entry.args?.[2] === "right");
    assert(box, "ORD2 needs the physical selection box over the troop and post pixels");
    assert(order, "ORD2 needs the physical right-click order");
    const boxUp = pointerEvent(box, "pointerup");
    const orderUp = pointerEvent(order, "pointerup");
    const result = checkpointForSave(input, checkpoints, "ord2-result.png");
    const selectedFrame = frameForSequence(result.debug, boxUp.seq);
    const orderFrame = frameForSequence(result.debug, orderUp.seq);
    assert.equal(selectedFrame.selection.length, 40, "ORD2 box selection excludes the own static post");
    const accepted = (result.debug.results || []).filter((entry) => entry.status === "accepted");
    assert.equal(accepted.length, 40, "ORD2 browser order accepts the 40 own mobile units");
    assert.equal((result.debug.results || []).filter((entry) => entry.status === "refused").length, 0,
        "ORD2 browser selection contains no static or foreign units");
    assert(orderFrame.markers?.length >= 1, "ORD2 order shows a marker in the click frame");

    const before = checkpointForSave(input, checkpoints, "feedback-initial.png");
    const beforeUnits = new Map(before.debug.replicated_world.units.map((unit) => [unit.unit_id, unit]));
    const afterUnits = new Map(result.debug.replicated_world.units.map((unit) => [unit.unit_id, unit]));
    const post = [...afterUnits.values()].find((unit) => unit.content_id === "muster:post" && unit.owner_slot === result.debug.slot);
    assert(post, "ORD2 fixture has the own static post");
    assert.deepEqual(afterUnits.get(post.unit_id).pos, beforeUnits.get(post.unit_id).pos, "ORD2 static post stays in place");
    const tickRows = parseTickLog(await view.serverFile("server-tick.log"));
    const moved = accepted.filter((entry) => movedAfter(tickRows, entry.unit));
    assert.equal(moved.length, 40, "ORD2 tick log shows all 40 accepted mobile units start moving");
}

function judgeOrd6(input, checkpoints, debug) {
    const burstEntry = input.find((entry) => entry.call === "click" && entry.args?.[2] === "right" && entry.args?.[3]?.count === 100);
    assert(burstEntry, "ORD6 uses the bounded 100-click trusted input burst");
    assert.equal(burstEntry.frame_hold?.requested, true, "ORD6 starts the pre-WASM frame hold");
    assert.equal(burstEntry.frame_hold?.count, 100);
    assert.equal(burstEntry.frame_hold?.released, true, "ORD6 releases the frame hold after dispatch");
    assert(Number.isInteger(burstEntry.frame_hold?.held_callbacks), "ORD6 records actual held RAF callbacks");
    const releases = burstEntry.actual_events.filter((event) => event.type === "pointerup" && event.canvas_target);
    assert.equal(releases.length, 100, "ORD6 observed all 100 trusted canvas pointer releases");
    assert.equal(new Set(releases.map((event) => event.seq)).size, 100, "each physical release has a distinct recorded sequence");

    // The sound is off at the start. The row judges the error sounds, so it turns the sound on
    // first with one click on the visible switch, before the burst.
    const soundOn = assertSoundTurnedOn(input, checkpoints, "ord6-sound");
    assert(input.indexOf(burstEntry) > soundOn.clickIndex, "ORD6 turns the sound on before the burst");
    const before = checkpointForSave(input, checkpoints, "feedback-initial.png").debug;
    const burst = checkpointForSave(input, checkpoints, "ord6-burst.png").debug;
    assert.equal(burst.audio.sound_on, true, "ORD6 the sound switch is on at the burst");
    const priorSeq = Math.max(0, ...(before.results || []).map((entry) => entry.seq).filter(Number.isInteger));
    const expectedBurstSeqs = Array.from({ length: 100 }, (_, index) => priorSeq + index + 1);
    const appliedFrames = releases.map((event) => frameForSequence(burst, event.seq));
    assert(appliedFrames.every((frame) => frame.frame_index === appliedFrames[0].frame_index),
        "all 100 physical releases reach the same real core input frame");
    const emittedSeqs = appliedFrames[0].emitted_orders?.map((order) => order.seq).sort((a, b) => a - b) || [];
    assert.deepEqual(emittedSeqs, expectedBurstSeqs,
        "the one core frame records exactly 100 actual sequence-assigned emitted orders");

    const burstResults = (burst.results || []).filter((entry) => expectedBurstSeqs.includes(entry.seq));
    const resultSeqs = [...new Set(burstResults.map((entry) => entry.seq))].sort((a, b) => a - b);
    assert.deepEqual(resultSeqs, expectedBurstSeqs, "the server returns authoritative results for all 100 burst envelopes");
    const acceptedSeqs = [...new Set(burstResults.filter((entry) => entry.status === "accepted").map((entry) => entry.seq))];
    const failedSeqs = [...new Set(burstResults.filter((entry) => entry.status !== "accepted").map((entry) => entry.seq))];
    assert.equal(acceptedSeqs.length, 64, "ORD6 admits exactly the initial 64-envelope bucket capacity");
    assert.equal(failedSeqs.length, 36, "ORD6 rate-limits the remaining 36 same-frame envelopes");
    assert(burstResults.filter((entry) => entry.status !== "accepted").every((entry) => entry.reason === "rate_limited"),
        "every excess envelope carries the authoritative rate_limited result");
    const limited = burstResults.filter((entry) => entry.reason === "rate_limited");

    const resultFrames = burst.frames?.recent || [];
    const failedByFrame = new Map();
    for (const seq of new Set(limited.map((entry) => entry.seq))) {
        const frames = resultFrames.filter((frame) => frame.results?.some((entry) => entry.seq === seq));
        assert.equal(frames.length, 1, `rate-limited sequence ${seq} is retained in exactly one result frame`);
        const frame = frames[0];
        const text = (frame.feedback?.notices || []).map(noticeText).join(" ");
        assert(/too many orders/i.test(text), `rate-limited sequence ${seq} has same-frame player text`);
        assert(!text.includes("rate_limited"), `rate-limited sequence ${seq} never exposes its wire code`);
        assert(frame.feedback?.audio_started?.some((sound) => sound.category === "effects"),
            `rate-limited sequence ${seq} requests its same-frame authored error response`);
        failedByFrame.set(frame.frame_index, (failedByFrame.get(frame.frame_index) || 0) + 1);
    }
    const actualErrorSamples = (burst.audio.sink_observations || []).filter((sample) =>
        sample.sound_id === "muster:muster_effect_error" && sample.category === "effects");
    for (const [frameIndex, refusalCount] of failedByFrame) {
        const playerEntities = new Set(actualErrorSamples
            .filter((sample) => sample.request_frame_index === frameIndex)
            .map((sample) => sample.player_entity));
        assert(playerEntities.size >= refusalCount,
            `result frame ${frameIndex} observed one actual error OGG sink for each rate-limited envelope`);
    }

    const waitIndex = input.findIndex((entry, index) => index > input.indexOf(burstEntry) && entry.call === "wait" && entry.args?.[0] === 1000);
    assert(waitIndex >= 0, "ORD6 waits one second after the burst");
    const refillEntry = input.slice(waitIndex + 1).find((entry) => entry.call === "click" && entry.args?.[2] === "right" && entry.args?.[3] === undefined);
    assert(refillEntry, "ORD6 issues one separate ordinary click after the wait");
    assert(refillEntry.wall_time_ms - burstEntry.completed_wall_time_ms >= 1000,
        "the separate refill click follows at least one second after burst dispatch completes");
    const refillRelease = refillEntry.actual_events.find((event) => event.type === "pointerup" && event.canvas_target);
    assert(refillRelease && Number.isInteger(refillRelease.seq), "ORD6 refill click has its own trusted release sequence");
    const refillFrame = frameForSequence(debug, refillRelease.seq);
    const refillSeq = priorSeq + 101;
    assert(refillFrame.emitted_orders?.some((order) => order.seq === refillSeq),
        "the post-refill physical release emits the exact 101st envelope");
    const refillResultFrames = (debug.frames?.recent || []).filter((frame) => frame.results?.some((entry) => entry.seq === refillSeq));
    assert.equal(refillResultFrames.length, 1, "the 101st envelope has exactly one retained authoritative result frame");
    assert(refillResultFrames[0].results.some((entry) => entry.seq === refillSeq && entry.status === "accepted"),
        "the exact 101st envelope is accepted after refill");
}

async function judgeOrd7(view, input, checkpoints, debug) {
    const order = input.find((entry) => entry.call === "click" && entry.args?.[2] === "right");
    assert(order, "ORD7 uses a physical ground right-click");
    // The sound is off at the start. The row judges the response sound, so it turns the sound on
    // first with one click on the visible switch, before the order.
    const soundOn = assertSoundTurnedOn(input, checkpoints, "ord7-sound");
    assert(input.indexOf(order) > soundOn.clickIndex, "ORD7 turns the sound on before the order");
    const mouseUp = pointerEvent(order, "pointerup");
    const result = checkpointForSave(input, checkpoints, "ord7-result.png");
    const clickFrame = frameForSequence(result.debug, mouseUp.seq);
    assert(clickFrame.markers?.some((marker) => marker.kind === "move"),
        "ORD7 marker is present in the first pointer-release frame");
    assert(clickFrame.feedback?.audio_started?.some((sound) => sound.category === "voice"),
        "ORD7 response sound is present in the first pointer-release frame");
    const orderVoice = (result.debug.audio.sink_observations || []).find((sample) =>
        sample.sound_id === "muster:muster_voice_order" && sample.category === "voice"
            && sample.request_frame_index === clickFrame.frame_index
            && sample.observed_core_frame_index === clickFrame.frame_index);
    assert(orderVoice, "ORD7 observes the matching real response sink in the same core frame as the physical release");
    const emitted = clickFrame.emitted_orders?.find((command) => command.order === "muster:order_move");
    assert(emitted && Number.isInteger(emitted.seq), "ORD7 retains the actual emitted order sequence");
    const resultFrame = (debug.frames?.retained_input || []).find((frame) =>
        frame.results?.some((entry) => entry.seq === emitted.seq));
    assert(resultFrame, "ORD7 later retains the OrderResult for that exact emitted sequence");
    assert(clickFrame.frame_index < resultFrame.frame_index, "ORD7 marker and response sound precede OrderResult");
    const tickRows = parseTickLog(await view.serverFile("server-tick.log"));
    const unit = clickFrame.selection?.[0];
    assert(Number.isInteger(unit), "ORD7 click frame retains the selected unit id");
    const firstMove = firstMovementTick(tickRows, unit);
    const appliedTick = resultFrame.results.find((entry) => entry.seq === emitted.seq && entry.status === "accepted")?.tick;
    assert(Number.isInteger(firstMove) && Number.isInteger(appliedTick), "ORD7 has actual server movement and accepted-result ticks");
    assert.equal(firstMove, appliedTick, "ORD7 first movement occurs in the tick that drains the order");
}

async function judgeAlr1(view, input, checkpoints, params) {
    const attackOrigin = checkpointForSave(input, checkpoints, "alr1-attack-origin.png");
    const first = checkpointForSave(input, checkpoints, "alr1-first.png");
    const afterAlertClick = checkpointForSave(input, checkpoints, "alr1-after-alert-click.png");
    const late = checkpointForSave(input, checkpoints, "alr1-cooldown-4.png");
    const alerts = first.debug.alerts || [];
    assert(alerts.some((alert) => alert.id === "muster_unit_damaged" && alert.text === "Unit under attack"),
        "ALR1 real own-unit health loss creates the fixture's player-facing alert line");
    const causalFrames = first.debug.frames?.retained_input || [];
    const firstDamageFrame = causalFrames.find((frame) => frame.feedback?.alerts?.some((alert) => alert.id === "muster_unit_damaged"));
    assert(firstDamageFrame, "ALR1 retains the exact damage event beyond the recent-frame window");
    const ownUnit = firstDamageFrame.feedback.alerts.find((alert) => alert.id === "muster_unit_damaged").unit;
    assert(Number.isInteger(ownUnit), "ALR1 retains the actual selected own-unit identity");
    const attackClick = input.find((entry) => entry.call === "click" && entry.args?.[2] === "right");
    // The sound is off at the start. The row judges the alert sound, so it turns the sound on
    // first with one click on the visible switch, before the attack.
    const soundOn = assertSoundTurnedOn(input, checkpoints, "alr1-sound");
    assert(input.indexOf(attackClick) > soundOn.clickIndex, "ALR1 turns the sound on before the attack");
    assert.equal(late.debug.audio.sound_on, true, "ALR1 the sound switch is on at the alert");
    assert(attackClick, "ALR1 physically targets the visible foreign unit after selecting a current screenshot troop column");
    const attackIndex = input.indexOf(attackClick);
    const selectionClick = input.slice(0, attackIndex).reverse().find((entry) => entry.call === "click" && entry.args?.[2] === "left");
    assert(selectionClick, "ALR1 has the real pre-attack physical unit selection");
    const selectedBeforeAttack = frameForSequence(attackOrigin.debug, pointerEvent(selectionClick, "pointerup").seq).selection;
    assert.deepEqual(selectedBeforeAttack, [ownUnit], "damage event is matched to the unit selected before the attack began");
    const attackFrame = frameForSequence(first.debug, pointerEvent(attackClick, "pointerup").seq);
    const attackOrder = attackFrame.emitted_orders?.find((order) => order.order === "muster:order_attack"
        && order.units?.includes(ownUnit) && order.target?.Unit !== undefined);
    assert(attackOrder, "ALR1 emits the actual attack order for the selected moving own unit and visible foreign target");
    const acceptedAttack = (first.debug.frames?.retained_input || []).find((frame) =>
        frame.results?.some((result) => result.seq === attackOrder.seq && result.status === "accepted" && result.unit === ownUnit));
    assert(acceptedAttack, "ALR1 actual server OrderResult accepts the selected unit attack before damage feedback");
    const selectionBeforeAlertClick = first.debug.selection;
    assert(Array.isArray(selectionBeforeAlertClick), "ALR1 captures the live selection immediately before the alert click");
    const visibleAlert = (first.debug.ui_snapshot || []).find((row) => row.kind === "button"
        && row.text === "Unit under attack" && row.visible === true);
    assert(visibleAlert, "ALR1 exposes the nonempty alert text in a physically visible authored button row");
    assert.equal(visibleAlert.widget_action?.kind, "center_camera", "the visible alert row carries the typed camera-centre action");
    const firstAlertSaveIndex = input.findIndex((entry) => entry.call === "save" && entry.args?.[0] === "alr1-first.png");
    const alertClick = input.slice(firstAlertSaveIndex + 1).find((entry) => entry.call === "click" && entry.args?.[2] === "left");
    assert(alertClick, "ALR1 physically clicks the first visible alert row detected from the screenshot");
    const clickSequence = pointerEvent(alertClick, "pointerup").seq;
    const release = pointerEvent(alertClick, "pointerup");
    const scale = Number(first.scene.scale_factor) || 1;
    const [rx, ry, rw, rh] = visibleAlert.rect;
    assert(release.client_x * scale >= rx && release.client_x * scale <= rx + rw
        && release.client_y * scale >= ry && release.client_y * scale <= ry + rh,
    "ALR1 physical pointer release lands inside that actual rendered alert button rectangle");
    const action = visibleAlert.widget_action;
    const actualPing = (first.debug.alert_pings || []).find((ping) => ping.alert_id === "muster_unit_damaged"
        && ping.text === visibleAlert.text && ping.x === action?.x && ping.y === action?.y);
    assert(actualPing, "the physically clicked visible row action resolves to its matching real alert ping");
    assert.equal(actualPing.unit, ownUnit,
        "ALR1 minimap pixels and cooldown evidence must identify the same selected unit that received damage");
    const matchingAlertFrame = causalFrames.find((frame) => frame.replica_tick === actualPing.tick
        && frame.feedback?.alerts?.some((alert) => alert.id === actualPing.alert_id
            && alert.unit === actualPing.unit && alert.position?.x === actualPing.x && alert.position?.y === actualPing.y));
    assert(matchingAlertFrame, "the clicked row ping matches the exact retained rule, unit, tick, and position");
    assert(Number.isInteger(actualPing.unit), "the clicked row ping identifies its actual source unit");
    assert.equal(action.x, actualPing.x, "the clicked visible row action uses its matching ping's player-unit x position");
    assert.equal(action.y, actualPing.y, "the clicked visible row action uses its matching ping's player-unit y position");
    const firstImage = decodePng(Buffer.from((await view.serverFile("alr1-first.png.b64")).trim(), "base64"));
    const noPingImage = decodePng(Buffer.from((await view.serverFile("alr1-attack-origin.png.b64")).trim(), "base64"));
    assertAlr1MinimapPixels({
        positiveImage: firstImage,
        negativeImage: noPingImage,
        scene: first.scene,
        noPingScene: attackOrigin.scene,
        noPingDebug: attackOrigin.debug,
        debug: first.debug,
        ping: actualPing,
        params,
    });
    const clickFrame = frameForSequence(afterAlertClick.debug, clickSequence);
    assert.deepEqual(clickFrame.selection, selectionBeforeAlertClick, "alert centering preserves the live selection from immediately before the click");
    assert.equal(clickFrame.emitted_orders?.length, 0, "alert centering does not emit a network order");
    const beforeCamera = first.scene.clip_from_world;
    const afterCamera = afterAlertClick.scene.clip_from_world;
    assert(Array.isArray(beforeCamera) && Array.isArray(afterCamera) && beforeCamera.length === 16 && afterCamera.length === 16,
        "alert click checkpoints carry actual camera projection matrices");
    const beforePoint = projectGround(beforeCamera, actualPing);
    const afterPoint = projectGround(afterCamera, actualPing);
    assert(Math.hypot(...afterPoint) < Math.hypot(...beforePoint),
        "the physical alert click moves the actual alert position toward camera centre");
    assert(Math.hypot(...afterPoint) <= 0.08,
        "the clicked ping position is centred by the actual map-clamped camera action");
    const alertPixels = decodePng(Buffer.from((await view.serverFile("alr1-first.png.b64")).trim(), "base64"));
    assert(whitePixelsInRect(alertPixels, visibleAlert.rect, scale) >= 8,
        "ALR1 screenshot contains rendered alert-title glyphs in that visible button rectangle");

    const tickHz = late.debug.tick_hz;
    assert(Number.isInteger(tickHz) && tickHz > 0, "ALR1 reads the authoritative simulation tick rate");
    const world = late.debug.replicated_world;
    const trace = world.health_trace?.find((entry) => entry.unit_id === ownUnit)?.entries || [];
    const damageTicks = trace.slice(1).flatMap((entry, index) => entry.current < trace[index].current ? [entry.tick] : []);
    assert(damageTicks.length >= 2, "ALR1 witnesses at least two actual own-unit health-loss transitions");
    const alertCooldownSeconds = 1;
    assert(Number.isFinite(alertCooldownSeconds) && alertCooldownSeconds > 0,
        "ALR1 uses the designed finite one-second alert cooldown");
    const cooldownTicks = Math.ceil(alertCooldownSeconds * tickHz);
    const pings = (late.debug.alert_pings || []).filter((ping) => ping.alert_id === "muster_unit_damaged" && ping.unit === ownUnit);
    assert(pings.length > 0, "ALR1 produces an identified alert ping in the player feed");
    const ping = pings[0];
    assert(Number.isInteger(ping.tick) && Number.isFinite(ping.x) && Number.isFinite(ping.y),
        "ALR1 alert ping retains its actual simulation tick and location");
    assert.equal(ping.text, "Unit under attack");
    const retainedFrames = late.debug.frames?.retained_input || [];
    const emittedDamageTicks = [...new Set(retainedFrames.flatMap((frame) =>
        frame.feedback?.alerts?.some((alert) => alert.id === "muster_unit_damaged" && alert.unit === ownUnit)
            ? [frame.replica_tick] : []))].sort((left, right) => left - right);
    for (let index = 1; index < emittedDamageTicks.length; index += 1) {
        assert(emittedDamageTicks[index] - emittedDamageTicks[index - 1] >= cooldownTicks,
            "ALR1 every consecutive actual own-unit damage alert respects the designed cooldown");
    }
    const pingFrame = retainedFrames.find((frame) => frame.replica_tick === ping.tick
        && frame.feedback?.alerts?.some((alert) => alert.id === ping.alert_id && alert.unit === ping.unit));
    assert(pingFrame, "ALR1 ping traces back to the same actual per-frame alert rule and unit");
    const emittedPing = pingFrame.feedback.alerts.find((alert) => alert.id === ping.alert_id && alert.unit === ping.unit && pingFrame.replica_tick === ping.tick);
    assert.equal(emittedPing.position?.x, ping.x, "ALR1 ping uses the alert's actual unit x position");
    assert.equal(emittedPing.position?.y, ping.y, "ALR1 ping uses the alert's actual unit y position");
    const acceptedAttackResult = acceptedAttack.results.find((result) => result.seq === attackOrder.seq
        && result.unit === ownUnit && result.status === "accepted");
    assert(Number.isSafeInteger(acceptedAttackResult?.tick), "ALR1 binds damage to the actual accepted Unit Attack tick");
    const baselineTick = attackOrigin.debug.replicated_world?.tick;
    const baselineHealth = attackOrigin.debug.replicated_world?.healths?.find((entry) => entry.unit === ownUnit)?.current;
    assert(Number.isSafeInteger(baselineTick) && Number.isFinite(baselineHealth),
        "ALR1 captures the real selected-unit health and replica tick before the attack stimulus");
    const sourceHistory = world.health_trace?.find((entry) => entry.unit_id === ownUnit)?.entries || [];
    const samplesAtPing = sourceHistory.filter((sample) => Number.isSafeInteger(sample.tick)
        && sample.tick > baselineTick && sample.tick >= acceptedAttackResult.tick && sample.tick <= pingFrame.replica_tick)
        .sort((left, right) => left.tick - right.tick);
    const firstDrop = samplesAtPing.find((sample) => sample.current < baselineHealth);
    const precedingSample = firstDrop && sourceHistory.filter((sample) => sample.tick < firstDrop.tick)
        .sort((left, right) => left.tick - right.tick).at(-1);
    const latestAtPing = samplesAtPing.at(-1);
    assert(firstDrop && precedingSample && firstDrop.current < precedingSample.current
        && precedingSample.current >= baselineHealth && latestAtPing?.current < baselineHealth,
    "ALR1 ping observer follows an adjacent same-unit health drop after the pre-attack baseline and accepted attack");
    const finalOwnHealth = world.healths?.find((entry) => entry.unit === ownUnit)?.current;
    assert(Number.isFinite(finalOwnHealth) && finalOwnHealth < baselineHealth,
        "ALR1 final capture retains the same surviving ping source at lower health than before the attack");
    assert((late.debug.audio.sink_observations || []).some((sample) =>
        sample.sound_id === "muster:muster_alert" && sample.category === "alerts"
            && sample.request_frame_index === pingFrame.frame_index),
    "ALR1 observed the real alert OGG sink requested in the ping's actual frame");
    const suppressedDrop = damageTicks.find((dropTick) => dropTick > pingFrame.replica_tick
        && dropTick - pingFrame.replica_tick < cooldownTicks);
    const noRepeatPing = suppressedDrop && !pings.some((candidate) =>
        candidate.tick > pingFrame.replica_tick && candidate.tick <= suppressedDrop);
    const noRepeatAlertFrame = suppressedDrop && !retainedFrames.some((frame) =>
        frame.replica_tick > pingFrame.replica_tick && frame.replica_tick <= suppressedDrop
        && frame.feedback?.alerts?.some((alert) => alert.id === "muster_unit_damaged" && alert.unit === ownUnit));
    assert(suppressedDrop && noRepeatPing && noRepeatAlertFrame,
        "ALR1 has a second actual health drop after the emitted ping within the one-second cooldown, with no repeated alert");
    assertObservedSound(first.debug.audio, "muster:muster_alert", "alerts",
        "ALR1 observed the real alert OGG sink in the alerts category");
}

function assertSoundTurnedOn(input, checkpoints, label) {
    return assertSwitchClickByLabel(input, (name) => checkpointForSave(input, checkpoints, name), label, false);
}

// SND1: the sound is off at the start. A select and an order make their sound requests and no sink
// starts. One click on the switch turns it on and the next order plays. One more click turns it
// off and the next order is silent again.
function judgeSnd1(input, checkpoints, final) {
    const named = (name) => checkpointForSave(input, checkpoints, name);
    const wordsOf = (checkpoint) => (checkpoint.debug.ui_snapshot || []).find((row) => row.kind === "sound_switch")?.text;
    const requested = (debug, category, id) => (debug.frames?.retained_input || []).some((frame) =>
        (frame.feedback?.audio_started || []).some((sound) => sound.category === category && sound.id === id));

    const start = named("snd1-default-off.png");
    assert.equal(start.debug.audio.sound_on, false, "SND1 the sound is off by default");
    assert.equal(wordsOf(start), "Sound: off", "SND1 the switch says Sound: off at the start");
    assert(start.debug.audio.gesture_seen === true && start.debug.audio.context_states?.length > 0 &&
        start.debug.audio.context_states.every((state) => state === "running"),
    "SND1 every browser audio context runs, so the silence below is the switch and not a locked browser");

    const silent = named("snd1-silent.png").debug;
    assert.equal(silent.audio.sound_on, false);
    assert(requested(silent, "voice", "muster:muster_voice_select"), "SND1 the select made its sound request while the switch was off");
    assert(requested(silent, "voice", "muster:muster_voice_order"), "SND1 the order made its sound request while the switch was off");
    assert.equal(silent.audio.player_entities, 0, "SND1 no sound player exists while the switch is off");
    assert.deepEqual(silent.audio.sink_observations, [], "SND1 no sink started while the switch was off");
    assert.deepEqual(silent.audio.peak_active_by_category, {}, "SND1 no sound was live at any time while the switch was off");

    const on = assertSoundTurnedOn(input, checkpoints, "snd1-on");
    const playing = named("snd1-playing.png").debug.audio;
    assert.equal(playing.sound_on, true);
    const onFrame = on.frame.frame_index;
    const sinks = playing.sink_observations || [];
    assert(sinks.some((sink) => sink.sound_id === "muster:muster_voice_order" && sink.category === "voice" && sink.gain > 0),
        "SND1 the order after the click plays a real voice sink at a positive gain");
    assert(sinks.every((sink) => sink.request_frame_index > onFrame),
        "SND1 every sink was requested after the click: no sound from the silent period was queued");

    const off = assertSwitchClickByLabel(input, named, "snd1-off", true);
    const after = named("snd1-off-again.png").debug.audio;
    assert.equal(after.sound_on, false, "SND1 the second click turns the sound off again");
    assert.equal(after.sink_observations.length, sinks.length, "SND1 the order after the second click starts no new sink");
    assert.equal(after.player_entities, 0, "SND1 no sound player exists after the second click");
    assert(off.clickIndex > on.clickIndex);
    assert.equal(final.audio.sound_on, false, "SND1 the sound switch is off at the end");
}

function projectGround(matrix, position) {
    const x = position.x;
    const z = position.y;
    const clipX = matrix[0] * x + matrix[8] * z + matrix[12];
    const clipY = matrix[1] * x + matrix[9] * z + matrix[13];
    const clipW = matrix[3] * x + matrix[11] * z + matrix[15];
    assert(Number.isFinite(clipW) && clipW > 0, "alert ground point remains in front of the actual camera");
    const point = [clipX / clipW, clipY / clipW];
    assert(point.every(Number.isFinite), "camera projection of alert position is finite");
    return point;
}

function whitePixelsInRect(image, rect, scale) {
    const left = Math.max(0, Math.floor(rect[0] / scale));
    const top = Math.max(0, Math.floor(rect[1] / scale));
    const right = Math.min(image.width, Math.ceil((rect[0] + rect[2]) / scale));
    const bottom = Math.min(image.height, Math.ceil((rect[1] + rect[3]) / scale));
    let count = 0;
    for (let y = top; y < bottom; y += 1) {
        for (let x = left; x < right; x += 1) {
            const offset = (y * image.width + x) * 4;
            if (image.rgba[offset + 3] > 0 && image.rgba[offset] > 220
                && image.rgba[offset + 1] > 220 && image.rgba[offset + 2] > 220) count += 1;
        }
    }
    return count;
}

function assertAlr1MinimapPixels({ positiveImage, negativeImage, scene, noPingScene, noPingDebug, debug, ping, params }) {
    const mapSize = params.map_size;
    assert(Array.isArray(mapSize) && mapSize.length === 2 && mapSize.every((value) => Number.isFinite(value) && value > 0),
        "ALR1 needs the authored map dimensions to project the real alert position");
    const widget = (scene.widgets || []).find((entry) => entry.id === "minimap");
    const noPingWidget = (noPingScene.widgets || []).find((entry) => entry.id === "minimap");
    assert(widget && Array.isArray(widget.rect) && widget.rect.length === 4 && widget.rect.every(Number.isFinite),
        "ALR1 uses the actual C84 minimap widget rectangle");
    assert(widget.rect[2] > 0 && widget.rect[3] > 0,
        "ALR1 minimap widget has positive actual rendered dimensions");

    const pingColour = parseFeedbackHex(params.ping_colour);
    const orange = parseFeedbackHex(params.slot_colours?.[1]);
    assert(Math.hypot(orange[0] - pingColour[0], orange[1] - pingColour[1], orange[2] - pingColour[2]) > 28,
        "ALR1 ping threshold separates the configured ping from slot 1 orange");
    assert(!((noPingDebug.alert_pings || []).some((candidate) =>
        candidate.alert_id === ping.alert_id && candidate.unit === ping.unit)),
    "ALR1 orange/no-ping image precedes the same unit's actual damage ping");
    assert(noPingWidget && Array.isArray(noPingWidget.rect),
        "ALR1 pre-attack C84 checkpoint exposes the actual minimap widget rectangle");
    assert.deepEqual(noPingWidget.rect, widget.rect,
        "ALR1 no-ping and ping screenshots use the same authored minimap rectangle");

    const rect = widget.rect;
    const point = [rect[0] + ping.x / mapSize[0] * rect[2], rect[1] + (1 - ping.y / mapSize[1]) * rect[3]];
    const pingPixels = pixelsNearColour(positiveImage, point, 7, pingColour, 28);
    const noPingPixels = pixelsNearColour(negativeImage, point, 7, pingColour, 28);
    assert(noPingPixels.length < 2,
        "ALR1 pre-attack screenshot has no configured ping-colour pixels at the actual future alert position");
    assert(pingPixels.length >= 4,
        "ALR1 actual alert must add rendered minimap pixels at its event-bound unit and world position");
    const bounds = pixelBounds(pingPixels);
    assert(bounds.width >= 2 && bounds.height >= 2 && bounds.width <= 15 && bounds.height <= 15,
        "ALR1 minimap ping pixels form a compact visible cross at the actual alert position");

    const owner = new Map((debug.replicated_world?.units || []).map((unit) => [unit.unit_id, unit.owner_slot]));
    const foreign = (debug.replicated_world?.units || []).filter((unit) => owner.get(unit.unit_id) === 1
        && Array.isArray(unit.pos) && unit.pos.length === 2 && unit.pos.every(Number.isFinite))
        .sort((left, right) => feedbackDistance(right.pos, [ping.x, ping.y])
            - feedbackDistance(left.pos, [ping.x, ping.y]));
    const orangeControl = foreign.map((unit) => {
        const center = [rect[0] + unit.pos[0] / mapSize[0] * rect[2],
            rect[1] + (1 - unit.pos[1] / mapSize[1]) * rect[3]];
        return { center, pixels: pixelsNearColour(positiveImage, center, 3, orange, 38) };
    }).find((candidate) => candidate.pixels.length >= 2);
    assert(orangeControl,
        "ALR1 screenshot must show a real slot 1 orange minimap marker for the negative colour control");
    assert(pixelsNearColour(positiveImage, orangeControl.center, 3, pingColour, 28).length < 2,
        "ALR1 configured ping pixels do not falsely match the real orange slot 1 marker");
}

function pixelsNearColour(image, [cx, cy], radius, colour, tolerance) {
    const matches = [];
    for (let y = Math.max(0, Math.floor(cy - radius)); y <= Math.min(image.height - 1, Math.ceil(cy + radius)); y += 1) {
        for (let x = Math.max(0, Math.floor(cx - radius)); x <= Math.min(image.width - 1, Math.ceil(cx + radius)); x += 1) {
            const pixel = pixelAt(image, x, y);
            if (pixel && pixel[3] >= 245 && Math.hypot(pixel[0] - colour[0], pixel[1] - colour[1], pixel[2] - colour[2]) <= tolerance) {
                matches.push([x, y]);
            }
        }
    }
    return matches;
}

function pixelBounds(points) {
    assert(points.length > 0, "pixel bounds need a visible component");
    const xs = points.map((point) => point[0]);
    const ys = points.map((point) => point[1]);
    return { width: Math.max(...xs) - Math.min(...xs) + 1, height: Math.max(...ys) - Math.min(...ys) + 1 };
}

function feedbackDistance(left, right) {
    return Math.hypot(left[0] - right[0], left[1] - right[1]);
}

function parseFeedbackHex(value) {
    assert(/^#[0-9a-fA-F]{6}$/.test(value), `ALR1 invalid colour parameter ${value}`);
    return [1, 3, 5].map((index) => Number.parseInt(value.slice(index, index + 2), 16));
}

async function judgeAud1(view, input, checkpoints, final) {
    const initial = checkpointForSave(input, checkpoints, "feedback-initial.png").debug.audio;
    const resumed = checkpointForSave(input, checkpoints, "aud1-after-resume.png").debug.audio;
    const volume = checkpointForSave(input, checkpoints, "aud1-volume-menu.png").debug.audio;
    const reloaded = checkpointForSave(input, checkpoints, "aud1-reloaded.png").debug.audio;
    const rejoinStart = checkpointForSave(input, checkpoints, "aud1-rejoin-start.png");
    const rejoined = checkpointForSave(input, checkpoints, "aud1-rejoined.png");
    const resumedAfterReloadGesture = checkpointForSave(input, checkpoints, "after-reload-context-resumed.png");
    const voicePlayback = checkpointForSave(input, checkpoints, "live-alert-sink.png").debug.audio;
    const effectsPlayback = checkpointForSave(input, checkpoints, "live-effects-sink.png").debug.audio;
    const resumedAfterReload = checkpointForSave(input, checkpoints, "after-reload-live-alert-sink.png").debug.audio;
    const effectsAfterReload = checkpointForSave(input, checkpoints, "after-reload-live-effects-sink.png").debug.audio;
    const initialDebug = checkpointForSave(input, checkpoints, "feedback-initial.png").debug;
    const reloadDebug = rejoined.debug;
    await assertMenuSliderClicksWereCaptured(view, input, checkpoints, checkpointForSave(input, checkpoints, "aud1-volume-menu.png").debug);
    const authoredHud = await view.serverFile("pack/muster/interface/hud.ron");
    const authoredPost = await view.serverFile("pack/muster/units/post.ron");
    const postName = readAuthoredString(authoredPost, "name", false);
    assert(typeof postName === "string" && postName.length > 0, "AUD1 reads the actual packed static unit name");
    const localReason = readAuthoredString(authoredHud, "no_applicable_order")
        ?? readAuthoredString(authoredHud, "not_applicable") ?? "Cannot do that";
    const expectedLocalRefusal = `${postName}: ${localReason}`;
    assertAud1CombatProof(input, checkpoints, "category-scan.png", "live-alert-sink.png", "after-combat-scan.png", "live-effects-sink.png", expectedLocalRefusal);
    assertAud1CombatProof(input, checkpoints, "after-reload-category-scan.png", "after-reload-live-alert-sink.png", "after-reload-after-combat-scan.png", "after-reload-live-effects-sink.png", expectedLocalRefusal);
    // The sound is off by default, also after the first gesture has resumed every context: no sink
    // before the switch. One click on the visible switch turns it on. It is still on after the reload.
    assert.equal(initial.sound_on, false, "AUD1 the sound switch is off at the start");
    assert.equal(resumed.sound_on, false, "AUD1 the sound switch is still off after the first gesture");
    assert.equal(resumed.player_entities, 0, "AUD1 no sound player exists after the first gesture while the switch is off");
    assert.deepEqual(resumed.sink_observations, [], "AUD1 no sink started before the player turned the sound on");
    assertSoundTurnedOn(input, checkpoints, "aud1-sound");
    assert.equal(final.audio.sound_on, true, "AUD1 the sound switch stays on after the reload (it is stored)");
    assert.equal(reloaded.sound_on, true, "AUD1 the stored switch loads after the reload");
    assert(initial.context_states?.length > 0, "AUD1 observes each actual browser AudioContext object");
    assert(initial.context_states.every((state) => state === "suspended"),
        "AUD1 every actual AudioContext starts suspended before the first click");
    assert.equal(initial.context_state, "suspended", "AUD1 actual AudioContext starts suspended before the first click");
    assert.equal(initial.player_entities, 0, "AUD1 creates no AudioPlayer while the actual context is suspended");
    assert(resumed.context_states?.length > 0 && resumed.context_states.every((state) => state === "running"),
        "AUD1 every actual AudioContext resumes after the first click");
    assert.equal(resumed.context_state, "running", "AUD1 actual AudioContext resumes after the first click");
    assertObservedSound(volume, "muster:muster_voice_select", "voice",
        "AUD1 plays the real select OGG in voice");
    assertObservedSound(volume, "muster:muster_voice_order", "voice",
        "AUD1 plays the real order OGG in voice");
    const levels = { master: 0.9, voice: 0.8, alerts: 0.7, effects: 0.6 };
    for (const [category, expected] of Object.entries(levels)) {
        assertClose(volume.categories?.[category], expected, `AUD1 ${category} control changes its actual gain`);
        assertClose(reloaded.categories?.[category], expected, `AUD1 ${category} gain survives reload`);
        assertClose(final.audio.categories?.[category], expected, `AUD1 final debug reports restored ${category} gain`);
    }
    assertSinkGain(voicePlayback, "voice", levels.master * levels.voice, "AUD1 live voice sink uses the adjusted master and voice gain");
    assertSinkGain(voicePlayback, "alerts", levels.master * levels.alerts, "AUD1 live alert sink uses the adjusted master and alert gain");
    assertSinkGain(effectsPlayback, "effects", levels.master * levels.effects, "AUD1 live effects sink uses the adjusted master and effects gain");
    assert(voicePlayback.peak_active_by_category?.voice <= 1,
        "AUD1 sampled real voice AudioPlayer entities and kept the peak at one");
    assertSinkGain(resumedAfterReload, "voice", levels.master * levels.voice,
        "AUD1 reload restores effective gain on a newly played real voice sink");
    assertSinkGain(resumedAfterReload, "alerts", levels.master * levels.alerts,
        "AUD1 reload restores effective gain on a newly played real alert sink");
    assertSinkGain(effectsAfterReload, "effects", levels.master * levels.effects,
        "AUD1 reload restores effective gain on a newly played real effects sink");
    assert(resumedAfterReload.context_states?.length > 0 && resumedAfterReload.context_states.every((state) => state === "running"),
        "AUD1 the first post-reload key gesture resumes every actual context");
    assert(reloaded.context_states?.length > 0
        && reloaded.context_states.every((state) => state === "running" || state === "suspended"),
    "AUD1 observes the fresh actual post-reload AudioContext objects before the next physical gesture");
    assert.equal(reloaded.gesture_seen, false, "AUD1 reload has not reused a prior gesture authorization");
    assert.equal(reloaded.player_entities, 0, "AUD1 has no actual audio player before its new post-reload gesture");
    const reloadCall = input.find((entry) => entry.call === "reload");
    assert(reloadCall && Number.isFinite(reloadCall.wall_time_ms) &&
        Number.isFinite(rejoined.wall_time_ms) &&
        rejoined.wall_time_ms >= reloadCall.wall_time_ms &&
        rejoined.wall_time_ms - reloadCall.wall_time_ms <= 15000,
    "AUD1 actual same-tab reload visibly rejoins within the designed 15-second bound");
    assert(rejoinStart.debug.rejoin?.stored_identity === true,
        "AUD1 first ready paint belongs to the actual stored browser identity");
    const retry = rejoined.debug.rejoin;
    assert(retry?.stored_identity === true && retry.hellos_sent >= 1,
        "AUD1 actual stored browser identity sends its real rejoin Hello");
    assert(Number.isFinite(retry.elapsed_secs) && retry.elapsed_secs >= 0 && retry.elapsed_secs < 15,
        "AUD1 actual Welcome ends the bounded retry clock before its deadline");
    const helloTimes = retry.hello_elapsed_secs || [];
    assert.equal(helloTimes.length, retry.hellos_sent,
        "AUD1 retains one elapsed-time observation per actual Hello send");
    assert(helloTimes.every((time) => Number.isFinite(time) && time >= 0 && time < 15),
        "AUD1 no actual Hello is sent beyond the retry deadline");
    for (let index = 1; index < helloTimes.length; index += 1) {
        assert(helloTimes[index] - helloTimes[index - 1] >= 1 - 1e-6,
            "AUD1 actual Hello sends respect the one-second cadence");
    }
    assert.equal(rejoined.debug.phase, "Joined { slot: 0 }", "AUD1 stops retrying only after actual Welcome joins slot zero");
    assert.equal(retry.text, null, "AUD1 clears the countdown after actual Welcome");
    assert.equal(final.rejoin?.hellos_sent, retry.hellos_sent,
        "AUD1 sends no further Hello after its successful rejoin while new category sounds play");
    const countdownFrames = input.filter((entry) => entry.call === "save" &&
        /^aud1-rejoin-(start|[0-9]+)\.png$/.test(entry.args?.[0] || ""))
        .map((entry) => ({ name: entry.args[0], checkpoint: checkpointForSave(input, checkpoints, entry.args[0]) }))
        .map((entry) => ({ ...entry, rows: (entry.checkpoint.debug.ui_snapshot || []).filter((row) =>
            row.kind === "client_notice" && row.visible === true &&
            /^Reconnecting to your game\n(?:1[0-5]|[1-9]) s$/.test(row.text || "")) }))
        .filter((entry) => entry.rows.length > 0);
    if (retry.duplicate_rejections > 0) {
        assert(retry.hellos_sent > retry.duplicate_rejections,
            "AUD1 sends a new actual Hello after the observed duplicate-window refusals");
        assert(countdownFrames.length > 0,
            "AUD1 observed duplicate waiting renders an actual shell countdown");
    }
    for (const frame of countdownFrames) {
        assert.equal(frame.rows.length, 1, "AUD1 waiting paints exactly one actual countdown row");
        assert(frame.checkpoint.debug.rejoin?.stored_identity === true,
            "AUD1 rendered countdown belongs to its actual stored identity");
        const image = decodePng(Buffer.from((await view.serverFile(`${frame.name}.b64`)).trim(), "base64"));
        assert(whitePixelsInRect(image, frame.rows[0].rect, Number(frame.checkpoint.scene.scale_factor) || 1) >= 8,
            "AUD1 countdown has actual rendered glyphs within its computed visible shell rectangle");
    }
    if (countdownFrames.length >= 2) {
        const first = countdownFrames[0], last = countdownFrames[countdownFrames.length - 1];
        const elapsed = last.checkpoint.debug.rejoin.elapsed_secs - first.checkpoint.debug.rejoin.elapsed_secs;
        assert(elapsed >= 0, "AUD1 countdown frames retain their actual monotonic retry clock");
        if (elapsed >= 1) {
            const remaining = (frame) => Number(frame.rows[0].text.split("\n")[1].replace(" s", ""));
            assert(remaining(last) < remaining(first),
                "AUD1 countdown decreases across actual waiting paints at least one second apart");
        }
    }
    assert(resumedAfterReloadGesture.debug.audio.gesture_seen === true
        && resumedAfterReloadGesture.debug.audio.context_states?.length > 0
        && resumedAfterReloadGesture.debug.audio.context_states.every((state) => state === "running"),
    "AUD1 records a new physical post-reload gesture resuming every live AudioContext");
    assertLiveVoiceCount(volume, "AUD1 never has more than one live voice player");
    assert(input.some((entry) => entry.call === "reload"), "AUD1 performs the actual same-tab reload");
    const serverLog = await view.serverFile("server.log");
    const joinedPlayers = [...serverLog.matchAll(/turts_server: player ([0-9]+) took slot ([0-9]+)/g)]
        .filter((match) => match[2] === "0")
        .map((match) => match[1]);
    assert(joinedPlayers.length >= 2, "AUD1 records slot zero on both the original join and rejoin in the real server log");
    const playerId = initialDebug.player_id_decimal;
    assert(typeof playerId === "string" && /^[0-9]+$/.test(playerId),
        "AUD1 captures the exact actual u64 player identity as a decimal string before reload");
    assert.equal(initialDebug.slot, 0, "AUD1 captures the actual first slot before reload");
    assert.equal(reloadDebug.player_id_decimal, playerId,
        "AUD1 reload checkpoint reports the same exact actual player identity");
    assert.equal(reloadDebug.slot, 0, "AUD1 reload checkpoint reports the actual rejoined slot zero");
    assert.equal(reloadDebug.player_id_decimal, playerId,
        "AUD1 actual post-rejoin client retains the exact lossless player identity");
    assert(joinedPlayers.includes(playerId), "AUD1 server log identity exactly matches the actual client resource");
    assert(joinedPlayers.every((playerId) => playerId === joinedPlayers[0]),
        "AUD1 preserves the exact full decimal player identity across the same-tab reload");
}

// Fixed fixture fields contain RON strings with JSON-compatible escapes. Read the actual
// packed content so the oracle follows the authored override before its engine fallback.
function readAuthoredString(source, key, quotedKey = true) {
    const field = quotedKey ? `"${key}"` : `\\b${key}`;
    const match = source.match(new RegExp(`${field}\\s*:\\s*("(?:\\\\.|[^"\\\\])*")`));
    return match ? JSON.parse(match[1]) : undefined;
}

function assertAud1CombatProof(input, checkpoints, scanName, alertName, staticScanName, effectsName, expectedLocalRefusal) {
    const alert = checkpointForSave(input, checkpoints, alertName);
    const effects = checkpointForSave(input, checkpoints, effectsName);
    const scanSave = input.findIndex((entry) => entry.call === "save" && entry.args?.[0] === scanName);
    const clicks = input.slice(scanSave + 1).filter((entry) => entry.call === "click");
    const originClick = clicks.find((entry) => entry.args?.[2] === "left");
    const targetClick = clicks.find((entry) => entry.args?.[2] === "right");
    assert(originClick && targetClick, `${scanName} drives a real selected-unit attack from current rendered pixels`);
    const selectedFrame = frameForSequence(alert.debug, pointerEvent(originClick, "pointerup").seq);
    const attackUnit = selectedFrame.selection?.[0];
    assert(Number.isInteger(attackUnit), `${scanName} physical front-column click selects one actual own unit`);
    const attackFrame = frameForSequence(alert.debug, pointerEvent(targetClick, "pointerup").seq);
    assert.deepEqual(attackFrame.selection, [attackUnit], `${scanName} keeps the selected attacker through the physical target click`);
    const attackOrder = attackFrame.emitted_orders?.find((order) => order.order === "muster:order_attack"
        && order.units?.includes(attackUnit) && order.target?.Unit !== undefined);
    assert(attackOrder, `${scanName} emits the actual attack order for the selected unit and visible target`);
    const accepted = (alert.debug.frames?.retained_input || []).find((frame) =>
        frame.results?.some((result) => result.seq === attackOrder.seq && result.status === "accepted" && result.unit === attackUnit));
    assert(accepted, `${scanName} has an actual accepted OrderResult for its physical attack`);
    assert((alert.debug.alert_pings || []).some((ping) => ping.alert_id === "muster_unit_damaged"),
        `${scanName} causes the fixture's actual own-unit damage alert`);
    assertObservedSound(alert.debug.audio, "muster:muster_alert", "alerts",
        `${scanName} observes the actual alert OGG sink after lawful attack input`);

    const staticScan = checkpointForSave(input, checkpoints, staticScanName);
    const staticScanSave = input.findIndex((entry) => entry.call === "save" && entry.args?.[0] === staticScanName);
    const staticClick = input.slice(staticScanSave + 1).find((entry) => entry.call === "click" && entry.args?.[2] === "left");
    assert(staticClick, `${staticScanName} physically selects the static shape target from the current screenshot`);
    const effectsFrame = frameForSequence(effects.debug, pointerEvent(staticClick, "pointerup").seq);
    const staticUnit = effectsFrame.selection?.[0];
    const description = staticScan.debug.units?.find((unit) => unit.unit_id === staticUnit);
    assert(description?.shape?.Cuboid, `${staticScanName} click selected an actual rendered cuboid structure, not a narrow troop column`);
    const staticReplica = staticScan.debug.replicated_world?.units?.find((unit) => unit.unit_id === staticUnit);
    assert(staticReplica?.content_id === "muster:post" && staticReplica.owner_slot === 0,
        `${staticScanName} selected the actual owned Post represented by that screenshot shape`);
    const staticSelectionIndex = input.indexOf(staticClick);
    const refusalClick = input.slice(staticSelectionIndex + 1).find((entry) => entry.call === "click" && entry.args?.[2] === "right");
    assert(refusalClick, `${staticScanName} physically right-clicks ground with the actual Post selected`);
    const refusalFrame = frameForSequence(effects.debug, pointerEvent(refusalClick, "pointerup").seq);
    assert.deepEqual(refusalFrame.selection, [staticUnit], `${staticScanName} local refusal belongs to the actual selected Post`);
    assert.deepEqual(refusalFrame.emitted_orders || [], [], `${staticScanName} immobile-unit refusal sends no world order`);
    assert((refusalFrame.feedback?.notices || []).some((notice) => {
        const text = notice.text || notice.message || "";
        return text === expectedLocalRefusal;
    }), `${staticScanName} exact physical refusal frame has the actual packed unit name and authored reason`);
    const refusalSound = (effects.debug.audio.sink_observations || []).find((sample) =>
        sample.sound_id === "muster:muster_effect_error" && sample.category === "effects" &&
        sample.request_frame_index === refusalFrame.frame_index);
    assert(refusalSound, `${staticScanName} actual effects OGG onset was requested by that precise static-unit refusal frame`);
    const refusalGain = (effects.debug.audio.gain_samples || []).find((sample) =>
        sample.player_entity === refusalSound.player_entity && sample.category === "effects" &&
        sample.request_frame_index === refusalFrame.frame_index && sample.frame_index >= refusalSound.frame_index);
    assert(refusalGain, `${staticScanName} actual refusal player has a real sink gain observation`);
    assertClose(refusalGain.gain, 0.9 * 0.6, `${staticScanName} that exact refusal sink uses the persisted effects gain`);
    assertSinkGain(alert.debug.audio, "alerts", 0.9 * 0.7,
        `${scanName} alert playback uses the persisted alerts category gain`);
    assertSinkGain(effects.debug.audio, "effects", 0.9 * 0.6,
        `${staticScanName} static-target response uses the persisted effects category gain`);
    const staticClickFrame = frameForSequence(effects.debug, pointerEvent(staticClick, "pointerup").seq);
    assert.deepEqual(staticClickFrame.selection, [staticUnit],
        `${staticScanName} actual effects response follows selecting the visible static structure`);
}

function assertSinkGain(audio, category, expected, message) {
    const samples = audio.gain_samples?.filter((sample) => sample.category === category) || [];
    assert(samples.length > 0, `${message}: no real Bevy sink observation for ${category}`);
    const latest = samples[samples.length - 1];
    assert(Number.isInteger(latest.player_entity), `${message}: observation includes a real player entity id`);
    assert(Number.isInteger(latest.frame_index), `${message}: observation includes the observed frame`);
    assertClose(latest.gain, expected, message);
}

function assertObservedSound(audio, soundId, category, message) {
    const samples = audio.sink_observations || [];
    assert(samples.some((sample) => sample.sound_id === soundId && sample.category === category), message);
}

function assertLiveVoiceCount(audio, message) {
    assert(audio.active_by_category && typeof audio.active_by_category === "object" &&
        !Array.isArray(audio.active_by_category), "AUD1 actual active-category map must exist");
    const voice = Object.hasOwn(audio.active_by_category, "voice") ? audio.active_by_category.voice : 0;
    assert(Number.isInteger(voice) && voice >= 0 && voice <= 1, message);
}

async function assertMenuSliderClicksWereCaptured(view, input, checkpoints, debug) {
    const saveIndex = input.findIndex((entry) => entry.call === "save" && entry.args?.[0] === "aud1-volume-menu.png");
    const menuOpenIndex = input.findLastIndex((entry, index) => index < saveIndex && entry.call === "key" && entry.args?.[0] === "F10");
    assert(menuOpenIndex >= 0, "AUD1 opens the existing settings menu with its real F10 key");
    const clicks = input.slice(menuOpenIndex + 1, saveIndex).filter((entry) => entry.call === "click");
    const expected = [[831, 319], [794, 365], [757, 411], [720, 457]];
    assert.equal(clicks.length, expected.length, "AUD1 physically uses the four visible volume sliders");
    for (let index = 0; index < clicks.length; index += 1) {
        assert.deepEqual(clicks[index].args.slice(0, 2), expected[index], "AUD1 slider click uses its visible track");
        const category = ["master", "voice", "alerts", "effects"][index];
        const beforeName = `aud1-volume-before-${category}.png`;
        const before = checkpointForSave(input, checkpoints, beforeName);
        const beforeSave = input.findIndex((entry) => entry.call === "save" && entry.args?.[0] === beforeName);
        assert(beforeSave > menuOpenIndex && beforeSave < input.indexOf(clicks[index]),
            `AUD1 ${category} current menu pixels precede its physical click`);
        const image = decodePng(Buffer.from((await view.serverFile(`${beforeName}.b64`)).trim(), "base64"));
        const x = expected[index][0], y = expected[index][1];
        const offset = (y * image.width + x) * 4;
        assert(x >= 0 && y >= 0 && x < image.width && y < image.height && image.rgba[offset + 3] >= 245,
            `AUD1 ${category} physical click point is an actual opaque captured pixel`);
        const rgb = [image.rgba[offset], image.rgba[offset + 1], image.rgba[offset + 2]];
        assert(Math.hypot(rgb[0] - 51, rgb[1] - 61, rgb[2] - 77) <= 12 ||
            Math.hypot(rgb[0] - 102, rgb[1] - 191, rgb[2] - 242) <= 12,
            `AUD1 ${category} click lands on its actually visible slider track or thumb`);
        assert(before.debug.phase === "Joined { slot: 0 }", "AUD1 menu changes occur in the actual joined game");
        const release = clicks[index].actual_events.find((event) => event.type === "pointerup" && event.canvas_target);
        assert(release && Number.isInteger(release.seq), "AUD1 slider interaction uses a trusted canvas release");
        const frame = frameForSequence(debug, release.seq);
        assert.deepEqual(frame.emitted_orders, [], "the full-screen settings overlay prevents a slider click from issuing a world order");
        const afterName = `aud1-volume-after-${category}.png`;
        const after = checkpointForSave(input, checkpoints, afterName);
        const afterSave = input.findIndex((entry) => entry.call === "save" && entry.args?.[0] === afterName);
        assert(afterSave > input.indexOf(clicks[index]), `AUD1 ${category} completion paint follows its physical click`);
        assertClose(after.debug.audio.categories?.[category], [0.9, 0.8, 0.7, 0.6][index],
            `AUD1 ${category} actual setting has changed before the next pointer move`);
        const afterImage = decodePng(Buffer.from((await view.serverFile(`${afterName}.b64`)).trim(), "base64"));
        const thumbOffset = (y * afterImage.width + x) * 4;
        assert(Math.hypot(afterImage.rgba[thumbOffset] - 102, afterImage.rgba[thumbOffset + 1] - 191,
            afterImage.rgba[thumbOffset + 2] - 242) <= 12,
        `AUD1 ${category} actual completion screenshot shows its thumb at the requested value`);
        const nextMoveIndex = input.findIndex((entry, position) => position > input.indexOf(clicks[index]) && entry.call === "move");
        if (nextMoveIndex >= 0) {
            assert(afterSave < nextMoveIndex, `AUD1 ${category} visible completion precedes the next slider pointer move`);
            const moveSequences = input[nextMoveIndex].event_seq || [];
            assert(moveSequences.every((seq) => !frame.applied_seq.includes(seq)),
                `AUD1 ${category} release frame does not include the following slider's pointer move`);
        }
    }
}

function checkpointForSave(input, checkpoints, filename) {
    const saveIndex = input.findIndex((entry) => entry.call === "save" && entry.args?.[0] === filename);
    assert(saveIndex > 0, `${filename} must come from a captured screenshot`);
    let screenshotIndex = saveIndex - 1;
    while (screenshotIndex >= 0 && input[screenshotIndex].call !== "screenshot") screenshotIndex -= 1;
    assert(screenshotIndex >= 0, `${filename} has no screenshot input`);
    const checkpoint = checkpoints.find((entry) => entry.input_call === screenshotIndex && entry.window === 1);
    assert(checkpoint?.debug && checkpoint?.scene, `${filename} has no actual scene/debug checkpoint`);
    return checkpoint;
}

function pointerEvent(entry, type) {
    const event = entry.actual_events?.find((candidate) => candidate.type === type && candidate.canvas_target);
    assert(Number.isInteger(event?.seq), `feedback ${type} must be an actual canvas input with a sequence`);
    return event;
}

function frameForSequence(debug, sequence) {
    const frames = debug.frames?.retained_input;
    assert(Array.isArray(frames), "feedback debug bridge has no retained input frames");
    const matches = frames.filter((frame) => frame.applied_seq?.includes(sequence));
    assert.equal(matches.length, 1, `input ${sequence} must map to exactly one first presented frame`);
    return matches[0];
}

function parseTickLog(source) {
    return source.trim().split(/\r?\n/).filter(Boolean).map((line) => {
        const [tick, wallNs, unit, x, y] = line.split(",").map(Number);
        assert([tick, wallNs, unit, x, y].every(Number.isFinite), "server tick log contains a malformed row");
        return { tick, unit, x, y };
    });
}

function firstMovementTick(rows, unit) {
    const positions = rows.filter((row) => row.unit === unit);
    if (positions.length < 2) return null;
    const origin = positions[0];
    return positions.find((row) => Math.hypot(row.x - origin.x, row.y - origin.y) > 0.001)?.tick ?? null;
}

function movedAfter(rows, unit) {
    return Number.isInteger(firstMovementTick(rows, unit));
}

function noticeText(notice) {
    if (typeof notice === "string") return notice;
    return String(notice?.text ?? notice?.message ?? JSON.stringify(notice));
}

function assertClose(actual, expected, message) {
    assert(Number.isFinite(actual) && Math.abs(actual - expected) < 0.015,
        `${message}: expected ${expected}, got ${actual}`);
}

module.exports = { judge, assertAlr1MinimapPixels };
