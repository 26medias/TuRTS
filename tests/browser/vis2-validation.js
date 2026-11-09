// turts-test-role: pure
"use strict";

const SOFTWARE_ADAPTER = /swiftshader|software|llvmpipe|lavapipe/i;

function adapterEvidence(report) {
    const webgpu = report.vis2?.webgpu || {};
    const adapter = report.vis2?.renderAdapter || {};
    const info = webgpu.adapterInfo || {};
    const metadata = [adapter.name, adapter.device_type, adapter.backend, ...Object.values(info)]
        .filter((value) => typeof value === "string")
        .join(" ")
        .toLowerCase();
    const renderAdapterComplete = [adapter.device_type, adapter.backend]
        .every((value) => typeof value === "string" && value.trim().length > 0);
    const hardwareInfo = [info.vendor, info.architecture, info.device, info.description]
        .filter((value) => typeof value === "string" && value.trim().length > 0 &&
            !/^(unknown|none|n\/a|undefined)$/i.test(value.trim()));
    const webgpuAdapterInfoComplete = hardwareInfo.length > 0;
    const softwareAdapter = adapter.device_type?.toLowerCase() === "cpu" || SOFTWARE_ADAPTER.test(metadata);
    const nvidiaAdapter = /nvidia|10de/i.test(metadata);
    return { renderAdapterComplete, webgpuAdapterInfoComplete, softwareAdapter, nvidiaAdapter };
}

function vis2Ready(report) {
    const gpu = report.vis2?.webgpu || {};
    const adapter = report.vis2?.renderAdapter || {};
    const { renderAdapterComplete, webgpuAdapterInfoComplete, softwareAdapter } = adapterEvidence(report);
    const memory = report.wasmMemoryBytes?.last;
    const errors = gpu.instrumentationErrors;
    return report.browserMode === "headed" && typeof report.display === "string" && report.display.length > 0 &&
        report.vis2?.actualUnits === 5000 && report.vis2?.worldUnits === 5000 &&
        gpu.adapterRequested === true && gpu.deviceAcquired === true && gpu.configuredDeviceAcquired === true &&
        Number.isInteger(gpu.activeDeviceId) && gpu.configuredContexts > 0 &&
        gpu.currentTextureAcquisitions > 0 && gpu.submittedDrawCalls > 0 &&
        Array.isArray(errors) && errors.length === 0 &&
        typeof adapter.backend === "string" && adapter.backend.toLowerCase().includes("webgpu") &&
        renderAdapterComplete && webgpuAdapterInfoComplete && !softwareAdapter &&
        Number.isFinite(report.frames?.p50_ms) && report.frames.p50_ms > 0 &&
        Number.isFinite(report.frames?.p99_ms) && report.frames.p99_ms > 0 &&
        Number.isFinite(memory) && memory > 0;
}

module.exports = { adapterEvidence, vis2Ready };
