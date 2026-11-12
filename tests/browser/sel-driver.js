// turts-test-role: driver
"use strict";

const { decodePng, pixelAt } = require("./harness/png.js");

const SLOT_COLORS = [[0, 114, 178], [230, 159, 0], [0, 158, 115], [213, 94, 0], [86, 180, 233], [204, 121, 167], [240, 228, 66], [0, 0, 0]];
const COLOR_TOLERANCE = 60;

async function drive(io, params) {
    if (!Array.isArray(params.stages) || params.stages.length === 0) throw new Error("SEL driver needs row stages");
    await io.key("F1");
    let initialPng = Buffer.from(await io.screenshot());
    let before = decodePng(initialPng);
    let sel3TargetPoint = null;
    if (params.stages.some((stage) => stage.label?.startsWith("sel3-"))) {
        const initialTargets = deriveTargets(before);
        const trooperColumn = trooperColumnFromPixels(initialTargets.slot0_bodies);
        if (!trooperColumn) throw new Error("SEL3 initial PNG has no trooper-column pixel target");
        const center = { x: Math.round(before.width / 2), y: Math.round(before.height / 2) };
        const foot = footPoint(trooperColumn);
        await io.drag(foot.x, foot.y, center.x, center.y, "middle");
        await io.wait(250);
        await io.wheel(-1200);
        await io.wait(250);
        const zoomed = decodePng(Buffer.from(await io.screenshot()));
        const centeredTrooper = nearestPalettePixel(zoomed, center, SLOT_COLORS[0], 18);
        if (!centeredTrooper) throw new Error("SEL3 zoomed PNG has no own-colour trooper at viewport center");
        sel3TargetPoint = { x: zoomed.width - 160, y: Math.round(zoomed.height / 2) };
        await io.drag(centeredTrooper.x, centeredTrooper.y, sel3TargetPoint.x, sel3TargetPoint.y, "middle");
        await io.wait(250);
        initialPng = Buffer.from(await io.screenshot());
        before = decodePng(initialPng);
        if (!nearestPalettePixel(before, sel3TargetPoint, SLOT_COLORS[0], 10)) {
            throw new Error("SEL3 final PNG does not show an own-colour body at its camera target");
        }
    }
    const targets = deriveTargets(before);
    targets.trooper_target_point = sel3TargetPoint;
    await io.save("sel-initial.png", initialPng);
    await io.save("sel-initial.png.b64", Buffer.from(initialPng.toString("base64")));
    await io.save("sel-pixel-targets.json", encodeJson(targets));
    for (const stage of params.stages) {
        const points = selectorPoints(stage.selector, targets, before, stage.gesture);
        if (stage.gesture === "double-click") {
            // One harness action sends both clicks back to back (count 2), so a loaded host cannot
            // stretch the gap between them past the 400 ms double-click window (RUN130: 468 ms).
            await io.click(points.start.x, points.start.y, "left", { count: 2 });
        } else if (stage.gesture === "click") {
            await withModifiers(io, stage.modifiers || [], async () => io.click(points.start.x, points.start.y, "left"));
        } else if (stage.gesture === "box") {
            await withModifiers(io, stage.modifiers || [], async () => io.drag(points.start.x, points.start.y, points.end.x, points.end.y, "left"));
        } else {
            throw new Error(`SEL ${stage.label} has unsupported gesture ${stage.gesture}`);
        }
        const captured = Buffer.from(await io.screenshot());
        await io.save(`${stage.label}.png`, captured);
        await io.save(`${stage.label}.png.b64`, Buffer.from(captured.toString("base64")));
    }
}

