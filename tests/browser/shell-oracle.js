// turts-test-role: oracle
"use strict";

const assert = require("node:assert/strict");
const crypto = require("node:crypto");
const path = require("node:path");
const { judgeSh1Lifecycle, judgeSh1ConflictOffer } = require("./sh1-oracle.js");
const { judgeSh1Settings } = require("./sh1-settings-oracle.js");
const { judgeSh1Bindings } = require("./sh1-binding-oracle.js");
const { evaluatePer1BrowserEvidence, validateOptionalNativePathDocument } = require("./per1-oracle-candidate.js");
const { decodePng } = require("./harness/png.js");
const { judgeSel4Pixels } = require("./harness/sel4-pixels.js");
const { readAuthoredCursorAssets, measureCursorPixels } = require("./harness/cursor-pixels.js");

const ROW_CHECKS = Object.freeze({
    KM1: ["keyq-bound-action", "mod-default", "player-override-reload", "reserved-player-notice", "reserved-mod-exit", "labels-guide", "labels-guide-pixels"],
    KM2: ["join-focus", "contextmenu-prevented-delta", "canvas-blur-overlay", "outside-key-stopped", "p1-attachment", "canvas-blur-pixels"],
    SEL4: ["ctrl-1-assign", "1-recall", "shift-1-add", "double-tap-centre", "same-frame-destroy-prune", "reload-live-ids", "old-match-prune", "double-tap-pixels"],
    SLOT1: ["reload-same-id", "same-slot-rejoin", "unclaimed-third-refusal", "copied-tab-terminal-refusal", "owners-stable", "reload-to-authority-removal", "clone-terminal-pixels"],
    CAM2: ["pointer-lock", "summed-motion-cursor", "edge-band-pan", "f10-menu", "p1-attachment", "cursor-pixels"],
    SH1: ["load-phases", "wrong-lockfile-guidance", "menu-controls", "conflict-swap-cancel-reset", "guide-effective-label", "scroll-distance-ratio", "ui-scale-rects", "cursor-lock", "connection-lost-reload", "zero-menu-world-orders", "menu-pixels"],
    PER1: ["browser-key-names", "native-path-names", "compatibility-line", "mod-isolation", "defaults-and-single-notice", "old-match-groups-pruned", "volume-fields-preserved", "notice-pixels"],
});
const ROW_PIXEL_CHECKS = Object.freeze({
    KM1: "labels-guide-pixels", KM2: "canvas-blur-pixels", SEL4: "double-tap-pixels",
    SLOT1: "clone-terminal-pixels", CAM2: "cursor-pixels", SH1: "menu-pixels", PER1: "notice-pixels",
});
const ROW_PENDING_CLAUSES = Object.freeze({
    KM1: {
        "labels-guide-pixels": "needs the accepted target's real Key M and Key Q guide captures with absent/nearby pixel controls",
    },
    KM2: {
        "canvas-blur-pixels": "needs an actual target overlay PNG and same-rectangle negative controls",
    },
    SEL4: {
        "old-match-prune": "no actual prior-match group storage fixture is retained for the join-pruning outcome",
        "double-tap-pixels": "needs calibrated target screenshots before and after actual group centering",
    },
    SLOT1: {
        "clone-terminal-pixels": "the source pixel shape still lacks matching target PNG and absent/nearby negative controls",
    },
    CAM2: {
        "summed-motion-cursor": "the harness observes browser pointer lock, but the engine has no source-backed locked pointer-delta cursor result",
        "edge-band-pan": "there is no accepted locked-cursor edge-band product outcome to observe yet",
        "f10-menu": "the actual F10 menu widgets exist but no accepted settings screenshot and control selectors have been calibrated",
        "p1-attachment": "the accepted headed Chrome P1 attachment is not present in this retained run",
        "cursor-pixels": "needs target cursor screenshots and matching geometric negative controls",
    },
    SH1: {
        "load-phases": "the required ordered acquisition phases are not yet retained by the row fixture",
        "wrong-lockfile-guidance": "no real wrong-lockfile acquisition fixture and visible guidance outcome is retained",
        "conflict-swap-cancel-reset": "needs the complete actual Swap, Cancel and Reset outcome proof",
        "guide-effective-label": "actual guide section 5 has not been exercised with a persisted player override",
        "scroll-distance-ratio": "no accepted scroll-speed setting changes camera distance yet",
        "ui-scale-rects": "no accepted UI-scale control or changed HUD layout outcome exists yet",
        "cursor-lock": "the shell has no actual cursor-lock setting",
        "connection-lost-reload": "no retained product frame proves connection-lost guidance and a successful reload path",
        "zero-menu-world-orders": "no menu interaction capture proves camera/UI input leaves world orders unchanged",
        "menu-pixels": "needs target menu screenshots and contrasting same-rectangle negative controls",
    },
    PER1: {
        "native-path-names": "needs a matching actual native process reference capsule",
        "defaults-and-single-notice": "no actual malformed browser-data fixture proves defaults plus one visible deduplicated notice",
        "notice-pixels": "needs the actual malformed-data notice PNG and matched negative controls",
    },
});
const ROW_PHYSICAL_CHECKS = Object.freeze({
    KM1: ["keyq-bound-action"], KM2: ["contextmenu-prevented-delta", "outside-key-stopped"],
    SEL4: ["ctrl-1-assign", "1-recall", "shift-1-add", "double-tap-centre"],
    SLOT1: ["unclaimed-third-refusal", "copied-tab-terminal-refusal", "reload-same-id"],
    CAM2: ["pointer-lock", "summed-motion-cursor", "edge-band-pan", "f10-menu"],
    SH1: ["menu-controls", "conflict-swap-cancel-reset", "cursor-lock", "connection-lost-reload"],
    PER1: ["browser-key-names", "defaults-and-single-notice"],
});
const ROW_INPUT_CALLS = Object.freeze({
    KM1: { "keyq-bound-action": "key" },
    KM2: { "contextmenu-prevented-delta": "click", "outside-key-stopped": "key" },
    SEL4: { "ctrl-1-assign": "key", "1-recall": "key", "shift-1-add": "key", "double-tap-centre": "key" },
    SLOT1: { "copied-tab-terminal-refusal": "use", "reload-same-id": "reload" },
    CAM2: { "pointer-lock": "click", "summed-motion-cursor": "move", "edge-band-pan": "move", "f10-menu": "key" },
    SH1: { "menu-controls": "key", "conflict-swap-cancel-reset": "click", "cursor-lock": "click", "connection-lost-reload": "reload" },
    PER1: { "browser-key-names": "click", "defaults-and-single-notice": "click" },
});

function semanticFailure(row, clause, message) {
    const error = new Error(message);
    error.task16_semantic = { row, clause };
    return error;
}

function requireProduct(condition, row, clause, message) {
    if (!condition) throw semanticFailure(row, clause, message);
}

function judgeStartupOutcome({ row, clause, role, startup }) {
    assert.equal(row, "SLOT1", "startup precondition evaluator is fixed to SLOT1");
    assert.equal(clause, "owners-stable", "startup precondition evaluator accepts only the fixed owner clause");
    assert(["A", "B"].includes(role), "startup owner role must be A or B");
    assert(startup && typeof startup.phase === "string" && startup.debug && typeof startup.debug === "object" &&
        startup.scene && typeof startup.scene === "object", "owner startup receipt lacks actual typed debug or scene bridge data");
    if (startup.phase.startsWith("Joined") && startup.last_frame && startup.cameras_3d === 1) {
        return { status: "passed", clause };
    }
    assert(startup.phase.startsWith("Failed") || startup.phase.startsWith("Joined"),
        `SLOT1 ${role} is not a terminal product result; missing owner frames remain a setup failure`);
    return {
        status: "semantic-failure",
        clause,
        message: `ordinary ${role} startup reached actual ${startup.phase} without a Joined owner frame and 3D scene`,
    };
}

async function judge(view, params) {
    const row = params.row;
    const plan = params.task16;
    if (!Object.hasOwn(ROW_CHECKS, row)) throw new Error(`unsupported Task16 row ${row}`);
    assert.equal(params.instrument_status, undefined, `${row} cannot claim baseline while the preparation guard is present`);
    assert.equal(plan?.status, "executable", `${row} physical plan is not executable`);
    assert(Array.isArray(plan.checks) && plan.checks.length > 0, `${row} needs nonempty acceptance checks`);
    assert(plan.checks.every((check) => typeof check === "string"), `${row} clause plan may contain IDs only`);
    const checkById = new Set(plan.checks);
    for (const id of ROW_CHECKS[row]) assert(checkById.has(id), `${row} lacks required fixed clause ${id}`);
    assert.equal(checkById.size, ROW_CHECKS[row].length, `${row} has duplicate or unrecognized clause IDs`);

    const input = view.inputLog();
    const epochsByWindow = new Map();
    for (const [index, entry] of input.entries()) {
        assert(Number.isSafeInteger(entry.document_epoch) && entry.document_epoch > 0,
            `${row} input ${index} lacks a positive dispatched document epoch`);
        const previous = epochsByWindow.get(entry.window) ?? 0;
        assert(entry.document_epoch >= previous, `${row} document epoch moved backward for window ${entry.window}`);
        epochsByWindow.set(entry.window, entry.document_epoch);
        if (entry.call === "reload") {
            assert.equal(entry.document_epoch_before, entry.document_epoch,
                `${row} reload must retain its actual pre-navigation document epoch`);
            assert(Number.isSafeInteger(entry.document_epoch_after) &&
                entry.document_epoch_after > entry.document_epoch_before,
            `${row} reload must retain an advanced actual post-navigation document epoch`);
        }
    }
    const scene = JSON.parse(await view.serverFile("scene-checkpoints.json"));
    const captures = JSON.parse(await view.serverFile(`${row}-capture-index.json`));
    assert.equal(params.capture_scene_checkpoints, true, `${row} requires actual same-checkpoint typed data`);
    assert(Array.isArray(scene), `${row} has no scene checkpoint list`);
    assert(Array.isArray(captures) && captures.length === plan.captures.length, `${row} missing retained PNG capture records`);
    const screenshotCalls = input.map((entry, index) => ({ entry, index })).filter(({ entry }) => entry.call === "screenshot");
    const captureByName = new Map(captures.map((capture) => [capture.name, capture]));
    const decoded = new Map();
    assert.equal(captureByName.size, captures.length, `${row} capture names are not unique`);

    for (const [ordinal, expected] of plan.captures.entries()) {
        const capture = captureByName.get(expected.name);
        assert(capture, `${row} missing capture ${expected.name}`);
        assert.equal(capture.ordinal, ordinal, `${row} capture order changed`);
        assert.match(capture.sha256, /^[a-f0-9]{64}$/, `${row} capture has no PNG hash`);
        assert(Number.isSafeInteger(capture.byte_length) && capture.byte_length > 8, `${row} capture has no PNG bytes`);
        const screenshot = screenshotCalls[ordinal];
        assert(screenshot, `${row} has no physical screenshot call for ${expected.name}`);
        assert.equal(capture.input_call, screenshot.index, `${row} ${expected.name} capture input index changed`);
        assert.equal(capture.document_epoch, screenshot.entry.document_epoch, `${row} ${expected.name} document epoch changed`);
        assert.equal(screenshot.entry.window, expected.window, `${row} ${expected.name} was captured from the wrong window`);
        assert.deepEqual(screenshot.entry.args, [], `${row} screenshot call shape changed`);
        const saved = input.find((entry) => entry.call === "save" && entry.args?.[0] === capture.file);
        assert(saved, `${row} ${expected.name} has no matching save event`);
        assert.equal(saved.window, screenshot.entry.window, `${row} ${expected.name} save window does not match screenshot`);
        assert.equal(saved.document_epoch, screenshot.entry.document_epoch, `${row} ${expected.name} save document epoch does not match screenshot`);
        assert.equal(saved.args?.[1]?.sha256, capture.sha256, `${row} ${expected.name} hash differs from the actual IO save record`);
        assert.equal(saved.args?.[1]?.byte_length, capture.byte_length, `${row} ${expected.name} length differs from the actual IO save record`);
        const pngBytes = await view.serverBytes(capture.file);
        assert.equal(pngBytes.length, capture.byte_length, `${row} ${expected.name} saved PNG length changed on disk`);
        assert.equal(crypto.createHash("sha256").update(pngBytes).digest("hex"), capture.sha256, `${row} ${expected.name} saved PNG hash changed on disk`);
        decoded.set(expected.name, decodePng(pngBytes));
        const checkpoint = scene.find((entry) => entry.input_call === screenshot.index && entry.window === expected.window);
        assert(checkpoint?.debug, `${row} ${expected.name} lacks actual typed debug at the exact screenshot call`);
        assert.equal(checkpoint.document_epoch, screenshot.entry.document_epoch, `${row} ${expected.name} checkpoint document epoch does not match screenshot`);
        capture.checkpoint = checkpoint;
        capture.screenshot_input_call = screenshot.index;
    }

    assertRequiredPhysicalReceipts(row, input, captures);
    let evaluated = new Set();
    if (row === "KM1") {
        evaluated = await judgeKm1(view, input, captures);
        judgeKm1GuidePixels(captures, decoded);
        evaluated.add("labels-guide-pixels");
    }
    if (row === "KM2") evaluated = await judgeKm2(view, input, captures, decoded);
    if (row === "SEL4") evaluated = await judgeSel4(view, input, captures, decoded);
    if (row === "CAM2") {
        evaluated = judgeCam2(input, captures);
        await judgeP1Attachment(view, "CAM2");
        evaluated.add("p1-attachment");
        await judgeCam2CursorPixels(view, captures, decoded);
        evaluated.add("cursor-pixels");
    }
    if (row === "SH1") {
        evaluated = judgeSh1MenuControls(input, captures);
        evaluated = await judgeSh1Lifecycle(view, input, captures, decoded, evaluated);
        judgeSh1ConflictOffer(captures);
        evaluated = await judgeSh1Settings(view, input, captures, decoded, evaluated);
        evaluated = judgeSh1Bindings(input, captures, decoded, evaluated);
    }
    if (row === "PER1") evaluated = await judgePer1Storage(view, input, captures, params, decoded);
    if (row === "SLOT1") {
        await judgeSlot1Evidence(view, input, captures, captureByName);
    } else {
        for (const id of ROW_CHECKS[row]) {
            if (evaluated.has(id)) continue;
            const reason = ROW_PENDING_CLAUSES[row]?.[id] ?? "the fixed semantic evaluator has not been qualified against an actual product capture";
            throw new Error(`${row} NOT READY: ${id}: ${reason}`);
        }
    }
}

function hasKeyBinding(bindings, code) {
    return Array.isArray(bindings) && bindings.some((binding) =>
        binding && binding.Key && binding.Key.code === code && Array.isArray(binding.Key.mods) && binding.Key.mods.length === 0);
}

function retainedFrames(debug) {
    const byIndex = new Map();
    for (const frame of [...(debug.frames?.recent || []), ...(debug.frames?.retained_input || [])]) {
        assert.equal(frame.named_dispatches_truncated, 0, "named Core dispatch evidence was truncated");
        if (Number.isSafeInteger(frame.frame_index)) byIndex.set(frame.frame_index, frame);
    }
    return [...byIndex.values()].sort((left, right) => left.frame_index - right.frame_index);
}

function keyCall(input, code, modifiers, after = -1) {
    const index = input.findIndex((entry, candidate) => candidate > after && entry.call === "key" &&
        entry.args?.[0] === code && JSON.stringify(entry.args?.[1] || []) === JSON.stringify(modifiers));
    assert(index >= 0, `SEL4 physical ${modifiers.join("+") || "plain"} ${code} call is absent`);
    const entry = input[index];
    const keydown = entry.actual_events.find((event) => event.type === "keydown" && event.code === code);
    assert(keydown && Number.isSafeInteger(keydown.seq) && keydown.seq > 0,
        `SEL4 ${code} input lacks its actual keydown sequence`);
    return { entry, index, keydown };
}

function captureBefore(captures, input, inputIndex, entry) {
    const previous = captures.filter((capture) => capture.input_call < inputIndex &&
        capture.window === entry.window && capture.document_epoch === entry.document_epoch).at(-1);
    assert(previous, "SEL4 requires a same-document retained product frame before the physical group action");
    return previous;
}

function captureAfter(captures, input, inputIndex, entry) {
    const next = captures.find((capture) => capture.input_call > inputIndex &&
        capture.window === entry.window && capture.document_epoch === entry.document_epoch);
    assert(next, "SEL4 requires a same-document retained product frame after the physical group action");
    return next;
}

function frameForKey(capture, keydown, action) {
    return retainedFrames(capture.checkpoint.debug).find((frame) => frame.applied_seq?.includes(keydown.seq) &&
        frame.named_dispatches?.some((dispatch) => dispatch.origin === "key" && dispatch.code === "Digit1" &&
            dispatch.matched_action === action));
}

// The product window of a double tap of a group key (crates/turts_interaction/src/lib.rs:1626, double_click_ms).
const SEL4_DOUBLE_TAP_WINDOW_MS = 400;

// The double tap is one key step with options.taps (tests/browser/harness/driver-io.js tapKey). The harness fails the
// call when the page-measured gap is over the window; this re-reads the page timestamps so that a slow tap names the
// instrument ("host or driver fault"), never the product, and runs before any product clause.
function sel4DoubleTapReceipt(input) {
    const index = input.findIndex((entry) => entry.call === "key" && entry.args?.[0] === "Digit1" &&
        JSON.stringify(entry.args?.[1] || []) === "[]" && entry.args?.[2]?.taps === 2);
    assert(index >= 0, "SEL4 double tap step is absent from the input log: instrument fault");
    const entry = input[index];
    const windowMs = entry.args[2].window_ms;
    assert.equal(windowMs, SEL4_DOUBLE_TAP_WINDOW_MS,
        `SEL4 double tap window must be the product window of ${SEL4_DOUBLE_TAP_WINDOW_MS} ms, the row says ${windowMs}: instrument fault`);
    const downs = (entry.actual_events || []).filter((event) => event.type === "keydown" && event.code === "Digit1");
    assert(downs.length === 2, `double tap: the browser saw ${downs.length} keydown events, expected 2: instrument fault`);
    const gap = downs[1].wall_time_ms - downs[0].wall_time_ms;
    assert(Number.isFinite(gap) && gap >= 0, "double tap: the keydown times are not numbers: instrument fault");
    if (!(gap <= windowMs)) throw new Error(`double tap took ${gap} ms, over the ${windowMs} ms window: host or driver fault`);
    return { index, entry, gap_ms: gap };
}

