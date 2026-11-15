// BF9 Stockyard: observe an unmodified public profile through a real browser.
// Skirmish Victory uses the existing damage-win.js driver in the final run plan.
const fs=require('node:fs'),path=require('node:path'),net=require('node:net'),crypto=require('node:crypto');
const {spawn,spawnSync}=require('node:child_process');
const {chromium}=require('playwright-core');
const [prepared,out,sourceArg]=process.argv.slice(2);
if(!out)throw Error('PREPARED OUTPUT [SOURCE_ROOT]');
const sourceRoot=sourceArg||path.resolve(__dirname,'../..');
fs.mkdirSync(out,{recursive:true});
const packageSource=fs.existsSync(prepared+'/pack/stockyard')?prepared+'/pack/stockyard':sourceRoot+'/mods/stockyard';
const fixture=out+'/pack/stockyard';fs.mkdirSync(out+'/pack',{recursive:true});fs.cpSync(packageSource,fixture,{recursive:true});
const sourceMap=fs.readFileSync(fixture+'/maps/field.ron','utf8');
const initialCounts={};for(const m of sourceMap.matchAll(/unit:\s*"stockyard:([^"]+)"\s*,\s*owner_slot:\s*1/g))initialCounts['stockyard:'+m[1]]=(initialCounts['stockyard:'+m[1]]||0)+1;
const profile=fs.readFileSync(fixture+'/bots/opponent.ron','utf8');
const sleep=ms=>new Promise(r=>setTimeout(r,ms));
const port=()=>new Promise(r=>{const s=net.createServer();s.listen(0,'127.0.0.1',()=>{const p=s.address().port;s.close(()=>r(p));});});
function project(scene,pos){const m=scene.clip_from_world,c=[0,1,2,3].map(r=>[...pos,1].reduce((v,x,i)=>v+m[i*4+r]*x,0));const[x,y,w,h]=scene.viewport_logical_rect;return[x+(c[0]/c[3]+1)*w/2,y+(1-c[1]/c[3])*h/2];}
(async()=>{
 let server,context,page,browserProfile,read;
 const started=Date.now();
 const result={status:'RUNNING',package:'stockyard',public_profile:'stockyard:opponent',seed:42,source:packageSource,initial_counts:initialCounts,profile_sha256:crypto.createHash('sha256').update(profile).digest('hex'),actions:[],samples:[],errors:[]};
 const save=()=>fs.writeFileSync(out+'/result.json',JSON.stringify(result,null,2));
 const units=s=>s.debug.replicated_world.units;
 const enemy=(s,kind)=>units(s).filter(u=>u.owner_slot===1&&(!kind||u.content_id==='stockyard:'+kind));
 const health=(s,id)=>s.debug.replicated_world.healths.find(h=>h.unit===id)?.current;
 const alive=s=>!s.ui.some(w=>w.id==='result_title'&&w.visible&&['Victory','Defeat','Draw'].includes(w.text));
 try{
  if(!Object.keys(initialCounts).length||!profile.includes('engine:opening'))throw Error('not the public Stockyard package');
  const packed=spawnSync(prepared+'/bin/turts-pack',[fixture],{encoding:'utf8'});fs.writeFileSync(out+'/pack.log',packed.stdout+packed.stderr);if(packed.status!==0)throw Error('Stockyard pack failed');
  result.package_sha256=crypto.createHash('sha256').update(fs.readFileSync(fixture+'.tar')).digest('hex');
  const http=await port(),ws=await port(),url=`http://127.0.0.1:${http}`;
  fs.writeFileSync(out+'/server.ron',`(game_package:"${fixture}.tar",http_listen:"127.0.0.1:${http}",ws_listen:"127.0.0.1:${ws}",public_url:"${url}",tick_hz:20,static_dir:"${prepared}/dist")`);
  const args=[out+'/server.ron','--bot','1,stockyard:opponent,42','--economy-log',out+'/economy.jsonl','--order-log',out+'/orders.jsonl'];
  result.server_arguments=args;
  const fd=fs.openSync(out+'/server.log','w');server=spawn(prepared+'/bin/turts_server',args,{stdio:['ignore',fd,fd],env:{...process.env,TURTS_COMPUTER_SLOT:''}});fs.closeSync(fd);
  for(let i=0;i<300;i++){try{if((await fetch(url+'/advertisement.json')).ok)break;}catch{}if(server.exitCode!==null)throw Error('Stockyard server stopped');await sleep(50);}
  browserProfile=fs.mkdtempSync('/tmp/turts-public-bots-');
  context=await chromium.launchPersistentContext(browserProfile,{executablePath:'/usr/bin/google-chrome',headless:false,viewport:{width:1280,height:720},ignoreDefaultArgs:['--enable-unsafe-swiftshader'],args:['--enable-gpu','--alsa-output-device=null','--enable-unsafe-webgpu','--enable-features=Vulkan','--use-angle=vulkan','--disable-vulkan-surface','--no-first-run','--no-default-browser-check']});
  page=context.pages()[0];page.on('pageerror',error=>result.errors.push(String(error)));
  await page.goto(url+'/?server='+encodeURIComponent(url)+'&ws='+encodeURIComponent(`ws://127.0.0.1:${ws}`));
  read=()=>page.evaluate(async()=>typeof turts_scene==='function'&&turts_scene()?{debug:JSON.parse(await turts_debug_request()),scene:JSON.parse(turts_scene()),ui:JSON.parse(turts_ui())}:null);
  const wait=async(test,label,ms=30000)=>{const end=Date.now()+ms;while(Date.now()<end){const s=await read();if(s&&test(s))return s;if(result.errors.length)throw Error(result.errors.join('\n'));await sleep(100);}throw Error(label+' timeout');};
  const click=async(p,label)=>{result.actions.push({label,point:p,seconds:(Date.now()-started)/1000});await page.mouse.click(...p);};
  const focus=async(x,y)=>{const s=await read(),map=s.scene.widgets.find(w=>w.id==='stockyard_minimap');if(!map)throw Error('Stockyard minimap missing');await click([map.rect[0]+x/64*map.rect[2],map.rect[1]+(1-y/48)*map.rect[3]],'look on map');await sleep(150);};
  const capture=async(name,s,point)=>{if(point)await focus(point[0],point[1]);s=await read();result[name]=s;save();await page.screenshot({path:out+'/'+name+'.png'});};
  const select=async id=>{const s=await read(),u=s.scene.units.find(u=>u.unit_id===id);if(!u)throw Error('rendered unit missing '+id);await click(project(s.scene,u.translation),'select public unit '+id);await wait(s=>s.debug.selection.includes(id),'selection '+id,5000);};
  result.initial=await wait(s=>s.debug.slot===0&&s.scene.models?.length>=12&&s.scene.models.every(m=>m.ready),'Stockyard ready',90000);
  await click([640,360],'start');await page.screenshot({path:out+'/initial.png'});save();
  const deadline=Date.now()+180000;
  while(Date.now()<deadline){
   const s=await read();if(!s){await sleep(100);continue;}
   if(result.errors.length)throw Error(result.errors.join('\n'));
   const tick=s.debug.replicated_world.tick;
   if(result.samples.at(-1)?.tick!==tick)result.samples.push({tick,seconds:(Date.now()-started)/1000,enemy_units:enemy(s).map(u=>({id:u.unit_id,content:u.content_id,hp:health(s,u.unit_id)})),nodes:s.debug.replicated_world.nodes});
   if(!result.gather){const node=s.debug.replicated_world.nodes.find(n=>n.pos.x>32&&n.remaining<2000);if(node){await capture('gather',s,[node.pos.x,node.pos.y]);}}
   if(!result.produce){const workers=enemy(s,'worker');if(workers.length>(initialCounts['stockyard:worker']||0)){const unit=workers.at(-1);await focus(unit.pos[0],unit.pos[1]);await select(unit.unit_id);await capture('produce',s);}}
   if(!result.build){const towers=enemy(s,'tower');if(towers.length>(initialCounts['stockyard:tower']||0)){const unit=towers.find(u=>u.unit_id>12&&health(s,u.unit_id)===400);if(unit){await focus(unit.pos[0],unit.pos[1]);await select(unit.unit_id);await capture('build',s);}}}
   if(!result.attack){const victim=units(s).find(u=>u.owner_slot===0&&health(s,u.unit_id)>0&&health(s,u.unit_id)<u.max_health);if(victim){await focus(victim.pos[0],victim.pos[1]);await select(victim.unit_id);await capture('attack',s);}}
   if(result.gather&&result.produce&&result.build&&result.attack)break;
   if(!alive(s))throw Error('match ended before all public profile actions were visible');
   save();await sleep(400);
  }
  if(!result.gather||!result.produce||!result.build||!result.attack)throw Error('public profile did not demonstrate gather/produce/build/attack');
  result.status='PASS';console.log(JSON.stringify({status:result.status,profile:result.public_profile,seconds:(Date.now()-started)/1000}));
 }catch(error){result.status='FAIL';result.error=String(error.stack||error);if(read)try{result.failure=await read();}catch{}if(page)try{await page.screenshot({path:out+'/failure.png'});}catch{}console.error(error);process.exitCode=1;}
 finally{if(context)await context.close();if(server&&server.exitCode===null){const ended=new Promise(r=>server.once('exit',r));server.kill('SIGTERM');await Promise.race([ended,sleep(8000)]);}if(browserProfile)fs.rmSync(browserProfile,{recursive:true,force:true});result.profile_removed=!browserProfile||!fs.existsSync(browserProfile);save();}
})();
