#!/usr/bin/env python3
"""L0-CORE purity proof for the move of the arm `WidgetAction::CancelProduction` of `activate_widget`
into the function `cancel::activate`.

The tokens of the old arm body (a `,` before a closing bracket ignored) must equal the tokens of the new function body. The arm and the
function end the same way (the `match` is the last expression of `activate_widget`, so each `return`
of the arm returns from the function), so no `return` changes. The parameters `producer` and
`job_id` keep the type `&u32` and `&u64` that the pattern gave them.

usage: move_equal.py OLD_ACTIVATION.rs NEW_CANCEL.rs
"""
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
_src = open(os.path.join(HERE, "..", "l0-split", "stmt_equal.py")).read().replace("sys.exit(main(sys.argv))", "")
_ns = {}
exec(compile(_src, "stmt_equal.py", "exec"), _ns)
tokenize, match, body_of = _ns["tokenize"], _ns["match"], _ns["body_of"]

old = tokenize(open(sys.argv[1]).read())
new = tokenize(open(sys.argv[2]).read())
head = ["primitives", "::", "WidgetAction", "::", "CancelProduction", "{"]
start = next(i for i in range(len(old)) if old[i:i + len(head)] == head)
brace = start + len(head) - 1
close = match(old, brace, "{", "}")
assert old[close + 1:close + 3] == ["=>", "{"], "the arm has another shape"
arm = old[close + 3:match(old, close + 2, "{", "}")]
fn = body_of(new, "activate")


def no_trailing_commas(tokens):
    """rustfmt adds or drops a `,` before `)`, `]` or `}` when code moves to another indentation."""
    return [t for i, t in enumerate(tokens) if not (t == "," and i + 1 < len(tokens) and tokens[i + 1] in (")", "]", "}"))]


arm, fn = no_trailing_commas(arm), no_trailing_commas(fn)
print(f"old arm body tokens {len(arm)}, new function body tokens {len(fn)}, `return` {arm.count('return')} and {fn.count('return')}")
first = next((i for i, (a, b) in enumerate(zip(arm, fn)) if a != b), None)
equal = first is None and len(arm) == len(fn)
if not equal:
    at = first if first is not None else min(len(arm), len(fn))
    print("DIFFERENT at token", at, "\n  old:", " ".join(arm[max(0, at - 5):at + 6]), "\n  new:", " ".join(fn[max(0, at - 5):at + 6]))
print("RESULT", "EQUAL" if equal else "DIFFERENT")
sys.exit(0 if equal else 1)
