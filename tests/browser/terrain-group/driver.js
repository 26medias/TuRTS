// Real GM1-GM4 pointer/key input and public outcome checks. No state/command injection.
'use strict';
const assert=(ok,message)=>{if(!ok)throw Error(message);};
const {SettledTracker,DistantOrderTracker}=require('./settled-tracker');
const {exactOwnedBox}=require('./selection-box');
const sleep=ms=>new Promise(resolve=>setTimeout(resolve,ms));
// Hold both sides of a gesture across frames. A zero-delay press/release can
// disappear between Bevy input polls, even when hover picked the correct body.
const PRESS_MS=80;
async function click(page,point,button='left'){
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
async function selectActors(page,fixture,bindings,labels) {
    if(fixture.scene_name==='GM3-opposing-large'){
        const state=await read(page),wanted=labels.map(label=>bindings.get(label));
        const units=world(state).units;
        const points=state.scene.units.map(unit=>({id:unit.unit_id,center:unit.ground_screen_center,owned:units.find(u=>u.unit_id===unit.unit_id)?.owner_slot===state.debug.slot}));
        const box=exactOwnedBox(points,wanted);assert(box,'Requested cohort has no exact owned selection box');
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
        await click(page,project(state.scene,[...unit.pos,contact(state,id)+actor.profile.pick_height]));
    }}finally{await page.keyboard.up('Shift');}
    const wanted=labels.map(label=>bindings.get(label));
    await wait(page,s=>s.debug.selection.length===wanted.length&&wanted.every(id=>s.debug.selection.includes(id)),'Physical body selection differs from frozen cohort');
}
async function issue(page,fixture,bindings,intent,preselected=false) {
    if(!preselected)await selectActors(page,fixture,bindings,intent.actors);
    if(intent.queue==='Append'){await page.keyboard.down('Shift');await sleep(PRESS_MS);}
    try{
        if(intent.action==='move'){
            const state=await read(page);await click(page,project(state.scene,[...intent.point,permittedTop(fixture,state,intent.point)]),'right');
        }else{
            const tool=fixture.package+':'+fixture.bindings[intent.action==='fill'?'fill':'dig'];
            let state=await wait(page,s=>s.ui.some(w=>w.visible&&w.enabled&&w.widget_action?.tool===tool),'Bound public TerrainTools button absent');
            const button=state.ui.find(w=>w.visible&&w.enabled&&w.widget_action?.tool===tool),[x,y,w,h]=button.rect;
            await click(page,[x+w/2,y+h/2]);
            const reference=intent.target_elevation===4?[30.5,19.5]:[28.5,24.5];
            state=await read(page);assert(permittedTop(fixture,state,reference)===intent.target_elevation*.25,'Permitted reference level changed');
            await click(page,project(state.scene,[...reference,intent.target_elevation*.25]));
            state=await read(page);const a=intent.cells.min.map(n=>n+.5),b=intent.cells.max.map(n=>n-.5);
            await page.mouse.move(...project(state.scene,[...a,permittedTop(fixture,state,a)]));await page.mouse.down();await sleep(PRESS_MS);
            await page.mouse.move(...project(state.scene,[...b,permittedTop(fixture,state,b)]),{steps:8});await sleep(PRESS_MS);await page.mouse.up();await sleep(PRESS_MS);
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
function physical(fixture,state,bindings){
    const bodies=fixture.actors.map(actor=>{const unit=world(state).units.find(u=>u.unit_id===bindings.get(actor.label));assert(unit,'Frozen actor disappeared: '+actor.label);bodySupport(fixture,state,actor,unit);return{actor,unit};});
    for(let i=0;i<bodies.length;i++)for(let j=0;j<i;j++){
        const a=bodies[i],b=bodies[j],dx=exact(a.unit.pos[0])-exact(b.unit.pos[0]),dy=exact(a.unit.pos[1])-exact(b.unit.pos[1]),r=exact(a.actor.profile.radius)+exact(b.actor.profile.radius);
        assert(square(dx)+square(dy)>=square(r),'Complete observed circles overlap: '+a.actor.label+'/'+b.actor.label);
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
        try{await selectActors(page,fixture,bindings,intent.actors);}catch(error){
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
    const settledBlocker=fixture.scene_name==='GM3-settled-blocker';
    const blockerStart=fixture.actors.find(a=>a.label==='blocker')?.position;
    let gm2Initial=false;const previousSurfaces=new Map();
    async function sample(){
        const state=await read(first);assert(state&&world(state),'Activated state disappeared');onPoll?.();
        const tick=world(state).tick,relative=tick-base;
        if(tick===lastTick)return state;
        if(lastTick>=0&&tick>lastTick+1)gaps+=tick-lastTick-1;lastTick=tick;
        physical(fixture,state,bindings);
        // Private unit orders are read from the actor's actual owner page only.
        const states=new Map([[state.debug.slot,state]]);
        for(const [slot,page]of pagesBySlot)if(!states.has(slot))states.set(slot,await read(page));
        const merged={...state,debug:{...state.debug,replicated_world:{...world(state),unit_orders:[...states.values()].flatMap(s=>world(s).unit_orders)}}};
        for(const intent of fixture.scene.commands.filter(c=>c.action==='move'))for(const label of intent.actors){
            const id=bindings.get(label),order=row(merged,id);
            if(order?.current?.order===fixture.package+':order_move'&&!original.has(label))original.set(label,JSON.parse(JSON.stringify(order.current)));
            const ownerTick=world(states.get(fixture.actors.find(a=>a.label===label).slot)).tick;
            if(original.has(label)&&idle(merged,id)&&ownerTick===tick){
                const position=world(state).units.find(u=>u.unit_id===id).pos;
                settled.observe(label,true,position);
            }
        }
        if(settledBlocker&&relative>=10){const blocker=world(state).units.find(u=>u.unit_id===bindings.get('blocker'));assert(same(blocker.pos,blockerStart),'Traffic displaced permanent settled blocker');}
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
            await Promise.all([...bySlot].map(async([slot,intents])=>{for(const intent of intents){
                monitor();const page=pagesBySlot.get(slot);assert(page,'Missing real peer for slot '+slot);
                settled.begin(intent.actors);
                const before=await read(page),lastSeq=Math.max(0,...(before.debug.command_results||[]).map(r=>r.seq));
                const inputObservation=(stage,state)=>recordInput?.({slot,stage,wall_ms:Date.now(),tick:world(state).tick,focus:state.debug.focus,cursor:state.debug.cursor,selection:state.debug.selection,phase:state.debug.phase,command_results:state.debug.command_results});
                inputObservation('before',before);
                try{await issue(page,fixture,bindings,intent,prepared.get(slot)===intent);inputObservation('after',await read(page));}catch(error){
                    const state=await read(page);await capture?.('failed-selection-slot'+slot,state);
                    const picks=intent.actors.map(label=>{const id=bindings.get(label),actor=fixture.actors.find(a=>a.label===label),unit=world(state).units.find(u=>u.unit_id===id);return {label,id,position:unit?.pos,projected:unit?project(state.scene,[...unit.pos,contact(state,id)+actor.profile.pick_height]):null};});
                    error.message+='; actual selection='+JSON.stringify(state?.debug?.selection)+'; intended picks='+JSON.stringify(picks);
                    throw error;
                }
                prepared.delete(slot);
                let acceptedState;
                try{acceptedState=await wait(page,s=>{
                    const results=(s.debug.command_results||[]).filter(r=>r.seq>lastSeq);
                    assert(!results.some(r=>r.rejected||r.envelope||r.refused?.length),'Pointer order was refused: '+JSON.stringify(results));
                    return results.some(r=>Number.isInteger(r.applied_tick)&&r.accepted?.length===intent.actors.length&&intent.actors.every(label=>r.accepted.includes(bindings.get(label))));
                },'Pointer order has no complete ordinary admission receipt',15000);}catch(error){
                    const state=await read(page);await capture?.('failed-admission-slot'+slot,state);
                    error.message+='; prior seq='+lastSeq+'; phase='+JSON.stringify(state?.debug?.phase)+'; selection='+JSON.stringify(state?.debug?.selection)+'; results='+JSON.stringify(state?.debug?.command_results);
                    throw error;
                }
                const receipt=acceptedState.debug.command_results.find(r=>r.seq>lastSeq&&Number.isInteger(r.applied_tick)&&r.accepted?.length===intent.actors.length);
                settled.commit(intent.actors,intent.action);
                for(const label of intent.actors)original.delete(label);
                const after=await read(page);actions.push({intended_tick:intent.tick,relative_before_selection:world(before).tick-base,relative_after_input:world(after).tick-base,accepted_tick:receipt.applied_tick,accepted_relative_tick:receipt.applied_tick-base,seq:receipt.seq,intent});
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
            assert(idle(final,bindings.get('blocker'))&&stable,'No settled blocker/stable waiting evidence');
            for(const actor of fixture.actors.filter(a=>a.label!=='blocker'))assert(!idle(final,bindings.get(actor.label))&&['Blocked','Pending'].includes(row(final,bindings.get(actor.label)).activity),'Impossible crossing lacks truthful wait state');
            checks.permanent_blocker_retained=true;checks.stable_wait_ticks=200;
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
module.exports={project,read,actorBindings,permittedTop,selectActors,issue,publicOutcomes,physical,drive};
