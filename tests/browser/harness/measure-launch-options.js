// turts-test-role: pure
"use strict";

const DEFAULT_VIS2_CHROME_ARGS = [
    "--enable-gpu",
    "--use-angle=vulkan",
    "--enable-features=Vulkan",
    "--enable-unsafe-webgpu",
];
const DEFAULT_VIS2_IGNORE_DEFAULT_ARGS = ["--enable-unsafe-swiftshader"];

function parseStringArray(raw, name, fallback) {
    if (raw === undefined) return [...fallback];
    let parsed;
    try {
        parsed = JSON.parse(raw);
    } catch (error) {
        throw new Error(`${name} must be a JSON array of strings`, { cause: error });
    }
    if (!Array.isArray(parsed) || !parsed.every((value) => typeof value === "string")) {
        throw new Error(`${name} must be a JSON array of strings`);
    }
    return parsed;
}

function buildVis2LaunchOptions(executablePath, extraArgs, env) {
    if (!Array.isArray(extraArgs) || !extraArgs.every((value) => typeof value === "string")) {
        throw new Error("VIS2 extra Chrome arguments must be strings");
    }
    const configuredArgs = parseStringArray(
        env.MC_CHROME_ARGS_JSON,
        "MC_CHROME_ARGS_JSON",
        DEFAULT_VIS2_CHROME_ARGS,
    );
    const configuredIgnoreDefaultArgs = parseStringArray(
        env.MC_CHROME_IGNORE_DEFAULT_ARGS_JSON,
        "MC_CHROME_IGNORE_DEFAULT_ARGS_JSON",
        DEFAULT_VIS2_IGNORE_DEFAULT_ARGS,
    );
    return {
        executablePath,
        headless: false,
        args: [...new Set([...configuredArgs, ...extraArgs])],
        ignoreDefaultArgs: [...new Set(configuredIgnoreDefaultArgs)],
    };
}

module.exports = { buildVis2LaunchOptions };
