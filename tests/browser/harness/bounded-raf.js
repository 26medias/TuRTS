// Runs as a trusted harness init script before the WASM application starts.
function installBoundedRafHold() {
    const nativeRequestAnimationFrame = window.requestAnimationFrame?.bind(window);
    const nativeCancelAnimationFrame = window.cancelAnimationFrame?.bind(window);
    if (!nativeRequestAnimationFrame || !nativeCancelAnimationFrame) return;

    let held = false;
    let heldCallbacks = [];
    let heldCount = 0;
    let releaseBatch = 0;
    const scheduled = new Map();

    window.requestAnimationFrame = callback => {
        const ticket = { callback, nativeId: null, canceled: false, returnedId: null };
        const id = nativeRequestAnimationFrame(timestamp => {
            ticket.nativeId = null;
            if (ticket.canceled) return;
            if (held) {
                heldCallbacks.push(ticket);
                heldCount += 1;
            } else {
                scheduled.delete(ticket.returnedId);
                callback(timestamp);
            }
        });
        ticket.nativeId = id;
        ticket.returnedId = id;
        scheduled.set(id, ticket);
        return id;
    };

    window.cancelAnimationFrame = id => {
        const ticket = scheduled.get(id);
        if (!ticket) return nativeCancelAnimationFrame(id);
        ticket.canceled = true;
        scheduled.delete(id);
        if (ticket.nativeId !== null) nativeCancelAnimationFrame(ticket.nativeId);
    };

    window.__turts_begin_bounded_input = () => {
        if (held) throw new Error("bounded input frame hold is already active");
        held = true;
        heldCallbacks = [];
        heldCount = 0;
    };

    window.__turts_end_bounded_input = () => {
        if (!held) return { held_callbacks: heldCount, released: false };
        held = false;
        const callbacks = heldCallbacks;
        heldCallbacks = [];
        releaseBatch += 1;
        for (const ticket of callbacks) {
            if (ticket.canceled) continue;
            ticket.nativeId = nativeRequestAnimationFrame(timestamp => {
                scheduled.delete(ticket.returnedId);
                if (!ticket.canceled) ticket.callback(timestamp);
            });
        }
        return { held_callbacks: heldCount, released: true, release_batch: releaseBatch };
    };
}

module.exports = { installBoundedRafHold };
