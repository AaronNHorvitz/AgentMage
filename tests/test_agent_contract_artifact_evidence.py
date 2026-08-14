import copy
import re
import unittest

from scripts.agent_contract_artifact_evidence import SPEC
from scripts.revision_evidence import expected_commands, validate_report, validate_sources


class AgentContractArtifactEvidenceTests(unittest.TestCase):
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
            "sources": [
                {"bytes": 1, "path": path, "sha256": "b" * 64}
                for path in SPEC.source_paths
            ],
            "status": SPEC.status,
            "task_ids": list(SPEC.task_ids),
            "verification_commands": expected_commands(SPEC),
        }

    def test_current_sources_report_and_graph_counts_are_closed(self):
        sources = self.sources()
        self.assertEqual(validate_sources(SPEC, sources), [])
        self.assertEqual(validate_report(SPEC, self.report()), [])
        reference = sources[SPEC.source_paths[1]]
        state_rows = re.findall(r"\| (?:Active|Terminal) \| `[^`]+` \|", reference)
        self.assertEqual(len(state_rows), 17)
        graph = reference.split("```mermaid", 1)[1].split("```", 1)[0]
        self.assertEqual(graph.count(" --> "), 52)

    def test_document_and_source_mutations_fail(self):
        for path, fragment in [
            (SPEC.source_paths[0], SPEC.document_fragments[0]),
            (SPEC.source_paths[1], SPEC.source_fragments[SPEC.source_paths[1]][0]),
            (SPEC.source_paths[2], SPEC.source_fragments[SPEC.source_paths[2]][0]),
            (SPEC.source_paths[7], SPEC.source_fragments[SPEC.source_paths[7]][0]),
        ]:
            changed = self.sources()
            changed[path] = changed[path].replace(fragment, "mutated-fragment", 1)
            self.assertTrue(validate_sources(SPEC, changed))

    def test_claim_command_revision_and_limit_mutations_fail(self):
        mutations = []
        changed = self.report()
        changed["claims"]["legal_transition_count"] = 51
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
        changed["private_user_data_used"] = True
        mutations.append(changed)
        changed = self.report()
        changed["status"] = "pass-release"
        mutations.append(changed)
        for mutation in mutations:
            self.assertTrue(validate_report(SPEC, mutation))


if __name__ == "__main__":
    unittest.main()
