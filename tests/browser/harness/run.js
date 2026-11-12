// turts-test-role: harness
"use strict";

const fs = require("node:fs/promises");
const crypto = require("node:crypto");
const path = require("node:path");
const { createDriverIo } = require("./driver-io.js");
const { createView } = require("./view.js");
const { prepareSh1Lifecycle } = require("./sh1-lifecycle.js");
const { createSceneCheckpoints } = require("./scene-checkpoints.js");
const { planFor, runM2Fixture } = require("./m2-ingest.js");
const { installContextMenuObserver } = require("./contextmenu-observer.js");
const { installBoundedRafHold } = require("./bounded-raf.js");
const { installDeviceAudit, collectDeviceAudit } = require("./device-audit-descriptor.js");
const { trackNewPageDocumentEpoch } = require("./document-epoch.js");
const { installIoEventObserver } = require("./io-event-observer.js");
const { per1FixtureFor, collectPer1FixtureReceipts } = require("./per1-preboot-fixture.js");

const P1_REFERENCE_SHA256 = "f41d55729fa45f291d36d6a780d9aeb021242d61703dba39eed38e78c16684c1";

function deepFreeze(value) {
    if (value && typeof value === "object" && !Object.isFrozen(value)) {
        Object.freeze(value);
        for (const key of Object.keys(value)) deepFreeze(value[key]);
    }
    return value;
}

function parseArgs(args) {
    if (args.length < 4) throw new Error("usage: run.js <url> <out-dir> <driver.js> <oracle.js> --params <params.json> --viewport <viewport>");
    const [url, outDir, driverPath, oraclePath, ...flags] = args;
    let paramsPath = "";
    let viewport = "";
    let windows = null;
    for (let index = 0; index < flags.length;) {
        const flag = flags[index++];
        if (flag === "--params" && flags[index]) paramsPath = flags[index++];
        else if (flag === "--viewport" && flags[index]) viewport = flags[index++];
        else if (flag === "--windows") {
            if (windows !== null || !flags[index] || !/^[1-9]\d*$/.test(flags[index]) || !Number.isSafeInteger(Number(flags[index]))) {
                throw new Error("--windows requires a positive integer");
            }
            windows = Number(flags[index++]);
        }
        else throw new Error(`unknown or incomplete argument: ${flag}`);
    }
    if (!paramsPath || !viewport) throw new Error("both --params and --viewport are required");
    const match = /^(\d+)x(\d+)(?:@([0-9]+(?:\.[0-9]+)?))?$/.exec(viewport);
    if (!match) throw new Error(`invalid viewport: ${viewport}`);
    return { url, outDir, driverPath, oraclePath, paramsPath, width: Number(match[1]), height: Number(match[2]), deviceScaleFactor: Number(match[3] || 1), windows };
}

// PT1 (MC task 21, ruling C386): the oracle judges any player of the io contract from harness
// records only. While a window joins, record the load screen (phase, load rows, focus overlay)
// every 150 ms, so the load steps need no screenshot that a player chose.
async function recordStartup(page, windowNumber, observations, waitForJoin) {
    let joined = false;
    const sampler = (async () => {
        while (!joined && observations.length < 2000) {
            try { observations.push({ window: windowNumber, wall_time_ms: Date.now(), ...(await page.evaluate(loadObservation)) }); }
            catch { /* the page is between documents */ }
            await page.waitForTimeout(150);
        }
    })();
    try { await waitForJoin(); }
    finally { joined = true; await sampler; }
    observations.push({ window: windowNumber, wall_time_ms: Date.now(), ...(await page.evaluate(loadObservation)) });
    // The layout labels that the web entry gives the client (crates/turts_client_web/src/lib.rs:187-235),
    // so an oracle renders the same key labels (client_shell/bindings_editor.rs BindingLabels::from_layout_entries).
    const keyboardLayout = await page.evaluate(async () => {
        try { return Object.fromEntries(await navigator.keyboard.getLayoutMap()); } catch { return null; }
    });
    observations.push({ window: windowNumber, wall_time_ms: Date.now(), keyboard_layout: keyboardLayout });
}

async function loadObservation() {
    let debug = null;
    // Ask for one record (design D 9, row DBG1); an older page publishes in each frame.
    try {
        debug = typeof window.turts_debug !== "function" ? null : JSON.parse(
            await (window.turts_debug_request?.() ?? window.turts_debug()));
    } catch { debug = null; }
    const rows = (debug?.ui_snapshot ?? []).filter((row) => String(row.id ?? "").startsWith("shell.load") || row.kind === "focus_overlay");
    return { phase: debug?.phase ?? null, rows };
}

// A long row (PT1) declares more checkpoints; the default stays 256 (harness/scene-checkpoints.js).
function checkpointLimit(params) {
    const limit = params.checkpoint_limit ?? 256;
    if (!Number.isSafeInteger(limit) || limit < 1 || limit > 2048) throw new Error("params.checkpoint_limit must be an integer from 1 to 2048");
    return limit;
}

function launchArgsForDpr(args, dpr) {
    if (!Array.isArray(args) || args.some((arg) => typeof arg !== "string")) {
        throw new Error("Chrome launch arguments must be an array of strings");
    }
    if (!Number.isFinite(dpr) || dpr <= 0) throw new Error("effective device scale factor must be positive and finite");
    const flag = "--force-device-scale-factor";
    const found = [];
    for (let index = 0; index < args.length; index += 1) {
        const arg = args[index];
        if (arg === flag) {
            const value = args[index + 1];
            if (!value || value.startsWith("--")) throw new Error(`${flag} needs one positive numeric value`);
            found.push({ index, value });
            index += 1;
        } else if (arg.startsWith(`${flag}=`)) {
            found.push({ index, value: arg.slice(flag.length + 1) });
        }
    }
    if (found.length > 1) throw new Error(`conflicting explicit ${flag} arguments`);
    if (found.length === 1) {
        const supplied = Number(found[0].value);
        if (!Number.isFinite(supplied) || supplied <= 0) throw new Error(`${flag} needs one positive numeric value`);
        if (Math.abs(supplied - dpr) > 1e-9) {
            throw new Error(`explicit ${flag}=${found[0].value} conflicts with effective DPR ${dpr}`);
        }
        return [...args];
    }
    return [...args, `${flag}=${dpr}`];
}

