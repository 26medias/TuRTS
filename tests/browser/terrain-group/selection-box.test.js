'use strict';
const test=require('node:test'),assert=require('node:assert/strict');
const {exactOwnedBox}=require('./selection-box');
test('box selects the requested owned centers and ignores foreign centers',()=>{
 const p=[{id:1,owned:true,center:[10,10]},{id:2,owned:true,center:[20,20]},{id:3,owned:false,center:[15,15]}];
 assert.deepEqual(exactOwnedBox(p,[1,2]),[8,8,22,22]);
});
test('box refuses extra owned or missing units',()=>{
 const p=[{id:1,owned:true,center:[10,10]},{id:2,owned:true,center:[20,20]},{id:3,owned:true,center:[15,15]}];
 assert.equal(exactOwnedBox(p,[1,2]),null);assert.equal(exactOwnedBox(p,[1,4]),null);
});
