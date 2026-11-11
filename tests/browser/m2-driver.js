// turts-test-role: driver
"use strict";

// M2 browser cases on the migrated M2 packages and their plain HUDs (MC design 12.2 and 13.3;
// replaces the browser actions of tests/browser/m2.js). The driver holds the package maps and
// the start camera as literals (MC design 13.4), finds each unit by its slot colour near the
// projected map point, and acts through the harness io only. The lost-delta faults of the
// recovery and loss cases are harness steps (harness/m2-ingest.js); the driver acts between them.
const { decodePng, pixelAt } = require("./harness/png.js");
const { projectWorldPoint } = require("./harness/project.js");
const { findPaletteTargetNear } = require("./sel1-driver.js");

// mods/<package>/maps/*.ron; scripts/m2-browser.sh moves base, haul and cedar 18 map units in x
// (tests/m2/browser_fixture.py). Each HUD camera: pitch 0.7, yaw 0, field of view 60, height 40,
// start MapCentre (mods/<package>/interface/hud.ron).
const PACKAGES = {
    base: { offset: 18, size: [64, 64], producer: [8, 8], worker: [12, 8], node: [24, 8], foreign: [52, 8] },
    haul: { offset: 18, size: [48, 48], producer: [8, 8], worker: [12, 8], node: [20, 8], foreign: [36, 8] },
    cedar: { offset: 18, size: [64, 64], producer: [8, 8], worker: [12, 8], node: [24, 8], foreign: [52, 8] },
    acme: { offset: 0, size: [48, 48], producer: [4, 4], worker: null, node: null, foreign: null },
};
const VIEWPORT = { width: 1280, height: 720 };

function layout(params) {
    const name = params.m2?.package;
    const map = PACKAGES[name];
    if (!map) throw new Error(`M2 driver has no map literals for package ${name}`);
    const at = (point) => point && [point[0] + map.offset, point[1]];
    const camera = { height: 40, pitch: 0.7, yaw: 0, fov: Math.PI / 3, lookAt: [(map.size[0] + map.offset) / 2, map.size[1] / 2] };
    const seed = (point) => {
        const pixel = projectWorldPoint(at(point), camera, VIEWPORT);
        if (!pixel) throw new Error(`M2 map point ${JSON.stringify(point)} of ${name} is not in view`);
        return { x: Math.round(pixel[0]), y: Math.round(pixel[1]) };
    };
    return { name, map, seed };
}

async function shot(io, label) {
    const bytes = Buffer.from(await io.screenshot());
    await io.save(`${label}.png`, bytes);
    await io.save(`${label}.png.b64`, Buffer.from(bytes.toString("base64")));
    return decodePng(bytes);
}

async function unitAt(io, where, slot, point, label) {
    const seed = where.seed(point);
    for (let attempt = 0; attempt < 20; attempt += 1) {
        const image = decodePng(Buffer.from(await io.screenshot()));
        const found = findPaletteTargetNear(image, slot, seed, 14);
        if (found) {
            await io.save(`${label}-target.json`, Buffer.from(JSON.stringify({ seed, ...found }) + "\n"));
            return found.target;
        }
        await io.wait(150);
    }
    throw new Error(`M2 ${label}: no slot ${slot} colour near ${JSON.stringify(seed)}; pixel target is NOT TRACED`);
}

// A node has no slot colour. The projected node point must not be the plain ground colour.
async function nodeAt(io, where, label) {
    const seed = where.seed(where.map.node);
    const image = decodePng(Buffer.from(await io.screenshot()));
    const pixel = pixelAt(image, seed.x, seed.y);
    if (!pixel || Math.max(pixel[0], pixel[1], pixel[2]) < 24) {
        throw new Error(`M2 ${label}: the node point ${JSON.stringify(seed)} shows only ground; pixel target is NOT TRACED`);
    }
    await io.save(`${label}-target.json`, Buffer.from(JSON.stringify({ seed, pixel }) + "\n"));
    return seed;
}

async function selectAndHover(io, where, point, label) {
    const target = await unitAt(io, where, 0, point, label);
    await io.click(target.x, target.y, "left");
    await io.wait(250);
    await shot(io, `${label}-selected`);
    await io.move(target.x, target.y);
    await io.wait(700);
    await shot(io, `${label}-hover`);
    return target;
}

async function gather(io, where, label, seconds) {
    const worker = await unitAt(io, where, 0, where.map.worker, `${label}-worker`);
    await io.click(worker.x, worker.y, "left");
    await io.wait(250);
    await shot(io, `${label}-worker-selected`);
    const node = await nodeAt(io, where, `${label}-node`);
    await io.click(node.x, node.y, "right");
    await io.wait(300);
    await shot(io, `${label}-gather-command`);
    for (let second = 1; second <= seconds; second += 1) {
        await io.wait(1000);
        await shot(io, `${label}-gather-${String(second).padStart(2, "0")}`);
    }
}

async function drive(io, params) {
    const where = layout(params);
    const caseId = params.m2.case_id;
    const phase = params.fixture_phase ?? null;
    await io.use(1);
    if (caseId === "E9-browser-base-recovery" || caseId === "E9-browser-base-loss-conversion") {
        if (phase === "select-worker") {
            await shot(io, "m2-joined");
            const worker = await unitAt(io, where, 0, where.map.worker, "m2-worker");
            await io.click(worker.x, worker.y, "left");
            await io.wait(250);
            await shot(io, "m2-worker-selected");
        } else if (phase === "gather") {
            const node = await nodeAt(io, where, "m2-node");
            await io.click(node.x, node.y, "right");
            await io.wait(300);
            await shot(io, "m2-gather-command");
        } else if (phase === "after") {
            await shot(io, "m2-after");
        } else {
            throw new Error(`M2 ${caseId} has no driver work for phase ${phase}`);
        }
        return;
    }
    await shot(io, "m2-joined");
    if (caseId.endsWith("-public-convergence")) {
        await io.use(2);
        await shot(io, "m2-owner1-joined");
        if (where.map.foreign) {
            const target = await unitAt(io, where, 1, where.map.foreign, "m2-owner1-unit");
            await io.click(target.x, target.y, "left");
            await io.wait(250);
            await shot(io, "m2-owner1-selected");
        }
        return;
    }
    await selectAndHover(io, where, where.map.producer, "m2-producer");
    if (where.map.worker && (caseId === "E9-browser-haul-economy" || caseId === "E9-browser-cedar-economy")) {
        await gather(io, where, "m2", 20);
    }
}

module.exports = { drive, PACKAGES };
