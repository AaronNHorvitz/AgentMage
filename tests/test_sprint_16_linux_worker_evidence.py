from __future__ import annotations

import copy
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch

from scripts import sprint_16_linux_worker_evidence as evidence


def command() -> dict[str, object]:
    return {
        "id": "guest-command-1",
        "executable": "cargo",
        "exit_code": 0,
        "output_bytes": 1,
        "output_sha256": "a" * 64,
    }


def target(target_id: str, distribution: str) -> dict[str, object]:
    return {
        "target_id": target_id,
        "source_revision": "b" * 40,
        "prepared_base_sha256": "c" * 64,
        "source_bundle_sha256": "d" * 64,
        "strict_offline": True,
        "guest_result": {
            "schema_version": 1,
            "record_type": "sprint-16-installed-worker-guest-result",
            "distribution": distribution,
            "status": "pass",
            "strict_offline": True,
            "package_sha256": "e" * 64,
            "worker": {
                "path_class": "root-owned-package-libexec",
                "mode": "0755",
                "sha256": "f" * 64,
            },
            "verified_operations": evidence.VERIFIED_OPERATIONS,
            "receipt_count": len(evidence.VERIFIED_OPERATIONS),
            "attack_cases": evidence.ATTACK_CASES,
            "linux_attack_matrix_complete": True,
            "lifecycle_cases": evidence.LIFECYCLE_CASES,
            "linux_lifecycle_campaign_complete": True,
            "terminal_receipts_per_lifecycle_case": 1,
            "false_completion_cases": 0,
            "workspace_invariant": True,
            "worker_process_residue": False,
            "transient_unit_residue": False,
            "package_residue": False,
            "network_used_during_execution": False,
            "private_data_used": False,
            "commands": [command()],
        },
        "observation_sha256": {"processes": "1" * 64, "sockets": "2" * 64},
        "cleanup": {
            "connected": {
                "qemu_process_absent": True,
                "loopback_ssh_listener_absent": True,
            },
            "offline": {
                "qemu_process_absent": True,
                "loopback_ssh_listener_absent": True,
            },
            "overlay_absent": True,
            "transient_source_absent": True,
            "credential_material_absent": True,
        },
    }


def report() -> dict[str, object]:
    return {
        "schema_version": 1,
        "record_type": "sprint-16-installed-linux-worker-matrix",
        "task_ids": ["16.1.1.5", "16.1.2.3", "16.1.3.3", "16.1.3.4"],
        "source_revision": "b" * 40,
        "status": "pass-installed-linux-worker-operation-attack-lifecycle-matrix",
        "qemu": {
            "launcher_class": "toolbox",
            "version": "qemu-test",
            "sha256": "3" * 64,
        },
        "targets": [
            target("fedora-44-x86_64", "fedora"),
            target("ubuntu-26.04-x86_64", "ubuntu"),
        ],
        "verified_operations": evidence.VERIFIED_OPERATIONS,
        "complete_ten_tool_matrix": True,
        "linux_attack_matrix_complete": True,
        "linux_lifecycle_campaign_complete": True,
        "attack_matrix_complete": False,
        "cleanup_campaign_complete": False,
        "macos_evidence_substituted": False,
        "private_host_data_used": False,
        "repository_credentials_injected": False,
        "release_claim": False,
        "sources": [
            {"path": path, "bytes": 1, "sha256": "4" * 64}
            for path in evidence.SOURCE_PATHS
        ],
    }


