// turts-test-role: driver
"use strict";

const { decodePng, pixelAt } = require("./harness/png.js");

const SLOT_RGB = [[0, 114, 178], [230, 159, 0]];
const SLOT_TOLERANCE = [48, 64];

async function drive(io, params) {
    const shots = [];
    async function capture(window, name) {
        const bytes = Buffer.from(await io.screenshot());
        const image = decodePng(bytes);
        await io.save(name, bytes);
        await io.save(`${name}.b64`, Buffer.from(bytes.toString("base64"), "utf8"));
        shots.push({ window, name });
        return image;
    }
    const { width, height } = params.viewport;
    const inset = params.minimap_inset;
    const size = params.minimap_size;
    const rect = { x: width - inset - size, y: height - inset - size, size };
    const point = (u, v) => ({ x: Math.round(rect.x + size * u), y: Math.round(rect.y + size * v) });
    const local = { focus: [0.87, 0.28], left: [0.18, 0.22], drag_start: [0.28, 0.72], drag_end: [0.72, 0.32], right: [0.95, 0.95] };
    const screen = Object.fromEntries(Object.entries(local).map(([key, value]) => [key, point(...value)]));

    await io.use(2);
    // The first join shows the "Click to play" overlay, which dims the scene (and the ping colour)
    // until a gesture. A keydown is the gesture; F9 has no binding (crates/turts_client_web/index.html:39-40).
    await io.key("F9");
    await io.wait(250);
    await capture(2, "mm1-window2-no-ping.png");

    await io.use(1);
    await io.key("F1");
    await io.wait(250);
    const image = await capture(1, "mm1-window1-before.png");
    const own = selectOuterComponent(findBodies(image, 0), image);
    if (!own) throw new Error("MM1 cannot locate the selected slot colour from current screenshot pixels");
    await io.click(own.x, own.y, "left");
    await io.wait(120);
    await capture(1, "mm1-window1-selected.png");

    await capture(1, "mm1-before-focus.png");
    await io.click(screen.focus.x, screen.focus.y, "left");
    await io.wait(200);
    const focusImage = await capture(1, "mm1-after-focus.png");
    const foreign = selectBody(findBodies(focusImage, 1), focusImage);
    let attackSkipped = !foreign;
    let pingPoll = null;
    if (foreign) {
        await io.click(foreign.x, foreign.y, "right");
        // The trooper walks 3 units a second: the window 1 shot shows its approach.
        await io.wait(2000);
        await capture(1, "mm1-window1-attack.png");
        // The slot 1 window shows the ping once the dummy is damaged. The dummy dies a few seconds
        // later, so the shot is taken when the ping pixels appear, not after a fixed wait.
        await io.use(2);
        pingPoll = await pollForPing(io, params, rect);
        await capture(2, "mm1-window2-ping.png");
        await io.use(1);
    } else {
        // The original stub leaves the enemy off-screen. Keep driving so the oracle records the
        // named physical camera red instead of stopping at a setup error.
        await capture(1, "mm1-window1-attack.png");
    }

    await capture(1, "mm1-before-left.png");
    await io.click(screen.left.x, screen.left.y, "left");
    await io.wait(200);
    await capture(1, "mm1-after-left.png");
    await io.drag(screen.drag_start.x, screen.drag_start.y, screen.drag_end.x, screen.drag_end.y, "left");
    await io.wait(200);
    await capture(1, "mm1-after-drag.png");
    await io.click(screen.right.x, screen.right.y, "right");
    await io.wait(500);
    await capture(1, "mm1-after-right.png");
    // One capture only, 500 ms after the click. The replica of window 1 stood still for about 1.1 s after the
    // kill of the target dummy (RUN142); that stall is repaired in the product (commit 50eb827), so a
    // later capture would only hide it. The oracle bounds the replica lag at this capture.
    await io.move(Math.round(width / 2), Math.round(height / 2));
    await io.wheel(-20);
    await io.wait(200);
    await capture(1, "mm1-after-zoom.png");

    if (!foreign) {
        await io.use(2);
        await io.wait(200);
        await capture(2, "mm1-window2-ping.png");
    }
    await io.save("mm1-driver-steps.json", Buffer.from(JSON.stringify({ rect, local, screen, pixel_targets: { own, foreign }, attack_skipped: attackSkipped, ping_poll: pingPoll, shots }, null, 2) + "\n"));
    // The driver records only screenshot-derived coordinates and uses the public input harness.
}

