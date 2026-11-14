#!/usr/bin/env python3
"""Validate a fresh-browser launch address before Chrome starts."""
import re
import sys
from urllib.parse import urlsplit


def valid_url(value, mode):
    if any(ord(char) <= 32 for char in value) or "\\" in value:
        return False
    try:
        url = urlsplit(value)
        port = url.port
        host = url.hostname
    except ValueError:
        return False
    if not host or url.username is not None or url.password is not None or url.fragment:
        return False
    if port is not None and not 1 <= port <= 65535:
        return False
    local = host in {"127.0.0.1", "localhost", "::1"}
    if mode == "--url":
        return local and url.scheme in {"http", "https"}
    if mode != "--join" or url.scheme != "https":
        return False
    return local or all(re.fullmatch(r"[A-Za-z0-9](?:[A-Za-z0-9-]{0,61}[A-Za-z0-9])?", label)
                        for label in host.split("."))


if __name__ == "__main__":
    if len(sys.argv) != 3 or not valid_url(sys.argv[2], sys.argv[1]):
        print("mp1-play: use a loopback URL with --url, or an HTTPS game URL with --join.", file=sys.stderr)
        raise SystemExit(2)
