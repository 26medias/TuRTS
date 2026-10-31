// turts-test-role: unit
"use strict";

const assert = require("node:assert/strict");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");
const { spawnSync } = require("node:child_process");
const test = require("node:test");

function runProfile(mode, runs = "ci", allowHeaded = "0") {
    const root = fs.mkdtempSync(path.join(os.tmpdir(), "turts-pixel-profile-"));
    try {
        fs.mkdirSync(path.join(root, "scripts"), { recursive: true });
        fs.mkdirSync(path.join(root, "tests/browser/rows"), { recursive: true });
        fs.copyFileSync(path.join(__dirname, "../../../scripts/mc-pixels.sh"), path.join(root, "scripts/mc-pixels.sh"));
        fs.writeFileSync(path.join(root, "scripts/mc-browser.sh"), "#!/bin/sh\nprintf '%s\\n' \"$MC_BROWSER_HEADED\" >> \"$MC_PROFILE_LOG\"\n");
        fs.writeFileSync(path.join(root, "tests/browser/rows/PROFILE.json"), JSON.stringify({
            row: "PROFILE", package: "mods/acme", driver: "driver.js", oracle: "oracle.js",
            runs, viewports: ["1280x720"],
        }));
        const log = path.join(root, "modes.log");
        const result = spawnSync("bash", [path.join(root, "scripts/mc-pixels.sh"), "tests/browser/rows/PROFILE.json"], {
            encoding: "utf8", env: { ...process.env, MC_BROWSER_HEADED: mode, MC_PIXELS_HEADED: allowHeaded, MC_PROFILE_LOG: log, DISPLAY: ":fixture" },
        });
        return { status: result.status, stdout: result.stdout, stderr: result.stderr, modes: fs.existsSync(log) ? fs.readFileSync(log, "utf8").trim().split("\n") : [] };
    } finally {
        fs.rmSync(root, { recursive: true, force: true });
    }
}

test("CI rows default to headless and still run twice", () => {
    const result = runProfile("");
    assert.equal(result.status, 0);
    assert.deepEqual(result.modes, ["0", "0"]);
    assert.match(result.stdout, /PASS PROFILE \(2 runs per viewport\)/);
});

test("explicit headed profile runs CI rows twice without a headed-row skip", () => {
    const result = runProfile("1");
    assert.equal(result.status, 0);
    assert.deepEqual(result.modes, ["1", "1"]);
    assert.doesNotMatch(result.stdout, /SKIP/);
    assert.match(result.stdout, /PASS PROFILE/);
});

test("headed-only rows retain their skip and required-headed behavior", () => {
    const skipped = runProfile("1", "headed");
    assert.equal(skipped.status, 0);
    assert.deepEqual(skipped.modes, []);
    assert.match(skipped.stdout, /SKIP PROFILE headed/);
    assert.doesNotMatch(skipped.stdout, /PASS PROFILE/);
    assert.deepEqual(runProfile("0", "headed", "1").modes, ["1", "1"]);
});

test("invalid explicit profile is rejected before a browser invocation", () => {
    const result = runProfile("invalid");
    assert.equal(result.status, 2);
    assert.deepEqual(result.modes, []);
    assert.match(result.stderr, /MC_BROWSER_HEADED must be 0 or 1/);
});
