// turts-test-role: unit
"use strict";

const assert = require("node:assert/strict");
const { EventEmitter } = require("node:events");
const { trackNewPageDocumentEpoch } = require("./document-epoch.js");

function fixture() {
    const page = new EventEmitter();
    const mainFrame = {};
    page.mainFrame = () => mainFrame;
    const documentEpochs = [1, 2, 3];
    let next = 3;
    const allocate = () => ++next;
    return { page, mainFrame, documentEpochs, allocate };
}

{
    const { page, documentEpochs, allocate } = fixture();
    assert.equal(trackNewPageDocumentEpoch(page, 2, documentEpochs, allocate), 4,
        "observing a browser-authentic new Page allocates its document epoch before navigation events");
    assert.equal(documentEpochs[2], 4);
}
{
    const { page, mainFrame, documentEpochs, allocate } = fixture();
    trackNewPageDocumentEpoch(page, 2, documentEpochs, allocate);
    page.emit("framenavigated", {});
    assert.equal(documentEpochs[2], 4, "subframe navigation cannot advance the page document epoch");
    page.emit("framenavigated", mainFrame);
    assert.equal(documentEpochs[2], 5, "later main-frame navigation advances the same shared counter");
    page.emit("framenavigated", mainFrame);
    assert.equal(documentEpochs[2], 6, "each subsequent main-document navigation remains monotonic");
}
{
    const { page, documentEpochs } = fixture();
    assert.throws(() => trackNewPageDocumentEpoch(page, 2, documentEpochs, () => 3),
        /must advance monotonically/, "an allocator cannot reuse the closed C document's epoch");
}
