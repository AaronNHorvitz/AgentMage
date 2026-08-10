from __future__ import annotations

import copy
import unittest

from scripts.fuzz_toolchain_policy import (
    CARGO_FUZZ_VERSION,
    DICTIONARY_PATHS,
    JAZZER_VERSION,
    POLICY_PATH,
    REPORT_PATH,
    RUST_NIGHTLY,
    build_policy,
    build_report,
    check_artifacts,
    read_json,
    validate_policy,
    validate_report,
)


class FuzzToolchainPolicyTests(unittest.TestCase):
    def test_checked_policy_and_report_are_current(self) -> None:
        self.assertEqual(check_artifacts(), [])
        self.assertEqual(read_json(POLICY_PATH), build_policy())
        self.assertEqual(read_json(REPORT_PATH), build_report())

    def test_engines_toolchains_and_integrities_are_pinned(self) -> None:
        engines = build_policy()["engines"]
        self.assertEqual(engines[0]["runner"]["version"], CARGO_FUZZ_VERSION)
        self.assertEqual(engines[0]["toolchain"]["channel"], RUST_NIGHTLY)
        self.assertEqual(engines[1]["runner"]["version"], JAZZER_VERSION)
        self.assertTrue(engines[1]["runner"]["npm_integrity"].startswith("sha512-"))
        self.assertEqual(engines[2]["runner"]["third_party_dependencies"], [])

    def test_dictionaries_and_seed_corpora_are_hash_bound(self) -> None:
        policy = build_policy()
        self.assertEqual(
            [item["path"] for item in policy["dictionaries"]],
            list(DICTIONARY_PATHS),
        )
        self.assertTrue(
            all(len(item["sha256"]) == 64 for item in policy["dictionaries"])
        )
        self.assertTrue(all(len(item["sha256"]) == 64 for item in policy["seed_corpora"]))

    def test_limits_durations_deduplication_and_retention_fail_closed(self) -> None:
        policy = build_policy()
        self.assertFalse(policy["resource_limits"]["host_resource_exhaustion_permitted"])
        self.assertGreaterEqual(
            policy["minimum_durations_seconds"]["continuous_integration_changed_target"],
            policy["minimum_durations_seconds"]["local_changed_target"],
        )
        self.assertFalse(
            policy["crash_deduplication"]["cross_target_deduplication_permitted"]
        )
        self.assertEqual(policy["regression_retention"]["failed_replay_disposition"], "block")

    def test_version_dictionary_budget_and_retention_mutations_are_rejected(self) -> None:
        policy = build_policy()
        version = copy.deepcopy(policy)
        version["engines"][0]["runner"]["version"] = "latest"
        dictionary = copy.deepcopy(policy)
        dictionary["dictionaries"][0]["sha256"] = "0" * 64
        budget = copy.deepcopy(policy)
        budget["resource_limits"]["host_resource_exhaustion_permitted"] = True
        retention = copy.deepcopy(policy)
        retention["regression_retention"]["raw_private_input_permitted"] = True
        for changed in (version, dictionary, budget, retention):
            self.assertTrue(validate_policy(changed))

    def test_acquisition_execution_product_and_macos_overclaims_are_rejected(self) -> None:
        policy = build_policy()
        networked = copy.deepcopy(policy)
        networked["execution_network_policy"] = "online"
        claimed = copy.deepcopy(policy)
        claimed["product_fuzz_execution_claim"] = "pass"
        macos = copy.deepcopy(policy)
        macos["macos_execution_status"] = "pass"
        for changed in (networked, claimed, macos):
            self.assertTrue(validate_policy(changed))

    def test_report_rejects_install_or_execution_claim(self) -> None:
        report = build_report()
        installed = copy.deepcopy(report)
        installed["tools_installed_by_task"] = True
        executed = copy.deepcopy(report)
        executed["product_fuzz_execution_claim"] = "pass"
        self.assertTrue(validate_report(installed))
        self.assertTrue(validate_report(executed))


if __name__ == "__main__":
    unittest.main()
