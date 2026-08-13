from __future__ import annotations

import copy
import unittest

from scripts import linux_docker_guard_evidence as evidence


class LinuxDockerGuardEvidenceTests(unittest.TestCase):
    def valid_report(self) -> dict:
        profile = evidence.load_profile()
        return {
            "allowed_caller": "guarded-kernel-docker-adapter",
            "artifact_id": "linux-docker-model-runner-endpoint-guard",
            "authority": profile["authority"],
            "denied_callers": list(evidence.DENIED_CALLERS),
            "docker_engine_directly_tested": False,
            "guard_process": profile["guard_process"],
            "host": {
                "architecture": "x86_64",
                "distribution": "fedora",
                "version": "44",
                "docker_cli_available": False,
                "live_enforcement_executed": False,
            },
            "inference_started": False,
            "kernel_transport": profile["kernel_transport"],
            "limitations": list(evidence.LIMITATIONS),
            "model_count": 0,
            "permit_contract": {
                "cloneable": False,
                "constructible_outside_guard": False,
                "contains_address": False,
                "contains_credential": False,
                "serializable": False,
            },
            "profile_sha256": evidence.PROFILE_SHA256,
            "raw_runtime_endpoint": profile["raw_runtime_endpoint"],
            "release_claim": "none",
            "runner_image_digest": evidence.RUNNER_DIGEST,
            "schema_version": 1,
            "source_revision": "a" * 40,
            "sources": [
                {"bytes": 1, "path": path, "sha256": "b" * 64}
                for path in evidence.SOURCE_PATHS
            ],
            "status": "pass-guard-contract-no-live-docker",
            "task_ids": ["9.2.1.3"],
        }

    def test_expected_report_contract_is_valid(self) -> None:
        self.assertEqual(evidence.validate_report(self.valid_report()), [])

    def test_profile_rejects_exposure_authority_and_caller_broadening(self) -> None:
        profile = evidence.load_profile()
        mutations = (
            lambda value: value["authority"].update({"workspace": True}),
            lambda value: value["callers"]["allowed"].append("tool-worker"),
            lambda value: value["raw_runtime_endpoint"].update(
                {"host_tcp_listeners": 1}
            ),
            lambda value: value["guard_process"].update({"docker_socket_mounts": 1}),
            lambda value: value["kernel_transport"].update({"mode": 0o600}),
            lambda value: value["decision"].update({"enforcement_live_tested": True}),
        )
        for mutate in mutations:
            with self.subTest(mutate=mutate):
                changed = copy.deepcopy(profile)
                mutate(changed)
                self.assertTrue(evidence.validate_profile(changed))

    def test_report_rejects_permit_and_live_support_overclaims(self) -> None:
        changed = self.valid_report()
        changed["permit_contract"]["serializable"] = True
        self.assertIn(
            "Docker raw-endpoint permit surface broadened",
            evidence.validate_report(changed),
        )
        changed = self.valid_report()
        changed["host"]["live_enforcement_executed"] = True
        changed["docker_engine_directly_tested"] = True
        self.assertTrue(evidence.validate_report(changed))

    def test_report_rejects_missing_denial_class(self) -> None:
        changed = self.valid_report()
        changed["denied_callers"].remove("arbitrary-container")
        self.assertIn(
            "Docker guard topology or caller closure changed",
            evidence.validate_report(changed),
        )


if __name__ == "__main__":
    unittest.main()
