// turts-test-role: unit
"use strict";
const assert = require("node:assert/strict");
const test = require("node:test");
const { projectWorldPoint, screenToWorldPoint } = require("./project.js");

test("HD1 pure camera projection round-trips world points in its authored viewport", () => {
    const camera = { lookAt: [28.85, 20.2], height: 38, pitch: 0.7, yaw: 0, fov: Math.PI / 3 };
    const viewport = { width: 1280, height: 720 };
    for (const world of [[28.85, 20.2], [32, 22], [24, 22]]) {
        const screen = projectWorldPoint(world, camera, viewport);
        assert.ok(screen, `world point ${world} projects into the viewport`);
        const recovered = screenToWorldPoint(screen, camera, viewport);
        assert.ok(Math.abs(recovered[0] - world[0]) < 1e-5);
        assert.ok(Math.abs(recovered[1] - world[1]) < 1e-5);
    }
});
