// turts-test-role: pure
"use strict";

// The sound switch of the play screen, found and used from screenshot pixels only. A switch is a
// 124 x 28 block of one of two fills: dark red is "Sound: off", dark green is "Sound: on". Two kinds
// of switch have that look. The engine fallback is a solid block (`client_shell/notices.rs`:
// SOUND_SWITCH_WIDTH, SOUND_SWITCH_HEIGHT; `volume_settings.rs`: `sound_fill`). The switch of a
// package is a frame of 2 px of the fill round a button (`mods/<package>/interface/hud.ron`:
// `sound_switch_off` and `sound_switch_on`). The finder does not know the place: it takes the
// connected block of a fill that has the size, anywhere on the screen. The words come from the
// debug state, not from here. The fills and the size are the same values as in the Rust and the
// RON: change them together.
const { decodePng } = require("./png.js");

const SOUND_SWITCH = Object.freeze({
    off: Object.freeze([77, 31, 31]),
    on: Object.freeze([31, 97, 56]),
    width: 124,
    height: 28,
    colourTolerance: 8,
    sizeTolerance: 4,
    // A solid block fills all of its box; a frame of 2 px fills about a sixth of it.
    minimumFill: 0.12,
});

function matches(image, offset, colour) {
    return image.rgba[offset + 3] >= 245 &&
        colour.every((channel, index) => Math.abs(image.rgba[offset + index] - channel) <= SOUND_SWITCH.colourTolerance);
}

// The connected blocks (8 neighbours) of one fill, as boxes with their pixel count.
function blocksOf(image, colour) {
    const size = image.width * image.height;
    const mask = new Uint8Array(size);
    for (let index = 0; index < size; index += 1) mask[index] = matches(image, index * 4, colour) ? 1 : 0;
    const blocks = [];
    const stack = [];
    for (let start = 0; start < size; start += 1) {
        if (mask[start] !== 1) continue;
        let minX = image.width, minY = image.height, maxX = -1, maxY = -1, count = 0;
        mask[start] = 2;
        stack.push(start);
        while (stack.length) {
            const at = stack.pop();
            const x = at % image.width, y = (at - x) / image.width;
            count += 1;
            if (x < minX) minX = x;
            if (x > maxX) maxX = x;
            if (y < minY) minY = y;
            if (y > maxY) maxY = y;
            for (let dy = -1; dy <= 1; dy += 1) {
                for (let dx = -1; dx <= 1; dx += 1) {
                    const nx = x + dx, ny = y + dy;
                    if (nx < 0 || ny < 0 || nx >= image.width || ny >= image.height) continue;
                    const next = ny * image.width + nx;
                    if (mask[next] === 1) { mask[next] = 2; stack.push(next); }
                }
            }
        }
        blocks.push({ minX, minY, maxX, maxY, count });
    }
    return blocks;
}

function switchBlock(image, colour, scale) {
    const tolerance = SOUND_SWITCH.sizeTolerance * scale;
    for (const block of blocksOf(image, colour)) {
        const width = block.maxX - block.minX + 1, height = block.maxY - block.minY + 1;
        if (Math.abs(width - SOUND_SWITCH.width * scale) > tolerance ||
            Math.abs(height - SOUND_SWITCH.height * scale) > tolerance) continue;
        if (block.count < SOUND_SWITCH.minimumFill * width * height) continue;
        return { ...block, x: Math.round((block.minX + block.maxX) / 2), y: Math.round((block.minY + block.maxY) / 2) };
    }
    return null;
}

// { state: "off" | "on", x, y, bounds } for a decoded screenshot, or null. `scale` is the device
// pixel ratio of the screenshot (the click point is returned in the same pixels).
function findSoundSwitch(image, scale = 1) {
    if (!image || !(image.rgba instanceof Uint8Array) || image.rgba.length !== image.width * image.height * 4) {
        throw new TypeError("findSoundSwitch needs decoded RGBA screenshot pixels");
    }
    for (const state of ["off", "on"]) {
        const block = switchBlock(image, SOUND_SWITCH[state], scale);
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
