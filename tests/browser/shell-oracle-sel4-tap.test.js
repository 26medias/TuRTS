// turts-test-role: unit
"use strict";

const assert = require("node:assert/strict");
const test = require("node:test");
const { sourceControls } = require("./shell-oracle.js");

// The SEL4 double tap is one key step with options.taps (tests/browser/harness/driver-io.js tapKey). The oracle
// re-reads the page timestamps and names the instrument when the gap is over the 400 ms product window
// (crates/turts_interaction/src/lib.rs:1626), before any product clause can blame the product.
const tapEntry = (times, options = { taps: 2, gap_ms: 80, window_ms: 400 }) => ({
    call: "key", args: ["Digit1", [], options], window: 1, document_epoch: 1,
    actual_events: times.flatMap((time, index) => [
        { type: "keydown", code: "Digit1", seq: 2 * index + 1, wall_time_ms: time },
        { type: "keyup", code: "Digit1", seq: 2 * index + 2, wall_time_ms: time + 20 },
    ]),
});
const single = () => ({ call: "key", args: ["Digit1", [], {}], window: 1, document_epoch: 1, actual_events: [{ type: "keydown", code: "Digit1", seq: 1, wall_time_ms: 5 }] });

test("the receipt of the tap step gives its index and the page-measured gap", () => {
    const receipt = sourceControls.sel4DoubleTapReceipt([single(), { call: "wait", args: [100] }, tapEntry([1000, 1090])]);
    assert.equal(receipt.index, 2);
    assert.equal(receipt.gap_ms, 90);
});

test("a gap over the window fails with the instrument text, not a product text", () => {
    assert.throws(() => sourceControls.sel4DoubleTapReceipt([tapEntry([1000, 1432])]),
        /^Error: double tap took 432 ms, over the 400 ms window: host or driver fault$/);
});

test("a row that widens the window past the product window is refused", () => {
    assert.throws(() => sourceControls.sel4DoubleTapReceipt([tapEntry([1000, 1090], { taps: 2, gap_ms: 80, window_ms: 2000 })]),
        /window must be the product window of 400 ms/);
});

test("a missing step and a wrong press count are instrument faults", () => {
    assert.throws(() => sourceControls.sel4DoubleTapReceipt([single(), single()]), /double tap step is absent from the input log: instrument fault/);
    assert.throws(() => sourceControls.sel4DoubleTapReceipt([tapEntry([1000])]), /the browser saw 1 keydown events, expected 2: instrument fault/);
});
