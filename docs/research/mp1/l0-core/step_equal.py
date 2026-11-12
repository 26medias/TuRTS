#!/usr/bin/env python3
"""L0-CORE purity proof for the split of `Core::step`.

The tokens of the old body of `step` (whitespace and comments removed) must equal the tokens of the
new body of `step` in which each call `self.NAME(args);` is replaced by the body of the method NAME.
Checks, each with its count printed:
  1. Each argument of a call is its parameter, by name and in order (`code` for `code`), or
     `&mut` of it (`&mut effects` for `effects`), or `self` for a parameter `core`. An argument
     that is `0`, `&mut Vec::new()` or another name is DIFFERENT.
  2. In a moved body, `return` at loop depth 0 becomes `continue` (the arm ended with `continue`;
     the method ends with `return`). A `return` inside a loop of the method stays `return`, so it
     differs from the `continue` of the old arm. A `continue` at loop depth 0 is refused.
  3. `effects` that is not followed by `.` becomes `& mut effects` (a method takes `effects` as
     `&mut Vec<Effect>`, so the calls inside it do not borrow again).
  4. Hooks that L0-CORE adds. The hook bodies are checked in their own files.
       --hook PATH=FILE          `PATH(args);` with an empty body, right after the second `prune`
                                 statement (after the event loop), in the given order, and before
                                 the `let` of the held camera actions.
       --intercept PATH=FILE     `if PATH(args) { continue; }` as the first statement of the event
                                 loop; the body of the function must be exactly `core.placement.taking_input`
                                 (set `--intercept-body` for another body).
       --arm-hook VARIANT=PATH=FILE   the match arm `InputEvent::VARIANT(x) => { PATH(args); }` that the
                                 old code did not have; the function body must be empty.
     Each declared hook must be found exactly once. A hook that moved is DIFFERENT.
  5. rustfmt shape, applied to the old and to the joined tokens (rustfmt changes it when code
     moves to a lower indentation): a closure `|x| { E }` and a match arm `=> { E }` whose block is
     one expression (no `;` at its top level) equal `|x| E` and `=> E ,`; a `,` before `)`, `]` or
     `}` is dropped.
Anything else is DIFFERENT (exit 1).

usage: step_equal.py --old OLD.rs --new NEW.rs [--hook ...] [--intercept ...] [--arm-hook ...]
       step_equal.py --self-test --old OLD.rs --new NEW.rs [same hook options]
The tokenizer is the one of docs/research/mp1/l0-split/stmt_equal.py.
"""
import os
import re
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
_src = open(os.path.join(HERE, "..", "l0-split", "stmt_equal.py")).read()
_src = _src.replace("sys.exit(main(sys.argv))", "")   # reuse tokenize, match, body_of, function_names
_ns = {}
exec(compile(_src, "stmt_equal.py", "exec"), _ns)
tokenize, match, body_of, function_names = _ns["tokenize"], _ns["match"], _ns["body_of"], _ns["function_names"]

PRUNE = ["}", "}", "self", ".", "prune", "(", "world", ")", ";"]   # the end of the event loop, then `prune`
LOOP_HEAD = ["enumerate", "(", ")", "{"]


class Refused(Exception):
    pass


def one_expression(tokens, open_at, close):
    depth, has_semicolon = 0, False
    for tok in tokens[open_at + 1:close]:
        depth += tok in ("(", "[", "{")
        depth -= tok in (")", "]", "}")
        has_semicolon = has_semicolon or (depth == 0 and tok == ";")
    return not has_semicolon and close > open_at + 1


def strip_shape(tokens):
    out, i, n = [], 0, len(tokens)
    while i < n:
        if (tokens[i] == "|" and i + 3 < n and tokens[i + 2] == "|" and tokens[i + 3] == "{"
                and re.fullmatch(r"[A-Za-z_][A-Za-z0-9_]*", tokens[i + 1])):
            close = match(tokens, i + 3, "{", "}")
            if one_expression(tokens, i + 3, close):
                out += tokens[i:i + 3] + strip_shape(tokens[i + 4:close])
                i = close + 1
                continue
        if tokens[i] == "=>" and i + 1 < n and tokens[i + 1] == "{":
            close = match(tokens, i + 1, "{", "}")
            if one_expression(tokens, i + 1, close):
                out += ["=>"] + strip_shape(tokens[i + 2:close]) + [","]
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


