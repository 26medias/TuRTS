// turts-test-role: unit
"use strict";

const assert = require("node:assert/strict");
const { installIoEventObserver } = require("./io-event-observer.js");

function exercise(options = {}) {
    const canvas = { tagName: "CANVAS", contains: () => false };
    const other = { tagName: "BODY" };
    const win = {
        __turts_io_actual_events: [],
        __turts_io_ambient_events: [],
        __turts_io_dispatching: options.dispatching !== false,
        __turts_io_expected_event: {
            seq: 17, type: options.expectedType || options.type || "pointermove",
            client_x: 1, client_y: 360, ...options.expected,
        },
        __turts_io_active_call: { call: options.call || "move" },
        __turts_io_downstream_events: [],
    };
    const listeners = new Map();
    const doc = {
        querySelector: () => canvas,
        getElementById: () => canvas,
        pointerLockElement: options.locked === false ? null : options.otherLock ? other : canvas,
        addEventListener(type, callback) {
            listeners.set(type, [...(listeners.get(type) || []), callback]);
        },
    };
    const old = { window: global.window, document: global.document, sessionStorage: global.sessionStorage };
    global.window = win;
    global.document = doc;
    global.sessionStorage = { getItem: () => null, setItem: () => {} };
    try {
        installIoEventObserver();
        win.__turts_io_dispatching = options.dispatching !== false;
        win.__turts_io_expected_event = {
            seq: 17, type: options.expectedType || options.type || "pointermove",
            client_x: 1, client_y: 360, ...options.expected,
        };
        win.__turts_io_active_call = { call: options.call || "move" };
        const event = {
            target: options.foreignTarget ? other : canvas,
            clientX: 640, clientY: 360, movementX: -639, movementY: 0,
            shiftKey: false, ctrlKey: false, altKey: false, metaKey: false,
            code: undefined, key: undefined, deltaY: undefined,
            ...options.event,
        };
        const type = options.type || "pointermove";
        for (const callback of listeners.get(type) || []) callback(event);
        if (options.repeat) for (const callback of listeners.get(type) || []) callback(event);
        return win;
    } finally {
        if (old.window === undefined) delete global.window;
        else global.window = old.window;
        if (old.document === undefined) delete global.document;
        else global.document = old.document;
        if (old.sessionStorage === undefined) delete global.sessionStorage;
        else global.sessionStorage = old.sessionStorage;
    }
}

const controls = [
    ["actual locked relative motion", {}, 1],
    ["unlocked exact absolute motion", { locked: false, expected: { client_x: 640 } }, 1],
    ["unlocked wrong absolute motion", { locked: false }, 0],
    ["locked foreign target", { foreignTarget: true }, 0],
    ["lock belongs to another element", { otherLock: true }, 0],
    ["missing actual relative x", { event: { movementX: undefined } }, 0],
    ["nonfinite actual relative y", { event: { movementY: Number.NaN } }, 0],
    ["wrong physical event type", { expectedType: "pointerdown" }, 0],
    ["outside physical dispatch", { dispatching: false }, 0],
    ["same actual event cannot match twice", { repeat: true }, 1],
    ["locked actual pointerdown", { type: "pointerdown", event: { movementX: 0, movementY: 0 } }, 1],
    ["locked actual pointerup", { type: "pointerup", event: { movementX: 0, movementY: 0 } }, 1],
    ["matching physical keyboard", { type: "keydown", expectedType: "keydown",
        expected: { client_x: undefined, client_y: undefined, code: "KeyQ" }, event: { code: "KeyQ" } }, 1],
    ["wrong physical keyboard", { type: "keydown", expectedType: "keydown",
        expected: { client_x: undefined, client_y: undefined, code: "KeyQ" }, event: { code: "KeyA" } }, 0],
];

for (const [name, options, count] of controls) {
    assert.equal(exercise(options).__turts_io_actual_events.length, count, name);
}
const observed = exercise().__turts_io_actual_events[0];
assert.equal(observed.client_x, 640, "keep actual locked client coordinate");
assert.equal(observed.client_y, 360, "keep actual locked client coordinate");
assert.equal(observed.movement_x, -639, "keep actual browser relative movement");
assert.equal(observed.movement_y, 0);
assert.equal(observed.canvas_pointer_locked, true);
assert.equal(observed.seq, 17);
