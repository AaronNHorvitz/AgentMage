from __future__ import annotations

import copy
import json
import unittest

from scripts import story_16_1_security_evidence as evidence


class Story161SecurityEvidenceTests(unittest.TestCase):
    def sources(self) -> dict[str, str]:
        return {path: (evidence.ROOT / path).read_text() for path in evidence.SOURCE_PATHS}

    def report(self) -> dict[str, object]:
        return {
            "artifact_id": "story-16-1-local-product-security-evidence-map",
            "external_network_used": False,
            "independent_review_complete": False,
            "limitations": [
                "Native macOS worker evidence and independent human review remain blockers.",
                "Manual fuzzing remains deferred; no release or supported-platform claim is made.",
            ],
            "mappings": copy.deepcopy(evidence.expected_mappings()),
            "private_user_data_used": False,
            "requirement_ids": list(evidence.EXPECTED_REQUIREMENTS),
            "schema_version": 1,
            "source_revision": "a" * 40,
            "sources": [
                {"bytes": 1, "path": path, "sha256": "b" * 64}
                for path in evidence.SOURCE_PATHS
            ],
            "status": "pass-local-mapping-with-external-blockers",
            "task_ids": ["16.1.3.5"],
        }

    def test_current_sources_and_report_are_closed(self) -> None:
        self.assertEqual(evidence.validate_sources(self.sources()), [])
        self.assertEqual(evidence.validate_report(self.report()), [])

    def test_requirement_document_and_report_scope_mutations_fail(self) -> None:
        changed = self.sources()
        changed[evidence.SOURCE_PATHS[0]] = changed[evidence.SOURCE_PATHS[0]].replace(
            evidence.DOCUMENT_FRAGMENTS[0], "mutated", 1
        )
        self.assertTrue(evidence.validate_sources(changed))

        changed = self.sources()
        changed["SECURITY-REVIEW.md"] = changed["SECURITY-REVIEW.md"].replace(
            "`SR-ACC-001`", "`SR-ACC-X01`", 1
        )
        self.assertTrue(evidence.validate_sources(changed))

        changed = self.sources()
        report = json.loads(changed[evidence.LOCAL_REPORT])
        report["verification_evidence"]["independent_review"] = True
        changed[evidence.LOCAL_REPORT] = json.dumps(report)
        self.assertTrue(evidence.validate_sources(changed))

    def test_worker_overclaim_and_omission_mutations_fail(self) -> None:
        changed = self.sources()
        report = json.loads(changed[evidence.WORKER_REPORT])
        report["attack_matrix_complete"] = True
        changed[evidence.WORKER_REPORT] = json.dumps(report)
        self.assertTrue(evidence.validate_sources(changed))

        changed = self.report()
        changed["mappings"] = changed["mappings"][:-1]
        self.assertTrue(evidence.validate_report(changed))

    def test_review_release_source_and_private_data_mutations_fail(self) -> None:
        mutations: list[dict[str, object]] = []
        for key, value in (
            ("independent_review_complete", True),
            ("private_user_data_used", True),
            ("status", "pass-release"),
            ("source_revision", "HEAD"),
        ):
            changed = self.report()
            changed[key] = value
            mutations.append(changed)
        changed = self.report()
        changed["sources"] = list(reversed(changed["sources"]))
        mutations.append(changed)
        for mutation in mutations:
            self.assertTrue(evidence.validate_report(mutation))


if __name__ == "__main__":
    unittest.main()
