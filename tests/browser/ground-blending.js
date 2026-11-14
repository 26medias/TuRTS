// Actual GPU pixels, ordinary fallback, and real quality controls. Each case has a fresh game and profile.
const fs=require('node:fs'),net=require('node:net'),{spawn}=require('node:child_process'),{chromium}=require('playwright-core');
const {decodePng}=require('./harness/png');
const [prepared,fixtures,out]=process.argv.slice(2);if(!out)throw Error('PREPARED FIXTURES OUTPUT');fs.mkdirSync(out,{recursive:true});
const sleep=ms=>new Promise(r=>setTimeout(r,ms));
const port=()=>new Promise(r=>{const s=net.createServer();s.listen(0,'127.0.0.1',()=>{const p=s.address().port;s.close(()=>r(p))})});
function project(s,v){const m=s.clip_from_world,c=[0,1,2,3].map(r=>[...v,1].reduce((a,x,k)=>a+m[k*4+r]*x,0)),[x,y,w,h]=s.viewport_logical_rect;return[x+(c[0]/c[3]+1)*w/2,y+(1-c[1]/c[3])*h/2]}
function average(image,point){let sum=[0,0,0],n=0;const [x,y]=point.map(Math.round);if(x<4||y<4||x>=image.width-4||y>=image.height-4)throw Error('sample outside image');for(let dy=-3;dy<=3;dy++)for(let dx=-3;dx<=3;dx++){const i=((y+dy)*image.width+x+dx)*4;for(let c=0;c<3;c++)sum[c]+=image.rgba[i+c];n++}return sum.map(v=>v/n)}
const result={status:'RUNNING',cases:{},errors:[]};
async function run(name){let server,context;const profile=fs.mkdtempSync('/tmp/turts-ground-'),folder=out+'/'+name;fs.mkdirSync(folder);const entry=result.cases[name]={profile,errors:[],console:[]};try{
 const http=await port(),ws=await port(),url=`http://127.0.0.1:${http}`;
 fs.writeFileSync(folder+'/server.ron',`(game_package:"${fixtures}/${name}.tar",http_listen:"127.0.0.1:${http}",ws_listen:"127.0.0.1:${ws}",public_url:"${url}",tick_hz:20,static_dir:"${prepared}/dist")`);
 const fd=fs.openSync(folder+'/server.log','w');server=spawn(prepared+'/bin/turts_server',[folder+'/server.ron'],{stdio:['ignore',fd,fd],env:{...process.env,TURTS_COMPUTER_SLOT:''}});fs.closeSync(fd);
 let ready=false;for(let i=0;i<300;i++){try{if((await fetch(url+'/advertisement.json')).ok){ready=true;break}}catch{}if(server.exitCode!==null)throw Error('server stopped');await sleep(50)}if(!ready)throw Error('server timeout');
 context=await chromium.launchPersistentContext(profile,{executablePath:'/usr/bin/google-chrome',headless:false,viewport:{width:1280,height:720},ignoreDefaultArgs:['--enable-unsafe-swiftshader'],args:['--enable-gpu','--alsa-output-device=null','--enable-unsafe-webgpu','--enable-features=Vulkan','--use-angle=vulkan','--disable-vulkan-surface','--no-first-run','--no-default-browser-check']});const page=context.pages()[0];
 page.on('pageerror',e=>entry.errors.push(String(e)));page.on('console',m=>{entry.console.push(m.text());if(m.type()==='error'||/panicked|shader.*error|validation error|rendererror/i.test(m.text()))entry.errors.push(m.text())});
 await page.goto(url+'/?server='+encodeURIComponent(url)+'&ws='+encodeURIComponent(`ws://127.0.0.1:${ws}`));
 const read=()=>page.evaluate(async()=>{if(typeof turts_debug_request!=='function'||typeof turts_scene!=='function'||!turts_scene())return null;const d=JSON.parse(await turts_debug_request());return{environment:d.environment,scene:JSON.parse(turts_scene()),ui:JSON.parse(turts_ui())}});
 const wait=async(fn,label)=>{const end=Date.now()+90000;while(Date.now()<end){if(entry.errors.some(e=>/panicked|shader|validation|rendererror/i.test(e)))throw Error(entry.errors.join('\n'));const s=await read();if(s&&fn(s))return s;await sleep(100)}throw Error(label+' timeout')};
 let s=await wait(s=>s.scene.models?.length>=6&&s.scene.models.every(m=>m.ready)&&s.environment?.ground?.length,'scene');await page.mouse.click(640,400);await sleep(600);s=await read();entry.high=s;
 const overlay=s.environment.ground[0].overlay;if(name==='reference'){if(overlay!==null)throw Error('reference retained overlay')}else if(overlay?.patches!==6)throw Error('layer patches absent');
 if(name==='textured'&&overlay.texture_size?.[0]!==1024)throw Error('High layer texture missing');
 const samples={interior:[6,0,27],edge:[3.15,0,27],outside:[32,0,32],path:[17,0,18]};
 const image=decodePng(await page.screenshot({path:folder+'/high.png'}));entry.samples={};for(const [key,world] of Object.entries(samples)){const point=project(s.scene,world);entry.samples[key]={world,point,rgb:average(image,point)}}
 if(name==='textured'){
  await page.keyboard.press('F10');const menu=s=>s.ui.find(w=>w.kind==='settings_menu'&&w.visible);await wait(menu,'settings');let quality;
  for(let i=0;i<15;i++){s=await read();const m=menu(s);quality=m?.text_nodes?.find(t=>t.text==='Graphics quality'&&t.visible&&t.rect);if(quality)break;const title=m?.text_nodes?.find(t=>t.text==='Settings'&&t.visible&&t.rect);if(!title)throw Error('settings title absent');await page.mouse.move(title.rect[0]+title.rect[2]/2,m.rect[1]+m.rect[3]*.7);await page.mouse.wheel(0,450);await sleep(300)}
  if(!quality)throw Error('quality control not reachable');const click=()=>page.mouse.click(quality.rect[0]+quality.rect[2]/2,quality.rect[1]+quality.rect[3]/2);
  await click();entry.low=await wait(s=>s.environment.camera.fxaa===false&&s.environment.ground[0].overlay?.texture_size?.[0]===512,'Low texture');
  if(entry.low.environment.ground[0].overlay.patches!==6)throw Error('Low dropped patches');
  await click();await wait(s=>s.environment.camera.fxaa===true,'Medium');await click();entry.restored=await wait(s=>s.environment.ground[0].overlay?.texture_size?.[0]===1024,'High texture restored');
  await page.keyboard.press('F10');await sleep(250);await page.screenshot({path:folder+'/restored.png'});
 }
 if(name==='plain'){
  await page.keyboard.down('ArrowRight');await sleep(180);await page.keyboard.up('ArrowRight');await sleep(250);s=await read();entry.panned=s;
  const image=decodePng(await page.screenshot({path:folder+'/panned.png'}));entry.panned_interior=average(image,project(s.scene,samples.interior));
 }
 if(entry.errors.some(e=>/panicked|shader|validation|rendererror/i.test(e)))throw Error(entry.errors.join('\n'));entry.status='PASS';
}finally{if(context)await context.close();if(server&&server.exitCode===null){const end=new Promise(r=>server.once('exit',r));server.kill('SIGTERM');await end}fs.rmSync(profile,{recursive:true,force:true});entry.profile_removed=!fs.existsSync(profile)}}
(async()=>{try{for(const name of ['reference','plain','textured'])await run(name);
const distance=(a,b)=>a.reduce((n,v,i)=>n+Math.abs(v-b[i]),0),ref=result.cases.reference.samples,plain=result.cases.plain.samples;
result.pixel_delta=Object.fromEntries(Object.keys(ref).map(k=>[k,distance(ref[k].rgb,plain[k].rgb)]));
if(result.pixel_delta.interior<35||result.pixel_delta.path<15)throw Error('clearings or paths did not change visible ground');
if(result.pixel_delta.edge>result.pixel_delta.interior*.5)throw Error('feather edge is not soft');
if(result.pixel_delta.outside>8)throw Error('overlay leaked outside authored patches');
result.pan_delta=distance(plain.interior.rgb,result.cases.plain.panned_interior);if(result.pan_delta>18)throw Error('world-aligned dirt changed during pan');
result.status='PASS';console.log(JSON.stringify({status:result.status,pixel_delta:result.pixel_delta,pan_delta:result.pan_delta}));
}catch(e){result.status='FAIL';result.error=String(e.stack||e);process.exitCode=1;console.error(e)}finally{fs.writeFileSync(out+'/result.json',JSON.stringify(result,null,2))}})();
