// Real rendered combat and warning probes. Every run owns a fresh silent browser.
const fs=require('node:fs'),net=require('node:net'),{spawn,spawnSync}=require('node:child_process');
const {chromium}=require('playwright-core');
const {decodePng,pixelAt}=require('./harness/png');
const [prepared,out,mode='pixels']=process.argv.slice(2);if(!out)throw Error('PREPARED OUTPUT [pixels|warnings|victory]');
fs.mkdirSync(out,{recursive:true});const fixture=out+'/fixture';fs.cpSync(prepared+'/pack/skirmish',fixture,{recursive:true});
const placement=(unit,slot,x,y)=>`(unit:"skirmish:${unit}",owner_slot:${slot},pos:(x:${x}.0,y:${y}.0),contents:[])`;
const placements=mode==='warnings'?[placement('base',0,10,20),placement('worker',0,15,25),placement('soldier',0,12,38),placement('soldier',1,15,20)]:mode==='victory'?[placement('base',0,10,12),placement('worker',0,12,9),placement('soldier',0,20,20),placement('base',1,30,20)]:[placement('base',0,10,15),placement('worker',0,13,8),placement('soldier',0,16,20),placement('soldier',1,24,20)];
fs.writeFileSync(fixture+'/maps/field.ron',`(id:"field",size:(x:${mode==='warnings'?100:40}.0,y:${mode==='warnings'?60:30}.0),placements:[${placements.join(',')}],nodes:[],slot_economy:[(slot:0,balances:[(balance:"skirmish:credits",amount:10000)])])`);
if(mode==='warnings'){const p=fixture+'/interface/hud.ron';fs.writeFileSync(p,fs.readFileSync(p,'utf8').replace('pan_speed: 1.0','pan_speed: 100.0'));}
if(mode!=='victory')fs.writeFileSync(fixture+'/match.ron','(start:Immediate,victory:None,open_sites_per_slot:Some(4))');
if(mode==='victory'){const p=fixture+'/units/base.ron';fs.writeFileSync(p,fs.readFileSync(p,'utf8').replace('max: 1000','max: 30'));}
else if(mode==='pixels'){const p=fixture+'/units/soldier.ron';fs.writeFileSync(p,fs.readFileSync(p,'utf8').replace('max: 120','max: 10000'));}
if(process.env.TURTS_EFFECT_NEGATIVE==='1'){const p=fixture+'/visuals/soldier.ron',s=fs.readFileSync(p,'utf8'),i=s.indexOf(',effects:');if(i<0)throw Error('No effects suffix');fs.writeFileSync(p,s.slice(0,i)+')');}
const sleep=ms=>new Promise(r=>setTimeout(r,ms)),port=()=>new Promise(r=>{const s=net.createServer();s.listen(0,'127.0.0.1',()=>{const p=s.address().port;s.close(()=>r(p))})});
function project(s,v){const m=s.clip_from_world,c=[0,1,2,3].map(r=>[...v,1].reduce((a,x,k)=>a+m[k*4+r]*x,0));return[(c[0]/c[3]+1)*640,(1-c[1]/c[3])*360]}
const onscreen=p=>p[0]>10&&p[0]<1270&&p[1]>10&&p[1]<710;
(async()=>{let server,context;const profile=fs.mkdtempSync('/tmp/turts-fx-accept-');const result={status:'RUNNING',mode,errors:[],frames:[]};try{
if(mode==='hash')fs.writeFileSync(out+'/commands.json',JSON.stringify([{tick:80,slot:0,envelope:{order:'skirmish:order_attack',units:[3],target:{Unit:4},queue:'Replace',group:0}}]));
const pack=spawnSync(prepared+'/bin/turts-pack',[fixture],{encoding:'utf8'});fs.writeFileSync(out+'/pack.log',pack.stdout+pack.stderr);if(pack.status!==0)throw Error('pack failed '+pack.stderr);
const http=await port(),ws=await port(),url=`http://127.0.0.1:${http}`;fs.writeFileSync(out+'/server.ron',`(game_package:"${fixture}.tar",http_listen:"127.0.0.1:${http}",ws_listen:"127.0.0.1:${ws}",public_url:"${url}",tick_hz:20,static_dir:"${prepared}/dist")`);
const fd=fs.openSync(out+'/server.log','w');server=spawn(prepared+'/bin/turts_server',[out+'/server.ron',...(mode==='hash'?['--economy-log',out+'/audit.jsonl','--run-ticks','260','--command-script',out+'/commands.json']:[])],{stdio:['ignore',fd,fd],env:{...process.env,TURTS_COMPUTER_SLOT:''}});fs.closeSync(fd);
for(let i=0;i<200;i++){try{if((await fetch(url+'/advertisement.json')).ok)break}catch{}await sleep(50)}
context=await chromium.launchPersistentContext(profile,{executablePath:'/usr/bin/google-chrome',headless:false,viewport:{width:1280,height:720},ignoreDefaultArgs:['--enable-unsafe-swiftshader'],args:['--enable-gpu','--alsa-output-device=null','--enable-unsafe-webgpu','--enable-features=Vulkan','--use-angle=vulkan','--disable-vulkan-surface','--no-first-run','--no-default-browser-check']});
await context.addInitScript(q=>localStorage.setItem('turts.v1.settings',JSON.stringify({controls:{quality:q}})),process.env.TURTS_EFFECT_QUALITY||'high');
const page=context.pages()[0];page.on('pageerror',e=>result.errors.push(String(e)));page.on('console',m=>{if(m.type()==='error')result.errors.push(m.text())});
await page.goto(url+'/?server='+encodeURIComponent(url)+'&ws='+encodeURIComponent(`ws://127.0.0.1:${ws}`));const read=()=>page.evaluate(()=>typeof turts_scene==='function'&&turts_scene()?JSON.parse(turts_scene()):null);
const wait=async(fn,label,ms=60000)=>{const end=Date.now()+ms;while(Date.now()<end){const s=await read();if(s&&fn(s))return s;if(result.errors.some(e=>/panicked|shader|validation error/i.test(e)))throw Error(result.errors.join('\n'));await sleep(25)}throw Error(label+' timeout')};
let scene=await wait(s=>s.units?.length===4&&s.models?.every(m=>m.ready),'models ready');await page.mouse.click(640,360);
if(mode==='hash'){
const samples=[];while(server.exitCode===null){scene=await read();if(scene?.effects?.pool?.active>0)samples.push({pool:scene.effects.pool,health:scene.effects.combat.health});await sleep(40)}
if(server.exitCode!==0||!samples.length)throw Error('hash match failed or effects never drew');
const records=fs.readFileSync(out+'/audit.jsonl','utf8').trim().split('\n').map(JSON.parse);const tick=records.filter(r=>r.record==='tick').map(r=>r.data).find(r=>r?.tick===240);if(!tick)throw Error('no tick240 state');
const state=tick.state;result.excluded={claims:state.claims,reason:'Random session owner identifiers; all gameplay fields retained'};delete state.claims;
const canonical=v=>v===null||typeof v!=='object'?JSON.stringify(v):Array.isArray(v)?'['+v.map(canonical).join(',')+']':'{'+Object.keys(v).sort().map(k=>JSON.stringify(k)+':'+canonical(v[k])).join(',')+'}';
fs.writeFileSync(out+'/canonical-state.json',canonical(state));result.hash=require('node:crypto').createHash('sha256').update(canonical(state)).digest('hex');result.tick=240;result.samples=samples;
}else if(mode==='warnings'){
const warning=s=>s.effects?.warning&&s.effects.warning.until>s.effects.combat.now;
await sleep(1800);scene=await read();result.onscreen={point:project(scene,[10,2,20]),warning:warning(scene),effects:scene.effects};if(!onscreen(result.onscreen.point)||warning(scene))throw Error('onscreen damage gave warning or fixture not onscreen');
await page.keyboard.down('ArrowRight');try{await wait(s=>!onscreen(project(s,[10,2,20])),'pan offscreen',10000)}finally{await page.keyboard.up('ArrowRight')}
scene=await wait(warning,'offscreen warning',5000);result.warning=scene;result.ui=await page.evaluate(()=>JSON.parse(turts_ui()));await page.screenshot({path:out+'/warning.png'});
if(!JSON.stringify(result.ui).includes('Your castle is under attack'))throw Error('warning text missing');
await page.keyboard.press('j');scene=await wait(s=>onscreen(project(s,[10,2,20])),'jump returns camera',3000);result.jump=scene;
await page.keyboard.down('ArrowRight');try{await wait(s=>!onscreen(project(s,[10,2,20])),'second pan',10000)}finally{await page.keyboard.up('ArrowRight')}
const started=result.warning.effects.warning.started;for(let i=0;i<30;i++){scene=await read();if(scene.effects.warning?.started>started)throw Error('same warning repeated inside cooldown');await sleep(80)}result.cooldown=true;
}else{
await page.mouse.move(...project(scene,[mode==='victory'?23:20,0,20]));for(let i=0;i<7;i++){await page.mouse.wheel(0,-100);await sleep(70)}scene=await read();
let own=scene.units.find(u=>u.unit_id===3),enemy=scene.units.find(u=>u.unit_id===4);await page.mouse.click(...project(scene,[own.translation[0],.75,own.translation[2]]));await page.mouse.click(...project(scene,[enemy.translation[0],mode==='victory'?1:.75,enemy.translation[2]]),{button:'right'});
await page.mouse.move(30,680);
if(mode==='victory'){scene=await wait(s=>!s.units.some(u=>u.unit_id===4),'castle destroyed',25000);await sleep(500);result.ui=await page.evaluate(()=>JSON.parse(turts_ui()));if(!JSON.stringify(result.ui).includes('Victory'))throw Error('victory HUD missing');await page.screenshot({path:out+'/victory.png'});result.final=scene;}
else{
// Capture scene and canvas in one browser task so each mask has the correct camera and actor positions.
for(let i=0;i<180;i++){const c=await page.evaluate(()=>({scene:JSON.parse(turts_scene()),png:document.querySelector('canvas').toDataURL('image/png').split(',')[1]}));const png=decodePng(Buffer.from(c.png,'base64'));scene=c.scene;own=scene.units.find(u=>u.unit_id===3);enemy=scene.units.find(u=>u.unit_id===4);if(!own||!enemy)throw Error('combat fixture died');
const a=project(scene,[...own.translation.slice(0,1),.9,own.translation[2]]),b=project(scene,[enemy.translation[0],.9,enemy.translation[2]]);let bolt=0,sparks=0,white=0,blue=0;
for(let y=Math.max(0,Math.floor(Math.min(a[1],b[1])-26));y<Math.min(720,Math.max(a[1],b[1])+27);y++)for(let x=Math.max(0,Math.floor(Math.min(a[0],b[0])-26));x<Math.min(1280,Math.max(a[0],b[0])+27);x++){const p=pixelAt(png,x,y);if(!p)continue;const [r,g,z]=p,da=Math.hypot(x-a[0],y-a[1]),db=Math.hypot(x-b[0],y-b[1]);if(da>20&&db>20&&r>140&&r>g+8&&g>z+25)bolt++;if(db<24&&db>9&&r>190&&g>160&&z<150&&r-g>25&&r-g<75&&g-z>45)sparks++;if(da<15&&r>220&&g>220&&z>220)white++;if(da<15&&z>r*1.3&&z>g*1.1)blue++;}
result.frames.push({i,now:scene.effects.combat.now,bolt,sparks,white,blue,effects:scene.effects});if(bolt||sparks||scene.effects.combat.flashes.includes(3))fs.writeFileSync(out+`/frame-${i}.png`,Buffer.from(c.png,'base64'));await sleep(10)}
result.bolt=Math.max(...result.frames.map(f=>f.bolt));result.sparks=Math.max(...result.frames.map(f=>f.sparks));const flash=result.frames.filter(f=>f.effects.combat.flashes.includes(3));result.flash=flash.map(f=>({now:f.now,white:f.white,blue:f.blue}));result.teamReturn=flash.some(f=>result.frames.some(l=>l.now>f.now&&l.now-f.now<=.3&&!l.effects.combat.flashes.includes(3)&&l.blue>0));
if(!result.bolt||result.sparks<2)throw Error('FX3 bolt or spark pixels missing');if(!flash.some(f=>f.white>0)||!result.teamReturn)throw Error('FX4 white tint or team return missing');}
}
if(result.errors.some(e=>/shader|panicked|validation error/i.test(e)))throw Error('GPU validation failure');result.status='PASS';console.log(JSON.stringify({status:result.status,mode,bolt:result.bolt,sparks:result.sparks,teamReturn:result.teamReturn}));
}catch(e){result.status='FAIL';result.error=String(e.stack||e);console.error(e);process.exitCode=1}finally{fs.writeFileSync(out+'/result.json',JSON.stringify(result,null,2));if(context)await context.close();if(server&&server.exitCode===null){server.kill('SIGTERM');await new Promise(r=>server.once('exit',r))}fs.rmSync(profile,{recursive:true,force:true})}})();
