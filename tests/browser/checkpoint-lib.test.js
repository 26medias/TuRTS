// turts-test-role: unit
"use strict";

const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const { test } = require("node:test");
const { MAX_REPLICA_LAG_TICKS } = require("./mm1-oracle.js");
const { NETCODE_TIMEOUT_MS, assertConnectionHeld, assertKillPathRan, assertNoResync, assertReplicaLag, longestTickGap, readServerEvidence, serverTickAt,
    serverTickIndex, unitRemovals } = require("./checkpoint-lib.js");

const T0 = 1794490000000; // a wall clock in ms
// A tick log of ticks 1..N. The tick k ends at T0 + 50 k ms, plus the listed extra stalls. Two unit rows for each tick.
function tickLog(ticks, stalls = {}) {
    const lines = [];
    let wall = T0;
    for (let tick = 1; tick <= ticks; tick += 1) {
        wall += 50 + (stalls[tick] || 0);
        for (const unit of [1, 2]) lines.push(`${tick},${BigInt(Math.round(wall)) * 1000000n},${unit},1.5,2.5`);
    }
    return `${lines.join("\n")}\n`;
}
const checkpoint = (replicaTick, pageTimeMs, extra = {}) => ({ input_call: 7, window: 1, page_time_ms: pageTimeMs,
    debug: { replicated_world: { tick: replicaTick }, ...extra } });

test("the server tick index keeps one entry for each tick and finds the last tick that ended by a time", () => {
    const index = serverTickIndex(tickLog(100));
    assert.equal(index.length, 100);
    assert.equal(index[0].tick, 1);
    assert.ok(Math.abs(index[0].wall_ms - (T0 + 50)) < 0.001, "the wall clock of a tick, from ns to ms");
    assert.equal(serverTickAt(index, T0 + 50 * 40), 40);
    assert.equal(serverTickAt(index, T0 + 50 * 40 + 49), 40, "a tick that is still running is not counted");
    assert.equal(serverTickAt(index, T0 + 49), null, "no tick before the first one");
    assert.equal(serverTickAt(index, T0 + 1e9), 100);
});

test("the replica lag bound is red for a stand-still replica and green for a replica at the server pace (the build without the stall repair)", () => {
    const index = serverTickIndex(tickLog(600));
    const at = (tick) => T0 + 50 * tick + 10;
    assert.equal(assertReplicaLag(checkpoint(497, at(500)), index, 3, "MM1 x"), 3, "lag 3 is inside the bound of 3");
    assert.equal(assertReplicaLag(checkpoint(500, at(500)), index, 3, "MM1 x"), 0);
    // The stall: the replica stands at 478 when the server is at 500 (22 ticks behind, the RUN142 size).
    assert.throws(() => assertReplicaLag(checkpoint(478, at(500)), index, 3, "MM1 mm1-after-right.png"),
        /MM1 mm1-after-right\.png: the replica stands 22 ticks behind the server \(replica tick 478, server tick 500\); the bound is 3/);
    assert.throws(() => assertReplicaLag(checkpoint(496, at(500)), index, 3, "MM1 x"), /stands 4 ticks behind/, "lag 4 is outside the bound of 3");
    assert.throws(() => assertReplicaLag(checkpoint(500, null), index, 3, "MM1 x"), /no browser clock/);
    assert.throws(() => assertReplicaLag({ page_time_ms: at(500), debug: {} }, index, 3, "MM1 x"), /no replica tick/);
    assert.throws(() => assertReplicaLag(checkpoint(5, T0), index, 3, "MM1 x"), /no tick at or before/);
});

// The evidence of a run: the tick index, the stats file (or null) and a load average.
const evidenceOf = (log, stats = null, load = [12.5, 9.25, 7]) => ({ index: serverTickIndex(log), removals: unitRemovals(log), stats, load });

test("a lost connection with a long server stall and a small tick compute time is a host fault (run rereview-2df2393-hold-a)", () => {
    const stalled = tickLog(600, { 457: 17850 });
    assert.equal(longestTickGap(serverTickIndex(stalled)).before_tick, 457);
    const lost = [checkpoint(100, T0 + 6000), checkpoint(451, T0 + 30000, { reconnect_required: true })];
    assert.throws(() => assertConnectionHeld(lost, evidenceOf(stalled, { tick_max_us: 12000 }), "ORD4"), (error) => {
        assert.match(error.message, /^ORD4: the client lost its connection \(reconnect_required\) at checkpoint 7, replica tick 451; longest server tick gap 17\.9 s before tick 457 \(netcode timeout 10 s\); /);
        assert.match(error.message, /slowest tick compute 12\.0 ms \(tick_max_us of --stats\); load average of the host at judgement 12\.50 9\.25 7\.00; host fault: the server was not scheduled/);
        assert.doesNotMatch(error.message, /product suspect/);
        return true;
    });
});

