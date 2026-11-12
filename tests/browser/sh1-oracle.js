// turts-test-role: oracle
'use strict';
const assert = require('node:assert/strict');

function product(condition, clause, message) {
    if (!condition) {
        const error = new Error(`SH1 ${clause}: ${message}`);
        error.task16_semantic = { row: 'SH1', clause };
        throw error;
    }
}
function visible(capture, id) {
    const nodes = capture?.checkpoint?.debug?.ui_snapshot;
    assert(Array.isArray(nodes), 'SH1 actual UI observation is missing');
    const values = nodes.filter((node) => node.id === id && node.visible === true);
    assert.equal(values.length, 1, `SH1 actual visible ${id} is absent or duplicated`);
    const node = values[0];
    assert(Array.isArray(node.rect) && node.rect.length === 4 && node.rect.every(Number.isFinite) &&
        node.rect[2] > 0 && node.rect[3] > 0, 'SH1 actual UI rectangle is incomplete');
    return node;
}
function whitePixels(image, rect) {
    assert(image && image.width >= 1280 && image.height >= 720 && image.width / 1280 === image.height / 720,
        'SH1 requires matching real physical screenshot dimensions');
    const [x, y, width, height] = rect.map(Math.round);
    assert(x >= 0 && y >= 0 && width > 0 && height > 0 && x + width <= image.width && y + height <= image.height,
        'SH1 actual text rectangle falls outside its PNG');
    let total = 0;
    for (let py = y; py < y + height; py += 1) {
        for (let px = x; px < x + width; px += 1) {
            const at = (py * image.width + px) * 4;
            if (image.rgba[at] > 235 && image.rgba[at + 1] > 235 && image.rgba[at + 2] > 235 && image.rgba[at + 3] >= 250) total += 1;
        }
    }
    return total;
}
function judgeSh1ConflictOffer(captures) {
    const offered = captures.find((capture) => capture.name === 'conflict-open');
    const menus = offered?.checkpoint?.debug?.ui_snapshot?.filter((node) => node.kind === 'settings_menu' && node.visible === true);
    assert(menus?.length === 1 && menus[0].descendants_truncated === false && menus[0].text_truncated === false,
        'SH1 conflict offer needs the complete actual menu observation');
    const text = menus[0].text_nodes;
    const explanation = text.find((node) => node.visible === true && node.text === 'M is also bound to Move selected units');
    const swap = text.find((node) => node.visible === true && node.text === 'Swap');
    const cancel = text.find((node) => node.visible === true && node.text === 'Cancel' && node.rect[1] < 145);
    product(explanation && swap && cancel, 'conflict-swap-cancel-reset',
        'actual conflicting Stop chord must name its owner and offer Swap and Cancel');
    const overlaps = (left, right) => left[0] < right[0] + right[2] && left[0] + left[2] > right[0] &&
        left[1] < right[1] + right[3] && left[1] + left[3] > right[1];
    assert(swap.button_visible === true && cancel.button_visible === true &&
        Array.isArray(swap.button_rect) && Array.isArray(cancel.button_rect),
        'SH1 conflict controls need actual complete button geometry');
    product(!overlaps(explanation.rect, swap.button_rect) && !overlaps(explanation.rect, cancel.button_rect),
        'conflict-swap-cancel-reset', 'actual conflict controls must not cover their own explanation');
    return offered;
}

