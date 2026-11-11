// turts-test-role: harness
// Private HARNESS fixtures only. Never expose this module through driver IO or product bridges.
'use strict';

const crypto = require('node:crypto');
const fs = require('node:fs/promises');
const path = require('node:path');

const PER1_FIXTURES = Object.freeze({
    'per1-settings-corrupt-v1': Object.freeze({
        id: 'per1-settings-corrupt-v1',
        seeds: Object.freeze([
            { storage: 'localStorage', key: 'turts.v1.settings', value: '{not-json', role: 'malformed-settings-json' },
            { storage: 'localStorage', key: 'turts.v1.bindings.forge.0.2', value: '{"actions":{}}', role: 'verified-own-mod' },
            { storage: 'localStorage', key: 'turts.v1.bindings.other_mod.1', value: '{"actions":{"key_move":[{"Key":{"code":"KeyQ","mods":[]}}]}}', role: 'foreign-mod' },
            { storage: 'localStorage', key: 'turts.v1.bindings.forge.0.3', value: '{"actions":{"key_move":[{"Key":{"code":"KeyR","mods":[]}}]}}', role: 'foreign-compatibility-line' },
            { storage: 'localStorage', key: 'turts.v1.groups.18446744073709551614.0', value: '{"groups":{"1":[7,8]}}', role: 'old-match-group' },
            { storage: 'localStorage', key: 'turts.v1.groups.18446744073709551613.1', value: '{"groups":{"2":[9,10]}}', role: 'old-match-group' },
        ]),
    }),
    'per1-invalid-binding-v1': Object.freeze({
        id: 'per1-invalid-binding-v1',
        seeds: Object.freeze([
            { storage: 'localStorage', key: 'turts.v1.settings', value: '{"audio":{"master":0.2,"voice":0.2,"alerts":0.2,"effects":0.2}}', role: 'valid-task15-audio-seed' },
            { storage: 'localStorage', key: 'turts.v1.bindings.forge.0.2', value: '{"actions":{"key_move":[{"Key":{"code":"F12","mods":[]}}]}}', role: 'malformed-known-action' },
            { storage: 'localStorage', key: 'turts.v1.bindings.other_mod.1', value: '{"actions":{"key_move":[{"Key":{"code":"KeyQ","mods":[]}}]}}', role: 'foreign-mod' },
            { storage: 'localStorage', key: 'turts.v1.bindings.forge.0.3', value: '{"actions":{"key_move":[{"Key":{"code":"KeyR","mods":[]}}]}}', role: 'foreign-compatibility-line' },
            { storage: 'localStorage', key: 'turts.v1.groups.18446744073709551614.0', value: '{"groups":{"1":[7,8]}}', role: 'old-match-group' },
            { storage: 'localStorage', key: 'turts.v1.groups.18446744073709551613.1', value: '{"groups":{"2":[9,10]}}', role: 'old-match-group' },
        ]),
    }),
    'sel4-authored-muster-death-v1': Object.freeze({
        id: 'sel4-authored-muster-death-v1',
        seeds: Object.freeze([
            { storage: 'localStorage', key: 'turts.v1.groups.18446744073709551614.0', value: '{"groups":{"1":[7,8]}}', role: 'old-match-group' },
            { storage: 'localStorage', key: 'turts.v1.groups.18446744073709551613.1', value: '{"groups":{"2":[9,10]}}', role: 'old-match-group' },
        ]),
    }),
});

function per1FixtureFor(id) {
    if (id === undefined || id === null) return null;
    const fixture = PER1_FIXTURES[id];
    if (!fixture) throw new Error(`unapproved Task16 preboot fixture id: ${id}`);
    return { id: fixture.id, seeds: fixture.seeds.map((seed) => ({ ...seed })) };
}

function sha256(value) { return crypto.createHash('sha256').update(String(value)).digest('hex'); }

