// turts-test-role: pure
"use strict";
const assert = require("node:assert/strict");
function blue(image,x,y) {
    const i=(y*image.width+x)*4;
    return image.rgba[i+2]>image.rgba[i]+60 && image.rgba[i+1]>image.rgba[i]+40 && image.rgba[i+2]>image.rgba[i+1]+20 && image.rgba[i+3]>=250;
}
function silhouette(image,point) {
    const [cx,cy]=point.map(Math.round);
    assert(cx>=12 && cy>=14 && cx+12<image.width && cy+10<image.height,"unit pixel region is outside image");
    let points=[];
    for(let y=cy-12;y<=cy+8;y++) for(let x=cx-10;x<=cx+10;x++) if(blue(image,x,y)) points.push([x,y]);
    const remaining=new Set(points.map(p=>p.join(","))),components=[];
    while(remaining.size) {
        const key=remaining.values().next().value,queue=[key],component=[];remaining.delete(key);
        while(queue.length) {
            const [x,y]=queue.pop().split(",").map(Number);component.push([x,y]);
            for(const [dx,dy] of [[-1,0],[1,0],[0,-1],[0,1]]){const n=`${x+dx},${y+dy}`;if(remaining.delete(n))queue.push(n);}
        }
        components.push(component);
    }
    points=components.sort((a,b)=>b.length-a.length)[0]||[];
    const width=points.length?Math.max(...points.map(p=>p[0]))-Math.min(...points.map(p=>p[0]))+1:0;
    const height=points.length?Math.max(...points.map(p=>p[1]))-Math.min(...points.map(p=>p[1]))+1:0;
    const centroid=points.length?points.reduce((s,p)=>[s[0]+p[0]/points.length,s[1]+p[1]/points.length],[0,0]):[Infinity,Infinity];
    assert(points.length>=40 && points.length<=180 && width>=8 && width<=16 && height>=8 && height<=16 && Math.hypot(centroid[0]-point[0],centroid[1]-point[1])<=7,`expected worker silhouette pixels are missing or misplaced: count=${points.length}, width=${width}, height=${height}, centroid=${centroid}, point=${point}`);
    return {count:points.length,width,height,centroid};
}
function judgeSel4Pixels(before,after,beforeImage,afterImage,ids) {
    assert(before.window===after.window && before.document_epoch===after.document_epoch && before.input_call<after.input_call,"centering PNGs must retain same-window/document ordered captures");
    assert(ids.length===2 && new Set(ids).size===2,"pixel fixture needs its two distinct actual workers");
    for(const image of [beforeImage,afterImage]) assert(image.width===1280 && image.height===720,"unexpected pixel dimensions");
    const stages=[];
    for(const [capture,image] of [[before,beforeImage],[after,afterImage]]) {
        const scene=capture.checkpoint.scene;
        assert.deepEqual(scene.viewport_physical_rect,[0,0,1280,720]);
        assert.equal(scene.scale_factor,1,"calibration requires source-observed DPR1");
        assert.equal(scene.coordinate_space,"window-logical-pixels-top-left");
        const units=ids.map(id=>scene.units.find(u=>u.unit_id===id));
        assert(units.every(Boolean),"actual target workers are absent");
        assert(units.every(u=>Array.isArray(u.translation) && u.translation.length===3 && u.translation.every(Number.isFinite)),"actual target translations must be finite 3-vectors");
        const shapes=units.map(u=>silhouette(image,u.ground_screen_center));
        stages.push({units,shapes,centroid:shapes.reduce((s,p)=>[s[0]+p.centroid[0]/2,s[1]+p.centroid[1]/2],[0,0])});
    }
    assert.deepEqual(stages[0].units.map(u=>u.translation),stages[1].units.map(u=>u.translation),"unit movement cannot stand in for camera centering");
    const distance=p=>Math.hypot(p[0]-640,p[1]-360);
    assert(distance(stages[1].centroid)<=10 && distance(stages[1].centroid)<distance(stages[0].centroid)-5,"actual worker pixels did not center on the viewport");
    // The actual previous worker regions must become empty after centering.
    // New regions already contain unrelated troopers before centering; they are not absence controls.
    for(const [image,units] of [[afterImage,stages[0].units]]) {
        for(const u of units) {
            const [cx,cy]=u.ground_screen_center.map(Math.round);let count=0;
            for(let y=cy-12;y<=cy+8;y++)for(let x=cx-10;x<=cx+10;x++)if(blue(image,x,y))count++;
            assert(count<=5,"same-region camera-movement absence control failed");
        }
    }
    return {stages,beforeDistance:distance(stages[0].centroid),afterDistance:distance(stages[1].centroid)};
}
module.exports={judgeSel4Pixels};
