from __future__ import annotations

import copy
import unittest

from scripts.fixture_provenance import (
    CONTRACT_SPECS,
    LEDGER_PATH,
    ROOT,
    SOURCE_ORDER,
    build_ledger,
    build_report,
    canonical_json,
    check_artifacts,
    expected_root_fixture_contracts,
    read_json,
    root_fixture_contracts,
    sha256_bytes,
    sha256_file,
    validate_ledger,
    validate_report,
)
from scripts.platform_result_recorder import validate_record as validate_platform_record


class FixtureProvenanceTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.ledger = build_ledger()

    def mutated(self) -> dict:
        return copy.deepcopy(self.ledger)

    @staticmethod
    def rehash(ledger: dict) -> None:
        ledger.pop("ledger_sha256", None)
        ledger["ledger_sha256"] = sha256_bytes(canonical_json(ledger))

    def test_checked_ledger_and_report_are_current(self) -> None:
        self.assertEqual(check_artifacts(), [])
        checked = read_json(LEDGER_PATH)
        self.assertEqual(checked, self.ledger)
        self.assertEqual(validate_ledger(checked), [])

        report = build_report()
        self.assertEqual(validate_report(report), [])
        self.assertEqual(report["task_id"], "2.1.2.3")
        self.assertEqual(report["status"], "pass")

    def test_generation_is_deterministic_and_self_hashed(self) -> None:
        self.assertEqual(build_ledger(), build_ledger())
        unhashed = dict(self.ledger)
        recorded = unhashed.pop("ledger_sha256")
        self.assertEqual(recorded, sha256_bytes(canonical_json(unhashed)))

    def test_every_lineage_artifact_exists_and_matches_its_hash(self) -> None:
        for node in self.ledger["lineage"]["nodes"]:
            path = ROOT / node["path"]
            with self.subTest(node=node["node_id"]):
                self.assertTrue(path.is_file())
                self.assertEqual(node["sha256"], sha256_file(path))

    def test_root_fixture_contract_closure_is_exact(self) -> None:
        manifest = read_json(ROOT / "fixtures/corpus/v1/manifest.json")
        actual = root_fixture_contracts(ROOT)
        expected = expected_root_fixture_contracts(manifest)
        self.assertEqual(actual, expected)
        self.assertEqual(len(actual), self.ledger["scope"]["root_fixture_contract_count"])

    def test_source_contract_and_golden_closure_is_exact(self) -> None:
        self.assertEqual(
            [item["family"] for item in self.ledger["source_families"]],
            list(SOURCE_ORDER),
        )
        self.assertEqual(
            [item["contract_id"] for item in self.ledger["contracts"]],
            [item["contract_id"] for item in CONTRACT_SPECS],
        )
        self.assertEqual(
            {item["evidence_state"] for item in self.ledger["golden_manifests"]},
            {"Observed", "Derived", "Inferred", "Unknown/Blocked"},
        )

    def test_platform_records_remain_allowlisted_and_self_hashed(self) -> None:
        report = read_json(
            ROOT
            / "artifacts/sprints/sprint-2/story-2.1/"
            "platform-result-recorder-report.json"
        )
        for record in report["synthetic_record_set"]["records"]:
            self.assertEqual(validate_platform_record(record), [])
            self.assertNotIn("hostname", canonical_json(record).decode("utf-8"))
            self.assertEqual(record["macos_support_claim"], "none")

    def test_changed_artifact_hash_is_rejected(self) -> None:
        ledger = self.mutated()
        ledger["lineage"]["nodes"][0]["sha256"] = "0" * 64
        self.rehash(ledger)
        failures = validate_ledger(ledger)
        self.assertTrue(any("artifact hash" in failure for failure in failures))

    def test_missing_and_duplicate_sources_are_rejected(self) -> None:
        missing = self.mutated()
        missing["source_families"].pop()
        self.rehash(missing)
        self.assertTrue(
            any("source-family closure" in failure for failure in validate_ledger(missing))
        )

        duplicate = self.mutated()
        duplicate["source_families"][1]["family"] = "base"
        self.rehash(duplicate)
        self.assertTrue(
            any("source-family closure" in failure for failure in validate_ledger(duplicate))
        )

    def test_duplicate_nodes_paths_and_edges_are_rejected(self) -> None:
        ledger = self.mutated()
        duplicate = copy.deepcopy(ledger["lineage"]["nodes"][0])
        ledger["lineage"]["nodes"].append(duplicate)
        ledger["lineage"]["edges"].append(
            copy.deepcopy(ledger["lineage"]["edges"][0])
        )
        self.rehash(ledger)
        failures = validate_ledger(ledger)
        self.assertTrue(any("duplicate node ids" in failure for failure in failures))
        self.assertTrue(any("duplicate artifact paths" in failure for failure in failures))
        self.assertTrue(any("duplicate edge ids" in failure for failure in failures))

    def test_dangling_edges_and_unknown_relations_are_rejected(self) -> None:
        ledger = self.mutated()
        ledger["lineage"]["edges"][0]["to_node"] = "missing-node"
        ledger["lineage"]["edges"][1]["relation"] = "trusts-without-proof"
        self.rehash(ledger)
        failures = validate_ledger(ledger)
        self.assertTrue(any("edge is dangling" in failure for failure in failures))
        self.assertTrue(any("relation is invalid" in failure for failure in failures))

    def test_lineage_cycles_are_rejected(self) -> None:
        ledger = self.mutated()
        ledger["lineage"]["edges"].append(
            {
                "edge_id": "edge-cycle-fixture",
                "from_node": "archive-corpus",
                "to_node": "generator-base",
                "relation": "verifies",
            }
        )
        self.rehash(ledger)
        self.assertTrue(
            any("contains a cycle" in failure for failure in validate_ledger(ledger))
        )

    def test_private_paths_and_secret_shaped_values_are_rejected(self) -> None:
        private = self.mutated()
        private["lineage"]["nodes"][0]["path"] = "/home/person/private.txt"
        self.rehash(private)
        failures = validate_ledger(private)
        self.assertTrue(any("path is invalid" in failure for failure in failures))
        self.assertTrue(any("contains a private path" in failure for failure in failures))

        secret = self.mutated()
        secret["corpus"]["corpus_id"] = "ghp_" + ("A" * 32)
        self.rehash(secret)
        self.assertTrue(
            any("secret-shaped" in failure for failure in validate_ledger(secret))
        )

    def test_weakened_controls_and_macos_claims_are_rejected(self) -> None:
        ledger = self.mutated()
        ledger["controls"]["acyclic"] = False
        ledger["macos_execution_status"] = "pass"
        ledger["macos_support_claim"] = "supported"
        self.rehash(ledger)
        failures = validate_ledger(ledger)
        self.assertTrue(any("controls were weakened" in failure for failure in failures))
        self.assertTrue(any("invalid macOS claim" in failure for failure in failures))


if __name__ == "__main__":
    unittest.main()
