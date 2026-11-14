// Exercise the real temporary HTTPS/WSS host and two fresh browser join commands.
const {decodePng,pixelAt}=require('./harness/png');
const fs=require('node:fs'),{spawn}=require('node:child_process'),{chromium}=require('playwright-core');
const [root,prepared,out]=process.argv.slice(2);if(!out)throw Error('ROOT PREPARED OUTPUT');fs.mkdirSync(out,{recursive:true});
const sleep=ms=>new Promise(r=>setTimeout(r,ms)),children=[],browsers=[],pages=[],result={status:'RUNNING',profiles:[],errors:[],networks:[]};
function child(args,name){const fd=fs.openSync(out+'/'+name+'.log','w'),p=spawn(args[0],args.slice(1),{detached:true,stdio:['ignore',fd,fd]});fs.closeSync(fd);children.push(p);return p}
async function line(name,pattern,p,ms=180000){const end=Date.now()+ms;while(Date.now()<end){const text=fs.readFileSync(out+'/'+name+'.log','utf8'),match=pattern.exec(text);if(match)return{match,text};if(p.exitCode!==null)throw Error(name+' exited '+p.exitCode+': '+text);await sleep(100)}throw Error(name+' startup timeout')}
async function read(i){return pages[i].evaluate(async()=>{
 if(typeof turts_debug_request!=='function'||typeof turts_ui!=='function')return null;
 const value=await turts_debug_request();if(!value)return null;const d=JSON.parse(value),scene=typeof turts_scene==='function'?turts_scene():null;
 return{scene:scene?JSON.parse(scene):{models:[],units:[],widgets:[]},ui:JSON.parse(turts_ui()),lobby:d.lobby,start:d.match_start,colours:d.slot_colours,
 world:d.replicated_world||{units:[]},slot:d.slot,match:d.match_id_decimal,selection:d.selection,secure:isSecureContext};
})}
async function wait(i,fn,label,ms=90000){const end=Date.now()+ms;while(Date.now()<end){const s=await read(i);if(s&&fn(s))return s;if(result.errors.length)throw Error(result.errors.join('\n'));await sleep(150)}throw Error(label+' timeout')}
async function clickWidget(i,id){const s=await wait(i,s=>s.ui.some(w=>w.id===id&&w.visible&&w.enabled),'widget '+id),w=s.ui.find(w=>w.id===id&&w.visible&&w.enabled);await pages[i].mouse.click(w.rect[0]+w.rect[2]/2,w.rect[1]+w.rect[3]/2)}
const modelReady=s=>s.scene.models.length>0&&s.scene.models.every(m=>m.ready);
const ownSeat=s=>s.lobby?.seats.find(seat=>seat.index===s.lobby.you);
function project(s,v){const m=s.clip_from_world,c=[0,1,2,3].map(r=>[...v,1].reduce((a,x,k)=>a+m[k*4+r]*x,0)),[x,y,w,h]=s.viewport_logical_rect;return[x+(c[0]/c[3]+1)*w/2,y+(1-c[1]/c[3])*h/2]}
(async()=>{try{
const host=child(['python3','-I','-B',root+'/scripts/skirmish_share.py','--prepared',prepared,'--out',out+'/services'],'host');const{match}=await line('host',/^URL: (https:\/\/.+)$/m,host);result.url=match[1];result.services=JSON.parse(fs.readFileSync(out+'/services/hosting.json','utf8'));
for(let i=0;i<2;i++){
 const name='player-'+i,p=child(['bash',root+'/scripts/mp1-play.sh','--join',result.url,'--silent'],name),{match:port,text}=await line(name,/^DEBUG_PORT: (\d+)$/m,p,90000);if(!text.includes('--alsa-output-device=null'))throw Error('missing null sound');const profile=/^PROFILE: (.+)$/m.exec(text)?.[1];if(!profile?.startsWith('/tmp/turts-play-profile.'))throw Error('unexpected profile');result.profiles.push(profile);
 const b=await chromium.connectOverCDP('http://127.0.0.1:'+port[1]);browsers.push(b);const context=b.contexts()[0],page=context.pages().find(p=>p.url().startsWith('https://'));if(!page)throw Error('HTTPS page absent');pages.push(page);page.on('pageerror',e=>result.errors.push(String(e)));
 const opening=await wait(i,s=>s.lobby||modelReady(s),'lobby or game');
 if(opening.lobby){
  if(i===0&&process.env.TURTS_LOBBY_CHECK==='1'){
   result.waiting=opening;await pages[i].screenshot({path:out+'/waiting.png'});await sleep(2000);
   if(!(await read(i)).lobby||modelReady(await read(i)))throw Error('solo human started without a second player');
   await clickWidget(i,'lobby.colour.1');await wait(i,s=>ownSeat(s)?.colour===1,'choose orange');
  }
  await clickWidget(i,'lobby.ready');await wait(i,s=>ownSeat(s)?.ready,'first-click Ready');
  if(i===1&&process.env.TURTS_LOBBY_CHECK==='1'){
   await wait(i,s=>s.lobby?.start?.Countdown,'countdown');await pages[i].screenshot({path:out+'/countdown.png'});
   await clickWidget(i,'lobby.ready');await wait(i,s=>ownSeat(s)?.ready===false&&s.lobby.start==='Waiting','cancel countdown');
   await sleep(3500);result.cancelled=await read(i);if(modelReady(result.cancelled)||result.cancelled.lobby?.start!=='Waiting')throw Error('cancelled countdown started');
   await pages[i].screenshot({path:out+'/cancelled.png'});await clickWidget(i,'lobby.ready');
  }
 }
}
for(let i=0;i<2;i++){
 const context=browsers[i].contexts()[0],page=pages[i];
 const first=await wait(i,s=>s.scene.models.length>0&&s.scene.models.every(m=>m.ready),'shared match start');
 if(process.env.TURTS_REJOIN_CHECK==='1'){
  const id=first.slot?6:3,x=first.slot?46:18,u=first.scene.units.find(u=>u.unit_id===id),[vx,vy,vw,vh]=first.scene.viewport_logical_rect;
  await page.mouse.click(vx+vw/2,vy+vh*.65);await page.mouse.click(...project(first.scene,u.translation));await wait(i,s=>s.selection.includes(id),'selection before reload');await page.mouse.click(...project((await read(i)).scene,[x,0,30]),{button:'right'});
  await wait(i,s=>{const moved=s.world.units.find(u=>u.unit_id===id);return Math.abs(moved.pos[0]-first.world.units.find(u=>u.unit_id===id).pos[0])>.1},'order admitted before reload',5000);
 }

 const session=await context.newCDPSession(page),network={websockets:[],packages:[]};result.networks.push(network);await session.send('Network.enable');session.on('Network.webSocketCreated',e=>network.websockets.push(e.url));session.on('Network.responseReceived',e=>{if(e.response.url.includes('/packages/'))network.packages.push({url:e.response.url,status:e.response.status})});
 const reloadStarted=Date.now();await page.reload({waitUntil:'domcontentloaded'});await wait(i,s=>s.scene.models.length>0&&s.scene.models.every(m=>m.ready),'reloaded public join');network.reload_ms=Date.now()-reloadStarted;if(process.env.TURTS_REJOIN_CHECK==='1'){const id=i?6:3,x=i?46:18;await wait(i,s=>{const u=s.world.units.find(u=>u.unit_id===id);return Math.hypot(u.pos[0]-x,u.pos[1]-30)<.25},'admitted order survives reload',10000);network.admitted_order_preserved=true;}if(process.env.TURTS_REJOIN_CHECK==='1'&&network.websockets.length>2)throw Error('rejoin opened repeated WebSockets: '+network.websockets.length);if(!network.websockets.includes(result.services.ws))throw Error('WSS endpoint not used: '+JSON.stringify(network));if(!network.packages.some(r=>r.url.startsWith(result.services.http+'/packages/')&&r.status===200))throw Error('public package did not load: '+JSON.stringify(network));
}
if(process.env.TURTS_LOBBY_CHECK==='1'){
 result.palette_checks=[];
 const colours=[[230,159,0],[0,114,178]];
 const count=(png,point,colour,radius)=>{let n=0;for(let y=Math.floor(point[1]-radius);y<=point[1]+radius;y++)for(let x=Math.floor(point[0]-radius);x<=point[0]+radius;x++){const p=pixelAt(png,x,y);if(p&&colour.every((v,k)=>Math.abs(v-p[k])<16))n++}return n};
 for(let i=0;i<2;i++){
  let s=await read(i);
  if(!colours.every((rgb,slot)=>rgb.every((value,k)=>Math.abs(s.colours?.[slot]?.[k]*255-value)<1)))throw Error('body materials do not use chosen slot colours');
  const id=i?6:3,u=s.scene.units.find(u=>u.unit_id===id),[vx,vy,vw,vh]=s.scene.viewport_logical_rect;
  await pages[i].mouse.click(vx+vw/2,vy+vh*.65);await pages[i].mouse.click(...project(s.scene,u.translation));await wait(i,s=>s.selection.includes(id),'colour-check selection');await sleep(250);s=await read(i);
  const png=decodePng(await pages[i].screenshot({path:out+'/palette-'+i+'.png'})),rect=s.scene.widgets.find(w=>w.id==='skirmish_minimap').rect;
  const markers=colours.map((rgb,slot)=>count(png,[rect[0]+(slot?54:10)/64*rect[2],rect[1]+(1-24/48)*rect[3]],rgb,5));
  const point=project(s.scene,u.translation);let ring=0;
  // The 3D tone mapper changes RGB values. Check the selected colour family in the world view.
  for(let y=Math.floor(point[1]-48);y<=point[1]+48;y++)for(let x=Math.floor(point[0]-48);x<=point[0]+48;x++){const p=pixelAt(png,x,y);if(p&&(i===0?p[0]>p[2]+12&&p[1]>p[2]+8:p[2]>p[0]+12&&p[2]>p[1]+8))ring++}
  result.palette_checks.push({body_materials:s.colours.slice(0,2),minimap_pixels:markers,selected_ring_pixels:ring});
  if(markers.some(n=>n<4)||ring<4)throw Error('chosen colours missing from minimap or selected ring: '+JSON.stringify(result.palette_checks));
 }
}
result.initial=[await read(0),await read(1)];if(process.env.TURTS_LOBBY_CHECK==='1'&&result.initial.some(s=>JSON.stringify(s.start?.colours)!=='[1,0]'))throw Error('chosen colours did not survive match start and reload');if(result.initial[0].slot!==0||result.initial[1].slot!==1||result.initial[0].match!==result.initial[1].match||result.initial.some(s=>!s.secure))throw Error('secure shared slots or match differ');
for(let i=0;i<2;i++){
 const s=await read(i),id=i?6:3,u=s.scene.units.find(u=>u.unit_id===id),x=i?48:16;const [vx,vy,vw,vh]=s.scene.viewport_logical_rect;await pages[i].mouse.click(vx+vw/2,vy+vh*.65);await pages[i].mouse.click(...project(s.scene,u.translation));await wait(i,s=>s.selection.includes(id),'selection');await pages[i].mouse.click(...project((await read(i)).scene,[x,0,30]),{button:'right'});
 for(let observer=0;observer<2;observer++)await wait(observer,s=>{const u=s.world.units.find(u=>u.unit_id===id);return Math.hypot(u.pos[0]-x,u.pos[1]-30)<.25},'both see public move',15000);
 await pages[i].screenshot({path:out+'/player-'+i+'.png'});
}
result.final=[await read(0),await read(1)];if(process.env.TURTS_REJOIN_CHECK==='1'&&fs.readFileSync(out+'/services/server.log','utf8').includes('refused: This game is open in another window'))throw Error('reload was refused as a second live window');result.status='PASS';console.log('PASS public HTTPS, WSS, two players, and ordinary orders');
}catch(e){result.status='FAIL';result.error=String(e.stack||e);console.error(e);process.exitCode=1;for(let i=0;i<pages.length;i++)try{await pages[i].screenshot({path:out+'/failure-'+i+'.png'})}catch{}}
finally{for(const b of browsers.reverse())await b.close();for(const p of children.reverse())if(p.exitCode===null){const ended=new Promise(r=>p.once('exit',r));try{process.kill(-p.pid,'SIGTERM')}catch{}await Promise.race([ended,sleep(15000)])}result.profiles_removed=result.profiles.every(p=>!fs.existsSync(p));result.services_stopped=(result.services?.owned_pids||[]).every(pid=>{try{process.kill(pid,0);return false}catch(e){return e.code==='ESRCH'}});if(!result.profiles_removed||!result.services_stopped){result.status='FAIL';result.error='owned processes or profiles remain';process.exitCode=1}fs.writeFileSync(out+'/result.json',JSON.stringify(result,null,2))}})();
