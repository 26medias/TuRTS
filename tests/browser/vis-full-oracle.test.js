// turts-test-role: unit
"use strict";

const assert = require("node:assert/strict");
const test = require("node:test");
const { project, ringSamples, visibleForeignGaps, visibleRingPixels, waypointPolylineColorMatches, healthFillRatioMatches, visibleMarkerRingPixels } = require("./vis-full-oracle.js");

function image(width, height) {
    return { width, height, rgba: new Uint8Array(width * height * 4) };
}

function paint(imageData, point, color, radius = 1) {
    for (let y = Math.floor(point[1] - radius); y <= Math.ceil(point[1] + radius); y += 1) {
        for (let x = Math.floor(point[0] - radius); x <= Math.ceil(point[0] + radius); x += 1) {
            if (x < 0 || y < 0 || x >= imageData.width || y >= imageData.height) continue;
            const offset = (y * imageData.width + x) * 4;
            imageData.rgba.set([...color, 255], offset);
        }
    }
}

function paintRing(imageData, scene, unit, color, gaps) {
    const samples = ringSamples(imageData, scene, unit, unit.pick_radius * 0.94, 64, 0.025);
    for (let index = 0; index < samples.length; index += 1) {
        const angle = index * Math.PI * 2 / samples.length;
        const atGap = gaps && [0, 1, 2, 3].some((gap) => {
            const center = gap * Math.PI / 2;
            const difference = Math.abs(((angle - center + Math.PI * 3) % (Math.PI * 2)) - Math.PI);
            return difference < 0.09;
        });
        if (samples[index].point) paint(imageData, samples[index].point, [245, 245, 245], 0);
        if (!atGap && samples[index].point) paint(imageData, samples[index].point, color, 0);
    }
}

function paintRingMissingQuadrant(imageData, scene, unit, color, missingQuadrant) {
    const samples = ringSamples(imageData, scene, unit, unit.pick_radius * 0.94, 64, 0.025);
    for (let index = 0; index < samples.length; index += 1) {
        const angle = index * Math.PI * 2 / samples.length;
        const quadrant = Math.floor(angle / (Math.PI / 2));
        const atCardinalGap = [0, 1, 2, 3].some((gap) => {
            const center = gap * Math.PI / 2;
            const difference = Math.abs(((angle - center + Math.PI * 3) % (Math.PI * 2)) - Math.PI);
            return difference < 0.09;
        });
        if (samples[index].point) paint(imageData, samples[index].point, [245, 245, 245], 0);
        if (quadrant !== missingQuadrant && !atCardinalGap && samples[index].point) paint(imageData, samples[index].point, color, 0);
    }
}

function fill(imageData, color) {
    for (let offset = 0; offset < imageData.rgba.length; offset += 4) {
        imageData.rgba.set([...color, 255], offset);
    }
}

test("VIS1 projection uses the actual column-major camera matrix and logical viewport", () => {
    const matrix = [0.5, 0, 0, 0, 0, 0.5, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1];
    assert.deepEqual(project(matrix, [10, 20, 800, 600], [0, 0, 0]), [410, 320]);
});

test("VIS1 pixel oracle accepts a solid own ring and four-gap foreign ring", () => {
    const scene = { clip_from_world: [1, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 1], viewport_logical_rect: [0, 0, 100, 100] };
    const unit = { unit_id: 7, owner_slot: 1, translation: [0, 0, 0], ground_screen_center: [50, 50], pick_radius: 0.5 };
    const orange = [230, 159, 0];
    const baseline = image(100, 100);
    fill(baseline, orange);
    const gaps = image(100, 100);
    gaps.rgba.set(baseline.rgba);
    paintRing(gaps, scene, unit, orange, true);
    assert.equal(visibleForeignGaps(gaps, scene, unit, baseline), true);
    const solid = image(100, 100);
    solid.rgba.set(baseline.rgba);
    paintRing(solid, scene, unit, orange, false);
    assert.equal(visibleForeignGaps(solid, scene, unit, baseline), false);
    assert.equal(visibleRingPixels(solid, scene, unit, unit.pick_radius), true);
    const missingQuadrant = image(100, 100);
    missingQuadrant.rgba.set(baseline.rgba);
    paintRingMissingQuadrant(missingQuadrant, scene, unit, orange, 2);
    assert.equal(visibleForeignGaps(missingQuadrant, scene, unit, baseline), false,
        "four-gap oracle must reject an otherwise good ring with a missing third quadrant");
});

