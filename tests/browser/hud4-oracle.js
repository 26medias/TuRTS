// turts-test-role: oracle
"use strict";

const assert = require("node:assert/strict");

async function judge(view, params) {
    const input = view.inputLog();
    const pixels = JSON.parse(await view.serverFile("hud4-pixel-targets.json"));
    const steps = JSON.parse(await view.serverFile("hud4-screenshot-steps.json"));
    const checkpoints = JSON.parse(await view.serverFile("scene-checkpoints.json"));
    validateFixtureCatalog(params.fixture_catalog);

    assert.equal(params.capture_scene_checkpoints, true, "HUD4 requires post-screenshot scene/debug checkpoints");
    assert(Array.isArray(checkpoints) && checkpoints.length > 0, "HUD4 needs actual scene/debug checkpoints");
    assert(Array.isArray(steps.screenshots), "HUD4 driver did not record screenshot step indices");
    const screenshotRows = input.map((entry, index) => ({ entry, index })).filter(({ entry }) => entry.call === "screenshot");
    assert.equal(screenshotRows.length, steps.screenshots.length, "driver and harness screenshot counts differ");
    const checkpointsByCall = new Map(checkpoints.map((checkpoint) => [checkpoint.input_call, checkpoint]));

    for (const stage of params.stages) {
        const trace = steps.screenshots.find((entry) => entry.label === stage.label);
        assert(trace, `HUD4 driver lacks screenshot stage ${stage.label}`);
        const screenshot = screenshotRows[trace.ordinal];
        assert(screenshot, `HUD4 stage ${stage.label} has no screenshot input call`);
        const checkpoint = checkpointsByCall.get(screenshot.index);
        assert(checkpoint, `HUD4 stage ${stage.label} has no matching C84 checkpoint`);
        const pixel = stage.target === "ground" ? pixels.ground : pixels.targets?.[stage.target];
        assert(pixel && Number.isFinite(pixel.x) && Number.isFinite(pixel.y), `HUD4 stage ${stage.label} has no pixel point`);

        verifyInput(input, stage, pixel);
        verifyStage(checkpoint, stage, pixel, params);
    }
}

function verifyInput(input, stage, pixel) {
    if (stage.label.startsWith("select-")) {
        const click = input.find((entry) => entry.call === "click" && entry.args?.[0] === pixel.x && entry.args?.[1] === pixel.y && entry.args?.[2] === "left");
        assert(click && click.event_seq?.length >= 3, `HUD4 ${stage.label} must use a real left click on its pixel target`);
    } else if (Object.hasOwn(stage, "selection") && stage.selection === null) {
        // The click itself proves the pointer is already over this pixel; an unchanged
        // follow-up move may legitimately emit no DOM event.
    } else {
        const move = input.find((entry) => entry.call === "move" && entry.args?.[0] === pixel.x && entry.args?.[1] === pixel.y);
        assert(move && move.event_seq?.length >= 1, `HUD4 ${stage.label} must hover its target using driver pixel IO`);
    }
    if (Object.hasOwn(stage, "selection") && stage.selection === null) {
        const click = input.find((entry) => entry.call === "click" && entry.args?.[0] === pixel.x && entry.args?.[1] === pixel.y && entry.args?.[2] === "left");
        const events = click?.actual_events || [];
        const down = events.find((event) => event.type === "pointerdown" && event.target === "CANVAS" && event.canvas_target === true);
        const up = events.find((event) => event.type === "pointerup" && event.target === "CANVAS" && event.canvas_target === true);
        assert(down && up && down.seq < up.seq, `HUD4 ${stage.label} must clear selection with an ordered canvas left click at its pixel target`);
    }
    if (stage.mode_key) {
        assert(input.some((entry) => entry.call === "key" && entry.args?.[0] === stage.mode_key),
            `HUD4 ${stage.label} must enter the authored ${stage.mode_key} target mode by physical key`);
    }
}

