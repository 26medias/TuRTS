// Exercises real Chrome pointer events with two peer windows on an owned Xvfb.
'use strict';
const assert=require('node:assert/strict');
const fs=require('node:fs'),os=require('node:os'),path=require('node:path');
const {spawn}=require('node:child_process');
const {chromium}=require('playwright-core');
const {activatePage}=require('./driver');
const sleep=ms=>new Promise(resolve=>setTimeout(resolve,ms));
(async()=>{
 const xvfb=spawn('Xvfb',['-displayfd','3','-screen','0','1280x720x24','-nolisten','tcp'],{stdio:['ignore','ignore','pipe','pipe']});
 const display=await new Promise((resolve,reject)=>{let text='';xvfb.stdio[3].on('data',data=>{text+=data;if(text.includes('\n'))resolve(':'+text.trim());});xvfb.once('error',reject);xvfb.once('exit',code=>reject(Error('Xvfb exited '+code)));});
 const contexts=[],profiles=[];
 try{
  for(let i=0;i<2;i++){
   const profile=fs.mkdtempSync(path.join(os.tmpdir(),'turts-focus-pointer-'));profiles.push(profile);
   const context=await chromium.launchPersistentContext(profile,{executablePath:process.env.CHROME_BIN||'/usr/bin/google-chrome',headless:false,viewport:{width:1280,height:720},env:{...process.env,DISPLAY:display},args:['--alsa-output-device=null','--no-first-run','--no-default-browser-check']});contexts.push(context);
   const page=context.pages()[0];await page.goto('data:text/html,<style>body{margin:0}canvas{width:100vw;height:100vh}</style><canvas></canvas>');
   await page.evaluate(()=>{window.events=[];for(const type of ['pointermove','pointerdown','pointerup'])window.addEventListener(type,e=>events.push({type,x:e.clientX,y:e.clientY,buttons:e.buttons}));});
  }
  const page=contexts[0].pages()[0];await contexts[1].pages()[0].bringToFront();await activatePage(page);await page.evaluate(()=>events=[]);
  await page.mouse.click(640,355,{delay:80});await sleep(150);
  const click=await page.evaluate(()=>events);const down=click.findIndex(e=>e.type==='pointerdown'),up=click.findIndex(e=>e.type==='pointerup');
  assert(down>=0&&up>down,'click must reach the page');
  assert(click.slice(down+1,up).every(e=>e.x===640&&e.y===355),'native pointer moved during click');
  await contexts[1].pages()[0].bringToFront();await activatePage(page);await page.evaluate(()=>events=[]);
  await page.mouse.move(600,300);await page.mouse.down();await page.mouse.move(700,400,{steps:4});await page.mouse.up();
  const drag=await page.evaluate(()=>events);assert(drag.some(e=>e.type==='pointermove'&&e.buttons===1&&e.x>600&&e.y>300),'genuine drag must remain visible');
  console.log('PASS: focused peer click and genuine drag');
 }finally{for(const context of contexts)await context.close();for(const profile of profiles)fs.rmSync(profile,{recursive:true,force:true});xvfb.kill();}
})().catch(error=>{console.error(error);process.exitCode=1;});
