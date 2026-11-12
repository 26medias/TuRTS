// turts-test-role: oracle
"use strict";

const assert = require("node:assert/strict");
const { decodePng, pixelAt } = require("./harness/png.js");
const { unprojectGround } = require("./hud4-oracle.js");

async function judge(view, params) {
    const input = view.inputLog();
    const steps = JSON.parse(await view.serverFile("mm1-driver-steps.json"));
    const expectedRect = {
        x: params.viewport.width - params.minimap_inset - params.minimap_size,
        y: params.viewport.height - params.minimap_inset - params.minimap_size,
        size: params.minimap_size,
    };
    const pixelParams = { ...params, _rect: expectedRect };
    assert.deepEqual(steps.rect, expectedRect, "MM1 screenshot target must use the declared authored minimap geometry");
    assert.deepEqual(params.map_size, [64, 48], "MM1 must use the actual authored 64 by 48 map dimensions");

    const checkpoints = JSON.parse(await view.serverFile("scene-checkpoints.json"));
    const shot = async (name, window) => {
        const saveIndex = input.findIndex((entry) => entry.call === "save" && entry.args?.[0] === name);
        assert(saveIndex >= 0, `MM1 is missing actual screenshot ${name}`);
        let screenshotIndex = saveIndex - 1;
        while (screenshotIndex >= 0 && input[screenshotIndex].call !== "screenshot") screenshotIndex -= 1;
        assert(screenshotIndex >= 0, `MM1 is missing screenshot input for ${name}`);
        const checkpoint = checkpoints.find((entry) => entry.input_call === screenshotIndex && entry.window === window);
        assert(checkpoint?.scene && checkpoint?.debug, `MM1 screenshot ${name} lacks its actual C84 checkpoint`);
        return { checkpoint, image: decodePng(Buffer.from(await view.serverFile(`${name}.b64`), "base64")) };
    };

    const noPing = await shot("mm1-window2-no-ping.png", 2);
    const before = await shot("mm1-window1-before.png", 1);
    const selected = await shot("mm1-window1-selected.png", 1);
    const beforeFocus = await shot("mm1-before-focus.png", 1);
    const afterFocus = await shot("mm1-after-focus.png", 1);
    const afterAttack = await shot("mm1-window1-attack.png", 1);
    const beforeLeft = await shot("mm1-before-left.png", 1);
    const afterLeft = await shot("mm1-after-left.png", 1);
    const afterDrag = await shot("mm1-after-drag.png", 1);
    const afterRight = await shot("mm1-after-right.png", 1);
    const afterZoom = await shot("mm1-after-zoom.png", 1);
    const pingWindow = await shot("mm1-window2-ping.png", 2);

    for (const { checkpoint } of [before, afterFocus, afterLeft, afterDrag, afterRight, afterZoom, noPing, pingWindow]) {
        assertMinimapRect(checkpoint.scene, expectedRect);
    }

    const ownId = unitAtPixel(before.checkpoint, steps.pixel_targets.own, 0);
    const publicUnits = new Map((before.checkpoint.debug.replicated_world?.units || []).map((unit) => [unit.unit_id, unit]));
    assert.equal(publicUnits.get(ownId)?.content_id, "muster:trooper", "selected screenshot pixel must identify a real trooper");
    assert((selected.checkpoint.debug.selection || []).includes(ownId), "the screenshot-selected trooper must remain the actual selected identity");

    const focusTarget = minimapWorldPoint(steps.local.focus, params.map_size);
    const focusInput = exactInput(input, "click", [steps.screen.focus.x, steps.screen.focus.y, "left"]);
    assertActualPointerGesture(focusInput, "focus left click");
    const beforeFocusScreen = projectMapPoint(beforeFocus.checkpoint.scene, focusTarget);
    const afterFocusScreen = projectMapPoint(afterFocus.checkpoint.scene, focusTarget);
    const focusCenter = viewportCenter(afterFocus.checkpoint.scene);
    assert(distance2(beforeFocusScreen, focusCenter) > 8, "MM1 focus target must differ from the initial camera centre");
    assertClose2(afterFocusScreen, focusCenter, 1.5, "camera did not move after the minimap click");

    assert(!steps.attack_skipped && steps.pixel_targets.foreign,
        "MM1 must expose a real slot 1 target after the minimap camera move");
    const targetId = unitAtPixel(afterFocus.checkpoint, steps.pixel_targets.foreign, 1);
    const targetUnit = (afterFocus.checkpoint.debug.replicated_world?.units || []).find((unit) => unit.unit_id === targetId);
    assert.equal(targetUnit?.content_id, "muster:dummy", "attack screenshot pixel must identify a real slot 1 dummy");

    const alertBefore = noPing.checkpoint.debug.alert_pings || [];
    assert(!alertBefore.some((ping) => ping.unit === targetId), "the no-ping control must precede the actual damage event");
    assert.equal(targetUnit?.owner_slot, 1, "the damaged target must belong to slot 1");
    const attackInput = exactInput(input, "click", [steps.pixel_targets.foreign.x, steps.pixel_targets.foreign.y, "right"]);
    assertActualPointerGesture(attackInput, "attack the slot 1 dummy");
    const acceptedAttack = assertOrderFromInput(afterAttack.checkpoint.debug, attackInput, "muster:order_attack", ownId, { unit: targetId },
        "window 1 must issue an actual attack order from the selected trooper to the slot 1 dummy");
    const startPosition = unitPosition(before.checkpoint.debug, ownId);
    const attackPosition = unitPosition(afterAttack.checkpoint.debug, ownId);
    const targetPosition = unitPosition(afterAttack.checkpoint.debug, targetId);
    const initialGap = distance2(startPosition, targetPosition);
    const remainingGap = distance2(attackPosition, targetPosition);
    assert(distance2(startPosition, attackPosition) > 2 && remainingGap < initialGap - 2,
        "the screenshot-selected exposed trooper must physically approach its accepted Unit Attack target");

    const leftTarget = minimapWorldPoint(steps.local.left, params.map_size);
    const leftInput = exactInput(input, "click", [steps.screen.left.x, steps.screen.left.y, "left"]);
    assertActualPointerGesture(leftInput, "left click");
    const beforeLeftScreen = projectMapPoint(beforeLeft.checkpoint.scene, leftTarget);
    const afterLeftScreen = projectMapPoint(afterLeft.checkpoint.scene, leftTarget);
    const center = viewportCenter(afterLeft.checkpoint.scene);
    assert(distance2(beforeLeftScreen, center) > 8, "MM1 left-click target must differ from the current camera centre");
    assertClose2(afterLeftScreen, center, 1.5, "camera did not move after the minimap click");

    const dragTarget = minimapWorldPoint(steps.local.drag_end, params.map_size);
    const dragInput = exactInput(input, "drag", [steps.screen.drag_start.x, steps.screen.drag_start.y,
        steps.screen.drag_end.x, steps.screen.drag_end.y, "left"]);
    assertActualPointerGesture(dragInput, "left drag");
    assertClose2(projectMapPoint(afterDrag.checkpoint.scene, dragTarget), viewportCenter(afterDrag.checkpoint.scene), 1.5,
        "camera did not follow the minimap drag endpoint");

    const rightTarget = minimapWorldPoint(steps.local.right, params.map_size);
    const rightInput = exactInput(input, "click", [steps.screen.right.x, steps.screen.right.y, "right"]);
    assertActualPointerGesture(rightInput, "right-click smart command");
    assertClose2(projectMapPoint(afterRight.checkpoint.scene, dragTarget), viewportCenter(afterRight.checkpoint.scene), 1.5,
        "right-click smart command must not move the camera");
    assertOrderFromInput(afterRight.checkpoint.debug, rightInput, "muster:order_move", ownId, { point: rightTarget },
        "minimap right click must issue the selected trooper's smart move at the minimap world point");

    assertClose2(projectMapPoint(afterZoom.checkpoint.scene, dragTarget), viewportCenter(afterZoom.checkpoint.scene), 1.5,
        "camera zoom must preserve the minimap drag target");
    assertZoomGesture(input);
    assertCameraZoom(afterRight.checkpoint.scene, afterZoom.checkpoint.scene);
    assertSlotDots(afterZoom.image, afterZoom.checkpoint, pixelParams);
    assertNodeDots(afterZoom.image, afterZoom.checkpoint, pixelParams);
    assertCameraFrame(afterZoom.image, afterZoom.checkpoint.scene, pixelParams);
    assertMinimapExteriorClean(afterZoom.image, pixelParams);
    assertMinimapExteriorClean(noPing.image, pixelParams);
    assertMinimapExteriorClean(pingWindow.image, pixelParams);

    const alert = (pingWindow.checkpoint.debug.alert_pings || []).find((ping) =>
        ping.unit === targetId && ping.alert_id === "muster_unit_damaged");
    assert(alert && Number.isSafeInteger(alert.tick), "slot 1 must observe the real OwnUnitDamaged AlertPings event");
    const target = publicUnits.get(targetId)?.pos;
    assert(Array.isArray(target) && Math.hypot(alert.x - target[0], alert.y - target[1]) < 1,
        "the alert ping world point must match the damaged dummy's replicated position");
    const pingLocal = [alert.x / params.map_size[0], 1 - alert.y / params.map_size[1]];
    const pingPixel = minimapPixel(pingLocal, expectedRect);
    // The ping pixels come before the health record: a ping that is not drawn must give the pixel
    // text, not the text of a dummy that died while the shot waited (RUN130, plant mm1-no-ping).
    const pingColor = parseHex(params.ping_colour);
    const noPingPixels = colorPixelsNear(noPing.image, pingPixel, 7, pingColor, 28);
    const pingPixels = colorPixelsNear(pingWindow.image, pingPixel, 7, pingColor, 28);
    assert(noPingPixels.length < 2, "no-ping screenshot control must have no configured ping-colour pixels at the target");
    assert(pingPixels.length >= 4, "actual OwnUnitDamaged event must add configured ping-shape pixels at its minimap world point");
    const pingBounds = boundsOf(pingPixels);
    assert(pingBounds.width >= 2 && pingBounds.height >= 2 && pingBounds.width <= 15 && pingBounds.height <= 15,
        "actual ping pixels must form a compact visible minimap ping shape at the alert world point");
    assert(!nearColor(parseHex(params.slot_colours[1]), pingColor, 28),
        "ping pixel threshold must distinguish slot 1 orange from the configured ping colour");
    const damaged = pingWindow.checkpoint.debug.replicated_world?.healths?.find((health) => unitId(health.unit) === targetId);
    assert(damaged && damaged.current < (publicUnits.get(targetId)?.max_health ?? 40),
        "the slot 1 ping must belong to the actually damaged dummy health record");
    const causalFrames = pingWindow.checkpoint.debug.frames?.retained_input || [];
    const pingFrame = causalFrames.find((frame) => frame.replica_tick === alert.tick && frame.feedback?.alerts?.some((event) =>
        event.id === alert.alert_id && event.unit === targetId && event.position?.x === alert.x && event.position?.y === alert.y));
    assert(pingFrame, "slot 1 ping must match the same retained feedback rule, unit, tick, and position");
    const healthHistory = pingWindow.checkpoint.debug.replicated_world?.health_trace || [];
    const observerTick = pingFrame.replica_tick;
    const baseline = noPing.checkpoint.debug.replicated_world?.healths?.find((record) => unitId(record.unit) === targetId)?.current;
    assert(Number.isFinite(baseline), "the no-ping checkpoint must retain the target unit's pre-stimulus health");
    const history = healthHistory.find((record) => unitId(record.unit_id) === targetId)?.entries || [];
    const baselineTick = noPing.checkpoint.debug.replicated_world?.tick;
    assert(Number.isSafeInteger(baselineTick), "the no-ping checkpoint must expose its actual replica tick");
    const eligible = history.filter((sample) => Number.isSafeInteger(sample.tick) &&
        sample.tick > baselineTick && sample.tick >= acceptedAttack.tick && sample.tick <= observerTick)
        .sort((left, right) => left.tick - right.tick);
    assert(eligible.length > 0 && eligible.at(-1).current < baseline,
        "the exact ping observer must follow an accepted attack stimulus and a same-unit health drop from the no-ping baseline");
    const firstDrop = eligible.find((sample) => sample.current < baseline);
    const priorSample = firstDrop && history.filter((sample) => sample.tick < firstDrop.tick)
        .sort((left, right) => left.tick - right.tick).at(-1);
    assert(firstDrop && priorSample && firstDrop.current < priorSample.current && priorSample.current >= baseline,
        "the qualifying post-stimulus health change must be a real adjacent decrease from the pre-stimulus health level");
    assert(damaged.current < baseline, "the observer's actual health record must remain below its qualified pre-stimulus baseline");
}

