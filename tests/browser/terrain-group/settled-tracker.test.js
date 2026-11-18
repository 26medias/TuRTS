'use strict';
const test = require('node:test');
const assert = require('node:assert/strict');
const { SettledTracker, DistantOrderTracker } = require('./settled-tracker');
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

test('distant order must be admitted by tick 340 and stay unchanged', () => {
    const tracker = new DistantOrderTracker();
    tracker.observe(20, null, [4, 46]);
    const order = { order: 'quarry:order_move', target: [22, 2] };
    tracker.observe(21, order, [4, 46]);
    tracker.observe(22, order, [4.1, 45.9]);
    assert.equal(tracker.moved, true);
    assert.throws(() => tracker.observe(23, null, [4.1, 45.9]), /order changed/);
    const missing = new DistantOrderTracker();
    assert.throws(() => missing.observe(341, null, [4, 46]), /not observed by tick 340/);
});
