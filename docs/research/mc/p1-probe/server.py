#!/usr/bin/env python3
"""Minimal local HTTP server for the headed P1 browser probe."""
from http.server import BaseHTTPRequestHandler, HTTPServer
from pathlib import Path
import json

ROOT = Path(__file__).resolve().parent
LOG = ROOT / "p1-log.jsonl"

class Handler(BaseHTTPRequestHandler):
    def do_GET(self):
        body = (ROOT / "page.html").read_bytes() if self.path.split("?", 1)[0] == "/" else b"not found"
        self.send_response(200 if self.path.split("?", 1)[0] == "/" else 404)
        self.send_header("Content-Type", "text/html; charset=utf-8")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def do_POST(self):
        if self.path != "/record":
            self.send_error(404)
            return
        try:
            row = json.loads(self.rfile.read(int(self.headers.get("Content-Length", "0"))))
            with LOG.open("a", encoding="utf-8") as stream:
                stream.write(json.dumps(row, separators=(",", ":"), sort_keys=True) + "\n")
            self.send_response(204)
            self.end_headers()
        except Exception as error:
            self.send_error(400, str(error))

    def log_message(self, fmt, *args):
        pass

if __name__ == "__main__":
    HTTPServer(("127.0.0.1", 8765), Handler).serve_forever()
