#!/usr/bin/env python3
"""MS aggregator (plan Task 6). Standard library only.

Reads the raw run files under OUT and writes OUT/summary.json and OUT/summary.md: the
distributions, the B1 to B11 rows of the spec for each candidate and target, the margins,
and the selection by spec rule 4.1. A missing number is a FAIL.
"""
import glob
import json
import math
import os
import subprocess
import sys

OUT = sys.argv[1]
CANDS = ["rhai", "rune", "lua"]
KINDS = ["none", "rust"] + CANDS
TARGETS = ["native", "chrome"]
MIB = 1024 * 1024
BUDGET = {
    "B1": {"native": 500.0, "chrome": 2000.0},
    "B2": {"native": 5000.0, "chrome": 15000.0},
    "B3": {"native": 1.0, "chrome": 3.0},
    "B4": 1_048_576,
    "B5": 4_194_304,
    "B6": 8 * MIB,
    "B7": 16 * MIB,
    "B9": {"native": 50.0, "chrome": 100.0},
}
PROBE_IDS = ["L1", "L2", "L3a", "L3b", "L3c-map", "L3c-list", "L3d", "L3e", "L4", "L7a", "L7b",
             "L8", "L9a", "L9b"]
TIE_ACCOUNTING = {"lua": True, "rhai": False, "rune": False}
TIE_FAMILIARITY = {"lua": 0, "rhai": 1, "rune": 2}
TIE_MATURITY = {"rhai": True, "rune": False, "lua": False}


def load(p):
    try:
        with open(p) as f:
            return json.load(f)
    except Exception:  # noqa: BLE001
        return None


def pct(xs, p):
    if not xs:
        return None
    s = sorted(xs)
    i = max(0, math.ceil(p / 100.0 * len(s)) - 1)
    return s[i]


def stats(xs):
    if not xs:
        return None
    return {"n": len(xs), "p50": pct(xs, 50), "p90": pct(xs, 90), "p99": pct(xs, 99),
            "max": max(xs), "mean": sum(xs) / len(xs)}


def suite_files(target, kind):
    files = sorted(glob.glob(f"{OUT}/{target}/{kind}-run[0-9]*.json"))
    recs = []
    for p in files:
        d = load(p)
        if d is None:
            recs.append(None)
            continue
        if target == "chrome":
            d = d.get("data", {}) | {"_chrome": d.get("chrome"), "_console": d.get("console_messages")}
        recs.append(d)
    return recs


summary = {"runs": {}, "dist": {}, "rows": {}, "notes": []}
suites = {t: {k: suite_files(t, k) for k in KINDS} for t in TARGETS}

# Distributions: pooled over runs, and the p99 of each run.
for t in TARGETS:
    for k in KINDS:
        recs = [r for r in suites[t][k] if r and "workloads" in r]
        summary["runs"][f"{t}/{k}"] = {
            "files": len(suites[t][k]), "valid": len(recs),
            "errors": [r.get("errors") for r in recs],
            "first_error": [r.get("first_error") for r in recs if r.get("first_error")],
            "hash_w1": sorted({r.get("hash_w1") for r in recs}),
            "hash_w2": sorted({r.get("hash_w2") for r in recs}),
            "instance_peak_w1w2_bytes": [r.get("instance_peak_w1w2_bytes") for r in recs],
            "wasm_memory_after_w2_bytes": [r.get("wasm_memory_after_w2_bytes") for r in recs],
            "create_us": [r.get("create_us") for r in recs],
            "timer_resolution_ms": [r.get("timer_resolution_ms") for r in recs],
            "cross_origin_isolated": [r.get("cross_origin_isolated") for r in recs],
            "import_calls": [r.get("import_calls") for r in recs],
            "console": [r.get("_console") for r in recs],
            "chrome": sorted({r.get("_chrome") for r in recs if r.get("_chrome")}),
        }
        for w in ["W1", "W2", "W3", "W4"]:
            for ph in ["sim_us", "script_us", "apply_us"]:
                pooled = []
                per_run = []
                for r in recs:
                    xs = r["workloads"].get(w, {}).get(ph, [])
                    pooled += xs
                    per_run.append(pct(xs, 99))
                st = stats(pooled)
                if st:
                    st["p99_each_run"] = per_run
                summary["dist"][f"{t}/{k}/{w}/{ph}"] = st


def d(t, k, w, ph, key):
    s = summary["dist"].get(f"{t}/{k}/{w}/{ph}")
    return None if s is None else s[key]


