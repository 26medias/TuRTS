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

function validateFixtureActions(fixtureRoot, relative) {
    const script = `import json,pathlib,sys\nsys.path.insert(0,sys.argv[1])\nfrom mc_browser_params import actions_file\ntry: print(json.dumps(actions_file({"actions_file":sys.argv[3]},pathlib.Path(sys.argv[2]))[1]))\nexcept ValueError as error: print(error); raise SystemExit(2)`;
    try {
        return execFileSync("python3", ["-c", script, path.resolve(__dirname, "../../../scripts"), fixtureRoot, relative], {
            encoding: "utf8", env: { ...process.env, PYTHONDONTWRITEBYTECODE: "1" },
        });
    } catch (error) {
        throw new Error([error.stdout?.trim(), error.stderr?.trim(), error.message].filter(Boolean).join("\n"));
    }
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

test("launcher carries explicit MM1 public map and pixel-oracle parameters", () => {
    const minimapParams = {
        map_size: [64, 48],
        camera_vertical_fov_degrees: 60,
        slot_colours: ["#0072B2", "#E69F00"],
        ping_colour: "#ffcc00",
        frame_colour: "#ffffff",
        background_colour: "#17232f",
        minimap_inset: 8,
        minimap_size: 200,
    };
    const { params } = launchParams({ row: "MM1", ...minimapParams });
    for (const [key, value] of Object.entries(minimapParams)) assert.deepEqual(params[key], value);
});

test("the actual MM1 row produces finite authored viewport and minimap camera targets", () => {
    const rowPath = path.join(__dirname, "../rows/MM1.json");
    const row = JSON.parse(fs.readFileSync(rowPath, "utf8"));
    const { params } = launchParams(row, "1280x720");
    assert.deepEqual(params.map_size, [64, 48]);
    assert.deepEqual(params.viewport, { width: 1280, height: 720, device_scale_factor: 1 });
    assert.deepEqual([params.minimap_inset, params.minimap_size], [8, 200]);
    assert.deepEqual(params.slot_colours, ["#0072B2", "#E69F00"]);
    assert.deepEqual([params.ping_colour, params.frame_colour, params.background_colour], ["#ffcc00", "#ffffff", "#17232f"]);
    const rect = { x: params.viewport.width - params.minimap_inset - params.minimap_size,
        y: params.viewport.height - params.minimap_inset - params.minimap_size, size: params.minimap_size };
    for (const [u, v] of [[0.87, 0.28], [0.18, 0.22], [0.28, 0.72], [0.72, 0.32], [0.95, 0.95]]) {
        assert(Number.isFinite(rect.x + rect.size * u));
        assert(Number.isFinite(rect.y + rect.size * v));
    }
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

test("launcher loads a fixture-contained actions_file as immutable harness data with path and digest", () => {
    const { params } = launchParams({ row: "HD1", actions_file: "hd1-actions.json" });
    assert.equal(params.actions_file, "hd1-actions.json");
    assert.match(params.actions_sha256, /^[a-f0-9]{64}$/);
    assert.ok(Array.isArray(params.actions) && params.actions.length > 0);
});

test("launcher rejects actions_file traversal, inline conflicts, and malformed action lists", () => {
    assert.throws(() => launchParams({ actions_file: "../../outside.json" }), /actions_file/);
    assert.throws(() => launchParams({ actions_file: "hd1-actions.json", actions: [] }), /inline actions/);
    assert.throws(() => launchParams({ actions_file: "missing-actions.json" }), /actions_file/);
});

test("actions_file validator rejects non-lists, malformed action ticks, and symlinks outside fixtures", () => {
    const root = fs.mkdtempSync(path.join(os.tmpdir(), "hd1-actions-") );
    const outside = fs.mkdtempSync(path.join(os.tmpdir(), "hd1-outside-") );
    try {
        fs.writeFileSync(path.join(root, "object.json"), JSON.stringify({ tick: 1 }));
        fs.writeFileSync(path.join(root, "bad-tick.json"), JSON.stringify([{ tick: true, do: "order" }]));
        fs.writeFileSync(path.join(root, "null-tick.json"), JSON.stringify([{ tick: null, do: "order" }]));
        fs.writeFileSync(path.join(root, "overflow-tick.json"), JSON.stringify([{ tick: 4294967296, do: "order" }]));
        fs.writeFileSync(path.join(root, "max-tick.json"), JSON.stringify([{ tick: 4294967295, do: "order", order: "muster:order_stop" }]));
        fs.writeFileSync(path.join(root, "mixed-ticks.json"), JSON.stringify([
            { tick: 10, do: "order", order: "muster:order_move", world_point: [2, 3] },
            { do: "order", order: "muster:order_stop" },
            { tick: 12, do: "order", order: "muster:order_move", world_point: [4, 5] },
        ]));
        fs.writeFileSync(path.join(root, "omitted-ticks.json"), JSON.stringify([
            { do: "order", order: "muster:order_stop" },
            { do: "order", order: "muster:order_hold" },
        ]));
        fs.writeFileSync(path.join(outside, "actions.json"), "[]");
        fs.symlinkSync(path.join(outside, "actions.json"), path.join(root, "escape.json"));
        assert.throws(() => validateFixtureActions(root, "object.json"), /JSON action list/);
        assert.throws(() => validateFixtureActions(root, "bad-tick.json"), /tick must be an integer in 0\.\.=/);
        assert.throws(() => validateFixtureActions(root, "null-tick.json"), /tick must be an integer in 0\.\.=/);
        assert.throws(() => validateFixtureActions(root, "overflow-tick.json"), /tick must be an integer in 0\.\.=/);
        assert.throws(() => validateFixtureActions(root, "escape.json"), /under tests\/browser\/fixtures/);
        assert.deepEqual(JSON.parse(validateFixtureActions(root, "mixed-ticks.json" )).map((action) => action.tick), [10, 10, 12]);
        assert.deepEqual(JSON.parse(validateFixtureActions(root, "omitted-ticks.json" )).map((action) => action.tick), [0, 0]);
        assert.deepEqual(JSON.parse(validateFixtureActions(root, "max-tick.json" )).map((action) => action.tick), [4294967295]);
    } finally {
        fs.rmSync(root, { recursive: true, force: true });
        fs.rmSync(outside, { recursive: true, force: true });
    }
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