async function run(options) {
    const loadedParams = JSON.parse(await fs.readFile(options.paramsPath, "utf8"));
    const windows = options.windows ?? loadedParams.windows ?? 1;
    if (!Number.isSafeInteger(windows) || windows < 1) throw new Error("params.windows must be a positive integer");
    const params = deepFreeze({ ...loadedParams, windows });
    const deviceAuditEnabled = params.task16?.gpu_device_audit === true;
    const startupRafProbeEnabled = params.task16?.startup_raf_probe === true;
    if (["KM2", "CAM2"].includes(params.row)) await attachAcceptedP1Reference(options.outDir);
    const effectiveDpr = Number(params.device_scale_factor ?? options.deviceScaleFactor);
    const baseChromeArgs = JSON.parse(process.env.MC_CHROME_ARGS_JSON || '["--enable-gpu","--use-angle=vulkan","--enable-features=Vulkan","--enable-unsafe-webgpu"]');
    const chromeArgs = launchArgsForDpr(baseChromeArgs, effectiveDpr);
    const headless = process.env.MC_BROWSER_HEADED !== "1";
    const boundedRafSource = await fs.readFile(path.join(__dirname, "bounded-raf.js"));
    await fs.writeFile(path.join(options.outDir, "browser-profile.json"), JSON.stringify({
        args: chromeArgs,
        device_scale_factor: effectiveDpr,
        headless,
        viewport_css: [options.width, options.height],
        bounded_raf_hook_sha256: crypto.createHash("sha256").update(boundedRafSource).digest("hex"),
    }, null, 2) + "\n");
    const { chromium } = require("playwright-core");
    const driver = require(path.resolve(options.driverPath));
    const oracle = require(path.resolve(options.oraclePath));
    if (typeof driver.drive !== "function") throw new Error("driver must export drive(io, params)");
    if (typeof oracle.judge !== "function") throw new Error("oracle must export judge(view, params)");
    const browser = await chromium.launch({
        executablePath: process.env.CHROME_BIN || undefined,
        headless,
        args: chromeArgs,
        ignoreDefaultArgs: JSON.parse(process.env.MC_CHROME_IGNORE_DEFAULT_ARGS_JSON || '["--enable-unsafe-swiftshader"]'),
    });
    const pages = [];
    const contexts = [];
    const deviceAuditKeys = [];
    const deviceAuditSessions = [];
    const deviceAuditAttachTasks = [];
    const deviceAuditRecords = [];
    const inputLog = [];
    const documentEpochs = [];
    const browserDiagnostics = createBrowserDiagnostics(documentEpochs);
    let initialSetupComplete = false;
    let runFailure = null;
    let nextDocumentEpoch = 0;
    const allocateDocumentEpoch = () => {
        nextDocumentEpoch = Number.isSafeInteger(nextDocumentEpoch + 1) ? nextDocumentEpoch + 1 : 0;
        if (nextDocumentEpoch < 1) throw new Error("Task16 document epoch counter exhausted");
        return nextDocumentEpoch;
    };
    const serverDir = options.outDir;
    const sh1NavigationArchive = [];
    const sh1StorageAudit = [];
    let sh1Lifecycle = null;
    let sh1Early = null;
    let prebootFixture = null;
    let prebootOrigin = null;
    const prebootReceipts = [];
    const startupObservations = [];
    try {
        const windows = Number(params.windows || 1);
        prebootFixture = ["PER1", "SEL4"].includes(params.row) ? per1FixtureFor(params.task16?.fixture) : null;
        prebootOrigin = prebootFixture ? new URL(options.url).origin : null;
        const slotFixture = params.task16?.fixture === "slot1-authentic-copy-v1";
        if (slotFixture && windows !== 3) throw new Error("SLOT1 fixture requires exactly three simultaneous windows");
        const slotIds = slotFixture ? makeSlotFixtureIds() : null;
        const slotStartup = [];
        for (let n = 0; n < windows; n += 1) {
            if (slotFixture && startupRafProbeEnabled && n === 2) {
                await runSlot1StartupRafProbe({ pages, documentEpochs, outDir: options.outDir,
                    boundedRafSha256: crypto.createHash("sha256").update(boundedRafSource).digest("hex") });
            }
            const context = await browser.newContext({ viewport: { width: options.width, height: options.height }, deviceScaleFactor: effectiveDpr });
            contexts.push(context);
            const deviceAuditKey = deviceAuditEnabled
                ? `__task16_gpu_audit_${crypto.randomBytes(12).toString("hex")}`
                : null;
            deviceAuditKeys[n] = deviceAuditKey;
            if (deviceAuditEnabled) {
                await context.addInitScript(installDeviceAudit, { key: deviceAuditKey, limit: 8 });
                context.on("page", (auditPage) => {
                    const task = attachDeviceAudit(auditPage, context, n, deviceAuditKey, deviceAuditSessions);
                    deviceAuditAttachTasks.push(task);
                });
            }
            await context.addInitScript(installBoundedRafHold);
            await context.addInitScript(installIoEventObserver);
            await context.addInitScript(installContextMenuObserver);
            if (["KM1", "PER1", "SEL4", "SH1", "PT1"].includes(params.row)) {
                await context.addInitScript(installTask16StorageAudit, {
                    fixture: prebootFixture, targetOrigin: prebootOrigin, window: n + 1,
                });
            }
            if (slotFixture) await installOneUseSessionSeed(context, slotIds[n]);
            const page = await context.newPage();
            if (deviceAuditEnabled) await Promise.all(deviceAuditAttachTasks.splice(0));
            documentEpochs[n] = 0;
            trackMainDocumentEpoch(page, n, documentEpochs, allocateDocumentEpoch);
            observeBrowserDiagnostics(page, n, browserDiagnostics);
            await page.mouse.move(options.width / 2, options.height / 2);
            pages.push(page);
            if (params.row === "SH1" && params.task16?.fixture === "sh1-real-acquisition-v1") {
                if (windows !== 1) throw new Error("SH1 lifecycle requires exactly one window");
                const checkpoints = createSceneCheckpoints(pages, 256, documentEpochs);
                sh1Early = { checkpoints, bundle: createDriverIo(pages, {
                    outputDir: options.outDir, inputLog,
                    initialCursor: { x: options.width / 2, y: options.height / 2 },
                    afterScreenshot: checkpoints.record, documentEpochs, navigationArchive: sh1NavigationArchive,
                    storageAudit: sh1StorageAudit,
                    screenshotScale: "device",
                }) };
                sh1Lifecycle = await prepareSh1Lifecycle({ context, params, outDir: options.outDir,
                    epoch: () => documentEpochs[0], navigationArchive: sh1NavigationArchive, page });
            }
            await page.goto(options.url, { waitUntil: "domcontentloaded" });
            await page.bringToFront();
            if (prebootFixture) {
                const receipt = await page.evaluate(() => {
                    try {
                        return {
                            ...JSON.parse(sessionStorage.getItem("__task16_preboot_receipt_v1") || "null"),
                            observed_document_url: document.URL,
                            observed_time_origin: performance.timeOrigin,
                            fixture_audit_operations: (window.__task16_storage_audit || []).filter((entry) => entry.provenance === "fixture_seed"),
                        };
                    } catch (error) { return { status: "fixture-read-error", error: String(error?.message || error),
                        observed_document_url: document.URL, observed_time_origin: performance.timeOrigin,
                        fixture_audit_operations: (window.__task16_storage_audit || []).filter((entry) => entry.provenance === "fixture_seed") }; }
                });
                prebootReceipts.push({ ...receipt, window: n + 1, document_epoch: documentEpochs[n] ?? 0 });
                if (receipt?.status !== "complete") {
                    throw new Error(`HARNESS preboot fixture ${prebootFixture.id} setup failed in window ${n + 1}`);
                }
            }
            if (sh1Lifecycle) await sh1Lifecycle.acquire(page, driver, sh1Early.bundle.io);
            let startup;
            if (slotFixture) {
                try {
                    startup = await waitForTask16Role(page, n === 2 ? "unclaimed-third" : "slot-owner");
                } catch (error) {
                    if (startupRafProbeEnabled && n === 2) {
                        let observation;
                        try { observation = await readTask16StartupObservation(page); }
                        catch (readError) { observation = { status: "NOT TRACED", reason: clipDiagnosticText(readError?.message || readError) }; }
                        slotStartup.push({ role: "unclaimed-third", window: 3, document_epoch: documentEpochs[n],
                            wait_outcome: "timeout-or-setup-error", wait_error: clipDiagnosticText(error?.message || error), observation });
                        await writePrivateJson(path.join(options.outDir, "slot1-startup-results.json"), slotStartup);
                    }
                    throw error;
                }
                startup = { ...startup, window: n + 1, document_epoch: documentEpochs[n] };
                slotStartup.push(startup);
                await writePrivateJson(path.join(options.outDir, "slot1-startup-results.json"), slotStartup);
                if (n < 2 && !(startup.phase.startsWith("Joined") && startup.last_frame && startup.cameras_3d === 1)) {
                    const outcome = oracle.judgeStartupOutcome({ row: "SLOT1", clause: "owners-stable", role: n === 0 ? "A" : "B", startup });
                    if (outcome.status === "semantic-failure") {
                        console.error(`SEMANTIC FAIL ${outcome.clause}: ${outcome.message}`);
                        process.exitCode = 1;
                        return;
                    }
                    throw new Error(`SLOT1 owner setup did not establish its required product precondition: ${outcome.message}`);
                }
            } else if (params.record_startup === true) {
                await recordStartup(page, n + 1, startupObservations, () => waitForJoinedRole(page));
            } else {
                await waitForJoinedRole(page);
            }
            const bootError = await page.evaluate(() => window.turts_error || null);
            if (bootError) throw new Error(`browser client failed: ${bootError}`);
            if (!sh1Lifecycle) await page.evaluate(() => {
                window.__turts_io_seq = 0;
                if (typeof window.turts_debug === "function" && typeof window.turts_debug.frames_clear === "function") window.turts_debug.frames_clear();
            });
        }
        initialSetupComplete = true;
        if (params.record_startup === true) {
            await fs.writeFile(path.join(options.outDir, "startup-observations.json"), JSON.stringify(startupObservations) + "\n");
        }
        await pages[0].bringToFront();
        await pages[0].waitForTimeout(100);
        if (slotFixture && startupRafProbeEnabled) {
            await writePrivateJson(path.join(options.outDir, "slot1-startup-raf-probe-completion.json"), {
                status: "startup-outcome-recorded-diagnostic-only",
                startup: slotStartup,
                no_driver_or_oracle: true,
                no_copy_or_reload_phase: true,
            });
            return;
        }
        if (slotFixture) {
            await writePrivateJson(path.join(options.outDir, "slot1-fixture-private.json"), {
                fixture: "slot1-authentic-copy-v1", simultaneous_window_limit: 3,
                lifetime_documents: pages.map((_, index) => ({ handle: index + 1, epoch: documentEpochs[index], alias: ["A", "B", "C"][index] })),
                identity_aliases: { A: slotIds[0], B: slotIds[1], C: slotIds[2] }, phases: [], startup: slotStartup,
            });
        }
        if (!sh1Lifecycle) for (const page of pages) await page.evaluate(() => { window.__turts_io_actual_events = []; window.__turts_io_downstream_events = []; window.__turts_io_contextmenu_events = []; window.__turts_io_contextmenu_overflow = false; window.__turts_io_active_call = null; window.__turts_io_seq = null; window.__turts_io_dispatching = false; window.__turts_io_expected_event = null; });
        const checkpoints = sh1Early?.checkpoints || (params.capture_scene_checkpoints === true ? createSceneCheckpoints(pages, checkpointLimit(params), documentEpochs) : null);
        const storageAudit = params.row === "SH1" ? sh1StorageAudit :
            (["KM1", "PER1", "SEL4", "PT1"].includes(params.row) ? [] : null);
        const storageInitialState = storageAudit ? await Promise.all(pages.map(async (page, index) => {
            const snapshot = await page.evaluate(async () => {
                const entries = (storage) => Object.fromEntries(Array.from({ length: storage.length }, (_, offset) => {
                    const key = storage.key(offset);
                    return [key, storage.getItem(key)];
                }));
                // Ask for one record (design D 9, row DBG1); an older page publishes in each frame.
                const debug = typeof window.turts_debug !== "function" ? null : JSON.parse(
                    await (window.turts_debug_request?.() ?? window.turts_debug()));
                return { sessionStorage: entries(sessionStorage), localStorage: entries(localStorage), debug, document_url: document.URL, origin: location.origin, document_time_origin: performance.timeOrigin };
            });
            return { window: index + 1, document_epoch: documentEpochs[index] ?? 0, ...snapshot };
        })) : null;
        const { io, state, flushStorageAudit } = sh1Early?.bundle || createDriverIo(pages, {
            outputDir: options.outDir, inputLog,
            initialCursor: { x: options.width / 2, y: options.height / 2 },
            afterScreenshot: checkpoints?.record,
            screenshotScale: params.row === "SH1" ? "device" : "css",
            documentEpochs,
            storageAudit,
            retainScreenshots: params.retain_screenshots === true,
            observeReloadLoad: params.record_startup === true,
            beforeReload: deviceAuditEnabled ? async (page, windowIndex) => {
                await collectDeviceAuditForPage(page, windowIndex, documentEpochs, deviceAuditSessions, deviceAuditRecords);
            } : null,
        });
        const view = createView(pages, inputLog, serverDir, state);
        try {
            if (slotFixture) {
                await runSlot1Fixture({ io, driver, params, pages, contexts, slotIds, documentEpochs, allocateDocumentEpoch, inputLog, checkpoints, outDir: options.outDir,
                    auditDevicePage: deviceAuditEnabled ? async (page, index) => {
                        await Promise.all(deviceAuditAttachTasks.splice(0));
                        await collectDeviceAuditForPage(page, index, documentEpochs, deviceAuditSessions, deviceAuditRecords);
                    } : null });
            } else if (sh1Lifecycle) {
                for (const phase of ["load-settings", "conflicts", "settings-controls"]) {
                    await driver.drive(io, { ...params, fixture_phase: phase }, {
                        beforeAction: async (action, index) => {
                            if (phase !== "load-settings" || index !== 6) return;
                            if (action.call !== "key" || action.code !== "F10") throw new Error("SH1 scale checkpoint must precede the first menu action");
                            const observed=checkpoints.snapshots().at(-1);
                            if (!observed?.debug?.phase?.startsWith("Joined") || observed.document_epoch !== documentEpochs[0]) {
                                throw new Error("SH1 initial scale check lacks its actual current Joined screenshot checkpoint");
                            }
                            const { decodePng } = require("./png.js");
                            const { judgeSh1ScaleBounds } = require("../sh1-settings-oracle.js");
                            judgeSh1ScaleBounds({checkpoint: observed}, decodePng(await fs.readFile(path.join(options.outDir,
                                "SH1-load-settings-world-ready.png"))), 1);
                        },
                    });
                }
                await sh1Lifecycle.disconnect(pages[0], driver, io);
                await waitForJoinedRole(pages[0]);
                await driver.drive(io, { ...params, fixture_phase: "connection-rejoined" });
            } else if (params.row === "M2" && planFor(params)) {
                await runM2Fixture({ io, driver, params, pages, outDir: options.outDir });
            } else {
                await driver.drive(io, params);
            }
        } finally {
            const finalStorageState = storageAudit ? await flushStorageAudit() : null;
            if (checkpoints) await fs.writeFile(path.join(options.outDir, "scene-checkpoints.json"), JSON.stringify(checkpoints.snapshots()) + "\n");
            await fs.writeFile(path.join(options.outDir, "input-log.json"), JSON.stringify(inputLog) + "\n");
            const ambient = await Promise.all(pages.map(async (page) => {
                try { return await page.evaluate(() => ({ ambient: window.__turts_io_ambient_events || [], downstream: window.__turts_io_downstream_events || [] })); }
                catch (error) { return { ambient: [{ kind: "capture-error", message: String(error) }], downstream: [] }; }
            }));
            await fs.writeFile(path.join(options.outDir, "ambient-events.json"), JSON.stringify(ambient) + "\n");
            if (storageAudit) {
                await writePrivateJson(path.join(options.outDir, "storage-audit.json"), {
                    entries: storageAudit, initial_state: storageInitialState, final_state: finalStorageState,
                });
            }
        }
        if (prebootFixture) {
            const report = await collectPer1FixtureReceipts({ pages, observed: prebootReceipts,
                documentEpochs, targetOrigin: prebootOrigin, fixture: prebootFixture,
                outDir: options.outDir, runError: null });
            if (report.status !== "complete") throw new Error(`HARNESS preboot fixture ${prebootFixture.id} receipt invalid: ${report.errors.join("; ")}`);
        }
        // The boot check (above) runs once; a client error later in the run also stops it, as the
        // legacy t7.js poll did at each read (MC task 21, M4). Checkpoints check it at each screenshot.
        for (const [index, page] of pages.entries()) {
            const clientError = await page.evaluate(() => window.turts_error || null);
            if (clientError) throw new Error(`browser client failed in window ${index + 1}: ${clientError}`);
        }
        await oracle.judge(view, params);
        console.log(`PASS ${path.basename(options.oraclePath)}`);
    } catch (error) {
        runFailure = error;
        await writePrivateJson(path.join(options.outDir, "harness-failure.json"), {
            message: String(error?.message || error), stack: String(error?.stack || error),
        });
        const message = error && error.message ? error.message : String(error);
        const semantic = error?.task16_semantic;
        if (semantic && typeof semantic.row === "string" && typeof semantic.clause === "string") {
            console.error(`SEMANTIC FAIL ${semantic.row}.${semantic.clause}: ${message}`);
        } else {
            console.error(`FAIL ${path.basename(options.oraclePath)}: ${message}`);
        }
        process.exitCode = 1;
    } finally {
        if (sh1Lifecycle) {
            await sh1Lifecycle.cleanup(pages[0]);
            await fs.writeFile(path.join(options.outDir, "scene-checkpoints.json"), JSON.stringify(sh1Early.checkpoints.snapshots()) + "\n");
            await fs.writeFile(path.join(options.outDir, "input-log.json"), JSON.stringify(inputLog) + "\n");
        }
        if (prebootFixture) {
            try {
                const report = await collectPer1FixtureReceipts({ pages, observed: prebootReceipts,
                    documentEpochs, targetOrigin: prebootOrigin, fixture: prebootFixture,
                    outDir: options.outDir, runError: runFailure });
                if (report.status !== "complete") {
                    console.error(`FAIL setup: preboot fixture ${prebootFixture.id} receipt status is ${report.status}: ${report.errors.join("; ")}`);
                    process.exitCode = 1;
                    if (!runFailure) runFailure = new Error(`HARNESS preboot fixture ${prebootFixture.id} receipt invalid`);
                }
            } catch (error) {
                console.error(`FAIL fixture receipt writer: ${error?.message || String(error)}`);
                process.exitCode = 1;
            }
        }
        if (runFailure && !initialSetupComplete) {
            try {
                await captureStartupFailurePages(pages, documentEpochs, options.outDir, runFailure);
            } catch (error) {
                console.error(`FAIL startup diagnostic capture: ${error.message || String(error)}`);
                process.exitCode = 1;
            }
        }
        try {
            await writePrivateJson(path.join(options.outDir, "browser-diagnostics.json"), browserDiagnostics.snapshot());
        } catch (error) {
            console.error(`FAIL browser diagnostic flush: ${error.message || String(error)}`);
            process.exitCode = 1;
        }
        if (deviceAuditEnabled) {
            try {
                await Promise.all(deviceAuditAttachTasks.splice(0));
                for (const entry of deviceAuditSessions) {
                    if (entry.page.isClosed()) continue;
                    await collectDeviceAuditForPage(entry.page, entry.windowIndex, documentEpochs, deviceAuditSessions, deviceAuditRecords);
                }
                await writePrivateJson(path.join(options.outDir, "gpu-device-audit.json"), {
                    source: "harness-only-cdp-promise-state",
                    enabled_by_explicit_param: true,
                    events: deviceAuditRecords,
                });
            } catch (error) {
                console.error(`FAIL GPU device audit flush: ${error.message || String(error)}`);
                process.exitCode = 1;
            }
        }
        await browser.close();
    }
}

