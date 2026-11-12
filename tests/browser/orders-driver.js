// turts-test-role: driver
"use strict";

const { decodePng, pixelAt } = require("./harness/png.js");

const BLUE = [0, 114, 178];
const ORANGE = [230, 159, 0];

async function drive(io, params) {
    const row = params.row;
    if (!["ORD3", "ORD4", "ORD5"].includes(row)) throw new Error(`no order driver for ${row}`);
    const size = params.minimap_size, inset = params.minimap_inset;
    const rect = { x: params.viewport.width - inset - size, y: params.viewport.height - inset - size, size };
    const screen = ([x, y]) => [Math.round(rect.x + size * x / params.map_size[0]),
        Math.round(rect.y + size * (1 - y / params.map_size[1]))];
    const steps = { row, rect, actions: [], pixels: {} };
    async function shot(label) {
        const bytes = Buffer.from(await io.screenshot());
        await io.save(`${row.toLowerCase()}-${label}.png`, bytes);
        await io.save(`${row.toLowerCase()}-${label}.png.b64`, Buffer.from(bytes.toString("base64"), "utf8"));
        steps.actions.push({ label, call: "screenshot", window: 1 });
        return decodePng(bytes);
    }
    async function click(label, point, button = "right") {
        await io.click(point[0], point[1], button);
        steps.actions.push({ label, call: "click", point, button });
        await io.wait(150);
    }
    async function key(label, code, modifiers = [], options = {}) {
        await io.key(code, modifiers, options);
        steps.actions.push({ label, call: "key", code, modifiers, options });
        await io.wait(100);
    }
    async function minimap(label, world, button = "right", settleMs = 150) {
        const point = screen(world);
        steps.actions.push({ label, call: "click", world, point, button, local: [world[0] / params.map_size[0], 1 - world[1] / params.map_size[1]] });
        await io.click(point[0], point[1], button);
        if (settleMs > 0) await io.wait(settleMs);
        return point;
    }
    // The first join shows the "Click to play" overlay, which dims the scene until a gesture
    // (client_shell.rs sync_focus_overlay, since 3d0cf2f). A keydown is the gesture
    // (crates/turts_client_web/index.html:39-40); F9 has no binding. It is not an order step.
    await io.key("F9");
    await io.wait(350);
    let image = await shot("before");
    if (row === "ORD5") {
        const worker = params.stages.worker_pixel;
        assertSlotPixel(image, worker, BLUE, "worker selection seed");
        const trooper = exposedTrooper(image);
        steps.pixels.worker = worker;
        steps.pixels.trooper = trooper;
        await click("select-worker", worker, "left");
        await key("shift-select-down", "ShiftLeft", [], { action: "down" });
        await click("shift-select-trooper", [trooper.x, trooper.y], "left");
        await key("shift-select-up", "ShiftLeft", [], { action: "up" });
        await shot("selected");
        for (const [label, target] of [["ground", params.stages.ground], ["node", params.stages.node], ["foreign", params.stages.foreign]]) {
            const point = await minimap(`smart-${label}`, target);
            await io.wait(800);
            await shot(`smart-${label}`);
            steps.actions.at(-1).interaction_point = point;
        }
    } else {
        const own = exposedTrooper(image);
        steps.pixels.trooper = own;
        await click("select-trooper", [own.x, own.y], "left");
        await shot("selected");
    if (row === "ORD3") await driveOrd3(io, params, steps, minimap, shot, click, key);
        if (row === "ORD4") await driveOrd4(io, params, steps, minimap, shot, click, key);
    }
    await io.save(`${row.toLowerCase()}-driver-steps.json`, Buffer.from(JSON.stringify(steps, null, 2) + "\n"));
}

