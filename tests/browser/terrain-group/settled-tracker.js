'use strict';
// 3 s at 20 Hz (Clara's proposals, not measurements).
const REST_TICKS = 60, RETURN_TICKS = 60;
const near = (list, at, bound) => list.some(p => Math.hypot(p[0] - at[0], p[1] - at[1]) <= bound);
// Settled-unit checks of the GM pointer campaign (Clara, 2026-11-24; vault/Decisions/2026-11-24 Movement rules V2 V3
// V5 V6.md). Both measure from the FIRST settle point of the unit after its last own order; a new order clears it.
// "Quiet" for a unit starts at the first tick with no own mover within two diameters (4 x radius) of the unit.
// (1) At rest (quiet for REST_TICKS or more) the unit is within 2 x radius of its settle point. It does not apply while
//     another body stands within one diameter of that point, except for the FIRST unit of its order to settle
//     (arrive first): that unit must always pass. Contact while units move is allowed (operator rule 4).
// (2) When the unit is one radius or more from its settle point, it is back within one radius at most RETURN_TICKS
//     after quiet starts. It does not apply while another body stands within one diameter of the settle point.
// `context` is {tick, ownMovers: [[x, y]], bodies: [[x, y]]} (the other bodies only).
class SettledTracker {
    constructor() {
        this.positions = new Map(); this.pending = new Set(); this.retired = new Set();
        this.quiet = new Map(); this.orderOf = new Map(); this.firstOf = new Map(); this.orders = 0;
    }
    begin(labels) { for (const label of labels) this.pending.add(label); }
    commit(labels, action) {
        const order = ++this.orders;
        for (const label of labels) {
            if (!this.pending.has(label)) throw Error('No pending order for ' + label);
            this.positions.delete(label);
            this.quiet.delete(label);
            this.orderOf.set(label, order);
            this.pending.delete(label);
            if (action === 'move') this.retired.delete(label); else this.retired.add(label);
        }
    }
    observe(label, idle, position, radius, context) {
        if (this.pending.has(label) || this.retired.has(label) || !idle) return;
        if (!(radius > 0)) throw Error('Settled unit needs its body radius: ' + label);
        if (!context) throw Error('Settled unit needs the tick, the own movers and the bodies: ' + label);
        const {tick, ownMovers, bodies} = context, home = this.positions.get(label), order = this.orderOf.get(label) ?? 0;
        if (!home) {
            this.positions.set(label, [...position]);
            if (!this.firstOf.has(order)) this.firstOf.set(order, label);
        }
        if (near(ownMovers, position, 4 * radius)) { this.quiet.delete(label); return; }
        if (!this.quiet.has(label)) this.quiet.set(label, tick);
        if (!home) return;
        const quietFor = tick - this.quiet.get(label), distance = Math.hypot(position[0] - home[0], position[1] - home[1]);
        const occupied = near(bodies, home, 2 * radius), first = this.firstOf.get(order) === label;
        const where = ' (' + distance.toFixed(3) + ' m from its settle point, quiet for ' + quietFor + ' ticks at tick ' + tick + ')';
        if (quietFor >= REST_TICKS && distance > 2 * radius && (first || !occupied))
            throw Error('Settled unit at rest is more than one body diameter from its settle point' + (first ? ' (first arrival of its order)' : '') + ': ' + label + where);
        if (quietFor > RETURN_TICKS && distance >= radius && !occupied)
            throw Error('Pushed settled unit did not go back within ' + RETURN_TICKS + ' ticks after the movers left: ' + label + where + ', bound ' + radius.toFixed(3) + ' m');
    }
}
class DistantOrderTracker {
    constructor() { this.original = null; this.start = null; this.moved = false; }
    observe(relative, order, position) {
        if (relative > 340 && !this.original) throw Error('Distant order not observed by tick 340');
        if (relative >= 1 && !this.original && order) {
            this.original = JSON.parse(JSON.stringify(order));
            this.start = [...position];
        }
        if (relative >= 20 && relative <= 340 && this.original) {
            if (JSON.stringify(order) !== JSON.stringify(this.original)) throw Error('Distant public order changed during local edits');
            this.moved ||= JSON.stringify(position) !== JSON.stringify(this.start);
        }
    }
}
module.exports = { SettledTracker, DistantOrderTracker };
