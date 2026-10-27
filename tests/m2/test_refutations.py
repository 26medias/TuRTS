#!/usr/bin/env python3
"""Focused tests for the M2 semantic refutation runner."""
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest

MODULE_PATH = Path(__file__).with_name("refutations.py")
spec = importlib.util.spec_from_file_location("m2_refutations", MODULE_PATH)
refutations = importlib.util.module_from_spec(spec)
assert spec.loader is not None
spec.loader.exec_module(refutations)


class RefutationRunnerTests(unittest.TestCase):
    def test_selection_rejects_unknown_and_empty_requested_cases(self):
        cases = [{"id": "E7-one"}, {"id": "E8-one"}]
        with self.assertRaisesRegex(ValueError, "unknown refutation case"):
            refutations.select_cases(cases, "E7-one,missing")
        with self.assertRaisesRegex(ValueError, "no refutation cases selected"):
            refutations.select_cases(cases, "product")

    def test_selection_preserves_registry_order_and_guards_expected_prefix(self):
        cases = [
            {"id": "E7-first", "kind": "product"},
            {"id": "E8-second", "kind": "product"},
            {"id": "E9-third", "kind": "process"},
        ]
        selected = refutations.select_cases(cases, "E8-second,E7-first")
        self.assertEqual([case["id"] for case in selected], ["E7-first", "E8-second"])
        only_e7 = refutations.select_cases(cases, "E7-first")
        self.assertEqual([case["id"] for case in refutations.require_case_prefix(only_e7, "E7")], [
            "E7-first"
        ])
        with self.assertRaisesRegex(ValueError, "expected case prefix E7"):
            refutations.require_case_prefix(selected, "E7")

    def test_run_workspaces_are_unique_and_preserve_previous_runs(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            first = refutations.create_run_workspace(root)
            sentinel = first / "keep.txt"
            sentinel.write_text("prior evidence")
            second = refutations.create_run_workspace(root)
            self.assertNotEqual(first, second)
            self.assertTrue(sentinel.exists())
            self.assertTrue((first / "scratch").is_dir())
            self.assertTrue((second / "build").is_dir())

    def test_browser_markers_require_exact_complete_semantic_phases(self):
        case = {"id": "E16-browser-case", "runtime": "browser", "expected_failure": "private balance"}
        baseline = "\n".join([
            "M2_CASE_START E16-browser-case",
            "M2_CASE_PASS E16-browser-case",
            'M2_CASE_SUMMARY {"selected_count":1,"completed_count":1,"passed_count":1,"semantic_failed_count":0,"infrastructure_failed_count":0,"timeout_count":0}',
        ])
        self.assertEqual(refutations.require_phase_result(case, "baseline.green", type("R", (), {"stdout":baseline,"returncode":0})()), None)
        red = "\n".join([
            "M2_CASE_START E16-browser-case",
            'M2_CASE_SEMANTIC_FAIL E16-browser-case "private balance leaked"',
            'M2_CASE_SUMMARY {"selected_count":1,"completed_count":1,"passed_count":0,"semantic_failed_count":1,"infrastructure_failed_count":0,"timeout_count":0}',
        ])
        self.assertEqual(refutations.require_phase_result(case, "plant.red", type("R", (), {"stdout":red,"returncode":1})()), "private balance leaked")
        for invalid in (
            baseline.replace("M2_CASE_SUMMARY", "M2_CASE_TIMEOUT E16-browser-case \"late\"\nM2_CASE_SUMMARY"),
            baseline.replace('"completed_count":1', '"completed_count":0'),
            baseline.replace(',"timeout_count":0', ''),
            baseline.replace('"infrastructure_failed_count":0', '"infrastructure_failed_count":1'),
            baseline.replace("M2_CASE_PASS", "M2_CASE_SEMANTIC_FAIL"),
        ):
            with self.subTest(invalid=invalid), self.assertRaises(ValueError):
                refutations.browser_summary(invalid, case)


    def test_browser_registry_anchors_are_unique_and_install_playwright_in_scratch(self):
        registry = json.loads((refutations.ROOT / "tests/m2/refutations.json").read_text())
        browser_cases = [case for case in registry if case.get("runtime") == "browser"]
        self.assertEqual(
            [case["id"] for case in browser_cases],
            [
                "E2-browser-hardcoded-label",
                "E9-browser-missing-button",
                "E9-browser-debug-only-ui",
                "E9-browser-missing-output-mesh",
                "E9-browser-position-only-snapshot",
                "E9-browser-unbounded-buffer",
                "E9-browser-duplicate-outstanding",
                "E9-browser-ignore-midtrace-rearm",
            ],
        )
        for case in browser_cases:
            self.assertEqual(case["expected_selected"], len(case["browser_case_ids"]))
            self.assertEqual(len(case["browser_case_ids"]), 1)
            self.assertIn(["npm", "ci", "--offline", "--prefix", "tests/browser"], case["pre_commands"])
            path = refutations.ROOT / case["plant_path"]
            source = path.read_text()
            replacements = case.get("replacements") or [{"anchor": case["anchor"], "replacement": case["replacement"]}]
            for item in replacements:
                self.assertEqual(source.count(item["anchor"]), 1, case["id"])
                source = source.replace(item["anchor"], item["replacement"], 1)

    def test_runtime_selectors_are_explicit_and_nonempty(self):
        cases = [
            {"id": "E8-native", "kind": "process", "runtime": "rust"},
            {"id": "E16-browser", "kind": "process", "runtime": "browser"},
            {"id": "E14-legacy", "kind": "process"},
        ]
        self.assertEqual(
            [case["id"] for case in refutations.select_cases(cases, "ci-native")],
            ["E8-native", "E14-legacy"],
        )
        self.assertEqual(
            [case["id"] for case in refutations.select_cases(cases, "ci-browser")],
            ["E16-browser"],
        )
        with self.assertRaisesRegex(ValueError, "no refutation cases selected"):
            refutations.select_cases([cases[0]], "ci-browser")

    def test_source_copy_skips_node_modules_and_dist(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp) / "root"
            root.mkdir()
            (root / "src").mkdir()
            (root / "src" / "main.rs").write_text("fn main() {}")
            (root / "node_modules").mkdir()
            (root / "node_modules" / "large.bin").write_bytes(b"x")
            (root / "dist").mkdir()
            (root / "dist" / "bundle.js").write_text("bundle")
            destination = Path(temp) / "copy"
            refutations.copy_source_tree(root, destination)
            self.assertTrue((destination / "src" / "main.rs").exists())
            self.assertFalse((destination / "node_modules").exists())
            self.assertFalse((destination / "dist").exists())

    def test_multiple_precommands_keep_legacy_single_command_support(self):
        self.assertEqual(
            refutations.pre_commands({"pre_commands": [["cargo", "build"], ["cargo", "test"]]}),
            [["cargo", "build"], ["cargo", "test"]],
        )
        self.assertEqual(
            refutations.pre_commands({"pre_command": ["cargo", "build"]}),
            [["cargo", "build"]],
        )

    def test_multi_anchor_plant_is_atomic_and_legacy_case_still_works(self):
        case = {
            "id": "E8-two-anchor",
            "replacements": [
                {"anchor": "first", "replacement": "FIRST"},
                {"anchor": "second", "replacement": "SECOND"},
            ],
        }
        self.assertEqual(refutations.apply_plant("first + second", case), "FIRST + SECOND")
        with self.assertRaisesRegex(RuntimeError, "anchor occurs 0 times"):
            refutations.apply_plant("first + absent", case)
        self.assertEqual(
            refutations.apply_plant("old", {"id": "E7-one", "anchor": "old", "replacement": "new"}),
            "new",
        )

    def test_selected_test_count_is_exact_and_nonzero(self):
        self.assertEqual(refutations.require_selected_count("running 1 test\n", 1), 1)
        for output in ("running 0 tests\n", "running 2 tests\n"):
            with self.subTest(output=output), self.assertRaisesRegex(ValueError, "expected exactly 1"):
                refutations.require_selected_count(output, 1)

    def test_real_subprocess_mutation_restores_source_and_keeps_all_phase_logs(self):
        import sys

        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            scratch = root / "scratch"
            scratch.mkdir()
            source = scratch / "sample.rs"
            source.write_text("let gate = true;\n")
            build = root / "build"
            build.mkdir()
            evidence = root / "evidence"
            script = (
                "from pathlib import Path; import sys; "
                "print('running 1 test'); "
                "broken = 'false' in Path('sample.rs').read_text(); "
                "print('semantic invariant violated' if broken else 'test result: ok'); "
                "sys.exit(1 if broken else 0)"
            )
            case = {
                "id": "E7-live",
                "plant_path": "sample.rs",
                "anchor": "let gate = true;",
                "replacement": "let gate = false;",
                "command": [sys.executable, "-c", script],
                "expected_failure": "semantic invariant violated",
            }
            refutations.run_case(case, scratch, build, evidence)
            self.assertEqual(source.read_text(), "let gate = true;\n")
            self.assertEqual(
                {path.name for path in evidence.iterdir()},
                {
                    "baseline.green.log",
                    "plant.red.log",
                    "restored.green.log",
                    "case.json",
                },
            )
            manifest = refutations.json.loads((evidence / "case.json").read_text())
            self.assertEqual(manifest["status"], "passed")
            self.assertEqual(manifest["baseline_sha256"], manifest["restored_sha256"])
            self.assertNotEqual(manifest["baseline_sha256"], manifest["planted_sha256"])

    def test_failed_plant_restores_shared_source_before_next_case(self):
        import sys

        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            scratch = root / "scratch"
            scratch.mkdir()
            source = scratch / "sample.rs"
            source.write_text("let gate = true;\n")
            build = root / "build"
            build.mkdir()
            failed_evidence = root / "failed"
            failed_script = (
                "from pathlib import Path; import sys; print('running 1 test'); "
                "broken = 'false' in Path('sample.rs').read_text(); "
                "print('error[E0001]: simulated compile failure' if broken else 'test result: ok'); "
                "sys.exit(1 if broken else 0)"
            )
            failed = {
                "id": "E8-failing-plant",
                "plant_path": "sample.rs",
                "anchor": "let gate = true;",
                "replacement": "let gate = false;",
                "command": [sys.executable, "-c", failed_script],
                "expected_failure": "unreached",
            }
            with self.assertRaisesRegex(RuntimeError, "plant caused a compile failure"):
                refutations.run_case(failed, scratch, build, failed_evidence)
            self.assertEqual(source.read_text(), "let gate = true;\n")

            next_evidence = root / "next"
            next_script = (
                "from pathlib import Path; import sys; print('running 1 test'); "
                "broken = 'false' in Path('sample.rs').read_text(); "
                "print('semantic invariant violated' if broken else 'test result: ok'); "
                "sys.exit(1 if broken else 0)"
            )
            next_case = {
                "id": "E9-after-failure",
                "plant_path": "sample.rs",
                "anchor": "let gate = true;",
                "replacement": "let gate = false;",
                "command": [sys.executable, "-c", next_script],
                "expected_failure": "semantic invariant violated",
            }
            refutations.run_case(next_case, scratch, build, next_evidence)
            self.assertEqual(source.read_text(), "let gate = true;\n")

    def test_case_evidence_writes_full_phase_logs_and_hashes(self):
        with tempfile.TemporaryDirectory() as temp:
            evidence = Path(temp)
            refutations.write_case_evidence(
                evidence,
                "E7-one",
                "a" * 64,
                "b" * 64,
                {
                    "baseline.green.log": "baseline output",
                    "plant.red.log": "semantic assertion",
                    "restored.green.log": "green",
                },
            )
            self.assertEqual((evidence / "baseline.green.log").read_text(), "baseline output")
            self.assertEqual((evidence / "plant.red.log").read_text(), "semantic assertion")
            manifest = refutations.json.loads((evidence / "case.json").read_text())
            self.assertEqual(manifest["baseline_sha256"], "a" * 64)
            self.assertEqual(manifest["restored_sha256"], "b" * 64)
            self.assertEqual(manifest["id"], "E7-one")


if __name__ == "__main__":
    unittest.main()
