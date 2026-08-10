from __future__ import annotations

import copy
import unittest

from scripts.configuration_profiles import (
    CATALOG_PATH,
    PROFILE_SPECS,
    REPORT_PATH,
    build_profiles,
    build_report,
    check_artifacts,
    read_json,
    validate_catalog,
    validate_report,
)


class ConfigurationProfilesTests(unittest.TestCase):
    def test_checked_profiles_catalog_and_report_are_current(self) -> None:
        self.assertEqual(check_artifacts(), [])
        catalog, _ = build_profiles()
        self.assertEqual(read_json(CATALOG_PATH), catalog)
        self.assertEqual(read_json(REPORT_PATH), build_report())

    def test_profile_identity_order_and_phase_closure_are_exact(self) -> None:
        catalog, _ = build_profiles()
        self.assertEqual(
            [item["profile_id"] for item in catalog["profiles"]],
            [item[0] for item in PROFILE_SPECS],
        )
        self.assertEqual(
            sum(item["phase"] == "current-foundation" for item in catalog["profiles"]),
            3,
        )
        self.assertEqual(
            sum(item["activation_status"] == "future-disabled" for item in catalog["profiles"]),
            4,
        )

    def test_future_profiles_retain_only_read_baseline_and_register_nothing(self) -> None:
        catalog, configurations = build_profiles()
        future = [
            item for item in catalog["profiles"] if item["phase"] != "current-foundation"
        ]
        self.assertEqual(len(future), 4)
        for item in future:
            self.assertFalse(item["product_registration"])
            self.assertFalse(item["network_effective"])
            self.assertLessEqual(set(item["effective_capabilities"]), {"workspace.read"})
            configuration = configurations[item["configuration_path"]]
            self.assertEqual(configuration["workspace"]["roots"][0]["access"], "read-only")
            self.assertEqual(configuration["permission"]["network"]["mode"], "deny-all")
            self.assertFalse(configuration["model"]["network_access"])
            self.assertFalse(configuration["shell"]["network_access"])

    def test_profile_catalog_files_and_capabilities_match_exactly(self) -> None:
        catalog = read_json(CATALOG_PATH)
        _, configurations = build_profiles()
        for item in catalog["profiles"]:
            configuration = read_json(CATALOG_PATH.parents[2] / item["configuration_path"])
            self.assertEqual(configuration, configurations[item["configuration_path"]])
            self.assertEqual(
                configuration["permission"]["allowed_capabilities"],
                item["effective_capabilities"],
            )

    def test_duplicate_missing_and_early_activation_mutations_fail_closed(self) -> None:
        catalog, _ = build_profiles()
        duplicate = copy.deepcopy(catalog)
        duplicate["profiles"][1] = copy.deepcopy(duplicate["profiles"][0])
        missing = copy.deepcopy(catalog)
        missing["profiles"].pop()
        activation = copy.deepcopy(catalog)
        activation["profiles"][3]["activation_status"] = "inactive-no-loader"
        registration = copy.deepcopy(catalog)
        registration["profiles"][3]["product_registration"] = True
        for changed in (duplicate, missing, activation, registration):
            self.assertTrue(validate_catalog(changed))

    def test_network_and_authority_broadening_mutations_fail_closed(self) -> None:
        catalog, _ = build_profiles()
        network = copy.deepcopy(catalog)
        network["profiles"][-1]["network_effective"] = True
        authority = copy.deepcopy(catalog)
        authority["profiles"][4]["effective_capabilities"].append("workspace.write")
        self.assertTrue(validate_catalog(network))
        self.assertTrue(validate_catalog(authority))

    def test_report_product_and_macos_overclaims_fail_closed(self) -> None:
        report = build_report()
        product = copy.deepcopy(report)
        product["product_profile_activation_claim"] = "pass"
        macos = copy.deepcopy(report)
        macos["macos_execution_status"] = "pass"
        self.assertTrue(validate_report(product))
        self.assertTrue(validate_report(macos))


if __name__ == "__main__":
    unittest.main()
