// turts-test-role: unit
"use strict";

const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const { test } = require("node:test");
const { RECIPE_FILES, parseDurationMs, validateRecipeDuration } = require("./harness/m2-recipe.js");

const ROOT = path.join(__dirname, "..", "..");
const shot = (jobs, tickHz = 20) => ({ debug: { tick_hz: tickHz, replicated_world: { economy: { queues: [{ producer: 1, jobs }] } } } });
const job = (ticks, id = 1) => ({ id, recipe: "base:train_rifleman", stage: "running", duration_ticks: ticks });
const receipt = (over = {}) => ({ package: "base", case_id: "E9-browser-base-economy", scale_requested: 4, scale: 4, applied: true,
    recipe_file: "production/rifleman.ron", source_duration_ms: 5000, packed_duration_ms: 20000, ...over });
const check = (r, checkpoints, over = {}) => validateRecipeDuration({ packageName: "base", caseId: "E9-browser-base-economy",
    receipt: r, sourceDurationMs: 5000, checkpoints, ...over });

test("scale 4 with a job of 400 ticks is accepted, and scale 1 with 100 ticks", () => {
    assert.deepEqual(check(receipt(), [shot([job(400)])]), { scale: 4, packed_duration_ms: 20000, jobs_checked: 1 });
    const one = receipt({ scale: 1, scale_requested: 1, applied: false, packed_duration_ms: 5000 });
    assert.equal(check(one, [shot([job(100)])]).scale, 1);
});

test("a scale that was recorded but not applied is red (the job keeps 100 ticks)", () => {
    assert.throws(() => check(receipt(), [shot([job(100)])]), /duration_ticks 100, the packed recipe needs 400/);
});

test("another factor than the recorded one is red", () => {
    assert.throws(() => check(receipt(), [shot([job(300)])]), /needs 400/);
    assert.throws(() => check(receipt(), [shot([job(400), job(100, 2)])]), /job 2 has duration_ticks 100/);
});

test("a receipt that is not the source times the scale is red", () => {
    assert.throws(() => check(receipt({ packed_duration_ms: 25000 }), [shot([job(500)])]), /is not 5000 ms times 4/);
    assert.throws(() => check(receipt({ source_duration_ms: 4000, packed_duration_ms: 16000 }), [shot([job(320)])]), /differs from mods/);
    assert.throws(() => check(receipt({ scale: 1, applied: true, packed_duration_ms: 5000 }), [shot([job(100)])]), /invalid scale/);
});

test("a world with no job is red, it is not an empty pass", () => {
    assert.throws(() => check(receipt(), [shot([])]), /saw no production job/);
    assert.throws(() => check(receipt(), []), /saw no production job/);
});

test("the receipt must name the launched case and the recipe the oracle expects", () => {
    assert.throws(() => check(receipt({ case_id: "E9-browser-haul-economy" }), [shot([job(400)])]), /does not match/);
    assert.throws(() => check(receipt({ recipe_file: "production/harvester.ron" }), [shot([job(400)])]), /expects production\/rifleman\.ron/);
    assert.throws(() => check(null, [shot([job(400)])]), /does not match/);
});

test("a case that is not an economy case must keep the true times", () => {
    const base = { packageName: "base", caseId: "E9-browser-base-recovery", sourceDurationMs: null, checkpoints: [] };
    const none = { package: "base", case_id: "E9-browser-base-recovery", scale_requested: 4, scale: 1, applied: false };
    assert.equal(validateRecipeDuration({ ...base, receipt: none }), null);
    assert.throws(() => validateRecipeDuration({ ...base, receipt: { ...none, scale: 4, applied: true } }), /true recipe times/);
});

