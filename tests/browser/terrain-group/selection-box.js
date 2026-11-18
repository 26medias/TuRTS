'use strict';
function exactOwnedBox(points, wanted) {
    const wantedSet = new Set(wanted);
    const selected = points.filter(p => wantedSet.has(p.id));
    if (selected.length !== wantedSet.size || selected.some(p => !p.owned || !Array.isArray(p.center) || !p.center.every(Number.isFinite))) return null;
    const xs=selected.map(p=>p.center[0]), ys=selected.map(p=>p.center[1]);
    const rect=[Math.min(...xs)-2,Math.min(...ys)-2,Math.max(...xs)+2,Math.max(...ys)+2];
    if(points.some(p=>p.owned&&!wantedSet.has(p.id)&&p.center?.[0]>=rect[0]&&p.center[0]<=rect[2]&&p.center?.[1]>=rect[1]&&p.center[1]<=rect[3]))return null;
    return rect;
}
module.exports={exactOwnedBox};
