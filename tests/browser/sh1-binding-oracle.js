// turts-test-role: oracle
'use strict';
const assert=require('node:assert/strict');
const {product,whitePixels}=require('./sh1-oracle.js');
function get(captures,name){const value=captures.find(c=>c.name===name);assert(value?.checkpoint?.debug,`SH1 actual ${name} is absent`);return value;}
function menu(capture){const values=capture.checkpoint.debug.ui_snapshot.filter(node=>node.kind==='settings_menu'&&node.visible===true);assert(values.length===1&&values[0].descendants_truncated===false&&values[0].text_truncated===false,'SH1 binding observation is missing or truncated');return values[0].text_nodes;}
function has(debug,action,code){const values=debug.effective_bindings?.actions?.[action];return values?.length===1&&values[0].Key?.code===code&&values[0].Key.mods.length===0;}
function buttonForRow(capture,label,action){const nodes=menu(capture),row=nodes.find(n=>n.visible&&n.text===action);assert(row,'SH1 target binding row is absent');const value=nodes.find(n=>n.text===label&&n.button_visible===true&&n.button_rect[0]>=row.rect[0]+row.rect[2]&&n.button_rect[1]<row.rect[1]+row.rect[3]&&n.button_rect[1]+n.button_rect[3]>row.rect[1]);assert(value,'SH1 real binding row button is absent');return value.button_rect;}
function button(capture,label){const value=menu(capture).find(n=>n.text===label&&n.button_visible===true);assert(value,'SH1 actual offered conflict control is absent');return value.button_rect;}
function physical(input,before,after,rect){const dpr=before.checkpoint.browser.device_pixel_ratio;assert(Number.isFinite(dpr)&&dpr>0,'SH1 button point lacks actual browser scale');const found=input.find((entry,index)=>index>before.input_call&&index<after.input_call&&entry.call==='click'&&entry.window===before.window&&entry.document_epoch===before.document_epoch&&entry.actual_events.some(event=>event.type==='pointerdown'&&entry.event_seq.includes(event.seq)&&event.canvas_target===true&&event.client_x*dpr>=rect[0]&&event.client_x*dpr<rect[0]+rect[2]&&event.client_y*dpr>=rect[1]&&event.client_y*dpr<rect[1]+rect[3]));assert(found,'SH1 outcome lacks a real same-document click inside its captured button');return found;}
function actualKey(input,before,after,code){const found=input.find((entry,index)=>index>before.input_call&&index<after.input_call&&entry.call==='key'&&entry.args?.[0]===code&&entry.window===before.window&&entry.document_epoch===before.document_epoch&&entry.actual_events.some(event=>event.type==='keydown'&&event.code===code&&entry.event_seq.includes(event.seq)));assert(found,'SH1 binding capture lacks the actual physical keydown');return found;}
function guide(capture,expected,decoded){const list=capture.checkpoint.debug.ui_snapshot.find(node=>node.id==='guide.4.bindings'&&node.visible);assert(list,'SH1 actual section-five binding list is absent');for(const text of expected){const node=capture.checkpoint.debug.ui_snapshot.find(node=>node.id?.startsWith('guide.4.')&&node.visible===true&&node.text===text);product(node,'guide-effective-label',`actual guide omits ${text}`);product(whitePixels(decoded.get(capture.name),node.rect)>=20,'guide-effective-label',`actual ${text} guide glyph pixels are absent`);}}
function judgeSh1GuideChange(before,after,decoded){
 assert(before.window===after.window&&before.document_epoch===after.document_epoch&&before.input_call<after.input_call,
  'SH1 guide glyph comparison needs one ordered actual document');
 for(const [label,oldKey,newKey,prefixWidth]of [['Move selected units','S','M',112],['Stop selected units','M','S',107]]){
  const samples=[[before,oldKey],[after,newKey]].map(([capture,key])=>{
   const node=capture.checkpoint.debug.ui_snapshot.find(n=>n.id?.startsWith('guide.4.')&&n.visible&&n.text===`${label}: ${key}`);
   assert(node,'SH1 changed key label lacks its actual guide rectangle');
   const image=decoded.get(capture.name),dpr=capture.checkpoint.browser.device_pixel_ratio;
   assert(Number.isFinite(dpr)&&dpr>=1&&dpr<=3&&image.width===1280*dpr&&image.height===720*dpr,
    'SH1 guide PNG must match its actual device factor');
   const [x,y,width,height]=node.rect;
   assert(width>=75*dpr&&height===24*dpr&&x>=0&&y>=0&&x+width<=image.width&&y+height+3*dpr<=image.height,
    'SH1 calibrated key glyph region falls outside its real screenshot');
   const pixels=(left,top,w,h)=>{
    const result=new Set();
    for(let py=Math.round(top);py<Math.round(top+h);py+=1)for(let px=Math.round(left);px<Math.round(left+w);px+=1){
     const at=(py*image.width+px)*4;
     if(image.rgba[at]>180&&image.rgba[at+1]>180&&image.rgba[at+2]>180&&image.rgba[at+3]>=250)result.add(`${px-Math.round(x)},${py-Math.round(y)}`);
    }
    return result;
   };
   return {dpr,prefix:pixels(x,y,prefixWidth*dpr,height),key:pixels(x+prefixWidth*dpr,y,18*dpr,height),blank:pixels(x,y+height,75*dpr,3*dpr)};
  });
  const [a,b]=samples;
  assert(a.dpr===b.dpr,'SH1 compared guide device factor changed');
  const area=a.dpr*a.dpr,common=[...a.prefix].filter(point=>b.prefix.has(point)).length;
  const changed=[...a.key].filter(point=>!b.key.has(point)).length+[...b.key].filter(point=>!a.key.has(point)).length;
  product(a.prefix.size>=100*area&&b.prefix.size>=100*area&&common/Math.max(a.prefix.size,b.prefix.size)>=0.95&&
   a.key.size>=15*area&&b.key.size>=15*area&&changed>=15*area&&a.blank.size<=5*area&&b.blank.size<=5*area,
   'guide-effective-label',`actual ${label} key glyph must change ${oldKey} to ${newKey} with a stable prefix and blank control (common=${common},prefix=${a.prefix.size}/${b.prefix.size},key=${a.key.size}/${b.key.size},blank=${a.blank.size}/${b.blank.size},changed=${changed})`);
 }
}
function judgeSh1Bindings(input,captures,decoded,evaluated){
 const initial=get(captures,'world-ready'),edit=get(captures,'conflict-edit-row'),offer=get(captures,'conflict-open'),swapped=get(captures,'conflict-swapped');
 product(has(initial.checkpoint.debug,'key_stop','KeyS')&&has(initial.checkpoint.debug,'key_move','KeyM'),'conflict-swap-cancel-reset','actual verified defaults must start StopS and MoveM');
 physical(input,edit,offer,buttonForRow(edit,'Change','Stop selected units'));actualKey(input,edit,offer,'KeyM');
 product(has(offer.checkpoint.debug,'key_stop','KeyS')&&has(offer.checkpoint.debug,'key_move','KeyM'),'conflict-swap-cancel-reset','an unresolved offer changed an existing action');
 physical(input,offer,swapped,button(offer,'Swap'));
 product(has(swapped.checkpoint.debug,'key_stop','KeyM')&&has(swapped.checkpoint.debug,'key_move','KeyS'),'conflict-swap-cancel-reset','actual Swap did not move Move to the old Stop chord');
 const cancelEdit=get(captures,'conflict-cancel-edit-row'),cancelOffer=get(captures,'conflict-cancel-offer'),cancelled=get(captures,'conflict-cancelled'),reset=get(captures,'conflict-reset');
 physical(input,cancelEdit,cancelOffer,buttonForRow(cancelEdit,'Change','Stop selected units'));actualKey(input,cancelEdit,cancelOffer,'KeyS');
 product(menu(cancelOffer).some(node=>node.visible&&node.text==='S is also bound to Move selected units'),'conflict-swap-cancel-reset','second real conflict did not identify Move');
 physical(input,cancelOffer,cancelled,button(cancelOffer,'Cancel'));
 product(JSON.stringify(cancelled.checkpoint.debug.effective_bindings.actions)===JSON.stringify(swapped.checkpoint.debug.effective_bindings.actions),'conflict-swap-cancel-reset','actual Cancel changed either effective action');
 physical(input,cancelled,reset,buttonForRow(cancelled,'Reset','Stop selected units'));
 product(has(reset.checkpoint.debug,'key_stop','KeyS')&&has(reset.checkpoint.debug,'key_move','KeyM'),'conflict-swap-cancel-reset','actual Reset did not restore executable, conflict-free defaults');
 for(const[action,value]of Object.entries(initial.checkpoint.debug.effective_bindings.actions)){
  if(action==='key_stop'||action==='key_move')continue;
  for(const capture of[swapped,cancelled,reset])product(JSON.stringify(capture.checkpoint.debug.effective_bindings.actions[action])===JSON.stringify(value),'conflict-swap-cancel-reset',`binding edit changed unrelated ${action}`);
 }
 evaluated.add('conflict-swap-cancel-reset');
 const swappedGuide=get(captures,'guide-swapped'),resetGuide=get(captures,'guide-reset');
 actualKey(input,swapped,swappedGuide,'F1');actualKey(input,reset,resetGuide,'F1');
 guide(swappedGuide,['Stop selected units: M','Move selected units: S'],decoded);guide(resetGuide,['Stop selected units: S','Move selected units: M'],decoded);
 judgeSh1GuideChange(swappedGuide,resetGuide,decoded);
 const end=get(captures,'conflict-menu-closed');product(!end.checkpoint.debug.ui_snapshot.some(node=>(node.kind==='settings_menu'||node.id==='guide')&&node.visible===true),'guide-effective-label','actual guide/menu remained open after their physical close');
 evaluated.add('guide-effective-label');return evaluated;
}
module.exports={judgeSh1Bindings,judgeSh1GuideChange};
