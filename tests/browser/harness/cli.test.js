// turts-test-role: unit
"use strict";

const assert = require("node:assert/strict");
const test = require("node:test");
const { parseArgs } = require("./run.js");

test("run.js reads params from --params and parses optional device scale", () => {
    assert.deepEqual(parseArgs([
        "http://127.0.0.1:9000/?server=x&ws=y", "/tmp/run", "/tmp/driver.js", "/tmp/oracle.js",
        "--params", "/tmp/params.json", "--viewport", "1280x720@2",
    ]), {
        url: "http://127.0.0.1:9000/?server=x&ws=y",
        outDir: "/tmp/run",
        driverPath: "/tmp/driver.js",
        oraclePath: "/tmp/oracle.js",
        paramsPath: "/tmp/params.json",
        width: 1280,
        height: 720,
        deviceScaleFactor: 2,
        windows: null,
    });
});

test("run.js accepts a positive --windows count", () => {
    assert.equal(parseArgs([
        "http://127.0.0.1:9000/", "/tmp/run", "/tmp/driver.js", "/tmp/oracle.js",
        "--params", "/tmp/params.json", "--viewport", "1280x720", "--windows", "2",
    ]).windows, 2);
});

test("run.js rejects missing params and malformed viewport arguments", () => {
    assert.throws(() => parseArgs(["url", "out", "driver", "oracle", "--viewport", "1280x720"]), /both --params and --viewport/);
    assert.throws(() => parseArgs(["url", "out", "driver", "oracle", "--params", "p", "--viewport", "bad"]), /invalid viewport/);
    assert.throws(() => parseArgs(["url", "out", "driver", "oracle", "--params", "p", "--viewport", "1280x720", "--windows"]), /--windows requires a positive integer/);
    for (const count of ["0", "-1", "1.5", "many", "9007199254740992"]) {
        assert.throws(() => parseArgs(["url", "out", "driver", "oracle", "--params", "p", "--viewport", "1280x720", "--windows", count]), /--windows requires a positive integer/);
    }
});
