// turts-test-role: oracle
"use strict";

const assert = require("node:assert/strict");
const { decodePng, pixelAt } = require("./harness/png.js");

const SLOT_RGB = [[0, 114, 178], [230, 159, 0], [0, 158, 115], [213, 94, 0], [86, 180, 233], [204, 121, 167], [240, 228, 66], [0, 0, 0]];
const RANGE_RGB = [242, 204, 96];
const HEALTH_RGB = [86, 212, 168];
const TOOLTIP_BG_RGB = [17, 24, 32];
const WAYPOINT_RGB = [230, 230, 217];

async function judge(view) {
    const input = view.inputLog();
    const checkpoints = JSON.parse(await view.serverFile("scene-checkpoints.json"));
    const driver = JSON.parse(await view.serverFile("vis1-full-steps.json"));
    const targetEvidence = JSON.parse(await view.serverFile("vis1-full-pixel-targets.json"));
    assert(Array.isArray(checkpoints), "VIS1 requires actual C84 screenshot checkpoints");
    assert(Array.isArray(driver.shots) && Array.isArray(driver.actions), "VIS1 driver evidence is incomplete");
    assert.equal(targetEvidence.method, "authored silhouette/color component recognition and green floor pixels only",
        "VIS1 input targets must come from PNG pixels, not scene/debug reads");
    const target = targetEvidence.targets;

    const checkpointsByLabel = new Map();
    for (const shot of driver.shots) {
        const saveIndex = input.findIndex((entry) => entry.call === "save" && entry.args?.[0] === `${shot.label}.png`);
        if (saveIndex < 0) continue;
        const screenshotIndex = previousCall(input, saveIndex, "screenshot");
        if (screenshotIndex < 0) continue;
        const checkpoint = checkpoints.find((entry) => entry.input_call === screenshotIndex && entry.window === 1);
        assert(checkpoint?.scene && checkpoint?.debug, `VIS1 ${shot.label} lacks its exact actual C84 screenshot checkpoint`);
        checkpointsByLabel.set(shot.label, { checkpoint, screenshotIndex });
    }

    // Keep the original named one-colour failure before checking the added features.
    const selectedOwn = requireStage(checkpointsByLabel, "vis1-own-selected");
    const selectedForeign = requireStage(checkpointsByLabel, "vis1-foreign-selected");
    const ownUnit = resolveUnit(selectedOwn.checkpoint, target.attacker, 0, "muster:trooper");
    const foreignUnit = resolveUnit(selectedForeign.checkpoint, target.foreign, 1, "muster:dummy");
    const ownImage = await screenshot(view, "vis1-own-selected");
    const foreignImage = await screenshot(view, "vis1-foreign-selected");
    const initial = requireStage(checkpointsByLabel, "vis1-full-initial");
    const initialImage = await screenshot(view, "vis1-full-initial");
    const initialForeign = resolveUnit(initial.checkpoint, target.foreign, 1, "muster:dummy");
    assert.equal(initialForeign.unit_id, foreignUnit.unit_id,
        "VIS1 foreign ring reference must be the same public unit as the selected capture");
    assert.deepEqual(initialForeign.translation, foreignUnit.translation,
        "VIS1 foreign ring reference must keep the same unit at the same world position");
    assert(!(initial.checkpoint.debug.selection || []).includes(foreignUnit.unit_id),
        "VIS1 initial foreign ring reference must have the target unselected");
    assert(!(initial.checkpoint.debug.rings_shown || []).some((ring) => ring.unit === foreignUnit.unit_id),
        "VIS1 initial foreign ring reference must not already contain the target ring");
    assert.deepEqual(initial.checkpoint.scene.clip_from_world, selectedForeign.checkpoint.scene.clip_from_world,
        "VIS1 selected and initial ring captures must use the same camera projection");
    assertSlotColor(ownImage, ownUnit, 0);
    assertSlotColor(foreignImage, foreignUnit, 1);

    assert((selectedOwn.checkpoint.debug.selection || []).includes(ownUnit.unit_id),
        "VIS1 own trooper must be selected in the actual C84 snapshot");
    assert(visibleRingPixels(ownImage, selectedOwn.checkpoint.scene, ownUnit, ownUnit.pick_radius),
        "VIS1 own selection must draw a solid slot-coloured ring in actual screenshot pixels");
    assert((selectedForeign.checkpoint.debug.selection || []).includes(foreignUnit.unit_id),
        "VIS1 foreign dummy must be selected in the actual C84 snapshot");
    assert(visibleForeignGaps(foreignImage, selectedForeign.checkpoint.scene, foreignUnit, initialImage),
        "VIS1 foreign selection must draw slot-coloured ring segments with four visible gaps");
    assert((selectedOwn.checkpoint.debug.rings_shown || []).some((ring) => ring.unit === ownUnit.unit_id && ring.own === true),
        "VIS1 C84 must record the selected own ring as actually visible");
    assert((selectedForeign.checkpoint.debug.rings_shown || []).some((ring) => ring.unit === foreignUnit.unit_id && ring.own === false),
        "VIS1 C84 must record the selected foreign ring as actually visible");

    assertHealthBar(ownImage, ownUnit, "selected own trooper");
    assertHealthBar(foreignImage, foreignUnit, "selected foreign dummy");
    assertRangePixels(ownImage, selectedOwn.checkpoint.scene, ownUnit, 4.0);

    const selectionCleared = requireStage(checkpointsByLabel, "vis1-selection-cleared");
    assert.deepEqual(selectionCleared.checkpoint.debug.selection || [], [],
        "VIS1 cleared-selection capture must have no selected public units");
    const clearedImage = await screenshot(view, "vis1-selection-cleared");
    const clearedOwn = resolveUnit(selectionCleared.checkpoint, target.attacker, 0, "muster:trooper");
    const clearedForeign = resolveUnit(selectionCleared.checkpoint, target.foreign, 1, "muster:dummy");
    assert.equal(clearedOwn.max_health, ownUnit.max_health,
        "VIS1 own unit max health must come from the same accepted catalog across captures");
    assert.equal(clearedForeign.max_health, foreignUnit.max_health,
        "VIS1 foreign unit max health must come from the same accepted catalog across captures");
    assertNoHealthBar(clearedImage, clearedOwn, "unselected full-health own trooper");
    assertNoHealthBar(clearedImage, clearedForeign, "unselected full-health foreign dummy");

    const hoverImmediate = requireStage(checkpointsByLabel, "vis1-hover-immediate");
    const hoverTooltip = requireStage(checkpointsByLabel, "vis1-hover-tooltip");
    const hoverAction = precedingInput(input, hoverImmediate.screenshotIndex, (entry) =>
        entry.call === "move" && nearPoint(entry.args, target.foreign));
    assert(hoverAction, "VIS1 hover target must be a real physical pointer move to the PNG-derived foreign unit");
    const tooltipWait = input.find((entry) => entry.call === "wait" && entry.args?.[0] === 500 && entry.wall_time_ms >= hoverAction.wall_time_ms &&
        entry.wall_time_ms <= hoverTooltip.checkpoint.wall_time_ms);
    assert(tooltipWait, "VIS1 tooltip must follow an actual 500 ms physical-driver wait");
    const hoverImage = await screenshot(view, "vis1-hover-immediate");
    const tooltipImage = await screenshot(view, "vis1-hover-tooltip");
    assert(visibleRingPixels(hoverImage, hoverImmediate.checkpoint.scene, foreignUnit, foreignUnit.pick_radius),
        "VIS1 hovered foreign unit must show an actual slot-coloured hover ring");
    assertTooltipPixels(tooltipImage, hoverTooltip.checkpoint.scene, foreignUnit);
    assert.equal(ownerOf(hoverTooltip.checkpoint.debug, foreignUnit.unit_id), 1,
        "VIS1 tooltip target must be the actual slot-1 foreign unit");

    const damage = requireStage(checkpointsByLabel, "vis1-foreign-damaged");
    const damageImage = await screenshot(view, "vis1-foreign-damaged");
    const damageShot = driver.shots.find((shot) => shot.label === "vis1-foreign-damaged");
    assert(damageShot?.point, "VIS1 damage capture must retain its current PNG-derived target point");
    const damagedUnit = resolveUnit(damage.checkpoint, damageShot.point, 1, "muster:dummy");
    const health = (damage.checkpoint.debug.replicated_world?.healths || []).find((entry) =>
        orderUnitId(entry.unit) === damagedUnit.unit_id || entry.unit_id === damagedUnit.unit_id);
    assert(health && health.current < 40, "VIS1 screenshot checkpoint must show the foreign dummy at actually reduced health");
    assert.equal(damagedUnit.unit_id, foreignUnit.unit_id,
        "VIS1 health fill comparison must track the same public foreign unit across captures");
    const maxHealth = damagedUnit.max_health;
    assert(Number.isFinite(maxHealth) && maxHealth > 0,
        "VIS1 must read max health from the accepted client catalog for the actual damaged unit");
    const fullFill = assertHealthBar(foreignImage, foreignUnit, "full-health selected foreign dummy");
    const damagedFill = assertHealthBar(damageImage, damagedUnit, "damaged foreign dummy");
    const expectedFillRatio = health.current / maxHealth;
    const actualFillRatio = damagedFill / fullFill;
    const pixelTolerance = Math.max(0.12, 2 / fullFill);
    assert(damagedFill < fullFill,
        `VIS1 damaged foreign health fill must be shorter than its same-unit full-health reference (${damagedFill}px vs ${fullFill}px)`);
    assert(healthFillRatioMatches(damagedFill, fullFill, health.current, maxHealth),
        `VIS1 damaged fill ratio ${actualFillRatio.toFixed(3)} must track actual health ratio ${expectedFillRatio.toFixed(3)} within ${pixelTolerance.toFixed(3)}`);
    assert((damage.checkpoint.debug.markers_shown || []).some((marker) => marker.kind === "attack"),
        "VIS1 accepted attack marker must remain visible through observed damage");

    const attackImmediate = requireStage(checkpointsByLabel, "vis1-attack-immediate");
    const attackClick = precedingInput(input, attackImmediate.screenshotIndex, (entry) =>
        entry.call === "click" && entry.args?.[2] === "right" && nearPoint(entry.args, target.foreign));
    const attackMarker = assertFirstFrameMarker(attackClick, attackImmediate, "attack", ownUnit.unit_id);
    const attackImage = await screenshot(view, "vis1-attack-immediate");
    assertAttackPixels(attackImage, attackImmediate.checkpoint.scene, attackMarker);
    assertAttackPixels(damageImage, damage.checkpoint.scene, attackMarker);
    const afterAttackStop = requireStage(checkpointsByLabel, "vis1-after-damage-stop");
    const afterAttackStopShot = driver.shots.find((shot) => shot.label === "vis1-after-damage-stop");
    assert.equal(afterAttackStopShot?.attack_marker_clear_seen, true,
        "VIS1 driver must observe attack marker clearance in actual screenshot pixels after Stop");
    assert(!(afterAttackStop.checkpoint.debug.markers_shown || []).some((marker) => marker.kind === "attack"),
        "VIS1 attack marker must clear after the stopped order reaches terminal replica state");

    const moveImmediate = requireStage(checkpointsByLabel, "vis1-move-immediate");
    const moveClick = precedingInput(input, moveImmediate.screenshotIndex, (entry) =>
        entry.call === "click" && entry.args?.[2] === "right" && nearPoint(entry.args, target.ground2));
    const moveMarker = assertFirstFrameMarker(moveClick, moveImmediate, "move", ownUnit.unit_id);
    const waypoint = requireStage(checkpointsByLabel, "vis1-waypoint-queued");
    const waypointClick = precedingInput(input, waypoint.screenshotIndex, (entry) =>
        entry.call === "click" && entry.args?.[2] === "right" && nearPoint(entry.args, target.ground));
    const waypointMarker = assertFirstFrameMarker(waypointClick, waypoint, "move", ownUnit.unit_id);
    const waypointImage = await screenshot(view, "vis1-waypoint-queued");
    assertMarkerPixels(waypointImage, waypoint.checkpoint.scene, waypointMarker);
    const waypointPath = requireStage(checkpointsByLabel, "vis1-waypoint-path");
    assertWaypointPixels(await screenshot(view, "vis1-waypoint-path"), waypointPath.checkpoint, waypointMarker);
    const moveImage = await screenshot(view, "vis1-move-immediate");
    assertMarkerPixels(moveImage, moveImmediate.checkpoint.scene, moveMarker);

    const active = requireStage(checkpointsByLabel, "vis1-move-active");
    assert((active.checkpoint.debug.markers_shown || []).some((marker) => marker.kind === "move"),
        "VIS1 active move marker must remain visible while the accepted order is in flight");
    assertMarkerPixels(await screenshot(view, "vis1-move-active"), active.checkpoint.scene, moveMarker);
    const stopped = requireStage(checkpointsByLabel, "vis1-move-stopped");
    const stoppedShot = driver.shots.find((shot) => shot.label === "vis1-move-stopped");
    assert.equal(stoppedShot?.marker_clear_seen, true, "VIS1 driver must observe marker clearance in actual PNG pixels");
    assert(!(stopped.checkpoint.debug.markers_shown || []).some((marker) => marker.kind === "move"),
        "VIS1 move marker must clear after the selected unit rests from the stopped order");

    const gatherBefore = requireStage(checkpointsByLabel, "vis1-gather-before");
    const gatherImmediate = requireStage(checkpointsByLabel, "vis1-gather-immediate");
    const gatherAfterPointerAway = requireStage(checkpointsByLabel, "vis1-gather-after-pointer-away");
    assert.deepEqual(gatherBefore.checkpoint.scene.clip_from_world, gatherImmediate.checkpoint.scene.clip_from_world,
        "VIS1 Gather dot comparison must keep the actual camera pose fixed");
    assert.deepEqual(gatherBefore.checkpoint.scene.clip_from_world, gatherAfterPointerAway.checkpoint.scene.clip_from_world,
        "VIS1 unobscured Gather dot capture must keep the actual camera pose fixed");
    const gatherClick = precedingInput(input, gatherImmediate.screenshotIndex, (entry) =>
        entry.call === "click" && entry.args?.[2] === "right" && nearPoint(entry.args, target.node));
    const gatherMarker = assertFirstFrameMarker(gatherClick, gatherImmediate, "gather");
    const node = resolveNode(gatherImmediate.checkpoint, target.node);
    assert.equal(gatherMarker.units.length, 1, "VIS1 Gather marker must preserve the selected worker only");
    assert(Math.hypot(gatherMarker.x - node.translation[0], gatherMarker.y - node.translation[2]) <= 0.1,
        "VIS1 Gather marker target must match the actual rendered resource node position");
    assert(Math.hypot(gatherMarker.x - node.position.x, gatherMarker.y - node.position.y) <= 0.1,
        "VIS1 Gather marker target must match the actual public replica node position");
    const beforeGatherImage = await screenshot(view, "vis1-gather-before");
    const afterGatherImage = await screenshot(view, "vis1-gather-after-pointer-away");
    assertGatherDotPixels(afterGatherImage, beforeGatherImage, gatherAfterPointerAway.checkpoint.scene, gatherMarker);
}

