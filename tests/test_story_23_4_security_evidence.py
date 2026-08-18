from __future__ import annotations

import copy
import json
import unittest

from scripts.story_23_4_security_evidence import (
    COMMANDS,
    DEMONSTRATED,
    MAPPINGS,
    NOT_APPLICABLE,
    PARTIAL,
    REPORT_PATH,
    SOURCE_PATHS,
    read_report,
    validate_mapping_shape,
    validate_report,
)


class Story234SecurityEvidenceTests(unittest.TestCase):
    def test_mapping_is_closed_complete_for_declared_scope_and_path_bound(self) -> None:
        self.assertEqual(validate_mapping_shape(), [])
        self.assertEqual(validate_mapping_shape("HEAD"), [])
        self.assertEqual(len(MAPPINGS), 40)
        self.assertEqual(len(set(MAPPINGS)), len(MAPPINGS))
        self.assertTrue({"RV-05", "RV-17", "RV-18", "RV-20"} <= set(MAPPINGS))
        self.assertTrue(
            {"SR-ACC", "SR-AI", "SR-DAT", "SR-OPS", "SR-TST"}
            <= {"-".join(requirement.split("-")[:2]) for requirement in MAPPINGS}
        )
        available = set(SOURCE_PATHS)
        self.assertTrue(
            all(path in available for record in MAPPINGS.values() for path in record["evidence"])
        )

    def test_statuses_preserve_partial_and_ephemeral_boundaries(self) -> None:
        statuses = [record["status"] for record in MAPPINGS.values()]
        self.assertGreater(statuses.count(DEMONSTRATED), 0)
        self.assertGreater(statuses.count(PARTIAL), 0)
        self.assertEqual(statuses.count(NOT_APPLICABLE), 2)
        self.assertEqual(MAPPINGS["RV-05"]["status"], PARTIAL)
        self.assertEqual(MAPPINGS["RV-20"]["status"], PARTIAL)
        self.assertIn("not occurred", MAPPINGS["SR-TST-011"]["remaining"])

    def test_commands_are_unique_bounded_and_do_not_use_network_tools(self) -> None:
        self.assertEqual(len(COMMANDS), 7)
        self.assertEqual(len({identity for identity, _, _ in COMMANDS}), len(COMMANDS))
        forbidden = {"curl", "wget", "ssh", "scp", "nc"}
        self.assertTrue(
            all(command[0] not in forbidden for _, command, _ in COMMANDS)
        )

    @unittest.skipUnless(REPORT_PATH.is_file(), "retained report is generated after source commit")
    def test_current_report_is_exact_and_hash_bound(self) -> None:
        report = read_report()
        self.assertEqual(validate_report(report), [])

        completed = copy.deepcopy(report)
        completed["summary"]["complete_story_security_evidence"] = True
        self.assertTrue(validate_report(completed))

        omitted = copy.deepcopy(report)
        omitted["mappings"].pop()
        self.assertTrue(validate_report(omitted))

        digest = copy.deepcopy(report)
        digest["raw_trace"]["sha256"] = "f" * 64
        self.assertTrue(validate_report(digest))

        self.assertEqual(json.loads(json.dumps(report)), report)


if __name__ == "__main__":
    unittest.main()
