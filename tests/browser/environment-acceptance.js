// Real browser input checks for map presentation and its quality fallback.
const fs=require('node:fs'),net=require('node:net'),{spawn}=require('node:child_process'),{chromium}=require('playwright-core');
const {sceneProblem}=require('./t7-oracle');
const [prepared,out]=process.argv.slice(2);if(!out)throw Error('PREPARED OUTPUT');fs.mkdirSync(out,{recursive:true});
const sleep=ms=>new Promise(r=>setTimeout(r,ms)),port=()=>new Promise(r=>{const s=net.createServer();s.listen(0,'127.0.0.1',()=>{const p=s.address().port;s.close(()=>r(p))})});
function project(s,v){const m=s.clip_from_world,c=[0,1,2,3].map(r=>[...v,1].reduce((a,x,k)=>a+m[k*4+r]*x,0)),[x,y,w,h]=s.viewport_logical_rect;return[x+(c[0]/c[3]+1)*w/2,y+(1-c[1]/c[3])*h/2]}
(async()=>{let server,context,page,read;const profile=fs.mkdtempSync('/tmp/turts-environment-'),result={status:'RUNNING',errors:[],profile};try{
 const http=await port(),ws=await port(),url=`http://127.0.0.1:${http}`;
 fs.writeFileSync(out+'/server.ron',`(game_package:"${prepared}/pack/skirmish.tar",http_listen:"127.0.0.1:${http}",ws_listen:"127.0.0.1:${ws}",public_url:"${url}",tick_hz:20,static_dir:"${prepared}/dist")`);
 const fd=fs.openSync(out+'/server.log','w');server=spawn(prepared+'/bin/turts_server',[out+'/server.ron'],{stdio:['ignore',fd,fd],env:{...process.env,TURTS_COMPUTER_SLOT:''}});fs.closeSync(fd);
 for(let i=0;i<300;i++){try{if((await fetch(url+'/advertisement.json')).ok)break}catch{}if(server.exitCode!==null)throw Error('server stopped');await sleep(50)}
 context=await chromium.launchPersistentContext(profile,{executablePath:'/usr/bin/google-chrome',headless:false,viewport:{width:1280,height:720},ignoreDefaultArgs:['--enable-unsafe-swiftshader'],args:['--enable-gpu','--alsa-output-device=null','--enable-unsafe-webgpu','--enable-features=Vulkan','--use-angle=vulkan','--disable-vulkan-surface','--no-first-run','--no-default-browser-check']});page=context.pages()[0];page.on('pageerror',e=>result.errors.push(String(e)));
 await page.goto(url+'/?server='+encodeURIComponent(url)+'&ws='+encodeURIComponent(`ws://127.0.0.1:${ws}`));
 read=()=>page.evaluate(async()=>{if(typeof turts_debug_request!=='function'||typeof turts_scene!=='function'||!turts_scene())return null;const d=JSON.parse(await turts_debug_request());return{debug:d,environment:d.environment,world:d.replicated_world,selection:d.selection,notices:d.notices,scene:JSON.parse(turts_scene()),ui:JSON.parse(turts_ui())}});
 const wait=async(fn,label,ms=90000)=>{const end=Date.now()+ms;while(Date.now()<end){const s=await read();if(s&&fn(s))return s;if(result.errors.length)throw Error(result.errors.join('\n'));await sleep(100)}throw Error(label+' timeout')};
 let s=await wait(s=>s.scene.models?.length>0&&s.scene.models.every(m=>m.ready)&&s.environment?.camera?.fxaa===true,'models and environment');
 const [vx,vy,vw,vh]=s.scene.viewport_logical_rect;await page.mouse.click(vx+vw/2,vy+vh*.65);await sleep(300);s=await read();result.high=s;
 const near=(a,b)=>Math.abs(a-b)<.001;
 if(s.environment.camera.samples!==1||!s.environment.camera.fog||!near(s.environment.camera.fog.colour[3],.2))throw Error('High camera components wrong');
 if(JSON.stringify(s.environment.map_size)!=='[64,48]')throw Error('decorative extent changed map dimensions');
 const g=s.environment.ground[0];if(!g||!near(g.min[0],-64)||!near(g.min[2],-64)||!near(g.max[0],128)||!near(g.max[2],112))throw Error('actual ground vertices do not extend on all sides');
 await page.screenshot({path:out+'/high.png'});
 await page.evaluate(()=>{window.__envWheel=[];document.addEventListener('wheel',event=>window.__envWheel.push([event.clientX,event.clientY,event.deltaY,document.hasFocus()]),{capture:true,passive:true})});
 await page.keyboard.press('F10');
 const menu=s=>s.ui.find(w=>w.kind==='settings_menu'&&w.visible);
 await wait(s=>menu(s),'settings');
 let quality;
 for(let n=0;n<15;n++){s=await read();const m=menu(s);quality=m?.text_nodes?.find(t=>t.text==='Graphics quality'&&t.visible&&t.rect);if(quality)break;if(!m)throw Error('settings disappeared');const title=m.text_nodes.find(t=>t.text==='Settings'&&t.visible&&t.rect);if(!title)throw Error('settings title missing');await page.mouse.move(title.rect[0]+title.rect[2]/2,m.rect[1]+m.rect[3]*.7);await sleep(100);await page.mouse.wheel(0,450);await sleep(300)}
 if(!quality)throw Error('graphics quality control not reachable');
 const clickQuality=async()=>{await page.mouse.click(quality.rect[0]+quality.rect[2]/2,quality.rect[1]+quality.rect[3]/2)};
 await clickQuality();result.low=await wait(s=>s.environment.camera.fxaa===false&&s.environment.camera.fog===null,'Low fallback');
 await clickQuality();result.medium=await wait(s=>s.environment.camera.fxaa===true&&s.environment.camera.fog!==null,'Medium effects');
 await clickQuality();await sleep(350);await page.keyboard.press('F10');await wait(s=>!menu(s),'settings closed');
 s=await read();result.restored=s;if(s.environment.camera.fxaa!==true||s.environment.camera.samples!==1)throw Error('High restoration failed');
 await page.screenshot({path:out+'/restored.png'});
 // The default debug cap can delay publication. The scene must use the served frame.
 const moving=s.scene.units.find(u=>u.unit_id===3);
 await page.mouse.click(...project(s.scene,moving.translation));await wait(s=>s.selection.includes(3),'coherence selection');
 await page.mouse.click(...project((await read()).scene,[20,0,25]),{button:'right'});
 result.coherent_movement=[];
 for(let i=0;i<20;i++){const sample=await read();if(sample.debug.units.length!==sample.scene.units.length)throw Error('debug/scene unit count mismatch');for(const unit of sample.debug.units){const problem=sceneProblem({units:[unit]},{...sample.scene,units:sample.scene.units.filter(mesh=>mesh.unit_id===unit.unit_id)});if(problem)throw Error('debug/scene frame mismatch: '+problem)}result.coherent_movement.push(sample.scene.units.find(u=>u.unit_id===3).translation)}
 if(new Set(result.coherent_movement.map(v=>JSON.stringify(v))).size<2)throw Error('coherence fixture did not move');
 const map=s.scene.widgets.find(w=>w.id==='skirmish_minimap').rect;
 await page.mouse.click(map[0]+.5/64*map[2],map[1]+map[3]/2);await sleep(300);s=await read();
 const soldier=s.scene.units.find(u=>u.unit_id===3);await page.mouse.click(...project(s.scene,soldier.translation));await wait(s=>s.selection.includes(3),'soldier selection');
 const outside=[-4,0,30],target=project((await read()).scene,outside);if(target[0]<vx||target[0]>=vx+vw||target[1]<vy||target[1]>=vy+vh)throw Error('outside ground not visible for input');
 await page.mouse.click(...target,{button:'right'});await sleep(2000);s=await read();result.outside_move={target:outside,state:s};
 if(s.world.units.some(u=>u.pos[0]<0||u.pos[0]>64||u.pos[1]<0||u.pos[1]>48))throw Error('unit moved outside authoritative map');
 const castle=s.scene.units.find(u=>u.unit_id===1);await page.mouse.click(...project(s.scene,castle.translation));
 s=await wait(s=>s.ui.some(w=>w.kind==='button'&&w.text?.includes('Build Barracks')&&w.visible&&w.enabled),'castle construction button');
 const button=s.ui.find(w=>w.kind==='button'&&w.text?.includes('Build Barracks')&&w.visible&&w.enabled),credits=s.world.economy.balances.find(b=>b.resource==='skirmish:credits').on_hand;
 await page.mouse.click(button.rect[0]+button.rect[2]/2,button.rect[1]+button.rect[3]/2);await page.mouse.click(...project((await read()).scene,[-4,0,24]));await sleep(1000);s=await read();result.outside_build=s;
 if(s.world.units.some(u=>u.content_id==='skirmish:barracks')||s.world.economy.balances.find(b=>b.resource==='skirmish:credits').on_hand!==credits)throw Error('decorative ground accepted an outside building or charge');
 await page.screenshot({path:out+'/outside.png'});
 const idleBaseline=await read();const beforeIdle=idleBaseline.scene.effects.combat.now;
 await sleep(1200);
 const cached=await page.evaluate(()=>JSON.parse(turts_scene()).effects.combat.now);
 result.idle_capture={before:beforeIdle,cached};
 if(cached-beforeIdle>.3)throw Error('scene captured repeatedly without a reader: '+JSON.stringify(result.idle_capture));
 const refreshed=await read();result.idle_capture.refreshed=refreshed.scene.effects.combat.now;
 if(refreshed.scene.effects.combat.now-cached<.9)throw Error('scene did not refresh with the debug request');
 result.status='PASS';console.log('PASS environment, real quality controls, map bounds, and outside construction');
}catch(e){result.status='FAIL';result.error=String(e.stack||e);if(read)try{result.failure_state=await read();result.dom=await page.evaluate(()=>({focus:document.hasFocus(),active:document.activeElement?.tagName,wheel:window.__envWheel}))}catch{}console.error(e);if(page)try{await page.screenshot({path:out+'/failure.png'})}catch{}process.exitCode=1}
finally{if(context)await context.close();if(server&&server.exitCode===null){const end=new Promise(r=>server.once('exit',r));server.kill('SIGTERM');await end}fs.rmSync(profile,{recursive:true,force:true});result.profile_removed=!fs.existsSync(profile);result.server_stopped=!server||server.exitCode!==null||server.signalCode!==null;result.server_pid=server?.pid;fs.writeFileSync(out+'/result.json',JSON.stringify(result,null,2))}})();
