// turts-test-role: driver
"use strict";

async function drive(io) {
    await io.key("F1"); // close the authored guide with its real physical key
    const screenshot = Buffer.from(await io.screenshot());
    await io.save("vis1-slots.png", screenshot);
    await io.save("vis1-slots.png.b64", Buffer.from(screenshot.toString("base64")));
}

module.exports = { drive };