async function judgeSel4(view, input, captures, decoded) {
    const assign = keyCall(input, "Digit1", ["Control"]);
    const beforeAssign = captureBefore(captures, input, assign.index, assign.entry);
    const afterAssign = captureAfter(captures, input, assign.index, assign.entry);
    assert.equal(beforeAssign.checkpoint.debug.verified_root_mod?.id, "muster",
        "SEL4 must use the actual verified root from the isolated Muster fixture");
    const selectedBeforeAssign = beforeAssign.checkpoint.debug.selection;
    assert(Array.isArray(selectedBeforeAssign), "SEL4 pre-assignment actual selection is absent");
    const currentUnits = beforeAssign.checkpoint.debug.replicated_world?.units;
    assert(Array.isArray(currentUnits), "SEL4 lacks the actual pre-assignment replicated unit list");
    const workerAt = (x, y) => currentUnits.filter((unit) => unit.content_id === "muster:worker" &&
        unit.owner_slot === beforeAssign.checkpoint.debug.slot && Array.isArray(unit.pos) &&
        Math.abs(unit.pos[0] - x) <= 0.01 && Math.abs(unit.pos[1] - y) <= 0.01);
    const worker41Rows = workerAt(10, 12);
    const worker42Rows = workerAt(12, 12);
    assert.equal(worker41Rows.length, 1, "SEL4 fixture must retain its actual own worker at (10,12)");
    assert.equal(worker42Rows.length, 1, "SEL4 fixture must retain its actual own worker at (12,12)");
    const worker41 = worker41Rows[0].unit_id;
    const worker42 = worker42Rows[0].unit_id;
    assert(Number.isSafeInteger(worker41) && Number.isSafeInteger(worker42) && worker41 !== worker42,
        "SEL4 fixture worker IDs must come from distinct actual replicated units");
    assert.deepEqual([...selectedBeforeAssign].sort((a, b) => a - b), [worker41, worker42].sort((a, b) => a - b),
        "SEL4 setup invalid: the world gesture must first select the two actual owned fixture workers");
    assert(!beforeAssign.checkpoint.debug.ui_snapshot?.some((node) => node.kind === "focus_overlay" && node.visible === true),
        "SEL4 setup invalid: the focus overlay must be cleared before selection/group assignment");
    const assignFrame = frameForKey(afterAssign, assign.keydown, "group_assign_1");
    requireProduct(Boolean(assignFrame), "SEL4", "ctrl-1-assign",
        "physical Ctrl+1 did not reach the actual group_assign_1 Core dispatch branch");
    requireProduct(JSON.stringify(assignFrame.groups?.["1"]) === JSON.stringify([...selectedBeforeAssign].sort((a, b) => a - b)),
        "SEL4", "ctrl-1-assign", "group 1 did not retain the exact selected live unit IDs in the dispatch frame");


    const recall = keyCall(input, "Digit1", []);
    const beforeRecall = captureBefore(captures, input, recall.index, recall.entry);
    const afterRecall = captureAfter(captures, input, recall.index, recall.entry);
    const groupBeforeRecall = beforeRecall.checkpoint.debug.groups?.["1"];
    assert(Array.isArray(groupBeforeRecall), "SEL4 actual group 1 is unavailable before recall");
    const recallFrame = frameForKey(afterRecall, recall.keydown, "group_recall_1");
    requireProduct(Boolean(recallFrame), "SEL4", "1-recall", "physical 1 did not reach the actual group_recall_1 Core dispatch branch");
    requireProduct(JSON.stringify(recallFrame.selection) === JSON.stringify(groupBeforeRecall), "SEL4", "1-recall",
        "the same-frame actual selection after 1 does not equal the live saved group IDs");

    const add = keyCall(input, "Digit1", ["Shift"]);
    const beforeAdd = captureBefore(captures, input, add.index, add.entry);
    const afterAdd = captureAfter(captures, input, add.index, add.entry);
    const groupBeforeAdd = beforeAdd.checkpoint.debug.groups?.["1"];
    const selectionBeforeAdd = beforeAdd.checkpoint.debug.selection;
    assert(Array.isArray(groupBeforeAdd) && Array.isArray(selectionBeforeAdd), "SEL4 actual group and selection before Shift+1 are unavailable");
    const addFrame = frameForKey(afterAdd, add.keydown, "group_add_1");
    requireProduct(Boolean(addFrame), "SEL4", "shift-1-add", "physical Shift+1 did not reach the actual group_add_1 Core dispatch branch");
    const expectedAdded = [...new Set([...groupBeforeAdd, ...selectionBeforeAdd])].sort((a, b) => a - b);
    requireProduct(JSON.stringify(addFrame.groups?.["1"]) === JSON.stringify(expectedAdded), "SEL4", "shift-1-add",
        "the same-frame actual group 1 does not equal the old group plus the selected live unit IDs");

    const reloadIndex = input.findIndex((entry) => entry.call === "reload");
    assert(reloadIndex >= 0, "SEL4 needs a real browser reload to check persisted group identity");
    const doubleTap = sel4DoubleTapReceipt(input);
    judgeSel4DeathPrune(captures, input, reloadIndex, assignFrame, afterAssign, worker41, worker42);

    const beforeDouble = captureBefore(captures, input, doubleTap.index, doubleTap.entry);
    const afterDouble = captureAfter(captures, input, doubleTap.index, doubleTap.entry);
    const group = afterDouble.checkpoint.debug.groups?.["1"];
    assert(Array.isArray(group) && group.length > 0, "SEL4 double-tap target group has no actual live members");
    const viewport = afterDouble.checkpoint.scene?.viewport_physical_rect;
    const units = afterDouble.checkpoint.scene?.units;
    assert(Array.isArray(viewport) && viewport.length === 4 && viewport.every(Number.isFinite) && Array.isArray(units),
        "SEL4 double-tap actual scene lacks physical viewport or projected unit centers");
    const centerDistance = (scene) => {
        const byId = new Map(scene.units.filter((unit) => Array.isArray(unit.ground_screen_center) && unit.ground_screen_center.length === 2)
            .map((unit) => [unit.unit_id, unit.ground_screen_center]));
        const points = group.map((id) => byId.get(id)).filter(Boolean);
        if (points.length !== group.length) return Infinity;
        const centroid = points.reduce((sum, point) => [sum[0] + point[0] / points.length, sum[1] + point[1] / points.length], [0, 0]);
        const center = [viewport[0] + viewport[2] / 2, viewport[1] + viewport[3] / 2];
        return Math.hypot(centroid[0] - center[0], centroid[1] - center[1]);
    };
    const beforeDistance = centerDistance(beforeDouble.checkpoint.scene);
    const afterDistance = centerDistance(afterDouble.checkpoint.scene);
    requireProduct(Number.isFinite(beforeDistance) && Number.isFinite(afterDistance) && afterDistance < beforeDistance,
        "SEL4", "double-tap-centre", "actual double-tap group recall did not move the group's projected centroid closer to the viewport center");

    const beforeReload = captures.filter((capture) => capture.input_call < reloadIndex && capture.window === 1).at(-1);
    const afterReload = captures.find((capture) => capture.input_call > reloadIndex && capture.window === 1);
    assert(beforeReload && afterReload, "SEL4 reload needs actual before and after captures");
    const beforeDebug = beforeReload.checkpoint.debug;
    const afterDebug = afterReload.checkpoint.debug;
    assert(typeof afterDebug.phase === "string" && afterDebug.phase.startsWith("Joined") &&
        afterDebug.last_frame && typeof afterDebug.match_id_decimal === "string" &&
        Number.isSafeInteger(afterDebug.slot) && afterReload.document_epoch === afterReload.checkpoint.document_epoch,
    "SEL4 reload checkpoint is not an actual joined product frame with a current match, slot, and document identity");
    requireProduct(beforeDebug.match_id_decimal === afterDebug.match_id_decimal && beforeDebug.slot === afterDebug.slot,
        "SEL4", "reload-live-ids", "group reload changed the actual match or player slot");
    const beforeGroups = beforeDebug.groups?.["1"];
    const afterGroups = afterDebug.groups?.["1"];
    requireProduct(Array.isArray(beforeGroups) && beforeGroups.length > 0 && JSON.stringify(afterGroups) === JSON.stringify(beforeGroups),
        "SEL4", "reload-live-ids", "group 1 did not restore the exact same live unit IDs after reload");
    const recalledAfterReload = keyCall(input, "Digit1", [], reloadIndex);
    const afterReloadRecall = captureAfter(captures, input, recalledAfterReload.index, recalledAfterReload.entry);
    const reloadRecallFrame = frameForKey(afterReloadRecall, recalledAfterReload.keydown, "group_recall_1");
    requireProduct(Boolean(reloadRecallFrame) && JSON.stringify(reloadRecallFrame.selection) === JSON.stringify(afterGroups),
        "SEL4", "reload-live-ids", "physical 1 after reload did not recall the exact restored live group IDs in the actual dispatch frame");

    if (decoded) {
        // Reject malformed calibration inputs as setup errors before product pixel predicates.
        for (const capture of [beforeDouble, afterDouble]) {
            const image = decoded.get(capture.name);
            const scene = capture.checkpoint.scene;
            assert(image && image.width === 1280 && image.height === 720, "SEL4 centering PNG dimensions changed");
            assert.deepEqual(scene.viewport_physical_rect, [0, 0, 1280, 720]);
            assert.equal(scene.scale_factor, 1, "SEL4 centering calibration requires actual DPR1");
            assert.equal(scene.coordinate_space, "window-logical-pixels-top-left");
            for (const id of [worker41, worker42]) {
                const unit = scene.units.find((unit) => unit.unit_id === id);
                assert(unit && Array.isArray(unit.translation) && unit.translation.length === 3 && unit.translation.every(Number.isFinite) &&
                    Array.isArray(unit.ground_screen_center) && unit.ground_screen_center.length === 2 && unit.ground_screen_center.every(Number.isFinite),
                    "SEL4 centering calibration requires actual finite worker positions/projections");
            }
        }
        try {
            judgeSel4Pixels(beforeDouble, afterDouble, decoded.get(beforeDouble.name), decoded.get(afterDouble.name), [worker41, worker42]);
        } catch (error) {
            throw semanticFailure("SEL4", "double-tap-pixels", error.message);
        }
    }
    await judgeSel4OldMatchPrune(view, beforeAssign.checkpoint.debug);
    return new Set(["ctrl-1-assign", "1-recall", "shift-1-add", "double-tap-centre", "same-frame-destroy-prune", "reload-live-ids", "old-match-prune", ...(decoded ? ["double-tap-pixels"] : [])]);
}

async function judgeSel4OldMatchPrune(view, debug) {
    const fixture = JSON.parse(await view.serverFile("task16-preboot-storage-fixture-private.json"));
    const audit = JSON.parse(await view.serverFile("storage-audit.json"));
    assert.equal(fixture.fixture, "sel4-authored-muster-death-v1", "SEL4 stale-group proof requires its bounded private preboot fixture");
    assert.equal(fixture.status, "complete", "SEL4 preboot group seed receipt is incomplete");
    const stale = fixture.seed_receipts.filter((seed) => seed.role === "old-match-group");
    assert(stale.length >= 2, "SEL4 requires two distinct actual stale group-key seeds");
    const staleIds = stale.map((seed) => /^turts\.v1\.groups\.(0|[1-9]\d*)\.(0|[1-9]\d*)$/.exec(seed.key)?.[1]);
    assert(staleIds.every((id) => id !== undefined) && new Set(staleIds).size === stale.length,
        "SEL4 fixture setup invalid: stale group keys are malformed or share a match id");
    const match = debug.match_id_decimal;
    assert(typeof match === "string" && /^(0|[1-9]\d*)$/.test(match) && staleIds.every((id) => id !== match),
        "SEL4 fixture setup invalid: actual match id collides with a seeded stale match id");
    for (const seed of stale) {
        assert(seed.set_returned === true && seed.readback_matches === true && seed.readback === seed.value &&
            seed.target_origin === fixture.windows[0].target_origin && seed.document_epoch > 0,
        `SEL4 stale key ${seed.key} lacks an actual target-origin set/readback receipt`);
    }
    const final = audit.final_state?.find((state) => state.window === 1);
    assert(final && final.document_epoch > 0, "SEL4 final storage state is not bound to the active browser document");
    const productEvents = audit.entries.filter((entry) => entry.source === "storage_api" &&
        entry.provenance !== "fixture_seed" && entry.storage === "localStorage" && entry.window === 1);
    for (const seed of stale) {
        requireProduct(productEvents.some((entry) => entry.operation === "remove" && entry.key === seed.key) &&
            !Object.hasOwn(final.localStorage || {}, seed.key), "SEL4", "old-match-prune",
        `SEL4 old-match-prune lacks a successful product remove receipt or final absence for ${seed.key}`);
    }
    const currentKey = `turts.v1.groups.${match}.${debug.slot}`;
    requireProduct(!stale.some((seed) => seed.key === currentKey) && Object.hasOwn(final.localStorage || {}, currentKey) &&
        productEvents.some((entry) => entry.operation === "set" && entry.key === currentKey &&
            entry.value === final.localStorage[currentKey]), "SEL4", "old-match-prune",
    "SEL4 current live group key lacks a distinct actual product set receipt or final presence");
}

function judgeSel4DeathPrune(captures, input, reloadIndex, assignFrame, afterAssign, worker41, worker42) {
    const beforeDebug = afterAssign.checkpoint.debug;
    const beforeWorld = beforeDebug.replicated_world;
    assert(beforeWorld && Array.isArray(beforeWorld.units), "SEL4 pre-death actual replicated world is missing");
    assert(beforeWorld.units.some((unit) => unit.unit_id === worker41) &&
        beforeWorld.units.some((unit) => unit.unit_id === worker42),
    "SEL4 actual own workers were not both present before the physical death phase");
    const frames = new Map();
    for (const capture of captures.filter((item) => item.input_call > afterAssign.input_call && item.input_call < reloadIndex &&
        item.window === afterAssign.window && item.document_epoch === afterAssign.document_epoch)) {
        const debug = capture.checkpoint.debug;
        assert.equal(debug.match_id_decimal, beforeDebug.match_id_decimal, "SEL4 physical death phase changed the actual match");
        assert.equal(debug.slot, beforeDebug.slot, "SEL4 physical death phase changed the actual player slot");
        const world = debug.replicated_world;
        assert(world && Array.isArray(world.units), "SEL4 death-phase actual replicated unit list is missing");
        for (const frame of retainedFrames(debug)) frames.set(frame.frame_index, { frame, capture, world });
    }
    const ordered = [...frames.values()].sort((left, right) => left.frame.frame_index - right.frame.frame_index);
    const present = (frame, unit) => frame.positions.some((position) => Array.isArray(position) && position[0] === unit);
    const selectionHas = (frame, unit) => Array.isArray(frame.selection) && frame.selection.includes(unit);
    const group = (frame) => frame.groups?.["1"];
    const transition = ordered.some((current, index) => {
        const previous = ordered[index - 1];
        if (!previous || current.frame.frame_index !== previous.frame.frame_index + 1 ||
            !Number.isSafeInteger(previous.frame.replica_tick) || !Number.isSafeInteger(current.frame.replica_tick) ||
            current.frame.replica_tick <= previous.frame.replica_tick) return false;
        return present(previous.frame, worker41) && !present(current.frame, worker41) &&
            selectionHas(previous.frame, worker41) && !selectionHas(current.frame, worker41) &&
            Array.isArray(group(previous.frame)) && group(previous.frame).includes(worker41) &&
            Array.isArray(group(current.frame)) && !group(current.frame).includes(worker41) &&
            group(current.frame).includes(worker42) &&
            current.world.units.some((unit) => unit.unit_id === worker42) &&
            !current.world.units.some((unit) => unit.unit_id === worker41);
    });
    requireProduct(transition, "SEL4", "same-frame-destroy-prune",
        "actual worker 41 death did not coincide in one retained Core frame with removal from selection and group 1 while worker 42 remained live");
}

function frameNamedDispatches(debug) {
    const frames = [...(debug.frames?.recent || []), ...(debug.frames?.retained_input || [])];
    return frames.flatMap((frame) => {
        assert.equal(frame.named_dispatches_truncated, 0, "named Core dispatch evidence was truncated");
        return Array.isArray(frame.named_dispatches) ? frame.named_dispatches : [];
    });
}

function verifiedRoot(debug, row) {
    const root = debug.verified_root_mod;
    assert(root && typeof root.id === "string" && typeof root.version === "string",
        `${row} actual verified lockfile root identity/version is missing`);
    assert(/^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:[-+][0-9A-Za-z.-]+)?$/.test(root.version),
        `${row} verified lockfile root version is not semantic version text`);
    return root;
}

