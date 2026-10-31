// turts-test-role: pure
"use strict";

function expectedOffset(packageName) {
    if (packageName === "base" || packageName === "haul" || packageName === "cedar") return 18;
    if (packageName === "acme") return 0;
    throw new Error(`unknown M2 fixture package ${packageName}`);
}

function fixtureOffset(packageName, rawOffset) {
    if (packageName === undefined) {
        if (rawOffset !== undefined) throw new Error("M2 fixture offset requires an explicit package");
        return 0;
    }
    const expected = expectedOffset(packageName);
    if (rawOffset !== String(expected)) {
        throw new Error(`M2 ${packageName} fixture requires explicit X offset ${expected}`);
    }
    return expected;
}

function fixtureWorldX(value, offset) {
    if (!Number.isFinite(value) || ![0, 18].includes(offset)) throw new Error("invalid M2 fixture X coordinate or offset");
    return value + offset;
}

function validateTranslationReceipt(packageName, caseId, rawOffset, receipt) {
    const offset = fixtureOffset(packageName, rawOffset);
    if (!receipt || receipt.package !== packageName || receipt.case_id !== caseId || receipt.offset_x !== offset) {
        throw new Error("M2 fixture translation receipt does not match the launched package, case, and offset");
    }
    if (offset === 0) {
        if (receipt.map_translation !== "none") throw new Error("untranslated M2 package has an unexpected map override");
    } else {
        if (typeof receipt.map_relative_path !== "string" || !/^[0-9a-f]{64}$/.test(receipt.packed_after_translation_sha256 ?? "")) {
            throw new Error("translated M2 package is missing its map path or packed-map hash proof");
        }
    }
    return offset;
}

module.exports = { expectedOffset, fixtureOffset, fixtureWorldX, validateTranslationReceipt };