if (require.main === module) run(parseArgs(process.argv.slice(2))).catch((error) => {
    console.error(`FAIL harness: ${error.message || String(error)}`);
    process.exitCode = 1;
});

module.exports = { checkpointLimit, deepFreeze, loadObservation, recordStartup, launchArgsForDpr, parseArgs, trackMainDocumentEpoch,
    waitForJoinedRole, waitForTask16Role };

async function attachDeviceAudit(page, context, windowIndex, key, sessions) {
    try {
        const cdp = await context.newCDPSession(page);
        sessions.push({ page, cdp, windowIndex, key });
    } catch (error) {
        sessions.push({ page, cdp: null, windowIndex, key, setupError: clipDiagnosticText(error?.message || error) });
    }
}

async function collectDeviceAuditForPage(page, windowIndex, documentEpochs, sessions, records) {
    const entry = [...sessions].reverse().find((item) => item.page === page);
    const startedAt = Date.now();
    let result;
    if (!entry || !entry.cdp) {
        result = { status: "NOT TRACED", reason: entry?.setupError || "CDP session unavailable" };
    } else {
        const objectGroup = `task16-gpu-audit-${crypto.randomBytes(8).toString("hex")}`;
        try {
            result = await withTimeout(
                collectDeviceAudit(entry.cdp, entry.key, objectGroup),
                2500,
                "GPU device audit read timed out",
            );
        } catch (error) {
            result = { status: "NOT TRACED", reason: clipDiagnosticText(error?.message || error) };
        }
    }
    records.push({
        window: windowIndex + 1,
        document_epoch: Number.isSafeInteger(documentEpochs[windowIndex]) && documentEpochs[windowIndex] > 0
            ? documentEpochs[windowIndex] : null,
        host_read_started_ms: startedAt,
        host_read_completed_ms: Date.now(),
        result,
    });
}