function assertMinimapRect(scene, expected) {
    const widget = (scene.widgets || []).find((entry) => entry.id === "minimap");
    assert(widget && Array.isArray(widget.rect), "actual C84 snapshot must expose the authored minimap rectangle");
    const [x, y, width, height] = widget.rect;
    assert.deepEqual([x, y, width, height], [expected.x, expected.y, expected.size, expected.size],
        "MM1 target and pixel oracle must use the actual C84 minimap rectangle");
}

function unitAtPixel(checkpoint, pixel, owner) {
    const worldPoint = unprojectGround(checkpoint.scene, pixel);
    const matches = (checkpoint.scene.units || []).filter((unit) =>
        Number.isFinite(unit.pick_radius) && Array.isArray(unit.translation) && unit.translation.length === 3)
        .map((unit) => ({ unit, distance: Math.hypot(unit.translation[0] - worldPoint[0], unit.translation[2] - worldPoint[1]) }))
        .filter(({ unit, distance }) => distance <= unit.pick_radius + 0.15 && ownerOf(checkpoint.debug, unit.unit_id) === owner)
        .sort((left, right) => left.distance - right.distance);
    assert(matches.length > 0, "screenshot-derived pixel ground ray must bind to an actual C84 unit footprint and identity");
    return matches[0].unit.unit_id;
}

