// turts-test-role: pure
"use strict";

// The sound switch of the play screen, found and used from screenshot pixels only. The client
// draws it at the bottom, right of the centre, as a filled button (`client_shell/notices.rs`:
// SOUND_SWITCH_WIDTH, SOUND_SWITCH_HEIGHT; `volume_settings.rs`: `sound_fill`). The two fills and the
// size below are the same values: change both together. A fill says the state: dark red is
// "Sound: off", dark green is "Sound: on". The words come from the debug state, not from here.
const { decodePng } = require("./png.js");

const SOUND_SWITCH = Object.freeze({
    off: Object.freeze([77, 31, 31]),
    on: Object.freeze([31, 97, 56]),
    width: 124,
    height: 28,
    colourTolerance: 8,
    sizeTolerance: 4,
    bandHeight: 90,
    minimumFill: 0.55,
});

function matches(image, offset, colour) {
    return image.rgba[offset + 3] >= 245 &&
        colour.every((channel, index) => Math.abs(image.rgba[offset + index] - channel) <= SOUND_SWITCH.colourTolerance);
}

// The bounding box of one fill in the bottom band, or null when it is not a switch-sized block.
function blockOf(image, colour, scale) {
    const top = Math.max(0, image.height - Math.ceil(SOUND_SWITCH.bandHeight * scale));
    let minX = image.width, minY = image.height, maxX = -1, maxY = -1, count = 0;
    for (let y = top; y < image.height; y += 1) {
        for (let x = 0; x < image.width; x += 1) {
            if (!matches(image, (y * image.width + x) * 4, colour)) continue;
            count += 1;
            if (x < minX) minX = x;
            if (x > maxX) maxX = x;
            if (y < minY) minY = y;
            if (y > maxY) maxY = y;
        }
    }
    if (!count) return null;
    const width = maxX - minX + 1, height = maxY - minY + 1;
    const tolerance = SOUND_SWITCH.sizeTolerance * scale;
    if (Math.abs(width - SOUND_SWITCH.width * scale) > tolerance ||
        Math.abs(height - SOUND_SWITCH.height * scale) > tolerance) return null;
    if (count < SOUND_SWITCH.minimumFill * width * height) return null;
    return { minX, minY, maxX, maxY, count, x: Math.round((minX + maxX) / 2), y: Math.round((minY + maxY) / 2) };
}

// { state: "off" | "on", x, y, bounds } for a decoded screenshot, or null. `scale` is the device
// pixel ratio of the screenshot (the click point is returned in the same pixels).
function findSoundSwitch(image, scale = 1) {
    if (!image || !(image.rgba instanceof Uint8Array) || image.rgba.length !== image.width * image.height * 4) {
        throw new TypeError("findSoundSwitch needs decoded RGBA screenshot pixels");
    }
    for (const state of ["off", "on"]) {
        const block = blockOf(image, SOUND_SWITCH[state], scale);
        if (block) return { state, x: block.x, y: block.y, bounds: [block.minX, block.minY, block.maxX, block.maxY] };
    }
    return null;
}

async function waitForSoundSwitch(io, wanted, label, millis = 10000, decode = decodePng) {
    const deadline = Date.now() + millis;
    let last = null;
    while (Date.now() < deadline) {
        const screenshot = Buffer.from(await io.screenshot());
        last = { screenshot, found: findSoundSwitch(decode(screenshot)) };
        if (last.found && (wanted === undefined || last.found.state === wanted)) return last;
        await io.wait(Math.min(100, Math.max(1, deadline - Date.now())));
    }
    throw new Error(`${label}: the sound switch did not show ${wanted === undefined ? "at all" : `"${wanted}"`} in screenshot pixels within ${millis} ms` +
        (last?.found ? ` (it shows "${last.found.state}")` : ""));
}

// Click the switch once, from the pixels of a current screenshot, and wait until the pixels show
// the other state. It saves `<label>-before.png` just before the click and `<label>-after.png`
// when the new state shows; an oracle reads the debug state of each. The page must show no
// "Click to play" overlay: the overlay hides the switch.
async function clickSoundSwitch(io, label, from, decode = decodePng) {
    const to = from === "off" ? "on" : "off";
    const before = await waitForSoundSwitch(io, from, `${label} before the click`, 10000, decode);
    await io.save(`${label}-before.png`, before.screenshot);
    await io.click(before.found.x, before.found.y, "left");
    await io.wait(150);
    const after = await waitForSoundSwitch(io, to, `${label} after the click`, 5000, decode);
    await io.save(`${label}-after.png`, after.screenshot);
    return { before: before.found, after: after.found };
}

const turnSoundOn = (io, label, decode) => clickSoundSwitch(io, label, "off", decode);
const turnSoundOff = (io, label, decode) => clickSoundSwitch(io, label, "on", decode);

module.exports = { SOUND_SWITCH, findSoundSwitch, waitForSoundSwitch, turnSoundOn, turnSoundOff, clickSoundSwitch };
