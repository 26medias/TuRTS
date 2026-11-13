// turts-test-role: oracle
"use strict";

const assert = require("node:assert/strict");
const { decodePng } = require("./harness/png.js");
const { assertSwitchClickByLabel } = require("./harness/sound-switch-proof.js");

const SELECTION_PANEL = { x: 8, y: 8, width: 272, height: 180 };
const PANEL_FILL = [17, 24, 32];
const TOLERANCE = 28;
const REQUIRED_FILL_FRACTION = 0.18;

async function judge(view, params) {
    const input = view.inputLog();
    assert(saved(input, "hud2-initial.png"), "HUD2 needs the first-join screenshot");
    if (params.row === "HUD2-MUSTER" || params.package === "muster") return judgeMuster(view, input);

    return judgeForge(view, input, params);
}

async function judgeForge(view, input, params) {
    const target = JSON.parse(await view.serverFile("hud2-pixel-target.json")).target;
    assert(target && Number.isInteger(target.x) && Number.isInteger(target.y),
        "HUD2 own-unit click target must come from screenshot pixels");
    const closeGuide = input.findIndex((entry) => entry.call === "key" && entry.args?.[0] === "F1");
    assert(closeGuide >= 0, "HUD2 must close the first-join guide through physical F1 before selection");
    const clickIndex = input.findIndex((entry, index) => index > closeGuide && entry.call === "click" &&
        entry.args?.[0] === target.x && entry.args?.[1] === target.y && entry.args?.[2] === "left");
    assert(clickIndex >= 0, "HUD2 own unit must be selected with a physical left click on its pixel target");

    const panelShotIndex = screenshotIndexForSave(input, "hud2-selected-own.png");
    assert(panelShotIndex > clickIndex, "HUD2 panel pixels must be captured after own-unit selection");
    const checkpoints = JSON.parse(await view.serverFile("scene-checkpoints.json"));
    // The sound is off at the start and one click on the visible switch turns it on, before the
    // refusal that this row judges by its sound.
    const soundOn = assertSwitchClickByLabel(input, (name) => checkpointForSave(input, checkpoints, name), "hud2-sound", false);
    const checkpoint = checkpoints.find((entry) => entry.input_call === panelShotIndex && entry.window === 1);
    assert(checkpoint?.debug && checkpoint?.scene,
        "HUD2 selected screenshot needs its actual C84 scene/debug checkpoint");
    const selected = resolveSelectedOwn(checkpoint, target);
    assert(selected, "HUD2 panel baseline requires an actual owned selected unit");

    const encoded = await view.serverFile("hud2-selected-own.png.b64");
    const image = decodePng(Buffer.from(encoded, "base64"));
    assert(SELECTION_PANEL.x + SELECTION_PANEL.width <= image.width &&
        SELECTION_PANEL.y + SELECTION_PANEL.height <= image.height,
    "HUD2 selection panel test rectangle must fit the captured viewport");

    const matching = countFillPixels(image, SELECTION_PANEL, PANEL_FILL, TOLERANCE);
    const fraction = matching / (SELECTION_PANEL.width * SELECTION_PANEL.height);
    assert(fraction >= REQUIRED_FILL_FRACTION,
        "no selection panel: expected the Forge selection panel fill after selecting an owned unit");

    const candidateEvidence = JSON.parse(await view.serverFile("hud2-selection-candidates.json"));
    const candidateCheckpoints = candidateEvidence.attemptedCandidates.map((candidate) => ({
        ...candidate,
        checkpoint: checkpointForSave(input, checkpoints, `${candidate.label}.png`),
    }));
    const identified = new Map();
    for (const candidate of candidateCheckpoints) {
        const selection = candidate.checkpoint.debug.selection;
        assert(Array.isArray(selection) && selection.length === 1,
            `HUD2 physical palette click ${candidate.label} must select one actual unit`);
        const replica = candidate.checkpoint.debug.replicated_world?.units?.find(
            (unit) => unit.unit_id === selection[0]);
        assert(replica && replica.owner_slot === candidate.slot,
            `HUD2 ${candidate.label} actual selection must belong to its screenshot-derived slot colour`);
        const rendered = candidate.checkpoint.scene.units?.find((unit) => unit.unit_id === replica.unit_id);
        assert(rendered && Array.isArray(rendered.ground_screen_center) &&
            Math.hypot(rendered.ground_screen_center[0] - candidate.target.x,
                rendered.ground_screen_center[1] - candidate.target.y) <= 40,
        `HUD2 ${candidate.label} actual selected unit must match its pixel target`);
        const row = (candidate.checkpoint.debug.ui_snapshot || []).find((item) =>
            item.id.startsWith("selection_units/") && item.kind === "button" &&
            item.visible === true && item.text);
        assert(row, `HUD2 ${candidate.label} must render an actual selected-unit row`);
        const nameWidgetId = replica.owner_slot === candidate.checkpoint.debug.slot
            ? "info_name" : "enemy_name";
        const primaryName = (candidate.checkpoint.debug.ui_snapshot || []).find((item) =>
            item.id === nameWidgetId && item.visible === true);
        assert(primaryName?.visible === true && typeof primaryName.text === "string",
            `HUD2 ${candidate.label} must render the actual selected unit name`);
        const expectedName = primaryName.text.split(" · Slot ")[0];
        assert(row.text.includes(expectedName) && row.text.includes("×1"),
            `HUD2 ${candidate.label} list row must name the actual unit and show its selected count`);
        const publicHealth = (candidate.checkpoint.debug.replicated_world.healths || [])
            .find((health) => health.unit === replica.unit_id);
        if (publicHealth) {
            assert(row.text.includes(`— ${publicHealth.current} HP`),
                `HUD2 ${candidate.label} selected-unit row must show its actual public health`);
            await assertTextPixels(view, candidate.checkpoint, row, "selected-unit health row");
        }
        if (replica.content_id === "forge:lab" || replica.content_id === "forge:turret" ||
            replica.content_id === "forge:drone") identified.set(replica.content_id, candidate.checkpoint);
    }
    assert(identified.has("forge:lab"), "HUD2 must physically select the actual Forge lab");
    assert(identified.has("forge:turret"), "HUD2 must physically select the actual own Forge turret");
    assert(identified.has("forge:drone"), "HUD2 must physically select an actual foreign Forge drone");

    const lab = identified.get("forge:lab");
    const labReplica = lab.debug.replicated_world.units.find((unit) => unit.content_id === "forge:lab");
    const labInventory = lab.debug.replicated_world.economy.inventories.find((entry) =>
        entry.unit === labReplica.unit_id && entry.port === "main");
    assert(labInventory, "HUD2 lab inventory proof requires its actual known main inventory");
    const inventoryText = actualWidget(lab.debug, "info_inventory", "lab inventory");
    const expectedInventoryLines = labInventory.items.length === 0
        ? ["Main: Main inventory 0 available (capacity 10)"]
        : labInventory.items.map((item) => {
            assert.equal(item.resource, "forge:shell", "HUD2 Forge lab main inventory accepts only Shell");
            const available = Math.max(0, item.on_hand - item.reserved);
            const reserved = item.reserved > 0 ? `, ${item.reserved} reserved` : "";
            return `Main: Shell ${available} available (${item.on_hand} on hand${reserved}) (capacity 10)`;
        });
    assert.equal(inventoryText.text, `Inventory: ${expectedInventoryLines.join("\n")}`,
        "HUD2 lab inventory text must match the same checkpoint's actual available/reserved amounts");
    await assertTextPixels(view, lab, inventoryText, "lab inventory");
    const research = (lab.debug.ui_snapshot || []).find((row) =>
        row.id.startsWith("research_entries/") && row.kind === "button");
    const shell = labInventory.items.find((item) => item.resource === "forge:shell");
    const availableShells = shell ? Math.max(0, shell.on_hand - shell.reserved) : 0;
    assert(availableShells < 10,
        "HUD2 shortage checkpoint must precede the actual lab becoming affordable");
    assert(research && research.visible === true && research.enabled === false &&
        research.text.includes(`Hardened Shells — Needs 10 Shell (${availableShells}/10)`),
    "HUD2 lab must render actual reserved-aware disabled research shortage text");
    await assertTextPixels(view, lab, research, "disabled research reason");

    const turret = identified.get("forge:turret");
    const selectedTurretId = turret.debug.selection?.[0];
    const turretUnit = turret.debug.replicated_world.units.find((unit) =>
        unit.unit_id === selectedTurretId && unit.content_id === "forge:turret");
    assert(turretUnit && turretUnit.owner_slot === turret.debug.slot,
        "HUD2 turret stats must resolve the actual selected owned Forge turret");
    const weapon = turret.debug.replicated_world.weapons.find((item) => item.unit === turretUnit.unit_id);
    assert(weapon && weapon.damage?.Ok !== undefined,
        "HUD2 turret stats must come from the actual weapon replica");
    const damageText = actualWidget(turret.debug, "info_damage", "turret damage");
    const rangeText = actualWidget(turret.debug, "info_range", "turret range");
    const ammoText = actualWidget(turret.debug, "info_ammo", "turret ammo");
    assert(damageText.text === `Damage: ${weapon.damage.Ok}`,
        "HUD2 turret damage text must match the actual weapon record");
    assert(rangeText.text === "Range: 8.0",
        "HUD2 turret range must match the authored weapon definition");
    const turretInventory = turret.debug.replicated_world.economy.inventories.find((entry) =>
        entry.unit === turretUnit.unit_id && entry.port === "main");
    assert(turretUnit.owner_slot === turret.debug.slot && turretInventory &&
        Array.isArray(turretInventory.items),
    "HUD2 turret ammo proof requires the selected owner's known main inventory");
    const turretShells = turretInventory.items.find((item) => item.resource === "forge:shell");
    if (turretShells) {
        assert(Number.isInteger(turretShells.on_hand) && turretShells.on_hand >= 0 &&
            Number.isInteger(turretShells.reserved) && turretShells.reserved >= 0,
        "HUD2 known turret Shell stock must have nonnegative integer on-hand and reserved values");
    }
    const availableTurretShells = turretShells
        ? Math.max(0, turretShells.on_hand - turretShells.reserved) : 0;
    assert(availableTurretShells === 0 && ammoText.text === "Shells: 0",
        "HUD2 empty Forge turret must render zero ammo from its actual known inventory");
    await assertTextPixels(view, turret, damageText, "turret damage");
    await assertTextPixels(view, turret, rangeText, "turret range");
    await assertTextPixels(view, turret, ammoText, "turret ammo");

    const drone = identified.get("forge:drone");
    const selectedDroneId = drone.debug.selection?.[0];
    const droneUnit = drone.debug.replicated_world.units.find((unit) =>
        unit.unit_id === selectedDroneId && unit.content_id === "forge:drone");
    assert(droneUnit && droneUnit.owner_slot !== drone.debug.slot &&
        Number.isFinite(droneUnit.max_health),
    "HUD2 must identify the selected foreign drone and its finite public maximum health");
    const droneHealth = drone.debug.replicated_world.healths.find((health) => health.unit === droneUnit.unit_id);
    assert(Number.isFinite(droneHealth?.current),
        "HUD2 selected foreign drone must expose finite current public health");
    const enemyHealth = actualWidget(drone.debug, "enemy_health", "foreign drone health");
    const expectedDroneHealth = `Health: ${droneHealth.current}/${droneUnit.max_health}`;
    assert(enemyHealth.text === expectedDroneHealth,
        "HUD2 foreign selection must render actual current/max health");
    await assertTextPixels(view, drone, enemyHealth, "foreign drone health");
    assert(!(drone.debug.ui_snapshot || []).some((row) => row.id === "command_card" && row.visible),
        "HUD2 foreign selection must keep the owner command card hidden");

    const turretTarget = candidateCheckpoints.find((candidate) =>
        candidate.checkpoint.debug.selection?.length === 1 &&
        candidate.checkpoint.debug.replicated_world?.units?.some((unit) =>
            unit.unit_id === candidate.checkpoint.debug.selection[0] && unit.content_id === "forge:turret"));
    assert(turretTarget, "HUD2 cannot-move response must start from an actual selected Forge turret");
    const noMoveTarget = candidateEvidence.clearTarget;
    const refusalClick = input.findIndex((entry) => entry.call === "click" &&
        entry.args?.[0] === noMoveTarget.x && entry.args?.[1] === noMoveTarget.y &&
        entry.args?.[2] === "right" && entry.event_seq?.length >= 1);
    assert(refusalClick > turretTarget.checkpoint.screenshotIndex,
        "HUD2 turret refusal must follow its actual selected screenshot and physical right click");
    assert(refusalClick > soundOn.clickIndex, "HUD2 turns the sound on before the refusal that it judges by sound");
    const refusal = checkpointForSave(input, checkpoints, "hud2-turret-cannot-move.png");
    const refusalInput = input[refusalClick];
    const refusalPointerUp = refusalInput.actual_events?.find((event) =>
        event.type === "pointerup" && event.canvas_target && Number.isInteger(event.seq));
    assert(refusalPointerUp,
        "HUD2 turret refusal sound must map to the physical canvas pointer-up sequence");
    assert.equal(input[refusalClick - 1]?.call, "screenshot",
        "HUD2 needs the immediate pre-click screenshot for the refusal sound onset control");
    const beforeRefusal = checkpoints.find((entry) =>
        entry.input_call === refusalClick - 1 && entry.window === 1);
    assert(beforeRefusal?.debug &&
        beforeRefusal.debug.selection?.[0] === turretTarget.checkpoint.debug.selection?.[0],
    "HUD2 pre-click control must retain the same actual selected Forge turret");
    const refusalFrames = (refusal.debug.frames?.retained_input || []).filter((frame) =>
        frame.applied_seq?.includes(refusalPointerUp.seq));
    assert.equal(refusalFrames.length, 1,
        "HUD2 physical refusal pointer-up must map to exactly one actual presented frame");
    const refusalFrame = refusalFrames[0];
    assert.deepEqual(refusalFrame.selection, turretTarget.checkpoint.debug.selection,
        "HUD2 actual refusal frame must retain the physically selected turret");
    assert.deepEqual(refusalFrame.emitted_orders || [], [],
        "HUD2 actual turret refusal frame must not emit a world order");
    assert((refusalFrame.feedback?.notices || []).some((notice) =>
        (notice.text || notice.message || "").includes("Shell Turret: Cannot move")),
    "HUD2 actual physical refusal frame must contain the named Shell Turret refusal");
    const refusalNotices = (refusal.debug.notices || []).map((notice) =>
        notice.text || notice.message || "").join("\n");
    const visibleNotice = (refusal.debug.ui_snapshot || []).find((row) => row.id === "last_notice");
    assert(refusalNotices.includes("Shell Turret: Cannot move") ||
        visibleNotice?.text?.includes("Shell Turret: Cannot move"),
    "HUD2 actual turret right click must show the named Shell Turret: Cannot move refusal");
    const configuredError = (refusalFrame.feedback?.audio_started || []).find((sound) =>
        sound.category === "effects" && sound.id === "forge:forge_effect_error");
    assert(configuredError && turretTarget.checkpoint.debug.replicated_world?.units?.some((unit) =>
        unit.unit_id === refusalFrame.selection?.[0] && unit.content_id === "forge:turret"),
    "HUD2 exact refusal frame must configure the authored Forge turret UnitPresentation error response");
    assert.equal(refusal.debug.audio?.sound_on, true, "HUD2 the sound switch is on at the refusal");
    const sinkObservations = refusal.debug.audio?.sink_observations || [];
    const errorSound = sinkObservations.find((sound) =>
        sound.sound_id === configuredError.id && sound.category === "effects" &&
        sound.request_frame_index === refusalFrame.frame_index &&
        sound.observed_core_frame_index >= refusalFrame.frame_index &&
        Number.isFinite(sound.gain) && sound.gain > 0 &&
        Number.isInteger(sound.play_id) && Number.isInteger(sound.player_entity));
    assert(errorSound,
        "HUD2 exact refusal frame must reach a real positive-gain UnitPresentation error OGG sink");
    const priorSinks = beforeRefusal.debug.audio?.sink_observations || [];
    assert(!priorSinks.some((sound) => sound.play_id === errorSound.play_id ||
        sound.player_entity === errorSound.player_entity),
    "HUD2 refusal OGG sink must be a new actual playback relative to the immediate pre-click control");

    const researchEnabled = checkpointForSave(input, checkpoints, "hud2-research-enabled.png");
    assert.deepEqual(researchEnabled.debug.selection, [labReplica.unit_id],
        "HUD2 actual research-enabled screenshot must retain the selected Forge lab");
    const enabledLabInventory = researchEnabled.debug.replicated_world?.economy?.inventories?.find((entry) =>
        entry.unit === labReplica.unit_id && entry.port === "main");
    assert(enabledLabInventory,
        "HUD2 actual enabled screenshot needs the same lab's public main inventory");
    const enabledShells = enabledLabInventory.items.find((item) => item.resource === "forge:shell");
    const availableEnabledShells = enabledShells
        ? Math.max(0, enabledShells.on_hand - enabledShells.reserved) : 0;
    assert(availableEnabledShells >= 10,
        "HUD2 research-enabled screenshot must show ten actually available Shells");
    const enabledResearchRow = (researchEnabled.debug.ui_snapshot || []).find((row) =>
        row.id.startsWith("research_entries/") && row.widget_action?.content_id === "forge:hardened_shells");
    assert(enabledResearchRow?.visible === true && enabledResearchRow.enabled === true &&
        enabledResearchRow.text.includes("Hardened Shells"),
    "HUD2 same-checkpoint research widget must be actually visible and enabled");
    await assertTextPixels(view, researchEnabled, enabledResearchRow, "enabled research option");

    const enqueued = checkpointForSave(input, checkpoints, "hud2-research-enqueued.png");
    assert(researchEnabled.screenshotIndex < enqueued.screenshotIndex,
        "HUD2 queue evidence must follow the actual enabled research screenshot");
    const queueCancel = checkpointForSave(input, checkpoints, "hud2-queue-cancelled.png");
    const labQueue = enqueued.debug.replicated_world.economy.queues.find((entry) =>
        entry.producer === labReplica.unit_id);
    assert(labQueue?.jobs?.length === 1 && labQueue.jobs[0].recipe === "forge:hardened_shells",
        "HUD2 physical research click must create the real Forge queue job");
    const researchJob = labQueue.jobs[0];
    const queueRow = (enqueued.debug.ui_snapshot || []).find((row) =>
        row.id.includes("job_seconds") && row.visible === true && /\d+(\.\d+)?s remaining/.test(row.text || ""));
    assert(queueRow, "HUD2 real research queue must render seconds remaining");
    await assertTextPixels(view, enqueued, queueRow, "queue seconds");
    const jobName = (enqueued.debug.ui_snapshot || []).find((row) =>
        row.id === "job_name" && row.visible === true);
    assert(jobName?.text === "Hardened Shells",
        "HUD2 visible queue name must identify the same actual research job");
    const progressBar = (enqueued.debug.ui_snapshot || []).find((row) =>
        row.id === "job_progress" && row.visible === true && pointInside(row.rect, row.rect?.[0], row.rect?.[1]));
    assert(progressBar,
        "HUD2 same-job queue screenshot must expose its actual visible progress-bar widget rect");
    const cancelButton = (enqueued.debug.ui_snapshot || []).find((row) =>
        row.id === "job_cancel" && row.kind === "button" && row.visible === true);
    assert(cancelButton?.widget_action?.kind === "cancel_production" &&
        cancelButton.widget_action.job === researchJob.id &&
        cancelButton.widget_action.producer === labReplica.unit_id,
    "HUD2 visible cancel button must address the same actual queued research job");
    const progress = await view.serverFile("hud2-research-enqueued.png.b64");
    const [progressX, progressY, progressWidth, progressHeight] = progressBar.rect;
    assertProgressPixels(decodePng(Buffer.from(progress, "base64")),
        { x: progressX, y: progressY, width: progressWidth, height: progressHeight });
    const cancelClick = input.findIndex((entry) => entry.call === "click" &&
        entry.args?.[0] === 229 && entry.args?.[1] === 658 && entry.args?.[2] === "left");
    assert(cancelClick > enqueued.screenshotIndex && cancelClick < queueCancel.screenshotIndex,
        "HUD2 queue cancel must physically click after enqueue and before its cancelled screenshot");
    assert(cancelButton && pointInside(cancelButton.rect, 229, 658),
        "HUD2 physical cancel point must be inside the actual computed queue button");
    const cancelledQueue = queueCancel.debug.replicated_world.economy.queues.find((entry) =>
        entry.producer === labReplica.unit_id);
    assert(cancelledQueue?.jobs?.length === 0,
        "HUD2 real CancelProduction button must remove the research job");

    await assertOrderLabelsAndTooltips(view, input, checkpoints);

    const initial = checkpointForSave(input, checkpoints, "hud2-initial.png");
    const controlsBottom = checkpointForSave(input, checkpoints, "hud2-controls-bottom.png");
    assertGuideVisible(controlsBottom.debug, true, "Controls scroll");
    assert(input.some((entry, index) => index < controlsBottom.screenshotIndex &&
        entry.call === "wheel" && entry.args?.[0] > 0),
    "HUD2 Controls tail must be reached with actual physical wheel input");
    assert.deepEqual(controlsBottom.scene.clip_from_world, initial.scene.clip_from_world,
        "HUD2 wheel inside the guide must not zoom the world camera");
    assert.deepEqual(controlsBottom.debug.replicated_world?.unit_orders || [],
        initial.debug.replicated_world?.unit_orders || [],
        "HUD2 Controls scrolling must not issue world orders");
    const orderLabels = await orderLabelsByHotkey(view, params, controlsBottom.debug);
    const tail = controlsTailRow(controlsBottom.debug, orderLabels);
    await assertTextPixels(view, controlsBottom, tail, "Controls tail row");
    const onScreen = controlsRowsOnScreen(controlsBottom.debug);
    assert(onScreen.length >= 5 && onScreen.some((row) => row.index === tail.index),
        "HUD2 Controls must show at least five whole rows, with the tail row, after scrolling");
    for (const row of onScreen) await assertTextPixels(view, controlsBottom, row, `Controls row ${row.index}`);
    const f1Closed = checkpointForSave(input, checkpoints, "hud2-f1-closed.png");
    const reopened = checkpointForSave(input, checkpoints, "hud2-guide-reopened.png");
    assertGuideVisible(initial.debug, true, "first join");
    assertGuideVisible(f1Closed.debug, false, "F1 close");
    assertGuideVisible(reopened.debug, true, "F1 reopen");
    assert((initial.scene.units || []).length > 0,
        "HUD2 first-join camera must show factory units behind the guide");
    assert((initial.debug.replicated_world?.units || []).some((unit) => unit.owner_slot === 0 &&
        (initial.scene.units || []).some((rendered) => rendered.unit_id === unit.unit_id)),
    "HUD2 first-join camera must render an owned factory unit");

    const closeEvidence = JSON.parse(await view.serverFile("hud2-close-button-target.json"));
    assert(closeEvidence.target && Number.isInteger(closeEvidence.target.x) &&
        Number.isInteger(closeEvidence.target.y),
    "HUD2 guide close button must be found by its authored orange pixels before clicking");
    const close = closeEvidence.target;
    const closeClickIndex = input.findIndex((entry) => entry.call === "click" &&
        entry.args?.[0] === close.x && entry.args?.[1] === close.y && entry.args?.[2] === "left");
    assert(closeClickIndex >= 0,
        "HUD2 close button must receive physical pointer IO at its screenshot-derived target");
    const closeWidget = (reopened.debug.ui_snapshot || []).find((row) => row.id === "guide_close");
    assert(closeWidget?.kind === "button" && closeWidget.visible === true &&
        closeWidget.widget_action?.kind === "input" && closeWidget.widget_action?.name === "toggle_guide",
    "HUD2 guide close hitbox must be an actual visible toggle_guide button");
    const [left, top, width, height] = closeWidget.rect || [];
    assert([left, top, width, height].every(Number.isFinite) && width > 0 && height > 0 &&
        close.x >= left && close.x <= left + width && close.y >= top && close.y <= top + height,
    "HUD2 close-button pixel click must land inside its actual computed widget hitbox");
    const closedByButton = checkpointForSave(input, checkpoints, "hud2-close-button-closed.png");
    assert(closedByButton.screenshotIndex > closeClickIndex,
        "HUD2 close-button state must be captured after the real pointer click");
    assertGuideVisible(closedByButton.debug, false, "close-button close");
    assert.deepEqual(closedByButton.debug.replicated_world?.unit_orders || [],
        initial.debug.replicated_world?.unit_orders || [],
        "HUD2 F1 and close-button guide actions must not create a world order");
}

