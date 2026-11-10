// turts-test-role: unit
"use strict";

const assert = require("node:assert/strict");
const fs = require("node:fs");
const vm = require("node:vm");

const html = fs.readFileSync("crates/turts_client_web/index.html", "utf8");
const scripts = [...html.matchAll(/<script(?:\s[^>]*)?>([\s\S]*?)<\/script>/gi)].map((match) => match[1]);
assert.ok(scripts.length > 0, "the web entry has an inline script before wasm startup");

class FakeAudioContext {
    constructor() {
        this.state = "suspended";
        this.resumeCalls = 0;
    }

    resume() {
        this.resumeCalls += 1;
        this.state = "running";
        return Promise.resolve();
    }
}

function createHarness() {
    const listeners = new Map();
    const canvas = { id: "turts" };
    let resumeAllowed = true;
    class FakeAudioContext {
        constructor() {
            this.state = "suspended";
            this.resumeCalls = 0;
        }

        resume() {
            this.resumeCalls += 1;
            if (resumeAllowed) this.state = "running";
            return Promise.resolve();
        }
    }
    const document = {
        getElementById(id) {
            return id === "turts" ? canvas : null;
        },
        addEventListener(type, callback) {
            const callbacks = listeners.get(type) || [];
            callbacks.push(callback);
            listeners.set(type, callbacks);
        },
        removeEventListener(type, callback) {
            listeners.set(type, (listeners.get(type) || []).filter((entry) => entry !== callback));
        },
    };
    const sandbox = {
        document,
        AudioContext: FakeAudioContext,
        console,
    };
    sandbox.window = sandbox;
    for (const script of scripts) vm.runInNewContext(script, sandbox, { filename: "index.html" });
    return {
        AudioContext: sandbox.AudioContext,
        contexts() {
            return Array.from(sandbox.__turts_audio_contexts || []);
        },
        contextStates() {
            const snapshot = sandbox.__turts_audio_context_states;
            return typeof snapshot === "function" ? Array.from(snapshot()) : null;
        },
        refuseResume() {
            resumeAllowed = false;
        },
        dispatch(type) {
            for (const callback of listeners.get(type) || []) callback({ target: canvas });
        },
    };
}

const pointerHarness = createHarness();
const pointerContexts = [new pointerHarness.AudioContext(), new pointerHarness.AudioContext()];
assert.ok(pointerContexts.every((context) => context.state === "suspended"), "contexts start suspended before player input");
assert.equal(pointerHarness.contexts().length, 2, "the page hook tracks each actual AudioContext object");
assert.ok(pointerHarness.contexts().every((context, index) => context === pointerContexts[index]),
    "the observer retains the actual contexts Bevy created");
assert.deepEqual(pointerHarness.contextStates(), ["suspended", "suspended"],
    "the observer reports actual state for every context before the gesture");
pointerHarness.dispatch("pointerdown");
assert.ok(pointerContexts.every((context) => context.resumeCalls === 1), "the first pointer down resumes each created AudioContext");
assert.ok(pointerContexts.every((context) => context.state === "running"), "actual contexts are running after the gesture");
assert.deepEqual(pointerHarness.contextStates(), ["running", "running"],
    "the observer reports actual state after each context changes");
pointerHarness.dispatch("keydown");
assert.ok(pointerContexts.every((context) => context.resumeCalls === 1), "the resume hook runs once after the first gesture");

const keyHarness = createHarness();
const keyContext = new keyHarness.AudioContext();
assert.equal(keyHarness.contexts()[0], keyContext, "the key path observes the same actual AudioContext");
keyHarness.dispatch("keydown");
assert.equal(keyContext.resumeCalls, 1, "the first key down also resumes the AudioContext");

const deniedHarness = createHarness();
const deniedContext = new deniedHarness.AudioContext();
deniedHarness.refuseResume();
deniedHarness.dispatch("pointerdown");
assert.equal(deniedContext.state, "suspended", "a resume request does not prove the actual context changed state");
assert.deepEqual(deniedHarness.contextStates(), ["suspended"], "the observer keeps a denied context locked");

const lateContextHarness = createHarness();
lateContextHarness.dispatch("pointerdown");
const lateContext = new lateContextHarness.AudioContext();
lateContextHarness.dispatch("keydown");
assert.equal(lateContext.resumeCalls, 1,
    "a later real gesture retries contexts created after an earlier click with no contexts");
assert.deepEqual(lateContextHarness.contextStates(), ["running"],
    "the readiness observer reports the context's actual post-resume state");

console.log("AUD1 resume hook: suspended context resumes once on the first pointer or key gesture");
