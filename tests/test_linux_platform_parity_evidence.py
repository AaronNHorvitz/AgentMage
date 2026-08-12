"""Tests for the bounded Fedora/Ubuntu platform parity report."""

from __future__ import annotations

import copy
import unittest

from scripts import linux_platform_parity_evidence as evidence


class LinuxPlatformParityEvidenceTests(unittest.TestCase):
    def valid_report(self) -> dict:
        values = evidence.load_inputs()
        return {
            "schema_version": 1,
            "artifact_id": "linux-fedora-ubuntu-adapter-parity",
            "task_ids": ["9.1.2.4"],
            "status": "partial-parity-with-open-native-controls",
            "source_revision": "a" * 40,
            "sources": [
                {"path": path, "sha256": "b" * 64} for path in evidence.SOURCE_PATHS
            ],
            "platform_scope": ["fedora-44-x86_64", "ubuntu-26.04-x86_64"],
            "input_evidence": evidence.input_records(values),
            "dimensions": copy.deepcopy(list(evidence.DIMENSIONS)),
            "summary": {
                "dimension_count": 10,
                "verified_parity_dimensions": 6,
                "blocked_parity_dimensions": 4,
                "full_fedora_ubuntu_parity": False,
            },
            "blocking_gates": copy.deepcopy(list(evidence.BLOCKERS)),
            "private_values_present": False,
            "macos_evidence_substituted": False,
            "release_claim": "none",
            "limitations": list(evidence.LIMITATIONS),
        }

    def test_current_inputs_and_bounded_report_are_valid(self) -> None:
        self.assertEqual(evidence.validate_input_evidence(evidence.load_inputs()), [])
        self.assertEqual(evidence.validate_report(self.valid_report()), [])

    def test_missing_or_promoted_dimension_fails(self) -> None:
        missing = self.valid_report()
        missing["dimensions"].pop()
        self.assertIn(
            "Linux platform parity dimensions changed",
            evidence.validate_report(missing),
        )
        promoted = self.valid_report()
        promoted["dimensions"][6]["ubuntu"] = "verified-live"
        promoted["dimensions"][6]["parity"] = "verified"
        self.assertIn(
            "Linux platform parity dimensions changed",
            evidence.validate_report(promoted),
        )

    def test_full_parity_or_removed_blocker_fails(self) -> None:
        full = self.valid_report()
        full["summary"]["full_fedora_ubuntu_parity"] = True
        self.assertIn(
            "Linux platform parity summary overclaimed or changed",
            evidence.validate_report(full),
        )
        missing = self.valid_report()
        missing["blocking_gates"].pop()
        self.assertIn(
            "Linux platform parity blockers changed",
            evidence.validate_report(missing),
        )

    def test_private_macos_or_release_overclaim_fails(self) -> None:
        for field, replacement in (
            ("private_values_present", True),
            ("macos_evidence_substituted", True),
            ("release_claim", "supported"),
        ):
            with self.subTest(field=field):
                report = self.valid_report()
                report[field] = replacement
                self.assertIn(
                    "Linux platform parity report made an unsupported claim",
                    evidence.validate_report(report),
                )

    def test_each_input_family_rejects_its_key_overclaim(self) -> None:
        mutations = []

        clean = copy.deepcopy(evidence.load_inputs())
        clean["clean-build"]["platform_runs"]["ubuntu-x86_64"]["status"] = "fail"
        mutations.append((clean, "clean Linux build parity input is incomplete"))

        paths = copy.deepcopy(evidence.load_inputs())
        paths["path-conformance"]["release_claim"] = "supported"
        mutations.append((paths, "path policy parity input is incomplete"))

        contract = copy.deepcopy(evidence.load_inputs())
        contract["platform-contract"]["api"][
            "operating_system_branches_in_kernel_selector"
        ] = 1
        mutations.append((contract, "shared adapter contract parity input is incomplete"))

        package = copy.deepcopy(evidence.load_inputs())
        package["package-lifecycle"]["container_lifecycle"]["platforms"].pop()
        mutations.append((package, "clean package lifecycle parity input is incomplete"))

        controls = copy.deepcopy(evidence.load_inputs())
        controls["linux-controls"]["platform_status"][
            "ubuntu_26_04_x86_64"
        ] = "verified-local"
        mutations.append((controls, "live Linux control parity input is incomplete"))

        startup_controls = copy.deepcopy(evidence.load_inputs())
        startup_controls["linux-controls"]["startup_control_ids"].pop()
        mutations.append(
            (startup_controls, "live Linux control parity input is incomplete")
        )

        attacks = copy.deepcopy(evidence.load_inputs())
        attacks["sandbox-attacks"]["summary"][
            "native_ubuntu_isolation_verified"
        ] = True
        mutations.append(
            (attacks, "bounded cross-distribution sandbox input is incomplete")
        )

        inference = copy.deepcopy(evidence.load_inputs())
        inference["inactive-inference"]["enabled_models"] = 1
        mutations.append((inference, "inactive inference parity input is incomplete"))

        for values, expected in mutations:
            with self.subTest(expected=expected):
                self.assertIn(expected, evidence.validate_input_evidence(values))


if __name__ == "__main__":
    unittest.main()
