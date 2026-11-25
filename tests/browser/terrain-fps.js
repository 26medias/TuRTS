// Editable terrain adaptation of SDK16's High400 RAF instrument. No runtime assets are generated.
// Existing accepted combat script is byte-preserved. Terrain work is issued by real UI input.
'use strict';
const fs=require('node:fs'),os=require('node:os'),path=require('node:path'),net=require('node:net'),crypto=require('node:crypto');
const {spawn,spawnSync}=require('node:child_process'),{chromium}=require('playwright-core');
const {movementIntervals}=require('./terrain-fps-intervals');
const {orderAccepted}=require('./terrain-fps-orders');
const [prepared,fixtures,output]=process.argv.slice(2);if(!output)throw Error('PREPARED TERRAIN_FIXTURES OUTPUT');
const metadata=JSON.parse(fs.readFileSync(path.join(fixtures,'terrain-fps.json')));fs.mkdirSync(output,{recursive:false});
const selectedViews=process.env.TURTS_TERRAIN_FPS_VIEWS?process.env.TURTS_TERRAIN_FPS_VIEWS.split(','):metadata.zooms;if(!selectedViews.length||new Set(selectedViews).size!==selectedViews.length||selectedViews.some(view=>!metadata.zooms.includes(view)))throw Error('Invalid selected camera views');
const groupMode=process.env.TURTS_TERRAIN_FPS_GROUP==='1';
const result={status:'RUNNING',group_movement:groupMode,metadata,selected_views:selectedViews,omitted_views:metadata.zooms.filter(view=>!selectedViews.includes(view)),rows:[],errors:[]},save=()=>fs.writeFileSync(path.join(output,'result.json'),JSON.stringify(result,null,2));
const sleep=ms=>new Promise(r=>setTimeout(r,ms)),assert=(ok,message)=>{if(!ok)throw Error(message);};
const port=()=>new Promise(resolve=>{const listener=net.createServer();listener.listen(0,'127.0.0.1',()=>{const value=listener.address().port;listener.close(()=>resolve(value));});});
function project(scene,[x,y,z=0]){const m=scene.clip_from_world,v=[x,z,y,1],c=[0,1,2,3].map(r=>v.reduce((n,k,i)=>n+m[i*4+r]*k,0));const [left,top,w,h]=scene.viewport_logical_rect;return[left+(c[0]/c[3]+1)*w/2,top+(1-c[1]/c[3])*h/2];}
function eyeHeight(scene){const m=scene.clip_from_world,a=[0,1,3].map(r=>[m[r],m[4+r],m[8+r],-m[12+r]]);for(let c=0;c<3;c++){let pivot=c;for(let r=c+1;r<3;r++)if(Math.abs(a[r][c])>Math.abs(a[pivot][c]))pivot=r;[a[c],a[pivot]]=[a[pivot],a[c]];const scale=a[c][c];assert(Math.abs(scale)>1e-10,'Singular camera');for(let k=c;k<4;k++)a[c][k]/=scale;for(let r=0;r<3;r++)if(r!==c){const f=a[r][c];for(let k=c;k<4;k++)a[r][k]-=f*a[c][k];}}return a[1][3];}
const delta=(a,b)=>Math.max(...a.map((n,i)=>Math.abs(n-b[i])));
(async()=>{let xvfb,context,profile,server;try{
 assert(metadata.actors===400&&metadata.quality==='high'&&metadata.sample_ms===10000&&metadata.fps_floor===30&&JSON.stringify(metadata.zooms)===JSON.stringify(['normal','near','far']),'Unexpected frozen workload');
 const checked=spawnSync('python3',['-B',path.resolve(__dirname,'terrain-prepared.py'),prepared,'--root',path.resolve(__dirname,'../..')],{encoding:'utf8'});fs.writeFileSync(path.join(output,'prepared-check.log'),checked.stdout+checked.stderr);assert(checked.status===0,'Prepared candidate does not match source');
 const sha=file=>crypto.createHash('sha256').update(fs.readFileSync(file)).digest('hex');assert(sha(path.join(fixtures,'battle400-commands.json'))===metadata.commands_sha256,'Accepted combat commands changed');for(const [file,hash]of Object.entries(metadata.public_files_sha256)){assert(sha(path.join(fixtures,'battle400',file))===hash,'Frozen public fixture changed: '+file);}
 xvfb=spawn('Xvfb',['-displayfd','3','-screen','0','1280x720x24','-nolisten','tcp'],{stdio:['ignore','ignore','pipe','pipe']});
 const display=await new Promise((resolve,reject)=>{let text='';xvfb.stdio[3].on('data',data=>{text+=data;if(text.includes('\n'))resolve(':'+text.trim());});xvfb.once('error',reject);xvfb.once('exit',code=>reject(Error('Xvfb '+code)));});
 for(const zoom of selectedViews){
  const rowOut=path.join(output,zoom);fs.mkdirSync(rowOut);const packed=spawnSync(path.join(prepared,'bin/turts-pack'),[path.join(fixtures,'battle400')],{encoding:'utf8'});fs.writeFileSync(path.join(rowOut,'pack.log'),packed.stdout+packed.stderr);assert(packed.status===0,'Terrain FPS public package validation failed');
  const privateDir=path.join(rowOut,'private');fs.mkdirSync(privateDir,{mode:0o700});fs.writeFileSync(path.join(privateDir,'seed.txt'),'0\n',{mode:0o600});const http=await port(),ws=await port(),url=`http://127.0.0.1:${http}`;
  const config=path.join(rowOut,'server.ron'),q=JSON.stringify;fs.writeFileSync(config,`(game_package:${q(path.resolve(fixtures,'battle400.tar'))},http_listen:"127.0.0.1:${http}",ws_listen:"127.0.0.1:${ws}",public_url:${q(url)},tick_hz:20,static_dir:Some(${q(path.resolve(prepared,'dist'))}),start_policy:Some(Immediate),terrain_seed_file:Some(${q(path.resolve(privateDir,'seed.txt'))}),terrain_provenance:Some(${q(path.resolve(privateDir,'terrain.json'))}))`);
  const fd=fs.openSync(path.join(rowOut,'server.log'),'w');server=spawn(path.join(prepared,'bin/turts_server'),[config,'--command-script',path.join(fixtures,'battle400-commands.json')],{stdio:['ignore',fd,fd],env:{...process.env,TURTS_COMPUTER_SLOT:''}});fs.closeSync(fd);
  let ready=false;for(let i=0;i<300;i++){try{if((await fetch(url+'/advertisement.json')).ok){ready=true;break;}}catch{}assert(server.exitCode===null,'Server stopped');await sleep(100);}assert(ready,'Server not ready');
  profile=fs.mkdtempSync(path.join(os.tmpdir(),'turts-terrain-fps-'));context=await chromium.launchPersistentContext(profile,{executablePath:'/usr/bin/google-chrome',headless:false,viewport:{width:1280,height:720},env:{...process.env,DISPLAY:display},ignoreDefaultArgs:['--enable-unsafe-swiftshader'],args:['--enable-gpu','--alsa-output-device=null','--enable-unsafe-webgpu','--enable-features=Vulkan','--use-angle=vulkan','--disable-vulkan-surface','--no-first-run','--no-default-browser-check']});
  await context.addInitScript(()=>localStorage.setItem('turts.v1.settings',JSON.stringify({controls:{quality:'high'}})));const page=context.pages()[0];page.on('pageerror',error=>result.errors.push(String(error)));await page.goto(url+'/?server='+encodeURIComponent(url)+'&ws='+encodeURIComponent(`ws://127.0.0.1:${ws}`));await page.keyboard.press('F9');
  const click=async(x,y)=>{await page.mouse.move(x,y);await sleep(80);await page.mouse.down();await sleep(80);await page.mouse.up();await sleep(80);};
  const scene=()=>page.evaluate(()=>new Promise(resolve=>{if(typeof turts_scene!=='function')return resolve(null);turts_scene();requestAnimationFrame(()=>requestAnimationFrame(()=>{const value=turts_scene();resolve(value?JSON.parse(value):null);}));})),read=()=>page.evaluate(async()=>typeof turts_debug_request==='function'?JSON.parse(await turts_debug_request()):null);
  const wait=async(get,predicate,message)=>{const end=Date.now()+90000;while(Date.now()<end){const s=await get();if(s&&predicate(s))return s;assert(!result.errors.length,'Browser errors');await sleep(100);}throw Error(message);};
  let s=await wait(scene,s=>s.units?.length===400&&s.models?.filter(m=>m.unit_id!=null).length===400&&s.models.every(m=>m.ready)&&s.animations?.length===400,'Detailed400 scene did not load');assert(s.animations.filter(a=>a.visible&&!a.dead).length===400,'Normal camera did not retain all400 visible actors');assert(s.models.filter(m=>m.unit_id!=null).every(m=>m.quality==='high'&&m.level===0),'Detailed model quality changed');
  const adapter=await page.evaluate(async()=>{const a=await navigator.gpu.requestAdapter();return {...a.info.toJSON?.(),isFallbackAdapter:a.info.isFallbackAdapter};});assert(!adapter.isFallbackAdapter,'Software GPU');
  let d=await wait(read,d=>d.replicated_world?.terrain,'No permitted editable terrain');const actor=d.replicated_world.units.find(u=>u.unit_id===metadata.work.actor);assert(actor&&actor.owner_slot===d.slot,'Intended existing tool actor is not owned');const body=project(s,[...actor.pos,.9]);await click(...body);await wait(read,d=>d.selection?.includes(metadata.work.actor),'Real body selection failed');
  const ui=await wait(()=>page.evaluate(()=>JSON.parse(turts_ui())),rows=>rows.some(w=>w.visible&&w.enabled&&w.widget_action?.tool===metadata.work.tool),'Real tool button missing');const tool=ui.find(w=>w.visible&&w.enabled&&w.widget_action?.tool===metadata.work.tool),[x,y,w,h]=tool.rect;await click(x+w/2,y+h/2);s=await scene();await click(...project(s,metadata.work.reference));s=await scene();await click(...project(s,metadata.work.point));
  d=await wait(read,d=>d.replicated_world.terrain.work.some(w=>w.unit===metadata.work.actor&&w.completed_work>0),'Actual terrain work never progressed');await sleep(metadata.warmup_ms);
  let groupIds=[];
  if(groupMode){
   const goal=[14,32,0];
   const candidates=d.replicated_world.units.filter(u=>u.owner_slot===d.slot&&u.unit_id!==metadata.work.actor&&Math.hypot(u.pos[0]-goal[0],u.pos[1]-goal[1])>10).sort((a,b)=>b.pos[1]-a.pos[1]||Math.abs(a.pos[0]-14)-Math.abs(b.pos[0]-14));
   assert(candidates.length>=2,'Not enough owned actors for the group move');
   // Select separate owned bodies at their current physical locations.
   for(const u of candidates){
    s=await scene();d=await read();const current=d.replicated_world.units.find(v=>v.unit_id===u.unit_id);if(!current)continue;
    const p=project(s,[...current.pos,.9]);if(p[0]<10||p[0]>1270||p[1]<10||p[1]>710)continue;
    if(groupIds.length)await page.keyboard.down('Shift');
    await click(...p);await page.keyboard.up('Shift');
    d=await read();if(!d.selection?.includes(u.unit_id))continue;
    groupIds=d.selection.filter(id=>id!==metadata.work.actor);
    if(groupIds.length>=2)break;
   }
   assert(groupIds.length>=2&&!d.selection.includes(metadata.work.actor),'Could not select two owned moving actors');
   s=await scene();const point=project(s,goal);assert(point[0]>0&&point[0]<1280&&point[1]>0&&point[1]<720,'Group goal outside viewport');
   // A short move can start and end between two slow reads: accept a new command result or the arrival too.
   let seqs=new Set((await read()).command_results.map(r=>r.seq));
   await page.mouse.move(...point);await page.mouse.click(...point,{button:'right',delay:80});
   await wait(read,x=>orderAccepted(seqs,x,groupIds,':order_move',p=>p[1]>29),'Group move was not accepted');
   await wait(read,x=>groupIds.every(id=>{const u=x.replicated_world.units.find(v=>v.unit_id===id);return u&&u.pos[1]>29;}),'Group did not reach clear terrain');
   seqs=new Set((await read()).command_results.map(r=>r.seq));
   s=await scene();const onward=project(s,[43,32,0]);await page.mouse.move(...onward);await page.mouse.click(...onward,{button:'right',delay:80});
   await wait(read,x=>orderAccepted(seqs,x,groupIds,':order_move',p=>Math.hypot(p[0]-43,p[1]-32)<3),'Onward move was not accepted');
   // Queue return trips before zoom so the same two units keep moving during the sample.
   // Three legs: with two, the faster unit arrived at about 9.1 s of the 10 s sample.
   for(const destination of [[14,32,0],[43,32,0],[14,32,0]]){
    s=await scene();const queued=project(s,destination);assert(queued[0]>0&&queued[0]<1280&&queued[1]>0&&queued[1]<720,'Queued goal outside viewport');
    await page.keyboard.down('Shift');await sleep(150);await page.mouse.move(...queued);await page.mouse.click(...queued,{button:'right',delay:80});await sleep(150);await page.keyboard.up('Shift');await sleep(150);
   }
   d=await read();fs.writeFileSync(path.join(rowOut,'queued.json'),JSON.stringify({ids:groupIds,orders:d.replicated_world.unit_orders.filter(o=>groupIds.includes(o.unit))},null,2));assert(groupIds.every(id=>d.replicated_world.unit_orders.some(o=>o.unit===id&&o.queued.length>=1)),'Return moves were not queued');
  }
  if(zoom==='normal') {
   s=await scene();assert(Math.abs(eyeHeight(s)-metadata.camera_bounds.normal)<.05,'Normal authored camera height changed');assert(s.animations.filter(a=>a.visible&&!a.dead).length===400,'Normal sample must keep all400 detailed actors visible');
  } else {
   await page.mouse.move(640,360);await page.mouse.wheel(0,zoom==='near'?-10000:10000);await sleep(400);const clamped=await scene();await page.mouse.wheel(0,zoom==='near'?-10000:10000);await sleep(200);s=await scene();assert(delta(clamped.clip_from_world,s.clip_from_world)<1e-4,'Camera did not reach '+zoom+' clamp');assert(Math.abs(eyeHeight(s)-metadata.camera_bounds[zoom==='near'?'minimum':'maximum'])<.05,'Authored zoom bound changed');
  }
  const before=await read();fs.writeFileSync(path.join(rowOut,'before.json'),JSON.stringify({debug:before,scene:s},null,2));await page.screenshot({path:path.join(rowOut,'before.png')});
  const sample=await page.evaluate(({duration,groupIds})=>new Promise(resolve=>{let first,last,peak=0;const frames=[],states=[];function frame(now){const due=!states.length||now-states.at(-1).at>=500||(first!==undefined&&now-first>=duration);peak=Math.max(peak,turts_memory_bytes());if(first===undefined)first=now;if(last!==undefined)frames.push(now-last);last=now;if(due){const s=JSON.parse(turts_scene());states.push({at:now,group_positions:s.units.filter(u=>groupIds.includes(u.unit_id)).map(u=>({unit:u.unit_id,position:u.translation})),units:s.units.length,visible:s.animations.filter(a=>a.visible&&!a.dead).length,shooting:s.animations.filter(a=>/2H_Ranged|1H_Melee/.test(a.clip||'')).length,attacking_ids:s.animations.filter(a=>/2H_Ranged|1H_Melee/.test(a.clip||'')).map(a=>a.unit_id),melee:s.animations.filter(a=>/1H_Melee/.test(a.clip||'')).length,ranged:s.animations.filter(a=>/2H_Ranged/.test(a.clip||'')).length,work_clips:s.animations.filter(a=>/Interact/.test(a.clip||'')).length,models:s.models.filter(m=>m.unit_id!=null).map(m=>({unit:m.unit_id,ready:m.ready,quality:m.quality,level:m.level})),shared_skins:s.shared_skins,effects:s.effects,clip_from_world:s.clip_from_world});}if(now-first>=duration)return resolve({frames_ms:frames,states,wasm_high_water_bytes:peak});requestAnimationFrame(frame);}requestAnimationFrame(frame);}),{duration:metadata.sample_ms,groupIds});
  fs.writeFileSync(path.join(rowOut,'raw-sample.json'),JSON.stringify(sample));
  const after=await read(),work=after.replicated_world.terrain.work.find(w=>w.unit===metadata.work.actor),cell=metadata.work.cell,top=d=>d.replicated_world.terrain.surfaces.find(r=>r.cell.x===cell.x&&r.cell.y===cell.y)?.elevation;
  assert(work&&work.required_work!==null&&['Unknown','Occupied','Blocked','Disabled','Invalid','FullCargo'].every(wait=>work.wait!==wait),'Work stopped or was refused during FPS window');assert(top(after)<top(before),'No committed edits during the FPS interval');assert(JSON.stringify(work.designation.cells)===JSON.stringify(before.replicated_world.terrain.work.find(w=>w.unit===metadata.work.actor).designation.cells),'Work target changed');
  const sorted=[...sample.frames_ms].sort((a,b)=>a-b),pct=p=>sorted[Math.min(sorted.length-1,Math.ceil(sorted.length*p)-1)],fps=1000*sample.frames_ms.length/sample.frames_ms.reduce((a,b)=>a+b,0);const row={zoom,quality:'high',actors:400,fps,p99_ms:pct(.99),max_ms:sorted.at(-1),camera_height:eyeHeight(s),adapter,before_level:top(before),after_level:top(after),work_before:before.replicated_world.terrain.work.find(w=>w.unit===metadata.work.actor),work_after:work,...sample};
  result.rows.push(row);save();
  fs.writeFileSync(path.join(rowOut,'after.json'),JSON.stringify({debug:after,scene:await scene()},null,2));
  assert(zoom!=='normal'||sample.states.every(s=>s.visible===400),'Normal sample lost visible actors');assert(sample.states.every(s=>s.units===400&&s.models.length===400&&s.models.every(m=>m.ready&&m.quality==='high'&&m.level===0)),'400 detailed High actor workload changed');assert(sample.states.every(s=>s.shared_skins?.enabled&&s.shared_skins.matrices<=65536),'Bounded High shared skin path missing');const actors=before.replicated_world.units,melee=actors.filter(u=>/:(warrior|swordsman)$/.test(u.content_id)).length,ranged=actors.filter(u=>u.content_id==='skirmish:soldier').length-1;
  const observedAttack=new Set(sample.states.flatMap(s=>s.attacking_ids));
  if(!groupMode){
   assert(actors.every(u=>u.unit_id===metadata.work.actor||observedAttack.has(u.unit_id)),'A combatant never played its accepted attack animation during the sample');
   assert(sample.states.some(s=>s.melee===melee&&s.ranged>=ranged),'Accepted mixed attack animation coverage was not observed');
  }else{
   const moved=groupIds.filter(id=>{const a=before.replicated_world.units.find(u=>u.unit_id===id),b=after.replicated_world.units.find(u=>u.unit_id===id);return a&&b&&Math.hypot(a.pos[0]-b.pos[0],a.pos[1]-b.pos[1])>.01;});
   row.group_ids=groupIds;row.group_moved=moved;assert(moved.length>=2,'Fewer than two selected group members moved during the FPS sample');
   const intervals=movementIntervals(sample.states,groupIds);
   row.group_intervals=intervals;assert(intervals.length>=1&&intervals.every(x=>x.active.length>=2),'Fewer than two group members moved in an FPS sample interval');
  }
  const commands=JSON.parse(fs.readFileSync(path.join(fixtures,'battle400-commands.json'))),formerTarget=commands.find(c=>c.envelope.units.includes(metadata.work.actor)).envelope.target.Unit;
  const firstHealth=sample.states[0].effects.combat.health,lastHealth=sample.states.at(-1).effects.combat.health;
  row.expected_mix={melee,ranged};row.former_tool_target=formerTarget;row.damaged_units=actors.filter(u=>lastHealth[u.unit_id]<firstHealth[u.unit_id]).map(u=>u.unit_id);
  save();
  assert(sample.states.every(s=>Object.keys(s.effects.combat.health).length===400&&Object.values(s.effects.combat.health).every(h=>h>0)),'Accepted live combat actor set changed');
  if(!groupMode)assert(actors.every(u=>u.unit_id===formerTarget||lastHealth[u.unit_id]<firstHealth[u.unit_id]),'A required victim stopped taking damage during mixed combat');assert(fps>=30,'High400 terrain FPS floor failed');assert(!result.errors.length,'Browser errors');
  fs.writeFileSync(path.join(rowOut,'after.json'),JSON.stringify({debug:after,scene:await scene()},null,2));await page.screenshot({path:path.join(rowOut,'after.png')});row.validation_status='PASS';save();await context.close();context=null;fs.rmSync(profile,{recursive:true,force:true});profile=null;server.kill('SIGTERM');await new Promise(resolve=>server.once('exit',resolve));server=null;
 }
 result.status=selectedViews.length===metadata.zooms.length?'PASS':'PASS_SELECTED';
}catch(error){result.status='FAIL';result.error=String(error.stack||error);process.exitCode=1;console.error(error);}
finally{if(context)await context.close();if(profile)fs.rmSync(profile,{recursive:true,force:true});if(server){server.kill('SIGTERM');await new Promise(resolve=>server.once('exit',resolve));}if(xvfb&&xvfb.exitCode===null){const stopped=new Promise(resolve=>xvfb.once('exit',resolve));xvfb.kill('SIGTERM');await Promise.race([stopped,sleep(2000)]);if(xvfb.exitCode===null&&xvfb.signalCode===null){xvfb.kill('SIGKILL');await stopped;}}result.xvfb_stopped=!xvfb||xvfb.exitCode!==null||xvfb.signalCode!==null;result.profile_removed=!profile||!fs.existsSync(profile);save();}
})();
