#!/usr/bin/env python3
"""Make ground-only browser comparisons and preserve the existing battle workloads."""
import argparse
import io
from pathlib import Path
import shutil
import tarfile


def read(path):
    with tarfile.open(path) as archive:
        return {m.name: archive.extractfile(m).read() for m in archive if m.isfile()}


def write(path, files):
    with tarfile.open(path, 'w') as archive:
        for name, data in sorted(files.items()):
            item = tarfile.TarInfo(name)
            item.size = len(data)
            item.mode = 0o644
            archive.addfile(item, io.BytesIO(data))


def ground_span(text):
    start = text.index('ground:')
    opening = text.index('(', start)
    depth = 0
    quoted = False
    escaped = False
    for i in range(opening, len(text)):
        char = text[i]
        if quoted:
            if escaped:
                escaped = False
            elif char == '\\':
                escaped = True
            elif char == '"':
                quoted = False
        elif char == '"':
            quoted = True
        elif char == '(':
            depth += 1
        elif char == ')':
            depth -= 1
            if depth == 0:
                return start, i + 1
    raise ValueError('unclosed ground')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('prepared')
    parser.add_argument('battle_fixtures')
    parser.add_argument('output')
    args = parser.parse_args()
    output = Path(args.output)
    output.mkdir(parents=True, exist_ok=True)
    files = read(Path(args.prepared) / 'pack/skirmish.tar')
    authored = files['maps/field.ron'].decode()
    start, end = ground_span(authored)
    ground = authored[start:end]
    for case in ('plain', 'reference', 'textured'):
        copy = dict(files)
        text = authored
        if case == 'reference':
            a = text.index(', overlay:')
            b = text.index('\n))))', a)
            text = text[:a] + '))' + text[b + 5:]
        elif case == 'textured':
            text = text.replace('base_colour: None, tile_size: 8.0, roughness:',
                                'base_colour: Some("skirmish:ground_grass"), tile_size: 8.0, roughness:')
        copy['maps/field.ron'] = text.encode()
        write(output / (case + '.tar'), copy)
    battle = Path(args.battle_fixtures)
    for scene in ('battle400', 'battle2000'):
        copy = read(battle / (scene + '.tar'))
        text = copy['maps/field.ron'].decode()
        a, b = ground_span(text)
        copy['maps/field.ron'] = (text[:a] + ground + text[b:]).encode()
        copy['manifest.ron'] = copy['manifest.ron'].replace(b'^0.2.10', b'^0.2.11')
        write(output / (scene + '.tar'), copy)
        # The measurement driver repacks this directory with the actual release packer.
        for name, data in copy.items():
            relative = Path(name)
            if relative.is_absolute() or '..' in relative.parts:
                raise ValueError('unsafe fixture path: ' + name)
            target = output / scene / relative
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_bytes(data)
        shutil.copyfile(battle / (scene + '-commands.json'), output / (scene + '-commands.json'))


if __name__ == '__main__':
    main()
