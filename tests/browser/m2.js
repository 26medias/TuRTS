"use strict";

// M2 browser interaction helpers. These read only the actual Bevy scene/UI snapshots and use
// Playwright's normal pointer path; they never submit commands through the debug bridge.
const assert = require("node:assert/strict");

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

async function clickWindowLogicalPoint(page, scene, point) {
    const [windowWidth, windowHeight] = scene.window_logical_size ?? [];
    assert(Number.isFinite(windowWidth) && windowWidth > 0 && Number.isFinite(windowHeight) && windowHeight > 0);
    const box = await canvasBox(page);
    await page.mouse.click(
        box.x + point[0] * box.width / windowWidth,
        box.y + point[1] * box.height / windowHeight,
    );
}

async function clickRenderedEntity(page, scene, entry) {
    const point = projectRenderedEntity(scene, entry);
    await clickWindowLogicalPoint(page, scene, point);
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

async function clickRenderedUiAction(page, scene, actionPredicate) {
    const rows = await readBridge(page, "turts_ui");
    const control = rows.find((row) =>
        row.visible && row.enabled && row.action && actionPredicate(row.action, row.text)
    );
    assert(control, "no visible enabled rendered Text node matches the requested UI action");
    const point = uiActionWindowPoint(scene, control.rect);
    assert(point, `rendered UI action text is partially clipped or lacks projection metadata: ${JSON.stringify(control)}`);
    const [windowWidth, windowHeight] = scene.window_logical_size;
    const box = await canvasBox(page);
    await page.mouse.click(
        box.x + point[0] * box.width / windowWidth,
        box.y + point[1] * box.height / windowHeight,
    );
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
};
