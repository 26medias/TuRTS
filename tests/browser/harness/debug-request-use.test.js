// turts-test-role: unit
"use strict";

// Row DBG1 ruling (orchestrator, 2026-11-12, errata-pending-6.3.md, "From the L0-DBG review"): a
// debug request keeps the 4 Hz cap of the client. `min_interval_ms` 0 is a legal test-only form
// for the SH1 per-frame phase capture only. These tests read the tree: no other caller passes an
// argument, and the two request entries say so in a comment.
const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const test = require("node:test");

const REPO = path.resolve(__dirname, "../../..");
const SH1 = "tests/browser/harness/sh1-lifecycle.js";
const WEB_BRIDGE = "crates/turts_client_web/src/lib.rs";

function files(directory, pattern) {
    return fs.readdirSync(directory, { withFileTypes: true }).flatMap((entry) => {
        if (entry.name === "node_modules" || entry.name === "target") return [];
        const full = path.join(directory, entry.name);
        if (entry.isDirectory()) return files(full, pattern);
        return pattern.test(entry.name) && full !== __filename ? [full] : [];
    });
}

// A call of `turts_debug_request` that passes an argument: `[file, line, argument]`.
function argumentCalls(source) {
    return [...source.matchAll(/turts_debug_request\s*(?:\?\.)?\s*\(\s*([^)\s][^)]*)\)/g)]
        .map((match) => [source.slice(0, match.index).split("\n").length, match[1].trim()]);
}

// The Rust calls of `DebugRequest::request` with a value: a `Some(...)` that is not `None`.
function rustCalls(source) {
    return [...source.matchAll(/\.request\(\s*(Some\([^)]*\))\s*\)/g)]
        .map((match) => [source.slice(0, match.index).split("\n").length, match[1]]);
}

test("only the SH1 phase capture passes an interval to turts_debug_request, and it passes 0", () => {
    const callers = files(path.join(REPO, "tests/browser"), /\.(?:js|mjs|cjs)$/)
        .flatMap((file) => argumentCalls(fs.readFileSync(file, "utf8")).map(([line, argument]) =>
            ({ at: `${path.relative(REPO, file)}:${line}`, file: path.relative(REPO, file), argument })));
    assert.deepEqual(callers.map(({ file }) => file), [SH1], `callers with an argument: ${JSON.stringify(callers)}`);
    assert.equal(callers[0].argument, "0");
});

test("the scripts of the repository do not pass an interval to turts_debug_request", () => {
    const scripts = files(path.join(REPO, "scripts"), /\.(?:js|mjs|cjs|sh|py)$/);
    const callers = scripts.flatMap((file) => argumentCalls(fs.readFileSync(file, "utf8")).map(([line]) => `${path.relative(REPO, file)}:${line}`));
    assert.deepEqual(callers, []);
});

test("only the web bridge passes a value to DebugRequest::request outside the tests", () => {
    const sources = ["crates/turts_client/src", "crates/turts_client_web/src"]
        .flatMap((directory) => files(path.join(REPO, directory), /\.rs$/));
    assert.ok(sources.length > 10);
    const callers = sources.flatMap((file) => rustCalls(fs.readFileSync(file, "utf8")).map(([line, value]) => [path.relative(REPO, file), value]));
    assert.deepEqual(callers, [[WEB_BRIDGE, "Some(min_interval_secs)"]]);
});

test("the request entries say that the cap stays and that 0 is the SH1 test form", () => {
    for (const file of [WEB_BRIDGE, "crates/turts_client/src/debug.rs"]) {
        const source = fs.readFileSync(path.join(REPO, file), "utf8").replace(/\n\s*\/\/[\/!]?/g, " ").replace(/\s+/g, " ");
        assert.match(source, /4 Hz cap stays[^]*?`min_interval_ms` 0[^]*?SH1 per-frame phase capture only/, `${file} lacks the ruling comment`);
    }
});

// A file that reads the published record (`turts_debug()`) and never asks for one reads an empty
// string once the client builds the state only on a request (design D 9, row DBG1). The E-GPU
// instrument did so (egpu-run.js, merged after the L0-DBG base).
test("every browser file that reads turts_debug() also asks for a record", () => {
    const readers = files(path.join(REPO, "tests/browser"), /\.js$/)
        .filter((file) => !/\.test\.js$/.test(file) && !file.includes(`${path.sep}tests${path.sep}qa${path.sep}`))
        .map((file) => [path.relative(REPO, file), fs.readFileSync(file, "utf8")])
        .filter(([, source]) => /turts_debug\(\)/.test(source.replace(/^\s*\/\/.*$/gm, "")) && !/turts_debug_request/.test(source));
    assert.deepEqual(readers.map(([file]) => file), []);
});

test("the scan finds a planted caller", () => {
    assert.deepEqual(argumentCalls("a;\nwindow.turts_debug_request(0).catch(() => {});"), [[2, "0"]]);
    assert.deepEqual(argumentCalls("window.turts_debug_request?.(50)"), [[1, "50"]]);
    assert.deepEqual(argumentCalls("window.turts_debug_request?.().catch(() => {})"), []);
    assert.deepEqual(argumentCalls("window.turts_debug_request()"), []);
    assert.deepEqual(rustCalls("a.request(Some(0.0));\nb.request(None);"), [[1, "Some(0.0)"]]);
});
