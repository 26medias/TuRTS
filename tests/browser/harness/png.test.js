// turts-test-role: unit
"use strict";

const assert = require("node:assert/strict");
const { deflateSync } = require("node:zlib");
const test = require("node:test");
const { decodePng, pixelAt } = require("./png.js");

function crc32(bytes) {
    let value = 0xffffffff;
    for (const byte of bytes) {
        value ^= byte;
        for (let bit = 0; bit < 8; bit += 1) value = value & 1 ? (value >>> 1) ^ 0xedb88320 : value >>> 1;
    }
    return (value ^ 0xffffffff) >>> 0;
}

function chunk(type, data) {
    const name = Buffer.from(type);
    const length = Buffer.alloc(4);
    length.writeUInt32BE(data.length);
    const crc = Buffer.alloc(4);
    crc.writeUInt32BE(crc32(Buffer.concat([name, data])));
    return Buffer.concat([length, name, data, crc]);
}

function fixture(channels, filter, options = {}) {
    const width = options.width ?? 5;
    const height = options.height ?? 4;
    const stride = width * channels;
    const raw = Buffer.from(Array.from({ length: stride * height }, (_, index) => (index * 37 + Math.floor(index / stride) * 13) & 255));
    const filtered = Buffer.alloc(height * (stride + 1));
    for (let y = 0; y < height; y += 1) {
        filtered[y * (stride + 1)] = filter;
        for (let x = 0; x < stride; x += 1) {
            const index = y * stride + x;
            const left = x >= channels ? raw[index - channels] : 0;
            const up = y ? raw[index - stride] : 0;
            const upperLeft = y && x >= channels ? raw[index - stride - channels] : 0;
            let predicted = 0;
            if (filter === 1) predicted = left;
            if (filter === 2) predicted = up;
            if (filter === 3) predicted = Math.floor((left + up) / 2);
            if (filter === 4) {
                const estimate = left + up - upperLeft;
                const candidates = [left, up, upperLeft];
                predicted = candidates.reduce((best, value) => Math.abs(estimate - value) < Math.abs(estimate - best) ? value : best);
            }
            filtered[y * (stride + 1) + 1 + x] = (raw[index] - predicted) & 255;
        }
    }
    const header = Buffer.alloc(13);
    header.writeUInt32BE(width, 0);
    header.writeUInt32BE(height, 4);
    header[8] = 8;
    header[9] = channels === 3 ? 2 : 6;
    header[12] = options.interlaced ? 1 : 0;
    const png = Buffer.concat([
        Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]),
        chunk("IHDR", header),
        chunk("IDAT", deflateSync(options.short ? filtered.subarray(0, filtered.length - 1) : filtered)),
        chunk("IEND", Buffer.alloc(0)),
    ]);
    const expected = Buffer.alloc(width * height * 4);
    for (let index = 0; index < width * height; index += 1) {
        raw.copy(expected, index * 4, index * channels, index * channels + 3);
        expected[index * 4 + 3] = channels === 4 ? raw[index * channels + 3] : 255;
    }
    return { png, expected };
}

for (const channels of [3, 4]) {
    for (let filter = 0; filter <= 4; filter += 1) {
        test("PNG " + channels + " channels reconstructs filter " + filter, () => {
            const data = fixture(channels, filter);
            const actual = decodePng(data.png);
            assert.equal(actual.width, 5);
            assert.equal(actual.height, 4);
            assert.deepEqual(actual.rgba, data.expected);
            assert.deepEqual(pixelAt(actual, 4, 3), Array.from(data.expected.subarray(-4)));
        });
    }
}

test("PNG rejects missing or truncated chunks", () => {
    assert.throws(() => decodePng(Buffer.alloc(33)), /signature/);
    const { png } = fixture(3, 0);
    assert.throws(() => decodePng(png.subarray(0, -3)), /Incomplete|Truncated/);
});

test("PNG rejects invalid filters, interlacing and short scanlines", () => {
    assert.throws(() => decodePng(fixture(3, 5).png), /filter/);
    assert.throws(() => decodePng(fixture(3, 0, { interlaced: true }).png), /format/);
    assert.throws(() => decodePng(fixture(3, 0, { short: true }).png), /length/);
});

test("PNG rejects dimensions before allocating pixels", () => {
    const { png } = fixture(3, 0);
    png.writeUInt32BE(20000, 16);
    assert.throws(() => decodePng(png), /dimensions/);
});

test("pixel coordinates outside the image return no colour", () => {
    const actual = decodePng(fixture(4, 0).png);
    for (const [x, y] of [[-1, 0], [0, -1], [5, 0], [0, 4], [0.5, 0]]) assert.equal(pixelAt(actual, x, y), null);
});

module.exports = { fixture };
