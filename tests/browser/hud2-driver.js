// turts-test-role: driver
"use strict";

const { decodePng } = require("./harness/png.js");
const { readOwnTarget, readSlotTargets, findOpenGround, findPaletteTargetNear } = require("./sel1-driver.js");
const { turnSoundOn } = require("./harness/sound-switch-pixels.js");

// HUD2 keeps its authored layout literal in the test. The driver captures pixels
// before it sends any HUD input, as required for pixel-only UI tests.
const SELECTION_PANEL = { x: 8, y: 8, width: 272, height: 180 };
const PANEL_FILL = [17, 24, 32];
const TOLERANCE = 28;
const GUIDE_CLOSE_AREA = { x: 1176, y: 12, width: 96, height: 48 };
const GUIDE_CLOSE_FILL = [230, 159, 0];
const RESEARCH_PANEL = { x: 8, y: 438, width: 272, height: 128 };
const RESEARCH_BUTTON_TARGET = { x: 144, y: 487 };
const RESEARCH_BUTTON_SAMPLES = [
    { x: 240, y: 487 }, { x: 250, y: 487 }, { x: 260, y: 487 },
    { x: 240, y: 496 }, { x: 250, y: 496 }, { x: 260, y: 496 },
];
const RESEARCH_BUTTON_ENABLED_FILL = [33, 66, 77];
const RESEARCH_BUTTON_DISABLED_FILL = [51, 51, 51];
const COMMAND_ROWS = { x: 1001, y: 255, width: 264, height: 228, maxItems: 9, rowHeight: 28, gap: 4 };
const QUEUE_CANCEL_TARGET = { x: 229, y: 658 };
const FORGE_MAP_TARGETS = [
    // Authored placements seed only a localized screenshot search. A target is usable only
    // when current slot-colour/shape pixels are observed at that projected footprint.
    { slot: 0, seed: { x: 703, y: 311 }, source: "forge:lab at (17,4)" },
    { slot: 0, seed: { x: 712, y: 416 }, source: "forge:turret at (17,12)" },
];

