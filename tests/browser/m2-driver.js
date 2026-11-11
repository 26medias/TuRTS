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
// The plain HUD lists of mods/<package>/interface/hud.ron (ruling C384): m2_build_rows at
// (16, 190), 240 x 28 rows with a gap of 4, in the recipe order of the producer; m2_job_rows at
// (16, 258), Cancel at offset 176 in each 28-pixel row with a gap of 4.
const BUILD_ROW = (index) => ({ x: 136, y: 204 + index * 32, sample: { x: 252, y: 204 + index * 32 } });
const JOB_CANCEL = (index) => ({ x: 224, y: 272 + index * 32, sample: { x: 195, y: 272 + index * 32 } });
// Engine button fills (crates/turts_client/src/widgets/mod.rs update_widget_button_states).
const BUTTON_FILL = { enabled: [33, 66, 77], hovered: [46, 102, 112], pressed: [26, 77, 92], disabled: [51, 51, 51] };

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
    return stop(io, label, `no slot ${slot} colour near ${JSON.stringify(seed)}`);
}

// A product fault (a unit, node or button not shown) ends the drive with a record, so the oracle
// gives the semantic failure (MC task 21, H1). A driver throw is for a harness or setup fault only.
async function stop(io, label, reason) {
    await io.save(`${label}-driver-stop.json`, Buffer.from(JSON.stringify({ reason }) + "\n"));
    return null;
}

// A node has no slot colour. The projected node point must not be the plain ground colour.
async function nodeAt(io, where, label) {
    const seed = where.seed(where.map.node);
    const image = decodePng(Buffer.from(await io.screenshot()));
    const pixel = pixelAt(image, seed.x, seed.y);
    if (!pixel || Math.max(pixel[0], pixel[1], pixel[2]) < 24) {
        return stop(io, label, `the node point ${JSON.stringify(seed)} shows only ground`);
    }
    await io.save(`${label}-target.json`, Buffer.from(JSON.stringify({ seed, pixel }) + "\n"));
    return seed;
}

function buttonState(image, point) {
    const pixel = pixelAt(image, point.x, point.y);
    if (!pixel) return null;
    for (const [state, fill] of Object.entries(BUTTON_FILL)) {
        if (Math.hypot(pixel[0] - fill[0], pixel[1] - fill[1], pixel[2] - fill[2]) <= 12) return state;
    }
    return null;
}

// Click a HUD button only after a screenshot shows it in the enabled fill.
async function clickButton(io, target, label) {
    let state = null;
    for (let attempt = 0; attempt < 20; attempt += 1) {
        // Away from the HUD, the units and the edge-scroll band, so the sample shows no hover fill.
        await io.move(700, 520);
        state = buttonState(decodePng(Buffer.from(await io.screenshot())), target.sample);
        if (state === "enabled") break;
        await io.wait(150);
    }
    await io.save(`${label}-button.json`, Buffer.from(JSON.stringify({ target, state }) + "\n"));
    if (state !== "enabled") {
        await stop(io, label, `the HUD button at ${JSON.stringify(target.sample)} is not shown enabled (${state})`);
        return false;
    }
    await io.click(target.x, target.y, "left");
    await io.wait(300);
    return true;
}

async function selectAndHover(io, where, point, label) {
    const target = await unitAt(io, where, 0, point, label);
    if (!target) return null;
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
    if (!worker) return false;
    await io.click(worker.x, worker.y, "left");
    await io.wait(250);
    await shot(io, `${label}-worker-selected`);
    const node = await nodeAt(io, where, `${label}-node`);
    if (!node) return false;
    await io.click(node.x, node.y, "right");
    await io.wait(300);
    await shot(io, `${label}-gather-command`);
    for (let second = 1; second <= seconds; second += 1) {
        await io.wait(1000);
        await shot(io, `${label}-gather-${String(second).padStart(2, "0")}`);
    }
}

const FAULT_CASES = new Set(["E9-browser-base-recovery", "E9-browser-base-loss-conversion", "E9-browser-base-loss-charge",
    "E9-browser-base-loss-refund", "E9-browser-base-loss-spawn", "E9-browser-base-loss-final_change"]);
const WORKER_CASES = new Set(["E9-browser-base-recovery", "E9-browser-base-loss-conversion"]);

async function drive(io, params) {
    const where = layout(params);
    const caseId = params.m2.case_id;
    const phase = params.fixture_phase ?? null;
    await io.use(1);
    if (FAULT_CASES.has(caseId)) {
        if (phase === "select") {
            await shot(io, "m2-joined");
            const worker = WORKER_CASES.has(caseId);
            const target = await unitAt(io, where, 0, worker ? where.map.worker : where.map.producer, worker ? "m2-worker" : "m2-producer");
            if (!target) return;
            await io.click(target.x, target.y, "left");
            await io.wait(250);
            await shot(io, worker ? "m2-worker-selected" : "m2-producer-selected");
        } else if (phase === "produce") {
            if (!await clickButton(io, BUILD_ROW(0), "m2-loss-produce")) return;
            await shot(io, "m2-loss-produce");
        } else if (phase === "show-queue") {
            await shot(io, "m2-loss-queue");
        } else if (phase === "cancel") {
            if (!await clickButton(io, JOB_CANCEL(0), "m2-loss-cancel")) return;
            await shot(io, "m2-loss-cancel");
        } else if (phase === "spawn-output") {
            await shot(io, "m2-spawn-output");
        } else if (phase === "gather") {
            const node = await nodeAt(io, where, "m2-node");
            if (!node) return;
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
            if (!target) return;
            await io.click(target.x, target.y, "left");
            await io.wait(250);
            await shot(io, "m2-owner1-selected");
        }
        return;
    }
    if (!await selectAndHover(io, where, where.map.producer, "m2-producer")) return;
    if (caseId === "E9-browser-base-economy") {
        // m2.js: one rifleman completes; a second is cancelled for its refund; the harvester gathers.
        if (!await clickButton(io, BUILD_ROW(0), "m2-econ-first")) return;
        await shot(io, "m2-econ-first-queued");
        for (let second = 1; second <= 15; second += 1) {
            await io.wait(1000);
            await shot(io, `m2-econ-output-${String(second).padStart(2, "0")}`);
        }
        if (!await clickButton(io, BUILD_ROW(0), "m2-econ-second")) return;
        await shot(io, "m2-econ-second-queued");
        if (!await clickButton(io, JOB_CANCEL(0), "m2-econ-cancel")) return;
        await shot(io, "m2-econ-cancelled");
        await gather(io, where, "m2", 15);
    } else if (caseId === "E9-browser-haul-economy" || caseId === "E9-browser-cedar-economy") {
        if (!await clickButton(io, BUILD_ROW(0), "m2-prod")) return;
        await shot(io, "m2-prod-queued");
        if (!await clickButton(io, JOB_CANCEL(0), "m2-prod-cancel")) return;
        await shot(io, "m2-prod-cancelled");
        await gather(io, where, "m2", 20);
    }
}

module.exports = { drive, PACKAGES, BUILD_ROW, JOB_CANCEL };