function verifyStage(checkpoint, stage, pixel, params) {
    const scene = checkpoint.scene;
    const debug = checkpoint.debug;
    assert(scene && debug, `HUD4 ${stage.label} checkpoint has no actual scene/debug data`);
    const catalog = params.fixture_catalog;
    const selectedRole = stage.selection;
    const targetRole = stage.target;

    let targetId;
    if (targetRole === "ground") {
        assertOutsidePickFootprints(scene, pixel, params.pick_world_epsilon, stage.label);
        if (Object.hasOwn(stage, "selection") && stage.selection === null) {
            assertOutsideRenderedWidgets(scene, pixel, stage.label);
        }
    } else if (targetRole === "node") {
        const hit = nearest(scene.nodes, pixel, "node_id", stage.label);
        assertPointInsideFootprint(scene, pixel, hit.entry, params.pick_world_epsilon, `HUD4 ${stage.label} pixel misses the actual rendered node pick footprint`);
        const actualNode = (debug.replicated_world?.nodes || []).find((node) => node.id === hit.entry.node_id);
        assert(actualNode, `HUD4 ${stage.label} node is absent from the actual public replica`);
        assert.equal(actualNode.content_id, catalog.node.content_id, `HUD4 ${stage.label} node content mismatch`);
        targetId = hit.entry.node_id;
    } else {
        const expected = catalog.units[targetRole];
        assert(expected, `HUD4 row has no public unit definition for target role ${targetRole}`);
        const hit = nearest(scene.units, pixel, "unit_id", stage.label);
        assertPointInsideFootprint(scene, pixel, hit.entry, params.pick_world_epsilon, `HUD4 ${stage.label} pixel misses the actual rendered unit pick footprint`);
        const actualUnit = (debug.replicated_world?.units || []).find((unit) => unit.unit_id === hit.entry.unit_id);
        assert(actualUnit, `HUD4 ${stage.label} unit is absent from the actual public replica`);
        assert.equal(actualUnit.content_id, expected.content_id, `HUD4 ${stage.label} content role mismatch`);
        assert.equal(actualUnit.owner_slot, expected.owner_slot, `HUD4 ${stage.label} public owner_slot mismatch`);
        targetId = hit.entry.unit_id;
    }

    if (selectedRole) {
        const selected = debug.selection;
        assert(Array.isArray(selected), `HUD4 ${stage.label} has no actual selection feed`);
        const expected = catalog.units[selectedRole];
        assert(expected, `HUD4 row has no public unit definition for selection role ${selectedRole}`);
        assert.equal(selected.length, 1, `HUD4 ${stage.label} should have exactly one selected unit`);
        const selectedId = selected[0];
        const selectedUnit = (debug.replicated_world?.units || []).find((unit) => unit.unit_id === selectedId);
        assert(selectedUnit, `HUD4 ${stage.label} selected ID is absent from the public replica`);
        assert.equal(selectedUnit.content_id, expected.content_id, `HUD4 ${stage.label} selected content role mismatch`);
        assert.equal(selectedUnit.owner_slot, expected.owner_slot, `HUD4 ${stage.label} selected owner_slot mismatch`);
        if (selectedRole === "attacker") {
            assert.equal(expected.weapon_target_slots, "Other", "HUD4 attacker fixture must publicly target other slots");
        }
        if (selectedRole === "worker") {
            assert(expected.gatherer_items.includes(catalog.node.item), "HUD4 worker fixture must gather the authored node item");
        }
        if (targetRole !== "ground" && targetRole !== "node" && stage.selection === targetRole) {
            assert.equal(selectedId, targetId, `HUD4 ${stage.label} click selection differs from actual hovered target`);
        }
    }
    if (Object.hasOwn(stage, "selection") && stage.selection === null) {
        assert(Array.isArray(debug.selection), `HUD4 ${stage.label} has no actual selection feed`);
        assert.equal(debug.selection.length, 0, `HUD4 ${stage.label} must prove the selection is cleared`);
    }

    if (stage.cursor) {
        const actual = debug.cursor;
        assert(actual && typeof actual === "object", `HUD4 ${stage.label} has no typed cursor feed`);
        assert.equal(actual.context, stage.cursor.context, `HUD4 ${stage.label} cursor context mismatch`);
        assert.equal(actual.image_id, stage.cursor.image_id, `HUD4 ${stage.label} mod cursor image mismatch`);
        assert.equal(actual.applied_icon, stage.cursor.applied_icon, `HUD4 ${stage.label} actual applied CursorIcon mismatch`);
    }

    if (stage.label === "hover-foreign-attack") {
        const actor = catalog.units[stage.selection];
        const target = catalog.units[targetRole];
        assert.equal(actor.weapon_target_slots, "Other");
        assert.notEqual(actor.owner_slot, target.owner_slot, "attack target must be foreign to the selected weapon owner");
    }
    if (stage.label === "hover-node-gather") {
        assert(catalog.units[stage.selection].gatherer_items.includes(catalog.node.item));
    }
    if (stage.label === "attack-mode-invalid-node") {
        assert.equal(catalog.units[stage.selection].weapon_target_slots, "Other");
        assert.equal(stage.target, "node", "the invalid attack point must be an actual node, not a substituted coordinate");
    }
    if (stage.label === "move-mode-ground-point") {
        assert.equal(stage.cursor.context, "target", "explicit point target mode uses the canonical target context");
        assert.equal(stage.cursor.image_id, catalog.move_order_cursor_override, "target context keeps the authored move OrderDef.cursor image");
    }
}