async function judgeMuster(view, input) {
    assert(input.some((entry) => entry.call === "key" && entry.args?.[0] === "F1"),
        "HUD2-MUSTER must close the first-join guide through physical F1");
    const screenshotIndex = screenshotIndexForSave(input, "hud2-muster-resource.png");
    assert(screenshotIndex >= 0, "HUD2-MUSTER needs a post-guide resource screenshot");
    const checkpoints = JSON.parse(await view.serverFile("scene-checkpoints.json"));
    const checkpoint = checkpoints.find((entry) => entry.input_call === screenshotIndex && entry.window === 1);
    assert(checkpoint?.debug, "HUD2-MUSTER resource screenshot needs an actual UI checkpoint");
    const ore = (checkpoint.debug.ui_snapshot || []).find((row) => row.id === "ore");
    assert(ore?.visible === true && /^Ore \d+$/.test(ore.text || ""),
        "HUD2-MUSTER must render the actual ore balance as player-facing text");

    const encoded = await view.serverFile("hud2-muster-resource.png.b64");
    const image = decodePng(Buffer.from(encoded, "base64"));
    const rect = { x: (image.width - 160) / 2, y: image.height - 32, width: 160, height: 28 };
    let foreground = 0;
    for (let y = rect.y; y < rect.y + rect.height; y += 1) {
        for (let x = rect.x; x < rect.x + rect.width; x += 1) {
            const offset = (y * image.width + x) * 4;
            if (image.rgba[offset] > 185 && image.rgba[offset + 1] > 185 && image.rgba[offset + 2] > 185) foreground += 1;
        }
    }
    assert(foreground >= 12,
        "HUD2-MUSTER must show visible ore balance glyph pixels in the authored bottom-center line");
}

