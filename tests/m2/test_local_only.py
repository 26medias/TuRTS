#!/usr/bin/env python3
"""Tests of the local-only browser cases: the ledger, the check, and the hosted notice."""
import hashlib
import importlib.util
import json
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
                                 (local_only.LEDGER.read_text() + [line for line in local_only.LEDGER.read_text().splitlines() if not line.startswith("#")][1] + "\n", "appears twice")]:
                self.assertTrue(any(needle in problem for problem in self.check(temporary, ",".join(CONVERGENCE + [PLANT]), text)), needle)

    def test_a_mark_in_a_form_that_the_check_cannot_read_is_red(self):
        with tempfile.TemporaryDirectory() as temporary:
            workflow = Path(temporary) / "ci.yml"
            workflow.write_text("jobs:\n  browser:\n    env:\n      M2_LOCAL_ONLY_CASES: ${{ vars.CASES }}\n")
            problems = local_only.check(workflow=workflow)
            self.assertEqual(len(problems), 1)
            self.assertIn("cannot read", problems[0])


class MarkPlacementTests(unittest.TestCase):
    """Review follow-up M1: exactly one non-comment occurrence, canonical form, in the env of the browser job."""
    CASES = ",".join(CONVERGENCE + [PLANT])

    def problems(self, text):
        with tempfile.TemporaryDirectory() as temporary:
            workflow = Path(temporary) / "ci.yml"
            workflow.write_text(text)
            return local_only.check(workflow=workflow)

    def assert_one_red(self, text, needle):
        problems = self.problems(text)
        self.assertTrue(any(needle in problem for problem in problems), f"{needle!r} not in {problems}")

    def test_the_canonical_text_is_green(self):
        self.assertEqual(self.problems(workflow_with(self.CASES)), [])

    def test_a_single_quoted_extra_mark_is_red(self):
        self.assert_one_red(workflow_with(self.CASES, "      M2_LOCAL_ONLY_CASES: 'E9-browser-base-economy'\n"), "exactly one")

    def test_an_unquoted_extra_mark_is_red(self):
        self.assert_one_red(workflow_with(self.CASES, "      M2_LOCAL_ONLY_CASES: E9-browser-base-economy\n"), "exactly one")

    def test_an_inline_prefix_on_a_command_line_is_red(self):
        extra = "  check:\n    steps:\n      - run: M2_LOCAL_ONLY_CASES=E9-browser-base-economy bash scripts/m2-browser.sh\n"
        self.assert_one_red(workflow_with(self.CASES, extra), "exactly one")

    def test_the_mark_in_the_check_job_is_red(self):
        text = ('jobs:\n  browser:\n    env:\n      CARGO_BUILD_JOBS: 2\n  check:\n    env:\n'
                f'      M2_LOCAL_ONLY_CASES: "{self.CASES}"\n')
        self.assert_one_red(text, "browser job")

    def test_no_mark_and_a_mark_outside_the_env_block_are_red(self):
        self.assert_one_red("jobs:\n  browser:\n    env:\n      CARGO_BUILD_JOBS: 2\n", "exactly one")
        text = f'jobs:\n  browser:\n    env:\n      CARGO_BUILD_JOBS: 2\n    steps:\n      - uses: x\n        with:\n          M2_LOCAL_ONLY_CASES: "{self.CASES}"\n'
        self.assert_one_red(text, "env")

    def test_a_comment_or_a_trailing_comment_is_not_an_occurrence(self):
        extra = '      # M2_LOCAL_ONLY_CASES: "E9-browser-base-economy"\n      OTHER: 1 # M2_LOCAL_ONLY_CASES in a trailing comment\n'
        self.assertEqual(self.problems(workflow_with(self.CASES, extra)), [])


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
        result, summary, runs = self.run_case(CONVERGENCE[0], {"M2_LOCAL_ONLY_CASES": CONVERGENCE[0], "GITHUB_ACTIONS": "true"})
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
        result, summary, runs = self.run_case("E9-browser-base-selection", {"M2_LOCAL_ONLY_CASES": "E9-browser-base-selection", "GITHUB_ACTIONS": "true"})
        self.assertEqual(result.returncode, 2)
        self.assertIn("has no row", result.stderr)
        self.assertNotIn("M2_CASE_LOCAL_ONLY", result.stdout)

    def test_the_default_suite_lists_the_four_convergence_cases_and_runs_the_other_eleven(self):
        with tempfile.TemporaryDirectory() as temporary:
            result = suite.run_default_suite(Path(temporary), {"M2_LOCAL_ONLY_CASES": ",".join(CONVERGENCE + [PLANT]), "GITHUB_ACTIONS": "true"})
        started = re.findall(r"^M2_CASE_START (\S+)$", result.stdout, re.M)
        local = re.findall(r"^M2_CASE_LOCAL_ONLY (\S+):", result.stdout, re.M)
        self.assertEqual(sorted(local), sorted(CONVERGENCE))
        self.assertEqual(len(started), 11)
        self.assertFalse(set(started) & set(CONVERGENCE))

    def test_the_refutation_leg_of_a_local_only_case_prints_the_notice_and_runs_nothing(self):
        with tempfile.TemporaryDirectory() as temporary:
            evidence = Path(temporary) / "evidence"
            env = {**os.environ, "GITHUB_ACTIONS": "true", "M2_LOCAL_ONLY_CASES": ",".join(CONVERGENCE + [PLANT]), "TURTS_REFUTATION_EVIDENCE_DIR": str(evidence),
                   "TURTS_REFUTATION_BUILD_ROOT": str(Path(temporary) / "build")}
            result = subprocess.run(["bash", str(local_only.ROOT / "scripts" / "m2-refutations.sh"), "--case", PLANT], cwd=local_only.ROOT,
                                    env=env, capture_output=True, text=True, timeout=120)
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertIn(f"M2_CASE_LOCAL_ONLY {PLANT}: not run on this runner", result.stdout)
            self.assertFalse(evidence.exists(), "no refutation may start for a local-only case")
            result = subprocess.run(["bash", str(local_only.ROOT / "scripts" / "m2-refutations.sh"), "--case", "E9-browser-base-economy-nothing"],
                                    cwd=local_only.ROOT, env={**env, "M2_LOCAL_ONLY_CASES": ""}, capture_output=True, text=True, timeout=120)
            self.assertNotEqual(result.returncode, 0, "an id that is not local-only goes to refutations.py, which refuses an unknown id")


