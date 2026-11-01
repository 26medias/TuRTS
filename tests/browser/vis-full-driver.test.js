// turts-test-role: unit
"use strict";

const assert = require("node:assert/strict");
const test = require("node:test");
const { saveScreenshotFiles } = require("./vis-full-driver.js");

test("VIS1 screenshots persist both PNG and oracle base64 artifacts under the same label", async () => {
    const saved = [];
    const io = { save: async (name, bytes) => saved.push([name, Buffer.from(bytes)]) };
    const bytes = Buffer.from([0, 1, 2, 255]);

    await saveScreenshotFiles(io, "vis1-full-initial", bytes);

    assert.deepEqual(saved.map(([name]) => name), [
        "vis1-full-initial.png",
        "vis1-full-initial.png.b64",
    ]);
    assert.deepEqual(saved[0][1], bytes);
    assert.deepEqual(saved[1][1], Buffer.from(bytes.toString("base64")));
});
