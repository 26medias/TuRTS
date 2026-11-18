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
module.exports = { SettledTracker };