function actualWidget(debug, id, label) {
    const row = (debug.ui_snapshot || []).find((item) => item.id === id);
    assert(row && row.visible === true && typeof row.text === "string" && row.text.length > 0,
        `HUD2 ${label} must be an actual visible text widget`);
    return row;
}

async function assertTextPixels(view, checkpoint, row, label) {
    const [left, top, width, height] = row.rect || [];
    assert([left, top, width, height].every(Number.isFinite) && width > 0 && height > 0,
        `HUD2 ${label} requires actual computed UI geometry`);
    assert(Number.isInteger(checkpoint.screenshotIndex) && checkpoint.saveFile,
        `HUD2 ${label} screenshot checkpoint is missing`);
    const encoded = await view.serverFile(`${checkpoint.saveFile}.b64`);
    const image = decodePng(Buffer.from(encoded, "base64"));
    const x0 = Math.max(0, Math.floor(left));
    const y0 = Math.max(0, Math.floor(top));
    const x1 = Math.min(image.width, Math.ceil(left + width));
    const y1 = Math.min(image.height, Math.ceil(top + height));
    let glyphPixels = 0;
    for (let y = y0; y < y1; y += 1) {
        for (let x = x0; x < x1; x += 1) {
            const offset = (y * image.width + x) * 4;
            if (image.rgba[offset + 3] > 0 && Math.max(image.rgba[offset], image.rgba[offset + 1],
                image.rgba[offset + 2]) > 150) glyphPixels += 1;
        }
    }
    assert(glyphPixels >= 3, `HUD2 ${label} must have visible rendered glyph pixels inside its actual widget rect`);
}

