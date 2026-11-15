// SV7 renderer matrix. Reuse the accepted combat instrument under an owned silent display.
'use strict';
const fs=require('node:fs'),path=require('node:path'),{spawn}=require('node:child_process');
const [prepared,fixtures,output]=process.argv.slice(2);
if(!output)throw Error('Usage: visibility-measure.js PREPARED VISIBILITY_FIXTURES OUTPUT');
fs.mkdirSync(output,{recursive:false});
const metadata=JSON.parse(fs.readFileSync(path.join(fixtures,'fixtures.json')));
const skip=new Set((process.env.TURTS_VISIBILITY_SKIP||'').split(',').filter(Boolean));
const result={status:'RUNNING',fixtures:metadata,rows:[],commands:[],skipped_rows:[]};
const sleep=ms=>new Promise(resolve=>setTimeout(resolve,ms));
const save=()=>fs.writeFileSync(path.join(output,'result.json'),JSON.stringify(result,null,2));
(async()=>{let xvfb;try{
 xvfb=spawn('Xvfb',['-displayfd','3','-screen','0','1280x720x24','-nolisten','tcp'],{stdio:['ignore','ignore','pipe','pipe']});
 xvfb.stderr.on('data',data=>fs.appendFileSync(path.join(output,'display.log'),data));
 const display=await new Promise((resolve,reject)=>{let text='';xvfb.stdio[3].on('data',data=>{text+=data;if(text.includes('\n'))resolve(':'+text.trim())});xvfb.once('error',reject);xvfb.once('exit',code=>reject(Error('Xvfb exited '+code)))});
 result.display=display;
 for(const fixture of metadata.cases)for(const zoom of fixture.zooms){
  if(skip.has(fixture.case+'-'+zoom)){result.skipped_rows.push(fixture.case+'-'+zoom);save();continue;}
  const rowOutput=path.join(output,fixture.case+'-'+zoom);
  const argv=[path.join(__dirname,'effects-measure.js'),prepared,path.join(fixtures,fixture.case),rowOutput];
  const env={...process.env,DISPLAY:display,TURTS_EFFECT_SCENE:'battle400',TURTS_EFFECT_QUALITY:'high',TURTS_EFFECT_MIXED:'1',TURTS_SHARED_SKIN_CHECK:'1',TURTS_ENVIRONMENT_CHECK:'1',TURTS_EFFECT_ZOOM:zoom,TURTS_VISIBILITY_MODE:fixture.mode};
  for(const name of ['TURTS_EFFECT_PAN','TURTS_EFFECT_FLASHES','TURTS_SKIN_REFERENCE','TURTS_SKIN_MATRIX_LIMIT','TURTS_EFFECT_SCENE_POLL','TURTS_CPU_PROFILE'])delete env[name];
  const fd=fs.openSync(path.join(output,fixture.case+'-'+zoom+'.log'),'w');
  result.commands.push({argv:[process.execPath,...argv],environment:{DISPLAY:display,TURTS_EFFECT_SCENE:env.TURTS_EFFECT_SCENE,TURTS_EFFECT_QUALITY:env.TURTS_EFFECT_QUALITY,TURTS_EFFECT_ZOOM:zoom,TURTS_VISIBILITY_MODE:fixture.mode}});save();
  const child=spawn(process.execPath,argv,{stdio:['ignore',fd,fd],env});fs.closeSync(fd);
  const exit=await new Promise((resolve,reject)=>{child.once('error',reject);child.once('exit',resolve)});
  const row=JSON.parse(fs.readFileSync(path.join(rowOutput,'result.json')));
  result.rows.push({case:fixture.case,zoom,exit,...row});save();
  if(exit!==0||row.status!=='PASS')throw Error(fixture.case+' '+zoom+' failed: '+row.status);
  if(row.rows.length!==1||row.rows[0].quality!=='high'||row.rows[0].fps<30)throw Error('SV7 High30FPS floor failed');
 }
 if(!result.rows.length)throw Error('No SV7 rows selected');result.status='PASS';console.log('PASS SV7 selected rows: '+result.rows.length+'; explicitly skipped: '+result.skipped_rows.length);
}catch(error){result.status='FAIL';result.error=String(error.stack||error);process.exitCode=1;console.error(error)}
finally{
 if(xvfb&&xvfb.exitCode===null){const stopped=new Promise(resolve=>xvfb.once('exit',resolve));xvfb.kill('SIGTERM');await Promise.race([stopped,sleep(2000)]);if(xvfb.exitCode===null&&xvfb.signalCode===null){xvfb.kill('SIGKILL');await stopped}}
 result.xvfb_stopped=!xvfb||xvfb.exitCode!==null||xvfb.signalCode!==null;save();
}})();
