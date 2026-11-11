// turts-test-role: oracle
'use strict';
const assert = require('node:assert/strict');
const {product, visible} = require('./sh1-oracle.js');

function get(captures, name) {
    const found = captures.find((capture) => capture.name === name);
    assert(found?.checkpoint?.debug, `SH1 actual ${name} capture is absent`);
    return found;
}
function currentSettings(audit, capture, input) {
    const last = audit.entries.filter((entry) => entry.source === 'storage_api' && entry.storage === 'localStorage' &&
        entry.operation === 'set' && entry.key === 'turts.v1.settings' && entry.window === capture.window &&
        entry.document_epoch === capture.document_epoch && entry.wall_time_ms <= input[capture.input_call].wall_time_ms).at(-1);
    assert(last && typeof last.value === 'string', 'SH1 setting lacks its actual successful Storage write before capture');
    const document = JSON.parse(last.value);
    assert(document.controls && typeof document.controls === 'object', 'SH1 saved controls are absent');
    return document.controls;
}
function centre(capture) {
    const matrix = capture.checkpoint.scene.clip_from_world;
    assert(Array.isArray(matrix) && matrix.length === 16 && matrix.every(Number.isFinite), 'SH1 camera matrix is incomplete');
    const [a,b,c,d,e,f] = [matrix[0],matrix[8],matrix[1],matrix[9],matrix[12],matrix[13]];
    const det = a*d-b*c;
    assert(Math.abs(det) > 1e-9, 'SH1 actual camera cannot intersect the ground at its viewport centre');
    return [(-e*d+b*f)/det,(-a*f+c*e)/det];
}
function panMeasurement(captures, input, prefix) {
    const before = get(captures, `${prefix}-before`), after = get(captures, `${prefix}-after`);
    assert(before.window === after.window && before.document_epoch === after.document_epoch && before.input_call < after.input_call,
        'SH1 pan must retain one actual ordered document');
    const key = input.find((entry,index) => index > before.input_call && index < after.input_call &&
        entry.call === 'key' && entry.args?.[0] === 'ArrowRight' && entry.args?.[2]?.hold_ms === 300);
    const down = key?.actual_events?.find((event) => event.type === 'keydown' && event.code === 'ArrowRight');
    const up = key?.actual_events?.find((event) => event.type === 'keyup' && event.code === 'ArrowRight');
    assert(down && up && up.wall_time_ms > down.wall_time_ms, 'SH1 pan lacks its actual held physical key interval');
    const start = centre(before), end = centre(after);
    const distance = Math.hypot(end[0]-start[0],end[1]-start[1]);
    const seconds = (up.wall_time_ms-down.wall_time_ms)/1000;
    product(distance > 0.001, 'scroll-distance-ratio', 'actual held pan did not change the ground-space camera centre');
    return { distance, seconds, velocity: distance/seconds, before: start, after: end };
}
function physicalDpr(capture, image) {
    const dpr = capture.checkpoint.browser.device_pixel_ratio;
    assert(Number.isFinite(dpr) && dpr >= 1 && dpr <= 3 && image.width === 1280*dpr && image.height === 720*dpr,
        'SH1 pixel dimensions must match the actual observed browser device factor');
    return dpr;
}
function light(image, rect) {
    const [x,y,width,height] = rect.map(Math.round);
    assert(x >= 0 && y >= 0 && width > 0 && height > 0 && x+width <= image.width && y+height <= image.height,
        'SH1 actual glyph region is outside its PNG');
    const points=[];
    for(let py=y;py<y+height;py+=1)for(let px=x;px<x+width;px+=1){
        const at=(py*image.width+px)*4;
        if(image.rgba[at]>235&&image.rgba[at+1]>235&&image.rgba[at+2]>235&&image.rgba[at+3]>=250)points.push([px,py]);
    }
    return points;
}
function judgeSh1ScaleBounds(capture, image, scale) {
    const dpr=physicalDpr(capture,image),group=visible(capture,'control_groups');
    const actual=group.rect.map(value=>value/dpr);
    const reference=[296,8,620,40].map(value=>value*scale);
    product(actual.every((value,index)=>Math.abs(value-reference[index])<=1.5),
        'ui-scale-rects',`actual HUD bounds must follow authored logical geometry at this setting and device factor (scale=${scale},dpr=${dpr},rect=${actual})`);
}
async function judgeSh1Settings(view, input, captures, decoded, evaluated) {
    const audit=JSON.parse(await view.serverFile('storage-audit.json'));
    assert(Array.isArray(audit.entries), 'SH1 actual preference write audit is absent');
    const clickRange=currentSettings(audit,get(captures,'double-click-range-setting'),input).double_click_pixels;
    assert(clickRange>7.5&&clickRange<8.5,'SH1 physical distance control did not change its existing saved preference');
    const lowSetting=get(captures,'speed-low-setting'),highSetting=get(captures,'speed-high-setting');
    const lowValue=currentSettings(audit,lowSetting,input).scroll_speed;
    const highValue=currentSettings(audit,highSetting,input).scroll_speed;
    assert(Number.isFinite(lowValue)&&Number.isFinite(highValue)&&lowValue>0&&highValue/lowValue>=3,
        'SH1 physical slider points did not produce two sufficiently different actual speeds');
    const low=panMeasurement(captures,input,'speed-low'),high=panMeasurement(captures,input,'speed-high');
    const expected=highValue/lowValue,actual=high.velocity/low.velocity;
    product(Math.abs(actual/expected-1)<=0.2,'scroll-distance-ratio',
        `actual camera distance per held time must follow its saved speed ratio (expected=${expected},actual=${actual})`);
    evaluated.add('scroll-distance-ratio');

    const initial=get(captures,'world-ready'),changed=get(captures,'scale-changed-world');
    const setting=currentSettings(audit,changed,input).ui_scale;
    assert(setting>1.4&&setting<1.6,'SH1 physical UI-scale point did not reach its deliberate interior setting');
    for(const [capture,scale]of[[initial,1],[changed,setting]]){
        judgeSh1ScaleBounds(capture,decoded.get(capture.name),scale);
    }
    evaluated.add('ui-scale-rects');

    const option=get(captures,'cursor-option-on'),locked=get(captures,'cursor-locked'),released=get(captures,'cursor-menu-released');
    assert(currentSettings(audit,option,input).cursor_lock===true,'SH1 cursor option did not persist after its actual toggle');
    product(locked.checkpoint.browser.canvas_pointer_locked===true&&locked.checkpoint.debug.cursor?.lock_active===true,
        'cursor-lock','actual enabled option plus physical canvas gesture did not acquire pointer lock');
    product(released.checkpoint.browser.canvas_pointer_locked===false&&released.checkpoint.debug.cursor?.lock_active===false&&
        released.checkpoint.debug.ui_snapshot.some(node=>node.kind==='settings_menu'&&node.visible===true),
        'cursor-lock','actual F10 did not release the cursor into the settings menu');
    assert(currentSettings(audit,get(captures,'cursor-option-off'),input).cursor_lock===false,
        'SH1 cursor option was not actually disabled before the following layout episode');
    evaluated.add('cursor-lock');

    const before=get(captures,'world-ready'),blocked=get(captures,'menu-world-input-captured');
    const f10Index=input.findIndex(entry=>entry.call==='key'&&entry.args?.[0]==='F10');
    const keys=input.map((entry,index)=>({entry,index})).filter(({entry})=>entry.call==='key'&&entry.args?.[0]==='KeyD');
    const control=keys.find(({index})=>index<f10Index),captured=keys.find(({index})=>index>f10Index&&index<blocked.input_call);
    assert(control&&captured,'SH1 zero-order proof lacks both actual control and captured hotkey');
    const frames=cap=>[...(cap.checkpoint.debug.frames?.retained_input||[]),...(cap.checkpoint.debug.frames?.recent||[])];
    const relevant=(cap,key)=>{
        assert(key.entry.window===cap.window&&key.entry.document_epoch===cap.document_epoch,
            'SH1 hotkey and consumer capture belong to different documents');
        const down=key.entry.actual_events.find(event=>event.type==='keydown'&&event.code==='KeyD');
        assert(down&&Number.isSafeInteger(down.seq),'SH1 physical game hotkey lacks its actual keydown sequence');
        const observed=frames(cap).filter(frame=>frame.applied_seq.some(seq=>key.entry.actual_events.some(event=>event.seq===seq)));
        assert(observed.length>0&&observed.every(frame=>frame.named_dispatches_truncated===0&&
            Array.isArray(frame.named_dispatches)&&Array.isArray(frame.emitted_orders)),
            'SH1 hotkey frame dispatch/order observation is absent or truncated');
        const keydown=observed.filter(frame=>frame.applied_seq.includes(down.seq));
        assert(keydown.length>0,'SH1 hotkey lacks the actual applied keydown frame; keyup alone cannot qualify');
        return {all:observed,keydown};
    };
    const controlFrames=relevant(before,control),blockedEvidence=relevant(blocked,captured),blockedFrames=blockedEvidence.all;
    assert(controlFrames.keydown.some(frame=>frame.emitted_orders.some(order=>order.order==='forge:order_disable'&&order.units.includes(7))),
        'SH1 nonvacuity control must issue a real typed order for owned unit7 outside the menu');
    product(blockedFrames.length>0&&blockedFrames.every(frame=>frame.emitted_orders.length===0)&&
        blocked.checkpoint.debug.ui_snapshot.some(node=>node.kind==='settings_menu'&&node.visible===true),
        'zero-menu-world-orders','the same real game hotkey issued a world order while the menu captured it');
    evaluated.add('zero-menu-world-orders');

    const top=get(captures,'menu-inventory-top'),bottom=get(captures,'menu-inventory-bottom');
    const required=[['Settings',top],['Key bindings',top],['Change',top],['Reset',top],['Scroll speed',bottom],
        ['Edge scroll',bottom],['UI scale',bottom],['Health bars',bottom],['Keep the cursor in the game',bottom],
        ['Double-click time',bottom],['Double-click range',bottom],['Master',bottom],['Voice',bottom],['Alerts',bottom],['Effects',bottom]];
    for(const [label,capture]of required){
        const image=decoded.get(capture.name),dpr=physicalDpr(capture,image);
        const menu=capture.checkpoint.debug.ui_snapshot.find(node=>node.kind==='settings_menu'&&node.visible===true);
        const text=menu?.text_nodes?.find(node=>node.visible===true&&node.text===label);
        assert(text,'SH1 actual calibrated menu text is missing');
        const pixels=light(image,text.rect);
        product(pixels.length>=10*dpr*dpr,'menu-pixels',`actual ${label} menu glyph pixels are absent`);
    }
    const heading=top.checkpoint.debug.ui_snapshot.find(node=>node.kind==='settings_menu').text_nodes.find(node=>node.text==='Settings');
    const dpr=physicalDpr(top,decoded.get(top.name));
    const absent=light(decoded.get(before.name),heading.rect);
    const nearby=[heading.rect[0]+heading.rect[2]-100*dpr,heading.rect[1],100*dpr,heading.rect[3]];
    product(absent.length<=20*dpr*dpr&&light(decoded.get(top.name),nearby).length<=20*dpr*dpr,
        'menu-pixels','actual menu title lacks the same-document absence and nearby-blank pixel controls');
    evaluated.add('menu-pixels');
    return evaluated;
}
module.exports={judgeSh1Settings,judgeSh1ScaleBounds,centre,currentSettings};
