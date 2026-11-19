'use strict';

// Adjacent intervals cover the whole sample, including a short final interval.
function movementIntervals(states, groupIds, stride = 3) {
  if (states.length < 2 || !Number.isInteger(stride) || stride < 1) return [];
  const intervals = [];
  for (let start = 0; start < states.length - 1; start += stride) {
    const end = Math.min(start + stride, states.length - 1);
    const a = states[start], b = states[end];
    if (!(b.at > a.at)) throw Error('Movement sample times must increase');
    const active = groupIds.filter(id => {
      const x = a.group_positions.find(p => p.unit === id);
      const y = b.group_positions.find(p => p.unit === id);
      return x && y && Math.hypot(x.position[0] - y.position[0], x.position[2] - y.position[2]) > .01;
    });
    intervals.push({from_ms: a.at - states[0].at, to_ms: b.at - states[0].at, active});
  }
  return intervals;
}
module.exports = {movementIntervals};
