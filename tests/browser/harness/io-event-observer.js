// turts-test-role: harness
"use strict";

// This init script records the real DOM events caused by physical driver IO. Pointer movement
// under browser pointer lock retains the browser's actual client coordinates and movement deltas;
// the planned absolute destination is never substituted for those observations.
function installIoEventObserver() {
    window.__turts_io_actual_events = [];
    window.__turts_io_dispatching = false;
    window.__turts_io_expected_event = null;
    window.__turts_io_downstream_events = [];
    window.__turts_io_active_call = null;
    const ambientKey = "__turts_io_ambient_events";
    let ambient = [];
    try { ambient = JSON.parse(sessionStorage.getItem(ambientKey) || "[]"); } catch {}
    window.__turts_io_ambient_events = Array.isArray(ambient) ? ambient : [];
    const types = ["pointerdown", "pointerup", "pointermove", "keydown", "keyup", "wheel"];
    for (const type of types) document.addEventListener(type, (event) => {
        const canvas = document.getElementById("turts") || document.querySelector("canvas");
        const expected = window.__turts_io_expected_event;
        const observed = {
            type,
            target: event.target?.tagName || null,
            canvas_target: Boolean(canvas && (event.target === canvas || canvas.contains(event.target))),
            canvas_pointer_locked: Boolean(canvas && document.pointerLockElement === canvas),
            movement_x: Number.isFinite(event.movementX) ? event.movementX : null,
            movement_y: Number.isFinite(event.movementY) ? event.movementY : null,
            shift_key: Boolean(event.shiftKey),
            ctrl_key: Boolean(event.ctrlKey),
            alt_key: Boolean(event.altKey),
            meta_key: Boolean(event.metaKey),
            wall_time_ms: Date.now(),
            client_x: Number.isFinite(event.clientX) ? event.clientX : null,
            client_y: Number.isFinite(event.clientY) ? event.clientY : null,
            code: typeof event.code === "string" ? event.code : null,
            key: typeof event.key === "string" ? event.key : null,
            delta_y: Number.isFinite(event.deltaY) ? event.deltaY : null,
        };
        const lockedPointerEvent = ["pointermove", "pointerdown", "pointerup"].includes(type) &&
            observed.canvas_pointer_locked;
        const positionMatches = lockedPointerEvent
            ? observed.canvas_target && Number.isFinite(observed.movement_x) && Number.isFinite(observed.movement_y)
            : ((expected?.client_x === undefined || Math.abs(observed.client_x - expected.client_x) < 0.01) &&
                (expected?.client_y === undefined || Math.abs(observed.client_y - expected.client_y) < 0.01));
        const matches = window.__turts_io_dispatching && !expected?.matched && Number.isInteger(expected?.seq) &&
            expected?.type === type && positionMatches &&
            (expected.code === undefined || observed.code === expected.code) &&
            (expected.key === undefined || observed.key === expected.key) &&
            (expected.delta_y === undefined || Math.abs(observed.delta_y - expected.delta_y) < 0.01);
        if (matches) {
            expected.matched = true;
            window.__turts_io_seq = expected.seq;
            window.__turts_io_actual_events.push({ ...observed, seq: expected.seq });
        } else {
            window.__turts_io_seq = null;
            window.__turts_io_ambient_events.push({ ...observed, seq: null,
                during_dispatch_seq: window.__turts_io_dispatching ? expected?.seq ?? null : null,
                expected_type: expected?.type || null });
            if (window.__turts_io_ambient_events.length > 1000) window.__turts_io_ambient_events.shift();
            try { sessionStorage.setItem(ambientKey, JSON.stringify(window.__turts_io_ambient_events)); } catch {}
        }
    }, true);
    for (const type of types) document.addEventListener(type, (event) => {
        const canvas = document.getElementById("turts") || document.querySelector("canvas");
        window.__turts_io_downstream_events.push({
            type,
            seq: window.__turts_io_seq,
            wall_time_ms: Date.now(),
            canvas_pointer_locked: Boolean(canvas && document.pointerLockElement === canvas),
            movement_x: Number.isFinite(event.movementX) ? event.movementX : null,
            movement_y: Number.isFinite(event.movementY) ? event.movementY : null,
            client_x: Number.isFinite(event.clientX) ? event.clientX : null,
            client_y: Number.isFinite(event.clientY) ? event.clientY : null,
            code: typeof event.code === "string" ? event.code : null,
            key: typeof event.key === "string" ? event.key : null,
            delta_y: Number.isFinite(event.deltaY) ? event.deltaY : null,
            shift_key: Boolean(event.shiftKey),
        });
    }, true);
}

module.exports = { installIoEventObserver };