async function driveOrd3(io, params, steps, minimap, shot, click, key) {
    const targets = params.stages;
    await key("route-shift-down", "ShiftLeft", [], { action: "down" });
    for (let index = 0; index < 3; index += 1) await minimap(`route-append-${index + 1}`, targets.route[index]);
    await key("route-shift-up", "ShiftLeft", [], { action: "up" });
    await shot("route-queued");
    await io.wait(targets.route_wait_ms);
    await shot("route-complete");

    await minimap("replace-tail-first", targets.replace[0]);
    await key("replace-shift-down", "ShiftLeft", [], { action: "down" });
    await minimap("replace-tail-second", targets.replace[1]);
    await key("replace-shift-up", "ShiftLeft", [], { action: "up" });
    await shot("replace-before");
    await minimap("plain-replace", targets.replace[2]);
    await shot("replace-after");

    await minimap("stop-prequeue-first", targets.stop[0]);
    await key("stop-prequeue-shift-down", "ShiftLeft", [], { action: "down" });
    await minimap("stop-prequeue-second", targets.stop[1]);
    await key("stop-prequeue-shift-up", "ShiftLeft", [], { action: "up" });
    await shot("stop-before");
    await key("stop-order", "KeyS");
    await io.wait(500);
    await shot("stop-after");

    await minimap("hold-approach", targets.hold_point);
    await io.wait(targets.hold_approach_wait_ms);
    await shot("hold-context");
    await key("hold-order", "KeyH");
    await io.wait(targets.hold_fire_wait_ms);
    await shot("hold-after");
    await key("hold-shift-down", "ShiftLeft", [], { action: "down" });
    await minimap("append-behind-hold", targets.stop[2]);
    await key("hold-shift-up", "ShiftLeft", [], { action: "up" });
    await shot("hold-refusal");

    const queueBase = targets.queue[0];
    await minimap("queue-start", queueBase);
    await io.wait(200);
    await key("overflow-shift-down", "ShiftLeft", [], { action: "down" });
    // No wait between the 17 appends: each click already takes 0.3 to 0.9 s on a loaded host, and the
    // active point must stay unreached until the last of them (RUN130: 17.8 s against a 15.9 s walk).
    for (let i = 0; i < 17; i += 1) await minimap(`queue-overflow-${i + 1}`, targets.queue[1 + (i % (targets.queue.length - 1))], "right", 0);
    await key("overflow-shift-up", "ShiftLeft", [], { action: "up" });
    await shot("queue-overflow-final");

    if (targets.gather_worker_pixel) {
        const image = await shot("gather-worker-context");
        assertSlotPixel(image, targets.gather_worker_pixel, BLUE, "gather worker seed");
        await click("select-gather-worker", targets.gather_worker_pixel, "left");
        await key("gather-order", "KeyG");
        const node = oreNodePixel(image);
        steps.pixels.node = node;
        await click("gather-node", [node.x, node.y], "left");
        await key("gather-shift-down", "ShiftLeft", [], { action: "down" });
        await minimap("append-behind-gather", targets.queue[0]);
        await key("gather-shift-up", "ShiftLeft", [], { action: "up" });
        await shot("gather-accepted");
        await io.wait(targets.gather_wait_ms);
        await shot("gather-depletion-visible");
        await shot("gather-depleted");
    }
}

