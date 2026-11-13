#!/usr/bin/env python3
"""Copy the supplied animated art and size the Crossbowman prop for the idle pose."""
import argparse
import json
from pathlib import Path
import struct

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('source', type=Path)
parser.add_argument('destination', type=Path)
args = parser.parse_args()
args.destination.mkdir(parents=True, exist_ok=True)
for name in ['soldier_anim.glb', 'worker_anim.glb']:
    data = (args.source / name).read_bytes()
    if name == 'soldier_anim.glb':
        length = struct.unpack_from('<I', data, 12)[0]
        document = json.loads(data[20:20 + length])
        prop = next(node for node in document['nodes'] if node.get('name') == 'KEEP_2H_Crossbow')
        prop['scale'] = [value * 0.5 for value in prop.get('scale', [1, 1, 1])]
        encoded = json.dumps(document, separators=(',', ':')).encode()
        encoded += b' ' * (-len(encoded) % 4)
        remaining = data[20 + length:]
        data = (b'glTF' + struct.pack('<III', 2, 20 + len(encoded) + len(remaining), len(encoded))
                + b'JSON' + encoded + remaining)
    (args.destination / name).write_bytes(data)
