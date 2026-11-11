// turts-test-role: unit
"use strict";

const assert = require("node:assert/strict");
const vm = require("node:vm");
const { installContextMenuObserver } = require("./contextmenu-observer.js");

const listeners = new Map();
const scheduled = [];
const canvas = {
    tagName: "CANVAS",
    id: "turts",
    contains(target) { return target === this; },
};
const window = {
    __turts_io_seq: 23,
    __turts_io_active_call: { call: "click", args: [90, 100, "right"], call_index: 7 },
};
const document = {
    addEventListener(type, listener, capture) { listeners.set(type, { listener, capture }); },
    querySelector(selector) { return selector === "canvas" ? canvas : null; },
};
vm.runInNewContext(`(${installContextMenuObserver.toString()})();`, {
    window,
    document,
    Date: { now: () => 123456 },
    setTimeout(callback) { scheduled.push(callback); },
});

const receiptListener = listeners.get("contextmenu");
assert(receiptListener && receiptListener.capture === true, "observer must receive the real event during capture");
const event = { target: canvas, cancelable: true, defaultPrevented: false };
receiptListener.listener(event);
window.__turts_io_seq = 99;
window.__turts_io_active_call = null;
event.defaultPrevented = true;
scheduled.shift()();

assert.deepEqual({ ...window.__turts_io_contextmenu_events[0] }, {
    seq: 23,
    source_call_index: 7,
    target: "CANVAS",
    target_id: "turts",
    canvas_target: true,
    wall_time_ms: 123456,
    cancelable: true,
    default_prevented: true,
});

window.__turts_io_seq = null;
window.__turts_io_active_call = { call: "key", args: ["KeyQ"], call_index: 8 };
const unrelated = { target: canvas, cancelable: false, defaultPrevented: false };
receiptListener.listener(unrelated);
window.__turts_io_seq = 101;
window.__turts_io_active_call = null;
scheduled.shift()();
assert.equal(window.__turts_io_contextmenu_events[1].seq, null, "observer retains an absent actual sequence as null");
assert.equal(window.__turts_io_contextmenu_events[1].source_call_index, null, "observer does not invent a physical source call");
assert.equal(window.__turts_io_contextmenu_events[1].default_prevented, false, "post-dispatch outcome remains tied to its own event");
