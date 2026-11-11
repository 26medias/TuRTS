// turts-test-role: driver
"use strict";

// Screenshot-only helpers for the forge drivers of the MC ledger rewrites (m3-driver.js,
// pt1-driver.js). The driver holds the forge layout and map as literals, as a person learns a
// HUD and a map (MC design 13.4); it never reads the package files (A7). Each target is a seed
// that a current screenshot must confirm by its slot colour before the driver clicks it.
const { decodePng } = require("./harness/png.js");
const { projectWorldPoint } = require("./harness/project.js");
const { findPaletteTargetNear, findOpenGround, readSlotTargets } = require("./sel1-driver.js");
const { hasEnabledResearchButton } = require("./hud2-driver.js");

// mods/forge/interface/hud.ron: camera (pitch 0.70, yaw 0.0, field_of_view 60, height 28,
// start OwnedCentre). The centre of the own units, belt pieces included, at the first join.
const FORGE_CAMERA = { height: 28, pitch: 0.7, yaw: 0, fov: Math.PI / 3 };
const OWNED_CENTRE = [[13.07, 8], [27, 8]];
// mods/forge/maps/proving_ground.ron placements.
const FORGE_MAP = {
    drill: [6, 8], press: [17, 8], turret: [17, 12], lab: [17, 4],
    belt: [11, 8], nearDrone: [24, 12], farDrone: [30, 4],
};
const TURRET_RANGE = 8;
// Forge HUD literals (hud2-driver.js and mods/forge/interface/hud.ron).
const RESEARCH_BUTTON_TARGET = { x: 144, y: 487 };
const QUEUE_CANCEL_TARGET = { x: 229, y: 658 };
const GUIDE_CLOSE_TARGET = { x: 1224, y: 36 };
const VIEWPORT = { width: 1280, height: 720 };

function seedFor(windowNumber, worldPoint) {
    const lookAt = OWNED_CENTRE[windowNumber - 1];
    const pixel = projectWorldPoint(worldPoint, { ...FORGE_CAMERA, lookAt }, VIEWPORT);
    return pixel ? { x: Math.round(pixel[0]), y: Math.round(pixel[1]) } : null;
}

async function shot(io, label) {
    const bytes = Buffer.from(await io.screenshot());
    if (label) {
        await io.save(`${label}.png`, bytes);
        await io.save(`${label}.png.b64`, Buffer.from(bytes.toString("base64")));
    }
    return { bytes, image: decodePng(bytes) };
}

// Find the slot-coloured body near the projected seed of a map placement.
async function findPlacement(io, windowNumber, slot, worldPoint, label, radius = 40, attempts = 40) {
    const seed = seedFor(windowNumber, worldPoint);
    if (!seed) throw new Error(`${label}: the map point ${JSON.stringify(worldPoint)} is not in view; pixel target is NOT TRACED`);
    for (let attempt = 0; attempt < attempts; attempt += 1) {
        const { image } = await shot(io, null);
        const found = findPaletteTargetNear(image, slot, seed, radius);
        if (found) {
            await io.save(`${label}-target.json`, Buffer.from(JSON.stringify({ seed, ...found }, null, 2) + "\n"));
            return found.target;
        }
        await io.wait(100);
    }
    throw new Error(`${label}: no slot ${slot} colour near the seed ${JSON.stringify(seed)}; pixel target is NOT TRACED`);
}

// A ground pixel for a map point: the projected seed, moved until it is open ground.
async function groundNear(io, windowNumber, worldPoint, label) {
    const seed = seedFor(windowNumber, worldPoint);
    if (!seed) throw new Error(`${label}: the map point ${JSON.stringify(worldPoint)} is not in view`);
    const { image } = await shot(io, null);
    const bodies = [...readSlotTargets(image, 0), ...readSlotTargets(image, 1)];
    const clear = (x, y) => !bodies.some((body) => {
        const [left, top, width, height] = body.bounds;
        return x >= left - 12 && x <= left + width + 12 && y >= top - 12 && y <= top + height + 12;
    });
    for (let ring = 0; ring <= 48; ring += 8) {
        for (const [dx, dy] of [[0, 0], [ring, 0], [-ring, 0], [0, ring], [0, -ring]]) {
            const x = seed.x + dx;
            const y = seed.y + dy;
            if (clear(x, y)) {
                await io.save(`${label}-ground.json`, Buffer.from(JSON.stringify({ seed, ground: { x, y } }) + "\n"));
                return { x, y };
            }
        }
    }
    const fallback = findOpenGround(image, bodies);
    if (!fallback) throw new Error(`${label}: no open ground near ${JSON.stringify(seed)}`);
    return fallback;
}

async function waitForResearchButton(io, seconds, label) {
    for (let attempt = 0; attempt < seconds; attempt += 1) {
        const current = await shot(io, null);
        if (hasEnabledResearchButton(current.image)) {
            await io.save(`${label}.png`, current.bytes);
            await io.save(`${label}.png.b64`, Buffer.from(current.bytes.toString("base64")));
            return true;
        }
        await io.wait(1000);
    }
    return false;
}

// Press one effective binding. `binding` is one entry of an effective_bindings action list:
// {"Key": {"code": ..., "mods": [...]}}. The modifier names of the input map ("Ctrl", "Shift",
// "Alt") are the names that the harness io accepts.
async function pressBinding(io, binding, label) {
    const key = binding?.Key;
    if (!key || typeof key.code !== "string") throw new Error(`${label}: the effective binding is not a key: ${JSON.stringify(binding)}`);
    await io.key(key.code, key.mods ?? []);
}

module.exports = {
    FORGE_MAP,
    GUIDE_CLOSE_TARGET,
    OWNED_CENTRE,
    QUEUE_CANCEL_TARGET,
    RESEARCH_BUTTON_TARGET,
    TURRET_RANGE,
    findPlacement,
    groundNear,
    pressBinding,
    seedFor,
    shot,
    waitForResearchButton,
};
