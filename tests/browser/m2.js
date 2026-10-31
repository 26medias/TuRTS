// turts-test-role: legacy-bridge
"use strict";

// M2 browser interaction helpers. These read only the actual Bevy scene/UI snapshots and use
// Playwright's normal pointer path; they never submit commands through the debug bridge.
const assert = require("node:assert/strict");
const fs = require("node:fs");
const { fixtureOffset, fixtureWorldX, validateTranslationReceipt } = require("./harness/m2-fixture.js");
const M2_PACKAGE_NAME = process.env.M2_PACKAGE_NAME;
const M2_CASE_ID = process.env.M2_CASE_ID ?? "";
const FIXTURE_OFFSET_RAW = process.env.M2_FIXTURE_X_OFFSET;
const FIXTURE_RECEIPT_PATH = process.env.M2_FIXTURE_TRANSLATION_RECEIPT;
const FIXTURE_X_OFFSET = M2_PACKAGE_NAME === undefined
    ? fixtureOffset(undefined, FIXTURE_OFFSET_RAW)
    : validateTranslationReceipt(
        M2_PACKAGE_NAME,
        M2_CASE_ID,
        FIXTURE_OFFSET_RAW,
        FIXTURE_RECEIPT_PATH ? JSON.parse(fs.readFileSync(FIXTURE_RECEIPT_PATH, "utf8")) : null,
    );
const {
    assertOwnerWorldMatchesAudit,
    auditStateAt,
    projectAuditOwnerWorld,
    readCompleteAuditLog,
    worldChangeMatchesAudit,
} = require("./m2-audit.js");

function finitePair(value) {
    return Array.isArray(value) && value.length === 2 && value.every(Number.isFinite);
}

function projectGroundPoint(scene, mapPoint) {
    const matrix = scene?.clip_from_world;
    const viewport = scene?.viewport_logical_rect;
    if (
        !Array.isArray(matrix) || matrix.length !== 16 || !matrix.every(Number.isFinite) ||
        !Array.isArray(viewport) || viewport.length !== 4 || !viewport.every(Number.isFinite) ||
        !finitePair(mapPoint) || scene.coordinate_space !== "window-logical-pixels-top-left"
    ) {
        return null;
    }

    const clipX = matrix[0] * mapPoint[0] + matrix[8] * mapPoint[1] + matrix[12];
    const clipY = matrix[1] * mapPoint[0] + matrix[9] * mapPoint[1] + matrix[13];
    const clipZ = matrix[2] * mapPoint[0] + matrix[10] * mapPoint[1] + matrix[14];
    const clipW = matrix[3] * mapPoint[0] + matrix[11] * mapPoint[1] + matrix[15];
    if (!Number.isFinite(clipW) || clipW <= 0 || !Number.isFinite(clipX) || !Number.isFinite(clipY) || !Number.isFinite(clipZ)) {
        return null;
    }
    const ndcX = clipX / clipW;
    const ndcY = clipY / clipW;
    const ndcZ = clipZ / clipW;
    if (ndcZ < 0 || ndcZ > 1) return null;

    const [left, top, width, height] = viewport;
    const screenX = left + ((ndcX + 1) / 2) * width;
    const screenY = top + ((1 - ndcY) / 2) * height;
    const windowSize = scene.window_logical_size;
    if (
        !finitePair(windowSize) || screenX < left || screenY < top ||
        screenX >= left + width || screenY >= top + height ||
        screenX < 0 || screenY < 0 || screenX >= windowSize[0] || screenY >= windowSize[1]
    ) {
        return null;
    }
    return [screenX, screenY];
}

function projectRenderedEntity(scene, entry) {
    assert(entry && finitePair(entry.ground_screen_center), "rendered entity has no in-viewport ground center");
    assert(Number.isFinite(entry.pick_radius) && entry.pick_radius > 0, "rendered entity has no declared pick radius");
    const translation = entry.translation;
    assert(Array.isArray(translation) && translation.length === 3 && translation.every(Number.isFinite));
    const projected = projectGroundPoint(scene, [translation[0], translation[2]]);
    assert(projected, "the rendered entity ground position must project inside the active viewport");
    const distance = Math.hypot(projected[0] - entry.ground_screen_center[0], projected[1] - entry.ground_screen_center[1]);
    assert(distance <= 1.25, `Rust camera center differs from JS projection by ${distance.toFixed(3)} logical pixels`);
    return entry.ground_screen_center;
}

async function readBridge(page, name) {
    const raw = await page.evaluate((bridge) => {
        const getter = window[bridge];
        return typeof getter === "function" ? getter() : null;
    }, name);
    assert.equal(typeof raw, "string", `window.${name} is not available`);
    return JSON.parse(raw);
}

async function canvasBox(page) {
    const box = await page.locator("canvas").first().boundingBox();
    assert(box && box.width > 0 && box.height > 0, "the rendered Bevy canvas is not visible");
    return box;
}

async function clickWindowLogicalPoint(page, scene, point, options = {}) {
    const [windowWidth, windowHeight] = scene.window_logical_size ?? [];
    assert(Number.isFinite(windowWidth) && windowWidth > 0 && Number.isFinite(windowHeight) && windowHeight > 0);
    const box = await canvasBox(page);
    await page.mouse.click(
        box.x + point[0] * box.width / windowWidth,
        box.y + point[1] * box.height / windowHeight,
        options,
    );
}

async function clickRenderedEntity(page, scene, entry, options = {}) {
    const point = projectRenderedEntity(scene, entry);
    await clickWindowLogicalPoint(page, scene, point, options);
}

async function pressRenderedEntityUntil(page, scene, entry, predicate, timeoutMs, description, options = {}) {
    const point = projectRenderedEntity(scene, entry);
    const [windowWidth, windowHeight] = scene.window_logical_size ?? [];
    assert(Number.isFinite(windowWidth) && windowWidth > 0 && Number.isFinite(windowHeight) && windowHeight > 0);
    const box = await canvasBox(page);
    const x = box.x + point[0] * box.width / windowWidth;
    const y = box.y + point[1] * box.height / windowHeight;
    const button = options.button ?? "left";
    await page.mouse.move(x, y);
    await page.mouse.down({ button });
    try {
        return await waitForSnapshot(page, predicate, timeoutMs, description);
    } finally {
        await page.mouse.up({ button });
    }
}

function uiActionWindowPoint(scene, rect) {
    const [x, y, width, height] = rect ?? [];
    const [viewportX, viewportY, viewportWidth, viewportHeight] = scene?.viewport_physical_rect ?? [];
    const scale = scene?.scale_factor;
    if (
        ![x, y, width, height, viewportX, viewportY, viewportWidth, viewportHeight, scale].every(Number.isFinite) ||
        width <= 0 || height <= 0 || scale <= 0 || x < 0 || y < 0 ||
        x + width > viewportWidth || y + height > viewportHeight
    ) {
        return null;
    }
    return [
        (viewportX + x + width / 2) / scale,
        (viewportY + y + height / 2) / scale,
    ];
}

function findVisibleUiAction(rows, type, predicate = () => true) {
    const normalizedType = String(type).toLowerCase();
    return rows.find((row) =>
        row.kind === "text" && row.visible === true && row.enabled === true &&
        row.action?.type === normalizedType && predicate(row.action, row.text, row)
    );
}

function findPressedUiAction(rows, type, predicate = () => true) {
    return rows.find((row) =>
        row.button_pressed === true && row.action?.type === String(type).toLowerCase() &&
        findVisibleUiAction([row], type, predicate) === row
    );
}

function findVisibleUiText(rows, predicate) {
    return rows.find((row) => row.kind === "text" && row.visible === true && predicate(row.text, row));
}

async function clickRenderedUiAction(page, scene, type, actionPredicate) {
    const rows = await readBridge(page, "turts_ui");
    const control = findVisibleUiAction(rows, type, actionPredicate);
    assert(control, "no visible enabled rendered Text node matches the requested UI action");
    const point = uiActionWindowPoint(scene, control.rect);
    assert(point, `rendered UI action text is partially clipped or lacks projection metadata: ${JSON.stringify(control)}`);
    const [windowWidth, windowHeight] = scene.window_logical_size;
    const box = await canvasBox(page);
    const x = box.x + point[0] * box.width / windowWidth;
    const y = box.y + point[1] * box.height / windowHeight;
    await page.mouse.move(x, y);
    await page.mouse.down();
    try {
        const deadline = Date.now() + 3000;
        let pressed = false;
        while (Date.now() < deadline) {
            const currentRows = await readBridge(page, "turts_ui");
            pressed = Boolean(findPressedUiAction(currentRows, type, actionPredicate));
            if (pressed) break;
            await page.waitForTimeout(50);
        }
        assert(pressed, `real pointer never produced Bevy Pressed state for ${control.text}`);
    } finally {
        await page.mouse.up();
    }
    return control;
}

module.exports = {
    projectGroundPoint,
    projectRenderedEntity,
    uiActionWindowPoint,
    readBridge,
    canvasBox,
    clickWindowLogicalPoint,
    clickRenderedEntity,
    clickRenderedUiAction,
    findVisibleUiAction,
    findPressedUiAction,
    findVisibleUiText,
    visibleTexts,
    hasSingleDebugSelection,
    lastAppliedSequence,
    lastCommand,
    waitForSnapshot,
    captureStage,
    uiActionWindowPoint,
    publicProjection,
    assertPublicProjectionSchema,
};

function decodeSnapshot(raw) {
    const parse = (value) => {
        if (typeof value !== "string" || value.length === 0) return null;
        try { return JSON.parse(value); } catch { return null; }
    };
    return {
        scene: parse(raw.scene_raw),
        ui: parse(raw.ui_raw),
        debug: parse(raw.debug_raw),
        ingestConfigStatus: parse(raw.ingest_config_status_raw),
        error: raw.error,
    };
}

async function liveSnapshot(page) {
    const raw = await page.evaluate(() => ({
        scene_raw: typeof window.turts_scene === "function" ? window.turts_scene() : null,
        ui_raw: typeof window.turts_ui === "function" ? window.turts_ui() : null,
        debug_raw: typeof window.turts_debug === "function" ? window.turts_debug() : null,
        ingest_config_status_raw: typeof window.turts_ingest_config_status === "function" ? window.turts_ingest_config_status() : null,
        error: window.turts_error || null,
    }));
    return { raw, ...decodeSnapshot(raw) };
}

async function configureBrowserIngest(page, config) {
    await page.evaluate((value) => {
        if (typeof window.turts_configure_ingest !== "function") throw new Error("turts_configure_ingest is unavailable");
        window.turts_configure_ingest(value);
    }, config);
    return waitForSnapshot(page, (snapshot) => {
        const status = snapshot.ingestConfigStatus;
        if (status?.state === "rejected") throw new Error(`browser ingest config rejected: ${JSON.stringify(status)}`);
        return status?.state === "applied";
    }, 5000, "queued browser ingest configuration application");
}

async function captureStage(page, outDir, label, logs) {
    const fs = require("node:fs");
    const snapshot = await liveSnapshot(page);
    const safe = label.replace(/[^a-z0-9-]/gi, "-").toLowerCase();
    const screenshot = await page.screenshot({ path: `${outDir}/${safe}-full.png`, fullPage: true });
    await page.locator("canvas").first().screenshot({ path: `${outDir}/${safe}-canvas.png` });
    for (const key of ["scene", "ui", "debug"]) {
        const raw = snapshot.raw[`${key}_raw`];
        if (typeof raw === "string") fs.writeFileSync(`${outDir}/${safe}-${key}-raw.json`, raw + "\n");
    }
    fs.writeFileSync(`${outDir}/${safe}-snapshot.json`, JSON.stringify({
        ...snapshot,
        screenshot_bytes: screenshot.length,
        logs: [...logs],
    }, null, 2) + "\n");
    return snapshot;
}

