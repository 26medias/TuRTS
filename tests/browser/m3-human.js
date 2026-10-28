"use strict";

// Human-facing discovery regression: select named controls with real pointer gestures.
const assert = require("node:assert/strict");
const fs = require("node:fs");
const { chromium } = require("playwright-core");
const { waitForSnapshot, captureStage, clickRenderedUiAction, visibleTexts } = require("./m2.js");

async function main() {
    const [url, out] = process.argv.slice(2);
    assert(url && out, "usage: node m3-human.js <fresh-owner-url> <artifacts>");
    fs.mkdirSync(out, { recursive: true });
    const browser = await chromium.launch({ executablePath: process.env.CHROME_BIN || "/usr/bin/google-chrome", headless: false,
        args: ["--enable-gpu", "--use-angle=vulkan", "--enable-features=Vulkan", "--enable-unsafe-webgpu"],
        ignoreDefaultArgs: ["--enable-unsafe-swiftshader"] });
    const errors = [];
    let page;
    try {
        page = await browser.newPage({ viewport: { width: 1280, height: 720 } });
        page.on("pageerror", error => errors.push(error.message));
        await page.goto(url);
        const joined = await waitForSnapshot(page, s => s.debug?.phase?.startsWith("Joined { slot: 0") &&
            s.ui?.some(r => r.visible && r.text === "Select Proving Lab"), 30000, "named factory selection visible without knowing shapes");
        await captureStage(page, out, "01-names-and-start", []);
        const lab = joined.debug.replicated_world.units.find(u => u.content_id === "forge:lab");
        assert(lab);
        await clickRenderedUiAction(page, joined.scene, "select", action => action.unit === lab.unit_id);
        const selected = await waitForSnapshot(page, s => s.debug?.selection === lab.unit_id &&
            visibleTexts(s).some(t => t.includes("Research cost") && t.includes("10")), 5000, "named selector opens research and explains cost");
        assert.equal(selected.debug.command_results.length, joined.debug.command_results.length, "select does not send a command");
        assert(visibleTexts(selected).some(t => t.includes("stationary")));
        await captureStage(page, out, "02-lab-explains-action", []);
        const ready = await waitForSnapshot(page, s => s.ui?.some(r => r.visible && r.enabled && r.action?.type === "produce" &&
            r.action.producer === lab.unit_id), 120000, "research becomes affordable through the real factory");
        await clickRenderedUiAction(page, ready.scene, "produce", action => action.producer === lab.unit_id);
        const running = await waitForSnapshot(page, s => visibleTexts(s).some(t => t.startsWith("Research progress:")) &&
            s.debug.command_results.some(r => Number.isInteger(r.applied_tick)), 10000, "actual research progress and applied receipt");
        await captureStage(page, out, "03-research-progress", []);
        const complete = await waitForSnapshot(page, s => visibleTexts(s).includes("Research complete: Hardened Shells"),
            20000, "visible authoritative completion");
        await captureStage(page, out, "04-research-complete", []);
        const turret = complete.debug.replicated_world.units.find(u => u.content_id === "forge:turret");
        await clickRenderedUiAction(page, complete.scene, "select", a => a.unit === turret.unit_id);
        await waitForSnapshot(page, s => s.debug?.selection === turret.unit_id && visibleTexts(s).some(t => t.includes("Damage: 20")),
            5000, "named turret selector reveals upgraded damage");
        await captureStage(page, out, "05-upgraded-turret", []);
        assert.deepEqual(errors, []);
        fs.writeFileSync(`${out}/result.json`, JSON.stringify({ passed: true, lab: lab.unit_id, turret: turret.unit_id,
            applied: running.debug.command_results, errors }, null, 2));
        console.log("PASS: names -> select lab -> cost -> real research -> progress -> completion -> damage20");
    } catch (error) {
        if (page) await page.screenshot({ path: `${out}/failure.png` }).catch(() => {});
        throw error;
    } finally { await browser.close(); }
}
main().catch(error => { console.error(error); process.exitCode = 1; });
