// turts-test-role: oracle
"use strict";

const assert = require("node:assert/strict");
const { decodePng, pixelAt } = require("./harness/png.js");

const SLOT_RGB = [[0, 114, 178], [230, 159, 0], [0, 158, 115], [213, 94, 0], [86, 180, 233], [204, 121, 167], [240, 228, 66], [0, 0, 0]];

async function judge(view, params) {
    const input = view.inputLog();
    const actions = input.filter((entry) => entry.call === "click" ||
        (entry.call === "drag" && entry.args?.[4] !== "middle"));
    assertSelectionCameraPans(input, params, actions[0]);
    const checkpoints = JSON.parse(await view.serverFile("scene-checkpoints.json"));
    assert(Array.isArray(checkpoints), "SEL needs actual C84 scene checkpoints");
    const initialCheckpoint = screenshotCheckpoint(input, checkpoints, "sel-initial.png");
    const initialImage = decodePng(Buffer.from(await view.serverFile("sel-initial.png.b64"), "base64"));
    let actionIndex = 0;
    for (const stage of params.stages || []) {
        const count = stage.gesture === "double-click" ? 2 : 1;
        const stageActions = actions.slice(actionIndex, actionIndex + count);
        actionIndex += count;
        const action = stageActions.at(-1);
        const expectedCall = stage.gesture === "box" ? "drag" : "click";
        assert.equal(stageActions.length, count, `SEL ${stage.label} is missing a real harness action`);
        assert(stageActions.every((entry) => entry.call === expectedCall), `SEL ${stage.label} used the wrong real harness gesture`);
        assert(action, `SEL ${stage.label} must be a physical harness ${stage.gesture}`);
        if (stage.gesture === "double-click") {
            const [first, second] = stageActions;
            assert(second.wall_time_ms - first.wall_time_ms <= 400, `SEL ${stage.label} clicks exceed the 400 ms double-click window`);
            assert(Math.hypot(second.args[0] - first.args[0], second.args[1] - first.args[1]) <= 4, `SEL ${stage.label} clicks exceed the 4 px double-click radius`);
        }
        const mouseUp = action.actual_events?.find((event) => event.type === "pointerup");
        assert(mouseUp?.canvas_target, `SEL ${stage.label} pointer release did not land on the rendered canvas`);
        const mouseUpSeq = mouseUp?.seq;
        assert(Number.isInteger(mouseUpSeq), `SEL ${stage.label} has no actual canvas pointer-release sequence`);
        const saveIndex = input.findIndex((entry) => entry.call === "save" && entry.args?.[0] === `${stage.label}.png`);
        assert(saveIndex > 0, `SEL ${stage.label} must capture its result from screenshot pixels`);
        let screenshotIndex = saveIndex - 1;
        while (screenshotIndex >= 0 && input[screenshotIndex].call !== "screenshot") screenshotIndex -= 1;
        assert(screenshotIndex >= 0, `SEL ${stage.label} has no result screenshot input`);
        const checkpoint = checkpoints.find((entry) => entry.input_call === screenshotIndex && entry.window === 1);
        assert(checkpoint?.debug && checkpoint?.scene, `SEL ${stage.label} screenshot has no actual C84 scene/debug checkpoint`);
        const debug = checkpoint.debug;
        const scene = checkpoint.scene;
        const frames = debug.frames?.retained_input;
        assert(Array.isArray(frames), `SEL ${stage.label} C84 checkpoint has no retained input-correlated frame list`);
        const appliedFrames = frames.filter((record) => record.applied_seq?.includes(mouseUpSeq));
        assert.equal(appliedFrames.length, 1, `SEL ${stage.label} pointer release ${mouseUpSeq} must apply to exactly one presented frame`);
        const frame = appliedFrames[0];
        if (Number.isInteger(stage.expected_rings)) assert.equal(frame.rings?.length, stage.expected_rings,
            `SEL ${stage.label} first pointer-release frame actual rendered ring count`);
        if (Number.isInteger(stage.expected_selection)) assert.equal(frame.selection?.length, stage.expected_selection,
            `SEL ${stage.label} first pointer-release frame selection count`);
        assertFrameSelectionIsOwn(frame, debug, stage);
        assertModifiers(action, stage);
        if (stage.gesture === "box") assertBoxSelectionCenters(action, frame, scene, stage);
        if (stage.label === "sel2-static-only") {
            assertStaticOnlySelection(frame, debug, stage);
            const encoded = await view.serverFile(`${stage.label}.png.b64`);
            assertStaticRingAppears(initialImage, initialCheckpoint, decodePng(Buffer.from(encoded, "base64")), checkpoint, frame, debug, stage);
        }
        if (stage.label === "sel2-mixed-owner") assertMixedOwnerBox(action, scene, debug);
        if (frame.selection.length > 0) {
            const encoded = await view.serverFile(`${stage.label}.png.b64`);
            const image = decodePng(Buffer.from(encoded, "base64"));
            const visible = frame.selection.filter((id) => hasRingPixels(image, scene, id, SLOT_RGB[stage.owner_slot ?? 0]));
            assert.equal(visible.length, frame.selection.length,
                `SEL ${stage.label} must show slot-coloured ring pixels for every selected unit in its saved screenshot`);
        }
        if (stage.expected_unit_content_id) assertTypeSelectionIsVisible(frame, debug, scene, stage);
    }
}

