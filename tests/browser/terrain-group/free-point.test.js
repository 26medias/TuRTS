// turts-test-role: unit
'use strict';
// Free click point and cohort substitution of the GM pointer driver (Iris, 2026-11-24). Planted scenes with a simple
// camera: map x is screen x, height is screen up, map y is the depth (clip w); the camera stands at the origin.
const test=require('node:test'),assert=require('node:assert/strict');
const {freePoint,reachableIntent,selectActors}=require('./driver');
const m=new Array(16).fill(0);m[0]=1;m[5]=1;m[11]=1;m[14]=1;
const profile={radius:.4,pick_height:.68,max_step:0};
function world(units){
    return {debug:{slot:0,selection:[],replicated_world:{units:units.map(([id,x,y])=>({unit_id:id,pos:[x,y],owner_slot:0})),
        terrain:{anchors:units.map(([id])=>({unit:id,current:true,elevation:0})),surfaces:[]}}},
        scene:{clip_from_world:m,viewport_logical_rect:[0,0,1280,720],units:units.map(([id])=>({unit_id:id,pick_radius:.4}))},ui:[]};
}
const shape={Cuboid:{x:.6,y:.6,z:1}};
const fixture=labels=>({actors:labels.map(label=>({label,kind:label.split('.')[0],slot:0,profile,pick_shape:shape}))});
const bind=pairs=>new Map(pairs);
test('a body with nothing in front has a free point on its own column',()=>{
    const state=world([[1,0,10]]),free=freePoint(fixture(['small.0']),state,bind([['small.0',1]]),1);
    assert.ok(free);assert.ok(Math.abs(free.point[0]-640)<=0.6*19.2+1e-9,'the point is on the body: '+free.point);
});
test('a nearer body that covers the whole column leaves no free point',()=>{
    const state=world([[1,0,10],[2,0,9.5]]);
    assert.equal(freePoint(fixture(['small.0','small.1']),state,bind([['small.0',1],['small.1',2]]),1),null);
});
test('a nearer body that covers one side leaves a free point on the other side',()=>{
    const state=world([[1,0,10],[2,-.35,9.5]]),free=freePoint(fixture(['small.0','small.1']),state,bind([['small.0',1],['small.1',2]]),1);
    assert.ok(free);assert.ok(free.point[0]>640,'the free point is on the uncovered (right) side: '+free.point);
});
test('a body behind the intended body does not cover it',()=>{
    const state=world([[1,0,10],[2,0,10.6]]);
    assert.ok(freePoint(fixture(['small.0','small.1']),state,bind([['small.0',1],['small.1',2]]),1));
});
test('a covered one-unit order goes to the nearest free body of its kind and slot, and records the fact',()=>{
    const state=world([[1,0,10],[2,0,9.5],[3,3,10],[4,6,10]]),f=fixture(['small.0','small.1','small.2','large.0']);
    f.actors[3].kind='large';
    const facts=[],b=bind([['small.0',1],['small.1',2],['small.2',3],['large.0',4]]);
    const intent=reachableIntent(f,state,b,{actors:['small.0'],action:'excavate'},fact=>facts.push(fact));
    assert.deepEqual(intent.actors,['small.1']);assert.deepEqual(facts.map(x=>[x.label,x.substitute]),[['small.0','small.1']]);
    assert.deepEqual(reachableIntent(f,state,b,{actors:['small.0','small.2'],action:'move'},()=>{}).actors,['small.0','small.2']);
});
test('a covered one-unit order with no reachable body of its kind fails',()=>{
    const state=world([[1,0,10],[2,0,9.5]]),f=fixture(['small.0','large.0']);f.actors[1].kind='large';
    assert.throws(()=>reachableIntent(f,state,bind([['small.0',1],['large.0',2]]),{actors:['small.0']},()=>{}),/No body of the cohort can be clicked/);
});
// A page whose click always selects `picked`: the selection check after a click at a free point must still fail on a
// wrong pick, and pass on the right one.
function page(state,picked){
    const clicks=[];
    return {clicks,bringToFront:async()=>{},keyboard:{down:async()=>{},up:async()=>{}},
        mouse:{click:async(x,y)=>{clicks.push([x,y]);state.debug.selection=[picked];}},
        evaluate:async()=>JSON.parse(JSON.stringify(state))};
}
test('a planted wrong pick at a free point fails; the right pick passes',async()=>{
    const f=fixture(['small.0','small.1']),b=bind([['small.0',1],['small.1',2]]);
    const wrong=page(world([[1,0,10],[2,-.35,9.5]]),2);
    await assert.rejects(selectActors(wrong,f,b,['small.0'],null,300),/Physical body selection differs/);
    assert.equal(wrong.clicks.length,1);assert.ok(wrong.clicks[0][0]>640,'the click went to the free side');
    const right=page(world([[1,0,10],[2,-.35,9.5]]),1);
    await selectActors(right,f,b,['small.0'],null,300);
});
test('a body that moved under the click is aimed at again',async()=>{
    const f=fixture(['small.0']),state=world([[1,0,10]]),clicks=[];
    // The first click misses because the body walked on; the second selects it.
    const walking={bringToFront:async()=>{},keyboard:{down:async()=>{},up:async()=>{}},
        mouse:{click:async()=>{clicks.push(1);if(clicks.length===1)state.debug.replicated_world.units[0].pos=[1,10];else state.debug.selection=[1];}},
        evaluate:async()=>JSON.parse(JSON.stringify(state))};
    await selectActors(walking,f,bind([['small.0',1]]),['small.0'],null,300);
    assert.equal(clicks.length,2);
});