function ownerOf(debug, unitId) {
    return (debug.replicated_world?.units || []).find((unit) => unit.unit_id === unitId)?.owner_slot;
}

function unitPosition(debug, id) {
    const position = debug.replicated_world?.units?.find((unit) => unit.unit_id === id)?.pos;
    assert(Array.isArray(position) && position.length === 2 && position.every(Number.isFinite),
        `MM1 actual replicated position for unit ${id} is missing`);
    return position;
}

function unitId(value) {
    if (Number.isSafeInteger(value)) return value;
    if (Number.isSafeInteger(value?.unit_id)) return value.unit_id;
    if (Number.isSafeInteger(value?.UnitId)) return value.UnitId;
    if (Number.isSafeInteger(value?.[0])) return value[0];
    return undefined;
}

function exactInput(input, call, args) {
    const entry = input.find((row) => row.call === call && JSON.stringify(row.args) === JSON.stringify(args));
    assert(entry, `MM1 must use exact harness ${call} arguments ${JSON.stringify(args)}`);
    return entry;
}

function assertActualPointerGesture(entry, label) {
    assert(entry.event_types?.includes("pointerdown") && entry.event_types?.includes("pointerup"),
        `MM1 ${label} must include actual browser pointer down/up events`);
    assert(entry.event_seq?.length >= 2 && entry.actual_events?.length >= 2,
        `MM1 ${label} must have actual sequenced browser input receipts`);
}

