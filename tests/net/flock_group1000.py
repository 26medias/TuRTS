#!/usr/bin/env python3
"""Write the test-only package `flock1000` for the T-7 wire measure. Standard library only.

flock_group1000.py <mods/quarry_ridge folder> <new output folder>

The output is a copy of the Quarry ridge package with the id `flock1000` and one changed map:
a flat map of 256 x 256 cells with no obstacle, and 1,000 Walkers (radius 0.4) of slot 0 in a
grid of 40 x 25 with 1.2 between centres. It also writes `<output>.move.json`, a command script
with ONE Move order for all 1,000 units to a point 150 cells from the centre of the grid.
Pack the folder with `turts-pack`. This package is a test fixture, not a default mod.
"""

import json
import pathlib
import re
import shutil
import sys

COLUMNS, ROWS, SPACING = 40, 25, 1.2
ORIGIN = (20.0, 113.0)
MAP_CELLS = 256
ORDER_TICK = 100
TARGET_DISTANCE = 150.0


def main():
    source, out = pathlib.Path(sys.argv[1]), pathlib.Path(sys.argv[2])
    shutil.copytree(source, out)
    for path in out.rglob("*"):
        if path.is_file() and path.suffix in (".ron", ".md"):
            path.write_text(path.read_text().replace("quarry_ridge", "flock1000"))
    field = out / "maps" / "field.ron"
    text = field.read_text()
    points = [
        (ORIGIN[0] + column * SPACING, ORIGIN[1] + row * SPACING)
        for row in range(ROWS)
        for column in range(COLUMNS)
    ]
    placements = ",".join(
        f'(unit:"flock1000:walker",owner_slot:0,pos:(x:{x:.1f},y:{y:.1f}),contents:[])'
        for x, y in points
    )
    text, count = re.subn(r"placements:\[.*?\]\)\]", f"placements:[{placements}]", text, flags=re.S)
    assert count == 1, "placements block not found"
    for pattern, value in (
        (r"size:\(x:[0-9.]+,y:[0-9.]+\)", f"size:(x:{MAP_CELLS}.0,y:{MAP_CELLS}.0)"),
        (r"columns:\d+,rows:\d+", f"columns:{MAP_CELLS},rows:{MAP_CELLS}"),
        (r"initial_heights:\[.*?\],floor", "initial_heights:[],floor"),
        # The bands must end at the highest initial surface, which is now 0.
        (r"\(bottom:-2,top:4,", "(bottom:-2,top:0,"),
    ):
        text, count = re.subn(pattern, value, text, flags=re.S)
        assert count == 1, pattern
    field.write_text(text)
    centre_x = ORIGIN[0] + (COLUMNS - 1) * SPACING / 2
    centre_y = ORIGIN[1] + (ROWS - 1) * SPACING / 2
    order = {
        "tick": ORDER_TICK,
        "slot": 0,
        "envelope": {
            "order": "flock1000:order_move",
            "units": list(range(1, len(points) + 1)),
            "target": {"Point": {"x": centre_x + TARGET_DISTANCE, "y": centre_y}},
            "queue": "Replace",
            "group": 1,
        },
    }
    pathlib.Path(str(out) + ".move.json").write_text(json.dumps([order]))
    print(f"units {len(points)} target {order['envelope']['target']}")


if __name__ == "__main__":
    main()