function requireStage(stages, label) {
    const stage = stages.get(label);
    assert(stage, `VIS1 is missing saved screenshot ${label}.png and its C84 checkpoint`);
    return stage;
}

function previousCall(input, before, name) {
    for (let index = before - 1; index >= 0; index -= 1) if (input[index].call === name) return index;
    return -1;
}

function precedingInput(input, screenshotIndex, predicate) {
    for (let index = screenshotIndex - 1; index >= 0; index -= 1) if (predicate(input[index])) return input[index];
    return null;
}

async function screenshot(view, label) {
    return decodePng(Buffer.from(await view.serverFile(`${label}.png.b64`), "base64"));
}

function resolveUnit(stage, target, expectedSlot, expectedContent) {
    const sceneUnits = stage.scene.units || [];
    const publicUnits = stage.debug.replicated_world?.units || [];
    const publicById = new Map(publicUnits.map((unit) => [unit.unit_id, unit]));
    const entry = sceneUnits.map((unit) => ({ unit, replica: publicById.get(unit.unit_id) }))
        .filter(({ unit, replica }) => replica && Array.isArray(unit.ground_screen_center) &&
            Math.hypot(unit.ground_screen_center[0] - target.x, unit.ground_screen_center[1] - target.y) <= 40)
        .sort((left, right) => Math.hypot(left.unit.ground_screen_center[0] - target.x, left.unit.ground_screen_center[1] - target.y) -
            Math.hypot(right.unit.ground_screen_center[0] - target.x, right.unit.ground_screen_center[1] - target.y))[0];
    assert(entry, `VIS1 PNG-derived target at ${target.x},${target.y} has no actual rendered C84 unit`);
    assert.equal(entry.replica.owner_slot, expectedSlot, "VIS1 target owner must match actual public unit replica");
    assert.equal(entry.replica.content_id, expectedContent, "VIS1 target content must match the authored fixture role");
    return {
        ...entry.unit,
        owner_slot: entry.replica.owner_slot,
        content_id: entry.replica.content_id,
        max_health: entry.replica.max_health,
    };
}

