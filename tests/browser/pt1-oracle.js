// turts-test-role: oracle
"use strict";

// PT1 oracle: the "Player path" list of the MC design, steps 1 to 19, checked on the harness
// checkpoints of the screenshots that pt1-driver.js saved, on the server tick log, and on the
// guide sheet 5 (Controls) as rendered against params.bindings. It replaces the asserts of the
// legacy tests/browser/m3-human.js. The texts are the forge texts of
// mods/forge/interface/hud.ron (alerts, names) and of crates/turts_client (reasons).
const assert = require("node:assert/strict");
const { decodePng, pixelAt } = require("./harness/png.js");
const { bindingLabel, checkpointBefore, checkpointForSave, readCheckpoints, replicaUnit, unitTrack, widget, widgets } = require("./checkpoint-lib.js");
const { unprojectGround } = require("./hud4-oracle.js");
const { centre } = require("./sh1-settings-oracle.js");
const { project, visibleRingPixels } = require("./vis-full-oracle.js");

const SLOT_RGB = [[0, 114, 178], [230, 159, 0]];
const RANGE_RGB = [242, 204, 96];
const TEXT = {
    drillTooltip: "Bore Drill · Slot 1",
    labName: "Proving Lab",
    loaderWaiting: "A loader is waiting for an item",
    researchReady: "The lab can begin research",
    researchComplete: "Hardened Shells research is complete",
    cannotMove: "Shell Turret: Cannot move",
    underAttack: "A unit is under attack",
    destroyed: "A unit was destroyed",
    guideFirst: "Build shells",
};

function step(number, condition, message) {
    assert(condition, `PT1 step ${number}: ${message}`);
}

function own(debug, contentId) {
    return (debug.replicated_world?.units ?? []).filter((unit) => unit.content_id === contentId && unit.owner_slot === debug.slot);
}

function alertTexts(debug) {
    return [
        ...(debug.alerts ?? []).map((alert) => alert.text),
        ...widgets(debug, (row) => row.id.startsWith("alert_rows/")).map((row) => row.text),
    ];
}

function sounds(debug) {
    return [
        ...(debug.audio?.sink_observations ?? []).map((sound) => sound.sound_id),
        ...(debug.frames?.retained_input ?? []).flatMap((frame) => (frame.feedback?.audio_started ?? []).map((sound) => sound.id)),
    ];
}

// Rust `humanize_action` (crates/turts_client/src/widgets/sources.rs:1012-1046).
function humanizeAction(action) {
    let known;
    if (action.startsWith("group_assign_")) known = `Assign group ${action.slice(13)}`;
    else if (action.startsWith("group_recall_")) known = `Select group ${action.slice(13)}`;
    else if (action.startsWith("group_add_")) known = `Add to group ${action.slice(10)}`;
    else if (action.startsWith("camera_pan_")) known = `Pan camera ${action.slice(11)}`;
    else {
        known = {
            toggle_guide: "Toggle guide", centre_selection: "Centre selection", camera_zoom: "Zoom camera",
            camera_rotate: "Rotate camera", selection_add: "Add to selection", selection_remove: "Remove from selection",
        }[action] ?? action;
    }
    return known.replace(/[_-]/g, " ").split(/\s+/).filter(Boolean)
        .map((word, index) => index === 0 ? word[0].toUpperCase() + word.slice(1) : word).join(" ");
}

// Rust `binding_label` (crates/turts_client/src/client_shell.rs:80-133), with no layout labels.
function playerLabel(binding) {
    if (binding?.Key) {
        const mods = ["Ctrl", "Shift", "Alt"].filter((mod) => (binding.Key.mods ?? []).includes(mod)).map((mod) => `${mod}+`).join("");
        const code = binding.Key.code;
        const key = /^Key.$/.test(code) ? code.slice(3) : /^Digit.$/.test(code) ? code.slice(5) :
            { Escape: "Esc", Space: "Space", ArrowUp: "Up", ArrowDown: "Down", ArrowLeft: "Left", ArrowRight: "Right" }[code] ?? code;
        return `${mods}${key}`;
    }
    if (binding?.Wheel) return `Wheel ${String(binding.Wheel).toLowerCase()}`;
    return bindingLabel(binding);
}

function controlsRows(debug) {
    return widgets(debug, (row) => row.id.startsWith("guide.4.bindings/") && typeof row.text === "string");
}

