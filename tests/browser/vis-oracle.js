// turts-test-role: oracle
"use strict";

const assert = require("node:assert/strict");
const { decodePng, pixelAt } = require("./harness/png.js");

async function judge(view) {
    const scene = await view.bridge("turts_scene");
    const input = view.inputLog();
    assert(input.some((entry) => entry.call === "key" && entry.args?.[0] === "F1"), "VIS1 must close the guide through physical input");
    const image = decodePng(await view.screenshot());
    const units = scene?.units;
    assert(Array.isArray(units), "VIS1 needs actual rendered unit records");
    const slots = [...new Set(units.map((unit) => unit.owner_slot).filter(Number.isInteger))];
    assert(slots.length >= 2, "VIS1 fixture must show actual rendered units from at least two owner slots");

    const bySlot = slots.map((slot) => {
        const samples = units.filter((unit) => unit.owner_slot === slot).flatMap((unit) => sampleUnitPixels(image, unit));
        assert(samples.length >= 3, `VIS1 slot ${slot} has too few visible unit-body pixels in the actual screenshot`);
        return { slot, samples };
    });
    const signatures = bySlot.map(({ samples }) => dominantColor(samples));
    assert(new Set(signatures).size >= 2, `VIS1: one colour for all units (${signatures.join(", ")})`);
}

function sampleUnitPixels(image, unit) {
    const center = unit?.ground_screen_center;
    if (!Array.isArray(center) || center.length !== 2 || !center.every(Number.isFinite)) return [];
    const samples = [];
    // Sample visible body pixels above the actual ground center. Avoid HUD, map and labels by
    // requiring a saturated slot-colour candidate and taking only the nearest unit footprint.
    for (let y = Math.round(center[1] - 28); y <= Math.round(center[1] - 4); y += 2) {
        for (let x = Math.round(center[0] - 12); x <= Math.round(center[0] + 12); x += 2) {
            const pixel = pixelAt(image, x, y);
            if (pixel && pixel[3] >= 250 && Math.max(...pixel.slice(0, 3)) - Math.min(...pixel.slice(0, 3)) >= 35) samples.push(pixel.slice(0, 3));
        }
    }
    return samples;
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
