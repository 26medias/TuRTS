// turts-test-role: pure
"use strict";

// What an oracle asks of one sound-switch click made by `harness/sound-switch-pixels.js`. The
// checkpoints are the scene checkpoints of the two screenshots (`<label>-before.png` and
// `<label>-after.png`); `click` is the io log entry of the click between them.
const assert = require("node:assert/strict");

function switchRow(debug) {
    const rows = (debug?.ui_snapshot || []).filter((row) => row.kind === "sound_switch");
    assert.equal(rows.length, 1, "the debug ui snapshot has exactly one sound_switch row");
    return rows[0];
}

// The words that the switch must show for a debug state: on only when the player switched the
// sound on and every browser audio context runs after a gesture.
function expectedWords(audio) {
    const running = audio.gesture_seen === true && audio.context_states?.length > 0 &&
        audio.context_states.every((state) => state === "running");
    return audio.sound_on === true && running ? "Sound: on" : "Sound: off";
}

function assertSwitchClick({ label, before, after, click, fromOn }) {
    const rowBefore = switchRow(before.debug);
    const rowAfter = switchRow(after.debug);
    assert.equal(before.debug.audio.sound_on, fromOn, `${label}: the debug state shows the switch ${fromOn ? "on" : "off"} before the click`);
    assert.equal(after.debug.audio.sound_on, !fromOn, `${label}: the debug state shows the switch ${fromOn ? "off" : "on"} after one click`);
    assert.equal(rowBefore.visible, true, `${label}: the switch is visible before the click`);
    assert.equal(rowBefore.capture_root, true, `${label}: a click on the switch does not reach the world`);
    assert.equal(rowBefore.text, expectedWords(before.debug.audio), `${label}: the switch says its state in words before the click`);
    assert.equal(rowAfter.text, expectedWords(after.debug.audio), `${label}: the switch says its state in words after the click`);
    assert.notEqual(rowBefore.text, rowAfter.text, `${label}: one click changes the words`);
    assert.equal(click?.call, "click", `${label}: the switch is clicked with the pointer`);
    assert.equal(click.args?.[2], "left");
    const scale = Number(before.scene?.scale_factor) || 1;
    const release = click.actual_events?.find((event) => event.type === "pointerup" && event.canvas_target);
    assert(release && Number.isInteger(release.seq), `${label}: the click has a trusted canvas release`);
    const [left, top, width, height] = rowBefore.rect;
    assert(release.client_x * scale >= left && release.client_x * scale <= left + width &&
        release.client_y * scale >= top && release.client_y * scale <= top + height,
    `${label}: the physical release lands inside the rectangle of the switch`);
    const frame = (after.debug.frames?.retained_input || []).filter((candidate) => candidate.applied_seq?.includes(release.seq));
    assert.equal(frame.length, 1, `${label}: the release maps to one presented frame`);
    assert.deepEqual(frame[0].emitted_orders || [], [], `${label}: the click on the switch sends no world order`);
    return { release, frame: frame[0] };
}

// The one click between the saves `<label>-before.png` and `<label>-after.png` of a click made by
// `clickSoundSwitch`, with its two checkpoints. `checkpointFor(name)` gives the scene checkpoint of
// the screenshot that was saved as `name`.
function assertSwitchClickByLabel(input, checkpointFor, label, fromOn) {
    const saveAt = (name) => input.findIndex((entry) => entry.call === "save" && entry.args?.[0] === name);
    const beforeAt = saveAt(`${label}-before.png`), afterAt = saveAt(`${label}-after.png`);
    assert(beforeAt >= 0 && afterAt > beforeAt, `${label}: both screenshots are saved, the first before the second`);
    const clicks = input.slice(beforeAt + 1, afterAt).filter((entry) => entry.call === "click");
    assert.equal(clicks.length, 1, `${label}: exactly one click between the two screenshots`);
    const before = checkpointFor(`${label}-before.png`), after = checkpointFor(`${label}-after.png`);
    return { before, after, clickIndex: input.indexOf(clicks[0]), ...assertSwitchClick({ label, before, after, click: clicks[0], fromOn }) };
}

module.exports = { assertSwitchClick, assertSwitchClickByLabel, expectedWords, switchRow };
