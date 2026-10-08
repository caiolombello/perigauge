import importlib.machinery
import importlib.util
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

MODULE_PATH = Path(__file__).parents[1] / "perif-battery"
LOADER = importlib.machinery.SourceFileLoader("perif_battery", str(MODULE_PATH))
SPEC = importlib.util.spec_from_loader(LOADER.name, LOADER)
perif_battery = importlib.util.module_from_spec(SPEC)
LOADER.exec_module(perif_battery)


class PerifBatteryTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.cache = Path(self.directory.name) / "cache" / "last.json"
        self.cache_patch = patch.object(perif_battery, "CACHE", self.cache)
        self.cache_patch.start()

    def tearDown(self):
        self.cache_patch.stop()
        self.directory.cleanup()

    def test_cache_is_private_and_round_trips_json(self):
        value = {"m6": {"level": 55}, "now": 1}
        perif_battery._save_cache(value)
        self.assertEqual(perif_battery._load_cache(), value)
        self.assertEqual(oct(self.cache.stat().st_mode & 0o777), "0o600")
        self.assertEqual(oct(self.cache.parent.stat().st_mode & 0o777), "0o700")

    @patch.object(perif_battery, "read_mxkeys")
    @patch.object(perif_battery, "read_m6")
    def test_sleeping_mouse_keeps_last_level(self, read_m6, read_mxkeys):
        perif_battery._save_cache({"m6": {"level": 72, "ts": 10}})
        read_m6.return_value = {"level": None, "connected": True, "stale": True, "ts": 20}
        read_mxkeys.return_value = {"level": 80, "connected": True, "ts": 20}
        result = perif_battery.collect()
        self.assertEqual(result["m6"]["level"], 72)
        self.assertTrue(result["m6"]["stale"])
        self.assertEqual(result["m6"]["last_read"], 10)

    @patch.object(perif_battery, "read_mxkeys")
    @patch.object(perif_battery, "read_m6")
    def test_sleeping_mouse_without_cache_does_not_claim_cached_reading(self, read_m6, read_mxkeys):
        read_m6.return_value = {"level": None, "connected": True, "stale": True, "error": "asleep", "ts": 20}
        read_mxkeys.return_value = {"level": 80, "connected": True, "ts": 20}
        result = perif_battery.collect()
        self.assertIsNone(result["m6"]["level"])
        self.assertNotIn("last_read", result["m6"])
        self.assertEqual(result["m6"]["error"], "asleep")

    def test_solaar_parser_accepts_charging_status(self):
        completed = type("Result", (), {"stdout": "  1: MX Keys\n    Battery: 46%, BatteryStatus.charging", "returncode": 0})()
        with patch.object(perif_battery.subprocess, "run", return_value=completed):
            result = perif_battery.read_mxkeys()
        self.assertEqual(result["level"], 46)
        self.assertTrue(result["charging"])

    def test_solaar_uses_mxkeys_block_not_first_logitech_device(self):
        output = """  1: MX Master 3
    Battery: 92%, BatteryStatus.discharging
  2: MX Keys
    Battery: 31%, BatteryStatus.discharging
  3: MX Anywhere
    Battery: 7%, BatteryStatus.discharging
"""
        completed = type("Result", (), {"stdout": output, "returncode": 0})()
        with patch.object(perif_battery.subprocess, "run", return_value=completed):
            result = perif_battery.read_mxkeys()
        self.assertEqual(result["level"], 31)

    def test_solaar_textual_level_is_not_misrepresented_as_percentage(self):
        completed = type("Result", (), {"stdout": "  1: MX Keys\n    Battery: low, BatteryStatus.discharging", "returncode": 0})()
        with patch.object(perif_battery.subprocess, "run", return_value=completed):
            result = perif_battery.read_mxkeys()
        self.assertIsNone(result["level"])
        self.assertEqual(result["level_text"], "low")
        self.assertNotIn("error", result)

    def test_solaar_missing_program_is_a_specific_error(self):
        with patch.object(perif_battery.subprocess, "run", side_effect=FileNotFoundError):
            result = perif_battery.read_mxkeys()
        self.assertEqual(result["error"], "solaar-not-found")
