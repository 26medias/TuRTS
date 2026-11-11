// turts-test-role: pure
'use strict';

function qualifyNavigationReceipt({ archives, entry, traceStart, callIndex, afterEpoch, nextDocument }) {
    const need = (condition, message) => { if (!condition) throw new Error(`HARNESS navigation receipt: ${message}`); };
    need(entry.call === 'click' && Number.isSafeInteger(afterEpoch) && afterEpoch > entry.document_epoch,
        'actual click must advance the tracked document epoch');
    need(Array.isArray(archives) && archives.length <= 8 && Number.isFinite(traceStart.document_time_origin) &&
        Number.isFinite(nextDocument?.document_time_origin) &&
        nextDocument.document_time_origin !== traceStart.document_time_origin &&
        nextDocument.document_url === traceStart.document_url,
    'new document must have a distinct actual time origin at the same URL');
    const archived = archives.find((item) => item.active_call?.call_index === callIndex &&
        item.active_call.call === entry.call && JSON.stringify(item.active_call.args) === JSON.stringify(entry.args) &&
        item.document_url === traceStart.document_url && item.document_time_origin === traceStart.document_time_origin &&
        Number.isFinite(item.wall_time_ms) && item.wall_time_ms >= entry.wall_time_ms);
    need(archived && Array.isArray(archived.actual) && archived.actual.length <= 4096 &&
        Array.isArray(archived.downstream) && archived.downstream.length <= 4096 &&
        Array.isArray(archived.contextmenu) && archived.contextmenu.length <= 64,
    'exact prior-document call archive is missing, foreign, or oversized');
    for (const name of ['actual', 'downstream', 'contextmenu']) {
        need(Number.isSafeInteger(traceStart[name]) && traceStart[name] >= 0 && traceStart[name] <= archived[name].length,
            'prior observer offset is invalid');
    }
    const actual = archived.actual.slice(traceStart.actual);
    need(actual.some((event) => event.type === 'pointerdown' && entry.event_seq.includes(event.seq) &&
        event.canvas_target === true && event.target === 'CANVAS' &&
        Number.isFinite(event.client_x) && Number.isFinite(event.client_y) &&
        Math.abs(event.client_x - entry.args[0]) < 0.01 && Math.abs(event.client_y - entry.args[1]) < 0.01 &&
        event.wall_time_ms >= entry.wall_time_ms && event.wall_time_ms <= archived.wall_time_ms),
    'archive lacks the real sequence-bound click at its actual canvas point');
    return {
        trace: { actual, downstream: archived.downstream.slice(traceStart.downstream),
            contextmenu: archived.contextmenu.slice(traceStart.contextmenu), contextmenu_overflow: archived.contextmenu_overflow },
        navigation: { document_epoch_before: entry.document_epoch, document_epoch_after: afterEpoch,
            document_time_origin_before: archived.document_time_origin,
            document_time_origin_after: nextDocument.document_time_origin, archived_wall_time_ms: archived.wall_time_ms },
    };
}

module.exports = { qualifyNavigationReceipt };
