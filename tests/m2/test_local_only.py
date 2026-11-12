#!/usr/bin/env python3
"""Tests of the local-only browser cases: the ledger, the check, and the hosted notice."""
import importlib.util
import os
from pathlib import Path
import re
import subprocess
import sys
import tempfile
import unittest

HERE = Path(__file__).resolve().parent


def load(name):
    spec = importlib.util.spec_from_file_location(name, HERE / f"{name}.py")
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    spec.loader.exec_module(module)
    return module


local_only = load("local_only")
suite = load("test_m2_browser_suite")
ROOT = local_only.ROOT
CONVERGENCE = ["E9-browser-base-public-convergence", "E9-browser-haul-public-convergence",
               "E9-browser-acme-public-convergence", "E9-browser-cedar-public-convergence"]
PLANT = "E9-browser-convergence-owner-offset"


def workflow_with(marks, extra=""):
    return f'jobs:\n  browser:\n    env:\n      CARGO_BUILD_JOBS: 2\n      M2_LOCAL_ONLY_CASES: "{marks}"\n{extra}'


class CheckTests(unittest.TestCase):
    def check(self, temporary, marks, ledger_text=None, extra=""):
        workflow = Path(temporary) / "ci.yml"
        workflow.write_text(workflow_with(marks, extra))
        ledger = Path(temporary) / "local-only.tsv"
        ledger.write_text(ledger_text if ledger_text is not None else local_only.LEDGER.read_text())
        return local_only.check(ledger=ledger, workflow=workflow)

    def test_the_real_ledger_and_ci_yml_agree(self):
        self.assertEqual(local_only.check(), [])
        self.assertEqual(sorted(local_only.read_ledger()), sorted(CONVERGENCE + [PLANT]))

    def test_a_case_marked_without_a_ledger_row_is_red(self):
        with tempfile.TemporaryDirectory() as temporary:
            problems = self.check(temporary, ",".join(CONVERGENCE + [PLANT, "E9-browser-base-economy"]))
            self.assertEqual(problems, ["E9-browser-base-economy is marked local-only in ci.yml but has no row in local-only.tsv"])

    def test_a_ledger_row_that_is_not_marked_is_red(self):
        with tempfile.TemporaryDirectory() as temporary:
            problems = self.check(temporary, ",".join(CONVERGENCE))
            self.assertEqual(len(problems), 1)
            self.assertIn(f"ledger row {PLANT} is not marked in ci.yml", problems[0])

    def test_a_case_that_is_not_registered_is_red(self):
        with tempfile.TemporaryDirectory() as temporary:
            ledger = local_only.LEDGER.read_text() + "E9-browser-nothing-here\treason\tlocal E9-browser-nothing-here\treceipt\towner\n"
            problems = self.check(temporary, ",".join(CONVERGENCE + [PLANT, "E9-browser-nothing-here"]), ledger)
            self.assertEqual(sorted(problems), ["E9-browser-nothing-here is marked local-only but is not a registered browser case",
                                                "ledger row E9-browser-nothing-here is not a registered browser case"])

    def test_a_commented_mark_is_not_a_mark_and_bad_ledgers_are_refused(self):
        with tempfile.TemporaryDirectory() as temporary:
            problems = self.check(temporary, ",".join(CONVERGENCE + [PLANT]), extra='      # M2_LOCAL_ONLY_CASES: "E9-browser-base-economy"\n')
            self.assertEqual(problems, [])
            for text, needle in [("case\treason\n", "header"),
                                 (local_only.LEDGER.read_text().replace("\tqa-lead", "\t", 1), "non-empty"),
                                 (local_only.LEDGER.read_text() + local_only.LEDGER.read_text().splitlines()[1] + "\n", "appears twice")]:
                self.assertTrue(any(needle in problem for problem in self.check(temporary, ",".join(CONVERGENCE + [PLANT]), text)), needle)

    def test_a_mark_in_a_form_that_the_check_cannot_read_is_red(self):
        with tempfile.TemporaryDirectory() as temporary:
            workflow = Path(temporary) / "ci.yml"
            workflow.write_text("env:\n  M2_LOCAL_ONLY_CASES: ${{ vars.CASES }}\n")
            problems = local_only.check(workflow=workflow)
            self.assertEqual(len(problems), 1)
            self.assertIn("cannot read", problems[0])


