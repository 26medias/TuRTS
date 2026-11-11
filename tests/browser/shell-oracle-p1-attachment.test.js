// turts-test-role: unit
"use strict";

const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const { sourceControls } = require("./shell-oracle.js");

const judge = sourceControls.judgeP1Attachment;
assert(judge, "the fixed headed Chrome P1 attachment contract must be exposed to source controls");
const attachmentPath = path.resolve(__dirname, "../../docs/research/mc/p1-keys.md");
const bytes = fs.readFileSync(attachmentPath);
const metadata = {
    source_path: "docs/research/mc/p1-keys.md",
    sha256: "f41d55729fa45f291d36d6a780d9aeb021242d61703dba39eed38e78c16684c1",
    byte_length: bytes.length,
    attached_before_physical_run: true,
};

function view(fileBytes = bytes, fileMetadata = metadata) {
    return {
        async serverBytes(name) {
            assert.equal(name, "p1-keys.md");
            return fileBytes;
        },
        async serverFile(name) {
            assert.equal(name, "p1-attachment.json");
            return JSON.stringify(fileMetadata);
        },
    };
}

(async () => {
    assert.deepEqual(await judge(view(), "CAM2"), {
        sha256: metadata.sha256,
        byteLength: bytes.length,
    });
    assert.deepEqual(await judge(view(), "KM2"), {
        sha256: metadata.sha256,
        byteLength: bytes.length,
    });

    await assert.rejects(judge(view(Buffer.from(`${bytes.toString("utf8")}\n`)), "CAM2"), /exact accepted headed Chrome P1 attachment/,
        "changed bytes cannot satisfy the attachment contract");
    await assert.rejects(judge(view(bytes, { ...metadata, attached_before_physical_run: false }), "CAM2"), /not attached before the physical run/,
        "late attachment metadata cannot stand in for pre-run evidence");
    await assert.rejects(judge(view(bytes, { ...metadata, source_path: "other.md" }), "KM2"), /wrong recorded source path/,
        "a byte-identical file from an unapproved source path is not accepted");
    await assert.rejects(judge(view(bytes, { ...metadata, byte_length: bytes.length + 1 }), "CAM2"), /length differs from attached bytes/,
        "the metadata length must match the attached bytes");
})();
