from __future__ import annotations

import copy
import json
import unittest

from scripts import story_17_1_security_evidence as evidence


class Story171SecurityEvidenceTests(unittest.TestCase):
    def sources(self) -> dict[str, str]:
        return {path: (evidence.ROOT / path).read_text() for path in evidence.SOURCE_PATHS}

    def report(self) -> dict[str, object]:
        return {
            "artifact_id": "story-17-1-local-product-security-evidence-map",
            "independent_review_complete": False,
            "limitations": [
                "Native macOS evidence and independent human review remain blockers.",
                "Manual Git parser fuzzing remains deferred; no release claim is made.",
                "Network closure is demonstrated only during native Linux subject execution.",
            ],
            "mappings": copy.deepcopy(evidence.expected_mappings()),
            "network_used_during_subject_execution": False,
            "private_user_data_used": False,
            "requirement_ids": list(evidence.EXPECTED_REQUIREMENTS),
            "schema_version": 1,
            "source_revision": "a" * 40,
            "sources": [
                {"bytes": 1, "path": path, "sha256": "b" * 64}
                for path in evidence.SOURCE_PATHS
            ],
            "status": "pass-local-mapping-with-external-blockers",
            "task_ids": ["17.1.3.5"],
        }

    def test_current_sources_and_report_are_closed(self) -> None:
        self.assertEqual(evidence.validate_sources(self.sources()), [])
        self.assertEqual(evidence.validate_report(self.report()), [])

    def test_requirement_document_and_local_scope_mutations_fail(self) -> None:
        changed = self.sources()
        changed[evidence.DOCUMENT] = changed[evidence.DOCUMENT].replace(
            evidence.DOCUMENT_FRAGMENTS[0], "mutated", 1
        )
        self.assertTrue(evidence.validate_sources(changed))

        changed = self.sources()
        report = json.loads(changed[evidence.LOCAL_REPORT])
        report["platform_evidence"]["macos_git_worker"] = True
        changed[evidence.LOCAL_REPORT] = json.dumps(report)
        self.assertTrue(evidence.validate_sources(changed))

    def test_linux_overclaim_and_mapping_omission_fail(self) -> None:
        changed = self.sources()
        report = json.loads(changed[evidence.LINUX_REPORT])
        report["release_claim"] = True
        changed[evidence.LINUX_REPORT] = json.dumps(report)
        self.assertTrue(evidence.validate_sources(changed))

        changed_report = self.report()
        changed_report["mappings"] = changed_report["mappings"][:-1]
        self.assertTrue(evidence.validate_report(changed_report))

    def test_review_release_network_source_and_private_data_mutations_fail(self) -> None:
        mutations: list[dict[str, object]] = []
        for key, value in (
            ("independent_review_complete", True),
            ("private_user_data_used", True),
            ("network_used_during_subject_execution", True),
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
