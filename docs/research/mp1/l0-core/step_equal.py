#!/usr/bin/env python3
"""L0-CORE purity proof for the split of `Core::step`.

The tokens of the old body of `step` (whitespace and comments removed) must equal the tokens of the
new body of `step` in which each call `self.NAME(args);` is replaced by the body of the method NAME.
Allowed differences, each printed with its count:
  1. `return` in a moved body becomes `continue` (the arm ended with `continue`, the method ends
     with `return`). A `continue` that the method keeps belongs to an inner loop of the old arm.
  2. `effects` that is not followed by `.` becomes `& mut effects` (a method takes `effects` as
     `&mut Vec<Effect>`, so the call sites inside it do not borrow again).
  3. A call of a hook (`--hook path::name=FILE`), a function that L0-CORE adds with an empty body,
     is dropped; its body in FILE must have no token.
  4. rustfmt shape, applied to the old and to the joined tokens (rustfmt changes it when code
     moves to a lower indentation): a closure `|x| { E }` and a match arm `=> { E }` whose block is
     one expression (no `;` at its top level) equal `|x| E` and `=> E ,`; a `,` before `)`, `]` or
     `}` is dropped.
Anything else (a swapped call, a changed literal, a moved `continue`) is DIFFERENT (exit 1).

usage: step_equal.py --old OLD.rs --new NEW.rs [--hook placement::update=placement.rs ...]
       step_equal.py --self-test --old OLD.rs --new NEW.rs
The tokenizer is the one of docs/research/mp1/l0-split/stmt_equal.py.
"""
import importlib.util
import os
import re
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
_src = open(os.path.join(HERE, "..", "l0-split", "stmt_equal.py")).read()
_src = _src.replace("sys.exit(main(sys.argv))", "")   # reuse tokenize, match, body_of, function_names
_ns = {}
exec(compile(_src, "stmt_equal.py", "exec"), _ns)
tokenize, match, body_of, function_names = _ns["tokenize"], _ns["match"], _ns["body_of"], _ns["function_names"]


def one_expression(tokens, open_at, close):
    """True when the block tokens[open_at+1:close] is not empty and has no `;` at its top level."""
    depth, has_semicolon = 0, False
    for tok in tokens[open_at + 1:close]:
        depth += tok in ("(", "[", "{")
        depth -= tok in (")", "]", "}")
        has_semicolon = has_semicolon or (depth == 0 and tok == ";")
    return not has_semicolon and close > open_at + 1


def strip_closure_braces(tokens):
    """The rustfmt shape of rule 4. Applied to both sides."""
    out, i, n = [], 0, len(tokens)
    while i < n:
        if (tokens[i] == "|" and i + 3 < n and tokens[i + 2] == "|" and tokens[i + 3] == "{"
                and re.fullmatch(r"[A-Za-z_][A-Za-z0-9_]*", tokens[i + 1])):
            close = match(tokens, i + 3, "{", "}")
            if one_expression(tokens, i + 3, close):
                out += tokens[i:i + 3]
                out += strip_closure_braces(tokens[i + 4:close])
                i = close + 1
                continue
        if tokens[i] == "=>" and i + 1 < n and tokens[i + 1] == "{":
            close = match(tokens, i + 1, "{", "}")
            if one_expression(tokens, i + 1, close):
                out += ["=>"] + strip_closure_braces(tokens[i + 2:close]) + [","]
                i = close + 1
                if i < n and tokens[i] == ",":
                    i += 1
                continue
        if tokens[i] == "," and i + 1 < n and tokens[i + 1] in (")", "]", "}"):
            i += 1
            continue
        out.append(tokens[i])
        i += 1
    return out


