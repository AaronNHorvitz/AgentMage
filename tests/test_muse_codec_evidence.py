from __future__ import annotations

import copy
import unittest

from scripts import muse_codec_evidence as evidence


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


class MuseCodecEvidenceTests(unittest.TestCase):
    def test_complete_mutation_matrix_and_neutral_kernel_pass(self) -> None:
        report = evidence.build_report("a" * 40, commands())
        self.assertEqual(evidence.validate_report(report), [])
        self.assertEqual(report["disposition"]["status"], "PASS-CONTRACT")
        self.assertEqual(len(report["mutations"]), len(evidence.MUTATIONS))

    def test_failed_command_forces_blocked_disposition(self) -> None:
        values = commands()
        values[1]["exit_code"] = 1
        report = evidence.build_report("a" * 40, values)
        self.assertEqual(report["disposition"]["status"], "BLOCKED")
        self.assertEqual(evidence.validate_report(report), [])

    def test_activation_mutation_matrix_and_check_drift_fail(self) -> None:
        base = evidence.build_report("a" * 40, commands())
        for mutate in (
            lambda report: report["disposition"].update({"profile_activated": True}),
            lambda report: report["mutations"].pop(),
            lambda report: report["checks"][0].update({"result": "FAIL"}),
            lambda report: report.update({"unknown": True}),
        ):
            report = copy.deepcopy(base)
            mutate(report)
            self.assertTrue(evidence.validate_report(report))


if __name__ == "__main__":
    unittest.main()
