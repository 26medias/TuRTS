// turts-test-role: oracle
"use strict";

// PT1 oracle: the "Player path" list of the MC design, steps 1 to 19, and guide sheet 5
// (Controls) as rendered against the effective input map. It replaces the asserts of the legacy
// tests/browser/m3-human.js.
//
// Ruling C386: the oracle judges any player of the harness io contract (the pt1-driver.js
// instrument, or the `player` agent of task 23). It takes its evidence only from harness records:
// the input log, the scene checkpoints and the harness screenshots (harness-shot-<n>.png) in
// capture order, the startup and reload load records, the storage audit, params.bindings (the
// effective map that scripts/mc-browser.sh reads from the packed copy) and the server files. It
// reads no driver-written file and no driver-chosen screenshot name. The texts are the forge
// texts of mods/forge/interface/hud.ron and mods/forge/units/*.ron.
const assert = require("node:assert/strict");
const { decodePng, pixelAt } = require("./harness/png.js");
const { assertNoDriverStop, readCheckpoints, replicaUnit, unitTrack, widget, widgets } = require("./checkpoint-lib.js");
const { unprojectGround } = require("./hud4-oracle.js");
const { tapGapReceipt, retainedFrames, assertPressesInDifferentFrames } = require("./tap-receipt.js");
const { project, visibleRingPixels } = require("./vis-full-oracle.js");

const SLOT_RGB = [[0, 114, 178], [230, 159, 0]];
const RANGE_RGB = [242, 204, 96];
const TURRET_RANGE = 8;
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

// The product window of a double tap of a group key (crates/turts_interaction/src/lib.rs:1626, double_click_ms).
const PT1_DOUBLE_TAP_WINDOW_MS = 400;

// Both double taps of the driver (step 13 and step 15) are tap steps (tests/browser/harness/driver-io.js tapKey). Their
// page-measured gap must be inside the product window: a slow tap names the instrument, never the product.
function judgeTapSteps(input) {
    const taps = input.map((entry, index) => ({ entry, index })).filter(({ entry }) => entry.call === "key" && entry.args?.[2]?.taps === 2);
    assert.equal(taps.length, 2, `PT1 needs exactly two double tap steps in the input log, found ${taps.length}: instrument fault`);
    return new Map(taps.map(({ entry, index }) => [index, tapGapReceipt(entry, { code: entry.args[0], windowMs: PT1_DOUBLE_TAP_WINDOW_MS })]));
}

// ---- Labels of the rendered Controls rows --------------------------------------------------

// Rust `humanize_action` (crates/turts_client/src/widgets/sources.rs), for the actions of no order.
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

const NAMED_KEYS = { Escape: "Esc", Space: "Space", ArrowUp: "Up", ArrowDown: "Down", ArrowLeft: "Left", ArrowRight: "Right" };

// Rust BindingLabels::from_layout_entries (crates/turts_client/src/client_shell.rs:52-74): the
// browser keyboard layout map that the harness records at startup (harness/run.js recordStartup).
function layoutLabels(layout) {
    const labels = {};
    for (const [code, label] of Object.entries(layout ?? {})) {
        if (typeof label !== "string" || label.length === 0 || /^\s+$/.test(label) || /[\u0000-\u001f\u007f]/.test(label)) continue;
        labels[code] = label.length === 1 && /[a-z]/.test(label) ? label.toUpperCase() : label;
    }
    return labels;
}

// Rust `binding_label` and `key_label` (crates/turts_client/src/client_shell.rs:80-133).
function playerLabel(binding, labels = {}) {
    if (binding?.Key) {
        const mods = ["Ctrl", "Shift", "Alt"].filter((mod) => (binding.Key.mods ?? []).includes(mod)).map((mod) => `${mod}+`).join("");
        const code = binding.Key.code;
        const key = labels[code] ?? (/^Key.$/.test(code) ? code.slice(3) : /^Digit.$/.test(code) ? code.slice(5) : NAMED_KEYS[code] ?? code);
        return `${mods}${key}`;
    }
    if (binding?.Wheel) return `Wheel ${String(binding.Wheel).toLowerCase()}`;
    if (binding?.Pointer) return `${(binding.Pointer.mods ?? []).map((mod) => `${mod}+`).join("")}${binding.Pointer.button} click`;
    return JSON.stringify(binding);
}

// Rust `action_label` (crates/turts_client/src/widgets/sources.rs): a row of a mod action takes
// the name of the forge order that has the action as its hotkey (mods/forge/orders/*.ron).
const FORGE_ORDER_LABELS = {
    key_cancel: "Cancel this job", key_disable: "Disable this machine", key_enable: "Enable this machine",
    key_hold: "Hold this position", key_move: "Move selected units", key_remove: "Remove this machine",
    key_research: "Research an upgrade", key_stop: "Stop selected units",
};

