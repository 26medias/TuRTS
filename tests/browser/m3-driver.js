// turts-test-role: driver
"use strict";

// M3 F10 through the forge HUD (MC design 13.3; replaces tests/browser/m3.js). Window 1 is the
// factory slot: the press panel, research on the lab, disable and enable of the lab, and the
// removal of one belt piece. Window 2 is the drone slot: one drone moves into turret range.
// The order keys are the keys that the forge command card shows (mods/forge/input/bindings.ron);
// m3.js clicked the engine panel, which MC task 12 removed.
const {
    FORGE_MAP,
    findPlacement,
    groundNear,
    shot,
    waitForResearchButton,
    RESEARCH_BUTTON_TARGET,
} = require("./forge-pixels.js");

const KEY_DISABLE = "KeyD";
const KEY_ENABLE = "KeyE";
const KEY_REMOVE = "KeyX";
const RESEARCH_WAIT_SECONDS = 120;
// A point inside the turret range (turret (17, 12), range 8) and clear of the factory.
const RANGE_POINT = [21, 13];

async function drive(io) {
    await io.use(1);
    await shot(io, "m3-01-joined");
    // The forge guide opens at the first join; F1 closes it (mods/forge/interface/hud.ron).
    await io.key("F1");
    await io.wait(200);
    await shot(io, "m3-01-guide-closed");

    const press = await findPlacement(io, 1, 0, FORGE_MAP.press, "m3-press");
    await io.click(press.x, press.y, "left");
    await io.wait(200);
    await shot(io, "m3-02-press-two-port-inventory");

    const lab = await findPlacement(io, 1, 0, FORGE_MAP.lab, "m3-lab");
    await io.click(lab.x, lab.y, "left");
    await io.wait(200);
    await shot(io, "m3-03-lab-selected");
    if (!await waitForResearchButton(io, RESEARCH_WAIT_SECONDS, "m3-03-lab-research-ready")) {
        throw new Error(`M3 the forge research button did not show enabled within ${RESEARCH_WAIT_SECONDS} s`);
    }
    await io.click(RESEARCH_BUTTON_TARGET.x, RESEARCH_BUTTON_TARGET.y, "left");
    await io.wait(400);
    await shot(io, "m3-04-research-applied");

    await io.key(KEY_DISABLE);
    await io.wait(400);
    await shot(io, "m3-05-lab-disabled");
    await io.key(KEY_ENABLE);
    await io.wait(400);
    await shot(io, "m3-06-lab-enabled");

    const belt = await findPlacement(io, 1, 0, FORGE_MAP.belt, "m3-belt", 24);
    await io.click(belt.x, belt.y, "left");
    await io.wait(200);
    await shot(io, "m3-07-belt-selected");
    await io.key(KEY_REMOVE);
    await io.wait(600);
    await shot(io, "m3-08-belt-removed");

    await io.use(2);
    await shot(io, "m3-09-drone-window");
    await io.key("F1");
    await io.wait(200);
    const drone = await findPlacement(io, 2, 1, FORGE_MAP.farDrone, "m3-drone");
    await io.drag(drone.x - 30, drone.y - 30, drone.x + 30, drone.y + 30, "left");
    await io.wait(200);
    await shot(io, "m3-10-drone-selected");
    const ground = await groundNear(io, 2, RANGE_POINT, "m3-range-point");
    await io.click(ground.x, ground.y, "right");
    await io.wait(300);
    await shot(io, "m3-11-drone-ordered");
    // Drone speed 2.0 (mods/forge/units/drone.ron); about 9.5 map units to go.
    await io.wait(8000);
    await shot(io, "m3-12-drone-in-range");
}

module.exports = { drive };
