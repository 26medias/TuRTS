// turts-test-role: pure
'use strict';
// 3 s at 20 Hz (Clara's proposals, not measurements).
const REST_TICKS = 60, RETURN_TICKS = 60;
const near = (list, at, bound) => list.some(p => Math.hypot(p[0] - at[0], p[1] - at[1]) <= bound);
// Settled-unit checks of the GM pointer campaign (Clara, 2026-11-24; vault/Decisions/2026-11-24 Movement rules V2 V3
// V5 V6.md). Both measure from the FIRST settle point of the unit after its last own order; a new order clears it.
// "Quiet" for a unit starts at the first tick with no own mover within two diameters (4 x radius) of the unit.
// (1) At rest (quiet for REST_TICKS or more) the FIRST unit to settle in each crowd is within 2 x radius of its settle
//     point, with no exception (arrive first). A crowd is one owner and one goal point: two orders to one point make one
//     crowd. The other members of a crowd have no per-unit bound; the crowd checks judge them (Clara, 2026-11-24, after
//     Kenji's rerun). Contact while units move is allowed (operator rule 4).
// (2) RECORD ONLY (Iris, 2026-11-26; the requirement moved to T-59): the unit is back within one radius at most
//     RETURN_TICKS after the latest of three events: the quiet time starts, the settle point becomes free, or the unit
//     goes one radius or more from it. A point is free when no other body has its centre nearer to it than the sum of
//     the two radii (Clara, 2026-11-26). A miss goes into `records` (one for each due stretch); it fails nothing.
// `context` is {tick, ownMovers: [[x, y]], bodies: [[x, y, radius]]} (the other bodies only).
class SettledTracker {
    constructor() {
        this.positions = new Map(); this.pending = new Set(); this.retired = new Set();
        this.quiet = new Map(); this.crowdOf = new Map(); this.firstOf = new Map();
        this.due = new Map(); // The first tick of the current stretch in which a return is due.
        this.reported = new Map(); // The `due` tick of the stretch that is in `records` already.
        this.records = [];
    }
    begin(labels) { for (const label of labels) this.pending.add(label); }
    // `crowd` names the crowd of a move order: its owner and its goal point.
    commit(labels, action, crowd) {
        for (const label of labels) {
            if (!this.pending.has(label)) throw Error('No pending order for ' + label);
            this.positions.delete(label);
            this.quiet.delete(label);
            this.due.delete(label);
            this.crowdOf.set(label, crowd ?? null);
            this.pending.delete(label);
            if (action === 'move') this.retired.delete(label); else this.retired.add(label);
        }
    }
    observe(label, idle, position, radius, context) {
        if (this.pending.has(label) || this.retired.has(label) || !idle) return;
        if (!(radius > 0)) throw Error('Settled unit needs its body radius: ' + label);
        if (!context) throw Error('Settled unit needs the tick, the own movers and the bodies: ' + label);
        const {tick, ownMovers, bodies} = context, home = this.positions.get(label), crowd = this.crowdOf.get(label) ?? null;
        if (!home) {
            this.positions.set(label, [...position]);
            if (crowd !== null && !this.firstOf.has(crowd)) this.firstOf.set(crowd, label);
        }
        if (near(ownMovers, position, 4 * radius)) { this.quiet.delete(label); this.due.delete(label); return; }
        if (!this.quiet.has(label)) this.quiet.set(label, tick);
        if (!home) return;
        const quietFor = tick - this.quiet.get(label), distance = Math.hypot(position[0] - home[0], position[1] - home[1]);
        const occupied = bodies.some(body => {
            if (!(body[2] > 0)) throw Error('Each body needs its radius: ' + label);
            return Math.hypot(body[0] - home[0], body[1] - home[1]) < radius + body[2];
        }), first = crowd !== null && this.firstOf.get(crowd) === label;
        const where = ' (' + distance.toFixed(3) + ' m from its settle point, quiet for ' + quietFor + ' ticks at tick ' + tick + ')';
        if (first && quietFor >= REST_TICKS && distance > 2 * radius)
            throw Error('First unit of its crowd at rest is more than one body diameter from its settle point: ' + label + where);
        if (distance < radius || occupied) { this.due.delete(label); return; }
        if (!this.due.has(label)) this.due.set(label, tick);
        const dueFor = tick - this.due.get(label);
        if (dueFor > RETURN_TICKS && this.reported.get(label) !== this.due.get(label)) {
            this.reported.set(label, this.due.get(label));
            this.records.push({label, tick, due_since: this.due.get(label), quiet_for: quietFor, distance: +distance.toFixed(3), radius: +radius.toFixed(3)});
        }
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
