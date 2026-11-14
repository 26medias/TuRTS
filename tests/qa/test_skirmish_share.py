"""Run the host controller with owned fake tunnel/server processes and real sockets."""
import importlib.util
import json
from pathlib import Path
import os
import tempfile
import subprocess
import sys
import unittest
from unittest.mock import patch
from urllib.request import urlopen

ROOT = Path(__file__).resolve().parents[2]

def load(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module

share = load("skirmish_share", ROOT / "scripts/skirmish_share.py")
urls = load("play_url", ROOT / "scripts/play_url.py")


class SharedHostTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.prepared = self.root / "prepared"
        (self.prepared / "bin").mkdir(parents=True)
        self.tunnel = self.root / "cloudflared"
        self.tunnel.write_text('''#!/usr/bin/env python3
import json, os, pathlib, signal, sys, time
args=sys.argv[1:]; out=pathlib.Path(args[args.index('--config')+1]).parent
(out/(str(os.getpid())+'.args.json')).write_text(json.dumps(args))
if os.environ.get('FAKE_TUNNEL_FAIL'): raise SystemExit(7)
print('https://example.trycloudflare.com', flush=True)
print('Registered tunnel connection', flush=True)
while True: time.sleep(.1)
''')
        self.tunnel.chmod(0o755)
        server = self.prepared / "bin/turts_server"
        server.write_text('''#!/usr/bin/env python3
import json, pathlib, re, sys
from http.server import BaseHTTPRequestHandler, HTTPServer
text=pathlib.Path(sys.argv[1]).read_text()
values={key:json.loads(value) for key,value in re.findall(r'  (\\w+): (.+),',text) if key != 'start_policy'}
class Handler(BaseHTTPRequestHandler):
 def log_message(self,*args): pass
 def do_GET(self):
  body=json.dumps({'lockfile_hash':'test-hash','lockfile_url':values['public_url']+'/lockfile.json'}).encode()
  self.send_response(200);self.end_headers();self.wfile.write(body)
host,port=values['http_listen'].split(':');HTTPServer((host,int(port)),Handler).serve_forever()
''')
        server.chmod(0o755)
        self.host = share.SharedHost(self.prepared, self.root / "out", str(self.tunnel), timeout=2)
        self.addCleanup(self.host.close)

    def fake_public_probe(self, origin):
        return share.SharedHost.http_ready(self.host.addresses['local_http'] if origin.startswith('https:') else origin)

    def test_start_uses_public_package_origin_and_stops_all_owned_children(self):
        with patch.object(self.host, 'http_ready', side_effect=self.fake_public_probe):
            url = self.host.start()
        self.assertIn('server=https%3A%2F%2Fexample.trycloudflare.com', url)
        self.assertIn('ws=wss%3A%2F%2Fexample.trycloudflare.com', url)
        self.assertEqual(len(self.host.children), 3)
        with urlopen(self.host.addresses['local_http']+'/advertisement.json') as response:
            self.assertEqual(json.load(response)['lockfile_url'], 'https://example.trycloudflare.com/lockfile.json')
        for path in self.host.output.glob('*.args.json'):
            args = json.loads(path.read_text())
            self.assertEqual(Path(args[args.index('--config')+1]).parent, self.host.output)
            self.assertEqual(args[args.index('--metrics')+1], '127.0.0.1:0')
            self.assertIn('--no-autoupdate', args)
            self.assertNotIn('--computer-slot', args)
        pids = [p.pid for p in self.host.children]
        self.host.close()
        for pid in pids:
            with self.assertRaises(ProcessLookupError): os.kill(pid, 0)

    def test_stale_prepared_build_starts_no_hosting_process(self):
        command = subprocess.run([sys.executable, '-I', '-B', str(ROOT/'scripts/skirmish_share.py'),
                                  '--prepared', str(self.prepared), '--cloudflared', str(self.tunnel),
                                  '--out', str(self.root/'cli-out')], capture_output=True, text=True)
        self.assertEqual(command.returncode, 66, command.stderr)
        self.assertFalse((self.root/'cli-out').exists())
        self.assertNotIn('Traceback', command.stderr)

    def test_tunnel_death_is_reported_after_start(self):
        with patch.object(self.host, 'http_ready', side_effect=self.fake_public_probe):
            self.host.start()
        self.host.children[0].terminate()
        self.host.children[0].wait(timeout=2)
        with self.assertRaisesRegex(RuntimeError, 'hosting process stopped'):
            self.host.check_children()

    def test_failed_tunnel_stops_startup(self):
        with patch.dict(os.environ, {'FAKE_TUNNEL_FAIL':'1'}):
            with self.assertRaisesRegex(RuntimeError, 'code 7'): self.host.start()
        self.assertEqual(len(self.host.children), 1)
        self.assertFalse((self.host.output/'hosting.json').exists())

    def test_bad_public_response_does_not_print_a_ready_game(self):
        def probe(origin):
            return {'lockfile_hash':'wrong'} if origin.startswith('https:') else share.SharedHost.http_ready(origin)
        self.host.timeout = .5
        with patch.object(self.host, 'http_ready', side_effect=probe):
            with self.assertRaisesRegex(RuntimeError, 'matching public HTTPS'): self.host.start()
        self.assertFalse((self.host.output/'hosting.json').exists())

    def test_tunnel_url_cannot_escape_the_expected_domain(self):
        self.assertIsNone(share.TUNNEL_URL.search('https://example.trycloudflare.com.evil/'))
        self.assertIsNone(share.TUNNEL_URL.search('https://user@example.trycloudflare.com/'))
        self.assertIsNotNone(share.TUNNEL_URL.search('| https://one-two.trycloudflare.com |'))


class JoinUrlTests(unittest.TestCase):
    def test_public_join_requires_https_and_rejects_ambiguous_addresses(self):
        for url in ['https://game.example/?server=x&ws=y', 'https://127.0.0.1:8443/', 'https://[::1]/']:
            self.assertTrue(urls.valid_url(url, '--join'), url)
        for url in ['http://game.example/', 'https://user:pass@game.example/', 'https://game.example/#x',
                    'https://game.example:0/', 'https://game.example:99999/', 'https://bad host/',
                    'https://game.example\\@evil/', 'https://game.example\n.evil/', 'file:///tmp/game', 'https://-bad/']:
            self.assertFalse(urls.valid_url(url, '--join'), url)

    def test_tool_url_stays_local(self):
        self.assertTrue(urls.valid_url('http://localhost:1234/?ws=x', '--url'))
        self.assertTrue(urls.valid_url('http://127.0.0.1:1234/', '--url'))
        self.assertFalse(urls.valid_url('https://game.example/', '--url'))


if __name__ == '__main__':
    unittest.main()