function screenshotCheckpoint(input, checkpoints, filename) {
    const saveIndex = input.findIndex((entry) => entry.call === "save" && entry.args?.[0] === filename);
    assert(saveIndex > 0, `SEL ${filename} must be saved from an actual screenshot`);
    let screenshotIndex = saveIndex - 1;
    while (screenshotIndex >= 0 && input[screenshotIndex].call !== "screenshot") screenshotIndex -= 1;
    assert(screenshotIndex >= 0, `SEL ${filename} has no screenshot input`);
    const checkpoint = checkpoints.find((entry) => entry.input_call === screenshotIndex && entry.window === 1);
    assert(checkpoint?.debug && checkpoint?.scene, `SEL ${filename} has no actual C84 scene/debug checkpoint`);
    return checkpoint;
}

function assertStaticRingAppears(initialImage, initialCheckpoint, selectedImage, selectedCheckpoint, frame, debug, stage) {
    const initialScene = initialCheckpoint.scene;
    const selectedScene = selectedCheckpoint.scene;
    assert.deepEqual(selectedScene.clip_from_world, initialScene.clip_from_world,
        "SEL2 static ring visibility comparison must keep the actual camera matrix fixed");
    assert.deepEqual(selectedScene.viewport_logical_rect, initialScene.viewport_logical_rect,
        "SEL2 static ring visibility comparison must keep the actual viewport fixed");
    const selectedIds = new Set(frame.selection);
    const selectedPublic = new Map((debug.replicated_world.units || []).map((unit) => [unit.unit_id, unit]));
    const initialPublic = new Map((initialCheckpoint.debug.replicated_world?.units || []).map((unit) => [unit.unit_id, unit]));
    const initialSceneUnits = new Map((initialScene.units || []).map((unit) => [unit.unit_id, unit]));
    const selectedSceneUnits = new Map((selectedScene.units || []).map((unit) => [unit.unit_id, unit]));
    const expectedColor = SLOT_RGB[stage.owner_slot ?? 0];
    const contents = [...stage.expected_content_ids].sort();
    for (const contentId of contents) {
        const unit = [...selectedPublic.values()].find((entry) => selectedIds.has(entry.unit_id) && entry.content_id === contentId);
        assert(unit, `SEL2 static ring check must resolve the selected public ${contentId}`);
        const initialUnit = initialPublic.get(unit.unit_id);
        const selectedSceneUnit = selectedSceneUnits.get(unit.unit_id);
        const initialSceneUnit = initialSceneUnits.get(unit.unit_id);
        assert(initialUnit && selectedSceneUnit && initialSceneUnit,
            `SEL2 static ring check must resolve ${contentId} in both actual replica and scene captures`);
        assert.equal(unit.owner_slot, debug.slot, `SEL2 ${contentId} must remain owned by the local slot`);
        assert.equal(initialUnit.owner_slot, unit.owner_slot, `SEL2 ${contentId} owner must stay fixed across captures`);
        assert.deepEqual(selectedSceneUnit.translation, initialSceneUnit.translation,
            `SEL2 ${contentId} world pose must stay fixed across captures`);
        assert.equal(selectedSceneUnit.pick_radius, initialSceneUnit.pick_radius,
            `SEL2 ${contentId} pick radius must stay fixed across captures`);
        assert.equal(changedRingColorPixels(initialImage, initialImage, initialScene, unit.unit_id, expectedColor), 0,
            `SEL2 ${contentId} unchanged-initial negative control must show no changed ring pixels`);
        const changed = changedRingColorPixels(initialImage, selectedImage, selectedScene, unit.unit_id, expectedColor);
        assert(changed >= 1,
            `SEL2 selected ${contentId} must add a changed slot-colour circumference pixel over the same-pose initial PNG; observed ${changed}`);
    }
}

