import copy
import unittest

from scripts.proposal_identity_evidence import SPEC
from scripts.revision_evidence import expected_commands, validate_report, validate_sources


class ProposalIdentityEvidenceTests(unittest.TestCase):
    def sources(self):
        return {path: (SPEC.root / path).read_text() for path in SPEC.source_paths}

    def report(self):
        return {
            "artifact_id": SPEC.artifact_id,
            "case_ids": list(SPEC.case_ids),
            "claims": copy.deepcopy(SPEC.claims),
            "external_network_used": False,
            "limitations": list(SPEC.limitations),
            "private_user_data_used": False,
            "schema_version": 1,
            "source_revision": "a" * 40,
            "sources": [{"bytes": 1, "path": path, "sha256": "b" * 64} for path in SPEC.source_paths],
            "status": SPEC.status,
            "task_ids": list(SPEC.task_ids),
            "verification_commands": expected_commands(SPEC),
        }

    def test_current_sources_and_report_are_closed(self):
        self.assertEqual(validate_sources(SPEC, self.sources()), [])
        self.assertEqual(validate_report(SPEC, self.report()), [])

    def test_document_contract_and_engine_mutations_fail(self):
        for path, fragment in [
            (SPEC.source_paths[0], SPEC.document_fragments[0]),
            (SPEC.source_paths[1], SPEC.source_fragments[SPEC.source_paths[1]][0]),
            (SPEC.source_paths[4], SPEC.source_fragments[SPEC.source_paths[4]][0]),
        ]:
            changed = self.sources()
            changed[path] = changed[path].replace(fragment, "mutated-fragment", 1)
            self.assertTrue(validate_sources(SPEC, changed))

    def test_claim_command_revision_and_limit_mutations_fail(self):
        mutations = []
        changed = self.report()
        changed["claims"]["maximum_admitted_proposals_per_turn"] = 2
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
            self.assertTrue(validate_report(SPEC, mutation))

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
            self.assertTrue(validate_report(SPEC, mutation))


if __name__ == "__main__":
    unittest.main()