function projectMapPoint(scene, point) {
    const matrix = scene?.clip_from_world;
    const viewport = scene?.viewport_logical_rect;
    assert(Array.isArray(matrix) && matrix.length === 16 && matrix.every(Number.isFinite),
        "MM1 needs the actual same-window clip-from-world camera matrix");
    assert(Array.isArray(viewport) && viewport.length === 4 && viewport.every(Number.isFinite),
        "MM1 needs the actual same-window camera viewport");
    const clip = transform4(matrix, [point[0], 0, point[1], 1]);
    assert(Number.isFinite(clip[3]) && Math.abs(clip[3]) > 1e-9, "MM1 camera projection has an invalid homogeneous point");
    const nx = clip[0] / clip[3];
    const ny = clip[1] / clip[3];
    const [left, top, width, height] = viewport;
    return [left + (nx + 1) * width / 2, top + (1 - ny) * height / 2];
}

function transform4(matrix, vector) {
    return Array.from({ length: 4 }, (_, row) =>
        matrix[row] * vector[0] + matrix[4 + row] * vector[1] + matrix[8 + row] * vector[2] + matrix[12 + row] * vector[3]);
}

function minimapWorldPoint([u, v], [width, height]) {
    return [u * width, (1 - v) * height];
}

function minimapPixel([u, v], rect) {
    return [rect.x + u * rect.size, rect.y + v * rect.size];
}

