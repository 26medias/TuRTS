"use strict";

// F10 M3 browser acceptance. Every action uses a Bevy-rendered control and Playwright's
// pointer path. Replica state and command receipts are read-only proof from turts_debug.
const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const {
    readBridge,
    waitForSnapshot,
    captureStage,
    clickRenderedEntity,
    clickRenderedUiAction,
    visibleTexts,
} = require("./m2.js");

function portSectionHasCapacity(lines, port, capacity) {
    const start = lines.indexOf(`Inventory · ${port}`);
    if (start < 0) return false;
    const nextPort = lines.findIndex((line, index) => index > start && line.startsWith("Inventory · "));
    const end = nextPort < 0 ? lines.length : nextPort;
    return lines.slice(start + 1, end).some((line) =>
        /^Capacity: \d+\/\d+$/.test(line) && line.endsWith(`/${capacity}`));
}

function commandProof(snapshot, seq) {
    const results = snapshot.debug?.command_results ?? [];
    return {
        acked: results.some((result) => result.seq === seq && Number.isInteger(result.ack_tick)),
        applied: results.some((result) => result.seq === seq && Number.isInteger(result.applied_tick)),
    };
}

async function m3_panel_actions_send_commands(page, outDir, logs = []) {
    const joined = await waitForSnapshot(page, (snapshot) =>
        snapshot.debug?.phase?.startsWith("Joined") &&
        snapshot.scene?.cameras_3d === 1 &&
        snapshot.debug?.units?.some((unit) => unit.content_id.startsWith("forge:")),
    30000, "forge joined in a real WebGPU browser context");
    assert.equal(joined.error, null);
    const initial = await captureStage(page, outDir, "01-forge-joined", logs);
    const world = initial.debug.replicated_world;
    const press = world.units.find((unit) => unit.content_id === "forge:press" && unit.owner_slot === 0);
    const pressMesh = press && initial.scene.units.find((unit) => unit.unit_id === press.unit_id);
    assert(press && pressMesh, "owner machine is absent from actual rendered state");
    await clickRenderedEntity(page, initial.scene, pressMesh);
    const pressSelected = await waitForSnapshot(page, (snapshot) => {
        const texts = visibleTexts(snapshot);
        return snapshot.debug?.selection === press.unit_id &&
            texts.includes("Inventory · Main") &&
            texts.includes("Inventory · Output") &&
            portSectionHasCapacity(texts, "Main", 6) &&
            portSectionHasCapacity(texts, "Output", 4);
    }, 5000, "actual rendered press panel shows Main and Output inventory capacity");
    const pressArtifact = await captureStage(page, outDir, "02-press-two-port-inventory", logs);
    const pressLines = visibleTexts(pressArtifact);
    assert(pressLines.includes("Inventory · Main") && pressLines.includes("Inventory · Output") &&
        portSectionHasCapacity(pressLines, "Main", 6) &&
        portSectionHasCapacity(pressLines, "Output", 4),
        `actual press panel omitted a labelled port or its catalog capacity: ${pressLines.join("\n")}`);

    const lab = pressSelected.debug.replicated_world.units.find((unit) => unit.content_id === "forge:lab" && unit.owner_slot === 0);
    const labMesh = lab && pressArtifact.scene.units.find((unit) => unit.unit_id === lab.unit_id);
    assert(lab && labMesh, "owner research station is absent from actual rendered state");
    await clickRenderedEntity(page, pressArtifact.scene, labMesh);
    const readyLab = await waitForSnapshot(page, (snapshot) => {
        const current = snapshot.debug?.replicated_world;
        const inventory = current?.economy?.inventories?.find((row) => row.unit === lab.unit_id && row.port === "main");
        const shells = inventory?.items?.find((item) => item.resource === "forge:shell");
        const control = (snapshot.ui ?? []).find((row) => row.visible && row.enabled && row.action?.type === "produce" && row.action.producer === lab.unit_id && row.action.recipe === "forge:hardened_shells");
        return snapshot.debug?.selection === lab.unit_id && shells?.on_hand - shells?.reserved >= 10 && Boolean(control);
    }, 120000, "replica inventory and visible enabled research control");
    const labSelected = await captureStage(page, outDir, "03-lab-research-ready", logs);
    const beforeProduce = Math.max(0, ...(readyLab.debug.command_results ?? []).map((result) => result.seq));
    const produce = await clickRenderedUiAction(page, labSelected.scene, "produce", (action, text) =>
        action.producer === lab.unit_id && action.recipe === "forge:hardened_shells" && text === "Research Hardened Shells");
    const produced = await waitForSnapshot(page, (snapshot) => {
        const receipt = (snapshot.debug?.command_results ?? []).find((result) => result.seq > beforeProduce && Number.isInteger(result.applied_tick));
        const queue = snapshot.debug?.replicated_world?.economy?.queues?.find((row) => row.producer === lab.unit_id);
        const proof = receipt && commandProof(snapshot, receipt.seq);
        return Boolean(receipt && proof.acked && proof.applied && queue?.jobs?.some((job) => job.recipe === "forge:hardened_shells"));
    }, 10000, "research command Acked, Applied and present in replicated queue");
    assert(produce.action?.producer === lab.unit_id);
    const producedArtifact = await captureStage(page, outDir, "04-research-applied", logs);
    const produceReceipt = produced.debug.command_results.filter((result) => result.seq > beforeProduce).at(-1);

    await clickRenderedUiAction(page, producedArtifact.scene, "set_enabled", (action, text) =>
        action.unit === lab.unit_id && action.enabled === false && text === "Disable unit");
    const disabled = await waitForSnapshot(page, (snapshot) => {
        const receipt = (snapshot.debug?.command_results ?? []).find((result) => result.seq > produceReceipt.seq && Number.isInteger(result.applied_tick));
        const queue = snapshot.debug?.replicated_world?.economy?.queues?.find((row) => row.producer === lab.unit_id);
        const proof = receipt && commandProof(snapshot, receipt.seq);
        return Boolean(receipt && proof.acked && proof.applied && queue?.enabled === false);
    }, 10000, "SetEnabled control Acked and Applied with disabled replica state");
    const disabledArtifact = await captureStage(page, outDir, "05-lab-disabled", logs);
    const disableReceipt = disabled.debug.command_results.filter((result) => result.seq > produceReceipt.seq).at(-1);

    const belt = disabled.debug.replicated_world.belts[0];
    assert(belt, "replica has no belt piece to remove");
    const beltUnit = disabled.debug.replicated_world.units.find((unit) => unit.unit_id === belt.piece);
    const beltMesh = beltUnit && disabledArtifact.scene.units.find((unit) => unit.unit_id === belt.piece);
    assert(beltUnit && beltMesh, "belt piece has no actual rendered mesh");
    await clickRenderedEntity(page, disabledArtifact.scene, beltMesh);
    const beltSelected = await waitForSnapshot(page, (snapshot) => snapshot.debug?.selection === belt.piece, 5000, "belt piece selected from rendered scene");
    const beltSelectedArtifact = await captureStage(page, outDir, "06-belt-selected", logs);
    await clickRenderedUiAction(page, beltSelectedArtifact.scene, "remove", (action, text) =>
        action.unit === belt.piece && text === "Remove unit");
    const removed = await waitForSnapshot(page, (snapshot) => {
        const receipt = (snapshot.debug?.command_results ?? []).find((result) => result.seq > disableReceipt.seq && Number.isInteger(result.applied_tick));
        const stillPresent = snapshot.debug?.replicated_world?.units?.some((unit) => unit.unit_id === belt.piece);
        const proof = receipt && commandProof(snapshot, receipt.seq);
        return Boolean(receipt && proof.acked && proof.applied && !stillPresent);
    }, 10000, "Remove control Acked and Applied with belt removed from replica");
    const removedArtifact = await captureStage(page, outDir, "07-belt-removed", logs);
    return {
        browser: await page.evaluate(() => navigator.userAgent),
        actions: {
            produce: { action: produce.action, receipt: produceReceipt },
            set_enabled: { unit: lab.unit_id, receipt: disableReceipt, disabled: true },
            remove: { unit: belt.piece, receipt: removed.debug.command_results.filter((result) => result.seq > disableReceipt.seq).at(-1), removed: true },
        },
        visible_lines: visibleTexts(removedArtifact),
        machine_inventory_panel: { unit_id: press.unit_id, main_visible: true, output_visible: true },
        captures: ["01-forge-joined", "02-press-two-port-inventory", "03-lab-research-ready", "04-research-applied", "05-lab-disabled", "06-belt-selected", "07-belt-removed"],
    };
}