function clipDiagnosticText(value, limit = 2048) {
    const text = String(value);
    return text.length <= limit ? text : `${text.slice(0, limit)}…`;
}

async function withTimeout(promise, ms, message) {
    let timer;
    try {
        return await Promise.race([promise, new Promise((_, reject) => {
            timer = setTimeout(() => reject(new Error(message)), ms);
        })]);
    } finally {
        clearTimeout(timer);
    }
}

function createBrowserDiagnostics(documentEpochs, maxEvents = 512) {
    const events = [];
    const counts = {};
    const dropped = {};
    return {
        record(kind, windowIndex, fields) {
            counts[kind] = (counts[kind] || 0) + 1;
            if (events.length >= maxEvents) {
                dropped[kind] = (dropped[kind] || 0) + 1;
                return;
            }
            events.push({
                kind,
                window: windowIndex + 1,
                document_epoch: Number.isSafeInteger(documentEpochs[windowIndex]) && documentEpochs[windowIndex] > 0
                    ? documentEpochs[windowIndex] : null,
                host_time_ms: Date.now(),
                ...fields,
            });
        },
        snapshot() {
            return { events, counts_by_kind: counts, dropped_by_kind: dropped,
                overflow: Object.keys(dropped).length > 0, max_events: maxEvents };
        },
    };
}

