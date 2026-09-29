from __future__ import annotations

import copy
import hashlib
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
        self.assertEqual(evidence.validate_report(report, verify_current=False), [])
        self.assertEqual(report["disposition"]["status"], "PASS-CONTRACT")
        self.assertEqual(len(report["mutations"]), len(evidence.MUTATIONS))
        self.assertEqual(len(report["diagnostics"]), len(evidence.DIAGNOSTICS))

    def test_failed_command_forces_blocked_disposition(self) -> None:
        values = commands()
        values[1]["exit_code"] = 1
        report = evidence.build_report("a" * 40, values)
        self.assertEqual(report["disposition"]["status"], "BLOCKED")
        self.assertEqual(evidence.validate_report(report, verify_current=False), [])

    def test_activation_mutation_matrix_and_check_drift_fail(self) -> None:
        base = evidence.build_report("a" * 40, commands())
        for mutate in (
            lambda report: report["disposition"].update({"profile_activated": True}),
            lambda report: report["mutations"].pop(),
            lambda report: report["diagnostics"].pop(),
            lambda report: report["checks"][0].update({"result": "FAIL"}),
            lambda report: report.update({"unknown": True}),
        ):
            report = copy.deepcopy(base)
            mutate(report)
            self.assertTrue(evidence.validate_report(report, verify_current=False))

    def test_current_validation_requires_the_whole_scan_set_at_its_revision(self) -> None:
        report = evidence.build_report("a" * 40, commands())

        def working_tree(revision: str, path: str) -> bytes:
            self.assertEqual(revision, "a" * 40)
            return (evidence.ROOT / path).read_bytes()

        self.assertEqual(evidence.validate_report(report, read_revision=working_tree), [])
        scanned = sorted(set(report["source_sha256"]) - set(evidence.SOURCE_PATHS))
        self.assertGreater(len(scanned), 1)
        reduced = copy.deepcopy(report)
        for path in scanned[1:]:
            del reduced["source_sha256"][path]
        # The structural check alone accepts one scanned file; the current check does not.
        self.assertEqual(evidence.validate_report(reduced, verify_current=False), [])
        self.assertIn(
            "codec source set differs from the current scan set",
            evidence.validate_report(reduced, read_revision=working_tree),
        )
        changed = copy.deepcopy(report)
        changed["source_sha256"][scanned[0]] = hashlib.sha256(b"other").hexdigest()
        self.assertIn(
            f"codec source differs from its revision: {scanned[0]}",
            evidence.validate_report(changed, read_revision=working_tree),
        )
        # A synthetic revision has no committed files.
        self.assertTrue(evidence.validate_report(report))


if __name__ == "__main__":
    unittest.main()