test("a lost connection with a tick that computed for most of the gap is a slow server tick, a product suspect", () => {
    const stalled = tickLog(600, { 457: 17850 });
    const lost = [checkpoint(451, T0 + 30000, { reconnect_required: true })];
    assert.throws(() => assertConnectionHeld(lost, evidenceOf(stalled, { tick_max_us: 17000000 }), "ORD4"), (error) => {
        assert.match(error.message, /slowest tick compute 17000\.0 ms/);
        assert.match(error.message, /slow server tick: a product suspect/);
        assert.doesNotMatch(error.message, /host fault/);
        return true;
    });
    // The share is half of the gap: 8.9 s of 17.9 s is a slow tick, 8.8 s is not.
    assert.throws(() => assertConnectionHeld(lost, evidenceOf(stalled, { tick_max_us: 8950000 }), "ORD4"), /slow server tick/);
    assert.throws(() => assertConnectionHeld(lost, evidenceOf(stalled, { tick_max_us: 8800000 }), "ORD4"), /host fault/);
});

test("a lost connection with no stats file, or with no server stall, is undecided and a product suspect, never a host fault", () => {
    const lost = [checkpoint(451, T0 + 30000, { reconnect_required: true })];
    assert.throws(() => assertConnectionHeld(lost, evidenceOf(tickLog(600, { 457: 17850 })), "ORD4"),
        /no server-stats\.json: the tick compute time is unknown; load average .*stall of the server with unknown tick compute time: cause undecided, a product suspect/);
    assert.throws(() => assertConnectionHeld(lost, evidenceOf(tickLog(600), { tick_max_us: 800 }), "ORD4"),
        /longest server tick gap 0\.1 s before tick 2 .*connection lost with no server stall: cause undecided, a product suspect/);
    // No load average is given: the text has no load part.
    assert.throws(() => assertConnectionHeld(lost, { ...evidenceOf(tickLog(600), { tick_max_us: 800 }), load: null }, "ORD4"), (error) => !/load average/.test(error.message));
    // A run that kept its connection passes, also when the server stalled a little.
    assert.doesNotThrow(() => assertConnectionHeld([checkpoint(100, T0 + 6000, { reconnect_required: false })], evidenceOf(tickLog(600, { 457: 17850 })), "ORD4"));
});

test("readServerEvidence reads the tick log and the stats file, and tolerates a missing stats file", async () => {
    const log = tickLog(10);
    const view = (files) => ({ serverFile: async (name) => { if (!(name in files)) throw new Error(`ENOENT ${name}`); return files[name]; } });
    const withStats = await readServerEvidence(view({ "server-tick.log": log, "server-stats.json": JSON.stringify({ tick_max_us: 321 }) }), [1, 2, 3]);
    assert.equal(withStats.stats.tick_max_us, 321);
    assert.deepEqual(withStats.load, [1, 2, 3]);
    assert.equal(withStats.index.length, 10);
    assert.equal((await readServerEvidence(view({ "server-tick.log": log }))).stats, null);
});

test("the netcode timeout of the checker is the client value in the source", () => {
    const source = fs.readFileSync(path.join(__dirname, "..", "..", "crates", "turts_client", "src", "join.rs"), "utf8");
    const match = /client_timeout_secs: (\d+),/.exec(source);
    assert.ok(match, "crates/turts_client/src/join.rs sets client_timeout_secs");
    assert.equal(Number(match[1]) * 1000, NETCODE_TIMEOUT_MS);
});

test("a resync request anywhere in a local run is red, also when no capture fell inside the stall (the build without the stall repair)", () => {
    const ok = (window, resync, tick = 100, call = 3) => ({ input_call: call, window, debug: { ingest: { resync_requests_sent: resync }, replicated_world: { tick } } });
    assert.doesNotThrow(() => assertNoResync([ok(1, 0), ok(2, 0), ok(1, 0)], "MM1", evidenceOf(tickLog(300))));
    // Window 1 stands at tick 100 here: the later captures read the repaired pace, the counter keeps the stall.
    assert.throws(() => assertNoResync([ok(1, 0), ok(1, 12), ok(2, 0)], "MM1"),
        /MM1: the replica of window 1 asked the server for 12 resyncs by checkpoint 3 \(replica tick 100\)/);
    assert.throws(() => assertNoResync([ok(1, 0), ok(2, 1)], "MM1"), /window 2 asked the server for 1 resyncs/);
    assert.throws(() => assertNoResync([{ input_call: 4, window: 1, debug: {} }], "MM1"), /has no ingest\.resync_requests_sent/);
});

