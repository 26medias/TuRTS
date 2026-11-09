// turts-test-role: driver
"use strict";

const { decodePng } = require("./harness/png.js");
const { findScout } = require("./harness/click9-pixels.js");

async function capture(io, name) {
    const bytes = await io.screenshot();
    await io.save(`${name}.png`, bytes);
    return decodePng(bytes);
}

async function findScoutFromPixels(io, stage) {
    let lastScreenshot = null;
    for (let attempt = 0; attempt < 80; attempt += 1) {
        const bytes = await io.screenshot();
        lastScreenshot = bytes;
        const image = decodePng(bytes);
        const scout = findScout(image);
        if (scout) {
            await io.save(`${stage}.png`, bytes);
            await io.save(`${stage}-pixel-target.json`, Buffer.from(JSON.stringify(scout) + "\n"));
            return scout;
        }
        await io.wait(100);
    }
    if (lastScreenshot) await io.save(`${stage}-recognition-failure.png`, lastScreenshot);
    throw new Error("CAM1 could not find the rendered scout from screenshot pixels");
}

async function drive(io) {
    const initial = await findScoutFromPixels(io, "00-camera-start");
    await io.click(initial.x, initial.y, "left");
    await io.wait(100);
    await capture(io, "01-scout-selected");

    await io.key("ArrowRight", [], { hold_ms: 1000 });
    await io.wait(100);
    await capture(io, "02-arrow-right");

    await io.move(1278, 360);
    await io.wait(1000);
    await capture(io, "03-right-edge-band");

    await io.move(640, 360);
    await io.wait(100);
    await io.wheel(-1200);
    await io.wait(100);
    await capture(io, "04-zoom-limit-a");
    await io.wait(100);
    await io.wheel(-1200);
    await io.wait(100);
    await capture(io, "05-zoom-limit-a-repeat");
    await io.wheel(2400);
    await io.wait(100);
    await capture(io, "06-zoom-limit-b");
    await io.wheel(2400);
    await io.wait(100);
    await capture(io, "07-zoom-limit-b-repeat");
    await io.key("Space");
    await io.wait(100);
    await capture(io, "08-space-centred");

    await io.move(640, 1);
    await io.wait(1000);
    await capture(io, "09-top-edge-band");
    await io.move(640, -1);
    await io.wait(100);
    await capture(io, "10-after-pointer-left-short");
    await io.wait(1000);
    await capture(io, "11-after-pointer-left-long");
    await io.save("cam1-pixel-targets.json", Buffer.from(JSON.stringify({ initial }) + "\n"));
}

module.exports = { drive };