function ownerOf(debug, unitId) {
    return (debug.replicated_world?.units || []).find((unit) => unit.unit_id === unitId)?.owner_slot;
}

function assertSlotColor(image, unit, slot) {
    const expected = SLOT_RGB[slot];
    let matches = 0;
    for (let y = Math.round(unit.ground_screen_center[1] - 30); y <= Math.round(unit.ground_screen_center[1] - 3); y += 1) {
        for (let x = Math.round(unit.ground_screen_center[0] - 13); x <= Math.round(unit.ground_screen_center[0] + 13); x += 1) {
            if (nearColor(pixelAt(image, x, y), expected, 70)) matches += 1;
        }
    }
    assert(matches >= 5, `VIS1 actual slot-${slot} unit body has ${matches} pixels near its authored palette material`);
}

function visibleRingPixels(image, scene, unit, radius) {
    if (!Number.isFinite(radius) || radius <= 0) return false;
    const samples = ringSamples(image, scene, unit, radius, 16, 0.025);
    return samples.filter((sample) => sample.coloured).length >= 9;
}

function visibleForeignGaps(image, scene, unit, unselectedImage) {
    const radius = unit.pick_radius;
    if (!Number.isFinite(radius) || radius <= 0 || !unselectedImage ||
        image.width !== unselectedImage.width || image.height !== unselectedImage.height) return false;
    const ring = ringSamples(image, scene, unit, radius * 0.94, 16, 0.025, 1);
    const colouredQuadrants = [1, 5, 9, 13].every((start) =>
        ring.slice(start, start + 3).some((sample) => sample.coloured));
    const gapsAtCardinals = [0, 4, 8, 12].every((index) => {
        const point = ring[index].point;
        if (!point) return true;
        return ringGapDiffersFromUnit(image, unselectedImage, point, SLOT_RGB[unit.owner_slot]);
    });
    return colouredQuadrants && gapsAtCardinals;
}

