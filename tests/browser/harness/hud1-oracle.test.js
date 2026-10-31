// turts-test-role: unit
"use strict";

const assert = require("node:assert/strict");
const test = require("node:test");
const { judge } = require("../hud-oracle.js");

function params(width, height, dpr = 1) {
    return {
        device_scale_factor: dpr,
        viewport: { width, height, device_scale_factor: dpr },
    };
}

function scene(width, height, widgets, dpr = 1) {
    return {
        cameras_3d: 1,
        coordinate_space: "window-logical-pixels-top-left",
        window_logical_size: [width, height],
        viewport_logical_rect: [0, 0, width, height],
        scale_factor: dpr,
        widgets,
    };
}

function widgets(width, height) {
    return [
        { id: "card", rect: [8, 8, 272, 204] },
        { id: "minimap", rect: [width - 208, height - 208, 200, 200] },
        { id: "ore", rect: [(width - 160) / 2, height - 32, 160, 28] },
    ];
}

function fakeView({ width = 1280, height = 720, dpr = 1, rendered = widgets(width, height),
    legacyUi = [], selection = [7], click = true } = {}) {
    const currentScene = scene(width, height, rendered, dpr);
    return {
        async bridge(name) {
            if (name === "turts_debug") return { selection };
            if (name === "turts_scene") return currentScene;
            if (name === "turts_ui") return legacyUi;
            throw new Error(`unexpected bridge ${name}`);
        },
        inputLog() {
            return click ? [{ call: "click", args: [400, 300, "left"], event_seq: [1, 2, 3] }] : [];
        },
    };
}

for (const [width, height, dpr] of [[1280, 720, 1], [1920, 1080, 1], [1280, 720, 2]]) {
    test(`authored logical widget rectangles pass at ${width}x${height}@${dpr}`, async () => {
        await judge(fakeView({ width, height, dpr }), params(width, height, dpr));
    });
}

test("missing card reports the semantic failure and a visible engine panel", async () => {
    const rendered = widgets(1280, 720).filter((widget) => widget.id !== "card");
    const view = fakeView({ rendered, legacyUi: [{ kind: "panel", visible: true }] });
    await assert.rejects(judge(view, params(1280, 720)), (error) => {
        assert.match(error.message, /widget card missing/);
        assert.match(error.message, /engine panel present/);
        return true;
    });
});

test("placement beyond one logical pixel fails", async () => {
    const rendered = widgets(1280, 720);
    rendered[0] = { id: "card", rect: [10, 8, 272, 204] };
    await assert.rejects(judge(fakeView({ rendered }), params(1280, 720)), /differs from authored logical rect/);
});

test("DPR-two physical rectangles fail instead of receiving a second conversion", async () => {
    const rendered = widgets(1280, 720).map(({ id, rect }) => ({ id, rect: rect.map((value) => value * 2) }));
    await assert.rejects(judge(fakeView({ rendered, dpr: 2 }), params(1280, 720, 2)), /differs from authored logical rect/);
});

test("non-finite and zero-size rectangles fail", async () => {
    const rendered = widgets(1280, 720);
    rendered[0] = { id: "card", rect: [8, 8, Number.NaN, 204] };
    rendered[1] = { id: "minimap", rect: [1072, 512, 0, 200] };
    await assert.rejects(judge(fakeView({ rendered }), params(1280, 720)), /non-finite, zero-size, or invalid rect/);
});

test("duplicate rendered widget ids fail", async () => {
    const rendered = widgets(1280, 720);
    rendered.push({ id: "card", rect: [8, 8, 272, 204] });
    await assert.rejects(judge(fakeView({ rendered }), params(1280, 720)), /duplicate rendered widget id: card/);
});

test("duplicate nested rendered widget ids also fail", async () => {
    const rendered = widgets(1280, 720);
    rendered.push({ id: "cell", rect: [20, 20, 10, 10] }, { id: "cell", rect: [30, 20, 10, 10] });
    await assert.rejects(judge(fakeView({ rendered }), params(1280, 720)), /duplicate rendered widget id: cell/);
});

test("every rendered widget requires an id and a positive finite rectangle", async () => {
    const rendered = widgets(1280, 720);
    rendered.push({ id: "", rect: [0, 0, 10, 10] }, { id: "child", rect: [0, 0, Infinity, 2] });
    await assert.rejects(judge(fakeView({ rendered }), params(1280, 720)), /missing or invalid id.*non-finite, zero-size, or invalid rect/);
});

test("selection must be real and nonempty before placement is judged", async () => {
    await assert.rejects(judge(fakeView({ selection: [] }), params(1280, 720)), /actual debug selection must be nonempty/);
});

test("an oracle row without a harness selection click fails", async () => {
    await assert.rejects(judge(fakeView({ click: false }), params(1280, 720)), /actual harness left click/);
});

test("DPR-two rows reject a DPR-one window even when logical rectangles look right", async () => {
    await assert.rejects(judge(fakeView({ dpr: 1 }), params(1280, 720, 2)), /scale factor/);
});