async function judgeKm1(view, input, captures) {
    const debugs = captures.map((capture) => capture.checkpoint.debug);
    assert(debugs.length > 0, "KM1 needs actual joined debug captures");
    for (const debug of debugs) {
        const root = verifiedRoot(debug, "KM1");
        assert.equal(root.id, "forge", "KM1 must acquire its declared Forge root package");
        const defaults = debug.effective_bindings?.defaults;
        assert(defaults?.actions && defaults?.orders, "KM1 lacks resolver-derived package defaults");
        requireProduct(hasKeyBinding(defaults.actions.key_move, "KeyM"), "KM1", "mod-default",
            "verified Forge package key_move default does not contain KeyM");
        requireProduct(JSON.stringify(defaults.orders["forge:order_move"]) === JSON.stringify(defaults.actions.key_move),
            "KM1", "mod-default", "Forge order_move package default does not resolve through its actual hotkey action");
    }

    const keyReceipts = input.map((entry, index) => ({ entry, index })).filter(({ entry }) =>
        entry.call === "key" && entry.args?.[0] === "KeyQ" &&
        entry.actual_events?.some((event) => event.type === "keydown" && event.code === "KeyQ" && event.key === "a"));
    assert(keyReceipts.length >= 2,
        "KM1 needs one actual KeyQ/logical-a binding-capture event and a second physical KeyQ after the menu closes");
    const captureKey = keyReceipts[0];
    const captureKeyDown = captureKey.entry.actual_events.find((event) => event.type === "keydown" &&
        event.code === "KeyQ" && event.key === "a");
    assert(Number.isSafeInteger(captureKeyDown.seq) && captureKeyDown.seq > 0,
        "KM1 binding-capture KeyQ lacks its actual browser sequence");
    const captureResult = captures.find((capture) => capture.input_call > captureKey.index &&
        capture.window === captureKey.entry.window && capture.document_epoch === captureKey.entry.document_epoch);
    assert(captureResult, "KM1 binding-capture KeyQ lacks a same-document post-input checkpoint");
    assert(captureResult.checkpoint.debug.ui_snapshot?.some((node) => node.kind === "settings_menu" && node.visible === true),
        "KM1 binding-capture KeyQ checkpoint does not show the actual settings menu still open");
    const captureFrames = retainedFrames(captureResult.checkpoint.debug);
    requireProduct(!captureFrames.some((frame) => frame.applied_seq?.includes(captureKeyDown.seq) &&
        frame.named_dispatches?.some((dispatch) => dispatch.origin === "key" && dispatch.code === "KeyQ")),
    "KM1", "keyq-bound-action", "the physical KeyQ used to capture a binding also dispatched a gameplay action while the settings menu was open");
    requireKm1ChangeClickHitsMoveRow(input, captures, "edit-row", captureKey.index);
    const menuCloseIndex = input.findIndex((entry, index) => index > captureKey.index && entry.call === "key" &&
        entry.args?.[0] === "F10" && JSON.stringify(entry.args?.[1] || []) === "[]");
    assert(menuCloseIndex >= 0, "KM1 player KeyQ action must occur after physical F10 closes the binding menu");
    const gameplayKey = keyReceipts.find(({ index }) => index > menuCloseIndex);
    requireProduct(Boolean(gameplayKey), "KM1", "keyq-bound-action",
        "there is no physical KeyQ/logical-a action after the actual binding menu closed");
    const keyReceipt = gameplayKey.entry;
    const keyCallIndex = gameplayKey.index;
    const keyCapture = captures.find((capture) => capture.input_call > keyCallIndex &&
        capture.window === keyReceipt.window && capture.document_epoch === keyReceipt.document_epoch);
    assert(keyCapture, "KM1 needs a same-document actual debug capture after the KeyQ/logical-a receipt");
    const keyDown = keyReceipt.actual_events.find((event) => event.type === "keydown" && event.code === "KeyQ" && event.key === "a");
    assert(Number.isSafeInteger(keyDown.seq) && keyDown.seq > 0 && Number.isFinite(keyDown.wall_time_ms),
        "KM1 physical KeyQ receipt lacks its actual sequence and observed host time");
    const keyCaptureEnd = completedCaptureHostMs(keyCapture, input);
    const keyFrame = [...(keyCapture.checkpoint.debug.frames?.recent || []),
        ...(keyCapture.checkpoint.debug.frames?.retained_input || [])].find((frame) => {
        assert.equal(frame.named_dispatches_truncated, 0, "named Core dispatch evidence was truncated");
        if (!Array.isArray(frame.applied_seq) || !frame.applied_seq.includes(keyDown.seq) ||
            !Number.isFinite(frame.wall_ms) || frame.wall_ms < keyDown.wall_time_ms || frame.wall_ms > keyCaptureEnd) return false;
        return frame.named_dispatches?.some((entry) => entry.origin === "key" && entry.code === "KeyQ" &&
            entry.matched_action === "key_move" && entry.matched_order === "forge:order_move" &&
            Number.isSafeInteger(entry.input_index) && entry.input_index >= 0);
    });
    requireProduct(keyFrame, "KM1", "keyq-bound-action",
        "KeyQ/logical-a did not produce the actual key_move dispatch and armed-order result in its same post-receipt frame");
    requireProduct(keyFrame.pending_order_id === "forge:order_move", "KM1", "keyq-bound-action",
        "the actual KeyQ dispatch did not leave forge:order_move armed in that frame");
    requireProduct(hasKeyBinding(keyCapture.checkpoint.debug.effective_bindings?.actions?.key_move, "KeyQ"),
        "KM1", "keyq-bound-action", "the captured effective runtime key_move binding does not include KeyQ");
    const defaultGuideWheel = input.map((entry, index) => ({ entry, index })).find(({ entry }) => entry.call === "wheel" &&
        entry.window === keyReceipt.window && entry.document_epoch === keyReceipt.document_epoch);
    assert(defaultGuideWheel, "KM1 needs a physical section-five guide wheel before changing the default binding");
    const defaultWheelEvent = defaultGuideWheel.entry.actual_events?.find((event) => event.type === "wheel" &&
        Number.isFinite(event.client_x) && Number.isFinite(event.client_y) && Number.isFinite(event.delta_y) && event.delta_y !== 0);
    assert(defaultWheelEvent, "KM1 default guide scroll lacks its actual nonzero browser wheel receipt");
    const defaultGuideCapture = captures.find((capture) => capture.name === "guide-default" &&
        capture.input_call > defaultGuideWheel.index && capture.window === defaultGuideWheel.entry.window &&
        capture.document_epoch === defaultGuideWheel.entry.document_epoch);
    assert(defaultGuideCapture, "KM1 package-default guide label needs a same-document capture before rebinding");
    const defaultGuideRoot = defaultGuideCapture.checkpoint.debug.ui_snapshot?.find((node) =>
        node.id === "guide.4.bindings" && node.visible === true);
    assert(defaultGuideRoot && Array.isArray(defaultGuideRoot.rect) && defaultGuideRoot.rect.length === 4 &&
        defaultGuideRoot.rect.every(Number.isFinite), "KM1 default guide capture lacks its actual section-five bindings-list rectangle");
    assert(defaultWheelEvent.client_x >= defaultGuideRoot.rect[0] && defaultWheelEvent.client_x < defaultGuideRoot.rect[0] + defaultGuideRoot.rect[2] &&
        defaultWheelEvent.client_y >= defaultGuideRoot.rect[1] && defaultWheelEvent.client_y < defaultGuideRoot.rect[1] + defaultGuideRoot.rect[3],
    "KM1 pre-override physical wheel missed the actual section-five bindings list rectangle");
    requireProduct(hasBindingLabel(defaultGuideCapture.checkpoint.debug, "M"),
        "KM1", "labels-guide", "actual pre-override guide section five omits the package default Move selected units label after a physical list scroll");

    const guideWheel = input.map((entry, index) => ({ entry, index })).find(({ entry, index }) =>
        index > keyCallIndex && entry.call === "wheel" && entry.window === keyReceipt.window &&
        entry.document_epoch === keyReceipt.document_epoch);
    assert(guideWheel, "KM1 needs a physical guide-list wheel after the actual KeyQ action");
    const guideWheelEvent = guideWheel.entry.actual_events?.find((event) => event.type === "wheel" &&
        Number.isFinite(event.client_x) && Number.isFinite(event.client_y) && Number.isFinite(event.delta_y) && event.delta_y !== 0);
    assert(guideWheelEvent, "KM1 guide scroll lacks its actual nonzero browser wheel receipt");
    const preGuide = keyCapture.checkpoint.debug.ui_snapshot?.find((node) => node.id === "guide.4.bindings" && node.visible === true);
    assert(preGuide && Array.isArray(preGuide.rect) && preGuide.rect.length === 4 && preGuide.rect.every(Number.isFinite),
        "KM1 guide scroll has no actual visible section-five bindings-list rectangle");
    assert(guideWheelEvent.client_x >= preGuide.rect[0] && guideWheelEvent.client_x < preGuide.rect[0] + preGuide.rect[2] &&
        guideWheelEvent.client_y >= preGuide.rect[1] && guideWheelEvent.client_y < preGuide.rect[1] + preGuide.rect[3],
    "KM1 physical guide wheel missed the actual section-five bindings list rectangle");
    const guideCapture = captures.find((capture) => capture.name === "guide-effective" &&
        capture.input_call > guideWheel.index && capture.window === guideWheel.entry.window &&
        capture.document_epoch === guideWheel.entry.document_epoch);
    assert(guideCapture, "KM1 actual guide list lacks the same-document post-wheel screenshot/checkpoint");
    requireProduct(hasBindingLabel(guideCapture.checkpoint.debug, "Q"),
        "KM1", "labels-guide", "actual guide section five omits the effective player Move selected units label after a physical scroll over its list");
    await judgeKm1ReservedPack(view);

    const reload = input.findIndex((entry) => entry.call === "reload");
    assert(reload >= 0, "KM1 has no actual browser reload for player binding persistence");
    const postReload = captures.filter((capture) => capture.input_call > reload).map((capture) => capture.checkpoint.debug);
    assert(postReload.length > 0, "KM1 has no actual joined capture after reload");
    for (const debug of postReload) {
        requireProduct(hasKeyBinding(debug.effective_bindings?.runtime_overrides?.key_move, "KeyQ"),
            "KM1", "player-override-reload", "actual player key_move override did not survive reload");
        const defaults = debug.effective_bindings.defaults.actions;
        const effective = debug.effective_bindings.actions;
        for (const [action, bindings] of Object.entries(defaults)) {
            if (action !== "key_move") requireProduct(JSON.stringify(effective[action]) === JSON.stringify(bindings),
                "KM1", "mod-default", `player key_move override changed unrelated action ${action}`);
        }
    }
    const reservedKey = input.map((entry, index) => ({ entry, index })).find(({ entry }) => entry.call === "key" &&
        entry.args?.[0] === "F12" && entry.actual_events?.some((event) => event.type === "keydown" && event.code === "F12"));
    assert(reservedKey, "KM1 reserved-player rejection lacks an actual physical F12 keydown receipt");
    requireKm1ChangeClickHitsMoveRow(input, captures, "reserved-edit-row", reservedKey.index);
    const noticeCapture = captures.find((capture) => capture.input_call > reservedKey.index &&
        capture.window === reservedKey.entry.window && capture.document_epoch === reservedKey.entry.document_epoch);
    assert(noticeCapture, "KM1 reserved F12 lacks a same-document post-input capture");
    const reservationMenus = noticeCapture.checkpoint.debug.ui_snapshot?.filter((node) =>
        node.kind === "settings_menu" && node.visible === true) || [];
    assert.equal(reservationMenus.length, 1, "KM1 actual reserved-key menu root is missing or duplicated");
    assert(reservationMenus[0].descendants_truncated === false && reservationMenus[0].text_truncated === false,
        "KM1 reserved-key menu observation is truncated");
    const reservations = reservationMenus[0].text_nodes?.filter((node) => node.visible === true &&
        node.text === "F12 is reserved by the browser" && Array.isArray(node.rect) &&
        node.rect.length === 4 && node.rect.every(Number.isFinite) && node.rect[2] > 0 && node.rect[3] > 0) || [];
    requireProduct(reservations.length === 1 &&
        hasKeyBinding(noticeCapture.checkpoint.debug.effective_bindings?.actions?.key_move, "KeyQ") &&
        !hasKeyBinding(noticeCapture.checkpoint.debug.effective_bindings?.actions?.key_move, "F12"),
        "KM1", "reserved-player-notice",
        "actual F12 capture must show one visible settings reservation notice and preserve effective KeyQ");
    const audit = JSON.parse(await view.serverFile("storage-audit.json"));
    assert(Array.isArray(audit.entries) && Array.isArray(audit.final_state),
        "KM1 needs private actual storage write and final-state receipts");
    const latest = postReload.at(-1);
    const root = verifiedRoot(latest, "KM1");
    const line = compatibilityLine(root.version);
    const storageKey = `turts.v1.bindings.${root.id}.${line}`;
    const stored = audit.final_state.find((state) => state.window === 1)?.localStorage?.[storageKey];
    assert(typeof stored === "string", `KM1 player override storage ${storageKey} is absent`);
    requireProduct(audit.entries.some((entry) => entry.source === "storage_api" && entry.storage === "localStorage" && entry.operation === "set" &&
        entry.key === storageKey && entry.value === stored), "KM1", "player-override-reload",
    "KM1 override lacks a successful actual localStorage write receipt");
    return new Set(["keyq-bound-action", "mod-default", "player-override-reload", "reserved-player-notice", "reserved-mod-exit", "labels-guide"]);
}

function judgeKm1GuidePixels(captures, decoded) {
    const labels = [["guide-default", "Move selected units: M"], ["guide-effective", "Move selected units: Q"]];
    const samples = labels.map(([name, expected]) => {
        const capture = captures.find((item) => item.name === name);
        const node = capture?.checkpoint.debug.ui_snapshot?.find((item) => item.id?.startsWith("guide.4.") &&
            item.visible === true && item.text === expected);
        assert(node && Array.isArray(node.rect) && node.rect.length === 4 && node.rect.every(Number.isInteger),
            "KM1 actual guide key label lacks a bounded rectangle");
        const image = decoded.get(name);
        assert(image && image.width === 1280 && image.height === 720, "KM1 guide PNG dimensions differ");
        const [x, y, width, height] = node.rect;
        assert(width >= 75 && height === 24 && x >= 0 && y >= 0 && x + width <= image.width && y + height + 3 <= image.height,
            "KM1 calibrated guide text rectangle is outside the screenshot");
        const light = (rx, ry, rw, rh) => {
            const points = new Set();
            for (let py = ry; py < ry + rh; py += 1) {
                for (let px = rx; px < rx + rw; px += 1) {
                    const at = (py * image.width + px) * 4;
                    if (image.rgba[at] > 180 && image.rgba[at + 1] > 180 && image.rgba[at + 2] > 180 &&
                        image.rgba[at + 3] >= 250) points.add(`${px - x},${py - y}`);
                }
            }
            return points;
        };
        return { capture, rect: node.rect, prefix: light(x, y, 112, height),
            key: light(x + 112, y, 18, height), absence: light(x, y + height, 75, 3) };
    });
    assert(samples[0].capture.document_epoch === samples[1].capture.document_epoch &&
        samples[0].capture.window === samples[1].capture.window && samples[0].capture.input_call < samples[1].capture.input_call,
        "KM1 guide pixel comparison must stay in one actual ordered document");
    const [before, after] = samples;
    const common = [...before.prefix].filter((point) => after.prefix.has(point)).length;
    const changed = [...before.key].filter((point) => !after.key.has(point)).length +
        [...after.key].filter((point) => !before.key.has(point)).length;
    requireProduct(before.prefix.size >= 100 && after.prefix.size >= 100 &&
        common / Math.max(before.prefix.size, after.prefix.size) >= 0.95 &&
        before.key.size >= 20 && after.key.size >= 20 && changed >= 15 &&
        before.absence.size <= 5 && after.absence.size <= 5,
        "KM1", "labels-guide-pixels", `actual guide must keep its label prefix and visibly change M to Q (common=${common}, before=${before.key.size}, after=${after.key.size}, changed=${changed})`);
    return { common, changed, before: before.key.size, after: after.key.size };
}

function requireKm1ChangeClickHitsMoveRow(input, captures, captureName, beforeInputIndex) {
    const rowCapture = captures.find((capture) => capture.name === captureName);
    assert(rowCapture && Array.isArray(rowCapture.checkpoint.debug.ui_snapshot),
        `KM1 ${captureName} lacks an actual pre-click settings-menu snapshot`);
    const menu = rowCapture.checkpoint.debug.ui_snapshot.find((node) => node.kind === "settings_menu" && node.id === "settings_menu");
    assert(menu && menu.visible === true && menu.descendants_truncated === false && menu.text_truncated === false &&
        Array.isArray(menu.text_nodes), `KM1 ${captureName} has no complete visible menu labels`);
    const move = menu.text_nodes.find((node) => node.text === "Move selected units" && node.visible === true &&
        Array.isArray(node.rect) && node.rect.length === 4 && node.rect.every(Number.isFinite));
    const expectedLabel = captureName === "edit-row" ? "M" : "Q";
    const defaultKey = menu.text_nodes.find((node) => node.text === expectedLabel && node.visible === true &&
        Array.isArray(node.rect) && node.rect.length === 4 && node.rect.every(Number.isFinite));
    assert(move && defaultKey && Math.abs(move.rect[1] - defaultKey.rect[1]) <= 1,
        `KM1 ${captureName} does not show the actual Move selected units row with its expected effective label`);
    const [mx, my, mw, mh] = move.rect;
    const change = menu.text_nodes.find((node) => node.text === "Change" && node.visible === true &&
        Array.isArray(node.rect) && node.rect.length === 4 && node.rect.every(Number.isFinite) &&
        node.rect[0] >= mx + mw && node.rect[1] + node.rect[3] >= my && node.rect[1] <= my + mh);
    assert(change, `KM1 ${captureName} has no actual Change button on the Move selected units row`);
    const click = input.map((entry, index) => ({ entry, index })).find(({ entry, index }) => index > rowCapture.input_call &&
        index < beforeInputIndex && entry.call === "click" && entry.args?.[2] === "left" &&
        entry.window === rowCapture.window && entry.document_epoch === rowCapture.document_epoch);
    assert(click, `KM1 ${captureName} has no following physical click on the actual Change button`);
    const point = click.entry.actual_events?.find((event) => event.type === "pointerdown");
    assert(point && Number.isFinite(point.client_x) && Number.isFinite(point.client_y) &&
        point.client_x >= change.rect[0] && point.client_x < change.rect[0] + change.rect[2] &&
        point.client_y >= change.rect[1] && point.client_y < change.rect[1] + change.rect[3],
    `KM1 ${captureName} physical click missed the actual Move selected units Change button rectangle`);
    return click;
}

async function judgeKm1ReservedPack(view) {
    const record = JSON.parse(await view.serverFile("km1-reserved-pack.json"));
    assert.equal(record.source_file, "input/bindings.ron");
    assert.equal(record.reserved_code, "F12");
    assert.match(record.source_sha256, /^[a-f0-9]{64}$/);
    assert.match(record.binary_sha256, /^[a-f0-9]{64}$/);
    assert(Array.isArray(record.argv) && record.argv.length === 2 && record.argv[1].endsWith("km1-reserved-forge"),
        "KM1 reserved-binding check must retain the actual copied-mod packer argv");
    assert.match(record.command_sha256, /^[a-f0-9]{64}$/);
    const commandHash = crypto.createHash("sha256").update(JSON.stringify({
        argv: record.argv,
        binary_sha256: record.binary_sha256,
    })).digest("hex");
    assert.equal(record.command_sha256, commandHash, "KM1 reserved packer command hash does not match its actual argv and binary hash");
    const source = await view.serverBytes("km1-reserved-forge/input/bindings.ron");
    assert.equal(crypto.createHash("sha256").update(source).digest("hex"), record.source_sha256,
        "KM1 reserved-binding source hash does not match the actual copied package file");
    requireProduct(record.exit_code === 2, "KM1", "reserved-mod-exit",
        `copied Forge with a reserved default key made actual turts-pack exit ${record.exit_code}, expected 2`);
    requireProduct(record.stderr.includes("forge/input/bindings.ron") &&
        record.stderr.includes("actions[0].default[0]") &&
        record.stderr.includes("F12 is reserved by the browser"), "KM1", "reserved-mod-exit",
    "turts-pack failure does not identify the copied package input, exact default field, and reserved key");
}

function hasBindingLabel(debug, label) {
    return Array.isArray(debug.ui_snapshot) && debug.ui_snapshot.some((node) =>
        typeof node.id === "string" && node.id.startsWith("guide.4.") && node.visible === true &&
        typeof node.text === "string" && node.text.includes(`Move selected units: ${label}`));
}

