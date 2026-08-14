from __future__ import annotations

import copy
import unittest

from scripts import muse_native_adapter_evidence as evidence


def commands(exit_code: int = 0) -> list[dict[str, object]]:
    return [
        {
            "id": identifier,
            "argv": list(command),
            "exit_code": exit_code,
            "output_sha256": "a" * 64,
        }
        for identifier, command in evidence.COMMANDS
    ]


class MuseNativeAdapterEvidenceTests(unittest.TestCase):
    def test_closed_nonactivating_report_validates(self) -> None:
        report = evidence.build_report("a" * 40, commands())
        self.assertEqual(evidence.validate_report(report), [])
        self.assertEqual(report["disposition"]["status"], "CONTRACT-PASS-LIVE-BLOCKED")

    def test_failed_command_forces_blocked_disposition(self) -> None:
        values = commands()
        values[0]["exit_code"] = 1
        report = evidence.build_report("a" * 40, values)
        self.assertEqual(report["disposition"]["status"], "BLOCKED")
        self.assertFalse(report["disposition"]["adapter_contract_implemented"])
        self.assertEqual(evidence.validate_report(report), [])

    def test_activation_live_claim_command_drift_and_unknown_fields_fail(self) -> None:
        base = evidence.build_report("a" * 40, commands())
        for mutate in (
            lambda report: report["disposition"].update({"activation": True}),
            lambda report: report["disposition"].update({"adapter_live_proven": True}),
            lambda report: report["commands"][0].update({"argv": ["cargo", "test"]}),
            lambda report: report["checks"][0].update({"result": "FAIL", "code": "changed"}),
            lambda report: report["package"].update({"sha256": "f" * 64}),
            lambda report: report.update({"source_revision": "HEAD"}),
            lambda report: report.update({"unknown": True}),
        ):
            report = copy.deepcopy(base)
            mutate(report)
            self.assertTrue(evidence.validate_report(report))


if __name__ == "__main__":
    unittest.main()
