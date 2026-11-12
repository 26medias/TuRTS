// turts-test-role: driver
"use strict";

const { decodePng, pixelAt } = require("./harness/png.js");
const { findVolumeTracks, volumeClickPoint } = require("./volume-tracks.js");

const SLOT_RGB = [[0, 114, 178], [230, 159, 0]];
const COLOR_TOLERANCE = 48;
const FOREIGN_COLOR_TOLERANCE = 64;
const GROUND_RGB = [104, 148, 100];
const GROUND_COLOR_TOLERANCE = 64;
// The first-join "Click to play" overlay (client_shell.rs sync_focus_overlay, since 3d0cf2f) is
// black at alpha 0.55. Blended in linear light, it scales each sRGB channel by 0.45^(1/2.2).
const OVERLAY_DIM = 0.45 ** (1 / 2.2);
const dimmed = (rgb) => rgb.map((channel) => Math.round(channel * OVERLAY_DIM));

async function drive(io, params) {
    const row = params.row;
    if (row === "NOORD") {
        // A keydown is the gesture that removes the "Click to play" overlay, which dims the
        // notice (crates/turts_client_web/index.html:39-40); F9 has no binding (RUN130).
        await io.key("F9");
        await io.wait(1500);
        await savePngWithBase64(io, "noorders-visible.png");
        return;
    }
    if (!new Set(["ORD2", "ORD6", "ORD7", "ALR1", "AUD1"]).has(row)) {
        throw new Error(`feedback driver has no route for ${row}`);
    }
    if (row !== "AUD1") await io.key("F1");
    const initialPng = row === "AUD1"
        ? await waitForInitialOwnPixels(io)
        : Buffer.from(await io.screenshot());
    await io.save("feedback-initial.png", initialPng);
    let image = decodePng(initialPng);
    if (row === "AUD1") {
        // The first click is the audio gesture; under the overlay the scene is dimmed (RUN130).
        const underOverlay = findBodies(image, SLOT_RGB[0]).length === 0;
        const resume = findGround(image, underOverlay ? dimmed(GROUND_RGB) : GROUND_RGB);
        await io.click(resume.x, resume.y, "left");
        await io.wait(200);
        await io.save("aud1-after-resume.png", await io.screenshot());
        await io.key("F1");
        await io.wait(100);
        image = decodePng(Buffer.from(await io.screenshot()));
    }
    const own = selectBody(findBodies(image, SLOT_RGB[0]), image, "own slot");
    const ground = findGround(image);

    if (row === "ORD2") {
        const box = ord2SelectionBounds(image);
        await io.drag(box.start.x, box.start.y, box.end.x, box.end.y, "left");
        await io.wait(150);
        await io.click(ground.x, ground.y, "right");
        await io.wait(1200);
        await io.save("ord2-result.png", await io.screenshot());
        return;
    }

    if (row === "ALR1") {
        const enemy = selectBody(findBodies(image, SLOT_RGB[1], FOREIGN_COLOR_TOLERANCE), image, "foreign slot");
        const origin = selectAttackOrigin(image, SLOT_RGB[0], COLOR_TOLERANCE, enemy);
        await io.click(origin.x, origin.y, "left");
        await io.wait(100);
        await savePngWithBase64(io, "alr1-attack-origin.png");
        await io.click(enemy.x, enemy.y, "right");
        const alert = await waitForAlr1VisibleRow(io);
        await io.save("alr1-first.png", alert.screenshot);
        await io.save("alr1-first.png.b64", Buffer.from(alert.screenshot.toString("base64"), "utf8"));
        await io.click(alert.point.x, alert.point.y, "left");
        await io.wait(250);
        await savePngWithBase64(io, "alr1-after-alert-click.png");
        for (let sample = 1; sample <= 4; sample += 1) {
            await io.wait(200);
            await savePng(io, `alr1-cooldown-${sample}.png`);
        }
        return;
    }
    await io.click(own.x, own.y, "left");
    await io.wait(100);
    if (row === "ORD6") {
        await io.click(ground.x, ground.y, "right", { count: 100 });
        await io.wait(150);
        await io.save("ord6-burst.png", await io.screenshot());
        await io.wait(1000);
        await io.click(ground.x, ground.y, "right");
        await io.wait(200);
        await io.save("ord6-refill.png", await io.screenshot());
        return;
    }
    if (row === "ORD7") {
        await io.click(ground.x, ground.y, "right");
        await io.wait(1000);
        await io.save("ord7-result.png", await io.screenshot());
        return;
    }
    if (row === "AUD1") {
        await io.click(ground.x, ground.y, "right");
        await io.wait(250);
        await io.key("F10");
        await io.wait(150);
        // The volume sliders are the last controls of the scrolling settings panel (since
        // 3d0cf2f): scroll to the end, then find each track by pixels (RUN130).
        await io.move(640, 400);
        await scrollMenuToEnd(io);
        const volume = { master: 0.9, voice: 0.8, alerts: 0.7, effects: 0.6 };
        for (const [category, value] of Object.entries(volume)) {
            const { tracks } = await waitForVolumeMenuPixels(io, category);
            const { x, y } = volumeClickPoint(tracks[category], value);
            await io.move(x, y);
            const { screenshot: beforeClick } = await waitForVolumeMenuPixels(io, category);
            await savePngWithBase64(io, `aud1-volume-before-${category}.png`, beforeClick);
            await io.click(x, y, "left");
            const afterClick = await waitForVolumeThumbPixels(io, category, x, y);
            await savePngWithBase64(io, `aud1-volume-after-${category}.png`, afterClick);
        }
        await io.wait(150);
        await io.save("aud1-volume-menu.png", await io.screenshot());
        await io.key("F10");
        await io.wait(100);
        await driveAud1SoundCategories(io, SLOT_RGB, COLOR_TOLERANCE);
        const reloadStartedAt = Date.now();
        await io.reload();
        await waitForAud1Rejoin(io, reloadStartedAt);
        await io.key("F1");
        await io.wait(100);
        await savePng(io, "after-reload-context-resumed.png");
        await driveAud1SoundCategories(io, SLOT_RGB, COLOR_TOLERANCE, "after-reload-");
    }
}

