// turts-test-role: pure
"use strict";

const { pixelAt } = require("./png.js");

function readTargets(image) {
    const candidates = components(image);
    const classified = { attacker: [], worker: [], foreign: [], node: [] };
    for (const candidate of candidates) {
        const role = classify(candidate);
        if (role) classified[role].push(candidate);
    }
    if (Object.values(classified).some((matches) => matches.length !== 1)) return null;
    const targets = Object.fromEntries(Object.entries(classified).map(([role, matches]) => [role, foot(matches[0], role)]));
    const ground = findOpenGround(image, candidates.concat(Object.values(classified).flat()));
    if (!ground) return null;
    return { targets, ground, candidates };
}

function components(image) {
    const count = image.width * image.height;
    const visited = new Uint8Array(count);
    const queue = new Int32Array(count);
    const found = [];
    for (let origin = 0; origin < count; origin += 1) {
        if (visited[origin] || !isUnitMaterialAt(image, origin)) continue;
        let tail = 1;
        queue[0] = origin;
        visited[origin] = 1;
        let minX = image.width;
        let minY = image.height;
        let maxX = -1;
        let maxY = -1;
        for (let head = 0; head < tail; head += 1) {
            const index = queue[head];
            const x = index % image.width;
            const y = Math.floor(index / image.width);
            minX = Math.min(minX, x); minY = Math.min(minY, y);
            maxX = Math.max(maxX, x); maxY = Math.max(maxY, y);
            if (x > 0) tail = visit(index - 1, tail);
            if (x + 1 < image.width) tail = visit(index + 1, tail);
            if (y > 0) tail = visit(index - image.width, tail);
            if (y + 1 < image.height) tail = visit(index + image.width, tail);
        }
        const width = maxX - minX + 1;
        const height = maxY - minY + 1;
        if (tail >= 8 && width >= 3 && height >= 3 && width <= 120 && height <= 120) {
            found.push({ minX, minY, maxX, maxY, width, height, pixels: tail, fill: tail / (width * height) });
        }

        function visit(index, currentTail) {
            if (visited[index] || !isUnitMaterialAt(image, index)) return currentTail;
            visited[index] = 1;
            queue[currentTail] = index;
            return currentTail + 1;
        }
    }
    return found;
}

function classify(candidate) {
    const aspect = candidate.height / candidate.width;
    // Fixture visual geometry is purposefully distinct; roles are silhouettes only here.
    // Perspective makes the authored capsule much less tall than its world-space shape.
    // The resource block plus its world label is a broad component, while the unit roles
    // remain compact in the actual 1280x720 canvas capture.
    if (aspect >= 1.4 && candidate.pixels >= 200) return "attacker";
    if (aspect <= 0.45 && candidate.pixels >= 1000) return "node";
    if (aspect >= 0.65 && aspect <= 1.5 && candidate.pixels >= 700) return "foreign";
    if (aspect >= 0.65 && aspect <= 1.5 && candidate.pixels >= 100 && candidate.pixels < 700) return "worker";
    return null;
}

function foot(candidate, role) {
    return {
        x: Math.round((candidate.minX + candidate.maxX) / 2),
        y: role === "node"
            ? Math.round(candidate.minY + candidate.height * 0.3)
            : Math.max(candidate.minY, candidate.maxY - Math.max(1, Math.round(candidate.height * 0.08))),
        bounds: [candidate.minX, candidate.minY, candidate.width, candidate.height],
        pixels: candidate.pixels,
        fill: candidate.fill,
    };
}

function findOpenGround(image, targets) {
    const bounds = targets.map(({ minX, minY, maxX, maxY }) => ({ minX: minX - 24, minY: minY - 24, maxX: maxX + 24, maxY: maxY + 24 }));
    const center = { x: Math.floor(image.width / 2), y: Math.floor(image.height * 0.55) };
    if (isOpenGround(image, center, bounds)) return center;
    for (let y = Math.floor(image.height * 0.28); y < image.height * 0.84; y += 5) {
        for (let x = Math.floor(image.width * 0.2); x < image.width * 0.8; x += 5) {
            const candidate = { x, y };
            if (isOpenGround(image, candidate, bounds)) return candidate;
        }
    }
    return null;
}

function isOpenGround(image, point, bounds) {
    if (bounds.some((rect) => point.x >= rect.minX && point.x <= rect.maxX && point.y >= rect.minY && point.y <= rect.maxY)) return false;
    const rgba = pixelAt(image, point.x, point.y);
    if (!rgba || rgba[3] < 128 || isUnitMaterial(rgba) || isNodeMaterial(rgba)) return false;
    const [r, g, b] = rgba;
    // Muster's visible floor is green; the dark gray outside the camera frustum is
    // backdrop, even though both are neutral, unoccupied pixels.
    return g >= 90 && g > r * 1.18 && g > b * 1.18;
}

function isUnitMaterial(rgba) {
    if (!rgba || rgba[3] < 128) return false;
    const [r, g, b] = rgba;
    return (r > g * 1.28 && r > b * 1.35) || (b > r * 1.3 && b > g * 1.25) || (r > g * 1.12 && g > b * 1.35);
}

function isUnitMaterialAt(image, pixelIndex) {
    const offset = pixelIndex * 4;
    const r = image.rgba[offset];
    const g = image.rgba[offset + 1];
    const b = image.rgba[offset + 2];
    const a = image.rgba[offset + 3];
    return a >= 128 && ((r > g * 1.28 && r > b * 1.35) || (b > r * 1.3 && b > g * 1.25) || (r > g * 1.12 && g > b * 1.35));
}

function isNodeMaterial(rgba) {
    if (!rgba || rgba[3] < 128) return false;
    const [r, g, b] = rgba;
    return r > g * 1.08 && g > b * 1.3;
}

module.exports = { readTargets, components, classify, foot, isOpenGround, isNodeMaterial, isUnitMaterial };