function changedRingColorPixels(before, after, scene, unitId, expected) {
    const unit = scene?.units?.find((entry) => entry.unit_id === unitId);
    const radius = unit?.pick_radius;
    const world = unit?.translation;
    const matrix = scene?.clip_from_world;
    const viewport = scene?.viewport_logical_rect;
    if (!Array.isArray(world) || world.length !== 3 || !Number.isFinite(radius) || radius <= 0 ||
        !Array.isArray(matrix) || matrix.length !== 16 || !Array.isArray(viewport) || viewport.length !== 4) return 0;
    const changed = new Set();
    for (let step = 0; step < 16; step += 1) {
        const angle = Math.PI * 2 * step / 16;
        const point = project(matrix, viewport, [
            world[0] + Math.cos(angle) * radius * 0.94,
            0.025,
            world[2] + Math.sin(angle) * radius * 0.94,
        ]);
        if (!point) continue;
        for (let dy = -2; dy <= 2; dy += 1) for (let dx = -2; dx <= 2; dx += 1) {
            const x = Math.round(point[0] + dx);
            const y = Math.round(point[1] + dy);
            const afterPixel = pixelAt(after, x, y);
            const beforePixel = pixelAt(before, x, y);
            if (!afterPixel || !beforePixel || afterPixel[3] < 245 ||
                Math.hypot(afterPixel[0] - expected[0], afterPixel[1] - expected[1], afterPixel[2] - expected[2]) > 85 ||
                Math.hypot(afterPixel[0] - beforePixel[0], afterPixel[1] - beforePixel[1], afterPixel[2] - beforePixel[2]) < 25) continue;
            changed.add(`${x},${y}`);
        }
    }
    return changed.size;
}

function assertSelectionCameraPans(input, params, firstSelectionAction) {
    const sel3 = (params.stages || []).some((stage) => stage.label?.startsWith("sel3-"));
    const pans = input.filter((entry) => entry.call === "drag" && entry.args?.[4] === "middle");
    assert(firstSelectionAction, "SEL needs at least one physical selection action");
    const selectionIndex = input.indexOf(firstSelectionAction);
    assert.equal(pans.length, sel3 ? 2 : 0,
        "SEL3 must use the two declared physical middle-drag camera calibrations before zoomed selection");
    if (!sel3) return;
    for (const [index, pan] of pans.entries()) {
        assert(Number.isFinite(pan.wall_time_ms), `SEL3 camera pan ${index + 1} must be an actual logged physical input`);
        assert(Math.hypot(pan.args[2] - pan.args[0], pan.args[3] - pan.args[1]) > 0,
            `SEL3 camera pan ${index + 1} must physically move the pointer`);
        const down = pan.actual_events?.find((event) => event.type === "pointerdown");
        const up = pan.actual_events?.find((event) => event.type === "pointerup");
        assert(down?.canvas_target && up?.canvas_target,
            `SEL3 camera pan ${index + 1} must begin and end on the rendered canvas`);
        assert(input.indexOf(pan) < selectionIndex,
            `SEL3 camera pan ${index + 1} must precede the selection gesture`);
    }
}

