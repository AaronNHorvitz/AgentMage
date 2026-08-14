"""Mutation tests for S-012-I02 task-classification evidence."""

import copy
import unittest

from scripts import task_classification_evidence as evidence


class TaskClassificationEvidenceTests(unittest.TestCase):
    def sources(self) -> dict[str, str]:
        return {path: (evidence.ROOT / path).read_text() for path in evidence.SOURCE_PATHS}

    def valid(self) -> dict:
        return {
            "artifact_id": "s-012-i02-task-classification",
            "case_ids": list(evidence.CASE_IDS),
            "claims": copy.deepcopy(evidence.CLAIMS),
            "external_network_used": False,
            "limitations": list(evidence.LIMITATIONS),
            "private_user_data_used": False,
            "schema_version": 1,
            "source_revision": "a" * 40,
            "sources": [{"bytes": 1, "path": path, "sha256": "b" * 64} for path in evidence.SOURCE_PATHS],
            "status": "pass-deterministic-descriptive-classification",
            "task_ids": ["12.1.1.2", "S-012-I02"],
            "verification_commands": evidence.expected_commands(),
        }

    def test_current_sources_and_report_are_closed(self) -> None:
        self.assertEqual(evidence.validate_sources(self.sources()), [])
        self.assertEqual(evidence.validate_report(self.valid()), [])

    def test_case_document_and_source_mutations_fail(self) -> None:
        sources = self.sources()
        changed = copy.deepcopy(sources)
        changed[evidence.SOURCE_PATHS[0]] = changed[evidence.SOURCE_PATHS[0]].replace("`TC-03`", "`TC-99`", 1)
        self.assertIn("S-012-I02 case closure changed", evidence.validate_sources(changed))
        for path, fragments in evidence.SOURCE_FRAGMENTS.items():
            for fragment in fragments:
                changed = copy.deepcopy(sources)
                changed[path] = changed[path].replace(fragment, "removed", 1)
                self.assertTrue(evidence.validate_sources(changed))

    def test_authority_semantic_execution_network_and_fuzz_overclaims_fail(self) -> None:
        for key, value in (
            ("classification_authority_admitted_count", 1),
            ("natural_language_intent_parsing_implemented", True),
            ("semantic_classifier_implemented", True),
            ("tool_or_model_executions", 1),
            ("persistent_state_changed", True),
            ("external_network_used", True),
            ("manual_fuzzing_executed", True),
            ("release_support", True),
        ):
            report = self.valid()
            report["claims"][key] = value
            self.assertTrue(evidence.validate_report(report))

    def test_revision_source_limit_command_and_data_scope_mutations_fail(self) -> None:
        for mutation in ("revision", "source", "limit", "command", "private", "network"):
            report = self.valid()
            if mutation == "revision":
                report["source_revision"] = "HEAD"
            elif mutation == "source":
                report["sources"].pop()
            elif mutation == "limit":
                report["limitations"].pop()
            elif mutation == "command":
                report["verification_commands"].pop()
            elif mutation == "private":
                report["private_user_data_used"] = True
            else:
                report["external_network_used"] = True
            self.assertTrue(evidence.validate_report(report))


if __name__ == "__main__":
    unittest.main()
