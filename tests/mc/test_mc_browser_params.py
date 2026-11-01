import json
import pathlib
import sys
import unittest

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parents[2] / "scripts"))

from mc_browser_params import add_synthetic_param, synthetic_server_args, synthetic_units


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


if __name__ == "__main__":
    unittest.main()
