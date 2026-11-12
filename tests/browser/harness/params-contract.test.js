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

// The two places where scripts/mc-browser.sh sets the server binary: from TARGET (the build inside
// the run) and from PREPARED (a prepared folder, Q-PREBUILD). The argv recorder below uses
// $SERVER_BIN, so each test runs ONE of the two assignments of the script, taken from the script
// text, before the recorder. SERVER_BIN is not given to the recorder from the test environment:
// that would make argv[0] a value of the test (L0-DBG follow-up, finding L1).
// The first is the one line of the script that starts at column 0; the second is the one indented
// line (inside `if [[ -n "$PREPARED" ]]`). Each must exist exactly once, so a moved or doubled
// assignment makes the test red instead of leaving a path unchecked.
const SERVER_BIN_ASSIGNMENT = {
    target: /^SERVER_BIN="[^"\n]*"$/gm,
    prepared: /^[ \t]+SERVER_BIN="[^"\n]*"$/gm,
};
const SERVER_BIN_EXPECTED = { target: "/fixture/target/release/turts_server", prepared: "/fixture/prepared/bin/turts_server" };

function serverBinAssignment(source, branch) {
    const matches = source.match(SERVER_BIN_ASSIGNMENT[branch]) ?? [];
    assert.equal(matches.length, 1, `scripts/mc-browser.sh has ${matches.length} SERVER_BIN assignments for the ${branch} branch, not one`);
    return matches[0].trim();
}

function launcherServerArgv(syntheticSet, syntheticUnits, orderLogSet = "false", branch = "target") {
    const source = fs.readFileSync(path.join(__dirname, "../../../scripts/mc-browser.sh"), "utf8");
    const start = source.indexOf("SERVER_ARGS=(");
    const end = source.indexOf("PYSERVERARGV\n", start) + "PYSERVERARGV\n".length;
    assert(start >= 0 && end > start, "extract the actual production server argv array and recorder");
    const output = fs.mkdtempSync(path.join(os.tmpdir(), "mc-server-argv-"));
    try {
        const env = { ...process.env, TARGET: "/fixture/target", PREPARED: "/fixture/prepared", OUT: output, SYNTHETIC_SET: syntheticSet,
            SYNTHETIC_UNITS: syntheticUnits, ORDER_LOG_SET: orderLogSet };
        delete env.SERVER_BIN;
        execFileSync("bash", ["-c", `${serverBinAssignment(source, branch)}\n${source.slice(start, end)}`], { env, stdio: "pipe" });
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
        const output = execFileSync("python3", ["-c", block[1], root, rowPath, row.package || "mods/acme"], {
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

test("GM40 order-log opt-in forwards only the actual server log path and readonly oracle contract", () => {
    const row = { row: "GM40", capture_scene_checkpoints: true, capture_order_log: true };
    const { params } = launchParams(row);
    assert.equal(params.capture_order_log, true);
    assert.equal(params.order_log_file, "server-order.log");
    assert.deepEqual(launcherServerArgv("false", "", "true"), [
        "/fixture/target/release/turts_server", "<OUT>/server.ron",
        "--tick-log", "<OUT>/server-tick.log", "--order-log", "<OUT>/server-order.log",
    ]);
});

test("GM40 order-log opt-in rejects wrong row, wrong type, and missing scene checkpoints", () => {
    assert.throws(() => launchParams({ row: "GMU", capture_scene_checkpoints: true, capture_order_log: true }), /only for row GM40/);
    assert.throws(() => launchParams({ row: "GM40", capture_scene_checkpoints: true, capture_order_log: "true" }), /must be a boolean/);
    assert.throws(() => launchParams({ row: "GM40", capture_scene_checkpoints: false, capture_order_log: true }), /requires capture_scene_checkpoints=true/);
});

test("rows without the GM40 order-log opt-in preserve the exact previous server argv and params", () => {
    const before = launchParams({ row: "CAM1", capture_scene_checkpoints: true, windows: 1 });
    const after = launchParams({ row: "CAM1", capture_scene_checkpoints: true, windows: 1, capture_order_log: false });
    assert.equal(after.bytes, before.bytes);
    assert.equal(Object.hasOwn(after.params, "order_log_file"), false);
    assert.deepEqual(launcherServerArgv("false", ""), ["/fixture/target/release/turts_server", "<OUT>/server.ron", "--tick-log", "<OUT>/server-tick.log"]);
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

test("MC task 21: a row with effective_bindings gets params.bindings from the packed copy and keeps checkpoint_limit", () => {
    const source = fs.readFileSync(path.join(__dirname, "../../../scripts/mc-browser.sh"), "utf8");
    const block = source.match(/<<'PYPARAMS'\n([\s\S]*?)\nPYPARAMS/)[1];
    const output = fs.mkdtempSync(path.join(os.tmpdir(), "mc-params-bindings-"));
    try {
        const root = path.resolve(__dirname, "../../..");
        const rowPath = path.join(output, "row.json");
        const outPath = path.join(output, "params.json");
        const bindings = { actions: { key_stop: [{ Key: { code: "KeyS", mods: [] } }] }, orders: {} };
        fs.writeFileSync(rowPath, JSON.stringify({ row: "PT1", windows: 2, effective_bindings: true, checkpoint_limit: 900, stages: { click_to_play: [500, 430] } }));
        fs.writeFileSync(path.join(output, "effective-bindings.json"), JSON.stringify(bindings));
        const run = () => execFileSync("python3", ["-c", block, root, "forge", "http://fixture", "http://server", "ws://server", rowPath, outPath, "1280x720", ""], { stdio: "pipe" });
        run();
        const params = JSON.parse(fs.readFileSync(outPath, "utf8"));
        assert.deepEqual(params.bindings, bindings);
        assert.equal(params.checkpoint_limit, 900);
        assert.equal(params.windows, 2);
        fs.rmSync(path.join(output, "effective-bindings.json"));
        assert.throws(run, "a row that asks for bindings must not start without the effective map");
        fs.writeFileSync(rowPath, JSON.stringify({ row: "PT1", windows: 2 }));
        run();
        assert.equal(Object.hasOwn(JSON.parse(fs.readFileSync(outPath, "utf8")), "bindings"), false);
    } finally { fs.rmSync(output, { recursive: true, force: true }); }
});

test("argv[0] is the server binary of the TARGET branch and of the prepared branch of scripts/mc-browser.sh", () => {
    for (const branch of ["target", "prepared"]) {
        assert.equal(launcherServerArgv("false", "", "false", branch)[0], SERVER_BIN_EXPECTED[branch], branch);
    }
});

test("an ambient SERVER_BIN does not reach the recorder, and a script with no assignment is refused", () => {
    process.env.SERVER_BIN = "/ambient/turts_server";
    try { assert.equal(launcherServerArgv("false", "")[0], SERVER_BIN_EXPECTED.target); } finally { delete process.env.SERVER_BIN; }
    assert.throws(() => serverBinAssignment("SERVER_ARGS=(x)", "target"), /0 SERVER_BIN assignments for the target branch/);
    assert.throws(() => serverBinAssignment('SERVER_BIN="a"\nSERVER_BIN="b"', "target"), /2 SERVER_BIN assignments/);
    assert.equal(serverBinAssignment('x\nSERVER_BIN="$TARGET/release/other"\n  SERVER_BIN="p"', "target"), 'SERVER_BIN="$TARGET/release/other"');
});