// Literal slider coordinates are usable only after the current screenshot shows all four
// tracks in that layout. Two actual paints also let pointer picking observe the new hover.
async function waitForVolumeMenuPixels(io, category) {
    const deadline = Date.now() + 5000;
    let previousReady = false, savedNegative = false;
    while (Date.now() < deadline) {
        const screenshot = Buffer.from(await io.screenshot());
        const tracks = findVolumeTracks(decodePng(screenshot));
        const ready = Boolean(tracks);
        if (ready && previousReady) return { screenshot, tracks };
        if (!ready && !savedNegative) {
            await io.save(`aud1-volume-unready-${category}.png`, screenshot);
            savedNegative = true;
        }
        previousReady = ready;
        await io.wait(Math.min(100, Math.max(1, deadline - Date.now())));
    }
    throw new Error(`AUD1 ${category} volume tracks did not reach their visible stable layout within 5 seconds`);
}

async function waitForVolumeThumbPixels(io, category, x, y) {
    const deadline = Date.now() + 5000;
    let previousReady = false;
    while (Date.now() < deadline) {
        const screenshot = Buffer.from(await io.screenshot());
        const image = decodePng(screenshot);
        const ready = Boolean(findVolumeTracks(image)) && volumeThumbVisible(image, x, y);
        if (ready && previousReady) return screenshot;
        previousReady = ready;
        await io.wait(Math.min(100, Math.max(1, deadline - Date.now())));
    }
    throw new Error(`AUD1 ${category} volume thumb did not visibly reach its requested value within 5 seconds`);
}

function volumeThumbVisible(image, x, y) {
    return [-2, 0, 2].every((dy) => [-2, 0, 2].every((dx) => {
        const pixel = pixelAt(image, x + dx, y + dy);
        return pixel && pixel[3] >= 245 &&
            Math.hypot(pixel[0] - 102, pixel[1] - 191, pixel[2] - 242) <= 12;
    }));
}

// Wheel steps inside the menu until two steps in a row leave the panel unchanged.
const MENU_PANEL = { x: 400, y: 40, width: 480, height: 640 };
async function scrollMenuToEnd(io) {
    let previous = decodePng(Buffer.from(await io.screenshot()));
    let still = 0;
    for (let step = 0; step < 40 && still < 2; step += 1) {
        await io.wheel(300);
        await io.wait(200);
        const current = decodePng(Buffer.from(await io.screenshot()));
        still = samePanel(previous, current) ? still + 1 : 0;
        previous = current;
    }
}

