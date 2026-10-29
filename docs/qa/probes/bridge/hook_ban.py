#!/usr/bin/env python3
"""Probe: a player-path browser test may use page.mouse / page.keyboard and screenshots only.
Red when it calls a window.turts_* bridge (directly or through a helper in the same dir) to
send orders or to find where a control is."""
import re, sys, os
BRIDGE = re.compile(r"turts_(move|ui|scene|debug)|readBridge|clickRenderedUiAction|findVisibleUiAction")
red = 0
for path in sys.argv[1:]:
    for n, line in enumerate(open(path, encoding="utf-8"), 1):
        if BRIDGE.search(line) and not line.lstrip().startswith("//"):
            print(f"RED {os.path.basename(path)}:{n}: {line.strip()[:110]}")
            red += 1
print("VERDICT", "red" if red else "green", f"({red} bridge uses)")
sys.exit(1 if red else 0)
