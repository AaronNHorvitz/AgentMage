from __future__ import annotations

import copy
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

from scripts import sprint_16_evidence as evidence
from tests import test_sprint_16_linux_worker_evidence as worker_tests


def commands() -> list[dict[str, object]]:
    return [
        {"id": identifier, "argv": list(argv), "exit_code": 0, "output_sha256": "a" * 64}
        for identifier, argv in evidence.COMMANDS
    ]


def report() -> dict[str, object]:
    with patch.object(evidence, "git_file", return_value=b"source"):
        return evidence.build_report(
            "b" * 40,
            commands(),
            {
                "artifact": "artifacts/sprints/sprint-16/installed-linux-worker-matrix.json",
                "artifact_sha256": "c" * 64,
                "source_revision": "d" * 40,
                "target_ids": ["fedora-44-x86_64", "ubuntu-26.04-x86_64"],
                "verified_operations": evidence.worker_evidence.VERIFIED_OPERATIONS,
                "linux_attack_cases": evidence.worker_evidence.ATTACK_CASES,
                "linux_lifecycle_cases": evidence.worker_evidence.LIFECYCLE_CASES,
            },
        )


class Sprint16EvidenceTests(unittest.TestCase):
    def test_current_report_requires_exact_artifact_and_summary(self) -> None:
        native = worker_tests.Sprint16LinuxWorkerEvidenceTests.current_report()
        encoded = json.dumps(native).encode()
        value = report()
        value["implemented_contracts"]["installed_linux_worker_operation_matrix"] = evidence.installed_worker_summary(native, encoded)
        with tempfile.TemporaryDirectory(prefix="agentmage-local-evidence-") as directory:
            artifact = Path(directory) / "installed.json"
            artifact.write_bytes(encoded)
            with (
                patch.object(evidence, "INSTALLED_WORKER_OUTPUT", artifact),
                patch.object(evidence, "git_file", return_value=b"source"),
                patch.object(evidence.worker_evidence, "source_records", return_value=native["sources"]),
                patch.object(evidence.worker_evidence, "complete_source_closure", return_value=native["complete_source_closure"]),
            ):
                self.assertEqual(evidence.validate_report(value), [])
                changed = copy.deepcopy(value)
                changed["implemented_contracts"]["installed_linux_worker_operation_matrix"]["artifact_sha256"] = "f" * 64
                self.assertIn("installed worker artifact identity or summary drift", evidence.validate_report(changed))
                changed = copy.deepcopy(value)
                changed["source_sha256"].pop(next(iter(changed["source_sha256"])))
                self.assertIn("source inventory drift", evidence.validate_report(changed))
                with patch.object(evidence, "git_file", side_effect=ValueError("missing commit")):
                    self.assertTrue(evidence.validate_report(value))
                artifact.write_bytes(b"not json")
                self.assertTrue(evidence.validate_report(value))
                artifact.write_bytes(json.dumps(dict(native, complete_ten_tool_matrix=False)).encode())
                self.assertTrue(evidence.validate_report(value))

    def test_writer_refuses_stale_native_evidence_before_tests_or_overwrite(self) -> None:
        with (
            patch("sys.argv", ["sprint_16_evidence.py", "--write"]),
            patch.object(evidence.subprocess, "run") as revision,
            patch.object(evidence, "load_installed_worker_summary", side_effect=ValueError("historical")),
            patch.object(evidence, "run_commands") as run,
        ):
            revision.return_value.stdout = "b" * 40
            self.assertEqual(evidence.main(), 1)
            run.assert_not_called()

    def test_current_report_cannot_borrow_an_absent_installed_worker_artifact(self) -> None:
        # Valid local-source bytes and syntactically valid installed-summary
        # claims do not prove that their native source/evidence artifact exists.
        # This is a deterministic developer-gate test, not native qualification.
        value = report()
        with tempfile.TemporaryDirectory(prefix="agentmage-sprint16-freshness-") as directory:
            missing = Path(directory) / "absent-installed-worker.json"
            self.assertFalse(missing.exists())
            with (
                patch.object(evidence, "INSTALLED_WORKER_OUTPUT", missing),
                patch.object(evidence, "git_file", return_value=b"source"),
            ):
                self.assertTrue(
                    evidence.validate_report(value, verify_current=True),
                    "current local/platform claims require their exact installed evidence",
                )

    def test_local_contracts_pass_without_platform_or_release_overclaim(self) -> None:
        value = report()
        self.assertEqual(evidence.validate_report(value, verify_current=False), [])
        self.assertTrue(value["summary"]["local_contract_passed"])
        self.assertEqual(value["summary"]["sprint_status"], "BLOCKED")
        self.assertEqual(value["implemented_contracts"]["catalog_tools"], 10)
        self.assertEqual(value["implemented_contracts"]["write_capable_tools"], 0)
        self.assertTrue(value["implemented_contracts"]["packaged_worker_payload_declared"])
        self.assertTrue(value["platform_evidence"]["linux_packaged_live_worker"])

    def test_platform_cleanup_disclosure_and_review_claim_drift_fails(self) -> None:
        mutations = (
            lambda value: value["summary"].update({"sprint_status": "PASS"}),
            lambda value: value["platform_evidence"].update(
                {"linux_packaged_live_worker": False}
            ),
            lambda value: value["platform_evidence"].update(
                {"linux_complete_operation_matrix": False}
            ),
            lambda value: value["platform_evidence"].update(
                {"linux_live_attack_matrix": False}
            ),
            lambda value: value["platform_evidence"].update(
                {"linux_live_lifecycle_campaign": False}
            ),
            lambda value: value["platform_evidence"].update({"macos_xpc_worker": True}),
            lambda value: value["verification_evidence"].update({"live_cleanup_campaign": True}),
            lambda value: value["verification_evidence"].update(
                {"model_context_disclosure_redaction": False}
            ),
            lambda value: value["verification_evidence"].update({"independent_review": True}),
            lambda value: value["blockers"].pop(),
        )
        for mutate in mutations:
            changed = copy.deepcopy(report())
            mutate(changed)
            self.assertTrue(evidence.validate_report(changed, verify_current=False))

    def test_command_security_and_catalog_mutations_fail(self) -> None:
        mutations = (
            lambda value: value["commands"][0].update({"exit_code": 1}),
            lambda value: value["commands"].pop(),
            lambda value: value["security_requirement_ids"].pop(),
            lambda value: value["implemented_contracts"].update({"write_capable_tools": 1}),
        )
        for mutate in mutations:
            changed = copy.deepcopy(report())
            mutate(changed)
            self.assertTrue(evidence.validate_report(changed, verify_current=False))


if __name__ == "__main__":
    unittest.main()
