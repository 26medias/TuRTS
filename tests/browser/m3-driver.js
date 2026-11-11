// turts-test-role: driver
"use strict";

// M3 F10 through the forge HUD (MC design 13.3; replaces tests/browser/m3.js). Window 1 is the
// factory slot: the press panel, research on the lab, disable and enable of the lab, and the
// removal of one belt piece, each through a HUD button. Window 2 is the drone slot: one drone
// moves into turret range. After each action the driver takes a bounded series of screenshots;
// the oracle uses the first one that passes (m3.js waited up to 10 s for each result).
const { findOrderRowCenters } = require("./hud2-driver.js");
const {
    FORGE_MAP,
    findPlacement,
    groundNear,
    series,
    shot,
    stop,
    waitForResearchButton,
    RESEARCH_BUTTON_TARGET,
} = require("./forge-pixels.js");

const RESEARCH_WAIT_SECONDS = 120;
// A point inside the turret range (turret (17, 12), range 8) and clear of the factory.
const RANGE_POINT = [21, 13];
// The forge command card lists the orders of the selection in content-id order
// (crates/turts_client/src/widgets/sources.rs:313-322): for the lab order_cancel,
// order_disable, order_enable, order_research; for a belt piece order_remove only
// (mods/forge/orders/*.ron `units`). The driver holds this layout as literals (design 13.4).
const LAB_CARD = { disable: 1, enable: 2 };
const BELT_CARD = { remove: 0 };

// Click card row `index` after a screenshot shows the rendered card rows.
async function clickCardRow(io, index, label) {
    const { image } = await shot(io, `${label}-card`);
    const rows = findOrderRowCenters(image);
    if (rows.length <= index) {
        await stop(io, label, `the command card shows ${rows.length} rows; row ${index} is not there`);
        return false;
    }
    await io.click(rows[index].x, rows[index].y, "left");
    return true;
}

// The first click of a fresh page dismisses the "Click to play" overlay (audio is locked until a
// gesture: crates/turts_client/src/client_shell.rs sync_focus_overlay); the overlay blocks it.
async function dismissOverlay(io) {
    await io.click(640, 690, "left");
    await io.wait(300);
}

async function drive(io) {
    await io.use(1);
    await shot(io, "m3-01-joined");
    await dismissOverlay(io);
    // The forge guide opens at the first join; F1 closes it (mods/forge/interface/hud.ron).
    await io.key("F1");
    await io.wait(200);
    await shot(io, "m3-01-guide-closed");

    const press = await findPlacement(io, 1, 0, FORGE_MAP.press, "m3-press");
    if (!press) return;
    await io.click(press.x, press.y, "left");
    await series(io, "m3-02-press", 10, 300);

    const lab = await findPlacement(io, 1, 0, FORGE_MAP.lab, "m3-lab");
    if (!lab) return;
    await io.click(lab.x, lab.y, "left");
    await io.wait(200);
    await shot(io, "m3-03-lab-selected");
    if (!await waitForResearchButton(io, RESEARCH_WAIT_SECONDS, "m3-03-lab-research-ready")) {
        return stop(io, "m3-research", `the research button did not show enabled within ${RESEARCH_WAIT_SECONDS} s`);
    }
    await io.click(RESEARCH_BUTTON_TARGET.x, RESEARCH_BUTTON_TARGET.y, "left");
    await series(io, "m3-04-research");

    if (!await clickCardRow(io, LAB_CARD.disable, "m3-05-disable")) return;
    await series(io, "m3-05-disabled");
    if (!await clickCardRow(io, LAB_CARD.enable, "m3-06-enable")) return;
    await series(io, "m3-06-enabled");

    const belt = await findPlacement(io, 1, 0, FORGE_MAP.belt, "m3-belt", 24);
    if (!belt) return;
    await io.click(belt.x, belt.y, "left");
    await series(io, "m3-07-belt", 10, 300);
    if (!await clickCardRow(io, BELT_CARD.remove, "m3-08-remove")) return;
    await series(io, "m3-08-removed");

    await io.use(2);
    await shot(io, "m3-09-drone-window");
    await dismissOverlay(io);
    await io.key("F1");
    await io.wait(200);
    const drone = await findPlacement(io, 2, 1, FORGE_MAP.farDrone, "m3-drone");
    if (!drone) return;
    await io.drag(drone.x - 30, drone.y - 30, drone.x + 30, drone.y + 30, "left");
    await series(io, "m3-10-drone", 10, 300);
    const ground = await groundNear(io, 2, RANGE_POINT, "m3-range-point");
    if (!ground) return;
    await io.click(ground.x, ground.y, "right");
    // Drone speed 2.0 (mods/forge/units/drone.ron); about 9.5 map units to go; 12 s bound.
    await series(io, "m3-11-drone-order", 24, 500);
}

module.exports = { drive, BELT_CARD, LAB_CARD };