async function drive(io, params) {
    const initial = Buffer.from(await io.screenshot());
    await saveScreenshot(io, "hud2-initial", initial);

    if (params.row === "HUD2-MUSTER" || params.package === "muster") {
        await io.key("F1");
        await io.wait(200);
        const resourceLine = Buffer.from(await io.screenshot());
        await saveScreenshot(io, "hud2-muster-resource", resourceLine);
        return;
    }

    // Inspect the Controls list with physical wheel input inside its authored region.
    // The initial screenshot precedes this literal UI target; the driver reads no semantics.
    await io.move(850, 500);
    await io.wheel(1200);
    await io.wait(200);
    await saveScreenshot(io, "hud2-controls-bottom", Buffer.from(await io.screenshot()));
    await io.wheel(-1200);
    await io.wait(150);

    // The first join opens the mod guide. Close it through the physical F1 binding
    // before selecting a unit; a guide overlay must never make the panel baseline red.
    await io.key("F1");
    await io.wait(200);
    const f1Closed = Buffer.from(await io.screenshot());
    await saveScreenshot(io, "hud2-f1-closed", f1Closed);
    await io.key("F1");
    await io.wait(200);
    const guideReopened = Buffer.from(await io.screenshot());
    await saveScreenshot(io, "hud2-guide-reopened", guideReopened);
    const closeButton = findCloseButton(decodePng(guideReopened));
    await io.save("hud2-close-button-target.json", Buffer.from(JSON.stringify({
        area: GUIDE_CLOSE_AREA,
        fill: GUIDE_CLOSE_FILL,
        target: closeButton,
        method: "orange fill pixels in the fixed guide header region; no package, bridge, or debug reads",
    }) + "\n"));
    if (closeButton) {
        await io.click(closeButton.x, closeButton.y, "left");
        await io.wait(150);
        await saveScreenshot(io, "hud2-close-button-closed", Buffer.from(await io.screenshot()));
    }

    // The game makes no sound until the player turns it on (CEO decision, 2026-11-12). The
    // turret refusal below is judged by its sound, so turn the sound on with one physical click on
    // the visible switch. The guide is closed here (by F1, and by its close button when found).
    await turnSoundOn(io, "hud2-sound");

    let target = null;
    let selectableScreenshot = null;
    for (let attempt = 0; attempt < 80; attempt += 1) {
        selectableScreenshot = Buffer.from(await io.screenshot());
        target = readOwnTarget(decodePng(selectableScreenshot));
        if (target) break;
        if (attempt < 79) await io.wait(100);
    }
    if (!target) throw new Error("HUD2 could not find an own-unit colour component after F1; pixel target is NOT TRACED");
    await saveScreenshot(io, "hud2-after-guide-close", selectableScreenshot);
    await io.save("hud2-pixel-target.json", Buffer.from(JSON.stringify({
        target,
        method: "largest slot-0 colour component in a PNG screenshot after physical F1; no package, bridge, or debug reads",
    }) + "\n"));

    await io.click(target.x, target.y, "left");
    await io.wait(150);
    const selected = Buffer.from(await io.screenshot());
    await saveScreenshot(io, "hud2-selected-own", selected);
    const tooltipEvidence = [];
    await captureOrderTooltips(io, selected, "hud2-selected-own", 0, tooltipEvidence);

    // Capture selection snapshots for every screenshot-visible unit candidate. The driver
    // derives targets only from slot-colour pixels and uses physical clicks; the oracle later
    // identifies Forge lab/turret/drone from the actual replica checkpoint.
    const targetSourceShot = Buffer.from(await io.screenshot());
    await saveScreenshot(io, "hud2-target-source", targetSourceShot);
    const selectableImage = decodePng(targetSourceShot);
    const ownCandidates = readSlotTargets(selectableImage, 0);
    const foreignCandidates = readSlotTargets(selectableImage, 1);
    const placementCandidates = FORGE_MAP_TARGETS.map((placement) => {
        const observed = findPaletteTargetNear(selectableImage, placement.slot, placement.seed);
        if (!observed) {
            throw new Error(`HUD2 could not observe current slot-colour/shape pixels near authored ${placement.source}; pixel target is NOT TRACED`);
        }
        return { ...placement, target: observed.target, observation: observed.observation };
    });
    const clearTarget = findOpenGround(selectableImage, [...ownCandidates, ...foreignCandidates]);
    if (!clearTarget) throw new Error("HUD2 could not identify clear world ground from pixels");
    const candidates = [
        ...placementCandidates,
        ...foreignCandidates.map((candidate) => ({ slot: 1, target: candidate })),
    ];
    const uniqueCandidates = candidates.filter((candidate, index) => candidates.findIndex((other) =>
        other.slot === candidate.slot && other.target.x === candidate.target.x &&
        other.target.y === candidate.target.y) === index);
    const attemptedCandidates = [];
    let candidateIndex = 0;
    let sawResearchPanel = false;
    let researchTarget = null;
    // Preserve the current-image observations before any seeded target receives physical IO.
    await io.save("hud2-target-observations.json", Buffer.from(JSON.stringify({
        screenshot: "hud2-target-source.png",
        screenCandidates: { own: ownCandidates, foreign: foreignCandidates },
        placementSeeds: placementCandidates.map(({ source, ...candidate }) => ({ source, ...candidate })),
        method: "authored map/camera literals seed local search; each click point is a current screenshot pixel matching the slot palette",
    }, null, 2) + "\n"));
    let errorAudioTriggered = false;
    for (const candidateEntry of uniqueCandidates) {
        const { slot, source, seed } = candidateEntry;
        let candidate = candidateEntry.target;
        if (candidate.x === target.x && candidate.y === target.y) continue;
        await io.click(clearTarget.x, clearTarget.y, "left");
        await io.wait(120);
        let targetObservationFile = null;
        if (seed) {
            const sourceLabel = `hud2-target-source-${seed.x}-${seed.y}`;
            const sourceShot = Buffer.from(await io.screenshot());
            await saveScreenshot(io, sourceLabel, sourceShot);
            const observed = findPaletteTargetNear(decodePng(sourceShot), slot, seed);
            if (!observed) {
                throw new Error(`HUD2 current screenshot has no slot-colour/shape pixels near authored ${source}; pixel target is NOT TRACED`);
            }
            candidate = observed.target;
            targetObservationFile = `hud2-target-observation-${seed.x}-${seed.y}.json`;
            await io.save(targetObservationFile, Buffer.from(JSON.stringify({
                screenshot: `${sourceLabel}.png`, source, ...observed.observation,
            }, null, 2) + "\n"));
        }
        await io.click(candidate.x, candidate.y, "left");
        await io.wait(150);
        const label = `hud2-candidate-${String(candidateIndex).padStart(2, "0")}-slot-${slot}`;
        const candidateShot = Buffer.from(await io.screenshot());
        await saveScreenshot(io, label, candidateShot);
        attemptedCandidates.push({ label, slot, target: candidate,
            targetSource: source || "screen palette component", targetObservationFile });
        candidateIndex += 1;
        await captureOrderTooltips(io, candidateShot, label, slot, tooltipEvidence);
        const candidateImage = decodePng(Buffer.from(await io.screenshot()));
        if (slot === 0 && hasResearchPanel(candidateImage)) {
            sawResearchPanel = true;
            researchTarget = candidateEntry;
        }
        if (!errorAudioTriggered && slot === 0 && hasTurretInfoRows(decodePng(candidateShot))) {
            await io.click(clearTarget.x, clearTarget.y, "right");
            await io.wait(200);
            await saveScreenshot(io, "hud2-turret-cannot-move", Buffer.from(await io.screenshot()));
            errorAudioTriggered = true;
        }
    }
    if (sawResearchPanel && researchTarget) {
        await io.click(clearTarget.x, clearTarget.y, "left");
        await io.wait(120);
        const researchSeed = researchTarget.seed;
        const researchSourceLabel = `hud2-target-source-research-${researchSeed.x}-${researchSeed.y}`;
        const researchSourceShot = Buffer.from(await io.screenshot());
        await saveScreenshot(io, researchSourceLabel, researchSourceShot);
        const researchPixelTarget = findPaletteTargetNear(decodePng(researchSourceShot), 0, researchSeed);
        if (!researchPixelTarget) {
            throw new Error("HUD2 current screenshot has no slot-colour/shape pixels at the Forge lab; pixel target is NOT TRACED");
        }
        await io.save("hud2-research-target-observation.json", Buffer.from(JSON.stringify({
            screenshot: `${researchSourceLabel}.png`, source: researchTarget.source,
            ...researchPixelTarget.observation,
        }, null, 2) + "\n"));
        await io.click(researchPixelTarget.target.x, researchPixelTarget.target.y, "left");
        await io.wait(150);
        let enabledResearch = false;
        for (let attempt = 0; attempt < 45; attempt += 1) {
            await io.wait(1000);
            const currentBytes = Buffer.from(await io.screenshot());
            if (hasEnabledResearchButton(decodePng(currentBytes))) {
                enabledResearch = true;
                await saveScreenshot(io, "hud2-research-enabled", currentBytes);
                break;
            }
        }
        if (!enabledResearch) {
            throw new Error("HUD2 lab did not visibly enable its authored research button within 45 seconds of the real factory simulation");
        }
        await io.click(RESEARCH_BUTTON_TARGET.x, RESEARCH_BUTTON_TARGET.y, "left");
        await io.wait(150);
        await saveScreenshot(io, "hud2-research-enqueued", Buffer.from(await io.screenshot()));
        await io.click(QUEUE_CANCEL_TARGET.x, QUEUE_CANCEL_TARGET.y, "left");
        await io.wait(150);
        await saveScreenshot(io, "hud2-queue-cancelled", Buffer.from(await io.screenshot()));
    }
    await io.save("hud2-selection-candidates.json", Buffer.from(JSON.stringify({
        clearTarget,
        candidates,
        attemptedCandidates,
        method: "Forge lab/turret placements seed localized current-PNG slot-colour searches; foreign candidates come from current-PNG slot-colour components; no package, bridge, or debug reads",
    }, null, 2) + "\n"));
    await io.save("hud2-order-tooltip-targets.json", Buffer.from(JSON.stringify(tooltipEvidence, null, 2) + "\n"));
}

