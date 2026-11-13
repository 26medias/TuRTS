// turts-test-role: pure
"use strict";

const assert = require("node:assert/strict");

// Receipts of a tap step (a key step with options taps, gap_ms and window_ms: tests/browser/harness/driver-io.js
// tapKey). The window is the product window of a double tap (crates/turts_interaction/src/lib.rs, double_click_ms).
// Both checks name the instrument: a slow tap or two presses in one frame is a fault of the host or the driver.

// The page keydown times of the entry: the count, the pinned window and every gap against the window.
function tapGapReceipt(entry, { code, windowMs }) {
    const options = entry?.args?.[2] ?? {};
    assert.equal(options.window_ms, windowMs,
        `double tap window must be the product window of ${windowMs} ms, the row says ${options.window_ms}: instrument fault`);
    const downs = (entry.actual_events || []).filter((event) => event.type === "keydown" && event.code === code);
    assert(downs.length === options.taps, `double tap: the browser saw ${downs.length} keydown events, expected ${options.taps}: instrument fault`);
    const gaps = downs.slice(1).map((event, index) => event.wall_time_ms - downs[index].wall_time_ms);
    assert(gaps.every((gap) => Number.isFinite(gap) && gap >= 0), "double tap: the keydown times are not numbers: instrument fault");
    const over = gaps.find((gap) => !(gap <= windowMs));
    if (over !== undefined) throw new Error(`double tap took ${over} ms, over the ${windowMs} ms window: host or driver fault`);
    return { downs, gaps_ms: gaps };
}

// The retained product frames of a debug record (recent and retained_input), by frame index.
function retainedFrames(debug) {
    const byIndex = new Map();
    for (const frame of [...(debug?.frames?.recent || []), ...(debug?.frames?.retained_input || [])]) {
        if (Number.isSafeInteger(frame.frame_index)) byIndex.set(frame.frame_index, frame);
    }
    return [...byIndex.values()].sort((left, right) => left.frame_index - right.frame_index);
}

// The product reads its clock once for each frame (crates/turts_client/src/core_adapter.rs, now_ms), so two presses in
// one frame have a gap of 0 ms and prove nothing about the window. Each press must be applied in its own retained frame.
function assertPressesInDifferentFrames(frames, downs) {
    const frameOf = (down) => frames.find((frame) => Array.isArray(frame.applied_seq) && frame.applied_seq.includes(down.seq));
    const found = downs.map((down) => {
        const frame = frameOf(down);
        assert(frame, `double tap: no retained product frame holds keydown seq ${down.seq}: instrument fault`);
        return frame.frame_index;
    });
    assert.equal(new Set(found).size, found.length,
        `double tap: the presses reached the product in frames ${found.join(" and ")}: the frame hold is back or the host stalled: instrument fault`);
    return found;
}

module.exports = { tapGapReceipt, retainedFrames, assertPressesInDifferentFrames };
