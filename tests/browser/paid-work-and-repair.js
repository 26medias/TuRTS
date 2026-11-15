// IO2. Isolated SDK14 package. All game changes use ordinary mouse controls.
const fs = require('node:fs'), net = require('node:net');
const {spawn, spawnSync} = require('node:child_process');
const {chromium} = require('playwright-core');
const [prepared, out, mode] = process.argv.slice(2);
if (!out || (mode && mode !== '--fixture-only')) throw Error('PREPARED OUTPUT [--fixture-only]');
fs.mkdirSync(out, {recursive:true});
const fixture = out + '/paid';
fs.cpSync(prepared + '/pack/skirmish', fixture, {recursive:true});
function qualify(dir) {for(const e of fs.readdirSync(dir,{withFileTypes:true})) {const f=dir+'/'+e.name;if(e.isDirectory())qualify(f);else if(e.name.endsWith('.ron'))fs.writeFileSync(f,fs.readFileSync(f,'utf8').replaceAll('skirmish:','paid:'));}}
qualify(fixture);
const write=(p,s)=>fs.writeFileSync(fixture+'/'+p,s);
const basic=(id,name,fields='',radius=1.5,health=200)=>`(id:"${id}",name:"${name}",footprint:Circle(radius:${radius}),movement:None,inventory:None,gatherer:None,drop_off:None,producer:None,health:Some((max:${health})),removal:Some((by_command:true,contents:Destroy))${fields})`;
write('resources/shell.ron','(id:"shell",name:"Test shell",unit_name:"shell",kind:Item)');
write('units/workshop.ron',`(id:"workshop",name:"Workshop",footprint:Circle(radius:1.5),movement:None,inventory:None,gatherer:None,drop_off:None,
 producer:Some((recipes:["paid:train_trainee"],queue_capacity:2,spawn_offsets:[(x:3.5,y:0.0)])),
 constructor:Some((buildings:["paid:build_wall","paid:build_mint"],concurrent_sites:2,queue_capacity:3)),build_area:Some(24.0),
 health:Some((max:500)),removal:Some((by_command:true,contents:Destroy)))`);
