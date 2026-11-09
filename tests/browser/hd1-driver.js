// turts-test-role: driver
"use strict";

const { projectWorldPoint } = require("./harness/project.js");

async function drive(io, params) {
    if (!Array.isArray(params.actions) || params.actions.length === 0) throw new Error("HD1 row has no validated fixture actions");
    const viewport = params.viewport;
    const camera = { lookAt: [28.85, 20.2], height: 38, pitch: 0.7, yaw: 0, fov: Math.PI / 3 };
    let previousTick = 0;
    for (const action of params.actions) {
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

function project(world, camera, viewport) {
    const point = projectWorldPoint(world, camera, viewport);
    if (!point) throw new Error("HD1 world point " + JSON.stringify(world) + " is outside its projected viewport");
    return point.map(Math.round);
}

module.exports = { drive };