function judgeSh1MenuControls(input, captures) {
    const f10Index = input.findIndex((entry) => entry.call === "key" && entry.args?.[0] === "F10" &&
        JSON.stringify(entry.args?.[1] || []) === "[]");
    assert(f10Index >= 0, "SH1 menu baseline requires a physical F10 input call");
    const f10 = input[f10Index];
    const keydown = f10.actual_events?.find((event) => event.type === "keydown" && event.code === "F10");
    assert(keydown && Number.isSafeInteger(keydown.seq) && keydown.seq > 0,
        "SH1 physical F10 call lacks its actual browser keydown receipt");
    const escIndex = input.findIndex((entry, index) => index > f10Index && entry.call === "key" &&
        entry.args?.[0] === "Escape" && JSON.stringify(entry.args?.[1] || []) === "[]" &&
        entry.window === f10.window && entry.document_epoch === f10.document_epoch &&
        entry.actual_events?.some((event) => event.type === "keydown" && event.code === "Escape"));
    assert(escIndex >= 0, "SH1 menu episode needs the actual same-document physical Escape close");
    const esc = input[escIndex];
    const interveningMenuKey = input.findIndex((entry, index) => index > f10Index && index < escIndex &&
        entry.call === "key" && entry.args?.[0] === "F10" && entry.window === f10.window &&
        entry.document_epoch === f10.document_epoch &&
        entry.actual_events?.some((event) => event.type === "keydown" && event.code === "F10"));
    assert.equal(interveningMenuKey, -1,
        "SH1 menu inventory cannot combine multiple F10 open/close episodes before its matching Escape close");
    const menuCaptures = captures.filter((item) => item.input_call > f10Index && item.input_call < escIndex &&
        item.window === f10.window && item.document_epoch === f10.document_epoch);
    assert(menuCaptures.length >= 2, "SH1 menu episode needs actual top and bottom screenshots before Escape");
    const debug = menuCaptures[0].checkpoint.debug;
    const menuFrame = retainedFrames(debug).find((frame) => frame.applied_seq?.includes(keydown.seq) &&
        frame.named_dispatches?.some((dispatch) => dispatch.origin === "key" &&
            dispatch.code === "F10" && dispatch.matched_action === "menu"));
    requireProduct(Boolean(menuFrame), "SH1", "menu-controls",
        "physical F10 did not reach the actual menu action dispatch in the retained Core frame");
    const visibleLabelsByCapture = new Map();
    for (const capture of menuCaptures) {
        const captureDebug = capture.checkpoint.debug;
        assert(Array.isArray(captureDebug.ui_snapshot), "SH1 actual UiSnapshotFeed is unavailable at a menu screenshot");
        const menus = captureDebug.ui_snapshot.filter((node) => node.kind === "settings_menu" && node.id === "settings_menu");
        requireProduct(menus.length === 1, "SH1", "menu-controls",
            "the actual settings-menu root disappeared or multiplied during one physical scroll episode");
        const menu = menus[0];
        assert.equal(menu.descendants_truncated, false, "settings-menu UI-tree traversal overflowed");
        assert.equal(menu.text_truncated, false, "settings-menu text observation overflowed");
        assert(Array.isArray(menu.text_nodes), "actual settings-menu descendant text list is missing");
        requireProduct(menu.visible === true && menu.capture_root === true && menu.focus_policy_block === true &&
            Array.isArray(menu.rect) && menu.rect.length === 4 && menu.rect.every(Number.isFinite) &&
            menu.rect[2] > 0 && menu.rect[3] > 0,
        "SH1", "menu-controls", "the actual settings menu was not visible with its bounded capture root throughout the physical scroll episode");
        visibleLabelsByCapture.set(capture, new Set(menu.text_nodes.filter((node) => node.visible === true &&
            typeof node.text === "string" && Array.isArray(node.rect) && node.rect.length === 4 &&
            node.rect.every(Number.isFinite) && node.rect[2] > 0 && node.rect[3] > 0).map((node) => node.text)));
    }
    const labelsIn = (capture, required) => required.every((label) => visibleLabelsByCapture.get(capture)?.has(label));
    const topCapture = menuCaptures.find((capture) => labelsIn(capture, ["Settings", "Key bindings"]));
    const bottomCapture = menuCaptures.find((capture) => labelsIn(capture, ["Audio levels", "Master", "Voice", "Alerts", "Effects"]));
    requireProduct(Boolean(topCapture && bottomCapture && topCapture.input_call < bottomCapture.input_call),
        "SH1", "menu-controls", "the same menu episode lacks ordered visible top and bottom layout captures");
    const wheelReceipt = input.find((entry) => entry.call === "wheel" && entry.window === f10.window &&
        entry.document_epoch === f10.document_epoch && input.indexOf(entry) > topCapture.input_call &&
        input.indexOf(entry) < bottomCapture.input_call && entry.actual_events?.some((event) => event.type === "wheel"));
    assert(wheelReceipt,
        "SH1 menu inventory is missing the actual same-document wheel receipt between top and bottom screenshots");
    const visibleLabels = new Set([...visibleLabelsByCapture.values()].flatMap((labels) => [...labels]));
    const requiredLabels = [
        "Settings", "Key bindings", "Change", "Reset", "Audio levels",
        "Master", "Voice", "Alerts", "Effects", "Scroll speed", "Edge scroll",
        "UI scale", "Health bars", "Keep the cursor in the game", "Double-click time", "Double-click range",
    ];
    for (const label of requiredLabels) {
        requireProduct(visibleLabels.has(label), "SH1", "menu-controls",
            "the actual settings-menu tree is missing visible label " + JSON.stringify(label));
    }
    const toggleRows = [
        ["Edge scroll", new Set(["On", "Off"])],
        ["Health bars", new Set(["Selected", "Damaged or selected", "Always"])],
        ["Keep the cursor in the game", new Set(["On", "Off"])],
    ];
    for (const [labelText, allowedStates] of toggleRows) {
        const hasActualState = menuCaptures.some((capture) => {
            const nodes = capture.checkpoint.debug.ui_snapshot
                .find((node) => node.kind === "settings_menu" && node.id === "settings_menu")?.text_nodes || [];
            return nodes.some((label) => label.text === labelText && label.visible === true &&
                Array.isArray(label.rect) && label.rect.length === 4 && label.rect.every(Number.isFinite) &&
                label.rect[2] > 0 && label.rect[3] > 0 && nodes.some((state) => allowedStates.has(state.text) &&
                    state.visible === true && Array.isArray(state.rect) && state.rect.length === 4 &&
                    state.rect.every(Number.isFinite) && state.rect[2] > 0 && state.rect[3] > 0 &&
                    Math.abs(state.rect[1] - label.rect[1]) <= 1 && state.rect[0] >= label.rect[0] + label.rect[2] - 1));
        });
        requireProduct(hasActualState, "SH1", "menu-controls",
            `the actual settings menu lacks a visible state value aligned with ${JSON.stringify(labelText)}`);
    }
    const closeCapture = captures.find((item) => item.input_call > escIndex &&
        item.window === f10.window && item.document_epoch === f10.document_epoch);
    assert(closeCapture, "SH1 physical Escape requires a same-document post-close screenshot");
    const closeFrames = retainedFrames(closeCapture.checkpoint.debug);
    requireProduct(closeFrames.some((frame) => frame.applied_seq?.some((seq) => esc.actual_events.some((event) => event.seq === seq)) &&
        frame.named_dispatches?.some((dispatch) => dispatch.origin === "key" && dispatch.code === "Escape" &&
            ["menu", "cancel"].includes(dispatch.matched_action))), "SH1", "menu-controls",
    "physical Escape did not reach the actual menu action dispatch");
    assert(Array.isArray(closeCapture.checkpoint.debug.ui_snapshot), "SH1 closed-menu UiSnapshotFeed is unavailable");
    requireProduct(!closeCapture.checkpoint.debug.ui_snapshot.some((node) => node.kind === "settings_menu"),
        "SH1", "menu-controls", "the actual settings menu remained open after its physical Escape close");
    return new Set(["menu-controls"]);
}

function compatibilityLine(version) {
    const match = /^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:[-+][0-9A-Za-z.-]+)?$/.exec(version);
    assert(match, "verified root version must retain exact semantic-version components");
    const [, major, minor] = match;
    return major === "0" ? `${major}.${minor}` : major;
}

function completedCaptureHostMs(capture, input) {
    const receipt = input[capture.screenshot_input_call ?? capture.input_call];
    assert(receipt?.call === "screenshot" && receipt.window === capture.window &&
        receipt.document_epoch === capture.document_epoch && Number.isFinite(receipt.wall_time_ms) &&
        Number.isFinite(receipt.completed_wall_time_ms) && receipt.completed_wall_time_ms >= receipt.wall_time_ms,
    "capture boundary requires its actual completed screenshot receipt in the same document");
    return receipt.completed_wall_time_ms;
}

function visibleCountdown(capture) {
    const debug = capture.checkpoint.debug;
    const text = debug.rejoin?.text;
    const match = typeof text === "string" && /^Reconnecting to your game\n([1-9]|1[0-5]) s$/.exec(text);
    if (!match) return null;
    const notice = debug.ui_snapshot?.find((node) => node.kind === "client_notice" && node.id === "client_notice" &&
        node.visible === true && node.text === text && Array.isArray(node.rect) && node.rect.length === 4 &&
        node.rect.every(Number.isFinite) && node.rect[2] > 0 && node.rect[3] > 0);
    assert(notice, "actual reconnect countdown must be visible in its same-capture engine notice");
    const elapsed = debug.rejoin.elapsed_secs;
    assert(Number.isFinite(elapsed) && elapsed >= 0 && elapsed < 15 && Number(match[1]) === Math.ceil(15 - elapsed),
        "visible reconnect countdown must match the actual bounded retry clock");
    return { capture, remaining: Number(match[1]), elapsed };
}

function assertSlot1CapacityBoundary(records, captureHostUs, popupOpenedHostUs, acceptedSessions) {
    assert(Number.isFinite(captureHostUs) && Number.isFinite(popupOpenedHostUs) && popupOpenedHostUs > captureHostUs,
        "SLOT1 capacity boundary needs ordered actual capture and popup-open host times");
    assert(Array.isArray(acceptedSessions) && acceptedSessions.length === 2,
        "SLOT1 capacity boundary is fixed to the actual accepted A and B sessions");
    const boundary = records.find((record) => record.event === "owner_checkpoint" &&
        record.host_unix_us >= captureHostUs && record.host_unix_us < popupOpenedHostUs);
    assert(boundary, "SLOT1 requires a healthy actual owner checkpoint after C's capacity refusal capture and before opening the copied window");
    assert.equal(boundary.match_id_decimal, acceptedSessions[0].match_id_decimal,
        "SLOT1 capacity boundary must remain in the owners' actual match");
    for (const accepted of acceptedSessions) {
        assert(boundary.active_epochs.some((active) =>
            active.entity_bits_with_generation_decimal === accepted.entity_bits_with_generation_decimal &&
            active.connection_epoch_decimal === accepted.connection_epoch_decimal &&
            active.identity_alias === accepted.identity_alias &&
            active.match_id_decimal === accepted.match_id_decimal && active.slot === accepted.decision.slot),
        "SLOT1 capacity boundary must prove each exact accepted A/B Session epoch active");
        assert.equal(boundary.owners.find((owner) => owner.slot === accepted.decision.slot)?.identity_alias,
            accepted.identity_alias,
            "SLOT1 capacity boundary must retain each actual owner before C is replaced by a copied document");
    }
    return boundary;
}

function assertSlot1CloneOwnerContinuity(records, popupOpenedHostUs, terminalCaptureHostUs, acceptedSessions) {
    assert(Number.isFinite(popupOpenedHostUs) && Number.isFinite(terminalCaptureHostUs) &&
        terminalCaptureHostUs > popupOpenedHostUs,
    "SLOT1 owner continuity needs ordered actual popup-open and completed terminal-capture host times");
    assert(Array.isArray(acceptedSessions) && acceptedSessions.length === 2,
        "SLOT1 owner continuity is fixed to the actual accepted A and B sessions");
    const stillOpen = (record) => record.host_unix_us >= popupOpenedHostUs && record.host_unix_us <= terminalCaptureHostUs;
    for (const accepted of acceptedSessions) {
        assert(!records.some((record) => record.event === "session_authority_removed" && stillOpen(record) &&
            record.entity_bits_with_generation_decimal === accepted.entity_bits_with_generation_decimal &&
            record.connection_epoch_decimal === accepted.connection_epoch_decimal),
        `SLOT1 original owner ${accepted.entity_bits_with_generation_decimal}/${accepted.connection_epoch_decimal} must remain live through the copied-tab terminal capture`);
    }
    const checkpoints = records.filter((record) => record.event === "owner_checkpoint" && stillOpen(record));
    assert(checkpoints.length >= 10, "SLOT1 needs bounded actual owner checkpoints from browser-open through terminal capture");
    for (const checkpoint of checkpoints) {
        assert.equal(checkpoint.match_id_decimal, acceptedSessions[0].match_id_decimal,
            "SLOT1 copied-tab checkpoint must remain in the actual owners' match");
        for (const accepted of acceptedSessions) {
            assert(checkpoint.active_epochs.some((active) =>
                active.entity_bits_with_generation_decimal === accepted.entity_bits_with_generation_decimal &&
                active.connection_epoch_decimal === accepted.connection_epoch_decimal &&
                active.identity_alias === accepted.identity_alias &&
                active.match_id_decimal === accepted.match_id_decimal && active.slot === accepted.decision.slot),
            "SLOT1 every copied-tab interval checkpoint must prove both exact original Session epochs remain active");
            assert.equal(checkpoint.owners.find((owner) => owner.slot === accepted.decision.slot)?.identity_alias,
                accepted.identity_alias,
                "SLOT1 every copied-tab interval checkpoint must retain each original owner in its actual slot");
        }
    }
    return checkpoints;
}

function assertSlot1DeadlineChecks(rejoin) {
    const anchor = rejoin?.first_transport_attempt_real_secs;
    const terminalAt = rejoin?.terminal_at_real_secs;
    const terminalElapsed = rejoin?.terminal_elapsed_secs;
    const pending = rejoin?.last_pending_deadline_check;
    const terminal = rejoin?.first_terminal_deadline_check;
    assert(Number.isFinite(anchor) && Number.isFinite(terminalAt) && Number.isFinite(terminalElapsed) &&
        pending && terminal, "SLOT1 is missing the actual paired deadline-check timestamps and frame identities");
    const cutoff = anchor + 15;
    assert(Number.isFinite(cutoff) && Number.isFinite(pending.real_now_secs) &&
        Number.isFinite(pending.elapsed_secs) && Number.isFinite(terminal.real_now_secs) &&
        Number.isFinite(terminal.elapsed_secs) && Number.isInteger(pending.check_index) &&
        Number.isInteger(terminal.check_index) && Number.isInteger(pending.frame_index) &&
        Number.isInteger(terminal.frame_index) && typeof pending.source === "string" &&
        typeof terminal.source === "string" && pending.source.length > 0 && terminal.source.length > 0,
    "SLOT1 deadline observations lack finite actual times, monotonic check indices, frame identities, or producer sources");
    assert.equal(pending.elapsed_secs, pending.real_now_secs - anchor,
        "SLOT1 last pending sample must preserve the unclamped elapsed time");
    assert.equal(terminal.elapsed_secs, terminal.real_now_secs - anchor,
        "SLOT1 first terminal sample must preserve the unclamped elapsed time");
    assert(pending.real_now_secs < cutoff,
        "SLOT1 latest pending deadline check must precede the actual first-transport cutoff");
    assert(terminal.real_now_secs >= cutoff,
        "SLOT1 first terminal deadline check must be at or after the exact 15-second cutoff");
    assert.equal(pending.check_index + 1, terminal.check_index,
        "SLOT1 must not retain an intermediate deadline check between last pending and first terminal");
    assert(((terminal.frame_index - pending.frame_index) >>> 0) <= 1,
        "SLOT1 deadline observations must bind adjacent actual Bevy update frames");
    assert.equal(terminalAt, terminal.real_now_secs,
        "SLOT1 actual terminal event must be the first eligible deadline check");
    assert.equal(terminalElapsed, terminal.elapsed_secs,
        "SLOT1 terminal elapsed time must remain the raw elapsed sample, not a clamped countdown value");
    assert.equal(rejoin.deadline_checks_causal, true,
        "SLOT1 actual producer did not qualify the paired deadline observations");
    return {
        crossingGapSecs: terminal.real_now_secs - cutoff,
        observedElapsedSecs: terminal.elapsed_secs,
        pendingFrame: pending.frame_index,
        terminalFrame: terminal.frame_index,
    };
}

function assertSlot1HelloTimes(retryTimes) {
    assert(Array.isArray(retryTimes) && retryTimes.length >= 2 && retryTimes.every((value, index) =>
        Number.isFinite(value) && value >= 0 && value < 15 &&
        (index === 0 || value - retryTimes[index - 1] >= 1)),
    "SLOT1 every actual client Hello must be sent before the absolute 15-second cutoff and at least one second after the prior send");
}

function assertSlot1TerminalPixels(capture, image) {
    assert(image && image.width === 1280 && image.height === 720,
        "SLOT1 terminal pixel contract requires the actual 1280x720 product screenshot");
    const notice = capture.checkpoint.debug.ui_snapshot?.find((node) =>
        node.kind === "client_notice" && node.id === "client_notice");
    assert(notice && notice.visible === true &&
        notice.text === "This game is open in another window. Close it, then reload this \u0070age",
    "SLOT1 terminal pixels must bind to the actual visible engine-owned duplicate-window notice");
    assert(!capture.checkpoint.debug.ui_snapshot.some((node) => node.kind === "focus_overlay" && node.visible === true),
        "SLOT1 terminal pixel setup retains a focus overlay; use the real canvas focus gesture before capture");
    const rect = notice.rect;
    assert(Array.isArray(rect) && rect.length === 4 && rect.every(Number.isFinite),
        "SLOT1 terminal notice has no actual post-layout rectangle");
    const [x, y, width, height] = rect;
    assert(Math.abs(x - 16) <= 1 && Math.abs(width - 420) <= 1 && Math.abs(y + height - 704) <= 1 &&
        height >= 24 && height <= 100,
    "SLOT1 terminal notice rectangle differs from the actual 16px inset, 420px wide client notice layout");
    const left = Math.max(0, Math.ceil(x + 8));
    const right = Math.min(image.width, Math.floor(x + width - 8));
    const top = Math.max(0, Math.ceil(y + 8));
    const bottom = Math.min(image.height, Math.floor(y + height - 8));
    assert(right > left && bottom > top, "SLOT1 terminal notice text box falls outside the screenshot");
    let whiteTextPixels = 0;
    let contrastingTextEdges = 0;
    let minWhiteX = image.width;
    let maxWhiteX = -1;
    let minWhiteY = image.height;
    let maxWhiteY = -1;
    const whitePixelKeys = [];
    for (let py = top; py < bottom; py += 1) {
        for (let px = left; px < right; px += 1) {
            const index = (py * image.width + px) * 4;
            const [red, green, blue, alpha] = image.rgba.subarray(index, index + 4);
            if (red < 245 || green < 245 || blue < 245 || alpha < 250) continue;
            whiteTextPixels += 1;
            minWhiteX = Math.min(minWhiteX, px);
            maxWhiteX = Math.max(maxWhiteX, px);
            minWhiteY = Math.min(minWhiteY, py);
            maxWhiteY = Math.max(maxWhiteY, py);
            whitePixelKeys.push(`${px},${py}`);
            let hasContrastingNeighbour = false;
            for (let dy = -1; dy <= 1 && !hasContrastingNeighbour; dy += 1) {
                for (let dx = -1; dx <= 1; dx += 1) {
                    if (dx === 0 && dy === 0) continue;
                    const nx = px + dx;
                    const ny = py + dy;
                    if (nx < left || nx >= right || ny < top || ny >= bottom) continue;
                    const neighbour = (ny * image.width + nx) * 4;
                    if (image.rgba[neighbour] < 220 || image.rgba[neighbour + 1] < 220 || image.rgba[neighbour + 2] < 220) {
                        hasContrastingNeighbour = true;
                        break;
                    }
                }
            }
            if (hasContrastingNeighbour) contrastingTextEdges += 1;
        }
    }
    const whiteTextWidth = maxWhiteX - minWhiteX + 1;
    const whiteTextHeight = maxWhiteY - minWhiteY + 1;
    const remainingWhitePixels = new Set(whitePixelKeys);
    let glyphComponents = 0;
    while (remainingWhitePixels.size > 0) {
        const pending = [remainingWhitePixels.values().next().value];
        remainingWhitePixels.delete(pending[0]);
        let componentSize = 0;
        while (pending.length > 0) {
            const [componentX, componentY] = pending.pop().split(",").map(Number);
            componentSize += 1;
            for (let dy = -1; dy <= 1; dy += 1) {
                for (let dx = -1; dx <= 1; dx += 1) {
                    if (dx === 0 && dy === 0) continue;
                    const neighbor = `${componentX + dx},${componentY + dy}`;
                    if (remainingWhitePixels.delete(neighbor)) pending.push(neighbor);
                }
            }
        }
        if (componentSize >= 2) glyphComponents += 1;
    }
    assert(whiteTextPixels >= 24 && contrastingTextEdges >= 8 && whiteTextWidth >= 80 && whiteTextHeight >= 8 && glyphComponents >= 4,
        `SLOT1 terminal screenshot lacks distributed white notice glyph pixels and contrast inside its observed rectangle (white=${whiteTextPixels}, edges=${contrastingTextEdges}, span=${whiteTextWidth}x${whiteTextHeight}, components=${glyphComponents})`);
    return { width: image.width, height: image.height, rect, whiteTextPixels, contrastingTextEdges, whiteTextWidth, whiteTextHeight, glyphComponents };
}

