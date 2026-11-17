// MS spike page driver. Same call order as ms_native (sim, script, apply for each tick).
// URL: index.html?wasm=<module>&mode=suite|probes&kind=<code>&run=<n>&mutant=<code>
'use strict';
const KINDS = { none: 0, rust: 1, rhai: 2, rune: 3, lua: 4 };
const WORKLOADS = [
    { name: 'W1', warm: 200, meas: 2000 },
    { name: 'W2', warm: 200, meas: 2000 },
    { name: 'W3', warm: 200, meas: 2000 },
    { name: 'W4', warm: 50, meas: 500 },
];
const PROBES = ['L1', 'L2', 'L3a', 'L3b', 'L3c-map', 'L3c-list', 'L3d', 'L3e', 'L4', 'L5b',
    'L7a', 'L7b', 'L8', 'L9a', 'L9b'];

let importCalls = 0;
function stubImports(module) {
    const imp = {};
    for (const d of WebAssembly.Module.imports(module)) {
        imp[d.module] = imp[d.module] || {};
        if (d.kind === 'function') {
            imp[d.module][d.name] = () => { importCalls++; throw new Error('import ' + d.name + ' denied'); };
        }
    }
    return imp;
}

async function instantiate(module) {
    const inst = await WebAssembly.instantiate(module, stubImports(module));
    return inst.exports;
}

function text(ex, len) {
    const ptr = ex.ms_out_ptr();
    return new TextDecoder().decode(new Uint8Array(ex.memory.buffer, ptr, len));
}

function timerResolutionMs() {
    let min = Infinity;
    let last = performance.now();
    for (let i = 0; i < 200000; i++) {
        const t = performance.now();
        if (t > last) { min = Math.min(min, t - last); last = t; }
    }
    return min;
}

function r3(x) { return Math.round(x * 1000) / 1000; }

async function suite(module, kind, run, mutant) {
    const ex = await instantiate(module);
    ex.ms_set_mutant(mutant);
    const memBefore = ex.memory.buffer.byteLength;
    const liveBefore = ex.ms_live();
    const t = performance.now();
    const st = ex.ms_bench_new(kind);
    const createUs = (performance.now() - t) * 1000;
    if (st !== 0) throw new Error('create: ' + text(ex, st));
    const parts = [];
    let peakW1W2 = 0, memAfterW2 = 0;
    for (let w = 0; w < WORKLOADS.length; w++) {
        const W = WORKLOADS[w];
        ex.ms_select(w);
        const sim = [], scr = [], app = [];
        for (let i = 0; i < W.warm + W.meas; i++) {
            const t0 = performance.now();
            ex.ms_sim();
            const t1 = performance.now();
            ex.ms_script(w);
            const t2 = performance.now();
            ex.ms_apply();
            const t3 = performance.now();
            if (i >= W.warm) {
                sim.push(r3((t1 - t0) * 1000)); scr.push(r3((t2 - t1) * 1000)); app.push(r3((t3 - t2) * 1000));
            }
        }
        if (w === 1) { peakW1W2 = ex.ms_instance_peak(); memAfterW2 = ex.memory.buffer.byteLength; }
        parts.push(`"${W.name}":{"warmup":${W.warm},"sim_us":${JSON.stringify(sim)},"script_us":${JSON.stringify(scr)},"apply_us":${JSON.stringify(app)}}`);
    }
    const fields = text(ex, ex.ms_report());
    return `{"target":"chrome","mutant":${mutant},"run":${run},"create_us":${r3(createUs)},"instance_peak_w1w2_bytes":${peakW1W2},"process_live_before_bytes":${liveBefore},"wasm_memory_before_bytes":${memBefore},"wasm_memory_after_w2_bytes":${memAfterW2},"wasm_memory_end_bytes":${ex.memory.buffer.byteLength},"timer_resolution_ms":${timerResolutionMs()},"cross_origin_isolated":${self.crossOriginIsolated},"import_calls":${importCalls},${fields},"workloads":{${parts.join(',')}}}`;
}

async function probes(module, kind, mutant, only) {
    const out = [];
    for (let i = 0; i < PROBES.length; i++) {
        if (only && PROBES[i] !== only) continue;
        const id = PROBES[i];
        let ex;
        try {
            ex = await instantiate(module);
            ex.ms_set_mutant(mutant);
            if (id === 'L1') {
                const prep = text(ex, ex.ms_l1_prepare(kind));
                const t = performance.now();
                ex.ms_l1_call();
                const stop = performance.now() - t;
                const fin = text(ex, ex.ms_l1_finish(kind));
                out.push(`{"target":"chrome","stop_ms":${r3(stop)},"prepare":${prep},"result":${fin}}`);
            } else {
                // probes::RUN_ORDER has L1 at 0, so index i matches.
                out.push(`{"target":"chrome","result":${text(ex, ex.ms_probe(kind, i))}}`);
            }
        } catch (e) {
            out.push(JSON.stringify({ target: 'chrome', result: { probe: id, pass: false, trap: String(e) } }));
        }
    }
    return `{"import_calls":${importCalls},"probes":[${out.join(',')}]}`;
}

(async () => {
    const q = new URLSearchParams(location.search);
    try {
        const module = await WebAssembly.compileStreaming(fetch(q.get('wasm') + '.wasm'));
        const kind = KINDS[q.get('kind')];
        const res = q.get('mode') === 'probes'
            ? await probes(module, kind, Number(q.get('mutant') || 0), q.get('only'))
            : await suite(module, kind, Number(q.get('run') || 0), Number(q.get('mutant') || 0));
        window.__result = res;
    } catch (e) {
        window.__result = JSON.stringify({ error: String(e && e.stack || e) });
    }
    window.__done = true;
})();
