// turts-test-role: harness
"use strict";

// Only the harness keeps these observations. Driver IO still returns PNG bytes alone.
function createSceneCheckpoints(pages, limit = 256) {
    const rows = [];
    async function record(metadata) {
        if (!Number.isSafeInteger(metadata.input_call) || metadata.input_call < 0 ||
            !Number.isSafeInteger(metadata.window) || metadata.window < 1 || metadata.window > pages.length ||
            !Number.isFinite(metadata.wall_time_ms)) throw new Error("invalid scene checkpoint identity");
        if (rows.length >= limit) throw new Error("scene checkpoint limit exceeded");
        const raw = await pages[metadata.window - 1].evaluate(() => ({
            scene: typeof window.turts_scene === "function" ? window.turts_scene() : null,
            debug: typeof window.turts_debug === "function" ? window.turts_debug() : null,
        }));
        const parse = (value, name) => {
            const parsed = typeof value === "string" ? JSON.parse(value) : value;
            if (!parsed || typeof parsed !== "object" || Array.isArray(parsed)) throw new Error(`missing ${name} checkpoint bridge`);
            return parsed;
        };
        rows.push({ ...metadata, scene: parse(raw.scene, "scene"), debug: parse(raw.debug, "debug") });
    }
    return { record, snapshots: () => structuredClone(rows) };
}

module.exports = { createSceneCheckpoints };