class Sprint16LinuxWorkerEvidenceTests(unittest.TestCase):
    @staticmethod
    def current_report() -> dict[str, object]:
        value = report()
        value["schema_version"] = 2
        value["complete_source_closure"] = {
            "schema_version": 1, "scope": "complete-committed-source-tree",
            "file_count": 50, "total_bytes": 1000, "sha256": "5" * 64,
        }
        return value

    def validate_current(self, value: dict[str, object]) -> list[str]:
        with (
            patch.object(evidence, "source_records", return_value=report()["sources"]),
            patch.object(evidence, "complete_source_closure",
                         return_value=self.current_report()["complete_source_closure"]),
        ):
            return evidence.validate_current_source(value, "c" * 40)

    def test_identical_complete_source_at_different_commits_is_eligible(self) -> None:
        self.assertEqual(self.validate_current(self.current_report()), [])

    def test_legacy_matrix_is_historical_not_current(self) -> None:
        self.assertEqual(evidence.validate_report(report()), [])
        self.assertIn("historical", self.validate_current(report())[0])

    def test_schema_versions_require_integer_not_python_equal_boolean_or_float(self) -> None:
        for version in (True, 1.0, 2.0, "2", None):
            with self.subTest(outer=version):
                value = self.current_report()
                value["schema_version"] = version
                self.assertTrue(evidence.validate_report(value))
                self.assertTrue(self.validate_current(value))
        for version in (True, 1.0, "1", None):
            with self.subTest(closure=version):
                value = self.current_report()
                value["complete_source_closure"]["schema_version"] = version
                self.assertTrue(evidence.validate_report(value))
                self.assertTrue(self.validate_current(value))

    def test_missing_forged_duplicate_and_incomplete_closures_fail(self) -> None:
        mutations = (
            lambda value: value.pop("complete_source_closure"),
            lambda value: value["complete_source_closure"].update({"sha256": "6" * 64}),
            lambda value: value["complete_source_closure"].update({"file_count": True}),
            lambda value: value["complete_source_closure"].update({"scope": "fourteen-files"}),
            lambda value: value["complete_source_closure"].update({"extra": True}),
            lambda value: value["sources"].append(value["sources"][0]),
            lambda value: value["sources"][0].update({"sha256": "9" * 64}),
            lambda value: value["sources"][0].update({"bytes": 20}),
            lambda value: value["sources"][0].update({"bytes": True}),
            lambda value: value["targets"][0]["guest_result"].update({"status": "failed"}),
        )
        for mutate in mutations:
            with self.subTest(mutation=mutate):
                value = self.current_report()
                mutate(value)
                self.assertTrue(self.validate_current(value))

    def test_changed_or_unavailable_committed_source_fails(self) -> None:
        expected = self.current_report()["complete_source_closure"]
        changed = dict(expected, sha256="6" * 64)
        with patch.object(evidence, "source_records", return_value=report()["sources"]):
            with patch.object(evidence, "complete_source_closure", side_effect=[expected, changed]):
                self.assertIn("not equivalent", evidence.validate_current_source(self.current_report(), "c" * 40)[0])
            with patch.object(evidence, "complete_source_closure", side_effect=ValueError("missing commit")):
                self.assertTrue(evidence.validate_current_source(self.current_report(), "c" * 40))

    def test_real_git_closure_covers_child_modules_modes_and_docs(self) -> None:
        # Synthetic developer fixture only: no installed guest or platform claim.
        with tempfile.TemporaryDirectory(prefix="agentmage-source-closure-") as directory:
            root = Path(directory)
            def git(*args: str) -> str:
                return subprocess.run(
                    ["git", "-c", "user.name=Fixture", "-c", "user.email=fixture@example.invalid", *args],
                    cwd=root, check=True, capture_output=True, text=True, timeout=10,
                ).stdout.strip()
            git("init", "--quiet")
            child = root / "platforms/linux/src/sandbox_supervision.rs"
            child.parent.mkdir(parents=True)
            child.write_bytes(b"pub fn child() {}\n")
            (child.parent / "sandbox.rs").write_bytes(b"mod sandbox_supervision;\n")
            git("add", ".")
            git("commit", "--quiet", "-m", "fixture")
            first = git("rev-parse", "HEAD")
            with patch.object(evidence, "ROOT", root):
                original = evidence.complete_source_closure(first)
                git("commit", "--quiet", "--allow-empty", "-m", "same tree")
                self.assertEqual(original, evidence.complete_source_closure(git("rev-parse", "HEAD")))
                child.write_bytes(b"pub fn changed_child() {}\n")
                git("add", ".")
                git("commit", "--quiet", "-m", "child changed, parent unchanged")
                updated = evidence.complete_source_closure(git("rev-parse", "HEAD"))
                self.assertNotEqual(original["sha256"], updated["sha256"])
                child.chmod(0o755)
                git("add", ".")
                git("commit", "--quiet", "-m", "mode changed")
                mode = evidence.complete_source_closure(git("rev-parse", "HEAD"))
                self.assertNotEqual(updated["sha256"], mode["sha256"])
                (root / "README.md").write_bytes(b"Conservative whole-tree policy.\n")
                git("add", ".")
                git("commit", "--quiet", "-m", "docs changed")
                self.assertNotEqual(mode, evidence.complete_source_closure(git("rev-parse", "HEAD")))
                self.assertEqual(original, evidence.complete_source_closure(first))
                with patch.object(evidence, "MAX_SOURCE_BYTES", 1):
                    with self.assertRaisesRegex(ValueError, "exceeds bound"):
                        evidence.complete_source_closure(first)
                with self.assertRaises(subprocess.CalledProcessError):
                    evidence.complete_source_closure("0" * 40)

    def test_batch_integrity_refuses_truncation_identity_and_trailing_bytes(self) -> None:
        def output(data: bytes) -> subprocess.CompletedProcess:
            return subprocess.CompletedProcess([], 0, stdout=data)
        oid = b"a" * 40
        inventory = b"100644 blob " + oid + b"       3\tfile.rs\0"
        correct = oid + b" blob 3\nabc\n"
        for batch in (correct[:-1], correct + b"extra", correct.replace(b" blob 3", b" blob 2")):
            with self.subTest(batch=batch), patch.object(
                evidence.subprocess, "run", side_effect=[output(b"commit\n"), output(inventory), output(batch)]
            ):
                with self.assertRaises(ValueError):
                    evidence.complete_source_closure("b" * 40)
        with patch.object(evidence.subprocess, "run", side_effect=[output(b"commit\n"), output(inventory * 2)]):
            with self.assertRaisesRegex(ValueError, "inventory invalid"):
                evidence.complete_source_closure("b" * 40)

    def test_exact_operation_matrix_report_is_valid(self) -> None:
        self.assertEqual(evidence.validate_report(report()), [])

    def test_target_receipt_and_cleanup_mutations_fail(self) -> None:
        mutations = (
            lambda value: value["targets"].pop(),
            lambda value: value["targets"][0]["guest_result"].update({"receipt_count": 9}),
            lambda value: value["targets"][0]["guest_result"]["commands"][0].update(
                {"exit_code": 1}
            ),
            lambda value: value["targets"][1]["cleanup"].update({"overlay_absent": False}),
            lambda value: value["targets"][1]["guest_result"].update(
                {"worker_process_residue": True}
            ),
        )
        for mutate in mutations:
            changed = copy.deepcopy(report())
            mutate(changed)
            self.assertTrue(evidence.validate_report(changed))

    def test_platform_privacy_and_completion_overclaims_fail(self) -> None:
        fields = (
            "attack_matrix_complete",
            "cleanup_campaign_complete",
            "macos_evidence_substituted",
            "private_host_data_used",
            "repository_credentials_injected",
            "release_claim",
        )
        for field in fields:
            changed = copy.deepcopy(report())
            changed[field] = True
            self.assertTrue(evidence.validate_report(changed))
        changed = copy.deepcopy(report())
        changed["complete_ten_tool_matrix"] = False
        self.assertTrue(evidence.validate_report(changed))
        changed = copy.deepcopy(report())
        changed["linux_attack_matrix_complete"] = False
        self.assertTrue(evidence.validate_report(changed))
        changed = copy.deepcopy(report())
        changed["linux_lifecycle_campaign_complete"] = False
        self.assertTrue(evidence.validate_report(changed))

    def test_source_and_qemu_identity_mutations_fail(self) -> None:
        changed = copy.deepcopy(report())
        changed["sources"].pop()
        changed["qemu"]["sha256"] = "invalid"
        failures = evidence.validate_report(changed)
        self.assertIn("installed worker source closure drifted", failures)
        self.assertIn("installed worker QEMU identity drifted", failures)


if __name__ == "__main__":
    unittest.main()
