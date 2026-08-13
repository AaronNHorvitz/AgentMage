from __future__ import annotations

import copy
import unittest

from scripts import linux_docker_runtime_evidence as evidence


class LinuxDockerRuntimeEvidenceTests(unittest.TestCase):
    def valid_report(self) -> dict:
        profile = evidence.load_profile()
        return {
            "artifact_id": "linux-docker-model-runner-compatibility-profile",
            "authority": profile["authority"],
            "daemon_prerequisites": profile["daemon_prerequisites"],
            "enabled_models": 0,
            "host": {
                "architecture": "x86_64",
                "distribution": "fedora",
                "version": "44",
                "docker_cli_available": False,
                "docker_engine_contacted": False,
            },
            "inference_started": False,
            "limitations": list(evidence.LIMITATIONS),
            "model_artifact": profile["model_artifact"],
            "mount_policy": profile["mount_policy"],
            "network_policy": profile["network_policy"],
            "package": profile["package"],
            "prior_evidence": {
                "docker_engine_directly_tested": False,
                "podman_compatibility_bundle": "artifacts/sprints/sprint-0/story-0.3-dmr/evidence-manifest.json",
                "podman_compatibility_bundle_sha256": "a" * 64,
                "quality_status": "FAIL",
            },
            "process_boundary": copy.deepcopy(evidence.EXPECTED_DESCRIPTOR),
            "profile_sha256": evidence.PROFILE_SHA256,
            "release_claim": "none",
            "resource_ceiling": profile["resource_ceiling"],
            "runner_engine": profile["engine"],
            "schema_version": 1,
            "source_revision": "b" * 40,
            "sources": [
                {"bytes": 1, "path": path, "sha256": "c" * 64}
                for path in evidence.SOURCE_PATHS
            ],
            "status": "pass-pinned-contract-docker-not-installed",
            "task_ids": ["9.2.1.2"],
        }

    def test_expected_report_contract_is_valid(self) -> None:
        self.assertEqual(evidence.validate_report(self.valid_report()), [])

    def test_profile_rejects_authority_mount_network_and_activation_drift(self) -> None:
        profile = evidence.load_profile()
        mutations = (
            lambda value: value["authority"].update({"workspace": True}),
            lambda value: value["mount_policy"].update({"docker_socket_mounts": 1}),
            lambda value: value["network_policy"].update({"outbound_bytes": 1}),
            lambda value: value["decision"].update({"enabled_models": 1}),
        )
        for mutate in mutations:
            with self.subTest(mutate=mutate):
                changed = copy.deepcopy(profile)
                mutate(changed)
                self.assertTrue(evidence.validate_profile(changed))

    def test_report_rejects_docker_support_and_prior_evidence_overclaims(self) -> None:
        changed = self.valid_report()
        changed["host"]["docker_engine_contacted"] = True
        self.assertIn(
            "Docker-absent host evidence changed",
            evidence.validate_report(changed),
        )
        changed = self.valid_report()
        changed["prior_evidence"]["docker_engine_directly_tested"] = True
        changed["prior_evidence"]["quality_status"] = "PASS"
        self.assertIn(
            "prior compatibility evidence was overstated",
            evidence.validate_report(changed),
        )

    def test_report_rejects_mutable_identity_and_enabled_runtime(self) -> None:
        changed = self.valid_report()
        changed["runner_engine"]["manifest_digest"] = "sha256:" + "0" * 64
        self.assertIn(
            "Docker compatibility runner engine changed",
            evidence.validate_report(changed),
        )
        changed = self.valid_report()
        changed["enabled_models"] = 1
        changed["process_boundary"]["docker_compatibility_available"] = True
        self.assertIn(
            "Docker compatibility state or limitations were overclaimed",
            evidence.validate_report(changed),
        )


if __name__ == "__main__":
    unittest.main()
