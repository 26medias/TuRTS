// turts-test-role: driver
"use strict";

const { decodePng } = require("./harness/png.js");
const { readTargets, findOpenGround } = require("./harness/hud4-pixels.js");

async function drive(io) {
    let inputCall = 0;
    const actions = [];
    const shots = [];
    async function act(label, call, ...args) {
        const callIndex = inputCall++;
        await io[call](...args);
        actions.push({ label, call: callIndex, method: call, args });
        return callIndex;
    }
    async function shot(label, save = true) {
        const screenshotCall = inputCall++;
        const bytes = Buffer.from(await io.screenshot());
        const record = { label, screenshot_call: screenshotCall, point: null };
        if (save) {
            shots.push(record);
            await saveScreenshotFiles(io, label, bytes);
            inputCall += 2;
        }
        return { bytes, record, image: decodePng(bytes) };
    }

    await act("close-guide", "key", "F1");
    await act("guide-layout-settle", "wait", 200);
    let initial = null;
    let targetSet = null;
    for (let attempt = 0; attempt < 80; attempt += 1) {
        initial = await shot(`vis1-target-probe-${attempt}`, false);
        targetSet = readTargets(initial.image);
        if (targetSet) break;
        if (attempt < 79) await act(`target-probe-wait-${attempt}`, "wait", 100);
    }
    if (!targetSet) {
        await io.save("vis1-full-recognition-failed.png", initial.bytes);
        throw new Error("VIS1 PNG target recognition failed before input; no scene/debug data was read by the driver");
    }
    initial.record.label = "vis1-full-initial";
    shots.push(initial.record);
    await saveScreenshotFiles(io, initial.record.label, initial.bytes);
    inputCall += 2;
    const avoid = targetSet.candidates.concat([{
        minX: targetSet.ground.x - 2,
        minY: targetSet.ground.y - 2,
        maxX: targetSet.ground.x + 2,
        maxY: targetSet.ground.y + 2,
    }]);
    const ground2 = findOpenGround(initial.image, avoid);
    if (!ground2) throw new Error("VIS1 could not derive a second open-ground point from the PNG");
    const targets = { ...targetSet.targets, ground: targetSet.ground, ground2 };
    await io.save("vis1-full-pixel-targets.json", Buffer.from(JSON.stringify({
        viewport: { width: initial.image.width, height: initial.image.height },
        method: "authored silhouette/color component recognition and green floor pixels only",
        targets,
    }, null, 2) + "\n"));
    inputCall += 1;

    async function capture(label, point = null, save = true) {
        const result = await shot(label, save);
        result.record.point = point;
        if (save) {
            await io.save(`${label}-point.json`, Buffer.from(JSON.stringify({ label, point }) + "\n"));
            inputCall += 1;
        }
        return result;
    }
    let currentImage = initial.image;
    let currentTargets = targets;
    const updateTargets = (image) => {
        const found = readTargets(image);
        if (found) currentTargets = { ...found.targets, ground: targets.ground, ground2: targets.ground2 };
        currentImage = image;
    };

    let actionCall = await act("select-own", "click", targets.attacker.x, targets.attacker.y, "left");
    await capture("vis1-own-selected", targets.attacker);
    let foreign = targets.foreign;
    actionCall = await act("select-foreign", "click", foreign.x, foreign.y, "left");
    await capture("vis1-foreign-selected", foreign);

    await act("clear-selection", "click", targets.ground.x, targets.ground.y, "left");
    await capture("vis1-selection-cleared", targets.ground);
    foreign = currentTargets.foreign;
    actionCall = await act("hover-foreign", "move", foreign.x, foreign.y);
    const hover = await capture("vis1-hover-immediate", foreign);
    await act("tooltip-delay", "wait", 500);
    await capture("vis1-hover-tooltip", foreign);

    // The local trooper attacks the PNG-recognized foreign target. Poll only the screenshot
    // pixels for the authored health-fill colour; stop after the first visible damage bar.
    const attacker = currentTargets.attacker;
    await act("select-attacker-for-damage", "click", attacker.x, attacker.y, "left");
    const actualForeign = currentTargets.foreign;
    actionCall = await act("attack-foreign", "click", actualForeign.x, actualForeign.y, "right");
    await capture("vis1-attack-immediate", actualForeign);
    await act("move-away-from-health-bar", "move", targets.ground.x, targets.ground.y);
    let damaged = null;
    for (let attempt = 0; attempt < 120; attempt += 1) {
        await act(`damage-poll-wait-${attempt}`, "wait", 100);
        const poll = await shot(`vis1-damage-poll-${attempt}`, false);
        updateTargets(poll.image);
        const point = currentTargets.foreign;
        if (hasHealthFill(poll.image, point)) {
            damaged = poll;
            damaged.record.label = "vis1-foreign-damaged";
            damaged.record.point = point;
            shots.push(damaged.record);
            await saveScreenshotFiles(io, damaged.record.label, damaged.bytes);
            inputCall += 2;
            break;
        }
    }
    if (!damaged) {
        const final = await shot("vis1-foreign-damage-not-seen", true);
        updateTargets(final.image);
    }
    await act("stop-after-first-damage", "key", "KeyS");
    let attackMarkerCleared = false;
    for (let attempt = 0; attempt < 80; attempt += 1) {
        await act(`attack-marker-clear-wait-${attempt}`, "wait", 100);
        const poll = await shot(`vis1-attack-clear-poll-${attempt}`, false);
        if (!hasAttackCrossNear(poll.image, currentTargets.foreign)) {
            attackMarkerCleared = true;
            break;
        }
    }
    const afterStop = await capture("vis1-after-damage-stop", currentTargets.foreign);
    afterStop.record.attack_marker_clear_seen = attackMarkerCleared;
    updateTargets(afterStop.image);

    // Issue a move and then queue a second waypoint. The first screenshot is taken directly
    // after each physical input; no delay is inserted before the first-frame checkpoints.
    const moveTarget = targets.ground2;
    actionCall = await act("move-marker", "click", moveTarget.x, moveTarget.y, "right");
    const moveImmediate = await capture("vis1-move-immediate", moveTarget);
    updateTargets(moveImmediate.image);
    await act("queue-modifier-down", "key", "ShiftLeft", [], { action: "down" });
    actionCall = await act("queue-waypoint", "click", targets.ground.x, targets.ground.y, "right");
    const waypointImmediate = await capture("vis1-waypoint-queued", targets.ground);
    updateTargets(waypointImmediate.image);
    await act("queue-modifier-up", "key", "ShiftLeft", [], { action: "up" });
    let queuedPathVisible = false;
    for (let attempt = 0; attempt < 80; attempt += 1) {
        await act(`queued-path-poll-wait-${attempt}`, "wait", 100);
        const poll = await shot(`vis1-waypoint-path-poll-${attempt}`, false);
        if (hasWaypointColorNear(poll.image, targets.ground)) {
            queuedPathVisible = true;
            break;
        }
    }
    if (!queuedPathVisible) throw new Error("VIS1 current-to-queued waypoint line was not visible at the PNG-derived queued target");
    await capture("vis1-waypoint-path", targets.ground);
    await act("active-marker-sample-delay", "wait", 200);
    const active = await capture("vis1-move-active", moveTarget);
    updateTargets(active.image);
    await act("stop-waypoint-sequence", "key", "KeyS");
    let markerCleared = false;
    for (let attempt = 0; attempt < 80; attempt += 1) {
        await act(`marker-clear-wait-${attempt}`, "wait", 100);
        const poll = await shot(`vis1-marker-clear-poll-${attempt}`, false);
        if (!hasRangeColorNear(poll.image, moveTarget)) {
            markerCleared = true;
            break;
        }
    }
    const stopped = await capture("vis1-move-stopped", moveTarget);
    stopped.record.marker_clear_seen = markerCleared;
    updateTargets(stopped.image);

    await io.save("vis1-full-steps.json", Buffer.from(JSON.stringify({ targets, actions, shots }, null, 2) + "\n"));
}

