// turts-test-role: driver
"use strict";

const { decodePng, pixelAt } = require("./harness/png.js");
const { components } = require("./harness/hud4-pixels.js");

async function drive(io, params) {
    await io.key("F1");
    await io.wait(200);
    const before = await io.screenshot();
    const image = decodePng(before);
    await savePng(io, "gm40-before.png", before);

    // Candidate detection and the drag are based only on the captured PNG. The oracle later
    // resolves the actual selected IDs, owner/content, projection, and world positions.
    const formation = findTrooperFormation(image);
    const box = formationBox(image, formation);
    await io.drag(box.start.x, box.start.y, box.end.x, box.end.y, "left");
    await io.wait(150);
    const selected = await io.screenshot();
    await savePng(io, "gm40-after-box.png", selected);

    // The target is a visible ground pixel below the troop formation, clear of every unit
    // component. This uses only the screenshot; C84 scene metadata is reserved for the oracle.
    const groundOrigin = {
        x: Math.round((box.bounds.minX + box.bounds.maxX) / 2),
        y: box.bounds.maxY + 90,
    };
    const target = findGroundNear(image, groundOrigin);
    if (!target) throw new Error("GM40 could not find visible open ground below the troop formation from screenshot pixels; pixel half remains NOT TRACED");
    const pointEvidence = {
        method: "pixel-only: the largest adjacent cluster of filled disc components is the formation; the target is the nearest ground pixel 90 px below it",
        selection_box: box,
        target_pixel: target,
        viewport: { width: image.width, height: image.height },
        status: "pixel coordinates only; the oracle must unproject the actual click through C84 scene metadata",
    };
    await io.save("gm40-pixel-target.json", Buffer.from(JSON.stringify(pointEvidence, null, 2) + "\n"));
    await io.click(target.x, target.y, "right");
    await io.wait(6000);
    const arrival = await io.screenshot();
    await savePng(io, "gm40-after-arrival.png", arrival);
    await io.wait(2100);
    const rest = await io.screenshot();
    await savePng(io, "gm40-after-rest.png", rest);
}

async function savePng(io, name, bytes) {
    await io.save(name, bytes);
    await io.save(`${name}.b64`, Buffer.from(bytes.toString("base64"), "utf8"));
}

// Trooper discs are filled; at the start camera the discs of one column touch and merge into
// one tall component. Beacon dots are sparse or small, and buildings are wider than tall.
function findTrooperFormation(image) {
    const all = components(image);
    const bodies = all.filter((body) => body.pixels >= 100 && body.fill >= 0.6 &&
        body.height >= 0.7 * body.width && body.width <= 40);
    const parent = bodies.map((_, index) => index);
    const root = (index) => {
        while (parent[index] !== index) {
            parent[index] = parent[parent[index]];
            index = parent[index];
        }
        return index;
    };
    for (let left = 0; left < bodies.length; left += 1) {
        for (let right = left + 1; right < bodies.length; right += 1) {
            const a = bodies[left];
            const b = bodies[right];
            const gapX = Math.max(a.minX, b.minX) - Math.min(a.maxX, b.maxX);
            const gapY = Math.max(a.minY, b.minY) - Math.min(a.maxY, b.maxY);
            if (gapX <= 4 && gapY <= 4) parent[root(left)] = root(right);
        }
    }
    const groups = new Map();
    bodies.forEach((body, index) => {
        const key = root(index);
        if (!groups.has(key)) groups.set(key, []);
        groups.get(key).push(body);
    });
    const ranked = [...groups.values()]
        .map((members) => ({ members, pixels: members.reduce((total, body) => total + body.pixels, 0) }))
        .sort((left, right) => right.pixels - left.pixels);
    if (ranked.length === 0) throw new Error("GM40 screenshot has no filled disc components; pixel selection remains NOT TRACED");
    return { members: ranked[0].members, pixels: ranked[0].pixels, others: all };
}

function formationBox(image, formation) {
    const bounds = formation.members.reduce((box, body) => ({
        minX: Math.min(box.minX, body.minX),
        minY: Math.min(box.minY, body.minY),
        maxX: Math.max(box.maxX, body.maxX),
        maxY: Math.max(box.maxY, body.maxY),
    }), { minX: Infinity, minY: Infinity, maxX: -Infinity, maxY: -Infinity });
    const margin = 8;
    const start = findGroundNear(image, { x: bounds.minX - margin, y: bounds.minY - margin }, 4);
    const end = findGroundNear(image, { x: bounds.maxX + margin, y: bounds.maxY + margin }, 4);
    if (!start || !end) throw new Error("GM40 formation box corners are not on visible ground; pixel selection remains NOT TRACED");
    const inside = (body) => {
        const x = (body.minX + body.maxX) / 2;
        const y = (body.minY + body.maxY) / 2;
        return x >= start.x && x <= end.x && y >= start.y && y <= end.y;
    };
    const intruders = formation.others.filter((body) => inside(body) && !formation.members.includes(body));
    if (intruders.length > 0) {
        throw new Error(`GM40 formation box also contains ${intruders.length} other unit components; pixel selection remains NOT TRACED`);
    }
    return { start, end, bounds, formation_components: formation.members.length, formation_pixels: formation.pixels };
}

function findGroundNear(image, origin, limitOverride) {
    const margin = 16;
    const limit = limitOverride ?? Math.min(96, Math.floor(Math.min(image.width, image.height) / 6));
    for (let radius = 0; radius <= limit; radius += 1) {
        for (let dy = -radius; dy <= radius; dy += 1) {
            for (const dx of new Set([-radius, radius])) {
                const point = { x: origin.x + dx, y: origin.y + dy };
                if (isGround(image, point, margin)) return point;
            }
        }
        for (let dx = -radius + 1; dx < radius; dx += 1) {
            for (const dy of new Set([-radius, radius])) {
                const point = { x: origin.x + dx, y: origin.y + dy };
                if (isGround(image, point, margin)) return point;
            }
        }
    }
    return null;
}

function isGround(image, point, margin) {
    if (point.x < margin || point.y < margin || point.x >= image.width - margin || point.y >= image.height - margin) return false;
    const rgba = pixelAt(image, point.x, point.y);
    if (!rgba || rgba[3] < 128) return false;
    const [r, g, b] = rgba;
    return g > r + 12 && g > b + 12 && g < 240;
}

module.exports = { drive, findGroundNear, findTrooperFormation, formationBox };