function observeBrowserDiagnostics(page, windowIndex, diagnostics) {
    page.on("console", (message) => {
        const location = message.location();
        diagnostics.record("console", windowIndex, {
            level: clipDiagnosticText(message.type(), 32),
            location: { url: clipDiagnosticText(location.url || "", 512), line: location.lineNumber, column: location.columnNumber },
            text: clipDiagnosticText(message.text()),
        });
    });
    page.on("pageerror", (error) => {
        diagnostics.record("pageerror", windowIndex, {
            level: "error",
            location: { url: "", line: null, column: null },
            text: clipDiagnosticText(error?.stack || error?.message || error),
        });
    });
    page.on("requestfailed", (request) => {
        diagnostics.record("requestfailed", windowIndex, {
            level: "error",
            location: { url: clipDiagnosticText(request.url(), 512), line: null, column: null },
            text: clipDiagnosticText(request.failure()?.errorText || "request failed"),
        });
    });
}

async function captureStartupFailurePages(pages, documentEpochs, outputDir, failure) {
    const records = [];
    for (let index = 0; index < pages.length; index += 1) {
        const page = pages[index];
        const record = { window: index + 1,
            document_epoch: Number.isSafeInteger(documentEpochs[index]) && documentEpochs[index] > 0 ? documentEpochs[index] : null };
        try {
            record.page_state = await withTimeout(page.evaluate(async () => {
                // The client builds the debug state only when asked (design D 9, row DBG1). Ask for
                // one record, but never wait for more than 1.2 s: a failed page may run no frame. The
                // record that the last request served is read then.
                if (typeof window.turts_debug_request === "function") {
                    await Promise.race([window.turts_debug_request().catch(() => null),
                        new Promise((resolve) => setTimeout(resolve, 1200))]);
                }
                const bridge = (name) => {
                    if (typeof window[name] !== "function") return { status: "unavailable" };
                    try {
                        const value = JSON.parse(window[name]());
                        if (name === "turts_debug") {
                            const frame = value.last_frame;
                            return { status: "ok", value: {
                                phase: value.phase ?? null,
                                player_id_decimal: value.player_id_decimal ?? null,
                                match_id_decimal: value.match_id_decimal ?? null,
                                slot: value.slot ?? null,
                                reconnect_required: value.reconnect_required ?? null,
                                rejoin: value.rejoin ?? null,
                                last_frame: frame ? { lo_tick: frame.lo_tick ?? null, hi_tick: frame.hi_tick ?? null,
                                    latest_tick: frame.latest_tick ?? null, fraction: frame.fraction ?? null,
                                    applied_seq: frame.applied_seq ?? null,
                                    selection: frame.selection ?? null, pending_order_id: frame.pending_order_id ?? null } : null,
                                notices: Array.isArray(value.notices) ? value.notices.slice(0, 8) : [],
                                ui_snapshot: Array.isArray(value.ui_snapshot) ? value.ui_snapshot.slice(0, 32).map((row) => ({
                                    kind: row.kind ?? null, id: row.id ?? null, visible: row.visible ?? null,
                                    rect: row.rect ?? null, text: typeof row.text === "string" ? row.text.slice(0, 256) : null,
                                })) : [],
                            } };
                        }
                        return { status: "ok", value: {
                            cameras_3d: value.cameras_3d ?? null,
                            coordinate_space: value.coordinate_space ?? null,
                            ui_rect_coordinate_space: value.ui_rect_coordinate_space ?? null,
                            window_logical_size: value.window_logical_size ?? null,
                            viewport_logical_rect: value.viewport_logical_rect ?? null,
                            viewport_physical_rect: value.viewport_physical_rect ?? null,
                            scale_factor: value.scale_factor ?? null,
                            render_adapter: value.render_adapter ?? null,
                            nodes: Array.isArray(value.nodes) ? value.nodes.slice(0, 32).map((node) => ({
                                id: node.id ?? null, rect: node.rect ?? null, visible: node.visible ?? null,
                            })) : [],
                            widgets: Array.isArray(value.widgets) ? value.widgets.slice(0, 32).map((widget) => ({
                                id: widget.id ?? null, rect: widget.rect ?? null, visible: widget.visible ?? null,
                                text: typeof widget.text === "string" ? widget.text.slice(0, 256) : null,
                            })) : [],
                        } };
                    }
                    catch (error) { return { status: "read-error", error: String(error).slice(0, 512) }; }
                };
                return {
                    document_url: document.URL,
                    document_time_origin: performance.timeOrigin,
                    ready_state: document.readyState,
                    visibility_state: document.visibilityState,
                    turts_error: typeof window.turts_error === "string" ? window.turts_error.slice(0, 2048) : null,
                    canvas_count: document.querySelectorAll("canvas").length,
                    debug: bridge("turts_debug"),
                    scene: bridge("turts_scene"),
                };
            }), 2500, "page diagnostic read timed out");
        } catch (error) {
            record.page_state = { status: "capture-error", error: clipDiagnosticText(error?.message || error) };
        }
        try {
            const png = await page.screenshot({ type: "png", timeout: 2500 });
            const name = `startup-failure-window-${index + 1}-epoch-${record.document_epoch ?? "unknown"}.png`;
            const target = path.join(outputDir, name);
            await fs.writeFile(target, png, { mode: 0o600 });
            await fs.chmod(target, 0o600);
            record.actual_screenshot = { file: name, byte_length: png.length,
                sha256: crypto.createHash("sha256").update(png).digest("hex") };
        } catch (error) {
            record.screenshot_error = clipDiagnosticText(error?.message || error);
        }
        records.push(record);
    }
    await writePrivateJson(path.join(outputDir, "startup-failure-pages.json"), {
        failure: clipDiagnosticText(failure?.stack || failure?.message || failure),
        pages: records,
    });
}

function trackMainDocumentEpoch(page, windowIndex, documentEpochs, allocateEpoch) {
    page.on("framenavigated", (frame) => {
        if (frame === page.mainFrame()) documentEpochs[windowIndex] = allocateEpoch();
    });
}

function makeSlotFixtureIds() {
    const a = (9007199254740993n + BigInt(crypto.randomBytes(6).readUIntBE(0, 6))).toString();
    const ids = new Set([a]);
    while (ids.size < 3) ids.add(crypto.randomBytes(8).readBigUInt64BE().toString());
    return [...ids];
}

async function installOneUseSessionSeed(context, playerId) {
    const seed = JSON.stringify(playerId);
    await context.addInitScript(`(() => {
        const marker = "__codex_task16_seeded";
        if (sessionStorage.getItem(marker) === "1") return;
        sessionStorage.setItem("turts.v1.player_id", ${seed});
        sessionStorage.setItem(marker, "1");
    })();`);
}

