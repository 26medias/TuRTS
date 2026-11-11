// turts-test-role: harness
'use strict';

const crypto = require('node:crypto');
const fs = require('node:fs/promises');
const path = require('node:path');
const sha = (bytes) => crypto.createHash('sha256').update(bytes).digest('hex');

// Observe completed real application frames, and hold only the next requested
// frame while the existing IO takes its real screenshot. No product state is set.
function installSh1PhaseCapture() {
    const request = window.requestAnimationFrame.bind(window);
    const seen = new Set();
    window.__sh1_phase_capture = { enabled: false, held: null, records: [] };
    window.requestAnimationFrame = (callback) => request((timestamp) => {
        callback(timestamp);
        if (!window.__sh1_phase_capture.enabled || typeof window.turts_debug !== 'function' || window.__sh1_phase_capture.held) return;
        let debug;
        try { debug = JSON.parse(window.turts_debug()); } catch { return; }
        const kind = debug.phase?.startsWith('VerifyPackage') ? 'verify' :
            /^(Connecting|Joining)/.test(debug.phase || '') ? 'joining' : null;
        if (!kind || seen.has(kind)) return;
        seen.add(kind);
        window.__turts_begin_bounded_input();
        const record = { kind, phase: debug.phase, wall_time_ms: Date.now(), performance_ms: performance.now(),
            document_time_origin: performance.timeOrigin, document_url: document.URL, released: false };
        window.__sh1_phase_capture.records.push(record);
        window.__sh1_phase_capture.held = kind;
    });
}

