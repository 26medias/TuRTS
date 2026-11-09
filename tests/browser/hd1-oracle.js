// turts-test-role: oracle
"use strict";

const assert = require("node:assert/strict");
const { createHash } = require("node:crypto");

function sha256(value) {
    return createHash("sha256").update(value).digest("hex");
}

function sortedIds(value) {
    assert(Array.isArray(value), "unit id set is an array");
    return value.slice().sort((a, b) => a - b);
}

function normalizedResults(results) {
    assert(Array.isArray(results), "real per-unit order results are present");
    return results.map((result) => {
        assert(Number.isInteger(result.seq) && Number.isInteger(result.unit), "result has integer sequence and unit ids");
        assert(["accepted", "refused"].includes(result.status), "result status is accepted or refused");
        return [result.seq, result.unit, result.status];
    }).sort((a, b) => a[0] - b[0] || a[1] - b[1] || a[2].localeCompare(b[2]));
}

async function judge(view, params) {
    const input = view.inputLog();
    assert(input.length > 0 && input.every((entry) => ["click", "drag", "key", "wait", "screenshot", "save"].includes(entry.call)),
        "HD1 driver uses only recorded physical IO");

    const nativeRaw = await view.serverFile("native-reference/native-report.json");
    const bindingRaw = await view.serverFile("native-reference/binding.json");
    const native = JSON.parse(nativeRaw);
    const binding = JSON.parse(bindingRaw);
    assert.equal(sha256(nativeRaw), binding.native_report_sha256, "browser compares the bound native report bytes");
    assert.equal(binding.execution_evidence?.native_process_status, "exit=0", "the bound native report came from the successful named process test");
    assert.equal(binding.execution_evidence?.source_before?.manifest_sha256,
        binding.execution_evidence?.source_after?.manifest_sha256, "native execution source snapshots agree");
    assert.equal(binding.source?.manifest_sha256,
        binding.execution_evidence?.source_after?.manifest_sha256, "native source identity is internally consistent");
    assert.equal(binding.actions_file, "tests/browser/fixtures/hd1-actions.json");
    assert.equal(binding.actions_sha256, params.actions_sha256, "browser actions match the native execution fixture");
    assert.equal(params.actions_file, "hd1-actions.json");
    assert(/^[a-f0-9]{64}$/.test(params.actions_sha256), "fixture digest is recorded");

    const debug = await view.bridge("turts_debug");
    assert.match(debug.phase, /^Joined \{ slot: 0 \}$/, "HD1 joined the controlled Muster package");
    const scene = await view.bridge("turts_scene");
    const adapter = scene.render_adapter;
    assert(adapter && adapter.backend === "BrowserWebGpu" && typeof adapter.device_type === "string",
        `Bevy reports its actual WebGPU RenderAdapterInfo: ${JSON.stringify(adapter)}`);
    assert.notEqual(adapter.device_type, "Cpu", "Bevy must not report a CPU adapter");
    console.log("HD1_GPU_ADAPTER", JSON.stringify(adapter));
    const browserUnits = debug.replicated_world?.units || [];
    const nativeUnits = native.replicated_world?.units || [];
    const owned = (units, content) => units
        .filter((unit) => unit.owner_slot === 0 && unit.content_id === content)
        .map((unit) => unit.unit_id).sort((a, b) => a - b);
    const browserTroopers = owned(browserUnits, "muster:trooper");
    const browserWorkers = owned(browserUnits, "muster:worker");
    const nativeTroopers = owned(nativeUnits, "muster:trooper");
    const nativeWorkers = owned(nativeUnits, "muster:worker");
    assert.equal(browserTroopers.length, 39, "Remove deleted one real trooper");
    assert.deepEqual(browserTroopers, nativeTroopers);
    assert.deepEqual(browserWorkers, nativeWorkers);
    assert.deepEqual(sortedIds(debug.groups?.["1"]), browserTroopers, "group 1 stores the living troopers");
    assert.deepEqual(sortedIds(debug.groups?.["2"]), browserWorkers, "group 2 stores both workers");
    assert.deepEqual(sortedIds(native.groups?.["1"]), browserTroopers, "native and browser group 1 memberships match");
    assert.deepEqual(sortedIds(native.groups?.["2"]), browserWorkers, "native and browser group 2 memberships match");
    assert.deepEqual(sortedIds(debug.selection), sortedIds(native.selection), "final group recall selection matches native");
    assert.deepEqual(sortedIds(debug.selection), browserTroopers, "final group recall selects group 1");

    const browserResults = normalizedResults(debug.results);
    const nativeResults = normalizedResults(native.results);
    assert.deepEqual(browserResults, nativeResults,
        "browser per-envelope accepted/refused sets exactly match native, including missing and extra results");
    const refusals = nativeResults.filter(([, , status]) => status === "refused");
    assert(refusals.length > 0, "the authored Move while Hold is active exercises an authoritative refusal");
    assert(refusals.every(([seq]) => seq === 8), "only the held Shift+Move is refused in this fixture");
    assert(input.some((entry) => entry.call === "drag"), "box selection used a real pointer drag");
    assert(input.some((entry) => entry.call === "screenshot") && input.some((entry) => entry.call === "save"),
        "the final actual browser frame and scene checkpoint are retained for review");
    assert(input.some((entry) => entry.call === "key" && entry.args?.[0] === "Digit1" && entry.args?.[1]?.includes("Control")),
        "group assignment used a real Control+digit chord");
}

module.exports = { judge };
