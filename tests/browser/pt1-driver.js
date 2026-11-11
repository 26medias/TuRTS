// turts-test-role: driver
"use strict";

// PT1 instrument driver (MC design 13.3 and "Player path" steps 1 to 19; replaces
// tests/browser/m3-human.js). It plays the forge demo in two windows through the harness io
// only. Each key comes from params.bindings, the effective input map that scripts/mc-browser.sh
// reads from the packed copy (engine defaults merged with mods/forge/input/bindings.ron); the
// driver reads it once, at the start. After its own rebind of Stop it uses the new key.
//
// pt1-oracle.js judges from harness records only (ruling C386), so the screenshot names here are
// hints for a person who reads the run, not evidence. The load screens of steps 1, 10 and 15 are
// harness records (startup-observations.json, the load record of io.reload()).
// A product fault (a unit or control not shown) ends the drive with a record; the oracle gives
// the semantic failure. A throw is for a setup fault (a missing binding or row stage) only.
// Step 4 (pan and zoom) runs after step 9: the map seeds of steps 5 to 9 need the start camera.
const { pixelAt } = require("./harness/png.js");
const { findPaletteTargetNear } = require("./sel1-driver.js");
const {
    FORGE_MAP,
    RESEARCH_BUTTON_TARGET,
    findPlacement,
    groundNear,
    pressBinding,
    seedFor,
    series,
    setGuide,
    shot,
    stop,
    waitForResearchButton,
} = require("./forge-pixels.js");

const RESEARCH_WAIT_SECONDS = 120;
const RESEARCH_SHOT_MS = 2000;
const RESEARCH_DONE_SHOTS = 60;
// Drone positions and points of steps 15 to 17 (mods/forge/maps/proving_ground.ron).
const DRONE_START = FORGE_MAP.farDrone;
const WAYPOINT_1 = [30, 7];
const WAYPOINT_2 = [26, 7];
const IN_RANGE_POINT = [21, 9];
const CLEAR_POINT_B = [28, 10];
// Settings menu fills, sampled from a retained headed SH1 screenshot
// (target/mc-evidence/root-t16-sh1-complete-controls-v1/.../SH1-conflicts-conflict-edit-row.png):
// a "Change" button and a slider knob.
const MENU_BUTTON_FILL = [46, 56, 71];
const MENU_KNOB_FILL = [102, 191, 242];

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

function fillNear(image, point, fill, radius, tolerance = 14) {
    for (let dy = -radius; dy <= radius; dy += 1) {
        for (let dx = -radius; dx <= radius; dx += 1) {
            const pixel = pixelAt(image, point.x + dx, point.y + dy);
            if (pixel && Math.hypot(pixel[0] - fill[0], pixel[1] - fill[1], pixel[2] - fill[2]) <= tolerance) return true;
        }
    }
    return false;
}

// Step 16: press the new Stop key once the drone has passed the first point (it is seen there,
// then no longer there), with a bound of 8 s.
// Guide sheet 5 (Controls) shows about 21 of its rows at once (run t21-run118-pt1-1: rows 0-20
// at the top, 30-50 at the tail), so one wheel to the tail misses the middle rows. Scroll it in
// steps of 300 px and take a screenshot at each step, then scroll back.
async function scanControls(io, wheelPoint, label) {
    await io.move(wheelPoint.x, wheelPoint.y);
    for (let index = 0; index < 6; index += 1) {
        await io.wait(250);
        await shot(io, `${label}-${String(index).padStart(2, "0")}`);
        await io.wheel(300);
    }
    await io.wheel(-1800);
    await io.wait(150);
}