class DecideExactTests(unittest.TestCase):
    """Review follow-up L5: a case matches by its whole id, never by a prefix."""

    def test_a_prefix_or_an_extension_of_a_listed_id_runs(self):
        env = {"M2_LOCAL_ONLY_CASES": CONVERGENCE[0]}
        for other in ("E9-browser-base-public", "E9-browser-base", CONVERGENCE[0] + "-2", CONVERGENCE[0] + "x", "E9-browser-base-public-convergenc"):
            self.assertEqual(local_only.decide(other, env=env), 0, other)
        with tempfile.TemporaryDirectory() as temporary:
            self.assertEqual(local_only.decide(CONVERGENCE[0], env=env, summary_path=Path(temporary) / "s.md"), 10)

    def test_a_listed_prefix_does_not_capture_a_longer_case(self):
        self.assertEqual(local_only.decide("E9-browser-base-economy", env={"M2_LOCAL_ONLY_CASES": "E9-browser-base"}), 0)

    def test_count_matches_whole_ids_only(self):
        environ = dict(os.environ, M2_LOCAL_ONLY_CASES=CONVERGENCE[0])
        result = subprocess.run([sys.executable, "-I", "-B", str(HERE / "local_only.py"), "--count"], input=f"{CONVERGENCE[0]}-2\nE9-browser-base-public\n{CONVERGENCE[0]}\n",
                                capture_output=True, text=True, env=environ)
        self.assertEqual(result.stdout.strip(), "1")