async function saveScreenshotFiles(io, label, bytes) {
    await io.save(`${label}.png`, bytes);
    await io.save(`${label}.png.b64`, Buffer.from(bytes.toString("base64")));
}

function hasHealthFill(image, target) {
    if (!target?.bounds) return false;
    const [left, top, width] = target.bounds;
    const x0 = Math.max(0, Math.floor(left + width / 2 - 32));
    const x1 = Math.min(image.width - 1, Math.ceil(left + width / 2 + 32));
    const y0 = Math.max(0, top - 28);
    const y1 = Math.min(image.height - 1, top + 4);
    let bestRow = 0;
    for (let y = y0; y <= y1; y += 1) {
        let row = 0;
        for (let x = x0; x <= x1; x += 1) {
            const i = (y * image.width + x) * 4;
            const pixel = [image.rgba[i], image.rgba[i + 1], image.rgba[i + 2]];
            if (Math.hypot(pixel[0] - 86, pixel[1] - 212, pixel[2] - 168) <= 55) row += 1;
        }
        bestRow = Math.max(bestRow, row);
    }
    return bestRow >= 6;
}

function hasRangeColorNear(image, point) {
    for (let y = Math.max(0, Math.floor(point.y - 10)); y <= Math.min(image.height - 1, Math.ceil(point.y + 10)); y += 1) {
        for (let x = Math.max(0, Math.floor(point.x - 10)); x <= Math.min(image.width - 1, Math.ceil(point.x + 10)); x += 1) {
            const i = (y * image.width + x) * 4;
            if (Math.hypot(image.rgba[i] - 242, image.rgba[i + 1] - 204, image.rgba[i + 2] - 96) <= 70) return true;
        }
    }
    return false;
}