test("VIS1 waypoint oracle follows the current-to-queued polyline, not a direct shortcut", () => {
    const scene = {
        clip_from_world: [0.025, 0, 0, 0, 0, 1, 0, 0, 0, 0.025, 0, 0, 0, 0, 0, 1],
        viewport_logical_rect: [0, 0, 100, 100],
    };
    const unit = { translation: [0, 0, 0] };
    const route = [{ x: 16, y: 16 }, { x: 32, y: 0 }];
    const direct = [{ x: 32, y: 0 }];
    const routeImage = image(100, 100);
    const paintSegment = (start, end) => {
        for (let step = 1; step < 9; step += 1) {
            const t = step / 9;
            paint(routeImage, [start[0] * (1 - t) + end[0] * t, start[1] * (1 - t) + end[1] * t], [230, 230, 217]);
        }
    };
    const start = project(scene.clip_from_world, scene.viewport_logical_rect, [0, 0.04, 0]);
    const current = project(scene.clip_from_world, scene.viewport_logical_rect, [16, 0.04, 16]);
    const queued = project(scene.clip_from_world, scene.viewport_logical_rect, [32, 0.04, 0]);
    paintSegment(start, current);
    paintSegment(current, queued);
    const routeMatches = waypointPolylineColorMatches(routeImage, scene, unit, route);
    assert.equal(routeMatches.length, 2);
    assert(routeMatches.every((matches) => matches >= 3),
        "both the current and queued route segments must be visible");
    assert(waypointPolylineColorMatches(routeImage, scene, unit, direct).every((matches) => matches < 3),
        "a direct unit-to-final-target shortcut must not satisfy the actual queued route oracle");

    const missingQueuedSegment = image(100, 100);
    paintSegmentTo(missingQueuedSegment, start, current);
    const missingMatches = waypointPolylineColorMatches(missingQueuedSegment, scene, unit, route);
    assert(missingMatches[0] >= 3 && missingMatches[1] < 3,
        "a visible current segment without the queued segment must fail the full route requirement");
});

test("VIS1 damaged health fill must shrink and track the same unit's current/max ratio", () => {
    assert.equal(healthFillRatioMatches(42, 48, 35, 40), true);
    assert.equal(healthFillRatioMatches(48, 48, 35, 40), false,
        "a full-width bar at 35/40 health must fail even when the tolerance covers one pixel");
    assert.equal(healthFillRatioMatches(0, 48, 35, 40), false);
});

test("VIS1 marker pixel oracle checks the hollow annulus circumference, not its center", () => {
    const scene = { clip_from_world: [0.5, 0, 0, 0, 0, 0, 0, 0, 0, 0.5, 0, 0, 0, 0, 0, 1], viewport_logical_rect: [0, 0, 100, 100] };
    const marker = { x: 0, y: 0 };
    const ring = image(100, 100);
    for (let step = 0; step < 64; step += 1) {
        const angle = step * Math.PI * 2 / 64;
        const point = project(scene.clip_from_world, scene.viewport_logical_rect,
            [Math.cos(angle) * 0.38, 0.035, Math.sin(angle) * 0.38]);
        paint(ring, point, [242, 204, 96], 1);
    }
    assert(visibleMarkerRingPixels(ring, scene, marker) >= 5);
    const centerOnly = image(100, 100);
    paint(centerOnly, [50, 50], [242, 204, 96], 3);
    assert.equal(visibleMarkerRingPixels(centerOnly, scene, marker), 0,
        "a colored filled center with no annulus pixels must not satisfy the move marker oracle");
});

function paintSegmentTo(imageData, start, end) {
    for (let step = 1; step < 9; step += 1) {
        const t = step / 9;
        paint(imageData, [start[0] * (1 - t) + end[0] * t, start[1] * (1 - t) + end[1] * t], [230, 230, 217]);
    }
}
