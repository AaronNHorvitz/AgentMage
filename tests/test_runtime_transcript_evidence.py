import unittest

from scripts import runtime_transcript_evidence as evidence


class RuntimeTranscriptEvidenceTests(unittest.TestCase):
    def sources(self):
        return {path: (evidence.ROOT / path).read_text() for path in evidence.SOURCE_PATHS}

    def report(self):
        return {
            "artifact_id": "task-12-1-2-3-runtime-transcripts",
            "case_ids": list(evidence.CASE_IDS),
            "claims": evidence.CLAIMS,
            "external_network_used": False,
            "limitations": evidence.LIMITATIONS,
            "private_user_data_used": False,
            "schema_version": 1,
            "source_revision": "a" * 40,
            "sources": [
                {"bytes": 1, "path": path, "sha256": "b" * 64}
                for path in evidence.SOURCE_PATHS
            ],
            "status": "pass-runtime-transcripts",
            "task_ids": ["12.1.2.3"],
            "verification_commands": evidence.expected_commands(),
        }

    def test_current_sources_and_report_are_closed(self):
        self.assertEqual(evidence.validate_sources(self.sources()), [])
        self.assertEqual(evidence.validate_report(self.report()), [])

    def test_document_and_source_mutations_fail(self):
        for path in evidence.SOURCE_PATHS[:4]:
            changed = self.sources()
            changed[path] = changed[path].replace(
                next(
                    fragment
                    for fragment in (
                        evidence.DOCUMENT_FRAGMENTS
                        if path == evidence.SOURCE_PATHS[0]
                        else evidence.SOURCE_FRAGMENTS.get(path, ("#!/usr/bin/env python3",))
                    )
                    if fragment in changed[path]
                ),
                "mutated-fragment",
                1,
            )
            self.assertTrue(evidence.validate_sources(changed))

    def test_claim_command_revision_and_limit_mutations_fail(self):
        mutations = []
        changed = self.report()
        changed["claims"] = {**evidence.CLAIMS, "false_success_claim_count": 1}
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
