// turts-test-role: unit
"use strict";

const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const test = require("node:test");
const { SOUND_SWITCH, findSoundSwitch, turnSoundOn } = require("./sound-switch-pixels.js");
const { assertSwitchClick, expectedWords } = require("./sound-switch-proof.js");

const REPO = path.resolve(__dirname, "../../..");

function image(width = 1280, height = 720) {
    const rgba = new Uint8Array(width * height * 4);
    for (let index = 0; index < rgba.length; index += 4) rgba.set([20, 30, 40, 255], index);
    return { width, height, rgba };
}

function fill(target, left, top, width, height, colour) {
    for (let y = top; y < top + height; y += 1) {
        for (let x = left; x < left + width; x += 1) target.rgba.set([...colour, 255], (y * target.width + x) * 4);
    }
}

test("the finder reads the state and the click point of a switch at the client position", () => {
    for (const state of ["off", "on"]) {
        const shot = image();
        fill(shot, 744, 684, 124, 28, SOUND_SWITCH[state]);
        // The words are white pixels inside the fill: they do not break the find.
        fill(shot, 780, 690, 50, 14, [255, 255, 255]);
        const found = findSoundSwitch(shot);
        assert.equal(found.state, state);
        assert.deepEqual([found.x, found.y], [806, 698]);
        assert.deepEqual(found.bounds, [744, 684, 867, 711]);
    }
});

test("the finder scales with the device pixel ratio", () => {
    const shot = image(2560, 1440);
    fill(shot, 1488, 1368, 248, 56, SOUND_SWITCH.on);
    const found = findSoundSwitch(shot, 2);
    assert.equal(found.state, "on");
    assert.deepEqual([found.x, found.y], [1612, 1396]);
    assert.equal(findSoundSwitch(shot, 1), null, "a block twice the size is not a switch at ratio 1");
});

function frame(target, left, top, colour) {
    fill(target, left, top, 124, 28, colour);
    fill(target, left + 2, top + 2, 120, 24, [33, 66, 77]);
}

test("the finder reads the frame of a package switch at any place", () => {
    for (const [state, left, top] of [["off", 744, 684], ["on", 20, 300], ["off", 1100, 5]]) {
        const shot = image();
        frame(shot, left, top, SOUND_SWITCH[state]);
        fill(shot, left + 30, top + 7, 60, 12, [255, 255, 255]);
        const found = findSoundSwitch(shot);
        assert.equal(found.state, state);
        assert.deepEqual([found.x, found.y], [left + 62, top + 14]);
    }
});

test("the finder refuses a missing switch, a wrong size, a thin line, other fills and a lookalike", () => {
    assert.equal(findSoundSwitch(image()), null);
    const wrongSize = image();
    fill(wrongSize, 744, 684, 60, 28, SOUND_SWITCH.off);
    assert.equal(findSoundSwitch(wrongSize), null);
    const otherFill = image();
    fill(otherFill, 744, 684, 124, 28, [104, 148, 100]);
    assert.equal(findSoundSwitch(otherFill), null, "the ground colour is not a switch");
    const sparse = image();
    for (let x = 744; x < 868; x += 2) sparse.rgba.set([...SOUND_SWITCH.off, 255], (684 * sparse.width + x) * 4);
    sparse.rgba.set([...SOUND_SWITCH.off, 255], (711 * sparse.width + 744) * 4);
    assert.equal(findSoundSwitch(sparse), null, "the corner pixels of a hollow outline are not a button");
    const outline = image();
    fill(outline, 744, 684, 124, 1, SOUND_SWITCH.off);
    fill(outline, 744, 711, 124, 1, SOUND_SWITCH.off);
    fill(outline, 744, 684, 1, 28, SOUND_SWITCH.off);
    fill(outline, 867, 684, 1, 28, SOUND_SWITCH.off);
    assert.equal(findSoundSwitch(outline), null, "a 1 px outline is not a button");
    const split = image();
    // Two blocks of the fill that together span a switch box are not one switch.
    fill(split, 744, 684, 40, 28, SOUND_SWITCH.on);
    fill(split, 828, 684, 40, 28, SOUND_SWITCH.on);
    assert.equal(findSoundSwitch(split), null);
    assert.throws(() => findSoundSwitch({ width: 2, height: 2, rgba: new Uint8Array(3) }), /decoded RGBA/);
});