function installTask16StorageAudit({ fixture = null, targetOrigin = null, window: windowIndex = null } = {}) {
    window.__task16_storage_audit = [];
    window.__task16_storage_audit_sent = 0;
    const record = (storage, operation, key, value = null) => {
        window.__task16_storage_audit.push({
            source: "storage_api",
            storage,
            operation,
            key: key === null ? null : String(key),
            value: value === null ? null : String(value),
            wall_time_ms: Date.now(),
            performance_time_ms: performance.now(),
            provenance: window.__task16_storage_fixture_active === true ? "fixture_seed" : "product",
        });
    };
    const set = Storage.prototype.setItem;
    const remove = Storage.prototype.removeItem;
    const clear = Storage.prototype.clear;
    Storage.prototype.setItem = function (key, value) {
        const result = Reflect.apply(set, this, [key, value]);
        record(this === window.sessionStorage ? "sessionStorage" : "localStorage", "set", key, value);
        return result;
    };
    Storage.prototype.removeItem = function (key) {
        const result = Reflect.apply(remove, this, [key]);
        record(this === window.sessionStorage ? "sessionStorage" : "localStorage", "remove", key);
        return result;
    };
    Storage.prototype.clear = function () {
        const storage = this === window.sessionStorage ? "sessionStorage" : "localStorage";
        const result = Reflect.apply(clear, this, []);
        record(storage, "clear", null);
        return result;
    };
    // This is the same ordered initializer as the audit hook. Crucially, do not touch
    // Storage in about:blank: origin is checked before sessionStorage/localStorage access.
    if (!fixture || !targetOrigin || location.origin === "null" || location.origin !== targetOrigin) return;
    const marker = "__task16_preboot_seeded_v1";
    const receiptKey = "__task16_preboot_receipt_v1";
    try {
        if (sessionStorage.getItem(marker) === "1") return;
    } catch (error) {
        window.__task16_storage_fixture_error = String(error?.message || error);
        return;
    }
    const receipt = {
        fixture: fixture.id,
        status: "running",
        target_origin: targetOrigin,
        initial_url: document.URL,
        initial_time_origin: performance.timeOrigin,
        initial_wall_time_ms: Date.now(),
        initial_performance_ms: performance.now(),
        window: windowIndex,
        seeds: [],
    };
    window.__task16_storage_fixture_active = true;
    try {
        for (const seed of fixture.seeds) {
            const storage = seed.storage === "sessionStorage" ? sessionStorage : localStorage;
            const entry = {
                storage: seed.storage, key: seed.key, value: seed.value, role: seed.role,
                operation_started_wall_time_ms: Date.now(),
                operation_started_performance_ms: performance.now(),
                set_returned: false, readback: null, readback_matches: false,
            };
            receipt.seeds.push(entry);
            try {
                storage.setItem(seed.key, seed.value);
                entry.set_returned = true;
                entry.readback = storage.getItem(seed.key);
                entry.readback_matches = entry.readback === seed.value;
            } catch (error) {
                entry.error = String(error?.message || error);
                throw error;
            } finally {
                entry.operation_finished_wall_time_ms = Date.now();
                entry.operation_finished_performance_ms = performance.now();
            }
            if (!entry.set_returned || !entry.readback_matches) throw new Error(`fixture set/readback failed for ${seed.key}`);
        }
        receipt.status = "complete";
    } catch (error) {
        receipt.status = "failed";
        receipt.error = String(error?.message || error);
    }
    try {
        sessionStorage.setItem(marker, "1");
        sessionStorage.setItem(receiptKey, JSON.stringify(receipt));
    } catch (error) {
        receipt.status = "failed";
        receipt.receipt_write_error = String(error?.message || error);
        try { window.__task16_storage_fixture_error = receipt.receipt_write_error; } catch {}
    } finally {
        window.__task16_storage_fixture_active = false;
    }
}

async function waitForJoinedRole(page) {
    // Playwright does not await a predicate, so this one is sync (tests/browser/harness/wait-predicate.test.js).
    // Each poll starts a request for a record (design D 9, row DBG1) and reads the record that
    // the last request published; an older page publishes in each frame.
    await page.waitForFunction(() => {
        if (window.turts_error) return true;
        if (typeof window.turts_debug !== "function" || typeof window.turts_scene !== "function") return false;
        try {
            window.turts_debug_request?.().catch(() => {});
            const debug = JSON.parse(window.turts_debug());
            const scene = JSON.parse(window.turts_scene());
            return typeof debug.phase === "string" && debug.phase.startsWith("Joined") && Boolean(debug.last_frame) && scene.cameras_3d === 1;
        } catch { return false; }
    }, null, { timeout: 30000 });
}

async function waitForTask16Role(page, role) {
    // A sync predicate, as in waitForJoinedRole: it starts a request and reads the last record.
    await page.waitForFunction((expectedRole) => {
        if (window.turts_error) return true;
        if (typeof window.turts_debug !== "function" || typeof window.turts_scene !== "function") return false;
        try {
            window.turts_debug_request?.().catch(() => {});
            const debug = JSON.parse(window.turts_debug());
            const scene = JSON.parse(window.turts_scene());
            const terminal = typeof debug.phase === "string" && (debug.phase.startsWith("Failed") ||
                (debug.phase.startsWith("Joined") && Boolean(debug.last_frame) && scene?.cameras_3d === 1));
            const firstCopiedHello = expectedRole === "copied-live-duplicate" && Number.isInteger(debug.rejoin?.hellos_sent) && debug.rejoin.hellos_sent > 0;
            return terminal || firstCopiedHello;
        } catch { return false; }
    }, role, { timeout: 30000 }).catch((error) => { throw new Error(`Task16 ${role} readiness failed: ${error.message}`); });
    const status = await page.evaluate(async () => {
        if (window.turts_error) return { error: window.turts_error };
        const debug = JSON.parse(await (window.turts_debug_request?.() ?? window.turts_debug()));
        let scene = null;
        try { scene = JSON.parse(window.turts_scene()); } catch {}
        return { phase: debug.phase, player_id_decimal: debug.player_id_decimal ?? null, match_id_decimal: debug.match_id_decimal ?? null,
            rejoin: debug.rejoin ?? null, last_frame: debug.last_frame ?? null, cameras_3d: scene?.cameras_3d ?? null,
            debug, scene };
    });
    if (status.error) throw new Error(`Task16 ${role} setup error: ${status.error}`);
    if (!status.phase) throw new Error(`Task16 ${role} has no typed startup result`);
    return status;
}

async function readTask16StartupObservation(page) {
    return page.evaluate(async () => {
        if (typeof window.turts_debug !== "function" || typeof window.turts_scene !== "function") {
            return { status: "NOT TRACED", reason: "actual typed debug/scene bridge unavailable" };
        }
        const debug = JSON.parse(await (window.turts_debug_request?.() ?? window.turts_debug()));
        const scene = JSON.parse(window.turts_scene());
        const frames = [...(debug.frames?.recent || []), ...(debug.frames?.retained_input || [])];
        const frameIndices = frames.map((frame) => frame.frame_index).filter(Number.isSafeInteger);
        return {
            status: "actual-typed-startup-observation",
            phase: typeof debug.phase === "string" ? debug.phase : null,
            match_id_decimal: typeof debug.match_id_decimal === "string" ? debug.match_id_decimal : null,
            slot: Number.isSafeInteger(debug.slot) ? debug.slot : null,
            last_frame_present: debug.last_frame !== null && debug.last_frame !== undefined,
            last_frame_latest_tick: Number.isSafeInteger(debug.last_frame?.latest_tick) ? debug.last_frame.latest_tick : null,
            frame_indices: frameIndices,
            cameras_3d: Number.isSafeInteger(scene.cameras_3d) ? scene.cameras_3d : null,
        };
    });
}

