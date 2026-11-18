'use strict';
const test=require('node:test'),assert=require('node:assert/strict');
const {clickPointer}=require('./pointer-gesture');
test('move settles before the physical click press',async()=>{
 const events=[],page={mouse:{move:async(...xy)=>events.push(['move',...xy]),down:async(x)=>events.push(['down',x.button]),up:async(x)=>events.push(['up',x.button])}};
 await clickPointer(page,[12,34],'left',async()=>events.push(['pause']));
 assert.deepEqual(events,[['move',12,34],['pause'],['down','left'],['pause'],['up','left'],['pause']]);
});