sizes = {s["name"]: s for s in (load(f"{OUT}/meta/sizes.json") or [])}
lic_path = f"{OUT}/meta/licenses.json"
if not os.path.exists(lic_path):
    subprocess.run([sys.executable, os.path.join(os.path.dirname(__file__), "licenses.py"), OUT],
                   check=False, cwd=os.path.join(os.path.dirname(__file__), ".."))
lic = load(lic_path) or {}


def probes_native(c):
    """id -> list of (pass, record, exit, stdout_len) over runs."""
    res = {}
    for rd in sorted(glob.glob(f"{OUT}/probes/native-run[0-9]*")):
        if not os.path.isdir(rd):
            continue
        for pid in PROBE_IDS + ["L5b"]:
            base = f"{rd}/{c}-{pid}"
            rec = load(base + ".json")
            ex = open(base + ".exit").read().strip() if os.path.exists(base + ".exit") else "missing"
            so = len(open(base + ".stdout").read()) if os.path.exists(base + ".stdout") else -1
            r = (rec or {}).get("result", {})
            ok = bool(r.get("pass")) and ex == "0" and so == 0
            res.setdefault(pid, []).append({"pass": ok, "exit": ex, "stdout_bytes": so, "result": r,
                                            "stop_ms": (rec or {}).get("stop_ms")})
    return res


def probes_chrome(c):
    res = {}
    for p in sorted(glob.glob(f"{OUT}/probes/chrome-{c}-run[0-9]*.json")):
        dd = load(p)
        if not dd or "probes" not in dd.get("data", {}):
            for pid in PROBE_IDS + ["L5b"]:
                res.setdefault(pid, []).append({"pass": False, "result": {"detail": "no data"}})
            continue
        clean = not dd.get("console_messages") and dd["data"].get("import_calls") == 0
        for e in dd["data"]["probes"]:
            r = e.get("result", {})
            pid = r.get("probe")
            res.setdefault(pid, []).append({"pass": bool(r.get("pass")) and clean and "trap" not in r,
                                            "result": r, "stop_ms": e.get("stop_ms"),
                                            "console": dd.get("console_messages")})
    return res


def l1_exclusive(t, c):
    xs = []
    if t == "native":
        for p in sorted(glob.glob(f"{OUT}/native/{c}-L1-run[0-9]*.json")):
            r = load(p)
            xs.append(r.get("stop_ms") if r and r.get("result", {}).get("pass") else None)
    else:
        for p in sorted(glob.glob(f"{OUT}/chrome-l1/{c}-run[0-9]*.json")):
            dd = load(p)
            try:
                e = dd["data"]["probes"][0]
                xs.append(e.get("stop_ms") if e["result"].get("pass") else None)
            except Exception:  # noqa: BLE001
                xs.append(None)
    return xs


rows = {}
for c in CANDS:
    pn, pc = probes_native(c), probes_chrome(c)
    rows[c] = {}
    for t in TARGETS:
        R = {}
        runs_ok = all(r and r.get("errors") == 0 for r in suites[t][c]) and len(suites[t][c]) >= 5
        b1 = d(t, c, "W1", "script_us", "p99")
        b2 = d(t, c, "W2", "script_us", "p99")
        b3 = d(t, c, "W3", "script_us", "p50")
        b3 = None if b3 is None else b3 / 1000.0
        R["B1"] = (b1, BUDGET["B1"][t], runs_ok)
        R["B2"] = (b2, BUDGET["B2"][t], runs_ok)
        R["B3"] = (b3, BUDGET["B3"][t], runs_ok)
        peaks = [r.get("instance_peak_w1w2_bytes") for r in suites[t][c] if r]
        zero = [r.get("instance_peak_w1w2_bytes") for r in suites[t]["rust"] if r]
        b6 = (max(peaks) - min(zero)) if peaks and zero and None not in peaks + zero else None
        R["B6"] = (b6, BUDGET["B6"], True)
        if t == "chrome":
            mem = [r.get("wasm_memory_after_w2_bytes") for r in suites[t][c] if r]
            mz = [r.get("wasm_memory_after_w2_bytes") for r in suites[t]["rust"] if r]
            b7 = (max(mem) - min(mz)) if mem and mz and None not in mem + mz else None
            R["B7"] = (b7, BUDGET["B7"], True)
        l1 = l1_exclusive(t, c)
        b9 = max(l1) if l1 and None not in l1 else None
        R["B9"] = (b9, BUDGET["B9"][t], True)
        probes = pn if t == "native" else pc
        failed = [pid for pid in PROBE_IDS if not probes.get(pid) or not all(x["pass"] for x in probes[pid])]
        R["B8"] = {"pass": not failed and len(probes.get("L1", [])) >= 5, "failed": failed,
                   "runs": len(probes.get("L1", []))}
        # B10: L5 (a) over all runs, both targets, rust_ref; L5 (b) over runs of this target.
        ref = summary["runs"].get(f"native/rust", {})
        want = (ref.get("hash_w1"), ref.get("hash_w2"))
        mine = summary["runs"][f"{t}/{c}"]
        l5a = (mine["hash_w1"] == want[0] and mine["hash_w2"] == want[1] and len(want[0] or []) == 1
               and summary["runs"]["chrome/rust"]["hash_w1"] == want[0])
        orders = sorted({x["result"].get("order_hash") for x in probes.get("L5b", [])})
        l5b = len(orders) == 1 and None not in orders and len(probes.get("L5b", [])) >= 5 \
            and all(x["pass"] for x in probes.get("L5b", []))
        R["B10"] = {"pass": l5a and l5b, "l5a": l5a, "l5b": l5b, "order_hashes": orders}
        rows[c][t] = R
    sz, base = sizes.get(c), sizes.get("baseline")
    rows[c]["B4"] = ((sz["gzip_bytes"] - base["gzip_bytes"]) if sz and base else None, BUDGET["B4"], True)
    rows[c]["B5"] = ((sz["raw_bytes"] - base["raw_bytes"]) if sz and base else None, BUDGET["B5"], True)
    rows[c]["brotli_added"] = (sz["brotli_bytes"] - base["brotli_bytes"]) if sz and base else None
    built = bool(sz) and len(suites["native"][c]) >= 5
    rows[c]["B11"] = {"pass": built and bool(lic.get(c, {}).get("pass")),
                      "not_permissive": lic.get(c, {}).get("not_permissive"), "built": built}
    # L5 (b) across targets: recorded only.
    rows[c]["l5b_cross_target_equal"] = rows[c]["native"]["B10"]["order_hashes"] == rows[c]["chrome"]["B10"]["order_hashes"]


