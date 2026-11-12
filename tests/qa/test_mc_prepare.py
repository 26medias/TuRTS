#!/usr/bin/env python3
"""Tests of the prepare step and the prepared run (MP1 Q-PREBUILD).

Claim under test: a prepared run builds nothing, and it refuses (exit 66) when the prepared folder
does not match the present source, so a stale binary is never tested by mistake.

Each test makes a small fake git repository with copies of the REAL scripts, and puts fake `cargo`,
`trunk`, `brotli` and `node` on PATH. The fake builders make stub files. A prepared run gets
failing shims for `cargo`, `trunk`, `brotli` and `npm`: each one writes a line to a log, then exits
99. The log must stay empty. No GPU, no browser, and no real build is used.

Run: python3 -B tests/qa/test_mc_prepare.py     (Python 3 standard library only)
"""
import json
import os
import re
import select
import shutil
import signal
import subprocess
import sys
import tempfile
import time
import unittest
import urllib.request
from pathlib import Path

sys.dont_write_bytecode = True
REAL = Path(__file__).resolve().parents[2]
SCRIPT_FILES = ["mc-browser.sh", "mc-pixels.sh", "t7.sh", "m3-browser.sh", "m2-browser.sh", "mc_prepare.py", "mc_browser_params.py"]
STALE = 66

SERVER_STUB = '''#!/usr/bin/env python3
import http.server, re, sys
text = open(sys.argv[1]).read()
port = int(re.search(r'http_listen: "127.0.0.1:(\\d+)"', text).group(1))
class H(http.server.BaseHTTPRequestHandler):
    def do_GET(self):
        self.send_response(200); self.end_headers(); self.wfile.write(b"{}")
    def log_message(self, *a): pass
http.server.HTTPServer(("127.0.0.1", port), H).serve_forever()
'''
CARGO_OK = r'''#!/usr/bin/env bash
echo "cargo $*" >> "$FAKE_LOG"
[[ -z "${FAKE_MUTATE:-}" ]] || echo "// changed while the build ran" >> "$FAKE_MUTATE"
case "$1" in
build)
    mkdir -p "$CARGO_TARGET_DIR/release"
    case " $* " in
    *" turts_server "*) cp "$FAKE_STUBS/turts_server" "$CARGO_TARGET_DIR/release/turts_server" ;;
    *" turts_pack "*) cp "$FAKE_STUBS/turts-pack" "$CARGO_TARGET_DIR/release/turts-pack" ;;
    esac ;;
run)
    dir="${@: -1}"
    tar -cf "$dir.tar" -C "$(dirname "$dir")" "$(basename "$dir")"
    echo "$dir.tar $(sha256sum "$dir.tar" | cut -d' ' -f1)" ;;
test)
    exe="$CARGO_TARGET_DIR/release/deps/t7_oracle-fake"
    mkdir -p "$(dirname "$exe")"
    printf '#!/usr/bin/env bash\ntouch "$T7_DIR/oracle-ran"\n' > "$exe"; chmod +x "$exe"
    printf '{"reason":"compiler-artifact","target":{"name":"t7_oracle"},"executable":"%s"}\n' "$exe" ;;
esac
exit 0
'''
TRUNK_OK = r'''#!/usr/bin/env bash
echo "trunk $*" >> "$FAKE_LOG"
mkdir -p dist
echo "<html>fake page</html>" > dist/index.html
echo "// js" > dist/app.js
echo "wasm" > dist/app_bg.wasm
'''
BROTLI_OK = '#!/usr/bin/env bash\necho "brotli $*" >> "$FAKE_LOG"\nfor f in "$@"; do [[ -f "$f" ]] && cp "$f" "$f.br"; done\nexit 0\n'
PACKER_STUB = '''#!/usr/bin/env bash
tar -cf "$1.tar" -C "$(dirname "$1")" "$(basename "$1")"
echo "$1.tar $(sha256sum "$1.tar" | cut -d' ' -f1)"
'''
NPM_OK = '#!/usr/bin/env bash\necho "npm $*" >> "$FAKE_LOG"\nexit 0\n'
FAIL_SHIM = '#!/usr/bin/env bash\necho "{name} $*" >> "$FAKE_LOG"\nexit 99\n'
# A fake `node`: a call of harness/run.js writes the diagnostics file that m3-browser.sh reads (run.js PAGE OUT ...).
NODE_FAKE = '''#!/usr/bin/env bash
echo "node $*" >> "$FAKE_NODE_LOG"
if [[ "$1" == *harness/run.js ]]; then echo '{"counts_by_kind":{}}' > "$3/browser-diagnostics.json"; fi
exit 0
'''


