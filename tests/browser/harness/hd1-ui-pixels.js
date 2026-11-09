// turts-test-role: pure
"use strict";

const MIN_MATCHING_PIXELS = 5;
const COLOR_TOLERANCE = 3;
const UI_COLORS = Object.freeze({
    move: [121, 192, 255],
    stop: [255, 107, 107],
    hold: [166, 227, 161],
    attack: [255, 95, 95],
    gather: [242, 204, 96],
    remove: [255, 140, 200],
    disable: [255, 166, 87],
    enable: [86, 212, 168],
    produceDummy: [240, 155, 155],
    cancel: [33, 66, 77],
});

const ORDER_COLORS = Object.freeze({
    "muster:order_move": UI_COLORS.move,
    "muster:order_stop": UI_COLORS.stop,
    "muster:order_hold": UI_COLORS.hold,
    "muster:order_attack": UI_COLORS.attack,
    "muster:order_gather": UI_COLORS.gather,
    "muster:order_remove": UI_COLORS.remove,
    "muster:order_disable": UI_COLORS.disable,
    "muster:order_enable": UI_COLORS.enable,
});

function expectedButtonColor(action) {
    if (action && Number.isInteger(action.job_id)) return UI_COLORS.cancel;
    if (action?.content_id === "muster:make_dummy") return UI_COLORS.produceDummy;
    const color = ORDER_COLORS[action?.order];
    if (!color) throw new Error(`HD1 has no authored button color for ${action?.order}`);
    return color;
}

function countCellColor(image, center, size, color) {
    if (!image || !Number.isInteger(image.width) || !Number.isInteger(image.height) ||
        !(image.rgba instanceof Uint8Array) || image.rgba.length !== image.width * image.height * 4) {
        throw new TypeError("HD1 button check needs decoded RGBA screenshot pixels");
    }
    if (!Array.isArray(center) || center.length !== 2 || !center.every(Number.isFinite) ||
        ![32, 64].includes(size) || !Array.isArray(color) || color.length !== 3) {
        throw new TypeError("HD1 button check needs an authored pixel center, cell size, and RGB color");
    }
    const inset = Math.max(2, Math.floor(size * 0.08));
    const left = Math.max(0, Math.ceil(center[0] - size / 2 + inset));
    const right = Math.min(image.width - 1, Math.floor(center[0] + size / 2 - inset));
    const top = Math.max(0, Math.ceil(center[1] - size / 2 + inset));
    const bottom = Math.min(image.height - 1, Math.floor(center[1] + size / 2 - inset));
    let pixels = 0;
    for (let y = top; y <= bottom; y += 1) {
        for (let x = left; x <= right; x += 1) {
            const offset = (y * image.width + x) * 4;
            if (image.rgba[offset + 3] < 128) continue;
            if (color.every((channel, index) => Math.abs(image.rgba[offset + index] - channel) <= COLOR_TOLERANCE)) pixels += 1;
        }
    }
    return { region: [left, top, right, bottom], color: color.slice(), pixels, required_pixels: MIN_MATCHING_PIXELS };
}

module.exports = { COLOR_TOLERANCE, MIN_MATCHING_PIXELS, UI_COLORS, countCellColor, expectedButtonColor };
