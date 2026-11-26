'use strict';
const assert = require('node:assert/strict');
const {movementIntervals} = require('./terrain-fps-intervals');

const frame = (at, a, b) => ({at, positions: [
  {unit: 1, position: [a, 0, 0]}, {unit: 2, position: [b, 0, 0]},
]});
const frames = [0, 500, 1000, 1500, 2000, 2200].map(t => frame(t, t / 1000, t / 1000));
const ok = movementIntervals(frames, [1, 2]);
assert.deepEqual(ok.map(i => [i.from_ms, i.to_ms]), [[0, 1500], [1500, 2200]]);
assert.ok(ok.every(i => i.active.length === 2));
const stopped = frames.map(f => frame(f.at, Math.min(f.at, 1500) / 1000, f.at / 1000));
assert.deepEqual(movementIntervals(stopped, [1, 2]).at(-1).active, [2]);
assert.deepEqual(movementIntervals([frames[0]], [1, 2]), []);
assert.throws(() => movementIntervals([frames[0], frames[0]], [1, 2]));
console.log('terrain FPS movement intervals: pass');