function visibleTexts(snapshot) {
    return (snapshot.ui ?? [])
        .filter((row) => row.kind === "text" && row.visible === true)
        .map((row) => row.text);
}

function hasSingleDebugSelection(snapshot, unitId) {
    const selected = snapshot?.debug?.selection;
    return Array.isArray(selected) && selected.length === 1 && selected[0] === unitId;
}

function assertPickCircleClearOfPanel(scene, ui, entry) {
    const panel = ui.find((row) => row.kind === "panel" && row.visible);
    assert(panel && panel.rect?.length === 4, "actual visible Bevy economy panel geometry is missing");
    const radius = entry.pick_radius;
    assert(Number.isFinite(radius) && radius > 0, "selected unit has no catalog pick radius");
    const [wx, , wz] = entry.translation;
    const [viewportX, viewportY] = scene.viewport_physical_rect;
    const scale = scene.scale_factor;
    const samples = [];
    for (let index = 0; index < 32; index += 1) {
        const angle = index * Math.PI / 16;
        const point = projectGroundPoint(scene, [wx + Math.cos(angle) * radius, wz + Math.sin(angle) * radius]);
        assert(point, `pick-circle sample ${index} is outside the active camera viewport`);
        samples.push([point[0] * scale - viewportX, point[1] * scale - viewportY]);
    }
    const xs = samples.map((point) => point[0]);
    const ys = samples.map((point) => point[1]);
    const circle = [Math.min(...xs), Math.min(...ys), Math.max(...xs), Math.max(...ys)];
    const [panelX, panelY, panelWidth, panelHeight] = panel.rect;
    const overlaps = circle[0] < panelX + panelWidth && circle[2] > panelX &&
        circle[1] < panelY + panelHeight && circle[3] > panelY;
    assert(!overlaps, `catalog pick circle ${JSON.stringify(circle)} overlaps actual Bevy panel ${JSON.stringify(panel.rect)}`);
    return { panel_rect_physical: panel.rect, projected_pick_bounds_physical: circle };
}

function lastCommand(snapshot) {
    return visibleTexts(snapshot).find((text) => text.startsWith("Last command:")) ?? null;
}

function lastAppliedSequence(snapshot) {
    const text = lastCommand(snapshot);
    if (!text || !text.startsWith("Last command: applied")) return null;
    return Number(text.match(/seq (\d+)/)?.[1] ?? NaN);
}

function balanceOnHand(snapshot) {
    const line = visibleTexts(snapshot).find((text) => text.startsWith("Credits:") && text.includes("on hand"));
    const amount = line?.match(/^Credits: (\d+) on hand\b/);
    return amount ? Number(amount[1]) : null;
}

function jobLine(snapshot, jobId) {
    return visibleTexts(snapshot).find((text) => text.startsWith(`Job ${jobId} · `)) ?? null;
}

function sleep(ms) {
    return new Promise((resolve) => setTimeout(resolve, ms));
}

async function waitForSnapshot(page, predicate, timeoutMs, description, diagnostic = null) {
    const started = Date.now();
    let latest = await liveSnapshot(page);
    const observe = async (snapshot) => {
        if (!diagnostic) return;
        const phase = snapshot.debug?.phase ?? null;
        if (phase !== diagnostic.lastPhase) {
            diagnostic.lastPhase = phase;
            diagnostic.events.push({ at: new Date().toISOString(), kind: "phase", phase });
        }
        const gpuEvents = await page.evaluate(() => window.__m2GpuEvents || []);
        diagnostic.gpuEvents = gpuEvents;
        const lost = gpuEvents.find((event) => event.kind === "device-lost");
        const failed = typeof phase === "string" && phase.startsWith("Failed");
        const appError = snapshot.error;
        if (lost || failed || appError || diagnostic.fatalSignal) {
            throw new Error(`diagnostic fatal state during ${description}: ${JSON.stringify({ phase, appError, lost, fatalSignal: diagnostic.fatalSignal })}`);
        }
    };
    await observe(latest);
    while (Date.now() - started < timeoutMs) {
        if (predicate(latest)) return latest;
        await sleep(100);
        latest = await liveSnapshot(page);
        await observe(latest);
    }
    throw new BrowserCaseTimeoutError(
        `${diagnostic ? "diagnostic " : ""}timeout waiting for ${description} after ${timeoutMs}ms; last phase=${latest.debug?.phase ?? "unknown"}`,
    );
}

function expectedUnitLabel(contentId) {
    const known = {
        "base:command_center": "Command Center",
        "base:harvester": "Harvester",
        "base:rifleman": "Rifleman",
        "haul:depot": "Depot",
        "haul:worker": "Worker",
        "acme:scout": "Scout",
        "cedar:depot": "Task16 Cedar Yard",
        "cedar:worker": "Task16 Carrier",
    };
    return known[contentId] ?? contentId.split(":").at(-1).replaceAll("_", " ");
}

function selectedUnitText(rows, unit) {
    const expected = `Selected: ${expectedUnitLabel(unit.content_id)} · ${unit.unit_id}`;
    return findVisibleUiText(rows, (text) => text === expected);
}

function assertPublicProjectionSchema(snapshot) {
    const view = snapshot.debug?.replicated_world;
    assert(view && Number.isInteger(view.tick), "the coherent public replicated-world view is missing");
    assert.deepEqual(Object.keys(view).sort(), ["belts", "economy", "extractors", "health_trace", "healths", "inserters", "nodes", "research", "tick", "units", "weapons"],
        "browser replicated-world debug must contain the permitted M2/M3 client state only");
    assert.deepEqual(Object.keys(view.economy).sort(), ["balances", "inventories", "orders", "queues"],
        "browser economy debug must expose the complete permitted owner projection");
    for (const balance of view.economy.balances) assert.deepEqual(Object.keys(balance).sort(), ["on_hand", "reserved", "resource"]);
    for (const inventory of view.economy.inventories) assert.deepEqual(Object.keys(inventory).sort(), ["items", "port", "unit"]);
    for (const order of view.economy.orders) assert.deepEqual(Object.keys(order).sort(), ["node", "receiver", "stage", "unit"]);
    for (const queue of view.economy.queues) assert.deepEqual(Object.keys(queue).sort(), ["enabled", "jobs", "producer"]);
    for (const unit of view.units) {
        assert.deepEqual(Object.keys(unit).sort(), ["content_id", "facing", "owner_slot", "pos", "unit_id"]);
    }
    for (const node of view.nodes) {
        assert.deepEqual(Object.keys(node).sort(), ["content_id", "id", "pos", "remaining"]);
    }
    assert(!Object.hasOwn(snapshot.debug, "raw_world_dtos"));
    assert(!Object.hasOwn(snapshot.debug, "foreign_economy"));
    return view;
}

function publicProjection(snapshot) {
    const view = assertPublicProjectionSchema(snapshot);
    return {
        units: view.units.map(({ unit_id, content_id, owner_slot, pos }) => ({ unit_id, content_id, owner_slot, pos }))
            .sort((a, b) => a.unit_id - b.unit_id),
        nodes: view.nodes.map((node) => ({ id: node.id, content_id: node.content_id, pos: node.pos, remaining: node.remaining }))
            .sort((a, b) => a.id - b.id),
    };
}

async function runSecondOwnerConvergenceSmoke(page, pageAddress, packageName, outDir, logs, result, captureStage) {
    const ownerContext = await page.context().browser().newContext({
        viewport: page.viewportSize(),
        deviceScaleFactor: 1,
    });
    const ownerPage = await ownerContext.newPage();
    try {
        await ownerPage.goto(pageAddress.toString(), { waitUntil: "domcontentloaded", timeout: 30000 });
        const owner1 = await waitForSnapshot(ownerPage, (snapshot) =>
            snapshot.debug?.phase?.startsWith("Joined { slot: 1") && snapshot.scene?.units?.length > 0,
        20000, `${packageName} second-owner late join`);
        const owner1Artifact = await captureStage(ownerPage, outDir, "09-owner1-late-join", logs);
        result.stages.second_owner_slot = 1;
        const owner1Units = owner1.debug.replicated_world?.units?.filter((unit) => unit.owner_slot === 1) ?? [];
        if (owner1Units.length > 0) {
            const ownerUnit = owner1Units[0];
            const ownerMesh = owner1.scene.units.find((unit) => unit.unit_id === ownerUnit.unit_id);
            assert(ownerMesh, `${packageName} owner-1 unit is not actually rendered`);
            let ownerSelection;
            try {
                ownerSelection = await pressRenderedEntityUntil(ownerPage, owner1.scene, ownerMesh, (snapshot) =>
                    hasSingleDebugSelection(snapshot, ownerUnit.unit_id) && Boolean(selectedUnitText(snapshot.ui, ownerUnit)),
                5000, `${packageName} owner-1 real selection`);
            } catch (error) {
                await captureStage(ownerPage, outDir, "failure-owner1-selection", logs);
                throw error;
            }
            const ownerSelectionArtifact = await captureStage(ownerPage, outDir, "10-owner1-selection", logs);
            assert.deepEqual(ownerSelectionArtifact.debug.selection, [ownerUnit.unit_id]);
            assert.equal(lastAppliedSequence(ownerSelectionArtifact), lastAppliedSequence(owner1Artifact),
                "owner-1 selection must not issue a world command");
            const ownerCircle = assertPickCircleClearOfPanel(ownerSelectionArtifact.scene, ownerSelectionArtifact.ui, ownerMesh);
            result.stages.owner1_selection = { unit_id: ownerUnit.unit_id, ...ownerCircle, passed: true };
        }

        const deadline = Date.now() + 10000;
        let converged = null;
        let latestPair = null;
        while (Date.now() < deadline) {
            const [owner0Now, owner1Now] = await Promise.all([liveSnapshot(page), liveSnapshot(ownerPage)]);
            latestPair = [owner0Now, owner1Now];
            const world0 = owner0Now.debug?.replicated_world;
            const world1 = owner1Now.debug?.replicated_world;
            if (world0?.tick === world1?.tick && world0.tick !== null &&
                JSON.stringify(publicProjection(owner0Now)) === JSON.stringify(publicProjection(owner1Now))) {
                converged = { world0, world1 };
                break;
            }
            await sleep(100);
        }
        assert(converged, "public world projections differ between owners at a shared coherent tick");
        const projection = publicProjection(latestPair[0]);
        result.stages.public_convergence = {
            unit_count: projection.units.length,
            node_count: projection.nodes.length,
            tick: converged.world0.tick,
            private_payload_excluded: true,
            passed: true,
        };
    } finally {
        await ownerContext.close();
    }
}

async function waitForAuditTick(path, tick, timeoutMs = 10000) {
    const deadline = Date.now() + timeoutMs;
    let latestRecords = [];
    while (Date.now() < deadline) {
        latestRecords = await readCompleteAuditLog(path);
        const found = latestRecords.some((entry) => entry.record === "tick" && entry.data.tick === tick);
        if (found || latestRecords[0]?.data?.economy?.tick === tick) return latestRecords;
        await sleep(50);
    }
    throw new BrowserCaseTimeoutError(`trusted economy audit did not complete tick ${tick}; last complete tick=${latestRecords.filter((entry) => entry.record === "tick").at(-1)?.data?.tick ?? "none"}`);
}

async function assertLossAtAuditTick(kind, drop, auditPath, slot = 0) {
    assert(Number.isInteger(drop?.dropped_tick) && Number.isInteger(drop?.arm_tick), `${kind} drop lacks real arm/drop tick evidence`);
    assert(drop.dropped_tick > drop.arm_tick, `${kind} drop tick must follow its live arm tick`);
    const records = await waitForAuditTick(auditPath, drop.dropped_tick);
    const before = auditStateAt(records, drop.dropped_tick - 1);
    const after = auditStateAt(records, drop.dropped_tick);
    assert(worldChangeMatchesAudit(kind, before, after, slot),
        `${kind} typed browser loss at tick ${drop.dropped_tick} lacks the matching authoritative owner transition`);
    return { records, before, after };
}

