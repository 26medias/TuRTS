// turts-test-role: instrument
// fm-probe.js PORT FIXTURE_JSON PACKAGE_ID OUT_DIR
// Browser measure of the T-7 rows FM1 and FM2 on a public ridge package, in the Chrome that scripts/mp1-play.sh
// started. Input is pointer only (helpers of terrain-group/driver.js). The bridges are read only to observe.
// Sequence (Jade's play): box-select all 36, right-click (50,24) [outbound]; one ordinary dig that opens the ridge;
// box-select all 36, right-click (14,24) [return].
// FM1: ticks from the applied order to the first displacement (> 0.1 m) of each unit; pass when the first unit
//      moves within 20 ticks (1 s at 20 Hz). The clause "free space in front" is not judged here: the time of
//      each unit is recorded.
// The return order goes to all 36 units (default) to the point FM_RETURN_POINT (default 22,24).
// FM2: after the return order, all 36 units Idle and apart (no body overlap) within 1200 ticks (60 s at 20 Hz),
//      and no unit that is not Idle and moves less than 0.05 m during 100 ticks (5 s). The probe observes to
//      2400 ticks for information; the gate is 1200. Separation is judged 40 ticks after the last unit settles
//      (a unit can settle with a short overlap that the standing units relax), and not before settlement.
// The probe records all figures and fails at the end; a failed row does not stop the sequence.
'use strict';
const fs=require('fs'),path=require('path');
const [port,fixtureFile,packageId,out]=process.argv.slice(2);
const root=process.env.TURTS_ROOT;
const {chromium}=require('playwright-core');
const {activatePage,project,read,actorBindings,issue}=require(path.join(root,'tests/browser/terrain-group/driver.js'));
const sleep=ms=>new Promise(r=>setTimeout(r,ms));
const FIRST_MOVE_M=.1,FM1_TICKS=20,FM2_TICKS=1200,STALL_TICKS=100,STALL_M=.05;
// Separation at rest (simulation rule, Kenji 2026-11-23): a unit can settle with a short overlap, and the standing
// units relax it in the next ticks. So the pairs are measured REST_TICKS after the last unit settles, with the
// tolerance REST_TOLERANCE_M for the f32 positions of the wire.
const REST_TICKS=40,REST_TOLERANCE_M=1e-4;
const result={package:packageId,steps:[],rows:{},status:'FAIL',errors:[]};
const save=()=>fs.writeFileSync(path.join(out,'result.json'),JSON.stringify(result,null,2));
// ok === null: the row is not judged in this run (it is printed, and it is not a pass).
// FM1 verdict: NOT JUDGED (null) when no sample came inside FM1_TICKS of the applied order (the probe cannot see a
// first motion inside the bound), else the first unit must move inside the bound.
const fm1=m=>m.first_sample_ticks===null||m.first_sample_ticks>FM1_TICKS?null:m.first_unit_ticks!==null&&m.first_unit_ticks<=FM1_TICKS;
const fm1Detail=m=>fm1(m)===null?{...m,reason:'not judged: the first sample after the applied order came '+m.first_sample_ticks+' ticks later (bound '+FM1_TICKS+'); the probe samples too slowly to see the first motion'}:m;
const step=(name,ok,detail,fatal=true)=>{result.steps.push({name,ok,detail,at:new Date().toISOString()});save();console.log((ok===null?'NOT JUDGED ':ok?'PASS ':'FAIL ')+name+' '+JSON.stringify(detail??''));if(ok===false&&fatal)throw Error('Step failed: '+name);};
(async()=>{
 let browser;
 try{
    fs.mkdirSync(out,{recursive:true});
    const fixture=JSON.parse(fs.readFileSync(fixtureFile));
    const base=fixture.package;fixture.package=packageId;for(const a of fixture.actors)a.content=a.content.replace(base+':',packageId+':');
    browser=await chromium.connectOverCDP('http://127.0.0.1:'+port);
    const page=browser.contexts().flatMap(c=>c.pages()).find(p=>/^http:\/\/127\.0\.0\.1/.test(p.url()));
    if(!page)throw Error('No game page in the launcher Chrome');
    page.on('pageerror',e=>result.errors.push(String(e)));
    await page.setViewportSize({width:1280,height:720});await activatePage(page);
    const world=s=>s.debug.replicated_world;
    const shot=name=>page.screenshot({path:path.join(out,name+'.png')});
    let state,end=Date.now()+90000,f9=false;
    while(Date.now()<end){state=await read(page);if(state?.debug.replicated_world?.terrain)break;if(!f9&&Date.now()>end-80000){await page.keyboard.press('F9');f9=true;}await sleep(200);}
    step('launch: the game page joined and shows terrain',!!state?.debug.replicated_world?.terrain,{slot:state?.debug.slot,units:state?.debug.replicated_world?.units.length,tick_hz:state?.debug.tick_hz});
    result.tick_hz=state.debug.tick_hz;
    const bindings=actorBindings(fixture,state);
    const radius=new Map(fixture.actors.map(a=>[bindings.get(a.label),a.profile.radius])),kind=new Map(fixture.actors.map(a=>[bindings.get(a.label),a.kind]));
    const all=[...radius.keys()],small=all.filter(id=>kind.get(id)==='small'),large=all.filter(id=>kind.get(id)==='large');
    const pos=(s,id)=>world(s).units.find(u=>u.unit_id===id)?.pos,order=(s,id)=>world(s).unit_orders.find(o=>o.unit===id);
    const act=(s,list)=>{const c={};for(const id of list){const a=order(s,id)?.activity??'none';c[a]=(c[a]||0)+1;}return c;};
    const overlaps=(s,tolerance=REST_TOLERANCE_M)=>{const bad=[];for(let i=0;i<all.length;i++)for(let j=i+1;j<all.length;j++){const a=pos(s,all[i]),b=pos(s,all[j]),d=Math.hypot(a[0]-b[0],a[1]-b[1]),need=radius.get(all[i])+radius.get(all[j]);if(d<need-tolerance)bad.push({a:all[i],b:all[j],distance:d,need,depth:need-d,pos_a:a,pos_b:b});}return bad;};
    // One box drag around `wanted`. If the centre of another own unit is inside the box, the margins shrink.
    // The selection that the game gives is recorded; a difference from `wanted` is a reported fact, not hidden.
    const selectBox=async(name,wanted,exact)=>{
        const s=await read(page),centre=new Map(s.scene.units.map(u=>[u.unit_id,u.ground_screen_center])),c=wanted.map(id=>centre.get(id));
        const make=(mx,top,bottom)=>[Math.max(2,Math.min(...c.map(p=>p[0]))-mx),Math.max(2,Math.min(...c.map(p=>p[1]))-top),Math.min(1278,Math.max(...c.map(p=>p[0]))+mx),Math.min(700,Math.max(...c.map(p=>p[1]))+bottom)];
        const inside=(box,id)=>{const p=centre.get(id);return p[0]>=box[0]&&p[0]<=box[2]&&p[1]>=box[1]&&p[1]<=box[3];};
        let box=make(30,40,30);if(all.some(id=>!wanted.includes(id)&&inside(box,id)))box=make(10,14,10);
        await activatePage(page);await page.mouse.move(box[0],box[1]);await page.mouse.down();await sleep(80);await page.mouse.move(box[2],box[3],{steps:8});await sleep(80);await page.mouse.up();await sleep(300);
        const got=(await read(page)).debug.selection,missing=wanted.filter(id=>!got.includes(id)),extra=got.filter(id=>!wanted.includes(id));
        step(name+': one box drag selects the '+wanted.length+' wanted units',missing.length===0&&extra.length===0,{box,selected:got.length,missing,extra},exact);
        return got;
    };
    // One move order and its samples. `until(sample)` ends the phase; `limitMs` is the wall limit.
    const moveAndSample=async(name,point,limitMs,until)=>{
        const before=await read(page),start=new Map(all.map(id=>[id,pos(before,id)])),seqs=new Set(before.debug.command_results.map(r=>r.seq));
        await issue(page,fixture,bindings,{tick:name==='outbound'?0:2,actors:fixture.actors.map(a=>a.label),action:'move',point,queue:'Replace'},true);
        const file=fs.createWriteStream(path.join(out,name+'-samples.jsonl')),t0=Date.now();
        let applied=null,firstSample=null,first=new Map(),lastMove=new Map(all.map(id=>[id,{tick:null,pos:null}])),stalls=[],open=new Map(),nextShot=0,last=null,settled=null,samples=0;
        while(Date.now()-t0<limitMs){
            const s=await read(page);if(!s){await sleep(20);continue;}
            const tick=world(s).tick;last=s;samples++;
            if(applied===null){const r=s.debug.command_results.find(r=>!seqs.has(r.seq)&&Number.isInteger(r.applied_tick));if(r)applied={tick:r.applied_tick,seq:r.seq,accepted:r.accepted?.length,refused:r.refused};}
            if(applied&&firstSample===null&&tick>=applied.tick)firstSample=tick;
            const row={tick,wall_ms:Date.now()-t0,u:all.map(id=>{const p=pos(s,id);return[id,Math.round(p[0]*100)/100,Math.round(p[1]*100)/100,order(s,id)?.activity??'none'];})};
            file.write(JSON.stringify(row)+'\n');
            for(const id of all){
                const p=pos(s,id),a=order(s,id)?.activity,st=start.get(id);
                if(!first.has(id)&&Math.hypot(p[0]-st[0],p[1]-st[1])>FIRST_MOVE_M)first.set(id,tick);
                // Stall: not Idle, and less than STALL_M of displacement since the anchor sample, for STALL_TICKS.
                const m=lastMove.get(id);
                if(a==='Idle'||m.pos===null||Math.hypot(p[0]-m.pos[0],p[1]-m.pos[1])>=STALL_M){if(open.has(id)){open.get(id).to_tick=tick;open.delete(id);}lastMove.set(id,{tick,pos:p});}
                else if(tick-m.tick>=STALL_TICKS&&!open.has(id)){const e={unit:id,kind:kind.get(id),from_tick:m.tick,to_tick:null,activity:a,pos:p};stalls.push(e);open.set(id,e);}
            }
            if(Date.now()-t0>=nextShot){await shot(name+'-'+String(Math.round(nextShot/1000)).padStart(3,'0')+'s');nextShot+=10000;}
            const idleNow=all.every(id=>order(s,id)?.activity==='Idle');
            if(settled===null&&applied&&tick>applied.tick+5&&idleNow)settled={tick,overlaps_at_settle:overlaps(s),rest:null};
            // A unit that leaves Idle again before the rest sample moves the settle tick.
            else if(settled&&settled.rest===null&&!idleNow)settled=null;
            else if(settled&&settled.rest===null&&tick>=settled.tick+REST_TICKS)settled.rest={tick,overlaps:overlaps(s)};
            if(until({state:s,tick,applied,first,settled}))break;
            await sleep(60);
        }
        file.end();await shot(name+'-end');fs.writeFileSync(path.join(out,name+'-end.json'),JSON.stringify(last,null,2));
        const rel=id=>first.has(id)&&applied?first.get(id)-applied.tick:null,rels=all.map(rel).filter(n=>n!==null).sort((a,b)=>a-b);
        return{applied,samples,sample_ms:Math.round((Date.now()-t0)/Math.max(1,samples)),end_tick:world(last).tick,first_motion:{first_sample_ticks:applied&&firstSample!==null?firstSample-applied.tick:null,first_unit_ticks:rels[0]??null,units_moved:rels.length,units_within_20_ticks:rels.filter(n=>n<=FM1_TICKS).length,median_ticks:rels[Math.floor(rels.length/2)]??null,last_ticks:rels.at(-1)??null,small:small.map(rel),large:large.map(rel)},settled:settled&&{ticks_after_order:settled.tick-applied.tick,overlaps_at_settle:settled.overlaps_at_settle,rest:settled.rest&&{ticks_after_settle:settled.rest.tick-settled.tick,overlaps:settled.rest.overlaps}},stalls,end_activity:{small:act(last,small),large:act(last,large)},end_overlaps:overlaps(last)};
    };
    await shot('00-start');
    // Outbound.
    await selectBox('outbound',all,true);
    const outbound=await moveAndSample('outbound',[50,24],180000,({state:s})=>small.every(id=>pos(s,id)[0]>=34&&order(s,id)?.activity==='Idle'));
    result.rows.outbound=outbound;save();
    step('FM1 outbound: the first unit moves within 20 ticks of the applied order',fm1(outbound.first_motion),fm1Detail(outbound.first_motion),false);
    state=await read(page);
    step('outbound: all 24 small units are across',small.every(id=>pos(state,id)[0]>=34),{small:act(state,small),large:act(state,large),large_across:large.filter(id=>pos(state,id)[0]>=34).length});
    // Dig, as a person: click a small unit, take the one that the click selects.
    const smallProfile=fixture.actors.find(a=>a.kind==='small').profile;let worker=null;result.worker_clicks=[];
    for(const label of ['small.0.0','small.0.5','small.3.0','small.3.5','small.1.2']){
        const id=bindings.get(label),s=await read(page),p=pos(s,id),anchor=world(s).terrain.anchors.find(a=>a.unit===id&&a.current);
        await activatePage(page);await page.mouse.click(...project(s.scene,[...p,(anchor?anchor.elevation*.25:0)+smallProfile.pick_height]),{button:'left',delay:80});await sleep(400);
        const got=(await read(page)).debug.selection;result.worker_clicks.push({label,wanted:id,selection:got});save();
        if(got.length===1&&small.includes(got[0])){worker=fixture.actors.find(a=>bindings.get(a.label)===got[0]).label;break;}
    }
    step('dig: a click on a small unit selects one small unit',!!worker,result.worker_clicks);
    await issue(page,fixture,bindings,{tick:1,actors:[worker],action:'excavate',cells:{min:[30,20],max:[34,28]},target_elevation:0,queue:'Replace'},true);
    const level=(s,x,y)=>world(s).terrain.surfaces.find(r=>r.cell.x===x&&r.cell.y===y)?.elevation;
    const open=s=>{for(let y=20;y<28;y++)for(let x=30;x<34;x++){const e=level(s,x,y);if(y>=23&&y<25){if(e!==undefined&&e!==0)return false;}else if(e!==0)return false;}return true;};
    let stop=Date.now()+420000,digStart=world(await read(page)).tick;while(Date.now()<stop){state=await read(page);if(state&&open(state))break;await sleep(200);}
    step('dig: the ordinary dig opens the ridge',open(state),{ticks:world(state).tick-digStart});
    const openTick=world(state).tick;
    stop=Date.now()+300000;while(Date.now()<stop){state=await read(page);if(state&&all.every(id=>pos(state,id)[0]>=34||id===bindings.get(worker))&&all.every(id=>order(state,id)?.activity==='Idle'))break;await sleep(200);}
    result.rows.after_dig={ticks_from_open_to_all_idle:world(state).tick-openTick,small:act(state,small),large:act(state,large),large_across:large.filter(id=>pos(state,id)[0]>=34).length,overlaps_at_this_sample:overlaps(state).length};save();
    step('after the dig: the 12 large units are across with no new order',large.every(id=>pos(state,id)[0]>=34),result.rows.after_dig,false);
    await shot('10-before-return');
    // Return.
    // Return order (approved gate, T-49): one box drag selects all 36 units, the digger in the passage too, and
    // one right-click sends them to FM_RETURN_POINT (default 22,24).
    const returnPoint=(process.env.FM_RETURN_POINT||'22,24').split(',').map(Number);
    state=await read(page);const far=all.filter(id=>pos(state,id)[0]>=34),near=all.filter(id=>pos(state,id)[0]<34);
    const selected=await selectBox('return',all,true);
    result.rows.return_order={point:returnPoint,wanted:'all 36',far:far.length,not_far:near.map(id=>({unit:id,kind:kind.get(id),pos:pos(state,id)})),selected:selected.length,selected_units:selected};save();
    const ret=await moveAndSample('return',returnPoint,150000,({tick,applied,settled})=>(settled!==null&&settled.rest!==null)||(applied&&tick-applied.tick>FM2_TICKS+1200));
    result.rows.return=ret;save();
    step('FM1 return: the first unit moves within 20 ticks of the applied order',fm1(ret.first_motion),fm1Detail(ret.first_motion),false);
    step('FM1: every unit with free space in front of it moves within 20 ticks',null,{reason:'not judged in the browser; the headless check judges this clause',outbound_units_within_20_ticks:outbound.first_motion.units_within_20_ticks,return_units_within_20_ticks:ret.first_motion.units_within_20_ticks,per_unit_ticks:'rows.outbound.first_motion and rows.return.first_motion'},false);
    step('FM2 return: all 36 units settled within 1200 ticks (60 s at 20 Hz) of the applied order',!!ret.settled&&ret.settled.ticks_after_order<=FM2_TICKS,{settled_ticks:ret.settled&&ret.settled.ticks_after_order,observed_to_ticks:ret.end_tick-(ret.applied?.tick??ret.end_tick),end_activity:ret.end_activity,ordered_units:selected.length},false);
    step('FM2 return: 40 ticks after the last unit settles, no pair is closer than the sum of the radii (tolerance 0.0001 m)',ret.settled?.rest?ret.settled.rest.overlaps.length===0:null,ret.settled?.rest?{ticks_after_settle:ret.settled.rest.ticks_after_settle,overlaps:ret.settled.rest.overlaps,overlaps_at_the_settle_tick_for_information:ret.settled.overlaps_at_settle.length}:{reason:'no settlement that lasted 40 ticks in the observed time; separation is not judged',overlaps_in_the_last_sample:ret.end_overlaps.length},false);
    step('FM2 return: no unit with no motion and no settled state for more than 100 ticks',ret.stalls.length===0,{stalls:ret.stalls.length,first:ret.stalls.slice(0,5)},false);
    step('outbound: no unit with no motion and no settled state for more than 100 ticks (recorded for information)',outbound.stalls.length===0,{stalls:outbound.stalls.length,kinds:outbound.stalls.reduce((c,s)=>(c[s.kind]=(c[s.kind]||0)+1,c),{})},false);
    step('browser: no page error',result.errors.length===0,result.errors,false);
    const rows=result.steps.filter(s=>/^FM[12]/.test(s.name));
    // A row that is not judged is not a pass and not a fail: the status needs every judged row true, and the
    // settle and separation rows judged.
    result.not_judged=rows.filter(s=>s.ok===null).map(s=>s.name);
    const required=rows.filter(s=>/^FM2 return|^FM1 (outbound|return):/.test(s.name));
    result.status=rows.some(s=>s.ok===false)||result.errors.length?'FAIL':required.every(s=>s.ok===true)?'PASS':'NOT JUDGED';
    if(result.status!=='PASS')process.exitCode=1;
 }catch(error){result.error=String(error.stack||error);console.error(error);process.exitCode=1;}
 finally{save();if(browser)await browser.close().catch(()=>{});}
})();
