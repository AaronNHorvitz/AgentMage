from __future__ import annotations

import copy
import hashlib
import unittest

from scripts import story_22_2_security_evidence as evidence


def synthetic_report() -> dict[str, object]:
    raw = b"synthetic raw evidence"
    report: dict[str, object] = {
        "schema_version": 1,
        "artifact_id": "story-22.2-product-security-evidence-map",
        "source_revision": "a" * 40,
        "status": "pass-local-story-security-evidence",
        "task_ids": ["22.2.3.6", "RV-17", "RV-18"],
        "requirement_ids": list(evidence.REQUIREMENT_IDS),
        "mappings": [
            {"requirement_id": identifier, **copy.deepcopy(evidence.MAPPINGS[identifier])}
            for identifier in evidence.REQUIREMENT_IDS
        ],
        "verification_commands": [
            {**identity, "elapsed_ms": 1, "exit_code": 0, "status": "pass"}
            for identity in evidence.expected_command_identity()
        ],
        "raw_trace": {
            "path": str(evidence.LOG_PATH.relative_to(evidence.ROOT)),
            "bytes": len(raw), "sha256": hashlib.sha256(raw).hexdigest(),
            "redactions": ["repository-root"],
        },
        "sources": [
            {"path": path, "bytes": 1, "sha256": "b" * 64}
            for path in evidence.SOURCE_PATHS
        ],
        "external_network_used": False,
        "private_user_data_used": False,
        "independent_automated_review_performed": True,
        "independent_human_review_performed": False,
        "independent_cryptographic_review_performed": False,
        "manual_fuzzing_executed": False,
        "release_approved": False,
        "limitations": list(evidence.LIMITATIONS),
        "report_sha256": evidence.ZERO_SHA256,
    }
    return evidence.seal_report(report)


class Story222SecurityEvidenceTests(unittest.TestCase):
    def test_synthetic_report_and_exact_mapping_are_closed(self) -> None:
        self.assertEqual(len(evidence.REQUIREMENT_IDS), 25)
        self.assertEqual(tuple(evidence.MAPPINGS), evidence.REQUIREMENT_IDS)
        self.assertEqual(evidence.validate_report(synthetic_report(), b"synthetic raw evidence"), [])

    def test_requirement_mapping_and_command_mutations_fail(self) -> None:
        report = synthetic_report()
        report["requirement_ids"] = list(evidence.REQUIREMENT_IDS[:-1])
        self.assertTrue(evidence.validate_report(report, b"synthetic raw evidence"))

        report = synthetic_report()
        report["mappings"] = copy.deepcopy(report["mappings"])
        report["mappings"][0]["evidence"] = []
        self.assertTrue(evidence.validate_report(report, b"synthetic raw evidence"))

        report = synthetic_report()
        report["verification_commands"] = copy.deepcopy(report["verification_commands"])
        report["verification_commands"][0]["exit_code"] = 1
        self.assertTrue(evidence.validate_report(report, b"synthetic raw evidence"))

    def test_every_scope_overclaim_is_rejected(self) -> None:
        for field, value in (
            ("external_network_used", True),
            ("private_user_data_used", True),
            ("independent_human_review_performed", True),
            ("independent_cryptographic_review_performed", True),
            ("manual_fuzzing_executed", True),
            ("release_approved", True),
        ):
            report = synthetic_report()
            report[field] = value
            self.assertTrue(evidence.validate_report(report, b"synthetic raw evidence"), field)

    def test_trace_source_and_digest_mutations_fail(self) -> None:
        report = synthetic_report()
        self.assertTrue(evidence.validate_report(report, b"different raw evidence"))

        report = synthetic_report()
        report["sources"] = list(reversed(report["sources"]))
        self.assertTrue(evidence.validate_report(report, b"synthetic raw evidence"))

        report = synthetic_report()
        report["report_sha256"] = "c" * 64
        self.assertTrue(evidence.validate_report(report, b"synthetic raw evidence"))

    @unittest.skipUnless(
        evidence.REPORT_PATH.is_file() and evidence.LOG_PATH.is_file(),
        "retained report is generated after source commit",
    )
    def test_retained_report_and_log_are_hash_bound(self) -> None:
        self.assertEqual(
            evidence.validate_current(evidence.read_report(), evidence.read_log()), []
        )


if __name__ == "__main__":
    unittest.main()