async function stopAfterFirstPoint(io, newStop) {
    const seed = seedFor(DRONE_START, WAYPOINT_1);
    let seen = false;
    for (let index = 0; index < 40; index += 1) {
        const { image } = await shot(io, `pt1-s16-watch-${String(index).padStart(2, "0")}`);
        const there = Boolean(findPaletteTargetNear(image, 1, seed, 10));
        if (there) seen = true;
        if (seen && !there) break;
        await io.wait(150);
    }
    await pressBinding(io, newStop, "new stop key");
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
    };
    const stages = params.stages ?? {};
    const clickToPlay = requirePoint(stages.click_to_play, "click_to_play");
    const controlsWheel = requirePoint(stages.guide_controls_wheel, "guide_controls_wheel");
    const menu = stages.menu ?? {};
    const stopWheel = requirePoint(menu.stop_row_wheel, "menu.stop_row_wheel");
    const raise = requirePoint(menu.scroll_speed_raise, "menu.scroll_speed_raise");
    const rebind = requirePoint(menu.stop_rebind, "menu.stop_rebind");

    // Steps 1 and 2: window A after its join; "Click to play" opens the guide.
    await io.use(1);
    await shot(io, "pt1-s02-click-to-play");
    await io.click(clickToPlay.x, clickToPlay.y, "left");
    await io.wait(400);
    await shot(io, "pt1-s02-guide-open");
    // Guide sheet 5 (Controls) shows on the right of the open guide; scroll through all its rows.
    await scanControls(io, controlsWheel, "pt1-s02-controls");
    // Step 3: close the guide.
    await pressBinding(io, keys.guide, "toggle_guide");
    await io.wait(250);
    await shot(io, "pt1-s03-guide-closed");
    await setGuide(io, keys.guide, false, "toggle_guide");

    // Step 4, first half: hover over the drill; screenshots at once, then after the delay.
    const drill = await findPlacement(io, 1, 0, FORGE_MAP.drill, "pt1-s04-drill");
    if (!drill) return;
    await io.move(drill.x, drill.y);
    await series(io, "pt1-s04-hover", 8, 0);
    await io.wait(500);
    await series(io, "pt1-s04-tooltip", 3, 150);

    // Step 5: the Proving Lab, its shell count and the disabled research entry with its tooltip.
    const lab = await findPlacement(io, 1, 0, FORGE_MAP.lab, "pt1-s05-lab");
    if (!lab) return;
    await io.click(lab.x, lab.y, "left");
    await series(io, "pt1-s05-lab", 4, 250);
    await io.move(RESEARCH_BUTTON_TARGET.x, RESEARCH_BUTTON_TARGET.y);
    await io.wait(700);
    await series(io, "pt1-s05-research-tooltip", 3, 200);

    // Step 6: disable the Shell Press, wait for the loader alert, enable it again.
    const press = await findPlacement(io, 1, 0, FORGE_MAP.press, "pt1-s06-press");
    if (!press) return;
    await io.click(press.x, press.y, "left");
    await series(io, "pt1-s06-press", 3, 250);
    await pressBinding(io, keys.disable, "key_disable");
    await series(io, "pt1-s06-disabled", 24, 500);
    await pressBinding(io, keys.enable, "key_enable");
    await series(io, "pt1-s06-enabled", 12, 500);

    // Step 7: "Research ready" in the alert list; start the research with the button.
    await io.click(lab.x, lab.y, "left");
    await io.wait(250);
    if (!await waitForResearchButton(io, RESEARCH_WAIT_SECONDS, "pt1-s07-research-ready")) {
        return stop(io, "pt1-s07", "the research button did not show enabled");
    }
    await io.click(RESEARCH_BUTTON_TARGET.x, RESEARCH_BUTTON_TARGET.y, "left");
    await series(io, "pt1-s07-research-started", 6, 300);

    // Step 8: "Research complete" shows; the turret panel shows damage 20, range 8, ammo.
    await series(io, "pt1-s08-wait", RESEARCH_DONE_SHOTS, RESEARCH_SHOT_MS);
    const turret = await findPlacement(io, 1, 0, FORGE_MAP.turret, "pt1-s08-turret");
    if (!turret) return;
    await io.click(turret.x, turret.y, "left");
    await series(io, "pt1-s08-turret", 3, 250);

    // Step 9: the turret cannot move.
    const ground = await groundNear(io, 1, [22, 9], "pt1-s09-ground");
    if (!ground) return;
    await shot(io, "pt1-s09-before-right-click");
    await io.click(ground.x, ground.y, "right");
    await series(io, "pt1-s09-cannot-move", 4, 200);

    // Step 4, second half: pan with the arrow keys and zoom with the wheel.
    await shot(io, "pt1-s04-before-pan");
    await hold(io, keys.panRight, 400);
    await series(io, "pt1-s04-panned", 2, 150);
    await wheelFor(io, keys.zoomIn, 760, 360);
    await series(io, "pt1-s04-zoomed", 2, 150);
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
    if (!foreignTurret) return;
    await io.click(foreignTurret.x, foreignTurret.y, "left");
    await series(io, "pt1-s11-turret", 3, 250);

    // Step 12: a box drag round the own drone selects it.
    const drone = await findPlacement(io, 2, 1, DRONE_START, "pt1-s12-drone");
    if (!drone) return;
    await io.drag(drone.x - 28, drone.y - 28, drone.x + 28, drone.y + 28, "left");
    await series(io, "pt1-s12-drone", 3, 250);

    // Step 13: group 1, clear, recall, and recall twice to centre the camera.
    await pressBinding(io, keys.assign1, "group_assign_1");
    await io.wait(200);
    const clear = await groundNear(io, 2, CLEAR_POINT_B, "pt1-s13-clear");
    if (!clear) return;
    await io.click(clear.x, clear.y, "left");
    await series(io, "pt1-s13-cleared", 2, 250);
    await pressBinding(io, keys.recall1, "group_recall_1");
    await series(io, "pt1-s13-recalled", 2, 250);
    await pressBinding(io, keys.recall1, "group_recall_1");
    await io.wait(80);
    await pressBinding(io, keys.recall1, "group_recall_1");
    await series(io, "pt1-s13-centred", 3, 250);

    // Step 14: the menu; a faster scroll speed and a new Stop key; the guide Controls list.
    await pressBinding(io, keys.menu, "menu");
    await io.wait(300);
    // One wheel puts the "Key stop" row and the Scroll speed slider in view together. The menu
    // has no text that the driver can read (design 13.4: no OCR); the driver holds the layout as
    // literals and checks the control fill before each click; the oracle checks the target row.
    await io.move(stopWheel.x, stopWheel.y);
    await io.wheel(stopWheel.delta);
    await io.wait(250);
    const { image: menuImage } = await shot(io, "pt1-s14-menu-scrolled");
    if (!fillNear(menuImage, { x: raise.x, y: raise.y }, MENU_KNOB_FILL, 0) && !findKnobRow(menuImage, raise)) {
        return stop(io, "pt1-s14-scroll-speed", `no slider at ${JSON.stringify(raise)}`);
    }
    if (!fillNear(menuImage, { x: rebind.x - 24, y: rebind.y }, MENU_BUTTON_FILL, 2)) {
        return stop(io, "pt1-s14-rebind", `no Change button at ${JSON.stringify(rebind)}`);
    }
    await io.click(raise.x, raise.y, "left");
    await io.wait(250);
    await shot(io, "pt1-s14-scroll-speed");
    await io.click(rebind.x, rebind.y, "left");
    await io.wait(250);
    const newStop = freeLetterKey(bindings);
    await pressBinding(io, newStop, "new stop key");
    await series(io, "pt1-s14-stop-rebound", 2, 250);
    await pressBinding(io, keys.menu, "menu");
    await series(io, "pt1-s14-menu-closed", 2, 250);
    await setGuide(io, keys.guide, true, "toggle_guide");
    await scanControls(io, controlsWheel, "pt1-s14-controls-after-rebind");
    await setGuide(io, keys.guide, false, "toggle_guide");

    // Step 15: reload window B; it rejoins the same slot; group 1 and the new Stop key stay.
    await io.reload();
    await shot(io, "pt1-s15-after-reload");
    await io.click(clickToPlay.x, clickToPlay.y, "left");
    await io.wait(400);
    await series(io, "pt1-s15-rejoined", 2, 250);
    await setGuide(io, keys.guide, false, "toggle_guide");
    await pressBinding(io, keys.recall1, "group_recall_1");
    await io.wait(80);
    await pressBinding(io, keys.recall1, "group_recall_1");
    await series(io, "pt1-s15-group-recalled", 3, 250);

    // Step 16: Shift and two right clicks away from the turret; the new Stop key stops the drone.
    const point1 = await groundNear(io, DRONE_START, WAYPOINT_1, "pt1-s16-point-1");
    const point2 = await groundNear(io, DRONE_START, WAYPOINT_2, "pt1-s16-point-2");
    if (!point1 || !point2) return;
    await io.key(keys.queue.Key.code, [], { action: "down" });
    await io.click(point1.x, point1.y, "right");
    await io.click(point2.x, point2.y, "right");
    await io.key(keys.queue.Key.code, [], { action: "up" });
    await shot(io, "pt1-s16-waypoints");
    await stopAfterFirstPoint(io, newStop);
    await series(io, "pt1-s16-stopped", 6, 300);

    // Step 17: a right click inside the turret range; the marker shows at once.
    const inRange = await groundNear(io, DRONE_START, IN_RANGE_POINT, "pt1-s17-point");
    if (!inRange) return;
    await io.click(inRange.x, inRange.y, "right");
    await series(io, "pt1-s17-move", 17, 500);

    // Steps 18 and 19: the turret fires; the drone loses health and is destroyed.
    await series(io, "pt1-s18", 40, 1000);
}

// The Scroll speed slider: its knob fill in the row of the literal point (any x of the track).
function findKnobRow(image, point) {
    for (let x = 584; x <= 878; x += 2) {
        if (fillNear(image, { x, y: point.y }, MENU_KNOB_FILL, 1)) return true;
    }
    return false;
}

module.exports = { drive, freeLetterKey };
