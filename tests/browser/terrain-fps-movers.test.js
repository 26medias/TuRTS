// turts-test-role: unit
'use strict';
const test = require('node:test'), assert = require('node:assert/strict');
const {moverIds, scriptWithMovers} = require('./terrain-fps-movers');
const place = (kind, slot, x, y) => `(unit: "skirmish:${kind}", owner_slot: ${slot}, pos: (x: ${x}.0, y: ${y}.0)),`;
const map = [place('soldier', 0, 20, 2), place('warrior', 0, 21, 20), place('soldier', 1, 22, 20), place('soldier', 0, 10, 20),
    ...[19, 24, 29, 34, 39, 44, 49].map(x => place('soldier', 0, x, 20))].join('\n');
test('the movers are six slot 0 soldiers of the highest rows, without the excluded ones and the west edge', () => {
    assert.deepEqual(moverIds(map, [5]), [6, 7, 8, 9, 10, 11]);
    assert.throws(() => moverIds(map, [5, 6, 7]), /no 6 free movers/);
});
test('the accepted commands stay, and the movers get alternating moves in tick order', () => {
    const base = [{tick: 30, slot: 1, envelope: {order: 'skirmish:order_attack', units: [3]}}];
    const script = scriptWithMovers(base, [6, 7]);
    assert.deepEqual(script.find(c => c.tick === 30), base[0]);
    assert.deepEqual(script.slice(0, 2).map(c => c.tick), [30, 100]);
    assert.deepEqual(script.filter(c => c.tick === 100 || c.tick === 200).map(c => c.envelope.target.Point.x), [18, 36]);
    assert.equal(script.at(-1).tick, 12000);
});