function deriveTargets(image) {
    const components = SLOT_COLORS.map((color, slot) => ({ slot, color, components: colorComponents(image, color) }));
    const slot0 = components[0].components;
    const ownBodies = slot0.filter(isBodyComponent).sort((a, b) => b.pixels - a.pixels);
    if (ownBodies.length === 0) throw new Error("SEL screenshot has no slot-0 pixels matching the authored palette; pixel targets are NOT TRACED");
    const ownBounds = boundsOf(ownBodies);
    const allBodies = components.flatMap((entry) => entry.components.filter(isBodyComponent).map((body) => ({ ...body, slot: entry.slot })));
    return {
        viewport: [image.width, image.height],
        slot0_bodies: ownBodies,
        all_slot_bodies: allBodies,
        slot0_bounds: ownBounds,
        slot0_pixels: colorBounds(image, SLOT_COLORS[0]),
        all_slot_pixels: SLOT_COLORS.map((color) => colorBounds(image, color)).filter(Boolean),
    };
}

function selectorPoints(selector, targets, image, gesture) {
    if (selector === "all-visible-slot0") return boxAroundPixels(targets.slot0_pixels, image, 8);
    if (selector === "500-mobile-clusters") return boxAround(clusteredBodies(targets.slot0_bodies, 36), image, 8);
    if (selector === "500-mobile-plus-visible-foreign") {
        const own = clusteredBodies(targets.slot0_bodies, 36);
        const foreign = targets.all_slot_bodies.filter((entry) => entry.slot === 1);
        if (!foreign.length) throw new Error("SEL screenshot has no slot-1 body pixels; mixed-owner target is NOT TRACED");
        return boxAround([...own, ...foreign], image, 8);
    }
    if (selector === "static-footprints") {
        const square = targets.slot0_bodies.filter((entry) => {
            const aspect = entry.height / entry.width;
            return entry.pixels >= 40 && entry.width >= 10 && entry.height >= 8 && aspect >= 0.65 && aspect <= 1.45;
        }).sort((a, b) => b.pixels - a.pixels).slice(0, 2);
        if (square.length !== 2) throw new Error("SEL screenshot does not expose two isolated static-looking silhouettes; static pixel targets are NOT TRACED");
        return unionBox(square, image, 1);
    }
    if (selector === "mobile-slot0") {
        const mobile = targets.slot0_bodies.filter((entry) => {
            const aspect = entry.height / entry.width;
            return entry.pixels >= 30 && entry.pixels <= 150 && entry.width >= 8 && entry.width <= 14 &&
                entry.height >= 8 && entry.height <= 14 && aspect >= 0.75 && aspect <= 1.35;
        }).sort((a, b) => a.x - b.x)[0];
        if (!mobile) throw new Error("SEL screenshot has no isolated round slot-0 silhouette; mobile pixel target is NOT TRACED");
        return gesture === "box" ? unionBox([mobile], image, 3) : { start: footPoint(mobile), end: footPoint(mobile) };
    }
    if (selector === "trooper-column-slot0") {
        if (!targets.trooper_target_point) throw new Error("SEL3 has no screenshot-derived trooper target");
        return { start: targets.trooper_target_point, end: targets.trooper_target_point };
    }
    if (selector === "first-visible-slot0") {
        const body = targets.slot0_bodies[0];
        return gesture === "box" ? unionBox([body], image, 3) : { start: footPoint(body), end: footPoint(body) };
    }
    if (selector === "second-visible-slot0") {
        const body = targets.slot0_bodies[1];
        return { start: footPoint(body), end: footPoint(body) };
    }
    if (selector === "separate-visible-slot0") {
        const body = targets.slot0_bodies[2];
        if (!body) throw new Error("SEL screenshot has no second disjoint slot-0 target; pixel target is NOT TRACED");
        return { start: footPoint(body), end: footPoint(body) };
    }
    throw new Error(`SEL row has unknown screenshot-derived selector ${selector}`);
}

function trooperColumnFromPixels(entries) {
    return entries.filter((entry) => {
        const aspect = entry.height / entry.width;
        return entry.pixels >= 400 && entry.pixels <= 1_000 && entry.width >= 10 && entry.width <= 18 &&
            entry.height >= 40 && entry.height <= 70 && aspect >= 2.5 && aspect <= 6;
    }).sort((a, b) => Math.abs(a.x - 640) - Math.abs(b.x - 640))[0];
}

