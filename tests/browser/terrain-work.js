// Real-input terrain path. The caller owns the prepared server; this driver owns
// one fresh Chrome profile and one Xvfb. It never injects commands or changes state.
// node tests/browser/terrain-work.js URL SCENARIO.json OUTPUT [CHROME]
'use strict';
const fs=require('node:fs'),os=require('node:os'),path=require('node:path');
const {spawn}=require('node:child_process');
const {chromium}=require('playwright-core');
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
(async()=>{
 const result={status:'RUNNING',errors:[],actions:[],states:[],scope:'real tool/reference/rectangle/unload/resume/passsage input; rendering and FPS need separate checks'};
 let context,xvfb,profile;
 try {
  xvfb=spawn('Xvfb',['-displayfd','3','-screen','0','1280x720x24','-nolisten','tcp'],{stdio:['ignore','ignore','pipe','pipe']});
  const display=await new Promise((resolve,reject)=>{let value='';xvfb.stdio[3].on('data',data=>{value+=data;if(value.includes('\n'))resolve(':'+value.trim());});xvfb.once('error',reject);xvfb.once('exit',code=>reject(Error('Xvfb exited '+code)));});
  profile=fs.mkdtempSync(path.join(os.tmpdir(),'turts-terrain-'));
  context=await chromium.launchPersistentContext(profile,{executablePath,headless:false,viewport:{width:1280,height:720},env:{...process.env,DISPLAY:display},args:['--alsa-output-device=null','--enable-unsafe-webgpu','--enable-features=Vulkan','--use-angle=vulkan','--disable-vulkan-surface','--no-first-run','--no-default-browser-check']});
  const page=context.pages()[0];page.on('pageerror',error=>result.errors.push(String(error)));await page.goto(url);await page.keyboard.press('F9');
  const read=()=>page.evaluate(async()=>{if(typeof window.turts_debug_request!=='function'||typeof window.turts_scene!=='function')return null;const raw=await window.turts_debug_request(),scene=window.turts_scene();return raw&&scene?{debug:JSON.parse(raw),scene:JSON.parse(scene),ui:typeof window.turts_ui==='function'?JSON.parse(window.turts_ui()):[]}:null;});
  const wait=async(predicate,message,timeout=90000)=>{const end=Date.now()+timeout;while(Date.now()<end){const s=await read();if(s&&predicate(s))return s;await sleep(100);}throw Error(message);};
  const capture=async(name)=>{const s=await read();fs.writeFileSync(path.join(output,name+'.json'),JSON.stringify(s,null,2));await page.screenshot({path:path.join(output,name+'.png')});result.states.push(name);return s;};
  const click=async(point,button='left')=>{result.actions.push({kind:'click',point,button});await page.mouse.click(...point,{button});};
  const unit=(s,content)=>s.debug.replicated_world.units.find(u=>u.owner_slot===s.debug.slot&&u.content_id===content);
  const select=async(content)=>{const s=await read(),u=unit(s,content);assert(u,'Owned unit missing: '+content);await click(project(s.scene,[...u.pos,scenario.body_pick_height]));await wait(s=>s.debug.selection?.includes(u.unit_id),'Body click did not select '+content);return u.unit_id;};
  const widget=async(predicate)=>{const s=await wait(s=>s.ui.some(w=>w.visible&&predicate(w)),'Requested real widget not visible');const w=s.ui.find(w=>w.visible&&predicate(w)),[x,y,width,height]=w.rect;await click([x+width/2,y+height/2]);};
  const chooseTool=tool=>widget(w=>w.enabled&&w.action?.kind==='terrain_work'&&w.action.tool===tool);
  const designate=async(tool,reference,a,b)=>{await chooseTool(tool);let s=await read();await click(project(s.scene,reference));s=await read();const start=project(s.scene,a),end=project(s.scene,b);result.actions.push({kind:'drag',start,end});await page.mouse.move(...start);await page.mouse.down();await page.mouse.move(...end,{steps:8});await page.mouse.up();};
  let s=await wait(s=>s.debug.replicated_world?.terrain&&s.debug.replicated_world.units.length,'Terrain did not activate');
  assert(s.debug.slot===scenario.slot,'Scenario joined wrong slot');
  const actor=await select(scenario.worker);await capture('01-selected');
  await designate(scenario.dig,scenario.reference,scenario.rectangle[0],scenario.rectangle[1]);
  const min=scenario.cells.min,max=scenario.cells.max;
  const done=s=>{const rows=s.debug.replicated_world.terrain.surfaces;for(let y=min.y;y<max.y;y++)for(let x=min.x;x<max.x;x++){const row=rows.find(row=>row.cell.x===x&&row.cell.y===y);if(!row||!row.current||row.elevation!==scenario.target_elevation)return false;}return true;};
  const end=Date.now()+240000;let unloads=0;
  while(Date.now()<end){s=await read();if(done(s))break;const work=s.debug.replicated_world.terrain.work.find(w=>w.unit===actor);if(work?.wait==='FullCargo'){assert(unloads<32,'Unbounded unloading loop');await capture('cargo-'+unloads);await widget(w=>w.enabled&&w.action?.kind==='unload_resume'&&w.action.unit===actor);unloads++;await wait(s=>s.debug.replicated_world.terrain.work.find(w=>w.unit===actor)?.wait!=='FullCargo','Unload did not resume work');}else await sleep(200);}
  assert(done(s),'Work rectangle did not reach the captured reference level');
  assert(!scenario.require_unload||unloads>0,'Physical fixture never exercised FullCargo/unload');result.unloads=unloads;await capture('02-passage-open');
  const rover=await select(scenario.rover);s=await read();await click(project(s.scene,scenario.destination),'right');await wait(s=>{const u=unit(s,scenario.rover);return u&&Math.hypot(u.pos[0]-scenario.destination[0],u.pos[1]-scenario.destination[1])<1;},'Wide rover did not traverse the opened passage',120000);await capture('03-rover-through');
  assert(result.errors.length===0,'Browser errors');result.status='PASS';result.rover=rover;
 }catch(error){result.status='FAIL';result.error=String(error.stack||error);process.exitCode=1;console.error(error);}
 finally{if(context)await context.close();if(profile)fs.rmSync(profile,{recursive:true,force:true});if(xvfb&&xvfb.exitCode===null){const stopped=new Promise(resolve=>xvfb.once('exit',resolve));xvfb.kill('SIGTERM');await Promise.race([stopped,sleep(2000)]);if(xvfb.exitCode===null&&xvfb.signalCode===null){xvfb.kill('SIGKILL');await stopped;}}result.profile_removed=!profile||!fs.existsSync(profile);result.xvfb_stopped=!xvfb||xvfb.exitCode!==null||xvfb.signalCode!==null;fs.writeFileSync(path.join(output,'result.json'),JSON.stringify(result,null,2));}
})();