class LocalGuardTests(unittest.TestCase):
    """Review follow-up L6: no absolute path in the ledger; the local gate refuses the hosted-only values."""

    def test_the_real_ledger_has_no_absolute_path_and_names_the_wrapper_by_file_name(self):
        text = local_only.LEDGER.read_text()
        self.assertNotIn("/home/", text)
        rows = local_only.read_ledger()
        self.assertTrue(all("mc_headed_v6.sh" in row["local_command"] for row in rows.values()))
        self.assertIn("gate-runner", text)

    def test_an_absolute_path_in_any_field_is_red(self):
        for bad in ("bash /home/someone/target/mc-evidence/gate-runner/mc_headed_v6.sh --case E9-browser-base-public-convergence",
                    "bash ~/x/mc_headed_v6.sh --case E9-browser-base-public-convergence",
                    "bash 'C:\\tools\\mc_headed_v6.sh' --case E9-browser-base-public-convergence"):
            with tempfile.TemporaryDirectory() as temporary:
                ledger = Path(temporary) / "local-only.tsv"
                ledger.write_text("\t".join(local_only.COLUMNS) + "\n" + "\t".join(["E9-browser-base-public-convergence", "reason", bad, "receipt", "qa-lead"]) + "\n")
                with self.assertRaisesRegex(local_only.LedgerError, "absolute path"):
                    local_only.read_ledger(ledger)

    def test_a_relative_or_placeholder_path_is_fine(self):
        with tempfile.TemporaryDirectory() as temporary:
            ledger = Path(temporary) / "local-only.tsv"
            ledger.write_text("\t".join(local_only.COLUMNS) + "\n" + "\t".join(["E9-browser-base-public-convergence", "reason", "bash <gate-runner>/mc_headed_v6.sh -- bash scripts/m2-browser.sh --case E9-browser-base-public-convergence", "target/mc-evidence/<label>/receipt.json", "qa-lead"]) + "\n")
            self.assertEqual(len(local_only.read_ledger(ledger)), 1)

    def run_script(self, script, args, env_extra):
        """m2-browser.sh runs with the prebuilt fixture of test_m2_browser_suite, so that no run can start a build."""
        if script == "m2-browser.sh":
            with tempfile.TemporaryDirectory() as temporary:
                return suite.run_default_suite(Path(temporary), env_extra, tuple(args))
        env = {k: v for k, v in os.environ.items() if k not in ("GITHUB_ACTIONS", "M2_LOCAL_ONLY_CASES", "M2_RECIPE_DURATION_SCALE", "M2_RECIPE_DURATION_SCALE_CASES")}
        env.update(env_extra)
        return subprocess.run(["bash", str(ROOT / "scripts" / script), *args], cwd=ROOT, env=env, capture_output=True, text=True, timeout=120)

    def test_a_local_run_with_a_hosted_only_value_is_an_error_in_both_scripts(self):
        for script, args in (("m2-browser.sh", ["--case", "E9-browser-base-selection"]), ("m2-refutations.sh", ["--case", PLANT]), ("m2-browser.sh", [])):
            for name, value in (("M2_LOCAL_ONLY_CASES", PLANT), ("M2_RECIPE_DURATION_SCALE", "4"), ("M2_RECIPE_DURATION_SCALE_CASES", "E9-browser-base-loss-refund=5")):
                with self.subTest(script=script, name=name, args=args):
                    # The refutation script always gets the local-only list too, so that no old script can start a refutation build.
                    extra = {name: value} if script == "m2-browser.sh" else {name: value, "M2_LOCAL_ONLY_CASES": PLANT}
                    result = self.run_script(script, args, extra)
                    self.assertEqual(result.returncode, 2, result.stdout + result.stderr)
                    self.assertIn("is set and GITHUB_ACTIONS is not 'true'", result.stderr)
                    self.assertIn("M2_RECIPE_DURATION_SCALE_CASES", result.stderr)
                    self.assertIn("Unset them for a local gate run", result.stderr)
                    self.assertNotIn("M2_CASE_", result.stdout)

    def test_the_hosted_marker_allows_the_values_and_unset_values_are_fine_locally(self):
        result = self.run_script("m2-refutations.sh", ["--case", PLANT], {"GITHUB_ACTIONS": "true", "M2_LOCAL_ONLY_CASES": PLANT})
        self.assertEqual(result.returncode, 0, result.stderr)
        result = self.run_script("m2-browser.sh", ["--list-cases"], {"M2_LOCAL_ONLY_CASES": PLANT})
        self.assertEqual(result.returncode, 0, "--list-cases never starts a run")
        self.assertEqual(len(result.stdout.split()), 15)
        result = self.run_script("m2-browser.sh", ["--case", "E9-browser-base-selection", "--bad"], {})
        self.assertEqual(result.returncode, 2)
        self.assertNotIn("GITHUB_ACTIONS", result.stderr)


