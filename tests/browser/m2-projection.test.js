"use strict";

const assert = require("node:assert/strict");
const { test } = require("node:test");
const { projectGroundPoint, projectRenderedEntity, uiActionWindowPoint } = require("./m2.js");

const identityScene = {
    coordinate_space: "window-logical-pixels-top-left",
    window_logical_size: [1280, 720],
    viewport_logical_rect: [100, 40, 640, 360],
    clip_from_world: [
        1, 0, 0, 0,
        0, 1, 0, 0,
        0, 0, 1, 0,
        0, 0, 0, 1,
    ],
};

test("projects ground points through camera matrix into the target logical viewport", () => {
    assert.deepEqual(projectGroundPoint(identityScene, [0, 0.5]), [420, 220]);
    assert.equal(projectGroundPoint(identityScene, [2, 0.5]), null);
});

test("cross-checks actual mesh ground centers and requires the catalog pick circle", () => {
    const entry = {
        translation: [0, 0.9, 0.5],
        ground_screen_center: [420, 220],
        pick_radius: 0.5,
    };
    assert.deepEqual(projectRenderedEntity(identityScene, entry), [420, 220]);
    assert.throws(
        () => projectRenderedEntity(identityScene, { ...entry, pick_radius: null }),
        /declared pick radius/,
    );
});

test("rejects projection without camera metadata or outside the near and far planes", () => {
    assert.equal(projectGroundPoint({ ...identityScene, clip_from_world: null }, [0, 0.5]), null);
    assert.equal(projectGroundPoint(identityScene, [0, -0.1]), null);
    assert.equal(projectGroundPoint(identityScene, [0, 1.1]), null);
});

test("maps fully visible UI action rectangles to window-logical pointer coordinates", () => {
    const scene = {
        viewport_physical_rect: [200, 100, 1280, 720],
        scale_factor: 2,
    };
    assert.deepEqual(uiActionWindowPoint(scene, [24, 24, 100, 20]), [137, 67]);
    assert.equal(uiActionWindowPoint(scene, [-1, 24, 100, 20]), null);
    assert.equal(uiActionWindowPoint(scene, [24, 24, 1300, 20]), null);
});
