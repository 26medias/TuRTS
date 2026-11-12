// turts-test-role: unit
"use strict";

const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const { test } = require("node:test");
const { NETCODE_TIMEOUT_MS, assertConnectionHeld, assertNoResync, assertReplicaLag, longestTickGap, serverTickAt, serverTickIndex } = require("./checkpoint-lib.js");

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

test("a lost connection is named as a host fault with the server stall that caused it (run rereview-2df2393-hold-a)", () => {
    const stalled = serverTickIndex(tickLog(600, { 457: 17850 }));
    assert.equal(longestTickGap(stalled).before_tick, 457);
    const lost = [checkpoint(100, T0 + 6000), checkpoint(451, T0 + 30000, { reconnect_required: true })];
    assert.throws(() => assertConnectionHeld(lost, stalled, "ORD4"),
        /ORD4: host fault, the client lost its connection \(reconnect_required\) at checkpoint 7, replica tick 451; the server tick log has a stall of 17\.9 s before tick 457, longer than the netcode timeout of 10 s/);
    // A lost connection with no long stall still names the lost connection.
    const quiet = serverTickIndex(tickLog(600));
    assert.throws(() => assertConnectionHeld(lost, quiet, "ORD4"), /the longest gap of the server tick log is 0\.1 s/);
    // A run that kept its connection passes, also when the server stalled a little.
    assert.doesNotThrow(() => assertConnectionHeld([checkpoint(100, T0 + 6000, { reconnect_required: false })], stalled, "ORD4"));
});

test("the netcode timeout of the checker is the client value in the source", () => {
    const source = fs.readFileSync(path.join(__dirname, "..", "..", "crates", "turts_client", "src", "join.rs"), "utf8");
    const match = /client_timeout_secs: (\d+),/.exec(source);
    assert.ok(match, "crates/turts_client/src/join.rs sets client_timeout_secs");
    assert.equal(Number(match[1]) * 1000, NETCODE_TIMEOUT_MS);
});

test("a resync request anywhere in a local run is red, also when no capture fell inside the stall (the build without the stall repair)", () => {
    const ok = (window, resync) => ({ input_call: 3, window, debug: { ingest: { resync_requests_sent: resync }, replicated_world: { tick: 100 } } });
    assert.doesNotThrow(() => assertNoResync([ok(1, 0), ok(2, 0), ok(1, 0)], "MM1"));
    // Window 1 stands at tick 100 here: the later captures read the repaired pace, the counter keeps the stall.
    assert.throws(() => assertNoResync([ok(1, 0), ok(1, 12), ok(2, 0)], "MM1"),
        /MM1: the replica of window 1 asked the server for 12 resyncs by checkpoint 3 \(replica tick 100\)/);
    assert.throws(() => assertNoResync([ok(1, 0), ok(2, 1)], "MM1"), /window 2 asked the server for 1 resyncs/);
    assert.throws(() => assertNoResync([{ input_call: 4, window: 1, debug: {} }], "MM1"), /has no ingest\.resync_requests_sent/);
});
