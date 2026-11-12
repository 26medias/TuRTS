// turts-test-role: unit
"use strict";

const assert = require("node:assert/strict");
const { deflateSync } = require("node:zlib");
const { test } = require("node:test");
const { drive } = require("./mm1-driver.js");

function crc32(bytes) {
    let crc = 0xffffffff;
    for (const byte of bytes) {
        crc ^= byte;
        for (let bit = 0; bit < 8; bit += 1) crc = (crc >>> 1) ^ (0xedb88320 & -(crc & 1));
    }
    return (crc ^ 0xffffffff) >>> 0;
}

function chunk(type, data) {
    const length = Buffer.alloc(4);
    length.writeUInt32BE(data.length);
    const body = Buffer.concat([Buffer.from(type, "ascii"), data]);
    const crc = Buffer.alloc(4);
    crc.writeUInt32BE(crc32(body));
    return Buffer.concat([length, body, crc]);
}

const WIDTH = 1280;
const HEIGHT = 720;

// A 1280x720 PNG from a list of filled rectangles [x, y, w, h, [r, g, b]] on a dark ground.
function png(rects) {
    const raw = Buffer.alloc((WIDTH * 4 + 1) * HEIGHT);
    for (let y = 0; y < HEIGHT; y += 1) {
        for (let x = 0; x < WIDTH; x += 1) {
            const offset = y * (WIDTH * 4 + 1) + 1 + x * 4;
            raw[offset] = 20; raw[offset + 1] = 30; raw[offset + 2] = 40; raw[offset + 3] = 255;
        }
    }
    for (const [x0, y0, w, h, rgb] of rects) {
        for (let y = y0; y < y0 + h; y += 1) {
            for (let x = x0; x < x0 + w; x += 1) {
                const offset = y * (WIDTH * 4 + 1) + 1 + x * 4;
                raw[offset] = rgb[0]; raw[offset + 1] = rgb[1]; raw[offset + 2] = rgb[2];
            }
        }
    }
    const header = Buffer.alloc(13);
    header.writeUInt32BE(WIDTH, 0);
    header.writeUInt32BE(HEIGHT, 4);
    header[8] = 8;
    header[9] = 6;
    return Buffer.concat([Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]), chunk("IHDR", header), chunk("IDAT", deflateSync(raw)), chunk("IEND", Buffer.alloc(0))]);
}

const OWN_COLUMN = [600, 300, 12, 60, [0, 114, 178]];
const FOREIGN = [628, 345, 12, 24, [230, 159, 0]];
const PING = [1180, 560, 3, 3, [255, 204, 0]];
const WINDOW1 = png([OWN_COLUMN, FOREIGN]);
const NO_PING = png([]);
const WITH_PING = png([PING]);

const PARAMS = {
    row: "MM1", viewport: { width: WIDTH, height: HEIGHT }, minimap_inset: 8, minimap_size: 200, ping_colour: "#ffcc00",
};

// A fake pair of windows. The window 2 shot number k shows the ping when k >= pingAtShot.
function fakeIo(pingAtShot) {
    const log = [];
    let current = 1;
    const shots = { 1: 0, 2: 0 };
    const saved = {};
    const io = {
        log, saved,
        async use(window) { log.push(`use:${window}`); current = window; },
        async key(code) { log.push(`key:${code}@w${current}`); },
        async wait(ms) { log.push(`wait:${ms}`); },
        async click(x, y, button) { log.push(`click:${x},${y},${button}`); },
        async drag() { log.push("drag"); },
        async move() { log.push("move"); },
        async wheel(dy) { log.push(`wheel:${dy}`); },
        async screenshot() {
            const index = shots[current]; shots[current] += 1;
            log.push(`shot@w${current}#${index}`);
            if (current === 1) return WINDOW1;
            return index >= pingAtShot ? WITH_PING : NO_PING;
        },
        async save(name, bytes) { log.push(`save:${name}`); saved[name] = bytes; },
    };
    return io;
}

