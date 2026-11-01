// turts-test-role: oracle
"use strict";

const assert = require("node:assert/strict");
const { decodePng, pixelAt } = require("./harness/png.js");

const SLOT_RGB = [[0, 114, 178], [230, 159, 0], [0, 158, 115], [213, 94, 0], [86, 180, 233], [204, 121, 167], [240, 228, 66], [0, 0, 0]];

async function judge(view, params) {
    const input = view.inputLog();
    const debug = await view.bridge("turts_debug");
    const scene = await view.bridge("turts_scene");
    const frames = debug?.frames?.retained_input;
    assert(Array.isArray(frames), "SEL needs actual C84 input-correlated retained frames");
    for (const stage of params.stages || []) {
        const action = input.find((entry) => entry.call === (stage.gesture === "click" || stage.gesture === "double-click" ? "click" : "drag") &&
            entry.args?.[0] === stage.start.x && entry.args?.[1] === stage.start.y &&
            (entry.call !== "drag" || (entry.args?.[2] === stage.end?.x && entry.args?.[3] === stage.end?.y)));
        assert(action, `SEL ${stage.label} must be a physical harness ${stage.gesture}`);
        const mouseUpSeq = action.actual_events?.find((event) => event.type === "pointerup")?.seq;
        assert(Number.isInteger(mouseUpSeq), `SEL ${stage.label} has no actual canvas mouse-up sequence`);
        const frame = frames.find((record) => record.applied_seq?.includes(mouseUpSeq));
        assert(frame, `SEL ${stage.label} has no first presented frame for mouse-up sequence ${mouseUpSeq}`);
        assert.equal(frame.selection?.length, stage.expected_selection,
            `SEL ${stage.label} first mouse-up frame selection count`);
        assert.equal(frame.rings?.length, stage.expected_rings,
            `SEL ${stage.label} first mouse-up frame actual rendered ring count`);
        assertFrameSelectionIsOwn(frame, debug, stage);
        if (stage.expected_rings > 0) {
            const encoded = await view.serverFile(`${stage.label}.png.b64`);
            const image = decodePng(Buffer.from(encoded, "base64"));
            const visible = frame.selection.filter((id) => hasRingPixels(image, scene, id, SLOT_RGB[stage.owner_slot ?? 0]));
            assert.equal(visible.length, stage.expected_rings,
                `SEL ${stage.label} must show slot-coloured ring pixels for every selected unit in its saved screenshot`);
        }
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
