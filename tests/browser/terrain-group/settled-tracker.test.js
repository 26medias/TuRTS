'use strict';
const test = require('node:test');
const assert = require('node:assert/strict');
const { SettledTracker } = require('./settled-tracker');
test('a settled unit may not move without an accepted new order', () => {
    const tracker = new SettledTracker();
    tracker.observe('worker', true, [2, 3]);
    assert.throws(() => tracker.observe('worker', true, [2, 4]), /displaced settled point/);
    tracker.begin(['worker']);
    assert.deepEqual(tracker.positions.get('worker'), [2, 3]);
    tracker.commit(['worker'], 'excavate');
    tracker.observe('worker', true, [2, 4]);
    assert.equal(tracker.positions.has('worker'), false);
});
test('a later move establishes a new protected settled point', () => {
    const tracker = new SettledTracker();
    tracker.observe('worker', true, [2, 3]);
    tracker.begin(['worker']);
    tracker.commit(['worker'], 'move');
    tracker.observe('worker', true, [5, 6]);
    assert.throws(() => tracker.observe('worker', true, [5, 7]), /displaced settled point/);
    assert.throws(() => tracker.commit(['other'], 'move'), /No pending order/);
});
