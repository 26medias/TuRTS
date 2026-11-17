#!/usr/bin/env python3
"""Print one line for each probe record in a directory (native or Chrome)."""
import json, os, sys

d = sys.argv[1]
for name in sorted(os.listdir(d)):
    if not name.endswith('.json'):
        continue
    base = os.path.join(d, name[:-5])
    ec = open(base + '.exit').read().strip() if os.path.exists(base + '.exit') else '?'
    out = open(base + '.stdout').read() if os.path.exists(base + '.stdout') else ''
    try:
        rec = json.load(open(os.path.join(d, name)))
    except Exception as e:  # noqa: BLE001
        print(name, 'exit', ec, 'NO-JSON', e)
        continue
    r = rec.get('result', rec)
    e = r.get('error') or {}
    msg = (e.get('message') or '')[:80] if isinstance(e, dict) else ''
    kind = e.get('kind') if isinstance(e, dict) else ''
    print(f"{r.get('kind','?'):5} {r.get('probe','?'):9} exit={ec} {'PASS' if r.get('pass') else 'FAIL'}"
          f" stop_ms={rec.get('stop_ms','-')} err={kind} peak={r.get('instance_peak_bytes','-')}"
          f" rec={r.get('recovery_pass','-')} stdout={len(out)}B {msg}")