function compareBrowserSnapshotToAudit(snapshot, records, slot, label) {
    assertPublicProjectionSchema(snapshot);
    const tick = snapshot.debug.replicated_world.tick;
    const authoritative = auditStateAt(records, tick);
    assertOwnerWorldMatchesAudit(snapshot.debug.replicated_world, authoritative, slot);
    return {
        label,
        slot,
        tick,
        expected_owner_world: projectAuditOwnerWorld(authoritative, slot),
        actual_browser_world: JSON.parse(JSON.stringify(snapshot.debug.replicated_world)),
        passed: true,
    };
}

function assertFinalChangeQuiescence(records, droppedTick, browserTick, observationTick, slot) {
    assert(browserTick >= droppedTick + 8,
        `terminal recovery applied tick ${browserTick} must be at least eight authoritative ticks after final change ${droppedTick}`);
    assert(observationTick >= browserTick,
        `bounded final audit observation tick ${observationTick} must reach the compared browser tick ${browserTick}`);
    const projection = (tick) => {
        const world = projectAuditOwnerWorld(auditStateAt(records, tick), slot);
        const { tick: _tick, ...withoutTick } = world;
        return withoutTick;
    };
    const finalState = projection(droppedTick);
    for (let tick = droppedTick + 1; tick <= observationTick; tick += 1) {
        assert.deepStrictEqual(projection(tick), finalState,
            `owner economy, unit/node lifecycle, queues and orders changed after terminal final_change at audit tick ${tick}`);
        const record = records.find((entry) => entry.record === "tick" && entry.data.tick === tick);
        assert(record, `authoritative quiet interval is missing completed tick ${tick}`);
        assert.equal(record.data.events.length, 0, `terminal final_change interval had an economy event at tick ${tick}`);
    }
    return { browser_applied_tick: browserTick, audit_observation_tick: observationTick,
        first_quiet_tick: droppedTick + 1, last_quiet_tick: observationTick, quiet_tick_count: observationTick - droppedTick };
}

async function runParameterLossProof(page, caseId, kind, outDir, logs, result, captureStage, auditPath, joined) {
    const fs = require("node:fs");
    const snapshot = joined ?? await liveSnapshot(page);
    assert(snapshot.debug?.phase?.startsWith("Joined { slot: 0"), `${kind} loss fixture requires actual owner-0 join`);
    const units = snapshot.debug.replicated_world.units;
    const needsGather = kind === "extraction" || kind === "conversion";
    const worker = units.find((unit) => unit.owner_slot === 0 && unit.content_id === "base:harvester");
    const producer = units.find((unit) => unit.owner_slot === 0 && unit.content_id === "base:command_center" &&
        unit.pos[0] === fixtureWorldX(8, FIXTURE_X_OFFSET) && unit.pos[1] === 8);
    const target = needsGather ? worker : producer;
    const targetMesh = target && snapshot.scene.units.find((unit) => unit.unit_id === target.unit_id);
    assert(target && targetMesh, `${kind} loss fixture lacks its real owner-0 target mesh`);
    const targetLabel = needsGather ? "Harvester" : "Command Center";
    await clickRenderedEntity(page, snapshot.scene, targetMesh);
    await waitForSnapshot(page, (current) => visibleTexts(current).includes(`Selected: ${targetLabel} · ${target.unit_id}`),
        5000, `${kind} target actual rendered selection`);
    const selected = await captureStage(page, outDir, `${kind}-selected`, logs);
    assert.deepEqual(selected.debug.selection, [target.unit_id], `${kind} target selection uses actual Bevy UI`);
    assert.equal(lastAppliedSequence(selected), lastAppliedSequence(snapshot), `${kind} selection must not send a PlayerCommand`);
    assertPickCircleClearOfPanel(selected.scene, selected.ui, selected.scene.units.find((unit) => unit.unit_id === target.unit_id));

    let armed;
    let preAction = selected;
    let jobId = null;
    const arm = async () => {
        const beforeSequence = lastAppliedSequence(await liveSnapshot(page));
        armed = await configureBrowserIngest(page, {
            drop_world_changes: [kind],
            hold_resync_snapshots: 512,
            max_buffered_batches: 3,
        });
        assert.equal(lastAppliedSequence(armed), beforeSequence, `${kind} loss config must not allocate a PlayerCommand`);
        assert.equal(armed.debug.ingest.max_buffered_batches, 3, `${kind} loss must set the live cap to three`);
        assert.deepEqual(armed.debug.ingest.armed_drop_world_changes.kinds, [kind]);
        assert.equal(armed.ingestConfigStatus.state, "applied", `${kind} arm must apply on the Bevy main thread`);
        return armed;
    };
    const produce = async ({ awaitQueue = true } = {}) => {
        const action = findVisibleUiAction(preAction.ui, "produce", (value) => value.producer === producer.unit_id && value.recipe === "base:train_rifleman");
        assert(action, `${kind} loss fixture Command Center has no actual enabled Produce control`);
        const sequence = lastAppliedSequence(preAction);
        await clickRenderedUiAction(page, preAction.scene, "produce", (value) => value.producer === producer.unit_id && value.recipe === "base:train_rifleman");
        const queued = await waitForSnapshot(page, (current) => lastAppliedSequence(current) > sequence &&
            (!awaitQueue || visibleTexts(current).some((text) => text.startsWith("Job "))),
        5000, awaitQueue ? `${kind} real Produce application and rendered queue row` : `${kind} real Produce applied command result before intentional admission loss`);
        const queuedArtifact = await captureStage(page, outDir, awaitQueue ? `${kind}-job-visible` : `${kind}-produce-command-result`, logs);
        if (awaitQueue) {
            jobId = Number(lastCommand(queuedArtifact)?.match(/job (\d+)/)?.[1] ?? NaN);
            assert(Number.isSafeInteger(jobId), `${kind} actual applied Produce must identify its JobId`);
            assert(jobLine(queuedArtifact, jobId), `${kind} real JobId ${jobId} must appear in rendered queue text`);
        }
        preAction = queuedArtifact;
        return queuedArtifact;
    };

    if (kind === "refund") {
        await produce();
        armed = await arm();
        const queueLine = jobLine(preAction, jobId);
        assert(queueLine && /· (?:queued|running) ·/.test(queueLine), "refund loss requires the exact live job to remain cancelable");
        const sequence = lastAppliedSequence(preAction);
        const control = await clickRenderedUiAction(page, preAction.scene, "cancel", (action, text) =>
            action.producer === producer.unit_id && action.job === jobId && text === `Cancel job ${jobId}`);
        assert.equal(control.text, `Cancel job ${jobId}`);
        await waitForSnapshot(page, (current) => lastAppliedSequence(current) > sequence,
            5000, `refund exact JobId ${jobId} actual cancellation result`);
    } else if (kind === "final_change") {
        await produce();
        const blocked = await waitForSnapshot(page, (current) => {
            const unitQueue = current.debug.replicated_world.economy.queues.find((queue) => queue.producer === producer.unit_id);
            return unitQueue?.jobs.some((job) => job.id === jobId && job.stage === "output_blocked") &&
                jobLine(current, jobId)?.includes("output_blocked");
        }, 15000, "final_change exact job reaches actual stable output_blocked stage");
        const blockedArtifact = await captureStage(page, outDir, "final-change-output-blocked", logs);
        assert.equal(lastAppliedSequence(blockedArtifact), lastAppliedSequence(preAction), "waiting output_blocked sends no command");
        const blockedTick = blockedArtifact.debug.replicated_world.tick;
        const stableRecords = await waitForAuditTick(auditPath, blockedTick + 2, 5000);
        const stableAtA = auditStateAt(stableRecords, blockedTick + 1);
        const stableAtB = auditStateAt(stableRecords, blockedTick + 2);
        const stableOwnerAtA = projectAuditOwnerWorld(stableAtA, 0);
        const stableOwnerAtB = projectAuditOwnerWorld(stableAtB, 0);
        delete stableOwnerAtA.tick;
        delete stableOwnerAtB.tick;
        assert.deepStrictEqual(stableOwnerAtA, stableOwnerAtB,
            "output_blocked fixture must have a completed unchanged owner economy interval before arming");
        assert.equal(stableAtB.economy.economy.queues.find((queue) => queue.producer === producer.unit_id)?.jobs
            .find((job) => job.id === jobId)?.stage, "OutputBlocked");
        preAction = blockedArtifact;
        armed = await arm();
        const sequence = lastAppliedSequence(preAction);
        const control = await clickRenderedUiAction(page, preAction.scene, "cancel", (action, text) =>
            action.producer === producer.unit_id && action.job === jobId && text === `Cancel job ${jobId}`);
        assert.equal(control.text, `Cancel job ${jobId}`);
        await waitForSnapshot(page, (current) => lastAppliedSequence(current) > sequence,
            5000, "terminal final_change exact JobId cancellation result while its WorldDelta is intentionally lost");
    } else {
        armed = await arm();
        if (needsGather) {
            const node = selected.scene.nodes.find((entry) => Math.abs(entry.translation[0] - fixtureWorldX(24, FIXTURE_X_OFFSET)) < 1e-3 && Math.abs(entry.translation[2] - 8) < 1e-3);
            assert(node, `${kind} actual ore node mesh is missing`);
            const beforeSequence = lastAppliedSequence(armed);
            await clickRenderedEntity(page, selected.scene, node, { button: "right" });
            await waitForSnapshot(page, (current) => lastAppliedSequence(current) > beforeSequence &&
                visibleTexts(current).some((text) => text.startsWith("Gather:")),
            5000, `${kind} real right-pointer Gather application`);
        } else if (kind === "charge") {
            await produce({ awaitQueue: false });
        } else {
            await produce();
        }
    }

    const armTick = armed.ingestConfigStatus.arm_tick;
    assert(Number.isInteger(armTick), `${kind} live ingest arm must publish its applied authoritative tick`);
    const dropWait = kind === "conversion" ? 60000 : kind === "spawn" ? 20000 : 10000;
    const droppedSnapshot = await waitForSnapshot(page, (current) => {
        const drops = current.debug?.dropped_world_changes?.filter((record) => record.kind === kind && record.arm_tick === armTick) ?? [];
        return drops.length > 0;
    }, dropWait, `${kind} real ${kind} WorldDelta loss`);
    const matchingDrops = droppedSnapshot.debug.dropped_world_changes.filter((record) => record.kind === kind && record.arm_tick === armTick);
    assert.equal(matchingDrops.length, 1, `${kind} fixture must lose exactly one matching delta`);
    const drop = matchingDrops[0];
    const transition = await assertLossAtAuditTick(kind, drop, auditPath, 0);
    let chargedJobId = null;
    if (kind === "charge") {
        const previousJobs = transition.before.economy.economy.queues.find((queue) => queue.producer === producer.unit_id)?.jobs ?? [];
        const chargedJobs = transition.after.economy.economy.queues.find((queue) => queue.producer === producer.unit_id)?.jobs ?? [];
        const newChargedJobs = chargedJobs.filter((job) => job.charged.some((line) =>
            line.source === "SlotBalance" && line.resource.package === "base" && line.resource.local === "credits" && line.amount === 100) &&
            !previousJobs.some((previous) => previous.id === job.id));
        assert.equal(newChargedJobs.length, 1, "charge loss audit tick must add one actually charged owner job");
        chargedJobId = newChargedJobs[0].id;
    }
    const spawnedUnit = kind === "spawn" ? (() => {
        const beforeIds = new Set(transition.before.economy.units.map((unit) => unit.unit_id));
        return transition.after.economy.units.find((unit) => unit.owner_slot === 0 && !beforeIds.has(unit.unit_id));
    })() : null;
    if (kind === "spawn") assert(spawnedUnit, "spawn loss audit tick must contain the real new owner unit");

    const held = await waitForSnapshot(page, (current) => current.debug?.ingest?.held_snapshots === 1 &&
        current.debug.ingest.outstanding_request === true && current.debug.ingest.buffer_overflows > 0,
    20000, `${kind} real resync response held while bounded delta tail overflows`);
    const heldArtifact = await captureStage(page, outDir, `${kind}-held-response`, logs);
    assert.equal(heldArtifact.debug.ingest.max_buffered_batches, 3);
    assert(heldArtifact.debug.ingest.buffered_batches <= 3, "retained real delta tail exceeded configured cap=3");
    assert.equal(heldArtifact.debug.ingest.max_outstanding_requests, 1, "the real browser Replica must never have more than one outstanding request");
    const requestsDuringHold = heldArtifact.debug.ingest.resync_requests_sent;
    const holdUntil = Date.now() + 200;
    while (Date.now() < holdUntil) {
        const current = await liveSnapshot(page);
        assert.equal(current.debug.ingest.resync_requests_sent, requestsDuringHold,
            `${kind} must not issue another resync request while the actual response is held`);
        assert.equal(current.debug.ingest.max_outstanding_requests, 1,
            "only one actual resync request may be outstanding at a time");
        await sleep(50);
    }

    const heldTick = heldArtifact.debug.replicated_world.tick;
    const sequenceAtRelease = lastAppliedSequence(heldArtifact);
    const release = await configureBrowserIngest(page, {
        drop_world_changes: [],
        hold_resync_snapshots: 0,
        max_buffered_batches: 3,
    });
    assert.equal(lastAppliedSequence(release), sequenceAtRelease, `${kind} release must not allocate a command`);
    const recovered = await waitForSnapshot(page, (current) => current.debug?.ingest?.held_snapshots === 0 &&
        current.debug.ingest.outstanding_request === false && current.debug.ingest.resync_active === false &&
        current.debug.ingest.successor_requests >= heldArtifact.debug.ingest.successor_requests + 1 &&
        current.debug.replicated_world.tick > heldTick,
    10000, `${kind} held Snapshot, one successor and full recovery close`);
    assert.equal(recovered.debug.ingest.resync_requests_sent, requestsDuringHold + 1,
        `${kind} recovery must send one successor after releasing the held response`);
    assert.equal(recovered.debug.ingest.max_outstanding_requests, 1);
    if (spawnedUnit) {
        const output = await waitForSnapshot(page, (current) => current.debug?.replicated_world?.units?.some((unit) =>
            unit.unit_id === spawnedUnit.unit_id && unit.content_id === spawnedUnit.content_id) &&
            current.scene?.units?.some((unit) => unit.unit_id === spawnedUnit.unit_id),
        15000, "spawned output is both in the recovered browser replica and actual Bevy mesh scene");
        assert(output, "spawned output did not render after recovery closed");
        await captureStage(page, outDir, "spawn-output-mesh", logs);
    }

    let finalSnapshot = recovered;
    let quiescence = null;
    if (kind === "final_change") {
        finalSnapshot = await waitForSnapshot(page, (current) => current.debug?.replicated_world?.tick >= drop.dropped_tick + 8 &&
            current.debug?.ingest?.resync_active === false && current.debug.ingest.outstanding_request === false,
        10000, "eight completed authoritative heartbeats after terminal final_change recovery");
        assert(finalSnapshot.debug.ingest.highest_delta_tick >= finalSnapshot.debug.replicated_world.tick,
            "empty-heartbeat recovery must show continuing received ticks through the applied tick");
    }
    const actualTick = finalSnapshot.debug.replicated_world.tick;
    let auditRecords = await waitForAuditTick(auditPath, actualTick, 10000);
    const auditComparison = compareBrowserSnapshotToAudit(finalSnapshot, auditRecords, 0, `${kind}-recovered`);
    if (kind === "final_change") {
        auditRecords = await waitForAuditTick(auditPath, actualTick + 4, 5000);
        const observationTick = auditRecords.filter((entry) => entry.record === "tick")
            .reduce((latest, entry) => Math.max(latest, entry.data.tick), 0);
        assert(observationTick <= actualTick + 40,
            `bounded final observation advanced more than 40 audit ticks beyond browser capture: ${observationTick} vs ${actualTick}`);
        quiescence = assertFinalChangeQuiescence(auditRecords, drop.dropped_tick, actualTick, observationTick, 0);
        const before = auditStateAt(auditRecords, drop.dropped_tick - 1);
        const after = auditStateAt(auditRecords, drop.dropped_tick);
        assert(worldChangeMatchesAudit("refund", before, after, 0),
            "terminal final_change must be the actual exact-job cancellation/refund mutation");
        assert.equal(after.economy.economy.queues.some((queue) => queue.producer === producer.unit_id && queue.jobs.some((job) => job.id === jobId)), false,
            "terminal final_change audit state must remove the exact canceled JobId");
    }
    const stages = {
        case_id: caseId,
        loss_family: kind,
        charged_job_id: chargedJobId,
        arm_tick: drop.arm_tick,
        dropped_tick: drop.dropped_tick,
        matching_audit_transition: true,
        held_snapshots: heldArtifact.debug.ingest.held_snapshots,
        configured_buffer_cap: heldArtifact.debug.ingest.max_buffered_batches,
        retained_batches: heldArtifact.debug.ingest.buffered_batches,
        overflow_count: heldArtifact.debug.ingest.buffer_overflows,
        outstanding_request_during_hold: heldArtifact.debug.ingest.outstanding_request,
        requests_during_hold: requestsDuringHold,
        successor_requests: recovered.debug.ingest.successor_requests,
        max_outstanding_requests: recovered.debug.ingest.max_outstanding_requests,
        recovered_tick: recovered.debug.replicated_world.tick,
        final_audit_comparison: auditComparison,
        final_change_quiescence: quiescence,
        audit_log_scope: "private server-side all-slot audit; browser retained only permitted public units/nodes and owner-0 economy",
        raw_dto_privacy_evidence: {
            source: "approved Task15 native raw-wire owner-audit proof, reused as a separate Step1 boundary proof",
            verifier: "crates/turts_server/tests/economy.rs::assert_raw_wire_owner_records and e9_two_owners_late_join_and_output_lifecycle",
            retained_evidence: "/home/julien/.codex/worktrees/b54a/TuRTS/target/task15-evidence/e9-all-classes-rearm-pass",
            raw_snapshot_sample: "/home/julien/.codex/worktrees/b54a/TuRTS/target/task15-evidence/e9-all-classes-rearm-pass/client-32001-wire.jsonl",
            browser_cache: "none; Chrome retains only Bevy public units/nodes and owner-0 economy",
        },
        passed: true,
    };
    result.stages.loss_family = stages;
    fs.writeFileSync(require("node:path").join(outDir, "loss-family-evidence.json"), JSON.stringify(stages, null, 2) + "\n");
    return finalSnapshot;
}

