#!/usr/bin/env python3
"""Check a real native HTTP server against the prepared browser bytecode."""
import argparse
import gzip
import hashlib
import json
from pathlib import Path
import socket
import subprocess
import time
import urllib.request


def port():
    with socket.socket() as sock:
        sock.bind(('127.0.0.1', 0))
        return sock.getsockname()[1]


def run(prepared, output):
    output.mkdir(parents=True, exist_ok=True)
    http, ws = port(), port()
    url = f'http://127.0.0.1:{http}'
    config = output / 'server.ron'
    config.write_text(f'(game_package:"{prepared}/pack/skirmish.tar",http_listen:"127.0.0.1:{http}",ws_listen:"127.0.0.1:{ws}",public_url:"{url}",tick_hz:20,static_dir:"{prepared}/dist")')
    result = {'status': 'RUNNING', 'rows': []}
    with (output / 'server.log').open('w') as log:
        process = subprocess.Popen([str(prepared / 'bin/turts_server'), str(config)], stdout=log, stderr=subprocess.STDOUT)
        try:
            deadline = time.monotonic() + 10
            while True:
                try:
                    with urllib.request.urlopen(url + '/advertisement.json', timeout=.2):
                        break
                except OSError:
                    if process.poll() is not None or time.monotonic() >= deadline:
                        raise
                    time.sleep(.02)
            source = next((prepared / 'dist').glob('*.wasm'))
            raw_hash = hashlib.sha256(source.read_bytes()).hexdigest()
            result['raw_bytes'] = source.stat().st_size
            for encoding, method in [('gzip', 'GET'), ('gzip;q=0', 'GET'), ('identity', 'GET'), ('gzip', 'HEAD')]:
                request = urllib.request.Request(url + '/' + source.name + '?probe=1', method=method,
                                                 headers={'Accept-Encoding': encoding})
                with urllib.request.urlopen(request, timeout=20) as response:
                    headers = {key.lower(): value for key, value in response.headers.items()}
                    body = response.read()
                    compressed = encoding == 'gzip'
                    if headers.get('content-encoding') != ('gzip' if compressed else None):
                        raise AssertionError(f'ZIP3 {encoding} {method}: wrong Content-Encoding')
                    if headers.get('content-type', '').split(';')[0] != 'application/wasm':
                        raise AssertionError('ZIP3 wrong WASM MIME')
                    if 'accept-encoding' not in headers.get('vary', '').lower():
                        raise AssertionError('ZIP3 missing Vary')
                    if method == 'GET':
                        decoded = gzip.decompress(body) if compressed else body
                        if hashlib.sha256(decoded).hexdigest() != raw_hash:
                            raise AssertionError('ZIP3 decoded program changed')
                        if len(body) != int(headers['content-length']):
                            raise AssertionError('ZIP3 incorrect wire length')
                    elif body:
                        raise AssertionError('ZIP3 HEAD returned a body')
                    result['rows'].append({'encoding': encoding, 'method': method, 'status': response.status,
                                           'content_encoding': headers.get('content-encoding'),
                                           'content_length': int(headers['content-length']), 'decoded_sha256': raw_hash})
            result['status'] = 'PASS'
        except Exception as error:
            result['status'] = 'FAIL'
            result['error'] = str(error)
            raise
        finally:
            process.terminate()
            try:
                process.wait(timeout=5)
            except subprocess.TimeoutExpired:
                process.kill()
                process.wait()
            (output / 'result.json').write_text(json.dumps(result, indent=2))
    print(json.dumps(result, indent=2))


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('prepared', type=Path)
    parser.add_argument('output', type=Path)
    args = parser.parse_args()
    run(args.prepared.resolve(), args.output.resolve())
