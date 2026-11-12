// turts-test-role: driver
"use strict";

const { projectWorldPoint } = require("./harness/project.js");
const { decodePng } = require("./harness/png.js");
const { countCellColor, expectedButtonColor } = require("./harness/hd1-ui-pixels.js");

async function drive(io, params) {
    if (!Array.isArray(params.actions) || params.actions.length === 0) throw new Error("HD1 row has no validated fixture actions");
    const viewport = params.viewport;
    const camera = { lookAt: [28.85, 20.2], height: 38, pitch: 0.7, yaw: 0, fov: Math.PI / 3 };
    let previousTick = 0;
    // The first join shows the "Click to play" overlay until a gesture (client_shell.rs
    // sync_focus_overlay, since 3d0cf2f); it takes the first pointer input, here the box select.
    // A keydown is also the gesture (crates/turts_client_web/index.html:39-40); F9 has no binding
    // (RUN130).
    await io.key("F9");
    await io.wait(300);
    for (let index = 0; index < params.actions.length; index += 1) {
        const action = params.actions[index];
        await io.wait(Math.max(0, action.tick - previousTick) * 50 + 200);
        previousTick = action.tick;
        if (action.do === "box_select") {
            const start = project(action.world_rect.slice(0, 2), camera, viewport);
            const end = project(action.world_rect.slice(2, 4), camera, viewport);
            await io.drag(start[0], start[1], end[0], end[1], "left");
        } else if (action.do === "click_select") {
            const point = project(action.world_point, camera, viewport);
            await io.click(point[0], point[1], "left");
        } else if (action.do === "smart_command") {
            const point = project(action.world_point, camera, viewport);
            if (action.queue) await io.key("ShiftLeft", [], { action: "down" });
            await io.click(point[0], point[1], "right");
            if (action.queue) await io.key("ShiftLeft", [], { action: "up" });
        } else if (action.do === "group_assign") {
            await io.key("Digit" + action.group, ["Control"]);
        } else if (action.do === "group_recall") {
            await io.key("Digit" + action.group);
        } else if (action.do === "order") {
            await verifyUiCell(io, action, index, viewport);
            if (action.queue) await io.key("ShiftLeft", [], { action: "down" });
            const ui = action.ui_point;
            if (!ui) throw new Error("HD1 " + action.order + " has no fixture-authored widget pixel");
            await io.click(ui[0], ui[1], "left");
            if (action.world_point) {
                const point = project(action.world_point, camera, viewport);
                await io.click(point[0], point[1], "left");
            }
            if (action.queue) await io.key("ShiftLeft", [], { action: "up" });
        } else {
            throw new Error("HD1 unsupported action " + action.do);
        }
    }
    const finalPixels = await io.screenshot();
    await io.save("hd1-final.png", finalPixels);
}

async function verifyUiCell(io, action, index, viewport) {
    if (!Array.isArray(action.ui_point) || action.ui_point.length !== 2) {
        throw new Error(`HD1 ${action.order} has no fixture-authored widget pixel`);
    }
    const png = Buffer.from(await io.screenshot());
    const filename = `hd1-preclick-${String(index + 1).padStart(2, "0")}.png`;
    await io.save(filename, png);
    const image = decodePng(png);
    if (image.width !== viewport.width || image.height !== viewport.height) {
        throw new Error(`HD1 pre-click screenshot is ${image.width}x${image.height}, expected ${viewport.width}x${viewport.height}`);
    }
    const cellSize = Number.isInteger(action.job_id) ? 32 : 64;
    const expected = expectedButtonColor(action);
    const observation = countCellColor(image, action.ui_point, cellSize, expected);
    console.log("HD1_UI_PRECLICK", JSON.stringify({
        action_index: index + 1,
        order: action.order,
        ui_point: action.ui_point,
        cell_size: cellSize,
        expected_rgb: expected,
        region: observation.region,
        matching_pixels: observation.pixels,
        required_pixels: observation.required_pixels,
        screenshot: filename,
    }));
    if (observation.pixels < observation.required_pixels) {
        throw new Error(`HD1 pre-click ${filename} has ${observation.pixels} expected palette pixels for ${action.order}; needs ${observation.required_pixels}`);
    }
}

function project(world, camera, viewport) {
    const point = projectWorldPoint(world, camera, viewport);
    if (!point) throw new Error("HD1 world point " + JSON.stringify(world) + " is outside its projected viewport");
    return point.map(Math.round);
}

module.exports = { drive };
