// turts-test-role: unit
'use strict';
// The body overlap rule of the GM pointer driver (operator rule 4 of 2026-11-23, FM3): two Idle bodies may not
// overlap; while at least one moves, the depth must stay below 25 % of the smaller radius. Planted cases.
const test=require('node:test'),assert=require('node:assert/strict');
const {overlapFault,physical}=require('./driver');
const body=(x,activity,radius=.5,owner=0)=>({pos:[x,10],radius,activity,owner});
test('moving: a 26 % overlap fails and a 24 % overlap passes (radii 0.5 and 0.5)',()=>{
    assert.match(overlapFault(body(10,'Moving'),body(10.87,'Moving')),/25 %/);
    assert.equal(overlapFault(body(10,'Moving'),body(10.88,'Moving')),null);
});
test('moving: the cap uses the smaller radius (radii 0.5 and 1.1)',()=>{
    assert.match(overlapFault(body(10,'Moving',1.1),body(11.47,'Idle')),/25 %/);
    assert.equal(overlapFault(body(10,'Moving',1.1),body(11.48,'Idle')),null);
});
test('moving: one moving unit and one Idle unit are under the moving cap',()=>{
    assert.equal(overlapFault(body(10,'Blocked'),body(10.9,'Idle')),null);
});
test('rest: any overlap of two Idle bodies fails, also one f32 step deep',()=>{
    assert.match(overlapFault(body(10,'Idle'),body(10.99,'Idle')),/Idle circles overlap/);
    const touch=Math.fround(11),below=touch-2**-20;
    assert.equal(overlapFault(body(10,'Idle'),body(touch,'Idle')),null);
    assert.match(overlapFault(body(10,'Idle'),body(below,'Idle')),/Idle circles overlap/);
});
// physical() on a flat 64 x 48 map: it reads the activity of each actor from the merged unit orders.
const flat=(x2,activity,bSlot=0)=>{
    const fixture={scene:{map:{override_patches:[]}},actors:[
        {label:'a',slot:0,profile:{radius:.5,max_step:1}},{label:'b',slot:bSlot,profile:{radius:.5,max_step:1}}]};
    const state={debug:{replicated_world:{terrain:{surfaces:[]},units:[{unit_id:1,pos:[10,10]},{unit_id:2,pos:[x2,10]}],
        unit_orders:[{unit:1,activity},{unit:2,activity}]}}};
    return [fixture,state,new Map([['a',1],['b',2]])];
};
test('physical: a planted 26 % overlap while moving fails, 20 % passes',()=>{
    assert.throws(()=>physical(...flat(10.87,'Moving')),/25 % or more of the smaller radius: b\/a \(Moving\/Moving\)/);
    assert.doesNotThrow(()=>physical(...flat(10.9,'Moving')));
});
test('physical: a planted overlap at rest fails, touching at rest passes',()=>{
    assert.throws(()=>physical(...flat(10.95,'Idle')),/Idle circles overlap: b\/a \(Idle\/Idle\)/);
    assert.doesNotThrow(()=>physical(...flat(11,'Idle')));
});
test('physical: a unit with no order row counts as Idle (the stricter rule)',()=>{
    const [fixture,state,bindings]=flat(10.95,'Moving');state.debug.replicated_world.unit_orders=[];
    assert.throws(()=>physical(fixture,state,bindings),/Idle circles overlap/);
});
test('owners: bodies of different owners never overlap, also while moving and one f32 step deep',()=>{
    assert.match(overlapFault(body(10,'Moving'),body(10.99,'Moving',.5,1)),/different owners overlap/);
    assert.match(overlapFault(body(10,'Moving'),body(Math.fround(11)-2**-20,'Idle',.5,1)),/different owners overlap/);
    assert.equal(overlapFault(body(10,'Moving'),body(11,'Moving',.5,1)),null);
});
test('physical: a planted 5 % overlap of two moving units of different owners fails',()=>{
    assert.throws(()=>physical(...flat(10.95,'Moving',1)),/different owners overlap: b\/a/);
    assert.doesNotThrow(()=>physical(...flat(10.95,'Moving',0)));
});
test('the record-all mode keeps each fault once and goes on; the normal mode throws at the first',()=>{
    const {faultList}=require('./driver'),all=faultList(true),first=faultList(false);
    all.at(5);all.check(()=>physical(...flat(10.95,'Idle'),all.check));all.at(6);all.check(()=>{throw Error('A move ended outside its goal area: a at [1.5,2] (area 3 m)');});
    all.at(7);all.check(()=>{throw Error('A move ended outside its goal area: a at [1.7,2] (area 3 m)');});all.check(()=>{throw Error('A move ended outside its goal area: b at [1,2] (area 3 m)');});
    assert.deepEqual(all.faults.map(f=>f.tick),[5,6,7]);assert.match(all.faults[0].fault,/Idle circles overlap/);
    assert.throws(()=>physical(...flat(10.95,'Idle'),first.check),/Idle circles overlap/);assert.equal(first.faults.length,0);
});
