// turts-test-role: driver
"use strict";

// PT1 instrument driver (MC design 13.3 and "Player path" steps 1 to 19; replaces
// tests/browser/m3-human.js). It plays the forge demo in two windows through the harness io
// only. Each key comes from params.bindings, the effective input map that scripts/mc-browser.sh
// reads from the packed copy (engine defaults merged with mods/forge/input/bindings.ron); the
// driver reads it once, at the start. After its own rebind of Stop it uses the new key.
// It saves one screenshot for each oracle step: pt1-oracle.js reads their checkpoints.
//
// Deviations from the person's run, each forced by the harness (recorded in the task report):
// - The harness joins both windows before drive. Steps 1 and 10 (the load screen) are seen after
//   a reload of window A at the start and of window B at step 15.
// - Step 4 (pan and zoom) runs after step 9: the map seeds of steps 5 to 9 need the start camera.
const {
    FORGE_MAP,
    RESEARCH_BUTTON_TARGET,
    findPlacement,
    groundNear,
    pressBinding,
    seedFor,
    setGuide,
    shot,
    waitForResearchButton,
} = require("./forge-pixels.js");

const LOAD_SHOTS = 30;
const LOAD_SHOT_MS = 300;
const RESEARCH_WAIT_SHOTS = 60;
const RESEARCH_SHOT_MS = 2000;
// Drone positions and points of steps 15 to 17 (mods/forge/maps/proving_ground.ron).
const DRONE_START = FORGE_MAP.farDrone;
const WAYPOINT_1 = [30, 7];
const WAYPOINT_2 = [26, 7];
const IN_RANGE_POINT = [21, 9];
const CLEAR_POINT_B = [28, 10];

function first(bindings, action) {
    const list = bindings?.actions?.[action];
    if (!Array.isArray(list) || list.length === 0) throw new Error(`PT1 params.bindings has no binding for ${action}`);
    return list[0];
}

function requirePoint(value, name) {
    if (!Array.isArray(value) || value.length < 2 || !value.slice(0, 2).every(Number.isFinite)) {
        throw new Error(`NOT TRACED: PT1 stage ${name} needs a reviewed screenshot pixel point in tests/browser/rows/PT1.json`);
    }
    return { x: value[0], y: value[1], delta: value[2] };
}

// The first letter key that the effective map does not use: the driver's new Stop key.
function freeLetterKey(bindings) {
    const used = new Set();
    for (const list of Object.values(bindings?.actions ?? {})) {
        for (const binding of list) if (binding?.Key) used.add(binding.Key.code);
    }
    for (let letter = 0; letter < 26; letter += 1) {
        const code = `Key${String.fromCharCode(65 + letter)}`;
        if (!used.has(code)) return { Key: { code, mods: [] } };
    }
    throw new Error("PT1 the effective map uses every letter key; no free key for the Stop rebind");
}

async function hold(io, binding, ms) {
    const key = binding.Key;
    await io.key(key.code, key.mods ?? [], { hold_ms: ms });
}

async function wheelFor(io, binding, x, y) {
    await io.move(x, y);
    await io.wheel(binding?.Wheel === "Up" ? -240 : 240);
}

async function loadScreen(io, label) {
    for (let index = 0; index < LOAD_SHOTS; index += 1) {
        await shot(io, `${label}-${String(index).padStart(2, "0")}`);
        await io.wait(LOAD_SHOT_MS);
    }
}