def num_pass(v):
    val, budget, ok = v
    return val is not None and ok and val <= budget


selection = {}
for c in CANDS:
    margins, fails = {}, []
    for t in TARGETS:
        for b in ["B1", "B2", "B3", "B6", "B7", "B9"]:
            if b not in rows[c][t]:
                continue
            v = rows[c][t][b]
            if not num_pass(v):
                fails.append(f"{b} {t}")
            if v[0] is not None:
                margins[f"{b} {t}"] = (v[1] - v[0]) / v[1]
        for b in ["B8", "B10"]:
            if not rows[c][t][b]["pass"]:
                fails.append(f"{b} {t}")
    for b in ["B4", "B5"]:
        v = rows[c][b]
        if not num_pass(v):
            fails.append(b)
        if v[0] is not None:
            margins[b] = (v[1] - v[0]) / v[1]
    if not rows[c]["B11"]["pass"]:
        fails.append("B11")
    selection[c] = {"eligible": not fails, "fails": fails, "score": min(margins.values()) if margins else None,
                    "margins": margins}

elig = sorted([c for c in CANDS if selection[c]["eligible"]], key=lambda c: -selection[c]["score"])
chosen = None
if elig:
    top = selection[elig[0]]["score"]
    tied = [c for c in elig if top - selection[c]["score"] <= 0.10]
    tied.sort(key=lambda c: (not TIE_ACCOUNTING[c], TIE_FAMILIARITY[c], not TIE_MATURITY[c]))
    chosen = tied[0] if len(tied) > 1 else elig[0]
    summary["notes"].append(f"tied within 0.10: {tied}")
summary["rows"] = rows
summary["selection"] = {"per_candidate": selection, "eligible": elig, "selected": chosen,
                        "ms_row": "PASS" if chosen else "RED"}
# Instrument check (plan): the `none` script phase holds no sim work.
summary["none_script_p99_native_us"] = d("native", "none", "W1", "script_us", "p99")
summary["none_script_p99_chrome_us"] = d("chrome", "none", "W1", "script_us", "p99")

with open(f"{OUT}/summary.json", "w") as f:
    json.dump(summary, f, indent=1, default=str)


# ---- Markdown ---------------------------------------------------------------------------------
def fmt(x, nd=1):
    if x is None:
        return "MISSING"
    if isinstance(x, float):
        return f"{x:,.{nd}f}"
    return f"{x:,}"


L = []
L.append("# MS summary (generated by tools/aggregate.py)\n")
L.append(f"MS row: **{summary['selection']['ms_row']}**. Selected: **{chosen}**. Eligible: {elig}.\n")
L.append("## Budget rows\n")
L.append("| Row | Target | " + " | ".join(CANDS) + " | Budget |")
L.append("| --- | --- | " + " | ".join("---" for _ in CANDS) + " | --- |")