function assertFrameSelectionIsOwn(frame, debug, stage) {
    const localSlot = debug?.slot;
    assert(Number.isInteger(localSlot), `SEL ${stage.label} debug snapshot has no actual local slot`);
    const units = debug?.replicated_world?.units;
    assert(Array.isArray(units), `SEL ${stage.label} has no actual public unit replica`);
    const byId = new Map(units.map((unit) => [unit.unit_id, unit]));
    for (const id of frame.selection) {
        const unit = byId.get(id);
        assert(unit, `SEL ${stage.label} selected id ${id} is absent from the actual public replica`);
        assert.equal(unit.owner_slot, localSlot, `SEL ${stage.label} selected a foreign owner slot`);
        if (stage.expected_content_ids) assert(stage.expected_content_ids.includes(unit.content_id), `SEL ${stage.label} selected unexpected ${unit.content_id}`);
    }
}

function assertBoxSelectionCenters(action, frame, scene, stage) {
    const [x0, y0, x1, y1, button] = action.args;
    assert.equal(button, "left", `SEL ${stage.label} must use a physical left drag`);
    const box = { minX: Math.min(x0, x1), maxX: Math.max(x0, x1), minY: Math.min(y0, y1), maxY: Math.max(y0, y1) };
    const selected = new Set(frame.selection);
    const centers = (scene?.units || []).filter((unit) => selected.has(unit.unit_id))
        .map((unit) => unit.ground_screen_center)
        .filter((center) => Array.isArray(center) && center.length === 2 && center.every(Number.isFinite));
    assert.equal(centers.length, frame.selection.length,
        `SEL ${stage.label} selected centers must map to actual rendered scene units`);
    const inside = centers.filter(([x, y]) => x >= box.minX && x <= box.maxX && y >= box.minY && y <= box.maxY);
    const adds = (stage.modifiers || []).includes("Shift");
    if (adds) assert(inside.length > 0, `SEL ${stage.label} Shift+drag must contain an actual selected center`);
    else assert.equal(inside.length, centers.length,
        `SEL ${stage.label} box must contain every selected unit center in the captured frame`);
}

function assertStaticOnlySelection(frame, debug, stage) {
    const units = debug?.replicated_world?.units;
    assert(Array.isArray(units), `SEL ${stage.label} has no public unit replica`);
    const selected = new Set(frame.selection);
    const contents = units.filter((unit) => selected.has(unit.unit_id)).map((unit) => unit.content_id).sort();
    assert.deepEqual(contents, [...stage.expected_content_ids].sort(),
        `SEL ${stage.label} must select exactly the own post and depot static units`);
}

function assertMixedOwnerBox(action, scene, debug) {
    const [x0, y0, x1, y1, button] = action.args;
    assert.equal(button, "left", "SEL mixed-owner box must be a physical left drag");
    const box = { minX: Math.min(x0, x1), maxX: Math.max(x0, x1), minY: Math.min(y0, y1), maxY: Math.max(y0, y1) };
    const localSlot = debug?.slot;
    const publicUnits = new Map((debug?.replicated_world?.units || []).map((unit) => [unit.unit_id, unit]));
    const foreignVisible = (scene?.units || []).filter((unit) => publicUnits.get(unit.unit_id)?.owner_slot !== localSlot &&
        Array.isArray(unit.ground_screen_center) && unit.ground_screen_center.length === 2 &&
        unit.ground_screen_center.every(Number.isFinite) &&
        unit.ground_screen_center[0] >= box.minX && unit.ground_screen_center[0] <= box.maxX &&
        unit.ground_screen_center[1] >= box.minY && unit.ground_screen_center[1] <= box.maxY);
    assert(foreignVisible.length > 0, "SEL mixed-owner drag box must enclose an actual visible foreign unit center");
    assert(foreignVisible.every((unit) => publicUnits.get(unit.unit_id)?.owner_slot !== undefined),
        "SEL mixed-owner box targets must map to actual public replica units");
    for (const unit of foreignVisible) {
        assert.notEqual(publicUnits.get(unit.unit_id)?.owner_slot, localSlot,
            `SEL mixed-owner fixture unit ${unit.unit_id} must be foreign`);
    }
}

