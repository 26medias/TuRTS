// turts-test-role: pure
"use strict";

// The oracle side of tests/m2/recipe_duration.py. The packed copy of the three economy cases can
// have a longer recipe on the hosted software renderer (M2_RECIPE_DURATION_SCALE). This check
// keeps the oracle honest: it takes the true recipe time from mods/ by itself, and it needs every
// job of the received world to have exactly the packed duration, in ticks, from the same formula
// (packed ms times tick_hz over 1000). A scale that was recorded but not applied, an edit with
// another factor, and a world with no job at all are all red.
const RECIPE_FILES = {
    "E9-browser-base-economy": { package: "base", file: "production/rifleman.ron" },
    "E9-browser-haul-economy": { package: "haul", file: "production/worker.ron" },
    "E9-browser-cedar-economy": { package: "cedar", file: "production/hire.ron" },
    // A loss case with a per-case factor (tests/m2/recipe_duration.py PER_CASE_RECIPES).
    "E9-browser-base-loss-refund": { package: "base", file: "production/rifleman.ron" },
};

function recipeFor(caseId) {
    return RECIPE_FILES[caseId] ?? null;
}

function parseDurationMs(text, where) {
    const found = [...text.matchAll(/duration_ms:\s*(\d+)\s*,/g)];
    if (found.length !== 1) throw new Error(`${where}: expected exactly one duration_ms, found ${found.length}`);
    return Number(found[0][1]);
}

function jobsOf(checkpoints) {
    const jobs = [];
    for (const shot of checkpoints) {
        for (const queue of shot.debug?.replicated_world?.economy?.queues ?? []) {
            for (const job of queue.jobs ?? []) jobs.push({ job, tickHz: shot.debug?.tick_hz });
        }
    }
    return jobs;
}

// sourceDurationMs: the duration_ms of the recipe file in mods/ (the caller reads the file).
function validateRecipeDuration({ packageName, caseId, receipt, sourceDurationMs, checkpoints }) {
    const recipe = recipeFor(caseId);
    if (!receipt || receipt.package !== packageName || receipt.case_id !== caseId) {
        throw new Error("M2 recipe duration receipt does not match the launched package and case");
    }
    if (!recipe) {
        if (receipt.applied !== false || receipt.scale !== 1) throw new Error(`case ${caseId} must keep the true recipe times`);
        return null;
    }
    if (recipe.package !== packageName || receipt.recipe_file !== recipe.file) {
        throw new Error(`M2 recipe duration receipt names ${receipt.recipe_file}, the oracle expects ${recipe.file}`);
    }
    if (!Number.isInteger(sourceDurationMs) || sourceDurationMs <= 0 || receipt.source_duration_ms !== sourceDurationMs) {
        throw new Error(`M2 recipe duration receipt source ${receipt.source_duration_ms} ms differs from mods/ ${sourceDurationMs} ms`);
    }
    if (!Number.isInteger(receipt.scale) || receipt.scale < 1 || receipt.applied !== (receipt.scale !== 1)) {
        throw new Error(`M2 recipe duration receipt has an invalid scale ${receipt.scale} or applied flag ${receipt.applied}`);
    }
    const packedMs = sourceDurationMs * receipt.scale;
    if (receipt.packed_duration_ms !== packedMs) {
        throw new Error(`M2 recipe duration receipt packed ${receipt.packed_duration_ms} ms is not ${sourceDurationMs} ms times ${receipt.scale}`);
    }
    const jobs = jobsOf(checkpoints);
    if (jobs.length === 0) throw new Error("M2 recipe duration check saw no production job in the received world");
    for (const { job, tickHz } of jobs) {
        if (!Number.isInteger(tickHz) || tickHz <= 0 || (packedMs * tickHz) % 1000 !== 0) {
            throw new Error(`M2 recipe duration check needs a whole tick count: ${packedMs} ms at ${tickHz} Hz`);
        }
        const expected = (packedMs * tickHz) / 1000;
        if (job.duration_ticks !== expected) {
            throw new Error(`job ${job.id} has duration_ticks ${job.duration_ticks}, the packed recipe needs ${expected} ` +
                `(${sourceDurationMs} ms times ${receipt.scale} at ${tickHz} Hz)`);
        }
    }
    return { scale: receipt.scale, packed_duration_ms: packedMs, jobs_checked: jobs.length };
}

module.exports = { RECIPE_FILES, recipeFor, parseDurationMs, validateRecipeDuration };
