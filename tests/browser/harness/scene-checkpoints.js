// turts-test-role: harness
"use strict";

// Only the harness keeps these observations. Driver IO still returns PNG bytes alone.
function createSceneCheckpoints(pages, limit = 256, documentEpochs = null) {
    const rows = [];
    async function record(metadata) {
        if (!Number.isSafeInteger(metadata.input_call) || metadata.input_call < 0 ||
            !Number.isSafeInteger(metadata.window) || metadata.window < 1 || metadata.window > pages.length ||
            !Number.isFinite(metadata.wall_time_ms)) throw new Error("invalid scene checkpoint identity");
        if (documentEpochs && (!Number.isSafeInteger(metadata.document_epoch) || metadata.document_epoch < 1 ||
            documentEpochs[metadata.window - 1] !== metadata.document_epoch)) throw new Error("invalid scene checkpoint document identity");
        if (rows.length >= limit) throw new Error("scene checkpoint limit exceeded");
        const raw = await pages[metadata.window - 1].evaluate(async () => {
            // The client builds the debug state only when asked (design D 9, row DBG1). Ask here
            // and wait for one record: the promise resolves in the frame that published it, so
            // the scene read below is of the same frame. A held frame (the SH1 phase capture)
            // runs no callback, so the record that the held frame published is read as it is.
            // A page with no request bridge is an older build that publishes in each frame.
            const held = typeof window.__turts_input_held === "function" && window.__turts_input_held();
            const debug = typeof window.turts_debug !== "function" ? null
                : (!held && typeof window.turts_debug_request === "function")
                    ? await window.turts_debug_request() : window.turts_debug();
            return {
                // The page clock of this read (the legacy t7.js poll read it in the same call), so a
                // consumer gets a time that is not earlier than the frame it observes.
                page_time_ms: typeof performance === "object" ? performance.timeOrigin + performance.now() : null,
                client_error: window.turts_error || null,
                scene: typeof window.turts_scene === "function" ? window.turts_scene() : null,
                debug,
                browser: (() => {
                    const canvas = document.getElementById("turts") || document.querySelector("canvas");
                    const active = document.activeElement;
                    const pointerLock = document.pointerLockElement;
                    const identity = (element) => element ? {
                        tag_name: typeof element.tagName === "string" ? element.tagName : null,
                        id: typeof element.id === "string" && element.id ? element.id : null,
                    } : null;
                    return {
                        document_has_focus: document.hasFocus(),
                        device_pixel_ratio: window.devicePixelRatio,
                        active_element: identity(active),
                        canvas: identity(canvas),
                        canvas_focused: Boolean(canvas && active === canvas),
                        pointer_lock_element: identity(pointerLock),
                        canvas_pointer_locked: Boolean(canvas && pointerLock === canvas),
                    };
                })(),
            };
        });
        const parse = (value, name) => {
            const parsed = typeof value === "string" ? JSON.parse(value) : value;
            if (!parsed || typeof parsed !== "object" || Array.isArray(parsed)) throw new Error(`missing ${name} checkpoint bridge`);
            return parsed;
        };
        // A client error during the run stops it, as the legacy t7.js poll did (MC task 21, M4).
        if (raw.client_error) throw new Error(`browser client failed: ${raw.client_error}`);
        rows.push({ ...metadata, ...(documentEpochs ? { document_epoch: documentEpochs[metadata.window - 1] } : {}),
            page_time_ms: Number.isFinite(raw.page_time_ms) ? raw.page_time_ms : null,
            scene: parse(raw.scene, "scene"), debug: parse(raw.debug, "debug"), browser: parse(raw.browser, "browser") });
    }
    return { record, snapshots: () => structuredClone(rows) };
}

module.exports = { createSceneCheckpoints };
