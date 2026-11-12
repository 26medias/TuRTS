#!/usr/bin/env python3
"""Tests of the packed recipe time of the M2 economy browser cases (tests/m2/recipe_duration.py)."""
import hashlib
import importlib.util
import json
from pathlib import Path
import shutil
import tempfile
import unittest

MODULE_PATH = Path(__file__).with_name("recipe_duration.py")
SPEC = importlib.util.spec_from_file_location("m2_recipe_duration", MODULE_PATH)
recipe = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
SPEC.loader.exec_module(recipe)
ROOT = recipe.ROOT


def tree_hash(path):
    digest = hashlib.sha256()
    for item in sorted(path.rglob("*")):
        if item.is_file():
            digest.update(str(item.relative_to(path)).encode() + b"\0" + item.read_bytes())
    return digest.hexdigest()


class RecipeDurationTests(unittest.TestCase):
    def pack(self, temporary, package):
        packed = Path(temporary) / "pack" / package
        shutil.copytree(ROOT / "mods" / package, packed)
        return packed, Path(temporary) / "receipt.json"

    def test_true_recipe_times_of_mods_are_the_ones_the_hosted_runs_measured_against(self):
        for case_id, (package, relative) in recipe.ECONOMY_RECIPES.items():
            text = (ROOT / "mods" / package / relative).read_text()
            self.assertEqual(recipe.duration_ms(text, relative), 5000 if package == "base" else 6000, case_id)

    def test_scale_four_edits_the_packed_copy_only_and_writes_the_receipt(self):
        for case_id, (package, relative) in recipe.ECONOMY_RECIPES.items():
            with self.subTest(case=case_id), tempfile.TemporaryDirectory() as temporary:
                packed, receipt_path = self.pack(temporary, package)
                before = tree_hash(ROOT / "mods")
                source_ms = recipe.duration_ms((ROOT / "mods" / package / relative).read_text(), relative)
                receipt = recipe.apply(ROOT / "mods" / package, packed, package, case_id, "4", receipt_path)
                self.assertEqual(tree_hash(ROOT / "mods"), before, "mods/ must not change")
                self.assertEqual(recipe.duration_ms((packed / relative).read_text(), relative), source_ms * 4)
                self.assertEqual(json.loads(receipt_path.read_text()), receipt)
                self.assertEqual((receipt["scale"], receipt["applied"], receipt["packed_duration_ms"]), (4, True, source_ms * 4))
                # Only that one line of the one file differs.
                changed = [f for f in (ROOT / "mods" / package).rglob("*") if f.is_file() and
                           f.read_bytes() != (packed / f.relative_to(ROOT / "mods" / package)).read_bytes()]
                self.assertEqual([f.name for f in changed], [Path(relative).name])

    def test_scale_one_and_unset_keep_the_true_times_and_still_write_a_receipt(self):
        with tempfile.TemporaryDirectory() as temporary:
            packed, receipt_path = self.pack(temporary, "base")
            receipt = recipe.apply(ROOT / "mods" / "base", packed, "base", "E9-browser-base-economy", "1", receipt_path)
            self.assertEqual((receipt["scale"], receipt["applied"], receipt["packed_duration_ms"]), (1, False, 5000))
            self.assertEqual(tree_hash(packed), tree_hash(ROOT / "mods" / "base"))

    def test_the_other_cases_are_never_scaled(self):
        for case_id in ("E9-browser-base-recovery", "E9-browser-base-loss-spawn", "E9-browser-base-selection",
                        "E9-browser-base-public-convergence"):
            with self.subTest(case=case_id), tempfile.TemporaryDirectory() as temporary:
                packed, receipt_path = self.pack(temporary, "base")
                receipt = recipe.apply(ROOT / "mods" / "base", packed, "base", case_id, "4", receipt_path)
                self.assertEqual((receipt["scale"], receipt["applied"], receipt["scale_requested"]), (1, False, 4))
                self.assertEqual(tree_hash(packed), tree_hash(ROOT / "mods" / "base"))

    def test_bad_input_is_refused_and_never_a_silent_no_op(self):
        for raw in ("0", "-1", "1.5", "abc", "", "11", "04"):
            with self.subTest(scale=raw), tempfile.TemporaryDirectory() as temporary:
                packed, receipt_path = self.pack(temporary, "base")
                with self.assertRaises(ValueError):
                    recipe.apply(ROOT / "mods" / "base", packed, "base", "E9-browser-base-economy", raw, receipt_path)
                self.assertFalse(receipt_path.exists())
        with tempfile.TemporaryDirectory() as temporary:
            packed, receipt_path = self.pack(temporary, "base")
            (packed / "production" / "rifleman.ron").write_text("(\n    duration_ms: 5000, duration_ms: 5000,\n)\n")
            with self.assertRaisesRegex(ValueError, "differs from mods"):
                recipe.apply(ROOT / "mods" / "base", packed, "base", "E9-browser-base-economy", "4", receipt_path)
        with tempfile.TemporaryDirectory() as temporary:
            source = Path(temporary) / "source"
            packed = Path(temporary) / "packed"
            for folder in (source, packed):
                (folder / "production").mkdir(parents=True)
                (folder / "production" / "rifleman.ron").write_text("(\n    id: \"x\",\n)\n")
            with self.assertRaisesRegex(ValueError, "found 0"):
                recipe.apply(source, packed, "base", "E9-browser-base-economy", "4", Path(temporary) / "r.json")
        with tempfile.TemporaryDirectory() as temporary:
            packed, receipt_path = self.pack(temporary, "haul")
            with self.assertRaisesRegex(ValueError, "belongs to package base"):
                recipe.apply(ROOT / "mods" / "haul", packed, "haul", "E9-browser-base-economy", "4", receipt_path)


if __name__ == "__main__":
    unittest.main()
