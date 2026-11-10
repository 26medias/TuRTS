// turts-test-role: driver
"use strict";

const { decodePng, pixelAt } = require("./harness/png.js");

const SLOT_COLOURS = [[0, 114, 178], [230, 159, 0]];
const COLOUR_TOLERANCE = 60;

async function drive(io) {
    const calls = [];
    const shots = [];
    async function act(label, method, ...args) {
        await io[method](...args);
        calls.push({ label, method, args });
    }
    async function capture(label, target = null) {
        const imageBytes = Buffer.from(await io.screenshot());
        const image = decodePng(imageBytes);
        await io.save(`${label}.png`, imageBytes);
        await io.save(`${label}.png.b64`, Buffer.from(imageBytes.toString("base64")));
        shots.push({ label, target });
        return image;
    }

    await act("close-guide", "key", "F1");
    await act("guide-settle", "wait", 200);
    let initial = null;
    let initialBytes = null;
    let targets = null;
    for (let attempt = 0; attempt < 80; attempt += 1) {
        initialBytes = Buffer.from(await io.screenshot());
        initial = decodePng(initialBytes);
        targets = readTargets(initial);
        if (targets) break;
        if (attempt < 79) await act("unit-pixel-wait", "wait", 100);
    }
    if (!targets) {
        throw new Error("SEL1 could not identify own and foreign unit colour components in the screenshot; pixel targets are NOT TRACED");
    }
    const ground = findOpenGround(initial, [targets.own, targets.foreign]);
    if (!ground) throw new Error("SEL1 could not identify an open ground pixel for selection clear; pixel target is NOT TRACED");

    await io.save("sel1-initial.png", initialBytes);
    await io.save("sel1-initial.png.b64", Buffer.from(initialBytes.toString("base64")));
    shots.push({ label: "sel1-initial", target: null });

    const evidence = {
        viewport: { width: initial.width, height: initial.height },
        method: "connected components of the authored slot palette and open-ground pixels from screenshots only",
        targets: { own: targets.own, foreign: targets.foreign, ground },
        shots,
        calls,
    };
    await io.save("sel1-pixel-targets.json", Buffer.from(JSON.stringify(evidence, null, 2) + "\n"));

    await act("select-own", "click", targets.own.x, targets.own.y, "left");
    await act("own-settle", "wait", 150);
    await capture("sel1-own-selected", targets.own);

    await act("select-foreign-turret", "click", targets.foreign.x, targets.foreign.y, "left");
    await act("foreign-settle", "wait", 150);
    await capture("sel1-foreign-selected", targets.foreign);

    await act("clear-on-ground", "click", ground.x, ground.y, "left");
    await act("clear-settle", "wait", 150);
    await capture("sel1-ground-cleared", ground);

    await act("hover-foreign-turret", "move", targets.foreign.x, targets.foreign.y);
    await act("hover-immediate-wait", "wait", 100);
    await capture("sel1-hover-immediate", targets.foreign);
    await act("hover-tooltip-delay", "wait", 600);
    await capture("sel1-hover-delayed", targets.foreign);

    evidence.shots = shots;
    evidence.calls = calls;
    await io.save("sel1-steps.json", Buffer.from(JSON.stringify({ shots, calls }, null, 2) + "\n"));
}

function readTargets(image) {
    const slot0 = colourComponents(image, SLOT_COLOURS[0]).filter(isBody);
    const slot1 = colourComponents(image, SLOT_COLOURS[1]).filter(isBody);
    if (slot0.length === 0 || slot1.length < 3) return null;
    const own = slot0.sort((left, right) => right.pixels - left.pixels)[0];
    // The instrument fixture adds one existing Forge turret for slot 1. Its declared
    // footprint is twice the foreign drones, so the largest orange component is the target.
    const foreign = slot1.sort((left, right) => right.pixels - left.pixels)[0];
    return { own: foot(own), foreign: foot(foreign), bodies: [...slot0, ...slot1] };
}

function readOwnTarget(image) {
    const candidates = colourComponents(image, SLOT_COLOURS[0]).filter(isBody);
    const body = candidates.sort((left, right) => right.pixels - left.pixels)[0];
    return body ? foot(body) : null;
}

function readSlotTargets(image, slot) {
    const colour = SLOT_COLOURS[slot];
    if (!colour) throw new Error(`unknown authored slot palette ${slot}`);
    return colourComponents(image, colour)
        .filter(isBody)
        .sort((left, right) => right.pixels - left.pixels)
        .map(foot);
}

function findPaletteTargetNear(image, slot, seed, radius = 12) {
    const colour = SLOT_COLOURS[slot];
    if (!colour) throw new Error(`unknown authored slot palette ${slot}`);
    const pixels = [];
    for (let y = Math.max(0, seed.y - radius); y <= Math.min(image.height - 1, seed.y + radius); y += 1) {
        for (let x = Math.max(0, seed.x - radius); x <= Math.min(image.width - 1, seed.x + radius); x += 1) {
            if (isHudPixel(x, y, image)) continue;
            const pixel = pixelAt(image, x, y);
            if (colourNear(pixel, colour)) pixels.push({ x, y });
        }
    }
    if (pixels.length < 25) return null;
    const bounds = {
        left: Math.min(...pixels.map((point) => point.x)),
        top: Math.min(...pixels.map((point) => point.y)),
        right: Math.max(...pixels.map((point) => point.x)),
        bottom: Math.max(...pixels.map((point) => point.y)),
    };
    if (bounds.right - bounds.left < 3 || bounds.bottom - bounds.top < 3) return null;
    const target = pixels.reduce((nearest, point) =>
        Math.hypot(point.x - seed.x, point.y - seed.y) < Math.hypot(nearest.x - seed.x, nearest.y - seed.y)
            ? point : nearest);
    return {
        target: {
            x: target.x,
            y: target.y,
            bounds: [bounds.left, bounds.top, bounds.right - bounds.left + 1, bounds.bottom - bounds.top + 1],
            pixels: pixels.length,
        },
        observation: {
            slot,
            colour,
            seed,
            search_radius: radius,
            matching_pixel_count: pixels.length,
            bounds: [bounds.left, bounds.top, bounds.right - bounds.left + 1, bounds.bottom - bounds.top + 1],
            selected_pixel: target,
        },
    };
}

