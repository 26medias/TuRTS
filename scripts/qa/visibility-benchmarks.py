#!/usr/bin/env python3
"""Prepare SV7 from the accepted mixed 400-actor workload without copying its art."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import shutil


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('accepted_fixtures', type=Path)
    parser.add_argument('output', type=Path)
    args = parser.parse_args()
    source = args.accepted_fixtures.resolve()
    source_map = (source / 'battle400/maps/field.ron').read_text()
    source_commands = source / 'battle400-commands.json'
    if len(re.findall(r'unit:\s*"skirmish:(?:warrior|swordsman|soldier)"', source_map)) != 400:
        raise ValueError('source is not the accepted three-type 400-actor battle')
    if re.search(r'\bvisibility\s*:', source_map):
        raise ValueError('source must be the accepted AllVisible baseline')
    args.output.mkdir(parents=True, exist_ok=False)
    rows = []
    for name, fog, concentrated in [('fog400', True, False), ('allvisible400', False, False), ('concentrated400', False, True)]:
        case = args.output / name
        case.mkdir()
        def copy_file(src, dst):
            if str(src).endswith('.ron'):
                return shutil.copy2(src, dst)
            try:
                os.link(src, dst)
                return dst
            except OSError:
                # Cross-device evidence folders still preserve the exact existing bytes.
                return shutil.copy2(src, dst)
        package = case / 'battle400'
        shutil.copytree(source / 'battle400', package, copy_function=copy_file)
        shutil.copy2(source_commands, case / source_commands.name)
        manifest = package / 'manifest.ron'
        manifest.write_text(re.sub(r'engine_api:\s*"[^"]+"', 'engine_api: "^0.2.16"', manifest.read_text()))
        for kind in ['warrior', 'swordsman', 'soldier']:
            unit = package / 'units' / (kind + '.ron')
            unit.write_text(unit.read_text().replace('(', '(vision: Some((radius: 8.0)), ', 1))
        text = source_map
        if fog:
            text = text.replace('(', '(visibility: Some((cell_size: 2.0, sharing: OwnSlot, memory_limit: Some(65536))), ', 1)
        if concentrated:
            counts = [0, 0]
            pattern = r'(unit:\s*"skirmish:(?:warrior|swordsman|soldier)",owner_slot:([01]),pos:)\(x:[^,]+,y:[^)]+\)'
            def reposition(match):
                slot = int(match[2])
                index = counts[slot]
                counts[slot] += 1
                return f'{match[1]}(x:{2 + (index % 10)*2.8 + slot*1.4:.5f},y:{2 + (index//10)*1.4:.5f})'
            text, changed = re.subn(pattern, reposition, text)
            if changed != 400 or counts != [200, 200]:
                raise ValueError('concentrated layout did not preserve all actor identities')
        (package / 'maps/field.ron').write_text(text)
        rows.append({'case': name, 'mode': 'fog' if fog else 'allvisible', 'actors': 400,
                     'zooms': ['normal', 'near', 'far'], 'quality': 'high',
                     'map_sha256': hashlib.sha256(text.encode()).hexdigest(),
                     'commands_sha256': hashlib.sha256(source_commands.read_bytes()).hexdigest()})
    metadata = {'accepted_source': str(source), 'source_map_sha256': hashlib.sha256(source_map.encode()).hexdigest(),
                'art': 'unchanged source bytes; hard links where the filesystem permits',
                'camera': 'authored bounds unchanged; normal view keeps the full 400-onscreen guard', 'cases': rows}
    (args.output / 'fixtures.json').write_text(json.dumps(metadata, indent=2) + '\n')
    print(json.dumps({'status': 'PREPARED_NOT_RUN', **metadata}, indent=2))


if __name__ == '__main__':
    main()