async function saveScreenshot(io, label, screenshot) {
    await io.save(`${label}.png`, screenshot);
    await io.save(`${label}.png.b64`, Buffer.from(screenshot.toString("base64")));
}

function countFillPixels(image, rect, expected, tolerance) {
    let count = 0;
    for (let y = rect.y; y < rect.y + rect.height; y += 1) {
        for (let x = rect.x; x < rect.x + rect.width; x += 1) {
            const offset = (y * image.width + x) * 4;
            const distance = Math.hypot(
                image.rgba[offset] - expected[0],
                image.rgba[offset + 1] - expected[1],
                image.rgba[offset + 2] - expected[2],
            );
            if (distance <= tolerance) count += 1;
        }
    }
    return count;
}

function hasResearchPanel(image) {
    return countFillPixels(image, RESEARCH_PANEL, PANEL_FILL, TOLERANCE) >=
        RESEARCH_PANEL.width * RESEARCH_PANEL.height * 0.30;
}

function hasEnabledResearchButton(image) {
    return RESEARCH_BUTTON_SAMPLES.every(({ x, y }) => {
        const offset = (y * image.width + x) * 4;
        const pixel = [image.rgba[offset], image.rgba[offset + 1], image.rgba[offset + 2]];
        const enabledDistance = Math.hypot(...pixel.map((channel, index) =>
            channel - RESEARCH_BUTTON_ENABLED_FILL[index]));
        const disabledDistance = Math.hypot(...pixel.map((channel, index) =>
            channel - RESEARCH_BUTTON_DISABLED_FILL[index]));
        return enabledDistance <= 12 && enabledDistance + 12 < disabledDistance;
    });
}

