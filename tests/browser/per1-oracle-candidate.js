// turts-test-role: oracle
// PROPOSAL ONLY: browser semantic evaluator. Native path qualification is separate and pending.
'use strict';

const { per1FixtureFor } = require('./harness/per1-preboot-fixture.js');

function evaluatePer1BrowserEvidence({ audit, fixture, debug, initialDebug, expectedFixtureId, input, captures }) {
    const assert = require('node:assert/strict');
    const setup = (condition, message) => { if (!condition) throw new Error(`PER1 setup/data invalid: ${message}`); };
    const product = (condition, clause, message) => {
        if (!condition) {
            const error = new Error(`PER1 ${clause}: ${message}`);
            error.task16_semantic = { row: 'PER1', clause };
            throw error;
        }
    };
    setup(['per1-settings-corrupt-v1', 'per1-invalid-binding-v1'].includes(expectedFixtureId),
        'row does not select one of the two approved single-corrupt-value PER1 cases');
    setup(fixture && fixture.fixture === expectedFixtureId && fixture.status === 'complete',
        'private fixture report is absent, incomplete, or has the wrong allowlisted case id');
    setup(fixture.corrupt_case === (expectedFixtureId === 'per1-invalid-binding-v1' ? 'binding' : 'settings'),
        'fixture case kind does not match its approved ID');
    setup(Array.isArray(fixture.windows) && fixture.windows.length === 1 && fixture.windows[0].window === 1 &&
        fixture.windows[0].observed_window === 1 && fixture.windows[0].fixture === expectedFixtureId &&
        fixture.windows[0].status === 'complete', 'exactly one successful actual fixture window 1 is required');
    setup(audit && Array.isArray(audit.entries) && Array.isArray(audit.initial_state) && Array.isArray(audit.final_state),
        'actual browser storage audit is incomplete');
    const initial = audit.initial_state.find((state) => state.window === 1);
    const final = audit.final_state.find((state) => state.window === 1);
    setup(initial && final && initial.document_epoch > 0 && final.document_epoch === initial.document_epoch,
        'PER1 must remain in the actual first document through final capture');
    const windowReceipt = fixture.windows[0];
    const actualUrl = initial.document_url;
    let actualOrigin;
    try { actualOrigin = new URL(actualUrl).origin; } catch { setup(false, 'actual first document URL is invalid'); }
    setup(initial.origin === actualOrigin && windowReceipt.target_origin === actualOrigin &&
        windowReceipt.initial_url === actualUrl && windowReceipt.observed_document_url === actualUrl &&
        windowReceipt.initial_time_origin === initial.document_time_origin &&
        windowReceipt.observed_time_origin === initial.document_time_origin &&
        windowReceipt.document_epoch === initial.document_epoch &&
        Number.isSafeInteger(windowReceipt.observed_window) && windowReceipt.observed_window === 1,
    'fixture receipt does not bind the target origin, window, exact initial document, time origin, and epoch');
    setup(Number.isFinite(windowReceipt.initial_wall_time_ms) && Number.isFinite(windowReceipt.initial_performance_ms),
        'initializer lacks actual wall/performance clock anchors');
    setup(Array.isArray(windowReceipt.fixture_audit_operations) && windowReceipt.fixture_audit_operations.length > 0 &&
        windowReceipt.fixture_audit_operations.every((operation) => operation.source === 'storage_api' && operation.provenance === 'fixture_seed') &&
        windowReceipt.fixture_audit_operations.some((operation) => operation.key === '__task16_preboot_seeded_v1') &&
        windowReceipt.fixture_audit_operations.some((operation) => operation.key === '__task16_preboot_receipt_v1'),
    'fixture seed/sentinel/receipt audit operations are not all identified as harness-only');
    const expectedFixture = per1FixtureFor(expectedFixtureId);
    setup(Array.isArray(windowReceipt.seeds) && windowReceipt.seeds.length === expectedFixture.seeds.length &&
        Array.isArray(fixture.seed_receipts) && fixture.seed_receipts.length === expectedFixture.seeds.length,
    'seed receipt count does not match the exact allowlisted fixture');
    const crypto = require('node:crypto');
    const sha256 = (value) => crypto.createHash('sha256').update(String(value)).digest('hex');
    for (const [index, expected] of expectedFixture.seeds.entries()) {
        const seed = windowReceipt.seeds[index];
        const flat = fixture.seed_receipts[index];
        setup(seed && flat && seed.storage === expected.storage && seed.key === expected.key &&
            seed.value === expected.value && seed.role === expected.role && seed.set_returned === true &&
            seed.readback_matches === true && seed.readback === expected.value &&
            seed.key_sha256 === sha256(expected.key) && seed.value_sha256 === sha256(expected.value) &&
            seed.readback_sha256 === sha256(expected.value) &&
            Number.isFinite(seed.operation_started_wall_time_ms) && Number.isFinite(seed.operation_finished_wall_time_ms) &&
            seed.operation_finished_wall_time_ms >= seed.operation_started_wall_time_ms &&
            Number.isFinite(seed.operation_started_performance_ms) && Number.isFinite(seed.operation_finished_performance_ms) &&
            seed.operation_started_performance_ms >= windowReceipt.initial_performance_ms &&
            seed.operation_finished_performance_ms >= seed.operation_started_performance_ms &&
            seed.operation_started_wall_time_ms >= windowReceipt.initial_wall_time_ms &&
            seed.window === 1 && seed.observed_window === 1 && seed.document_epoch === initial.document_epoch &&
            flat.key === seed.key && flat.value === seed.value && flat.readback === seed.readback &&
            flat.key_sha256 === seed.key_sha256 && flat.value_sha256 === seed.value_sha256 &&
            flat.readback_sha256 === seed.readback_sha256 && flat.set_returned === true &&
            flat.readback_matches === true && flat.window === 1 && flat.observed_window === 1 &&
            flat.document_epoch === initial.document_epoch && flat.target_origin === actualOrigin &&
            flat.initial_url === actualUrl && flat.initial_time_origin === initial.document_time_origin,
        `seed ${index + 1} lacks the exact successful allowlisted value, hash, clock, origin, window, or document lineage`);
    }
    const initialLocal = initial.localStorage || {};
    const local = final.localStorage || {};
    const session = final.sessionStorage || {};
    for (const seed of windowReceipt.seeds) {
        setup(audit.entries.some((entry) => entry.source === 'storage_api' && entry.provenance === 'fixture_seed' &&
            entry.storage === seed.storage && entry.operation === 'set' && entry.key === seed.key && entry.value === seed.value),
        `fixture Storage.setItem for ${seed.key} is absent from the fixture-excluded audit stream`);
    }
    const productEvents = audit.entries.filter((entry) => entry.source === 'storage_api' &&
        entry.provenance !== 'fixture_seed' && entry.window === 1 && entry.document_epoch === initial.document_epoch);
    const eventFor = (storage, operation, key) => productEvents.filter((entry) =>
        entry.storage === storage && entry.operation === operation && entry.key === key &&
        Number.isSafeInteger(entry.wall_time_ms) && entry.wall_time_ms > 0 &&
        Number.isFinite(entry.performance_time_ms) && entry.input_call === null &&
        ['before_physical_action', 'after_physical_action', 'before_reload', 'end_of_run'].includes(entry.flush_point));
    const successfulSet = (storage, key, value) => eventFor(storage, 'set', key).some((entry) => entry.value === value);
    const seedFor = (key, role) => fixture.seed_receipts.find((entry) => entry.storage === 'localStorage' && entry.key === key && entry.role === role);

    // Identity and authored physical prerequisites are data/setup guards, never product reds.
    setup(debug && initialDebug && debug.verified_root_mod && initialDebug.verified_root_mod &&
        debug.verified_root_mod.id === initialDebug.verified_root_mod.id &&
        debug.verified_root_mod.version === initialDebug.verified_root_mod.version &&
        debug.verified_root_mod.id === 'forge', 'actual verified root Forge identity is absent or changed');
    const ownId = debug.verified_root_mod.id;
    const version = debug.verified_root_mod.version;
    const parts = version.split('.').map(Number);
    setup(parts.length >= 2 && parts.every(Number.isInteger), 'actual root version cannot define a compatibility line');
    const line = parts[0] >= 1 ? String(parts[0]) : `0.${parts[1]}`;
    const ownBinding = `turts.v1.bindings.${ownId}.${line}`;
    setup(ownBinding === 'turts.v1.bindings.forge.0.2', 'actual root binding namespace does not derive to Forge 0.2');
    const playerId = debug.player_id_decimal;
    const match = debug.match_id_decimal;
    const slot = debug.slot;
    setup(typeof playerId === 'string' && /^(0|[1-9]\d*)$/.test(playerId) &&
        typeof match === 'string' && /^(0|[1-9]\d*)$/.test(match) &&
        Number.isInteger(slot) && slot >= 0 && slot <= 255, 'actual player, match, or slot identity is invalid');
    const stale = fixture.seed_receipts.filter((entry) => entry.role === 'old-match-group');
    const staleIds = stale.map((seed) => /^turts\.v1\.groups\.(0|[1-9]\d*)\.(0|[1-9]\d*)$/.exec(seed.key)?.[1]);
    setup(stale.length >= 2 && staleIds.every((id) => id !== undefined) &&
        new Set(staleIds).size === stale.length && staleIds.every((id) => id !== match),
    'fixture setup requires at least two distinct valid stale match IDs, neither equal to the actual current match');

    const inputEvent = (call, code = null, modifiers = null) => input.find((entry) => entry.call === call &&
        (code === null || entry.args?.[0] === code) &&
        (modifiers === null || JSON.stringify(entry.args?.[1] || []) === JSON.stringify(modifiers)));
    const f1 = inputEvent('key', 'F1', []);
    const unlockWait = input.find((entry) => entry.call === 'wait' && entry.args?.[0] === 250 && entry.wall_time_ms >= (f1?.wall_time_ms || Infinity));
    const worldClick = input.find((entry, index) => entry.call === 'click' && entry.window === 1 &&
        index > input.indexOf(unlockWait) && entry.args?.[0] === 712 && entry.args?.[1] === 416 &&
        entry.actual_events?.some((event) => event.type === 'pointerdown' && event.canvas_target === true));
    const worldReady = captures.find((capture) => capture.name === 'guide-dismissed-world-ready');
    setup(f1?.actual_events?.some((event) => event.type === 'keydown' && event.code === 'F1') && unlockWait && worldClick &&
        input.indexOf(unlockWait) < input.indexOf(worldClick),
        'F1 guide-dismissal/wait and a real canvas world click are required before selecting a group');
    const plannedWorldPoint = [712, 416];
    const readyDebug = worldReady?.checkpoint?.debug;
    const readyScene = worldReady?.checkpoint?.scene;
    setup(worldReady && worldReady.window === 1 && worldReady.document_epoch === initial.document_epoch &&
        worldReady.input_call > input.indexOf(unlockWait) && worldReady.input_call < input.indexOf(worldClick) &&
        worldClick.args?.[0] === plannedWorldPoint[0] && worldClick.args?.[1] === plannedWorldPoint[1],
    'post-F1 pre-selection screenshot is absent, out of order, or does not match the fixed Forge target point');
    const guideNodes = readyDebug?.ui_snapshot?.filter((node) => node.kind === 'widget' &&
        (node.id === 'guide' || (typeof node.id === 'string' && node.id.startsWith('guide.'))));
    setup(Array.isArray(readyDebug?.ui_snapshot) && Array.isArray(guideNodes) && guideNodes.every((node) => node.visible !== true),
        'actual post-F1 checkpoint still has a visible guide overlay over the map');
    const pointCoveredByUiRoot = (readyDebug?.ui_snapshot || []).some((node) => {
        const rect = node.rect;
        return node.kind === 'widget' && node.capture_root === true && node.visible === true &&
            Array.isArray(rect) && rect.length === 4 &&
            plannedWorldPoint[0] >= rect[0] && plannedWorldPoint[0] <= rect[0] + rect[2] &&
            plannedWorldPoint[1] >= rect[1] && plannedWorldPoint[1] <= rect[1] + rect[3];
    });
    setup(!pointCoveredByUiRoot, 'actual post-F1 UI panel root occludes the planned owned-unit point');
    const targetUnitId = 7;
    const actualTarget = readyDebug?.replicated_world?.units?.find((unit) => unit.unit_id === targetUnitId);
    const targetProjection = readyScene?.units?.find((unit) => unit.unit_id === targetUnitId);
    setup(actualTarget?.content_id === 'forge:turret' && actualTarget.owner_slot === readyDebug?.slot &&
        Array.isArray(targetProjection?.ground_screen_center) && targetProjection.ground_screen_center.length === 2 &&
        targetProjection.ground_screen_center.every(Number.isFinite) &&
        Math.hypot(targetProjection.ground_screen_center[0] - plannedWorldPoint[0],
            targetProjection.ground_screen_center[1] - plannedWorldPoint[1]) <= 1,
    'actual pre-selection replica does not show the fixed point centered on owned Forge unit 7');
    const ctrlAssign = inputEvent('key', 'Digit1', ['Control']);
    const plainRecall = inputEvent('key', 'Digit1', []);
    const f10Open = inputEvent('key', 'F10', []);
    const changeClick = input.find((entry) => entry.call === 'click' && entry.actual_events?.some((event) => event.type === 'pointerdown') &&
        entry.wall_time_ms > (f10Open?.wall_time_ms || Infinity));
    const keyZ = inputEvent('key', 'KeyZ', []);
    const wheel = input.find((entry) => entry.call === 'wheel' && entry.args?.[0] === 10000 &&
        entry.actual_events?.some((event) => event.type === 'wheel' && event.delta_y === 10000));
    const f10Close = [...input].reverse().find((entry) => entry.call === 'key' && entry.args?.[0] === 'F10' &&
        JSON.stringify(entry.args?.[1] || []) === '[]');
    const closeWait = input.find((entry) => entry.call === 'wait' && entry.args?.[0] === 250 &&
        entry.wall_time_ms > (f10Close?.wall_time_ms || Infinity));
    const masterClick = expectedFixtureId === 'per1-settings-corrupt-v1'
        ? input.find((entry) => entry.call === 'click' && entry.actual_events?.some((event) => event.type === 'pointerdown') &&
            entry.wall_time_ms > (wheel?.wall_time_ms || Infinity) && entry.wall_time_ms < (f10Close?.wall_time_ms || Infinity))
        : null;
    setup(ctrlAssign?.actual_events?.some((event) => event.type === 'keydown' && event.code === 'Digit1' && event.ctrl_key) &&
        plainRecall?.actual_events?.some((event) => event.type === 'keydown' && event.code === 'Digit1' && !event.ctrl_key && !event.shift_key) &&
        f10Open?.actual_events?.some((event) => event.type === 'keydown' && event.code === 'F10') &&
        changeClick && keyZ?.actual_events?.some((event) => event.type === 'keydown' && event.code === 'KeyZ') &&
        input.indexOf(keyZ) > input.indexOf(changeClick) && wheel &&
        f10Close?.wall_time_ms > (wheel?.wall_time_ms || Infinity) && closeWait &&
        (expectedFixtureId !== 'per1-settings-corrupt-v1' || masterClick),
    'required physical F1, group, binding-editor, KeyZ, bottom-scroll, Master, and menu-close calls are not all actual/reviewable');
    const menuBefore = captures.find((capture) => capture.name === 'menu-before-binding');
    const menuBeforeNode = menuBefore?.checkpoint?.debug?.ui_snapshot?.find((node) => node.kind === 'settings_menu' && node.visible === true);
    setup(menuBefore && menuBefore.document_epoch === initial.document_epoch &&
        menuBeforeNode?.text_nodes?.some((node) => node.visible === true && node.text === 'Add modifier') &&
        menuBeforeNode?.text_nodes?.some((node) => node.visible === true && node.text === 'Change'),
    'actual visible binding editor row is absent at the uncalibrated click location');
    const bindingArmed = captures.find((capture) => capture.name === 'binding-editor-armed');
    const armedMenu = bindingArmed?.checkpoint?.debug?.ui_snapshot?.find((node) => node.kind === 'settings_menu' && node.visible === true);
    setup(bindingArmed && bindingArmed.document_epoch === initial.document_epoch &&
        armedMenu?.text_nodes?.some((node) => node.text?.startsWith('Press a key or ') && node.text.endsWith(' button for add_modifier')) &&
        menuBeforeNode.text_nodes.some((node) => node.visible === true && node.text === 'Change' &&
            Array.isArray(node.rect) && node.rect.length === 4 &&
            changeClick.args?.[0] >= node.rect[0] && changeClick.args?.[0] <= node.rect[0] + node.rect[2] &&
            changeClick.args?.[1] >= node.rect[1] && changeClick.args?.[1] <= node.rect[1] + node.rect[3]),
    'fixed click did not hit an actual visible Change label and arm the actual Add modifier editor state');

    const selectedCapture = captures.find((capture) => capture.name === 'selected-before-assignment');
    const assignedCapture = captures.find((capture) => capture.name === 'group-assigned');
    const recalledCapture = captures.find((capture) => capture.name === 'group-recalled');
    setup(selectedCapture && assignedCapture && recalledCapture && selectedCapture.window === 1 && assignedCapture.window === 1 && recalledCapture.window === 1 &&
        selectedCapture.document_epoch === initial.document_epoch && assignedCapture.document_epoch === initial.document_epoch &&
        selectedCapture.checkpoint?.debug && assignedCapture.checkpoint?.debug,
    'physical coordinates were not calibrated to actual same-document selection/group checkpoints');
    const selectedDebug = selectedCapture.checkpoint.debug;
    const selected = selectedDebug.selection;
    setup(Array.isArray(selected) && selected.includes(targetUnitId),
        'fixed world click did not select the actual projected owner-slot Forge target unit 7');
    setup(Array.isArray(selected) && selected.length > 0 && selected.every((id) => Number.isSafeInteger(id)),
        'world selection was empty before Ctrl+Digit1; physical point is setup-invalid');
    const selectedUnits = selectedDebug.replicated_world?.units;
    setup(Array.isArray(selectedUnits) && selected.every((id) => selectedUnits.some((unit) =>
        unit.unit_id === id && unit.owner_slot === selectedDebug.slot)),
    'selected units are not verified as owned by the current actual slot; physical point is setup-invalid');
    const assignCall = input.findIndex((entry) => entry.call === 'key' && entry.args?.[0] === 'Digit1' &&
        JSON.stringify(entry.args?.[1] || []) === '["Control"]');
    setup(assignCall >= 0 && selectedCapture.input_call < assignCall && assignCall < assignedCapture.input_call,
        'selection and assignment captures do not bracket actual physical Ctrl+Digit1');
    const assignEntry = input[assignCall];
    const assignKey = assignEntry.actual_events?.find((event) => event.type === 'keydown' && event.code === 'Digit1');
    setup(assignKey && Number.isSafeInteger(assignKey.seq), 'Ctrl+Digit1 has no actual browser keydown sequence');
    const assignedDebug = assignedCapture.checkpoint.debug;
    const assignedFrame = [...(assignedDebug.frames?.recent || []), ...(assignedDebug.frames?.retained_input || [])].find((frame) =>
        frame.applied_seq?.includes(assignKey.seq) && frame.named_dispatches?.some((dispatch) =>
            dispatch.origin === 'key' && dispatch.code === 'Digit1' && dispatch.matched_action === 'group_assign_1'));
    product(Boolean(assignedFrame), 'old-match-groups-pruned', 'actual physical Ctrl+Digit1 did not reach the group_assign_1 branch');
    const assignedGroup = assignedFrame.groups?.['1'];
    product(Array.isArray(assignedGroup) && assignedGroup.length > 0 &&
        JSON.stringify([...assignedGroup].sort((a, b) => a - b)) === JSON.stringify([...selected].sort((a, b) => a - b)),
    'old-match-groups-pruned', 'actual group 1 did not persist the nonempty pre-assignment own selection');
    const currentGroup = `turts.v1.groups.${match}.${slot}`;
    const recallEntry = inputEvent('key', 'Digit1', []);
    const recallKey = recallEntry?.actual_events?.find((event) => event.type === 'keydown' && event.code === 'Digit1');
    const recalledDebug = recalledCapture.checkpoint.debug;
    setup(recallEntry && recalledCapture.input_call > input.indexOf(recallEntry) &&
        Array.isArray(recalledDebug.selection) && recalledDebug.selection.length > 0,
    'plain Digit1 did not produce an actual nonempty recalled group selection');
    const recalledFrame = [...(recalledDebug.frames?.recent || []), ...(recalledDebug.frames?.retained_input || [])].find((frame) =>
        frame.applied_seq?.includes(recallKey?.seq) && frame.named_dispatches?.some((dispatch) =>
            dispatch.origin === 'key' && dispatch.code === 'Digit1' && dispatch.matched_action === 'group_recall_1'));
    product(Boolean(recalledFrame) && Array.isArray(recalledFrame.selection) && recalledFrame.selection.length > 0 &&
        JSON.stringify([...recalledFrame.selection].sort((a, b) => a - b)) === JSON.stringify([...assignedGroup].sort((a, b) => a - b)),
    'old-match-groups-pruned', 'physical Digit1 did not recall the same nonempty group 1 assignment');

    // Product persistence predicates run only after all fixture/identity/physical setup guards.
    product(session['turts.v1.player_id'] === playerId && successfulSet('sessionStorage', 'turts.v1.player_id', playerId),
        'browser-key-names', 'actual session player id lacks a successful product Storage API write matching the client decimal identity');
    const ownRaw = local[ownBinding];
    product(typeof ownRaw === 'string' && successfulSet('localStorage', ownBinding, ownRaw),
        'browser-key-names', 'verified Forge binding namespace lacks an actual product Storage.setItem matching final storage');
    const settingsRaw = local['turts.v1.settings'];
    const settingsSeed = seedFor('turts.v1.settings', expectedFixtureId === 'per1-settings-corrupt-v1'
        ? 'malformed-settings-json' : 'valid-task15-audio-seed');
    setup(settingsSeed && settingsSeed.readback === settingsSeed.value, 'the expected settings fixture value was not seeded/read back');
    product(typeof settingsRaw === 'string' &&
        (successfulSet('localStorage', 'turts.v1.settings', settingsRaw) || expectedFixtureId === 'per1-invalid-binding-v1'),
    'browser-key-names', 'settings namespace lacks an actual successful product write or the exact valid C326 seed');
    product(Object.hasOwn(local, currentGroup) && successfulSet('localStorage', currentGroup, local[currentGroup]),
        'old-match-groups-pruned', 'actual current match/slot group lacks a successful product Storage.setItem');
    let currentGroupData;
    try { currentGroupData = JSON.parse(local[currentGroup]); } catch { setup(false, 'current group Storage value is not JSON'); }
    product(Array.isArray(currentGroupData?.groups?.['1']) && currentGroupData.groups['1'].length > 0 &&
        JSON.stringify([...currentGroupData.groups['1']].sort((a, b) => a - b)) === JSON.stringify([...assignedGroup].sort((a, b) => a - b)),
    'old-match-groups-pruned', 'current persisted group 1 is empty or differs from actual assigned/recalled live group');

    const foreign = fixture.seed_receipts.filter((entry) => entry.role === 'foreign-mod' || entry.role === 'foreign-compatibility-line');
    const otherMod = foreign.filter((entry) => entry.role === 'foreign-mod');
    const otherLine = foreign.filter((entry) => entry.role === 'foreign-compatibility-line');
    setup(otherMod.length === 1 && otherLine.length === 1, 'fixture must contain exactly one foreign mod and one alternate Forge line');
    for (const seed of foreign) {
        product(initialLocal[seed.key] === seed.value && local[seed.key] === seed.value,
            'mod-isolation', `foreign namespace ${seed.key} changed between actual initial/final storage`);
        product(!productEvents.some((entry) => entry.storage === 'localStorage' &&
            ((entry.key === seed.key && ['set', 'remove'].includes(entry.operation)) || entry.operation === 'clear')),
        'mod-isolation', `product mutated foreign namespace ${seed.key}`);
    }
    const effective = debug.effective_bindings;
    const defaults = effective?.defaults?.actions;
    product(defaults && Array.isArray(defaults.key_move) && defaults.key_move.length > 0 &&
        JSON.stringify(defaults.key_move) === JSON.stringify([{ Key: { code: 'KeyM', mods: [] } }]),
    'defaults-and-single-notice', 'actual verified Forge key_move default is not the required nonempty KeyM binding');
    product(effective.actions && JSON.stringify(effective.actions.key_move) === JSON.stringify(defaults.key_move),
        'defaults-and-single-notice', 'corrupt key_move did not fall back to the actual verified Forge KeyM default');
    product(effective.runtime_overrides && Object.keys(effective.runtime_overrides).every((name) => name === 'add_modifier') &&
        JSON.stringify(effective.runtime_overrides.add_modifier) === JSON.stringify([{ Key: { code: 'KeyZ', mods: [] } }]) &&
        JSON.stringify(effective.actions.add_modifier) === JSON.stringify([{ Key: { code: 'KeyZ', mods: [] } }]),
    'defaults-and-single-notice', 'physical Add modifier edit did not leave exactly add_modifier=KeyZ as the only runtime override');
    let bindingData;
    try { bindingData = JSON.parse(ownRaw); } catch { setup(false, 'actual own binding Storage value is not JSON'); }
    product(JSON.stringify(bindingData.actions?.add_modifier) === JSON.stringify([{ Key: { code: 'KeyZ', mods: [] } }]),
        'browser-key-names', 'actual own binding product write does not persist the physical add_modifier KeyZ edit');
    product(!bindingData.actions?.key_move || JSON.stringify(bindingData.actions.key_move) === JSON.stringify(defaults.key_move),
        'defaults-and-single-notice', 'reserved F12 key_move value remained in the persisted own binding namespace');

    for (const seed of stale) {
        setup(seed.set_returned === true && seed.readback_matches === true && seed.readback === seed.value,
            `stale group ${seed.key} lacks a successful actual seed/readback receipt`);
        product(eventFor('localStorage', 'remove', seed.key).length > 0 && !Object.hasOwn(local, seed.key),
            'old-match-groups-pruned', `stale match key ${seed.key} lacks actual removeItem success or final absence`);
    }
    product(!stale.some((seed) => seed.key === currentGroup), 'old-match-groups-pruned',
        'current group key aliases a seeded stale key');

    const corrupt = fixture.seed_receipts.find((entry) => entry.key === (expectedFixtureId === 'per1-settings-corrupt-v1'
        ? 'turts.v1.settings' : ownBinding));
    setup(corrupt && corrupt.set_returned === true && corrupt.readback_matches === true && corrupt.readback === corrupt.value,
        'exact corrupt storage input lacks successful actual preboot set/readback');
    const expectedNotice = expectedFixtureId === 'per1-settings-corrupt-v1'
        ? 'Settings were unreadable; default settings are in use.'
        : 'Player settings were unreadable; defaults are in use.';
    const noticeTexts = (debug.notices || []).map((notice) => String(notice?.text ?? notice?.message ?? ''))
        .filter((text) => /unreadable|defaults are in use|default settings are in use/i.test(text));
    const visibleNotices = (debug.ui_snapshot || []).filter((node) => node.kind === 'widget' && node.id === 'last_notice' &&
        node.visible === true && typeof node.text === 'string' &&
        /unreadable|defaults are in use|default settings are in use/i.test(node.text));
    product(noticeTexts.length === 1 && noticeTexts[0] === expectedNotice && visibleNotices.length === 1 &&
        visibleNotices[0].text === expectedNotice && Array.isArray(visibleNotices[0].rect) && visibleNotices[0].rect.length === 4 &&
        visibleNotices[0].rect.every(Number.isFinite) && visibleNotices[0].rect[2] > 0 && visibleNotices[0].rect[3] > 0,
    'defaults-and-single-notice', `actual engine notice and authored last_notice widget must show exactly: ${expectedNotice}`);
    const finalCapture = captures.filter((capture) => capture.name === 'settings-and-notice-final' || capture.name === 'binding-defaults-notice-final').at(-1);
    setup(finalCapture && finalCapture.input_call > input.indexOf(closeWait) && finalCapture.document_epoch === initial.document_epoch,
        'final notice screenshot is not after physical F10 close in the actual first document');
    const finalMenu = (debug.ui_snapshot || []).find((node) => node.kind === 'settings_menu' && node.id === 'settings_menu');
    setup(!finalMenu || finalMenu.visible === false, 'final notice capture remains behind the open settings menu; close F10 and recapture');

    const categories = ['master', 'voice', 'alerts', 'effects'];
    let storedSettings;
    try { storedSettings = JSON.parse(settingsRaw); } catch { setup(false, 'actual settings Storage value is not JSON'); }
    setup(storedSettings?.audio && initialDebug.audio?.categories && debug.audio?.categories,
        'actual settings/audio producer fields are incomplete');
    const f32 = (value, label) => {
        setup(typeof value === 'number' && Number.isFinite(value) && value >= 0 && value <= 1,
            `${label} is not a finite audio value in [0,1]`);
        return Math.fround(value);
    };
    if (expectedFixtureId === 'per1-invalid-binding-v1') {
        let initialSeed;
        try { initialSeed = JSON.parse(settingsSeed.value); } catch { setup(false, 'valid C326 settings seed is not JSON'); }
        product(initialSeed.audio && categories.every((category) =>
            f32(initialDebug.audio.categories[category], `initial ${category}`) === f32(initialSeed.audio[category], `seed ${category}`)),
        'volume-fields-preserved', 'actual initial audio producer differs from the nested valid C326 seed after f32 conversion');
    } else {
        product(categories.every((category) => initialDebug.audio.categories[category] === 1),
            'defaults-and-single-notice', 'malformed settings did not load all four actual default audio values');
    }
    if (expectedFixtureId === 'per1-settings-corrupt-v1') {
        setup(Math.abs(debug.audio.categories.master - initialDebug.audio.categories.master) > 0.1,
            'physical Master point did not change the actual volume; slider coordinate remains setup-invalid');
    }
    for (const category of categories) {
        const saved = f32(storedSettings.audio[category], `saved ${category}`);
        const actual = f32(debug.audio.categories[category], `actual ${category}`);
        product(saved === actual, 'volume-fields-preserved', `saved ${category} differs from the actual final volume producer`);
        if (category !== 'master' || expectedFixtureId === 'per1-invalid-binding-v1') {
            product(actual === f32(initialDebug.audio.categories[category], `initial ${category}`),
                'volume-fields-preserved', `${category} changed from its actual initial Task15 value`);
        }
    }
    return new Set(['browser-key-names', 'compatibility-line', 'mod-isolation',
        'defaults-and-single-notice', 'old-match-groups-pruned', 'volume-fields-preserved']);
}

