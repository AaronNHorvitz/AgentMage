"""Mutation tests for S-011-IT01 provider-substitution evidence."""

import copy
import unittest

from scripts import provider_substitution_evidence as evidence


class ProviderSubstitutionEvidenceTests(unittest.TestCase):
    def sources(self) -> dict[str, str]:
        return {path: (evidence.ROOT / path).read_text() for path in evidence.SOURCE_PATHS}

    def valid(self) -> dict:
        return {
            "artifact_id": "s-011-it01-provider-substitution-results",
            "case_ids": list(evidence.CASE_IDS),
            "claims": copy.deepcopy(evidence.CLAIMS),
            "external_network_used": False,
            "limitations": list(evidence.LIMITATIONS),
            "private_user_data_used": False,
            "schema_version": 1,
            "source_revision": "a" * 40,
            "sources": [{"bytes": 1, "path": path, "sha256": "b" * 64} for path in evidence.SOURCE_PATHS],
            "status": "pass-current-linux-provider-boundary",
            "task_ids": ["11.1.3.4", "S-011-IT01"],
            "verification_commands": evidence.expected_commands(),
        }

    def test_current_sources_and_report_are_closed(self) -> None:
        self.assertEqual(evidence.validate_sources(self.sources()), [])
        self.assertEqual(evidence.validate_report(self.valid()), [])

    def test_case_document_and_source_mutations_fail(self) -> None:
        sources = self.sources()
        changed = copy.deepcopy(sources)
        changed[evidence.SOURCE_PATHS[1]] = changed[evidence.SOURCE_PATHS[1]].replace("`IT-06`", "`IT-99`", 1)
        self.assertIn("S-011-IT01 case closure changed", evidence.validate_sources(changed))
        for path, fragments in evidence.SOURCE_FRAGMENTS.items():
            for fragment in fragments:
                changed = copy.deepcopy(sources)
                changed[path] = changed[path].replace(fragment, "removed", 1)
                self.assertTrue(evidence.validate_sources(changed))

    def test_claim_limit_and_command_mutations_fail(self) -> None:
        for key in ("integration_case_count", "plaintext_fallback_accepted_count", "secret_argument_or_environment_occurrences"):
            report = self.valid()
            report["claims"][key] += 1
            self.assertTrue(evidence.validate_report(report))
        report = self.valid()
        report["limitations"].pop()
        self.assertTrue(evidence.validate_report(report))
        report = self.valid()
        report["verification_commands"].pop()
        self.assertTrue(evidence.validate_report(report))

    def test_revision_source_and_data_scope_mutations_fail(self) -> None:
        report = self.valid()
        report["source_revision"] = "HEAD"
        self.assertTrue(evidence.validate_report(report))
        report = self.valid()
        report["sources"].pop()
        self.assertTrue(evidence.validate_report(report))
        for key in ("private_user_data_used", "external_network_used"):
            report = self.valid()
            report[key] = True
            self.assertTrue(evidence.validate_report(report))


if __name__ == "__main__":
    unittest.main()
