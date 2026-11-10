// turts-test-role: oracle
"use strict";

const assert = require("node:assert/strict");
const { decodePng, pixelAt } = require("./harness/png.js");
const { project, ringSamples, visibleForeignGaps } = require("./vis-full-oracle.js");

const SLOT_RGB = [[0, 114, 178], [230, 159, 0]];
const RANGE_RGB = [242, 204, 96];

async function judge(view) {
    const input = view.inputLog();
    const checkpoints = JSON.parse(await view.serverFile("scene-checkpoints.json"));
    const driver = JSON.parse(await view.serverFile("sel1-steps.json"));
    const pixels = JSON.parse(await view.serverFile("sel1-pixel-targets.json"));
    assert(Array.isArray(checkpoints) && Array.isArray(driver.shots) && Array.isArray(driver.calls),
        "SEL1 requires screenshot checkpoints and pixel-derived driver records");
    assert.equal(pixels.method,
        "connected components of the authored slot palette and open-ground pixels from screenshots only",
        "SEL1 targets must come from screenshots, without package or debug reads");

    const stages = new Map();
    for (const shot of driver.shots) {
        const saveIndex = input.findIndex((entry) => entry.call === "save" && entry.args?.[0] === `${shot.label}.png`);
        assert(saveIndex >= 0, `SEL1 ${shot.label} must save its actual screenshot`);
        const screenshotIndex = previousScreenshot(input, saveIndex);
        assert(screenshotIndex >= 0, `SEL1 ${shot.label} must follow a screenshot IO call`);
        const checkpoint = checkpoints.find((entry) => entry.input_call === screenshotIndex && entry.window === 1);
        assert(checkpoint?.scene && checkpoint?.debug, `SEL1 ${shot.label} needs an actual C84 checkpoint`);
        stages.set(shot.label, { checkpoint, screenshotIndex, target: shot.target });
    }

    const ownTarget = pixels.targets.own;
    const foreignTarget = pixels.targets.foreign;
    const initialStage = requireStage(stages, "sel1-initial");
    const ownStage = requireStage(stages, "sel1-own-selected");
    const foreignStage = requireStage(stages, "sel1-foreign-selected");
    const clearedStage = requireStage(stages, "sel1-ground-cleared");
    const hoverImmediate = requireStage(stages, "sel1-hover-immediate");
    const hoverDelayed = requireStage(stages, "sel1-hover-delayed");

    const initialOwnUnit = resolveTarget(initialStage.checkpoint, ownTarget, 0);
    const initialForeignTurret = resolveTarget(initialStage.checkpoint, foreignTarget, 1);
    const ownUnit = resolveTarget(ownStage.checkpoint, ownTarget, 0);
    const foreignTurret = resolveTarget(foreignStage.checkpoint, foreignTarget, 1);
    assert.equal(initialOwnUnit.replica.unit_id, ownUnit.replica.unit_id,
        "SEL1 own reference and selected captures must resolve to the same actual unit");
    assert.equal(initialForeignTurret.replica.unit_id, foreignTurret.replica.unit_id,
        "SEL1 foreign reference and selected captures must resolve to the same actual turret");
    assertSamePixelReference(initialStage.checkpoint.scene, ownStage.checkpoint.scene,
        initialOwnUnit.scene, ownUnit.scene, "own selection ring");
    assertSamePixelReference(initialStage.checkpoint.scene, clearedStage.checkpoint.scene,
        initialOwnUnit.scene, ownUnit.scene, "cleared selection ring");
    assertSamePixelReference(initialStage.checkpoint.scene, foreignStage.checkpoint.scene,
        initialForeignTurret.scene, foreignTurret.scene, "foreign selection ring");
    assertSamePixelReference(initialStage.checkpoint.scene, hoverImmediate.checkpoint.scene,
        initialForeignTurret.scene, foreignTurret.scene, "foreign hover ring");
    assertSamePixelReference(initialStage.checkpoint.scene, hoverDelayed.checkpoint.scene,
        initialForeignTurret.scene, foreignTurret.scene, "world tooltip");
    assert.equal(foreignTurret.replica.content_id, "forge:turret",
        "SEL1 foreign pixel target must resolve to the fixture's weapon turret");
    assertSelection(ownStage.checkpoint.debug, ownUnit.scene, "own unit", true);
    assertSelection(foreignStage.checkpoint.debug, foreignTurret.scene, "foreign turret", false);
    assertSelectionCleared(clearedStage.checkpoint.debug);
    assert.deepEqual(clearedStage.checkpoint.debug.rings_shown || [], [],
        "SEL1 ground-cleared checkpoint must report no remaining selected-ring records");
    assertSelectionCleared(hoverImmediate.checkpoint.debug);
    assertSelectionCleared(hoverDelayed.checkpoint.debug);
    assert.deepEqual(foreignStage.checkpoint.debug.replicated_world?.unit_orders || [],
        ownStage.checkpoint.debug.replicated_world?.unit_orders || [],
        "SEL1 selecting a foreign turret must not issue an order");

    assertPhysicalClick(input, driver.calls, "select-own", ownTarget);
    assertPhysicalClick(input, driver.calls, "select-foreign-turret", foreignTarget);
    assertPhysicalClick(input, driver.calls, "clear-on-ground", pixels.targets.ground);

    const foreignSnapshot = foreignStage.checkpoint.debug.ui_snapshot || [];
    const commandCard = foreignSnapshot.find((row) => row.id === "command_card");
    assert(commandCard && commandCard.visible === false,
        "SEL1 foreign selection must keep the owner command card hidden");
    const foreignButtons = foreignSnapshot.filter((row) => row.id.startsWith("command_rows/") &&
        row.kind === "button" && row.visible && row.widget_action?.kind === "order");
    assert.equal(foreignButtons.length, 0,
        "SEL1 foreign selection must expose no actionable order buttons");

    const ownImage = await screenshot(view, "sel1-own-selected");
    const unselectedImage = await screenshot(view, "sel1-initial");
    const foreignImage = await screenshot(view, "sel1-foreign-selected");
    const clearImage = await screenshot(view, "sel1-ground-cleared");
    const hoverImage = await screenshot(view, "sel1-hover-immediate");
    const tooltipImage = await screenshot(view, "sel1-hover-delayed");
    assertSelectionRingPixels(ownImage, unselectedImage, ownStage.checkpoint.scene, ownUnit.scene,
        "SEL1 selected own unit must show its slot-coloured selection ring");
    assertSelectionRingPixels(foreignImage, unselectedImage, foreignStage.checkpoint.scene, foreignTurret.scene,
        "SEL1 selected foreign turret must show its slot-coloured read-only ring");
    assertRangeCircle(foreignImage, foreignStage.checkpoint.scene, foreignTurret.scene,
        8.0);
    assertNoSelectionRing(clearImage, unselectedImage, clearedStage.checkpoint.scene, ownUnit.scene,
        "SEL1 ground click must remove the own selection ring");

    assertHoverWait(input, foreignTarget, hoverImmediate.checkpoint.wall_time_ms,
        hoverDelayed.checkpoint.wall_time_ms);
    assertNoWorldTooltip(hoverImmediate.checkpoint.debug, foreignTurret.replica.unit_id);
    assertWorldTooltip(hoverDelayed.checkpoint.debug, foreignTurret.replica.unit_id);
    const bridgeUi = await view.bridge("turts_ui");
    assertWorldTooltipRows(typeof bridgeUi === "string" ? JSON.parse(bridgeUi) : bridgeUi,
        foreignTurret.replica.unit_id);
    assert.equal(visibleForeignGaps(unselectedImage, hoverImmediate.checkpoint.scene,
        foreignTurret.scene, unselectedImage), false,
    "SEL1 unhovered foreign-unit reference must not satisfy the actual hover-ring oracle");
    assert(visibleForeignGaps(hoverImage, hoverImmediate.checkpoint.scene,
        foreignTurret.scene, unselectedImage),
    "SEL1 hover must show the actual foreign-ring segments and neutral gaps before the tooltip delay");
    assertTooltip(tooltipImage, hoverDelayed.checkpoint.scene, foreignTurret.scene);
}

