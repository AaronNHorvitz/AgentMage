from __future__ import annotations

import copy
import unittest
from unittest.mock import patch

from scripts import sprint_18_evidence as evidence


def commands() -> list[dict[str, object]]:
    return [
        {"id": identifier, "argv": list(argv), "exit_code": 0, "output_sha256": "a" * 64}
        for identifier, argv in evidence.COMMANDS
    ]


def grammar_bom() -> dict[str, object]:
    languages = ["rust", "python", "type_script", "tsx", "java_script", "swift"]
    return {
        "schema_version": 1,
        "grammar_set_sha256": "b" * 64,
        "grammars": [
            {
                "language": language,
                "crate_name": f"tree-sitter-{language}",
                "crate_version": "1.0.0",
                "repository": "https://example.invalid/grammar",
                "parser_version": "0.26.12",
                "abi_version": 15,
                "node_types_sha256": "c" * 64,
                "descriptor_sha256": "d" * 64,
            }
            for language in languages
        ],
    }


def report() -> dict[str, object]:
    with patch.object(evidence, "git_file", return_value=b"source"):
        return evidence.build_report("e" * 40, commands(), grammar_bom())


class Sprint18EvidenceTests(unittest.TestCase):
    def test_local_core_passes_without_platform_or_release_overclaim(self) -> None:
        value = report()
        self.assertEqual(evidence.validate_report(value, verify_current=False), [])
        self.assertTrue(value["summary"]["local_contract_passed"])
        self.assertEqual(value["summary"]["sprint_status"], "BLOCKED")
        self.assertEqual(value["implemented_contracts"]["supported_language_dialects"], 6)
        self.assertFalse(value["implemented_contracts"]["network_authority"])

    def test_platform_fuzz_review_and_release_overclaims_fail(self) -> None:
        mutations = (
            lambda value: value["summary"].update({"sprint_status": "PASS"}),
            lambda value: value["platform_evidence"].update({"linux_packaged_worker": True}),
            lambda value: value["platform_evidence"].update(
                {"linux_encrypted_persistent_cache": True}
            ),
            lambda value: value["platform_evidence"].update({"macos_native_map": True}),
            lambda value: value["platform_evidence"].update({"windows_native_map": True}),
            lambda value: value["verification_evidence"].update(
                {"manual_parser_fuzzing": True}
            ),
            lambda value: value["verification_evidence"].update({"independent_review": True}),
            lambda value: value["blockers"].pop(),
        )
        for mutate in mutations:
            changed = copy.deepcopy(report())
            mutate(changed)
            self.assertTrue(evidence.validate_report(changed, verify_current=False))

    def test_command_bom_security_and_contract_mutations_fail(self) -> None:
        mutations = (
            lambda value: value["commands"][0].update({"exit_code": 1}),
            lambda value: value["commands"].pop(),
            lambda value: value["security_requirement_ids"].pop(),
            lambda value: value["grammar_bom"]["grammars"].pop(),
            lambda value: value["grammar_bom"].update({"grammar_set_sha256": "bad"}),
            lambda value: value["implemented_contracts"].update({"network_authority": True}),
            lambda value: value["implemented_contracts"].update(
                {"excluded_content_admitted": True}
            ),
        )
        for mutate in mutations:
            changed = copy.deepcopy(report())
            mutate(changed)
            self.assertTrue(evidence.validate_report(changed, verify_current=False))


if __name__ == "__main__":
    unittest.main()
