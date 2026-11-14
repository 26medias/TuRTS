"""Independent review cases for static representation selection."""
import gzip
import os
from pathlib import Path
import unittest
import test_web_compression as authored


class IndependentLocalHttpTest(authored.LocalHttpTest):
    def test_url_escaping_and_zero_quality_from_a_second_header(self):
        source = Path(self.tmp.name) / 'space name.wasm'
        source.write_bytes(self.raw)
        source.with_name(source.name + '.gz').write_bytes(self.encoded)
        status, headers, body = self.request('gzip', path='/space%20name.wasm?probe=yes')
        self.assertEqual(status, 200)
        self.assertEqual(headers.get('content-encoding'), 'gzip')
        self.assertEqual(gzip.decompress(body), self.raw)
        status, headers, body = self.request('gzip', headers={'Accept-Encoding': 'gzip;q=0,identity;q=1'})
        self.assertEqual(status, 200)
        self.assertIsNone(headers.get('content-encoding'))
        self.assertEqual(body, self.raw)

    def test_invalid_quality_values_do_not_enable_compression(self):
        for value in ('2', '-1', 'NaN', '1.0001', '0.1234'):
            status, headers, body = self.request('gzip;q=' + value)
            self.assertEqual(status, 200)
            self.assertIsNone(headers.get('content-encoding'))
            self.assertEqual(body, self.raw)

    def test_unreadable_selected_sidecar_does_not_label_an_error_body_as_gzip(self):
        if os.name != 'posix' or os.geteuid() == 0:
            self.skipTest('This permission check needs an unprivileged POSIX process.')
        sidecar = Path(self.tmp.name) / 'app.wasm.gz'
        sidecar.chmod(0)
        try:
            status, headers, body = self.request('gzip')
            self.assertEqual(status, 404)
            self.assertIsNone(headers.get('content-encoding'), 'error HTML must not claim gzip encoding')
            self.assertIn(b'Error response', body)
        finally:
            sidecar.chmod(0o600)


if __name__ == '__main__':
    unittest.main()