function viewportCenter(scene) {
    const viewport = scene?.viewport_logical_rect;
    assert(Array.isArray(viewport) && viewport.length === 4 && viewport.every(Number.isFinite),
        "MM1 needs the actual viewport to check camera centring");
    return [viewport[0] + viewport[2] / 2, viewport[1] + viewport[3] / 2];
}

function assertClose2(actual, expected, tolerance, message) {
    assert(Array.isArray(actual) && actual.every(Number.isFinite), `${message}: actual camera point is missing`);
    assert(Math.hypot(actual[0] - expected[0], actual[1] - expected[1]) <= tolerance,
        `${message}: actual=${actual.map((value) => value.toFixed(2))}, expected=${expected.map((value) => value.toFixed(2))}`);
}

function assertZoomGesture(input) {
    const entry = input.find((row) => row.call === "wheel" && JSON.stringify(row.args) === JSON.stringify([-20]));
    assert(entry?.event_types?.includes("wheel") && entry.event_seq?.length === 1 && entry.actual_events?.length === 1,
        "MM1 camera frame zoom must follow an actual physical wheel event");
}

function assertCameraZoom(before, after) {
    const beforeMatrix = before?.clip_from_world;
    const afterMatrix = after?.clip_from_world;
    assert(Array.isArray(beforeMatrix) && beforeMatrix.length === 16 && Array.isArray(afterMatrix) && afterMatrix.length === 16,
        "MM1 zoom needs actual before/after camera matrices");
    const maximumDelta = Math.max(...beforeMatrix.map((value, index) => Math.abs(value - afterMatrix[index])));
    assert(maximumDelta > 1e-5, "MM1 physical wheel input must change the actual camera matrix before the frame-pixel check");
}

function distance2(a, b) { return Math.hypot(a[0] - b[0], a[1] - b[1]); }

function assertOrderFromInput(debug, input, orderId, unit, target, message) {
    const sequence = input.event_seq?.at(-1);
    assert(Number.isSafeInteger(sequence), `${message}: missing physical pointer release sequence`);
    const frames = debug.frames?.retained_input || [];
    const frame = frames.find((entry) => entry.applied_seq?.includes(sequence));
    assert(frame, `${message}: no retained core frame applied the pointer release`);
    assert.equal(frame.applied_seq.filter((value) => value === sequence).length, 1,
        `${message}: pointer release must apply exactly once`);
    const emitted = (frame.emitted_orders || []).find((entry) => entry.order === orderId && entry.units?.includes(unit));
    assert(emitted, message);
    assert(Number.isSafeInteger(emitted.seq), `${message}: emitted order has no retained sequence`);
    const result = frames.flatMap((entry) => entry.results || []).find((entry) =>
        entry.seq === emitted.seq && entry.unit === unit);
    assert.equal(result?.status, "accepted",
        `${message}: the server must accept the exact emitted sequence for this unit`);
    const active = (debug.replicated_world?.unit_orders || []).find((entry) => entry.unit === unit);
    assert.equal(active?.current?.order, orderId,
        `${message}: the actual replicated unit order must still name the accepted order`);
    if (Object.hasOwn(target, "unit")) {
        const actual = emitted.target?.Unit ?? emitted.target?.unit ?? emitted.target?.Unit?.unit_id;
        assert.equal(unitId(actual), target.unit, message);
        const activeTarget = active?.current?.target?.Unit ?? active?.current?.target?.unit;
        assert.equal(unitId(activeTarget), target.unit,
            `${message}: replicated current target must match the accepted target`);
    } else {
        const point = emitted.target?.Point || emitted.target?.point;
        assert(point && Math.hypot(point.x - target.point[0], point.y - target.point[1]) < 0.1, message);
        const activePoint = active?.current?.target?.Point || active?.current?.target?.point;
        assert(activePoint && Math.hypot(activePoint.x - target.point[0], activePoint.y - target.point[1]) < 0.1,
            `${message}: replicated current ground target must match the accepted point`);
    }
    return result;
}

