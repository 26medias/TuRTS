"""The local retry starts a fresh server and leaves no child behind."""
import json
from pathlib import Path
import re
import signal
import socket
import subprocess
import sys
import tempfile
import time
import unittest
import urllib.error
import urllib.request

ROOT = Path(__file__).resolve().parents[2]


def port():
    with socket.socket() as sock:
        sock.bind(('127.0.0.1', 0))
        return sock.getsockname()[1]


class SkirmishHostTest(unittest.TestCase):
    def test_retry_replaces_server_and_shutdown_reaps_it(self):
        with tempfile.TemporaryDirectory() as tmp:
            out = Path(tmp)
            static_port, game_port = port(), port()
            (out / 'index.html').write_text('<body>game</body>')
            (out / 'server.ron').write_text(f'(http_listen: "127.0.0.1:{game_port}")')
            # The stand-in exposes its PID so the test can distinguish two real processes.
            (out / 'game.py').write_text('''import http.server, os, sys
class Handler(http.server.BaseHTTPRequestHandler):
 def do_GET(self):
  self.send_response(200); self.end_headers(); self.wfile.write(str(os.getpid()).encode())
http.server.HTTPServer(('127.0.0.1', int(sys.argv[1])), Handler).serve_forever()
''')
            (out / 'argv.json').write_text(json.dumps([
                sys.executable, str(out / 'server.ron'), str(game_port)]))
            # Game.restart reads argv[1] as the config path. Execute this config as Python too.
            (out / 'server.ron').write_text(
                f'# http_listen: "127.0.0.1:{game_port}"\n' + (out / 'game.py').read_text())
            origin = f'http://127.0.0.1:{static_port}'
            process = subprocess.Popen([
                sys.executable, '-B', str(ROOT / 'scripts/skirmish_host.py'),
                str(static_port), str(out), str(out / 'argv.json'), str(out)],
                stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
            children = []
            try:
                deadline = time.monotonic() + 5
                while True:
                    try:
                        with urllib.request.urlopen(f'http://127.0.0.1:{game_port}/', timeout=.2) as response:
                            first = int(response.read())
                        children.append(first)
                        break
                    except OSError:
                        if time.monotonic() >= deadline:
                            raise
                        time.sleep(.02)
                with urllib.request.urlopen(origin) as response:
                    self.assertIn(b'/__skirmish/retry.js', response.read())
                with urllib.request.urlopen(origin + '/__skirmish/retry.js') as response:
                    script = response.read().decode()
                token = re.search(r"'X-TuRTS-Restart': (\"[^\"]+\")", script).group(1)
                token = json.loads(token)
                for wrong_origin, wrong_token in [(origin, 'bad'), ('https://other.test', token)]:
                    request = urllib.request.Request(origin + '/__skirmish/restart', method='POST',
                        headers={'Origin': wrong_origin, 'X-TuRTS-Restart': wrong_token})
                    with self.assertRaises(urllib.error.HTTPError) as rejected:
                        urllib.request.urlopen(request)
                    self.assertEqual(rejected.exception.code, 403)
                request = urllib.request.Request(origin + '/__skirmish/restart', method='POST',
                    headers={'Origin': origin, 'X-TuRTS-Restart': token})
                with urllib.request.urlopen(request) as response:
                    self.assertEqual(response.status, 204)
                with urllib.request.urlopen(f'http://127.0.0.1:{game_port}/') as response:
                    second = int(response.read())
                children.append(second)
                self.assertNotEqual(first, second)
                self.assertFalse(Path(f'/proc/{first}').exists())
            finally:
                process.send_signal(signal.SIGTERM)
                process.wait(timeout=10)
            for child in children:
                self.assertFalse(Path(f'/proc/{child}').exists())


if __name__ == '__main__':
    unittest.main()