// Guide sheet 5 as rendered matches the effective map: each visible row is the row of one action.
function assertControlsMatch(debug, actions, label) {
    const expected = new Map(Object.entries(actions).map(([name, list]) =>
        [name, `${humanizeAction(name)}: ${list.map(playerLabel).join(", ")}`]));
    const rows = controlsRows(debug);
    step("guide-sheet-5", rows.length > 0, `${label}: guide sheet 5 (Controls) shows no binding row`);
    const texts = new Set(expected.values());
    for (const row of rows) {
        step("guide-sheet-5", texts.has(row.text), `${label}: the rendered Controls row "${row.text}" matches no binding of params.bindings`);
    }
    return { rows: rows.map((row) => row.text), expected };
}

function inViewport(scene, point) {
    const [left, top, width, height] = scene.viewport_logical_rect ?? [];
    return Array.isArray(point) && point[0] >= left && point[1] >= top && point[0] < left + width && point[1] < top + height;
}

function colourNear(image, point, colour, radius, tolerance = 60) {
    for (let dy = -radius; dy <= radius; dy += 1) {
        for (let dx = -radius; dx <= radius; dx += 1) {
            const pixel = pixelAt(image, Math.round(point[0] + dx), Math.round(point[1] + dy));
            if (pixel && Math.hypot(pixel[0] - colour[0], pixel[1] - colour[1], pixel[2] - colour[2]) <= tolerance) return true;
        }
    }
    return false;
}

function rangeCircleHits(image, scene, unit, radius) {
    let hits = 0;
    for (let index = 0; index < 24; index += 1) {
        const angle = index * Math.PI * 2 / 24;
        const point = project(scene.clip_from_world, scene.viewport_logical_rect,
            [unit.translation[0] + Math.cos(angle) * radius, 0.018, unit.translation[2] + Math.sin(angle) * radius]);
        if (point && colourNear(image, point, RANGE_RGB, 2, 70)) hits += 1;
    }
    return hits;
}

async function image(view, file) {
    return decodePng(Buffer.from(await view.serverFile(`${file}.png.b64`), "base64"));
}

function series(input, checkpoints, prefix) {
    return input.map((entry, index) => ({ entry, index }))
        .filter(({ entry }) => entry.call === "save" && typeof entry.args?.[0] === "string" &&
            entry.args[0].startsWith(prefix) && entry.args[0].endsWith(".png"))
        .map(({ entry }) => checkpointForSave(input, checkpoints, entry.args[0], "PT1"));
}

function keyIndex(input, code, after) {
    return input.findIndex((entry, index) => index > after && entry.call === "key" && entry.args?.[0] === code);
}