function previousScreenshot(input, from) {
    for (let index = from - 1; index >= 0; index -= 1) {
        if (input[index].call === "screenshot") return index;
    }
    return -1;
}

function requireStage(stages, label) {
    const stage = stages.get(label);
    assert(stage, `SEL1 driver is missing screenshot stage ${label}`);
    return stage;
}

function resolveTarget(checkpoint, target, ownerSlot) {
    const replicas = checkpoint.debug.replicated_world?.units || [];
    const rendered = checkpoint.scene.units || [];
    const matches = rendered.map((scene) => ({
        scene,
        replica: replicas.find((unit) => unit.unit_id === scene.unit_id),
    })).filter(({ scene, replica }) => replica && Array.isArray(scene.ground_screen_center) &&
        Math.hypot(scene.ground_screen_center[0] - target.x,
            scene.ground_screen_center[1] - target.y) <= 48)
        .sort((left, right) => Math.hypot(left.scene.ground_screen_center[0] - target.x,
            left.scene.ground_screen_center[1] - target.y) -
            Math.hypot(right.scene.ground_screen_center[0] - target.x,
                right.scene.ground_screen_center[1] - target.y));
    assert(matches.length > 0, "SEL1 pixel target must map to an actual rendered unit");
    const resolved = matches[0];
    assert.equal(resolved.replica.owner_slot, ownerSlot,
        `SEL1 pixel target must belong to slot ${ownerSlot}`);
    return {
        scene: { ...resolved.scene, owner_slot: resolved.replica.owner_slot },
        replica: resolved.replica,
    };
}

