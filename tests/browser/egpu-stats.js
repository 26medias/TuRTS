// turts-test-role: pure
"use strict";

// Frame statistics of the E-GPU measurement (MP1 task E-GPU, design section 14 "One GPU for a
// two-client match"). Pure functions: no browser, no file. egpu-run.js feeds them the
// requestAnimationFrame timestamps (ms) of one window.

// The fail rule of the task: a window under 1 frame each second for 5 s. A window of 5 s that holds
// fewer than 5 frames is under 1 frame each second on average, so it counts as a fail. This also
// covers the narrower reading (5 whole seconds in a row with no frame), which is reported apart
// as longest_run_of_empty_seconds.
const FAIL_WINDOW_S = 5;

function percentile(sorted, p) {
    if (sorted.length === 0) return null;
    return sorted[Math.floor(p * (sorted.length - 1))];
}

// Frame times in ms: the intervals between consecutive timestamps.
function intervals(timestamps) {
    const out = [];
    for (let i = 1; i < timestamps.length; i += 1) out.push(timestamps[i] - timestamps[i - 1]);
    return out;
}

// Count of frames in each whole second of [0, durationS). A timestamp is in ms from the start.
function secondBins(timestamps, durationS) {
    const bins = new Array(durationS).fill(0);
    for (const ts of timestamps) {
        const index = Math.floor(ts / 1000);
        if (index >= 0 && index < durationS) bins[index] += 1;
    }
    return bins;
}

// Smallest frame count in a sliding window of `width` whole seconds (step 1 s).
function minWindowFrames(bins, width) {
    if (bins.length < width) return null;
    let sum = 0;
    for (let i = 0; i < width; i += 1) sum += bins[i];
    let min = sum;
    for (let i = width; i < bins.length; i += 1) {
        sum += bins[i] - bins[i - width];
        if (sum < min) min = sum;
    }
    return min;
}

function longestZeroRun(bins) {
    let best = 0;
    let run = 0;
    for (const count of bins) {
        run = count === 0 ? run + 1 : 0;
        if (run > best) best = run;
    }
    return best;
}

// timestamps: ms from the start of the window's measured period. durationS: the measured length.
function summarize(timestamps, durationS) {
    const sorted = intervals(timestamps).sort((a, b) => a - b);
    const bins = secondBins(timestamps, durationS);
    const minFrames5s = minWindowFrames(bins, FAIL_WINDOW_S);
    const zeroRun = longestZeroRun(bins);
    return {
        frames: timestamps.length,
        duration_s: durationS,
        mean_fps: durationS > 0 ? timestamps.length / durationS : null,
        interval_ms: {
            p50: percentile(sorted, 0.5),
            p95: percentile(sorted, 0.95),
            p99: percentile(sorted, 0.99),
            max: sorted.length ? sorted[sorted.length - 1] : null,
        },
        min_frames_in_5s: minFrames5s,
        lowest_5s_fps: minFrames5s === null ? null : minFrames5s / FAIL_WINDOW_S,
        seconds_with_no_frame: bins.filter((count) => count === 0).length,
        longest_run_of_empty_seconds: zeroRun,
        // true when the window is under 1 frame each second for 5 s (the task's fail condition)
        starved: minFrames5s === null || minFrames5s < FAIL_WINDOW_S,
    };
}

// Event types that count as a client disconnect when they happen in the measured period.
const DISCONNECT_TYPES = ["websocket_close", "page_close", "page_crash", "phase_not_joined"];

// The verdict of the task: FAIL if a window is under 1 frame each second for 5 s, or a client disconnects.
// NO_VERDICT when the run did not set up both windows (nothing was measured).
function judge({ summaries, events, setupError }) {
    const disconnects = events.filter((e) => e.during_measure === true && DISCONNECT_TYPES.includes(e.type)).length;
    if (setupError || summaries.length !== 2) return { verdict: "NO_VERDICT", disconnects };
    const starved = summaries.map((summary) => summary.starved);
    return { verdict: starved.some(Boolean) || disconnects > 0 ? "FAIL" : "PASS", disconnects, starved };
}

module.exports = { FAIL_WINDOW_S, DISCONNECT_TYPES, judge, percentile, intervals, secondBins, minWindowFrames, longestZeroRun, summarize };
