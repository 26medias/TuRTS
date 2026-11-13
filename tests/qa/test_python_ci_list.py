#!/usr/bin/env python3
"""Every Python test in the repository runs in CI, or is left out with a reason.

A Python test that no workflow step runs is a test that rots: tests/m2/test_variants.py had two
failures that nobody saw (MC pixel lane re-check, item 6). The files that count are the files under
tests/ and scripts/ named test_*.py or *_test.py, and the scripts that take a --self-test option. A
file counts as run when a `python3 <path>` command of .github/workflows/ci.yml names it. A file that CI
does not run goes in NOT_IN_CI below, with its reason. Standard library only.
"""
import re
import sys
import tempfile
import unittest
from pathlib import Path

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[2]
WORKFLOW = Path(".github/workflows/ci.yml")

# File -> why CI does not run it. A reason has at least 30 characters. An entry for a file that CI runs,
# or for a file that does not exist, is an error (the entry would hide the next change).
NOT_IN_CI = {
    "tests/m2/test_variants.py": "needs a cargo run of the m2_variants example in its own target dir; a cold build on the hosted runner is not measured",
}

SCAN_DIRS = ("tests", "scripts")
SKIP_PARTS = {"node_modules", "target", "__pycache__"}


def discover(root):
    """The Python files that must run in CI, as paths relative to `root`."""
    found = set()
    for directory in SCAN_DIRS:
        base = root / directory
        if not base.is_dir():
            continue
        for path in base.rglob("*.py"):
            if SKIP_PARTS & set(path.relative_to(root).parts):
                continue
            name = path.name
            text = path.read_text(encoding="utf-8", errors="replace")
            if name.startswith("test_") or name.endswith("_test.py") or "--self-test" in text:
                found.add(path.relative_to(root).as_posix())
    return found


def ci_listed(workflow_text):
    """The `.py` paths that a `python3` command of the workflow names."""
    listed = set()
    for line in workflow_text.splitlines():
        if line.lstrip().startswith("#"):
            continue  # a commented-out step does not run
        for command in re.finditer(r"python3\s+((?:-\S+\s+)*)((?:tests|scripts)/[\w./-]+\.py)", line):
            listed.add(command.group(2))
    return listed


def problems(root, not_in_ci):
    """The list of faults. An empty list is a pass."""
    workflow = (root / WORKFLOW).read_text(encoding="utf-8")
    listed = ci_listed(workflow)
    found = discover(root)
    faults = []
    for path in sorted(found - listed - set(not_in_ci)):
        faults.append(f"{path}: no CI step runs it and it has no NOT_IN_CI entry")
    for path in sorted(listed):
        if not (root / path).is_file():
            faults.append(f"{path}: a CI step runs a file that does not exist")
    for path, reason in sorted(not_in_ci.items()):
        if len(reason) < 30:
            faults.append(f"{path}: the NOT_IN_CI reason is shorter than 30 characters")
        if not (root / path).is_file():
            faults.append(f"{path}: a NOT_IN_CI entry for a file that does not exist")
        if path in listed:
            faults.append(f"{path}: a NOT_IN_CI entry for a file that CI runs")
    return faults


class PythonCiListTests(unittest.TestCase):
    def test_every_python_test_runs_in_ci_or_has_a_reason(self):
        self.assertEqual(problems(ROOT, NOT_IN_CI), [])

    def test_the_real_discovery_finds_the_known_files(self):
        found = discover(ROOT)
        # The files that this check was written for. A discovery that finds none cannot fail.
        for path in ("tests/m2/test_variants.py", "tests/m2/test_refutations.py", "tests/mc/test_mc_browser_params.py",
                     "scripts/qa/completeness.py", "scripts/qa/driver_rule.py"):
            self.assertIn(path, found)
        self.assertGreaterEqual(len(ci_listed((ROOT / WORKFLOW).read_text(encoding="utf-8"))), 8)

    # The tests below build a small repository and break it, so each fault is seen red.
    def repo(self, workflow, files):
        temp = tempfile.TemporaryDirectory(prefix="turts-pylist-")
        self.addCleanup(temp.cleanup)
        root = Path(temp.name)
        (root / WORKFLOW).parent.mkdir(parents=True)
        (root / WORKFLOW).write_text(workflow, encoding="utf-8")
        for name, text in files.items():
            (root / name).parent.mkdir(parents=True, exist_ok=True)
            (root / name).write_text(text, encoding="utf-8")
        return root

    def test_a_test_file_that_no_step_runs_is_red(self):
        root = self.repo("steps:\n  - run: python3 tests/a/test_one.py\n",
                         {"tests/a/test_one.py": "", "tests/a/test_two.py": "", "scripts/tool_test.py": ""})
        self.assertEqual(problems(root, {}), [
            "scripts/tool_test.py: no CI step runs it and it has no NOT_IN_CI entry",
            "tests/a/test_two.py: no CI step runs it and it has no NOT_IN_CI entry",
        ])

    def test_a_self_test_script_that_no_step_runs_is_red(self):
        # A script with a --self-test option is a test. Run with any option, it counts as run.
        root = self.repo("steps:\n  - run: python3 -B scripts/x.py --check\n",
                         {"scripts/x.py": "parser.add_argument('--self-test')\n", "scripts/y.py": "print('no option')\n"})
        self.assertEqual(problems(root, {}), [])
        root = self.repo("steps:\n  - run: echo none\n", {"scripts/x.py": "parser.add_argument('--self-test')\n"})
        self.assertEqual(problems(root, {}), ["scripts/x.py: no CI step runs it and it has no NOT_IN_CI entry"])

    def test_a_not_in_ci_entry_needs_a_reason_a_file_and_must_not_hide_a_ci_file(self):
        root = self.repo("steps:\n  - run: python3 tests/a/test_one.py\n", {"tests/a/test_one.py": "", "tests/a/test_two.py": ""})
        reason = "needs a cargo build in its own target directory"
        self.assertEqual(problems(root, {"tests/a/test_two.py": reason}), [])
        self.assertEqual(problems(root, {"tests/a/test_two.py": "too short"}),
                         ["tests/a/test_two.py: the NOT_IN_CI reason is shorter than 30 characters"])
        self.assertEqual(problems(root, {"tests/a/test_two.py": reason, "tests/a/gone.py": reason}),
                         ["tests/a/gone.py: a NOT_IN_CI entry for a file that does not exist"])
        self.assertEqual(problems(root, {"tests/a/test_two.py": reason, "tests/a/test_one.py": reason}),
                         ["tests/a/test_one.py: a NOT_IN_CI entry for a file that CI runs"])

    def test_a_commented_out_step_is_not_listed(self):
        files = {"tests/a/test_one.py": ""}
        root = self.repo("steps:\n  - run: python3 tests/a/test_one.py\n", files)
        self.assertEqual(problems(root, {}), [])
        root = self.repo("steps:\n  # - run: python3 tests/a/test_one.py\n    #   python3 tests/a/test_one.py\n", files)
        self.assertEqual(problems(root, {}), ["tests/a/test_one.py: no CI step runs it and it has no NOT_IN_CI entry"])
        self.assertEqual(ci_listed("      # python3 tests/m2/test_x.py\n      python3 tests/m2/test_y.py\n"), {"tests/m2/test_y.py"})

    def test_a_step_that_names_a_missing_file_is_red(self):
        root = self.repo("steps:\n  - run: python3 tests/a/test_gone.py\n", {})
        self.assertEqual(problems(root, {}), ["tests/a/test_gone.py: a CI step runs a file that does not exist"])


if __name__ == "__main__":
    unittest.main(verbosity=2)
