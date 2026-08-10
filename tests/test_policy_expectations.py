from __future__ import annotations

import io
import json
import tempfile
import unittest
from contextlib import redirect_stderr
from pathlib import Path

from scripts.policy_expectations import (
    DEFAULT_INVENTORY,
    DEFAULT_OUTPUT,
    DEFAULT_PRD,
    EXPECTED_COUNTS,
    PolicyExpectationError,
    build_policy_registry,
    check_policy_registry,
    render_policy_registry,
    write_policy_registry,
)


class PolicyExpectationTests(unittest.TestCase):
    def test_covers_every_canonical_policy_class(self) -> None:
        registry = build_policy_registry()

        self.assertEqual(registry["schema_version"], 1)
        self.assertEqual(registry["counts"]["by_kind"], EXPECTED_COUNTS)
        self.assertEqual(registry["counts"]["total"], 73)
        self.assertEqual(len(registry["expectations"]), 73)
        self.assertEqual(
            [source["document"] for source in registry["sources"]],
            ["PRD.md", "Agent-Scaffolding-Inventory.md"],
        )
        for source in registry["sources"]:
            self.assertRegex(source["sha256"], r"^[a-f0-9]{64}$")
        self.assertEqual(
            len({record["id"] for record in registry["expectations"]}),
            73,
        )

    def test_every_expectation_is_source_pinned_and_executable(self) -> None:
        registry = build_policy_registry()
        contract = registry["test_contract"]

        self.assertEqual(contract["id"], "PX-TEST-001")
        self.assertEqual(
            set(contract["allowed_results"]),
            {"absent", "disabled", "denied_before_effect"},
        )
        self.assertIn("state_or_external_side_effect", contract["prohibited_results"])
        for record in registry["expectations"]:
            self.assertEqual(record["test_contract_id"], "PX-TEST-001")
            self.assertEqual(record["status"], "enforced_policy")
            self.assertIn(record["expected_result"], {
                "absent_or_denied_in_v0.1",
                "denied_by_default",
                "disabled_before_target_release",
                "disabled_until_formally_promoted",
            })
            self.assertRegex(record["source"]["statement_sha256"], r"^[a-f0-9]{64}$")
            self.assertGreater(record["source"]["line"], 0)

    def test_committed_registry_matches_canonical_sources(self) -> None:
        generated = build_policy_registry()
        committed = json.loads(DEFAULT_OUTPUT.read_text(encoding="utf-8"))

        self.assertEqual(committed, generated)
        self.assertTrue(check_policy_registry(DEFAULT_PRD, DEFAULT_INVENTORY, DEFAULT_OUTPUT))

    def test_rendering_is_byte_deterministic_and_sources_are_not_mutated(self) -> None:
        before = (DEFAULT_PRD.read_bytes(), DEFAULT_INVENTORY.read_bytes())
        first = render_policy_registry(build_policy_registry())
        second = render_policy_registry(build_policy_registry())

        self.assertEqual(first, second)
        self.assertEqual(before, (DEFAULT_PRD.read_bytes(), DEFAULT_INVENTORY.read_bytes()))

    def test_check_mode_reports_missing_and_stale_without_writing(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            output = Path(temp_dir) / "policy-expectations.json"
            with redirect_stderr(io.StringIO()):
                self.assertFalse(check_policy_registry(DEFAULT_PRD, DEFAULT_INVENTORY, output))
            self.assertFalse(output.exists())

            output.write_text("{}\n", encoding="utf-8")
            stale = output.read_bytes()
            with redirect_stderr(io.StringIO()):
                self.assertFalse(check_policy_registry(DEFAULT_PRD, DEFAULT_INVENTORY, output))
            self.assertEqual(output.read_bytes(), stale)

            write_policy_registry(DEFAULT_PRD, DEFAULT_INVENTORY, output)
            self.assertTrue(check_policy_registry(DEFAULT_PRD, DEFAULT_INVENTORY, output))

    def test_new_non_goal_requires_a_new_policy_expectation(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            prd = Path(temp_dir) / "PRD.md"
            source = DEFAULT_PRD.read_text(encoding="utf-8")
            marker = "## 5. Product Architecture"
            prd.write_text(
                source.replace(
                    marker,
                    "- A newly excluded fixture.\n\n" + marker,
                    1,
                ),
                encoding="utf-8",
            )

            with self.assertRaisesRegex(PolicyExpectationError, "counts changed"):
                build_policy_registry(prd, DEFAULT_INVENTORY)

    def test_removed_or_reclassified_defer_item_blocks_generation(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            inventory = Path(temp_dir) / "Agent-Scaffolding-Inventory.md"
            source = DEFAULT_INVENTORY.read_text(encoding="utf-8")
            inventory.write_text(
                source.replace("- [ ] `DEFER`", "- [ ] `BUILD`", 1),
                encoding="utf-8",
            )

            with self.assertRaisesRegex(PolicyExpectationError, "counts changed"):
                build_policy_registry(DEFAULT_PRD, inventory)

    def test_removed_rejected_default_blocks_generation(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            inventory = Path(temp_dir) / "Agent-Scaffolding-Inventory.md"
            source = DEFAULT_INVENTORY.read_text(encoding="utf-8")
            rejected = (
                '- [ ] `CAPABILITY GATE` Reject unrestricted, "yolo," blanket, '
                "wildcard, or approve-everything tool execution in every interface "
                "and operating mode."
            )
            inventory.write_text(source.replace(rejected, "", 1), encoding="utf-8")

            with self.assertRaisesRegex(PolicyExpectationError, "counts changed"):
                build_policy_registry(DEFAULT_PRD, inventory)


if __name__ == "__main__":
    unittest.main()