function assertSelection(debug, unit, label, own) {
    assert.deepEqual(debug.selection, [unit.unit_id], `SEL1 ${label} must be the only selected unit`);
    assert(Number.isFinite(unit.pick_radius) && unit.pick_radius > 0,
        `SEL1 ${label} must have a positive actual pick radius`);
    assert((debug.rings_shown || []).some((ring) => ring.unit === unit.unit_id &&
        ring.own === own && ring.slot === unit.owner_slot &&
        Math.abs(ring.radius - unit.pick_radius) <= 1e-5),
        `SEL1 ${label} rendered ring must match actual unit, owner slot, ownership and pick radius`);
}

function assertSelectionCleared(debug) {
    assert.deepEqual(debug.selection, [], "SEL1 ground click must clear the selection");
}

function assertPhysicalClick(input, calls, label, target) {
    assert(calls.some((call) => call.label === label && call.method === "click" &&
        call.args[0] === target.x && call.args[1] === target.y && call.args[2] === "left"),
    `SEL1 ${label} must use its screenshot-derived physical target`);
    assert(input.some((entry) => entry.call === "click" && entry.args?.[0] === target.x &&
        entry.args?.[1] === target.y && entry.args?.[2] === "left" && entry.event_seq?.length >= 3),
    `SEL1 ${label} must reach the canvas as real pointer IO`);
}

function assertSelectionRingPixels(image, unselectedImage, scene, unit, message) {
    const samples = ringSamples(image, scene, unit, unit.pick_radius, 24, 0.025, 2);
    const changed = samples.filter((sample) => {
        if (!sample.point || !sample.coloured) return false;
        const selected = pixelAt(image, Math.round(sample.point[0]), Math.round(sample.point[1]));
        const unselected = pixelAt(unselectedImage, Math.round(sample.point[0]), Math.round(sample.point[1]));
        return selected && unselected && colorDistance(selected, unselected) >= 18;
    }).length;
    assert(changed >= 3, `${message}; only ${changed} slot-coloured projected samples differ from the unselected reference`);
}

function assertNoSelectionRing(image, unselectedImage, scene, unit, message) {
    const samples = ringSamples(image, scene, unit, unit.pick_radius, 24, 0.025, 2);
    const deltas = samples.filter((sample) => sample.point).map((sample) => {
        const current = pixelAt(image, Math.round(sample.point[0]), Math.round(sample.point[1]));
        const reference = pixelAt(unselectedImage, Math.round(sample.point[0]), Math.round(sample.point[1]));
        return current && reference ? colorDistance(current, reference) : Infinity;
    });
    assert(deltas.length >= 20 && deltas.every((delta) => delta <= 8),
        `${message}; cleared ring samples must match the actual unselected screenshot`);
}

function colorDistance(left, right) {
    return Math.hypot(left[0] - right[0], left[1] - right[1], left[2] - right[2]);
}

function assertSamePixelReference(reference, current, referenceUnit, currentUnit, label) {
    assert.deepEqual(current.clip_from_world, reference.clip_from_world,
        `SEL1 ${label} must keep the actual C84 camera projection fixed`);
    assert.deepEqual(current.viewport_logical_rect, reference.viewport_logical_rect,
        `SEL1 ${label} must keep the actual C84 viewport fixed`);
    assert.deepEqual(currentUnit.translation, referenceUnit.translation,
        `SEL1 ${label} must keep the same actual unit position`);
}

function assertRangeCircle(image, scene, unit, range) {
    assert.equal(range, 8.0, "SEL1 foreign turret fixture range must be the authored eight world units");
    const colourHits = projectedCircleHits(image, scene, unit, range, RANGE_RGB);
    assert(colourHits >= 5,
        `SEL1 foreign weapon must draw its actual range circle; found ${colourHits} range-coloured samples`);
}

