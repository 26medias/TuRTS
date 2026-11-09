// turts-test-role: unit
"use strict";

const test = require("node:test");
const assert = require("node:assert/strict");
const { buildVis2LaunchOptions } = require("./measure-launch-options.js");

const RUNNER_ARGS = [
    "--enable-gpu",
    "--use-angle=vulkan",
    "--enable-features=Vulkan",
    "--enable-unsafe-webgpu",
];
const RUNNER_IGNORE_DEFAULT_ARGS = ["--enable-unsafe-swiftshader"];

test("VIS2 uses the bound headed Vulkan profile and excludes SwiftShader defaults", () => {
    assert.deepEqual(
        buildVis2LaunchOptions("/chrome", [], {
            MC_CHROME_ARGS_JSON: JSON.stringify(RUNNER_ARGS),
            MC_CHROME_IGNORE_DEFAULT_ARGS_JSON: JSON.stringify(RUNNER_IGNORE_DEFAULT_ARGS),
        }),
        {
            executablePath: "/chrome",
            headless: false,
            args: RUNNER_ARGS,
            ignoreDefaultArgs: RUNNER_IGNORE_DEFAULT_ARGS,
        },
    );
});

test("VIS2 defaults to the headed Vulkan profile and SwiftShader exclusion", () => {
    assert.deepEqual(
        buildVis2LaunchOptions("/chrome", [], {}),
        {
            executablePath: "/chrome",
            headless: false,
            args: RUNNER_ARGS,
            ignoreDefaultArgs: RUNNER_IGNORE_DEFAULT_ARGS,
        },
    );
});

test("VIS2 keeps explicit CLI flags and avoids duplicate environment flags", () => {
    assert.deepEqual(
        buildVis2LaunchOptions("/chrome", ["--trace-startup", "--enable-gpu"], {
            MC_CHROME_ARGS_JSON: JSON.stringify(RUNNER_ARGS),
            MC_CHROME_IGNORE_DEFAULT_ARGS_JSON: JSON.stringify(RUNNER_IGNORE_DEFAULT_ARGS),
        }),
        {
            executablePath: "/chrome",
            headless: false,
            args: [...RUNNER_ARGS, "--trace-startup"],
            ignoreDefaultArgs: RUNNER_IGNORE_DEFAULT_ARGS,
        },
    );
});

test("VIS2 rejects malformed argument environment values before launch", () => {
    assert.throws(
        () => buildVis2LaunchOptions("/chrome", [], { MC_CHROME_ARGS_JSON: "{}" }),
        /MC_CHROME_ARGS_JSON must be a JSON array of strings/,
    );
    assert.throws(
        () => buildVis2LaunchOptions("/chrome", [], { MC_CHROME_IGNORE_DEFAULT_ARGS_JSON: "[false]" }),
        /MC_CHROME_IGNORE_DEFAULT_ARGS_JSON must be a JSON array of strings/,
    );
});