test("cedar and haul: 6000 ms times 4 is 480 ticks at 20 Hz", () => {
    for (const [pkg, file] of [["cedar", "production/hire.ron"], ["haul", "production/worker.ron"]]) {
        const caseId = `E9-browser-${pkg}-economy`;
        const r = { package: pkg, case_id: caseId, scale_requested: 4, scale: 4, applied: true, recipe_file: file,
            source_duration_ms: 6000, packed_duration_ms: 24000 };
        validateRecipeDuration({ packageName: pkg, caseId, receipt: r, sourceDurationMs: 6000, checkpoints: [shot([job(480)])] });
        assert.throws(() => validateRecipeDuration({ packageName: pkg, caseId, receipt: r, sourceDurationMs: 6000,
            checkpoints: [shot([job(120)])] }), /needs 480/);
    }
});

test("the true recipe times of mods/ and the recipe table agree with tests/m2/recipe_duration.py", () => {
    const python = fs.readFileSync(path.join(__dirname, "..", "m2", "recipe_duration.py"), "utf8");
    const pairs = [...python.matchAll(/"(E9-browser-[a-z_-]+)": \("([a-z]+)", "([a-z/._]+)"\)/g)];
    assert.equal(pairs.length, 4);
    assert.deepEqual(Object.fromEntries(pairs.map(([, id, pkg, file]) => [id, { package: pkg, file }])), RECIPE_FILES);
    for (const [id, { package: pkg, file }] of Object.entries(RECIPE_FILES)) {
        const ms = parseDurationMs(fs.readFileSync(path.join(ROOT, "mods", pkg, file), "utf8"), file);
        assert.equal(ms, pkg === "base" ? 5000 : 6000, id);
    }
});

const REFUND = "E9-browser-base-loss-refund";
const refundReceipt = (over = {}) => ({ package: "base", case_id: REFUND, scale_requested: 5, scale: 5, applied: true,
    recipe_file: "production/rifleman.ron", source_duration_ms: 5000, packed_duration_ms: 25000, ...over });
const checkRefund = (r, checkpoints) => validateRecipeDuration({ packageName: "base", caseId: REFUND, receipt: r, sourceDurationMs: 5000, checkpoints });

test("loss-refund: scale 5 with a job of 500 ticks is accepted", () => {
    assert.deepEqual(checkRefund(refundReceipt(), [shot([job(500)])]), { scale: 5, packed_duration_ms: 25000, jobs_checked: 1 });
    const one = refundReceipt({ scale: 1, scale_requested: 1, applied: false, packed_duration_ms: 5000 });
    assert.equal(checkRefund(one, [shot([job(100)])]).scale, 1);
});

test("loss-refund: a claimed scale with an unscaled pack is red, and the reverse is red", () => {
    assert.throws(() => checkRefund(refundReceipt(), [shot([job(100)])]), /duration_ticks 100, the packed recipe needs 500/);
    const unclaimed = refundReceipt({ scale: 1, scale_requested: 1, applied: false, packed_duration_ms: 5000 });
    assert.throws(() => checkRefund(unclaimed, [shot([job(500)])]), /duration_ticks 500, the packed recipe needs 100/);
    assert.throws(() => checkRefund(refundReceipt({ scale: 4, packed_duration_ms: 20000 }), [shot([job(500)])]), /needs 400/);
});

test("the other loss cases and the recovery case must keep the true times", () => {
    for (const caseId of ["E9-browser-base-loss-conversion", "E9-browser-base-loss-charge", "E9-browser-base-loss-spawn",
        "E9-browser-base-loss-final_change", "E9-browser-base-recovery"]) {
        const none = { package: "base", case_id: caseId, scale_requested: 5, scale: 1, applied: false };
        assert.equal(validateRecipeDuration({ packageName: "base", caseId, receipt: none, sourceDurationMs: null, checkpoints: [] }), null);
        assert.throws(() => validateRecipeDuration({ packageName: "base", caseId, receipt: { ...none, scale: 5, applied: true },
            sourceDurationMs: null, checkpoints: [] }), /true recipe times/, caseId);
    }
});