function colourComponents(image, colour) {
    const size = image.width * image.height;
    const seen = new Uint8Array(size);
    const queue = new Int32Array(size);
    const found = [];
    const matches = (index) => {
        const x = index % image.width;
        const y = Math.floor(index / image.width);
        if (isHudPixel(x, y, image)) return false;
        const offset = index * 4;
        return image.rgba[offset + 3] >= 245 && Math.hypot(
            image.rgba[offset] - colour[0],
            image.rgba[offset + 1] - colour[1],
            image.rgba[offset + 2] - colour[2],
        ) <= COLOUR_TOLERANCE;
    };
    for (let origin = 0; origin < size; origin += 1) {
        if (seen[origin] || !matches(origin)) continue;
        let tail = 1;
        queue[0] = origin;
        seen[origin] = 1;
        let minX = image.width;
        let minY = image.height;
        let maxX = -1;
        let maxY = -1;
        for (let head = 0; head < tail; head += 1) {
            const index = queue[head];
            const x = index % image.width;
            const y = Math.floor(index / image.width);
            minX = Math.min(minX, x);
            minY = Math.min(minY, y);
            maxX = Math.max(maxX, x);
            maxY = Math.max(maxY, y);
            for (const next of [
                x > 0 ? index - 1 : -1,
                x + 1 < image.width ? index + 1 : -1,
                y > 0 ? index - image.width : -1,
                y + 1 < image.height ? index + image.width : -1,
            ]) {
                if (next >= 0 && !seen[next] && matches(next)) {
                    seen[next] = 1;
                    queue[tail++] = next;
                }
            }
        }
        const width = maxX - minX + 1;
        const height = maxY - minY + 1;
        if (tail >= 20 && width >= 4 && height >= 4 && width <= 120 && height <= 120) {
            found.push({ minX, minY, maxX, maxY, width, height, pixels: tail });
        }
    }
    return found;
}

function isBody(body) {
    return body.pixels >= 20 && body.width >= 4 && body.height >= 4;
}

function foot(body) {
    return {
        x: Math.round((body.minX + body.maxX) / 2),
        y: Math.max(body.minY, body.maxY - Math.max(1, Math.round(body.height * 0.08))),
        bounds: [body.minX, body.minY, body.width, body.height],
        pixels: body.pixels,
    };
}

function findOpenGround(image, bodies) {
    let selected = null;
    let best = -Infinity;
    for (let y = Math.round(image.height * 0.30); y < Math.round(image.height * 0.80); y += 16) {
        for (let x = 32; x < image.width - 32; x += 16) {
            if (isHudPixel(x, y, image) || bodies.some((body) => {
                const [left, top, width, height] = body.bounds;
                return x >= left - 24 && x <= left + width + 24 && y >= top - 24 && y <= top + height + 24;
            })) continue;
            if (hasPaletteNear(image, x, y, 12)) continue;
            if (!isGroundPatch(image, x, y)) continue;
            const score = Math.hypot(x - image.width / 2, y - image.height / 2);
            if (score > best) { best = score; selected = { x, y }; }
        }
    }
    return selected;
}

// The current scene floor is a green PBR surface. Require a whole visible patch,
// so empty canvas, HUD, unit bodies and thin health bars cannot stand in for ground.
function isGroundPatch(image, x, y) {
    for (const dy of [-8, -4, 0, 4, 8]) {
        for (const dx of [-8, -4, 0, 4, 8]) {
            const pixel = pixelAt(image, x + dx, y + dy);
            if (!pixel || pixel[3] < 245 || pixel[1] <= pixel[0] + 12 ||
                pixel[1] <= pixel[2] + 12) return false;
        }
    }
    return true;
}

function hasPaletteNear(image, x, y, radius) {
    for (let py = Math.max(0, y - radius); py <= Math.min(image.height - 1, y + radius); py += 2) {
        for (let px = Math.max(0, x - radius); px <= Math.min(image.width - 1, x + radius); px += 2) {
            const pixel = pixelAt(image, px, py);
            if (SLOT_COLOURS.some((colour) => colourNear(pixel, colour))) return true;
        }
    }
    return false;
}

function colourNear(pixel, colour) {
    return pixel && pixel[3] >= 245 && Math.hypot(pixel[0] - colour[0], pixel[1] - colour[1], pixel[2] - colour[2]) <= COLOUR_TOLERANCE;
}

function isHudPixel(x, y, image) {
    return x < 288 ||
        (x >= 288 && x < 700 && y < 112) ||
        (x >= 928 && y < 208) ||
        (x >= 992 && y >= 222 && y < 492) ||
        (x >= 1072 && y >= 520) ||
        y > image.height - 40;
}

module.exports = { drive, readOwnTarget, readSlotTargets, findOpenGround, findPaletteTargetNear };