async function prepareSh1Lifecycle({ context, params, outDir, epoch, navigationArchive, page }) {
    const origin = new URL(params.server_url).origin;
    const records = [];
    const gates = new Map();
    let advertisementCount = 0;
    let lockfileCount = 0;
    let packageCount = 0;
    let liveSocket = null;
    let socketCount = 0;
    const save = async () => fs.writeFile(path.join(outDir, 'sh1-lifecycle-private.json'),
        JSON.stringify({ fixture: 'sh1-real-acquisition-v1', records, navigation_archive: navigationArchive }, null, 2), { mode: 0o600 });
    await context.exposeBinding('__sh1_before_navigation', (source, receipt) => {
        if (source.page !== page || source.frame !== page.mainFrame() || receipt.document_url !== params.url ||
            navigationArchive.length >= 8 || receipt.actual?.length > 4096 || receipt.downstream?.length > 4096 ||
            receipt.contextmenu?.length > 64 ||
            !Number.isFinite(receipt.document_time_origin) || !Array.isArray(receipt.actual)) {
            throw new Error('HARNESS SH1 navigation archive has wrong source identity');
        }
        navigationArchive.push(structuredClone(receipt));
    });
    await context.addInitScript(() => {
        window.addEventListener('beforeunload', () => {
            void window.__sh1_before_navigation({
                document_url: document.URL, document_time_origin: performance.timeOrigin,
                wall_time_ms: Date.now(), active_call: window.__turts_io_active_call,
                actual: window.__turts_io_actual_events || [],
                downstream: window.__turts_io_downstream_events || [],
                contextmenu: window.__turts_io_contextmenu_events || [],
                contextmenu_overflow: window.__turts_io_contextmenu_overflow === true,
            }).catch(() => {});
        });
    });
    await context.addInitScript(installSh1PhaseCapture);
    await context.route(`${origin}/**`, async (route) => {
        const url = new URL(route.request().url());
        const response = await route.fetch();
        const original = await response.body();
        let kind = null;
        let served = original;
        if (url.pathname === '/advertisement.json') {
            advertisementCount += 1;
            if (advertisementCount === 1) kind = 'advertisement';
        } else if (url.pathname === '/lockfile.json') {
            lockfileCount += 1;
            if (lockfileCount === 1) {
                kind = 'lockfile';
                served = Buffer.concat([original, Buffer.from('\n ')]);
            }
        } else if (url.pathname.startsWith('/packages/')) {
            packageCount += 1;
            if (packageCount === 1) kind = 'package';
        }
        const headers = { ...response.headers() };
        delete headers['content-encoding'];
        delete headers['transfer-encoding'];
        headers['content-length'] = String(served.length);
        const record = { original_byte_length: original.length, served_byte_length: served.length,
            original_headers: response.headers(), served_headers: headers, source: 'real_http_response', url: url.href, document_epoch: epoch(),
            original_sha256: sha(original), served_sha256: sha(served), status: response.status(),
            observed_wall_time_ms: Date.now(), hold: kind, released: false };
        records.push(record);
        try {
            if (kind) await new Promise((resolve) => gates.set(kind, resolve));
            await route.fulfill({ response, headers, body: served });
            record.released = true;
            record.released_wall_time_ms = Date.now();
        } finally { await save(); }
    });
    await context.routeWebSocket(params.ws_url, (socket) => {
        const server = socket.connectToServer();
        socketCount += 1;
        const record = { source: 'real_websocket_proxy', url: socket.url(), index: socketCount,
            document_epoch: epoch(), observed_wall_time_ms: Date.now(), closed_by_fixture: false };
        records.push(record);
        liveSocket = { socket, server, record };
    });
    const waitGate = async (kind) => {
        const deadline = Date.now() + 30000;
        while (!gates.has(kind)) {
            if (Date.now() >= deadline) throw new Error(`HARNESS SH1 actual ${kind} request was not held`);
            await new Promise((resolve) => setTimeout(resolve, 20));
        }
    };
    const release = (kind) => {
        const resolve = gates.get(kind);
        if (!resolve) throw new Error(`HARNESS SH1 ${kind} has no actual held response`);
        gates.delete(kind);
        resolve();
    };
    const waitPhase = async (page, pattern) => page.waitForFunction((source) => {
        if (typeof window.turts_debug !== 'function') return false;
        try { return new RegExp(source).test(JSON.parse(window.turts_debug()).phase); } catch { return false; }
    }, pattern, { timeout: 30000, polling: 20 });
    const drive = (driver, io, phase) => driver.drive(io, { ...params, fixture_phase: phase });
    const releaseFrame = async (page, kind) => {
        const record = await page.evaluate((expected) => {
            const state = window.__sh1_phase_capture;
            if (state?.held !== expected) throw new Error('actual phase frame was not held');
            const entry = state.records.find((item) => item.kind === expected && !item.released);
            const released = window.__turts_end_bounded_input();
            state.held = null;
            Object.assign(entry, { released: released.released, release_wall_time_ms: Date.now(),
                held_callbacks: released.held_callbacks });
            return entry;
        }, kind);
        records.push({ source: 'actual_completed_frame_capture', document_epoch: epoch(), ...record });
        await save();
    };
    return {
        async acquire(page, driver, io) {
            await waitGate('advertisement');
            await waitPhase(page, '^FetchAdvertisement');
            await drive(driver, io, 'fetch-advertisement');
            release('advertisement');
            await waitGate('lockfile');
            await waitPhase(page, '^FetchLockfile');
            await drive(driver, io, 'fetch-lockfile');
            release('lockfile');
            await waitPhase(page, '^Failed');
            await drive(driver, io, 'wrong-lockfile');
            await driver.drive(io, { ...params, fixture_phase: 'healthy-reload' }, {
                beforeAction: async (_action, index) => {
                    if (index === 1) { await waitGate('package'); await waitPhase(page, '^FetchPackages'); }
                },
            });
            await waitGate('package');
            await waitPhase(page, '^FetchPackages');
            await drive(driver, io, 'fetch-package');
            await page.evaluate(() => { window.__sh1_phase_capture.enabled = true; });
            release('package');
            for (const kind of ['verify', 'joining']) {
                await page.waitForFunction((expected) => window.__sh1_phase_capture?.held === expected,
                    kind, { timeout: 30000, polling: 20 });
                await drive(driver, io, `load-${kind}`);
                await releaseFrame(page, kind);
            }
        },
        async disconnect(page, driver, io) {
            if (!liveSocket || liveSocket.record.closed_by_fixture) throw new Error('HARNESS SH1 real live socket is absent');
            const before = await page.evaluate(() => {
                if (typeof window.turts_debug !== 'function') return null;
                const debug = JSON.parse(window.turts_debug());
                return { phase: debug.phase, player_id_decimal: debug.player_id_decimal,
                    reconnect_required: debug.reconnect_required };
            });
            if (!before?.phase?.startsWith('Joined') || before.reconnect_required ||
                liveSocket.record.document_epoch !== epoch()) throw new Error('HARNESS SH1 disconnect lacks a current Joined transport');
            const target = liveSocket;
            target.record.closed_by_fixture = true;
            target.record.close_wall_time_ms = Date.now();
            target.record.close_document_epoch = epoch();
            await Promise.all([target.socket.close({ code: 1011, reason: 'Task16 physical connection-loss fixture' }),
                target.server.close({ code: 1011, reason: 'Task16 physical connection-loss fixture' })]);
            target.record.both_sides_closed = true;
            target.record.joined_before_close = before;
            await save();
            await waitPhase(page, '^Joined');
            await page.waitForFunction(() => typeof window.turts_debug === 'function' &&
                JSON.parse(window.turts_debug()).reconnect_required === true, null, { timeout: 30000, polling: 20 });
            let actualNavigation;
            await driver.drive(io, { ...params, fixture_phase: 'connection-lost' }, {
                beforeAction: async (action) => {
                    if (action.call === 'click') actualNavigation = page.waitForEvent('framenavigated', {
                        predicate: (frame) => frame === page.mainFrame(), timeout: 30000,
                    });
                },
                afterAction: async (action) => {
                    if (action.call === 'click') { await actualNavigation; await waitPhase(page, '^Joined'); }
                },
            });
            await save();
        },
        async cleanup(page) {
            for (const resolve of gates.values()) resolve();
            gates.clear();
            if (page && !page.isClosed()) {
                await page.evaluate(() => { window.__turts_end_bounded_input?.(); }).catch(() => {});
            }
            await save();
        },
    };
}

module.exports = { prepareSh1Lifecycle, installSh1PhaseCapture };