class DecideTests(unittest.TestCase):
    def test_not_listed_runs_listed_with_a_row_is_local_only_and_listed_without_a_row_is_an_error(self):
        with tempfile.TemporaryDirectory() as temporary:
            summary = Path(temporary) / "summary.md"
            env = {"M2_LOCAL_ONLY_CASES": CONVERGENCE[0], "GITHUB_STEP_SUMMARY": str(summary)}
            self.assertEqual(local_only.decide("E9-browser-base-economy", env=env), 0)
            self.assertFalse(summary.exists())
            self.assertEqual(local_only.decide(CONVERGENCE[0], env={}), 0)
            self.assertEqual(local_only.decide(CONVERGENCE[0], env=env), 10)
            text = summary.read_text()
            for needle in (CONVERGENCE[0], "needs a real GPU", "mc_headed_v6.sh", "lock-receipt.json", "qa-lead"):
                self.assertIn(needle, text)
            self.assertEqual(local_only.decide("E9-browser-base-economy", env={"M2_LOCAL_ONLY_CASES": "E9-browser-base-economy"}), 2)
            self.assertEqual(local_only.decide("all", env=env), 2)


class ScriptTests(unittest.TestCase):
    def run_case(self, case, env):
        with tempfile.TemporaryDirectory() as temporary:
            temporary = Path(temporary)
            summary = temporary / "summary.md"
            result = suite.run_default_suite(temporary, {**env, "GITHUB_STEP_SUMMARY": str(summary)}, ("--case", case))
            runs = temporary / "runs"
            return result, (summary.read_text() if summary.exists() else ""), (list(runs.iterdir()) if runs.exists() else [])

    def test_a_listed_case_prints_the_notice_writes_the_summary_and_starts_nothing(self):
        result, summary, runs = self.run_case(CONVERGENCE[0], {"M2_LOCAL_ONLY_CASES": CONVERGENCE[0]})
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertRegex(result.stdout, rf"^M2_CASE_LOCAL_ONLY {CONVERGENCE[0]}: not run on this runner \(needs a real GPU", )
        self.assertIn('"local_only_count":1', result.stdout)
        self.assertNotIn("M2_CASE_START", result.stdout)
        self.assertIn(CONVERGENCE[0], summary)
        self.assertEqual(runs, [], "a local-only case must not create a run folder")

    def test_the_same_case_without_the_mark_goes_the_normal_way(self):
        result, summary, runs = self.run_case(CONVERGENCE[0], {})
        self.assertIn(f"M2_CASE_START {CONVERGENCE[0]}", result.stdout)
        self.assertNotEqual(result.returncode, 0)
        self.assertNotIn("M2_CASE_LOCAL_ONLY", result.stdout)
        self.assertEqual(summary, "")

    def test_a_mark_without_a_ledger_row_is_an_error_not_a_skip(self):
        result, summary, runs = self.run_case("E9-browser-base-selection", {"M2_LOCAL_ONLY_CASES": "E9-browser-base-selection"})
        self.assertEqual(result.returncode, 2)
        self.assertIn("has no row", result.stderr)
        self.assertNotIn("M2_CASE_LOCAL_ONLY", result.stdout)

    def test_the_default_suite_lists_the_four_convergence_cases_and_runs_the_other_eleven(self):
        with tempfile.TemporaryDirectory() as temporary:
            result = suite.run_default_suite(Path(temporary), {"M2_LOCAL_ONLY_CASES": ",".join(CONVERGENCE + [PLANT])})
        started = re.findall(r"^M2_CASE_START (\S+)$", result.stdout, re.M)
        local = re.findall(r"^M2_CASE_LOCAL_ONLY (\S+):", result.stdout, re.M)
        self.assertEqual(sorted(local), sorted(CONVERGENCE))
        self.assertEqual(len(started), 11)
        self.assertFalse(set(started) & set(CONVERGENCE))

    def test_the_refutation_leg_of_a_local_only_case_prints_the_notice_and_runs_nothing(self):
        with tempfile.TemporaryDirectory() as temporary:
            evidence = Path(temporary) / "evidence"
            env = {**os.environ, "M2_LOCAL_ONLY_CASES": ",".join(CONVERGENCE + [PLANT]), "TURTS_REFUTATION_EVIDENCE_DIR": str(evidence),
                   "TURTS_REFUTATION_BUILD_ROOT": str(Path(temporary) / "build")}
            result = subprocess.run(["bash", str(local_only.ROOT / "scripts" / "m2-refutations.sh"), "--case", PLANT], cwd=local_only.ROOT,
                                    env=env, capture_output=True, text=True, timeout=120)
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertIn(f"M2_CASE_LOCAL_ONLY {PLANT}: not run on this runner", result.stdout)
            self.assertFalse(evidence.exists(), "no refutation may start for a local-only case")
            result = subprocess.run(["bash", str(local_only.ROOT / "scripts" / "m2-refutations.sh"), "--case", "E9-browser-base-economy-nothing"],
                                    cwd=local_only.ROOT, env={**env, "M2_LOCAL_ONLY_CASES": ""}, capture_output=True, text=True, timeout=120)
            self.assertNotEqual(result.returncode, 0, "an id that is not local-only goes to refutations.py, which refuses an unknown id")


if __name__ == "__main__":
    unittest.main()
