// turts-test-role: unit
"use strict";

const assert = require("node:assert/strict");
const { sourceControls } = require("./shell-oracle.js");

const check = sourceControls.assertSlot1DeadlineChecks;
assert(check, "the fixed SLOT1 oracle exports its deadline evaluator for source controls");
const checkHellos = sourceControls.assertSlot1HelloTimes;
assert(checkHellos, "the fixed SLOT1 oracle exports its Hello deadline evaluator for source controls");

function fixture() {
    const anchor = 100;
    const firstTerminalAt = anchor + 15.0165;
    return {
        first_transport_attempt_real_secs: anchor,
        terminal_at_real_secs: firstTerminalAt,
        terminal_elapsed_secs: firstTerminalAt - anchor,
        deadline_checks_causal: true,
        last_pending_deadline_check: {
            check_index: 41,
            frame_index: 650,
            source: "advance_rejoin",
            real_now_secs: anchor + 14.999,
            elapsed_secs: anchor + 14.999 - anchor,
        },
        first_terminal_deadline_check: {
            check_index: 42,
            frame_index: 651,
            source: "advance_rejoin",
            real_now_secs: firstTerminalAt,
            elapsed_secs: firstTerminalAt - anchor,
        },
    };
}

{
    const result = check(fixture());
    assert.equal(result.crossingGapSecs, fixture().first_terminal_deadline_check.real_now_secs - 115);
    assert.equal(result.observedElapsedSecs, fixture().terminal_elapsed_secs);
}
{
    const sample = fixture();
    sample.first_terminal_deadline_check.real_now_secs = 114.1;
    sample.first_terminal_deadline_check.elapsed_secs = 114.1 - sample.first_transport_attempt_real_secs;
    sample.terminal_at_real_secs = 114.1;
    sample.terminal_elapsed_secs = 114.1 - sample.first_transport_attempt_real_secs;
    assert.throws(() => check(sample), /at or after the exact 15-second cutoff/,
        "an early 14.1-second terminal cannot qualify");
}
{
    const sample = fixture();
    sample.first_terminal_deadline_check.check_index += 2;
    assert.throws(() => check(sample), /intermediate deadline check/,
        "an extra eligible deadline update cannot be skipped");
}
{
    const sample = fixture();
    sample.first_terminal_deadline_check.elapsed_secs = 15;
    sample.terminal_elapsed_secs = 15;
    assert.throws(() => check(sample), /unclamped elapsed time/,
        "a stale or clamped 15-second sample cannot replace the actual elapsed timestamp");
}
{
    const sample = fixture();
    sample.last_pending_deadline_check.frame_index = null;
    assert.throws(() => check(sample), /frame identities/,
        "missing actual frame identity fails closed");
}
{
    assert.doesNotThrow(() => checkHellos([0, 1, 14.999]),
        "actual Hello timestamps before the exact deadline with one-second cadence qualify");
    assert.throws(() => checkHellos([0, 15]), /before the absolute 15-second cutoff/,
        "a two-sample Hello sequence with the second send at 15.0 seconds is rejected");
    assert.throws(() => checkHellos([0, 15.01]), /before the absolute 15-second cutoff/,
        "a Hello sent after 15.0 seconds is rejected");
}
