// turts-test-role: instrument
// spade-check.js PORT PACKAGE_ID OUT_DIR : the Spade as a person uses it (operator, 2026-11-26): select the units, click
// the tool button, drag over the ground, release. No other click first. Pointer and keyboard input only; the bridges
// are read to find the button and to judge. Screenshots of each stage for a person to look at.
'use strict';
const fs=require('fs'),path=require('path');
const [port,packageId,out]=process.argv.slice(2);
const {chromium}=require('playwright-core');
const {activatePage,project,read}=require('./driver');
const sleep=ms=>new Promise(r=>setTimeout(r,ms));
const HINT='Drag on the ground to choose the area. Right-click or Esc cancels.';
const result={package:packageId,steps:[],status:'FAIL',errors:[]};
const save=()=>fs.writeFileSync(path.join(out,'result.json'),JSON.stringify(result,null,2));
const step=(name,ok,detail)=>{result.steps.push({name,ok,detail});save();console.log((ok?'PASS ':'FAIL ')+name+' '+JSON.stringify(detail??''));if(!ok)throw Error('Step failed: '+name);};
(async()=>{
 let browser;
 try{
    fs.mkdirSync(out,{recursive:true});
    browser=await chromium.connectOverCDP('http://127.0.0.1:'+port);
    const page=browser.contexts().flatMap(c=>c.pages()).find(p=>/^http:\/\/127\.0\.0\.1/.test(p.url()));
    page.on('pageerror',e=>result.errors.push(String(e)));
    await page.setViewportSize({width:1280,height:720});await activatePage(page);
    const world=s=>s.debug.replicated_world,shot=n=>page.screenshot({path:path.join(out,n+'.png')});
    const texts=s=>s.ui.filter(w=>w.visible&&w.text).map(w=>w.text);
    const hint=s=>texts(s).some(t=>t.includes(HINT));
    let s,end=Date.now()+90000,f9=false;
    while(Date.now()<end){s=await read(page);if(s?.debug.replicated_world?.terrain)break;if(!f9&&Date.now()>end-80000){await page.keyboard.press('F9');f9=true;}await sleep(200);}
    step('the game shows terrain',!!s?.debug.replicated_world?.terrain,{units:world(s)?.units.length});
    await shot('0-start');
    // Select all own units with one drag box.
    const own=new Set(world(s).units.filter(u=>u.owner_slot===s.debug.slot).map(u=>u.unit_id));
    const c=s.scene.units.filter(u=>own.has(u.unit_id)).map(u=>u.ground_screen_center);
    await page.mouse.move(Math.min(...c.map(p=>p[0]))-30,Math.min(...c.map(p=>p[1]))-40);await page.mouse.down();await sleep(80);
    await page.mouse.move(Math.max(...c.map(p=>p[0]))+30,Math.max(...c.map(p=>p[1]))+30,{steps:8});await sleep(80);await page.mouse.up();await sleep(400);
    s=await read(page);step('a drag box selects the units',s.debug.selection.length===own.size,{selected:s.debug.selection.length});
    const toolButton=s=>s.ui.find(w=>w.visible&&w.enabled&&w.widget_action?.kind==='terrain_work'&&/:(spade|scraper)$/.test(w.widget_action.tool));
    const clickTool=async()=>{const b=toolButton(await read(page));const [x,y,w,h]=b.rect;await page.mouse.click(x+w/2,y+h/2,{delay:80});await sleep(500);return b;};
    const button=await clickTool();
    s=await read(page);
    const [bx,by,bw,bh]=button.rect;
    step('after the tool button: the hint line shows and the cursor is the target cursor',hint(s)&&s.debug.cursor?.context==='target',{cursor:s.debug.cursor,hint:hint(s),button:button.text,rect:button.rect});
    const ground=(x,y,h)=>project(s.scene,[x,y,h]);
    // Hover the start point on the flat ground at the foot of the ridge.
    await page.mouse.move(...ground(29.5,20.5,0));await sleep(300);await shot('1-armed');
    result.button_rect=button.rect;
    // Drag from the foot of the ridge across it, then release.
    const from=ground(29.5,20.5,0),to=ground(33.5,27.5,1);
    await page.mouse.down();await sleep(150);await page.mouse.move(...to,{steps:10});await sleep(500);
    await shot('2-drag');s=await read(page);
    step('during the drag the hint is still there',hint(s),{});
    await page.mouse.up();await sleep(700);
    s=await read(page);await shot('3-released');
    // The order reaches the replica later on a loaded host (3 s at load 13, 2026-11-26).
    const work=()=>world(s).unit_orders.map(o=>o.current?.target?.TerrainWork).filter(Boolean);
    for(const limit=Date.now()+10000;!work().length&&Date.now()<limit;){await sleep(200);s=await read(page);}
    const w=work()[0];
    step('release orders the dragged area at the height where the drag started',!!w&&w.target_elevation===0&&w.cells.min.x===29&&w.cells.min.y===20&&w.cells.max.x===34&&w.cells.max.y===28,{work:w,workers:work().length});
    step('after the order the hint is gone and the cursor is not the target cursor',!hint(s)&&s.debug.cursor?.context!=='target',{cursor:s.debug.cursor,texts:texts(s).filter(t=>/Drag/.test(t))});
    const before=new Map(world(s).units.map(u=>[u.unit_id,u.pos]));
    await sleep(6000);s=await read(page);await shot('4-walking');
    const walked=world(s).units.filter(u=>Math.hypot(u.pos[0]-before.get(u.unit_id)[0],u.pos[1]-before.get(u.unit_id)[1])>1).length;
    step('a digger walks to the area',walked>0,{walked});
    const open=x=>{for(let cy=20;cy<28;cy++)for(let cx=30;cx<34;cx++){const e=world(x).terrain.surfaces.find(r=>r.cell.x===cx&&r.cell.y===cy)?.elevation;if(e===undefined?!(cy>=22&&cy<25):e!==0)return false;}return true;};
    end=Date.now()+300000;while(Date.now()<end){s=await read(page);if(s&&open(s))break;await sleep(1000);}
    await shot('5-open');step('the ridge area is dug down to the start height',open(s),{tick:world(s).tick});
    // A refused area: a drag that starts on the ridge top.
    await clickTool();s=await read(page);
    await page.mouse.move(...ground(31.5,10.5,1));await page.mouse.down();await sleep(150);await page.mouse.move(...ground(29.5,12.5,0),{steps:8});await sleep(400);
    await shot('6-refused-drag');await page.mouse.up();await sleep(700);s=await read(page);await shot('7-refused');
    step('a refused area says why in text and keeps the tool armed',texts(s).some(t=>t.includes('Nothing to dig above the height where you started'))&&hint(s)&&s.debug.cursor?.context==='target',{texts:texts(s)});
    await page.mouse.click(...ground(27.5,10.5,0),{button:'right',delay:80});await sleep(500);s=await read(page);
    step('a right-click leaves the tool',!hint(s)&&s.debug.cursor?.context!=='target',{cursor:s.debug.cursor});
    await clickTool();await page.keyboard.press('Escape');await sleep(500);s=await read(page);await shot('8-after-escape');
    step('Esc leaves the tool',!hint(s)&&s.debug.cursor?.context!=='target',{cursor:s.debug.cursor});
    step('no page error',result.errors.length===0,result.errors);
    result.status='PASS';
 }catch(error){result.error=String(error.stack||error);console.error(error);process.exitCode=1;}
 finally{save();if(browser)await browser.close().catch(()=>{});}
})();
