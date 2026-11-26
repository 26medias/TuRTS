// turts-test-role: pure
// The FPS row measures frame time while units move. The battle's own units stand in melee, and a Move leg that crosses
// it ends there (rule 2a, T-59), so the row's motion ran out inside a sample. These script-driven movers walk to and
// fro in the clear ground above the battle for the whole run. The accepted combat script stays as the first part.
'use strict';
// Two groups turn at different times, and a leg is shorter than the lane, so one group is always on its way.
const MOVERS = 6, PERIOD_TICKS = 50, OFFSET_TICKS = 25, LAST_TICK = 12000, LANE_Y = 31, WEST = 18, EAST = 36;
// Unit ids are the placement order of the map, from 1. The movers are the slot 0 soldiers of the highest row.
function moverIds(mapText, excluded) {
    const placement = /unit:\s*"skirmish:(\w+)",\s*owner_slot:\s*([01]),\s*pos:\s*\(x:\s*([0-9.]+),\s*y:\s*([0-9.]+)\)/g;
    const placements = Array.from(mapText.matchAll(placement), ([, kind, slot, x, y], i) => ({id: i + 1, kind, slot: +slot, x: +x, y: +y}));
    const ids = placements.filter(p => p.kind === 'soldier' && p.slot === 0 && !excluded.includes(p.id) && p.x >= WEST)
        .sort((a, b) => b.y - a.y || a.x - b.x).slice(0, MOVERS).map(p => p.id);
    if (ids.length !== MOVERS) throw Error('The map has no ' + MOVERS + ' free movers');
    return ids;
}
function scriptWithMovers(commands, ids) {
    const moves = [];
    for (const [group, units] of [ids.slice(0, ids.length / 2), ids.slice(ids.length / 2)].entries())
        for (let tick = PERIOD_TICKS + group * OFFSET_TICKS, leg = 0; tick <= LAST_TICK; tick += PERIOD_TICKS, leg++)
            moves.push({tick, slot: 0, envelope: {order: 'skirmish:order_move', units, target: {Point: {x: (leg + group) % 2 ? EAST : WEST, y: LANE_Y}}, queue: 'Replace', group: 0}});
    return [...commands, ...moves].sort((a, b) => a.tick - b.tick);
}
module.exports = {moverIds, scriptWithMovers};
