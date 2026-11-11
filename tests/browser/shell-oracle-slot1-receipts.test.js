// turts-test-role: unit
"use strict";

const assert = require("node:assert/strict");
const { sourceControls } = require("./shell-oracle.js");

const check = sourceControls.assertRequiredPhysicalReceipts;
assert(check, "SLOT1 physical receipts are checked by the fixed public oracle");

function fixture() {
    const input = [
        { call: "use", args: [3], window: 3, document_epoch: 3 },
        { call: "screenshot", args: [], window: 3, document_epoch: 3 },
        { call: "use", args: [3], window: 3, document_epoch: 3 },
        { call: "reload", args: [], window: 1, document_epoch: 1 },
    ];
    const captures = [{
        name: "unclaimed-C-refusal", phase: "capacity", window: 3,
        screenshot_input_call: 1, document_epoch: 3,
    }];
    return { input, captures };
}

{
    const { input, captures } = fixture();
    assert.doesNotThrow(() => check("SLOT1", input, captures),
        "the actual capacity use(3) call before C's same-epoch capture is retained");
}
{
    const { input, captures } = fixture();
    input.shift();
    assert.throws(() => check("SLOT1", input, captures),
        /unclaimed-third-refusal required physical use receipt is missing/,
        "a later copied-tab use(3) cannot stand in for the capacity-phase receipt");
}
{
    const { input, captures } = fixture();
    captures[0].phase = "copy-refusal";
    assert.throws(() => check("SLOT1", input, captures),
        /capture belongs to the wrong physical phase/,
        "the actual use(3) receipt must be bound to the capacity capture");
}
{
    const { input, captures } = fixture();
    captures[0].document_epoch = 4;
    assert.throws(() => check("SLOT1", input, captures),
        /required physical use receipt is missing/,
        "a receipt from another document epoch cannot qualify C's capture");
}
