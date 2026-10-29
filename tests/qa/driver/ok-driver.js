// turts-test-role: driver
"use strict";
const assert = require("node:assert/strict");
async function drive(io) {
    await io.drag(400, 300, 600, 450, "left");
    const png = await io.screenshot();
    assert.ok(png.length > 0);
    await io.save("after-drag.png", png);
}
Object.assign(exports, { drive });
