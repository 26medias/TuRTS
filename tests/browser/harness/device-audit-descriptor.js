// turts-test-role: harness
'use strict';

// Harness-only candidate. Inject before navigation with an opaque per-document key.
// Never expose this registry or helper through driver io or a product bridge.
function installDeviceAudit({ key, limit = 8 }) {
    const registry = Object.create(null);
    registry.status = 'unavailable';
    registry.calls = [];
    registry.total = 0;
    registry.overflow = 0;
    registry.recordErrors = 0;
    try {
        Object.defineProperty(globalThis, key, { value: registry });
        const prototype = globalThis.GPUAdapter?.prototype;
        if (!prototype) return;
        const descriptor = Object.getOwnPropertyDescriptor(prototype, 'requestDevice');
        if (!descriptor || typeof descriptor.value !== 'function') return;
        const original = descriptor.value;
        function requestDevice(...args) {
            let record;
            try {
                registry.total += 1;
                if (registry.calls.length < limit) {
                    record = Object.create(null);
                    record.call = registry.total;
                    record.calledAt = performance.now();
                    registry.calls.push(record);
                } else {
                    registry.overflow += 1;
                }
            } catch {
                registry.recordErrors += 1;
            }
            let result;
            try {
                result = Reflect.apply(original, this, args);
            } catch (error) {
                if (record) record.synchronousThrow = true;
                throw error;
            }
            try {
                if (record) {
                    record.promise = result;
                    record.descriptor = args[0];
                    record.returnedAt = performance.now();
                }
            } catch {
                registry.recordErrors += 1;
            }
            return result;
        }
        Object.defineProperty(prototype, 'requestDevice', { ...descriptor, value: requestDevice });
        registry.status = 'installed';
    } catch {
        registry.status = 'installation-error';
    }
}

// CDP observes internal PromiseState without attaching handlers or awaiting settlement.
// CDP is optional diagnostic machinery; failures become explicit NOT TRACED records.
async function collectDeviceAudit(cdp, key, objectGroup) {
    const expressionKey = JSON.stringify(key);
    try {
        const metadata = await cdp.send('Runtime.evaluate', {
            expression: `(() => { const r = globalThis[${expressionKey}]; if (!r) return null; return {status:r.status,total:r.total,overflow:r.overflow,recordErrors:r.recordErrors,observedAt:performance.now(),calls:r.calls.map(c => ({call:c.call,calledAt:c.calledAt,returnedAt:c.returnedAt,synchronousThrow:c.synchronousThrow === true}))}; })()`,
            awaitPromise: false,
            returnByValue: true,
        });
        if (metadata.exceptionDetails || !metadata.result?.value) {
            return { status: 'NOT TRACED', reason: 'diagnostic registry not readable' };
        }
        const audit = metadata.result.value;
        for (let index = 0; index < audit.calls.length; index += 1) {
            const call = audit.calls[index];
            if (call.synchronousThrow) continue;
            const handle = await cdp.send('Runtime.evaluate', {
                expression: `globalThis[${expressionKey}].calls[${index}].promise`,
                awaitPromise: false,
                returnByValue: false,
                objectGroup,
            });
            if (handle.exceptionDetails || !handle.result?.objectId) {
                call.promiseState = 'NOT TRACED';
                continue;
            }
            call.stateReadHostStartMs = Date.now();
            const properties = await cdp.send('Runtime.getProperties', {
                objectId: handle.result.objectId,
                ownProperties: true,
                generatePreview: false,
            });
            call.stateReadHostEndMs = Date.now();
            const state = properties.internalProperties?.find(item => item.name === '[[PromiseState]]');
            const value = state?.value?.value;
            call.promiseState = !properties.exceptionDetails && ['pending', 'fulfilled', 'rejected'].includes(value) ? value : 'NOT TRACED';
            // Do not read PromiseResult, descriptors, request arguments or error objects.
            call.settlementTime = 'NOT TRACED';
            call.requirementsReadHostStartMs = Date.now();
            call.requirements = await readDeviceRequirements(cdp, key, index, objectGroup);
            call.requirementsReadHostEndMs = Date.now();
        }
        return audit;
    } catch (error) {
        return { status: 'NOT TRACED', reason: String(error) };
    } finally {
        try { await cdp.send('Runtime.releaseObjectGroup', { objectGroup }); } catch {}
    }
}


// C312: only CDP own DATA descriptors, never property getters or iterable evaluation.
async function readDeviceRequirements(cdp, key, index, objectGroup) {
    async function properties(value) {
        if (!value?.objectId) throw new Error('requirement object is not observable');
        const result = await cdp.send('Runtime.getProperties', {objectId:value.objectId,ownProperties:true,generatePreview:false});
        if (result.exceptionDetails || !Array.isArray(result.result)) throw new Error('requirement descriptors unavailable');
        return result.result;
    }
    function data(property, name) {
        if (!property || !Object.hasOwn(property, 'value')) throw new Error(`${name} is absent or an accessor; no getter invoked`);
        return property.value;
    }
    try {
        const handle = await cdp.send('Runtime.evaluate', {
            expression:`globalThis[${JSON.stringify(key)}].calls[${index}].descriptor`,
            awaitPromise:false,returnByValue:false,objectGroup,
        });
        if (handle.exceptionDetails) throw new Error('descriptor reference unavailable');
        if (handle.result?.type === 'undefined' || handle.result?.subtype === 'null') {
            return {status:'observed',descriptorKind:handle.result.subtype === 'null' ? 'null' : 'undefined',requiredFeatures:[],requiredLimits:{}};
        }
        const fields = await properties(handle.result);
        const featureField = fields.find(field => field.name === 'requiredFeatures');
        const limitField = fields.find(field => field.name === 'requiredLimits');
        const requiredFeatures = [];
        const requiredLimits = Object.create(null);
        if (featureField) {
            const featureValue = data(featureField, 'requiredFeatures');
            if (featureValue.subtype !== 'array') throw new Error('features are not a source-traced plain array');
            const entries = await properties(featureValue);
            const length = data(entries.find(field => field.name === 'length'), 'feature length').value;
            if (!Number.isSafeInteger(length) || length < 0 || length > 64) throw new Error('feature count exceeds bound64');
            for (let n = 0; n < length; n++) {
                const value = data(entries.find(field => field.name === String(n)), 'feature entry');
                if (value.type !== 'string' || typeof value.value !== 'string' || value.value.length > 128) throw new Error('unsupported or oversized feature value');
                requiredFeatures.push(value.value);
            }
        }
        if (limitField) {
            const entries = await properties(data(limitField, 'requiredLimits'));
            if (entries.length > 64) throw new Error('limit count exceeds bound64');
            for (const entry of entries) {
                if (typeof entry.name !== 'string' || entry.name.length > 128) throw new Error('oversized limit name');
                const value = data(entry, 'limit entry');
                if (value.type !== 'number' || typeof value.value !== 'number' || !Number.isFinite(value.value)) throw new Error('unsupported numeric limit');
                requiredLimits[entry.name] = value.value;
            }
        }
        return {status:'observed',descriptorKind:'object',fieldsPresent:{requiredFeatures:Boolean(featureField),requiredLimits:Boolean(limitField)},requiredFeatures,requiredLimits};
    } catch (error) {
        return {status:'NOT TRACED',reason:String(error)};
    }
}

module.exports = { installDeviceAudit, collectDeviceAudit, readDeviceRequirements };