function assertProgressPixels(image, rect) {
    let blue = 0;
    for (let y = rect.y; y < rect.y + rect.height; y += 1) {
        for (let x = rect.x; x < rect.x + rect.width; x += 1) {
            const offset = (y * image.width + x) * 4;
            if (image.rgba[offset] > 60 && image.rgba[offset] < 120 &&
                image.rgba[offset + 1] > 150 && image.rgba[offset + 1] < 220 &&
                image.rgba[offset + 2] > 190) blue += 1;
        }
    }
    assert(blue > 0, "HUD2 queue must show actual progress-bar fill pixels");
}

function pointInside(rect, x, y) {
    if (!Array.isArray(rect) || rect.length !== 4 || !rect.every(Number.isFinite)) return false;
    const [left, top, width, height] = rect;
    return width > 0 && height > 0 && x >= left && x <= left + width && y >= top && y <= top + height;
}

async function assertOrderLabelsAndTooltips(view, input, checkpoints) {
    const evidence = JSON.parse(await view.serverFile("hud2-order-tooltip-targets.json"));
    const observed = new Set();
    const expectedButtons = new Map();
    for (const candidate of checkpoints.filter((item) => item.debug?.selection?.length === 1)) {
        for (const button of (candidate.debug.ui_snapshot || []).filter((row) =>
            row.id.startsWith("command_rows/") && row.kind === "button" && row.visible === true &&
            row.widget_action?.kind === "order")) {
            const bindings = candidate.debug.effective_bindings?.orders?.[button.widget_action.id] || [];
            assert(bindings.length > 0, `HUD2 ${button.widget_action.id} must have an actual effective key binding`);
            const labels = bindings.map(bindingLabel);
            assert(labels.some((key) => button.text?.includes(`[${key}]`)),
                `HUD2 ${button.widget_action.id} visible label must show its actual key`);
            expectedButtons.set(button.widget_action.id, labels);
        }
    }
    for (const target of evidence) {
        const checkpoint = checkpointForSave(input, checkpoints, `${target.label}.png`);
        const snapshot = checkpoint.debug.ui_snapshot || [];
        const button = snapshot.find((row) => row.id.startsWith("command_rows/") &&
            row.kind === "button" && row.visible === true && row.widget_action?.kind === "order" &&
            pointInside(row.rect, target.target.x, target.target.y));
        assert(button, `HUD2 ${target.label} physical hover must land on an actual order button`);
        const bindings = checkpoint.debug.effective_bindings?.orders?.[button.widget_action.id] || [];
        assert(bindings.length > 0, `HUD2 ${button.widget_action.id} must have an actual effective key binding`);
        const labels = bindings.map(bindingLabel);
        assert(labels.some((key) => button.text?.includes(`[${key}]`)),
            `HUD2 ${button.widget_action.id} button label must show its actual key`);
        let moveIndex = -1;
        for (let index = 0; index < checkpoint.screenshotIndex; index += 1) {
            const entry = input[index];
            if (entry.call === "move" && entry.args?.[0] === target.target.x &&
                entry.args?.[1] === target.target.y && entry.event_seq?.length >= 1) moveIndex = index;
        }
        assert(moveIndex >= 0, `HUD2 ${button.widget_action.id} must receive physical pointer hover`);
        const delay = input.find((entry, index) => index > moveIndex && index < checkpoint.screenshotIndex &&
            entry.call === "wait" && entry.args?.[0] === 600);
        assert(delay, `HUD2 ${button.widget_action.id} tooltip screenshot must follow a physical 600 ms hover`);
        const tooltip = snapshot.find((row) => row.id === `${button.id}::tooltip` &&
            row.kind === "widget" && row.visible === true && typeof row.text === "string");
        assert(tooltip, `HUD2 ${button.widget_action.id} must publish its actual visible rendered tooltip`);
        assert(labels.some((key) => tooltip.text.includes(`[${key}]`)),
            `HUD2 ${button.widget_action.id} rendered tooltip must name its actual effective key`);
        await assertTextPixels(view, checkpoint, tooltip, `${button.widget_action.id} tooltip`);
        const encoded = await view.serverFile(`${target.label}.png.b64`);
        assertTooltipBubblePixels(decodePng(Buffer.from(encoded, "base64")), tooltip.rect,
            `HUD2 ${button.widget_action.id}`);
        observed.add(button.widget_action.id);
    }
    assert(expectedButtons.size > 0, "HUD2 owned unit selections must expose actual Forge command buttons");
    assert.deepEqual([...observed].sort(), [...expectedButtons.keys()].sort(),
        "HUD2 must physically hover every visible Forge order button and verify its actual rendered key tooltip");
}

