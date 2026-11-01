// turts-test-role: oracle
"use strict";

const assert = require("node:assert/strict");

function maxMatrixDelta(left, right) {
    assert(Array.isArray(left) && left.length === 16, "CAM1 checkpoint is missing its actual clip_from_world matrix");
    assert(Array.isArray(right) && right.length === 16, "CAM1 checkpoint is missing its actual clip_from_world matrix");
    assert([...left, ...right].every(Number.isFinite), "CAM1 camera matrix must contain finite values");
    return Math.max(...left.map((value, index) => Math.abs(value - right[index])));
}

async function judge(view) {
    const input = view.inputLog();
    const checkpoints = JSON.parse(await view.serverFile("scene-checkpoints.json"));
    assert(Array.isArray(checkpoints), "CAM1 needs read-only scene checkpoints for each marked camera action");

    function checkpointFor(name) {
        const saveIndex = input.findIndex((entry) => entry.call === "save" && entry.args?.[0] === `${name}.png`);
        assert(saveIndex > 0, `CAM1 driver must save the ${name} screenshot after a real input stage`);
        let screenshotIndex = saveIndex - 1;
        while (screenshotIndex >= 0 && input[screenshotIndex].call !== "screenshot") screenshotIndex -= 1;
        assert(screenshotIndex >= 0, `CAM1 has no screenshot checkpoint for ${name}`);
        const checkpoint = checkpoints.find((entry) => entry.input_call === screenshotIndex && entry.window === 1);
        assert(checkpoint, `CAM1 has no captured scene for ${name}`);
        assert(checkpoint.scene?.cameras_3d === 1, `CAM1 ${name} must read the one actual active 3D camera`);
        assert(checkpoint.scene?.clip_from_world, `CAM1 ${name} is missing the actual clip_from_world matrix`);
        return checkpoint;
    }

    const initial = checkpointFor("00-camera-start");
    const selected = checkpointFor("01-scout-selected");
    const arrow = checkpointFor("02-arrow-right");
    const edge = checkpointFor("03-right-edge-band");
    const zoomA = checkpointFor("04-zoom-limit-a");
    const zoomARepeat = checkpointFor("05-zoom-limit-a-repeat");
    const zoomB = checkpointFor("06-zoom-limit-b");
    const zoomBRepeat = checkpointFor("07-zoom-limit-b-repeat");
    const centred = checkpointFor("08-space-centred");
    const topEdge = checkpointFor("09-top-edge-band");
    const pointerLeftShort = checkpointFor("10-after-pointer-left-short");
    const pointerLeftLong = checkpointFor("11-after-pointer-left-long");

    const arrowInput = input.find((entry) => entry.call === "key" && entry.args?.[0] === "ArrowRight" && entry.args?.[2]?.hold_ms === 1000);
    assert(arrowInput, "CAM1 must hold the real ArrowRight key for one second");
    assert(selected.debug?.selection?.length === 1, "the screenshot-sampled left click must select one rendered unit before camera movement");
    assert(maxMatrixDelta(selected.scene.clip_from_world, arrow.scene.clip_from_world) > 1e-5, "camera did not move during ArrowRight");
    const edgeInput = input.find((entry) => entry.call === "move" && entry.args?.[0] >= 1272 && entry.args?.[0] < 1280 && entry.args?.[1] === 360);
    assert(edgeInput, "CAM1 must move the real pointer into the rightmost 8 logical pixels");
    assert(maxMatrixDelta(arrow.scene.clip_from_world, edge.scene.clip_from_world) > 1e-5, "camera did not move during edge-band pan");

    assert(maxMatrixDelta(zoomA.scene.clip_from_world, zoomARepeat.scene.clip_from_world) < 1e-4, "wheel input past one zoom limit must remain clamped");
    assert(maxMatrixDelta(zoomB.scene.clip_from_world, zoomBRepeat.scene.clip_from_world) < 1e-4, "wheel input past the other zoom limit must remain clamped");
    assert(maxMatrixDelta(zoomA.scene.clip_from_world, zoomB.scene.clip_from_world) > 1e-5, "wheel input must zoom between distinct camera heights");

    const click = input.find((entry) => entry.call === "click" && entry.args?.[2] === "left");
    assert(click, "CAM1 must select the rendered scout with a real left click");
    const selectedIds = selected.debug?.selection;
    assert(Array.isArray(selectedIds) && selectedIds.length === 1, "pixel-sampled scout click must produce a single-unit selection");
    const selectedId = selectedIds[0];
    assert(centred.debug?.selection?.includes(selectedId), "Space must preserve the selected unit while centring the camera");
    assert(maxMatrixDelta(zoomBRepeat.scene.clip_from_world, centred.scene.clip_from_world) > 1e-5,
        "Space must change the actual camera matrix to centre the selected unit");
    const centredUnit = centred.scene.units?.find((unit) => unit.unit_id === selectedId);
    assert(Array.isArray(centredUnit?.ground_screen_center), "CAM1 must use the actual selected unit's rendered ground centre");
    const viewport = centred.scene.viewport_logical_rect;
    assert(Array.isArray(viewport) && viewport.length === 4, "CAM1 scene is missing its logical camera viewport");
    const centreX = viewport[0] + viewport[2] / 2;
    const centreY = viewport[1] + viewport[3] / 2;
    const unitCenter = centredUnit.ground_screen_center;
    assert(Math.hypot(unitCenter[0] - centreX, unitCenter[1] - centreY) <= 2.0, "Space did not centre the camera on the selected rendered unit");

    assert(maxMatrixDelta(centred.scene.clip_from_world, topEdge.scene.clip_from_world) > 1e-5, "top edge band must pan before the pointer leaves the canvas");
    assert(maxMatrixDelta(pointerLeftShort.scene.clip_from_world, pointerLeftLong.scene.clip_from_world) < 1e-4, "PointerLeft must stop edge panning after the pointer leaves the canvas");
    const leftInput = input.find((entry) => entry.call === "move" && entry.args?.[0] === 640 && entry.args?.[1] === -1);
    assert(leftInput, "CAM1 must move the real pointer out through the top of the canvas");
}

module.exports = { judge };