async function drive(io, params) {
    // Read the effective input map once (gate 2 ruling 14).
    const bindings = params.bindings;
    const keys = {
        guide: first(bindings, "toggle_guide"),
        menu: first(bindings, "menu"),
        panLeft: first(bindings, "camera_pan_left"),
        panRight: first(bindings, "camera_pan_right"),
        zoomIn: first(bindings, "zoom_in"),
        zoomOut: first(bindings, "zoom_out"),
        assign1: first(bindings, "group_assign_1"),
        recall1: first(bindings, "group_recall_1"),
        queue: first(bindings, "queue_modifier"),
        disable: first(bindings, "key_disable"),
        enable: first(bindings, "key_enable"),
        stop: first(bindings, "key_stop"),
    };
    const stages = params.stages ?? {};
    const clickToPlay = requirePoint(stages.click_to_play, "click_to_play");
    const controlsWheel = requirePoint(stages.guide_controls_wheel, "guide_controls_wheel");

    // Step 1: window A, the load screen.
    await io.use(1);
    await io.reload();
    await loadScreen(io, "pt1-s01-load");
    // Step 2: "Click to play" opens the guide at page 1; the camera shows the own factory.
    await io.click(clickToPlay.x, clickToPlay.y, "left");
    await io.wait(400);
    await shot(io, "pt1-s02-guide-open");
    // Guide page 5 (Controls) shows on the right of the open guide; scroll it to its tail.
    await io.move(controlsWheel.x, controlsWheel.y);
    await io.wheel(controlsWheel.delta);
    await io.wait(250);
    await shot(io, "pt1-s02-controls-tail");
    await io.wheel(-controlsWheel.delta);
    await io.wait(150);
    // Step 3: close the guide.
    await pressBinding(io, keys.guide, "toggle_guide");
    await io.wait(250);
    await shot(io, "pt1-s03-guide-closed");
    await setGuide(io, keys.guide, false, "toggle_guide");

    // Step 4, first half: hover over the drill.
    const drill = await findPlacement(io, 1, 0, FORGE_MAP.drill, "pt1-s04-drill");
    await io.move(drill.x, drill.y);
    await io.wait(100);
    await shot(io, "pt1-s04-hover-immediate");
    await io.wait(600);
    await shot(io, "pt1-s04-hover-tooltip");

    // Step 5: the Proving Lab, its shell count and the disabled research entry with its tooltip.
    const lab = await findPlacement(io, 1, 0, FORGE_MAP.lab, "pt1-s05-lab");
    await io.click(lab.x, lab.y, "left");
    await io.wait(250);
    await shot(io, "pt1-s05-lab-selected");
    await io.move(RESEARCH_BUTTON_TARGET.x, RESEARCH_BUTTON_TARGET.y);
    await io.wait(700);
    await shot(io, "pt1-s05-research-tooltip");

    // Step 6: disable the Shell Press, wait for the loader alert, enable it again.
    const press = await findPlacement(io, 1, 0, FORGE_MAP.press, "pt1-s06-press");
    await io.click(press.x, press.y, "left");
    await io.wait(250);
    await shot(io, "pt1-s06-press-selected");
    await pressBinding(io, keys.disable, "key_disable");
    await io.wait(250);
    await shot(io, "pt1-s06-disabled");
    for (let second = 1; second <= 12; second += 1) {
        await io.wait(1000);
        await shot(io, `pt1-s06-wait-${String(second).padStart(2, "0")}`);
    }
    await pressBinding(io, keys.enable, "key_enable");
    await io.wait(250);
    await shot(io, "pt1-s06-enabled");
    await io.wait(1500);
    await shot(io, "pt1-s06-loader-moves");

    // Step 7: "Research ready" in the alert list; start the research with the button.
    await io.click(lab.x, lab.y, "left");
    await io.wait(250);
    if (!await waitForResearchButton(io, RESEARCH_WAIT_SHOTS, "pt1-s07-research-ready")) {
        throw new Error("PT1 step 7: the research button did not show enabled");
    }
    await io.click(RESEARCH_BUTTON_TARGET.x, RESEARCH_BUTTON_TARGET.y, "left");
    await io.wait(400);
    await shot(io, "pt1-s07-research-started");

    // Step 8: "Research complete" shows; the turret panel shows damage 20, range 8, ammo.
    for (let index = 0; index < RESEARCH_WAIT_SHOTS; index += 1) {
        await io.wait(RESEARCH_SHOT_MS);
        await shot(io, `pt1-s08-wait-${String(index).padStart(2, "0")}`);
    }
    const turret = await findPlacement(io, 1, 0, FORGE_MAP.turret, "pt1-s08-turret");
    await io.click(turret.x, turret.y, "left");
    await io.wait(250);
    await shot(io, "pt1-s08-turret-selected");

    // Step 9: the turret cannot move.
    const ground = await groundNear(io, 1, [22, 9], "pt1-s09-ground");
    await shot(io, "pt1-s09-before-right-click");
    await io.click(ground.x, ground.y, "right");
    await io.wait(250);
    await shot(io, "pt1-s09-cannot-move");

    // Step 4, second half: pan with the arrow keys and zoom with the wheel.
    await shot(io, "pt1-s04-before-pan");
    await hold(io, keys.panRight, 400);
    await io.wait(150);
    await shot(io, "pt1-s04-panned");
    await wheelFor(io, keys.zoomIn, 760, 360);
    await io.wait(250);
    await shot(io, "pt1-s04-zoomed");
    await wheelFor(io, keys.zoomOut, 760, 360);
    await hold(io, keys.panLeft, 400);
    await io.wait(150);

    // Step 10: window B joined in the colour of slot 1.
    await io.use(2);
    await shot(io, "pt1-s10-joined");
    await io.click(clickToPlay.x, clickToPlay.y, "left");
    await io.wait(400);
    await setGuide(io, keys.guide, false, "toggle_guide");
    await shot(io, "pt1-s10-playing");

    // Step 11: the foreign turret, its gapped ring, empty card and range circle.
    const foreignTurret = await findPlacement(io, 2, 0, FORGE_MAP.turret, "pt1-s11-turret");
    await io.click(foreignTurret.x, foreignTurret.y, "left");
    await io.wait(250);
    await shot(io, "pt1-s11-turret-selected");

    // Step 12: a box drag round the own drone selects it.
    const drone = await findPlacement(io, 2, 1, DRONE_START, "pt1-s12-drone");
    await io.drag(drone.x - 28, drone.y - 28, drone.x + 28, drone.y + 28, "left");
    await io.wait(250);
    await shot(io, "pt1-s12-drone-selected");

    // Step 13: group 1, clear, recall, and recall twice to centre the camera.
    await pressBinding(io, keys.assign1, "group_assign_1");
    await io.wait(200);
    const clear = await groundNear(io, 2, CLEAR_POINT_B, "pt1-s13-clear");
    await io.click(clear.x, clear.y, "left");
    await io.wait(250);
    await shot(io, "pt1-s13-cleared");
    await pressBinding(io, keys.recall1, "group_recall_1");
    await io.wait(250);
    await shot(io, "pt1-s13-recalled");
    await pressBinding(io, keys.recall1, "group_recall_1");
    await io.wait(80);
    await pressBinding(io, keys.recall1, "group_recall_1");
    await io.wait(400);
    await shot(io, "pt1-s13-centred");

    // Step 14: the menu; a faster scroll speed (measured by a pan before and after) and a new
    // Stop key, then close the menu and check the guide Controls list.
    await hold(io, keys.panRight, 300);
    await io.wait(150);
    await shot(io, "pt1-s14-pan-slow");
    await hold(io, keys.panLeft, 300);
    await io.wait(150);
    await shot(io, "pt1-s14-pan-slow-back");
    await pressBinding(io, keys.menu, "menu");
    await io.wait(300);
    await shot(io, "pt1-s14-menu-open");
    const menu = stages.menu ?? {};
    if (!menu.scroll_speed_raise || !menu.stop_row_wheel || !menu.stop_rebind) {
        for (let index = 0; index < 8; index += 1) {
            await shot(io, `pt1-calibrate-menu-${index}`);
            await io.move(650, 500);
            await io.wheel(400);
            await io.wait(200);
        }
        throw new Error("NOT TRACED: PT1 stage menu points are missing; read pt1-calibrate-menu-*.png and write them in tests/browser/rows/PT1.json");
    }
    // One wheel puts the "Key stop" row and the Scroll speed slider in view together.
    const stopWheel = requirePoint(menu.stop_row_wheel, "menu.stop_row_wheel");
    await io.move(stopWheel.x, stopWheel.y);
    await io.wheel(stopWheel.delta);
    await io.wait(250);
    await shot(io, "pt1-s14-menu-scrolled");
    const raise = requirePoint(menu.scroll_speed_raise, "menu.scroll_speed_raise");
    await io.click(raise.x, raise.y, "left");
    await io.wait(250);
    await shot(io, "pt1-s14-scroll-speed");
    const rebind = requirePoint(menu.stop_rebind, "menu.stop_rebind");
    await io.click(rebind.x, rebind.y, "left");
    await io.wait(250);
    const newStop = freeLetterKey(bindings);
    await pressBinding(io, newStop, "new stop key");
    await io.wait(300);
    await shot(io, "pt1-s14-stop-rebound");
    await pressBinding(io, keys.menu, "menu");
    await io.wait(300);
    await shot(io, "pt1-s14-menu-closed");
    await hold(io, keys.panRight, 300);
    await io.wait(150);
    await shot(io, "pt1-s14-pan-fast");
    await hold(io, keys.panLeft, 300);
    await io.wait(150);
    await setGuide(io, keys.guide, true, "toggle_guide");
    await io.move(controlsWheel.x, controlsWheel.y);
    await io.wheel(controlsWheel.delta);
    await io.wait(250);
    await shot(io, "pt1-s14-controls-after-rebind");
    await io.wheel(-controlsWheel.delta);
    await setGuide(io, keys.guide, false, "toggle_guide");

    // Step 15: reload window B; it rejoins the same slot; group 1 and the new Stop key stay.
    await io.reload();
    await loadScreen(io, "pt1-s15-load");
    await io.click(clickToPlay.x, clickToPlay.y, "left");
    await io.wait(400);
    await shot(io, "pt1-s15-rejoined");
    await setGuide(io, keys.guide, false, "toggle_guide");
    await pressBinding(io, keys.recall1, "group_recall_1");
    await io.wait(80);
    await pressBinding(io, keys.recall1, "group_recall_1");
    await io.wait(400);
    await shot(io, "pt1-s15-group-recalled");

    // Step 16: Shift and two right clicks away from the turret; the new Stop key stops the drone.
    const point1 = await groundNear(io, DRONE_START, WAYPOINT_1, "pt1-s16-point-1");
    const point2 = await groundNear(io, DRONE_START, WAYPOINT_2, "pt1-s16-point-2");
    await io.key(keys.queue.Key.code, [], { action: "down" });
    await io.click(point1.x, point1.y, "right");
    await io.click(point2.x, point2.y, "right");
    await io.key(keys.queue.Key.code, [], { action: "up" });
    await io.wait(150);
    await shot(io, "pt1-s16-waypoints");
    await io.wait(2300);
    await pressBinding(io, newStop, "new stop key");
    await io.wait(200);
    await shot(io, "pt1-s16-stopped");
    await io.wait(1500);
    await shot(io, "pt1-s16-at-rest");

    // Step 17: a right click inside the turret range; the marker shows at once.
    const inRange = await groundNear(io, DRONE_START, IN_RANGE_POINT, "pt1-s17-point");
    await io.click(inRange.x, inRange.y, "right");
    await shot(io, "pt1-s17-marker");
    for (let index = 0; index < 16; index += 1) {
        await io.wait(500);
        await shot(io, `pt1-s17-move-${String(index).padStart(2, "0")}`);
    }

    // Steps 18 and 19: the turret fires; the drone loses health and is destroyed.
    for (let index = 0; index < 40; index += 1) {
        await io.wait(1000);
        await shot(io, `pt1-s18-${String(index).padStart(2, "0")}`);
    }
    await io.save("pt1-driver-keys.json", Buffer.from(JSON.stringify({ keys, newStop, seeds: {
        waypoint1: seedFor(DRONE_START, WAYPOINT_1), waypoint2: seedFor(DRONE_START, WAYPOINT_2),
        inRange: seedFor(DRONE_START, IN_RANGE_POINT),
    } }, null, 2) + "\n"));
}

module.exports = { drive, freeLetterKey };