const placement=',placement:Some((gap:1.5,node_clearance:0.0,in_build_area:true,site_health:(numerator:1,denominator:4)))';
write('units/wall.ron',basic('wall','Paid wall',placement));
write('units/mint.ron',basic('mint','Mint',placement+',income:Some((resource:"paid:credits",amount:20,period_ms:1000))'));
write('units/trainee.ron','(id:"trainee",name:"Paid trainee",footprint:Circle(radius:0.5),movement:Some((speed:3.5)),inventory:None,gatherer:None,drop_off:None,producer:None,health:Some((max:60)),removal:Some((by_command:true,contents:Destroy)))');
write('units/engineer.ron','(id:"engineer",name:"Engineer",footprint:Circle(radius:0.5),movement:Some((speed:3.5)),inventory:None,gatherer:None,drop_off:None,producer:None,repairer:Some((rate_per_second:1,reach:0.5,cost:[(resource:"paid:credits",amount:100,per_health:1)],targets:(slots:Same),classes:[],auto_range:None)),health:Some((max:60)),removal:Some((by_command:true,contents:Destroy)))');
write('units/target.ron',basic('target','Damaged workshop'));
write('units/shooter.ron','(id:"shooter",name:"One-shot enemy",footprint:Circle(radius:1.0),movement:None,inventory:Some((capacity:1,accepts:["paid:shell"])),gatherer:None,drop_off:None,producer:None,health:Some((max:200)),acquire:Some((range:9.0,leash:0.0)),weapon:Some((range:9.0,range_mode:TargetSurface,cooldown_ms:1000,damage:20,ammo:Some((item:"paid:shell",per_shot:1)),targets:(slots:Enemy))),removal:Some((by_command:true,contents:Destroy)))');
const refunds='(queued:(numerator:1,denominator:1),running:(numerator:1,denominator:2),output_blocked:(numerator:1,denominator:2),destroyed:(numerator:0,denominator:1))';
for(const [id,name,unit,duration,cost,payment] of [
 ['build_wall','Build Paid wall','wall',12000,120,'ChargeOverTime'],
 ['build_mint','Build Mint','mint',1000,0,'ChargeOnQueue'],
 ['train_trainee','Train Paid trainee','trainee',6000,60,'ChargeOverTime'],
]) write('production/'+id+'.ron',`(id:"${id}",name:"${name}",output_unit:"paid:${unit}",duration_ms:${duration},costs:[${cost ? `(resource:"paid:credits",amount:${cost},source:SlotBalance)` : ""}],payment:${payment},cancel_refunds:${refunds})`);
const visuals={workshop:'barracks',wall:'barracks',mint:'base',trainee:'worker',engineer:'worker',target:'barracks',shooter:'tower'};
for(const[k,v]of Object.entries(visuals))write('visuals/'+k+'.ron',fs.readFileSync(fixture+'/visuals/'+v+'.ron','utf8').replace(`Unit("paid:${v}")`,`Unit("paid:${k}")`));
const all=Object.keys(visuals).map(x=>`"paid:${x}"`).join(',');
function order(id,name,behaviour,target,units=all) {write('orders/'+id+'.ron',`(id:"${id}",name:"${name}",behaviour:${behaviour},target:${target},fire:None,engage:None,icon:"icon_order_produce",hotkey:"fixture_order",cursor:None,card:None,units:[${units}],description:"${name}.")`);}
order('remove','Remove','Remove','None');order('stop','Stop','Stop','None');
order('cancel_site','Cancel site','CancelConstruction','None','"paid:wall","paid:mint"');
order('build','Build','Construct','ContentAtPoint','"paid:workshop"');
order('train','Train','Produce','Content','"paid:workshop"');
order('repair','Repair','Repair','Unit','"paid:engineer"');
write('orders/rules.ron','(queue_depth:16,smart:[(target:Content,needs:Producer,order:"paid:train"),(target:OwnDamaged,needs:Repairer,order:"paid:repair")])');
write('input/bindings.ron','(actions:[(name:"fixture_order",default:[])],engine_overrides:[])');
let manifest=fs.readFileSync(fixture+'/manifest.ron','utf8').replace('id: "skirmish"','id: "paid"').replace(/engine_api:\s*"[^"]+"/,'engine_api: "^0.2.14"');
for(const type of ['units','visuals'])manifest=manifest.replace(type+': [',type+': ['+Object.keys(visuals).map(id=>`"${type}/${id}.ron"`).join(',')+',');
manifest=manifest.replace('resources: [','resources: ["resources/shell.ron",').replace('production: [','production: ["production/build_wall.ron","production/build_mint.ron","production/train_trainee.ron",').replace(/orders:\s*\[[^\]]*\]/,'orders: ["orders/remove.ron","orders/stop.ron","orders/cancel_site.ron","orders/build.ron","orders/train.ron","orders/repair.ron","orders/rules.ron"]');
write('manifest.ron',manifest);
const place=(id,slot,x,y,contents='[]')=>`(unit:"paid:${id}",owner_slot:${slot},pos:(x:${x}.0,y:${y}.0),contents:${contents})`;
write('maps/field.ron',`(id:"field",size:(x:64.0,y:48.0),placements:[${[
 place('workshop',0,18,24),place('engineer',0,18,32),place('target',0,36,24),place('shooter',1,44,24,'[(item:"paid:shell",amount:1)]'),
].join(',')}],nodes:[],slot_economy:[(slot:0,balances:[(balance:"paid:credits",amount:3)]),(slot:1,balances:[(balance:"paid:credits",amount:777)])])`);
write('match.ron','(start:Immediate,victory:None,open_sites_per_slot:Some(4))');
const text=(id,y,value,visible='[]')=>`(id:"${id}",at:Some((anchor:TopLeft,offset:(16.0,${y}.0),pivot:None)),size:(w:Px(294.0),h:Px(26.0)),visible:${visible},widget:Text(text:${value},font:None,size:14,colour:"#ffffff",align:Left,wrap:true))`;
const button=(id,y,name,action,visible='[PrimaryOwned]')=>`(id:"${id}",at:Some((anchor:TopLeft,offset:(16.0,${y}.0),pivot:None)),size:(w:Px(294.0),h:Px(28.0)),visible:${visible},widget:Button(icon:None,label:Some(Literal("${name}")),action:Order("paid:${action}"),hotkey_label:false))`;
const widgets=[
 '(id:"backdrop",at:Some((anchor:TopLeft,offset:(6.0,6.0),pivot:None)),size:(w:Px(310.0),h:Px(690.0)),layer:-1,widget:Box(fill:Some(Colour("#10251c")),border:None,children:[]))',
 text('credits',12,'Format("Credits: {0}",[Balance("paid:credits")])'),text('name',42,'Format("{0}: {1}/{2}",[SelectionPrimary(Name),SelectionPrimary(Health),SelectionPrimary(MaxHealth)])','[SelectionNotEmpty]'),
 text('repair_state',72,'Bind(SelectionPrimary(RepairState))','[PrimaryHasValue(RepairState)]'),
 text('repair_target',102,'Bind(SelectionPrimary(RepairTargetName))','[PrimaryHasValue(RepairTargetName)]'),
 text('repair_resource',132,'Bind(SelectionPrimary(RepairResourceName))','[PrimaryHasValue(RepairResourceName)]'),
 text('repair_wait',162,'Literal("Repair needs funds")','[RepairStateIs(WaitingForFunds)]'),
 button('stop',198,'Stop','stop'),button('cancel',232,'Cancel site','cancel_site'),button('remove',266,'Remove','remove'),
 '(id:"build_menu",at:Some((anchor:TopLeft,offset:(16.0,306.0),pivot:None)),size:(w:Px(294.0),h:Px(100.0)),visible:[PrimaryOwned],widget:List(source:BuildMenu,direction:Vertical,gap:4,max_items:3,item:(id:"build_item",size:(w:Fill,h:Px(28.0)),widget:Button(icon:None,label:Some(Item(Name)),action:Item(Action),hotkey_label:false))))',
 '(id:"work",at:Some((anchor:TopLeft,offset:(16.0,420.0),pivot:None)),size:(w:Px(294.0),h:Px(80.0)),visible:[PrimaryOwned],widget:List(source:ProductionQueue,direction:Vertical,gap:4,max_items:1,item:(id:"work_row",size:(w:Fill,h:Px(72.0)),widget:Text(text:Format("Work: {0}\\n{1}\\nPaid {2}; refund {3}",[Item(Progress),Item(State),Item(Paid),Item(Refund)]),font:None,size:12,colour:"#ffffff",align:Left,wrap:false))))',
 '(id:"sites",at:Some((anchor:TopLeft,offset:(16.0,510.0),pivot:None)),size:(w:Px(294.0),h:Px(120.0)),widget:List(source:Sites,direction:Vertical,gap:4,max_items:2,item:(id:"site_row",size:(w:Fill,h:Px(56.0)),widget:Text(text:Format("Site: {0}\\n{1}\\nPaid {2}; refund {3}",[Item(Progress),Item(State),Item(Paid),Item(Refund)]),font:None,size:12,colour:"#ffffff",align:Left,wrap:false))))',
];
write('interface/hud.ron',`(id:"paid_hud",root:[${widgets.join(',')}],tooltip_style:(background:"#000000",colour:"#ffffff",font:None,size:12),cursors:{},camera:(pitch:0.7,yaw:0.0,field_of_view:60.0,minimum:10.0,maximum:100.0,start:OwnedCentre,height:36.0,margin:0.0,pan_speed:100.0,edge_band:false),light:None,slot_colours:["#0072B2","#E69F00"],world:(ground:"#243b2c",grid:"#354b3c",selection:"#ffffff",health:"#00ff00",range:"#ffffff",show_range:true,tooltip:(background:"#000000",colour:"#ffffff",font:None,size:12),health_bar:(width:32,height:4,background:"#000000",health:"#00ff00")),alerts:[],reasons:{},units:{},selection_priority:[],guide:[])`);
const sleep=ms=>new Promise(r=>setTimeout(r,ms));
const port=()=>new Promise(r=>{const s=net.createServer();s.listen(0,'127.0.0.1',()=>{const p=s.address().port;s.close(()=>r(p));});});
function project(scene,pos){const m=scene.clip_from_world,c=[0,1,2,3].map(r=>[...pos,1].reduce((v,x,i)=>v+m[i*4+r]*x,0));const[x,y,w,h]=scene.viewport_logical_rect;return[x+(c[0]/c[3]+1)*w/2,y+(1-c[1]/c[3])*h/2];}
(async()=>{
 if(mode==='--fixture-only'){console.log(JSON.stringify({status:'WRITTEN_NOT_RUN',fixture}));return;}
 let server,context,page,profile,read;
 const result={status:'RUNNING',fixture,errors:[]};
 const save=()=>fs.writeFileSync(out+'/result.json',JSON.stringify(result,null,2));
 const world=s=>s.debug.replicated_world,units=s=>world(s).units;
 const widget=(s,id)=>s.ui.find(x=>x.visible&&x.id===id);
 const balance=s=>world(s).economy.balances.find(x=>x.resource==='paid:credits').on_hand;
 const health=(s,id)=>world(s).healths.find(x=>x.unit===id)?.current;
 const site=(s,id)=>world(s).progress.sites.find(x=>x.unit===id);
 const waiting=rate=>rate?.state&&typeof rate.state==='object'&&rate.state.WaitingForFunds==='paid:credits';
 try{
  const packed=spawnSync(prepared+'/bin/turts-pack',[fixture],{encoding:'utf8'});fs.writeFileSync(out+'/pack.log',packed.stdout+packed.stderr);if(packed.status!==0)throw Error('SDK14 fixture pack failed');
  const http=await port(),ws=await port(),url=`http://127.0.0.1:${http}`;
  fs.writeFileSync(out+'/server.ron',`(game_package:"${fixture}.tar",http_listen:"127.0.0.1:${http}",ws_listen:"127.0.0.1:${ws}",public_url:"${url}",tick_hz:20,static_dir:"${prepared}/dist")`);
  const fd=fs.openSync(out+'/server.log','w');server=spawn(prepared+'/bin/turts_server',[out+'/server.ron'],{stdio:['ignore',fd,fd],env:{...process.env,TURTS_COMPUTER_SLOT:''}});fs.closeSync(fd);
  for(let i=0;i<300;i++){try{if((await fetch(url+'/advertisement.json')).ok)break;}catch{}if(server.exitCode!==null)throw Error('server stopped');await sleep(50);}
  profile=fs.mkdtempSync('/tmp/turts-paid-');
  context=await chromium.launchPersistentContext(profile,{executablePath:'/usr/bin/google-chrome',headless:false,viewport:{width:1280,height:720},ignoreDefaultArgs:['--enable-unsafe-swiftshader'],args:['--enable-gpu','--alsa-output-device=null','--enable-unsafe-webgpu','--enable-features=Vulkan','--use-angle=vulkan','--disable-vulkan-surface','--no-first-run','--no-default-browser-check']});
  page=context.pages()[0];page.on('pageerror',e=>result.errors.push(String(e)));
  await page.goto(url+'/?server='+encodeURIComponent(url)+'&ws='+encodeURIComponent(`ws://127.0.0.1:${ws}`));
  read=()=>page.evaluate(async()=>typeof turts_scene==='function'&&turts_scene()?{debug:JSON.parse(await turts_debug_request()),scene:JSON.parse(turts_scene()),ui:JSON.parse(turts_ui())}:null);
  const wait=async(test,label,ms=30000)=>{const end=Date.now()+ms;while(Date.now()<end){const s=await read();if(s&&test(s))return s;if(result.errors.length)throw Error(result.errors.join('\n'));await sleep(60);}throw Error(label+' timeout');};
  const select=async id=>{const s=await read(),u=s.scene.units.find(x=>x.unit_id===id);if(!u)throw Error('missing unit '+id);await page.mouse.click(...project(s.scene,u.translation));return wait(s=>s.debug.selection.length===1&&s.debug.selection[0]===id,'select '+id,5000);};
  const click=async label=>{const s=await wait(s=>s.ui.some(x=>x.kind==='button'&&x.visible&&x.enabled&&x.text===label),label,5000);const w=s.ui.find(x=>x.kind==='button'&&x.visible&&x.enabled&&x.text===label);await page.mouse.click(w.rect[0]+w.rect[2]/2,w.rect[1]+w.rect[3]/2);};
  const build=async(label,x,z)=>{await select(1);await click(label);await sleep(120);await page.mouse.click(...project((await read()).scene,[x,0,z]));};
  const shot=async(name,s)=>{result[name]=s;save();await page.screenshot({path:out+'/'+name+'.png'});};
  result.initial=await wait(s=>s.debug.slot===0&&s.scene.models?.length===4&&s.scene.models.every(x=>x.ready)&&health(s,3)===180,'one ordinary enemy shot');
  if(balance(result.initial)!==3||widget(result.initial,'credits')?.text!=='Credits: 3')throw Error('initial displayed balance mismatch');
  await page.mouse.click(640,360);
  await select(3);
  await shot('ordinary-damage',await wait(s=>widget(s,'name')?.text==='Damaged workshop: 180/200','visible ordinary damage'));
  await build('Build Paid wall',24,14);
  const first=await wait(s=>world(s).progress.sites.some(x=>waiting(x.rate)),'paid site funding stall');
  const wallId=world(first).progress.sites.find(x=>waiting(x.rate)).unit;
  await shot('site-stalled',first);
  if(balance(first)!==0 || widget(first,'credits')?.text!=='Credits: 0' || !first.ui.some(x=>x.visible&&x.id.includes('site_row')&&x.text.includes('Waiting for funds: Credits'))) throw Error('paid-site funding display mismatch');
  const stalledValue=site(first,wallId).value,stalledHealth=health(first,wallId),stalledBalance=balance(first),tick=world(first).tick;
  const unchanged=await wait(s=>world(s).tick>=tick+25,'stalled interval');
  if(site(unchanged,wallId).value!==stalledValue||health(unchanged,wallId)!==stalledHealth||balance(unchanged)!==stalledBalance)throw Error('unfunded site mutated');
  await build('Build Paid wall',16,14);
  const second=await wait(s=>world(s).progress.sites.length===2,'second active site');
  const secondId=world(second).progress.sites.find(x=>x.unit!==wallId).unit;
  await build('Build Paid wall',32,14);
  await shot('held-request',await wait(s=>world(s).progress.construction_queues.some(x=>x.constructor===1&&x.orders.length===1),'held construction'));
  await select(1);await click('Stop');
  await shot('held-stopped',await wait(s=>!world(s).progress.construction_queues.some(x=>x.constructor===1),'Stop clears held requests'));
  if(world(result['held-stopped']).progress.sites.length!==2)throw Error('Stop cancelled placed sites');
  await select(secondId);await click('Cancel site');await wait(s=>!units(s).some(x=>x.unit_id===secondId),'cancel second site');
  await build('Build Mint',24,34);
  const resumed=await wait(s=>site(s,wallId)?.value>stalledValue,'income resumes paid site');
  await shot('site-resumed',resumed);
  if(!site(resumed,wallId).rate.held)throw Error('expected advancing hold hysteresis');
  await select(wallId);await click('Cancel site');
  await shot('site-cancelled',await wait(s=>!units(s).some(x=>x.unit_id===wallId),'cancel paid site'));
  await select(1);await click('Train Paid trainee');
  await shot('paid-unit-running',await wait(s=>world(s).progress.jobs.some(x=>x.producer===1&&x.value>0),'paid production advances'));
  await shot('paid-unit-complete',await wait(s=>units(s).some(x=>x.content_id==='paid:trainee'),'paid production completes',30000));
  await select(2);const before=await read();await page.mouse.click(...project(before.scene,before.scene.units.find(x=>x.unit_id===3).translation),{button:'right'});
  await shot('repair-approach',await wait(s=>widget(s,'repair_state')?.text==='approaching','repair approaches'));
  if(widget(result['repair-approach'],'repair_target')?.text!=='Damaged workshop')throw Error('repair target name');
  await shot('repair-funds',await wait(s=>widget(s,'repair_state')?.text==='waiting_for_funds'&&widget(s,'repair_wait')&&world(s).repairs.some(x=>x.repairer===2&&x.state.WaitingForFunds==='paid:credits'),'repair real funding wait',15000));
  if(widget(result['repair-funds'],'repair_resource')?.text!=='Credits')throw Error('repair resource name');
  const damage=health(result['repair-funds'],3);
  await shot('repair-healed',await wait(s=>health(s,3)>damage,'income pays real repair',15000));
  await select(2);await click('Stop');
  await shot('repair-stopped',await wait(s=>!world(s).repairs.some(x=>x.repairer===2)&&!widget(s,'repair_state')&&!widget(s,'repair_target')&&!widget(s,'repair_resource'),'repair status cleared'));
  await select(4);const foreign=await read();if(widget(foreign,'repair_state')||widget(foreign,'repair_target')||widget(foreign,'repair_resource'))throw Error('foreign selection leaked repair fields');
  if(widget(foreign,'credits')?.text!==`Credits: ${balance(foreign)}`)throw Error('displayed credits mismatch');
  result.status='PASS';console.log(JSON.stringify({status:result.status,wallId,damage,healed:health(result['repair-healed'],3)}));
 }catch(e){result.status='FAIL';result.error=String(e.stack||e);if(read)try{result.failure=await read();}catch{}if(page)try{await page.screenshot({path:out+'/failure.png'});}catch{}console.error(e);process.exitCode=1;}
 finally{if(context)await context.close();if(server&&server.exitCode===null){const end=new Promise(r=>server.once('exit',r));server.kill('SIGTERM');await end;}if(profile)fs.rmSync(profile,{recursive:true,force:true});result.profile_removed=!profile||!fs.existsSync(profile);save();}
})();
