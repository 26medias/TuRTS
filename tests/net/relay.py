#!/usr/bin/env python3
"""TCP relay for the networking spike (Task 6a). Standard library only.

relay.py <listen_port> <target_port> [--pause-after A] [--pause-secs B] [--rcvbuf N] [--out PATH]

Accepts one client on 127.0.0.1:<listen_port> and connects it to 127.0.0.1:<target_port>.
One thread per direction counts the bytes. --rcvbuf sets SO_RCVBUF on the socket toward the
server before it connects. A seconds after the start, the relay stops reading from the server
for B seconds. --out gets {"to_client_bytes", "to_server_bytes", "seconds"} when both
directions reach EOF, or on SIGTERM.

Stdout lines: "relay listening <port>", "pause <unix_ms>", "resume <unix_ms>".
"""

import argparse
import json
import os
import signal
import socket
import sys
import threading
import time


def unix_ms():
    return int(time.time() * 1000)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("listen_port", type=int)
    parser.add_argument("target_port", type=int)
    parser.add_argument("--pause-after", type=float, default=None)
    parser.add_argument("--pause-secs", type=float, default=0.0)
    parser.add_argument("--rcvbuf", type=int, default=0)
    parser.add_argument("--out", default=None)
    args = parser.parse_args()

    t0 = time.monotonic()
    counts = {"to_client_bytes": 0, "to_server_bytes": 0}
    lock = threading.Lock()
    written = [False]

    def write_out():
        with lock:
            if written[0]:
                return
            written[0] = True
            result = dict(counts)
            result["seconds"] = round(time.monotonic() - t0, 3)
            if args.out:
                with open(args.out, "w") as f:
                    json.dump(result, f)
            print("relay result " + json.dumps(result), flush=True)

    def on_sigterm(_signum, _frame):
        write_out()
        os._exit(0)

    signal.signal(signal.SIGTERM, on_sigterm)

    listener = socket.socket()
    listener.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
    listener.bind(("127.0.0.1", args.listen_port))
    listener.listen(1)
    print(f"relay listening {args.listen_port}", flush=True)

    client, _ = listener.accept()
    server = socket.socket()
    if args.rcvbuf:
        server.setsockopt(socket.SOL_SOCKET, socket.SO_RCVBUF, args.rcvbuf)
    server.connect(("127.0.0.1", args.target_port))

    def pump(src, dst, key, pausing):
        paused = False
        while True:
            if (
                pausing
                and not paused
                and args.pause_after is not None
                and time.monotonic() - t0 >= args.pause_after
            ):
                paused = True
                print(f"pause {unix_ms()}", flush=True)
                time.sleep(args.pause_secs)
                print(f"resume {unix_ms()}", flush=True)
            try:
                data = src.recv(65536)
            except OSError:
                break
            if not data:
                break
            with lock:
                counts[key] += len(data)
            try:
                dst.sendall(data)
            except OSError:
                break
        try:
            dst.shutdown(socket.SHUT_WR)
        except OSError:
            pass

    to_server = threading.Thread(target=pump, args=(client, server, "to_server_bytes", False))
    to_client = threading.Thread(target=pump, args=(server, client, "to_client_bytes", True))
    to_server.start()
    to_client.start()
    # Join with a timeout so that SIGTERM can run its handler.
    while to_server.is_alive() or to_client.is_alive():
        to_server.join(0.2)
        to_client.join(0.2)
    write_out()
    return 0


if __name__ == "__main__":
    sys.exit(main())
