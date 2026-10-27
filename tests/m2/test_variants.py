#!/usr/bin/env python3
"""Focused acceptance coverage for the typed rename-only package variant tool."""
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
TARGET = Path("/tmp/turts-m2-variants-target")
SCRIPT = ROOT / "tests/m2/variants.py"

class VariantCliTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="turts-m2-variants-")
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.source = self.root / "source"
        self.out = self.root / "out"
        shutil.copytree(ROOT / "mods/cedar", self.source)
        (self.source / "assets").mkdir()
        (self.source / "assets/keep.bin").write_bytes(b"unchanged authored asset\x00\xff")
        self.env = os.environ.copy()
        self.env["CARGO_TARGET_DIR"] = str(TARGET)

    def copy_package(self, package, destination=None):
        destination = destination or self.source
        if destination.exists():
            shutil.rmtree(destination)
        shutil.copytree(ROOT / "mods" / package, destination)
        return destination

    def invoke(self, source=None, out=None, seed=123, mode="rename", payment="reserve", cwd=ROOT):
        return subprocess.run(
            ["python3", str(SCRIPT), "--source", str(source or self.source),
             "--out", str(out or self.out), "--seed", str(seed), "--mode", mode,
             "--payment", payment],
            cwd=cwd, env=self.env, text=True, capture_output=True, check=False,
        )

    def test_physical_cedar_rewrites_typed_package_and_records_complete_mapping(self):
        result = self.invoke()
        self.assertEqual(result.returncode, 0, result.stderr)
        mapping = json.loads((self.out / "mapping.json").read_text())
        renamed = mapping["packages"]["cedar"]["new"]
        self.assertTrue(renamed.startswith("p") and renamed != "cedar")
        self.assertTrue((self.out / renamed / "manifest.ron").is_file())
        self.assertEqual(mapping["version"], 1)
        self.assertEqual((mapping["seed"], mapping["mode"], mapping["payment"]), (123, "rename", "reserve"))
        self.assertGreaterEqual(len(mapping["definitions"]), 6)
        self.assertTrue(all(row["new_local_id"] != row["old_local_id"] for row in mapping["definitions"]))
        names = {(row["definition_path"], row["field"]) for row in mapping["names"]}
        self.assertIn(("resources/wood.ron", "name"), names)
        self.assertIn(("resources/wood.ron", "unit_name"), names)
        paths = mapping["paths"]
        self.assertTrue(all(row["old_export_path"] != row["new_export_path"] for row in paths))
        self.assertTrue(all(row["old_package_path"] and row["new_package_path"] and row["old_archive_path"] and row["new_archive_path"] for row in paths))
        asset = mapping["unchanged_assets"][0]
        self.assertTrue(asset["unchanged"])
        self.assertEqual(asset["old_path"], asset["new_path"])
        self.assertEqual((self.out / renamed / asset["new_path"]).read_bytes(), b"unchanged authored asset\x00\xff")

    def test_same_seed_has_identical_bytes_independent_of_output_path(self):
        first = self.invoke()
        self.assertEqual(first.returncode, 0, first.stderr)
        second_out = self.root / "other"
        second = self.invoke(out=second_out)
        self.assertEqual(second.returncode, 0, second.stderr)
        self.assertEqual(self.snapshot(self.out), self.snapshot(second_out))

    def test_same_seed_is_independent_of_filesystem_creation_order(self):
        first = self.invoke()
        self.assertEqual(first.returncode, 0, first.stderr)
        shuffled = self.root / "shuffled-source"
        shuffled.mkdir()
        entries = sorted((p for p in self.source.rglob("*") if p.is_file()), reverse=True)
        for source_file in entries:
            target = shuffled / source_file.relative_to(self.source)
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_bytes(source_file.read_bytes())
        other = self.root / "shuffled-out"
        result = self.invoke(source=shuffled, out=other)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(self.snapshot(self.out), self.snapshot(other))

    def test_different_seed_changes_generated_bytes(self):
        first = self.invoke()
        self.assertEqual(first.returncode, 0, first.stderr)
        second_out = self.root / "other"
        second = self.invoke(out=second_out, seed=124)
        self.assertEqual(second.returncode, 0, second.stderr)
        self.assertNotEqual((self.out / "mapping.json").read_bytes(), (second_out / "mapping.json").read_bytes())

    def test_real_base_transforms_all_content_and_public_visuals(self):
        self.copy_package("base")
        result = self.invoke(payment="charge")
        self.assertEqual(result.returncode, 0, result.stderr)
        mapping = json.loads((self.out / "mapping.json").read_text())
        self.assertIn("base", mapping["packages"])
        self.assertTrue(any(row["category"] == "maps" for row in mapping["definitions"]))
        self.assertTrue(any(row["category"] == "production" for row in mapping["definitions"]))

    def test_raw_base_without_the_reviewed_visual_repair_fails_closed(self):
        shutil.rmtree(self.source)
        shutil.copytree(ROOT / "mods/base", self.source)
        visual = self.source / "visuals/ore.ron"
        visual.unlink()
        manifest = self.source / "manifest.ron"
        manifest.write_text(manifest.read_text().replace(', "visuals/ore.ron"', ""))
        result = self.invoke(payment="charge")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("no visual", result.stderr)
        self.assertFalse(self.out.exists())

    def test_source_root_symlink_is_rejected(self):
        linked = self.root / "linked-source"
        linked.symlink_to(self.source, target_is_directory=True)
        result = self.invoke(source=linked)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("symlink", result.stderr.lower())
        self.assertFalse(self.out.exists())

    def test_relative_paths_are_resolved_from_wrapper_caller_directory(self):
        result = self.invoke(source=Path("source"), out=Path("relative-out"), cwd=self.root)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertTrue((self.root / "relative-out" / "mapping.json").is_file())
        self.assertFalse((ROOT / "relative-out").exists())

    def test_both_unsupported_modes_fail_before_writes(self):
        for mode in ("balance", "physical"):
            out = self.root / mode
            result = self.invoke(out=out, mode=mode)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("only rename", result.stderr)
            self.assertFalse(out.exists())

    def test_payment_mismatch_fails_without_writes(self):
        result = self.invoke(payment="charge")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("payment policy mismatches", result.stderr)
        self.assertFalse(self.out.exists())

    def test_existing_output_is_preserved(self):
        self.out.mkdir()
        marker = self.out / "keep.txt"
        marker.write_text("keep")
        result = self.invoke()
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(marker.read_text(), "keep")

    def test_overlapping_output_is_rejected(self):
        result = self.invoke(out=self.source / "nested")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("overlap", result.stderr)
        self.assertFalse((self.source / "nested").exists())

    def test_empty_and_malformed_sources_fail_without_output(self):
        empty = self.root / "empty"
        empty.mkdir()
        result = self.invoke(source=empty)
        self.assertNotEqual(result.returncode, 0)
        self.assertFalse(self.out.exists())
        malformed = self.root / "malformed"
        malformed.mkdir()
        (malformed / "manifest.ron").write_text("not RON")
        result = self.invoke(source=malformed, out=self.root / "bad-out")
        self.assertNotEqual(result.returncode, 0)
        self.assertFalse((self.root / "bad-out").exists())

    def test_native_and_symlink_inputs_fail_without_output(self):
        native = self.source / "hook.rs"
        native.write_text("fn hook() {}")
        result = self.invoke()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("native input", result.stderr)
        native.unlink()
        target = self.root / "outside.txt"
        target.write_text("asset")
        (self.source / "linked.txt").symlink_to(target)
        result = self.invoke()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("symlink", result.stderr)
        self.assertFalse(self.out.exists())

    def test_dependency_manifests_and_unexported_ron_fail(self):
        manifest = self.source / "manifest.ron"
        manifest.write_text(manifest.read_text().replace("dependencies: {},", 'dependencies: {"other": "^0.2"},'))
        result = self.invoke()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("dependencies", result.stderr)
        manifest.write_text(manifest.read_text().replace('dependencies: {"other": "^0.2"},', "dependencies: {},"))
        (self.source / "hidden.ron").write_text("()")
        result = self.invoke()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("unexported RON", result.stderr)
        self.assertFalse(self.out.exists())

    def test_dangling_references_fail_before_publication(self):
        node = self.source / "nodes/grove.ron"
        node.write_text(node.read_text().replace("cedar:wood", "cedar:missing"))
        result = self.invoke()
        self.assertNotEqual(result.returncode, 0)
        self.assertFalse(self.out.exists())

    def test_missing_exported_definition_fails_before_publication(self):
        (self.source / "nodes/grove.ron").unlink()
        result = self.invoke()
        self.assertNotEqual(result.returncode, 0)
        self.assertFalse(self.out.exists())

    def test_duplicate_package_identities_fail_before_publication(self):
        packages = self.root / "packages"
        packages.mkdir()
        shutil.copytree(ROOT / "mods/cedar", packages / "copy-a")
        shutil.copytree(ROOT / "mods/cedar", packages / "copy-b")
        result = self.invoke(source=packages)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("duplicate package identity", result.stderr)
        self.assertFalse(self.out.exists())

    def test_duplicate_definition_identity_fails_before_publication(self):
        worker = self.source / "units/worker.ron"
        worker.write_text(worker.read_text().replace('id: "worker"', 'id: "depot"'))
        result = self.invoke()
        self.assertNotEqual(result.returncode, 0)
        self.assertFalse(self.out.exists())

    @staticmethod
    def snapshot(root):
        return {p.relative_to(root).as_posix(): p.read_bytes() for p in sorted(root.rglob("*")) if p.is_file()}

if __name__ == "__main__":
    unittest.main(verbosity=2)
