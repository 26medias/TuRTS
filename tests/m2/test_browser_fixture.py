#!/usr/bin/env python3
"""Focused tests for the test-only packed M2 map translation."""
import importlib.util
from pathlib import Path
import tempfile
import unittest

MODULE_PATH = Path(__file__).with_name("browser_fixture.py")
SPEC = importlib.util.spec_from_file_location("m2_browser_fixture", MODULE_PATH)
fixture = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
SPEC.loader.exec_module(fixture)


class BrowserFixtureTranslationTests(unittest.TestCase):
    def test_all_three_scratch_maps_translate_every_position_and_keep_bounds(self):
        for package in ("base", "haul", "cedar"):
            with self.subTest(package=package), tempfile.TemporaryDirectory() as temporary:
                relative = fixture.MAPS[package][0]
                source_path = fixture.ROOT / "mods" / package / relative
                packed_path = Path(temporary) / "pack" / package / relative
                packed_path.parent.mkdir(parents=True)
                packed_path.write_bytes(source_path.read_bytes())
                receipt_path = Path(temporary) / "translation.json"
                receipt = fixture.translate(source_path, packed_path, receipt_path, package, "", 18)
                before = fixture.parse_map(source_path.read_text(), package)
                after = fixture.parse_map(packed_path.read_text(), package, translated=True)
                self.assertEqual(after["width"], before["width"] + 18)
                self.assertEqual(after["height"], before["height"])
                self.assertEqual(len(before["records"]), len(after["records"]))
                self.assertEqual(receipt["pairwise_squared_distance_sha256_before"], receipt["pairwise_squared_distance_sha256_after"])
                self.assertTrue(all(0 <= record["x"] <= after["width"] and 0 <= record["y"] <= after["height"] for record in after["records"]))

    def test_translation_is_rigid_bounded_and_preserves_case_overrides(self):
        source = (fixture.ROOT / "mods/base/maps/flat.ron").read_text()
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source_path = fixture.ROOT / "mods/base/maps/flat.ron"
            packed_path = root / "pack/base/maps/flat.ron"
            packed_path.parent.mkdir(parents=True)
            blocker_case = "E9-browser-base-loss-final_change"
            packed_path.write_text(fixture.add_expected_case_override(source, "base", blocker_case))
            receipt_path = root / "translation.json"
            receipt = fixture.translate(source_path, packed_path, receipt_path, "base", blocker_case, 18)
            translated = fixture.parse_map(packed_path.read_text(), "base", translated=True)
            self.assertEqual(translated["width"], 82)
            self.assertEqual(translated["height"], 64)
            self.assertEqual(receipt["placement_count"], 6)
            self.assertEqual(receipt["node_count"], 2)
            self.assertEqual(receipt["pairwise_squared_distance_sha256_before"], receipt["pairwise_squared_distance_sha256_after"])
            self.assertEqual(receipt["packed_after_translation_sha256"], fixture.sha256(packed_path.read_bytes()))
            self.assertTrue(receipt["non_coordinate_source_text_preserved"])
            self.assertEqual([row["x"] for row in translated["records"][:2]], [26, 30])
            self.assertEqual([row["x"] for row in translated["records"][-2:]], [42, 58])

    def test_old_world_x_lookup_fails_and_shifted_lookup_finds_same_base_subjects(self):
        source_path = fixture.ROOT / "mods/base/maps/flat.ron"
        with tempfile.TemporaryDirectory() as temporary:
            packed_path = Path(temporary) / "pack/base/maps/flat.ron"
            packed_path.parent.mkdir(parents=True)
            packed_path.write_bytes(source_path.read_bytes())
            receipt_path = Path(temporary) / "translation.json"
            fixture.translate(source_path, packed_path, receipt_path, "base", "", 18)
            before = fixture.parse_map(source_path.read_text(), "base")
            after = fixture.parse_map(packed_path.read_text(), "base", translated=True)

        def unit(rows, content_id, owner, x, y):
            return next((row for row in rows if row["kind"] == "unit" and row["id"] == content_id and row["owner_slot"] == owner and row["x"] == x and row["y"] == y), None)

        def node(rows, content_id, x, y):
            return next((row for row in rows if row["kind"] == "node" and row["id"] == content_id and row["x"] == x and row["y"] == y), None)

        self.assertIsNotNone(unit(before["records"], "base:command_center", 0, 8, 8))
        self.assertIsNone(unit(after["records"], "base:command_center", 0, 8, 8))
        self.assertEqual(unit(after["records"], "base:command_center", 0, 26, 8)["id"], "base:command_center")
        self.assertIsNotNone(unit(before["records"], "base:harvester", 0, 12, 8))
        self.assertIsNone(unit(after["records"], "base:harvester", 0, 12, 8))
        self.assertEqual(unit(after["records"], "base:harvester", 0, 30, 8)["id"], "base:harvester")
        self.assertIsNotNone(node(before["records"], "base:ore_deposit", 24, 8))
        self.assertIsNone(node(after["records"], "base:ore_deposit", 24, 8))
        self.assertEqual(node(after["records"], "base:ore_deposit", 42, 8)["id"], "base:ore_deposit")

    def test_unexpected_map_shape_and_wrong_offset_fail_closed(self):
        source_path = fixture.ROOT / "mods/cedar/maps/clearing.ron"
        with tempfile.TemporaryDirectory() as temporary:
            packed_path = Path(temporary) / "pack/cedar/maps/clearing.ron"
            packed_path.parent.mkdir(parents=True)
            packed_path.write_bytes(source_path.read_bytes())
            receipt = Path(temporary) / "translation.json"
            with self.assertRaisesRegex(ValueError, "explicit X offset 18"):
                fixture.translate(source_path, packed_path, receipt, "cedar", "", 17)
            with self.assertRaisesRegex(ValueError, "unexpected cedar dimensions"):
                fixture.parse_map(source_path.read_text().replace("x: 64.0", "x: 63.0", 1), "cedar")


if __name__ == "__main__":
    unittest.main()
