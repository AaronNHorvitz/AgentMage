from __future__ import annotations

import copy
import unittest

from scripts.macos_runtime_admission import load_record, validate_record


class MacosRuntimeAdmissionTests(unittest.TestCase):
    def setUp(self) -> None:
        self.record = load_record()

    def test_committed_runtime_is_exact_and_blocked_on_hardware(self) -> None:
        self.assertEqual(validate_record(self.record), [])
        self.assertEqual(
            self.record["decision"]["hardware_status"],
            "BLOCKED_REQUIRED_MACBOOK_PRO_M5",
        )
        self.assertFalse(self.record["decision"]["linux_evidence_substituted"])
        self.assertFalse(self.record["decision"]["release_approval"])

    def test_unknown_or_missing_fields_are_rejected(self) -> None:
        missing = copy.deepcopy(self.record)
        del missing["artifact"]
        extra = copy.deepcopy(self.record)
        extra["mutable_tag"] = "latest"
        self.assertTrue(validate_record(missing))
        self.assertTrue(validate_record(extra))

    def test_archive_binary_and_source_substitutions_are_rejected(self) -> None:
        mutations = {
            "archive": lambda value: value["artifact"].update({"sha256": "0" * 64}),
            "binary": lambda value: value["critical_files"]["llama_server"].update(
                {"sha256": "1" * 64}
            ),
            "source": lambda value: value.update({"source_commit": "2" * 40}),
        }
        for label, mutate in mutations.items():
            with self.subTest(label=label):
                changed = copy.deepcopy(self.record)
                mutate(changed)
                self.assertTrue(validate_record(changed))

    def test_hardware_and_release_status_cannot_be_overstated(self) -> None:
        changed = copy.deepcopy(self.record)
        changed["decision"]["execution_status"] = "PASS"
        changed["decision"]["hardware_status"] = "AVAILABLE"
        changed["decision"]["linux_evidence_substituted"] = True
        changed["decision"]["release_approval"] = True
        self.assertTrue(validate_record(changed))


if __name__ == "__main__":
    unittest.main()