test("the pixel constants are the values that the client draws", () => {
    const volume = fs.readFileSync(path.join(REPO, "crates/turts_client/src/volume_settings.rs"), "utf8");
    const notices = fs.readFileSync(path.join(REPO, "crates/turts_client/src/client_shell/notices.rs"), "utf8");
    const fillOf = (state) => {
        const body = volume.match(/pub fn sound_fill\(shown_on: bool\) -> Color \{\s*if shown_on \{\s*Color::srgb\(([^)]*)\)\s*\} else \{\s*Color::srgb\(([^)]*)\)/);
        assert(body, "sound_fill keeps its two-colour form");
        return body[state === "on" ? 1 : 2].split(",").map((value) => Math.round(Number(value) * 255));
    };
    for (const state of ["off", "on"]) {
        fillOf(state).forEach((channel, index) => assert(Math.abs(channel - SOUND_SWITCH[state][index]) <= 1, `${state} fill channel ${index}`));
    }
    assert.equal(Number(notices.match(/SOUND_SWITCH_WIDTH: f32 = ([0-9.]+);/)[1]), SOUND_SWITCH.width);
    assert.equal(Number(notices.match(/SOUND_SWITCH_HEIGHT: f32 = ([0-9.]+);/)[1]), SOUND_SWITCH.height);
});

test("the six packages place the switch with the fills and the size that the finder reads", () => {
    const hex = (colour) => `#${colour.map((channel) => channel.toString(16).padStart(2, "0")).join("")}`;
    for (const name of ["muster", "forge", "base", "cedar", "haul", "acme"]) {
        const text = fs.readFileSync(path.join(REPO, "mods", name, "interface/hud.ron"), "utf8");
        for (const [id, state] of [["sound_switch_off", "off"], ["sound_switch_on", "on"]]) {
            const line = text.split("\n").find((candidate) => candidate.includes(`id: "${id}"`));
            assert(line, `${name} places ${id}`);
            assert(line.includes(`size: (w: Px(${SOUND_SWITCH.width}.0), h: Px(${SOUND_SWITCH.height}.0))`), `${name} ${id} size`);
            assert(line.includes(`Colour("${hex(SOUND_SWITCH[state])}")`), `${name} ${id} fill ${hex(SOUND_SWITCH[state])}`);
        }
        assert.equal((text.match(/Input\("toggle_sound"\)/g) || []).length, 2, `${name} has the two buttons`);
        const manifest = fs.readFileSync(path.join(REPO, "mods", name, "manifest.ron"), "utf8");
        assert(manifest.includes('engine_api: "^0.2.4"'), `${name} needs the API that has the switch names`);
    }
});

// A fake `io`: each screenshot is a token, and `decode` turns a token into a synthetic image.
function fakeIo(shows) {
    const log = [];
    let taken = 0;
    return {
        log,
        async screenshot() {
            const state = shows[Math.min(taken, shows.length - 1)];
            taken += 1;
            log.push(["screenshot"]);
            return Buffer.from(state || "none");
        },
        async click(x, y, button) { log.push(["click", x, y, button]); },
        async save(name) { log.push(["save", name]); },
        async wait() {},
    };
}

function decodeToken(buffer) {
    const state = buffer.toString();
    const shot = image();
    if (state !== "none") fill(shot, 744, 684, 124, 28, SOUND_SWITCH[state]);
    return shot;
}

test("turnSoundOn clicks the centre of an off switch once and waits for the on fill", async () => {
    // The first screenshot shows no switch (the page is still loading), then off, then the click
    // takes effect one screenshot late.
    const io = fakeIo(["none", "off", "off", "on"]);
    const result = await turnSoundOn(io, "t1", decodeToken);
    assert.equal(result.before.state, "off");
    assert.equal(result.after.state, "on");
    assert.deepEqual(io.log.filter((entry) => entry[0] === "click"), [["click", 806, 698, "left"]]);
    assert.deepEqual(io.log.filter((entry) => entry[0] === "save").map((entry) => entry[1]), ["t1-before.png", "t1-after.png"]);
    const clickAt = io.log.findIndex((entry) => entry[0] === "click");
    assert(io.log.slice(0, clickAt).some((entry) => entry[0] === "screenshot"), "it reads pixels before it clicks");
});