function assertTooltipBubblePixels(image, rect, label) {
    assert(Array.isArray(rect) && rect.length === 4 && rect.every(Number.isFinite),
        `${label} tooltip requires its actual computed text rectangle`);
    const [left, top, width, height] = rect;
    const x0 = Math.max(0, Math.floor(left - 8));
    const y0 = Math.max(0, Math.floor(top - 4));
    const x1 = Math.min(image.width, Math.ceil(left + width + 8));
    const y1 = Math.min(image.height, Math.ceil(top + height + 4));
    let background = 0;
    for (let y = y0; y < y1; y += 1) {
        for (let x = x0; x < x1; x += 1) {
            const offset = (y * image.width + x) * 4;
            if (Math.hypot(image.rgba[offset] - 17, image.rgba[offset + 1] - 24,
                image.rgba[offset + 2] - 32) <= 35) background += 1;
        }
    }
    assert(background >= 12, `${label} hover tooltip must show its actual dark bubble pixels`);
}

// The key text of one binding as the product writes it (crates/turts_client/src/client_shell.rs:90-141):
// modifiers in the order Ctrl, Shift, Alt; the key names Esc, Up, Down, Left, Right, Space.
function bindingLabel(binding) {
    const modifiers = (mods) => ["Ctrl", "Shift", "Alt"].filter((mod) => (mods || []).includes(mod)).map((mod) => `${mod}+`).join("");
    if (binding?.Key) {
        const code = binding.Key.code || "";
        const names = { Escape: "Esc", Space: "Space", ArrowUp: "Up", ArrowDown: "Down", ArrowLeft: "Left", ArrowRight: "Right" };
        const key = /^Key.$/.test(code) ? code.slice(3) : /^Digit.$/.test(code) ? code.slice(5) : names[code] ?? code;
        return `${modifiers(binding.Key.mods)}${key}`;
    }
    if (binding?.Wheel) return `Wheel ${String(binding.Wheel).toLowerCase()}`;
    if (binding?.Pointer) return `${modifiers(binding.Pointer.mods)}${binding.Pointer.button} click`;
    return "";
}

