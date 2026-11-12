// turts-test-role: harness
"use strict";

// A row can store player settings before the client starts, in the same browser settings
// document that the client loads and saves (`turts.v1.settings`, volume_settings.rs). This is a
// stored player setting, not a hold: the client reads it through its normal settings path. Only
// SLOT1 uses it (scripts/mc_browser_params.py refuses it for other rows): three clients on one
// host store the frame limit 30 (row file, client_settings.reason).

const SETTINGS_KEY = "turts.v1.settings";

// The init script. It writes the document only when the window has none, as a first start does.
function seedStoredClientSettings({ key, controls }) {
    try {
        if (window.localStorage.getItem(key) !== null) return;
        window.localStorage.setItem(key, JSON.stringify({ controls }));
    } catch (error) {
        window.__turts_settings_seed_error = String(error && error.message || error);
    }
}

// The seed for a row plan, or null when the row stores no settings.
function clientSettingsSeed(task16) {
    const settings = task16?.client_settings;
    if (!settings) return null;
    if (!Number.isInteger(settings.frame_limit)) throw new Error("client_settings.frame_limit must be an integer");
    return { key: SETTINGS_KEY, controls: { frame_limit: settings.frame_limit } };
}

// Read the stored document of one window and check that it holds the seeded frame limit.
async function assertStoredClientSettings(page, seed, windowNumber) {
    const stored = await page.evaluate((key) => ({
        value: window.localStorage.getItem(key),
        error: window.__turts_settings_seed_error || null,
    }), seed.key);
    if (stored.error) throw new Error(`HARNESS settings seed failed in window ${windowNumber}: ${stored.error}`);
    let frameLimit = null;
    try { frameLimit = JSON.parse(stored.value)?.controls?.frame_limit ?? null; } catch {}
    if (frameLimit !== seed.controls.frame_limit) {
        throw new Error(`HARNESS window ${windowNumber} stored frame_limit ${frameLimit}, expected ${seed.controls.frame_limit}`);
    }
    return { window: windowNumber, key: seed.key, frame_limit: frameLimit };
}

module.exports = { SETTINGS_KEY, seedStoredClientSettings, clientSettingsSeed, assertStoredClientSettings };
