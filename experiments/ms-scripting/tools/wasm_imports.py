#!/usr/bin/env python3
"""Print the import list of a wasm module as JSON: [["module", "name", kind], ...].

Standard library only. `--self-test` builds a module with one function import and checks
that the parser reports it (plan red check), and a module with no import section.
"""
import json
import sys

KINDS = {0: "func", 1: "table", 2: "memory", 3: "global", 4: "tag"}


def leb(b, i):
    r = s = 0
    while True:
        x = b[i]
        i += 1
        r |= (x & 0x7F) << s
        s += 7
        if x < 0x80:
            return r, i


def name(b, i):
    n, i = leb(b, i)
    return b[i:i + n].decode("utf-8"), i + n


def imports(b):
    assert b[:4] == b"\0asm", "not a wasm module"
    i, out = 8, []
    while i < len(b):
        sid = b[i]
        size, j = leb(b, i + 1)
        if sid == 2:
            count, k = leb(b, j)
            for _ in range(count):
                mod, k = name(b, k)
                nm, k = name(b, k)
                kind = b[k]
                k += 1
                if kind == 0:
                    _, k = leb(b, k)
                elif kind == 1:
                    k += 1
                    flags, k = leb(b, k)
                    _, k = leb(b, k)
                    if flags & 1:
                        _, k = leb(b, k)
                elif kind == 2:
                    flags, k = leb(b, k)
                    _, k = leb(b, k)
                    if flags & 1:
                        _, k = leb(b, k)
                elif kind == 3:
                    k += 2
                elif kind == 4:
                    k += 1
                    _, k = leb(b, k)
                out.append([mod, nm, KINDS.get(kind, str(kind))])
        i = j + size
    return out


def self_test():
    header = b"\0asm\x01\0\0\0"
    types = bytes([1, 4, 1, 0x60, 0, 0])  # one type: () -> ()
    imp_body = bytes([1, 3]) + b"env" + bytes([3]) + b"now" + bytes([0, 0])
    imp = bytes([2, len(imp_body)]) + imp_body
    with_import = imports(header + types + imp)
    assert with_import == [["env", "now", "func"]], with_import
    assert imports(header + types) == []
    print("self-test ok: one import found, empty module has none")


if __name__ == "__main__":
    if sys.argv[1:] == ["--self-test"]:
        self_test()
    else:
        print(json.dumps(imports(open(sys.argv[1], "rb").read())))
