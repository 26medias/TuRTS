// DM12: play the shipped package against its computer through real browser input.
// No map, stats, economy, or wire commands are changed.
const fs=require('node:fs'),net=require('node:net'),{spawn}=require('node:child_process'),{chromium}=require('playwright-core');
const [prepared,out]=process.argv.slice(2),mode='win';if(!out)throw Error('PREPARED OUTPUT');fs.mkdirSync(out,{recursive:true});
const freePort=()=>new Promise(resolve=>{const server=net.createServer();server.listen(0,'127.0.0.1',()=>{const port=server.address().port;server.close(()=>resolve(port))})});
const sleep=ms=>new Promise(r=>setTimeout(r,ms));
function project(scene,v){const m=scene.clip_from_world,c=[0,1,2,3].map(r=>[...v,1].reduce((sum,x,k)=>sum+m[k*4+r]*x,0)),[x,y,w,h]=scene.viewport_logical_rect;return[x+(c[0]/c[3]+1)*w/2,y+(1-c[1]/c[3])*h/2]}
(async()=>{let child,browser,page,read;const started=Date.now(),result={status:'RUNNING',mode,actions:[],samples:[],errors:[]};
const saveStatus=label=>{fs.writeFileSync(out+'/status.tmp',JSON.stringify({label,seconds:(Date.now()-started)/1000,last:result.samples.at(-1)},null,2));fs.renameSync(out+'/status.tmp',out+'/status.json')};
try{
const http=await freePort(),ws=await freePort(),url=`http://127.0.0.1:${http}`;
fs.writeFileSync(out+'/server.ron',`(game_package:"${prepared}/pack/skirmish.tar",http_listen:"127.0.0.1:${http}",ws_listen:"127.0.0.1:${ws}",public_url:"${url}",tick_hz:20,static_dir:"${prepared}/dist")`);
const fd=fs.openSync(out+'/server.log','w');child=spawn(prepared+'/bin/turts_server',[out+'/server.ron','--computer-slot','1'],{stdio:['ignore',fd,fd],env:{...process.env,TURTS_COMPUTER_SLOT:'1'}});fs.closeSync(fd);
for(let i=0;i<300;i++){try{if((await fetch(url+'/advertisement.json')).ok)break}catch{}if(child.exitCode!==null)throw Error('server stopped');await sleep(50)}
result.profile=fs.mkdtempSync('/tmp/turts-damage-win-');
browser=await chromium.launchPersistentContext(result.profile,{executablePath:'/usr/bin/google-chrome',headless:false,viewport:{width:1280,height:720},ignoreDefaultArgs:['--enable-unsafe-swiftshader'],args:['--enable-gpu','--alsa-output-device=null','--enable-unsafe-webgpu','--enable-features=Vulkan','--use-angle=vulkan','--disable-vulkan-surface','--no-first-run','--no-default-browser-check']});
page=browser.pages()[0];page.on('pageerror',e=>result.errors.push(String(e)));await page.goto(url+'/?server='+encodeURIComponent(url)+'&ws='+encodeURIComponent(`ws://127.0.0.1:${ws}`));
read=async()=>{try{return await page.evaluate(async()=>{if(typeof turts_scene!=='function'||!turts_scene()||typeof turts_debug_request!=='function')return null;const d=JSON.parse(await turts_debug_request()),w=d.replicated_world;return{scene:JSON.parse(turts_scene()),ui:JSON.parse(turts_ui()),world:w,slot:d.slot,selection:d.selection,match:d.match_id_decimal,notices:d.notices,results:d.command_results}});}catch(e){if(/Execution context was destroyed|Cannot find context/.test(String(e)))return null;throw e}};
const outcome=s=>s.ui.find(w=>w.id==='result_title'&&w.visible&&['Victory','Defeat','Draw'].includes(w.text))?.text;
const hp=(s,id)=>s.world.healths.find(h=>h.unit===id)?.current;
const own=s=>s.world.units.filter(u=>u.owner_slot===s.slot);
const combatants=(s,owner)=>s.world.units.filter(u=>u.owner_slot===owner&&['skirmish:soldier','skirmish:swordsman','skirmish:warrior'].includes(u.content_id));
let lastRecordedTick=-1;
const record=(s,label)=>{if(!s?.world?.units)return;if(!result.enemy_content&&combatants(s,1).some(u=>u.content_id==='skirmish:warrior')&&s.world.units.some(u=>u.owner_slot===1&&u.content_id==='skirmish:tower'&&hp(s,u.unit_id)===400))result.enemy_content=s;if(s.world.tick===lastRecordedTick)return;lastRecordedTick=s.world.tick;result.samples.push({label,tick:s.world.tick,seconds:(Date.now()-started)/1000,own_castle:hp(s,1),enemy_castle:hp(s,4),own_combatants:combatants(s,0).length,enemy_combatants:combatants(s,1).length,credits:s.world.economy.balances.find(b=>b.resource==='skirmish:credits')?.on_hand,outcome:outcome(s)});saveStatus(label)};
const wait=async(fn,label,ms=30000,allowTerminal=false)=>{const end=Date.now()+ms;while(Date.now()<end){const s=await read();if(s?.world?.units){record(s,label);if(fn(s))return s;if(!allowTerminal&&outcome(s))throw Error('match ended during '+label+': '+outcome(s))}if(result.errors.length)throw Error(result.errors.join('\n'));await sleep(150)}throw Error(label+' timeout')};
let state=await wait(s=>s.scene.models?.length>0&&s.scene.models.every(m=>m.ready)&&s.scene.widgets.some(w=>w.id==='skirmish_minimap')&&s.world.units.length>=6,'ready',90000);
if(state.slot!==0||hp(state,1)!==1000||hp(state,4)!==1000||state.world.units.find(u=>u.unit_id===1)?.max_health!==1000||state.world.units.find(u=>u.unit_id===4)?.max_health!==1000)throw Error('not the normal two-castle package');
result.initial=state;await page.screenshot({path:out+'/initial.png'});const viewport=state.scene.viewport_logical_rect;await page.mouse.click(viewport[0]+viewport[2]/2,viewport[1]+viewport[3]*.65);await page.mouse.move(viewport[2]/2,viewport[3]-40);
const clickPoint=async(point,label,button='left')=>{if(!point.every(Number.isFinite))throw Error('invalid point '+label);result.actions.push({label,point,button,seconds:(Date.now()-started)/1000});await page.mouse.click(...point,{button});await sleep(80)};
const clickUnit=async(id,button='left')=>{const s=await read(),u=s.scene.units.find(u=>u.unit_id===id);if(!u?.ground_screen_center){if(button==='right')return false;throw Error('rendered unit absent '+id)}const [x,y,w,h]=s.scene.viewport_logical_rect,p=u.ground_screen_center;
if(p[0]<x+4||p[0]>x+w-4||p[1]<y+4||p[1]>y+h-4){if(button==='right'){const rect=s.scene.widgets.find(w=>w.id==='skirmish_minimap').rect;await clickPoint([rect[0]+u.translation[0]/64*rect[2],rect[1]+(1-u.translation[2]/48)*rect[3]],'follow battle on map');await sleep(150);return false}throw Error('unit offscreen '+id)}await clickPoint(project(s.scene,u.translation),'unit '+id,button);return true};
const clickWorld=async(x,y,button='left')=>{const s=await read();await clickPoint(project(s.scene,[x,0,y]),`world ${x},${y}`,button)};
const clickButton=async(text,ms=30000)=>{const s=await wait(s=>s.ui.some(w=>w.kind==='button'&&w.visible&&w.enabled&&w.text?.includes(text)),'button '+text,ms),w=s.ui.find(w=>w.kind==='button'&&w.visible&&w.enabled&&w.text?.includes(text));await clickPoint([w.rect[0]+w.rect[2]/2,w.rect[1]+w.rect[3]/2],text);return s};
if(mode==='win'){
await clickUnit(2);state=await read();const tree=state.scene.nodes.find(n=>n.node_id===1);await clickPoint(tree.ground_screen_center,'harvest tree 1','right');
await wait(s=>s.world.economy.orders.some(o=>o.unit===2&&o.stage!=='Idle'),'harvest order');
await clickUnit(1);await clickButton('Train Villager');await clickButton('Train Villager');
state=await wait(s=>own(s).filter(u=>u.content_id==='skirmish:worker').length===3,'three Villagers',15000);
for(const worker of own(state).filter(u=>u.content_id==='skirmish:worker')){await clickUnit(worker.unit_id);const current=await read();await clickPoint(current.scene.nodes.find(n=>n.node_id===1).ground_screen_center,'all Villagers harvest timber','right')}
await clickUnit(1);await clickButton('Build Barracks');await clickWorld(17,24);
state=await wait(s=>own(s).some(u=>u.content_id==='skirmish:barracks'&&hp(s,u.unit_id)===500),'completed Barracks',20000);const barracks=own(state).find(u=>u.content_id==='skirmish:barracks').unit_id;
await clickUnit(barracks);await clickWorld(22,26,'right');await clickUnit(3);await clickWorld(22,26,'right');await clickUnit(barracks);
for(const name of Array(8).fill('Swordsman')){await clickButton('Train '+name,60000);await sleep(350)}
state=await wait(s=>combatants(s,0).length>=6,'funded surviving army ready',30000);await sleep(1800);state=await read();result.army=state;await page.screenshot({path:out+'/mixed-army.png'});
const swords=combatants(state,0).filter(u=>u.content_id==='skirmish:swordsman');if(swords.length<5||swords.length>8||swords.some(u=>u.max_health!==300))throw Error('normal swordsman stats or count changed');
const credits=state.world.economy.balances.find(b=>b.resource==='skirmish:credits').on_hand;result.harvest_credits=credits+800-200;if(result.harvest_credits<600)throw Error('army was not funded by real timber deliveries');
result.enemy_content=result.enemy_content||await wait(s=>combatants(s,1).some(u=>u.content_id==='skirmish:warrior')&&s.world.units.some(u=>u.owner_slot===1&&u.content_id==='skirmish:tower'&&hp(s,u.unit_id)===400),'computer Warrior and completed tower',30000);
const army=combatants(state,0).map(u=>u.unit_id),points=state.scene.units.filter(u=>army.includes(u.unit_id)).map(u=>u.ground_screen_center);
const box=[Math.min(...points.map(p=>p[0]))-20,Math.min(...points.map(p=>p[1]))-20,Math.max(...points.map(p=>p[0]))+20,Math.max(...points.map(p=>p[1]))+20];
result.actions.push({label:'box-select mixed army',box,seconds:(Date.now()-started)/1000});await page.mouse.move(box[0],box[1]);await page.mouse.down();await page.mouse.move(box[2],box[3],{steps:12});await sleep(100);await page.mouse.up();await sleep(150);
state=await read();result.selected_army=state;await page.screenshot({path:out+'/selected-army.png'});if(JSON.stringify([...state.selection].sort((a,b)=>a-b))!==JSON.stringify([...army].sort((a,b)=>a-b)))throw Error('mixed selection failed: '+JSON.stringify(state.selection)+' expected '+JSON.stringify(army));
const rect=state.scene.widgets.find(w=>w.id==='skirmish_minimap').rect;await clickPoint([rect[0]+54/64*rect[2],rect[1]+.5*rect[3]],'look at enemy castle');await sleep(250);result.enemy_before_fight=await read();await page.screenshot({path:out+'/enemy-warrior-and-tower.png'});
let target=null,lastOrder=0;const deadline=Date.now()+180000;
while(Date.now()<deadline){state=await read();record(state,'fight');if(outcome(state))break;const fighters=combatants(state,0);if(!fighters.length)throw Error('player army was defeated');
const enemies=combatants(state,1).filter(u=>hp(state,u.unit_id)>0),center=fighters.reduce((p,u)=>[p[0]+u.pos[0]/fighters.length,p[1]+u.pos[1]/fighters.length],[0,0]);
const selected=enemies.sort((a,b)=>(hp(state,a.unit_id)-hp(state,b.unit_id))||(Math.hypot(a.pos[0]-center[0],a.pos[1]-center[1])-Math.hypot(b.pos[0]-center[0],b.pos[1]-center[1])))[0]||state.world.units.find(u=>u.owner_slot===1&&u.content_id==='skirmish:tower'&&hp(state,u.unit_id)>0)||state.world.units.find(u=>u.unit_id===4);
if(!selected)break;if(target!==selected.unit_id||Date.now()-lastOrder>3000){target=selected.unit_id;if(!await clickUnit(target,'right')){target=null;continue}lastOrder=Date.now();await page.mouse.move(viewport[2]/2,viewport[3]-40)}await sleep(400)}
}else{
const deadline=Date.now()+420000;
while(Date.now()<deadline){state=await read();record(state,'idle defense');if(outcome(state))break;await sleep(1000)}
}
state=await read();result.terminal=state;const expected=mode==='win'?'Victory':'Defeat';if(outcome(state)!==expected)throw Error('expected '+expected+', got '+outcome(state));await sleep(500);await page.screenshot({path:out+'/terminal.png'});
result.terminal_seconds=(Date.now()-started)/1000;if(!state.ui.some(w=>w.id==='result_title'&&w.visible&&w.text===expected))throw Error('terminal result not visible');
result.status='PASS';console.log(JSON.stringify({status:result.status,mode,terminal_seconds:result.terminal_seconds,harvest_credits:result.harvest_credits}));
}catch(e){result.status='FAIL';result.error=String(e.stack||e);console.error(e);if(read)try{result.failure=await read()}catch{}if(page)try{await page.screenshot({path:out+'/failure.png'})}catch{}process.exitCode=1}finally{if(browser)await browser.close();if(child&&child.exitCode===null){const ended=new Promise(r=>child.once('exit',r));try{child.kill('SIGTERM')}catch{}await Promise.race([ended,sleep(8000)])}if(result.profile)fs.rmSync(result.profile,{recursive:true,force:true});result.profile_removed=result.profile?!fs.existsSync(result.profile):null;if(result.profile_removed===false){result.status='FAIL';result.error='fresh profile was not removed';process.exitCode=1}fs.writeFileSync(out+'/result.json',JSON.stringify(result,null,2));saveStatus(result.status)}})();
