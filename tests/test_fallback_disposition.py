from __future__ import annotations

import copy
import tempfile
import unittest
from pathlib import Path

from scripts.fallback_disposition import (
    FallbackDispositionError,
    build_record,
    validate_record,
    write_record,
)


class FallbackDispositionTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.record = build_record("HEAD")

    def test_generated_record_is_rejected_disabled_and_valid(self) -> None:
        self.assertEqual(validate_record(self.record), [])
        self.assertEqual(self.record["decision"]["status"], "REJECTED")
        self.assertFalse(self.record["decision"]["candidate_enabled"])
        self.assertEqual(self.record["state"]["candidate_state"], "REJECTED_DISABLED")

    def test_identity_or_threshold_binding_changes_are_rejected(self) -> None:
        changed = copy.deepcopy(self.record)
        changed["corpus"]["sha256"] = "0" * 64
        changed["adapters"]["native_linux"]["metrics"]["citation_recall"] = 1.0
        failures = validate_record(changed)
        self.assertTrue(any("hash-bound source evidence" in item for item in failures))

    def test_activation_and_automatic_switch_are_rejected(self) -> None:
        changed = copy.deepcopy(self.record)
        changed["state"]["candidate_state"] = "ENABLED"
        changed["state"]["automatic_switch"] = True
        changed["state"]["activation_authorized"] = True
        failures = validate_record(changed)
        self.assertTrue(any("permits activation" in item for item in failures))

    def test_mac_result_cannot_be_claimed(self) -> None:
        changed = copy.deepcopy(self.record)
        changed["adapters"]["macos_native"]["execution_status"] = "COMPLETE"
        failures = validate_record(changed)
        self.assertTrue(any("hash-bound source evidence" in item for item in failures))

    def test_writer_refuses_to_overwrite_record(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            target = Path(temporary) / "record.json"
            target.write_text("{}", encoding="utf-8")
            with self.assertRaises(FallbackDispositionError):
                write_record(target, "HEAD")


if __name__ == "__main__":
    unittest.main()