async function judgeSh1Lifecycle(view, input, captures, decoded, evaluated) {
    const fixture = JSON.parse(await view.serverFile('sh1-lifecycle-private.json'));
    assert.equal(fixture.fixture, 'sh1-real-acquisition-v1');
    assert(Array.isArray(fixture.records), 'SH1 actual HTTP/transport receipts are missing');
    const get = (name) => {
        const capture = captures.find((item) => item.name === name);
        assert(capture?.checkpoint?.debug, `SH1 lacks actual retained ${name}`);
        return capture;
    };
    const phases = [
        ['fetch-advertisement', /^FetchAdvertisement/, 'Fetching server details'],
        ['fetch-lockfile', /^FetchLockfile/, 'Fetching and checking the package list'],
        ['fetch-package', /^FetchPackages/, 'Downloading package (0 of 1 packages verified)'],
        ['load-verify', /^VerifyPackage/, 'Verifying package (0 of 1 packages verified)'],
        ['load-joining', /^(Connecting|Joining)/, 'Joining the game'],
    ];
    let previous = -1;
    for (const [name, phase, detail] of phases) {
        const capture = get(name);
        assert(capture.input_call > previous, 'SH1 acquisition observations are not ordered');
        previous = capture.input_call;
        product(phase.test(capture.checkpoint.debug.phase), 'load-phases', `actual ${name} phase differs`);
        const heading = visible(capture, 'shell.load.heading');
        const actualDetail = visible(capture, 'shell.load.detail');
        visible(capture, 'shell.load.root');
        const track = visible(capture, 'shell.load.progress.track');
        product(heading.text === 'Loading game' && actualDetail.text === detail && track.rect[2] > track.rect[3] && track.rect[3] > 0,
            'load-phases', `actual ${name} lacks its player-visible phase or progress track`);
        if (name !== 'fetch-advertisement' && name !== 'fetch-lockfile') {
            product(visible(capture, 'shell.load.package').text === 'forge 0.2.0', 'load-phases',
                'actual acquired package identity is missing from loading');
        }
        assert(!capture.checkpoint.debug.ui_snapshot.some((node) => node.kind === 'focus_overlay' && node.visible === true),
            'SH1 load pixel setup retains a focus overlay');
        product(whitePixels(decoded.get(name), heading.rect) >= 200, 'load-phases',
            `actual ${name} loading heading pixels are absent`);
    }
    for (const [kind, name] of [['advertisement', 'fetch-advertisement'], ['lockfile', 'fetch-lockfile'], ['package', 'fetch-package']]) {
        const record = fixture.records.find((item) => item.source === 'real_http_response' && item.hold === kind);
        const capture = get(name);
        assert(record && record.released === true && record.status === 200 &&
            record.document_epoch === capture.document_epoch && record.original_byte_length > 0 &&
            record.served_byte_length > 0 && record.served_headers['content-length'] === String(record.served_byte_length) &&
            record.released_wall_time_ms >= input[capture.input_call].wall_time_ms,
        `SH1 ${kind} screenshot is not causally before release of its actual held response`);
        if (kind !== 'lockfile') assert.equal(record.served_sha256, record.original_sha256);
    }
    for (const [kind, name] of [['verify', 'load-verify'], ['joining', 'load-joining']]) {
        const record = fixture.records.find((item) => item.source === 'actual_completed_frame_capture' && item.kind === kind);
        const capture = get(name);
        assert(record && record.released === true && record.held_callbacks >= 1 &&
            record.document_epoch === capture.document_epoch && record.phase === capture.checkpoint.debug.phase &&
            record.release_wall_time_ms >= input[capture.input_call].wall_time_ms,
        `SH1 ${kind} capture lacks an actual held/released completed consumer frame`);
    }
    const joining = get('load-joining');
    const fill = visible(joining, 'shell.load.progress.fill');
    product(fill.rect[2] === visible(joining, 'shell.load.progress.track').rect[2], 'load-phases',
        'verified package must fill the real progress bar before joining');
    evaluated.add('load-phases');

    const wrong = get('wrong-lockfile');
    const lock = fixture.records.find((item) => item.hold === 'lockfile');
    assert(lock && lock.original_sha256 !== lock.served_sha256 &&
        wrong.checkpoint.debug.phase.includes('lockfile hash mismatch:') &&
        wrong.checkpoint.debug.phase.includes(`sha256:${lock.served_sha256}`) &&
        wrong.checkpoint.debug.phase.includes(`expected hash sha256:${lock.original_sha256}`),
    'SH1 wrong-lock setup must bind actual received and expected hashes to the held real response');
    const rejection = visible(wrong, 'shell.rejection.heading');
    const guidance = visible(wrong, 'shell.rejection.detail');
    visible(wrong, 'shell.rejection.reload');
    product(rejection.text === 'Game files do not match the server' &&
        guidance.text.startsWith('Reload to fetch the game files again.') &&
        !wrong.checkpoint.debug.ui_snapshot.some((node) => node.visible === true && node.text?.includes('sha256:')),
    'wrong-lockfile-guidance', 'actual mismatch rejection lacks player words and a relevant action');
    product(whitePixels(decoded.get(wrong.name), rejection.rect) >= 200, 'wrong-lockfile-guidance',
        'actual player-word rejection heading pixels are absent');
    evaluated.add('wrong-lockfile-guidance');

    const lost = get('connection-lost');
    const after = get('connection-rejoined');
    const heading = visible(lost, 'shell.connection_lost.heading');
    const reload = visible(lost, 'shell.connection_lost.reload');
    const transport = fixture.records.filter((item) => item.source === 'real_websocket_proxy' && item.closed_by_fixture);
    assert.equal(transport.length, 1, 'SH1 must close exactly one actual current transport');
    assert(transport[0].both_sides_closed === true && transport[0].joined_before_close?.phase.startsWith('Joined') &&
        transport[0].close_document_epoch === lost.document_epoch &&
        transport[0].close_wall_time_ms < input[lost.input_call].wall_time_ms,
    'SH1 connection-loss setup lacks the actual current Joined socket closure');
    product(lost.checkpoint.debug.reconnect_required === true && heading.text === 'Connection lost',
        'connection-lost-reload', 'actual transport closure did not reach the required screen');
    product(whitePixels(decoded.get(lost.name), heading.rect) >= 200, 'connection-lost-reload',
        'actual connection-loss heading pixels are absent');
    const clicked = input.find((entry, index) => index > lost.input_call && index < after.input_call && entry.call === 'click');
    const dpr = lost.checkpoint.browser.device_pixel_ratio;
    assert(Number.isFinite(dpr) && dpr >= 1 && dpr <= 3,
        'SH1 Reload hit proof lacks its actual browser device factor');
    assert(clicked && clicked.actual_events.some((event) => event.type === 'pointerdown' &&
        clicked.window === lost.window && clicked.document_epoch === lost.document_epoch &&
        clicked.event_seq.includes(event.seq) && event.canvas_target === true &&
        event.client_x*dpr >= reload.rect[0] && event.client_x*dpr < reload.rect[0] + reload.rect[2] &&
        event.client_y*dpr >= reload.rect[1] && event.client_y*dpr < reload.rect[1] + reload.rect[3]),
    'SH1 reload consumer requires an actual click inside the captured real Reload widget');
    product(after.document_epoch > lost.document_epoch && after.checkpoint.debug.phase.startsWith('Joined') &&
        after.checkpoint.debug.reconnect_required === false &&
        after.checkpoint.debug.player_id_decimal === lost.checkpoint.debug.player_id_decimal &&
        after.checkpoint.debug.slot === lost.checkpoint.debug.slot &&
        !after.checkpoint.debug.ui_snapshot.some((node) => node.id === 'shell.connection_lost.root'),
    'connection-lost-reload', 'actual Reload widget did not produce a fresh healthy document with the retained identity/slot');
    evaluated.add('connection-lost-reload');
    return evaluated;
}

module.exports = { judgeSh1Lifecycle, judgeSh1ConflictOffer, product, visible, whitePixels };
