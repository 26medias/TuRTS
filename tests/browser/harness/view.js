// turts-test-role: harness
"use strict";

const fs = require("node:fs/promises");
const path = require("node:path");

const BRIDGES = new Set(["turts_debug", "turts_scene", "turts_ui", "turts_memory_bytes"]);

// The optional one-based window argument keeps the oracle read-only while allowing it to inspect
// every page in a multi-window row. Omitting it follows the page most recently selected by io.use.
function createView(pages, inputLog, serverDir, state = { selected: 0 }) {
    const view = {
        async bridge(name, windowNumber = state.selected + 1) {
            if (name === "turts_move") throw new Error("view.bridge refuses turts_move");
            if (!BRIDGES.has(name)) throw new Error(`unknown read-only bridge: ${name}`);
            const selected = checkedWindow(windowNumber, pages.length);
            const raw = await pages[selected].evaluate((bridge) => {
                const getter = window[bridge];
                return typeof getter === "function" ? getter() : null;
            }, name);
            if (name === "turts_memory_bytes") return raw;
            if (typeof raw !== "string") return raw;
            return JSON.parse(raw);
        },
        inputLog() { return structuredClone(inputLog); },
        async screenshot(windowNumber = state.selected + 1) { return pages[checkedWindow(windowNumber, pages.length)].screenshot({ type: "png", scale: "css" }); },
        async serverFile(file) {
            if (typeof file !== "string" || !file || file.includes("\0")) throw new TypeError("server file path must be non-empty");
            const base = path.resolve(serverDir);
            const target = path.resolve(base, file);
            if (target !== base && !target.startsWith(base + path.sep)) throw new Error("server file path escapes output directory");
            return fs.readFile(target, "utf8");
        },
    };
    return Object.freeze(view);
}

function checkedWindow(windowNumber, count) {
    if (!Number.isInteger(windowNumber) || windowNumber < 1 || windowNumber > count) throw new RangeError(`window must be in 1..${count}`);
    return windowNumber - 1;
}

module.exports = { createView };
