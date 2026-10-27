#!/usr/bin/env python3
"""Focused tests for the M2 semantic refutation runner."""
import importlib.util
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
