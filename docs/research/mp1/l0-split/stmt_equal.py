#!/usr/bin/env python3
"""Statement-level proof that a function body was cut into functions and nothing else changed.

The old body (tokens, whitespace and comments removed) must equal the new caller body in which
each call of a concern function is replaced by the body of that function. Allowed differences,
all printed as the glue leftover:
  1. the parameter lists and signatures of the new functions (not compared);
  2. the call statements (replaced; `let X = f(..);` also drops the callee's last token X);
  3. the substitutions of GLUE, each of which must match its stated number of times.
Anything else makes the result DIFFERENT (exit 1). Control flow is compared as tokens, so a moved
`return`, a swapped statement, a changed operator or a changed literal is DIFFERENT.

usage: stmt_equal.py --old OLD.rs [OLD2.rs ...] --caller NAME --new FILE... [--glue GLUE.json]
"""
import json
import re
import sys

PUNCT = [
    "..=", "...", "<<=", ">>=", "::", "->", "=>", "==", "!=", "<=", ">=", "&&", "||", "+=", "-=",
    "*=", "/=", "%=", "^=", "&=", "|=", "<<", ">>", "..",
]
NUMBER = re.compile(r"\d[\d_]*(\.\d[\d_]*)?([eE][+-]?\d+)?[A-Za-z0-9_]*")
IDENT = re.compile(r"[A-Za-z_][A-Za-z0-9_]*")


def tokenize(text):
    out, i, n = [], 0, len(text)
    while i < n:
        c = text[i]
        if c.isspace():
            i += 1
        elif text.startswith("//", i):
            while i < n and text[i] != "\n":
                i += 1
        elif text.startswith("/*", i):
            depth = 1
            i += 2
            while i < n and depth:
                if text.startswith("/*", i):
                    depth += 1
                    i += 2
                elif text.startswith("*/", i):
                    depth -= 1
                    i += 2
                else:
                    i += 1
        elif c == '"':
            j = i + 1
            while text[j] != '"':
                j += 2 if text[j] == "\\" else 1
            out.append(text[i:j + 1])
            i = j + 1
        elif c == "'":
            m = re.match(r"'(\\.[^']*|[^\\'])'", text[i:])
            if m:
                out.append(m.group(0))
                i += len(m.group(0))
            else:  # a lifetime
                m = re.match(r"'[A-Za-z_][A-Za-z0-9_]*", text[i:])
                out.append(m.group(0))
                i += len(m.group(0))
        elif c.isdigit():
            m = NUMBER.match(text, i)
            out.append(m.group(0))
            i = m.end()
        elif c.isalpha() or c == "_":
            m = IDENT.match(text, i)
            out.append(m.group(0))
            i = m.end()
        else:
            for p in PUNCT:
                if text.startswith(p, i):
                    out.append(p)
                    i += len(p)
                    break
            else:
                out.append(c)
                i += 1
    return out


def match(tokens, i, open_, close):
    """Index of the token that closes the bracket opened at tokens[i]."""
    depth = 0
    for j in range(i, len(tokens)):
        if tokens[j] == open_:
            depth += 1
        elif tokens[j] == close:
            depth -= 1
            if depth == 0:
                return j
    raise ValueError("unbalanced " + open_)


def body_of(tokens, name):
    """Tokens inside the braces of `fn name`. The parameter list and the return type are skipped."""
    for i in range(len(tokens) - 2):
        if tokens[i] == "fn" and tokens[i + 1] == name and tokens[i + 2] == "(":
            end = match(tokens, i + 2, "(", ")")
            j = end + 1
            while tokens[j] != "{":
                j += 1
            close = match(tokens, j, "{", "}")
            return tokens[j + 1:close]
    raise KeyError(name)


def function_names(tokens):
    return [tokens[i + 1] for i in range(len(tokens) - 1) if tokens[i] == "fn"]