function assertSlotDots(image, checkpoint, params) {
    const owners = new Map((checkpoint.debug.replicated_world?.units || []).map((unit) => [unit.unit_id, unit.owner_slot]));
    const units = checkpoint.scene.units || [];
    for (const slot of [0, 1]) {
        const candidates = units.filter((unit) => owners.get(unit.unit_id) === slot && Array.isArray(unit.translation));
        assert(candidates.length > 0, `MM1 actual scene must contain slot ${slot} units`);
        const colour = parseHex(params.slot_colours[slot]);
        const found = candidates.some((unit) => {
            const [u, v] = [unit.translation[0] / params.map_size[0], 1 - unit.translation[2] / params.map_size[1]];
            return countColorNear(image, minimapPixel([u, v], params._rect), 3, colour, 38) >= 2;
        });
        assert(found, `MM1 must draw actual slot ${slot} unit pixels in its authored slot colour`);
    }
}

function assertNodeDots(image, checkpoint, params) {
    const nodes = checkpoint.scene.nodes || [];
    assert(nodes.length > 0, "MM1 actual scene must contain the authored ore node");
    const background = parseHex(params.background_colour);
    const visible = nodes.some((node) => {
        const [u, v] = [node.translation[0] / params.map_size[0], 1 - node.translation[2] / params.map_size[1]];
        const [cx, cy] = minimapPixel([u, v], params._rect);
        for (let y = Math.max(0, Math.floor(cy - 3)); y <= Math.ceil(cy + 3); y += 1) {
            for (let x = Math.max(0, Math.floor(cx - 3)); x <= Math.ceil(cx + 3); x += 1) {
                const pixel = pixelAt(image, x, y);
                if (pixel && pixel[3] >= 245 && !nearColor(pixel, background, 18)) return true;
            }
        }
        return false;
    });
    assert(visible, "MM1 must draw an actual resource-node marker at its minimap world position");
}

function assertCameraFrame(image, scene, params) {
    const viewport = scene?.viewport_logical_rect;
    assert(Array.isArray(viewport) && viewport.length === 4 && viewport.every(Number.isFinite),
        "MM1 camera frame needs the actual logical viewport");
    const [left, top, width, height] = viewport;
    const corners = [
        [left, top], [left + width - 1, top],
        [left + width - 1, top + height - 1], [left, top + height - 1],
    ].map((pixel) => unprojectGround(scene, { x: pixel[0], y: pixel[1] })
        .map((value, index) => index === 0 ? value / params.map_size[0] : 1 - value / params.map_size[1]));
    const rect = params._rect;
    const colour = parseHex(params.frame_colour);
    const framePixels = new Set();
    for (let index = 0; index < corners.length; index += 1) {
        const segment = clipUnitSegment(corners[index], corners[(index + 1) % corners.length]);
        if (!segment) continue;
        const a = minimapPixel(segment[0], rect);
        const b = minimapPixel(segment[1], rect);
        countColorNearSegment(image, a, b, colour, 2.5, 30, {
            left: rect.x + 4,
            top: rect.y + 4,
            right: rect.x + rect.size - 4,
            bottom: rect.y + rect.size - 4,
        }, framePixels);
    }
    assert(framePixels.size >= 8, "MM1 must draw distinct actual camera-footprint pixels inside the minimap in configured frame colour");
}