function hasWaypointColorNear(image, point) {
    for (let y = Math.max(0, Math.floor(point.y - 8)); y <= Math.min(image.height - 1, Math.ceil(point.y + 8)); y += 1) {
        for (let x = Math.max(0, Math.floor(point.x - 8)); x <= Math.min(image.width - 1, Math.ceil(point.x + 8)); x += 1) {
            const i = (y * image.width + x) * 4;
            if (Math.hypot(image.rgba[i] - 230, image.rgba[i + 1] - 230, image.rgba[i + 2] - 217) <= 80) return true;
        }
    }
    return false;
}

function hasAttackCrossNear(image, point) {
    const colorAt = (x, y) => {
        if (x < 0 || y < 0 || x >= image.width || y >= image.height) return false;
        const offset = (y * image.width + x) * 4;
        return Math.hypot(image.rgba[offset] - 245, image.rgba[offset + 1] - 224, image.rgba[offset + 2] - 51) <= 80;
    };
    let best = 0;
    for (let cy = Math.max(4, Math.floor(point.y - 8)); cy <= Math.min(image.height - 5, Math.ceil(point.y + 8)); cy += 2) {
        for (let cx = Math.max(4, Math.floor(point.x - 8)); cx <= Math.min(image.width - 5, Math.ceil(point.x + 8)); cx += 2) {
            let matches = 0;
            for (let offset = -4; offset <= 4; offset += 1) {
                if (colorAt(cx + offset, cy + offset)) matches += 1;
                if (colorAt(cx + offset, cy - offset)) matches += 1;
            }
            best = Math.max(best, matches);
        }
    }
    return best >= 6;
}

module.exports = { drive, saveScreenshotFiles, hasHealthFill, hasRangeColorNear, hasWaypointColorNear, hasAttackCrossNear };