function ringGapDiffersFromUnit(image, unselectedImage, point, slotColour) {
    for (let dy = -2; dy <= 2; dy += 1) for (let dx = -2; dx <= 2; dx += 1) {
        const x = Math.round(point[0] + dx);
        const y = Math.round(point[1] + dy);
        const selected = pixelAt(image, x, y);
        const unselected = pixelAt(unselectedImage, x, y);
        if (!selected || !unselected) continue;
        const changed = Math.hypot(selected[0] - unselected[0], selected[1] - unselected[1], selected[2] - unselected[2]) >= 25;
        if (changed && !nearColor(selected, slotColour, 85)) return true;
    }
    return false;
}

function ringSamples(image, scene, unit, radius, count, height, colourSearchRadius = 2) {
    const matrix = scene.clip_from_world;
    const viewport = scene.viewport_logical_rect;
    assert(Array.isArray(matrix) && matrix.length === 16 && Array.isArray(viewport), "VIS1 requires actual C84 camera projection metadata");
    return Array.from({ length: count }, (_, index) => {
        const angle = index * Math.PI * 2 / count;
        const point = project(matrix, viewport, [unit.translation[0] + Math.cos(angle) * radius, height,
            unit.translation[2] + Math.sin(angle) * radius]);
        return { point, coloured: Boolean(point && hasColor(image, point, SLOT_RGB[unit.owner_slot], 85, colourSearchRadius)) };
    });
}