function assertModifiers(action, stage) {
    const events = action.actual_events || [];
    const wanted = new Set(stage.modifiers || []);
    for (const event of events.filter((item) => item.type === "pointerdown" || item.type === "pointerup")) {
        if (wanted.has("Shift")) assert(event.shift_key, `SEL ${stage.label} pointer event did not carry physical Shift`);
        if (wanted.has("Control")) assert(event.ctrl_key, `SEL ${stage.label} pointer event did not carry physical Control`);
    }
}

function assertTypeSelectionIsVisible(frame, debug, scene, stage) {
    const localSlot = debug.slot;
    const candidates = (debug.replicated_world?.units || []).filter((unit) => unit.owner_slot === localSlot && unit.content_id === stage.expected_unit_content_id);
    const viewport = scene.viewport_logical_rect;
    assert(Array.isArray(viewport) && viewport.length === 4,
        `SEL ${stage.label} actual scene must provide its viewport for the offscreen precondition`);
    const visible = candidates.filter((unit) => scene.units?.some((entry) => entry.unit_id === unit.unit_id &&
        Array.isArray(entry.ground_screen_center) && entry.ground_screen_center.length === 2 && entry.ground_screen_center.every(Number.isFinite) &&
        entry.ground_screen_center[0] >= viewport[0] && entry.ground_screen_center[0] <= viewport[0] + viewport[2] &&
        entry.ground_screen_center[1] >= viewport[1] && entry.ground_screen_center[1] <= viewport[1] + viewport[3]));
    assert(visible.length > 0, `SEL ${stage.label} has no visible own ${stage.expected_unit_content_id} candidates in actual scene`);
    assert(candidates.length > visible.length,
        `SEL ${stage.label} requires actual same-type own candidates offscreen: total ${candidates.length}, visible ${visible.length}`);
    assert.equal(frame.selection.length, visible.length, `SEL ${stage.label} must select each visible own ${stage.expected_unit_content_id} and no off-screen unit`);
    const selected = new Set(frame.selection);
    for (const unit of visible) assert(selected.has(unit.unit_id), `SEL ${stage.label} omitted visible ${stage.expected_unit_content_id} ${unit.unit_id}`);
}

function hasRingPixels(image, scene, unitId, expected) {
    const entry = scene?.units?.find((unit) => unit.unit_id === unitId);
    const radius = entry?.pick_radius;
    const center = entry?.ground_screen_center;
    const matrix = scene?.clip_from_world;
    const viewport = scene?.viewport_logical_rect;
    if (!Array.isArray(center) || !Number.isFinite(radius) || radius <= 0 || !Array.isArray(matrix) || matrix.length !== 16 || !Array.isArray(viewport)) return false;
    const world = entry.translation;
    if (!Array.isArray(world) || world.length !== 3) return false;
    let matches = 0;
    for (let step = 0; step < 8; step += 1) {
        const angle = Math.PI * 2 * step / 8;
        const point = project(matrix, viewport, [world[0] + Math.cos(angle) * radius, 0.015, world[2] + Math.sin(angle) * radius]);
        if (!point) continue;
        let found = false;
        for (let dy = -2; dy <= 2 && !found; dy += 1) for (let dx = -2; dx <= 2; dx += 1) {
            const pixel = pixelAt(image, Math.round(point[0] + dx), Math.round(point[1] + dy));
            if (pixel && Math.hypot(pixel[0] - expected[0], pixel[1] - expected[1], pixel[2] - expected[2]) <= 90) { found = true; break; }
        }
        if (found) matches += 1;
    }
    return matches >= 3;
}

function project(matrix, viewport, point) {
    const [x, y, z] = point;
    const clip = [
        matrix[0] * x + matrix[4] * y + matrix[8] * z + matrix[12],
        matrix[1] * x + matrix[5] * y + matrix[9] * z + matrix[13],
        matrix[2] * x + matrix[6] * y + matrix[10] * z + matrix[14],
        matrix[3] * x + matrix[7] * y + matrix[11] * z + matrix[15],
    ];
    if (!Number.isFinite(clip[3]) || clip[3] <= 1e-12) return null;
    const ndcX = clip[0] / clip[3];
    const ndcY = clip[1] / clip[3];
    return [viewport[0] + (ndcX + 1) * viewport[2] / 2, viewport[1] + (1 - ndcY) * viewport[3] / 2];
}

module.exports = { judge };
