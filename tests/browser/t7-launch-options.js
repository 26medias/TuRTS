// turts-test-role: pure
"use strict";

function parseBoolean(value, name, fallback) {
    if (value === undefined) return fallback;
    if (value === "true") return true;
    if (value === "false") return false;
    throw new Error(`${name} must be exactly \"true\" or \"false\"`);
}

function parseStringArray(value, name) {
    if (value === undefined) return undefined;
    let parsed;
    try {
        parsed = JSON.parse(value);
    } catch (error) {
        throw new Error(`${name} must be a JSON array of strings: ${error.message}`);
    }
    if (!Array.isArray(parsed) || !parsed.every((item) => typeof item === "string")) {
        throw new Error(`${name} must be a JSON array of strings`);
    }
    return parsed;
}

function browserLaunchOptions(env = process.env) {
    const options = { headless: parseBoolean(env.M2_HEADLESS, "M2_HEADLESS", true) };
    const args = parseStringArray(env.M2_CHROME_ARGS_JSON, "M2_CHROME_ARGS_JSON");
    const ignoreDefaultArgs = parseStringArray(
        env.M2_IGNORE_DEFAULT_ARGS_JSON,
        "M2_IGNORE_DEFAULT_ARGS_JSON",
    );
    if (args !== undefined) options.args = args;
    if (ignoreDefaultArgs !== undefined) options.ignoreDefaultArgs = ignoreDefaultArgs;
    return options;
}

module.exports = browserLaunchOptions;
