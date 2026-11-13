#!/usr/bin/env python3
"""Serve a local skirmish and let its player start another match."""
import functools
import http.server
import json
from pathlib import Path
import re
import secrets
import signal
import subprocess
import sys
import threading
import time
import urllib.request


class Game:
    def __init__(self, argv, out):
        self.argv = argv
        self.log = (out / 'server.log').open('ab')
        self.process = None
        self.lock = threading.Lock()
        self.origin = None
        self.token = secrets.token_urlsafe(32)

    def stop(self):
        if self.process is not None:
            self.process.terminate()
            try:
                self.process.wait(timeout=5)
            except subprocess.TimeoutExpired:
                self.process.kill()
                self.process.wait()
            self.process = None

    def start(self):
        self.process = subprocess.Popen(self.argv, stdout=self.log, stderr=subprocess.STDOUT)

    def restart(self):
        with self.lock:
            self.stop()
            self.start()
            # The browser must not reconnect until the new server can accept it.
            config = Path(self.argv[1]).read_text()
            address = re.search(r'http_listen:\s*"([^"]+)"', config).group(1)
            deadline = time.monotonic() + 8
            while time.monotonic() < deadline:
                if self.process.poll() is not None:
                    raise RuntimeError('The new match could not start.')
                try:
                    with urllib.request.urlopen('http://' + address + '/advertisement.json', timeout=.2):
                        return
                except OSError:
                    time.sleep(.05)
            raise RuntimeError('The new match did not start in time.')


RETRY_JS = r'''
(() => {
  const button = document.createElement('button');
  button.id = 'new-skirmish';
  button.textContent = 'New skirmish';
  button.hidden = true;
  button.style.cssText = 'position:fixed;left:50%;top:calc(50% + 94px);transform:translateX(-50%);z-index:1000;padding:12px 24px;font:18px monospace;color:white;background:#215537;border:2px solid white;cursor:pointer';
  document.body.append(button);
  const timer = setInterval(() => {
    try {
      if (typeof window.turts_ui !== 'function') return;
      const raw = window.turts_ui();
      const widgets = typeof raw === 'string' ? JSON.parse(raw) : raw;
      if (widgets.some(row => row.visible && ['Victory', 'Defeat', 'Draw'].includes(row.text))) {
        button.hidden = false;
        clearInterval(timer);
      }
    } catch (_) { /* The game can still be loading. */ }
  }, 250);
  button.onclick = async () => {
    button.disabled = true;
    button.textContent = 'Starting...';
    try {
      const response = await fetch('/__skirmish/restart', {method:'POST', headers:{'X-TuRTS-Restart': TOKEN}});
      if (!response.ok) throw new Error('Start failed');
      location.reload();
    } catch (_) {
      button.textContent = 'Could not start. Try again';
      button.disabled = false;
    }
  };
})();
'''


class Handler(http.server.SimpleHTTPRequestHandler):
    def do_GET(self):
        if self.path == '/__skirmish/retry.js':
            data = RETRY_JS.replace('TOKEN', json.dumps(self.server.game.token)).encode()
            content_type = 'text/javascript'
        elif self.path.split('?', 1)[0] in ('/', '/index.html'):
            data = (Path(self.directory) / 'index.html').read_bytes().replace(
                b'</body>', b'<script src="/__skirmish/retry.js"></script></body>')
            content_type = 'text/html'
        else:
            return super().do_GET()
        self.send_response(200)
        self.send_header('Content-Type', content_type)
        self.send_header('Content-Length', str(len(data)))
        self.send_header('Cache-Control', 'no-store')
        self.end_headers()
        self.wfile.write(data)

    def do_POST(self):
        game = self.server.game
        if (self.path != '/__skirmish/restart'
                or self.headers.get('Origin') != game.origin
                or not secrets.compare_digest(self.headers.get('X-TuRTS-Restart', ''), game.token)):
            self.send_error(403)
            return
        try:
            game.restart()
        except (OSError, RuntimeError) as error:
            self.send_error(503, str(error))
            return
        self.send_response(204)
        self.send_header('Content-Length', '0')
        self.end_headers()


def main():
    port, dist, argv_file, output = sys.argv[1:]
    game = Game(json.loads(Path(argv_file).read_text()), Path(output))
    server = http.server.ThreadingHTTPServer(('127.0.0.1', int(port)), functools.partial(Handler, directory=dist))
    server.game = game
    game.origin = 'http://127.0.0.1:' + port
    def stop(_signum, _frame):
        signal.signal(signal.SIGTERM, signal.SIG_IGN)
        signal.signal(signal.SIGINT, signal.SIG_IGN)
        raise SystemExit(0)
    signal.signal(signal.SIGTERM, stop)
    signal.signal(signal.SIGINT, stop)
    try:
        game.start()
        server.serve_forever()
    finally:
        server.server_close()
        game.stop()
        game.log.close()


if __name__ == '__main__':
    main()
