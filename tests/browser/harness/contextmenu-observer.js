// turts-test-role: pure
// Runs as a document-start init script in the browser context. Each receipt
// snapshots event and input identity during dispatch; only defaultPrevented is
// read later, after all dispatch listeners have run.
function installContextMenuObserver() {
    window.__turts_io_contextmenu_events = [];
    window.__turts_io_contextmenu_overflow = false;
    document.addEventListener("contextmenu", (event) => {
        if (window.__turts_io_contextmenu_events.length >= 512) {
            window.__turts_io_contextmenu_overflow = true;
            return;
        }
        const activeCall = window.__turts_io_active_call;
        const activeRightClick = activeCall?.call === "click" && activeCall.args?.[2] === "right";
        const target = event.target;
        const entry = {
            seq: Number.isSafeInteger(window.__turts_io_seq) ? window.__turts_io_seq : null,
            source_call_index: activeRightClick ? activeCall.call_index : null,
            target: target?.tagName || null,
            target_id: typeof target?.id === "string" && target.id ? target.id : null,
            canvas_target: Boolean(document.querySelector("canvas")?.contains(target)),
            wall_time_ms: Date.now(),
            cancelable: event.cancelable,
            default_prevented: null,
        };
        window.__turts_io_contextmenu_events.push(entry);
        setTimeout(() => { entry.default_prevented = Boolean(event.defaultPrevented); }, 0);
    }, true);
}

module.exports = { installContextMenuObserver };