// Poll window 2 for the ping colour inside the minimap. The first ping seen is the one for the
// first unit that the attacker hits (the dummy next to the target, about 2 s before the target
// ping). The target ping follows by the alert cooldown, and the target dies 6 s after that, so the
// shot waits PING_SETTLE_MS after the first sighting. The wait ends the poll without an error
// when no ping appears: the oracle names that fault from the shot.
const PING_POLL_MS = 30000;
const PING_POLL_MAX = 60;
const PING_SETTLE_MS = 3500;
const PING_MIN_PIXELS = 4;
const PING_TOLERANCE = 28;

async function pollForPing(io, params, rect) {
    const colour = parseHex(params.ping_colour);
    const started = Date.now();
    const record = { seen: false, polls: 0, settle_ms: 0, pixels: 0 };
    for (let poll = 0; poll < PING_POLL_MAX && Date.now() - started < PING_POLL_MS; poll += 1) {
        const image = decodePng(Buffer.from(await io.screenshot()));
        record.polls += 1;
        record.pixels = countColour(image, rect, colour, PING_TOLERANCE);
        if (record.pixels >= PING_MIN_PIXELS) {
            record.seen = true;
            record.seen_after_ms = Date.now() - started;
            await io.wait(PING_SETTLE_MS);
            record.settle_ms = PING_SETTLE_MS;
            return record;
        }
        await io.wait(250);
    }
    return record;
}

function countColour(image, rect, colour, tolerance) {
    let count = 0;
    for (let y = rect.y; y < rect.y + rect.size; y += 1) {
        for (let x = rect.x; x < rect.x + rect.size; x += 1) {
            const pixel = pixelAt(image, x, y);
            if (pixel && pixel[3] >= 245 &&
                Math.hypot(pixel[0] - colour[0], pixel[1] - colour[1], pixel[2] - colour[2]) <= tolerance) count += 1;
        }
    }
    return count;
}

function parseHex(value) {
    const match = /^#([0-9a-fA-F]{6})$/.exec(String(value));
    if (!match) throw new Error(`MM1 ping_colour must be a #rrggbb value, got ${value}`);
    return [0, 2, 4].map((offset) => parseInt(match[1].slice(offset, offset + 2), 16));
}

function findBodies(image, slot) {
    const color = SLOT_RGB[slot];
    const tolerance = SLOT_TOLERANCE[slot];
    const size = image.width * image.height;
    const seen = new Uint8Array(size);
    const queue = new Int32Array(size);
    const components = [];
    const matches = (index) => {
        const x = index % image.width;
        const y = Math.floor(index / image.width);
        if (isHud(x, y, image)) return false;
        const offset = index * 4;
        return image.rgba[offset + 3] >= 245 &&
            Math.hypot(image.rgba[offset] - color[0], image.rgba[offset + 1] - color[1], image.rgba[offset + 2] - color[2]) <= tolerance;
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

function selectBody(bodies, image) {
    const center = { x: image.width / 2, y: image.height / 2 };
    return bodies.sort((left, right) =>
        Math.hypot(left.x - center.x, left.y - center.y) - Math.hypot(right.x - center.x, right.y - center.y))[0] || null;
}

function selectOuterComponent(bodies, image) {
    const columns = bodies
        .filter((body) => {
            const width = body.maxX - body.minX + 1;
            const height = body.maxY - body.minY + 1;
            return width >= 8 && width <= 16 && height >= 45 && height <= 70 &&
                body.pixels >= 450 && body.pixels <= 750;
        })
        .sort((left, right) => right.maxX - left.maxX || right.maxY - left.maxY);
    for (const column of columns) {
        const point = {
            x: Math.round((column.minX + column.maxX) / 2),
            y: column.maxY - 2,
        };
        const pixel = pixelAt(image, point.x, point.y);
        if (pixel && pixel[3] >= 245 &&
            Math.hypot(pixel[0] - SLOT_RGB[0][0], pixel[1] - SLOT_RGB[0][1], pixel[2] - SLOT_RGB[0][2]) <= SLOT_TOLERANCE[0]) {
            return point;
        }
    }
    return null;
}

function isHud(x, y, image) {
    return (x < 300 && y < 330) || (x > image.width - 320 && y < 180) ||
        (x > image.width - 220 && y > image.height - 220) || y > image.height - 40;
}

module.exports = { drive, findBodies, selectBody, selectOuterComponent, countColour };
