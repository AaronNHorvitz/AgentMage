from __future__ import annotations

import copy
import unittest

from scripts.story_22_2_artifact_integrity_evidence import (
    COMMANDS,
    COVERAGE,
    REPORT_PATH,
    command_digest,
    read_report,
    validate_report,
)


class Story222ArtifactIntegrityEvidenceTests(unittest.TestCase):
    def test_coverage_names_every_required_integrity_class(self) -> None:
        self.assertEqual(
            set(COVERAGE),
            {
                "digest-size-media-preview-retention",
                "identity-reference-owner-checkpoint",
                "encryption-key-binding-and-plaintext-exclusion",
                "missing-corrupt-quarantine-and-cleanup",
                "duplicate-collision-partial-and-oversized",
                "expired-stale-and-unknown-version",
                "path-link-namespace-and-public-evidence-separation",
                "operator-projection-and-generated-file-route",
                "schema-lifecycle-and-path-mutations",
            },
        )
        exercised = {
            reference
            for references in COVERAGE.values()
            for reference in references
        }
        for case in COMMANDS:
            for test_name in case["tests"]:
                reference = f"{case['id']}:{test_name}"
                if test_name in {
                    "publication_deduplicates_without_broadening_owner_or_reference_state",
                    "story_22_2_native_crash_matrix_reconciles_every_artifact_boundary",
                }:
                    continue
                self.assertIn(reference, exercised)

    def test_command_identifiers_bind_exact_argv(self) -> None:
        identifiers = [command_digest(case["argv"]) for case in COMMANDS]
        self.assertEqual(len(identifiers), len(set(identifiers)))
        changed = list(COMMANDS[0]["argv"])
        changed.append("--ignored")
        self.assertNotEqual(command_digest(COMMANDS[0]["argv"]), command_digest(tuple(changed)))
        self.assertIn(
            "publication_rejects_mismatched_producer_authority_before_staging",
            COMMANDS[0]["tests"],
        )

    @unittest.skipUnless(REPORT_PATH.is_file(), "retained report is generated after source commit")
    def test_current_report_and_raw_trace_are_hash_bound(self) -> None:
        report = read_report()
        self.assertEqual(validate_report(report), [])

        changed = copy.deepcopy(report)
        changed["coverage"]["expired-stale-and-unknown-version"] = []
        self.assertIn("runtime.artifact_integrity.report_coverage", validate_report(changed))

        changed = copy.deepcopy(report)
        changed["commands"][0]["command_id"] = "0" * 64
        self.assertIn("runtime.artifact_integrity.command.kernel-contracts", validate_report(changed))


if __name__ == "__main__":
    unittest.main()
