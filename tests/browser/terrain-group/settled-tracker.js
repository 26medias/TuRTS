'use strict';
class SettledTracker {
    constructor() { this.positions = new Map(); this.pending = new Set(); this.retired = new Set(); }
    begin(labels) { for (const label of labels) this.pending.add(label); }
    commit(labels, action) {
        for (const label of labels) {
            if (!this.pending.has(label)) throw Error('No pending order for ' + label);
            this.positions.delete(label);
            this.pending.delete(label);
            if (action === 'move') this.retired.delete(label); else this.retired.add(label);
        }
    }
    // Settled-unit bound (Clara, 2026-11-24; vault/Decisions/2026-11-24 Movement rules V2 V3 V5 V6.md): a settled
    // unit with no new order of its own may be pushed or make room, but it fails when it is more than one body
    // diameter (2 x radius) from its settle point. A new order of that unit clears its point (begin/commit).
    observe(label, idle, position, radius) {
        if (this.pending.has(label) || this.retired.has(label) || !idle) return;
        if (!(radius > 0)) throw Error('Settled unit needs its body radius: ' + label);
        const old = this.positions.get(label);
        if (!old) { this.positions.set(label, [...position]); return; }
        const distance = Math.hypot(position[0] - old[0], position[1] - old[1]);
        if (distance > 2 * radius) throw Error('Settled unit moved more than one body diameter from its settle point: ' + label + ' (' + distance.toFixed(3) + ' m, bound ' + (2 * radius).toFixed(3) + ' m)');
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
