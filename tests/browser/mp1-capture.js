// turts-test-role: instrument
// MP1 capture tool (plan task L0-DBG, step 2; ruling R-V): read the debug records of a person's
// Chrome through its remote debugging port. The page posts nothing: this tool asks the page
// bridge, as the harness does (design D 9, row DBG1).
//
// Usage: node mp1-capture.js <port> [--out <file>]
//
// <port> is the port that `scripts/mp1-play.sh` printed (`DEBUG_PORT: <n>`). The tool has no
// default port. It connects to 127.0.0.1 only, never opens or closes a page, and disconnects at
// the end; the person's windows stay open. For each open window (page) that has the debug bridge it
// asks for one record with the default 4 Hz cap, and keeps three parts of it: `hints`, `audio`
// and `effects.recent`. A part that the build does not publish yet is `null`.
//
// Output: one JSON object on stdout (and in <file>): { port, windows: [{ url, available, hints,
// audio, effects_recent, error }] }. Exit: 0 every window with the bridge gave a record and at
// least one exists; 1 no record; 2 wrong arguments.
"use strict";

const fs = require("node:fs/promises");

const PAGE_TIMEOUT_MS = 12000;

// Runs in the page: one fresh record, and the three parts of it that this tool keeps.
async function readRecord() {
    if (typeof window.turts_debug !== "function") return { available: false };
    const text = await (window.turts_debug_request?.() ?? window.turts_debug());
    const debug = JSON.parse(text);
    return { available: true, hints: debug.hints ?? null, audio: debug.audio ?? null, effects_recent: debug.effects?.recent ?? null };
}

function parsePort(text) {
    if (!/^\d+$/.test(text ?? "") || Number(text) < 1 || Number(text) > 65535) {
        throw new Error("usage: mp1-capture.js <port 1-65535> [--out <file>] (the port is printed by scripts/mp1-play.sh)");
    }
    return Number(text);
}

function parseArgs(argv) {
    const [portText, ...rest] = argv;
    const port = parsePort(portText);
    let out = null;
    for (let i = 0; i < rest.length; i += 1) {
        if (rest[i] === "--out" && rest[i + 1]) { out = rest[i + 1]; i += 1; } else throw new Error(`unknown argument ${rest[i]}`);
    }
    return { port, out };
}

// Read each window through `chromium` (the playwright-core object). `chromium` is a parameter so
// that a unit test can give a fake.
async function capture(chromium, port) {
    const browser = await chromium.connectOverCDP(`http://127.0.0.1:${port}`);
    const windows = [];
    try {
        for (const context of browser.contexts()) {
            for (const page of context.pages()) {
                const url = page.url();
                // Only the pages of a game window: the browser's own pages have no bridge.
                if (!/^(https?|file):/.test(url)) continue;
                const entry = { url, available: false, hints: null, audio: null, effects_recent: null, error: null };
                try {
                    let timer;
                    const record = await Promise.race([page.evaluate(readRecord),
                        new Promise((_, reject) => { timer = setTimeout(() => reject(new Error("no record within 12 s (a hidden window runs no frame)")), PAGE_TIMEOUT_MS); })])
                        .finally(() => clearTimeout(timer));
                    Object.assign(entry, record);
                } catch (error) {
                    entry.error = String(error?.message || error);
                }
                windows.push(entry);
            }
        }
    } finally {
        // For a browser that was connected to, this drops the connection and leaves Chrome and its windows open.
        await browser.close();
    }
    return { port, windows };
}

async function main(argv) {
    let options;
    try { options = parseArgs(argv); } catch (error) { console.error(error.message); return 2; }
    const { chromium } = require("playwright-core");
    const result = await capture(chromium, options.port);
    const text = JSON.stringify(result, null, 2) + "\n";
    if (options.out) await fs.writeFile(options.out, text);
    process.stdout.write(text);
    const reads = result.windows.filter((window) => window.available);
    return reads.length > 0 && result.windows.every((window) => !window.error) ? 0 : 1;
}

if (require.main === module) {
    main(process.argv.slice(2)).then((code) => { process.exitCode = code; },
        (error) => { console.error(`mp1-capture: ${error.message || error}`); process.exitCode = 1; });
}

module.exports = { capture, parseArgs, parsePort, readRecord };
