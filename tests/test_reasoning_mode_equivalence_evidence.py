import copy
import unittest

from scripts import reasoning_mode_equivalence_evidence as evidence


class ReasoningModeEquivalenceEvidenceTests(unittest.TestCase):
    def sources(self):
        return {path: (evidence.ROOT / path).read_text() for path in evidence.SOURCE_PATHS}

    def report(self):
        return {
            "artifact_id": "s-012-it01-reasoning-mode-equivalence",
            "case_ids": list(evidence.CASE_IDS),
            "claims": copy.deepcopy(evidence.CLAIMS),
            "external_network_used": False,
            "limitations": list(evidence.LIMITATIONS),
            "private_user_data_used": False,
            "schema_version": 1,
            "source_revision": "a" * 40,
            "sources": [{"bytes": 1, "path": path, "sha256": "b" * 64} for path in evidence.SOURCE_PATHS],
            "status": "pass-reasoning-mode-equivalence",
            "task_ids": ["12.1.3.4", "S-012-IT01"],
            "verification_commands": evidence.expected_commands(),
        }

    def test_current_sources_and_report_are_closed(self):
        self.assertEqual(evidence.validate_sources(self.sources()), [])
        self.assertEqual(evidence.validate_report(self.report()), [])

    def test_document_fixture_and_runtime_mutations_fail(self):
        for path, fragment in [
            (evidence.SOURCE_PATHS[0], evidence.DOCUMENT_FRAGMENTS[0]),
            (evidence.SOURCE_PATHS[1], evidence.SOURCE_FRAGMENTS[evidence.SOURCE_PATHS[1]][0]),
            (evidence.SOURCE_PATHS[2], evidence.SOURCE_FRAGMENTS[evidence.SOURCE_PATHS[2]][0]),
        ]:
            changed = self.sources()
            changed[path] = changed[path].replace(fragment, "mutated-fragment", 1)
            self.assertTrue(evidence.validate_sources(changed))

    def test_claim_command_revision_and_limit_mutations_fail(self):
        mutations = []
        changed = self.report()
        changed["claims"]["status_event_count_per_task"] = 7
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
        changed["external_network_used"] = True
        mutations.append(changed)
        changed = self.report()
        changed["status"] = "pass-release"
        mutations.append(changed)
        for mutation in mutations:
            self.assertTrue(evidence.validate_report(mutation))


if __name__ == "__main__":
    unittest.main()
