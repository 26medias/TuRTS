#!/usr/bin/env python3
"""The default suite of scripts/m2-browser.sh runs all 15 registered cases, also after a red one.

Hosted run 43214215645: the base package failed, `set -e` ended the suite, and the 11 cases of
haul, acme, cedar and the five loss families never ran, so the job showed 4 of 15 cases.
The test needs no browser and no build. Every case stops at the check for the missing server
binary, which is one real exit path of the script (the case reports M2_CASE_INFRA_FAIL).
"""
from pathlib import Path
import json
import os
import re
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
SCRIPT = ROOT / "scripts" / "m2-browser.sh"


def run_default_suite(temporary: Path, extra_env=None, args=()) -> subprocess.CompletedProcess:
    logs = temporary / "prebuilt-logs"
    logs.mkdir()
    for name in ("server-build.log", "packer-build.log", "trunk-build.log"):
        (logs / name).write_text("fixture\n")
    env = os.environ.copy()
    env.update(
        M2_PREBUILT="1",
        M2_PREBUILT_LOGS=str(logs),
        M2_CARGO_TARGET_DIR=str(temporary / "empty-target"),
        M2_ARTIFACT_ROOT=str(temporary / "runs"),
        CHROME_BIN="/bin/true",
    )
    for name in ("M2_PACKAGE_NAME", "M2_CASE_ID", "M2_LOCAL_ONLY_CASES", "GITHUB_STEP_SUMMARY", "M2_RECIPE_DURATION_SCALE", "GITHUB_ACTIONS"):
        env.pop(name, None)
    env.update(extra_env or {})
    return subprocess.run(["bash", str(SCRIPT), *args], cwd=ROOT, env=env, capture_output=True, text=True, timeout=300)


class DefaultSuiteTests(unittest.TestCase):
    def test_a_red_case_does_not_hide_the_other_cases(self):
        listed = subprocess.run(["bash", str(SCRIPT), "--list-cases"], cwd=ROOT, capture_output=True, text=True, check=True).stdout.split()
        self.assertEqual(len(listed), 15)
        with tempfile.TemporaryDirectory() as temporary:
            result = run_default_suite(Path(temporary))
        output = result.stdout + result.stderr
        started = re.findall(r"^M2_CASE_START (\S+)$", output, re.M)
        self.assertEqual(sorted(started), sorted(listed), "every registered case must start")
        self.assertNotEqual(result.returncode, 0, "a suite with red cases must exit non-zero")
        summaries = re.findall(r"^M2_DEFAULT_SUITE_SUMMARY (\{.*\})$", result.stdout, re.M)
        self.assertEqual(len(summaries), 1, "the summary line must print also when cases are red")
        summary = json.loads(summaries[0])
        self.assertEqual((summary["selected_count"], summary["completed_count"], summary["passed_count"], summary["infrastructure_failed_count"],
                          summary["local_only_count"]), (15, 15, 0, 15, 0))


if __name__ == "__main__":
    unittest.main()
