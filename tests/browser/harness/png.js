// turts-test-role: pure
"use strict";

const { inflateSync } = require("node:zlib");
const signature = Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]);

function paeth(left, up, upperLeft) {
    const estimate = left + up - upperLeft;
    const a = Math.abs(estimate - left);
    const b = Math.abs(estimate - up);
    const c = Math.abs(estimate - upperLeft);
    return a <= b && a <= c ? left : b <= c ? up : upperLeft;
}

// Playwright emits noninterlaced eight-bit RGB or RGBA screenshots.
function decodePng(input) {
    const bytes = Buffer.from(input);
    if (bytes.length < 33 || !bytes.subarray(0, 8).equals(signature)) throw new Error("Invalid PNG signature");
    let width;
    let height;
    let channels;
    let ended = false;
    const compressed = [];
    for (let offset = 8; offset + 12 <= bytes.length;) {
        const length = bytes.readUInt32BE(offset);
        const type = bytes.toString("ascii", offset + 4, offset + 8);
        const end = offset + 12 + length;
        if (end > bytes.length) throw new Error("Truncated PNG chunk");
        const data = bytes.subarray(offset + 8, offset + 8 + length);
        if (type === "IHDR") {
            if (width !== undefined || offset !== 8 || length !== 13) throw new Error("Invalid PNG header");
            width = data.readUInt32BE(0);
            height = data.readUInt32BE(4);
            channels = data[9] === 2 ? 3 : data[9] === 6 ? 4 : 0;
            if (!width || !height || width > 16384 || height > 16384 || width * height > 16777216) throw new Error("PNG dimensions exceed screenshot limits");
            if (data[8] !== 8 || !channels || data[10] || data[11] || data[12]) throw new Error("Unsupported PNG screenshot format");
        } else if (type === "IDAT") {
            compressed.push(data);
        } else if (type === "IEND") {
            if (length !== 0) throw new Error("Invalid PNG end");
            ended = true;
            break;
        }
        offset = end;
    }
    if (!channels || !ended || compressed.length === 0) throw new Error("Incomplete PNG screenshot");
    const stride = width * channels;
    const expected = height * (stride + 1);
    const filtered = inflateSync(Buffer.concat(compressed), { maxOutputLength: expected });
    if (filtered.length !== expected) throw new Error("PNG scanline length mismatch");
    const decoded = Buffer.alloc(height * stride);
    for (let y = 0; y < height; y += 1) {
        const filter = filtered[y * (stride + 1)];
        if (filter > 4) throw new Error("Invalid PNG scanline filter");
        for (let x = 0; x < stride; x += 1) {
            const index = y * stride + x;
            const left = x >= channels ? decoded[index - channels] : 0;
            const up = y ? decoded[index - stride] : 0;
            const upperLeft = y && x >= channels ? decoded[index - stride - channels] : 0;
            const predictor = filter === 0 ? 0 : filter === 1 ? left : filter === 2 ? up : filter === 3 ? Math.floor((left + up) / 2) : paeth(left, up, upperLeft);
            decoded[index] = (filtered[y * (stride + 1) + 1 + x] + predictor) & 255;
        }
    }
    const rgba = Buffer.alloc(width * height * 4);
    for (let index = 0; index < width * height; index += 1) {
        rgba[index * 4] = decoded[index * channels];
        rgba[index * 4 + 1] = decoded[index * channels + 1];
        rgba[index * 4 + 2] = decoded[index * channels + 2];
        rgba[index * 4 + 3] = channels === 4 ? decoded[index * channels + 3] : 255;
    }
    return { width, height, rgba };
}

function pixelAt(image, x, y) {
    if (!Number.isInteger(x) || !Number.isInteger(y) || x < 0 || y < 0 || x >= image.width || y >= image.height) return null;
    const index = (y * image.width + x) * 4;
    return Array.from(image.rgba.subarray(index, index + 4));
}

// Pure screen-geometry helpers moved from the M2 legacy file (MC design 13.3). They read only
// the numbers that a caller gives them; a driver gets no camera matrix, so only an oracle or a
// unit test gives them a scene.
function finitePair(value) {
    return Array.isArray(value) && value.length === 2 && value.every(Number.isFinite);
}

function projectGroundPoint(scene, mapPoint) {
    const matrix = scene?.clip_from_world;
    const viewport = scene?.viewport_logical_rect;
    if (
        !Array.isArray(matrix) || matrix.length !== 16 || !matrix.every(Number.isFinite) ||
        !Array.isArray(viewport) || viewport.length !== 4 || !viewport.every(Number.isFinite) ||
        !finitePair(mapPoint) || scene.coordinate_space !== "window-logical-pixels-top-left"
    ) {
        return null;
    }

    const clipX = matrix[0] * mapPoint[0] + matrix[8] * mapPoint[1] + matrix[12];
    const clipY = matrix[1] * mapPoint[0] + matrix[9] * mapPoint[1] + matrix[13];
    const clipZ = matrix[2] * mapPoint[0] + matrix[10] * mapPoint[1] + matrix[14];
    const clipW = matrix[3] * mapPoint[0] + matrix[11] * mapPoint[1] + matrix[15];
    if (!Number.isFinite(clipW) || clipW <= 0 || !Number.isFinite(clipX) || !Number.isFinite(clipY) || !Number.isFinite(clipZ)) {
        return null;
    }
    const ndcX = clipX / clipW;
    const ndcY = clipY / clipW;
    const ndcZ = clipZ / clipW;
    if (ndcZ < 0 || ndcZ > 1) return null;

    const [left, top, width, height] = viewport;
    const screenX = left + ((ndcX + 1) / 2) * width;
    const screenY = top + ((1 - ndcY) / 2) * height;
    const windowSize = scene.window_logical_size;
    if (
        !finitePair(windowSize) || screenX < left || screenY < top ||
        screenX >= left + width || screenY >= top + height ||
        screenX < 0 || screenY < 0 || screenX >= windowSize[0] || screenY >= windowSize[1]
    ) {
        return null;
    }
    return [screenX, screenY];
}

module.exports = { decodePng, pixelAt, finitePair, projectGroundPoint };