async function runSlot1StartupRafProbe({ pages, documentEpochs, outDir, boundedRafSha256 }) {
    if (!Array.isArray(pages) || pages.length !== 2) throw new Error("SLOT1 startup RAF probe requires exactly the joined A/B pages before C creation");
    const records = [];
    const errors = [];
    const warnings = [];
    const before = await Promise.all(pages.map((page) => readTask16StartupObservation(page)));
    const beforeOwners = await readPrivateOwnerCheckpointSnapshot(outDir);
    const begun = [];
    let holdStartedHostMs = null;
    let holdTimerCompletedHostMs = null;
    let releaseCompletedHostMs = null;
    try {
        const beginResults = await Promise.allSettled(pages.map(async (page, index) => {
            const hostStartedMs = Date.now();
            const result = await page.evaluate(() => {
                if (typeof window.__turts_begin_bounded_input !== "function") throw new Error("existing bounded RAF begin hook is unavailable");
                window.__turts_begin_bounded_input();
                return { begun: true };
            });
            const record = {
                window: index + 1,
                document_epoch: documentEpochs[index] ?? null,
                begin_host_started_ms: hostStartedMs,
                begin_host_completed_ms: Date.now(),
                result,
            };
            begun.push({ page, index, record });
            return record;
        }));
        for (const result of beginResults) {
            if (result.status === "fulfilled") records.push(result.value);
            else errors.push(`begin: ${clipDiagnosticText(result.reason?.message || result.reason)}`);
        }
        if (beginResults.every((result) => result.status === "fulfilled")) {
            holdStartedHostMs = Date.now();
            await new Promise((resolve) => setTimeout(resolve, 2000));
            holdTimerCompletedHostMs = Date.now();
        }
    } catch (error) {
        errors.push(`hold: ${clipDiagnosticText(error?.message || error)}`);
    } finally {
        const releaseResults = await Promise.allSettled(begun.map(async ({ page, index, record }) => {
            const releaseStartedMs = Date.now();
            const result = await page.evaluate(() => {
                if (typeof window.__turts_end_bounded_input !== "function") throw new Error("existing bounded RAF end hook is unavailable");
                return window.__turts_end_bounded_input();
            });
            return {
                window: index + 1,
                document_epoch: documentEpochs[index] ?? null,
                release_host_started_ms: releaseStartedMs,
                release_host_completed_ms: Date.now(),
                held_elapsed_ms: releaseStartedMs - record.begin_host_completed_ms,
                held_callbacks: Number.isSafeInteger(result.held_callbacks) ? result.held_callbacks : null,
                released: result.released === true,
                release_batch: Number.isSafeInteger(result.release_batch) ? result.release_batch : null,
            };
        }));
        for (const result of releaseResults) {
            if (result.status === "fulfilled") records.push(result.value);
            else errors.push(`release: ${clipDiagnosticText(result.reason?.message || result.reason)}`);
        }
        releaseCompletedHostMs = Date.now();
        const after = await Promise.allSettled(pages.map((page) => readTask16StartupObservation(page)));
        let afterOwnerCheckpoint = null;
        try { afterOwnerCheckpoint = await waitForPrivateOwnerCheckpoint(outDir, releaseCompletedHostMs); }
        catch (error) { warnings.push(`owner_checkpoint: ${clipDiagnosticText(error?.message || error)}`); }
        const afterOwners = afterOwnerCheckpoint ? { status: "actual", checkpoint: afterOwnerCheckpoint }
            : await readPrivateOwnerCheckpointSnapshot(outDir);
        const pageRows = pages.map((_, index) => {
            const result = after[index];
            const value = result.status === "fulfilled" ? result.value : { status: "NOT TRACED", reason: clipDiagnosticText(result.reason?.message || result.reason) };
            const beforeIndices = before[index].frame_indices || [];
            const afterIndices = value.frame_indices || [];
            const beforeMax = beforeIndices.length ? Math.max(...beforeIndices) : null;
            const afterMax = afterIndices.length ? Math.max(...afterIndices) : null;
            return {
                window: index + 1,
                document_epoch: documentEpochs[index] ?? null,
                before: before[index],
                after: value,
                resumed_frame_observed: beforeMax !== null && afterMax !== null && afterMax > beforeMax,
            };
        });
        const beforeOwnerRow = beforeOwners.checkpoints.at(-1) || null;
        const afterOwnerRow = afterOwners.checkpoint || afterOwners.checkpoints?.at(-1) || null;
        const diagnostic = {
            status: errors.length ? "instrumentation-error-retained" : "diagnostic-only",
            source: "existing-bounded-raf-hooks",
            bounded_raf_hook_sha256: boundedRafSha256,
            requested_hold_ms: 2000,
            hold_started_host_ms: holdStartedHostMs,
            hold_timer_completed_host_ms: holdTimerCompletedHostMs,
            release_completed_host_ms: releaseCompletedHostMs,
            pages: pageRows,
            hold_release_records: records,
            server_owner_observation: {
                before_status: beforeOwners.status,
                before_checkpoint: beforeOwnerRow,
                after_status: afterOwnerCheckpoint ? "actual-checkpoint-after-release" : afterOwners.status,
                after_checkpoint: afterOwnerRow,
                unchanged_owner_payload: Boolean(beforeOwnerRow && afterOwnerRow &&
                    JSON.stringify(beforeOwnerRow.owners) === JSON.stringify(afterOwnerRow.owners)),
            },
            errors,
            warnings,
            no_product_or_acceptance_credit: true,
        };
        try { await writePrivateJson(path.join(outDir, "slot1-startup-raf-probe.json"), diagnostic); }
        catch (error) { errors.push(`diagnostic_write: ${clipDiagnosticText(error?.message || error)}`); }
    }
    if (errors.length) throw new Error(`SLOT1 startup RAF diagnostic did not complete cleanly: ${errors.join("; ")}`);
}

async function readPrivateOwnerCheckpointSnapshot(outDir) {
    const target = path.join(outDir, "server-stats.json.sessions.jsonl");
    let text;
    try { text = await fs.readFile(target, "utf8"); }
    catch (error) {
        return { status: error?.code === "ENOENT" ? "NOT TRACED: observer file unavailable" : `NOT TRACED: ${clipDiagnosticText(error?.message || error)}`, checkpoints: [] };
    }
    const lines = text.split(/\r?\n/).filter(Boolean);
    try {
        const records = lines.map((line) => JSON.parse(line));
        return { status: "actual", checkpoints: records.filter((record) => record.event === "owner_checkpoint") };
    } catch (error) {
        return { status: `NOT TRACED: ${clipDiagnosticText(error?.message || error)}`, checkpoints: [] };
    }
}

async function assertNoHeldModifiers(inputLog) {
    const held = new Set();
    for (const entry of inputLog) {
        for (const event of entry.actual_events || []) {
            if (!event.code || !["keydown", "keyup"].includes(event.type)) continue;
            const key = `${entry.window}:${event.code}`;
            if (event.type === "keydown") held.add(key);
            else held.delete(key);
        }
    }
    if (held.size) throw new Error(`SLOT1 phase boundary has held physical keys: ${[...held].join(", ")}`);
}

async function appendFixturePhase(outDir, phase) {
    const target = path.join(outDir, "slot1-fixture-private.json");
    const current = JSON.parse(await fs.readFile(target, "utf8"));
    if (phase.phase === "copy-refusal") {
        current.lifetime_documents.push({ handle: phase.replaced_handle, epoch: phase.new_document_epoch, alias: "A" });
        current.total_lifetime_document_count = current.lifetime_documents.length;
    }
    current.phases.push(phase);
    await writePrivateJson(target, current);
}

async function writePrivateJson(target, value) {
    await fs.writeFile(target, JSON.stringify(value, null, 2) + "\n", { mode: 0o600 });
    await fs.chmod(target, 0o600);
}