async function runBaseRecoveryProof(page, scene, worker, node, outDir, logs, result, captureStage, auditPath = require("node:path").join(outDir, "economy-audit-private.jsonl")) {
    const before = await liveSnapshot(page);
    const sequenceBefore = lastAppliedSequence(before);
    const initialUnit = before.debug.replicated_world.units.find((unit) => unit.unit_id === worker.unit_id);
    const initialNode = before.debug.replicated_world.nodes.find((item) => item.id === node.node_id);
    assert(initialUnit && Array.isArray(initialUnit.pos), "recovery worker lacks its actual initial replicated position");
    assert(initialNode, "recovery proof node lacks its initial actual replicated amount");
    const kinds = ["extraction"];
    const heldConfig = await configureBrowserIngest(page, {
        drop_world_changes: kinds,
        hold_resync_snapshots: 512,
        max_buffered_batches: 3,
    });
    assert.equal(lastAppliedSequence(heldConfig), sequenceBefore, "configuring a loss must not issue a PlayerCommand");
    assert.equal(heldConfig.debug.ingest.max_buffered_batches, 3, "browser cap must update the already joined Replica");
    assert.deepEqual(heldConfig.debug.ingest.armed_drop_world_changes.kinds, ["extraction"]);

    assert(hasSingleDebugSelection(before, worker.unit_id),
        "recovery proof requires the Harvester selected through the actual rendered UI before loss configuration");
    assert(visibleTexts(before).includes(`Selected: Harvester · ${worker.unit_id}`),
        "recovery proof requires actual visible Harvester selection Text");
    assert.equal(lastAppliedSequence(before), sequenceBefore, "selecting the recovery worker must not issue a PlayerCommand");
    const selectedScene = scene;
    const selectedNode = selectedScene.nodes.find((item) => item.node_id === node.node_id);
    assert(selectedNode, "recovery proof node has no rendered mesh");
    await clickRenderedEntity(page, selectedScene, selectedNode, { button: "right" });
    await waitForSnapshot(page, (snapshot) =>
        lastAppliedSequence(snapshot) > sequenceBefore && visibleTexts(snapshot).some((text) => text.startsWith("Gather:")),
    5000, "real Gather command before intentional extraction loss");

    await waitForSnapshot(page, (snapshot) =>
        snapshot.debug?.ingest?.held_snapshots > 0 && snapshot.debug.ingest.outstanding_request === true,
    15000, "actual held resync Snapshot with one outstanding request");
    const overflowed = await waitForSnapshot(page, (snapshot) =>
        snapshot.debug?.ingest?.held_snapshots > 0 && snapshot.debug.ingest.outstanding_request === true &&
        snapshot.debug.ingest.buffer_overflows > 0,
    10000, "real extraction-loss tail to overflow the live cap while the response remains held");
    const heldArtifact = await captureStage(page, outDir, "08-recovery-held-response", logs);
    assert(heldArtifact.debug.ingest.buffer_overflows > 0, "real cap=3 loss did not produce an observed buffer overflow");
    assert(heldArtifact.debug.ingest.buffered_batches <= 3, "retained real delta tail exceeded configured cap=3");
    assert.equal(heldArtifact.debug.ingest.max_outstanding_requests, 1,
        "only one actual resync request may be outstanding at a time");
    assert.equal(heldArtifact.debug.ingest.held_snapshots, 1,
        "only one actual resync response may remain held for one outstanding request");
    assert.equal(heldArtifact.debug.ingest.max_buffered_batches, 3,
        "the real retained delta tail never exceeds the configured cap");
    assert.equal(heldArtifact.debug.ingest.outstanding_request, true);
    const requestsDuringHold = heldArtifact.debug.ingest.resync_requests_sent;
    const holdDeadline = Date.now() + 200;
    while (Date.now() < holdDeadline) {
        const current = await liveSnapshot(page);
        assert.equal(current.debug.ingest.resync_requests_sent, requestsDuringHold,
            "only one actual resync request may be outstanding at a time");
        assert.equal(current.debug.ingest.max_outstanding_requests, 1);
        await sleep(50);
    }
    const originalArm = heldArtifact.debug.dropped_world_changes.find((record) => record.kind === "extraction");
    assert(originalArm, "actual extraction delta was not dropped by the browser-armed predicate");
    const firstLossAudit = await assertLossAtAuditTick("extraction", originalArm, auditPath, 0);

    const heldAppliedTick = heldArtifact.debug.replicated_world.tick;
    const releaseConfig = await configureBrowserIngest(page, {
        drop_world_changes: kinds,
        hold_resync_snapshots: 0,
        max_buffered_batches: 3,
    });
    assert(releaseConfig.debug.replicated_world.tick >= heldAppliedTick,
        "release configuration must not roll back the applied replica tick");
    assert.equal(releaseConfig.ingestConfigStatus.arm_tick, heldAppliedTick,
        "same-kind re-arm while the first response is held must anchor to the current coherent applied tick");
    assert.equal(lastAppliedSequence(releaseConfig), lastAppliedSequence(heldArtifact),
        "reconfiguration and held Snapshot application must not allocate a command sequence");
    const recoveryObserved = releaseConfig.debug.ingest.held_snapshots === 0 && !releaseConfig.debug.ingest.outstanding_request
        ? releaseConfig
        : await waitForSnapshot(page, (snapshot) =>
            snapshot.debug?.ingest?.held_snapshots === 0 && snapshot.debug.ingest.outstanding_request === false &&
            snapshot.debug.replicated_world?.tick > heldAppliedTick,
        5000, "recovered replica after releasing the pre-existing held Snapshot");
    assert.equal(recoveryObserved.debug.ingest.successor_requests, 1,
        "one successor request must be observed only after the prior held response was released");
    assert.equal(recoveryObserved.debug.ingest.max_outstanding_requests, 1,
        "the real browser Replica must never have more than one outstanding request");
    const firstDropCount = recoveryObserved.debug.dropped_world_changes.filter((record) =>
        record.kind === "extraction" && record.arm_tick === originalArm.arm_tick).length;
    assert.equal(firstDropCount, 1, "the first configured extraction loss must occur exactly once");
    assert.equal(recoveryObserved.debug.ingest.resync_active, false, "coherent held response must close recovery");
    const recoveryAuditRecords = await waitForAuditTick(auditPath, recoveryObserved.debug.replicated_world.tick);
    const recoveryObservationAudit = compareBrowserSnapshotToAudit(recoveryObserved, recoveryAuditRecords, 0, "extraction-recovery-observation");
    const releasedWorker = recoveryObserved.debug.replicated_world.units.find((unit) => unit.unit_id === worker.unit_id);
    assert(releasedWorker, "released full Snapshot omitted the active Harvester");
    assert(Math.hypot(releasedWorker.pos[0] - initialUnit.pos[0], releasedWorker.pos[1] - initialUnit.pos[1]) > 0.25,
        "released Snapshot did not converge the Harvester's moved position");
    const releasedTexts = visibleTexts(recoveryObserved);
    assert(releasedTexts.some((text) => /^Ore: [1-9]\d* on hand/.test(text)),
        "released full Snapshot did not converge the Harvester inventory");
    const releasedNode = recoveryObserved.debug.replicated_world.nodes.find((item) => item.id === node.node_id);
    assert(releasedNode && releasedNode.remaining < initialNode.remaining,
        "released full Snapshot did not converge the actual node depletion");

    const rearmTick = releaseConfig.ingestConfigStatus.arm_tick;
    assert.equal(rearmTick, heldAppliedTick,
        "same-kind re-arm must anchor to the applied authoritative tick");
    const activeRearmMatcher = releaseConfig.debug.ingest.armed_drop_world_changes?.arm_tick === rearmTick;
    const consumedRearmMatcher = releaseConfig.debug.dropped_world_changes?.some((record) =>
        record.kind === "extraction" && record.arm_tick === rearmTick && record.dropped_tick > rearmTick,
    ) === true;
    assert(activeRearmMatcher || consumedRearmMatcher,
        "same-kind re-arm must install a fresh matcher at its applied authoritative tick");
    const redropped = await waitForSnapshot(page, (snapshot) =>
        snapshot.debug?.dropped_world_changes?.filter((record) => record.kind === "extraction" && record.arm_tick === rearmTick).length === 1,
    20000, "same-kind re-arm while the first response was held drops one later real extraction batch");
    const redrop = redropped.debug.dropped_world_changes.find((record) => record.kind === "extraction" && record.arm_tick === rearmTick);
    assert(redrop && redrop.dropped_tick > redrop.arm_tick, "re-armed extraction must drop a later tick exactly once");
    const secondLossAudit = await assertLossAtAuditTick("extraction", redrop, auditPath, 0);
    assert.notEqual(rearmTick, originalArm.arm_tick, "the re-armed extraction must use a distinct arm tick");
    const rearmRecovered = await waitForSnapshot(page, (snapshot) => snapshot.debug?.ingest?.resync_active === false &&
        snapshot.debug.ingest.held_snapshots === 0 && snapshot.debug.ingest.outstanding_request === false &&
        snapshot.debug.replicated_world.tick > redrop.dropped_tick,
    10000, "re-armed extraction loss recovers through its own successor Snapshot");
    const extractionDrops = rearmRecovered.debug.dropped_world_changes.filter((record) => record.kind === "extraction");
    assert.equal(extractionDrops.length, 2, "the scenario must observe exactly the original and re-armed extraction loss");
    assert.deepEqual(extractionDrops.map((record) => record.arm_tick), [originalArm.arm_tick, rearmTick]);
    assert.equal(rearmRecovered.debug.ingest.resync_requests_sent, requestsDuringHold + 2,
        "the held response successor and the later re-armed loss must each issue exactly one request");
    assert.equal(rearmRecovered.debug.ingest.successor_requests, 1,
        "the first held response must cause exactly one successor request");
    assert.equal(rearmRecovered.debug.ingest.max_outstanding_requests, 1,
        "the real browser Replica must never have more than one outstanding request");
    const secondAuditRecords = await waitForAuditTick(auditPath, rearmRecovered.debug.replicated_world.tick);
    const secondAuditComparison = compareBrowserSnapshotToAudit(rearmRecovered, secondAuditRecords, 0, "extraction-rearm-recovery");
    result.stages.recovery = {
        first_drop: originalArm,
        held_snapshot_count: heldArtifact.debug.ingest.held_snapshots,
        configured_buffer_cap: heldArtifact.debug.ingest.max_buffered_batches,
        retained_batches_at_capture: heldArtifact.debug.ingest.buffered_batches,
        observed_overflows: heldArtifact.debug.ingest.buffer_overflows,
        outstanding_request_during_hold: heldArtifact.debug.ingest.outstanding_request,
        requests_during_hold: requestsDuringHold,
        requests_at_recovery_observation: recoveryObserved.debug.ingest.resync_requests_sent,
        requests_after_both_recoveries: rearmRecovered.debug.ingest.resync_requests_sent,
        successor_requests: rearmRecovered.debug.ingest.successor_requests,
        max_outstanding_requests: rearmRecovered.debug.ingest.max_outstanding_requests,
        recovery_observation_tick: recoveryObserved.debug.replicated_world.tick,
        rearm_tick: rearmTick,
        rearmed_drop: redrop,
        first_loss_audit_tick: firstLossAudit.after.economy.tick,
        rearm_loss_audit_tick: secondLossAudit.after.economy.tick,
        recovery_observation_audit_comparison: recoveryObservationAudit,
        rearm_audit_comparison: secondAuditComparison,
        audit_log_scope: "private server-side all-slot audit; browser retained only permitted public units/nodes and owner-0 economy",
        raw_dto_privacy_evidence: {
            source: "approved Task15 native raw-wire owner-audit proof, reused as a separate Step1 boundary proof",
            verifier: "crates/turts_server/tests/economy.rs::assert_raw_wire_owner_records and e9_two_owners_late_join_and_output_lifecycle",
            retained_evidence: "/home/julien/.codex/worktrees/b54a/TuRTS/target/task15-evidence/e9-all-classes-rearm-pass",
            raw_snapshot_sample: "/home/julien/.codex/worktrees/b54a/TuRTS/target/task15-evidence/e9-all-classes-rearm-pass/client-32001-wire.jsonl",
            browser_cache: "none; Chrome retains only Bevy public units/nodes and owner-0 economy",
        },
        passed: true,
    };
    return redropped;
}

