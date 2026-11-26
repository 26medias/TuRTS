// Real GM1-GM4 pointer/key input and public outcome checks. No state/command injection.
'use strict';
const assert=(ok,message)=>{if(!ok)throw Error(message);};
const {SettledTracker,DistantOrderTracker}=require('./settled-tracker');
const {exactOwnedBox}=require('./selection-box');
const sleep=ms=>new Promise(resolve=>setTimeout(resolve,ms));
// Hold both sides of a gesture across frames. A zero-delay press/release can
// disappear between Bevy input polls, even when hover picked the correct body.
const PRESS_MS=80;
// Xvfb has one real pointer shared by the peer windows. Activate the target
// before CDP input so Chrome delivers the native pointer position before down.
async function activatePage(page){await page.bringToFront();await sleep(150);}
async function click(page,point,button='left'){
    await activatePage(page);
    await page.mouse.click(...point,{button,delay:PRESS_MS});
    await sleep(PRESS_MS);
}
const same=(a,b)=>JSON.stringify(a)===JSON.stringify(b);
const world=state=>state.debug.replicated_world;
const row=(state,id)=>world(state).unit_orders.find(order=>order.unit===id);
const idle=(state,id)=>{const order=row(state,id);return order&&order.activity==='Idle'&&order.current===null&&!order.queued.length&&!order.last_failure;};
function project(scene,[x,y,height=0]) {
    const m=scene.clip_from_world,v=[x,height,y,1],clip=[0,1,2,3].map(r=>v.reduce((sum,n,c)=>sum+m[c*4+r]*n,0));
    assert(clip[3]>0,'Pointer projection is behind camera');
    const [left,top,width,depth]=scene.viewport_logical_rect;
    const point=[left+(clip[0]/clip[3]+1)*width/2,top+(1-clip[1]/clip[3])*depth/2];
    assert(point[0]>0&&point[0]<1280&&point[1]>0&&point[1]<720,'Frozen input outside viewport: '+point);
    return point;
}
async function read(page) {
    return page.evaluate(async()=>{
        if(typeof turts_debug_request!=='function'||typeof turts_scene!=='function')return null;
        const debug=await turts_debug_request(),scene=turts_scene();
        return debug&&scene?{debug:JSON.parse(debug),scene:JSON.parse(scene),ui:typeof turts_ui==='function'?JSON.parse(turts_ui()):[]}:null;
    });
}
async function wait(page,predicate,message,timeout=90000) {
    const end=Date.now()+timeout;
    while(Date.now()<end){const state=await read(page);if(state&&predicate(state))return state;await sleep(50);}
    throw Error(message);
}
function contact(state,id) {
    const anchor=world(state).terrain.anchors.find(a=>a.unit===id&&a.current);
    assert(anchor,'Owned body lacks permitted current contact: '+id);return anchor.elevation*.25;
}
// Free click point (Iris, 2026-11-24). The client picks the nearest body hit along the click ray, using the declared
// pick shape of each unit (crates/turts_interaction/src/smart.rs, picking.rs). In a packed crowd a nearer unit can
// cover the intended body. The driver casts the same ray for each candidate point, with a margin on both sides: the
// intended body is the cylinder inside its shape's footprint, every other body the cylinder round it. A point is free
// when the intended body is the nearest hit there and at the four pixels round it. The selection check after the click
// is unchanged: a wrong pick still fails.
function invert4(m){
    const a=[0,1,2,3].map(r=>[0,1,2,3].map(c=>m[c*4+r])),inv=[0,1,2,3].map(r=>[0,1,2,3].map(c=>r===c?1:0));
    for(let c=0;c<4;c++){
        let p=c;for(let r=c+1;r<4;r++)if(Math.abs(a[r][c])>Math.abs(a[p][c]))p=r;
        assert(Math.abs(a[p][c])>1e-12,'Camera matrix is singular');
        [a[c],a[p]]=[a[p],a[c]];[inv[c],inv[p]]=[inv[p],inv[c]];
        const d=a[c][c];for(let k=0;k<4;k++){a[c][k]/=d;inv[c][k]/=d;}
        for(let r=0;r<4;r++)if(r!==c){const f=a[r][c];for(let k=0;k<4;k++){a[r][k]-=f*a[c][k];inv[r][k]-=f*inv[c][k];}}
    }
    return inv;
}
function screen(scene,[x,y,height]){const m=scene.clip_from_world,v=[x,height,y,1],c=[0,1,2,3].map(r=>v.reduce((sum,n,k)=>sum+m[k*4+r]*n,0)),[left,top,width,depth]=scene.viewport_logical_rect;return c[3]>0?[left+(c[0]/c[3]+1)*width/2,top+(1-c[1]/c[3])*depth/2]:null;}
// The scene ray through a page point: origin and direction in scene coordinates (x, height, map y).
function screenRay(scene,inverse,[px,py]){
    const [left,top,width,depth]=scene.viewport_logical_rect,nx=(px-left)/width*2-1,ny=1-(py-top)/depth*2;
    const at=z=>{const w=inverse.map(row=>row[0]*nx+row[1]*ny+row[2]*z+row[3]);return [w[0]/w[3],w[1]/w[3],w[2]/w[3]];};
    const near=at(1),far=at(.5);return {origin:near,direction:far.map((n,k)=>n-near[k])};
}
// Ray distance (in direction units) to a vertical cylinder of `radius` and `height` standing at map `pos` on `base`.
function cylinderHit({origin,direction},pos,base,radius,height){
    const o=[origin[0]-pos[0],origin[1]-base,origin[2]-pos[1]],d=direction;let best=null;
    const keep=t=>{if(t>=0&&(best===null||t<best))best=t;};
    const a=d[0]*d[0]+d[2]*d[2],b=o[0]*d[0]+o[2]*d[2],c=o[0]*o[0]+o[2]*o[2]-radius*radius,disc=b*b-a*c;
    if(a>1e-12&&disc>=0)for(const t of [(-b-Math.sqrt(disc))/a,(-b+Math.sqrt(disc))/a]){const y=o[1]+t*d[1];if(y>=0&&y<=height)keep(t);}
    if(Math.abs(d[1])>1e-12)for(const level of [0,height]){const t=(level-o[1])/d[1],x=o[0]+t*d[0],z=o[2]+t*d[2];if(x*x+z*z<=radius*radius)keep(t);}
    return best;
}
// Inner (intended body) or outer (other bodies) cylinder of a declared pick shape.
function pickCylinder(shape,inner){
    if(shape?.Cuboid){const {x,y,z}=shape.Cuboid;return {radius:inner?Math.min(x,y)/2:Math.hypot(x,y)/2,height:z};}
    if(shape?.Capsule)return {radius:shape.Capsule.radius,height:shape.Capsule.height};
    return null;
}
function freePoint(fixture,state,bindings,id){
    const scene=state.scene,units=world(state).units,byId=new Map([...bindings].map(([l,v])=>[v,fixture.actors.find(a=>a.label===l)]));
    const actor=byId.get(id),unit=units.find(u=>u.unit_id===id);assert(unit&&actor,'Free point needs a bound live actor: '+id);
    const own=pickCylinder(actor.pick_shape,true);assert(own,'Free point needs the declared pick shape of '+actor.label);
    const base=contact(state,id),inverse=invert4(scene.clip_from_world),[left,top,width,depth]=scene.viewport_logical_rect;
    const others=units.filter(u=>u.unit_id!==id).map(u=>{const a=byId.get(u.unit_id),c=pickCylinder(a?.pick_shape,false)??{radius:u.pick_radius??1,height:1.6};
        const anchor=world(state).terrain.anchors.find(x=>x.unit===u.unit_id&&x.current);return {pos:u.pos,base:anchor?anchor.elevation*.25:0,...c};});
    const margin=point=>{const ray=screenRay(scene,inverse,point),mine=cylinderHit(ray,unit.pos,base,own.radius,own.height);if(mine===null)return -Infinity;
        const theirs=Math.min(Infinity,...others.map(o=>cylinderHit(ray,o.pos,o.base,o.radius,o.height)).filter(t=>t!==null));return theirs-mine;};
    const centre=screen(scene,[unit.pos[0],unit.pos[1],base+own.height/2]),side=screen(scene,[unit.pos[0]+own.radius,unit.pos[1],base+own.height/2]);
    if(!centre)return null;
    const halfWidth=side?Math.abs(side[0]-centre[0]):0;let best=null;
    for(const f of [.25,.5,.75])for(const dx of [-.6,-.3,0,.3,.6]){
        const c=screen(scene,[unit.pos[0],unit.pos[1],base+f*own.height]);if(!c)continue;
        const point=[c[0]+dx*halfWidth,c[1]];
        if(point[0]<=left+1||point[0]>=left+width-1||point[1]<=top+1||point[1]>=top+depth-1)continue;
        const clearance=Math.min(...[[0,0],[1,0],[-1,0],[0,1],[0,-1]].map(([x,y])=>margin([point[0]+x,point[1]+y])));
        if(clearance>0&&(!best||clearance>best.clearance))best={point,clearance};
    }
    return best;
}
function actorBindings(fixture,state) {
    const result=new Map();
    for(const actor of fixture.actors){
        const matches=world(state).units.filter(u=>u.content_id===actor.content&&u.owner_slot===actor.slot&&u.pos.every((n,i)=>Math.fround(n)===Math.fround(actor.position[i])));
        assert(matches.length===1,'Frozen physical start does not identify one actor: '+actor.label);
        result.set(actor.label,matches[0].unit_id);
    }
    return result;
}
function initialElevation(fixture,[x,y]) {
    const map=fixture.scene.map;
    if(map.override_patches){for(const p of map.override_patches)if(p.min[0]<=x&&x<p.max[0]&&p.min[1]<=y&&y<p.max[1])return p.elevation;return 0;}
    return x>=30&&x<34&&(y<map.passage_y[0]||y>=map.passage_y[1])?4:0;
}
function elevation(fixture,state,x,y) {
    const surface=world(state).terrain.surfaces.find(r=>r.cell.x===x&&r.cell.y===y);
    if(surface)assert(surface.current===true,'AllVisible fixture supplied stale terrain');
    return surface?.elevation??initialElevation(fixture,[x+.5,y+.5]);
}
function permittedTop(fixture,state,point){return elevation(fixture,state,Math.floor(point[0]),Math.floor(point[1]))*.25;}
async function selectActors(page,fixture,bindings,labels,recordClick,timeout=90000) {
    if(fixture.scene_name==='GM3-opposing-large'){
        const state=await read(page),wanted=labels.map(label=>bindings.get(label));
        const units=world(state).units;
        const points=state.scene.units.map(unit=>({id:unit.unit_id,center:unit.ground_screen_center,owned:units.find(u=>u.unit_id===unit.unit_id)?.owner_slot===state.debug.slot}));
        const box=exactOwnedBox(points,wanted);assert(box,'Requested cohort has no exact owned selection box');
        await activatePage(page);
        await page.mouse.move(box[0],box[1]);await page.mouse.down();await sleep(PRESS_MS);
        await page.mouse.move(box[2],box[3],{steps:8});await sleep(PRESS_MS);await page.mouse.up();await sleep(PRESS_MS);
        await wait(page,s=>s.debug.selection.length===wanted.length&&wanted.every(id=>s.debug.selection.includes(id)),'Physical box selection differs from frozen cohort');
        return;
    }
    // Selection is performed at actual current poses. Shift is held only for additive picks.
    try{for(let i=0;i<labels.length;i++){
        const state=await read(page),actor=fixture.actors.find(a=>a.label===labels[i]),id=bindings.get(labels[i]);
        const unit=world(state).units.find(u=>u.unit_id===id);
        assert(unit&&unit.owner_slot===state.debug.slot,'Selection requires owned live body');
        if(i===1){await page.keyboard.down('Shift');await sleep(PRESS_MS);}
        const free=freePoint(fixture,state,bindings,id);assert(free,'No free click point on the intended body: '+labels[i]);
        const point=free.point;
        await click(page,point);
        if(recordClick){const after=await read(page);recordClick({label:labels[i],id,point,tick:world(after).tick,focus:after.debug.focus,selection:after.debug.selection,frames:after.debug.frames?.recent?.map(f=>({frame_index:f.frame_index,tick:f.tick,selection:f.selection,named_dispatches:f.named_dispatches}))});}
    }}finally{await page.keyboard.up('Shift');}
    const wanted=labels.map(label=>bindings.get(label));
    await wait(page,s=>s.debug.selection.length===wanted.length&&wanted.every(id=>s.debug.selection.includes(id)),'Physical body selection differs from frozen cohort',timeout);
}
// A one-unit order whose body no click can reach in a packed crowd goes to the nearest body of the same kind and slot
// that has a free point (Iris, 2026-11-24). The covered body is a reported fact; with no reachable body the case fails.
function reachableIntent(fixture,state,bindings,intent,record){
    if(intent.actors.length!==1)return intent;
    const label=intent.actors[0],actor=fixture.actors.find(a=>a.label===label);
    if(freePoint(fixture,state,bindings,bindings.get(label)))return intent;
    const at=world(state).units.find(u=>u.unit_id===bindings.get(label)).pos;
    const others=fixture.actors.filter(a=>a.label!==label&&a.kind===actor.kind&&a.slot===actor.slot).map(a=>({label:a.label,pos:world(state).units.find(u=>u.unit_id===bindings.get(a.label))?.pos})).filter(a=>a.pos).sort((a,b)=>Math.hypot(a.pos[0]-at[0],a.pos[1]-at[1])-Math.hypot(b.pos[0]-at[0],b.pos[1]-at[1]));
    const substitute=others.find(a=>freePoint(fixture,state,bindings,bindings.get(a.label)));
    assert(substitute,'No body of the cohort can be clicked: '+label+' and '+others.length+' bodies of its kind are covered');
    record({tick:world(state).tick,label,substitute:substitute.label});
    return {...intent,actors:[substitute.label]};
}
// The one drag that gives the work order of `cells` ({min,max}) at `level` (the operator's rule: the drag start is the
// reference height). A corner of the area at the level starts the drag. Else the area grows by one row or column whose
// cells are all at the level (they need no work), and the drag starts at a corner of it. `levelOf(x,y)` is the level.
function dragFor(cells,level,levelOf){
    const grow=[[0,0,0,0],[-1,0,0,0],[0,-1,0,0],[0,0,1,0],[0,0,0,1]];
    for(const [left,top,right,bottom] of grow){
        const rect={min:[cells.min[0]+left,cells.min[1]+top],max:[cells.max[0]+right,cells.max[1]+bottom]};
        const added=[];for(let y=rect.min[1];y<rect.max[1];y++)for(let x=rect.min[0];x<rect.max[0];x++)if(x<cells.min[0]||x>=cells.max[0]||y<cells.min[1]||y>=cells.max[1])added.push([x,y]);
        if(added.some(([x,y])=>levelOf(x,y)!==level))continue;
        const [x0,y0,x1,y1]=[rect.min[0],rect.min[1],rect.max[0]-1,rect.max[1]-1];
        for(const [start,end] of [[[x0,y0],[x1,y1]],[[x1,y0],[x0,y1]],[[x0,y1],[x1,y0]],[[x1,y1],[x0,y0]]])
            if(levelOf(...start)===level)return {start,end,cells:rect};
    }
    throw Error('No drag gives this work order: '+JSON.stringify({cells,level}));
}
async function issue(page,fixture,bindings,intent,preselected=false) {
    // Terrain work is aimed at the tops that the replica shows, and its actor is picked
    // at its replica position. Earlier work of the same actors must be complete first,
    // or the actor moves and a top changes between aim and press.
    if(intent.action!=='move')await wait(page,s=>intent.actors.every(label=>!row(s,bindings.get(label))?.current?.target?.TerrainWork),'Earlier terrain work of the actors did not finish');
    if(!preselected)await selectActors(page,fixture,bindings,intent.actors);
    if(intent.queue==='Append'){await page.keyboard.down('Shift');await sleep(PRESS_MS);}
    try{
        if(intent.action==='hold'){
            const order=fixture.package+':order_hold';
            const state=await wait(page,s=>s.ui.some(w=>w.visible&&w.enabled&&w.widget_action?.id===order),'Hold button absent');
            const button=state.ui.find(w=>w.visible&&w.enabled&&w.widget_action?.id===order),[x,y,w,h]=button.rect;
            await click(page,[x+w/2,y+h/2]);
        }else if(intent.action==='move'){
            // A smart right-click on the blocker's own body has no Ground
            // action. The ordinary Move button targets the same ground point.
            if(intent.tick===0&&intent.actors.length===1&&intent.actors[0]==='blocker'){
                const order=fixture.package+':order_move';
                const state=await wait(page,s=>s.ui.some(w=>w.visible&&w.enabled&&w.widget_action?.id===order),'Move button absent');
                const button=state.ui.find(w=>w.visible&&w.enabled&&w.widget_action?.id===order),[x,y,w,h]=button.rect;
                await click(page,[x+w/2,y+h/2]);
                const target=await read(page);await click(page,project(target.scene,[...intent.point,permittedTop(fixture,target,intent.point)]));
            }else{
                const state=await read(page);await click(page,project(state.scene,[...intent.point,permittedTop(fixture,state,intent.point)]),'right');
            }
        }else{
            const tool=fixture.package+':'+fixture.bindings[intent.action==='fill'?'fill':'dig'];
            let state=await wait(page,s=>s.ui.some(w=>w.visible&&w.enabled&&w.widget_action?.tool===tool),'Bound public TerrainTools button absent');
            const button=state.ui.find(w=>w.visible&&w.enabled&&w.widget_action?.tool===tool),[x,y,w,h]=button.rect;
            await click(page,[x+w/2,y+h/2]);
            // The player's gesture (2026-11-26): the tool button, then a drag that starts at the target level.
            state=await read(page);const drag=dragFor(intent.cells,intent.target_elevation,(x,y)=>elevation(fixture,state,x,y));
            const a=drag.start.map(n=>n+.5),b=drag.end.map(n=>n+.5);
            await activatePage(page);
            await page.mouse.move(...project(state.scene,[...a,permittedTop(fixture,state,a)]));await page.mouse.down();await sleep(PRESS_MS);
            await page.mouse.move(...project(state.scene,[...b,permittedTop(fixture,state,b)]),{steps:8});await sleep(PRESS_MS);await page.mouse.up();await sleep(PRESS_MS);
            return drag.cells;
        }
    }finally{await page.keyboard.up('Shift');}
}
// Exact f32 endpoint circle predicates. No epsilon conceals a one-ULP overlap.
const SCALE=33554432;
function exact(n){const scaled=Math.fround(n)*SCALE;assert(Number.isSafeInteger(scaled),'Exact circle coordinate outside frozen domain');return BigInt(scaled);}
const square=x=>x*x;
function bodySupport(fixture,state,actor,unit){
    const [x,y]=unit.pos.map(exact),r=exact(actor.profile.radius),size=BigInt(SCALE),heights=[];
    assert(x>=r&&y>=r&&x+r<=64n*size&&y+r<=48n*size,'Complete body leaves map: '+actor.label);
    for(let cy=Math.max(0,Math.floor(unit.pos[1]-actor.profile.radius));cy<Math.min(48,Math.ceil(unit.pos[1]+actor.profile.radius));cy++){
        for(let cx=Math.max(0,Math.floor(unit.pos[0]-actor.profile.radius));cx<Math.min(64,Math.ceil(unit.pos[0]+actor.profile.radius));cx++){
            const lowX=BigInt(cx)*size,highX=lowX+size,lowY=BigInt(cy)*size,highY=lowY+size;
            const dx=x<lowX?lowX-x:x>highX?x-highX:0n,dy=y<lowY?lowY-y:y>highY?y-highY:0n;
            if(square(dx)+square(dy)<square(r))heights.push(elevation(fixture,state,cx,cy));
        }
    }
    assert(heights.length>0&&Math.max(...heights)-Math.min(...heights)<=actor.profile.max_step,'Complete body intersects known cliff: '+actor.label);
}
// Body overlap rule of the flock path (operator rule 4 of 2026-11-23, FM3): bodies of different owners never overlap
// (enemy units stay solid; the sim checks the same each tick, physical.rs); two Idle bodies of one owner may not
// overlap; while at least one of the two is not Idle, the overlap depth must stay below 25 % of the smaller radius.
// `a` and `b` are {pos:[x,y],radius,activity,owner}. Exact f32 arithmetic; returns a fault text or null.
function overlapFault(a,b){
    const dx=exact(a.pos[0])-exact(b.pos[0]),dy=exact(a.pos[1])-exact(b.pos[1]),d2=square(dx)+square(dy);
    const ra=exact(a.radius),rb=exact(b.radius),r=ra+rb,small=ra<rb?ra:rb;
    if(a.owner!==b.owner)return d2<square(r)?'Circles of different owners overlap':null;
    if(a.activity==='Idle'&&b.activity==='Idle')return d2<square(r)?'Idle circles overlap':null;
    // depth >= small/4  <=>  4*distance <= 4*r - small  <=>  16*d2 <= (4*r - small)^2 (4*r - small > 0)
    return 16n*d2<=square(4n*r-small)?'Moving circles overlap 25 % or more of the smaller radius':null;
}
function physical(fixture,state,bindings){
    const bodies=fixture.actors.map(actor=>{const id=bindings.get(actor.label),unit=world(state).units.find(u=>u.unit_id===id);assert(unit,'Frozen actor disappeared: '+actor.label);bodySupport(fixture,state,actor,unit);return{actor,body:{pos:unit.pos,radius:actor.profile.radius,activity:row(state,id)?.activity??'Idle',owner:actor.slot}};});
    for(let i=0;i<bodies.length;i++)for(let j=0;j<i;j++){
        const a=bodies[i],b=bodies[j],fault=overlapFault(a.body,b.body);
        assert(!fault,fault+': '+a.actor.label+'/'+b.actor.label+' ('+a.body.activity+'/'+b.body.activity+')');
    }
}
function publicOutcomes(state,fixture,bindings){return fixture.actors.map(actor=>{
    const id=bindings.get(actor.label),unit=world(state).units.find(u=>u.unit_id===id),order=row(state,id),work=world(state).terrain.work.find(w=>w.unit===id);
    return{label:actor.label,position:unit?.pos,activity:order?.activity,current:order?.current,queued:order?.queued,last_failure:order?.last_failure,work_wait:work?.wait,work_cell:work?.current_cell,completed_work:work?.completed_work,required_work:work?.required_work};
});}
function rectIs(fixture,state,min,max,height){for(let y=min[1];y<max[1];y++)for(let x=min[0];x<max[0];x++)if(elevation(fixture,state,x,y)!==height)return false;return true;}
function completed(fixture,state,bindings,labels){return labels.every(label=>idle(state,bindings.get(label)));}
function crossed(fixture,state,bindings,labels,right=true){
    assert(completed(fixture,state,bindings,labels),'Required actors have not completed');
    for(const label of labels){const actor=fixture.actors.find(a=>a.label===label),unit=world(state).units.find(u=>u.unit_id===bindings.get(label)),radius=Math.fround(actor.profile.radius);assert(right?unit.pos[0]-radius>=34:unit.pos[0]+radius<=30,'Order completed on wrong plateau: '+label);}
}
async function drive({pagesBySlot,fixture,capture,record,recordInput,onPoll}) {
    const bindings=new Map(),actions=[],samples=[],checks={observed_full_bodies:true,observed_no_overlap:true},original=new Map(),settled=new SettledTracker(),owned=new Map();
    const first=pagesBySlot.values().next().value;
    // Record only: the client frame of each order emit and of its first result, read from
    // the debug frame ring in states that the driver reads anyway. `wall_ms` is the frame
    // clock of the client, so the two times of one order share one clock.
    const orderFrames=new Map();
    const harvest=(slot,state)=>{
        for(const frame of state?.debug?.frames?.recent||[]){
            const stamp={frame_index:frame.frame_index,wall_ms:frame.wall_ms,tick:frame.tick,replica_tick:frame.replica_tick};
            for(const [kind,rows]of [['emit',frame.emitted_orders],['result',frame.results]])for(const row of rows||[]){
                const key=slot+':'+kind+':'+row.seq;
                if(Number.isInteger(row.seq)&&!orderFrames.has(key))orderFrames.set(key,stamp);
            }
        }
        return state;
    };
    for(const [slot,page]of pagesBySlot){
        let state=await wait(page,s=>world(s)?.terrain&&world(s).units.length===fixture.actors.length,'Public fixture did not activate');
        assert(state.debug.slot===slot,'Actual receiving slot differs');
        // Public camera controls only. Keep authored far clamp and High assets unchanged.
        await page.mouse.move(640,360);await page.mouse.wheel(0,10000);await sleep(250);
        state=await read(page);const before=state.scene.clip_from_world;await page.mouse.wheel(0,10000);await sleep(150);state=await read(page);
        assert(Math.max(...before.map((v,i)=>Math.abs(v-state.scene.clip_from_world[i])))<1e-4,'Authored far camera clamp not reached');
        const physicalBindings=actorBindings({...fixture,actors:fixture.actors.filter(a=>a.slot===slot)},state);for(const [label,id]of physicalBindings)bindings.set(label,id);
        owned.set(slot,fixture.actors.filter(a=>a.slot===slot).map(a=>a.label));
        await wait(page,s=>{const models=s.scene.models.filter(m=>m.unit_id!=null);return models.length===fixture.actors.length&&models.every(m=>m.ready&&m.quality==='high'&&m.level===0);},'Detailed High models missing');
    }
    // Preselect each peer's first cohort before starting the measured action timeline.
    const prepared=new Map();
    for(const [slot,page]of pagesBySlot){const intent=fixture.scene.commands.find(c=>c.slot===slot);if(intent?.tick===0){
        const beforePreselect=await read(page);
        recordInput?.({slot,stage:'preselect-before',wall_ms:Date.now(),tick:world(beforePreselect).tick,dom_focus:await page.evaluate(()=>document.hasFocus()),focus:beforePreselect.debug.focus,selection:beforePreselect.debug.selection});
        try{await selectActors(page,fixture,bindings,intent.actors,observation=>recordInput?.({slot,stage:'preselect-click',...observation}));}catch(error){
            const state=await read(page);
            recordInput?.({slot,stage:'preselect-failed',wall_ms:Date.now(),tick:world(state).tick,dom_focus:await page.evaluate(()=>document.hasFocus()),focus:state.debug.focus,selection:state.debug.selection,cursor:state.debug.cursor});
            await capture?.('failed-preselection-slot'+slot,state);
            const picks=intent.actors.map(label=>{const id=bindings.get(label),actor=fixture.actors.find(a=>a.label===label),unit=world(state).units.find(u=>u.unit_id===id);return {label,id,position:unit?.pos,projected:unit?project(state.scene,[...unit.pos,contact(state,id)+actor.profile.pick_height]):null};});
            error.message+='; actual selection='+JSON.stringify(state?.debug?.selection)+'; intended picks='+JSON.stringify(picks);
            throw error;
        }
        prepared.set(slot,intent);
    }}
    let base=world(await read(first)).tick,lastTick=-1,gaps=0,closed=false,reopened=false,openingTick=null,stable=null;
    const distant=new DistantOrderTracker();
    const labels=filter=>fixture.actors.filter(filter).map(a=>a.label),large=labels(a=>a.kind==='large'),small=labels(a=>a.kind==='small');
    const initialDeadline=fixture.scene.initial_deadline_ticks??fixture.scene.deadline_ticks;
    const settledBlocker=fixture.scene_name.startsWith('GM3-settled-blocker'),blockerHold=fixture.scene_name==='GM3-settled-blocker-hold';
    const blockerStart=fixture.actors.find(a=>a.label==='blocker')?.position;
    let blockerSettledPoint=null;
    let gm2Initial=false;const previousSurfaces=new Map();
    async function sample(){
        const state=await read(first);assert(state&&world(state),'Activated state disappeared');onPoll?.();
        const tick=world(state).tick,relative=tick-base;
        if(tick===lastTick)return state;
        if(lastTick>=0&&tick>lastTick+1)gaps+=tick-lastTick-1;lastTick=tick;
        // Private unit orders are read from the actor's actual owner page only.
        const states=new Map([[state.debug.slot,state]]);
        for(const [slot,page]of pagesBySlot)if(!states.has(slot))states.set(slot,await read(page));
        const merged={...state,debug:{...state.debug,replicated_world:{...world(state),unit_orders:[...states.values()].flatMap(s=>world(s).unit_orders)}}};
        // The overlap rule needs the activity of every actor, so it reads the merged orders of all owner pages.
        physical(fixture,merged,bindings);
        for(const intent of fixture.scene.commands.filter(c=>c.action==='move'))for(const label of intent.actors){
            const id=bindings.get(label),order=row(merged,id);
            if(order?.current?.order===fixture.package+':order_move'&&!original.has(label))original.set(label,JSON.parse(JSON.stringify(order.current)));
            const ownerTick=world(states.get(fixture.actors.find(a=>a.label===label).slot)).tick;
            if(original.has(label)&&idle(merged,id)&&ownerTick===tick){
                const position=world(state).units.find(u=>u.unit_id===id).pos;
                const actor=fixture.actors.find(a=>a.label===label),others=fixture.actors.filter(a=>a.label!==label).map(a=>({actor:a,unit:world(state).units.find(u=>u.unit_id===bindings.get(a.label))})).filter(x=>x.unit);
                const ownMovers=others.filter(x=>x.actor.slot===actor.slot&&(row(merged,x.unit.unit_id)?.activity??'Idle')!=='Idle').map(x=>x.unit.pos);
                settled.observe(label,true,position,actor.profile.radius,{tick,ownMovers,bodies:others.map(x=>x.unit.pos)});
            }
        }
        // The settled blocker is a slot-0 unit with its own move order, so the settled tracker above judges it with the
        // at-rest rule and as the first arrival of its order (Clara, 2026-11-24). Its settle point stays in the checks.
        if(fixture.scene_name==='GM2-narrow'){
            if(relative>=2400&&!gm2Initial){
                crossed(fixture,merged,bindings,small);
                for(const label of large){const id=bindings.get(label),order=row(merged,id),actor=fixture.actors.find(a=>a.label===label),unit=world(state).units.find(u=>u.unit_id===id);assert(order&&['Blocked','Pending'].includes(order.activity)&&same(order.current,original.get(label)),'Large actor lost truthful original blocked move: '+label);assert(unit.pos[0]+Math.fround(actor.profile.radius)<=30,'Large actor entered narrow passage');}
                gm2Initial=true;checks.small24_complete_before_widening=true;
            }
            for(const label of large)if(original.has(label)&&!idle(merged,bindings.get(label)))assert(same(row(merged,bindings.get(label)).current,original.get(label)),'Widening replaced original large move');
            if(gm2Initial&&rectIs(fixture,state,[30,20],[34,28],0)&&openingTick===null)openingTick=tick;
        }
        if(fixture.scene_name==='GM4-edit-during-approach'){
            closed||=rectIs(fixture,state,[31,20],[32,28],4);
            if(closed&&relative>240)reopened||=rectIs(fixture,state,[31,20],[32,28],0);
            const id=bindings.get('large.2.3'),order=row(merged,id),unit=world(state).units.find(u=>u.unit_id===id);
            distant.observe(relative,order?.current,unit.pos);
        }
        if(settledBlocker&&relative>=initialDeadline-200){const positions=publicOutcomes(merged,fixture,bindings).map(r=>[r.label,r.position,r.current]);if(stable)assert(same(stable,positions),'Impossible traffic oscillates instead of waiting');else stable=positions;}
        const surfaceChanges=[];for(const surface of world(state).terrain.surfaces){const key=surface.cell.x+","+surface.cell.y,encoded=JSON.stringify(surface);if(previousSurfaces.get(key)!==encoded){surfaceChanges.push(surface);previousSurfaces.set(key,encoded);}}
        const sample={tick,relative_tick:relative,surface_changes:surfaceChanges,peer_ticks:[...states].map(([slot,s])=>({slot,tick:world(s).tick})),outcomes:publicOutcomes(merged,fixture,bindings),closed,reopened,opening_tick:openingTick};
        record?.(sample);samples.push({tick,relative_tick:relative});
        return merged;
    }
    let pollTail=Promise.resolve();
    function poll(){const task=pollTail.then(sample);pollTail=task.catch(()=>{});return task;}
    // Monitor during selection and UI work as well as during the outcome wait.
    let monitorError=null,inPoll=false,latest;
    const timer=setInterval(async()=>{if(inPoll||monitorError)return;inPoll=true;try{latest=await poll();}catch(error){monitorError=error;}finally{inPoll=false;}},50);
    const monitor=()=>{if(monitorError)throw monitorError;onPoll?.();};
    try{
        for(let at=0;at<fixture.scene.commands.length;){
            const tick=fixture.scene.commands[at].tick,batch=[];while(at<fixture.scene.commands.length&&fixture.scene.commands[at].tick===tick)batch.push(fixture.scene.commands[at++]);
            while(world(await read(first)).tick<base+tick){monitor();await sleep(25);}
            if(fixture.scene_name==='GM2-narrow'&&tick===2401){latest=await poll();assert(gm2Initial,'Editor work cannot precede verified narrow outcome');await capture?.('narrow-before-widen',latest);}
            // Different peers may issue simultaneous intents concurrently. Same-owner intents
            // remain sequential because one real pointer cannot select two cohorts at once.
            const bySlot=new Map();for(const intent of batch){if(!bySlot.has(intent.slot))bySlot.set(intent.slot,[]);bySlot.get(intent.slot).push(intent);}
            await Promise.all([...bySlot].map(async([slot,intents])=>{for(const planned of intents){
                monitor();const page=pagesBySlot.get(slot);assert(page,'Missing real peer for slot '+slot);
                const intent=prepared.get(slot)===planned?planned:reachableIntent(fixture,await read(page),bindings,planned,fact=>(checks.covered_substitutions??=[]).push(fact));
                settled.begin(intent.actors);
                const before=await read(page),lastSeq=Math.max(0,...(before.debug.command_results||[]).map(r=>r.seq));
                const inputObservation=(stage,state)=>recordInput?.({slot,stage,...(harvest(slot,state),{}),wall_ms:Date.now(),tick:world(state).tick,focus:state.debug.focus,cursor:state.debug.cursor,selection:state.debug.selection,phase:state.debug.phase,command_results:state.debug.command_results});
                inputObservation('before',before);
                let ordered;
                try{ordered=await issue(page,fixture,bindings,intent,prepared.get(slot)===planned);inputObservation('after',await read(page));}catch(error){
                    const state=await read(page);await capture?.('failed-selection-slot'+slot,state);
                    const picks=intent.actors.map(label=>{const id=bindings.get(label),actor=fixture.actors.find(a=>a.label===label),unit=world(state).units.find(u=>u.unit_id===id);return {label,id,position:unit?.pos,projected:unit?project(state.scene,[...unit.pos,contact(state,id)+actor.profile.pick_height]):null};});
                    error.message+='; actual selection='+JSON.stringify(state?.debug?.selection)+'; intended picks='+JSON.stringify(picks);
                    throw error;
                }
                prepared.delete(slot);
                let acceptedState;
                try{acceptedState=await wait(page,s=>{
                    harvest(slot,s);
                    const results=(s.debug.command_results||[]).filter(r=>r.seq>lastSeq);
                    assert(!results.some(r=>r.rejected||r.envelope||r.refused?.length),'Pointer order was refused: '+JSON.stringify(results));
                    return results.some(r=>Number.isInteger(r.applied_tick)&&r.accepted?.length===intent.actors.length&&intent.actors.every(label=>r.accepted.includes(bindings.get(label))));
                },'Pointer order has no complete ordinary admission receipt',15000);}catch(error){
                    const state=await read(page);await capture?.('failed-admission-slot'+slot,state);
                    error.message+='; prior seq='+lastSeq+'; phase='+JSON.stringify(state?.debug?.phase)+'; selection='+JSON.stringify(state?.debug?.selection)+'; results='+JSON.stringify(state?.debug?.command_results);
                    throw error;
                }
                const receipt=acceptedState.debug.command_results.find(r=>r.seq>lastSeq&&Number.isInteger(r.applied_tick)&&r.accepted?.length===intent.actors.length);
                if(settledBlocker&&intent.tick===0&&intent.actors.length===1&&intent.actors[0]==='blocker'){
                    const id=bindings.get('blocker');
                    const state=await wait(page,s=>world(s).tick>=receipt.applied_tick&&idle(s,id),'Blocker did not settle at its starting point');
                    const position=world(state).units.find(u=>u.unit_id===id).pos;
                    const offset=Math.hypot(position[0]-blockerStart[0],position[1]-blockerStart[1]);
                    // Pointer projection is floating point. This small bound preserves
                    // the frozen passage geometry before exact retention begins.
                    assert(offset<0.001,'Blocker initial point differs from fixture: '+offset);
                    blockerSettledPoint=[...position];checks.blocker_projection_offset=offset;checks.blocker_settled_position=blockerSettledPoint;
                }
                // A receipt can arrive before the replica reaches its applied tick. The
                // next input must see this work order in the replica, not the state before it.
                if(intent.action!=='move'&&intent.action!=='hold'){
                    // The drag gave the order that the fixture names: its level, over its cells (and the start row or column).
                    const done=await wait(page,s=>world(s).tick>=receipt.applied_tick,'Replica did not reach the applied tick of the work order');
                    const work=row(done,bindings.get(intent.actors[0]))?.current?.target?.TerrainWork;
                    assert(work&&work.target_elevation===intent.target_elevation&&work.cells.min.x===ordered.min[0]&&work.cells.min.y===ordered.min[1]&&work.cells.max.x===ordered.max[0]&&work.cells.max.y===ordered.max[1],'The drag gave a different work order: '+JSON.stringify({work,ordered}));
                }
                settled.commit(intent.actors,intent.action,intent.action==='move'?intent.slot+'@'+intent.point.join(','):null);
                for(const label of intent.actors)original.delete(label);
                const after=harvest(slot,await read(page));
                const emit=orderFrames.get(slot+':emit:'+receipt.seq)||null,result=orderFrames.get(slot+':result:'+receipt.seq)||null;
                recordInput?.({slot,stage:'order-latency',seq:receipt.seq,action:intent.action,units:intent.actors.length,ack_tick:after.debug.command_results.find(r=>r.seq===receipt.seq&&Number.isInteger(r.ack_tick))?.ack_tick??null,applied_tick:receipt.applied_tick,emit,result,latency_ms:emit&&result?result.wall_ms-emit.wall_ms:null});
                actions.push({intended_tick:intent.tick,relative_before_selection:world(before).tick-base,relative_after_input:world(after).tick-base,accepted_tick:receipt.applied_tick,accepted_relative_tick:receipt.applied_tick-base,seq:receipt.seq,intent});
                await capture?.('input-slot'+slot+'-seq'+receipt.seq,after);
            }}));
        }
        const guard=Date.now()+420000;let final;
        while(Date.now()<guard){
            monitor();latest=await poll();const relative=world(latest).tick-base;
            let deadline=initialDeadline;
            if(fixture.scene_name==='GM2-narrow'){
                // Widening has a separate 120-second movement deadline after the actual edit.
                assert(openingTick!==null||relative<=4802,'Ordinary editor did not return and widen passage');
                if(openingTick===null){await sleep(50);continue;}
                deadline=openingTick-base+2400;
            }
            if(relative>=deadline){final=latest;break;}
            await sleep(50);
        }
        assert(final,'Completed ticks stopped before outcome deadline');monitor();
        if(settledBlocker){
            // Clara rule 1a (2026-11-24), as the sim test gap_with_a_big_unit (crates/turts_server/tests/terrain_group/
            // scenarios.rs): nothing moves in the last 200 ticks (above); a move of either slot that ends, ends inside its
            // goal area; bodies of different owners never overlap (physical, each sample). On Hold: the blocker stays and
            // nobody passes, every other unit waits on its side. Idle: own units may push it; no wait-state check.
            assert(stable,'No stable evidence in the last 200 ticks');
            const blocker=world(final).units.find(u=>u.unit_id===bindings.get('blocker'));
            // Each move order of both slots (Clara, 2026-11-24: slot 1 too, no F4 exception): a move that ends, ends
            // inside the goal area of its order.
            for(const command of fixture.scene.commands.filter(c=>c.action==='move'&&!c.actors.includes('blocker'))){
                const members=fixture.actors.filter(a=>command.actors.includes(a.label)),goal=command.point;
                const area=Math.sqrt(members.reduce((sum,a)=>sum+a.profile.radius**2,0)/0.35);
                for(const actor of members){
                    const at=world(final).units.find(u=>u.unit_id===bindings.get(actor.label)).pos;
                    assert(!idle(final,bindings.get(actor.label))||Math.hypot(at[0]-goal[0],at[1]-goal[1])<=area,'A move ended outside its goal area: '+actor.label+' (slot '+actor.slot+') at '+JSON.stringify(at)+' (area '+area.toFixed(3)+' m)');
                }
            }
            if(blockerHold){
                assert(Math.hypot(blocker.pos[0]-blockerSettledPoint[0],blocker.pos[1]-blockerSettledPoint[1])<1e-3,'A unit on Hold moved: '+JSON.stringify(blocker.pos));
                for(const actor of fixture.actors.filter(a=>a.label!=='blocker')){
                    const id=bindings.get(actor.label),at=world(final).units.find(u=>u.unit_id===id).pos;
                    assert(!idle(final,id)&&['Blocked','Pending'].includes(row(final,id).activity),'Impossible crossing lacks truthful wait state: '+actor.label);
                    assert(actor.slot===0?at[0]+actor.profile.radius<=30:at[0]-actor.profile.radius>=34,'A unit left its side of a shut gap: '+actor.label);
                }
                checks.permanent_blocker_retained=true;
            }else{
                assert(idle(final,bindings.get('blocker')),'The idle blocker did not end at rest');
                checks.blocker_final_position=blocker.pos;
            }
            checks.stable_wait_ticks=200;checks.blocker_hold=blockerHold;
        }else if(fixture.scene_name==='GM2-narrow'){
            crossed(fixture,final,bindings,large);checks.large12_original_move_completed=true;checks.opening_tick=openingTick;
        }else if(fixture.scene_name==='GM4-edit-during-approach'){
            assert(closed&&reopened&&distant.moved,'Actual fill/reopen or distant motion missing');crossed(fixture,final,bindings,labels(a=>a.label!=='small.0.0'&&a.label!=='large.2.3'));checks.closed_reopened=true;checks.distant_order_unchanged_through_tick=340;
        }else{
            for(const actor of fixture.actors){const id=bindings.get(actor.label);assert(idle(final,id),'Feasible cohort did not finish: '+actor.label);
                if(actor.label==='blocker')continue;
                if(actor.label==='rear.small.0'){const p=world(final).units.find(u=>u.unit_id===id).pos;assert(Math.hypot(p[0]-20,p[1]-6)<=actor.profile.radius,'Unique destination not reached');continue;}
                crossed(fixture,final,bindings,[actor.label],fixture.scene_name==='GM3-shared-and-unique'||actor.slot===0);
            }checks.feasible_all_completed=true;
        }
        await capture?.('final',final);
        return{status:'PASS',scene:fixture.scene_name,package:fixture.package,base_tick:base,final_tick:world(final).tick,actions,checks,samples,
            limits:['Browser checks observed complete bodies and permitted terrain. Native acceptance certifies every actual committed sweep; polling can skip ticks.','Real pointer acceptance ticks are recorded and may differ from native script ticks.','Internal order identity/route lease and same-seed trajectory proof remain native checks.'],skipped_observation_ticks:gaps};
    }finally{clearInterval(timer);while(inPoll)await sleep(10);await pollTail;}
}
module.exports={activatePage,project,read,actorBindings,permittedTop,selectActors,issue,publicOutcomes,physical,overlapFault,freePoint,reachableIntent,dragFor,drive};
