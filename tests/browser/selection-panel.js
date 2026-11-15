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
const kinds = ['base','worker','soldier','barracks','swordsman','base'];
const positions = [[10,24],[14,20],[14,28],[20,24],[20,28],[30,28]];
const placements = kinds.map((kind,i)=>`(unit:"skirmish:${kind}",owner_slot:${i===5?1:0},pos:(x:${positions[i][0]}.0,y:${positions[i][1]}.0),contents:[])`);
fs.writeFileSync(fixture+'/maps/field.ron',`(id:"field",size:(x:64.0,y:48.0),placements:[${placements}],nodes:[],slot_economy:[])`);
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
 const profile=fs.mkdtempSync('/tmp/turts-selection-panel-');
 const result={status:'RUNNING',errors:[],selections:[],profile};
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
  const portraitHashes={};
  for(const [width,height] of [[1280,720],[1024,768]]) {
   await page.setViewportSize({width,height});await sleep(300);
   for(let id=1;id<=5;id++) {
    await clickUnit(id);
    let s=await wait(s=>s.selection.length===1&&s.selection[0]===id&&widget(s,'selection_name'),'select '+id);
    const expected=['Castle','Villager','Crossbowman','Barracks','Swordsman'][id-1];
    if(widget(s,'selection_name').text!==expected)throw Error('wrong name '+JSON.stringify(widget(s,'selection_name')));
    const max=s.debug.replicated_world.units.find(u=>u.unit_id===id).max_health;
    if(widget(s,'selection_health_text').text!==`Health: ${max} / ${max}`)throw Error('wrong full health');
    for(const name of ['selection_panel','selection_portrait','selection_name','selection_health','selection_health_text']) {
     const w=widget(s,name);if(!w||!w.rect||w.rect[0]<0||w.rect[1]<0||w.rect[0]+w.rect[2]>width||w.rect[1]+w.rect[3]>height)throw Error('panel bounds '+name);
    }
    await sleep(250);const shot=await page.screenshot({path:`${out}/${width}-${kinds[id-1]}.png`});
    const image=decodePng(shot),rect=widget(s,'selection_portrait').rect, colours=new Set();
    for(let y=Math.ceil(rect[1]);y<rect[1]+rect[3];y++)for(let x=Math.ceil(rect[0]);x<rect[0]+rect[2];x++){const k=(y*image.width+x)*4;colours.add(Array.from(image.rgba.slice(k,k+3)).join(','))}
    if(colours.size<40)throw Error('portrait missing pixels '+id);
    portraitHashes[id]=[...colours].sort().join(';');
    result.selections.push({viewport:[width,height],id,name:expected,health:widget(s,'selection_health_text').text,colours:colours.size,panel:widget(s,'selection_panel').rect});
   }
  }
  if(new Set(Object.values(portraitHashes)).size!==5)throw Error('portraits not distinct');
  await clickUnit(2);await page.keyboard.down('Shift');await clickUnit(3);await page.keyboard.up('Shift');
  result.multiple=await wait(s=>s.selection.includes(2)&&s.selection.includes(3),'multiple selection');
  if(!widget(result.multiple,'selection_name'))throw Error('multiple primary missing');
  let s=await read();await page.mouse.click(...project(s.scene,[25,0,36]));
  result.empty=await wait(s=>s.selection.length===0,'clear selection');
  if(widget(result.empty,'selection_panel'))throw Error('panel remained after clear');
  await clickUnit(6);s=await wait(s=>s.selection.includes(6),'enemy selection');
  if(widget(s,'selection_health_text').text!=='Health: 1000 / 1000')throw Error('enemy initial health missing');
  await clickUnit(3);await clickUnit(6,'right');await clickUnit(6);
  result.damaged=await wait(s=>s.selection.includes(6)&&s.scene.effects.combat.health['6']<1000,'enemy damage',20000);
  const hp=result.damaged.scene.effects.combat.health['6'];
  if(widget(result.damaged,'selection_health_text').text!==`Health: ${hp} / 1000`)throw Error('health panel does not follow damage');
  await page.screenshot({path:out+'/enemy-damaged.png'});
  result.status='PASS';console.log('PASS five portraits, names, health, enemy damage, multiple and empty selection, two viewport bounds');
 } catch(e) {result.status='FAIL';result.error=String(e.stack||e);if(read)try{result.failure=await read()}catch{}if(page)try{await page.screenshot({path:out+'/failure.png'})}catch{}console.error(e);process.exitCode=1;}
 finally {if(context)await context.close();if(server&&server.exitCode===null){const ended=new Promise(r=>server.once('exit',r));server.kill('SIGTERM');await ended}fs.rmSync(profile,{recursive:true,force:true});result.profile_removed=!fs.existsSync(profile);fs.writeFileSync(out+'/result.json',JSON.stringify(result,null,2))}
})();