function nearestPalettePixel(image, point, color, radius) {
    let nearest = null;
    let nearestDistance = Infinity;
    for (let y = Math.max(0, Math.floor(point.y - radius)); y <= Math.min(image.height - 1, Math.ceil(point.y + radius)); y += 1) {
        for (let x = Math.max(0, Math.floor(point.x - radius)); x <= Math.min(image.width - 1, Math.ceil(point.x + radius)); x += 1) {
            const pixel = pixelAt(image, x, y);
            if (!pixel || pixel[3] < 245 || Math.hypot(pixel[0] - color[0], pixel[1] - color[1], pixel[2] - color[2]) > COLOR_TOLERANCE) continue;
            const distance = Math.hypot(x - point.x, y - point.y);
            if (distance < nearestDistance) { nearest = { x, y }; nearestDistance = distance; }
        }
    }
    return nearest;
}

function colorComponents(image, color) {
    const size = image.width * image.height;
    const seen = new Uint8Array(size);
    const queue = new Int32Array(size);
    const out = [];
    const matches = (index) => {
        const x = index % image.width;
        const y = Math.floor(index / image.width);
        if (isHudPixel(x, y, image)) return false;
        const offset = index * 4;
        const rgb = [image.rgba[offset], image.rgba[offset + 1], image.rgba[offset + 2]];
        return image.rgba[offset + 3] >= 245 && Math.hypot(rgb[0] - color[0], rgb[1] - color[1], rgb[2] - color[2]) <= COLOR_TOLERANCE;
    };
    for (let index = 0; index < size; index += 1) {
        if (seen[index] || !matches(index)) continue;
        let tail = 1;
        queue[0] = index;
        seen[index] = 1;
        let minX = image.width, minY = image.height, maxX = 0, maxY = 0;
        for (let head = 0; head < tail; head += 1) {
            const current = queue[head];
            const x = current % image.width;
            const y = Math.floor(current / image.width);
            minX = Math.min(minX, x); minY = Math.min(minY, y); maxX = Math.max(maxX, x); maxY = Math.max(maxY, y);
            for (const next of [x ? current - 1 : -1, x + 1 < image.width ? current + 1 : -1, y ? current - image.width : -1, y + 1 < image.height ? current + image.width : -1]) {
                if (next >= 0 && !seen[next] && matches(next)) { seen[next] = 1; queue[tail++] = next; }
            }
        }
        const width = maxX - minX + 1;
        const height = maxY - minY + 1;
        if (tail >= 20 && width >= 4 && height >= 4 && width <= 100 && height <= 100) {
            out.push({ x: (minX + maxX) / 2, y: (minY + maxY) / 2, minX, minY, maxX, maxY, width, height, pixels: tail });
        }
    }
    return out;
}

function colorBounds(image, color) {
    let minX = image.width, minY = image.height, maxX = -1, maxY = -1;
    for (let y = 0; y < image.height; y += 1) for (let x = 0; x < image.width; x += 1) {
        const pixel = pixelAt(image, x, y);
        if (!pixel || pixel[3] < 245 || Math.hypot(pixel[0] - color[0], pixel[1] - color[1], pixel[2] - color[2]) > COLOR_TOLERANCE) continue;
        if (isHudPixel(x, y, image)) continue;
        minX = Math.min(minX, x); minY = Math.min(minY, y); maxX = Math.max(maxX, x); maxY = Math.max(maxY, y);
    }
    return maxX >= minX && maxY >= minY ? { minX, minY, maxX, maxY } : null;
}

function isHudPixel(x, y, image) {
    return (x < 300 && y < 330) ||
        (x > image.width - 320 && y < 180) ||
        (x > image.width - 220 && y > image.height - 220) ||
        (x < 380 && y > image.height - 140) ||
        (x > image.width / 2 - 100 && x < image.width / 2 + 100 && y > image.height - 32) ||
        y > image.height - 40;
}

