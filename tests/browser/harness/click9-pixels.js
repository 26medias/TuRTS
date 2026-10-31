// turts-test-role: pure
"use strict";

const { pixelAt } = require("./png.js");

function unitColour(r, g, b, a) {
    // Acme has one scout. Accept its legacy warm material and the later blue slot palette.
    return a >= 128 && ((r >= 12 && r > g * 1.35 && r > b * 1.4) || (b >= 50 && b > r * 1.4 && b > g * 1.1));
}

function findScout(image) {
    const seen = new Uint8Array(image.width * image.height);
    const queue = new Int32Array(seen.length);
    const candidates = [];
    const coloured = (index) => {
        const offset = index * 4;
        return unitColour(image.rgba[offset], image.rgba[offset + 1], image.rgba[offset + 2], image.rgba[offset + 3]);
    };
    for (let index = 0; index < seen.length; index += 1) {
        if (seen[index] || !coloured(index)) continue;
        let tail = 1;
        queue[0] = index;
        seen[index] = 1;
        let minX = image.width;
        let minY = image.height;
        let maxX = 0;
        let maxY = 0;
        for (let head = 0; head < tail; head += 1) {
            const current = queue[head];
            const x = current % image.width;
            const y = Math.floor(current / image.width);
            minX = Math.min(minX, x); maxX = Math.max(maxX, x);
            minY = Math.min(minY, y); maxY = Math.max(maxY, y);
            const adjacent = [];
            if (x) adjacent.push(current - 1);
            if (x + 1 < image.width) adjacent.push(current + 1);
            if (y) adjacent.push(current - image.width);
            if (y + 1 < image.height) adjacent.push(current + image.width);
            for (const next of adjacent) {
                if (!seen[next] && coloured(next)) {
                    seen[next] = 1;
                    queue[tail++] = next;
                }
            }
        }
        const width = maxX - minX + 1;
        const height = maxY - minY + 1;
        if (tail < 12 || width < 3 || height < 3 || width > 80 || height > 80) continue;
        // Ground-plane picking uses the foot of the visible cuboid, not its elevated bright top.
        const desiredX = (minX + maxX) / 2;
        const desiredY = maxY - Math.max(1, height * 0.1);
        let closest = null;
        let distance = Infinity;
        for (let item = 0; item < tail; item += 1) {
            const x = queue[item] % image.width;
            const y = Math.floor(queue[item] / image.width);
            const squared = (x - desiredX) ** 2 + (y - desiredY) ** 2;
            if (squared < distance) { distance = squared; closest = { x, y }; }
        }
        candidates.push({ ...closest, bounds: [minX, minY, width, height], pixels: tail });
    }
    return candidates.length === 1 ? candidates[0] : null;
}

function isVisibleGround(image, point) {
    const pixel = pixelAt(image, point.x, point.y);
    return pixel !== null && pixel[3] >= 128 && pixel[1] >= 40 && pixel[1] > pixel[0] + 12 && pixel[1] > pixel[2] + 12;
}

function findGround(image) {
    const preferred = { x: Math.floor(image.width * 0.625), y: Math.floor(image.height * 5 / 9) };
    if (isVisibleGround(image, preferred)) return preferred;
    for (let y = Math.floor(image.height * 0.4); y < image.height * 0.85; y += 8) {
        for (let x = Math.floor(image.width * 0.5); x < image.width * 0.9; x += 8) {
            const candidate = { x, y };
            if (isVisibleGround(image, candidate)) return candidate;
        }
    }
    return null;
}

module.exports = { findScout, findGround, isVisibleGround };