def check(old_text, new_text, hooks, quiet=False):
    old_tokens = tokenize(old_text)
    new_tokens = tokenize(new_text)
    old_body = body_of(old_tokens, "step")
    caller = body_of(new_tokens, "step")
    old_names = set(function_names(old_tokens))
    methods = {}
    for name in dict.fromkeys(function_names(new_tokens)):
        if name not in old_names and name.startswith("on_"):
            methods[name] = body_of(new_tokens, name)
    joined, i, calls, returns, ampersands = [], 0, [], 0, 0
    empty_calls = []
    while i < len(caller):
        t = caller[i]
        if t == "self" and caller[i + 1] == "." and caller[i + 2] in methods and caller[i + 3] == "(":
            name = caller[i + 2]
            end = match(caller, i + 3, "(", ")")
            assert caller[end + 1] == ";", f"call of {name} is not a statement"
            body = []
            for j, tok in enumerate(methods[name]):
                if tok == "return":
                    body.append("continue")
                    returns += 1
                elif tok == "effects" and (j + 1 >= len(methods[name]) or methods[name][j + 1] != "."):
                    body += ["&", "mut", "effects"]
                    ampersands += 1
                else:
                    body.append(tok)
            joined += body
            calls.append(name)
            i = end + 2
            continue
        hook = None
        for path in hooks:
            parts = tokenize(path)
            if caller[i:i + len(parts)] == parts and caller[i + len(parts)] == "(":
                hook = (path, parts)
        if hook:
            end = match(caller, i + len(hook[1]), "(", ")")
            assert caller[end + 1] == ";", f"call of {hook[0]} is not a statement"
            empty_calls.append(hook[0])
            i = end + 2
            continue
        joined.append(t)
        i += 1
    old_body, joined = strip_closure_braces(old_body), strip_closure_braces(joined)
    first = next((j for j, (x, y) in enumerate(zip(old_body, joined)) if x != y), None)
    unused = [m for m in methods if m not in calls]
    equal = first is None and len(old_body) == len(joined) and not unused
    out = []
    out.append(f"old `step` tokens {len(old_body)}, joined tokens {len(joined)}")
    out.append(f"calls inlined, in order: {' '.join(calls)}")
    out.append(f"`return` -> `continue`: {returns}; `effects` -> `& mut effects`: {ampersands}")
    out.append(f"hook calls dropped: {' '.join(empty_calls) or 'none'}")
    for name in unused:
        out.append(f"NOT-CALLED: {name}")
    if not equal:
        at = first if first is not None else min(len(old_body), len(joined))
        out.append(f"DIFFERENT at token {at}")
        out.append("  old:    " + " ".join(old_body[max(0, at - 6):at + 8]))
        out.append("  joined: " + " ".join(joined[max(0, at - 6):at + 8]))
    out.append("RESULT " + ("EQUAL" if equal else "DIFFERENT"))
    if not quiet:
        print("\n".join(out))
    return equal, calls, empty_calls


def hook_bodies_empty(hooks):
    for spec in hooks:
        path, file = spec.split("=")
        name = path.split("::")[-1]
        body = body_of(tokenize(open(file).read()), name)
        if body:
            print(f"HOOK-NOT-EMPTY {path}: {len(body)} tokens in {file}")
            return False
    return True


def main(argv):
    old = open(argv[argv.index("--old") + 1]).read()
    new = open(argv[argv.index("--new") + 1]).read()
    specs = [argv[k + 1] for k, a in enumerate(argv) if a == "--hook"]
    hooks = [s.split("=")[0] for s in specs]
    if "--self-test" in argv:
        # Each mutation of the NEW text must give DIFFERENT. The unmutated text must give EQUAL.
        muts = {
            "swap two calls": (r"self\.on_pointer_down\(", "self.on_pointer_up_x("),
            "change a literal": (r"start\.distance\(px\) > 4\.0", "start.distance(px) > 5.0"),
            "drop a return": (r"\n(\s+)return;", r"\n\1"),
            "move a call out of its arm": (r"self\.on_key_up\(code, world, view, &mut effects\);", ""),
        }
        ok = check(old, new, hooks, quiet=True)[0]
        print("unmutated:", "EQUAL" if ok else "DIFFERENT (self-test cannot run)")
        bad = not ok
        for label, (pat, rep) in muts.items():
            mutated, n = re.subn(pat, rep, new, count=1)
            assert n == 1, f"mutation `{label}` did not apply"
            try:
                equal = check(old, mutated, hooks, quiet=True)[0]
            except (AssertionError, KeyError, IndexError, ValueError) as err:
                equal = False
                print(f"mutation `{label}`: proof refused ({err.__class__.__name__}) -> red")
                continue
            print(f"mutation `{label}`:", "STILL EQUAL (the proof is blind)" if equal else "DIFFERENT -> red")
            bad = bad or equal
        print("SELF-TEST", "FAILED" if bad else "OK")
        return 1 if bad else 0
    equal, _, _ = check(old, new, hooks)
    return 0 if equal and hook_bodies_empty(specs) else 1


sys.exit(main(sys.argv))
