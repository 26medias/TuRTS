// turts-test-role: unit
"use strict";

// The `node --test` step of .github/workflows/ci.yml lists the unit tests of tests/browser by name.
// A test that is not in the list runs in no gate: params-contract.test.js was red on the
// integration branch for that reason (L0-DBG re-check, finding L1). This test fails when a
// `*.test.js` file is missing from the list, or when the list names a file that does not exist.
// A test that cannot run in CI goes in LEFT_OUT, with its reason. The list is empty: every test
// passes with no browser (checked with Chrome off the PATH, L0-DBG follow-up); the two tests with
// a real-Chrome case (mp1-capture, wait-sync) skip that case when there is no Chrome binary.
const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const test = require("node:test");

const REPO = path.resolve(__dirname, "../../..");
const LEFT_OUT = {};

function testFiles(directory) {
    return fs.readdirSync(directory, { withFileTypes: true }).flatMap((entry) => {
        if (entry.name === "node_modules") return [];
        const full = path.join(directory, entry.name);
        if (entry.isDirectory()) return testFiles(full);
        return entry.name.endsWith(".test.js") ? [path.relative(REPO, full)] : [];
    });
}

// The files that the `node --test` line names (one line of the workflow).
function listedFiles(workflow) {
    const lines = workflow.split("\n").filter((line) => /^\s*run:\s*node --test\s/.test(line));
    assert.equal(lines.length, 1, "the workflow has one `node --test` step");
    return lines[0].replace(/^\s*run:\s*node --test\s+/, "").trim().split(/\s+/);
}

test("every unit test of tests/browser is in the CI node list, or is left out with a reason", () => {
    const listed = new Set(listedFiles(fs.readFileSync(path.join(REPO, ".github/workflows/ci.yml"), "utf8")));
    const missing = testFiles(path.join(REPO, "tests/browser")).filter((file) => !listed.has(file) && !(file in LEFT_OUT)).sort();
    assert.deepEqual(missing, []);
    const absent = [...listed].filter((file) => !fs.existsSync(path.join(REPO, file)));
    assert.deepEqual(absent, [], "the CI list names a file that does not exist");
    for (const [file, reason] of Object.entries(LEFT_OUT)) assert.ok(reason.length > 20, `${file} needs a reason`);
});

test("the list reader refuses a workflow with no node --test step", () => {
    assert.throws(() => listedFiles("jobs:\n  a:\n    steps:\n      - run: echo"), /one `node --test` step/);
    assert.deepEqual(listedFiles("      - name: x\n        run: node --test a.test.js b/c.test.js\n"), ["a.test.js", "b/c.test.js"]);
});