test("the resync text names the first non-zero checkpoint, the nearest unit removal and the longest server tick gap (M3)", () => {
    // Unit 3 is removed after tick 105 and unit 4 after tick 400. The tick gap of 6 s comes before tick 200.
    const lines = [];
    let wall = T0;
    for (let tick = 1; tick <= 450; tick += 1) {
        wall += 50 + (tick === 200 ? 5950 : 0);
        for (const unit of [1, 2, 3, 4]) {
            if ((unit === 3 && tick > 105) || (unit === 4 && tick > 400)) continue;
            lines.push(`${tick},${BigInt(wall) * 1000000n},${unit},1,1`);
        }
    }
    const evidence = evidenceOf(`${lines.join("\n")}\n`);
    assert.deepEqual(evidence.removals, [{ unit: 3, last_tick: 105 }, { unit: 4, last_tick: 400 }]);
    const ok = (resync, tick, call) => ({ input_call: call, window: 1, debug: { ingest: { resync_requests_sent: resync }, replicated_world: { tick } } });
    assert.throws(() => assertNoResync([ok(0, 90, 10), ok(0, 100, 11), ok(7, 130, 12), ok(9, 160, 13)], "MM1", evidence), (error) => {
        assert.match(error.message, /asked the server for 7 resyncs by checkpoint 12 \(replica tick 130\)/);
        assert.match(error.message, /first non-zero count at checkpoint 12, replica tick 130 \(the checkpoint before it, of the same window, was at tick 100 with none\)/);
        assert.match(error.message, /nearest unit removal: unit 3, last tick 105 \(-25 ticks from the replica tick\)/);
        assert.match(error.message, /longest server tick gap 6\.00 s before tick 200/);
        return true;
    });
    assert.throws(() => assertNoResync([ok(2, 50, 5)], "MM1", evidenceOf(tickLog(60))), /no unit left the server tick log; longest server tick gap 0\.05 s/);
});

test("the MM1 kill rule: an enemy unit must leave the server tick log before the after-right capture, else the run is void (M1)", () => {
    // Units 1 and 2 stay; enemy unit 9 has its last tick at 507; the after-right capture is at server tick 541.
    const lines = [];
    let wall = T0;
    for (let tick = 1; tick <= 600; tick += 1) {
        wall += 50;
        for (const unit of [1, 2, 9, 10]) { if (unit === 9 && tick > 507) continue; lines.push(`${tick},${BigInt(wall) * 1000000n},${unit},1,1`); }
    }
    const evidence = evidenceOf(`${lines.join("\n")}\n`);
    const right = { page_time_ms: T0 + 50 * 541 + 10 };
    assert.deepEqual(assertKillPathRan(new Set([9, 10]), evidence.removals, evidence.index, right, "MM1"), [{ unit: 9, last_tick: 507 }]);
    // The after-right capture came before the kill (server tick 441 < 507): the kill path did not run in time.
    assert.throws(() => assertKillPathRan(new Set([9, 10]), evidence.removals, evidence.index, { page_time_ms: T0 + 50 * 441 + 10 }, "MM1"),
        /MM1: void run, the kill path did not run: no enemy unit \(9,10\) left the server tick log before server tick 441/);
    // A removal of a unit that is not an enemy does not count.
    assert.throws(() => assertKillPathRan(new Set([10]), evidence.removals, evidence.index, right, "MM1"), /the kill path did not run/);
    // No unit leaves the log in the whole run (run wAYdrM of the re-check).
    const none = evidenceOf(tickLog(600));
    assert.deepEqual(none.removals, []);
    assert.throws(() => assertKillPathRan(new Set([1, 2]), none.removals, none.index, right, "MM1"), /the kill path did not run/);
});

test("unitRemovals lists the units whose last line is before the last tick, in order", () => {
    assert.deepEqual(unitRemovals("1,100,5,0,0\n1,100,6,0,0\n2,200,5,0,0\n2,200,6,0,0\n3,300,6,0,0\n"), [{ unit: 5, last_tick: 2 }]);
    assert.deepEqual(unitRemovals(""), []);
});

test("the MM1 lag bound of 6 is green at a lag of 6 and red at a lag of 7 (clean captures read 0 to 3, stalled runs 11 to 20)", () => {
    const index = serverTickIndex(tickLog(600));
    const at = T0 + 50 * 500 + 10; // the server is at tick 500
    assert.equal(MAX_REPLICA_LAG_TICKS, 6);
    assert.equal(assertReplicaLag(checkpoint(494, at), index, MAX_REPLICA_LAG_TICKS, "MM1 x"), 6);
    assert.throws(() => assertReplicaLag(checkpoint(493, at), index, MAX_REPLICA_LAG_TICKS, "MM1 x"), /stands 7 ticks behind the server .* the bound is 6/);
    assert.equal(assertReplicaLag(checkpoint(497, at), index, MAX_REPLICA_LAG_TICKS, "MM1 x"), 3, "the largest clean lag is inside the bound");
    assert.throws(() => assertReplicaLag(checkpoint(489, at), index, MAX_REPLICA_LAG_TICKS, "MM1 x"), /stands 11 ticks behind/, "the smallest stalled lag is outside it");
});
