#!/usr/bin/env python3
"""Create deterministic gzip sidecars for the browser's largest text and code assets."""
import argparse
import gzip
import hashlib
import io
import json
from pathlib import Path


def compress_dist(directory):
    rows = []
    for source in sorted(directory.rglob('*')):
        if not source.is_file() or source.suffix not in ('.wasm', '.js', '.css'):
            continue
        raw = source.read_bytes()
        output = io.BytesIO()
        with gzip.GzipFile(filename='', mode='wb', fileobj=output, compresslevel=6, mtime=0) as encoded:
            encoded.write(raw)
        data = output.getvalue()
        sidecar = source.with_name(source.name + '.gz')
        if len(data) < len(raw):
            sidecar.write_bytes(data)
            encoded_size = len(data)
        else:
            sidecar.unlink(missing_ok=True)
            encoded_size = None
        rows.append({'file': source.relative_to(directory).as_posix(), 'raw_bytes': len(raw),
                     'gzip_bytes': encoded_size, 'sha256': hashlib.sha256(raw).hexdigest()})
    return rows


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('directory', type=Path)
    args = parser.parse_args()
    if not args.directory.is_dir():
        parser.error('The browser output directory does not exist.')
    print(json.dumps(compress_dist(args.directory), indent=2))


if __name__ == '__main__':
    main()