async function judgeKm2(view, input, captures, decoded) {
    await judgeP1Attachment(view, "KM2");
    const joined = captures.find((capture) => capture.name === "joined-before-input" && capture.checkpoint.debug.phase?.startsWith("Joined"));
    assert(joined && !input.slice(0, joined.input_call).some((entry) => entry.actual_events?.length ||
        ["click", "drag", "key", "move", "wheel"].includes(entry.call)),
        "KM2 joined focus must be observed before any physical focus gesture");
    assert(joined?.checkpoint.browser && typeof joined.checkpoint.browser.canvas_focused === "boolean",
        "KM2 needs a same-capture read-only DOM canvas focus observation after join");
    requireProduct(joined.checkpoint.browser.canvas_focused, "KM2", "join-focus",
        "the actual joined document does not have the canvas as document.activeElement");

    const rightClick = input.find((entry) => entry.call === "click" && entry.args?.[2] === "right");
    assert(rightClick, "KM2 has no actual physical right-click receipt");
    const canvasCapture = captures.find((capture) => capture.window === rightClick.window &&
        capture.document_epoch === rightClick.document_epoch && capture.checkpoint.browser?.canvas);
    assert(canvasCapture, "KM2 has no same-document actual canvas identity for the physical contextmenu target");
    const contextEvents = rightClick.contextmenu_events || [];
    assert(contextEvents.length > 0, "KM2 contextmenu listener observed no event for the physical right-click");
    for (const event of contextEvents) {
        assert.equal(event.source_call_index, input.indexOf(rightClick),
            "KM2 contextmenu event lost its actual physical source call");
        assert(Number.isSafeInteger(event.wall_time_ms) && event.wall_time_ms >= rightClick.wall_time_ms &&
            event.wall_time_ms <= rightClick.completed_wall_time_ms,
        "KM2 contextmenu receipt is outside its physical click's actual time interval");
        assert(event.seq === null || (Number.isSafeInteger(event.seq) && rightClick.event_seq.includes(event.seq)),
            "KM2 contextmenu receipt sequence must be an actual physical event sequence or null");
        assert.equal(event.target, "CANVAS", "KM2 contextmenu event must target the actual canvas");
        assert.equal(event.canvas_target, true, "KM2 contextmenu event target is outside the actual canvas");
        assert.equal(event.target_id, canvasCapture.checkpoint.browser.canvas.id,
            "KM2 contextmenu event target ID differs from the actual same-document canvas identity");
        assert.equal(event.cancelable, true, "KM2 contextmenu event must report its actual cancelable state");
        assert.equal(typeof event.default_prevented, "boolean",
            "KM2 contextmenu observer did not retain defaultPrevented after event dispatch");
    }
    const preventedCount = contextEvents.filter((event) => event.default_prevented).length;
    requireProduct(preventedCount === contextEvents.length, "KM2", "contextmenu-prevented-delta",
        `the actual canvas contextmenu count (${contextEvents.length}) differs from its prevented count (${preventedCount})`);

    const menuOpenIndex = input.findIndex((entry) => entry.call === "key" && entry.args?.[0] === "F10" &&
        JSON.stringify(entry.args?.[1] || []) === "[]");
    assert(menuOpenIndex >= 0, "KM2 needs the physical F10 settings-menu open before focus loss");
    const menuOpen = input[menuOpenIndex];
    const menuOpenDown = menuOpen.actual_events?.find((event) => event.type === "keydown" && event.code === "F10");
    assert(menuOpenDown && Number.isSafeInteger(menuOpenDown.seq), "KM2 F10 menu-open call lacks actual browser sequence evidence");
    const menuCapture = captures.find((capture) => capture.input_call > menuOpenIndex &&
        capture.checkpoint.debug.phase?.startsWith("Joined") && capture.window === menuOpen.window &&
        capture.document_epoch === menuOpen.document_epoch &&
        retainedFrames(capture.checkpoint.debug).some((frame) => frame.applied_seq?.includes(menuOpenDown.seq) &&
            frame.named_dispatches?.some((dispatch) => dispatch.origin === "key" && dispatch.code === "F10" &&
                dispatch.matched_action === "menu")));
    assert(menuCapture, "KM2 F10 menu-open has no same-document post-dispatch capture");
    const menuRows = menuCapture.checkpoint.debug.ui_snapshot;
    assert(Array.isArray(menuRows), "KM2 F10 menu-open capture lacks the actual UiSnapshotFeed");
    requireProduct(menuRows.filter((node) => node.kind === "settings_menu" && node.id === "settings_menu").some((menu) =>
        menu.visible === true && menu.capture_root === true && menu.focus_policy_block === true &&
        Array.isArray(menu.rect) && menu.rect.length === 4 && menu.rect.every(Number.isFinite) && menu.rect[2] > 0 && menu.rect[3] > 0),
    "KM2", "join-focus", "physical F10 did not present the actual bounded settings menu before canvas blur");
    const menuCloseIndex = input.findIndex((entry, index) => index > menuOpenIndex && entry.call === "key" &&
        entry.args?.[0] === "F10" && JSON.stringify(entry.args?.[1] || []) === "[]" &&
        entry.window === menuOpen.window && entry.document_epoch === menuOpen.document_epoch);
    assert(menuCloseIndex >= 0, "KM2 needs the physical same-document F10 menu close before canvas blur");
    const menuClose = input[menuCloseIndex];
    const menuCloseDown = menuClose.actual_events?.find((event) => event.type === "keydown" && event.code === "F10");
    assert(menuCloseDown && Number.isSafeInteger(menuCloseDown.seq), "KM2 F10 menu-close call lacks actual browser sequence evidence");
    const menuClosedCapture = captures.find((capture) => capture.input_call > menuCloseIndex &&
        capture.window === menuOpen.window && capture.document_epoch === menuOpen.document_epoch &&
        retainedFrames(capture.checkpoint.debug).some((frame) => frame.applied_seq?.includes(menuCloseDown.seq) &&
            frame.named_dispatches?.some((dispatch) => dispatch.origin === "key" && dispatch.code === "F10" &&
                dispatch.matched_action === "menu")));
    assert(menuClosedCapture, "KM2 F10 menu-close has no same-document post-dispatch capture");
    assert(Array.isArray(menuClosedCapture.checkpoint.debug.ui_snapshot), "KM2 menu-close capture lacks the actual UiSnapshotFeed");
    requireProduct(!menuClosedCapture.checkpoint.debug.ui_snapshot.some((node) => node.kind === "settings_menu"),
        "KM2", "join-focus", "physical F10 did not close the actual settings menu before canvas blur");

    const outsideIndex = input.findIndex((entry) => entry.call === "click" && entry.args?.[0] === -5 && entry.args?.[1] === -5);
    assert(outsideIndex >= 0, "KM2 outside-canvas action must use the brief's physical (-5,-5) click");
    const outsideClick = input[outsideIndex];
    const outsideDown = outsideClick.actual_events.find((event) => event.type === "pointerdown");
    assert(outsideDown && outsideDown.canvas_target === false && outsideDown.client_x === -5 && outsideDown.client_y === -5,
        "KM2 outside click receipt must show the actual pointer landed outside the canvas");
    const outsideUp = outsideClick.actual_events.find((event) => event.type === "pointerup");
    assert(outsideUp && outsideUp.canvas_target === false && outsideUp.client_x === -5 && outsideUp.client_y === -5,
        "KM2 outside click receipt must retain its actual pointerup outside the canvas");
    const outsideCapture = captures.find((capture) => capture.input_call > outsideIndex &&
        capture.window === outsideClick.window && capture.document_epoch === outsideClick.document_epoch);
    assert(outsideCapture?.checkpoint.browser && typeof outsideCapture.checkpoint.browser.canvas_focused === "boolean",
        "KM2 outside click lacks a same-document DOM focus checkpoint");
    assert.equal(outsideCapture.checkpoint.browser.canvas_focused, false,
        "KM2 physical outside click did not move document.activeElement away from the canvas");
    assert.equal(outsideCapture.checkpoint.browser.active_element?.tag_name, "BODY",
        "KM2 outside click lacks the actual body activeElement receipt from canvas blur");
    const overlay = judgeKm2FocusOverlay(outsideCapture.checkpoint.debug);

    const keyIndex = input.findIndex((entry, index) => index > outsideIndex && entry.call === "key");
    assert(keyIndex >= 0, "KM2 has no next physical key after the outside click");
    const outsideKey = input[keyIndex];
    const keyDown = outsideKey.actual_events.find((event) => event.type === "keydown");
    assert(keyDown && Number.isSafeInteger(keyDown.seq), "KM2 outside key lacks its actual keydown sequence");
    requireProduct(keyDown.canvas_target === false, "KM2", "outside-key-stopped",
        "the next key after focus loss still targets the game canvas");
    const keyCapture = captures.find((capture) => capture.input_call > keyIndex &&
        capture.window === outsideKey.window && capture.document_epoch === outsideKey.document_epoch);
    assert(keyCapture, "KM2 outside key lacks a same-document actual post-key frame capture");
    const frames = [...(keyCapture.checkpoint.debug.frames?.recent || []),
        ...(keyCapture.checkpoint.debug.frames?.retained_input || [])];
    requireProduct(!frames.some((frame) => frame.applied_seq?.includes(keyDown.seq)), "KM2", "outside-key-stopped",
        "the actual outside key sequence reached the interaction Core after focus loss");
    judgeKm2OverlayPixels(captures, decoded, outsideCapture);
    return new Set(["join-focus", "contextmenu-prevented-delta", "canvas-blur-overlay", "outside-key-stopped", "p1-attachment", "canvas-blur-pixels"]);
}

async function judgeP1Attachment(view, row) {
    assert(["KM2", "CAM2"].includes(row), "P1 attachment contract is fixed to KM2 and CAM2");
    const p1Bytes = await view.serverBytes("p1-keys.md");
    const expectedHash = "f41d55729fa45f291d36d6a780d9aeb021242d61703dba39eed38e78c16684c1";
    const actualHash = crypto.createHash("sha256").update(p1Bytes).digest("hex");
    assert.equal(actualHash, expectedHash,
        `${row} requires the exact accepted headed Chrome P1 attachment; Firefox remains NOT TRACED`);
    const attachment = JSON.parse(await view.serverFile("p1-attachment.json"));
    assert.equal(attachment.source_path, "docs/research/mc/p1-keys.md",
        `${row} P1 attachment has the wrong recorded source path`);
    assert.equal(attachment.sha256, expectedHash, `${row} P1 metadata hash differs from attached bytes`);
    assert.equal(attachment.byte_length, p1Bytes.length, `${row} P1 metadata length differs from attached bytes`);
    assert.equal(attachment.attached_before_physical_run, true,
        `${row} P1 evidence was not attached before the physical run`);
    const text = p1Bytes.toString("utf8");
    assert.match(text, /^\| Esc \(pointer locked\) \| Chrome \| Google Chrome 156\.0\.8078\.47 \| no \| n\/a \| n\/a \| yes \|$/m,
        `${row} accepted P1 attachment lacks the headed Chrome pointer-lock Escape outcome`);
    assert.match(text, /^Firefox (?:remains )?NOT TRACED$/m,
        `${row} accepted P1 attachment must preserve the Firefox limitation`);
    return { sha256: actualHash, byteLength: p1Bytes.length };
}

function judgeKm2FocusOverlay(debug) {
    assert(Array.isArray(debug.ui_snapshot), "KM2 focus overlay needs the actual UiSnapshotFeed captured after canvas blur");
    assert(!debug.ui_snapshot.some((node) => node.kind === "focus_overlay_overflow"),
        "KM2 focus overlay root observation overflowed");
    const rows = debug.ui_snapshot.filter((node) => node.kind === "focus_overlay" && node.id === "focus_overlay");
    assert.equal(rows.length, 1, "KM2 focus overlay snapshot row is missing or duplicated");
    const overlay = rows[0];
    assert(Number.isSafeInteger(overlay.root_count) && overlay.root_count >= 0,
        "KM2 focus overlay row has no actual bounded root count");
    if (overlay.root_count > 1) {
        requireProduct(false, "KM2", "canvas-blur-overlay",
            "canvas blur produced multiple actual focus overlays instead of one");
    }
    requireProduct(overlay.root_count === 1, "KM2", "canvas-blur-overlay",
        "actual DOM canvas blur did not produce the existing engine focus overlay");
    assert.equal(overlay.descendants_truncated, false, "KM2 focus overlay descendant observation overflowed");
    assert.equal(overlay.text_truncated, false, "KM2 focus overlay text observation overflowed");
    assert(Array.isArray(overlay.text_nodes), "KM2 focus overlay has no actual descendant text observation");
    const rect = overlay.rect;
    requireProduct(overlay.visible === true && overlay.capture_root === true && overlay.focus_policy_block === true &&
        Array.isArray(rect) && rect.length === 4 && rect.every(Number.isFinite) && rect[2] > 0 && rect[3] > 0,
    "KM2", "canvas-blur-overlay", "the actual focus overlay is not visible, laid out, and capturing UI input");
    const label = overlay.text_nodes.find((node) => node.text === "Click to play" && node.visible === true &&
        Array.isArray(node.rect) && node.rect.length === 4 && node.rect.every(Number.isFinite) &&
        node.rect[2] > 0 && node.rect[3] > 0);
    requireProduct(Boolean(label), "KM2", "canvas-blur-overlay",
        "the visible focus overlay lacks its actual Click to play label");
    return overlay;
}

function judgeKm2OverlayPixels(captures, decoded, afterCapture) {
    assert(afterCapture?.name === "canvas-blurred", "KM2 overlay pixels must use the exact physical outside-click capture");
    const beforeCapture = captures.find((capture) => capture.name === "joined-focused");
    assert(beforeCapture, "KM2 overlay pixels need the same-document pre-blur capture as an absence control");
    assert(beforeCapture.window === afterCapture.window && beforeCapture.document_epoch === afterCapture.document_epoch &&
        beforeCapture.input_call < afterCapture.input_call,
    "KM2 same-rectangle pixel control must come from the same actual window and document before physical blur");

    const beforeRows = beforeCapture.checkpoint.debug.ui_snapshot;
    assert(Array.isArray(beforeRows), "KM2 pre-blur pixel control lacks the actual UiSnapshotFeed");
    const beforeOverlay = beforeRows.find((node) => node.kind === "focus_overlay" && node.id === "focus_overlay");
    assert(beforeOverlay && beforeOverlay.root_count === 0 && beforeOverlay.visible === false,
        "KM2 same-rectangle absence control requires an actual no-overlay pre-blur capture");
    const overlay = judgeKm2FocusOverlay(afterCapture.checkpoint.debug);
    const label = overlay.text_nodes.find((node) => node.text === "Click to play" && node.visible === true);
    const rect = label?.rect;
    assert(Array.isArray(rect) && rect.length === 4 && rect.every(Number.isInteger) &&
        rect[0] >= 0 && rect[1] >= 0 && rect[2] > 0 && rect[3] > 0,
    "KM2 pixel ROI must come from the actual laid-out Click to play snapshot rectangle");

    const before = decoded.get(beforeCapture.name);
    const after = decoded.get(afterCapture.name);
    assert(before && after && before.width === 1280 && before.height === 720 &&
        after.width === before.width && after.height === before.height,
    "KM2 pixel proof requires matching real 1280x720 same-document screenshots");
    const [x, y, width, height] = rect;
    assert(x + width <= after.width && y + height <= after.height,
        "KM2 actual label rectangle falls outside the physical PNG");
    const beforeWhite = nearWhitePixels(before, rect);
    const afterWhite = nearWhitePixels(after, rect);
    const beforeSet = new Set(beforeWhite.map(([px, py]) => `${px},${py}`));
    const addedWhite = afterWhite.filter(([px, py]) => !beforeSet.has(`${px},${py}`));
    const glyph = pixelGlyphShape(addedWhite);
    const nearby = y + (2 * height) + 2 <= after.height ? [x, y + height + 2, width, height] : [x, y - height - 2, width, height];
    assert(nearby[1] >= 0 && nearby[1] + nearby[3] <= after.height,
        "KM2 actual overlay label has no same-sized nearby negative region in the screenshot");
    const nearbyWhite = nearWhitePixels(after, nearby).length;

    requireProduct(afterWhite.length >= 250 && addedWhite.length >= 250 && glyph.width >= 100 &&
        glyph.height >= 14 && glyph.components >= 8 && beforeWhite.length <= 100 && nearbyWhite <= 100,
    "KM2", "canvas-blur-pixels",
    `actual Click to play pixels are absent, too small, or lack same-rect/nearby contrast (before=${beforeWhite.length}, after=${afterWhite.length}, added=${addedWhite.length}, span=${glyph.width}x${glyph.height}, components=${glyph.components}, nearby=${nearbyWhite})`);
    return { rect, beforeWhite: beforeWhite.length, afterWhite: afterWhite.length,
        addedWhite: addedWhite.length, glyph, nearby, nearbyWhite };
}

function nearWhitePixels(image, rect) {
    const [x, y, width, height] = rect;
    const points = [];
    for (let py = y; py < y + height; py += 1) {
        for (let px = x; px < x + width; px += 1) {
            const index = (py * image.width + px) * 4;
            if (image.rgba[index] > 235 && image.rgba[index + 1] > 235 && image.rgba[index + 2] > 235 && image.rgba[index + 3] >= 250) {
                points.push([px, py]);
            }
        }
    }
    return points;
}

function pixelGlyphShape(points) {
    if (points.length === 0) return { width: 0, height: 0, components: 0 };
    const remaining = new Set(points.map(([x, y]) => `${x},${y}`));
    let minX = Number.MAX_SAFE_INTEGER;
    let maxX = -1;
    let minY = Number.MAX_SAFE_INTEGER;
    let maxY = -1;
    let components = 0;
    while (remaining.size > 0) {
        const first = remaining.values().next().value;
        const pending = [first];
        remaining.delete(first);
        let size = 0;
        while (pending.length > 0) {
            const [x, y] = pending.pop().split(",").map(Number);
            size += 1;
            minX = Math.min(minX, x);
            maxX = Math.max(maxX, x);
            minY = Math.min(minY, y);
            maxY = Math.max(maxY, y);
            for (let dy = -1; dy <= 1; dy += 1) {
                for (let dx = -1; dx <= 1; dx += 1) {
                    if (!dx && !dy) continue;
                    const neighbor = `${x + dx},${y + dy}`;
                    if (remaining.delete(neighbor)) pending.push(neighbor);
                }
            }
        }
        if (size >= 2) components += 1;
    }
    return { width: maxX - minX + 1, height: maxY - minY + 1, components };
}

async function judgeCam2CursorPixels(view, captures, decoded) {
    const first = captures.find((capture) => capture.name === "joined-cursor");
    const last = captures.find((capture) => capture.name === "edge-move");
    assert(first && last, "CAM2 cursor pixels need actual acquired-lock and later edge captures");
    for (const capture of [first, last]) {
        const root = verifiedRoot(capture.checkpoint.debug, "CAM2");
        assert.equal(root.id, "forge", "CAM2 must use its actual declared Forge package");
    }
    const archive = await view.serverBytes("pack/forge.tar");
    const cursors = readAuthoredCursorAssets(archive, decodePng);
    const pixels = measureCursorPixels(first, last, decoded.get(first.name), decoded.get(last.name), cursors);
    for (const stage of pixels.stages) {
        requireProduct(stage.positive.total >= 80 && stage.positive.ratio >= 0.90,
            "CAM2", "cursor-pixels", `actual authored ${stage.image_id} cursor glyph is absent or misplaced (${stage.positive.matched}/${stage.positive.total})`);
        requireProduct(stage.nearby.total >= 80 && stage.nearby.ratio <= 0.35,
            "CAM2", "cursor-pixels", "actual nearby region does not establish cursor glyph contrast");
    }
    requireProduct(pixels.old_region_absence.total >= 80 && pixels.old_region_absence.ratio <= 0.35,
        "CAM2", "cursor-pixels", "the old actual cursor region still contains its glyph after physical motion");
    return { archive_sha256: crypto.createHash("sha256").update(archive).digest("hex"), ...pixels };
}

