from __future__ import annotations

import copy
import io
import tempfile
import unittest
from contextlib import redirect_stderr
from pathlib import Path

from scripts.additions_only import (
    DEFAULT_BASELINE,
    DEFAULT_INVENTORY,
    DEFAULT_REGISTRY,
    AdditionsOnlyError,
    audit_additions_only,
    build_initial_baseline,
    load_json,
    parse_checklist,
    render,
    update_baseline,
)
from scripts.policy_expectations import (
    DEFAULT_OUTPUT as DEFAULT_POLICY_OUTPUT,
    DEFAULT_PRD,
    PolicyExpectationError,
    build_policy_registry,
    check_policy_registry,
)


class AdditionsOnlyTests(unittest.TestCase):
    def setUp(self) -> None:
        self.registry = load_json(DEFAULT_REGISTRY)
        self.baseline = load_json(DEFAULT_BASELINE)
        self.protected_ids = {
            record["id"] for record in self.baseline["requirements"]
        }

    def protected_record(self, registry: dict[str, object]) -> dict[str, object]:
        return next(
            record
            for record in registry["requirements"]
            if record["id"] in self.protected_ids
        )

    def categories(self, diagnostics: list[object]) -> list[str]:
        return [diagnostic.category for diagnostic in diagnostics]

    def test_committed_baseline_protects_every_current_entry(self) -> None:
        diagnostics = audit_additions_only(
            self.baseline,
            self.registry,
            DEFAULT_INVENTORY,
        )

        self.assertEqual(diagnostics, [])
        self.assertEqual(
            self.baseline["counts"]["requirements"],
            len(self.baseline["requirements"]),
        )
        self.assertEqual(
            self.baseline["counts"]["checklist"],
            len(self.baseline["checklist"]),
        )

    def test_initial_baseline_is_deterministic(self) -> None:
        first = build_initial_baseline(self.registry, DEFAULT_INVENTORY)
        second = build_initial_baseline(self.registry, DEFAULT_INVENTORY)

        self.assertEqual(render(first), render(second))
        normalized_baseline = copy.deepcopy(self.baseline)
        for record in first["checklist"]:
            record["source"].pop("line", None)
        for record in normalized_baseline["checklist"]:
            record["source"].pop("line", None)
        self.assertEqual(first, normalized_baseline)

    def test_removed_requirement_is_blocked(self) -> None:
        mutated = copy.deepcopy(self.registry)
        removed = self.protected_record(mutated)
        mutated["requirements"].remove(removed)

        diagnostics = audit_additions_only(
            self.baseline,
            mutated,
            DEFAULT_INVENTORY,
        )

        self.assertIn("removed_requirement", self.categories(diagnostics))
        self.assertTrue(any(item.location == removed["id"] for item in diagnostics))

    def test_changed_title_release_kind_or_disposition_is_blocked(self) -> None:
        for field, replacement in (
            ("title", "A shorter and weaker fixture."),
            ("release", "v9.0"),
            ("kind", "competitive_requirement"),
            ("disposition", "optional"),
        ):
            with self.subTest(field=field):
                mutated = copy.deepcopy(self.registry)
                self.protected_record(mutated)[field] = replacement
                diagnostics = audit_additions_only(
                    self.baseline,
                    mutated,
                    DEFAULT_INVENTORY,
                )
                self.assertIn(
                    "weakened_or_changed_requirement",
                    self.categories(diagnostics),
                )

    def test_s_000_st01_requirement_remove_rename_weaken_and_reclassify_are_exact(self) -> None:
        baseline_id = self.protected_record(self.registry)["id"]
        cases = {
            "remove": lambda records, index: records.pop(index),
            "rename": lambda records, index: records[index].update({"id": "AM-RENAMED-001"}),
            "weaken": lambda records, index: records[index].update(
                {"title": "Optionally perform the protected behavior."}
            ),
            "reclassify": lambda records, index: records[index].update(
                {"kind": "competitive_requirement", "disposition": "optional"}
            ),
        }
        for name, mutate in cases.items():
            with self.subTest(mutation=name):
                changed = copy.deepcopy(self.registry)
                protected_index = next(
                    index
                    for index, record in enumerate(changed["requirements"])
                    if record["id"] == baseline_id
                )
                mutate(changed["requirements"], protected_index)
                diagnostics = audit_additions_only(
                    self.baseline,
                    changed,
                    DEFAULT_INVENTORY,
                )
                self.assertTrue(diagnostics)
                self.assertTrue(
                    any(diagnostic.location == baseline_id for diagnostic in diagnostics)
                )
                self.assertTrue(
                    any(
                        baseline_id in diagnostic.location
                        or baseline_id in diagnostic.message
                        for diagnostic in diagnostics
                    )
                )

    def test_removed_dependency_or_acceptance_test_is_blocked(self) -> None:
        for field in ("dependencies", "acceptance_tests"):
            with self.subTest(field=field):
                mutated = copy.deepcopy(self.registry)
                record = next(
                    item
                    for item in mutated["requirements"]
                    if item["id"] in self.protected_ids and item[field]
                )
                removed = record[field].pop()
                diagnostics = audit_additions_only(
                    self.baseline,
                    mutated,
                    DEFAULT_INVENTORY,
                )
                self.assertIn("weakened_requirement", self.categories(diagnostics))
                self.assertTrue(
                    any(removed in diagnostic.message for diagnostic in diagnostics)
                )

    def test_added_dependencies_tests_and_requirements_are_allowed(self) -> None:
        mutated = copy.deepcopy(self.registry)
        product = next(
            item
            for item in mutated["requirements"]
            if item["id"] in self.protected_ids
            and item["kind"] == "product_requirement"
        )
        product["dependencies"].append("AM-NEW-001")
        product["acceptance_tests"].append("AT-NEW-001")
        added = copy.deepcopy(product)
        added["id"] = "AM-NEW-001"
        added["dependencies"] = []
        added["acceptance_tests"] = ["AT-NEW-001"]
        mutated["requirements"].append(added)

        diagnostics = audit_additions_only(
            self.baseline,
            mutated,
            DEFAULT_INVENTORY,
        )

        self.assertEqual(diagnostics, [])

    def test_update_retains_approved_dependency_and_test_additions(self) -> None:
        mutated = copy.deepcopy(self.registry)
        product = self.protected_record(mutated)
        product["dependencies"].append("AM-NEW-DEPENDENCY-001")
        product["acceptance_tests"].append("AT-NEW-ACCEPTANCE-001")

        updated = update_baseline(self.baseline, mutated, DEFAULT_INVENTORY)
        retained = next(
            record for record in updated["requirements"] if record["id"] == product["id"]
        )

        self.assertEqual(retained["dependencies"], product["dependencies"])
        self.assertEqual(retained["acceptance_tests"], product["acceptance_tests"])
        for field in ("kind", "title", "release", "disposition"):
            self.assertEqual(retained[field], product[field])

    def test_removed_modified_moved_or_reclassified_checklist_entry_is_blocked(self) -> None:
        source = DEFAULT_INVENTORY.read_text(encoding="utf-8")
        original = "- [ ] `BUILD` a five-minute install/start guide."
        moved = source.replace(original, "", 1).replace(
            "## 33. Recommended Build Order",
            "## 33. Recommended Build Order\n\n" + original,
            1,
        )
        mutations = {
            "removed": source.replace(original, "", 1),
            "modified": source.replace(original, original + " Maybe.", 1),
            "moved": moved,
            "reclassified": source.replace(original, original.replace("`BUILD`", "`DEFER`"), 1),
        }
        for name, mutated_source in mutations.items():
            with self.subTest(name=name), tempfile.TemporaryDirectory() as temp_dir:
                inventory = Path(temp_dir) / DEFAULT_INVENTORY.name
                inventory.write_text(mutated_source, encoding="utf-8")
                diagnostics = audit_additions_only(
                    self.baseline,
                    self.registry,
                    inventory,
                )
                self.assertTrue(
                    {
                        "removed_or_modified_checklist_entry",
                        "moved_or_reclassified_checklist_entry",
                    }
                    & set(self.categories(diagnostics))
                )

    def test_s_000_st01_exclusion_mutations_block_both_gates_with_exact_statement(self) -> None:
        source = DEFAULT_INVENTORY.read_text(encoding="utf-8")
        original = next(
            line for line in source.splitlines() if line.startswith("- [ ] `DEFER`")
        )
        statement = original.split("`DEFER`", 1)[1].strip()
        mutations = {
            "remove": source.replace(original, "", 1),
            "rename": source.replace(original, original + " Renamed.", 1),
            "weaken": source.replace(original, original + " Unless convenient.", 1),
            "reclassify": source.replace(
                original,
                original.replace("`DEFER`", "`BUILD`"),
                1,
            ),
        }
        for name, changed_source in mutations.items():
            with self.subTest(mutation=name), tempfile.TemporaryDirectory() as temp_dir:
                inventory = Path(temp_dir) / DEFAULT_INVENTORY.name
                inventory.write_text(changed_source, encoding="utf-8")
                diagnostics = audit_additions_only(
                    self.baseline,
                    self.registry,
                    inventory,
                )
                self.assertTrue(diagnostics)
                self.assertTrue(
                    any(statement in diagnostic.message for diagnostic in diagnostics)
                )

                if name in {"remove", "reclassify"}:
                    with self.assertRaisesRegex(PolicyExpectationError, "counts changed"):
                        build_policy_registry(DEFAULT_PRD, inventory)
                else:
                    with redirect_stderr(io.StringIO()):
                        self.assertFalse(
                            check_policy_registry(
                                DEFAULT_PRD,
                                inventory,
                                DEFAULT_POLICY_OUTPUT,
                            )
                        )

    def test_new_checklist_entry_is_allowed_and_update_only_appends_it(self) -> None:
        source = DEFAULT_INVENTORY.read_text(encoding="utf-8")
        marker = "## 32. Documentation and Operating Guides"
        addition = "- [ ] `BUILD` an additions-only fixture.\n\n"
        with tempfile.TemporaryDirectory() as temp_dir:
            inventory = Path(temp_dir) / DEFAULT_INVENTORY.name
            inventory.write_text(
                source.replace(marker, marker + "\n\n" + addition, 1),
                encoding="utf-8",
            )
            diagnostics = audit_additions_only(
                self.baseline,
                self.registry,
                inventory,
            )
            updated = update_baseline(self.baseline, self.registry, inventory)

        self.assertEqual(diagnostics, [])
        self.assertEqual(
            len(updated["checklist"]),
            len(self.baseline["checklist"]) + 1,
        )
        self.assertEqual(updated["requirements"], self.baseline["requirements"])

    def test_update_refuses_to_hide_a_removal(self) -> None:
        mutated = copy.deepcopy(self.registry)
        mutated["requirements"].remove(self.protected_record(mutated))

        with self.assertRaisesRegex(AdditionsOnlyError, "cannot update"):
            update_baseline(self.baseline, mutated, DEFAULT_INVENTORY)

    def test_check_is_side_effect_free(self) -> None:
        inventory_before = DEFAULT_INVENTORY.read_bytes()
        baseline_before = DEFAULT_BASELINE.read_bytes()
        registry_before = copy.deepcopy(self.registry)
        baseline_value_before = copy.deepcopy(self.baseline)

        audit_additions_only(self.baseline, self.registry, DEFAULT_INVENTORY)

        self.assertEqual(DEFAULT_INVENTORY.read_bytes(), inventory_before)
        self.assertEqual(DEFAULT_BASELINE.read_bytes(), baseline_before)
        self.assertEqual(self.registry, registry_before)
        self.assertEqual(self.baseline, baseline_value_before)

    def test_checkbox_completion_does_not_change_semantic_identity(self) -> None:
        source = DEFAULT_INVENTORY.read_text(encoding="utf-8")
        with tempfile.TemporaryDirectory() as temp_dir:
            inventory = Path(temp_dir) / DEFAULT_INVENTORY.name
            inventory.write_text(source.replace("- [ ]", "- [x]", 1), encoding="utf-8")

            self.assertEqual(
                parse_checklist(inventory),
                parse_checklist(DEFAULT_INVENTORY),
            )


if __name__ == "__main__":
    unittest.main()