def split_top(tokens):
    """Split a token list at the commas of depth 0 (brackets and angle brackets)."""
    parts, cur, depth = [], [], 0
    for tok in tokens:
        if tok in ("(", "[", "{", "<"):
            depth += 1
        elif tok in (")", "]", "}", ">"):
            depth -= 1
        if tok == "," and depth == 0:
            parts.append(cur)
            cur = []
        else:
            cur.append(tok)
    if cur:
        parts.append(cur)
    return parts


def params_of(tokens, name):
    """The parameter names of `fn name(...)`, in order (`self` for `&mut self`)."""
    for i in range(len(tokens) - 2):
        if tokens[i] == "fn" and tokens[i + 1] == name and tokens[i + 2] == "(":
            end = match(tokens, i + 2, "(", ")")
            names = []
            for part in split_top(tokens[i + 3:end]):
                names.append(part[part.index(":") - 1] if ":" in part else "self")
            return names
    raise Refused(f"no fn {name}")


def args_of(tokens, open_at):
    end = match(tokens, open_at, "(", ")")
    return split_top(tokens[open_at + 1:end]), end


def check_args(label, args, params):
    if len(args) != len(params):
        raise Refused(f"{label}: {len(args)} arguments for {len(params)} parameters")
    for arg, param in zip(args, params):
        param = param.lstrip("_")   # an unused parameter of a stub is written `_name`
        ok = arg in ([param], ["&", "mut", param]) or (param == "core" and arg == ["self"])
        if not ok:
            raise Refused(f"{label}: argument `{' '.join(arg)}` for parameter `{param}`")


def loop_depths(tokens):
    """The loop depth at each token (the number of `for`, `while` and `loop` bodies around it)."""
    depths, stack, pending = [], [], False
    for tok in tokens:
        if tok in ("for", "while", "loop"):
            pending = True
        depths.append(sum(stack))
        if tok == "{":
            stack.append(1 if pending else 0)
            pending = False
        elif tok == "}":
            stack.pop()
    return depths