function judgeCam2(input, captures) {
    const f10s = input.map((entry, index) => ({ entry, index }))
        .filter(({ entry }) => entry.call === "key" && entry.args?.[0] === "F10" &&
            JSON.stringify(entry.args?.[1] || []) === "[]");
    assert(f10s.length >= 2, "CAM2 needs physical F10 menu-open and menu-close inputs");
    const [open, close] = f10s;
    const f10Down = open.entry.actual_events?.find((event) => event.type === "keydown" && event.code === "F10");
    const closeDown = close.entry.actual_events?.find((event) => event.type === "keydown" && event.code === "F10");
    assert(f10Down && Number.isSafeInteger(f10Down.seq), "CAM2 F10 open lacks an actual keydown sequence");
    assert(closeDown && Number.isSafeInteger(closeDown.seq), "CAM2 F10 close lacks an actual keydown sequence");
    const guideClick = input.findIndex((entry, index) => index < open.index && entry.call === "click" &&
        entry.args?.[2] === "left" && entry.actual_events?.some((event) => event.type === "pointerdown" && Number.isSafeInteger(event.seq)));
    assert(guideClick >= 0, "CAM2 guide dismissal needs a real physical canvas click before F10");
    const guideF1 = input.findIndex((entry, index) => index > guideClick && index < open.index &&
        entry.call === "key" && entry.args?.[0] === "F1" && entry.actual_events?.some((event) => event.type === "keydown" && event.code === "F1" && Number.isSafeInteger(event.seq)));
    assert(guideF1 > guideClick, "CAM2 guide dismissal needs its actual physical F1 keydown after the opening click");
    const guideCapture = captures.find((capture) => capture.name === "world-ready" &&
        capture.input_call > guideF1 && capture.input_call < open.index && capture.window === open.entry.window &&
        capture.document_epoch === open.entry.document_epoch);
    assert(guideCapture, "CAM2 requires a same-document physical guide-dismissed setup capture before F10");
    const guideRows = guideCapture.checkpoint.debug.ui_snapshot;
    assert(Array.isArray(guideRows), "CAM2 guide-dismissed setup capture lacks its actual UiSnapshotFeed");
    const guideRoot = guideRows.find((row) => row.kind === "widget" && row.id === "guide");
    assert(guideRoot, "CAM2 guide-dismissed setup capture lacks the actual guide root receipt");
    assert.equal(guideRoot.visible, false, "CAM2 setup capture must show the tutorial guide hidden before gameplay input");
    const wheelIndex = input.findIndex((entry, index) => index > open.index && index < close.index && entry.call === "wheel");
    const wheel = wheelIndex >= 0 ? input[wheelIndex] : null;
    assert(wheel && wheel.actual_events?.some((event) => event.type === "wheel" && Number.isSafeInteger(event.seq)),
        "CAM2 bottom-menu capture lacks its actual physical wheel receipt");
    const topCapture = captures.find((capture) => capture.name === "settings-menu-top" &&
        capture.input_call > open.index && capture.input_call <= wheelIndex &&
        capture.window === open.entry.window && capture.document_epoch === open.entry.document_epoch);
    assert(topCapture, "CAM2 top settings-menu capture is missing before the physical scroll");
    const topRows = topCapture.checkpoint.debug.ui_snapshot;
    assert(Array.isArray(topRows), "CAM2 top menu capture lacks the actual UiSnapshotFeed");
    const topMenus = topRows.filter((row) => row.kind === "settings_menu" && row.id === "settings_menu");
    assert.equal(topMenus.length, 1, "CAM2 top settings menu root is absent or duplicated");
    const topMenu = topMenus[0];
    requireProduct(topMenu.visible === true && topMenu.capture_root === true && topMenu.focus_policy_block === true &&
        topMenu.text_nodes?.some((node) => node.text === "Settings" && node.visible === true &&
            Array.isArray(node.rect) && node.rect.length === 4 && node.rect.every(Number.isFinite) && node.rect[2] > 0 && node.rect[3] > 0),
    "CAM2", "f10-menu", "physical F10 did not present the actual laid-out settings menu heading");
    const menuToggleIndex = input.findIndex((entry, index) => index > wheelIndex && index < close.index &&
        entry.call === "click" && entry.args?.[2] === "left");
    const menuToggle = menuToggleIndex >= 0 ? input[menuToggleIndex] : null;
    assert(menuToggle && menuToggle.actual_events?.some((event) => event.type === "pointerdown" && Number.isSafeInteger(event.seq)),
        "CAM2 cursor opt-in lacks its actual physical pointerdown receipt");
    const menuCapture = captures.find((capture) => capture.name === "settings-menu" &&
        capture.input_call > menuToggleIndex && capture.input_call <= close.index &&
        capture.window === open.entry.window && capture.document_epoch === open.entry.document_epoch &&
        retainedFrames(capture.checkpoint.debug).some((frame) => frame.applied_seq?.includes(f10Down.seq) &&
            frame.named_dispatches?.some((dispatch) => dispatch.origin === "key" && dispatch.code === "F10" &&
                dispatch.matched_action === "menu")));
    assert(menuCapture, "CAM2 F10 menu capture is not bound to the actual named Core menu dispatch");
    const rows = menuCapture.checkpoint.debug.ui_snapshot;
    assert(Array.isArray(rows), "CAM2 menu capture lacks the actual UiSnapshotFeed");
    assert(!rows.some((row) => row.kind === "settings_menu_overflow"), "CAM2 settings menu snapshot overflowed");
    const menus = rows.filter((row) => row.kind === "settings_menu" && row.id === "settings_menu");
    assert.equal(menus.length, 1, "CAM2 settings menu root is absent or duplicated");
    const menu = menus[0];
    requireProduct(menu.visible === true && menu.capture_root === true && menu.focus_policy_block === true &&
        Array.isArray(menu.rect) && menu.rect.length === 4 && menu.rect.every(Number.isFinite) && menu.rect[2] > 0 && menu.rect[3] > 0,
    "CAM2", "f10-menu", "physical F10 did not present the actual laid-out settings menu");
    assert(menu.descendants_truncated === false && menu.text_truncated === false && Array.isArray(menu.text_nodes),
        "CAM2 menu label inventory is truncated or missing");
    const lockLabel = menu.text_nodes.find((node) => node.text === "Keep the cursor in the game" && node.visible === true &&
        Array.isArray(node.rect) && node.rect.length === 4 && node.rect.every(Number.isFinite) && node.rect[2] > 0 && node.rect[3] > 0);
    requireProduct(Boolean(lockLabel), "CAM2", "pointer-lock",
        "the actual settings-menu tree has no visible Keep the cursor in the game opt-in");
    const lockOn = menu.text_nodes.some((node) => node.text === "On" && node.visible === true &&
        Array.isArray(node.rect) && node.rect.length === 4 && node.rect.every(Number.isFinite) &&
        node.rect[2] > 0 && node.rect[3] > 0);
    requireProduct(lockOn, "CAM2", "pointer-lock", "the physical cursor opt-in did not leave the actual setting enabled");

    const canvasClickIndex = input.findIndex((entry, index) => index > close.index && entry.call === "click" &&
        entry.args?.[2] === "left");
    const canvasClick = canvasClickIndex >= 0 ? input[canvasClickIndex] : null;
    assert(canvasClick && canvasClick.actual_events?.some((event) => event.type === "pointerdown" && Number.isSafeInteger(event.seq)),
        "CAM2 lock acquisition requires a real canvas click after closing the settings menu");
    const joined = captures.find((capture) => capture.name === "joined-cursor" &&
        capture.input_call > canvasClickIndex && capture.window === canvasClick.window &&
        capture.document_epoch === canvasClick.document_epoch);
    assert(joined, "CAM2 actual lock capture is missing after the canvas click");
    const joinedDebug = joined.checkpoint.debug;
    requireProduct(joined.checkpoint.browser?.canvas_pointer_locked === true && joinedDebug.cursor?.lock_requested === true &&
        joinedDebug.cursor?.lock_active === true,
    "CAM2", "pointer-lock", "the opted-in physical canvas click did not acquire actual browser pointer lock");

    const moves = input.map((entry, index) => ({ entry, index })).filter(({ entry, index }) =>
        index > canvasClickIndex && entry.call === "move");
    assert(moves.length >= 2 && moves.every(({ entry }) => entry.actual_events?.some((event) =>
        event.type === "pointermove" && Number.isSafeInteger(event.seq))),
    "CAM2 requires two real browser pointer-motion receipts after actual pointer lock");
    const [centerMove, edgeMove] = moves;
    assert.deepEqual(centerMove.entry.args, [700, 360], "CAM2 first locked motion must use the fixed interior calibration point");
    assert.deepEqual(edgeMove.entry.args, [1, 360], "CAM2 edge motion must use the fixed logical left-edge point");
    assert(centerMove.index >= joined.input_call,
        "CAM2 must capture actual pointer-lock acquisition before sending relative movement");
    const motionCapture = captures.find((capture) => capture.name === "summed-motion" &&
        capture.input_call > centerMove.index && capture.window === joined.window &&
        capture.document_epoch === joined.document_epoch);
    assert(motionCapture, "CAM2 summed-motion capture is missing after the first locked move");
    const motionCursor = motionCapture.checkpoint.debug.cursor;
    assert(motionCursor && Number.isSafeInteger(motionCursor.frame_index) &&
        Array.isArray(motionCursor.engine_position) && motionCursor.engine_position.length === 2 &&
        motionCursor.engine_position.every(Number.isFinite) && Array.isArray(motionCursor.relative_motion) &&
        motionCursor.relative_motion.length === 2 && motionCursor.relative_motion.every(Number.isFinite),
    "CAM2 real cursor producer lacks a bounded position, relative-motion pair, and source frame");
    const joinedPosition = joinedDebug.cursor.engine_position;
    const moved = Math.hypot(motionCursor.engine_position[0] - joinedPosition[0],
        motionCursor.engine_position[1] - joinedPosition[1]);
    const centerEvent = centerMove.entry.actual_events.find((event) => event.type === "pointermove");
    const centerDelta = Math.hypot(centerEvent?.movement_x ?? NaN, centerEvent?.movement_y ?? NaN);
    requireProduct(motionCursor.lock_requested === true && motionCursor.lock_active === true &&
        motionCursor.frame_index > joinedDebug.cursor.frame_index &&
        centerEvent?.canvas_pointer_locked === true && centerDelta > 0 && moved > 1,
    "CAM2", "summed-motion-cursor", "the locked engine cursor did not follow real summed relative motion");

    const edgeCapture = captures.find((capture) => capture.name === "edge-move" &&
        capture.input_call > edgeMove.index && capture.window === joined.window &&
        capture.document_epoch === joined.document_epoch);
    assert(edgeCapture, "CAM2 edge-move capture is missing after the second locked move");
    const edgeCursor = edgeCapture.checkpoint.debug.cursor;
    assert(edgeCursor && Number.isSafeInteger(edgeCursor.frame_index) &&
        Array.isArray(edgeCursor.engine_position) && edgeCursor.engine_position.length === 2 &&
        edgeCursor.engine_position.every(Number.isFinite),
    "CAM2 edge capture lacks actual cursor producer position and frame identity");
    const edgeEvent = edgeMove.entry.actual_events.find((event) => event.type === "pointermove");
    const edgeDelta = Math.hypot(edgeEvent?.movement_x ?? NaN, edgeEvent?.movement_y ?? NaN);
    requireProduct(edgeEvent?.canvas_pointer_locked === true && edgeDelta > 0 && edgeCursor.lock_active === true && edgeCursor.engine_position[0] <= 4 &&
        edgeCursor.engine_position[1] >= 340 && edgeCursor.engine_position[1] <= 380 &&
        edgeCursor.frame_index > motionCursor.frame_index,
    "CAM2", "edge-band-pan", "the real locked cursor did not reach the logical left-edge band");
    const motionMatrix = motionCapture.checkpoint.scene?.clip_from_world;
    const edgeMatrix = edgeCapture.checkpoint.scene?.clip_from_world;
    assert(Array.isArray(motionMatrix) && motionMatrix.length === 16 && motionMatrix.every(Number.isFinite) &&
        Array.isArray(edgeMatrix) && edgeMatrix.length === 16 && edgeMatrix.every(Number.isFinite),
    "CAM2 edge-pan captures lack the actual scene projection matrices");
    const cameraDelta = motionMatrix.reduce((sum, value, index) => sum + Math.abs(value - edgeMatrix[index]), 0);
    requireProduct(cameraDelta > 1e-5, "CAM2", "edge-band-pan",
        "the actual camera projection did not change while the locked cursor moved into the edge band");
    return new Set(["pointer-lock", "summed-motion-cursor", "edge-band-pan", "f10-menu"]);
}

async function judgePer1Storage(view, input, captures, params, decoded) {
    const audit = JSON.parse(await view.serverFile("storage-audit.json"));
    const fixture = JSON.parse(await view.serverFile("per1-fixture-private.json"));
    const initialState = audit.initial_state?.find((state) => state.window === 1);
    const finalState = audit.final_state?.find((state) => state.window === 1);
    const lastCapture = captures.filter((capture) => capture.window === 1).at(-1);
    assert(initialState && finalState && lastCapture, "PER1 requires actual current-window initial/final states and capture");
    assert.equal(finalState.document_epoch, lastCapture.document_epoch, "PER1 final state must match the current captured document");
    const debug = lastCapture.checkpoint.debug;
    verifiedRoot(debug, "PER1");

    // Browser fixtures and product predicates run before the optional native artifact is even read.
    // Their setup/data errors are generic; actual product failures carry task16_semantic metadata.
    const expectedFixtureId = params.task16?.fixture;
    const evaluated = evaluatePer1BrowserEvidence({
        audit, fixture, debug, initialDebug: initialState.debug,
        expectedFixtureId, input, captures,
    });

    const qualifiedNative = params.per1_native_reference;
    assert.equal(qualifiedNative?.validated_before_physical_run, true,
        "PER1 native capsule was not qualified by the harness before physical input");
    assert.deepEqual(Object.keys(qualifiedNative.files || {}).sort(), ["build.log", "launcher.json",
        "native-path-audit.json", "native-process.log", "native-raw.json", "source-after.json", "source-before.json"].sort(),
        "PER1 native capsule file set differs from the generated harness manifest");
    for (const [file, expectedSha] of Object.entries(qualifiedNative.files)) {
        assert.match(expectedSha, /^[a-f0-9]{64}$/, "PER1 generated native manifest hash is invalid");
        const bytes = await view.serverBytes(`native-reference/${file}`);
        assert.equal(crypto.createHash("sha256").update(bytes).digest("hex"), expectedSha,
            `PER1 native capsule ${file} differs from harness-qualified bytes`);
    }
    const nativeRaw = await view.serverFile("native-reference/native-path-audit.json");
    const native = validateOptionalNativePathDocument(nativeRaw);
    const nativeProcessRaw = await view.serverFile("native-reference/native-raw.json");
    const nativeProcess = JSON.parse(nativeProcessRaw);
    const launcherRaw = await view.serverFile("native-reference/launcher.json");
    const launcher = JSON.parse(launcherRaw);
    const nativeLog = await view.serverFile("native-reference/native-process.log");
    const sourceBefore = await view.serverFile("native-reference/source-before.json");
    const sourceAfter = await view.serverFile("native-reference/source-after.json");
    const sha = (value) => crypto.createHash("sha256").update(value).digest("hex");
    assert.equal(sourceBefore, sourceAfter, "PER1 native source changed during execution");
    assert.equal(native.process.source_sha256, sha(sourceBefore), "PER1 native source bytes differ");
    assert.equal(native.process.runner_receipt_sha256, sha(launcherRaw), "PER1 native launcher bytes differ");
    assert.equal(native.provenance.raw_receipt_sha256, sha(nativeProcessRaw), "PER1 native raw process bytes differ");
    assert.equal(launcher.log_sha256, sha(nativeLog), "PER1 actual native log bytes differ");
    assert.equal(launcher.exit_code, 0, "PER1 actual native process failed");
    assert.equal(launcher.native_pid, native.process.pid, "PER1 native PID differs from observed child");
    assert.deepEqual(launcher.argv, native.process.argv, "PER1 actual native argument list differs");
    assert.equal(launcher.executable_sha256, native.process.executable_sha256, "PER1 actual native binary differs");
    assert(nativeLog.includes("test per1_native_paths ... ok") && nativeLog.includes("1 passed; 0 failed; 0 ignored;"),
        "PER1 named native process did not pass exactly once");
    const reboundRaw = structuredClone(nativeProcess);
    reboundRaw.schema = "per1-native-path-audit-v2";
    Object.assign(reboundRaw.process, { exit_code: launcher.exit_code, source_sha256: sha(sourceBefore),
        runner_receipt_sha256: sha(launcherRaw) });
    reboundRaw.provenance = { raw_receipt_sha256: sha(nativeProcessRaw),
        operation_rows: "emitted by actual test process; preserved without synthesis" };
    assert.deepEqual(native, reboundRaw, "PER1 audit altered actual native process operation rows");
    evaluated.add("native-path-names");
    judgePer1NoticePixels(debug, captures, decoded);
    evaluated.add("notice-pixels");
    return evaluated;
}

function per1GoldPixels(image, region) {
        const [x, y, width, height] = region;
        assert(x >= 0 && y >= 0 && width > 0 && height > 0 && x + width <= image.width && y + height <= image.height,
            "PER1 notice pixel region falls outside actual screenshot");
        const points = [];
        for (let py = y; py < y + height; py += 1) {
            for (let px = x; px < x + width; px += 1) {
                const at = (py * image.width + px) * 4;
                const [r, g, b, a] = image.rgba.subarray(at, at + 4);
                if (r > 180 && g > 140 && b < 65 && r > g * 1.05 && a >= 250) points.push([px, py]);
            }
        }
        return points;
}

function judgePer1NoticePixels(debug, captures, decoded) {
    const final = captures.find((capture) => ["settings-and-notice-final", "binding-defaults-notice-final"].includes(capture.name));
    const notice = debug.ui_snapshot.filter((node) => node.id === "last_notice" && node.visible === true);
    assert.equal(notice.length, 1, "PER1 actual notice rectangle is absent or duplicated");
    const rect = notice[0].rect;
    const image = decoded.get(final.name);
    assert(image && image.width === 1280 && image.height === 720 && Array.isArray(rect) &&
        rect.length === 4 && rect.every(Number.isInteger), "PER1 notice pixel coordinates are incomplete");
    const points = per1GoldPixels(image, rect);
    const glyph = pixelGlyphShape(points);
    const adjacent = [rect[0], rect[1] + rect[3] + 2, rect[2], rect[3]];
    const adjacentPoints = per1GoldPixels(image, adjacent);
    const unrelatedTextPoints = per1GoldPixels(image, [8, 574, 272, 52]);
    requireProduct(points.length >= 650 && glyph.width >= 270 && glyph.height >= 10 && glyph.components >= 25 &&
        adjacentPoints.length <= 15 && unrelatedTextPoints.length <= 15,
        "PER1", "notice-pixels",
        `actual corruption notice lacks distributed golden glyphs or matched absence controls (gold=${points.length}, span=${glyph.width}x${glyph.height}, components=${glyph.components}, adjacent=${adjacentPoints.length}, other=${unrelatedTextPoints.length})`);
    return { points: points.length, glyph, adjacent: adjacentPoints.length, unrelatedText: unrelatedTextPoints.length };
}

