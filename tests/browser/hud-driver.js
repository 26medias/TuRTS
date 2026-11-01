// turts-test-role: driver
"use strict";

const { decodePng, pixelAt } = require("./harness/png.js");
const { components } = require("./harness/hud4-pixels.js");

async function drive(io, params) {
    // Muster starts with its guide open. F1 is the real, physical guide toggle.
    await io.key("F1");

    let candidates = [];
    let before = null;
    let recognitionScreenshots = 0;
    for (let attempt = 0; attempt < 80; attempt += 1) {
        before = await io.screenshot();
        recognitionScreenshots += 1;
        if (attempt === 0) await io.save("hud1-initial.png", before);
        candidates = findMusterUnits(decodePng(before));
        if (candidates.length > 0) break;
        if (attempt < 79) await io.wait(100);
    }
    if (candidates.length === 0) {
        throw new Error("HUD1 could not identify a unit-shaped candidate from screenshot pixels; Muster recognition is NOT TRACED");
    }

    const selectionBox = selectionBoxFor(decodePng(before), candidates);
    await io.drag(selectionBox.start.x, selectionBox.start.y,
        selectionBox.end.x, selectionBox.end.y, "left");
    await io.wait(150);
    const after = await io.screenshot();
    await io.save("hud1-after-selection-box.png", after);

    await io.save("hud1-pixel-target.json", Buffer.from(JSON.stringify({
        candidates,
        selection_box: selectionBox,
        screenshot_ordinal: recognitionScreenshots,
        viewport: params.viewport,
        method: "pixel-only bounds around visible unit-material components; physical drag selection is verified from public captured state",
        recognition_status: "component roles are NOT TRACED; oracle must prove an owned rendered unit was selected",
    }) + "\n"));
}

function findMusterUnits(image) {
    const candidates = components(image)
        .filter((candidate) => {
            const aspect = candidate.height / candidate.width;
            return candidate.pixels >= 40 && candidate.width <= 80 && candidate.height <= 80 &&
                aspect >= 0.5 && aspect <= 1.8;
        })
        .sort((left, right) => right.pixels - left.pixels);
    return candidates.map((body) => ({
        x: Math.round((body.minX + body.maxX) / 2),
        y: Math.max(body.minY, body.maxY - Math.max(1, Math.round(body.height * 0.1))),
        bounds: [body.minX, body.minY, body.width, body.height],
        pixels: body.pixels,
        fill: body.fill,
    }));
}

function selectionBoxFor(image, candidates) {
    const clusters = componentClusters(candidates).filter((cluster) => cluster.length >= 2);
    const safe = [];
    const margin = 16;
    for (const cluster of clusters) {
        const minX = Math.min(...cluster.map((candidate) => candidate.bounds[0]));
        const minY = Math.min(...cluster.map((candidate) => candidate.bounds[1]));
        const maxX = Math.max(...cluster.map((candidate) => candidate.bounds[0] + candidate.bounds[2]));
        const maxY = Math.max(...cluster.map((candidate) => candidate.bounds[1] + candidate.bounds[3]));
        const rawStart = { x: Math.max(margin, minX - margin), y: Math.max(margin, minY - margin) };
        const rawEnd = { x: Math.min(image.width - margin, maxX + margin), y: Math.min(image.height - margin, maxY + margin) };
        const start = findGroundNear(image, rawStart);
        const end = findGroundNear(image, rawEnd);
        if (!start || !end || end.x - start.x < 8 || end.y - start.y < 8) continue;
        safe.push({ start, end, cluster, pixel_area: cluster.reduce((total, candidate) => total + candidate.pixels, 0) });
    }
    safe.sort((left, right) => right.cluster.length - left.cluster.length || right.pixel_area - left.pixel_area);
    const selected = safe[0];
    if (!selected) {
        throw new Error("HUD1 unit-material bounds do not yield a safe visible-ground selection box; Muster recognition is NOT TRACED");
    }
    return {
        start: selected.start,
        end: selected.end,
        material_components: selected.cluster.length,
        bounds: selected.cluster.reduce((bounds, candidate) => ({
            minX: Math.min(bounds.minX, candidate.bounds[0]),
            minY: Math.min(bounds.minY, candidate.bounds[1]),
            maxX: Math.max(bounds.maxX, candidate.bounds[0] + candidate.bounds[2]),
            maxY: Math.max(bounds.maxY, candidate.bounds[1] + candidate.bounds[3]),
        }), { minX: Infinity, minY: Infinity, maxX: -Infinity, maxY: -Infinity }),
    };
}

function componentClusters(candidates) {
    const parent = candidates.map((_, index) => index);
    const root = (index) => {
        while (parent[index] !== index) {
            parent[index] = parent[parent[index]];
            index = parent[index];
        }
        return index;
    };
    for (let left = 0; left < candidates.length; left += 1) {
        const a = candidates[left];
        const ax = a.x;
        const ay = a.y;
        for (let right = left + 1; right < candidates.length; right += 1) {
            const b = candidates[right];
            if (Math.hypot(ax - b.x, ay - b.y) > 72) continue;
            const leftRoot = root(left);
            const rightRoot = root(right);
            if (leftRoot !== rightRoot) parent[rightRoot] = leftRoot;
        }
    }
    const groups = new Map();
    for (let index = 0; index < candidates.length; index += 1) {
        const key = root(index);
        if (!groups.has(key)) groups.set(key, []);
        groups.get(key).push(candidates[index]);
    }
    return [...groups.values()];
}

function findGroundNear(image, origin) {
    const maxRadius = Math.min(96, Math.floor(Math.min(image.width, image.height) / 6));
    for (let radius = 0; radius <= maxRadius; radius += 1) {
        for (let dy = -radius; dy <= radius; dy += 1) {
            for (const dx of new Set([-radius, radius])) {
                const point = { x: origin.x + dx, y: origin.y + dy };
                if (isVisibleGround(image, point)) return point;
            }
        }
        for (let dx = -radius + 1; dx < radius; dx += 1) {
            for (const dy of new Set([-radius, radius])) {
                const point = { x: origin.x + dx, y: origin.y + dy };
                if (isVisibleGround(image, point)) return point;
            }
        }
    }
    return null;
}

function isVisibleGround(image, point) {
    const margin = 16;
    if (point.x < margin || point.y < margin ||
        point.x >= image.width - margin || point.y >= image.height - margin) return false;
    const pixel = pixelAt(image, point.x, point.y);
    if (!pixel || pixel[3] < 128) return false;
    const [r, g, b] = pixel;
    return g > r + 12 && g > b + 12 && g < 240;
}

module.exports = { drive, findMusterUnits, selectionBoxFor };
