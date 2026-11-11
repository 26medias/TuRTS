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

// `where` is a window number (the camera at its first-join start) or a map point that the camera
// looks at after the driver centred it (for example with a group key pressed twice).
function seedFor(where, worldPoint) {
    const lookAt = Array.isArray(where) ? where : OWNED_CENTRE[where - 1];
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
async function findPlacement(io, where, slot, worldPoint, label, radius = 40, attempts = 10) {
    const seed = seedFor(where, worldPoint);
    if (!seed) throw new Error(`${label}: the map point ${JSON.stringify(worldPoint)} is not in the start camera view (instrument literal)`);
    for (let attempt = 0; attempt < attempts; attempt += 1) {
        const { image } = await shot(io, null);
        const found = findPaletteTargetNear(image, slot, seed, radius);
        if (found) {
            await io.save(`${label}-target.json`, Buffer.from(JSON.stringify({ seed, ...found }, null, 2) + "\n"));
            return found.target;
        }
        await io.wait(100);
    }
    await stop(io, label, `no slot ${slot} colour near the seed ${JSON.stringify(seed)}`);
    return null;
}

// A product fault (a unit or control not shown) ends the drive with a record, so the oracle
// gives the semantic failure (MC task 21, H1). A driver throw is for a harness or setup fault only.
async function stop(io, label, reason) {
    await io.save(`${label}-driver-stop.json`, Buffer.from(JSON.stringify({ reason }) + "\n"));
}

// A ground pixel for a map point: the projected seed, moved until it is open ground.
async function groundNear(io, where, worldPoint, label) {
    const seed = seedFor(where, worldPoint);
    if (!seed) throw new Error(`${label}: the map point ${JSON.stringify(worldPoint)} is not in the start camera view (instrument literal)`);
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
    if (!fallback) {
        await stop(io, label, `no open ground near ${JSON.stringify(seed)}`);
        return null;
    }
    await io.save(`${label}-ground.json`, Buffer.from(JSON.stringify({ seed, ground: fallback, fallback: true }) + "\n"));
    return fallback;
}

// A bounded series of screenshots after an action (the legacy m3.js waited up to 10 s for each
// result); the oracle uses the first checkpoint of the series that passes.
async function series(io, label, count = 20, ms = 500) {
    for (let index = 0; index < count; index += 1) {
        await shot(io, `${label}-${String(index).padStart(2, "0")}`);
        if (index + 1 < count) await io.wait(ms);
    }
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

// The guide close button (mods/forge/interface/hud.ron guide_close at (1178, 14), 92 x 44, over
// the orange guide_close_fill box) shows only while the guide is open. The button draws the
// engine button fill over the orange box (run t21-run118-pt1-1, pt1-s10-playing.png: a teal
// Close), and right after F1 the fade shows it darker (20, 44, 51). The four inner corners of the
// button, away from the "Close" label (x 1179-1222, y 15-29), must all show it: an alert line of the Alerts panel under that area is teal too, but it
// ends at x 1261 (run t21-run118-pt1-2, checkpoints 770-801: one teal sample, a false "open").
const GUIDE_CLOSE_CORNERS = [[1240, 20], [1266, 20], [1190, 52], [1266, 52]];
const GUIDE_CLOSE_FILLS = [[230, 159, 0], [33, 66, 77], [46, 102, 112], [26, 77, 92], [20, 44, 51]];

function guideOpen(image) {
    return GUIDE_CLOSE_CORNERS.every(([x, y]) => {
        const offset = (y * image.width + x) * 4;
        return GUIDE_CLOSE_FILLS.some((fill) => Math.hypot(image.rgba[offset] - fill[0], image.rgba[offset + 1] - fill[1],
            image.rgba[offset + 2] - fill[2]) <= 14);
    });
}

// Open or close the guide with its key, after a screenshot shows its current state.
async function setGuide(io, binding, open, label) {
    const { image } = await shot(io, null);
    if (guideOpen(image) !== open) {
        await pressBinding(io, binding, label);
        await io.wait(250);
    }
}

// Press one effective binding. `binding` is one entry of an effective_bindings action list:
// {"Key": {"code": ..., "mods": [...]}}. The modifier names of the input map ("Ctrl", "Shift",
// "Alt") are the names that the harness io accepts.
async function pressBinding(io, binding, label) {
    const key = binding?.Key;
    // params.bindings is setup data: a binding that is not a key is a setup fault.
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
    guideOpen,
    pressBinding,
    seedFor,
    series,
    setGuide,
    shot,
    stop,
    waitForResearchButton,
};
