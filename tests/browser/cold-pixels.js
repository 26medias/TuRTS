// turts-test-role: instrument
"use strict";

// Q-COLD pass 2: the two pixel measures of the cold-play probes. Images are the `{ width, height, rgba }`
// of harness/png.js. A box or rect is `[x, y, width, height]` in the pixels of the image.

const CHANGE_THRESHOLD = 24; // the sum of the three channel differences; below it a pixel is noise
const TEXT_LUMINANCE = 150;
const TEXT_MARGIN = 2;
const NONFLAT_DELTA = 12;

function clamp(image, rect) {
    const x0 = Math.max(0, Math.floor(rect[0]));
    const y0 = Math.max(0, Math.floor(rect[1]));
    const x1 = Math.min(image.width, Math.ceil(rect[0] + rect[2]));
    const y1 = Math.min(image.height, Math.ceil(rect[1] + rect[3]));
    return { x0, y0, x1: Math.max(x0, x1), y1: Math.max(y0, y1) };
}

// How many pixels differ between two images of one size: in the whole page and inside `box`.
function changeStats(a, b, box) {
    if (a.width !== b.width || a.height !== b.height) throw new Error("the two pictures have different sizes");
    const area = clamp(a, box);
    let pageChanged = 0;
    let boxChanged = 0;
    for (let y = 0; y < a.height; y += 1) {
        for (let x = 0; x < a.width; x += 1) {
            const o = (y * a.width + x) * 4;
            const d = Math.abs(a.rgba[o] - b.rgba[o]) + Math.abs(a.rgba[o + 1] - b.rgba[o + 1]) + Math.abs(a.rgba[o + 2] - b.rgba[o + 2]);
            if (d <= CHANGE_THRESHOLD) continue;
            pageChanged += 1;
            if (x >= area.x0 && x < area.x1 && y >= area.y0 && y < area.y1) boxChanged += 1;
        }
    }
    return { page_changed: pageChanged, page_share: Math.round((pageChanged / (a.width * a.height)) * 10000) / 10000,
        box_changed: boxChanged, box_px: (area.x1 - area.x0) * (area.y1 - area.y0) };
}

// The share of the pixels in `rect` that are not the fill colour of the layer and not text. A layer with an
// opaque fill is flat under its text (about 0); a layer that the world shows through is not. Pixels that are
// bright (text) and the pixels within 2 px of them (the soft edge of a glyph) are left out.
function nonflatShare(image, rect) {
    const area = clamp(image, rect);
    const width = area.x1 - area.x0;
    const height = area.y1 - area.y0;
    if (width <= 0 || height <= 0) return 0;
    const bright = new Uint8Array(width * height);
    for (let y = 0; y < height; y += 1) {
        for (let x = 0; x < width; x += 1) {
            const o = ((area.y0 + y) * image.width + area.x0 + x) * 4;
            if ((image.rgba[o] + image.rgba[o + 1] + image.rgba[o + 2]) / 3 >= TEXT_LUMINANCE) bright[y * width + x] = 1;
        }
    }
    // Dilate in two passes (a row, then a column), so a margin of 2 px costs 4 reads for each pixel.
    const wide = new Uint8Array(width * height);
    for (let y = 0; y < height; y += 1) {
        for (let x = 0; x < width; x += 1) {
            for (let k = -TEXT_MARGIN; k <= TEXT_MARGIN; k += 1) {
                const nx = x + k;
                if (nx >= 0 && nx < width && bright[y * width + nx]) { wide[y * width + x] = 1; break; }
            }
        }
    }
    const bins = new Map();
    const kept = [];
    for (let y = 0; y < height; y += 1) {
        for (let x = 0; x < width; x += 1) {
            let text = false;
            for (let k = -TEXT_MARGIN; k <= TEXT_MARGIN && !text; k += 1) {
                const ny = y + k;
                if (ny >= 0 && ny < height && wide[ny * width + x]) text = true;
            }
            if (text) continue;
            const o = ((area.y0 + y) * image.width + area.x0 + x) * 4;
            const r = image.rgba[o];
            const g = image.rgba[o + 1];
            const b = image.rgba[o + 2];
            kept.push(r, g, b);
            const key = ((r >> 3) << 10) | ((g >> 3) << 5) | (b >> 3);
            bins.set(key, (bins.get(key) ?? 0) + 1);
        }
    }
    const count = kept.length / 3;
    if (count === 0) return 0;
    let modeKey = -1;
    let modeCount = 0;
    for (const [key, n] of bins) if (n > modeCount) { modeKey = key; modeCount = n; }
    // The reference colour is the average of the pixels in the mode bin.
    let sr = 0;
    let sg = 0;
    let sb = 0;
    for (let i = 0; i < kept.length; i += 3) {
        const key = ((kept[i] >> 3) << 10) | ((kept[i + 1] >> 3) << 5) | (kept[i + 2] >> 3);
        if (key === modeKey) { sr += kept[i]; sg += kept[i + 1]; sb += kept[i + 2]; }
    }
    const ref = [sr / modeCount, sg / modeCount, sb / modeCount];
    let off = 0;
    for (let i = 0; i < kept.length; i += 3) {
        if (Math.max(Math.abs(kept[i] - ref[0]), Math.abs(kept[i + 1] - ref[1]), Math.abs(kept[i + 2] - ref[2])) > NONFLAT_DELTA) off += 1;
    }
    return Math.round((off / count) * 10000) / 10000;
}

module.exports = { changeStats, nonflatShare };