class RefutationLegOutputTests(unittest.TestCase):
    """Review follow-up M3: the local-only leg ends with an annotation and a skipped count."""

    def run_leg(self, ids, marks):
        with tempfile.TemporaryDirectory() as temporary:
            env = {**os.environ, "GITHUB_ACTIONS": "true", "M2_LOCAL_ONLY_CASES": marks, "TURTS_REFUTATION_EVIDENCE_DIR": str(Path(temporary) / "evidence"),
                   "TURTS_REFUTATION_BUILD_ROOT": str(Path(temporary) / "build")}
            return subprocess.run(["bash", str(ROOT / "scripts" / "m2-refutations.sh"), "--case", ids], cwd=ROOT, env=env, capture_output=True, text=True, timeout=120)

    def test_the_local_only_leg_prints_the_warning_annotation_and_the_count_as_its_last_lines(self):
        result = self.run_leg(PLANT, ",".join(CONVERGENCE + [PLANT]))
        self.assertEqual(result.returncode, 0, result.stderr)
        lines = result.stdout.splitlines()
        self.assertRegex(lines[-2], rf"^::warning title=M2 local-only::1 case\(s\) were not run on this runner and need the local GPU gate: {PLANT}$")
        self.assertEqual(lines[-1], "m2-refutations: skipped_local_only_count=1 ran_count=0")

    def test_a_leg_with_nothing_local_only_prints_a_zero_count_and_keeps_the_exit_status(self):
        result = self.run_leg("E9-browser-no-such-case", "")
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(result.stdout.splitlines()[-1], "m2-refutations: skipped_local_only_count=0 ran_count=1")
        self.assertNotIn("::warning", result.stdout)


class SuiteSummaryWhenRedTests(unittest.TestCase):
    """Review follow-up M4: the suite summary carries local_only_count also when a case is red."""

    def test_the_summary_counts_the_local_only_cases_and_the_red_ones(self):
        with tempfile.TemporaryDirectory() as temporary:
            result = suite.run_default_suite(Path(temporary), {"M2_LOCAL_ONLY_CASES": ",".join(CONVERGENCE + [PLANT]), "GITHUB_ACTIONS": "true"})
        self.assertNotEqual(result.returncode, 0)
        summary = json.loads(re.findall(r"^M2_DEFAULT_SUITE_SUMMARY (\{.*\})$", result.stdout, re.M)[0])
        self.assertEqual((summary["selected_count"], summary["completed_count"], summary["passed_count"], summary["infrastructure_failed_count"],
                          summary["local_only_count"]), (15, 11, 0, 11, 4))


