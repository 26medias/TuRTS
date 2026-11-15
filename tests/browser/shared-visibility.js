// SV6: real pointer/key scouting on any public fog fixture. No debug mutation or command injection.
// node tests/browser/shared-visibility.js URL SCENARIO.json OUTPUT [CHROME]
// Scenario: {scout_content, enemy_content, observe:[x,y], withdraw:[x,y], revisit:[x,y],
// enemy_destination:[x,y], ground_samples:{observed:[x,y],remembered:[x,y],unexplored:[x,y]}}
'use strict';
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const {spawn} = require('node:child_process');
const {chromium} = require('playwright-core');
const {decodePng} = require('./harness/png');
const [url, scenarioFile, output, executablePath = '/usr/bin/google-chrome'] = process.argv.slice(2);
if (!output) throw Error('Usage: shared-visibility.js URL SCENARIO.json OUTPUT [CHROME]');
const scenario = JSON.parse(fs.readFileSync(scenarioFile));
fs.mkdirSync(output, {recursive: true});
const sleep = ms => new Promise(resolve => setTimeout(resolve, ms));
const assert = (condition, message) => { if (!condition) throw Error(message); };
function project(scene, point) {
    const matrix = scene.clip_from_world;
    const vector = [point[0], 0, point[1], 1];
    const clip = [0,1,2,3].map(row => vector.reduce((sum, value, column) => sum + matrix[column*4+row]*value, 0));
    const [x,y,width,height] = scene.viewport_logical_rect;
    return [x+(clip[0]/clip[3]+1)*width/2, y+(1-clip[1]/clip[3])*height/2];
}
function level(knowledge, point) {
    const grid = knowledge.grid;
    const x = Math.min(Math.floor(point[0]/grid.cell_size), grid.columns-1);
    const y = Math.min(Math.floor(point[1]/grid.cell_size), grid.rows-1);
    const cell = y*grid.columns+x;
    return knowledge.observed[cell>>3] & (1<<(cell%8)) ? 2 : knowledge.explored[cell>>3] & (1<<(cell%8)) ? 1 : 0;
}
function luminance(image, point) {
    let total = 0, count = 0;
    for (let y=Math.round(point[1])-2;y<=Math.round(point[1])+2;y++) {
        for (let x=Math.round(point[0])-2;x<=Math.round(point[0])+2;x++) {
            if (x<0||y<0||x>=image.width||y>=image.height) continue;
            const offset=(y*image.width+x)*4;
            total += image.rgba[offset]*.2126+image.rgba[offset+1]*.7152+image.rgba[offset+2]*.0722;
            count++;
        }
    }
    assert(count>0,'Fog sample is outside screenshot');
    return total/count;
}
(async () => {
    const result = {status:'RUNNING',errors:[],actions:[],states:{}};
    const contexts=[], profiles=[];
    let xvfb;
    try {
        // Xvfb chooses an unused display. This process owns and stops only this display.
        xvfb=spawn('Xvfb',['-displayfd','3','-screen','0','1280x720x24','-nolisten','tcp'],{stdio:['ignore','ignore','pipe','pipe']});
        const display=await new Promise((resolve,reject)=>{
            let text='';
            xvfb.stdio[3].on('data',data=>{text+=data;if(text.includes('\n'))resolve(':'+text.trim());});
            xvfb.once('error',reject);xvfb.once('exit',code=>reject(Error('Xvfb exited '+code)));
        });
        result.display=display;
        for (let slot=0;slot<2;slot++) {
            const profile=fs.mkdtempSync(path.join(os.tmpdir(),'turts-sv6-')); profiles.push(profile);
            const context=await chromium.launchPersistentContext(profile,{
                executablePath,headless:false,viewport:{width:1280,height:720},env:{...process.env,DISPLAY:display},
                args:['--disable-background-timer-throttling','--disable-renderer-backgrounding','--disable-backgrounding-occluded-windows','--alsa-output-device=null','--enable-unsafe-webgpu','--enable-features=Vulkan','--use-angle=vulkan','--disable-vulkan-surface','--no-first-run','--no-default-browser-check'],
            });
            contexts.push(context);
            const page=context.pages()[0];
            page.on('pageerror',error=>result.errors.push(String(error)));
            await page.goto(url);
            await page.keyboard.press('F9');
        }
        const pages=contexts.map(context=>context.pages()[0]);
        const read=page=>page.evaluate(async()=>{
            if(typeof window.turts_debug_request!=='function'||typeof window.turts_scene!=='function')return null;
            const raw=await window.turts_debug_request();if(!raw||!window.turts_scene())return null;
            return {debug:JSON.parse(raw),scene:JSON.parse(window.turts_scene()),ui:typeof window.turts_ui==='function'?JSON.parse(window.turts_ui()):null};
        });
        const wait=async(page,predicate,message)=>{
            const deadline=Date.now()+90000;
            while(Date.now()<deadline){const state=await read(page);if(state&&predicate(state))return state;await sleep(100);}
            throw Error(message);
        };
        let a=await wait(pages[0],s=>s.debug.knowledge&&s.debug.replicated_world?.units?.length,'Slot A did not install fog state');
        let b=await wait(pages[1],s=>s.debug.knowledge&&s.debug.replicated_world?.units?.length,'Slot B did not install fog state');
        assert(a.debug.slot!==b.debug.slot,'Browsers joined the same slot');
        assert(JSON.stringify(a.debug.knowledge.observed)!==JSON.stringify(b.debug.knowledge.observed),'Slots did not receive different coverage');
        const own=(state,content)=>state.debug.replicated_world.units.find(row=>row.owner_slot===state.debug.slot&&row.content_id===content);
        const scout=own(a,scenario.scout_content),enemy=own(b,scenario.enemy_content);
        assert(scout&&enemy,'Scenario content does not identify owned scout and enemy');
        const scoutId=scout.unit_id, enemyId=enemy.unit_id;
        const click=async(page,point,button='left')=>{result.actions.push({kind:'click',point,button});await page.mouse.click(point[0],point[1],{button});};
        const move=async(page,unitId,destination)=>{
            const state=await read(page),unit=state.debug.replicated_world.units.find(row=>row.unit_id===unitId);
            assert(unit,'Owned command unit is missing');
            await page.bringToFront();await page.keyboard.press('F9');
            await click(page,project(state.scene,unit.pos));
            await click(page,project((await read(page)).scene,destination),'right');
        };
        const capture=async(name,page,state)=>{
            result.states[name]=state;
            const bytes=await page.screenshot({path:path.join(output,name+'.png')});
            return decodePng(bytes);
        };
        await capture('00-slot-a',pages[0],a);await capture('00-slot-b',pages[1],b);
        await move(pages[0],scoutId,scenario.observe);
        a=await wait(pages[0],s=>s.debug.replicated_world.units.some(row=>row.unit_id===enemyId),'Scout did not observe enemy');
        await capture('01-observed',pages[0],a);
        const oldEnemy=a.debug.replicated_world.units.find(row=>row.unit_id===enemyId);
        await move(pages[0],scoutId,scenario.withdraw);
        a=await wait(pages[0],s=>{
            const unit=s.debug.replicated_world.units.find(row=>row.unit_id===scoutId);
            return unit&&Math.hypot(unit.pos[0]-scenario.withdraw[0],unit.pos[1]-scenario.withdraw[1])<1
                &&s.debug.knowledge.remembered_units.some(row=>row.unit===enemyId);
        },'Withdraw did not finish with a stale sighting');
        const stale=a.debug.knowledge.remembered_units.find(row=>row.unit===enemyId);
        assert(!a.debug.units.some(row=>row.unit_id===enemyId),'Hidden live interpolation survived');
        assert(!a.debug.replicated_world.healths.some(row=>row.unit===enemyId),'Hidden health survived');
        let hidden=await capture('02-remembered',pages[0],a);
        await click(pages[0],project(a.scene,oldEnemy.pos));
        a=await read(pages[0]);
        assert(!a.debug.selection.includes(enemyId),'Stale sighting remained a command selection target');
        await move(pages[1],enemyId,scenario.enemy_destination);
        b=await wait(pages[1],s=>{const u=s.debug.replicated_world.units.find(row=>row.unit_id===enemyId);return u&&Math.hypot(u.pos[0]-scenario.enemy_destination[0],u.pos[1]-scenario.enemy_destination[1])<1;},'Enemy did not move while hidden');
        a=await read(pages[0]);
        assert(JSON.stringify(a.debug.knowledge.remembered_units.find(row=>row.unit===enemyId))===JSON.stringify(stale),'Hidden movement changed stale public facts');
        hidden=await capture('02-hidden-movement',pages[0],a);
        if(scenario.ground_samples){
            const levels={};
            for(const [name,point] of Object.entries(scenario.ground_samples)){
                assert(level(a.debug.knowledge,point)==={unexplored:0,remembered:1,observed:2}[name],'Ground sample has wrong authoritative state: '+name);
                levels[name]=luminance(hidden,project(a.scene,point));
            }
            const minimap=a.scene.widgets?.find(widget=>widget.id===scenario.minimap_widget);
            assert(minimap,'Scenario must identify its authored minimap_widget');
            const mapLevels={};
            for(const [name,point] of Object.entries(scenario.ground_samples)){
                const [x,y,width,height]=minimap.rect;
                const grid=a.debug.knowledge.grid;
                mapLevels[name]=luminance(hidden,[x+point[0]/grid.size.x*width,y+(1-point[1]/grid.size.y)*height]);
            }
            result.minimap_luminance=mapLevels;
            assert(mapLevels.observed>mapLevels.remembered+3&&mapLevels.remembered>mapLevels.unexplored+2,'Screenshot does not distinguish matching minimap fog states');
            result.fog_luminance=levels;
            assert(levels.observed>levels.remembered+3&&levels.remembered>levels.unexplored+2,'Screenshot does not distinguish three ground fog states');
        }
        await move(pages[0],scoutId,scenario.revisit);
        a=await wait(pages[0],s=>level(s.debug.knowledge,stale.position? [stale.position.x,stale.position.y]:oldEnemy.pos)===2&&!s.debug.knowledge.remembered_units.some(row=>row.unit===enemyId),'Old empty location did not clear stale sighting');
        await capture('03-old-empty',pages[0],a);
        assert(result.errors.length===0,'Browser reported errors');
        result.status='PASS';console.log('PASS SV6: two slots, scout observe/hide, stale picking, hidden movement, old empty location');
    } catch(error) {result.status='FAIL';result.error=String(error.stack||error);process.exitCode=1;console.error(error);}
    finally {
        for(const context of contexts)await context.close();
        for(const profile of profiles)fs.rmSync(profile,{recursive:true,force:true});
        if(xvfb&&xvfb.exitCode===null)xvfb.kill('SIGTERM');
        result.profiles_removed=profiles.every(profile=>!fs.existsSync(profile));
        fs.writeFileSync(path.join(output,'result.json'),JSON.stringify(result,null,2));
    }
})();
