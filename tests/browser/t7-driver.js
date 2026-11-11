// turts-test-role: driver
"use strict";

// T7 (QA0 design 4.6 option C, MC design 13.3): find the one acme unit by its colour in a
// screenshot, select it with a left click, and send it to a ground pixel with a right click.
// Then take screenshots until the unit stops in the screenshots. Each screenshot gives the
// harness one scene checkpoint; the oracle and scripts/t7.sh read those, the driver does not.
const { decodePng } = require("./harness/png.js");
const { findScout, findGround, isVisibleGround } = require("./harness/click9-pixels.js");

const FIND_ATTEMPTS = 80;
const FIND_WAIT_MS = 100;
// The harness keeps at most 256 checkpoints; the driver uses at most 200 for the move.
const MAX_MOVE_SHOTS = 200;
const SHOT_WAIT_MS = 40;
// The unit is at rest when its pixel bounds do not change for this many screenshots in a row.
const REST_SHOTS = 12;
const MIN_MOVE_SHOTS = 20;

async function drive(io) {
    let targets = null;
    for (let attempt = 0; attempt < FIND_ATTEMPTS && !targets; attempt += 1) {
        const shot = await io.screenshot();
        const image = decodePng(shot);
        const unit = findScout(image);
        const ground = findGround(image);
        if (unit && ground && Math.hypot(unit.x - ground.x, unit.y - ground.y) >= 120) {
            targets = { unit, ground };
            await io.save("t7-00-before-input.png", shot);
        } else if (attempt + 1 < FIND_ATTEMPTS) {
            await io.wait(FIND_WAIT_MS);
        }
    }
    if (!targets) throw new Error("T7 could not find the unit colour and an open ground pixel at least 120 px apart in the screenshot");
    await io.save("t7-pixel-targets.json", Buffer.from(JSON.stringify({
        ...targets,
        method: "one connected unit-colour component and one ground-colour pixel, from screenshot pixels only",
    }, null, 2) + "\n"));

    await io.click(targets.unit.x, targets.unit.y, "left");
    await io.wait(150);
    const selected = await io.screenshot();
    await io.save("t7-01-selected.png", selected);
    if (!isVisibleGround(decodePng(selected), targets.ground)) throw new Error("T7 ground pixel is covered after the selection");

    await io.click(targets.ground.x, targets.ground.y, "right");
    let last = null;
    let still = 0;
    let shots = 0;
    for (; shots < MAX_MOVE_SHOTS; shots += 1) {
        const shot = await io.screenshot();
        const unit = findScout(decodePng(shot));
        const bounds = unit ? unit.bounds.join(",") : null;
        still = bounds !== null && bounds === last ? still + 1 : 0;
        last = bounds;
        if (shots >= MIN_MOVE_SHOTS && still >= REST_SHOTS) {
            await io.save("t7-02-at-rest.png", shot);
            break;
        }
        await io.wait(SHOT_WAIT_MS);
    }
    await io.save("t7-move-shots.json", Buffer.from(JSON.stringify({ shots: shots + 1, rest_shots: still }) + "\n"));
}

module.exports = { drive };
