"""Tests for direct inference-client isolation evidence."""

from __future__ import annotations

import copy
import unittest

from scripts import inference_client_isolation_evidence as evidence


class InferenceClientIsolationEvidenceTests(unittest.TestCase):
    def valid_report(self) -> dict:
        return {
            "schema_version": 1,
            "artifact_id": "inference-client-isolation",
            "source_revision": "a" * 40,
            "task_ids": ["10.1.1.2"],
            "status": "pass-shipped-vscode-and-linux-worker-no-enabled-native-inference",
            "client_boundaries": copy.deepcopy(evidence.CLIENT_BOUNDARIES),
            "verification_commands": evidence.expected_commands(),
            "prerequisite_evidence": {
                "docker": {
                    "path": evidence.REACHABILITY_PATH.relative_to(evidence.ROOT).as_posix(),
                    "sha256": "b" * 64,
                    "source_revision": "c" * 40,
                    "targets": [
                        {
                            "target_id": target_id,
                            "positions": [
                                {
                                    "position": position,
                                    "raw_tcp_connected": False,
                                    "status": "denied",
                                }
                                for position in ("vscode-extension", "tool-worker")
                            ],
                            "authenticated_guard_control": True,
                            "inference_request_sent": False,
                            "cleanup_complete": True,
                        }
                        for target_id in (
                            "fedora-44-x86_64",
                            "ubuntu-26.04-x86_64",
                        )
                    ],
                },
                "native": {
                    "path": evidence.NATIVE_PATH.relative_to(evidence.ROOT).as_posix(),
                    "sha256": "d" * 64,
                    "source_revision": "e" * 40,
                    "enabled_models": 0,
                    "inference_started": False,
                    "release_claim": "none",
                },
            },
            "claims": {
                "shipped_vscode_direct_raw_client_absent": True,
                "tool_worker_direct_raw_client_denied": True,
                "docker_vscode_and_worker_positions_denied": True,
                "enabled_native_runtime_tested": False,
                "arbitrary_same_user_process_confinement_tested": False,
                "inference_performed": False,
                "release_support": False,
            },
            "limitations": list(evidence.LIMITATIONS),
            "private_user_data_used": False,
            "network_used": False,
            "sources": [
                {"path": path, "bytes": 1, "sha256": "f" * 64}
                for path in evidence.SOURCE_PATHS
            ],
        }

    def test_exact_report_is_valid(self) -> None:
        self.assertEqual(evidence.validate_report(self.valid_report()), [])

    def test_client_or_command_broadening_fails(self) -> None:
        client = self.valid_report()
        client["client_boundaries"][0]["direct_raw_runtime_client"] = True
        self.assertIn(
            "inference client boundary matrix changed",
            evidence.validate_report(client),
        )
        command = self.valid_report()
        command["verification_commands"].pop()
        self.assertIn(
            "inference client verification commands changed",
            evidence.validate_report(command),
        )

    def test_docker_position_or_native_activation_mutation_fails(self) -> None:
        docker = self.valid_report()
        docker["prerequisite_evidence"]["docker"]["targets"][0]["positions"][0][
            "status"
        ] = "allowed"
        self.assertIn(
            "Docker direct-client isolation evidence changed",
            evidence.validate_report(docker),
        )
        native = self.valid_report()
        native["prerequisite_evidence"]["native"]["enabled_models"] = 1
        self.assertIn(
            "native inactive-runtime evidence changed",
            evidence.validate_report(native),
        )

    def test_claim_or_limitation_overstatement_fails(self) -> None:
        claim = self.valid_report()
        claim["claims"]["enabled_native_runtime_tested"] = True
        self.assertIn(
            "inference client isolation evidence overclaimed",
            evidence.validate_report(claim),
        )
        limitation = self.valid_report()
        limitation["limitations"].pop()
        self.assertIn(
            "inference client isolation limitations changed",
            evidence.validate_report(limitation),
        )

    def test_source_omission_or_prohibited_use_fails(self) -> None:
        source = self.valid_report()
        source["sources"].pop()
        self.assertIn(
            "inference client source evidence changed",
            evidence.validate_report(source),
        )
        for field in ("private_user_data_used", "network_used"):
            with self.subTest(field=field):
                changed = self.valid_report()
                changed[field] = True
                self.assertIn(
                    "inference client evidence used prohibited data or network",
                    evidence.validate_report(changed),
                )


if __name__ == "__main__":
    unittest.main()
