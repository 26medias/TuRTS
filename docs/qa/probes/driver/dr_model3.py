# Model of design revision 2, section 4.4: Part A (banned) and Part B (allowed).
# Applies Part A to a driver and, through literal "./x.js" requires, to every local file it needs.
import os, re, sys
BRIDGES = r"turts_(ui|move|debug|scene|\w+)"
ROOTS = r"(process|global|globalThis|window|module)"
CHANNEL_MODULES = {"child_process", "vm", "worker_threads", "inspector", "module", "net", "http",
                   "https", "http2", "dgram", "cluster", "repl"}
NPM_ALLOW = set()  # QA0: empty; playwright-core is never on it
PROCESS_MEMBERS = r"(binding|_linkedBinding|dlopen|mainModule|getBuiltinModule)"
def strip(src):
    return "\n".join(re.sub(r"//.*", "", l) for l in src.split("\n"))
def check_file(path, seen, driver=True):
    if path in seen:
        return []
    seen.add(path)
    code = strip(open(path).read())
    red = []
    name = os.path.basename(path)
    if re.search(r"(?<![\w$])" + BRIDGES, code): red.append(f"A1 bridge ({name})")
    if re.search(r"(?<![\w$.])eval\s*\(", code) or re.search(r"(?<![\w$])eval(?![\w$])", code): red.append(f"A2 eval ({name})")
    if re.search(r"(?<![\w$])Function(?![\w$])", code) or re.search(r"\.constructor(?![\w$])|\[\s*['\"]constructor['\"]\s*\]", code): red.append(f"A2 Function constructor ({name})")
    if re.search(r"(?<![\w$.])(global|globalThis)(?![\w$])", code): red.append(f"A3 global ({name})")
    if re.search(r"(?<![\w$.])" + ROOTS + r"\s*(\?\.)?\s*\[(?!\s*(['\"])[^'\"+`$]*\3\s*\])", code): red.append(f"A4 computed member, non-literal key ({name})")
    if re.search(r"\[[^\]\n]*(['\"`][^\]\n]*\+|\+[^\]\n]*['\"`])[^\]\n]*\]", code) or re.search(r"\[\s*`[^`]*\$\{", code): red.append(f"A4 string-built computed key ({name})")
    if re.search(r"(?<![\w$.])process\s*\.\s*" + PROCESS_MEMBERS + r"(?![\w$])", code): red.append(f"A4 process member ({name})")
    if re.search(r"(?<![\w$.])(fetch|WebSocket)(?![\w$])", code): red.append(f"A6 network global ({name})")
    if re.search(r"(?<![\w$.])module\s*(\?\.)?\s*\.\s*(?!exports(?![\w$]))[\w$]+", code): red.append(f"A4 module member other than exports ({name})")
    if re.search(r"\bimport\s*\(", code): red.append(f"A5 import() ({name})")
    for m in re.finditer(r"(?<![\w$])require\b(\s*\()?", code):
        if not m.group(1):
            red.append(f"A5 require as value ({name})"); continue
        a = re.match(r"""\s*(['"])([^'"`+$]+)\1\s*\)""", code[m.end():])
        if not a:
            red.append(f"A5 non-literal require ({name})"); continue
        mod = a.group(2)
        if mod.startswith("./") or mod.startswith("../"):
            target = os.path.normpath(os.path.join(os.path.dirname(path), mod))
            if not target.endswith(".js"):
                target += ".js"
            red += check_file(target, seen, driver)
        else:
            base = mod[5:] if mod.startswith("node:") else mod
            if driver and base.split("/")[0] == "fs":
                red.append(f"A7 fs in a driver closure {mod} ({name})")
                continue
            if base.split("/")[0] in CHANNEL_MODULES: red.append(f"A6 channel module {mod} ({name})")
            elif not mod.startswith("node:") and mod not in NPM_ALLOW and base.split("/")[0] not in {"assert","fs","zlib","path","util","buffer","crypto","stream","events","os","url","string_decoder","timers"}:
                red.append(f"A6 npm package {mod} ({name})")
    return red
args = sys.argv[1:]
driver = True
if args and args[0] == "--pure":
    driver = False
    args = args[1:]
for f in args:
    r = check_file(f, set(), driver)
    print(f"{os.path.basename(f):24s}", "RED" if r else "GREEN", r)