def replace_all(tokens, old, new, expected, where):
    count, out, i = 0, [], 0
    while i < len(tokens):
        if tokens[i:i + len(old)] == old:
            out.extend(new)
            i += len(old)
            count += 1
        else:
            out.append(tokens[i])
            i += 1
    if count != expected:
        raise SystemExit(f"GLUE-COUNT {where}: `{' '.join(old)}` matched {count}, expected {expected}")
    return out


def main(argv):
    a = argv.index("--old")
    c = argv.index("--caller")
    n = argv.index("--new")
    glue_at = argv.index("--glue") if "--glue" in argv else None
    old_files, caller = argv[a + 1:c], argv[c + 1]
    old_file = old_files[0]
    new_files = [x for x in argv[n + 1:(glue_at if glue_at and glue_at > n else len(argv))]]
    glue = json.load(open(argv[glue_at + 1])) if glue_at else {}
    old_body = body_of(tokenize(open(old_file).read()), caller)
    new_tokens = []
    for path in new_files:
        new_tokens += tokenize(open(path).read())
    caller_body = body_of(new_tokens, caller)
    # the concern functions are the functions that the new files define and the old file did not
    old_names = set()
    for path in old_files:
        old_names |= set(function_names(tokenize(open(path).read())))
    concerns = [f for f in dict.fromkeys(function_names(new_tokens)) if f not in old_names]
    bodies = {}
    for name in concerns:
        body = body_of(new_tokens, name)
        for rule in glue.get(name, []):
            body = replace_all(body, tokenize(rule["new"]), tokenize(rule["old"]), rule.get("count", 1), name)
        bodies[name] = body
    joined, i, calls, let_forms = [], 0, [], 0
    while i < len(caller_body):
        t = caller_body[i]
        is_let_call = (
            t == "let" and i + 4 < len(caller_body) and caller_body[i + 2] == "="
            and caller_body[i + 3] in bodies and caller_body[i + 4] == "("
        )
        is_call = t in bodies and i + 1 < len(caller_body) and caller_body[i + 1] == "("
        if is_let_call or is_call:
            k = i + 3 if is_let_call else i
            name = caller_body[k]
            end = match(caller_body, k + 1, "(", ")")
            assert caller_body[end + 1] == ";", f"call of {name} is not a statement"
            body = bodies[name]
            if is_let_call:
                var = caller_body[i + 1]
                assert body[-1] == var, f"{name} must end with its result `{var}`"
                body = body[:-1]
                let_forms += 1
            joined += body
            calls.append(name)
            i = end + 2
        else:
            joined.append(t)
            i += 1
    unused = [f for f in bodies if f not in calls]
    first = next((j for j, (x, y) in enumerate(zip(old_body, joined)) if x != y), None)
    equal = first is None and len(old_body) == len(joined) and not unused
    print(f"caller `{caller}`: old tokens {len(old_body)}, joined tokens {len(joined)}")
    print(f"calls inlined, in order: {' '.join(calls)}")
    print(f"let-form calls (callee tail dropped): {let_forms}")
    glue_total = sum(rule.get("count", 1) for rules in glue.values() for rule in rules)
    print(f"glue substitutions applied: {glue_total}")
    for name, rules in glue.items():
        for rule in rules:
            print(f"  glue {name}: `{rule['new']}` -> `{rule['old']}` x{rule.get('count', 1)}")
    for name in unused:
        print(f"NOT-CALLED: {name}")
    returns = {name: bodies[name].count("return") for name in bodies}
    print(f"`return` tokens: old body {old_body.count('return')}, caller {caller_body.count('return')}, "
          f"concern functions {sum(returns.values())}")
    if first is not None or len(old_body) != len(joined):
        at = first if first is not None else min(len(old_body), len(joined))
        print("DIFFERENT at token", at)
        print("  old:   ", " ".join(old_body[max(0, at - 6):at + 8]))
        print("  joined:", " ".join(joined[max(0, at - 6):at + 8]))
    if sum(returns.values()):
        print("DIFFERENT: a concern function has `return`")
        equal = False
    print("RESULT", "EQUAL" if equal else "DIFFERENT")
    return 0 if equal else 1


sys.exit(main(sys.argv))
