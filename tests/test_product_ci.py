from __future__ import annotations

import copy
import io
import subprocess
import unittest
from contextlib import redirect_stderr
from pathlib import Path
from unittest.mock import patch

from scripts.product_ci import (
    ROOT,
    check_contract,
    read_json,
    run_lane,
    sanitize_output,
    validate_contract,
)


class ProductCiTests(unittest.TestCase):
    def setUp(self) -> None:
        self.policy = read_json(ROOT / "architecture/product-ci-policy.json")
        self.package = read_json(ROOT / "package.json")
        import tomllib

        self.rust = tomllib.loads((ROOT / "rust-toolchain.toml").read_text())
        self.workflow = (ROOT / ".github/workflows/product.yml").read_text()
        self.documentation = (ROOT / ".github/workflows/documentation.yml").read_text()

    def validate(self, policy=None, workflow=None, package=None, rust=None, documentation=None):
        return validate_contract(
            policy if policy is not None else self.policy,
            workflow if workflow is not None else self.workflow,
            package if package is not None else self.package,
            rust if rust is not None else self.rust,
            documentation if documentation is not None else self.documentation,
        )

    def test_checked_in_contract_is_valid(self) -> None:
        self.assertEqual(check_contract(), [])

    def test_rust_and_node_version_mismatches_are_rejected(self) -> None:
        rust = copy.deepcopy(self.rust)
        rust["toolchain"]["channel"] = "stable"
        package = copy.deepcopy(self.package)
        package["engines"]["node"] = ">=22 <23"
        self.assertTrue(self.validate(rust=rust))
        self.assertTrue(self.validate(package=package))

    def test_every_declared_command_failure_fails_its_lane(self) -> None:
        for lane, commands in self.policy["lanes"].items():
            for failed_index in range(len(commands)):
                results = [
                    subprocess.CompletedProcess(command, 0, "")
                    for command in commands[:failed_index]
                ]
                results.append(
                    subprocess.CompletedProcess(
                        commands[failed_index],
                        1,
                        f"failed under {ROOT} and {Path.home()}",
                    )
                )
                with patch("scripts.product_ci._run", side_effect=results):
                    with redirect_stderr(io.StringIO()) as stderr:
                        self.assertEqual(run_lane(lane), 1, (lane, failed_index))
                self.assertNotIn(str(ROOT), stderr.getvalue())
                self.assertNotIn(str(Path.home()), stderr.getvalue())

    def test_command_weakening_is_rejected(self) -> None:
        policy = copy.deepcopy(self.policy)
        policy["lanes"]["lint"][0].remove("--locked")
        self.assertTrue(self.validate(policy=policy))
        policy = copy.deepcopy(self.policy)
        policy["native_linux"]["inventory_command"].remove("--ignored")
        self.assertTrue(self.validate(policy=policy))

    def test_missing_or_duplicated_lane_is_rejected(self) -> None:
        missing = self.workflow.replace(
            "python3 scripts/product_ci.py --run-lane build", "python3 -V", 1
        )
        duplicated = self.workflow.replace(
            "python3 scripts/product_ci.py --run-lane build",
            "python3 scripts/product_ci.py --run-lane build && "
            "python3 scripts/product_ci.py --run-lane build",
            1,
        )
        self.assertTrue(self.validate(workflow=missing))
        self.assertTrue(self.validate(workflow=duplicated))

    def test_job_removal_and_continue_on_error_are_rejected(self) -> None:
        removed = self.workflow.replace("  contract:\n", "  contract_removed:\n", 1)
        weakened = self.workflow.replace(
            "    runs-on: ubuntu-24.04",
            "    continue-on-error: true\n    runs-on: ubuntu-24.04",
            1,
        )
        self.assertTrue(self.validate(workflow=removed))
        self.assertTrue(self.validate(workflow=weakened))

    def test_documentation_and_product_gates_cannot_be_merged(self) -> None:
        product = self.workflow + "\n# npm run docs:check\n"
        documentation = self.documentation + "\n# product_ci.py\n"
        self.assertTrue(self.validate(workflow=product))
        self.assertTrue(self.validate(documentation=documentation))

    def test_unpinned_action_is_rejected(self) -> None:
        workflow = self.workflow.replace(
            "actions/checkout@11d5960a326750d5838078e36cf38b85af677262",
            "actions/checkout@v4",
            1,
        )
        self.assertTrue(self.validate(workflow=workflow))

    def test_failure_output_removes_private_host_paths(self) -> None:
        private = f"failure at {ROOT}/src and {Path.home()}/secret"
        sanitized = sanitize_output(private)
        self.assertNotIn(str(ROOT), sanitized)
        self.assertNotIn(str(Path.home()), sanitized)
        self.assertIn("<REPOSITORY>", sanitized)

    def test_native_inventory_is_explicitly_pending(self) -> None:
        native = self.policy["native_linux"]
        self.assertEqual(native["generic_ci_disposition"], "pending-native-execution")
        self.assertEqual(len(native["expected_tests"]), 12)
        self.assertEqual(
            self.policy["native_windows"]["disposition"],
            "contract-only-native-enforcement-blocked",
        )


if __name__ == "__main__":
    unittest.main()
