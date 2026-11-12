// turts-test-role: harness
"use strict";

const fs = require("node:fs/promises");
const { createReadStream } = require("node:fs");
const path = require("node:path");
const readline = require("node:readline");

const BRIDGES = new Set(["turts_debug", "turts_scene", "turts_ui", "turts_memory_bytes"]);

// The optional one-based window argument keeps the oracle read-only while allowing it to inspect
// every page in a multi-window row. Omitting it follows the page most recently selected by io.use.
function createView(pages, inputLog, serverDir, state = { selected: 0 }) {
    const view = {
        async bridge(name, windowNumber = state.selected + 1) {
            if (name === "turts_move") throw new Error("view.bridge refuses turts_move");
            if (!BRIDGES.has(name)) throw new Error(`unknown read-only bridge: ${name}`);
            const selected = checkedWindow(windowNumber, pages.length);
            // The client builds the debug state only when asked (design D 9, row DBG1). The
            // request is made here, in one place, for every oracle: it waits for one fresh record
            // (the 4 Hz cap of the client applies). A page with no request bridge is an older
            // build that publishes in each frame, so the plain read is right for it. A held frame
            // (the SH1 phase capture) runs no callback, so a request would wait 10 s and reject: the
            // record that the held frame published is read as it is (harness/scene-checkpoints.js).
            const raw = await pages[selected].evaluate(async (bridge) => {
                const getter = window[bridge];
                const held = typeof window.__turts_input_held === "function" && window.__turts_input_held();
                if (bridge === "turts_debug" && !held && typeof window.turts_debug_request === "function") {
                    return window.turts_debug_request();
                }
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
        // The lines of a large server file, one at a time (the M3 economy log reaches 1.5 GB, past
        // the largest string that serverFile can return: run t21-run118-m3-2).
        async *serverLines(file) {
            if (typeof file !== "string" || !file || file.includes("\0")) throw new TypeError("server file path must be non-empty");
            const base = path.resolve(serverDir);
            const target = path.resolve(base, file);
            if (target !== base && !target.startsWith(base + path.sep)) throw new Error("server file path escapes output directory");
            const lines = readline.createInterface({ input: createReadStream(target), crlfDelay: Infinity });
            for await (const line of lines) yield line;
        },
        async serverBytes(file) {
            if (typeof file !== "string" || !file || file.includes("\0")) throw new TypeError("server artifact path must be non-empty");
            const base = path.resolve(serverDir);
            const target = path.resolve(base, file);
            if (target !== base && !target.startsWith(base + path.sep)) throw new Error("server artifact path escapes output directory");
            return fs.readFile(target);
        },
    };
    return Object.freeze(view);
}

function checkedWindow(windowNumber, count) {
    if (!Number.isInteger(windowNumber) || windowNumber < 1 || windowNumber > count) throw new RangeError(`window must be in 1..${count}`);
    return windowNumber - 1;
}

module.exports = { createView };
