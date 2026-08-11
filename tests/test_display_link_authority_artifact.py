import copy
import json
import unittest

from scripts import display_link_authority_artifact as artifact


class DisplayLinkAuthorityArtifactTests(unittest.TestCase):
    def test_generated_corpus_is_deterministic_and_closed(self) -> None:
        content = artifact.generated_corpus()
        self.assertEqual(content, artifact.generated_corpus())
        self.assertEqual(
            artifact.validate_corpus(content),
            {
                "authority_surface_count": 5,
                "candidate_form_count": 2,
                "display_link_count": 128,
                "filesystem_observation_count": 0,
                "issued_grant_count": 0,
                "rejection_count": 1280,
            },
        )

    def test_omitted_or_changed_link_fails_closed(self) -> None:
        value = json.loads(artifact.generated_corpus())
        omitted = copy.deepcopy(value)
        omitted["links"].pop()
        changed = copy.deepcopy(value)
        changed["links"][0]["rendered_target"] += "-changed"
        for candidate in (omitted, changed):
            with self.subTest():
                with self.assertRaises(artifact.DisplayLinkAuthorityArtifactError):
                    artifact.validate_corpus(artifact.canonical_json(candidate))

    def test_report_rejects_authority_or_platform_overclaim(self) -> None:
        value = {
            "schema_version": 1,
            "task_ids": ["6.1.2.2", "6.1.3.3"],
            "artifact_id": "display-link-authority-replay-matrix",
            "status": "pass-shared-kernel-scope",
            "reference_revision": "a" * 40,
            "coverage": artifact.validate_corpus(artifact.generated_corpus()),
            "verification": {surface: "pass" for surface in artifact.AUTHORITY_SURFACES},
            "direct_grant_target_status": "descriptive-candidate-only",
            "macos_execution_status": "blocked-macos",
            "macos_evidence_substituted": False,
            "release_claim": "none",
            "limitations": ["one", "two"],
        }
        self.assertEqual(artifact.validate_report(value), [])
        for field, replacement in (
            ("direct_grant_target_status", "authoritative"),
            ("macos_execution_status", "verified"),
            ("macos_evidence_substituted", True),
            ("release_claim", "approved"),
        ):
            changed = copy.deepcopy(value)
            changed[field] = replacement
            with self.subTest(field=field):
                self.assertTrue(artifact.validate_report(changed))


if __name__ == "__main__":
    unittest.main()