async function attachAcceptedP1Reference(outDir) {
    const source = path.resolve(__dirname, "../../../docs/research/mc/p1-keys.md");
    const bytes = await fs.readFile(source);
    const sha256 = crypto.createHash("sha256").update(bytes).digest("hex");
    if (sha256 !== P1_REFERENCE_SHA256) throw new Error(`accepted P1 reference hash changed: ${sha256}`);
    const target = path.join(outDir, "p1-keys.md");
    await fs.writeFile(target, bytes, { mode: 0o600 });
    await fs.chmod(target, 0o600);
    await writePrivateJson(path.join(outDir, "p1-attachment.json"), {
        source_path: "docs/research/mc/p1-keys.md",
        sha256,
        byte_length: bytes.length,
        attached_before_physical_run: true,
    });
}

async function runSlot1Fixture({ io, driver, params, pages, contexts, slotIds, documentEpochs, allocateDocumentEpoch, inputLog, checkpoints, outDir, auditDevicePage }) {
    const flush = async () => {
        if (checkpoints) await fs.writeFile(path.join(outDir, "scene-checkpoints.json"), JSON.stringify(checkpoints.snapshots()) + "\n");
        await fs.writeFile(path.join(outDir, "input-log.json"), JSON.stringify(inputLog) + "\n");
        const ambient = await Promise.all(pages.map(async (page) => {
            try { return await page.evaluate(() => ({ ambient: window.__turts_io_ambient_events || [], downstream: window.__turts_io_downstream_events || [] })); }
            catch (error) { return { ambient: [{ kind: "capture-error", message: String(error) }], downstream: [] }; }
        }));
        await fs.writeFile(path.join(outDir, "ambient-events.json"), JSON.stringify(ambient) + "\n");
    };
    await appendFixturePhase(outDir, { phase: "capacity", readiness: await Promise.all(pages.map((page, i) => waitForTask16Role(page, i === 2 ? "unclaimed-third" : "slot-owner"))) });
    await driver.drive(io, { ...params, fixture_phase: "capacity" });
    await flush();
    const capacityCapture = [...inputLog].reverse().find((entry) => entry.call === "screenshot" && entry.window === 3);
    if (!capacityCapture) throw new Error("SLOT1 capacity phase has no actual C refusal screenshot before its owner boundary");
    await waitForPrivateOwnerCheckpoint(outDir, capacityCapture.completed_wall_time_ms);
    await assertNoHeldModifiers(inputLog);
    if (auditDevicePage) await auditDevicePage(pages[2], 2);
    await pages[2].close();

    const opener = pages[0];
    const popupEpochTracker = (page) => {
        // Playwright exposes a new Page after navigation has started, so the first
        // main-frame navigation event can precede this tracker. Allocate for the
        // actual observed popup identity before tracking any later document changes.
        trackNewPageDocumentEpoch(page, 2, documentEpochs, allocateDocumentEpoch);
    };
    contexts[0].once("page", popupEpochTracker);
    const popupPromise = contexts[0].waitForEvent("page", { timeout: 5000 });
    await opener.evaluate(() => {
        window.__task16_open_copy = true;
        document.addEventListener("click", function openCopiedWindow(event) {
            if (!window.__task16_open_copy) return;
            window.__task16_open_copy = false;
            event.preventDefault();
            event.stopImmediatePropagation();
            window.open(location.href, "_blank");
        }, { capture: true, once: true });
    });
    const popupOpenStartedMs = Date.now();
    await opener.mouse.click(2, 2);
    const copied = await popupPromise;
    const popupOpenCompletedMs = Date.now();
    pages[2] = copied;
    const openerId = await opener.evaluate(() => sessionStorage.getItem("turts.v1.player_id"));
    const copiedId = await copied.evaluate(() => sessionStorage.getItem("turts.v1.player_id"));
    if (!openerId || openerId !== copiedId || openerId !== slotIds[0]) throw new Error("SLOT1 opener did not produce a browser-authentic sessionStorage copy");
    const openerMethod = typeof copied.opener === "function" ? await copied.opener() : null;
    if (openerMethod !== opener) throw new Error("SLOT1 copied page opener does not match live window 1");
    const cloneStatus = await waitForTask16Role(copied, "copied-live-duplicate");
    await appendFixturePhase(outDir, {
        phase: "copy-refusal", opener_handle: 1, replaced_handle: 3,
        popup_open_started_wall_ms: popupOpenStartedMs,
        popup_open_completed_wall_ms: popupOpenCompletedMs,
        old_document_epoch: JSON.parse(await fs.readFile(path.join(outDir, "slot1-fixture-private.json"), "utf8")).lifetime_documents.find((doc) => doc.handle === 3 && doc.alias === "C").epoch,
        new_document_epoch: documentEpochs[2],
        storage_copy_equal: true, browser_opener_copy: true,
        startup: { ...cloneStatus, window: 3, document_epoch: documentEpochs[2] },
    });
    await driver.drive(io, { ...params, fixture_phase: "copy-refusal" });
    await flush();
    const terminalCapture = [...inputLog].reverse().find((entry) => entry.call === "screenshot" && entry.window === 3);
    if (!terminalCapture) throw new Error("SLOT1 copied-tab phase has no actual terminal screenshot before its owner boundary");
    await waitForPrivateOwnerCheckpoint(outDir, terminalCapture.completed_wall_time_ms);
    await assertNoHeldModifiers(inputLog);
    if (auditDevicePage) await auditDevicePage(copied, 2);
    await copied.close();

    await driver.drive(io, { ...params, fixture_phase: "reload-rejoin" }, {
        beforeAction: async (action) => {
            if (action.call !== "reload") return;
            const joinedBeforeReload = [...inputLog].reverse().find((entry) => entry.call === "screenshot" && entry.window === 1);
            if (!joinedBeforeReload) throw new Error("SLOT1 original reload has no actual pre-reload joined capture");
            await waitForPrivateOwnerCheckpoint(outDir, joinedBeforeReload.completed_wall_time_ms);
        },
    });
    await flush();
    const rejoinedCapture = [...inputLog].reverse().find((entry) => entry.call === "screenshot" && entry.window === 1);
    if (!rejoinedCapture) throw new Error("SLOT1 rejoin phase has no final original-window screenshot");
    await waitForPrivateOwnerCheckpoint(outDir, rejoinedCapture.completed_wall_time_ms);
    await appendFixturePhase(outDir, { phase: "reload-rejoin", original_window: 1, original_player_id_retained: true });
    await appendFixturePhase(outDir, { phase: "complete", owners_windows_live: [1, 2], transport_close: "NOT TRACED until a real server event is correlated" });
}

async function waitForPrivateOwnerCheckpoint(outDir, afterWallMs) {
    if (!Number.isFinite(afterWallMs)) throw new Error("SLOT1 owner boundary has no captured host wall time");
    const observer = path.join(outDir, "server-stats.json.sessions.jsonl");
    const deadline = Date.now() + 2_500;
    while (Date.now() <= deadline) {
        let text;
        try { text = await fs.readFile(observer, "utf8"); }
        catch { await new Promise((resolve) => setTimeout(resolve, 50)); continue; }
        const lines = text.split(/\r?\n/).filter(Boolean);
        let records;
        try { records = lines.map((line) => JSON.parse(line)); }
        catch (error) { throw new Error(`SLOT1 private observer contains a partial record before its boundary: ${error.message}`); }
        const checkpoint = records.findLast((record) => record.event === "owner_checkpoint" &&
            record.host_unix_us >= afterWallMs * 1000);
        if (checkpoint) return checkpoint;
        await new Promise((resolve) => setTimeout(resolve, 50));
    }
    throw new Error("SLOT1 private observer produced no bounded owner checkpoint after the required capture");
}
