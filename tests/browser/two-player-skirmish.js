// Two fresh browsers play one unchanged package through the host and join launchers.
const fs=require('node:fs'),{spawn}=require('node:child_process'),{chromium}=require('playwright-core');
const [root,prepared,out]=process.argv.slice(2);if(!out)throw Error('ROOT PREPARED OUTPUT');fs.mkdirSync(out,{recursive:true});
const sleep=ms=>new Promise(r=>setTimeout(r,ms)),result={status:'RUNNING',actions:[],errors:[],profiles:[]},children=[],browsers=[],pages=[];
function project(s,v){const m=s.clip_from_world,c=[0,1,2,3].map(r=>[...v,1].reduce((a,x,k)=>a+m[k*4+r]*x,0)),[x,y,w,h]=s.viewport_logical_rect;return[x+(c[0]/c[3]+1)*w/2,y+(1-c[1]/c[3])*h/2]}
const hp=(s,id)=>s.world.healths.find(h=>h.unit===id)?.current;
const outcome=s=>s.ui.find(w=>w.id==='result_title'&&w.visible&&['Victory','Defeat','Draw'].includes(w.text))?.text;
async function launch(args,name){const log=out+'/'+name+'.log',fd=fs.openSync(log,'w'),child=spawn('bash',args,{detached:true,stdio:['ignore',fd,fd]});fs.closeSync(fd);children.push(child);let text,port;
for(let i=0;i<600;i++){text=fs.readFileSync(log,'utf8');port=/DEBUG_PORT: (\d+)/.exec(text)?.[1];if(port)break;if(child.exitCode!==null)throw Error(name+' exited '+child.exitCode);await sleep(100)}
if(!port||!text.includes('--alsa-output-device=null'))throw Error(name+' missing silent Chrome');const profile=/^PROFILE: (.+)$/m.exec(text)?.[1];if(!profile?.startsWith('/tmp/turts-play-profile.'))throw Error('unexpected profile');result.profiles.push(profile);
const browser=await chromium.connectOverCDP('http://127.0.0.1:'+port);browsers.push(browser);const page=browser.contexts()[0].pages().find(p=>p.url().startsWith('http://127.0.0.1:'));if(!page)throw Error('page missing');pages.push(page);page.on('pageerror',e=>result.errors.push(String(e)));return text}
async function read(i){return pages[i].evaluate(async()=>{if(typeof turts_debug_request!=='function'||typeof turts_ui!=='function')return null;const value=await turts_debug_request();if(!value)return null;const d=JSON.parse(value),scene=typeof turts_scene==='function'?turts_scene():null;return{lobby:d.lobby,scene:scene?JSON.parse(scene):{models:[],units:[],widgets:[]},ui:JSON.parse(turts_ui()),world:d.replicated_world||{units:[]},slot:d.slot,match:d.match_id_decimal,selection:d.selection}})}
async function wait(i,fn,label,ms=30000){const end=Date.now()+ms;while(Date.now()<end){const s=await read(i);if(s?.world?.units&&fn(s))return s;if(result.errors.length)throw Error(result.errors.join('\n'));await sleep(150)}throw Error(label+' timeout')}
async function click(i,p,label,button='left'){result.actions.push({player:i,label,point:p,button});await pages[i].mouse.click(...p,{button});await sleep(150)}
async function look(i,x,y){const s=await read(i),r=s.scene.widgets.find(w=>w.id==='skirmish_minimap').rect;await click(i,[r[0]+x/64*r[2],r[1]+(1-y/48)*r[3]],'look on map');await sleep(200)}
async function unit(i,id,button='left'){const s=await read(i),u=s.scene.units.find(u=>u.unit_id===id);if(!u)return false;const p=project(s.scene,u.translation),[x,y,w,h]=s.scene.viewport_logical_rect;if(p[0]<x+5||p[0]>x+w-5||p[1]<y+5||p[1]>y+h-5){await look(i,u.translation[0],u.translation[2]);return false}await click(i,p,'unit '+id,button);return true}
async function select(i,id){for(let n=0;n<5;n++){await unit(i,id);const s=await read(i);if(s.selection.includes(id))return s}throw Error('selection failed '+id)}
async function world(i,x,y,button='left'){await click(i,project((await read(i)).scene,[x,0,y]),'world '+x+','+y,button)}
async function button(i,text){const s=await wait(i,s=>s.ui.some(w=>w.kind==='button'&&w.visible&&w.enabled&&w.text?.includes(text)),'button '+text,60000),w=s.ui.find(w=>w.kind==='button'&&w.visible&&w.enabled&&w.text?.includes(text));await click(i,[w.rect[0]+w.rect[2]/2,w.rect[1]+w.rect[3]/2],text)}
(async()=>{try{
const host=await launch([root+'/scripts/skirmish-play.sh','--prepared',prepared,'--human','--silent'],'host'),url=/^JOIN_URL: (.+)$/m.exec(host)?.[1];if(!url||!host.includes('JOIN_COMMAND:'))throw Error('join instructions absent');result.url=url;
await launch([root+'/scripts/mp1-play.sh','--url',url,'--silent'],'join');
for(let i=0;i<2;i++){
 const opening=await wait(i,s=>s.lobby||s.scene.models.length>0,'lobby or game',90000);
 if(opening.lobby){const w=opening.ui.find(w=>w.id==='lobby.ready'&&w.visible&&w.enabled);if(!w)throw Error('Ready button absent');await pages[i].mouse.click(w.rect[0]+w.rect[2]/2,w.rect[1]+w.rect[3]/2);await wait(i,s=>s.lobby?.seats.find(seat=>seat.index===s.lobby.you)?.ready||s.scene.models.length>0,'Ready accepted');}
}
const ready=s=>s.scene.models.length>0&&s.scene.models.every(m=>m.ready)&&s.world.units.length===6;
result.initial=[await wait(0,ready,'host ready',90000),await wait(1,ready,'join ready',90000)];
if(result.initial[0].slot!==0||result.initial[1].slot!==1||result.initial[0].match!==result.initial[1].match)throw Error('slots or match differ');if(result.profiles[0]===result.profiles[1])throw Error('profiles shared');
for(let i=0;i<2;i++){await pages[i].screenshot({path:out+'/ready-'+i+'.png'});const soldier=i?6:3,x=i?48:16;await select(i,soldier);await world(i,x,30,'right');for(let observer=0;observer<2;observer++)await wait(observer,s=>{const u=s.world.units.find(u=>u.unit_id===soldier);return Math.hypot(u.pos[0]-x,u.pos[1]-30)<.25},'both observe move');}
result.moves=[await read(0),await read(1)];
// An enemy castle may be selected for inspection, but must not expose build orders.
await look(1,10,24);await unit(1,1);const enemy=await read(1);result.enemy_selection=enemy;if(enemy.ui.some(w=>w.kind==='button'&&w.visible&&w.enabled&&w.text?.includes('Build Barracks')))throw Error('enemy castle exposes build');
await look(1,54,24);await select(1,5);const s=await read(1),tree=s.scene.nodes.find(n=>n.node_id===2);await click(1,tree.ground_screen_center,'harvest tree 2','right');await wait(1,s=>s.world.economy.orders.some(o=>o.unit===5&&o.stage!=='Idle'),'harvest');
await select(1,4);await button(1,'Build Barracks');await world(1,47,24);let army=await wait(1,s=>s.world.units.some(u=>u.owner_slot===1&&u.content_id==='skirmish:barracks'&&hp(s,u.unit_id)===500),'Barracks');const barracks=army.world.units.find(u=>u.owner_slot===1&&u.content_id==='skirmish:barracks').unit_id;
await select(1,barracks);await world(1,42,26,'right');await select(1,6);await world(1,42,26,'right');await select(1,barracks);
for(const name of ['Swordsman','Crossbowman','Swordsman','Crossbowman','Crossbowman'])await button(1,'Train '+name);
const fighters=s=>s.world.units.filter(u=>u.owner_slot===1&&['skirmish:soldier','skirmish:swordsman'].includes(u.content_id));
army=await wait(1,s=>fighters(s).length===6,'mixed army');await wait(0,s=>fighters(s).length===6,'host sees army');await sleep(2000);army=await read(1);result.army=army;
const credits=army.world.economy.balances.find(b=>b.resource==='skirmish:credits').on_hand;result.harvest_credits=credits+380-200;if(result.harvest_credits<180)throw Error('army not funded by harvest');
const ids=fighters(army).map(u=>u.unit_id),points=army.scene.units.filter(u=>ids.includes(u.unit_id)).map(u=>u.ground_screen_center),box=[Math.min(...points.map(p=>p[0]))-20,Math.min(...points.map(p=>p[1]))-20,Math.max(...points.map(p=>p[0]))+20,Math.max(...points.map(p=>p[1]))+20];
await pages[1].mouse.move(box[0],box[1]);await pages[1].mouse.down();await pages[1].mouse.move(box[2],box[3],{steps:12});await sleep(100);await pages[1].mouse.up();await sleep(200);const selected=(await read(1)).selection;if(selected.length!==6||ids.some(id=>!selected.includes(id)))throw Error('army selection '+selected);
await pages[1].screenshot({path:out+'/army.png'});await look(1,10,24);const end=Date.now()+180000;let target=null,last=0;
while(Date.now()<end){const s=await read(1);if(outcome(s))break;const enemy=s.world.units.filter(u=>u.owner_slot===0&&['skirmish:soldier','skirmish:swordsman'].includes(u.content_id)).sort((a,b)=>hp(s,a.unit_id)-hp(s,b.unit_id))[0]||s.world.units.find(u=>u.unit_id===1);if(!enemy)break;if(target!==enemy.unit_id||Date.now()-last>3000){target=enemy.unit_id;if(!await unit(1,target,'right')){target=null;continue}last=Date.now()}await sleep(400)}
result.terminal=[await wait(0,s=>outcome(s)==='Defeat','host Defeat'),await wait(1,s=>outcome(s)==='Victory','guest Victory')];for(let i=0;i<2;i++){const s=result.terminal[i];if(s.world.units.some(u=>u.unit_id===1)||hp(s,4)!==1000)throw Error('castle result inconsistent');await pages[i].screenshot({path:out+'/terminal-'+i+'.png'})}result.status='PASS';console.log('PASS two-player match');
}catch(e){result.status='FAIL';result.error=String(e.stack||e);console.error(e);process.exitCode=1;for(let i=0;i<pages.length;i++)try{await pages[i].screenshot({path:out+'/failure-'+i+'.png'})}catch{}}
finally{for(const b of browsers.reverse())await b.close();for(const child of children.reverse())if(child.exitCode===null){const ended=new Promise(r=>child.once('exit',r));try{process.kill(-child.pid,'SIGTERM')}catch{}await Promise.race([ended,sleep(8000)])}result.profiles_removed=result.profiles.every(p=>!fs.existsSync(p));if(!result.profiles_removed){result.status='FAIL';result.error='profiles remain';process.exitCode=1}fs.writeFileSync(out+'/result.json',JSON.stringify(result,null,2))}})();