function hasTurretInfoRows(image) {
    return [350, 374, 398].every((y) => {
        let bright = 0;
        for (let py = y; py < y + 20; py += 1) {
            for (let x = 18; x < 270; x += 1) {
                const offset = (py * image.width + x) * 4;
                if (Math.max(image.rgba[offset], image.rgba[offset + 1], image.rgba[offset + 2]) > 170) bright += 1;
            }
        }
        return bright >= 4;
    });
}

async function captureOrderTooltips(io, screenshot, candidateLabel, slot, evidence) {
    const image = decodePng(screenshot);
    const rowCenters = findOrderRowCenters(image);
    for (const target of rowCenters) {
        const index = evidence.length;
        await io.move(target.x, target.y);
        await io.wait(600);
        const label = `hud2-order-tooltip-${String(index).padStart(2, "0")}`;
        const bytes = Buffer.from(await io.screenshot());
        await saveScreenshot(io, label, bytes);
        evidence.push({ label, candidateLabel, slot, target });
    }
}

function findOrderRowCenters(image) {
    const rowCenters = [];
    const visibleRows = Math.min(COMMAND_ROWS.maxItems,
        Math.floor((COMMAND_ROWS.height + COMMAND_ROWS.gap) /
            (COMMAND_ROWS.rowHeight + COMMAND_ROWS.gap)));
    for (let index = 0; index < visibleRows; index += 1) {
        const rowY = COMMAND_ROWS.y + index * (COMMAND_ROWS.rowHeight + COMMAND_ROWS.gap);
        const point = { x: COMMAND_ROWS.x + COMMAND_ROWS.width - 9, y: rowY + COMMAND_ROWS.rowHeight / 2 };
        const buttonArea = { x: point.x - 5, y: point.y - 4, width: 8, height: 8 };
        const blackButtonPixels = countFillPixels(image, buttonArea, [0, 0, 0], 8);
        const textArea = { x: COMMAND_ROWS.x + 4, y: rowY, width: COMMAND_ROWS.width - 24, height: COMMAND_ROWS.rowHeight };
        let brightGlyphPixels = 0;
        for (let y = textArea.y; y < textArea.y + textArea.height; y += 1) {
            for (let x = textArea.x; x < textArea.x + textArea.width; x += 1) {
                const offset = (y * image.width + x) * 4;
                if (Math.max(image.rgba[offset], image.rgba[offset + 1], image.rgba[offset + 2]) > 180) {
                    brightGlyphPixels += 1;
                }
            }
        }
        if (blackButtonPixels >= 48 && brightGlyphPixels >= 24) rowCenters.push(point);
    }
    return rowCenters;
}

function findCloseButton(image) {
    let minX = image.width;
    let minY = image.height;
    let maxX = -1;
    let maxY = -1;
    let count = 0;
    for (let y = GUIDE_CLOSE_AREA.y; y < GUIDE_CLOSE_AREA.y + GUIDE_CLOSE_AREA.height; y += 1) {
        for (let x = GUIDE_CLOSE_AREA.x; x < GUIDE_CLOSE_AREA.x + GUIDE_CLOSE_AREA.width; x += 1) {
            const offset = (y * image.width + x) * 4;
            if (Math.hypot(image.rgba[offset] - GUIDE_CLOSE_FILL[0],
                image.rgba[offset + 1] - GUIDE_CLOSE_FILL[1],
                image.rgba[offset + 2] - GUIDE_CLOSE_FILL[2]) > 35) continue;
            count += 1;
            minX = Math.min(minX, x);
            minY = Math.min(minY, y);
            maxX = Math.max(maxX, x);
            maxY = Math.max(maxY, y);
        }
    }
    if (count < 20) return null;
    return { x: Math.round((minX + maxX) / 2), y: Math.round((minY + maxY) / 2), pixels: count };
}

module.exports = { drive, hasEnabledResearchButton, findOrderRowCenters };
