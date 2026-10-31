"use strict";

const test = require("node:test");
const assert = require("node:assert/strict");
const { fixtureOffset, fixtureWorldX, validateTranslationReceipt } = require("./m2-fixture.js");

test("translated M2 fixtures require the matching explicit offset", () => {
    assert.equal(fixtureOffset("base", "18"), 18);
    assert.equal(fixtureOffset("haul", "18"), 18);
    assert.equal(fixtureOffset("cedar", "18"), 18);
    assert.equal(fixtureOffset("acme", "0"), 0);
    assert.equal(fixtureOffset(undefined, undefined), 0);
    assert.throws(() => fixtureOffset("cedar", undefined), /explicit X offset 18/);
    assert.throws(() => fixtureOffset("base", "17"), /explicit X offset 18/);
    assert.throws(() => fixtureOffset("acme", "18"), /explicit X offset 0/);
    assert.equal(validateTranslationReceipt("acme", "E9-browser-acme-selection", "0", {
        package: "acme", case_id: "E9-browser-acme-selection", offset_x: 0, map_translation: "none",
    }), 0);
    assert.throws(() => validateTranslationReceipt("cedar", "case", "18", {
        package: "cedar", case_id: "case", offset_x: 0,
    }), /receipt does not match/);
    assert.equal(validateTranslationReceipt("cedar", "case", "18", {
        package: "cedar", case_id: "case", offset_x: 18, map_relative_path: "maps/clearing.ron",
        packed_after_translation_sha256: "a".repeat(64),
    }), 18);
});

test("old absolute world lookups fail after translation and translated lookups succeed", () => {
    const offset = fixtureOffset("cedar", "18");
    const translated = fixtureWorldX(8, offset);
    assert.equal(translated, 26);
    assert.notEqual(translated, 8);
    assert.equal(fixtureWorldX(24, offset), 42);
});