async function runAdditionalPackageSmoke(page, pageAddress, packageName, joined, outDir, logs, result, captureStage) {
    const owner0Units = joined.debug.replicated_world?.units?.filter((unit) => unit.owner_slot === 0) ?? [];
    assert(owner0Units.length > 0, `${packageName} fixture has no owner-0 units in the public replica`);
    const owner0Unit = owner0Units.find((unit) => unit.content_id === `${packageName}:depot`) ?? owner0Units[0];
    const owner0Mesh = joined.scene.units.find((unit) => unit.unit_id === owner0Unit.unit_id);
    assert(owner0Mesh, `owner-0 ${owner0Unit.content_id} is absent from rendered meshes`);
    const beforeSequence = lastAppliedSequence(joined);
    await clickRenderedEntity(page, joined.scene, owner0Mesh);
    const selected = await waitForSnapshot(page, (snapshot) =>
        hasSingleDebugSelection(snapshot, owner0Unit.unit_id),
    5000, `${packageName} actual selected unit`);
    const selectedArtifact = await captureStage(page, outDir, "02-owner0-selection", logs);
    const selectedText = selectedUnitText(selectedArtifact.ui, owner0Unit);
    assert(selectedText, "actual rendered unit selection was not visible");
    assert.deepEqual(selectedArtifact.debug.selection, [owner0Unit.unit_id]);
    assert.equal(lastAppliedSequence(selectedArtifact), beforeSequence, "unit selection must not submit a world command");
    const circle = assertPickCircleClearOfPanel(selectedArtifact.scene, selectedArtifact.ui, owner0Mesh);
    result.stages.selection = { package: packageName, unit_id: owner0Unit.unit_id, rendered_text: selectedText.text, ...circle, passed: true };

    if (packageName === "haul" || packageName === "cedar") {
        const producer = owner0Units.find((unit) => unit.content_id === `${packageName}:depot`);
        const producerMesh = producer && selectedArtifact.scene.units.find((unit) => unit.unit_id === producer.unit_id);
        assert(producer && producerMesh, `${packageName} fixture producer is not rendered`);
        if (producer.unit_id !== owner0Unit.unit_id) {
            await clickRenderedEntity(page, selectedArtifact.scene, producerMesh);
        }
        const producerSelected = await waitForSnapshot(page, (snapshot) =>
            visibleTexts(snapshot).some((text) => text.startsWith("Selected:") && text.endsWith(`· ${producer.unit_id}`)),
        5000, `${packageName} producer selection`);
        const producerArtifact = await captureStage(page, outDir, "03-producer-selection", logs);
        const produceControl = findVisibleUiAction(producerArtifact.ui, "produce", (action) => action.producer === producer.unit_id);
        assert(produceControl, `${packageName} producer has no visible enabled Produce control`);
        await clickRenderedUiAction(page, producerArtifact.scene, "produce", (action) => action.producer === producer.unit_id);
        const queued = await waitForSnapshot(page, (snapshot) =>
            lastAppliedSequence(snapshot) !== null && visibleTexts(snapshot).some((text) => text.startsWith("Job ")),
        5000, `${packageName} applied production`);
        const queuedArtifact = await captureStage(page, outDir, "04-production-queued", logs);
        const jobId = Number(lastCommand(queuedArtifact)?.match(/job (\d+)/)?.[1] ?? NaN);
        assert(Number.isSafeInteger(jobId), `${packageName} applied production did not identify its JobId`);
        assert(jobLine(queuedArtifact, jobId), `${packageName} Job ${jobId} is absent from actual visible queue text`);
        const queueSequence = lastAppliedSequence(queuedArtifact);
        await clickRenderedUiAction(page, queuedArtifact.scene, "cancel", (action, text) =>
            action.producer === producer.unit_id && action.job === jobId && text === `Cancel job ${jobId}`);
        const cancelled = await waitForSnapshot(page, (snapshot) =>
            lastAppliedSequence(snapshot) > queueSequence && jobLine(snapshot, jobId) === null,
        5000, `${packageName} applied exact-job cancellation`);
        const cancelledArtifact = await captureStage(page, outDir, "05-production-cancelled", logs);
        assert.equal(jobLine(cancelledArtifact, jobId), null);
        result.stages.production_cancel = { package: packageName, job_id: jobId, passed: true };
    }

    if (packageName === "haul" || packageName === "cedar") {
        const worker = owner0Units.find((unit) => unit.content_id === `${packageName}:worker`);
        const workerMesh = worker && joined.scene.units.find((unit) => unit.unit_id === worker.unit_id);
        const node = joined.scene.nodes[0];
        assert(worker && workerMesh && node, `${packageName} gather fixture is missing its worker or rendered node`);
        await clickRenderedEntity(page, joined.scene, workerMesh);
        const workerSelected = await waitForSnapshot(page, (snapshot) =>
            visibleTexts(snapshot).some((text) => text.startsWith("Selected:") && text.endsWith(`· ${worker.unit_id}`)),
        5000, `${packageName} gatherer selection`);
        const workerArtifact = await captureStage(page, outDir, "06-gatherer-selection", logs);
        const initialNode = workerArtifact.debug.replicated_world.nodes.find((item) => item.id === node.node_id);
        assert(initialNode, `${packageName} node absent from coherent public node view`);
        const gatherSequenceBefore = lastAppliedSequence(workerArtifact) ?? 0;
        await clickRenderedEntity(page, workerArtifact.scene, node, { button: "right" });
        const gatherStart = await waitForSnapshot(page, (snapshot) =>
            lastAppliedSequence(snapshot) > gatherSequenceBefore &&
            visibleTexts(snapshot).some((text) => text.startsWith("Gather:")),
        5000, `${packageName} real Gather command and rendered status`);
        const gatherStartArtifact = await captureStage(page, outDir, "07-gather-command", logs);
        const resourcePrefix = packageName === "haul" ? "Ore:" : "Timber:";
        const gathered = await waitForSnapshot(page, (snapshot) => {
            const inventory = visibleTexts(snapshot).some((text) => text.startsWith(resourcePrefix) && /[1-9]\d* on hand/.test(text));
            const current = snapshot.debug?.replicated_world?.nodes?.find((item) => item.id === node.node_id);
            return inventory && current && current.remaining < initialNode.remaining;
        }, 20000, `${packageName} actual extraction and inventory increase`);
        const gatheredArtifact = await captureStage(page, outDir, "08-gather-positive", logs);
        assert(visibleTexts(gatheredArtifact).some((text) => text.startsWith(resourcePrefix) && /[1-9]\d* on hand/.test(text)),
            "actual rendered Gather did not produce inventory");
        const finalNode = gatheredArtifact.debug.replicated_world.nodes.find((item) => item.id === node.node_id);
        assert(finalNode.remaining < initialNode.remaining, "Gather did not reduce the public node amount");
        result.stages.gather = { package: packageName, unit_id: worker.unit_id, node_id: node.node_id, initial_remaining: initialNode.remaining, final_remaining: finalNode.remaining, passed: true };
    }

}