function saved(input, filename) {
    return input.some((entry) => entry.call === "save" && entry.args?.[0] === filename);
}

function screenshotIndexForSave(input, filename) {
    const saveIndex = input.findIndex((entry) => entry.call === "save" && entry.args?.[0] === filename);
    if (saveIndex < 0) return -1;
    for (let index = saveIndex - 1; index >= 0; index -= 1) {
        if (input[index].call === "screenshot") return index;
    }
    return -1;
}

function checkpointForSave(input, checkpoints, filename) {
    const screenshotIndex = screenshotIndexForSave(input, filename);
    assert(screenshotIndex >= 0, `HUD2 is missing screenshot ${filename}`);
    const checkpoint = checkpoints.find((entry) => entry.input_call === screenshotIndex && entry.window === 1);
    assert(checkpoint?.debug && checkpoint?.scene, `HUD2 ${filename} requires an actual C84 checkpoint`);
    return { ...checkpoint, screenshotIndex, saveFile: filename };
}

// The Controls list (guide page 5). Facts that the oracle requires, for every row it can read:
//   1. The list holds one row for each effective action, with the indexes 0..n-1 once each.
//   2. The sorted key texts of all rows equal the sorted key strings of the effective actions.
//   3. Each row is the exact text "label: keys" of one action, and each action gives one row, so a
//      row cannot carry the keys of another action or a label of another action.
//   4. The label of an action is the name of the first order that has the action as its hotkey,
//      else the action name made readable (crates/turts_client/src/widgets/sources.rs:1054-1118).
// The last row (the highest index) must also be visible and inside the Controls viewport, and its
// glyph pixels are read by the caller. The row order is not required (RUN130; RUN132 changes both
// the order and the labels).
const CONTROLS_VIEWPORT = { left: 490, top: 94, right: 1191, bottom: 679 };
const ENGINE_LABELS = {
    toggle_guide: "Toggle guide", centre_selection: "Centre selection", camera_zoom: "Zoom camera",
    camera_rotate: "Rotate camera", selection_add: "Add to selection", selection_remove: "Remove from selection",
};

