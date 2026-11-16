// Real-pointer/key TW6 probe. No debug mutation, test commands, or command injection.
// node tests/browser/terrain-work.js URL SCENARIO.json OUTPUT [CHROME]
'use strict';
const fs=require('node:fs'),os=require('node:os'),path=require('node:path');
const {spawn}=require('node:child_process'),{chromium}=require('playwright-core');
const {decodePng}=require('./harness/png');
const [url,scenarioFile,output,executablePath='/usr/bin/google-chrome']=process.argv.slice(2);
if(!output)throw Error('URL SCENARIO.json OUTPUT [CHROME]');
const scenario=JSON.parse(fs.readFileSync(scenarioFile));
fs.mkdirSync(output,{recursive:true});
const sleep=ms=>new Promise(resolve=>setTimeout(resolve,ms));
const assert=(value,message)=>{if(!value)throw Error(message);};
function project(scene,[x,y,height=0]) {
 const m=scene.clip_from_world,v=[x,height,y,1],c=[0,1,2,3].map(r=>v.reduce((a,n,k)=>a+m[k*4+r]*n,0));
 assert(c[3]>0,'Point behind camera');const [left,top,width,depth]=scene.viewport_logical_rect;
 return [left+(c[0]/c[3]+1)*width/2,top+(1-c[1]/c[3])*depth/2];
}
const matrixDelta=(a,b)=>Math.max(...a.map((n,i)=>Math.abs(n-b[i])));
function ghostPixels(beforeBytes,afterBytes,center,radius,color='blue') {
 const a=decodePng(beforeBytes),b=decodePng(afterBytes);assert(a.width===b.width&&a.height===b.height,'Screenshot size changed');let count=0;
 for(let y=Math.max(0,Math.floor(center[1]-radius));y<Math.min(a.height,Math.ceil(center[1]+radius));y++)for(let x=Math.max(0,Math.floor(center[0]-radius));x<Math.min(a.width,Math.ceil(center[0]+radius));x++){
  const i=(y*a.width+x)*4,[r,g,blue]=b.rgba.subarray(i,i+3),change=Math.abs(r-a.rgba[i])+Math.abs(g-a.rgba[i+1])+Math.abs(blue-a.rgba[i+2]);
  if(change>40&&(color==='green'?(g>r*1.3&&g>blue*1.15):(blue>r*1.3&&blue>g*1.1)))count++;
 }return count;
}
(async()=>{
 const result={status:'RUNNING',errors:[],actions:[],states:[],checks:{},continuity_limit:'Public order/target/queue and partial-work continuity only. Internal order identity and fractional carry remain covered by the real sim test.'};
 let context,xvfb,profile;
 try {
  xvfb=spawn('Xvfb',['-displayfd','3','-screen','0','1280x720x24','-nolisten','tcp'],{stdio:['ignore','ignore','pipe','pipe']});
  const display=await new Promise((resolve,reject)=>{let value='';xvfb.stdio[3].on('data',data=>{value+=data;if(value.includes('\n'))resolve(':'+value.trim());});xvfb.once('error',reject);xvfb.once('exit',code=>reject(Error('Xvfb exited '+code)));});
  profile=fs.mkdtempSync(path.join(os.tmpdir(),'turts-terrain-'));
  context=await chromium.launchPersistentContext(profile,{executablePath,headless:false,viewport:{width:1280,height:720},env:{...process.env,DISPLAY:display},args:['--alsa-output-device=null','--enable-unsafe-webgpu','--enable-features=Vulkan','--use-angle=vulkan','--disable-vulkan-surface','--no-first-run','--no-default-browser-check']});
  const page=context.pages()[0];page.on('console',message=>fs.appendFileSync(path.join(output,'console.log'),message.type()+': '+message.text()+'\n'));page.on('pageerror',error=>result.errors.push(String(error)));await page.goto(url);await page.keyboard.press('F9');
  const read=()=>page.evaluate(async()=>{if(typeof window.turts_debug_request!=='function'||typeof window.turts_scene!=='function')return null;const raw=await window.turts_debug_request(),scene=window.turts_scene();return raw&&scene?{debug:JSON.parse(raw),scene:JSON.parse(scene),ui:typeof window.turts_ui==='function'?JSON.parse(window.turts_ui()):[]}:null;});
  const wait=async(predicate,message,timeout=90000)=>{const end=Date.now()+timeout;while(Date.now()<end){const s=await read();if(s&&predicate(s))return s;await sleep(75);}throw Error(message);};
  const capture=async name=>{const s=await read();fs.writeFileSync(path.join(output,name+'.json'),JSON.stringify(s,null,2));await page.screenshot({path:path.join(output,name+'.png')});result.states.push(name);return s;};
  const terrain=s=>s.debug.replicated_world.terrain,world=s=>s.debug.replicated_world;
  const unit=(s,content)=>world(s).units.find(u=>u.owner_slot===s.debug.slot&&u.content_id===content&&(!scenario.require_raised_picking||terrain(s).anchors.find(a=>a.unit===u.unit_id)?.elevation===0));
  const work=(s,id)=>terrain(s).work.find(w=>w.unit===id),order=(s,id)=>world(s).unit_orders.find(o=>o.unit===id);
  const surface=(s,cell)=>terrain(s).surfaces.find(r=>r.cell.x===cell.x&&r.cell.y===cell.y);
  const amount=(s,id)=>{const e=world(s).economy;return scenario.output_destination==='Inventory'?(e.inventories.find(i=>i.unit===id&&i.port==='main')?.items.find(i=>i.resource===scenario.resource)?.on_hand||0):(e.balances.find(b=>b.resource===scenario.resource)?.on_hand||0);};
  const contact=(s,id)=>{const anchor=terrain(s).anchors.find(a=>a.unit===id&&a.current);assert(anchor,'No current picking contact');return anchor.elevation*scenario.elevation_step;};
  const click=async(point,button='left')=>{assert(point.every(Number.isFinite)&&point[0]>0&&point[0]<1280&&point[1]>0&&point[1]<720,'Pointer target is outside viewport: '+point);result.actions.push({kind:'click',point,button});await page.mouse.click(...point,{button});};
  const select=async(content,height=scenario.body_pick_height,specific)=>{const s=await read(),u=specific?world(s).units.find(u=>u.unit_id===specific):unit(s,content);assert(u,'Owned unit missing: '+content);await click(project(s.scene,[...u.pos,contact(s,u.unit_id)+height]));await wait(s=>s.debug.selection?.includes(u.unit_id),'Actual height-aware body click did not select '+content);return u.unit_id;};
  const widget=async(predicate)=>{const s=await wait(s=>s.ui.some(w=>w.visible&&predicate(w)),'Requested real widget not visible');const w=s.ui.find(w=>w.visible&&predicate(w)),[x,y,width,height]=w.rect;await click([x+width/2,y+height/2]);};
  const button=id=>widget(w=>w.enabled&&w.widget_action?.kind==='order'&&w.widget_action.id===id);
  const chooseTool=tool=>widget(w=>w.enabled&&w.widget_action?.kind==='terrain_work'&&w.widget_action.tool===tool);
  const point=(cell,elevation)=>([(cell.x+.5)*scenario.cell_width,(cell.y+.5)*scenario.cell_width,elevation*scenario.elevation_step]);
  const begin=async(tool,reference)=>{await chooseTool(tool);const s=await read();await click(project(s.scene,reference));};
  const drag=async(a,b)=>{const s=await read(),start=project(s.scene,a),end=project(s.scene,b);result.actions.push({kind:'drag',start,end});await page.mouse.move(...start);await page.mouse.down();await page.mouse.move(...end,{steps:8});await page.mouse.up();};
  const designate=async(tool,reference,a,b=a)=>{await begin(tool,reference);await drag(a,b);};
  const stop=async id=>{await button(scenario.stop_order);await wait(s=>!work(s,id),'Stop did not cancel the active work');};
  const status=s=>s.ui.find(w=>w.id==='terrain_work_status'&&w.visible)?.text||'';
  let s=await wait(s=>s.debug.replicated_world?.terrain&&world(s).units.length,'Terrain did not activate');assert(s.debug.slot===scenario.slot,'Scenario joined wrong slot');
  const actor=await select(scenario.worker);await capture('01-selected');
  // Both actual wheel clamps, actual elevated body picking, and screenshot-visible fill ghosts.
  const zooms=[];
  for(const [zoom,delta] of [['near',-10000],['far',10000]]){
   s=await read();const raised=world(s).units.find(u=>u.owner_slot===s.debug.slot&&u.content_id===scenario.worker&&contact(s,u.unit_id)>0);assert(!scenario.require_raised_picking||raised,'No raised-body fixture');const u=raised||unit(s,scenario.worker),anchor=project(s.scene,[...u.pos,contact(s,u.unit_id)]);await page.mouse.move(...anchor);await page.mouse.wheel(0,delta);await sleep(350);
   const first=await read();await page.mouse.wheel(0,delta);await sleep(200);const clamped=await read();assert(matrixDelta(first.scene.clip_from_world,clamped.scene.clip_from_world)<1e-4,'Wheel did not clamp at '+zoom);if(raised)await select(scenario.worker,scenario.body_pick_height,raised.unit_id);await select(scenario.worker);
   await begin(scenario.fill,scenario.ghost_reference);s=await read();const flat=[...scenario.ghost_point,0],p=project(s.scene,flat);await page.mouse.move(...p);await sleep(100);const before=await page.screenshot();await page.mouse.down();await sleep(150);await wait(s=>/Target level/.test(status(s))&&/1 cells/.test(status(s)),'No real rectangle preview at '+zoom);const ghost=await capture('ghost-'+zoom);const after=fs.readFileSync(path.join(output,'ghost-'+zoom+'.png'));const target=project(ghost.scene,[...scenario.ghost_point,scenario.ghost_reference[2]]),edge=project(ghost.scene,[scenario.ghost_point[0]+scenario.cell_width/2,scenario.ghost_point[1]+scenario.cell_width/2,scenario.ghost_reference[2]]),pixels=ghostPixels(before,after,target,Math.max(4,Math.hypot(edge[0]-target[0],edge[1]-target[1])+4));assert(pixels>=2,'Ghost is not screenshot-visible at '+zoom);
   await page.keyboard.press('Escape');await page.mouse.up();await wait(s=>s.debug.pending_order_id===null&&!work(s,actor),'Escape did not cancel the unsubmitted ghost');
   await begin(scenario.dig,scenario.reference);s=await read();const cutPoint=project(s.scene,scenario.ghost_reference);await page.mouse.move(...cutPoint);await sleep(100);const cutBefore=await page.screenshot();await page.mouse.down();await sleep(150);await wait(s=>/Target level/.test(status(s))&&/1 cells/.test(status(s)),'No cut ghost at '+zoom);const cutState=await capture('cut-ghost-'+zoom),cutAfter=fs.readFileSync(path.join(output,'cut-ghost-'+zoom+'.png')),cutCenter=project(cutState.scene,scenario.ghost_reference),cutEdge=project(cutState.scene,[scenario.ghost_reference[0]+scenario.cell_width/2,scenario.ghost_reference[1]+scenario.cell_width/2,scenario.ghost_reference[2]]),cutPixels=ghostPixels(cutBefore,cutAfter,cutCenter,Math.max(4,Math.hypot(cutEdge[0]-cutCenter[0],cutEdge[1]-cutCenter[1])+4),'green');assert(cutPixels>=2,'Buried cut target has no visible current-top outline at '+zoom);await page.keyboard.press('Escape');await page.mouse.up();await wait(s=>s.debug.pending_order_id===null&&!work(s,actor),'Cut ghost was submitted instead of cancelled');
   zooms.push({zoom,matrix:clamped.scene.clip_from_world,fill_ghost_pixels:pixels,cut_top_outline_pixels:cutPixels,raised_body_selected:raised?.unit_id,body_selected:actor});
  }
  assert(matrixDelta(zooms[0].matrix,zooms[1].matrix)>.01,'Zoom extremes are identical');result.checks.zoom_ghost_picking=zooms;
  // Preserve observable partial progress through an ordinary Disable/Enable pause.
  const partial=scenario.partial_cell,partialTop=scenario.initial_ridge_elevation;
  await designate(scenario.dig,scenario.reference,point(partial,partialTop));
  s=await wait(s=>{const w=work(s,actor);return w&&w.completed_work>0&&w.completed_work<w.required_work;},'No observable partial work');const initialOrder=order(s,actor).current;
  await button(scenario.pause_order);s=await wait(s=>['Disabled','Paused'].includes(work(s,actor)?.wait),'Pause did not retain active work');const paused=work(s,actor),pausedTick=world(s).tick;assert(paused.completed_work>0&&paused.completed_work<paused.required_work,'Pause lost the partial prism');assert(JSON.stringify(order(s,actor).current)===JSON.stringify(initialOrder),'Pause replaced the public work target');await capture('02-partial-paused');
  s=await wait(s=>world(s).tick>=pausedTick+6,'No completed ticks during pause');assert(JSON.stringify(work(s,actor))===JSON.stringify(paused),'Paused public work changed');
  await button(scenario.resume_order);s=await wait(s=>surface(s,partial)?.elevation===partialTop-1,'Resumed work did not commit its first prism');assert(JSON.stringify(order(s,actor).current)===JSON.stringify(initialOrder),'Resume recreated a different public work target');result.checks.pause_resume={before:paused,after:work(s,actor),order:initialOrder};
  await wait(s=>work(s,actor)?.completed_work>0,'No new partial prism before cancellation');await stop(actor);const canceled=await capture('03-active-cancelled'),cancelTick=world(canceled).tick,cancelLevel=surface(canceled,partial).elevation,cancelAmount=amount(canceled,actor);s=await wait(s=>world(s).tick>=cancelTick+6,'No ticks after Stop');assert(surface(s,partial).elevation===cancelLevel&&amount(s,actor)===cancelAmount,'Cancelled work kept editing or paying');assert(cancelLevel===partialTop-1,'Cancellation fixture did not retain exactly the first committed prism');result.checks.cancel={level:cancelLevel,amount:cancelAmount};
  // Static occupied foundations refuse an otherwise ordinary fill designation.
  await designate(scenario.fill,scenario.ghost_reference,point(scenario.refusal_cell,0));s=await wait(s=>work(s,actor)?.wait==='Occupied','Static foundation did not report an occupied refusal');assert(/occupies/.test(status(s)),'Occupied status is not presented');const refusal=await capture('04-foundation-refusal'),refusalTick=world(refusal).tick,refusalAmount=amount(refusal,actor);s=await wait(s=>world(s).tick>=refusalTick+6,'No refusal ticks');assert(surface(s,scenario.refusal_cell)?.current&&surface(s,scenario.refusal_cell).elevation===0&&amount(s,actor)===refusalAmount,'Refused work edited or charged');await stop(actor);result.checks.refusal='Occupied; no edit/payment';
  await designate(scenario.dig,scenario.reference,scenario.rectangle[0],scenario.rectangle[1]);
  const min=scenario.cells.min,max=scenario.cells.max;
  const done=s=>{for(let y=min.y;y<max.y;y++)for(let x=min.x;x<max.x;x++){const row=surface(s,{x,y});if(!row||!row.current||row.elevation!==scenario.target_elevation)return false;}return true;};
  // Append an ordinary targeted unload during a nonzero partial prism. No replacement work packet.
  if(scenario.require_unload){
   s=await wait(s=>{const w=work(s,actor);return amount(s,actor)>0&&w?.completed_work>0&&w.completed_work<w.required_work;},'No partial/cargo window before appended utility');const original=order(s,actor),before=work(s,actor);
   await page.keyboard.down('Shift');try{await button(scenario.unload_order);s=await read();const depot=unit(s,scenario.depot);await click(project(s.scene,[...depot.pos,contact(s,depot.unit_id)+scenario.depot_pick_height]));}finally{await page.keyboard.up('Shift');}
   s=await wait(s=>work(s,actor)?.wait==='Unloading','Appended unload did not suspend work');const unloading=work(s,actor);assert(unloading.completed_work>0&&JSON.stringify(unloading.designation)===JSON.stringify(before.designation),'Utility lost the partial original work');const utilityTick=world(s).tick;await capture('05-partial-unloading');
   s=await wait(s=>world(s).tick>utilityTick&&JSON.stringify(order(s,actor)?.current)===JSON.stringify(original.current)&&work(s,actor)?.current_cell?.x===unloading.current_cell.x&&work(s,actor)?.current_cell?.y===unloading.current_cell.y&&work(s,actor).required_work!==null,'Original partial work did not resume after unload');assert(work(s,actor).completed_work>=unloading.completed_work,'Unload reset partial work');assert(JSON.stringify(order(s,actor).queued)===JSON.stringify(original.queued),'Unload changed the pending queue');result.checks.unload_partial={before,unloading,resumed:work(s,actor),public_order:original.current};
  }
  const end=Date.now()+300000;let unloads=0;
  while(Date.now()<end){s=await read();if(done(s))break;if(work(s,actor)?.wait==='FullCargo'){assert(unloads<32,'Unbounded unloading loop');await capture('cargo-'+unloads);await widget(w=>w.enabled&&w.widget_action?.kind==='unload_resume'&&w.widget_action.unit===actor);unloads++;await wait(s=>work(s,actor)?.wait!=='FullCargo','Unload did not resume work');}else await sleep(150);}
  assert(done(s),'Work rectangle did not reach the captured reference level');result.unloads=unloads;await capture('06-passage-open');
  const rover=await select(scenario.rover);s=await read();await click(project(s.scene,scenario.destination),'right');await wait(s=>{const u=unit(s,scenario.rover);return u&&Math.hypot(u.pos[0]-scenario.destination[0],u.pos[1]-scenario.destination[1])<1;},'Wide rover did not traverse the opened passage',120000);await capture('07-rover-through');result.checks.rover=rover;
  // Fill one cleared cell using its known partial-cut reference, then remove placed fill with NoYield.
  await select(scenario.worker);s=await read();const fillCell=min,fillLevel=cancelLevel,beforeFill=amount(s,actor),fillReference=point(partial,fillLevel),fillPoint=point(fillCell,0);
  await designate(scenario.fill,fillReference,fillPoint);s=await wait(s=>surface(s,fillCell)?.elevation===fillLevel&&!work(s,actor),'Fill did not reach its captured reference level');const filled=await capture('08-filled');assert(surface(filled,fillCell).provenance==='Placed','Fill did not publish explicit Placed provenance');assert(amount(filled,actor)<beforeFill,'Fill consumed no public input');result.checks.fill={before:beforeFill,after:amount(filled,actor),level:fillLevel};
  await begin(scenario.dig,scenario.reference);s=await read();const cut=project(s.scene,point(fillCell,fillLevel));await page.mouse.move(...cut);await page.mouse.down();await wait(s=>/NoYield/.test(status(s)),'Placed preview did not present NoYield');await capture('09-noyield-preview');await page.mouse.up();s=await wait(s=>surface(s,fillCell)?.elevation===0&&!work(s,actor),'Placed material removal did not finish');assert(amount(s,actor)===amount(filled,actor),'Placed fill manufactured natural resources');await capture('09-noyield');result.checks.no_yield=true;
  assert(result.errors.length===0,'Browser errors');result.status='PASS';
 }catch(error){if(context){const page=context.pages()[0];await page.screenshot({path:path.join(output,'failure.png')}).catch(()=>{});result.failure_state=await page.evaluate(()=>({error:window.turts_error,debug:window.turts_debug?.(),scene:window.turts_scene?.(),ui:window.turts_ui?.(),body:document.body.innerText})).catch(String);}result.status='FAIL';result.error=String(error.stack||error);process.exitCode=1;console.error(error);}
 finally{if(context)await context.close();if(profile)fs.rmSync(profile,{recursive:true,force:true});if(xvfb&&xvfb.exitCode===null){const stopped=new Promise(resolve=>xvfb.once('exit',resolve));xvfb.kill('SIGTERM');await Promise.race([stopped,sleep(2000)]);if(xvfb.exitCode===null&&xvfb.signalCode===null){xvfb.kill('SIGKILL');await stopped;}}result.profile_removed=!profile||!fs.existsSync(profile);result.xvfb_stopped=!xvfb||xvfb.exitCode!==null||xvfb.signalCode!==null;fs.writeFileSync(path.join(output,'result.json'),JSON.stringify(result,null,2));}
})();