let browserCaseRun = null;

function browserCaseTerminal(kind, caseId, message = null) {
    const suffix = message === null ? "" : ` ${JSON.stringify(message)}`;
    console.log(`M2_CASE_${kind} ${caseId}${suffix}`);
    browserCaseRun.completed += 1;
    if (kind === "PASS") browserCaseRun.passed += 1;
    if (kind === "SEMANTIC_FAIL") browserCaseRun.semanticFailed += 1;
    if (kind === "INFRA_FAIL") browserCaseRun.infrastructureFailed += 1;
    if (kind === "TIMEOUT") browserCaseRun.timedOut += 1;
    browserCaseRun.terminal.add(caseId);
}

async function runBrowserCase(caseId, body) {
    const selected = browserCaseRun.targetCaseId === null || browserCaseRun.targetCaseId === caseId;
    if (!selected) return body();
    console.log(`M2_CASE_START ${caseId}`);
    browserCaseRun.started.add(caseId);
    try {
        const result = await body();
        browserCaseTerminal("PASS", caseId);
        return result;
    } catch (error) {
        const timeout = error instanceof BrowserCaseTimeoutError;
        const semantic = error instanceof assert.AssertionError;
        browserCaseTerminal(timeout ? "TIMEOUT" : semantic ? "SEMANTIC_FAIL" : "INFRA_FAIL", caseId, error.message || String(error));
        throw error;
    }
}

function emitBrowserCaseSummary() {
    if (!browserCaseRun) return;
    const summary = {
        selected_count: browserCaseRun.selectedCount,
        completed_count: browserCaseRun.completed,
        passed_count: browserCaseRun.passed,
        semantic_failed_count: browserCaseRun.semanticFailed,
        infrastructure_failed_count: browserCaseRun.infrastructureFailed,
        timeout_count: browserCaseRun.timedOut,
    };
    console.log(`M2_CASE_SUMMARY ${JSON.stringify(summary)}`);
}

class BrowserCaseTimeoutError extends Error {}