class ReceiptTests(unittest.TestCase):
    """Review follow-up M2: local receipts are fresh while the inputs of the cases are unchanged."""
    CASES = CONVERGENCE + [PLANT]

    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.root = Path(self.temporary.name) / "repo"
        self.root.mkdir()
        self.run_git("init", "-q")
        self.run_git("config", "user.email", "qa@example.invalid")
        self.run_git("config", "user.name", "qa")
        for rel in ("tests/m2/a.py", "tests/browser/b.js", "scripts/m2-browser.sh", "scripts/other.sh", "mods/base/x.ron", "docs/a.md",
                    "crates/turts_client/src/lib.rs", "crates/turts_checks/src/lib.rs", "crates/turts_server/src/main.rs"):
            self.write(rel, "one\n")
        self.write("tests/m2/local-receipts.tsv", "x\n")
        self.first = self.commit("first")
        self.ledger = Path(self.temporary.name) / "local-only.tsv"
        self.ledger.write_text(local_only.LEDGER.read_text())
        self.evidence = Path(self.temporary.name) / "evidence"
        self.evidence.mkdir()
        self.receipts = Path(self.temporary.name) / "local-receipts.tsv"

    def tearDown(self):
        self.temporary.cleanup()

    def run_git(self, *args):
        return subprocess.run(["git", "-C", str(self.root), *args], capture_output=True, text=True, check=True).stdout.strip()

    def write(self, rel, text):
        path = self.root / rel
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text)

    def commit(self, message):
        self.run_git("add", "-A")
        self.run_git("commit", "-q", "-m", message)
        return self.run_git("rev-parse", "HEAD")

    def make_evidence(self, label, case, head, plant, command_exit=0, lines=None):
        folder = self.evidence / label
        folder.mkdir(exist_ok=True)
        (folder / "receipt.json").write_text(json.dumps({"expected_head": head, "command_exit": command_exit}) + "\n")
        wanted = lines if lines is not None else ([f"{case}: RED assertion='x'", f"{case}: RESTORED GREEN source_sha256=1"] if plant else [f"M2_CASE_PASS {case}"])
        (folder / "command.log").write_text("\n".join(wanted) + "\nexit=0\n")
        return hashlib.sha256((folder / "receipt.json").read_bytes()).hexdigest()

    def write_receipts(self, head, only=None, result=None, with_evidence=True, label_prefix="lab", sha=None):
        rows = ["\t".join(local_only.RECEIPT_COLUMNS)]
        for index, case in enumerate(self.CASES):
            if only is not None and case not in only:
                continue
            plant = case == PLANT
            label = f"{label_prefix}-{index}"
            digest = self.make_evidence(label, case, head, plant) if with_evidence else (sha or "0" * 64)
            rows.append("\t".join([case, head, label, result or (local_only.PLANT_RESULT if plant else "pass"), "2026-11-12", digest]))
        self.receipts.write_text("# comment\n" + "\n".join(rows) + "\n")

    def check(self, head=None):
        return local_only.check_receipts(head or self.run_git("rev-parse", "HEAD"), root=self.root, ledger=self.ledger,
                                         receipts=self.receipts, evidence=self.evidence, plants={PLANT})

    def test_the_real_receipts_file_has_no_rows_and_the_check_is_red_for_every_ledger_row(self):
        problems, _ = local_only.check_receipts(local_only.git(local_only.ROOT, "rev-parse", "HEAD")[1])
        self.assertEqual(sorted(p.split()[2] for p in problems), sorted(self.CASES))
        self.assertTrue(all("has no receipt row" in p for p in problems))

    def test_fresh_receipts_with_good_evidence_are_green(self):
        self.write_receipts(self.first)
        self.assertEqual(self.check(), ([], []))

    def test_a_change_outside_the_input_list_keeps_the_receipts_fresh(self):
        self.write_receipts(self.first)
        self.write("docs/a.md", "two\n")
        self.write("scripts/other.sh", "two\n")
        self.write("crates/turts_checks/src/lib.rs", "two\n")
        self.write("tests/m2/local-receipts.tsv", "y\n")
        head = self.commit("docs only")
        self.assertEqual(self.check(head)[0], [])

    def test_a_change_of_any_input_makes_the_receipts_stale(self):
        self.write_receipts(self.first)
        for rel in ("tests/m2/a.py", "tests/browser/b.js", "scripts/m2-browser.sh", "mods/base/x.ron", "crates/turts_client/src/lib.rs", "crates/turts_server/src/main.rs"):
            with self.subTest(rel=rel):
                self.write(rel, "changed\n")
                head = self.commit(f"touch {rel}")
                problems = self.check(head)[0]
                self.assertEqual(len(problems), len(self.CASES), problems)
                self.assertTrue(all("is stale" in p and rel in p for p in problems), problems)
                self.run_git("reset", "-q", "--hard", self.first)

    def test_a_ledger_row_without_a_receipt_row_is_red(self):
        self.write_receipts(self.first, only=set(self.CASES[:3]))
        problems = self.check()[0]
        self.assertEqual(len(problems), 2)
        self.assertTrue(all("has no receipt row" in p for p in problems))

    def test_a_result_that_is_not_a_pass_is_red_and_the_plant_needs_its_own_result(self):
        self.write_receipts(self.first, result="fail")
        self.assertEqual(len(self.check()[0]), len(self.CASES))
        self.write_receipts(self.first)
        text = self.receipts.read_text().replace(local_only.PLANT_RESULT, "pass")
        self.receipts.write_text(text)
        problems = self.check()[0]
        self.assertEqual(len(problems), 1)
        self.assertIn(local_only.PLANT_RESULT, problems[0])

    def test_the_latest_row_of_a_case_counts(self):
        self.write_receipts(self.first)
        lines = self.receipts.read_text().splitlines()
        self.receipts.write_text("\n".join(lines[:2] + [lines[2].replace("pass", "fail")] + lines[2:]) + "\n")
        self.assertEqual(self.check()[0], [])

    def test_evidence_that_exists_is_recomputed(self):
        self.write_receipts(self.first)
        (self.evidence / "lab-0" / "receipt.json").write_text(json.dumps({"expected_head": self.first, "command_exit": 0, "x": 1}) + "\n")
        self.assertTrue(any("receipt_sha256 does not match" in p for p in self.check()[0]))
        self.write_receipts(self.first, label_prefix="two")
        (self.evidence / "two-1" / "command.log").write_text("no pass line here\n")
        self.assertTrue(any("has no line that matches" in p and "two-1" in p for p in self.check()[0]))
        for folder in list(self.evidence.iterdir()):
            for item in folder.iterdir():
                item.unlink()
            folder.rmdir()
        self.write_receipts(self.first, label_prefix="three")
        (self.evidence / "three-4" / "command.log").write_text(f"{PLANT}: RED assertion='x'\n")
        self.assertTrue(any("RESTORED GREEN" in p for p in self.check()[0]))
        self.write_receipts(self.first, label_prefix="four")
        (self.evidence / "four-0" / "receipt.json").write_text(json.dumps({"expected_head": "0" * 40, "command_exit": 1}) + "\n")
        self.assertTrue(any("was made at head" in p for p in self.check()[0]))

    def test_a_missing_evidence_folder_is_a_note_not_a_failure(self):
        self.write_receipts(self.first, with_evidence=False)
        problems, notes = self.check()
        self.assertEqual(problems, [])
        self.assertEqual(len(notes), len(self.CASES))

    def test_head_must_be_the_checked_out_commit_and_a_commit(self):
        self.write_receipts(self.first)
        self.write("tests/m2/a.py", "z\n")
        self.commit("second")
        self.assertIn("is not the checked-out HEAD", self.check(self.first)[0][0])
        self.assertIn("is not a commit", self.check("0" * 40)[0][0])

    def test_a_receipt_row_with_a_head_that_is_not_in_this_repository_is_red(self):
        self.write_receipts(self.first, with_evidence=False)
        self.receipts.write_text(self.receipts.read_text().replace(self.first, "1" * 40))
        self.assertTrue(all("is not a commit of this repository" in p for p in self.check()[0]))

    def test_bad_receipt_rows_are_refused(self):
        for bad in ("E9-browser-base-public-convergence\tabc\tl\tpass\t2026-11-12\t" + "0" * 64,
                    "E9-browser-base-public-convergence\t" + "0" * 40 + "\tl\tpass\t12-11-2026\t" + "0" * 64,
                    "E9-browser-base-public-convergence\t" + "0" * 40 + "\tl\tpass\t2026-11-12"):
            self.receipts.write_text("\t".join(local_only.RECEIPT_COLUMNS) + "\n" + bad + "\n")
            with self.assertRaises(local_only.LedgerError):
                local_only.read_receipts(self.receipts)
            self.assertEqual(len(self.check()[0]), 1)

    def test_the_cli_is_red_on_the_real_repository_and_needs_head(self):
        result = subprocess.run([sys.executable, "-I", "-B", str(HERE / "local_only.py"), "--receipts", "--head", local_only.git(local_only.ROOT, "rev-parse", "HEAD")[1]],
                                capture_output=True, text=True)
        self.assertEqual(result.returncode, 1)
        self.assertIn("has no receipt row", result.stderr)
        result = subprocess.run([sys.executable, "-I", "-B", str(HERE / "local_only.py"), "--receipts"], capture_output=True, text=True)
        self.assertEqual(result.returncode, 2)

    def test_hosted_ci_does_not_run_the_receipts_check(self):
        workflow = (ROOT / ".github" / "workflows" / "ci.yml").read_text()
        self.assertNotIn("--receipts", workflow)
        self.assertIn("never", local_only.RECEIPTS.read_text().lower())
        self.assertIn("Hosted CI never runs it", local_only.LEDGER.read_text())


if __name__ == "__main__":
    unittest.main()