function actionLabel(action) {
    return FORGE_ORDER_LABELS[action] ?? humanizeAction(action);
}

function rowText(action, bindings, labels = {}) {
    return `${actionLabel(action)}: ${bindings.map((binding) => playerLabel(binding, labels)).join(", ")}`;
}

function controlsRows(debug) {
    return widgets(debug, (row) => row.id.startsWith("guide.4.bindings/") && typeof row.text === "string");
}

// Guide sheet 5 as rendered matches the effective map: each visible row is the row of one action.
function assertControlsMatch(debug, actions, label, labels = {}) {
    const expected = Object.entries(actions).map(([name, list]) => [name, rowText(name, list, labels)]);
    const rows = controlsRows(debug);
    step("guide-sheet-5", rows.length > 0, `${label}: guide sheet 5 (Controls) shows no binding row`);
    const matched = new Set();
    for (const row of rows) {
        const found = expected.find(([, text]) => text === row.text);
        step("guide-sheet-5", found, `${label}: the rendered Controls row "${row.text}" matches no binding of the effective map`);
        matched.add(found[0]);
    }
    return { rows: rows.map((row) => row.text), matched };
}

// ---- Harness records -------------------------------------------------------------------------

function normaliseMod(name) {
    if (/^(Ctrl|Control)/.test(name)) return "Ctrl";
    if (/^Shift/.test(name)) return "Shift";
    if (/^(Alt|Option)/.test(name)) return "Alt";
    return name;
}

// The input-log entries that press `binding` (its code, with exactly its modifiers).
function presses(input, binding, windowNumber, from = -1, to = Infinity) {
    const key = binding?.Key;
    if (!key) return [];
    const mods = [...(key.mods ?? [])].sort().join("+");
    return input.map((entry, index) => ({ entry, index })).filter(({ entry, index }) => index > from && index < to &&
        entry.window === windowNumber && entry.call === "key" && entry.args?.[0] === key.code && (entry.args?.[2]?.action ?? "press") !== "up" &&
        [...new Set((entry.args?.[1] ?? []).map(normaliseMod))].sort().join("+") === mods);
}

function clicks(input, windowNumber, button, from = -1, to = Infinity) {
    return input.map((entry, index) => ({ entry, index })).filter(({ entry, index }) => index > from && index < to &&
        entry.window === windowNumber && entry.call === "click" && (entry.args?.[2] ?? "left") === button);
}