async function runBaseUiSmoke(argv) {
    const fs = require("node:fs");
    const path = require("node:path");
    const [pageUrl, serverUrl, wsUrl, outDir] = argv;
    const packageName = process.env.M2_PACKAGE_NAME || "base";
    const targetCaseId = process.env.M2_CASE_ID || null;
    const caseIds = {
        base: ["E9-browser-base-selection", "E9-browser-base-economy", "E9-browser-base-recovery", "E9-browser-base-public-convergence"],
        haul: ["E9-browser-haul-economy", "E9-browser-haul-public-convergence"],
        acme: ["E9-browser-acme-selection", "E9-browser-acme-public-convergence"],
        cedar: ["E9-browser-cedar-economy", "E9-browser-cedar-public-convergence"],
    }[packageName];
    if (!caseIds) throw new Error(`unknown browser package ${packageName}`);
    const lossFamilies = ["conversion", "charge", "refund", "spawn", "final_change"];
    const lossCaseIds = lossFamilies.map((kind) => `E9-browser-base-loss-${kind}`);
    if (targetCaseId && !caseIds.includes(targetCaseId) && !lossCaseIds.includes(targetCaseId)) {
        throw new Error(`browser case ${targetCaseId} does not belong to package ${packageName}`);
    }
    browserCaseRun = {
        targetCaseId,
        selectedCount: targetCaseId ? 1 : caseIds.length,
        completed: 0,
        passed: 0,
        semanticFailed: 0,
        infrastructureFailed: 0,
        timedOut: 0,
        started: new Set(),
        terminal: new Set(),
    };
    const diagnosticMode = argv[4] === "--diagnostic-replay" || argv[4] === "--selection-only-diagnostic";
    const selectionOnly = argv[4] === "--selection-only" || argv[4] === "--selection-only-diagnostic";
    assert(pageUrl && serverUrl && wsUrl && outDir, "usage: node m2.js <static-url> <server-url> <ws-url> <artifact-dir> [--selection-only|--full-smoke|--diagnostic-replay]");
    fs.mkdirSync(outDir, { recursive: true });
    const { chromium } = require("playwright-core");
    const chromeArgs = process.env.M2_CHROME_ARGS_JSON
        ? JSON.parse(process.env.M2_CHROME_ARGS_JSON)
        : ["--enable-unsafe-webgpu", "--use-webgpu-adapter=swiftshader"];
    const ignoreDefaultArgs = process.env.M2_IGNORE_DEFAULT_ARGS_JSON
        ? JSON.parse(process.env.M2_IGNORE_DEFAULT_ARGS_JSON)
        : undefined;
    const headless = process.env.M2_HEADLESS !== "false";
    assert(Array.isArray(chromeArgs) && chromeArgs.every((arg) => typeof arg === "string"), "M2_CHROME_ARGS_JSON must be a JSON string array");
    assert(ignoreDefaultArgs === undefined || (Array.isArray(ignoreDefaultArgs) && ignoreDefaultArgs.every((arg) => typeof arg === "string")),
        "M2_IGNORE_DEFAULT_ARGS_JSON must be a JSON string array");
    let browser;
    let page;
    try {
        browser = await chromium.launch({
            executablePath: process.env.CHROME_BIN || "/usr/bin/google-chrome",
            headless,
            args: chromeArgs,
            ...(ignoreDefaultArgs ? { ignoreDefaultArgs } : {}),
        });
        page = await browser.newPage({ viewport: { width: 1280, height: 720 }, deviceScaleFactor: 1 });
    } catch (error) {
        if (browser) await browser.close().catch(() => {});
        if (targetCaseId) {
            console.log(`M2_CASE_START ${targetCaseId}`);
            browserCaseRun.started.add(targetCaseId);
            browserCaseTerminal("INFRA_FAIL", targetCaseId, error.message || String(error));
            emitBrowserCaseSummary();
            browserCaseRun = null;
        }
        throw error;
    }
    const logs = [];
    const diagnostic = diagnosticMode ? { events: [], gpuEvents: [], lastPhase: undefined, fatalSignal: null } : null;
    const timestamp = () => new Date().toISOString();
    page.on("console", (message) => {
        const entry = `${timestamp()} console ${message.type()}: ${message.text()}`;
        logs.push(entry);
        if (diagnostic) {
            const event = { at: timestamp(), kind: "console", level: message.type(), text: message.text() };
            diagnostic.events.push(event);
            if (/DeviceLost|device lost|Quitting the application due to/i.test(message.text())) diagnostic.fatalSignal = event;
        }
    });
    page.on("pageerror", (error) => {
        logs.push(`${timestamp()} pageerror: ${error.message}`);
        if (diagnostic) diagnostic.events.push({ at: timestamp(), kind: "pageerror", message: error.message, stack: error.stack });
    });
    page.on("request", (request) => {
        if (diagnostic) diagnostic.events.push({ at: timestamp(), kind: "request", method: request.method(), url: request.url() });
    });
    page.on("response", (response) => {
        if (diagnostic) diagnostic.events.push({ at: timestamp(), kind: "response", status: response.status(), url: response.url() });
    });
    page.on("requestfailed", (request) => {
        const failure = (request.failure() || {}).errorText || "";
        logs.push(`${timestamp()} requestfailed: ${request.url()} ${failure}`);
        if (diagnostic) diagnostic.events.push({ at: timestamp(), kind: "requestfailed", url: request.url(), error: failure });
    });
    page.on("close", () => {
        if (diagnostic) diagnostic.events.push({ at: timestamp(), kind: "page-close" });
    });
    if (diagnostic) {
        await page.addInitScript(() => {
            window.__m2GpuEvents = [];
            const emit = (event) => window.__m2GpuEvents.push({ at: Date.now(), ...event });
            const adapterRequestDevice = globalThis.GPUAdapter?.prototype?.requestDevice;
            if (adapterRequestDevice) {
                globalThis.GPUAdapter.prototype.requestDevice = function (...args) {
                    emit({ kind: "request-device-start" });
                    return adapterRequestDevice.apply(this, args).then((device) => {
                        emit({ kind: "request-device-resolved" });
                        device.lost.then((info) => emit({ kind: "device-lost", reason: info.reason, message: info.message }));
                        return device;
                    }, (error) => {
                        emit({ kind: "request-device-rejected", message: String(error) });
                        throw error;
                    });
                };
            } else {
                emit({ kind: "adapter-prototype-unavailable" });
            }
            const deviceDestroy = globalThis.GPUDevice?.prototype?.destroy;
            if (deviceDestroy) {
                globalThis.GPUDevice.prototype.destroy = function (...args) {
                    emit({ kind: "device-destroy", stack: new Error("GPUDevice.destroy called").stack });
                    return deviceDestroy.apply(this, args);
                };
            } else {
                emit({ kind: "device-destroy-method-unavailable" });
            }
        });
    }
    const result = {
        objective: diagnosticMode ? "Task 16 bounded first BrowserWebGPU loss diagnostic; not acceptance" : "Task 16 UI positive smoke prep; not final E9/Task 16 acceptance",
        chrome: browser.version(),
        chrome_launch_args: chromeArgs,
        chrome_ignore_default_args: ignoreDefaultArgs ?? [],
        chrome_headless: headless,
        page_url: pageUrl,
        server_url: serverUrl,
        ws_url: wsUrl,
        stages: {},
        errors: [],
    };
    let cdp = null;
    const ownedChromeProcesses = () => {
        const fs = require("node:fs");
        const rows = require("node:child_process").execFileSync("ps", ["-eo", "pid=,ppid=,comm="], { encoding: "utf8" }).trim().split("\n");
        const processes = rows.map((row) => {
            const match = row.trim().match(/^(\d+)\s+(\d+)\s+(.*)$/);
            return match ? { pid: Number(match[1]), ppid: Number(match[2]), comm: match[3] } : null;
        }).filter(Boolean);
        const owned = new Set([process.pid]);
        let changed = true;
        while (changed) {
            changed = false;
            for (const row of processes) {
                if (owned.has(row.ppid) && !owned.has(row.pid)) { owned.add(row.pid); changed = true; }
            }
        }
        return processes.filter((row) => owned.has(row.pid) && /chrome|chromium/i.test(row.comm)).map(({ pid, ppid }) => {
            let args = "";
            try { args = fs.readFileSync(`/proc/${pid}/cmdline`).toString().split("\0").filter(Boolean).join(" "); }
            catch (error) { args = `cmdline unavailable: ${error.message}`; }
            return { pid, ppid, args };
        });
    };
    const captureChromeGpuEnvironment = async (stage, scene) => {
        if (!cdp) return;
        const environment = { stage, actual_bevy_render_adapter: scene?.render_adapter ?? null, captured_at: timestamp(), owned_chrome_process_argv: ownedChromeProcesses() };
        try { environment.system_info = await cdp.send("SystemInfo.getInfo"); }
        catch (error) { environment.system_info_error = String(error); }
        result.stages.chrome_gpu_environment = environment;
        if (diagnostic) {
            diagnostic.events.push({ at: timestamp(), kind: "chrome-gpu-environment", stage, adapter: environment.actual_bevy_render_adapter, gpu_devices: environment.system_info?.gpu?.devices ?? null, feature_status: environment.system_info?.gpu?.featureStatus ?? null, crash_count: environment.system_info?.gpu?.auxAttributes?.processCrashCount ?? null });
        }
    };
    cdp = await browser.newBrowserCDPSession();
    if (diagnostic) {
        cdp.on("Target.targetCrashed", (event) => diagnostic.events.push({ at: timestamp(), kind: "cdp-target-crashed", ...event }));
        result.diagnostic = { events: diagnostic.events, gpu_events: diagnostic.gpuEvents };
    }
    const finish = async () => {
        if (diagnostic) result.diagnostic = { events: diagnostic.events, gpu_events: diagnostic.gpuEvents };
        fs.writeFileSync(path.join(outDir, "result.json"), JSON.stringify(result, null, 2) + "\n");
        await browser.close();
        if (diagnostic) {
            diagnostic.events.push({ at: timestamp(), kind: "browser-close-complete" });
            result.diagnostic = { events: diagnostic.events, gpu_events: diagnostic.gpuEvents };
            fs.writeFileSync(path.join(outDir, "result.json"), JSON.stringify(result, null, 2) + "\n");
        }
    };
    const pageAddress = new URL(pageUrl);
    pageAddress.searchParams.set("server", serverUrl);
    pageAddress.searchParams.set("ws", wsUrl);

    try {
        await page.goto(pageAddress.toString(), { waitUntil: "domcontentloaded", timeout: 30000 });
        const joined = await waitForSnapshot(page, (snapshot) =>
            snapshot.debug?.phase?.startsWith("Joined") &&
            snapshot.scene?.cameras_3d === 1 &&
            snapshot.debug.units?.some((unit) => unit.content_id.startsWith(`${packageName}:`)),
        20000, `${packageName} join and rendered scene`, diagnostic);
        const joinedArtifact = await captureStage(page, outDir, "01-joined", logs);
        assert(joined.debug?.phase?.startsWith("Joined { slot: 0"), `expected first owner slot, saw ${joined.debug?.phase}`);
        await captureChromeGpuEnvironment("joined", joined.scene);
        const renderAdapter = joined.scene?.render_adapter;
        assert(renderAdapter && typeof renderAdapter.backend === "string" && renderAdapter.backend.toLowerCase().includes("webgpu"),
            `Bevy did not select a WebGPU adapter: ${JSON.stringify(renderAdapter)}`);
        assert(typeof renderAdapter.name === "string" && ["Cpu", "IntegratedGpu", "DiscreteGpu", "VirtualGpu", "Other"].includes(renderAdapter.device_type),
            `actual WebGPU adapter metadata is incomplete: ${JSON.stringify(renderAdapter)}`);
        result.stages.renderer = { ...renderAdapter, backend_provenance: "Bevy RenderAdapterInfo", chrome_launch_args: chromeArgs };
        assert.equal(joined.scene?.cameras_3d, 1, "one active primary 3D camera must render the scene");
        assert(joined.scene?.clip_from_world?.length === 16, "actual camera projection metadata is missing");
        assert(joined.scene?.units?.length > 0, "no rendered unit meshes in the actual scene");
        assert.equal(joinedArtifact.error, null, `browser app error: ${joinedArtifact.error}`);
        if (targetCaseId === `E9-browser-${packageName}-public-convergence`) {
            await runBrowserCase(targetCaseId, () =>
                runSecondOwnerConvergenceSmoke(page, pageAddress, packageName, outDir, logs, result, captureStage));
            result.stages.browser_logs = logs;
            fs.writeFileSync(path.join(outDir, "result.json"), JSON.stringify(result, null, 2) + "\n");
            return;
        }
        if (packageName !== "base") {
            const economyCaseId = packageName === "cedar" ? "E9-browser-cedar-economy" :
                packageName === "haul" ? "E9-browser-haul-economy" : "E9-browser-acme-selection";
            await runBrowserCase(economyCaseId, async () =>
                runAdditionalPackageSmoke(page, pageAddress, packageName, joinedArtifact, outDir, logs, result, captureStage));
            if (!targetCaseId) {
                await runBrowserCase(`E9-browser-${packageName}-public-convergence`, () =>
                    runSecondOwnerConvergenceSmoke(page, pageAddress, packageName, outDir, logs, result, captureStage));
            }
            result.stages.browser_logs = logs;
            fs.writeFileSync(path.join(outDir, "result.json"), JSON.stringify(result, null, 2) + "\n");
            return;
        }

        if (targetCaseId && lossCaseIds.includes(targetCaseId)) {
            const kind = targetCaseId.slice("E9-browser-base-loss-".length);
            await runBrowserCase(targetCaseId, () => runParameterLossProof(
                page, targetCaseId, kind, outDir, logs, result, captureStage,
                path.join(outDir, "economy-audit-private.jsonl"), joinedArtifact,
            ));
            result.stages.browser_logs = logs;
            fs.writeFileSync(path.join(outDir, "result.json"), JSON.stringify(result, null, 2) + "\n");
            return;
        }

        // The fixture's owner-0 Command Center is selected from its received public unit position.
        // The click uses the actual rendered mesh center and Rust-published active-camera projection.
        const commandCenter = joined.debug.units.find((unit) =>
            unit.content_id === "base:command_center" && Math.abs(unit.pos[0] - fixtureWorldX(8, FIXTURE_X_OFFSET)) < 1e-3 && Math.abs(unit.pos[1] - 8) < 1e-3
        );
        const commandCenterMesh = commandCenter && joined.scene.units.find((unit) => unit.unit_id === commandCenter.unit_id);
        assert(commandCenter && commandCenterMesh, "owner-0 Command Center is absent from actual received/rendered state");
        result.stages.selection_target = { unit_id: commandCenter.unit_id, world_pos: commandCenter.pos };
        await clickRenderedEntity(page, joined.scene, commandCenterMesh);
        const selected = await waitForSnapshot(page, (snapshot) => {
            const texts = visibleTexts(snapshot);
            return hasSingleDebugSelection(snapshot, commandCenter.unit_id) &&
                texts.some((text) => /^Credits:/.test(text)) &&
                !texts.includes("Select a unit or producer");
        }, 5000, "Command Center selection and rebuilt owner economy panel", diagnostic);
        let selectedArtifact;
        let producerId;
        let visibleProduceAction;
        await runBrowserCase("E9-browser-base-selection", async () => {
            selectedArtifact = await captureStage(page, outDir, "02-selected-command-center", logs);
            await captureChromeGpuEnvironment("selected-command-center", selectedArtifact.scene);
            const pickCircleGeometry = assertPickCircleClearOfPanel(selectedArtifact.scene, selectedArtifact.ui, commandCenterMesh);
            const selectedText = visibleTexts(selectedArtifact).find((text) => text.startsWith("Selected:"));
            assert.equal(selectedText, `Selected: Command Center · ${commandCenter.unit_id}`, "selection must appear in actual visible Bevy Text");
            assert(hasSingleDebugSelection(selectedArtifact, commandCenter.unit_id), "read-only debug selection must agree with actual rendered Text");
            assert(!visibleTexts(selectedArtifact).some((text) => text.startsWith("Last command:")), "unit selection must not submit a world command");
            producerId = commandCenter.unit_id;
            visibleProduceAction = findVisibleUiAction(selectedArtifact.ui, "produce", (action, text) =>
                text === "Produce Train Rifleman" && action.producer === producerId
            );
            result.stages.selection = {
                unit_id: commandCenter.unit_id,
                rendered_text: selectedText,
                producer_id: producerId,
                producer_action_visible: Boolean(visibleProduceAction),
                ...pickCircleGeometry,
                passed: true,
            };
        });
        if (selectionOnly || targetCaseId === "E9-browser-base-selection") {
            result.stages.browser_logs = logs;
            fs.writeFileSync(path.join(outDir, "result.json"), JSON.stringify(result, null, 2) + "\n");
            return;
        }
        if (targetCaseId === "E9-browser-base-recovery") {
            const units = joinedArtifact.debug.replicated_world.units;
            const harvester = units.find((unit) => unit.content_id === "base:harvester" && unit.owner_slot === 0);
            const mesh = harvester && joinedArtifact.scene.units.find((unit) => unit.unit_id === harvester.unit_id);
            const node = joinedArtifact.scene.nodes.find((candidate) =>
                Math.abs(candidate.translation[0] - fixtureWorldX(24, FIXTURE_X_OFFSET)) < 1e-3 && Math.abs(candidate.translation[2] - 8) < 1e-3);
            assert(harvester && mesh && node, "base recovery fixture lacks its owner-0 Harvester or ore node");
            await clickRenderedEntity(page, joinedArtifact.scene, mesh);
            await waitForSnapshot(page, (snapshot) =>
                hasSingleDebugSelection(snapshot, harvester.unit_id) &&
                visibleTexts(snapshot).some((text) => text.startsWith("Gather:")),
            5000, "recovery Harvester selection and rebuilt Gather row");
            const workerSelection = await captureStage(page, outDir, "02-recovery-harvester-selected", logs);
            const selectedHarvester = workerSelection.debug.replicated_world.units.find((unit) => unit.unit_id === harvester.unit_id);
            const recoveryArtifact = await runBrowserCase("E9-browser-base-recovery", () => runBaseRecoveryProof(
                page, workerSelection.scene, selectedHarvester, node, outDir, logs, result, captureStage));
            result.stages.recovery_final_tick = recoveryArtifact.debug.replicated_world.tick;
            result.stages.browser_logs = logs;
            fs.writeFileSync(path.join(outDir, "result.json"), JSON.stringify(result, null, 2) + "\n");
            return;
        }

        await runBrowserCase("E9-browser-base-economy", async () => {
        assert(visibleProduceAction, "no visible enabled rendered Produce control for the selected producer");
        const trainAction = (action, text) =>
            text === "Produce Train Rifleman" && action.producer === producerId && action.recipe === "base:train_rifleman";
        const produceControl = await clickRenderedUiAction(page, selectedArtifact.scene, "produce", trainAction);
        const firstApplied = await waitForSnapshot(page, (snapshot) =>
            lastAppliedSequence(snapshot) !== null && visibleTexts(snapshot).some((text) => text.startsWith("Job ")),
        5000, "applied production and visible queue row");
        const firstQueuedArtifact = await captureStage(page, outDir, "03-first-production-queued", logs);
        assert(produceControl.text === "Produce Train Rifleman", "the real visible production control was not clicked");
        assert(lastAppliedSequence(firstQueuedArtifact) !== null, `production did not apply: ${lastCommand(firstQueuedArtifact)}`);
        const firstJobMatch = lastCommand(firstQueuedArtifact).match(/job (\d+)/);
        assert(firstJobMatch, "applied production result did not expose its JobId in rendered Text");
        const initialUnitIds = new Set(joined.debug.units.map((unit) => unit.unit_id));
        const output = await waitForSnapshot(page, (snapshot) =>
            snapshot.debug?.units?.some((unit) => unit.content_id === "base:rifleman" && !initialUnitIds.has(unit.unit_id)),
        15000, "actual completed rifleman output and rendered mesh");
        const outputArtifact = await captureStage(page, outDir, "04-first-output-mesh", logs);
        const riflemen = outputArtifact.debug.units.filter((unit) => unit.content_id === "base:rifleman" && !initialUnitIds.has(unit.unit_id));
        assert.equal(riflemen.length, 1, "exactly one new rifleman must appear in received state");
        assert.equal(outputArtifact.scene.units.filter((unit) => unit.unit_id === riflemen[0].unit_id).length, 1, "the completed output must have exactly one rendered mesh");
        result.stages.output = { first_job_id: Number(firstJobMatch[1]), unit_id: riflemen[0].unit_id, rendered_mesh_count: 1, passed: true };

        // Keep the Harvester idle until the running production cancellation/refund is observed.
        const creditsBeforeSecond = balanceOnHand(outputArtifact);
        assert.equal(creditsBeforeSecond, 400, "first production should have charged the declared 100-credit cost");
        await clickRenderedUiAction(page, outputArtifact.scene, "produce", trainAction);
        const secondQueued = await waitForSnapshot(page, (snapshot) => {
            const seq = lastAppliedSequence(snapshot);
            return seq !== null && visibleTexts(snapshot).filter((text) => text.startsWith("Job ")).length > 0;
        }, 5000, "second applied production and visible queue row");
        const secondQueuedArtifact = await captureStage(page, outDir, "05-second-production-queue-before-cancel", logs);
        const secondAppliedLine = lastCommand(secondQueuedArtifact);
        const secondJobId = Number(secondAppliedLine.match(/job (\d+)/)?.[1] ?? NaN);
        assert(Number.isSafeInteger(secondJobId), "second applied production did not expose its JobId");
        const secondJob = jobLine(secondQueuedArtifact, secondJobId);
        assert(secondJob, `second JobId ${secondJobId} is not rendered in the visible queue`);
        const stage = secondJob.match(/· (queued|running|output_blocked) ·/)?.[1];
        assert(stage, `unknown rendered job stage: ${secondJob}`);
        const expectedRefund = stage === "running" ? 50 : 100;
        const expectedCredits = creditsBeforeSecond - 100 + expectedRefund;
        const cancelControl = await clickRenderedUiAction(page, secondQueuedArtifact.scene, "cancel", (action, text) =>
            text === `Cancel job ${secondJobId}` && action.producer === producerId && action.job === secondJobId
        );
        assert.equal(cancelControl.text, `Cancel job ${secondJobId}`, "cancel must target the exact applied JobId");
        const cancelSequenceBefore = lastAppliedSequence(secondQueuedArtifact);
        const cancelled = await waitForSnapshot(page, (snapshot) => {
            const seq = lastAppliedSequence(snapshot);
            return seq !== null && seq > cancelSequenceBefore && jobLine(snapshot, secondJobId) === null && balanceOnHand(snapshot) === expectedCredits;
        }, 5000, "applied exact-job cancellation and exact visible refund");
        const cancelledArtifact = await captureStage(page, outDir, "06-cancel-refund", logs);
        assert(lastAppliedSequence(cancelledArtifact) > cancelSequenceBefore, `cancel did not apply: ${lastCommand(cancelledArtifact)}`);
        assert.equal(jobLine(cancelledArtifact, secondJobId), null, "cancelled job must disappear from the rendered queue");
        assert.equal(balanceOnHand(cancelledArtifact), expectedCredits, `exact ${stage} refund should yield ${expectedCredits} Credits`);
        result.stages.refund = {
            job_id: secondJobId,
            stage,
            starting_credits: creditsBeforeSecond,
            charge: 100,
            refund: expectedRefund,
            final_credits: expectedCredits,
            passed: true,
        };

        const harvester = cancelledArtifact.debug.units.find((unit) =>
            unit.content_id === "base:harvester" && Math.abs(unit.pos[0] - fixtureWorldX(12, FIXTURE_X_OFFSET)) < 1e-3 && Math.abs(unit.pos[1] - 8) < 1e-3
        );
        const harvesterMesh = harvester && cancelledArtifact.scene.units.find((unit) => unit.unit_id === harvester.unit_id);
        assert(harvester && harvesterMesh, "owner-0 Harvester is absent from actual received/rendered state");
        await clickRenderedEntity(page, cancelledArtifact.scene, harvesterMesh);
        await waitForSnapshot(page, (snapshot) =>
            hasSingleDebugSelection(snapshot, harvester.unit_id) &&
            visibleTexts(snapshot).some((text) => text.startsWith("Gather:")),
        5000, "Harvester selection and rebuilt Gather row");
        const selectedHarvesterArtifact = await captureStage(page, outDir, "07-selected-harvester", logs);
        assert(visibleTexts(selectedHarvesterArtifact).includes(`Selected: Harvester · ${harvester.unit_id}`));
        assert.equal(
            lastAppliedSequence(selectedHarvesterArtifact),
            lastAppliedSequence(cancelledArtifact),
            "selecting a Harvester must not advance the applied command sequence",
        );
        assert.equal(
            lastCommand(selectedHarvesterArtifact),
            lastCommand(cancelledArtifact),
            "selecting a Harvester must leave the prior rendered command result unchanged",
        );

        const node = selectedHarvesterArtifact.scene.nodes.find((candidate) =>
            Math.abs(candidate.translation[0] - fixtureWorldX(24, FIXTURE_X_OFFSET)) < 1e-3 && Math.abs(candidate.translation[2] - 8) < 1e-3
        );
        assert(node, "fixture ore node is absent from actual rendered scene meshes");
        let recoveryArtifact;
        if (targetCaseId === "E9-browser-base-economy") {
            const beforeGather = await liveSnapshot(page);
            assert(hasSingleDebugSelection(beforeGather, harvester.unit_id),
                "selected Harvester must remain selected before the economy Gather command");
            const freshNode = beforeGather.scene.nodes.find((candidate) => candidate.node_id === node.node_id);
            assert(freshNode, "fixture ore node is absent from the fresh rendered scene");
            const sequenceBeforeGather = lastAppliedSequence(beforeGather);
            assert(Number.isInteger(sequenceBeforeGather), "economy Gather requires a known applied command sequence");
            const gatherStarted = await pressRenderedEntityUntil(page, beforeGather.scene, freshNode, (snapshot) => {
                const gatherStatus = visibleTexts(snapshot).find((text) => text.startsWith("Gather:"));
                return lastAppliedSequence(snapshot) === sequenceBeforeGather + 1 &&
                    gatherStatus !== undefined && !/^Gather:\s*idle\b/i.test(gatherStatus);
            }, 5000, "one applied Gather command with a non-idle rendered order", { button: "right" });
            assert.equal(lastAppliedSequence(gatherStarted), sequenceBeforeGather + 1,
                "the economy Gather click must apply exactly one fresh command");
        } else {
            recoveryArtifact = await runBrowserCase("E9-browser-base-recovery", () => runBaseRecoveryProof(
                page, selectedHarvesterArtifact.scene, harvester, node, outDir, logs, result, captureStage,
            ));
        }
        const gatherStartArtifact = await captureStage(page, outDir, "11-gather-command-after-recovery", logs);
        assert(lastAppliedSequence(gatherStartArtifact) !== null, `Gather click did not apply: ${lastCommand(gatherStartArtifact)}`);
        assert(visibleTexts(gatherStartArtifact).some((text) => text.startsWith("Gather:")), "actual panel has no gather order status");
        const gathered = await waitForSnapshot(page, (snapshot) => {
            const texts = visibleTexts(snapshot);
            const positiveOre = texts.some((text) => /^Ore: [1-9]\d* on hand/.test(text));
            const depletedNode = texts.some((text) => /^Ore Deposit · (?:[1-9]\d{0,2}) remaining/.test(text));
            return positiveOre && depletedNode;
        }, 15000, "positive received extraction into the Harvester inventory");
        const gatherPositiveArtifact = await captureStage(page, outDir, "12-gather-positive", logs);
        if (recoveryArtifact) {
            assert(gatherPositiveArtifact.debug.replicated_world.tick >= recoveryArtifact.debug.replicated_world.tick);
        }
        const gatherTexts = visibleTexts(gatherPositiveArtifact);
        assert(gatherTexts.some((text) => /^Ore: [1-9]\d* on hand/.test(text)), "visible Harvester inventory did not receive ore");
        assert(gatherTexts.some((text) => /^Ore Deposit · (?:[1-9]\d{0,2}) remaining/.test(text)), "visible node amount did not decrease");
        assert(lastAppliedSequence(gatherPositiveArtifact) !== null, "Gather did not remain the latest applied client command");
        result.stages.gather = {
            harvester_id: harvester.unit_id,
            node_id: node.node_id,
            status: gatherTexts.find((text) => text.startsWith("Gather:")),
            positive_inventory_line: gatherTexts.find((text) => /^Ore: [1-9]\d* on hand/.test(text)),
            reduced_node_line: gatherTexts.find((text) => /^Ore Deposit · (?:[1-9]\d{0,2}) remaining/.test(text)),
            passed: true,
        };
        });
        if (!targetCaseId) {
            await runBrowserCase("E9-browser-base-public-convergence", async () =>
                runSecondOwnerConvergenceSmoke(page, pageAddress, packageName, outDir, logs, result, captureStage));
        }
        result.stages.browser_logs = logs;
        fs.writeFileSync(path.join(outDir, "result.json"), JSON.stringify(result, null, 2) + "\n");
    } catch (error) {
        if (browserCaseRun.targetCaseId && !browserCaseRun.terminal.has(browserCaseRun.targetCaseId)) {
            const caseId = browserCaseRun.targetCaseId;
            if (!browserCaseRun.started.has(caseId)) {
                console.log(`M2_CASE_START ${caseId}`);
                browserCaseRun.started.add(caseId);
            }
            const timeout = error instanceof BrowserCaseTimeoutError;
            browserCaseTerminal(timeout ? "TIMEOUT" : "INFRA_FAIL", caseId, error.message || String(error));
        }
        result.errors.push(error.stack || String(error));
        if (diagnostic) diagnostic.events.push({ at: timestamp(), kind: "caught-error", message: String(error) });
        result.stages.browser_logs = logs;
        if (diagnostic) result.diagnostic = { events: diagnostic.events, gpu_events: diagnostic.gpuEvents };
        try {
            await captureStage(page, outDir, `failure-${result.errors.length}`, logs);
        } catch (captureError) {
            result.errors.push(`artifact capture failed: ${captureError.stack || captureError}`);
        }
        if (diagnostic) result.diagnostic = { events: diagnostic.events, gpu_events: diagnostic.gpuEvents };
        fs.writeFileSync(path.join(outDir, "result.json"), JSON.stringify(result, null, 2) + "\n");
        throw error;
    } finally {
        if (diagnostic) diagnostic.events.push({ at: timestamp(), kind: "browser-close-start" });
        await finish();
        emitBrowserCaseSummary();
        browserCaseRun = null;
    }
}

if (require.main === module) {
    runBaseUiSmoke(process.argv.slice(2)).catch((error) => {
        console.error(error.stack || error);
        process.exitCode = 1;
    });
}