function assertRangePixels(image, scene, unit, range) {
    const samples = ringSamples(image, scene, unit, range, 16, 0.018);
    assert(samples.filter((sample) => sample.point && hasColor(image, sample.point, RANGE_RGB, 70)).length >= 5,
        "VIS1 selected trooper range circle must have actual authored range-colour screenshot pixels");
}

function healthFillWidth(image, unit) {
    const center = unit.ground_screen_center;
    assert(Array.isArray(center) && center.length === 2,
        "VIS1 health bar target must map to an actual rendered scene unit");
    const x0 = Math.max(0, Math.floor(center[0] - 32));
    const x1 = Math.min(image.width - 1, Math.ceil(center[0] + 32));
    const y0 = Math.max(0, Math.floor(center[1] - 38));
    const y1 = Math.min(image.height - 1, Math.ceil(center[1] - 3));
    let maximum = 0;
    for (let y = y0; y <= y1; y += 1) {
        let run = 0;
        for (let x = x0; x <= x1; x += 1) {
            if (nearColor(pixelAt(image, x, y), HEALTH_RGB, 55)) {
                run += 1;
                maximum = Math.max(maximum, run);
            } else {
                run = 0;
            }
        }
    }
    return maximum;
}

function assertHealthBar(image, unit, label) {
    const width = healthFillWidth(image, unit);
    assert(width >= 6, `VIS1 ${label} must draw its actual health fill in screenshot pixels`);
    return width;
}