function samePanel(left, right) {
    for (let y = MENU_PANEL.y; y < MENU_PANEL.y + MENU_PANEL.height; y += 2) {
        for (let x = MENU_PANEL.x; x < MENU_PANEL.x + MENU_PANEL.width; x += 2) {
            const offset = (y * left.width + x) * 4;
            for (let channel = 0; channel < 3; channel += 1) {
                if (Math.abs(left.rgba[offset + channel] - right.rgba[offset + channel]) > 24) return false;
            }
        }
    }
    return true;
}

async function waitForInitialOwnPixels(io) {
    const deadline = Date.now() + 10000;
    let savedPrepaint = false;
    while (Date.now() < deadline) {
        const screenshot = Buffer.from(await io.screenshot());
        const image = decodePng(screenshot);
        if (findBodies(image, SLOT_RGB[0]).length > 0 || findBodies(image, dimmed(SLOT_RGB[0])).length > 0) return screenshot;
        if (!savedPrepaint) {
            await io.save("aud1-prepaint.png", screenshot);
            savedPrepaint = true;
        }
        await io.wait(Math.min(100, Math.max(1, deadline - Date.now())));
    }
    throw new Error("AUD1 initial scene did not paint own-unit pixels before the bounded readiness deadline");
}

async function driveAud1SoundCategories(io, slotColors, tolerance, prefix = "") {
    const screenshot = Buffer.from(await io.screenshot());
    await savePngWithBase64(io, `${prefix}category-scan.png`, screenshot);
    const image = decodePng(screenshot);
    const enemy = selectBody(findBodies(image, slotColors[1], FOREIGN_COLOR_TOLERANCE), image, "foreign slot");
    const own = selectAttackOrigin(image, slotColors[0], tolerance, enemy);
    await io.click(own.x, own.y, "left");
    await io.wait(100);
    await io.click(enemy.x, enemy.y, "right");
    await io.wait(7000);
    await savePng(io, `${prefix}live-alert-sink.png`);

    const afterCombatPng = Buffer.from(await io.screenshot());
    await savePngWithBase64(io, `${prefix}after-combat-scan.png`, afterCombatPng);
    const afterCombat = decodePng(afterCombatPng);
    const ownBodies = findBodies(afterCombat, slotColors[0]);
    const statics = ownBodies.filter((body) => body.maxX - body.minX > 30 && body.maxY - body.minY > 30);
    const post = selectBody(statics, afterCombat, "large own static unit");
    const ground = findGround(afterCombat);
    await io.click(post.x, post.y, "left");
    await io.wait(100);
    await io.click(ground.x, ground.y, "right");
    await io.wait(250);
    await savePng(io, `${prefix}live-effects-sink.png`);
}

async function waitForAud1Rejoin(io, reloadStartedAt) {
    let connected = false;
    let countdownStartedAt = null;
    let second = 0;
    while (!connected && Date.now() - reloadStartedAt < 15000) {
        const elapsed = Date.now() - reloadStartedAt;
        await io.wait(elapsed === 0 ? 250 : Math.min(countdownStartedAt === null ? 250 : 1000, 15000 - elapsed));
        const screenshot = Buffer.from(await io.screenshot());
        const image = decodePng(screenshot);
        // After the reload the overlay waits for a new gesture and dims the scene (RUN130).
        connected = findBodies(image, SLOT_RGB[0]).length > 0 || findBodies(image, dimmed(SLOT_RGB[0])).length > 0;
        if (countdownStartedAt === null && (connected || hasRejoinTextPixels(image))) {
            countdownStartedAt = Date.now();
            await savePngWithBase64(io, "aud1-rejoin-start.png", screenshot);
            await io.save("aud1-reloaded.png", screenshot);
        } else if (countdownStartedAt !== null) {
            second += 1;
            await savePngWithBase64(io, `aud1-rejoin-${String(second).padStart(2, "0")}.png`, screenshot);
        }
        if (connected) await io.save("aud1-rejoined.png", screenshot);
    }
    if (!connected) throw new Error("AUD1 stored identity did not visibly rejoin within the designed 15-second window");
}