def check(old_text, new_text, hooks, intercepts, arm_hooks, quiet=False):
    """hooks, intercepts, arm_hooks: lists of dicts with `path`, `tokens` (the file tokens)."""
    old_tokens = tokenize(old_text)
    new_tokens = tokenize(new_text)
    old_body = body_of(old_tokens, "step")
    caller = body_of(new_tokens, "step")
    old_names = set(function_names(old_tokens))
    methods = {}
    for name in dict.fromkeys(function_names(new_tokens)):
        if name not in old_names and name.startswith("on_"):
            methods[name] = body_of(new_tokens, name)
    joined, i, calls, returns, ampersands, args_checked = [], 0, [], 0, 0, 0
    found = {"hook": [], "intercept": [], "arm": []}
    last_hook_end = None
    problems = []
    while i < len(caller):
        t = caller[i]
        if t == "self" and caller[i + 1] == "." and caller[i + 2] in methods and caller[i + 3] == "(":
            name = caller[i + 2]
            args, end = args_of(caller, i + 3)
            assert caller[end + 1] == ";", f"call of {name} is not a statement"
            check_args(f"call of {name}", args, [p for p in params_of(new_tokens, name) if p != "self"])
            args_checked += len(args)
            body, depths = methods[name], loop_depths(methods[name])
            for j, tok in enumerate(body):
                if tok == "return" and depths[j] == 0:
                    joined.append("continue")
                    returns += 1
                elif tok == "continue" and depths[j] == 0:
                    raise Refused(f"{name}: `continue` outside a loop")
                elif tok == "effects" and (j + 1 >= len(body) or body[j + 1] != "."):
                    joined += ["&", "mut", "effects"]
                    ampersands += 1
                else:
                    joined.append(tok)
            calls.append(name)
            i = end + 2
            continue
        dropped = False
        for h in intercepts:
            p = tokenize(h["path"])
            if t == "if" and caller[i + 1:i + 1 + len(p)] == p and caller[i + 1 + len(p)] == "(":
                args, end = args_of(caller, i + 1 + len(p))
                if caller[end + 1:end + 5] != ["{", "continue", ";", "}"]:
                    raise Refused(f"{h['path']}: the `if` has another body")
                check_args(h["path"], args, params_of(h["tokens"], p[-1]))
                if joined[-len(LOOP_HEAD):] != LOOP_HEAD:
                    problems.append(f"{h['path']} is not the first statement of the event loop")
                found["intercept"].append(h["path"])
                i = end + 5
                dropped = True
                break
        if dropped:
            continue
        for h in arm_hooks:
            p = tokenize(h["path"])
            head = ["InputEvent", "::", h["variant"], "("]
            if caller[i:i + 4] == head and caller[i + 5:i + 8] == [")", "=>", "{"] and caller[i + 8:i + 8 + len(p)] == p:
                args, end = args_of(caller, i + 8 + len(p))
                check_args(h["path"], args, params_of(h["tokens"], p[-1]))
                if caller[end + 1:end + 3] != [";", "}"]:
                    raise Refused(f"{h['path']}: the arm has another body")
                found["arm"].append(h["path"])
                i = end + 3
                if i < len(caller) and caller[i] == ",":
                    i += 1
                dropped = True
                break
        if dropped:
            continue
        for h in hooks:
            p = tokenize(h["path"])
            if caller[i:i + len(p)] == p and caller[i + len(p)] == "(":
                args, end = args_of(caller, i + len(p))
                assert caller[end + 1] == ";", f"call of {h['path']} is not a statement"
                check_args(h["path"], args, params_of(h["tokens"], p[-1]))
                if not (joined[-len(PRUNE):] == PRUNE or last_hook_end == i):
                    problems.append(f"{h['path']} does not follow the `prune` after the event loop")
                found["hook"].append(h["path"])
                last_hook_end = end + 2
                i = end + 2
                dropped = True
                break
        if dropped:
            continue
        joined.append(t)
        i += 1
    for kind, declared in (("hook", hooks), ("intercept", intercepts), ("arm", arm_hooks)):
        want = [h["path"] for h in declared]
        if found[kind] != want:
            problems.append(f"{kind} calls found {found[kind]}, expected {want} (once each, in this order)")
    if hooks and last_hook_end is not None and caller[last_hook_end] != "let":
        problems.append("the hook calls are not followed by the `let` of the held camera actions")
    old_body, joined = strip_shape(old_body), strip_shape(joined)
    first = next((j for j, (x, y) in enumerate(zip(old_body, joined)) if x != y), None)
    unused = [m for m in methods if m not in calls]
    equal = first is None and len(old_body) == len(joined) and not unused and not problems
    out = [f"old `step` tokens {len(old_body)}, joined tokens {len(joined)}",
           f"calls inlined, in order: {' '.join(calls)}",
           f"arguments checked against parameter names: {args_checked}",
           f"`return` -> `continue` (loop depth 0): {returns}; `effects` -> `& mut effects`: {ampersands}",
           f"hooks dropped: {' '.join(found['hook'] + found['intercept'] + found['arm']) or 'none'}"]
    out += [f"NOT-CALLED: {m}" for m in unused]
    out += [f"PROBLEM: {p}" for p in problems]
    if first is not None or len(old_body) != len(joined):
        at = first if first is not None else min(len(old_body), len(joined))
        out.append(f"DIFFERENT at token {at}")
        out.append("  old:    " + " ".join(old_body[max(0, at - 6):at + 8]))
        out.append("  joined: " + " ".join(joined[max(0, at - 6):at + 8]))
    out.append("RESULT " + ("EQUAL" if equal else "DIFFERENT"))
    if not quiet:
        print("\n".join(out))
    return equal


def hook_files_ok(hooks, intercepts, arm_hooks, intercept_body):
    ok = True
    for h in hooks + arm_hooks:
        body = body_of(h["tokens"], tokenize(h["path"])[-1])
        if body:
            print(f"HOOK-NOT-EMPTY {h['path']}: {len(body)} tokens")
            ok = False
    for h in intercepts:
        body = body_of(h["tokens"], tokenize(h["path"])[-1])
        if body != tokenize(intercept_body):
            print(f"INTERCEPT-BODY {h['path']}: `{' '.join(body)}` is not `{intercept_body}`")
            ok = False
    return ok