function humanizeAction(action) {
    const prefixes = [["group_assign_", "Assign group "], ["group_recall_", "Select group "],
        ["group_add_", "Add to group "], ["camera_pan_", "Pan camera "]];
    const prefix = prefixes.find(([start]) => action.startsWith(start));
    const known = prefix ? `${prefix[1]}${action.slice(prefix[0].length)}` : ENGINE_LABELS[action] ?? action;
    return known.replace(/[_-]/g, " ").split(/\s+/).filter(Boolean)
        .map((word, index) => (index === 0 ? word[0].toUpperCase() + word.slice(1) : word)).join(" ");
}

// The order names of the served package, by hotkey action: { key_stop: "Stop selected units" }.
// They come from the package files of the run (pack/<package>/orders/<id>.ron), not from the product.
async function orderLabelsByHotkey(view, params, debug) {
    const labels = {};
    for (const orderId of Object.keys(debug?.effective_bindings?.orders || {}).sort()) {
        const local = orderId.split(":").pop();
        let text;
        try { text = await view.serverFile(`pack/${params.package}/orders/${local}.ron`); } catch (error) {
            assert.fail(`HUD2 cannot read the package file of order ${orderId}: ${error.message}`);
        }
        const name = /\bname:\s*"([^"]*)"/.exec(text)?.[1];
        const hotkey = /\bhotkey:\s*"([^"]*)"/.exec(text)?.[1];
        assert(name && hotkey, `HUD2 package file of order ${orderId} must give a name and a hotkey`);
        labels[hotkey] ??= name;
    }
    return labels;
}

