// turts-test-role: instrument
// Install before navigation. Patch prototypes while returning the original WebGPU
// objects, so native WebIDL brand checks still receive real devices and contexts.
function installWebGpuTelemetry() {
    const metric = {
        adapterRequested: false,
        adapterInfo: null,
        acquiredDeviceCount: 0,
        deviceAcquired: false,
        configuredDeviceAcquired: false,
        configuredContexts: 0,
        currentTextureAcquisitions: 0,
        submittedDrawCalls: 0,
        submissions: 0,
        executedBundles: 0,
        instrumentationErrors: [],
    };
    Object.defineProperty(window, "__mcGpuMetrics", { value: metric, configurable: false });

    const patched = new WeakMap();
    const bundleEncoderStates = new WeakMap();
    const commandEncoderStates = new WeakMap();
    const passStates = new WeakMap();
    const bundleDrawCounts = new WeakMap();
    const commandDrawCounts = new WeakMap();
    const deviceRecords = new WeakMap();
    const queueDeviceIds = new WeakMap();
    const contextDeviceIds = new WeakMap();
    const drawMethods = ["draw", "drawIndexed", "drawIndirect", "drawIndexedIndirect"];
    let nextDeviceId = 1;
    const recordError = (error) => metric.instrumentationErrors.push(String(error));

    function patchMethod(prototype, name, wrap) {
        if (!prototype) return;
        let names = patched.get(prototype);
        if (!names) {
            names = new Set();
            patched.set(prototype, names);
        }
        if (names.has(name)) return;
        const descriptor = Object.getOwnPropertyDescriptor(prototype, name);
        if (!descriptor || typeof descriptor.value !== "function") return;
        try {
            Object.defineProperty(prototype, name, { ...descriptor, value: wrap(descriptor.value) });
            names.add(name);
        } catch (error) {
            recordError(`${name}: ${error}`);
        }
    }

    function patchDrawMethods(prototype, stateForThis) {
        for (const name of drawMethods) {
            patchMethod(prototype, name, (original) => function (...args) {
                stateForThis(this).draws += 1;
                return Reflect.apply(original, this, args);
            });
        }
    }

    function patchBundleEncoder(encoder, deviceId) {
        const prototype = Object.getPrototypeOf(encoder);
        bundleEncoderStates.set(encoder, { draws: 0, deviceId });
        patchDrawMethods(prototype, (thisEncoder) => bundleEncoderStates.get(thisEncoder));
        patchMethod(prototype, "finish", (original) => function (...args) {
            const bundle = Reflect.apply(original, this, args);
            bundleDrawCounts.set(bundle, bundleEncoderStates.get(this));
            return bundle;
        });
    }

    function patchRenderPass(pass) {
        const prototype = Object.getPrototypeOf(pass);
        patchDrawMethods(prototype, (thisPass) => passStates.get(thisPass));
        patchMethod(prototype, "executeBundles", (original) => function (bundles) {
            const materialized = Array.from(bundles);
            const commandState = passStates.get(this);
            for (const bundle of materialized) {
                const counts = bundleDrawCounts.get(bundle);
                if (counts && counts.deviceId === commandState.deviceId) {
                    commandState.draws += counts.draws;
                    commandState.bundles += 1;
                }
            }
            return Reflect.apply(original, this, [materialized]);
        });
    }

    function patchCommandEncoder(encoder, deviceId) {
        const prototype = Object.getPrototypeOf(encoder);
        commandEncoderStates.set(encoder, { draws: 0, bundles: 0, deviceId });
        patchMethod(prototype, "beginRenderPass", (original) => function (...args) {
            const pass = Reflect.apply(original, this, args);
            passStates.set(pass, commandEncoderStates.get(this));
            patchRenderPass(pass);
            return pass;
        });
        patchMethod(prototype, "finish", (original) => function (...args) {
            const commandBuffer = Reflect.apply(original, this, args);
            commandDrawCounts.set(commandBuffer, commandEncoderStates.get(this));
            return commandBuffer;
        });
    }

    function patchQueue(queue, deviceId) {
        queueDeviceIds.set(queue, deviceId);
        patchMethod(Object.getPrototypeOf(queue), "submit", (original) => function (buffers) {
            const materialized = Array.from(buffers);
            const active = queueDeviceIds.get(this) === metric.activeDeviceId;
            const totals = materialized.reduce((sum, buffer) => {
                const counts = commandDrawCounts.get(buffer);
                if (active && counts && counts.deviceId === metric.activeDeviceId) {
                    sum.draws += counts.draws;
                    sum.bundles += counts.bundles;
                }
                return sum;
            }, { draws: 0, bundles: 0 });
            const result = Reflect.apply(original, this, [materialized]);
            if (active) {
                metric.submissions += 1;
                metric.submittedDrawCalls += totals.draws;
                metric.executedBundles += totals.bundles;
            }
            return result;
        });
    }

    function adapterInfo(adapter) {
        try {
            const info = adapter.info;
            return info ? {
                vendor: String(info.vendor || ""),
                architecture: String(info.architecture || ""),
                device: String(info.device || ""),
                description: String(info.description || ""),
            } : null;
        } catch (error) {
            recordError(`adapter info: ${error}`);
            return null;
        }
    }

    function patchAdapter(adapter) {
        const prototype = Object.getPrototypeOf(adapter);
        patchMethod(prototype, "requestDevice", (original) => async function (...args) {
            const actualAdapter = this;
            const info = adapterInfo(actualAdapter);
            const device = await Reflect.apply(original, actualAdapter, args);
            const record = { id: nextDeviceId++, info };
            deviceRecords.set(device, record);
            metric.acquiredDeviceCount += 1;
            metric.deviceAcquired = true;
            patchDevice(device, record);
            return device;
        });
    }

    function patchDevice(device, record) {
        const prototype = Object.getPrototypeOf(device);
        patchMethod(prototype, "createCommandEncoder", (original) => function (...args) {
            const encoder = Reflect.apply(original, this, args);
            patchCommandEncoder(encoder, deviceRecords.get(this).id);
            return encoder;
        });
        patchMethod(prototype, "createRenderBundleEncoder", (original) => function (...args) {
            const encoder = Reflect.apply(original, this, args);
            patchBundleEncoder(encoder, deviceRecords.get(this).id);
            return encoder;
        });
        try {
            patchQueue(device.queue, record.id);
        } catch (error) {
            recordError(`queue submit: ${error}`);
        }
    }

    try {
        const gpu = navigator.gpu;
        if (!gpu) throw new Error("navigator.gpu is unavailable");
        patchMethod(Object.getPrototypeOf(gpu), "requestAdapter", (original) => async function (...args) {
            metric.adapterRequested = true;
            const adapter = await Reflect.apply(original, this, args);
            if (adapter) patchAdapter(adapter);
            return adapter;
        });

        const originalGetContext = HTMLCanvasElement.prototype.getContext;
        Object.defineProperty(HTMLCanvasElement.prototype, "getContext", {
            configurable: true,
            writable: true,
            value: function (kind, ...args) {
                const context = Reflect.apply(originalGetContext, this, [kind, ...args]);
                if (kind !== "webgpu" || !context) return context;
                const prototype = Object.getPrototypeOf(context);
                patchMethod(prototype, "configure", (original) => function (...configureArgs) {
                    const result = Reflect.apply(original, this, configureArgs);
                    metric.configuredContexts += 1;
                    const device = configureArgs[0]?.device;
                    const record = device && deviceRecords.get(device);
                    if (record) {
                        contextDeviceIds.set(this, record.id);
                        metric.activeDeviceId = record.id;
                        metric.configuredDeviceAcquired = true;
                        metric.adapterInfo = record.info;
                    }
                    return result;
                });
                patchMethod(prototype, "getCurrentTexture", (original) => function (...textureArgs) {
                    if (contextDeviceIds.get(this) === metric.activeDeviceId) {
                        metric.currentTextureAcquisitions += 1;
                    }
                    return Reflect.apply(original, this, textureArgs);
                });
                return context;
            },
        });
    } catch (error) {
        recordError(error);
    }
}

module.exports = { installWebGpuTelemetry };
