// turts-test-role: unit
"use strict";

const assert = require("node:assert/strict");
const test = require("node:test");
const { pressRenderedEntityUntil } = require("../m2.js");

test("rendered entity press completes mouseup before waiting for the predicate", async () => {
    const events = [];
    let released = false;
    const page = {
        locator(selector) {
            assert.equal(selector, "canvas");
            return { first: () => ({ boundingBox: async () => ({ x: 0, y: 0, width: 1280, height: 720 }) }) };
        },
        mouse: {
            move: async (x, y) => events.push(["move", x, y]),
            down: async ({ button }) => events.push(["down", button]),
            up: async ({ button }) => {
                events.push(["up", button]);
                released = true;
            },
        },
        evaluate: async () => {
            events.push(["snapshot", released]);
            return {
                scene_raw: "{}",
                ui_raw: "[]",
                debug_raw: JSON.stringify({ released }),
                ingest_config_status_raw: "null",
                error: null,
            };
        },
    };
    const scene = {
        clip_from_world: [
            1, 0, 0, 0,
            0, 1, 0, 0,
            0, 0, 1, 0,
            0, 0, 0.5, 1,
        ],
        viewport_logical_rect: [0, 0, 1280, 720],
        window_logical_size: [1280, 720],
        coordinate_space: "window-logical-pixels-top-left",
    };
    const entry = {
        translation: [0, 0, 0],
        ground_screen_center: [640, 360],
        pick_radius: 1,
    };

    const snapshot = await pressRenderedEntityUntil(
        page,
        scene,
        entry,
        (value) => value.debug.released,
        1,
        "mouseup feedback",
    );

    assert.equal(snapshot.debug.released, true);
    assert.deepEqual(events.map(([kind]) => kind), ["move", "down", "up", "snapshot"]);
});