async function waitForAlr1VisibleRow(io) {
    const deadline = Date.now() + 8000;
    while (Date.now() < deadline) {
        const screenshot = Buffer.from(await io.screenshot());
        const point = findVisibleAlertText(decodePng(screenshot));
        if (point) return { point, screenshot };
        await io.wait(Math.min(200, Math.max(1, deadline - Date.now())));
    }
    throw new Error("ALR1 did not render a physically visible alert row within the bounded damage window");
}

function findVisibleAlertText(image) {
    const left = Math.max(0, image.width - 320);
    const right = Math.max(left, image.width - 10);
    const bottom = Math.min(180, image.height);
    const rows = [];
    for (let y = 0; y < bottom; y += 1) {
        let count = 0, minX = image.width, maxX = 0;
        for (let x = left; x < right; x += 1) {
            const pixel = pixelAt(image, x, y);
            if (pixel && pixel[3] > 245 && pixel[0] > 220 && pixel[1] > 220 && pixel[2] > 220) {
                count += 1;
                minX = Math.min(minX, x);
                maxX = Math.max(maxX, x);
            }
        }
        if (count >= 2) rows.push({ y, count, minX, maxX });
    }
    const bands = [];
    for (const row of rows) {
        let band = bands[bands.length - 1];
        if (!band || row.y > band.maxY + 2) {
            band = { minY: row.y, maxY: row.y, count: 0, minX: image.width, maxX: 0 };
            bands.push(band);
        }
        band.maxY = row.y;
        band.count += row.count;
        band.minX = Math.min(band.minX, row.minX);
        band.maxX = Math.max(band.maxX, row.maxX);
    }
    const band = bands.find((candidate) => candidate.count >= 16 && candidate.maxX - candidate.minX >= 10);
    return band ? { x: Math.round((band.minX + band.maxX) / 2), y: Math.round((band.minY + band.maxY) / 2) } : null;
}

function hasRejoinTextPixels(image) {
    let white = 0;
    const top = Math.floor(image.height * 0.68);
    const bottom = image.height - 12;
    for (let y = top; y < bottom; y += 1) {
        for (let x = 0; x < Math.min(image.width, 480); x += 1) {
            const pixel = pixelAt(image, x, y);
            if (pixel && pixel[3] > 245 && pixel[0] > 220 && pixel[1] > 220 && pixel[2] > 220) white += 1;
        }
    }
    return white >= 24;
}

function selectAttackOrigin(image, ownColor, tolerance, target) {
    const columns = findBodies(image, ownColor, tolerance).filter((body) => {
        const width = body.maxX - body.minX + 1;
        const height = body.maxY - body.minY + 1;
        return width <= 22 && height >= 20 && height <= 64;
    });
    if (!columns.length) throw new Error("AUD1 screenshot has no visible narrow own troop column for a lawful attack origin");
    return columns.sort((left, right) =>
        Math.hypot(left.x - target.x, left.y - target.y) - Math.hypot(right.x - target.x, right.y - target.y))[0];
}

async function savePng(io, filename, capturedScreenshot = undefined) {
    const screenshot = capturedScreenshot === undefined
        ? Buffer.from(await io.screenshot())
        : Buffer.from(capturedScreenshot);
    await io.save(filename, screenshot);
}

async function savePngWithBase64(io, filename, capturedScreenshot = undefined) {
    const screenshot = capturedScreenshot === undefined
        ? Buffer.from(await io.screenshot())
        : Buffer.from(capturedScreenshot);
    await io.save(filename, screenshot);
    await io.save(`${filename}.b64`, Buffer.from(screenshot.toString("base64"), "utf8"));
}

