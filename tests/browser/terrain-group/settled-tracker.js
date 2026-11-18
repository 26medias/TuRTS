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
    observe(label, idle, position) {
        if (this.pending.has(label) || this.retired.has(label) || !idle) return;
        const old = this.positions.get(label);
        if (old && JSON.stringify(old) !== JSON.stringify(position)) throw Error('Later arrival displaced settled point: ' + label);
        this.positions.set(label, [...position]);
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
