// turts-test-role: unit
"use strict";

const assert = require("node:assert/strict");
const test = require("node:test");
const { adapterEvidence, vis2Ready } = require("./vis2-validation.js");

function payload({ name = "AMD Radeon RX 7900 XT", device_type = "DiscreteGpu", backend = "WebGpu", ...overrides } = {}) {
    const report = {
        browserMode: "headed",
        display: ":1",
        frames: { p50_ms: 12.5, p99_ms: 21.2 },
        wasmMemoryBytes: { last: 345678901 },
        vis2: {
            actualUnits: 5000,
            worldUnits: 5000,
            renderAdapter: { name, device_type, backend },
            webgpu: {
                adapterRequested: true,
                adapterInfo: { vendor: "amd", architecture: "rdna3", device: "gfx1100" },
                activeDeviceId: 4,
                deviceAcquired: true,
                configuredDeviceAcquired: true,
                configuredContexts: 1,
                currentTextureAcquisitions: 500,
                submittedDrawCalls: 500,
                instrumentationErrors: [],
            },
        },
    };
    Object.assign(report, overrides.report || {});
    Object.assign(report.vis2, overrides.vis2 || {});
    Object.assign(report.vis2.webgpu, overrides.webgpu || {});
    return report;
}

test("VIS2 accepts actual AMD and Intel WebGPU adapter metadata", () => {
    for (const metadata of [
        { name: "AMD Radeon RX 7900 XT", device_type: "DiscreteGpu", backend: "WebGpu", webgpu: { adapterInfo: { vendor: "amd", architecture: "rdna3", device: "gfx1100" } } },
        { name: "Intel Arc A770", device_type: "DiscreteGpu", backend: "WebGpu", webgpu: { adapterInfo: { vendor: "intel", architecture: "xe-hpg", device: "arc-a770" } } },
        { name: "Intel Iris Xe", device_type: "IntegratedGpu", backend: "WebGpu", webgpu: { adapterInfo: { vendor: "intel", device: "xe" } } },
        { name: "", device_type: "Other", backend: "BrowserWebGpu", webgpu: { adapterInfo: { vendor: "amd", device: "gfx1100" } } },
    ]) {
        assert.equal(vis2Ready(payload(metadata)), true, JSON.stringify(metadata));
        assert.equal(adapterEvidence(payload(metadata)).softwareAdapter, false);
    }
    const nvidia = payload({
        name: "NVIDIA GeForce RTX 3080",
        device_type: "DiscreteGpu",
        backend: "WebGpu",
        webgpu: { adapterInfo: { vendor: "nvidia", device: "rtx3080" } },
    });
    assert.equal(vis2Ready(nvidia), true);
    assert.equal(adapterEvidence(nvidia).nvidiaAdapter, true,
        "NVIDIA remains a diagnostic adapter identity, not a pass requirement");
    assert.equal(adapterEvidence(payload()).nvidiaAdapter, false);
});

test("VIS2 rejects CPU and known software adapter metadata", () => {
    for (const metadata of [
        { name: "CPU adapter", device_type: "Cpu", backend: "WebGpu" },
        { name: "llvmpipe (LLVM 19.1)", device_type: "Cpu", backend: "Vulkan" },
        { name: "lavapipe", device_type: "Cpu", backend: "Vulkan" },
        { name: "Google SwiftShader", device_type: "Other", backend: "WebGpu", webgpu: { adapterInfo: { vendor: "google", device: "swiftshader" } } },
    ]) {
        assert.equal(vis2Ready(payload(metadata)), false, JSON.stringify(metadata));
        assert.equal(adapterEvidence(payload(metadata)).softwareAdapter, true);
    }
});

test("VIS2 rejects missing actual adapter metadata and a missing WebGPU request", () => {
    assert.equal(vis2Ready(payload({ name: "", device_type: "", backend: "" })), false);
    assert.equal(vis2Ready(payload({ webgpu: { adapterRequested: false } })), false);
    assert.equal(vis2Ready(payload({ webgpu: { adapterInfo: {} } })), false);
    assert.equal(vis2Ready(payload({ webgpu: { adapterInfo: { vendor: "unknown" } } })), false);
});

test("VIS2 requires an acquired and configured device, context, and submitted draw", () => {
    for (const webgpu of [
        { deviceAcquired: false },
        { configuredDeviceAcquired: false },
        { configuredContexts: 0 },
        { currentTextureAcquisitions: 0 },
        { submittedDrawCalls: 0 },
        { instrumentationErrors: ["context mismatch"] },
    ]) assert.equal(vis2Ready(payload({ webgpu })), false, JSON.stringify(webgpu));
});

test("VIS2 requires the actual 5000 units and headed proof", () => {
    assert.equal(vis2Ready(payload({ vis2: { actualUnits: 4999 } })), false);
    assert.equal(vis2Ready(payload({ vis2: { worldUnits: 5001 } })), false);
    assert.equal(vis2Ready(payload({ report: { browserMode: "headless" } })), false);
    assert.equal(vis2Ready(payload({ report: { display: null } })), false);
});

test("VIS2 requires finite positive p50, p99, and memory metrics", () => {
    for (const report of [
        { frames: { p50_ms: 0 } },
        { frames: { p99_ms: Infinity } },
        { wasmMemoryBytes: { last: 0 } },
    ]) assert.equal(vis2Ready(payload({ report })), false, JSON.stringify(report));
});
