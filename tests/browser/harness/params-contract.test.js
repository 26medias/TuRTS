// turts-test-role: unit
"use strict";
const assert = require("node:assert/strict");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");
const { execFileSync } = require("node:child_process");
const test = require("node:test");

function launchParams(row, viewport = "1280x720@2") {
    const source = fs.readFileSync(path.join(__dirname, "../../../scripts/mc-browser.sh"), "utf8");
    const sections = [...source.matchAll(/<<'PYPARAMS'\n([\s\S]*?)\nPYPARAMS/g)];
    assert.equal(sections.length, 1, "test the one actual launcher parameter generator");
    const output = fs.mkdtempSync(path.join(os.tmpdir(), "mc-params-"));
    try {
        const rowPath = path.join(output, "row.json");
        const outPath = path.join(output, "params.json");
        fs.writeFileSync(rowPath, JSON.stringify(row));
        const root = path.resolve(__dirname, "../../..");
        execFileSync("python3", ["-c", sections[0][1], root, "fixture", "http://fixture", "http://server", "ws://server", rowPath, outPath, viewport, ""], { stdio: "pipe" });
        const bytes = fs.readFileSync(outPath, "utf8");
        return { bytes, params: JSON.parse(bytes) };
    } finally { fs.rmSync(output, { recursive: true, force: true }); }
}

function launcherServerArgv(syntheticSet, syntheticUnits) {
    const source = fs.readFileSync(path.join(__dirname, "../../../scripts/mc-browser.sh"), "utf8");
    const start = source.indexOf("SERVER_ARGS=(");
    const end = source.indexOf("PYSERVERARGV\n", start) + "PYSERVERARGV\n".length;
    assert(start >= 0 && end > start, "extract the actual production server argv array and recorder");
    const output = fs.mkdtempSync(path.join(os.tmpdir(), "mc-server-argv-"));
    try {
        execFileSync("bash", ["-c", source.slice(start, end)], {
            env: { ...process.env, TARGET: "/fixture/target", OUT: output, SYNTHETIC_SET: syntheticSet, SYNTHETIC_UNITS: syntheticUnits },
            stdio: "pipe",
        });
        return JSON.parse(fs.readFileSync(path.join(output, "server-argv.json"), "utf8"))
            .map((arg) => arg.startsWith(output + path.sep) ? `<OUT>/${arg.slice(output.length + 1)}` : arg);
    } finally { fs.rmSync(output, { recursive: true, force: true }); }
}

function launcherSyntheticUnits(row) {
    const source = fs.readFileSync(path.join(__dirname, "../../../scripts/mc-browser.sh"), "utf8");
    const block = source.match(/<<'PYSYNTHETIC'\n([\s\S]*?)\nPYSYNTHETIC/);
    assert(block, "extract the actual pre-build synthetic row validator");
    const root = path.resolve(__dirname, "../../..");
    const directory = fs.mkdtempSync(path.join(os.tmpdir(), "mc-synthetic-row-"));
    try {
        const rowPath = path.join(directory, "row.json");
        fs.writeFileSync(rowPath, JSON.stringify(row));
        const output = execFileSync("python3", ["-c", block[1], root, rowPath], {
            encoding: "utf8", stdio: ["pipe", "pipe", "pipe"],
        });
        return output.trim();
    } finally { fs.rmSync(directory, { recursive: true, force: true }); }
}

test("launcher carries the top-level checkpoint opt-in to the production harness params", () => {
    const { params } = launchParams({ row: "CAM1", capture_scene_checkpoints: true, windows: 2 });
    assert.equal(params.capture_scene_checkpoints, true);
    assert.equal(params.windows, 2);
    assert.equal(params.row, "CAM1");
    assert.deepEqual(params.viewport, { width: 1280, height: 720, device_scale_factor: 2 });
    assert.equal(params.device_scale_factor, 2);
    const legacyBytes = '{"capture_scene_checkpoints": true, "device_scale_factor": 2.0, "package": "fixture", "row": "CAM1", "server_url": "http://server", "url": "http://fixture", "viewport": {"device_scale_factor": 2.0, "height": 720, "width": 1280}, "windows": 1, "ws_url": "ws://server"}\n';
    assert.equal(launchParams({ row: "CAM1", capture_scene_checkpoints: true, windows: 1 }).bytes, legacyBytes);
});

