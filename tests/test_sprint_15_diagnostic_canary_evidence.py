from __future__ import annotations

import copy
import unittest
from unittest.mock import patch

from scripts.sprint_15_diagnostic_canary_evidence import (
    COMMANDS,
    COVERAGE,
    SOURCE_FAMILIES,
    SOURCE_PATHS,
    build_report,
    expected_command_records,
    validate_report,
)


def command_results() -> list[dict[str, object]]:
    return [
        {
            **record,
            "elapsed_ms": 1,
            "exit_code": 0,
            "output_bytes": 1,
            "output_sha256": "a" * 64,
            "canary_match_count": 0,
        }
        for record in expected_command_records()
    ]


class Sprint15DiagnosticCanaryEvidenceTests(unittest.TestCase):
    def report(self) -> dict[str, object]:
        sources = [
            {"path": path, "bytes": 1, "sha256": "c" * 64}
            for path in SOURCE_PATHS
        ]
        with patch(
            "scripts.sprint_15_diagnostic_canary_evidence.source_records",
            return_value=sources,
        ):
            return build_report("b" * 40, b"log", command_results())

    def test_scope_names_every_source_and_output_surface(self) -> None:
        self.assertEqual(len(SOURCE_FAMILIES), 9)
        self.assertEqual(len(COMMANDS), 3)
        self.assertEqual(
            set(COVERAGE),
            {
                "internal-harness",
                "native-chat",
                "authenticated-host-transport",
                "command-output-log",
                "preview",
                "receipt",
                "exported-diagnostics",
            },
        )
        self.assertEqual(
            {identifier for values in COVERAGE.values() for identifier in values},
            {record["id"] for record in expected_command_records()},
        )

    def test_bounded_report_validates_without_retained_files(self) -> None:
        self.assertEqual(
            validate_report(self.report(), verify_sources=False),
            [],
        )

    def test_disclosure_semantic_command_and_scope_mutations_fail(self) -> None:
        mutations = (
            lambda value: value.update({"status": "pass-all-platforms"}),
            lambda value: value["commands"][0].update({"canary_match_count": 1}),
            lambda value: value["commands"][0].update({"command_id": "d" * 64}),
            lambda value: value["semantic_reconciliation"].update(
                {"chat_disclosure_count": 1}
            ),
            lambda value: value.update({"external_network_used": True}),
            lambda value: value["coverage"].pop("native-chat"),
            lambda value: value["limitations"].clear(),
        )
        for mutate in mutations:
            changed = copy.deepcopy(self.report())
            mutate(changed)
            self.assertTrue(validate_report(changed, verify_sources=False))


if __name__ == "__main__":
    unittest.main()
