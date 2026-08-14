import copy
import json
import unittest

from scripts.d027_s12_classifier_evidence import SPEC
from scripts.revision_evidence import expected_commands, validate_report, validate_sources


EXPECTED_CLASSES = [
    "allow_like_unknown_fields",
    "low_confidence",
    "disagreement",
    "truncated",
    "timed_out",
    "malformed",
    "unavailable",
    "out_of_distribution",
    "authority_field_escalation",
    "complete_restrictive",
]


class D027S12ClassifierEvidenceTests(unittest.TestCase):
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

    def test_current_sources_profile_and_report_are_closed(self):
        sources = self.sources()
        self.assertEqual(validate_sources(SPEC, sources), [])
        self.assertEqual(validate_report(SPEC, self.report()), [])
        profile = json.loads(sources[SPEC.source_paths[1]])
        self.assertEqual(profile["classes"], EXPECTED_CLASSES)
        self.assertEqual(
            len(profile["classes"]) * profile["outputs_per_class"],
            profile["output_count"],
        )
        self.assertEqual(sum(profile["expected_counts"].values()), 1280)

    def test_document_profile_contract_and_campaign_mutations_fail(self):
        for path, fragment in [
            (SPEC.source_paths[0], SPEC.document_fragments[0]),
            (SPEC.source_paths[1], SPEC.source_fragments[SPEC.source_paths[1]][0]),
            (SPEC.source_paths[2], SPEC.source_fragments[SPEC.source_paths[2]][0]),
            (SPEC.source_paths[3], SPEC.source_fragments[SPEC.source_paths[3]][0]),
        ]:
            changed = self.sources()
            changed[path] = changed[path].replace(fragment, "mutated-fragment", 1)
            self.assertTrue(validate_sources(SPEC, changed))

    def test_claim_command_revision_and_limit_mutations_fail(self):
        mutations = []
        changed = self.report()
        changed["claims"]["broader_authority_count"] = 1
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