function projectedCircleHits(image, scene, unit, radius, colour) {
    let hits = 0;
    for (let index = 0; index < 24; index += 1) {
        const angle = index * Math.PI * 2 / 24;
        const point = project(scene.clip_from_world, scene.viewport_logical_rect,
            [unit.translation[0] + Math.cos(angle) * radius, 0.018,
                unit.translation[2] + Math.sin(angle) * radius]);
        if (!point) continue;
        let found = false;
        for (let dy = -2; dy <= 2 && !found; dy += 1) {
            for (let dx = -2; dx <= 2; dx += 1) {
                const pixel = pixelAt(image, Math.round(point[0] + dx), Math.round(point[1] + dy));
                if (pixel && pixel[3] >= 245 && Math.hypot(pixel[0] - colour[0],
                    pixel[1] - colour[1], pixel[2] - colour[2]) <= 70) {
                    found = true;
                    break;
                }
            }
        }
        if (found) hits += 1;
    }
    return hits;
}

function assertHoverWait(input, target, immediateTime, delayedTime) {
    const move = input.find((entry) => entry.call === "move" && entry.args?.[0] === target.x &&
        entry.args?.[1] === target.y && entry.event_seq?.length >= 1);
    assert(move, "SEL1 tooltip requires a physical move to its pixel-derived target");
    const beforeDelayMs = immediateTime - move.wall_time_ms;
    const afterDelayMs = delayedTime - move.wall_time_ms;
    assert(beforeDelayMs >= 0 && beforeDelayMs < 500,
        `SEL1 immediate checkpoint must precede the 500 ms tooltip delay; elapsed=${beforeDelayMs} ms`);
    assert(afterDelayMs >= 500,
        `SEL1 delayed checkpoint must follow the 500 ms tooltip delay; elapsed=${afterDelayMs} ms`);
    assert(input.some((entry) => entry.call === "wait" && entry.args?.[0] === 600 &&
        entry.wall_time_ms >= move.wall_time_ms && entry.wall_time_ms <= delayedTime),
    "SEL1 tooltip must follow the authored 500 ms hover delay");
}

function assertNoWorldTooltip(debug, unitId) {
    const id = `world.tooltip.${unitId}`;
    assert(!(debug.ui_snapshot || []).some((row) => row.kind === "world_tooltip" && row.id === id),
        "SEL1 world tooltip must remain absent before the 500 ms hover delay");
}

function assertWorldTooltip(debug, unitId) {
    assertWorldTooltipRows(debug.ui_snapshot || [], unitId);
}

function assertWorldTooltipRows(rows, unitId) {
    assert(Array.isArray(rows), "SEL1 turts_ui must return actual UI snapshot rows");
    const tooltip = rows.find((row) => row.kind === "world_tooltip" &&
        row.id === `world.tooltip.${unitId}` && row.unit_id === unitId);
    assert(tooltip, "SEL1 delayed turts_ui snapshot must include the hovered unit's world tooltip");
    assert.equal(tooltip.text, "Shell Turret · Slot 2",
        "SEL1 world tooltip text must name the foreign turret and its owner");
    assert.equal(tooltip.visible, true,
        "SEL1 delayed world tooltip must have actual visible computed UI geometry");
    assert(Array.isArray(tooltip.rect) && tooltip.rect.length === 4 &&
        tooltip.rect.every(Number.isFinite) && tooltip.rect[2] > 0 && tooltip.rect[3] > 0,
    "SEL1 world tooltip must expose its actual computed rectangle");
}

function assertTooltip(image, scene, unit) {
    const anchor = project(scene.clip_from_world, scene.viewport_logical_rect,
        [unit.translation[0], 2.0, unit.translation[2]]);
    assert(anchor, "SEL1 hovered unit must have an actual projected tooltip anchor");
    let background = 0;
    let foreground = 0;
    for (let y = Math.max(0, Math.floor(anchor[1] - 28));
        y < Math.min(image.height, anchor[1] + 2); y += 1) {
        for (let x = Math.max(0, Math.floor(anchor[0] + 8));
            x < Math.min(image.width, anchor[0] + 198); x += 1) {
            const pixel = pixelAt(image, x, y);
            if (pixel && Math.hypot(pixel[0] - 17, pixel[1] - 24, pixel[2] - 32) <= 45) background += 1;
            if (pixel && pixel[0] > 185 && pixel[1] > 185 && pixel[2] > 185) foreground += 1;
        }
    }
    assert(background >= 80, "SEL1 delayed world tooltip must show its panel pixels");
    assert(foreground >= 8, "SEL1 delayed world tooltip must show text glyph pixels");
}

async function screenshot(view, label) {
    return decodePng(Buffer.from(await view.serverFile(`${label}.png.b64`), "base64"));
}

module.exports = { judge };