function inside(rect, x, y, scale = 1) {
    return Array.isArray(rect) && x * scale >= rect[0] && x * scale <= rect[0] + rect[2] && y * scale >= rect[1] && y * scale <= rect[1] + rect[3];
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

function applied(debug) {
    return (debug?.command_results ?? []).filter((result) => Number.isInteger(result.applied_tick)).length;
}

function guideOpen(debug) {
    return Boolean(widget(debug, (row) => row.id === "guide_copy"));
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

function near(point, target) {
    return Math.hypot(point.x - target.x, point.y - target.y);
}

function makeContext(input, checkpoints, view) {
    const ordered = [...checkpoints].sort((left, right) => left.input_call - right.input_call);
    const win = (windowNumber) => ordered.filter((shot) => shot.window === windowNumber);
    return {
        input,
        win,
        // The first checkpoint of a window after input index `from` (and before `to`) that passes.
        first(windowNumber, predicate, from = -1, to = Infinity) {
            return win(windowNumber).find((shot) => shot.input_call > from && shot.input_call < to && predicate(shot)) ?? null;
        },
        all(windowNumber, predicate, from = -1, to = Infinity) {
            return win(windowNumber).filter((shot) => shot.input_call > from && shot.input_call < to && predicate(shot));
        },
        before(windowNumber, index) {
            return win(windowNumber).filter((shot) => shot.input_call < index).at(-1) ?? null;
        },
        async image(shot) {
            return decodePng(await view.serverBytes(`harness-shot-${shot.input_call}.png`));
        },
    };
}

// ---- The steps -----------------------------------------------------------------------------

function judgeLoad(observations, number, label) {
    step(number, observations.some((item) => !String(item.phase ?? "").startsWith("Joined") &&
        item.rows.some((row) => row.id === "shell.load.progress.track" && row.visible === true)), `${label} shows no load screen with a progress bar`);
    const phases = new Set(observations.filter((item) => item.phase && !item.phase.startsWith("Joined")).map((item) => item.phase.split(/[ {(]/)[0]));
    step(number, phases.size >= 2, `${label} shows fewer than two load phases (${[...phases].join(", ")})`);
}

function clickToPlayShown(debug) {
    return (debug?.ui_snapshot ?? debug?.rows ?? []).some((row) => row.kind === "focus_overlay" && row.visible === true &&
        (row.text_nodes ?? []).some((node) => node.text === "Click to play" && node.visible === true));
}

async function judgeRun(view, params) {
    const input = view.inputLog();
    const checkpoints = await readCheckpoints(view);
    const ctx = makeContext(input, checkpoints, view);
    const startActions = params.bindings?.actions;
    assert(startActions && typeof startActions === "object", "PT1 needs params.bindings, the effective input map");
    const tickLog = await view.serverFile("server-tick.log");
    const startup = JSON.parse(await view.serverFile("startup-observations.json"));
    const labelsOf = (windowNumber) => layoutLabels(startup.find((item) => item.window === windowNumber && "keyboard_layout" in item)?.keyboard_layout);
    const A = 1;
    const B = 2;
    const tapReceipts = judgeTapSteps(input);

    // Step 1: window A, the load screen shows each phase and a progress bar.
    judgeLoad(startup.filter((item) => item.window === A), 1, "window A");

    // Step 2: "Click to play"; a click opens the guide; the camera shows the own factory.
    const firstA = ctx.first(A, () => true);
    step(2, firstA, "window A has no screenshot");
    step(2, clickToPlayShown(firstA.debug) || startup.filter((item) => item.window === A).some((item) => clickToPlayShown(item)),
        "\"Click to play\" is not shown before the first click");
    const openA = ctx.first(A, (shot) => guideOpen(shot.debug) && !clickToPlayShown(shot.debug));
    step(2, openA && clicks(input, A, "left", -1, openA.input_call).length > 0, "no click opens the guide");
    const copy = widget(openA.debug, (row) => row.id === "guide_copy");
    step(2, copy.text.includes(TEXT.guideFirst), `the guide does not show sheet 1 ("${TEXT.guideFirst}")`);
    step(2, (openA.scene.units ?? []).some((unit) => inViewport(openA.scene, unit.ground_screen_center) &&
        replicaUnit(openA.debug, unit.unit_id)?.owner_slot === openA.debug.slot), "the camera shows no own factory unit");

    // Guide sheet 5 matches the effective map that the client uses (gate 2 ruling 14).
    assert.deepEqual(openA.debug.effective_bindings?.actions, startActions,
        "PT1 step guide-sheet-5: the client effective map differs from params.bindings");
    const sheetShots = ctx.all(A, (shot) => controlsRows(shot.debug).length > 0);
    const matched = new Set();
    for (const shot of sheetShots) for (const name of assertControlsMatch(shot.debug, shot.debug.effective_bindings?.actions ?? startActions, `checkpoint ${shot.input_call}`, labelsOf(A)).matched) matched.add(name);
    for (const name of ["key_stop", "menu", "toggle_guide", "group_assign_1", "group_recall_1", "group_add_1", "queue_modifier"]) {
        step("guide-sheet-5", matched.has(name), `guide sheet 5 shows no row for ${name}`);
    }

    // Step 3: the guide closes with its key or its Close button.
    const closedA = ctx.first(A, (shot) => !guideOpen(shot.debug), openA.input_call);
    step(3, closedA, "the guide never closes");
    const closeInputs = [...presses(input, startActions.toggle_guide[0], A, openA.input_call, closedA.input_call),
        ...clicks(input, A, "left", openA.input_call, closedA.input_call).filter(({ entry }) => {
            const close = widget(ctx.before(A, closedA.input_call).debug, (row) => row.id === "guide_close");
            return close && inside(close.rect, entry.args[0], entry.args[1], openA.scene.scale_factor || 1);
        })];
    step(3, closeInputs.length > 0, "no F1 press or Close click closes the guide");

    // Step 4: pan (a pan key), zoom (the wheel), the hover ring at once and the tooltip after 0.5 s.
    const panKeys = ["camera_pan_left", "camera_pan_right", "camera_pan_up", "camera_pan_down"].flatMap((name) => presses(input, startActions[name][0], A));
    step(4, panKeys.some(({ index }) => {
        const before = ctx.before(A, index);
        const after = ctx.first(A, () => true, index);
        return before && after && JSON.stringify(before.scene.clip_from_world) !== JSON.stringify(after.scene.clip_from_world);
    }), "no pan key moves the camera");
    const wheels = input.map((entry, index) => ({ entry, index })).filter(({ entry }) => entry.window === A && entry.call === "wheel");
    step(4, wheels.some(({ index }) => {
        const before = ctx.before(A, index);
        const after = ctx.first(A, () => true, index);
        return before && after && !guideOpen(before.debug) && JSON.stringify(before.scene.clip_from_world) !== JSON.stringify(after.scene.clip_from_world);
    }), "the wheel does not zoom the camera");
    const tooltipShot = ctx.first(A, (shot) => widgets(shot.debug, (row) => row.kind === "world_tooltip" && row.text === TEXT.drillTooltip).length > 0);
    step(4, tooltipShot, `no world tooltip "${TEXT.drillTooltip}"`);
    const hoverMove = input.map((entry, index) => ({ entry, index })).filter(({ entry, index }) => entry.window === A &&
        entry.call === "move" && index < tooltipShot.input_call).at(-1);
    step(4, hoverMove && tooltipShot.wall_time_ms - hoverMove.entry.wall_time_ms >= 500, "the world tooltip shows less than 0.5 s after the pointer came to the drill");
    const early = ctx.all(A, (shot) => shot.wall_time_ms - hoverMove.entry.wall_time_ms < 450, hoverMove.index, tooltipShot.input_call);
    step(4, early.every((shot) => !widgets(shot.debug, (row) => row.kind === "world_tooltip").length), "the world tooltip shows before 0.5 s");
    const ringShot = early[0] ?? tooltipShot;
    const drill = own(ringShot.debug, "forge:drill")[0];
    const drillScene = ringShot.scene.units.find((unit) => unit.unit_id === drill?.unit_id);
    step(4, drillScene && visibleRingPixels(await ctx.image(ringShot), ringShot.scene, { ...drillScene, owner_slot: drill.owner_slot }, drillScene.pick_radius),
        "no hover ring round the drill");

    // Step 5: the lab panel; the disabled research entry; its tooltip with the reason and the key.
    const labShot = ctx.first(A, (shot) => {
        const lab = own(shot.debug, "forge:lab")[0];
        return lab && shot.debug.selection?.length === 1 && shot.debug.selection[0] === lab.unit_id;
    });
    step(5, labShot, "no click selects the Proving Lab");
    const lab = own(labShot.debug, "forge:lab")[0];
    const labClick = clicks(input, A, "left", -1, labShot.input_call).at(-1);
    step(5, labClick && applied(ctx.before(A, labClick.index)?.debug) === applied(labShot.debug), "selecting the lab sends a command");
    step(5, widget(labShot.debug, (row) => row.id === "info_name" && row.text === TEXT.labName), `the info panel does not show "${TEXT.labName}"`);
    step(5, widget(labShot.debug, (row) => row.id === "info_inventory" && /(Shell \d+ available|Main inventory 0 available)/.test(row.text ?? "")),
        "the info panel shows no shell count");
    const shortage = ctx.first(A, (shot) => shot.debug.selection?.[0] === lab.unit_id && widget(shot.debug, (row) =>
        row.id.startsWith("research_entries/") && row.kind === "button" && row.enabled === false && /Needs 10 Shell \(\d+\/10\)/.test(row.text ?? "")));
    step(5, shortage, "the research entry is never disabled with \"Needs 10 Shell (n/10)\" while the lab is selected");
    const researchTooltip = ctx.first(A, (shot) => {
        const key = shot.debug.effective_bindings?.orders?.["forge:order_research"]?.[0];
        const keyLabel = key ? playerLabel(key, labelsOf(A)) : null;
        return widgets(shot.debug, (row) => /tooltip/.test(`${row.kind ?? ""} ${row.id ?? ""}`)).some((row) =>
            (row.text ?? "").includes("Needs 10 Shell (") && keyLabel && row.text.includes(keyLabel));
    });
    step(5, researchTooltip, "the research tooltip never says \"Needs 10 Shell (n/10)\" with the research key");

    // Step 6: disable the press (key or card button); its input fills; the loader alert within
    // 10 s with a sound; enable it again, and the arm loader moves.
    const pressShot = ctx.first(A, (shot) => own(shot.debug, "forge:press").some((press) => shot.debug.selection?.length === 1 && shot.debug.selection[0] === press.unit_id));
    step(6, pressShot, "no click selects the Shell Press");
    const press = own(pressShot.debug, "forge:press")[0];
    const queueOf = (debug) => debug.replicated_world?.economy?.queues?.find((queue) => queue.producer === press.unit_id);
    const disabled = ctx.first(A, (shot) => queueOf(shot.debug)?.enabled === false, pressShot.input_call);
    step(6, disabled, "the press is never disabled in the replica");
    const disableInput = input.map((entry, index) => ({ entry, index })).filter(({ entry, index }) => entry.window === A &&
        index > pressShot.input_call && index < disabled.input_call && (entry.call === "key" || entry.call === "click")).at(-1);
    step(6, disableInput, "no key or click disables the press");
    step(6, ctx.first(A, (shot) => alertTexts(shot.debug).includes(TEXT.loaderWaiting) &&
        shot.wall_time_ms - disableInput.entry.wall_time_ms <= 10000, disableInput.index), `no alert "${TEXT.loaderWaiting}" within 10 s of the disable`);
    step(6, ctx.first(A, (shot) => sounds(shot.debug).some((id) => String(id).endsWith("forge_alert")), disableInput.index), "the loader alert plays no alert sound");
    step(6, ctx.first(A, (shot) => (shot.debug.replicated_world.economy.inventories ?? []).some((row) => row.unit === press.unit_id &&
        row.port === "main" && (row.items ?? []).some((item) => item.on_hand >= 6)), disableInput.index), "the press input does not fill to 6 ore while the press is off");
    const enabled = ctx.first(A, (shot) => queueOf(shot.debug)?.enabled === true, disabled.input_call);
    step(6, enabled, "the press is never enabled again");
    // M1: the arm loader that feeds the press (inserter destination) moves again after the enable.
    const loaderStage = (debug) => (debug.replicated_world.inserters ?? []).filter((row) => row.destination === press.unit_id)
        .map((row) => JSON.stringify([row.stage, row.elapsed]));
    const stalled = loaderStage(disabled.debug);
    step(6, stalled.length > 0, "the replica has no arm loader that feeds the press");
    step(6, ctx.first(A, (shot) => shot.wall_time_ms - enabled.wall_time_ms <= 5000 && JSON.stringify(loaderStage(shot.debug)) !== JSON.stringify(stalled),
        enabled.input_call), "the arm loader does not move within 5 s of the enable");

    // Step 7: "Research ready" in the alert list; the research starts with a bar and seconds left.
    step(7, ctx.first(A, (shot) => alertTexts(shot.debug).includes(TEXT.researchReady)), `no alert "${TEXT.researchReady}"`);
    const started = ctx.first(A, (shot) => (shot.debug.replicated_world.economy.queues ?? []).some((queue) => queue.producer === lab.unit_id &&
        queue.jobs.some((job) => job.recipe === "forge:hardened_shells")) && widget(shot.debug, (row) => row.id === "job_progress") &&
        widget(shot.debug, (row) => row.id === "job_seconds" && /\d+(\.\d+)?s remaining/.test(row.text ?? "")));
    step(7, started, "the research never shows in the production queue with a bar and the seconds left");
    const readyShot = ctx.all(A, (shot) => widget(shot.debug, (row) => row.id.startsWith("research_entries/") && row.enabled === true &&
        row.widget_action?.content_id === "forge:hardened_shells"), -1, started.input_call).at(-1);
    step(7, readyShot, "the research entry is not enabled before the research starts");
    step(7, applied(started.debug) > applied(readyShot.debug), "the server did not apply the research order");

    // Step 8: "Research complete" with a sound; the turret panel shows damage 20, range 8, ammo.
    const complete = ctx.first(A, (shot) => alertTexts(shot.debug).includes(TEXT.researchComplete), started.input_call);
    step(8, complete, `no alert "${TEXT.researchComplete}"`);
    step(8, ctx.first(A, (shot) => sounds(shot.debug).some((id) => String(id).endsWith("forge_alert")), complete.input_call - 1), "the research complete alert plays no sound");
    const turretShot = ctx.first(A, (shot) => widget(shot.debug, (row) => row.id === "info_damage" && row.text === "Damage: 20") &&
        widget(shot.debug, (row) => row.id === "info_range" && row.text === "Range: 8.0") &&
        widget(shot.debug, (row) => row.id === "info_ammo" && /^Shells: \d+$/.test(row.text ?? "")), complete.input_call - 1);
    step(8, turretShot, "the turret panel never shows \"Damage: 20\", \"Range: 8.0\" and the ammo count");

    // Step 9: a right click on the ground with the turret selected: notice and error sound.
    const turretId = turretShot.debug.selection?.[0];
    const refusal = clicks(input, A, "right", turretShot.input_call - 1).find(({ index }) => ctx.before(A, index)?.debug.selection?.[0] === turretId);
    step(9, refusal, "no right click while the turret is selected");
    const notice = ctx.first(A, (shot) => [...(shot.debug.notices ?? []).map((item) => item.text ?? item.message ?? ""),
        ...widgets(shot.debug, (row) => row.id === "last_notice").map((row) => row.text ?? "")].some((text) => text.includes(TEXT.cannotMove)), refusal.index);
    step(9, notice, `no notice "${TEXT.cannotMove}"`);
    step(9, (notice.debug.frames?.retained_input ?? []).some((frame) => (frame.feedback?.audio_started ?? [])
        .some((sound) => sound.category === "effects" && String(sound.id).endsWith("error"))), "the refusal plays no error sound");

    // Step 10: window B: the load screen, then the own drone in the colour of slot 1.
    judgeLoad(startup.filter((item) => item.window === B), 10, "window B");
    // The start camera of window B shows an own drone in the colour of slot 1: a window B
    // screenshot before the first drag (step 12), with no overlay. The overlay and its fade
    // darken the scene (run t21-run118-pt1-2: drone pixel (61, 43, 20) under it, (194, 141, 43)
    // after), so the oracle takes the first such screenshot that shows the colour.
    const firstB = ctx.first(B, (shot) => shot.debug.phase?.startsWith("Joined"));
    step(10, firstB && firstB.debug.slot === 1, "window B did not join slot 1");
    const firstDragB = input.findIndex((entry) => entry.window === B && entry.call === "drag");
    let droneShown = false;
    for (const shot of ctx.all(B, (item) => item.debug.phase?.startsWith("Joined") &&
        !(item.debug.ui_snapshot ?? []).some((row) => row.kind === "focus_overlay" && row.visible === true), -1, firstDragB < 0 ? Infinity : firstDragB)) {
        const imageB = await ctx.image(shot);
        droneShown = own(shot.debug, "forge:drone").some((drone) => {
            const shown = shot.scene.units.find((unit) => unit.unit_id === drone.unit_id);
            return shown && inViewport(shot.scene, shown.ground_screen_center) && colourNear(imageB, shown.ground_screen_center, SLOT_RGB[1], 10);
        });
        if (droneShown) break;
    }
    step(10, droneShown, "the camera of window B does not show an own drone in the colour of slot 1");

    // Step 11: the foreign turret: gapped ring, empty command card, range circle.
    const foreign = ctx.first(B, (shot) => {
        const unit = replicaUnit(shot.debug, shot.debug.selection?.[0]);
        return shot.debug.selection?.length === 1 && unit?.content_id === "forge:turret" && unit.owner_slot !== shot.debug.slot;
    });
    step(11, foreign, "no click selects the foreign turret");
    const foreignTurret = foreign.debug.selection[0];
    step(11, (foreign.debug.rings_shown ?? []).some((ring) => ring.unit === foreignTurret && ring.own === false), "the turret ring is not the foreign ring with gaps");
    step(11, !widget(foreign.debug, (row) => row.id.startsWith("command_rows/")), "the command card is not empty for the foreign turret");
    const turretScene = foreign.scene.units.find((unit) => unit.unit_id === foreignTurret);
    step(11, turretScene && rangeCircleHits(await ctx.image(foreign), foreign.scene, turretScene, TURRET_RANGE) >= 5, "no range circle round the turret");

    // Step 12: a box drag selects the own drone; the selection panel shows it with health.
    const drags = input.map((entry, index) => ({ entry, index })).filter(({ entry }) => entry.window === B && entry.call === "drag");
    const boxed = drags.map(({ index }) => ctx.first(B, (shot) => {
        const unit = replicaUnit(shot.debug, shot.debug.selection?.[0]);
        return shot.debug.selection?.length === 1 && unit?.content_id === "forge:drone" && unit.owner_slot === 1;
    }, index)).find(Boolean);
    step(12, boxed, "no box drag selects exactly the own drone");
    const droneId = boxed.debug.selection[0];
    step(12, widget(boxed.debug, (row) => row.id.startsWith("selection_units/") && /HP/.test(row.text ?? "")), "the selection panel shows no drone row with health");

    // Step 13: group 1 holds the drone; a clear click; recall selects it; recall twice centres it.
    const assigned = ctx.first(B, (shot) => JSON.stringify(shot.debug.groups?.["1"]) === JSON.stringify([droneId]), boxed.input_call - 1);
    step(13, assigned, "group 1 never holds the drone");
    const cleared = ctx.first(B, (shot) => (shot.debug.selection ?? []).length === 0, assigned.input_call - 1);
    step(13, cleared, "no click on empty ground clears the selection");
    const effectiveB = (shot) => shot?.debug?.effective_bindings?.actions ?? startActions;
    const recallPresses = presses(input, effectiveB(cleared).group_recall_1[0], B, cleared.input_call);
    step(13, recallPresses.some(({ index }) => JSON.stringify(ctx.first(B, () => true, index)?.debug.selection) === JSON.stringify([droneId])),
        "the group key does not select the drone again");
    const tapPress = recallPresses.find(({ index }) => tapReceipts.has(index));
    step(13, tapPress, "the driver sent no group key double tap step");
    // The product reads its clock once for each frame: the two presses must reach it in different frames.
    assertPressesInDifferentFrames(retainedFrames(ctx.first(B, () => true, tapPress.index)?.debug), tapReceipts.get(tapPress.index).downs);
    const centredAt = (() => {
        const shot = ctx.first(B, () => true, tapPress.index);
        const drone = shot?.scene.units.find((unit) => unit.unit_id === droneId);
        const [vx, vy, vw, vh] = shot?.scene.viewport_logical_rect ?? [];
        return drone && Math.hypot(drone.ground_screen_center[0] - (vx + vw / 2), drone.ground_screen_center[1] - (vy + vh / 2)) <= 48 ? tapPress : null;
    })();
    step(13, centredAt, "the group key twice does not centre the camera on the drone");

    // Step 14: the menu; a faster scroll speed (storage audit); a new Stop key (the stored map).
    const menuShot = ctx.first(B, (shot) => (shot.debug.ui_snapshot ?? []).some((row) => row.kind === "settings_menu" && row.visible === true), centredAt.index);
    step(14, menuShot, "the menu never opens");
    step(14, presses(input, effectiveB(cleared).menu[0], B, centredAt.index, menuShot.input_call).length +
        presses(input, effectiveB(cleared).menu[1], B, centredAt.index, menuShot.input_call).length > 0, "no menu key opens the menu");
    const audit = JSON.parse(await view.serverFile("storage-audit.json"));
    const speeds = (audit.entries ?? []).filter((entry) => entry.window === B && entry.storage === "localStorage" && entry.operation === "set" &&
        entry.key === "turts.v1.settings").map((entry) => JSON.parse(entry.value)?.controls?.scroll_speed).filter(Number.isFinite);
    const initialSettings = (audit.initial_state ?? []).find((state) => state.window === B)?.localStorage?.["turts.v1.settings"];
    const initialSpeed = initialSettings ? JSON.parse(initialSettings)?.controls?.scroll_speed ?? 1 : 1;
    step(14, speeds.some((speed) => speed > initialSpeed), `the scroll speed is never raised above ${initialSpeed} (storage audit)`);
    const stopBefore = JSON.stringify(startActions.key_stop);
    const rebound = ctx.first(B, (shot) => {
        const stop = shot.debug.effective_bindings?.actions?.key_stop;
        return Array.isArray(stop) && JSON.stringify(stop) !== stopBefore;
    }, menuShot.input_call - 1);
    step(14, rebound, "Stop is never bound to another key");
    const newStop = rebound.debug.effective_bindings.actions.key_stop;
    const sheetAfter = ctx.first(B, (shot) => controlsRows(shot.debug).some((row) => row.text === rowText("key_stop", newStop, labelsOf(B))), rebound.input_call - 1);
    step(14, sheetAfter, "guide sheet 5 never shows the new Stop key after the rebind");

    // Step 15: a reload rejoins the same slot; group 1 and the new Stop key stay; group 1 works.
    const reload = input.map((entry, index) => ({ entry, index })).find(({ entry, index }) => entry.window === B && entry.call === "reload" && index > rebound.input_call);
    step(15, reload, "window B is not reloaded after the rebind");
    judgeLoad(reload.entry.load_observations ?? [], 15, "the reload of window B");
    const rejoined = ctx.first(B, (shot) => shot.debug.phase?.startsWith("Joined"), reload.index);
    step(15, rejoined && rejoined.debug.slot === 1 && typeof rejoined.debug.player_id_decimal === "string" &&
        rejoined.debug.player_id_decimal === rebound.debug.player_id_decimal, "the reload does not rejoin the same slot with the stored id");
    step(15, JSON.stringify(rejoined.debug.groups?.["1"]) === JSON.stringify([droneId]), "group 1 is lost after the reload");
    step(15, JSON.stringify(rejoined.debug.effective_bindings?.actions?.key_stop) === JSON.stringify(newStop), "the new Stop key is lost after the reload");
    const recallAfter = presses(input, rejoined.debug.effective_bindings.actions.group_recall_1[0], B, reload.index);
    const recalled = recallAfter.map(({ index }) => ctx.first(B, () => true, index)).find((shot) => JSON.stringify(shot?.debug.selection) === JSON.stringify([droneId]));
    step(15, recalled, "group 1 does not work after the reload");

    // Step 16: two queued points (Shift and right clicks) visited in order; the new Stop key stops it.
    const rights = clicks(input, B, "right", recalled.input_call);
    const queuedPair = rights.find(({ index }, position) => position + 1 < rights.length && (() => {
        const shot = ctx.first(B, () => true, rights[position + 1].index);
        const record = (shot?.debug.replicated_world.unit_orders ?? []).find((order) => order.unit === droneId);
        return record && (record.queued ?? []).length >= 1;
    })());
    step(16, queuedPair, "no Shift and two right clicks queue two points for the drone");
    const pairIndex = rights.indexOf(queuedPair);
    const scene16 = ctx.before(B, queuedPair.index).scene;
    const [p1x, p1y] = unprojectGround(scene16, { x: rights[pairIndex].entry.args[0], y: rights[pairIndex].entry.args[1] });
    const [p2x, p2y] = unprojectGround(scene16, { x: rights[pairIndex + 1].entry.args[0], y: rights[pairIndex + 1].entry.args[1] });
    const point1 = { x: p1x, y: p1y };
    const point2 = { x: p2x, y: p2y };
    const waypoints = ctx.first(B, (shot) => (shot.debug.markers_shown ?? []).filter((marker) => (marker.units ?? []).includes(droneId)).length >= 2,
        rights[pairIndex + 1].index);
    step(16, waypoints, "no waypoint lines for the two queued points");
    const stopPress = presses(input, newStop[0], B, rights[pairIndex + 1].index)[0];
    step(16, stopPress, "the new Stop key is not pressed after the queued order");
    const stopTick = ctx.before(B, stopPress.index + 1)?.debug.replicated_world.tick ?? 0;
    const startTick = ctx.before(B, rights[pairIndex + 1].index + 1).debug.replicated_world.tick;
    const nextOrder = clicks(input, B, "right", stopPress.index)[0];
    const endTick = nextOrder ? ctx.before(B, nextOrder.index)?.debug.replicated_world.tick ?? Infinity : Infinity;
    const track = unitTrack(tickLog, droneId);
    const reached1 = track.find((point) => point.tick >= startTick && near(point, point1) <= 0.6);
    step(16, reached1 && reached1.tick <= stopTick + 2, "the drone does not reach the first point before the Stop key");
    const settle = track.filter((point) => point.tick >= stopTick + 10 && point.tick < endTick);
    step(16, settle.length >= 2 && settle.every((point) => near(point, settle[0]) <= 0.01), "the new Stop key did not stop the drone");
    step(16, near(settle[0], point2) > 0.5, "the drone reached the second point: the new Stop key did not stop it");

    // Step 17: a right click inside the turret range; a marker at once; the drone rests there.
    const turretUnit = replicaUnit(foreign.debug, foreignTurret);
    const turretAt = { x: turretUnit.pos[0], y: turretUnit.pos[1] };
    const inRange = clicks(input, B, "right", stopPress.index).find(({ entry, index }) => {
        const scene = ctx.before(B, index)?.scene;
        if (!scene) return false;
        const [gx, gy] = unprojectGround(scene, { x: entry.args[0], y: entry.args[1] });
        return near({ x: gx, y: gy }, turretAt) <= TURRET_RANGE;
    });
    step(17, inRange, "no right click inside the turret range");
    const [tx, ty] = unprojectGround(ctx.before(B, inRange.index).scene, { x: inRange.entry.args[0], y: inRange.entry.args[1] });
    const markerShot = ctx.first(B, () => true, inRange.index);
    step(17, markerShot && (markerShot.debug.markers_shown ?? []).some((marker) => (marker.units ?? []).includes(droneId)), "no marker in the first screenshot after the right click");
    const orderTick = markerShot.debug.replicated_world.tick;
    const later = track.filter((point) => point.tick >= orderTick);
    const restAt = later.findIndex((point, index) => index + 5 < later.length && later.slice(index, index + 5).every((next) => near(next, point) <= 0.001));
    const rest = restAt >= 0 ? later[restAt] : null;
    step(17, rest && Math.hypot(rest.x - tx, rest.y - ty) <= 0.1, "the drone does not stop on the ordered point");
    step(17, ctx.all(B, (shot) => shot.debug.replicated_world.tick < rest.tick, inRange.index)
        .every((shot) => (shot.debug.markers_shown ?? []).some((marker) => (marker.units ?? []).includes(droneId))), "the marker goes before the drone rests");

    // Steps 18 and 19: damage with the alert and ping; destruction leaves selection and group 1.
    const lateShots = ctx.all(B, () => true, inRange.index);
    step(18, lateShots.some((shot) => {
        const health = (shot.debug.replicated_world.healths ?? []).find((record) => record.unit === droneId);
        return health && health.current < (replicaUnit(shot.debug, droneId)?.max_health ?? Infinity);
    }), "the health of the drone does not drop");
    step(18, lateShots.some((shot) => alertTexts(shot.debug).includes(TEXT.underAttack)), `no alert "${TEXT.underAttack}"`);
    step(18, lateShots.some((shot) => (shot.debug.alert_pings ?? []).some((ping) => ping.unit === droneId)), "the damage alert does not ping the minimap");
    const gone = lateShots.find((shot) => !replicaUnit(shot.debug, droneId));
    step(19, gone, "the drone is not destroyed");
    step(19, !(gone.debug.selection ?? []).includes(droneId), "the destroyed drone stays in the selection");
    step(19, !(gone.debug.groups?.["1"] ?? []).includes(droneId), "the destroyed drone stays in group 1");
    step(19, lateShots.filter((shot) => shot.input_call >= gone.input_call).some((shot) => alertTexts(shot.debug).includes(TEXT.destroyed)), `no alert "${TEXT.destroyed}"`);
}

// The run checks first, then the driver stop record (RUN123 L2).
async function judge(view, params) {
    const result = await judgeRun(view, params);
    await assertNoDriverStop(view, "PT1");
    return result;
}

module.exports = { judge, judgeTapSteps, actionLabel, assertControlsMatch, humanizeAction, layoutLabels, playerLabel, presses, rowText };
