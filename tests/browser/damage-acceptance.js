// Exercise the content panel with real input and screenshot pixels.
const fs = require('node:fs'), net = require('node:net');
const {spawn, spawnSync} = require('node:child_process');
const {chromium} = require('playwright-core');
const {decodePng} = require('./harness/png');
const [prepared, out] = process.argv.slice(2);
if (!out) throw Error('PREPARED OUTPUT');
fs.mkdirSync(out, {recursive:true});
const fixture = out + '/fixture';
fs.cpSync(prepared + '/pack/skirmish', fixture, {recursive:true});
const place=(kind,owner,x,y)=>`(unit:"skirmish:${kind}",owner_slot:${owner},pos:(x:${x}.0,y:${y}.0),contents:[])`;
fs.writeFileSync(fixture+'/maps/field.ron',`(id:"field",size:(x:64.0,y:48.0),placements:[${[place('base',0,10,24),place('barracks',0,16,24),place('worker',0,14,18),place('soldier',0,14,30),place('base',1,54,24),place('warrior',1,42,24)].join(',')}],nodes:[],slot_economy:[(slot:0,balances:[(balance:"skirmish:credits",amount:1000)])])`);
fs.writeFileSync(fixture+'/match.ron','(start:Immediate,victory:None,open_sites_per_slot:Some(4))');
const sleep = ms=>new Promise(r=>setTimeout(r,ms));
const port = ()=>new Promise(r=>{const s=net.createServer();s.listen(0,'127.0.0.1',()=>{const p=s.address().port;s.close(()=>r(p))})});
function project(s, v) {
 const m=s.clip_from_world, c=[0,1,2,3].map(r=>[...v,1].reduce((a,x,k)=>a+m[k*4+r]*x,0));
 const [x,y,w,h]=s.viewport_logical_rect;
 return [x+(c[0]/c[3]+1)*w/2,y+(1-c[1]/c[3])*h/2];
}
(async()=>{
 let server, context, page, read;
 const profile=fs.mkdtempSync('/tmp/turts-damage-');
 const result={status:'RUNNING',errors:[],profile};
 try {
  const packed=spawnSync(prepared+'/bin/turts-pack',[fixture],{encoding:'utf8'});
  fs.writeFileSync(out+'/pack.log',packed.stdout+packed.stderr);
  if(packed.status!==0) throw Error('fixture pack failed');
  const http=await port(), ws=await port(), url=`http://127.0.0.1:${http}`;
  fs.writeFileSync(out+'/server.ron',`(game_package:"${fixture}.tar",http_listen:"127.0.0.1:${http}",ws_listen:"127.0.0.1:${ws}",public_url:"${url}",tick_hz:20,static_dir:"${prepared}/dist")`);
  const fd=fs.openSync(out+'/server.log','w');
  server=spawn(prepared+'/bin/turts_server',[out+'/server.ron'],{stdio:['ignore',fd,fd],env:{...process.env,TURTS_COMPUTER_SLOT:''}});fs.closeSync(fd);
  for(let i=0;i<300;i++){try{if((await fetch(url+'/advertisement.json')).ok)break}catch{}if(server.exitCode!==null)throw Error('server stopped');await sleep(50)}
  context=await chromium.launchPersistentContext(profile,{executablePath:'/usr/bin/google-chrome',headless:false,viewport:{width:1280,height:720},ignoreDefaultArgs:['--enable-unsafe-swiftshader'],args:['--enable-gpu','--alsa-output-device=null','--enable-unsafe-webgpu','--enable-features=Vulkan','--use-angle=vulkan','--disable-vulkan-surface','--no-first-run','--no-default-browser-check']});
  page=context.pages()[0];page.on('pageerror',e=>result.errors.push(String(e)));
  await page.goto(url+'/?server='+encodeURIComponent(url)+'&ws='+encodeURIComponent(`ws://127.0.0.1:${ws}`));
  read=()=>page.evaluate(async()=>{if(typeof turts_scene!=='function'||!turts_scene())return null;const debug=JSON.parse(await turts_debug_request());return{debug,selection:debug.selection,scene:JSON.parse(turts_scene()),ui:JSON.parse(turts_ui())}});
  const wait=async(fn,label,ms=60000)=>{const end=Date.now()+ms;while(Date.now()<end){const s=await read();if(s&&fn(s))return s;if(result.errors.length)throw Error(result.errors.join('\n'));await sleep(80)}throw Error(label+' timeout')};
  const widget=(s,id)=>s.ui.find(w=>w.id===id&&w.visible);
  const clickUnit=async(id,button='left')=>{const s=await read(),unit=s.scene.units.find(u=>u.unit_id===id);await page.mouse.click(...project(s.scene,unit.translation),{button});};
  await wait(s=>s.scene.models?.length===6&&s.scene.models.every(m=>m.ready),'models');
  await page.mouse.click(640,360);
  const units=s=>s.debug.replicated_world.units;
  const health=(s,id)=>s.debug.replicated_world.healths.find(row=>row.unit===id)?.current;
  const weapon=(s,id)=>s.debug.replicated_world.weapons.find(row=>row.unit===id);
  const button=async(label)=>{const s=await wait(s=>s.ui.some(w=>w.kind==='button'&&w.visible&&w.enabled&&w.text?.includes(label)),label);const w=s.ui.find(w=>w.kind==='button'&&w.visible&&w.enabled&&w.text?.includes(label));await page.mouse.click(w.rect[0]+w.rect[2]/2,w.rect[1]+w.rect[3]/2);};
  await clickUnit(2);await wait(s=>s.selection.includes(2),'Barracks selection');
  const trainStart=Date.now();await button('Train Warrior');
  result.queued=await wait(s=>s.ui.some(w=>w.visible&&w.id.startsWith('m2_job_line')&&w.text?.includes('Warrior')),'Warrior queue');
  await clickUnit(1);await wait(s=>s.selection.includes(1),'castle selection');
  await button('Build Guard tower');await sleep(200);
  const towerStart=Date.now();await page.mouse.click(...project((await read()).scene,[24,0,24]));
  let state=await wait(s=>units(s).some(unit=>unit.owner_slot===0&&unit.content_id==='skirmish:tower'),'placed tower');
  const tower=units(state).find(unit=>unit.owner_slot===0&&unit.content_id==='skirmish:tower').unit_id;
  result.site=state;
  if(weapon(state,tower))throw Error('unfinished tower already has an active weapon');
  state=await wait(s=>units(s).some(unit=>unit.owner_slot===0&&unit.content_id==='skirmish:warrior')&&s.scene.models.every(model=>model.ready),'trained Warrior');
  result.train_ms=Date.now()-trainStart;if(result.train_ms<5900)throw Error('Warrior trained before six seconds');
  const warrior=units(state).find(unit=>unit.owner_slot===0&&unit.content_id==='skirmish:warrior').unit_id;
  result.ids={warrior,tower,enemy_warrior:6};
  await clickUnit(warrior);
  result.armour=await wait(s=>s.selection.includes(warrior)&&widget(s,'selection_armour')?.text==='Armour: 2','own armour 2');
  await page.screenshot({path:out+'/warrior-armour.png'});
  const name=widget(result.armour,'selection_name');await page.mouse.move(name.rect[0]+name.rect[2]/2,name.rect[1]+name.rect[3]/2);
  result.tooltip=await wait(s=>s.ui.some(w=>w.visible&&w.text?.includes('Damage: 20 blade')),'weapon tooltip',5000);
  await page.screenshot({path:out+'/weapon-tooltip.png'});await page.mouse.move(500,680);
  await clickUnit(6);state=await wait(s=>s.selection.includes(6),'enemy selection');
  if(widget(state,'selection_armour'))throw Error('enemy armour leaked without its research');
  result.enemy_armour_absent=true;
  state=await wait(s=>weapon(s,tower)&&s.scene.models.every(model=>model.ready),'completed tower');
  result.build_ms=Date.now()-towerStart;if(result.build_ms<14900)throw Error('tower completed before fifteen seconds');
  result.complete=state;if(health(state,tower)!==400)throw Error('completed tower health');
  await page.screenshot({path:out+'/completed-tower.png'});
  await clickUnit(warrior);await clickUnit(6,'right');
  result.walk=await wait(s=>s.scene.animations.some(a=>a.unit_id===warrior&&a.clip==='Walking_A'&&a.distance_loops>0),'Warrior walk');
  await page.screenshot({path:out+'/warrior-walk.png'});
  result.attack=await wait(s=>health(s,6)<160&&s.scene.animations.some(a=>a.unit_id===warrior&&a.clip==='1H_Melee_Attack_Chop'),'Warrior melee',20000);
  await page.screenshot({path:out+'/warrior-melee.png'});
  // Draw the automatic enemy chase into the player's completed tower range.
  await page.mouse.click(...project((await read()).scene,[26,0,24]),{button:'right'});
  result.tower_shot=await wait(s=>weapon(s,tower)?.last_target===6&&health(s,6)<160,'tower shot',20000);
  await page.screenshot({path:out+'/tower-shot.png'});
  result.status='PASS';console.log(JSON.stringify({status:'PASS',train_ms:result.train_ms,build_ms:result.build_ms,ids:result.ids}));
 } catch(e) {result.status='FAIL';result.error=String(e.stack||e);if(read)try{result.failure=await read()}catch{}if(page)try{await page.screenshot({path:out+'/failure.png'})}catch{}console.error(e);process.exitCode=1;}
 finally {if(context)await context.close();if(server&&server.exitCode===null){const ended=new Promise(r=>server.once('exit',r));server.kill('SIGTERM');await ended}fs.rmSync(profile,{recursive:true,force:true});result.profile_removed=!fs.existsSync(profile);fs.writeFileSync(out+'/result.json',JSON.stringify(result,null,2))}
})();
