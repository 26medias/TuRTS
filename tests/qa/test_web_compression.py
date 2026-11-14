"""Static delivery must send fewer bytes without changing the decoded program."""
import functools
import gzip
import http.client
import http.server
import importlib.util
from pathlib import Path
import tempfile
import threading
import unittest

ROOT = Path(__file__).resolve().parents[2]


def module(name, path):
    spec = importlib.util.spec_from_file_location(name, ROOT / path)
    value = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(value)
    return value


compressor = module('compress_web', 'scripts/compress_web.py')
host = module('skirmish_host', 'scripts/skirmish_host.py')


class CompressionTest(unittest.TestCase):
    def test_sidecars_keep_the_decoded_program_and_repeat_exactly(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            raw = b'\0asm' + b'game-code;' * 1000
            (root / 'app.wasm').write_bytes(raw)
            (root / 'small.js').write_bytes(b'x')
            (root / 'small.js.gz').write_bytes(b'old')
            rows = compressor.compress_dist(root)
            first = (root / 'app.wasm.gz').read_bytes()
            self.assertEqual(gzip.decompress(first), raw)
            self.assertEqual((root / 'app.wasm').read_bytes(), raw)
            compressor.compress_dist(root)
            self.assertEqual((root / 'app.wasm.gz').read_bytes(), first)
            self.assertFalse((root / 'small.js.gz').exists())
            self.assertLess(rows[0]['gzip_bytes'], rows[0]['raw_bytes'])


class LocalHttpTest(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        root = Path(self.tmp.name)
        self.raw = b'\0asm' + b'same-program;' * 1000
        self.encoded = gzip.compress(self.raw, mtime=0)
        (root / 'app.wasm').write_bytes(self.raw)
        (root / 'app.wasm.gz').write_bytes(self.encoded)
        (root / 'plain.css').write_text('x{}')

        class Quiet(host.Handler):
            def log_message(self, *_args):
                pass

        self.server = http.server.ThreadingHTTPServer(('127.0.0.1', 0),
            functools.partial(Quiet, directory=str(root)))
        self.thread = threading.Thread(target=self.server.serve_forever, daemon=True)
        self.thread.start()

    def tearDown(self):
        self.server.shutdown()
        self.server.server_close()
        self.thread.join()
        self.tmp.cleanup()

    def request(self, encoding=None, method='GET', path='/app.wasm', headers=None):
        connection = http.client.HTTPConnection(*self.server.server_address, timeout=3)
        connection.putrequest(method, path, skip_accept_encoding=True)
        if encoding is not None:
            connection.putheader('Accept-Encoding', encoding)
        for name, value in (headers or {}).items():
            connection.putheader(name, value)
        connection.endheaders()
        response = connection.getresponse()
        result = response.status, {name.lower(): value for name, value in response.getheaders()}, response.read()
        connection.close()
        return result

    def test_gzip_get_head_query_and_conditional_keep_type_and_lengths(self):
        for method in ('GET', 'HEAD'):
            status, headers, body = self.request('gzip', method, '/app.wasm?version=1')
            self.assertEqual(status, 200)
            self.assertEqual(headers.get('content-encoding'), 'gzip')
            self.assertEqual(headers['content-type'], 'application/wasm')
            self.assertEqual(int(headers['content-length']), len(self.encoded))
            self.assertIn('accept-encoding', headers.get('vary', '').lower())
            self.assertEqual(body, self.encoded if method == 'GET' else b'')
        status, _, body = self.request('gzip', headers={'If-Modified-Since': headers['last-modified']})
        self.assertEqual(status, 304)
        self.assertEqual(body, b'')

    def test_quality_preferences_and_identity_fallback(self):
        cases = [(None, False), ('', False), ('br', False), ('gzip', True),
                 ('GZIP;Q=1', True), ('gzip;q=0', False), ('gzip;q=bad', False),
                 ('*;q=1', True), ('gzip;q=0,*;q=1', False),
                 ('identity;q=1,gzip;q=0.5', False), ('identity;q=0,gzip;q=1', True)]
        for encoding, compressed in cases:
            with self.subTest(encoding=encoding):
                status, headers, body = self.request(encoding)
                self.assertEqual(status, 200)
                self.assertEqual(headers.get('content-encoding'), 'gzip' if compressed else None)
                self.assertEqual(gzip.decompress(body) if compressed else body, self.raw)
                self.assertIn('accept-encoding', headers.get('vary', '').lower())
        status, _, body = self.request('gzip', path='/plain.css')
        self.assertEqual(status, 200)
        self.assertEqual(body, b'x{}')
        self.assertEqual(self.request('gzip;q=0,identity;q=0')[0], 406)
        self.assertEqual(self.request('*;q=0')[0], 406)


if __name__ == '__main__':
    unittest.main()
