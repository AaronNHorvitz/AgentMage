import copy
import json
import unittest

from scripts import path_corpus_artifact as artifact


class PathCorpusArtifactTests(unittest.TestCase):
    def test_generated_corpus_has_closed_distribution_and_zero_escape(self) -> None:
        content = artifact.generated_corpus()
        self.assertEqual(content, artifact.generated_corpus())
        self.assertEqual(
            artifact.validate_corpus(content),
            {
                "accepted_control_count": 128,
                "admitted_escape_count": 0,
                "case_count": 640,
                "case_class_count": 10,
                "cases_per_class": 64,
                "invalid_encoding_count": 64,
                "rejected_escape_attempt_count": 512,
            },
        )

    def test_duplicate_or_admitted_case_fails_closed(self) -> None:
        value = json.loads(artifact.generated_corpus())
        duplicate = copy.deepcopy(value)
        duplicate["cases"][1]["case_id"] = duplicate["cases"][0]["case_id"]
        admitted = copy.deepcopy(value)
        admitted["cases"][0]["admitted_escape_count"] = 1
        for changed in (duplicate, admitted):
            with self.subTest():
                with self.assertRaises(artifact.PathCorpusArtifactError):
                    artifact.validate_corpus(artifact.canonical_json(changed))

    def test_report_rejects_macos_substitution_and_coverage_drift(self) -> None:
        value = {
            "schema_version": 1,
            "task_ids": ["6.1.2.2", "6.1.3.1"],
            "artifact_id": "canonicalization-display-link-path-corpus",
            "status": "pass-shared-fedora-parser-scope",
            "reference_revision": "a" * 40,
            "coverage": artifact.validate_corpus(artifact.generated_corpus()),
            "verification": {f"check-{index}": "pass" for index in range(6)},
            "platform_status": {
                "shared_contract": "verified",
                "fedora": "verified-local",
                "ubuntu": "not-executed",
                "macos_collision_handling": "blocked-macos",
            },
            "macos_evidence_substituted": False,
            "filesystem_observation_count": 0,
            "release_claim": "none",
            "limitations": ["one", "two", "three"],
        }
        self.assertEqual(artifact.validate_report(value), [])
        substituted = copy.deepcopy(value)
        substituted["macos_evidence_substituted"] = True
        drifted = copy.deepcopy(value)
        drifted["coverage"]["admitted_escape_count"] = 1
        promoted = copy.deepcopy(value)
        promoted["platform_status"]["macos_collision_handling"] = "verified"
        self.assertTrue(artifact.validate_report(substituted))
        self.assertTrue(artifact.validate_report(drifted))
        self.assertTrue(artifact.validate_report(promoted))


if __name__ == "__main__":
    unittest.main()
