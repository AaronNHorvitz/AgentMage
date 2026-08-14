from __future__ import annotations

import copy
import unittest

from scripts.planning_scope import (
    EXPECTED_NEGATIVE_CONTROLS,
    ROOT,
    load_object,
    validate_planning_scope,
)


class PlanningScopeTests(unittest.TestCase):
    def setUp(self) -> None:
        self.manifest = load_object(
            ROOT / "requirements" / "planning-scope-decisions.json"
        )
        self.registry = load_object(ROOT / "requirements" / "registry.json")
        self.normative = load_object(ROOT / "requirements" / "normative-map.json")
        self.baseline = load_object(
            ROOT / "requirements" / "additions-only-baseline.json"
        )
        self.traceability = load_object(
            ROOT / "requirements" / "traceability-report.json"
        )
        self.status = load_object(ROOT / "architecture" / "status-model.json")
        self.tasks = (ROOT / "TASKS.md").read_text(encoding="utf-8")

    def validate(
        self,
        *,
        manifest: dict | None = None,
        registry: dict | None = None,
        normative: dict | None = None,
        baseline: dict | None = None,
        traceability: dict | None = None,
        status: dict | None = None,
        tasks: str | None = None,
    ) -> tuple[list[str], dict]:
        return validate_planning_scope(
            self.manifest if manifest is None else manifest,
            self.registry if registry is None else registry,
            self.normative if normative is None else normative,
            self.baseline if baseline is None else baseline,
            self.traceability if traceability is None else traceability,
            self.status if status is None else status,
            self.tasks if tasks is None else tasks,
        )

    def test_accepted_decision_chain_reconciles_historical_and_current_truth(self) -> None:
        failures, report = self.validate()

        self.assertEqual(failures, [])
        self.assertEqual(report["accepted_decisions"], ["ADR-0027", "ADR-0040"])
        self.assertEqual(
            report["historical_baseline"],
            {"stable_requirements": 241, "normative_mappings": 30},
        )
        self.assertEqual(
            report["current_counts"],
            {
                "stable_requirements": 241,
                "normative_mappings": 31,
                "epics": 17,
                "sprints": 169,
            },
        )
        self.assertEqual(len(report["appended_requirement_ids"]), 12)
        self.assertEqual(len(report["preserved_requirement_ids"]), 229)
        self.assertEqual(
            tuple(report["required_negative_controls"]), EXPECTED_NEGATIVE_CONTROLS
        )

    def test_preserved_requirement_mutation_is_rejected(self) -> None:
        changed = copy.deepcopy(self.registry)
        changed["requirements"][0]["title"] += " changed"

        failures, _ = self.validate(registry=changed)

        self.assertTrue(any("weakened_or_changed_requirement" in item for item in failures))

    def test_preserved_requirement_deletion_is_rejected(self) -> None:
        changed = copy.deepcopy(self.registry)
        removed = changed["requirements"].pop(0)["id"]
        changed["counts"]["total"] -= 1

        failures, _ = self.validate(registry=changed)

        self.assertTrue(any(removed in item for item in failures))

    def test_preserved_requirement_duplication_is_rejected(self) -> None:
        changed = copy.deepcopy(self.registry)
        duplicate = copy.deepcopy(changed["requirements"][0])
        changed["requirements"].insert(1, duplicate)
        changed["counts"]["total"] += 1

        failures, _ = self.validate(registry=changed)

        self.assertTrue(any("duplicate ids" in item for item in failures))

    def test_preserved_requirement_reordering_is_rejected(self) -> None:
        changed = copy.deepcopy(self.registry)
        changed["requirements"][0], changed["requirements"][1] = (
            changed["requirements"][1],
            changed["requirements"][0],
        )

        failures, _ = self.validate(registry=changed)

        self.assertTrue(any("reordered" in item for item in failures))

    def test_preserved_requirement_renumbering_is_rejected(self) -> None:
        changed = copy.deepcopy(self.registry)
        old_id = changed["requirements"][0]["id"]
        changed["requirements"][0]["id"] = "AM-ZZZ-999"
        changed["requirements"] = sorted(changed["requirements"], key=lambda item: item["id"])

        failures, _ = self.validate(registry=changed)

        self.assertTrue(any(old_id in item and "missing" in item for item in failures))

    def test_unapproved_addition_is_rejected_even_when_reported_count_is_edited(self) -> None:
        changed = copy.deepcopy(self.registry)
        added = copy.deepcopy(changed["requirements"][-1])
        added["id"] = "AT-ZZZ-999"
        changed["requirements"].append(added)
        changed["counts"]["total"] = 242

        failures, _ = self.validate(registry=changed)

        self.assertTrue(any("AT-ZZZ-999" in item and "unapproved" in item for item in failures))

    def test_missing_decision_approval_is_rejected_precisely(self) -> None:
        changed = copy.deepcopy(self.manifest)
        changed["decisions"][0]["status"] = "proposed"

        failures, _ = self.validate(manifest=changed)

        self.assertTrue(any("ADR-0027: decision approval" in item for item in failures))

    def test_corrupt_appended_id_set_is_rejected_precisely(self) -> None:
        changed = copy.deepcopy(self.manifest)
        changed["decisions"][0]["appended_requirement_ids"].pop()

        failures, _ = self.validate(manifest=changed)

        self.assertTrue(any("appended requirement id set" in item for item in failures))

    def test_corrupt_status_count_is_rejected_precisely(self) -> None:
        changed = copy.deepcopy(self.status)
        changed["scope_control"]["stable_requirements"] = 240

        failures, _ = self.validate(status=changed)

        self.assertTrue(any("status count" in item for item in failures))

    def test_corrupt_supersession_boundary_is_rejected_precisely(self) -> None:
        changed = copy.deepcopy(self.manifest)
        changed["model_direction"]["supersession_markers"][0] = "missing-boundary"

        failures, _ = self.validate(manifest=changed)

        self.assertTrue(any("model supersession marker" in item for item in failures))

    def test_corrupt_generated_registry_linkage_is_rejected_precisely(self) -> None:
        changed = copy.deepcopy(self.traceability)
        changed["requirements"].pop()
        changed["counts"]["total"] -= 1

        failures, _ = self.validate(traceability=changed)

        self.assertTrue(any("generated registry linkage" in item for item in failures))

    def test_normative_reconciliation_cannot_pass_by_editing_only_a_count(self) -> None:
        changed = copy.deepcopy(self.manifest)
        changed["snapshots"]["post-0027"]["counts"]["normative_mappings"] = 31

        failures, _ = self.validate(manifest=changed)

        self.assertTrue(any("post-0027 snapshot counts changed" in item for item in failures))

    def test_source_hash_change_is_rejected_with_exact_path(self) -> None:
        changed = copy.deepcopy(self.manifest)
        changed["current_source_hashes"]["requirements/registry.json"] = "0" * 64

        failures, _ = self.validate(manifest=changed)

        self.assertTrue(
            any("current source hash changed for requirements/registry.json" in item for item in failures)
        )


if __name__ == "__main__":
    unittest.main()