function isBodyComponent(entry) {
    return entry.pixels >= 20 && entry.width >= 4 && entry.height >= 4 && entry.width <= 100 && entry.height <= 100;
}

function boundsOf(entries) {
    return entries.reduce((bounds, entry) => ({ minX: Math.min(bounds.minX, entry.minX), minY: Math.min(bounds.minY, entry.minY), maxX: Math.max(bounds.maxX, entry.maxX), maxY: Math.max(bounds.maxY, entry.maxY) }),
        { minX: Infinity, minY: Infinity, maxX: -Infinity, maxY: -Infinity });
}

function unionBox(entries, image, margin) {
    const bounds = boundsOf(entries);
    const start = { x: Math.max(1, Math.floor(bounds.minX - margin)), y: Math.max(1, Math.floor(bounds.minY - margin)) };
    const end = { x: Math.min(image.width - 2, Math.ceil(bounds.maxX + margin)), y: Math.min(image.height - 2, Math.ceil(bounds.maxY + margin)) };
    if (end.x - start.x < 4 || end.y - start.y < 4) throw new Error("SEL screenshot-derived box is too small");
    return { start, end };
}

function boxAround(entries, image, margin) {
    if (!entries || entries.length === 0) throw new Error("SEL screenshot has no isolated body components for this box");
    return unionBox(entries, image, margin);
}

function boxAroundPixels(entries, image, margin) {
    if (!entries || entries.length === 0) throw new Error("SEL screenshot contains no visible unit-colour pixels for this box");
    const bounds = entries.reduce((all, entry) => ({ minX: Math.min(all.minX, entry.minX), minY: Math.min(all.minY, entry.minY), maxX: Math.max(all.maxX, entry.maxX), maxY: Math.max(all.maxY, entry.maxY) }),
        { minX: Infinity, minY: Infinity, maxX: -Infinity, maxY: -Infinity });
    return unionBox([{ ...bounds, x: 0, y: 0, width: 0, height: 0, pixels: 1 }], image, margin);
}

function clusteredBodies(entries, distance) {
    const parent = entries.map((_, index) => index);
    const root = (index) => {
        while (parent[index] !== index) { parent[index] = parent[parent[index]]; index = parent[index]; }
        return index;
    };
    for (let left = 0; left < entries.length; left += 1) for (let right = left + 1; right < entries.length; right += 1) {
        const a = entries[left]; const b = entries[right];
        if (Math.hypot(a.x - b.x, a.y - b.y) > distance) continue;
        const aRoot = root(left); const bRoot = root(right);
        if (aRoot !== bRoot) parent[bRoot] = aRoot;
    }
    const groups = new Map();
    entries.forEach((entry, index) => {
        const key = root(index);
        if (!groups.has(key)) groups.set(key, []);
        groups.get(key).push(entry);
    });
    const selected = [...groups.values()].sort((a, b) =>
        b.reduce((sum, entry) => sum + entry.pixels, 0) - a.reduce((sum, entry) => sum + entry.pixels, 0))[0];
    if (!selected || selected.length < 2) throw new Error("SEL screenshot has no multi-component own-colour cluster; pixel target is NOT TRACED");
    return selected;
}
function footPoint(body) { return { x: Math.round(body.x), y: Math.round(body.maxY - Math.max(1, body.height * 0.08)) }; }
function inside(x, y, bounds) { return x >= bounds.minX && x <= bounds.maxX && y >= bounds.minY && y <= bounds.maxY; }
function encodeJson(value) { return Buffer.from(JSON.stringify(value) + "\n"); }

async function withModifiers(io, modifiers, action) {
    for (const modifier of modifiers) await io.key(modifier, [], { action: "down" });
    try { await action(); }
    finally { for (const modifier of [...modifiers].reverse()) await io.key(modifier, [], { action: "up" }); }
}

module.exports = { drive };
