#!/usr/bin/env python3
"""Start a temporary HTTPS human match from a prepared build."""
import argparse
import json
import os
from pathlib import Path
import re
import shutil
import signal
import socket
import subprocess
import sys
import tempfile
import uuid
import time
from urllib.parse import urlencode
from urllib.request import urlopen

ROOT = Path(__file__).resolve().parents[1]
TUNNEL_URL = re.compile(r"https://[a-z0-9]+(?:-[a-z0-9]+)*\.trycloudflare\.com(?=[/\s|]|$)")


class SharedHost:
    def __init__(self, prepared, output, cloudflared, timeout=120):
        self.prepared = Path(prepared).resolve()
        self.output = Path(output).resolve()
        self.cloudflared = cloudflared
        self.timeout = timeout
        self.children = []
        self.logs = []
        self.addresses = {}

    def start_child(self, name, argv):
        log = (self.output / (name + ".log")).open("wb")
        self.logs.append(log)
        child = subprocess.Popen(argv, stdout=log, stderr=subprocess.STDOUT, start_new_session=True,
                                 env={k: v for k, v in os.environ.items()
                                      if not k.startswith(("TUNNEL_", "CLOUDFLARE_", "CF_"))})
        self.children.append(child)
        return child

    def check_children(self):
        for child in self.children:
            if child.poll() is not None:
                raise RuntimeError(f"A hosting process stopped with code {child.returncode}. See {self.output}.")

    def wait_for(self, check, label):
        deadline = time.monotonic() + self.timeout
        while time.monotonic() < deadline:
            self.check_children()
            value = check()
            if value:
                return value
            time.sleep(0.1)
        raise RuntimeError(f"Timeout: {label}. See {self.output}.")

    def start_tunnel(self, name, port):
        config = self.output / "cloudflared-empty.yml"
        config.write_text("{}\n")
        self.start_child(name, [self.cloudflared, "tunnel", "--config", str(config), "--origincert",
                               str(self.output / "unused-origin-certificate.pem"), "--no-autoupdate",
                               "--metrics", "127.0.0.1:0", "--protocol", "http2", "--grace-period", "2s",
                               "--url", f"http://127.0.0.1:{port}"])
        def address():
            text = (self.output / (name + ".log")).read_text(errors="replace")
            match = TUNNEL_URL.search(text)
            return match.group(0) if match and "Registered tunnel connection" in text else None
        return self.wait_for(address, name + " address")

    def start(self):
        self.output.mkdir(parents=True, exist_ok=False, mode=0o700)
        sockets = [socket.socket(), socket.socket()]
        try:
            for listener in sockets:
                listener.bind(("127.0.0.1", 0))
            http, ws = [listener.getsockname()[1] for listener in sockets]
            public_http = self.start_tunnel("http-tunnel", http)
            public_ws = self.start_tunnel("ws-tunnel", ws).replace("https://", "wss://", 1) + "/"
            cfg = {"game_package": str(self.prepared / "pack/skirmish.tar"),
                   "http_listen": f"127.0.0.1:{http}", "ws_listen": f"127.0.0.1:{ws}",
                   "public_url": public_http, "tick_hz": 20, "static_dir": str(self.prepared / "dist")}
            config = self.output / "server.ron"
            config.write_text("(\n" + "".join(f"  {key}: {json.dumps(value, ensure_ascii=False)},\n"
                                              for key, value in cfg.items()) + ")\n")
            for listener in sockets:
                listener.close()
            self.start_child("server", [str(self.prepared / "bin/turts_server"), str(config)])
            self.addresses = {"http": public_http, "ws": public_ws, "local_http": f"http://127.0.0.1:{http}"}
            local_ad = self.wait_for(lambda: self.http_ready(self.addresses["local_http"]), "local server")
            self.wait_for(lambda: self.http_ready(public_http) == local_ad, "matching public HTTPS server")
            self.addresses["url"] = public_http + "/?" + urlencode({"server": public_http, "ws": public_ws})
            (self.output / "hosting.json").write_text(json.dumps(
                {**self.addresses, "owned_pids": [child.pid for child in self.children]}, indent=2) + "\n")
            return self.addresses["url"]
        finally:
            for listener in sockets:
                listener.close()

    @staticmethod
    def http_ready(origin):
        try:
            with urlopen(origin + "/advertisement.json", timeout=2) as response:
                value = json.load(response)
                return value if response.status == 200 and isinstance(value, dict) and value.get("lockfile_hash") else False
        except (OSError, ValueError):
            return False

    def close(self):
        # Kill only process groups created by this object. The operator's services are untouched.
        for child in reversed(self.children):
            if child.poll() is None:
                try:
                    os.killpg(child.pid, signal.SIGTERM)
                except ProcessLookupError:
                    pass
        for child in reversed(self.children):
            try:
                child.wait(timeout=5)
            except subprocess.TimeoutExpired:
                os.killpg(child.pid, signal.SIGKILL)
                child.wait()
        for log in self.logs:
            log.close()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--prepared", required=True, type=Path)
    parser.add_argument("--out", type=Path, help="New folder for logs. Keep it outside the source tree.")
    parser.add_argument("--cloudflared", default=shutil.which("cloudflared"))
    args = parser.parse_args()
    if not args.cloudflared:
        parser.error("Install cloudflared or set --cloudflared to its executable path.")
    env = os.environ.copy()
    env.setdefault("CARGO_PROFILE_DEV_DEBUG", "0")
    env.setdefault("CARGO_PROFILE_TEST_DEBUG", "0")
    checked = subprocess.run([sys.executable, "-B", str(ROOT / "scripts/mc_prepare.py"), "check", str(args.prepared),
                    "--root", str(ROOT), "--package", "mods/skirmish", "--need", "bin/turts_server",
                    "--need", "pack/skirmish.tar", "--need", "dist/index.html"], env=env)
    if checked.returncode:
        return checked.returncode
    output = args.out or Path(tempfile.gettempdir()) / ("turts-share-" + uuid.uuid4().hex)
    host = SharedHost(args.prepared, output, args.cloudflared)
    def stop(_signal, _frame):
        raise KeyboardInterrupt
    signal.signal(signal.SIGTERM, stop)
    signal.signal(signal.SIGINT, stop)
    try:
        url = host.start()
        print("URL: " + url, flush=True)
        print("This temporary link lets two players join. Anyone with the link can join.", flush=True)
        print("Keep this command running. Press Ctrl+C to stop the server and both tunnels.", flush=True)
        print("Logs: " + str(output), flush=True)
        while True:
            host.check_children()
            time.sleep(0.5)
    except KeyboardInterrupt:
        return 0
    except (OSError, RuntimeError) as error:
        print(f"skirmish-share: {error}", file=sys.stderr)
        return 1
    finally:
        host.close()


if __name__ == "__main__":
    raise SystemExit(main())
