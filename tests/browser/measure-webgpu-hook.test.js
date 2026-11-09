// turts-test-role: unit
"use strict";

const assert = require("node:assert/strict");
const test = require("node:test");
const vm = require("node:vm");
const { installWebGpuTelemetry } = require("./measure-webgpu-hook.js");

class GPU {
    async requestAdapter(options = {}) { return new GPUAdapter(options.label || "renderer"); }
}
class GPUAdapter {
    constructor(label) { this.label = label; }
    get info() {
        return this.label === "probe"
            ? { vendor: "Google", architecture: "software", device: "probe", description: "unused probe" }
            : { vendor: "10de", architecture: "ampere", device: "test-device", description: "test NVIDIA adapter" };
    }
    async requestDevice() { return new GPUDevice(); }
}
class GPUDevice {
    constructor() { this.queue = new GPUQueue(); }
    createCommandEncoder() { return new GPUCommandEncoder(); }
    createRenderBundleEncoder() { return new GPURenderBundleEncoder(); }
}
class GPUQueue {
    constructor() { this.submissions = []; }
    submit(buffers) { this.submissions.push(buffers); }
}
class GPUCommandEncoder {
    beginRenderPass() { return new GPURenderPassEncoder(); }
    finish() { return { commandBuffer: true }; }
}
class GPURenderPassEncoder {
    draw() { this.calls = (this.calls || 0) + 1; }
    drawIndexed() { this.calls = (this.calls || 0) + 1; }
    drawIndirect() { this.calls = (this.calls || 0) + 1; }
    drawIndexedIndirect() { this.calls = (this.calls || 0) + 1; }
    executeBundles(bundles) { this.bundles = bundles; }
}
class GPURenderBundleEncoder {
    draw() { this.calls = (this.calls || 0) + 1; }
    drawIndexed() { this.calls = (this.calls || 0) + 1; }
    drawIndirect() { this.calls = (this.calls || 0) + 1; }
    drawIndexedIndirect() { this.calls = (this.calls || 0) + 1; }
    finish() { return { bundle: true }; }
}
class GPUCanvasContext {
    configure() { this.configured = true; }
    getCurrentTexture() { return {}; }
}
class HTMLCanvasElement {
    getContext(kind) { return kind === "webgpu" ? new GPUCanvasContext() : null; }
}

function mockBrowser() {
    const context = vm.createContext({
        window: {},
        navigator: { gpu: new GPU() },
        HTMLCanvasElement,
        console,
    });
    vm.runInContext(`(${installWebGpuTelemetry.toString()})()`, context);
    return { context, HTMLCanvasElement };
}

test("VIS2 hook counts submitted pass and executed bundle draws once", async () => {
    const { context, HTMLCanvasElement } = mockBrowser();
    const adapter = await context.navigator.gpu.requestAdapter({ label: "renderer" });
    const device = await adapter.requestDevice();
    const canvas = new HTMLCanvasElement();
    const gpuContext = canvas.getContext("webgpu");
    gpuContext.configure({ device });
    gpuContext.getCurrentTexture();

    // A later unused probe must not replace the adapter associated with the configured context.
    const probe = await context.navigator.gpu.requestAdapter({ label: "probe" });
    const probeDevice = await probe.requestDevice();
    const encoder = device.createCommandEncoder();

    // Recording bundle draws alone is not a submitted draw call.
    const bundleEncoder = device.createRenderBundleEncoder();
    bundleEncoder.draw(3);
    bundleEncoder.drawIndexed(6);
    const bundle = bundleEncoder.finish();

    const pass = encoder.beginRenderPass({});
    pass.draw(3);
    let bundleIteratorReads = 0;
    pass.executeBundles({
        *[Symbol.iterator]() {
            bundleIteratorReads += 1;
            yield bundle;
            yield bundle;
        },
    });
    assert.equal(bundleIteratorReads, 1);
    const commandBuffer = encoder.finish();
    device.queue.submit([commandBuffer]);

    const metric = context.window.__mcGpuMetrics;
    assert.equal(metric.adapterRequested, true);
    assert.equal(metric.adapterInfo.architecture, "ampere");
    assert.equal(metric.deviceAcquired, true);
    assert.equal(metric.acquiredDeviceCount, 2);
    assert.equal(metric.configuredDeviceAcquired, true);
    assert.equal(metric.configuredContexts, 1);
    assert.equal(metric.currentTextureAcquisitions, 1);
    assert.equal(metric.submissions, 1);
    assert.equal(metric.executedBundles, 2);
    assert.equal(metric.submittedDrawCalls, 5);
    const probeEncoder = probeDevice.createCommandEncoder();
    probeEncoder.beginRenderPass({}).draw(1);
    probeDevice.queue.submit([probeEncoder.finish()]);
    assert.equal(metric.submittedDrawCalls, 5, "unused probe device draws are excluded from renderer metrics");
    assert.equal(metric.instrumentationErrors.length, 0);
});

test("VIS2 hook materializes iterables once and ignores unsubmitted command buffers", async () => {
    const { context, HTMLCanvasElement } = mockBrowser();
    const device = await (await context.navigator.gpu.requestAdapter()).requestDevice();
    const canvas = new HTMLCanvasElement();
    canvas.getContext("webgpu").configure({ device });
    const abandoned = device.createCommandEncoder();
    const abandonedPass = abandoned.beginRenderPass({});
    abandonedPass.draw(1);
    abandonedPass.drawIndexed(1);
    abandonedPass.drawIndirect(1, 0);
    abandonedPass.drawIndexedIndirect(1, 0);
    abandoned.finish();
    const abandonedBundleEncoder = device.createRenderBundleEncoder();
    abandonedBundleEncoder.draw(1);
    const abandonedBundle = abandonedBundleEncoder.finish();
    const abandonedBundlePassEncoder = device.createCommandEncoder();
    abandonedBundlePassEncoder.beginRenderPass({}).executeBundles([abandonedBundle]);
    abandonedBundlePassEncoder.finish();

    const encoder = device.createCommandEncoder();
    const pass = encoder.beginRenderPass({});
    pass.draw(1);
    pass.drawIndexed(1);
    pass.drawIndirect(1, 0);
    pass.drawIndexedIndirect(1, 0);
    const buffer = encoder.finish();
    let iteratorReads = 0;
    const iterable = {
        *[Symbol.iterator]() {
            iteratorReads += 1;
            yield buffer;
        },
    };
    device.queue.submit(iterable);
    assert.equal(iteratorReads, 1);
    assert.equal(context.window.__mcGpuMetrics.submittedDrawCalls, 4);
});
