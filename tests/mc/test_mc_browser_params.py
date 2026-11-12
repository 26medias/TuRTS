import json
import pathlib
import sys
import unittest

sys.dont_write_bytecode = True
sys.path.insert(0, str(pathlib.Path(__file__).resolve().parents[2] / "scripts"))

from mc_browser_params import add_synthetic_param, synthetic_server_args, synthetic_units, validate_task16_plan


class McBrowserParamsTests(unittest.TestCase):
    def test_absent_field_keeps_params_bytes_and_old_server_args(self) -> None:
        before = {"package": "muster", "url": "http://127.0.0.1:8080/", "windows": 1}
        expected = json.dumps(before, sort_keys=True) + "\n"
        actual = json.dumps(add_synthetic_param(before.copy(), {}), sort_keys=True) + "\n"

        self.assertEqual(actual.encode(), expected.encode())
        self.assertEqual(synthetic_server_args({}), [])

    def test_declared_4485_is_retained_and_forwarded_exactly(self) -> None:
        row = {"synthetic_units": 4485}
        self.assertEqual(synthetic_units(row), 4485)
        self.assertEqual(synthetic_server_args(row), ["--synthetic-units", "4485"])
        self.assertEqual(add_synthetic_param({"package": "muster"}, row), {"package": "muster", "synthetic_units": 4485})

    def test_zero_and_u32_max_are_valid_json_integers(self) -> None:
        self.assertEqual(synthetic_server_args({"synthetic_units": 0}), ["--synthetic-units", "0"])
        self.assertEqual(synthetic_units({"synthetic_units": 0xFFFF_FFFF}), 0xFFFF_FFFF)

    def test_invalid_synthetic_values_are_rejected(self) -> None:
        for value in (True, False, "4485", 1.0, -1, 0x1_0000_0000, None, [], {}):
            with self.subTest(value=value), self.assertRaises(ValueError):
                synthetic_units({"synthetic_units": value})

    def test_gpu_device_audit_is_default_off_and_limited_to_slot1_physical_runs(self) -> None:
        slot1 = json.loads((pathlib.Path(__file__).resolve().parents[1] / "browser/rows/SLOT1.json").read_text())
        slot1["stages"]["task16"].pop("gpu_device_audit", None)
        slot1["stages"]["task16"].pop("startup_raf_probe", None)
        self.assertFalse(validate_task16_plan(slot1).get("gpu_device_audit", False))
        slot1["stages"]["task16"]["gpu_device_audit"] = True
        self.assertTrue(validate_task16_plan(slot1)["gpu_device_audit"])
        candidate = json.loads((pathlib.Path(__file__).resolve().parents[1] / "browser/rows/SLOT1.json").read_text())
        candidate["stages"]["task16"]["status"] = "candidate"
        candidate["instrument_status"] = "SOURCE CANDIDATE ONLY; guarded SLOT1 source preparation"
        self.assertTrue(validate_task16_plan(candidate)["gpu_device_audit"])
        slot1["stages"]["task16"]["gpu_device_audit"] = "true"
        with self.assertRaisesRegex(ValueError, "must be a boolean"):
            validate_task16_plan(slot1)

    def test_startup_raf_probe_is_default_off_and_requires_guarded_slot1_gpu_audit(self) -> None:
        slot1 = json.loads((pathlib.Path(__file__).resolve().parents[1] / "browser/rows/SLOT1.json").read_text())
        plan = slot1["stages"]["task16"]
        plan["status"] = "candidate"
        slot1["instrument_status"] = "SOURCE CANDIDATE ONLY; guarded SLOT1 source preparation"
        plan.pop("gpu_device_audit", None)
        plan.pop("startup_raf_probe", None)
        self.assertFalse(validate_task16_plan(slot1).get("startup_raf_probe", False))
        plan["startup_raf_probe"] = True
        with self.assertRaisesRegex(ValueError, "GPU device audit enabled"):
            validate_task16_plan(slot1)
        plan["gpu_device_audit"] = True
        self.assertTrue(validate_task16_plan(slot1)["startup_raf_probe"])
        plan["startup_raf_probe"] = "true"
        with self.assertRaisesRegex(ValueError, "startup_raf_probe must be a boolean"):
            validate_task16_plan(slot1)

    def test_stored_client_settings_are_limited_to_slot1_with_a_valid_frame_limit_and_reason(self) -> None:
        rows = pathlib.Path(__file__).resolve().parents[1] / "browser/rows"
        slot1 = json.loads((rows / "SLOT1.json").read_text())
        self.assertEqual(validate_task16_plan(slot1)["client_settings"]["frame_limit"], 30)
        plan = slot1["stages"]["task16"]
        for value in (5, 241, -1, "30", 30.0, True, None):
            plan["client_settings"] = {"frame_limit": value, "reason": "three clients"}
            with self.subTest(value=value), self.assertRaisesRegex(ValueError, "frame_limit must be 0 or 10 to 240"):
                validate_task16_plan(slot1)
        plan["client_settings"] = {"frame_limit": 30, "reason": " "}
        with self.assertRaisesRegex(ValueError, "reason must be a nonempty text"):
            validate_task16_plan(slot1)
        plan["client_settings"] = {"frame_limit": 30}
        with self.assertRaisesRegex(ValueError, "exactly frame_limit and reason"):
            validate_task16_plan(slot1)
        for name in sorted(path.name for path in rows.glob("*.json")):
            row = json.loads((rows / name).read_text())
            if row.get("row") == "SLOT1":
                continue
            stages = row.get("stages")
            task16 = stages.get("task16") if isinstance(stages, dict) else None
            with self.subTest(row=name):
                self.assertFalse(isinstance(task16, dict) and "client_settings" in task16, "only SLOT1 stores client settings")
            if isinstance(task16, dict) and row.get("row") in {"KM1", "KM2", "SEL4", "CAM2", "SH1", "PER1"}:
                task16["client_settings"] = {"frame_limit": 30, "reason": "three clients"}
                with self.subTest(row=name), self.assertRaisesRegex(ValueError, "limited to SLOT1|unknown fields"):
                    validate_task16_plan(row)


if __name__ == "__main__":
    unittest.main()