function assertOutsideRenderedWidgets(scene, pixel, label) {
    assert(Array.isArray(scene.widgets), `HUD4 ${label} scene has no rendered authored widget bounds`);
    for (const widget of scene.widgets) {
        const rect = widget?.rect;
        assert(Array.isArray(rect) && rect.length === 4 && rect.every(Number.isFinite) && rect[2] > 0 && rect[3] > 0,
            `HUD4 ${label} rendered widget ${widget?.id} has invalid geometry`);
        assert(!(pixel.x >= rect[0] && pixel.x < rect[0] + rect[2] && pixel.y >= rect[1] && pixel.y < rect[1] + rect[3]),
            `HUD4 ${label} clearing click falls inside rendered widget ${widget.id}`);
    }
}

function validateFixtureCatalog(catalog) {
    assert(catalog && catalog.units && catalog.node && catalog.cursors);
    const attacker = catalog.units.attacker;
    const worker = catalog.units.worker;
    const foreign = catalog.units.foreign;
    assert(attacker && worker && foreign);
    assert.equal(attacker.owner_slot, worker.owner_slot, "attacker and gatherer must be the local slot");
    assert.notEqual(attacker.owner_slot, foreign.owner_slot, "foreign target must have a different public owner slot");
    assert.equal(attacker.weapon_target_slots, "Other", "attacker fixture needs public other-slot weapon capability");
    assert(worker.gatherer_items.includes(catalog.node.item), "worker and public node item must match");
    assert.equal(catalog.cursors.attack, "muster_cursor_attack");
    assert.equal(catalog.cursors.gather, "muster_cursor_gather");
    assert.equal(catalog.cursors.forbidden, "muster_cursor_forbidden");
    assert.equal(catalog.move_order_cursor_override, "muster_cursor_move");
}

function nearest(entries, pixel, idKey, label) {
    assert(Array.isArray(entries) && entries.length > 0, `HUD4 ${label} scene has no rendered targets`);
    const measured = entries.map((entry) => {
        const center = entry?.ground_screen_center;
        assert(Array.isArray(center) && center.length === 2 && center.every(Number.isFinite), `HUD4 ${label} scene target has no finite ground center`);
        return { entry, distance: Math.hypot(center[0] - pixel.x, center[1] - pixel.y) };
    }).sort((left, right) => left.distance - right.distance);
    assert.notEqual(measured[0].entry[idKey], undefined, `HUD4 ${label} scene target has no stable id`);
    if (measured.length > 1) {
        assert(measured[1].distance - measured[0].distance > 1, `HUD4 ${label} pixel target is ambiguous between rendered entities`);
    }
    return measured[0];
}

function assertOutsidePickFootprints(scene, pixel, epsilon, label) {
    const world = unprojectGround(scene, pixel);
    for (const [kind, entries] of [["unit", scene.units], ["node", scene.nodes]]) {
        assert(Array.isArray(entries), `HUD4 ${label} has no rendered ${kind} pick metadata`);
        for (const entry of entries) {
            const { center, radius } = pickFootprint(entry, kind, label);
            assert(Math.hypot(world[0] - center[0], world[1] - center[1]) > radius + epsilon,
                `HUD4 ${label} point falls inside a rendered ${kind} pick footprint, not open ground`);
        }
    }
}

function assertPointInsideFootprint(scene, pixel, entry, epsilon, message) {
    const world = unprojectGround(scene, pixel);
    const kind = Object.hasOwn(entry, "unit_id") ? "unit" : "node";
    const { center, radius } = pickFootprint(entry, kind, "target");
    assert(Math.hypot(world[0] - center[0], world[1] - center[1]) <= radius + epsilon, message);
}

function pickFootprint(entry, kind, label) {
    assert(Number.isFinite(entry?.pick_radius) && entry.pick_radius > 0,
        `HUD4 ${label} rendered ${kind} has a missing or invalid actual pick radius`);
    assert(Array.isArray(entry.translation) && entry.translation.length === 3 && entry.translation.every(Number.isFinite),
        `HUD4 ${label} rendered ${kind} has missing or invalid world translation`);
    return { center: [entry.translation[0], entry.translation[2]], radius: entry.pick_radius };
}