test("MM1 sends a key gesture in window 2 before its first screenshot (the play overlay dims the ping colour)", async () => {
    const io = fakeIo(1);
    await drive(io, PARAMS);
    const firstUseTwo = io.log.indexOf("use:2");
    const key = io.log.indexOf("key:F9@w2");
    const firstShot = io.log.indexOf("shot@w2#0");
    assert.ok(firstUseTwo >= 0 && key > firstUseTwo && key < firstShot, `F9 in window 2 comes before its first shot: ${io.log.slice(0, 6)}`);
});

test("MM1 takes the window 2 ping shot when the ping pixels appear, before the window 1 camera steps", async () => {
    const io = fakeIo(3);
    await drive(io, PARAMS);
    const polls = io.log.filter((entry) => /^shot@w2#[1-9]/.test(entry));
    assert.deepEqual(polls, ["shot@w2#1", "shot@w2#2", "shot@w2#3", "shot@w2#4"], "polls 1 and 2 show no ping, poll 3 shows it, shot 4 is the capture after the settle");
    assert.ok(!io.log.includes("wait:11000"), "no fixed 11 s wait before the ping shot");
    const seen = io.log.indexOf("shot@w2#3");
    const settle = io.log.indexOf("wait:3500", seen);
    const capture = io.log.indexOf("shot@w2#4");
    const saved = io.log.indexOf("save:mm1-window2-ping.png");
    const leftClick = io.log.findIndex((entry, index) => index > saved && entry.startsWith("click:") && entry.endsWith(",left"));
    assert.ok(seen < settle && settle < capture && capture < saved, `settle after the sighting, then the capture: ${io.log.slice(seen - 2, saved + 1)}`);
    assert.ok(saved < leftClick, "the window 2 ping shot is saved before the window 1 minimap left click");
    assert.equal(io.log.filter((entry) => entry === "save:mm1-window2-ping.png").length, 1, "one ping shot only");
    const steps = JSON.parse(Buffer.from(io.saved["mm1-driver-steps.json"]).toString("utf8"));
    assert.equal(steps.ping_poll.seen, true);
    assert.equal(steps.ping_poll.polls, 3);
});

test("MM1 ends the ping poll without an error when no ping appears, and still saves the ping shot", async () => {
    const io = fakeIo(Infinity);
    await drive(io, PARAMS);
    assert.ok(io.log.includes("save:mm1-window2-ping.png"), "the ping shot exists so that the oracle names the fault");
    const steps = JSON.parse(Buffer.from(io.saved["mm1-driver-steps.json"]).toString("utf8"));
    assert.equal(steps.ping_poll.seen, false);
    assert.ok(steps.ping_poll.polls >= 20, "the poll tried more than a few shots");
    assert.ok(!io.log.some((entry) => entry === "wait:3500"), "no settle wait without a sighting");
});

test("MM1 takes a bounded series of captures after the minimap right click, before the zoom (RUN142: the replica stands still after a kill)", async () => {
    const io = fakeIo(1);
    await drive(io, PARAMS);
    const rightClick = io.log.findLastIndex((entry) => /^click:.*,right$/.test(entry));
    const saves = ["mm1-after-right.png", "mm1-after-right-2.png", "mm1-after-right-3.png", "mm1-after-right-4.png"].map((name) => io.log.indexOf(`save:${name}`));
    assert.ok(rightClick >= 0 && saves[0] > rightClick, `the first capture follows the right click: ${saves}`);
    assert.ok(saves.every((index, at) => index > 0 && (at === 0 || index > saves[at - 1])), `four captures in order: ${saves}`);
    const wheel = io.log.indexOf("wheel:-20");
    assert.ok(saves[3] < wheel, "all captures after the right click come before the zoom");
    assert.equal(io.log.slice(saves[0], saves[3]).filter((entry) => entry === "wait:700").length, 3, "700 ms between the captures");
    const steps = JSON.parse(Buffer.from(io.saved["mm1-driver-steps.json"]).toString("utf8"));
    assert.deepEqual(steps.after_right_later, ["mm1-after-right-2.png", "mm1-after-right-3.png", "mm1-after-right-4.png"]);
});