async function judge(view, params) {
    const input = view.inputLog();
    const checkpoints = await readCheckpoints(view);
    const at = (file) => checkpointForSave(input, checkpoints, `${file}.png`, `PT1 ${file}`);
    const actions = params.bindings?.actions;
    assert(actions && typeof actions === "object", "PT1 needs params.bindings, the effective input map");
    const keys = JSON.parse(await view.serverFile("pt1-driver-keys.json"));
    const tickLog = await view.serverFile("server-tick.log");

    // Step 1: the load screen shows each phase and a progress bar.
    const loadA = series(input, checkpoints, "pt1-s01-load-");
    step(1, loadA.some((shot) => !shot.debug.phase?.startsWith("Joined") && widget(shot.debug, (row) => row.id === "shell.load.progress.track")),
        "window A shows no load screen with a progress bar after the reload");
    step(1, new Set(loadA.map((shot) => shot.debug.phase?.split(/[ {(]/)[0])).size >= 2, "window A shows fewer than two load phases");

    // Step 2: "Click to play" opens the guide at sheet 1; the camera shows the own factory.
    const beforePlay = loadA.at(-1);
    step(2, (beforePlay.debug.ui_snapshot ?? []).some((row) => row.kind === "focus_overlay" && row.visible === true &&
        (row.text_nodes ?? []).some((node) => node.text === "Click to play" && node.visible === true)),
    "\"Click to play\" is not shown before the first click");
    const guideOpen = at("pt1-s02-guide-open");
    const guideCopy = widget(guideOpen.debug, (row) => row.id === "guide_copy");
    step(2, guideCopy && guideCopy.text.includes(TEXT.guideFirst), `the guide does not open at sheet 1 ("${TEXT.guideFirst}")`);
    step(2, (guideOpen.scene.units ?? []).some((unit) => inViewport(guideOpen.scene, unit.ground_screen_center) &&
        replicaUnit(guideOpen.debug, unit.unit_id)?.owner_slot === guideOpen.debug.slot), "the camera shows no own factory unit");

    // Guide sheet 5 as rendered matches params.bindings (gate 2 ruling 14).
    const effective = guideOpen.debug.effective_bindings?.actions;
    assert.deepEqual(effective, actions, "PT1 step guide-sheet-5: the client effective map differs from params.bindings that the driver used");
    const sheet = [assertControlsMatch(guideOpen.debug, actions, "guide open"),
        assertControlsMatch(at("pt1-s02-controls-tail").debug, actions, "Controls tail")];
    const shown = new Set(sheet.flatMap((entry) => entry.rows));
    for (const name of ["key_stop", "menu", "toggle_guide", "group_assign_1", "group_recall_1", "group_add_1", "queue_modifier"]) {
        step("guide-sheet-5", shown.has(sheet[0].expected.get(name)), `guide sheet 5 has no visible row for ${name}`);
    }

    // Step 3: the tester closes the guide.
    step(3, !widget(at("pt1-s03-guide-closed").debug, (row) => row.id === "guide_copy"), "the guide stays open after its key");

    // Step 4: hover ring at once, tooltip after 0.5 s; pan and zoom move the camera.
    const hoverNow = at("pt1-s04-hover-immediate");
    const hoverLate = at("pt1-s04-hover-tooltip");
    const drill = own(hoverNow.debug, "forge:drill")[0];
    const drillScene = hoverNow.scene.units.find((unit) => unit.unit_id === drill?.unit_id);
    step(4, drillScene && visibleRingPixels(await image(view, "pt1-s04-hover-immediate"), hoverNow.scene, { ...drillScene, owner_slot: drill.owner_slot }, drillScene.pick_radius),
        "no hover ring round the drill");
    step(4, !widgets(hoverNow.debug, (row) => row.kind === "world_tooltip").length, "the world tooltip shows before 0.5 s");
    step(4, widget(hoverLate.debug, (row) => row.kind === "world_tooltip" && row.text === TEXT.drillTooltip),
        `no world tooltip "${TEXT.drillTooltip}" after 0.5 s`);
    const beforePan = at("pt1-s04-before-pan");
    const panned = at("pt1-s04-panned");
    const zoomed = at("pt1-s04-zoomed");
    step(4, JSON.stringify(beforePan.scene.clip_from_world) !== JSON.stringify(panned.scene.clip_from_world), "the pan key does not move the camera");
    step(4, JSON.stringify(panned.scene.clip_from_world) !== JSON.stringify(zoomed.scene.clip_from_world), "the wheel does not zoom the camera");

    // Step 5: the lab panel; the disabled research entry and its tooltip with the reason and key.
    const labShot = at("pt1-s05-lab-selected");
    const lab = own(labShot.debug, "forge:lab")[0];
    step(5, lab && labShot.debug.selection?.length === 1 && labShot.debug.selection[0] === lab.unit_id, "the click does not select the Proving Lab");
    const beforeLab = checkpointBefore(checkpoints, labShot.screenshotIndex, 1);
    step(5, (labShot.debug.command_results ?? []).length === (beforeLab?.debug?.command_results ?? []).length, "selecting the lab sends a command");
    step(5, widget(labShot.debug, (row) => row.id === "info_name" && row.text === TEXT.labName), `the info panel does not show "${TEXT.labName}"`);
    step(5, widget(labShot.debug, (row) => row.id === "info_inventory" && /Shell \d+ available/.test(row.text ?? "")) ||
        widget(labShot.debug, (row) => row.id === "info_inventory" && /Main inventory 0 available/.test(row.text ?? "")), "the info panel shows no shell count");
    const entry = widget(labShot.debug, (row) => row.id.startsWith("research_entries/") && row.kind === "button");
    step(5, entry && entry.enabled === false && /Needs 10 Shell \(\d+\/10\)/.test(entry.text ?? ""), "the research entry is not disabled with \"Needs 10 Shell (n/10)\"");
    const researchKey = playerLabel(params.bindings.orders?.["forge:order_research"]?.[0]);
    const tooltipShot = at("pt1-s05-research-tooltip");
    const tooltips = widgets(tooltipShot.debug, (row) => /tooltip/.test(row.kind ?? "") || /tooltip/.test(row.id ?? "")).map((row) => row.text ?? "");
    step(5, tooltips.some((text) => text.includes("Needs 10 Shell (") && text.includes(researchKey)),
        `the research tooltip does not say "Needs 10 Shell (n/10)" and "${researchKey}"`);

    // Step 6: disable the press; its input fills; the loader alert within 10 s; enable again.
    const pressShot = at("pt1-s06-press-selected");
    const press = own(pressShot.debug, "forge:press")[0];
    step(6, press && pressShot.debug.selection?.[0] === press.unit_id, "the click does not select the Shell Press");
    const disableIndex = keyIndex(input, keys.keys.disable.Key.code, pressShot.screenshotIndex);
    step(6, disableIndex > 0, "the disable key of params.bindings was not pressed");
    const disabled = at("pt1-s06-disabled");
    const pressQueue = (debug) => debug.replicated_world?.economy?.queues?.find((queue) => queue.producer === press.unit_id);
    step(6, (disabled.debug.command_results ?? []).some((result) => Number.isInteger(result.applied_tick) && result.seq > Math.max(0, ...(pressShot.debug.command_results ?? []).map((item) => item.seq))),
        "the server did not apply the disable order");
    step(6, !pressQueue(disabled.debug) || pressQueue(disabled.debug).enabled === false, "the press is not disabled in the replica");
    const waits = series(input, checkpoints, "pt1-s06-wait-");
    const disableTime = input[disableIndex].wall_time_ms;
    const waiting = waits.find((shot) => alertTexts(shot.debug).includes(TEXT.loaderWaiting));
    step(6, waiting && waiting.wall_time_ms - disableTime <= 10000, `no alert "${TEXT.loaderWaiting}" within 10 s of the disable`);
    step(6, sounds(waiting.debug).some((id) => String(id).endsWith("forge_alert")), "the loader alert plays no alert sound");
    const enabled = at("pt1-s06-enabled");
    step(6, !pressQueue(enabled.debug) || pressQueue(enabled.debug).enabled === true, "the enable key does not start the press again");

    // Step 7: "Research ready" in the alert list; the research starts with a bar and seconds left.
    const ready = at("pt1-s07-research-ready");
    const readyShots = checkpoints.filter((shot) => shot.window === 1 && shot.input_call <= ready.screenshotIndex);
    step(7, readyShots.some((shot) => alertTexts(shot.debug).includes(TEXT.researchReady)), `no alert "${TEXT.researchReady}"`);
    step(7, widget(ready.debug, (row) => row.id.startsWith("research_entries/") && row.enabled === true &&
        row.widget_action?.content_id === "forge:hardened_shells"), "the research entry is not enabled when the lab can research");
    const started = at("pt1-s07-research-started");
    const appliedBefore = (ready.debug.command_results ?? []).filter((result) => Number.isInteger(result.applied_tick)).length;
    step(7, (started.debug.command_results ?? []).filter((result) => Number.isInteger(result.applied_tick)).length > appliedBefore,
        "the server did not apply the research order");
    step(7, widget(started.debug, (row) => row.id === "job_progress"), "the production queue shows no bar");
    step(7, widget(started.debug, (row) => row.id === "job_seconds" && /\d+(\.\d+)?s remaining/.test(row.text ?? "")), "the production queue shows no seconds left");

    // Step 8: "Research complete"; the turret shows damage 20, range 8 and the ammo count.
    const complete = series(input, checkpoints, "pt1-s08-wait-").find((shot) => alertTexts(shot.debug).includes(TEXT.researchComplete));
    step(8, complete, `no alert "${TEXT.researchComplete}"`);
    step(8, sounds(complete.debug).some((id) => String(id).endsWith("forge_alert")), "the research complete alert plays no sound");
    const turretShot = at("pt1-s08-turret-selected");
    step(8, widget(turretShot.debug, (row) => row.id === "info_damage" && row.text === "Damage: 20"), "the turret panel does not show \"Damage: 20\"");
    step(8, widget(turretShot.debug, (row) => row.id === "info_range" && row.text === "Range: 8.0"), "the turret panel does not show \"Range: 8.0\"");
    step(8, widget(turretShot.debug, (row) => row.id === "info_ammo" && /^Shells: \d+$/.test(row.text ?? "")), "the turret panel shows no ammo count");

    // Step 9: a right click on the ground with the turret selected: notice and error sound.
    const refusal = at("pt1-s09-cannot-move");
    const notices = [...(refusal.debug.notices ?? []).map((notice) => notice.text ?? notice.message ?? ""),
        ...widgets(refusal.debug, (row) => row.id === "last_notice").map((row) => row.text ?? "")];
    step(9, notices.some((text) => text.includes(TEXT.cannotMove)), `no notice "${TEXT.cannotMove}"`);
    step(9, (refusal.debug.frames?.retained_input ?? []).some((frame) => (frame.feedback?.audio_started ?? [])
        .some((sound) => sound.category === "effects" && String(sound.id).endsWith("error"))), "the refusal plays no error sound");

    // Step 10: window B joins in the colour of slot 1.
    const joinedB = at("pt1-s10-joined");
    step(10, joinedB.window === 2 && joinedB.debug.slot === 1 && joinedB.debug.phase?.startsWith("Joined"), "window B did not join slot 1");
    const ownDrones = own(joinedB.debug, "forge:drone");
    const imageB = await image(view, "pt1-s10-joined");
    step(10, ownDrones.some((drone) => {
        const shownUnit = joinedB.scene.units.find((unit) => unit.unit_id === drone.unit_id);
        return shownUnit && inViewport(joinedB.scene, shownUnit.ground_screen_center) &&
            colourNear(imageB, shownUnit.ground_screen_center, SLOT_RGB[1], 10);
    }), "the camera of window B does not show an own drone in the colour of slot 1");

    // Step 11: the foreign turret: gapped ring, empty command card, range circle.
    const foreign = at("pt1-s11-turret-selected");
    const turretId = foreign.debug.selection?.[0];
    const turretUnit = replicaUnit(foreign.debug, turretId);
    step(11, foreign.debug.selection?.length === 1 && turretUnit?.content_id === "forge:turret" && turretUnit.owner_slot !== 1, "the click does not select the foreign turret");
    step(11, (foreign.debug.rings_shown ?? []).some((ring) => ring.unit === turretId && ring.own === false), "the turret ring is not the foreign ring with gaps");
    step(11, !widget(foreign.debug, (row) => row.id.startsWith("command_rows/")), "the command card is not empty for the foreign turret");
    const turretScene = foreign.scene.units.find((unit) => unit.unit_id === turretId);
    step(11, turretScene && rangeCircleHits(await image(view, "pt1-s11-turret-selected"), foreign.scene, turretScene, 8) >= 5, "no range circle round the turret");

    // Step 12: the box drag selects the drone; the selection panel shows it with health.
    const boxed = at("pt1-s12-drone-selected");
    const droneId = boxed.debug.selection?.[0];
    step(12, boxed.debug.selection?.length === 1 && replicaUnit(boxed.debug, droneId)?.content_id === "forge:drone" &&
        replicaUnit(boxed.debug, droneId)?.owner_slot === 1, "the box drag does not select exactly the own drone");
    step(12, widget(boxed.debug, (row) => row.id.startsWith("selection_units/") && /HP/.test(row.text ?? "")), "the selection panel shows no drone row with health");

    // Step 13: group 1, a clear click, recall, and recall twice to centre the camera.
    step(13, JSON.stringify(at("pt1-s13-cleared").debug.groups?.["1"]) === JSON.stringify([droneId]), "group 1 does not hold the drone");
    step(13, (at("pt1-s13-cleared").debug.selection ?? []).length === 0, "the click on empty ground does not clear the selection");
    step(13, JSON.stringify(at("pt1-s13-recalled").debug.selection) === JSON.stringify([droneId]), "the group key does not select the drone again");
    const centred = at("pt1-s13-centred");
    const centredDrone = centred.scene.units.find((unit) => unit.unit_id === droneId);
    const [vx, vy, vw, vh] = centred.scene.viewport_logical_rect;
    step(13, centredDrone && Math.hypot(centredDrone.ground_screen_center[0] - (vx + vw / 2), centredDrone.ground_screen_center[1] - (vy + vh / 2)) <= 48,
        "the group key twice does not centre the camera on the drone");

    // Step 14: a faster scroll speed and a new Stop key.
    const slow = Math.hypot(...[0, 1].map((axis) => centre({ checkpoint: at("pt1-s14-pan-slow") })[axis] - centre({ checkpoint: centred })[axis]));
    const fastFrom = at("pt1-s14-menu-closed");
    const fast = Math.hypot(...[0, 1].map((axis) => centre({ checkpoint: at("pt1-s14-pan-fast") })[axis] - centre({ checkpoint: fastFrom })[axis]));
    step(14, fast > slow * 1.1, `the scroll speed change does not make the pan faster (${slow.toFixed(3)} then ${fast.toFixed(3)})`);
    // The two menu clicks land on the "Key stop" Change button and on the Scroll speed row of
    // the rendered settings menu (ui_snapshot kind settings_menu, text_nodes with button_rect).
    const scrolled = at("pt1-s14-menu-scrolled");
    const menuRow = (scrolled.debug.ui_snapshot ?? []).find((row) => row.kind === "settings_menu" && row.visible === true);
    step(14, menuRow, "F10 does not open the settings menu");
    const nodes = menuRow.text_nodes ?? [];
    const clicksAfter = input.map((item, index) => ({ item, index })).filter(({ item, index }) =>
        index > scrolled.screenshotIndex && item.call === "click" && item.args?.[2] === "left").slice(0, 2);
    const inside = (rect, x, y) => Array.isArray(rect) && x >= rect[0] && x <= rect[0] + rect[2] && y >= rect[1] && y <= rect[1] + rect[3];
    const speedLabel = nodes.find((node) => node.text === "Scroll speed" && node.visible === true);
    step(14, speedLabel && clicksAfter[0] && Math.abs(clicksAfter[0].item.args[1] - (speedLabel.rect[1] + speedLabel.rect[3] / 2)) <= 12,
        "the scroll speed click is not on the Scroll speed row of the menu");
    const stopLabel = nodes.findIndex((node) => node.text === "Key stop" && node.visible === true);
    const change = stopLabel >= 0 ? nodes.slice(stopLabel + 1, stopLabel + 4).find((node) => node.text === "Change") : null;
    step(14, change && clicksAfter[1] && inside(change.button_rect, clicksAfter[1].item.args[0], clicksAfter[1].item.args[1]),
        "the rebind click is not on the Change button of the Key stop row");
    const newStop = keys.newStop;
    step(14, JSON.stringify(fastFrom.debug.effective_bindings?.actions?.key_stop) === JSON.stringify([newStop]),
        `Stop is not bound to the new key ${newStop.Key.code}`);
    const after = at("pt1-s14-controls-after-rebind");
    step(14, controlsRows(after.debug).some((row) => row.text === `${humanizeAction("key_stop")}: ${playerLabel(newStop)}`),
        "guide sheet 5 does not show the new Stop key after the rebind");

    // Step 15: the reload rejoins the same slot; group 1 and the new Stop key stay.
    const loadB = series(input, checkpoints, "pt1-s15-load-");
    step(15, loadB.some((shot) => widget(shot.debug, (row) => row.id === "shell.load.progress.track")), "window B shows no load screen after the reload");
    const rejoined = at("pt1-s15-rejoined");
    step(15, rejoined.debug.slot === 1 && typeof rejoined.debug.player_id_decimal === "string" &&
        rejoined.debug.player_id_decimal === fastFrom.debug.player_id_decimal, "the reload does not rejoin the same slot with the stored id");
    step(15, JSON.stringify(rejoined.debug.groups?.["1"]) === JSON.stringify([droneId]), "group 1 is lost after the reload");
    step(15, JSON.stringify(rejoined.debug.effective_bindings?.actions?.key_stop) === JSON.stringify([newStop]), "the new Stop key is lost after the reload");
    const recalled = at("pt1-s15-group-recalled");
    step(15, JSON.stringify(recalled.debug.selection) === JSON.stringify([droneId]), "group 1 does not work after the reload");

    // Step 16: two queued points in order; the new Stop key stops the drone.
    const ground = async (file) => {
        const target = JSON.parse(await view.serverFile(file)).ground;
        const [x, y] = unprojectGround(recalled.scene, target);
        return { x, y };
    };
    const point1 = await ground("pt1-s16-point-1-ground.json");
    const point2 = await ground("pt1-s16-point-2-ground.json");
    const waypoints = at("pt1-s16-waypoints");
    step(16, (waypoints.debug.markers_shown ?? []).filter((marker) => (marker.units ?? []).includes(droneId)).length >= 2, "no waypoint lines for the two queued points");
    const stopIndex = keyIndex(input, newStop.Key.code, waypoints.screenshotIndex);
    step(16, stopIndex > 0, "the driver did not press the new Stop key");
    const stopShot = checkpointBefore(checkpoints, stopIndex, 2);
    const restShot = at("pt1-s16-at-rest");
    const startTick = waypoints.debug.replicated_world.tick;
    const stopTick = stopShot.debug.replicated_world.tick;
    const track = unitTrack(tickLog, droneId);
    const near = (point, target) => Math.hypot(point.x - target.x, point.y - target.y);
    const first1 = track.find((point) => point.tick >= startTick && near(point, point1) <= 0.6);
    step(16, first1 && first1.tick <= stopTick + 2, "the drone does not reach the first point before the Stop key");
    const settle = track.filter((point) => point.tick >= stopTick + 10 && point.tick <= restShot.debug.replicated_world.tick);
    step(16, settle.length >= 2 && settle.every((point) => near(point, settle[0]) <= 0.01), "the new Stop key did not stop the drone");
    step(16, settle.length >= 1 && near(settle[0], point2) > 0.5, "the drone reached the second point: the new Stop key did not stop it");

    // Step 17: a marker at once; the drone moves and stops on the point; the marker stays.
    const markerShot = at("pt1-s17-marker");
    const target = JSON.parse(await view.serverFile("pt1-s17-point-ground.json")).ground;
    const [tx, ty] = unprojectGround(recalled.scene, target);
    step(17, (markerShot.debug.markers_shown ?? []).some((marker) => (marker.units ?? []).includes(droneId)), "no marker in the first screenshot after the right click");
    const moves = series(input, checkpoints, "pt1-s17-move-");
    const restTick = (() => {
        const later = track.filter((point) => point.tick >= markerShot.debug.replicated_world.tick);
        const index = later.findIndex((point, i) => i + 5 < later.length && later.slice(i, i + 5).every((next) => near(next, point) <= 0.001));
        return index >= 0 ? later[index] : null;
    })();
    step(17, restTick && Math.hypot(restTick.x - tx, restTick.y - ty) <= 0.1, "the drone does not stop on the ordered point");
    step(17, moves.filter((shot) => shot.debug.replicated_world.tick < restTick.tick)
        .every((shot) => (shot.debug.markers_shown ?? []).some((marker) => (marker.units ?? []).includes(droneId))), "the marker goes before the drone rests");

    // Steps 18 and 19: damage with the alert and ping; destruction leaves selection and group 1.
    const late = series(input, checkpoints, "pt1-s18-");
    const damaged = late.find((shot) => {
        const health = (shot.debug.replicated_world.healths ?? []).find((record) => record.unit === droneId);
        return health && health.current < (replicaUnit(shot.debug, droneId)?.max_health ?? Infinity);
    });
    step(18, damaged, "the health of the drone does not drop");
    step(18, late.some((shot) => alertTexts(shot.debug).includes(TEXT.underAttack)), `no alert "${TEXT.underAttack}"`);
    step(18, late.some((shot) => (shot.debug.alert_pings ?? []).some((ping) => ping.unit === droneId)), "the damage alert does not ping the minimap");
    const gone = late.find((shot) => !replicaUnit(shot.debug, droneId));
    step(19, gone, "the drone is not destroyed");
    step(19, !(gone.debug.selection ?? []).includes(droneId), "the destroyed drone stays in the selection");
    step(19, !(gone.debug.groups?.["1"] ?? []).includes(droneId), "the destroyed drone stays in group 1");
    step(19, late.filter((shot) => shot.input_call >= gone.input_call).some((shot) => alertTexts(shot.debug).includes(TEXT.destroyed)), `no alert "${TEXT.destroyed}"`);
}

module.exports = { judge, assertControlsMatch, humanizeAction, playerLabel };