function assertNoHealthBar(image, unit, label) {
    const width = healthFillWidth(image, unit);
    assert(width <= 1, `VIS1 ${label} must not draw a health fill after selection clears; observed ${width}px`);
}

function healthFillRatioMatches(damagedWidth, fullWidth, current, maximum) {
    if (!(Number.isFinite(damagedWidth) && Number.isFinite(fullWidth) && fullWidth > 0 &&
        Number.isFinite(current) && Number.isFinite(maximum) && maximum > 0 && damagedWidth < fullWidth)) return false;
    const actual = damagedWidth / fullWidth;
    const expected = current / maximum;
    return Math.abs(actual - expected) <= Math.max(0.12, 2 / fullWidth);
}

function assertTooltipPixels(image, scene, unit) {
    const anchor = project(scene.clip_from_world, scene.viewport_logical_rect,
        [unit.translation[0], 2.0, unit.translation[2]]);
    assert(anchor, "VIS1 tooltip anchor must project from the actual hovered unit");
    const x0 = Math.max(0, Math.floor(anchor[0] + 8));
    const y0 = Math.max(0, Math.floor(anchor[1] - 28));
    let background = 0;
    let foreground = 0;
    for (let y = y0; y < Math.min(image.height, y0 + 30); y += 1) for (let x = x0; x < Math.min(image.width, x0 + 190); x += 1) {
        const pixel = pixelAt(image, x, y);
        if (nearColor(pixel, TOOLTIP_BG_RGB, 45)) background += 1;
        if (pixel && pixel[0] > 185 && pixel[1] > 185 && pixel[2] > 185) foreground += 1;
    }
    assert(background >= 80, "VIS1 delayed tooltip panel must be visible at the actual hovered unit anchor");
    assert(foreground >= 8, "VIS1 delayed tooltip must render visible name/owner glyph pixels");
}

function assertFirstFrameMarker(action, stage, kind, unitId = null) {
    assert(action, `VIS1 ${kind} marker has no corresponding physical target click`);
    const up = action.actual_events?.find((event) => event.type === "pointerup");
    assert(up?.canvas_target && Number.isInteger(up.seq), `VIS1 ${kind} click must release on canvas with an actual input sequence`);
    const records = stage.checkpoint.debug.frames?.retained_input || [];
    const applied = records.filter((record) => record.applied_seq?.includes(up.seq));
    assert.equal(applied.length, 1, `VIS1 ${kind} pointer release ${up.seq} must apply to exactly one first frame`);
    const frame = applied[0];
    const marker = (frame.markers || []).find((entry) => entry.kind === kind && (!unitId || entry.units?.includes(unitId)));
    assert(marker, `VIS1 ${kind} marker must be current in the first applied pointer-release frame`);
    const expectedOrder = kind === "attack" ? "muster:order_attack" :
        kind === "gather" ? "muster:order_gather" : "muster:order_move";
    assert.equal(marker.order, expectedOrder, `VIS1 ${kind} marker must preserve the exact sent order`);
    const localSlot = stage.checkpoint.debug.slot;
    const owners = new Map((stage.checkpoint.debug.replicated_world?.units || []).map((unit) => [unit.unit_id, unit.owner_slot]));
    assert(Array.isArray(marker.units) && marker.units.length > 0 && marker.units.every((id) => owners.get(id) === localSlot),
        `VIS1 ${kind} marker must preserve the exact own units from its sent envelope`);
    assert.deepEqual([...marker.units].sort((left, right) => left - right), [...frame.selection].sort((left, right) => left - right),
        `VIS1 ${kind} marker units must equal the actual selected units in the first applied frame`);
    return marker;
}

