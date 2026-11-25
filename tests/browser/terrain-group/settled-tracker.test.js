'use strict';
const test = require('node:test');
const assert = require('node:assert/strict');
const { SettledTracker, DistantOrderTracker } = require('./settled-tracker');
test('a settled unit may shift up to one body diameter and fails beyond it', () => {
    const tracker = new SettledTracker();
    tracker.observe('worker', true, [2, 3], 0.5);
    tracker.observe('worker', true, [2, 3.99], 0.5);
    tracker.observe('worker', true, [3, 3], 0.5);
    assert.throws(() => tracker.observe('worker', true, [2, 4.01], 0.5), /more than one body diameter/);
    assert.throws(() => tracker.observe('worker', true, [2, 4], 0), /needs its body radius/);
});
test('the bound is measured from the first settle point, not from the last sample', () => {
    const tracker = new SettledTracker();
    tracker.observe('worker', true, [2, 3], 0.5);
    tracker.observe('worker', true, [2, 3.9], 0.5);
    assert.throws(() => tracker.observe('worker', true, [2, 4.8], 0.5), /more than one body diameter/);
});
test('a new order clears the settle point', () => {
    const tracker = new SettledTracker();
    tracker.observe('worker', true, [2, 3], 0.5);
    tracker.begin(['worker']);
    assert.deepEqual(tracker.positions.get('worker'), [2, 3]);
    tracker.commit(['worker'], 'excavate');
    tracker.observe('worker', true, [9, 9], 0.5);
    assert.equal(tracker.positions.has('worker'), false);
});
test('a later move establishes a new settle point with the same bound', () => {
    const tracker = new SettledTracker();
    tracker.observe('worker', true, [2, 3], 0.5);
    tracker.begin(['worker']);
    tracker.commit(['worker'], 'move');
    tracker.observe('worker', true, [5, 6], 0.5);
    tracker.observe('worker', true, [5, 6.9], 0.5);
    assert.throws(() => tracker.observe('worker', true, [5, 7.1], 0.5), /more than one body diameter/);
    assert.throws(() => tracker.commit(['other'], 'move'), /No pending order/);
});

const quiet = (tick, bodies = []) => ({tick, ownMovers: [], bodies});
test('return: a pushed settled unit that goes back within 60 quiet ticks passes', () => {
    const tracker = new SettledTracker();
    tracker.observe('u', true, [10, 10], 0.5, quiet(0));
    tracker.observe('u', true, [10.8, 10], 0.5, {tick: 5, ownMovers: [[11.5, 10]], bodies: [[11.5, 10]]});
    tracker.observe('u', true, [10.8, 10], 0.5, quiet(10));
    tracker.observe('u', true, [10.8, 10], 0.5, quiet(70));
    tracker.observe('u', true, [10.3, 10], 0.5, quiet(71));
    tracker.observe('u', true, [10.3, 10], 0.5, quiet(500));
});
test('return: a pushed settled unit that stays away more than 60 quiet ticks fails', () => {
    const tracker = new SettledTracker();
    tracker.observe('u', true, [10, 10], 0.5, quiet(0));
    tracker.observe('u', true, [10.8, 10], 0.5, quiet(10));
    tracker.observe('u', true, [10.8, 10], 0.5, quiet(70));
    assert.throws(() => tracker.observe('u', true, [10.8, 10], 0.5, quiet(71)), /did not go back within 60 ticks/);
});
test('return: the quiet time starts again when an own mover comes within two diameters', () => {
    const tracker = new SettledTracker();
    tracker.observe('u', true, [10, 10], 0.5, quiet(0));
    tracker.observe('u', true, [10.8, 10], 0.5, quiet(10));
    tracker.observe('u', true, [10.8, 10], 0.5, {tick: 60, ownMovers: [[12.7, 10]], bodies: []});
    tracker.observe('u', true, [10.8, 10], 0.5, quiet(61));
    tracker.observe('u', true, [10.8, 10], 0.5, quiet(121));
    assert.throws(() => tracker.observe('u', true, [10.8, 10], 0.5, quiet(122)), /did not go back/);
});
test('return: no check while another body stands within one diameter of the settle point', () => {
    const tracker = new SettledTracker();
    tracker.observe('u', true, [10, 10], 0.5, quiet(0));
    tracker.observe('u', true, [10.8, 10], 0.5, quiet(10, [[9.5, 10]]));
    tracker.observe('u', true, [10.8, 10], 0.5, quiet(200, [[9.5, 10]]));
    assert.throws(() => tracker.observe('u', true, [10.8, 10], 0.5, quiet(201, [[8.9, 10]])), /did not go back/);
});
test('return: measured from the FIRST settle point, so small steps that add up are caught', () => {
    const tracker = new SettledTracker();
    tracker.observe('u', true, [10, 10], 0.5, quiet(0));
    for (const [i, x] of [10.2, 10.4, 10.6].entries()) tracker.observe('u', true, [x, 10], 0.5, {tick: 1 + i, ownMovers: [[x + 1, 10]], bodies: []});
    tracker.observe('u', true, [10.6, 10], 0.5, quiet(4));
    assert.throws(() => tracker.observe('u', true, [10.6, 10], 0.5, quiet(65)), /did not go back/);
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
