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

// Actual camera matrix from CI run 43734968635, base final_change fixture.
// Ground-centre (502,191) hit the nearer blocker; the owner's mesh centre is higher.
test("projects an authored mesh height independently of its occluded ground centre", () => {
    const camera = { lookAt: [41, 32], height: 40, pitch: 0.7, yaw: 0, fov: Math.PI / 3 };
    const viewport = { width: 1280, height: 720 };
    const matrix = [0.9742785692214966,0,0,0,0,1.1158181428909302,0,-0.7648422122001648,0,-1.324745535850525,0,-0.6442179083824158,-39.94542694091797,42.391845703125,0.10000000149011612,72.91334533691406];
    const world = [26, 1, 8, 1];
    const clip = [0, 1, 2, 3].map(row => world.reduce((sum, value, column) => sum + matrix[column * 4 + row] * value, 0));
    const expected = [(clip[0] / clip[3] + 1) * 640, (1 - clip[1] / clip[3]) * 360];
    const actual = projectWorldPoint([26, 8], camera, viewport, 1);
    assert.ok(Math.hypot(actual[0] - expected[0], actual[1] - expected[1]) < 0.001);
    assert.equal(projectWorldPoint([26, 8], camera, viewport, NaN), null);
});
