// turts-test-role: driver
"use strict";

const { decodePng } = require("./harness/png.js");
const { readTargets } = require("./harness/hud4-pixels.js");

async function drive(io, params) {
    await io.key("F1");
    await io.wait(200);

    let pixels = null;
    let targetSet = null;
    let firstImage = null;
    const screenshots = [];
    for (let attempt = 0; attempt < 80; attempt += 1) {
        firstImage = await io.screenshot();
        screenshots.push({ label: "pixel-recognition-attempt", ordinal: screenshots.length, point: null });
        pixels = decodePng(firstImage);
        targetSet = readTargets(pixels);
        if (targetSet) break;
        if (attempt < 79) await io.wait(100);
    }
    if (!targetSet) {
        if (firstImage) await io.save("hud4-recognition-failed.png", firstImage);
        if (pixels) {
            const diagnostic = {
                viewport: { width: pixels.width, height: pixels.height },
                component_candidates: require("./harness/hud4-pixels.js").components(pixels),
                material_pixel_count: pixels.rgba.length / 4,
                status: "diagnostic only; no scene/debug data was read by the driver",
            };
            await io.save("hud4-recognition-failed.json", Buffer.from(JSON.stringify(diagnostic, null, 2) + "\n"));
        }
        throw new Error("HUD4 pixel role recognition failed; Muster HUD4 silhouettes remain NOT TRACED until a real browser row runs");
    }

    const savePng = async (name, bytes) => io.save(name, bytes);
    await savePng("hud4-initial.png", firstImage);

    const targets = targetSet.targets;
    const ground = targetSet.ground;
    const pixelEvidence = {
        viewport: { width: pixels.width, height: pixels.height },
        method: "color-connected-component grouping by fixture-authored primitive silhouette and size; no scene/debug bridge",
        targets,
        ground,
        recognition_status: "unverified until actual HUD4 browser execution; driver labels do not establish game entity identity",
    };
    await io.save("hud4-pixel-targets.json", Buffer.from(JSON.stringify(pixelEvidence, null, 2) + "\n"));

    async function capture(label, point = null) {
        await io.wait(100);
        const bytes = await io.screenshot();
        const ordinal = screenshots.length;
        screenshots.push({ label, ordinal, point });
        await savePng(`hud4-${String(ordinal).padStart(2, "0")}-${label}.png`, bytes);
    }

    // Establish actual selections by clicking each detected silhouette. The oracle checks the
    // selected ID, public content ID, and owner slot from the captured debug checkpoint.
    await io.click(targets.attacker.x, targets.attacker.y, "left");
    await capture("select-attacker", targets.attacker);
    await io.click(targets.worker.x, targets.worker.y, "left");
    await capture("select-worker", targets.worker);
    await io.click(targets.foreign.x, targets.foreign.y, "left");
    await capture("select-foreign", targets.foreign);

    // Hover each target with a relevant real selection already present.
    await io.click(targets.attacker.x, targets.attacker.y, "left");
    await io.move(targets.foreign.x, targets.foreign.y);
    await capture("hover-foreign-attack", targets.foreign);

    await io.click(targets.worker.x, targets.worker.y, "left");
    await io.move(targets.node.x, targets.node.y);
    await capture("hover-node-gather", targets.node);

    await io.click(targets.attacker.x, targets.attacker.y, "left");
    await io.key("KeyA");
    await io.move(targets.node.x, targets.node.y);
    await capture("attack-mode-invalid-node", targets.node);

    await io.key("Escape");
    await io.wait(100);
    await io.key("KeyM");
    await io.move(ground.x, ground.y);
    await capture("move-mode-ground-point", ground);

    // Cancel target mode and clear selection with a real ground click. This proves the
    // authored default cursor mapping against the actual unselected ground context.
    await io.key("Escape");
    await io.click(ground.x, ground.y, "left");
    await io.move(ground.x, ground.y);
    await capture("default-ground", ground);

    await io.save("hud4-screenshot-steps.json", Buffer.from(JSON.stringify({ screenshots }, null, 2) + "\n"));
}

module.exports = { drive };