test("launcher carries HUD fixture and stage metadata into readonly oracle params", () => {
    const fixture_catalog = { units: { worker: { content_id: "muster:worker" } } };
    const stages = [{ label: "select-worker", target: "worker" }];
    const { params } = launchParams({
        row: "HUD4",
        fixture_tree_sha256: "fixture-hash",
        fixture_catalog,
        pick_world_epsilon: 0.0001,
        stages,
    });
    assert.equal(params.fixture_tree_sha256, "fixture-hash");
    assert.deepEqual(params.fixture_catalog, fixture_catalog);
    assert.equal(params.pick_world_epsilon, 0.0001);
    assert.deepEqual(params.stages, stages);
});

test("ordinary rows keep checkpoint capture disabled", () => {
    assert.notEqual(launchParams({ row: "CLICK9" }).params.capture_scene_checkpoints, true);
    assert.equal(launchParams({ row: "CAM1", capture_scene_checkpoints: false }).params.capture_scene_checkpoints, false);
});

test("launcher rejects a non-boolean checkpoint setting instead of silently losing evidence", () => {
    assert.throws(() => launchParams({ row: "CAM1", capture_scene_checkpoints: "true" }), /must be a boolean/);
});

test("launcher retains a declared synthetic count in oracle params", () => {
    const { params } = launchParams({ row: "VIS2", synthetic_units: 4485 });
    assert.equal(params.synthetic_units, 4485);
});

test("the production params generator is byte-identical when synthetic_units is absent", () => {
    const base = launchParams({ row: "VIS2" });
    const absent = launchParams({ row: "VIS2", capture_scene_checkpoints: false });
    assert.equal(base.bytes, absent.bytes);
    assert.equal(Object.hasOwn(base.params, "synthetic_units"), false);
    const expected = '{"capture_scene_checkpoints": false, "device_scale_factor": 2.0, "package": "fixture", "row": "VIS2", "server_url": "http://server", "url": "http://fixture", "viewport": {"device_scale_factor": 2.0, "height": 720, "width": 1280}, "windows": 1, "ws_url": "ws://server"}\n';
    assert.equal(base.bytes, expected);
});

test("the production synthetic argument helper forwards 4485 as exactly two argv elements", () => {
    assert.equal(launcherSyntheticUnits({ row: "VIS2", synthetic_units: 4485 }), "4485");
    assert.deepEqual(launcherServerArgv("true", "4485"), ["/fixture/target/release/turts_server", "<OUT>/server.ron", "--synthetic-units", "4485"]);
    assert.deepEqual(launcherServerArgv("false", ""), ["/fixture/target/release/turts_server", "<OUT>/server.ron", "--tick-log", "<OUT>/server-tick.log"]);
});

test("synthetic_units rejects booleans, non-integers, negatives, and overflow", () => {
    for (const value of [true, false, 4485.0 + 0.5, -1, 4294967296, "4485", null]) {
        assert.throws(() => launcherSyntheticUnits({ synthetic_units: value }), { status: 2 }, `value ${JSON.stringify(value)}`);
    }
});

test("synthetic_units accepts the full unsigned 32-bit integer endpoints", () => {
    assert.equal(launcherSyntheticUnits({ synthetic_units: 0 }), "0");
    assert.deepEqual(launcherServerArgv("true", "0"), ["/fixture/target/release/turts_server", "<OUT>/server.ron", "--synthetic-units", "0", "--tick-log", "<OUT>/server-tick.log"]);
    assert.equal(launcherSyntheticUnits({ synthetic_units: 4294967295 }), "4294967295");
});