function resolveNode(stage, target) {
    const sceneNodes = stage.scene.nodes || [];
    const node = sceneNodes.map((entry) => ({ entry, distance: Math.hypot(
        entry.ground_screen_center?.[0] - target.x, entry.ground_screen_center?.[1] - target.y) }))
        .filter(({ entry, distance }) => Number.isFinite(distance) && distance <= 40 &&
            Array.isArray(entry.translation) && entry.translation.length === 3)
        .sort((left, right) => left.distance - right.distance)[0];
    assert(node, "VIS1 PNG-derived Gather click must identify an actual rendered node");
    const publicNode = (stage.debug.replicated_world?.nodes || []).find((entry) =>
        numericId(entry.id) === node.entry.node_id);
    assert(publicNode, "VIS1 Gather target node must exist in the actual public replica");
    assert(publicNode.pos && Number.isFinite(publicNode.pos.x) && Number.isFinite(publicNode.pos.y),
        "VIS1 Gather target must carry its actual replicated world position");
    return { ...node.entry, position: publicNode.pos };
}

function numericId(value) {
    if (Number.isInteger(value)) return value;
    if (Array.isArray(value) && Number.isInteger(value[0])) return value[0];
    if (value && Number.isInteger(value[0])) return value[0];
    return null;
}

function assertGatherDotPixels(image, before, scene, marker) {
    const expected = RANGE_RGB;
    const changedSamples = new Set();
    for (const dx of [-0.04, 0, 0.04]) {
        for (const dz of [-0.04, 0, 0.04]) {
            const point = project(scene.clip_from_world, scene.viewport_logical_rect,
                [marker.x + dx, 0.035, marker.y + dz]);
            if (!point) continue;
            for (let py = Math.round(point[1]) - 1; py <= Math.round(point[1]) + 1; py += 1) {
                for (let px = Math.round(point[0]) - 1; px <= Math.round(point[0]) + 1; px += 1) {
                    const afterPixel = pixelAt(image, px, py);
                    const beforePixel = pixelAt(before, px, py);
                    if (!nearColor(afterPixel, expected, 85) || !beforePixel) continue;
                    const delta = Math.hypot(afterPixel[0] - beforePixel[0],
                        afterPixel[1] - beforePixel[1], afterPixel[2] - beforePixel[2]);
                    if (delta >= 25) changedSamples.add(`${px},${py}`);
                }
            }
        }
    }
    assert(changedSamples.size >= 3,
        `VIS1 Gather marker must change at least three actual dot pixels over the same-pose pre-Gather image; observed ${changedSamples.size}`);
}

function assertMarkerPixels(image, scene, marker) {
    const visible = visibleMarkerRingPixels(image, scene, marker);
    assert(visible >= 5, `VIS1 hollow move-marker ring must have actual circumference pixels; observed ${visible}`);
}

function visibleMarkerRingPixels(image, scene, marker) {
    const matches = [];
    for (let step = 0; step < 16; step += 1) {
        const angle = step * Math.PI * 2 / 16;
        const point = project(scene.clip_from_world, scene.viewport_logical_rect,
            [marker.x + Math.cos(angle) * 0.38, 0.035, marker.y + Math.sin(angle) * 0.38]);
        if (point && hasColor(image, point, RANGE_RGB, 85)) matches.push(step);
    }
    return matches.length;
}

function assertAttackPixels(image, scene, marker) {
    const center = project(scene.clip_from_world, scene.viewport_logical_rect, [marker.x, 0.04, marker.y]);
    assert(center, "VIS1 attack marker target must project into the actual viewport");
    const expected = [245, 224, 51];
    let matches = 0;
    for (let offset = -5; offset <= 5; offset += 1) {
        if (hasColor(image, [center[0] + offset, center[1] + offset], expected, 80)) matches += 1;
        if (hasColor(image, [center[0] + offset, center[1] - offset], expected, 80)) matches += 1;
    }
    assert(matches >= 4, "VIS1 attack marker must be drawn in actual screenshot pixels at the C84 target");
}

