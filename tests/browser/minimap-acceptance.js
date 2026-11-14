// Exercise the authored map with real pointer input and rendered pixels.
const fs=require('node:fs'),net=require('node:net'),{spawn,spawnSync}=require('node:child_process');
const {chromium}=require('playwright-core'),{decodePng,pixelAt}=require('./harness/png');
const [prepared,out]=process.argv.slice(2);if(!out)throw Error('PREPARED OUTPUT');fs.mkdirSync(out,{recursive:true});
const fixture=out+'/fixture';fs.cpSync(prepared+'/pack/skirmish',fixture,{recursive:true});
const place=(kind,slot,x,y)=>`(unit:"skirmish:${kind}",owner_slot:${slot},pos:(x:${x}.0,y:${y}.0),contents:[])`;
fs.writeFileSync(fixture+'/maps/field.ron',`(id:"field",size:(x:100.0,y:60.0),placements:[${[place('base',0,10,20),place('worker',0,15,25),place('soldier',0,12,38),place('base',1,80,35)].join(',')}],nodes:[(node:"skirmish:trees",pos:(x:20.0,y:40.0),remaining:2000)],slot_economy:[(slot:0,balances:[(balance:"skirmish:credits",amount:1000)])])`);
fs.writeFileSync(fixture+'/match.ron','(start:Immediate,victory:None,open_sites_per_slot:Some(4))');
const sleep=ms=>new Promise(r=>setTimeout(r,ms)),port=()=>new Promise(r=>{const s=net.createServer();s.listen(0,'127.0.0.1',()=>{const n=s.address().port;s.close(()=>r(n))})});
function project(s,v){const m=s.clip_from_world,c=[0,1,2,3].map(r=>[...v,1].reduce((a,x,k)=>a+m[k*4+r]*x,0));return[(c[0]/c[3]+1)*640,(1-c[1]/c[3])*360]}
function colourCount(png,point,colour,radius=5){let count=0;for(let y=Math.floor(point[1]-radius);y<=point[1]+radius;y++)for(let x=Math.floor(point[0]-radius);x<=point[0]+radius;x++){const p=pixelAt(png,x,y);if(p&&colour.every((v,k)=>Math.abs(v-p[k])<12))count++}return count}
(async()=>{let server,context;const profile=fs.mkdtempSync('/tmp/turts-minimap-');const result={status:'RUNNING',errors:[]};try{
const pack=spawnSync(prepared+'/bin/turts-pack',[fixture],{encoding:'utf8'});fs.writeFileSync(out+'/pack.log',pack.stdout+pack.stderr);if(pack.status!==0)throw Error('pack failed '+pack.stderr);
const http=await port(),ws=await port(),url=`http://127.0.0.1:${http}`;fs.writeFileSync(out+'/server.ron',`(game_package:"${fixture}.tar",http_listen:"127.0.0.1:${http}",ws_listen:"127.0.0.1:${ws}",public_url:"${url}",tick_hz:20,static_dir:"${prepared}/dist")`);
const fd=fs.openSync(out+'/server.log','w');server=spawn(prepared+'/bin/turts_server',[out+'/server.ron'],{stdio:['ignore',fd,fd],env:{...process.env,TURTS_COMPUTER_SLOT:''}});fs.closeSync(fd);
for(let i=0;i<200;i++){try{if((await fetch(url+'/advertisement.json')).ok)break}catch{}if(server.exitCode!==null)throw Error('server stopped');await sleep(50)}
context=await chromium.launchPersistentContext(profile,{executablePath:'/usr/bin/google-chrome',headless:false,viewport:{width:1280,height:720},ignoreDefaultArgs:['--enable-unsafe-swiftshader'],args:['--enable-gpu','--alsa-output-device=null','--enable-unsafe-webgpu','--enable-features=Vulkan','--use-angle=vulkan','--disable-vulkan-surface','--no-first-run','--no-default-browser-check']});const page=context.pages()[0];page.on('pageerror',e=>result.errors.push(String(e)));
await page.goto(url+'/?server='+encodeURIComponent(url)+'&ws='+encodeURIComponent(`ws://127.0.0.1:${ws}`));
const read=()=>page.evaluate(()=>typeof turts_scene==='function'&&turts_scene()?JSON.parse(turts_scene()):null);
const debug=()=>page.evaluate(async()=>JSON.parse(await turts_debug_request()));
const wait=async(fn,label,ms=60000)=>{const end=Date.now()+ms;while(Date.now()<end){const s=await read();if(s&&fn(s))return s;if(result.errors.length)throw Error(result.errors.join('\n'));await sleep(40)}throw Error(label+' timeout')};
let scene=await wait(s=>s.units?.length===4&&s.models?.every(m=>m.ready)&&s.widgets.some(w=>w.id==='skirmish_minimap'),'models and map');await page.mouse.click(640,360);await sleep(200);scene=await read();
const rect=scene.widgets.find(w=>w.id==='skirmish_minimap').rect;result.rect=rect;const mapPoint=(x,y)=>[rect[0]+x/100*rect[2],rect[1]+(1-y/60)*rect[3]];
const png=decodePng(await page.screenshot({path:out+'/map.png'}));
result.pixels={own:colourCount(png,mapPoint(10,20),[0,114,178]),enemy:colourCount(png,mapPoint(80,35),[230,159,0]),resource:colourCount(png,mapPoint(20,40),[242,204,96])};
let white=0;for(let y=Math.ceil(rect[1]+6);y<rect[1]+rect[3]-6;y++)for(let x=Math.ceil(rect[0]+6);x<rect[0]+rect[2]-6;x++){const p=pixelAt(png,x,y);if(p&&p[0]>240&&p[1]>240&&p[2]>240)white++}result.pixels.frame=white;
if(Object.values(result.pixels).some(n=>n<4)||white<40)throw Error('map marker or camera frame pixels missing');
const unit=scene.units.find(u=>u.unit_id===3);await page.mouse.click(...project(scene,[unit.translation[0],.75,unit.translation[2]]));await sleep(150);result.before=await debug();if(JSON.stringify(result.before.selection)!=='[3]')throw Error('soldier selection failed');
await page.mouse.click(...mapPoint(80,35));result.look=await wait(s=>{const p=project(s,[80,0,35]);return Math.hypot(p[0]-640,p[1]-360)<6},'left click looks at map point',3000);if(JSON.stringify((await debug()).selection)!=='[3]')throw Error('map look changed selection');
await page.mouse.move(...mapPoint(60,30));await page.mouse.down();await page.mouse.move(...mapPoint(40,20),{steps:8});await sleep(120);await page.mouse.up();result.drag=await wait(s=>{const p=project(s,[40,0,20]);return Math.hypot(p[0]-640,p[1]-360)<6},'map drag',3000);if(JSON.stringify((await debug()).selection)!=='[3]')throw Error('map drag changed selection');
const camera=result.drag.clip_from_world;await page.mouse.click(...mapPoint(30,30),{button:'right'});await page.mouse.move(400,680);await sleep(200);scene=await read();if(JSON.stringify(scene.clip_from_world)!==JSON.stringify(camera))throw Error('right click moved camera');
result.arrival=await wait(s=>{const u=s.units.find(u=>u.unit_id===3);return u&&Math.hypot(u.translation[0]-30,u.translation[2]-30)<.2},'map order arrival',12000);result.after=await debug();if(JSON.stringify(result.after.selection)!=='[3]')throw Error('map order changed selection');if(JSON.stringify(result.arrival.clip_from_world)!==JSON.stringify(camera))throw Error('camera drifted during map order');await page.screenshot({path:out+'/map-order.png'});
result.status='PASS';console.log(JSON.stringify({status:result.status,pixels:result.pixels,selection:result.after.selection}));
}catch(e){result.status='FAIL';result.error=String(e.stack||e);console.error(e);process.exitCode=1}finally{fs.writeFileSync(out+'/result.json',JSON.stringify(result,null,2));if(context)await context.close();if(server&&server.exitCode===null){server.kill('SIGTERM');await new Promise(r=>server.once('exit',r))}fs.rmSync(profile,{recursive:true,force:true})}})();
