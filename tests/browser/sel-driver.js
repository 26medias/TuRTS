// turts-test-role: driver
"use strict";

const { decodePng } = require("./harness/png.js");

async function drive(io, params) {
    if (!Array.isArray(params.stages) || params.stages.length === 0) throw new Error("SEL driver needs literal pixel stages");
    const initial = decodePng(await io.screenshot());
    await io.save("sel-initial.png", Buffer.from(await io.screenshot()));
    for (const stage of params.stages) {
        const { start, end } = stage;
        if (!validPoint(start, initial) || !validPoint(end, initial)) throw new Error(`SEL ${stage.label} has invalid screenshot pixel coordinates`);
        if (stage.gesture === "box") {
            await withModifiers(io, stage.modifiers || [], async () => io.drag(start.x, start.y, end.x, end.y, "left"));
        } else if (stage.gesture === "click") {
            await withModifiers(io, stage.modifiers || [], async () => io.click(start.x, start.y, "left"));
        } else if (stage.gesture === "double-click") {
            await io.click(start.x, start.y, "left");
            await io.wait(80);
            await io.click(start.x, start.y, "left");
        } else {
            throw new Error(`SEL ${stage.label} has unsupported pixel gesture ${stage.gesture}`);
        }
        const captured = Buffer.from(await io.screenshot());
        await io.save(`${stage.label}.png`, captured);
        await io.save(`${stage.label}.png.b64`, Buffer.from(captured.toString("base64")));
    }
}

async function withModifiers(io, modifiers, action) {
    for (const modifier of modifiers) await io.key(modifier, [], { action: "down" });
    try { await action(); }
    finally { for (const modifier of [...modifiers].reverse()) await io.key(modifier, [], { action: "up" }); }
}

function validPoint(point, image) {
    return point && Number.isFinite(point.x) && Number.isFinite(point.y) &&
        point.x >= 0 && point.x < image.width && point.y >= 0 && point.y < image.height;
}

module.exports = { drive };