function assertRequiredPhysicalReceipts(row, input, captures) {
    const physical = new Set(ROW_PHYSICAL_CHECKS[row]);
    const expected = {
        KM1: { "keyq-bound-action": { call: "key", code: "KeyQ", key: "a" } },
        KM2: { "contextmenu-prevented-delta": { call: "click", button: "right" }, "outside-key-stopped": { call: "key" } },
        SEL4: {
            "ctrl-1-assign": { call: "key", code: "Digit1", modifiers: ["Control"] },
            "1-recall": { call: "key", code: "Digit1", modifiers: [] },
            "shift-1-add": { call: "key", code: "Digit1", modifiers: ["Shift"] },
            "double-tap-centre": { call: "key", code: "Digit1", modifiers: [] },
        },
        SLOT1: {
            "unclaimed-third-refusal": { call: "use", window: 3, capture_name: "unclaimed-C-refusal", phase: "capacity" },
            "copied-tab-terminal-refusal": { call: "use", window: 3 },
            "reload-same-id": { call: "reload", window: 1 },
        },
        CAM2: { "pointer-lock": { call: "click" }, "summed-motion-cursor": { call: "move" }, "edge-band-pan": { call: "move" }, "f10-menu": { call: "key", code: "F10" } },
        SH1: { "menu-controls": { call: "key", code: "F10", modifiers: [] }, "conflict-swap-cancel-reset": { call: "click" }, "cursor-lock": { call: "click" }, "connection-lost-reload": { call: "reload" } },
        PER1: { "browser-key-names": { call: "click" }, "defaults-and-single-notice": { call: "click" } },
    }[row];
    for (const id of physical) {
        const contract = expected[id];
        assert(contract, `${row} ${id} has no harness-owned physical receipt contract`);
        let capture;
        if (contract.capture_name !== undefined) {
            capture = captures.find((item) => item.name === contract.capture_name);
            assert(capture, `${row} ${id} has no matching retained capture ${contract.capture_name}`);
            assert.equal(capture.phase, contract.phase, `${row} ${id} capture belongs to the wrong physical phase`);
            assert.equal(capture.window, contract.window, `${row} ${id} capture belongs to the wrong window`);
        }
        const receiptIndexes = input.flatMap((entry, index) => entry.call === contract.call &&
            (contract.window === undefined || entry.window === contract.window) &&
            (capture === undefined || (index < capture.screenshot_input_call &&
                entry.document_epoch === capture.document_epoch)) &&
            (contract.code === undefined || (entry.args?.[0] === contract.code && entry.actual_events?.some((event) => event.code === contract.code))) &&
            (contract.key === undefined || entry.actual_events?.some((event) => event.key === contract.key)) &&
            (contract.modifiers === undefined || JSON.stringify(entry.args?.[1] || []) === JSON.stringify(contract.modifiers)) &&
            (contract.button === undefined || entry.args?.[2] === contract.button) &&
            (contract.call !== "key" || contract.code !== undefined || entry.actual_events?.some((event) => event.type === "keydown")) ? [index] : []);
        const receiptIndex = receiptIndexes[0];
        const receipt = receiptIndex === undefined ? undefined : input[receiptIndex];
        assert(receipt, `${row} ${id} required physical ${contract.call} receipt is missing`);
        if (capture !== undefined) {
            assert(receiptIndex < capture.screenshot_input_call,
                `${row} ${id} physical ${contract.call} receipt must precede its matching capture`);
            assert.equal(receipt.document_epoch, capture.document_epoch,
                `${row} ${id} physical ${contract.call} receipt and matching capture must share the actual document epoch`);
        }
        if (id === "contextmenu-prevented-delta") {
            const events = receipt.contextmenu_events || [];
            assert(events.length > 0, `${row} ${id} has no physical contextmenu event`);
            assert.equal(receipt.contextmenu_overflow, false, `${row} ${id} contextmenu observer overflowed its bounded receipt buffer`);
            const callIndex = input.indexOf(receipt);
            assert(events.every((event) => event.source_call_index === callIndex && event.canvas_target === true &&
                event.target === "CANVAS" && event.cancelable === true && typeof event.default_prevented === "boolean" &&
                (event.seq === null || receipt.event_seq.includes(event.seq)) &&
                event.wall_time_ms >= receipt.wall_time_ms && event.wall_time_ms <= receipt.completed_wall_time_ms),
            `${row} ${id} receipts must bind actual canvas target, sequence/null, cancelable state, source click and deferred defaultPrevented read`);
        }
    }
}

function assertSlotObserverRecordShapes(records) {
    const exactU64 = (value) => typeof value === "string" && /^(0|[1-9]\d{0,19})$/.test(value) &&
        BigInt(value) <= 18_446_744_073_709_551_615n;
    assert(records.length > 0 && records[0].event === "boot_start" && records[0].sequence === 1,
        "SLOT1 current observer stream must begin with its actual boot_start record");
    for (const [index, record] of records.entries()) {
        assert(Number.isSafeInteger(record.sequence) && record.sequence === index + 1,
            "SLOT1 actual observer stream must be contiguous from sequence one");
        assert(typeof record.run_nonce === "string" && record.run_nonce.length > 0 &&
            Number.isSafeInteger(record.server_process_id) && record.server_process_id > 0 &&
            record.run_nonce.startsWith(`${record.server_process_id}-`) &&
            Number.isSafeInteger(record.host_unix_us) && Number.isSafeInteger(record.server_monotonic_us),
        "SLOT1 actual observer record lacks its process nonce or clock anchors");
        if (index === 0) {
            assert.equal(record.event, "boot_start");
            assert(!Object.hasOwn(record, "entity_bits_with_generation_decimal") &&
                !Object.hasOwn(record, "connection_epoch_decimal") && !Object.hasOwn(record, "match_id_decimal"),
            "SLOT1 boot_start must not invent connection-owner identity fields");
            continue;
        }
        assert(typeof record.wall_vs_monotonic_delta_us === "string" && /^-?\d+$/.test(record.wall_vs_monotonic_delta_us) &&
            record.clock_correlation_error_bound_us === 250000 && record.clock_jump_detected === false,
        "SLOT1 actual observer record needs signed clock correlation, its bound, and a healthy clock");
        if (record.event === "identity_alias") {
            assert(Number.isSafeInteger(record.identity_alias) && exactU64(record.player_id_decimal),
                "SLOT1 identity alias map must retain the actual exact player decimal ID");
            continue;
        }
        if (record.event === "owner_checkpoint") {
            assert(Array.isArray(record.owners) && Array.isArray(record.active_epochs) && exactU64(record.match_id_decimal),
                "SLOT1 owner checkpoint must match the actual bounded checkpoint producer schema");
            assert(!Object.hasOwn(record, "entity_bits_with_generation_decimal") &&
                !Object.hasOwn(record, "connection_epoch_decimal"),
            "SLOT1 checkpoint identity must come from actual active_epochs, not invented top-level IDs");
            assert(record.active_epochs.every((active) => exactU64(active.entity_bits_with_generation_decimal) &&
                exactU64(active.connection_epoch_decimal) && exactU64(active.match_id_decimal) &&
                Number.isSafeInteger(active.identity_alias) && Number.isInteger(active.slot)),
            "SLOT1 checkpoint active epoch rows must match actual Session producer fields");
            continue;
        }
        if (["hello_decision", "session_added", "session_authority_removed"].includes(record.event)) {
            assert(exactU64(record.entity_bits_with_generation_decimal) &&
                exactU64(record.match_id_decimal) && exactU64(record.connection_epoch_decimal) &&
                Number.isSafeInteger(record.identity_alias) &&
                (record.slot === null || Number.isInteger(record.slot)),
            `SLOT1 ${record.event} must match the actual connection lifecycle producer schema`);
            continue;
        }
        assert.fail(`SLOT1 private observer emitted unsupported record type ${String(record.event)}`);
    }
}