function assertMinimapExteriorClean(image, params) {
    const rect = params._rect;
    const colours = [parseHex(params.frame_colour), parseHex(params.ping_colour)];
    let leaked = 0;
    for (let y = rect.y - 2; y <= rect.y + rect.size + 1; y += 1) {
        for (let x = rect.x - 2; x <= rect.x + rect.size + 1; x += 1) {
            if (x >= rect.x && x < rect.x + rect.size && y >= rect.y && y < rect.y + rect.size) continue;
            const pixel = pixelAt(image, x, y);
            if (pixel && pixel[3] >= 245 && colours.some((colour, index) =>
                nearColor(pixel, colour, index === 0 ? 30 : 28))) leaked += 1;
        }
    }
    assert.equal(leaked, 0, "MM1 frame and alert-ping glyph pixels must stay clipped inside the authored minimap rectangle");
}

function clipUnitSegment(start, end) {
    let t0 = 0;
    let t1 = 1;
    const dx = end[0] - start[0];
    const dy = end[1] - start[1];
    for (const [p, q] of [[-dx, start[0]], [dx, 1 - start[0]], [-dy, start[1]], [dy, 1 - start[1]]]) {
        if (Math.abs(p) < 1e-12) { if (q < 0) return null; continue; }
        const t = q / p;
        if (p < 0) t0 = Math.max(t0, t); else t1 = Math.min(t1, t);
        if (t0 > t1) return null;
    }
    return [[start[0] + t0 * dx, start[1] + t0 * dy], [start[0] + t1 * dx, start[1] + t1 * dy]];
}

function countColorNear(image, [cx, cy], radius, color, tolerance) {
    let count = 0;
    for (let y = Math.max(0, Math.floor(cy - radius)); y <= Math.min(image.height - 1, Math.ceil(cy + radius)); y += 1) {
        for (let x = Math.max(0, Math.floor(cx - radius)); x <= Math.min(image.width - 1, Math.ceil(cx + radius)); x += 1) {
            const pixel = pixelAt(image, x, y);
            if (pixel && pixel[3] >= 245 && nearColor(pixel, color, tolerance)) count += 1;
        }
    }
    return count;
}

function colorPixelsNear(image, [cx, cy], radius, color, tolerance) {
    const pixels = [];
    for (let y = Math.max(0, Math.floor(cy - radius)); y <= Math.min(image.height - 1, Math.ceil(cy + radius)); y += 1) {
        for (let x = Math.max(0, Math.floor(cx - radius)); x <= Math.min(image.width - 1, Math.ceil(cx + radius)); x += 1) {
            const pixel = pixelAt(image, x, y);
            if (pixel && pixel[3] >= 245 && nearColor(pixel, color, tolerance)) pixels.push([x, y]);
        }
    }
    return pixels;
}

function boundsOf(points) {
    assert(points.length > 0, "pixel bounds need at least one sample");
    const xs = points.map((point) => point[0]);
    const ys = points.map((point) => point[1]);
    return {
        width: Math.max(...xs) - Math.min(...xs) + 1,
        height: Math.max(...ys) - Math.min(...ys) + 1,
    };
}

function countColorNearSegment(image, a, b, color, radius, tolerance, clip, pixels) {
    const steps = Math.max(1, Math.ceil(Math.hypot(b[0] - a[0], b[1] - a[1])));
    for (let index = 0; index <= steps; index += 1) {
        const t = index / steps;
        const cx = a[0] + (b[0] - a[0]) * t;
        const cy = a[1] + (b[1] - a[1]) * t;
        for (let y = Math.max(clip.top, Math.floor(cy - radius)); y <= Math.min(clip.bottom, Math.ceil(cy + radius)); y += 1) {
            for (let x = Math.max(clip.left, Math.floor(cx - radius)); x <= Math.min(clip.right, Math.ceil(cx + radius)); x += 1) {
                const pixel = pixelAt(image, x, y);
                if (pixel && pixel[3] >= 245 && nearColor(pixel, color, tolerance)) pixels.add(`${x},${y}`);
            }
        }
    }
}

function parseHex(hex) {
    assert(/^#[0-9a-fA-F]{6}$/.test(hex), `invalid configured RGB colour ${hex}`);
    return [1, 3, 5].map((index) => Number.parseInt(hex.slice(index, index + 2), 16));
}

function nearColor(pixel, color, tolerance) {
    return Math.hypot(pixel[0] - color[0], pixel[1] - color[1], pixel[2] - color[2]) <= tolerance;
}

module.exports = { judge };