test("turnSoundOn stops with an error when the switch never shows or never changes", async () => {
    const realNow = Date.now;
    let clock = 0;
    Date.now = () => (clock += 4000); // each read of the clock is 4 s later: the deadlines pass
    try {
        await assert.rejects(turnSoundOn(fakeIo(["none"]), "t2", decodeToken), /did not show "off"/);
        await assert.rejects(turnSoundOn(fakeIo(["off"]), "t3", decodeToken), /after the click: the sound switch did not show "on".*shows "off"/);
        await assert.rejects(turnSoundOn(fakeIo(["on"]), "t4", decodeToken), /before the click: the sound switch did not show "off".*shows "on"/);
    } finally {
        Date.now = realNow;
    }
});

test("the expected words follow the debug audio state", () => {
    const running = ["running"];
    assert.equal(expectedWords({ sound_on: false, gesture_seen: true, context_states: running }), "Sound: off");
    assert.equal(expectedWords({ sound_on: true, gesture_seen: true, context_states: running }), "Sound: on");
    assert.equal(expectedWords({ sound_on: true, gesture_seen: true, context_states: ["suspended"] }), "Sound: off");
    assert.equal(expectedWords({ sound_on: true, gesture_seen: false, context_states: running }), "Sound: off");
});

function checkpoint(soundOn, text, extra = {}) {
    return {
        scene: { scale_factor: 1 },
        debug: {
            audio: { sound_on: soundOn, gesture_seen: true, context_states: ["running"] },
            ui_snapshot: [{ kind: "sound_switch", id: "sound_switch", rect: [744, 684, 124, 28], visible: true, capture_root: true, text }],
            frames: { retained_input: [{ applied_seq: [7], emitted_orders: [] }] },
            ...extra,
        },
    };
}

const CLICK = { call: "click", args: [806, 698, "left"], actual_events: [{ type: "pointerup", canvas_target: true, seq: 7, client_x: 806, client_y: 698 }] };

test("the proof accepts one click that flips the switch and refuses the usual faults", () => {
    const before = checkpoint(false, "Sound: off");
    const after = checkpoint(true, "Sound: on");
    assertSwitchClick({ label: "t", before, after, click: CLICK, fromOn: false });
    assert.throws(() => assertSwitchClick({ label: "t", before, after: checkpoint(false, "Sound: off"), click: CLICK, fromOn: false }), /after one click/);
    assert.throws(() => assertSwitchClick({ label: "t", before, after: checkpoint(true, "Sound: off"), click: CLICK, fromOn: false }), /words after the click/);
    const away = { ...CLICK, actual_events: [{ ...CLICK.actual_events[0], client_x: 100 }] };
    assert.throws(() => assertSwitchClick({ label: "t", before, after, click: away, fromOn: false }), /inside the rectangle/);
    const ordered = checkpoint(true, "Sound: on", { frames: { retained_input: [{ applied_seq: [7], emitted_orders: [{ seq: 1 }] }] } });
    assert.throws(() => assertSwitchClick({ label: "t", before, after: ordered, click: CLICK, fromOn: false }), /no world order/);
    assert.throws(() => assertSwitchClick({ label: "t", before: checkpoint(true, "Sound: on"), after, click: CLICK, fromOn: false }), /before the click/);
});

test("the proof reads the button of a package as the switch, and refuses two switches or none", () => {
    const placed = (text, visible = true) => ({ kind: "button", id: "sound_off_button", rect: [744, 684, 124, 28], visible, capture_root: true, text, widget_action: { kind: "input", name: "toggle_sound" } });
    const make = (soundOn, rows) => ({
        ...checkpoint(soundOn, soundOn ? "Sound: on" : "Sound: off"),
        debug: { ...checkpoint(soundOn, "").debug, ui_snapshot: rows },
    });
    const before = make(false, [placed("Sound: off"), placed("Sound: on", false)]);
    const after = make(true, [placed("Sound: off", false), placed("Sound: on")]);
    assertSwitchClick({ label: "p", before, after, click: CLICK, fromOn: false });
    assert.throws(() => assertSwitchClick({ label: "p", before: make(false, []), after, click: CLICK, fromOn: false }), /exactly one sound switch/);
    const both = make(false, [placed("Sound: off"), { kind: "sound_switch", id: "sound_switch", rect: [1, 1, 1, 1], visible: true, capture_root: true, text: "Sound: off" }]);
    assert.throws(() => assertSwitchClick({ label: "p", before: both, after, click: CLICK, fromOn: false }), /exactly one sound switch/);
});
