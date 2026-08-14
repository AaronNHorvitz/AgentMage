import copy
import json
import unittest

from scripts import story_12_2_security_evidence as evidence


class Story122SecurityEvidenceTests(unittest.TestCase):
    def sources(self):
        return {path: (evidence.ROOT / path).read_text() for path in evidence.SOURCE_PATHS}

    def report(self):
        return {
            "artifact_id": "story-12-2-product-security-evidence-map",
            "external_network_used": False,
            "limitations": list(evidence.LIMITATIONS),
            "mappings": [
                {
                    "requirement_id": identifier,
                    **copy.deepcopy(evidence.MAPPINGS[identifier]),
                }
                for identifier in evidence.EXPECTED_IDENTIFIERS
            ],
            "private_user_data_used": False,
            "requirement_ids": list(evidence.EXPECTED_IDENTIFIERS),
            "schema_version": 1,
            "source_revision": "a" * 40,
            "sources": [
                {"bytes": 1, "path": path, "sha256": "b" * 64}
                for path in evidence.SOURCE_PATHS
            ],
            "status": "pass-story-scope-security-mapping",
            "task_ids": ["12.2.4.5"],
            "verification_commands": evidence.expected_commands(),
        }

    def test_current_sources_and_report_are_closed(self):
        self.assertEqual(evidence.validate_sources(self.sources()), [])
        self.assertEqual(evidence.validate_report(self.report()), [])

    def test_document_registry_profile_and_artifact_mutations_fail(self):
        changed = self.sources()
        changed[evidence.SOURCE_PATHS[0]] = changed[evidence.SOURCE_PATHS[0]].replace(
            evidence.DOCUMENT_FRAGMENTS[0], "mutated", 1
        )
        self.assertTrue(evidence.validate_sources(changed))
        changed = self.sources()
        changed["SECURITY-REVIEW.md"] = changed["SECURITY-REVIEW.md"].replace(
            "`SR-AI-017`", "`SR-AI-X17`", 1
        )
        self.assertTrue(evidence.validate_sources(changed))
        changed = self.sources()
        profile_path = evidence.PROFILE_PATHS[0]
        profile = json.loads(changed[profile_path])
        profile["external_network"] = True
        changed[profile_path] = json.dumps(profile)
        self.assertTrue(evidence.validate_sources(changed))
        changed = self.sources()
        artifact_path = evidence.EVIDENCE_PATHS[0]
        artifact = json.loads(changed[artifact_path])
        artifact["private_user_data_used"] = True
        changed[artifact_path] = json.dumps(artifact)
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