async function driveOrd4(io, params, steps, minimap, shot, click, key) {
    const stages = params.stages;
    await minimap("attack-view", stages.attack_view, "left");
    const attackView = await shot("attack-view-ready");
    const attackTarget = closestForeign(attackView);
    steps.pixels.attack_target = attackTarget;
    await key("attack-mode", "KeyA");
    await click("attack-selected-unit", [attackTarget.x, attackTarget.y], "left");
    await shot("attack-issued");
    await io.wait(stages.attack_wait_ms);
    await shot("attack-after-approach");

    const postAttack = await shot("rush-context");
    const rushGround = findGroundNear(postAttack, attackTarget.x + 120, attackTarget.y);
    steps.pixels.rush_ground = rushGround;
    await key("rush-mode", "KeyQ");
    await click("rush-target", [rushGround.x, rushGround.y], "left");
    await shot("rush-issued");
    await io.wait(2000);
    await shot("rush-engagement");
    await io.wait(stages.rush_wait_ms);
    await shot("rush-paused-or-resumed");

    await minimap("force-safe-approach", stages.force_safe_point);
    await io.wait(stages.force_safe_wait_ms);
    await shot("force-safe-arrived");

    const forceContext = await shot("force-context");
    const forceTarget = closestForeign(forceContext);
    steps.pixels.force_target = forceTarget;
    await key("force-move-mode", "KeyM");
    await click("force-move-destination", [forceTarget.x, forceTarget.y], "left");
    await shot("force-issued");
    await io.wait(stages.force_wait_ms);
    await shot("force-move-arrival");
}

function assertSlotPixel(image, point, rgb, label) {
    const pixel = pixelAt(image, point[0], point[1]);
    if (!pixel || pixel[3] < 245 || Math.hypot(pixel[0] - rgb[0], pixel[1] - rgb[1], pixel[2] - rgb[2]) > 64) {
        throw new Error(`${label} does not match a current screenshot pixel`);
    }
}

function exposedTrooper(image) {
    const bodies = components(image, BLUE).filter((body) => {
        const width = body.maxX - body.minX + 1, height = body.maxY - body.minY + 1;
        return body.pixels >= 450 && body.pixels <= 750 && width >= 8 && width <= 16 && height >= 45 && height <= 70
            && body.minY > 180 && body.maxY < 500;
    }).sort((a, b) => b.maxX - a.maxX || b.maxY - a.maxY);
    const body = bodies[0];
    if (!body) throw new Error("current screenshot has no exposed trooper column component");
    return { x: Math.round((body.minX + body.maxX) / 2), y: body.maxY - 2 };
}

function outerForeign(image) {
    const bodies = components(image, ORANGE, 64).filter((body) => body.pixels >= 30 && body.maxX < image.width - 210 && body.minY > 180)
        .sort((a, b) => a.minX - b.minX || a.minY - b.minY);
    const body = bodies[0];
    if (!body) throw new Error("current screenshot has no foreign orange body component after camera focus");
    return { x: Math.round((body.minX + body.maxX) / 2), y: Math.round((body.minY + body.maxY) / 2) };
}

function closestForeign(image) {
    const bodies = components(image, ORANGE, 64).filter((body) => body.pixels >= 30 && body.minY > 180
        && body.maxX < image.width - 210 && body.maxY < image.height - 220);
    const cx = image.width / 2, cy = image.height / 2;
    const body = bodies.sort((a, b) => Math.hypot((a.minX + a.maxX) / 2 - cx, (a.minY + a.maxY) / 2 - cy)
        - Math.hypot((b.minX + b.maxX) / 2 - cx, (b.minY + b.maxY) / 2 - cy))[0];
    if (!body) throw new Error("current screenshot has no foreign orange body near the main view centre");
    let best = null;
    const center = { x: (body.minX + body.maxX) / 2, y: (body.minY + body.maxY) / 2 };
    for (let y = body.minY; y <= body.maxY; y += 1) for (let x = body.minX; x <= body.maxX; x += 1) {
        const pixel = pixelAt(image, x, y);
        if (!pixel || pixel[3] < 245 || Math.hypot(pixel[0] - ORANGE[0], pixel[1] - ORANGE[1], pixel[2] - ORANGE[2]) > 64) continue;
        const candidate = { x, y, distance: Math.hypot(x - center.x, y - center.y) };
        if (!best || candidate.distance < best.distance) best = candidate;
    }
    if (!best) throw new Error("foreign body component has no verified current orange pixel");
    return { x: best.x, y: best.y };
}

