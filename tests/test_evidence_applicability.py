from __future__ import annotations

import copy
import json
import subprocess
import tempfile
import unittest
from pathlib import Path

from scripts.evidence_applicability import (
    CATALOG_PATH,
    CatalogError,
    build_report,
    evaluate_catalog,
    validate_catalog_shape,
)
from scripts.evidence_core import git_source_identity, sha256_bytes


class EvidenceApplicabilityTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary = tempfile.TemporaryDirectory()
        self.root = Path(self.temporary.name)
        subprocess.run(["git", "init", "-q"], cwd=self.root, check=True)
        subprocess.run(["git", "config", "user.name", "Fixture"], cwd=self.root, check=True)
        subprocess.run(
            ["git", "config", "user.email", "fixture@example.invalid"],
            cwd=self.root,
            check=True,
        )
        (self.root / "artifacts").mkdir()
        (self.root / "source").mkdir()
        self.artifact_path = "artifacts/gate.json"
        self.source_path = "source/owner.txt"
        self.signature_path = "source/review.sig"
        (self.root / self.artifact_path).write_text(
            json.dumps({"acceptance_test_id": "AT-AUTH-001"}) + "\n",
            encoding="utf-8",
        )
        (self.root / self.source_path).write_text("owner-v1\n", encoding="utf-8")
        (self.root / self.signature_path).write_text("signature-v1\n", encoding="utf-8")
        subprocess.run(["git", "add", "."], cwd=self.root, check=True)
        subprocess.run(["git", "commit", "-qm", "evidence"], cwd=self.root, check=True)
        identity = git_source_identity(self.root, "HEAD")
        self.record = {
            "applicability_mode": "evaluate-current",
            "artifact": {
                "path": self.artifact_path,
                "revision": identity["revision"],
                "sha256": sha256_bytes((self.root / self.artifact_path).read_bytes()),
                "tree": identity["tree"],
            },
            "claims": ["AT-AUTH-001"],
            "dependencies": [],
            "disposition": "accepted",
            "evidence_id": "evidence-a",
            "owned_inputs": [
                {
                    "path": self.source_path,
                    "sha256": sha256_bytes((self.root / self.source_path).read_bytes()),
                }
            ],
            "platform_lanes": ["shared"],
            "signatures": [
                {
                    "path": self.signature_path,
                    "sha256": sha256_bytes((self.root / self.signature_path).read_bytes()),
                }
            ],
            "supersedes": [],
        }

    def tearDown(self) -> None:
        self.temporary.cleanup()

    def catalog(self, *records: dict[str, object]) -> dict[str, object]:
        return {
            "catalog_id": "agentmage-evidence-catalog-v1",
            "records": list(records or (self.record,)),
            "schema_version": 1,
            "selected_release": "v0.1",
        }

    def test_current_record_is_historically_valid_and_current(self) -> None:
        report = evaluate_catalog(self.catalog(), self.root)
        record = report["records"][0]
        self.assertEqual(record["historical_validity"], "valid")
        self.assertEqual(record["current_applicability"], "current")

    def test_later_policy_or_owner_change_preserves_history_but_marks_current_stale(self) -> None:
        (self.root / self.source_path).write_text("owner-v2\n", encoding="utf-8")
        report = evaluate_catalog(self.catalog(), self.root)
        record = report["records"][0]
        self.assertEqual(record["historical_validity"], "valid")
        self.assertEqual(record["current_applicability"], "stale")

    def test_unrelated_change_does_not_change_applicability(self) -> None:
        (self.root / "unrelated.txt").write_text("later\n", encoding="utf-8")
        self.assertEqual(
            evaluate_catalog(self.catalog(), self.root)["records"][0]["current_applicability"],
            "current",
        )

    def test_signature_mutation_is_current_staleness_not_historical_corruption(self) -> None:
        (self.root / self.signature_path).write_text("signature-v2\n", encoding="utf-8")
        record = evaluate_catalog(self.catalog(), self.root)["records"][0]
        self.assertEqual(record["historical_validity"], "valid")
        self.assertFalse(record["signatures_current"])
        self.assertEqual(record["current_applicability"], "stale")

    def test_staleness_propagates_to_transitive_dependency(self) -> None:
        dependent = copy.deepcopy(self.record)
        dependent["evidence_id"] = "evidence-b"
        dependent["dependencies"] = ["evidence-a"]
        dependent["claims"] = ["AT-CFG-001"]
        dependent_source = self.root / "source/dependent.txt"
        dependent_source.write_text("dependent-v1\n", encoding="utf-8")
        dependent["owned_inputs"] = [
            {
                "path": "source/dependent.txt",
                "sha256": sha256_bytes(dependent_source.read_bytes()),
            }
        ]
        artifact = self.root / self.artifact_path
        artifact.write_text(
            json.dumps(
                {
                    "acceptance_test_id": "AT-AUTH-001",
                    "requirement_id": "AT-CFG-001",
                }
            )
            + "\n",
            encoding="utf-8",
        )
        subprocess.run(["git", "add", "."], cwd=self.root, check=True)
        subprocess.run(["git", "commit", "-qm", "dependent"], cwd=self.root, check=True)
        identity = git_source_identity(self.root, "HEAD")
        for record in (self.record, dependent):
            record["artifact"] = {
                "path": self.artifact_path,
                "revision": identity["revision"],
                "sha256": sha256_bytes(artifact.read_bytes()),
                "tree": identity["tree"],
            }
            record["signatures"][0]["sha256"] = sha256_bytes(
                (self.root / self.signature_path).read_bytes()
            )
        self.record["owned_inputs"][0]["sha256"] = sha256_bytes(
            (self.root / self.source_path).read_bytes()
        )
        dependent["artifact"] = dict(dependent["artifact"])
        dependent["artifact"]["path"] = "artifacts/dependent.json"
        (self.root / "artifacts/dependent.json").write_bytes(artifact.read_bytes())
        subprocess.run(["git", "add", "."], cwd=self.root, check=True)
        subprocess.run(["git", "commit", "-qm", "dependent artifact"], cwd=self.root, check=True)
        identity = git_source_identity(self.root, "HEAD")
        for record in (self.record, dependent):
            record["artifact"]["revision"] = identity["revision"]
            record["artifact"]["tree"] = identity["tree"]
        (self.root / self.source_path).write_text("changed\n", encoding="utf-8")
        report = evaluate_catalog(self.catalog(self.record, dependent), self.root)
        by_id = {record["evidence_id"]: record for record in report["records"]}
        self.assertEqual(by_id["evidence-a"]["current_applicability"], "stale")
        self.assertEqual(by_id["evidence-b"]["current_applicability"], "stale")
        self.assertEqual(by_id["evidence-b"]["stale_via"], ["evidence-a"])

    def test_explicit_supersession_keeps_both_records_and_selects_new(self) -> None:
        old = copy.deepcopy(self.record)
        old["evidence_id"] = "evidence-a"
        new = copy.deepcopy(self.record)
        new["evidence_id"] = "evidence-b"
        new["supersedes"] = ["evidence-a"]
        old["artifact"] = dict(old["artifact"])
        old["artifact"]["path"] = "artifacts/old.json"
        (self.root / "artifacts/old.json").write_bytes(
            (self.root / self.artifact_path).read_bytes()
        )
        subprocess.run(["git", "add", "."], cwd=self.root, check=True)
        subprocess.run(["git", "commit", "-qm", "supersession"], cwd=self.root, check=True)
        identity = git_source_identity(self.root, "HEAD")
        for record in (old, new):
            record["artifact"]["revision"] = identity["revision"]
            record["artifact"]["tree"] = identity["tree"]
        report = evaluate_catalog(self.catalog(old, new), self.root)
        by_id = {record["evidence_id"]: record for record in report["records"]}
        self.assertEqual(by_id["evidence-a"]["current_applicability"], "superseded")
        self.assertEqual(by_id["evidence-b"]["current_applicability"], "current")

    def test_wrong_claim_missing_source_and_cycles_fail_closed(self) -> None:
        wrong = copy.deepcopy(self.record)
        wrong["claims"] = ["AT-WRONG-001"]
        self.assertEqual(
            evaluate_catalog(self.catalog(wrong), self.root)["records"][0][
                "historical_validity"
            ],
            "invalid",
        )
        missing = copy.deepcopy(self.record)
        missing["artifact"]["revision"] = "f" * 40
        self.assertEqual(
            evaluate_catalog(self.catalog(missing), self.root)["records"][0][
                "historical_validity"
            ],
            "invalid",
        )
        cycle_a = copy.deepcopy(self.record)
        cycle_b = copy.deepcopy(self.record)
        cycle_a["evidence_id"] = "evidence-a"
        cycle_b["evidence_id"] = "evidence-b"
        cycle_a["dependencies"] = ["evidence-b"]
        cycle_b["dependencies"] = ["evidence-a"]
        cycle_b["artifact"] = dict(cycle_b["artifact"])
        cycle_b["artifact"]["path"] = "artifacts/other.json"
        with self.assertRaisesRegex(CatalogError, "dependencies_cycle"):
            evaluate_catalog(self.catalog(cycle_a, cycle_b), self.root)

    def test_repository_catalog_replays_and_discovers_auth_artifact(self) -> None:
        report = build_report()
        by_id = {record["evidence_id"]: record for record in report["records"]}
        auth = by_id["evidence-at-auth-001-sprint-5"]
        self.assertEqual(auth["historical_validity"], "valid")
        self.assertIn("AT-AUTH-001", auth["claims"])
        self.assertEqual(
            auth["artifact"]["path"],
            "artifacts/sprints/sprint-5/sprint-gate-report.json",
        )


if __name__ == "__main__":
    unittest.main()
