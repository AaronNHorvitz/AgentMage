from __future__ import annotations

import copy
import unittest
from unittest.mock import patch

from scripts import sprint_40_lifecycle as lifecycle
from scripts.package_lifecycle import (
    NONZERO_STEP_IDS,
    lifecycle_admin_step_ids,
    lifecycle_step_ids,
)


def command_records(
    commands: tuple[tuple[str, tuple[str, ...]], ...], *, tests: bool
) -> list[dict[str, object]]:
    return [
        {
            "id": identifier,
            "argv": list(argv),
            "exit_code": 0,
            "output_bytes": 1,
            "output_sha256": "a" * 64,
            "observed_test_passes": 1 if tests else None,
            "observed_test_ignores": 0 if tests else None,
        }
        for identifier, argv in commands
    ]


def package_lifecycle() -> dict[str, object]:
    steps = lifecycle_step_ids(
        lifecycle.BASELINE_VERSION, lifecycle.UPGRADE_VERSION
    )
    admin = lifecycle_admin_step_ids(
        lifecycle.BASELINE_VERSION, lifecycle.UPGRADE_VERSION
    )
    nonzero = NONZERO_STEP_IDS
    platforms = []
    for platform_id, image in (
        ("fedora-x86_64", "docker.io/library/fedora@sha256:" + "a" * 64),
        ("ubuntu-x86_64", "docker.io/library/ubuntu@sha256:" + "b" * 64),
    ):
        platforms.append(
            {
                "platform_id": platform_id,
                "status": "pass",
                "package_administrator": "0:0",
                "runtime_user": "10001:10001",
                "image": {
                    "reference": image,
                    "id": "sha256:" + "c" * 64,
                    "repo_digests": [image],
                    "architecture": "amd64",
                    "os": "linux",
                },
                "controls": {
                    "runtime": "rootless-podman",
                    "network": "none",
                    "privileged": False,
                    "capabilities_added": [],
                    "capabilities_dropped": ["ALL"],
                    "no_new_privileges": True,
                    "selinux_label_isolated_mount": True,
                    "pids_limit": 64,
                    "memory_limit_bytes": 512 * 1024 * 1024,
                    "root_filesystem": "ephemeral-writable-for-package-manager",
                    "package_mount": "read-only",
                },
                "checks": {
                    key: True
                    for key in (
                        "clean_install",
                        "standard_user_launch",
                        "component_manifest_exact",
                        "root_owned_runtime_read_only",
                        "corrupt_upgrade_refused",
                        "prior_valid_state_preserved",
                        "upgrade",
                        "rollback",
                        "uninstall",
                        "reinstall_recovery",
                        "final_package_record_absent",
                        "final_filesystem_residue_absent",
                        "network_disabled",
                    )
                },
                "steps": [
                    {
                        "id": step,
                        "status": "pass",
                        "actor": "package-administrator" if step in admin else "standard-user",
                        "uid_gid": "0:0" if step in admin else "10001:10001",
                        "expected_exit": "nonzero" if step in nonzero else "zero",
                        "observed_exit": "nonzero" if step in nonzero else "zero",
                        "output_bytes": 0,
                        "output_sha256": "d" * 64,
                    }
                    for step in steps
                ],
            }
        )
    return {
        "schema_version": 1,
        "status": "pass",
        "rootless_runtime": True,
        "network_used": False,
        "platforms": platforms,
    }


def report() -> dict[str, object]:
    return {
        "schema_version": 1,
        "record_type": "sprint_40_v0_3_lifecycle",
        "source_revision": "a" * 40,
        "source_tree": "b" * 40,
        "source_sha256": {path: "c" * 64 for path in lifecycle.SOURCE_PATHS},
        "environment": {},
        "versions": {
            "baseline": lifecycle.BASELINE_VERSION,
            "upgrade": lifecycle.UPGRADE_VERSION,
        },
        "build_commands": command_records(lifecycle.BUILD_COMMANDS, tests=False),
        "candidate_artifacts": [
            {
                "version": version,
                "kind": kind,
                "name": f"candidate-{version}.{kind}",
                "size": 1,
                "sha256": "d" * 64,
                "signed": False,
                "published": False,
                "retained_in_repository": False,
            }
            for version in (lifecycle.BASELINE_VERSION, lifecycle.UPGRADE_VERSION)
            for kind in ("deb", "rpm", "vsix")
        ],
        "extracted_candidate_checks": {
            "extracted_payloads": "pass",
            "mutation_refusal": "pass",
        },
        "package_lifecycle": package_lifecycle(),
        "state_lifecycle_commands": command_records(lifecycle.STATE_COMMANDS, tests=True),
        "acceptance": {
            "clean_install": True,
            "corrupt_upgrade_refusal": True,
            "upgrade": True,
            "downgrade": True,
            "uninstall_and_residue": True,
            "backup_and_restore": True,
            "migration_interruption_recovery": True,
            "interrupted_authority_resume_without_replay": True,
            "interrupted_write_resume_without_replay": True,
            "complete_local_lifecycle_campaign": True,
        },
        "limitations": list(lifecycle.LIMITATIONS),
        "release_claim": False,
    }


class Sprint40LifecycleTests(unittest.TestCase):
    def validate(self, value: dict[str, object]) -> list[str]:
        with patch.object(lifecycle, "git_file", return_value=b"source"):
            return lifecycle.validate_report(value, verify_current=False)

    def test_complete_local_lifecycle_passes_without_release_claim(self) -> None:
        value = report()
        self.assertEqual(self.validate(value), [])
        self.assertTrue(value["acceptance"]["complete_local_lifecycle_campaign"])
        self.assertFalse(value["release_claim"])

    def test_authority_version_test_and_package_mutations_fail(self) -> None:
        mutations = (
            lambda value: value["versions"].update({"upgrade": "0.3.1"}),
            lambda value: value["state_lifecycle_commands"][0].update({"exit_code": 1}),
            lambda value: value["state_lifecycle_commands"][0].update({"observed_test_ignores": 1}),
            lambda value: value["package_lifecycle"]["platforms"][0]["controls"].update({"network": "bridge"}),
            lambda value: value["candidate_artifacts"][0].update({"signed": True}),
            lambda value: value["acceptance"].update({"backup_and_restore": False}),
            lambda value: value.update({"release_claim": True}),
        )
        for mutate in mutations:
            changed = copy.deepcopy(report())
            mutate(changed)
            self.assertTrue(self.validate(changed))

    def test_missing_source_and_command_records_fail(self) -> None:
        for field in ("source_sha256", "build_commands", "state_lifecycle_commands"):
            changed = copy.deepcopy(report())
            if isinstance(changed[field], dict):
                changed[field].pop(next(iter(changed[field])))
            else:
                changed[field].pop()
            self.assertTrue(self.validate(changed), field)


if __name__ == "__main__":
    unittest.main()