function expectedControlsRows(actions, orderLabels) {
    return Object.entries(actions).map(([action, bindings]) => ({
        action, label: orderLabels[action] ?? humanizeAction(action), keys: bindings.map(bindingLabel).join(", "),
    }));
}

function controlsRows(debug) {
    const actions = debug?.effective_bindings?.actions;
    assert(actions && Object.keys(actions).length > 48,
        "HUD2 Controls must cover the actual effective action set beyond the old 48-entry cap");
    const rows = (debug.ui_snapshot || []).map((item) => ({ item, index: /^guide\.4\.bindings\/[^/]*\[index:(\d+)\]$/.exec(item.id ?? "") }))
        .filter((row) => row.index).map((row) => ({ ...row.item, index: Number(row.index[1]) }));
    assert.equal(rows.length, Object.keys(actions).length, "HUD2 Controls must show one row for each effective action");
    rows.sort((left, right) => left.index - right.index);
    assert.deepEqual(rows.map((row) => row.index), rows.map((_, index) => index),
        "HUD2 Controls row indexes must run from 0 once each");
    return rows;
}

function assertControlsTexts(rows, actions, orderLabels) {
    const expected = expectedControlsRows(actions, orderLabels);
    const keysOf = (text) => text.split(": ").slice(1).join(": ");
    assert.deepEqual(rows.map((row) => keysOf(row.text ?? "")).sort(), expected.map((row) => row.keys).sort(),
        "HUD2 the sorted key texts of the Controls rows must equal the sorted key strings of the effective actions");
    const pending = new Map();
    for (const row of expected) pending.set(`${row.label}: ${row.keys}`.trimEnd(), (pending.get(`${row.label}: ${row.keys}`.trimEnd()) ?? 0) + 1);
    for (const row of rows) {
        const text = (row.text ?? "").trimEnd();
        assert((pending.get(text) ?? 0) > 0,
            `HUD2 Controls row ${row.index} "${row.text}" must be the label and the keys of one effective action`);
        pending.set(text, pending.get(text) - 1);
    }
}

// The last row that the product shows in the Controls list. All the facts above are checked here;
// the tail row must be on the screen inside the Controls viewport.
function controlsTailRow(debug, orderLabels = {}) {
    const rows = controlsRows(debug);
    assertControlsTexts(rows, debug.effective_bindings.actions, orderLabels);
    const tail = rows[rows.length - 1];
    assert(tail.visible === true, `HUD2 Controls must visibly render its last row after scrolling: ${tail.text}`);
    assertInsideControlsViewport(tail, "last row");
    return tail;
}

function assertInsideControlsViewport(row, what) {
    const [x, y, width, height] = row.rect || [];
    assert([x, y, width, height].every(Number.isFinite) && width > 0 && height > 0 && x >= CONTROLS_VIEWPORT.left &&
        y >= CONTROLS_VIEWPORT.top && x + width <= CONTROLS_VIEWPORT.right && y + height <= CONTROLS_VIEWPORT.bottom,
    `HUD2 Controls ${what} must fit the actual visible Controls viewport after scrolling: ${row.text} at ${JSON.stringify(row.rect)}`);
}

// Rows that the screenshot shows whole inside the Controls viewport: their glyph pixels are read.
function controlsRowsOnScreen(debug) {
    return controlsRows(debug).filter((row) => row.visible === true && (() => {
        const [x, y, width, height] = row.rect || [];
        return [x, y, width, height].every(Number.isFinite) && x >= CONTROLS_VIEWPORT.left && y >= CONTROLS_VIEWPORT.top &&
            x + width <= CONTROLS_VIEWPORT.right && y + height <= CONTROLS_VIEWPORT.bottom;
    })());
}

function assertGuideVisible(debug, expected, label) {
    const guide = (debug.ui_snapshot || []).find((row) => row.id === "guide" || row.id.startsWith("guide_"));
    assert(guide, `HUD2 ${label} checkpoint must include the authored guide widget`);
    assert.equal(guide.visible, expected, `HUD2 GuideOpen must be ${expected} at ${label}`);
}

function resolveSelectedOwn(checkpoint, target) {
    const selected = checkpoint.debug.selection;
    if (!Array.isArray(selected) || selected.length !== 1) return false;
    const replicas = checkpoint.debug.replicated_world?.units;
    const replica = replicas?.find((unit) => unit.unit_id === selected[0]);
    if (!replica || replica.owner_slot !== 0) return false;
    const sceneUnit = checkpoint.scene.units?.find((unit) => unit.unit_id === replica.unit_id);
    if (!sceneUnit || !Array.isArray(sceneUnit.ground_screen_center)) return false;
    return Math.hypot(sceneUnit.ground_screen_center[0] - target.x,
        sceneUnit.ground_screen_center[1] - target.y) <= 40;
}

function countFillPixels(image, rect, expected, tolerance) {
    let count = 0;
    for (let y = rect.y; y < rect.y + rect.height; y += 1) {
        for (let x = rect.x; x < rect.x + rect.width; x += 1) {
            const offset = (y * image.width + x) * 4;
            if (Math.hypot(
                image.rgba[offset] - expected[0],
                image.rgba[offset + 1] - expected[1],
                image.rgba[offset + 2] - expected[2],
            ) <= tolerance) count += 1;
        }
    }
    return count;
}

module.exports = { judge, controlsTailRow, humanizeAction, orderLabelsByHotkey, controlsRowsOnScreen };
