// turts-test-role: unit
"use strict";

const assert = require("node:assert/strict");
const { test } = require("node:test");
const { findSliderTracks, findVolumeTracks, volumeClickPoint } = require("./volume-tracks.js");

function image(bands) {
    const width = 640;
    const height = 480;
    const rgba = new Uint8Array(width * height * 4);
    for (let offset = 0; offset < rgba.length; offset += 4) rgba.set([17, 24, 32, 255], offset);
    for (const { top, left, width: bandWidth, height: bandHeight = 12 } of bands) {
        for (let y = top; y < top + bandHeight; y += 1) {
            for (let x = left; x < left + bandWidth; x += 1) rgba.set([51, 61, 77, 255], (y * width + x) * 4);
        }
        // A thumb at value 1: 20 px at the right end, 4 px above and below the track.
        for (let y = top - 4; y < top + bandHeight + 4; y += 1) {
            for (let x = left + bandWidth - 20; x < left + bandWidth; x += 1) rgba.set([102, 191, 242, 255], (y * width + x) * 4);
        }
    }
    return { width, height, rgba };
}

test("the volume tracks are the last four slider tracks, found by pixels (RUN130)", () => {
    const bands = [40, 90, 200, 250, 300, 350].map((top) => ({ top, left: 100, width: 390 }));
    const found = findVolumeTracks(image(bands));
    assert.deepEqual(Object.keys(found), ["master", "voice", "alerts", "effects"]);
    assert.deepEqual(found.master, { y: 206, left: 100, width: 390 });
    assert.deepEqual(found.effects, { y: 356, left: 100, width: 390 });
    // bevy slider.rs:347-350: value = (x - left - 10) / (width - 20).
    assert.deepEqual(volumeClickPoint(found.master, 0.9), { x: 100 + 10 + Math.round(0.9 * 370), y: 206 });
    assert.equal(findVolumeTracks(image(bands.slice(0, 3))), null, "fewer than four tracks is no menu");
    assert.equal(findSliderTracks(image([{ top: 100, left: 100, width: 390, height: 30 }])).length, 0, "a tall block is not a track");
    assert.equal(findSliderTracks(image([{ top: 100, left: 100, width: 100 }])).length, 0, "a short bar is not a track");
});