function oreNodePixel(image) {
    const candidates = components(image, [215, 188, 95], 65).filter((body) => body.pixels >= 30 && body.pixels <= 600
        && body.minY > 180 && body.maxX < image.width - 220 && body.maxY < image.height - 220);
    const body = candidates.sort((a, b) => a.minY - b.minY || a.minX - b.minX)[0];
    if (!body) throw new Error("current screenshot has no bounded gold ore-node pixel component");
    return { x: Math.round((body.minX + body.maxX) / 2), y: body.maxY - 1 };
}

function findGroundNear(image, x, y) {
    const candidates = [];
    for (let py = Math.max(180, Math.round(y - 20)); py <= Math.min(image.height - 220, Math.round(y + 20)); py += 1) {
        for (let px = Math.max(10, Math.round(x - 20)); px <= Math.min(image.width - 220, Math.round(x + 20)); px += 1) {
            const pixel = pixelAt(image, px, py);
            if (pixel && pixel[3] >= 245 && Math.hypot(pixel[0] - 104, pixel[1] - 148, pixel[2] - 100) <= 44) {
                candidates.push({ x: px, y: py, distance: Math.hypot(px - x, py - y) });
            }
        }
    }
    candidates.sort((a, b) => a.distance - b.distance);
    if (!candidates.length) throw new Error("current screenshot has no visible open-ground pixel near the planned Rush point");
    return candidates[0];
}

async function waitForPixelGone(io, target, colour, label) {
    const deadline = Date.now() + 30000;
    let absentTwice = 0, sawPresent = false;
    while (Date.now() < deadline) {
        const image = decodePng(Buffer.from(await io.screenshot()));
        let matches = 0;
        for (let y = target.y - 2; y <= target.y + 2; y += 1) for (let x = target.x - 2; x <= target.x + 2; x += 1) {
            const pixel = pixelAt(image, x, y);
            if (pixel && pixel[3] >= 245 && Math.hypot(pixel[0] - colour[0], pixel[1] - colour[1], pixel[2] - colour[2]) <= 64) matches += 1;
        }
        if (matches > 0) sawPresent = true;
        absentTwice = matches === 0 ? absentTwice + 1 : 0;
        if (sawPresent && absentTwice >= 2) return;
        await io.wait(250);
    }
    throw new Error(`${label} did not disappear from its original screenshot-pixel footprint within 30 seconds`);
}

function components(image, colour, tolerance = 48) {
    const seen = new Uint8Array(image.width * image.height), queue = new Int32Array(image.width * image.height), output = [];
    const match = (index) => {
        const x = index % image.width, y = Math.floor(index / image.width);
        if (y > image.height - 220 || (x > image.width - 220 && y > image.height - 220)) return false;
        const p = index * 4;
        return image.rgba[p + 3] >= 245 && Math.hypot(image.rgba[p] - colour[0], image.rgba[p + 1] - colour[1], image.rgba[p + 2] - colour[2]) <= tolerance;
    };
    for (let i = 0; i < seen.length; i += 1) {
        if (seen[i] || !match(i)) continue;
        let tail = 1, minX = image.width, minY = image.height, maxX = 0, maxY = 0;
        queue[0] = i; seen[i] = 1;
        for (let head = 0; head < tail; head += 1) {
            const at = queue[head], x = at % image.width, y = Math.floor(at / image.width);
            minX = Math.min(minX, x); minY = Math.min(minY, y); maxX = Math.max(maxX, x); maxY = Math.max(maxY, y);
            for (const next of [x ? at - 1 : -1, x + 1 < image.width ? at + 1 : -1, y ? at - image.width : -1, y + 1 < image.height ? at + image.width : -1]) {
                if (next >= 0 && !seen[next] && match(next)) { seen[next] = 1; queue[tail++] = next; }
            }
        }
        if (tail >= 20 && maxX - minX >= 3 && maxY - minY >= 3) output.push({ minX, minY, maxX, maxY, pixels: tail });
    }
    return output;
}

module.exports = { drive, exposedTrooper, components };
