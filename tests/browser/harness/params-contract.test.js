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
