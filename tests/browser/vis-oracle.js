// turts-test-role: oracle
"use strict";

const assert = require("node:assert/strict");
const { decodePng, pixelAt } = require("./harness/png.js");

async function judge(view) {
    const input = view.inputLog();
    assert(input.some((entry) => entry.call === "key" && entry.args?.[0] === "F1"), "VIS1 must close the guide through physical input");
    const saveIndex = input.findIndex((entry) => entry.call === "save" && entry.args?.[0] === "vis1-slots.png");
    assert(saveIndex > 0, "VIS1 must save its actual screenshot pixels");
    let screenshotIndex = saveIndex - 1;
    while (screenshotIndex >= 0 && input[screenshotIndex].call !== "screenshot") screenshotIndex -= 1;
    assert(screenshotIndex >= 0, "VIS1 has no screenshot input checkpoint");
    const checkpoints = JSON.parse(await view.serverFile("scene-checkpoints.json"));
    assert(Array.isArray(checkpoints), "VIS1 needs actual C84 scene checkpoints");
    const checkpoint = checkpoints.find((entry) => entry.input_call === screenshotIndex && entry.window === 1);
    assert(checkpoint?.scene && checkpoint?.debug, "VIS1 screenshot has no actual C84 scene/debug checkpoint");
    const scene = checkpoint.scene;
    const debug = checkpoint.debug;
    const publicUnits = debug?.replicated_world?.units;
    assert(Array.isArray(publicUnits), "VIS1 has no actual public unit ownership snapshot");
    const ownerById = new Map(publicUnits.map((unit) => [unit.unit_id, unit.owner_slot]));
    const image = decodePng(Buffer.from(await view.serverFile("vis1-slots.png.b64"), "base64"));
    const units = scene?.units;
    assert(Array.isArray(units), "VIS1 needs actual rendered unit records");
    const slots = [...new Set(units.map((unit) => ownerById.get(unit.unit_id)).filter(Number.isInteger))];
    assert(slots.length >= 2, "VIS1 fixture must show actual rendered units from at least two owner slots");

    const bySlot = slots.map((slot) => {
        const samples = units.filter((unit) => ownerById.get(unit.unit_id) === slot)
            .flatMap((unit) => sampleUnitPixels(image, unit, scene.widgets || []));
        assert(samples.length >= 3, `VIS1 slot ${slot} has too few visible unit-body pixels in the actual screenshot`);
        return { slot, samples };
    });
    const hues = bySlot.map(({ samples }) => medianHue(samples));
    if (hues.every((hue) => circularHueDistance(hue, hues[0]) <= 25)) {
        throw new assert.AssertionError({
            message: `VIS1: one colour for all units (slot hues ${hues.map((hue) => hue.toFixed(1)).join(", ")})`,
        });
    }
    const signatures = bySlot.map(({ slot, samples }) => {
        const expected = SLOT_RGB[slot];
        assert(expected, `VIS1 has no authored palette colour for actual owner slot ${slot}`);
        const matches = samples.filter((pixel) => distance(pixel, expected) <= 70);
        assert(matches.length >= 5,
            `VIS1 slot ${slot} has ${matches.length} pixels near its authored palette colour in actual unit footprints`);
        return dominantColor(matches);
    });
    assert(new Set(signatures).size >= 2, `VIS1: one colour for all units (${signatures.join(", ")})`);
}

const SLOT_RGB = [[0, 114, 178], [230, 159, 0], [0, 158, 115], [213, 94, 0], [86, 180, 233], [204, 121, 167], [240, 228, 66], [0, 0, 0]];

function sampleUnitPixels(image, unit, widgets) {
    const center = unit?.ground_screen_center;
    if (!Array.isArray(center) || center.length !== 2 || !center.every(Number.isFinite)) return [];
    const samples = [];
    // Sample visible body pixels above the actual ground center. Avoid HUD, map and labels by
    // requiring a saturated slot-colour candidate and taking only the nearest unit footprint.
    for (let y = Math.round(center[1] - 28); y <= Math.round(center[1] - 4); y += 2) {
        for (let x = Math.round(center[0] - 12); x <= Math.round(center[0] + 12); x += 2) {
            if (widgets.some((widget) => {
                const [left, top, width, height] = widget.rect || [];
                return [left, top, width, height].every(Number.isFinite) &&
                    x >= left && x <= left + width && y >= top && y <= top + height;
            })) continue;
            const pixel = pixelAt(image, x, y);
            if (pixel && pixel[3] >= 250 && Math.max(...pixel.slice(0, 3)) - Math.min(...pixel.slice(0, 3)) >= 35) samples.push(pixel.slice(0, 3));
        }
    }
    return samples;
}

function medianHue(samples) {
    const hues = samples.map(([red, green, blue]) => {
        const max = Math.max(red, green, blue);
        const min = Math.min(red, green, blue);
        const delta = max - min;
        if (delta === 0) return 0;
        if (max === red) return 60 * (((green - blue) / delta) % 6);
        if (max === green) return 60 * ((blue - red) / delta + 2);
        return 60 * ((red - green) / delta + 4);
    });
    const anchor = hues[0];
    const relative = hues.map((hue) => ((hue - anchor + 540) % 360) - 180).sort((a, b) => a - b);
    return (anchor + relative[Math.floor(relative.length / 2)] + 360) % 360;
}

function circularHueDistance(left, right) {
    return Math.abs(((left - right + 540) % 360) - 180);
}

function distance(left, right) {
    return Math.hypot(left[0] - right[0], left[1] - right[1], left[2] - right[2]);
}

function dominantColor(samples) {
    const counts = new Map();
    for (const [r, g, b] of samples) {
        const key = `${Math.round(r / 16) * 16},${Math.round(g / 16) * 16},${Math.round(b / 16) * 16}`;
        counts.set(key, (counts.get(key) || 0) + 1);
    }
    return [...counts].sort((left, right) => right[1] - left[1])[0]?.[0] || "none";
}

module.exports = { judge };