def write_exe(path, text):
    Path(path).write_text(text)
    Path(path).chmod(0o755)


def sh(cwd, *argv):
    return subprocess.run(argv, cwd=cwd, check=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True).stdout


class Fake:
    """A fake repository, the stubs, the shims and the run helpers."""

    def __init__(self):
        self.tmp = Path(tempfile.mkdtemp(prefix="mc-prepare-test-"))
        self.repo = self.tmp / "repo"
        self.log = self.tmp / "tool.log"
        self.node_log = self.tmp / "node.log"
        self.make_repo()
        self.make_shims()

    def make_repo(self):
        repo = self.repo
        for rel, text in {
            ".gitignore": "/target\n/tests/browser/node_modules/\n/crates/turts_client_web/dist/\n",
            "Cargo.toml": "[workspace]\n", "Cargo.lock": "# lock\n", "rust-toolchain.toml": "[toolchain]\n",
            "crates/turts_server/src/main.rs": "fn main() {}\n",
            "crates/turts_client_web/index.html": "<html></html>\n",
            "mods/pkg/pack.ron": "(id: \"pkg\")\n", "mods/pkg/units/a.ron": "(a: 1)\n",
            "mods/other/pack.ron": "(id: \"other\")\n", "mods/acme/pack.ron": "(id: \"acme\")\n", "mods/forge/pack.ron": "(id: \"forge\")\n",
            "tests/browser/package.json": "{}\n", "tests/browser/harness/run.js": "// fake\n",
            "tests/browser/t7-frames.js": "// fake\n", "tests/browser/d.js": "// d\n", "tests/browser/o.js": "// o\n",
            "tests/browser/rows/X.json": json.dumps({"row": "X", "package": "mods/pkg", "driver": "tests/browser/d.js",
                                                     "oracle": "tests/browser/o.js", "viewports": ["1280x720"]}),
        }.items():
            (repo / rel).parent.mkdir(parents=True, exist_ok=True)
            (repo / rel).write_text(text)
        (repo / "scripts").mkdir()
        for name in SCRIPT_FILES:
            shutil.copy(REAL / "scripts" / name, repo / "scripts" / name)
        pkg = repo / "tests/browser/node_modules/playwright-core"
        pkg.mkdir(parents=True)
        (pkg / "package.json").write_text('{"name":"playwright-core","main":"index.js"}\n')
        (pkg / "index.js").write_text("module.exports = {};\n")
        sh(repo, "git", "init", "-q")
        sh(repo, "git", "-c", "user.name=t", "-c", "user.email=t@t", "add", "-A")
        sh(repo, "git", "-c", "user.name=t", "-c", "user.email=t@t", "commit", "-q", "-m", "base")

    def make_shims(self):
        self.stubs = self.tmp / "stubs"
        self.stubs.mkdir()
        write_exe(self.stubs / "turts_server", SERVER_STUB)
        write_exe(self.stubs / "turts-pack", PACKER_STUB)
        self.ok_bin, self.fail_bin, self.node_bin = self.tmp / "okbin", self.tmp / "failbin", self.tmp / "nodebin"
        for directory in (self.ok_bin, self.fail_bin, self.node_bin):
            directory.mkdir()
        write_exe(self.ok_bin / "cargo", CARGO_OK)
        write_exe(self.ok_bin / "trunk", TRUNK_OK)
        write_exe(self.ok_bin / "brotli", BROTLI_OK)
        write_exe(self.ok_bin / "npm", NPM_OK)
        for name in ("cargo", "trunk", "brotli", "npm"):
            write_exe(self.fail_bin / name, FAIL_SHIM.format(name=name))
        write_exe(self.node_bin / "node", NODE_FAKE)

    def env(self, bin_dir, fake_node=False, **extra):
        env = dict(os.environ)
        parts = [str(bin_dir)] + ([str(self.node_bin)] if fake_node else [])
        env.update(PATH=os.pathsep.join(parts + [env["PATH"]]), FAKE_LOG=str(self.log), FAKE_NODE_LOG=str(self.node_log),
                   FAKE_STUBS=str(self.stubs), CARGO_TARGET_DIR=str(self.tmp / "target"), CHROME_BIN="/bin/true",
                   MC_BROWSER_OUT=str(self.tmp / "runs"), M3_T7_OUT=str(self.tmp / "t7run"),
                   M3_ARTIFACT_ROOT=str(self.tmp / "m3runs"), M2_ARTIFACT_ROOT=str(self.tmp / "m2runs"), M2_HEADLESS="true", PYTHONDONTWRITEBYTECODE="1")
        env.update(extra)
        return env

    def run(self, argv, bin_dir, fake_node=False, timeout=90, **extra):
        return subprocess.run(["bash", *argv], cwd=self.repo, env=self.env(bin_dir, fake_node, **extra), text=True,
                              stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=timeout)

    def serve(self, argv, bin_dir):
        """Start a `--serve-only` run. Return (process, the lines read up to 'URL:', the page URL or None).
        With no URL the process has ended, or gave no line in 60 s (the caller must stop it)."""
        proc = subprocess.Popen(["bash", *argv], cwd=self.repo, env=self.env(bin_dir), text=True, start_new_session=True,
                                stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        lines, deadline = [], time.time() + 60
        while time.time() < deadline:
            if not select.select([proc.stdout], [], [], 1)[0]:
                continue
            line = proc.stdout.readline()
            if not line:
                break
            lines.append(line)
            if line.startswith("URL: "):
                return proc, lines, line[5:].strip()
        return proc, lines, None

    @staticmethod
    def stop(proc):
        try:
            os.killpg(proc.pid, signal.SIGTERM)
        except ProcessLookupError:
            pass
        try:
            proc.wait(timeout=15)
        except subprocess.TimeoutExpired:
            os.killpg(proc.pid, signal.SIGKILL)
            proc.wait()
        proc.stdout.close()
        proc.stderr.close()

    def prepare(self, name="prep", extra=(), package="mods/pkg"):
        done = self.run(["scripts/mc-browser.sh", package, "--prepare", str(self.tmp / name), *extra], self.ok_bin)
        assert done.returncode == 0, done.stdout + done.stderr
        self.log.unlink(missing_ok=True)
        return self.tmp / name

    def tool_log(self):
        return self.log.read_text() if self.log.exists() else ""

    def commit(self, message="x"):
        sh(self.repo, "git", "-c", "user.name=t", "-c", "user.email=t@t", "commit", "-q", "--allow-empty", "-am", message)


class PrepareTests(unittest.TestCase):
    def setUp(self):
        self.fake = Fake()
        self.addCleanup(shutil.rmtree, self.fake.tmp, ignore_errors=True)

    def served(self, prepared, package="mods/pkg"):
        """A prepared `--serve-only` run with failing build shims. Return (proc, lines, url)."""
        return self.fake.serve(["scripts/mc-browser.sh", package, "--prepared", str(prepared), "--serve-only"], self.fake.fail_bin)

    def refused(self, prepared, text, absent=(), package="mods/pkg"):
        """A prepared run must stop with exit 66, print `text`, start no service and no build."""
        proc, lines, url = self.served(prepared, package)
        if url is not None:
            self.fake.stop(proc)
            self.fail("a stale prepared run started its services: " + "".join(lines))
        try:
            code = proc.wait(timeout=30)
        except subprocess.TimeoutExpired:
            self.fake.stop(proc)
            self.fail("the stale prepared run neither ended nor started its services: " + "".join(lines))
        stderr = proc.stderr.read()
        proc.stdout.close()
        proc.stderr.close()
        self.assertEqual(code, STALE, "".join(lines) + stderr)
        self.assertIn(text, stderr)
        for other in absent:
            self.assertNotIn(other, stderr)
        self.assertEqual(self.fake.tool_log(), "")

    def test_default_form_still_builds_inside_the_run(self):
        proc, lines, url = self.fake.serve(["scripts/mc-browser.sh", "mods/pkg", "--serve-only"], self.fake.ok_bin)
        self.fake.stop(proc)
        self.assertIsNotNone(url, "".join(lines))
        log = self.fake.tool_log()
        self.assertIn("cargo build --release -p turts_server", log)
        self.assertIn("trunk build --release", log)

    def test_prepare_writes_the_manifest(self):
        prepared = self.fake.prepare()
        manifest = json.loads((prepared / "manifest.json").read_text())
        self.assertEqual(manifest["schema"], "mc-prepared-v1")
        self.assertEqual(manifest["head"], sh(self.fake.repo, "git", "rev-parse", "HEAD").strip())
        self.assertFalse(manifest["dirty"])
        for rel in ("bin/turts_server", "bin/turts-pack", "dist/index.html", "pack/pkg.tar", "pack/pkg/pack.ron"):
            self.assertIn(rel, manifest["artefacts"])
        self.assertFalse((prepared / ".begin.json").exists())

    def test_prepared_run_starts_no_cargo_and_no_trunk(self):
        prepared = self.fake.prepare()
        proc, lines, url = self.served(prepared)
        try:
            self.assertIsNotNone(url, "the prepared run did not reach the page: " + "".join(lines))
            page = urllib.request.urlopen(url, timeout=10)
            self.assertEqual(page.status, 200)
            self.assertIn(b"fake page", page.read())
        finally:
            self.fake.stop(proc)
        self.assertEqual(self.fake.tool_log(), "", "a prepared run started cargo, trunk, brotli or npm")

    def test_prepared_run_uses_the_prepared_copies(self):
        """The binary of the cache and the dist of the lane can change after prepare; the run must not use them."""
        prepared = self.fake.prepare()
        write_exe(self.fake.tmp / "target/release/turts_server", "#!/bin/sh\nexit 1\n")
        lane_dist = self.fake.repo / "crates/turts_client_web/dist"
        lane_dist.mkdir(parents=True, exist_ok=True)
        (lane_dist / "index.html").write_text("<html>WRONG PAGE from the lane dist</html>\n")
        proc, lines, url = self.served(prepared)
        try:
            self.assertIsNotNone(url, "the prepared run did not reach the page: " + "".join(lines))
            body = urllib.request.urlopen(url, timeout=10).read()
            self.assertIn(b"fake page", body)
            self.assertNotIn(b"WRONG PAGE", body)
            self.assertEqual(urllib.request.urlopen(re.search(r"server=([^&]+)", url).group(1).replace("%3A", ":").replace("%2F", "/")
                                                    + "/advertisement.json", timeout=10).status, 200)
        finally:
            self.fake.stop(proc)

    def test_source_edit_after_prepare_is_refused(self):
        prepared = self.fake.prepare()
        with open(self.fake.repo / "crates/turts_server/src/main.rs", "a") as handle:
            handle.write("// edited after prepare\n")
        self.refused(prepared, "build source differs (1 file(s): crates/turts_server/src/main.rs)")

    def test_new_untracked_source_file_is_refused(self):
        prepared = self.fake.prepare()
        (self.fake.repo / "crates/turts_server/src/extra.rs").write_text("// new\n")
        self.refused(prepared, "crates/turts_server/src/extra.rs")

    def test_package_change_is_refused(self):
        prepared = self.fake.prepare()
        (self.fake.repo / "mods/pkg/units/a.ron").write_text("(a: 2)\n")
        self.refused(prepared, "package content differs (mods/pkg)", absent=["build source differs", "HEAD differs"])

    def test_head_change_is_refused(self):
        prepared = self.fake.prepare()
        self.fake.commit("empty commit")
        self.refused(prepared, "HEAD differs", absent=["build source differs", "package content differs"])

    def test_changed_artefact_is_refused(self):
        prepared = self.fake.prepare()
        (prepared / "bin/turts_server").write_text("#!/bin/sh\nexit 0\n")
        self.refused(prepared, "artefact bin/turts_server changed after the prepare step")

    def test_prepared_for_another_package_is_refused(self):
        prepared = self.fake.prepare()
        self.refused(prepared, "package differs (prepared mods/pkg, run mods/other)", package="mods/other")

    def test_folder_without_manifest_is_refused(self):
        prepared = self.fake.prepare()
        (prepared / "manifest.json").unlink()
        self.refused(prepared, "no manifest.json")

    def test_source_change_while_prepare_runs_makes_no_manifest(self):
        done = self.fake.run(["scripts/mc-browser.sh", "mods/pkg", "--prepare", str(self.fake.tmp / "mid")], self.fake.ok_bin,
                             FAKE_MUTATE=str(self.fake.repo / "crates/turts_server/src/main.rs"))
        self.assertEqual(done.returncode, STALE, done.stdout + done.stderr)
        self.assertIn("the source changed while the prepare step ran", done.stderr)
        self.assertFalse((self.fake.tmp / "mid/manifest.json").exists())

    def test_prepare_refuses_a_folder_that_is_not_a_prepared_folder(self):
        other = self.fake.tmp / "precious"
        other.mkdir()
        (other / "keep.txt").write_text("keep")
        done = self.fake.run(["scripts/mc-browser.sh", "mods/pkg", "--prepare", str(other)], self.fake.ok_bin)
        self.assertEqual(done.returncode, 2, done.stdout + done.stderr)
        self.assertEqual((other / "keep.txt").read_text(), "keep")

    def test_prepare_and_prepared_together_are_refused(self):
        done = self.fake.run(["scripts/mc-browser.sh", "mods/pkg", "--prepare", "a", "--prepared", "b"], self.fake.ok_bin)
        self.assertEqual(done.returncode, 2)
        self.assertIn("cannot be used together", done.stderr)

    def test_pixels_prepare_then_prepared_run_builds_nothing(self):
        done = self.fake.run(["scripts/mc-pixels.sh", "--prepare", str(self.fake.tmp / "px"), "tests/browser/rows/X.json"], self.fake.ok_bin)
        self.assertEqual(done.returncode, 0, done.stdout + done.stderr)
        manifest = json.loads((self.fake.tmp / "px/manifest.json").read_text())
        self.assertEqual(manifest["row"]["row"], "X")
        self.fake.log.unlink(missing_ok=True)
        done = self.fake.run(["scripts/mc-pixels.sh", "--prepared", str(self.fake.tmp / "px"), "tests/browser/rows/X.json"],
                             self.fake.fail_bin, fake_node=True)
        self.assertEqual(done.returncode, 0, done.stdout + done.stderr)
        self.assertIn("PASS X", done.stdout)
        self.assertEqual(self.fake.tool_log(), "")
        self.assertEqual(self.fake.node_log.read_text().count("harness/run.js"), 2, "two runs per viewport")

    def test_pixels_stale_folder_stops_at_once_with_exit_66(self):
        self.fake.run(["scripts/mc-pixels.sh", "--prepare", str(self.fake.tmp / "px"), "tests/browser/rows/X.json"], self.fake.ok_bin)
        (self.fake.repo / "crates/turts_server/src/main.rs").write_text("fn main() { /* edit */ }\n")
        done = self.fake.run(["scripts/mc-pixels.sh", "--prepared", str(self.fake.tmp / "px"), "tests/browser/rows/X.json"],
                             self.fake.fail_bin, fake_node=True)
        self.assertEqual(done.returncode, STALE, done.stdout + done.stderr)
        self.assertEqual(done.stdout.count("ROW X"), 1, "the loop must stop at the first refusal")
        self.assertFalse(self.fake.node_log.exists(), "the browser must not start")

    def test_t7_prepare_then_prepared_run_builds_nothing(self):
        done = self.fake.run(["scripts/t7.sh", "--prepare", str(self.fake.tmp / "t7p")], self.fake.ok_bin)
        self.assertEqual(done.returncode, 0, done.stdout + done.stderr)
        manifest = json.loads((self.fake.tmp / "t7p/manifest.json").read_text())
        for rel in ("bin/turts_server", "bin/t7_oracle", "dist/index.html", "dist/app.js.br", "pack/acme.tar"):
            self.assertIn(rel, manifest["artefacts"])
        self.fake.log.unlink(missing_ok=True)
        done = self.fake.run(["scripts/t7.sh", "--prepared", str(self.fake.tmp / "t7p")], self.fake.fail_bin, fake_node=True)
        self.assertEqual(done.returncode, 0, done.stdout + done.stderr)
        self.assertEqual(self.fake.tool_log(), "")
        self.assertIn("harness/run.js", self.fake.node_log.read_text())
        self.assertTrue((self.fake.tmp / "t7run/oracle-ran").exists(), "the prepared oracle binary did not run")

    def test_t7_stale_folder_is_refused(self):
        self.fake.run(["scripts/t7.sh", "--prepare", str(self.fake.tmp / "t7p")], self.fake.ok_bin)
        (self.fake.repo / "crates/turts_client_web/index.html").write_text("<html>edited</html>\n")
        self.fake.log.unlink(missing_ok=True)
        done = self.fake.run(["scripts/t7.sh", "--prepared", str(self.fake.tmp / "t7p")], self.fake.fail_bin, fake_node=True)
        self.assertEqual(done.returncode, STALE, done.stdout + done.stderr)
        self.assertIn("build source differs (1 file(s): crates/turts_client_web/index.html)", done.stderr)
        self.assertEqual(self.fake.tool_log(), "")
        self.assertFalse(self.fake.node_log.exists())

    def test_m3_prepare_then_prepared_run_builds_nothing(self):
        done = self.fake.run(["scripts/m3-browser.sh", "--prepare", str(self.fake.tmp / "m3p")], self.fake.ok_bin)
        self.assertEqual(done.returncode, 0, done.stdout + done.stderr)
        manifest = json.loads((self.fake.tmp / "m3p/manifest.json").read_text())
        for rel in ("bin/turts_server", "dist/index.html", "pack/forge.tar"):
            self.assertIn(rel, manifest["artefacts"])
        self.assertNotIn("harness/run.js", self.fake.node_log.read_text() if self.fake.node_log.exists() else "")
        self.fake.log.unlink(missing_ok=True)
        done = self.fake.run(["scripts/m3-browser.sh", "--prepared", str(self.fake.tmp / "m3p")], self.fake.fail_bin, fake_node=True)
        self.assertEqual(done.returncode, 0, done.stdout + done.stderr)
        self.assertEqual(self.fake.tool_log(), "")
        self.assertIn("harness/run.js", self.fake.node_log.read_text())

    def test_m3_stale_folder_is_refused(self):
        self.fake.run(["scripts/m3-browser.sh", "--prepare", str(self.fake.tmp / "m3p")], self.fake.ok_bin)
        (self.fake.repo / "mods/forge/pack.ron").write_text("(id: \"forge2\")\n")
        self.fake.log.unlink(missing_ok=True)
        done = self.fake.run(["scripts/m3-browser.sh", "--prepared", str(self.fake.tmp / "m3p")], self.fake.fail_bin, fake_node=True)
        self.assertEqual(done.returncode, STALE, done.stdout + done.stderr)
        self.assertIn("package content differs (mods/forge)", done.stderr)
        self.assertEqual(self.fake.tool_log(), "")
        self.assertFalse(self.fake.node_log.exists())

    def test_m3_default_form_still_builds_inside(self):
        done = self.fake.run(["scripts/m3-browser.sh"], self.fake.ok_bin, fake_node=True)
        self.assertEqual(done.returncode, 0, done.stdout + done.stderr)
        log = self.fake.tool_log()
        for text in ("cargo build --release -p turts_server", "trunk build --release", "npm ci --offline"):
            self.assertIn(text, log)

    def test_m2_prepare_then_prepared_list_cases_builds_nothing(self):
        done = self.fake.run(["scripts/m2-browser.sh", "--prepare", str(self.fake.tmp / "m2p")], self.fake.ok_bin)
        self.assertEqual(done.returncode, 0, done.stdout + done.stderr)
        manifest = json.loads((self.fake.tmp / "m2p/manifest.json").read_text())
        for rel in ("bin/turts_server", "bin/turts-pack", "dist/index.html"):
            self.assertIn(rel, manifest["artefacts"])
        self.fake.log.unlink(missing_ok=True)
        done = self.fake.run(["scripts/m2-browser.sh", "--prepared", str(self.fake.tmp / "m2p"), "--list-cases"], self.fake.fail_bin)
        self.assertEqual(done.returncode, 0, done.stdout + done.stderr)
        self.assertEqual(len([line for line in done.stdout.split() if line.startswith("E9-browser-")]), 15)
        self.assertEqual(self.fake.tool_log(), "")

    def test_m2_stale_folder_is_refused_before_any_case(self):
        self.fake.run(["scripts/m2-browser.sh", "--prepare", str(self.fake.tmp / "m2p")], self.fake.ok_bin)
        (self.fake.repo / "mods/other/pack.ron").write_text("(id: \"other2\")\n")
        self.fake.log.unlink(missing_ok=True)
        done = self.fake.run(["scripts/m2-browser.sh", "--prepared", str(self.fake.tmp / "m2p"), "--list-cases"], self.fake.fail_bin)
        self.assertEqual(done.returncode, STALE, done.stdout + done.stderr)
        self.assertIn("package content differs (mods)", done.stderr)
        self.assertEqual(done.stdout, "")
        self.assertEqual(self.fake.tool_log(), "")

    def test_t7_default_form_still_builds_inside(self):
        done = self.fake.run(["scripts/t7.sh"], self.fake.ok_bin, fake_node=True)
        self.assertEqual(done.returncode, 0, done.stdout + done.stderr)
        log = self.fake.tool_log()
        self.assertIn("cargo build --release -p turts_server", log)
        self.assertIn("trunk build --release", log)
        self.assertIn("cargo test --release -p turts_server --test t7_oracle", log)


if __name__ == "__main__":
    unittest.main(verbosity=2)
