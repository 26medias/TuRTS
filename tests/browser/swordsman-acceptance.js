// Train and rally a swordsman through the HUD. Fight with a mixed army, then observe its death.
const fs=require('node:fs'),net=require('node:net'),{spawn,spawnSync}=require('node:child_process');
const {chromium}=require('playwright-core');
const [prepared,out]=process.argv.slice(2);if(!out)throw Error('PREPARED OUTPUT');
fs.mkdirSync(out,{recursive:true});const fixture=out+'/fixture';fs.cpSync(prepared+'/pack/skirmish',fixture,{recursive:true});
const place=(kind,slot,x,y)=>`(unit:"skirmish:${kind}",owner_slot:${slot},pos:(x:${x}.0,y:${y}.0),contents:[])`;
fs.writeFileSync(fixture+'/maps/field.ron',`(id:"field",size:(x:48.0,y:32.0),placements:[${[place('base',0,8,8),place('barracks',0,14,12),place('worker',0,10,12),place('soldier',0,14,20),place('base',1,32,20),place('soldier',1,30,24)].join(',')}],nodes:[],slot_economy:[(slot:0,balances:[(balance:"skirmish:credits",amount:1000)])])`);
fs.writeFileSync(fixture+'/match.ron','(start:Immediate,victory:None,open_sites_per_slot:Some(4))');
// Let the mixed army keep fighting long enough to observe both attacks and a melee death.
let p=fixture+'/units/base.ron';fs.writeFileSync(p,fs.readFileSync(p,'utf8').replace('max: 1000','max: 5000'));
p=fixture+'/units/soldier.ron';fs.writeFileSync(p,fs.readFileSync(p,'utf8').replace('damage: 15','damage: 25'));
const sleep=ms=>new Promise(r=>setTimeout(r,ms)),port=()=>new Promise(r=>{const s=net.createServer();s.listen(0,'127.0.0.1',()=>{const n=s.address().port;s.close(()=>r(n))})});
function project(s,v){const m=s.clip_from_world,c=[0,1,2,3].map(r=>[...v,1].reduce((a,x,k)=>a+m[k*4+r]*x,0));return[(c[0]/c[3]+1)*640,(1-c[1]/c[3])*360]}
(async()=>{let server,context;const profile=fs.mkdtempSync('/tmp/turts-swordsman-');const result={status:'RUNNING',errors:[],samples:[]};try{
const pack=spawnSync(prepared+'/bin/turts-pack',[fixture],{encoding:'utf8'});fs.writeFileSync(out+'/pack.log',pack.stdout+pack.stderr);if(pack.status!==0)throw Error('pack failed '+pack.stderr);
const http=await port(),ws=await port(),url=`http://127.0.0.1:${http}`;fs.writeFileSync(out+'/server.ron',`(game_package:"${fixture}.tar",http_listen:"127.0.0.1:${http}",ws_listen:"127.0.0.1:${ws}",public_url:"${url}",tick_hz:20,static_dir:"${prepared}/dist")`);
const fd=fs.openSync(out+'/server.log','w');server=spawn(prepared+'/bin/turts_server',[out+'/server.ron'],{stdio:['ignore',fd,fd],env:{...process.env,TURTS_COMPUTER_SLOT:''}});fs.closeSync(fd);
for(let i=0;i<200;i++){try{if((await fetch(url+'/advertisement.json')).ok)break}catch{}if(server.exitCode!==null)throw Error('server stopped');await sleep(50)}
context=await chromium.launchPersistentContext(profile,{executablePath:'/usr/bin/google-chrome',headless:false,viewport:{width:1280,height:720},ignoreDefaultArgs:['--enable-unsafe-swiftshader'],args:['--enable-gpu','--alsa-output-device=null','--enable-unsafe-webgpu','--enable-features=Vulkan','--use-angle=vulkan','--disable-vulkan-surface','--no-first-run','--no-default-browser-check']});
const page=context.pages()[0];page.on('pageerror',e=>result.errors.push(String(e)));
await page.goto(url+'/?server='+encodeURIComponent(url)+'&ws='+encodeURIComponent(`ws://127.0.0.1:${ws}`));
const read=()=>page.evaluate(()=>typeof turts_scene==='function'&&turts_scene()?JSON.parse(turts_scene()):null);
const wait=async(fn,label,ms=60000)=>{const end=Date.now()+ms;while(Date.now()<end){const s=await read();if(s&&fn(s))return s;if(result.errors.length)throw Error(result.errors.join('\n'));await sleep(40)}throw Error(label+' timeout')};
const ui=()=>page.evaluate(()=>JSON.parse(turts_ui()));
const clickUnit=async(id,button='left')=>{const s=await read(),u=s.units.find(u=>u.unit_id===id);if(!u)throw Error('missing unit '+id);await page.mouse.click(...project(s,[u.translation[0],id===2?1:.75,u.translation[2]]),{button})};
let scene=await wait(s=>s.units?.length===6&&s.models?.every(m=>m.ready),'models ready');await page.mouse.click(640,360);
// Keep training, rally and combat in view.
await clickUnit(2);await page.waitForFunction(()=>JSON.parse(turts_ui()).some(w=>w.kind==='button'&&w.visible&&w.text?.includes('Train Swordsman')),null,{timeout:5000});
result.buildMenu=await ui();for(const label of ['Train Crossbowman','Train Swordsman'])if(!result.buildMenu.some(w=>w.visible&&w.text?.includes(label)))throw Error('missing '+label);
const train=result.buildMenu.find(w=>w.kind==='button'&&w.visible&&w.text?.includes('Train Swordsman'));
if(!train.text.includes('80'))throw Error('missing sword cost');
scene=await read();await page.mouse.click(...project(scene,[22,0,20]),{button:'right'});
const started=Date.now();await page.mouse.click(train.rect[0]+train.rect[2]/2,train.rect[1]+train.rect[3]/2);
await sleep(200);result.queue=await ui();if(!result.queue.some(w=>w.visible&&w.text?.includes('Swordsman')))throw Error('no production queue');
if(!result.queue.some(w=>w.id==='credits'&&w.text?.includes('920')))throw Error('training did not charge exactly 80');
await sleep(4500);scene=await read();if(scene.units.some(u=>u.unit_id===7))throw Error('swordsman trained before 6 seconds');
scene=await wait(s=>s.units.some(u=>u.unit_id===7)&&s.models.some(m=>m.unit_id===7&&m.ready),'trained sword',10000);result.train_ms=Date.now()-started;
result.walk=await wait(s=>s.animations.some(a=>a.unit_id===7&&a.clip==='Walking_A'),'rally walking',5000);
await page.screenshot({path:out+'/rally-walk.png'});
scene=await wait(s=>{const u=s.units.find(u=>u.unit_id===7);return u&&Math.hypot(u.translation[0]-22,u.translation[2]-20)<1},'rally arrival',10000);result.rally=scene;
await clickUnit(7);await clickUnit(5,'right');await clickUnit(4);await clickUnit(5,'right');await page.mouse.move(400,680);
const end=Date.now()+25000;let swordAttack=false,bowAttack=false,hit=false,death=false,body=false;
while(Date.now()<end){scene=await read();result.samples.push(scene);swordAttack ||= scene.animations.some(a=>a.unit_id===7&&a.clip==='1H_Melee_Attack_Chop'&&!a.dead);bowAttack ||= scene.animations.some(a=>a.unit_id===4&&a.clip==='2H_Ranged_Shoot');hit ||= scene.effects.combat.flashes.includes(7);death ||= scene.animations.some(a=>a.dead&&a.clip==='Death_A'&& !scene.units.some(u=>u.unit_id===7));body ||= scene.body_count>0;
if(swordAttack&&!result.attack){result.attack=scene;await page.screenshot({path:out+'/mixed-attack.png'})}
if(death&&body){result.death=scene;await page.screenshot({path:out+'/swordsman-death.png'});break}await sleep(50)}
Object.assign(result,{swordAttack,bowAttack,hit,death,body});if(!swordAttack||!bowAttack||!hit||!death||!body)throw Error('missing combat observation');
const sword=result.attack.units.find(u=>u.unit_id===7),castle=result.attack.units.find(u=>u.unit_id===5);result.melee_distance=Math.hypot(sword.translation[0]-castle.translation[0],sword.translation[2]-castle.translation[2]);if(result.melee_distance<2.6-1e-5||result.melee_distance>3.201)throw Error('wrong sword range');
result.status='PASS';console.log(JSON.stringify({status:result.status,train_ms:result.train_ms,melee_distance:result.melee_distance}));
}catch(e){result.status='FAIL';result.error=String(e.stack||e);console.error(e);process.exitCode=1}finally{fs.writeFileSync(out+'/result.json',JSON.stringify(result,null,2));if(context)await context.close();if(server&&server.exitCode===null){server.kill('SIGTERM');await new Promise(r=>server.once('exit',r))}fs.rmSync(profile,{recursive:true,force:true})}})();