// Invert the captured column-major clip-from-world matrix and intersect the
// cursor ray with world y=0. This makes hit checks independent of the driver's
// candidate classification and of arbitrary CSS-distance tolerances.
function unprojectGround(scene, pixel) {
    const matrix = scene?.clip_from_world;
    const viewport = scene?.viewport_logical_rect;
    assert(Array.isArray(matrix) && matrix.length === 16 && matrix.every(Number.isFinite),
        "HUD4 actual camera clip_from_world matrix is missing or invalid");
    assert(Array.isArray(viewport) && viewport.length === 4 && viewport.every(Number.isFinite) && viewport[2] > 0 && viewport[3] > 0,
        "HUD4 actual logical viewport is missing or invalid");
    assert(scene.coordinate_space === "window-logical-pixels-top-left",
        "HUD4 scene coordinates are not window-logical-pixels-top-left");
    assert(Number.isFinite(pixel?.x) && Number.isFinite(pixel?.y), "HUD4 cursor pixel is invalid");
    const [left, top, width, height] = viewport;
    assert(pixel.x >= left && pixel.y >= top && pixel.x < left + width && pixel.y < top + height,
        "HUD4 cursor pixel falls outside the actual camera viewport");
    const ndc = [2 * (pixel.x - left) / width - 1, 1 - 2 * (pixel.y - top) / height];
    const inverse = invert4(matrix);
    // Bevy's reverse-Z projection has an infinite far plane; use two finite
    // depths from its core screen-to-ground projection (1.0 and 0.0001).
    const near = transform4(inverse, [ndc[0], ndc[1], 1, 1]);
    const far = transform4(inverse, [ndc[0], ndc[1], 0.0001, 1]);
    assert(Math.abs(near[3]) > 1e-12 && Math.abs(far[3]) > 1e-12,
        "HUD4 actual camera unprojection has invalid homogeneous coordinates");
    const start = near.slice(0, 3).map((value) => value / near[3]);
    const end = far.slice(0, 3).map((value) => value / far[3]);
    const dy = end[1] - start[1];
    assert(Number.isFinite(dy) && Math.abs(dy) > 1e-12,
        "HUD4 cursor ray is parallel to or invalid at the ground plane");
    const distance = -start[1] / dy;
    assert(Number.isFinite(distance) && distance >= 0,
        "HUD4 cursor ray does not intersect the ground plane in front of the camera");
    const x = start[0] + distance * (end[0] - start[0]);
    const z = start[2] + distance * (end[2] - start[2]);
    assert(Number.isFinite(x) && Number.isFinite(z), "HUD4 ground unprojection is non-finite");
    return [x, z];
}

function invert4(columnMajor) {
    const rows = Array.from({ length: 4 }, (_, row) => [
        ...Array.from({ length: 4 }, (_, col) => columnMajor[col * 4 + row]),
        ...Array.from({ length: 4 }, (_, col) => row === col ? 1 : 0),
    ]);
    for (let col = 0; col < 4; col += 1) {
        let pivot = col;
        for (let row = col + 1; row < 4; row += 1) {
            if (Math.abs(rows[row][col]) > Math.abs(rows[pivot][col])) pivot = row;
        }
        assert(Number.isFinite(rows[pivot][col]) && Math.abs(rows[pivot][col]) > 1e-12,
            "HUD4 actual camera matrix is singular");
        [rows[col], rows[pivot]] = [rows[pivot], rows[col]];
        const scale = rows[col][col];
        for (let entry = 0; entry < 8; entry += 1) rows[col][entry] /= scale;
        for (let row = 0; row < 4; row += 1) {
            if (row === col) continue;
            const factor = rows[row][col];
            for (let entry = 0; entry < 8; entry += 1) rows[row][entry] -= factor * rows[col][entry];
        }
    }
    return Array.from({ length: 16 }, (_, index) => rows[index % 4][4 + Math.floor(index / 4)]);
}

function transform4(matrix, vector) {
    return Array.from({ length: 4 }, (_, row) =>
        matrix[row] * vector[0] + matrix[4 + row] * vector[1] + matrix[8 + row] * vector[2] + matrix[12 + row] * vector[3]);
}

module.exports = { judge, validateFixtureCatalog, verifyInput, verifyStage, unprojectGround, assertOutsideRenderedWidgets };