function validateOptionalNativePathDocument(raw) {
    const document = JSON.parse(raw);
    const bad = (reason) => { throw new Error(`PER1 native path artifact is malformed source data: ${reason}`); };
    const hash = (value) => typeof value === 'string' && /^[a-f0-9]{64}$/.test(value);
    const exactU64 = (value) => typeof value === 'string' && /^(0|[1-9]\d*)$/.test(value) && BigInt(value) <= 18_446_744_073_709_551_615n;
    if (!document || typeof document !== 'object' || document.schema !== 'per1-native-path-audit-v2') bad('schema version is absent');
    const process = document.process;
    if (!process || !Number.isSafeInteger(process.pid) || process.pid <= 0 || process.exit_code !== 0 ||
        !Array.isArray(process.argv) || !process.argv.some((arg) => String(arg).includes('per1_native_paths')) ||
        typeof process.executable !== 'string' || !pathIsAbsolute(process.executable) || !hash(process.executable_sha256) ||
        !Number.isFinite(process.started_unix_ms) || !Number.isFinite(process.ended_unix_ms) || process.ended_unix_ms < process.started_unix_ms ||
        !hash(process.source_sha256) || !hash(process.runner_receipt_sha256)) bad('real process/source/runner identity fields are incomplete');
    if (typeof document.config_root !== 'string' || !pathIsAbsolute(document.config_root)) bad('config root is not absolute');
    const input = document.inputs;
    if (!input || typeof input.mod_id !== 'string' || !/^[a-z0-9_]+$/.test(input.mod_id) || typeof input.version !== 'string' ||
        !exactU64(input.match_id_decimal) || !Number.isInteger(input.slot) || input.slot < 0 || input.slot > 255) bad('actual native test match/slot/mod inputs are absent');
    const parts = input.version.split('.').map(Number);
    if (parts.length < 2 || !parts.every(Number.isInteger)) bad('native test mod version is invalid');
    const line = parts[0] >= 1 ? String(parts[0]) : `0.${parts[1]}`;
    const expected = {
        bindings: `turts/bindings-${input.mod_id}-${line}.json`,
        settings: 'turts/settings.json',
        groups: `turts/groups-${input.match_id_decimal}-${input.slot}.json`,
    };
    if (!document.paths || Object.entries(expected).some(([name, path]) => document.paths[name] !== path)) bad('resolved design 10.4 path names do not match actual test inputs');
    for (const operation of ['reads', 'writes']) {
        if (!Array.isArray(document[operation])) bad(`${operation} is absent`);
        for (const record of document[operation]) {
            if (!record || !Number.isSafeInteger(record.pid) || record.pid !== process.pid ||
                !['read', 'write'].includes(record.operation) || typeof record.path !== 'string' ||
                !record.path.startsWith(`${document.config_root}/`) || typeof record.method !== 'string' ||
                record.success !== true || !Number.isSafeInteger(record.byte_length) || record.byte_length <= 0 || !hash(record.sha256)) {
                bad(`${operation} contains a non-process-bound or incomplete actual file operation`);
            }
        }
    }
    for (const [name, path] of Object.entries(expected)) {
        if (!document.reads.some((record) => record.path === `${document.config_root}/${path}`) ||
            !document.writes.some((record) => record.path === `${document.config_root}/${path}`)) bad(`${name} lacks a real read and write receipt`);
    }
    return document;
}

function pathIsAbsolute(value) { return typeof value === 'string' && value.startsWith('/'); }

module.exports = { evaluatePer1BrowserEvidence, validateOptionalNativePathDocument };
