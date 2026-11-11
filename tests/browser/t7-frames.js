// turts-test-role: instrument
"use strict";

// Usage: node tests/browser/t7-frames.js <run dir>
// Writes <run dir>/frames.jsonl and <run dir>/result.json from <run dir>/scene-checkpoints.json
// for the Rust segment oracle (crates/turts_server/tests/t7_oracle.rs). The conversion is
// t7FramesFromCheckpoints() of t7-oracle.js, so the oracle and this file count the same frames.
const fs = require("node:fs");
const path = require("node:path");
const { t7FramesFromCheckpoints } = require("./t7-oracle.js");

const [dir] = process.argv.slice(2);
if (!dir) {
    console.error("usage: node tests/browser/t7-frames.js <run dir>");
    process.exit(2);
}
const checkpoints = JSON.parse(fs.readFileSync(path.join(dir, "scene-checkpoints.json"), "utf8"));
const { frames, result } = t7FramesFromCheckpoints(checkpoints);
fs.writeFileSync(path.join(dir, "frames.jsonl"), frames.join("\n") + "\n");
fs.writeFileSync(path.join(dir, "result.json"), JSON.stringify(result, null, 2) + "\n");
console.log(`T7_FRAMES ${JSON.stringify(result)}`);
