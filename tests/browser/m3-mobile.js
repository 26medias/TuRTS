// turts-test-role: legacy-bridge
"use strict";
const assert = require("node:assert/strict");
const fs = require("node:fs");
const { chromium } = require("playwright-core");
const { waitForSnapshot, captureStage, clickRenderedUiAction, visibleTexts, projectGroundPoint, clickWindowLogicalPoint, hasSingleDebugSelection } = require("./m2.js");
async function main() {
    const [url, out] = process.argv.slice(2);
    assert(url && out, "usage: node m3-mobile.js <fresh-owner-url> <artifacts>");
    fs.mkdirSync(out, { recursive: true });
    const browser = await chromium.launch({ executablePath: "/usr/bin/google-chrome", headless: false,
        args: ["--enable-gpu", "--use-angle=vulkan", "--enable-features=Vulkan", "--enable-unsafe-webgpu"],
        ignoreDefaultArgs: ["--enable-unsafe-swiftshader"] });
    try {
        const context = await browser.newContext({ viewport: { width: 1280, height: 720 } });
        const owner = await context.newPage(); await owner.goto(url);
        await waitForSnapshot(owner, s => s.debug?.phase?.startsWith("Joined { slot: 0"), 30000, "factory owns first slot");
        const dronePage = await context.newPage(); await dronePage.goto(url); await dronePage.bringToFront();
        const joined = await waitForSnapshot(dronePage, s => s.debug?.phase?.startsWith("Joined { slot: 1") &&
            s.ui?.some(r => r.visible && r.action?.type === "select"), 30000, "second tab offers named mobile units");
        const drone = joined.debug.replicated_world.units.find(u => u.content_id === "forge:drone" && u.pos[0] > 25);
        assert(drone);
        assert(!joined.ui.some(r => r.visible && r.action?.type === "select" && r.text === "Select Proving Lab"), "no enemy factory selection control");
        await clickRenderedUiAction(dronePage, joined.scene, "select", a => a.unit === drone.unit_id);
        const selected = await waitForSnapshot(dronePage, s => hasSingleDebugSelection(s, drone.unit_id) &&
            visibleTexts(s).includes("Mobile: right-click the ground to move."), 5000, "selected drone explains movement");
        await captureStage(dronePage, out, "01-mobile-selected", []);
        await clickWindowLogicalPoint(dronePage, selected.scene, projectGroundPoint(selected.scene, [28, 6]), { button: "right" });
        const moved = await waitForSnapshot(dronePage, s => s.debug?.command_results.some(r => Number.isInteger(r.applied_tick)) &&
            s.debug.replicated_world.units.some(u => u.unit_id === drone.unit_id && u.pos[0] < 29.5), 10000, "right-click command applied and drone actually moves");
        await captureStage(dronePage, out, "02-mobile-moved", []);
        fs.writeFileSync(`${out}/result.json`, JSON.stringify({ passed: true, before: drone, after: moved.debug.replicated_world.units.find(u => u.unit_id === drone.unit_id), receipts: moved.debug.command_results }, null, 2));
        console.log("PASS: second-tab named drone -> visible controls -> real right-click movement; no enemy selectors");
    } finally { await browser.close(); }
}
main().catch(e => { console.error(e); process.exitCode = 1; });
