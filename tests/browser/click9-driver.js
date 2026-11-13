// turts-test-role: driver
"use strict";

const { decodePng } = require("./harness/png.js");
const { findScout, findGround, isVisibleGround } = require("./harness/click9-pixels.js");

async function drive(io) {
    // The first join shows the "Click to play" overlay until a gesture (client_shell.rs
    // sync_focus_overlay, since 3d0cf2f); it takes the first click. A keydown is also the gesture
    // (crates/turts_client_web/index.html:39-40); F9 has no binding (RUN130).
    await io.key("F9");
    await io.wait(300);
    let targets = null;
    for (let attempt = 0; attempt < 80; attempt += 1) {
        const shot = await io.screenshot();
        const image = decodePng(shot);
        const unit = findScout(image);
        const ground = findGround(image);
        if (unit && ground) {
            targets = { unit, ground };
            await io.save("00-before-input.png", shot);
            await io.save("pixel-targets.json", Buffer.from(JSON.stringify(targets) + "\n"));
            break;
        }
        await io.wait(100);
    }
    if (!targets) throw new Error("CLICK9 scout and ground are not visible in screenshot pixels");
    await io.click(targets.unit.x, targets.unit.y, "left");
    await io.wait(150);
    const selected = await io.screenshot();
    await io.save("01-after-left-click.png", selected);
    if (!isVisibleGround(decodePng(selected), targets.ground)) throw new Error("CLICK9 ground is obscured after selection");
    await io.click(targets.ground.x, targets.ground.y, "right");
    // Count actual frames in the oracle; this delay alone is not the retention proof.
    await io.wait(15000);
    await io.save("02-late-scene.png", await io.screenshot());
}

module.exports = { drive };
