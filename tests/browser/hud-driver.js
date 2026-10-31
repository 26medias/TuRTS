// turts-test-role: driver
"use strict";

const { decodePng } = require("./harness/png.js");
const { findScout } = require("./harness/click9-pixels.js");

async function drive(io, params) {
    // Muster starts with its guide open. F1 is the real, physical guide toggle.
    await io.key("F1");

    let candidate = null;
    let before = null;
    for (let attempt = 0; attempt < 80; attempt += 1) {
        before = await io.screenshot();
        candidate = findScout(decodePng(before));
        if (candidate) break;
        if (attempt < 79) await io.wait(100);
    }
    if (!candidate) {
        throw new Error("HUD1 could not identify one unit candidate from screenshot pixels; Muster recognition is NOT TRACED");
    }

    await io.save("hud1-before-selection.png", before);
    await io.save("hud1-pixel-target.json", Buffer.from(JSON.stringify({
        pixel: { x: candidate.x, y: candidate.y },
        bounds: candidate.bounds,
        coloured_pixels: candidate.pixels,
        viewport: params.viewport,
        method: "existing screenshot-pixel unit-colour component helper",
        recognition_status: "Muster recognition is NOT TRACED until an actual HUD1 browser row runs",
    }) + "\n"));

    await io.click(candidate.x, candidate.y, "left");
    await io.wait(150);
    await io.save("hud1-after-selection.png", await io.screenshot());
}

module.exports = { drive };