def cell(v, nd=1):
    val, budget, ok = v
    mark = "pass" if num_pass(v) else "FAIL"
    extra = "" if ok else " (script errors)"
    return f"{fmt(val, nd)} {mark}{extra}"


for b, unit, nd in [("B1", "µs", 1), ("B2", "µs", 1), ("B3", "µs per call", 3), ("B6", "bytes", 0),
                    ("B9", "ms", 3)]:
    for t in TARGETS:
        L.append(f"| {b} ({unit}) | {t} | " + " | ".join(cell(rows[c][t][b], nd) for c in CANDS)
                 + f" | {fmt(rows[CANDS[0]][t][b][1], nd)} |")
L.append("| B7 (bytes) | chrome | " + " | ".join(cell(rows[c]["chrome"]["B7"], 0) for c in CANDS)
         + f" | {fmt(BUDGET['B7'])} |")
for b in ["B4", "B5"]:
    L.append(f"| {b} (bytes) | both | " + " | ".join(cell(rows[c][b], 0) for c in CANDS) + f" | {fmt(BUDGET[b])} |")
for t in TARGETS:
    L.append(f"| B8 probes | {t} | " + " | ".join(
        ("pass" if rows[c][t]["B8"]["pass"] else "FAIL: " + ", ".join(rows[c][t]["B8"]["failed"])) for c in CANDS) + " | all pass |")
    L.append(f"| B10 determinism | {t} | " + " | ".join(
        ("pass" if rows[c][t]["B10"]["pass"] else f"FAIL (L5a {rows[c][t]['B10']['l5a']}, L5b {rows[c][t]['B10']['l5b']})") for c in CANDS) + " | equal |")
L.append("| B11 build and license | both | " + " | ".join(
    ("pass" if rows[c]["B11"]["pass"] else f"FAIL {rows[c]['B11']['not_permissive']}") for c in CANDS) + " | permissive |")
L.append("")
L.append("## Selection (spec 4.1)\n")
L.append("| Candidate | Eligible | Score (smallest margin) | Failing rows |")
L.append("| --- | --- | --- | --- |")
for c in CANDS:
    s = selection[c]
    L.append(f"| {c} | {s['eligible']} | {fmt(s['score'], 3)} | {', '.join(s['fails']) or '-'} |")
L.append("")
L.append("## Distributions (pooled over 5 runs; µs)\n")
L.append("| Target | Kind | Workload | Phase | n | p50 | p90 | p99 | max | p99 of each run |")
L.append("| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |")
for t in TARGETS:
    for k in KINDS:
        for w in ["W1", "W2", "W3", "W4"]:
            for ph in ["script_us", "sim_us", "apply_us"]:
                s = summary["dist"].get(f"{t}/{k}/{w}/{ph}")
                if not s:
                    continue
                if ph != "script_us" and k not in ("none",):
                    continue
                L.append(f"| {t} | {k} | {w} | {ph[:-3]} | {s['n']} | {fmt(s['p50'], 2)} | {fmt(s['p90'], 2)} | "
                         f"{fmt(s['p99'], 2)} | {fmt(s['max'], 2)} | {', '.join(fmt(x, 1) for x in s['p99_each_run'])} |")
L.append("")
L.append("## Other records\n")
for c in CANDS:
    L.append(f"- {c}: brotli added {fmt(rows[c]['brotli_added'])} bytes; L5 (b) equal across targets: "
             f"{rows[c]['l5b_cross_target_equal']} (recorded only); order hashes native "
             f"{rows[c]['native']['B10']['order_hashes']}, chrome {rows[c]['chrome']['B10']['order_hashes']}.")
for t in TARGETS:
    for k in KINDS:
        r = summary["runs"][f"{t}/{k}"]
        L.append(f"- {t}/{k}: valid runs {r['valid']}/{r['files']}, errors {r['errors']}, hash W1 {r['hash_w1']}, "
                 f"W2 {r['hash_w2']}, instance peak {r['instance_peak_w1w2_bytes']}, wasm memory after W2 "
                 f"{r['wasm_memory_after_w2_bytes']}, create µs {r['create_us']}, timer ms {r['timer_resolution_ms']}, "
                 f"isolated {r['cross_origin_isolated']}, chrome {r['chrome']}")
L.append(f"- `none` script p99: native {fmt(summary['none_script_p99_native_us'], 3)} µs, chrome "
         f"{fmt(summary['none_script_p99_chrome_us'], 3)} µs (instrument check: script timer holds no sim work).")
with open(f"{OUT}/summary.md", "w") as f:
    f.write("\n".join(L) + "\n")
print("\n".join(L[:40]))