def parse_options(argv):
    hooks, intercepts, arm_hooks = [], [], []
    for k, a in enumerate(argv):
        if a == "--hook":
            path, file = argv[k + 1].split("=")
            hooks.append({"path": path, "tokens": tokenize(open(file).read())})
        elif a == "--intercept":
            path, file = argv[k + 1].split("=")
            intercepts.append({"path": path, "tokens": tokenize(open(file).read())})
        elif a == "--arm-hook":
            variant, path, file = argv[k + 1].split("=")
            arm_hooks.append({"variant": variant, "path": path, "tokens": tokenize(open(file).read())})
    return hooks, intercepts, arm_hooks


def self_test(old, new, options):
    hooks, intercepts, arm_hooks = options

    def run(text):
        try:
            return check(old, text, hooks, intercepts, arm_hooks, quiet=True)
        except (Refused, AssertionError, KeyError, IndexError, ValueError):
            return False

    muts = {
        "swap two calls": (r"self\.on_pointer_down\(", "self.on_pointer_up_x("),
        "change a literal": (r"start\.distance\(px\) > 4\.0", "start.distance(px) > 5.0"),
        "drop a return": (r"\n(\s+)return;", r"\n\1"),
        "move a call out of its arm": (r"self\.on_key_up\(code, world, view, &mut effects\);", ""),
        "inner continue to return": (r"(contains\(group\) \{\s*)continue;", r"\1return;"),
        "argument changed to 0": (r"self\.on_ui_action\(input_index,", "self.on_ui_action(0,"),
        "argument replaced by a new Vec": (r"&mut wheel_camera_actions,", "&mut Vec::new(),"),
        "now_ms changed to 0 at a call": (r"(self\.on_pointer_down\(button, px, over_widget, world,) now_ms\)", r"\1 0)"),
        "intercept call removed": (r"\n\s+if placement::intercept\([^;]*?\{\s*continue;\s*\}", ""),
        "intercept moved after the match": (r"(\n\s+if placement::intercept\([^;]*?\{\s*continue;\s*\})(\s+match event \{)", r"\2"),
        "arm hook removed": (r"\n\s+InputEvent::Hint\(input\) => \{[^}]*\}", ""),
    }
    ok = run(new)
    print("unmutated:", "EQUAL" if ok else "DIFFERENT (self-test cannot run)")
    bad = not ok
    for label, (pat, rep) in muts.items():
        mutated, n = re.subn(pat, rep, new, count=1)
        if n != 1:
            print(f"mutation `{label}` did not apply")
            bad = True
            continue
        equal = run(mutated)
        print(f"mutation `{label}`:", "STILL EQUAL (the proof is blind)" if equal else "DIFFERENT -> red")
        bad = bad or equal
    # the hook calls moved in front of the loop (the reviewer's fifth mutation)
    block = re.search(r"(\n\s+placement::update\([^;]*;\n\s+hints::update\([^;]*;\n\s+alerts::update\([^;]*;)", new)
    if block:
        moved = new.replace(block.group(1), "", 1)
        moved = moved.replace("        for (input_index, event) in events.iter().enumerate() {",
                              block.group(1).strip("\n") + "\n        for (input_index, event) in events.iter().enumerate() {", 1)
        equal = run(moved)
        print("mutation `hook calls moved in front of the loop`:", "STILL EQUAL (the proof is blind)" if equal else "DIFFERENT -> red")
        bad = bad or equal
    else:
        print("mutation `hook calls moved` did not apply")
        bad = True
    print("SELF-TEST", "FAILED" if bad else "OK")
    return 1 if bad else 0


def main(argv):
    old = open(argv[argv.index("--old") + 1]).read()
    new = open(argv[argv.index("--new") + 1]).read()
    options = parse_options(argv)
    intercept_body = argv[argv.index("--intercept-body") + 1] if "--intercept-body" in argv else "core.placement.taking_input"
    if "--self-test" in argv:
        return self_test(old, new, options)
    try:
        equal = check(old, new, *options)
    except Refused as err:
        print(f"PROBLEM: {err}\nRESULT DIFFERENT")
        return 1
    return 0 if equal and hook_files_ok(*options, intercept_body) else 1


sys.exit(main(sys.argv))