async function main(argv) {
    const [pageUrl, serverUrl, wsUrl, outDir] = argv;
    assert(pageUrl && serverUrl && wsUrl && outDir, "usage: node m3.js <page-url> <server-url> <ws-url> <artifact-dir>");
    fs.mkdirSync(outDir, { recursive: true });
    const { chromium } = require("playwright-core");
    const headless = process.env.M2_HEADLESS !== "false";
    const chromeArgs = process.env.M2_CHROME_ARGS_JSON
        ? JSON.parse(process.env.M2_CHROME_ARGS_JSON)
        : ["--enable-unsafe-webgpu", "--use-webgpu-adapter=swiftshader"];
    const ignoreDefaultArgs = process.env.M2_IGNORE_DEFAULT_ARGS_JSON
        ? JSON.parse(process.env.M2_IGNORE_DEFAULT_ARGS_JSON)
        : undefined;
    const browser = await chromium.launch({
        executablePath: process.env.CHROME_BIN || "/usr/bin/google-chrome",
        headless,
        args: chromeArgs,
        ...(ignoreDefaultArgs ? { ignoreDefaultArgs } : {}),
    });
    let page;
    const logs = [];
    try {
        const context = await browser.newContext({ viewport: { width: 1280, height: 720 }, deviceScaleFactor: 1 });
        page = await context.newPage();
        page.on("console", (message) => logs.push({ type: message.type(), text: message.text() }));
        page.on("pageerror", (error) => logs.push({ type: "pageerror", text: error.message }));
        page.on("requestfailed", (request) => logs.push({ type: "requestfailed", url: request.url(), error: request.failure()?.errorText ?? null }));
        page.on("request", (request) => {
            if (/lockfile/i.test(request.url())) logs.push({ type: "lockfile-request", method: request.method(), url: request.url() });
        });
        page.on("response", (response) => {
            if (/lockfile/i.test(response.url())) logs.push({
                type: "lockfile-response",
                url: response.url(),
                status: response.status(),
                cors: response.headers()["access-control-allow-origin"] ?? null,
            });
        });
        const pageAddress = new URL(pageUrl);
        pageAddress.searchParams.set("server", serverUrl);
        pageAddress.searchParams.set("ws", wsUrl);
        await page.goto(pageAddress.toString(), { waitUntil: "domcontentloaded", timeout: 30000 });
        const result = await m3_panel_actions_send_commands(page, outDir, logs);
        assert(!logs.some((entry) => entry.type === "pageerror"), `browser page error: ${JSON.stringify(logs)}`);
        fs.writeFileSync(path.join(outDir, "m3-result.json"), JSON.stringify({ result, logs }, null, 2) + "\n");
        console.log(`M3_BROWSER_RESULT ${JSON.stringify(result)}`);
    } catch (error) {
        let bridge = null;
        try {
            bridge = await page.evaluate(() => Object.fromEntries(
                ["turts_scene", "turts_ui", "turts_debug", "turts_ingest_config_status", "turts_error"]
                    .map((name) => [name, typeof window[name] === "function" ? window[name]() : (window[name] ?? null)]),
            ));
        } catch (bridgeError) {
            logs.push({ type: "diagnostic-error", text: bridgeError.message });
        }
        try { await page.screenshot({ path: path.join(outDir, "failure-full.png"), fullPage: true }); }
        catch (screenshotError) { logs.push({ type: "screenshot-error", text: screenshotError.message }); }
        fs.writeFileSync(path.join(outDir, "m3-failure.json"), JSON.stringify({ error: error.stack || String(error), bridge, logs }, null, 2) + "\n");
        throw error;
    } finally {
        await browser.close();
    }
}

if (require.main === module) {
    main(process.argv.slice(2)).catch((error) => {
        console.error(error.stack || error);
        process.exitCode = 1;
    });
}

module.exports = { commandProof, m3_panel_actions_send_commands };