function assertWaypointPixels(image, checkpoint, marker) {
    assert(Number.isFinite(marker.x) && Number.isFinite(marker.y), "VIS1 queued waypoint needs actual C84 target coordinates");
    const firstUnitId = marker.units?.[0];
    const scene = checkpoint.scene;
    const unit = scene.units?.find((entry) => entry.unit_id === firstUnitId);
    assert(unit, "VIS1 queued waypoint must map to an actual rendered own unit");
    const records = checkpoint.debug.replicated_world?.unit_orders;
    assert(Array.isArray(records), "VIS1 waypoint oracle requires actual owner-public UnitOrderRecord DTOs");
    const record = records.find((entry) => orderUnitId(entry.unit) === firstUnitId);
    assert(record, `VIS1 waypoint unit ${firstUnitId} has no actual owner-public order record`);
    const actualOrders = [record.current, ...(record.queued || [])].filter(Boolean);
    const points = actualOrders.map(pointTarget).filter(Boolean);
    assert(points.length >= 2,
        "VIS1 waypoint screenshot must show the actual current and queued point order records");
    const finalTarget = points.at(-1);
    assert(Math.hypot(finalTarget.x - marker.x, finalTarget.y - marker.y) <= 0.1,
        "VIS1 final waypoint must match the actual queued C84 target after the physical click");
    const matches = waypointPolylineColorMatches(image, scene, unit, points);
    assert(matches.length >= 2 && matches.every((segment) => segment >= 3),
        "VIS1 every actual current/queued UnitOrderRecord segment must appear in screenshot pixels");
}

function waypointPolylineColorMatches(image, scene, unit, points) {
    let previous = [unit.translation[0], 0.04, unit.translation[2]];
    const matches = [];
    for (const orderTarget of points) {
        const next = [orderTarget.x, 0.04, orderTarget.y];
        const startPixel = project(scene.clip_from_world, scene.viewport_logical_rect, previous);
        const endPixel = project(scene.clip_from_world, scene.viewport_logical_rect, next);
        assert(startPixel && endPixel, "VIS1 current/queued waypoint segments must project into the actual viewport");
        let segmentMatches = 0;
        for (let step = 1; step < 9; step += 1) {
            const t = step / 9;
            const point = [startPixel[0] * (1 - t) + endPixel[0] * t,
                startPixel[1] * (1 - t) + endPixel[1] * t];
            if (hasColor(image, point, WAYPOINT_RGB, 80)) segmentMatches += 1;
        }
        matches.push(segmentMatches);
        previous = next;
    }
    return matches;
}

function orderUnitId(value) {
    if (Number.isInteger(value)) return value;
    if (Array.isArray(value) && Number.isInteger(value[0])) return value[0];
    if (value && Number.isInteger(value[0])) return value[0];
    return null;
}

function pointTarget(order) {
    const target = order?.target?.Point ?? order?.target?.point;
    return target && Number.isFinite(target.x) && Number.isFinite(target.y) ? target : null;
}

function nearPoint(args, point) {
    return Array.isArray(args) && Number.isFinite(args[0]) && Number.isFinite(args[1]) &&
        Math.hypot(args[0] - point.x, args[1] - point.y) <= 1;
}

function project(matrix, viewport, point) {
    const [x, y, z] = point;
    const clip = [matrix[0] * x + matrix[4] * y + matrix[8] * z + matrix[12],
        matrix[1] * x + matrix[5] * y + matrix[9] * z + matrix[13],
        matrix[2] * x + matrix[6] * y + matrix[10] * z + matrix[14],
        matrix[3] * x + matrix[7] * y + matrix[11] * z + matrix[15]];
    if (!Number.isFinite(clip[3]) || clip[3] <= 1e-12) return null;
    return [viewport[0] + (clip[0] / clip[3] + 1) * viewport[2] / 2,
        viewport[1] + (1 - clip[1] / clip[3]) * viewport[3] / 2];
}

function hasColor(image, point, expected, tolerance, radius = 2) {
    for (let dy = -radius; dy <= radius; dy += 1) for (let dx = -radius; dx <= radius; dx += 1) {
        if (nearColor(pixelAt(image, Math.round(point[0] + dx), Math.round(point[1] + dy)), expected, tolerance)) return true;
    }
    return false;
}

function nearColor(pixel, expected, tolerance) {
    return Boolean(pixel && pixel[3] >= 245 && Math.hypot(pixel[0] - expected[0], pixel[1] - expected[1], pixel[2] - expected[2]) <= tolerance);
}

module.exports = { judge, project, ringSamples, visibleRingPixels, visibleForeignGaps, ringGapDiffersFromUnit, assertHealthBar, assertTooltipPixels, waypointPolylineColorMatches, healthFillRatioMatches, visibleMarkerRingPixels };
