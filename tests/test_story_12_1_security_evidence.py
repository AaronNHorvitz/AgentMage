import copy
import unittest

from scripts import story_12_1_security_evidence as evidence


class Story121SecurityEvidenceTests(unittest.TestCase):
    def sources(self):
        return {path: (evidence.ROOT / path).read_text() for path in evidence.SOURCE_PATHS}

    def report(self):
        return {
            "artifact_id": "story-12-1-product-security-evidence-map",
            "external_network_used": False,
            "limitations": [
                "The map closes Story 12.1 evidence organization and does not mark any requirement complete for a product release.",
                "Production model and tool execution, durable restart, live UI, cross-platform packaging, release acceptance, and manual fuzzing remain later gates.",
            ],
            "mappings": [{"requirement_id": requirement, **copy.deepcopy(evidence.MAPPINGS[requirement])} for requirement in evidence.EXPECTED_REQUIREMENTS],
            "private_user_data_used": False,
            "requirement_ids": list(evidence.EXPECTED_REQUIREMENTS),
            "schema_version": 1,
            "source_revision": "a" * 40,
            "sources": [{"bytes": 1, "path": path, "sha256": "b" * 64} for path in evidence.SOURCE_PATHS],
            "status": "pass-story-scope-security-mapping",
            "task_ids": ["12.1.3.5"],
            "verification_commands": evidence.expected_commands(),
        }

    def test_current_sources_and_report_are_closed(self):
        self.assertEqual(evidence.validate_sources(self.sources()), [])
        self.assertEqual(evidence.validate_report(self.report()), [])

    def test_document_requirement_and_artifact_mutations_fail(self):
        changed = self.sources()
        changed[evidence.SOURCE_PATHS[0]] = changed[evidence.SOURCE_PATHS[0]].replace(evidence.DOCUMENT_FRAGMENTS[0], "mutated", 1)
        self.assertTrue(evidence.validate_sources(changed))
        changed = self.sources()
        changed["SECURITY-REVIEW.md"] = changed["SECURITY-REVIEW.md"].replace("`SR-AI-005`", "`SR-AI-X05`", 1)
        self.assertTrue(evidence.validate_sources(changed))
        changed = self.sources()
        artifact_path = evidence.EVIDENCE_PATHS[0]
        artifact = __import__("json").loads(changed[artifact_path])
        artifact["external_network_used"] = True
        changed[artifact_path] = __import__("json").dumps(artifact)
        self.assertTrue(evidence.validate_sources(changed))

    def test_mapping_command_revision_and_limit_mutations_fail(self):
        mutations = []
        changed = self.report()
        changed["mappings"] = changed["mappings"][:-1]
        mutations.append(changed)
        changed = self.report()
        changed["verification_commands"] = changed["verification_commands"][:-1]
        mutations.append(changed)
        changed = self.report()
        changed["source_revision"] = "HEAD"
        mutations.append(changed)
        changed = self.report()
        changed["limitations"] = changed["limitations"][:-1]
        mutations.append(changed)
        for mutation in mutations:
            self.assertTrue(evidence.validate_report(mutation))

    def test_source_identity_and_top_level_mutations_fail(self):
        mutations = []
        changed = self.report()
        changed["sources"][0]["sha256"] = "invalid"
        mutations.append(changed)
        changed = self.report()
        changed["sources"] = list(reversed(changed["sources"]))
        mutations.append(changed)
        changed = self.report()
        changed["private_user_data_used"] = True
        mutations.append(changed)
        changed = self.report()
        changed["status"] = "pass-release"
        mutations.append(changed)
        for mutation in mutations:
            self.assertTrue(evidence.validate_report(mutation))


if __name__ == "__main__":
    unittest.main()
