// turts-test-role: unit
'use strict';
// The one drag of a terrain work order under the operator's rule (2026-11-26): the drag start is the reference height.
const test=require('node:test'),assert=require('node:assert/strict');
const {dragFor}=require('./driver');
// A ridge of level 4 at x 30..33; a passage of level 0 at y 22..24.
const ridge=(x,y)=>x>=30&&x<34&&(y<22||y>=25)?4:0;
test('a corner at the level starts the drag and the area is unchanged',()=>{
    assert.deepEqual(dragFor({min:[29,20],max:[34,28]},0,ridge),{start:[29,20],end:[33,27],cells:{min:[29,20],max:[34,28]}});
});
test('an area of raised ground grows by one flat column and starts there',()=>{
    const drag=dragFor({min:[30,20],max:[34,28]},0,ridge);
    assert.deepEqual(drag.cells,{min:[29,20],max:[34,28]});assert.equal(ridge(...drag.start),0);
});
test('a fill of low ground starts on a raised row above it',()=>{
    // As GM4: the passage covers x 30..33 at y 20..27, and the fill column is x 31.
    const open=(x,y)=>x>=30&&x<34&&y>=20&&y<28?0:4;
    const drag=dragFor({min:[31,20],max:[32,28]},4,open);
    assert.equal(open(...drag.start),4);assert.deepEqual(drag.cells,{min:[31,19],max:[32,28]});
});
test('an added row or column that would need work is refused; no drag at all is an error',()=>{
    assert.throws(()=>dragFor({min:[31,5],max:[33,8]},0,ridge),/No drag gives this work order/);
});