function findBodies(image, color, tolerance = COLOR_TOLERANCE) {
    const size = image.width * image.height;
    const seen = new Uint8Array(size);
    const queue = new Int32Array(size);
    const components = [];
    const matches = (index) => {
        const x = index % image.width;
        const y = Math.floor(index / image.width);
        if (isHud(x, y, image)) return false;
        const offset = index * 4;
        if (image.rgba[offset + 3] < 245) return false;
        return Math.hypot(image.rgba[offset] - color[0], image.rgba[offset + 1] - color[1], image.rgba[offset + 2] - color[2]) <= tolerance;
    };
    for (let index = 0; index < size; index += 1) {
        if (seen[index] || !matches(index)) continue;
        let tail = 1;
        queue[0] = index;
        seen[index] = 1;
        let minX = image.width, minY = image.height, maxX = 0, maxY = 0;
        for (let head = 0; head < tail; head += 1) {
            const current = queue[head];
            const x = current % image.width;
            const y = Math.floor(current / image.width);
            minX = Math.min(minX, x); minY = Math.min(minY, y); maxX = Math.max(maxX, x); maxY = Math.max(maxY, y);
            for (const next of [x ? current - 1 : -1, x + 1 < image.width ? current + 1 : -1,
                y ? current - image.width : -1, y + 1 < image.height ? current + image.width : -1]) {
                if (next >= 0 && !seen[next] && matches(next)) { seen[next] = 1; queue[tail++] = next; }
            }
        }
        const width = maxX - minX + 1;
        const height = maxY - minY + 1;
        if (tail >= 20 && width >= 4 && height >= 4 && width <= 100 && height <= 100) {
            components.push({ x: (minX + maxX) / 2, y: (minY + maxY) / 2, minX, minY, maxX, maxY, pixels: tail });
        }
    }
    return components;
}

function selectBody(bodies, image, label) {
    if (!bodies.length) throw new Error(`feedback screenshot has no ${label} body pixels`);
    const center = { x: image.width / 2, y: image.height / 2 };
    return bodies.sort((left, right) => Math.hypot(left.x - center.x, left.y - center.y) - Math.hypot(right.x - center.x, right.y - center.y))[0];
}

function findGround(image, groundRgb = GROUND_RGB) {
    const anchor = { x: Math.round(image.width * 0.75), y: Math.round(image.height * 0.52) };
    let best = null;
    let distance = Infinity;
    for (let y = Math.max(80, anchor.y - 90); y < Math.min(image.height - 100, anchor.y + 90); y += 4) {
        for (let x = Math.max(330, anchor.x - 130); x < Math.min(image.width - 230, anchor.x + 130); x += 4) {
            const pixel = pixelAt(image, x, y);
            if (!pixel || pixel[3] < 245 || Math.hypot(pixel[0] - groundRgb[0], pixel[1] - groundRgb[1], pixel[2] - groundRgb[2]) > GROUND_COLOR_TOLERANCE) continue;
            const candidateDistance = Math.hypot(x - anchor.x, y - anchor.y);
            if (candidateDistance < distance) { best = { x, y }; distance = candidateDistance; }
        }
    }
    if (!best) throw new Error("feedback screenshot has no ground-colour click target outside the widgets");
    return best;
}

function bounds(entries, image, margin) {
    if (!entries.length) throw new Error("ORD2 screenshot has no visible own units in its trooper region");
    const minX = Math.min(...entries.map((entry) => entry.minX));
    const minY = Math.min(...entries.map((entry) => entry.minY));
    const maxX = Math.max(...entries.map((entry) => entry.maxX));
    const maxY = Math.max(...entries.map((entry) => entry.maxY));
    return {
        start: { x: Math.max(1, minX - margin), y: Math.max(1, minY - margin) },
        end: { x: Math.min(image.width - 2, maxX + margin), y: Math.min(image.height - 2, maxY + margin) },
    };
}

function ord2SelectionBounds(image) {
    const ownBodies = findBodies(image, SLOT_RGB[0]);
    const troopColumns = ownBodies.filter((body) => {
        const width = body.maxX - body.minX + 1;
        const height = body.maxY - body.minY + 1;
        return width <= 22 && height >= 20 && height <= 64;
    });
    const wideTallPosts = ownBodies.filter((body) => {
        const width = body.maxX - body.minX + 1;
        const height = body.maxY - body.minY + 1;
        return width >= 30 && height >= 30;
    });
    if (troopColumns.length === 0 || wideTallPosts.length !== 1) {
        throw new Error(`ORD2 expected visible troop columns and one wide post; got ${troopColumns.length} and ${wideTallPosts.length}`);
    }
    return bounds([...troopColumns, ...wideTallPosts], image, 6);
}

function isHud(x, y, image) {
    return (x < 300 && y < 330) || (x > image.width - 320 && y < 180) ||
        (x > image.width - 220 && y > image.height - 220) || y > image.height - 40;
}

module.exports = { drive };
