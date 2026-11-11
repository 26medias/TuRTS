// turts-test-role: harness
"use strict";

function trackNewPageDocumentEpoch(page, windowIndex, documentEpochs, allocateEpoch) {
    if (!page || !Number.isSafeInteger(windowIndex) || windowIndex < 0 || !Array.isArray(documentEpochs) ||
        typeof allocateEpoch !== "function") {
        throw new TypeError("new page epoch tracking requires a page, window index, epoch list, and allocator");
    }
    const previous = documentEpochs[windowIndex] ?? 0;
    const initial = allocateEpoch();
    if (!Number.isSafeInteger(initial) || initial <= previous) {
        throw new Error("new page document epoch must advance monotonically");
    }
    documentEpochs[windowIndex] = initial;
    page.on("framenavigated", (frame) => {
        if (frame === page.mainFrame()) {
            const next = allocateEpoch();
            if (!Number.isSafeInteger(next) || next <= documentEpochs[windowIndex]) {
                throw new Error("main document epoch must advance monotonically");
            }
            documentEpochs[windowIndex] = next;
        }
    });
    return initial;
}

module.exports = { trackNewPageDocumentEpoch };