async function collectPer1FixtureReceipts({ pages, observed = [], documentEpochs, targetOrigin, fixture, outDir, runError = null }) {
    const windows = [];
    for (let index = 0; index < pages.length; index += 1) {
        let receipt = observed[index] || null;
        if (!receipt || !Array.isArray(receipt.seeds)) {
            try {
                receipt = await pages[index].evaluate((expectedOrigin) => {
                    if (location.origin === 'null' || location.origin !== expectedOrigin) {
                        return { status: 'wrong-origin', observed_origin: location.origin, observed_url: document.URL };
                    }
                    try {
                        const raw = sessionStorage.getItem('__task16_preboot_receipt_v1');
                        const base = raw ? JSON.parse(raw) : { status: 'missing-receipt' };
                        return { ...base, observed_document_url: document.URL, observed_time_origin: performance.timeOrigin,
                            fixture_audit_operations: (window.__task16_storage_audit || []).filter((entry) => entry.provenance === 'fixture_seed') };
                    } catch (error) {
                        return { status: 'receipt-read-error', error: String(error?.message || error), observed_document_url: document.URL, observed_time_origin: performance.timeOrigin, fixture_audit_operations: (window.__task16_storage_audit || []).filter((entry) => entry.provenance === 'fixture_seed') };
                    }
                }, targetOrigin);
            } catch (error) {
                receipt = { status: 'receipt-read-error', error: String(error?.message || error) };
            }
        }
        receipt = receipt || { status: 'missing-receipt' };
        const observedWindow = receipt.window ?? index + 1;
        const documentEpoch = receipt.document_epoch ?? documentEpochs[index] ?? 0;
        const seeds = Array.isArray(receipt.seeds) ? receipt.seeds.map((seed) => ({
            ...seed,
            observed_window: observedWindow,
            key_sha256: sha256(seed.key),
            value_sha256: sha256(seed.value),
            readback_sha256: typeof seed.readback === 'string' ? sha256(seed.readback) : null,
            window: observedWindow,
            document_epoch: documentEpoch,
        })) : [];
        windows.push({
            ...receipt,
            fixture: receipt.fixture || fixture.id,
            observed_window: observedWindow,
            window: index + 1,
            document_epoch: documentEpoch,
            target_origin: receipt.target_origin || targetOrigin,
            seeds,
        });
    }
    const seedReceipts = windows.flatMap((window) => window.seeds.map((seed) => ({
        ...seed,
        fixture: window.fixture,
        target_origin: window.target_origin,
        initial_url: window.initial_url,
        initial_time_origin: window.initial_time_origin,
        initial_wall_time_ms: window.initial_wall_time_ms,
        initial_performance_ms: window.initial_performance_ms,
        observed_window: window.observed_window,
        window: window.window,
        document_epoch: window.document_epoch,
    })));
    const expected = PER1_FIXTURES[fixture.id];
    const receiptErrors = [];
    if (windows.length !== pages.length || windows.length === 0) receiptErrors.push('receipt window count mismatch');
    for (const [index, window] of windows.entries()) {
        if (window.fixture !== fixture.id) receiptErrors.push(`window ${index + 1}: fixture id mismatch`);
        if (window.status !== 'complete') receiptErrors.push(`window ${index + 1}: initializer status ${window.status}`);
        if (window.target_origin !== targetOrigin) receiptErrors.push(`window ${index + 1}: target origin mismatch`);
        if (window.observed_window !== index + 1) receiptErrors.push(`window ${index + 1}: initializer window mismatch`);
        if (window.initial_url !== window.observed_document_url) receiptErrors.push(`window ${index + 1}: actual initial URL lineage mismatch`);
        if (!Number.isFinite(window.initial_time_origin) || window.initial_time_origin !== window.observed_time_origin) receiptErrors.push(`window ${index + 1}: actual document time-origin mismatch`);
        if (!Number.isFinite(window.initial_wall_time_ms) || !Number.isFinite(window.initial_performance_ms)) receiptErrors.push(`window ${index + 1}: actual clock anchors absent`);
        if (!Number.isSafeInteger(window.document_epoch) || window.document_epoch < 1) receiptErrors.push(`window ${index + 1}: document epoch absent`);
        if (!Array.isArray(window.seeds) || window.seeds.length !== expected.seeds.length) receiptErrors.push(`window ${index + 1}: seed count mismatch`);
        for (const [seedIndex, wanted] of expected.seeds.entries()) {
            const actual = window.seeds?.[seedIndex];
            if (!actual || actual.storage !== wanted.storage || actual.key !== wanted.key || actual.value !== wanted.value || actual.role !== wanted.role ||
                actual.set_returned !== true || actual.readback_matches !== true || actual.readback !== wanted.value ||
                !Number.isFinite(actual.operation_started_wall_time_ms) || !Number.isFinite(actual.operation_finished_wall_time_ms) ||
                actual.operation_finished_wall_time_ms < actual.operation_started_wall_time_ms ||
                !Number.isFinite(actual.operation_started_performance_ms) || !Number.isFinite(actual.operation_finished_performance_ms) ||
                actual.operation_finished_performance_ms < actual.operation_started_performance_ms ||
                actual.key_sha256 !== sha256(wanted.key) || actual.value_sha256 !== sha256(wanted.value) || actual.readback_sha256 !== sha256(wanted.value)) {
                receiptErrors.push(`window ${index + 1}: seed ${seedIndex + 1} failed exact set/readback/hash validation`);
            }
        }
    }
    const report = {
        fixture: fixture.id,
        corrupt_case: fixture.id === 'per1-invalid-binding-v1' ? 'binding' : fixture.id === 'per1-settings-corrupt-v1' ? 'settings' : null,
        status: receiptErrors.length === 0 ? 'complete' : 'incomplete',
        errors: receiptErrors,
        runner_error: runError ? { name: runError?.name || 'Error', message: String(runError?.message || runError), stack: String(runError?.stack || '') } : null,
        windows,
        seed_receipts: seedReceipts,
    };
    const name = fixture.id.startsWith('per1-') ? 'per1-fixture-private.json' : 'task16-preboot-storage-fixture-private.json';
    await writePrivateJson(path.join(outDir, name), report);
    return report;
}

async function writePrivateJson(target, value) {
    await fs.writeFile(target, JSON.stringify(value, null, 2) + '\n', { mode: 0o600 });
    await fs.chmod(target, 0o600);
}

module.exports = { per1FixtureFor, collectPer1FixtureReceipts, PER1_FIXTURES };
