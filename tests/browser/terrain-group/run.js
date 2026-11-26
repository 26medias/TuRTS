#!/usr/bin/env node
// Prepared-bundle-only M4b functional browser campaign. This file builds nothing.
'use strict';
const fs=require('node:fs'),os=require('node:os'),path=require('node:path'),net=require('node:net'),crypto=require('node:crypto');
const {spawn,spawnSync}=require('node:child_process'),{chromium}=require('playwright-core');
const {drive,read}=require('./driver');
const root=path.resolve(__dirname,'../../..'),args=process.argv.slice(2),prepared=path.resolve(args.shift()||''),output=path.resolve(args.shift()||'');
const scenes=['GM1-wide','GM2-narrow','GM3-opposing-large','GM3-settled-blocker','GM3-settled-blocker-hold','GM3-movable-blocker','GM3-shared-and-unique','GM4-edit-during-approach'];
const options={packages:['quarry','terrain_stockyard'],scenes,allocation:false,members:false,recordAll:false,connections:'both',chrome:'/usr/bin/google-chrome'};
while(args.length){const flag=args.shift();if(flag==='--record-all')options.recordAll=true;else if(flag==='--package')options.packages=[args.shift()];else if(flag==='--scene')options.scenes=[args.shift()];else if(flag==='--reverse-allocation')options.allocation=true;else if(flag==='--reverse-members')options.members=true;else if(flag==='--connections')options.connections=args.shift();else if(flag==='--chrome')options.chrome=args.shift();else throw Error('Unknown argument '+flag);}
const assert=(ok,message)=>{if(!ok)throw Error(message);},sleep=ms=>new Promise(resolve=>setTimeout(resolve,ms));
assert(process.argv.length>=4,'run.js PREPARED FRESH_OUTPUT [--record-all] [--package quarry|terrain_stockyard] [--scene SCENE] [--connections both|normal|reversed] [--reverse-allocation] [--reverse-members] [--chrome PATH]');
assert(options.packages.every(p=>['quarry','terrain_stockyard'].includes(p))&&options.scenes.every(s=>scenes.includes(s)),'Unknown frozen package/scene');
assert(['both','normal','reversed'].includes(options.connections),'Unknown connection variant');
assert(!fs.existsSync(output),'Use fresh output; preserve existing evidence');fs.mkdirSync(output,{recursive:true});
const result={status:'RUNNING',source_head:null,options,cases:[],errors:[],limits:['No every-fixed-tick swept claim from browser polling. Native real-tick harness supplies that proof.','The unchanged400-model High FPS instrument is a separate run; this functional campaign makes no FPS claim.']};
const save=()=>fs.writeFileSync(path.join(output,'result.json'),JSON.stringify(result,null,2));
const hash=file=>crypto.createHash('sha256').update(fs.readFileSync(file)).digest('hex');
function checked(executable,argv,log){const command=spawnSync(executable,argv,{cwd:root,encoding:'utf8',maxBuffer:8*1024*1024});fs.writeFileSync(log,(command.stdout||'')+(command.stderr||''));assert(!command.error&&command.status===0,`${executable} failed (${command.status}): ${log}`);return command.stdout;}
const port=()=>new Promise((resolve,reject)=>{const socket=net.createServer();socket.once('error',reject);socket.listen(0,'127.0.0.1',()=>{const selected=socket.address().port;socket.close(()=>resolve(selected));});});
async function stop(child){if(!child||child.exitCode!==null||child.signalCode!==null)return;const done=new Promise(resolve=>child.once('exit',resolve));child.kill('SIGTERM');await Promise.race([done,sleep(2500)]);if(child.exitCode===null&&child.signalCode===null){child.kill('SIGKILL');await done;}}
async function ready(url,server){const deadline=Date.now()+30000;while(Date.now()<deadline){assert(server.exitCode===null&&server.signalCode===null,'Case server stopped; inspect server.log');try{if((await fetch(url+'/advertisement.json')).ok)return;}catch{}await sleep(100);}throw Error('Case server did not become ready');}
(async()=>{
 let xvfb;const profiles=[],contexts=[],children=[];
 try{
    // Verify the actual sealed package identity and every canonical source/tool/artifact check.
    checked('python3',['-B',path.join(root,'tests/browser/terrain-prepared.py'),prepared,'--root',root],path.join(output,'prepared-check.log'));
    result.source_head=checked('git',['rev-parse','HEAD'],path.join(output,'source-head.log')).trim();
    const sourceFiles=['tests/browser/terrain-group/generate.py','tests/browser/terrain-group/scenes.json','tests/browser/terrain-group/driver.js','tests/browser/terrain-group/settled-tracker.js','tests/browser/terrain-group/settled-tracker.test.js','tests/browser/terrain-group/selection-box.js','tests/browser/terrain-group/selection-box.test.js','tests/browser/terrain-group/run.js'];
    checked('git',['diff','--exit-code','HEAD','--',...sourceFiles],path.join(output,'driver-source-check.log'));
    const tracked=checked('git',['ls-files','--',...sourceFiles],path.join(output,'driver-tracked.log')).trim().split('\n');
    assert(sourceFiles.every(file=>tracked.includes(file)),'Campaign source must be committed before use');
    result.source_sha256=Object.fromEntries(sourceFiles.map(file=>[file,hash(path.join(root,file))]));
    for(const packageName of options.packages){
        checked('python3',['-B',path.join(root,'tests/browser/terrain-prepared.py'),prepared,'--root',root,'--fixture',path.join(root,'mods',packageName),'--record',path.join(output,packageName+'-public-source.json')],path.join(output,packageName+'-public-check.log'));
    }
    // Each peer needs its own X pointer and active window. Otherwise concurrent
    // input in another peer can activate a background window during a gesture.
    const displays=[];
    for(let peer=0;peer<2;peer++){
        xvfb=spawn('Xvfb',['-displayfd','3','-screen','0','1280x720x24','-nolisten','tcp'],{stdio:['ignore','ignore','pipe','pipe']});children.push(xvfb);
        const instance=xvfb;
        displays.push(await new Promise((resolve,reject)=>{let text='';const timeout=setTimeout(()=>reject(Error('Owned Xvfb startup timeout')),10000);instance.stdio[3].on('data',data=>{text+=data;if(text.includes('\n')){clearTimeout(timeout);resolve(':'+text.trim());}});instance.once('error',reject);instance.once('exit',code=>reject(Error('Owned Xvfb exited '+code)));}));
    }
    for(const packageName of options.packages)for(const scene of options.scenes){
        const peers=['GM3-opposing-large','GM3-settled-blocker','GM3-settled-blocker-hold','GM3-movable-blocker'].includes(scene)?2:1;
        const connections=peers===2&&options.connections==='both'?['normal','reversed']:[options.connections==='reversed'?'reversed':'normal'];
        for(const connection of connections){
            const caseName=`${packageName}-${scene}-a${+options.allocation}-m${+options.members}-${connection}`,caseOut=path.join(output,caseName);fs.mkdirSync(caseOut);
            const row={name:caseName,package:packageName,scene,connection,status:'RUNNING',errors:[],profiles:[],server_stopped:false};result.cases.push(row);save();
            let server;const caseContexts=[],caseProfiles=[];let stream;
            try{
                const generated=path.join(caseOut,'generated'),generator=['-B',path.join(root,'tests/browser/terrain-group/generate.py'),'--root',root,'--package',packageName,'--scene',scene,'--output',generated];
                if(options.allocation)generator.push('--reverse-allocation');if(options.members)generator.push('--reverse-members');
                checked('python3',generator,path.join(caseOut,'generate.log'));
                const fixture=JSON.parse(fs.readFileSync(path.join(generated,'fixture.json')));assert(fixture.scene_name===scene&&fixture.package===packageName,'Generated identity differs');
                assert(fixture.source_scenes_sha256===hash(path.join(root,'tests/browser/terrain-group/scenes.json')),'Frozen scene source changed');
                assert(fixture.commands_sha256===hash(path.join(generated,'commands.json')),'Finite command metadata changed');
                for(const [file,sha]of Object.entries(fixture.public_files_sha256))assert(hash(path.join(generated,packageName,file))===sha,'Generated public file changed: '+file);
                checked(path.join(prepared,'bin/turts-pack'),[path.join(generated,packageName)],path.join(caseOut,'typed-pack.log'));
                row.archive_sha256=hash(path.join(generated,packageName+'.tar'));
                const privateDir=path.join(caseOut,'private');fs.mkdirSync(privateDir,{mode:0o700});fs.writeFileSync(path.join(privateDir,'seed.txt'),'3457144321\n',{mode:0o600});
                const http=await port();let ws=await port();while(ws===http)ws=await port();const url=`http://127.0.0.1:${http}`,q=JSON.stringify;
                fs.writeFileSync(path.join(caseOut,'server.ron'),`(game_package:${q(path.join(generated,packageName+'.tar'))},http_listen:"127.0.0.1:${http}",ws_listen:"127.0.0.1:${ws}",public_url:${q(url)},tick_hz:20,static_dir:Some(${q(path.join(prepared,'dist'))}),start_policy:Some(Immediate),terrain_seed_file:Some(${q(path.join(privateDir,'seed.txt'))}),terrain_provenance:Some(${q(path.join(privateDir,'terrain.json'))}))\n`);
                const fd=fs.openSync(path.join(caseOut,'server.log'),'w');server=spawn(path.join(prepared,'bin/turts_server'),[path.join(caseOut,'server.ron'),...(process.env.TURTS_TERRAIN_SEND_TRACE==='1'?['--send-trace',path.join(caseOut,'send-trace.csv')]:[])],{stdio:['ignore',fd,fd],env:{...process.env,TURTS_COMPUTER_SLOT:''}});fs.closeSync(fd);children.push(server);await ready(url,server);
                // Context identities are created before navigation. Reversing the navigation
                // sequence genuinely reverses peer connection order while slots remain observed.
                for(let peer=0;peer<peers;peer++){
                    const profile=fs.mkdtempSync(path.join(os.tmpdir(),'turts-terrain-group-'));profiles.push(profile);caseProfiles.push(profile);row.profiles.push(profile);
                    const context=await chromium.launchPersistentContext(profile,{executablePath:options.chrome,headless:false,viewport:{width:1280,height:720},env:{...process.env,DISPLAY:displays[peer]},ignoreDefaultArgs:['--enable-unsafe-swiftshader'],args:['--enable-gpu','--alsa-output-device=null','--enable-unsafe-webgpu','--enable-features=Vulkan','--use-angle=vulkan','--disable-vulkan-surface','--no-first-run','--no-default-browser-check']});contexts.push(context);caseContexts.push(context);
                    await context.addInitScript(()=>localStorage.setItem('turts.v1.settings',JSON.stringify({controls:{quality:'high'}})));
                    const page=context.pages()[0];page.on('pageerror',error=>row.errors.push(String(error)));page.on('requestfailed',request=>row.errors.push('Request failed '+request.url()+': '+request.failure()?.errorText));page.on('response',response=>{if(response.status()>=400)row.errors.push('HTTP '+response.status()+' '+response.url());});
                }
                const sequence=connection==='reversed'?[...caseContexts].reverse():caseContexts,pagesBySlot=new Map();
                for(const context of sequence){const page=context.pages()[0];await page.goto(url+'/?server='+encodeURIComponent(url)+'&ws='+encodeURIComponent(`ws://127.0.0.1:${ws}`));await page.keyboard.press('F9');const deadline=Date.now()+90000;let state;
                    while(Date.now()<deadline){state=await read(page);if(state?.debug.replicated_world?.terrain)break;await sleep(100);}assert(state?.debug.replicated_world?.terrain,'Peer activation timeout');
                    assert(!pagesBySlot.has(state.debug.slot),'Two peers received same slot');pagesBySlot.set(state.debug.slot,page);row.connection_order??=[];row.connection_order.push({profile:caseProfiles[caseContexts.indexOf(context)],slot:state.debug.slot});
                }
                assert(pagesBySlot.has(0)&&(peers===1||pagesBySlot.has(1)),'Required owner peer absent');
                stream=fs.createWriteStream(path.join(caseOut,'observed-public.jsonl'));
                const capture=async(name,state)=>{fs.writeFileSync(path.join(caseOut,name+'.json'),JSON.stringify(state,null,2));for(const [slot,page]of pagesBySlot)await page.screenshot({path:path.join(caseOut,`${name}-slot${slot}.png`)});};
                row.outcome=await drive({pagesBySlot,fixture,capture,record:sample=>stream.write(JSON.stringify(sample)+'\n'),recordInput:sample=>fs.appendFileSync(path.join(caseOut,'pointer-input-observations.jsonl'),JSON.stringify(sample)+'\n'),onPoll:()=>assert(row.errors.length===0,'Browser errors: '+row.errors.join('\n')),recordAll:options.recordAll});
                row.faults=row.outcome.faults;row.return_records=row.outcome.return_records;
                assert(!row.faults.length,row.faults.length+' recorded faults: '+row.faults.map(f=>'['+f.tick+'] '+f.fault).join(' | '));
                row.status='PASS';
            }catch(error){row.status='FAIL';row.error=String(error.stack||error);row.faults??=error.faults;row.return_records??=error.return_records;
                // Record-all: one run lists every failing case; the run still ends FAIL.
                if(!options.recordAll)throw error;}
            finally{
                if(stream){stream.end();await new Promise(resolve=>stream.once('finish',resolve));}
                for(const context of caseContexts)await context.close();for(const profile of caseProfiles)fs.rmSync(profile,{recursive:true,force:true});
                await stop(server);row.server_stopped=!server||server.exitCode!==null||server.signalCode!==null;row.profiles_removed=caseProfiles.every(profile=>!fs.existsSync(profile));
                fs.writeFileSync(path.join(caseOut,'result.json'),JSON.stringify(row,null,2));save();
            }
        }
    }
    const failed=result.cases.filter(row=>row.status!=='PASS').map(row=>row.name);
    assert(!failed.length,'Failed cases: '+failed.join(', '));
    result.status='PASS';
 }catch(error){result.status='FAIL';result.error=String(error.stack||error);process.exitCode=1;console.error(error);}
 finally{
    for(const context of contexts)try{await context.close();}catch{}for(const child of children.reverse())await stop(child);for(const profile of profiles)fs.rmSync(profile,{recursive:true,force:true});
    result.cleanup={profiles_removed:profiles.every(profile=>!fs.existsSync(profile)),children_stopped:children.every(child=>child.exitCode!==null||child.signalCode!==null)};save();
 }
})();
