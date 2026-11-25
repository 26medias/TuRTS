// turts-test-role: pure
'use strict';
// Acceptance of a group order in the FPS guard. Reads are slow on a loaded page, so a short move can start and end
// between two reads. The order counts as accepted when (a) every group unit shows the order in progress, or (b) a
// new command result (a seq that was not there before the click) accepts every group unit, or (c) every group unit
// stands at the goal. A new result that refuses a group unit is a fault, not a wait.
function orderAccepted(seqsBefore, state, groupIds, orderSuffix, atGoal) {
    const world = state.replicated_world;
    // Result shapes (turts_client debug.rs command_result_json): an order gives `accepted` [ids] and `refused`
    // [{unit, reason}], plus `rejected` when no unit is accepted; a rejected command gives only `rejected`.
    const fresh = (state.command_results || []).filter(r => !seqsBefore.has(r.seq));
    const rejected = fresh.filter(r => typeof r.rejected === 'string').map(r => ({seq: r.seq, rejected: r.rejected}));
    const refused = fresh.flatMap(r => (r.refused || []).filter(x => groupIds.includes(x.unit)).map(x => ({seq: r.seq, ...x})));
    if (rejected.length || refused.length) throw Error('Group order refused: ' + JSON.stringify([...rejected, ...refused]));
    if (groupIds.every(id => world.unit_orders.some(o => o.unit === id && o.current?.order?.endsWith(orderSuffix)))) return 'in progress';
    if (fresh.some(r => Number.isInteger(r.applied_tick) && Array.isArray(r.accepted) && groupIds.every(id => r.accepted.includes(id)))) return 'accepted result';
    if (atGoal && groupIds.every(id => { const u = world.units.find(v => v.unit_id === id); return u && atGoal(u.pos); })) return 'at goal';
    return null;
}
module.exports = {orderAccepted};