async function judgeSlot1Evidence(view, input, captures, captureByName) {
    const fixture = JSON.parse(await view.serverFile("slot1-fixture-private.json"));
    assert.equal(fixture.fixture, "slot1-authentic-copy-v1");
    assert.equal(fixture.simultaneous_window_limit, 3);
    assert.equal(fixture.total_lifetime_document_count, 4, "SLOT1 must retain C and the replacement copied A document");
    assert.deepEqual(fixture.phases.map((phase) => phase.phase), ["capacity", "copy-refusal", "reload-rejoin", "complete"]);
    assert.equal(fixture.lifetime_documents.length, 4);
    const initialDocuments = fixture.lifetime_documents.slice(0, 3);
    assert.deepEqual(initialDocuments.map((document) => document.alias), ["A", "B", "C"]);
    assert(initialDocuments.every((document) => Number.isSafeInteger(document.epoch) && document.epoch > 0));
    const copiedDocument = fixture.lifetime_documents[3];
    assert.equal(copiedDocument.handle, 3);
    assert.equal(copiedDocument.alias, "A");
    assert(copiedDocument.epoch > initialDocuments[2].epoch, "SLOT1 replacement must advance the handle 3 document epoch");
    const copyPhase = fixture.phases[1];
    assert.equal(copyPhase.old_document_epoch, initialDocuments[2].epoch);
    assert.equal(copyPhase.new_document_epoch, copiedDocument.epoch);
    assert.notEqual(copyPhase.old_document_epoch, copyPhase.new_document_epoch);
    assert(fixture.startup?.[0]?.phase?.startsWith("Joined") && fixture.startup[0].last_frame && fixture.startup[0].cameras_3d === 1,
        "SLOT1 A ownership must be established before B/C startup");
    assert(fixture.startup?.[1]?.phase?.startsWith("Joined") && fixture.startup[1].last_frame && fixture.startup[1].cameras_3d === 1,
        "SLOT1 B ownership must be established before C startup");
    requireProduct(fixture.startup?.[2]?.phase?.startsWith("Failed"), "SLOT1", "unclaimed-third-refusal",
        "ordinary unclaimed C did not receive the actual capacity refusal");
    assert.equal(fixture.phases[1].browser_opener_copy, true, "W3 must be an opener-created sessionStorage copy");
    assert.equal(fixture.phases[1].storage_copy_equal, true);
    assert(fixture.startup.every((status, index) => status.window === index + 1 && status.document_epoch === initialDocuments[index].epoch),
        "SLOT1 startup receipts must name their actual initial document epochs");
    assert.equal(copyPhase.startup.window, 3);
    assert.equal(copyPhase.startup.document_epoch, copiedDocument.epoch);
    const capacityRefusal = captures.find((capture) => capture.phase === "capacity" && capture.window === 3);
    assert(capacityRefusal, "SLOT1 lacks the named physical capture of C's result");
    requireProduct(capacityRefusal.checkpoint.debug.phase?.startsWith("Failed"), "SLOT1", "unclaimed-third-refusal",
        "the actual C capture did not show the terminal capacity refusal");
    requireProduct(capacityRefusal.checkpoint.debug.player_id_decimal === fixture.identity_aliases.C,
        "SLOT1", "unclaimed-third-refusal", "C's refusal capture does not retain C's actual distinct player ID");
    const copiedTerminal = captureByName.get("clone-terminal");
    assert(copiedTerminal, "SLOT1 lacks the named physical copied-tab terminal capture");
    requireProduct(copiedTerminal.window === 3 && copiedTerminal.phase === "copy-refusal" &&
        copiedTerminal.checkpoint.debug.phase?.startsWith("Failed"), "SLOT1", "copied-tab-terminal-refusal",
    "the actual copied live A document was not refused in its named terminal capture");
    const terminalPng = decodePng(await view.serverBytes(copiedTerminal.file));
    assertSlot1TerminalPixels(copiedTerminal, terminalPng);

    const identities = fixture.identity_aliases;
    assert(typeof identities.A === "string" && typeof identities.B === "string" && typeof identities.C === "string");
    assert(BigInt(identities.A) > 9007199254740991n, "SLOT1 A must exercise exact decimal identity above 2^53");
    assert.equal(new Set([identities.A, identities.B, identities.C]).size, 3, "SLOT1 A, B, and C IDs must be distinct");
    const text = await view.serverFile("server-stats.json.sessions.jsonl");
    const allRecords = text.split(/\r?\n/).filter(Boolean).map((line) => JSON.parse(line));
    const latestBoot = allRecords.filter((record) => record.event === "boot_start").at(-1);
    assert(latestBoot && latestBoot.sequence === 1, "SLOT1 observer has no complete current boot start record");
    const records = allRecords.filter((record) => record.run_nonce === latestBoot.run_nonce);
    assert(records.length > 0, "SLOT1 private server observer has no records for the current boot");
    assertSlotObserverRecordShapes(records);
    const launch = JSON.parse(await view.serverFile("server-launch.json"));
    assert(Number.isSafeInteger(launch.server_pid) && launch.server_pid === latestBoot.server_process_id,
        "SLOT1 actual boot nonce must bind to the server process launched by the harness");
    const statsArgIndex = launch.argv?.indexOf("--stats") ?? -1;
    assert(Array.isArray(launch.argv) && statsArgIndex >= 0 && launch.argv[statsArgIndex + 1] === launch.stats_arg &&
        `${path.resolve(launch.stats_arg)}.sessions.jsonl` === launch.observer_path,
    "SLOT1 observer launch must retain the actual server argv and stats sibling path");
    const argvHash = crypto.createHash("sha256").update(JSON.stringify(launch.argv)).digest("hex");
    assert.equal(launch.argv_sha256, argvHash, "SLOT1 launch argv hash does not match its actual command");
    assert(records.every((record) => record.server_process_id === launch.server_pid && record.run_nonce === latestBoot.run_nonce),
        "SLOT1 selected observer boot must retain one exact launched process and nonce");
    const aliasesById = new Map(records.filter((record) => record.event === "identity_alias")
        .map((record) => [record.player_id_decimal, record.identity_alias]));
    const aliasA = aliasesById.get(identities.A);
    const aliasB = aliasesById.get(identities.B);
    const aliasC = aliasesById.get(identities.C);
    assert([aliasA, aliasB, aliasC].every(Number.isSafeInteger), "SLOT1 server observer must map actual IDs to private aliases");
    assert.equal(new Set([aliasA, aliasB, aliasC]).size, 3, "SLOT1 server aliases must be distinct");
    assert(records.every((record, index) => record.sequence === index + 1), "SLOT1 current-boot observer sequence must be contiguous from its boot start");
    assert(records.every((record) => record.clock_jump_detected === false), "SLOT1 elapsed-time correlation cannot cross a detected wall-clock jump");
    assert(records.filter((record) => record.event !== "identity_alias").every((record) => !Object.hasOwn(record, "player_id_decimal")),
        "raw player IDs may appear only in the mode-0600 alias map");
    const observedLifecycle = records.filter((record) =>
        ["hello_decision", "session_added", "session_authority_removed"].includes(record.event));
    const exactU64 = (value) => typeof value === "string" && /^(0|[1-9]\d{0,19})$/.test(value) && BigInt(value) <= 18_446_744_073_709_551_615n;
    assert(observedLifecycle.every((record) => exactU64(record.entity_bits_with_generation_decimal) &&
        exactU64(record.match_id_decimal) && exactU64(record.connection_epoch_decimal)),
    "SLOT1 private entity, match, and connection epoch identities must remain exact decimal u64 text");
    for (const checkpoint of records.filter((record) => record.event === "owner_checkpoint")) {
        assert(Array.isArray(checkpoint.owners) && Array.isArray(checkpoint.active_epochs),
            "SLOT1 owner checkpoint must retain actual owners and active Session epochs");
        for (const active of checkpoint.active_epochs) {
            assert(exactU64(active.entity_bits_with_generation_decimal) &&
                exactU64(active.match_id_decimal) && exactU64(active.connection_epoch_decimal),
            "SLOT1 checkpoint Session identity fields must be exact decimal u64 strings");
        }
    }
    assert(records.slice(1).every((record) => typeof record.wall_vs_monotonic_delta_us === "string" &&
        /^-?\d+$/.test(record.wall_vs_monotonic_delta_us) && record.clock_correlation_error_bound_us === 250000),
    "SLOT1 private observer must retain signed clock correlation and its explicit error bound");
    for (let index = 1; index < records.length; index += 1) {
        const previous = records[index - 1];
        const current = records[index];
        const wallDelta = BigInt(current.host_unix_us) - BigInt(previous.host_unix_us);
        const monotonicDelta = BigInt(current.server_monotonic_us) - BigInt(previous.server_monotonic_us);
        const correlationDelta = wallDelta - monotonicDelta;
        const actualDelta = BigInt(current.wall_vs_monotonic_delta_us);
        assert.equal(actualDelta, correlationDelta,
            "SLOT1 signed wall/monotonic correlation must match actual adjacent producer clock readings");
        assert.equal(current.clock_jump_detected, (actualDelta < 0n ? -actualDelta : actualDelta) > 250000n,
            "SLOT1 clock jump flag must match the source's explicit 250ms correlation bound");
    }
    assert(records.every((record, index) => index === 0 ||
        (record.host_unix_us >= records[index - 1].host_unix_us && record.server_monotonic_us >= records[index - 1].server_monotonic_us)),
    "SLOT1 server wall and monotonic clocks must both advance across the contiguous evidence stream");

    const hello = records.filter((record) => record.event === "hello_decision");
    const sessionAdded = records.filter((record) => record.event === "session_added");
    const removed = records.filter((record) => record.event === "session_authority_removed");
    const acceptedA = hello.find((record) => record.identity_alias === aliasA && record.decision?.accepted === true);
    const acceptedB = hello.find((record) => record.identity_alias === aliasB && record.decision?.accepted === true);
    requireProduct(Boolean(acceptedA && acceptedB), "SLOT1", "owners-stable", "actual A and B Hellos did not both establish accepted sessions");
    assert.equal(acceptedB.match_id_decimal, acceptedA.match_id_decimal, "SLOT1 original owners must share the same actual match");
    for (const record of records) {
        if (Object.hasOwn(record, "match_id_decimal")) {
            assert.equal(record.match_id_decimal, acceptedA.match_id_decimal,
                "SLOT1 every current-boot lifecycle/checkpoint must retain the server's actual fixed match");
        }
        for (const active of record.active_epochs || []) {
            assert.equal(active.match_id_decimal, acceptedA.match_id_decimal,
                "SLOT1 every observed active Session must retain the server's actual fixed match");
        }
    }
    const slotA = acceptedA.decision.slot;
    const slotB = acceptedB.decision.slot;
    requireProduct(Number.isInteger(slotA) && Number.isInteger(slotB) && slotA !== slotB,
        "SLOT1", "owners-stable", "actual A and B sessions did not claim distinct owner slots");
    assertSlot1CapacityBoundary(records, completedCaptureHostMs(capacityRefusal, input) * 1000,
        copyPhase.popup_open_started_wall_ms * 1000, [acceptedA, acceptedB]);
    assert(sessionAdded.some((record) => record.identity_alias === aliasA && record.entity_bits_with_generation_decimal === acceptedA.entity_bits_with_generation_decimal &&
        record.connection_epoch_decimal === acceptedA.connection_epoch_decimal && record.slot === slotA));
    assert(sessionAdded.some((record) => record.identity_alias === aliasB && record.entity_bits_with_generation_decimal === acceptedB.entity_bits_with_generation_decimal &&
        record.connection_epoch_decimal === acceptedB.connection_epoch_decimal && record.slot === slotB));
    const acceptedBOwners = acceptedB.owners_after || [];
    assert.equal(acceptedBOwners.find((owner) => owner.slot === slotA)?.identity_alias, aliasA,
        "SLOT1 owner validation begins after B's actual grant; A must remain in its slot");
    assert.equal(acceptedBOwners.find((owner) => owner.slot === slotB)?.identity_alias, aliasB,
        "SLOT1 B's actual grant must establish both persistent owners");

    const refusedC = hello.find((record) => record.identity_alias === aliasC && record.decision?.reason === "no open slot");
    requireProduct(Boolean(refusedC), "SLOT1", "unclaimed-third-refusal", "server did not reject unclaimed C with the actual no-open-slot decision");
    requireProduct(refusedC.slot === null, "SLOT1", "unclaimed-third-refusal", "refused C received a player slot");
    requireProduct(!sessionAdded.some((record) => record.entity_bits_with_generation_decimal === refusedC.entity_bits_with_generation_decimal &&
        record.connection_epoch_decimal === refusedC.connection_epoch_decimal), "SLOT1", "unclaimed-third-refusal",
    "refused C gained actual Session authority");
    const owners = (record) => record?.owners_after || record?.owners || [];
    const ownerAt = (snapshot, slot) => snapshot.find((owner) => owner.slot === slot)?.identity_alias ?? null;
    for (const snapshot of [refusedC.owners_before, refusedC.owners_after]) {
        assert(Array.isArray(snapshot), "SLOT1 refused C Hello must retain owners before and after the real decision");
        assert.equal(ownerAt(snapshot, slotA), aliasA);
        assert.equal(ownerAt(snapshot, slotB), aliasB);
    }

    const cloneOpenedMs = copyPhase.popup_open_started_wall_ms;
    const cloneOpenCompletedMs = copyPhase.popup_open_completed_wall_ms;
    const cloneCaptureEndMs = completedCaptureHostMs(copiedTerminal, input);
    assert(Number.isFinite(cloneOpenedMs) && Number.isFinite(cloneOpenCompletedMs) &&
        cloneOpenCompletedMs >= cloneOpenedMs && cloneCaptureEndMs >= cloneOpenCompletedMs,
    "SLOT1 copied-document phase requires actual browser-open and terminal-capture host boundaries");
    const cloneRows = hello.filter((record) => record.host_unix_us >= cloneOpenedMs * 1000 &&
        record.host_unix_us <= cloneCaptureEndMs * 1000 && record.identity_alias === aliasA && record.has_live_session === true &&
        record.decision?.accepted === false && record.decision?.reason === "This game is open in another window");
    requireProduct(cloneRows.length >= 2, "SLOT1", "copied-tab-terminal-refusal",
        "the copied live A document did not produce repeated actual duplicate-window Hello refusals");
    const cloneConnections = new Set(cloneRows.map((record) =>
        `${record.entity_bits_with_generation_decimal}:${record.connection_epoch_decimal}`));
    assert.equal(cloneConnections.size, cloneRows.length,
        "SLOT1 each retry must retain its distinct actual fresh transport identity");
    requireProduct(cloneRows.every((record) => record.entity_bits_with_generation_decimal !== acceptedA.entity_bits_with_generation_decimal &&
        record.connection_epoch_decimal !== acceptedA.connection_epoch_decimal), "SLOT1", "copied-tab-terminal-refusal",
    "copied-document retries reused the original owner's live transport identity");
    for (const record of cloneRows) {
        assert.equal(ownerAt(owners(record), slotA), aliasA, "original A must retain its slot throughout clone retries");
        assert.equal(ownerAt(owners(record), slotB), aliasB, "original B must retain its slot throughout clone retries");
        assert(!sessionAdded.some((entry) => entry.entity_bits_with_generation_decimal === record.entity_bits_with_generation_decimal &&
            entry.connection_epoch_decimal === record.connection_epoch_decimal), "copied A must not gain authority");
    }
    const oldAEpoch = acceptedA.connection_epoch_decimal;
    assert(typeof oldAEpoch === "string" && /^\d+$/.test(oldAEpoch), "SLOT1 connection epoch must remain exact decimal text");
    const cloneOpenedUs = cloneOpenedMs * 1000;
    const terminalCaptureUs = cloneCaptureEndMs * 1000;
    const copyCaptures = captures.filter((capture) => capture.phase === "copy-refusal");
    assert(copyCaptures.length >= 2, "SLOT1 copied-tab phase needs beginning and terminal retained captures");
    assert.equal(copiedTerminal, copyCaptures.at(-1), "SLOT1 copied-tab terminal failure must be the final retained phase capture");
    const cloneCountdowns = copyCaptures.filter((capture) => capture.window === 3 &&
        capture.document_epoch === copiedDocument.epoch).map(visibleCountdown).filter(Boolean);
    requireProduct(cloneCountdowns.length >= 2 && cloneCountdowns.some((row, index) => index > 0 &&
        row.remaining < cloneCountdowns[0].remaining && row.elapsed > cloneCountdowns[0].elapsed),
    "SLOT1", "copied-tab-terminal-refusal", "copied-tab captures do not show the actual visible countdown decreasing over time");
    const terminalRejoin = copiedTerminal.checkpoint.debug.rejoin;
    requireProduct(terminalRejoin?.text === "This game is open in another window. Close it, then reload this \u0070age",
        "SLOT1", "copied-tab-terminal-refusal", "copied-tab terminal document does not show the exact duplicate-window guidance");
    const retryTimes = terminalRejoin?.hello_elapsed_secs;
    assert(Array.isArray(retryTimes) && retryTimes.length >= 2 && retryTimes.length === terminalRejoin.hellos_sent,
        "SLOT1 copied-tab retries must retain every actual client Hello send timestamp");
    assertSlot1HelloTimes(retryTimes);
    const deadlineTiming = assertSlot1DeadlineChecks(terminalRejoin);
    assert.equal(cloneRows.length, terminalRejoin.hellos_sent,
        "SLOT1 every actual copied-tab Hello send must have one matching server decision record");
    const firstCopyCall = Math.min(...copyCaptures.map((capture) => capture.input_call));
    const lastCopyCall = Math.max(...copyCaptures.map((capture) => capture.input_call));
    const captureUseIndexes = new Set(copyCaptures.map((capture) => capture.input_call - 1));
    const focusCalls = input.map((entry, index) => ({ entry, index })).filter(({ entry, index }) =>
        entry.call === "use" && index >= firstCopyCall && index <= lastCopyCall && !captureUseIndexes.has(index));
    const focusWindows = focusCalls.map(({ entry }) => entry.window);
    assert(focusWindows.length >= 30, "SLOT1 copied-tab phase must physically rotate all three visible windows during the refusal countdown");
    assert.equal(focusWindows.length % 3, 0, "SLOT1 physical focus rotations must contain complete A/B/copied-document cycles");
    for (let index = 0; index + 2 < focusWindows.length; index += 3) {
        assert.deepEqual(focusWindows.slice(index, index + 3), [1, 2, 3], "SLOT1 physical window rotation must preserve A/B foreground activity around the copied tab");
    }
    const copyWaits = input.slice(firstCopyCall, lastCopyCall + 1).filter((entry) => entry.call === "wait");
    assert(copyWaits.every((entry) => entry.args[0] <= 400),
        "SLOT1 copied-tab phase may not leave the real owner windows backgrounded for a long wait");
    assert(copyWaits.reduce((sum, entry) => sum + entry.args[0], 0) >= 12_000,
        "SLOT1 copied-tab phase must retain at least 12 seconds of explicit visible-window waits");
    for (const [window, identity, slot] of [[1, identities.A, slotA], [2, identities.B, slotB]]) {
        const ownerFrames = captures.filter((capture) => capture.phase === "copy-refusal" && capture.window === window)
            .sort((left, right) => left.ordinal - right.ordinal);
        assert(ownerFrames.length >= 2, `SLOT1 needs beginning and ending copied-tab interval frames from owner window ${window}`);
        const first = ownerFrames[0].checkpoint.debug;
        const last = ownerFrames.at(-1).checkpoint.debug;
        requireProduct(first.phase?.startsWith("Joined") && last.phase?.startsWith("Joined"), "SLOT1", "owners-stable",
            `owner window ${window} stopped being Joined during the copied-tab interval`);
        assert.equal(first.player_id_decimal, identity);
        assert.equal(last.player_id_decimal, identity);
        assert.equal(first.slot, slot);
        assert.equal(last.slot, slot);
        assert.equal(last.match_id_decimal, first.match_id_decimal,
            `SLOT1 owner window ${window} must remain in the same exact match during clone retries`);
        requireProduct(first.last_frame && last.last_frame && last.last_frame.latest_tick > first.last_frame.latest_tick,
            "SLOT1", "owners-stable", `owner window ${window} actual client frames did not advance across the copied-tab interval`);
    }
    const ownerRecords = records.filter((record) =>
        (record.server_monotonic_us > acceptedB.server_monotonic_us ||
            (record.server_monotonic_us === acceptedB.server_monotonic_us && record.sequence > acceptedB.sequence)) &&
        ["hello_decision", "session_added", "session_authority_removed", "owner_checkpoint"].includes(record.event));
    for (const record of ownerRecords) {
        const snapshots = record.event === "hello_decision" ? [record.owners_before, record.owners_after] : [record.owners];
        for (const snapshot of snapshots) {
            assert(Array.isArray(snapshot), `SLOT1 ${record.event} lacks actual owner state`);
            assert.equal(ownerAt(snapshot, slotA), aliasA, `SLOT1 ${record.event} changed A owner`);
            assert.equal(ownerAt(snapshot, slotB), aliasB, `SLOT1 ${record.event} changed B owner`);
        }
    }
    assertSlot1CloneOwnerContinuity(records, cloneOpenedUs, terminalCaptureUs, [acceptedA, acceptedB]);
    const originalOwners = [acceptedA, acceptedB].map((accepted) => ({
        entity: accepted.entity_bits_with_generation_decimal,
        epoch: accepted.connection_epoch_decimal,
        alias: accepted.identity_alias,
        slot: accepted.decision.slot,
        match: accepted.match_id_decimal,
    }));
    const afterTerminal = records.filter((record) => record.event === "owner_checkpoint" && record.host_unix_us >= terminalCaptureUs);
    assert(afterTerminal.length > 0, "SLOT1 needs a flushed actual owner checkpoint after the final copied-tab capture");
    assert.equal(afterTerminal[0].match_id_decimal, acceptedA.match_id_decimal,
        "SLOT1 post-terminal checkpoint must retain the actual owners' match");
    for (const owner of originalOwners) {
        assert(afterTerminal[0].active_epochs.some((active) => active.entity_bits_with_generation_decimal === owner.entity &&
            active.connection_epoch_decimal === owner.epoch && active.identity_alias === owner.alias &&
            active.match_id_decimal === owner.match && active.slot === owner.slot),
        "SLOT1 post-capture checkpoint must prove both original exact Session epochs remain active");
    }
    assert.equal(ownerAt(afterTerminal[0].owners, slotA), aliasA,
        "SLOT1 post-capture checkpoint must retain A as the actual owner of A's permanent slot");
    assert.equal(ownerAt(afterTerminal[0].owners, slotB), aliasB,
        "SLOT1 post-capture checkpoint must retain B as the actual owner of B's permanent slot");

    const reload = input.find((entry) => entry.call === "reload" && entry.window === 1);
    assert(reload, "SLOT1 original A reload must be a physical harness IO call");
    const removal = removed.find((record) => record.entity_bits_with_generation_decimal === acceptedA.entity_bits_with_generation_decimal && record.connection_epoch_decimal === oldAEpoch);
    assert(removal, "SLOT1 must timestamp actual removal of the old authoritative Session");
    const reloadToRemovalUs = removal.host_unix_us - reload.wall_time_ms * 1000;
    assert(Number.isFinite(reloadToRemovalUs) && reloadToRemovalUs >= 0, "SLOT1 reload-to-authority-removal time must use the shared host clock");
    const reloadedCaptures = captures.filter((capture) => capture.phase === "reload-rejoin" && capture.window === 1);
    assert(reloadedCaptures.length >= 2, "SLOT1 must retain the original before-reload and after-rejoin frames");
    const originalFrames = reloadedCaptures.map((capture) => ({ capture, debug: capture.checkpoint.debug }));
    const before = originalFrames.find(({ capture, debug }) => debug.phase?.startsWith("Joined") &&
        debug.player_id_decimal === identities.A && debug.slot === slotA && input[capture.screenshot_input_call]?.wall_time_ms <= reload.wall_time_ms);
    const after = originalFrames.find(({ capture, debug }) => debug.phase?.startsWith("Joined") &&
        debug.player_id_decimal === identities.A && debug.slot === slotA && input[capture.screenshot_input_call]?.wall_time_ms > reload.wall_time_ms);
    requireProduct(Boolean(before && after), "SLOT1", "reload-same-id",
        "reload did not preserve the actual exact player ID and slot before and after rejoin");
    const beforeCaptureHostUs = completedCaptureHostMs(before.capture, input) * 1000;
    const reloadHostUs = reload.wall_time_ms * 1000;
    assert(Number.isFinite(beforeCaptureHostUs), "SLOT1 original pre-reload capture has no actual host clock");
    const preReloadBoundaries = records.filter((record) => record.event === "owner_checkpoint" &&
        record.host_unix_us >= beforeCaptureHostUs && record.host_unix_us <= reloadHostUs);
    assert(preReloadBoundaries.length > 0,
        "SLOT1 requires an actual private owner checkpoint after A's final Joined capture and before reload");
    assert.equal(preReloadBoundaries[0].match_id_decimal, acceptedA.match_id_decimal,
        "SLOT1 pre-reload checkpoint must retain the actual owners' match");
    for (const owner of originalOwners) {
        assert(preReloadBoundaries[0].active_epochs.some((active) =>
            active.entity_bits_with_generation_decimal === owner.entity && active.connection_epoch_decimal === owner.epoch &&
            active.identity_alias === owner.alias && active.match_id_decimal === owner.match && active.slot === owner.slot),
        "SLOT1 pre-reload boundary must prove both exact original Session epochs remain active");
    }
    assert.equal(ownerAt(preReloadBoundaries[0].owners, slotA), aliasA,
        "SLOT1 pre-reload boundary must retain A's actual permanent slot owner");
    assert.equal(ownerAt(preReloadBoundaries[0].owners, slotB), aliasB,
        "SLOT1 pre-reload boundary must retain B's actual permanent slot owner");
    const reloadFromJoinedMs = reload.wall_time_ms - input[before.capture.screenshot_input_call].wall_time_ms;
    assert(reloadFromJoinedMs >= 0 && reloadFromJoinedMs <= 5_000, `SLOT1 original reload must follow a joined capture within five seconds: ${reloadFromJoinedMs}ms`);
    assert(typeof before.debug.match_id_decimal === "string" && /^\d+$/.test(before.debug.match_id_decimal), "SLOT1 public match identity must be lossless decimal text");
    requireProduct(after.debug.match_id_decimal === before.debug.match_id_decimal, "SLOT1", "same-slot-rejoin",
        "reload changed the actual match identity");
    assert(hello.filter((record) => record.identity_alias === aliasA).every((record) => record.match_id_decimal === before.debug.match_id_decimal),
        "SLOT1 private Hello decisions and client frames must share the same exact match identity");
    const rejoinDelayMs = input[after.capture.screenshot_input_call].wall_time_ms - reload.wall_time_ms;
    requireProduct(rejoinDelayMs >= 0 && rejoinDelayMs <= 15_000, "SLOT1", "same-slot-rejoin",
        `same-slot rejoin exceeded 15 seconds: ${rejoinDelayMs}ms`);
    assert(after.capture.ordinal > before.capture.ordinal, "SLOT1 post-reload capture must follow the pre-reload frame");
    const newSession = sessionAdded.find((record) => record.identity_alias === aliasA && record.slot === slotA &&
        record.connection_epoch_decimal !== oldAEpoch && record.entity_bits_with_generation_decimal !== acceptedA.entity_bits_with_generation_decimal &&
        record.server_monotonic_us > removal.server_monotonic_us);
    requireProduct(Boolean(newSession), "SLOT1", "same-slot-rejoin", "accepted A did not rejoin in a new actual Session on the same permanent slot");
    assert.equal(newSession.match_id_decimal, after.debug.match_id_decimal,
        "SLOT1 rejoined Session lifecycle must match the actual rejoined client world");
    requireProduct(hello.some((record) => record.decision?.accepted === true && record.decision.slot === slotA &&
        record.identity_alias === aliasA && record.entity_bits_with_generation_decimal === newSession.entity_bits_with_generation_decimal &&
        record.connection_epoch_decimal === newSession.connection_epoch_decimal && record.match_id_decimal === newSession.match_id_decimal),
    "SLOT1", "same-slot-rejoin", "rejoined Session is not bound to its actual accepted new transport Hello");
    const reloadCountdowns = reloadedCaptures.filter((capture) =>
        input[capture.screenshot_input_call]?.wall_time_ms >= reload.wall_time_ms &&
        capture.document_epoch === after.capture.document_epoch).map(visibleCountdown).filter(Boolean);
    assert(reloadCountdowns.length > 0,
        "SLOT1 original reload must retain its actual visible reconnect countdown before rejoining");
    const finalRejoinUs = completedCaptureHostMs(after.capture, input) * 1000;
    const finalBoundary = records.find((record) => record.event === "owner_checkpoint" && record.host_unix_us >= finalRejoinUs);
    assert(finalBoundary, "SLOT1 observer needs a healthy actual checkpoint after the final rejoin capture");
    assert.equal(finalBoundary.match_id_decimal, newSession.match_id_decimal,
        "SLOT1 final rejoin checkpoint must retain the actual rejoined match");
    for (const owner of [newSession, acceptedB]) {
        assert(finalBoundary.active_epochs.some((active) => active.entity_bits_with_generation_decimal === owner.entity_bits_with_generation_decimal &&
            active.connection_epoch_decimal === owner.connection_epoch_decimal && active.identity_alias === owner.identity_alias &&
            active.match_id_decimal === owner.match_id_decimal && active.slot === (owner.decision?.slot ?? owner.slot)),
        "SLOT1 final checkpoint must retain the exact rejoined A and original B Session epochs");
    }
    assert.equal(ownerAt(finalBoundary.owners, slotA), aliasA);
    assert.equal(ownerAt(finalBoundary.owners, slotB), aliasB);
    return {
        reloadToAuthorityRemovalUs: String(reloadToRemovalUs),
        rejoinDelayMs,
        oldConnectionEpoch: oldAEpoch,
        deadlineTiming,
    };
}

module.exports = {
    ROW_CHECKS, ROW_PHYSICAL_CHECKS, ROW_INPUT_CALLS, judge, judgeStartupOutcome,
    sourceControls: { sel4DoubleTapReceipt, judgeKm1GuidePixels, per1GoldPixels, pixelGlyphShape, judgePer1NoticePixels, hasKeyBinding, frameNamedDispatches, verifiedRoot, compatibilityLine, judgeKm1, judgeKm1ReservedPack, judgePer1Storage, judgeSel4OldMatchPrune, judgeCam2, judgeCam2CursorPixels, judgeP1Attachment, judgeSh1MenuControls, judgeKm2FocusOverlay, judgeKm2OverlayPixels, assertRequiredPhysicalReceipts, assertSlotObserverRecordShapes, assertSlot1CapacityBoundary, assertSlot1CloneOwnerContinuity, assertSlot1DeadlineChecks, assertSlot1HelloTimes, assertSlot1TerminalPixels },
};
